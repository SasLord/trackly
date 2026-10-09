//! Display-rule pure-Rust tests for `format_act_number` (Phase 40.5, D-01).
//!
//! Эти тесты — копия unit-test'ов из `dto/act.rs::tests` в виде integration-
//! ranged set, поэтому их можно отдельно запускать через `cargo test --test
//! acts_display_rule` (требование плана 03-03, acceptance criteria).
//!
//! Phase 40.2 Plan 06 (NUM-14): `number`/`parent_number` are now `&str`, not
//! `i64` — the display rule itself is unchanged, only the input type widened
//! from "integer" to "any free-text/templated number".
//!
//! Набор — сознательная копия юнит-тестов `dto/act.rs::tests` для отдельного
//! запуска. Тавтологичный кейс «стабильность при соседях» (сравнение вызова
//! с самим собой) снят (WR-04/D-20): инвариант доказывается сигнатурой
//! функции и ассертом на поверхности таймлайна (`place_movements_timeline`).

use trackly_app::dto::act::format_act_number;
use trackly_core::domain::acts::ActType;

#[test]
fn format_handover() {
    assert_eq!(format_act_number(ActType::Handover, "42", None, None), "42");
}

#[test]
fn format_single_return_uses_v1() {
    assert_eq!(
        format_act_number(ActType::Return, "999", Some(1), Some("42")),
        "42в1"
    );
}

#[test]
fn format_multi_returns() {
    assert_eq!(
        format_act_number(ActType::Return, "999", Some(1), Some("42")),
        "42в1"
    );
    assert_eq!(
        format_act_number(ActType::Return, "1000", Some(2), Some("42")),
        "42в2"
    );
}

#[test]
fn format_templated_non_numeric_number() {
    // NUM-14: a templated act number is not guaranteed to parse as an
    // integer (e.g. "2026/09-1") — the display rule must work identically.
    assert_eq!(
        format_act_number(ActType::Handover, "2026/09-1", None, None),
        "2026/09-1"
    );
    assert_eq!(
        format_act_number(ActType::Return, "ignored", Some(1), Some("2026/09-1")),
        "2026/09-1в1"
    );
}

#[test]
fn format_return_of_parent_with_literal_v_in_mask() {
    // NUM-14 / F7: литеральная «в» внутри маски родителя не путается с
    // суффиксом возврата; ожидание — литерал, а не копия формулы (WR-04/D-20).
    assert_eq!(
        format_act_number(ActType::Return, "ignored", Some(1), Some("АКТв-2026/09-1")),
        "АКТв-2026/09-1в1"
    );
}
