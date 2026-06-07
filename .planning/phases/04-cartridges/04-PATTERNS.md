# Phase 4: Картриджи — Pattern Map

**Mapped:** 2026-06-07
**Files analyzed:** 26 новых/изменяемых файлов
**Analogs found:** 26 / 26

---

## File Classification

| Новый / изменяемый файл | Роль | Data Flow | Ближайший аналог | Качество совпадения |
|-------------------------|------|-----------|------------------|----------------------|
| `migrations/V016__cartridges_kind_color_settings.sql` | migration | batch/transform | `migrations/V013__devices_fts_triggers.sql` | exact |
| `crates/trackly-core/src/domain/cartridges.rs` | model | CRUD | `crates/trackly-core/src/domain/acts.rs` | exact |
| `crates/trackly-core/src/ports/cartridges.rs` | port | request-response | `crates/trackly-core/src/ports/acts.rs` | exact |
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs` | repository | CRUD | `crates/trackly-infra/src/repos/acts_sqlite.rs` | exact |
| `crates/trackly-app/src/dto/cartridge.rs` | dto | request-response | `crates/trackly-app/src/dto/act.rs` | exact |
| `crates/trackly-app/src/services/cartridge_service.rs` | service | CRUD | `crates/trackly-app/src/services/act_service.rs` | exact |
| `crates/trackly-app/src/tauri_cmds/cartridges.rs` | controller | request-response | `crates/trackly-app/src/tauri_cmds/acts.rs` | exact |
| `crates/trackly-app/src/http/cartridges.rs` | controller | request-response | `crates/trackly-app/src/http/acts.rs` | exact |
| `crates/trackly-app/src/context.rs` (расширить) | config | — | `crates/trackly-app/src/context.rs` | self |
| `crates/trackly-app/src/specta_export.rs` (расширить) | config | — | `crates/trackly-app/src/specta_export.rs` | self |
| `crates/trackly-app/tests/cartridges_crud.rs` | test | CRUD | `crates/trackly-app/tests/acts_crud.rs` | exact |
| `crates/trackly-app/tests/cartridges_numbering.rs` | test | CRUD | `crates/trackly-app/tests/acts_numbering.rs` | exact |
| `crates/trackly-app/tests/cartridges_lifecycle.rs` | test | event-driven | `crates/trackly-app/tests/acts_returns.rs` | role-match |
| `crates/trackly-app/tests/cartridges_search.rs` | test | CRUD | `crates/trackly-app/tests/acts_search.rs` | exact |
| `crates/trackly-app/tests/cartridges_low_stock.rs` | test | CRUD | `crates/trackly-app/tests/acts_crud.rs` | role-match |
| `crates/trackly-app/tests/cartridges_history.rs` | test | CRUD | `crates/trackly-app/tests/acts_undo.rs` | role-match |
| `crates/trackly-infra/src/test_support/test_db.rs` (обновить assertion) | test | — | self | self |
| `ui/src/features/cartridges/CartridgesPage.svelte` | component | request-response | `ui/src/features/acts/ActsPage.svelte` | exact |
| `ui/src/features/cartridges/CartridgesSearchAndTabs.svelte` | component | request-response | `ui/src/features/acts/ActsSearchAndTabs.svelte` | exact |
| `ui/src/features/cartridges/CartridgesMasterDetail.svelte` | component | request-response | `ui/src/features/acts/ActsMasterDetail.svelte` | exact |
| `ui/src/features/cartridges/CartridgesList.svelte` | component | request-response | `ui/src/features/acts/ActsList.svelte` | exact |
| `ui/src/features/cartridges/CartridgeListRow.svelte` | component | request-response | `ui/src/features/acts/ActListRow.svelte` | exact |
| `ui/src/features/cartridges/CartridgeDetail.svelte` | component | request-response | `ui/src/features/acts/ActDetail.svelte` | exact |
| `ui/src/features/cartridges/CartridgeContextMenu.svelte` | component | event-driven | `ui/src/features/devices/DeviceContextMenu.svelte` | exact |
| `ui/src/features/cartridges/CartridgeFilters.svelte` | component | request-response | `ui/src/features/devices/DeviceFilters.svelte` | exact |
| `ui/src/features/cartridges/CartridgeFormModal.svelte` | component | request-response | `ui/src/features/devices/DeviceFormModal.svelte` | exact |
| `ui/src/features/cartridges/OperationModal.svelte` | component | event-driven | `ui/src/features/acts/ReturnModal.svelte` | role-match |
| `ui/src/features/cartridges/LowStockBanner.svelte` | component | request-response | нет — новый паттерн | no analog |
| `ui/src/features/cartridges/ModelsList.svelte` | component | CRUD | `ui/src/features/acts/ActsList.svelte` | role-match |
| `ui/src/features/cartridges/ModelListRow.svelte` | component | CRUD | `ui/src/features/acts/ActListRow.svelte` | role-match |
| `ui/src/features/cartridges/ModelFormModal.svelte` | component | CRUD | `ui/src/features/devices/DeviceFormModal.svelte` | role-match |
| `ui/src/features/cartridges/CompatibilityEditor.svelte` | component | CRUD | нет — новый паттерн | no analog |
| `ui/src/features/cartridges/api.ts` | utility | request-response | `ui/src/lib/api/acts.ts` | exact |

---

## Pattern Assignments

---

### `migrations/V016__cartridges_kind_color_settings.sql` (migration, batch)

**Аналог:** `migrations/V013__devices_fts_triggers.sql`

**Паттерн структуры** (строки 1–67 V013):
```sql
-- Три AFTER-триггера: ai (INSERT) / ad (DELETE) / au (UPDATE).
-- Обязательно: WHEN NEW.deleted_at_utc IS NULL на INSERT-триггере.
-- DELETE-триггер: INSERT с первым аргументом 'delete' (FTS5 external-content semantics).
-- UPDATE-триггер: сначала delete, затем conditional insert.

CREATE TRIGGER cartridges_fts_ai
AFTER INSERT ON cartridges
WHEN NEW.deleted_at_utc IS NULL
BEGIN
  INSERT INTO cartridges_fts(rowid, code, location, holder_name)
  VALUES (NEW.id, NEW.code, NEW.location, NEW.holder_name);
END;

CREATE TRIGGER cartridges_fts_ad
AFTER DELETE ON cartridges
BEGIN
  INSERT INTO cartridges_fts(cartridges_fts, rowid, code, location, holder_name)
  VALUES ('delete', OLD.id, OLD.code, OLD.location, OLD.holder_name);
END;

CREATE TRIGGER cartridges_fts_au
AFTER UPDATE ON cartridges
BEGIN
  INSERT INTO cartridges_fts(cartridges_fts, rowid, code, location, holder_name)
  VALUES ('delete', OLD.id, OLD.code, OLD.location, OLD.holder_name);
  INSERT INTO cartridges_fts(rowid, code, location, holder_name)
  SELECT NEW.id, NEW.code, NEW.location, NEW.holder_name
  WHERE NEW.deleted_at_utc IS NULL;
END;

PRAGMA user_version = 13;  -- → заменить на 16
```

**Паттерн ALTER TABLE + DEFAULT** (критически важно, Pitfall 2 из RESEARCH.md):
```sql
-- ВСЕГДА указывать DEFAULT при NOT NULL на ADD COLUMN — иначе SQLite откажет.
ALTER TABLE cartridge_models
  ADD COLUMN kind_id INTEGER NOT NULL DEFAULT 1
    REFERENCES cartridge_kinds(id);
ALTER TABLE cartridge_models
  ADD COLUMN color TEXT NULL;
```

**Паттерн lookup-таблицы** (по образцу V001 `cartridge_statuses`):
```sql
CREATE TABLE cartridge_kinds (
  id   INTEGER PRIMARY KEY,
  name TEXT    NOT NULL UNIQUE
);
INSERT INTO cartridge_kinds (id, name) VALUES
  (1, 'Картридж'),
  (2, 'Фотобарабан');
```

**Паттерн app_settings** (новая таблица, D-LowStock-01):
```sql
CREATE TABLE app_settings (
  key            TEXT    NOT NULL PRIMARY KEY,
  value          TEXT    NOT NULL,
  created_at_utc INTEGER NOT NULL,
  updated_at_utc INTEGER NOT NULL
);
INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc)
  VALUES ('low_stock_threshold', '2', unixepoch(), unixepoch());
```

---

### `crates/trackly-core/src/domain/cartridges.rs` (model, CRUD)

**Аналог:** `crates/trackly-core/src/domain/acts.rs`

**Паттерн импортов** (строки 1–11 acts.rs):
```rust
//! NO serde::Serialize/Deserialize or specta::Type derives here — those live
//! in the DTO layer in trackly-app. Only `#[derive(Debug, Clone, PartialEq, Eq)]`.
use crate::error::AppError;
```

**Паттерн структуры строки** (строки 119–152 acts.rs → адаптировать под CartridgeRow):
```rust
/// Full cartridge row as returned from the repository read path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartridgeRow {
    pub id: i64,
    pub code: String,
    pub model_id: i64,
    pub model_brand: Option<String>,   // joined
    pub model_name: Option<String>,    // joined
    pub model_kind_id: Option<i64>,    // joined
    pub status_id: i64,
    pub status_name: Option<String>,   // joined
    pub state_id: Option<i64>,
    pub state_name: Option<String>,    // joined
    pub location: Option<String>,
    pub holder_name: Option<String>,
    pub notes: Option<String>,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub deleted_at_utc: Option<i64>,
    pub version: i64,
}
```

**Паттерн New-struct** (строки 50–77 acts.rs → адаптировать):
```rust
/// Data needed to create a new cartridge instance.
/// code_override = None → service increments cartridge_seq.
/// code_override = Some(s) → custom code, counter NOT incremented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartridgeNew {
    pub model_id: i64,
    pub code_override: Option<String>,
    pub state_id: Option<i64>,
    pub location: Option<String>,
    pub notes: Option<String>,
}
```

**Паттерн Filter и Counts** (строки 163–178 acts.rs):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CartridgeFilter {
    pub status_id: Option<i64>,
    pub kind_id: Option<i64>,    // Картридж / Фотобарабан
    pub model_id: Option<i64>,
    pub search: Option<String>,
    pub include_deleted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CartridgeCounts {
    pub all: i64,
    pub in_stock: i64,      // status_id=1
    pub in_use: i64,        // status_id=2
    pub at_refill: i64,     // status_id=3
    pub written_off: i64,   // status_id=4
}
```

**Паттерн TransitionOp enum** (`Claude's Discretion` — рекомендуется tagged enum):
```rust
/// Lifecycle transition payload — one enum covers all ops (D-Op-Modal-01).
/// Tagged with #[serde(tag="op")] for TS discriminated-union.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CartridgeTransitionOp {
    Install {
        date_utc: i64,
        given_by_name: String,
        given_to_name: String,
        location: String,
    },
    ReturnToStock {
        state_id: i64,      // default: 3 (Пустой)
        location: String,
        notes: Option<String>,
    },
    ToRefill {
        date_utc: i64,
        given_by_name: String,
        given_to_name: String,
        location: String,
    },
    FromRefill {
        state_id: i64,      // default: 1 (Полный)
        location: String,
        notes: Option<String>,
    },
    WriteOff {
        date_utc: i64,
        notes: Option<String>,
    },
}
```

---

### `crates/trackly-core/src/ports/cartridges.rs` (port, request-response)

**Аналог:** `crates/trackly-core/src/ports/acts.rs`

**Полный паттерн порта** (строки 1–54 acts.rs):
```rust
//! Write methods that participate in larger transactions are NOT part of this trait —
//! they live as `*_in_tx` helpers on `SqliteCartridgeRepository` and are
//! orchestrated by the service layer inside a single `WriterHandle::execute` closure.

use crate::domain::cartridges::{CartridgeCounts, CartridgeFilter, CartridgeRow, Pagination};
use crate::error::AppError;

pub trait CartridgeRepository {
    type Conn;

    fn get(&self, conn: &Self::Conn, id: i64) -> Result<CartridgeRow, AppError>;

    fn list(
        &self,
        conn: &Self::Conn,
        filter: &CartridgeFilter,
        page: &Pagination,
    ) -> Result<(Vec<CartridgeRow>, u64), AppError>;

    fn delete_soft(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        version: i64,
        now_utc: i64,
    ) -> Result<(), AppError>;

    fn peek_next_code(&self, conn: &Self::Conn) -> Result<i64, AppError>;

    fn counts(&self, conn: &Self::Conn) -> Result<CartridgeCounts, AppError>;
}
```

---

### `crates/trackly-infra/src/repos/cartridges_sqlite.rs` (repository, CRUD)

**Аналог:** `crates/trackly-infra/src/repos/acts_sqlite.rs`

**Паттерн импортов** (строки 1–20 acts_sqlite.rs):
```rust
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use trackly_core::domain::cartridges::{CartridgeCounts, CartridgeFilter, CartridgeRow, Pagination};
use trackly_core::error::AppError;
use trackly_core::ports::cartridges::CartridgeRepository;
use crate::error_conversions::map_rusqlite;

#[derive(Debug, Default, Clone)]
pub struct SqliteCartridgeRepository;
```

**Паттерн SELECT с JOIN** (строки 22–83 acts_sqlite.rs → адаптировать):
```rust
const SELECT_CARTRIDGES: &str = "
    SELECT c.id, c.code, c.model_id,
           m.brand AS model_brand, m.model AS model_name, m.kind_id AS model_kind_id,
           c.status_id, cs.name AS status_name,
           c.state_id, cst.name AS state_name,
           c.location, c.holder_name, c.notes,
           c.created_at_utc, c.updated_at_utc, c.deleted_at_utc, c.version
      FROM cartridges c
      LEFT JOIN cartridge_models m ON m.id = c.model_id
      LEFT JOIN cartridge_statuses cs ON cs.id = c.status_id
      LEFT JOIN cartridge_states cst ON cst.id = c.state_id
";
```

**Паттерн INSERT картриджа в транзакции** (строки 86–116 acts_sqlite.rs → адаптировать):
```rust
pub fn insert_cartridge_in_tx(
    &self,
    tx: &Transaction<'_>,
    row: &CartridgeRow,
    now_utc: i64,
) -> Result<i64, AppError> {
    tx.execute(
        "INSERT INTO cartridges \
         (code, model_id, status_id, state_id, location, holder_name, notes, \
          created_at_utc, updated_at_utc, version) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, 1)",
        params![
            row.code, row.model_id, row.status_id, row.state_id,
            row.location, row.holder_name, row.notes, now_utc,
        ],
    ).map_err(map_rusqlite)?;
    Ok(tx.last_insert_rowid())
}
```

**Паттерн increment_counter_in_tx** (строки 365–383 acts_sqlite.rs — копировать без изменений через pub use):
```rust
// Экспортировать из acts_sqlite (уже существует) — использовать напрямую:
use trackly_infra::repos::acts_sqlite::increment_counter_in_tx;

// Авто-код с retry при UNIQUE-коллизии (D-Code-01 + D-Code-Override-01):
pub fn assign_code_in_tx(
    tx: &Transaction<'_>,
    code_override: Option<&str>,
    now_utc: i64,
) -> Result<(String, bool), AppError> {
    // returns (code, was_auto)
    if let Some(custom) = code_override {
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM cartridges WHERE code = ?1 LIMIT 1)",
            params![custom],
            |r| r.get(0),
        ).map_err(map_rusqlite)?;
        if exists {
            return Err(AppError::Conflict {
                reason: format!("Картридж с кодом «{}» уже существует", custom),
            });
        }
        return Ok((custom.to_owned(), false));
    }
    // Auto-code: increment + retry loop (counter never lost on collision)
    loop {
        let seq = increment_counter_in_tx(tx, "cartridge_seq")?;
        let candidate = format!("C-{:06}", seq);
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM cartridges WHERE code = ?1 LIMIT 1)",
            params![&candidate],
            |r| r.get(0),
        ).map_err(map_rusqlite)?;
        if !exists {
            return Ok((candidate, true));
        }
        // На коллизии — инкрементируем ещё раз, счётчик не теряется.
    }
}
```

**Паттерн location round-trip** (INSERT OR IGNORE в locations, D-Op-Location-01):
```rust
// В writer-tx при create и transition — если location непустая:
if let Some(loc) = location_name.as_deref().filter(|s| !s.is_empty()) {
    tx.execute(
        "INSERT OR IGNORE INTO locations (name, created_at_utc, updated_at_utc, version) \
         VALUES (?1, ?2, ?2, 1)",
        params![loc, now_utc],
    ).map_err(map_rusqlite)?;
}
```

**Паттерн FTS LIKE UNION CTE search** (адаптировать из acts_sqlite.rs search):
```sql
WITH fts_hits AS (
  SELECT f.rowid AS id FROM cartridges_fts f
  WHERE cartridges_fts MATCH ?1
),
like_hits AS (
  SELECT c.id FROM cartridges c
  LEFT JOIN cartridge_models m ON c.model_id = m.id
  WHERE c.code LIKE ?2
     OR c.location LIKE ?2
     OR c.holder_name LIKE ?2
     OR m.brand LIKE ?2
     OR m.model LIKE ?2
)
SELECT ... FROM cartridges c
LEFT JOIN cartridge_models m ON c.model_id = m.id
LEFT JOIN cartridge_statuses cs ON c.status_id = cs.id
LEFT JOIN cartridge_states cst ON c.state_id = cst.id
WHERE c.id IN (SELECT id FROM fts_hits UNION SELECT id FROM like_hits)
  AND c.deleted_at_utc IS NULL
  AND (?3 IS NULL OR c.status_id = ?3)
ORDER BY c.created_at_utc DESC
LIMIT ?4 OFFSET ?5
```

**Паттерн LOW STOCK query** (D-LowStock-02):
```sql
SELECT m.id, m.brand, m.model, COUNT(c.id) AS cnt
FROM cartridge_models m
LEFT JOIN cartridges c ON c.model_id = m.id
  AND c.status_id = 1   -- На складе (id=1 из V001)
  AND c.state_id = 1    -- Полный (id=1 из V001)
  AND c.deleted_at_utc IS NULL
WHERE m.deleted_at_utc IS NULL
GROUP BY m.id
HAVING cnt < ?1          -- threshold из app_settings
ORDER BY cnt ASC, m.brand ASC, m.model ASC
```

---

### `crates/trackly-app/src/dto/cartridge.rs` (dto, request-response)

**Аналог:** `crates/trackly-app/src/dto/act.rs` + `crates/trackly-app/src/dto/device.rs`

**Паттерн импортов** (строки 1–13 act.rs):
```rust
use serde::{Deserialize, Serialize};
use specta::Type;
use trackly_core::domain::cartridges::CartridgeRow;
```

**Паттерн CartridgeDto с #[specta(type = i32)]** (строки 44–80 act.rs + строки 32–60 device.rs):
```rust
/// #[specta(type = i32)] на всех i64 — TS получает number, не bigint (S-3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CartridgeDto {
    #[specta(type = i32)]   pub id: i64,
    #[specta(type = i32)]   pub version: i64,
    pub code: String,
    #[specta(type = i32)]   pub model_id: i64,
    pub model_brand: Option<String>,
    pub model_name: Option<String>,
    #[specta(type = Option<i32>)] pub model_kind_id: Option<i64>,
    #[specta(type = i32)]   pub status_id: i64,
    pub status_name: Option<String>,
    #[specta(type = Option<i32>)] pub state_id: Option<i64>,
    pub state_name: Option<String>,
    pub location: Option<String>,
    pub holder_name: Option<String>,
    pub notes: Option<String>,
    #[specta(type = i32)]   pub created_at_utc: i64,
    #[specta(type = i32)]   pub updated_at_utc: i64,
}
```

**Паттерн CartridgeTransitionPayload** (tagged enum для Tauri specta, Claude's Discretion):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "op")]
pub enum CartridgeTransitionPayload {
    Install {
        cartridge_id: i64,
        version: i64,
        date_utc: i64,
        given_by_name: String,
        given_to_name: String,
        location: String,
    },
    ReturnToStock {
        cartridge_id: i64,
        version: i64,
        #[specta(type = i32)] state_id: i64,  // default 3 = Пустой
        location: String,
        notes: Option<String>,
    },
    ToRefill {
        cartridge_id: i64,
        version: i64,
        date_utc: i64,
        given_by_name: String,
        given_to_name: String,
        location: String,
    },
    FromRefill {
        cartridge_id: i64,
        version: i64,
        #[specta(type = i32)] state_id: i64,  // default 1 = Полный
        location: String,
        notes: Option<String>,
    },
    WriteOff {
        cartridge_id: i64,
        version: i64,
        date_utc: i64,
        notes: Option<String>,
    },
}
```

---

### `crates/trackly-app/src/services/cartridge_service.rs` (service, CRUD + event-driven)

**Аналог:** `crates/trackly-app/src/services/act_service.rs`

**Паттерн структуры и конструктора** (строки 40–74 act_service.rs):
```rust
use std::sync::Arc;
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::acts_sqlite::increment_counter_in_tx;
use trackly_infra::repos::audit_log_sqlite::AuditEntry;
use trackly_infra::repos::{SqliteAuditLogRepository, SqliteCartridgeRepository};

#[derive(Clone)]
pub struct CartridgeService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) cart_repo: Arc<SqliteCartridgeRepository>,
    pub(crate) audit_repo: Arc<SqliteAuditLogRepository>,
}

impl CartridgeService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            cart_repo: Arc::new(SqliteCartridgeRepository),
            audit_repo: Arc::new(SqliteAuditLogRepository),
        }
    }
```

**Паттерн write-operation через writer.execute** (строки 185–250 act_service.rs):
```rust
pub async fn create(&self, payload: CartridgeCreateDto) -> Result<CartridgeDto, AppError> {
    Self::validate_create(&payload)?;
    let now = self.clock.unix_seconds();
    let cart_repo = self.cart_repo.clone();
    let audit_repo = self.audit_repo.clone();

    let cart_id = self.writer.execute(move |conn| {
        let tx = conn.transaction().map_err(map_rusqlite)?;

        // 1. Assign code (auto or override, with UNIQUE retry)
        let (code, was_auto) =
            SqliteCartridgeRepository::assign_code_in_tx(&tx, payload.code_override.as_deref(), now)?;

        // 2. INSERT OR IGNORE location round-trip
        if let Some(loc) = payload.location.as_deref().filter(|s| !s.is_empty()) {
            tx.execute(
                "INSERT OR IGNORE INTO locations (name, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?2, 1)",
                params![loc, now],
            ).map_err(map_rusqlite)?;
        }

        // 3. INSERT cartridge
        let row_id = cart_repo.insert_cartridge_in_tx(&tx, &CartridgeRow { code, ... }, now)?;

        // 4. audit_log
        let action = if was_auto { "create" } else { "custom:cartridge_code_override" };
        audit_repo.insert(&tx, AuditEntry {
            entity_type: "cartridge",
            entity_id: row_id,
            action,
            user_id: None,  // Phase 4: always NULL
            before_json: None,
            after_json: Some(serde_json::to_string(&after_snapshot).map_err(|e| ...)?),
            payload_json: None,
            created_at_utc: now,
        })?;

        tx.commit().map_err(map_rusqlite)?;
        Ok(row_id)
    }).await?;
    // Fetch and return DTO
    self.get(cart_id).await
}
```

**Паттерн transition под single-writer** (адаптировать из act_service.rs do_return):
```rust
pub async fn transition(
    &self,
    cartridge_id: i64,
    version: i64,
    op: CartridgeTransitionOp,
) -> Result<CartridgeDto, AppError> {
    let now = self.clock.unix_seconds();
    let cart_repo = self.cart_repo.clone();
    let audit_repo = self.audit_repo.clone();

    self.writer.execute(move |conn| {
        let tx = conn.transaction().map_err(map_rusqlite)?;

        // 1. Fetch current + optimistic lock check
        let before = cart_repo.fetch_full_in_tx(&tx, cartridge_id)?;
        if before.version != version {
            return Err(AppError::OptimisticLockMismatch { ... });
        }

        // 2. Validate status transition
        let (new_status_id, new_state_id, new_location, new_holder) = match &op { ... };

        // 3. UPDATE cartridges
        tx.execute(
            "UPDATE cartridges SET status_id=?1, state_id=?2, location=?3, holder_name=?4, \
             updated_at_utc=?5, version=version+1 WHERE id=?6 AND version=?7",
            params![new_status_id, new_state_id, new_location, new_holder, now, cartridge_id, version],
        ).map_err(map_rusqlite)?;

        // 4. Location round-trip
        if let Some(loc) = new_location.as_deref().filter(|s| !s.is_empty()) {
            tx.execute(
                "INSERT OR IGNORE INTO locations (name, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?2, 1)",
                params![loc, now],
            ).map_err(map_rusqlite)?;
        }

        // 5. audit_log — action с префиксом custom:
        let (action, payload_json) = match &op {
            CartridgeTransitionOp::Install { .. } => ("custom:install", serde_json::to_string(...)?),
            CartridgeTransitionOp::ReturnToStock { .. } => ("custom:return_to_stock", ...),
            CartridgeTransitionOp::ToRefill { .. } => ("custom:to_refill", ...),
            CartridgeTransitionOp::FromRefill { .. } => ("custom:from_refill", ...),
            CartridgeTransitionOp::WriteOff { .. } => ("custom:write_off", ...),
        };
        audit_repo.insert(&tx, AuditEntry {
            entity_type: "cartridge",
            entity_id: cartridge_id,
            action,
            user_id: None,
            before_json: Some(serde_json::to_string(&before)?),
            after_json: None,  // заполняется после commit
            payload_json: Some(payload_json),
            created_at_utc: now,
        })?;

        tx.commit().map_err(map_rusqlite)?;
        Ok(cartridge_id)
    }).await?;
    self.get(cartridge_id).await
}
```

**Паттерн low_stock через ReaderPool** (read-only, D-LowStock-02):
```rust
pub async fn low_stock(&self) -> Result<Vec<LowStockItem>, AppError> {
    let conn = self.readers.acquire()?;
    let threshold: i64 = conn.query_row(
        "SELECT CAST(value AS INTEGER) FROM app_settings WHERE key = 'low_stock_threshold'",
        [], |r| r.get(0),
    ).unwrap_or(2);
    // ... LOW STOCK query (см. Pattern 3 в repos) ...
}
```

---

### `crates/trackly-app/src/tauri_cmds/cartridges.rs` (controller, request-response)

**Аналог:** `crates/trackly-app/src/tauri_cmds/acts.rs`

**Паттерн build_* helpers + thin command wrappers** (строки 1–77 acts.rs):
```rust
//! Pattern (S-1): `build_*` helper + thin `#[tauri::command] #[specta::specta]` wrapper.
//! Both transports (Tauri invoke + axum POST) delegate to the same helper.
//! `#[specta::specta]` MUST appear AFTER `#[tauri::command]`.

use crate::context::AppCtx;
use crate::dto::cartridge::{CartridgeDto, CartridgeFilter, CartridgeListResponse,
                              CartridgeTransitionPayload, Pagination};
use trackly_core::error::AppError;

pub async fn build_cartridges_list(
    ctx: &AppCtx,
    filter: CartridgeFilter,
    pagination: Pagination,
) -> Result<CartridgeListResponse, AppError> {
    ctx.cartridges.list(filter, pagination).await
}

pub async fn build_cartridges_transition(
    ctx: &AppCtx,
    payload: CartridgeTransitionPayload,
) -> Result<CartridgeDto, AppError> {
    ctx.cartridges.transition(payload).await
}

// --- thin tauri::command wrappers ---

#[tauri::command]
#[specta::specta]
pub async fn cartridges_list(
    state: tauri::State<'_, AppCtx>,
    filter: CartridgeFilter,
    pagination: Pagination,
) -> Result<CartridgeListResponse, AppError> {
    build_cartridges_list(&state, filter, pagination).await
}

#[tauri::command]
#[specta::specta]
pub async fn cartridges_transition(
    state: tauri::State<'_, AppCtx>,
    payload: CartridgeTransitionPayload,
) -> Result<CartridgeDto, AppError> {
    build_cartridges_transition(&state, payload).await
}
```

---

### `crates/trackly-app/src/http/cartridges.rs` (controller, request-response)

**Аналог:** `crates/trackly-app/src/http/acts.rs`

**Паттерн router + handlers** (строки 1–228 acts.rs):
```rust
//! Mirrors `tauri_cmds::cartridges` via POST endpoints. Router BUILT but NOT bound
//! to TCP listener — server-mode wiring is Phase 5/8.

use axum::{extract::State, routing::post, Json, Router};
use crate::context::AppCtx;
use crate::dto::cartridge::{CartridgeFilter, CartridgeListResponse, CartridgeTransitionPayload};
use crate::error_axum::AppErrorResponse;
use crate::tauri_cmds::cartridges::{build_cartridges_list, build_cartridges_transition};

pub async fn handler_list(
    State(ctx): State<AppCtx>,
    Json(p): Json<ListPayload>,
) -> Result<Json<CartridgeListResponse>, AppErrorResponse> {
    Ok(Json(
        build_cartridges_list(&ctx, p.filter, p.pagination)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/cartridges_list", post(handler_list))
        .route("/api/v1/cartridges_get", post(handler_get))
        .route("/api/v1/cartridges_create", post(handler_create))
        .route("/api/v1/cartridges_update", post(handler_update))
        .route("/api/v1/cartridges_delete", post(handler_delete))
        .route("/api/v1/cartridges_transition", post(handler_transition))
        .route("/api/v1/cartridges_search", post(handler_search))
        .route("/api/v1/cartridges_status_counts", post(handler_status_counts))
        .route("/api/v1/cartridges_get_history", post(handler_get_history))
        .route("/api/v1/cartridges_low_stock", post(handler_low_stock))
        .route("/api/v1/cartridge_models_list", post(handler_models_list))
        .route("/api/v1/cartridge_models_create", post(handler_models_create))
        .route("/api/v1/cartridge_models_update", post(handler_models_update))
        .route("/api/v1/cartridge_models_delete", post(handler_models_delete))
}
```

---

### `crates/trackly-app/src/context.rs` (расширить, config)

**Аналог:** self — паттерн добавления поля (строки 59–74, 148–175):
```rust
// В struct AppCtx добавить после `acts: Arc<ActService>`:
pub cartridges: Arc<CartridgeService>,

// В AppCtx::build после Step 12 (после acts):
let cartridges = Arc::new(CartridgeService::new(
    writer.clone(),
    readers.clone(),
    clock.clone(),
));

// В конструкторе Self { ... } добавить:
cartridges,
```

---

### `crates/trackly-app/src/specta_export.rs` (расширить, config)

**Аналог:** self (строки 17–64):
```rust
// Добавить в builder() после acts_* команд:
// Phase 4 — Cartridges
crate::tauri_cmds::cartridges::cartridges_list,
crate::tauri_cmds::cartridges::cartridges_get,
crate::tauri_cmds::cartridges::cartridges_create,
crate::tauri_cmds::cartridges::cartridges_update,
crate::tauri_cmds::cartridges::cartridges_delete,
crate::tauri_cmds::cartridges::cartridges_transition,
crate::tauri_cmds::cartridges::cartridges_search,
crate::tauri_cmds::cartridges::cartridges_status_counts,
crate::tauri_cmds::cartridges::cartridges_get_history,
crate::tauri_cmds::cartridges::cartridges_low_stock,
crate::tauri_cmds::cartridges::cartridge_models_list,
crate::tauri_cmds::cartridges::cartridge_models_get,
crate::tauri_cmds::cartridges::cartridge_models_create,
crate::tauri_cmds::cartridges::cartridge_models_update,
crate::tauri_cmds::cartridges::cartridge_models_delete,
crate::tauri_cmds::cartridges::cartridges_suggest_brand,
crate::tauri_cmds::cartridges::cartridges_suggest_model,
crate::tauri_cmds::cartridges::cartridges_suggest_compat_printer,
crate::tauri_cmds::cartridges::cartridges_suggest_location,
```

---

### `crates/trackly-infra/src/test_support/test_db.rs` (обновить assertion, test)

**Паттерн** (строки 36–47):
```rust
// Строка 41: изменить 15 → 16
assert_eq!(user_version, 16);  // было 15
```

---

### Тесты: `cartridges_crud.rs`, `cartridges_numbering.rs`, `cartridges_lifecycle.rs`, `cartridges_search.rs`, `cartridges_low_stock.rs`, `cartridges_history.rs`

**Аналоги:** `acts_crud.rs`, `acts_numbering.rs`, `acts_returns.rs`, `acts_search.rs`

**Паттерн setup-функции** (строки 20–25 acts_crud.rs):
```rust
use trackly_infra::test_support::test_writer_and_readers;

fn make_cartridge_service() -> (CartridgeService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = CartridgeService::new(writer, readers, clock);
    (svc, dir)
}
```

**Паттерн tokio timeout** (строки 60–62 acts_crud.rs):
```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_cartridge_happy() {
    tokio::time::timeout(Duration::from_secs(30), async {
        // ...
    }).await.expect("no timeout");
}
```

**Паттерн concurrent numbering test** (строки 21–80 acts_numbering.rs — полностью повторить для `cartridge_seq`):
```rust
// Spawn 50 concurrent writer.execute closures, каждая делает:
//   BEGIN IMMEDIATE → increment_counter_in_tx("cartridge_seq") → INSERT cartridges
// Verify: все 50 кодов уникальны и имеют формат C-NNNNNN.
```

---

### UI: `CartridgesPage.svelte` (component, request-response)

**Аналог:** `ui/src/features/acts/ActsPage.svelte`

**Паттерн двух табов** (адаптировать из ActsPage):
```svelte
<script lang="ts">
  import CartridgesSearchAndTabs from './CartridgesSearchAndTabs.svelte';
  import LowStockBanner from './LowStockBanner.svelte';
  import CartridgesMasterDetail from './CartridgesMasterDetail.svelte';
  import ModelsList from './ModelsList.svelte';

  let activeTab = $state<'cartridges' | 'models'>('cartridges');
</script>

<main class="page-content">
  <CartridgesSearchAndTabs {activeTab} onTabChange={(t) => (activeTab = t)} ... />
  {#if activeTab === 'cartridges'}
    <LowStockBanner />
    <CartridgesMasterDetail>
      {#snippet master()}...{/snippet}
      {#snippet detail()}...{/snippet}
    </CartridgesMasterDetail>
  {:else}
    <ModelsList />
  {/if}
</main>
```

---

### UI: `CartridgesSearchAndTabs.svelte` (component, request-response)

**Аналог:** `ui/src/features/acts/ActsSearchAndTabs.svelte`

**Паттерн поиска + табов со счётчиками** (строки 1–133 ActsSearchAndTabs.svelte):
```svelte
<script lang="ts">
  import Input from '$lib/components/Input.svelte';
  import Badge from '$lib/components/Badge.svelte';

  // Два таба: 'cartridges' | 'models' (вместо трёх у Acts)
  type TabKey = 'cartridges' | 'models';

  // Debounce 250ms — идентично
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  function handleInput(v: string) {
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => onSearchChange(v), 250);
  }
</script>
```

---

### UI: `CartridgesMasterDetail.svelte` (component, request-response)

**Аналог:** `ui/src/features/acts/ActsMasterDetail.svelte`

**Паттерн 35/65 grid** (строки 1–53 ActsMasterDetail.svelte — копировать без изменений):
```svelte
<div class="master-detail">
  <aside class="master">{@render master()}</aside>
  <section class="detail">{@render detail()}</section>
</div>
<style lang="scss">
  .master-detail {
    display: grid;
    grid-template-columns: 35% 65%;
    gap: var(--space-md);
    min-height: calc(100vh - 240px);
  }
  .master {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    overflow: hidden;
    min-width: 320px;
  }
  .detail {
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    overflow: auto;
    min-width: 480px;
  }
  @media (max-width: 1099px) {
    .master-detail { grid-template-columns: 380px 1fr; min-width: 900px; }
  }
</style>
```

---

### UI: `CartridgesList.svelte` (component, request-response)

**Аналог:** `ui/src/features/acts/ActsList.svelte`

**Паттерн list с пустыми состояниями** (строки 80–165 ActsList.svelte):
```svelte
<div class="acts-list">
  {#if loading && items.length === 0}
    <div class="loading"><Spinner size="md" /></div>
  {:else if items.length === 0}
    <div class="empty">
      <h3 class="empty-heading">{emptyConfig.heading}</h3>
      <p class="empty-body">{emptyConfig.body}</p>
      <!-- ... -->
    </div>
  {:else}
    <div class="rows">
      {#each items as c (c.id)}
        <CartridgeListRow cartridge={c} selected={c.id === selectedId} {onSelect} />
      {/each}
    </div>
    <footer class="pagination">...</footer>
  {/if}
</div>
```

**Пустые состояния** (из UI-SPEC §Пустые состояния):
- Нет картриджей → «Картриджей пока нет» / «Добавьте первый картридж…» / «+ Добавить картридж»
- Фильтр пустой → «Ничего не найдено» / «Попробуйте изменить фильтры…»

---

### UI: `CartridgeListRow.svelte` (component, request-response)

**Аналог:** `ui/src/features/acts/ActListRow.svelte`

**Паттерн строки списка** (строки 52–139 ActListRow.svelte):
```svelte
<div class="row" class:selected role="button" tabindex="0"
     aria-pressed={selected} onclick={handleClick} onkeydown={handleKeydown}>
  <div class="top">
    <span class="number" style="font-variant-numeric: tabular-nums">{cartridge.code}</span>
    <!-- Бренд + Модель через cartridge.model_brand + cartridge.model_name -->
    <span class="badge-wrap"><Badge variant={statusVariant}>{cartridge.status_name}</Badge></span>
    <!-- kebab: CartridgeContextMenu -->
  </div>
  <div class="bottom">
    <span class="location">{cartridge.location ?? '—'}</span>
  </div>
</div>
```

Badge-варианты статусов (из UI-SPEC §Badge-цвета статусов):
- `status_id=1` (На складе) → `variant="success"`
- `status_id=2` (В работе) → `variant="accent"`
- `status_id=3` (На заправке) → `variant="warning"`
- `status_id=4` (Списано) → `variant="default"`

---

### UI: `CartridgeContextMenu.svelte` (component, event-driven)

**Аналог:** `ui/src/features/devices/DeviceContextMenu.svelte`

**Паттерн portal + mousedown-outside** (строки 1–245 DeviceContextMenu.svelte — копировать структуру полностью):
```svelte
<script lang="ts">
  import { portal } from '$lib/utils/portal';  // use:portal для fixed-позиционирования
  // menuX / menuY рассчитываются из rect = triggerEl.getBoundingClientRect()
  // menuX = rect.right - 160;  // 160px — min-width меню
  // menuY = rect.bottom + 4;

  // Status-dependent пункты — показывать только допустимые переходы (D-Op-Transitions-01)
  const menuItems = $derived.by(() => {
    const items = [];
    if (cartridge.status_id === 1) {  // На складе
      items.push({ label: 'Установить в принтер', action: () => onInstall(cartridge) });
      items.push({ label: 'Отправить на заправку', action: () => onToRefill(cartridge) });
    }
    if (cartridge.status_id === 2) {  // В работе
      items.push({ label: 'Вернуть на склад', action: () => onReturnToStock(cartridge) });
    }
    if (cartridge.status_id === 3) {  // На заправке
      items.push({ label: 'Забрать с заправки', action: () => onFromRefill(cartridge) });
    }
    items.push({ label: 'Редактировать', action: () => onEdit(cartridge) });
    // separator + destructive
    if (cartridge.status_id === 1) {
      items.push({ label: 'Списать', action: () => onWriteOff(cartridge), destructive: true });
    }
    items.push({ label: 'Удалить', action: () => onDelete(cartridge), destructive: true });
    return items;
  });
</script>

<!-- kebab trigger: aria-label="Действия с картриджем {code}" -->
<button bind:this={triggerEl} class="kebab-btn"
        aria-label="Действия с картриджем {cartridge.code}"
        aria-expanded={menuOpen} onclick={toggleMenu}>⋮</button>

<!-- Portal menu: z-index:2000, :global(.ctx-menu-portal) стили из DeviceContextMenu -->
{#if menuOpen}
  <div use:portal class="ctx-menu-portal" role="menu" tabindex="-1"
       style="left:{menuX}px; top:{menuY}px;">
    {#each menuItems as item}
      <button class="ctx-menu-item" class:ctx-menu-item--destructive={item.destructive}
              role="menuitem" onclick={item.action}>{item.label}</button>
    {/each}
  </div>
{/if}
```

Закрытие меню (строки 79–94 DeviceContextMenu.svelte):
```svelte
<svelte:window onmousedown={handleBodyMousedown} onscroll={handleScrollOrResize} onresize={handleScrollOrResize} />

function handleBodyMousedown(e: MouseEvent) {
  if (!menuOpen) return;
  const target = e.target as HTMLElement;
  if (triggerEl && triggerEl.contains(target)) return;
  if (target.closest('.ctx-menu-portal')) return;  // клик внутри меню — игнор
  menuOpen = false;
}
```

---

### UI: `CartridgeFilters.svelte` (component, request-response)

**Аналог:** `ui/src/features/devices/DeviceFilters.svelte`

**Паттерн switch-bar со счётчиками** (строки 41–113 DeviceFilters.svelte):
```svelte
<script lang="ts">
  // cartridge statuses (из V001): id=null→Все, 1→На складе, 2→В работе, 3→На заправке, 4→Списано
  const STATUSES = [
    { id: null, label: 'Все' },
    { id: 1,    label: 'На складе' },
    { id: 2,    label: 'В работе' },
    { id: 3,    label: 'На заправке' },
    { id: 4,    label: 'Списано' },
  ] as const;

  // totalCount = sum of all counts
  const totalCount = $derived(Array.from(counts.values()).reduce((sum, c) => sum + c, 0));

  // Дополнительные фильтры (в отличие от DeviceFilters, без group-toggle):
  // kindFilter: null | 1 | 2  (Все / Картридж / Фотобарабан)
  // modelFilter: null | number (model_id)
</script>

<!-- Search input с debounce 250ms — идентично DeviceFilters строки 33–39 -->
<!-- status-bar tablist — идентично строки 85–101 DeviceFilters -->
<!-- Дополнительный ряд: Select «Тип» + Select «Модель» -->
```

**CSS классы** (строки 115–244 DeviceFilters.svelte — копировать без изменений):
```scss
.status-tab.active {
  color: var(--color-accent);
  border-bottom-color: var(--color-accent);
  font-weight: var(--font-weight-medium);
}
.count-badge.count-active {
  background: color-mix(in srgb, var(--color-accent) 15%, transparent);
  color: var(--color-accent);
}
```

---

### UI: `CartridgeDetail.svelte` (component, request-response)

**Аналог:** `ui/src/features/acts/ActDetail.svelte`

**Паттерн header + секции + кнопки** (строки 1–100 ActDetail.svelte):
```svelte
<script lang="ts">
  // Код отображается с font-variant-numeric: tabular-nums (как number в ActDetail)
  // Кнопки действий зависят от status_id картриджа
</script>

<div class="act-detail" aria-live="polite">
  {#if cartridge === null}
    <!-- empty state: «Выберите картридж» -->
  {:else}
    <header class="detail-header">
      <h2 class="detail-title" style="font-variant-numeric: tabular-nums">
        {cartridge.code}
      </h2>
      <!-- статус-badge + название модели -->
      <div class="actions">
        <!-- кнопки size="sm" по статусу (как в ActDetail.svelte строки 62–83) -->
      </div>
    </header>

    <!-- Секция полей экземпляра -->
    <section class="section">
      <!-- Расположение, Держатель, Состояние заряда, Примечания -->
    </section>

    <!-- История перемещений из audit_log (D-History-01) -->
    <section class="section">
      <h3 class="section-heading">История перемещений</h3>
      {#each history as entry}
        <!-- «12.06.2026 — Установлен в принтер; выдал Иванов, получил Петров» -->
        <!-- row-height: --row-height-dense (32px) -->
      {/each}
    </section>
  {/if}
</div>
```

---

### UI: `OperationModal.svelte` (component, event-driven)

**Аналог:** `ui/src/features/acts/ReturnModal.svelte`

**Паттерн Modal + $state runes + $effect reset** (строки 1–71 ReturnModal.svelte):
```svelte
<script lang="ts">
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import DatePicker from '$lib/components/DatePicker.svelte';
  import PersonAutocomplete from '$lib/components/PersonAutocomplete.svelte';
  import LocationAutocomplete from '$lib/components/LocationAutocomplete.svelte';
  import Select from '$lib/components/Select.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { cartridges } from './api';

  type Op = 'install' | 'return_to_stock' | 'to_refill' | 'from_refill' | 'write_off';

  interface Props {
    open: boolean;
    op: Op;
    cartridge: CartridgeDto | null;
    onClose: () => void;
    onSuccess: () => void;
  }

  // Поля формы сбрасываются через $effect при открытии (как в ReturnModal строки 44–70)
  let dateUtc = $state(Math.floor(Date.now() / 1000));
  let givenByName = $state('');
  let givenToName = $state('');
  let location = $state('');
  let stateId = $state<number>(1);  // default зависит от op
  let notes = $state('');
  let submitting = $state(false);

  $effect(() => {
    if (open) {
      dateUtc = Math.floor(Date.now() / 1000);
      givenByName = '';
      givenToName = '';
      location = '';
      // Default заряда по операции (D-Op-Fields-01):
      stateId = op === 'from_refill' ? 1 : 3;  // Полный или Пустой
      notes = '';
    }
  });

  // Заголовок модала по op (UI-SPEC §Заголовки OperationModal)
  const modalTitle = $derived({
    install: 'Установка в принтер',
    return_to_stock: 'Возврат на склад',
    to_refill: 'Отправка на заправку',
    from_refill: 'Получение с заправки',
    write_off: 'Списание картриджа',
  }[op]);
</script>

<Modal open={open} title={modalTitle} size="md" onClose={onClose}>
  <!-- Поля рендерятся условно по op — только нужные -->
  {#if op === 'install' || op === 'to_refill'}
    <DatePicker label="Дата" bind:value={dateUtc} />
    <PersonAutocomplete label="Кто выдал" bind:value={givenByName} required />
    <PersonAutocomplete label="Кому выдал" bind:value={givenToName} required />
    <LocationAutocomplete label="Расположение" bind:value={location} required />
  {:else if op === 'return_to_stock' || op === 'from_refill'}
    <Select label="Состояние заряда" bind:value={stateId} options={STATE_OPTIONS} />
    <LocationAutocomplete label="Расположение" bind:value={location} required />
    <Textarea label="Примечание" bind:value={notes} />
  {:else if op === 'write_off'}
    <DatePicker label="Дата" bind:value={dateUtc} />
    <Textarea label="Причина / Примечание" bind:value={notes} />
  {/if}

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button variant="primary" loading={submitting} onclick={handleSubmit}>
      {CONFIRM_LABELS[op]}
    </Button>
  {/snippet}
</Modal>
```

---

### UI: `CartridgeFormModal.svelte` (component, CRUD)

**Аналог:** `ui/src/features/devices/DeviceFormModal.svelte`

**Паттерн {#key openInstanceCounter} для сброса формы** (строки 36–55 DeviceFormModal.svelte):
```svelte
let openInstanceCounter = $state(0);
let _wasOpen = $state(false);

$effect(() => {
  if (open && !_wasOpen) openInstanceCounter += 1;
  _wasOpen = open;
});

// Под Modal: {#key openInstanceCounter}<CartridgeFormBody .../>{/key}
```

**Паттерн modalTitle / submitLabel** (строки 28–31 DeviceFormModal.svelte):
```svelte
const isEdit = $derived(target !== null);
const modalTitle = $derived(isEdit ? 'Редактирование картриджа' : 'Новый картридж');
const submitLabel = $derived(isEdit ? 'Сохранить изменения' : 'Добавить картридж');
```

---

### UI: `ModelFormModal.svelte` (component, CRUD)

**Аналог:** `ui/src/features/devices/DeviceFormModal.svelte`

**Ключевые отличия:**
- `size="wide"` (960px) из-за CompatibilityEditor
- Поле «Цвет» скрыто когда `kindId === 2` (Фотобарабан):
```svelte
{#if kindId !== 2}
  <Select label="Цвет" bind:value={color} options={COLOR_OPTIONS} />
{/if}
```
- CompatibilityEditor — дочерний компонент внутри формы

---

### UI: `api.ts` (utility, request-response)

**Аналог:** `ui/src/lib/api/acts.ts`

**Паттерн apiCall wrapper** (строки 1–63 acts.ts):
```typescript
import { apiCall } from '$lib/api/client';
import type { CartridgeDto, CartridgeFilter, CartridgeListResponse,
               CartridgeModelDto, CartridgeTransitionPayload, Pagination } from '../../bindings';

export const cartridges = {
  list: (filter: CartridgeFilter, pagination: Pagination) =>
    apiCall<CartridgeListResponse>('cartridges_list', { filter, pagination }),

  get: (id: number) => apiCall<CartridgeDto>('cartridges_get', { id }),

  create: (payload: CartridgeCreateDto) => apiCall<CartridgeDto>('cartridges_create', { payload }),

  transition: (payload: CartridgeTransitionPayload) =>
    apiCall<CartridgeDto>('cartridges_transition', { payload }),

  delete: (id: number, version: number) => apiCall<null>('cartridges_delete', { id, version }),

  statusCounts: () => apiCall<CartridgeCountsDto>('cartridges_status_counts'),

  getHistory: (id: number) => apiCall<AuditEntryDto[]>('cartridges_get_history', { id }),

  lowStock: () => apiCall<LowStockItem[]>('cartridges_low_stock'),

  search: (query: string, filter: CartridgeFilter, pagination: Pagination) =>
    apiCall<CartridgeListResponse>('cartridges_search', { query, filter, pagination }),

  // Models CRUD
  modelsList: () => apiCall<CartridgeModelDto[]>('cartridge_models_list'),
  modelsCreate: (payload: CartridgeModelCreateDto) =>
    apiCall<CartridgeModelDto>('cartridge_models_create', { payload }),
  modelsUpdate: (id: number, version: number, patch: CartridgeModelPatchDto) =>
    apiCall<CartridgeModelDto>('cartridge_models_update', { id, version, patch }),
  modelsDelete: (id: number, version: number) =>
    apiCall<null>('cartridge_models_delete', { id, version }),

  // Autocomplete suggest endpoints
  suggestBrand: (prefix: string) => apiCall<string[]>('cartridges_suggest_brand', { prefix }),
  suggestModel: (brand: string, prefix: string) =>
    apiCall<string[]>('cartridges_suggest_model', { brand, prefix }),
  suggestCompatPrinter: (field: 'brand' | 'model', prefix: string) =>
    apiCall<string[]>('cartridges_suggest_compat_printer', { field, prefix }),
};
```

---

## Shared Patterns

### Single-writer + BEGIN IMMEDIATE транзакция

**Источник:** `crates/trackly-infra/src/db/writer_worker.rs::WriterHandle::execute`
**Применить к:** всем service-методам с мутациями (create, update, delete, transition)

```rust
// Паттерн из acts_sqlite.rs строки 365–383:
// WriterHandle::execute возвращает BEGIN IMMEDIATE автоматически через rusqlite::Connection::transaction()
let result = self.writer.execute(move |conn| {
    let tx = conn.transaction().map_err(map_rusqlite)?;  // BEGIN IMMEDIATE
    // ... mutations ...
    tx.commit().map_err(map_rusqlite)?;
    Ok(result)
}).await?;
```

### audit_log insert паттерн

**Источник:** `crates/trackly-infra/src/repos/audit_log_sqlite.rs` строки 27–57
**Применить к:** cartridges_create, cartridges_transition, cartridge_models_create, cartridge_models_delete

```rust
use trackly_infra::repos::audit_log_sqlite::{AuditEntry, SqliteAuditLogRepository};

audit_repo.insert(&tx, AuditEntry {
    entity_type: "cartridge",   // или "cartridge_model"
    entity_id: id,
    action: "custom:install",   // см. action-коды ниже
    user_id: None,              // Phase 4: ВСЕГДА NULL
    before_json: Some(serde_json::to_string(&before_snapshot).unwrap()),
    after_json: Some(serde_json::to_string(&after_snapshot).unwrap()),
    payload_json: Some(serde_json::to_string(&payload_fields).unwrap()),
    created_at_utc: now,
})?;
```

Action-коды (D-History-01, префикс `custom:` для не-CRUD операций):
- `"create"` — создание экземпляра с авто-кодом
- `"custom:cartridge_code_override"` — создание с пользовательским кодом
- `"custom:install"` — установить в принтер (status: На складе → В работе)
- `"custom:return_to_stock"` — вернуть на склад (status: В работе → На складе)
- `"custom:to_refill"` — отправить на заправку (status: На складе → На заправке)
- `"custom:from_refill"` — забрать с заправки (status: На заправке → На складе)
- `"custom:write_off"` — списать (status: → Списано)
- `"update"` — редактирование экземпляра
- `"delete"` — soft-delete экземпляра

### AppError типы

**Источник:** `crates/trackly-core/src/error.rs`
**Применить к:** всем service- и repo-методам

```rust
// Conflict на UNIQUE code
AppError::Conflict { reason: format!("Картридж с кодом «{}» уже существует", code) }

// OptimisticLockMismatch при UPDATE version != expected
AppError::OptimisticLockMismatch { entity: "cartridge", id, expected: version, actual }

// Validation на пустые обязательные поля
AppError::Validation { field: "model_id".into(), message: "Выберите модель картриджа".into() }

// NotFound при get по несуществующему id
AppError::NotFound { entity: "cartridge", id }

// Conflict при удалении модели с живыми экземплярами
AppError::Conflict { reason: format!("Нельзя удалить модель: она используется {} картриджами", n) }
```

### specta::Type на i64 полях

**Источник:** `crates/trackly-app/src/dto/device.rs` строки 32–60
**Применить к:** всем полям `i64` в CartridgeDto, CartridgeModelDto, DTOs транзиций

```rust
#[specta(type = i32)]   pub id: i64,
#[specta(type = i32)]   pub version: i64,
#[specta(type = Option<i32>)] pub state_id: Option<i64>,
```

### Svelte 5 runes (без store)

**Источник:** Все существующие .svelte компоненты фич acts/devices
**Применить к:** всем новым компонентам cartridges

```svelte
// $state — реактивное состояние
let selectedId = $state<number | null>(null);
let loading = $state(false);

// $derived — вычисляемые значения
const statusVariant = $derived(statusToVariant(cartridge.status_id));

// $effect — сайд-эффекты (загрузка данных, сброс формы)
$effect(() => {
  if (selectedId !== null) {
    loadCartridge(selectedId);
  }
});

// $props — props компонента
const { cartridge, onSelect }: Props = $props();
```

### Modal backdrop discipline

**Источник:** `ui/src/lib/components/Modal.svelte` (G-1 fix Phase 3.1)
**Применить к:** CartridgeFormModal, OperationModal, ModelFormModal, confirm-диалоги

```svelte
<!-- НЕ использовать onclick на backdrop — только onmousedown/onmouseup паттерн из Modal.svelte -->
<!-- Передавать size="md" или size="wide" через prop -->
<Modal open={open} title="..." size="md" onClose={onClose}>
```

---

## No Analog Found

| Файл | Роль | Data Flow | Причина |
|------|------|-----------|---------|
| `ui/src/features/cartridges/LowStockBanner.svelte` | component | request-response | Нет баннеров предупреждения в Phase 2/3; паттерн простой (условный рендер + список + warning-цвета из UI-SPEC) |
| `ui/src/features/cartridges/CompatibilityEditor.svelte` | component | CRUD | Нет добавляемых списков пар в существующем коде; паттерн создаётся с нуля (массив пар + autocomplete на каждое поле) |

Для этих двух файлов планировщик ориентируется на UI-SPEC §ModelFormModal layout и §CompatibilityEditor, а также на токены из `_tokens.scss`:
- `LowStockBanner`: `color-mix(in srgb, var(--color-warning) 10%, transparent)` для фона, inline SVG 16×16 иконка предупреждения, `--color-warning` для рамки/иконки
- `CompatibilityEditor`: `grid-template-columns: 1fr 1fr 28px` (Бренд / Модель / кнопка удаления), `LocationAutocomplete`-паттерн focus-open на каждом поле

---

## Metadata

**Аналоги искались в:** `crates/trackly-core/`, `crates/trackly-infra/`, `crates/trackly-app/`, `migrations/`, `ui/src/features/acts/`, `ui/src/features/devices/`, `ui/src/lib/api/`, `ui/src/lib/components/`
**Файлов прочитано:** 36
**Дата маппинга:** 2026-06-07
