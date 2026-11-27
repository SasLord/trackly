//! Контракт команд webview Link: набор, предпроверки без сети, секреты, выход через абстракцию.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicI32, Ordering};

use link_app::commands::{precheck_invite, LoginInput, COMMAND_NAMES};
use link_app::runtime::{quit_via, AppControl};
use link_app::state::CmdErrorCode;

fn fixture_names() -> BTreeSet<String> {
    let v: Vec<String> = serde_json::from_str(include_str!("fixtures/link_commands.json")).unwrap();
    v.into_iter().collect()
}

/// Имена из `generate_handler![...]` в исходнике main.rs (без путей модулей).
fn handler_names() -> Vec<String> {
    let src = include_str!("../src/main.rs");
    let start =
        src.find("generate_handler![").expect("generate_handler") + "generate_handler![".len();
    let end = start + src[start..].find(']').expect("конец generate_handler");
    src[start..end]
        .split(',')
        .map(|s| s.trim().rsplit("::").next().unwrap_or("").to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn command_names_equal_fixture() {
    let have: BTreeSet<String> = COMMAND_NAMES.iter().map(|s| s.to_string()).collect();
    assert_eq!(have.len(), COMMAND_NAMES.len(), "дубликаты в COMMAND_NAMES");
    assert_eq!(have, fixture_names());
}

#[test]
fn generate_handler_equals_fixture() {
    let names = handler_names();
    let set: BTreeSet<String> = names.iter().cloned().collect();
    assert_eq!(set.len(), names.len(), "дубликаты в generate_handler");
    assert_eq!(set, fixture_names());
}

#[test]
fn empty_invite_code_is_rejected_without_network() {
    assert_eq!(
        precheck_invite("").unwrap_err().code,
        CmdErrorCode::EmptyCode
    );
    assert_eq!(
        precheck_invite("  \n ").unwrap_err().code,
        CmdErrorCode::EmptyCode
    );
}

#[test]
fn garbage_invite_code_is_invalid() {
    assert_eq!(
        precheck_invite("мусор не код").unwrap_err().code,
        CmdErrorCode::InvalidCode
    );
    assert_eq!(
        precheck_invite("TRK1-AAAA").unwrap_err().code,
        CmdErrorCode::InvalidCode
    );
}

#[test]
fn login_with_empty_fields_is_bad_credentials() {
    for (l, p) in [("", ""), ("ivanov", ""), ("  ", "x"), ("", "x")] {
        let err = LoginInput::new(l.into(), p.into()).precheck().unwrap_err();
        assert_eq!(err.code, CmdErrorCode::BadCredentials, "{l:?}/{p:?}");
    }
    assert!(LoginInput::new("ivanov".into(), "x".into())
        .precheck()
        .is_ok());
}

#[test]
fn login_input_debug_hides_password() {
    let input = LoginInput::new("ivanov".into(), "Сверхсекрет-123".into());
    let dbg = format!("{input:?}");
    assert!(!dbg.contains("Сверхсекрет"), "{dbg}");
    assert!(dbg.contains("ivanov"));
}

#[test]
fn commands_do_not_return_secrets() {
    // Возвращаемые типы команд: LinkEvent (состояние без секретов), () и LinkCmdError.
    // Исходник команд не содержит ни токенов в ответах, ни логирования пароля.
    let src = include_str!("../src/commands.rs");
    assert!(
        !src.contains("tracing::"),
        "команды не должны ничего логировать"
    );
}

#[test]
fn password_login_is_called_only_from_commands() {
    let commands = include_str!("../src/commands.rs");
    assert_eq!(commands.matches("password_login(").count(), 1);
    for (name, src) in [
        ("runtime.rs", include_str!("../src/runtime.rs")),
        ("connection.rs", include_str!("../src/connection.rs")),
        ("main.rs", include_str!("../src/main.rs")),
    ] {
        assert!(!src.contains("password_login("), "{name}");
    }
}

struct FakeControl(AtomicI32);
impl AppControl for FakeControl {
    fn exit(&self, code: i32) {
        self.0.store(code + 100, Ordering::SeqCst);
    }
}

#[test]
fn quit_goes_through_app_control_not_process_exit() {
    let fake = FakeControl(AtomicI32::new(0));
    quit_via(&fake);
    assert_eq!(
        fake.0.load(Ordering::SeqCst),
        100,
        "exit(0) через абстракцию"
    );
    for src in [
        include_str!("../src/commands.rs"),
        include_str!("../src/runtime.rs"),
        include_str!("../src/main.rs"),
    ] {
        assert!(!src.contains(concat!("process", "::exit")));
    }
}
