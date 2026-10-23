use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, mpsc};
use std::time::Duration;

use chrono::{DateTime, Utc};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::shell::{quote, run_zsh};

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
}

fn running_state() -> String {
    "running".to_owned()
}

fn orchestra_home() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(".orchestra"))
        .ok_or_else(|| "Не удалось определить домашнюю папку".to_owned())
}

fn agents_command(args: &[String]) -> Result<String, String> {
    let script = orchestra_home()?.join("agents.py");
    let mut parts = vec![
        "python3".to_owned(),
        "-I".to_owned(),
        quote(&script.to_string_lossy()),
    ];
    parts.extend(args.iter().map(|arg| quote(arg)));
    Ok(parts.join(" "))
}

fn parse_steps(markdown: &str) -> Vec<Step> {
    let mut in_acceptance = false;
    let mut steps = Vec::new();
    for line in markdown.lines() {
        if line.trim() == "## Приёмка" {
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

async fn screen_for(name: &str, lines: u32) -> String {
    let args = vec!["tail".to_owned(), name.to_owned(), lines.to_string()];
    let Ok(command) = agents_command(&args) else {
        return String::new();
    };
    run_zsh(&command, None).await.unwrap_or_default()
}

async fn enrich_job(name: String, raw: RawJob) -> Job {
    let steps = read_steps(&raw);
    let screen = if matches!(raw.state.as_str(), "running" | "asking") {
        screen_for(&name, 3).await
    } else {
        String::new()
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
        model: Some(model_from_screen(raw.agent, &screen)),
    }
}

pub async fn collect_jobs() -> Result<Vec<Job>, String> {
    let path = orchestra_home()?.join("jobs.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("Не удалось прочитать {}: {error}", path.display()))?;
    let raw_jobs: HashMap<String, RawJob> = serde_json::from_str(&content)
        .map_err(|error| format!("Некорректный jobs.json: {error}"))?;
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
    let args = vec![
        "tail".to_owned(),
        name,
        lines.unwrap_or(3).clamp(1, 10_000).to_string(),
    ];
    run_zsh(&agents_command(&args)?, None).await
}

#[tauri::command]
pub async fn job_send(name: String, text: String) -> Result<(), String> {
    let args = vec!["send".to_owned(), name, text];
    run_zsh(&agents_command(&args)?, None).await.map(|_| ())
}

#[tauri::command]
pub async fn job_kill(name: String) -> Result<(), String> {
    let args = vec!["kill".to_owned(), name];
    run_zsh(&agents_command(&args)?, None).await.map(|_| ())
}

#[tauri::command]
pub async fn job_new(repo: String, name: String) -> Result<String, String> {
    let args = vec!["new".to_owned(), repo, name];
    run_zsh(&agents_command(&args)?, None).await
}

#[tauri::command]
pub async fn job_run(
    agent: AgentKind,
    worktree: String,
    task_file: String,
    name: Option<String>,
) -> Result<Job, String> {
    let mut args = vec![
        "run".to_owned(),
        agent.as_str().to_owned(),
        worktree,
        task_file,
    ];
    if let Some(name) = name {
        args.extend(["--name".to_owned(), name]);
    }
    let actual_name = run_zsh(&agents_command(&args)?, None).await?;
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

    #[tokio::test]
    #[ignore = "читает реальный ~/.orchestra/jobs.json"]
    async fn live_jobs_snapshot() {
        let jobs = collect_jobs().await.expect("live jobs");
        println!("{}", serde_json::to_string_pretty(&jobs).unwrap());
        assert!(jobs.iter().all(|job| !job.started_at.is_empty()));
    }
}
