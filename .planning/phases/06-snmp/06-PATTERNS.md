# Phase 6: Принтеры (SNMP-мониторинг) и Заявки — Pattern Map

**Mapped:** 2026-06-14
**Files analyzed:** 27 new files + 3 modified files
**Analogs found:** 27 / 27 (все с точным или role-match аналогом)

---

## File Classification

| New / Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---------------------|------|-----------|----------------|---------------|
| `crates/trackly-core/src/ports/printers.rs` | core port | CRUD + event-driven | `crates/trackly-core/src/ports/cartridges.rs` | exact |
| `crates/trackly-core/src/ports/requests.rs` | core port | CRUD + request-response | `crates/trackly-core/src/ports/devices.rs` | exact |
| `crates/trackly-core/src/ports/snmp.rs` | core port (trait) | event-driven / I/O | `crates/trackly-core/src/primitives/clock.rs` | role-match (infra trait) |
| `crates/trackly-core/src/domain/printers.rs` | core domain | batch + streaming | `crates/trackly-core/src/domain/cartridges.rs` | exact |
| `crates/trackly-core/src/domain/requests.rs` | core domain | request-response | `crates/trackly-core/src/domain/cartridges.rs` | role-match |
| `crates/trackly-infra/src/repos/printers_sqlite.rs` | infra repo | CRUD | `crates/trackly-infra/src/repos/cartridges_sqlite.rs` | exact |
| `crates/trackly-infra/src/repos/requests_sqlite.rs` | infra repo | CRUD | `crates/trackly-infra/src/repos/cartridges_sqlite.rs` | exact |
| `crates/trackly-infra/src/snmp/real.rs` | infra adapter | event-driven / I/O | no direct analog | partial (new capability) |
| `crates/trackly-infra/src/snmp/mock.rs` | infra adapter (test) | event-driven | no direct analog | partial (new capability) |
| `crates/trackly-app/src/services/printer_service.rs` | app service | CRUD + batch + event-driven | `crates/trackly-app/src/services/cartridge_service.rs` | exact |
| `crates/trackly-app/src/services/request_service.rs` | app service | CRUD + request-response | `crates/trackly-app/src/services/cartridge_service.rs` | exact |
| `crates/trackly-app/src/dto/printer.rs` | app dto | request-response | `crates/trackly-app/src/dto/cartridge.rs` | exact |
| `crates/trackly-app/src/dto/request.rs` | app dto | request-response | `crates/trackly-app/src/dto/cartridge.rs` | exact |
| `crates/trackly-app/src/tauri_cmds/printers.rs` | app tauri_cmd | request-response | `crates/trackly-app/src/tauri_cmds/cartridges.rs` | exact |
| `crates/trackly-app/src/tauri_cmds/requests.rs` | app tauri_cmd | request-response | `crates/trackly-app/src/tauri_cmds/cartridges.rs` | exact |
| `crates/trackly-app/src/http/printers.rs` | app http | request-response | `crates/trackly-app/src/http/cartridges.rs` | exact |
| `crates/trackly-app/src/http/requests.rs` | app http | request-response | `crates/trackly-app/src/http/cartridges.rs` | exact |
| `crates/trackly-app/src/http/ws.rs` | app http (WS) | pub-sub | no direct analog (new capability) | partial |
| `crates/trackly-app/src/context.rs` (modified) | config / composition root | — | self (extend existing) | exact |
| `ui/src/features/printers/PrintersPage.svelte` | ui feature (page) | request-response | `ui/src/features/cartridges/CartridgesPage.svelte` | exact |
| `ui/src/features/printers/PrintersMasterDetail.svelte` | ui component | — | `ui/src/features/cartridges/CartridgesMasterDetail.svelte` | exact |
| `ui/src/features/printers/PrinterDetail.svelte` | ui component | request-response | `ui/src/features/cartridges/CartridgeDetail.svelte` | exact |
| `ui/src/features/printers/DiscoveryModal.svelte` | ui component (modal) | request-response | `ui/src/features/cartridges/OperationModal.svelte` | role-match |
| `ui/src/features/printers/PrinterAlertBanner.svelte` | ui component (alert) | — | `ui/src/features/cartridges/LowStockBanner.svelte` | exact |
| `ui/src/features/printers/api.ts` | ui api wrapper | request-response | `ui/src/features/cartridges/api.ts` | exact |
| `ui/src/features/requests/RequestsPage.svelte` | ui feature (page) | request-response | `ui/src/features/cartridges/CartridgesPage.svelte` | role-match |
| `ui/src/features/requests/RequestFormModal.svelte` | ui component (modal) | request-response | `ui/src/features/cartridges/OperationModal.svelte` | role-match |
| `ui/src/features/requests/api.ts` | ui api wrapper | request-response | `ui/src/features/cartridges/api.ts` | exact |
| `ui/src/lib/api/ws.ts` | ui lib (WS client) | pub-sub | `ui/src/lib/api/client.ts` | partial |
| `migrations/V020__printers.sql` | migration | — | `migrations/V005__cartridges.sql` | exact |
| `migrations/V021__oid_profiles_seed.sql` | migration (seed) | — | `migrations/V001__init_pragmas_and_lookups.sql` | role-match |
| `migrations/V022__printer_readings.sql` | migration | — | `migrations/V005__cartridges.sql` | role-match |
| `migrations/V023__printer_alerts.sql` | migration | — | `migrations/V005__cartridges.sql` | role-match |
| `migrations/V024__request_categories.sql` | migration (seed) | — | `migrations/V001__init_pragmas_and_lookups.sql` | role-match |

---

## Pattern Assignments

### `crates/trackly-core/src/ports/printers.rs` (core port, CRUD + event-driven)

**Analog:** `crates/trackly-core/src/ports/cartridges.rs`

**Module doc pattern** (lines 1-13):
```rust
//! `PrinterRepository` port — repository trait for the Printers entity.
//!
//! Pattern: associated `type Conn` keeps rusqlite out of trackly-core.
//! Write methods that participate in larger transactions are NOT part of
//! this trait — they live as `*_in_tx` helpers on `SqlitePrinterRepository`
//! and are orchestrated by the service layer inside a single
//! `WriterHandle::execute` closure.
```

**Core trait pattern** (analog lines 19-52):
```rust
pub trait PrinterRepository {
    type Conn;

    fn get(&self, conn: &Self::Conn, id: i64) -> Result<PrinterRow, AppError>;

    fn list(
        &self,
        conn: &Self::Conn,
        filter: &PrinterFilter,
        page: &Pagination,
    ) -> Result<(Vec<PrinterRow>, u64), AppError>;

    // Additional read-only methods (last_reading, alert status)
    fn get_last_reading(
        &self,
        conn: &Self::Conn,
        printer_id: i64,
    ) -> Result<Option<PrinterReadingRow>, AppError>;

    fn list_active_alerts(
        &self,
        conn: &Self::Conn,
    ) -> Result<Vec<PrinterAlertRow>, AppError>;
}
```

**Note:** Write paths (create_from_device, upsert_reading, upsert_alert) go as `*_in_tx` helpers on `SqlitePrinterRepository`, NOT in the trait — exactly as `SqliteCartridgeRepository` does with `insert_cartridge_in_tx` / `transition_in_tx`.

---

### `crates/trackly-core/src/ports/requests.rs` (core port, CRUD + request-response)

**Analog:** `crates/trackly-core/src/ports/devices.rs`

**Core trait pattern** (analog lines 19-94):
```rust
pub trait RequestRepository {
    type Conn;

    fn get(&self, conn: &Self::Conn, id: i64) -> Result<RequestRow, AppError>;

    fn list(
        &self,
        conn: &Self::Conn,
        filter: &RequestFilter,
        page: &Pagination,
    ) -> Result<(Vec<RequestRow>, u64), AppError>;

    fn counts(&self, conn: &Self::Conn) -> Result<RequestCounts, AppError>;
}
```

---

### `crates/trackly-core/src/ports/snmp.rs` (core port / trait, event-driven I/O)

**Analog:** `crates/trackly-core/src/primitives/clock.rs` (pattern: infra trait in core, two impls)

**Core trait pattern** (from RESEARCH.md Code Examples section):
```rust
// crates/trackly-core/src/ports/snmp.rs
#[async_trait::async_trait]
pub trait SnmpClient: Send + Sync {
    /// Fetch OID values. Returns None if target is unreachable/timeout.
    async fn get_oids(
        &self,
        target: &str,
        community: &str,
        oids: &[&str],
        timeout_secs: u64,
    ) -> Result<Option<Vec<OidValue>>, AppError>;

    /// Discovery probe: fetch sysObjectID + sysDescr + sysName.
    async fn probe(
        &self,
        target: &str,
        community: &str,
    ) -> Result<Option<ProbedDevice>, AppError>;
}
```

**No-IO-deps invariant:** like `Clock` trait, `SnmpClient` must have NO tokio/snmp2 imports in trackly-core — those live only in the infra impl. Enforce via `crates/trackly-core/tests/no_io_deps.rs` (analog exists).

---

### `crates/trackly-core/src/domain/printers.rs` (core domain, batch + streaming)

**Analog:** `crates/trackly-core/src/domain/cartridges.rs`

**Module doc + derives pattern** (analog lines 1-11):
```rust
//! Domain value types for the Printers entity.
//!
//! NO serde::Serialize/Deserialize or specta::Type derives here — those live
//! in the DTO layer in trackly-app. Only `#[derive(Debug, Clone, PartialEq, Eq)]`.

use crate::error::AppError;
```

**Domain row structs pattern** (analog lines 15-61):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrinterRow {
    pub id: i64,
    pub device_id: i64,               // FK → devices
    pub ip_address: Option<String>,   // None for USB-only printers
    // community NOT here — Secret<String> lives in service layer only
    pub snmp_version: String,         // "v2c"
    pub vendor: Option<String>,
    pub oid_profile_id: Option<i64>,
    pub last_seen_utc: Option<i64>,
    pub usb_host_device_id: Option<i64>,  // PRN-04 USB учёт
    // Joined from devices:
    pub device_name: Option<String>,
    pub device_location: Option<String>,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrinterReadingRow {
    pub id: i64,
    pub printer_id: i64,
    pub ts_utc: i64,
    pub toner_levels_json: String,    // JSON: {"black":{"level":45,"max":100,"pct":45}}
    pub page_count: Option<i64>,
    pub status: String,              // "ok" | "warning" | "error" | "offline"
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrinterAlertRow {
    pub id: i64,
    pub printer_id: i64,
    pub alert_type: String,          // "offline" | "error"
    pub first_seen_utc: i64,
    pub last_seen_utc: i64,
    pub acknowledged_at_utc: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidProfileRow {
    pub id: i64,
    pub name: String,               // "pantum" | "kyocera" | "hp" | "canon" | "rfc3805"
    pub vendor_prefix: String,      // sysObjectID prefix для маппинга
    pub toner_level_oid: Option<String>,
    pub toner_max_oid: Option<String>,
    pub toner_encoding: String,     // "percent" | "level_over_max"
    pub page_counter_oid: Option<String>,
    pub status_oid: String,
    pub serial_oid: Option<String>,
}
```

**Lifecycle enum pattern** (analog lines 102-194 — `CartridgeTransitionOp`):
```rust
/// Request status transitions enforced at service layer (D-Req-Lifecycle-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestTransitionOp {
    Accept,           // open → in_progress
    Reject { notes: Option<String> },    // open → rejected
    Complete { notes: Option<String>, linked_cartridge_op: bool }, // in_progress → completed
}

impl RequestTransitionOp {
    pub fn validate_from_status(&self, current: &str) -> Result<(), AppError> {
        let (expected, op_name) = match self {
            RequestTransitionOp::Accept => ("open", "Принять в работу"),
            RequestTransitionOp::Reject { .. } => ("open", "Отклонить"),
            RequestTransitionOp::Complete { .. } => ("in_progress", "Выполнить"),
        };
        if current != expected {
            return Err(AppError::Validation {
                field: "status".into(),
                message: format!(
                    "Операция «{}» недопустима для статуса «{}»",
                    op_name, current
                ),
            });
        }
        Ok(())
    }
    pub fn target_status(&self) -> &'static str {
        match self {
            RequestTransitionOp::Accept => "in_progress",
            RequestTransitionOp::Reject { .. } => "rejected",
            RequestTransitionOp::Complete { .. } => "completed",
        }
    }
    pub fn audit_action(&self) -> &'static str {
        match self {
            RequestTransitionOp::Accept => "custom:accept",
            RequestTransitionOp::Reject { .. } => "custom:reject",
            RequestTransitionOp::Complete { .. } => "custom:complete",
        }
    }
}
```

---

### `crates/trackly-infra/src/repos/printers_sqlite.rs` (infra repo, CRUD)

**Analog:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs`

**Module doc + imports pattern** (analog lines 1-24):
```rust
//! SQLite adapter for `PrinterRepository` + tx-helper methods used by the
//! service layer to compose multi-step write paths inside a single transaction.
//!
//! All SQL is parameterised through `rusqlite::params![...]`.

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use trackly_core::domain::printers::{
    OidProfileRow, PrinterAlertRow, PrinterFilter, PrinterReadingRow, PrinterRow, Pagination,
};
use trackly_core::error::AppError;
use trackly_core::ports::printers::PrinterRepository;

use crate::error_conversions::map_rusqlite;
use crate::repos::audit_log_sqlite::{AuditEntry, SqliteAuditLogRepository};

#[derive(Debug, Default, Clone)]
pub struct SqlitePrinterRepository;
```

**SELECT constant + map_row pattern** (analog lines 50-80):
```rust
const SELECT_PRINTERS: &str = "
    SELECT p.id, p.device_id, p.ip_address, p.snmp_version, p.vendor,
           p.oid_profile_id, p.last_seen_utc, p.usb_host_device_id,
           d.name AS device_name, d.location AS device_location,
           p.created_at_utc, p.updated_at_utc, p.version
      FROM printers p
      LEFT JOIN devices d ON d.id = p.device_id
";

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PrinterRow> {
    Ok(PrinterRow {
        id: row.get(0)?,
        device_id: row.get(1)?,
        ip_address: row.get(2)?,
        // ... etc
    })
}
```

**Tx-helper methods pattern** (analog lines ~86+): create `upsert_reading_in_tx`, `upsert_alert_in_tx`, `create_in_tx` — same pattern as `insert_cartridge_in_tx`.

**Retention prune helper** (new, no direct analog — plain SQL DELETE):
```rust
pub fn prune_old_readings_in_tx(
    tx: &Transaction,
    retention_cutoff_utc: i64,
    downsample_cutoff_utc: i64,
) -> Result<u64, AppError> {
    // Step 1: delete rows older than retention cutoff
    let deleted = tx.execute(
        "DELETE FROM printer_readings WHERE ts_utc < ?1",
        params![retention_cutoff_utc],
    ).map_err(map_rusqlite)? as u64;
    // Step 2: downsample rows between downsample_cutoff and retention cutoff
    // (keep one row per printer per day, delete rest)
    tx.execute(
        "DELETE FROM printer_readings
          WHERE ts_utc < ?1
            AND id NOT IN (
              SELECT MIN(id) FROM printer_readings
               WHERE ts_utc < ?1
               GROUP BY printer_id, date(ts_utc, 'unixepoch')
            )",
        params![downsample_cutoff_utc],
    ).map_err(map_rusqlite)?;
    Ok(deleted)
}
```

---

### `crates/trackly-infra/src/repos/requests_sqlite.rs` (infra repo, CRUD)

**Analog:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs`

Same structural pattern as `printers_sqlite.rs` above. Key difference: the `requests` table already exists (V006) — no CREATE needed, only read/write helpers.

**SELECT constant:**
```rust
const SELECT_REQUESTS: &str = "
    SELECT r.id, r.request_type, r.status,
           r.requested_by_user_id, r.assigned_to_user_id,
           r.printer_device_id, r.cartridge_model_id,
           r.description, r.resolution_notes,
           u.display_name AS requester_name,
           d.name AS printer_name,
           r.created_at_utc, r.updated_at_utc, r.deleted_at_utc, r.version
      FROM requests r
      LEFT JOIN users u ON u.id = r.requested_by_user_id
      LEFT JOIN devices d ON d.id = r.printer_device_id
";
```

**Lifecycle tx-helper** (mirrors `transition_in_tx` from cartridges):
```rust
pub fn transition_in_tx(
    tx: &Transaction,
    request_id: i64,
    version: i64,
    op: &RequestTransitionOp,
    assigned_to: Option<i64>,
    now_utc: i64,
) -> Result<(), AppError> { ... }
```

---

### `crates/trackly-infra/src/snmp/real.rs` (infra adapter, event-driven I/O)

**No direct analog** in codebase (new capability). Use RESEARCH.md patterns directly.

**Import pattern** (must stay in trackly-infra only, never trackly-core):
```rust
use snmp2::AsyncSession;
use std::time::Duration;
use tokio::time::timeout;
use trackly_core::error::AppError;
use trackly_core::ports::snmp::{OidValue, ProbedDevice, SnmpClient};
use async_trait::async_trait;

pub struct RealSnmpClient;

#[async_trait]
impl SnmpClient for RealSnmpClient {
    async fn get_oids(
        &self,
        target: &str,
        community: &str,
        oids: &[&str],
        timeout_secs: u64,
    ) -> Result<Option<Vec<OidValue>>, AppError> {
        let mut sess = match AsyncSession::new_v2c(target, community.as_bytes(), 0).await {
            Ok(s) => s,
            Err(_) => return Ok(None),
        };
        // ALWAYS wrap in timeout — AsyncSession has no built-in timeout (Pitfall 1)
        let pdu = match timeout(
            Duration::from_secs(timeout_secs),
            sess.get(&parsed_oids)
        ).await {
            Ok(Ok(pdu)) => pdu,
            _ => return Ok(None), // timeout or error = unreachable
        };
        // parse pdu.varbinds → Vec<OidValue>
        Ok(Some(parse_varbinds(pdu)))
    }
}
```

---

### `crates/trackly-infra/src/snmp/mock.rs` (infra adapter test, event-driven)

**No direct analog.** Pattern: deterministic fixtures returning preset values for UI dev/tests.

```rust
use std::collections::HashMap;
use async_trait::async_trait;
use trackly_core::ports::snmp::{OidValue, ProbedDevice, SnmpClient};
use trackly_core::error::AppError;

/// Fixture for a single "printer" in the mock.
#[derive(Clone)]
pub struct PrinterFixture {
    pub toner_pct: u8,
    pub page_count: i64,
    pub status: &'static str,    // "ok" | "warning" | "error" | "offline"
    pub vendor: &'static str,
    pub model: &'static str,
    pub sys_object_id: &'static str,
}

pub struct MockSnmpClient {
    pub fixtures: HashMap<String, PrinterFixture>,
}

impl MockSnmpClient {
    /// Sensible defaults for dev macOS (no real printers).
    pub fn default_fixtures() -> Self {
        let mut map = HashMap::new();
        map.insert("192.168.1.100".into(), PrinterFixture {
            toner_pct: 45, page_count: 12345, status: "ok",
            vendor: "Pantum", model: "BM5100ADN",
            sys_object_id: "1.3.6.1.4.1.40093.1",
        });
        map.insert("192.168.1.101".into(), PrinterFixture {
            toner_pct: 8, page_count: 54321, status: "warning",
            vendor: "HP", model: "LaserJet M403dn",
            sys_object_id: "1.3.6.1.4.1.11.2.3.9.1",
        });
        // Simulate offline printer for alert testing:
        map.insert("192.168.1.102".into(), PrinterFixture {
            toner_pct: 0, page_count: 0, status: "offline",
            vendor: "Canon", model: "iR2206",
            sys_object_id: "1.3.6.1.4.1.1602.1.1",
        });
        Self { fixtures: map }
    }
}
```

**Runtime switching pattern** (from RESEARCH.md, used in `AppCtx::build`):
```rust
let snmp_client: Arc<dyn SnmpClient + Send + Sync> =
    if config.snmp.use_mock || std::env::var("TRACKLY_SNMP_MOCK").is_ok() {
        Arc::new(MockSnmpClient::default_fixtures())
    } else {
        Arc::new(RealSnmpClient)
    };
```

---

### `crates/trackly-app/src/services/printer_service.rs` (app service, CRUD + batch + event-driven)

**Analog:** `crates/trackly-app/src/services/cartridge_service.rs`

**Struct + new() pattern** (analog lines 33-55):
```rust
#[derive(Clone)]
pub struct PrinterService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) printer_repo: Arc<SqlitePrinterRepository>,
    pub(crate) audit_repo: Arc<SqliteAuditLogRepository>,
    // Runtime-configured SNMP client (D-Mock-01)
    pub(crate) snmp_client: Arc<dyn SnmpClient + Send + Sync>,
    // Channel for on-demand single-printer refresh (D-Poll-01)
    pub(crate) poll_tx: tokio::sync::mpsc::Sender<i64>,
    // WS broadcast sender (D-Notify-01)
    pub(crate) ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>,
}

impl PrinterService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
        snmp_client: Arc<dyn SnmpClient + Send + Sync>,
        poll_tx: tokio::sync::mpsc::Sender<i64>,
        ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>,
    ) -> Self { ... }
```

**Read path pattern** (analog lines 325-337 — `get` via `spawn_blocking`):
```rust
pub async fn get(&self, id: i64) -> Result<PrinterDto, AppError> {
    let readers = self.readers.clone();
    let repo = self.printer_repo.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        let row = repo.get(&conn, id)?;
        Ok(PrinterDto::from(row))
    })
    .await
    .map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking: {e}") })?
}
```

**Write path pattern** (analog lines 104-171 — `create` via `writer.execute`):
```rust
pub async fn create_from_device(
    &self,
    payload: PrinterCreateDto,
    caller: &Identity,
) -> Result<PrinterDto, AppError> {
    authorize(caller, &Action::MutatePrinters)?;
    let now = self.clock.unix_seconds();
    let printer_repo = self.printer_repo.clone();
    let audit_repo = self.audit_repo.clone();

    let printer_id = self.writer.execute(move |conn| {
        let tx = conn.transaction().map_err(map_rusqlite)?;
        let id = printer_repo.create_in_tx(&tx, &payload, now)?;
        audit_repo.insert(&tx, AuditEntry {
            entity_type: "printer",
            entity_id: id,
            action: "create",
            user_id: Some(caller.user_id),
            ..Default::default()
        })?;
        tx.commit().map_err(map_rusqlite)?;
        Ok(id)
    }).await?;

    self.get(printer_id).await
}
```

**Background poll task pattern** (from RESEARCH.md + `crates/trackly-app/src/shutdown.rs`):
```rust
/// Запустить фоновый poll-task.
/// Вызывается из AppCtx::build с child CancellationToken (паттерн Phase 5 D-Server-01).
pub async fn run_poll_task(
    printer_svc: Arc<PrinterService>,
    mut on_demand_rx: tokio::sync::mpsc::Receiver<i64>,
    shutdown: tokio_util::sync::CancellationToken,
) {
    let mut interval = tokio::time::interval(
        std::time::Duration::from_secs(300) // default 5 min, read from app_settings
    );
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                printer_svc.poll_all().await;
            }
            Some(printer_id) = on_demand_rx.recv() => {
                printer_svc.poll_single(printer_id).await;
            }
            _ = shutdown.cancelled() => {
                tracing::info!("printer poll task: shutdown");
                break;
            }
        }
    }
}
```

---

### `crates/trackly-app/src/services/request_service.rs` (app service, CRUD + request-response)

**Analog:** `crates/trackly-app/src/services/cartridge_service.rs`

Same `Arc<WriterHandle>` + `Arc<ReaderPool>` + `clock` + repos pattern. Key additions:

**WS push after mutation pattern** (from RESEARCH.md Code Examples):
```rust
pub async fn transition(
    &self,
    request_id: i64,
    version: i64,
    op: RequestTransitionOp,
    caller: &Identity,
) -> Result<RequestDto, AppError> {
    // enforce transition at service layer (D-Req-Lifecycle-01)
    // ... writer.execute transition ...

    // After successful write: push WS event (D-Notify-01)
    let _ = self.ws_tx.send(WsEvent::RequestStatusChanged {
        request_id,
        new_status: op.target_status().to_string(),
    });
    // Tauri desktop push is handled by the tauri_cmd wrapper (not here)
    self.get(request_id).await
}
```

---

### `crates/trackly-app/src/dto/printer.rs` (app dto, request-response)

**Analog:** `crates/trackly-app/src/dto/cartridge.rs`

**Imports + derives pattern** (analog lines 1-21):
```rust
//! Printer DTOs — shared between Tauri command handlers and axum HTTP handlers.
//!
//! Snake_case JSON (S-2). All `i64` fields carry `#[specta(type = i32)]`.
//! community NEVER appears in PrinterDto (Pitfall 4 from RESEARCH.md).

use serde::{Deserialize, Serialize};
use specta::Type;
use trackly_core::domain::printers::{PrinterRow, PrinterReadingRow, PrinterAlertRow};
```

**DTO struct pattern** (analog lines 22-74):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct PrinterDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub device_id: i64,
    pub ip_address: Option<String>,
    pub snmp_version: String,
    pub vendor: Option<String>,
    #[specta(type = Option<i32>)]
    pub oid_profile_id: Option<i64>,
    #[specta(type = Option<i32>)]
    pub last_seen_utc: Option<i64>,
    // community deliberately absent — never serialize to frontend (Pitfall 4)
    pub community_configured: bool,  // just a bool indicator
    pub device_name: Option<String>,
    pub device_location: Option<String>,
    // Latest reading fields (denormalized for card display):
    pub toner_levels: Option<serde_json::Value>,  // parsed from toner_levels_json
    pub page_count: Option<i64>,
    pub status: Option<String>,
    // Alert indicator:
    pub has_alert: bool,
    pub alert_type: Option<String>,
    #[specta(type = i32)]
    pub version: i64,
}
```

**Input DTO pattern** (analog `CartridgeCreateDto`):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PrinterCreateDto {
    pub device_id: i32,
    pub ip_address: Option<String>,
    /// community_update: Some(s) = set/change, None = keep existing (Pitfall 4)
    pub community_update: Option<String>,
    pub snmp_version: String,
    #[specta(type = Option<i32>)]
    pub oid_profile_id: Option<i64>,
    pub usb_host_device_id: Option<i32>,
}
```

---

### `crates/trackly-app/src/dto/request.rs` (app dto, request-response)

**Analog:** `crates/trackly-app/src/dto/cartridge.rs`

**Tagged enum DTO pattern** (analog `CartridgeTransitionPayload` with `#[serde(tag = "op")]`):
```rust
/// Request transition payload — UI sends { "op": "accept" } | { "op": "reject", ... }
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum RequestTransitionPayload {
    Accept {
        #[specta(type = i32)]
        request_id: i64,
        #[specta(type = i32)]
        version: i64,
        assigned_to_user_id: Option<i32>,
    },
    Reject {
        #[specta(type = i32)]
        request_id: i64,
        #[specta(type = i32)]
        version: i64,
        notes: Option<String>,
    },
    Complete {
        #[specta(type = i32)]
        request_id: i64,
        #[specta(type = i32)]
        version: i64,
        notes: Option<String>,
        /// If Some, links a cartridge installation (REQ-05 D-Req-CART07-01)
        linked_cartridge_id: Option<i32>,
    },
}
```

---

### `crates/trackly-app/src/tauri_cmds/printers.rs` (app tauri_cmd, request-response)

**Analog:** `crates/trackly-app/src/tauri_cmds/cartridges.rs`

**build_* helper + thin wrapper pattern** (analog lines 1-50):
```rust
//! Printers Tauri commands — Phase 6.
//!
//! Pattern (S-1): `build_*` helper + thin `#[tauri::command] #[specta::specta]`
//! wrapper. Both transports (Tauri invoke + axum POST) delegate to the same helper.
//!
//! `#[specta::specta]` MUST appear AFTER `#[tauri::command]`.

use crate::context::AppCtx;
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;

pub async fn build_printers_list(ctx: &AppCtx, filter: PrinterFilter, ...) -> Result<PrinterListResponse, AppError> {
    ctx.printers.list(filter, pagination).await
}

pub async fn build_printers_create(ctx: &AppCtx, caller: &Identity, payload: PrinterCreateDto) -> Result<PrinterDto, AppError> {
    authorize(caller, &Action::MutatePrinters)?;
    ctx.printers.create_from_device(payload, caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn printers_list(state: tauri::State<'_, AppCtx>, filter: PrinterFilter, ...) -> Result<PrinterListResponse, AppError> {
    build_printers_list(state.inner(), filter, pagination).await
}

#[tauri::command]
#[specta::specta]
pub async fn printers_create(state: tauri::State<'_, AppCtx>, payload: PrinterCreateDto) -> Result<PrinterDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_printers_create(state.inner(), &caller, payload).await
}

// Discovery command — returns list for review, does NOT write to DB yet
#[tauri::command]
#[specta::specta]
pub async fn printers_discover(
    state: tauri::State<'_, AppCtx>,
    ip_start: String,
    ip_end: String,
) -> Result<Vec<DiscoveredPrinterDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_printers_discover(state.inner(), &caller, ip_start, ip_end).await
}

// On-demand poll trigger (D-Poll-01)
#[tauri::command]
#[specta::specta]
pub async fn printers_refresh(state: tauri::State<'_, AppCtx>, id: i32) -> Result<PrinterDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_printers_refresh(state.inner(), &caller, id as i64).await
}

// Desktop Tauri event push for WS events (D-Notify-01)
// app: tauri::AppHandle is the extra arg for desktop event emission
#[tauri::command]
#[specta::specta]
pub async fn requests_transition(
    state: tauri::State<'_, AppCtx>,
    app: tauri::AppHandle,
    payload: RequestTransitionPayload,
) -> Result<RequestDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    let result = build_requests_transition(state.inner(), &caller, payload).await?;
    // Desktop push (no WS server needed on desktop)
    app.emit("trackly-event", &WsEvent::RequestStatusChanged {
        request_id: result.id,
        new_status: result.status.clone(),
    }).ok();
    Ok(result)
}
```

---

### `crates/trackly-app/src/http/printers.rs` and `http/requests.rs` (app http, request-response)

**Analog:** `crates/trackly-app/src/http/cartridges.rs`

**Handler pattern** (analog lines 121-135):
```rust
pub async fn handler_list(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListPayload>,
) -> Result<Json<PrinterListResponse>, AppErrorResponse> {
    let _identity = session_identity(&session).await.map_err(AppErrorResponse::from)?;
    Ok(Json(build_printers_list(&ctx, p.filter, p.pagination).await.map_err(AppErrorResponse::from)?))
}

pub async fn handler_create(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<CreatePayload>,
) -> Result<Json<PrinterDto>, AppErrorResponse> {
    let identity = session_identity(&session).await.map_err(AppErrorResponse::from)?;
    // After mutation: push WS event from HTTP transport
    let result = build_printers_create(&ctx, &identity, p.payload).await.map_err(AppErrorResponse::from)?;
    ctx.ws_broadcast.send(WsEvent::PrinterAlert { ... }).ok();  // if applicable
    Ok(Json(result))
}
```

**Router pattern** (analog lines 405-453):
```rust
pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/printers_list", post(handler_list))
        .route("/api/v1/printers_get", post(handler_get))
        .route("/api/v1/printers_create", post(handler_create))
        .route("/api/v1/printers_discover", post(handler_discover))
        .route("/api/v1/printers_refresh", post(handler_refresh))
        // ...
}
```

---

### `crates/trackly-app/src/http/ws.rs` (app http WS, pub-sub)

**No direct analog** in codebase. Use RESEARCH.md WS pattern directly.

**Full WS handler pattern** (from RESEARCH.md WebSocket Pattern section):
```rust
// crates/trackly-app/src/http/ws.rs
use axum::{extract::{State, WebSocketUpgrade}, response::IntoResponse};
use axum::extract::ws::{WebSocket, Message};
use tower_sessions::Session;
use tokio::sync::broadcast;
use crate::context::AppCtx;
use crate::http::auth::session_identity;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    session: Session,        // auth ДО on_upgrade — Pitfall 6
    State(ctx): State<AppCtx>,
) -> impl IntoResponse {
    let identity = match session_identity(&session).await {
        Ok(id) => id,
        Err(_) => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
    };
    let rx = ctx.ws_broadcast.subscribe();
    ws.on_upgrade(move |socket| handle_ws_socket(socket, identity, rx))
}

async fn handle_ws_socket(
    mut socket: WebSocket,
    identity: Identity,
    mut rx: broadcast::Receiver<WsEvent>,
) {
    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(evt) if evt.is_visible_to(&identity) => {
                        let json = serde_json::to_string(&evt).unwrap();
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("WS client lagged {} events", n);
                        // continue — don't break on lag (Pitfall 5)
                    }
                    _ => {}
                }
            }
            msg = socket.recv() => {
                match msg {
                    None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                    _ => {}
                }
            }
        }
    }
}

pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/ws", axum::routing::get(ws_handler))
}
```

**WsEvent struct** (add to `crates/trackly-app/src/dto/` or `http/ws.rs`):
```rust
#[derive(Clone, Serialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    NewRequest {
        #[specta(type = i32)] request_id: i64,
        request_type: String,
        requester_name: String,
    },
    RequestStatusChanged {
        #[specta(type = i32)] request_id: i64,
        new_status: String,
    },
    PrinterAlert {
        #[specta(type = i32)] printer_id: i64,
        printer_name: String,
        alert_type: String,
    },
}

impl WsEvent {
    pub fn is_visible_to(&self, identity: &Identity) -> bool {
        match self {
            WsEvent::PrinterAlert { .. } => {
                identity.role == Role::Admin
            }
            _ => {
                identity.role == Role::Admin || identity.role == Role::Specialist
            }
        }
    }
}
```

---

### `crates/trackly-app/src/context.rs` (modified — extend AppCtx)

**Analog:** self (lines 38-84 — existing service fields)

**Extension pattern** (add to `AppCtx` struct, analog lines 59-84):
```rust
// Add to AppCtx struct:
/// Printer service — SNMP polling, discovery, alert detection.
/// Added in Phase 6 Plan XX.
pub printers: Arc<PrinterService>,
/// Request service — lifecycle, CART-07 link, WS push.
/// Added in Phase 6 Plan XX.
pub requests: Arc<RequestService>,
/// WebSocket broadcast sender — fan-out to all connected WS clients.
/// Capacity 128. Added in Phase 6 Plan XX.
pub ws_broadcast: Arc<tokio::sync::broadcast::Sender<WsEvent>>,
```

**Build method extension pattern** (analog lines 164-206 — stepwise service construction):
```rust
// Add in AppCtx::build after existing services:
let (ws_tx, _) = tokio::sync::broadcast::channel::<WsEvent>(128);
let ws_broadcast = Arc::new(ws_tx);

let (poll_tx, poll_rx) = tokio::sync::mpsc::channel::<i64>(64);
let snmp_client: Arc<dyn SnmpClient + Send + Sync> =
    if config.snmp.use_mock || std::env::var("TRACKLY_SNMP_MOCK").is_ok() {
        Arc::new(MockSnmpClient::default_fixtures())
    } else {
        Arc::new(RealSnmpClient)
    };

let printers = Arc::new(PrinterService::new(
    writer.clone(), readers.clone(), clock.clone(),
    snmp_client, poll_tx, ws_broadcast.clone(),
));
let requests = Arc::new(RequestService::new(
    writer.clone(), readers.clone(), clock.clone(),
    ws_broadcast.clone(),
));

// Spawn poll task with child cancellation token (D-Arch-01, D-Poll-01)
let poll_token = shutdown.child_token();
let printers_clone = printers.clone();
tokio::spawn(async move {
    run_poll_task(printers_clone, poll_rx, poll_token).await;
});
```

---

### `ui/src/features/printers/PrintersPage.svelte` (ui feature page, request-response)

**Analog:** `ui/src/features/cartridges/CartridgesPage.svelte`

**Page structure pattern** (analog lines 1-170, Svelte 5 runes):
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import PrintersMasterDetail from './PrintersMasterDetail.svelte';
  import PrintersList from './PrintersList.svelte';
  import PrinterDetail from './PrinterDetail.svelte';
  import PrinterAlertBanner from './PrinterAlertBanner.svelte';
  import DiscoveryModal from './DiscoveryModal.svelte';
  import { printers } from './api';
  import type { PrinterDto, PrinterFilter, Pagination } from '../../bindings';

  let items = $state<PrinterDto[]>([]);
  let total = $state(0);
  let listLoading = $state(false);
  let selectedId = $state<number | null>(null);
  let selectedPrinter = $state<PrinterDto | null>(null);
  let detailLoading = $state(false);
  let discoveryOpen = $state(false);

  const pagination = $state<Pagination>({ offset: 0, limit: 50 });
  const activeFilter = $derived<PrinterFilter>({ ... });

  // $effect for filter → refresh (analog line 133)
  $effect(() => {
    void activeFilter;
    refresh();
  });

  // $effect for selected item → load detail (analog lines 140-165)
  $effect(() => {
    const id = selectedId;
    if (id === null) { selectedPrinter = null; return; }
    detailLoading = true;
    printers.get(id)
      .then(dto => { selectedPrinter = dto; })
      .catch(() => pushToast('error', 'Не удалось загрузить принтер'))
      .finally(() => { detailLoading = false; });
  });

  onMount(() => loadAll());
</script>
```

**Page header CSS pattern** (analog lines 486-533):
```svelte
<style lang="scss">
  .printers-page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-lg) var(--space-xl);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
    gap: var(--space-md);
    flex-wrap: wrap;
  }
  .page-content {
    flex: 1;
    overflow: auto;
    padding: var(--space-lg) var(--space-xl);
  }
</style>
```

---

### `ui/src/features/printers/PrintersMasterDetail.svelte` (ui component, layout)

**Analog:** `ui/src/features/cartridges/CartridgesMasterDetail.svelte`

**Exact copy** of the master-detail grid (lines 1-53) — same 35%/65% grid, same SCSS, same Svelte 5 snippet props. Only rename class from `.master-detail` if needed.

---

### `ui/src/features/printers/PrinterAlertBanner.svelte` (ui component, alert indicator)

**Analog:** `ui/src/features/cartridges/LowStockBanner.svelte`

**Pattern** (analog lines 1-95 — exact same structure): `role="alert"`, `aria-live="polite"`, warning SVG icon, warning color tokens, conditional render `{#if alerts.length > 0}`.

Adapt: instead of `LowStockItemDto[]`, accept `PrinterAlertRow[]` or derived alert info from `PrinterDto.has_alert`.

---

### `ui/src/features/printers/DiscoveryModal.svelte` (ui component modal, request-response)

**Analog:** `ui/src/features/cartridges/OperationModal.svelte`

**Modal wrapper + form state pattern** (analog lines 1-72):
```svelte
<script lang="ts">
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { printers } from './api';
  import type { DiscoveredPrinterDto } from '../../bindings';

  interface Props {
    open: boolean;
    onClose: () => void;
    onSuccess: (created: number) => void;
  }
  const { open, onClose, onSuccess }: Props = $props();

  let ipStart = $state('');
  let ipEnd = $state('');
  let community = $state('public');
  let scanning = $state(false);
  let discovered = $state<DiscoveredPrinterDto[]>([]);
  let selected = $state<Set<number>>(new Set());

  // Reset on open (pattern from OperationModal lines 55-72)
  $effect(() => {
    if (open) {
      ipStart = ''; ipEnd = ''; community = 'public';
      scanning = false; discovered = []; selected = new Set();
    }
  });
```

**Submit pattern** (analog lines 85-120 — try/catch + pushToast + onSuccess):
```svelte
  async function handleScan() {
    scanning = true;
    try {
      discovered = await printers.discover(ipStart, ipEnd, community);
    } catch (e: unknown) {
      pushToast('error', extractMessage(e, 'Ошибка сканирования'));
    } finally {
      scanning = false;
    }
  }

  async function handleCreate() {
    // ... create selected printers + call onSuccess
  }
```

---

### `ui/src/features/requests/RequestFormModal.svelte` (ui component modal, request-response)

**Analog:** `ui/src/features/cartridges/OperationModal.svelte`

Same Modal + form pattern. Key difference: two form modes (cartridge_replace / free_form) selected by radio/select at top. `cartridge_replace` shows printer dropdown; `free_form` shows category dropdown + textarea. Pattern for conditional fields: use `{#if requestType === 'cartridge_replace'}` block.

---

### `ui/src/features/printers/api.ts` and `ui/src/features/requests/api.ts` (ui api wrappers)

**Analog:** `ui/src/features/cartridges/api.ts`

**Exact same pattern** (analog lines 1-71):
```typescript
// ui/src/features/printers/api.ts
import { apiCall } from '$lib/api/client';
import type { PrinterDto, PrinterFilter, PrinterListResponse, PrinterCreateDto, DiscoveredPrinterDto, Pagination } from '../../bindings';

export const printers = {
  list: (filter: PrinterFilter, pagination: Pagination) =>
    apiCall<PrinterListResponse>('printers_list', { filter, pagination }),

  get: (id: number) => apiCall<PrinterDto>('printers_get', { id }),

  create: (payload: PrinterCreateDto) => apiCall<PrinterDto>('printers_create', { payload }),

  delete: (id: number, version: number) => apiCall<null>('printers_delete', { id, version }),

  discover: (ipStart: string, ipEnd: string, community: string) =>
    apiCall<DiscoveredPrinterDto[]>('printers_discover', { ipStart, ipEnd, community }),

  // On-demand refresh (D-Poll-01)
  refresh: (id: number) => apiCall<PrinterDto>('printers_refresh', { id }),
};
```

---

### `ui/src/lib/api/ws.ts` (ui lib WS client, pub-sub)

**Partial analog:** `ui/src/lib/api/client.ts` (dual-transport detection pattern)

**Full WS client pattern** (from RESEARCH.md Frontend WS reconnect pattern + Tauri events):
```typescript
// ui/src/lib/api/ws.ts
import { isTauri } from '$lib/stores/transport.svelte';
import type { WsEvent } from '../../bindings';

type WsEventHandler = (event: WsEvent) => void;

let handlers: WsEventHandler[] = [];
let ws: WebSocket | null = null;
let reconnectDelay = 1000;

export function onWsEvent(handler: WsEventHandler): () => void {
  handlers.push(handler);
  return () => { handlers = handlers.filter(h => h !== handler); };
}

function dispatch(event: WsEvent) {
  handlers.forEach(h => h(event));
}

export async function connectWs() {
  if (isTauri) {
    // Tauri path: listen for native events (D-Notify-01)
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<WsEvent>('trackly-event', (e) => dispatch(e.payload));
    return unlisten; // caller should call on destroy
  }

  // Browser path: WebSocket with reconnect
  function connect() {
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    ws = new WebSocket(`${protocol}//${location.host}/api/v1/ws`);

    ws.onmessage = (e) => {
      try { dispatch(JSON.parse(e.data)); } catch {}
    };

    ws.onclose = () => {
      ws = null;
      // Exponential backoff: 1s, 2s, 4s, ..., max 30s
      setTimeout(() => { connect(); reconnectDelay = Math.min(reconnectDelay * 2, 30000); }, reconnectDelay);
    };

    ws.onopen = () => { reconnectDelay = 1000; }; // reset on successful connect
  }
  connect();
}
```

---

### `migrations/V020__printers.sql` (migration, CRUD schema)

**Analog:** `migrations/V005__cartridges.sql`

**Migration pattern** (analog lines 1-47):
```sql
-- V020: Printers — SNMP metadata table extending devices.
--
-- `printers` extends `devices` (type_id=2) with SNMP fields.
-- community stored as plain text — Secret<T> wrapping happens in Rust service layer.
-- USB учёт (PRN-04): usb_host_device_id links printer to its host workstation.

CREATE TABLE printers (
  id                    INTEGER PRIMARY KEY AUTOINCREMENT,
  device_id             INTEGER NOT NULL UNIQUE REFERENCES devices(id),
  ip_address            TEXT    NULL,                        -- NULL for USB-only printers
  community             TEXT    NOT NULL DEFAULT 'public',   -- SNMP community string
  snmp_version          TEXT    NOT NULL DEFAULT 'v2c' CHECK (snmp_version IN ('v1', 'v2c', 'v3')),
  vendor                TEXT    NULL,                        -- detected at discovery
  oid_profile_id        INTEGER NULL REFERENCES oid_profiles(id),
  last_seen_utc         INTEGER NULL,
  usb_host_device_id    INTEGER NULL REFERENCES devices(id), -- PRN-04 USB учёт
  created_at_utc        INTEGER NOT NULL,
  updated_at_utc        INTEGER NOT NULL,
  version               INTEGER NOT NULL DEFAULT 1,
  CHECK (ip_address IS NOT NULL OR usb_host_device_id IS NOT NULL)  -- must have at least one
);

PRAGMA user_version = 20;
```

---

### `migrations/V021__oid_profiles_seed.sql` (migration seed, lookup data)

**Analog:** `migrations/V001__init_pragmas_and_lookups.sql` (lookup table + INSERT seed pattern)

```sql
-- V021: OID profiles table + seed data for 4 vendors + RFC3805 fallback (D-OID-01).

CREATE TABLE oid_profiles (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  name              TEXT    NOT NULL UNIQUE,
  vendor_prefix     TEXT    NOT NULL,        -- sysObjectID prefix for matching
  toner_level_oid   TEXT    NULL,
  toner_max_oid     TEXT    NULL,
  toner_encoding    TEXT    NOT NULL DEFAULT 'level_over_max'
                            CHECK (toner_encoding IN ('percent', 'level_over_max')),
  page_counter_oid  TEXT    NULL,
  status_oid        TEXT    NOT NULL,
  serial_oid        TEXT    NULL,
  notes             TEXT    NULL
);

INSERT INTO oid_profiles (name, vendor_prefix, toner_level_oid, toner_max_oid,
    toner_encoding, page_counter_oid, status_oid, notes) VALUES
('pantum',   '1.3.6.1.4.1.40093',  '1.3.6.1.4.1.40093.6.3.1',  NULL,
    'percent',        '1.3.6.1.4.1.40093.10.3.1.1', '1.3.6.1.2.1.25.3.5.1.1.1',
    'Pantum BM5100ADN и аналоги'),
('kyocera',  '1.3.6.1.4.1.1347',   '1.3.6.1.2.1.43.11.1.1.9.1.1', '1.3.6.1.2.1.43.11.1.1.8.1.1',
    'level_over_max', '1.3.6.1.2.1.43.10.2.1.4.1.1', '1.3.6.1.2.1.25.3.5.1.1.1',
    'Kyocera ECOSYS'),
('hp',       '1.3.6.1.4.1.11',     '1.3.6.1.2.1.43.11.1.1.9.1.1', '1.3.6.1.2.1.43.11.1.1.8.1.1',
    'level_over_max', '1.3.6.1.2.1.43.10.2.1.4.1.1', '1.3.6.1.2.1.25.3.5.1.1.1',
    'HP LaserJet'),
('canon',    '1.3.6.1.4.1.1602',   '1.3.6.1.2.1.43.11.1.1.9.1.1', '1.3.6.1.2.1.43.11.1.1.8.1.1',
    'level_over_max', '1.3.6.1.2.1.43.10.2.1.4.1.1', '1.3.6.1.2.1.25.3.5.1.1.1',
    'Canon iR/imageRUNNER'),
('rfc3805',  '',                    '1.3.6.1.2.1.43.11.1.1.9.1.1', '1.3.6.1.2.1.43.11.1.1.8.1.1',
    'level_over_max', '1.3.6.1.2.1.43.10.2.1.4.1.1', '1.3.6.1.2.1.25.3.5.1.1.1',
    'RFC 3805 fallback — any printer');

PRAGMA user_version = 21;
```

---

### `migrations/V022__printer_readings.sql` (migration, time-series)

**Analog:** `migrations/V005__cartridges.sql` (standard table structure)

```sql
-- V022: printer_readings — one row per poll snapshot (D-History-01).
-- toner_levels stored as JSON: {"black":{"level":45,"max":100,"pct":45},"drum":...}
-- Retention/downsample managed by fon background task (D-Retention-01).

CREATE TABLE printer_readings (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  printer_id      INTEGER NOT NULL REFERENCES printers(id) ON DELETE CASCADE,
  ts_utc          INTEGER NOT NULL,
  toner_levels    TEXT    NULL,          -- JSON
  page_count      INTEGER NULL,
  status          TEXT    NOT NULL DEFAULT 'unknown'
                          CHECK (status IN ('ok', 'warning', 'error', 'offline', 'unknown'))
);

CREATE INDEX idx_printer_readings_printer_ts
  ON printer_readings(printer_id, ts_utc DESC);

PRAGMA user_version = 22;
```

---

### `migrations/V023__printer_alerts.sql` (migration, alert storage)

**Analog:** `migrations/V005__cartridges.sql`

```sql
-- V023: printer_alerts — one active alert per printer (D-Alert-01).
-- UNIQUE on printer_id enforces dedup (one alert per printer at a time).

CREATE TABLE printer_alerts (
  id                    INTEGER PRIMARY KEY AUTOINCREMENT,
  printer_id            INTEGER NOT NULL UNIQUE REFERENCES printers(id) ON DELETE CASCADE,
  alert_type            TEXT    NOT NULL CHECK (alert_type IN ('offline', 'error')),
  first_seen_utc        INTEGER NOT NULL,
  last_seen_utc         INTEGER NOT NULL,
  acknowledged_at_utc   INTEGER NULL
);

PRAGMA user_version = 23;
```

---

### `migrations/V024__request_categories.sql` (migration, lookup seed)

**Analog:** `migrations/V001__init_pragmas_and_lookups.sql`

```sql
-- V024: Request categories for free_form requests (D-Req-Categories-01).
-- Lookup table (not CHECK enum) — easier to extend in Phase 7.
-- Also: add category_id FK to requests, and printer_device_id FK for CART-07 link.

CREATE TABLE request_categories (
  id    INTEGER PRIMARY KEY AUTOINCREMENT,
  name  TEXT NOT NULL UNIQUE
);

INSERT INTO request_categories (name) VALUES
  ('Ремонт техники'),
  ('Расходные материалы'),
  ('Программное обеспечение'),
  ('Прочее');

-- Add category_id to requests (nullable — only for free_form type)
ALTER TABLE requests ADD COLUMN category_id INTEGER NULL REFERENCES request_categories(id);

-- PRN-07: Add FK linking cartridge installation to the printer device
-- (was deferred from Phase 4 D-Op-Modal-01)
ALTER TABLE requests ADD COLUMN completed_cartridge_id INTEGER NULL REFERENCES cartridges(id);

PRAGMA user_version = 24;
```

---

## Shared Patterns

### Authentication — session_identity() check
**Source:** `crates/trackly-app/src/http/cartridges.rs` lines 126-128
**Apply to:** all `http/printers.rs` and `http/requests.rs` handlers
```rust
let identity = session_identity(&session).await.map_err(AppErrorResponse::from)?;
// For read-only handlers: let _identity = ... (identity not needed but session required)
```

### RBAC — authorize() at service or build_* layer
**Source:** `crates/trackly-app/src/tauri_cmds/cartridges.rs` lines 41-47
**Apply to:** all mutation build_* helpers in `tauri_cmds/printers.rs`, `tauri_cmds/requests.rs`
```rust
authorize(caller, &Action::MutatePrinters)?;
// or:
authorize(caller, &Action::MutateRequests)?;
```
Note: new `Action` variants `MutatePrinters` and `MutateRequests` must be added to `trackly_core::auth`.

### Error handling — AppErrorResponse
**Source:** `crates/trackly-app/src/http/cartridges.rs` line 131
**Apply to:** every axum handler return type
```rust
) -> Result<Json<T>, AppErrorResponse> {
    Ok(Json(build_fn(...).await.map_err(AppErrorResponse::from)?))
}
```

### Writer execute + transaction commit
**Source:** `crates/trackly-app/src/services/cartridge_service.rs` lines 111-165
**Apply to:** all write paths in `printer_service.rs`, `request_service.rs`
```rust
self.writer.execute(move |conn| {
    let tx = conn.transaction().map_err(map_rusqlite)?;
    // ... write operations ...
    tx.commit().map_err(map_rusqlite)?;
    Ok(result)
}).await?
```

### Audit log insert (history)
**Source:** `crates/trackly-app/src/services/cartridge_service.rs` lines 150-163
**Apply to:** all mutations in `printer_service.rs`, `request_service.rs`
```rust
audit_repo.insert(&tx, AuditEntry {
    entity_type: "printer",  // or "request"
    entity_id: id,
    action: "create",        // or lifecycle op name
    user_id: Some(caller.user_id),
    before_json: None,
    after_json: None,
    payload_json: Some(payload_json),
    created_at_utc: now,
})?;
```

### Reader spawn_blocking pattern
**Source:** `crates/trackly-app/src/services/cartridge_service.rs` lines 325-337
**Apply to:** all read methods in `printer_service.rs`, `request_service.rs`
```rust
tokio::task::spawn_blocking(move || {
    let conn = readers.acquire();
    let row = repo.get(&conn, id)?;
    Ok(Dto::from(row))
})
.await
.map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking: {e}") })?
```

### Secret<T> for SNMP community
**Source:** `crates/trackly-core/src/primitives/secret.rs`
**Apply to:** `PrinterService` wherever community is read from DB and passed to `SnmpClient`
```rust
// Community read from DB as plain String, wrapped before use:
let community = Secret::new(community_from_db);
// Expose only at the SNMP call site:
sess = AsyncSession::new_v2c(target, community.expose().as_bytes(), 0).await?;
// NEVER pass Secret<T> into a DTO or serialize it — Pitfall 4
```

### CancellationToken child for background tasks
**Source:** `crates/trackly-app/src/server/mod.rs` lines 10-15 + `crates/trackly-app/src/shutdown.rs`
**Apply to:** poll task spawned in `AppCtx::build`
```rust
let poll_token = ctx.shutdown.child_token(); // child — never cancels master shutdown
tokio::spawn(run_poll_task(printers.clone(), poll_rx, poll_token));
```

### Svelte 5 runes pattern (Page component)
**Source:** `ui/src/features/cartridges/CartridgesPage.svelte` lines 33-170
**Apply to:** `PrintersPage.svelte`, `RequestsPage.svelte`
```svelte
let items = $state<Dto[]>([]);
let selectedId = $state<number | null>(null);
const activeFilter = $derived<Filter>({ ... });

$effect(() => { void activeFilter; refresh(); });
$effect(() => {
  const id = selectedId;
  if (id === null) { selected = null; return; }
  api.get(id).then(dto => selected = dto).catch(...);
});
onMount(() => loadAll());
```

### apiCall dual-transport
**Source:** `ui/src/lib/api/client.ts` lines 1-42
**Apply to:** `ui/src/features/printers/api.ts`, `ui/src/features/requests/api.ts`
```typescript
import { apiCall } from '$lib/api/client';
// All calls follow same pattern — name maps to Tauri command AND axum POST route
export const printers = {
  list: (filter: PrinterFilter, pagination: Pagination) =>
    apiCall<PrinterListResponse>('printers_list', { filter, pagination }),
};
```

---

## No Analog Found

All files have analogs. The following are **new capabilities** with no exact codebase match — planner should use RESEARCH.md patterns directly:

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/trackly-infra/src/snmp/real.rs` | infra adapter | event-driven I/O | First SNMP implementation; no network I/O adapters exist yet |
| `crates/trackly-infra/src/snmp/mock.rs` | infra adapter test | event-driven | First fixture-based mock adapter |
| `crates/trackly-app/src/http/ws.rs` | app http | pub-sub | First WebSocket handler; axum WS is new to the project |
| `ui/src/lib/api/ws.ts` | ui lib | pub-sub | First real-time push client; no WebSocket or event listener in UI yet |

For these four files, use patterns from `06-RESEARCH.md` sections: "snmp2 API Reference", "WebSocket Pattern", and "Frontend WS reconnect pattern".

---

## Metadata

**Analog search scope:** `crates/trackly-{core,infra,app}/src/**`, `ui/src/**`, `migrations/`
**Files scanned:** 42 Rust source files + 19 Svelte/TS files + 19 migration SQL files
**Pattern extraction date:** 2026-06-14
