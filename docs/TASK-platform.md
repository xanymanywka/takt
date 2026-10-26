# TASK: Платформенный слой и сборки Windows/Linux (SPEC §13)

## Цель
Ядро Takt собирается и запускается на macOS, Windows и Linux. Всё, что зависит от ОС (оболочка, домашняя папка,
внешний терминал, запуск исполнителей), собрано в одном модуле `platform.rs`; на Windows исполнители работают
без tmux через PTY ядра; `release.yml` собирает три платформы и один общий `latest.json`.

## Объём
- Новый модуль `src-tauri/src/platform.rs`:
  - `pub fn login_shell() -> (program, args_prefix)`: macOS `zsh -lc`, Linux `bash -lc` (или `$SHELL -lc`, если он
    задан и существует), Windows `pwsh -NoProfile -Command` с фолбэком на `powershell`.
  - `pub fn tool_path() -> String`: PATH с `~/.local/bin`, `/opt/homebrew/bin`, `/usr/local/bin` на unix;
    на Windows — `%LOCALAPPDATA%\Programs\*`, `%APPDATA%\npm`, `%USERPROFILE%\.cargo\bin`, `scoop\shims` + системный PATH.
  - `pub fn orchestra_home() -> PathBuf` — `~/.orchestra` везде (`dirs::home_dir()`).
  - `pub fn open_terminal(cwd, command) -> Result<(), String>`: macOS — текущая реализация через osascript
    (перенести из `setup.rs`), Windows — `wt.exe -d <cwd> pwsh -NoExit -Command <cmd>` с фолбэком
    `cmd /c start pwsh ...`, Linux — `x-terminal-emulator`, иначе `gnome-terminal`/`konsole`/`xterm`,
    первый найденный.
  - `pub fn has_tmux() -> bool`, `pub fn python() -> &str` (`python3` на unix, `python` на Windows).
- `shell.rs`: `run_zsh` → `run_shell` поверх `platform::login_shell()`/`platform::tool_path()`; старое имя оставить
  как `#[deprecated]`-обёртку или переименовать во всех вызовах (grep по репо). Сохранить вычистку
  `ANTHROPIC_*` переменных.
- `setup.rs`: `open_in_terminal` зовёт `platform::open_terminal`; белый список `TERMINAL_ACTIONS` остаётся,
  но команды установки — по платформе (macOS `brew install tmux`, Linux `sudo apt install tmux`, Windows — для tmux
  действие недоступно → `Err("На Windows tmux не нужен: исполнители запускаются через встроенный терминал")`).
  Проверка `agents.py`: на Windows инструмент `agents` считается `found: true, warn: true,
  statusText: "Запуск через PTY ядра"`.
- `jobs.rs`: при отсутствии tmux (или на Windows) `job_run` запускает исполнителя напрямую через
  `portable-pty` (как `pty.rs`) с теми же командами из `docs/SPEC.md` §3 оркестровки
  (`codex -c model_reasoning_effort=high --dangerously-bypass-approvals-and-sandbox "<prompt>"`,
  `agy --dangerously-skip-permissions --effort high -i "<prompt>"`, `claude --dangerously-skip-permissions "<prompt>"`),
  cwd = ворктри, запись в `~/.orchestra/jobs.json` в том же формате, что пишет `agents.py`
  (посмотри его структуру в `~/.orchestra/agents.py`, функция регистрации задачи), поле `"runner": "pty"`.
  Экран таких задач хранится в памяти ядра (кольцевой буфер 2000 строк на задачу) и отдаётся в `job_screen`;
  `job_send` пишет в PTY + `\r`; `job_kill` убивает процесс. Задачи `runner: "tmux"` — как раньше, через `agents.py`.
- `.github/workflows/release.yml`: матрица `macos-latest` (universal, как сейчас), `windows-latest`
  (`.msi` + NSIS `.exe`), `ubuntu-22.04` (`.deb` + `.AppImage`, зависимости `libwebkit2gtk-4.1-dev
  libappindicator3-dev librsvg2-dev patchelf`), все с `includeUpdaterJson: true` в один релиз (tauri-action
  сам сливает `latest.json`). `.github/workflows/ci.yml`: `cargo test --locked` на трёх ОС (матрица), tsc и
  i18n — один раз на ubuntu.
- `src-tauri/tauri.conf.json`: `bundle.targets` оставить `"all"`; добавить `bundle.windows` (`webviewInstallMode:
  downloadBootstrapper`), `bundle.linux.deb.depends` при необходимости; иконки `.ico` уже есть в `icons/` —
  проверить, что `bundle.icon` их включает.
- Тесты: выбор оболочки по `cfg!(target_os)`, сборка PATH, выбор терминала (мок наличия бинарей через
  параметр), разбор/запись `jobs.json` с `runner`.

## Не делать
- Не трогать `src/` (фронт), `conductor.rs`, `models.rs`, `limits.rs`, `projects.rs`, `preview.rs`, `companion.rs`
  (их пишут соседние исполнители).
- Не менять версию, CHANGELOG, README. Не ставить теги, не публиковать релиз.
- Не ломать macOS-путь: всё текущее поведение на macOS должно остаться байт-в-байт тем же.

## Кто что правит
- Ты: `src-tauri/src/platform.rs` (новый), `src-tauri/src/shell.rs`, `src-tauri/src/setup.rs`, `src-tauri/src/jobs.rs`,
  `src-tauri/src/pty.rs` (только если нужно вынести общий PTY-хелпер), `src-tauri/src/lib.rs` (строка `mod platform;`),
  `src-tauri/tauri.conf.json` (только блоки `bundle.windows`/`bundle.linux`/`bundle.icon`),
  `.github/workflows/release.yml`, `.github/workflows/ci.yml`, `src-tauri/Cargo.toml` при необходимости.
- Не трогать: `src/`, другие `.rs`, CSP/`assetProtocol` в `tauri.conf.json` (правит соседний исполнитель).

## Контекст
- Спека: `docs/SPEC.md` §7, §13; логика исполнителей и формат задач — `~/Documents/orchestration-logic.md`
  (прочитай разделы 3, 4, 6, 9) и `~/.orchestra/agents.py` (эталон, что именно пишется в `jobs.json`).
- `shell::run_zsh` сейчас используется в `jobs.rs`, `limits.rs`, `models.rs`, `setup.rs`, `conductor.rs` — меняй
  сигнатуру совместимо, чужие файлы не правь (оставь `run_zsh` как алиас).
- Локально только macOS: Windows/Linux проверяются `cargo check --target` там, где это возможно без тулчейна
  (хотя бы `cargo check` на macOS + `cfg` ветки покрыты юнит-тестами) и в CI-матрице.
- Tauri 2.12, Rust edition 2024.

## Приёмка
- [ ] `cd src-tauri && cargo test --lib` → зелёный, новые тесты platform/jobs runner
- [ ] `cd src-tauri && cargo check` без предупреждений в новом коде
- [ ] `grep -rn "zsh" src-tauri/src --include=*.rs` → `zsh` встречается только в `platform.rs`
- [ ] Живой тест на macOS: `cargo test --lib -- --ignored jobs::live_pty_runner` запускает `claude --dangerously-skip-permissions "скажи ok и выйди"` через PTY-раннер в временной ворктри, экран содержит ответ, запись `runner: "pty"` появляется в `jobs.json` и убирается после `job_kill`
- [ ] `pnpm tauri dev` на macOS работает как раньше (чат, терминал, реестр задач)
- [ ] `act`-прогон не требуется; достаточно, что YAML валиден (`node -e "require('yaml')"` нет в зависимостях — проверь `pnpm dlx yaml-lint .github/workflows/*.yml` или аккуратно глазами) и в отчёте перечислены job-ы матрицы

## Обязательные правила
- Коммиты только от xanymanywka
  (`git -c user.name="xanymanywka" -c user.email="faust.9o8@gmail.com" commit ...`),
  без Co-Authored-By, без упоминаний ИИ в коммитах и README.
- Секреты в файлы и коммиты не класть.
- Никакого безымянного `git stash`.
- Продакшн не деплоить, теги не ставить, `main` не трогать — в конце отчёт дирижёру по пунктам приёмки с выводом команд.
