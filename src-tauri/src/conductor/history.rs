//! История чата дирижёра по проектам: `~/.orchestra/chats/<путь>.json`.
//!
//! Пока файла нет, история один раз импортируется из транскрипта Claude Code
//! (`~/.claude/projects/<путь>/<сессия>.jsonl`) — так старые разговоры не теряются.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ConductorAgent, orchestra_path, read_sessions, write_json_atomic};

static CHATS_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Одно сообщение ленты чата в том виде, в каком его показывает интерфейс.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryMessage {
    pub id: String,
    /// `user`, `assistant` или `tool`.
    pub role: String,
    pub text: String,
    /// Момент сообщения в RFC 3339; у старых записей может отсутствовать.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

/// Путь проекта в имя файла — так же, как делает Claude Code для своих транскриптов.
pub fn encode_project_path(project: &str) -> String {
    project
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect()
}

fn chat_file(project: &str) -> Result<PathBuf, String> {
    orchestra_path("chats").map(|dir| dir.join(format!("{}.json", encode_project_path(project))))
}

fn read_chat(path: &PathBuf) -> Result<Vec<HistoryMessage>, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("Не удалось прочитать {}: {error}", path.display()))?;
    serde_json::from_str(&content).map_err(|error| format!("Некорректный {}: {error}", path.display()))
}

/// Транскрипт Claude Code для проекта: сессия Takt, если она есть, иначе самый
/// свежий разговор в `~/.claude/projects/<проект>/` — так видна переписка и для
/// проектов, с которыми работали из терминала до Takt.
fn claude_transcript(project: &str) -> Result<Option<PathBuf>, String> {
    let home = dirs::home_dir().ok_or_else(|| "Не удалось определить домашнюю папку".to_owned())?;
    let dir = home
        .join(".claude/projects")
        .join(encode_project_path(project));
    if let Some(entry) = read_sessions()?.remove(project) {
        if entry.agent != ConductorAgent::Claude {
            return Ok(None);
        }
        let path = dir.join(format!("{}.jsonl", entry.id));
        if path.is_file() {
            return Ok(Some(path));
        }
    }
    Ok(newest_transcript(&dir))
}

fn newest_transcript(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .filter_map(|path| {
            let modified = fs::metadata(&path).ok()?.modified().ok()?;
            Some((modified, path))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

fn instructions_text() -> Option<String> {
    orchestra_path("INSTRUCTIONS.md")
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
}

/// Takt перед текстом пользователя подставляет INSTRUCTIONS.md — в ленте этот
/// служебный префикс не нужен.
fn strip_instructions(text: &str, instructions: Option<&str>) -> String {
    const MARKER: &str = "Инструкции дирижёра (~/.orchestra/INSTRUCTIONS.md):\n\n";
    const SEPARATOR: &str = "\n\n---\n\n";
    if !text.starts_with(MARKER) {
        return text.to_owned();
    }
    if let Some(instructions) = instructions {
        let prefix = format!("{MARKER}{}{SEPARATOR}", instructions.trim_end());
        if let Some(rest) = text.strip_prefix(prefix.as_str()) {
            return rest.to_owned();
        }
    }
    match text.find(SEPARATOR) {
        Some(index) => text[index + SEPARATOR.len()..].to_owned(),
        None => text.to_owned(),
    }
}

fn text_blocks(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|block| block.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => String::new(),
    }
}

/// Подпись для вызова инструмента: имя и, если есть, файл или команда.
fn tool_label(block: &Value) -> String {
    let name = block
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("tool")
        .to_owned();
    let detail = block.get("input").and_then(|input| {
        ["file_path", "path", "command", "CommandLine", "pattern", "description"]
            .iter()
            .find_map(|key| input.get(key).and_then(Value::as_str))
    });
    match detail {
        Some(detail) => format!("{name}: {detail}"),
        None => name,
    }
}

/// Разбирает транскрипт Claude Code (JSONL) в ленту чата. Служебные строки,
/// результаты инструментов и ответвления (sidechain) пропускаются.
pub fn import_claude_transcript(text: &str, instructions: Option<&str>) -> Vec<HistoryMessage> {
    let mut messages: Vec<HistoryMessage> = Vec::new();
    for line in text.lines() {
        let Ok(entry) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if entry.get("isMeta").and_then(Value::as_bool) == Some(true)
            || entry.get("isSidechain").and_then(Value::as_bool) == Some(true)
        {
            continue;
        }
        let Some(message) = entry.get("message") else {
            continue;
        };
        let at = entry
            .get("timestamp")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let Some(content) = message.get("content") else {
            continue;
        };
        match entry.get("type").and_then(Value::as_str) {
            Some("user") => {
                let text = strip_instructions(text_blocks(content).trim(), instructions);
                if text.is_empty() || text.starts_with("<system-reminder>") {
                    continue;
                }
                let id = entry
                    .get("uuid")
                    .and_then(Value::as_str)
                    .map(|uuid| format!("usr_{uuid}"))
                    .unwrap_or_else(|| format!("usr_{}", messages.len()));
                messages.push(HistoryMessage {
                    id,
                    role: "user".to_owned(),
                    text,
                    at,
                });
            }
            Some("assistant") => {
                let id = message
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .or_else(|| {
                        entry
                            .get("uuid")
                            .and_then(Value::as_str)
                            .map(|uuid| format!("msg_{uuid}"))
                    })
                    .unwrap_or_else(|| format!("msg_{}", messages.len()));
                let text = text_blocks(content);
                if !text.trim().is_empty() {
                    match messages.iter_mut().find(|m| m.id == id) {
                        Some(existing) => {
                            existing.text.push_str("\n\n");
                            existing.text.push_str(text.trim());
                        }
                        None => messages.push(HistoryMessage {
                            id: id.clone(),
                            role: "assistant".to_owned(),
                            text: text.trim().to_owned(),
                            at: at.clone(),
                        }),
                    }
                }
                if let Some(blocks) = content.as_array() {
                    for block in blocks
                        .iter()
                        .filter(|block| block.get("type").and_then(Value::as_str) == Some("tool_use"))
                    {
                        let tool_id = block
                            .get("id")
                            .and_then(Value::as_str)
                            .map(|tool| format!("tool_{tool}"))
                            .unwrap_or_else(|| format!("tool_{}", messages.len()));
                        messages.push(HistoryMessage {
                            id: tool_id,
                            role: "tool".to_owned(),
                            text: tool_label(block),
                            at: at.clone(),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    messages
}

/// Лента чата проекта. Если Takt ещё ничего не записывал, подтягивает разговор
/// из транскрипта Claude Code и сохраняет его как свою историю.
#[tauri::command]
pub fn chat_history(project_path: String) -> Result<Vec<HistoryMessage>, String> {
    let _guard = CHATS_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "Не удалось заблокировать историю чатов".to_owned())?;
    let path = chat_file(&project_path)?;
    if path.exists() {
        return read_chat(&path);
    }
    let Some(transcript) = claude_transcript(&project_path)? else {
        return Ok(Vec::new());
    };
    let text = fs::read_to_string(&transcript)
        .map_err(|error| format!("Не удалось прочитать {}: {error}", transcript.display()))?;
    let messages = import_claude_transcript(&text, instructions_text().as_deref());
    if !messages.is_empty() {
        write_json_atomic(&path, &messages)?;
    }
    Ok(messages)
}

/// Дописывает сообщение в историю; сообщение с тем же `id` заменяется
/// (так обновляется текст ответа по мере стриминга).
#[tauri::command]
pub fn chat_record(project_path: String, message: HistoryMessage) -> Result<(), String> {
    let _guard = CHATS_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "Не удалось заблокировать историю чатов".to_owned())?;
    let path = chat_file(&project_path)?;
    let mut messages = if path.exists() {
        read_chat(&path)?
    } else {
        Vec::new()
    };
    match messages.iter_mut().find(|m| m.id == message.id) {
        Some(existing) => *existing = message,
        None => messages.push(message),
    }
    write_json_atomic(&path, &messages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_path_like_claude_code() {
        assert_eq!(
            encode_project_path("/Users/xanymanywka/projects/orchestra"),
            "-Users-xanymanywka-projects-orchestra"
        );
        assert_eq!(encode_project_path("/a/b.c d"), "-a-b-c-d");
    }

    #[test]
    fn strips_instruction_prefix() {
        let wrapped = "Инструкции дирижёра (~/.orchestra/INSTRUCTIONS.md):\n\n# Правила\n- раз\n\n---\n\nПривет";
        assert_eq!(strip_instructions(wrapped, Some("# Правила\n- раз\n")), "Привет");
        assert_eq!(strip_instructions(wrapped, None), "Привет");
        assert_eq!(strip_instructions("Просто текст", None), "Просто текст");
    }

    #[test]
    fn imports_user_assistant_and_tools() {
        let transcript = concat!(
            r#"{"type":"queue-operation","content":"x"}"#, "\n",
            r#"{"type":"user","uuid":"u1","timestamp":"2026-10-26T13:15:59Z","message":{"role":"user","content":"Скажи одно слово: готов"}}"#, "\n",
            r#"{"type":"assistant","uuid":"a1","timestamp":"2026-10-26T13:16:00Z","message":{"id":"msg_1","role":"assistant","content":[{"type":"text","text":"Готов"}]}}"#, "\n",
            r#"{"type":"assistant","uuid":"a2","message":{"id":"msg_1","role":"assistant","content":[{"type":"tool_use","id":"toolu_1","name":"Read","input":{"file_path":"/tmp/a.rs"}}]}}"#, "\n",
            r#"{"type":"user","uuid":"u2","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"..."}]}}"#, "\n",
            r#"{"type":"user","uuid":"u3","isMeta":true,"message":{"role":"user","content":"meta"}}"#, "\n",
            r#"{"type":"user","uuid":"u4","isSidechain":true,"message":{"role":"user","content":"side"}}"#, "\n",
            "not json\n",
        );
        let messages = import_claude_transcript(transcript, None);
        let summary: Vec<(&str, &str)> = messages
            .iter()
            .map(|m| (m.role.as_str(), m.text.as_str()))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("user", "Скажи одно слово: готов"),
                ("assistant", "Готов"),
                ("tool", "Read: /tmp/a.rs"),
            ]
        );
        assert_eq!(messages[0].id, "usr_u1");
        assert_eq!(messages[0].at.as_deref(), Some("2026-10-26T13:15:59Z"));
        assert_eq!(messages[1].id, "msg_1");
    }

    #[test]
    fn merges_split_assistant_text_by_message_id() {
        let transcript = concat!(
            r#"{"type":"assistant","message":{"id":"m","content":[{"type":"text","text":"Раз"}]}}"#, "\n",
            r#"{"type":"assistant","message":{"id":"m","content":[{"type":"text","text":"Два"}]}}"#, "\n",
        );
        let messages = import_claude_transcript(transcript, None);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "Раз\n\nДва");
    }
}
