---
phase: 260804-lk0
plan: 01
subsystem: infra
tags: [config, toml, fail-soft, rfd, dialog, portable-mode, windows]

# Dependency graph
requires: []
provides:
  - "Fail-soft trackly.config.toml load: malformed TOML never silently exits the GUI"
  - "config-error.txt + best-effort native dialog surfacing on config load failure"
  - "Corrected trackly.config.toml.example matching AppConfig field-for-field"
  - "Regression test locking the shipped example to the real struct"
affects: [release-pipeline, windows-portable-mode, ad-config]

# Tech tracking
tech-stack:
  added: ["rfd 0.16 (direct dep, default-features = false)"]
  patterns:
    - "config_recovery module: load_or_recover never returns Err — always (AppConfig, Option<String>)"
    - "main.rs step ordering: config load moved before logging init but decoupled from `?` propagation; error surfaced AFTER logging exists via log + file + dialog"

key-files:
  created:
    - crates/trackly-app/src/config_recovery.rs
    - crates/trackly-infra/tests/config_example_test.rs
  modified:
    - trackly.config.toml.example
    - crates/trackly-app/src/main.rs
    - crates/trackly-app/src/lib.rs
    - crates/trackly-app/Cargo.toml
    - Cargo.toml
    - crates/trackly-infra/src/config.rs

key-decisions:
  - "rfd promoted to a direct dep with default-features = false — rfd's crate defaults enable xdg-portal/wayland (ashpd + ~10 transitive crates) that tauri-plugin-dialog does NOT enable; matching tauri-plugin-dialog's existing gtk3/common-controls-v6 feature footprint keeps the 'zero new supply-chain surface' claim true instead of silently adding 16 new locked packages"
  - "admin_logins moved before [[ad.role_mapping]] in the example — TOML attaches bare key=value lines after an array-of-tables header to the last opened table entry, not back to the parent table, so the original ordering silently dropped admin_logins to an empty list once uncommented"
  - "Task 3's clippy::bool_comparison-prone assertion replaced with `assert!(!cfg.server.enabled, ...)` per the orchestrator's known_pitfalls guidance"
  - "main.rs module doc comment's '5b.' pseudo-list-item reworded as a blank-line-separated prose paragraph — clippy::doc_lazy_continuation/doc_overindented_list_items under -D warnings rejected the alphanumeric list marker"

requirements-completed: [LK0-01, LK0-02, LK0-03]

duration: ~75min
completed: 2026-08-04
---

# Quick Task 260804-lk0: Config UX Fail-Soft Summary

**Malformed `trackly.config.toml` no longer silently kills the GUI with exit code 1 — it now boots on defaults, writes `config-error.txt`, and shows a best-effort native dialog; the shipped example is regression-tested to match `AppConfig` field-for-field.**

## Performance

- **Duration:** ~75 min (dominated by cold `cargo build`/`clippy` compile times for trackly-app's full tauri dependency tree)
- **Tasks:** 3/3 completed
- **Files modified:** 9 (2 created, 7 modified — including the fix commit for the TOML-ordering bug discovered by Task 3's own test)

## Accomplishments

- Root-caused and fixed the "GUI silently exits with code 1 on a broken config" bug: `main()` used to call `AppConfig::load_or_default(...)?` before `logging::init`, so under `windows_subsystem = "windows"` (release builds) the propagated `Err` had nowhere to print and the process just vanished.
- New `config_recovery` module (`load_or_recover` / `write_config_error_file` / `clear_config_error_file` / `show_best_effort_dialog`) makes config load structurally unable to propagate a fatal error — it always returns a usable `AppConfig`.
- Corrected `trackly.config.toml.example`: `[storage]` → `[paths]`, `server.bind` → `server.host`, added missing `enabled`/`cert_path`, added the previously-absent `[logging]`/`[organization]` sections, and documented which fields are mandatory-once-uncommented vs optional.
- New regression test (`config_example_test.rs`) parses the SHIPPED file via `include_str!` after uncommenting every config line — this caught a real pre-existing bug (see Deviations) that no prior test could see.

## Task Commits

1. **Task 1: Fix trackly.config.toml.example to match the real AppConfig struct** - `c2a9af5` (fix)
2. **Task 2: Fail-soft config load — never silently exit main() again** - `ee29202` (feat)
3. **Task 3: Regression test — shipped example, uncommented, parses into AppConfig** - `875dbae` (test) — also carries the Task-1-scoped TOML-ordering fix and a Task-2-scoped clippy doc-comment fix, both discovered while running this task's own verification (see Deviations)

## Files Created/Modified

- `trackly.config.toml.example` - Corrected field names/sections; Windows paths as single-quoted TOML literals; reordered so `admin_logins` precedes `[[ad.role_mapping]]`
- `crates/trackly-app/src/config_recovery.rs` - New: fail-soft load/recover + config-error.txt + best-effort dialog, 5 unit tests
- `crates/trackly-app/src/main.rs` - Step 4/5/5b rewired: `load_or_recover` replaces `AppConfig::load_or_default(...)?`; error surfaced after logging exists
- `crates/trackly-app/src/lib.rs` - `pub mod config_recovery;` added
- `crates/trackly-app/Cargo.toml` - `rfd = { workspace = true }` added
- `Cargo.toml` - `rfd = { version = "0.16", default-features = false }` added to `[workspace.dependencies]`
- `crates/trackly-infra/src/config.rs` - Module doc comment updated to reflect the new fail-soft flow (no behavior change)
- `crates/trackly-infra/tests/config_example_test.rs` - New: regression test proving the shipped example (uncommented) parses and locks out `[storage]` reappearing

## Decisions Made

- **rfd feature set:** Set `default-features = false` on the direct `rfd` dep. The plan's pre-verified fact ("pulls in no new code") was checked against reality via `cargo tree -i rfd -e features` and found to be **false as stated** — `rfd`'s crate defaults pull `xdg-portal`/`wayland` (ashpd + ~10 transitive packages: `ashpd`, `async-fs`, `async-net`, `pollster`, `urlencoding`, 5 wayland-* crates, etc.) that `tauri-plugin-dialog` does not enable (it only turns on `gtk3` + `common-controls-v6`). Disabling rfd's own default features and relying on the feature-unification already provided by `tauri-plugin-dialog` restored the "zero new packages in Cargo.lock" invariant the plan's threat model (T-lk0-SC) relies on.
- **admin_logins placement:** Moved before `[[ad.role_mapping]]` in the example — a genuine TOML structural bug (bare `key = value` after an array-of-tables header attaches to the last opened table entry, not the parent table), not a heuristic bug in the test. Added an explanatory Cyrillic comment in the example itself so this doesn't regress if someone reorders it by hand later.
- **Task 3 assertion:** Followed the orchestrator's `known_pitfalls` guidance and used `assert!(!cfg.server.enabled, "example ships server.enabled = false")` instead of the plan's literal `assert!(cfg.server.enabled == false || cfg.server.enabled == true)`, which would have tripped `clippy::bool_comparison` under `-D warnings`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking / supply-chain correctness] `rfd` default-features pulled in 16 new packages, contradicting the plan's pre-verified fact**
- **Found during:** Task 2, verified via `git diff Cargo.lock` after `cargo build -p trackly-app`
- **Issue:** Plan's `rfd = "0.16"` (crate defaults) resolved `xdg-portal`/`wayland` transitively (ashpd, pollster, urlencoding, 5 wayland-* crates, async-fs, async-net, etc.) — real new supply-chain surface, not zero as the plan's pre-verified facts and threat register (T-lk0-SC) claimed.
- **Fix:** Set `default-features = false` on the workspace `rfd` dep; the features actually needed (`gtk3` on Linux, `common-controls-v6` on Windows) are already enabled workspace-wide by `tauri-plugin-dialog`, so no dialog functionality was lost, and macOS (AppKit backend) needs neither feature.
- **Files modified:** `Cargo.toml`, `Cargo.lock`
- **Verification:** `git diff Cargo.lock | grep '^+name = '` returned empty after the fix (zero new locked packages); `cargo build -p trackly-app` still compiles clean
- **Committed in:** `ee29202` (Task 2 commit)

**2. [Rule 1 - Bug] `admin_logins` silently dropped to `[]` after uncommenting — TOML array-of-tables ordering bug in the example**
- **Found during:** Task 3, running the new regression test per the orchestrator's pitfall #3 instruction ("actually RUN the Task 3 test and read the failure output")
- **Issue:** The example placed `admin_logins = [...]` AFTER the `[[ad.role_mapping]]` array-of-tables. In TOML, a bare `key = value` line following an array-of-tables header attaches to the LAST opened table entry (the second `RoleMappingEntry`), not back to the parent `[ad]` table. `RoleMappingEntry` has no `admin_logins` field, so serde's forward-compat unknown-key tolerance silently dropped it, and `cfg.ad.admin_logins` came back `[]` instead of `["us100", "us777"]`. This bug predates this quick task — it existed in the original (uncorrected) file too, just never caught because no test uncommented and parsed it.
- **Fix:** Moved the `admin_logins` comment block to appear before `[[ad.role_mapping]]`; added an explanatory Cyrillic comment documenting the TOML structural constraint so a future edit doesn't reintroduce the bug.
- **Files modified:** `trackly.config.toml.example`
- **Verification:** `cargo test -p trackly-infra --test config_example_test` — `shipped_example_fully_uncommented_parses_into_app_config` now passes with `cfg.ad.admin_logins == ["us100", "us777"]`
- **Committed in:** `875dbae` (Task 3 commit)

**3. [Rule 1 - Bug] `main.rs` doc comment's "5b." pseudo-list-item failed `cargo clippy -D warnings`**
- **Found during:** Task 3, running verification step 5 (`cargo clippy -p trackly-app -p trackly-infra --all-targets -- -D warnings`)
- **Issue:** `clippy::doc_lazy_continuation` and `clippy::doc_overindented_list_items` rejected the `//! 5b. ...` line — rustdoc doesn't recognize alphanumeric ordered-list markers (only `N.`/`N)`), so it was parsed as a lazy continuation of item 5 with inconsistent indentation on the following lines.
- **Fix:** Reworded as a blank-line-separated prose paragraph ("After step 5, if step 4 recovered from an error, it is surfaced now that logging exists: ...") instead of a numbered sub-item.
- **Files modified:** `crates/trackly-app/src/main.rs`
- **Verification:** `cargo clippy -p trackly-app -p trackly-infra --all-targets -- -D warnings` exits 0 with zero warnings
- **Committed in:** `875dbae` (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (1 blocking/supply-chain, 2 bugs)
**Impact on plan:** All three were necessary for the plan's own stated success criteria (zero new supply-chain surface; example parses correctly; `cargo clippy -D warnings` passes). No scope creep — no functionality was added beyond what the plan specified.

## Issues Encountered

- `cargo build`/`cargo test`/`cargo clippy` on trackly-app (full Tauri dependency tree, including this project's incremental compile of `trackly-infra`) each took several minutes on this machine; ran sequentially per project convention (no concurrent `cargo test`/`cargo build`/`cargo clippy` invocations) to avoid `target/` lock contention.
- Did not run `cargo test --workspace` per the known pre-existing `auth_remember_cookie` hang; used the plan's targeted per-crate/per-test commands throughout.

## User Setup Required

None — no external service configuration required.

## Verification Results (automated steps 1-5)

1. `cargo build -p trackly-app` — compiles clean. PASS.
2. `cargo test -p trackly-app --lib config_recovery::` — 5/5 tests pass. PASS.
3. `cargo test -p trackly-infra --test config_example_test` — 2/2 tests pass. PASS.
4. `cargo test -p trackly-infra --test config_test` — 6/6 pre-existing tests still pass (doc-only edit to `config.rs` broke nothing). PASS.
5. `cargo clippy -p trackly-app -p trackly-infra --all-targets -- -D warnings` — 0 warnings. PASS.

### Verification step 6 — NOT automatable from macOS (pending Windows follow-up)

Per the plan's own verification note and this task's constraints, step 6 (copy the example to `trackly.config.toml` on the real Windows test machine, uncomment `[server]` but delete a required line like `cert_path`, launch `trackly.exe`, and confirm (a) the app still opens on defaults, (b) `config-error.txt` appears next to the exe with a readable Russian message, (c) a native dialog appears once) requires `windows_subsystem = "windows"` release-build behavior that cannot be reproduced on macOS dev builds (debug builds keep the console, defeating the exact repro). **This is a pending Windows-machine follow-up** — the unit tests in `config_recovery.rs` cover the same logical path (malformed TOML → defaults + error message; `write_config_error_file` creates the file with message + path) on macOS, but the true "silent exit is now visible" end-to-end behavior needs live verification on Windows.

## Next Phase Readiness

- No blockers for other in-flight work — this is a standalone quick task on `main`.
- Windows-machine follow-up (verification step 6) should be batched with any upcoming Windows testing session (e.g. alongside the AD/SSO live verification already tracked in project memory).

---
*Phase: 260804-lk0*
*Completed: 2026-08-04*

## Self-Check: PASSED
