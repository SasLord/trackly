//! Integration tests locking the `Secret<T>` security contract (FOUND-06).
//!
//! Three guarantees:
//! 1. `Debug` never leaks the inner value (replaced with `"***"`).
//! 2. `Secret<T>` does NOT implement `serde::Serialize` (compile-time gate).
//! 3. `Drop` calls `zeroize()` on the inner value (behavioural / smoke check).

use trackly_core::Secret;

// COMPILE-TIME GATE: `Secret<String>` must NOT impl `serde::Serialize`.
// Если кто-то добавит `#[derive(Serialize)]` на `Secret`, эта проверка упадёт
// на этапе компиляции, а не в рантайме — Pitfall #5 mitigated structurally.
static_assertions::assert_not_impl_all!(Secret<String>: serde::Serialize);
static_assertions::assert_not_impl_all!(Secret<Vec<u8>>: serde::Serialize);

#[test]
fn debug_does_not_leak_string_value() {
    let s = Secret::new(String::from("hunter2"));
    let dbg = format!("{s:?}");
    assert_eq!(dbg, "***");
    assert!(!dbg.contains("hunter2"));
}

#[test]
fn debug_inside_vec_hides_every_value() {
    let v: Vec<Secret<String>> = (0..5)
        .map(|_| Secret::new(String::from("hunter2")))
        .collect();
    let dbg = format!("{v:?}");
    assert!(!dbg.contains("hunter2"));
    assert_eq!(dbg.matches("***").count(), 5);
}

#[test]
fn expose_returns_original_until_drop() {
    let s = Secret::new(String::from("password123"));
    assert_eq!(s.expose(), "password123");
    // Drop happens here; subsequent ABI is moot — we cannot observe the
    // post-drop memory safely from Rust. The compile-time `Zeroize` bound and
    // the manual `impl Drop` calling `.zeroize()` form the contract;
    // `zeroize`'s own test suite proves the memory is actually zeroed.
}
