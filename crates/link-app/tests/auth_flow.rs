//! Потоки входа Link на моке сервера (D-09..D-14, D-25). SSO-ветка проверена ТОЛЬКО на MockSso:
//! реальный SPNEGO проверяется живьём на Windows с AD (план 20). Имена вымышленные.

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use common::{api_err, ident, json, pin_of, spawn, Handler, Ident, MockSrv};
use link_app::authflow::{AuthFlow, AuthResult, Platform};
use link_app::config::{LinkConfig, LocalState, SourceKind, STATE_FILE};
use link_app::http_client::{DeviceCreds, LinkHttp};
use link_app::secrets::{login_token_key, Secret, TokenStore};
use link_app::sso::{NegotiateToken, SsoProvider, SsoProviderError};
use link_app::state::{CmdErrorCode, LoginMethod, LoginReason};

const DOMAIN: &str = "trackly.example.local";
const AUTH_OK_PASSWORD: &str = r#"{"user":{"id":1,"login":"ivanov","displayName":"Иванов И.И."},"method":"password","deviceToken":"tok-new"}"#;
const AUTH_OK_SSO: &str = r#"{"user":{"id":1,"login":"ivanov","displayName":"Иванов И.И."},"method":"sso","deviceToken":"tok-sso"}"#;
const AUTH_OK_RESUME: &str =
    r#"{"user":{"id":1,"login":"ivanov","displayName":"Иванов И.И."},"method":"password"}"#;

type SsoCase = (
    common::Resp,
    Result<Vec<u8>, SsoProviderError>,
    LoginReason,
    bool,
);

/// Провайдер SSO со счётчиком вызовов.
struct CountSso {
    calls: Arc<AtomicUsize>,
    result: Result<Vec<u8>, SsoProviderError>,
}

impl CountSso {
    fn new(result: Result<Vec<u8>, SsoProviderError>) -> (Self, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        (
            Self {
                calls: calls.clone(),
                result,
            },
            calls,
        )
    }
}

impl SsoProvider for CountSso {
    fn token(&self, _spn: &str) -> Result<NegotiateToken, SsoProviderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.result
            .as_ref()
            .map(|b| NegotiateToken::from_bytes(b))
            .map_err(|e| *e)
    }
}

struct Env {
    tmp: tempfile::TempDir,
    http: LinkHttp,
    tokens: TokenStore,
    local: LocalState,
    cfg: LinkConfig,
    _id: Ident,
    srv: MockSrv,
}

async fn env(host: &str, handler: Handler) -> Env {
    let id = ident("srv.example.local");
    let srv = spawn(&id, handler).await;
    let tmp = tempfile::tempdir().unwrap();
    let local = LocalState::load_or_init(tmp.path()).unwrap();
    let pin = pin_of(&id);
    let http = LinkHttp::new(
        "127.0.0.1",
        srv.port,
        pin,
        Some(DeviceCreds {
            device_id: local.device_id.clone(),
            secret: "dev-secret".into(),
        }),
    )
    .unwrap();
    let cfg = LinkConfig {
        host: host.to_string(),
        port: srv.port,
        pin,
        spn: None,
        source: SourceKind::User,
    };
    Env {
        tmp,
        http,
        tokens: TokenStore::new(None),
        local,
        cfg,
        _id: id,
        srv,
    }
}

impl Env {
    fn flow<'a>(&'a mut self, sso: &'a dyn SsoProvider, platform: Platform) -> AuthFlow<'a> {
        AuthFlow {
            http: &self.http,
            sso,
            tokens: &self.tokens,
            local: &mut self.local,
            cfg: &self.cfg,
            platform,
        }
    }

    fn key(&self) -> String {
        login_token_key(&self.cfg.host, self.cfg.port)
    }

    fn set_token(&self, v: &str) {
        self.tokens.set(&self.key(), &Secret::new(v));
    }

    fn token(&self) -> Option<String> {
        self.tokens.get(&self.key()).map(|s| s.expose().to_string())
    }

    fn state_on_disk(&self) -> LocalState {
        LocalState::load_or_init(self.tmp.path()).unwrap()
    }
}

fn routes(f: impl Fn(&str) -> common::Resp + Send + Sync + 'static) -> Handler {
    Arc::new(move |r| f(&r.path))
}

fn standard() -> Handler {
    routes(|p| match p {
        "/api/v1/link_auth_login" => json(200, AUTH_OK_PASSWORD),
        "/api/v1/link_auth_sso" => json(200, AUTH_OK_SSO),
        "/api/v1/link_resume" => json(200, AUTH_OK_RESUME),
        "/api/v1/link_logout" => json(200, "{}"),
        _ => json(404, "{}"),
    })
}

fn no_sso() -> (CountSso, Arc<AtomicUsize>) {
    CountSso::new(Err(SsoProviderError::Unsupported))
}

#[tokio::test]
async fn test1_password_login_stores_token_but_not_password() {
    let mut e = env(DOMAIN, standard()).await;
    let (sso, _) = no_sso();
    let r = e
        .flow(&sso, Platform::Other)
        .password_login("ivanov", Secret::new("Секрет-123"))
        .await
        .unwrap();
    assert_eq!(
        r,
        AuthResult::Connected {
            user: link_app::state::LinkUserView {
                display_name: "Иванов И.И.".into(),
                login: "ivanov".into()
            },
            method: LoginMethod::Password
        }
    );
    assert_eq!(e.token().as_deref(), Some("tok-new"));
    let state_file = std::fs::read_to_string(e.tmp.path().join(STATE_FILE)).unwrap();
    assert!(!state_file.contains("Секрет-123"));
    assert_ne!(e.token().as_deref(), Some("Секрет-123"));
}

#[tokio::test]
async fn test2_startup_resumes_by_token_without_sso() {
    let mut e = env(DOMAIN, standard()).await;
    e.set_token("tok-old");
    let (sso, calls) = CountSso::new(Ok(b"t".to_vec()));
    let r = e.flow(&sso, Platform::Windows).startup_auth().await;
    assert!(
        matches!(
            r,
            AuthResult::Connected {
                method: LoginMethod::Password,
                ..
            }
        ),
        "{r:?}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(e.srv.count_path("/api/v1/link_resume"), 1);
    assert_eq!(e.srv.count_path("/api/v1/link_auth_sso"), 0);
    assert!(e
        .srv
        .last("/api/v1/link_resume")
        .unwrap()
        .body
        .contains("tok-old"));
}

#[tokio::test]
async fn test3_resume_network_error_is_offline_and_keeps_token() {
    let mut e = env(DOMAIN, standard()).await;
    // Тот же пин, но порт, на котором никто не слушает.
    let free = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dead_port = free.local_addr().unwrap().port();
    drop(free);
    e.http = LinkHttp::new("127.0.0.1", dead_port, e.cfg.pin, None).unwrap();
    e.set_token("tok-old");
    let (sso, calls) = CountSso::new(Ok(b"t".to_vec()));
    let r = e.flow(&sso, Platform::Windows).startup_auth().await;
    assert_eq!(r, AuthResult::Offline);
    assert_eq!(e.token().as_deref(), Some("tok-old"));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn test3b_resume_5xx_is_offline() {
    let mut e = env(DOMAIN, routes(|_| json(500, "{}"))).await;
    e.set_token("tok-old");
    let (sso, _) = no_sso();
    let r = e.flow(&sso, Platform::Other).startup_auth().await;
    assert_eq!(r, AuthResult::Offline);
    assert!(e.token().is_some());
}

#[tokio::test]
async fn test4_invalid_token_is_erased_then_login_needed() {
    let mut e = env(DOMAIN, routes(|_| api_err(401, "LINK_NO_SESSION"))).await;
    e.set_token("tok-stale");
    let (sso, _) = no_sso();
    let r = e.flow(&sso, Platform::Other).startup_auth().await;
    assert_eq!(
        r,
        AuthResult::NeedsLogin {
            reason: None,
            can_retry_sso: false,
            after_logout: false
        }
    );
    assert!(e.token().is_none());
    assert!(!e.state_on_disk().revoked);
}

#[tokio::test]
async fn test4b_invalid_token_then_sso_on_windows() {
    let handler = routes(|p| match p {
        "/api/v1/link_resume" => api_err(401, "LINK_NO_SESSION"),
        "/api/v1/link_auth_sso" => json(200, AUTH_OK_SSO),
        _ => json(404, "{}"),
    });
    let mut e = env(DOMAIN, handler).await;
    e.set_token("tok-stale");
    let (sso, calls) = CountSso::new(Ok(b"ticket".to_vec()));
    let r = e.flow(&sso, Platform::Windows).startup_auth().await;
    assert!(
        matches!(
            r,
            AuthResult::Connected {
                method: LoginMethod::Sso,
                ..
            }
        ),
        "{r:?}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(e.token().as_deref(), Some("tok-sso"));
    let head = e.srv.last("/api/v1/link_auth_sso").unwrap().head;
    assert!(head.contains("authorization: negotiate "));
}

#[tokio::test]
async fn test5_revoked_blocks_sso_until_password_login_accepted() {
    for code in ["LINK_REVOKED", "LINK_DEVICE_BANNED"] {
        let handler = routes(move |p| match p {
            "/api/v1/link_resume" => api_err(401, code),
            "/api/v1/link_auth_login" => json(200, AUTH_OK_PASSWORD),
            _ => json(404, "{}"),
        });
        let mut e = env(DOMAIN, handler).await;
        e.set_token("tok-old");
        let (sso, calls) = CountSso::new(Ok(b"t".to_vec()));
        assert_eq!(
            e.flow(&sso, Platform::Windows).startup_auth().await,
            AuthResult::Revoked
        );
        assert!(e.token().is_none());
        assert!(e.state_on_disk().revoked, "признак переживает перезапуск");

        // Следующий запуск: ни SSO, ни сетевых запросов.
        let before = e.srv.requests();
        assert_eq!(
            e.flow(&sso, Platform::Windows).startup_auth().await,
            AuthResult::Revoked
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(e.srv.requests(), before);
        // Явная кнопка автовхода тоже не обходит отзыв.
        assert_eq!(
            e.flow(&sso, Platform::Windows).retry_sso().await,
            AuthResult::Revoked
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        // Вход паролем разрешён и снимает отзыв.
        let r = e
            .flow(&sso, Platform::Windows)
            .password_login("ivanov", Secret::new("x"))
            .await
            .unwrap();
        assert!(matches!(r, AuthResult::Connected { .. }));
        assert!(!e.local.revoked && !e.state_on_disk().revoked);
    }
}

#[tokio::test]
async fn test6_ip_address_skips_sso_and_flow_never_sends_password() {
    let mut e = env("10.0.0.5", standard()).await;
    let (sso, calls) = CountSso::new(Ok(b"t".to_vec()));
    let r = e.flow(&sso, Platform::Windows).startup_auth().await;
    assert_eq!(
        r,
        AuthResult::NeedsLogin {
            reason: Some(LoginReason::IpAddress),
            can_retry_sso: false,
            after_logout: false
        }
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(e.srv.count_path("/api/v1/link_auth_login"), 0);
}

#[tokio::test]
async fn test6b_sso_failures_map_to_reasons_and_never_send_password() {
    // (ответ сервера на SSO, провайдер, ожидаемая причина, can_retry)
    let cases: Vec<SsoCase> = vec![
        (
            json(200, AUTH_OK_SSO),
            Err(SsoProviderError::NoTicket),
            LoginReason::Kerberos,
            true,
        ),
        (
            json(200, AUTH_OK_SSO),
            Err(SsoProviderError::NtlmFallback),
            LoginReason::Kerberos,
            true,
        ),
        (
            api_err(401, "LINK_SSO_DENIED"),
            Ok(b"t".to_vec()),
            LoginReason::NotMapped,
            true,
        ),
        (
            api_err(401, "LINK_SSO_CHALLENGE"),
            Ok(b"t".to_vec()),
            LoginReason::Kerberos,
            true,
        ),
        (
            api_err(503, "LINK_SSO_DISABLED"),
            Ok(b"t".to_vec()),
            LoginReason::SsoDisabled,
            false,
        ),
    ];
    for (resp, provider, reason, retry) in cases {
        let (status, body) = (resp.status, resp.body.clone());
        let handler: Handler = Arc::new(move |_| json(status, &body));
        let mut e = env(DOMAIN, handler).await;
        let (sso, _) = CountSso::new(provider);
        let r = e.flow(&sso, Platform::Windows).startup_auth().await;
        assert_eq!(
            r,
            AuthResult::NeedsLogin {
                reason: Some(reason),
                can_retry_sso: retry,
                after_logout: false
            }
        );
        assert_eq!(e.srv.count_path("/api/v1/link_auth_login"), 0);
        assert!(e.token().is_none());
    }
}

#[tokio::test]
async fn test6c_sso_network_failure_is_no_answer() {
    let mut e = env(DOMAIN, standard()).await;
    let free = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dead_port = free.local_addr().unwrap().port();
    drop(free);
    e.http = LinkHttp::new("127.0.0.1", dead_port, e.cfg.pin, None).unwrap();
    let (sso, _) = CountSso::new(Ok(b"t".to_vec()));
    let r = e.flow(&sso, Platform::Windows).startup_auth().await;
    assert_eq!(
        r,
        AuthResult::NeedsLogin {
            reason: Some(LoginReason::NoAnswer),
            can_retry_sso: true,
            after_logout: false
        }
    );
}

#[tokio::test]
async fn test7_logout_blocks_auto_sso_until_retry() {
    let mut e = env(DOMAIN, standard()).await;
    e.set_token("tok-old");
    let (sso, calls) = CountSso::new(Ok(b"t".to_vec()));
    let r = e.flow(&sso, Platform::Windows).logout().await;
    assert_eq!(
        r,
        AuthResult::NeedsLogin {
            reason: None,
            can_retry_sso: true,
            after_logout: true
        }
    );
    assert_eq!(e.srv.count_path("/api/v1/link_logout"), 1);
    assert!(e.token().is_none());
    assert!(e.state_on_disk().logged_out);

    // Старт после выхода: SSO не вызывается.
    let r = e.flow(&sso, Platform::Windows).startup_auth().await;
    assert_eq!(
        r,
        AuthResult::NeedsLogin {
            reason: None,
            can_retry_sso: true,
            after_logout: true
        }
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    // «Войти автоматически» снимает флаг и пробует SSO.
    let r = e.flow(&sso, Platform::Windows).retry_sso().await;
    assert!(
        matches!(
            r,
            AuthResult::Connected {
                method: LoginMethod::Sso,
                ..
            }
        ),
        "{r:?}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(!e.state_on_disk().logged_out);
}

#[tokio::test]
async fn test7b_logout_works_locally_when_server_unreachable() {
    let mut e = env(DOMAIN, standard()).await;
    let free = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dead_port = free.local_addr().unwrap().port();
    drop(free);
    e.http = LinkHttp::new("127.0.0.1", dead_port, e.cfg.pin, None).unwrap();
    e.set_token("tok-old");
    let (sso, _) = no_sso();
    let r = e.flow(&sso, Platform::Windows).logout().await;
    assert!(matches!(
        r,
        AuthResult::NeedsLogin {
            after_logout: true,
            ..
        }
    ));
    assert!(e.token().is_none());
    assert!(e.state_on_disk().logged_out);
}

#[tokio::test]
async fn test8_non_windows_goes_straight_to_login_form() {
    let mut e = env(DOMAIN, standard()).await;
    let (sso, calls) = CountSso::new(Ok(b"t".to_vec()));
    let expect = AuthResult::NeedsLogin {
        reason: None,
        can_retry_sso: false,
        after_logout: false,
    };
    assert_eq!(e.flow(&sso, Platform::Other).startup_auth().await, expect);
    assert_eq!(e.flow(&sso, Platform::Other).retry_sso().await, expect);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    // Выход вне Windows флаг не ставит: автовхода нет, блокировать нечего.
    let r = e.flow(&sso, Platform::Other).logout().await;
    assert!(matches!(
        r,
        AuthResult::NeedsLogin {
            can_retry_sso: false,
            after_logout: true,
            ..
        }
    ));
    assert!(!e.state_on_disk().logged_out);
}

#[tokio::test]
async fn test9_password_errors_map_to_cmd_codes() {
    let cases = [
        (429, "LINK_RATE_LIMITED", CmdErrorCode::RateLimited, false),
        (
            401,
            "LINK_BAD_CREDENTIALS",
            CmdErrorCode::BadCredentials,
            false,
        ),
        (403, "LINK_USER_BLOCKED", CmdErrorCode::UserBlocked, false),
        (403, "LINK_DEVICE_BANNED", CmdErrorCode::DeviceBanned, true),
    ];
    for (status, code, expect, revoked) in cases {
        let mut e = env(DOMAIN, routes(move |_| api_err(status, code))).await;
        e.set_token("tok-old");
        let (sso, _) = no_sso();
        let err = e
            .flow(&sso, Platform::Other)
            .password_login("ivanov", Secret::new("bad"))
            .await
            .unwrap_err();
        assert_eq!(err.code, expect, "{code}");
        assert_eq!(e.local.revoked, revoked, "{code}");
        assert_eq!(e.token().is_none(), revoked, "{code}");
    }
}

#[tokio::test]
async fn test9b_429_without_code_and_network_errors() {
    let mut e = env(DOMAIN, routes(|_| json(429, "{}"))).await;
    let (sso, _) = no_sso();
    let err = e
        .flow(&sso, Platform::Other)
        .password_login("ivanov", Secret::new("x"))
        .await
        .unwrap_err();
    assert_eq!(err.code, CmdErrorCode::RateLimited);

    let mut e = env(DOMAIN, routes(|_| json(500, "{}"))).await;
    let err = e
        .flow(&sso, Platform::Other)
        .password_login("ivanov", Secret::new("x"))
        .await
        .unwrap_err();
    assert_eq!(err.code, CmdErrorCode::AuthUnavailable);

    let free = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dead_port = free.local_addr().unwrap().port();
    drop(free);
    e.http = LinkHttp::new("127.0.0.1", dead_port, e.cfg.pin, None).unwrap();
    let err = e
        .flow(&sso, Platform::Other)
        .password_login("ivanov", Secret::new("x"))
        .await
        .unwrap_err();
    assert_eq!(err.code, CmdErrorCode::NoNetwork);
}

#[tokio::test]
async fn pin_mismatch_is_reported_and_no_request_sent() {
    let mut e = env(DOMAIN, standard()).await;
    e.http = LinkHttp::new("127.0.0.1", e.srv.port, [0x11; 32], None).unwrap();
    e.set_token("tok-old");
    let (sso, _) = CountSso::new(Ok(b"t".to_vec()));
    let r = e.flow(&sso, Platform::Windows).startup_auth().await;
    assert!(matches!(r, AuthResult::PinMismatch { .. }), "{r:?}");
    assert_eq!(e.srv.requests(), 0);
    assert!(e.token().is_some());
}

#[tokio::test]
async fn on_api_error_revokes_only_for_revoking_codes() {
    let mut e = env(DOMAIN, standard()).await;
    e.set_token("tok-old");
    let (sso, _) = no_sso();
    let mut f = e.flow(&sso, Platform::Other);
    assert!(f.on_api_error("LINK_BAD_CREDENTIALS").is_none());
    assert_eq!(f.on_api_error("LINK_REVOKED"), Some(AuthResult::Revoked));
    assert!(e.token().is_none() && e.state_on_disk().revoked);
}
