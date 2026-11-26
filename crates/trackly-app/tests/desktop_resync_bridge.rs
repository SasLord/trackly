//! Мост десктопа при `Lagged` эмитит `trackly-resync` (Phase 43, D-07).
//! (а) юнит: `resync_on_lagged` зовёт emit ровно один раз;
//! (б) исходный гейт: ветка `Lagged` в main.rs вызывает `resync_on_lagged`
//! и не содержит старого «continuing» (main.rs без теста невидим аудиту).

use trackly_app::desktop_event_bridge::{resync_on_lagged, RESYNC_EVENT};

#[test]
fn resync_on_lagged_emits_exactly_once() {
    for n in [0u64, 1, u64::MAX] {
        let mut calls = 0;
        resync_on_lagged(n, || calls += 1);
        assert_eq!(calls, 1, "n = {n}");
    }
    assert_eq!(RESYNC_EVENT, "trackly-resync");
}

#[test]
fn main_rs_lagged_branch_calls_resync() {
    let src = include_str!("../src/main.rs");
    let anchor = "RecvError::Lagged(";
    assert_eq!(
        src.matches(anchor).count(),
        1,
        "якорь должен быть уникален в main.rs"
    );
    let start = src.find(anchor).unwrap();
    // Тело ветки — до следующей ветки `Closed`.
    let end = start
        + src[start..]
            .find("RecvError::Closed")
            .expect("ветка Closed");
    let body = &src[start..end];
    assert!(
        body.contains("desktop_event_bridge::resync_on_lagged("),
        "ветка Lagged не зовёт resync_on_lagged: {body}"
    );
    assert!(
        body.contains("RESYNC_EVENT"),
        "ветка Lagged не эмитит RESYNC_EVENT: {body}"
    );
    assert!(
        !body.contains("continuing"),
        "остался молчаливый continuing"
    );
}
