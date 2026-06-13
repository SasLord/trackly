//! RED scaffold — Phase 5 Plan 01 Task 3.
//!
//! Tests that server starts on a random port, allows TCP connection,
//! then stops and refuses further connections (port freed).
//! Will be GREEN after Plan 03 implements ServerToggle.

#![allow(dead_code, unused_imports)]

use std::time::Duration;

// todo: imports from crate when Phase 3 implements them
// use trackly_app::server::ServerToggle;
// use tokio::net::TcpStream;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn server_starts_stops_port_freed() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("RED: ServerToggle not yet implemented — Plan 03 fills this")
    })
    .await
    .expect("test exceeded 30s budget");
}
