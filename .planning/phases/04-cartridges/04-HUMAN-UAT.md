---
status: partial
phase: 04-cartridges
source: [04-VERIFICATION.md, 04-06-PLAN.md]
started: "2026-06-12T00:00:00Z"
updated: "2026-06-12T00:00:00Z"
---

## Current Test

[awaiting human testing]

## Tests

### 1. Полный end-to-end lifecycle в разделе «Картриджи»
expected: Запустить `cargo tauri dev`, открыть раздел «Картриджи», выполнить сценарий из PLAN 04-06 Task 3 (шаги 1–15): создание модели с матрицей совместимости, создание экземпляра с авто-кодом C-000001, установка в принтер, возврат на склад, switch-bar фильтрация, поиск, баннер низкого остатка. Все шаги без ошибок; Toast-уведомления появляются; статусы и счётчики обновляются реактивно.
result: [pending]

### 2. Скрытие поля «Цвет» при выборе типа «Фотобарабан»
expected: В форме «Новая модель картриджа» переключить тип с «Картридж» на «Фотобарабан» — поле «Цвет» немедленно скрывается; при возврате к «Картридж» поле снова появляется.
result: [pending]

### 3. Focus-open autocomplete в CompatibilityEditor
expected: В форме модели нажать «+ Добавить принтер», кликнуть в поле «Бренд принтера» без ввода — dropdown открывается сразу и показывает ранее введённые бренды (пустой при первом запуске). Аналогично для «Модель принтера» после выбора бренда. (Примечание: бэкенд-баг field-name mismatch исправлен — CR-01.)
result: [pending]

### 4. Human-verify checkpoint из PLAN 04-06 Task 3 (blocking gate)
expected: Выполнить все 15 шагов сценария из `<how-to-verify>` Task 3. Написать "approved", если раздел работает корректно.
result: [pending]

## Summary

total: 4
passed: 0
issues: 3
pending: 4
skipped: 0
blocked: 0

## Gaps

### UAT Round 1 — 2026-06-12 (issues found, fixes applied)

- **GAP-1 (Critical) — `ReaderPool` exhaustion → mutex poison panic.** `cargo tauri dev`
  паниковал при открытии раздела «Картриджи»: `ReaderPool exhausted` под удержанным
  локом → `PoisonError` каскадом → весь reader-пул мёртв процесс-wide. Причина:
  CartridgesPage `loadAll()` (`Promise.all([list, counts, lowStock])` + model_list/
  search) даёт >4 одновременных чтений против пула size=4.
  **Fix:** `acquire()` теперь блокируется на `Condvar` (queue-on-exhaust) + poison-
  устойчивые `lock()` (into_inner); размер 4→8. Регресс-тест добавлен. Commit `c2e5626`.

- **GAP-2 — Модель создаётся («уже создана» при повторе), но не отображается в списках;
  первый сабмит даёт «Данные изменились в другом окне».** Downstream-симптом GAP-1:
  writer-create проходит (отдельный канал), но последующие reader-чтения (refresh
  списка, post-create read) били в мёртвый пул → пустые списки и неверно
  смапленная ошибка. `list_models` SQL корректен (`WHERE deleted_at_utc IS NULL`).
  **Ожидается resolved через GAP-1 — требует повторного ручного теста.**

- **GAP-3 (UX) — switch-bar статусов был зажат внутри узкой колонки списка.**
  **Fix:** `CartridgeFilters` вынесен на уровень страницы (полная ширина, между
  строкой поиска и списком). Commit `158bab5`.

Status: fixes applied, backend tests + clippy + svelte-check + lint зелёные.
Awaiting UAT Round 2 (re-run `cargo tauri dev`).

### UAT Round 2 — 2026-06-12 (паника устранена; 5 замечаний, фиксы применены)

Критическая паника `ReaderPool` устранена — lifecycle работает. Найдено 5 замечаний:

- **R2-1 — Форма модели не очищается при повторном открытии.** `{#key}` ремаунтил
  только разметку; component-level `$state` не реинициализировался. **Fix:** сброс
  полей из `target` в open-transition `$effect`. Commit `6ceb2da`.
- **R2-2 — Метки «Бренд/Модель принтера» дублировались над каждой строкой
  совместимости.** **Fix:** один header-row сверху; per-row метки → visually-hidden;
  убран `wrapperEls` bind:this (ушёл console-warning `binding_property_non_reactive`).
  Commit `a8a8578`.
- **R2-3 — Поиск и свитч-бар (Картриджи/Модели) распались на 2 строки.** `.search-and-
  tabs` был column до брейкпоинта 1280px. **Fix:** всегда одна строка (поиск слева,
  свитч-бар справа). Commit `e0ccfc7`.
- **R2-4 — В списке моделей всегда «0 шт.».** `instanceCount` был хардкод `0`, DTO не
  нёс счётчик. **Fix:** `CartridgeModelDto.instance_count` + repo
  `count_instances_by_model` (GROUP BY model_id, живые) + enrich в сервисе + bind в UI;
  регресс-тест. Commit `c237224`.
- **R2-5 — Нет визуального индикатора заряда; столбец статуса избыточен при фильтре по
  статусу.** **Fix:** цветной charge-dot в строке (Полный→зелёный, Частичный→янтарный,
  Пустой→красный, title=state_name); бейдж статуса скрывается при `statusFiltered`.
  Commit `eca481f`.

Status: fixes applied; `cargo test` + `clippy -D warnings` + `svelte-check` (0 errors) +
`lint` зелёные. Awaiting UAT Round 3.

### UAT Round 3 — 2026-06-13 (4 замечания; фиксы применены, включая крупную доработку)

- **R3-1 — Нет реактивности детали после редактирования картриджа.** Редактирование
  уже выбранного картриджа не меняло `selectedCartridgeId` → detail `$effect` не
  перезапускался. **Fix:** явный refresh detail+history из возвращённого DTO. Commit `e34728d`.
- **R3-2 — Списанные картриджи: серый индикатор заряда.** Commit `3f55baa`.
- **R3-3 — Фильтр «Тип»: метка пропадала; «Модель» не зависела от типа.** Строковые
  value у `<select>` vs числовой kindId (строгое сравнение Svelte). **Fix:** числовые
  value; модели фильтруются по типу; сброс модели при смене типа. Commit `b3fc6a5`.
- **R3-4 (крупная доработка) — Разделение картриджей и фотобарабанов на уровне
  экземпляров.** Миграция V017 (состояния барабанов Новый/Изношенный/Отработанный +
  kind_id на cartridge_states + счётчик drum_seq); коды C-/D- по виду модели; домен:
  фотобарабан без заправки, отработанный нельзя устанавливать; форма создания с
  выбором вида первым полем, модели/состояние/код по виду; OperationModal и
  контекстное меню учитывают вид. Commits `e60e2d3` (backend) + `a5572b3` (UI).
  Схема-версия 16→17 (существующие dev-БД авто-апгрейдятся при следующем запуске).

Status: `cargo test` (вкл. новые тесты D-кода и переходов) + `clippy -D warnings` +
`svelte-check` (0 errors) + `lint` зелёные. Awaiting UAT Round 4.
