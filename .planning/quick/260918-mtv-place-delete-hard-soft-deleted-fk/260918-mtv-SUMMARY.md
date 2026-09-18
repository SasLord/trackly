---
phase: 260918-mtv
plan: 01
subsystem: places
tags: [rusqlite, sqlite, soft-delete, foreign-keys, error-mapping]
status: complete
key-files:
  modified:
    - crates/trackly-infra/src/repos/places_sqlite.rs
    - crates/trackly-app/tests/places_delete_blocked.rs
---

# Quick 260918-mtv: удаление места со ссылками только от удалённых записей

## Что было

Предполётная проверка `delete_hard` (`subtree_stats_impl`, сервисный и
репозиторный слои) считает только живые строки. Soft-delete устройства/картриджа/
акта оставляет `place_id`, а FK V038 (`ON DELETE RESTRICT`) действуют и на такие
строки. Место, на которое ссылаются только удалённые записи, проходило проверку,
`DELETE` падал на FK, и `map_rusqlite` отдавал клиенту сырое
`FOREIGN KEY constraint failed` (находка 40.1-SECURITY, low — только админ).

## Что стало

`places_sqlite::delete_hard` маппит ошибку `DELETE FROM places` через новую
`map_place_delete_error`: FK-нарушение → `AppError::Conflict` с текстом
«Место нельзя удалить: на него ссылаются удалённые записи (устройства, картриджи,
акты или вложенные места), которые сохраняются в истории. Архивируйте место.»
Прочие ошибки — как раньше, через `map_rusqlite`.

Решения:
- Catch-all на DELETE, а не расширение счётчиков: ловит все RESTRICT-пути (в т.ч.
  удалённые дочерние места и будущие FK), а `SubtreeStats` питают UI-счётчики,
  куда удалённые записи попадать не должны. Точные счётчики для живых записей не
  тронуты.
- Распознавание по классу `ConstraintViolation` + фиксированному тексту SQLite,
  а не по `SQLITE_CONSTRAINT_FOREIGNKEY` (787): bundled SQLite отдаёт эту
  проверку в конце выражения с extended code 1811 (`SQLITE_CONSTRAINT_TRIGGER`)
  — проверено отладочным выводом; сопоставление по 787 тест не проходило.

## Тесты

`delete_blocked_by_soft_deleted_device_has_russian_reason` в
`places_delete_blocked.rs`: место с одним soft-deleted устройством → `Conflict`,
reason без «FOREIGN KEY», начинается с «Место нельзя удалить:», содержит
«удалённые записи»; место физически осталось. Подтверждено красным без фикса
(`FOREIGN KEY constraint failed`) и зелёным с фиксом.

## Гейты

- `places_delete_blocked` 9/9; все `place*`-тесты trackly-app зелёные
  (act_link, bulk_move, timeline, write_sites ×2, contents, move_cycle, search,
  service_crud); `trackly-infra --lib` 149/149.
- `cargo clippy --workspace --all-targets -D warnings` — чисто.
- `rustfmt --check` по затронутым файлам — чисто.
- Приватность: только вымышленные данные («Ноутбук удалённый», место «515»).

## Отклонения

Нет, кроме способа распознавания FK-ошибки (см. выше).
