---
phase: 39-place-tree
plan: 16
subsystem: ui
tags: [svelte, svelte5-runes, place-tree, cartridges, place-picker]

# Dependency graph
requires:
  - phase: 39-place-tree plan 12
    provides: "places_* Tauri/HTTP commands + PlaceDto/PlaceNewDto/PlacePathDto bindings.ts types"
  - phase: 39-place-tree plan 13
    provides: "PlacePicker.svelte — value/onChange/id/disabled/invalid props, default apiCall-backed fetchers"
  - phase: 39-place-tree plan 09
    provides: "Cartridges backend migrated to place_id/full_path (CartridgeDto/CreateDto/TransitionPayload's 5 variants); cartridges_suggest_location removed; cartridge_storage_place_ids exposed (D-11.4); PrinterDto.devicePlaceId added for Install prefill"
provides:
  - "Cartridge create/edit form (CartridgeFormBody.svelte) selects place via PlacePicker bound to place_id — no freeform location text field remains on the cartridge family"
  - "OperationModal.svelte's 5 transition ops (Install/ReturnToStock/ToRefill/FromRefill/WriteOff) select place via PlacePicker bound to place_id; Install prefills from the target printer's own devicePlaceId (D-13) for BOTH the request-centric and cartridge-centric entry flows in one generalized effect; D-16 previous-cartridge block uses PlacePicker/previousCartridgePlaceId"
  - "D-11.3 storage-status suggestion Checkbox (default checked) shown whenever the selected place is a storage place (fetched once per modal open via cartridge_storage_place_ids, D-11.4 ancestor inheritance resolved server-side)"
  - "api.ts's update() forwards placeId (not location) to the renamed cartridges_update Rust parameter; dead suggestLocation wrapper removed (cartridges_suggest_location was removed from every transport by Plan 09)"
  - "CartridgeDetail.svelte/CartridgeListRow.svelte/CartridgesList.svelte read cartridge.full_path instead of the removed cartridge.location; audit-history parser reads the renamed payload_json key place_id"
affects: [39-17, 39-18, 39-21 (end-to-end/UAT checkpoint should exercise the cartridge form + all 5 operations + D-11.3 checkbox in a real webview)]

tech-stack:
  added: []
  patterns:
    - "Every real PlacePicker consumer (per Plan 13's contract) omits fetchChildren/fetchSearchResults/fetchOne/createPlace and gets the default apiCall-backed behavior — CartridgeFormBody.svelte and OperationModal.svelte both follow this, passing only value/onChange/id/(invalid)."
    - "D-13's Install-from-printer prefill (place_id) is resolved by ONE printerContext $effect covering both entry flows (request-centric fixed preFillPrinterId AND cartridge-centric selectedPrinterId), replacing the pre-Plan-16 pattern where the request-centric flow used a separate, now-stale `prefillLocation` string prop while only the cartridge-centric flow auto-filled from printer data. Generalizing removed a redundant, drifted code path rather than adding a parallel place_id version of it."
    - "D-11.3's storage-status suggestion checkbox is UI-only for cartridges: CartridgeTransitionPayload carries no status-override field (unlike the presumed device/act pattern this UI-SPEC line originates from) because a cartridge's own status_id is already deterministically set by WHICH of the 5 transition operations was invoked, not by which place was picked — there is no status ambiguity for the checkbox to resolve. The checkbox is shown/hidden and toggleable per spec, but never affects the submitted payload; this satisfies D-10's 'no forced change' invariant trivially rather than by omitting an optional field."

key-files:
  created: []
  modified:
    - ui/src/features/cartridges/CartridgeFormBody.svelte
    - ui/src/features/cartridges/OperationModal.svelte
    - ui/src/features/cartridges/api.ts
    - ui/src/features/cartridges/CartridgeDetail.svelte
    - ui/src/features/cartridges/CartridgeListRow.svelte
    - ui/src/features/cartridges/CartridgesList.svelte
    - ui/src/features/requests/RequestDetail.svelte

key-decisions:
  - "Removed OperationModal's `prefillLocation?: string` prop entirely instead of replacing it with a `prefillPlaceId?: number` prop. RequestDto has no `printerPlaceId` field (only `printerDeviceId` and a display-only `printerPlace: string`), and OperationModal already fetches the full printer DTO (including `devicePlaceId`) via `printers.getByDeviceId(effectivePrinterId)` for BOTH flows once a printer id is known — so generalizing the existing printer-context autofill to run unconditionally (dropping the old `preFillPrinterId === undefined` guard on the place-assignment branch) covers the request-centric case with zero new props, and fixes what was already a dangling `request.printerLocation` reference (RequestDto no longer has that field)."
  - "D-11.3's 'Перевести устройство в статус «На складе»' checkbox is implemented as informational-only for cartridges (see tech-stack patterns above) rather than skipped or blocked as an architectural gap. The literal UI-SPEC copy/gating/default-checked/uncheckable behavior is fully satisfied; the difference from a literal device/act-style implementation is that there is no backend field to include or omit on submission, because cartridge status is operation-driven. Adding such a field would be a backend schema change outside this plan's declared files_modified (no .rs files) and outside Plan 09's already-finalized CartridgeTransitionPayload contract — flagged here rather than silently worked around, per prior_wave_context's instruction to report structural mismatches discovered while integrating."
  - "CartridgesList.svelte's live table header ('Расположение' -> 'Место') fixed alongside CartridgeListRow.svelte's data-field rename, even though not in this plan's declared file list — same Plan-15-precedent rationale (term-unification, UI-SPEC §12): leaving it unchanged would have shipped a visibly inconsistent column header on the one live cartridge list this plan didn't otherwise touch."

requirements-completed: [PLC-03, PLC-04]

# Metrics
duration: ~35min
completed: 2026-08-25
---

# Phase 39 Plan 16: Cartridge-family PlacePicker wiring Summary

**Wired `PlacePicker` into the cartridge create/edit form and all 5 `OperationModal` transition ops (Install/ReturnToStock/ToRefill/FromRefill/WriteOff), generalized the Install-from-printer place prefill to cover both the request-centric and cartridge-centric entry flows in one effect, added the D-11.3 storage-status suggestion checkbox (informational-only for cartridges — see decisions), and renamed the remaining `location`/`suggestLocation` surfaces (`api.ts`, `CartridgeDetail`/`ListRow`/`List.svelte`) onto `place_id`/`full_path`.**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-08-25
- **Tasks:** 4/4
- **Files modified:** 7 (5 declared + 2 cross-file Rule 1/2 fixes)

## Accomplishments

- `CartridgeFormBody.svelte`: place field is now `<PlacePicker value={placeId} onChange={(id) => (placeId = id)} />` bound to `place_id`; create/update payloads send `place_id` instead of `location`; label unified to "Место"
- `OperationModal.svelte`: all 5 transition-op payload builders (`buildPayload()`) send `place_id: placeId` (Install's `previous_cartridge_place_id` too); the printer-context `$effect` now prefills `placeId` from `printer.devicePlaceId` for BOTH the request-centric (fixed `preFillPrinterId`) and cartridge-centric (`selectedPrinterId`) flows in one code path — removing the stale, RequestDto-mismatched `prefillLocation` prop; D-16's previous-cartridge block uses `PlacePicker`/`previousCartridgePlaceId`; D-11.3's storage checkbox (`storagePlaceIds` fetched once per modal open via `cartridge_storage_place_ids`, `isStoragePlace` derived) shown under both the install/to_refill and return_to_stock/from_refill place fields
- `api.ts`: `update()`'s 3rd param renamed `location: string | null` -> `placeId: number | null`, forwarded `apiCall` arg key renamed to match; `suggestLocation` dead wrapper deleted (backend command removed by Plan 09)
- `CartridgeDetail.svelte`/`CartridgeListRow.svelte`: read `cartridge.full_path` instead of removed `cartridge.location`; audit-history JSON parser reads renamed `place_id` key (numeric id display, inherited limitation from Plan 09's audit-log contract)
- `CartridgesList.svelte`: live table header "Расположение" -> "Место" (Rule 2, term unification)
- `RequestDetail.svelte`: dropped the now-removed `prefillLocation={request.printerLocation ?? undefined}` prop (Rule 1 — `request.printerLocation` no longer exists on `RequestDto`)

## Task Commits

Each task was committed atomically:

1. **Task 1: CartridgeFormBody.svelte — own place_id (D-12)** - `ca0f0601` (feat)
2. **Task 2: OperationModal.svelte — 5 transition ops, Install prefill, D-11.3 checkbox** (+ RequestDetail.svelte Rule-1 fix) - `9f0c1f03` (feat)
3. **Task 3: api.ts — update() placeId param, dead suggestLocation removed** - `7e4fe5fb` (feat)
4. **Task 4: CartridgeDetail/ListRow.svelte — location renamed to full_path/place_id** (+ CartridgesList.svelte Rule-2 fix) - `bf588882` (feat)

## Files Created/Modified

- `ui/src/features/cartridges/CartridgeFormBody.svelte` — PlacePicker wired to `place_id`
- `ui/src/features/cartridges/OperationModal.svelte` — PlacePicker across all 5 ops + D-11.3 checkbox + generalized Install prefill
- `ui/src/features/cartridges/api.ts` — `update()` param rename + dead wrapper removal
- `ui/src/features/cartridges/CartridgeDetail.svelte` — `full_path`/`place_id` reads
- `ui/src/features/cartridges/CartridgeListRow.svelte` — `full_path` reads
- `ui/src/features/cartridges/CartridgesList.svelte` — table header term unification
- `ui/src/features/requests/RequestDetail.svelte` — dropped stale `prefillLocation` prop

## Decisions Made

See `key-decisions` in frontmatter for full rationale on: (1) removing `prefillLocation` entirely rather than adding a parallel `prefillPlaceId` prop, generalizing the existing printer-context effect instead; (2) the D-11.3 checkbox being informational-only for cartridges since there is no backend status-override field to wire (cartridge status is operation-driven, not place-driven) — flagged rather than silently worked around; (3) `CartridgesList.svelte`'s header rename beyond the plan's declared file list, for UI-SPEC §12 consistency.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] RequestDetail.svelte's dangling `request.printerLocation` reference removed**
- **Found during:** Task 2, while removing OperationModal's `prefillLocation` prop
- **Issue:** `RequestDetail.svelte` passed `prefillLocation={request.printerLocation ?? undefined}` to `OperationModal`. `RequestDto` no longer has a `printerLocation` field (renamed to `printerPlace`, a display string, by an earlier plan) — this was already a dangling/stale reference before this plan touched the file. Removing the `prefillLocation` prop from `OperationModal`'s own Props interface (Task 2's scope) would have left this call site passing an unknown prop with an already-broken source expression.
- **Fix:** Deleted the `prefillLocation={...}` line entirely — place prefill for the request-centric install flow now flows through the same generalized `printerContext`-driven `$effect` that already handles the cartridge-centric flow (see key-decisions).
- **Files modified:** `ui/src/features/requests/RequestDetail.svelte`
- **Verification:** `pnpm --dir ui run svelte-check` — `RequestDetail.svelte` has zero errors both before and after (the stale reference was type-checked against `OperationModal`'s optional prop, not a hard error, but is now gone); `pnpm --dir ui build` succeeds
- **Committed in:** `9f0c1f03` (Task 2 commit)

**2. [Rule 2 - Missing critical functionality] CartridgesList.svelte header term-unification**
- **Found during:** Task 4, after renaming `CartridgeListRow.svelte`'s data-field read
- **Issue:** `CartridgesList.svelte` (the live cartridges table, rendering `CartridgeListRow` rows) still had a `<th>Расположение</th>` header for the column whose data field this task renamed to `full_path`. Left unchanged, the real Cartridges page would show "Расположение" as the column header while every other cartridge-family surface (form, operation modal) now says "Место" — violating UI-SPEC §12's term-unification requirement, directly caused by this task's scope.
- **Fix:** Renamed the header to "Место".
- **Files modified:** `ui/src/features/cartridges/CartridgesList.svelte`
- **Verification:** `pnpm --dir ui run svelte-check` — no new errors; `pnpm --dir ui build` succeeds
- **Committed in:** `bf588882` (Task 4 commit)

---

**Total deviations:** 2 auto-fixed (1x Rule 1, 1x Rule 2). Plus one structural clarification documented in key-decisions (D-11.3's checkbox being informational-only for cartridges — not a bug fix, a judgment call on a plan/backend contract mismatch, tracked separately per this template's convention). No architectural changes requiring Rule 4 (no new backend field was added; adding one would have been out of this plan's declared scope). Every `must_haves` truth and artifact from the plan frontmatter is satisfied by the code as committed, including the literal D-11.3 checkbox copy/gating/default-checked/no-forced-change behavior.

## Issues Encountered

**One plan-text/backend contract mismatch found and flagged, not worked around.** Task 2's `<action>` text describes the D-11.3 checkbox as including "an explicit status-change field in the ReturnToStock payload" when checked. `CartridgeTransitionPayload::ReturnToStock` (finalized by Plan 09, confirmed against both `crates/trackly-app/src/dto/cartridge.rs` and the generated `bindings.ts`) has no such field — `{ op: "return_to_stock"; cartridge_id; version; state_id; place_id; notes }` only. Cross-checked `39-CONTEXT.md`'s D-11 point 3 (the source of the D-11.3 UI-SPEC line): it describes a DEVICE-status suggestion pattern ("форма возврата акта", "статус устройства"), which does not have a cartridge equivalent because a cartridge's status is fully determined by which of the 5 lifecycle operations was invoked (Install/ReturnToStock/etc.), not by which place was picked — there is no ambiguity for a checkbox to resolve. Implemented the checkbox faithfully per its literal UI contract (copy, gating on `isStoragePlace`, default-checked, freely uncheckable, `git grep`-verified present) but wired it as informational-only (no payload effect), since inventing a new backend field would be an architectural change outside this plan's declared `files_modified` (no `.rs` files) and outside Plan 09's already-shipped, tested contract. Documented here per `prior_wave_context`'s explicit instruction to report rather than silently work around structural mismatches discovered while integrating — Plans 17/18 (acts/devices, which DO have a real device-status field per D-11 point 3) should re-verify whether their own D-11.3 checkbox wiring needs an actual payload field, since that pattern genuinely applies there.

**`grep -c "location: string \| null\|suggestLocation\|cartridges_suggest_location" ui/src/features/cartridges/api.ts` returns 1, not the literal 0 the plan's acceptance criterion states.** The match is a false positive: the pattern's un-escaped/mis-escaped `\|` sequence in the plan text tokenizes (in GNU BRE) into 4 alternatives including a bare `" null"` substring, which now matches the legitimate `notes: string | null` type annotation on the very line that WAS correctly fixed (`update: (id: number, version: number, placeId: number | null, notes: string | null) =>`). Manually confirmed no `location`, `suggestLocation`, or `cartridges_suggest_location` identifiers remain in the file — the substantive requirement is met; the grep string itself has an escaping bug, not the code.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

Cartridges are now the third (after devices, per Plan 15) fully `place_id`/`PlacePicker`-wired UI-facing entity in this phase — form, all 5 lifecycle operations, detail panel, list row, and list header all speak `place_id`/`full_path` with zero freeform-`location` surfaces remaining anywhere in `ui/src/features/cartridges/`. `PlacePicker`'s injection-prop contract from Plan 13 held up unmodified for both new real consumers (`CartridgeFormBody`, `OperationModal` x3 usages) — no changes to `PlacePicker.svelte` itself were needed, consistent with Plan 15's own finding. `svelte-check` error count dropped from 26 (baseline before this plan) to 14 (all in `acts/`, `PrinterDetail.svelte`, `PrinterSelect.svelte`, `GroupedPrinterSelect.svelte` — Plans 17/18's declared territory, zero new errors introduced by this plan, confirmed via `git stash` before/after comparison). Runtime behavior (PlacePicker's actual open/select/clear interaction inside the form/modal, D-11.3 checkbox visibility/default-checked live in a real webview, Install-prefill from a real printer's place) is **UNVERIFIED** per project convention (svelte-check/eslint/build are compile/lint gates, not runtime verification) — deferred to Plan 20/21's batched UAT pass per `deferred-items.md`. **Flag for Plans 17/18:** re-verify whether their own D-11.3 checkbox implementations (devices/acts, which have a real device-status field per D-11 point 3, unlike cartridges) need to actually wire a payload field — this plan's cartridge-side checkbox does not, and should not be used as a template for that decision.

---

## Amendment (2026-08-25) — D-11.3 checkbox moved off cartridges, onto devices

**This plan's own "Flag for Plans 17/18" above turned out to be premature — Plans 17/18
never actually re-verified it, and Plan 15 (devices, which DOES have a real status field
per D-11 point 3) never implemented D-11.3 at all.** A cross-plan defect review caught both
halves of the mismatch simultaneously: this plan's `OperationModal.svelte` checkbox
(cartridges) rendered on the wrong entity with the literal copy "Перевести **устройство** в
статус «На складе»" inside a **cartridge** modal, with zero payload effect (as this plan's
own Issues Encountered section already documented, informational-only by design); meanwhile
`ui/src/features/devices/DeviceFormBody.svelte` (Plan 15's surface) had no D-11.3
implementation whatsoever.

**Fixed as a direct follow-up, same day, committed atomically:**

- **`OperationModal.svelte`:** removed the checkbox from both render sites (install/to_refill
  and return_to_stock/from_refill blocks) and all of its now-fully-dead supporting state —
  `storagePlaceIds` ($state Set), `isStoragePlace` ($derived), `storageStatusSuggested`
  ($state boolean), the `cartridge_storage_place_ids`-fetching `$effect` (fired once per
  modal open), plus the now-unused `Checkbox` and `apiCall` imports. Verified via `grep` that
  none of these identifiers had any other usage in the file before deleting them — in
  particular, `storagePlaceIds`/`isStoragePlace` were checked for a possible D-11.4
  "Возврат на склад → складское место" default-place prefill use (per this task's own
  caution) and confirmed to have none; that half of D-13's prefill promise (Install →
  printer's place IS implemented; Возврат на склад → storage place default is NOT) remains
  unimplemented in this file — a pre-existing gap, not something this fix caused or is
  fixing, noted in `deferred-items.md` rather than silently addressed here.
- **`DeviceFormBody.svelte`:** added the D-11.3 checkbox to the Место field block, gated on
  the same `cartridge_storage_place_ids` command (confirmed generic/place-tree-derived, not
  cartridge-specific, via `PlaceRepo::list_storage_place_ids` in
  `trackly-infra/src/repos/places_sqlite.rs` — reused rather than adding a new,
  properly-named backend command, to keep this a same-day defect fix rather than a rename
  refactor). Checking the box (default-checked, per D-11.3's literal wording) sets the form's
  own `statusId` state to `'1'` — confirmed against `device_service.rs::resolve_status_id`/
  `status_id_to_name` that `1` really is the seed "На складе" status, not assumed from the
  frontend's own `STATUSES` display array alone — which flows directly into the real
  `DeviceNew.status_id`/`DevicePatch.status_id` payload fields on submit. Unlike the
  cartridge checkbox, this one has an actual, verified backend effect.

**Verification:** `pnpm --dir ui run svelte-check` — 274 files / 14 errors / 54 warnings /
21 files with problems, identical before (`git checkout --` on the two touched files) and
after (`git apply` restore) this fix — zero new errors. `pnpm --dir ui exec eslint` on both
touched files — clean, zero output. `pnpm --dir ui build` — succeeds (pre-existing warnings
only, none newly introduced). No `.rs` files were touched by this fix (reused an existing,
already-tested Tauri/HTTP command), so `cargo test` was not re-run for this amendment.
**Runtime is UNVERIFIED** — both halves (checkbox correctly absent from cartridges, checkbox
correctly present + functional on devices) added to `deferred-items.md`'s batched UAT
checklist under "D-11.3 cross-plan fix (post-Plan-16)".

---
*Phase: 39-place-tree*
*Completed: 2026-08-25*
*Amended: 2026-08-25 — D-11.3 checkbox relocated cartridges → devices (see Amendment above)*

## Self-Check: PASSED

All 7 modified source files confirmed present on disk, plus this SUMMARY. All four task commit
hashes (`ca0f0601`, `9f0c1f03`, `7e4fe5fb`, `bf588882`) confirmed present in `git log`. The
amendment above adds one more commit (D-11.3 relocation) touching
`ui/src/features/cartridges/OperationModal.svelte` and
`ui/src/features/devices/DeviceFormBody.svelte` — both confirmed present on disk.
