//! procmon-check — Phase 1 scaffold. Real implementation in Plan 06.
//!
//! On non-Windows hosts the binary is a no-op so `cargo build --workspace`
//! succeeds on macOS / Linux dev machines.

#[cfg(not(windows))]
fn main() {
    println!("procmon-check is Windows-only; skipping on this host");
    std::process::exit(0);
}

#[cfg(windows)]
fn main() {
    println!("procmon-check Phase 1 scaffold — real implementation lands in Plan 06");
    std::process::exit(0);
}
