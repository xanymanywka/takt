//! Платформенный слой (SPEC §13): всё, что зависит от ОС, живёт здесь — login-оболочка,
//! PATH для инструментов, `~/.orchestra`, внешний терминал, наличие tmux и имя python.
//! Остальные модули зовут только эти функции и не ветвятся по `cfg!` сами.
//! На macOS поведение совпадает с прежним: `zsh -lc`, Terminal.app через osascript.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlatformKind {
    MacOs,
    Linux,
    Windows,
}

/// Платформа, под которую собрано ядро; все остальные варианты unix считаем Linux.
fn current_platform() -> PlatformKind {
    if cfg!(target_os = "windows") {
        PlatformKind::Windows
    } else if cfg!(target_os = "macos") {
        PlatformKind::MacOs
    } else {
        PlatformKind::Linux
    }
}

/// Есть ли исполняемый файл: абсолютный путь проверяется напрямую, короткое имя — по `tool_path()`;
/// на Windows подставляются расширения `.exe`/`.cmd`/`.bat`.
fn executable_exists(program: &str) -> bool {
    let path = Path::new(program);
    if path.components().count() > 1 {
        return path.is_file();
    }

    let extensions: &[&str] = if cfg!(target_os = "windows") {
        &["", ".exe", ".cmd", ".bat"]
    } else {
        &[""]
    };
    std::env::split_paths(&tool_path()).any(|directory| {
        extensions
            .iter()
            .any(|extension| directory.join(format!("{program}{extension}")).is_file())
    })
}

/// Выбор оболочки без обращения к ОС (`exists` подменяется в тестах): macOS — `zsh -lc`,
/// Linux — `$SHELL -lc`, если он задан и существует, иначе `bash -lc`,
/// Windows — `pwsh -NoProfile -Command` с фолбэком на встроенный `powershell`.
fn login_shell_for(
    platform: PlatformKind,
    configured_shell: Option<&str>,
    exists: impl Fn(&str) -> bool,
) -> (String, Vec<String>) {
    match platform {
        PlatformKind::MacOs => ("zsh".to_owned(), vec!["-lc".to_owned()]),
        PlatformKind::Linux => {
            let shell = configured_shell
                .filter(|shell| exists(shell))
                .unwrap_or("bash");
            (shell.to_owned(), vec!["-lc".to_owned()])
        }
        PlatformKind::Windows => {
            let shell = if exists("pwsh") { "pwsh" } else { "powershell" };
            (
                shell.to_owned(),
                vec!["-NoProfile".to_owned(), "-Command".to_owned()],
            )
        }
    }
}

/// Оболочка для разовых команд `run_shell`: программа и префикс аргументов, после которого
/// идёт сама команда одной строкой.
pub fn login_shell() -> (String, Vec<String>) {
    let configured_shell = std::env::var("SHELL").ok();
    login_shell_for(
        current_platform(),
        configured_shell.as_deref(),
        executable_exists,
    )
}

/// Интерактивная оболочка для встроенного терминала (`pty_open`): на unix login-режим `-l`,
/// на Windows `pwsh -NoLogo`.
pub fn interactive_shell() -> (String, Vec<String>) {
    let (program, _) = login_shell();
    let args = if cfg!(target_os = "windows") {
        vec!["-NoLogo".to_owned()]
    } else {
        vec!["-l".to_owned()]
    };
    (program, args)
}

/// Сборка PATH из готовых входов (без обращения к ОС, чтобы тестировать Windows-ветку на macOS).
/// Unix: `~/.local/bin`, Homebrew, `/usr/local/bin`, системные папки, затем унаследованный PATH.
/// Windows: `%LOCALAPPDATA%\Programs\*`, `%APPDATA%\npm`, `~\.cargo\bin`, `~\scoop\shims`,
/// затем системный PATH.
fn build_tool_path(
    platform: PlatformKind,
    home: &Path,
    inherited: &str,
    local_app_data: Option<&Path>,
    app_data: Option<&Path>,
    program_directories: &[PathBuf],
) -> String {
    fn windows_join(base: &Path, child: &str) -> String {
        format!(
            "{}\\{}",
            base.to_string_lossy().trim_end_matches(['\\', '/']),
            child.replace('/', "\\")
        )
    }

    let separator = match platform {
        PlatformKind::Windows => ';',
        PlatformKind::MacOs | PlatformKind::Linux => ':',
    };

    if matches!(platform, PlatformKind::MacOs | PlatformKind::Linux) {
        return format!(
            "{}:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:{inherited}",
            home.join(".local/bin").display()
        );
    }

    let mut entries = Vec::new();
    match platform {
        PlatformKind::Windows => {
            entries.extend(
                program_directories
                    .iter()
                    .map(|path| path.display().to_string()),
            );
            if let Some(path) = app_data {
                entries.push(windows_join(path, "npm"));
            }
            entries.push(windows_join(home, ".cargo/bin"));
            entries.push(windows_join(home, "scoop/shims"));
            if program_directories.is_empty()
                && let Some(path) = local_app_data
            {
                entries.push(windows_join(path, "Programs"));
            }
        }
        PlatformKind::MacOs | PlatformKind::Linux => unreachable!(),
    }

    entries.extend(
        inherited
            .split(separator)
            .filter(|entry| !entry.is_empty())
            .map(ToOwned::to_owned),
    );
    entries.dedup();
    entries.join(&separator.to_string())
}

/// PATH, в котором ядро ищет claude/codex/agy/gh/codexbar и прочие инструменты.
pub fn tool_path() -> String {
    let home = dirs::home_dir().unwrap_or_default();
    let inherited = std::env::var("PATH").unwrap_or_default();
    if cfg!(target_os = "windows") {
        // Установщики Windows (Claude, Codex, VS Code…) кладут программы в отдельные папки
        // внутри %LOCALAPPDATA%\Programs — добавляем каждую, раз они не в системном PATH.
        let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
        let app_data = std::env::var_os("APPDATA").map(PathBuf::from);
        let mut programs = local_app_data
            .as_ref()
            .map(|path| path.join("Programs"))
            .and_then(|path| fs::read_dir(path).ok())
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>();
        programs.sort();
        build_tool_path(
            PlatformKind::Windows,
            &home,
            &inherited,
            local_app_data.as_deref(),
            app_data.as_deref(),
            &programs,
        )
    } else {
        build_tool_path(current_platform(), &home, &inherited, None, None, &[])
    }
}

/// `~/.orchestra` на всех платформах: там живут `jobs.json`, `agents.py`, `INSTRUCTIONS.md`.
pub fn orchestra_home() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".orchestra")
}

/// Экранирование аргумента для POSIX-оболочки одинарными кавычками.
fn posix_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Строковый литерал AppleScript для `osascript -e`.
fn applescript_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Эмуляторы терминала Linux в порядке предпочтения: сначала системный `x-terminal-emulator`
/// (Debian alternatives), затем GNOME, KDE и классический xterm.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LinuxTerminal {
    Default,
    Gnome,
    Konsole,
    Xterm,
}

impl LinuxTerminal {
    fn program(self) -> &'static str {
        match self {
            Self::Default => "x-terminal-emulator",
            Self::Gnome => "gnome-terminal",
            Self::Konsole => "konsole",
            Self::Xterm => "xterm",
        }
    }
}

/// Первый установленный терминал из списка; `exists` подменяется в тестах.
fn select_linux_terminal(mut exists: impl FnMut(&str) -> bool) -> Option<LinuxTerminal> {
    [
        LinuxTerminal::Default,
        LinuxTerminal::Gnome,
        LinuxTerminal::Konsole,
        LinuxTerminal::Xterm,
    ]
    .into_iter()
    .find(|terminal| exists(terminal.program()))
}

fn ensure_success(output: std::process::Output, label: &str) -> Result<(), String> {
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{label} вернул ошибку: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// macOS: Terminal.app через osascript — тот же сценарий, что раньше жил в `setup.rs`.
fn open_macos_terminal(cwd: &Path, command: Option<&str>) -> Result<(), String> {
    let mut shell = format!("cd {}", posix_quote(&cwd.to_string_lossy()));
    if let Some(command) = command.filter(|command| !command.trim().is_empty()) {
        shell.push_str(" && ");
        shell.push_str(command);
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
    ensure_success(output, "Terminal")
}

/// Windows: `wt.exe -d <cwd> pwsh -NoExit -Command <cmd>`; если Windows Terminal нет или он
/// вернул ошибку — отдельное окно через `cmd /c start pwsh …`.
#[cfg(target_os = "windows")]
fn open_windows_terminal(cwd: &Path, command: Option<&str>) -> Result<(), String> {
    let shell = if executable_exists("pwsh") {
        "pwsh"
    } else {
        "powershell"
    };
    let mut args = vec![
        "-d".to_owned(),
        cwd.to_string_lossy().into_owned(),
        shell.to_owned(),
        "-NoExit".to_owned(),
    ];
    if let Some(command) = command.filter(|command| !command.trim().is_empty()) {
        args.extend(["-Command".to_owned(), command.to_owned()]);
    }
    match Command::new("wt.exe").args(&args).output() {
        Ok(output) if output.status.success() => Ok(()),
        Ok(_) | Err(_) => {
            let mut fallback = vec![
                "/c".to_owned(),
                "start".to_owned(),
                "".to_owned(),
                shell.to_owned(),
                "-NoExit".to_owned(),
            ];
            if let Some(command) = command.filter(|command| !command.trim().is_empty()) {
                fallback.extend(["-Command".to_owned(), command.to_owned()]);
            }
            let output = Command::new("cmd.exe")
                .args(fallback)
                .current_dir(cwd)
                .output()
                .map_err(|error| format!("Не удалось открыть терминал: {error}"))?;
            ensure_success(output, "Терминал")
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn open_windows_terminal(_cwd: &Path, _command: Option<&str>) -> Result<(), String> {
    Err("Windows Terminal доступен только на Windows".to_owned())
}

/// Linux: первый найденный терминал выполняет `cd && <cmd>`, затем остаётся открытым
/// с login-оболочкой пользователя (`exec $SHELL -l`), как Terminal.app на macOS.
fn open_linux_terminal(cwd: &Path, command: Option<&str>) -> Result<(), String> {
    let terminal = select_linux_terminal(executable_exists)
        .ok_or_else(|| "Не найден поддерживаемый терминал Linux".to_owned())?;
    let (shell, _) = login_shell();
    let mut script = format!("cd {}", posix_quote(&cwd.to_string_lossy()));
    if let Some(command) = command.filter(|command| !command.trim().is_empty()) {
        script.push_str(" && ");
        script.push_str(command);
    }
    script.push_str("; exec ");
    script.push_str(&posix_quote(&shell));
    script.push_str(" -l");

    let mut process = Command::new(terminal.program());
    if terminal == LinuxTerminal::Gnome {
        process.arg("--");
    } else {
        process.arg("-e");
    }
    process
        .arg(&shell)
        .arg("-lc")
        .arg(script)
        .spawn()
        .map_err(|error| format!("Не удалось открыть терминал Linux: {error}"))?;
    Ok(())
}

/// Открыть внешний терминал в папке `cwd` и, если задано, выполнить там `command`.
/// Команду сюда пропускает только белый список `setup::TERMINAL_ACTIONS`.
pub fn open_terminal(cwd: &Path, command: Option<&str>) -> Result<(), String> {
    if !cwd.is_dir() {
        return Err(format!("Папка не найдена: {}", cwd.display()));
    }
    match current_platform() {
        PlatformKind::MacOs => open_macos_terminal(cwd, command),
        PlatformKind::Windows => open_windows_terminal(cwd, command),
        PlatformKind::Linux => open_linux_terminal(cwd, command),
    }
}

/// Есть ли tmux — тогда исполнители идут через `agents.py`; иначе (и всегда на Windows)
/// `jobs.rs` запускает их напрямую через PTY ядра.
pub fn has_tmux() -> bool {
    !cfg!(target_os = "windows") && executable_exists("tmux")
}

/// Имя интерпретатора для `agents.py`: `python3` на unix, `python` на Windows.
pub fn python() -> &'static str {
    if cfg!(target_os = "windows") {
        "python"
    } else {
        "python3"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_selection_covers_all_platforms() {
        assert_eq!(
            login_shell_for(PlatformKind::MacOs, None, |_| false),
            ("zsh".to_owned(), vec!["-lc".to_owned()])
        );
        assert_eq!(
            login_shell_for(PlatformKind::Linux, Some("/bin/fish"), |path| path
                == "/bin/fish"),
            ("/bin/fish".to_owned(), vec!["-lc".to_owned()])
        );
        assert_eq!(
            login_shell_for(PlatformKind::Linux, Some("/missing"), |_| false).0,
            "bash"
        );
        assert_eq!(
            login_shell_for(PlatformKind::Windows, None, |name| name == "pwsh").0,
            "pwsh"
        );
        assert_eq!(
            login_shell_for(PlatformKind::Windows, None, |_| false).0,
            "powershell"
        );
    }

    #[test]
    fn current_shell_matches_compile_target() {
        let (program, args) = login_shell();
        if cfg!(target_os = "windows") {
            assert!(matches!(program.as_str(), "pwsh" | "powershell"));
            assert_eq!(args, ["-NoProfile", "-Command"]);
        } else if cfg!(target_os = "macos") {
            assert_eq!(program, "zsh");
            assert_eq!(args, ["-lc"]);
        } else {
            assert!(!program.is_empty());
            assert_eq!(args, ["-lc"]);
        }
    }

    #[test]
    fn tool_path_contains_platform_directories_before_inherited_path() {
        let unix = build_tool_path(
            PlatformKind::Linux,
            Path::new("/home/test"),
            "/custom/bin",
            None,
            None,
            &[],
        );
        assert!(unix.starts_with("/home/test/.local/bin:/opt/homebrew/bin:/usr/local/bin:"));
        assert!(unix.ends_with(":/custom/bin"));

        let windows = build_tool_path(
            PlatformKind::Windows,
            Path::new(r"C:\Users\test"),
            r"C:\Windows\System32",
            Some(Path::new(r"C:\Users\test\AppData\Local")),
            Some(Path::new(r"C:\Users\test\AppData\Roaming")),
            &[PathBuf::from(r"C:\Users\test\AppData\Local\Programs\Tool")],
        );
        assert!(windows.contains(r"Programs\Tool"));
        assert!(windows.contains(r"Roaming\npm"));
        assert!(windows.contains(r".cargo\bin"));
        assert!(windows.contains(r"scoop\shims"));
        assert!(windows.ends_with(r"C:\Windows\System32"));
    }

    #[test]
    fn terminal_selection_uses_first_available_candidate() {
        assert_eq!(
            select_linux_terminal(|name| matches!(name, "konsole" | "xterm")),
            Some(LinuxTerminal::Konsole)
        );
        assert_eq!(select_linux_terminal(|_| false), None);
        assert_eq!(
            select_linux_terminal(|name| name == "x-terminal-emulator"),
            Some(LinuxTerminal::Default)
        );
    }
}
