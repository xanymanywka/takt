# TASK: Сервер телефона-компаньона (SPEC §12, часть Rust)

## Цель
Десктоп Takt поднимает локальный защищённый сервер в домашней сети, телефон может привязаться по QR или
6-значному коду и вызывать те же команды и получать те же события, что и фронт (§7), через WebSocket.
PWA-клиент делает другой исполнитель позже; здесь — сервер, протокол, привязка, реестр устройств и IPC для
экрана «Доступы → Телефон».

## Объём
- Новый модуль `src-tauri/src/companion.rs` (можно разбить на `companion/{mod,server,pairing,devices}.rs`):
  - HTTP(S)+WS сервер на `axum` + `tokio-tungstenite`/`axum::extract::ws`, порт случайный свободный (0 → реальный),
    слушать `0.0.0.0`. TLS — самоподписанный сертификат `rcgen`, сохранённый в `~/.orchestra/companion/cert.pem`
    + `key.pem` (0600), отпечаток SHA-256 отдаётся фронту для QR. Сервер — `axum-server` с `rustls`.
  - Статика: `GET /` и `/pwa/*` отдают файлы из папки `pwa_dir` (параметр; по умолчанию
    `<resource_dir>/pwa`, если нет — ответ 404 с текстом «PWA ещё не собран»). Содержимое PWA здесь не делается.
  - mDNS-объявление `_orchestra._tcp.local.` с TXT `fp=<отпечаток>`, `v=<версия>` через `mdns-sd`.
  - **Привязка.** `POST /pair` с JSON `{ code | token, name }`: одноразовый токен из QR
    (`orch://pair?host=<ip>&port=<p>&token=<t>&fp=<sha256>`) ИЛИ 6-значный код, оба живут 5 минут, один
    активный токен/код в момент времени. Успех → `{ deviceId, key }`, где `key` — 32 случайных байта base64url;
    на десктопе хранится **только** его SHA-256 (`~/.orchestra/companion/devices.json`:
    `[{ id, name, keyHash, createdAt, lastSeen }]`, права 0600). Keychain не использовать (без учётной записи
    разработчика keychain-доступ в Tauri не подписан; файл 0600 — осознанное упрощение, задокументировать в комментарии).
  - **Сессия.** WS `/ws` с заголовком `Authorization: Bearer <key>`; неверный ключ → 401. Протокол:
    запрос `{ id, cmd, args }` → ответ `{ id, ok: true, result }` или `{ id, ok: false, error }`;
    события `{ event, payload }`. Команды — ровно имена §7 (`list_projects`, `list_jobs`, `job_screen`,
    `job_send`, `job_kill`, `limits`, `setup_status`, `conductor_start/send/stop`, `list_models`), вызываются
    через те же публичные функции модулей (`jobs::list_jobs()` и т. д.). **Запрещены по WS**: `pty_*`,
    `open_in_terminal`, `write_instructions`, `save_projects`, `job_new`, `job_run`, `write_task`, `save_model` —
    ответ `{ ok:false, error: "Команда недоступна с телефона" }`. Терминал на телефоне только чтение (`job_screen`).
  - **События** на телефон: `jobs://changed`, `limits://changed`, `conductor://event`, `conductor://exit`,
    `preview://changed` — подписка через `app.listen_any` и рассылка всем аутентифицированным WS.
  - Rate-limit на `/pair`: 5 неудачных попыток → 10 минут блок по IP.
- Tauri-команды (фронт-экран сделает другой исполнитель):
  - `companion_status() -> CompanionStatus { running: bool, host: Option<String> /* локальный IPv4 */, port, fingerprint, devices: Vec<Device{id,name,createdAt,lastSeen}> }`
  - `companion_start() -> CompanionStatus`, `companion_stop()`
  - `companion_pair_begin() -> { url: String /* orch://pair?... */, code: String /* 6 цифр */, expiresAt }`
  - `companion_revoke(device_id: String)` — удаляет устройство и рвёт его WS.
  - Автостарт: если `~/.orchestra/companion/enabled` существует — стартовать в `setup`; `companion_start`
    создаёт этот файл, `companion_stop` удаляет.
- Регистрация в `lib.rs` (`mod companion;`, `.manage(CompanionState::default())`, команды, автостарт).
- Юнит-тесты: генерация/проверка кода и токена с TTL, хеширование ключа и проверка Bearer, фильтр
  запрещённых команд, сериализация протокола, rate-limit. Интеграционный тест `#[ignore]`: поднять сервер
  на 127.0.0.1, пройти `/pair` и вызвать `limits` по WS.

## Не делать
- Не трогать фронт (`src/`), `jobs.rs`, `setup.rs`, `shell.rs`, `conductor.rs`, `pty.rs`, `preview.rs` (его
  пишет соседний исполнитель; если `preview://changed` ещё нет — подписка просто ничего не получит).
- Не делать PWA, QR-рендер, экран настроек — только ядро и IPC.
- Не менять версию, CHANGELOG, `.github/`.
- Не выставлять доступ вне LAN (никаких реле/облаков, Tailscale — только упоминание в комментарии).

## Кто что правит
- Ты: `src-tauri/src/companion.rs` (или папка `companion/`), `src-tauri/src/lib.rs` (только регистрация),
  `src-tauri/Cargo.toml` (axum, axum-server+rustls, rcgen, mdns-sd, sha2, rand, base64, futures),
  `src-tauri/capabilities/default.json` — только если реально нужно.
- Не трогать: `src/`, остальные `.rs`, `tauri.conf.json` (CSP правит соседний исполнитель; если нужна правка —
  напиши в отчёте, не правь).

## Контекст
- Спека: `docs/SPEC.md` §7 (контракт) и §12.
- Публичные функции для вызова по WS уже есть: `jobs::{list_jobs, job_screen, job_send, job_kill}`,
  `limits::limits`, `setup::setup_status`, `projects::list_projects`, `models::list_models`,
  `conductor::{conductor_start, conductor_send, conductor_stop}` — у части сигнатура с `AppHandle`/`State`;
  вызывай через `app.state::<T>()` и клон `AppHandle`.
- Ошибки — `Err(String)` по-русски. Tauri 2.12, edition 2024, tokio уже в зависимостях.
- Крипто и TLS — делать аккуратно: ключи/сертификаты только в `~/.orchestra/companion/` с правами 0600,
  в репозиторий ничего не класть; в логи ключи не печатать.

## Приёмка
- [ ] `cd src-tauri && cargo test --lib` → зелёный, новые тесты: TTL кода/токена, Bearer по хешу, запрещённые команды, rate-limit, протокол
- [ ] `cd src-tauri && cargo test --lib -- --ignored companion::live` → сервер на 127.0.0.1, `/pair` кодом, затем `limits` по WS возвращает `ok:true`
- [ ] `cd src-tauri && cargo check` без предупреждений в новом коде
- [ ] `ls -l ~/.orchestra/companion/` после live-теста → `cert.pem`, `key.pem`, `devices.json` с правами `-rw-------`
- [ ] `pnpm tauri dev` стартует, в консоли нет ошибок

## Обязательные правила
- Коммиты только от xanymanywka
  (`git -c user.name="xanymanywka" -c user.email="faust.9o8@gmail.com" commit ...`),
  без Co-Authored-By, без упоминаний ИИ в коммитах и README.
- Секреты в файлы и коммиты не класть.
- Никакого безымянного `git stash`.
- Продакшн не деплоить, теги не ставить, `main` не трогать — в конце отчёт дирижёру по пунктам приёмки с выводом команд.
