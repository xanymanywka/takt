use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use serde::Serialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, State};

const ANTHROPIC_ENV: [&str; 3] = [
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_SMALL_FAST_MODEL",
];
static SESSIONS_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

struct Process {
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<ChildStdin>>,
}

#[derive(Clone, Default)]
pub struct ConductorState(Arc<Mutex<HashMap<String, Process>>>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConductorStart {
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConductorEvent {
    project_path: String,
    event: Value,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConductorExit {
    project_path: String,
    code: i32,
}

fn orchestra_path(name: &str) -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(".orchestra").join(name))
        .ok_or_else(|| "Не удалось определить домашнюю папку".to_owned())
}

fn read_sessions() -> Result<HashMap<String, String>, String> {
    let path = orchestra_path("sessions.json")?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("Не удалось прочитать {}: {error}", path.display()))?;
    serde_json::from_str(&content).map_err(|error| format!("Некорректный sessions.json: {error}"))
}

fn save_session(project: &str, session_id: &str) -> Result<(), String> {
    let _guard = SESSIONS_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "Не удалось заблокировать sessions.json".to_owned())?;
    let path = orchestra_path("sessions.json")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Не удалось создать {}: {error}", parent.display()))?;
    }
    let mut sessions = read_sessions()?;
    sessions.insert(project.to_owned(), session_id.to_owned());
    write_sessions(&path, &sessions)
}

fn remove_session(project: &str) -> Result<(), String> {
    let _guard = SESSIONS_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "Не удалось заблокировать sessions.json".to_owned())?;
    let path = orchestra_path("sessions.json")?;
    if !path.exists() {
        return Ok(());
    }
    let mut sessions = read_sessions()?;
    if sessions.remove(project).is_none() {
        return Ok(());
    }
    write_sessions(&path, &sessions)
}

fn write_sessions(path: &Path, sessions: &HashMap<String, String>) -> Result<(), String> {
    let json = serde_json::to_string_pretty(sessions)
        .map_err(|error| format!("Не удалось сериализовать сессии: {error}"))?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, format!("{json}\n"))
        .and_then(|_| fs::rename(&temporary, path))
        .map_err(|error| format!("Не удалось записать {}: {error}", path.display()))
}

fn read_lossy_lines(reader: impl Read, mut on_line: impl FnMut(String)) {
    let mut reader = BufReader::new(reader);
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        match reader.read_until(b'\n', &mut bytes) {
            Ok(0) => break,
            Ok(_) => {
                while matches!(bytes.last(), Some(b'\n' | b'\r')) {
                    bytes.pop();
                }
                on_line(String::from_utf8_lossy(&bytes).into_owned());
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
}

fn claude_command(project: &Path, session_id: Option<&str>) -> Result<Command, String> {
    let mut command = Command::new("claude");
    command
        .args([
            "--dangerously-skip-permissions",
            "-p",
            "--verbose",
            "--output-format",
            "stream-json",
            "--input-format",
            "stream-json",
        ])
        .current_dir(project)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(session_id) = session_id {
        command.args(["--resume", session_id]);
    }
    let instructions = orchestra_path("INSTRUCTIONS.md")?;
    if instructions.is_file() {
        command.arg("--append-system-prompt-file").arg(instructions);
    }
    for key in ANTHROPIC_ENV {
        command.env_remove(key);
    }
    command.env("PATH", crate::shell::tool_path());
    Ok(command)
}

fn is_login_error(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("not logged in")
        || lower.contains("please log in")
        || lower.contains("authentication required")
        || lower.contains("invalid api key")
        || lower.contains("нужен вход")
}

fn is_stale_session_error(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("no conversation found")
        || lower.contains("session not found")
        || lower.contains("session id") && lower.contains("not found")
}

fn session_from_event(event: &Value) -> Option<&str> {
    if event.get("type").and_then(Value::as_str) == Some("system")
        && event.get("subtype").and_then(Value::as_str) == Some("init")
    {
        event.get("session_id").and_then(Value::as_str)
    } else {
        None
    }
}

fn spawn_readers(
    app: AppHandle,
    state: ConductorState,
    project: String,
    stdout: impl Read + Send + 'static,
    stderr: impl Read + Send + 'static,
    child: Arc<Mutex<Child>>,
    resumed: bool,
) {
    let initialized = Arc::new(AtomicBool::new(false));
    let stale_session = Arc::new(AtomicBool::new(false));
    let stderr_project = project.clone();
    let stderr_app = app.clone();
    let stderr_stale_session = stale_session.clone();
    let stderr_thread = std::thread::spawn(move || {
        read_lossy_lines(stderr, |line| {
            if is_stale_session_error(&line) {
                stderr_stale_session.store(true, Ordering::Release);
            }
            let message = if is_login_error(&line) {
                "Нужен вход: выполните claude в терминале".to_owned()
            } else {
                line
            };
            let event = json!({ "type": "stderr", "message": message });
            let _ = stderr_app.emit(
                "conductor://event",
                ConductorEvent {
                    project_path: stderr_project.clone(),
                    event,
                },
            );
        });
    });

    let reader_initialized = initialized.clone();
    std::thread::spawn(move || {
        read_lossy_lines(stdout, |line| {
            let event = serde_json::from_str::<Value>(&line).unwrap_or(Value::String(line));
            if let Some(session_id) = session_from_event(&event)
                && let Err(error) = save_session(&project, session_id)
            {
                eprintln!("conductor session: {error}");
            }
            if session_from_event(&event).is_some() {
                reader_initialized.store(true, Ordering::Release);
            }
            let _ = app.emit(
                "conductor://event",
                ConductorEvent {
                    project_path: project.clone(),
                    event,
                },
            );
        });
        let code = loop {
            let status = child
                .lock()
                .ok()
                .and_then(|mut child| child.try_wait().ok())
                .flatten();
            if let Some(status) = status {
                break status.code().unwrap_or(-1);
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        let _ = stderr_thread.join();
        let should_emit = if let Ok(mut processes) = state.0.lock() {
            match processes.get(&project) {
                Some(process) if Arc::ptr_eq(&process.child, &child) => {
                    processes.remove(&project);
                    true
                }
                Some(_) => false,
                None => true,
            }
        } else {
            true
        };
        if should_emit
            && resumed
            && !reader_initialized.load(Ordering::Acquire)
            && stale_session.load(Ordering::Acquire)
            && let Err(error) = remove_session(&project)
        {
            eprintln!("conductor stale session: {error}");
        }
        if should_emit {
            let _ = app.emit(
                "conductor://exit",
                ConductorExit {
                    project_path: project,
                    code,
                },
            );
        }
    });
}

#[tauri::command(async)]
pub fn conductor_start(
    app: AppHandle,
    state: State<'_, ConductorState>,
    project_path: String,
) -> Result<ConductorStart, String> {
    let project = Path::new(&project_path);
    if !project.is_dir() {
        return Err(format!("Проект не найден: {project_path}"));
    }
    if state
        .0
        .lock()
        .map_err(|_| "Не удалось заблокировать список дирижёров".to_owned())?
        .contains_key(&project_path)
    {
        return Err("Дирижёр для этого проекта уже запущен".to_owned());
    }

    let session_id = {
        let _guard = SESSIONS_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .map_err(|_| "Не удалось заблокировать sessions.json".to_owned())?;
        read_sessions()?.get(&project_path).cloned()
    };
    let mut child = claude_command(project, session_id.as_deref())?
        .spawn()
        .map_err(|error| format!("Не удалось запустить claude: {error}"))?;
    std::thread::sleep(Duration::from_secs(1));
    if child
        .try_wait()
        .map_err(|error| format!("Не удалось проверить claude: {error}"))?
        .is_some()
    {
        let mut output = String::new();
        if let Some(mut stderr) = child.stderr.take() {
            let _ = stderr.read_to_string(&mut output);
        }
        if is_login_error(&output) {
            return Err("Нужен вход: выполните claude в терминале".to_owned());
        }
        return Err(format!("Claude завершился при запуске: {}", output.trim()));
    }

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Claude не открыл stdin".to_owned())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Claude не открыл stdout".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Claude не открыл stderr".to_owned())?;
    let child = Arc::new(Mutex::new(child));
    {
        let mut processes = state
            .0
            .lock()
            .map_err(|_| "Не удалось заблокировать список дирижёров".to_owned())?;
        if processes.contains_key(&project_path) {
            if let Ok(mut spawned) = child.lock() {
                let _ = spawned.kill();
                let _ = spawned.wait();
            }
            return Err("Дирижёр для этого проекта уже запущен".to_owned());
        }
        processes.insert(
            project_path.clone(),
            Process {
                child: child.clone(),
                stdin: Arc::new(Mutex::new(stdin)),
            },
        );
    }
    spawn_readers(
        app,
        state.inner().clone(),
        project_path,
        stdout,
        stderr,
        child,
        session_id.is_some(),
    );
    Ok(ConductorStart { session_id })
}

#[tauri::command(async)]
pub fn conductor_send(
    state: State<'_, ConductorState>,
    project_path: String,
    text: String,
) -> Result<(), String> {
    let stdin = state
        .0
        .lock()
        .map_err(|_| "Не удалось заблокировать список дирижёров".to_owned())?
        .get(&project_path)
        .map(|process| process.stdin.clone())
        .ok_or_else(|| "Дирижёр для проекта не запущен".to_owned())?;
    let line = serde_json::to_string(&json!({
        "type": "user",
        "message": { "role": "user", "content": text }
    }))
    .map_err(|error| format!("Не удалось сериализовать сообщение: {error}"))?;
    let mut stdin = stdin
        .lock()
        .map_err(|_| "Не удалось заблокировать stdin дирижёра".to_owned())?;
    writeln!(stdin, "{line}")
        .map_err(|error| format!("Не удалось отправить сообщение: {error}"))?;
    stdin
        .flush()
        .map_err(|error| format!("Не удалось отправить сообщение: {error}"))
}

#[tauri::command(async)]
pub fn conductor_stop(
    state: State<'_, ConductorState>,
    project_path: String,
) -> Result<(), String> {
    let process = state
        .0
        .lock()
        .map_err(|_| "Не удалось заблокировать список дирижёров".to_owned())?
        .remove(&project_path)
        .ok_or_else(|| "Дирижёр для проекта не запущен".to_owned())?;
    process
        .child
        .lock()
        .map_err(|_| "Не удалось заблокировать процесс дирижёра".to_owned())?
        .kill()
        .map_err(|error| format!("Не удалось остановить дирижёра: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_only_system_init_session() {
        let event = json!({"type":"system", "subtype":"init", "session_id":"abc"});
        assert_eq!(session_from_event(&event), Some("abc"));
        assert_eq!(
            session_from_event(&json!({"type":"result", "session_id":"no"})),
            None
        );
        assert!(is_stale_session_error(
            "No conversation found with session ID abc"
        ));
        assert!(!is_stale_session_error("network unavailable"));
    }

    #[test]
    #[ignore = "запускает реальный Claude Code и расходует подписочный лимит"]
    fn live_conductor_stream() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .canonicalize()
            .unwrap();
        let project_key = project.to_string_lossy().into_owned();
        let mut child = claude_command(&project, None)
            .expect("command")
            .spawn()
            .expect("spawn claude");
        let mut stdin = child.stdin.take().expect("stdin");
        writeln!(
            stdin,
            "{}",
            serde_json::to_string(&json!({
                "type": "user",
                "message": {"role": "user", "content": "Скажи одно слово: готов"}
            }))
            .unwrap()
        )
        .unwrap();
        drop(stdin);
        let output = child.wait_with_output().expect("claude output");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut types = Vec::new();
        let mut session_id = None;
        for line in stdout.lines() {
            let event: Value = serde_json::from_str(line).expect("stream-json line");
            if let Some(kind) = event.get("type").and_then(Value::as_str) {
                types.push(kind.to_owned());
            }
            if let Some(found) = session_from_event(&event) {
                session_id = Some(found.to_owned());
                save_session(&project_key, found).expect("save session");
            }
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "eventTypes": types,
                "sessionIdSaved": session_id.is_some(),
                "exitCode": output.status.code(),
            }))
            .unwrap()
        );
        assert!(types.iter().any(|kind| kind == "assistant"));
        assert!(types.iter().any(|kind| kind == "result"));
        let session_id = session_id.expect("system/init session_id");
        assert_eq!(
            read_sessions().unwrap().get(&project_key),
            Some(&session_id)
        );
    }
}
