# Quick Task 260904-x7s: удаление акта должно освобождать его номер - Context

**Gathered:** 2026-09-04
**Status:** Ready for planning

<domain>
## Task Boundary

Сейчас после удаления акта №20 создать новый акт с номером 20 невозможно: сервис
специально проверяет уникальность номера **включая soft-deleted** строки
(`act_service.rs:275` в `create`, `act_service.rs:896` в `update`, комментарий
ссылается на решение D-Soft-vs-Hard-Acts-01). Плюс автономер берётся из
монотонного счётчика `counters.act_number`, который никогда не откатывается,
поэтому кнопка «Следующий» после удаления последнего акта предлагает 21, а не 20.

Нужно: удалённый акт освобождает свой номер — и для ручного ввода, и для
автономера.

</domain>

<decisions>
## Implementation Decisions

### Способ удаления (заменяет часть решения D-Soft-vs-Hard-Acts-01)
- Soft-delete СОХРАНЯЕТСЯ: строка акта остаётся с `deleted_at_utc`, история и
  возможность undo через `audit_log` не трогаются. Никакого hard-delete строк.
- Меняется ТОЛЬКО правило уникальности: проверка перестаёт учитывать
  soft-deleted акты. Т.е. `SELECT EXISTS(SELECT 1 FROM acts WHERE number=?1)`
  должен получить фильтр `AND deleted_at_utc IS NULL` — так он совпадёт с уже
  существующим частичным UNIQUE-индексом `idx_acts_number_sub_unique`
  (`WHERE deleted_at_utc IS NULL`, V004__acts.sql), который и так считает номер
  удалённого акта свободным.
- Обе точки проверки (`create` и `update`/переименование номера) должны
  измениться одинаково — иначе поведение разъедется.

### Автономер («Следующий»)
- `counters.act_number` перестаёт быть источником истины для предлагаемого
  номера: следующий номер = `MAX(number) + 1` среди ЖИВЫХ актов
  (`deleted_at_utc IS NULL`), с fallback на 1 для пустой таблицы.
- Ожидаемое поведение: удалили последний акт №20 → «Следующий» снова
  предлагает 20, без ручного ввода и без бейджа «override».
- Затрагивает `ActRepository::peek_next_number` (`acts_sqlite.rs:652`) и то,
  как `create` резолвит номер при отсутствии `number_override`
  (`increment_counter_in_tx(&tx, "act_number")` — при переходе на MAX+1
  автоприсвоение должно давать тот же освобождённый номер, а не значение
  счётчика).
- Гонок не возникает: единственный писатель (D-WriterChannel-01) + BEGIN
  IMMEDIATE, вычисление MAX идёт внутри той же транзакции, что и INSERT.

### Claude's Discretion
- Оставлять ли строку `counters.act_number` в БД (миграция на удаление НЕ
  требуется — при сомнении оставить как есть, чтобы не ломать V009 и
  `cartridge_seq`).
- Как именно поступить с audit-записью `custom:act_number_override` и полем
  `next_auto_would_be` — сохранить смысл, пересчитав от нового источника.
- Формулировка и место тестов; какие существующие тесты
  (`acts_numbering.rs`, `acts_crud.rs`) требуют обновления.

</decisions>

<specifics>
## Specific Ideas

Точки в коде, найденные при разборе:
- `crates/trackly-app/src/services/act_service.rs:272-284` — проверка
  уникальности в `create` (комментарий «Uniqueness check INCLUDING
  soft-deleted (D-Soft-vs-Hard-Acts-01)» тоже надо обновить).
- `crates/trackly-app/src/services/act_service.rs:896` — та же проверка в
  `update` (переименование номера, Phase 19, тест
  `number_change_rejects_duplicate`).
- `crates/trackly-app/src/services/act_service.rs:285` — авто-ветка
  `increment_counter_in_tx(&tx, "act_number")`.
- `crates/trackly-infra/src/repos/acts_sqlite.rs:652` — `peek_next_number`
  (сейчас `peek_counter + 1`).
- `migrations/V004__acts.sql:30` — `idx_acts_number_sub_unique` уже частичный
  (`WHERE deleted_at_utc IS NULL`); менять схему не нужно.
- Возвраты нумеруются отдельно через `next_sub_number_for_parent`, который уже
  фильтрует `deleted_at_utc IS NULL` — его трогать не нужно.

</specifics>

<canonical_refs>
## Canonical References

- `.planning/phases/03-pdf/03-CONTEXT.md` §D-Soft-vs-Hard-Acts-01 — исходное
  решение блокировать переиспользование номеров удалённых актов. Данная
  quick-задача **осознанно отменяет** этот пункт по прямому решению
  пользователя; остальная часть решения (soft-delete акта + hard-delete
  `act_items`, cascade) остаётся в силе.
- `.planning/phases/19-acts-date-edit/19-RESEARCH.md` — требование, чтобы
  `update`-путь переиспользовал ту же проверку уникальности, что и `create`.

</canonical_refs>
