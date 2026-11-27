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
