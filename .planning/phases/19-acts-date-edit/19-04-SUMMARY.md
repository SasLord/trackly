---
phase: 19-acts-date-edit
plan: 04
subsystem: acts
tags: [rust, tauri, axum, specta, rbac, act-edit]

# Dependency graph
requires: [19-02, 19-03]
provides:
  - build_acts_update (shared helper, Action::MutateActs-gated)
  - acts_update Tauri command (thin wrapper, registered in specta_export.rs)
  - POST /api/v1/acts_update axum route (UpdatePayload + handler_update)
  - acts.update(payload) frontend API client method
  - Case 42 RBAC regression test (Employee -> acts_update -> 403)
affects: [19-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "S-1 build_* + thin-wrapper pattern applied unchanged to a single-DTO
      mutation (id/expected_version live inside ActUpdateDto itself, unlike
      build_acts_return's split act_id + payload args)."
    - "Dual-transport delegation: acts_update (Tauri) and handler_update
      (axum) both call the SAME build_acts_update — no duplicated
      authorize()/validation logic to drift out of sync."

key-files:
  created: []
  modified:
    - crates/trackly-app/src/tauri_cmds/acts.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/export_bindings.rs
    - crates/trackly-app/src/http/acts.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs
    - ui/src/lib/api/acts.ts

key-decisions:
  - "build_acts_update mirrors build_acts_create's single-DTO shape exactly
    (not build_acts_return's split act_id/payload args) — id and
    expected_version already live inside ActUpdateDto per Plan 19-02's
    contract."
  - "RBAC regression test landed as Case 42 (file's actual running max was
    41, not a stale plan-suggested number) — grepped the file first per the
    plan's explicit instruction."
  - "ui/src/bindings.ts stays gitignored/regenerated, not committed — same
    convention as every prior plan touching Tauri commands (19-01's
    documented precedent)."

requirements-completed: []

# Metrics
duration: 45min
completed: 2026-07-12
---

# Phase 19 Plan 04: Act Update Transports (ACT-02, transports wave) Summary

**`ActService::update` (built in Plan 19-03) is now reachable from both transports — `acts_update` over Tauri invoke and `POST /api/v1/acts_update` over axum, both delegating to one shared `build_acts_update` helper (S-1 pattern) — with a new RBAC regression test proving Employee role still gets 403, and a typed `acts.update(payload)` frontend client method wired against the regenerated `bindings.ts`.**

## Performance

- **Duration:** ~45 min (includes a ~53-min full `cargo test --workspace` background run as the final verification step)
- **Completed:** 2026-07-12
- **Tasks:** 3/3 completed
- **Files modified:** 6 (no new files)

## Accomplishments

- `build_acts_update` added to `crates/trackly-app/src/tauri_cmds/acts.rs` — single-DTO shape (`payload: ActUpdateDto`) mirroring `build_acts_create`, gated by `authorize(caller, &Action::MutateActs)`, delegating to `ctx.acts.update(payload)`.
- Thin `#[tauri::command] #[specta::specta] acts_update` wrapper added, resolving identity via `resolve_tauri_identity` then calling `build_acts_update` — identical shape to every other acts mutation command in the file.
- `acts_update` registered in `specta_export.rs`'s `collect_commands![...]` list; `cargo test -p trackly-app --test export_bindings` regenerated `ui/src/bindings.ts` with `ActUpdateDto`/`acts_update` present (confirmed via grep). Two new assertions added to `export_bindings.rs` for regression protection.
- `UpdatePayload` + `handler_update` added to `crates/trackly-app/src/http/acts.rs`, mirroring `CreatePayload`/`handler_create`'s exact shape — session-gated via `session_identity`, delegates to the SAME `build_acts_update` the Tauri command calls. `POST /api/v1/acts_update` route added to `router()` immediately after `acts_delete`.
- New RBAC regression case (Case 42, since the file's actual running max was 41 — grepped first per the plan's explicit warning) in `role_endpoint_matrix.rs`: Employee session -> `POST /api/v1/acts_update` -> expects `403 Forbidden`. Uses a synthetic `id`/`expected_version` since RBAC must reject before any act lookup.
- `acts.update(payload)` added to `ui/src/lib/api/acts.ts`, typed against the regenerated `ActUpdateDto`/`ActDto`, using the same `apiCall` wrapper as every other `acts.*` method — works identically over Tauri invoke and the LAN/browser HTTP transport with no changes needed to `client.ts`.
- Verified end-to-end: `cargo test -p trackly-app --test export_bindings` (green), `cargo test -p trackly-app --test role_endpoint_matrix` (green, includes Case 42), `cargo build -p trackly-app` (clean), `pnpm --dir ui exec svelte-check` (0 errors, 0 new warnings), and a full `cargo test --workspace` background run (exit code 0, no `FAILED`/`error[` anywhere in the run — confirms no regression across `trackly-core`/`trackly-infra`/`trackly-app`).

## Task Commits

Each task was committed atomically:

1. **Task 1: Tauri command + specta registration + bindings regen** - `d626468` (feat)
2. **Task 2: axum HTTP handler + router entry + RBAC regression test** - `e1253ad` (feat)
3. **Task 3: Frontend API client method** - `c47954a` (feat)

## Files Created/Modified

- `crates/trackly-app/src/tauri_cmds/acts.rs` - `ActUpdateDto` import added; `build_acts_update` helper (after `build_acts_delete`); `acts_update` thin `#[tauri::command]` wrapper (after `acts_delete`)
- `crates/trackly-app/src/specta_export.rs` - `crate::tauri_cmds::acts::acts_update` registered in `collect_commands![...]`, immediately after `acts_delete`
- `crates/trackly-app/tests/export_bindings.rs` - two new assertions (`ActUpdateDto` type, `acts_update` command present in `bindings.ts`)
- `crates/trackly-app/src/http/acts.rs` - `ActUpdateDto` import + `build_acts_update` import added; `UpdatePayload` struct (after `DeletePayload`); `handler_update` (after `handler_delete`); `/api/v1/acts_update` route (after `/api/v1/acts_delete`)
- `crates/trackly-app/tests/role_endpoint_matrix.rs` - `act_update_payload` fixture (after `act_payload`); Case 42 regression test (after Case 41), appended at the end of the test body
- `ui/src/lib/api/acts.ts` - `ActUpdateDto` import added; `acts.update(payload)` method added (after `create`)

## Decisions Made

- `build_acts_update` uses the single-DTO shape (`payload: ActUpdateDto`, matching `build_acts_create`) rather than `build_acts_return`'s split `act_id`/`payload` args — `id`/`expected_version` already live inside `ActUpdateDto` per Plan 19-02's design, so no split-args form was needed.
- The RBAC regression case is numbered 42, not a plan-suggested placeholder — grepped the file's actual `// Case N:` comments first (highest existing was 41) per the plan's explicit instruction to avoid renumbering/collision.
- No change was needed to `ui/src/lib/api/client.ts` — its existing `apiCall` wrapper already routes `{ payload }`-shaped args identically to both the Tauri `invoke` path and the axum `POST /api/v1/{name}` JSON-body path, confirmed by inspection (same as `create`/`doReturn`).

## Deviations from Plan

None — all three tasks were executed exactly as specified in the plan's `<action>` blocks; no Rule 1-4 auto-fixes were required.

**`requirements-completed: []` (not `[ACT-02]`) despite the plan frontmatter listing `requirements: [ACT-02]`:** following Plans 19-02/19-03's established precedent, ACT-02 spans Plans 19-02 through 19-05. This plan completes the transport wiring half (`acts_update` reachable via Tauri + HTTP); the requirement is deferred to Plan 19-05, which closes the user-visible UI loop (`ActFormBody`/`ActFormModal` edit mode, `ActDetail` D-07 gating).

## Issues Encountered

None — all three tasks passed verification on the first attempt.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `acts_update` is fully reachable and RBAC-correct on both transports: `build_acts_update` is the single point of authorization (`Action::MutateActs`), called identically by `acts_update` (Tauri) and `handler_update` (axum) — no duplicated logic to drift out of sync.
- `ui/src/lib/api/acts.ts`'s `acts.update(payload)` is ready for Plan 19-05's UI wiring (`ActFormBody`/`ActFormModal` edit mode, `ActDetail`'s «Редактировать» button, D-07 client-side gating that mirrors the server-side `ActType::Handover`-only check already enforced inside `ActService::update`).
- Full `cargo test --workspace` confirmed no regression anywhere in the stack after this plan's changes.

---
*Phase: 19-acts-date-edit*
*Completed: 2026-07-12*

## Self-Check: PASSED

All 6 claimed modified files found on disk; this SUMMARY.md found on disk;
all 3 claimed commit hashes (`d626468`, `e1253ad`, `c47954a`) found in git
log; `ui/src/bindings.ts` confirmed to contain `ActUpdateDto`/`acts_update`
via grep (gitignored, not committed, per established convention).
