# Deferred items — Phase 39 (place-tree)

Out-of-scope discoveries logged during plan execution (per executor scope-boundary rule:
only auto-fix issues directly caused by the current task's changes; log everything else here
instead of fixing it).

## Plan 12

- **`export_bindings.rs` assertion failure — pre-existing, unrelated to Plan 12.**
  `cargo test -p trackly-app --test export_bindings` fails at
  `crates/trackly-app/tests/export_bindings.rs:304` with `bindings.ts missing
  ActItemDto.device_location_id field`. Confirmed via `git stash`/re-run that this
  failure reproduces identically with and without every one of Plan 12's changes —
  it is caused by `ActItemDto`'s old `location_id`/`location` vocabulary fields
  (`device_location_id`/`device_location`) not having been renamed alongside the
  rest of Phase 39's `place_id`/`full_path` migration. This is exactly the class of
  "old-vocabulary test file" the phase's `prior_wave_context` assigns to Plan 39-22
  — `export_bindings.rs` was left untouched per that boundary.
  **Verified the part of the test that matters for Plan 12's own threat mitigation
  (T-39-12-03) still works**: the `builder().export(...)` call itself (which runs
  BEFORE the failing assertion) succeeds and writes all 12 `places_*` commands plus
  `PlaceDto`/`PlaceNewDto`/`SubtreeStatsDto`/`PlaceContentDto`/`PlacePathDto` into
  `ui/src/bindings.ts` — confirmed via `grep -c "places_" ui/src/bindings.ts` (16
  matches) and spot-checking `placesCreate`/`PlaceDto` function/type signatures in
  the generated file. The specta-registration gap this plan's Task 3 exists to close
  is genuinely closed; the test's own unrelated `ActItemDto` assertion is what fails.
  **Action for Plan 39-22 (or whichever plan next touches `ActItemDto`):** rename
  `device_location_id`/`device_location` to the `place_id`/`full_path` vocabulary (or
  update the two `export_bindings.rs` assertions to match whatever the current field
  names are) so `cargo test -p trackly-app --test export_bindings` is green again.

## Wave 6 (logged by orchestrator)

- **`crates/trackly-app/tests/role_endpoint_matrix.rs` — stale location keys in JSON payloads.**
  Lines ~322-362 still send `"location": null`, `"location_id": null`, `"location_name": null`
  in the device/act RBAC payloads. The file is NOT in Plan 39-22's inventory (that plan's
  31-file consumer sweep does not list it), and Plan 39-12 only appended Cases 45-48 to it.
  The suite is green because these are role-REJECTION cases: RBAC refuses the request before
  the payload is deserialized, so the stale field names are never exercised. Harmless today,
  but misleading, and it would mask a genuine deserialization regression if any of these cases
  ever started passing RBAC.
  **Action for Plan 39-21** (the phase-closing vocabulary sweep): rename these keys to the
  `place_id` vocabulary, or drop them from the payloads if the DTOs no longer carry them.
