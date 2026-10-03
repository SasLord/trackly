# Фаза 41: Группы: модель и редактор — карта паттернов

**Составлено:** 2026-10-04
**Файлов проанализировано:** 62 (новых и изменяемых)
**Найдено аналогов:** 58 / 62 (4 файла без прямого аналога — раздел «Аналог не найден»)

Номера строк даны на HEAD `262f3658`. Перед использованием в плане перепроверять `grep`-ом
(файлы живые, особенно `act_service.rs`, `device_service.rs`, `report_service.rs`).
Все примеры данных вымышленные (CLAUDE.md, гейт приватности): «Иванов И.И.», «Петров П.П.», «АРМ #3».

---

## Классификация файлов

### Схема и домен (Rust)

| Новый / изменяемый файл | Роль | Поток данных | Ближайший аналог | Качество |
|-------------------------|------|--------------|------------------|----------|
| `migrations/V045__groups.sql` | migration | CRUD (DDL + индексы + триггер) | `migrations/V041__number_templates.sql`, шапка `V040__place_movements.sql` | role-match |
| `migrations/V046__place_movements_batch.sql` | migration | DDL (аддитивный `ALTER`) | `migrations/V039__place_path_display.sql` | exact |
| `crates/trackly-core/src/domain/groups.rs` | model (домен, без serde/specta) | CRUD | `domain/places.rs` (`PlaceRow`, `PlaceKind`, `PlaceNew`) | exact |
| `crates/trackly-core/src/domain/group_values.rs` | utility (чистые функции) | transform | `domain/place_movements.rs::is_reportable_place_change` + `domain/places.rs::sibling_cmp` | role-match |
| `crates/trackly-core/src/ports/groups.rs` | port | CRUD | `ports/places.rs` | exact |
| `crates/trackly-core/src/auth.rs` (+3 `Action`) | config/guard | request-response | сам файл, ветка `ReadPlaces`/`MutatePlaces` (стр. 125-171) | exact |
| `crates/trackly-core/src/domain/place_movements.rs` (+`Group`) | model | — | сам файл (стр. 19-90) | exact |

### Репозиторий и сервис (Rust)

| Файл | Роль | Поток данных | Ближайший аналог | Качество |
|------|------|--------------|------------------|----------|
| `crates/trackly-infra/src/repos/groups_sqlite.rs` | repository | CRUD + рекурсивные CTE | `repos/places_sqlite.rs` (CTE, цикл-гард) + `repos/place_movements_sqlite.rs` (`*_in_tx`) | exact |
| `crates/trackly-infra/src/repos/mod.rs` | config | — | сам файл | exact |
| `crates/trackly-infra/src/repos/place_movements_sqlite.rs` (+`batch_id`, `entity_label`, соседний `record_batch_movement_if_applicable`) | repository | event-driven (журнал) | сам файл | exact |
| `crates/trackly-app/src/dto/groups.rs` | model (DTO, specta) | request-response | `dto/place.rs` + `DevicePatch` в `dto/device.rs:182-217` | exact |
| `crates/trackly-app/src/services/group_service.rs` | service | CRUD + batch-транзакция + стартовый засев | `services/place_service.rs` (чтения, writer) + `move_subtree_contents` (одна tx) + `device_service.rs::update` + `supervisor.rs::seed_supervisor_tasks` | exact |
| `crates/trackly-app/src/services/mod.rs`, `dto/mod.rs`, `tauri_cmds/mod.rs`, `http/mod.rs`, `context.rs`, `specta_export.rs` | config (регистрация) | — | те же файлы, строки `places` | exact |
| `crates/trackly-app/src/tauri_cmds/groups.rs` | controller (Tauri) | request-response | `tauri_cmds/places.rs` | exact |
| `crates/trackly-app/src/http/groups.rs` | controller (axum) | request-response | `http/places.rs` | exact |

### Чужие write-site'ы `devices.place_id` (изменяемые, Pattern 5 RESEARCH)

| Файл / место | Роль | Поток данных | Аналог правки | Качество |
|--------------|------|--------------|---------------|----------|
| `services/device_service.rs::update` (стр. 633-780) — guard S1 | service | CRUD | сам метод (`before = repo.get_in_tx`) | exact |
| `services/device_service.rs::delete_soft` (стр. 801) — S8 | service | CRUD | сам метод | exact |
| `services/act_service.rs` (8 вызовов: 596, 948, 1133, 1756, 2285, 2355, 2443, 3645) — release S3-S7 | service | CRUD | `place_movements_repo.record_movement_if_applicable` после UPDATE | role-match |
| `services/place_service.rs::move_subtree_contents` (стр. 686) — S2 | service | batch | сам метод | exact |
| `services/report_service.rs::query_movements_inner` + `movement_reason` (стр. 1402-1620) | service | transform | сам запрос | exact |
| `services/place_movement_service.rs::get_timeline`, `dto/place_movements.rs` | service/DTO | request-response | сами файлы | exact |

### Тесты и гейты (Rust)

| Файл | Роль | Ближайший аналог | Качество |
|------|------|------------------|----------|
| `crates/trackly-infra/tests/groups_migration.rs` | test | `tests/acts_number_text_migration.rs` (`run_up_to`) + `tests/place_movements_migration.rs` | exact |
| `crates/trackly-infra/tests/per_record_invariants.rs` (правка списков таблиц) | test | сам файл, стр. 17-40 | exact |
| `crates/trackly-app/tests/group_write_sites.rs` (счётный гейт + таблица-драйвер S1-S9) | test/gate | `tests/number_space_broadcast_gate.rs` | role-match |
| `crates/trackly-app/tests/groups_*.rs` (сервис, перенос, атомарность) | test | `tests/place_movements_bulk_move.rs` | exact |
| `crates/trackly-app/tests/role_endpoint_matrix.rs` (Cases 76+) | test | сам файл, Case 45/48 | exact |

### UI (Svelte 5 + TS)

| Файл | Роль | Поток данных | Ближайший аналог | Качество |
|------|------|--------------|------------------|----------|
| `ui/src/features/groups/GroupsPage.svelte` | page | request-response | `features/places/PlacesPage.svelte` | exact |
| `features/groups/GroupTree.svelte` | component (дерево) | CRUD + pointer-события | `features/places/PlaceTree.svelte` | exact (копия анатомии, не обобщение) |
| `features/groups/GroupTreeNode.svelte` | component | CRUD | `features/places/PlaceTreeNode.svelte` | exact |
| `features/groups/GroupTypePanel.svelte` | component | CRUD | `features/places/PlaceContents.svelte` (шапка) + `DetailPanel/DetailSection/DetailField` | role-match |
| `features/groups/GroupTypePropertiesTable.svelte` | component | CRUD + drag-reorder | `PlaceTree.svelte:717-846` (pointer-drag) + `PlaceContents.svelte` (`Table`) | role-match |
| `features/groups/GroupPanel.svelte`, `GroupPropertiesForm.svelte` | component | CRUD | `PlaceContents.svelte` + `DeviceFormBody.svelte` | role-match |
| `features/groups/GroupUsersField.svelte` | component | CRUD | `PersonAutocomplete.svelte` / `Dropdown` flat | partial |
| `features/groups/GroupPrintersList.svelte` | component | read | `PlaceContents.svelte` (`Table` + `Badge`) | role-match |
| `features/groups/GroupContentsTable.svelte` | component | CRUD | `PlaceContents.svelte` (столбцы) + `ActFormItemsTable.svelte` (`Dropdown`) + `DeviceGroupRow.svelte` (раскрытие) | role-match |
| `features/groups/GroupAddDevicesModal.svelte` | component (modal) | batch | `PlaceContents.svelte` блок `<Modal>` переноса + `DeviceFilters` | role-match |
| `features/groups/GroupFormModal.svelte`, `GroupTypeFormModal.svelte` | component (modal) | CRUD | `features/places/PlaceFormModal.svelte` | exact |
| `features/groups/GroupMoveModal.svelte` | component (modal) | request-response | `PlaceContents.svelte` блок массового переноса (ближе, чем `PlaceMoveModal`) | exact |
| `features/groups/PropertyRequiredViolatorsPopup.svelte` | component (popup) | read | `lib/components/NumberTakenPopup.svelte` | exact |
| `ui/src/lib/api/groups.ts` | service (клиентская обёртка) | request-response | `lib/api/devices.ts` | exact |
| `ui/src/lib/utils/reorder.ts` | utility (чистые функции) | transform | `lib/utils/pluralize.ts` (стиль) | role-match |
| `routes.ts`, `features/layout/sidebar-config.ts` | config | — | сами файлы | exact |
| `lib/components/Dropdown.svelte` (+`getGroupSection`) | component | — | сам файл | exact |
| `lib/components/TableRow.svelte` (aria-label), `MovementTimeline.svelte`, `features/reports/ReportTable.svelte`, `features/devices/DeviceFormBody.svelte` (D-19), `DeviceFilters.svelte`, `DeviceList.svelte`, `showcase/sections/TableSection.svelte`, `DropdownSection.svelte` | component | — | сами файлы | exact |
| `ui/scripts/check-group-vocabulary.mjs` (+`--selftest`) | gate | — | `scripts/check-device-form-quantity-gate.mjs` | exact |
| `ui/scripts/check-place-tree-invalidation.mjs` (реестр INV-7) | gate | — | сам файл (стр. 716-750) | exact |
| `ui/scripts/check-reorder.mjs` (golden-фикстура `reorder.ts`) | gate | — | `scripts/check-place-path-short.mjs` / `check-placepath-parity.mjs` | role-match |
| `ui/package.json` (`lint`) | config | — | сам файл, стр. 16 | exact |

---

## Назначение паттернов

### `migrations/V045__groups.sql` (migration, DDL)

**Аналог:** `migrations/V041__number_templates.sql` (аддитивные таблицы без перестройки; последняя строка — `PRAGMA user_version = 41;`), шапка-комментарий `V040__place_movements.sql` (объяснение «почему без CHECK»).

**Что копировать:** каждая миграция обязана заканчиваться `PRAGMA user_version = N;`. Токены без SQL `CHECK` (V040:30-44, `entity_type`/`source` — голый `TEXT NOT NULL`, валидация на Rust-стороне через `from_str_lenient`) — для `behavior` и `data_type` тоже без `CHECK` (GRPX-02, Pitfall 9).

```sql
-- V040__place_movements.sql:30-44 — образец колонок-токенов без CHECK
CREATE TABLE place_movements (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type         TEXT    NOT NULL,  -- 'device' | 'cartridge' (D-21...)
  source              TEXT    NOT NULL,  -- 'manual' | 'act' | 'map' | 'workstation' (D-07, no CHECK)
  ...
  created_at_utc      INTEGER NOT NULL
);
```

```sql
-- V041: идиома `ON CONFLICT`-свободного засева внутри миграции НЕ используется для групп —
-- засев встроенных типов идёт СЕРВИСОМ при старте (Pattern 3 RESEARCH), миграция — чистый DDL.
PRAGMA user_version = 41;
```

**Жёсткие правила:**
- Только аддитивно. Урок V042: перестройка таблицы с детьми внутри refinery-транзакции стёрла `act_items` (`PRAGMA foreign_keys = OFF` в файле миграции — no-op). `V043__cartridges_code_unique_live.sql` — пример того, как перестройка выглядит и почему здесь запрещена (`migrations::run` сам отключает FK вне транзакции и сверяет `PRAGMA foreign_key_check`).
- Все `*_at_utc` — `INTEGER` (тест `per_record_invariants::all_timestamp_columns_use_at_utc_suffix_and_integer_type`).
- Полная DDL (5 таблиц + индексы + триггер неизменяемости `code`/`behavior`) — `41-RESEARCH.md` Pattern 1, проверена в `sqlite3`.
- Уникальность имён свойств: частичный индекс по байтам + проверка без учёта регистра в Rust (SQLite `lower()` не сворачивает кириллицу; прецедент — комментарий V043).

---

### `migrations/V046__place_movements_batch.sql` (migration, аддитивный ALTER)

**Аналог:** `migrations/V039__place_path_display.sql` — `ALTER TABLE places ADD COLUMN path_variant_override TEXT NULL;` без перестройки.

```sql
-- V039: форма аддитивной правки
ALTER TABLE places ADD COLUMN path_variant_override TEXT NULL;
...
PRAGMA user_version = 39;
```

**Применить:** `ALTER TABLE place_movements ADD COLUMN batch_id TEXT NULL; ALTER TABLE place_movements ADD COLUMN entity_label TEXT NULL; CREATE INDEX idx_place_movements_batch ON place_movements(batch_id) WHERE batch_id IS NOT NULL; PRAGMA user_version = 46;`. Перестройка `place_movements` запрещена (V040 содержит FK `ON DELETE RESTRICT` на `places` и `SET NULL` на `acts`/`users`).

---

### `crates/trackly-core/src/domain/groups.rs` и `ports/groups.rs`

**Аналог:** `domain/places.rs` (`PlaceRow` стр. 70-94, `PlaceKind` стр. 17-60 с `from_str`/`as_str`, возвращающим `AppError::Validation` по-русски) и `ports/places.rs` (стр. 1-60).

```rust
// domain/places.rs:32-52 — паттерн закрытого набора токенов
impl PlaceKind {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "territory" => Ok(Self::Territory),
            ...
            other => Err(AppError::Validation {
                field: "kind".to_string(),
                message: format!("Неизвестный тип места: «{other}». Допустимые значения: ..."),
            }),
        }
    }
    pub fn as_str(&self) -> &'static str { ... }
}
```

**Применить:** `GroupBehavior` (`container|substitute|teardown`), `PropertyDataType` (`text|number|ip|mac|users|device_refs`) — по образцу `PlaceKind`; домен БЕЗ `Serialize`/`specta` (строковое преобразование делает DTO-слой, как `PlaceKind::as_str()` в `dto/place.rs:52-57`). `GroupRow`/`GroupTypeRow`/`PropertyRow` — плоские структуры с `deleted_at_utc`/`version` (standard4 — только у `group_types`, `group_type_properties`, `groups`).

**Порт:** trait `GroupRepository` с `type Conn` — только для простого CRUD (мутирующие методы — `&mut Self::Conn`, читающие — `&Self::Conn`, `ports/places.rs:9-11`). Составные операции (перенос, состав, значения) — inherent-методы `*_in_tx(&self, tx: &Transaction<'_>, …)` на `SqliteGroupRepository` (см. `SqlitePlaceMovementsRepository::insert_in_tx`), потому что `&mut Transaction` не удовлетворяет `&mut Self::Conn` (нет `DerefMut`; объяснение — шапка `place_service.rs:16-30`).

---

### `crates/trackly-core/src/domain/group_values.rs` (utility, transform)

**Аналог:** `domain/place_movements.rs::is_reportable_place_change` (чистая функция + тесты рядом, стр. 100+) и `domain/places.rs::sibling_cmp`. Модуль обязан проходить `tests/no_io_deps.rs` (никакого I/O).

**Что применить:** `normalize_ip` (`std::net::IpAddr::from_str` + `to_string()`), `normalize_mac` (один и тот же разделитель по всей строке, 12 hex → `aa:bb:cc:dd:ee:ff`), `normalize_number` (запятая → точка, `is_finite`, длина ≤ 32), `normalize_text` (`trim`, ≤ 2000). Ошибки — `AppError::Validation { field: "values.<property_id>", message: <по-русски> }`. Эскизы — Code Examples в `41-RESEARCH.md` (стр. ~746-775). Тесты — табличные, рядом в `#[cfg(test)] mod tests`, как в `place_movements.rs:109-187`.

---

### `crates/trackly-core/src/auth.rs` (+ `ManageGroupTypes`, `MutateGroups`, `ReadGroups`)

**Аналог:** сам файл, стр. 94-171. `match` в `authorize` исчерпывающий — вариант без ветки не скомпилируется.

```rust
// auth.rs:155-173 — куда добавлять ветки
Action::ManageUsers | Action::ManageSettings | Action::MutatePlaces => {
    matches!(identity.role, Role::Admin)
}
Action::MutateDevices | Action::MutateActs | ... | Action::ReadPlaces => {
    matches!(identity.role, Role::Admin | Role::Manager)
}
```

**Применить:** `ManageGroupTypes` — в Admin-only бакет (рядом с `MutatePlaces`); `MutateGroups` и `ReadGroups` — в Admin|Manager бакет. НЕ использовать `MutatePlaces` (он Admin-only — замаскирует ошибку «менеджер не создаёт группу»). Обновить таблицу прав в доккомментарии (стр. 133-147) и добавить unit-тесты в стиле `authorize_manager_mutate_places_forbidden` (стр. 389-426): admin ok, manager по таблице, employee → `Forbidden`.

---

### `crates/trackly-core/src/domain/place_movements.rs` (+ `Group`)

**Аналог:** сам файл: `MovementSource` (стр. 19-52), `MovementEntityKind` (стр. 59-90).

**Применить:** `MovementEntityKind::Group` (`as_str "group"`, `label_ru "Группа"`, `from_str_lenient`), `MovementSource::Group` (`"group"`). `Workstation` НЕ удалять (CONTEXT: оставить мёртвым). Существующий тест `…rejects_printer` остаётся. Без этого чтение деградирует в «причина не определена»/сырой `group` (расхождение №10 RESEARCH).

---

### `crates/trackly-infra/src/repos/groups_sqlite.rs` (repository, CRUD + CTE)

**Аналог:** `repos/places_sqlite.rs` (рекурсивные CTE, цикл-гард, `map_rusqlite`) и `repos/place_movements_sqlite.rs` (нулевое поле структуры, `*_in_tx`).

**Заголовок/импорты** (`places_sqlite.rs:1-30`):
```rust
use rusqlite::{Connection, OptionalExtension};
use trackly_core::error::AppError;
use crate::error_conversions::map_rusqlite;

/// SQLite-backed place repository adapter (zero-sized, mirrors `SqliteDeviceRepository`).
#[derive(Debug, Default, Clone)]
pub struct SqlitePlaceRepository;
```
Все SQL — параметризованные `rusqlite::params![...]` (T-39-04-01), без конкатенации.

**Защита от циклов вложенности групп** — перенести дословно, заменив `places`/`parent_id` на `groups`/`parent_group_id` (`places_sqlite.rs:479-520`), в той же транзакции ПЕРЕД `UPDATE`:
```rust
let is_cycle: i64 = tx.query_row(
    "WITH RECURSIVE ancestors(id) AS (
        SELECT parent_id FROM places WHERE id = ?1
        UNION ALL
        SELECT p.parent_id FROM places p
        JOIN ancestors a ON p.id = a.id
        WHERE p.parent_id IS NOT NULL
     )
     SELECT EXISTS(
       SELECT 1 WHERE ?1 = ?2
       UNION ALL
       SELECT 1 FROM ancestors WHERE id = ?2
     )",
    rusqlite::params![np, id], |r| r.get(0),
).map_err(map_rusqlite)?;
if is_cycle != 0 {
    return Err(AppError::Validation { field: "parent_id".to_string(), message: "Нельзя ...".to_string() });
}
```
Для групп `field: "parent_group_id"`. Дополнительно: родитель с `behavior = 'teardown'` отклоняется (SPEC req.8).

**Состав с вложенными** — форма `subtree_stats_impl` (`places_sqlite.rs:183-190`):
```rust
"WITH RECURSIVE subtree(id) AS (
    SELECT id FROM places WHERE id = ?1 AND deleted_at_utc IS NULL
    UNION ALL
    SELECT p.id FROM places p JOIN subtree s ON p.parent_id = s.id
    WHERE p.deleted_at_utc IS NULL
 ) SELECT ... WHERE place_id IN (SELECT id FROM subtree) AND deleted_at_utc IS NULL"
```
Для групп: CTE `sub(id)` вниз по `groups.parent_group_id`, затем `group_devices gd JOIN sub` + `JOIN devices d ... d.deleted_at_utc IS NULL` (Pattern 4 RESEARCH).

**CAS по `version`** — как `move_node`:
```rust
"UPDATE places SET parent_id = ?1, updated_at_utc = ?2, version = version + 1 \
 WHERE id = ?3 AND version = ?4 AND deleted_at_utc IS NULL"
if affected == 0 { return Err(resolve_cas_failure(&tx, id, version)); }
```
`resolve_cas_failure` (`places_sqlite.rs:127`) — вернуть `NotFound` или `OptimisticLockMismatch`; скопировать идиому.

**`*_in_tx` для составных операций** (`place_movements_sqlite.rs:76-108`): метод принимает `tx: &Transaction<'_>`, НЕ открывает свою транзакцию («движение должно лечь в ту же транзакцию, что и мутация»).

**Принтеры состава (Pattern 9):** USB-принтеры — `printers.usb_host_device_id IN (device_id состава)`; явные — `value_ref` у свойств `device_refs`; дедуп по `printers.device_id` на сервере, `origin='usb'` побеждает.

---

### `repos/place_movements_sqlite.rs` (расширение журнала)

**Аналог:** сам файл. Единая точка записи: `record_movement_if_applicable` (стр. 128-183) + `NewMovement` (стр. 30-47) + `get_history` (стр. 205-237).

**Что менять:**
1. В `NewMovement` добавить `batch_id: Option<&'a str>`, `entity_label: Option<&'a str>`; в `insert_in_tx` расширить `INSERT` (стр. 80-100) до 14 колонок; в `get_history`/`MovementRow` — читать `batch_id`, `entity_label`.
2. Добавить СОСЕДНИЙ метод `record_batch_movement_if_applicable` + общий приватный хелпер. НЕ менять сигнатуру `record_movement_if_applicable` (11 позиционных аргументов, 7 существующих write-site'ов — класс ошибки «5 копий, одна забыла»).

```rust
// place_movements_sqlite.rs:128-145 — сигнатура, которую НЕЛЬЗЯ ломать
#[allow(clippy::too_many_arguments)]
pub fn record_movement_if_applicable(
    &self, tx: &Transaction<'_>,
    places_repo: &dyn PlaceRepository<Conn = Connection>,
    entity_type: MovementEntityKind, entity_id: i64,
    before_place_id: Option<i64>, after_place_id: Option<i64>,
    source: MovementSource, note: Option<&str>, act_id: Option<i64>,
    user_id: Option<i64>, now_utc: i64,
) -> Result<(), AppError> {
    if !is_reportable_place_change(before_place_id, after_place_id) { return Ok(()); }
    ...
```
Помнить: `is_reportable_place_change` пропускает `NULL → место` и `место → NULL` (D-06) — строка группы в журнале появляется только при переносе между двумя реальными местами; событие «устройство без места попало в группу с местом» пишется только в `audit_log` (D-30).

---

### `crates/trackly-app/src/dto/groups.rs` (DTO)

**Аналог:** `dto/place.rs` (стр. 1-65) и `DevicePatch` в `dto/device.rs:196-217`.

**Правила:**
```rust
// dto/place.rs:19-35 — snake_case JSON, specta, i64 → i32 на проводе
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct PlaceDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = Option<i32>)]
    pub parent_id: Option<i64>,
    pub kind: String,           // токен через as_str().to_string()
    ...
}
impl From<PlaceRow> for PlaceDto { ... }
```
```rust
// dto/device.rs:202-214 — двойной Option для «не передано» vs «очистить»
#[serde(default, with = "serde_with::rust::double_option")]
#[specta(type = Option<Option<i32>>)]
pub place_id: Option<Option<i64>>,
```
НИКАКИХ `rename_all = "camelCase"` в DTO (PATTERNS Phase 39 §Pattern 3; camelCase — только у HTTP-payload структур в `http/*.rs`). `group_types_update` DTO должен ВКЛЮЧАТЬ опциональные `code`/`behavior`, отклоняемые при отличии (иначе HTTP-приёмка «изменение `code` отклоняется» вакуумна — serde молча проигнорирует неизвестное поле). Ответы `groups_move`/`groups_add_devices`/`groups_set_parent` обязаны нести `changed_place_ids: Vec<i64>` (по образцу `ActDto::changed_place_ids`) — по ним UI вызывает `notifyPlaceContentChanged`.

---

### `crates/trackly-app/src/services/group_service.rs` (service)

**Аналоги (брать из разных мест):**

**1. Каркас и чтения** — `place_service.rs:36-95` и `:533-548`:
```rust
#[derive(Clone)]
pub struct PlaceService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) repo: Arc<SqlitePlaceRepository>,
    pub(crate) audit_repo: Arc<SqliteAuditLogRepository>,
}
pub fn new(writer, readers, clock) -> Self { ... }   // тот же трёхаргументный вид

pub async fn get(&self, caller: &Identity, id: i64) -> Result<PlaceRow, AppError> {
    authorize(caller, &Action::ReadPlaces)?;          // ПЕРВАЯ строка
    let readers = self.readers.clone();
    let repo = self.repo.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        repo.get(&conn, id)
    })
    .await
    .map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking: {e}") })?
}
```
Чтения — reader pool через `spawn_blocking`, никогда не через writer.

**2. Единая транзакция переноса группы — копировать `move_subtree_contents`, а НЕ идиом `create/rename`** (`place_service.rs:686-790`):
```rust
authorize(caller, &Action::MutateDevices)?;
let now = self.clock.unix_seconds();
let user_id = caller.user_id;           // вынуть ДО замыкания: Identity не Send через границу
self.writer.execute(move |conn| {
    let tx = conn.transaction().map_err(map_rusqlite)?;
    ... // цикл: devices_repo.get_in_tx → update_status_and_place_in_tx → record_movement_if_applicable
    tx.commit().map_err(map_rusqlite)?;
    Ok(moved)
}).await
```
Идиом `PlaceService::create` (repo-вызов в автокоммите на `conn`, потом короткая `conn.transaction()` ради `audit_log`, стр. 153-200) для групп НЕПРИГОДЕН: сбой между ними оставит мутацию без аудита или частично перенесённую группу. Для групп мутация, `place_movements` и `audit_log` — в ОДНОЙ `tx`.

**3. Аудит внутри той же tx** — `device_service.rs:722-728`:
```rust
tx.execute(
    "INSERT INTO audit_log \
     (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
     VALUES ('device', ?1, 'update', ?2, ?3, ?4, NULL, ?5)",
    rusqlite::params![id, user_id_opt, before_json, after_json, now],
).map_err(map_rusqlite)?;
```
(или `audit_repo.insert(&tx, AuditEntry { entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc })` — `place_service.rs:177-189`).

**4. Каскад картриджей принтера-члена** (Pitfall 3 RESEARCH) — `device_service.rs:757-768` + сигнатура `cartridges_sqlite.rs:1182-1190`:
```rust
if after.place_id.is_some() && before_place_id != after.place_id {
    cartridge_repo.cascade_place_for_printer_in_tx(
        &tx, id, after.place_id, MovementSource::Manual, "вместе с принтером", user_id_opt, now,
    )?;
}
```
При переносе группы вызывать с `MovementSource::Group` и тем же `batch_id`/подписью пакета; `debug_assert!(new_place_id.is_some())` — гейтить `Some`.

**5. Идемпотентный засев встроенных типов** — `supervisor.rs:331-360` (`INSERT OR IGNORE`) и `context.rs` (вызов сразу после создания writer'а, рядом с `seed_supervisor_tasks`, стр. ~293; `templates.seed_defaults_on_startup().await?` стр. ~239):
```rust
pub async fn seed_supervisor_tasks(writer: &Arc<WriterHandle>, now: i64) -> Result<(), AppError> {
    writer.execute(move |conn| {
        conn.execute("INSERT OR IGNORE INTO scheduled_tasks (name, status, next_run_at_utc) VALUES ('db_backup', 'idle', NULL)", [])
            .map_err(map_rusqlite)?;
        Ok(())
    }).await
}
```
Для типов — `INSERT … ON CONFLICT(code) DO NOTHING` (НЕ `DO UPDATE` и НЕ `INSERT OR REPLACE`: переименование типа админом должно пережить рестарт, Pitfall 11), все три вставки в одном замыкании writer'а. Засеваемые `code`: `workstation` (container), `system_unit` (substitute), `teardown` (teardown). Метод `seed_builtin_types_on_startup()`.

**6. Валидация и дружелюбные ошибки уникальности** — `place_service.rs:97-140`: `validate_name` → `AppError::Validation { field, message }`; перевод сырого `AppError::Conflict { reason }` по имени индекса в русское сообщение (`is_duplicate_name_conflict`). Для групп — аналогично для `idx_gtp_name_live`, `idx_groups_type_seq`, PK `group_devices.device_id` («устройство уже в группе «…»»).

**7. Поиск по кириллице** — `place_service.rs:802-855`: НИКОГДА `LIKE` для пользовательских подстрок — достать кандидатов и фильтровать `to_lowercase().contains(...)` в Rust, с ограничением длины запроса (`SEARCH_QUERY_MAX_CHARS`, стр. 56) и лимитом результата. Применять для `groups_search` и `groups_user_options(query)` (возвращать только `id`, `full_name`, `login`; НЕ `users_list`, он `Action::ManageUsers`).

**8. Склонение** — `ru_plural` приватна в `place_service.rs:859-870`; вынести в общее место (напр. `services/mod.rs` или утилиту) для D-24 («Перенесено: группа и 6 устройств»).

**Guard S1 (`DeviceService::update`)** — вставить внутрь writer-замыкания ПОСЛЕ `let before = repo.get_in_tx(&tx, id).ok();` (device_service.rs:~707) и ДО `repo.update_in_tx(...)` (стр. ~715): сравнить `patch.place_id` с `before.place_id`; отклонять только РЕАЛЬНУЮ смену (форма шлёт `place_id` в каждом сохранении — `DeviceFormBody.svelte:615/684`, расхождение №8) и только если у прямой группы устройства `place_id IS NOT NULL` (D-21: «запрет спит»):
```rust
AppError::Validation {
    field: "place_id".into(),
    message: "Место задаётся группой «…». Выведите устройство из состава, чтобы переместить его отдельно.".into(),
}
```
**S8 (`delete_soft`, стр. 801-835)** — в той же `tx` после `delete_soft_in_tx` удалить строку `group_devices` (Pitfall 12).

**S3-S7 (акты):** после каждого из 8 вызовов (`act_service.rs` строки 596, 948, 1133, 1756, 2285, 2355, 2443, 3645) — явный вызов helper'а `release_device_in_tx` (удалить строку `group_devices` + `audit_log` `action: "custom:group_member_released"`, payload `{group_id, act_id}`). Для restore-путей (1133, 2285, 3645) — release, только если устройство сейчас член группы с местом. НЕ прятать логику в `update_status_and_place_in_tx`/`update_full_in_tx` (их используют акты и массовый перенос с РАЗНЫМ желаемым поведением).

**S2 (`move_subtree_contents`):** в цикле (стр. ~710-785) для устройств, чья корневая группа имеет место, — не двигать устройство, а вызвать `move_group_in_tx` для корневой группы (дедуп по группе), устройства группы из цикла исключить (иначе двойные строки журнала, Pitfall 2). Возвращаемый `usize` не менять.

---

### `crates/trackly-app/src/tauri_cmds/groups.rs` (контроллер Tauri)

**Аналог:** `tauri_cmds/places.rs` — вся структура: шапка, `build_*`, `#[tauri::command]`-обёртки.

```rust
// tauri_cmds/places.rs:28-43 — build_*: authorize (защита в глубину) → сервис
pub async fn build_places_create(ctx: &AppCtx, caller: &Identity, new: PlaceNewDto) -> Result<PlaceDto, AppError> {
    authorize(caller, &Action::MutatePlaces)?;
    let domain_new = new.into_domain()?;
    ctx.places.create(caller, domain_new).await
}

// tauri_cmds/places.rs:~190-200 — тонкая обёртка; specta ПОСЛЕ tauri::command
#[tauri::command]
#[specta::specta]
pub async fn places_rename(
    state: tauri::State<'_, AppCtx>,
    id: i32, name: String, version: i32,          // id/version — i32 (specta запрещает BigInt)
) -> Result<PlaceDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_places_rename(state.inner(), &caller, id as i64, name, version as i64).await
}
```
**Применить:** ~24 команды по таблице Pattern 6 RESEARCH. Гейт: `group_types_*`/`group_type_properties_*` → `Action::ManageGroupTypes`; `groups_create/update/delete/set_parent/add_devices/remove_devices/move/set_values` → `Action::MutateGroups`; чтения (`group_types_list`, `groups_list/get/composition/search/for_devices/user_options`, `group_type_properties_empty_groups`) → `Action::ReadGroups`. `build_*` и сервис оба вызывают `authorize` (защита в глубину — шапка `places.rs:11-21`). Регистрация каждой команды — в `specta_export.rs` рядом со стр. 188-201.

---

### `crates/trackly-app/src/http/groups.rs` (контроллер axum)

**Аналог:** `http/places.rs`. HTTP-хендлер — тонкий адаптер, делегирует в ТЕ ЖЕ `build_*` из `tauri_cmds`.

```rust
// http/places.rs:14-30 — импорты
use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;
use crate::context::AppCtx;
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;
use crate::tauri_cmds::places::{build_places_archive, ...};

// payload — camelCase
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamePayload { pub id: i64, pub name: String, pub version: i64 }

// http/places.rs:145-158 — хендлер
pub async fn handler_rename(State(ctx): State<AppCtx>, session: Session, Json(payload): Json<RenamePayload>)
    -> Result<Json<PlaceDto>, AppErrorResponse>
{
    let identity = session_identity(&session).await.map_err(AppErrorResponse::from)?;
    Ok(Json(build_places_rename(&ctx, &identity, payload.id, payload.name, payload.version)
        .await.map_err(AppErrorResponse::from)?))
}

// http/places.rs:362-380 — маршруты: POST /api/v1/<имя_команды>
pub fn router() -> Router<AppCtx> {
    Router::new().route("/api/v1/places_create", post(handler_create)) ...
}
```
Подключить: `http/mod.rs:160` — `.merge(groups::router())`; `pub mod groups;` в `http/mod.rs:20`. Коды ошибок: `Validation` → 400, `Conflict` → 409, `Forbidden` → 403 (в проекте нет 422, `error_axum.rs:38` — расхождение №6).

---

### Регистрация (`context.rs`, `services/mod.rs`, `dto/mod.rs`, `specta_export.rs`)

**Аналог:** строки `places` в тех же файлах.
- `context.rs:117` — поле `pub places: Arc<PlaceService>`; `context.rs:324-328` — построение:
```rust
let places = Arc::new(PlaceService::new(writer.clone(), readers.clone(), clock.clone()));
```
и перечисление в конструкторе `AppCtx { ..., places, place_movements, number_templates }` (стр. ~440-463). Добавить `groups: Arc<GroupService>` и вызов `groups.seed_builtin_types_on_startup().await?` после засева шаблонов/задач.
- `services/mod.rs:13-30`: `pub mod group_service;` (+ `pub use`), `dto/mod.rs:9-21`: `pub mod groups;`, `tauri_cmds/mod.rs:9-25`: `pub mod groups;`.
- После добавления DTO/команд пересоздать `ui/src/bindings.ts` (`cargo test -p trackly-app --test export_bindings`; файл в `.gitignore`).

---

### Отчёт «Перемещения», таймлайн, DTO (`report_service.rs`, `place_movement_service.rs`, `dto/place_movements.rs`)

**Аналог:** сами файлы; список мест, где группа ломает отчёт без правок (Pattern 10 п.5 RESEARCH):
```sql
-- report_service.rs:1520-1523 — три места правки
COALESCE(d.name, c.code) AS device_name,                    -- → COALESCE(d.name, c.code, pm.entity_label)
CASE pm.entity_type WHEN 'device' THEN 'Устройство'
                    WHEN 'cartridge' THEN 'Картридж'
                    ELSE pm.entity_type END AS entity_type_label,   -- + WHEN 'group' THEN 'Группа'
```
`movement_reason` (стр. 1402-1427) — добавить ветку `Some(MovementSource::Group) => …` («в составе группы «{label}»»/«перенос группы»); для неизвестных токенов остаётся сырой `source.to_string()`.
`LIMIT 1000` может разрезать пакет → счётчик «(N устройств)» считать подзапросом по `batch_id`. В `ReportRow` добавить опциональные `batch_id`, `batch_role` (`header|member|null`), `batch_size`; НЕ менять `columns_for("movements")` (тянет `check-report-type-parity.mjs`, CSV, `COLUMNS_MAP`) и НЕ менять `templates/report.html` (механизм `_legacy_defaults/vNN` + «strict-undefined» предпросмотр — две известные ловушки проекта). Печать (D-27) — полные плоские строки из `ORDER BY pm.created_at_utc ASC, pm.id ASC`; сворачивание — только в `ReportTable.svelte` на экране.

`dto/place_movements.rs` — `MovementEntryDto`: добавить `batch_id`, `group_id`, `group_label` (из `entity_label`). `place_movements_get_timeline(entity_type="group", id)` работает без правок сервиса (репозиторий не валидирует токен — `place_movement_service.rs:49-79`).

---

### `crates/trackly-infra/tests/groups_migration.rs`

**Аналог:** `tests/acts_number_text_migration.rs:1-70` (`run_up_to` + посев + `run`) и `tests/place_movements_migration.rs:1-60`.
```rust
fn conn_at_v041() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("acts-number-text-migration.db");
    let mut conn = Connection::open(&path).expect("open");
    apply_writer_pragmas(&conn).expect("writer pragmas");
    let report = migrations::run_up_to(&mut conn, 41).expect("run migrations up to V041");
    assert_eq!(report.schema_version, 41, "fixture must be a real V041 DB");
    (conn, dir)
}
```
**Применить:** `run_up_to(conn, 44)` → засеять `places`/`devices`/`place_movements`/`act_items` (вымышленные: `Склад А`, `Ноутбук`) → `migrations::run` → проверить: id и количество строк `place_movements`/`act_items` не изменились, `PRAGMA foreign_key_check` пуст, в `sqlite_master` нет `place_movements_new`, есть колонки `batch_id`/`entity_label`, `max_known_version() == 46`. Плюс повторный прогон идемпотентен (`place_movements_migration.rs:75-95`).

**`per_record_invariants.rs`** (стр. 17-40): `group_types`, `group_type_properties`, `groups` → `USER_MUTABLE_TABLES`; `group_devices`, `group_property_values` → `SYSTEM_TABLES` (junction-инвариант: нет `version`/`deleted_at_utc`).

---

### `crates/trackly-app/tests/group_write_sites.rs` (счётный гейт + таблица-драйвер)

**Аналог:** `tests/number_space_broadcast_gate.rs:1-120` — реестровый гейт: фикстура `fixture()` на `test_writer_and_readers()`, по кейсу на каждую серверную мутацию, в шапке — комментарий «добавь кейс при новом write-site».
```rust
// number_space_broadcast_gate.rs:36-52
fn fixture() -> Fixture {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    ...
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn every_number_space_mutation_broadcasts() {
    tokio::time::timeout(Duration::from_secs(60), async { ... }).await ...
}
```
**Применить:** (а) поведенческий тест на каждую строку S1-S9; (б) счётный гейт на исходник: `include_str!("../src/services/act_service.rs")` → `matches("update_status_and_place_in_tx(").count() + matches("update_full_in_tx(").count() + matches("restore_from_snapshot_in_tx(").count() == 8` с сообщением «новое место записи devices.place_id в act_service.rs — добавь кейс release»; расширить на весь `crates/*/src`: вызовы допустимы только в `act_service.rs` (8), `place_service.rs` (1 — `move_subtree_contents`) и определениях в `devices_sqlite.rs`. **Якорь мутационной проверки — уникальный** (память `mutation_test_anchor_must_be_unique`): строка вызова `release_device_in_tx(` с аргументами конкретного сайта, не общее имя метода.

**Сервисные тесты групп** — по образцу `tests/place_movements_bulk_move.rs`: полный `AppCtx` через `make_test_ctx()` (стр. 46-57), `create_identity(ctx, login, full_name, role)` (стр. 61-80), `seed_place` через `ctx.writer.execute` raw SQL (стр. 83-97), атомарность — триггер `BEFORE UPDATE`, роняющий последний элемент (стр. 14-19 шапки: «атомарность доказана fault-injection, не комментарием»). Фикстура атомарности переноса группы (R9) — тот же приём.

**Матрица прав** (`role_endpoint_matrix.rs`, Cases 76+): образцы — Case 45 (HTTP, Manager → 403 на мутациях места, стр. 1740-1810) и Case 48 (Tauri path — прямой вызов `build_places_*` → `Err(AppError::Forbidden)`, стр. 1941-2021). Хелперы: `make_test_ctx()`, `create_session_cookie(&store, user_id, role)`, `post_with_cookie(app, uri, body, Some(&cookie))` (стр. 205-290). Таблица-драйвер `(команда, GateKind::{TypeMut, GroupMut, Read}, payload)` × 3 роли × 2 транспорта + тест полноты: `include_str!` исходника `http/groups.rs`, регэксп `\.route\("/api/v1/([a-z_]+)"` → множество, сравнить с таблицей (новый маршрут без строки — красный). Тест HTTP на `Validation` ждёт 400, `Conflict` — 409, `Forbidden` — 403. Не забыть тест «изменение `code`/`behavior` по HTTP отклоняется» (Pitfall 10 — не вакуумный: DTO несёт эти поля).

---

### `ui/src/features/groups/GroupsPage.svelte`

**Аналог:** `features/places/PlacesPage.svelte` (1-287).

**Каркас** (`PlacesPage.svelte:200-250`, стили стр. 264-285):
```svelte
<div class="places-page">
  <PageHeader title="Места">
    {#snippet actions()}
      {#if isAdmin}<Button variant="primary" onclick={...}>Создать место</Button>{/if}
    {/snippet}
  </PageHeader>
  <div class="page-content">
    <PlacesMasterDetail>
      {#snippet master()}<PlaceTree {initialSelectedId} onSelect={handleTreeSelect} {refreshToken} .../>{/snippet}
      {#snippet detail()}
        {#if selectedPlace}{#key `${selectedPlace.id}:${contentsResetToken}`}<PlaceContents .../>{/key}
        {:else}<DetailPanel empty={true} emptyTitle="Место не выбрано" emptyBody="..."/>{/if}
      {/snippet}
    </PlacesMasterDetail>
  </div>
</div>
```
```scss
.places-page { display:flex; flex-direction:column; height:100%; min-height:0; }
.page-content { flex:1 1 auto; display:flex; flex-direction:column; min-height:0; overflow:hidden;
                padding: var(--tr-space-lg) var(--tr-space-xl); }
```
**Применить:** `PlacesMasterDetail` — импортировать как есть (запрет пересчёта 39-UI-SPEC §3), не копировать. Оболочка `.content` не скроллится; каждая `*-page` — `height:100%; min-height:0`, прокрутка во внутренней области (память `app_shell_scroll_pattern`).

**Состояние выбора (стр. 53-130):** `localStorage` с префиксом `trackly:groups:*` по образцу `trackly:places:selectedId`/`onlyHere`/`activeTab`, `try/catch` вокруг каждого доступа; URL-хеш `#/groups?id=…` побеждает сохранённое (`parseIdFromHash`, стр. 41-51; чтение один раз при монтировании). Запись хеша — `history.replaceState(null, '', newHash)` без `hashchange`. Значения вкладки/тумблеров — поднимать в страницу, чтобы пережить `{#key}`-ремонтаж (GAP-10).

---

### `GroupTree.svelte` + `GroupTreeNode.svelte`

**Аналог:** `features/places/PlaceTree.svelte` (1219 строк) и `PlaceTreeNode.svelte`. `PlaceTree` НЕ обобщается (завязан на `PlaceDto`, `places_search`, drag-to-move, архив) — копировать анатомию.

**ARIA/клавиатура** (`PlaceTree.svelte:207-221`, `:495-550`, `:886-895`):
```ts
const visibleNodes = $derived.by<VisibleRow[]>(() => {   // плоский список для клавиатуры
  const rows: VisibleRow[] = [];
  function walk(list, depth, parentId) {
    for (const p of list) { rows.push({ place: p, depth, parentId });
      if (expandedIds.includes(p.id)) walk(childrenMap.get(p.id) ?? [], depth + 1, p.id); }
  }
  walk(roots, 0, null); return rows;
});
```
```svelte
<div class="tree-body" role="tree" aria-label="Дерево мест" tabindex="-1" onkeydown={handleContainerKeydown} ...>
...
<div class="sr-only" aria-live="polite">{liveMessage}</div>
```
Клавиши: ArrowDown/Up, Home/End, ArrowRight (раскрыть/к первому ребёнку), ArrowLeft (свернуть/к родителю), Enter, F2. Roving tabindex (`tabindex={isFocused ? 0 : -1}`).

**Строка узла** (`PlaceTreeNode.svelte:107-215`): `role="treeitem"`, `aria-level={depth+1}`, `aria-selected`, `aria-expanded={hasChildren ? expanded : undefined}`, `data-place-id`, отступ `padding-left: calc(var(--tr-space-xs) + ${depth} * var(--tr-space-md))`, chevron-слот, имя, `Badge appearance="count"` для счётчика, `ActionMenu variant="ghost-sm" label={`Действия: ${node.name}`}` с `role="menuitem"`-кнопками (`menu-danger` для «Удалить»). Для групп — два вида узла: «тип» (корень) и «группа»; меню узла группы: «Переименовать», «Перенести…» (D-18, общий обработчик с кнопкой панели), «Удалить».

**Инвалидация счётчиков** (`PlaceTree.svelte:330-395`) — скопировать ОБА эффекта вместе с гейтами сходимости. Ленивый догруз сходится ТОЛЬКО благодаря `statsCache[id] !== undefined`; эффект инвалидации читает кэш в `untrack` и пишет условно, иначе `effect_update_depth_exceeded`:
```ts
$effect(() => {
  for (const row of visibleNodes) {
    const id = row.place.id;
    if (statsCache[id] !== undefined || statsInFlight.has(id)) continue;   // НЕ убирать
    statsInFlight.add(id);
    apiCall<...>('places_subtree_stats', { rootId: id }).then(...).finally(() => statsInFlight.delete(id));
  }
});
$effect(() => {
  if (placeContentEventsStore.seq === 0) return;
  ...
  untrack(() => { ... if (changed) statsCache = next; });   // условная запись
});
```
Для групп счётчик — число устройств состава; подписка на `placeContentEventsStore` нужна, чтобы «Перенести»/добавление/вывод обновляли и дерево «Места».

---

### `GroupTypePropertiesTable.svelte` — перестановка на pointer-events (D-10)

**Аналог:** `PlaceTree.svelte:717-846` (механизм прошёл живой UAT фазы 39 на Tauri и в LAN-браузере). НЕ использовать HTML5 DnD (в WKWebView не срабатывает drop).

```ts
// PlaceTree.svelte:743-750, 790-840 — структура обработчиков для копирования
const DRAG_START_THRESHOLD_PX = 6;
let pointerDragOriginId: number | null = null; let pointerDragPointerId: number | null = null;
let pointerDragStartX = 0, pointerDragStartY = 0; let pointerDragStarted = false;

function handleTreePointerDown(e: PointerEvent) {
  if (e.pointerType === 'mouse' && e.button !== 0) return;
  if ((e.target as HTMLElement).closest('.chevron, .row-actions')) return;   // не стартовать с интерактива
  pointerDragOriginId = id; pointerDragPointerId = e.pointerId; ...; pointerDragStarted = false;
}
function handleTreePointerMove(e: PointerEvent) {
  if (pointerDragOriginId === null || e.pointerId !== pointerDragPointerId) return;
  if (!pointerDragStarted) {
    if (Math.hypot(e.clientX - pointerDragStartX, e.clientY - pointerDragStartY) < DRAG_START_THRESHOLD_PX) return;
    pointerDragStarted = true; dragGhost = { label, x: e.clientX, y: e.clientY };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  e.preventDefault();
  if (dragGhost) dragGhost = { ...dragGhost, x: e.clientX, y: e.clientY };
  // hit-test
}
function handleTreePointerUp(e) { ... try { releasePointerCapture } catch {} resetPointerDrag(); if (!started) return; ... }
function handleTreePointerCancel(e) { ... resetPointerDrag(); }
```
Призрак: `position:fixed; pointer-events:none` (иначе перехватывает `elementFromPoint`, GAP-11), стили `.drag-ghost` — `PlaceTree.svelte:1197-1218` (`--tr-surface-raised`, `--tr-elev-3`, `z-index:1000`).

**Что нового:** вместо `elementFromPoint` — индекс вставки по серединам прямоугольников строк (стабильнее над `<tr>` и призраком) — чистые функции в `lib/utils/reorder.ts`.

**Правила (RESEARCH Pattern 11):** драг только за явную ручку (`button`, `aria-label="Переместить свойство"`, `touch-action:none`); обработчики на `<tbody>`, `setPointerCapture` на него; во время драга `user-select:none` через класс; **клавиатурный путь — основной**: пункты «Выше»/«Ниже» в `ActionMenu` строки + `aria-live`; сохранение — один `group_type_properties_reorder(type_id, ordered_ids)` с оптимистичным обновлением и ОТКАТОМ при ошибке (в `PlaceTree` отката нет — не копировать эту часть).

---

### `ui/src/lib/utils/reorder.ts` (чистые функции)

**Аналог:** `lib/utils/pluralize.ts` — один экспорт-функция с JSDoc, без DOM и рун. Готовый код — `41-RESEARCH.md` Pattern 11 (`insertionIndex(rects, pointerY)` и `reorder(items, from, to)`; `to` — индекс вставки в ИСХОДНОМ массиве, после удаления сдвигается). Проверяется node-гейтом на golden-фикстуре (память `js_rust_mirror_needs_fixture_gate`): `ui/scripts/check-reorder.mjs` по образцу `check-place-path-short.mjs`; подключить в `pnpm lint`. Отдельного JS-тест-раннера в проекте нет (vitest/jest отсутствуют) — не вводить.

---

### `GroupContentsTable.svelte` (состав группы, D-01…D-07)

**Аналоги:** `PlaceContents.svelte` (столбцы, пустое состояние, массовые действия), `ActFormItemsTable.svelte` (`Dropdown` combobox + поиск), `DeviceGroupRow.svelte` и `TableRow group` (раскрываемая строка).

**Таблица и пустое состояние** (`PlaceContents.svelte:200-262`) — столбцы копируются один в один (D-05): «Тип» (на «Все»), «Название», «Инв. № / Серийный №», «Место» (при `!onlyHere`), «Статус»:
```svelte
<Table columns={columnCount} fillHeight framed={false} loading={loading && rows === null}
       empty={isWholeNodeEmpty} emptyTitle="..." emptyBody="...">
  {#snippet head()}<th>Название</th><th>Инв. № / Серийный №</th>...{/snippet}
  {#each filteredRows as row (`${row.kind}-${row.id}`)}
    <TableRow class="place-content-row">
      <td class="cell" title={row.name} role="button" tabindex="0" onclick={() => openView(row)} onkeydown={...}>{row.name}</td>
      <td class="cell"><span class="tr-mono">{row.inventory_or_code ?? '—'}</span></td>
      <td class="cell">{#if row.status_name}<Badge variant={statusVariant(row.status_name)}>{row.status_name}</Badge>{:else}—{/if}</td>
    </TableRow>
  {/each}
</Table>
```
D-06: `Placeholder.svelte` НЕ подходит (заглушка «раздел в разработке», расхождение №2). Пустой состав — `<Table empty emptyTitle="Состав пуст" emptyBody="Добавьте устройства поиском ниже">`. Фокус таблицы — только inset-кольцо на первой ячейке (память `table_focus_ring_decision`), клавиатурная точка входа — `<td role="button" tabindex="0">` первой ячейки.

**Раскрываемая строка вложенной группы (D-03):** `TableRow group groupExpanded groupName groupColspan onToggleGroup` (`TableRow.svelte:7-60`); aria-label шеврона заменить на нейтральные «Свернуть»/«Развернуть» (стр. 61). Паттерн загрузки детей при раскрытии — `DeviceGroupRow.svelte` (ленивая загрузка, `loadingChildren`, `pushToast` при ошибке).

**Строка добавления (D-01/D-04)** — `Dropdown variant="combobox"` с поиском по устройствам и группам (`ActFormItemsTable.svelte:409-430`):
```svelte
<Dropdown variant="combobox" value={row.query} placeholder="..." loading={!!loadingByRow[idx]}
  groups={suggestionsByRow[idx] ?? []}
  getGroupId={(g: DeviceGroup) => g.repr.id} getGroupName={(g: DeviceGroup) => g.repr.name}
  getGroupMeta={...} getGroupSub={groupSub} getGroupCount={(g: DeviceGroup) => g.count}
  isGroupExpandable={isExpandable} onExpandGroup={(g) => expandGroup(idx, g)}
  getMemberId={(m: MemberRow) => m.key} getMemberName={memberName} ...
  onSearch={(query) => void fetchGroups(idx, query)} onQueryInput={...}
  onPickGroup={(g) => pickGroup(idx, g)} onPickMember={(m) => pickMember(idx, m)} />
```
**Не брать буквально:** в акте стоит `status_id: 1` («на складе», стр. 147-156) — для состава группы фильтр по статусу не нужен (АРМ в работе). Пометка «уже в группе «…»» — пачкой `groups_for_devices(ids)` по результату поиска. Секции «Устройства/Группы» — аддитивная правка `Dropdown` (`getGroupSection`, см. ниже). Тот же серверный `groups_add_devices(group_id, device_ids[])` для кнопки «Добавить несколько…» (`GroupAddDevicesModal`).

**Меню строки (⋯)** — `ActionMenu variant="ghost-sm"`: «Вывести из состава» (без подтверждения, D-02); у вложенной группы — «Открыть группу» (переход-фокус через `push('#/groups?id=…')`, прецедент `PlaceEntityViewModal`: `await push(...)`, затем `onClose()`), «Вывести из состава», «Переименовать» (D-07).

---

### `GroupAddDevicesModal.svelte`, `GroupFormModal.svelte`, `GroupTypeFormModal.svelte`, `GroupMoveModal.svelte`

**Аналог форм-модалок:** `features/places/PlaceFormModal.svelte`. Контракт монтирования: родитель монтирует через `{#if}` (свежий экземпляр на открытие), пропсов `open` нет — `<Modal open={true} ...>`.

```ts
// PlaceFormModal.svelte:163-230 — валидация + маппинг серверной ошибки по полю
function validate(): boolean { nameErr = null; ... if (name.trim().length === 0) { nameErr = 'Укажите название места.'; ok = false; } ... }
function mapServerError(e: unknown): void {
  const err = e as Partial<AppError> | undefined;
  const details = err?.details;
  const field = details && typeof details === 'object' && !Array.isArray(details) && 'field' in details
    ? (details as { field?: unknown }).field : undefined;
  if (err?.code === 'VALIDATION' && field === 'name') { nameErr = err.message ?? 'Ошибка валидации'; return; }
  serverErr = err?.message ?? 'Не удалось сохранить место.';
}
async function handleSubmit() {
  if (!validate()) return;
  saving = true; serverErr = null;
  try { ...await apiCall<PlaceDto>('places_rename', { id: place.id, name: name.trim(), version: place.version });
        pushToast('success', '...'); onSaved(saved);
  } catch (e) { mapServerError(e); } finally { saving = false; }
}
```
```svelte
<Modal open={true} title={modalTitle} {onClose}>   <!-- size: 'md' | 'wide' | 'xwide' -->
  <div class="place-form">
    <div class="form-field" class:has-error={nameErr !== null}>
      <label class="form-label" for="pf-name">Название</label>
      <Input id="pf-name" value={name} invalid={nameErr !== null} disabled={saving} oninput={(v) => { name = v; nameErr = null; }} />
      {#if nameErr}<span class="field-error">{nameErr}</span>{/if}
    </div>
```
Выбор из списка — ТОЛЬКО `Dropdown variant="select" flat` (память `native_select_vs_custom_dropdown`: НЕ нативный `<select>` и не `Select`-обёртка). Образец плоского Dropdown с пустым `onExpandGroup` — `PlaceFormModal.svelte:73-76, 290+` (`noExpandKind(): KindOption[] { return []; }`) и `CartridgeFilters.svelte:103-125`:
```svelte
<Dropdown variant="select" flat={true} searchable={false} value={kindLabel} placeholder="Все" loading={false}
  groups={OPTIONS} getGroupId={(o) => o.id} getGroupName={(o) => o.label} getGroupCount={() => 0}
  isGroupExpandable={() => false} isGroupSelected={(o) => o.id === current}
  onExpandGroup={noExpand} getMemberId={(o) => o.id} getMemberName={(o) => o.label}
  onSearch={() => {}} onPickGroup={(o) => ...} onPickMember={() => {}} />
```
D-16: `disabled` на `Dropdown` типа данных у свойства с заполненными значениями + текст «заполнено в N группах» (серверная проверка остаётся источником истины).

**`GroupMoveModal` (D-17/D-24)** — берём блок массового переноса из `PlaceContents.svelte:225-320` (а не `PlaceMoveModal`, который переносит узел дерева): `PlacePicker` для цели + текст подтверждения с числом + `Button` «Перенести» `disabled={moveTargetId === null}`; после успеха:
```ts
notifyPlaceContentChanged([place.id, moveTargetId]);   // источник + цель; PlaceTree расширит до цепочек предков
pushToast('success', 'Перенесено: ...');               // текст из ответа groups_move («группа и N устройств»)
```
Счётчик «переедет N устройств» — свой запрос при открытии модалки (отменяемый `$effect` с `cancelled`-флагом, стр. 244-262), а не переиспользование уже загруженного списка.

---

### `PropertyRequiredViolatorsPopup.svelte` (D-14)

**Аналог:** `lib/components/NumberTakenPopup.svelte` (презентационный попап: все данные через props, ноль собственных запросов).
```svelte
<Modal open={true} size="md" title={`Номер «${number}» уже занят`} {onClose}>
  <p class="intro">...</p>
  <dl class="record-card">{#each rows as row (row.label)}<dt>{row.label}</dt><dd title={row.fullValue ?? undefined}>{row.value}</dd>{/each}</dl>
  {#snippet footer()}<Button variant="secondary" onclick={onClose}>Закрыть</Button>...{/snippet}
</Modal>
```
Данные — из `group_type_properties_empty_groups(property_id)` после отказа сервера. Список групп-нарушителей — кнопки-ссылки с переходом к группе (`onNavigateToGroup`).

---

### `ui/src/lib/api/groups.ts`

**Аналог:** `lib/api/devices.ts` — объект с методами-обёртками над `apiCall<R>(name, args)`; типы из `../../bindings`; Tauri-id как `number`.
```ts
import { apiCall } from './client';
import type { DeviceDto, ... } from '../../bindings';
export const devices = {
  get: (id: number) => apiCall<DeviceDto>('devices_get', { id }),
  update: (id: number, version: number, patch: DevicePatch) =>
    apiCall<DeviceSaveOutcome>('devices_update', { id, version, patch }),
  delete: (id: number, version: number) => apiCall<null>('devices_delete', { id, version }),
};
```
`apiCall` (`lib/api/client.ts`) сам выбирает транспорт (Tauri `invoke` или `POST /api/v1/<name>`), парсит `AppError`, на 403 показывает тост. Имена методов важны для INV-7: маркер реестра — `'groups.move('` (стиль вызова в компоненте), а не имя команды (как `acts.delete(`).

---

### `routes.ts` и `features/layout/sidebar-config.ts`

**Аналог:** сами файлы.
```ts
// routes.ts — добавить в `routes`, НЕ добавлять в `employeeRoutes` ('*' → AccessDenied)
import PlacesPage from './features/places/PlacesPage.svelte';
export const routes = { ..., '/places': PlacesPage, '/devices': DevicesPage, ... } as const;
```
```ts
// sidebar-config.ts:16-35 — вставить между /devices и /acts; обновить PINNED (13 items + 4 dividers = 17 entries)
{ kind: 'item', route: '/places', label: 'Места', phase: 39, roles: ['admin', 'manager'] },
{ kind: 'divider' },
{ kind: 'item', route: '/devices', label: 'Устройства' },
{ kind: 'item', route: '/acts', label: 'Акты' },
```
Новая запись: `{ kind: 'item', route: '/groups', label: 'Группы', roles: ['admin', 'manager'] }`. Комментарий над массивом про позиции разделителей уже устарел — переписать.

---

### `lib/components/Dropdown.svelte` (+ `getGroupSection`)

**Аналог:** сам файл (1094 строки; `Props` — стр. 17-75, дженерик `TGroup`/`TMember`). Правка — НЕОБЯЗАТЕЛЬНЫЙ проп; когда не задан — ни одного изменения в DOM/стилях/поведении существующих потребителей (Pitfall 14).
```ts
/** …новый проп в interface Props, рядом с getGroupMeta/getGroupSub */
getGroupSection?: (g: TGroup) => string | undefined;
```
Заголовок секции: нефокусируемый элемент `role="presentation"`, не участвует в стрелках, `aria-activedescendant` и подсчёте результатов; клавиатурный контракт (двухступенчатый Escape, Home/End, drill-in) не меняется. Демонстрация — в существующей `showcase/sections/DropdownSection.svelte` (иначе правка не проверяема глазами).

---

### `lib/components/MovementTimeline.svelte` (D-28/D-29)

**Аналог:** сам файл — «единственное место, где закодирована анатомия строки таймлайна» (шапка). Четвёртый потребитель (история группы) подключается к нему же без форка.
```ts
// MovementTimeline.svelte:56-69 — reasonText: добавить ветки
function reasonText(entry: MovementEntryDto): string {
  if (entry.source === 'manual') { ... }
  if (entry.source === 'act') { return entry.act_number ? `актом №${entry.act_number}` : 'актом'; }
  return 'причина не определена';          // безопасный фолбэк
}
```
```svelte
<!-- :115-125 — образец кликабельного сегмента основания -->
{#if entry.source === 'act' && entry.act_id !== null && entry.act_number}
  <span class="timeline-reason">актом №<button type="button" class="timeline-link"
    onclick={() => handleNavigateToAct(entry.act_id as number)}>{entry.act_number}</button></span>
{:else}<span class="timeline-reason">{reasonText(entry)}</span>{/if}
```
**Применить:** новый проп `onNavigateToGroup?: (groupId: number) => ...` (по образцу `onNavigateToAct`); для `source === 'group'` у устройства — «в составе группы «{group_label}»» с кнопкой-ссылкой; для `entity_type === 'group'` — «перенос группы». JS-зеркала формулы `*_place_path_short` НЕ создавать (WR-03/WR-08 фазы 39.2: копии расходились; `from_place_path_short` приходит с сервера). Потребители, откуда берётся запрос: `PlaceEntityViewModal.svelte:145-150` (`apiCall<MovementEntryDto[]>('place_movements_get_timeline', { entityType, entityId })`), `CartridgeDetail.svelte:61`, `PrinterDetail.svelte:175`. Для группы — `entityType: 'group'`; навигация — `await push(`#/groups?id=${id}`); onClose();` (стр. 255-266 PlaceEntityViewModal).

---

### `features/reports/ReportTable.svelte` (D-25/D-26)

**Аналог:** сам файл (стр. 1-240; `TableRow` из общих примитивов). Свёрнутая строка пакета — `TableRow group groupExpanded groupName="АРМ #3 (6 устройств)" onToggleGroup` + раскрытие в строки устройств; строки с `batch_id` без заголовка (после фильтра типа устройства) рисовать как обычные. Печать не трогать — серверный HTML из плоских строк.

---

### `features/devices/DeviceFormBody.svelte` (D-19)

**Аналог:** сам файл, блок «9. Required: Место» (стр. 981-1000): `PlacePicker value={placeId} onChange id="f-place" invalid disabled={readonly}`. Применить `disabled={readonly || inGroup}` + подпись «Место задаётся группой «АРМ #3»» со ссылкой-переходом на `#/groups?id=…`. Данные — `groups_for_devices([id])` при открытии формы редактирования, без расширения `DeviceDto`. Гейт словаря (`check-group-vocabulary.mjs`) должен иметь allowlist-пару файл+маркер для этой надписи.

---

### Переименование свёртки (SPEC req.14) и гейт словаря

| Файл:строка | Сейчас | Действие |
|-------------|--------|----------|
| `features/devices/DeviceFilters.svelte:102` | «Группировать похожие» | → «Свернуть одинаковые» |
| `features/devices/DeviceList.svelte:99` | «Групп: {groups.length}» | переформулировать («Строк: N»/«Свёрнуто: N») |
| `lib/components/TableRow.svelte:61` | «Свернуть группу»/«Развернуть группу» | → «Свернуть»/«Развернуть» |
| `showcase/sections/TableSection.svelte:209` | «Строка-группа» | «Раскрываемая строка» |
| `showcase/sections/DropdownSection.svelte:109` | «Комбобокс с группами (drill-in)» | «… со свёрнутыми наборами (drill-in)» |

Идентификаторы (`group`, `DeviceGroup`, `DeviceGroupRow`) НЕ переименовывать. `bindings.ts` комментарии — править doc-комментарии в Rust (`dto/device.rs`), файл генерируется.

**Гейт `ui/scripts/check-group-vocabulary.mjs` (+`--selftest`)** — аналог `scripts/check-device-form-quantity-gate.mjs` (шапка стр. 1-50: объяснение «почему существует», правила, zero-dependency, режимы `--src=<dir>` и `--selftest` на фикстурах в памяти). `stripComments` и `balancedParens` — скопировать из `check-place-tree-invalidation.mjs:73-130`. Правило: в `ui/src/**/*.{svelte,ts}` вне allowlist (`features/groups/`, `lib/api/groups.ts` и явные пары файл+маркер) пользовательские литералы не содержат корней `[Гг]руп`/`[Сс]груп`. Selftest: подложенная копия с «Группировать» → гейт падает; без неё — проходит (чтобы «зелёный» отличался от «ничего не ищет»). Подключить в `ui/package.json` → `lint` (стр. 16): `... && node scripts/check-group-vocabulary.mjs --selftest && node scripts/check-group-vocabulary.mjs`.

---

### `ui/scripts/check-place-tree-invalidation.mjs` (реестр INV-7, D-08)

**Аналог:** сам файл, реестр стр. ~716-750:
```js
const MUTATING_COMPONENTS = [
  { component: 'OperationModal', props: ['onSuccess'] },
  { component: 'DeviceFormModal', props: ['onSaved'] },
  { component: 'ActFormModal', props: ['onSaved'] },
  ...
];
const FORWARDING_COMPONENTS = [{ component: 'PlaceEntityViewModal', forwardProp: 'onChanged' }];
const DIRECT_CALL_MARKERS = [
  { marker: "'places_move_subtree_contents'", label: 'массовый перенос (places_move_subtree_contents)' },
  { marker: 'acts.delete(', label: 'мягкое удаление/undo акта (acts.delete)' },
];
```
**Применить:** добавить в `DIRECT_CALL_MARKERS` маркеры `'groups.move('`, `'groups.addDevices('`, `'groups.setParent('` (литеральная подстрока ДОЛЖНА совпадать со стилем вызова в компоненте); `groups_remove_devices`/`groups_delete` место не меняют, но счётчики состава — тоже в реестр, если решит план. Серверные ответы этих команд обязаны нести `changed_place_ids`. Скан идёт по всему `ui/src/features`, поэтому новый компонент, вызывающий зарегистрированный маркер без `notifyPlaceContentChanged(`, ловится автоматически (INV-7 «перечислять от сервера», урок 40.1: три раунда верификации по одному экрану).

---

## Общие паттерны

### Единая точка записи и одна транзакция
**Источник:** `place_service.rs:686-790` (`move_subtree_contents`), `device_service.rs:633-780` (`update`).
**Применять к:** все мутации `group_service.rs`, особенно перенос, добавление/вывод состава, `groups_set_values`, `group_type_properties_reorder`.
Замыкание writer'а: `let tx = conn.transaction().map_err(map_rusqlite)?; … tx.commit().map_err(map_rusqlite)?;`. Мутация + журнал `place_movements` + `audit_log` — в одной `tx`. `user_id` извлекать ДО замыкания (`Identity` не `Send`).

### Авторизация (двойной гейт)
**Источник:** `tauri_cmds/places.rs:28-43` и первая строка методов `place_service.rs`.
`authorize(caller, &Action::X)?` в `build_*` И первой строкой метода сервиса. HTTP не дублирует гейт — делегирует в `build_*` (`http/places.rs` шапка стр. 9-11).

### Обработка ошибок
**Источник:** `trackly-core/src/error.rs` (`AppError::{NotFound, Conflict, OptimisticLockMismatch, Validation{field,message}, Forbidden, Internal{source_chain}}`) + `trackly_infra::error_conversions::map_rusqlite`.
Сообщения валидации — по-русски. `spawn_blocking` → `.map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking: {e}") })`. HTTP: `Validation` 400, `Conflict` 409, `Forbidden` 403.

### Журнал перемещений (одна точка записи)
**Источник:** `place_movements_sqlite.rs::record_movement_if_applicable`. Никакого собственного `INSERT INTO place_movements` в сервисах; для пакета — соседний `record_batch_movement_if_applicable` с общим `batch_id` (UUID v4, `uuid::Uuid::new_v4()` в сервисе) и `entity_label`.

### Инвалидация клиентских счётчиков
**Источник:** `ui/src/lib/stores/placeContentEvents.svelte.ts` (`notifyPlaceContentChanged(placeIds)`), потребитель — `PlaceTree.svelte:372-395`. Каждый клиентский producer смены места звонит после успешного ответа; реестр INV-7 это принудительно проверяет.

### Рунные эффекты — сходимость
**Источник:** `PlaceTree.svelte:330-395` и память `compile_gates_miss_svelte_runtime`. Любой `$effect`, пишущий `$state`, который сам же читает, — запись в `untrack` и/или условная. `svelte-check`/`eslint`/`pnpm build` этого не видят: нужна живая проверка на Tauri-десктопе и в LAN-браузере (`pnpm --dir ui build` перед проверкой браузера — серверный режим отдаёт устаревший `ui/dist`).

### Стили и токены
Только CSS-переменные `--tr-*` (`check-tokens.mjs`, `check-contrast.mjs`, `check-focus-outline.mjs` в `lint`). Ключевые токены из образцов: `--tr-space-{2xs,xs,sm,md,lg,xl}`, `--tr-surface-raised`, `--tr-border`, `--tr-border-strong`, `--tr-radius-{xs,sm,md}`, `--tr-elev-{1,3}`, `--tr-text-{primary,secondary,tertiary}`, `--tr-focus-ring`, `--tr-danger`, `--tr-font-size-{body,caption,label,h3}`.

### Приватность тестов и фикстур
**Источник:** `scripts/check-privacy.mjs` (pre-commit + CI). Только вымышленные данные: «Иванов И.И.», «Петров П.П.», `Склад А`, `Ноутбук`, «АРМ #3». Агентские артефакты (REVIEW/VERIFICATION) грепать на реальные реквизиты ДО коммита.

---

## Аналог не найден

| Файл | Роль | Поток | Причина / что использовать |
|------|------|-------|----------------------------|
| `features/groups/GroupUsersField.svelte` (чипсы + звёздочка «основной» + `Dropdown` добавления) | component | CRUD | Чипсов с переключаемой звёздочкой в проекте нет; ближайшие — `PersonAutocomplete.svelte` (ввод людей) и `Badge`. Каркас — UI-SPEC §10.4; данные — `groups_user_options`, не `users_list` |
| `Dropdown` секции-заголовки (`getGroupSection`) | component | — | В `Dropdown` секций нет (расхождение №3); писать аддитивно по контракту UI-SPEC §6.3 |
| Нормализация `ip`/`mac`/`number` (`group_values.rs`) | utility | transform | В проекте нет нормализаторов этих типов; опора — `std::net::IpAddr` и эскизы RESEARCH «Code Examples» |
| EAV-схема значений свойств (`group_property_values`) | migration/repo | CRUD | Прецедента key-value нет (`org_settings` V026 — фиксированные колонки); схема из RESEARCH Pattern 1 (проверена DDL в `sqlite3`) |

---

## Метаданные

**Область поиска аналогов:** `crates/trackly-core/src/{auth.rs,domain,ports}`, `crates/trackly-infra/src/repos` + `tests`, `crates/trackly-app/src/{services,tauri_cmds,http,dto,context.rs,specta_export.rs}` + `tests`, `migrations/V037-V044`, `ui/src/{features/places,features/acts,features/devices,features/reports,features/layout,lib/components,lib/api,lib/utils,lib/stores,routes.ts}`, `ui/scripts`, `ui/package.json`.
**Файлов просканировано:** ~75 (прочитаны целиком или целевыми диапазонами: 41 — CONTEXT/RESEARCH/UI-SPEC, 34 — кодовая база).
**Дата извлечения паттернов:** 2026-10-04

**Расхождения, которые планировщик обязан учесть (из RESEARCH, подтверждены чтением кода):**
1. `PlaceService::{create,rename,…}` (repo в автокоммите + отдельная tx для аудита) — не копировать; брать `move_subtree_contents`.
2. `PlaceTree` не обобщается — `GroupTree` копирует анатомию.
3. `Placeholder.svelte` — не пустое состояние; использовать `Table empty`.
4. `Dropdown` без секций — нужна аддитивная правка.
5. `DeviceFormBody` шлёт `place_id` при каждом сохранении — guard сравнивает с текущим.
6. В `act_service.rs` ровно 8 write-site'ов `devices.place_id` (строки 596, 948, 1133, 1756, 2285, 2355, 2443, 3645); S9 (`cartridges_sqlite.rs:646`) — без guard'а сознательно.
7. В `Cargo.toml` фактически rusqlite 0.38 / refinery 0.9 (CLAUDE.md устарел) — сверяться с `Cargo.toml`.
