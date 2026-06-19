# Deferred Items — Phase 09 (AD auth + registration)

Issues discovered during execution that are out of scope for the current
plan/task and were NOT auto-fixed (per executor scope-boundary rule).

## 09-02: `graceful_shutdown_drain` test pre-existing failure

- **Discovered during:** Plan 09-02, Task 1 verification (`cargo test -p trackly-app`)
- **Symptom:** `graceful_shutdown_exits_within_timeout` and
  `shutdown_before_server_starts_is_noop` both panic with:
  `Could not automatically determine the process-level CryptoProvider from
  Rustls crate features. ... install_default() before this point ...`
- **Root cause (suspected):** rustls 0.23's process-level `CryptoProvider`
  singleton is never explicitly installed (`rustls::crypto::ring::default_provider().install_default()`
  or aws-lc-rs equivalent) anywhere in the server/TLS startup path exercised by
  this test binary. Whether the test passes depends on cargo's test-binary
  execution order and whether some other test binary in the same `cargo test`
  invocation happened to initialize a provider first as a side effect.
- **Verified pre-existing:** reproduced on a clean `git stash` (no Phase 9
  Plan 2 changes applied) — same two failures, same panic message.
- **Scope:** unrelated to AD auth (Phase 9). Touches `trackly-app`'s axum/TLS
  server startup path (likely `crates/trackly-app/src/server.rs` or
  `rcgen`/rustls cert-loading code from the Phase-7/8 HTTPS server-mode work).
- **Action:** NOT fixed in this plan. Flagging for a future phase/plan that
  owns the server-mode TLS bring-up, or a standalone `/gsd-debug` session.

## 09-02: `template_service.rs` clippy::len_zero under `--tests`

- **Discovered during:** Plan 09-02, Task 2 verification
  (`cargo clippy -p trackly-app --tests -- -D warnings`)
- **Symptom:** Two `assert!(bytes.len() > 0, ...)` calls inside
  `#[cfg(test)]` code in `crates/trackly-app/src/services/template_service.rs`
  (lines 379, 430) trip `clippy::len_zero` — `-D warnings` turns it into a
  build failure when running clippy with `--tests`.
- **Verified pre-existing:** reproduced on `git stash` (no Phase 9 Plan 2
  changes applied) — same two errors, same lines.
- **Scope:** unrelated to AD auth. Plan 09-02's specified verification
  command is `cargo clippy -p trackly-app -- -D warnings` (no `--tests`
  flag), which IS clean — this only surfaces under `--tests`.
- **Action:** NOT fixed in this plan (out of scope — pre-existing in a file
  this plan does not touch). One-line fix for a future cleanup pass:
  `bytes.len() > 0` → `!bytes.is_empty()` at both call sites.

## 09-03: `backup_service.rs` clippy::disallowed_methods under `--all-targets`

- **Discovered during:** Plan 09-03, Task 1/2 verification
  (`cargo clippy -p trackly-app --all-targets -- -D warnings`)
- **Symptom:** `crates/trackly-app/tests/backup_service.rs:168` uses
  `std::fs::copy` directly (disallowed-methods lint — project convention
  requires `rusqlite::backup::Backup` for DB files, though this call copies
  a fake/placeholder backup file in a test, not a live DB).
- **Verified pre-existing:** file is untouched by this plan (`git status`
  shows no changes to `tests/backup_service.rs`); failure is identical
  before and after this plan's edits.
- **Scope:** unrelated to AD auth/registration (Phase 9 Plan 03). Plan's
  specified verify command is `cargo clippy -p trackly-app -- -D warnings`
  (no `--all-targets`), which IS clean.
- **Action:** NOT fixed in this plan (out of scope). Future cleanup: add
  `#[allow(clippy::disallowed_methods)]` on that specific test helper, since
  the copied file there is a synthetic fixture, not a real SQLite DB file.
