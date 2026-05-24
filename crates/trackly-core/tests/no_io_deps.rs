//! Integration test: assert that `trackly-core` does NOT pull any I/O crate
//! into its dependency closure. Enforces FOUND-01 (hexagonal core boundary).
//!
//! Runs `cargo tree -p trackly-core --prefix none --edges no-build,no-dev`
//! and fails if any forbidden crate name appears in the closure.

use std::process::Command;

const FORBIDDEN_CRATES: &[&str] = &[
    "tokio",
    "rusqlite",
    "tauri",
    "axum",
    "hyper",
    "tower",
    "reqwest",
    "sqlx",
    "libsqlite3-sys",
];

#[test]
fn trackly_core_has_no_io_deps() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = Command::new(&cargo)
        .args([
            "tree",
            "-p",
            "trackly-core",
            "--prefix",
            "none",
            "--edges",
            "no-build,no-dev",
        ])
        .output()
        .expect("failed to execute `cargo tree`");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "`cargo tree` exited non-zero. stderr:\n{stderr}"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut offenders: Vec<&str> = Vec::new();
    for line in stdout.lines() {
        // `cargo tree --prefix none` emits lines like `name v0.1.2` (one crate per line).
        // Take the crate name = first whitespace-delimited token.
        let Some(name) = line.split_whitespace().next() else {
            continue;
        };
        if FORBIDDEN_CRATES.contains(&name) {
            offenders.push(name);
        }
    }

    assert!(
        offenders.is_empty(),
        "trackly-core dependency closure contains forbidden I/O crates: {offenders:?}\n\
         Full `cargo tree` output:\n{stdout}"
    );
}
