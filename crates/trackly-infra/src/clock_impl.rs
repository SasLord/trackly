//! `SystemClock` — production-impl `Clock`, возвращающий `OffsetDateTime::now_utc()`.
//!
//! Живёт в `trackly-infra` (не в `trackly-core`), потому что вызов системного
//! времени — это I/O (syscall `clock_gettime` / `GetSystemTimePreciseAsFileTime`),
//! а `trackly-core` обязан оставаться I/O-free (`tests/no_io_deps.rs`).
//!
//! `chrono::Local::now` запрещён клиппи через `[workspace.lints.clippy]`;
//! здесь используется `time::OffsetDateTime::now_utc()` — единственная
//! легитимная точка чтения часов в проде (D-Time-01, Pitfall #15).

use time::OffsetDateTime;
use trackly_core::primitives::clock::Clock;

/// Production impl `Clock`. Stateless — можно держать как `Arc<SystemClock>`
/// или `Arc<dyn Clock + Send + Sync>` (любой вариант имеет нулевой оверхед).
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_is_within_one_second_of_now_utc() {
        let clock = SystemClock;
        let direct = OffsetDateTime::now_utc();
        let through_trait = clock.now();
        let delta = (through_trait - direct).whole_seconds().abs();
        assert!(delta <= 1, "SystemClock.now() drifted from now_utc(): {delta}s");
    }

    #[test]
    fn unix_seconds_default_impl_works() {
        let clock = SystemClock;
        let s = clock.unix_seconds();
        // Должен быть в адекватном диапазоне (после 2026-01-01, до 2100).
        assert!(s > 1_767_225_600, "unix_seconds suspiciously small: {s}");
        assert!(s < 4_102_444_800, "unix_seconds suspiciously large: {s}");
    }
}
