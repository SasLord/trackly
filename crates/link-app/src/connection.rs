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
