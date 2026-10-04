---
scope: frontend
phase: 41
depth: standard
files_reviewed: 40
critical: 0
warning: 5
info: 7
status: findings
reviewed: 2026-10-04
---

# Phase 41 — Frontend code review

## Summary

All 40 files were read. For the six pre-existing shared files, only the phase-41 delta was reviewed. The diff base given in the task (`199438c9…^`) is a Phase-13 commit, so a diff against it pulls in about 2200 unrelated commits. The real phase start is `f9d07be4^`, and every diff below was taken against that.

**Runes (priority 1): no defect found.** I traced every `$effect` in `GroupsPage`, `GroupTree`, `GroupPanel`, `GroupTypePanel`, `GroupContentsTable`, `GroupPropertiesForm`, `GroupMoveModal` and `DeviceFormBody`.
- Each one reads only its inputs, and each state write that the effect could itself read is inside `untrack`. There is no `effect_update_depth_exceeded` path.
- Async work uses a `loadSeq`/`cardSeq`/`searchSeq` counter or a `cancelled` flag, so a stale response cannot overwrite a newer one.
- `{#key}` on the selected node removes the `groupId`-changes-in-place case entirely.
- The `state_referenced_locally` warnings in `features/groups/**` are suppressed deliberately with `svelte-ignore`. The 68 svelte-check warnings are the pre-existing baseline, and 0 are new in phase-41 files.

**Gates run read-only.**
- `svelte-check`: 0 errors, 68 warnings (baseline).
- `--selftest` and live run of `check-reorder`, `check-group-vocabulary`, `check-groups-section`, `check-place-tree-invalidation` and `check-print-isolation`: all green.
- I also ran the gates against mutated copies of `ui/src` in the scratchpad. That is how W-01 and W-02 below were proven rather than guessed.

**INV-7 (priority 2): no group mutation is missing from the registry.** I enumerated from the server side.
- `groups_move`, `groups_add_devices` and `groups_set_parent` return `changed_place_ids`, and all three are registered.
- `groups_remove_devices` and `groups_delete` are deliberately absent. `group_service.rs` states "место не меняется" for both. `release_device_in_tx` is called with no place change, and `delete_group_in_tx` leaves `devices.place_id` untouched.
- `places_move_subtree_contents` now also moves groups. It is already a registered marker, and `PlaceContents.handleMoveConfirm` notifies.
- Every client call site notifies. The gap is in the gate's matching, not in the code (see W-01).

**Deferred items.**
- 41-18 is confirmed resolved: svelte-check shows 0 errors.
- 41-23 is confirmed: the `GroupTypePanel` hint is a false permission claim (W-03). The device-oriented empty block in `MovementTimeline` is still device-oriented, and `GroupPanel` handles it by rendering its own block. I found one sibling wording issue (IN-05).

**Privacy:** clean. The only addresses are `192.168.1.10`, `192.168.1.100` and the doc-example MAC `00:1b:44:11:3a:b7`. No names, emails or organisation data in any reviewed file.

**Shared-component blast radius (priority 5): additive and backward-compatible.**
- `Dropdown.getGroupSection` is optional, and without it the markup is unchanged. Headings are `role="presentation"` and stay out of `activeIndex` navigation.
- `MovementTimeline` gains two optional props with unchanged defaults.
- `TableRow` changes only the chevron `aria-label` (see IN-07).
- No `@media print` rule was added in `ReportTable`. Its batch layout is screen-only, and `check-print-isolation` passes.

## Warnings

### W-01: INV-7 gate is blind to aliased `groups` imports — `GroupsPage.removeFromParent` is unprotected

**File:** `ui/src/features/groups/GroupsPage.svelte:24,238` and `ui/scripts/check-place-tree-invalidation.mjs:745-747`

**Defect:** `GroupsPage` imports `groups as groupsApi` and calls `groupsApi.setParent(…)`. The gate's `DIRECT_CALL_MARKERS` are plain substring needles (`'groups.setParent('`), and `groupsApi.setParent(` does not contain `groups.setParent(`. So the one call site that moves a group to the root is not checked at all. The comment on that function says "реестровый гейт INV-7" as if it were covered.

**Scenario (reproduced).** I deleted `notifyPlaceContentChanged(result.changed_place_ids);` from `removeFromParent` in a scratch copy. `node scripts/check-place-tree-invalidation.mjs --src=<copy>` printed `PASS — 0 нарушений`. The same deletion in `GroupContentsTable.detachGroup`, which uses the unaliased name, is flagged. A future edit that drops the call leaves the "Места" counters stale and CI stays green. This is the exact false-green class INV-7 exists to stop.

**Fix:**
- Make `GroupsPage` use the same identifier as the registry: `import { groups } from '$lib/api/groups'` (rename the local `groups` state variables if any collide).
- Harden the gate so the needle is alias-proof. Match `/\b\w*[gG]roups\w*\.(move|addDevices|setParent)\(/`, or add `'groupsApi.setParent('`, `'groupsApi.move('` and `'groupsApi.addDevices('`.
- Add a gate check that fails if `groups as <alias>` is imported anywhere under `src/features`.

### W-02: `check-group-vocabulary` has undocumented holes beyond the known `'групп'` one

**File:** `ui/scripts/check-group-vocabulary.mjs:101,115-129`

**Defect 1 — markers match by prefix.** A marker whitelists any literal that begins with it, because the span check is `idx >= a && idx < b` and the rest of the word or phrase is never looked at. Confirmed in a scratch copy:
- `<p>Группа похожих устройств</p>` appended to `ReportTable.svelte` (marker `'Группа'`): **passes**.
- `<p>Группы похожих</p>` appended to `sidebar-config.ts` (marker `'Группы'`): **passes**.
- Only `<p>Группы устройств</p>` in `ReportTable` was caught.

Those are the very strings (a "группа" that means a collapse of identical devices) that GRD-06 forbids.

**Defect 2 — `//` inside a string literal hides the rest of the line.** `stripComments` treats `//` after whitespace as a comment start anywhere. `export const T = 'Все // группы';` in `reorder.ts` passes the gate.

**Scenario:** the next edit that reintroduces "Группа" in the sidebar or the report table in the old sense merges green, and the vocabulary regresses without a signal. The selftest only exercises `Группировать`/`Сгруппировать` against these files, so it does not show the hole.

**Fix:**
- Make markers whole-literal. After matching a marker, require that the next character is not a Cyrillic letter, or compare the full text node or string literal against the allowed text.
- Add selftest fixtures `'Группа похожих'` in `ReportTable` and `'Группы похожих'` in `sidebar-config`, both expecting 1 violation.
- For defect 2, only treat `//` as a comment when the preceding text on the line has balanced quotes, as `check-place-tree-invalidation` does, or document it as a known blind spot.

### W-03: `GroupTypePanel` tells a read-only user they may edit (confirmed deferred item), plus a sibling

**File:** `ui/src/features/groups/GroupTypePanel.svelte:130`; sibling at `ui/src/features/groups/GroupTypePropertiesTable.svelte:484`

**Defect:** line 130 always renders «Код и поведение типа изменить нельзя. Название и набор свойств — можно.», but `canEdit={isAdmin}`. For a manager the panel is read-only. There is also no rename control in this panel for anyone: rename is only in the tree menu, which is hidden from non-admins. The sibling is the empty-state body «Добавьте первое свойство в строке ниже…». The add row is rendered only under `{#if canEdit}`, so a manager sees an instruction pointing at a row that does not exist.

**Scenario:** a manager opens a freshly created type with no properties. The panel says "Добавьте первое свойство в строке ниже" and offers nothing below.

**Fix:**
- `GroupTypePanel`: show the sentence only when `canEdit`. Otherwise show «Код и поведение типа изменить нельзя. Свойства настраивает администратор.»
- `GroupTypePropertiesTable`: pass `emptyBody={canEdit ? '…в строке ниже…' : 'Свойства настраивает администратор.'}`.

### W-04: "Скрыть свойство" promises reversibility that the server does not always provide

**File:** `ui/src/features/groups/GroupTypePropertiesTable.svelte:649-652` (modal) and `:369`

**Defect:** the confirm text always says «Заполненные значения останутся в базе, свойство можно будет вернуть.» The server decides: `outcome.archived ? 'Свойство скрыто' : 'Свойство удалено'` (line 369). A property with `filled_group_count === 0` is physically deleted, and "Вернуть" will never be available.

**Scenario:** an admin hides an unfilled property, trusts "можно будет вернуть", and later finds it gone.

**Fix:** the client already knows `filled_group_count`. Branch the modal text on it: if `> 0`, «Свойство исчезнет из форм групп. Значения останутся в базе, свойство можно вернуть.»; if `0`, «Свойство ни у одной группы не заполнено — оно будет удалено без возможности возврата.» Consider titling the action «Скрыть или удалить».

### W-05: `GroupAddDevicesModal` filters by place with a client-side string-prefix subtree rule

**File:** `ui/src/features/groups/GroupAddDevicesModal.svelte:70-73,105-120` (design comment at `:9-15`)

**Defect:** "device lies under place P" is reimplemented as `full === placePath || full.startsWith(placePath + ' / ')`. That is a JS mirror of the server's place-tree semantics (item 2 of the review criteria), built on a display string with a hard-coded separator.
- A root place whose name contains `" / "`, or a rename, makes it match the wrong subtree.
- The header comment says "Фильтра по месту нет нигде", yet `DeviceFilter.place_id` exists (`devices.list` receives `place_id: null` here).
- The `FETCH_CAP = 1000` loop silently stops, so devices past the first 1000 rows never appear under a narrow place filter, and there is no "ещё есть" hint.

**Scenario:** an admin picks place «Каб. 1» and expects to see its subtree. Devices under a differently-shaped path appear or vanish, and in a large inventory the list ends at 1000 without saying so.

**Fix:** use the server filter or an id-based subtree check. Either pass `place_id` to `devices.list`/`search` if the server supports the subtree, or add a server read that returns the subtree place ids (like `places_contents{nested}`) and filter on `d.place_id`. If the client filter stays, show «Показаны первые 1000 — уточните фильтры» when `FETCH_CAP` is hit.

## Info

### IN-01: `check-reorder --selftest` mutates an embedded copy, not `reorder.ts`

**File:** `ui/scripts/check-reorder.mjs:148-163,174-199`

**Issue:** the live run correctly executes the real `reorder.ts` against `cases.json`, and the mutant anchors are unique in `GOOD`. But the mutants are applied to a hand-copied `GOOD` string, so the selftest proves the fixture kills mutants of a copy. If `reorder.ts` is refactored, `GOOD` can drift unnoticed. The fixture has no non-empty case with `from` out of range or negative, and the `from < 0 || from >= length` guard has no mutant.

**Fix:** derive the selftest subject from `reorder.ts` (read the file, apply the anchored `replace`, assert each anchor occurs exactly once in the real source). Add a `from = -1` and a `from = length` case on a non-empty array, plus a guard-removal mutant.

### IN-02: `GroupPanel.handleSaved` can be overwritten by an older in-flight `loadCard`

**File:** `ui/src/features/groups/GroupPanel.svelte:156-159` (vs `:79-94`)

**Issue:** `handleSaved` assigns `card = next` without bumping `cardSeq`. A `loadCard()` started earlier (for example by a `refreshToken` bump) can resolve after the save and replace the fresh card with a pre-save one. The form then holds a stale `version`, and the next save fails with `OPTIMISTIC_LOCK_MISMATCH`. The window is small.

**Fix:** `cardSeq++;` at the top of `handleSaved`.

### IN-03: `GroupContentsTable.loadComposition` can drop a just-expanded child and leave a permanent "Загрузка…"

**File:** `ui/src/features/groups/GroupContentsTable.svelte:85-114`, `136-149`

**Issue:** `loadComposition` builds `fresh` only from the `expanded` ids it read before its awaits, then does `childComp = fresh`. If the user expands a nested group and that `toggleChild` fetch completes before the reload finishes, its data is discarded. `expanded[id]` stays `true` with no `childComp[id]`, so the row shows «Загрузка…» forever (`!state` branch) until another reload.

**Fix:** merge instead of replace, `childComp = { ...childComp, ...fresh }` (drop only ids no longer expanded). Alternatively, re-read `expanded` after the awaits and fetch any id that is missing.

### IN-04: `GroupDeleteModal` hand-rolls Russian inflection and omits nested groups

**File:** `ui/src/features/groups/GroupDeleteModal.svelte:54-63`

**Issue:** the verb «освободится/освободятся» is chosen with an inline `% 10 === 1 && % 100 !== 11` test, duplicating `pluralizeRu` (which `GroupMoveModal` uses for the same job). The text also does not say that nested groups become root groups, though the server reports it (`nested_group_count`, and the comment on `delete_group` says so).

**Fix:** `pluralizeRu(n, ['освободится', 'освободятся', 'освободятся'])`. Add a sentence for `nested_group_count > 0`: «Вложенные группы станут корневыми.»

### IN-05: History-error text in `GroupPanel` refers to a window that does not exist

**File:** `ui/src/features/groups/GroupPanel.svelte:229-243` (renders `MovementTimeline` with `loadError`); string at `ui/src/lib/components/MovementTimeline.svelte:109-111`

**Issue:** on a failed history request the group panel shows «…Закройте окно и попробуйте ещё раз.», written for a modal. In a tab nothing needs closing.

**Fix:** let `MovementTimeline` take an optional `errorText` (default unchanged) and pass «Не удалось загрузить историю. Откройте вкладку ещё раз.» from `GroupPanel`.

### IN-06: `DeviceList` pagination label "Свёрнуто: N" is the row count, not the number collapsed

**File:** `ui/src/features/devices/DeviceList.svelte:99`

**Issue:** `groups.length` is the number of resulting rows. 30 devices collapsing into 12 rows reads «Свёрнуто: 12», which suggests 12 were collapsed.

**Fix:** «Строк: {groups.length}» or «Показано {groups.length} (одинаковые свёрнуты)».

### IN-07: `ReportTable` batch rows and generic chevron labels

**File:** `ui/src/features/reports/ReportTable.svelte` (batch header block) and `ui/src/lib/components/TableRow.svelte:61`

**Issue (a):** a batch whose header is visible but whose members were filtered out (device-type filter, `LIMIT 1000`) shows `(N устройств)` and a chevron that expands to nothing.

**Issue (b):** the batch chevron uses `aria-label` «Развернуть»/«Свернуть», and the shared `TableRow` chevron changed from «…группу» to the same bare labels. With several expandable rows, a screen reader announces identical buttons with no row context.

**Fix:** (a) hide the chevron when `members.length === 0`. (b) Use `aria-label={`${open ? 'Свернуть' : 'Развернуть'}: ${name}`}`, or `aria-labelledby` pointing at the name span.

---

_Reviewed: 2026-10-04_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
