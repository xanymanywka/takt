use std::collections::{HashMap, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, mpsc};
use std::time::Duration;

use chrono::{DateTime, Utc};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use portable_pty::{Child, CommandBuilder, PtySize, native_pty_system};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tauri::{AppHandle, Emitter};

use crate::platform;
use crate::pty::decode_available;
use crate::shell::{quote, run_shell};

static CHECKBOX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*-\s*\[([ xX])\]\s*(.+?)\s*$").expect("valid regex"));
static MODEL_RES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"(?i)\b(gpt-[a-z0-9._-]+(?:\s+codex)?)\b",
        r"(?i)\b(claude[- ](?:opus|sonnet|haiku)[a-z0-9._ -]*)\b",
        r"(?i)\b(gemini\s*[0-9][a-z0-9._ -]*)\b",
    ]
    .into_iter()
    .map(|pattern| Regex::new(pattern).expect("valid regex"))
    .collect()
});
static LIMIT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)hit your usage limit|purchase more credits|usage limit reached|quota (?:limit )?(?:exceeded|reached|exhausted)|exhausted your (?:daily |weekly )?quota|(?:five.hour|weekly) limit (?:reached|exceeded|hit)|out of (?:model )?quota|RESOURCE_EXHAUSTED|reached your (?:daily |weekly )?(?:usage )?limit",
    )
    .expect("valid regex")
});
static ASKING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:\?\s*$|\(y/n\)|\bAllow\b|Разрешить)").expect("valid regex")
});
static ANSI_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1b\\))").expect("valid regex")
});
static PTY_JOBS: LazyLock<Mutex<HashMap<String, PtyJob>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
const PTY_HISTORY_LINES: usize = 2_000;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentKind {
    Codex,
    Gemini,
    Claude,
}

impl AgentKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Gemini => "gemini",
            Self::Claude => "claude",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Runner {
    #[default]
    Tmux,
    Pty,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub text: String,
    pub done: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub name: String,
    pub agent: AgentKind,
    pub worktree: String,
    pub task: String,
    pub started_at: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handed_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub takeover_of: Option<String>,
    pub steps: Vec<Step>,
    pub last_line: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawJob {
    agent: AgentKind,
    worktree: String,
    #[serde(default)]
    task: String,
    #[serde(default)]
    started_at: serde_json::Value,
    #[serde(default = "running_state")]
    state: String,
    handed_to: Option<String>,
    takeover_of: Option<String>,
    /// Модель, явно переданная при запуске (`agents run … --model`).
    model: Option<String>,
    #[serde(default)]
    runner: Runner,
}

/// Экран PTY-задачи в памяти ядра: кольцевой буфер последних `PTY_HISTORY_LINES` строк
/// без ANSI-последовательностей. Это не эмулятор терминала — курсорные перемещения TUI
/// не воспроизводятся, но текст ответов исполнителя и вопросы «y/n» в нём видны.
#[derive(Default)]
struct ScreenBuffer {
    lines: VecDeque<String>,
    current: String,
    /// Хвост незавершённой escape-последовательности с границы предыдущего чтения.
    escape_tail: String,
}

impl ScreenBuffer {
    /// Escape-последовательность, разрезанная границей чтения из PTY, иначе просочилась бы
    /// в текст как `[38;2;153;153;153m`; её хвост откладываем до следующего куска.
    fn split_incomplete_escape(text: &str) -> (&str, &str) {
        const MAX_ESCAPE_LEN: usize = 64;
        let Some(start) = text.rfind('\u{1b}') else {
            return (text, "");
        };
        let tail = &text[start..];
        if tail.len() <= MAX_ESCAPE_LEN && !ANSI_RE.is_match(tail) {
            (&text[..start], tail)
        } else {
            (text, "")
        }
    }

    fn push(&mut self, text: &str) {
        let joined = format!("{}{text}", self.escape_tail);
        let (complete, incomplete) = Self::split_incomplete_escape(&joined);
        self.escape_tail = incomplete.to_owned();
        let clean = ANSI_RE.replace_all(complete, "");
        for character in clean.chars() {
            match character {
                '\r' | '\n' => {
                    if !self.current.is_empty() {
                        self.finish_line();
                    }
                }
                '\u{8}' => {
                    self.current.pop();
                }
                character if !character.is_control() => {
                    self.current.push(character);
                }
                _ => {}
            }
        }
    }

    fn finish_line(&mut self) {
        self.lines.push_back(std::mem::take(&mut self.current));
        while self.lines.len() > PTY_HISTORY_LINES {
            self.lines.pop_front();
        }
    }

    fn render(&self, lines: usize) -> String {
        let mut all = self.lines.iter().map(String::as_str).collect::<Vec<_>>();
        if !self.current.is_empty() {
            all.push(&self.current);
        }
        let start = all.len().saturating_sub(lines);
        all[start..].join("\n")
    }
}

struct PtyJob {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    screen: Arc<Mutex<ScreenBuffer>>,
}

fn running_state() -> String {
    "running".to_owned()
}

fn orchestra_home() -> Result<PathBuf, String> {
    let home = platform::orchestra_home();
    if home == Path::new(".orchestra") {
        Err("Не удалось определить домашнюю папку".to_owned())
    } else {
        Ok(home)
    }
}

fn agents_command(args: &[String]) -> Result<String, String> {
    let script = orchestra_home()?.join("agents.py");
    let mut parts = vec![
        platform::python().to_owned(),
        "-I".to_owned(),
        quote(&script.to_string_lossy()),
    ];
    parts.extend(args.iter().map(|arg| quote(arg)));
    Ok(parts.join(" "))
}

/// Заголовок раздела приёмки в ТЗ: исполнители пишут на русском, украинском или английском.
fn is_acceptance_header(line: &str) -> bool {
    let header = line.trim().trim_start_matches('#').trim().to_lowercase();
    line.trim_start().starts_with("## ")
        && matches!(
            header.as_str(),
            "приёмка" | "приемка" | "приймання" | "acceptance" | "acceptance criteria"
        )
}

fn parse_steps(markdown: &str) -> Vec<Step> {
    let mut in_acceptance = false;
    let mut steps = Vec::new();
    for line in markdown.lines() {
        if is_acceptance_header(line) {
            in_acceptance = true;
            continue;
        }
        if in_acceptance && line.starts_with("## ") {
            break;
        }
        if in_acceptance && let Some(captures) = CHECKBOX_RE.captures(line) {
            steps.push(Step {
                done: !captures[1].eq(" "),
                text: captures[2].trim().to_owned(),
            });
        } else if in_acceptance
            && !line.trim().is_empty()
            && line.starts_with(char::is_whitespace)
            && let Some(step) = steps.last_mut()
        {
            step.text.push(' ');
            step.text.push_str(line.trim());
        }
    }
    steps
}

fn read_steps(raw: &RawJob) -> Vec<Step> {
    if raw.task.is_empty() {
        return Vec::new();
    }
    let path = Path::new(&raw.worktree).join(&raw.task);
    fs::read_to_string(path)
        .map(|markdown| parse_steps(&markdown))
        .unwrap_or_default()
}

fn started_at(value: &serde_json::Value) -> String {
    if let Some(value) = value.as_str() {
        return value.to_owned();
    }
    value
        .as_i64()
        .or_else(|| value.as_f64().map(|seconds| seconds.trunc() as i64))
        .and_then(|seconds| DateTime::<Utc>::from_timestamp(seconds, 0))
        .map(|date| date.to_rfc3339())
        .unwrap_or_default()
}

fn last_nonempty(screen: &str) -> String {
    screen
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_owned())
        .unwrap_or_default()
}

#[cfg(test)]
fn has_standalone_ok(screen: &str) -> bool {
    screen.lines().any(|line| {
        line.trim_matches(|character: char| !character.is_alphanumeric())
            .eq_ignore_ascii_case("ok")
    })
}

fn model_from_screen(agent: AgentKind, screen: &str) -> String {
    for line in screen.lines().rev() {
        for regex in MODEL_RES.iter() {
            if let Some(found) = regex.captures(line).and_then(|capture| capture.get(1)) {
                return found.as_str().trim().to_owned();
            }
        }
    }
    match agent {
        AgentKind::Codex => "gpt-5-codex",
        AgentKind::Gemini => "Gemini 2.5",
        AgentKind::Claude => "Claude",
    }
    .to_owned()
}

fn computed_state(raw: &str, steps: &[Step], screen: &str, last_line: &str) -> String {
    if raw == "limit" || LIMIT_RE.is_match(screen) {
        "limit".to_owned()
    } else if raw == "done"
        || (raw == "exited" && !steps.is_empty() && steps.iter().all(|step| step.done))
    {
        "done".to_owned()
    } else if matches!(raw, "killed" | "failed" | "exited") {
        raw.to_owned()
    } else if matches!(raw, "ended" | "handed-off") {
        "exited".to_owned()
    } else if ASKING_RE.is_match(last_line) {
        "asking".to_owned()
    } else {
        "running".to_owned()
    }
}

fn read_jobs(path: &Path) -> Result<HashMap<String, RawJob>, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("Не удалось прочитать {}: {error}", path.display()))?;
    serde_json::from_str(&content).map_err(|error| format!("Некорректный jobs.json: {error}"))
}

fn mutate_registry<T>(
    path: &Path,
    mutate: impl FnOnce(&mut Map<String, Value>) -> Result<T, String>,
) -> Result<T, String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("Некорректный путь реестра: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Не удалось создать {}: {error}", parent.display()))?;
    let lock_path = parent.join(".lock");
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(|error| format!("Не удалось открыть {}: {error}", lock_path.display()))?;
    lock.lock()
        .map_err(|error| format!("Не удалось заблокировать {}: {error}", lock_path.display()))?;

    let result = (|| {
        let mut registry = if path.exists() {
            let content = fs::read_to_string(path)
                .map_err(|error| format!("Не удалось прочитать {}: {error}", path.display()))?;
            serde_json::from_str::<Map<String, Value>>(&content)
                .map_err(|error| format!("Некорректный jobs.json: {error}"))?
        } else {
            Map::new()
        };
        let value = mutate(&mut registry)?;
        let content = serde_json::to_string_pretty(&registry)
            .map_err(|error| format!("Не удалось сериализовать jobs.json: {error}"))?;
        fs::write(path, content)
            .map_err(|error| format!("Не удалось записать {}: {error}", path.display()))?;
        Ok(value)
    })();
    let unlock = lock
        .unlock()
        .map_err(|error| format!("Не удалось разблокировать {}: {error}", lock_path.display()));
    match (result, unlock) {
        (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        (Ok(value), Ok(())) => Ok(value),
    }
}

fn register_pty_job(
    path: &Path,
    name: &str,
    agent: AgentKind,
    worktree: &Path,
    task: &str,
    model: Option<&str>,
) -> Result<(), String> {
    mutate_registry(path, |registry| {
        if registry.contains_key(name) {
            return Err(format!("Задача уже существует: {name}"));
        }
        let mut entry = json!({
            "agent": agent.as_str(),
            "worktree": worktree.to_string_lossy(),
            "task": task,
            "startedAt": Utc::now().timestamp(),
            "state": "running",
            "runner": "pty"
        });
        if let Some(model) = model {
            entry["model"] = Value::String(model.to_owned());
        }
        registry.insert(name.to_owned(), entry);
        Ok(())
    })
}

fn set_pty_job_state(path: &Path, name: &str, state: &str) -> Result<(), String> {
    mutate_registry(path, |registry| {
        if let Some(entry) = registry.get_mut(name).and_then(Value::as_object_mut) {
            entry.insert("state".to_owned(), Value::String(state.to_owned()));
        }
        Ok(())
    })
}

fn remove_pty_job(path: &Path, name: &str) -> Result<(), String> {
    mutate_registry(path, |registry| {
        registry.remove(name);
        Ok(())
    })
}

fn runner_for(name: &str) -> Result<Runner, String> {
    let path = orchestra_home()?.join("jobs.json");
    if !path.exists() {
        return Err(format!("Задача не найдена: {name}"));
    }
    read_jobs(&path)?
        .remove(name)
        .map(|job| job.runner)
        .ok_or_else(|| format!("Задача не найдена: {name}"))
}

fn pty_screen(name: &str, lines: usize) -> Result<String, String> {
    let screen = PTY_JOBS
        .lock()
        .map_err(|_| "Не удалось заблокировать список PTY-задач".to_owned())?
        .get(name)
        .map(|job| job.screen.clone())
        .ok_or_else(|| format!("PTY-задача не найдена: {name}"))?;
    let rendered = screen
        .lock()
        .map_err(|_| format!("Не удалось прочитать экран PTY-задачи: {name}"))?
        .render(lines);
    Ok(rendered)
}

fn pty_send(name: &str, text: &str) -> Result<(), String> {
    let writer = PTY_JOBS
        .lock()
        .map_err(|_| "Не удалось заблокировать список PTY-задач".to_owned())?
        .get(name)
        .map(|job| job.writer.clone())
        .ok_or_else(|| format!("PTY-задача не найдена: {name}"))?;
    let mut writer = writer
        .lock()
        .map_err(|_| format!("Не удалось заблокировать ввод PTY-задачи: {name}"))?;
    writer
        .write_all(text.as_bytes())
        .and_then(|_| writer.write_all(b"\r"))
        .and_then(|_| writer.flush())
        .map_err(|error| format!("Не удалось отправить ответ PTY-задаче {name}: {error}"))
}

fn kill_pty_job(name: &str, registry_path: &Path) -> Result<(), String> {
    let process = PTY_JOBS
        .lock()
        .map_err(|_| "Не удалось заблокировать список PTY-задач".to_owned())?
        .remove(name);
    if let Some(process) = process {
        let mut child = process
            .child
            .lock()
            .map_err(|_| format!("Не удалось заблокировать PTY-задачу: {name}"))?;
        let running = child
            .try_wait()
            .map_err(|error| format!("Не удалось проверить PTY-задачу {name}: {error}"))?
            .is_none();
        if running {
            child
                .kill()
                .map_err(|error| format!("Не удалось остановить PTY-задачу {name}: {error}"))?;
        }
    }
    remove_pty_job(registry_path, name)
}

fn task_prompt(task_file: &str) -> String {
    format!(
        "Прочитай {task_file} (и docs/SPEC.md, если он есть) и выполни задачу целиком в этой ворктри. \
Коммиты только от xanymanywka, без Co-Authored-By. Продакшн не деплоить. \
В конце дай отчёт по пунктам приёмки с выводом команд."
    )
}

fn agent_command(
    agent: AgentKind,
    model: Option<&str>,
    prompt: &str,
) -> (&'static str, Vec<String>) {
    let mut args = match agent {
        AgentKind::Codex => vec![
            "-c".to_owned(),
            "model_reasoning_effort=high".to_owned(),
            "--dangerously-bypass-approvals-and-sandbox".to_owned(),
        ],
        AgentKind::Gemini => vec![
            "--dangerously-skip-permissions".to_owned(),
            "--effort".to_owned(),
            "high".to_owned(),
            "-i".to_owned(),
        ],
        AgentKind::Claude => vec!["--dangerously-skip-permissions".to_owned()],
    };
    if let Some(model) = model {
        args.extend([
            match agent {
                AgentKind::Codex => "-m",
                AgentKind::Gemini | AgentKind::Claude => "--model",
            }
            .to_owned(),
            model.to_owned(),
        ]);
    }
    args.push(prompt.to_owned());
    let program = match agent {
        AgentKind::Codex => "codex",
        AgentKind::Gemini => "agy",
        AgentKind::Claude => "claude",
    };
    (program, args)
}

fn trust_agy_workspace(worktree: &Path) -> Result<(), String> {
    let Some(home) = dirs::home_dir() else {
        return Err("Не удалось определить домашнюю папку".to_owned());
    };
    let path = home.join(".gemini/antigravity-cli/settings.json");
    let mut settings = if path.exists() {
        serde_json::from_str::<Value>(
            &fs::read_to_string(&path)
                .map_err(|error| format!("Не удалось прочитать {}: {error}", path.display()))?,
        )
        .map_err(|error| format!("Некорректный {}: {error}", path.display()))?
    } else {
        json!({})
    };
    let trusted = settings
        .as_object_mut()
        .ok_or_else(|| format!("Некорректный {}: ожидается объект", path.display()))?
        .entry("trustedWorkspaces")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or_else(|| {
            format!(
                "Некорректный {}: trustedWorkspaces не массив",
                path.display()
            )
        })?;
    let worktree = worktree.to_string_lossy();
    if !trusted
        .iter()
        .any(|value| value.as_str() == Some(worktree.as_ref()))
    {
        trusted.push(Value::String(worktree.into_owned()));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Не удалось создать {}: {error}", parent.display()))?;
        }
        fs::write(
            &path,
            serde_json::to_string_pretty(&settings)
                .map_err(|error| format!("Не удалось сериализовать {}: {error}", path.display()))?,
        )
        .map_err(|error| format!("Не удалось записать {}: {error}", path.display()))?;
    }
    Ok(())
}

fn spawn_pty_job(
    agent: AgentKind,
    worktree: &Path,
    task_file: &str,
    name: &str,
    model: Option<&str>,
    prompt: &str,
    registry_path: &Path,
) -> Result<String, String> {
    if !worktree.is_dir() {
        return Err(format!("Ворктри не найдена: {}", worktree.display()));
    }
    if !worktree.join(task_file).is_file() {
        return Err(format!(
            "Файл задачи не найден: {}",
            worktree.join(task_file).display()
        ));
    }
    if name.trim().is_empty() {
        return Err("Имя задачи не может быть пустым".to_owned());
    }
    if agent.as_str() == "gemini" {
        trust_agy_workspace(worktree)?;
    }

    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 50,
            cols: 220,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("Не удалось открыть PTY задачи {name}: {error}"))?;
    let (program, args) = agent_command(agent, model, prompt);
    let mut command = CommandBuilder::new(program);
    command.args(args);
    command.cwd(worktree);
    command.env("PATH", platform::tool_path());
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    command.env("PWD", worktree);
    command.env("OLDPWD", worktree);
    command.env_remove("ANTHROPIC_BASE_URL");
    command.env_remove("ANTHROPIC_AUTH_TOKEN");
    command.env_remove("ANTHROPIC_SMALL_FAST_MODEL");
    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("Не удалось запустить {program} через PTY: {error}"))?;
    drop(pair.slave);
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("Не удалось открыть экран PTY-задачи {name}: {error}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| format!("Не удалось открыть ввод PTY-задачи {name}: {error}"))?;

    let child = Arc::new(Mutex::new(child));
    if let Err(error) = register_pty_job(registry_path, name, agent, worktree, task_file, model) {
        if let Ok(mut child) = child.lock() {
            let _ = child.kill();
        }
        return Err(error);
    }
    let screen = Arc::new(Mutex::new(ScreenBuffer::default()));
    PTY_JOBS
        .lock()
        .map_err(|_| "Не удалось заблокировать список PTY-задач".to_owned())?
        .insert(
            name.to_owned(),
            PtyJob {
                writer: Arc::new(Mutex::new(writer)),
                child: child.clone(),
                screen: screen.clone(),
            },
        );

    let thread_name = name.to_owned();
    let thread_registry = registry_path.to_owned();
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        let mut pending = Vec::new();
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    pending.extend_from_slice(&buffer[..count]);
                    let text = decode_available(&mut pending, false);
                    if !text.is_empty()
                        && let Ok(mut screen) = screen.lock()
                    {
                        screen.push(&text);
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        let tail = decode_available(&mut pending, true);
        if !tail.is_empty()
            && let Ok(mut screen) = screen.lock()
        {
            screen.push(&tail);
        }
        if let Ok(mut child) = child.lock() {
            let _ = child.wait();
        }
        let _ = set_pty_job_state(&thread_registry, &thread_name, "exited");
    });
    Ok(name.to_owned())
}

async fn tmux_screen_for(name: &str, lines: u32) -> String {
    let args = vec!["tail".to_owned(), name.to_owned(), lines.to_string()];
    let Ok(command) = agents_command(&args) else {
        return String::new();
    };
    run_shell(&command, None).await.unwrap_or_default()
}

async fn enrich_job(name: String, raw: RawJob) -> Job {
    let steps = read_steps(&raw);
    let screen = match raw.runner {
        Runner::Pty => pty_screen(&name, 3).unwrap_or_default(),
        Runner::Tmux if matches!(raw.state.as_str(), "running" | "asking") => {
            tmux_screen_for(&name, 3).await
        }
        Runner::Tmux => String::new(),
    };
    let last_line = last_nonempty(&screen);
    Job {
        name,
        agent: raw.agent,
        worktree: raw.worktree,
        task: raw.task,
        started_at: started_at(&raw.started_at),
        state: computed_state(&raw.state, &steps, &screen, &last_line),
        handed_to: raw.handed_to,
        takeover_of: raw.takeover_of,
        steps,
        last_line,
        model: Some(
            raw.model
                .clone()
                .unwrap_or_else(|| model_from_screen(raw.agent, &screen)),
        ),
    }
}

pub async fn collect_jobs() -> Result<Vec<Job>, String> {
    let path = orchestra_home()?.join("jobs.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw_jobs = read_jobs(&path)?;
    let mut pending = tokio::task::JoinSet::new();
    for (name, raw) in raw_jobs {
        pending.spawn(enrich_job(name, raw));
    }
    let mut jobs = Vec::with_capacity(pending.len());
    while let Some(result) = pending.join_next().await {
        jobs.push(result.map_err(|error| format!("Не удалось собрать данные задачи: {error}"))?);
    }
    jobs.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(jobs)
}

#[tauri::command]
pub async fn list_jobs() -> Result<Vec<Job>, String> {
    collect_jobs().await
}

#[tauri::command]
pub async fn job_screen(name: String, lines: Option<u32>) -> Result<String, String> {
    if runner_for(&name)? == Runner::Pty {
        return pty_screen(&name, lines.unwrap_or(3).clamp(1, 10_000) as usize);
    }
    let args = vec![
        "tail".to_owned(),
        name,
        lines.unwrap_or(3).clamp(1, 10_000).to_string(),
    ];
    run_shell(&agents_command(&args)?, None).await
}

#[tauri::command]
pub async fn job_send(name: String, text: String) -> Result<(), String> {
    if runner_for(&name)? == Runner::Pty {
        return pty_send(&name, &text);
    }
    let args = vec!["send".to_owned(), name, text];
    run_shell(&agents_command(&args)?, None).await.map(|_| ())
}

#[tauri::command]
pub async fn job_kill(name: String) -> Result<(), String> {
    if runner_for(&name)? == Runner::Pty {
        return kill_pty_job(&name, &orchestra_home()?.join("jobs.json"));
    }
    let args = vec!["kill".to_owned(), name];
    run_shell(&agents_command(&args)?, None).await.map(|_| ())
}

#[tauri::command]
pub async fn job_new(repo: String, name: String) -> Result<String, String> {
    let args = vec!["new".to_owned(), repo, name];
    run_shell(&agents_command(&args)?, None).await
}

#[tauri::command]
pub async fn job_run(
    agent: AgentKind,
    worktree: String,
    task_file: String,
    name: Option<String>,
) -> Result<Job, String> {
    // Экран «Модели»: выбор из ~/.orchestra/models.json (ключи claude / codex / agy).
    let provider = match agent {
        AgentKind::Codex => "codex",
        AgentKind::Gemini => "agy",
        AgentKind::Claude => "claude",
    };
    let model = crate::models::selected_for(provider);
    let actual_name = if platform::has_tmux() {
        let mut args = vec![
            "run".to_owned(),
            agent.as_str().to_owned(),
            worktree.clone(),
            task_file.clone(),
        ];
        if let Some(name) = name.as_ref() {
            args.extend(["--name".to_owned(), name.clone()]);
        }
        if let Some(model) = model.as_ref() {
            args.extend(["--model".to_owned(), model.clone()]);
        }
        run_shell(&agents_command(&args)?, None).await?
    } else {
        let worktree_path = PathBuf::from(&worktree);
        let actual_name = name.unwrap_or_else(|| {
            format!(
                "{}-{}",
                agent.as_str(),
                worktree_path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("job")
            )
        });
        spawn_pty_job(
            agent,
            &worktree_path,
            &task_file,
            &actual_name,
            model.as_deref(),
            &task_prompt(&task_file),
            &orchestra_home()?.join("jobs.json"),
        )?
    };
    collect_jobs()
        .await?
        .into_iter()
        .find(|job| job.name == actual_name.trim())
        .ok_or_else(|| {
            format!(
                "Задача {} запущена, но не найдена в jobs.json",
                actual_name.trim()
            )
        })
}

#[tauri::command]
pub fn write_task(worktree: String, slug: String, markdown: String) -> Result<String, String> {
    let slug_regex = Regex::new(r"^[A-Za-z0-9_-]+$").expect("valid regex");
    if !slug_regex.is_match(&slug) {
        return Err("Slug может содержать только буквы, цифры, _ и -".to_owned());
    }
    let worktree = PathBuf::from(worktree);
    if !worktree.is_dir() {
        return Err(format!("Ворктри не найдена: {}", worktree.display()));
    }
    let docs = worktree.join("docs");
    fs::create_dir_all(&docs)
        .map_err(|error| format!("Не удалось создать {}: {error}", docs.display()))?;
    let path = docs.join(format!("TASK-{slug}.md"));
    fs::write(&path, markdown)
        .map_err(|error| format!("Не удалось записать {}: {error}", path.display()))?;
    Ok(path.to_string_lossy().into_owned())
}

pub fn start_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let Ok(home) = orchestra_home() else {
            return;
        };
        let _ = fs::create_dir_all(&home);
        let (tx, rx) = mpsc::channel();
        let mut watcher: RecommendedWatcher = match notify::recommended_watcher(move |event| {
            let _ = tx.send(event);
        }) {
            Ok(watcher) => watcher,
            Err(error) => {
                eprintln!("jobs watcher: {error}");
                return;
            }
        };
        loop {
            match watcher.watch(&home, RecursiveMode::NonRecursive) {
                Ok(()) => break,
                Err(error) => {
                    eprintln!("jobs watcher: {error}");
                    std::thread::sleep(Duration::from_secs(2));
                }
            }
        }

        loop {
            // SPEC §4/§7 требует обновлять экраны активных задач каждые 2 секунды;
            // collect_jobs пропускает завершённые задачи и опрашивает активные параллельно.
            match rx.recv_timeout(Duration::from_secs(2)) {
                Ok(_) => {
                    std::thread::sleep(Duration::from_millis(150));
                    while rx.try_recv().is_ok() {}
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
            let app = app.clone();
            tauri::async_runtime::block_on(async move {
                if let Ok(jobs) = collect_jobs().await {
                    let _ = app.emit("jobs://changed", jobs);
                }
            });
        }
    });
}

#[cfg(test)]
#[test]
#[ignore = "запускает реальный claude через PTY"]
fn live_pty_runner() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!(
            "takt-live-pty-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
    let worktree = root.join("worktree");
    let task = "docs/TASK-live.md";
    fs::create_dir_all(worktree.join("docs")).expect("create temporary worktree");
    fs::write(worktree.join(task), "# Live PTY\n").expect("write task");
    let registry = root.join("orchestra/jobs.json");
    let name = format!("live-pty-{}", std::process::id());

    spawn_pty_job(
        AgentKind::Claude,
        &worktree,
        task,
        &name,
        None,
        "скажи ok и выйди",
        &registry,
    )
    .expect("start claude PTY runner");

    let deadline = std::time::Instant::now() + Duration::from_secs(90);
    let output = loop {
        let screen = pty_screen(&name, PTY_HISTORY_LINES).unwrap_or_default();
        if has_standalone_ok(&screen) || std::time::Instant::now() >= deadline {
            break screen;
        }
        std::thread::sleep(Duration::from_millis(250));
    };
    println!("{output}");
    let contains_answer = has_standalone_ok(&output);
    let jobs = read_jobs(&registry).expect("read PTY registry");
    let runner = jobs.get(&name).map(|job| job.runner);

    kill_pty_job(&name, &registry).expect("kill PTY runner");
    let removed = !read_jobs(&registry)
        .expect("read cleaned registry")
        .contains_key(&name);
    fs::remove_dir_all(&root).expect("remove temporary live PTY files");
    assert!(contains_answer);
    assert_eq!(runner, Some(Runner::Pty));
    assert!(removed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_acceptance_checklist() {
        let steps = parse_steps(
            "- [x] вне\n## Приёмка\n- [ ] Первый шаг\n  с продолжением\n- [X] Второй шаг\n## Правила\n- [ ] вне",
        );
        assert_eq!(steps.len(), 2);
        assert!(!steps[0].done);
        assert_eq!(steps[0].text, "Первый шаг с продолжением");
        assert!(steps[1].done);
    }

    #[test]
    fn acceptance_header_is_multilingual() {
        assert!(is_acceptance_header("## Приёмка"));
        assert!(is_acceptance_header("## Приймання"));
        assert!(is_acceptance_header("## Acceptance"));
        assert!(is_acceptance_header("##  acceptance criteria "));
        assert!(!is_acceptance_header("### Приёмка"));
        assert!(!is_acceptance_header("## Правила"));
        let steps =
            parse_steps("## Acceptance\n- [ ] tsc passes\n- [x] tests pass\n## Notes\n- [ ] out");
        assert_eq!(steps.len(), 2);
        assert!(steps[1].done);
    }

    #[test]
    fn state_priorities_are_deterministic() {
        let steps = vec![Step {
            text: "Готово".to_owned(),
            done: true,
        }];
        assert_eq!(
            computed_state("running", &steps, "", "Продолжить?"),
            "asking"
        );
        assert_eq!(computed_state("exited", &steps, "", ""), "done");
        assert_eq!(
            computed_state("running", &steps, "usage limit reached", ""),
            "limit"
        );
        assert_eq!(computed_state("handed-off", &steps, "", ""), "exited");
    }

    #[test]
    fn pty_screen_keeps_last_two_thousand_lines() {
        let mut screen = ScreenBuffer::default();
        for line in 0..2_010 {
            screen.push(&format!("line-{line}\r\n"));
        }
        let rendered = screen.render(PTY_HISTORY_LINES);
        assert!(!rendered.contains("line-0\n"));
        assert!(rendered.starts_with("line-10\n"));
        assert!(rendered.ends_with("line-2009"));
        assert_eq!(rendered.lines().count(), PTY_HISTORY_LINES);
    }

    #[test]
    fn pty_screen_drops_escape_split_between_reads() {
        let mut screen = ScreenBuffer::default();
        screen.push("предупреждение\u{1b}[38;2;153");
        screen.push(";153;153m · текст\r\n");
        assert_eq!(screen.render(10), "предупреждение · текст");

        let mut screen = ScreenBuffer::default();
        screen.push("ответ \u{1b}[1mok\u{1b}[0m\n");
        assert_eq!(screen.render(10), "ответ ok");
    }

    #[test]
    fn live_answer_is_not_confused_with_prompt_or_hooks() {
        assert!(!has_standalone_ok(
            "❯ скажи ok и выйди\n(running SessionStart hooks…)"
        ));
        assert!(has_standalone_ok("● ok"));
        assert!(has_standalone_ok("OK"));
    }

    #[test]
    fn pty_commands_match_orchestration_contract() {
        let (program, args) = agent_command(AgentKind::Codex, None, "prompt");
        assert_eq!(program, "codex");
        assert_eq!(
            args,
            [
                "-c",
                "model_reasoning_effort=high",
                "--dangerously-bypass-approvals-and-sandbox",
                "prompt"
            ]
        );
        let (program, args) = agent_command(AgentKind::Gemini, Some("gemini-test"), "prompt");
        assert_eq!(program, "agy");
        assert_eq!(
            args[0..4],
            ["--dangerously-skip-permissions", "--effort", "high", "-i"]
        );
        assert_eq!(args[4..], ["--model", "gemini-test", "prompt"]);
        let (program, args) = agent_command(AgentKind::Claude, None, "prompt");
        assert_eq!(program, "claude");
        assert_eq!(args, ["--dangerously-skip-permissions", "prompt"]);
    }

    #[test]
    fn registry_round_trip_preserves_runner() {
        let root = std::env::temp_dir().join(format!(
            "takt-jobs-registry-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let registry = root.join("jobs.json");
        register_pty_job(
            &registry,
            "demo",
            AgentKind::Codex,
            Path::new("/tmp/worktree"),
            "docs/TASK-demo.md",
            Some("gpt-test"),
        )
        .unwrap();
        let jobs = read_jobs(&registry).unwrap();
        let job = jobs.get("demo").unwrap();
        assert_eq!(job.runner, Runner::Pty);
        assert_eq!(job.model.as_deref(), Some("gpt-test"));

        remove_pty_job(&registry, "demo").unwrap();
        assert!(read_jobs(&registry).unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_registry_entry_defaults_to_tmux() {
        let jobs: HashMap<String, RawJob> = serde_json::from_str(
            r#"{"legacy":{"agent":"claude","worktree":"/tmp/wt","task":"docs/TASK.md","startedAt":1,"state":"running"}}"#,
        )
        .unwrap();
        assert_eq!(jobs["legacy"].runner, Runner::Tmux);
    }

    #[tokio::test]
    #[ignore = "читает реальный ~/.orchestra/jobs.json"]
    async fn live_jobs_snapshot() {
        let jobs = collect_jobs().await.expect("live jobs");
        println!("{}", serde_json::to_string_pretty(&jobs).unwrap());
        assert!(jobs.iter().all(|job| !job.started_at.is_empty()));
    }
}
