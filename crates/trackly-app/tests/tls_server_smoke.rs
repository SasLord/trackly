//! RED scaffold — Phase 5 Plan 01 Task 3.
//!
//! Tests that self-signed TLS cert can be generated, server started on
//! random port, and SHA-256 fingerprint is non-empty.
//! Will be GREEN after Plan 03 implements TlsServerMode.

#![allow(dead_code, unused_imports)]

use std::time::Duration;

// todo: imports from crate when Phase 3 implements them
// use trackly_app::server::TlsServerMode;
// use rcgen::generate_simple_self_signed;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn tls_server_binds_and_fingerprint_computed() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("RED: TlsServerMode not yet implemented — Plan 03 fills this")
    })
    .await
    .expect("test exceeded 30s budget");
}
