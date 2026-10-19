---
phase: quick-261019-q3o
plan: 01
subsystem: tests/entities_changed_gate
tags: [gate, registry, regression]
key-files:
  modified:
    - crates/trackly-app/tests/entities_support/registry.rs
metrics:
  completed: 2026-10-19
---

# Quick 261019-q3o: вердикт ReadOnly для export_csv_with_limit

Одна строка `ro(DEVICE, "export_csv_with_limit")` в `REGISTRY` закрыла регрессию фазы 41.11: функция, добавленная планом 41.11-08, не имела вердикта, и три теста гейта `entities_changed_gate` были красными (сканер находил 120 функций, в реестре было 119 строк).

## Что сделано

- В `crates/trackly-app/tests/entities_support/registry.rs` после `ro(DEVICE, "export_csv")` добавлена строка с вердиктом ReadOnly. Функция только читает выборку и собирает CSV, записи в БД и `broadcast_entities` в ней нет.
- Гейт не ослаблялся: пороги, логика, `SCOPE_FILES`, освобождения и остальные строки реестра не тронуты.
- Коммит: `196c1071`.

## Проверка

- До правки: 46 passed, 3 failed.
- После: `49 passed; 0 failed`.
- Мутация: якорь уникален (`grep -c` вернул 1). Без строки `registry_matches_sources_exactly` красный (left: 120, right: 119), вместе с ним ещё два теста (46 passed, 3 failed). После возврата строки снова 49 passed.
- `git diff`: ровно одна добавленная строка. Гейт приватности: 0 нарушений.

## Отклонения от плана

Нет.

## Self-Check: PASSED
