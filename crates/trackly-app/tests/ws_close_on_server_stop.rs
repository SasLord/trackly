//! Отладочная сессия `ws-disconnect-toast-no-show` (GAP-1 фазы 41.7).
//!
//! Проверяет КОНТРАКТ остановки серверного режима: «сервер остановлен» должно
//! означать «живых соединений нет». Браузерный WS-клиент узнаёт об обрыве ТОЛЬКО
//! из события `onclose`; если после остановки сервера установленный сокет
//! остаётся открытым, `ui/src/lib/api/ws.ts` не получает `onclose`, не ставит
//! флаг эпизода и не показывает тост «Соединение с сервером потеряно» (D-18).
//!
//! Дефект: `start_server` отменой токена прерывал ТОЛЬКО цикл `accept`. Каждое
//! принятое соединение жило в отдельной detached-задаче `tokio::spawn`, которая
//! токен не видела, а `handle_ws_socket` висел в `select!` на двух вечно-pending
//! ветвях (`socket.recv()` — клиент молчит, сервер только пушит; `rx.recv()`
//! вернёт `Closed` лишь при дропе `broadcast::Sender`, который лежит в
//! `AppCtx.ws_broadcast` и переживает остановку сервера).
//!
//! Тест идёт по НАСТОЯЩЕМУ продакшн-пути: реальный `AppCtx`, реальный
//! `build_router`, реальный `start_server` поверх TLS, настоящая cookie сессии и
//! настоящий WS-клиент поверх rustls. Данные вымышленные.

mod entities_support;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use time::OffsetDateTime;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message as TMessage;
use tokio_util::sync::CancellationToken;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use entities_support::fixture::make_test_ctx;
use trackly_app::dto::auth::UserNew;
use trackly_app::dto::printer::WsEvent;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::server::{start_server, tls};
use trackly_core::auth::Role;

const BUDGET: Duration = Duration::from_secs(60);
/// Сколько даём клиенту на наблюдение закрытия после `cancel()`.
/// Браузер узнаёт об обрыве мгновенно; 5с — щедрый запас на CI.
const CLOSE_WINDOW: Duration = Duration::from_secs(5);
const RECV: Duration = Duration::from_secs(3);

// ---------------------------------------------------------------------------
// TLS-клиент
// ---------------------------------------------------------------------------

/// Клиентский верификатор, принимающий любой сертификат.
///
/// Доверие к self-signed сертификату ОРТОГОНАЛЬНО предмету теста (жизненный
/// цикл соединения при остановке сервера). Браузер в LAN решает тот же вопрос
/// постоянным исключением безопасности. Подписи рукопожатия при этом
/// проверяются по-настоящему — подменён только путь валидации цепочки.
#[derive(Debug)]
struct AcceptAnyServerCert(Arc<rustls::crypto::CryptoProvider>);

impl rustls::client::danger::ServerCertVerifier for AcceptAnyServerCert {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

fn client_tls_config() -> rustls::ClientConfig {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .expect("safe default protocol versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AcceptAnyServerCert(provider)))
        .with_no_client_auth();
    // Сервер прибивает ALPN к http/1.1 (`tls::pin_http1_alpn`) — WS-апгрейд
    // возможен только по HTTP/1.1, поэтому клиент предлагает то же самое.
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    config
}

type TlsWs = tokio_tungstenite::WebSocketStream<tokio_rustls::client::TlsStream<TcpStream>>;

/// Поднять WS поверх TLS к `/api/v1/ws` с cookie сессии.
async fn connect_wss(addr: SocketAddr, cookie: &str) -> TlsWs {
    let mut request = format!("wss://{addr}/api/v1/ws")
        .into_client_request()
        .expect("client request");
    request
        .headers_mut()
        .insert("cookie", cookie.parse().expect("cookie header"));

    let tcp = TcpStream::connect(addr).await.expect("tcp connect");
    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_tls_config()));
    let server_name = rustls::pki_types::ServerName::try_from("127.0.0.1").expect("server name");
    let tls_stream = connector
        .connect(server_name, tcp)
        .await
        .expect("tls handshake");

    let (ws, resp) = tokio_tungstenite::client_async(request, tls_stream)
        .await
        .expect("ws handshake (101 expected; 401 means the cookie was rejected)");
    assert_eq!(resp.status().as_u16(), 101, "handshake must upgrade");
    ws
}

// ---------------------------------------------------------------------------
// Окружение
// ---------------------------------------------------------------------------

async fn create_session_cookie(store: &RusqliteSessionStore, user_id: i64, role: Role) -> String {
    let session_id = Id::default();
    let si = SessionIdentity {
        user_id: Some(user_id),
        role: role.as_str().to_string(),
    };
    let mut record = Record {
        id: session_id,
        data: Default::default(),
        expiry_date: OffsetDateTime::now_utc() + time::Duration::days(1),
    };
    record.data.insert(
        "identity".to_string(),
        serde_json::to_value(&si).expect("serialize identity"),
    );
    store.create(&mut record).await.expect("create session");
    format!("id={session_id}")
}

struct Harness {
    ctx: trackly_app::context::AppCtx,
    _dir: tempfile::TempDir,
    addr: SocketAddr,
    cookie: String,
    /// Дочерний токен ЭТОГО инстанса сервера — ровно то, что дёргает
    /// «Остановить сервер» (`build_server_toggle_tauri` → `handle.cancel.cancel()`).
    server_token: CancellationToken,
    server_task: tokio::task::JoinHandle<()>,
}

async fn start_harness() -> Harness {
    let (ctx, _dir) = make_test_ctx().await;

    let admin_id = ctx
        .auth
        .create_user(
            UserNew {
                login: "ws_stop_admin".to_string(),
                full_name: "Иванов И.И.".to_string(),
                password: "password123".to_string(),
                role: "admin".to_string(),
                email: None,
            },
            &entities_support::fixture::admin(),
        )
        .await
        .expect("create admin")
        .id;

    let store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
    let cookie = create_session_cookie(&store, admin_id, Role::Admin).await;

    let router = build_router(
        &ctx,
        RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
    );
    let bundle = tls::generate_self_signed("127.0.0.1").expect("tls bundle");
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");

    let server_token = CancellationToken::new();
    let token_clone = server_token.clone();
    let server_task = tokio::spawn(async move {
        start_server(router, listener, bundle.acceptor, token_clone)
            .await
            .expect("start_server");
    });

    Harness {
        ctx,
        _dir,
        addr,
        cookie,
        server_token,
        server_task,
    }
}

/// Подтвердить, что сокет ЖИВОЙ и подписан — иначе последующее «закрылся»
/// ничего не доказывает (мог быть сломан апгрейд, как в Bug A).
async fn assert_socket_live(h: &Harness, ws: &mut TlsWs) {
    h.ctx
        .ws_broadcast
        .send(WsEvent::NumberSpaceChanged {
            contexts: vec!["device_create".to_string()],
        })
        .expect("send control event");

    loop {
        let msg = tokio::time::timeout(RECV, ws.next())
            .await
            .expect("контрольный кадр не пришёл — сокет не живой/не подписан")
            .expect("поток закрыт до остановки сервера")
            .expect("ошибка сокета до остановки сервера");
        match msg {
            TMessage::Text(t) => {
                assert!(
                    t.contains("number_space_changed"),
                    "ожидали контрольный кадр, пришло: {t}"
                );
                return;
            }
            TMessage::Ping(_) | TMessage::Pong(_) => continue,
            other => panic!("ожидали текстовый кадр, пришло {other:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// D-18: остановка сервера обязана закрыть установленный WS
// ---------------------------------------------------------------------------

/// Остановка серверного режима ОБЯЗАНА закрыть уже установленный WS-сокет.
///
/// Без этого браузер не получает `onclose`, `ws.ts` не вызывает
/// `showReconnectingToast()` и тост D-18 не появляется НИКОГДА (GAP-1 фазы 41.7).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stopping_server_closes_established_ws() {
    tokio::time::timeout(BUDGET, async {
        let h = start_harness().await;
        let mut ws = connect_wss(h.addr, &h.cookie).await;

        assert_socket_live(&h, &mut ws).await;

        // Ровно то, что делает кнопка «Остановить сервер».
        h.server_token.cancel();

        // Клиент обязан увидеть конец потока (Close-кадр или разрыв соединения).
        // `None` или `Err` — оба годятся: браузерный `onclose` наступает и там, и там.
        let observed_close = loop {
            match tokio::time::timeout(CLOSE_WINDOW, ws.next()).await {
                Err(_elapsed) => break false,
                Ok(None) => break true,
                Ok(Some(Err(_))) => break true,
                Ok(Some(Ok(TMessage::Close(_)))) => break true,
                // Служебные/прикладные кадры до закрытия — продолжаем ждать.
                Ok(Some(Ok(_))) => continue,
            }
        };

        assert!(
            observed_close,
            "после остановки сервера клиент должен увидеть закрытие WS в пределах \
             {CLOSE_WINDOW:?}; сокет остался открытым → браузерный onclose не наступает \
             → тост D-18 «Соединение с сервером потеряно» не показывается никогда"
        );

        let _ = tokio::time::timeout(Duration::from_secs(5), h.server_task).await;
    })
    .await
    .expect("test exceeded 60s budget");
}
