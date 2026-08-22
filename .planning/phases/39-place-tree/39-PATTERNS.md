# Phase 39: Дерево мест - Pattern Map

**Mapped:** 2026-08-22
**Files analyzed:** 34 (new: 20, modified: 11, deleted/gutted: 3)
**Analogs found:** 34 / 34

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `migrations/V037__places.sql` | migration | CRUD (schema) | `migrations/V002__core_entities.sql` (`locations` table) | role-match (new tree-shaped table, no direct analog for adjacency-list) |
| `migrations/V038__places_migrate_devices_acts_cartridges.sql` | migration | CRUD (schema) | `migrations/V025__cartridge_printer_link.sql` (FK-add migration touching existing tables) | role-match |
| `crates/trackly-core/src/domain/places.rs` | model | CRUD | `crates/trackly-core/src/domain/devices.rs` | exact |
| `crates/trackly-core/src/ports/places.rs` | model (port/trait) | CRUD | `crates/trackly-core/src/ports/devices.rs` | exact |
| `crates/trackly-core/src/auth.rs` (modify: `+ReadPlaces, +MutatePlaces`) | middleware (authz) | request-response | itself (existing `Action`/`authorize`) | exact, **with a locked deviation — see Shared Patterns → Authorization** |
| `crates/trackly-infra/src/repos/places_sqlite.rs` | service (repo impl) | CRUD | `crates/trackly-infra/src/repos/devices_sqlite.rs` | exact |
| `crates/trackly-app/src/dto/place.rs` | model (DTO) | request-response | `crates/trackly-app/src/dto/device.rs` | exact |
| `crates/trackly-app/src/services/place_service.rs` | service | CRUD (+ single-writer) | `crates/trackly-app/src/services/device_service.rs` | exact |
| `crates/trackly-app/src/tauri_cmds/places.rs` | controller (Tauri) | request-response | `crates/trackly-app/src/tauri_cmds/devices.rs` | exact |
| `crates/trackly-app/src/http/places.rs` | controller (axum) | request-response | `crates/trackly-app/src/http/devices.rs` | exact |
| `crates/trackly-app/src/specta_export.rs` (modify) | config (command registry) | — | itself | exact |
| `crates/trackly-app/src/services/act_service.rs` (modify: 8+ call sites) | service | CRUD | itself (`resolve_location_id_in_tx` call sites) | exact — **being replaced, not mirrored**, see "Files Being Deleted/Gutted" |
| `crates/trackly-app/src/services/device_service.rs` (modify: `locations_autocomplete` removed, `place_id` wired) | service | CRUD | itself | exact |
| `crates/trackly-app/src/services/report_service.rs` (modify: 3 report queries) | service | CRUD (aggregate/report) | itself (`LEFT JOIN locations` sites) | exact |
| `crates/trackly-core/src/domain/cartridges.rs` (modify: `location: String` → `place_id`) | model | CRUD | itself | exact |
| `crates/trackly-core/src/domain/acts.rs` (modify: `location_id`→`place_id`, `+place_path_snapshot`) | model | CRUD | itself | exact |
| `crates/trackly-core/src/domain/printers.rs` (modify: `device_location`→resolved path) | model | CRUD | itself | exact |
| `crates/trackly-core/src/domain/requests.rs` (modify: `printer_location`) | model | CRUD | itself | exact |
| `crates/trackly-app/templates/act_handover.minijinja` (modify: `location_name`→`place_path`) | config (template) | file-I/O (render) | itself | exact — see Common Pitfall 5 in RESEARCH.md |
| `ui/src/routes.ts` (modify: `+'/places'`) | route | request-response | itself | exact |
| `ui/src/features/layout/sidebar-config.ts` (modify: `+places item`) | config | — | itself | exact |
| `ui/src/features/places/PlacesPage.svelte` | component (page) | request-response | `ui/src/pages/RequestsPage.svelte` / `ui/src/features/requests/RequestsMasterDetail.svelte` (wrapper level) | role-match |
| `ui/src/features/places/PlacesMasterDetail.svelte` | component | request-response | `ui/src/features/requests/RequestsMasterDetail.svelte` | exact (UI-SPEC §6.2 mandates literal copy) |
| `ui/src/features/places/PlaceTree.svelte` | component | request-response | `ui/src/lib/components/Dropdown.svelte` (tree/keyboard mechanics only) | role-match (new: no existing `role="tree"` component) |
| `ui/src/features/places/PlaceTreeNode.svelte` | component | request-response | `ui/src/lib/components/Dropdown.svelte` (option-row rendering) | role-match |
| `ui/src/features/places/PlaceContents.svelte` | component | request-response | existing `Tabs` + `Table(fillHeight)` consumers (e.g. `ui/src/features/reports/ReportsPage.svelte` tab usage) | role-match |
| `ui/src/lib/components/PlacePicker.svelte` | component | request-response | `ui/src/lib/components/LocationAutocomplete.svelte` | exact — **replaces it, see Files Being Deleted/Gutted** |
| `ui/src/features/places/PlaceFormModal.svelte` | component | request-response | any existing `Modal size="md"` form (e.g. act/device create modal) | role-match |
| `ui/src/features/places/PlaceMoveModal.svelte` | component | request-response | same as above + embeds `PlacePicker` | role-match |
| `ui/src/features/showcase/sections/PlacePickerSection.svelte` | component (showcase) | request-response | `ui/src/features/showcase/sections/*Section.svelte` (Dropdown/Table/Tabs sections) | exact |
| `ui/src/features/devices/DeviceAutocompleteField.svelte` (modify: drop `field="location"` special-case) | component | request-response | itself | exact |
| `ui/src/features/reports/ReportsPage.svelte` / `ReportFilters.svelte` (modify) | component | request-response | itself | exact |
| `ui/src/features/devices/DeviceImportCsvModal.svelte` (modify: path-based resolution) | component | file-I/O | itself | exact |
| `crates/trackly-infra/tests/places_crud.rs` | test | CRUD | `crates/trackly-app/tests/devices_crud.rs` | exact |
| `crates/trackly-app/tests/places_move_cycle.rs` | test | CRUD | `crates/trackly-app/tests/devices_crud.rs` (structure) | role-match |
| `crates/trackly-app/tests/places_search.rs` | test | request-response | `crates/trackly-app/tests/devices_search.rs` (not read this session — same naming family) | role-match |
| `crates/trackly-app/tests/places_contents.rs` | test | request-response | `crates/trackly-app/tests/devices_crud.rs` | role-match |
| `crates/trackly-app/tests/places_delete_blocked.rs` | test | CRUD | `crates/trackly-app/tests/devices_crud.rs` | role-match |
| `crates/trackly-app/tests/acts_place_snapshot.rs` | test | CRUD | `crates/trackly-app/tests/acts_update_return.rs` (not read this session — same naming family) | role-match |
| `crates/trackly-app/tests/role_endpoint_matrix.rs` (extend) | test | request-response | itself | exact |
| `crates/trackly-infra/tests/migration_idempotency.rs` (extend) | test | CRUD (schema assertion) | itself | exact |

---

## Pattern Assignments

### `crates/trackly-core/src/domain/places.rs` (model, CRUD)

**Analog:** `crates/trackly-core/src/domain/devices.rs` (192 lines, read in full)

**Module doc / no-I/O convention** (lines 1-13):
```rust
//! Domain value types for the Devices entity.
//!
//! These types use UI-friendly field names (Path B from Phase 2 PATTERNS.md):
//! ...
//! NO serde::Serialize/Deserialize or specta::Type derives here — those live
//! in the DTO layer in trackly-app. Only `#[derive(Debug, Clone, PartialEq, Eq)]`.

use crate::error::AppError;
```
`places.rs` must follow this exactly: `PlaceRow`/`PlaceNew`/`PlacePatch`/`PlaceFilter` get `#[derive(Debug, Clone, PartialEq, Eq)]` only — no serde/specta here (that's `dto/place.rs`'s job).

**New/Patch/Row triad shape** (lines 15-105, `DeviceNew`/`DevicePatch`/`DeviceRow` — mirror field-for-field):
```rust
pub struct DeviceNew {
    pub type_id: i64,
    pub name: String,
    // ...
    pub location_id: Option<i64>,
    pub status_id: i64,
}

pub struct DevicePatch {
    pub type_id: Option<i64>,
    pub name: Option<String>,
    // every field Option<T>, all-optional
    pub location_id: Option<i64>,
    pub status_id: Option<i64>,
}

pub struct DeviceRow {
    pub id: i64,
    // ...
    pub location_id: Option<i64>,
    /// Resolved location name from the `locations` table (via LEFT JOIN on read paths).
    pub location: Option<String>,
    pub status_id: i64,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub deleted_at_utc: Option<i64>,
    pub version: i64,
}
```
`PlaceRow` mirror: `id, parent_id: Option<i64>, kind: PlaceKind, name, level: Option<i64>, is_storage: bool, sort_order: Option<i64>, archived_at_utc: Option<i64>, notes: Option<String>, created_at_utc, updated_at_utc, deleted_at_utc, version`. Note the `location` field's "resolved via LEFT JOIN, populated on read paths only" convention — apply the same for a `full_path: Option<String>` field on `PlaceRow`, populated via the `place_full_paths` view JOIN (RESEARCH.md §Pattern 1), not stored.

**Closed enum with `from_str` + validation-error convention** (lines 132-192, `AutocompleteField`):
```rust
pub enum AutocompleteField {
    Name, Model, Specs, Kit, State, Location,
}
impl AutocompleteField {
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "name" => Ok(Self::Name),
            // ...
            other => Err(AppError::Validation {
                field: "field".to_string(),
                message: format!(
                    "Неподдерживаемое поле автодополнения: «{other}». \
                     Поддерживаемые поля: name, model, specs, kit, state, location."
                ),
            }),
        }
    }
}
```
`PlaceKind` (D-02's closed 6-value enum) must follow this exact shape — `from_str`/`as_str` roundtrip, Russian validation message listing the permitted values. This is also the canonical precedent for `Role::from_str` (`crates/trackly-core/src/auth.rs:34-44`) — same idiom, third confirmation in the codebase.

---

### `crates/trackly-core/src/ports/places.rs` (model/port, CRUD)

**Analog:** `crates/trackly-core/src/ports/devices.rs` (94 lines, read in full)

**Trait shape with associated `Conn` type** (lines 1-36):
```rust
//! `DeviceRepository` port — repository trait for the Devices entity.
//!
//! Pattern: associated `type Conn` keeps rusqlite out of trackly-core.
//! The concrete type (`rusqlite::Connection`) is specified in the adapter
//! impl in `trackly-infra::repos::devices_sqlite`.

use crate::domain::devices::{...};
use crate::error::AppError;

pub trait DeviceRepository {
    type Conn;
    fn create(&self, conn: &mut Self::Conn, new: &DeviceNew, now_utc: i64) -> Result<i64, AppError>;
    fn get(&self, conn: &Self::Conn, id: i64) -> Result<DeviceRow, AppError>;
    fn list(&self, conn: &Self::Conn, filter: &DeviceFilter, page: &Pagination) -> Result<(Vec<DeviceRow>, u64), AppError>;
    fn update(&self, conn: &mut Self::Conn, id: i64, version: i64, patch: &DevicePatch, now_utc: i64) -> Result<DeviceRow, AppError>;
    fn delete_soft(&self, conn: &mut Self::Conn, id: i64, version: i64, now_utc: i64) -> Result<(), AppError>;
    // ... search_fts, autocomplete, list_grouped, count_by_status, list_by_ids
}
```
`PlaceRepository` follows this shape exactly (RESEARCH.md's Code Examples section already sketches the full trait — `create/get/list_children/list_all/rename/move_node/archive/unarchive/delete_hard/subtree_stats/full_path`). Copy the doc-comment convention: every mutating method takes `&mut Self::Conn` (transaction access), every read-only method takes `&Self::Conn`.

---

### `crates/trackly-core/src/auth.rs` (modify — Shared Pattern, see below)

**Analog:** itself — existing `Action` enum + `authorize()` (364 lines, read in full)

**Full permission-matrix idiom** (lines 88-164):
```rust
pub enum Action {
    ManageUsers,
    ManageSettings,
    MutateDevices,
    MutateActs,
    MutateCartridges,
    ReadData,
    CreateRequest,
    MutatePrinters,
    TransitionRequests,
    ReadPrinters,
    ReadRequests,
    DeleteRequests,
    CancelOwnRequest,
}

pub fn authorize(identity: &Identity, action: &Action) -> Result<(), AppError> {
    let allowed = match action {
        Action::ManageUsers | Action::ManageSettings => {
            matches!(identity.role, Role::Admin)
        }
        Action::MutateDevices
        | Action::MutateActs
        | Action::MutateCartridges
        | Action::MutatePrinters
        | Action::TransitionRequests
        | Action::ReadPrinters
        | Action::ReadData
        | Action::DeleteRequests => {
            matches!(identity.role, Role::Admin | Role::Manager)
        }
        Action::CreateRequest | Action::ReadRequests | Action::CancelOwnRequest => true,
    };
    if allowed { Ok(()) } else { Err(AppError::Forbidden) }
}
```

**THE DEVIATION (D-20) — do not copy the `MutateDevices` bucket pattern for places:**
Every existing `Mutate*` action lands in the `Admin | Role::Manager` arm. Places is the first entity where **mutate is Admin-only** while **read is Admin|Manager**. Add exactly:
```rust
pub enum Action {
    // ...existing variants unchanged...
    /// Просмотр дерева мест и содержимого узла (PLC-06). Admin | Manager.
    ReadPlaces,
    /// Создание/переименование/перемещение/архивация/удаление места. Admin ONLY (D-20).
    MutatePlaces,
}

pub fn authorize(identity: &Identity, action: &Action) -> Result<(), AppError> {
    let allowed = match action {
        Action::ManageUsers | Action::ManageSettings | Action::MutatePlaces => {
            matches!(identity.role, Role::Admin)
        }
        Action::MutateDevices
        | Action::MutateActs
        | Action::MutateCartridges
        | Action::MutatePrinters
        | Action::TransitionRequests
        | Action::ReadPrinters
        | Action::ReadData
        | Action::ReadPlaces          // <- new, same bucket as ReadData
        | Action::DeleteRequests => {
            matches!(identity.role, Role::Admin | Role::Manager)
        }
        Action::CreateRequest | Action::ReadRequests | Action::CancelOwnRequest => true,
    };
    // ...
}
```
`MutatePlaces` joins `ManageUsers`/`ManageSettings` (Admin-only bucket) — **NOT** `MutateDevices`'s bucket. `ReadPlaces` joins `ReadData`'s bucket. Confirmed against `crates/trackly-core/src/auth.rs:141-164` (full text read this session).

**Test-doc convention to mirror** (lines 181-364, `mod tests`): every `Action` variant gets an Admin-ok test and an Employee/Manager-forbidden test, named `authorize_<role>_<action>_{ok,forbidden}`. Add `authorize_admin_mutate_places_ok`, `authorize_manager_mutate_places_forbidden` (the one that would catch a copy-paste regression), `authorize_manager_read_places_ok`, `authorize_employee_read_places_forbidden`.

---

### `crates/trackly-infra/src/repos/places_sqlite.rs` (repository, CRUD)

**Analog:** `crates/trackly-infra/src/repos/devices_sqlite.rs` (1277 lines total; read lines 1-234 this session)

**Adapter struct + column-mapping doc convention** (lines 1-37):
```rust
//! SQLite adapter for `DeviceRepository`.
//!
//! `SqliteDeviceRepository` implements `trackly_core::ports::devices::DeviceRepository`
//! using `rusqlite::Connection` as the `Conn` associated type.
//! ...
//! Все SQL параметризованы через `rusqlite::params![...]` (T-02-03-01).

use rusqlite::{Connection, OptionalExtension};
use trackly_core::domain::devices::{...};
use trackly_core::error::AppError;
use trackly_core::ports::devices::DeviceRepository;
use crate::error_conversions::map_rusqlite;

#[derive(Debug, Default, Clone)]
pub struct SqliteDeviceRepository;

const SELECT_DEVICES: &str = "
    SELECT d.id, d.type_id, d.name, ..., l.name AS location_name
    FROM devices d
    LEFT JOIN locations l ON d.location_id = l.id
";

fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DeviceRow> { /* positional row.get(N) */ }
```
`SqlitePlaceRepository` mirror: replace the `LEFT JOIN locations` with `LEFT JOIN place_full_paths pfp ON pfp.place_id = <root>.id` where a resolved path is needed on the read path; every other SELECT/mapping convention identical.

**`_in_tx` transaction-helper convention** (lines 135-230, `impl SqliteDeviceRepository` inherent block — used directly by the service layer inside `writer.execute` closures):
```rust
/// Вспомогательные методы для использования внутри rusqlite-транзакций.
/// `DeviceService` использует эти методы внутри `writer.execute` closures.
impl SqliteDeviceRepository {
    pub fn resolve_location_id_in_tx(&self, tx: &rusqlite::Transaction<'_>, location: Option<&str>, now_utc: i64) -> Result<Option<i64>, AppError> { ... }
    pub fn create_in_tx(&self, tx: &rusqlite::Transaction<'_>, new: &DeviceNew, now_utc: i64) -> Result<i64, AppError> {
        tx.execute("INSERT INTO devices (...) VALUES (...)", rusqlite::params![...]).map_err(map_rusqlite)?;
        Ok(tx.last_insert_rowid())
    }
    pub fn get_in_tx(&self, tx: &rusqlite::Transaction<'_>, id: i64) -> Result<DeviceRow, AppError> {
        tx.query_row(&format!("{SELECT_DEVICES} WHERE d.id = ?1"), rusqlite::params![id], from_row)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => AppError::NotFound { entity: "device", id },
                other => map_rusqlite(other),
            })
    }
    // update_in_tx with optimistic-lock version check...
}
```
`SqlitePlaceRepository` needs the equivalent `_in_tx` set: `create_in_tx`, `get_in_tx`, `rename_in_tx`, `move_node_in_tx` (RESEARCH.md Pattern 3's cycle-check query runs *inside* this method, before the `UPDATE`), `archive_in_tx`/`unarchive_in_tx`, `delete_hard_in_tx` (must run the Pattern 2 subtree-stats query first and return `AppError::Conflict`-shaped data on non-empty), `subtree_stats` (read-only, Pattern 2 SQL), `full_path` (single-row `SELECT full_path FROM place_full_paths WHERE place_id = ?1`).

**Do NOT copy `resolve_location_id_in_tx`'s auto-create semantics** — see "Files Being Deleted/Gutted" below; `places_sqlite.rs` has no equivalent helper. Callers pass a validated `place_id: Option<i64>` directly.

---

### `crates/trackly-app/src/services/place_service.rs` (service, CRUD + single-writer)

**Analog:** `crates/trackly-app/src/services/device_service.rs` (1099 lines; targeted reads: lines 150-260, 415-464)

**Single-writer `create()` shape** (lines 150-199):
```rust
pub async fn create(&self, new: DeviceNew) -> Result<DeviceDto, AppError> {
    Self::validate_new(&new)?;
    let now = self.clock.unix_seconds();
    let repo = self.repo.clone();
    let mut domain_new: trackly_core::domain::devices::DeviceNew = new.into();
    let id = self
        .writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            // ... resolve, insert ...
            let id = repo.create_in_tx(&tx, &domain_new, now)?;
            let after = repo.get_in_tx(&tx, id)?;
            let after_dto = DeviceDto::from(after);
            let after_json = serde_json::to_string(&after_dto).map_err(|e| AppError::Internal {
                source_chain: format!("audit_log after-json: {e}"),
            })?;
            tx.execute(
                "INSERT INTO audit_log (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
                 VALUES ('device', ?1, 'create', ?2, NULL, ?3, NULL, ?4)",
                rusqlite::params![id, user_id_opt, after_json, now],
            ).map_err(map_rusqlite)?;
            tx.commit().map_err(map_rusqlite)?;
            Ok(id)
        })
        .await?;
    self.get(id).await
}
```
`PlaceService::create/rename/move/archive/unarchive/delete_hard` all follow this shape: `self.writer.execute(move |conn| { tx = conn.transaction(); ...; audit_log INSERT with entity_type='place'; tx.commit(); Ok(...) }).await`. `move_node` additionally runs the Pattern 3 cycle-check query as the first statement inside the closure, before the `UPDATE places SET parent_id = ...` (RESEARCH.md §Architecture Patterns > Pattern 3 — must be in the same transaction/connection, no separate round-trip, because this is the single-writer task with no concurrent-writer race).

**Read-path `spawn_blocking` shape** (lines 202-213, `get()`):
```rust
pub async fn get(&self, id: i64) -> Result<DeviceDto, AppError> {
    let readers = self.readers.clone();
    let repo = self.repo.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        repo.get(&conn, id).map(DeviceDto::from)
    })
    .await
    .map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking: {e}") })?
}
```
`PlaceService::get/list_children/list_all/subtree_stats/full_path` follow this exactly — reader-pool `.acquire()` inside `spawn_blocking`, never touching the writer.

**Freeform-value validation + prefix-search shape** (lines 417-464, `locations_autocomplete` — being replaced but its input-length/escaping convention should carry over to place search):
```rust
pub async fn locations_autocomplete(&self, prefix: String) -> Result<Vec<String>, AppError> {
    if prefix.chars().count() > 100 {
        return Err(AppError::Validation { field: "prefix".into(), message: "prefix слишком длинный (макс. 100 символов)".into() });
    }
    let escaped: String = prefix.chars().map(|c| if c == '%' || c == '_' || c == '\\' { format!("\\{c}") } else { c.to_string() }).collect();
    // ... spawn_blocking, prepare/query_map, ESCAPE '\\' ...
}
```
Per RESEARCH.md Pitfall 2, `PlaceService`'s search method must NOT do `LIKE` in SQL for the Cyrillic-case-fold reason — but the length-validation guard (100 chars) and the general "validate → spawn_blocking → readers.acquire()" shape still apply; fetch the small `place_full_paths` candidate set and filter with Rust `.to_lowercase()` substring matching instead of a parameterized `LIKE` pattern.

---

### `crates/trackly-app/src/tauri_cmds/places.rs` + `crates/trackly-app/src/http/places.rs` (dual-transport controller pair, request-response)

**Analog pair:** `crates/trackly-app/src/tauri_cmds/devices.rs` (targeted read, lines 1-190) + `crates/trackly-app/src/http/devices.rs` (targeted reads, lines 1-50, 160-235, 375-394)

**The `build_*` helper — shared by both transports** (`tauri_cmds/devices.rs` lines 1-10, 46-104):
```rust
//! Паттерн: `build_*` helper + thin `#[tauri::command] #[specta::specta]` wrapper.
//! Оба транспорта делегируют одному и тому же `build_*` функции.
//! `#[specta::specta]` ПОСЛЕ `#[tauri::command]` — требование tauri-specta v2 rc.21.

use trackly_core::auth::{authorize, Action, Identity};

/// Мутация: требует `caller` с правом `MutateDevices` (Admin | Manager).
pub async fn build_devices_create(ctx: &AppCtx, caller: &Identity, new: DeviceNew) -> Result<DeviceDto, AppError> {
    authorize(caller, &Action::MutateDevices)?;
    ctx.devices.create(new).await
}
pub async fn build_locations_autocomplete(ctx: &AppCtx, caller: &Identity, prefix: String) -> Result<Vec<String>, AppError> {
    authorize(caller, &Action::ReadData)?;
    ctx.devices.locations_autocomplete(prefix).await
}
```
`tauri_cmds/places.rs` needs `build_places_create/rename/move/archive/unarchive/delete/get/list_children/list_all/subtree_stats/search`, each calling `authorize(caller, &Action::MutatePlaces)?` for every mutation and `authorize(caller, &Action::ReadPlaces)?` for every read — **note this differs from the devices template**, which uses `MutateDevices`/`ReadData` for both mutate and read gates respectively; places needs the two-bucket split from the auth.rs deviation above applied consistently at every one of these call sites.

**Tauri `#[tauri::command]` thin wrapper** (`tauri_cmds/devices.rs` lines 163-190):
```rust
#[tauri::command]
#[specta::specta]
pub async fn devices_create(state: tauri::State<'_, AppCtx>, device: DeviceNew) -> Result<DeviceDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_devices_create(state.inner(), &caller, device).await
}
```
Copy verbatim for `places_create`, `places_rename`, `places_move`, `places_archive`, `places_unarchive`, `places_delete`, `places_get`, `places_list_children`, `places_list_all`, `places_subtree_stats`, `places_search`.

**axum handler + payload struct + router registration** (`http/devices.rs` lines 36-50, 168-181, 380-394, 399-420):
```rust
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayload { pub device: DeviceNew }

pub async fn handler_create(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<CreatePayload>,
) -> Result<Json<DeviceDto>, AppErrorResponse> {
    let identity = session_identity(&session).await.map_err(AppErrorResponse::from)?;
    Ok(Json(build_devices_create(&ctx, &identity, payload.device).await.map_err(AppErrorResponse::from)?))
}

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/devices_list", post(handler_list))
        .route("/api/v1/devices_create", post(handler_create))
        // ...
        .route("/api/v1/locations_autocomplete", post(handler_locations_autocomplete))
}
```
`http/places.rs` mirrors this exactly, routes at `/api/v1/places_create`, `/api/v1/places_rename`, `/api/v1/places_move`, `/api/v1/places_archive`, `/api/v1/places_unarchive`, `/api/v1/places_delete`, `/api/v1/places_get`, `/api/v1/places_list_children`, `/api/v1/places_list_all`, `/api/v1/places_subtree_stats`, `/api/v1/places_search`.

---

### `crates/trackly-app/src/specta_export.rs` (config, modify)

**Analog:** itself (183 lines, read lines 1-60)

**Registration checklist idiom** (lines 1-12, 16-40):
```rust
//! Каждое следующее phase, добавляющее `#[tauri::command]`, ОБЯЗАНО
//! зарегистрировать её здесь — иначе frontend (через bindings.ts) не увидит
//! новый API. Code-review checklist (T-05-06 в threat model плана 05).

pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        // Phase 1
        crate::tauri_cmds::health::health,
        // Phase 2 — Devices CRUD (Plan 03)
        crate::tauri_cmds::devices::devices_list,
        crate::tauri_cmds::devices::devices_create,
        // ...
    ])
}
```
Add a `// Phase 39 — Places CRUD` block listing every new `places_*` command. This is checked by `cargo test --test export_bindings` (regenerates `ui/src/bindings.ts`) — the planner should schedule a task specifically for this registration step, since it is a documented recurring miss ("code-review checklist" comment implies it has been missed before).

---

### Files touched mechanically (existing entities losing their location fields)

**`crates/trackly-app/src/services/act_service.rs`** — `resolve_location_id_in_tx` is called at **8 call sites** (grepped, lines 273-1284: `create`, `update`, per-item override resolution, `bulk_location_id` resolution, return-item resolution). Every one of these:
```rust
let resolved_location_id: Option<i64> =
    if let Some(name) = &payload.location_name {
        devices_repo.resolve_location_id_in_tx(&tx, Some(name), now)?
    } else {
        payload.location_id
    };
```
must become:
```rust
let resolved_place_id: Option<i64> = payload.place_id; // caller already resolved via PlacePicker
// service validates existence/non-archived, does NOT auto-create (D-18)
```
See Common Pitfall 4 in RESEARCH.md — every one of these 8 sites needs conversion, not a subset.

**`crates/trackly-app/src/services/device_service.rs`** — `locations_autocomplete` (lines 415-464, quoted above) is deleted; `place_id` replaces `location_id` throughout `DeviceNew`/`DevicePatch`/`DeviceFilter` construction (same file, lines 150-260 read this session show the exact shape to mirror for `place_id`).

**`crates/trackly-app/src/services/report_service.rs`** — 3 reports join `locations` (per RESEARCH.md/CONTEXT.md citation: lines ~1054, 1084, 1146, 1160, 1245 — not re-read this session, cited in canonical_refs). Each `LEFT JOIN locations l ON ... l.id = ...` becomes `LEFT JOIN place_full_paths pfp ON pfp.place_id = ...`, and the report's `location_name`/`location_id` output columns become `place_path`/`place_id`. Line 1245's cartridge-FK gap comment ("у картриджей FK нет") disappears once `cartridges.place_id` is a real FK.

---

### `migrations/V037__places.sql` / `V038__places_migrate_devices_acts_cartridges.sql` (migration, CRUD)

**Analog:** `migrations/V002__core_entities.sql` (flat `locations` table, standard4 columns) — read in full (30 lines):
```sql
CREATE TABLE locations (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  name            TEXT    NOT NULL UNIQUE,
  kind            TEXT    NULL,
  address         TEXT    NULL,
  notes           TEXT    NULL,
  created_at_utc  INTEGER NOT NULL,
  updated_at_utc  INTEGER NOT NULL,
  deleted_at_utc  INTEGER NULL,
  version         INTEGER NOT NULL DEFAULT 1
);
PRAGMA user_version = 2;
```
This is the `standard4` column convention (`created_at_utc`/`updated_at_utc`/`deleted_at_utc`/`version`) that `places` must also follow — confirmed identical in RESEARCH.md's proposed schema (§Pattern 1). RESEARCH.md's full V037/V038 SQL (already drafted, `WITH RECURSIVE` view, `UNIQUE(COALESCE(parent_id,0), name) WHERE deleted_at_utc IS NULL`, `ON DELETE RESTRICT` FKs) is the concrete artifact to use — do not re-derive it, it is already schema-complete and verified against this migration numbering (`V036__org_settings_full_name.sql` is the last committed migration; `V037` is next).

**`ON DELETE RESTRICT` FK precedent** — confirmed in `migrations/V004__acts.sql:18`:
```sql
location_id       INTEGER NULL REFERENCES locations(id),
```
Note: `V004`'s existing `location_id` FK has **no explicit `ON DELETE` clause** (defaults to `NO ACTION`, which in SQLite behaves like `RESTRICT` for FK-enforced connections). RESEARCH.md's proposed `places` FKs use explicit `ON DELETE RESTRICT` — this is a slight strengthening over the existing implicit pattern, consistent with `acts.parent_act_id ON DELETE RESTRICT` cited as the closest *explicit* precedent (not re-read this session, cited in RESEARCH.md Security Domain table).

**`cartridges.location` freeform-text precedent** (`migrations/V005__cartridges.sql:38`):
```sql
location        TEXT    NULL,                                    -- freeform; locations table is for devices
```
This comment is the exact historical decision D-12/D-13 override — cartridges intentionally did NOT get a `locations` FK before; Phase 39 gives them one via `cartridges.place_id`.

---

### FTS5 trigger pattern (for reference/negative-pattern — NOT to be applied to place text)

**Analog:** `migrations/V013__devices_fts_triggers.sql` (three AFTER triggers, read in full) — the canonical "keep FTS5 external-content table in sync" idiom in this codebase:
```sql
CREATE TRIGGER devices_fts_ai
AFTER INSERT ON devices
WHEN NEW.deleted_at_utc IS NULL
BEGIN
  INSERT INTO devices_fts(rowid, name, inventory_number, serial_number, model)
  VALUES (NEW.id, NEW.name, NEW.inventory_number, NEW.serial_number, NEW.model);
END;

CREATE TRIGGER devices_fts_ad
AFTER DELETE ON devices
BEGIN
  INSERT INTO devices_fts(devices_fts, rowid, name, inventory_number, serial_number, model)
  VALUES ('delete', OLD.id, OLD.name, OLD.inventory_number, OLD.serial_number, OLD.model);
END;

CREATE TRIGGER devices_fts_au
AFTER UPDATE ON devices
BEGIN
  INSERT INTO devices_fts(devices_fts, rowid, name, inventory_number, serial_number, model)
  VALUES ('delete', OLD.id, OLD.name, OLD.inventory_number, OLD.serial_number, OLD.model);
  INSERT INTO devices_fts(rowid, name, inventory_number, serial_number, model)
  SELECT NEW.id, NEW.name, NEW.inventory_number, NEW.serial_number, NEW.model
  WHERE NEW.deleted_at_utc IS NULL;
END;
```
Same idiom repeated verbatim for `cartridges_fts` in `migrations/V016__cartridges_kind_color_settings.sql:52-77` (comment there: `-- Pattern: V013 devices_fts_* triggers (exact analog)`), including the `location` FTS5 column:
```sql
CREATE TRIGGER cartridges_fts_ai
AFTER INSERT ON cartridges
WHEN NEW.deleted_at_utc IS NULL
BEGIN
  INSERT INTO cartridges_fts(rowid, code, location, holder_name)
  VALUES (NEW.id, NEW.code, NEW.location, NEW.holder_name);
END;
-- ...ad/au mirror devices_fts_ad/au exactly, same delete-then-reinsert shape
```
**Do not extend this trigger pattern to `place_path` text on `devices_fts`/`cartridges_fts`.** RESEARCH.md Common Pitfall 1 explains why: the trigger fires on `places`, but the FTS rows needing update live on `devices`/`cartridges`, and by the time the trigger runs the *old* place text is unrecoverable. `V038` instead: (a) drops `location` from `cartridges_fts`'s column list (cartridges.location column is gone — requires `DROP TRIGGER` + `CREATE TRIGGER` for `cartridges_fts_ai/ad/au` without the `location` column, since refinery can't `ALTER TRIGGER`, then `INSERT INTO cartridges_fts(cartridges_fts) VALUES('rebuild')`), and (b) `devices_fts` gets NO new place-related column at all — place search happens via the live-JOIN path (`place_full_paths`) in the service layer, not FTS5.

---

### `migrations/V012__indexes_and_fts.sql` (index precedent, for `idx_places_*`)

Read in full (52 lines) — the index-per-domain convention: `idx_devices_location ON devices(location_id);` and `idx_devices_status`/`idx_devices_type` — plain single-column indexes, no partial-index filter at this layer (partial-index filtering is a V013-level pattern, see below). `idx_places_parent` follows `idx_devices_location`'s shape but SHOULD add the `WHERE deleted_at_utc IS NULL` partial filter (V013's convention, see `idx_devices_autocomplete_name_location` below), since `places` needs it for tree-traversal performance at scale.

**Partial-index-with-soft-delete-filter precedent** (`migrations/V013__devices_fts_triggers.sql:52-71`):
```sql
CREATE INDEX idx_devices_autocomplete_name_location
  ON devices(name, location_id)
  WHERE deleted_at_utc IS NULL AND location_id IS NOT NULL;
```
This exact index is DROPPED in V038 (superseded by `place_id`); `idx_devices_place`/`idx_cartridges_place` in RESEARCH.md's V038 draft follow the same `WHERE deleted_at_utc IS NULL AND place_id IS NOT NULL` shape.

---

### Frontend routing/sidebar (config, modify)

**Analog:** `ui/src/routes.ts` (40 lines, read in full):
```typescript
import MapPage from './pages/MapPage.svelte';
// ...
export const routes = {
  '/': Dashboard,
  '/map': MapPage,
  '/devices': DevicesPage,
  // ...
  '*': NotFound,
} as const;
```
Add `import PlacesPage from './features/places/PlacesPage.svelte';` and `'/places': PlacesPage,` — inserted after `/map` per D-19 ordering. `/map` entry (`Placeholder`) is untouched.

**Analog:** `ui/src/features/layout/sidebar-config.ts` (46 lines, read in full):
```typescript
// PINNED: 11 items + 4 dividers = 15 entries — source of truth per UI-SPEC §Copywriting Sidebar.
export const SIDEBAR_ITEMS: SidebarEntry[] = [
  { kind: 'item', route: '/', label: 'Дашборд', phase: 7 },
  { kind: 'item', route: '/map', label: 'Карта', phase: 'v2' },
  { kind: 'divider' },
  { kind: 'item', route: '/devices', label: 'Устройства' },
  // ...
  { kind: 'item', route: '/users', label: 'Пользователи', phase: 5, roles: ['admin'] },
  // ...
];
```
Insert immediately after the `/map` entry (before the first divider, per UI-SPEC §7): `{ kind: 'item', route: '/places', label: 'Места', roles: ['admin','manager'] }`. Update the `PINNED:` comment count — UI-SPEC §7 states "12 пунктов + 4 разделителя = 16 записей" (current file has 11 items + 4 dividers = 15). The `roles: ['admin']` precedent on `/users` and `/settings` is the exact analog for gating a sidebar entry by role client-side (UX-only, per D-20's note that the real gate is server-side `authorize()`).

---

### `ui/src/lib/components/PlacePicker.svelte` (component, request-response)

**Analog (mechanics to inherit):** `ui/src/lib/components/LocationAutocomplete.svelte` (249 lines, read in full — **this file is deleted**, see below) — debounce/portal/focus-open/cleanup idioms:
```svelte
<script lang="ts">
  import { onDestroy } from 'svelte';
  import { apiCall } from '$lib/api/client';
  import { portal } from '$lib/utils/portal';
  import { dropdownAnchor } from '$lib/utils/dropdownAnchor';

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  // WR-05: scheduleFetch() ... did not cancel pending timer on unmount
  onDestroy(() => {
    if (debounceTimer !== null) clearTimeout(debounceTimer);
  });

  function scheduleFetch(prefix: string, delayMs: number) {
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      await fetchSuggestions(prefix);
      if (!suppress) open = suggestions.length > 0;
      activeIndex = -1;
    }, delayMs);
  }

  function handleFocus() {
    // UAT-fix #3: открываем dropdown сразу на focus (empty prefix → top 20).
    suppress = false;
    scheduleFetch(value, 0);
  }
</script>

<div class="dropdown--location" role="listbox" use:portal use:dropdownAnchor={{ anchorEl: inputEl }}>
```
```scss
// WR-03: дропдаун портирован в <body> из НЕСКОЛЬКИХ компонентов — без
// namespace-класса на корне глобальные правила .dropdown/.dropdown-item/...
// коллизируют между компонентами. Все правила ниже скопированы под
// :global(.dropdown--location ...).
:global(.dropdown--location) {
  position: fixed;
  z-index: 1000;
  background: var(--tr-surface-raised);
  border: 1px solid var(--tr-border);
  border-radius: var(--tr-radius-xs);
  box-shadow: var(--tr-elev-2);
  max-height: 240px;
  overflow-y: auto;
}
```
`PlacePicker.svelte` inherits: `onDestroy` timer cleanup (WR-05), `use:portal` + `use:dropdownAnchor`, open-on-focus (UAT-fix #3 precedent), 200ms debounce, and the **namespaced global-class discipline** (WR-03) — UI-SPEC §10.2 mandates `.dropdown--place` as the new namespaced class, following exactly this `.dropdown--location` precedent (max-height changes 240px→320px per UI-SPEC §3). Endpoint changes from `locations_autocomplete` to the new tree/search `places_*` endpoints; rendering changes from a flat suggestion list to the two-mode (tree/search) panel UI-SPEC §10 specifies. Two-stage Escape (§10.3) is new behavior with no direct analog in `LocationAutocomplete` (which has single-stage Escape, line 87-92) — build fresh per UI-SPEC.

**Analog (visual/keyboard language only, NOT structure — per UI-SPEC §6.2 explicit call-out):** `ui/src/lib/components/Dropdown.svelte` (1094 lines; targeted grep only — `variant: 'combobox' | 'select'`, `flat?: boolean` at lines ~21-25). UI-SPEC §6.2 states explicitly: *"Почему `Dropdown.svelte` не переиспользуется как есть: его drill-in — ровно два уровня... `PlacePicker` наследует у него визуальный язык и клавиатурный контракт (двухступенчатый Escape, `aria-activedescendant`, `scrollIntoView`, состояние «Загрузка…»), но рисует собственную панель."* Do not attempt to parametrize `Dropdown.svelte` for arbitrary depth — UI-SPEC has already ruled this out as a "conscious departure, not a forgotten reuse."

---

### `ui/src/features/places/PlacesMasterDetail.svelte` (component, request-response)

**Analog:** `ui/src/features/requests/RequestsMasterDetail.svelte` (read in full, ~90+ lines shown) — UI-SPEC §6.2 mandates this be a **literal copy**:
```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';
  interface Props { master: Snippet; detail: Snippet; }
  const { master, detail }: Props = $props();
</script>

<div class="master-detail">
  <aside class="master">{@render master()}</aside>
  <section class="detail">{@render detail()}</section>
</div>

<style lang="scss">
  .master-detail {
    display: grid;
    grid-template-columns: 35% 65%;
    gap: var(--tr-space-md);
    align-items: stretch;
    flex: 1 1 auto;
    min-height: 0;
  }
  .master {
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    box-shadow: var(--tr-elev-1);
    overflow: hidden;
    min-width: 320px;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .master > :global(*) { flex: 1 1 auto; min-height: 0; }
  .detail {
    /* same as .master but min-width: 480px */
  }
</style>
```
UI-SPEC §3 explicitly cites this exact `320px`/`480px` min-width pair as "скопировано дословно из `RequestsMasterDetail.svelte`, не пересчитывается." Do not invent new grid values.

---

### `crates/trackly-app/tests/places_crud.rs` (test, CRUD)

**Analog:** `crates/trackly-app/tests/devices_crud.rs` (590 lines; read lines 1-70) — the whole-file setup/fixture idiom:
```rust
//! Интеграционные тесты CRUD-операций `DeviceService`.
//! Каждый тест обёрнут в `tokio::time::timeout(30s)` — защита от Linux-CI deadlock.

use trackly_app::dto::device::{DeviceFilter, DeviceNew, DevicePatch, Pagination};
use trackly_app::services::DeviceService;
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

fn make_service() -> (DeviceService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = DeviceService::new(writer, readers, clock);
    (svc, dir)
}

fn minimal_new(name: &str) -> DeviceNew { DeviceNew { type_id: 1, name: name.to_string(), /* ... */ } }

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_inserts_device_and_audit_log() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let new = minimal_new("Ноутбук Lenovo");
        let dto = svc.create(new).await.expect("create device");
        assert!(dto.id > 0);
        // ... audit_log assertion via readers.acquire() + query_row ...
    }).await.expect("timeout");
}
```
`places_crud.rs` mirrors this exactly: `make_service() -> (PlaceService, TempDir)` via `test_writer_and_readers()`, `tokio::time::timeout(30s)` wrapper on every test (Linux-CI deadlock defense), `#[tokio::test(flavor = "multi_thread", worker_threads = 4)]`. Cover PLC-01's full CRUD surface: create/rename/move/archive/delete, `UNIQUE(parent_id,name)` violation (D-04), FK survival on device re-parenting.

**Analog for `migration_idempotency.rs` extension:** `crates/trackly-infra/tests/migration_idempotency.rs` (61 lines, read in full) — `migrations::max_known_version()` dynamic-count pattern means the test does NOT need a hardcoded bump when V037/V038 land; extend it with a separate assertion block (schema-presence, not the idempotency test itself) checking `locations` table absence and old columns dropped, per RESEARCH.md's Phase Requirements → Test Map for PLC-04.

**Analog for `role_endpoint_matrix.rs` extension:** file header doc-comment convention (grepped, not fully read) numbers every case `//! N. <Role> session → POST /api/v1/<endpoint> → <expected status> (<Action> gate)`. New cases for places: Manager→`places_create`/`places_rename`/`places_move`/`places_archive`/`places_delete` → 403 (D-20's Admin-only-mutate, the one place this differs from every existing `Mutate*` case in the file); Employee→`places_list_all`/`places_get` → 403 (`ReadPlaces` gate, same shape as existing `Employee→devices_list→403` cases already in the file per line 15).

---

## Shared Patterns

### Single-writer discipline
**Source:** `crates/trackly-app/src/services/device_service.rs:154-199` (`create`), `:202-213` (`get`)
**Apply to:** `place_service.rs` — every mutation goes through `self.writer.execute(move |conn| { ... }).await`; every read goes through `tokio::task::spawn_blocking(move || { readers.acquire(); ... })`. This is non-negotiable per CLAUDE.md's SQLite WAL + single-writer architectural note.

### Authorization (with the D-20 deviation)
**Source:** `crates/trackly-core/src/auth.rs:88-164` (full `Action`/`authorize` read)
**Apply to:** `tauri_cmds/places.rs`, `http/places.rs`. **Deviation from every other entity:** mutation gate is `Action::MutatePlaces` (Admin-only, joins `ManageUsers`/`ManageSettings` bucket), NOT `Action::MutateDevices`'s Admin|Manager bucket. Read gate is `Action::ReadPlaces` (Admin|Manager, joins `ReadData` bucket) — this part IS the standard pattern. Flag any plan/task that copy-pastes the `MutateDevices` bucket for `MutatePlaces` as a D-20 violation (RESEARCH.md Common Pitfall 3).

### Dual-transport `build_*` helper
**Source:** `crates/trackly-app/src/tauri_cmds/devices.rs:27-113` + `crates/trackly-app/src/http/devices.rs:168-181,380-393`
**Apply to:** every new places endpoint — one `build_places_*` function in `tauri_cmds/places.rs` called by both the `#[tauri::command]` wrapper (same file) and the axum handler (`http/places.rs`). Never duplicate business logic across transports.

### `specta_export.rs` registration
**Source:** `crates/trackly-app/src/specta_export.rs:1-12,16-40`
**Apply to:** every new `places_*` Tauri command — must be added to `collect_commands![...]` or the frontend never sees it via `bindings.ts`. Documented as a recurring gap in the file's own doc-comment ("Code-review checklist (T-05-06)").

### Soft-delete `standard4` columns
**Source:** `migrations/V002__core_entities.sql` (`locations` table definition)
**Apply to:** `places` table — `created_at_utc`, `updated_at_utc`, `deleted_at_utc`, `version` (optimistic-lock CAS), all `NOT NULL` except `deleted_at_utc`. Confirmed as the project-wide convention across every entity examined this session (devices, acts, cartridges, locations).

### FTS5 external-content sync triggers (apply to intrinsic fields ONLY, not place text)
**Source:** `migrations/V013__devices_fts_triggers.sql` (full read) + `migrations/V016__cartridges_kind_color_settings.sql:52-77`
**Apply to:** nothing new in this phase for `place_path` — this pattern is explicitly NOT extended to place text (RESEARCH.md Common Pitfall 1). It IS touched mechanically: `cartridges_fts_ai/ad/au` triggers must be `DROP TRIGGER` + `CREATE TRIGGER`'d without the `location` column (refinery can't `ALTER TRIGGER`), followed by `INSERT INTO cartridges_fts(cartridges_fts) VALUES('rebuild')`.

### Cyrillic case-fold gap in `LIKE`
**Source:** RESEARCH.md Common Pitfall 2 (SQLite docs, cited not verified against this exact bundled build)
**Apply to:** `PlaceService`'s search method AND `PlacePicker.svelte`'s search-mode data source — do substring matching in Rust (`.to_lowercase()`) against a small fetched candidate set (`SELECT * FROM place_full_paths`), never `WHERE full_path LIKE '%...%'` in SQL for user-typed Cyrillic queries. `locations_autocomplete`'s existing `ESCAPE '\\'` pattern (`device_service.rs:417-464`) handles SQL-injection-via-LIKE-metacharacters correctly but does NOT solve the Cyrillic case-fold problem — do not treat it as a complete precedent for place search.

---

## Files Being Deleted / Gutted

### `ui/src/lib/components/LocationAutocomplete.svelte` — DELETED (D-17)

Full current content read (249 lines, quoted in relevant excerpts above). Consumers that reference it directly must be re-pointed to `PlacePicker.svelte`:
- `ui/src/features/devices/DeviceAutocompleteField.svelte` (lines 164, 235 — the `field === 'location'` special-case that does a parallel `apiCall<string[]>('locations_autocomplete', ...)` fetch, quoted above) — this special-case is removed entirely; devices' place field becomes a plain `PlacePicker` binding, not routed through the generic autocomplete-field component's location branch.
- `ui/src/features/reports/ReportsPage.svelte:452,454` (`const locs = await apiCall<string[]>('locations_autocomplete', { prefix: '' })`) and `ReportFilters.svelte:28` (`locations?: string[]` prop) — replaced by `PlacePicker` bound to a `place_id` filter value; report filter output columns rename `location_name`→`place_path` (ReportsPage.svelte has 9 `location_name` column-label occurrences at lines 131-183 — every one needs its `key` renamed).

### `crates/trackly-infra/src/repos/devices_sqlite.rs::resolve_location_id_in_tx` — GUTTED (D-18)

Full current implementation read (lines 145-172):
```rust
/// Разрешает строковое название расположения в `location_id`.
///
/// Если строка непустая:
///   - Создаёт запись в `locations` если не существует (INSERT OR IGNORE).
///   - Возвращает id существующей или только что созданной записи.
///
/// Если строка пустая / None — возвращает None.
pub fn resolve_location_id_in_tx(
    &self,
    tx: &rusqlite::Transaction<'_>,
    location: Option<&str>,
    now_utc: i64,
) -> Result<Option<i64>, AppError> {
    let name = match normalize_str(location) {
        Some(n) => n,
        None => return Ok(None),
    };
    tx.execute(
        "INSERT OR IGNORE INTO locations (name, created_at_utc, updated_at_utc) \
         VALUES (?1, ?2, ?2)",
        rusqlite::params![name, now_utc],
    ).map_err(map_rusqlite)?;
    let id: i64 = tx.query_row(
        "SELECT id FROM locations WHERE name = ?1",
        rusqlite::params![name],
        |r| r.get(0),
    ).map_err(map_rusqlite)?;
    Ok(Some(id))
}
```
This method is removed entirely (the `locations` table it targets is dropped in V038). **Every one of its 6+ call sites** (grepped: `device_service.rs::create` line 172, `act_service.rs` lines 273-1284 — 8 occurrences across create/update/bulk-return/per-item-override) must switch from "pass a name string, get-or-create an id" to "pass a validated `place_id: Option<i64>` the caller already resolved through `PlacePicker`." The replacement has no auto-create helper — `PlaceService::create` is the only path that creates a `places` row, and it is `MutatePlaces`-gated (Admin-only per D-20), unlike the old helper which had no role check at all (RESEARCH.md Common Pitfall 4).

### `crates/trackly-core/src/domain/devices.rs::AutocompleteField::Location` / `is_location()` — REMOVED

Current shape (lines 132-192, quoted in full above under Domain Pattern). `AutocompleteField` shrinks to `Name | Model | Specs | Kit | State` (drop `Location`); `is_location()` and its special-case handling in `devices_sqlite.rs` (lines ~846-865, `"Location is special: queries locations table via JOIN with context filtering"`, not fully re-read this session but confirmed present via grep) are deleted along with it — device place lookup moves entirely to `PlacePicker`/`places_*` endpoints, not the generic per-field devices-autocomplete mechanism.

### `/api/v1/locations_autocomplete` (HTTP) + `locations_autocomplete` (Tauri command) — REMOVED

`http/devices.rs:380-393,415-418` (`handler_locations_autocomplete`, router registration) and the corresponding Tauri command in `tauri_cmds/devices.rs` (registered in `specta_export.rs:31` as `crate::tauri_cmds::devices::locations_autocomplete`) are deleted. Replaced by the new `places_search`/`places_list_all` endpoints on both transports.

---

## No Analog Found

None. Every file in this phase's blast radius has either a direct structural analog (the devices five-file pattern, RequestsMasterDetail, LocationAutocomplete) or is an existing file being mechanically modified (its own prior version is its analog).

---

## Metadata

**Analog search scope:** `crates/trackly-core/src/{domain,ports}/`, `crates/trackly-core/src/auth.rs`, `crates/trackly-infra/src/repos/`, `crates/trackly-app/src/{services,tauri_cmds,http,dto}/`, `crates/trackly-app/src/specta_export.rs`, `migrations/V002,V004,V005,V012,V013,V016.sql`, `ui/src/{routes.ts,features/layout/sidebar-config.ts,lib/components/{LocationAutocomplete,Dropdown}.svelte,features/{devices,reports,requests}/}`, `crates/trackly-app/tests/`, `crates/trackly-infra/tests/`
**Files scanned:** 26 read directly this session (several targeted/partial reads), plus ~15 grepped for line-level confirmation
**Pattern extraction date:** 2026-08-22

