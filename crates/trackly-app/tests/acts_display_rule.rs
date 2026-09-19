//! Display-rule pure-Rust tests for `format_act_number` (D-Numbering-01).
//!
//! Эти тесты — копия unit-test'ов из `dto/act.rs::tests` в виде integration-
//! ranged set, поэтому их можно отдельно запускать через `cargo test --test
//! acts_display_rule` (требование плана 03-03, acceptance criteria).
//!
//! Phase 40.2 Plan 06 (NUM-14): `number`/`parent_number` are now `&str`, not
//! `i64` — the display rule itself is unchanged, only the input type widened
//! from "integer" to "any free-text/templated number".

use trackly_app::dto::act::format_act_number;
use trackly_core::domain::acts::ActType;

#[test]
fn format_handover() {
    assert_eq!(
        format_act_number(ActType::Handover, "42", None, None, None),
        "42"
    );
}

#[test]
fn format_single_return() {
    // sibling_count = 1 → suffix без числа: «42в».
    assert_eq!(
        format_act_number(ActType::Return, "999", Some(1), Some("42"), Some(1)),
        "42в"
    );
}

#[test]
fn format_multi_returns() {
    assert_eq!(
        format_act_number(ActType::Return, "999", Some(1), Some("42"), Some(2)),
        "42в1"
    );
    assert_eq!(
        format_act_number(ActType::Return, "1000", Some(2), Some("42"), Some(2)),
        "42в2"
    );
}

#[test]
fn format_retroactive_promotion() {
    // Один и тот же sub_number=1 рендерится по-разному в зависимости от
    // sibling_count: «42в» (один возврат) → «42в1» (после появления второго).
    let one = format_act_number(ActType::Return, "999", Some(1), Some("42"), Some(1));
    let two = format_act_number(ActType::Return, "999", Some(1), Some("42"), Some(2));
    assert_eq!(one, "42в");
    assert_eq!(two, "42в1");
    assert_ne!(one, two, "promotion must change rendering");
}

#[test]
fn format_templated_non_numeric_number() {
    // NUM-14: a templated act number is not guaranteed to parse as an
    // integer (e.g. "2026/09-1") — the display rule must work identically.
    assert_eq!(
        format_act_number(ActType::Handover, "2026/09-1", None, None, None),
        "2026/09-1"
    );
    assert_eq!(
        format_act_number(
            ActType::Return,
            "ignored",
            Some(1),
            Some("2026/09-1"),
            Some(1)
        ),
        "2026/09-1в"
    );
}
