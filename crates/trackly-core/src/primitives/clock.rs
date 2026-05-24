//! `Clock` — абстракция «текущее UTC-время».
//!
//! Доменный код принимает `Arc<dyn Clock + Send + Sync>` вместо прямого вызова
//! `time::OffsetDateTime::now_utc()`, чтобы:
//! 1. Тесты могли подменить часы фиксированным значением (определимость).
//! 2. `trackly-core` остался I/O-free (никаких syscall'ов в core).
//!
//! Production impl — `SystemClock` в `trackly_infra::clock_impl`.
//!
//! `chrono::Local::now` запрещён клиппи через `[workspace.lints.clippy]`
//! `disallowed-methods` — в БД всё в UTC (D-Time-01, Pitfall #15).

use time::OffsetDateTime;

/// Источник текущего времени. Реализация ОБЯЗАНА возвращать UTC.
pub trait Clock: Send + Sync {
    /// Текущее UTC-время.
    fn now(&self) -> OffsetDateTime;

    /// Unix epoch seconds (i64). Дефолтная реализация — вызов [`Clock::now`]
    /// и перевод в секунды. Может быть переопределена ради эффективности.
    fn unix_seconds(&self) -> i64 {
        self.now().unix_timestamp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    /// Мок-часы — фиксированное время. Доказывает dyn-совместимость трейта.
    struct FixedClock(OffsetDateTime);

    impl Clock for FixedClock {
        fn now(&self) -> OffsetDateTime {
            self.0
        }
    }

    #[test]
    fn unix_seconds_consistent_with_now() {
        let fixed = datetime!(2026-05-25 12:00:00 UTC);
        let clock: Box<dyn Clock> = Box::new(FixedClock(fixed));
        assert_eq!(clock.now(), fixed);
        assert_eq!(clock.unix_seconds(), fixed.unix_timestamp());
    }

    #[test]
    fn trait_is_dyn_compatible() {
        // Если этот тест компилируется — трейт object-safe (dyn-compatible).
        let _: Box<dyn Clock + Send + Sync> =
            Box::new(FixedClock(datetime!(2026-01-01 00:00:00 UTC)));
    }
}
