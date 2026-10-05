---
phase: 41-groups-model-and-editor
plan: 32
subsystem: groups / типы и свойства
tags: [atomicity, audit, sqlite, transaction, gap-closure, W-B02]
requires: [41-29, 41-31]
provides:
  - "8 мутаций GroupTypeService выполняются в одной транзакции writer вместе со строкой audit_log"
  - "inherent-мутаторы репозитория на &Connection (`*_on`), вызываемые и на соединении, и внутри транзакции"
affects:
  - crates/trackly-app/src/services/group_type_service.rs
  - crates/trackly-infra/src/repos/group_types_sqlite.rs
  - crates/trackly-app/tests/groups_types_service.rs
tech-stack:
  patterns: ["let tx = conn.transaction() -> repo.*_on(&tx) -> audit_repo.insert(&tx) -> tx.commit()"]
key-files:
  modified:
    - crates/trackly-app/src/services/group_type_service.rs
    - crates/trackly-infra/src/repos/group_types_sqlite.rs
    - crates/trackly-app/tests/groups_types_service.rs
decisions:
  - "Мутаторы репозитория вынесены в inherent-методы `*_on(&Connection)`; трейт `GroupTypeRepository` (порт в trackly-core) не менялся"
  - "reorder_properties_on не открывает собственную транзакцию (ни BEGIN, ни savepoint); трейтовый reorder_properties открывает её сам"
metrics:
  tasks: 2
  files: 3
  completed: 2026-10-05
---

# Фаза 41 План 32: атомарность мутаций типов и свойств с аудитом (W-B02)

Каждая из 8 мутаций `GroupTypeService` (create_type, update_type, delete_type, create_property, update_property, delete_property, unarchive_property, reorder_properties) теперь делает изменение и вставку в `audit_log` в одной транзакции writer: сбой аудита откатывает саму мутацию.

## Выполнено

| Задача | Коммит | Суть |
| ------ | ------ | ---- |
| 1 (RED) | 097886e1 | 10 тестов `atomic_*` с триггером `BEFORE INSERT ON audit_log` для `group_type` / `group_type_property` |
| 2 (GREEN) | 5734bdbd | одна транзакция на мутацию; `*_on(&Connection)` в репозитории |

## Красный прогон (до правки кода)

Команда: `cargo test -p trackly-app --test groups_types_service atomic_`. Итог: `9 failed; 1 passed`, без `error[E`.

- `atomic_create_property_required_violation_leaves_nothing` — ok (охранный, зелёный и до, и после; работала ручная компенсация).
- Девять тестов с триггером упали на шаге 5, то есть на утверждении неизменности состояния. Шаг 4 (`assert_audit_fault`: ошибка содержит «audit down») они прошли, значит, триггер сработал и именно он вернул ошибку:
  - atomic_create_type: «тип не должен появиться» (left 4, right 3)
  - atomic_create_property: «свойство не выросло»
  - atomic_update_type: «имя и version прежние»
  - atomic_update_property: «имя, is_required, version прежние»
  - atomic_delete_type: «тип остался» (left 0)
  - atomic_delete_property_hard: «свойство осталось» (left 0)
  - atomic_delete_property_archive: «свойство осталось живым»
  - atomic_unarchive_property: «свойство осталось скрытым»
  - atomic_reorder_properties: «порядок прежний» (left 2, right 0)

Каждый тест после снятия триггера повторяет ту же мутацию и ждёт `Ok` и изменённое состояние, поэтому триггер является единственной причиной отказа.

## Зелёный прогон (после правки)

- `groups_types_service`: 39 passed (10 `atomic_*`, все `protect_*` и `protect_d_*`, `rights_*`, `properties_*`, `seed_*`).
- `trackly-infra --test group_types_repo`: 14 passed.
- `groups_values_card`: 19 passed. `role_endpoint_matrix`: 12 passed. `groups_property_removal_parity` (41-31): 1 passed.
- `cargo clippy -p trackly-app -p trackly-infra --all-targets -- -D warnings`: код 0.
- `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256`: PASS, 0 нарушений.

## Мутационные проверки

- **m1.** В `update_type` убран `tx.commit()` (маркер `// W-B02:update_type:commit`, `grep -c` = 1). Красными стали `atomic_update_type` и обычный `seed_rename_survives_restart_and_reseed` (37 passed, 2 failed). Значит, commit действительно последнее действие, и правка без него не закрепляется.
- **m2.** План предлагал вернуть `conn.transaction()` вместо `savepoint()`. Savepoint в итоговой реализации не используется (см. отклонение), поэтому аналог: в `reorder_properties_on` добавлен `conn.execute_batch("BEGIN")`. Красными стали `atomic_reorder_properties` и `properties_reorder_with_foreign_ids_rejected` с ошибкой `cannot start a transaction within a transaction`. Это подтверждает, что тело перестановки внутри внешней транзакции не должно открывать свою.
- Мутации откатывались восстановлением из копии файла в scratchpad, а не `git checkout`, так как правки ещё не были закоммичены. После отката маркер m2 отсутствует, маркер m1 на месте (`grep -c` = 1).

## Таблица «метод → транзакция → первый мутатор» (`group_type_service.rs`)

| Метод | Строка `conn.transaction()` | Строка первого мутатора репозитория |
| ----- | --------------------------- | ----------------------------------- |
| create_type | 264 | 271 (`create_type_on`) |
| update_type | 356 | 368 (`update_type_on`) |
| delete_type | 407 | 428 (`delete_type_on`) |
| create_property | 470 | 485 (`create_property_on`) |
| update_property | 574 | 614 (`update_property_on`) |
| delete_property | 657 | 662 (`archive_property_on`) |
| unarchive_property | 700 | 717 (`unarchive_property_on`) |
| reorder_properties | 762 | 764 (`reorder_properties_on`) |

Девятая `conn.transaction()` (стр. 866) — `seed_builtin_types_on_startup`, не менялась. `grep -c "repo.delete_property_hard"` = 1 (только в `delete_property`; компенсация в `create_property` удалена). Фраза «в короткой транзакции после изменения» из шапки модуля убрана.

## Отклонения от плана

**1. [Rule 3 - Blocking] Подстановка `&mut tx` вместо `&mut Connection` не компилируется**
- **Найдено в:** задача 2.
- **Проблема:** факт плана «`rusqlite::Transaction` реализует `DerefMut<Target = Connection>`» неверен. В rusqlite 0.38 у `Transaction` и `Savepoint` только `Deref`, поэтому получить `&mut Connection` из транзакции нельзя (10 ошибок `E0596`). Мутаторы трейта `GroupTypeRepository` принимают `&mut Connection`, а `conn.savepoint()` тоже требует `&mut`.
- **Решение:** тела 9 мутаторов вынесены в inherent-методы `SqliteGroupTypeRepository::*_on(&self, conn: &Connection, …)` по образцу `*_in_tx` в `groups_sqlite`. Трейтовые методы остались с прежними сигнатурами и делегируют в них, трейт в trackly-core не менялся. Сервис вызывает `repo.*_on(&tx, …)`: `&Transaction` приводится к `&Connection`. Для `reorder_properties_on` собственная транзакция убрана, а трейтовая обёртка открывает её сама.
- **Отличие от критериев плана:** `grep -c "conn.savepoint()"` в репозитории равен 0, а не 1. Задуманное (вложенность во внешнюю транзакцию) достигнуто тем, что тело перестановки не открывает ни BEGIN, ни savepoint. Новых методов в репозитории появилось 9, хотя план их «не вводить» предписывал (это следствие того же факта).
- **Файлы:** `crates/trackly-infra/src/repos/group_types_sqlite.rs`, `crates/trackly-app/src/services/group_type_service.rs`.
- **Коммит:** 5734bdbd.

**2. Побочный эффект `cargo fmt` откатан**
`cargo fmt -p trackly-app` переформатировал четыре чужих файла с прежним дрейфом форматирования (`dto/act.rs`, `tests/acts_*.rs`). Их правки откатаны `git checkout -- <файл>` до коммита и в коммиты не попали.

## Известные заглушки

Нет.

## Флаги угроз

Нет новой поверхности: авторизация (`authorize(ManageGroupTypes)` первой строкой) не менялась, транспортных изменений нет.

## Проверки в запущенном приложении

Не требуются: изменения только в бэкенде, фронтенд и `ui/dist` не затронуты. Ничего в запущенном приложении не проверялось.

## Self-Check: PASSED

- Файлы: `crates/trackly-app/tests/groups_types_service.rs`, `crates/trackly-app/src/services/group_type_service.rs`, `crates/trackly-infra/src/repos/group_types_sqlite.rs` существуют.
- Коммиты 097886e1 и 5734bdbd присутствуют в `git log`.
