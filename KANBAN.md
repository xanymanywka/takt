# Takt

## Backlog
- [ ] Панель «Результат» в чате: встроенный браузер на localhost, картинки, md/pdf, дифф (SPEC §11)
- [ ] Телефон-компаньон: QR/ручная привязка, LAN-сервер axum+WS+mDNS, PWA с тем же UI (SPEC §12)
- [ ] Windows и Linux: матрица сборки, platform.rs, запуск исполнителей без tmux через PTY (SPEC §13)
- [ ] Импорт чатов и проектов из Claude / Codex / Copilot / Cursor / Gemini при первом старте
- [ ] Несколько аккаунтов GitHub в «Доступах»
- [ ] Исполнители на локальных моделях Ollama (список уже в экране «Модели», запуск через них — отдельная задача)
- [ ] Пак рекомендуемых навыков при онбординге
- [ ] Мастер миграции для других пользователей (свой INSTRUCTIONS.md, свои проекты)

## In progress
- [ ] Ключ подписи обновлений (только владелец): `pnpm tauri signer generate -w ~/.tauri/takt.key`, pubkey в
  `src-tauri/tauri.conf.json`, секреты `TAURI_SIGNING_PRIVATE_KEY[_PASSWORD]` в GitHub — см. `docs/HANDOFF-updater.md`

## Review

## Done
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
