---
phase: 35-act-handover-body
plan: 01
subsystem: infra
tags: [rust, minijinja, html-templates, legacy-defaults, template-preview]

# Dependency graph
requires:
  - phase: 34-document-header
    provides: post-Phase-34 act_handover.html/act_acceptance.html bodies (the exact bytes this plan snapshots) and the v20/v21 _legacy_defaults precedent this plan extends
provides:
  - "_legacy_defaults/v22/{act_handover,act_acceptance}.html — byte-identical snapshot of the pre-Phase-35 act body, so installed copies still auto-upgrade after the body changes in Plan 02+"
  - "KNOWN_LEGACY_DEFAULTS registry extended with the v22 entry in both act_handover.html and act_acceptance.html slices"
  - "demo_context_for_kind's '_' (act_handover) branch now carries act.giver_name, matching production render context ahead of the template body change"
affects: [35-02-act-handover-body-text, 35-03, 35-04, 35-05]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Legacy-defaults snapshot BEFORE body change (pattern from Phase 16/34), not after"]

key-files:
  created:
    - crates/trackly-app/templates/_legacy_defaults/v22/act_handover.html
    - crates/trackly-app/templates/_legacy_defaults/v22/act_acceptance.html
  modified:
    - crates/trackly-app/src/pdf/html_templates.rs
    - crates/trackly-app/src/services/template_service.rs

key-decisions:
  - "Snapshot v22 contains only act_handover.html and act_acceptance.html (not report.html/_header.html), matching the plan's explicit scope — report.html and _header.html are untouched by Phase 35"
  - "demo_context_for_kind's giver_name literal reuses the existing fictional name 'Иванов И.И.' already present in the neighboring act_acceptance branch (privacy constraint C-07)"

patterns-established: []

requirements-completed: [DOC-08]

# Metrics
duration: ~15min
completed: 2026-08-11
---

# Phase 35 Plan 01: Legacy-defaults snapshot + preview context prep Summary

**Snapshotted pre-Phase-35 act body into `_legacy_defaults/v22/`, registered it in `KNOWN_LEGACY_DEFAULTS`, and added `act.giver_name` to the template-editor demo context ahead of Plan 02's body rewrite.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-08-11T12:12:51Z (approx, per STATE.md session marker)
- **Completed:** 2026-08-11T12:20:00Z (approx)
- **Tasks:** 2
- **Files modified:** 4 (2 new snapshot files, 2 modified source files)

## Accomplishments
- Created `_legacy_defaults/v22/act_handover.html` and `_legacy_defaults/v22/act_acceptance.html`, byte-identical to the current (post-Phase-34) HEAD bodies of those two templates, via `cp` (no manual edits)
- Registered `v22` as a third `include_str!` entry in the `act_handover.html` and `act_acceptance.html` slices of `KNOWN_LEGACY_DEFAULTS`, leaving `report.html`/`_header.html` untouched
- Added `"giver_name": "Иванов И.И."` to `demo_context_for_kind`'s `_` (act_handover) branch, mirroring the neighboring `act_acceptance` branch's existing `document.giver_name`, so the live template preview (`Settings → Шаблоны`) will not break under `UndefinedBehavior::Strict` once Plan 02 wires `act.giver_name` into the template body

## Task Commits

Each task was committed atomically:

1. **Task 1: Snapshot `_legacy_defaults/v22/` and register in KNOWN_LEGACY_DEFAULTS** - `e0d2dca` (feat)
2. **Task 2: Add `act.giver_name` to act_handover demo preview context** - `1249e5e` (feat)

**Plan metadata:** pending (see final commit below)

## Files Created/Modified
- `crates/trackly-app/templates/_legacy_defaults/v22/act_handover.html` - byte-identical snapshot of pre-Phase-35 act_handover.html body
- `crates/trackly-app/templates/_legacy_defaults/v22/act_acceptance.html` - byte-identical snapshot of pre-Phase-35 act_acceptance.html body
- `crates/trackly-app/src/pdf/html_templates.rs` - `KNOWN_LEGACY_DEFAULTS` gained a third `include_str!` element in the `act_handover.html` and `act_acceptance.html` slices, pointing at the new v22 snapshots
- `crates/trackly-app/src/services/template_service.rs` - `demo_context_for_kind`'s `_` branch (act_handover fallback) gained `"giver_name": "Иванов И.И."` next to the existing `"receiver_name"`

## Decisions Made
- None beyond what the plan specified — both tasks executed exactly as written, using the existing v21 slice and neighboring act_acceptance branch as direct templates to copy.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. Both automated verification commands passed on first run:
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib every_default_template_has_a_known_legacy_defaults_entry` → ok
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib validate_preview_act_handover_returns_html_with_title_marker` → ok

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 02 (act text/signature-block body rewrite) can now safely change the bytes of `act_handover.html`/`act_acceptance.html` and read `act.giver_name` in the template: installed copies will recognize the current bodies as "known prior default" (v22) and receive the upgrade, and the live template-editor preview will not break under `UndefinedBehavior::Strict` when `act.giver_name` is newly referenced. No blockers.

---
*Phase: 35-act-handover-body*
*Completed: 2026-08-11*

## Self-Check: PASSED

All created files and commit hashes verified present.
