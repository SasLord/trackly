//! Автомат связи Link: единый reducer состояний, приёмник событий, транспорт (D-14, D-22..D-25).
//!
//! Часть 1 (чистая): [`reduce`], [`Machine`], [`EventSink`]. Здесь нет tokio и Tauri:
//! время подаётся аргументами, события уходят через `EventSink`. Один владелец состояния:
//! любая смена `LinkState` идёт через `Machine::apply` и ровно один раз сообщается приёмнику.

use std::time::{Duration, Instant};

use crate::authflow::AuthResult;
use crate::backoff::OFFLINE_INDICATION_DELAY;
use crate::state::{LinkEvent, LinkState, LinkUserView, LoginMethod};

/// Вход автомата (UI-SPEC §6.1).
#[derive(Debug, Clone)]
pub enum Input {
    /// Итог потока входа (`AuthFlow`) или внешняя смена (`password_login`).
    AuthDone(AuthResult),
    /// Связь потеряна (или не удалась при старте) в момент `at`.
    ConnectionLost {
        at: Instant,
    },
    ConnectionRestored,
    /// Тик времени: решает, пора ли показать «нет связи».
    Tick {
        now: Instant,
    },
    /// Сервер отозвал вход/устройство (D-14).
    RevokedSignal,
    /// Перепроверка пина: из `PinMismatch` обратно в `Starting`.
    RecheckPin,
    UserLoggedOut,
}

/// Контекст reducer'а: то, что состояние само не хранит.
#[derive(Debug, Clone)]
pub struct ReduceCtx {
    pub server: String,
    /// С какого момента нет связи (для порога «нет связи»).
    pub offline_since: Option<Instant>,
    /// Последний вошедший пользователь: нужен, чтобы `Offline` вернулся в `Connected`.
    pub last_user: Option<(LinkUserView, LoginMethod)>,
    /// Порог индикации «нет связи» (D-22); по умолчанию 5 с, в тестах можно ускорить.
    pub offline_delay: Duration,
}

impl ReduceCtx {
    pub fn new(server: impl Into<String>) -> Self {
        Self {
            server: server.into(),
            offline_since: None,
            last_user: None,
            offline_delay: OFFLINE_INDICATION_DELAY,
        }
    }
}

/// Чистая таблица переходов UI-SPEC §6.1. Без побочных эффектов, кроме `ctx`.
pub fn reduce(state: &LinkState, ctx: &mut ReduceCtx, input: Input) -> LinkState {
    let server = ctx.server.clone();
    match input {
        Input::AuthDone(result) => match result {
            AuthResult::Connected { user, method } => {
                ctx.offline_since = None;
                ctx.last_user = Some((user.clone(), method));
                LinkState::Connected {
                    user,
                    method,
                    server,
                }
            }
            AuthResult::NeedsLogin {
                reason,
                can_retry_sso,
                after_logout,
            } => {
                ctx.offline_since = None;
                LinkState::NeedsLogin {
                    reason,
                    can_retry_sso,
                    after_logout,
                    server,
                }
            }
            AuthResult::Revoked => {
                ctx.offline_since = None;
                LinkState::Revoked { server }
            }
            AuthResult::Offline => LinkState::Offline { server },
            AuthResult::PinMismatch { expected, got } => {
                ctx.offline_since = None;
                LinkState::PinMismatch {
                    server,
                    expected,
                    got,
                }
            }
        },
        // Потеря связи считается только там, где связь вообще предполагалась. PinMismatch,
        // NeedsLogin, Revoked от обрыва не меняются (сеть тут ни при чём).
        Input::ConnectionLost { at } => {
            if matches!(state, LinkState::Connected { .. } | LinkState::Starting) {
                ctx.offline_since.get_or_insert(at);
            }
            state.clone()
        }
        Input::Tick { now } => match (state, ctx.offline_since) {
            (LinkState::Connected { .. } | LinkState::Starting, Some(since))
                if now.saturating_duration_since(since) >= ctx.offline_delay =>
            {
                LinkState::Offline { server }
            }
            _ => state.clone(),
        },
        Input::ConnectionRestored => {
            ctx.offline_since = None;
            match (state, &ctx.last_user) {
                (LinkState::Offline { .. }, Some((user, method))) => LinkState::Connected {
                    user: user.clone(),
                    method: *method,
                    server,
                },
                _ => state.clone(),
            }
        }
        Input::RevokedSignal => {
            ctx.offline_since = None;
            LinkState::Revoked { server }
        }
        Input::RecheckPin => match state {
            LinkState::PinMismatch { .. } => LinkState::Starting,
            _ => state.clone(),
        },
        Input::UserLoggedOut => {
            ctx.offline_since = None;
            LinkState::NeedsLogin {
                reason: None,
                can_retry_sso: false,
                after_logout: true,
                server,
            }
        }
    }
}

/// Приёмник событий. Реальная реализация (план 17): `link-event` / `trackly-event` /
/// `trackly-resync` и обновление трея. Вызывается из задачи автомата, поэтому не блокирует.
pub trait EventSink: Send + Sync + 'static {
    fn state_changed(&self, ev: &LinkEvent);
    /// Событие беседы с WS как есть (на фронте это `trackly-event`).
    fn bridge_event(&self, payload: serde_json::Value);
    /// Тихий «перечитай всё» (`trackly-resync`).
    fn resync(&self);
}

/// Поля `LinkEvent`, не зависящие от состояния.
#[derive(Debug, Clone)]
pub struct EventMeta {
    pub tray_available: bool,
    pub secrets_persistent: bool,
    pub platform: String,
}

/// Владелец состояния: reducer + приёмник, без повторных событий о том же состоянии.
pub struct Machine<S: EventSink> {
    state: LinkState,
    ctx: ReduceCtx,
    sink: std::sync::Arc<S>,
    meta: EventMeta,
}

impl<S: EventSink> Machine<S> {
    pub fn new(
        server: impl Into<String>,
        meta: EventMeta,
        sink: std::sync::Arc<S>,
        offline_delay: Duration,
    ) -> Self {
        let mut ctx = ReduceCtx::new(server);
        ctx.offline_delay = offline_delay;
        Self {
            state: LinkState::Starting,
            ctx,
            sink,
            meta,
        }
    }

    pub fn state(&self) -> &LinkState {
        &self.state
    }

    /// Применить вход; `true`, если состояние сменилось (и приёмник получил событие).
    pub fn apply(&mut self, input: Input) -> bool {
        let next = reduce(&self.state, &mut self.ctx, input);
        if next == self.state {
            return false;
        }
        self.state = next.clone();
        self.sink.state_changed(&LinkEvent {
            state: next,
            tray_available: self.meta.tray_available,
            secrets_persistent: self.meta.secrets_persistent,
            platform: self.meta.platform.clone(),
        });
        true
    }
}

// ============================================================================================
// Часть 2: ConnectionSupervisor. Все таймеры здесь tokio::time (в скрытом окне WebView2
// JS-таймеры не работают), тесты идут на виртуальном времени.
// ============================================================================================

use std::future::Future;
use std::sync::Arc;
use std::time::SystemTime;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{mpsc, Mutex};

use link_proto::dto::LINK_REVOKED;

use crate::authflow::{AuthFlow, Platform};
use crate::backoff::{Backoff, SleepDetector, WakeSignal};
use crate::config::{LinkConfig, LocalState};
use crate::http_client::LinkHttp;
use crate::secrets::TokenStore;
use crate::sso::SsoProvider;
use crate::ws_client::{WsConn, WsError, WsFrame};

/// Путь WS-эндпоинта сервера.
pub const WS_PATH: &str = "/api/v1/ws";

/// Живое WS-соединение (абстракция над `WsConn` для тестов).
pub trait Connection: Send {
    fn next_frame(&mut self) -> impl Future<Output = WsFrame> + Send;
}

impl<S: AsyncRead + AsyncWrite + Unpin + Send> Connection for WsConn<S> {
    fn next_frame(&mut self) -> impl Future<Output = WsFrame> + Send {
        WsConn::next_frame(self)
    }
}

/// Сеть и вход для автомата. В продакшене это [`RealTransport`], в тестах — мок.
pub trait Transport: Send + Sync + 'static {
    type Conn: Connection;
    /// Поток входа без пароля: токен -> SSO (`AuthFlow::startup_auth`).
    fn authenticate(&self) -> impl Future<Output = AuthResult> + Send;
    /// WS-апгрейд по пину с текущей сессией.
    fn connect(&self) -> impl Future<Output = Result<Self::Conn, WsError>> + Send;
    /// Сервер отозвал вход на живом соединении: стереть токен, `revoked=true` (D-14).
    fn revoke(&self) -> impl Future<Output = ()> + Send;
    /// «Выйти» (D-11).
    fn logout(&self) -> impl Future<Output = AuthResult> + Send;
}

/// Боевой транспорт: `AuthFlow` + `LinkHttp` + `ws_client::connect`.
pub struct RealTransport {
    pub http: Arc<LinkHttp>,
    pub sso: Arc<dyn SsoProvider>,
    pub tokens: Arc<TokenStore>,
    pub local: Arc<Mutex<LocalState>>,
    pub cfg: LinkConfig,
    pub platform: Platform,
}

impl Transport for RealTransport {
    type Conn = crate::ws_client::TlsWsConn;

    async fn authenticate(&self) -> AuthResult {
        let mut local = self.local.lock().await;
        let mut flow = AuthFlow {
            http: &self.http,
            sso: &*self.sso,
            tokens: &self.tokens,
            local: &mut local,
            cfg: &self.cfg,
            platform: self.platform,
        };
        flow.startup_auth().await
    }

    async fn connect(&self) -> Result<Self::Conn, WsError> {
        crate::ws_client::connect(&self.http, WS_PATH).await
    }

    async fn revoke(&self) {
        let mut local = self.local.lock().await;
        let mut flow = AuthFlow {
            http: &self.http,
            sso: &*self.sso,
            tokens: &self.tokens,
            local: &mut local,
            cfg: &self.cfg,
            platform: self.platform,
        };
        let _ = flow.on_api_error(LINK_REVOKED);
    }

    async fn logout(&self) -> AuthResult {
        let mut local = self.local.lock().await;
        let mut flow = AuthFlow {
            http: &self.http,
            sso: &*self.sso,
            tokens: &self.tokens,
            local: &mut local,
            cfg: &self.cfg,
            platform: self.platform,
        };
        flow.logout().await
    }
}

/// Команды управления автоматом.
#[derive(Debug, Clone)]
pub enum Command {
    /// «Повторить сейчас»: сбросить паузу и пробовать немедленно (D-23).
    RetryNow,
    /// Перепроверить пин (из `PinMismatch`).
    RecheckPin,
    Logout,
    /// Повторить поток входа с нуля (после смены конфига, enroll и т.п.).
    Restart,
    /// Итог входа, выполненного извне (`password_login`, `retry_sso`).
    External(AuthResult),
    Stop,
}

/// Настройки автомата; значения по умолчанию соответствуют D-22/D-23.
#[derive(Clone)]
pub struct SupervisorConfig {
    pub meta: EventMeta,
    pub server: String,
    /// Порог индикации «нет связи» (D-22), по умолчанию 5 с.
    pub offline_delay: Duration,
    /// Период внутреннего тика (порог offline и детект сна).
    pub tick: Duration,
    /// Источник джиттера [0,1]; подменяется в тестах.
    pub jitter: Arc<dyn Fn() -> f64 + Send + Sync>,
    /// Настенные часы; подменяются в тестах для имитации сна.
    pub wall: Arc<dyn Fn() -> SystemTime + Send + Sync>,
}

impl SupervisorConfig {
    pub fn new(server: impl Into<String>, meta: EventMeta) -> Self {
        Self {
            meta,
            server: server.into(),
            offline_delay: OFFLINE_INDICATION_DELAY,
            tick: Duration::from_secs(1),
            jitter: Arc::new(default_jitter),
            wall: Arc::new(SystemTime::now),
        }
    }
}

/// Дешёвый источник случайности без нового крейта: `RandomState` засеян ОС.
fn default_jitter() -> f64 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u64(0);
    (h.finish() % 10_000) as f64 / 10_000.0
}

fn now_std() -> Instant {
    tokio::time::Instant::now().into_std()
}

/// Что делает автомат дальше.
enum Phase {
    Auth,
    Connect,
    WaitAuth,
    WaitConnect,
    /// Ждём команду: автоматических попыток нет (NeedsLogin, Revoked, PinMismatch).
    Idle,
    Stop,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Идёт попытка: команды и сон её не прерывают.
    Work,
    /// Живое соединение: пробуждение ПК рвёт его.
    Online,
    /// Пауза backoff: прерывается сном и «Повторить сейчас».
    Wait,
}

enum Driven<O> {
    Done(O),
    Cmd(Command),
    Woke,
    Retry,
    Stop,
}

/// Управление запущенным автоматом.
pub struct SupervisorHandle {
    tx: mpsc::Sender<Command>,
    join: tokio::task::JoinHandle<()>,
}

impl SupervisorHandle {
    pub async fn command(&self, cmd: Command) {
        let _ = self.tx.send(cmd).await;
    }

    pub async fn retry_now(&self) {
        self.command(Command::RetryNow).await;
    }

    /// Остановить цикл и дождаться завершения задачи (без висящих задач).
    pub async fn stop(self) {
        let _ = self.tx.send(Command::Stop).await;
        let _ = self.join.await;
    }
}

pub struct ConnectionSupervisor<T: Transport, S: EventSink> {
    t: Arc<T>,
    sink: Arc<S>,
    machine: Machine<S>,
    rx: mpsc::Receiver<Command>,
    backoff: Backoff,
    jitter: Arc<dyn Fn() -> f64 + Send + Sync>,
    wall: Arc<dyn Fn() -> SystemTime + Send + Sync>,
    tick: tokio::time::Interval,
    detector: SleepDetector,
    last_sample: (Instant, SystemTime),
    /// Один проход повторной аутентификации на серию 401 при апгрейде (Pitfall 8).
    reauth_used: bool,
    /// Была ли потеря установленного соединения: после возврата нужен тихий ресинк (D-24).
    had_loss: bool,
}

impl<T: Transport, S: EventSink> ConnectionSupervisor<T, S> {
    pub fn new(
        transport: Arc<T>,
        sink: Arc<S>,
        cfg: SupervisorConfig,
    ) -> (Self, mpsc::Sender<Command>) {
        let (tx, rx) = mpsc::channel(16);
        let mut tick = tokio::time::interval(cfg.tick);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let machine = Machine::new(
            cfg.server.clone(),
            cfg.meta.clone(),
            sink.clone(),
            cfg.offline_delay,
        );
        let wall_now = (cfg.wall)();
        (
            Self {
                t: transport,
                sink,
                machine,
                rx,
                backoff: Backoff::new(),
                jitter: cfg.jitter,
                wall: cfg.wall,
                tick,
                detector: SleepDetector::default(),
                last_sample: (now_std(), wall_now),
                reauth_used: false,
                had_loss: false,
            },
            tx,
        )
    }

    /// Запустить задачу автомата.
    pub fn spawn(transport: Arc<T>, sink: Arc<S>, cfg: SupervisorConfig) -> SupervisorHandle {
        let (sup, tx) = Self::new(transport, sink, cfg);
        let join = tokio::spawn(sup.run());
        SupervisorHandle { tx, join }
    }

    /// Главный цикл. Завершается по `Stop` и когда все отправители команд закрыты.
    pub async fn run(mut self) {
        let mut phase = Phase::Auth;
        loop {
            phase = match phase {
                Phase::Stop => break,
                Phase::Idle => self.idle().await,
                Phase::Auth => self.phase_auth().await,
                Phase::Connect => self.phase_connect().await,
                Phase::WaitAuth => self.wait(Phase::Auth).await,
                Phase::WaitConnect => self.wait(Phase::Connect).await,
            };
        }
    }

    /// Тик: порог offline и детект сна. `true` — ПК проснулся.
    fn on_tick(&mut self) -> bool {
        let now = now_std();
        self.machine.apply(Input::Tick { now });
        let wall_now = (self.wall)();
        let (last_mono, last_wall) = self.last_sample;
        let mono = now.saturating_duration_since(last_mono);
        let wall = wall_now.duration_since(last_wall).unwrap_or(Duration::ZERO);
        self.last_sample = (now, wall_now);
        if self.detector.observe(mono, wall) == WakeSignal::Woke {
            self.backoff.reset();
            true
        } else {
            false
        }
    }

    async fn drive<F: Future>(&mut self, fut: F, mode: Mode) -> Driven<F::Output> {
        tokio::pin!(fut);
        loop {
            tokio::select! {
                out = &mut fut => return Driven::Done(out),
                _ = self.tick.tick() => {
                    if self.on_tick() && mode != Mode::Work {
                        return Driven::Woke;
                    }
                }
                cmd = self.rx.recv() => match cmd {
                    None | Some(Command::Stop) => return Driven::Stop,
                    Some(Command::RetryNow) => {
                        self.backoff.reset();
                        if mode == Mode::Wait {
                            return Driven::Retry;
                        }
                    }
                    Some(c) => return Driven::Cmd(c),
                },
            }
        }
    }

    async fn on_command(&mut self, cmd: Command) -> Phase {
        match cmd {
            Command::Stop => Phase::Stop,
            Command::Restart | Command::RetryNow => {
                self.backoff.reset();
                Phase::Auth
            }
            Command::RecheckPin => {
                self.machine.apply(Input::RecheckPin);
                self.backoff.reset();
                Phase::Auth
            }
            Command::Logout => {
                let t = self.t.clone();
                let r = t.logout().await;
                self.machine.apply(Input::AuthDone(r));
                Phase::Idle
            }
            Command::External(r) => self.on_auth_result(r, now_std()),
        }
    }

    fn on_auth_result(&mut self, r: AuthResult, started: Instant) -> Phase {
        match r {
            // D-25: нет связи при старте или возврате — тихо, форма пароля не показывается.
            AuthResult::Offline => {
                self.machine.apply(Input::ConnectionLost { at: started });
                Phase::WaitAuth
            }
            AuthResult::Connected { .. } => {
                self.machine.apply(Input::AuthDone(r));
                Phase::Connect
            }
            other => {
                self.machine.apply(Input::AuthDone(other));
                Phase::Idle
            }
        }
    }

    async fn idle(&mut self) -> Phase {
        loop {
            match self.rx.recv().await {
                None | Some(Command::Stop) => return Phase::Stop,
                Some(Command::RetryNow) => continue,
                Some(c) => return self.on_command(c).await,
            }
        }
    }

    async fn wait(&mut self, next: Phase) -> Phase {
        let delay = self.backoff.next_delay((self.jitter)());
        match self.drive(tokio::time::sleep(delay), Mode::Wait).await {
            Driven::Done(()) | Driven::Woke | Driven::Retry => next,
            Driven::Cmd(c) => self.on_command(c).await,
            Driven::Stop => Phase::Stop,
        }
    }

    async fn phase_auth(&mut self) -> Phase {
        let started = now_std();
        let t = self.t.clone();
        match self.drive(t.authenticate(), Mode::Work).await {
            Driven::Done(r) => self.on_auth_result(r, started),
            Driven::Cmd(c) => self.on_command(c).await,
            Driven::Stop => Phase::Stop,
            Driven::Woke | Driven::Retry => Phase::Auth,
        }
    }

    async fn phase_connect(&mut self) -> Phase {
        let started = now_std();
        let t = self.t.clone();
        match self.drive(t.connect(), Mode::Work).await {
            Driven::Done(Ok(conn)) => self.online(conn).await,
            Driven::Done(Err(WsError::Unauthorized)) => self.reauth(started).await,
            Driven::Done(Err(WsError::PinMismatch { expected, got })) => {
                self.machine
                    .apply(Input::AuthDone(AuthResult::PinMismatch { expected, got }));
                Phase::Idle
            }
            Driven::Done(Err(WsError::Network)) => {
                self.machine.apply(Input::ConnectionLost { at: started });
                Phase::WaitConnect
            }
            Driven::Cmd(c) => self.on_command(c).await,
            Driven::Stop => Phase::Stop,
            Driven::Woke | Driven::Retry => Phase::Connect,
        }
    }

    /// 401 на апгрейде: сессия истекла. Один проход resume/SSO и одна повторная попытка;
    /// это не «нет связи».
    async fn reauth(&mut self, started: Instant) -> Phase {
        if self.reauth_used {
            self.machine.apply(Input::ConnectionLost { at: started });
            return Phase::WaitConnect;
        }
        self.reauth_used = true;
        self.phase_auth().await
    }

    async fn online(&mut self, mut conn: T::Conn) -> Phase {
        self.reauth_used = false;
        self.machine.apply(Input::ConnectionRestored);
        if self.had_loss {
            // D-24: после возврата связи тихий ресинк (механизм Фазы 43).
            self.had_loss = false;
            self.sink.resync();
        }
        loop {
            match self.drive(conn.next_frame(), Mode::Online).await {
                Driven::Done(WsFrame::Event(v)) => self.sink.bridge_event(v),
                Driven::Done(WsFrame::ResyncRequired) => self.sink.resync(),
                // Сервер заговорил: связь здорова, пауза переподключения начинается заново.
                Driven::Done(WsFrame::Hello { .. }) => self.backoff.reset(),
                Driven::Done(WsFrame::Closed { revoked: true }) => {
                    // D-14: токен стереть, флаг поставить, цикл остановить.
                    let t = self.t.clone();
                    t.revoke().await;
                    self.machine.apply(Input::RevokedSignal);
                    return Phase::Idle;
                }
                Driven::Done(WsFrame::Closed { revoked: false } | WsFrame::Dead) => {
                    self.lost_connection();
                    return Phase::WaitConnect;
                }
                // ПК проснулся: соединение, скорее всего, мёртвое. Пробуем сразу.
                Driven::Woke => {
                    self.lost_connection();
                    return Phase::Connect;
                }
                Driven::Retry => {}
                Driven::Cmd(c) => return self.on_command(c).await,
                Driven::Stop => return Phase::Stop,
            }
        }
    }

    fn lost_connection(&mut self) {
        self.had_loss = true;
        self.machine.apply(Input::ConnectionLost { at: now_std() });
    }
}
