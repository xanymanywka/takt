use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

use crate::platform;

const ANTHROPIC_ENV: [&str; 3] = [
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_SMALL_FAST_MODEL",
];

#[derive(Debug, thiserror::Error)]
enum ShellError {
    #[error("Не удалось запустить оболочку: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("Команда завершилась с ошибкой: {0}")]
    Failed(String),
    #[error("Превышено время ожидания команды")]
    Timeout,
}

pub async fn run_shell(cmd: &str, cwd: Option<&Path>) -> Result<String, String> {
    run_shell_inner(cmd, cwd)
        .await
        .map_err(|error| error.to_string())
}

/// Прежнее имя: его ещё зовут `limits.rs` и `models.rs` (правят соседние исполнители).
/// Поведение то же, что у `run_shell`; удалить, когда вызовов не останется.
pub async fn run_zsh(cmd: &str, cwd: Option<&Path>) -> Result<String, String> {
    run_shell(cmd, cwd).await
}

/// Совместимость для `conductor.rs`; источник правды — `platform::tool_path`.
pub fn tool_path() -> String {
    platform::tool_path()
}

async fn run_shell_inner(cmd: &str, cwd: Option<&Path>) -> Result<String, ShellError> {
    let path = platform::tool_path();
    let (program, args_prefix) = platform::login_shell();

    let mut command = Command::new(program);
    command
        .args(args_prefix)
        .arg(cmd)
        .env("PATH", path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for key in ANTHROPIC_ENV {
        command.env_remove(key);
    }
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }

    command.kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(30), command.output())
        .await
        .map_err(|_| ShellError::Timeout)??;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        Err(ShellError::Failed(if stderr.is_empty() {
            stdout
        } else {
            stderr
        }))
    }
}

pub fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
