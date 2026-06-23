---
phase: 12-cartridge-request-interconnection
plan: 15
subsystem: ui
tags: [svelte, requests, lifecycle, rbac, confirmation-modal]

# Dependency graph
requires:
  - phase: 12-cartridge-request-interconnection
    provides: "requests_delete/requests_cancel Tauri commands + HTTP routes + RequestService::delete/cancel with BOLA-guard (plan 12-14)"
provides:
  - "requests.delete()/requests.cancel() frontend API wrappers"
  - "Удалить button (Specialist/Admin, any status) with confirmation modal in RequestDetail.svelte"
  - "Отменить заявку button (Employee author, open status only) with confirmation modal in RequestDetail.svelte"
affects: [requests, gap-closure, 12-human-uat]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Lifecycle action handlers (handleDeleteConfirm/handleCancelConfirm) follow the existing handleRejectConfirm submit/toast/onTransition pattern"
    - "isOwnRequest derived value is a UI-only cosmetic gate; server-side BOLA-guard (RequestService::cancel) is the authoritative check"

key-files:
  created: []
  modified:
    - ui/src/features/requests/api.ts
    - ui/src/features/requests/RequestDetail.svelte

key-decisions:
  - "Удалить button rendered as an unconditional section gated only on isSpecialist, independent of the existing status-branching {#if isAdRegister}/{:else if isSpecialist} blocks — visible for every status including ad_register requests (no explicit UAT exclusion found)"
  - "Отменить заявку added as a new {:else if isOwnRequest && request.status === 'open'} branch in the existing if/else-if chain, reached only when !isAdRegister && !isSpecialist (i.e. Employee) by construction of the parent chain"

requirements-completed: [GAP-12-07]

# Metrics
duration: 5min
completed: 2026-06-24
---

# Phase 12 Plan 15: Request Lifecycle Management UI (GAP-12-07/A4) Summary

**Added `requests.delete()`/`requests.cancel()` API wrappers and corresponding "Удалить" (Admin/Manager, any status) and "Отменить заявку" (Employee author, open-only) buttons with confirmation modals to `RequestDetail.svelte`.**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-06-23T17:52:00Z (approx, after prior plan 12-12 commit)
- **Completed:** 2026-06-23T17:56:04Z
- **Tasks:** 3 (planned) — executed as 2 commits (Task 1 standalone; Tasks 2+3 combined since both touch the same file's shared if/else-if chain and modal section)
- **Files modified:** 2

## Accomplishments
- `requests.delete(id, version)` and `requests.cancel(id, version)` thin wrappers over the plan-12-14 backend endpoints, typed against the real `requests_delete`/`requests_cancel` Tauri signatures.
- "Удалить" button visible to Admin/Manager (isSpecialist) regardless of request status (open/in_progress/completed/rejected/cancelled, including ad_register requests), with an irreversibility-warning confirmation modal.
- "Отменить заявку" button visible only to the request's own author (Employee), only while status is `open`, with a confirmation modal explaining that a new request must be created to continue.
- Both handlers follow the established `handleRejectConfirm` pattern: submitting flag, `pushToast` on success/error, `onTransition()` refresh, modal close on success.

## Task Commits

Each task was committed atomically:

1. **Task 1: API wrapper — requests.delete()/cancel()** - `aa5e964` (feat)
2. **Tasks 2+3: Delete/Cancel buttons + confirmation modals in RequestDetail.svelte** - `31d10ce` (feat)

**Plan metadata:** (this commit, following)

## Files Created/Modified
- `ui/src/features/requests/api.ts` - Added `delete`/`cancel` methods to the `requests` object.
- `ui/src/features/requests/RequestDetail.svelte` - Added `deleteModalOpen`/`deleteSubmitting`/`cancelModalOpen`/`cancelSubmitting` state, `isOwnRequest` derived, `handleDeleteConfirm`/`handleCancelConfirm` handlers, unconditional "Удалить" section for `isSpecialist`, new "Отменить заявку" branch for the Employee-author/open-status case, and two new confirmation `Modal`s.

## Decisions Made
- Combined Task 2 and Task 3 into a single commit: both modify the same `if`/`else if` chain and the same modal section of `RequestDetail.svelte`; splitting them would have produced an intermediate commit with an incomplete (Task 2 only) chain that doesn't compile cleanly against the plan's described final structure. Task 1 (the API wrapper, a separate file) remained its own commit as planned.
- Per the plan's explicit instruction, did not check or special-case `isAdRegister` for the delete button beyond what `isSpecialist` already implies — the UAT text says "заявки" without exclusions, so ad_register requests get the delete button too.
- Simplified the plan's suggested `!isAdRegister && isOwnRequest && request.status === 'open'` condition to `isOwnRequest && request.status === 'open'` since the surrounding `{#if isAdRegister} ... {:else if isSpecialist} ... {:else if ...}` chain already guarantees `!isAdRegister && !isSpecialist` by the time this branch is reached; added a comment explaining why the shorter condition is still correct.

## Deviations from Plan

None — plan executed as written, with the one combined-commit decision documented above (not a deviation rule, just atomic-commit organization given the two tasks share one file region).

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- GAP-12-07/A4 frontend is now complete (backend was already done in plan 12-14). Combined with 12-14, this closes GAP-12-07 in full.
- Manual UAT still pending: Admin/Manager delete-any-status flow and Employee browser-based cancel-own-open-request flow should be verified live (per plan's `<verification>` items 3-4) — `pnpm --dir ui build` was run, so the LAN browser will serve the current build.
- No blockers for closing out the remaining Phase 12 UAT gaps (A1-A5 / GAP-12-04..08) tracked elsewhere in Round 2 gap-closure plans.

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-24*

## Self-Check: PASSED

- FOUND: ui/src/features/requests/api.ts
- FOUND: ui/src/features/requests/RequestDetail.svelte
- FOUND: commit aa5e964 (Task 1)
- FOUND: commit 31d10ce (Tasks 2+3)
