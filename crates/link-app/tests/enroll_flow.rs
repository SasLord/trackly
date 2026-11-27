//! Первая настройка по коду-приглашению на моке сервера (D-03, D-04). Имена вымышленные.

mod common;

use std::path::Path;
use std::sync::{Arc, Mutex};

use common::{api_err, ident, json, pin_of, spawn, Handler, Ident, MockSrv};
use link_app::config::{LocalState, USER_CONFIG_FILE};
use link_app::enroll::{setup_with_invite, ClientInfo, ConfigPaths, EnrollError};
use link_app::secrets::{device_secret_key, file_store_dir, DeviceSecretStore, FileSecretStore};
use link_app::state::CmdErrorCode;
use link_proto::InviteCode;

const SECRET: [u8; 16] = [0x5A; 16];

fn far_future() -> i64 {
    4_000_000_000
}

fn code_for(port: u16, pin: [u8; 32], exp: i64) -> String {
    InviteCode {
        host: "127.0.0.1".into(),
        port,
        spki: pin,
        secret: SECRET,
        exp_unix: exp,
    }
    .encode()
    .unwrap()
}

fn ok_handler() -> Handler {
    Arc::new(|r| {
        if r.path == "/api/v1/link_enroll" {
            json(200, r#"{"deviceSecret":"issued-device-secret"}"#)
        } else {
            json(404, "{}")
        }
    })
}

struct Env {
    _tmp: tempfile::TempDir,
    paths: ConfigPaths,
    secrets: DeviceSecretStore,
    local: LocalState,
}

fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let user_dir = tmp.path().join("user");
    let state_dir = tmp.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let local = LocalState::load_or_init(&state_dir).unwrap();
    let secrets = DeviceSecretStore::new(None, FileSecretStore::new(file_store_dir(&user_dir)));
    let paths = ConfigPaths {
        machine_config: tmp.path().join("machine").join("link.toml"),
        user_dir,
    };
    Env {
        _tmp: tmp,
        paths,
        secrets,
        local,
    }
}

fn info() -> ClientInfo {
    ClientInfo {
        hostname: "PC-IVANOV".into(),
        os: "windows".into(),
        arch: "x86_64".into(),
        version: "0.1.0".into(),
    }
}

fn is_empty_tree(p: &Path) -> bool {
    match std::fs::read_dir(p) {
        Err(_) => true,
        Ok(rd) => rd
            .flatten()
            .all(|e| is_empty_tree(&e.path()) && e.path().is_dir()),
    }
}

async fn srv(id: &Ident, h: Handler) -> MockSrv {
    spawn(id, h).await
}

#[tokio::test]
async fn test1_valid_code_saves_config_and_device_secret() {
    let id = ident("srv.example.local");
    let s = srv(&id, ok_handler()).await;
    let e = env();
    let cfg = setup_with_invite(
        &code_for(s.port, pin_of(&id), far_future()),
        &e.paths,
        &e.secrets,
        &e.local,
        &info(),
    )
    .await
    .unwrap();
    assert_eq!(
        (cfg.host.as_str(), cfg.port, cfg.pin),
        ("127.0.0.1", s.port, pin_of(&id))
    );
    let toml = std::fs::read_to_string(e.paths.user_dir.join(USER_CONFIG_FILE)).unwrap();
    assert!(toml.contains(&format!("127.0.0.1:{}", s.port)));
    let stored = e
        .secrets
        .get(&device_secret_key("127.0.0.1", s.port))
        .unwrap()
        .unwrap();
    assert_eq!(stored.expose(), "issued-device-secret");
    // Тело запроса: device_id из локального состояния; сотрудника в нём нет (D-04).
    let req = s.last("/api/v1/link_enroll").unwrap();
    assert!(req.body.contains(&e.local.device_id));
    assert!(!req.body.contains("login") && !req.body.contains("password"));
}

#[tokio::test]
async fn test2_pin_mismatch_sends_nothing_and_saves_nothing() {
    let served = ident("srv.example.local");
    let other = ident("impostor.example.local");
    let s = srv(&served, ok_handler()).await;
    let e = env();
    let err = setup_with_invite(
        &code_for(s.port, pin_of(&other), far_future()),
        &e.paths,
        &e.secrets,
        &e.local,
        &info(),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, EnrollError::PinMismatch { .. }), "{err:?}");
    assert_eq!(err.to_cmd_error().code, CmdErrorCode::PinMismatch);
    assert_eq!(s.requests(), 0, "ни один запрос не должен дойти до сервера");
    assert!(is_empty_tree(e._tmp.path().join("user").as_path()));
    assert!(e
        .secrets
        .get(&device_secret_key("127.0.0.1", s.port))
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn test3_expired_and_broken_codes_make_no_network_call() {
    let id = ident("srv.example.local");
    let s = srv(&id, ok_handler()).await;
    let e = env();
    let expired = setup_with_invite(
        &code_for(s.port, pin_of(&id), 1_000),
        &e.paths,
        &e.secrets,
        &e.local,
        &info(),
    )
    .await
    .unwrap_err();
    assert_eq!(expired, EnrollError::ExpiredCode);
    let broken = setup_with_invite("TL1.не-код", &e.paths, &e.secrets, &e.local, &info())
        .await
        .unwrap_err();
    assert_eq!(broken, EnrollError::InvalidCode);
    let empty = setup_with_invite("  \n ", &e.paths, &e.secrets, &e.local, &info())
        .await
        .unwrap_err();
    assert_eq!(empty, EnrollError::EmptyCode);
    assert_eq!(s.connections.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert!(is_empty_tree(&e.paths.user_dir));
}

#[tokio::test]
async fn test4_server_invite_errors_map_and_save_nothing() {
    for (code, expect, cmd) in [
        (
            "LINK_INVITE_USED",
            EnrollError::UsedCode,
            CmdErrorCode::UsedCode,
        ),
        (
            "LINK_INVITE_EXPIRED",
            EnrollError::ExpiredCode,
            CmdErrorCode::ExpiredCode,
        ),
        (
            "LINK_INVITE_INVALID",
            EnrollError::InvalidCode,
            CmdErrorCode::InvalidCode,
        ),
    ] {
        let id = ident("srv.example.local");
        let status = match code {
            "LINK_INVITE_USED" => 409,
            "LINK_INVITE_EXPIRED" => 410,
            _ => 400,
        };
        let s = srv(&id, Arc::new(move |_| api_err(status, code))).await;
        let e = env();
        let err = setup_with_invite(
            &code_for(s.port, pin_of(&id), far_future()),
            &e.paths,
            &e.secrets,
            &e.local,
            &info(),
        )
        .await
        .unwrap_err();
        assert_eq!(err, expect);
        assert_eq!(err.to_cmd_error().code, cmd);
        assert!(is_empty_tree(&e.paths.user_dir), "{code}");
    }
}

#[tokio::test]
async fn test5_unreachable_server_saves_nothing() {
    let id = ident("srv.example.local");
    let free = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = free.local_addr().unwrap().port();
    drop(free);
    let e = env();
    let err = setup_with_invite(
        &code_for(port, pin_of(&id), far_future()),
        &e.paths,
        &e.secrets,
        &e.local,
        &info(),
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, EnrollError::ServerUnreachable { .. }),
        "{err:?}"
    );
    let cmd = err.to_cmd_error();
    assert_eq!(cmd.code, CmdErrorCode::ServerUnreachable);
    assert!(cmd.message.contains(&format!("127.0.0.1:{port}")));
    assert!(is_empty_tree(&e.paths.user_dir));
}

#[tokio::test]
async fn test6_machine_config_wins_and_user_config_untouched() {
    let id = ident("srv.example.local");
    let s = srv(&id, ok_handler()).await;
    let e = env();
    std::fs::create_dir_all(e.paths.machine_config.parent().unwrap()).unwrap();
    std::fs::write(
        &e.paths.machine_config,
        "server = \"srv.example.local\"\npin_sha256 = \"\"\n",
    )
    .unwrap();
    let err = setup_with_invite(
        &code_for(s.port, pin_of(&id), far_future()),
        &e.paths,
        &e.secrets,
        &e.local,
        &info(),
    )
    .await
    .unwrap_err();
    assert_eq!(err, EnrollError::AlreadyConfigured);
    assert_eq!(s.requests(), 0);
    assert!(is_empty_tree(&e.paths.user_dir));
}

/// Минимальный перехватчик tracing: собирает текст всех событий.
struct Capture(Arc<Mutex<Vec<String>>>);

struct Collect<'a>(&'a mut String);
impl tracing::field::Visit for Collect<'_> {
    fn record_debug(&mut self, f: &tracing::field::Field, v: &dyn std::fmt::Debug) {
        self.0.push_str(&format!("{}={v:?} ", f.name()));
    }
}

impl tracing::Subscriber for Capture {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, ev: &tracing::Event<'_>) {
        let mut line = String::new();
        ev.record(&mut Collect(&mut line));
        self.0.lock().unwrap().push(line);
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

#[tokio::test]
async fn test7_invite_secret_never_logged() {
    use base64::Engine as _;
    let lines = Arc::new(Mutex::new(Vec::new()));
    // Глобальный: кэш интереса вызовов иначе уже закрыт параллельными тестами.
    tracing::subscriber::set_global_default(Capture(lines.clone())).unwrap();
    let id = ident("srv.example.local");
    let s = srv(&id, ok_handler()).await;
    let e = env();
    let code = code_for(s.port, pin_of(&id), far_future());
    setup_with_invite(&code, &e.paths, &e.secrets, &e.local, &info())
        .await
        .unwrap();
    let all = lines.lock().unwrap().join("\n");
    assert!(!all.is_empty(), "перехватчик ничего не поймал");
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(SECRET);
    assert!(!all.contains(&b64) && !all.contains(&code) && !all.contains("issued-device-secret"));
}
