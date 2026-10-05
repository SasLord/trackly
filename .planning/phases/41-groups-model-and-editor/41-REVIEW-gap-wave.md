---
phase: 41-groups-model-and-editor
reviewed: 2026-10-06T00:00:00Z
depth: standard
scope: gap-closure wave, планы 41-27..41-34 (diff 89b13a1c..HEAD)
files_reviewed: 24
files_reviewed_list:
  - crates/trackly-app/src/services/group_type_service.rs
  - crates/trackly-app/src/services/report_service.rs
  - crates/trackly-app/src/tauri_cmds/reports.rs
  - crates/trackly-infra/src/repos/group_types_sqlite.rs
  - crates/trackly-app/tests/groups_types_service.rs
  - crates/trackly-app/tests/groups_property_removal_parity.rs
  - crates/trackly-app/tests/report_movements_truncation.rs
  - ui/src/lib/components/ActionMenu.svelte
  - ui/src/features/groups/GroupTypePanel.svelte
  - ui/src/features/groups/GroupTypePropertiesTable.svelte
  - ui/src/features/groups/propertyRemoval.ts
  - ui/src/features/reports/ReportsPage.svelte
  - ui/src/features/reports/truncationNotice.ts
  - ui/src/features/groups/GroupContentsTable.svelte
  - ui/src/features/groups/GroupPrintersList.svelte
  - ui/src/features/groups/GroupTreeNode.svelte
  - ui/src/features/places/PlaceTreeNode.svelte
  - ui/src/lib/components/NumberTemplateField.svelte
  - ui/scripts/check-action-menu-portal.mjs
  - ui/scripts/check-property-removal.mjs
  - ui/scripts/check-report-truncation.mjs
  - ui/scripts/fixtures/property-removal/cases.json
  - ui/scripts/fixtures/report-truncation/cases.json
  - ui/package.json
findings:
  critical: 0
  warning: 1
  info: 3
  total: 4
status: issues_found
---

# Фаза 41 (gap-wave 41-27..41-34): отчёт код-ревью

**Проверено:** 2026-10-06
**Глубина:** standard
**Статус:** issues_found

## Сводка

Критических дефектов нет. Все заявленные намерения волны выполнены, кроме одного
крайнего случая в 41-29 (WR-01): пара «скрыть сбрасывает обязательность» и
«показать отказывает легаси-строке» оставляет строку, которую из интерфейса не
вернуть.

Результаты по ранжированным проверкам:

1. **Вложенные транзакции (41-32): дефектов нет.** Все 8 мутаций `GroupTypeService`
   открывают одну `conn.transaction()` и зовут только `*_on(&tx, …)`, читатели
   принимают `&Connection`. Единственный трейтовый метод со своей транзакцией —
   `reorder_properties`, а сервис зовёт `reorder_properties_on`. Остальные трейтовые
   мутаторы — однооператорные делегаты без `BEGIN`. Вызовов трейтовых мутаторов
   изнутри открытой транзакции в `crates/*/src` нет (остались только
   `crates/trackly-infra/tests/group_types_repo.rs`). `seed_builtin_types` — девятая
   транзакция, не вложенная и без аудита (так было и раньше).
2. **COUNT(*) и основной запрос (41-30): расхождения нет.** `with_prefix`,
   `where_clause` и `param_refs` переиспользуются один в один. Второй запрос
   соединяет только `devices` — единственную таблицу, которую `where_clause`
   упоминает помимо `pm` (`d.type_id`). `LEFT JOIN` по первичному ключу не
   размножает строки, а `cartridges` и `users` в `WHERE` не участвуют. Условие
   `rows.len() >= LIMIT` запускает счёт ровно на потолке; кейс «ровно 1000» даёт
   `total == shown` и баннера нет.
3. **Покрытие аудитом (41-32): полное.** В каждой из 8 мутаций есть
   `audit_repo.insert(&tx, …)` и последующий `tx.commit()`. Ранние возвраты
   (ошибка валидации, нарушители обязательности, дубль имени) уничтожают `tx` через
   `Drop`, то есть откатывают. Откат при нарушителях в `create_property` заменил
   прежний ручной `delete_property_hard`, что корректно.
4. **Мёртвый CSS (41-27): нет.** `&:has(:global([aria-expanded='true']))` стоит внутри
   `.row-actions` в `GroupTreeNode` и `PlaceTreeNode`, `:global` внутри `:has()`.
   `aria-expanded={open}` действительно рендерит `ActionMenu` (строка 208). Других
   файлов с `opacity: 0` на триггере строки нет, гейт `[F]` их все видит.
5. **Руны Svelte (41-31, 41-33): замечаний нет.** `truncationNotice` и `removalCopy`
   — чистые `$derived`, без `$effect` и без записи читаемого состояния.
   `confirmRemove` захватывает `target` и `copy` до `await`.
6. **Уникальность якорей гейтов:** в `check-report-truncation.mjs` и
   `check-property-removal.mjs` `assertUniqueAnchor` считает вхождения до замены,
   а truncation-гейт использует `split/join`. В `check-property-removal.mjs`
   `.replace(m.from, m.to)` безопасен: в строках `to` нет `$&`, `$1` и т. п.
   Восьмой кейс «500 из 800» в фикстуре читает и Rust-тест
   (`report_trunc_notice_matches_golden_fixture`), и JS-гейт, так что мутант
   «`total <= 1000`» ловится на обеих сторонах.
7. **Приватность: чисто.** В коде, тестах, фикстурах и гейтах только вымышленные
   значения («Хост», «Стойка N», «значение»). Реквизитов, ФИО, e-mail и телефонов
   нет.

## Warnings

### WR-01: легаси «скрыто + обязательное» с нарушителями не возвращается — тупик

**Файл:** `crates/trackly-app/src/services/group_type_service.rs:709-717`
(в связке с `ui/src/features/groups/GroupTypePropertiesTable.svelte:570-581, 591`)

**Проблема:** `unarchive_property` для строки с `is_required = 1` и найденными
`groups_missing_required` возвращает `Validation{is_required}` с текстом
`required_violation_error`: «Нельзя сделать свойство обязательным: у групп нет
значения: … Заполните его в этих группах.» Выхода из этого состояния нет:

- скрытое свойство из форм групп выпало, значения в этих группах заполнить
  нельзя, а совет из сообщения выполнить невозможно;
- в таблице свойств у скрытой строки чекбоксы «Обязательное» и «На карте»
  заблокированы (`disabled={!canEdit || p.archived}`, строки 570 и 581), а меню
  у скрытой строки содержит только «Показать»;
- само сообщение описывает другое действие («сделать обязательным»), хотя
  пользователь нажимал «Показать».

Сценарий: строка скрыта до 41-29, когда `archive_property` флаг не сбрасывал
(`is_required = 1`, `archived_at_utc` задан), а в типе есть группа без значения.
Нажатие «Показать» всегда падает. Достижимо только на БД, созданных до 41-29, но
это ровно тот набор, ради которого проверка добавлена (комментарий `W-B01:2`).

Это расходится с политикой самой волны: `archive_property_on` сбрасывает флаг при
скрытии, то есть «скрытое не бывает обязательным». Честное поведение для
наследной строки — тот же сброс при возврате, а не отказ.

**Исправление:** в `unarchive_property` вместо отказа гасить флаг в том же UPDATE:
оставить проверку нарушителей только для вычисления, а при их наличии вызвать
`unarchive_property_on` вместе со сбросом `is_required`. Тогда возврат всегда
удаётся, а инвариант «живое обязательное свойство не запирает `set_values`»
сохраняется. Например, в `unarchive_property_on` добавить `is_required = CASE WHEN
?3 = 1 THEN 0 ELSE is_required END` (флаг «есть нарушители» приходит параметром),
либо сбрасывать `is_required` безусловно, как при скрытии. Тест
`protect_d_unarchive_refuses_legacy_required_with_violators` нужно заменить на
проверку «возврат удался, флаг снят».

## Info

### IN-01: копирайт модалки может устареть к моменту подтверждения

**Файл:** `ui/src/features/groups/GroupTypePropertiesTable.svelte:361-375`

**Проблема:** `removalCopy` считается от снимка строки `removeTarget`
(`filled_group_count` на момент открытия модалки). Если за время открытой
модалки значения были очищены или группы удалены, сервер выберет ветку «удалить
физически», хотя модалка обещала «свойство можно будет вернуть». Тост по факту
исхода (`outcome.archived`) честный, но предупреждение «удалено безвозвратно»
пользователь уже не увидел. Обратная гонка безопасна. Потеряется только
определение свойства (значений нет, иначе была бы ветка скрытия), поэтому Info.

**Исправление:** по желанию — перед открытием модалки перечитывать свойство, либо
в `confirmRemove` сравнивать `outcome.archived` с `copy.kind === 'hide'` и при
расхождении показывать явный тост («Свойство удалено: значений уже не было»).

### IN-02: отложенная транзакция начинается с чтения

**Файл:** `crates/trackly-app/src/services/group_type_service.rs:264, 356, 407, 470, 574, 657, 700, 762`

**Проблема:** `conn.transaction()` — `DEFERRED`, а в каждой мутации сначала идут
чтения (`get_type`, `list_properties`, `filled_group_count`), потом запись. Если
бы появился второй писатель на файл БД, повышение блокировки из-за чтения мог бы
завершить `SQLITE_BUSY_SNAPSHOT`, который `busy_timeout` не лечит. Сейчас писатель
единственный (ограничение проекта), поэтому недостижимо; в кодовой базе
`transaction_with_behavior` нигде не используется, паттерн единообразен.

**Исправление:** при желании `conn.transaction_with_behavior(TransactionBehavior::Immediate)`
для мутаций, которые читают перед записью. Сейчас можно оставить как есть.

### IN-03: фраза уведомления об усечении склеивается с резюме фильтров через точку

**Файл:** `crates/trackly-app/src/tauri_cmds/reports.rs:403-407`

**Проблема:** `format!("{summary}. {notice}")`. Резюме собирается как
`"Откуда: {path}; Куда: {path}; Тип устройства: {name}"` (`parts.join("; ")`) и
заканчивается именем без точки, так что обычно всё корректно. Если имя места или
типа заканчивается на «.», в печати получится «..». Косметика, на правильность не
влияет.

**Исправление:** `summary.trim_end_matches('.')` перед склейкой.

---

_Проверено: 2026-10-06_
_Ревьюер: Claude (gsd-code-reviewer)_
_Глубина: standard_
