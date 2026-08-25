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

## Plan 15 — Device-family PlacePicker wiring runtime verification NOT performed

Only `svelte-check`/`eslint`/`pnpm --dir ui build` ran on the frontend; the backend CSV
place-resolution path IS verified via `cargo test -p trackly-app --test devices_csv_import`
(11/11 passing, including a new regression test for the not-found error copy). The following
have NOT been exercised in a real webview — add to the same batched UAT pass:

**Device create/edit form (DeviceFormBody.svelte):**
1. Open "Создать устройство" — Место field renders as PlacePicker (not a text input),
   placeholder "Выберите место", required (submit disabled until a place is selected).
2. Pick a place via the tree panel — field shows the full path, submit succeeds, created
   device shows the correct `full_path` in the devices list.
3. Edit an existing device — PlacePicker pre-fills with the device's current place
   (verify the D-15 archived-value exception if the device's place happens to be archived).
4. Change the place on an existing device, save — devices list reflects the new `full_path`
   immediately without a full page reload.

**Printer creation (PrinterCreateModal.svelte):**
1. "Завести принтер" — Место field renders as PlacePicker, optional (submit succeeds with
   no place selected, same as before).
2. Pick a place, submit — created printer's underlying device has the correct `place_id`.

**CSV import (DeviceImportCsvModal.svelte):**
1. Step 3 (mapping): the place column's dropdown option reads "Место" (not "Расположение").
2. Import a CSV whose place column value does NOT exist in the tree — Step 4's error list
   shows exactly "Строка N: место «...» не найдено в дереве." (no duplicated "Строка N:"
   prefix — this was a real bug found and fixed by Plan 15, verified at the backend-test
   level only).
3. Import a CSV whose place column value DOES match an existing place's full path exactly
   (case-insensitive) — device inserts with the correct `place_id`.
4. Auto-mapping: a CSV with a "Место" (or "место"/"Place"/"place") header column
   auto-maps to the place field without manual re-mapping.

**Devices list/table (DeviceList.svelte / DeviceListRow.svelte / DeviceGroupRow.svelte):**
1. Devices page table header reads "Место" (not "Расположение").
2. Ungrouped rows show each device's `full_path` in the Место column.
3. Grouped rows (same name+model+specs+kit+state+place+status) show the group's
   representative `full_path`; changing a device's place moves it out of its old group.

## D-11.3 cross-plan fix (post-Plan-16) — runtime verification NOT performed

Plan 39-16 shipped D-11.3's "Перевести устройство в статус «На складе»" checkbox on the
WRONG surface (`OperationModal.svelte`, cartridges) and never implemented it on the RIGHT
one (`DeviceFormBody.svelte`, devices — see 39-16-SUMMARY.md's own flag to Plans 17/18: "the
device/act-status field pattern... should re-verify"). This fix:

- **Removed** the checkbox (and its now-fully-dead state: `storagePlaceIds`,
  `isStoragePlace`, `storageStatusSuggested`, the `cartridge_storage_place_ids` fetch
  `$effect`, the `Checkbox`/`apiCall` imports) from both `OperationModal.svelte` render
  sites (install/to_refill block and return_to_stock/from_refill block). Verified via grep
  that `storagePlaceIds`/`isStoragePlace`/`storageStatusSuggested` had zero other
  usages in the file (no D-11.4 "Возврат на склад → складское место" prefill exists there
  today — that half of D-13's prefill promise was apparently never built for cartridges;
  out of scope for this fix, not re-flagging beyond this note since it is a pre-existing
  gap, not something this fix touched or regressed).
- **Added** the checkbox to `DeviceFormBody.svelte`'s Место field block, gated on the same
  kind of storage-place-set lookup (reused the existing `cartridge_storage_place_ids`
  Tauri/HTTP command — confirmed via `crates/trackly-app/src/services/cartridge_service.rs`
  and `trackly-infra/src/repos/places_sqlite.rs` that the underlying
  `PlaceRepo::list_storage_place_ids` query is place-tree-derived and entity-agnostic
  despite the command's cartridge-era name, and is `Action::ReadData`-gated rather than
  cartridge-specific). Checking it sets `statusId` to `'1'` («На складе», confirmed against
  `crates/trackly-app/src/services/device_service.rs::resolve_status_id`/
  `status_id_to_name` as the real seed value, not assumed) — this DOES flow into the
  submitted `DeviceNew.status_id`/`DevicePatch.status_id` payload, unlike the cartridge
  checkbox which had no backend field to affect.

Compile/lint/build gates pass (svelte-check: 274 files/14 errors/54 warnings — identical
before and after via `git checkout --`/`git apply` A-B comparison, zero new errors; eslint
clean on both touched files; `pnpm --dir ui build` succeeds). **Runtime is UNVERIFIED** — add
to the batched UAT pass:

**OperationModal.svelte (cartridges) — checkbox should be GONE:**
1. Open any of the 5 cartridge operations (Install/Возврат на склад/В заправку/Из заправки/
   Списание) and pick a place known to be a storage place — confirm NO "Перевести устройство
   в статус «На складе»" checkbox appears anywhere in the modal (it was never wired to any
   payload field and always described the wrong entity — cartridges don't have a device-style
   status override).

**DeviceFormBody.svelte (devices) — checkbox should be NEW and functional:**
1. Create a new device, pick a place that is NOT a storage place — no checkbox appears;
   Статус dropdown behaves as before (fully manual).
2. Pick a place that IS a storage place (or a descendant of one, exercising D-11.4 ancestor
   inheritance) — checkbox "Перевести устройство в статус «На складе»" appears under Место,
   default checked, and Статус silently reads «На складе» (id 1).
3. Uncheck the checkbox — Статус dropdown becomes freely editable again; pick a different
   status (e.g. «В работе») and submit — the saved device's real status is NOT «На складе»
   (confirms the "no forced change" half of D-11.3/D-10).
4. Leave the checkbox checked and submit — the saved device's real status IS «На складе»
   (confirms the payload actually changes, not just cosmetic).
5. Edit an existing device that currently has some other status (e.g. «В работе»), change its
   place to a storage place — checkbox appears default-checked; verify this does NOT silently
   flip the status until the operator actually submits (i.e. no premature write), and that
   submitting with the box checked does flip it as expected.
6. Switch the place away from a storage place after the checkbox was checked — checkbox
   disappears; confirm the previously-applied `statusId='1'` value is NOT silently reverted
   (matches this fix's decision to only apply forward, never auto-revert on leave — flag if
   this reads as confusing in practice).

## Plan 17 — Act-family PlacePicker wiring runtime verification NOT performed

Only `svelte-check`/`eslint`/`node scripts/check-tokens.mjs`/`pnpm --dir ui build` ran on the
frontend. The following have NOT been exercised in a real webview — add to the batched UAT pass:

**Act create/edit form (ActFormBody.svelte):**
1. Open "Создать акт" — Место field renders as PlacePicker (not a text input), placeholder
   "Выберите место"; field is optional (act.place_id is nullable, submit does not require it).
2. Pick a place, submit — created act's `full_path` shows the correct live-resolved path in
   ActDetail; the printed act shows `place_path_snapshot` frozen at write time (D-16).
3. Edit an existing act, change its place, save — ActDetail's "Расположение" field reflects
   the NEW `full_path` immediately; a PREVIOUSLY printed act for the earlier place is
   unaffected (D-16 freeze — re-render the old print preview and confirm it still shows the
   old path, not the new one).

**Bulk return (ReturnModal.svelte, apply-to-all path):**
1. Open "Возврат по акту" — if the org's place tree has any storage places, a row of chip
   buttons appears above the Место field, one chip per storage place, labelled with its full
   path; the first one is preselected as the bulk place when the modal opens.
2. Click a different chip — PlacePicker's field updates to match; click a node inside
   PlacePicker directly — chip selection highlight (variant=primary) follows.
3. Submit a bulk return with the default (chip-preselected) place — the created return act's
   items carry the correct `place_id_override`/bulk place server-side.
4. If the org's place tree has NO storage places (edge case) — no chip row renders, PlacePicker
   alone remains fully usable, default preselection is skipped (no forced value).

**Bulk return edit mode (ReturnModal.svelte, `mode="edit"`):**
1. Edit an existing return act — modal opens in per-row mode (apply_to_all=false) with rows
   prefilled from the return's saved `place_id_override` per item; the storage quick-pick
   chips still populate the BULK field (which starts unused unless "Применить ко всем" is
   re-enabled).

**Per-row return override (ReturnItemsTable.svelte):**
1. Uncheck "Применить ко всем" — each checked row's Место cell becomes an editable PlacePicker
   pre-filled with the device's own `device_place_id` (or empty for newly-addable rows in edit
   mode).
2. With "Применить ко всем" checked, the disabled per-row Место cell shows the bulk place
   (mirrored, not blank) — confirms the "effective value" preview behavior.
3. Submit a per-row return with two rows sharing the same place + condition and one row with a
   different condition — confirms `buildReturnItems()`'s coalesce/split behavior still holds
   with `place_id_override` as part of the composite key (was previously
   `locationOverrideName`).

**ActDetail.svelte / ActFormItemsTable.svelte (rename-only, low risk):**
1. Any act's detail card shows "Расположение" populated from `full_path` (was `location`) —
   confirm no blank/undefined regression for acts with a place set.
2. Adding a new item to an act via the "на складе" device search still returns results (search
   payload now sends `place_id: null` instead of `location_id: null` — a no-op rename since
   this call always passed `null` here; not expected to change search behavior, but worth one
   smoke-test add-item pass).

## Plan 18 — Reports place filter runtime verification NOT performed

Only `svelte-check` (0 errors, down from the 4 pre-existing errors this plan's Task 3 territory
accounted for), `eslint`, `node scripts/check-tokens.mjs`, and `pnpm --dir ui build` ran on the
frontend. The following have NOT been exercised in a real webview — add to the batched UAT pass:

**ReportFilters.svelte (place filter, reactivated for the first time since GAP-R4):**
1. Open Отчёты for any domain/report tab — a "Место" field renders as PlacePicker (not a text
   input) next to a "Складское место" dropdown (Все/На складе/В эксплуатации); the hint
   "Включая вложенные места" is visible under the PlacePicker field.
2. Pick a building-level place — confirm the report table narrows to rows whose place is that
   building OR any of its floors/rooms/nested places (D-28 subtree semantics; the backend SQL
   was verified to already implement this via `WITH RECURSIVE subtree(...)` per Plan 39-10, but
   the wiring from PlacePicker's `onChange` through `filter.place_id` to the actual API call has
   not been exercised end-to-end).
3. Select "На складе" in the Складское место dropdown — table narrows to items in storage
   places (ancestor-inclusive per D-11.4); select "В эксплуатации" — narrows to non-storage;
   confirm this filter behaves independently of any status filter (no dropdown/label merge).
4. Clear the place filter (PlacePicker's ghost clear button) — table returns to unfiltered.
5. Switching domain/report tabs resets `filter = {}` (existing `onDomainChange` behavior) —
   confirm the Место field visually clears too, not just the underlying state.

**ReportsPage.svelte / ReportTable.svelte (place_path columns, D-26 short-path display):**
1. Every report table showing a "Место" column (Акты, Возвраты, В работе, На складе,
   consumption/refills, cartridge in_use/in_stock, all 4 Заявки tabs) renders a short
   (last-two-segment) path in the cell and the full path on hover (`title` attribute) for
   places nested 3+ levels deep; places with ≤2 segments show the full path in both.
2. Snapshot reports (В работе / На складе for devices and cartridges) still group rows by
   place under the correct separator row — this is the runtime behavior the ReportRow rename
   fix (Task 4) targets; a broken separatorKey would have silently collapsed all rows into one
   group with no compile-time signal, so this is worth a dedicated visual check.
3. The Заявки "Принтер / Локация" column (`place_path` key, label unified to "Место" per this
   plan's Task 1) — confirm it still shows the combined "Принтер, Место" text
   (`combine_printer_and_place`, Plan 39-10), not just a bare place path; the label rename to
   "Место" may read as slightly misleading for this specific column since its value combines
   two things, worth a product-copy sanity check during UAT.

**PrinterDetail.svelte / PrinterSelect.svelte / GroupedPrinterSelect.svelte (Task 3 renames):**
1. Printer detail page's device meta section shows the device's place (was blank/broken before
   this fix if `deviceData?.location` never resolved post-rename).
2. Printer dropdown (used in cartridge Install flow) shows "<printer name> — <place>" for
   printers with a resolved place, bare name otherwise.
3. Grouped printer select (create-request flow) groups printers by place correctly, "Без
   расположения" bucket for printers with no place.

## Wave 7 close — PlaceFormModal mount contract (orchestrator)

`ui/src/features/places/PlaceFormModal.svelte:74-76` initialises `name`/`parentId` from the
`mode`/`place`/`defaultParentId` props at CONSTRUCTION time:

    let name = $state(mode === 'rename' && place ? place.name : '');
    let parentId = $state<number | null>(defaultParentId);

svelte-check flags these as `state_referenced_locally` (4 warnings). They capture only the
INITIAL prop values. This is correct ONLY if the modal gets a fresh component instance per
open. If a consumer keeps it mounted and toggles an `open` prop instead, reopening it for a
DIFFERENT place shows the PREVIOUS place's name / parent — a silent wrong-data bug that no
compile gate catches (project rule: compile gates ≠ Svelte 5 rune runtime).

There is no consumer yet — Plan 39-14 is the first, wiring the tree's `ActionMenu` to these
modals. **Contract for Plan 39-14:** mount both `PlaceFormModal` and `PlaceMoveModal` inside
`{#if open}` (or otherwise force a fresh instance per open, e.g. a `{#key}` block), so their
initial-value state is always built from the current target node. If you instead keep them
mounted, you MUST convert these to `$derived`/`$effect` and say so.

Add to the batched UAT pass: open rename on node A, close, open rename on node B — the field
must show B's name, not A's.

## Plan 14 — PlaceTree/PlaceTreeNode runtime verification NOT performed

Only `svelte-check` (0 errors, 56 warnings — 2 new, both the accepted
`state_referenced_locally` pattern on `PlaceMoveModal.svelte`'s new
`defaultParentId` prop), `eslint` (0 problems after adding the missing
`DragEvent` browser global to `eslint.config.js`), `node scripts/check-tokens.mjs`
/`check-contrast.mjs`/`check-focus-outline.mjs` (all PASS), and `pnpm --dir ui build`
ran on the frontend. Per this plan's own `<verification>` block, full interactive
verification (keyboard, drag-drop, role gating, Tauri + LAN browser parity) is
explicitly deferred to Plan 20's end-of-wave checkpoint, once the right panel
exists to complete the screen. Nothing below has been exercised in a real webview:

**Routing / sidebar / master-detail shell (Task 1):**
1. As Admin, sidebar shows "Места" immediately after "Карта", before the first
   divider; navigating to it renders PageHeader "Места" + primary "Создать место".
2. As Manager, sidebar shows "Места" too, but with NO "Создать место" button.
3. As Employee, "Места" does not appear in the sidebar at all, and `#/places`
   is not directly reachable (falls through to AccessDenied per the employee
   route table).
4. Panels are 35%/65%, each scrolls independently (app-shell `.content{overflow:
   hidden}` convention) — resize the window narrower than 1099px, confirm the
   380px/1fr fallback.
5. `#/places?id=<some place id>` typed directly into the address bar (or via
   `window.location.hash =` in devtools) pre-selects and expands the tree to
   that node on load.
6. Selecting a different node in the tree updates the hash to `#/places?id=<new
   id>` WITHOUT adding a new browser-history entry (back button should not step
   through every node selection).

**PlaceTree — structure/sort/counters (Task 2, D-05/D-25):**
1. Root places render at `--depth: 0`; a grandchild renders at `--depth: 2`
   with the correct cumulative left padding.
2. Siblings with an explicit "Порядок" win over level/name; floor siblings
   (level 0, negative, positive, no explicit order) sort numerically, not
   alphabetically; siblings with neither sort naturally ("2 этаж" before
   "10 этаж").
3. A place with zero devices+cartridges (incl. nested) shows NO counter span
   at all — not "0" — confirm by inspecting the DOM, not just visually.
4. A place with content renders the correct SUM across its whole subtree in
   `.tr-mono`, with `title="Всего с вложенными: N"`.
5. Toggling "Показывать архивные" on/off reloads the tree and shows/hides
   archived nodes (with the "Архив" badge, tertiary-colored name).
6. "Обновить" button reloads the tree from a fresh `places_list_all` call.
7. Empty-tree states render the correct Admin vs Manager copy (§14.2).

**Keyboard/ARIA (§8.5, Фаза 30 parity — the phase's hardest-to-fake-with-compile-gates area):**
1. Tab into the tree lands on exactly ONE row (roving tabindex); ↑/↓ move
   among VISIBLE nodes only (collapsed subtrees skipped); Home/End jump to
   first/last visible node.
2. → expands a collapsed node with children, or moves into the first child if
   already expanded; ← collapses an expanded node, or moves to the parent if
   already collapsed/a leaf.
3. Enter selects the focused node (right panel selection — currently just
   flips the static placeholder's... actually nothing visible changes yet
   since Plan 20 owns the real content; confirm via the hash update instead).
4. F2 (Admin only) opens "Переименовать" for the focused node; Manager
   pressing F2 does nothing.
5. Typing into the search field switches to flat search-results mode (full
   path per row, no indentation); Escape clears the search and returns to
   tree mode, restoring the previous expand/select state.
6. Screen reader (or the `aria-live="polite"` region's text, inspectable via
   devtools) announces search-result counts, and move/archive/delete outcomes.

**ActionMenu wiring (§8.3) and mutation modals (Task 2, ties into Plan 19):**
1. "Переименовать" opens `PlaceFormModal` in rename mode for the correct node
   (verify against the mount-contract regression noted above — this is the
   FIRST real consumer of that contract).
2. "Создать вложенное место" opens create mode with the parent pre-set to the
   clicked node; after save, the tree reloads AND auto-expands+selects+
   scrolls to the new node.
3. "Переместить в…" opens `PlaceMoveModal` with NOTHING pre-selected — submit
   stays disabled until a target is picked via `PlacePicker`.
4. "Архивировать"/"Вернуть из архива" (label flips correctly per node state)
   opens the inline confirm with the exact §11.4 copy; submit calls the
   correct one of `places_archive`/`places_unarchive`.
5. "Удалить" opens the inline confirm (§11.5); deleting an EMPTY node
   succeeds with toast "Место удалено"; deleting a NON-empty node replaces
   the modal body with the server's literal D-14 message and swaps the
   footer to "Показать содержимое" (selects the node, closes the modal) /
   "Архивировать" (pivots straight into the archive confirm for the same
   node) — "Удалить" itself must be gone from the footer in this state.
6. "Показать содержимое" quick smoke: repeat the delete-blocked flow starting
   from a node whose PARENT is currently collapsed — confirm selection still
   works (Plan 20 will render real content; this plan only needs the
   selection/hash side-effect to fire correctly).

**Drag-n-drop (§8.4/D-03/D-21 — Admin only, native HTML5 DnD):**
1. As Manager, rows are NOT draggable at all (no drag ghost appears).
2. As Admin, dragging a row over an INVALID target (itself, or a descendant)
   shows the danger/no-drop styling and the drop is ignored (no modal opens).
3. Dragging onto a VALID target shows the accent-soft/inset-ring styling;
   dropping opens `PlaceMoveModal` with that target ALREADY selected as the
   destination (confirm the picker field shows the right full path
   immediately, not empty) and "Переместить" already enabled — the modal
   still shows the consequences callout and requires an explicit confirm
   click (never a silent move).
4. Starting a drag reveals the "В корень дерева" dashed drop zone at the
   bottom of the list; dropping there opens `PlaceMoveModal` pre-filled for
   root (confirm "Переместить" is enabled immediately here too — this is the
   scenario the `defaultParentId`/`targetChosen` fix in this plan exists for;
   before the fix this exact flow was unreachable).
5. Canceling the pre-filled move modal (either path) performs NO mutation.

**LAN browser parity:** `pnpm --dir ui build`, then repeat the keyboard and
drag-drop checks above from a LAN browser tab — HTML5 DnD and
`aria-activedescendant`-style composite-widget patterns are exactly the class
of thing that can behave differently between WebView2/WKWebView and a real
browser (per project convention: compile gates catch neither).
