//! Display-rule pure-Rust tests for `format_act_number` (D-Numbering-01).
//!
//! Эти тесты — копия unit-test'ов из `dto/act.rs::tests` в виде integration-
//! ranged set, поэтому их можно отдельно запускать через `cargo test --test
//! acts_display_rule` (требование плана 03-03, acceptance criteria).

use trackly_app::dto::act::format_act_number;
use trackly_core::domain::acts::ActType;

#[test]
fn format_handover() {
    assert_eq!(
        format_act_number(ActType::Handover, 42, None, None, None),
        "42"
    );
}

#[test]
fn format_single_return() {
    // sibling_count = 1 → suffix без числа: «42в».
    assert_eq!(
        format_act_number(ActType::Return, 999, Some(1), Some(42), Some(1)),
        "42в"
    );
}

#[test]
fn format_multi_returns() {
    assert_eq!(
        format_act_number(ActType::Return, 999, Some(1), Some(42), Some(2)),
        "42в1"
    );
    assert_eq!(
        format_act_number(ActType::Return, 1000, Some(2), Some(42), Some(2)),
        "42в2"
    );
}

#[test]
fn format_retroactive_promotion() {
    // Один и тот же sub_number=1 рендерится по-разному в зависимости от
    // sibling_count: «42в» (один возврат) → «42в1» (после появления второго).
    let one = format_act_number(ActType::Return, 999, Some(1), Some(42), Some(1));
    let two = format_act_number(ActType::Return, 999, Some(1), Some(42), Some(2));
    assert_eq!(one, "42в");
    assert_eq!(two, "42в1");
    assert_ne!(one, two, "promotion must change rendering");
}
