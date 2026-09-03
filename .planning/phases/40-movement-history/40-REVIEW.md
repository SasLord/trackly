---
phase: 40-movement-history
reviewed: 2026-09-03T06:46:01Z
depth: standard
scope: "gap-closure round 2 ONLY — commits b5e9b55f..c78d0377 (plans 40-28, 40-29). Plans 40-01..40-27 were reviewed in a prior round; that round's REVIEW.md is superseded by this file and is preserved in git history at the parent of commit 27bf19f0."
files_reviewed: 17
files_reviewed_list:
  - crates/trackly-app/src/services/device_service.rs
  - crates/trackly-app/src/services/act_number_display.rs
  - crates/trackly-app/src/services/place_movement_service.rs
  - crates/trackly-app/src/services/place_path_display.rs
  - crates/trackly-app/src/services/report_service.rs
  - crates/trackly-app/src/services/mod.rs
  - crates/trackly-app/src/dto/device.rs
  - crates/trackly-core/src/domain/devices.rs
  - crates/trackly-infra/src/db/pools.rs
  - crates/trackly-infra/src/repos/cartridges_sqlite.rs
  - crates/trackly-infra/src/repos/devices_sqlite.rs
  - crates/trackly-infra/src/test_support/mod.rs
  - crates/trackly-infra/src/test_support/test_app_ctx.rs
  - crates/trackly-app/tests/cartridges_lifecycle.rs
  - crates/trackly-app/tests/place_movements_timeline.rs
  - crates/trackly-app/tests/place_movements_write_sites_devices.rs
  - crates/trackly-app/tests/report_movements.rs
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
status: issues_found
---

# Phase 40: Code Review Report — Gap-Closure Round 2

**Scope:** commits `b5e9b55f..c78d0377` only (Plan 40-28: CR-03, CR-02; Plan 40-29: CR-01,
WR-10). This replaces the prior round's `40-REVIEW.md`; the prior review is preserved in git
history at the parent commit of `27bf19f0`. Plans 40-01..40-27 were NOT re-reviewed here.

**Reviewed:** 2026-09-03T06:46:01Z
**Depth:** standard
**Files Reviewed:** 17
**Status:** issues_found (0 Critical, 3 Warning, 2 Info)

## Summary

This round closes all 4 gaps left open by the prior verification pass (`40-VERIFICATION.md`):
CR-03 (unconditional printer-place-clear cascade wiping cartridge places), CR-02 (auto-return
fallback missing the `from_place_id` branch, the exact UAT-16 defect), CR-01 (nested
`ReaderPool::acquire()` deadlock risk in `get_timeline`/`query_movements_inner`), and WR-10
(report showing a bare act number instead of the canonical "20в" form).

I traced the full diff line-by-line, re-derived the SQL semantics for
`last_known_storage_place_in_tx`'s new fallback chain, and — going beyond static reading —
**empirically mutated the fixed code back to its pre-fix shape three times** (the CR-03 gate,
the CR-02 `from_place_id`/`p_from.is_storage` branch, confirmed via direct test runs) to verify
the new regression tests are genuinely red on unfixed code and green on the shipped fix. All
three mutations reproduced the exact panic messages documented in the plan's own
"Verification Evidence" section, then were reverted and the tree confirmed clean
(`git status --short crates/` empty, `cargo build`/`cargo clippy -D warnings` clean). Unlike the
round-1 defect this gap-closure round explicitly exists to fix, **none of the new tests in this
round are hand-seeded around the code path they claim to prove** — both `cartridges_lifecycle.rs`
and `place_movements_write_sites_devices.rs`'s new tests drive the real
`CartridgeService`/`DeviceService` flow end-to-end and read back real DB state.

The CR-01 nested-acquire fix is structurally sound (exactly one `readers.acquire()` remains in
`get_timeline`, and `query_movements_inner` no longer takes a `&ReaderPool` parameter at all —
confirmed by `grep`). `ReaderPool::acquire_timeout`'s `Condvar` loop correctly recomputes the
remaining wait duration against a fixed deadline on every iteration and re-checks the deadline
after a spurious/lost-race wakeup, so it is not vulnerable to the classic
recompute-from-full-timeout bug.

No Critical/Blocker findings. Three Warnings worth fixing before this is considered fully closed
— two are pre-existing gaps this round's own fixes made *more* likely to bite (a widened
attack surface for a documented but now more discoverable bug, and a reader-pool behavior
regression introduced by the very refactor meant to reduce reader-pool pressure), and one is an
unenforced precondition on a function whose rustdoc explicitly warns about the exact bug this
round just fixed. No real organization/personal data found in any new or modified file
(`node scripts/check-privacy.mjs` run against the working tree: 0 violations).

## Warnings

### WR-01: `cascade_place_for_printer_in_tx`'s `Some -> None` precondition is documentation-only, not enforced

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:996-1010` (rustdoc), function body
at `:1002-1050`

**Issue:** The CR-03 fix moved the guard (`after.place_id.is_some() && before_place_id !=
after.place_id`) to the ONLY call site, `device_service.rs:347`. The rustdoc added over
`cascade_place_for_printer_in_tx` explicitly says:

> **Caller MUST NOT call this with `new_place_id: None`** — ... this function does not
> re-check it.

That sentence is accurate today, but the function itself contains no runtime check of its own —
`new_place_id: Option<i64>` is accepted and, if `None`, the function will happily execute the
exact unconditional `UPDATE cartridges SET place_id=NULL ...` for every attached cartridge that
CR-03 was written to eliminate. The review brief for this round specifically asked whether the
precondition is enforceable if a second call site is ever added: it is not — a future
call site that skips the gate (or a refactor that inlines/duplicates the caller-side check
incorrectly) reintroduces CR-03 silently, with no compiler or test signal until someone notices
missing `place_movements` rows in production.

**Why it matters:** This is precisely the failure mode CR-03 itself was: an implicit,
undocumented-in-code invariant that only held because there happened to be exactly one caller.
Rustdoc is not load-bearing at runtime.

**Fix:** Add a cheap `debug_assert!` at the top of the function body, which costs nothing in
release builds but turns any future violation into an immediate, loud dev/test-time failure:

```rust
pub fn cascade_place_for_printer_in_tx(
    &self,
    tx: &Transaction<'_>,
    printer_device_id: i64,
    new_place_id: Option<i64>,
    source: MovementSource,
    note: &str,
    user_id: Option<i64>,
    now_utc: i64,
) -> Result<(), AppError> {
    debug_assert!(
        new_place_id.is_some(),
        "cascade_place_for_printer_in_tx must not be called with new_place_id: None \
         (CR-03) — caller must gate on after.place_id.is_some() first"
    );
    ...
```

---

### WR-02: `compute_place_path_short` now always acquires a reader-pool connection, even when it will return `None` — a regression introduced by the very refactor meant to reduce reader-pool pressure

**File:** `crates/trackly-app/src/services/place_path_display.rs:42-49`

**Issue:** Before this round:

```rust
pub fn compute_place_path_short(readers: &ReaderPool, place_id: Option<i64>, snapshot: Option<String>) -> Option<String> {
    let snapshot = snapshot?;          // early return BEFORE acquiring
    let conn = readers.acquire();
    ...
```

After this round:

```rust
pub fn compute_place_path_short(readers: &ReaderPool, place_id: Option<i64>, snapshot: Option<String>) -> Option<String> {
    let conn = readers.acquire();      // now unconditional
    compute_place_path_short_with_conn(&conn, place_id, snapshot)
}
pub fn compute_place_path_short_with_conn(conn: &Connection, place_id: Option<i64>, snapshot: Option<String>) -> Option<String> {
    let snapshot = snapshot?;          // early return moved INSIDE, after acquire already happened
    ...
```

The early-return-on-`None`-snapshot check moved from before the `acquire()` to after it, in the
wrapper's only remaining caller path. 40-29's own plan text asserts "Публичная сигнатура и
поведение `compute_place_path_short` не меняются" ("public signature and behavior unchanged") —
that claim is false for this specific case: every call with `snapshot: None` now needlessly
takes and holds a pool slot (how ever briefly) where it previously took none at all.

**Why it matters:** The sole remaining caller of the `&ReaderPool`-taking wrapper is
`act_service.rs:2867` (one call per printed act, not a per-row loop), so the practical blast
radius today is small. But this is the exact plan whose entire premise is "reduce needless
`ReaderPool` acquisitions to avoid a whole-app read deadlock under LAN concurrency" (CR-01) —
introducing a new, avoidable acquisition in the sibling wrapper it left behind is a step in the
wrong direction for the same risk class this round exists to close, and it silently breaks the
plan's own "no behavior change" acceptance criterion.

**Fix:** Keep the early return in the thin wrapper, before acquiring:

```rust
pub fn compute_place_path_short(
    readers: &ReaderPool,
    place_id: Option<i64>,
    snapshot: Option<String>,
) -> Option<String> {
    let snapshot = snapshot?;
    let conn = readers.acquire();
    compute_place_path_short_with_conn(&conn, place_id, Some(snapshot))
}
```

(and drop the redundant `snapshot?` from `compute_place_path_short_with_conn`, or leave it as a
defensive no-op for direct callers — either is fine as long as the wrapper does not pay for an
acquire it doesn't need).

---

### WR-03: `DevicePatch`'s tri-state (`Option<Option<T>>`) contract is now internally inconsistent — the DTO docstring claims uniform null-clearing semantics that only `place_id` actually honors

**Files:** `crates/trackly-app/src/dto/device.rs:133-135` (docstring, unchanged by this round),
`crates/trackly-app/src/dto/device.rs:176-183` (conversion, changed for `place_id` only, lines
165-172 for `inventory_no`/`serial_no`/`model`/`specs`/`kit`/`state` unchanged),
`crates/trackly-core/src/domain/devices.rs:32-48`

**Issue:** This round's own SUMMARY documents this as a deliberately narrow, "logged as a
deferred item" fix — that part is fine and transparent. What is not fine: the pre-existing
docstring above `DevicePatch` (untouched by this diff) reads:

> `Option<Option<T>>` — None означает «не менять», Some(None) — «установить NULL»,
> Some(Some(v)) — «установить v». Для обязательных полей: None = «не менять».

This describes the CORRECT tri-state contract, and it is written at the top of the struct as if
it applies uniformly to every field. After this round, exactly ONE field (`place_id`) actually
implements it end-to-end (DTO → domain → SQL `CASE WHEN`). The other six nullable fields
(`inventory_no`, `serial_no`, `model`, `specs`, `kit`, `state`) still flatten
`Some(inner) -> inner` in the `From<DevicePatch>` conversion
(`crates/trackly-app/src/dto/device.rs:165-172`) into a plain `Option<T>` on the domain struct,
and the domain-level SQL still uses `COALESCE(?, col)` for all of them
(`crates/trackly-infra/src/repos/devices_sqlite.rs:266,271-274` /
`:688,693-696`) — the exact bug class CR-03's investigation discovered for `place_id`.
Concretely: `DeviceService::update(..., DevicePatch { inventory_no: Some(None), ..Default::default() })`
looks, from the docstring and the type signature, like a legitimate "clear the inventory
number" call — it compiles, it returns `Ok`, and it silently does nothing.

**Why it matters:** This round's fix makes the inconsistency MORE dangerous, not less, because
now the codebase contains a proof-of-concept, working example of the tri-state pattern
(`place_id`) sitting right next to six fields that look identical at the type level but are
silently broken. A future caller (or a future executor extending this pattern to another entity)
has every reason to trust the docstring and no code-level signal that six of seven fields don't
honor it.

**Fix:** Either (a) narrow the docstring to explicitly call out that only `place_id` currently
implements the tri-state contract and the rest are `Some(inner) -> inner` best-effort (linking to
the deferred-item note), or — better, and consistent with the SUMMARY's own suggested follow-up
— apply the same `Option<Option<T>>` + `CASE WHEN <flag>` pattern to the remaining six fields in
a dedicated follow-up plan, since the SUMMARY already identifies the mechanical fix is "applicable
to all at once."

## Info

### IN-01: Deadlock regression test can misattribute an unrelated setup panic to "CR-01 regression"

**File:** `crates/trackly-app/tests/place_movements_timeline.rs:658-721`
(`get_timeline_does_not_deadlock_with_single_reader_slot`)

**Issue:** The test spawns a bare `std::thread::spawn(move || { ... })` whose `JoinHandle` is
discarded (never `.join()`'d), and forwards only the `get_timeline` outcome through an `mpsc`
channel. If any of the fixture helpers inside the worker thread panics for an unrelated reason
(e.g. `seed_manager_caller`/`seed_place`/`seed_device`/`seed_movement_row`'s `.expect(...)` calls,
all of which can panic on a genuine — if unlikely — DB error), the panic happens inside the
detached thread, is printed to stderr by the default panic hook, and the channel never receives a
message. The main test thread's `result_rx.recv_timeout(Duration::from_secs(5))` then times out
for a completely different reason and reports:

```
"get_timeline exceeded 5 s budget — nested reader-pool acquire regressed (CR-01)"
```

— a misleading diagnosis for whoever has to debug a future red run of this test, who would look
for a reader-pool regression instead of noticing the real panic buried in stderr output above the
test-framework summary.

**Fix:** Wrap the worker body in `std::panic::catch_unwind` and forward the panic payload through
the channel as a distinct `Err` variant (or at minimum, add a comment on the test documenting that
"exceeded 5s budget" can also mean "a fixture helper panicked — check stderr above this failure
for the real panic message" so a maintainer isn't misled the first time this fires for the wrong
reason).

### IN-02: New SQL string literals in `report_movements.rs` test helpers collapse onto single physical lines with irregular whitespace

**File:** `crates/trackly-app/tests/report_movements.rs:264 (seed_act), 281 (seed_return_act), 311
(seed_movement_with_act)`

**Issue:** The rest of the codebase's SQL string literals (e.g.
`crates/trackly-infra/src/repos/cartridges_sqlite.rs`, `crates/trackly-app/src/services/report_service.rs`)
consistently use `\`-continued multi-line strings, one clause per line, for readability. The three
new helpers added in this round instead have their `INSERT INTO ...` SQL as a single physical Rust
source line containing large runs of literal whitespace where line breaks would normally be
(e.g. `"INSERT INTO acts                  (number, act_type, ...) ...`). This is harmless — SQL
ignores the extra whitespace and the tests pass — but it is a readability regression versus the
established convention in this same codebase and makes the SQL harder to diff/review in the
future.

**Fix:** Reformat to the codebase's usual `\`-continued multi-line style (one clause/columns list
per line), matching the sibling helper `seed_movement` a few lines above in the same file.

---

_Reviewed: 2026-09-03T06:46:01Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
