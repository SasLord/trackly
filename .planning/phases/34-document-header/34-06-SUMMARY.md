---
phase: 34-document-header
plan: 06
subsystem: pdf-templates
tags: [html-templates, uat, checkpoint, gap-closure, test-suite]

# Dependency graph
requires:
  - phase: "34-02"
    provides: "_header.html canonical partial, ported from target/debug/templates/*.html reference bodies before deletion"
  - phase: "34-03"
    provides: "document-header context wiring across all three print forms"
  - phase: "34-04"
    provides: "OrgSettings full_name multiline field"
  - phase: "34-05"
    provides: "templates_status endpoint (D-17) — backend only, no UI consumer yet"
provides:
  - "Human-confirmed visual parity of the shared header across all three print forms on BOTH desktop Tauri webview and LAN browser transports (DOC-04 Success Criterion #2, satisfied by real rendered preview, not text-extraction)"
  - "Real-install confirmation of DOC-06's upgrade-delivery mechanism: untouched pre-Phase-34 templates silently upgrade, a genuinely hand-edited file is preserved"
  - "Fixed Russian period label in the report print form's .subtitle (ReportService::format_period_label), covering both Tauri and HTTP transports"
  - "target/debug/templates/act_handover.html, act_acceptance.html, report.html removed (D-18) — the phase's only remaining copies of the pre-Phase-34 hand-edited reference bodies"
affects: [35, DOC-12]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Human-verification checkpoints record findings even when the 'defect' turns out to be expected behavior (D-16 preserved a hand-edited file) — the checkpoint's value is in confirming BOTH branches of a conditional mechanism on a real install, not just the happy path"
    - "Full integration-test gate excludes exactly one known-hanging binary (auth_remember_cookie) by name, not by broad category, keeping the exclusion narrow and auditable per invocation"

key-files:
  created:
    - .planning/phases/34-document-header/34-06-GAP-subtitle.md
  modified:
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/src/tauri_cmds/reports.rs
  deleted:
    - target/debug/templates/act_handover.html
    - target/debug/templates/act_acceptance.html
    - target/debug/templates/report.html

key-decisions:
  - "act_handover.html's separate-looking header in the user's live install was NOT a defect: the canonical crates/trackly-app/templates/act_handover.html already includes _header.html; the on-disk file the user was viewing was their own hand-edited copy in target/debug/templates/, which D-14/D-16's upgrade logic correctly left untouched because its body matched neither the current default nor any legacy snapshot. This is DOC-06 Success Criterion #4 verified live, not a synthetic test."
  - "No 'template changed manually' UI affordance in Settings → Шаблоны is expected, not a gap: 34-05 is backend-only by design; the UI consumer of templates_status is deferred to backlog item DOC-12."
  - "C-01 (header with full_name empty, short name set) visual result — short name only, no blank line above — reviewed and explicitly accepted as-is by the user; it is the direct, intended consequence of D-04's independent-conditional-lines design. No follow-up needed."
  - "Task 1 Step 7's scripted TRACKLY_TEMPLATES_DIR scratch-directory procedure was NOT executed as written — it was superseded by discovering the same D-14/D-16 mechanism already exercised on the user's real pre-Phase-34 install (see act_handover.html finding above), which is a stronger proof than the scripted synthetic setup would have been."
  - "Report print-form .subtitle used the raw English PeriodDto.mode discriminator ('year 2026') — found during the checkpoint UAT, in scope for this phase per user's explicit direction, fixed via ReportService::format_period_label, recorded separately in 34-06-GAP-subtitle.md since 34-06-SUMMARY.md did not exist yet when the fix landed."

requirements-completed: [DOC-04, DOC-05, DOC-06]

duration: ~2h05min (Task 3 + full-suite gate; Tasks 1-2 were prior human-verification sessions)
completed: 2026-08-09
---

# Phase 34 Plan 06: Checkpoint UAT + D-18 reference-file cleanup Summary

**Closed the document-header phase with human-confirmed visual parity across desktop and LAN-browser transports, a live (not synthetic) confirmation of DOC-06's upgrade-delivery mechanism, a Russian-localization fix for the report subtitle found during UAT, and removal of the last hand-edited reference template files.**

## Performance

- **Duration:** ~2h05min for this execution session (Task 3 deletion + full regression gate + SUMMARY); Tasks 1-2 (desktop and LAN-browser checkpoints) were separate human-verification sessions, both approved before this session began.
- **Tasks:** 3/3 completed (2 checkpoints previously approved, Task 3 executed this session)
- **Files modified:** 2 (gap-closure fix) + 3 deleted (gitignored build artifacts, no git diff)

## Accomplishments

- **Task 1 (desktop checkpoint) — APPROVED.** Human confirmed, via real rendered previews on `cargo tauri dev`, that all three print forms (акт приёма-передачи, акт приёмки, отчёт) share an identical header (logo, bold 12pt full name, short name in parentheses, D-07-ordered requisites, Times New Roman, 80mm centered column) with no overflow on a long name/address, and reviewed the C-01 empty-`full_name` visual result as acceptable.
- **Task 2 (LAN browser checkpoint) — APPROVED.** Human confirmed the same parity after `pnpm --dir ui build`, in server mode via an ordinary browser, matching the desktop pass modulo expected browser-engine hyphenation differences.
- **Gap found and fixed during UAT:** the report print form's `.subtitle` rendered the raw English `PeriodDto.mode` discriminator (e.g. `"year 2026"`) instead of a localized Russian period label, and silently dropped month/range information. Fixed via a new `ReportService::format_period_label` (13 new unit tests), wired into `build_reports_export_pdf` — covers both the Tauri command and the LAN HTTP transport through the shared builder. Full details in `34-06-GAP-subtitle.md`.
- **Task 3 — executed this session:** deleted `target/debug/templates/act_handover.html`, `act_acceptance.html`, `report.html` (D-18) — confirmed absent via `test ! -e`; these are gitignored build artifacts, so deletion produced no git diff.
- **Final full-suite regression gate — green.** `pnpm --dir ui build` succeeded; the full `trackly-app` test suite (lib + all 84 integration test binaries, excluding the pre-existing `auth_remember_cookie` hang) ran with **zero failures**: 167 lib tests + 472 integration test-functions across 84 binaries, all `ok`, plus 2 intentionally `ignored` in `pdf_determinism` (unrelated to this phase). See "Deviations" for a scripting note on how this count was assembled.

## Task Commits

Tasks 1 and 2 are `checkpoint:human-verify` gates — no code commits, approval only (see Checkpoint Findings below).

1. **Gap closure (found during Task 1/2 UAT): Russian period label** — `50846b0` (fix) + `6837052` (docs: gap record)
2. **Task 3: delete rescued reference files (D-18)** — no commit (gitignored build artifacts under `target/debug/templates/`, zero git diff by design)

**Plan metadata:** commit created at the end of this execution (see final commit below)

## Files Created/Modified

- `crates/trackly-app/src/services/report_service.rs` — added `format_period_label`, `format_ru_short_date`, 13 unit tests (committed `50846b0`)
- `crates/trackly-app/src/tauri_cmds/reports.rs` — wired `format_period_label` into `build_reports_export_pdf` (committed `50846b0`)
- `.planning/phases/34-document-header/34-06-GAP-subtitle.md` — gap-closure record (committed `6837052`)
- `target/debug/templates/act_handover.html`, `act_acceptance.html`, `report.html` — deleted (gitignored, no commit)

## Checkpoint Findings (Tasks 1 & 2, both APPROVED by the user)

These are the actual findings from both human-verification passes, recorded faithfully:

1. **Report `.subtitle` showed an untranslated period (`year 2026`) — genuine defect**, found during this checkpoint. Fixed as an in-phase gap closure at the user's explicit direction (see Accomplishments above and `34-06-GAP-subtitle.md`).

2. **`act_handover.html` appeared to render its own separate header — not a defect.** The canonical `crates/trackly-app/templates/act_handover.html:110` does include `{% include "_header.html" %}`. The file the user was actually viewing was `target/debug/templates/act_handover.html`, a hand-edited copy the D-14/D-16 upgrade mechanism deliberately left untouched because its body matched neither the current default nor any legacy snapshot — while `act_acceptance.html` and `report.html` on the same install DID silently auto-upgrade to the new header. This is DOC-06 Success Criterion #4 verified live on a real pre-Phase-34 install — both branches of the mechanism (silent upgrade of untouched files, preservation of a hand-customized file) confirmed without any synthetic scratch-directory setup. The user then moved that file to `~/trackly-header-reference/` (outside the repo, outside `target/`) and re-verified the header on the regenerated file — approved.

3. **No "template changed manually" indicator in Settings → Шаблоны — not a defect.** Plan 34-05 is backend-only by design; the UI consumer of `templates_status` is deferred to backlog item `DOC-12`.

4. **C-01 (empty `full_name`) — reviewed and explicitly accepted.** With the full name cleared and only the short name set, the header shows only the short name in parentheses with no blank line above it — the direct, expected consequence of D-04's independent-conditional-lines design. No follow-up needed.

5. **Task 1 Step 7's scripted `TRACKLY_TEMPLATES_DIR` scratch-directory procedure was superseded, not run as written.** Finding 2 above exercised the identical upgrade mechanism on a real install, which is a stronger proof than the scripted synthetic setup — recorded honestly rather than claiming the scripted steps executed.

## Decisions Made

See `key-decisions` in frontmatter. Summary: the two apparent "defects" found in Tasks 1-2 (separate act_handover header, missing UI indicator) were both confirmed as expected behavior after investigation, not silently dismissed. The one genuine defect (report subtitle localization) was fixed in-phase per the user's explicit direction. C-01's visual consequence was reviewed and explicitly accepted.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Russian period label in report print-form subtitle**
- **Found during:** Task 1/2 checkpoint UAT (pre-existing defect, scoped into this phase by the user)
- **Issue:** `build_reports_export_pdf` rendered the raw English `PeriodDto.mode` discriminator plus a bare year; month mode lost the month, range mode read `"range 0"`.
- **Fix:** Added `ReportService::format_period_label` covering month/year/range modes with Russian output, wired into the single shared builder used by both Tauri and HTTP transports.
- **Files modified:** `crates/trackly-app/src/services/report_service.rs`, `crates/trackly-app/src/tauri_cmds/reports.rs`
- **Verification:** 22 unit tests green (13 new + 9 pre-existing); full-suite gate at end of this plan also green.
- **Committed in:** `50846b0` (fix), `6837052` (gap record)

---

**Total deviations:** 1 auto-fixed (Rule 1 bug fix)
**Impact on plan:** Necessary for DOC-04/DOC-05's Russian-only UI constraint; no scope creep — user explicitly directed the fix into this phase.

## Issues Encountered

**Test-gate tooling fragility (process issue, not a code defect):** while running the final full-suite gate, an earlier duplicate background invocation of the test-runner script was left running from a prior interrupted attempt. Two of its integration-test result lines (`pdf_text_extract`, `phase06_stubs`) were reported as `FAIL_OR_TIMEOUT(137)` by a stale 60-second watcher subshell that fired against a reused PID after the actual test run had already completed successfully — both tests' own captured logs show `test result: ok` with 0 failures (`1 passed` and `18 passed` respectively). These two were corrected to PASS based on their logged output rather than the watcher's spurious kill signal. Cleaning up the stale orphan process itself briefly disrupted the *active* run's shared session, which was resumed with a second, watcher-free script for the remaining 21 test binaries — all passed cleanly. No test code or product code was touched by this; it is purely an artifact of the execution session's process management, not the plan's `<files>` scope.

## Final Full-Suite Regression Gate

`pnpm --dir ui build` — clean, 642 modules transformed, no errors (pre-existing Svelte lint warnings only, unrelated to this phase).

`TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app` — run per test binary with `--test-threads=1`, **excluding `auth_remember_cookie.rs`** (the one pre-existing hanging test binary in this crate, per project convention — see project memory `workspace-test-hangs-auth-remember-cookie`).

- **Lib tests:** 167 passed, 0 failed.
- **Integration tests:** 84 binaries run (every file under `crates/trackly-app/tests/*.rs` except `auth_remember_cookie.rs`), **472 test-functions passed, 0 failed**, 2 intentionally `ignored` (in `pdf_determinism`, unrelated to this phase).
- **Excluded:** `auth_remember_cookie` (1 binary) — not run, per the known pre-existing hang documented in project memory. Not part of this phase's scope.
- **Total:** 639 passing test-functions across lib + all included integration binaries, zero failures.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Phase 34 (document-header) is functionally complete: DOC-04, DOC-05, DOC-06 human-verified on both transports, all print-form headers unified, upgrade-delivery mechanism proven on a real install.
- `target/debug/templates/` now contains only the canonical `_header.html` (materialized at runtime) — no stray hand-edited reference copies remain in the build directory.
- Backlog carries forward: `DOC-12` (UI consumer for `templates_status`), and Phase 35 continues the DOC- series (DOC-07 through DOC-11, act layout/signature-block work).
- No blockers for Phase 35.

---
*Phase: 34-document-header*
*Completed: 2026-08-09*
