//! Нормализация вывода `codex exec --json` (JSONL) в stream-json формат дирижёра.
//!
//! Фронт понимает только события Claude Code: `system/init`, `assistant`
//! (`text` и `tool_use`), `result`, `stderr`. Codex пишет свои события —
//! `thread.started`, `item.*`, `turn.*`, `error`; здесь они переводятся один в один.

use std::collections::HashSet;

use serde_json::{Map, Value, json};

/// Состояние нормализатора на один ход: какие инструменты уже показаны,
/// чтобы `item.started` и `item.completed` не давали двух строк в чате.
#[derive(Default)]
pub struct CodexNormalizer {
    seen_tools: HashSet<String>,
}

impl CodexNormalizer {
    /// Переводит одну строку JSONL в ноль или несколько событий stream-json.
    pub fn normalize(&mut self, line: &str) -> Vec<Value> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        let Ok(event) = serde_json::from_str::<Value>(trimmed) else {
            return vec![json!({ "type": "stderr", "message": trimmed })];
        };
        let Some(kind) = event.get("type").and_then(Value::as_str) else {
            return Vec::new();
        };
        match kind {
            "thread.started" => event
                .get("thread_id")
                .and_then(Value::as_str)
                .map(|thread_id| {
                    vec![json!({
                        "type": "system",
                        "subtype": "init",
                        "session_id": thread_id,
                    })]
                })
                .unwrap_or_default(),
            "item.started" | "item.updated" | "item.completed" => event
                .get("item")
                .map(|item| self.item(item))
                .unwrap_or_default(),
            "turn.completed" => {
                let mut result = Map::new();
                result.insert("type".into(), "result".into());
                result.insert("subtype".into(), "success".into());
                result.insert("is_error".into(), false.into());
                if let Some(usage) = event.get("usage") {
                    result.insert("usage".into(), usage.clone());
                }
                vec![Value::Object(result)]
            }
            "turn.failed" => {
                let message = event
                    .get("error")
                    .and_then(|error| error.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("Codex не завершил ход")
                    .to_owned();
                vec![
                    json!({ "type": "stderr", "message": message }),
                    json!({
                        "type": "result",
                        "subtype": "error",
                        "is_error": true,
                        "result": message,
                    }),
                ]
            }
            "error" => {
                let message = event
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Codex сообщил об ошибке")
                    .to_owned();
                vec![json!({ "type": "stderr", "message": message })]
            }
            _ => Vec::new(),
        }
    }

    fn item(&mut self, item: &Value) -> Vec<Value> {
        let id = item
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("codex-item")
            .to_owned();
        let kind = item.get("type").and_then(Value::as_str).unwrap_or_default();
        let blocks: Vec<Value> = match kind {
            "agent_message" => {
                let text = item.get("text").and_then(Value::as_str).unwrap_or_default();
                if text.is_empty() {
                    return Vec::new();
                }
                vec![json!({ "type": "text", "text": text })]
            }
            "command_execution" => {
                let command = item
                    .get("command")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                vec![tool_use(&id, "Bash", json!({ "command": command }))]
            }
            "file_change" => item
                .get("changes")
                .and_then(Value::as_array)
                .map(|changes| {
                    changes
                        .iter()
                        .filter_map(|change| change.get("path").and_then(Value::as_str))
                        .map(|path| tool_use(&id, "Edit", json!({ "path": path })))
                        .collect()
                })
                .unwrap_or_default(),
            "mcp_tool_call" => {
                let server = item.get("server").and_then(Value::as_str).unwrap_or("mcp");
                let tool = item.get("tool").and_then(Value::as_str).unwrap_or("tool");
                let arguments = item
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                vec![tool_use(&id, &format!("{server}/{tool}"), arguments)]
            }
            "web_search" => {
                let query = item.get("query").and_then(Value::as_str).unwrap_or_default();
                vec![tool_use(&id, "WebSearch", json!({ "query": query }))]
            }
            _ => Vec::new(),
        };
        if blocks.is_empty() {
            return Vec::new();
        }
        // Текст обновляется в одном пузыре (тот же `message.id`), инструмент показываем один раз.
        if kind != "agent_message" && !self.seen_tools.insert(id.clone()) {
            return Vec::new();
        }
        vec![json!({
            "type": "assistant",
            "message": { "id": id, "role": "assistant", "content": blocks },
        })]
    }
}

fn tool_use(id: &str, name: &str, input: Value) -> Value {
    json!({ "type": "tool_use", "id": id, "name": name, "input": input })
}

/// Значение для `-c key=<value>`: TOML basic string, чтобы многострочный текст
/// с кавычками и обратными слэшами дошёл до Codex как одна строка.
pub fn toml_basic_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Фикстуры собраны по описанию формата `codex exec --json`; живой захват
    // не делался — лимит Codex на момент разработки был исчерпан.
    const THREAD_STARTED: &str =
        r#"{"type":"thread.started","thread_id":"0199a213-81c0-7800-b1a1-2f9c0e5d4a11"}"#;
    const TURN_STARTED: &str = r#"{"type":"turn.started"}"#;
    const MESSAGE_STARTED: &str =
        r#"{"type":"item.started","item":{"id":"item_1","type":"agent_message","text":"Смотрю"}}"#;
    const MESSAGE_COMPLETED: &str = r#"{"type":"item.completed","item":{"id":"item_1","type":"agent_message","text":"Смотрю ворктри и план."}}"#;
    const COMMAND_STARTED: &str = r#"{"type":"item.started","item":{"id":"item_2","type":"command_execution","command":"bash -lc 'git status --short'","aggregated_output":"","status":"in_progress"}}"#;
    const COMMAND_COMPLETED: &str = r#"{"type":"item.completed","item":{"id":"item_2","type":"command_execution","command":"bash -lc 'git status --short'","aggregated_output":" M src/a.rs\n","exit_code":0,"status":"completed"}}"#;
    const FILE_CHANGE: &str = r#"{"type":"item.completed","item":{"id":"item_3","type":"file_change","changes":[{"path":"/repo/src/a.rs","kind":"update"},{"path":"/repo/docs/B.md","kind":"add"}],"status":"completed"}}"#;
    const REASONING: &str =
        r#"{"type":"item.completed","item":{"id":"item_4","type":"reasoning","text":"думаю"}}"#;
    const TURN_COMPLETED: &str = r#"{"type":"turn.completed","usage":{"input_tokens":1200,"cached_input_tokens":800,"output_tokens":90}}"#;
    const ERROR: &str = r#"{"type":"error","message":"stream disconnected before completion"}"#;
    const TURN_FAILED: &str =
        r#"{"type":"turn.failed","error":{"message":"Not logged in. Run `codex login`."}}"#;

    #[test]
    fn thread_started_becomes_system_init() {
        let events = CodexNormalizer::default().normalize(THREAD_STARTED);
        assert_eq!(
            events,
            vec![json!({
                "type": "system",
                "subtype": "init",
                "session_id": "0199a213-81c0-7800-b1a1-2f9c0e5d4a11"
            })]
        );
        assert!(CodexNormalizer::default().normalize(TURN_STARTED).is_empty());
    }

    #[test]
    fn agent_message_keeps_one_bubble() {
        let mut normalizer = CodexNormalizer::default();
        let started = normalizer.normalize(MESSAGE_STARTED);
        let completed = normalizer.normalize(MESSAGE_COMPLETED);
        assert_eq!(started.len(), 1);
        assert_eq!(completed.len(), 1);
        assert_eq!(started[0]["type"], "assistant");
        assert_eq!(started[0]["message"]["id"], completed[0]["message"]["id"]);
        assert_eq!(started[0]["message"]["content"][0]["text"], "Смотрю");
        assert_eq!(
            completed[0]["message"]["content"][0]["text"],
            "Смотрю ворктри и план."
        );
    }

    #[test]
    fn command_execution_is_bash_tool_use_once() {
        let mut normalizer = CodexNormalizer::default();
        let started = normalizer.normalize(COMMAND_STARTED);
        assert_eq!(started.len(), 1);
        let block = &started[0]["message"]["content"][0];
        assert_eq!(block["type"], "tool_use");
        assert_eq!(block["name"], "Bash");
        assert_eq!(block["input"]["command"], "bash -lc 'git status --short'");
        assert!(normalizer.normalize(COMMAND_COMPLETED).is_empty());
    }

    #[test]
    fn file_change_lists_each_path_as_edit() {
        let events = CodexNormalizer::default().normalize(FILE_CHANGE);
        let content = events[0]["message"]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["name"], "Edit");
        assert_eq!(content[0]["input"]["path"], "/repo/src/a.rs");
        assert_eq!(content[1]["input"]["path"], "/repo/docs/B.md");
    }

    #[test]
    fn reasoning_is_hidden_and_turn_completed_is_result() {
        let mut normalizer = CodexNormalizer::default();
        assert!(normalizer.normalize(REASONING).is_empty());
        let events = normalizer.normalize(TURN_COMPLETED);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["type"], "result");
        assert_eq!(events[0]["is_error"], false);
        assert_eq!(events[0]["usage"]["output_tokens"], 90);
    }

    #[test]
    fn errors_become_stderr() {
        let mut normalizer = CodexNormalizer::default();
        let events = normalizer.normalize(ERROR);
        assert_eq!(
            events,
            vec![json!({ "type": "stderr", "message": "stream disconnected before completion" })]
        );
        let failed = normalizer.normalize(TURN_FAILED);
        assert_eq!(failed.len(), 2);
        assert_eq!(failed[0]["type"], "stderr");
        assert_eq!(failed[1]["type"], "result");
        assert_eq!(failed[1]["is_error"], true);
        let plain = normalizer.normalize("warning: something odd");
        assert_eq!(plain[0]["type"], "stderr");
    }

    #[test]
    fn toml_string_escapes_control_characters() {
        assert_eq!(
            toml_basic_string("a \"b\"\n\\c\t\u{1}"),
            "\"a \\\"b\\\"\\n\\\\c\\t\\u0001\""
        );
        assert_eq!(toml_basic_string("дирижёр"), "\"дирижёр\"");
    }
}
