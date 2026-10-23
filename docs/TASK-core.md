# TASK: Ядро Tauri — jobs, limits, conductor, pty, setup

## Цель
Rust-сторона приложения «Оркестр» реализует IPC-контракт из `docs/SPEC.md` §7 целиком,
поверх существующей оркестрации `~/.orchestra/agents.py` (tmux). После задачи `pnpm tauri dev`
поднимает окно, и фронт (пишется параллельно другим исполнителем) получает реальные данные.

## Объём
- `src-tauri/Cargo.toml`: добавить `tokio` (full), `portable-pty`, `notify`, `dirs`, `regex`,
  `chrono`, `thiserror`; `tauri` с нужными фичами. Авторы — `xanymanywka`.
- `src-tauri/src/shell.rs`: помощник `run_zsh(cmd, cwd) -> Result<String,String>`:
  `zsh -lc`, PATH = `~/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin` впереди,
  без `ANTHROPIC_*` переменных в окружении дочернего процесса (явно удалить
  `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_SMALL_FAST_MODEL`, если вдруг есть).
- `src-tauri/src/projects.rs`: `list_projects`, `save_projects` (`~/.orchestra/projects.json`);
  при отсутствии файла — предложить `~/projects/*` с `.git` (не сохранять без вызова `save_projects`).
- `src-tauri/src/jobs.rs`: чтение `~/.orchestra/jobs.json`, парсинг шагов из `## Приёмка`
  TASK-файла (`- [ ]`/`- [x]`), последняя непустая строка экрана через
  `python3 ~/.orchestra/agents.py tail <name> 3`, вычисление `state` по таблице SPEC §4,
  модель исполнителя (codex → `gpt-5-codex`, gemini → `Gemini 2.5`, claude → `Claude`; если
  в экране видна строка с моделью — брать её). Команды `list_jobs`, `job_screen`, `job_send`,
  `job_kill`, `job_new`, `job_run`, `write_task`. Слежение за `jobs.json` (`notify`) +
  опрос экранов раз в 2 с для `running/asking` → событие `jobs://changed`.
- `src-tauri/src/limits.rs`: `codexbar usage --provider claude|codex|antigravity`. Сначала
  проверить `codexbar usage --help` на JSON-вывод и использовать его, если есть; иначе парсить
  строки `Session: 65% left`, `Weekly: 94% left`, `Resets in 2h 34m`,
  `Gemini weekly: 98% left`. Раз в 60 с → `limits://changed`. Команда `limits`.
- `src-tauri/src/conductor.rs`: процесс `claude` по SPEC §3 на проект, stdin/stdout по строкам,
  каждая строка → `conductor://event`; `session_id` из события `system/init` сохранять в
  `~/.orchestra/sessions.json` и использовать для `--resume` при следующем старте;
  `--append-system-prompt-file ~/.orchestra/INSTRUCTIONS.md`, если файл есть. Команды
  `conductor_start/send/stop`, событие `conductor://exit`. Если `claude` печатает ошибку
  входа — вернуть `Err("Нужен вход: выполните claude в терминале")`.
- `src-tauri/src/pty.rs`: `portable-pty`, `zsh -l` в cwd, события `pty://data`, `pty://exit`,
  команды `pty_open/write/resize/close`. Данные во фронт — строкой (UTF-8, lossy).
- `src-tauri/src/setup.rs`: `setup_status` (инструменты через `command -v` и `--version`,
  вход: `claude auth status` если команда существует иначе наличие `~/.claude/.credentials.json`;
  `codex login status`; `gh auth status` (аккаунт из вывода); SSH-хосты из `~/.ssh/config`
  только `Host/HostName/User/Port/IdentityFile`, без ключей; MCP из `~/.codex/config.toml`
  (секция `[mcp_servers.*]`) и `~/.gemini/config/mcp_config.json`), `write_instructions`
  (шаблон `src-tauri/templates/INSTRUCTIONS.md` → `~/.orchestra/INSTRUCTIONS.md`, не
  перезаписывать существующий без `force`), `open_in_terminal` (osascript → Terminal.app).
- `src-tauri/templates/INSTRUCTIONS.md`: правила дирижёра, сжатые из `docs/SPEC.md` §2–§5:
  роли (Codex — крипто/авторизация/платежи/миграции/сложная логика; Antigravity — фронт/i18n/
  доки/тесты/чтение большого кода; Claude — мелкое и спорное), команды `agents new/run/ls/tail/
  send/kill`, шаблон `docs/TASK-<slug>.md` (Цель/Объём/Не делать/Кто что правит/Контекст/
  Приёмка), «одна ворктри — один исполнитель», проверка лимитов `codexbar usage` перед
  раздачей, CodeRabbit `--gate major` перед мёржем, коммиты только от владельца репозитория,
  без Co-Authored-By. **Никаких упоминаний личного `~/.claude/CLAUDE.md`** — приложение его не трогает.
- `src-tauri/capabilities/default.json` и `tauri.conf.json`: окно 1280×820, min 980×640,
  `title: "Оркестр"`, identifier `space.xanyman.orchestra`, разрешения событий.
- `src-tauri/src/lib.rs`: регистрация всех команд и фоновых задач.

## Не делать
- Не трогать `src/` (фронт пишет другой исполнитель), `docs/design/`, `KANBAN.md`.
- Не переписывать `~/.orchestra/agents.py`, не менять формат `jobs.json`.
- Не выполнять логины и не хранить токены. Не читать и не писать `~/.claude/CLAUDE.md`.
- Не выставлять `ANTHROPIC_BASE_URL` и прочие `ANTHROPIC_*`. Не ставить прокси/роутеры.
- Не деплоить, не публиковать.

## Кто что правит
- Ты: `src-tauri/**`, `package.json` только если нужен скрипт (`"tauri"` уже есть).
- Не трогать: `src/**`, `index.html`, `vite.config.ts`, `docs/**` кроме `docs/HANDOFF-core.md`.

## Контекст
- `docs/SPEC.md` — архитектура и **IPC-контракт §7**: имена команд, аргументы (camelCase со
  стороны фронта — Tauri сам маппит в snake_case), формы ответов и событий должны совпадать
  буквально. Фронт пишется параллельно по тому же контракту.
- `~/.orchestra/agents.py` — рабочий оркестратор, его команды и regex лимита (`LIMIT`) — источник
  правды для состояния `limit`.
- Rust ставится в `~/.cargo/bin` (rustup, профиль minimal). Если `cargo` не найден —
  `source ~/.cargo/env`. Если rustup ещё не доехал — подождать, не ставить Rust иначе.
- Документацию Tauri 2 / portable-pty брать через Context7 MCP.
- Пример реального `jobs.json` лежит у пользователя в `~/.orchestra/jobs.json` — читать можно,
  изменять нельзя.
- Если упрёшься в лимит подписки — запиши состояние в `docs/HANDOFF-core.md` (что сделано,
  что осталось, как проверить) и остановись.

## Приёмка
- [ ] `cd src-tauri && cargo check` → без ошибок и предупреждений `unused`
- [ ] `pnpm install && pnpm tauri dev` поднимает окно «Оркестр» (фронт может быть заглушкой)
- [ ] `list_jobs` на реальном `~/.orchestra/jobs.json` возвращает задачи с `steps` и `lastLine`
      (приложи JSON-вывод через временный `console.log` в devtools или `cargo test`)
- [ ] `limits` возвращает три провайдера с числами (вывод приложи)
- [ ] `conductor_start` + `conductor_send("Скажи одно слово: готов")` → события `assistant`
      и `result` приходят, `session_id` записан в `~/.orchestra/sessions.json`
- [ ] `pty_open` → `pty://data` содержит приглашение zsh; `pty_write("pwd\n")` печатает cwd
- [ ] `setup_status` находит `claude`, `codex`, `agy`, `gh`, `codexbar`, `tmux`, `git`, читает
      SSH-хосты и MCP без секретов (вывод приложи, ключи и токены замазать)
- [ ] `write_instructions` создаёт `~/.orchestra/INSTRUCTIONS.md`; повторный вызов без `force`
      не перезаписывает
- [ ] `coderabbit review --agent --gate major --dir .` → код выхода не 2
- [ ] коммиты от xanymanywka, `git log --format='%an %ae'` без других авторов

## Обязательные правила
- Коммиты только от xanymanywka
  (`git -c user.name="xanymanywka" -c user.email="<email>" commit ...`),
  без Co-Authored-By, без упоминаний ИИ в коммитах и README.
- Секреты в файлы и коммиты не класть.
- Никакого безымянного `git stash`.
- Продакшн не деплоить — в конце отчёт дирижёру по пунктам приёмки с выводом команд.
