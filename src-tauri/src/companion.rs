//! Сервер телефона-компаньона (SPEC §12): локальный HTTPS+WebSocket в домашней сети.
//!
//! - Порт случайный, адрес объявляется по mDNS `_orchestra._tcp.local.`.
//! - TLS — самоподписанный сертификат в `~/.orchestra/companion/` (права 0600),
//!   отпечаток SHA-256 уходит в QR, чтобы телефон закрепил сертификат.
//! - Привязка: одноразовый токен из QR или 6-значный код, TTL 5 минут; в ответ телефон
//!   получает долгоживущий ключ, на десктопе хранится только его SHA-256 (`devices.json`).
//! - По WS доступны команды §7 через те же функции модулей; команды, меняющие файлы или
//!   открывающие терминал, с телефона запрещены.
//! - Доступ только внутри LAN: реле и облака не делаются. Вне дома можно подключиться через
//!   Tailscale (устройства в одной tailnet видят порт напрямую), это вне объёма v0.3.
//! - Ключи и сертификаты никогда не печатаются в логи и не попадают в репозиторий.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, UdpSocket};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Path as AxumPath, State as AxumState};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_server::Handle;
use axum_server::tls_rustls::RustlsConfig;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, SecondsFormat, Utc};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use rand::Rng;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, EventId, Listener, Manager, State};
use tokio::sync::broadcast;

const PAIRING_TTL: Duration = Duration::from_secs(5 * 60);
const RATE_LIMIT_FAILURES: u8 = 5;
const RATE_LIMIT_BLOCK: Duration = Duration::from_secs(10 * 60);
const SERVICE_TYPE: &str = "_orchestra._tcp.local.";
const FORBIDDEN_ERROR: &str = "Команда недоступна с телефона";
const FORWARDED_EVENTS: [&str; 5] = [
    "jobs://changed",
    "limits://changed",
    "conductor://event",
    "conductor://exit",
    "preview://changed",
];

#[derive(Clone)]
pub struct CompanionState {
    inner: Arc<Mutex<CompanionData>>,
}

struct CompanionData {
    base_dir: PathBuf,
    runtime: Option<Runtime>,
    pairing: Option<PairingSecret>,
    devices: Vec<StoredDevice>,
    rate_limit: RateLimiter,
    starting: bool,
}

struct Runtime {
    host: String,
    port: u16,
    fingerprint: String,
    handle: Handle<SocketAddr>,
    listener_ids: Vec<EventId>,
    mdns: Option<MdnsRegistration>,
    revoke_tx: broadcast::Sender<String>,
}

struct MdnsRegistration {
    daemon: ServiceDaemon,
    fullname: String,
}

#[derive(Clone)]
struct PairingSecret {
    token: String,
    code: String,
    expires_at: Instant,
    expires_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredDevice {
    id: String,
    name: String,
    key_hash: String,
    created_at: String,
    last_seen: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    id: String,
    name: String,
    created_at: String,
    last_seen: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionStatus {
    running: bool,
    host: Option<String>,
    port: Option<u16>,
    fingerprint: Option<String>,
    devices: Vec<Device>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairBegin {
    url: String,
    code: String,
    expires_at: String,
}

#[derive(Default)]
struct RateLimiter {
    clients: HashMap<IpAddr, RateEntry>,
}

struct RateEntry {
    failures: u8,
    blocked_until: Option<Instant>,
}

#[derive(Clone)]
struct WebState {
    companion: CompanionState,
    app: Option<AppHandle>,
    pwa_dir: PathBuf,
    event_tx: broadcast::Sender<String>,
    revoke_tx: broadcast::Sender<String>,
}

#[derive(Deserialize)]
struct PairRequest {
    code: Option<String>,
    token: Option<String>,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PairResponse {
    device_id: String,
    key: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ProtocolRequest {
    id: Value,
    cmd: String,
    #[serde(default)]
    args: Value,
}

#[derive(Debug, Serialize)]
struct ProtocolResponse {
    id: Value,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl ProtocolResponse {
    fn success(id: Value, result: Value) -> Self {
        Self {
            id,
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    fn failure(id: Value, error: impl Into<String>) -> Self {
        Self {
            id,
            ok: false,
            result: None,
            error: Some(error.into()),
        }
    }
}

impl Default for CompanionState {
    fn default() -> Self {
        let base_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".orchestra/companion");
        Self::at(base_dir)
    }
}

impl CompanionState {
    fn at(base_dir: PathBuf) -> Self {
        Self {
            inner: Arc::new(Mutex::new(CompanionData {
                base_dir,
                runtime: None,
                pairing: None,
                devices: Vec::new(),
                rate_limit: RateLimiter::default(),
                starting: false,
            })),
        }
    }

    fn base_dir(&self) -> Result<PathBuf, String> {
        self.inner
            .lock()
            .map(|data| data.base_dir.clone())
            .map_err(|_| "Не удалось заблокировать состояние телефона".to_owned())
    }
}

pub fn install_crypto_provider() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}

impl From<&StoredDevice> for Device {
    fn from(device: &StoredDevice) -> Self {
        Self {
            id: device.id.clone(),
            name: device.name.clone(),
            created_at: device.created_at.clone(),
            last_seen: device.last_seen.clone(),
        }
    }
}

impl RateLimiter {
    fn is_blocked(&mut self, ip: IpAddr, now: Instant) -> bool {
        self.cleanup(now);
        self.clients
            .get(&ip)
            .and_then(|entry| entry.blocked_until)
            .is_some_and(|until| until > now)
    }

    fn record_failure(&mut self, ip: IpAddr, now: Instant) -> bool {
        self.cleanup(now);
        let entry = self.clients.entry(ip).or_insert(RateEntry {
            failures: 0,
            blocked_until: None,
        });
        entry.failures = entry.failures.saturating_add(1);
        if entry.failures >= RATE_LIMIT_FAILURES {
            entry.blocked_until = Some(now + RATE_LIMIT_BLOCK);
            true
        } else {
            false
        }
    }

    fn record_success(&mut self, ip: IpAddr) {
        self.clients.remove(&ip);
    }

    fn cleanup(&mut self, now: Instant) {
        self.clients.retain(|_, entry| {
            if entry.blocked_until.is_some_and(|until| until <= now) {
                return false;
            }
            true
        });
    }
}

fn now_string() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn random_urlsafe<const N: usize>() -> String {
    let mut bytes = [0_u8; N];
    rand::rng().fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn new_pairing(now: Instant, utc_now: DateTime<Utc>) -> PairingSecret {
    PairingSecret {
        token: random_urlsafe::<32>(),
        code: format!("{:06}", rand::rng().random_range(0..1_000_000_u32)),
        expires_at: now + PAIRING_TTL,
        expires_at_utc: utc_now + chrono::Duration::from_std(PAIRING_TTL).expect("valid TTL"),
    }
}

fn pairing_matches(
    pairing: &PairingSecret,
    code: Option<&str>,
    token: Option<&str>,
    now: Instant,
) -> bool {
    pairing.expires_at > now
        && (token.is_some_and(|token| constant_time_eq(token, &pairing.token))
            || code.is_some_and(|code| constant_time_eq(code, &pairing.code)))
}

fn hash_key(key: &[u8]) -> String {
    Sha256::digest(key)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.bytes()
        .zip(right.bytes())
        .fold(0_u8, |diff, (a, b)| diff | (a ^ b))
        == 0
}

fn bearer_key(headers: &HeaderMap) -> Option<Vec<u8>> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let encoded = value.strip_prefix("Bearer ")?;
    let decoded = URL_SAFE_NO_PAD.decode(encoded).ok()?;
    (decoded.len() == 32).then_some(decoded)
}

fn authenticate(headers: &HeaderMap, devices: &[StoredDevice]) -> Option<String> {
    let key = bearer_key(headers)?;
    let candidate = hash_key(&key);
    devices
        .iter()
        .find(|device| constant_time_eq(&device.key_hash, &candidate))
        .map(|device| device.id.clone())
}

fn devices_path(base_dir: &Path) -> PathBuf {
    base_dir.join("devices.json")
}

#[cfg(unix)]
fn write_private(path: &Path, contents: &[u8]) -> Result<(), String> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Не удалось создать {}: {error}", parent.display()))?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("Не удалось защитить {}: {error}", parent.display()))?;
    }
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|error| format!("Не удалось открыть {}: {error}", temporary.display()))?;
    file.write_all(contents)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("Не удалось записать {}: {error}", temporary.display()))?;
    fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))
        .and_then(|_| fs::rename(&temporary, path))
        .and_then(|_| fs::set_permissions(path, fs::Permissions::from_mode(0o600)))
        .map_err(|error| format!("Не удалось сохранить {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn write_private(path: &Path, contents: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Не удалось создать {}: {error}", parent.display()))?;
    }
    fs::write(path, contents)
        .map_err(|error| format!("Не удалось записать {}: {error}", path.display()))
}

fn load_devices(base_dir: &Path) -> Result<Vec<StoredDevice>, String> {
    let path = devices_path(base_dir);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("Не удалось прочитать {}: {error}", path.display()))?;
    serde_json::from_str(&contents).map_err(|error| format!("Некорректный devices.json: {error}"))
}

fn save_devices(base_dir: &Path, devices: &[StoredDevice]) -> Result<(), String> {
    // Keychain намеренно не используется: в неподписанной Tauri-сборке доступ нестабилен.
    // В файле остаётся только SHA-256 ключа, сам ключ выдаётся телефону один раз.
    let json = serde_json::to_vec_pretty(devices)
        .map_err(|error| format!("Не удалось сериализовать устройства: {error}"))?;
    let mut contents = json;
    contents.push(b'\n');
    write_private(&devices_path(base_dir), &contents)
}

fn certificate_fingerprint(cert_pem: &[u8]) -> Result<String, String> {
    let mut reader = BufReader::new(cert_pem);
    let certificate = rustls_pemfile::certs(&mut reader)
        .next()
        .ok_or_else(|| "В cert.pem нет сертификата".to_owned())?
        .map_err(|error| format!("Не удалось прочитать cert.pem: {error}"))?;
    Ok(Sha256::digest(certificate.as_ref())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn local_ipv4() -> Ipv4Addr {
    UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .and_then(|socket| {
            socket.connect((Ipv4Addr::new(192, 0, 2, 1), 80))?;
            socket.local_addr()
        })
        .ok()
        .and_then(|address| match address.ip() {
            IpAddr::V4(ip) if !ip.is_unspecified() => Some(ip),
            _ => None,
        })
        .unwrap_or(Ipv4Addr::LOCALHOST)
}

fn ensure_certificate(
    base_dir: &Path,
    host: Ipv4Addr,
) -> Result<(PathBuf, PathBuf, String), String> {
    let cert_path = base_dir.join("cert.pem");
    let key_path = base_dir.join("key.pem");
    if !cert_path.is_file() || !key_path.is_file() {
        let names = vec!["localhost".to_owned(), host.to_string()];
        let rcgen::CertifiedKey { cert, key_pair } = rcgen::generate_simple_self_signed(names)
            .map_err(|error| format!("Не удалось создать TLS-сертификат: {error}"))?;
        write_private(&cert_path, cert.pem().as_bytes())?;
        write_private(&key_path, key_pair.serialize_pem().as_bytes())?;
    } else {
        // Сертификат публичен, но одинаковый режим для обоих файлов исключает случайное
        // ослабление прав на приватный ключ при обновлениях или ручном копировании.
        let cert = fs::read(&cert_path)
            .map_err(|error| format!("Не удалось прочитать {}: {error}", cert_path.display()))?;
        let key = fs::read(&key_path)
            .map_err(|error| format!("Не удалось прочитать {}: {error}", key_path.display()))?;
        write_private(&cert_path, &cert)?;
        write_private(&key_path, &key)?;
    }
    let cert_pem = fs::read(&cert_path)
        .map_err(|error| format!("Не удалось прочитать {}: {error}", cert_path.display()))?;
    let fingerprint = certificate_fingerprint(&cert_pem)?;
    Ok((cert_path, key_path, fingerprint))
}

fn status_from(data: &CompanionData) -> CompanionStatus {
    let (running, host, port, fingerprint) = match &data.runtime {
        Some(runtime) => (
            true,
            Some(runtime.host.clone()),
            Some(runtime.port),
            Some(runtime.fingerprint.clone()),
        ),
        None => (false, None, None, None),
    };
    CompanionStatus {
        running,
        host,
        port,
        fingerprint,
        devices: data.devices.iter().map(Device::from).collect(),
    }
}

#[tauri::command]
pub fn companion_status(state: State<'_, CompanionState>) -> Result<CompanionStatus, String> {
    let mut data = state
        .inner
        .lock()
        .map_err(|_| "Не удалось заблокировать состояние телефона".to_owned())?;
    if data.runtime.is_none() {
        data.devices = load_devices(&data.base_dir)?;
    }
    Ok(status_from(&data))
}

#[tauri::command]
pub async fn companion_start(
    app: AppHandle,
    state: State<'_, CompanionState>,
) -> Result<CompanionStatus, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|error| format!("Не удалось определить папку ресурсов: {error}"))?;
    start_server(
        state.inner().clone(),
        Some(app),
        Ipv4Addr::UNSPECIFIED,
        resource_dir.join("pwa"),
        true,
        true,
    )
    .await
}

#[tauri::command]
pub fn companion_stop(app: AppHandle, state: State<'_, CompanionState>) -> Result<(), String> {
    stop_server(&state, Some(&app))?;
    let enabled = state.base_dir()?.join("enabled");
    match fs::remove_file(&enabled) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Не удалось удалить {}: {error}", enabled.display())),
    }
}

#[tauri::command]
pub fn companion_pair_begin(state: State<'_, CompanionState>) -> Result<PairBegin, String> {
    begin_pairing(&state)
}

#[tauri::command]
pub fn companion_revoke(state: State<'_, CompanionState>, device_id: String) -> Result<(), String> {
    revoke_device(&state, &device_id)
}

fn revoke_device(state: &CompanionState, device_id: &str) -> Result<(), String> {
    let mut data = state
        .inner
        .lock()
        .map_err(|_| "Не удалось заблокировать состояние телефона".to_owned())?;
    if data.runtime.is_none() {
        data.devices = load_devices(&data.base_dir)?;
    }
    let previous_len = data.devices.len();
    data.devices.retain(|device| device.id != device_id);
    if data.devices.len() == previous_len {
        return Err("Устройство не найдено".to_owned());
    }
    save_devices(&data.base_dir, &data.devices)?;
    if let Some(runtime) = &data.runtime {
        let _ = runtime.revoke_tx.send(device_id.to_owned());
    }
    Ok(())
}

pub fn auto_start(app: AppHandle) {
    let state = app.state::<CompanionState>().inner().clone();
    let Ok(base_dir) = state.base_dir() else {
        return;
    };
    if !base_dir.join("enabled").is_file() {
        return;
    }
    let pwa_dir = match app.path().resource_dir() {
        Ok(path) => path.join("pwa"),
        Err(error) => {
            eprintln!("companion: не удалось определить папку ресурсов: {error}");
            return;
        }
    };
    tauri::async_runtime::spawn(async move {
        if let Err(error) = start_server(
            state,
            Some(app),
            Ipv4Addr::UNSPECIFIED,
            pwa_dir,
            true,
            false,
        )
        .await
        {
            eprintln!("companion autostart: {error}");
        }
    });
}

fn begin_pairing(state: &CompanionState) -> Result<PairBegin, String> {
    let mut data = state
        .inner
        .lock()
        .map_err(|_| "Не удалось заблокировать состояние телефона".to_owned())?;
    let runtime = data
        .runtime
        .as_ref()
        .ok_or_else(|| "Сервер телефона не запущен".to_owned())?;
    let host = runtime.host.clone();
    let port = runtime.port;
    let fingerprint = runtime.fingerprint.clone();
    let pairing = new_pairing(Instant::now(), Utc::now());
    let result = PairBegin {
        url: format!(
            "orch://pair?host={host}&port={port}&token={}&fp={fingerprint}",
            pairing.token
        ),
        code: pairing.code.clone(),
        expires_at: pairing
            .expires_at_utc
            .to_rfc3339_opts(SecondsFormat::Millis, true),
    };
    data.pairing = Some(pairing);
    Ok(result)
}

async fn start_server(
    state: CompanionState,
    app: Option<AppHandle>,
    bind_ip: Ipv4Addr,
    pwa_dir: PathBuf,
    advertise: bool,
    persist_enabled: bool,
) -> Result<CompanionStatus, String> {
    {
        let mut data = state
            .inner
            .lock()
            .map_err(|_| "Не удалось заблокировать состояние телефона".to_owned())?;
        if data.runtime.is_some() {
            return Ok(status_from(&data));
        }
        if data.starting {
            return Err("Сервер телефона уже запускается".to_owned());
        }
        data.starting = true;
    }

    let result = start_server_inner(
        state.clone(),
        app,
        bind_ip,
        pwa_dir,
        advertise,
        persist_enabled,
    )
    .await;
    if result.is_err()
        && let Ok(mut data) = state.inner.lock()
    {
        data.starting = false;
    }
    result
}

async fn start_server_inner(
    state: CompanionState,
    app: Option<AppHandle>,
    bind_ip: Ipv4Addr,
    pwa_dir: PathBuf,
    advertise: bool,
    persist_enabled: bool,
) -> Result<CompanionStatus, String> {
    install_crypto_provider();
    let base_dir = state.base_dir()?;
    let public_host = if bind_ip.is_unspecified() {
        local_ipv4()
    } else {
        bind_ip
    };
    let (cert_path, key_path, fingerprint) = ensure_certificate(&base_dir, public_host)?;
    let devices = load_devices(&base_dir)?;
    save_devices(&base_dir, &devices)?;
    let listener = TcpListener::bind((bind_ip, 0))
        .map_err(|error| format!("Не удалось открыть порт телефона: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("Не удалось определить порт телефона: {error}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("Не удалось настроить порт телефона: {error}"))?;
    let tls = RustlsConfig::from_pem_file(&cert_path, &key_path)
        .await
        .map_err(|error| format!("Не удалось загрузить TLS-сертификат: {error}"))?;
    let handle = Handle::new();
    let (event_tx, _) = broadcast::channel(128);
    let (revoke_tx, _) = broadcast::channel(32);
    let listener_ids = app
        .as_ref()
        .map(|app| register_event_forwarders(app, &event_tx))
        .unwrap_or_default();
    let web_state = WebState {
        companion: state.clone(),
        app: app.clone(),
        pwa_dir,
        event_tx,
        revoke_tx: revoke_tx.clone(),
    };
    let router = Router::new()
        .route("/", get(pwa_root))
        .route("/pwa", get(pwa_root))
        .route("/pwa/", get(pwa_root))
        .route("/pwa/{*path}", get(pwa_file))
        .route("/pair", post(pair))
        .route("/ws", get(websocket))
        .with_state(web_state);

    let mdns = if advertise {
        match register_mdns(public_host, address.port(), &fingerprint) {
            Ok(registration) => Some(registration),
            Err(error) => {
                for id in listener_ids {
                    if let Some(app) = &app {
                        app.unlisten(id);
                    }
                }
                return Err(error);
            }
        }
    } else {
        None
    };

    if persist_enabled && let Err(error) = write_private(&base_dir.join("enabled"), b"") {
        if let Some(app) = &app {
            for id in listener_ids.iter().copied() {
                app.unlisten(id);
            }
        }
        if let Some(mdns) = &mdns {
            let _ = mdns.daemon.unregister(&mdns.fullname);
            let _ = mdns.daemon.shutdown();
        }
        return Err(error);
    }

    let server = axum_server::from_tcp_rustls(listener, tls)
        .map_err(|error| format!("Не удалось запустить TLS-сервер: {error}"))?
        .handle(handle.clone())
        .serve(router.into_make_service_with_connect_info::<SocketAddr>());

    {
        let mut data = state
            .inner
            .lock()
            .map_err(|_| "Не удалось заблокировать состояние телефона".to_owned())?;
        data.devices = devices;
        data.runtime = Some(Runtime {
            host: public_host.to_string(),
            port: address.port(),
            fingerprint,
            handle,
            listener_ids,
            mdns,
            revoke_tx,
        });
        data.starting = false;
    }

    tauri::async_runtime::spawn(async move {
        if let Err(error) = server.await {
            eprintln!("companion server: {error}");
        }
    });

    let data = state
        .inner
        .lock()
        .map_err(|_| "Не удалось заблокировать состояние телефона".to_owned())?;
    Ok(status_from(&data))
}

fn stop_server(state: &CompanionState, app: Option<&AppHandle>) -> Result<(), String> {
    let runtime = {
        let mut data = state
            .inner
            .lock()
            .map_err(|_| "Не удалось заблокировать состояние телефона".to_owned())?;
        data.pairing = None;
        data.runtime.take()
    };
    let Some(runtime) = runtime else {
        return Ok(());
    };
    runtime
        .handle
        .graceful_shutdown(Some(Duration::from_secs(2)));
    if let Some(app) = app {
        for id in runtime.listener_ids {
            app.unlisten(id);
        }
    }
    if let Some(mdns) = runtime.mdns {
        let _ = mdns.daemon.unregister(&mdns.fullname);
        let _ = mdns.daemon.shutdown();
    }
    Ok(())
}

fn register_event_forwarders(
    app: &AppHandle,
    event_tx: &broadcast::Sender<String>,
) -> Vec<EventId> {
    FORWARDED_EVENTS
        .iter()
        .map(|event_name| {
            let tx = event_tx.clone();
            let event_name = (*event_name).to_owned();
            app.listen_any(event_name.clone(), move |event| {
                let payload = serde_json::from_str(event.payload())
                    .unwrap_or_else(|_| Value::String(event.payload().to_owned()));
                if let Ok(message) = serde_json::to_string(&json!({
                    "event": event_name,
                    "payload": payload,
                })) {
                    let _ = tx.send(message);
                }
            })
        })
        .collect()
}

fn register_mdns(host: Ipv4Addr, port: u16, fingerprint: &str) -> Result<MdnsRegistration, String> {
    let daemon =
        ServiceDaemon::new().map_err(|error| format!("Не удалось запустить mDNS: {error}"))?;
    let hostname = format!("takt-{}.local.", std::process::id());
    let properties = [("fp", fingerprint), ("v", env!("CARGO_PKG_VERSION"))];
    let service = ServiceInfo::new(
        SERVICE_TYPE,
        "Takt",
        &hostname,
        IpAddr::V4(host),
        port,
        properties.as_slice(),
    )
    .map_err(|error| format!("Не удалось подготовить mDNS: {error}"))?;
    let fullname = service.get_fullname().to_owned();
    daemon
        .register(service)
        .map_err(|error| format!("Не удалось объявить mDNS: {error}"))?;
    Ok(MdnsRegistration { daemon, fullname })
}

async fn pair(
    AxumState(state): AxumState<WebState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(request): Json<PairRequest>,
) -> Response {
    match pair_device(&state.companion, peer.ip(), request, Instant::now()) {
        Ok(response) => Json(response).into_response(),
        Err((status, error)) => (status, Json(json!({ "error": error }))).into_response(),
    }
}

fn pair_device(
    state: &CompanionState,
    ip: IpAddr,
    request: PairRequest,
    now: Instant,
) -> Result<PairResponse, (StatusCode, String)> {
    let mut data = state.inner.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Не удалось заблокировать состояние телефона".to_owned(),
        )
    })?;
    if data.rate_limit.is_blocked(ip, now) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Слишком много попыток. Повторите через 10 минут".to_owned(),
        ));
    }
    let valid = data.pairing.as_ref().is_some_and(|pairing| {
        pairing_matches(
            pairing,
            request.code.as_deref(),
            request.token.as_deref(),
            now,
        )
    });
    if !valid || request.name.trim().is_empty() {
        let blocked = data.rate_limit.record_failure(ip, now);
        let status = if blocked {
            StatusCode::TOO_MANY_REQUESTS
        } else {
            StatusCode::UNAUTHORIZED
        };
        return Err((status, "Неверный или просроченный код привязки".to_owned()));
    }
    data.pairing = None;
    data.rate_limit.record_success(ip);
    let mut key_bytes = [0_u8; 32];
    rand::rng().fill(&mut key_bytes);
    let key = URL_SAFE_NO_PAD.encode(key_bytes);
    let timestamp = now_string();
    let device = StoredDevice {
        id: random_urlsafe::<16>(),
        name: request.name.trim().chars().take(100).collect(),
        key_hash: hash_key(&key_bytes),
        created_at: timestamp.clone(),
        last_seen: timestamp,
    };
    let response = PairResponse {
        device_id: device.id.clone(),
        key,
    };
    data.devices.push(device);
    save_devices(&data.base_dir, &data.devices)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;
    Ok(response)
}

async fn websocket(
    AxumState(state): AxumState<WebState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let device_id = {
        let Ok(mut data) = state.companion.inner.lock() else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };
        let Some(device_id) = authenticate(&headers, &data.devices) else {
            return StatusCode::UNAUTHORIZED.into_response();
        };
        let timestamp = now_string();
        if let Some(device) = data
            .devices
            .iter_mut()
            .find(|device| device.id == device_id)
        {
            device.last_seen = timestamp;
        }
        if save_devices(&data.base_dir, &data.devices).is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        device_id
    };
    ws.on_upgrade(move |socket| websocket_session(socket, state, device_id))
}

async fn websocket_session(mut socket: WebSocket, state: WebState, device_id: String) {
    let mut events = state.event_tx.subscribe();
    let mut revocations = state.revoke_tx.subscribe();
    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let Some(Ok(message)) = incoming else { break };
                match message {
                    Message::Text(text) => {
                        let response = match serde_json::from_str::<ProtocolRequest>(text.as_str()) {
                            Ok(request) => dispatch_request(&state, request).await,
                            Err(error) => ProtocolResponse::failure(
                                Value::Null,
                                format!("Некорректный запрос: {error}"),
                            ),
                        };
                        let Ok(serialized) = serde_json::to_string(&response) else { break };
                        if socket.send(Message::Text(serialized.into())).await.is_err() {
                            break;
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            event = events.recv() => {
                match event {
                    Ok(event) => {
                        if socket.send(Message::Text(event.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            revoked = revocations.recv() => {
                match revoked {
                    Ok(revoked) if revoked == device_id => {
                        let _ = socket.send(Message::Close(None)).await;
                        break;
                    }
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

fn forbidden_command(command: &str) -> bool {
    matches!(
        command,
        "pty_open"
            | "pty_write"
            | "pty_resize"
            | "pty_close"
            | "open_in_terminal"
            | "write_instructions"
            | "save_projects"
            | "job_new"
            | "job_run"
            | "write_task"
            | "save_model"
    ) || command.starts_with("pty_")
}

fn parse_args<T: DeserializeOwned>(args: Value) -> Result<T, String> {
    serde_json::from_value(args).map_err(|error| format!("Некорректные аргументы: {error}"))
}

fn as_json<T: Serialize>(result: Result<T, String>) -> Result<Value, String> {
    result.and_then(|value| {
        serde_json::to_value(value)
            .map_err(|error| format!("Не удалось сериализовать ответ: {error}"))
    })
}

async fn dispatch_request(state: &WebState, request: ProtocolRequest) -> ProtocolResponse {
    let id = request.id;
    if forbidden_command(&request.cmd) {
        return ProtocolResponse::failure(id, FORBIDDEN_ERROR);
    }
    let result = dispatch_command(state, &request.cmd, request.args).await;
    match result {
        Ok(result) => ProtocolResponse::success(id, result),
        Err(error) => ProtocolResponse::failure(id, error),
    }
}

async fn dispatch_command(state: &WebState, command: &str, args: Value) -> Result<Value, String> {
    #[derive(Deserialize)]
    struct NameArgs {
        name: String,
    }
    #[derive(Deserialize)]
    struct ScreenArgs {
        name: String,
        lines: Option<u32>,
    }
    #[derive(Deserialize)]
    struct SendArgs {
        name: String,
        text: String,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ProjectArgs {
        project_path: String,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ConductorSendArgs {
        project_path: String,
        text: String,
    }

    match command {
        "list_projects" => as_json(crate::projects::list_projects()),
        "list_jobs" => as_json(crate::jobs::list_jobs().await),
        "job_screen" => {
            let args: ScreenArgs = parse_args(args)?;
            as_json(crate::jobs::job_screen(args.name, args.lines).await)
        }
        "job_send" => {
            let args: SendArgs = parse_args(args)?;
            as_json(crate::jobs::job_send(args.name, args.text).await)
        }
        "job_kill" => {
            let args: NameArgs = parse_args(args)?;
            as_json(crate::jobs::job_kill(args.name).await)
        }
        "limits" => as_json(crate::limits::limits().await),
        "setup_status" => as_json(crate::setup::setup_status().await),
        "list_models" => as_json(crate::models::list_models().await),
        "conductor_start" => {
            let args: ProjectArgs = parse_args(args)?;
            let app = state
                .app
                .clone()
                .ok_or_else(|| "Команда недоступна без приложения".to_owned())?;
            let conductor = app.state::<crate::conductor::ConductorState>();
            as_json(crate::conductor::conductor_start(
                app.clone(),
                conductor,
                args.project_path,
            ))
        }
        "conductor_send" => {
            let args: ConductorSendArgs = parse_args(args)?;
            let app = state
                .app
                .as_ref()
                .ok_or_else(|| "Команда недоступна без приложения".to_owned())?;
            as_json(crate::conductor::conductor_send(
                app.state::<crate::conductor::ConductorState>(),
                args.project_path,
                args.text,
            ))
        }
        "conductor_stop" => {
            let args: ProjectArgs = parse_args(args)?;
            let app = state
                .app
                .as_ref()
                .ok_or_else(|| "Команда недоступна без приложения".to_owned())?;
            as_json(crate::conductor::conductor_stop(
                app.state::<crate::conductor::ConductorState>(),
                args.project_path,
            ))
        }
        _ => Err(format!("Неизвестная команда: {command}")),
    }
}

async fn pwa_root(AxumState(state): AxumState<WebState>) -> Response {
    serve_pwa_file(&state.pwa_dir, "index.html").await
}

async fn pwa_file(
    AxumState(state): AxumState<WebState>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    serve_pwa_file(&state.pwa_dir, &path).await
}

async fn serve_pwa_file(pwa_dir: &Path, relative: &str) -> Response {
    if !pwa_dir.is_dir() {
        return (StatusCode::NOT_FOUND, "PWA ещё не собран").into_response();
    }
    let relative = Path::new(relative);
    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    let candidate = pwa_dir.join(relative);
    let Ok(root) = tokio::fs::canonicalize(pwa_dir).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(candidate) = tokio::fs::canonicalize(candidate).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !candidate.starts_with(root) || !candidate.is_file() {
        return StatusCode::NOT_FOUND.into_response();
    }
    match tokio::fs::read(&candidate).await {
        Ok(contents) => {
            let content_type = match candidate.extension().and_then(|value| value.to_str()) {
                Some("html") => "text/html; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                Some("js" | "mjs") => "text/javascript; charset=utf-8",
                Some("json" | "webmanifest") => "application/json; charset=utf-8",
                Some("svg") => "image/svg+xml",
                Some("png") => "image/png",
                Some("jpg" | "jpeg") => "image/jpeg",
                Some("webp") => "image/webp",
                Some("woff2") => "font/woff2",
                _ => "application/octet-stream",
            };
            ([(header::CONTENT_TYPE, content_type)], contents).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use serde_json::json;

    use super::*;

    #[test]
    fn pairing_code_and_token_have_expected_shape_and_ttl() {
        let start = Instant::now();
        let utc = Utc::now();
        let pairing = new_pairing(start, utc);
        assert_eq!(pairing.code.len(), 6);
        assert!(pairing.code.bytes().all(|byte| byte.is_ascii_digit()));
        assert_eq!(URL_SAFE_NO_PAD.decode(&pairing.token).unwrap().len(), 32);
        assert_eq!(pairing.expires_at.duration_since(start), PAIRING_TTL);
        assert_eq!(pairing.expires_at_utc - utc, chrono::Duration::minutes(5));
        assert!(pairing.expires_at > start + PAIRING_TTL - Duration::from_millis(1));
        assert!(pairing.expires_at <= start + PAIRING_TTL);
        assert!(pairing_matches(
            &pairing,
            Some(&pairing.code),
            None,
            start + Duration::from_secs(1),
        ));
        assert!(pairing_matches(
            &pairing,
            None,
            Some(&pairing.token),
            start + Duration::from_secs(1),
        ));
        assert!(!pairing_matches(
            &pairing,
            Some(&pairing.code),
            Some(&pairing.token),
            start + PAIRING_TTL,
        ));
    }

    #[test]
    fn expired_pairing_is_rejected() {
        let state = CompanionState::at(std::env::temp_dir().join("takt-expired-pairing"));
        let start = Instant::now();
        let pairing = new_pairing(start, Utc::now());
        let code = pairing.code.clone();
        state.inner.lock().unwrap().pairing = Some(pairing);
        let result = pair_device(
            &state,
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            PairRequest {
                code: Some(code),
                token: None,
                name: "Phone".to_owned(),
            },
            start + PAIRING_TTL,
        );
        assert!(matches!(result, Err((StatusCode::UNAUTHORIZED, _))));
    }

    #[test]
    fn bearer_is_verified_only_by_stored_hash() {
        let mut key = [0_u8; 32];
        key[0] = 42;
        let encoded = URL_SAFE_NO_PAD.encode(key);
        let devices = vec![StoredDevice {
            id: "device".to_owned(),
            name: "Phone".to_owned(),
            key_hash: hash_key(&key),
            created_at: now_string(),
            last_seen: now_string(),
        }];
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            format!("Bearer {encoded}").parse().unwrap(),
        );
        assert_eq!(authenticate(&headers, &devices).as_deref(), Some("device"));
        headers.insert(
            header::AUTHORIZATION,
            format!("Bearer {}", random_urlsafe::<32>())
                .parse()
                .unwrap(),
        );
        assert!(authenticate(&headers, &devices).is_none());
        assert!(!serde_json::to_string(&devices).unwrap().contains(&encoded));
    }

    #[test]
    fn phone_forbidden_commands_are_filtered() {
        for command in [
            "pty_open",
            "pty_write",
            "pty_resize",
            "pty_close",
            "open_in_terminal",
            "write_instructions",
            "save_projects",
            "job_new",
            "job_run",
            "write_task",
            "save_model",
        ] {
            assert!(forbidden_command(command), "{command}");
        }
        for command in ["list_projects", "job_screen", "limits", "conductor_send"] {
            assert!(!forbidden_command(command), "{command}");
        }
    }

    #[test]
    fn protocol_serializes_success_error_and_event_shapes() {
        let response = ProtocolResponse::success(json!(7), json!({"value": 1}));
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            json!({"id": 7, "ok": true, "result": {"value": 1}})
        );
        let response = ProtocolResponse::failure(json!("a"), "ошибка");
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            json!({"id": "a", "ok": false, "error": "ошибка"})
        );
        let event = json!({"event": "jobs://changed", "payload": []});
        assert_eq!(event["event"], "jobs://changed");
    }

    #[test]
    fn rate_limit_blocks_fifth_failure_for_ten_minutes() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5));
        let start = Instant::now();
        let mut limiter = RateLimiter::default();
        for offset in 0..4 {
            assert!(!limiter.record_failure(ip, start + Duration::from_secs(offset)));
        }
        assert!(limiter.record_failure(ip, start + Duration::from_secs(4)));
        assert!(limiter.is_blocked(ip, start + RATE_LIMIT_BLOCK - Duration::from_secs(1)));
        assert!(!limiter.is_blocked(ip, start + RATE_LIMIT_BLOCK + Duration::from_secs(5)));
    }
}

#[cfg(test)]
#[tokio::test]
#[ignore = "поднимает локальный TLS-сервер и вызывает реальный codexbar"]
async fn live() {
    use futures_util::{SinkExt, StreamExt};
    use rustls::pki_types::ServerName;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let state = CompanionState::default();
    let status = start_server(
        state.clone(),
        None,
        Ipv4Addr::LOCALHOST,
        PathBuf::from("missing-pwa"),
        false,
        false,
    )
    .await
    .expect("start companion");
    let port = status.port.expect("port");
    let pair = begin_pairing(&state).expect("pair begin");
    let cert_pem = fs::read(state.base_dir().unwrap().join("cert.pem")).unwrap();
    let mut roots = rustls::RootCertStore::empty();
    let mut reader = BufReader::new(cert_pem.as_slice());
    for cert in rustls_pemfile::certs(&mut reader) {
        roots.add(cert.unwrap()).unwrap();
    }
    let tls = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let body = json!({"code": pair.code, "name": "Live test"}).to_string();
    let tcp = tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, port))
        .await
        .expect("pair TCP");
    let mut stream = tokio_rustls::TlsConnector::from(Arc::new(tls.clone()))
        .connect(ServerName::try_from("localhost").unwrap(), tcp)
        .await
        .expect("pair TLS");
    stream
        .write_all(
            format!(
                "POST /pair HTTP/1.1\r\nHost: localhost:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .as_bytes(),
        )
        .await
        .expect("pair request");
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .expect("pair response");
    let response = String::from_utf8(response).expect("UTF-8 HTTP response");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    let paired: Value = serde_json::from_str(response.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    let key = paired["key"].as_str().expect("key");
    let device_id = paired["deviceId"].as_str().expect("device id").to_owned();
    let request = axum::http::Request::builder()
        .uri(format!("wss://localhost:{port}/ws"))
        .header(header::AUTHORIZATION, format!("Bearer {key}"))
        .header(header::HOST, format!("localhost:{port}"))
        .header(header::UPGRADE, "websocket")
        .header(header::CONNECTION, "Upgrade")
        .header("Sec-WebSocket-Version", "13")
        .header(
            "Sec-WebSocket-Key",
            tokio_tungstenite::tungstenite::handshake::client::generate_key(),
        )
        .body(())
        .unwrap();
    let (mut socket, _) = tokio_tungstenite::connect_async_tls_with_config(
        request,
        None,
        false,
        Some(tokio_tungstenite::Connector::Rustls(Arc::new(tls))),
    )
    .await
    .expect("websocket connect");
    socket
        .send(tokio_tungstenite::tungstenite::Message::Text(
            json!({"id": 1, "cmd": "limits", "args": {}})
                .to_string()
                .into(),
        ))
        .await
        .expect("send limits");
    let response = socket.next().await.unwrap().unwrap();
    let response: Value = serde_json::from_str(response.to_text().unwrap()).unwrap();
    assert_eq!(response["id"], 1);
    assert_eq!(response["ok"], true);
    assert!(response["result"].is_array());
    let _ = socket.close(None).await;
    revoke_device(&state, &device_id).expect("remove live test device");
    stop_server(&state, None).expect("stop companion");
}
