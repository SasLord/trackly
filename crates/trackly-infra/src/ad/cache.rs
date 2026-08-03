//! `TtlCache<V>` — small hand-rolled TTL cache (no `moka`/`cached` crate).
//!
//! Per RESEARCH's "Don't Hand-Roll" table, this ONE data structure IS meant
//! to be hand-rolled — LAN/~20-user scale, single map, no eviction policy
//! needed (mirrors `ReaderPool`'s "small hand-rolled `Mutex`-guarded
//! primitive, no external crate" convention in `crates/trackly-infra/src/db/pools.rs`).
//!
//! Used by `RealAdDirectory` (Plan 31-02) as TWO SEPARATE instances — one
//! for `display_name: TtlCache<String>` (long TTL, e.g. 30 min, low security
//! stakes) and one for `role: TtlCache<Option<Role>>` (short TTL, e.g. 5 min,
//! direct authorization impact) — the simpler two-instance approach matches
//! the "small hand-rolled primitive, no cleverness" philosophy better than a
//! single fixed-shape entry struct (RESEARCH Open Question 2).
//!
//! Cache key MUST be the normalized login (Pitfall 3) — callers are
//! responsible for normalizing (e.g. via the same logic as
//! `MockAdDirectory::lookup_key`/`RealAdClient::normalize_bind_name`) before
//! calling `get`/`put`.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Generic TTL-bounded in-memory cache. `Instant` (monotonic, process-local)
/// is correct here — NOT `time::OffsetDateTime`/UTC (that's reserved for
/// wall-clock DB persistence elsewhere in this codebase; this cache is
/// purely ephemeral in-process state).
pub struct TtlCache<V: Clone> {
    entries: Mutex<HashMap<String, (V, Instant)>>,
    ttl: Duration,
}

impl<V: Clone> TtlCache<V> {
    /// Create a new cache with a fixed TTL applied to every `put` entry.
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    /// Look up `key`. Returns `None` if never `put`, or if the entry's TTL
    /// has expired — expired-but-still-present entries are simply not
    /// returned, no eager eviction needed at this scale.
    pub fn get(&self, key: &str) -> Option<V> {
        let map = self.entries.lock().expect("TtlCache mutex poisoned");
        map.get(key)
            .filter(|(_, expires_at)| *expires_at > Instant::now())
            .map(|(value, _)| value.clone())
    }

    /// Insert or overwrite `key` with `value`, resetting its expiry to
    /// `now + ttl`.
    pub fn put(&self, key: String, value: V) {
        let mut map = self.entries.lock().expect("TtlCache mutex poisoned");
        map.insert(key, (value, Instant::now() + self.ttl));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_then_get_returns_value() {
        let cache: TtlCache<String> = TtlCache::new(Duration::from_secs(60));
        cache.put("us100".to_string(), "Иванов Иван Иванович".to_string());
        assert_eq!(cache.get("us100"), Some("Иванов Иван Иванович".to_string()));
    }

    #[test]
    fn get_on_empty_cache_returns_none() {
        let cache: TtlCache<String> = TtlCache::new(Duration::from_secs(60));
        assert_eq!(cache.get("us200"), None);
    }

    #[test]
    fn entry_expires_after_ttl() {
        let cache: TtlCache<String> = TtlCache::new(Duration::from_millis(10));
        cache.put("us100".to_string(), "Иванов Иван Иванович".to_string());
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(cache.get("us100"), None);
    }

    #[test]
    fn distinct_keys_do_not_interfere() {
        let cache: TtlCache<i32> = TtlCache::new(Duration::from_secs(60));
        cache.put("a".to_string(), 1);
        cache.put("b".to_string(), 2);
        assert_eq!(cache.get("a"), Some(1));
        assert_eq!(cache.get("b"), Some(2));
    }
}
