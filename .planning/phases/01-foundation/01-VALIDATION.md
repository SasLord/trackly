---
phase: 1
slug: foundation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-24
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Filled per the Validation Architecture section of `01-RESEARCH.md`.
> Concrete per-task rows will be expanded by the planner once PLAN.md files exist.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust workspace, `cargo-nextest` optional) + `pnpm svelte-check` + `pnpm lint` (UI workspace stub) |
| **Config file** | `Cargo.toml` workspace, `ui/package.json` (Wave 0 installs both) |
| **Quick run command** | `cargo test --workspace --no-fail-fast -- --nocapture` |
| **Full suite command** | `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && (cd ui && pnpm install --frozen-lockfile && pnpm svelte-check && pnpm lint)` |
| **Estimated runtime** | ~45–90 s on M1 dev box (cold), ~15–30 s warm. ProcMon-test is Windows-CI-only (~3 min). |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p <crate-touched>` (scoped to the crate the task modified)
- **After every plan wave:** Run quick command (workspace-wide `cargo test`)
- **Before `/gsd-verify-work`:** Full suite must be green (including `svelte-check` once `ui/` scaffolding lands)
- **Max feedback latency:** 30 s for scoped runs, 90 s for full workspace

---

## Per-Task Verification Map

> Per-task rows are populated by the planner once `*-PLAN.md` files exist. The plan-checker enforces that every task carries an `<automated>` verify line or a Wave 0 dependency.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 1-01-01 | 01 | 0 | BLD-01 | — | N/A | infra | `cargo build --workspace` | ❌ W0 | ⬜ pending |
| (rest)  | …  | …  | …          | …          | …               | …         | …                 | …           | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `Cargo.toml` (workspace root) + 3 member crates (`trackly-core`, `trackly-infra`, `trackly-app`) — REQ-FOUND-01..02, BLD-01
- [ ] `clippy.toml` with `disallowed-methods` list (D-CI-02) — REQ-FOUND-09 acceptance gate
- [ ] `rustfmt.toml` + `.editorconfig` — formatting baseline for CI
- [ ] `ui/package.json` + `ui/pnpm-lock.yaml` (Svelte 5 + Vite 6 + svelte-check + eslint) — even though Phase 1 ships no UI screens, `pnpm svelte-check` must run green in CI (success criterion #3)
- [ ] `tools/procmon-check/` Rust bin (Windows-only feature-gated) — REQ-FOUND-11 / BLD-06
- [ ] `.github/workflows/ci-fast.yml` + `.github/workflows/ci-full.yml` — REQ-BLD-01 / BLD-06
- [ ] `migrations/V001..V012` SQL stubs (refinery-discoverable) — REQ-FOUND-04..06
- [ ] `trackly-app/tests/export_bindings.rs` — REQ-FOUND-12 (tauri-specta smoke)
- [ ] `trackly-app/tests/concurrent_writes.rs` — REQ-FOUND-03 (success criterion #2)
- [ ] `trackly-app/tests/downgrade_protection.rs` — REQ-FOUND-05 (success criterion #4)

---

## Validation Architecture (from RESEARCH)

The following invariants MUST be enforceable from automated tests by phase end:

| Dimension | Invariant | Test Approach |
|-----------|-----------|---------------|
| **Schema** | Every user-mutable table has `deleted_at_utc INTEGER NULL`, `version INTEGER NOT NULL DEFAULT 1` | SQL introspection test against fresh tempfile DB (`pragma_table_info`) |
| **Schema** | All timestamp columns suffixed `_at_utc`, type `INTEGER NOT NULL` (or NULL for optional) | SQL introspection test enumerating columns of all tables |
| **Schema** | Lookup tables (`device_types`, `device_statuses`, `cartridge_states`, `cartridge_statuses`) seeded in V001 | SELECT count + value assertions in test |
| **PRAGMA** | `journal_mode=WAL`, `synchronous=NORMAL`, `busy_timeout=5000`, `foreign_keys=ON` applied on every connection | Read PRAGMAs back, assert values |
| **Single-writer** | 50 parallel writes (25 via Tauri pattern + 25 via axum pattern) complete with no "database is locked" | `tests/concurrent_writes.rs` (success criterion #2) |
| **Single-writer** | Migrations run on write-pool BEFORE any read-pool connection is opened | Trace ordering assertion in `AppCtx::init()` test |
| **Portable mode** | No file creates outside `<exe_dir>/` and `%TEMP%/` during `trackly --self-test` | `tools/procmon-check` ProcMon CSV-parse test (Win CI) (success criterion #1) |
| **Portable mode** | Cyrillic install path `%TEMP%\Документы\Учёт\Trackly\` works end-to-end | ProcMon test fixture uses cyrillic path |
| **Downgrade protection** | Opening a DB with `PRAGMA user_version > embedded_max` → `AppError::DatabaseFromNewerVersion`, file byte-identical | `tests/downgrade_protection.rs` (success criterion #4) |
| **tauri-specta** | `HealthDto` round-trips identically through Tauri-invoke and `GET /api/v1/health` | `tests/specta_roundtrip.rs` deserializes both responses into same Rust type (success criterion #5) |
| **Clippy gates** | `disallowed-methods` blocks `dirs::*_dir()`, `chrono::Local::now`, `tauri::Manager::path` | CI step `cargo clippy --workspace --all-targets -- -D warnings` green (success criterion #3) |
| **AppError** | Single `Serialize` shape `{code, message, details}` identical in Tauri and axum responses | Round-trip test serializing each variant via both paths |

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| ProcMon on Windows runner end-to-end (download + capture + parse) | FOUND-11, BLD-06 | First-time runner setup may require Sysinternals download cache | First CI run on `windows-latest` after merge; observe logs and confirm CSV produced |
| WebView2 `WEBVIEW2_USER_DATA_FOLDER` honored on real Win10/11 box with cyrillic path | FOUND-08 | Tauri webview behavior depends on system WebView2 runtime version | Manual smoke on Windows VM after first portable build (Phase 8 will automate via NSIS post-install test) |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (`cargo`, `pnpm`, `ProcMon`, refinery layout)
- [ ] No watch-mode flags
- [ ] Feedback latency < 90 s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
