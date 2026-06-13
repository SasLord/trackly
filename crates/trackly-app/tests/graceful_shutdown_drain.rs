//! RED scaffold — Phase 5 Plan 01 Task 3.
//!
//! Tests that when a server shutdown is requested while a slow request is
//! in-flight, the server drains the request before closing the port.
//! Will be GREEN after Plan 03 implements graceful shutdown.

#![allow(dead_code, unused_imports)]

use std::time::Duration;

// todo: imports from crate when Phase 3 implements them
// use trackly_app::server::ServerToggle;
// use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graceful_shutdown_drains_inflight() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("RED: graceful shutdown not yet implemented — Plan 03 fills this")
    })
    .await
    .expect("test exceeded 30s budget");
}
