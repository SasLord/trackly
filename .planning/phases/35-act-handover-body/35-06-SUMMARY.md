---
phase: 35-act-handover-body
plan: 06
subsystem: printing
tags: [minijinja, rust, html-templates, regression-tests, act-handover]

# Dependency graph
requires:
  - phase: 35-act-handover-body (planы 01-05)
    provides: переработанное тело акта приёма-передачи (D-01..D-12), горизонтальный блок подписей, срез _legacy_defaults/v22/
provides:
  - "act_handover.html: имя устройства печатается в КАЖДОМ .device-block независимо от количества устройств (снят гейт length==1, D-02a)"
  - "Регрессионный тест co-location имени и полей per device-block (N=3, разный набор опциональных полей)"
  - "Регрессионный тест доставки среза v22 в установленные копии (bodies.get(2))"
  - "Точные ассерции меток подписи, не коллидирующие с ФИО-префиксом фикстуры"
  - "Структурный DOC-07-эквивалентный гейт подчёркиваний для act_acceptance.html"
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
  - "Task 4 (человеческая UAT-проверка на обоих транспортах) — блокирующий чекпоинт, НЕ авто-одобрен; удалён устаревший target/debug/templates/ (материализован до Task 1, содержал старый гейт) для корректного повторного материализования при следующем cargo tauri dev"

patterns-established: []

requirements-completed: []  # DOC-07/DOC-08/DOC-09 переподтверждены (не впервые закрыты этим планом) — human UAT (Task 4) ещё не подтверждён, поэтому не отмечаются как newly satisfied здесь

# Metrics
duration: ~55min
completed: 2026-08-11
---

# Phase 35 Plan 06: GAP CLOSURE (CR-01/WR-01/WR-02/IN-01) Summary

**Снят гейт `length==1` в act_handover.html — device-block теперь самоидентифицируется именем устройства при любом N, плюс три закрывающих регрессионных теста для находок VERIFICATION.md/REVIEW.md; Task 4 (блокирующий human-UAT) ожидает подтверждения пользователя.**

## Performance

- **Duration:** ~55 min (Tasks 1-3; Task 4 remains open as a blocking checkpoint)
- **Started:** 2026-08-11T~17:15Z
- **Completed (Tasks 1-3):** 2026-08-11T~18:10Z
- **Tasks:** 3 of 4 completed (Task 4 is a blocking human-verify checkpoint, not yet resolved)
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

## Task Commits

Each task was committed atomically:

1. **Task 1: G-01 — снять гейт length==1, добавить регрессионный тест атрибуции** — `d274e6b` (fix)
2. **Task 2: G-02 — регрессионный гейт для среза v22 (WR-01)** — `f0b89d4` (test)
3. **Task 3: G-03 + G-04 — точные ассерции меток подписи и структурный гейт для act_acceptance.html** — `5ab29c1` (test)

**Task 4** (checkpoint:human-verify, gate=blocking) — NOT executed by this agent; requires human
confirmation on both transports (desktop + LAN-browser). See "Next Phase Readiness" below.

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

## Deviations from Plan

None - plan executed exactly as written for Tasks 1-3. `target/debug/templates/` deletion is
build-artifact housekeeping (outside git, outside `files_modified`), required by the plan's own
Task 4 instructions ("если ещё не удалён с прошлого раунда UAT этой фазы — удалить") — not a
deviation from the plan, but the exact action the plan's Task 4 prescribes, performed proactively
during checkpoint preparation.

## Issues Encountered

- The initial full-suite wave-boundary test run (`cargo test -p trackly-app --
  --test-threads=1 --skip login_remember_persistent_cookie`) was killed mid-run by an
  unrelated background-process-management issue during this session (multiple redundant
  polling loops accumulated). Cleaned up stray processes and re-ran the full suite once
  cleanly to completion — final result: 0 failed. No impact on code correctness; purely an
  execution-environment hiccup, not a deviation from the plan.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

**BLOCKED on Task 4** (human-verify checkpoint, gate=blocking) — see the `## CHECKPOINT REACHED`
block returned alongside this summary. Automated preparation completed:
`./scripts/check-privacy-requisites.sh` green, `target/debug/templates/` cleared,
`ui/dist` confirmed to be a real recent pnpm build (no frontend files touched by this plan, so
no rebuild needed). Once the user confirms per-block device-name attribution on both transports
for a 3-device act with differing optional-field counts, a continuation agent should:
1. Record the "approved" outcome.
2. Finalize plan-completion STATE.md/ROADMAP.md updates (this SUMMARY intentionally does NOT
   run `state advance-plan` / `roadmap update-plan-progress` yet — the plan is not complete
   until Task 4 resolves).
3. Re-run `/gsd-verify-work` for Phase 35 to confirm gaps_found → clean.

---
*Phase: 35-act-handover-body*
*Completed (Tasks 1-3): 2026-08-11 — Task 4 pending human checkpoint*
