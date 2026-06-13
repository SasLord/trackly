//! RED scaffold — Phase 5 Plan 01 Task 3.
//!
//! Tests that RusqliteSessionStore persists sessions across store recreation
//! (simulating application restart).
//! Will be GREEN after Plan 02 implements RusqliteSessionStore.

#![allow(dead_code, unused_imports)]

use std::time::Duration;

// todo: imports from crate when Phase 2 implements them
// use trackly_app::auth::RusqliteSessionStore;
// use tower_sessions::SessionStore;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_persists_across_store_recreate() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("RED: RusqliteSessionStore not yet implemented — Plan 02 fills this")
    })
    .await
    .expect("test exceeded 30s budget");
}
