---
phase: 260904-x7s
plan: 01
subsystem: acts
tags: [rusqlite, sqlite, act-numbering, soft-delete]

# Dependency graph
requires:
  - phase: 03-pdf
    provides: "D-Soft-vs-Hard-Acts-01 — soft-delete акта + hard-delete act_items"
  - phase: 19-acts-date-edit
    provides: "update-путь, использующий ту же проверку уникальности номера, что и create"
provides:
  - "next_act_number_from_max(conn) — MAX(number)+1 среди живых актов, единый источник авто-номера/подсказки «Следующий»"
  - "Уникальность number_override в create/update фильтруется по deleted_at_utc IS NULL — номер удалённого акта свободен"
affects: [acts, act-numbering, act-service]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Единая функция источника истины MAX(live)+1 принимает и &Connection (readers), и &Transaction (write-path) через Deref coercion — без _in_tx twin-функции"

key-files:
  created: []
  modified:
    - crates/trackly-infra/src/repos/acts_sqlite.rs
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-core/src/domain/acts.rs
    - crates/trackly-app/src/dto/act.rs
    - crates/trackly-app/tests/acts_crud.rs
    - crates/trackly-app/tests/acts_update.rs

key-decisions:
  - "counters.act_number больше не читается/не пишется на пути нумерации актов, но строка и функции increment_counter_in_tx/peek_counter/peek_counter_in_tx НЕ удаляются — cartridge_seq и другие счётчики их всё ещё используют"
  - "next_auto_would_be для audit override вычисляется ДО INSERT (в шаге 1 create), не пересчитывается заново после — иначе в MAX попал бы только что вставленный акт"

patterns-established: []

requirements-completed: [X7S-01, X7S-02]

# Metrics
duration: ~25min
completed: 2026-09-05
---

# Quick Task 260904-x7s: Удаление акта освобождает номер Summary

**Удаление акта №N (soft-delete) немедленно освобождает номер N — и для ручного `number_override` (create/update), и для авто-подсказки «Следующий» (`peek_next_number`), заменив монотонный `counters.act_number` на `MAX(number)+1` среди живых актов.**

## Performance

- **Duration:** ~25 min
- **Tasks:** 2 (2 из 2 выполнены)
- **Files modified:** 6

## Accomplishments
- `next_act_number_from_max(conn: &Connection)` в `acts_sqlite.rs` — единственная функция источника истины авто-номера, принимает и reader-`&Connection`, и write-`&Transaction` (Deref coercion, без `_in_tx`-твина)
- `create`/`update`: EXISTS-проверка уникальности `number_override` получила `AND deleted_at_utc IS NULL` — номер soft-deleted акта больше не блокирует переиспользование
- `ActRepository::peek_next_number` и `ActService::peek_next_number` переведены на `next_act_number_from_max` — кнопка «Следующий» после удаления последнего акта снова предлагает освободившийся номер
- Doc-комментарии модуля/типов (`ActNew`, `ActCreateDto::number_override`) обновлены — убрана ссылка на инкремент `counters.act_number` как источник авто-номера актов

## Task Commits

1. **Task 1: MAX(live)+1 источник номера — уникальность фильтруется по deleted_at_utc IS NULL** - `d9dd1e12` (feat)
2. **Task 2: Интеграционные тесты — переиспользование номера через create/update + peek_next_number** - `e1d67fd3` (test)

_SUMMARY.md/STATE.md docs commit handled by orchestrator, not by this executor per constraints._

## Files Created/Modified
- `crates/trackly-infra/src/repos/acts_sqlite.rs` - добавлена `next_act_number_from_max`, `peek_next_number` trait-impl переведён на неё, 2 новых unit-теста
- `crates/trackly-app/src/services/act_service.rs` - `create`/`update` EXISTS-проверки фильтруются по `deleted_at_utc IS NULL`, авто-номер и `next_auto_would_be` берутся из `next_act_number_from_max`, `peek_next_number` переведён на неё, обновлён импорт и doc-комментарий модуля
- `crates/trackly-core/src/domain/acts.rs` - обновлён doc-комментарий `ActNew` про источник авто-номера и правило уникальности
- `crates/trackly-app/src/dto/act.rs` - обновлён doc-комментарий `ActCreateDto::number_override`
- `crates/trackly-app/tests/acts_crud.rs` - `override_number_reuses_deleted_act_number`, `peek_next_number_frees_on_delete_of_last_act`
- `crates/trackly-app/tests/acts_update.rs` - `update_number_reuses_deleted_act_number`

## Decisions Made
- `counters` таблица и её функции (`increment_counter_in_tx`, `peek_counter`, `peek_counter_in_tx`) остаются в коде — используются другими счётчиками (например `cartridge_seq`); менять схему/миграции не потребовалось (частичный уникальный индекс `idx_acts_number_sub_unique` уже фильтровал по `deleted_at_utc IS NULL`)
- `next_auto_would_be` в audit-payload override вычисляется в шаге 1 `create` (до INSERT) и переиспользуется в шаге 3, а не пересчитывается заново — иначе после вставки акта MAX включал бы уже сам вставленный акт и подсказка исказилась бы

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Функциональность закрыта и покрыта тестами; изменений в БД/миграциях не требовалось. Фронтенд (`ActNumberField.svelte`) не трогался по решению CONTEXT.md — своей копии формулы номера у него нет, поведение подхватится автоматически через `acts.peekNextNumber()`.

---
*Quick task: 260904-x7s*
*Completed: 2026-09-05*

## Self-Check: PASSED

All 7 files created/modified verified present on disk; both task commits
(`d9dd1e12`, `e1d67fd3`) verified present in `git log --all`.
