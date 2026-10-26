use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, State};

mod codex;
mod history;

use codex::CodexNormalizer;
pub use history::{chat_history, chat_record};

const ANTHROPIC_ENV: [&str; 3] = [
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_SMALL_FAST_MODEL",
];
static SESSIONS_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static CONFIG_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Какой CLI сидит за пультом дирижёра. Хранится в `~/.orchestra/conductor.json`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConductorAgent {
    Claude,
    Codex,
    Agy,
}

impl ConductorAgent {
    pub const ALL: [ConductorAgent; 3] = [
        ConductorAgent::Claude,
        ConductorAgent::Codex,
        ConductorAgent::Agy,
    ];

    fn parse(id: &str) -> Option<Self> {
        match id.trim() {
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            "agy" => Some(Self::Agy),
            _ => None,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Agy => "agy",
        }
    }

    /// Имя исполняемого файла в PATH.
    fn binary(self) -> &'static str {
        self.id()
    }

    fn title(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Codex => "Codex",
            Self::Agy => "Antigravity",
        }
    }

    fn login_hint(self) -> &'static str {
        match self {
            Self::Claude => "Нужен вход: выполните claude в терминале",
            Self::Codex => "Нужен вход: выполните `codex login` в терминале",
            Self::Agy => "Нужен вход: выполните `agy` в терминале",
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ConductorFile {
    agent: ConductorAgent,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConductorOption {
    id: ConductorAgent,
    found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    logged_in: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConductorConfig {
    agent: ConductorAgent,
    available: Vec<ConductorOption>,
}

/// Запись в `sessions.json`: кто вёл разговор и его идентификатор у этого CLI.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEntry {
    agent: ConductorAgent,
    id: String,
}

/// Старый формат хранил только строку — считаем её сессией Claude.
#[derive(Deserialize)]
#[serde(untagged)]
enum StoredSession {
    Legacy(String),
    Entry(SessionEntry),
}

/// Для Codex каждое сообщение — отдельный процесс `codex exec`; между ходами
/// дирижёр «жив» только как сохранённый идентификатор треда.
enum CodexTurn {
    Idle(Option<String>),
    Running(Arc<Mutex<Child>>),
}

struct CodexSlot {
    project: PathBuf,
    turn: Mutex<CodexTurn>,
}

enum Process {
    Stream {
        child: Arc<Mutex<Child>>,
        stdin: Arc<Mutex<ChildStdin>>,
        /// Antigravity не умеет брать инструкции флагом — они уходят в первое сообщение.
        instructions_pending: Arc<AtomicBool>,
    },
    Codex(Arc<CodexSlot>),
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

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Не удалось создать {}: {error}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(value)
        .map_err(|error| format!("Не удалось сериализовать {}: {error}", path.display()))?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, format!("{json}\n"))
        .and_then(|_| fs::rename(&temporary, path))
        .map_err(|error| format!("Не удалось записать {}: {error}", path.display()))
}

// ---- conductor.json ----

fn parse_conductor_config(text: &str) -> ConductorAgent {
    serde_json::from_str::<ConductorFile>(text)
        .map(|file| file.agent)
        .unwrap_or(ConductorAgent::Claude)
}

/// Нет файла или он битый — дирижирует Claude, как и до появления выбора.
fn read_conductor_agent() -> ConductorAgent {
    orchestra_path("conductor.json")
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|text| parse_conductor_config(&text))
        .unwrap_or(ConductorAgent::Claude)
}

fn write_conductor_agent(agent: ConductorAgent) -> Result<(), String> {
    let _guard = CONFIG_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "Не удалось заблокировать conductor.json".to_owned())?;
    let path = orchestra_path("conductor.json")?;
    write_json_atomic(&path, &ConductorFile { agent })
}

// ---- sessions.json ----

fn parse_sessions(text: &str) -> Result<HashMap<String, SessionEntry>, String> {
    let stored: HashMap<String, StoredSession> =
        serde_json::from_str(text).map_err(|error| format!("Некорректный sessions.json: {error}"))?;
    Ok(stored
        .into_iter()
        .map(|(project, session)| {
            let entry = match session {
                StoredSession::Legacy(id) => SessionEntry {
                    agent: ConductorAgent::Claude,
                    id,
                },
                StoredSession::Entry(entry) => entry,
            };
            (project, entry)
        })
        .collect())
}

fn read_sessions() -> Result<HashMap<String, SessionEntry>, String> {
    let path = orchestra_path("sessions.json")?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("Не удалось прочитать {}: {error}", path.display()))?;
    parse_sessions(&content)
}

fn save_session(project: &str, agent: ConductorAgent, session_id: &str) -> Result<(), String> {
    let _guard = SESSIONS_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "Не удалось заблокировать sessions.json".to_owned())?;
    let path = orchestra_path("sessions.json")?;
    let mut sessions = read_sessions()?;
    sessions.insert(
        project.to_owned(),
        SessionEntry {
            agent,
            id: session_id.to_owned(),
        },
    );
    write_json_atomic(&path, &sessions)
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
    write_json_atomic(&path, &sessions)
}

/// Сохранённая сессия годится, только если её вёл тот же агент.
fn saved_session(project: &str, agent: ConductorAgent) -> Result<Option<String>, String> {
    let _guard = SESSIONS_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "Не удалось заблокировать sessions.json".to_owned())?;
    Ok(read_sessions()?
        .remove(project)
        .filter(|entry| entry.agent == agent)
        .map(|entry| entry.id))
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

// ---- Команды запуска ----

fn instructions_file() -> Result<Option<PathBuf>, String> {
    let path = orchestra_path("INSTRUCTIONS.md")?;
    Ok(path.is_file().then_some(path))
}

/// Команда запуска выбранного CLI. `instructions` — путь к `INSTRUCTIONS.md`, если файл есть.
fn command_with_instructions(
    agent: ConductorAgent,
    project: &Path,
    session_id: Option<&str>,
    instructions: Option<&Path>,
) -> Result<Command, String> {
    let mut command = Command::new(agent.binary());
    command
        .current_dir(project)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match agent {
        ConductorAgent::Claude => {
            command.args([
                "--dangerously-skip-permissions",
                "-p",
                "--verbose",
                "--output-format",
                "stream-json",
                "--input-format",
                "stream-json",
            ]);
            if let Some(session_id) = session_id {
                command.args(["--resume", session_id]);
            }
            if let Some(instructions) = instructions {
                command.arg("--append-system-prompt-file").arg(instructions);
            }
            for key in ANTHROPIC_ENV {
                command.env_remove(key);
            }
        }
        ConductorAgent::Codex => {
            command.arg("exec");
            if let Some(session_id) = session_id {
                command.args(["resume", session_id]);
            }
            command.args([
                "--json",
                "--dangerously-bypass-approvals-and-sandbox",
                "--skip-git-repo-check",
            ]);
            if session_id.is_none() {
                command.arg("-C").arg(project);
            }
            if let Some(instructions) = instructions {
                let text = fs::read_to_string(instructions).map_err(|error| {
                    format!("Не удалось прочитать {}: {error}", instructions.display())
                })?;
                command.arg("-c").arg(format!(
                    "developer_instructions={}",
                    codex::toml_basic_string(&text)
                ));
            }
            command.arg("-");
        }
        ConductorAgent::Agy => {
            command.args([
                "-p",
                "--output-format",
                "stream-json",
                "--input-format",
                "stream-json",
                "--dangerously-skip-permissions",
            ]);
            if let Some(session_id) = session_id {
                command.args(["--conversation", session_id]);
            }
        }
    }
    command.env("PATH", crate::shell::tool_path());
    Ok(command)
}

fn command(
    agent: ConductorAgent,
    project: &Path,
    session_id: Option<&str>,
) -> Result<Command, String> {
    let instructions = instructions_file()?;
    command_with_instructions(agent, project, session_id, instructions.as_deref())
}

fn is_login_error(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "not logged in",
        "please log in",
        "authentication required",
        "invalid api key",
        "нужен вход",
        "codex login",
        "login required",
        "not authenticated",
        "not signed in",
        "please sign in",
        "sign in required",
        "agy login",
        "unauthorized",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn is_stale_session_error(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("no conversation found")
        || lower.contains("session not found")
        || lower.contains("conversation not found")
        || lower.contains("thread not found")
        || lower.contains("no rollout found")
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

/// Antigravity отдаёт stream-json того же вида; на всякий случай подкладываем
/// `session_id` из `conversation_id`, если в `system/init` его нет.
fn normalize_agy_event(mut event: Value) -> Value {
    let is_init = event.get("type").and_then(Value::as_str) == Some("system")
        && event.get("subtype").and_then(Value::as_str) == Some("init");
    if is_init
        && event.get("session_id").and_then(Value::as_str).is_none()
        && let Some(object) = event.as_object_mut()
    {
        let conversation = ["conversation_id", "conversationId", "sessionId"]
            .iter()
            .find_map(|key| object.get(*key).and_then(Value::as_str))
            .map(str::to_owned);
        if let Some(conversation) = conversation {
            object.insert("session_id".into(), Value::String(conversation));
        }
    }
    event
}

/// Сообщение пользователя в stdin потокового дирижёра (Claude и agy).
fn user_line(text: &str) -> Result<String, String> {
    serde_json::to_string(&json!({
        "type": "user",
        "message": { "role": "user", "content": text }
    }))
    .map_err(|error| format!("Не удалось сериализовать сообщение: {error}"))
}

fn with_instructions(text: &str) -> Result<String, String> {
    match instructions_file()? {
        Some(path) => {
            let instructions = fs::read_to_string(&path)
                .map_err(|error| format!("Не удалось прочитать {}: {error}", path.display()))?;
            Ok(format!(
                "Инструкции дирижёра (~/.orchestra/INSTRUCTIONS.md):\n\n{}\n\n---\n\n{text}",
                instructions.trim_end()
            ))
        }
        None => Ok(text.to_owned()),
    }
}

fn emit_event(app: &AppHandle, project: &str, event: Value) {
    let _ = app.emit(
        "conductor://event",
        ConductorEvent {
            project_path: project.to_owned(),
            event,
        },
    );
}

fn emit_exit(app: &AppHandle, project: &str, code: i32) {
    let _ = app.emit(
        "conductor://exit",
        ConductorExit {
            project_path: project.to_owned(),
            code,
        },
    );
}

fn wait_child(child: &Arc<Mutex<Child>>) -> i32 {
    loop {
        let status = child
            .lock()
            .ok()
            .and_then(|mut child| child.try_wait().ok())
            .flatten();
        if let Some(status) = status {
            return status.code().unwrap_or(-1);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn spawn_stderr_reader(
    app: AppHandle,
    project: String,
    agent: ConductorAgent,
    stderr: impl Read + Send + 'static,
    stale_session: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        read_lossy_lines(stderr, |line| {
            if is_stale_session_error(&line) {
                stale_session.store(true, Ordering::Release);
            }
            let message = if is_login_error(&line) {
                agent.login_hint().to_owned()
            } else {
                line
            };
            emit_event(&app, &project, json!({ "type": "stderr", "message": message }));
        });
    })
}

/// Потоковый дирижёр (Claude, agy): один процесс на весь разговор.
fn spawn_stream_readers(
    app: AppHandle,
    state: ConductorState,
    project: String,
    agent: ConductorAgent,
    stdout: impl Read + Send + 'static,
    stderr: impl Read + Send + 'static,
    child: Arc<Mutex<Child>>,
    resumed: bool,
) {
    let initialized = Arc::new(AtomicBool::new(false));
    let stale_session = Arc::new(AtomicBool::new(false));
    let stderr_thread = spawn_stderr_reader(
        app.clone(),
        project.clone(),
        agent,
        stderr,
        stale_session.clone(),
    );

    let reader_initialized = initialized.clone();
    std::thread::spawn(move || {
        read_lossy_lines(stdout, |line| {
            let mut event = serde_json::from_str::<Value>(&line).unwrap_or(Value::String(line));
            if agent == ConductorAgent::Agy {
                event = normalize_agy_event(event);
            }
            if let Some(session_id) = session_from_event(&event) {
                reader_initialized.store(true, Ordering::Release);
                if let Err(error) = save_session(&project, agent, session_id) {
                    eprintln!("conductor session: {error}");
                }
            }
            emit_event(&app, &project, event);
        });
        let code = wait_child(&child);
        let _ = stderr_thread.join();
        let should_emit = if let Ok(mut processes) = state.0.lock() {
            match processes.get(&project) {
                Some(Process::Stream { child: current, .. }) if Arc::ptr_eq(current, &child) => {
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
            emit_exit(&app, &project, code);
        }
    });
}

/// Один ход Codex: процесс живёт до конца ответа, затем слот возвращается в `Idle`.
/// `conductor://exit` уходит только если пользователь остановил дирижёра.
fn spawn_codex_turn(
    app: AppHandle,
    state: ConductorState,
    project: String,
    slot: Arc<CodexSlot>,
    stdout: impl Read + Send + 'static,
    stderr: impl Read + Send + 'static,
    child: Arc<Mutex<Child>>,
    resumed: Option<String>,
) {
    let stale_session = Arc::new(AtomicBool::new(false));
    let stderr_thread = spawn_stderr_reader(
        app.clone(),
        project.clone(),
        ConductorAgent::Codex,
        stderr,
        stale_session.clone(),
    );
    std::thread::spawn(move || {
        let mut normalizer = CodexNormalizer::default();
        let mut session_id = resumed.clone();
        let mut initialized = false;
        let mut completed = false;
        read_lossy_lines(stdout, |line| {
            for event in normalizer.normalize(&line) {
                if let Some(found) = session_from_event(&event) {
                    initialized = true;
                    session_id = Some(found.to_owned());
                    if let Err(error) = save_session(&project, ConductorAgent::Codex, found) {
                        eprintln!("conductor session: {error}");
                    }
                }
                if event.get("type").and_then(Value::as_str) == Some("result") {
                    completed = true;
                }
                emit_event(&app, &project, event);
            }
        });
        let code = wait_child(&child);
        let _ = stderr_thread.join();
        if resumed.is_some() && !initialized && stale_session.load(Ordering::Acquire) {
            session_id = None;
            if let Err(error) = remove_session(&project) {
                eprintln!("conductor stale session: {error}");
            }
        }
        if code != 0 && !completed {
            emit_event(
                &app,
                &project,
                json!({ "type": "stderr", "message": format!("Codex завершился с кодом {code}") }),
            );
        }
        let still_registered = state
            .0
            .lock()
            .map(|processes| {
                matches!(
                    processes.get(&project),
                    Some(Process::Codex(current)) if Arc::ptr_eq(current, &slot)
                )
            })
            .unwrap_or(false);
        if still_registered {
            if let Ok(mut turn) = slot.turn.lock() {
                *turn = CodexTurn::Idle(session_id);
            }
        } else {
            emit_exit(&app, &project, code);
        }
    });
}

fn take_pipes(
    child: &mut Child,
    agent: ConductorAgent,
) -> Result<
    (
        ChildStdin,
        std::process::ChildStdout,
        std::process::ChildStderr,
    ),
    String,
> {
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| format!("{} не открыл stdin", agent.title()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("{} не открыл stdout", agent.title()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("{} не открыл stderr", agent.title()))?;
    Ok((stdin, stdout, stderr))
}

// ---- Доступность CLI ----

fn find_binary(name: &str) -> bool {
    crate::shell::tool_path()
        .split(':')
        .filter(|dir| !dir.is_empty())
        .any(|dir| Path::new(dir).join(name).is_file())
}

fn probe_succeeds(binary: &str, args: &[&str]) -> bool {
    let mut command = Command::new(binary);
    command
        .args(args)
        .env("PATH", crate::shell::tool_path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for key in ANTHROPIC_ENV {
        command.env_remove(key);
    }
    command.status().map(|status| status.success()).unwrap_or(false)
}

fn home_file_exists(parts: &[&str]) -> bool {
    dirs::home_dir()
        .map(|home| parts.iter().fold(home, |path, part| path.join(part)))
        .is_some_and(|path| path.is_file())
}

/// Признак входа: сначала файлы учётных данных (быстро), потом сам CLI.
fn logged_in(agent: ConductorAgent) -> Option<bool> {
    match agent {
        ConductorAgent::Claude => Some(
            home_file_exists(&[".claude", ".credentials.json"])
                || probe_succeeds("claude", &["auth", "status"]),
        ),
        ConductorAgent::Codex => Some(
            home_file_exists(&[".codex", "auth.json"])
                || probe_succeeds("codex", &["login", "status"]),
        ),
        ConductorAgent::Agy => {
            let found = home_file_exists(&[".gemini", "antigravity-cli", "antigravity-oauth-token"])
                || home_file_exists(&[".gemini", "oauth_creds.json"]);
            found.then_some(true)
        }
    }
}

fn conductor_options() -> Vec<ConductorOption> {
    ConductorAgent::ALL
        .iter()
        .map(|&agent| {
            let found = find_binary(agent.binary());
            ConductorOption {
                id: agent,
                found,
                logged_in: if found { logged_in(agent) } else { None },
            }
        })
        .collect()
}

// ---- Команды Tauri ----

#[tauri::command(async)]
pub fn conductor_config() -> Result<ConductorConfig, String> {
    Ok(ConductorConfig {
        agent: read_conductor_agent(),
        available: conductor_options(),
    })
}

/// Запоминает выбор; действующий дирижёр не трогается — выбор применится при следующем запуске.
#[tauri::command(async)]
pub fn set_conductor(agent: String) -> Result<(), String> {
    let agent = ConductorAgent::parse(&agent)
        .ok_or_else(|| format!("Неизвестный дирижёр: {agent}"))?;
    write_conductor_agent(agent)
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

    let agent = read_conductor_agent();
    let session_id = saved_session(&project_path, agent)?;

    if agent == ConductorAgent::Codex {
        let slot = Arc::new(CodexSlot {
            project: project.to_path_buf(),
            turn: Mutex::new(CodexTurn::Idle(session_id.clone())),
        });
        let mut processes = state
            .0
            .lock()
            .map_err(|_| "Не удалось заблокировать список дирижёров".to_owned())?;
        if processes.contains_key(&project_path) {
            return Err("Дирижёр для этого проекта уже запущен".to_owned());
        }
        processes.insert(project_path, Process::Codex(slot));
        return Ok(ConductorStart { session_id });
    }

    let mut child = command(agent, project, session_id.as_deref())?
        .spawn()
        .map_err(|error| format!("Не удалось запустить {}: {error}", agent.binary()))?;
    std::thread::sleep(Duration::from_secs(1));
    if child
        .try_wait()
        .map_err(|error| format!("Не удалось проверить {}: {error}", agent.binary()))?
        .is_some()
    {
        let mut output = String::new();
        if let Some(mut stderr) = child.stderr.take() {
            let _ = stderr.read_to_string(&mut output);
        }
        if is_login_error(&output) {
            return Err(agent.login_hint().to_owned());
        }
        return Err(format!(
            "{} завершился при запуске: {}",
            agent.title(),
            output.trim()
        ));
    }

    let (stdin, stdout, stderr) = take_pipes(&mut child, agent)?;
    let child = Arc::new(Mutex::new(child));
    let instructions_pending = Arc::new(AtomicBool::new(
        agent == ConductorAgent::Agy && session_id.is_none() && instructions_file()?.is_some(),
    ));
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
            Process::Stream {
                child: child.clone(),
                stdin: Arc::new(Mutex::new(stdin)),
                instructions_pending,
            },
        );
    }
    spawn_stream_readers(
        app,
        state.inner().clone(),
        project_path,
        agent,
        stdout,
        stderr,
        child,
        session_id.is_some(),
    );
    Ok(ConductorStart { session_id })
}

fn send_codex(
    app: AppHandle,
    state: ConductorState,
    project_path: String,
    slot: Arc<CodexSlot>,
    text: String,
) -> Result<(), String> {
    let mut turn = slot
        .turn
        .lock()
        .map_err(|_| "Не удалось заблокировать ход Codex".to_owned())?;
    let session_id = match &*turn {
        CodexTurn::Running(_) => {
            return Err("Codex ещё отвечает — дождитесь конца хода".to_owned());
        }
        CodexTurn::Idle(session_id) => session_id.clone(),
    };
    let mut child = command(ConductorAgent::Codex, &slot.project, session_id.as_deref())?
        .spawn()
        .map_err(|error| format!("Не удалось запустить codex: {error}"))?;
    let (mut stdin, stdout, stderr) = take_pipes(&mut child, ConductorAgent::Codex)?;
    if let Err(error) = stdin.write_all(text.as_bytes()).and_then(|_| stdin.flush()) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!("Не удалось отправить сообщение: {error}"));
    }
    drop(stdin);
    let child = Arc::new(Mutex::new(child));
    *turn = CodexTurn::Running(child.clone());
    drop(turn);
    spawn_codex_turn(
        app,
        state,
        project_path,
        slot,
        stdout,
        stderr,
        child,
        session_id,
    );
    Ok(())
}

#[tauri::command(async)]
pub fn conductor_send(
    app: AppHandle,
    state: State<'_, ConductorState>,
    project_path: String,
    text: String,
) -> Result<(), String> {
    enum Target {
        Stream(Arc<Mutex<ChildStdin>>, Arc<AtomicBool>),
        Codex(Arc<CodexSlot>),
    }
    let target = {
        let processes = state
            .0
            .lock()
            .map_err(|_| "Не удалось заблокировать список дирижёров".to_owned())?;
        match processes.get(&project_path) {
            Some(Process::Stream {
                stdin,
                instructions_pending,
                ..
            }) => Target::Stream(stdin.clone(), instructions_pending.clone()),
            Some(Process::Codex(slot)) => Target::Codex(slot.clone()),
            None => return Err("Дирижёр для проекта не запущен".to_owned()),
        }
    };
    match target {
        Target::Codex(slot) => send_codex(app, state.inner().clone(), project_path, slot, text),
        Target::Stream(stdin, instructions_pending) => {
            let content = if instructions_pending.swap(false, Ordering::AcqRel) {
                with_instructions(&text)?
            } else {
                text
            };
            let line = user_line(&content)?;
            let mut stdin = stdin
                .lock()
                .map_err(|_| "Не удалось заблокировать stdin дирижёра".to_owned())?;
            writeln!(stdin, "{line}")
                .map_err(|error| format!("Не удалось отправить сообщение: {error}"))?;
            stdin
                .flush()
                .map_err(|error| format!("Не удалось отправить сообщение: {error}"))
        }
    }
}

#[tauri::command(async)]
pub fn conductor_stop(
    app: AppHandle,
    state: State<'_, ConductorState>,
    project_path: String,
) -> Result<(), String> {
    let process = state
        .0
        .lock()
        .map_err(|_| "Не удалось заблокировать список дирижёров".to_owned())?
        .remove(&project_path)
        .ok_or_else(|| "Дирижёр для проекта не запущен".to_owned())?;
    let child = match process {
        Process::Stream { child, .. } => child,
        Process::Codex(slot) => {
            let turn = slot
                .turn
                .lock()
                .map_err(|_| "Не удалось заблокировать ход Codex".to_owned())?;
            match &*turn {
                CodexTurn::Running(child) => child.clone(),
                CodexTurn::Idle(_) => {
                    emit_exit(&app, &project_path, 0);
                    return Ok(());
                }
            }
        }
    };
    child
        .lock()
        .map_err(|_| "Не удалось заблокировать процесс дирижёра".to_owned())?
        .kill()
        .map_err(|error| format!("Не удалось остановить дирижёра: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    fn args(command: &Command) -> Vec<String> {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

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
    fn sessions_migrate_legacy_strings_to_claude() {
        let parsed = parse_sessions(
            r#"{
              "/old/project": "sess-legacy",
              "/new/project": { "agent": "codex", "id": "thread-1" }
            }"#,
        )
        .unwrap();
        assert_eq!(
            parsed["/old/project"],
            SessionEntry {
                agent: ConductorAgent::Claude,
                id: "sess-legacy".into()
            }
        );
        assert_eq!(
            parsed["/new/project"],
            SessionEntry {
                agent: ConductorAgent::Codex,
                id: "thread-1".into()
            }
        );
        let serialized = serde_json::to_value(&parsed["/new/project"]).unwrap();
        assert_eq!(serialized, json!({ "agent": "codex", "id": "thread-1" }));
        assert!(parse_sessions("[1,2]").is_err());
    }

    #[test]
    fn conductor_config_defaults_to_claude() {
        assert_eq!(parse_conductor_config(""), ConductorAgent::Claude);
        assert_eq!(parse_conductor_config("{\"agent\":\"nope\"}"), ConductorAgent::Claude);
        assert_eq!(parse_conductor_config("{\"agent\":\"agy\"}"), ConductorAgent::Agy);
        assert_eq!(
            serde_json::to_string(&ConductorFile {
                agent: ConductorAgent::Codex
            })
            .unwrap(),
            "{\"agent\":\"codex\"}"
        );
        assert_eq!(ConductorAgent::parse(" codex "), Some(ConductorAgent::Codex));
        assert_eq!(ConductorAgent::parse("gemini"), None);
    }

    #[test]
    fn claude_command_is_unchanged() {
        let project = Path::new("/tmp/project");
        let instructions = Path::new("/home/x/.orchestra/INSTRUCTIONS.md");
        let fresh =
            command_with_instructions(ConductorAgent::Claude, project, None, None).unwrap();
        assert_eq!(fresh.get_program(), OsStr::new("claude"));
        assert_eq!(
            args(&fresh),
            [
                "--dangerously-skip-permissions",
                "-p",
                "--verbose",
                "--output-format",
                "stream-json",
                "--input-format",
                "stream-json",
            ]
        );
        let resumed = command_with_instructions(
            ConductorAgent::Claude,
            project,
            Some("sess-1"),
            Some(instructions),
        )
        .unwrap();
        assert_eq!(
            args(&resumed),
            [
                "--dangerously-skip-permissions",
                "-p",
                "--verbose",
                "--output-format",
                "stream-json",
                "--input-format",
                "stream-json",
                "--resume",
                "sess-1",
                "--append-system-prompt-file",
                "/home/x/.orchestra/INSTRUCTIONS.md",
            ]
        );
        let removed: Vec<_> = resumed
            .get_envs()
            .filter(|(_, value)| value.is_none())
            .map(|(key, _)| key.to_string_lossy().into_owned())
            .collect();
        for key in ANTHROPIC_ENV {
            assert!(removed.contains(&key.to_owned()), "{key} должен быть снят");
        }
        assert_eq!(resumed.get_current_dir(), Some(project));
    }

    #[test]
    fn codex_and_agy_commands() {
        let project = Path::new("/tmp/project");
        let fresh = command_with_instructions(ConductorAgent::Codex, project, None, None).unwrap();
        assert_eq!(fresh.get_program(), OsStr::new("codex"));
        assert_eq!(
            args(&fresh),
            [
                "exec",
                "--json",
                "--dangerously-bypass-approvals-and-sandbox",
                "--skip-git-repo-check",
                "-C",
                "/tmp/project",
                "-",
            ]
        );
        let resumed =
            command_with_instructions(ConductorAgent::Codex, project, Some("thread-9"), None)
                .unwrap();
        assert_eq!(
            args(&resumed),
            [
                "exec",
                "resume",
                "thread-9",
                "--json",
                "--dangerously-bypass-approvals-and-sandbox",
                "--skip-git-repo-check",
                "-",
            ]
        );

        let dir = std::env::temp_dir().join(format!("takt-instr-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("INSTRUCTIONS.md");
        fs::write(&file, "# Правила\nстрока \"два\"\n").unwrap();
        let with_file =
            command_with_instructions(ConductorAgent::Codex, project, None, Some(&file)).unwrap();
        let listed = args(&with_file);
        let position = listed.iter().position(|arg| arg == "-c").unwrap();
        assert_eq!(
            listed[position + 1],
            "developer_instructions=\"# Правила\\nстрока \\\"два\\\"\\n\""
        );
        assert_eq!(listed.last().map(String::as_str), Some("-"));
        let _ = fs::remove_dir_all(dir);

        let agy = command_with_instructions(ConductorAgent::Agy, project, None, None).unwrap();
        assert_eq!(agy.get_program(), OsStr::new("agy"));
        assert_eq!(
            args(&agy),
            [
                "-p",
                "--output-format",
                "stream-json",
                "--input-format",
                "stream-json",
                "--dangerously-skip-permissions",
            ]
        );
        let agy_resumed =
            command_with_instructions(ConductorAgent::Agy, project, Some("conv-3"), None).unwrap();
        assert_eq!(args(&agy_resumed)[6..], ["--conversation", "conv-3"]);
    }

    #[test]
    fn login_errors_for_every_cli() {
        assert!(is_login_error("Not logged in · Please run /login"));
        assert!(is_login_error("Error: Not logged in. Run `codex login` to authenticate."));
        assert!(is_login_error("Not authenticated: run agy login first"));
        assert!(is_login_error("401 Unauthorized"));
        assert!(!is_login_error("fatal: not a git repository"));
        assert_eq!(
            ConductorAgent::Codex.login_hint(),
            "Нужен вход: выполните `codex login` в терминале"
        );
        assert_eq!(
            ConductorAgent::Agy.login_hint(),
            "Нужен вход: выполните `agy` в терминале"
        );
    }

    #[test]
    fn agy_init_gets_session_id_from_conversation() {
        let event = normalize_agy_event(json!({
            "type": "system", "subtype": "init", "conversation_id": "conv-7"
        }));
        assert_eq!(session_from_event(&event), Some("conv-7"));
        let kept = normalize_agy_event(json!({
            "type": "system", "subtype": "init", "session_id": "s-1", "conversation_id": "c-1"
        }));
        assert_eq!(session_from_event(&kept), Some("s-1"));
        let other = normalize_agy_event(json!({ "type": "assistant" }));
        assert_eq!(other, json!({ "type": "assistant" }));
    }

    fn live_project() -> (PathBuf, String) {
        let project = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .canonicalize()
            .unwrap();
        let key = project.to_string_lossy().into_owned();
        (project, key)
    }

    #[test]
    #[ignore = "запускает реальный Claude Code и расходует подписочный лимит"]
    fn live_conductor_stream() {
        let (project, project_key) = live_project();
        let mut child = command(ConductorAgent::Claude, &project, None)
            .expect("command")
            .spawn()
            .expect("spawn claude");
        let mut stdin = child.stdin.take().expect("stdin");
        writeln!(
            stdin,
            "{}",
            user_line("Скажи одно слово: готов").unwrap()
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
                save_session(&project_key, ConductorAgent::Claude, found).expect("save session");
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
            Some(&SessionEntry {
                agent: ConductorAgent::Claude,
                id: session_id
            })
        );
    }

    #[test]
    #[ignore = "запускает реальный Codex CLI и расходует подписочный лимит"]
    fn live_codex() {
        let (project, _) = live_project();
        let mut child = command(ConductorAgent::Codex, &project, None)
            .expect("command")
            .spawn()
            .expect("spawn codex");
        let mut stdin = child.stdin.take().expect("stdin");
        stdin.write_all("Скажи одно слово: готов".as_bytes()).unwrap();
        drop(stdin);
        let output = child.wait_with_output().expect("codex output");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut normalizer = CodexNormalizer::default();
        let events: Vec<Value> = stdout
            .lines()
            .flat_map(|line| normalizer.normalize(line))
            .collect();
        println!("{}", serde_json::to_string_pretty(&events).unwrap());
        assert!(events.iter().any(|event| session_from_event(event).is_some()));
        assert!(events.iter().any(|event| event["type"] == "assistant"));
        assert!(events.iter().any(|event| event["type"] == "result"));
    }

    #[test]
    #[ignore = "запускает реальный Antigravity CLI и расходует подписочный лимит"]
    fn live_agy() {
        let (project, _) = live_project();
        let mut child = command(ConductorAgent::Agy, &project, None)
            .expect("command")
            .spawn()
            .expect("spawn agy");
        let mut stdin = child.stdin.take().expect("stdin");
        writeln!(stdin, "{}", user_line("Скажи одно слово: готов").unwrap()).unwrap();
        drop(stdin);
        let output = child.wait_with_output().expect("agy output");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let events: Vec<Value> = stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .map(normalize_agy_event)
            .collect();
        println!("{}", serde_json::to_string_pretty(&events).unwrap());
        assert!(events.iter().any(|event| session_from_event(event).is_some()));
        assert!(events.iter().any(|event| event["type"] == "assistant"));
    }
}
