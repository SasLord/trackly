---
phase: 12-cartridge-request-interconnection
plan: 03
subsystem: ui
tags: [svelte5, runes, frontend, cartridges, requests]

# Dependency graph
requires:
  - phase: 12-cartridge-request-interconnection (Wave 1, Plan 01)
    provides: "CartridgeFilter.installable_only (SQL-level state_id IN (1,2) gate), RequestDto.printerLocation (joined location, camelCase wire)"
  - phase: 12-cartridge-request-interconnection (Wave 2, Plan 02)
    provides: "RequestService.transition() reads the linked cartridge pre-write and enriches history notes; completed_cartridge_id persistence confirmed; RBAC closure (T-12-01)"
provides:
  - "CartridgeSelect.svelte — flat (no optgroup) cartridge picker for OperationModal's request-centric install flow"
  - "OperationModal.svelte effectiveCartridge pattern — single code path serves both cartridge-centric (menu) and request-centric (RequestDetail) install entries (D-08)"
  - "OperationModal new props: cartridgeModelId, prefillLocation, prefillGivenToName; onSuccess(cartridgeId: number) signature"
  - "RequestDetail.handleInstallSuccess(cartridgeId) — wires the real linkedCartridgeId into requests.transition(complete) instead of null (D-06 end-to-end)"
affects: [request-cartridge-install-ux, future-cartridge-picker-reuse]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "effectiveCartridge = $derived(cartridge ?? selectedCartridge) — lets a single set of buildPayload/validate/canSubmit functions serve two distinct entry points (prop-provided vs internally-selected) without branching every call site"
    - "Conditional-fetch $effect gated on three conditions (open && op==='install' && cartridge===null) — mirrors RequestDetail's existing history-load effect (.then/.catch/.finally with a loading flag), avoids firing the new network call on the unaffected cartridge-centric path"

key-files:
  created:
    - ui/src/lib/components/CartridgeSelect.svelte
  modified:
    - ui/src/features/cartridges/OperationModal.svelte
    - ui/src/features/requests/RequestDetail.svelte

key-decisions:
  - "CartridgeSelect copies GroupedPrinterSelect's select/caret SCSS 1:1 minus the optgroup rule — DISC-03 (cartridges have no natural location-group) makes a flat list correct, not a missing feature"
  - "effectiveCartridge resolves at the top of OperationModal's script and is threaded through every existing op-branch (buildPayload/validate-adjacent canSubmit/handleSubmit) instead of adding a second cartridge-resolution branch per function — keeps the diff minimal and avoids divergent behavior between the two entry points"
  - "Cartridge-list effect is unconditionally re-evaluatable (Svelte effect dependency tracking on open/op/cartridge) rather than gated by an explicit cleanup flag — matches the project's existing async-effect convention in RequestDetail.svelte's history loader"

requirements-completed: [D-01, D-02, D-03, D-04, D-05, D-06, D-08]

# Metrics
duration: 18min
completed: 2026-06-22
---

# Phase 12 Plan 03: Frontend cartridge selector + request-detail wiring Summary

**New `CartridgeSelect.svelte` flat picker plus an `effectiveCartridge` derived-state pattern in `OperationModal.svelte` let a specialist pick a physical cartridge straight from a `cartridge_replace` request, with «Кому отдал»/«Расположение» pre-filled from the request and a real `linkedCartridgeId` now flowing into `complete` — closing out the cartridge-request interconnection feature end-to-end.**

## Performance

- **Duration:** 18 min
- **Started:** 2026-06-22T05:09:53Z (continuation from prior session, per STATE.md)
- **Completed:** 2026-06-22T05:15:35Z
- **Tasks:** 3 auto + 1 checkpoint (auto-approved under AUTO_MODE) = 4/4
- **Files modified:** 3 (1 created, 2 modified)

## Accomplishments
- `CartridgeSelect.svelte` — new flat select component (no optgroup), rendering `{code} — {brand} {name} ({state})` per option, NULL-safe, with the DISC-02 empty state "Нет подходящих картриджей на складе".
- `OperationModal.svelte` now serves both install entry points off one code path: `effectiveCartridge = $derived(cartridge ?? selectedCartridge)` feeds `isDrum`, `defaultStateId`, `buildPayload()`, `validate()`-gated `canSubmit`, and `handleSubmit()` uniformly. The cartridge-centric entry (cartridge menu → «Установить в принтер») is byte-for-byte unaffected (D-08) because `cartridge` stays non-null there and the new picker/effect/props are all gated on `cartridge === null`.
- New `$effect` loads `cartridges.list({status_id: 1, installable_only: true, model_id: cartridgeModelId ?? null, kind_id: null, search: null, include_deleted: false}, {offset: 0, limit: 200})` only for the request-centric branch — matches the project's existing `.then/.catch/.finally` async-effect convention (`RequestDetail`'s history loader).
- `onSuccess` signature changed from `() => void` to `(cartridgeId: number) => void`; `RequestDetail.handleInstallSuccess(cartridgeId)` now passes the real id as `linkedCartridgeId` into `requests.transition({op: 'complete', ...})`, completing the D-06 chain Wave 2 built (`completedCartridgeId` persistence + human-readable history snapshot).
- `RequestDetail`'s `OperationModal` invocation gained `cartridgeModelId`, `prefillLocation`, `prefillGivenToName` props sourced from `request.cartridgeModelId`/`request.printerLocation`/`request.requesterName` — both prefilled fields remain editable (reset effect seeds them via `?? ''`, no `readonly`/`disabled` attribute added).

## Task Commits

Each task was committed atomically:

1. **Task 1: CartridgeSelect.svelte — новый компонент выбора картриджа** - `faed70a` (feat)
2. **Task 2: OperationModal.svelte — селектор + auto-prefill props (D-01..D-05)** - `8f394a3` (feat)
3. **Task 3: RequestDetail.svelte — проброс данных заявки + linkedCartridgeId (D-06)** - `9e367fd` (feat)
4. **Task 4: Human-verify checkpoint** — auto-approved under AUTO_MODE (no code change; see Checkpoint Handling below)

**Plan metadata:** _pending — final docs commit follows this summary_

## Files Created/Modified
- `ui/src/lib/components/CartridgeSelect.svelte` - New flat cartridge `<select>` component, modeled on `GroupedPrinterSelect.svelte` minus optgroup grouping
- `ui/src/features/cartridges/OperationModal.svelte` - `effectiveCartridge` derived state; `selectedCartridge`/`cartridgeOptions`/`cartridgeListLoading` internal state; new props (`cartridgeModelId`, `prefillLocation`, `prefillGivenToName`); reset effect prefills `givenToName`/`location`; new cartridge-list-loading `$effect`; install-branch JSX renders `CartridgeSelect` only when `cartridge === null`; `onSuccess(cartridgeId)` signature
- `ui/src/features/requests/RequestDetail.svelte` - `handleInstallSuccess(cartridgeId)` passes `linkedCartridgeId: cartridgeId` instead of `null`; `OperationModal` JSX gains the three new prefill/filter props

## Decisions Made
- `effectiveCartridge` chosen over per-branch null-checks (e.g. `cartridge ?? selectedCartridge` resolved once vs. resolved inline in `buildPayload`/`handleSubmit`/`canSubmit` separately) — single source of truth, smaller diff, and impossible for the two entry points to drift out of sync.
- Cartridge picker only renders/loads for `op === 'install' && cartridge === null` — `to_refill` (which shares the same JSX branch in the template) never shows the picker or fires the list-load effect, matching the plan's scope (D-01..D-08 only target the install-from-request flow).
- Checkpoint Task 4 (`type="checkpoint:human-verify"`, `gate="blocking"`) auto-approved per this run's explicit AUTO_MODE instructions — `gate="blocking"` is not the package-legitimacy `gate="blocking-human"` exclusion, and `.planning/config.json` has `workflow.auto_advance: true`. All automated verification (`svelte-check`, `build`) passed; the happy path, DISC-02 empty state, and D-08 regression were confirmed by direct code inspection (CartridgeSelect render is strictly gated on `cartridge === null`; old entry point's `cartridge` prop is always non-null).

## Deviations from Plan

None - plan executed exactly as written. All three tasks' acceptance criteria (svelte-check 0 errors, build success, grep markers present) were met without needing fixes.

## Issues Encountered

None. `pnpm --dir ui lint` reports 22 pre-existing errors (theme-init.js, ChartWidget.svelte, EmployeeLayout.svelte, ws.ts, AccessDenied.svelte, etc.) — confirmed via targeted grep that none originate in the three files this plan touched (`CartridgeSelect.svelte`, `OperationModal.svelte`, `RequestDetail.svelte`). Left untouched per deviation-rules scope boundary; not newly introduced by this plan.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- D-01 through D-08 are all implemented end-to-end: backend filters (Wave 1) → service wiring/history enrichment/RBAC (Wave 2) → frontend selector/prefill/linkedCartridgeId wiring (Wave 3, this plan). The cartridge-request interconnection feature is functionally complete.
- The human-verify checkpoint's interactive scenarios (live browser/desktop walkthrough of create→accept→install→complete→history, DISC-02 empty-list UX, D-08 regression on the cartridge-menu entry, optional LAN-browser repeat) were not run against a live UI session in this autonomous execution — they were auto-approved per AUTO_MODE and corroborated by static code review + automated `svelte-check`/`build` gates. If the user wants an actual interactive pass, re-run the steps in `12-03-PLAN.md` Task 4's `<how-to-verify>` manually.
- No blockers for closing Phase 12.

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-22*

## Self-Check: PASSED

All created/modified files verified present on disk:
- ui/src/lib/components/CartridgeSelect.svelte — FOUND
- ui/src/features/cartridges/OperationModal.svelte — FOUND
- ui/src/features/requests/RequestDetail.svelte — FOUND
- .planning/phases/12-cartridge-request-interconnection/12-03-SUMMARY.md — FOUND

All commit hashes verified present in git log:
- faed70a — FOUND
- 8f394a3 — FOUND
- 9e367fd — FOUND
- d33b271 — FOUND
