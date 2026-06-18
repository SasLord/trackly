//! Process-global serializer for `rusqlite::Connection` **teardown**.
//!
//! ## Why this exists
//!
//! The bundled SQLite (3.45.3) unix VFS has a lock-order inversion in the
//! connection-*close* path: closing a WAL connection takes a per-inode mutex
//! and then re-enters the process-global VFS mutex (`unixBigLock`) via
//! `sqlite3WalClose → sqlite3OsLock → unixLock → unixEnterMutex`, while a
//! plain `unixClose` on another connection grabs `unixBigLock` first. Two
//! threads each tearing down a *different* WAL database at the same time can
//! deadlock on that mutex — both park forever in `__psynch_mutexwait` with no
//! owner making progress.
//!
//! In production this never triggers: an `AppCtx` (writer conn + 8-reader
//! pool) is built once at startup and dropped once at shutdown, so closes are
//! sequential. It surfaced under `cargo test --workspace`, where many
//! `#[tokio::test]` build a full ctx and drop its 9 connections concurrently
//! across the test runner's threads — a `~13h` wedge of the lib unittest
//! binary (observed) that `tokio::time::timeout` cannot rescue, because the
//! block is a *synchronous* C call on a worker thread, not an awaitable point.
//!
//! ## The fix
//!
//! Funnel every owned-connection drop through one global mutex. With closes
//! serialized, only one thread is ever inside SQLite's close machinery at a
//! time, so the `unixBigLock`/per-inode ordering can't invert. Opens are left
//! alone: the inversion is specific to the multi-step close path, and an open
//! that briefly waits on `unixBigLock` held by an in-progress close does not
//! itself hold anything the close wants (no cycle).
//!
//! Cost is nil in production (teardown is a one-shot, uncontended event) and
//! negligible in tests (close is microseconds).

use std::sync::{Mutex, MutexGuard};

/// The one global lock. Guards the moment a `rusqlite::Connection` is dropped.
static CLOSE_LOCK: Mutex<()> = Mutex::new(());

/// Acquire the global close-serialization guard. Hold it across the drop of an
/// owned `Connection` (or a batch of them) so SQLite's VFS close path runs
/// single-threaded process-wide.
///
/// Poison-resilient: a panic elsewhere while holding the guard must not wedge
/// every future connection close (that would just reintroduce the hang in a
/// different shape), so we recover the guard via `into_inner`.
#[must_use = "the guard must be held for the duration of the connection drop"]
pub fn close_guard() -> MutexGuard<'static, ()> {
    CLOSE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
