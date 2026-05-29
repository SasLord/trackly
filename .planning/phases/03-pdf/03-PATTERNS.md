# Phase 3: Акты приёма-передачи и первая PDF-печать — Pattern Map

**Mapped:** 2026-05-29
**Files analyzed:** ~60 новых файлов (backend + frontend + ассеты + тесты)
**Analogs found:** 48 / 60 (12 файлов = NEW PATTERN — DocSpec, krilla, MiniJinja, шаблоны, шрифты, org.json, фикстуры)

> Все ссылки на путях — абсолютные. Все сниппеты приведены ровно в той форме, в которой исполнитель их копирует/адаптирует. Колонка "Match Quality": **exact** = тот же role + тот же data-flow (можно скопировать структуру 1-к-1), **role-match** = тот же role, иной data-flow (структуру повторить, тело переписать), **NEW PATTERN** = аналога нет, ниже даётся минимальная идиоматичная форма, согласованная с workspace conventions.

---

## File Classification

### Backend — `crates/trackly-core/`

| New / Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---------------------|------|-----------|----------------|---------------|
| `crates/trackly-core/src/domain/acts.rs` | domain | typed value | `/Users/madsas/Projects/trackly/crates/trackly-core/src/domain/devices.rs` | exact |
| `crates/trackly-core/src/domain/mod.rs` (модификация — `pub mod acts;`) | module | re-export | `/Users/madsas/Projects/trackly/crates/trackly-core/src/domain/mod.rs` | exact |
| `crates/trackly-core/src/ports/acts.rs` | port (trait) | repo trait | `/Users/madsas/Projects/trackly/crates/trackly-core/src/ports/devices.rs` | exact |
| `crates/trackly-core/src/ports/mod.rs` (модификация — `pub mod acts;`) | module | re-export | `/Users/madsas/Projects/trackly/crates/trackly-core/src/ports/mod.rs` | exact |

### Backend — `crates/trackly-infra/`

| New / Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---------------------|------|-----------|----------------|---------------|
| `migrations/V014__acts_audit_indexes.sql` (новая, опц.) | migration | DDL | `/Users/madsas/Projects/trackly/migrations/V008__audit_log.sql` + `V012__indexes_and_fts.sql` | role-match |
| `crates/trackly-infra/src/repos/acts_sqlite.rs` | repository | CRUD + tx | `/Users/madsas/Projects/trackly/crates/trackly-infra/src/repos/devices_sqlite.rs` | exact |
| `crates/trackly-infra/src/repos/audit_log_sqlite.rs` (новая, тонкая) | repository | insert + select-by-payload | `/Users/madsas/Projects/trackly/crates/trackly-infra/src/repos/devices_sqlite.rs` (методы `*_in_tx`) | role-match |
| `crates/trackly-infra/src/repos/mod.rs` (модификация) | module | re-export | `/Users/madsas/Projects/trackly/crates/trackly-infra/src/repos/mod.rs` | exact |

### Backend — `crates/trackly-app/` (services + DTOs + commands + http)

| New / Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---------------------|------|-----------|----------------|---------------|
| `crates/trackly-app/src/services/act_service.rs` | service | mutate + tx + counter | `/Users/madsas/Projects/trackly/crates/trackly-app/src/services/device_service.rs` | exact (CRUD), role-match (counter + return lifecycle) |
| `crates/trackly-app/src/services/organization_service.rs` | service | file-I/O read | `/Users/madsas/Projects/trackly/crates/trackly-app/src/csv/decode.rs` (file decode) + `Paths::exe_dir()` usage | role-match (нет прямого analog для JSON-file read; см. NEW PATTERN ниже) |
| `crates/trackly-app/src/services/template_service.rs` | service | DB read + seed | `/Users/madsas/Projects/trackly/crates/trackly-app/src/services/device_service.rs` (read paths) | role-match |
| `crates/trackly-app/src/services/mod.rs` (модификация) | module | re-export | `/Users/madsas/Projects/trackly/crates/trackly-app/src/services/mod.rs` | exact |
| `crates/trackly-app/src/dto/act.rs` | DTO | serde + specta + From<Row> | `/Users/madsas/Projects/trackly/crates/trackly-app/src/dto/device.rs` | exact |
| `crates/trackly-app/src/dto/organization.rs` | DTO | serde + specta | `/Users/madsas/Projects/trackly/crates/trackly-app/src/dto/health.rs` | role-match |
| `crates/trackly-app/src/dto/doc_spec.rs` | DTO | serde value-AST | `/Users/madsas/Projects/trackly/crates/trackly-app/src/dto/health.rs` | NEW PATTERN (enum AST) |
| `crates/trackly-app/src/dto/mod.rs` (модификация) | module | re-export | `/Users/madsas/Projects/trackly/crates/trackly-app/src/dto/mod.rs` | exact |
| `crates/trackly-app/src/tauri_cmds/acts.rs` | command | request-response | `/Users/madsas/Projects/trackly/crates/trackly-app/src/tauri_cmds/devices.rs` | exact |
| `crates/trackly-app/src/tauri_cmds/organization.rs` | command | request-response | `/Users/madsas/Projects/trackly/crates/trackly-app/src/tauri_cmds/devices.rs` | exact (тонкая) |
| `crates/trackly-app/src/tauri_cmds/templates.rs` | command | request-response | `/Users/madsas/Projects/trackly/crates/trackly-app/src/tauri_cmds/devices.rs` | exact (тонкая) |
| `crates/trackly-app/src/tauri_cmds/mod.rs` (модификация) | module | re-export | `/Users/madsas/Projects/trackly/crates/trackly-app/src/tauri_cmds/mod.rs` | exact |
| `crates/trackly-app/src/http/acts.rs` | http handler | POST adapter | `/Users/madsas/Projects/trackly/crates/trackly-app/src/http/devices.rs` | exact |
| `crates/trackly-app/src/http/organization.rs` | http handler | POST adapter | `/Users/madsas/Projects/trackly/crates/trackly-app/src/http/devices.rs` | exact (тонкая) |
| `crates/trackly-app/src/http/templates.rs` | http handler | POST adapter | `/Users/madsas/Projects/trackly/crates/trackly-app/src/http/devices.rs` | exact (тонкая) |
| `crates/trackly-app/src/http/mod.rs` (модификация) | module | re-export | `/Users/madsas/Projects/trackly/crates/trackly-app/src/http/mod.rs` | exact |
| `crates/trackly-app/src/context.rs` (модификация) | composition root | wiring | `/Users/madsas/Projects/trackly/crates/trackly-app/src/context.rs` | exact (расширение D-AppCtx-Extension) |
| `crates/trackly-app/src/specta_export.rs` (модификация) | binding builder | `collect_commands!` | `/Users/madsas/Projects/trackly/crates/trackly-app/src/specta_export.rs` | exact |
| `crates/trackly-app/src/lib.rs` (модификация — `pub mod pdf;`) | module | re-export | `/Users/madsas/Projects/trackly/crates/trackly-app/src/lib.rs` | exact |

### Backend — `crates/trackly-app/src/pdf/` (NEW PATTERN block)

| New File | Role | Data Flow | Analog | Match Quality |
|----------|------|-----------|--------|---------------|
| `crates/trackly-app/src/pdf/mod.rs` | module | re-export | `crates/trackly-app/src/csv/mod.rs` (структура mod + re-export) | role-match |
| `crates/trackly-app/src/pdf/renderer.rs` | renderer | DocSpec → Vec<u8> | — | NEW PATTERN |
| `crates/trackly-app/src/pdf/docspec.rs` | DTO/AST | serde value tree | `crates/trackly-app/src/dto/device.rs` (#[derive(Serialize, Deserialize, Type)] подход) | role-match |
| `crates/trackly-app/src/pdf/fonts.rs` | constants | `include_bytes!` | — | NEW PATTERN |
| `crates/trackly-app/src/pdf/minijinja_env.rs` | wrapper | Environment factory | — | NEW PATTERN |

### Backend — assets / templates

| New File | Role | Data Flow | Analog | Match Quality |
|----------|------|-----------|--------|---------------|
| `crates/trackly-app/assets/fonts/DejaVuSans.ttf` | binary asset | `include_bytes!` source | — | NEW PATTERN |
| `crates/trackly-app/assets/fonts/DejaVuSans-Bold.ttf` | binary asset | `include_bytes!` source | — | NEW PATTERN |
| `crates/trackly-app/templates/act_handover.minijinja` | text template | `include_str!` source | — | NEW PATTERN |
| `crates/trackly-app/templates/act_acceptance.minijinja` | text template | `include_str!` source | — | NEW PATTERN |
| `org.json.example` (репо-уровень docs) | sample config | static | — | NEW PATTERN |

### Backend — integration tests

| New File | Role | Data Flow | Analog | Match Quality |
|----------|------|-----------|--------|---------------|
| `crates/trackly-app/tests/acts_crud.rs` | test | service + readers | `/Users/madsas/Projects/trackly/crates/trackly-app/tests/devices_crud.rs` | exact |
| `crates/trackly-app/tests/acts_returns.rs` | test | service + tx | `/Users/madsas/Projects/trackly/crates/trackly-app/tests/devices_crud.rs` + `concurrent_writes.rs` | exact |
| `crates/trackly-app/tests/acts_undo.rs` | test | audit_log replay | `/Users/madsas/Projects/trackly/crates/trackly-app/tests/devices_crud.rs` | role-match |
| `crates/trackly-app/tests/acts_search.rs` | test | FTS5 | `/Users/madsas/Projects/trackly/crates/trackly-app/tests/devices_search.rs` | exact |
| `crates/trackly-app/tests/acts_numbering.rs` | test | concurrent counter | `/Users/madsas/Projects/trackly/crates/trackly-app/tests/concurrent_writes.rs` | exact |
| `crates/trackly-app/tests/acts_http_smoke.rs` | test | axum smoke | `/Users/madsas/Projects/trackly/crates/trackly-app/tests/devices_http_smoke.rs` | exact |
| `crates/trackly-app/tests/pdf_determinism.rs` | test | render + sha256 | — | NEW PATTERN |
| `crates/trackly-app/tests/pdf_text_extract.rs` | test | render + pdf-extract | — | NEW PATTERN |
| `crates/trackly-app/tests/templates_seed.rs` | test | startup seed idempotency | `/Users/madsas/Projects/trackly/crates/trackly-infra/tests/seed_data.rs` | role-match |
| `crates/trackly-app/tests/fixtures/act_42.json` | fixture | static JSON | — | NEW PATTERN |
| `crates/trackly-app/tests/fixtures/act_42.sha256` | fixture | static hex | — | NEW PATTERN |

### Frontend — `ui/src/features/acts/`

| New / Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---------------------|------|-----------|----------------|---------------|
| `ui/src/features/acts/ActsPage.svelte` | page shell | runes + onMount + apiCall | `/Users/madsas/Projects/trackly/ui/src/features/devices/DevicesPage.svelte` | exact (структура), role-match (master-detail вместо list-only) |
| `ui/src/features/acts/ActsList.svelte` | list view | runes + props | `/Users/madsas/Projects/trackly/ui/src/features/devices/DeviceList.svelte` | exact |
| `ui/src/features/acts/ActListRow.svelte` | row | display + click | `/Users/madsas/Projects/trackly/ui/src/features/devices/DeviceListRow.svelte` | role-match (двухстрочная карточка вместо таблицы) |
| `ui/src/features/acts/ActDetail.svelte` | detail panel | display | `/Users/madsas/Projects/trackly/ui/src/features/devices/DeviceFormBody.svelte` (read-only зона) | role-match |
| `ui/src/features/acts/ActHeaderField.svelte` | display field | label+value | — | NEW PATTERN (тривиальный) |
| `ui/src/features/acts/ActItemsTable.svelte` | table | display | `/Users/madsas/Projects/trackly/ui/src/features/devices/DeviceList.svelte` (table layout) | role-match |
| `ui/src/features/acts/ActFormModal.svelte` | modal shell | Modal + form | `/Users/madsas/Projects/trackly/ui/src/features/devices/DeviceFormModal.svelte` | exact |
| `ui/src/features/acts/ActFormBody.svelte` | form body | runes form | `/Users/madsas/Projects/trackly/ui/src/features/devices/DeviceFormBody.svelte` | exact |
| `ui/src/features/acts/ActFormItemsTable.svelte` | inline-editable table | runes + autocomplete | `/Users/madsas/Projects/trackly/ui/src/features/devices/DeviceFormBody.svelte` (form-state runes) | role-match |
| `ui/src/features/acts/ActNumberField.svelte` | specialized input | runes + debounce | `/Users/madsas/Projects/trackly/ui/src/features/devices/DeviceAutocompleteField.svelte` (debounce + state) | role-match |
| `ui/src/features/acts/ReturnModal.svelte` | modal shell | Modal + form | `/Users/madsas/Projects/trackly/ui/src/features/devices/DeviceFormModal.svelte` | role-match |
| `ui/src/features/acts/ReturnItemsTable.svelte` | inline-editable table | runes + checkboxes | `/Users/madsas/Projects/trackly/ui/src/features/devices/DeviceList.svelte` | role-match |
| `ui/src/features/acts/PdfPreviewModal.svelte` | modal shell + iframe | blob URL | `/Users/madsas/Projects/trackly/ui/src/features/devices/DeviceImportCsvModal.svelte` (Modal + loading state pattern) | role-match |
| `ui/src/features/acts/DocumentAcceptanceModal.svelte` | modal shell | Modal + form (DEV-14) | `/Users/madsas/Projects/trackly/ui/src/features/devices/DeviceFormModal.svelte` | role-match |
| `ui/src/features/acts/api.ts` | re-export | feature barrel | `/Users/madsas/Projects/trackly/ui/src/features/devices/api.ts` | exact |
| `ui/src/lib/api/acts.ts` | api wrapper | apiCall<R> | `/Users/madsas/Projects/trackly/ui/src/lib/api/devices.ts` | exact |
| `ui/src/lib/api/organization.ts` | api wrapper | apiCall<R> | `/Users/madsas/Projects/trackly/ui/src/lib/api/devices.ts` | exact |
| `ui/src/lib/api/pdf.ts` | api wrapper | apiCall<R> → Blob | `/Users/madsas/Projects/trackly/ui/src/lib/api/devices.ts` | role-match (Vec<u8> → Blob) |
| `ui/src/lib/api/templates.ts` | api wrapper | apiCall<R> | `/Users/madsas/Projects/trackly/ui/src/lib/api/devices.ts` | exact |
| `ui/src/pages/ActsPage.svelte` (модификация — заменить placeholder на import) | route shell | — | `/Users/madsas/Projects/trackly/ui/src/features/devices/DevicesPage.svelte` (паттерн «feature-folder owns the page; pages/ — однострочный re-export») | exact |
| `ui/src/lib/components/Modal.svelte` (МОДИФИКАЦИЯ — расширение `size`) | shared component | extend prop | `/Users/madsas/Projects/trackly/ui/src/lib/components/Modal.svelte` | exact (расширение enum-литерала) |
| `ui/src/features/devices/DeviceAutocompleteField.svelte` (модификация — добавить `statusIn` prop) | shared component | extend prop | `/Users/madsas/Projects/trackly/ui/src/features/devices/DeviceAutocompleteField.svelte` | exact |
| `ui/src/features/devices/DeviceContextMenu.svelte` (модификация — добавить пункт «Печать документа приёма») | shared component | extend menu | `/Users/madsas/Projects/trackly/ui/src/features/devices/DeviceContextMenu.svelte` | exact |
| `ui/src/features/layout/sidebar-config.ts` (модификация — снять placeholder с «Акты») | config | edit data | `/Users/madsas/Projects/trackly/ui/src/features/layout/sidebar-config.ts` | exact |

---

## Pattern Assignments

### A. Backend — domain layer (`trackly-core`)

#### `crates/trackly-core/src/domain/acts.rs` (domain, typed value)

**Analog:** `/Users/madsas/Projects/trackly/crates/trackly-core/src/domain/devices.rs`

**Imports / module header pattern** (lines 1-14 в аналоге):

```rust
//! Domain value types for the Acts entity.
//!
//! NO serde::Serialize/Deserialize or specta::Type derives here — those live
//! in the DTO layer in trackly-app. Only `#[derive(Debug, Clone, PartialEq, Eq)]`.

use crate::error::AppError;
```

**Core pattern — type shape** (повторить структуру `DeviceNew`/`DevicePatch`/`DeviceFilter`/`DeviceRow`/`Pagination` lines 16-93):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActNew {
    pub act_type: ActType,            // ActType = Handover | Return
    pub number_override: Option<i64>, // None = auto-increment counter
    pub parent_act_id: Option<i64>,   // только для Return
    pub giver_name: String,
    pub receiver_name: String,
    pub location_id: Option<i64>,
    pub notes: Option<String>,
    pub items: Vec<ActItemNew>,
    pub deadline_utc: Option<i64>,    // «Сроком до» (D-Acts-Create-01)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActItemNew {
    pub device_id: i64,
    pub quantity: i64,
    pub condition_at_time: Option<String>,
    pub complectation_at_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActRow {
    pub id: i64,
    pub number: i64,
    pub sub_number: Option<i64>,
    pub parent_act_id: Option<i64>,
    pub act_type: ActType,
    pub giver_name: String,
    pub receiver_name: String,
    pub location_id: Option<i64>,
    pub notes: Option<String>,
    pub archived: bool,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub deleted_at_utc: Option<i64>,
    pub version: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActType { Handover, Return }

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActFilter {
    pub act_type: Option<ActType>,
    pub archived: Option<bool>,
    pub search: Option<String>,         // free-text → service строит FTS query
    pub include_deleted: bool,
}
```

**Validation enum** — повторить паттерн `AutocompleteField::from_str` (lines 129-148 в `devices.rs`) для `ActType::from_str(&str) -> Result<Self, AppError>` (returns `AppError::Validation`).

---

#### `crates/trackly-core/src/ports/acts.rs` (port, repo trait)

**Analog:** `/Users/madsas/Projects/trackly/crates/trackly-core/src/ports/devices.rs`

**Imports / module-doc** (lines 1-13):

```rust
//! `ActRepository` port — repository trait for the Acts entity.
//!
//! Pattern: associated `type Conn` keeps rusqlite out of trackly-core.
//! The concrete type (`rusqlite::Connection`) is specified in the adapter
//! impl in `trackly-infra::repos::acts_sqlite`.

use crate::domain::acts::{ActFilter, ActNew, ActPatch, ActRow, Pagination, ReturnItem};
use crate::error::AppError;
```

**Core pattern — trait** (lines 19-97):

```rust
pub trait ActRepository {
    type Conn;
    fn create(&self, conn: &mut Self::Conn, new: &ActNew, now_utc: i64) -> Result<i64, AppError>;
    fn get(&self, conn: &Self::Conn, id: i64) -> Result<ActRow, AppError>;
    fn list(&self, conn: &Self::Conn, filter: &ActFilter, page: &Pagination) -> Result<(Vec<ActRow>, u64), AppError>;
    fn update(&self, conn: &mut Self::Conn, id: i64, version: i64, patch: &ActPatch, now_utc: i64) -> Result<ActRow, AppError>;
    fn delete_soft(&self, conn: &mut Self::Conn, id: i64, version: i64, now_utc: i64) -> Result<(), AppError>;
    fn search_fts(&self, conn: &Self::Conn, fts_query: &str, filter: &ActFilter, page: &Pagination) -> Result<(Vec<ActRow>, u64), AppError>;
    // Acts-specific:
    fn peek_next_number(&self, conn: &Self::Conn) -> Result<i64, AppError>;
    fn counts(&self, conn: &Self::Conn) -> Result<ActCounts, AppError>; // {handover_active, returns, archived}
    fn list_returns_for_parent(&self, conn: &Self::Conn, parent_act_id: i64) -> Result<Vec<ActRow>, AppError>;
}
```

Метод `create` НЕ инкрементирует counter сам — это делает сервис в выделенном tx-helper-методе (см. ниже). Trait описывает «pure SQL» операции.

---

### B. Backend — infra layer (`trackly-infra`)

#### `crates/trackly-infra/src/repos/acts_sqlite.rs` (repository, CRUD + tx)

**Analog:** `/Users/madsas/Projects/trackly/crates/trackly-infra/src/repos/devices_sqlite.rs`

**Imports pattern** (lines 1-23):

```rust
use rusqlite::{Connection, OptionalExtension};
use trackly_core::domain::acts::{ActFilter, ActNew, ActPatch, ActRow, Pagination};
use trackly_core::error::AppError;
use trackly_core::ports::acts::ActRepository;

use crate::error_conversions::map_rusqlite;
```

**SQL-constant pattern** (lines 28-37 — `const SELECT_DEVICES`):

```rust
const SELECT_ACTS: &str = "
    SELECT a.id, a.number, a.sub_number, a.parent_act_id, a.act_type,
           a.giver_name, a.receiver_name, a.location_id, a.notes,
           a.archived, a.created_at_utc, a.updated_at_utc, a.deleted_at_utc, a.version,
           l.name AS location_name,
           p.number AS parent_number,
           (SELECT COUNT(*) FROM acts r
              WHERE r.parent_act_id = a.parent_act_id
                AND r.deleted_at_utc IS NULL) AS sibling_return_count
    FROM acts a
    LEFT JOIN locations l ON a.location_id = l.id
    LEFT JOIN acts p ON p.id = a.parent_act_id
";
```

**Row mapper pattern** (lines 41-60 в аналоге — `fn from_row`):

```rust
fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActRow> {
    Ok(ActRow {
        id: row.get(0)?, number: row.get(1)?, sub_number: row.get(2)?,
        parent_act_id: row.get(3)?,
        act_type: parse_act_type(row.get::<_, String>(4)?),
        giver_name: row.get(5)?, receiver_name: row.get(6)?,
        location_id: row.get(7)?, notes: row.get(8)?,
        archived: row.get::<_, i64>(9)? == 1,
        created_at_utc: row.get(10)?, updated_at_utc: row.get(11)?,
        deleted_at_utc: row.get(12)?, version: row.get(13)?,
    })
}
```

**FTS helper** (lines 75-83 — `build_fts_query`) — **повторить 1-к-1** в `act_service.rs` (или общем utility) для D-Search-01 расширения на акты. Это уже работающий sanitizer:

```rust
fn build_fts_query(user_input: &str) -> String {
    user_input.split_whitespace()
        .map(|t| t.replace('\0', "").replace('"', "\"\""))
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{t}\"*"))
        .collect::<Vec<_>>().join(" ")
}
```

**Tx-method pattern** (lines 95-150 в аналоге — `resolve_location_id_in_tx`, `create_in_tx`):

```rust
impl SqliteActRepository {
    /// INSERT в пределах транзакции. Возвращает новый act `id`.
    /// Counter increment и audit_log делает сервис в той же tx — НЕ здесь.
    pub fn insert_act_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        new: &ActRow,           // already-resolved row (number может быть override OR из counter)
    ) -> Result<i64, AppError> {
        tx.execute(
            "INSERT INTO acts (number, sub_number, parent_act_id, act_type, giver_name, \
             receiver_name, location_id, notes, archived, created_at_utc, updated_at_utc, version) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10, 1)",
            rusqlite::params![
                new.number, new.sub_number, new.parent_act_id,
                act_type_to_sql(new.act_type),
                new.giver_name, new.receiver_name,
                new.location_id, new.notes,
                if new.archived { 1 } else { 0 },
                new.created_at_utc,
            ],
        ).map_err(map_rusqlite)?;
        Ok(tx.last_insert_rowid())
    }

    pub fn insert_act_item_in_tx(
        &self,
        tx: &rusqlite::Transaction<'_>,
        act_id: i64,
        device_id: i64,
        condition_at_time: Option<&str>,
        complectation_at_time: Option<&str>,
    ) -> Result<(), AppError> {
        tx.execute(
            "INSERT INTO act_items (act_id, device_id, condition_at_time, complectation_at_time) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![act_id, device_id, condition_at_time, complectation_at_time],
        ).map_err(map_rusqlite)?;
        Ok(())
    }
}
```

**Counter atomic-increment helper** (NEW PATTERN, использует V009.counters):

```rust
/// Атомарный инкремент именованного счётчика. Должен вызываться внутри
/// `BEGIN IMMEDIATE` transaction (которую начинает caller через `conn.transaction()`).
/// SQLite + single-writer + RETURNING ⇒ полная гарантия (Pitfall 1).
pub fn increment_counter_in_tx(tx: &rusqlite::Transaction<'_>, name: &str) -> Result<i64, AppError> {
    tx.query_row(
        "UPDATE counters SET current_value = current_value + 1 \
         WHERE name = ?1 RETURNING current_value",
        rusqlite::params![name],
        |r| r.get::<_, i64>(0),
    ).map_err(map_rusqlite)
}

/// Peek (read-only) для UI «предсказать следующий номер». НЕ инкрементирует.
pub fn peek_counter(conn: &impl rusqlite::types::FromSql, name: &str) -> Result<i64, AppError> {
    // ... SELECT current_value FROM counters WHERE name=?
}
```

---

#### `crates/trackly-infra/src/repos/audit_log_sqlite.rs` (repository, insert + JSON1 select)

**Analog (closest):** методы `*_in_tx` в `devices_sqlite.rs` (тот же стиль — тонкий repo с `tx`-функциями).

**Imports / module pattern:**

```rust
use rusqlite::{Connection, OptionalExtension};
use trackly_core::error::AppError;
use crate::error_conversions::map_rusqlite;

#[derive(Debug, Default, Clone)]
pub struct SqliteAuditLogRepository;
```

**Core pattern — типизированный `AuditEntry` (NEW PATTERN, согласован с V008 schema):**

```rust
pub struct AuditEntry<'a> {
    pub entity_type: &'static str,         // "device" | "act" | …
    pub entity_id: i64,
    pub action: &'a str,                   // "create" | "update" | "delete" | "restore" | "custom:..."
    pub user_id: Option<i64>,              // Phase 3: всегда None
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    pub payload_json: Option<String>,      // {"act_id": N, "kind": "handover"|"return"}
    pub created_at_utc: i64,
}

impl SqliteAuditLogRepository {
    pub fn insert(&self, tx: &rusqlite::Transaction<'_>, e: AuditEntry<'_>) -> Result<(), AppError> {
        tx.execute(
            "INSERT INTO audit_log \
             (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                e.entity_type, e.entity_id, e.action, e.user_id,
                e.before_json, e.after_json, e.payload_json, e.created_at_utc,
            ],
        ).map_err(map_rusqlite)?;
        Ok(())
    }

    /// Используется в undo: восстановление state из before_json по `payload_json.act_id`.
    /// SQLite JSON1 (`json_extract`) доступен в bundled rusqlite — verified.
    pub fn select_device_mutations_for_act(
        &self, tx: &rusqlite::Transaction<'_>, act_id: i64,
    ) -> Result<Vec<(i64, String)>, AppError> {
        let mut stmt = tx.prepare(
            "SELECT entity_id, before_json FROM audit_log \
             WHERE entity_type = 'device' \
               AND json_extract(payload_json, '$.act_id') = ?1 \
               AND before_json IS NOT NULL \
             ORDER BY created_at_utc ASC, id ASC",
        ).map_err(map_rusqlite)?;
        let rows = stmt.query_map([act_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        }).map_err(map_rusqlite)?
          .collect::<Result<Vec<_>, _>>().map_err(map_rusqlite)?;
        Ok(rows)
    }
}
```

**Migration delta (V014):** добавить partial-index на `audit_log(entity_type, json_extract(payload_json,'$.act_id'))` если профилирование покажет медленный undo (опционально для plan; решение исполнителя).

---

#### `migrations/V014__acts_audit_indexes.sql` (опционально)

**Analog:** `/Users/madsas/Projects/trackly/migrations/V012__indexes_and_fts.sql` (CREATE INDEX...; PRAGMA user_version = 14;).

**Pattern:**

```sql
-- V014: Phase 3 supporting indexes (Acts + audit_log lookup).
CREATE INDEX IF NOT EXISTS idx_act_items_act_id ON act_items(act_id);
CREATE INDEX IF NOT EXISTS idx_act_items_device_id ON act_items(device_id);
CREATE INDEX IF NOT EXISTS idx_acts_parent_act_id ON acts(parent_act_id) WHERE parent_act_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_audit_log_entity ON audit_log(entity_type, entity_id, created_at_utc);

PRAGMA user_version = 14;
```

После добавления — `max_known_version()` станет 14, тест `run_applies_all_known_migrations_on_fresh_db` (`crates/trackly-infra/src/db/migrations.rs:90-95`) НУЖНО обновить с `13` на `14` (`assert_eq!(report.schema_version, 14)` × 2).

---

### C. Backend — service layer (`trackly-app`)

#### `crates/trackly-app/src/services/act_service.rs` (service, mutate + tx + counter)

**Analog:** `/Users/madsas/Projects/trackly/crates/trackly-app/src/services/device_service.rs`

**Struct + constructor pattern** (lines 45-72 в аналоге):

```rust
#[derive(Clone)]
pub struct ActService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) acts_repo: Arc<SqliteActRepository>,
    pub(crate) audit_repo: Arc<SqliteAuditLogRepository>,
    pub(crate) devices_repo: Arc<SqliteDeviceRepository>,
}

impl ActService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            writer, readers, clock,
            acts_repo: Arc::new(SqliteActRepository),
            audit_repo: Arc::new(SqliteAuditLogRepository),
            devices_repo: Arc::new(SqliteDeviceRepository),
        }
    }
}
```

**Write-path pattern** (lines 109-152 в аналоге — `create`):

```rust
pub async fn create(&self, payload: ActCreateDto) -> Result<ActDto, AppError> {
    Self::validate_new(&payload)?;
    let now = self.clock.unix_seconds();
    let acts_repo = self.acts_repo.clone();
    let audit_repo = self.audit_repo.clone();
    let devices_repo = self.devices_repo.clone();
    let user_id_opt: Option<i64> = None; // Phase 3 — no auth

    let id = self.writer.execute(move |conn| {
        let tx = conn.transaction().map_err(map_rusqlite)?;  // BEGIN IMMEDIATE по умолчанию для rusqlite Transaction

        // 1. Resolve number: override OR atomic counter increment
        let number = if let Some(custom) = payload.number_override {
            // Check uniqueness INCLUDING soft-deleted (D-Soft-vs-Hard-Acts-01)
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM acts WHERE number=?1 LIMIT 1)",
                rusqlite::params![custom], |r| r.get(0),
            ).map_err(map_rusqlite)?;
            if exists {
                return Err(AppError::Conflict {
                    reason: format!("Акт №{custom} уже существует"),
                });
            }
            // Audit override (per D-Counter-Acts-01)
            let next_auto = peek_counter(&tx, "act_number")? + 1;
            audit_repo.insert(&tx, AuditEntry {
                entity_type: "act", entity_id: 0,        // patched after INSERT в отдельной audit-записи
                action: "custom:act_number_override",
                user_id: user_id_opt,
                before_json: None, after_json: None,
                payload_json: Some(serde_json::json!({
                    "requested": custom, "next_auto_would_be": next_auto,
                }).to_string()),
                created_at_utc: now,
            })?;
            custom
        } else {
            increment_counter_in_tx(&tx, "act_number")?
        };

        // 2. INSERT act
        let new_row = ActRow {
            id: 0, number, sub_number: None, parent_act_id: None,
            act_type: ActType::Handover, giver_name: payload.giver_name.clone(),
            receiver_name: payload.receiver_name.clone(),
            location_id: payload.location_id, notes: payload.notes.clone(),
            archived: false, created_at_utc: now, updated_at_utc: now,
            deleted_at_utc: None, version: 1,
        };
        let act_id = acts_repo.insert_act_in_tx(&tx, &new_row)?;

        // 3. INSERT act_items + UPDATE devices + audit each (D-Undo-01 schema)
        for item in &payload.items {
            let before = devices_repo.get_in_tx(&tx, item.device_id)?;
            acts_repo.insert_act_item_in_tx(&tx, act_id, item.device_id,
                before.state.as_deref(), before.kit.as_deref())?;
            // status='в_работе' lookup id (или передан в сервис как const) + location_id
            let after = devices_repo.update_status_and_location_in_tx(
                &tx, item.device_id, IN_WORK_STATUS_ID, payload.location_id, now)?;
            audit_repo.insert(&tx, AuditEntry {
                entity_type: "device", entity_id: item.device_id,
                action: "update", user_id: user_id_opt,
                before_json: Some(serde_json::to_string(&before).map_err(internal)?),
                after_json: Some(serde_json::to_string(&after).map_err(internal)?),
                payload_json: Some(serde_json::json!({
                    "act_id": act_id, "kind": "handover",
                }).to_string()),
                created_at_utc: now,
            })?;
        }

        // 4. Final audit for act creation
        let act_after = acts_repo.fetch_full_in_tx(&tx, act_id)?;
        audit_repo.insert(&tx, AuditEntry {
            entity_type: "act", entity_id: act_id, action: "create",
            user_id: user_id_opt, before_json: None,
            after_json: Some(serde_json::to_string(&act_after).map_err(internal)?),
            payload_json: None,
            created_at_utc: now,
        })?;

        tx.commit().map_err(map_rusqlite)?;
        Ok(act_id)
    }).await?;

    self.get(id).await
}
```

**Read-path pattern** (lines 154-166 в аналоге — `get`):

```rust
pub async fn get(&self, id: i64) -> Result<ActDto, AppError> {
    let readers = self.readers.clone();
    let repo = self.acts_repo.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        let row = repo.get(&conn, id)?;
        // ВАЖНО: format_act_number — pure-Rust, делаем в DTO::from_row
        // (а не в SQL CASE-WHEN). RESEARCH §Anti-Pattern.
        Ok(ActDto::from_row(row))
    }).await.map_err(|e| AppError::Internal {
        source_chain: format!("spawn_blocking: {e}"),
    })?
}
```

**Display-rule helper** (NEW PATTERN, pure Rust):

```rust
/// «42» / «42в» / «42в1» / «42в2» — D-Numbering-01.
pub fn format_act_number(
    act_type: ActType, number: i64,
    sub_number: Option<i64>, parent_number: Option<i64>,
    sibling_return_count: Option<i64>,
) -> String {
    match act_type {
        ActType::Handover => format!("{}", number),
        ActType::Return => {
            let sub = sub_number.expect("return must have sub_number");
            let parent = parent_number.expect("return must have parent_number");
            if sibling_return_count == Some(1) {
                format!("{}в", parent)
            } else {
                format!("{}в{}", parent, sub)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn format_handover_is_plain_number() {
        assert_eq!(format_act_number(ActType::Handover, 42, None, None, None), "42");
    }
    #[test]
    fn format_single_return_has_no_sub_suffix() {
        assert_eq!(
            format_act_number(ActType::Return, 999, Some(1), Some(42), Some(1)),
            "42в"
        );
    }
    #[test]
    fn format_multiple_returns_have_subscripts() {
        assert_eq!(
            format_act_number(ActType::Return, 999, Some(1), Some(42), Some(2)),
            "42в1"
        );
        assert_eq!(
            format_act_number(ActType::Return, 1000, Some(2), Some(42), Some(2)),
            "42в2"
        );
    }
}
```

**Error usage corrections (важно — сверка с реальным `AppError` из `crates/trackly-core/src/error.rs`):**

- `AppError::Conflict { reason: String }` — НЕ `{field, message}` (так в RESEARCH черновике; реальная enum-форма — одно поле `reason`). См. `crates/trackly-core/src/error.rs:46-49`.
- `AppError::NotFound { entity: &'static str, id: i64 }` — `entity` это `&'static str`, не `String` (важно при формировании сообщений). См. `crates/trackly-core/src/error.rs:37-42`.
- `AppError::Validation { field: String, message: String }` — стандарт для валидации форм (используется в `device_service.rs:79-95`).
- `AppError::Internal { source_chain: String }` — для spawn_blocking errors, JSON serde errors, неожиданных rusqlite ошибок (см. `device_service.rs:163-165`).

---

#### `crates/trackly-app/src/services/organization_service.rs` (service, file-I/O read)

**Analog (closest):** структуры `device_service.rs` (Arc-fields, `Clone`-derive); file-I/O делается через `Paths::exe_dir()` (см. `crates/trackly-infra/src/paths.rs:69-90`).

**NEW PATTERN — минимальная идиоматичная форма:**

```rust
use std::sync::Arc;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use trackly_core::error::AppError;
use trackly_infra::Paths;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrgData {
    pub name: String,
    pub inn: String,
    pub kpp: String,
    pub address: String,
    pub logo_path: String,
}

impl OrgData {
    fn placeholder() -> Self {
        Self {
            name: "Ваша организация".into(),
            inn: "0000000000".into(),
            kpp: "000000000".into(),
            address: "Укажите адрес в settings/org.json".into(),
            logo_path: "logo.png".into(),
        }
    }
}

#[derive(Clone)]
pub struct OrganizationService {
    paths: Arc<Paths>,
}

impl OrganizationService {
    pub fn new(paths: Arc<Paths>) -> Self { Self { paths } }

    fn file_path(&self) -> PathBuf {
        // ВАЖНО: НИКАКОГО `dirs::*_dir()` — портабельная дисциплина.
        self.paths.exe_dir().join("org.json")
    }

    /// Phase 3: чтение на запрос (no caching). Phase 7 — file-watch.
    pub async fn read(&self) -> Result<OrgData, AppError> {
        let path = self.file_path();
        tokio::task::spawn_blocking(move || {
            if !path.exists() {
                // first-run: создаём placeholder, логируем warning
                let placeholder = OrgData::placeholder();
                let bytes = serde_json::to_vec_pretty(&placeholder)
                    .map_err(|e| AppError::Internal { source_chain: format!("org.json placeholder serialize: {e}") })?;
                std::fs::write(&path, bytes)
                    .map_err(|e| AppError::Internal { source_chain: format!("org.json write placeholder: {e}") })?;
                tracing::warn!(?path, "org.json не найден — создан placeholder");
                return Ok(placeholder);
            }
            let bytes = std::fs::read(&path)
                .map_err(|e| AppError::Internal { source_chain: format!("org.json read: {e}") })?;
            serde_json::from_slice::<OrgData>(&bytes)
                .map_err(|e| AppError::Validation {
                    field: "org.json".into(),
                    message: format!("Не удалось распарсить org.json: {e}"),
                })
        }).await.map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking: {e}") })?
    }

    /// Возвращает абсолютный путь к logo-файлу (`exe_dir/org.logo_path`).
    /// Не проверяет существование — krilla сам логирует warning при отсутствии.
    pub fn logo_abs_path(&self, org: &OrgData) -> PathBuf {
        self.paths.exe_dir().join(&org.logo_path)
    }
}
```

---

#### `crates/trackly-app/src/services/template_service.rs` (service, DB read + seed)

**Analog (read paths):** `device_service.rs::get` / `list`.

**Pattern:**

```rust
pub const DEFAULT_TEMPLATES: &[(&str, &str)] = &[
    ("act_handover",   include_str!("../../templates/act_handover.minijinja")),
    ("act_acceptance", include_str!("../../templates/act_acceptance.minijinja")),
];

#[derive(Clone)]
pub struct TemplateService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
}

impl TemplateService {
    /// Idempotent seed of default templates. Вызывается из AppCtx::build.
    /// При count(kind, not deleted) = 0 — INSERT дефолта; иначе skip.
    pub async fn seed_defaults_on_startup(&self) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        self.writer.execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            for (kind, body) in DEFAULT_TEMPLATES {
                let count: i64 = tx.query_row(
                    "SELECT COUNT(*) FROM document_templates \
                     WHERE kind = ?1 AND deleted_at_utc IS NULL",
                    rusqlite::params![kind], |r| r.get(0),
                ).map_err(map_rusqlite)?;
                if count == 0 {
                    tx.execute(
                        "INSERT INTO document_templates \
                         (kind, name, body_minijinja, is_active, created_at_utc, updated_at_utc, version) \
                         VALUES (?1, 'Дефолтный (v1)', ?2, 1, ?3, ?3, 1)",
                        rusqlite::params![kind, body, now],
                    ).map_err(map_rusqlite)?;
                }
            }
            tx.commit().map_err(map_rusqlite)?;
            Ok(())
        }).await
    }

    pub async fn get_active(&self, kind: &str) -> Result<String, AppError> { /* SELECT body_minijinja ... */ }
}
```

---

### D. Backend — DTO layer

#### `crates/trackly-app/src/dto/act.rs` (DTO, serde + specta + From<Row>)

**Analog:** `/Users/madsas/Projects/trackly/crates/trackly-app/src/dto/device.rs`

**Core pattern** (lines 32-82 в аналоге — `DeviceDto` + `From<DeviceRow>`):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ActDto {
    #[specta(type = i32)] pub id: i64,
    #[specta(type = i32)] pub version: i64,
    pub number: String,                 // formatted via format_act_number (D-Numbering-01)
    pub act_type: String,               // "handover" | "return"
    #[specta(type = Option<i32>)] pub parent_act_id: Option<i64>,
    pub giver_name: String,
    pub receiver_name: String,
    #[specta(type = Option<i32>)] pub location_id: Option<i64>,
    pub location: Option<String>,       // resolved name via LEFT JOIN
    pub notes: Option<String>,
    pub archived: bool,
    #[specta(type = i32)] pub created_at_utc: i64,
    #[specta(type = i32)] pub updated_at_utc: i64,
    pub items: Vec<ActItemDto>,
    pub returns: Vec<ActDto>,           // только в detail-load для handover
}

impl ActDto {
    /// Builder, который применяет format_act_number и подгружает joined fields.
    pub fn from_row_with_context(/* row, joined fields */) -> Self { /* ... */ }
}
```

**КРИТИЧНОЕ ОТЛИЧИЕ от device_service:** все `i64` поля (`id`, `version`, timestamps) обязаны иметь `#[specta(type = i32)]` — это паттерн из всего `dto/device.rs` (см. lines 33-59), без него specta-typescript падает (см. модуль-doc lines 30-32). Для **Vec<u8>** PDF — тоже специальный аттрибут: `#[specta(type = Vec<u8>)]` (или `Vec<i32>`). См. ниже PDF command sigs.

**Counts DTO:**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ActsCountsDto {
    pub handover_active: i64,    // #[specta(type = i32)] на всех полях
    pub returns: i64,
    pub archived: i64,
}
```

**Filter / pagination — повторить паттерн `DeviceFilter` (lines 192-216)** с `#[derive(Type, Default)]`.

---

#### `crates/trackly-app/src/dto/organization.rs`

**Analog:** `/Users/madsas/Projects/trackly/crates/trackly-app/src/dto/health.rs` (тонкий serializable DTO).

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OrgDto {
    pub name: String, pub inn: String, pub kpp: String,
    pub address: String, pub logo_path: String,
}

impl From<OrgData> for OrgDto { /* trivial */ }
```

---

#### `crates/trackly-app/src/dto/doc_spec.rs` (DTO, serde value-AST) — NEW PATTERN

**Analog (closest):** `crates/trackly-app/src/dto/device.rs` — общий стиль `#[derive(Serialize, Deserialize, Type)]`, но shape — value-AST, не плоский DTO. Это DocSpec-IR (D-PDF-Render-Path-01).

**Pattern:**

```rust
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DocSpec {
    pub title: String,
    pub header: HeaderBlock,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct HeaderBlock {
    pub org_name: String,
    pub org_inn: String,
    pub org_kpp: String,
    pub org_address: String,
    pub logo_path: Option<String>,     // absolute path; renderer читает файл
    pub act_label: String,             // «Акт приёма-передачи №42»
    pub date_label: String,            // «28 мая 2026 г.»
}

/// Все секции типизированы через tagged enum — нет произвольных строк PDF-ops.
/// `#[serde(tag = "type")]` — frontend-friendly discriminator.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Section {
    Paragraph { text: String, style: TextStyle },
    Heading { level: u8, text: String },
    KeyValueTable { rows: Vec<KvRow> },
    ItemsTable { columns: Vec<String>, rows: Vec<Vec<String>> },
    Signature { left_label: String, right_label: String, spacer_pt: f32 },
    Spacer { height_pt: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct KvRow { pub key: String, pub value: String }

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum TextStyle { Regular, Bold, Italic }
```

**Anti-pattern (RESEARCH §Anti-Patterns):** НЕ добавлять `raw_pdf_op: Vec<u8>` варианты — DocSpec обязан быть полностью typed.

---

### E. Backend — PDF subsystem (NEW PATTERN block)

#### `crates/trackly-app/src/pdf/mod.rs`

**Analog (closest):** `crates/trackly-app/src/csv/mod.rs` (стиль mod + re-export).

```rust
//! PDF rendering subsystem (Phase 3).
//!
//! 3-stage pipeline (D-PDF-Render-Path-01):
//! 1. `minijinja_env::render_with_timeout(template_src, ctx) -> String` (JSON)
//! 2. `serde_json::from_str::<DocSpec>(rendered)` (validation)
//! 3. `renderer::PdfRenderer::render_docspec(&spec) -> Vec<u8>`

pub mod docspec;
pub mod fonts;
pub mod minijinja_env;
pub mod renderer;

pub use renderer::PdfRenderer;
```

#### `crates/trackly-app/src/pdf/fonts.rs`

```rust
//! Cyrillic-safe fonts embedded at compile time via `include_bytes!`.
//! License: DejaVu Sans — public-domain-derived (Bitstream Vera).

pub static DEJAVU_SANS_REGULAR: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");
pub static DEJAVU_SANS_BOLD:    &[u8] = include_bytes!("../../assets/fonts/DejaVuSans-Bold.ttf");
```

#### `crates/trackly-app/src/pdf/minijinja_env.rs`

См. RESEARCH §Pattern 4 — копировать 1-к-1 (UndefinedBehavior::Strict, recursion_limit 64, fuel 100_000, без loader; render через `spawn_blocking + tokio::time::timeout(5s)`; mapping ошибок на `AppError::Validation { field: "template", message }`).

#### `crates/trackly-app/src/pdf/renderer.rs`

См. RESEARCH §Pattern 5 — `PdfRenderer` со встроенными font-bytes (`Arc<Vec<u8>>`) и `Arc<minijinja::Environment<'static>>`. Метод `render_docspec(&DocSpec) -> Result<Vec<u8>, AppError>` обходит секции, эмитит krilla `Document` / `surface.draw_text(...)`.

**Determinism note (Pitfall 4):** в plan 01 ОБЯЗАТЕЛЬНО проверить:
1. `Document::new()` — не вставляет ли timestamp в `/Info` dict.
2. Если krilla даёт API `set_creation_date(None)` / `set_modify_date(None)` — использовать.
3. Если нет — обёрнуть `doc.finish()` постпроцессом: regex по PDF bytes, заменить `/CreationDate(D:...)` на `/CreationDate(D:20260101000000Z)`.
4. SHA256 на 3 ОС-runners из CI matrix должен быть одинаков.

---

### F. Backend — commands (Tauri) + http

#### `crates/trackly-app/src/tauri_cmds/acts.rs` (command, request-response)

**Analog:** `/Users/madsas/Projects/trackly/crates/trackly-app/src/tauri_cmds/devices.rs`

**Pattern (lines 17-89 — `build_*` helpers + lines 99-228 — Tauri thin wrappers):**

```rust
use crate::context::AppCtx;
use crate::dto::act::{ActDto, ActCreateDto, ActReturnDto, ActsCountsDto, ActFilter, Pagination};
use trackly_core::error::AppError;

// build_* helpers (lines 17-97 в devices.rs)
pub async fn build_acts_list(ctx: &AppCtx, filter: ActFilter, pagination: Pagination) -> Result<ActListResponse, AppError> {
    ctx.acts.list(filter, pagination).await
}
pub async fn build_acts_get(ctx: &AppCtx, id: i64) -> Result<ActDto, AppError> { ctx.acts.get(id).await }
pub async fn build_acts_create(ctx: &AppCtx, payload: ActCreateDto) -> Result<ActDto, AppError> { ctx.acts.create(payload).await }
pub async fn build_acts_return(ctx: &AppCtx, act_id: i64, payload: ActReturnDto) -> Result<ActDto, AppError> { ctx.acts.do_return(act_id, payload).await }
pub async fn build_acts_delete(ctx: &AppCtx, id: i64, version: i64) -> Result<(), AppError> { ctx.acts.delete_soft(id, version).await }
pub async fn build_acts_counts(ctx: &AppCtx) -> Result<ActsCountsDto, AppError> { ctx.acts.counts().await }
pub async fn build_acts_search(ctx: &AppCtx, query: String, filter: ActFilter, pagination: Pagination) -> Result<ActListResponse, AppError> { ctx.acts.search(query, filter, pagination).await }
pub async fn build_acts_peek_next_number(ctx: &AppCtx) -> Result<i64, AppError> { ctx.acts.peek_next_number().await }
pub async fn build_acts_render_pdf(ctx: &AppCtx, act_id: i64) -> Result<Vec<u8>, AppError> { ctx.acts.render_pdf(act_id).await }

// Thin Tauri wrappers (lines 99-228 в devices.rs — обязательно `#[specta::specta]` ПОСЛЕ `#[tauri::command]`):
#[tauri::command]
#[specta::specta]
pub async fn acts_list(state: tauri::State<'_, AppCtx>, filter: ActFilter, pagination: Pagination) -> Result<ActListResponse, AppError> {
    build_acts_list(state.inner(), filter, pagination).await
}
// ... аналогично для acts_get, acts_create, acts_return, acts_delete, acts_counts, acts_search, acts_peek_next_number, acts_render_pdf
```

**Тип возврата для `acts_render_pdf`:** `Result<Vec<u8>, AppError>`. На фронтенде через Tauri invoke это вернётся как `number[]` (массив байт). Преобразование в `Blob` — на UI стороне (`new Blob([new Uint8Array(bytes)], { type: 'application/pdf' })`).

#### `crates/trackly-app/src/tauri_cmds/organization.rs` / `templates.rs`

Тривиальные thin commands: `organization_get`, `templates_get_active(kind)`, `templates_render_preview(kind, sample_act_dto)`.

---

#### `crates/trackly-app/src/http/acts.rs` (http handler, POST adapter)

**Analog:** `/Users/madsas/Projects/trackly/crates/trackly-app/src/http/devices.rs`

**Pattern (lines 8-313 в аналоге):** Payload-structs (`#[derive(serde::Deserialize)]`), handlers через `State(ctx): State<AppCtx>` + `Json(payload)`, маппинг через `AppErrorResponse::from(...)`, `pub fn router() -> Router<AppCtx>` в конце с `Router::new().route("/api/v1/acts_list", post(handler_list))`...

Phase 3 ХРАНИТ router но НЕ bind'ит сервер — bind будет в Phase 5 (CONTEXT §«Server-mode HTTP-handlers — axum router строится (как в Phase 2), но не bind'ится»).

---

### G. Backend — composition root

#### `crates/trackly-app/src/context.rs` (модификация — D-AppCtx-Extension-03)

**Analog:** существующий `AppCtx::build` (lines 76-153).

**Pattern delta (insert после `let devices = Arc::new(DeviceService::new(...))` на line 136):**

```rust
let acts = Arc::new(ActService::new(writer.clone(), readers.clone(), clock.clone()));
let organization = Arc::new(OrganizationService::new(paths_arc.clone()));
let templates = Arc::new(TemplateService::new(writer.clone(), readers.clone(), clock.clone()));
let pdf = Arc::new(PdfRenderer::new());      // загружает font bytes + строит MiniJinja env

// Idempotent seed of default templates on first start.
templates.seed_defaults_on_startup().await?;
```

Затем дополнить struct:

```rust
pub struct AppCtx {
    // ... existing fields ...
    pub devices: Arc<DeviceService>,
    pub acts: Arc<ActService>,                 // NEW
    pub organization: Arc<OrganizationService>, // NEW
    pub templates: Arc<TemplateService>,        // NEW
    pub pdf: Arc<PdfRenderer>,                  // NEW
}
```

---

#### `crates/trackly-app/src/specta_export.rs` (модификация)

**Analog:** `/Users/madsas/Projects/trackly/crates/trackly-app/src/specta_export.rs`

**Pattern (line 18 — `collect_commands!`):** добавить все новые commands. КРИТИЧНО: пропуск регистрации = команда невидима для frontend и для axum-handlers (если бы они были bound) — это инвариант проекта.

```rust
Builder::<tauri::Wry>::new().commands(collect_commands![
    // ... все Phase 1+2 ...

    // Phase 3 — Acts
    crate::tauri_cmds::acts::acts_list,
    crate::tauri_cmds::acts::acts_get,
    crate::tauri_cmds::acts::acts_create,
    crate::tauri_cmds::acts::acts_return,
    crate::tauri_cmds::acts::acts_delete,
    crate::tauri_cmds::acts::acts_counts,
    crate::tauri_cmds::acts::acts_search,
    crate::tauri_cmds::acts::acts_peek_next_number,
    crate::tauri_cmds::acts::acts_render_pdf,

    // Phase 3 — Organization
    crate::tauri_cmds::organization::organization_get,

    // Phase 3 — Templates (Phase 3 — только get + preview; CRUD UI — Phase 7)
    crate::tauri_cmds::templates::templates_get_active,
    crate::tauri_cmds::templates::templates_render_preview,

    // Phase 3 — DEV-14 acceptance
    crate::tauri_cmds::acts::devices_render_acceptance_pdf,
])
```

---

### H. Backend — tests

#### `crates/trackly-app/tests/acts_crud.rs`, `acts_returns.rs`, `acts_undo.rs`, `acts_search.rs`, `acts_numbering.rs`, `acts_http_smoke.rs`

**Analog:** `/Users/madsas/Projects/trackly/crates/trackly-app/tests/devices_crud.rs` (структура), `concurrent_writes.rs` (для acts_numbering — 50 параллельных create через `tokio::join_all` → assert все номера уникальны), `devices_http_smoke.rs` (для acts_http_smoke).

**Pattern (lines 1-80 в `devices_crud.rs`):**

```rust
use std::sync::Arc;
use std::time::Duration;

use trackly_app::dto::act::{ActCreateDto, ActFilter, Pagination};
use trackly_app::services::ActService;
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

fn make_acts_service() -> (ActService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = ActService::new(writer, readers, clock);
    (svc, dir)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_handover_increments_counter_and_audits() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        // seed device первым: вне ActService, через прямой repo (devices_repo через AppCtx-style helper)
        // ... assertions: dto.number == "1", counter inc'd, audit_log row exists ...
    }).await.expect("budget");
}
```

**Гарантия `tokio::time::timeout(30s)`** обязательна на каждый тест (см. PATTERNS.md §Pattern 4 в `devices_crud.rs:7-8`).

---

#### `crates/trackly-app/tests/pdf_determinism.rs` (NEW PATTERN)

```rust
use sha2::{Digest, Sha256};
use trackly_app::dto::doc_spec::DocSpec;
use trackly_app::pdf::PdfRenderer;

#[test]
fn fixture_act_42_renders_to_known_hash() {
    let json = include_str!("fixtures/act_42.json");
    let spec: DocSpec = serde_json::from_str(json).expect("fixture parse");
    let renderer = PdfRenderer::new();
    let bytes = renderer.render_docspec(&spec).expect("render");

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let actual = format!("{:x}", hasher.finalize());

    let expected = include_str!("fixtures/act_42.sha256").trim();
    assert_eq!(actual, expected,
        "PDF hash drift detected. Если изменение намеренное — обнови act_42.sha256.");
}

#[test]
fn rendering_twice_yields_identical_bytes() {
    let json = include_str!("fixtures/act_42.json");
    let spec: DocSpec = serde_json::from_str(json).expect("fixture parse");
    let renderer = PdfRenderer::new();
    let a = renderer.render_docspec(&spec).expect("render a");
    let b = renderer.render_docspec(&spec).expect("render b");
    assert_eq!(a, b, "non-deterministic PDF output between two consecutive renders");
}
```

#### `crates/trackly-app/tests/pdf_text_extract.rs` (NEW PATTERN)

```rust
#[test]
fn fixture_pdf_contains_cyrillic_marker() {
    let json = include_str!("fixtures/act_42.json");
    let spec: DocSpec = serde_json::from_str(json).expect("parse");
    let bytes = PdfRenderer::new().render_docspec(&spec).expect("render");
    let text = pdf_extract::extract_text_from_mem(&bytes).expect("extract");
    assert!(text.contains("Сидоров-Петроградский"),
        "Cyrillic marker missing in extracted text; check DejaVu Sans glyph coverage");
    assert!(text.contains("№42"), "Act number missing");
    assert!(text.contains("(ё)"), "yo char (ё) missing — encoding regression");
}
```

#### `crates/trackly-app/tests/templates_seed.rs`

**Analog:** `/Users/madsas/Projects/trackly/crates/trackly-infra/tests/seed_data.rs` (структура idempotency).

Проверяет: после `seed_defaults_on_startup().await` дважды подряд — count(`act_handover`) == 1 (не 2); soft-delete всех записей kind'а → следующий seed восстанавливает дефолт.

---

### I. Frontend — `ui/src/features/acts/` (Svelte 5 runes)

#### `ui/src/features/acts/ActsPage.svelte`

**Analog:** `/Users/madsas/Projects/trackly/ui/src/features/devices/DevicesPage.svelte`

**Imports / state pattern (lines 1-32 в аналоге):**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';
  import ActsList from './ActsList.svelte';
  import ActDetail from './ActDetail.svelte';
  import ActFormModal from './ActFormModal.svelte';
  import ReturnModal from './ReturnModal.svelte';
  import PdfPreviewModal from './PdfPreviewModal.svelte';
  import { acts } from './api';
  import type { ActDto, ActFilter, ActsCountsDto, Pagination } from '../../bindings';

  type TabKey = 'handover' | 'returns' | 'archive';

  let items = $state<ActDto[]>([]);
  let total = $state(0);
  let loading = $state(false);
  let counts = $state<ActsCountsDto>({ handover_active: 0, returns: 0, archived: 0 });
  let activeTab = $state<TabKey>('handover');
  let selectedActId = $state<number | null>(null);
  let selectedAct = $state<ActDto | null>(null);
  let createModalOpen = $state(false);
  let returnModalOpen = $state(false);
  let pdfModalOpen = $state(false);

  let searchQuery = $state('');
  const pagination = $state<Pagination>({ offset: 0, limit: 50 });

  const baseFilter = $derived<ActFilter>({
    act_type: activeTab === 'returns' ? 'return' : 'handover',
    archived: activeTab === 'archive' ? true : (activeTab === 'handover' ? false : null),
    include_deleted: false,
    search: null,
  });
</script>
```

**Effect-based refresh pattern (lines 52-101 в аналоге):** `refresh()` + `refreshCounts()` + `$effect(() => { void activeTab; refresh(); refreshCounts(); })`.

**Selection sync** (FLAG-001):

```svelte
$effect(() => {
  void activeTab;
  selectedActId = null;        // null-reset on tab switch (D-Acts-List-01 + FLAG-001)
  selectedAct = null;
});

$effect(() => {
  if (selectedActId === null) { selectedAct = null; return; }
  acts.get(selectedActId).then((a) => { selectedAct = a; })
       .catch((e) => pushToast('error', e?.message ?? 'Не удалось загрузить акт'));
});
```

**Error pattern (lines 73-78):** копировать 1-к-1 (`e && typeof e === 'object' && 'message' in e ? String(...) : 'fallback msg'`).

---

#### `ui/src/features/acts/ActFormModal.svelte`

**Analog:** `/Users/madsas/Projects/trackly/ui/src/features/devices/DeviceFormModal.svelte`

**Pattern (lines 1-92 в аналоге):** Modal-shell + `{#key openInstanceCounter}` для гарантированного remount при каждом open + `bodySubmitFn` callback paradigm (footer button → bodySubmitFn directly). Для Phase 3 — `<Modal size="xwide">` (см. модификацию Modal.svelte ниже).

---

#### `ui/src/features/acts/PdfPreviewModal.svelte` (role-match, частично NEW)

**Analog:** `/Users/madsas/Projects/trackly/ui/src/features/devices/DeviceImportCsvModal.svelte` (Modal + loading-state).

**NEW PATTERN — blob URL lifecycle:**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { acts } from './api';

  interface Props {
    open: boolean;
    actId: number;
    title: string;     // «Печать акта №42» / «Печать акта возврата №42в1» / «Печать документа приёма»
    onClose: () => void;
  }
  const { open, actId, title, onClose }: Props = $props();

  let blobUrl = $state<string | null>(null);
  let loading = $state(false);
  let errorMsg = $state<string | null>(null);
  let iframeEl = $state<HTMLIFrameElement | null>(null);

  async function loadPdf() {
    loading = true; errorMsg = null;
    try {
      const bytes = await acts.renderPdf(actId);    // Vec<u8> приходит как number[]
      const blob = new Blob([new Uint8Array(bytes)], { type: 'application/pdf' });
      blobUrl = URL.createObjectURL(blob);
    } catch (e: unknown) {
      errorMsg = e && typeof e === 'object' && 'message' in e
        ? String((e as { message: unknown }).message)
        : 'Не удалось сформировать PDF';
    } finally { loading = false; }
  }

  $effect(() => {
    if (open) loadPdf();
    return () => {
      if (blobUrl) { URL.revokeObjectURL(blobUrl); blobUrl = null; }
    };
  });

  async function handlePrint() {
    iframeEl?.contentWindow?.print();
  }
</script>

<Modal {open} {title} size="pdf-preview" {onClose}>
  {#if loading}
    <!-- spinner overlay -->
  {:else if errorMsg}
    <!-- error state + повторить -->
  {:else if blobUrl}
    <iframe bind:this={iframeEl} src={blobUrl} title="PDF preview" />
  {/if}
  {#snippet footer()}
    <Button variant="ghost" onclick={onClose}>Закрыть</Button>
    <Button variant="primary" onclick={handlePrint}>Печать</Button>
    <!-- Сохранить как PDF / Открыть в системном просмотрщике — через tauri-plugin-dialog / tauri-plugin-shell -->
  {/snippet}
</Modal>
```

---

#### `ui/src/lib/api/acts.ts`

**Analog:** `/Users/madsas/Projects/trackly/ui/src/lib/api/devices.ts`

**Pattern (lines 1-58 в аналоге — `apiCall<R>('devices_*', { args })` wrapper):**

```typescript
import { apiCall } from './client';
import type {
  ActDto, ActCreateDto, ActReturnDto, ActsCountsDto,
  ActFilter, ActListResponse, Pagination,
} from '../../bindings';

export const acts = {
  list: (filter: ActFilter, pagination: Pagination) =>
    apiCall<ActListResponse>('acts_list', { filter, pagination }),

  get: (id: number) => apiCall<ActDto>('acts_get', { id }),

  create: (payload: ActCreateDto) => apiCall<ActDto>('acts_create', { payload }),

  doReturn: (actId: number, payload: ActReturnDto) =>
    apiCall<ActDto>('acts_return', { actId, payload }),

  delete: (id: number, version: number) =>
    apiCall<null>('acts_delete', { id, version }),

  counts: () => apiCall<ActsCountsDto>('acts_counts'),

  search: (query: string, filter: ActFilter, pagination: Pagination) =>
    apiCall<ActListResponse>('acts_search', { query, filter, pagination }),

  peekNextNumber: () => apiCall<number>('acts_peek_next_number'),

  /** Возвращает PDF как массив байт (Tauri invoke сериализует Vec<u8> в number[]). */
  renderPdf: (actId: number) => apiCall<number[]>('acts_render_pdf', { actId }),
};
```

**ВАЖНО — naming convention args в apiCall:** объект ключей camelCase _фронтенда_ → tauri-specta автоматически конвертирует в snake_case Rust-аргументы. Это видно в `devices.ts:33-39` (`ctxName`, `ctxStatusId` → `ctx_name`, `ctx_status_id` на бэке).

---

#### `ui/src/lib/components/Modal.svelte` — МОДИФИКАЦИЯ

**Текущий код** (lines 1-13):

```svelte
interface Props {
  open: boolean;
  title: string;
  size?: 'md' | 'wide';      // <-- расширить
  onClose: () => void;
  ...
}
const { open, title, size = 'md', ... }: Props = $props();
```

**Delta:**

```svelte
size?: 'md' | 'wide' | 'xwide' | 'pdf-preview';
```

CSS дополнение (lines 65+ в `<style lang="scss">`):

```scss
.modal-xwide { max-width: 1000px; }
.modal-pdf-preview {
  max-width: min(95vw, 1100px);
  height: min(90vh, 920px);
  // тёмный backdrop делается на .modal-backdrop через :has(.modal-pdf-preview)
}
```

---

#### `ui/src/features/devices/DeviceAutocompleteField.svelte` — МОДИФИКАЦИЯ

**Текущая Props (lines 31-43):**

```typescript
interface Props {
  field: FieldName;
  value: string;
  contextName?: string;
  contextStatusId?: number | null;
  // ... добавить:
  statusIn?: string[];          // array of status code strings; filter at backend
  // ...
}
```

**Backend изменение (RESEARCH §FLAG):** `devices_autocomplete` принимает `status_in: Option<Vec<String>>` (с маппингом code → id внутри сервиса). `build_devices_autocomplete` сигнатура расширяется. Это касается `crates/trackly-app/src/tauri_cmds/devices.rs:62-69` и `crates/trackly-app/src/services/device_service.rs::autocomplete`.

---

## Shared Patterns

### S-1. Error handling (cross-cutting)

**Source:** `/Users/madsas/Projects/trackly/crates/trackly-core/src/error.rs:33-100`
**Apply to:** ALL service / command / repo files in Phase 3.

**Correct variant usage:**

```rust
// Validation: обязательные поля, длина, формат
return Err(AppError::Validation {
    field: "giver_name".into(),
    message: "Поле «Сдал» обязательно".into(),
});

// Conflict: UNIQUE violations, custom-override номера, версия не та
return Err(AppError::Conflict { reason: format!("Акт №{n} уже существует") });

// NotFound: чтение по id, который не существует или soft-deleted
return Err(AppError::NotFound { entity: "act", id });

// OptimisticLockMismatch: update/delete с устаревшей version
return Err(AppError::OptimisticLockMismatch {
    entity: "act", id, expected: version, actual: row.version,
});

// Internal: спецификации, JSON serde, неожиданные rusqlite/tokio ошибки
return Err(AppError::Internal {
    source_chain: format!("spawn_blocking: {e}"),
});
```

`rusqlite::Error` → `AppError` через `map_rusqlite` (`crates/trackly-infra/src/error_conversions.rs`) — никогда не пишите свой mapping.

### S-2. Snake_case JSON (project-wide invariant)

**Source:** `/Users/madsas/Projects/trackly/crates/trackly-app/src/dto/device.rs:7-8`
**Apply to:** ALL DTO files in Phase 3.

> Snake_case JSON — НИКАКИХ `rename_all = "camelCase"`. (см. модуль-doc lines 7-8). На фронте — camelCase в _аргументах_ apiCall (tauri-specta конвертирует), но _shape ответа_ в bindings.ts = snake_case (`act.giver_name`, не `act.giverName`).

### S-3. specta `#[specta(type = i32)]` на всех i64

**Source:** `/Users/madsas/Projects/trackly/crates/trackly-app/src/dto/device.rs:30-32, 33-59`
**Apply to:** ALL DTO files.

```rust
#[specta(type = i32)] pub id: i64,
#[specta(type = i32)] pub version: i64,
#[specta(type = i32)] pub created_at_utc: i64,
#[specta(type = Option<i32>)] pub location_id: Option<i64>,
```

Без этого `specta-typescript = 0.0.9` не сгенерирует bindings (фейл при `cargo test --test export_bindings`).

### S-4. Single-writer для ВСЕХ мутаций

**Source:** `/Users/madsas/Projects/trackly/crates/trackly-infra/src/db/writer_worker.rs:1-30`
**Apply to:** ALL service mutation paths (create/update/delete/return) AND counter increment AND template seed AND org.json placeholder write.

**Anti-pattern (RESEARCH §Anti-Patterns):** прямой `Connection::open(...)` вне `AppCtx::build`. Прямые `repo.create(conn, ...)` вне writer-closures. Никаких параллельных writer-connections.

### S-5. Snake_case Rust args ↔ camelCase frontend args

**Source:** `/Users/madsas/Projects/trackly/ui/src/lib/api/devices.ts:33-39` (`ctxName`, `ctxStatusId` ↔ `ctx_name`, `ctx_status_id`).

tauri-specta автомагически конвертирует. Поэтому в TS-обёртках для acts:
- `actId` → бэк увидит `act_id`
- `numberOverride` → бэк увидит `number_override`
- `locationId` → бэк увидит `location_id`

### S-6. `tokio::time::timeout(30s)` в каждом интеграционном тесте

**Source:** `/Users/madsas/Projects/trackly/crates/trackly-app/tests/devices_crud.rs:7-8, 49-80`
**Apply to:** ALL `crates/trackly-app/tests/acts_*.rs` и `pdf_*.rs`.

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn my_test() {
    tokio::time::timeout(Duration::from_secs(30), async {
        // test body
    }).await.expect("my_test exceeded 30 s budget");
}
```

Без timeout — Linux CI может deadlock'нуться при тонком race-condition (PATTERNS.md §Pattern 4 из Phase 2).

### S-7. Russian user-facing strings; English code/idents

**Source:** `/Users/madsas/Projects/trackly/crates/trackly-app/src/services/device_service.rs:78-95` (validation messages — RU); `crates/trackly-core/src/error.rs:79` (field names — EN).

- `AppError::Validation.message` — на русском.
- `AppError::Validation.field` — английское имя поля (`"giver_name"`, `"items[0].quantity"`).
- Имена commands / DTO / структур — английские snake_case.

### S-8. Portable invariant: НИКОГДА `dirs::*_dir()`

**Source:** `/Users/madsas/Projects/trackly/crates/trackly-infra/src/paths.rs:5-8` (`dirs::*_dir() запрещено через clippy.toml disallowed-methods`).
**Apply to:** `organization_service.rs`, `pdf` подсистема (tmp-файл для «Открыть в системном просмотрщике»), любые file paths в Phase 3.

Всё через `paths.exe_dir().join(...)`. Для tmp PDF — `paths.exe_dir().join("tmp").join(filename)` (создать dir при необходимости через `std::fs::create_dir_all`).

### S-9. Router composition в Phase 5 (готовим, не bind'им)

**Source:** `/Users/madsas/Projects/trackly/crates/trackly-app/src/http/devices.rs:292-313`
**Apply to:** `http/acts.rs`, `http/organization.rs`, `http/templates.rs`.

Каждый файл экспортирует `pub fn router() -> Router<AppCtx>`. В Phase 3 эти routers НЕ mount'ятся в реальный bind — это будет делать Phase 5. Но `cargo build` обязан их типизировать (как и в Phase 2).

---

## No Analog Found (NEW PATTERN — full responsibility)

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/trackly-app/src/pdf/renderer.rs` | renderer | DocSpec → Vec<u8> | krilla API не имеет прямого аналога в кодовой базе; rendering pipeline новый |
| `crates/trackly-app/src/pdf/docspec.rs` | typed AST | serde + specta | enum-AST с tagged variants — отличается от плоских DTO Phase 2 |
| `crates/trackly-app/src/pdf/minijinja_env.rs` | wrapper | safe-mode Environment | template engine впервые в проекте |
| `crates/trackly-app/src/pdf/fonts.rs` | const data | `include_bytes!` | впервые binary asset embedded в crate |
| `crates/trackly-app/assets/fonts/*.ttf` | binary asset | static | новый класс файлов |
| `crates/trackly-app/templates/*.minijinja` | text template | static | новый класс файлов |
| `crates/trackly-app/tests/pdf_determinism.rs` | test | sha256 hash | специфичный тест-паттерн (hash на 3 ОС) |
| `crates/trackly-app/tests/pdf_text_extract.rs` | test | pdf-extract assert | первое использование pdf-extract |
| `crates/trackly-app/tests/fixtures/act_42.json` | fixture | static JSON | первая фикстура для PDF |
| `crates/trackly-app/tests/fixtures/act_42.sha256` | fixture | static hex | первая хеш-фикстура |
| `ui/src/features/acts/ActHeaderField.svelte` | display field | label+value pair | тривиально новый; ниже минимальный шаблон |
| `org.json.example` (repo-root docs) | sample config | static | новый класс файлов |

Каждый из NEW PATTERN файлов получил минимальную идиоматичную форму в соответствующих секциях выше. Planner должен ссылаться на эти секции как на «контракт» для plan-actions.

**Trivial NEW PATTERN — `ActHeaderField.svelte`:**

```svelte
<script lang="ts">
  interface Props { label: string; value: string | null; }
  const { label, value }: Props = $props();
</script>
<div class="field">
  <div class="label">{label}</div>
  <div class="value" class:muted={value === null}>{value ?? '—'}</div>
</div>
<style lang="scss">
  .field { display: flex; flex-direction: column; gap: var(--space-xs); }
  .label { font-size: var(--font-size-label); font-weight: 500; color: var(--color-text-secondary); }
  .value { font-size: var(--font-size-body); color: var(--color-text-primary); }
  .value.muted { color: var(--color-text-muted); }
</style>
```

---

## Metadata

**Analog search scope:**
- `crates/trackly-core/src/domain/`, `crates/trackly-core/src/ports/`, `crates/trackly-core/src/error.rs`
- `crates/trackly-infra/src/repos/`, `crates/trackly-infra/src/db/`, `crates/trackly-infra/src/paths.rs`, `crates/trackly-infra/src/test_support/`
- `crates/trackly-app/src/services/`, `crates/trackly-app/src/dto/`, `crates/trackly-app/src/tauri_cmds/`, `crates/trackly-app/src/http/`, `crates/trackly-app/src/csv/`, `crates/trackly-app/src/context.rs`, `crates/trackly-app/src/specta_export.rs`
- `crates/trackly-app/tests/devices_crud.rs`, `tests/devices_http_smoke.rs`, `tests/concurrent_writes.rs`, `tests/export_bindings.rs`
- `migrations/V004`, `V007`, `V008`, `V009`, `V012`, `V013`
- `ui/src/lib/api/`, `ui/src/lib/components/`, `ui/src/features/devices/`, `ui/src/features/layout/`, `ui/src/routes.ts`, `ui/src/pages/ActsPage.svelte`

**Files scanned:** ~35 (исходники Phase 1+2 + миграции + тесты).
**Files NOT scanned (out of scope):** placeholders в `ui/src/pages/` (Cartridges, Printers, Requests, etc.), `tools/procmon-check/`, `webview_env.rs`, `logging.rs`.

**Pattern extraction date:** 2026-05-29

---

## PATTERN MAPPING COMPLETE
