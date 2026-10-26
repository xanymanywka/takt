# Takt — архитектура v0.1

Десктоп-приложение для дирижирования агентами: пользователь говорит с главным агентом
(Claude Code) как в чате, а под сообщением видит «спутники» — карточки исполнителей
(Codex, Antigravity/agy, Claude), которых дирижёр запустил, с честным прогрессом по шагам.

Заменяет Emdash / Nimbalyst / Conductor / Vibe Kanban (все отклонены 23.10.2026).
Дизайн утверждён: `docs/design/orchestra-hand.html` (открыть в браузере), токены — `docs/DESIGN.md`.

## 1. Стек

- **Tauri 2** (Rust 2024) + **React 19** + **TypeScript** + **Vite**. Менеджер — pnpm.
- CSS: обычные CSS-файлы с токенами из DESIGN.md. Без Tailwind, без UI-китов.
- Состояние: `zustand`. Терминал: `@xterm/xterm` + `@xterm/addon-fit`.
- Rust: `tokio`, `serde`, `portable-pty` (PTY для вкладки «Код»), `notify` (слежение за файлами).
- Никаких прокси и подмены провайдеров: `claude` ходит напрямую в Anthropic.
  Приложение **не выставляет** `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_SMALL_FAST_MODEL`.

## 2. Что уже есть и переиспользуется

Рабочая оркестрация живёт в `~/.orchestra/agents.py` (tmux). v0.1 **не переписывает** её,
а строит интерфейс поверх:

| Что | Где | Как используем |
|---|---|---|
| Реестр задач | `~/.orchestra/jobs.json` (`{name:{agent,worktree,task,startedAt,state,handedTo?,takeoverOf?}}`) | читаем и следим (`notify`) |
| Экран исполнителя | `agents tail <name> [N]` | опрашиваем раз в 2 с для открытых карточек |
| Ввод исполнителю | `agents send <name> <text>` | кнопка «Ответить» в карточке |
| Запуск | `agents new <repo> <name>`, `agents run <codex\|gemini\|claude> <wt> <task-file>` | команды из интерфейса и из дирижёра |
| Остановка | `agents kill <name>` | |
| Лимиты | `codexbar usage --provider claude\|codex\|antigravity` | парсим текст (проверить `--format json`/`--json` — если есть, использовать) |
| Ворктри | `~/orchestra/<repo>/<name>` | путь из jobs.json |

`agents` вызывается как `python3 ~/.orchestra/agents.py …` через login-shell `zsh -lc`,
PATH начинается с `~/.local/bin:/opt/homebrew/bin`.

## 3. Дирижёр

Главный агент — процесс `claude` в режиме потокового JSON:

```
claude --dangerously-skip-permissions -p --verbose \
  --output-format stream-json --input-format stream-json \
  [--resume <session_id>]
```

- cwd = выбранный проект. Процесс живёт, пока открыт чат проекта; новые реплики пишутся
  в stdin строкой `{"type":"user","message":{"role":"user","content":"…"}}`.
- Каждая строка stdout — JSON-событие; Rust пробрасывает его как есть во фронт
  (`conductor://event`) и дополнительно хранит `session_id` в `~/.orchestra/sessions.json`
  (`{ "<projectPath>": "<session_id>" }`) для `--resume`.
- Фронт рендерит: `assistant` → текст (шрифт голоса, Newsreader), `tool_use` → одна тихая
  строка «читает файл…/запускает…», `result` → конец хода.
- Дирижёр узнаёт правила из `~/.orchestra/INSTRUCTIONS.md` (см. §5): приложение передаёт его
  через `--append-system-prompt-file ~/.orchestra/INSTRUCTIONS.md`.

## 4. Честный прогресс («партитура»)

Шаг = пункт чек-листа в `docs/TASK-<slug>.md` исполнителя, раздел `## Приёмка`
(`- [ ]` / `- [x]`). Rust читает файл задачи из jobs.json, считает `done/total`,
отдаёт список шагов. Состояния карточки:

| state | откуда |
|---|---|
| `running` | jobs.json |
| `asking` | последняя строка экрана оканчивается на `?` или содержит `(y/n)`, `Allow`, `Разрешить` |
| `limit` | jobs.json `state=limit` или regex лимита из agents.py |
| `done` | jobs.json `state=done` или `exited`, все шаги `[x]` |
| `killed`/`failed` | jobs.json |

Никаких процентов «на глаз»: только `k из n` и текущая строка экрана.

## 5. Первый запуск и инструкции

При первом старте приложение:

1. Проверяет инструменты: `claude`, `codex`, `agy`, `gh`, `codexbar`, `coderabbit`, `tmux`, `git`
   (`zsh -lc 'command -v …'`) и вход: `claude auth status` (или `claude --version` + файл
   `~/.claude/.credentials.json`), `codex login status`, `gh auth status`, `agy --version`.
2. Создаёт `~/.orchestra/INSTRUCTIONS.md` из шаблона `src-tauri/templates/INSTRUCTIONS.md`
   (правила дирижёра: кому что отдавать, команды `agents`, шаблон TASK, коммиты только от
   владельца, CodeRabbit-гейт). **Не читает и не правит `~/.claude/CLAUDE.md`** — это личный
   файл пользователя.
3. Создаёт `~/.orchestra/projects.json` (`[{name, path, group, color}]`), если его нет,
   предлагая папки из `~/projects/*` с `.git`.
4. Логины приложение **не выполняет**: показывает список и кнопку «Открыть терминал», где
   пользователь сам вводит `claude` / `codex login` / `gh auth login` / `agy`.

## 6. Экраны (см. дизайн)

- **Шапка:** заголовок, переключатель «Чат с дирижёром · Старт · Доступы», кнопка темы.
- **Боковая панель:** проекты (цвет группы, точка активности), внизу «Осталось на сегодня» —
  три полоски лимитов.
- **Чат:** сегменты «Чат · Код · Оркестр»; реплики пользователя (пузырь), реплики дирижёра
  (голосом), под репликой — **партитура** (строка на исполнителя, такты = шаги) и
  **спутники** (карточки: имя, ворктри·модель, состояние, 3 последних строки, «Дальше:», вопрос
  с полем ответа, футер «шаг k из n · Дифф · Терминал»). Внизу стеклянный композер.
- **Код:** xterm в выбранной ворктри (PTY `zsh -l`), слева список ворктри.
- **Оркестр:** все задачи всех проектов таблицей (jobs.json) с состояниями.
- **Старт · Доступы:** инструменты (найден/вход), SSH-хосты из `~/.ssh/config` (только чтение),
  MCP из `~/.codex/config.toml` и `~/.gemini/config/mcp_config.json` (только чтение), CTA
  «Создать INSTRUCTIONS.md» / «Открыть терминал для входа».

## 7. IPC-контракт (единственный источник правды)

Rust реализует команды и события ровно с этими именами и формами; фронт пишет
`src/lib/ipc.ts` по этому контракту и **мок** `src/lib/mock.ts`, который работает в обычном
браузере (`pnpm dev`, когда `window.__TAURI_INTERNALS__` отсутствует).

```ts
// ---- типы ----
export type AgentKind = 'codex' | 'gemini' | 'claude';
export type JobState = 'running' | 'asking' | 'limit' | 'done' | 'killed' | 'failed' | 'exited';

export interface Project { name: string; path: string; group?: string; color?: string }
export interface Step { text: string; done: boolean }
export interface Job {
  name: string; agent: AgentKind; worktree: string; task: string; // task — путь к TASK-файлу
  startedAt: string; state: JobState; handedTo?: string; takeoverOf?: string;
  steps: Step[]; lastLine: string; model?: string;
}
export interface Limit { provider: 'claude' | 'codex' | 'antigravity'; sessionLeft?: number; weeklyLeft?: number; resetsIn?: string; raw: string }
export interface Tool {
  id: 'claude'|'codex'|'agy'|'gh'|'codexbar'|'coderabbit'|'tmux'|'git'|'agents';
  found: boolean; path?: string; version?: string; loggedIn?: boolean; account?: string;
  warn?: boolean; statusText?: string; // agents на Windows: found, warn, «Запуск через PTY ядра»
}
export interface SshHost { host: string; hostname?: string; user?: string; port?: number; identityFile?: string }
export interface McpServer { name: string; source: 'codex' | 'agy'; command?: string; url?: string }
export interface Setup { instructionsExists: boolean; projectsExists: boolean; tools: Tool[]; ssh: SshHost[]; mcp: McpServer[] }

// ---- команды (tauri invoke) ----
list_projects(): Project[]
save_projects(projects: Project[]): void
list_jobs(): Job[]                                  // jobs.json + шаги + последняя строка
job_screen(name: string, lines?: number): string    // agents tail; runner "pty" — буфер ядра
job_send(name: string, text: string): void          // agents send; runner "pty" — запись в PTY + \r
job_kill(name: string): void                        // runner "pty" — kill процесса и удаление из jobs.json
job_new(repo: string, name: string): string         // agents new → путь ворктри
job_run(agent: AgentKind, worktree: string, taskFile: string, name?: string): Job
write_task(worktree: string, slug: string, markdown: string): string // docs/TASK-<slug>.md → путь
limits(): Limit[]
setup_status(): Setup
write_instructions(): string                        // создаёт ~/.orchestra/INSTRUCTIONS.md, возвращает путь
conductor_start(projectPath: string): { sessionId?: string }
conductor_send(projectPath: string, text: string): void
conductor_stop(projectPath: string): void
pty_open(cwd: string, cols: number, rows: number): string  // id
pty_write(id: string, data: string): void
pty_resize(id: string, cols: number, rows: number): void
pty_close(id: string): void
open_in_terminal(cwd: string, command?: string): void // Terminal.app / Windows Terminal / x-terminal-emulator (§13)

// ---- события (tauri listen) ----
'jobs://changed'           payload: Job[]           // при изменении jobs.json или экрана
'limits://changed'         payload: Limit[]         // раз в 60 с
'conductor://event'        payload: { projectPath: string; event: unknown } // строка stream-json
'conductor://exit'         payload: { projectPath: string; code: number }
'pty://data'               payload: { id: string; data: string }
'pty://exit'               payload: { id: string; code: number }
```

Ошибки команд — строкой на русском (`Err(String)`), фронт показывает их в карточке/тосте.

## 8. Структура репозитория

```
src/                    фронт (agy)
  main.tsx, App.tsx
  styles/tokens.css     токены из DESIGN.md (день на :root, ночь в [data-theme=dark] и media)
  styles/app.css
  lib/ipc.ts            обёртки invoke/listen по §7
  lib/mock.ts           мок-данные и фейковые события для браузера
  store.ts              zustand
  screens/Chat.tsx  Code.tsx  Orchestra.tsx  Start.tsx
  components/Lead.tsx Sidebar.tsx Score.tsx Satellite.tsx Composer.tsx Terminal.tsx ThemeToggle.tsx Icons.tsx
src-tauri/              ядро (Codex)
  src/lib.rs            регистрация команд
  src/jobs.rs  limits.rs  conductor.rs  pty.rs  setup.rs  projects.rs  shell.rs (zsh -lc helper)
  templates/INSTRUCTIONS.md
  capabilities/default.json   разрешения shell/fs/event
docs/                   SPEC.md, DESIGN.md, TASK-*.md, design/orchestra-hand.html
KANBAN.md
```

## 9. Публикация, обновления, защита (v0.2)

- **Репозиторий** — публичный, на аккаунте GitHub `xanymanywka`. README:
  описание, скриншоты обеих тем, анимированная схема архитектуры (`docs/architecture.*`),
  инструкция «скачать → установить → настроить за 5 минут».
- **Сборка и релизы** — GitHub Actions `release.yml`: `pnpm tauri build` на `macos-latest`
  (universal: aarch64 + x86_64), артефакты `.dmg` и `.app.tar.gz` + подпись обновлений
  (`tauri signer`), публикация в GitHub Releases по тегу `v*`. Ключ подписи — только в
  секретах репозитория, приватный ключ локально у владельца, в репо не класть.
- **Автообновление** — `tauri-plugin-updater`: при запуске (и раз в сутки) читает
  `https://github.com/<owner>/<repo>/releases/latest/download/latest.json`, при новой версии
  показывает плашку «Доступна версия X — Обновить · Позже», ставит и перезапускает.
  Канал один (stable); отключить проверку можно в настройках.
- **Лицензия** — исходники открыты для чтения, но **не** для копирования и перепродажи:
  PolyForm Noncommercial 1.0.0 (личное и некоммерческое использование бесплатно, коммерческое —
  по договорённости с автором). В `LICENSE` + плашка в README и в «О программе». Код
  Tauri-сборки дополнительно минифицируется (vite) — но это защита от скриптов, не от людей.
- **Макет публичного репо** (не делать до выбора имени): `LICENSE`, `README.md` (RU/EN/UK),
  `CHANGELOG.md`, `.github/workflows/{ci,release}.yml`, `docs/architecture.svg`.

## 10. Модели, язык, «О программе» (v0.2)

- **Модели.** Экран «Доступы → Модели»: для каждого исполнителя (claude, codex, agy, ollama)
  список **всех** доступных моделей (из `claude --help`/документации, `codex` конфиг
  `~/.codex/config.toml`, `agy` настройки, `ollama list`), сверху блок «Популярные» —
  2–3 на провайдера по умолчанию; выбор пишется в `~/.orchestra/models.json` и
  подставляется в команды запуска (`-c model=…`, `--model …`).
- **Язык интерфейса.** `src/i18n/{uk,en,ru}.json`, определение по системной локали, выбор в
  настройках; дата/число через `Intl`. Все строки через `t()`, в коде ни одной сырой.
- **О программе.** Имя, версия, автор: Андрей (xanymanywka), сайт https://xanyman.space,
  Telegram @xanymanywka, GitHub, LinkedIn; абзац описания проекта; лицензия; кнопка
  «Проверить обновления».
- **Поддержать автора.** Отдельная плашка в «О программе» и пункт в боковой панели:
  карта, IBAN, PayPal, Binance — данные приходят из `src/about/support.json` (владелец
  заполняет сам, пока пустые поля скрыты), копирование по клику, QR для крипто-адресов.

## 11. Предпросмотр результата в чате (v0.2)

Под репликой дирижёра или в карточке исполнителя — панель **«Результат»**: встроенный
просмотрщик того, что получилось, без выхода из приложения.
- Источники: ссылки `http(s)://localhost:*` и `file://` из событий (`tool_use`/`result`),
  файлы, которые исполнитель создал или изменил в ворктри (картинки, html, svg, pdf, md).
- Режимы: веб (Tauri `WebviewWindow`/`<webview>` на локальный dev-сервер, адресная строка,
  перезагрузка, размер «телефон/планшет/десктоп»), картинка (zoom, до/после двух версий),
  документ (md → html, pdf), дифф файла.
- Открывается сворачиваемой панелью справа (ширина 40 %, тянется) или отдельным окном;
  запоминает последний источник на проект.
- IPC: `preview_sources(projectPath): PreviewSource[]` (`{kind:'url'|'image'|'doc'|'diff', path|url, title, mtime}`),
  событие `preview://changed` при появлении новых файлов/портов (слежение `notify` + проверка
  открытых портов dev-серверов `lsof -iTCP -sTCP:LISTEN` в ворктри процессов).

## 12. Телефон-компаньон (v0.3)

Управление приложением с телефона в той же сети, без облака.
- Десктоп поднимает локальный сервер (`axum`, WebSocket + статика), порт случайный, адрес
  объявляется по mDNS (`_orchestra._tcp`). Привязка: на экране QR (`orch://pair?host=…&port=…&token=…`)
  либо ручной ввод хоста, порта и 6-значного кода. Токен — одноразовый, после привязки устройство
  получает долгоживущий ключ (хранится в Keychain десктопа, список устройств в «Доступах» с
  кнопкой «Отозвать»). Весь трафик — TLS с самоподписанным сертификатом, отпечаток в QR.
- Телефон — PWA (тот же React, адаптивный макет, та же цветовая система): проекты, чат с
  дирижёром, партитура и спутники, ответы исполнителям, лимиты, предпросмотр результата.
  Терминал на телефоне только чтение. Push внутри сети: вопрос исполнителя → уведомление PWA.
- Протокол: те же команды и события §7, завёрнутые в JSON по WebSocket
  (`{id, cmd, args}` → `{id, ok, result|error}`, события как `{event, payload}`).
- Вне домашней сети — не в объёме v0.3 (можно через Tailscale, документировать).

## 13. Windows и Linux (v0.3)

- Матрица сборки в `release.yml`: `macos-latest` (universal), `windows-latest` (`.msi`, `.exe`
  NSIS), `ubuntu-22.04` (`.deb`, `.AppImage`). Updater читает один `latest.json` со всеми платформами.
- Абстракции платформы в `src-tauri/src/platform.rs`: оболочка (`zsh -lc` / `pwsh -NoProfile -Command`
  с фолбэком `powershell` / `$SHELL -lc`, иначе `bash -lc`), PATH инструментов (`tool_path`),
  `~/.orchestra` (`orchestra_home`), внешний терминал (`open_terminal`: Terminal.app через osascript /
  `wt.exe` с фолбэком `cmd /c start pwsh` / `x-terminal-emulator` → `gnome-terminal` → `konsole` → `xterm`),
  `has_tmux` и `python` (`python3` / `python`). `shell::run_shell` работает поверх этих функций
  и по-прежнему вычищает `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_SMALL_FAST_MODEL`.
- Исполнители без tmux (и всегда на Windows): `job_run` запускает их напрямую через `portable-pty`
  с теми же командами, что в §3 (cwd = ворктри, PATH = `tool_path`, PTY 220×50), и пишет в
  `~/.orchestra/jobs.json` запись в формате `agents.py` (`agent`, `worktree`, `task`, `startedAt`,
  `state`, `model?`) с полем `"runner": "pty"`. Записи без `runner` — tmux, обслуживаются `agents.py`
  как раньше. Экран PTY-задачи — кольцевой буфер 2000 строк в памяти ядра без ANSI
  (`job_screen`), `job_send` пишет текст + `\r` в PTY, `job_kill` убивает процесс и убирает запись.
  Завершение процесса переводит `state` в `exited`. После перезапуска приложения экран PTY-задач
  прошлой сессии недоступен — их можно только снять через `job_kill`.
- `setup_status` на Windows: инструмент `agents` отдаётся `found: true, warn: true,
  statusText: "Запуск через PTY ядра"`; действие `install:tmux` возвращает ошибку
  «На Windows tmux не нужен…», на Linux это `sudo apt install tmux`, на macOS `brew install tmux`.
- Шрифты вшиты в бандл (Montserrat, Newsreader), не Google Fonts, чтобы работало офлайн везде.
- Проверять на реальных машинах/VM до релиза; агенты-исполнители (claude/codex/agy) должны
  быть установлены пользователем на той платформе — экран «Старт» это показывает.

## 14. Качество

- `pnpm build` (tsc + vite) и `cargo check` в `src-tauri` зелёные.
- Фронт проверяется в браузере на моках (`pnpm dev`), ядро — `pnpm tauri dev` с реальным
  `~/.orchestra/jobs.json`.
- Перед «готово»: `coderabbit review --agent --gate major --dir <ворктри>`.
- Коммиты только от xanymanywka, без Co-Authored-By и упоминаний ИИ.
