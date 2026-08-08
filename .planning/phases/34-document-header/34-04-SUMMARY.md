---
phase: 34-document-header
plan: 04
subsystem: ui
tags: [svelte, org-settings, template-editor, textarea, full-name]

# Dependency graph
requires:
  - phase: 34-01
    provides: "org_settings.full_name column + OrgPatch.full_name / OrgSettingsDto.full_name DTO fields (settings_get_org / settings_save_org_fields)"
  - phase: 34-03
    provides: "org.full_name wired into all render contexts via org_full_name_html; list_all_for_editor filters _-prefixed partials"
provides:
  - "OrgSettings.svelte: user-facing multiline 'Полное юридическое наименование' field (shared Textarea.svelte), round-tripping through settings_get_org / settings_save_org_fields"
  - "TemplateEditor.svelte: org.full_name documented in all three VARIABLES_BY_KIND arrays (act_handover, act_acceptance, report)"
affects: [34-05, 34-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Shared Textarea.svelte usage convention confirmed: value={x} + oninput={(v) => (x = v)}, one-way + callback, NOT bind:value (Textarea's own internal bind:value is private to the component)"

key-files:
  created: []
  modified:
    - ui/src/features/settings/OrgSettings.svelte
    - ui/src/features/settings/TemplateEditor.svelte

key-decisions:
  - "Placed the new full_name field immediately after address-line2 and before phone in the markup, matching the plan's explicit insertion point and mirroring the address_line2 sibling's 4-touchpoint shape (interface field, $state, load, save payload) plus the markup block."

requirements-completed: [DOC-05]

duration: ~15min
completed: 2026-08-09
---

# Phase 34 Plan 04: OrgSettings full_name field + TemplateEditor variable docs Summary

**Added a user-facing multiline "Полное юридическое наименование" field to Settings → Организация using the shared Textarea.svelte component, wired to the full_name DB column from Plan 34-01, and documented org.full_name in all three TemplateEditor variable-reference lists.**

## Performance

- **Duration:** ~15 min
- **Tasks:** 2 completed
- **Files modified:** 2

## Accomplishments
- `OrgSettings.svelte` now has a `fullName` state field, round-tripping through `settings_get_org` (`dto.full_name`) and `settings_save_org_fields` (`patch.full_name`), rendered via the shared `Textarea.svelte` component (not a raw `<textarea>`, not `Input.svelte`) — matching the multiline-by-design intent from Plan 34-01/34-02/34-03.
- `TemplateEditor.svelte`'s `VARIABLES_BY_KIND` now documents `org.full_name` in all three kind arrays (`act_handover`, `act_acceptance`, `report`), since `_header.html` is `{% include %}`-d by all three document kinds (Plan 34-03).

## Task Commits

Each task was committed atomically:

1. **Task 1: fullName field in OrgSettings.svelte (D-02, shared Textarea)** - `64d8c5a` (feat)
2. **Task 2: Document org.full_name in TemplateEditor's variable list (all 3 kinds)** - `8c6ecef` (docs)

## Files Created/Modified
- `ui/src/features/settings/OrgSettings.svelte` - import `Textarea`; `full_name: string` added to local `OrgSettingsDto` interface; `fullName` `$state`; `loadOrg()` populates it from `dto.full_name`; `saveOrg()` sends `full_name: fullName` in the patch payload; new `<div class="form-field form-field--full">` block (label + `Textarea`, `rows={3}`) inserted after the address-line2 field and before the phone field — reuses existing `.form-field`/`.form-label` SCSS, no new CSS
- `ui/src/features/settings/TemplateEditor.svelte` - `{ code: 'org.full_name', desc: 'полное юридическое наименование (многострочное)' }` added immediately after each kind's existing `org.name` entry, in all three `VARIABLES_BY_KIND` arrays

## Decisions Made
- Followed the plan's exact insertion point and Textarea usage convention (one-way `value` + `oninput` callback, not `bind:value`) — no deviation needed since the interfaces block in the plan already confirmed the component's prop shape against the live source.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Verification

- `pnpm --dir ui exec svelte-check --output human` — 0 errors, 48 pre-existing warnings in unrelated files (none attributable to `OrgSettings.svelte` or `TemplateEditor.svelte`).
- `pnpm --dir ui build` — succeeded (pre-existing unused-CSS-selector warning in `ActFormItemsTable.svelte`, unrelated to this plan).
- `grep -c "fullName" ui/src/features/settings/OrgSettings.svelte` → 5 (state decl, interface reference via `dto.full_name`/`patch.full_name`, load, save payload, markup usage — matches acceptance criteria "at least 5").
- `grep -c "Textarea" ui/src/features/settings/OrgSettings.svelte` → 2 (import + usage).
- `grep -c "<textarea" ui/src/features/settings/OrgSettings.svelte` → 0 (no raw element introduced).
- `grep -c "org.full_name" ui/src/features/settings/TemplateEditor.svelte` → 3 (one per kind array).

## Next Phase Readiness

- A user can now type a multi-line legal name into Settings → Организация, save it, reload, and see it persisted through the shared `Textarea` component.
- `org.full_name` is discoverable in the TemplateEditor's variable reference for every document kind that can reference it.
- No blockers for Plan 34-05/34-06.

## Self-Check: PASSED

- FOUND: ui/src/features/settings/OrgSettings.svelte
- FOUND: ui/src/features/settings/TemplateEditor.svelte
- FOUND commit: 64d8c5a (Task 1)
- FOUND commit: 8c6ecef (Task 2)

---
*Phase: 34-document-header*
*Completed: 2026-08-09*
