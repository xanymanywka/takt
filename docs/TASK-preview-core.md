# TASK: Ядро предпросмотра результата (SPEC §11, часть Rust)

## Цель
Ядро Tauri умеет сказать фронту, «что получилось» у исполнителя в ворктри: открытые dev-серверы на
localhost, изменённые/созданные файлы-артефакты (картинки, html, svg, pdf, md) и файлы с диффом.
Фронт (отдельный исполнитель, другая ворктри) рисует панель «Результат» по этому контракту на моках.

## Объём
- Новый модуль `src-tauri/src/preview.rs`:
  - `#[tauri::command] preview_sources(project_path: String) -> Result<Vec<PreviewSource>, String>`.
    `PreviewSource { kind: "url"|"image"|"doc"|"diff", path: Option<String>, url: Option<String>, title: String, mtime: String /* RFC3339 */ }`
    (serde `rename_all = "camelCase"`, фронт получает `kind/path/url/title/mtime`).
  - Источники:
    1. `url` — dev-серверы: процессы, чей cwd лежит внутри `project_path` (включая ворктри `~/orchestra/<repo>/*`
       этого репозитория), слушающие TCP на localhost. Реализация через `lsof -iTCP -sTCP:LISTEN -P -n -Fpcn` +
       `lsof -a -d cwd -p <pid> -Fn` (или `/proc`-независимый аналог для macOS). Заголовок — имя процесса и порт
       (`vite :1420`), url `http://localhost:<port>`.
    2. `image` / `doc` — файлы в `project_path` и его ворктри, изменённые за последние 24 ч, с расширениями
       png jpg jpeg gif webp svg (image) и html htm md pdf (doc); исключать `node_modules`, `target`, `dist`,
       `.git`, скрытые папки. Сортировка по mtime убыв., не больше 60 записей.
    3. `diff` — файлы из `git status --porcelain` (изменённые/новые) в `project_path` и в каждой ворктри
       (`git worktree list --porcelain`), kind `diff`, path абсолютный, title относительный путь.
  - `#[tauri::command] preview_diff(path: String, worktree: String) -> Result<String, String>` — текст
    `git diff -- <path>` (для untracked: `git diff --no-index /dev/null <path>`), не больше 200 КБ, остальное
    обрезать с пометкой.
  - `#[tauri::command] preview_read(path: String) -> Result<String, String>` — читает md/html/svg (текст) только
    если файл внутри одного из проектов из `~/.orchestra/projects.json` (см. `projects.rs`) или внутри
    `~/orchestra/`; иначе `Err`. Картинки и pdf фронт открывает через asset protocol, не через эту команду.
  - `pub fn start_watcher(app: AppHandle)` — `notify` на корни всех проектов из `projects.json` и `~/orchestra`,
    дебаунс 700 мс, + опрос портов раз в 10 с; при изменении набора источников эмитит
    `preview://changed` с payload `{ projectPath: String, sources: Vec<PreviewSource> }` для каждого
    затронутого проекта. Следить рекурсивно, но игнорировать те же папки-исключения.
- Регистрация в `src-tauri/src/lib.rs` (`mod preview;`, три команды в `generate_handler!`,
  `preview::start_watcher(app.handle().clone())` в `setup`).
- `src-tauri/capabilities/default.json`: включить asset protocol для картинок/pdf —
  `"core:webview:default"` не нужно; добавить в `tauri.conf.json` `app.security.assetProtocol`
  `{ "enable": true, "scope": ["$HOME/orchestra/**", "$HOME/projects/**"] }` и расширить CSP `img-src`
  значениями `asset: http://asset.localhost`, `frame-src http://localhost:* http://127.0.0.1:* asset: http://asset.localhost`.
  Scope проектов вне `~/projects` — через `tauri::scope::fs` в рантайме (`app.asset_protocol_scope().allow_directory`) при чтении `projects.json`.
- Юнит-тесты (без живых процессов): разбор вывода `lsof -F`, фильтр расширений/исключений, обрезка диффа,
  проверка границ `preview_read`. Живые проверки — `#[ignore]`.

## Не делать
- Не трогать фронт (`src/`), `jobs.rs`, `setup.rs`, `shell.rs`, `conductor.rs`, `pty.rs`.
- Не менять версию и CHANGELOG, не трогать `.github/`.
- Не добавлять окна/webview — панель живёт во фронте, это задача другого исполнителя.

## Кто что правит
- Ты: `src-tauri/src/preview.rs` (новый), `src-tauri/src/lib.rs` (только регистрация), `src-tauri/Cargo.toml`
  (если нужны `walkdir`/`ignore`), `src-tauri/tauri.conf.json` (assetProtocol + CSP), `src-tauri/capabilities/default.json`.
- Не трогать: всё в `src/`, остальные `.rs`.

## Контекст
- Спека: `docs/SPEC.md` §7 (контракт, ошибки строкой по-русски) и §11.
- Запуск команд в login-shell: `shell::run_zsh(cmd, cwd)` (тайм-аут 30 с, PATH с `~/.local/bin`).
- Проекты: `projects.rs` читает `~/.orchestra/projects.json` (`Project { name, path, group?, color? }`).
- Ворктри лежат в `~/orchestra/<repo>/<name>` (см. `jobs.rs`).
- Паттерн вотчера с `notify` и дебаунсом есть в `jobs::start_watcher`.
- Tauri 2.12, Rust edition 2024. Ошибки команд — `Err(String)` по-русски.

## Приёмка
- [ ] `cd src-tauri && cargo test --lib` → все тесты зелёные, новые тесты для lsof-парсера, фильтра файлов, обрезки диффа, границ `preview_read`
- [ ] `cd src-tauri && cargo check` без предупреждений в новом коде
- [ ] `cd src-tauri && cargo test --lib -- --ignored preview::live` при запущенном `pnpm dev` (порт 1420) показывает источник `url` `http://localhost:1420` для проекта `~/projects/orchestra`
- [ ] `pnpm tauri dev` стартует, в консоли нет ошибок про capabilities/CSP (проверить, что приложение открывается и чат работает)

## Обязательные правила
- Коммиты только от xanymanywka
  (`git -c user.name="xanymanywka" -c user.email="faust.9o8@gmail.com" commit ...`),
  без Co-Authored-By, без упоминаний ИИ в коммитах и README.
- Секреты в файлы и коммиты не класть.
- Никакого безымянного `git stash`.
- Продакшн не деплоить, теги не ставить, `main` не трогать — в конце отчёт дирижёру по пунктам приёмки с выводом команд.
