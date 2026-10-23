use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter};

use crate::shell::run_zsh;

const PROVIDERS: [&str; 3] = ["claude", "codex", "antigravity"];

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Limit {
    provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_left: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    weekly_left: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resets_in: Option<String>,
    raw: String,
}

fn left_from_used(value: Option<f64>) -> Option<u8> {
    value.map(|used| (100.0 - used).clamp(0.0, 100.0).round() as u8)
}

fn window_used(root: &Value, name: &str) -> Option<f64> {
    root.pointer(&format!("/usage/{name}/usedPercent"))
        .and_then(Value::as_f64)
}

fn human_reset(timestamp: Option<&str>) -> Option<String> {
    let target = DateTime::parse_from_rfc3339(timestamp?)
        .ok()?
        .with_timezone(&Utc);
    let seconds = (target - Utc::now()).num_seconds().max(0);
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    Some(if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    })
}

fn parse_json(provider: &str, raw: &str) -> Option<Limit> {
    let value: Value = serde_json::from_str(raw).ok()?;
    let root = value.as_array()?.first()?;
    let mut session_left = left_from_used(window_used(root, "primary"));
    let mut weekly_left = left_from_used(window_used(root, "secondary"));
    let mut resets_in = root
        .pointer("/usage/primary/resetDescription")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            human_reset(
                root.pointer("/usage/primary/resetsAt")
                    .and_then(Value::as_str),
            )
        });

    if provider == "antigravity" {
        session_left = None;
        weekly_left = root
            .pointer("/usage/extraRateWindows")
            .and_then(Value::as_array)
            .and_then(|windows| {
                windows.iter().find(|window| {
                    window
                        .get("title")
                        .and_then(Value::as_str)
                        .is_some_and(|title| title.eq_ignore_ascii_case("Gemini weekly"))
                })
            })
            .and_then(|window| window.pointer("/window/usedPercent"))
            .and_then(Value::as_f64)
            .and_then(|used| left_from_used(Some(used)))
            .or_else(|| left_from_used(window_used(root, "primary")));
        resets_in = root
            .pointer("/usage/primary/resetDescription")
            .and_then(Value::as_str)
            .and_then(parse_reset_description)
            .or_else(|| {
                human_reset(
                    root.pointer("/usage/primary/resetsAt")
                        .and_then(Value::as_str),
                )
            });
    }

    Some(Limit {
        provider: provider.to_owned(),
        session_left,
        weekly_left,
        resets_in,
        raw: raw.to_owned(),
    })
}

fn parse_reset_description(description: &str) -> Option<String> {
    let regex = Regex::new(r"(?i)(\d+)\s*days?,\s*(\d+)\s*hours?").expect("valid regex");
    let captures = regex.captures(description)?;
    Some(format!("{}d {}h", &captures[1], &captures[2]))
}

fn parse_text(provider: &str, raw: &str) -> Limit {
    let session = Regex::new(r"(?im)^Session:\s*(\d+)%\s+left").expect("valid regex");
    let weekly = Regex::new(r"(?im)^(?:Gemini\s+)?Weekly:\s*(\d+)%\s+left").expect("valid regex");
    let reset = Regex::new(r"(?im)^Resets in\s+(.+?)\s*$").expect("valid regex");
    Limit {
        provider: provider.to_owned(),
        session_left: session
            .captures(raw)
            .and_then(|capture| capture[1].parse().ok()),
        weekly_left: weekly
            .captures(raw)
            .and_then(|capture| capture[1].parse().ok()),
        resets_in: reset.captures(raw).map(|capture| capture[1].to_owned()),
        raw: raw.to_owned(),
    }
}

pub async fn collect_limits() -> Result<Vec<Limit>, String> {
    let help = run_zsh("codexbar usage --help", None)
        .await
        .unwrap_or_default();
    let has_json = help.contains("--format text|json") || help.contains("--json");
    let mut limits = Vec::with_capacity(PROVIDERS.len());
    for provider in PROVIDERS {
        let command = if has_json {
            format!("codexbar usage --provider {provider} --format json")
        } else {
            format!("codexbar usage --provider {provider} --no-color")
        };
        let raw = match run_zsh(&command, Option::<&Path>::None).await {
            Ok(raw) => raw,
            Err(error) => {
                limits.push(Limit {
                    provider: provider.to_owned(),
                    session_left: None,
                    weekly_left: None,
                    resets_in: None,
                    raw: error,
                });
                continue;
            }
        };
        limits.push(if has_json {
            parse_json(provider, &raw).unwrap_or_else(|| parse_text(provider, &raw))
        } else {
            parse_text(provider, &raw)
        });
    }
    Ok(limits)
}

#[tauri::command]
pub async fn limits() -> Result<Vec<Limit>, String> {
    collect_limits().await
}

pub fn start_poller(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            if let Ok(limits) = collect_limits().await {
                let _ = app.emit("limits://changed", limits);
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_text_fallback() {
        let result = parse_text(
            "claude",
            "Session: 65% left\nWeekly: 94% left\nResets in 2h 34m",
        );
        assert_eq!(result.session_left, Some(65));
        assert_eq!(result.weekly_left, Some(94));
        assert_eq!(result.resets_in.as_deref(), Some("2h 34m"));
    }

    #[test]
    fn parses_json_percent_used_as_percent_left() {
        let result = parse_json(
            "codex",
            r#"[{"usage":{"primary":{"usedPercent":35,"resetsAt":"2099-01-01T00:00:00Z"},"secondary":{"usedPercent":6}}}]"#,
        )
        .unwrap();
        assert_eq!(result.session_left, Some(65));
        assert_eq!(result.weekly_left, Some(94));
    }

    #[tokio::test]
    #[ignore = "вызывает реальный codexbar"]
    async fn live_limits_snapshot() {
        let limits = collect_limits().await.expect("live limits");
        let summary: Vec<_> = limits
            .iter()
            .map(|limit| {
                serde_json::json!({
                    "provider": limit.provider,
                    "sessionLeft": limit.session_left,
                    "weeklyLeft": limit.weekly_left,
                    "resetsIn": limit.resets_in,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&summary).unwrap());
        assert_eq!(limits.len(), 3);
    }
}
