# Оркестр

## Backlog
- [ ] Имя и иконка приложения — выбор владельца из вариантов (блокирует репо и identifier)
- [ ] Публичный репозиторий на GitHub `xanymanywka`, лицензия PolyForm Noncommercial, README RU/EN/UK
- [ ] Релизы: `release.yml` → universal `.dmg` в GitHub Releases, подпись обновлений
- [ ] Автообновление `tauri-plugin-updater` + плашка «Доступна версия X»
- [ ] Анимированная схема архитектуры в README (`docs/architecture.svg`)
- [ ] Экран «Модели»: все доступные модели по провайдерам, популярные сверху
- [ ] i18n: украинский и английский (+русский), выбор в настройках
- [ ] «О программе»: автор, сайт, Telegram, описание, лицензия, проверка обновлений
- [ ] Плашка «Поддержать автора»: карта / IBAN / PayPal / Binance из `support.json`
- [ ] Импорт чатов и проектов из Claude / Codex / Copilot / Cursor / Gemini при первом старте
- [ ] Несколько аккаунтов GitHub в «Доступах»
- [ ] Локальные модели (Ollama) по месту на диске
- [ ] Пак рекомендуемых навыков при онбординге
- [ ] Мастер миграции для других пользователей (свой INSTRUCTIONS.md, свои проекты)

## In progress
- [ ] Ядро Tauri: jobs, limits, conductor, pty, setup — `docs/TASK-core.md` (Codex)
  - [ ] `cargo check` зелёный
  - [ ] `list_jobs` читает реальный `~/.orchestra/jobs.json` и считает шаги
  - [ ] `conductor_send` получает ответ `claude` потоком
  - [ ] `pty_open` даёт рабочий `zsh -l`
  - [ ] `limits` показывает три провайдера
- [ ] Интерфейс по дизайну «Партитура» — `docs/TASK-ui.md` (Antigravity)
  - [ ] `pnpm build` зелёный
  - [ ] все экраны на моках в браузере совпадают с `docs/design/orchestra-hand.html`
  - [ ] две темы, переключатель, контраст по DESIGN.md
  - [ ] xterm во вкладке «Код»

## Review

## Done
- [x] Дизайн «Партитура», две темы, контраст под слабое зрение (23.10.2026)
- [x] Скаффолд Tauri 2 + React 19 + TS, спека и IPC-контракт (23.10.2026)
