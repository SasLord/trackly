---
status: diagnosed
trigger: "Клик по ссылке документа в таймлайне перемещений открывает не тот подраздел раздела «Акты» и не выделяет строку."
created: 2026-09-03T00:00:00Z
updated: 2026-09-03T00:00:00Z
---

## Current Focus

hypothesis: CONFIRMED (two independent causes)
  (A) ActsPage.svelte hardcodes activeTab='handover' at mount and never derives the
      subsection from the linked act's act_type/archived; the row is highlighted only
      if it is present in the currently filtered list.
  (B) place_movement_service.rs resolves act_number with a raw `SELECT number FROM acts`
      instead of the canonical `format_act_number`, so a RETURN act renders with its
      parent's plain number («20») instead of «20в»/«20в2».
test: code read of both write and read sides + schema check against dev DB (structure only)
expecting: n/a — root cause confirmed, diagnose-only mode
next_action: hand off to fix planning (goal was find_root_cause_only, no fix applied)

## Symptoms

expected: |
  Клик по ссылке документа в строке таймлайна открывает раздел «Акты» в нужном подразделе
  и выделяет нужную строку: акт передачи → «Акты»; возврат → «Возвраты»; архив → «Архив».
actual: |
  Ссылка у записи о возврате ведёт на исходный акт передачи, а не на документ возврата.
  Когда акт ушёл в архив, переход открывает подраздел «Акты», а не «Архив»; детальная
  карточка открывается, но строка в списке не выделяется.
errors: None reported
reproduction: Test 7 в .planning/phases/40-movement-history/40-UAT.md
started: Discovered during UAT фазы 40

## Eliminated

## Evidence

- timestamp: phase-0
  checked: .planning/debug/knowledge-base.md
  found: no keyword overlap with existing entries (PDF/CSP/logo/chart topics)
  implication: no known-pattern shortcut; investigate from scratch

- timestamp: e1
  checked: ui/src/features/acts/ActsPage.svelte (full script)
  found: |
    `let activeTab = $state<TabKey>('handover')` — the tab is ALWAYS 'handover' at mount.
    `initialFocusId = parseIdFromHash()` only seeds `selectedActId`. baseFilter:
    handover → act_type='handover', archived=false; returns → act_type='return', archived=null;
    archive → act_type='handover', archived=true. Nothing anywhere derives the tab from the
    linked act. `acts.get(id)` (detail $effect) is filter-independent, so the detail card
    loads regardless of tab.
  implication: |
    Deep link `#/acts?id=N` always lands on «Акты». An archived handover (archived=true) and
    every return act are excluded by that tab's filter → the row is absent from `items`.

- timestamp: e2
  checked: ui/src/features/acts/ActsList.svelte:134
  found: "`selected={act.id === selectedActId}` — highlight is per-rendered-row only."
  implication: |
    Highlighting is impossible when the act is not in the active tab's result set. Explains
    «детальная информация открывается, но в списке не выделяется» exactly.

- timestamp: e3
  checked: crates/trackly-app/src/services/act_service.rs (do_return ~1530, update_return ~2022/2109/2195)
  found: "every return-driven record_movement_if_applicable passes Some(return_act_id) / Some(payload.id)."
  implication: |
    The stored act_id ALREADY points at the return document, not the parent handover.
    The link TARGET is correct; the defect is in the displayed label and in the landing tab.

- timestamp: e4
  checked: crates/trackly-app/src/services/place_movement_service.rs:93-101
  found: |
    act_number = `SELECT number FROM acts WHERE id = ?1` read as i64 then `.to_string()`.
    act_type / sub_number / parent number / sibling count are never read.
  implication: |
    Hand-rolled mirror of act-number display that bypasses the single owner
    `format_act_number` (crates/trackly-app/src/dto/act.rs:23), used by act_dto_from_row
    everywhere else in the app.

- timestamp: e5
  checked: act_service.rs:1318-1321 + dev DB `SELECT id, number, sub_number, act_type FROM acts WHERE act_type='return'`
  found: |
    Return acts are inserted with `number: parent.number` and a distinguishing `sub_number`.
    Dev DB confirms rows with identical `number` shared by a handover and 1..4 returns
    (counts only, no field contents read).
  implication: |
    `SELECT number` for a return act returns the PARENT's number → the timeline literally
    prints «актом №20» for the return, identical to the handover row. This is exactly what the
    user reported as «появилась снова ссылка на акт, а не на Возврат». Canonical display would
    be «20в» (single return) or «20в2» (multiple).

- timestamp: e6
  checked: crates/trackly-app/tests/place_movements_timeline.rs:349-390 and place_movements_act_link.rs
  found: |
    The only act_number assertion (`place_movements_act_number_resolves`) seeds a HANDOVER act
    ('handover' hardcoded in seed_act) and asserts "777". act_link tests assert act_id linkage
    only, never the rendered number for a return act.
  implication: "Return-act label formatting has zero test coverage — the regression was invisible to the gates."
## Resolution

root_cause: |
  Two independent defects, both on the READ/навигация side (the recorded data is correct).

  (1) Subsection is never derived. `ActsPage.svelte` initialises `activeTab` to 'handover'
      unconditionally; the `#/acts?id=N` deep link (Plan 40-15) only seeds `selectedActId`.
      Because each tab applies a server-side filter (handover: act_type=handover+archived=false;
      returns: act_type=return; archive: act_type=handover+archived=true), a return act and an
      auto-archived handover are never in the handover tab's list, so `ActsList` cannot mark the
      row selected — while `acts.get(id)` still populates the detail card (filter-independent).

  (2) The act-number LABEL for return documents is wrong. `place_movement_service.rs` builds
      `act_number` with a raw `SELECT number FROM acts WHERE id = ?`, bypassing the single owner
      `format_act_number` (dto/act.rs). Return acts store `number = parent.number` + `sub_number`,
      so the timeline shows the parent's plain number for a return — indistinguishable from the
      handover row, which is why the user read it as «ссылка на акт, а не на Возврат».

  The link target itself is NOT broken: do_return / update_return record the movement with the
  RETURN act's own id.
fix: (not applied — goal: find_root_cause_only)
verification: (n/a — diagnose-only)
files_changed: []
