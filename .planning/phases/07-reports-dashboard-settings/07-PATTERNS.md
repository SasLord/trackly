# Phase 7: Отчёты, Дашборд и Настройки — Pattern Map

**Mapped:** 2026-06-15
**Files analyzed:** 29 new/modified files
**Analogs found:** 27 / 29

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/trackly-app/src/services/report_service.rs` | service | CRUD (read-heavy, period filter) | `crates/trackly-infra/src/repos/cartridges_sqlite.rs` (list/search/low_stock) | role-match |
| `crates/trackly-app/src/services/dashboard_service.rs` | service | CRUD (aggregate counts) | `crates/trackly-infra/src/repos/cartridges_sqlite.rs` (low_stock + counts) | role-match |
| `crates/trackly-app/src/services/backup_service.rs` | service | file-I/O | `crates/trackly-app/src/services/organization_service.rs` (spawn_blocking pattern) | partial |
| `crates/trackly-app/src/services/org_db_service.rs` | service | CRUD | `crates/trackly-app/src/services/organization_service.rs` + `template_service.rs` | exact (replaces) |
| `crates/trackly-app/src/services/supervisor.rs` | service | event-driven (tokio bg task) | `crates/trackly-app/src/context.rs` (shutdown token pattern) | partial |
| `crates/trackly-app/src/services/template_service.rs` | service | CRUD | `crates/trackly-app/src/services/template_service.rs` (extend existing) | exact |
| `crates/trackly-app/src/http/reports.rs` | middleware/handler | request-response | `crates/trackly-app/src/http/devices.rs` | exact |
| `crates/trackly-app/src/http/dashboard.rs` | middleware/handler | request-response | `crates/trackly-app/src/http/devices.rs` | exact |
| `crates/trackly-app/src/http/settings_org.rs` | middleware/handler | request-response | `crates/trackly-app/src/http/settings.rs` | exact |
| `crates/trackly-app/src/tauri_cmds/reports.rs` | controller | request-response | `crates/trackly-app/src/tauri_cmds/devices.rs` | exact |
| `crates/trackly-app/src/tauri_cmds/dashboard.rs` | controller | request-response | `crates/trackly-app/src/tauri_cmds/devices.rs` | exact |
| `crates/trackly-app/src/tauri_cmds/settings_org.rs` | controller | request-response | `crates/trackly-app/src/tauri_cmds/fs_helpers.rs` + `settings.rs` | role-match |
| `crates/trackly-app/src/dto/reports.rs` | model | transform | `crates/trackly-app/src/dto/device.rs` | role-match |
| `crates/trackly-app/src/pdf/docspec.rs` | model | transform | self (extend `HeaderBlock`) | exact |
| `migrations/V026__org_settings.sql` | migration | CRUD | `migrations/V016__cartridges_kind_color_settings.sql` (app_settings seed) | role-match |
| `ui/src/pages/ReportsPage.svelte` | component | request-response | `ui/src/features/acts/ActsPage.svelte` | role-match |
| `ui/src/pages/Dashboard.svelte` | component | request-response | `ui/src/features/devices/DevicesPage.svelte` | role-match |
| `ui/src/pages/SettingsPage.svelte` | component | request-response | self (extend existing) | exact |
| `ui/src/features/reports/ReportSubNav.svelte` | component | event-driven | `ui/src/features/acts/ActsSearchAndTabs.svelte` | exact |
| `ui/src/features/reports/PeriodSelector.svelte` | component | event-driven | `ui/src/features/cartridges/CartridgeFilters.svelte` (button group pattern) | role-match |
| `ui/src/features/reports/ReportTable.svelte` | component | transform | `ui/src/features/acts/ActsList.svelte` + `ActItemsTable.svelte` | role-match |
| `ui/src/features/reports/ReportFilters.svelte` | component | event-driven | `ui/src/features/cartridges/CartridgeFilters.svelte` | exact |
| `ui/src/features/dashboard/DashboardPage.svelte` | component | request-response | `ui/src/features/requests/RequestsPage.svelte` | role-match |
| `ui/src/features/dashboard/StatWidget.svelte` | component | transform | `ui/src/features/cartridges/LowStockBanner.svelte` (card + warning pattern) | partial |
| `ui/src/features/dashboard/ChartWidget.svelte` | component | transform | `ui/src/features/acts/PdfPreviewModal.svelte` (SVG/iframe loading state) | partial |
| `ui/src/features/dashboard/PeriodToggle.svelte` | component | event-driven | `ui/src/features/cartridges/CartridgeFilters.svelte` (status-bar tabs) | role-match |
| `ui/src/features/settings/OrgSettings.svelte` | component | request-response | `ui/src/features/settings/NetworkSettings.svelte` | exact |
| `ui/src/features/settings/StorageSettings.svelte` | component | request-response | `ui/src/features/settings/NetworkSettings.svelte` | exact |
| `ui/src/features/settings/BackupSettings.svelte` | component | request-response | `ui/src/features/settings/NetworkSettings.svelte` | exact |
| `ui/src/features/settings/ThresholdSettings.svelte` | component | request-response | `ui/src/features/settings/NetworkSettings.svelte` | role-match |
| `ui/src/features/settings/TemplateEditor.svelte` | component | request-response | `ui/src/features/acts/PdfPreviewModal.svelte` (PDF preview + template body) | role-match |
| `crates/trackly-app/tests/report_acts.rs` | test | CRUD | `crates/trackly-app/tests/cartridges_low_stock.rs` | exact |
| `crates/trackly-app/tests/backup_service.rs` | test | file-I/O | `crates/trackly-app/tests/devices_csv_export.rs` | role-match |

---

## Pattern Assignments

### `crates/trackly-app/src/services/report_service.rs` (service, CRUD read-heavy)

**Analog:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs` (list/search/low_stock, lines 530–667)
**Also reference:** `crates/trackly-infra/src/repos/acts_sqlite.rs` (filter pattern, lines 1–44, 222–320)

**Imports pattern** (cartridges_sqlite.rs lines 10–23):
```rust
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use trackly_core::domain::cartridges::{CartridgeCounts, CartridgeFilter, CartridgeRow, Pagination};
use trackly_core::error::AppError;
use crate::error_conversions::map_rusqlite;
```

**Core read-with-filter pattern** (cartridges_sqlite.rs lines 585–665):
```rust
pub fn search(
    &self,
    conn: &Connection,
    query: &str,
    filter: &CartridgeFilter,
) -> Result<Vec<CartridgeRow>, AppError> {
    let like_query = format!("%{}%", query);
    // Build SQL dynamically (no user value concatenated — only structural changes)
    let sql = format!(
        "{SELECT_CARTRIDGES} \
         WHERE c.deleted_at_utc IS NULL \
           AND (?2 IS NULL OR c.status_id = ?2) \
           AND (?3 IS NULL OR m.kind_id = ?3) \
           AND (?4 IS NULL OR c.model_id = ?4) \
         ORDER BY c.created_at_utc DESC, c.id DESC \
         LIMIT 200"
    );
    let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
    let rows = stmt
        .query_map(
            params![like_query, filter.status_id, filter.kind_id, filter.model_id],
            map_row,
        )
        .map_err(map_rusqlite)?;
    let mut out = Vec::new();
    for row in rows { out.push(row.map_err(map_rusqlite)?); }
    Ok(out)
}
```

**Period filter addition** (RESEARCH Pattern 2, new in Phase 7):
```rust
// Extend ActFilter / CartridgeFilter with:
//   date_from_utc: Option<i64>
//   date_to_utc:   Option<i64>
//
// In SQL: AND a.handover_date_utc >= ?N AND a.handover_date_utc <= ?M
// Use params_from_iter for dynamic param lists (RESEARCH Pitfall 6).
```

**Low-stock / app_settings read pattern** (cartridges_sqlite.rs lines 678–688):
```rust
let threshold: i64 = conn
    .query_row(
        "SELECT value FROM app_settings WHERE key = 'low_stock_threshold'",
        [],
        |r| r.get::<_, String>(0),
    )
    .ok()
    .and_then(|s| s.trim().parse::<i64>().ok())
    .filter(|&t| t > 0)
    .unwrap_or(2);
```

**audit_log cartridge history pattern** (cartridges_sqlite.rs lines 727–768):
```rust
let mut stmt = conn
    .prepare(
        "SELECT id, entity_type, entity_id, action, user_id, \
                before_json, after_json, payload_json, created_at_utc \
           FROM audit_log \
          WHERE entity_type = 'cartridge' \
            AND entity_id = ?1 \
            AND action NOT IN ('list', 'get') \
          ORDER BY created_at_utc DESC, id DESC",
    )
    .map_err(map_rusqlite)?;
// For RPT-02 consumption query, filter: action = 'custom:install'
// (VERIFIED: CartridgeTransitionOp::Install => 'custom:install' in cartridges.rs:176)
```

**spawn_blocking read pattern** (template_service.rs lines 94–121):
```rust
pub async fn get_active(&self, kind: &str) -> Result<String, AppError> {
    let readers = self.readers.clone();
    let kind_owned = kind.to_string();
    tokio::task::spawn_blocking(move || -> Result<String, AppError> {
        let conn = readers.acquire();
        // ... SQL query ...
    })
    .await
    .map_err(|e| AppError::Internal {
        source_chain: format!("spawn_blocking get_active: {e}"),
    })?
}
```

---

### `crates/trackly-app/src/services/dashboard_service.rs` (service, CRUD aggregate)

**Analog:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs` (low_stock + counts pattern)
**Also reference:** `crates/trackly-infra/src/repos/devices_sqlite.rs` (count_by_status)

**Core aggregate pattern** (cartridges_sqlite.rs lines 690–724):
```rust
let sql = "SELECT m.id, m.brand, m.model, COUNT(c.id) AS cnt \
           FROM cartridge_models m \
           LEFT JOIN cartridges c ON c.model_id = m.id \
             AND c.status_id = 1 AND c.state_id = 1 \
             AND c.deleted_at_utc IS NULL \
           WHERE m.deleted_at_utc IS NULL \
           GROUP BY m.id \
           HAVING cnt < ?1 \
           ORDER BY cnt ASC, m.brand ASC, m.model ASC";
let mut stmt = conn.prepare(sql).map_err(map_rusqlite)?;
let rows = stmt
    .query_map(params![threshold], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, ...))
    })
    .map_err(map_rusqlite)?;
```

**Consumption SQL (DASH-03 / RPT-02)** (RESEARCH Pattern 3):
```sql
SELECT
    m.brand || ' ' || m.model AS model_label,
    strftime('%Y-%m', datetime(al.created_at_utc, 'unixepoch', '+3 hours')) AS month_key,
    COUNT(*) AS installs
FROM audit_log al
JOIN cartridges c ON c.id = al.entity_id
JOIN cartridge_models m ON m.id = c.model_id
WHERE al.entity_type = 'cartridge'
  AND al.action = 'custom:install'   -- VERIFIED: CartridgeTransitionOp::Install => 'custom:install'
  AND al.created_at_utc >= ?1
GROUP BY model_label, month_key
ORDER BY month_key ASC, model_label ASC
```

---

### `crates/trackly-app/src/services/backup_service.rs` (service, file-I/O)

**Analog:** `crates/trackly-app/src/services/organization_service.rs` (spawn_blocking + path validation)
**Also reference:** `crates/trackly-app/src/tauri_cmds/fs_helpers.rs` (reject_unc, lines 126–133)

**spawn_blocking file operation pattern** (organization_service.rs lines 66–95):
```rust
pub async fn read(&self) -> Result<OrgData, AppError> {
    let path = self.file_path();
    tokio::task::spawn_blocking(move || -> Result<OrgData, AppError> {
        if !path.exists() { /* ... */ }
        // sync file operation here
    })
    .await
    .map_err(|e| AppError::Internal {
        source_chain: format!("spawn_blocking read org.json: {e}"),
    })?
}
```

**UNC rejection pattern** (fs_helpers.rs lines 126–133):
```rust
fn reject_unc(path: &str) -> Result<(), AppError> {
    if path.starts_with("\\\\") || path.starts_with("//") {
        return Err(AppError::Validation {
            field: "path".to_string(),
            message: "UNC-пути не поддерживаются".to_string(),
        });
    }
    Ok(())
}
```

**rusqlite::backup::Backup pattern** (RESEARCH Pattern 4 — ASSUMED API):
```rust
// Must run against a reader conn (not writer) in WAL mode.
// Wrap in spawn_blocking — rusqlite::backup is synchronous.
use rusqlite::backup::Backup;
fn backup_db(src_conn: &Connection, dest_path: &Path) -> rusqlite::Result<()> {
    let mut dest = Connection::open(dest_path)?;
    let backup = Backup::new(src_conn, &mut dest)?;
    backup.run_to_completion(500, std::time::Duration::from_millis(250), None)
}
// Post-backup integrity_check:
let check: String = dest.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
if check != "ok" { return Err(...); }
```

**Retention cleanup pattern** (RESEARCH Pattern 4):
```rust
// List files matching trackly-backup-*.db in backup_folder,
// sort by mtime ascending, delete oldest until count <= retention.
```

**writer.execute for write path** (settings.rs lines 127–145):
```rust
ctx.writer
    .execute(move |conn| {
        let upsert_sql = "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
                          VALUES (?1, ?2, ?3, ?3) \
                          ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at_utc = ?3";
        conn.execute(upsert_sql, rusqlite::params!["key_name", value, now])
            .map(|_| ())
            .map_err(map_rusqlite)
    })
    .await
```

---

### `crates/trackly-app/src/services/org_db_service.rs` (service, CRUD)

**Analog:** `crates/trackly-app/src/services/organization_service.rs` (existing, to be replaced)
**Also reference:** `crates/trackly-app/src/services/template_service.rs` (writer.execute + seed pattern)

**Current OrgData struct** (organization_service.rs lines 23–45) — to be replaced with DB-backed version:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrgData {
    pub name: String,
    pub inn: String,
    pub kpp: String,
    pub address: String,
    pub logo_path: String,  // replaced by logo_blob: Option<Vec<u8>>, logo_mime: Option<String>
}
```

**writer.execute seed/upsert pattern** (template_service.rs lines 60–90):
```rust
pub async fn seed_defaults_on_startup(&self) -> Result<(), AppError> {
    let now = self.clock.unix_seconds();
    self.writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            // check if row exists, INSERT if not
            tx.commit().map_err(map_rusqlite)?;
            Ok(())
        })
        .await
}
```

**One-time org.json migration** (new startup hook, not SQL migration):
```rust
// During AppCtx::build (or first OrgDbService::read call):
// if org.json exists AND org_settings row has placeholder values:
//   read org.json → copy fields to DB → rename org.json → org.json.migrated
//   tracing::info!("Migrated org data from org.json to org_settings");
```

**Logo BLOB size validation** (RESEARCH Pitfall 3 — new guard):
```rust
if logo_bytes.len() > 512 * 1024 {
    return Err(AppError::Validation {
        field: "logo".to_string(),
        message: "Логотип слишком большой. Максимальный размер: 512 КБ".to_string(),
    });
}
```

---

### `crates/trackly-app/src/services/supervisor.rs` (service, event-driven)

**Analog:** No exact analog — first background task in the project.
**Closest reference:** `crates/trackly-app/src/context.rs` shutdown token pattern (lines 57, 214–228)

**tokio background loop pattern** (RESEARCH Pattern 7 — ASSUMED):
```rust
pub async fn run_supervisor(ctx: AppCtx) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let now = ctx.clock.unix_seconds();
                // Read scheduled_tasks WHERE next_run_at_utc <= now AND status != 'running'
                // Atomic claim: UPDATE...WHERE status != 'running'
                // If rows_affected == 0 → skip (already claimed)
                // Dispatch job via spawn_blocking, update status
            }
            _ = ctx.shutdown.cancelled() => break,
        }
    }
}
```

**Atomic task claim guard** (RESEARCH Pitfall 4):
```sql
UPDATE scheduled_tasks SET status='running', last_run_at_utc=?1
WHERE name=?2 AND status != 'running'
-- Check rows_affected == 1 before running job
```

---

### `crates/trackly-app/src/services/template_service.rs` (service, CRUD — extend existing)

**Analog:** `crates/trackly-app/src/services/template_service.rs` (existing, self-analog — extend)

**Existing `get_active` pattern to reuse** (lines 92–121):
```rust
pub async fn get_active(&self, kind: &str) -> Result<String, AppError> {
    let readers = self.readers.clone();
    let kind_owned = kind.to_string();
    tokio::task::spawn_blocking(move || -> Result<String, AppError> {
        let conn = readers.acquire();
        let body: Option<String> = conn.query_row(
            "SELECT body_minijinja FROM document_templates \
             WHERE kind = ?1 AND is_active = 1 AND deleted_at_utc IS NULL \
             ORDER BY updated_at_utc DESC, id DESC LIMIT 1",
            params![kind_owned],
            |r| r.get(0),
        ).map(Some).or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(map_rusqlite(other)),
        })?;
        body.ok_or(AppError::NotFound { entity: "document_template", id: 0 })
    })
    .await
    .map_err(|e| AppError::Internal {
        source_chain: format!("spawn_blocking get_active: {e}"),
    })?
}
```

**New methods to add** (writer.execute pattern from existing seed_defaults_on_startup):
```rust
// update_body: writer.execute → UPDATE document_templates SET body_minijinja=?1, updated_at_utc=?2
// reset_to_default: writer.execute → UPDATE + re-insert default body from DEFAULT_TEMPLATES const
// list_all_for_editor: spawn_blocking → SELECT id, kind, name, body_minijinja WHERE is_active=1
```

---

### `crates/trackly-app/src/http/reports.rs` (handler, request-response)

**Analog:** `crates/trackly-app/src/http/devices.rs` (lines 1–310) — exact pattern

**Imports pattern** (devices.rs lines 11–31):
```rust
use axum::{extract::State, routing::post, Json, Router};
use tower_sessions::Session;
use crate::context::AppCtx;
use crate::http::auth::session_identity;
use crate::error_axum::AppErrorResponse;
use crate::tauri_cmds::reports::{
    build_reports_device_acts,
    build_reports_cartridge_consumption,
    build_reports_export_csv,
    build_reports_export_pdf,
    // ...
};
```

**Payload struct pattern** (devices.rs lines 36–98):
```rust
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportListPayload {
    pub filter: ReportFilter,
    pub period: PeriodDto,
}
```

**Handler pattern** (devices.rs lines 138–165):
```rust
pub async fn handler_device_acts_report(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<ReportListPayload>,
) -> Result<Json<ReportResponse>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_reports_device_acts(&ctx, payload.filter, payload.period)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}
```

**PDF bytes response pattern** (templates.rs lines 35–47):
```rust
pub async fn handler_export_pdf(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ReportListPayload>,
) -> Result<impl IntoResponse, AppErrorResponse> {
    let bytes = build_reports_export_pdf(&ctx, &session, p.filter, p.period)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/pdf")],
        bytes,
    ))
}
```

**Router pattern** (settings.rs lines 329–336):
```rust
pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/reports_device_acts", post(handler_device_acts_report))
        .route("/api/v1/reports_export_pdf", post(handler_export_pdf))
        .route("/api/v1/reports_export_csv", post(handler_export_csv))
        // ...
}
```

---

### `crates/trackly-app/src/http/settings_org.rs` (handler, request-response)

**Analog:** `crates/trackly-app/src/http/settings.rs` (entire file, lines 1–337) — exact pattern

**Auth + ManageSettings guard pattern** (settings.rs lines 106–116):
```rust
pub async fn build_settings_set_network(
    ctx: &AppCtx,
    session: &Session,
    patch: NetworkPatch,
) -> Result<(), AppError> {
    let caller = session_identity(session).await?;
    trackly_core::auth::authorize(&caller, &Action::ManageSettings)?;
    // validate + writer.execute
}
```

**app_settings upsert pattern** (settings.rs lines 127–145):
```rust
ctx.writer
    .execute(move |conn| {
        let upsert_sql = "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
                          VALUES (?1, ?2, ?3, ?3) \
                          ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at_utc = ?3";
        conn.execute(upsert_sql, rusqlite::params!["low_stock_threshold", value, now])
            .map(|_| ())
            .map_err(map_rusqlite)
    })
    .await
```

---

### `crates/trackly-app/src/tauri_cmds/reports.rs` + `dashboard.rs` + `settings_org.rs` (controller, request-response)

**Analog:** `crates/trackly-app/src/tauri_cmds/devices.rs` (lines 1–250) — exact pattern

**build_* helper pattern** (devices.rs lines 27–70):
```rust
// build_* helper (shared by both transports)
pub async fn build_reports_device_acts(
    ctx: &AppCtx,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    ctx.reports.list_device_acts(filter, period).await
}

// Mutation requires caller
pub async fn build_settings_save_org(
    ctx: &AppCtx,
    caller: &Identity,
    patch: OrgPatch,
) -> Result<(), AppError> {
    authorize(caller, &Action::ManageSettings)?;
    ctx.org.save(patch).await
}
```

**Tauri command wrapper pattern** (devices.rs lines 138–185):
```rust
#[tauri::command]
#[specta::specta]
pub async fn reports_device_acts(
    state: tauri::State<'_, AppCtx>,
    filter: ReportFilter,
    period: PeriodDto,
) -> Result<ReportResponse, AppError> {
    build_reports_device_acts(state.inner(), filter, period).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_save_org(
    state: tauri::State<'_, AppCtx>,
    patch: OrgPatch,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_settings_save_org(state.inner(), &caller, patch).await
}
```

---

### `crates/trackly-app/src/dto/reports.rs` (model, transform)

**Analog:** `crates/trackly-app/src/dto/device.rs`

**DTO struct pattern** (device.rs — serde + specta types):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ReportRow {
    pub id: i64,
    pub month_key: Option<String>,   // "2026-06" for temporal reports, None for snapshot
    pub number: Option<String>,       // act number (RPT-01)
    pub location: Option<String>,
    pub status: Option<String>,
    // ... columns vary by report type
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ReportFilter {
    pub date_from_utc: Option<i64>,  // Unix epoch seconds
    pub date_to_utc: Option<i64>,
    pub location_id: Option<i64>,
    pub status_id: Option<i64>,
    pub act_type: Option<String>,    // "handover" | "return"
    pub model_id: Option<i64>,       // cartridge reports
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DashboardWidgetDto {
    pub devices_total: i64,
    pub devices_by_status: Vec<StatusCount>,
    pub cartridge_by_status: Vec<StatusCount>,  // flat — matches 07-01 DTO
    pub low_stock_count: i64,
    pub low_stock_models: Vec<String>,
    pub request_counts_open: i64,
    pub request_counts_in_progress: i64,
    pub request_counts_completed: i64,
    pub printer_online: i64,
    pub printer_offline: i64,
    pub printer_problematic: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConsumptionPoint {
    pub month_key: String,   // "2026-06"
    pub model_label: String, // "HP CF226A"
    pub installs: i64,
}
```

---

### `crates/trackly-app/src/pdf/docspec.rs` (model, transform — extend)

**Analog:** `crates/trackly-app/src/pdf/docspec.rs` (self — existing HeaderBlock, lines 28–39)

**Existing HeaderBlock** (lines 28–39):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeaderBlock {
    pub org_name: String,
    pub org_inn: String,
    pub org_kpp: String,
    pub org_address: String,
    pub logo_path: Option<String>,
    pub act_label: String,
    pub date_label: String,
}
```

**Extension to add (RESEARCH Pattern 6, Pitfall 7)**:
```rust
// New fields with #[serde(default)] to keep backward compat with existing templates:
#[serde(default)]
pub logo_bytes: Option<Vec<u8>>,  // takes priority over logo_path
#[serde(default)]
pub logo_mime: Option<String>,    // "image/png" | "image/jpeg" | "image/svg+xml"
```

**For universal report PDF** — build DocSpec programmatically (NOT via MiniJinja template):
```rust
// The Section::ItemsTable variant already handles multi-column tables:
Section::ItemsTable {
    columns: vec!["Номер акта".into(), "Дата".into(), "Получатель".into(), "Локация".into()],
    rows: report_rows.iter().map(|r| vec![...]).collect(),
}
// Month separator → Section::Heading { level: 3, text: "Сентябрь 2026".into() }
```

---

### `migrations/V026__org_settings.sql` (migration)

**Analog:** `migrations/V016__cartridges_kind_color_settings.sql` (app_settings seed pattern)

**Migration pattern** (RESEARCH Pattern 5):
```sql
CREATE TABLE org_settings (
  id              INTEGER PRIMARY KEY CHECK (id = 1),
  org_name        TEXT    NOT NULL DEFAULT 'Ваша организация',
  inn             TEXT    NOT NULL DEFAULT '0000000000',
  kpp             TEXT    NOT NULL DEFAULT '000000000',
  address         TEXT    NOT NULL DEFAULT 'Адрес не указан',
  logo_blob       BLOB    NULL,
  logo_mime       TEXT    NULL,
  created_at_utc  INTEGER NOT NULL,
  updated_at_utc  INTEGER NOT NULL,
  version         INTEGER NOT NULL DEFAULT 1
);
INSERT INTO org_settings (id, org_name, inn, kpp, address, created_at_utc, updated_at_utc)
  VALUES (1, 'Ваша организация', '0000000000', '000000000', 'Адрес не указан',
          unixepoch(), unixepoch());
PRAGMA user_version = 26;
```

---

### `ui/src/features/reports/ReportSubNav.svelte` (component, event-driven)

**Analog:** `ui/src/features/acts/ActsSearchAndTabs.svelte` (lines 1–132) — exact pattern

**Two-level nav structure** (ActsSearchAndTabs.svelte lines 1–72):
```svelte
<script lang="ts">
  import Badge from '$lib/components/Badge.svelte';
  type DomainKey = 'devices' | 'cartridges';
  type ReportKey = 'acts' | 'returns' | 'in_use' | 'in_stock';   // or cartridge variants

  interface Props {
    activeDomain: DomainKey;
    activeReport: ReportKey;
    rowCount: number;
    onDomainChange: (_d: DomainKey) => void;
    onReportChange: (_r: ReportKey) => void;
  }
  const { activeDomain, activeReport, rowCount, onDomainChange, onReportChange } = $props();
</script>

<nav class="domain-nav" aria-label="Домен отчётов">
  {#each DOMAINS as d}
    <button class="tab" class:active={d.key === activeDomain} onclick={() => onDomainChange(d.key)}>
      {d.label}
    </button>
  {/each}
</nav>
<nav class="report-nav" role="tablist" aria-label="Тип отчёта">
  {#each activeReports as r}
    <button class="tab" class:active={r.key === activeReport} onclick={() => onReportChange(r.key)}>
      {r.label}
      <Badge variant={r.key === activeReport ? 'accent' : 'default'} size="sm">{rowCount}</Badge>
    </button>
  {/each}
</nav>
```

**Tab SCSS** (ActsSearchAndTabs.svelte lines 91–131):
```scss
.tab {
  display: inline-flex;
  align-items: center;
  gap: var(--space-xs);
  padding: var(--space-xs) var(--space-md);
  background: transparent;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-body);
  font-weight: var(--font-weight-medium);
  cursor: pointer;
  height: 32px;

  &:hover { background: var(--color-surface-sunken); }
  &:focus-visible { outline: none; box-shadow: 0 0 0 3px var(--color-accent-focus); }
  &.active {
    background: color-mix(in srgb, var(--color-accent) 10%, transparent);
    border-color: var(--color-accent);
  }
}
```

---

### `ui/src/features/reports/ReportFilters.svelte` (component, event-driven)

**Analog:** `ui/src/features/cartridges/CartridgeFilters.svelte` (lines 1–213) — exact pattern

**Filter select pattern** (CartridgeFilters.svelte lines 51–108):
```svelte
<script lang="ts">
  // Contextual by report type — Устройства: location/type/status; Картриджи: model/status/color
  interface Props {
    reportType: ReportKey;
    locationId: number | null;
    statusId: number | null;
    // ...
    onLocationChange: (_l: number | null) => void;
  }
</script>

<div class="extra-filters">
  <label class="filter-label">
    <span class="filter-name">Локация</span>
    <select class="filter-select" value={locationId ?? ''} onchange={...}>
      <option value="">Все</option>
      {#each locations as l (l.id)}
        <option value={l.id}>{l.name}</option>
      {/each}
    </select>
  </label>
</div>
```

**Filter SCSS** (CartridgeFilters.svelte lines 183–213):
```scss
.filter-select {
  height: 28px;
  padding: 0 var(--space-sm);
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-label);
  &:focus-visible {
    outline: none;
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px var(--color-accent-focus);
  }
}
```

---

### `ui/src/features/reports/PeriodSelector.svelte` (component, event-driven)

**Analog:** `ui/src/features/cartridges/CartridgeFilters.svelte` (status-bar button group pattern, lines 119–155)

**Button group SCSS** (CartridgeFilters.svelte status-bar):
```scss
.status-bar { display: flex; gap: 2px; overflow-x: auto; }
.status-tab {
  padding: var(--space-xs) var(--space-sm);
  border: none;
  border-bottom: 2px solid transparent;
  background: transparent;
  &.active { color: var(--color-accent); border-bottom-color: var(--color-accent); }
}
```

**Period mode switch implementation**:
```svelte
<script lang="ts">
  type PeriodMode = 'month' | 'year' | 'range';
  let mode = $state<PeriodMode>('month');
  // Default: current month (D-03)
  let selectedMonth = $state(new Date().getMonth() + 1);
  let selectedYear = $state(new Date().getFullYear());
  const MONTHS = ['январь','февраль','март','апрель','май','июнь',
                  'июль','август','сентябрь','октябрь','ноябрь','декабрь'];
</script>

<div class="period-selector" role="group" aria-label="Выбор периода">
  {#each (['month','year','range'] as const) as m}
    <button class="period-btn" class:active={mode === m} onclick={() => mode = m}>
      {m === 'month' ? 'Месяц' : m === 'year' ? 'Год' : 'Диапазон'}
    </button>
  {/each}
</div>
{#if mode === 'month'}
  <select bind:value={selectedMonth}>
    {#each MONTHS as name, i}
      <option value={i + 1}>{name}</option>
    {/each}
  </select>
  <select bind:value={selectedYear}> ... </select>
{/if}
```

---

### `ui/src/features/reports/ReportTable.svelte` (component, transform)

**Analog:** `ui/src/features/acts/ActsList.svelte` (loading/empty/error state pattern, lines 34–69)

**Loading + empty state pattern** (ActsList.svelte lines 34–69):
```svelte
<script lang="ts">
  let { items, loading, error } = $props();
</script>

{#if loading}
  <div class="state state-loading"><Spinner size="md" /></div>
{:else if error}
  <div class="state state-error">Не удалось загрузить отчёт. Попробуйте ещё раз.</div>
{:else if items.length === 0}
  <div class="state state-empty">
    <p class="empty-heading">Нет данных за выбранный период</p>
    <p class="empty-body">Измените диапазон дат или выберите другой тип отчёта.</p>
  </div>
{:else}
  <table>
    <thead><tr>{#each columns as col}<th scope="col">{col}</th>{/each}</tr></thead>
    <tbody>
      {#each grouped as item}
        {#if item.type === 'separator'}
          <!-- Month separator row (D-02, RPT-06) -->
          <tr class="month-separator" aria-hidden="true">
            <td colspan={columns.length}>{item.label}</td>
          </tr>
        {:else}
          <tr>...</tr>
        {/if}
      {/each}
    </tbody>
  </table>
{/if}
```

**Month separator SCSS** (UI-SPEC §Reports Layout):
```scss
.month-separator td {
  padding: var(--space-xs) var(--space-md);
  height: var(--row-height-dense);   // 32px
  background: var(--color-surface-sunken);
  font-size: var(--font-size-body);
  font-weight: var(--font-weight-semibold);
  border-top: 1px solid var(--color-border-strong);
}
```

---

### `ui/src/features/settings/OrgSettings.svelte` (component, request-response)

**Analog:** `ui/src/features/settings/NetworkSettings.svelte` (entire file, lines 1–457) — exact pattern

**Section card + form pattern** (NetworkSettings.svelte lines 137–230, 255–300):
```svelte
<section class="settings-section">
  <h2 class="section-title">Организация</h2>
  <!-- form fields here -->
  <div class="save-row">
    <Button variant="primary" loading={saving} onclick={saveOrg}>
      Сохранить настройки организации
    </Button>
  </div>
</section>

<style lang="scss">
  .settings-section {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-lg);
  }
  .section-title {
    margin: 0 0 var(--space-md);
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
  }
</style>
```

**apiCall + toast pattern** (NetworkSettings.svelte lines 38–81):
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';

  let saving = $state(false);

  async function saveOrg() {
    saving = true;
    try {
      await apiCall<void>('settings_save_org', { patch: { ... } });
      pushToast('success', 'Настройки сохранены');
    } catch (e: unknown) {
      const msg = e && typeof e === 'object' && 'message' in e
        ? String((e as { message: unknown }).message)
        : 'Не удалось сохранить настройки организации';
      pushToast('error', msg);
    } finally {
      saving = false;
    }
  }

  onMount(() => { /* load */ });
</script>
```

---

### `ui/src/features/settings/BackupSettings.svelte` (component, request-response)

**Analog:** `ui/src/features/settings/NetworkSettings.svelte` — same section card + loading state

**Loading spinner + success flash pattern** (NetworkSettings.svelte toggling pattern, lines 83–104):
```svelte
let backingUp = $state(false);
let lastBackupTime = $state<string | null>(null);

async function runManualBackup() {
  backingUp = true;
  try {
    const result = await apiCall<{ timestamp: number }>('backup_run_manual', {});
    lastBackupTime = new Date(result.timestamp * 1000).toLocaleString('ru-RU');
    pushToast('success', 'Резервная копия создана');
  } catch (e: unknown) {
    const msg = ...;
    pushToast('error', `Резервная копия не создана: ${msg}. Проверьте путь к папке.`);
  } finally {
    backingUp = false;
  }
}
```

**Tauri folder picker pattern** (PdfPreviewModal.svelte lines 172–193):
```svelte
async function pickBackupFolder() {
  const { open } = await import('@tauri-apps/plugin-dialog');
  const path = await open({ directory: true, multiple: false });
  if (path) {
    await apiCall<void>('settings_save_backup_config', { backup_folder: path });
    backupFolder = path as string;
  }
}
```

---

### `ui/src/features/settings/TemplateEditor.svelte` (component, request-response)

**Analog:** `ui/src/features/acts/PdfPreviewModal.svelte` (PDF preview loading state, lines 56–169)

**Template load + preview pattern** (PdfPreviewModal.svelte lines 120–169):
```svelte
<script lang="ts">
  import Textarea from '$lib/components/Textarea.svelte';
  import Button from '$lib/components/Button.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';

  let templateBody = $state('');
  let validating = $state(false);
  let saving = $state(false);
  let blobUrl = $state<string | null>(null);

  async function validateAndPreview() {
    validating = true;
    try {
      const bytes = await apiCall<number[]>('templates_validate_preview', { body: templateBody });
      // create blob URL from bytes, open in iframe (same pattern as PdfPreviewModal)
      const blob = new Blob([new Uint8Array(bytes)], { type: 'application/pdf' });
      blobUrl = URL.createObjectURL(blob);
    } catch (e: unknown) {
      pushToast('error', `Шаблон содержит ошибки: ${...}`);
    } finally {
      validating = false;
    }
  }
</script>

<div class="template-editor">
  <details class="variables-panel">
    <summary>Доступные переменные</summary>
    <!-- list org_name, inn, act_number, etc. -->
  </details>
  <Textarea bind:value={templateBody} style="font-family: monospace; min-height: 320px;" />
  <div class="footer-row">
    <Button variant="secondary" loading={validating} onclick={validateAndPreview}>
      Проверить (превью PDF)
    </Button>
    <Button variant="primary" loading={saving} onclick={saveTemplate}>
      Сохранить шаблон
    </Button>
    <Button variant="destructive" onclick={() => confirmReset = true}>
      Сбросить до умолчания
    </Button>
  </div>
</div>
```

---

### `ui/src/features/dashboard/StatWidget.svelte` (component, transform)

**Analog:** `ui/src/features/cartridges/LowStockBanner.svelte` (warning banner pattern + `ui/src/features/settings/NetworkSettings.svelte` card pattern)

**Widget card structure** (UI-SPEC §Dashboard Layout + NetworkSettings.svelte section card):
```svelte
<section class="stat-widget" aria-labelledby="widget-title-{id}">
  <h2 class="widget-title" id="widget-title-{id}">{title}</h2>
  {#if loading}
    <div class="widget-loading"><Spinner size="sm" /></div>
  {:else if error}
    <div class="widget-error">Ошибка загрузки</div>
  {:else}
    <p class="stat-number">{value}</p>
    <p class="stat-label">{label}</p>
    {#if breakdown.length > 0}
      <ul class="breakdown-list">
        {#each breakdown as row}
          <li>{row.label}: {row.count}</li>
        {/each}
      </ul>
    {/if}
  {/if}
</section>

<style lang="scss">
  .stat-widget {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-lg);
    min-height: 120px;  // UI-SPEC Phase 7 constraint
  }
  .stat-number {
    font-size: var(--font-size-display);   // 28px
    font-weight: var(--font-weight-semibold);
    margin: 0;
  }
  .stat-label {
    font-size: var(--font-size-label);    // 13px
    color: var(--color-text-secondary);
    margin: 0;
  }
</style>
```

---

### `ui/src/features/dashboard/ChartWidget.svelte` (component, transform)

**Analog:** `ui/src/features/acts/PdfPreviewModal.svelte` (loading/error/content state pattern)

**SVG polyline chart** (RESEARCH Pattern 9 — hand-drawn, zero npm deps):
```svelte
<script lang="ts">
  interface Props {
    data: { month: string; model: string; installs: number }[];
    windowMonths: 3 | 6 | 12;
    loading: boolean;
  }
  const { data, windowMonths, loading } = $props();

  function toPoints(series: number[], maxVal: number): string {
    if (series.length < 2) return '';
    return series.map((v, i) => {
      const x = (i / (series.length - 1)) * 380 + 10;
      const y = 190 - (v / (maxVal || 1)) * 170;
      return `${x},${y}`;
    }).join(' ');
  }
</script>

<section class="chart-widget" aria-labelledby="chart-title">
  <h2 class="widget-title" id="chart-title">Динамика расхода картриджей</h2>
  <!-- 3/6/12 switcher -->
  <div class="period-toggle" role="group" aria-label="Период графика">
    {#each ([3, 6, 12] as const) as m}
      <button class="toggle-btn" class:active={windowMonths === m}
              onclick={() => onWindowChange(m)}>{m} мес.</button>
    {/each}
  </div>

  {#if loading}
    <div class="chart-state"><Spinner /></div>
  {:else if data.length === 0}
    <div class="chart-state chart-empty">Нет данных о расходе за выбранный период</div>
  {:else}
    <svg role="img" aria-label="График динамики расхода картриджей за {windowMonths} месяцев"
         viewBox="0 0 400 200" preserveAspectRatio="none" class="chart-svg">
      {#each seriesKeys as key, i}
        <polyline points={toPoints(seriesData[key], maxVal)}
          fill="none" stroke="var(--color-accent)" stroke-width="2" />
      {/each}
    </svg>
    <!-- Visually-hidden data table for accessibility (UI-SPEC §Accessibility) -->
    <table class="sr-only" aria-label="Данные графика">...</table>
  {/if}
</section>
```

---

### `ui/src/pages/SettingsPage.svelte` (component — extend existing)

**Analog:** `ui/src/pages/SettingsPage.svelte` (self — extend, lines 1–39)

**Existing page structure to extend** (SettingsPage.svelte lines 1–39):
```svelte
<script lang="ts">
  import NetworkSettings from '../features/settings/NetworkSettings.svelte';
  // Phase 7: add imports for new sections
  import OrgSettings from '../features/settings/OrgSettings.svelte';
  import StorageSettings from '../features/settings/StorageSettings.svelte';
  import BackupSettings from '../features/settings/BackupSettings.svelte';
  import ThresholdSettings from '../features/settings/ThresholdSettings.svelte';
  import TemplateEditor from '../features/settings/TemplateEditor.svelte';
</script>

<div class="settings-page">
  <header class="page-header">
    <h1 class="page-title">Настройки</h1>
  </header>
  <div class="settings-content">
    <NetworkSettings />
    <!-- Phase 7: add below in order from UI-SPEC §Settings sections order -->
    <OrgSettings />
    <StorageSettings />
    <BackupSettings />
    <ThresholdSettings />
    <TemplateEditor />
  </div>
</div>
```

**Page header + content scroll pattern** (SettingsPage.svelte lines 14–39):
```scss
.settings-page { display: flex; flex-direction: column; height: 100%; }
.page-header {
  padding: var(--space-lg) var(--space-xl);
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
}
.settings-content { flex: 1; overflow: auto; padding: var(--space-lg) var(--space-xl); }
```

---

### Test files (integration tests)

**Analog:** `crates/trackly-app/tests/cartridges_low_stock.rs` (entire file)

**Integration test structure** (cartridges_low_stock.rs lines 1–127):
```rust
use std::sync::Arc;
use std::time::Duration;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;
use trackly_app::services::ReportService;  // or DashboardService, BackupService

fn make_service() -> (ReportService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let svc = ReportService::new(writer, readers, clock);
    (svc, dir)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_acts_filtered_by_period() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        // seed data, call service, assert
    })
    .await
    .expect("test budget")
}
```

---

## Shared Patterns

### Authentication Guard
**Source:** `crates/trackly-app/src/http/settings.rs` lines 106–116
**Apply to:** All new HTTP handlers in `http/reports.rs`, `http/dashboard.rs`, `http/settings_org.rs`

```rust
// Read-only endpoints: session check only
let _identity = session_identity(session).await?;

// Mutation endpoints (settings, backup trigger): also authorize
let caller = session_identity(session).await?;
trackly_core::auth::authorize(&caller, &Action::ManageSettings)?;
```

### app_settings Key-Value Upsert
**Source:** `crates/trackly-app/src/http/settings.rs` lines 127–145
**Apply to:** `settings_org.rs`, `backup_service.rs` (threshold, backup_folder, backup_schedule, backup_retention)

```rust
ctx.writer.execute(move |conn| {
    let upsert_sql = "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
                      VALUES (?1, ?2, ?3, ?3) \
                      ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at_utc = ?3";
    conn.execute(upsert_sql, rusqlite::params!["key_name", value_str, now])
        .map(|_| ())
        .map_err(map_rusqlite)
}).await
```

### Error Handling (AppError variants in use)
**Source:** `crates/trackly-core/src/error.rs` + `crates/trackly-app/src/error_axum.rs`
**Apply to:** All new services, handlers, tauri_cmds

```rust
// Service layer errors:
AppError::NotFound { entity: "org_settings", id: 1 }
AppError::Validation { field: "logo".to_string(), message: "...".to_string() }
AppError::Internal { source_chain: format!("spawn_blocking: {e}") }
// Writer errors auto-propagated by WriterHandle::execute
```

### spawn_blocking Read Pattern
**Source:** `crates/trackly-app/src/services/template_service.rs` lines 94–121
**Apply to:** All read queries in `report_service.rs`, `dashboard_service.rs`, `org_db_service.rs`

```rust
let readers = self.readers.clone();
tokio::task::spawn_blocking(move || -> Result<T, AppError> {
    let conn = readers.acquire();
    // synchronous rusqlite query here
})
.await
.map_err(|e| AppError::Internal {
    source_chain: format!("spawn_blocking <operation>: {e}"),
})?
```

### CSV Export (UTF-8 BOM + semicolon)
**Source:** `crates/trackly-app/src/services/device_service.rs` lines 801–903
**Apply to:** `report_service.rs` (RPT-07 export)

```rust
let mut wtr = csv::WriterBuilder::new()
    .delimiter(b';')
    .from_writer(Vec::new());
wtr.write_record(["Колонка1", "Колонка2"])?;
for row in &rows {
    wtr.write_record(&[csv_safe(&row.col1), csv_safe(&row.col2)])?;
}
let inner = wtr.into_inner()?;
let body = String::from_utf8(inner)?;
let mut output = String::with_capacity(3 + body.len());
output.push('\u{FEFF}');  // UTF-8 BOM
output.push_str(&body);
```

### MiniJinja Validation
**Source:** `crates/trackly-app/src/pdf/minijinja_env.rs` lines 31–85
**Apply to:** `template_service.rs` update_body + validate_preview (SET-09)

```rust
// Use render_with_timeout with a dummy context:
let dummy_ctx = serde_json::json!({
    "org_name": "ООО Тест", "inn": "0000000000",
    "act_number": "42", "handover_date": "15.06.2026",
    "items": [{"name": "Ноутбук", "qty": 1}]
});
let rendered = render_with_timeout(&ctx.pdf.minijinja_env, "preview", body, dummy_ctx).await?;
// Then parse as DocSpec to catch JSON structure errors
let spec: DocSpec = serde_json::from_str(&rendered)
    .map_err(|e| AppError::Validation { field: "template".into(), message: e.to_string() })?;
```

### Svelte apiCall + Toast
**Source:** `ui/src/features/settings/NetworkSettings.svelte` lines 38–81
**Apply to:** All new Svelte settings and reports components

```svelte
<script lang="ts">
  import { apiCall } from '$lib/api/client';
  import { pushToast } from '$lib/stores/toast.svelte';

  async function mutate() {
    loading = true;
    try {
      await apiCall<void>('command_name', { ...payload });
      pushToast('success', 'Операция выполнена');
    } catch (e: unknown) {
      const msg = e && typeof e === 'object' && 'message' in e
        ? String((e as { message: unknown }).message)
        : 'Ошибка операции';
      pushToast('error', msg);
    } finally {
      loading = false;
    }
  }
</script>
```

### Destructive Action Confirmation Modal
**Source:** `ui/src/lib/components/Modal.svelte` (referenced pattern from UI-SPEC)
**Apply to:** `StorageSettings.svelte` (move DB), `TemplateEditor.svelte` (reset template)

```svelte
{#if confirmOpen}
  <Modal open={true} title="Сменить расположение базы данных?" size="md"
         onClose={() => confirmOpen = false}>
    <p>База данных будет скопирована в новое расположение через безопасный API SQLite.
       Приложение потребует перезапуска.</p>
    {#snippet footer()}
      <Button variant="secondary" onclick={() => confirmOpen = false}>Отмена</Button>
      <Button variant="primary" onclick={proceedWithMove} loading={moving}>
        Выбрать новый путь
      </Button>
    {/snippet}
  </Modal>
{/if}
```

---

## No Analog Found

Files with no close match in the codebase (planner should use RESEARCH.md patterns):

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `crates/trackly-app/src/services/supervisor.rs` | service | event-driven (tokio bg loop) | No background supervisor pattern exists in project yet (Phase 7 activates first one) |

---

## Metadata

**Analog search scope:** `crates/trackly-app/src/`, `crates/trackly-infra/src/`, `ui/src/features/`, `ui/src/pages/`, `ui/src/lib/`
**Files scanned:** 47 source files (Rust + Svelte)
**Pattern extraction date:** 2026-06-15
**Critical codebase facts confirmed:**
- `rusqlite::backup` feature confirmed in workspace Cargo.toml (line 32)
- `std::fs::copy` is clippy-banned globally — all file copies MUST use `rusqlite::backup::Backup`
- `chrono::Local::now` is clippy-banned — use `time::UtcOffset::from_hms(3,0,0)` for Moscow TZ
- `dirs::*_dir()` is clippy-banned — all paths via `trackly_infra::Paths`
- Writer pattern: `ctx.writer.execute(move |conn| { ... }).await` — all mutations
- Reader pattern: `tokio::task::spawn_blocking(move || { let conn = readers.acquire(); ... }).await`
- Session auth: `session_identity(&session).await?` for read; + `authorize(&caller, &Action::ManageSettings)?` for settings mutations
- `#[tauri::command] #[specta::specta]` decorator order is fixed (tauri first, then specta)
- All Svelte components: Svelte 5 runes (`$state`, `$derived`, `$props`, `$effect`)
- All HTTP payloads: `#[serde(rename_all = "camelCase")]` on Deserialize structs
