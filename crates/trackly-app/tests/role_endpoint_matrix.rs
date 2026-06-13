//! RED scaffold — Phase 5 Plan 01 Task 3.
//!
//! Tests that employee role cannot mutate devices (RBAC via HTTP).
//! Will be GREEN after Plan 02+03 implement AuthService + full router.

#![allow(dead_code, unused_imports)]

use std::time::Duration;

// todo: imports from crate when Phase 2/3 implement them
// use axum::http::StatusCode;
// use trackly_app::http::router as full_router;
// use trackly_app::services::AuthService;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn employee_cannot_mutate_devices() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("RED: full router with auth middleware not yet implemented — Plan 02+03 fills this")
    })
    .await
    .expect("test exceeded 30s budget");
}
