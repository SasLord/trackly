//! RED scaffold — Phase 5 Plan 01 Task 3.
//!
//! Tests create/get UserDto lifecycle via AuthService.
//! Will be GREEN after Plan 02 implements AuthService.

#![allow(dead_code, unused_imports)]

use std::time::Duration;

// todo: imports from crate when Phase 2 implements them
// use trackly_app::services::AuthService;
// use trackly_app::dto::auth::{UserNew, UserDto};
// use trackly_infra::test_support::test_writer_and_readers;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn users_create_and_get() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("RED: AuthService not yet implemented — Plan 02 fills this")
    })
    .await
    .expect("test exceeded 30s budget");
}
