mod conductor;
mod jobs;
mod limits;
mod models;
mod preview;
mod projects;
mod pty;
mod setup;
mod shell;

use conductor::{
    ConductorState, conductor_config, conductor_send, conductor_start, conductor_stop,
    set_conductor,
};
use jobs::{job_kill, job_new, job_run, job_screen, job_send, list_jobs, write_task};
use limits::limits;
use models::{list_models, save_model};
use preview::{preview_diff, preview_read, preview_sources};
use projects::{list_projects, save_projects};
use pty::{PtyState, pty_close, pty_open, pty_resize, pty_write};
use setup::{open_in_terminal, setup_status, write_instructions};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .manage(ConductorState::default())
        .manage(PtyState::default())
        .invoke_handler(tauri::generate_handler![
            list_projects,
            save_projects,
            list_jobs,
            job_screen,
            job_send,
            job_kill,
            job_new,
            job_run,
            write_task,
            limits,
            setup_status,
            write_instructions,
            conductor_start,
            conductor_send,
            conductor_stop,
            conductor_config,
            set_conductor,
            pty_open,
            pty_write,
            pty_resize,
            pty_close,
            open_in_terminal,
            list_models,
            save_model,
            preview_sources,
            preview_diff,
            preview_read,
        ])
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            jobs::start_watcher(app.handle().clone());
            limits::start_poller(app.handle().clone());
            preview::start_watcher(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
