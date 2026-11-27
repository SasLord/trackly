//! Чистые функции переподключения (D-23): задержки, показ «нет связи», детект сна ПК.
//! Значения зеркалят `packages/transport/src/ws.ts` (старт 1 с, удвоение, потолок 30 с,
//! задержка индикации 5 с). Время подаётся аргументами; таймеры подключает автомат (план 15).

use std::time::{Duration, Instant};

pub const OFFLINE_INDICATION_DELAY: Duration = Duration::from_secs(5);
pub const MAX_DELAY: Duration = Duration::from_secs(30);
pub const DEFAULT_SLEEP_THRESHOLD: Duration = Duration::from_secs(15);

#[derive(Debug, Default, Clone)]
pub struct Backoff {
    attempt: u32,
}

impl Backoff {
    pub fn new() -> Self {
        Self::default()
    }

    /// Следующая пауза. `jitter` в [0,1]: множитель 0.8..1.2; результат не выше 30 с.
    pub fn next_delay(&mut self, jitter: f64) -> Duration {
        let base = 1u64 << self.attempt.min(5);
        self.attempt = self.attempt.saturating_add(1);
        let factor = 0.8 + 0.4 * jitter.clamp(0.0, 1.0);
        Duration::from_secs_f64(base as f64 * factor).min(MAX_DELAY)
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
    }
}

/// Показывать ли индикацию «нет связи».
pub fn should_show_offline(since: Instant, now: Instant) -> bool {
    now.saturating_duration_since(since) >= OFFLINE_INDICATION_DELAY
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeSignal {
    Tick,
    Woke,
}

#[derive(Debug, Clone, Copy)]
pub struct SleepDetector {
    pub threshold: Duration,
}

impl Default for SleepDetector {
    fn default() -> Self {
        Self {
            threshold: DEFAULT_SLEEP_THRESHOLD,
        }
    }
}

impl SleepDetector {
    /// `mono` — прошедшее по монотонным часам (в сне не идут), `wall` — по настенным.
    pub fn observe(&self, mono: Duration, wall: Duration) -> WakeSignal {
        if wall.saturating_sub(mono) > self.threshold {
            WakeSignal::Woke
        } else {
            WakeSignal::Tick
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(d: Duration) -> u64 {
        d.as_secs()
    }

    #[test]
    fn sequence_without_jitter() {
        let mut b = Backoff::new();
        let got: Vec<u64> = (0..7).map(|_| secs(b.next_delay(0.5))).collect();
        assert_eq!(got, [1, 2, 4, 8, 16, 30, 30]);
    }

    #[test]
    fn jitter_bounded() {
        for j in [0.0, 0.25, 1.0] {
            let mut b = Backoff::new();
            for _ in 0..10 {
                let d = b.next_delay(j);
                assert!(d <= MAX_DELAY);
            }
        }
        let mut b = Backoff::new();
        let d = b.next_delay(0.0);
        assert!(d >= Duration::from_millis(800) && d <= Duration::from_millis(1200));
        let mut b = Backoff::new();
        b.attempt = 3;
        let d = b.next_delay(1.0);
        assert!(d <= Duration::from_millis(9600));
    }

    #[test]
    fn reset_restarts() {
        let mut b = Backoff::new();
        b.next_delay(0.5);
        b.next_delay(0.5);
        b.reset();
        assert_eq!(secs(b.next_delay(0.5)), 1);
    }

    #[test]
    fn offline_indication() {
        assert_eq!(OFFLINE_INDICATION_DELAY, Duration::from_secs(5));
        let t0 = Instant::now();
        assert!(!should_show_offline(t0, t0 + Duration::from_millis(4900)));
        assert!(should_show_offline(t0, t0 + Duration::from_secs(5)));
    }

    #[test]
    fn sleep_detection() {
        let d = SleepDetector::default();
        assert_eq!(
            d.observe(Duration::from_secs(5), Duration::from_secs(5)),
            WakeSignal::Tick
        );
        assert_eq!(
            d.observe(Duration::from_secs(5), Duration::from_secs(3600)),
            WakeSignal::Woke
        );
    }
}
