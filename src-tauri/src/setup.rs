use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use serde::Serialize;
use serde_json::Value;

use crate::shell::{quote, run_zsh};

const TOOL_IDS: [&str; 8] = [
    "claude",
    "codex",
    "agy",
    "gh",
    "codexbar",
    "coderabbit",
    "tmux",
    "git",
];

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    id: String,
    found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    logged_in: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    account: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshHost {
    host: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    identity_file: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct McpServer {
    name: String,
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Setup {
    instructions_exists: bool,
    projects_exists: bool,
    tools: Vec<Tool>,
    ssh: Vec<SshHost>,
    mcp: Vec<McpServer>,
}

fn home() -> Result<PathBuf, String> {
    dirs::home_dir().ok_or_else(|| "Не удалось определить домашнюю папку".to_owned())
}

fn first_line(value: &str) -> Option<String> {
    value
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().chars().take(200).collect())
}

fn account_from(text: &str) -> Option<String> {
    let email = Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").expect("valid regex");
    email.find(text).map(|found| found.as_str().to_owned())
}

fn gh_account_from(text: &str) -> Option<String> {
    let account = Regex::new(r"(?m)account\s+([^\s(]+)").expect("valid regex");
    account.captures(text).map(|capture| capture[1].to_owned())
}

async fn auth_status(id: &str, home: &Path) -> (Option<bool>, Option<String>) {
    match id {
        "claude" => match run_zsh("claude auth status", None).await {
            Ok(output) => (Some(true), account_from(&output)),
            Err(_) => (Some(home.join(".claude/.credentials.json").is_file()), None),
        },
        "codex" => match run_zsh("codex login status", None).await {
            Ok(output) => (Some(true), account_from(&output)),
            Err(_) => (Some(false), None),
        },
        "gh" => match run_zsh("gh auth status 2>&1", None).await {
            Ok(output) => (Some(true), gh_account_from(&output)),
            Err(_) => (Some(false), None),
        },
        _ => (None, None),
    }
}

async fn inspect_tool(id: &str, home: &Path) -> Tool {
    let path = run_zsh(&format!("command -v {}", quote(id)), None)
        .await
        .ok()
        .and_then(|output| first_line(&output));
    let found = path.is_some();
    let version = if found {
        let version_command = if id == "tmux" {
            "tmux -V".to_owned()
        } else {
            format!("{} --version", quote(id))
        };
        run_zsh(&version_command, None)
            .await
            .ok()
            .and_then(|output| first_line(&output))
    } else {
        None
    };
    let (logged_in, account) = if found {
        auth_status(id, home).await
    } else {
        (None, None)
    };
    Tool {
        id: id.to_owned(),
        found,
        path,
        version,
        logged_in,
        account,
    }
}

fn parse_ssh(content: &str) -> Vec<SshHost> {
    let mut hosts = Vec::new();
    let mut current: Option<SshHost> = None;
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, char::is_whitespace);
        let key = parts.next().unwrap_or_default().to_ascii_lowercase();
        let value = parts.next().unwrap_or_default().trim();
        if key == "host" {
            if let Some(host) = current.take() {
                hosts.push(host);
            }
            let aliases = value
                .split_whitespace()
                .filter(|host| !host.contains('*') && !host.contains('?'))
                .collect::<Vec<_>>();
            if let Some(host) = aliases.first() {
                current = Some(SshHost {
                    host: (*host).to_owned(),
                    hostname: None,
                    user: None,
                    port: None,
                    identity_file: None,
                });
            }
            continue;
        }
        let Some(host) = current.as_mut() else {
            continue;
        };
        match key.as_str() {
            "hostname" => host.hostname = Some(value.to_owned()),
            "user" => host.user = Some(value.to_owned()),
            "port" => host.port = value.parse().ok(),
            "identityfile" => host.identity_file = Some(value.to_owned()),
            _ => {}
        }
    }
    if let Some(host) = current {
        hosts.push(host);
    }
    hosts
}

fn clean_url(value: &str) -> String {
    let without_fragment = value.split('#').next().unwrap_or(value);
    let without_query = without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment);
    let credentials = Regex::new(r"^(https?://)[^/@]+@(?P<rest>.+)$").expect("valid regex");
    credentials
        .replace(without_query, "${1}${rest}")
        .into_owned()
}

fn clean_command(value: &str) -> String {
    let assignment = Regex::new(
        r"(?i)\b([A-Za-z0-9_]*(?:token|secret|password|api_key)[A-Za-z0-9_]*)=([^\s;]+)",
    )
    .expect("valid regex");
    let query =
        Regex::new(r#"(?i)([?&](?:token|key|api_key|secret)=)[^&\s"']+"#).expect("valid regex");
    let redacted = assignment.replace_all(value, "${1}=<redacted>");
    query.replace_all(&redacted, "${1}<redacted>").into_owned()
}

fn unquote(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value[1..value.len() - 1].to_owned()
    } else {
        value.to_owned()
    }
}

fn parse_codex_mcp(content: &str) -> Vec<McpServer> {
    let section =
        Regex::new(r#"^\s*\[mcp_servers\.("[^"]+"|[A-Za-z0-9_-]+)\]\s*$"#).expect("valid regex");
    let field = Regex::new(r"^\s*(command|url)\s*=\s*(.+?)\s*$").expect("valid regex");
    let mut result = Vec::new();
    let mut current: Option<McpServer> = None;
    for line in content.lines() {
        if let Some(capture) = section.captures(line) {
            if let Some(server) = current.take() {
                result.push(server);
            }
            current = Some(McpServer {
                name: capture[1].trim_matches('"').to_owned(),
                source: "codex".to_owned(),
                command: None,
                url: None,
            });
        } else if line.trim_start().starts_with('[') {
            if let Some(server) = current.take() {
                result.push(server);
            }
        } else if let (Some(server), Some(capture)) = (current.as_mut(), field.captures(line)) {
            let value = unquote(&capture[2]);
            match &capture[1] {
                "command" => server.command = Some(clean_command(&value)),
                "url" => server.url = Some(clean_url(&value)),
                _ => {}
            }
        }
    }
    if let Some(server) = current {
        result.push(server);
    }
    result
}

fn string_field(value: &Value, names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| value.get(name).and_then(Value::as_str))
        .map(ToOwned::to_owned)
}

fn parse_agy_mcp(content: &str) -> Vec<McpServer> {
    let Ok(root) = serde_json::from_str::<Value>(content) else {
        return Vec::new();
    };
    let servers = root
        .get("mcpServers")
        .or_else(|| root.get("mcp_servers"))
        .unwrap_or(&root);
    let Some(servers) = servers.as_object() else {
        return Vec::new();
    };
    servers
        .iter()
        .map(|(name, config)| McpServer {
            name: name.to_owned(),
            source: "agy".to_owned(),
            command: string_field(config, &["command"]).map(|command| clean_command(&command)),
            url: string_field(config, &["url", "serverUrl"]).map(|url| clean_url(&url)),
        })
        .collect()
}

#[tauri::command]
pub async fn setup_status() -> Result<Setup, String> {
    let home = home()?;
    let mut tools = Vec::with_capacity(TOOL_IDS.len());
    for id in TOOL_IDS {
        tools.push(inspect_tool(id, &home).await);
    }
    let ssh = fs::read_to_string(home.join(".ssh/config"))
        .map(|content| parse_ssh(&content))
        .unwrap_or_default();
    let mut mcp = fs::read_to_string(home.join(".codex/config.toml"))
        .map(|content| parse_codex_mcp(&content))
        .unwrap_or_default();
    mcp.extend(
        fs::read_to_string(home.join(".gemini/config/mcp_config.json"))
            .map(|content| parse_agy_mcp(&content))
            .unwrap_or_default(),
    );
    Ok(Setup {
        instructions_exists: home.join(".orchestra/INSTRUCTIONS.md").is_file(),
        projects_exists: home.join(".orchestra/projects.json").is_file(),
        tools,
        ssh,
        mcp,
    })
}

#[tauri::command]
pub fn write_instructions(force: Option<bool>) -> Result<String, String> {
    let path = home()?.join(".orchestra/INSTRUCTIONS.md");
    write_instructions_at(&path, force.unwrap_or(false))?;
    Ok(path.to_string_lossy().into_owned())
}

fn write_instructions_at(path: &Path, force: bool) -> Result<(), String> {
    if path.exists() && !force {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Не удалось создать {}: {error}", parent.display()))?;
    }
    fs::write(path, include_str!("../templates/INSTRUCTIONS.md"))
        .map_err(|error| format!("Не удалось записать {}: {error}", path.display()))?;
    Ok(())
}

fn applescript_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[tauri::command(async)]
pub fn open_in_terminal(cwd: String, command: Option<String>) -> Result<(), String> {
    if !Path::new(&cwd).is_dir() {
        return Err(format!("Папка не найдена: {cwd}"));
    }
    let mut shell = format!("cd {}", quote(&cwd));
    if let Some(command) = command.filter(|command| !command.trim().is_empty()) {
        shell.push_str(" && ");
        shell.push_str(&command);
    }
    let script = format!(
        "tell application \"Terminal\"\nactivate\ndo script {}\nend tell",
        applescript_string(&shell)
    );
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|error| format!("Не удалось открыть Terminal: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Terminal вернул ошибку: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_parser_ignores_wildcards_and_unknown_fields() {
        let hosts = parse_ssh(
            "Host *\n  ForwardAgent yes\nHost demo\n  HostName example.com\n  User deploy\n  Port 2222\n  IdentityFile ~/.ssh/id_demo\n  ProxyCommand secret",
        );
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].host, "demo");
        assert_eq!(hosts[0].port, Some(2222));
    }

    #[test]
    fn mcp_parser_omits_env_and_url_secrets() {
        let servers = parse_codex_mcp(
            "[mcp_servers.demo]\ncommand = \"node\"\nenv = { TOKEN = \"secret\" }\nurl = \"https://user:pass@example.com/mcp?token=secret\"",
        );
        assert_eq!(servers[0].command.as_deref(), Some("node"));
        assert_eq!(servers[0].url.as_deref(), Some("https://example.com/mcp"));
    }

    #[test]
    fn instructions_are_not_overwritten_without_force() {
        let path = std::env::temp_dir().join(format!(
            "orchestra-instructions-{}-{}.md",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        fs::write(&path, "existing").unwrap();
        write_instructions_at(&path, false).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "existing");
        write_instructions_at(&path, true).unwrap();
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .contains("Правила дирижёра")
        );
        let _ = fs::remove_file(path);
    }

    #[tokio::test]
    #[ignore = "проверяет реальные инструменты и конфиги пользователя"]
    async fn live_setup_snapshot() {
        let setup = setup_status().await.expect("live setup");
        println!("{}", serde_json::to_string_pretty(&setup).unwrap());
        assert_eq!(setup.tools.len(), TOOL_IDS.len());
    }

    #[test]
    #[ignore = "создаёт реальный ~/.orchestra/INSTRUCTIONS.md"]
    fn live_write_instructions_is_idempotent() {
        let path = PathBuf::from(write_instructions(None).expect("first write"));
        let before = fs::read(&path).expect("read first write");
        let second = PathBuf::from(write_instructions(None).expect("second write"));
        let after = fs::read(&second).expect("read second write");
        println!("{} ({} bytes)", second.display(), after.len());
        assert_eq!(path, second);
        assert_eq!(before, after);
    }
}
