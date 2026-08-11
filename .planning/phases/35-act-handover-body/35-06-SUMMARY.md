---
phase: 35-act-handover-body
plan: 06
subsystem: printing
tags: [minijinja, rust, html-templates, regression-tests, act-handover]

# Dependency graph
requires:
  - phase: 35-act-handover-body (планы 01-05)
    provides: переработанное тело акта приёма-передачи (D-01..D-12), горизонтальный блок подписей, срез _legacy_defaults/v22/
provides:
  - "act_handover.html: имя устройства печатается в КАЖДОМ .device-block независимо от количества устройств (снят гейт length==1, D-02a)"
  - "Регрессионный тест co-location имени и полей per device-block (N=3, разный набор опциональных полей)"
  - "Регрессионный тест доставки среза v22 в установленные копии (bodies.get(2))"
  - "Точные ассерции меток подписи, не коллидирующие с ФИО-префиксом фикстуры"
  - "Структурный DOC-07-эквивалентный гейт подчёркиваний для act_acceptance.html"
  - "Human-UAT подтверждение per-block атрибуции на обоих транспортах (десктоп + LAN-браузер)"
affects: [35-верификация-повтор, phase-36-pagination]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "device-block co-location regression test: split HTML by literal <div class=\"device-block\"> marker, assert per-index name+field attribution"
    - "legacy-defaults index-N sibling test pattern (bodies.get(N)) для новых срезов KNOWN_LEGACY_DEFAULTS"

key-files:
  created: []
  modified:
    - crates/trackly-app/templates/act_handover.html
    - crates/trackly-app/tests/pdf_render_act.rs
    - crates/trackly-app/src/pdf/html_templates.rs
    - crates/trackly-app/tests/html_act_render.rs
    - crates/trackly-app/tests/html_field_row_underline_gate.rs

key-decisions:
  - "D-02a применена дословно: гейт act.items | length == 1 снят полностью, верхний перечень при N>1 сохранён как сводка (избыточность принята осознанно пользователем в CONTEXT.md)"
  - "Task 4 (человеческая UAT-проверка на обоих транспортах) прошла как approved — пользователь подтвердил per-block атрибуцию на десктопе и в LAN-браузере для акта на 3 устройства с разным набором опциональных полей"

patterns-established: []

requirements-completed: [DOC-07, DOC-08, DOC-09]  # переподтверждены (не впервые закрыты этим планом — validated в 35-01..35-05), но human UAT этого плана — финальное подтверждение перед закрытием gaps_found

# Metrics
duration: ~70min
completed: 2026-08-12
---

# Phase 35 Plan 06: GAP CLOSURE (CR-01/WR-01/WR-02/IN-01) Summary

**Снят гейт `length==1` в act_handover.html — device-block теперь самоидентифицируется именем устройства при любом N, плюс три закрывающих регрессионных теста для находок VERIFICATION.md/REVIEW.md; human-UAT на обоих транспортах подтверждён пользователем (approved).**

## Performance

- **Duration:** ~70 min
- **Started:** 2026-08-11T~17:15Z
- **Completed:** 2026-08-12T~00:20Z
- **Tasks:** 4 of 4 completed
- **Files modified:** 5

## Accomplishments

- **G-01 (CR-01, критично):** для многоустройственных актов приёма-передачи каждый `.device-block`
  снова самоидентифицируется строкой «было получено устройство: ⟨имя⟩», независимо от количества
  устройств. Верхний перечень «были получены устройства:» + `<ul>` при N>1 сохранён как сводка
  (D-02a). Новый регрессионный тест `render_handover_multi_device_fields_attributable_to_own_device`
  (N=3, разный набор опциональных полей — включая устройство совсем без опциональных полей) доказывает
  co-location имени с собственными полями и провалился бы против гейтированного шаблона.
- **G-02 (WR-01):** новый тест `upgrade_replaces_v22_legacy_default_with_current_bundled_body` —
  структурный сиблинг существующего v21-теста (`bodies.get(2)` вместо `bodies.get(1)`), доказывающий,
  что срез v22 реально отличается от текущего бандла и реально драйвит апгрейд установленных копий.
- **G-03 (WR-02):** `html_handover_contains_required_blocks_and_logo` больше не может пройти вхолостую
  на префиксном совпадении ФИО фикстуры — метки «Выдал»/«Получил» заменены на «Выдал:»/«Получил:».
- **G-04 (IN-01):** `act_acceptance.html` получил структурный DOC-07-эквивалентный гейт (ровно один
  легитимный `border-bottom`, принадлежащий `.signature-field .signature-line`).
- Полный `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --test-threads=1
  --skip login_remember_persistent_cookie` зелёный (0 failed) после G-01..G-04 — wave-boundary
  гейт подтверждён.
- Устаревший `target/debug/templates/` (материализован в прошлом раунде UAT ДО этого плана, всё
  ещё содержал снятый гейт `length==1`) удалён — иначе следующий `cargo tauri dev` показал бы
  пользователю старое поведение и человеческая проверка Task 4 ничего бы не доказала.
- **Task 4 (human-UAT, gate=blocking): APPROVED.** Пользователь подтвердил на обоих транспортах
  (десктоп + LAN-браузер) для акта на 3 устройства с разным набором опциональных полей: сводная
  строка «были получены устройства:» присутствует, каждый `.device-block` начинается со своей
  строки «было получено устройство: ⟨имя⟩», блок устройства без опциональных полей всё равно
  самоидентифицируется, редактор шаблонов (Settings → Шаблоны → Предпросмотр) не падает.
  Обезличено согласно приватность-константе CLAUDE.md — реальные ФИО/реквизиты в этом отчёте
  не фигурируют.

## Task Commits

Each task was committed atomically:

1. **Task 1: G-01 — снять гейт length==1, добавить регрессионный тест атрибуции** — `d274e6b` (fix)
2. **Task 2: G-02 — регрессионный гейт для среза v22 (WR-01)** — `f0b89d4` (test)
3. **Task 3: G-03 + G-04 — точные ассерции меток подписи и структурный гейт для act_acceptance.html** — `5ab29c1` (test)
4. **Task 4: Ручная UAT-проверка per-block атрибуции на обоих транспортах** — checkpoint:human-verify,
   gate=blocking; approved by user; no code commit (verification-only task)

**Plan metadata:** `586cea6` (docs: draft summary), `758daab` (docs: STATE.md session update,
pre-checkpoint) — superseded by this final commit closing Task 4.

## Files Created/Modified

- `crates/trackly-app/templates/act_handover.html` — снят `{%- if act.items | length == 1 %}` гейт
  вокруг строки имени устройства внутри `.device-block` (2 строки удалены, markup-only правка).
- `crates/trackly-app/tests/pdf_render_act.rs` — новый тест
  `render_handover_multi_device_fields_attributable_to_own_device` (N=3, co-location + empty-block
  case).
- `crates/trackly-app/src/pdf/html_templates.rs` — новый тест
  `upgrade_replaces_v22_legacy_default_with_current_bundled_body` (v22-индексный сиблинг v21-теста).
- `crates/trackly-app/tests/html_act_render.rs` — `"Выдал"`/`"Получил"` → `"Выдал:"`/`"Получил:"` в
  ассерции меток блока подписей.
- `crates/trackly-app/tests/html_field_row_underline_gate.rs` — новая константа
  `ACT_ACCEPTANCE_HTML` + новый тест
  `acceptance_signature_line_css_has_exactly_one_legitimate_border_bottom`.

## Decisions Made

- D-02a применена буквально: гейт снят полностью, верхний перечень при N>1 не тронут — оба блока
  (сводка + per-block имя) сосуществуют осознанно, избыточность не устраняется (решение пользователя,
  зафиксировано в 35-CONTEXT.md).
- Удалён устаревший `target/debug/templates/` (не входит в `files_modified` плана, но это
  build-артефакт вне git, не отслеживаемый репозиторием) как часть автоматизируемой подготовки к
  Task 4 — без этого шага UAT проверял бы старое, уже исправленное поведение.
- Task 4 разрешён как approved независимой перепроверкой (не повторным прогоном `cargo test` в
  этой же сессии — избегая гонки за `target/`-lock): `--lib pdf::` 61/0, `--test pdf_render_act`
  13/0, `--test html_act_render` 11/0, `--test html_field_row_underline_gate` 2/0, `git diff` по
  `crates/trackly-app/src/services/` пуст (бэкенд не тронут), `length == 1` → 0 вхождений в шаблоне,
  верхняя сводка на месте, `border-bottom` ровно 2 в act_handover.html.

## Deviations from Plan

None - plan executed exactly as written for all 4 tasks (Task 4 concluded with the plan-mandated
"approved" outcome, no discrepancies reported). `target/debug/templates/` deletion is build-artifact
housekeeping (outside git, outside `files_modified`), required by the plan's own Task 4 instructions
("если ещё не удалён с прошлого раунда UAT этой фазы — удалить") — not a deviation from the plan,
but the exact action the plan's Task 4 prescribes.

## Issues Encountered

- The initial full-suite wave-boundary test run (`cargo test -p trackly-app --
  --test-threads=1 --skip login_remember_persistent_cookie`) was killed mid-run by an
  unrelated background-process-management issue during this session (multiple redundant
  polling loops accumulated). Cleaned up stray processes and re-ran the full suite once
  cleanly to completion — final result: 0 failed. No impact on code correctness; purely an
  execution-environment hiccup, not a deviation from the plan.

## Known Follow-ups (out of scope for this plan — NOT fixed here)

**Missing `_legacy_defaults/v23/` snapshot for Task 1's `act_handover.html` change.** Task 1
changed the bundled `act_handover.html` body (removed the `length == 1` gate), but per this
module's own doc-comment discipline (`crates/trackly-app/src/pdf/html_templates.rs:52-63`,
"the extension point" instruction established in Phase 34/35), every bundle body change is
supposed to snapshot the PRE-change body into a new `_legacy_defaults/vNN/` slice element and
register it in `KNOWN_LEGACY_DEFAULTS` — otherwise installs currently on the pre-change body will
never be recognized as "provably untouched" and will not receive the auto-upgrade. This plan did
not do that for Task 1's change (no `v23/act_handover.html` snapshot, no new `KNOWN_LEGACY_DEFAULTS`
element). **No practical impact right now**: the intermediate (gated) body was never shipped in a
tagged release (last tag is `v1.3`, predates all of Phase 35's plans), and no materialized copy on
this development machine currently holds that intermediate body (the stale
`target/debug/templates/` copy that did was deleted during this plan's Task 4 preparation — see
Accomplishments). This is recorded here as a known follow-up so the gap is not lost; it should be
picked up as its own small task (either in a future Phase 35 gap-closure or as part of Phase 36) —
not fixed in this plan, per explicit coordinator instruction.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 35 gap-closure (Plan 06) is fully complete: G-01/CR-01, G-02/WR-01, G-03/WR-02, G-04/IN-01
all closed with regression tests, full test suite green, and Task 4 human-UAT approved on both
transports. Ready for `/gsd-verify-work` re-run on Phase 35 (orchestrator's responsibility, not
run by this agent) to confirm the prior `gaps_found` status now resolves clean. The
`_legacy_defaults/v23/` follow-up above should be tracked before any further `act_handover.html`
body changes ship.

---
*Phase: 35-act-handover-body*
*Completed: 2026-08-12*

## Self-Check: PASSED

Verified before finalizing this summary:
- `[ -f .planning/phases/35-act-handover-body/35-06-SUMMARY.md ]` → FOUND
- `git log --oneline --all | grep d274e6b` → FOUND (Task 1 commit)
- `git log --oneline --all | grep f0b89d4` → FOUND (Task 2 commit)
- `git log --oneline --all | grep 5ab29c1` → FOUND (Task 3 commit)
- `git log --oneline --all | grep 586cea6` → FOUND (draft summary commit)
- `git log --oneline --all | grep 758daab` → FOUND (STATE.md pre-checkpoint commit)
- `grep -c "act.items | length == 1" crates/trackly-app/templates/act_handover.html` → 0
- `grep -c border-bottom crates/trackly-app/templates/act_handover.html` → 2
- `git diff --stat <plan-start>..HEAD -- crates/trackly-app/src/services/` → empty (backend untouched)
- `./scripts/check-privacy-requisites.sh` → green before every commit in this plan
- Independent re-verification of test results by coordinator (not re-run in this session to avoid
  `target/`-lock contention): `--lib pdf::` 61/0 failed, `--test pdf_render_act` 13/0 failed,
  `--test html_act_render` 11/0 failed, `--test html_field_row_underline_gate` 2/0 failed
