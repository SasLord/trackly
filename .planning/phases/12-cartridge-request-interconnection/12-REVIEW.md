---
phase: 12-cartridge-request-interconnection
reviewed: 2026-06-25T00:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - ui/src/lib/components/PrinterSelect.svelte
  - ui/src/features/cartridges/OperationModal.svelte
findings:
  critical: 0
  warning: 3
  info: 4
  total: 7
status: issues_found
---

# Phase 12: Code Review Report (Round 4 gap-closure, plan 12-20)

**Reviewed:** 2026-06-25
**Depth:** standard
**Files Reviewed:** 2
**Status:** issues_found

## Summary

Scope was strictly the diff since `a6227c3` in two files: the brand-new `PrinterSelect.svelte`
and the optional-printer-selection wiring added to `OperationModal.svelte`
(`selectedPrinterId` $state, `effectivePrinterId = preFillPrinterId ?? selectedPrinterId`,
the new printer-list/compat `$effect`, and the `printer_device_id` payload generalization).

Overall the reactive wiring is sound: the `effectivePrinterId` precedence (request-centric
`preFillPrinterId` wins over the new local `selectedPrinterId`) is correct, the optionality
contract holds (`undefined` → `printer_device_id: null`, no regression to the legacy
printer-less path), and the gating predicates on the three relevant `$effect`s correctly
isolate the new cartridge-centric branch from the request-centric and pre-filled flows.
`svelte-check` reports 0 errors and 0 warnings for both files; the `PrinterListItemDto`
type-alias deviation compiles cleanly.

No blockers. The most material finding is a silent server-side pagination cap (WR-01) that
makes the new selector incomplete on fleets larger than 200 printers — including, potentially,
the compatible target printer the operator is looking for. Two smaller correctness/robustness
warnings and four quality items follow.

Note on the `printers.get(deviceId)` / `printer_device_id` id convention: the new code feeds
`effectivePrinterId` (a *device* id, since the option `value` is `p.deviceId`) into
`printers.get()`. This mirrors the **pre-existing** request-centric path
(`printers.get(preFillPrinterId)` where `preFillPrinterId = request.printerDeviceId`, also a
device id) verbatim — confirmed against `a6227c3`. `printers_get` resolves `WHERE p.id = ?1`
(printer record id, not device id). If that mismatch is a real bug it is **pre-existing and
out of scope** for this diff; the new code introduces no *new* inconsistency because it follows
the exact same convention as the verified request-centric flow. Flagged here for traceability
only, not as a finding against this change.

## Warnings

### WR-01: New printer selector silently truncated to 200 entries — compatible printer may be unreachable

**File:** `ui/src/features/cartridges/OperationModal.svelte:277`
**Issue:** The new `$effect` loads the full printer list with
`printers.list({ status: null, search: null }, { offset: 0, limit: 500 })`, on the assumption
that 500 covers the whole fleet ("Full printer list" per the comment at line 165). But the
backend hard-clamps the page size: `let limit = page.limit.min(200)` in
`crates/trackly-infra/src/repos/printers_sqlite.rs:314`. So at most **200** printers are
returned, there is no second-page fetch, and `printerOptions` is silently incomplete on any
deployment with >200 printers. Because the truncation happens *before* grouping (server-side
`ORDER BY p.id DESC LIMIT 200`), a compatible printer that falls outside the first 200 will be
absent from both the «Совместимые принтеры» and «Остальные принтеры» groups — the operator
simply cannot select it, with no error or "showing 200 of N" indication.
**Fix:** Either page through until exhausted, or (simpler, since this selector wants the whole
fleet) add a dedicated unpaginated/compat-filtered list command. Minimal interim mitigation —
fetch in a loop until exhausted:
```ts
const all: PrinterListItemDto[] = [];
let offset = 0;
const limit = 200; // match the server cap; don't request 500 and assume it's honored
// eslint-disable-next-line no-constant-condition
while (true) {
  const res = await printers.list({ status: null, search: null }, { offset, limit });
  all.push(...res.items);
  if (res.items.length < limit || all.length >= res.total) break;
  offset += limit;
}
printerOptions = all;
```
At minimum, change `limit: 500` → `limit: 200` so the request reflects what the server actually
honors, and add a TODO documenting the >200 gap.

### WR-02: Option value/label use `deviceId`, the device-vs-record id convention is now user-visible

**File:** `ui/src/lib/components/PrinterSelect.svelte:38-41`, `85`/`92`
**Issue:** `printerLabel` renders `Принтер #${p.deviceId}` when `deviceName` is null, and the
option `value` is also `String(p.deviceId)`. That is internally consistent for this component,
but `deviceId` here is the device foreign key, while `printers.get(id)` / `printers_get` key on
the printer *record* id (`p.id`, `WHERE p.id = ?1`). The value emitted (`deviceId`) becomes
`effectivePrinterId` and is passed to the pre-existing `printers.get(effectivePrinterId)`
lookup. This is the surface where the device-id-vs-record-id convention (see Summary) becomes
user-visible and, if the underlying lookup convention is wrong, will manifest as "I picked
printer #7 but it loaded the wrong device / nothing." Confirm the round-trip on real data before
shipping.
**Fix:** Verify on a dataset where `device_id != printers.id` (not the trivial seed where they
coincide). If `printers_get` truly needs `p.id`, the option value must be `p.id` while a separate
`deviceId` is carried for the `printer_device_id` payload — they are not interchangeable. If the
device-id convention is in fact correct app-wide, add a comment in `PrinterSelect` documenting
that the emitted value is intentionally `deviceId` (matching `printer_device_id` /
`preFillPrinterId`) to prevent a future "fix" to `p.id`.

### WR-03: Previous-cartridge location has no validation in the newly-reachable cartridge-centric printer path

**File:** `ui/src/features/cartridges/OperationModal.svelte:204-230`, `383-385`, `427-455`
**Issue:** The new selector means the «Предыдущий картридж» block (state/location editors) can
now appear in the *cartridge-centric* install flow too — previously it only surfaced via the
request-centric `preFillPrinterId`. When `previousCartridge !== null`, `buildPayload()` sends
`previous_cartridge_location: previousCartridgeLocation` (default `''`). `validate()` checks the
new cartridge's `location`/`givenBy`/`givenTo` but never the previous cartridge's location, so
the displaced cartridge can be returned to an empty location string. The new selector widens the
set of paths that can hit this gap.
**Fix:** If the backend treats `previous_cartridge_location: ""` as "unknown/stock", confirm that
is acceptable; otherwise validate it when `previousCartridge !== null`:
```ts
if (op === 'install' && previousCartridge !== null && !previousCartridgeLocation.trim()) {
  valid = false; // surface a field error for op-prev-location
}
```

## Info

### IN-01: `$bindable('')` on `value` is dead — the prop is consumed one-way

**File:** `ui/src/lib/components/PrinterSelect.svelte:31`
**Issue:** `value = $bindable('')` declares `value` as bindable, but the only consumer
(`OperationModal.svelte:527`) passes it one-way (`value={...}` + `onchange`), never `bind:value`.
The `$bindable` machinery is unused and misleads about the contract (implies two-way binding no
caller uses).
**Fix:** Drop `$bindable` and declare a plain prop: `value = '',`. Keep `onchange` as the single
output channel, matching how `OperationModal` wires it.

### IN-02: Two `<option value="">` entries can coexist

**File:** `ui/src/lib/components/PrinterSelect.svelte:79-81`
**Issue:** The component always renders `<option value="">Без привязки к принтеру</option>`, and
when `options.length === 0` *also* renders `<option value="" disabled>Принтеры не найдены</option>`.
Two options share `value=""`; the duplicate-empty-value pairing is fragile, and the "не найдены"
row is reachable state in the failure path (the WR-01 fail-safe sets `printerOptions = []`).
**Fix:** Render the "не найдены" hint outside the `<select>` (as a `.field-hint`) when
`options.length === 0`, or give it a sentinel disabled value that is never `""`.

### IN-03: Flat-group `{#each}` keyed by array reference is brittle

**File:** `ui/src/lib/components/PrinterSelect.svelte:83`
**Issue:** `{#each groups as [, printers] (printers)}` keys the outer loop by the `printers`
array identity. In the flat branch `groups` is `[['', options]]`, so the key is the `options`
prop reference — fine today, but array-reference keys silently break reconciliation if the parent
passes a structurally-equal but new array, and the key is unused for rendering. The compat loop
already keys by `(label)`, the cleaner pattern.
**Fix:** Key by a stable string, e.g. `(label || 'all')` for both loops, or by index in the flat
case.

### IN-04: `printerContextHint` and the new selector both render after a pick — minor redundancy

**File:** `ui/src/features/cartridges/OperationModal.svelte:517-541`
**Issue:** In the cartridge-centric flow, once a printer is chosen the «Принтер (опционально)»
selector stays visible *and* `printerContextHint` («Устанавливается в принтер: …») appears below
it (because `effectivePrinterId` is now defined). Both describe the same target printer. Not a
bug — the hint adds the resolved name+IP the dropdown label may lack — but worth a deliberate UX
call.
**Fix:** Optional. If redundancy is undesired, suppress `printerContextHint` when the selector is
shown (`cartridge !== null && preFillPrinterId === undefined`), keeping the hint only for the
pre-filled request-centric flow where there is no selector.

---

_Reviewed: 2026-06-25_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
