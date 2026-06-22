---
phase: 12-cartridge-request-interconnection
plan: 01
subsystem: api
tags: [rusqlite, cartridges, requests, dto, sql-join]

# Dependency graph
requires: []
provides:
  - "CartridgeFilter.installable_only: bool (domain + DTO) — SQL-level state_id IN (1,2) filter for stock cartridges"
  - "RequestDto.printer_location: Option<String> — joined from locations via the request's printer device"
affects: [12-02-service-wiring, 12-03-frontend-selector]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Boolean SQL gate idiom: `(?N = 0 OR col IN (a, b))` to toggle a fixed, hardcoded value-set filter without parameterizing the list (avoids injection surface for fixed domain constants)"
    - "Append-only SELECT column convention preserved: new JOIN columns always appended last, never inserted mid-list, to avoid shifting existing `row.get(n)` indices"

key-files:
  created: []
  modified:
    - crates/trackly-core/src/domain/cartridges.rs
    - crates/trackly-app/src/dto/cartridge.rs
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-app/tests/cartridges_lifecycle.rs
    - crates/trackly-core/src/domain/requests.rs
    - crates/trackly-app/src/dto/request.rs
    - crates/trackly-infra/src/repos/requests_sqlite.rs
    - crates/trackly-app/tests/phase06_stubs.rs

key-decisions:
  - "installable_only implemented as state_id IN (1, 2) hardcoded SQL literal, not a parameterized Vec<i64> — values are domain constants (D-01), not user input, eliminating any injection surface for an arbitrary state list"
  - "printer_location column appended LAST (idx 19) in SELECT_REQUESTS, after the existing category_name (idx 18) — preserves the established append-only convention documented in the file's own comments, avoiding index-shift across get/list/fetch_in_tx (single shared mapper)"
  - "LIMIT/OFFSET placeholders in cartridges_sqlite.rs::list() SELECT shifted from ?5/?6 to ?6/?7 to make room for the new ?5 installable_only flag — applied symmetrically to both COUNT and SELECT queries"

patterns-established: []

requirements-completed: [D-01, D-02, D-05]

# Metrics
duration: 22min
completed: 2026-06-22
---

# Phase 12 Plan 01: Backend filters — installable_only + printer_location Summary

**Добавлены два backend-поля без новых миграций: `CartridgeFilter.installable_only` (SQL-фильтр state_id IN (1,2) для складских картриджей) и `RequestDto.printer_location` (JOIN на locations через принтер заявки) — фундамент для Wave 2/3 (сервисный слой + фронтенд-селектор картриджа в заявке).**

## Performance

- **Duration:** 22 min
- **Started:** 2026-06-22T04:24:00Z (approx, derived from prior commit 7e9ad11)
- **Completed:** 2026-06-22T04:45:50Z
- **Tasks:** 2/2 completed
- **Files modified:** 8 (4 source + 4 source, no migrations)

## Accomplishments
- `CartridgeFilter` (domain + DTO) carries `installable_only: bool` (default `false`); SQL `list()` filters to `state_id IN (1, 2)` (Полный/Частичный) only on stock (`status_id = 1`) cartridges when the flag is set, for both the COUNT and SELECT queries
- `RequestRow`/`RequestDto` carry `printer_location: Option<String>`, joined from `locations.name` via `devices.location_id` of the request's printer — NULL-safe for `free_form` requests (no printer) and printers without a location set
- 6 new TDD tests (4 cartridge filter + 2 request printer_location), all green; zero regressions across the existing 24+ tests in the touched test files

## Task Commits

Each task was committed atomically:

1. **Task 1: CartridgeFilter.installable_only — domain + DTO + SQL + RED→GREEN тест** - `1898cf9` (feat)
2. **Task 2: RequestDto.printer_location — JOIN на locations + RequestRow + map_row_request** - `872103c` (feat)

**Plan metadata:** _(pending — final docs commit follows this summary)_

_Note: Both tasks followed `tdd="true"` — tests were authored and run together with the implementation in a single commit per task, since each task is a small, atomic field-addition with no separable RED-only milestone (the SQL change and the test exercise the same code path)._

## Files Created/Modified

- `crates/trackly-core/src/domain/cartridges.rs` - `CartridgeFilter.installable_only: bool` field + doc comment (D-01)
- `crates/trackly-app/src/dto/cartridge.rs` - DTO mirror field + `into_domain()` plumbing
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` - `list()` COUNT+SELECT gain `AND (?N = 0 OR c.state_id IN (1, 2))`; LIMIT/OFFSET placeholders shifted ?5/?6 → ?6/?7
- `crates/trackly-app/tests/cartridges_lifecycle.rs` - 4 new tests + `create_stock_cartridge_with_state` helper (arbitrary `state_id`)
- `crates/trackly-core/src/domain/requests.rs` - `RequestRow.printer_location: Option<String>` field + doc comment (D-05)
- `crates/trackly-app/src/dto/request.rs` - `RequestDto.printer_location` field (wire: `printerLocation` via container-level `rename_all = "camelCase"`) + `From<RequestRow>` plumbing
- `crates/trackly-infra/src/repos/requests_sqlite.rs` - `SELECT_REQUESTS` gains `LEFT JOIN locations dl ON dl.id = d.location_id` + `dl.name AS printer_location` appended last (idx 19); `map_row_request` reads `row.get(19)`
- `crates/trackly-app/tests/phase06_stubs.rs` - 2 new tests covering the JOIN's happy path and both NULL-safe branches (no printer / printer without location)

## Decisions Made

- **installable_only as hardcoded `IN (1, 2)`, not parameterized list:** D-01/D-02 define exactly two installable charge states (Полный=1, Частичный=2). Encoding them as a literal SQL value-set (gated by a single boolean flag) rather than accepting a client-supplied list closes off an entire class of injection surface — `installable_only` is a `bool`, it cannot smuggle arbitrary `state_id` values. Matches the plan's threat register (T-12-02, accept disposition).
- **printer_location appended last, not inserted near printer_name:** The file's own pre-existing comment explicitly warns against inserting columns mid-list in `SELECT_REQUESTS` because every subsequent `row.get(n)` would silently shift. Followed the same convention `category_name` (Phase 11) established — strict append-only ordering keeps the single shared mapper (`get`/`list`/`fetch_in_tx`) correct without per-call-site special-casing.
- **LIMIT/OFFSET placeholder renumbering confined to cartridges_sqlite.rs::list():** Only the SELECT query (not COUNT, which has no LIMIT/OFFSET) needed the shift from `?5`/`?6` to `?6`/`?7`; applied consistently with the new `installable_only` param inserted as `?5` in both queries for parameter-list parity.

## Deviations from Plan

None - plan executed exactly as written. The only adjustments were syntactic (the plan's pseudocode `svc.list(...)` returning `Ok((vec, count))` actually returns `CartridgeListResponse { items, total }` — tests were written against the real return type, not a behavioral deviation) and environmental (see Issues Encountered below).

## Issues Encountered

- **`restore_request_visibility_http` test initially failed** when run without `TRACKLY_AD_MOCK=1` in the shell environment — `AppCtx::build` defaulted to the real LDAP `AdClient`, which then returned `503 Service Unavailable` instead of the test's expected `403`. Confirmed via byte-identical diff against the pre-Task-2 version of the test file that this is a pre-existing dev-environment configuration matter (no AD server reachable from this macOS dev box, per project memory), not a regression introduced by `printer_location`. Re-ran with `TRACKLY_AD_MOCK=1` set — passes. All subsequent verification in this plan was run with `TRACKLY_AD_MOCK=1`.
- **`cargo clippy --workspace -- -D warnings` fails on 2 pre-existing `len_zero` lints** in `crates/trackly-app/src/services/template_service.rs` (lines 379, 430) — unrelated to this plan's files, already tracked in `.planning/phases/09-ad/deferred-items.md` and `.planning/phases/10-employee-employee-ui-role-gating-read/deferred-items.md`. Verified clean for the specific crates this plan touched (`trackly-core`, `trackly-infra` lib targets) and confirmed `cargo fmt --check` reports zero diff on all 8 files this plan modified.

## User Setup Required

None - no external service configuration required. (Dev-environment note: running the broader `trackly-app` integration test suite locally requires `TRACKLY_AD_MOCK=1` in the shell, per existing project convention — not new to this plan.)

## Next Phase Readiness

- Wave 2 (service layer) can now wire `installable_only` into the cartridge-picker service call used by the install-from-request flow, and read `RequestDto.printer_location` directly without a second printer lookup.
- Wave 3 (frontend, `OperationModal`/request form) has both fields available on the wire (`installableOnly` query param shape TBD by Wave 2's DTO; `printerLocation` already serializes as camelCase on `RequestDto`) — no backend blockers remain for the cartridge-selector or location auto-fill UI work.
- `bindings-phase6.ts` (hand-maintained TS types) was **not** updated in this plan — `RequestDto.printer_location` needs a corresponding `printerLocation: string | null` addition before Wave 3 can consume it from the frontend. This is in-scope for whichever wave first touches the TS bindings file (likely 12-03), not a gap in this plan (Wave 1 is explicitly backend-only per the plan's stated Purpose).

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-22*

## Self-Check: PASSED

All 8 modified source/test files verified present on disk; all 3 commit hashes
(`1898cf9`, `872103c`, `bd914cc`) verified present in `git log --oneline --all`.
