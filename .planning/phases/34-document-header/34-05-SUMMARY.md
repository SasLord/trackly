---
phase: 34-document-header
plan: 05
subsystem: pdf-templates
tags: [html-templates, status-endpoint, authz, backend-only]

# Dependency graph
requires: ["34-01: org_settings.full_name column + OrgSettingsDto.full_name", "34-02: DEFAULT_HTML_TEMPLATES/KNOWN_LEGACY_DEFAULTS/resolve_templates_dir mechanism"]
provides:
  - "TemplateFileStatus/TemplateStatusDto DTOs (dto/reports.rs) — D-17 file-based upgrade status shape"
  - "build_templates_status + templates_status Tauri command (tauri_cmds/settings_org.rs), ManageSettings-gated"
  - "handler_templates_status + POST /api/v1/templates_status (http/settings_org.rs), ManageSettings-gated"
  - "tests/templates_status.rs — Current-vs-Customized proof"
affects: [34-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Read-only status derivation reuses upgrade_untouched_defaults_on_startup's exact classification logic (current-default match / legacy-snapshot match / else) without duplicating it into a new comparison function — DEFAULT_HTML_TEMPLATES and KNOWN_LEGACY_DEFAULTS consumed directly, std::fs::read_to_string used instead of load_template so a missing file is distinguishable in principle (folded into Current by design choice, documented in the DTO doc-comment)"
    - "Deliberate authorization divergence: ManageSettings required on BOTH transports for a read-only endpoint, breaking from the closest structural analog (templates_list_for_editor, unguarded) — a phase-level security decision, not an oversight"

key-files:
  created:
    - crates/trackly-app/tests/templates_status.rs
  modified:
    - crates/trackly-app/src/dto/reports.rs
    - crates/trackly-app/src/tauri_cmds/settings_org.rs
    - crates/trackly-app/src/http/settings_org.rs
    - crates/trackly-app/src/specta_export.rs

key-decisions:
  - "Missing/unreadable on-disk file folds into TemplateFileStatus::Current (2-value enum), not a 3rd variant — same 'no evidence of user customization' reasoning the existing upgrade-on-startup pass already applies to an absent file; documented in TemplateStatusDto's doc-comment per the plan's explicit discretion clause."
  - "Registered templates_status in specta_export.rs's collect_commands! (Rule 2 — auto-add missing critical functionality): the plan's task list didn't call this out explicitly, but every other Tauri command in this file is registered there for TS binding generation and invoke-handler wiring; omitting it would silently make the command unreachable from the frontend/desktop invoke path."

requirements-completed: [DOC-06]

duration: ~40min
completed: 2026-08-09
---

# Phase 34 Plan 05: D-17 template status endpoint (backend-only) Summary

**Added a read-only `/templates_status` endpoint (Tauri command + HTTP handler) that reports per-file Current/Customized status for all 4 file-based HTML templates by reusing Plan 34-02's `DEFAULT_HTML_TEMPLATES`/`KNOWN_LEGACY_DEFAULTS` registry — gated behind `ManageSettings` on both transports, deliberately stricter than the unguarded `templates_list_for_editor` precedent, with zero UI consumer added.**

## Performance

- **Duration:** ~40 min
- **Tasks:** 2 completed
- **Files modified:** 4 (1 created, 3 modified... plus `specta_export.rs` as an in-scope Rule 2 addition, 5 total)

## Accomplishments

- `TemplateFileStatus` (Current | Customized) + `TemplateStatusDto` (filename, status, templates_dir) added to `dto/reports.rs`, doc-commented to contrast with the DB-backed `TemplateEditorItem` sibling and to document the missing-file → `Current` design choice.
- `build_templates_status(ctx: &AppCtx)` resolves `templates_dir` via `html_templates::resolve_templates_dir(&ctx.paths)`, then for each of the 4 `DEFAULT_HTML_TEMPLATES` entries reads the on-disk file directly (`std::fs::read_to_string`, not `load_template`) and classifies: missing/unreadable or byte-identical to the current default → `Current`; byte-identical to any `KNOWN_LEGACY_DEFAULTS` snapshot for that filename → still `Current` (pending the same auto-upgrade path, not user-customized); otherwise → `Customized`. Never writes to disk.
- `templates_status` Tauri command: `resolve_tauri_identity` → `authorize(&caller, &Action::ManageSettings)` → `build_templates_status`, mirroring `settings_save_org_fields`'s auth posture (not `templates_list_for_editor`'s unguarded one).
- `handler_templates_status` HTTP handler: `session_identity` → `authorize(&caller, &Action::ManageSettings)` → `build_templates_status`, placed in the "mutations, ManageSettings required" section of `http/settings_org.rs` despite being non-mutating, matching its authorization posture. Route `POST /api/v1/templates_status` added, preserving the file's uniform `post(...)`-only router convention (0 `get(...)` calls, confirmed).
- `tests/templates_status.rs` (new): two integration tests via a minimal fully-wired `AppCtx` fixture (mirrors `specta_roundtrip.rs`'s `minimal_ctx`) — `fresh_materialized_dir_reports_current_for_all_four` and `hand_edited_file_reports_customized_others_unaffected` (proves per-file isolation: only the hand-edited `act_handover.html` reports `Customized`, the other 3 stay `Current`).

## Task Commits

Each task was committed atomically:

1. **Task 1: TemplateStatusDto + build_templates_status + Tauri command** - `51eb610` (feat)
2. **Task 2: HTTP handler + POST route + integration test** - `53a10cc` (test)

## Files Created/Modified

- `crates/trackly-app/src/dto/reports.rs` — `TemplateFileStatus` enum + `TemplateStatusDto` struct
- `crates/trackly-app/src/tauri_cmds/settings_org.rs` — `build_templates_status` + `templates_status` Tauri command
- `crates/trackly-app/src/http/settings_org.rs` — `handler_templates_status` + POST route
- `crates/trackly-app/src/specta_export.rs` — `templates_status` registered in `collect_commands!`
- `crates/trackly-app/tests/templates_status.rs` — new integration test file (2 tests)

## Decisions Made

- Kept `TemplateFileStatus` a 2-value enum (`Current`/`Customized`) rather than adding a 3rd "Missing" variant — the plan explicitly left this to discretion; a missing file carries no evidence of user customization, matching the existing `upgrade_untouched_defaults_on_startup` pass's treatment of the same case.
- Registered the new Tauri command in `specta_export.rs`'s `collect_commands!` even though the plan's task list didn't explicitly enumerate this file — every sibling command in `tauri_cmds/settings_org.rs` is registered there, and skipping it would leave `templates_status` unreachable via `invoke()` from the frontend (Rule 2, missing critical functionality for the command to actually work end-to-end).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] `templates_status` not registered in `specta_export.rs`'s `collect_commands!`**
- **Found during:** Task 1, after adding the Tauri command wrapper
- **Issue:** The plan's task list scoped Task 1 to `dto/reports.rs` + `tauri_cmds/settings_org.rs` only. `main.rs`'s `invoke_handler` is driven entirely by the `tauri_specta::Builder` assembled in `specta_export.rs`'s `collect_commands!` macro — a Tauri command not listed there is invisible to both the invoke handler and TS binding generation, i.e. unreachable from any frontend caller despite compiling fine.
- **Fix:** Added `crate::tauri_cmds::settings_org::templates_status` to the existing `collect_commands![...]` list, alongside its siblings (`templates_list_for_editor`, `templates_update_body`, etc.).
- **Files modified:** `crates/trackly-app/src/specta_export.rs`
- **Verification:** `cargo build -p trackly-app` succeeds.
- **Committed in:** `51eb610` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 2)
**Impact on plan:** Direct, in-scope consequence of adding a new Tauri command in this repo's existing registration convention — no scope creep, no architectural change.

## Test Results (foreground, per orchestrator instruction)

- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test templates_status -- --test-threads=1` → **2/2 passed**, 0 failed.
- `cargo build -p trackly-app` → succeeds cleanly (no warnings in the new/modified files).
- Manual verification: `authorize(&caller, &Action::ManageSettings)` present in both `templates_status` (Tauri) and `handler_templates_status` (HTTP) — grep-confirmed, region-checked by reading each function body directly.
- `grep -c "post(handler_templates_status)"` → 1. `grep -c "get("` in `http/settings_org.rs` → 0 (router convention preserved).
- `cargo fmt --check` on all 4 touched-by-this-plan files (`dto/reports.rs`, `tauri_cmds/settings_org.rs`, `http/settings_org.rs`, `tests/templates_status.rs`) → clean, no drift introduced by this plan (pre-existing drift in unrelated files from earlier phases left untouched, per project convention).
- `auth_remember_cookie` / `cargo test --workspace` intentionally never run (pre-existing hang, out of scope per project convention).

## Issues Encountered

None beyond the one Rule 2 deviation documented above.

## User Setup Required

None — no external service configuration required. Backend-only endpoint, no UI consumer in this phase.

## Next Phase Readiness

- `/api/v1/templates_status` (HTTP) and `templates_status` (Tauri invoke) are both live, ManageSettings-gated, and proven correct by integration tests.
- No UI consumer exists yet — this is the explicit D-17 scope boundary. A future template-editor rework (`DOC-12`, deferred, ROADMAP backlog) can consume this endpoint directly; no further backend changes anticipated for that consumption.
- No blockers for Plan 34-06.

## Self-Check: PASSED

- FOUND: crates/trackly-app/tests/templates_status.rs
- FOUND: crates/trackly-app/src/dto/reports.rs (TemplateFileStatus/TemplateStatusDto present)
- FOUND: crates/trackly-app/src/tauri_cmds/settings_org.rs (build_templates_status/templates_status present)
- FOUND: crates/trackly-app/src/http/settings_org.rs (handler_templates_status present)
- FOUND: crates/trackly-app/src/specta_export.rs (templates_status registered)
- FOUND commit: 51eb610 (Task 1)
- FOUND commit: 53a10cc (Task 2)

---
*Phase: 34-document-header*
*Completed: 2026-08-09*
