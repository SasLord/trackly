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

## Plan 19 — PlaceFormModal/PlaceMoveModal runtime verification NOT performed

Plan 39-19's own `<verification>` block explicitly defers manual verification to Plan 14's
end-to-end checkpoint (Wave 8), since neither modal has an `ActionMenu` to open it from yet.
Only `svelte-check`/`eslint`/`pnpm --dir ui build` ran — same "compile gates ≠ runtime
verification" caveat as Plan 13's PlacePicker. Add these to the same batched UAT pass:

**PlaceFormModal:**
1. Create mode: Тип/Родительское место/Порядок/Складское место visible; Уровень hidden
   until Тип = «Этаж».
2. With Тип = «Этаж», type `-1` and `0` into Уровень — both accepted, no error.
3. Type `1.5` into Уровень, submit — inline error "Уровень этажа — целое число. Подвал —
   отрицательное значение." under Уровень, submit blocked.
4. Pick a parent whose kind is «Здание» — Тип pre-fills to «Этаж»; manually override to
   «Помещение» and reselect a different parent — Тип must NOT be clobbered back.
5. Submit a duplicate sibling name — inline error under Название matches §11.2's exact copy,
   field goes `.invalid`.
6. Rename mode: ONLY Название renders (see Plan 19's key-decisions for why); submitting
   renames via `places_rename`, toast "Место переименовано".
7. Create nested storage node (D-09 pattern): create a place, then "Создать вложенное место"
   with Складское место checked — resulting node has `is_storage=true`.

**PlaceMoveModal:**
1. Open on a node with 3 nested places + 47 devices → callout text "Вместе с местом переедет
   3 вложенных места и 47 устройств." exactly.
2. Open on a node with 0 nested places + 47 devices → "Вместе с местом переедет 47
   устройств." (nested-places clause fully omitted).
3. Open on a completely empty node → no callout renders at all.
4. Select the node's own descendant as target, submit → inline cycle error "Нельзя
   переместить место внутрь самого себя или своего вложенного места." renders under
   PlacePicker, modal stays open.
5. Successful move → toast "Место перемещено", modal closes, `onMoved` fires with the
   updated `PlaceDto`.
6. «Переместить» stays disabled until a target place is picked.
7. `pnpm --dir ui build`, repeat all of the above from a LAN browser tab.
  which is a thin wrapper with no section list.
