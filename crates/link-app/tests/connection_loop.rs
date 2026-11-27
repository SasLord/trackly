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
