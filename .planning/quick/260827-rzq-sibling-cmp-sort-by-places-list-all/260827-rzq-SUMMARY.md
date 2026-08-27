---
phase: 260827-rzq
plan: 01
subsystem: places
tags: [rust, svelte, sqlite, sort-order, total-order, dos-fix]

requires: []
provides:
  - "sibling_cmp — proven total order (reflexivity/antisymmetry/transitivity, exhaustive-tested)"
  - "PlaceService::list_all group-by-parent + sibling_cmp sort (no longer compares unrelated non-sibling nodes)"
  - "PlaceTree.svelte siblingCmp synchronized with the fixed Rust three-stage chain"
affects: [places, requests, devices, cartridges]

tech-stack:
  added: []
  patterns:
    - "Lexicographic comparator chain: every stage applies identically to every pair, explicit Some/None convention per stage (Some sorts before None), instead of skipping a stage when only one side has a value"

key-files:
  created: []
  modified:
    - crates/trackly-core/src/domain/places.rs
    - crates/trackly-app/src/services/place_service.rs
    - crates/trackly-app/tests/places_contents.rs
    - ui/src/features/places/PlaceTree.svelte

key-decisions:
  - "Some-before-None convention on every sibling_cmp stage (matches D-05: manual order wins if set, else automatic) — mixed-sibling ordering intentionally changes vs. the old non-deterministic behavior"
  - "list_all groups by parent_id then applies sibling_cmp within the group instead of a flat sort_by(sibling_cmp) over the whole tree, since no current consumer relied on the old flat order"

patterns-established:
  - "Total-order law tests (reflexivity/antisymmetry/transitivity) as an exhaustive cartesian-product test for any custom Ord-like comparator feeding slice::sort_by"

requirements-completed: [RZQ-01, RZQ-02, RZQ-03]

duration: ~90min
completed: 2026-08-27
---

# Quick 260827-rzq: sibling_cmp total-order fix Summary

**Replaced `sibling_cmp`'s selective-stage comparator (different rule per pair depending on which fields happened to be filled in) with a genuine three-stage total order, fixing the `places_list_all`/`places_list_children` panic (`ERR_EMPTY_RESPONSE`) on trees with partial manual `sort_order` from drag-and-drop reordering — plus the matching group-by-parent fix in `list_all` and a synchronized JS port in `PlaceTree.svelte`.**

## Performance

- **Duration:** ~90 min (majority spent on backend verification sweep, see Deviations)
- **Tasks:** 2 automated tasks committed; Task 3 (checkpoint:human-verify) prepared, not executed
- **Files modified:** 4

## Accomplishments

- `sibling_cmp` (crates/trackly-core/src/domain/places.rs) is now a proven total order: every pair goes through the same `sort_order → level → natural_name_cmp` chain, each stage explicitly deciding Some-vs-None (Some sorts before None) instead of silently skipping the stage when only one side has a value.
- Added an exhaustive total-order-law test over the `sort_order × level × name` cartesian product (36 rows) checking reflexivity, antisymmetry, and transitivity for every pair/triple.
- Added a 60-row regression test reproducing the exact production panic shape (partial `sort_order` contradicting name/level) — `sort_by(sibling_cmp)` now completes and yields a non-decreasing order instead of panicking with "user-provided comparison function does not correctly implement a total order".
- Added a total-order-law test for `natural_name_cmp` (unchanged implementation, now proven correct rather than assumed).
- `PlaceService::list_all` now sorts by `(parent_id, sibling_cmp)` instead of a flat `sibling_cmp` over the whole result set, so it no longer compares unrelated non-sibling nodes (e.g. a building against an unrelated room).
- Added an integration regression test in `places_contents.rs` exercising `list_children` and `list_all` through the real `PlaceService` with ~15 rooms carrying partial `sort_order` — the exact production-crashing shape — proving neither call panics.
- Ported the same fix to `PlaceTree.svelte`'s JS `siblingCmp`: the JS port shared the identical bug (only compares when both sides are non-null). `Array.prototype.sort` doesn't throw on an inconsistent comparator, so the browser tree could have silently rendered in the wrong order with no console error — now mirrors the fixed three-stage Rust chain exactly.
- Existing `sibling_cmp_orders_negative_zero_positive_levels` test (PLC-02, negative/zero floor levels) verified still green, unchanged.

## Task Commits

1. **Task 1: sibling_cmp — genuine total order + proving tests** - `996ee553` (fix)
2. **Task 2: list_all group-by-parent sort + service-level regression + JS parity** - `3021b776` (fix)

Task 3 (checkpoint:human-verify) — prepared below, not executed by the executor per plan constraints.

## Files Created/Modified

- `crates/trackly-core/src/domain/places.rs` — `sibling_cmp` rewritten as an explicit Some/None three-stage chain; 3 new tests (exhaustive total-order laws, 60-row case-C regression, `natural_name_cmp` total-order laws) added to `#[cfg(test)] mod tests`.
- `crates/trackly-app/src/services/place_service.rs` — `list_all` sorts `rows.sort_by(|a, b| a.parent_id.cmp(&b.parent_id).then_with(|| sibling_cmp(a, b)))`; doc comment updated to explain the group-by-parent rationale. `list_children` untouched (already true siblings).
- `crates/trackly-app/tests/places_contents.rs` — new test `list_children_and_list_all_survive_partial_sort_order_without_panicking`.
- `ui/src/features/places/PlaceTree.svelte` — `siblingCmp` rewritten to the same three-stage Some/None chain as Rust; `naturalNameCmp` untouched. `pnpm --dir ui build` re-run to refresh `ui/dist` for LAN/server-mode verification.

## Decisions Made

- **Some-before-None convention** on every `sibling_cmp` stage: a node with an explicit value at a given stage sorts before a node without one. This matches D-05 ("manual order wins if set, else automatic") and makes drag-and-drop-positioned nodes visibly take priority. Documented as an intentional behavior change for mixed sibling sets versus the old (already non-deterministic/panicking) behavior.
- **`list_all` groups by `parent_id` before sorting** rather than removing the sort entirely, since no current consumer (`PlaceTree.svelte` re-groups and re-sorts itself; `PlaceContents.svelte` is order-independent; `search()` bypasses the service) relies on the old flat order, but grouping keeps the array meaningful for any future direct consumer.

## Deviations from Plan

**None from the plan's task instructions** — Tasks 1 and 2 were implemented exactly as specified (three-stage Some/None chain, group-by-parent `list_all`, synchronized JS port).

### Verification sweep — partial completion, environment-caused

The plan's closing verification step ([Task 2] `<action>`) calls for one sequential `cargo test -p trackly-core` (full package) and one sequential `cargo test -p trackly-app` (full package, `--skip login_remember_persistent_cookie`), plus `-p trackly-infra`. What was actually completed, with real numbers:

- `cargo fmt --check` — **clean** (after `cargo fmt` was run once to fix two formatting diffs it introduced).
- `cargo test -p trackly-core --lib domain::places::` — **10/10 passed** (5 pre-existing + 3 new `sibling_cmp`/`natural_name_cmp` tests, +2 unrelated `PlaceKind` tests caught by the same module filter).
- `cargo test -p trackly-core` (full package) — **69/69 passed, 0 failed** (65 lib + 1 `no_io_deps` + 3 `secret_zeroize`).
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test places_contents -- --test-threads=1` — **4/4 passed** (the plan-mandated targeted regression check), run twice (once standalone, once via the full sweep) with identical results.
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app` lib unit tests (214 tests) — **214/214 passed, 0 failed**, including `services::report_service::tests::export_pdf_renders_org_header_name`, which failed once in an earlier attempt under full-suite thread contention but passed both standalone and in this clean run (pre-existing flake, unrelated to this task's files — logged, not fixed, per scope boundary).
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip login_remember_persistent_cookie` (full 96-binary integration sweep) — **NOT completed to 100%.** 38 of 96 integration test binaries ran to completion (484+ individual `... ok` assertions, **0 failures, 0 panics** across `acts_*`, `ad_admin_logins`, `ad_auth`, `ad_directory_sso`, `ad_register`, `auth_remember_cookie`, `auth_smoke`, `backup_service`, `cartridges_crud`, `cartridges_history`, `devices_autocomplete` through `devices_csv_session`) before the run was killed due to the root cause below. **Two prior attempts at this same full sweep also stalled** on different binaries (`ad_register`, then `auth_smoke`/`backup_service`/`cartridges_crud`) with 0% CPU for 40s–90s+ before spontaneously resuming — both stalled binaries were independently re-run in isolation and passed instantly (`ad_register`: 11/11 in 3.2s; `auth_remember_cookie`: 0 passed/1 filtered in <1s), proving the stalls were not code-related.
- `cargo test -p trackly-infra` — **NOT completed.** Two `rustc` compile processes stalled at fixed CPU time (3.85s / 4.57s) for 90+ seconds with zero progress; killed.

**Root cause identified:** `df -h .` showed the dev machine's disk at **98% full (865Gi used / 926Gi total, only 25Gi free)**, and `du -sh target` showed the `target/` build directory alone at **210GB**. This is a pre-existing, unrelated environment condition (not caused by this task — no new dependencies were added) that produces severe filesystem I/O degradation on a near-full APFS volume, manifesting as intermittent multi-second-to-minute process stalls across compilation and test execution alike (confirmed by even trivial commands like `df`/`du` stalling). This is flagged here for user awareness — **not auto-fixed**, since freeing 200GB+ of `target/` build artifacts is out of scope for this quick task and risks interrupting other in-progress work on the machine.

**Assessment:** the changes are well-covered by the tests that did complete — `trackly-core`'s full suite (which contains 100% of the new/changed logic), the specific `places_contents.rs` regression test for the production panic, and 38/96 `trackly-app` integration binaries with zero failures — combined with a clean compile of the full `trackly-app` package (which would fail to compile at all if `sibling_cmp`'s signature or any consumer were broken). No evidence of any regression was found in what did run. The unfinished portion of the sweep is honestly reported here rather than implied complete, per repo constraints.

### Frontend gates — all completed, all clean

- `pnpm --dir ui exec svelte-check` — **0 errors**, 57 pre-existing warnings across 17 files, none in `PlaceTree.svelte` or any file touched by this task.
- `pnpm --dir ui lint` (eslint + prettier + token/contrast/focus/CSP/print-isolation checks) — **all clean**.
- `pnpm --dir ui build` — **succeeded**, `ui/dist` rebuilt and up to date for the Task 3 live verification.

## Issues Encountered

See "Verification sweep" above — the disk-space/I/O degradation issue was discovered during closing verification and is documented, not fixed (out of scope). No issues encountered during implementation of Tasks 1/2 themselves.

## Task 3 — Checkpoint (prepared, not executed)

**Type:** human-verify (blocking)

**What was built:** Tasks 1/2 proved everything testable by automation: `sibling_cmp` is a genuine total order (exhaustive law tests + 60-row case-C regression reproducing the exact production panic), `list_all` no longer compares unrelated non-sibling nodes, and the JS port is synchronized with Rust. But (a) compile gates (svelte-check/eslint/build) don't prove Svelte 5 rune runtime behavior, and (b) there's no synthetic-DOM substitute for a real WKWebView/browser — this needs visual confirmation in the running app on a tree shaped like the one that crashed production.

**How to verify:**
1. Confirm `ui/dist` is fresh (`pnpm --dir ui build` was already run as part of this plan — see Files Modified), then run `cargo tauri dev` (desktop webview). AD/SNMP mocks are not required for this check — the defect is places-only.
2. In "Места", build a tree with ≥3 buildings/floors and ≥20 rooms under one building (mix names like "Кабинет 1"… "Кабинет 20" with "Кабинет 2", "Кабинет 10" to also exercise natural-sort).
3. If drag-and-drop reordering exists in the UI, drag 2–3 rooms to a new position (this sets `sort_order` on ONLY the dragged nodes — exactly the partial-set shape that crashed production). If drag-and-drop isn't available, set "Порядок" manually via the edit form for 2–3 rooms (`sort_order` field, per D-05/39-UI-SPEC.md).
4. Reload the "Места" page (or reopen the app) — the tree must load WITHOUT "Не удалось загрузить места…" and without console errors, regardless of tree size.
5. Confirm the dragged/manually-ordered nodes appear at the START of their sibling group (per the new Some-before-None convention), with the rest in natural name order, IDENTICALLY on every reload (determinism).
6. If LAN/server-mode browser access is available, repeat step 4 via `https://localhost:8443` (or configured port) — this is the transport where production actually failed with `net::ERR_EMPTY_RESPONSE`.
7. (Optional, not required for approval) Check `logs/` next to the executable for absence of new "user-provided comparison function does not correctly implement a total order" lines after the steps above.

**Resume-signal:** Type "approved" or describe what you saw that doesn't match.

## Next Phase Readiness

- Tasks 1 and 2 are committed and code-complete; Task 3 requires the user to run the app and confirm live behavior (not something the executor can fake).
- Separate from this task's scope: the dev machine's disk is at 98% capacity with a 210GB `target/` directory — worth a `cargo clean` or disk cleanup at the user's convenience; it is currently degrading local build/test performance (documented above), though it does not block correctness of this fix.

---
*Phase: 260827-rzq*
*Completed: 2026-08-27*

## Self-Check: PASSED

- FOUND: crates/trackly-core/src/domain/places.rs
- FOUND: crates/trackly-app/src/services/place_service.rs
- FOUND: crates/trackly-app/tests/places_contents.rs
- FOUND: ui/src/features/places/PlaceTree.svelte
- FOUND commit: 996ee553
- FOUND commit: 3021b776
