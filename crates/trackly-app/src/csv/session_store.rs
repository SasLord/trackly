//! In-memory CSV import session store (preview-then-commit token pattern).
//!
//! A CSV import is a two-phase operation:
//! 1. **Preview**: upload file → backend decodes + parses → returns token + preview rows.
//! 2. **Commit**: UI sends `(token, column_mapping)` → backend looks up full decoded data,
//!    runs inserts, returns `CsvImportReport`.
//!
//! Sessions expire after 5 minutes (TTL). Expired entries are swept lazily on `put`.
//! This avoids a background task while still bounding memory use.
//!
//! Design note (RESEARCH §Pattern 7):
//! - `take` (not `get`): single-use token. If the user reloads the preview without
//!   committing, they get a new preview + new token; the old one stays in the map
//!   until the next `put` triggers a lazy sweep.
//! - Lives as `Arc<ImportSessionStore>` inside `DeviceService`. No other code reads it.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// A decoded CSV file held in memory during the preview→commit window.
pub struct ImportSession {
    /// Detected encoding label (e.g. "UTF-8", "windows-1251").
    pub encoding: &'static encoding_rs::Encoding,
    /// Detected delimiter byte (b',' or b';').
    pub delimiter: u8,
    /// CSV header column names.
    pub headers: Vec<String>,
    /// Full decoded rows (all columns). Kept for the commit step.
    pub all_rows: Vec<Vec<String>>,
    /// Wall-clock time when the session was created (for TTL check).
    pub created: Instant,
}

const TTL: Duration = Duration::from_secs(5 * 60);

/// Thread-safe in-memory store for CSV import sessions.
pub struct ImportSessionStore {
    inner: Mutex<HashMap<Uuid, ImportSession>>,
}

impl ImportSessionStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Store `session` and return a new unique token (UUID v4).
    ///
    /// Lazily sweeps expired entries before inserting the new session,
    /// so the map never accumulates unbounded stale data.
    pub fn put(&self, session: ImportSession) -> Uuid {
        let token = Uuid::new_v4();
        let mut g = self.inner.lock().expect("ImportSessionStore mutex poisoned");
        // Lazy sweep — remove all expired entries on every put.
        let now = Instant::now();
        g.retain(|_, s| now.duration_since(s.created) < TTL);
        g.insert(token, session);
        token
    }

    /// Consume and return the session for `token`, or `None` if absent or expired.
    ///
    /// The token is removed from the store regardless (single-use semantics).
    pub fn take(&self, token: Uuid) -> Option<ImportSession> {
        let mut g = self.inner.lock().expect("ImportSessionStore mutex poisoned");
        let now = Instant::now();
        if let Some(s) = g.remove(&token) {
            if now.duration_since(s.created) < TTL {
                return Some(s);
            }
        }
        None
    }
}

impl Default for ImportSessionStore {
    fn default() -> Self {
        Self::new()
    }
}
