use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

struct PtyProcess {
    master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
}

#[derive(Clone, Default)]
pub struct PtyState(Arc<Mutex<HashMap<String, PtyProcess>>>);

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Serialize)]
struct PtyData {
    id: String,
    data: String,
}

#[derive(Clone, Serialize)]
struct PtyExit {
    id: String,
    code: i32,
}

fn size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        rows: rows.max(1),
        cols: cols.max(1),
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn decode_available(pending: &mut Vec<u8>, eof: bool) -> String {
    let mut decoded = String::new();
    loop {
        match std::str::from_utf8(pending) {
            Ok(valid) => {
                decoded.push_str(valid);
                pending.clear();
                break;
            }
            Err(error) if error.error_len().is_none() && !eof => {
                let valid = error.valid_up_to();
                if valid > 0 {
                    decoded.push_str(std::str::from_utf8(&pending[..valid]).expect("valid prefix"));
                    pending.drain(..valid);
                }
                break;
            }
            Err(error) => {
                let consumed = if eof {
                    pending.len()
                } else {
                    error.valid_up_to() + error.error_len().unwrap_or(1)
                };
                decoded.push_str(&String::from_utf8_lossy(&pending[..consumed]));
                pending.drain(..consumed);
                if pending.is_empty() {
                    break;
                }
            }
        }
    }
    decoded
}

#[tauri::command(async)]
pub fn pty_open(
    app: AppHandle,
    state: State<'_, PtyState>,
    cwd: String,
    cols: u16,
    rows: u16,
) -> Result<String, String> {
    if !std::path::Path::new(&cwd).is_dir() {
        return Err(format!("Папка не найдена: {cwd}"));
    }
    let pair = native_pty_system()
        .openpty(size(cols, rows))
        .map_err(|error| format!("Не удалось открыть PTY: {error}"))?;
    let mut command = CommandBuilder::new("zsh");
    command.arg("-l");
    command.cwd(&cwd);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    command.env_remove("ANTHROPIC_BASE_URL");
    command.env_remove("ANTHROPIC_AUTH_TOKEN");
    command.env_remove("ANTHROPIC_SMALL_FAST_MODEL");
    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("Не удалось запустить zsh: {error}"))?;
    drop(pair.slave);
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("Не удалось открыть чтение PTY: {error}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| format!("Не удалось открыть запись PTY: {error}"))?;
    let child = Arc::new(Mutex::new(child));
    let id = format!(
        "pty-{}-{}",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    );
    state
        .0
        .lock()
        .map_err(|_| "Не удалось заблокировать список PTY".to_owned())?
        .insert(
            id.clone(),
            PtyProcess {
                master: pair.master,
                writer: Arc::new(Mutex::new(writer)),
                child: child.clone(),
            },
        );

    let event_id = id.clone();
    let event_app = app.clone();
    let pty_state = state.inner().clone();
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        let mut pending = Vec::new();
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    pending.extend_from_slice(&buffer[..count]);
                    let data = decode_available(&mut pending, false);
                    if data.is_empty() {
                        continue;
                    }
                    let _ = event_app.emit(
                        "pty://data",
                        PtyData {
                            id: event_id.clone(),
                            data,
                        },
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        let tail = decode_available(&mut pending, true);
        if !tail.is_empty() {
            let _ = event_app.emit(
                "pty://data",
                PtyData {
                    id: event_id.clone(),
                    data: tail,
                },
            );
        }
        let code = loop {
            let status = child
                .lock()
                .ok()
                .and_then(|mut child| child.try_wait().ok())
                .flatten();
            if let Some(status) = status {
                break status.exit_code() as i32;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        };
        if let Ok(mut processes) = pty_state.0.lock() {
            processes.remove(&event_id);
        }
        let _ = event_app.emit("pty://exit", PtyExit { id: event_id, code });
    });
    Ok(id)
}

#[tauri::command(async)]
pub fn pty_write(state: State<'_, PtyState>, id: String, data: String) -> Result<(), String> {
    let writer = state
        .0
        .lock()
        .map_err(|_| "Не удалось заблокировать список PTY".to_owned())?
        .get(&id)
        .map(|process| process.writer.clone())
        .ok_or_else(|| format!("PTY не найден: {id}"))?;
    let mut writer = writer
        .lock()
        .map_err(|_| "Не удалось заблокировать PTY".to_owned())?;
    writer
        .write_all(data.as_bytes())
        .and_then(|_| writer.flush())
        .map_err(|error| format!("Не удалось записать в PTY: {error}"))
}

#[tauri::command(async)]
pub fn pty_resize(
    state: State<'_, PtyState>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let processes = state
        .0
        .lock()
        .map_err(|_| "Не удалось заблокировать список PTY".to_owned())?;
    let process = processes
        .get(&id)
        .ok_or_else(|| format!("PTY не найден: {id}"))?;
    process
        .master
        .resize(size(cols, rows))
        .map_err(|error| format!("Не удалось изменить размер PTY: {error}"))
}

#[tauri::command(async)]
pub fn pty_close(state: State<'_, PtyState>, id: String) -> Result<(), String> {
    let process = state
        .0
        .lock()
        .map_err(|_| "Не удалось заблокировать список PTY".to_owned())?
        .remove(&id)
        .ok_or_else(|| format!("PTY не найден: {id}"))?;
    process
        .child
        .lock()
        .map_err(|_| "Не удалось заблокировать процесс PTY".to_owned())?
        .kill()
        .map_err(|error| format!("Не удалось закрыть PTY: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    #[ignore = "запускает реальный zsh -l с профилем пользователя"]
    fn live_zsh_pty_emits_prompt_and_pwd() {
        let cwd = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .canonicalize()
            .unwrap();
        let pair = native_pty_system().openpty(size(100, 30)).unwrap();
        let mut command = CommandBuilder::new("zsh");
        command.arg("-l");
        command.cwd(&cwd);
        let mut child = pair.slave.spawn_command(command).unwrap();
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();
        let mut writer = pair.master.take_writer().unwrap();
        writer.write_all(b"pwd\nexit\n").unwrap();
        writer.flush().unwrap();
        drop(writer);

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut output = String::new();
            let _ = reader.read_to_string(&mut output);
            let _ = tx.send(output);
        });
        let output = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("PTY output timeout");
        let status = child.wait().unwrap();
        println!("{}", output.replace('\r', ""));
        assert!(status.success());
        assert!(output.contains(&cwd.to_string_lossy().into_owned()));
        assert!(output.contains('%') || output.contains('$') || output.contains('#'));
    }

    #[test]
    fn utf8_decoder_keeps_split_character() {
        let bytes = "готов 🎼".as_bytes();
        let split = bytes.len() - 2;
        let mut pending = bytes[..split].to_vec();
        let first = decode_available(&mut pending, false);
        pending.extend_from_slice(&bytes[split..]);
        let second = decode_available(&mut pending, false);
        assert_eq!(format!("{first}{second}"), "готов 🎼");
        assert!(pending.is_empty());
    }
}
