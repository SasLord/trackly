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

## Plan 13 — PlacePicker runtime verification NOT performed (auto-approved checkpoint)

Plan 39-13's checkpoint is `human-verify`. `workflow.auto_advance` is enabled, so the
orchestrator auto-approved it per the execute-phase contract. **Nobody has run PlacePicker
in a real webview.** The executor ran only svelte-check / eslint / token+contrast+focus
scripts / `pnpm --dir ui build` — none of which catch Svelte 5 rune runtime errors
(project rule: compile gates ≠ runtime verification; a Chromium harness ≠ WKWebView).

Runtime behaviour UNVERIFIED, to be checked in one batch at the phase's later checkpoints
(39-20 / 39-21) or via /gsd-verify-work:
1. `cargo tauri dev` → showcase (`/showcase`, Admin) → PlacePicker section.
2. Tab into field → panel opens in tree mode with demo roots.
3. ↑/↓ navigate, → expands a branch, Enter selects a leaf → field shows full path.
4. Type lowercase Cyrillic (e.g. "здание") → switches to search mode, highlights match.
5. As Admin, query with no match → D-18 «Создать «…» в «…»» row appears.
6. Second demo block «Значение — архивный узел» → "Архив" badge visible on reopen (D-15).
7. `pnpm --dir ui build`, then repeat 2-6 from a LAN browser tab (WebView2/WKWebView vs
   browser parity). NOTE: LAN browser serves ui/dist — stale until that build is run.

Also note two deviations recorded by the executor, worth a look during that pass:
- Task 1's commit contains the COMPLETE component (tree + search + D-18), not just tree
  mode: the two modes share one state machine and the §10.3 two-stage Escape contract
  spans both, so a tree-only commit would have shipped an inconsistent keyboard contract.
- Showcase registration went to `ui/src/features/showcase/ShowcasePage.svelte` (where all
  other sections register), not the plan's stated `ui/src/pages/ComponentShowcasePage.svelte`,
  which is a thin wrapper with no section list.
