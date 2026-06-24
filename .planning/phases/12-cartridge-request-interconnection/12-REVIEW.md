---
phase: 12-cartridge-request-interconnection
reviewed: 2026-06-24T00:00:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - crates/trackly-infra/src/repos/cartridges_sqlite.rs
  - crates/trackly-app/tests/cartridges_lifecycle.rs
  - ui/src/features/cartridges/OperationModal.svelte
  - ui/src/features/printers/PrinterListRow.svelte
  - ui/src/lib/api/ws.ts
findings:
  critical: 0
  warning: 4
  info: 3
  total: 7
status: issues_found
---

# Phase 12: Code Review Report

**Reviewed:** 2026-06-24
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

Round 3 gap-closure (GAP-12-09..12, plans 12-16..12-19) reviewed against diff base
`f6b5dbc`. Scope was limited to the changes in the five listed files: the
single-UPDATE refactor + inverted-actor auto-return in the cartridge repo, the
previous-cartridge lookup widening in OperationModal, the PrinterListRow IP/location
column split, and the refcounted WebSocket singleton.

No BLOCKER-tier defects: all SQL is parameterised, the `state_id` FK prevents hard
crashes, and the auto-return runs inside the install transaction. However there are
four WARNING-tier correctness gaps — the most material being a drum-vs-cartridge
state mismatch in the auto-return defaults (backend) and its matching frontend
Select, plus two async-lifecycle bugs in the new WS singleton that can leak or tear
down connections out from under live consumers.

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: Auto-return hardcodes cartridge-only state 3 (Пустой) for the previous cartridge — invalid for photo-drums

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:474`
**Issue:** The auto-return resolves the previous cartridge's new state with
`let resolved_state_id = previous_cartridge_state_id.unwrap_or(3);`. State 3
(Пустой) is a *cartridge* charge state. Drums (kind_id=2) use states 4/5/6
(Новый/Изношенный/Отработанный) — a distinction enforced everywhere else in this
phase (see `installable_only` kind branching at lines 1094-1095 and test
`installable_only_includes_new_drum_excludes_spent_drum`). When the cartridge being
auto-returned is a drum and no override was supplied (the cartridge-centric path,
where the frontend cannot even supply a valid drum state — see WR-02), the drum is
silently written to a nonsensical cartridge state. The FK to `cartridge_states(id)`
is satisfied (state 3 exists), so there is no crash — the corruption is silent and
surfaces later as a drum displaying "Пустой". The auto-returned cartridge's kind is
available from the `prev_current` snapshot taken at line 491.
**Fix:**
```rust
// Reorder so the snapshot is taken before resolving the default:
let prev_current = self.fetch_in_tx(tx, prev_id)?;
let default_state = if prev_current.model_kind_id == Some(2) { 6 } else { 3 };
let resolved_state_id = previous_cartridge_state_id.unwrap_or(default_state);
```

### WR-02: Previous-cartridge state Select offers only cartridge states (1/2/3) even when the previous cartridge is a drum

**File:** `ui/src/features/cartridges/OperationModal.svelte:506-514` (and defaults at lines 100, 131)
**Issue:** The «Предыдущий картридж» block hardcodes three `<option>`s
(Полный/Частичный/Пустой = 1/2/3). When `previousCartridge.model_kind_id === 2`
(photo-drum), the operator can only pick a cartridge state, never the correct drum
states (4/5/6). The component already derives drum-aware options for the main field
(`DRUM_STATES` / `stateOptions`, lines 295-300) but the previous-cartridge block
ignores them. Combined with WR-01, a drum auto-returned over another drum is
guaranteed to get a wrong state regardless of operator action. The default
`previousCartridgeStateId = 3` (lines 100, 131) is likewise a cartridge state.
**Fix:** Derive the options from `previousCartridge.model_kind_id` and default the
state when a drum resolves:
```svelte
{#each (previousCartridge?.model_kind_id === 2 ? DRUM_STATES : CARTRIDGE_STATES) as opt (opt.value)}
  <option value={String(opt.value)}>{opt.label}</option>
{/each}
```

### WR-03: `connectWs()` async race can leak or orphan the underlying connection

**File:** `ui/src/lib/api/ws.ts:115-142`
**Issue:** `refCount` is incremented synchronously, but the real connection is
established asynchronously (`await import(...)`, then `await listen(...)` on the
Tauri path). If a consumer releases while establishment is still pending, the
`refCount === 0 && activeCleanup` teardown observes `activeCleanup === null` (it is
assigned only after the awaits), so the `unlisten`/cleanup that resolves afterward
is stored and never invoked — the listener leaks and keeps dispatching events,
defeating the GAP-12-10 goal. Sequence: A `connectWs()` (0→1, awaiting) → B
`connectWs()` (1→2) → B release (2→1) → A release (1→0, but `activeCleanup` still
null) → `listen` resolves and assigns an orphaned `activeCleanup`.
**Fix:** After establishment completes inside the `refCount === 1` block, re-check
and tear down if the count already fell to zero:
```ts
if (refCount === 1) {
  // ...establish, assign activeCleanup...
  if (refCount === 0 && activeCleanup) { activeCleanup(); activeCleanup = null; }
}
```
or have `release()` await the in-flight establishment promise before tearing down.

### WR-04: `disconnectWs()` leaves stale release closures that can decrement a freshly-established connection to zero

**File:** `ui/src/lib/api/ws.ts:144-165`
**Issue:** `disconnectWs()` force-sets `refCount = 0` and runs `activeCleanup()`, but
consumers that obtained a `release` from an earlier `connectWs()` still hold
`released === false`. After a `disconnectWs()`, if a component remounts and calls
`connectWs()` (refCount 0→1, new connection), an *old* consumer's later release runs
`refCount = Math.max(0, refCount - 1)` → 0 and tears down the live connection out
from under the new consumer. There is no generation/epoch to invalidate release
closures created before a `disconnectWs()`.
**Fix:** Capture an epoch per call and no-op the release if the epoch advanced:
```ts
const myEpoch = epoch;        // module-scope `let epoch = 0`
return () => {
  if (released || myEpoch !== epoch) return;
  released = true;
  refCount = Math.max(0, refCount - 1);
  if (refCount === 0 && activeCleanup) { activeCleanup(); activeCleanup = null; }
};
// disconnectWs(): epoch += 1; refCount = 0; ...
```

## Info

### IN-01: Inverted-actor payload duplicates `op_payload_json` instead of extending it

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:545-553`
**Issue:** The auto-return hand-builds a `return_to_stock` payload inline with
inverted actor keys, diverging from `op_payload_json` (lines 626-635) which builds
the same shape without actor fields. The inline JSON is correct today but the two
will drift if the payload schema changes.
**Fix:** Factor out a base-payload helper (or an optional-actor variant of
`op_payload_json`) so the canonical shape lives in one place.

### IN-02: `ipText` column can display the literal "USB" under an IP-implying heading

**File:** `ui/src/features/printers/PrinterListRow.svelte:43-45`
**Issue:** Splitting connectivity into the dedicated `.row-ip` column means the value
shown there is sometimes the string `USB` (or `—`), not an IP. Minor UX ambiguity
introduced by the column split; not a correctness bug.
**Fix:** Optional — render USB as a small badge, or keep as-is.

### IN-03: No test covers a drum being auto-returned (would have caught WR-01)

**File:** `crates/trackly-app/tests/cartridges_lifecycle.rs:558-610, 946-1078`
**Issue:** Auto-return tests cover the inverted-actor payload and the
override/default branches for cartridges, and there is a separate drum
`installable_only` test, but nothing installs a drum over another drum to exercise
the auto-return default-state path. The WR-01 defect slipped through precisely
because no test combines "auto-return" with "drum".
**Fix:** Add a test: install drum A into a printer, install drum B into the same
printer with `previous_cartridge_state_id: None`, then assert A's resolved state is
a valid drum state (6=Отработанный after the WR-01 fix).

---

_Reviewed: 2026-06-24_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
