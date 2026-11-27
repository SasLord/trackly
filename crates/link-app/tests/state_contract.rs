//! Golden-контракт состояний и команд Link (интерфейс для link-ui).

use link_app::state::*;
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/link_state.json")).unwrap()
}

fn samples() -> Vec<LinkState> {
    let s = "trackly.example.local".to_string();
    vec![
        LinkState::Starting,
        LinkState::NeedsSetup,
        LinkState::PinMismatch {
            server: s.clone(),
            expected: "AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99".into(),
            got: "99:88:77:66:55:44:33:22:11:00:FF:EE:DD:CC:BB:AA:99:88:77:66:55:44:33:22:11:00:FF:EE:DD:CC:BB:AA".into(),
        },
        LinkState::NeedsLogin {
            reason: Some(LoginReason::Kerberos),
            can_retry_sso: true,
            after_logout: false,
            server: s.clone(),
        },
        LinkState::Revoked { server: s.clone() },
        LinkState::Connected {
            user: LinkUserView {
                display_name: "Иванов И.И.".into(),
                login: "ivanov".into(),
            },
            method: LoginMethod::Sso,
            server: s.clone(),
        },
        LinkState::Offline { server: s },
    ]
}

/// Исчерпывающий match: новый вариант без обновления контракта не соберётся.
fn kind_of(st: &LinkState) -> &'static str {
    match st {
        LinkState::Starting => "starting",
        LinkState::NeedsSetup => "needs_setup",
        LinkState::PinMismatch { .. } => "pin_mismatch",
        LinkState::NeedsLogin { .. } => "needs_login",
        LinkState::Revoked { .. } => "revoked",
        LinkState::Connected { .. } => "connected",
        LinkState::Offline { .. } => "offline",
    }
}

#[test]
fn every_state_matches_golden() {
    let fx = fixture();
    let states = fx["states"].as_object().unwrap();
    for st in samples() {
        let k = kind_of(&st);
        assert_eq!(serde_json::to_value(&st).unwrap(), states[k], "kind {k}");
        let back: LinkState = serde_json::from_value(states[k].clone()).unwrap();
        assert_eq!(back, st);
    }
}

#[test]
fn fixture_kinds_equal_enum_kinds() {
    let fx = fixture();
    let mut fx_kinds: Vec<_> = fx["states"].as_object().unwrap().keys().cloned().collect();
    let mut enum_kinds: Vec<_> = samples().iter().map(|s| kind_of(s).to_string()).collect();
    fx_kinds.sort();
    enum_kinds.sort();
    assert_eq!(fx_kinds, enum_kinds);
}

#[test]
fn event_matches_golden() {
    let ev = LinkEvent {
        state: LinkState::Offline {
            server: "trackly.example.local".into(),
        },
        tray_available: true,
        secrets_persistent: false,
        platform: "windows".into(),
    };
    assert_eq!(serde_json::to_value(&ev).unwrap(), fixture()["event"]);
    assert_eq!(LINK_EVENT, "link-event");
}

#[test]
fn commands_fixture_has_nine_names() {
    let v: Vec<String> = serde_json::from_str(include_str!("fixtures/link_commands.json")).unwrap();
    assert_eq!(v.len(), 9);
    assert!(v.contains(&"link_info".to_string()));
}

#[test]
fn error_codes_serialize_snake_case_with_russian_message() {
    let e = LinkCmdError::with_server(CmdErrorCode::ServerUnreachable, "trackly.example.local");
    let v = serde_json::to_value(&e).unwrap();
    assert_eq!(v["code"], "server_unreachable");
    assert!(v["message"]
        .as_str()
        .unwrap()
        .contains("trackly.example.local"));
    assert_eq!(
        serde_json::to_value(LinkCmdError::new(CmdErrorCode::EmptyCode)).unwrap()["message"],
        "Введите код приглашения"
    );
}
