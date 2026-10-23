# Takt

## Backlog
- [ ] Переименование в код: productName `Takt`, title Takt, identifier `space.xanyman.takt` (в core, Codex)
- [ ] Автообновление `tauri-plugin-updater` + плашка «Доступна версия X»
- [ ] Экран «Модели»: все доступные модели по провайдерам, популярные сверху
- [ ] i18n: украинский и английский (+русский), выбор в настройках
- [ ] «О программе»: автор, сайт, Telegram, описание, лицензия, проверка обновлений
- [ ] Плашка «Поддержать автора»: карта / IBAN / PayPal / Binance из `support.json`
- [ ] Панель «Результат» в чате: встроенный браузер на localhost, картинки, md/pdf, дифф (SPEC §11)
- [ ] Телефон-компаньон: QR/ручная привязка, LAN-сервер axum+WS+mDNS, PWA с тем же UI (SPEC §12)
- [ ] Windows и Linux: матрица сборки, platform.rs, запуск исполнителей без tmux через PTY (SPEC §13)
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
- [ ] Замечания CodeRabbit по UI (minor, после мёржа 23.10.2026): клавиатурный доступ к строкам проектов/ворктри/SSH
  (кнопки вместо div), «—» вместо выдуманных процентов лимитов, шрифты Montserrat/Newsreader в бандл
  (`@fontsource`), переключатель вкладок без `role="tab"` + `aria-pressed`

## Review

## Done
- [x] Интерфейс «Партитура» по `docs/TASK-ui.md`: Чат, Код (xterm), Оркестр, Доступы на данных Setup, две темы;
  CodeRabbit gate major пройден, мёрж `ui` в main (23.10.2026)
- [x] Публичное репо github.com/xanymanywka/takt, PolyForm NC, README EN/UK/RU, название Takt латиницей везде (23.10.2026)
- [x] Релизы: `release.yml` → universal `.dmg` + updater json в GitHub Releases (23.10.2026)
- [x] Анимированная схема архитектуры прямо в README на трёх языках (`docs/architecture/build_svg.py`) (23.10.2026)
- [x] Имя Takt и иконка (вариант 5 серии 3: Т из узлов с лаймовым кольцом), набор для Tauri (23.10.2026)
- [x] Схема архитектуры Archify `docs/architecture/takt.html` (23.10.2026)
- [x] Дизайн «Партитура», две темы, контраст под слабое зрение (23.10.2026)
- [x] Скаффолд Tauri 2 + React 19 + TS, спека и IPC-контракт (23.10.2026)
