---
quick_id: 260918-mtv
slug: place-delete-hard-soft-deleted-fk
phase: 260918-mtv
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-infra/src/repos/places_sqlite.rs
  - crates/trackly-app/tests/places_delete_blocked.rs
autonomous: true
must_haves:
  truths:
    - "Удаление места, на которое ссылаются только soft-deleted записи (устройство/картридж/акт/вложенное место), отклоняется AppError::Conflict с русским сообщением, а не сырым `FOREIGN KEY constraint failed`"
    - "Сообщение объясняет причину (удалённые записи хранятся в истории) и предлагает архивировать место"
    - "Счётчики SubtreeStats (дерево мест, превью последствий) не меняются — удалённые записи в них по-прежнему не видны"
    - "Удаление пустого листа по-прежнему проходит"
---

# Quick 260918-mtv: удаление места, на которое ссылаются только удалённые записи

Находка аудита 40.1-SECURITY (low, только админ): предполётная проверка
`delete_hard` считает лишь живые строки (`deleted_at_utc IS NULL`), а FK V038
(`ON DELETE RESTRICT`) держат и soft-deleted строки. Итог — `DELETE` падает на
FK, и `map_rusqlite` отдаёт клиенту сырой английский текст SQLite. Тот же класс,
что CR-01 фазы 39.

## Решение

Ловить FK-нарушение на самом `DELETE FROM places` в `places_sqlite::delete_hard`
(класс ConstraintViolation + фиксированный текст «FOREIGN KEY constraint failed»; extended code у bundled SQLite = 1811, а не 787) и превращать в контролируемый
`AppError::Conflict` на русском. Выбрано вместо расширения счётчиков:

- ловит ВСЕ пути RESTRICT-ссылок (devices, cartridges, acts, act_items,
  place_movements, дочерние places — в т.ч. удалённые дочерние места), включая
  будущие, а не только перечисленные в запросе;
- не трогает `SubtreeStats`, которые показываются в UI как счётчики содержимого —
  добавлять туда удалённые записи было бы неверно;
- предполётная проверка с точными счётчиками для живых записей остаётся как есть.

Текст: «Место нельзя удалить: на него ссылаются удалённые записи (устройства,
картриджи, акты или вложенные места), которые сохраняются в истории. Архивируйте
место.»

## Задачи

1. `places_sqlite.rs::delete_hard` — маппинг FK-ошибки DELETE в Conflict.
2. Тест в `places_delete_blocked.rs`: место со soft-deleted устройством → отказ,
   `Conflict`, reason без «FOREIGN KEY», начинается с «Место нельзя удалить:»;
   место физически осталось.
3. Гейты: `cargo test -p trackly-app --test places_delete_blocked`,
   `trackly-infra --lib`, clippy, fmt по файлам, check-privacy.
