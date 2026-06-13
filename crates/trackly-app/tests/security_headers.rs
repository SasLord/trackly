//! RED scaffold — Phase 5 Plan 01 Task 3.
//!
//! Tests that GET /api/v1/health on a live server returns expected
//! security headers: x-frame-options: DENY, x-content-type-options: nosniff.
//! Will be GREEN after Plan 03 implements security header middleware.

#![allow(dead_code, unused_imports)]

use std::time::Duration;

// todo: imports from crate when Phase 3 implements them
// use trackly_app::server::ServerToggle;
// use reqwest::Client;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn security_headers_present() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("RED: security header middleware not yet implemented — Plan 03 fills this")
    })
    .await
    .expect("test exceeded 30s budget");
}
