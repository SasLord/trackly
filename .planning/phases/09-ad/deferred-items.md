# Deferred Items — Phase 09 (AD auth + registration)

Issues discovered during execution that are out of scope for the current
plan/task and were NOT auto-fixed (per executor scope-boundary rule).

## 09-02: `graceful_shutdown_drain` test pre-existing failure — RESOLVED

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
- **Resolution (2026-06-20, gap-closure fix during 09-05 human-verify):**
  confirmed root cause via `cargo tree` — both `ring` (via `rcgen` +
  `tokio-rustls`) and `aws-lc-rs` (transitively via `ldap3`) are in the
  dependency graph, so rustls 0.23 cannot auto-select a provider. Fixed by
  adding `rustls::crypto::ring::default_provider().install_default()` behind
  an idempotent `Once` guard in `crates/trackly-app/src/server/tls.rs`
  (`ensure_crypto_provider()`), called as the first line of
  `build_server_config()` and `load_from_pem()`, plus an explicit call early
  in `crates/trackly-app/src/main.rs` startup. Enabled the `ring` feature on
  the `rustls` dependency in `crates/trackly-app/Cargo.toml` (pure-Rust, no
  C-toolchain — `aws-lc-rs` was deliberately not chosen, per CLAUDE.md's
  portable-build constraint). Added a regression test
  (`server::tls::tests::generate_self_signed_does_not_panic`) and confirmed
  both `graceful_shutdown_exits_within_timeout` and
  `shutdown_before_server_starts_is_noop` now pass.

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

## 09-05 human-verify: WS reconnect toast spam (browser/server mode) → /gsd-debug

- **Discovered during:** Phase 09 live human-verify (server mode, LAN browser).
- **Symptom:** A `warning` toast «Соединение с сервером потеряно. Переподключение…»
  appears roughly every second, indefinitely. Server log shows repeated TLS
  handshake failures: `TLS accept error ... received fatal alert: CertificateUnknown`
  and `tls handshake eof`.
- **Diagnosis (from code read, not yet fixed):** `ui/src/lib/api/ws.ts` —
  `ws.onclose` calls `showReconnectingToast()` on EVERY reconnect attempt with no
  dedup/throttle (line ~67), so a failing WSS connection spams toasts. Underlying
  cause of the failures: the browser does not trust the self-signed TLS cert for the
  `wss://` connection (`CertificateUnknown`) — partly environmental (cert trust), but
  the toast-spam UX is a real bug regardless. Fix directions: (a) throttle/dedup the
  reconnecting toast (show once per disconnection episode, not per attempt) and honor
  the existing exponential backoff; (b) investigate self-signed cert acceptance for WSS
  on the same origin.
- **Scope:** server-mode WS/TLS infra (Phase 5 TLS / Phase 6 WS client), NOT Phase 9
  AD. User explicitly deferred to a dedicated `/gsd-debug` session.
- **Action:** run `/gsd-debug` — investigate `ui/src/lib/api/ws.ts` reconnect/toast logic.

## 09-05 human-verify: Reports page constant reload/flicker → /gsd-debug

- **Discovered during:** Phase 09 live human-verify (admin, server desktop app).
- **Symptom:** The «Отчёты» (Reports) screen flickers / reloads continuously under admin.
- **Diagnosis (from code read, not yet fixed):** `ui/src/features/reports/ReportsPage.svelte:320`
  — a Svelte 5 `$effect` reads `activeDomain`/`activeReport`/`period`/`filter` and calls
  `loadReport()` + `loadStatusCounts()`. Classic infinite-loop signature: the load path
  (or `PeriodSelector.svelte:78`'s `$effect` watching `dateFrom`/`dateTo`) likely
  reassigns one of the tracked objects (`period`/`filter`), re-triggering the effect.
  Confirm which reactive dependency gets reassigned during load and break the cycle
  (e.g. compare-before-assign, untrack, or split read vs write state).
- **Scope:** Phase 7 (reports/dashboard), NOT Phase 9. User explicitly deferred to `/gsd-debug`.
- **Action:** run `/gsd-debug` — investigate `ui/src/features/reports/ReportsPage.svelte:320`
  `$effect` loop (and `PeriodSelector.svelte:78`).

## Phase 10 (future): Employee role restriction + dedicated employee UI

- **Discovered during:** Phase 09 live human-verify — an `employee` (Сотрудник) user
  logging in (now possible via AD) sees nearly the full admin shell.
- **Diagnosis:** `ui/src/features/layout/sidebar-config.ts` gates only `/users` and
  `/settings` to `admin`; all other sections are visible to every role. There is no
  "employee = Заявки only" gating, and no dedicated employee interface.
- **User decision:** This is a NEW feature/phase, not a Phase 9 defect. Build a separate
  **Phase 10**: restrict employees to request submission (Заявки) and design a dedicated
  employee UI. Note: backend operation-level RBAC exists (role gates), but the nav/shell
  is not gated — verify backend read-endpoint gating as part of Phase 10 scope.
- **Action:** Phase 10 is NOT yet in ROADMAP.md. To start: `/gsd-phase` to add it, then
  `/gsd-spec-phase 10` / `/gsd-discuss-phase 10` → `/gsd-plan-phase 10` → `/gsd-execute-phase 10`.
