//! ProcMon orchestration stub — full implementation lands in Plan 06 Task 2.
//!
//! This file exists only so `cargo fmt` (which walks `mod` declarations
//! independent of `cfg`) can resolve the `#[cfg(windows)] mod procmon;`
//! reference from `main.rs`. The real `ensure_procmon_on_path` / `run_capture`
//! functions are added in Task 2.

#![allow(dead_code, reason = "stub overwritten by Task 2")]
