---
phase: 12-cartridge-request-interconnection
plan: 16
subsystem: ui
tags: [svelte, printers, list-row, ux]

# Dependency graph
requires:
  - phase: 06-snmp
    provides: PrinterDto.deviceLocation (backend list-DTO already joins locations and exposes deviceLocation: string | null)
provides:
  - PrinterListRow.svelte bottom row now shows device location (left) and IP/USB/"—" (right) instead of only IP
affects: [printers]

# Tech tracking
tech-stack:
  added: []
  patterns: []

key-files:
  created: []
  modified:
    - ui/src/features/printers/PrinterListRow.svelte

key-decisions:
  - "Renamed locationLabel (which actually rendered IP, a stale name from the original component) to ipText; introduced a new locationText derived from printer.deviceLocation for the left column"
  - "IP/USB column pushed right via margin-left: auto on .row-ip rather than restructuring .bottom into a flex space-between container — keeps toner-hint's existing conditional position between location and IP unaffected"

patterns-established: []

requirements-completed: [PRN-07]

# Metrics
duration: 2min
completed: 2026-06-24
---

# Phase 12 Plan 16: Printer list row shows device location Summary

**`PrinterListRow.svelte` bottom row now renders device location on the left and IP/USB/"—" right-aligned, using the already-wired `PrinterDto.deviceLocation` field — pure frontend fix, no backend changes**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-06-24T15:54:12Z
- **Completed:** 2026-06-24T15:55:34Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- Operators can now see where a printer is physically located directly in the list, without opening the detail card
- Fixed a misleading variable name (`locationLabel` actually held the IP, not the location) that masked the missing field
- IP/USB/"—" right column preserved (functionally unchanged in content, only repositioned and renamed for clarity)

## Task Commits

1. **Task 1: Показать Расположение слева + IP/USB справа в строке списка принтеров** - `d8ac384` (feat)

**Plan metadata:** (this commit, see final commit below)

## Files Created/Modified
- `ui/src/features/printers/PrinterListRow.svelte` - Added `locationText` derived (from `printer.deviceLocation`, fallback `—`), renamed the old IP-holding `locationLabel` to `ipText`, restructured `.bottom` row markup to render location left / toner-hint / IP right, added `.row-location` (ellipsis truncation) and `.row-ip` (`margin-left: auto`, `tabular-nums`) SCSS rules replacing the old single `.location` rule

## Decisions Made
- Kept `tonerSummary`'s existing conditional render position (between location and IP) rather than relocating it, to minimize layout disruption and stay within the plan's "don't break existing conditional render" instruction
- Used `margin-left: auto` on `.row-ip` (already-flex `.bottom` container) instead of switching to `justify-content: space-between`, since `tonerSummary` needs to stay in natural flow between the two ends when present

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. The Edit tool in this environment required a tool-native `Read` of the file (a prior `Bash cat` read did not satisfy the precondition) before the first `Edit` call would succeed — resolved by re-reading the file with the `Read` tool, no functional impact.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

GAP-12-09 (B1) closed. No blockers. Backend untouched (`device_location` was already present in the list-DTO since Phase 6); this was purely a presentation-layer fix. `svelte-check` (0 errors) and `pnpm --dir ui build` both green.

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-24*

## Self-Check: PASSED

- FOUND: .planning/phases/12-cartridge-request-interconnection/12-16-SUMMARY.md
- FOUND: ui/src/features/printers/PrinterListRow.svelte
- FOUND commit: d8ac384
- FOUND commit: def221d
