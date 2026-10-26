# Takt

## Backlog
- [ ] Телефон-компаньон, вторая волна: PWA-клиент (адаптивная сборка React в `pwa/`), экран «Доступы → Телефон»
  с QR и списком устройств на `companion_*` IPC (SPEC §12)
- [ ] Импорт чатов и проектов из Claude / Codex / Copilot / Cursor / Gemini при первом старте
- [ ] Несколько аккаунтов GitHub в «Доступах»
- [ ] Исполнители на локальных моделях Ollama (список уже в экране «Модели», запуск через них — отдельная задача)
- [ ] Пак рекомендуемых навыков при онбординге
- [ ] Мастер миграции для других пользователей (свой INSTRUCTIONS.md, свои проекты)
- [ ] Режим прав исполнителя в карточке задачи: план / подтверждение / полный доступ вместо жёсткого
  `--dangerously-skip-permissions` в `agents.py` (замечание ревью 26.10.2026)
- [ ] Структурированное состояние исполнителя: `claude --output-format stream-json`, `codex exec --json` вместо
  разбора экрана tmux (замечание ревью 26.10.2026)
- [ ] `agents.py` в репозитории (или перенос логики в ядро Rust), чтобы Takt ставился без ручной подготовки
  `~/.orchestra` (замечание ревью 26.10.2026)
- [ ] Перевод ошибок ядра (сейчас только по-русски) под язык интерфейса

## In progress

## Review

## Done
- [x] v0.3.1 · история чата по проектам: `conductor/history.rs`, файлы `~/.orchestra/chats/<проект>.json`,
  импорт транскрипта Claude Code при первом открытии (сессия Takt, иначе самый свежий разговор папки),
  события дирижёра привязаны к проекту; IPC `chat_history`/`chat_record`, мок на localStorage (26.10.2026)
- [x] v0.3 · выбор дирижёра Claude Code / Codex CLI / Antigravity: `conductor.json`, адаптеры CLI (Codex JSONL → stream-json),
  шаг «Дирижёр» в мастере, радиогруппа в настройках, аватар дирижёра в чате, тексты без привязки к Claude Code;
  мёрж `conductor-choice` → main `3f99cd0` (26.10.2026)
- [x] v0.3 · платформенный слой `platform.rs`: PTY-раннер исполнителей без tmux (`portable-pty`, буфер 2000 строк,
  `runner: "pty"` в `jobs.json`), `open_in_terminal`/оболочка по ОС, матрица `release.yml` macOS/Windows/Linux
  (msi+nsis, deb+appimage) и `ci.yml` на трёх ОС; мёрж `platform` → main `a82f411` (26.10.2026)
- [x] v0.3 · сервер компаньона `companion.rs`: axum+rustls на случайном порту, самоподписанный сертификат, mDNS
  `_orchestra._tcp`, привязка токен/код (TTL 5 мин, рейт-лимит), WS с Bearer, реестр устройств, `companion_*` IPC,
  автостарт; мёрж `companion-core` → main `6bd237e` (26.10.2026)
- [x] v0.3 · ядро предпросмотра `preview.rs`: источники url (lsof) / image / doc / diff, вотчер с дебаунсом, asset
  protocol и CSP `frame-src localhost`; мёрж `preview-core` → main `20ba786` (26.10.2026)
- [x] v0.3 · панель «Результат» (`Preview.tsx`): веб/картинка/документ/дифф, тянущаяся панель, `⌘\`, кнопки на
  карточках исполнителей, i18n; мёрж `preview-ui` → main (26.10.2026)
- [x] v0.2.2: полноэкранная раскладка, экран «Настройки», мастер первого запуска (6 шагов, перезапуск из настроек),
  фон окна DMG под фирстиль (`src-tauri/dmg`, `scripts/dmg-background.mjs`), быстрые правки по ревью: белый список
  команд `open_in_terminal`, проверка `~/.orchestra/agents.py` в мастере, заголовок приёмки на трёх языках, CSP,
  `ci.yml` (tsc + i18n + cargo test), `CHANGELOG.md`, честный README про мастер; убраны фальшивые «светофоры»
  в сайдбаре (26.10.2026)
- [x] Установка без учётки разработчика и без диалога Gatekeeper: `scripts/install.sh` (curl → `/Applications`,
  без карантина), команда в README на трёх языках и в описании релиза; проверено на v0.2.1 (23.10.2026)
- [x] Ключ подписи обновлений: пара в `~/.tauri/takt.key`, pubkey в `tauri.conf.json`, приватный ключ в секрете
  `TAURI_SIGNING_PRIVATE_KEY`; локальная сборка подписывается, первый релиз `v0.2.1` через `release.yml`
  (см. `docs/HANDOFF-updater.md`, 23.10.2026)
- [x] Логотипы нейронок на аватарках: `src/components/BrandLogos.tsx` (Claude, Codex, Antigravity) в чате,
  спутниках, партитуре и лимитах сайдбара вместо букв/квадратов (23.10.2026)
- [x] Экран «Модели» (SPEC §10): `src-tauri/src/models.rs` собирает списки из `claude --help`, кэша Codex, `agy models`
  и `ollama list`, выбор пишется в `~/.orchestra/models.json`, `job_run` передаёт `--model` в `agents.py`; фронт
  `src/screens/Models.tsx` + общая навигация `SetupNav`, популярные чипами, полный список по клику, i18n uk/en/ru,
  6 юнит-тестов (23.10.2026)
- [x] Замечания CodeRabbit по UI: кнопки вместо div для проектов/ворктри/SSH-строк и «О программе», вкладки с
  `role="tab"` + `aria-selected`, шрифты Montserrat/Newsreader в бандле через `@fontsource-variable` (Google Fonts
  из `index.html` убран), выдуманный хост `nova-welcome-1` убран из мока (23.10.2026)
- [x] Дымовой тест, остаток 23.10.2026: `job_kill` → `agents.py kill` реально убивает tmux-сессию (проверено на
  `takt-smoke`, `jobs.json` не тронут); живые тесты `pty::live_zsh_pty_emits_prompt_and_pwd` и
  `conductor::live_conductor_stream` проходят (`cargo test -- --ignored`)
- [x] i18n uk/en/ru (`src/i18n`, `t()` с плюралами через `Intl.PluralRules`, переключатель в шапке и группа «Язык»
  в «Доступах»), экран «О программе» (автор, сайт, Telegram, GitHub, лицензия, проверка обновлений), плашка
  «Поддержать» из `src/about/support.json` (пока пустой — реквизиты вносит владелец), `scripts/check-i18n.mjs`
  по `docs/TASK-i18n-about.md` (сделано самому после лимита Codex, 23.10.2026)
- [x] Автообновление `tauri-plugin-updater` + плашка «Доступна версия X», настройки «Обновления», проверка раз в 24 ч
  по `docs/TASK-updater.md` (Codex, доделано после лимита; CodeRabbit: major только про пустой pubkey — шаг владельца;
  мёрж `updater` в main `8f8c133`, 23.10.2026)
- [x] «Реестр задач»: проект и папка берутся из `worktree`, дата старта в локальном формате, «Остановить» только у
  активных задач (`0007bca`, 23.10.2026)
- [x] Ядро Tauri: jobs, limits, conductor, pty, setup по `docs/TASK-core.md` (Codex); `cargo check` зелёный, gate major
  пройден, мёрж `core` в main `d14e334`; дымовой тест на реальных данных: проекты, реестр задач из `jobs.json`,
  11 SSH-хостов в «Доступах», заголовок окна Takt (23.10.2026)
- [x] Переименование в код: productName `Takt`, title Takt, identifier `space.xanyman.takt` (23.10.2026)
- [x] Интерфейс «Партитура» по `docs/TASK-ui.md`: Чат, Код (xterm), Оркестр, Доступы на данных Setup, две темы;
  CodeRabbit gate major пройден, мёрж `ui` в main (23.10.2026)
- [x] Публичное репо github.com/xanymanywka/takt, PolyForm NC, README EN/UK/RU, название Takt латиницей везде (23.10.2026)
- [x] Релизы: `release.yml` → universal `.dmg` + updater json в GitHub Releases (23.10.2026)
- [x] Анимированная схема архитектуры прямо в README на трёх языках (`docs/architecture/build_svg.py`) (23.10.2026)
- [x] Имя Takt и иконка (вариант 5 серии 3: Т из узлов с лаймовым кольцом), набор для Tauri (23.10.2026)
- [x] Схема архитектуры Archify `docs/architecture/takt.html` (23.10.2026)
- [x] Дизайн «Партитура», две темы, контраст под слабое зрение (23.10.2026)
- [x] Скаффолд Tauri 2 + React 19 + TS, спека и IPC-контракт (23.10.2026)
