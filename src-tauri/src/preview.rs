use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, SecondsFormat, Utc};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::projects;
use crate::shell::run_zsh;

const MAX_ARTIFACTS: usize = 60;
const MAX_DIFF_BYTES: usize = 200 * 1024;
const RECENT_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const DEBOUNCE: Duration = Duration::from_millis(700);
const PORT_POLL: Duration = Duration::from_secs(10);
const TRUNCATION_MARKER: &str = "\n\n… diff обрезан (лимит 200 КБ)\n";

const EXCLUDED_DIRS: [&str; 4] = ["node_modules", "target", "dist", ".git"];
const IMAGE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "webp", "svg"];
const DOC_EXTENSIONS: [&str; 4] = ["html", "htm", "md", "pdf"];
const READABLE_EXTENSIONS: [&str; 3] = ["md", "html", "svg"];

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSource {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub title: String,
    pub mtime: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Listener {
    pid: u32,
    process: String,
    port: u16,
}

#[derive(Debug)]
struct Artifact {
    path: PathBuf,
    kind: &'static str,
    modified: SystemTime,
}

fn timestamp(time: SystemTime) -> String {
    DateTime::<Utc>::from(time).to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
}

fn artifact_kind(path: &Path) -> Option<&'static str> {
    let extension = extension(path)?;
    if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
        Some("image")
    } else if DOC_EXTENSIONS.contains(&extension.as_str()) {
        Some("doc")
    } else {
        None
    }
}

fn ignored_component(component: &str) -> bool {
    component.starts_with('.') || EXCLUDED_DIRS.contains(&component)
}

fn has_ignored_component(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(value) => value.to_str().is_some_and(ignored_component),
        _ => false,
    })
}

fn collect_artifacts_in(directory: &Path, cutoff: SystemTime, artifacts: &mut Vec<Artifact>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            if !entry.file_name().to_str().is_some_and(ignored_component) {
                collect_artifacts_in(&path, cutoff, artifacts);
            }
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let Some(kind) = artifact_kind(&path) else {
            continue;
        };
        let Ok(modified) = entry.metadata().and_then(|metadata| metadata.modified()) else {
            continue;
        };
        if modified >= cutoff {
            artifacts.push(Artifact {
                path,
                kind,
                modified,
            });
        }
    }
}

fn canonical_directory(path: &Path) -> Option<PathBuf> {
    path.is_dir().then(|| fs::canonicalize(path).ok()).flatten()
}

fn unique_directories(paths: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .filter_map(|path| canonical_directory(&path))
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

fn git_output(worktree: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("git")
        .arg("-C")
        .arg(worktree)
        .args(args)
        .output()
        .map_err(|error| format!("Не удалось запустить git: {error}"))
}

fn worktree_roots(project_path: &Path) -> Vec<PathBuf> {
    let mut paths = vec![project_path.to_path_buf()];
    if let Ok(output) = git_output(project_path, &["worktree", "list", "--porcelain"])
        && output.status.success()
    {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                paths.push(PathBuf::from(path));
            }
        }
    }
    unique_directories(paths)
}

fn parse_port(endpoint: &str) -> Option<u16> {
    let (host, port) = endpoint.rsplit_once(':')?;
    let host = host.trim_matches(['[', ']']);
    let local = host == "*"
        || host == "localhost"
        || host == "0.0.0.0"
        || host == "::"
        || host == "::1"
        || host.starts_with("127.");
    local.then(|| port.parse().ok()).flatten()
}

fn parse_lsof_listeners(output: &str) -> Vec<Listener> {
    let mut pid = None;
    let mut process = String::new();
    let mut listeners = Vec::new();
    for line in output.lines() {
        let Some((field, value)) = line.split_at_checked(1) else {
            continue;
        };
        match field {
            "p" => {
                pid = value.parse().ok();
                process.clear();
            }
            "c" => process = value.to_owned(),
            "n" => {
                if let (Some(pid), Some(port)) = (pid, parse_port(value)) {
                    listeners.push(Listener {
                        pid,
                        process: process.clone(),
                        port,
                    });
                }
            }
            _ => {}
        }
    }
    listeners.sort_by_key(|listener| (listener.port, listener.pid));
    listeners.dedup();
    listeners
}

fn parse_lsof_cwds(output: &str) -> HashMap<u32, PathBuf> {
    let mut pid = None;
    let mut cwds = HashMap::new();
    for line in output.lines() {
        let Some((field, value)) = line.split_at_checked(1) else {
            continue;
        };
        match field {
            "p" => pid = value.parse().ok(),
            "n" if !value.is_empty() => {
                if let Some(pid) = pid {
                    cwds.insert(pid, PathBuf::from(value));
                }
            }
            _ => {}
        }
    }
    cwds
}

async fn url_sources(roots: &[PathBuf]) -> Vec<PreviewSource> {
    let Ok(output) = run_zsh(
        "lsof -iTCP -sTCP:LISTEN -P -n -Fpcn 2>/dev/null; true",
        None,
    )
    .await
    else {
        return Vec::new();
    };
    let listeners = parse_lsof_listeners(&output);
    let pids = listeners
        .iter()
        .map(|listener| listener.pid.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let cwd_by_pid = if pids.is_empty() {
        HashMap::new()
    } else {
        let command = format!("lsof -a -d cwd -p {pids} -Fn 2>/dev/null; true");
        run_zsh(&command, None)
            .await
            .map(|output| parse_lsof_cwds(&output))
            .unwrap_or_default()
    };
    let mut sources = Vec::new();
    for listener in listeners {
        let cwd = cwd_by_pid
            .get(&listener.pid)
            .and_then(|path| fs::canonicalize(path).ok());
        if cwd.is_some_and(|cwd| roots.iter().any(|root| cwd.starts_with(root))) {
            sources.push(PreviewSource {
                kind: "url".to_owned(),
                path: None,
                url: Some(format!("http://localhost:{}", listener.port)),
                title: format!("{} :{}", listener.process, listener.port),
                mtime: timestamp(SystemTime::now()),
            });
        }
    }
    sources
}

fn artifact_sources(roots: &[PathBuf]) -> Vec<PreviewSource> {
    let cutoff = SystemTime::now()
        .checked_sub(RECENT_AGE)
        .unwrap_or(UNIX_EPOCH);
    let mut artifacts = Vec::new();
    for root in roots {
        collect_artifacts_in(root, cutoff, &mut artifacts);
    }
    artifacts.sort_by(|left, right| right.modified.cmp(&left.modified));
    artifacts.truncate(MAX_ARTIFACTS);
    artifacts
        .into_iter()
        .map(|artifact| PreviewSource {
            kind: artifact.kind.to_owned(),
            title: artifact
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            path: Some(artifact.path.to_string_lossy().into_owned()),
            url: None,
            mtime: timestamp(artifact.modified),
        })
        .collect()
}

fn parse_status_paths(output: &[u8]) -> Vec<PathBuf> {
    let entries: Vec<&[u8]> = output
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .collect();
    let mut paths = Vec::new();
    let mut index = 0;
    while index < entries.len() {
        let entry = entries[index];
        if entry.len() >= 3 && entry[2] == b' ' {
            let status = &entry[..2];
            paths.push(PathBuf::from(
                String::from_utf8_lossy(&entry[3..]).into_owned(),
            ));
            if status.contains(&b'R') || status.contains(&b'C') {
                index += 1;
            }
        }
        index += 1;
    }
    paths
}

fn diff_sources(roots: &[PathBuf]) -> Vec<PreviewSource> {
    let mut sources = Vec::new();
    for root in roots {
        let Ok(output) = git_output(
            root,
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        ) else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        for relative in parse_status_paths(&output.stdout) {
            let path = root.join(&relative);
            let modified = fs::metadata(&path)
                .and_then(|metadata| metadata.modified())
                .unwrap_or(UNIX_EPOCH);
            sources.push(PreviewSource {
                kind: "diff".to_owned(),
                path: Some(path.to_string_lossy().into_owned()),
                url: None,
                title: relative.to_string_lossy().into_owned(),
                mtime: timestamp(modified),
            });
        }
    }
    sources.sort_by(|left, right| {
        left.title
            .cmp(&right.title)
            .then_with(|| left.path.cmp(&right.path))
    });
    sources
}

async fn sources_for_project(project_path: &Path) -> Result<Vec<PreviewSource>, String> {
    let project_path = fs::canonicalize(project_path).map_err(|error| {
        format!(
            "Не удалось открыть проект {}: {error}",
            project_path.display()
        )
    })?;
    if !project_path.is_dir() {
        return Err(format!(
            "Папка проекта не найдена: {}",
            project_path.display()
        ));
    }
    let roots = worktree_roots(&project_path);
    let mut sources = url_sources(&roots).await;
    sources.extend(artifact_sources(&roots));
    sources.extend(diff_sources(&roots));
    Ok(sources)
}

#[tauri::command]
pub async fn preview_sources(project_path: String) -> Result<Vec<PreviewSource>, String> {
    sources_for_project(Path::new(&project_path)).await
}

fn canonicalize_for_boundary(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        return fs::canonicalize(path)
            .map_err(|error| format!("Не удалось открыть {}: {error}", path.display()));
    }
    let parent = path
        .parent()
        .ok_or_else(|| format!("Некорректный путь: {}", path.display()))?;
    let parent = fs::canonicalize(parent)
        .map_err(|error| format!("Не удалось открыть {}: {error}", parent.display()))?;
    let name = path
        .file_name()
        .ok_or_else(|| format!("Некорректный путь: {}", path.display()))?;
    Ok(parent.join(name))
}

fn ensure_within(path: &Path, root: &Path) -> Result<(PathBuf, PathBuf), String> {
    let root = fs::canonicalize(root)
        .map_err(|error| format!("Не удалось открыть ворктри {}: {error}", root.display()))?;
    let path = canonicalize_for_boundary(path)?;
    if !path.starts_with(&root) || path == root {
        return Err("Файл находится вне указанной ворктри".to_owned());
    }
    Ok((path, root))
}

fn command_error(prefix: &str, output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let detail = if stderr.is_empty() { stdout } else { stderr };
    if detail.is_empty() {
        prefix.to_owned()
    } else {
        format!("{prefix}: {detail}")
    }
}

fn truncate_diff(diff: String) -> String {
    if diff.len() <= MAX_DIFF_BYTES {
        return diff;
    }
    let available = MAX_DIFF_BYTES.saturating_sub(TRUNCATION_MARKER.len());
    let mut end = available.min(diff.len());
    while !diff.is_char_boundary(end) {
        end -= 1;
    }
    let mut truncated = diff[..end].to_owned();
    truncated.push_str(TRUNCATION_MARKER);
    truncated
}

#[tauri::command(async)]
pub fn preview_diff(path: String, worktree: String) -> Result<String, String> {
    let (path, worktree) = ensure_within(Path::new(&path), Path::new(&worktree))?;
    if !configured_roots()?
        .iter()
        .any(|root| worktree.starts_with(root))
    {
        return Err("Ворктри находится вне разрешённых проектов".to_owned());
    }
    let relative = path
        .strip_prefix(&worktree)
        .map_err(|_| "Не удалось определить путь файла в ворктри".to_owned())?;
    let tracked = git_output(
        &worktree,
        &[
            "ls-files",
            "--error-unmatch",
            "--",
            &relative.to_string_lossy(),
        ],
    )
    .is_ok_and(|output| output.status.success());
    let output = if tracked {
        Command::new("git")
            .arg("-C")
            .arg(&worktree)
            .args(["diff", "--"])
            .arg(relative)
            .output()
            .map_err(|error| format!("Не удалось запустить git diff: {error}"))?
    } else {
        Command::new("git")
            .arg("diff")
            .args(["--no-index", "--", "/dev/null"])
            .arg(&path)
            .output()
            .map_err(|error| format!("Не удалось запустить git diff: {error}"))?
    };
    if !output.status.success() && !(output.status.code() == Some(1) && !tracked) {
        return Err(command_error("Не удалось получить diff", &output));
    }
    Ok(truncate_diff(
        String::from_utf8_lossy(&output.stdout).into_owned(),
    ))
}

fn configured_roots() -> Result<Vec<PathBuf>, String> {
    let mut roots = projects::list_projects()?
        .into_iter()
        .map(|project| PathBuf::from(project.path))
        .collect::<Vec<_>>();
    let home = dirs::home_dir().ok_or_else(|| "Не удалось определить домашнюю папку".to_owned())?;
    roots.push(home.join("orchestra"));
    Ok(unique_directories(roots))
}

fn preview_read_from_roots(path: &Path, roots: &[PathBuf]) -> Result<String, String> {
    let extension = extension(path).unwrap_or_default();
    if !READABLE_EXTENSIONS.contains(&extension.as_str()) {
        return Err("Предпросмотр текста доступен только для md, html и svg".to_owned());
    }
    let path = fs::canonicalize(path)
        .map_err(|error| format!("Не удалось открыть {}: {error}", path.display()))?;
    let allowed = roots
        .iter()
        .filter_map(|root| fs::canonicalize(root).ok())
        .any(|root| path.starts_with(&root) && path != root);
    if !allowed {
        return Err("Файл находится вне разрешённых проектов".to_owned());
    }
    fs::read_to_string(&path)
        .map_err(|error| format!("Не удалось прочитать {}: {error}", path.display()))
}

#[tauri::command]
pub fn preview_read(path: String) -> Result<String, String> {
    preview_read_from_roots(Path::new(&path), &configured_roots()?)
}

fn source_fingerprint(sources: &[PreviewSource]) -> Vec<String> {
    sources
        .iter()
        .map(|source| {
            let mtime = if source.kind == "url" {
                ""
            } else {
                &source.mtime
            };
            format!(
                "{}\0{}\0{}\0{}\0{}",
                source.kind,
                source.path.as_deref().unwrap_or_default(),
                source.url.as_deref().unwrap_or_default(),
                source.title,
                mtime
            )
        })
        .collect()
}

fn path_is_relevant(path: &Path, project: &Path, roots: &[PathBuf], orchestra: &Path) -> bool {
    if let Some(relative) = roots
        .iter()
        .filter_map(|root| path.strip_prefix(root).ok())
        .min_by_key(|relative| relative.components().count())
    {
        return !has_ignored_component(relative);
    }
    let Some(repo_name) = project.file_name() else {
        return false;
    };
    path.strip_prefix(orchestra)
        .ok()
        .and_then(|relative| relative.components().next())
        .is_some_and(|component| component.as_os_str() == repo_name)
        && path
            .strip_prefix(orchestra)
            .is_ok_and(|relative| !has_ignored_component(relative))
}

fn refresh_projects(
    app: &AppHandle,
    project_paths: &[PathBuf],
    selected: impl IntoIterator<Item = usize>,
    fingerprints: &mut HashMap<PathBuf, Vec<String>>,
) {
    for index in selected {
        let Some(project_path) = project_paths.get(index) else {
            continue;
        };
        let result = tauri::async_runtime::block_on(sources_for_project(project_path));
        let Ok(sources) = result else {
            continue;
        };
        let fingerprint = source_fingerprint(&sources);
        let changed = fingerprints
            .get(project_path)
            .is_none_or(|previous| previous != &fingerprint);
        fingerprints.insert(project_path.clone(), fingerprint);
        if changed {
            let payload = serde_json::json!({
                "projectPath": project_path.to_string_lossy(),
                "sources": sources,
            });
            let _ = app.emit("preview://changed", payload);
        }
    }
}

fn allow_project_asset_scopes(app: &AppHandle, project_paths: &[PathBuf]) {
    let scope = app.asset_protocol_scope();
    for path in project_paths {
        if let Err(error) = scope.allow_directory(path, true) {
            eprintln!("preview asset scope {}: {error}", path.display());
        }
    }
}

pub fn start_watcher(app: AppHandle) {
    let project_paths = projects::list_projects()
        .unwrap_or_default()
        .into_iter()
        .map(|project| PathBuf::from(project.path))
        .collect::<Vec<_>>();
    allow_project_asset_scopes(&app, &project_paths);
    std::thread::spawn(move || {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let orchestra = home.join("orchestra");
        let mut watch_roots = project_paths.clone();
        watch_roots.push(orchestra.clone());
        let watch_roots = unique_directories(watch_roots);
        let (tx, rx) = mpsc::channel();
        let mut watcher: RecommendedWatcher = match notify::recommended_watcher(move |event| {
            let _ = tx.send(event);
        }) {
            Ok(watcher) => watcher,
            Err(error) => {
                eprintln!("preview watcher: {error}");
                return;
            }
        };
        for root in &watch_roots {
            if let Err(error) = watcher.watch(root, RecursiveMode::Recursive) {
                eprintln!("preview watcher {}: {error}", root.display());
            }
        }

        let mut roots_by_project = project_paths
            .iter()
            .map(|project_path| worktree_roots(project_path))
            .collect::<Vec<_>>();
        let mut fingerprints = HashMap::new();
        for project_path in &project_paths {
            if let Ok(sources) = tauri::async_runtime::block_on(sources_for_project(project_path)) {
                fingerprints.insert(project_path.clone(), source_fingerprint(&sources));
            }
        }
        let mut pending = HashSet::new();
        let mut debounce_at = None;
        let mut port_poll_at = Instant::now() + PORT_POLL;

        loop {
            let now = Instant::now();
            let deadline = debounce_at
                .map(|deadline: Instant| deadline.min(port_poll_at))
                .unwrap_or(port_poll_at);
            let timeout = deadline.saturating_duration_since(now);
            match rx.recv_timeout(timeout) {
                Ok(Ok(Event { paths, .. })) => {
                    for (index, (project_path, roots)) in
                        project_paths.iter().zip(&roots_by_project).enumerate()
                    {
                        if paths
                            .iter()
                            .any(|path| path_is_relevant(path, project_path, roots, &orchestra))
                        {
                            pending.insert(index);
                        }
                    }
                    if !pending.is_empty() {
                        debounce_at = Some(Instant::now() + DEBOUNCE);
                    }
                }
                Ok(Err(error)) => eprintln!("preview watcher event: {error}"),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }

            let now = Instant::now();
            if debounce_at.is_some_and(|deadline| now >= deadline) {
                let selected = pending.drain().collect::<Vec<_>>();
                refresh_projects(&app, &project_paths, selected, &mut fingerprints);
                debounce_at = None;
            }
            if now >= port_poll_at {
                refresh_projects(
                    &app,
                    &project_paths,
                    0..project_paths.len(),
                    &mut fingerprints,
                );
                roots_by_project = project_paths
                    .iter()
                    .map(|project_path| worktree_roots(project_path))
                    .collect();
                port_poll_at = Instant::now() + PORT_POLL;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "takt-preview-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn parses_lsof_field_output_and_local_ports() {
        let listeners = parse_lsof_listeners(
            "p101\ncnode\nfcwd\nn[::1]:1420\nn127.0.0.1:1420\np202\ncpython\nn*:8000\np303\nclan\nn192.168.1.5:9000\n",
        );
        assert_eq!(listeners.len(), 2);
        assert!(listeners.contains(&Listener {
            pid: 101,
            process: "node".to_owned(),
            port: 1420,
        }));
        assert!(listeners.contains(&Listener {
            pid: 202,
            process: "python".to_owned(),
            port: 8000,
        }));
        assert!(!listeners.iter().any(|listener| listener.pid == 303));
        let cwds = parse_lsof_cwds("p101\nfcwd\nn/tmp/project\np202\nfcwd\nn/tmp/other\n");
        assert_eq!(cwds.get(&101), Some(&PathBuf::from("/tmp/project")));
        assert_eq!(cwds.get(&202), Some(&PathBuf::from("/tmp/other")));
    }

    #[test]
    fn filters_supported_files_and_excluded_directories() {
        assert_eq!(artifact_kind(Path::new("screen.PNG")), Some("image"));
        assert_eq!(artifact_kind(Path::new("notes.md")), Some("doc"));
        assert_eq!(artifact_kind(Path::new("data.json")), None);
        assert!(has_ignored_component(Path::new(
            "node_modules/pkg/image.png"
        )));
        assert!(has_ignored_component(Path::new(".cache/image.png")));
        assert!(has_ignored_component(Path::new("target/doc/index.html")));
        assert!(!has_ignored_component(Path::new("assets/image.png")));
    }

    #[test]
    fn parses_porcelain_status_including_renames() {
        // Переименование в `-z` занимает две записи: новый путь, затем старый — старый пропускаем.
        let output = b" M src/a.rs\0?? new.md\0R  src/new.rs\0src/old.rs\0A  img/shot.png\0";
        let paths = parse_status_paths(output);
        assert_eq!(
            paths,
            vec![
                PathBuf::from("src/a.rs"),
                PathBuf::from("new.md"),
                PathBuf::from("src/new.rs"),
                PathBuf::from("img/shot.png"),
            ]
        );
        assert!(parse_status_paths(b"").is_empty());
    }

    #[test]
    fn diff_boundary_rejects_paths_outside_worktree() {
        let worktree = temporary_directory("worktree");
        let outside = temporary_directory("elsewhere");
        let inside = worktree.join("file.rs");
        fs::write(&inside, "fn main() {}").unwrap();
        fs::write(outside.join("file.rs"), "fn other() {}").unwrap();

        let (path, root) = ensure_within(&inside, &worktree).unwrap();
        assert!(path.starts_with(&root));
        assert!(ensure_within(&outside.join("file.rs"), &worktree).is_err());
        assert!(ensure_within(&worktree, &worktree).is_err());
        // Ещё не созданный файл внутри ворктри (untracked-кандидат) проходит границу.
        assert!(ensure_within(&worktree.join("fresh.md"), &worktree).is_ok());
        assert!(ensure_within(&worktree.join("../escape.md"), &worktree).is_err());

        fs::remove_dir_all(worktree).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn truncates_diff_on_utf8_boundary_with_marker() {
        let diff = "я".repeat(MAX_DIFF_BYTES);
        let truncated = truncate_diff(diff);
        assert!(truncated.ends_with(TRUNCATION_MARKER));
        assert!(truncated.len() <= MAX_DIFF_BYTES);
        assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
        assert_eq!(truncate_diff("short".to_owned()), "short");
    }

    #[test]
    fn preview_read_enforces_root_extension_and_symlink_boundary() {
        let allowed = temporary_directory("allowed");
        let outside = temporary_directory("outside");
        let markdown = allowed.join("result.md");
        let binary = allowed.join("result.png");
        let secret = outside.join("secret.md");
        fs::write(&markdown, "# Готово").unwrap();
        fs::write(&binary, "not an image").unwrap();
        fs::write(&secret, "private").unwrap();

        assert_eq!(
            preview_read_from_roots(&markdown, std::slice::from_ref(&allowed)).unwrap(),
            "# Готово"
        );
        assert!(preview_read_from_roots(&binary, std::slice::from_ref(&allowed)).is_err());
        assert!(preview_read_from_roots(&secret, std::slice::from_ref(&allowed)).is_err());

        #[cfg(unix)]
        {
            let link = allowed.join("link.md");
            std::os::unix::fs::symlink(&secret, &link).unwrap();
            assert!(preview_read_from_roots(&link, std::slice::from_ref(&allowed)).is_err());
        }

        fs::remove_dir_all(allowed).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
}

#[cfg(test)]
mod live {
    use super::*;

    #[tokio::test]
    #[ignore = "требует pnpm dev на localhost:1420"]
    async fn detects_vite_1420_for_orchestra_project() {
        let project = dirs::home_dir().unwrap().join("projects/orchestra");
        let sources = sources_for_project(&project)
            .await
            .expect("preview sources");
        assert!(sources.iter().any(|source| {
            source.kind == "url" && source.url.as_deref() == Some("http://localhost:1420")
        }));
    }
}
