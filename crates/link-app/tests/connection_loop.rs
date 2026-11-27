//! Автомат связи Link (D-14, D-22..D-25). Имена и данные вымышленные.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use link_app::authflow::AuthResult;
use link_app::connection::{reduce, EventMeta, EventSink, Input, Machine, ReduceCtx};
use link_app::state::{LinkEvent, LinkState, LinkUserView, LoginMethod, LoginReason};

const SERVER: &str = "srv.example.local:8443";

fn user() -> LinkUserView {
    LinkUserView {
        display_name: "Иванов И.И.".into(),
        login: "ivanov".into(),
    }
}

fn connected_result() -> AuthResult {
    AuthResult::Connected {
        user: user(),
        method: LoginMethod::Password,
    }
}

fn connected_state() -> LinkState {
    LinkState::Connected {
        user: user(),
        method: LoginMethod::Password,
        server: SERVER.into(),
    }
}

fn ctx() -> ReduceCtx {
    ReduceCtx::new(SERVER)
}

#[derive(Default)]
struct FakeSink {
    states: Mutex<Vec<LinkState>>,
    events: Mutex<Vec<serde_json::Value>>,
    resyncs: Mutex<usize>,
}

impl EventSink for FakeSink {
    fn state_changed(&self, ev: &LinkEvent) {
        self.states.lock().unwrap().push(ev.state.clone());
    }
    fn bridge_event(&self, payload: serde_json::Value) {
        self.events.lock().unwrap().push(payload);
    }
    fn resync(&self) {
        *self.resyncs.lock().unwrap() += 1;
    }
}

fn meta() -> EventMeta {
    EventMeta {
        tray_available: true,
        secrets_persistent: true,
        platform: "test".into(),
    }
}

#[test]
fn reduce_starting_connected_and_silent_loss() {
    let mut c = ctx();
    let s = reduce(
        &LinkState::Starting,
        &mut c,
        Input::AuthDone(connected_result()),
    );
    assert_eq!(s, connected_state());
    let t0 = Instant::now();
    let s2 = reduce(&s, &mut c, Input::ConnectionLost { at: t0 });
    assert_eq!(s2, connected_state(), "обрыв сам по себе молчит");
}

#[test]
fn reduce_offline_only_after_threshold() {
    let mut c = ctx();
    let t0 = Instant::now();
    let s = connected_state();
    let s = reduce(&s, &mut c, Input::ConnectionLost { at: t0 });
    let s = reduce(
        &s,
        &mut c,
        Input::Tick {
            now: t0 + Duration::from_millis(4900),
        },
    );
    assert_eq!(s, connected_state());
    let s = reduce(
        &s,
        &mut c,
        Input::Tick {
            now: t0 + Duration::from_secs(5),
        },
    );
    assert_eq!(
        s,
        LinkState::Offline {
            server: SERVER.into()
        }
    );
}

#[test]
fn reduce_offline_restored_goes_straight_to_connected() {
    let mut c = ctx();
    let t0 = Instant::now();
    let s = reduce(
        &LinkState::Starting,
        &mut c,
        Input::AuthDone(connected_result()),
    );
    let s = reduce(&s, &mut c, Input::ConnectionLost { at: t0 });
    let s = reduce(
        &s,
        &mut c,
        Input::Tick {
            now: t0 + Duration::from_secs(6),
        },
    );
    assert!(matches!(s, LinkState::Offline { .. }));
    let s = reduce(&s, &mut c, Input::ConnectionRestored);
    assert_eq!(s, connected_state());
    assert!(c.offline_since.is_none());
}

#[test]
fn reduce_startup_offline_is_not_needs_login_but_server_verdict_is() {
    let mut c = ctx();
    let s = reduce(
        &LinkState::Starting,
        &mut c,
        Input::AuthDone(AuthResult::Offline),
    );
    assert_eq!(
        s,
        LinkState::Offline {
            server: SERVER.into()
        }
    );
    let s = reduce(
        &s,
        &mut c,
        Input::AuthDone(AuthResult::NeedsLogin {
            reason: Some(LoginReason::NoAnswer),
            can_retry_sso: false,
            after_logout: false,
        }),
    );
    assert!(matches!(s, LinkState::NeedsLogin { .. }));
}

#[test]
fn reduce_revoked_from_anywhere_and_sticky() {
    let t0 = Instant::now();
    for from in [
        LinkState::Starting,
        connected_state(),
        LinkState::Offline {
            server: SERVER.into(),
        },
    ] {
        let mut c = ctx();
        let s = reduce(&from, &mut c, Input::RevokedSignal);
        assert_eq!(
            s,
            LinkState::Revoked {
                server: SERVER.into()
            }
        );
        // Ни связь, ни тик, ни «восстановление» не выводят из Revoked.
        let s = reduce(&s, &mut c, Input::ConnectionRestored);
        let s = reduce(&s, &mut c, Input::ConnectionLost { at: t0 });
        let s = reduce(
            &s,
            &mut c,
            Input::Tick {
                now: t0 + Duration::from_secs(60),
            },
        );
        assert!(matches!(s, LinkState::Revoked { .. }));
    }
    // Только явный успешный вход (password_login) выводит из Revoked.
    let mut c = ctx();
    let revoked = LinkState::Revoked {
        server: SERVER.into(),
    };
    let s = reduce(&revoked, &mut c, Input::AuthDone(connected_result()));
    assert_eq!(s, connected_state());
}

#[test]
fn reduce_pin_mismatch_is_not_offline_until_recheck() {
    let t0 = Instant::now();
    let mut c = ctx();
    let pm = reduce(
        &LinkState::Starting,
        &mut c,
        Input::AuthDone(AuthResult::PinMismatch {
            expected: "aa".into(),
            got: "bb".into(),
        }),
    );
    assert!(matches!(pm, LinkState::PinMismatch { .. }));
    let s = reduce(&pm, &mut c, Input::ConnectionLost { at: t0 });
    let s = reduce(
        &s,
        &mut c,
        Input::Tick {
            now: t0 + Duration::from_secs(60),
        },
    );
    assert_eq!(s, pm);
    let s = reduce(&s, &mut c, Input::RecheckPin);
    assert_eq!(s, LinkState::Starting);
}

#[test]
fn reduce_sink_gets_exactly_one_event_per_change() {
    let sink = Arc::new(FakeSink::default());
    let mut m = Machine::new(SERVER, meta(), sink.clone(), Duration::from_secs(5));
    let t0 = Instant::now();
    assert!(m.apply(Input::AuthDone(connected_result())));
    // Повтор того же состояния не эмитится.
    assert!(!m.apply(Input::AuthDone(connected_result())));
    assert!(!m.apply(Input::ConnectionLost { at: t0 }));
    assert!(m.apply(Input::Tick {
        now: t0 + Duration::from_secs(5)
    }));
    assert!(!m.apply(Input::Tick {
        now: t0 + Duration::from_secs(9)
    }));
    assert!(m.apply(Input::ConnectionRestored));
    let states = sink.states.lock().unwrap();
    assert_eq!(states.len(), 3);
    assert!(matches!(states[0], LinkState::Connected { .. }));
    assert!(matches!(states[1], LinkState::Offline { .. }));
    assert!(matches!(states[2], LinkState::Connected { .. }));
}

// ---------------------------------------------------------------------------------------------
// ConnectionSupervisor на моке транспорта и виртуальном времени (D-14, D-22..D-25).
// ---------------------------------------------------------------------------------------------

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::SystemTime;

use link_app::connection::{
    Command, Connection, ConnectionSupervisor, SupervisorConfig, SupervisorHandle, Transport,
};
use link_app::ws_client::{WsError, WsFrame};
use tokio::time::Instant as TInstant;

struct MockConn {
    frames: VecDeque<WsFrame>,
}

impl Connection for MockConn {
    async fn next_frame(&mut self) -> WsFrame {
        match self.frames.pop_front() {
            Some(f) => f,
            None => std::future::pending().await,
        }
    }
}

type ConnectScript = VecDeque<Result<Vec<WsFrame>, WsError>>;

#[derive(Default)]
struct MockTransport {
    auth: Mutex<VecDeque<AuthResult>>,
    connects: Mutex<ConnectScript>,
    connect_times: Mutex<Vec<TInstant>>,
    auth_calls: AtomicUsize,
    revoke_calls: AtomicUsize,
}

impl MockTransport {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
    fn script_connects(&self, items: Vec<Result<Vec<WsFrame>, WsError>>) {
        *self.connects.lock().unwrap() = items.into();
    }
    fn script_auth(&self, items: Vec<AuthResult>) {
        *self.auth.lock().unwrap() = items.into();
    }
    fn times(&self) -> Vec<TInstant> {
        self.connect_times.lock().unwrap().clone()
    }
}

impl Transport for MockTransport {
    type Conn = MockConn;

    async fn authenticate(&self) -> AuthResult {
        self.auth_calls.fetch_add(1, Ordering::SeqCst);
        self.auth
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(connected_result)
    }

    async fn connect(&self) -> Result<MockConn, WsError> {
        self.connect_times.lock().unwrap().push(TInstant::now());
        let next = self.connects.lock().unwrap().pop_front();
        match next {
            Some(Ok(frames)) => Ok(MockConn {
                frames: frames.into(),
            }),
            Some(Err(e)) => Err(e),
            None => Ok(MockConn {
                frames: VecDeque::new(),
            }),
        }
    }

    async fn revoke(&self) {
        self.revoke_calls.fetch_add(1, Ordering::SeqCst);
    }

    async fn logout(&self) -> AuthResult {
        AuthResult::NeedsLogin {
            reason: None,
            can_retry_sso: false,
            after_logout: true,
        }
    }
}

fn hello() -> WsFrame {
    WsFrame::Hello {
        idle_timeout_secs: 60,
    }
}

fn start(t: &Arc<MockTransport>, sink: &Arc<FakeSink>) -> SupervisorHandle {
    start_with(t, sink, |_| {})
}

fn start_with(
    t: &Arc<MockTransport>,
    sink: &Arc<FakeSink>,
    tune: impl FnOnce(&mut SupervisorConfig),
) -> SupervisorHandle {
    let mut cfg = SupervisorConfig::new(SERVER, meta());
    cfg.jitter = Arc::new(|| 0.5);
    tune(&mut cfg);
    ConnectionSupervisor::spawn(t.clone(), sink.clone(), cfg)
}

async fn sleep_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

fn last_state(sink: &FakeSink) -> Option<LinkState> {
    sink.states.lock().unwrap().last().cloned()
}

fn resyncs(sink: &FakeSink) -> usize {
    *sink.resyncs.lock().unwrap()
}

#[tokio::test(start_paused = true)]
async fn loop_backoff_sequence_grows_to_30s_cap() {
    let t = MockTransport::new();
    t.script_connects((0..8).map(|_| Err(WsError::Network)).collect());
    let sink = Arc::new(FakeSink::default());
    let h = start(&t, &sink);
    sleep_ms(200_000).await;
    let times = t.times();
    assert!(times.len() >= 9, "попыток: {}", times.len());
    let gaps: Vec<u64> = times
        .windows(2)
        .take(7)
        .map(|w| (w[1] - w[0]).as_secs())
        .collect();
    assert_eq!(gaps, [1, 2, 4, 8, 16, 30, 30]);
    h.stop().await;
}

#[tokio::test(start_paused = true)]
async fn loop_sleep_jump_resets_backoff_and_retries_at_once() {
    let t = MockTransport::new();
    t.script_connects((0..20).map(|_| Err(WsError::Network)).collect());
    let sink = Arc::new(FakeSink::default());
    let base = SystemTime::now();
    let started = TInstant::now();
    let jump = Arc::new(AtomicU64::new(0));
    let jump_c = jump.clone();
    let h = start_with(&t, &sink, move |cfg| {
        cfg.wall = Arc::new(move || {
            base + started.elapsed() + Duration::from_secs(jump_c.load(Ordering::SeqCst))
        });
    });
    // Попытки на 0, 1, 3, 7 с; следующая по графику была бы на 15 с.
    sleep_ms(8_000).await;
    let before = t.times().len();
    assert_eq!(before, 4);
    jump.store(3_600, Ordering::SeqCst);
    sleep_ms(1_100).await; // не больше одного тика
    let after = t.times();
    // По графику следующая попытка была бы на 15 с: без сброса за 1,1 с её нет.
    assert!(after.len() > before, "попытка сразу после сна");
    // Пауза сброшена: паузы снова 1, 2, 4 с (попытки на 8, 9, 11 с), а без сброса
    // следующая была бы только на 15 с.
    sleep_ms(4_000).await;
    assert!(t.times().len() >= before + 3, "{}", t.times().len());
    h.stop().await;
}

#[tokio::test(start_paused = true)]
async fn loop_resync_once_after_reconnect_and_events_bridged() {
    let t = MockTransport::new();
    let ev = serde_json::json!({"type":"message","id":1});
    t.script_connects(vec![
        Ok(vec![hello(), WsFrame::Event(ev.clone()), WsFrame::Dead]),
        Ok(vec![hello()]),
    ]);
    let sink = Arc::new(FakeSink::default());
    let h = start(&t, &sink);
    sleep_ms(500).await;
    assert_eq!(sink.events.lock().unwrap().as_slice(), &[ev]);
    assert_eq!(resyncs(&sink), 0, "на первом подключении ресинка нет");
    sleep_ms(3_000).await;
    assert_eq!(t.times().len(), 2);
    assert_eq!(resyncs(&sink), 1);
    sleep_ms(30_000).await;
    assert_eq!(resyncs(&sink), 1, "ровно один");
    // Короткий обрыв (меньше порога) состояния не меняет: только Connected.
    assert_eq!(sink.states.lock().unwrap().len(), 1);
    h.stop().await;
}

#[tokio::test(start_paused = true)]
async fn loop_resync_required_frame_is_silent() {
    let t = MockTransport::new();
    t.script_connects(vec![Ok(vec![hello(), WsFrame::ResyncRequired])]);
    let sink = Arc::new(FakeSink::default());
    let h = start(&t, &sink);
    sleep_ms(500).await;
    assert_eq!(resyncs(&sink), 1);
    assert_eq!(sink.states.lock().unwrap().len(), 1, "смены состояния нет");
    h.stop().await;
}

#[tokio::test(start_paused = true)]
async fn loop_startup_without_network_is_quiet_offline_then_connects() {
    let t = MockTransport::new();
    t.script_auth((0..4).map(|_| AuthResult::Offline).collect());
    let sink = Arc::new(FakeSink::default());
    let h = start(&t, &sink);
    sleep_ms(4_900).await;
    assert!(sink.states.lock().unwrap().is_empty(), "до 5 с плашки нет");
    sleep_ms(300).await;
    assert_eq!(
        last_state(&sink),
        Some(LinkState::Offline {
            server: SERVER.into()
        })
    );
    // Ни одного NeedsLogin за всё время.
    assert!(!sink
        .states
        .lock()
        .unwrap()
        .iter()
        .any(|s| matches!(s, LinkState::NeedsLogin { .. })));
    // Сеть появилась (4 Offline израсходованы): Connected.
    sleep_ms(30_000).await;
    assert_eq!(last_state(&sink), Some(connected_state()));
    h.stop().await;
}

#[tokio::test(start_paused = true)]
async fn loop_revoked_close_wipes_token_and_stops_cycle() {
    let t = MockTransport::new();
    t.script_connects(vec![Ok(vec![hello(), WsFrame::Closed { revoked: true }])]);
    let sink = Arc::new(FakeSink::default());
    let h = start(&t, &sink);
    sleep_ms(60_000).await;
    assert_eq!(t.revoke_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        last_state(&sink),
        Some(LinkState::Revoked {
            server: SERVER.into()
        })
    );
    assert_eq!(t.times().len(), 1, "новых попыток подключения нет");
    assert_eq!(t.auth_calls.load(Ordering::SeqCst), 1, "автовхода нет");
    h.stop().await;
}

#[tokio::test(start_paused = true)]
async fn loop_unauthorized_upgrade_reauths_once() {
    let t = MockTransport::new();
    t.script_connects(vec![Err(WsError::Unauthorized), Ok(vec![hello()])]);
    let sink = Arc::new(FakeSink::default());
    let h = start(&t, &sink);
    sleep_ms(500).await;
    assert_eq!(
        t.auth_calls.load(Ordering::SeqCst),
        2,
        "один повторный вход"
    );
    assert_eq!(t.times().len(), 2);
    assert_eq!(last_state(&sink), Some(connected_state()));
    assert_eq!(
        sink.states.lock().unwrap().len(),
        1,
        "не «нет связи» и не смена экрана"
    );
    h.stop().await;
}

#[tokio::test(start_paused = true)]
async fn loop_retry_now_resets_backoff_during_wait() {
    let t = MockTransport::new();
    t.script_connects((0..6).map(|_| Err(WsError::Network)).collect());
    let sink = Arc::new(FakeSink::default());
    let h = start(&t, &sink);
    // Попытки: 0, 1, 3, 7 с. Ждём внутри паузы 8 с.
    sleep_ms(9_000).await;
    assert_eq!(t.times().len(), 4);
    h.retry_now().await;
    sleep_ms(100).await;
    assert_eq!(t.times().len(), 5, "попытка сразу");
    // Пауза после сброса снова 1 с.
    sleep_ms(1_200).await;
    assert_eq!(t.times().len(), 6);
    h.stop().await;
}

#[tokio::test(start_paused = true)]
async fn loop_stop_ends_task_in_every_phase() {
    // Во время паузы.
    let t = MockTransport::new();
    t.script_connects((0..3).map(|_| Err(WsError::Network)).collect());
    let sink = Arc::new(FakeSink::default());
    let h = start(&t, &sink);
    sleep_ms(500).await;
    h.stop().await;
    // На живом соединении.
    let t = MockTransport::new();
    let h = start(&t, &Arc::new(FakeSink::default()));
    sleep_ms(500).await;
    h.stop().await;
    // В Idle (NeedsLogin).
    let t = MockTransport::new();
    t.script_auth(vec![AuthResult::NeedsLogin {
        reason: None,
        can_retry_sso: false,
        after_logout: false,
    }]);
    let sink = Arc::new(FakeSink::default());
    let h = start(&t, &sink);
    sleep_ms(500).await;
    assert!(matches!(
        last_state(&sink),
        Some(LinkState::NeedsLogin { .. })
    ));
    h.stop().await;
}

#[tokio::test(start_paused = true)]
async fn loop_external_login_after_needs_login_connects() {
    let t = MockTransport::new();
    t.script_auth(vec![AuthResult::NeedsLogin {
        reason: None,
        can_retry_sso: false,
        after_logout: false,
    }]);
    let sink = Arc::new(FakeSink::default());
    let h = start(&t, &sink);
    sleep_ms(30_000).await;
    assert_eq!(t.times().len(), 0, "без входа подключений нет");
    assert_eq!(
        t.auth_calls.load(Ordering::SeqCst),
        1,
        "пароль/авто не повторяются"
    );
    h.command(Command::External(connected_result())).await;
    sleep_ms(500).await;
    assert_eq!(last_state(&sink), Some(connected_state()));
    h.stop().await;
}

// ---------------------------------------------------------------------------------------------
// Интеграция: RealTransport (LinkHttp + пин + WS) против локального TLS-мока (HTTP resume и WS
// на одном порту, как у настоящего сервера). Реальное время, вымышленные имена.
// ---------------------------------------------------------------------------------------------

mod integration {
    use super::*;

    use std::pin::Pin;
    use std::task::{Context, Poll};

    use futures_util::SinkExt;
    use link_app::authflow::Platform;
    use link_app::config::{LinkConfig, LocalState, SourceKind};
    use link_app::connection::RealTransport;
    use link_app::http_client::{DeviceCreds, LinkHttp};
    use link_app::secrets::{login_token_key, InMemoryStore, Secret, TokenStore};
    use link_app::sso::MockSso;
    use link_proto::dto::REVOKED_CLOSE_CODE;
    use link_proto::spki::spki_sha256;
    use rcgen::{CertificateParams, KeyPair};
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadBuf};
    use tokio::net::TcpListener;
    use tokio_rustls::TlsAcceptor;
    use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
    use tokio_tungstenite::tungstenite::protocol::CloseFrame;
    use tokio_tungstenite::tungstenite::Message;

    const RESUME_OK: &str =
        r#"{"user":{"id":1,"login":"ivanov","displayName":"Иванов И.И."},"method":"password"}"#;

    /// Поток с «возвращёнными» уже прочитанными байтами: нужен, чтобы отдать разобранный
    /// HTTP-заголовок WS-рукопожатию.
    struct Pre<S> {
        head: Vec<u8>,
        pos: usize,
        inner: S,
    }

    impl<S: tokio::io::AsyncRead + Unpin> tokio::io::AsyncRead for Pre<S> {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            if self.pos < self.head.len() {
                let n = (self.head.len() - self.pos).min(buf.remaining());
                let start = self.pos;
                buf.put_slice(&self.head[start..start + n]);
                self.pos += n;
                return Poll::Ready(Ok(()));
            }
            Pin::new(&mut self.inner).poll_read(cx, buf)
        }
    }

    impl<S: tokio::io::AsyncWrite + Unpin> tokio::io::AsyncWrite for Pre<S> {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            Pin::new(&mut self.inner).poll_write(cx, buf)
        }
        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.inner).poll_flush(cx)
        }
        fn poll_shutdown(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.inner).poll_shutdown(cx)
        }
    }

    #[derive(Default)]
    struct Mock {
        /// По сценарию на каждое принятое WS-соединение; пусто — hello и тишина.
        ws_scripts: Mutex<VecDeque<Vec<Message>>>,
        requests: AtomicUsize,
        resumes: AtomicUsize,
        upgrades: AtomicUsize,
    }

    fn server_cfg(cert: &[u8], key: &KeyPair) -> Arc<rustls::ServerConfig> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        Arc::new(
            rustls::ServerConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(
                    vec![CertificateDer::from(cert.to_vec())],
                    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der())),
                )
                .unwrap(),
        )
    }

    fn ws_hello() -> Message {
        Message::text(r#"{"type":"hello","protocol":1,"pingIntervalSecs":25,"idleTimeoutSecs":60}"#)
    }

    struct Cert {
        key: KeyPair,
        der: Vec<u8>,
    }

    fn cert() -> Cert {
        let key = KeyPair::generate().unwrap();
        let der = CertificateParams::new(vec!["srv.example.local".to_string()])
            .unwrap()
            .self_signed(&key)
            .unwrap()
            .der()
            .to_vec();
        Cert { key, der }
    }

    async fn spawn_server(c: &Cert, mock: Arc<Mock>) -> u16 {
        let acceptor = TlsAcceptor::from(server_cfg(&c.der, &c.key));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            loop {
                let Ok((tcp, _)) = listener.accept().await else {
                    return;
                };
                let (acceptor, mock) = (acceptor.clone(), mock.clone());
                tokio::spawn(async move {
                    let Ok(mut tls) = acceptor.accept(tcp).await else {
                        return;
                    };
                    let mut buf: Vec<u8> = Vec::new();
                    loop {
                        let mut chunk = [0u8; 2048];
                        let head_end = loop {
                            if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                                break p + 4;
                            }
                            match tls.read(&mut chunk).await {
                                Ok(0) | Err(_) => return,
                                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                            }
                        };
                        mock.requests.fetch_add(1, Ordering::SeqCst);
                        let head = String::from_utf8_lossy(&buf[..head_end]).to_ascii_lowercase();
                        if head.contains("upgrade: websocket") {
                            mock.upgrades.fetch_add(1, Ordering::SeqCst);
                            let script = mock.ws_scripts.lock().unwrap().pop_front();
                            let pre = Pre {
                                head: buf.clone(),
                                pos: 0,
                                inner: tls,
                            };
                            let Ok(mut ws) = tokio_tungstenite::accept_async(pre).await else {
                                return;
                            };
                            let script = script.unwrap_or_else(|| vec![ws_hello()]);
                            let hold = !script.iter().any(|m| matches!(m, Message::Close(_)));
                            for m in script {
                                if ws.send(m).await.is_err() {
                                    return;
                                }
                            }
                            if hold {
                                while futures_util::StreamExt::next(&mut ws).await.is_some() {}
                            } else {
                                let _ = ws.flush().await;
                            }
                            return;
                        }
                        // Обычный HTTP: тело игнорируем (тела resume малы и приходят вместе).
                        let clen = head
                            .lines()
                            .find_map(|l| {
                                l.strip_prefix("content-length:")
                                    .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                            })
                            .unwrap_or(0);
                        while buf.len() < head_end + clen {
                            match tls.read(&mut chunk).await {
                                Ok(0) | Err(_) => return,
                                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                            }
                        }
                        buf.drain(..head_end + clen);
                        mock.resumes.fetch_add(1, Ordering::SeqCst);
                        let resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                            RESUME_OK.len(),
                            RESUME_OK
                        );
                        if tls.write_all(resp.as_bytes()).await.is_err() {
                            return;
                        }
                    }
                });
            }
        });
        port
    }

    struct Rig {
        transport: Arc<RealTransport>,
        tokens: Arc<TokenStore>,
        local: Arc<tokio::sync::Mutex<LocalState>>,
        key: String,
        _tmp: tempfile::TempDir,
    }

    /// `client_pin_of` — чей отпечаток закреплён у клиента (для теста подмены).
    fn rig(port: u16, client_pin_of: &Cert) -> Rig {
        let tmp = tempfile::tempdir().unwrap();
        let local = LocalState::load_or_init(tmp.path()).unwrap();
        let pin = spki_sha256(&client_pin_of.der).unwrap();
        let http = LinkHttp::new(
            "127.0.0.1",
            port,
            pin,
            Some(DeviceCreds {
                device_id: local.device_id.clone(),
                secret: "dev-secret".into(),
            }),
        )
        .unwrap();
        let tokens = Arc::new(TokenStore::new(Some(Box::new(InMemoryStore::new()))));
        let key = login_token_key("127.0.0.1", port);
        tokens.set(&key, &Secret::new("tok"));
        let local = Arc::new(tokio::sync::Mutex::new(local));
        let transport = Arc::new(RealTransport {
            http: Arc::new(http),
            sso: Arc::new(MockSso(Err(link_app::sso::SsoProviderError::NoTicket))),
            tokens: tokens.clone(),
            local: local.clone(),
            cfg: LinkConfig {
                host: "127.0.0.1".into(),
                port,
                pin,
                spn: None,
                source: SourceKind::User,
            },
            platform: Platform::Other,
        });
        Rig {
            transport,
            tokens,
            local,
            key,
            _tmp: tmp,
        }
    }

    fn spawn_sup(r: &Rig, sink: &Arc<FakeSink>) -> SupervisorHandle {
        let mut cfg = SupervisorConfig::new(SERVER, meta());
        cfg.jitter = Arc::new(|| 0.0);
        ConnectionSupervisor::spawn(r.transport.clone(), sink.clone(), cfg)
    }

    async fn wait_for(what: &str, cond: impl Fn() -> bool) {
        for _ in 0..300 {
            if cond() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("не дождались: {what}");
    }

    fn is_connected(sink: &FakeSink) -> bool {
        matches!(last_state(sink), Some(LinkState::Connected { .. }))
    }

    #[tokio::test]
    async fn integration_connects_and_bridges_event() {
        let c = cert();
        let mock = Arc::new(Mock::default());
        mock.ws_scripts.lock().unwrap().push_back(vec![
            ws_hello(),
            Message::text(r#"{"type":"message","id":7}"#),
        ]);
        let port = spawn_server(&c, mock.clone()).await;
        let r = rig(port, &c);
        let sink = Arc::new(FakeSink::default());
        let h = spawn_sup(&r, &sink);
        wait_for("Connected", || is_connected(&sink)).await;
        wait_for("событие", || !sink.events.lock().unwrap().is_empty()).await;
        assert_eq!(sink.events.lock().unwrap()[0]["id"], 7);
        assert_eq!(mock.resumes.load(Ordering::SeqCst), 1);
        assert_eq!(resyncs(&sink), 0);
        h.stop().await;
    }

    #[tokio::test]
    async fn integration_short_drop_is_silent_then_one_resync() {
        let c = cert();
        let mock = Arc::new(Mock::default());
        // Первое соединение: hello и штатный Close (не 4401); второе: hello и тишина.
        mock.ws_scripts.lock().unwrap().push_back(vec![
            ws_hello(),
            Message::Close(Some(CloseFrame {
                code: CloseCode::Away,
                reason: "".into(),
            })),
        ]);
        let port = spawn_server(&c, mock.clone()).await;
        let r = rig(port, &c);
        let sink = Arc::new(FakeSink::default());
        let h = spawn_sup(&r, &sink);
        wait_for("второе подключение", || {
            mock.upgrades.load(Ordering::SeqCst) >= 2
        })
        .await;
        wait_for("ресинк", || resyncs(&sink) == 1).await;
        assert!(is_connected(&sink));
        // Обрыв короче порога: за всё время только одно состояние Connected.
        assert_eq!(sink.states.lock().unwrap().len(), 1);
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(resyncs(&sink), 1);
        h.stop().await;
    }

    #[tokio::test]
    async fn integration_foreign_certificate_is_pin_mismatch_without_any_request() {
        let served = cert();
        let pinned = cert();
        let mock = Arc::new(Mock::default());
        let port = spawn_server(&served, mock.clone()).await;
        let r = rig(port, &pinned);
        let sink = Arc::new(FakeSink::default());
        let h = spawn_sup(&r, &sink);
        wait_for("PinMismatch", || {
            matches!(last_state(&sink), Some(LinkState::PinMismatch { .. }))
        })
        .await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(mock.requests.load(Ordering::SeqCst), 0);
        assert_eq!(mock.upgrades.load(Ordering::SeqCst), 0);
        h.stop().await;
    }

    #[tokio::test]
    async fn integration_close_4401_revokes_and_stops() {
        let c = cert();
        let mock = Arc::new(Mock::default());
        mock.ws_scripts.lock().unwrap().push_back(vec![
            ws_hello(),
            Message::Close(Some(CloseFrame {
                code: CloseCode::from(REVOKED_CLOSE_CODE),
                reason: "".into(),
            })),
        ]);
        let port = spawn_server(&c, mock.clone()).await;
        let r = rig(port, &c);
        let sink = Arc::new(FakeSink::default());
        let h = spawn_sup(&r, &sink);
        wait_for("Revoked", || {
            matches!(last_state(&sink), Some(LinkState::Revoked { .. }))
        })
        .await;
        assert!(r.tokens.get(&r.key).is_none(), "токен стёрт");
        assert!(r.local.lock().await.revoked, "revoked=true на диске");
        // Цикл остановлен: ни новых подключений, ни повторного входа.
        tokio::time::sleep(Duration::from_millis(1_500)).await;
        assert_eq!(mock.upgrades.load(Ordering::SeqCst), 1);
        assert_eq!(mock.resumes.load(Ordering::SeqCst), 1);
        h.stop().await;
    }
}
