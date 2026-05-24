---
phase: 1
slug: foundation
status: ready
nyquist_compliant: true
wave_0_complete: false
created: 2026-05-24
updated: 2026-05-24
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Populated from `01-RESEARCH.md` Validation Architecture and the per-task `<automated>` lines of all six `01-0?-PLAN.md` files.
> Every task carries an `<automated>` verify line → `nyquist_compliant: true`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust workspace; `cargo-nextest` optional) + `pnpm svelte-check` + `pnpm lint` (UI workspace stub) |
| **Config file** | `Cargo.toml` workspace, `ui/package.json` (Wave 0 installs both) |
| **Quick run command** | `cargo test --workspace --no-fail-fast` |
| **Full suite command** | `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && (cd ui && pnpm install --frozen-lockfile && pnpm svelte-check && pnpm lint)` |
| **Estimated runtime** | ~45–90 s on M1 dev box (cold), ~15–30 s warm. ProcMon-test is Windows-CI-only (~3 min). |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p <crate-touched>` (scoped to the crate the task modified)
- **After every plan wave:** Run quick command (workspace-wide `cargo test`)
- **Before `/gsd-verify-work`:** Full suite must be green (including `svelte-check`)
- **Max feedback latency:** 30 s scoped, 90 s full workspace

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 1-01-01 | 01 | 0 | BLD-01 | — | N/A | structural | `cargo metadata --no-deps --format-version 1 \| jq` (assert 4 workspace members) | ❌ W0 | ⬜ pending |
| 1-01-02 | 01 | 0 | FOUND-01 | — | N/A | unit + lint | `cargo build --workspace && cargo test -p trackly-core --test no_io_deps` | ❌ W0 | ⬜ pending |
| 1-01-03 | 01 | 0 | BLD-01 | — | N/A | lint | `cd ui && pnpm install --frozen-lockfile && pnpm svelte-check && pnpm lint` | ❌ W0 | ⬜ pending |
| 1-01-04 | 01 | 0 | FOUND-09 | — | N/A | ci | `act push -j fast` (or visual inspection of `.github/workflows/ci-fast.yml`) | ❌ W0 | ⬜ pending |
| 1-02-01 | 02 | 1 | FOUND-04 | T-1-02-A | UNC/SMB roots rejected by Paths::resolve | integration | `cargo test -p trackly-infra --test paths_test` | ❌ W0 | ⬜ pending |
| 1-02-02 | 02 | 1 | FOUND-04 | T-1-02-B | No secrets logged from config | integration | `cargo test -p trackly-infra --test config_test` | ❌ W0 | ⬜ pending |
| 1-02-03 | 02 | 1 | FOUND-11 | T-1-02-C | WEBVIEW2 env var set before window | smoke | `cargo run -p trackly-app -- --self-test` | ❌ W0 | ⬜ pending |
| 1-03-01 | 03 | 1 | FOUND-04 | — | N/A | structural | `cargo build -p trackly-infra` (refinery `embed_migrations!` discovers V001..V012) | ❌ W0 | ⬜ pending |
| 1-03-02 | 03 | 1 | FOUND-07 | T-1-03-A | PRAGMAs WAL/busy/foreign-keys applied per-conn | unit | `cargo test -p trackly-infra --lib db::pragmas db::migrations test_support` | ❌ W0 | ⬜ pending |
| 1-03-03 | 03 | 1 | FOUND-08 | T-1-03-B | Schema invariants (`*_at_utc`, `deleted_at_utc`, `version`) enforced | integration | `cargo test -p trackly-infra --test seed_data --test per_record_invariants --test audit_log_schema --test migration_idempotency` | ❌ W0 | ⬜ pending |
| 1-04-01 | 04 | 2 | FOUND-02 | T-1-04-A | `Secret<T>: !Debug` + zeroize on drop | unit | `cargo test -p trackly-core --lib error::tests primitives::secret::tests primitives::clock::tests && cargo test -p trackly-core --test secret_zeroize && cargo test -p trackly-infra --lib clock_impl error_conversions` | ❌ W0 | ⬜ pending |
| 1-04-02 | 04 | 2 | FOUND-03 | T-1-04-B | Single writer; 5s backpressure timeout | unit | `cargo test -p trackly-infra --lib db::writer_worker db::pools db::migrations::tests` | ❌ W0 | ⬜ pending |
| 1-04-03 | 04 | 2 | FOUND-05 | T-1-04-C | 50 concurrent writes, no SQLITE_BUSY; downgrade leaves file byte-identical | integration + smoke | `cargo test -p trackly-app --test concurrent_writes --test downgrade_protection && cargo run -p trackly-app -- --self-test` | ❌ W0 | ⬜ pending |
| 1-05-01 | 05 | 3 | FOUND-12 | T-1-05-A | `HealthDto` single SoT for both transports | unit | `cargo build -p trackly-app && cargo test -p trackly-app --lib dto::health tauri_cmds::health http::health` | ❌ W0 | ⬜ pending |
| 1-05-02 | 05 | 3 | FOUND-10 / FOUND-12 | T-1-05-B | `Secret<T>` not leaked through tracing; bindings.ts gitignored | integration + smoke | `cargo test -p trackly-app --test export_bindings --test specta_roundtrip --test health_smoke && cargo run -p trackly-app -- --self-test` | ❌ W0 | ⬜ pending |
| 1-06-01 | 06 | 4 | FOUND-11 | T-1-06-A | `--self-test` exits 0 on cyrillic path | smoke | `cargo build -p procmon-check && cargo run -p trackly-app -- --self-test` | ❌ W0 | ⬜ pending |
| 1-06-02 | 06 | 4 | BLD-06 | T-1-06-B | ProcMon CSV check rejects any APPDATA write | ci + integration | `cargo build -p procmon-check && yaml-validate .github/workflows/ci-full.yml` (matrix=ubuntu/macos/windows; `procmon` job runs `procmon-check` on `trackly.exe`) | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*
*File-Exists is `❌ W0` until execute-phase produces the artefact; flips to `✅` upon first green run.*

---

## Wave 0 Requirements

- [ ] `Cargo.toml` (workspace root) + 3 member crates (`trackly-core`, `trackly-infra`, `trackly-app`) + `tools/procmon-check` (Plan 01-01) — `[workspace.dependencies]` MUST pin `axum`, `tower`, `tower-http` so Plan 05 consumes via `workspace = true`
- [ ] `clippy.toml` with `disallowed-methods` list (D-CI-02) (Plan 01-01) — gates success criterion #3
- [ ] `rustfmt.toml` + `.editorconfig` (Plan 01-01) — formatting baseline for CI
- [ ] `ui/package.json` + `ui/pnpm-lock.yaml` (Svelte 5 + Vite 6 + svelte-check + eslint) (Plan 01-01) — `pnpm svelte-check` must run green in CI even though Phase 1 ships no UI screens
- [ ] `.github/workflows/ci-fast.yml` (Plan 01-01) — fast checks per push
- [ ] `tools/procmon-check/` Rust bin (Plan 01-06) — REQ-FOUND-11 / BLD-06
- [ ] `.github/workflows/ci-full.yml` (Plan 01-06) — Windows matrix + ProcMon
- [ ] `migrations/V001..V012` SQL files (Plan 01-03) — REQ-FOUND-04..06
- [ ] `crates/trackly-app/tests/export_bindings.rs` (Plan 01-05) — REQ-FOUND-12 (tauri-specta smoke)
- [ ] `crates/trackly-app/tests/concurrent_writes.rs` (Plan 01-04) — REQ-FOUND-03 (success criterion #2)
- [ ] `crates/trackly-app/tests/downgrade_protection.rs` (Plan 01-04) — REQ-FOUND-05 (success criterion #4)

`wave_0_complete` flips to `true` once Plan 01-01 commits land on `main`.

---

## Validation Architecture (from RESEARCH)

The following invariants MUST be enforceable from automated tests by phase end:

| Dimension | Invariant | Test Plan |
|-----------|-----------|-----------|
| **Schema** | Every user-mutable table has `deleted_at_utc INTEGER NULL`, `version INTEGER NOT NULL DEFAULT 1` | `per_record_invariants.rs` (Plan 03) — SQL introspection via `pragma_table_info` |
| **Schema** | All timestamp columns suffixed `_at_utc`, type `INTEGER NOT NULL` (or NULL for optional) | `per_record_invariants.rs` (Plan 03) |
| **Schema** | Lookup tables (`device_types`, `device_statuses`, `cartridge_states`, `cartridge_statuses`) seeded in V001 | `seed_data.rs` (Plan 03) |
| **PRAGMA** | `journal_mode=WAL`, `synchronous=NORMAL`, `busy_timeout=5000`, `foreign_keys=ON` on every conn | `db::pragmas` unit tests (Plan 03) |
| **Single-writer** | 50 parallel writes (25 Tauri-pattern + 25 axum-pattern) → no `SQLITE_BUSY` | `concurrent_writes.rs` (Plan 04) — success criterion #2 |
| **Single-writer** | Migrations run on write-pool BEFORE any read-pool conn is opened | `AppCtx::build` ordering test (Plan 04) |
| **Portable mode** | No file creates outside `<exe_dir>/` and `%TEMP%/` during `trackly --self-test` | `procmon-check` CSV-parse (Plan 06) — success criterion #1 |
| **Portable mode** | Cyrillic install path `%TEMP%\Документы\Учёт\Trackly\` works end-to-end | `procmon-check` fixture (Plan 06) |
| **Downgrade protection** | `PRAGMA user_version > embedded_max` → `DatabaseFromNewerVersion`, file byte-identical (probe-read pattern) | `downgrade_protection.rs` (Plan 04) — success criterion #4 |
| **tauri-specta** | `HealthDto` round-trips identically through Tauri-invoke and `GET /api/v1/health` | `specta_roundtrip.rs` (Plan 05) — success criterion #5 |
| **Clippy gates** | `disallowed-methods` blocks `dirs::*_dir()`, `chrono::Local::now`, `tauri::Manager::path` | `cargo clippy --workspace --all-targets -- -D warnings` in `ci-fast.yml` (Plan 01) — success criterion #3 |
| **AppError** | Single `Serialize` shape `{code, message, details}` identical in Tauri and axum responses | `error_conversions` round-trip tests (Plan 04) |
| **Secret hygiene** | `Secret<T>` never logged via `tracing::info!(?secret, ...)`; zeroized on drop | `secret_zeroize.rs` + tracing leak-guard test (Plan 04 + Plan 05) |

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| ProcMon on Windows runner end-to-end (download + capture + parse) | FOUND-11, BLD-06 | First CI run after merge needs Sysinternals download cache warm-up | First CI run on `windows-latest` after merge; observe logs and confirm CSV produced |
| WebView2 `WEBVIEW2_USER_DATA_FOLDER` honored on real Win10/11 box with cyrillic path | FOUND-08 | Tauri webview behavior depends on system WebView2 runtime version | Manual smoke on Windows VM after first portable build (Phase 8 will automate via NSIS post-install test) |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (17/17)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (`cargo`, `pnpm`, `ProcMon`) — flips when Plan 01-01 lands
- [x] No watch-mode flags (`cargo test` runs once; `pnpm svelte-check` is one-shot)
- [x] Feedback latency < 90 s (workspace-wide quick run on M1; ProcMon excluded)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending — flips to `approved YYYY-MM-DD` once Wave 0 (Plan 01-01) commits land and CI fast is green on `main`.
