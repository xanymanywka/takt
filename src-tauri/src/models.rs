//! Экран «Модели» (SPEC §10): все доступные модели по исполнителям, популярные сверху,
//! выбор хранится в `~/.orchestra/models.json` и подставляется в команды запуска.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::shell::run_zsh;

pub const PROVIDERS: [&str; 4] = ["claude", "codex", "agy", "ollama"];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub popular: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelProvider {
    pub id: String,
    pub found: bool,
    /// Откуда взят список: `claude --help`, `~/.codex/models_cache.json`, `agy models`, `ollama list`.
    pub source: String,
    pub models: Vec<ModelInfo>,
    pub selected: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelsInfo {
    pub path: String,
    pub providers: Vec<ModelProvider>,
}

fn home() -> Result<PathBuf, String> {
    dirs::home_dir().ok_or_else(|| "Не удалось определить домашнюю папку".to_owned())
}

fn models_path() -> Result<PathBuf, String> {
    Ok(home()?.join(".orchestra").join("models.json"))
}

/// Текущий выбор `{ "claude": "opus", "codex": "gpt-5.6-sol", ... }`.
pub fn read_selection() -> BTreeMap<String, String> {
    let Ok(path) = models_path() else {
        return BTreeMap::new();
    };
    let Ok(text) = fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    serde_json::from_str::<BTreeMap<String, Value>>(&text)
        .map(|map| {
            map.into_iter()
                .filter_map(|(key, value)| value.as_str().map(|s| (key, s.to_owned())))
                .filter(|(_, value)| !value.trim().is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Выбранная модель исполнителя (по имени провайдера в models.json).
pub fn selected_for(provider: &str) -> Option<String> {
    read_selection().get(provider).cloned()
}

fn valid_model_id(id: &str) -> bool {
    let re = Regex::new(r"^[A-Za-z0-9][A-Za-z0-9._:/-]{0,120}$").expect("valid regex");
    re.is_match(id)
}

// ---------- claude ----------

fn claude_static() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "fable".into(), label: "Fable (latest)".into(), popular: true, note: None },
        ModelInfo { id: "opus".into(), label: "Opus (latest)".into(), popular: true, note: None },
        ModelInfo { id: "sonnet".into(), label: "Sonnet (latest)".into(), popular: true, note: None },
        ModelInfo { id: "haiku".into(), label: "Haiku (latest)".into(), popular: false, note: None },
        ModelInfo { id: "claude-fable-5-1".into(), label: "Claude Fable 5.1".into(), popular: false, note: None },
        ModelInfo { id: "claude-opus-5-5".into(), label: "Claude Opus 5.5".into(), popular: false, note: None },
        ModelInfo { id: "claude-sonnet-5-5".into(), label: "Claude Sonnet 5.5".into(), popular: false, note: None },
        ModelInfo { id: "claude-haiku-5-5".into(), label: "Claude Haiku 5.5".into(), popular: false, note: None },
    ]
}

/// Алиасы из текста `--model` в `claude --help` (`'fable', 'opus', or 'sonnet'`).
fn parse_claude_help(help: &str) -> Vec<String> {
    let block = help
        .split("--model")
        .nth(1)
        .map(|rest| rest.chars().take(400).collect::<String>())
        .unwrap_or_default();
    let re = Regex::new(r"'([a-z][a-z0-9-]*)'").expect("valid regex");
    re.captures_iter(&block).map(|c| c[1].to_owned()).collect()
}

async fn claude_models() -> ModelProvider {
    let mut models = claude_static();
    let (found, error) = match run_zsh("claude --help", None).await {
        Ok(help) => {
            let aliases = parse_claude_help(&help);
            for alias in aliases {
                if !models.iter().any(|m| m.id == alias) {
                    let label = format!("{} (latest)", capitalize(&alias));
                    models.insert(0, ModelInfo { id: alias, label, popular: true, note: None });
                }
            }
            (true, None)
        }
        Err(error) => (false, Some(error)),
    };
    ModelProvider {
        id: "claude".into(),
        found,
        source: "claude --help".into(),
        models,
        selected: None,
        error,
    }
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

// ---------- codex ----------

fn parse_codex_cache(text: &str) -> Vec<ModelInfo> {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let Some(list) = value.get("models").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter(|m| m.get("visibility").and_then(Value::as_str) != Some("hide"))
        .filter_map(|m| {
            let id = m.get("slug").and_then(Value::as_str)?;
            let label = m
                .get("display_name")
                .and_then(Value::as_str)
                .unwrap_or(id)
                .to_owned();
            let efforts: Vec<String> = m
                .get("supported_reasoning_levels")
                .and_then(Value::as_array)
                .map(|levels| {
                    levels
                        .iter()
                        .filter_map(|l| l.get("effort").and_then(Value::as_str))
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            let note = (!efforts.is_empty()).then(|| efforts.join(" · "));
            Some(ModelInfo { id: id.to_owned(), label, popular: false, note })
        })
        .collect()
}

fn parse_codex_config_model(text: &str) -> Option<String> {
    let re = Regex::new(r#"(?m)^\s*model\s*=\s*"([^"]+)""#).expect("valid regex");
    re.captures(text).map(|c| c[1].to_owned())
}

fn codex_models() -> ModelProvider {
    let home = match home() {
        Ok(home) => home,
        Err(error) => {
            return ModelProvider {
                id: "codex".into(),
                found: false,
                source: "~/.codex".into(),
                models: Vec::new(),
                selected: None,
                error: Some(error),
            };
        }
    };
    let codex_dir = home.join(".codex");
    let config_model = fs::read_to_string(codex_dir.join("config.toml"))
        .ok()
        .and_then(|text| parse_codex_config_model(&text));
    let cache = fs::read_to_string(codex_dir.join("models_cache.json")).ok();
    let mut models = cache.as_deref().map(parse_codex_cache).unwrap_or_default();
    let source = if cache.is_some() {
        "~/.codex/models_cache.json"
    } else {
        "~/.codex/config.toml"
    };
    if let Some(ref current) = config_model {
        if !models.iter().any(|m| &m.id == current) {
            models.insert(
                0,
                ModelInfo { id: current.clone(), label: current.clone(), popular: true, note: None },
            );
        }
    }
    // Популярные: модель из config.toml и первые две из кэша.
    let mut marked = 0;
    for model in models.iter_mut() {
        let is_current = config_model.as_deref() == Some(model.id.as_str());
        if is_current || marked < 2 {
            model.popular = true;
            if !is_current {
                marked += 1;
            }
        }
    }
    let found = codex_dir.is_dir();
    ModelProvider {
        id: "codex".into(),
        found,
        source: source.into(),
        models,
        selected: None,
        error: (!found).then(|| "Codex не настроен: нет ~/.codex".to_owned()),
    }
}

// ---------- agy (Antigravity) ----------

/// `agy models` печатает `id<TAB>Название`; служебные строки пропускаем.
fn parse_agy_models(text: &str) -> Vec<ModelInfo> {
    text.lines()
        .filter_map(|line| {
            let (id, label) = line.split_once('\t')?;
            let id = id.trim();
            if id.is_empty() || !valid_model_id(id) {
                return None;
            }
            Some(ModelInfo { id: id.to_owned(), label: label.trim().to_owned(), popular: false, note: None })
        })
        .collect()
}

async fn agy_models() -> ModelProvider {
    match run_zsh("agy models", None).await {
        Ok(out) => {
            let mut models = parse_agy_models(&out);
            for model in models.iter_mut().take(3) {
                model.popular = true;
            }
            ModelProvider {
                id: "agy".into(),
                found: true,
                source: "agy models".into(),
                models,
                selected: None,
                error: None,
            }
        }
        Err(error) => ModelProvider {
            id: "agy".into(),
            found: false,
            source: "agy models".into(),
            models: Vec::new(),
            selected: None,
            error: Some(error),
        },
    }
}

// ---------- ollama ----------

fn parse_ollama_list(text: &str) -> Vec<ModelInfo> {
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let mut cols = line.split_whitespace();
            let id = cols.next()?;
            if !valid_model_id(id) {
                return None;
            }
            let _digest = cols.next();
            let size = cols.next().zip(cols.next()).map(|(n, unit)| format!("{n} {unit}"));
            Some(ModelInfo { id: id.to_owned(), label: id.to_owned(), popular: false, note: size })
        })
        .collect()
}

async fn ollama_models() -> ModelProvider {
    match run_zsh("ollama list", None).await {
        Ok(out) => {
            let mut models = parse_ollama_list(&out);
            // Популярные: кодовые модели вперёд, затем остальные, максимум три.
            models.sort_by_key(|m| !m.id.contains("coder"));
            for model in models.iter_mut().take(3) {
                model.popular = true;
            }
            ModelProvider {
                id: "ollama".into(),
                found: true,
                source: "ollama list".into(),
                models,
                selected: None,
                error: None,
            }
        }
        Err(error) => ModelProvider {
            id: "ollama".into(),
            found: false,
            source: "ollama list".into(),
            models: Vec::new(),
            selected: None,
            error: Some(error),
        },
    }
}

// ---------- команды ----------

#[tauri::command]
pub async fn list_models() -> Result<ModelsInfo, String> {
    let path = models_path()?;
    let selection = read_selection();
    let (claude, agy, ollama) = tokio::join!(claude_models(), agy_models(), ollama_models());
    let mut providers = vec![claude, codex_models(), agy, ollama];
    for provider in providers.iter_mut() {
        provider.selected = selection.get(&provider.id).cloned();
        // Выбранная вручную модель, которой нет в списке, всё равно показывается.
        if let Some(ref selected) = provider.selected {
            if !provider.models.iter().any(|m| &m.id == selected) {
                provider.models.insert(
                    0,
                    ModelInfo { id: selected.clone(), label: selected.clone(), popular: false, note: None },
                );
            }
        }
    }
    Ok(ModelsInfo { path: path.to_string_lossy().into_owned(), providers })
}

/// Записать выбор: `model = null` снимает выбор провайдера (исполнитель идёт со своей моделью по умолчанию).
#[tauri::command]
pub fn save_model(provider: String, model: Option<String>) -> Result<BTreeMap<String, String>, String> {
    if !PROVIDERS.contains(&provider.as_str()) {
        return Err(format!("Неизвестный исполнитель: {provider}"));
    }
    if let Some(ref model) = model {
        if !valid_model_id(model) {
            return Err(format!("Недопустимое имя модели: {model}"));
        }
    }
    let path = models_path()?;
    let mut selection = read_selection();
    match model {
        Some(model) => {
            selection.insert(provider, model);
        }
        None => {
            selection.remove(&provider);
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Не удалось создать {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(&selection).map_err(|error| error.to_string())?;
    fs::write(&path, text + "\n")
        .map_err(|error| format!("Не удалось записать {}: {error}", path.display()))?;
    Ok(selection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_help_aliases() {
        let help = "  --model <model>  Model for the current session. Provide an alias for the latest model (e.g. 'fable', 'opus', or 'sonnet') or a model's full name.\n  -n, --name <name>";
        assert_eq!(parse_claude_help(help), vec!["fable", "opus", "sonnet"]);
    }

    #[test]
    fn codex_cache_skips_hidden() {
        let json = r#"{"models":[
            {"slug":"gpt-6-sol","display_name":"GPT-6-Sol","visibility":"list","supported_reasoning_levels":[{"effort":"low"},{"effort":"high"}]},
            {"slug":"gpt-reserve","display_name":"GPT-Reserve","visibility":"hide"}
        ]}"#;
        let models = parse_codex_cache(json);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "gpt-6-sol");
        assert_eq!(models[0].note.as_deref(), Some("low · high"));
    }

    #[test]
    fn codex_config_model() {
        assert_eq!(
            parse_codex_config_model("model = \"gpt-5.6-sol\"\nmodel_reasoning_effort = \"low\"\n"),
            Some("gpt-5.6-sol".to_owned())
        );
    }

    #[test]
    fn agy_models_tab_separated() {
        let out = "Fetching available models...\ngemini-3.8-flash-high\tGemini 3.8 Flash (High)\nclaude-sonnet-4-6\tClaude Sonnet 4.6 (Thinking)\n";
        let models = parse_agy_models(out);
        assert_eq!(models.len(), 2);
        assert_eq!(models[1].label, "Claude Sonnet 4.6 (Thinking)");
    }

    #[test]
    fn ollama_list_columns() {
        let out = "NAME                ID              SIZE      MODIFIED\nqwen2.5-coder:7b    dae161e27b0e    4.7 GB    6 weeks ago\nllama3.2:3b         a80c4f17acd5    2.0 GB    6 weeks ago\n";
        let models = parse_ollama_list(out);
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "qwen2.5-coder:7b");
        assert_eq!(models[0].note.as_deref(), Some("4.7 GB"));
    }

    #[test]
    fn model_id_validation() {
        assert!(valid_model_id("gpt-5.6-sol"));
        assert!(valid_model_id("qwen2.5-coder:7b"));
        assert!(!valid_model_id("-m"));
        assert!(!valid_model_id("a b"));
    }
}
