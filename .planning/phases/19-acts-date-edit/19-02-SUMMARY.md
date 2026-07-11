---
phase: 19-acts-date-edit
plan: 02
subsystem: acts
tags: [rust, rusqlite, specta, contracts]

# Dependency graph
requires: [19-01]
provides:
  - ActPatch (domain) extended with handover_date_utc, number, expected_version
  - ActUpdateDto / ActUpdateItemDto wire contracts (trackly-app::dto::act)
  - update_act_header_in_tx (SqliteActRepository) — CAS header UPDATE
  - select_latest_device_mutation (SqliteAuditLogRepository) — most-recent
    single-device audit snapshot lookup
affects: [19-03]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Interface-first contracts wave: domain patch + wire DTOs + repo
      helpers built and compiling, with zero callers — ActService::update
      (Plan 19-03) is the sole future consumer of all four artifacts."
    - "CAS UPDATE via single-statement WHERE version=? guard (no separate
      read-then-write) — mirrors soft_delete_in_tx's TOCTOU-safe pattern."
    - "Most-recent-snapshot audit lookup flips ORDER BY to DESC + LIMIT 1
      (vs. the existing ASC full-list bulk-undo query) — Pitfall 2 safe."

key-files:
  created: []
  modified:
    - crates/trackly-core/src/domain/acts.rs
    - crates/trackly-app/src/dto/act.rs
    - crates/trackly-infra/src/repos/acts_sqlite.rs
    - crates/trackly-infra/src/repos/audit_log_sqlite.rs

key-decisions:
  - "update_act_header_in_tx's SET clause is unconditional for the 5
    original header fields (giver_name/receiver_name/location_id/notes/
    deadline_utc) and COALESCE-based only for handover_date_utc/number —
    matches the plan's literal SQL. This means the future caller
    (ActService::update, Plan 19-03) MUST always supply resolved (Some)
    values for the 5 unconditional fields — partial/no-op semantics for
    those fields is the service's responsibility, not the repo's. This
    plan does not enforce that contract (no caller exists yet); documented
    inline in the function doc comment for Plan 19-03 to honor."
  - "complectation_at_time semantics documented inline on
    ActUpdateItemDto (retained device -> overwrite-or-leave-unchanged;
    newly-added device -> None falls back to source device's live kit
    value) per RESEARCH.md Open Question 1 / Pitfall 4."
  - "specs (тех. характеристики) intentionally has NO field on
    ActUpdateItemDto — documented as a live device attribute (devices.notes)
    read at render time, not an act-owned snapshot; editing it via act-edit
    would be a distinct, unreviewed security surface not covered by
    D-05/D-06."

requirements-completed: []

# Metrics
duration: 11min
completed: 2026-07-11
---

# Phase 19 Plan 02: Act Update Contracts (ACT-02, contracts wave) Summary

**Built the compiling, uncalled data-layer contracts (`ActPatch` extension, `ActUpdateDto`/`ActUpdateItemDto` wire DTOs, `update_act_header_in_tx` CAS repo helper, `select_latest_device_mutation` audit lookup) that Plan 19-03's `ActService::update` will consume — an interface-first groundwork wave with zero observable behavior change.**

## Performance

- **Duration:** ~11 min
- **Completed:** 2026-07-11
- **Tasks:** 3/3 completed
- **Files modified:** 4 (no new files)

## Accomplishments

- `ActPatch` (domain, `trackly-core`) extended with `handover_date_utc: Option<i64>` (D-01/D-04 date edit), `number: Option<i64>` (D-04 № override), and `expected_version: i64` (CAS token, always required) — alongside its 5 original fields. Stays serde-free per the module's domain/DTO separation rule.
- `ActUpdateDto` / `ActUpdateItemDto` added to `crates/trackly-app/src/dto/act.rs` (sibling of `ActCreateDto`/`ActItemNewDto`), with a snake_case JSON invariant test proving `expected_version`, `number_override`, `handover_date_utc`, and `complectation_at_time` all serialize snake_case.
- `update_act_header_in_tx` added to `SqliteActRepository` — a single CAS `UPDATE` statement mirroring `soft_delete_in_tx`'s lock-check-folded-into-the-write pattern (structurally TOCTOU-safe), touching only the header's mutable fields and explicitly excluding `sub_number`/`parent_act_id`/`act_type`/`created_at_utc`.
- `select_latest_device_mutation` added to `SqliteAuditLogRepository` — a single-device "most recent prior state" lookup (`ORDER BY created_at_utc DESC, id DESC LIMIT 1`), distinct from the existing bulk `select_device_mutations_for_act` (ASC, full list, used for LIFO undo).
- No endpoint became reachable — neither repo helper nor the DTOs have a caller yet; `ActService::update` (Plan 19-03) is the documented first consumer of all four artifacts.

## Task Commits

Each task was committed atomically:

1. **Task 1: Extend domain ActPatch (interface-first)** - `4a57365` (feat)
2. **Task 2: ActUpdateDto / ActUpdateItemDto wire contracts + snake_case invariant test** - `a1207aa` (feat)
3. **Task 3: Repo-layer CAS header UPDATE + single-device most-recent audit lookup** - `c45aab5` (feat)

## Files Created/Modified

- `crates/trackly-core/src/domain/acts.rs` - `ActPatch` extended with `handover_date_utc`/`number`/`expected_version` (8 fields total); doc comment updated to name `ActService::update` (Plan 19-03) as first real consumer
- `crates/trackly-app/src/dto/act.rs` - `ActUpdateDto` (11 fields) + `ActUpdateItemDto` (2 fields) added; snake_case invariant test extended with `act_update_dto_snake_case_json_invariant`
- `crates/trackly-infra/src/repos/acts_sqlite.rs` - `update_act_header_in_tx` added to `SqliteActRepository`; `ActPatch` added to the domain-type import
- `crates/trackly-infra/src/repos/audit_log_sqlite.rs` - `select_latest_device_mutation` added to `SqliteAuditLogRepository`; `OptionalExtension` added to the `rusqlite` import

## Decisions Made

- `update_act_header_in_tx`'s SET clause is unconditional for `giver_name`/`receiver_name`/`location_id`/`notes`/`deadline_utc` (per the plan's literal SQL) and `COALESCE`-based only for `handover_date_utc`/`number`. This means the future caller must always supply fully-resolved values for the 5 unconditional fields — this plan documents that contract inline but does not (and cannot, with no caller yet) enforce or test it. Flagging explicitly for Plan 19-03's review.
- `complectation_at_time`'s retained-vs-newly-added semantics documented inline on `ActUpdateItemDto`, resolving RESEARCH.md Open Question 1 / Pitfall 4 as specified in the plan.
- `specs` (тех. характеристики) has no corresponding update field by design — stays a read-only, render-time live device attribute, explicitly out of scope for this phase.

## Deviations from Plan

None — all three tasks were executed exactly as specified in the plan's `<action>` blocks; no Rule 1-4 auto-fixes were required.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All four artifacts (`ActPatch` extension, `ActUpdateDto`/`ActUpdateItemDto`, `update_act_header_in_tx`, `select_latest_device_mutation`) exist, compile, and match the shapes documented in the plan's `<interfaces>` section.
- No behavior change is observable yet — this was purely additive, compiling groundwork.
- Plan 19-03 (`ActService::update`) is unblocked to build on top of these contracts: it must (a) resolve `ActPatch`'s 5 unconditional header fields to concrete values before calling `update_act_header_in_tx`, (b) enforce D-07 (only handover acts editable) and D-08 (return-bound devices non-removable) at the service layer — both deferred per this plan's threat model, and (c) use `select_latest_device_mutation`'s `DESC LIMIT 1` result when restoring a device snapshot on item removal.

---
*Phase: 19-acts-date-edit*
*Completed: 2026-07-11*

## Self-Check: PASSED

All 4 claimed modified files found on disk; SUMMARY.md itself found on disk;
all 3 claimed commit hashes (`4a57365`, `a1207aa`, `c45aab5`) found in git log.
