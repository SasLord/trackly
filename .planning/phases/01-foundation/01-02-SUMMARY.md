---
phase: 01-foundation
plan: 02
subsystem: infra+app
tags: [rust, paths, portable, webview2, toml, config, app-error, unsafe, cyrillic, unc]

requires:
  - 01-01 (workspace topology, clippy disallowed-methods gate, MSRV 1.88, toml + thiserror + tempfile in [workspace.dependencies])
provides:
  - trackly_infra::Paths with sentinel-based portable detection (portable.txt OR trackly.config.toml) and Windows-only UNC rejection (Security V8 control)
  - trackly_infra::AppConfig — TOML parser for trackly.config.toml per D-Config-01 (server/paths/logging/organization sections, all optional, code-side defaults)
  - trackly_core::error::AppError — minimal 2-variant bootstrap (Internal + Validation); Plan 04 extends to full D-AppError-01 list
  - trackly_app::webview_env::set_webview2_data_folder — wraps unsafe std::env::set_var with SAFETY comment; called as second statement of main()
  - Real main.rs lifecycle: Paths::resolve → set WEBVIEW2 env → parse --self-test → AppConfig::load_or_default → (Plans 03-05 splice in tracing/DB/AppCtx) → exit/run
  - `cargo run -p trackly-app -- --self-test` exits 0, prints diagnostics, creates only <exe_dir>/data/webview/
affects: [03-schema-migrations, 04-appctx-writer, 05-tauri-specta-axum, 06-procmon-ci, all-future-plans]

tech-stack:
  added: []  # All deps already present from Plan 01-01 (toml, thiserror, tempfile, serde all workspace-pinned)
  patterns:
    - "Sentinel-based portable detection (NOT writability probe) per ARCHITECTURE.md"
    - "Test seam via `Paths::resolve_for_exe_dir(PathBuf)` — keeps current_exe()-rooted production path testable without env hacks"
    - "Hand-written impl Default per config section (clearer than #[serde(default = \"…\")]) — except PathsConfig which is trivially derivable (clippy::derivable_impls escalation made the choice for us)"
    - "Forward-compat config parsing: NO #[serde(deny_unknown_fields)] — older binaries must tolerate config keys added by newer versions"
    - "#[rustfmt::skip] on set_webview2_data_folder preserves the one-line `unsafe { std::env::set_var(\"WEBVIEW2_USER_DATA_FOLDER\", path); }` form required by the acceptance-criterion grep"
    - "// SAFETY: comment immediately precedes every unsafe block (Rust 1.85+ env-var safety pattern; Pitfall #8)"

key-files:
  created:
    - crates/trackly-core/src/error.rs
    - crates/trackly-infra/src/paths.rs
    - crates/trackly-infra/src/config.rs
    - crates/trackly-infra/tests/paths_test.rs
    - crates/trackly-infra/tests/config_test.rs
    - crates/trackly-app/src/webview_env.rs
  modified:
    - crates/trackly-core/src/lib.rs (pub mod error)
    - crates/trackly-infra/src/lib.rs (pub mod paths + config; re-exports)
    - crates/trackly-app/src/lib.rs (pub mod webview_env + empty pub mod context {} stub for Plan 04)
    - crates/trackly-app/src/main.rs (replaced placeholder with Plan 02 lifecycle)

key-decisions:
  - "UNC rejection uses `s.starts_with(r\"\\\\\")` (simple prefix check) — richer parsing (e.g., UNC path canonicalisation) is unnecessary because any legitimate exe path will never start with `\\\\`; this is sufficient to reject SMB share roots."
  - "Test seam `Paths::resolve_for_exe_dir(PathBuf)` is part of the PUBLIC API (not gated behind #[cfg(test)]) — tests live in `tests/` integration crate which cannot see private functions. Cost: one extra public function; benefit: clean testing without env-var manipulation."
  - "`impl Default for AppConfig` is `#[derive(Default)]` (not hand-written) because every section already has its own `impl Default` — the derive is correct by composition."
  - "PathsConfig uses `#[derive(Default)]` instead of a hand-written impl — clippy::derivable_impls (escalated to deny via [workspace.lints.clippy]) refused the manual version. Other config sections retain hand-written Default because their default strings are non-empty (`\"127.0.0.1\"`, `\"info\"`, etc.) — clippy did not complain about those."
  - "main.rs returns `anyhow::Result<()>` (not `Result<(), AppError>`) — `?` needs to bridge AppError (config/paths), std::io::Error (webview_env), and future TOML/refinery errors. anyhow at the binary boundary is the canonical Rust pattern; AppError stays the library error for trackly-infra/-core."
  - "Did NOT extend AppError beyond the 2 variants required by Plan 02 — Plan 04 owns the full D-AppError-01 enum, and pre-adding variants here would be speculative work that downstream plans might rework."

duration: ~7 min
completed: 2026-05-24
---

# Phase 1 Plan 02: Paths + Config + WEBVIEW2 + AppError stub Summary

**`trackly_infra::Paths` rooted at `current_exe()?.parent()?` with sentinel-based portable detection and Windows UNC rejection; `trackly_infra::AppConfig` TOML parser per D-Config-01 with forward-compat defaults; `trackly_app::webview_env::set_webview2_data_folder` wraps the `unsafe std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", …)` call with a SAFETY comment and runs as the second statement of `main()`. `cargo run -p trackly-app -- --self-test` now exits 0 after creating only `<exe_dir>/data/webview/`.**

## Performance

- **Duration:** ~7 min wall clock
- **Started:** 2026-05-24T15:16Z
- **Completed:** 2026-05-24T15:23Z
- **Tasks:** 3 / 3 (TDD: 2 RED + 3 GREEN commits)
- **Files created:** 6
- **Files modified:** 4

## Accomplishments

- `Paths::resolve()` correctly anchors to `std::env::current_exe()?.parent()?` and returns a typed `AppError` on failure (no panics, no `unwrap`).
- Sentinel-based portable detection (D-Config-01): `portable.txt` OR `trackly.config.toml` adjacent to the .exe marks the binary as portable. Writability probes deliberately avoided — sentinels are more predictable and don't trigger AV scanners.
- Windows-only `#[cfg(windows)]` UNC rejection branch in `resolve_for_exe_dir` returns `AppError::Validation` with a clear message ("UNC/SMB path rejected: SQLite WAL does not support network shares") — Security V8 control wired exactly as the threat register requires.
- Cyrillic path handling verified on macOS via the `test_5_resolve_accepts_cyrillic_path` test (`Документы/Учёт/Trackly_test`); path comparison uses `Path::components()` (not `to_string_lossy()`) to avoid Pitfall #3 silent normalisation. The test ran green on the first attempt.
- `AppConfig::load_or_default` matches D-Config-01 verbatim:
  - 4 sections (`[server] [paths] [logging] [organization]`) with hand-written `impl Default` (except `PathsConfig` which `#[derive(Default)]`s because clippy::derivable_impls forced the issue);
  - missing file → defaults (NOT error);
  - I/O failure → `AppError::Internal`;
  - parse failure → `AppError::Validation { field: "trackly.config.toml", message }`;
  - unknown keys silently ignored (forward-compat — no `#[serde(deny_unknown_fields)]`).
- `set_webview2_data_folder(path)`:
  - Step 1: `std::fs::create_dir_all(path)?` (WebView2 doesn't create the dir itself);
  - Step 2: `unsafe { std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", path); }` — the literal one-line form required by acceptance criterion's grep is preserved via `#[rustfmt::skip]` on the function;
  - Step 3: `// SAFETY:` comment immediately precedes the unsafe block, explaining why single-threaded `main()` makes this safe (Pitfall #8 — Rust 1.85+ env-var safety).
- `main.rs` ordering invariant locked: `Paths::resolve` is line 1, `set_webview2_data_folder` is line 2, BEFORE anything else. Future contributors who try to add `#[tokio::main]` or a `tokio::runtime::Builder` call above these lines will produce visible diffs on every PR.
- `--self-test` prints diagnostic lines covering all `paths.*` accessors plus `config.server.*`, `config.logging.*`, and `config.organization.timezone`, then exits 0 cleanly. No DB, no tracing setup, no Tauri — those land in Plans 03-05.

## Task Commits

1. **Task 1 RED:** `test(01-02): add failing paths_test for Paths::resolve` — `664e643`
2. **Task 1 GREEN:** `feat(01-02): implement Paths::resolve with sentinel + UNC rejection` — `1effbe3`
3. **Task 2 RED:** `test(01-02): add failing config_test for AppConfig::load_or_default` — `1a42c5b`
4. **Task 2 GREEN:** `feat(01-02): implement AppConfig TOML parser with defaults` — `975049f`
5. **Task 3:** `feat(01-02): wire webview_env + main.rs ordered lifecycle` — `c0792b0`

_Final plan-metadata commit will be added by the orchestrator after this SUMMARY is written._

## Decisions Made

(See `key-decisions` frontmatter above for the full list with rationale.)

Highlights:

- **UNC check is simple `starts_with(r"\\")`** — sufficient to block SMB roots; richer parsing deferred until we hit a real false-positive.
- **`Paths::resolve_for_exe_dir` is public** (test seam) — integration tests in `tests/` can't reach private functions, and keeping it public costs nothing.
- **`anyhow::Result<()>` at `main()` boundary** — `?` bridges `AppError`, `std::io::Error`, and future TOML/refinery errors cleanly. `AppError` stays the library error.
- **AppError was NOT extended beyond the 2 required variants** — Plan 04 owns the full D-AppError-01 enum; pre-adding variants here would be speculative.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Lint] `clippy::derivable_impls` on `PathsConfig`**

- **Found during:** Task 2 GREEN (clippy gate)
- **Issue:** Hand-written `impl Default for PathsConfig { fn default() -> Self { Self { db_path: String::new() } } }` tripped `clippy::derivable_impls` (escalated to deny via `[workspace.lints.clippy]` from Plan 01-01). The other config sections have non-empty string defaults so the derive can't replace them — only `PathsConfig` is trivial.
- **Fix:** Replaced with `#[derive(..., Default)]` on `PathsConfig` (kept the explanatory doc-comment).
- **Files modified:** `crates/trackly-infra/src/config.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` passes.
- **Committed in:** `975049f`

**2. [Rule 1 - Lint] `clippy::doc_lazy_continuation` on numbered list in `main.rs`**

- **Found during:** Task 3 (clippy gate)
- **Issue:** Module-level doc comment `//!   1. Paths::resolve …` (with extra indent for sub-points `//!         + reader pool`) trips `clippy::doc_lazy_continuation` because the indented continuation isn't recognised as part of the list item.
- **Fix:** Flattened the numbered list — removed multi-step "6-10" entry, changed to single-line items 1-8.
- **Files modified:** `crates/trackly-app/src/main.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` passes.
- **Committed in:** `c0792b0`

**3. [Rule 1 - Bug] rustfmt expanded one-line `unsafe { ... }` block, breaking acceptance-criterion grep**

- **Found during:** Task 3 (post-`cargo fmt --all` check)
- **Issue:** `cargo fmt` re-formatted `unsafe { std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", path); }` into the 3-line block form. The Task 3 acceptance criterion requires the literal substring `unsafe { std::env::set_var("WEBVIEW2_USER_DATA_FOLDER"` to appear verbatim in `webview_env.rs` (so a future ProcMon-test wrapper can grep for it).
- **Fix:** Added `#[rustfmt::skip]` at the function level (`#[rustfmt::skip] pub fn set_webview2_data_folder(...)`) because `#[rustfmt::skip]` on expressions is still unstable (E0658, tracking issue #15701). Documented the contract in a comment above the unsafe block.
- **Files modified:** `crates/trackly-app/src/webview_env.rs`
- **Verification:** `grep 'unsafe { std::env::set_var("WEBVIEW2_USER_DATA_FOLDER"' …/webview_env.rs` returns a match; `cargo fmt --all -- --check` passes; `cargo clippy` passes.
- **Committed in:** `c0792b0`

**4. [Rule 1 - Lint] unused imports in `paths_test.rs` on non-Windows targets**

- **Found during:** Task 1 GREEN (first compile of paths_test on macOS)
- **Issue:** `use std::path::PathBuf;` and `use trackly_core::error::AppError;` are only used inside the `#[cfg(windows)]` test (`test_4_resolve_rejects_unc_path_on_windows`). On macOS / Linux they triggered the `unused_imports` warning (would fail under `-D warnings` in CI clippy gate).
- **Fix:** Gated both imports behind `#[cfg(windows)]`.
- **Files modified:** `crates/trackly-infra/tests/paths_test.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` passes; the Windows UNC test will still compile when CI runs `windows-latest` (the gate matches the test's own `#[cfg(windows)]`).
- **Committed in:** `1effbe3` (folded into Task 1 GREEN since it was a same-cycle test-author bug)

**5. [Rule 1 - Bug] `Path` borrow lifetime in cyrillic-path test**

- **Found during:** Task 1 GREEN (first compile)
- **Issue:** `let expected_components: Vec<_> = cyrillic.join("data/webview").components().collect();` — the temporary `PathBuf` from `.join()` was dropped before `.components()` could finish iterating, causing E0716 ("temporary value dropped while borrowed").
- **Fix:** Bound the joined path to a named `let expected = cyrillic.join("data").join("webview");` and called `.components()` on the named binding.
- **Files modified:** `crates/trackly-infra/tests/paths_test.rs`
- **Verification:** `cargo test -p trackly-infra --test paths_test` passes.
- **Committed in:** `1effbe3` (folded into Task 1 GREEN)

---

**Total deviations:** 5 auto-fixed (3× Rule 1 Lint, 2× Rule 1 Bug). All test-/lint-driven; no architectural changes. No Rule 4 (architectural) checkpoints needed. No checkpoints surfaced to the user.

## Issues Encountered

- **macOS dev box cannot exercise the `#[cfg(windows)]` UNC-rejection branch.** The branch compiles on macOS (it's behind `#[cfg(windows)]` so the body is excluded), but the test (`test_4_resolve_rejects_unc_path_on_windows`) only runs on Windows runners. CI's `windows-latest` job in `ci-fast.yml` will be the first true test of this branch. If a regression slips through, Plan 06's ProcMon test catches it behaviourally.
- **`current_exe()` returns the cargo target path during `cargo run`.** The `--self-test` run shows `exe_dir=/Users/madsas/Projects/trackly/target/debug` — that's expected and unproblematic for a dev box. In a portable release build, the exe sits next to the user's chosen folder, and `Paths::resolve()` will resolve correctly.

## User Setup Required

None. No external services. `trackly.config.toml` is optional and the binary runs cleanly without one. To exercise portable mode locally on macOS, drop a `portable.txt` file into `target/debug/` and rerun `cargo run -p trackly-app -- --self-test`; `is_portable = true` will appear in the output.

## Next Phase Readiness

**Ready for Plan 03** (refinery migrations + DB connection):

- `Paths::db_path()` returns the canonical SQLite path Plan 03 will open.
- `AppConfig.paths.db_path` is parsed; Plan 03 should honour it (override only when non-empty).
- `AppError` stub is in place — Plan 04 extends it with `WriteQueueBusy`, `DatabaseFromNewerVersion`, etc., without touching paths/config.
- `main.rs` step 5+ is empty by design — Plan 03/04/05 will splice tracing / writer / migrations / AppCtx between Step 4 and Step 7 without re-ordering.

**Carry-forward notes for downstream plans:**

- `Paths::resolve_for_exe_dir(PathBuf)` is public — Plans 03/04 tests can use it the same way `paths_test.rs` does (no need to manipulate env vars).
- `AppConfig.paths.db_path == ""` is the "use default" sentinel — Plan 03's DB-open code should treat an empty string as "fall through to `Paths::db_path()`".
- Future contributors MUST NOT move `set_webview2_data_folder` below Step 1 of `main()`. The ordering invariant comment in `main.rs` documents this; the ProcMon test in Plan 06 is the behavioural gate.
- The `// SAFETY:` comment pattern is now established in this codebase for any future `unsafe` block (per Pitfall #8 and Rust 2024+ semantics).

**No blockers** for Plan 03.

## Threat Flags

None — no new security-relevant surface introduced beyond what the plan's `<threat_model>` already covers. The TOML config parser is in-scope (T-02-03), the WEBVIEW2 env setup is in-scope (T-02-01, T-02-06), the UNC rejection is in-scope (T-02-02), and the cyrillic path handling is in-scope (T-02-04). T-02-05 (future contributor moves `set_var` below tokio init) remains an "accept" disposition — the Plan 06 ProcMon test is the behavioural gate.

## Self-Check: PASSED

Verified after writing SUMMARY:

- `crates/trackly-core/src/error.rs` exists with `pub enum AppError { Internal { source_chain: String }, Validation { field: String, message: String } }`.
- `crates/trackly-infra/src/paths.rs` exists; contains `std::env::current_exe()`, `is_portable`, and `UNC` literal.
- `crates/trackly-infra/src/config.rs` exists; contains `"127.0.0.1"`, `8443`, `"Europe/Moscow"`, `"info"`, `"compact"`, `14`.
- `crates/trackly-app/src/webview_env.rs` exists; contains literal `unsafe { std::env::set_var("WEBVIEW2_USER_DATA_FOLDER"` and `// SAFETY:` comment.
- All 5 task commits present in git log: `664e643`, `1effbe3`, `1a42c5b`, `975049f`, `c0792b0`.
- `cargo test --workspace` passes (1 + 4 + 6 = 11 unique tests; 4 doc-tests sections green).
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo fmt --all -- --check` passes.
- `cargo run -p trackly-app -- --self-test` exits 0 and prints `paths resolved`, `config loaded`, `Plan 02 placeholder` markers.
- After `rm -rf target/debug/data && cargo run -p trackly-app -- --self-test`, only `target/debug/data/webview/` is created — no files in `~/Library/Application Support`, `~/.config`, or `/tmp` outside tempdir.

---
*Phase: 01-foundation*
*Completed: 2026-05-24*
