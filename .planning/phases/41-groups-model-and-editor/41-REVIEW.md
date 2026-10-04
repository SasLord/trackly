---
phase: 41
depth: standard
files_reviewed: 106
critical: 0
warning: 8
info: 13
status: findings
reviewed: 2026-10-04
parts:
  - 41-REVIEW-backend.md
  - 41-REVIEW-frontend.md
---

# Phase 41 — Code Review

Scope came from the 26 SUMMARY frontmatters: **106 source files** (66 Rust + SQL, 40 frontend).
That is well past the threshold where a single reviewer goes shallow, so the review was split
into two agents at `standard` depth, each with its own report:

- [`41-REVIEW-backend.md`](41-REVIEW-backend.md) — Rust crates and migrations, 66 files, 0 critical / 3 warning / 6 info
- [`41-REVIEW-frontend.md`](41-REVIEW-frontend.md) — Svelte, TS, gate scripts, 40 files, 0 critical / 5 warning / 7 info

Both reports passed the privacy gate (`scripts/check-privacy.mjs`), which matters because
agent-written review artifacts in this project have previously quoted real organization data
pulled out of git history.

## Verdict

**No Critical findings.** Every invariant the phase was built around was checked and holds:

| Invariant | Result |
|---|---|
| One transaction per group operation; no `commit()` in `group_place.rs` | holds |
| D-19/D-21: individual movement of a group member rejected; clearing the place counts as a change; a placeless group does not lock members | holds |
| All 8 `act_service.rs` write sites paired with the correct release variant (direct = unconditional, restore = release-if-locked) | holds |
| Cartridge cascade cannot touch a locked member (`WHERE place_id IS NULL`) | holds |
| No other server-side writer of `devices.place_id` | holds |
| Permissions: every HTTP route goes through the shared `build_*` authorizer; adapters duplicate no rules | holds |
| V045/V046 additive only — the refinery `PRAGMA foreign_keys = OFF` cascade-wipe class cannot recur | holds |
| Svelte 5 runes: no `effect_update_depth_exceeded` path; writes inside `untrack`; async guarded by sequence counter | holds (by reading, not by running) |
| INV-7 registry complete when enumerated from server mutations | holds (but see W-F01 — the gate itself is leaky) |
| No real organization or personal data | holds |

## The two findings worth acting on first

Both are defects in **gates this phase added**, proven by running the gate against a mutated
copy of the source. A gate that cannot fail is worse than no gate: it reports PASS forever and
every later phase trusts it.

- **W-F01 — the INV-7 gate misses an aliased import.** `GroupsPage` imports
  `groups as groupsApi`, and the gate matches the literal substring `groups.setParent(`, so the
  call in `removeFromParent` is invisible to it. Deleting that function's
  `notifyPlaceContentChanged` call still yields `PASS — 0 нарушений`. The same deletion in
  `GroupContentsTable.detachGroup` (unaliased) is caught, which is what made the hole look closed.
- **W-F02 — `check-group-vocabulary` has undocumented holes.** Markers match by prefix, so
  «Группа похожих устройств» in `ReportTable` and «Группы похожих» in `sidebar-config` both pass;
  and a `//` inside a string literal hides the rest of that line from the scan. One hole in this
  gate was already documented (lowercase «группировать» for `PlaceContents`); these were not.

## Backend warnings

- **W-B01 — un-hiding a required property skips the violators check**
  (`group_type_service.rs:582-590`, `:672-719`). `unarchive_property` does not run
  `groups_missing_required`, and a hidden property can be flipped to required with no check.
  Afterwards any `set_values` on a group lacking that value fails, including edits to unrelated
  fields. This is a genuine gap in success criterion 3 ("свойство с заполненными значениями
  скрывается, а не удаляется" and the required-flag protection).
- **W-B02 — type and property mutations are not atomic with their audit row.** Repo calls run on
  a bare `conn` and autocommit, then the audit row is written in a separate transaction; if the
  audit write fails the change persists but the caller sees an error. `create_property` with
  required-on emulates a rollback by hard-deleting the row it just inserted instead of using one
  transaction.
- **W-B03 — the movements report caps at `LIMIT 1000` ordered oldest-first**
  (`report_service.rs:1546-1563`, `:1692`), and `total` is the capped count. A few large group
  moves can push newest rows out and split a batch — which contradicts D-27 ("на бумаге все
  строки пакета"). Nothing tells the user the result was truncated.

## Frontend warnings

- **W-F03 — wording promises a permission the role lacks.** `GroupTypePanel`'s «Название и набор
  свойств — можно.» shown to a read-only manager (already logged in `deferred-items.md`, now
  confirmed), plus a sibling: `GroupTypePropertiesTable`'s empty state tells a manager to use an
  add row that only renders for admins.
- **W-F04 — the hide-property modal over-promises.** It says the property can be restored, but
  the server permanently deletes an unfilled property.
- **W-F05 — `GroupAddDevicesModal` mirrors server subtree logic in JS.** It filters by place with
  a client-side `full_path` string prefix and silently stops at 1000 rows. This is the
  "JS mirror of a server rule drifts silently" pattern; it exists because no server-side place
  filter is available on the device list.

## Info findings

Backend: unconditional release on the return-edit path (known, deferred to 41.2); required-count
treats a dead reference as filled; no archived-place check in `add_devices` / `set_parent`;
`delete_group` has no version check; `set_values` can return an error after the commit; minor
quality items.

Frontend: the `check-reorder` selftest mutates a copy of `reorder.ts` rather than the real file;
`GroupPanel.handleSaved` can be overwritten by an older in-flight card load; `GroupContentsTable`
can be left on a permanent «Загрузка…» after a reload/expand race; `GroupDeleteModal` hand-rolls
Russian plural inflection and omits nested groups; the history-error text tells the user to close
a window that does not exist; «Свёрнуто: N» in `DeviceList` is the row count, not the number
collapsed; `ReportTable` can show an empty expandable batch and its chevron `aria-label`s carry no
row context.

## Reviewer caveats

Both reviewers were given a diff base (`199438c9^`) that is actually a Phase 13 commit, so a diff
against it spans ~2200 unrelated commits. The file **scope** came from SUMMARY frontmatter and was
correct; only the "show me just this phase's delta" convenience was affected. The frontend reviewer
recomputed the real base (`f9d07be4^`) itself; the backend reviewer limited itself to the group,
membership, batch and movement-journal changes instead. Neither ran the application: runtime and
visual behaviour remains unverified and is covered by the 17 live checks in `41-VALIDATION.md`.
