# Phase 13: Редизайн совместимости Принтеры↔Картриджи + свёрнутые chip-задачи - Context

**Gathered:** 2026-06-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Перевод модели совместимости «принтер↔картридж» с per-device junction (`printer_cartridge_models`, V029) на free-text-связь по уникальному наименованию принтера (`devices.name`), консолидация UI совместимости в один блок на стороне модели картриджа, добавление на карточку принтера read-only агрегатов + блока данных устройства с диалогом редактирования, плюс две свёрнутые chip-задачи (kind-aware дефолт авто-возврата фотобарабана; устранение рассогласования лимита списка принтеров). Требования зафиксированы в 13-SPEC.md.

</domain>

<spec_lock>
## Requirements (locked via SPEC.md)

**8 requirements are locked.** See `13-SPEC.md` for full requirements, boundaries, and acceptance criteria.

Downstream agents MUST read `13-SPEC.md` before planning or implementing. Requirements are not duplicated here.

**In scope (from SPEC.md):**
- Миграция: `DROP TABLE printer_cartridge_models` (V029); строки V005 `cartridge_model_compatibility` сохраняются (с конверсией схемы — см. D-01).
- Удаление Rust-кода чтения/записи V029 и фильтра `compatible_with_printer_device_id` (перевод на V005).
- Удаление фронт-редакторов `CompatibleDevicesEditor.svelte` (сторона картриджа) и `CompatibleModelsEditor.svelte` (сторона принтера).
- Единый блок «Совместимые принтеры» в форме модели картриджа: автокомплит по DISTINCT наименованиям принтеров + свободный ввод, запись в V005.
- Карточка принтера: read-only блок агрегатов совместимости.
- Карточка принтера: блок данных устройства (Инвентарный №, Серийный №, Расположение, Состояние) + кнопка/карандаш «Редактировать» → диалог «Редактирование устройства».
- Карточка принтера: установленный картридж по коду + наименованию.
- Chip-задача 1: kind-aware дефолт состояния авто-возврата + реюз stateOptions на фронте + регресс-тест.
- Chip-задача 2: простой фикс/параметризация капа лимита принтеров.

**Out of scope (from SPEC.md):**
- Нормализованный каталог моделей принтеров (отдельная таблица `printer_models` с FK) — выбран free-text V005.
- Миграция данных V029 → новая модель — чистый старт, V029 дропается без переноса связей.
- Слияние/автодедупликация дублей текстовых наименований принтеров в V005.
- Полная пагинация списка принтеров в OperationModal — только фикс капа.
- Очистка/wipe существующих строк V005 — строки сохраняются.

</spec_lock>

<decisions>
## Implementation Decisions

### Ключ совместимости и схема V005
- **D-01:** Ключ матчинга совместимости = `devices.name` принтера (НЕ `devices.model`, НЕ `printers.vendor`). «Уникальное наименование/тип принтера» из SPEC = именно `devices.name`.
- **D-02:** **Миграция схемы V005** `cartridge_model_compatibility`: два столбца `printer_brand` + `printer_model` сворачиваются в один `printer_name`. Существующие строки конвертируются (collapse `TRIM(printer_brand || ' ' || printer_model)` → `printer_name`) — строки сохраняются, не удаляются. Новые строки пишут только `printer_name`.
- **D-03:** Матчинг «совместимый картридж для принтера» резолвится сопоставлением `cartridge_model_compatibility.printer_name` с `devices.name` принтера. Сравнение **case-insensitive + trimmed** (NOCASE/LOWER + TRIM с обеих сторон), чтобы «Pantum P2200» / «pantum p2200 » считались совпадением.
- **D-04:** Фильтр совместимости в `CartridgeRepository::list()` (текущий параметр `compatible_with_printer_device_id`, `cartridges_sqlite.rs:1097-1099`) переписывается с подзапроса по `printer_cartridge_models` на подзапрос по `cartridge_model_compatibility` через `devices.name` принтера (lookup name по device_id).
- **D-05:** **Пустая совместимость = pass-through.** Если у модели картриджа нет НИ ОДНОЙ записи в `cartridge_model_compatibility` — её картриджи доступны для ЛЮБОГО принтера (фильтр пропускает unfiltered). Сохраняет текущую семантику D-14 из Phase 12 (логика «NOT EXISTS … OR …»). Только при наличии хотя бы одной записи модель ограничивается совпадающими наименованиями.

### Источник списка принтеров для автокомплита (B2)
- **D-06:** Единый блок «Совместимые принтеры» в форме модели картриджа: автокомплит предлагает `SELECT DISTINCT devices.name` среди принтеров (`type_id = 2`), плюс допускает свободный ввод наименования модели, отсутствующей в БД (сохраняется как текст в `printer_name`, «под будущее»). `CompatibleDevicesEditor.svelte` удаляется; старый `CompatibilityEditor.svelte` заменяется/перерабатывается под единый блок и единый столбец `printer_name`.

### Карточка принтера — агрегаты (read-only)
- **D-07:** Агрегаты показывают **три статуса в строгом порядке: На складе (status_id=1) → На заправке (status_id=3) → В работе (status_id=2)**. Статус «Списано» (4) НЕ показывается. Группировка — по каждой совместимой модели картриджа (напр. «Cactus CS-TL-5120: На складе 4, На заправке 1, В работе 2»). Блок строго read-only — без элементов добавить/удалить совместимость. `CompatibleModelsEditor.svelte` удаляется.

### Карточка принтера — блок устройства + редактирование
- **D-08:** Блок данных устройства показывает поля: **Инвентарный №, Серийный №, Расположение, Состояние** (из `devices` для устройства принтера). Рядом — кнопка/иконка-карандаш «Редактировать».
- **D-09:** Диалог «Редактирование устройства» = **переиспользование существующего `ui/src/features/devices/DeviceFormModal.svelte`** (+ `DeviceFormBody.svelte`), открываемого с `device_id` принтера. Не создавать отдельный упрощённый диалог — единый UX и валидация.

### Chip-задача 1 — kind-aware дефолт авто-возврата
- **D-10:** В авто-возврате (`cartridges_sqlite.rs` ~:474, `previous_cartridge_state_id.unwrap_or(3)`) дефолт состояния выбирается по `model_kind_id` предыдущего картриджа: kind=1 (картридж) → прежний дефолт «Пустой» (3); kind=2 (фотобарабан) → **«Изношенный» (5)**. Снапшот предыдущего картриджа должен нести `model_kind_id`, чтобы выбрать ветку.
- **D-11:** Фронт `OperationModal.svelte` (~:506-514) перестаёт хардкодить состояния 1/2/3 — переиспользует `stateOptions`/`DRUM_STATES` по виду модели.
- **D-12:** Замечание для планировщика: дефолт «Изношенный» (5) попадает в installable-набор drum (фильтр `cartridges_sqlite.rs:1095` — `kind=2 AND state_id IN (4,5)`), т.е. авто-возвращённый барабан остаётся доступным для установки. Это намеренно (долгий ресурс фотобарабана) — НЕ менять фильтр.

### Chip-задача 2 — лимит списка принтеров
- **D-13:** **Uncapped read** для команды списка принтеров, обслуживающей селектор установки в `OperationModal.svelte`: бэкенд отдаёт ВСЕ принтеры без обрезки `min(200)` (`printers_sqlite.rs:314`). Парк маленький, пагинация не вводится. Согласовать фронт (убрать/обессмыслить `limit:500`) и бэкенд так, чтобы совместимый принтер не терялся при любом размере парка.

### Claude's Discretion
- Точная форма миграции (имя `V032`, ALTER vs пересоздание таблицы для смены столбцов в SQLite — вероятно create-new + copy + drop из-за ограничений ALTER) — на усмотрение планировщика/исполнителя, при сохранении строк.
- Нужно ли вводить новую/переименованную read-команду для uncapped-списка принтеров или параметризовать существующую — на усмотрение планировщика (с учётом двойного транспорта Tauri+HTTP, specta-экспорта `bindings.ts`, `role_endpoint_matrix`).
- Конкретная вёрстка блоков на карточке принтера.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Locked requirements (read first)
- `.planning/phases/13-per-device-junction-chip-drum-state/13-SPEC.md` — Locked requirements, boundaries, acceptance criteria — MUST read before planning.

### Schema / migrations
- `migrations/V029__printer_cartridge_models.sql` — per-device junction, подлежит DROP.
- `migrations/V005__cartridges.sql` §`cartridge_model_compatibility` — таблица free-text совместимости, подлежит миграции схемы (collapse → `printer_name`).
- `migrations/V001__init_pragmas_and_lookups.sql` §`cartridge_statuses` — статусы 1 На складе / 2 В работе / 3 На заправке / 4 Списано.
- `migrations/V017__drum_states_and_counter.sql` — drum-состояния 4 Новый / 5 Изношенный / 6 Отработанный (kind_id=2), счётчик drum_seq.
- `migrations/V003__devices.sql` — `devices.name` (ключ матчинга), `model`, `inventory_number`, `serial_number`, `condition`, `location_id`.
- `migrations/V020__printers.sql` / `migrations/V025__cartridge_printer_link.sql` — printers, `current_printer_device_id`.

### Backend (Rust)
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` — V005 read/write (~:290-317), фильтр `compatible_with_printer_device_id` (:1082-1152), авто-возврат drum-default (~:474), installable-фильтр (:1093-1096).
- `crates/trackly-app/src/services/cartridge_service.rs` — DISTINCT-автокомплит совместимости (~:880-895).
- `crates/trackly-infra/src/repos/printers_sqlite.rs` — список принтеров и кап `page.limit.min(200)` (:314).

### Frontend (Svelte)
- `ui/src/features/cartridges/CompatibilityEditor.svelte` — старый free-text редактор (перерабатывается под единый блок).
- `ui/src/features/cartridges/CompatibleDevicesEditor.svelte` — per-device (УДАЛИТЬ).
- `ui/src/features/cartridges/ModelFormModal.svelte` — форма модели картриджа (хост единого блока).
- `ui/src/features/cartridges/OperationModal.svelte` — установка картриджа: хардкод состояний (~:506-514), `limit:500` (~:277).
- `ui/src/features/printers/CompatibleModelsEditor.svelte` — per-device на карточке принтера (УДАЛИТЬ).
- `ui/src/features/printers/PrinterDetail.svelte` — карточка принтера (агрегаты read-only + блок устройства).
- `ui/src/features/devices/DeviceFormModal.svelte` / `DeviceFormBody.svelte` — переиспользуемый диалог редактирования устройства.

### Phase 12 context (фон)
- `.planning/phases/12-cartridge-request-interconnection/12-CONTEXT.md` — решения D-01..D-22 Phase 12; V029/per-device UI введены здесь как промежуточное решение (теперь сносятся).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `DeviceFormModal.svelte` + `DeviceFormBody.svelte`: готовый диалог редактирования устройства с полной валидацией — переиспользуется на карточке принтера (D-09).
- DISTINCT-автокомплит уже существует в `cartridge_service.rs` (~:893) поверх `cartridge_model_compatibility` — паттерн для DISTINCT `devices.name` автокомплита (D-06).
- Паттерн V005 read/write (DELETE+re-INSERT по `cartridge_model_id`, `cartridges_sqlite.rs:290-317`) переносится на единый столбец `printer_name`.

### Established Patterns
- Двойной транспорт: любые изменения read/write команд отражаются в Tauri-cmd + axum HTTP + specta-экспорт `ui/src/bindings.ts` + `role_endpoint_matrix` (см. Phase 12, план 12-21).
- Миграции refinery: следующий номер `V032__*`, `PRAGMA user_version` инкремент; смена набора столбцов в SQLite обычно через create-new-table + copy + drop (ALTER ограничен).
- Installable-семантика хранится в SQL-фильтре (`state_id IN (...)` по kind) — kind=1→(1,2), kind=2→(4,5).

### Integration Points
- Фильтр выбора картриджа при установке (`compatible_with_printer_device_id`) — главная точка переключения с V029 на V005 + name-lookup по device_id.
- `PrinterDetail.svelte` — хост двух новых блоков (агрегаты + устройство), удаление `CompatibleModelsEditor`.
- `ModelFormModal.svelte` — хост единого блока «Совместимые принтеры», удаление `CompatibleDevicesEditor`.

</code_context>

<specifics>
## Specific Ideas

- Агрегаты на карточке принтера — точный порядок и набор: «На складе → На заправке → В работе» (формат как «Cactus CS-TL-5120: …»), без «Списано».
- Блок устройства — конкретный набор полей: Инвентарный №, Серийный №, Расположение, Состояние; кнопка «Редактировать» либо иконка-карандаш.
- Drum-дефолт авто-возврата — именно «Изношенный» (5) как безопасный нейтральный аналог «Частичный/Пустой».

</specifics>

<deferred>
## Deferred Ideas

- Нормализованный каталог моделей принтеров (`printer_models` с FK) — отклонён в пользу free-text V005 (зафиксировано в SPEC out-of-scope).
- Автодедупликация/нормализация похожих текстовых наименований принтеров в V005 — будущая фаза при росте парка.
- Полная пагинация списка принтеров в OperationModal — будущая фаза, если парк превысит разумный uncapped-объём.

</deferred>

---

*Phase: 13-per-device-junction-chip-drum-state*
*Context gathered: 2026-06-25*
