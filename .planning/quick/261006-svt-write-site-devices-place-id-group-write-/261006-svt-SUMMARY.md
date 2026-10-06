---
phase: quick-261006-svt
plan: 01
subsystem: tests / group write-site gates
tags: [security-residual, inventory-gate, mutation-selftest]
requirements: [SEC-41-RESIDUAL-1]
key-files:
  modified:
    - crates/trackly-app/tests/group_write_sites.rs
metrics:
  tasks: 3
  files: 1
  tests: "group_write_sites: 30 passed, 0 failed"
---

# Quick 261006-svt: полный инвентарь-гейт write-site'ов devices.place_id

Гейт write-site'ов `devices.place_id` расширен с трёх методов до полного набора из 8 методов `SqliteDeviceRepository`, у каждого закодирован вердикт; мутационные самопроверки доказывают, что гейт краснеет. Продакшн-код не менялся (изменён только тест-файл).

## Что сделано

- **D-01/D-03 needle'ы по вердикту.** `WRITE_SITE_CALLS` (4: прежние три + `update_in_tx(`) и `BIRTH_EXEMPT_CALLS` (`repo.create_in_tx(`, `clone_device_in_tx(`) с обоснованием у константы; `BIRTH_SCAN_SKIP_PREFIX` исключает `crates/trackly-infra/src/repos/`. `count_calls` получил границу идентификатора слева (`printer_repo.create_in_tx(` не считается).
- **Реестры с точными числами.** `WRITE_SITE_REGISTRY` (+ `device_service.rs: 1`), `BIRTH_EXEMPT_REGISTRY` (`device_service.rs: 2`, `act_service.rs: 1`). Проверка вынесена в чистую `registry_violations`.
- **D-02 парность `update_in_tx`.** `uncovered_guarded_sites`: release в 14 строках после ИЛИ guard S1 (`locked_group_for_device_in_tx(` + `AppError::Validation`) в 20 строках до. На реальном `device_service.rs` сайт один, покрыт guard'ом.
- **D-04 гейт «от репозитория».** `PLACE_WRITER_METHODS` (8 имён с вердиктами ReleaseRequired / BirthExempt / NoProductionCallers), `place_writing_methods`, `place_writer_diff`; тест связи вердиктов и needle'ов.
- **D-05/D-06.** Комментарий «ровно тремя методами» заменён фактической картиной; каскад картриджа (`cartridges_sqlite.rs:651`) описан как известный третий класс, покрытый S9, вне инвентарь-гейтов.
- **Мутационные самопроверки** (`write_site_gate_mutation_*`): снятие guard S1 с реальной копии `device_service.rs`, новый/пропавший метод в копии `devices_sqlite.rs`, ловушка имён birth-гейта (мутации A-D). Якоря реальных мутаций утверждены `matches(anchor).count() == 1`; немутированные линии проходят.

## Deviations from Plan

**1. [Rule 1 - дефект плана] Эвристика `place_writing_methods` была вакуумной для мутации «пропал write».**
- **Найдено в:** Task 3, первый прогон: мутант `status_id = ?1, place_id = ?2,` -> `status_id = ?1,` не краснел (`исчезли` пусто).
- **Причина:** план требовал «тело содержит `UPDATE devices` И `place_id`», но `place_id` встречается в сигнатуре и в `params![...]` метода, поэтому удаление колонки из SQL ничего не меняло. Это случай «зелено при снятом гейте» (урок `vacuous_test_fixture_must_diverge`).
- **Исправление:** `writes_place_id_in_sql` смотрит только на SQL-литерал (от `INSERT INTO devices`/`UPDATE devices` до закрывающей кавычки). Реестр из 8 имён НЕ менялся; на реальном исходнике по-прежнему ровно 8 имён, мутант теперь краснит (`исчезли == [update_status_and_place_in_tx]`).
- Докстрока `place_writing_methods` обновлена.

**2. [Мелкое] Сигнатура `registry_violations`** получила пятый параметр `new_site_hint` (текст подсказки отличается у write-site- и birth-гейта). Условия проверки не менялись.

**3. [Процесс] Один коммит вместо трёх.** Все три задачи правят один файл и были написаны в одном проходе, поэтому закоммичены вместе: `ef87ff3c`. Границы задач в истории не разделены.

Числа на реальных исходниках сошлись с планом: 8 имён в реестре, birth-needle'ы 2 (`device_service.rs`) и 1 (`act_service.rs`), `update_in_tx` — 1 сайт.

## Verification

- `cargo test -p trackly-app --test group_write_sites` (с `DEVELOPER_DIR=/Library/Developer/CommandLineTools`): 30 passed, 0 failed.
- `git diff --stat`: только `crates/trackly-app/tests/group_write_sites.rs`. Подстроки «ровно тремя методами» в файле нет.
- Файл отформатирован `rustfmt --edition 2021` (только он). Хук приватности при коммите: PASS, 0 нарушений; фикстуры содержат только технические идентификаторы.

## Known Stubs

Нет.

## Self-Check: PASSED

- Файл `crates/trackly-app/tests/group_write_sites.rs` изменён; коммит `ef87ff3c` существует.
