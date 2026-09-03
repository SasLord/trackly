---
phase: 40-movement-history
plan: 23
subsystem: ui
tags: [svelte5, runes, cartridges, form-validation, place-picker]

# Dependency graph
requires:
  - phase: 40-movement-history (plan 21)
    provides: "cartridges_sqlite::transition_in_tx step 5a — Install with an explicit cartridge place backfills a placeless printer's devices.place_id, and D-13 server-side auto-resolve of place_id from the printer's own place — this is what makes the client-side placeId requirement on install obsolete when a printer is selected"
  - phase: 40-movement-history (plan 22)
    provides: "transition_in_tx auto-return branch fallback — when previous_cartridge_place_id is None, resolved_place_id derives from last_known_storage_place_in_tx (last is_storage=1 movement destination) instead of unconditionally clearing to NULL; no-history cartridges still resolve to NULL"
provides:
  - "OperationModal.svelte validate(): install's placeId requirement is now scoped to the legacy cartridge-centric path only (effectivePrinterId === undefined); to_refill keeps place mandatory unconditionally (no printer context exists for it)"
  - "Field-hint under 'Место (предыдущий картридж)' explaining the Plan 40-22 auto-return fallback (empty field derives last known storage place, not a silent clear)"
  - "Extended field-hint on the main 'Место' field for op=install explaining the field is optional when a printer is selected and has no place yet, and that filling it here also backfills the printer"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Client-side validation as UX convenience, not a security gate: the placeId requirement relaxation is safe specifically because the server (Plan 40-21/40-22) was already the source of truth accepting place_id: None independently of what the form enforced — this plan only removes a stale client-side block that no longer matched the accepted server contract, it does not open new input surface (per threat register T-40-23-01, disposition: accept)"

key-files:
  created: []
  modified:
    - ui/src/features/cartridges/OperationModal.svelte

key-decisions:
  - "Split the combined `op === 'install' || op === 'to_refill'` placeId check into per-op branches rather than a single relaxed condition covering both, because to_refill has no printer_device_id in its payload at all (D-08 legacy path always applies to it) — its place requirement must stay unconditional, only install's requirement is conditional on effectivePrinterId"
  - "Main 'Место' field-hint for install only swaps to the new 'optional, backfills printer' text when effectivePrinterId !== undefined AND placeId === null (printer selected but has no place to auto-resolve from); when a printer already has a place (placeId gets auto-filled) or no printer is selected at all, the original hint text is preserved unchanged — avoids over-promising the backfill behavior in cases where it doesn't apply"

requirements-completed: [HST-01, HST-02]

# Metrics
duration: ~15min
completed: 2026-09-03
---

# Phase 40 Plan 23: Install Place-Optional + Auto-Return Hint Summary

**`OperationModal.svelte`'s install validation no longer blocks submit when a printer is selected but placeless, and a new field-hint explains the Plan 40-22 auto-return storage-place fallback instead of leaving operators guessing what an empty «Место (предыдущий картридж)» field does.**

## Performance

- **Duration:** ~15 min
- **Completed:** 2026-09-03T01:45:03Z
- **Tasks:** 1/1
- **Files modified:** 1

## Accomplishments

- `validate()`'s combined `op === 'install' || op === 'to_refill'` placeId check is now split: `to_refill` keeps place mandatory unconditionally (it has no printer context); `install` only requires place when `effectivePrinterId === undefined` (the legacy cartridge-centric path, D-08) — when a printer is selected, the client no longer blocks submit on an empty place, matching the server's already-accepted contract (D-13 auto-resolve + Plan 40-21 step 5a backfill).
- Added a `field-hint` under the previous-cartridge's `PlacePicker` (`op-prev-place`) that explains Plan 40-22's actual shipped fallback: leaving the field empty derives the cartridge's last known storage place from its movement history, falling back to "место останется не указано" only when no such history exists — worded to match the real behavior (not "always restores to storage"), verified against 40-22's SUMMARY before writing.
- Extended the main "Место" field-hint for `op === 'install'`: when a printer is selected and it currently has no place (`effectivePrinterId !== undefined && placeId === null`), the hint now reads "Необязательно: у принтера пока не указано место. Если укажете здесь — оно будет проставлено и принтеру" (referencing Plan 40-21 Task 2's reverse-write backfill); the legacy no-printer path keeps its original "Укажите рабочее место или кабинет (не склад)" text unchanged.

## Task Commits

Each task was committed atomically:

1. **Task 1: Место необязательно при установке в принтер + подсказка автоподстановки** - `0dcd8e03` (feat)

**Plan metadata:** commit created after this SUMMARY (see below)

## Files Created/Modified

- `ui/src/features/cartridges/OperationModal.svelte` - split validate()'s install/to_refill placeId branch; added a field-hint to `op-prev-place`'s PlacePicker explaining the auto-return fallback; extended the main "Место" field's install hint with a conditional "printer has no place yet, optional" variant.

## Decisions Made

See `key-decisions` in frontmatter. Notably: the hint wording for `op-prev-place` deliberately mirrors the exact shipped semantics of Plan 40-22 (fallback to last **storage** place, NULL preserved when no history) rather than a more generic "будет восстановлено" phrasing that would over-promise for cartridges with no movement history.

## Deviations from Plan

None - plan executed exactly as written. The `validate()` condition matches the plan's specified `if (op === 'install' && effectivePrinterId === undefined && placeId === null)` verbatim (implemented as an `else if` branch alongside a separate unconditional `to_refill` check, functionally identical to the plan's described split); both new hints match the plan's specified wording and placement.

## Issues Encountered

None. `pnpm --dir ui svelte-check` — 0 errors (60 pre-existing warnings elsewhere, none new in this file). `pnpm --dir ui lint` — full chain including the new `check-report-type-parity` and `check-print-idempotency` gates (from plans 40-25/40-27) all green, `check-privacy` also passed on the commit.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- UAT-40 gaps "cartridge-does-not-follow-printer" (test 5, item 3) and "return-to-stock-empty-place-field" (test 16, item 3) are now closed on the UI side, completing the pairing with backend plans 40-21/40-22.
- Manual verification (per plan's `<verification>` section) still recommended after merge: install a cartridge into a printer without a place and confirm the form no longer blocks submit; confirm the new hints render as expected in a running app (svelte-check/build do not catch rune runtime behavior per project convention).
- No blockers identified. This was the last of the two-plan UI/backend pairing in the current wave; all wave-3 gap-closure plans (40-21 through 40-27) are now complete.

---
*Phase: 40-movement-history*
*Completed: 2026-09-03*

## Self-Check: PASSED

- FOUND: ui/src/features/cartridges/OperationModal.svelte
- FOUND commit: 0dcd8e03
- FOUND commit: a73d7ad3
