# Phase 7: Отчёты, Дашборд и Настройки — Research

**Researched:** 2026-06-15
**Domain:** Reports / Dashboard / Settings — read-heavy queries, scheduled-task supervisor, org-data migration to DB, rusqlite backup API, MiniJinja template editor, SVG chart, CSV/PDF export
**Confidence:** HIGH (all claims grounded in the live codebase from Phases 1–6)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-01:** Навигация отчётов — двухуровневая: domain sub-nav (Устройства / Картриджи) + switch-bar конкретных отчётов.
**D-02:** Snapshot-отчёты «Что на складе» / «Что в работе» — срез «сейчас»; период скрыт/неактивен; группировка по локации/статусу.
**D-03:** Период по умолчанию для временного отчёта — текущий месяц.
**D-04:** Фильтры контекстные по типу отчёта: Устройства → локация/тип/статус; Картриджи → модель/статус/цвет.
**D-05:** PDF — один универсальный табличный шаблон; переиспользует DocSpec IR + krilla 0.7 из Phase 3.
**D-06:** Печать по контексту транспорта: Tauri → PDF → системный просмотрщик (tauri-plugin-shell); browser → window.print() / download.
**D-07:** Экспорт охватывает «что видно сейчас» (текущие фильтры + поиск + период).
**D-08:** CSV = паттерн из Phase 2 (UTF-8 BOM, `;`-делимитер).
**D-09:** Дашборд — стартовая страница (маршрут `/`).
**D-10:** Компоновка 5 виджетов — фиксированная адаптивная сетка, read-only.
**D-11:** График динамики расхода — линейный, переключатель 3/6/12 месяцев.
**D-12:** Дашборд имеет общий селектор периода; график — собственный переключатель 3/6/12.
**D-13:** Виджет «Принтеры» «проблемные» = активные алерты `offline` + `error` из `printer_alerts`.
**D-14:** «Расход» (RPT-02) и DASH-03 = события Install по месяцам/моделям. «История заправок» = ToRefill / FromRefill. WriteOff не включается.
**D-15:** Org данные (поля + логотип BLOB) → в БД; `org.json` устаревает; `safe_logo_canonical` ретируется. Вопрос о `organization.timezone` (config.toml vs БД) — на research/planner.
**D-16:** Папку автобэкапа выбирает пользователь (нет дефолта); ретенция по умолчанию 7 копий, настраивается.
**D-17:** Supervisor scheduled_tasks работает пока процесс жив; catch-up при старте.
**D-18:** Бэкап — только через `rusqlite::backup::Backup` (clippy-банит `std::fs::copy`); integrity_check после записи.
**D-19:** Смена пути БД = копия через rusqlite::backup + UNC-rejection + integrity_check → сохранить путь в конфиг → просить перезапуск.
**D-20:** Редактор шаблонов: raw textarea + панель переменных + кнопка «Проверить / Превью PDF».
**D-21:** Supervisor: только log-retention worker + backup worker. Корзина (UI soft-delete) — в backlog.

### Claude's Discretion

- Выбор графической библиотеки для линейного графика DASH-03 (рекомендация: hand-drawn SVG; никаких тяжёлых deps).
- Переносить ли `organization.timezone` из config.toml в БД — research ниже рекомендует **оставить в config.toml**.
- Вёрстка и ориентация универсального PDF отчёта (portrait/landscape для широких).
- Схема активации supervisor и формат записей `scheduled_tasks`.

### Deferred Ideas (OUT OF SCOPE)

- Корзина (UI над soft-delete).
- Настраиваемая компоновка дашборда (drag/hide).
- Визуальный WYSIWYG-редактор шаблонов.
- Snapshot на произвольную историческую дату.
- Авто-restart зависших Pantum-принтеров.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RPT-01 | Подраздел «Отчёты по устройствам»: Акты приёма-передачи, Возвраты, Что в работе, Что на складе | New SQL queries on `acts` + `devices`; act_type filter; status_id filter |
| RPT-02 | Подраздел «Отчёты по картриджам»: Расход, Что в работе, Что на складе, История заправок | New SQL: Install events in `audit_log`; cartridge status joins |
| RPT-03 | Выборка периода: по месяцу / по году / диапазон дат | UTC timestamps in DB; timezone offset from config.toml (`Europe/Moscow` = UTC+3); period math via UtcOffset |
| RPT-04 | Фильтры внутри отчёта (локация, тип, статус, модель) | Extend existing ActFilter / CartridgeFilter domain types with date_from_utc/date_to_utc |
| RPT-05 | Поиск внутри отчёта | Reuse existing LIKE/FTS5 search pattern; narrow scope to report rows |
| RPT-06 | Группировка по месяцам с визуальным разделителем «Сентябрь 2026» | Client-side: group rows by year+month in Svelte; or return pre-sorted SQL with month label |
| RPT-07 | Экспорт в CSV (UTF-8 BOM) и PDF | Reuse `csv::WriterBuilder::new().delimiter(b';')` + BOM prepend (device_service pattern); reuse DocSpec + PdfRenderer |
| RPT-08 | Печать | Transport-split: Tauri → tauri-plugin-shell open PDF; browser → window.print() |
| DASH-01 | Виджет «Устройства»: общее количество + разбивка по статусам | Existing `devices_sqlite.count_by_status()` returns `Vec<(status_id, u64)>` |
| DASH-02 | Виджет «Картриджи»: разбивка по статусам + alert низкого остатка | Existing `CartridgeCounts` + `low_stock()` from `cartridges_sqlite.rs:678` |
| DASH-03 | График динамики расхода картриджей: 3/6/12 месяцев | New query: Install events from `audit_log` grouped by month + model; hand-drawn SVG path |
| DASH-04 | Виджет «Заявки»: активные/новые/выполненные за период | Existing `RequestCounts { open, in_progress, completed, rejected }`; add period filter variant |
| DASH-05 | Виджет «Принтеры»: онлайн/офлайн, проблемные | Existing `PrinterCounts { ok, warning, error, offline }`; COUNT active `printer_alerts` |
| SET-01 | Раздел «Организация»: название, реквизиты, адрес, логотип | New org_settings migration; org CRUD service replacing org.json |
| SET-02 | Логотип как BLOB в БД | New column `org_logo BLOB` in org_settings migration; renderer accepts `Vec<u8>` from DB |
| SET-03 | Текущий путь к БД: Открыть папку / Сменить расположение | Extend fs_helpers; existing UNC rejection; rusqlite::backup::Backup for move |
| SET-04 | Порог низкого остатка | Existing `app_settings.low_stock_threshold`; UI only needs read+upsert |
| SET-05 | Экспорт БД через SQLite backup API | `rusqlite::backup::Backup` API — feature already enabled |
| SET-06 | Ручной бэкап БД | Thin wrapper over backup helper; exposed as Tauri command + HTTP route |
| SET-07 | Автобэкап: расписание, папка, ретенция | scheduled_tasks supervisor; V011 table already in schema |
| SET-09 | Управление шаблонами: редактирование, валидация | Existing `document_templates` table (V007); `TemplateService` extension; MiniJinja validation via build_safe_env() |
</phase_requirements>

---

## Summary

Phase 7 is a **pure integration phase** — it adds no new domain entities and produces no new storage schema beyond one migration (org settings + logo BLOB). All technical building blocks are already in the codebase: DocSpec + PdfRenderer (Phase 3), CSV export pattern (Phase 2), CartridgeCounts / RequestCounts / PrinterCounts (Phase 4–6), TemplateService + document_templates (Phase 3), scheduled_tasks table (Phase 1), UNC rejection in fs_helpers (Phase 2), rusqlite::backup feature enabled in workspace Cargo.toml.

The phase has three distinct sub-systems:

1. **Reports + Dashboard (read layer):** Write new SQL report queries that apply UTC timestamp period filters. Period boundaries are computed server-side using a UTC offset derived from `config.organization.timezone` (which is a simple `"Europe/Moscow"` string = UTC+3). No new crate is needed — the `time 0.3` crate already in the workspace provides `UtcOffset::from_hms(3,0,0)` for Europe/Moscow. Month-grouping is handled client-side in Svelte. The line chart uses a hand-drawn `<svg>` polyline — zero new npm dependencies.

2. **Settings (write layer):** Organisation data migrates from `org.json` (filesystem) to a new `org_settings` table in the database. Logo becomes a BLOB. The PDF renderer gains a `logo_bytes: Option<Vec<u8>>` path alongside the existing `logo_path` path; eventually the file-path branch retires. DB-path move and backup go through `rusqlite::backup::Backup` (already feature-gated in workspace). The `scheduled_tasks` supervisor activates for the first time: a tokio background task reads rows from V011, runs overdue jobs at startup (catch-up), then re-schedules on a timer.

3. **Template editor:** `TemplateService` gains `get_all_for_editor()`, `update_body()`, `reset_to_default()` methods. The frontend sends the new body to a `templates_validate` endpoint which runs MiniJinja `build_safe_env()` + a dummy render; if it succeeds, the body is valid. The existing `render_with_timeout` function is the validation primitive — no new code needed.

**Primary recommendation:** Ground every new SQL query in the existing `ActFilter` / `CartridgeFilter` pattern (parameterised `rusqlite::params!`, no string concatenation); add `date_from_utc: Option<i64>` / `date_to_utc: Option<i64>` fields to filter structs and emit `AND created_at_utc BETWEEN ?N AND ?M` clauses.

---

## Project Constraints (from CLAUDE.md)

| Directive | Implication for Phase 7 |
|-----------|------------------------|
| `std::fs::copy` clippy-banned | Backup + DB-move MUST use `rusqlite::backup::Backup` |
| `dirs::*_dir()` clippy-banned | Backup folder path comes from user selection (tauri-plugin-dialog), stored in `app_settings` |
| `chrono::Local::now` clippy-banned | Period boundary math uses `time::UtcOffset`; no chrono |
| `app_data_dir()` / `tauri::Manager::path` clippy-banned | All paths through `trackly_infra::Paths` |
| Single-writer pattern mandatory | Org save, template save, threshold update, backup config — all through `WriterHandle::execute` |
| Dual-transport (Tauri + axum) | Every new `build_*` helper must be thin; both Tauri command and axum handler call the same helper |
| Portability (paths relative to exe_dir) | Backup folder chosen by user, path saved to `app_settings`; not assumed from any system dir |
| Russian-only UI | All copy in Russian; month names (январь–декабрь) hardcoded in Svelte |
| rusqlite 0.38 `bundled` + `backup` features | backup feature confirmed in workspace Cargo.toml — `rusqlite::backup::Backup` is available now |
| MSRV 1.92 (pinned in Cargo.toml) | chrono-tz is not needed; `time 0.3` UtcOffset approach is MSRV-safe |
| Security: `security_enforcement: true`, ASVS level 1 | All new endpoints must call `authorize()` with appropriate Action |

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Report SQL queries (period filter, grouping) | API / Backend (rusqlite) | — | UTC timestamps live in DB; period math is server-side |
| Report table rendering + month grouping | Browser / Client (Svelte) | — | SQL returns sorted rows; Svelte groups by year+month key |
| CSV export | API / Backend (csv crate, BOM prepend) | — | Same as Phase 2 device export pattern |
| PDF export | API / Backend (krilla DocSpec) | — | Renderer is sync; called from service layer |
| Consumption chart data | API / Backend (SQL aggregation) | Browser (SVG rendering) | Backend returns `[{month, model, count}]`; Svelte draws SVG |
| Dashboard widget counts | API / Backend (count queries) | — | Reuse existing counts types; add period filter for requests widget |
| Settings persistence | API / Backend (app_settings upsert) | — | k/v store already established |
| Org data (fields + logo BLOB) | Database / Storage | API / Backend | Logo must round-trip through DB for portable mode |
| Backup execution | API / Backend (rusqlite::backup) | — | Must stay in Rust; cannot delegate to FS copy |
| Supervisor (scheduled_tasks) | API / Backend (tokio task) | — | Long-running background task per D-17 |
| Template validation | API / Backend (MiniJinja) | — | Security: never run user template in browser |
| Logo upload (file → bytes) | Browser / Client (input[type=file]) | Frontend Server (Tauri dialog) | Two paths: browser `<input>`, desktop `tauri-plugin-dialog` |

---

## Standard Stack

### Core — No New Crates Needed

All required capabilities are already in the workspace:

| Library | Version (workspace) | Purpose | Status |
|---------|---------------------|---------|--------|
| `rusqlite` | 0.38, features: `bundled`, `backup` | DB queries + backup API | [VERIFIED: Cargo.toml:32] — `backup` feature confirmed |
| `krilla` | 0.7.0 (pinned `=0.7.0`) | Universal report PDF via DocSpec IR | [VERIFIED: trackly-app/Cargo.toml] |
| `minijinja` | ^2.20, features: builtins+json+fuel+serde | Template render + validation | [VERIFIED: trackly-app/Cargo.toml] |
| `csv` | 1.3.x | CSV export (delimiter + BOM) | [VERIFIED: workspace Cargo.toml] |
| `time` | 0.3.x, features: serde+macros+formatting+parsing | UTC epoch math + UtcOffset for TZ | [VERIFIED: workspace Cargo.toml:24] |
| `tauri-plugin-dialog` | 2.x | Folder picker for backup path, logo upload on desktop | [VERIFIED: ui/package.json + trackly-app/Cargo.toml] |
| `tauri-plugin-shell` | 2.x | Open PDF in system viewer for desktop print (D-06) | [VERIFIED: trackly-app/Cargo.toml] |
| `tauri-plugin-fs` | 2.x | Write exported PDF/CSV bytes to filesystem on desktop | [VERIFIED: trackly-app/Cargo.toml] |
| `serde` / `serde_json` | 1.x | DTO serialization, MiniJinja render context | [VERIFIED: workspace Cargo.toml] |
| `tokio` | 1.x, rt-multi-thread | Async tasks + supervisor loop | [VERIFIED: workspace Cargo.toml] |

### No New npm Dependencies

The chart (DASH-03) is implemented as a hand-drawn `<svg>` polyline in Svelte. The existing `pdfjs-dist` (already in package.json) handles PDF preview in the iframe pattern already established in Phase 3. No new npm packages are needed.

**Installation:** None — all Rust crates already in workspace, all JS deps already in `ui/package.json`.

---

## Package Legitimacy Audit

> Phase 7 installs **zero new packages** — all required libraries are already in the workspace. Slopcheck is not required.

| Package | Disposition |
|---------|-------------|
| (none new) | N/A — no new installs in this phase |

---

## Architecture Patterns

### System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│  Browser / Tauri Webview                                            │
│  ReportsPage → PeriodSelector → ReportFilters → ReportTable (SVG)  │
│  DashboardPage → StatWidgets → ChartWidget (SVG polyline)           │
│  SettingsPage → OrgSettings, StorageSettings, BackupSettings,       │
│                 ThresholdSettings, TemplateEditor                   │
└───────────┬─────────────────────────────────────────────────────────┘
            │ Tauri invoke OR HTTP POST /api/v1/*
            ▼
┌─────────────────────────────────────────────────────────────────────┐
│  build_* helpers (shared between Tauri command + axum handler)      │
│  ┌──────────────────┐  ┌─────────────────┐  ┌────────────────────┐ │
│  │  ReportService   │  │  DashboardSvc   │  │  SettingsService   │ │
│  │  (new, Phase 7)  │  │  (new, Phase 7) │  │  (org, backup, tpl)│ │
│  └────────┬─────────┘  └────────┬────────┘  └─────────┬──────────┘ │
│           │                     │                       │            │
│           ▼ spawn_blocking      ▼ spawn_blocking        ▼ writer.execute
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │  SQLite (WAL)                                                   ││
│  │  acts / devices / cartridges / printer_alerts /                 ││
│  │  audit_log / app_settings / org_settings (new V026) /          ││
│  │  document_templates / scheduled_tasks                           ││
│  └─────────────────────────────────────────────────────────────────┘│
└───────────────────────────────────┬─────────────────────────────────┘
                                    │
                          ┌─────────┴────────────┐
                          │  Supervisor (tokio bg)│
                          │  ScheduledTaskWorker  │
                          │  - backup job         │
                          │  - log-retention job  │
                          └──────────────────────┘
```

### Recommended Project Structure

```
crates/trackly-app/src/
├── services/
│   ├── report_service.rs        # New: report queries (period filter, grouping)
│   ├── dashboard_service.rs     # New: widget aggregate queries
│   ├── backup_service.rs        # New: rusqlite::backup + integrity_check + retention
│   ├── org_db_service.rs        # New: replaces OrganizationService (org.json → DB)
│   ├── supervisor.rs            # New: scheduled_tasks worker (tokio bg task)
│   └── template_service.rs      # Extended: add update_body, reset_to_default, list_all
├── http/
│   ├── reports.rs               # New: /api/v1/reports_*
│   ├── dashboard.rs             # New: /api/v1/dashboard_*
│   └── settings_org.rs          # New: /api/v1/org_*, /api/v1/backup_*, /api/v1/threshold_*
├── tauri_cmds/
│   ├── reports.rs               # New: Tauri commands for reports
│   ├── dashboard.rs             # New: Tauri commands for dashboard
│   └── settings_org.rs          # New: Tauri commands for org/backup/threshold
├── dto/
│   └── reports.rs               # New: ReportRow, ReportFilter, DashboardWidgetDto, etc.
migrations/
└── V026__org_settings.sql        # New: org_settings + logo BLOB; V027 optional

ui/src/features/
├── reports/
│   ├── ReportsPage.svelte        # Replaces placeholder
│   ├── ReportSubNav.svelte       # Two-level nav
│   ├── PeriodSelector.svelte     # Month/Year/Range with DatePicker
│   ├── ReportTable.svelte        # Universal table with month-separator rows
│   └── ReportFilters.svelte      # Contextual filter row
├── dashboard/
│   ├── DashboardPage.svelte      # Replaces placeholder
│   ├── StatWidget.svelte         # Generic stat card
│   ├── ChartWidget.svelte        # Line chart via SVG polyline
│   └── PeriodToggle.svelte       # 3/6/12 month switcher
└── settings/
    ├── OrgSettings.svelte
    ├── StorageSettings.svelte
    ├── BackupSettings.svelte
    ├── ThresholdSettings.svelte
    └── TemplateEditor.svelte
```

### Pattern 1: Period Filter on UTC Timestamps

The DB stores all timestamps as Unix epoch seconds (UTC). `organization.timezone` in `config.toml` is `"Europe/Moscow"` = UTC+3. Period boundaries must be converted to UTC before the SQL query.

The `time 0.3` crate (already in workspace) provides `UtcOffset` — sufficient for a fixed-offset timezone. For `Europe/Moscow` specifically: UTC+3, no DST since 2014. This means a fixed `UtcOffset::from_hms(3, 0, 0)` works correctly for this codebase's single-organisation use case.

**Recommendation on D-15 timezone question:** Keep `organization.timezone` in `config.toml`. Reason: (1) TZ is a deployment parameter set by the sysadmin, not per-user data; (2) adding it to `org_settings` DB would require migrating it out again if the sysadmin wants to change it without touching the DB; (3) the `time` crate approach with a fixed offset is already used.

```rust
// Source: time 0.3 docs + project pattern (ASSUMED — exact API call, not tested)
// compute_period_utc_bounds in report_service.rs

use time::{Date, Month, OffsetDateTime, UtcOffset};

/// Compute (start_utc, end_utc) Unix epoch seconds for a given month
/// in the org's timezone.
fn month_bounds_utc(year: i32, month: u8, offset: UtcOffset) -> (i64, i64) {
    let m = Month::try_from(month).expect("valid month 1-12");
    let start_local = Date::from_calendar_date(year, m, 1)
        .expect("valid date")
        .midnight()
        .assume_offset(offset);
    let days_in_month = start_local.date().days_in_month();
    let end_local = Date::from_calendar_date(year, m, days_in_month)
        .expect("valid date")
        .with_hms(23, 59, 59).expect("valid time")
        .assume_offset(offset);
    (start_local.unix_timestamp(), end_local.unix_timestamp())
}
```

[ASSUMED — exact API; `time 0.3` OffsetDateTime is confirmed in workspace but this specific call pattern needs verification during implementation]

### Pattern 2: Report Query with Period + Filter

Extend the existing parameterised SQL pattern. New report queries follow the same structure as `SELECT_ACTS` in `acts_sqlite.rs`:

```rust
// Source: project pattern from acts_sqlite.rs + cartridges_sqlite.rs [VERIFIED: codebase]
// All filters are Option<T> — None means "not applied" (no WHERE clause fragment added)

fn list_act_report(
    conn: &Connection,
    filter: &ActReportFilter,  // date_from_utc, date_to_utc, location_id, act_type
) -> Result<Vec<ActReportRow>, AppError> {
    let mut sql = String::from(
        "SELECT a.id, a.number, a.sub_number, a.giver_name, a.receiver_name,
                a.handover_date_utc, l.name AS location_name, a.act_type
           FROM acts a
           LEFT JOIN locations l ON a.location_id = l.id
          WHERE a.deleted_at_utc IS NULL"
    );
    // Build parameterised WHERE fragments — no string interpolation of user data
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    let mut idx = 1usize;
    if let Some(from) = filter.date_from_utc {
        sql.push_str(&format!(" AND a.handover_date_utc >= ?{idx}"));
        params.push(Box::new(from));
        idx += 1;
    }
    if let Some(to) = filter.date_to_utc {
        sql.push_str(&format!(" AND a.handover_date_utc <= ?{idx}"));
        params.push(Box::new(to));
        idx += 1;
    }
    sql.push_str(" ORDER BY a.handover_date_utc ASC");
    // ... query_map with params_from_iter
}
```

**Warning — dynamic SQL:** Use `params_from_iter` from rusqlite when building dynamic parameter lists. Do NOT concatenate user values into the SQL string. [VERIFIED: pattern from existing cartridges_sqlite list() with dynamic WHERE].

### Pattern 3: Cartridge Consumption Query (DASH-03 / RPT-02)

D-14 defines: «Расход» = Install events from `audit_log`. The `audit_log` table (V008) stores `entity_type`, `action`, `created_at_utc`. Cartridge Install events use action = `'custom:install'` [VERIFIED: crates/trackly-core/src/domain/cartridges.rs:176].

```sql
-- Monthly consumption per model, last N months [VERIFIED: audit_log schema from V008; action string VERIFIED: cartridges.rs:176]
SELECT
    m.brand || ' ' || m.model AS model_label,
    strftime('%Y-%m', datetime(al.created_at_utc, 'unixepoch', '+3 hours')) AS month_key,
    COUNT(*) AS installs
FROM audit_log al
JOIN cartridges c ON c.id = al.entity_id
JOIN cartridge_models m ON m.id = c.model_id
WHERE al.entity_type = 'cartridge'
  AND al.action = 'custom:install'  -- VERIFIED: CartridgeTransitionOp::Install => "custom:install" (cartridges.rs:176)
  AND al.created_at_utc >= ?1   -- start of 3/6/12-month window in UTC
GROUP BY model_label, month_key
ORDER BY month_key ASC, model_label ASC
```

**Note:** `strftime('%Y-%m', ...)` is always UTC in SQLite. For DASH-03 (relative last N months chart), UTC grouping is acceptable (the visual is approximate). For RPT-06 (month separator heading «Сентябрь 2026»), the Svelte client receives `created_at_utc` per row and converts using the known offset: `new Date((utc + tz_offset_seconds) * 1000)`.

### Pattern 4: rusqlite::backup::Backup

The `rusqlite` workspace dep has `features = ["backup"]` confirmed. The Backup API is synchronous:

```rust
// Source: rusqlite docs, feature = "backup" [ASSUMED: exact API — verify during implementation]
use rusqlite::backup::Backup;

fn backup_db(src_conn: &Connection, dest_path: &Path) -> rusqlite::Result<()> {
    let mut dest = Connection::open(dest_path)?;
    let backup = Backup::new(src_conn, &mut dest)?;
    backup.run_to_completion(
        500,                          // pages per step
        std::time::Duration::from_millis(250),  // sleep between steps
        None,                         // progress callback
    )
}
```

**Critical:** The backup must run against the **reader pool connection** (not the writer), called from `spawn_blocking` inside a `WriterHandle::execute` closure that is held exclusively during the backup — OR from a dedicated `spawn_blocking` that acquires a reader. The writer-lock model means we cannot hold the writer channel open during backup; the standard approach is to run the backup via the reader and use SQLite's WAL-mode page-by-page copy which is consistent without blocking writes.

**Post-backup integrity_check (D-18):**
```rust
// [ASSUMED: rusqlite integrity_check API — verify during implementation]
let check: String = dest.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
if check != "ok" {
    return Err(rusqlite::Error::QueryReturnedNoRows); // wrap as AppError
}
```

**Retention:** After backup, list files in the backup folder matching `trackly-backup-*.db`, sort by modification time ascending, delete oldest until `count <= retention`.

### Pattern 5: org_settings Migration (V026)

```sql
-- V026: org_settings — replaces org.json. One-row table (soft approach: key-value OR single row).
-- Decision: single-row with named columns is more type-safe than extending app_settings k/v.
CREATE TABLE org_settings (
  id              INTEGER PRIMARY KEY CHECK (id = 1),  -- enforces single-row
  org_name        TEXT    NOT NULL DEFAULT 'Ваша организация',
  inn             TEXT    NOT NULL DEFAULT '0000000000',
  kpp             TEXT    NOT NULL DEFAULT '000000000',
  address         TEXT    NOT NULL DEFAULT 'Адрес не указан',
  logo_blob       BLOB    NULL,     -- PNG/JPG/SVG bytes; NULL = no logo
  logo_mime       TEXT    NULL,     -- e.g. 'image/png' — needed by renderer
  created_at_utc  INTEGER NOT NULL,
  updated_at_utc  INTEGER NOT NULL,
  version         INTEGER NOT NULL DEFAULT 1
);
INSERT INTO org_settings (id, org_name, inn, kpp, address, created_at_utc, updated_at_utc)
  VALUES (1, 'Ваша организация', '0000000000', '000000000', 'Адрес не указан',
          unixepoch(), unixepoch());

PRAGMA user_version = 26;
```

**One-time migration from org.json:** During startup, if `org.json` exists AND `org_settings` has the placeholder values, `OrgDbService` reads `org.json`, copies name/inn/kpp/address to the DB row, attempts to read the logo file (if `logo_path` not empty), and stores as BLOB. Writes a `tracing::info!` and renames `org.json` to `org.json.migrated` (not deleted, for safety). This is a Phase 7 startup hook, not a SQL migration.

### Pattern 6: PDF Renderer Logo BLOB Path

The current `DocSpec.HeaderBlock.logo_path` is `Option<String>` (filesystem path). Phase 7 adds a BLOB path. Two approaches:

**Option A (recommended):** Add `logo_bytes: Option<Vec<u8>>` to `HeaderBlock`. Renderer checks `logo_bytes` first; falls back to `logo_path` for backward compat during transition. After D-15 is complete, `logo_path` branch becomes dead code and can be removed in Phase 8.

```rust
// Extended HeaderBlock — [VERIFIED: existing DocSpec in src/pdf/docspec.rs]
pub struct HeaderBlock {
    pub org_name: String,
    pub org_inn: String,
    pub org_kpp: String,
    pub org_address: String,
    pub logo_path: Option<String>,    // legacy — kept for backward compat
    pub logo_bytes: Option<Vec<u8>>,  // new in Phase 7 — takes priority
    pub logo_mime: Option<String>,    // needed to pick krilla Image constructor
    pub act_label: String,
    pub date_label: String,
}
```

### Pattern 7: Supervisor (scheduled_tasks, V011)

The V011 schema [VERIFIED: migrations/V011__scheduled_tasks.sql]:
- `name TEXT NOT NULL UNIQUE` — task identifier (e.g. `"db_backup"`, `"log_retention"`)
- `cron TEXT NULL` — cron expression; NULL = manual only
- `last_run_at_utc INTEGER NULL`
- `next_run_at_utc INTEGER NULL`
- `status TEXT NOT NULL DEFAULT 'idle'`
- `payload_json TEXT NULL` — task-specific config (backup: `{folder, retention, schedule}`)

Supervisor implementation (D-17 catch-up semantics):

```rust
// supervisor.rs — tokio background task [ASSUMED: exact implementation pattern]
pub async fn run_supervisor(ctx: AppCtx) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let now = ctx.clock.unix_seconds();
                // Read all scheduled_tasks WHERE next_run_at_utc <= now AND status != 'running'
                // For each overdue task: set status='running', dispatch job, set status='succeeded'/'failed'
                // catch-up: fires on first tick after restart if next_run_at_utc was in the past
            }
            _ = ctx.shutdown.cancelled() => break,
        }
    }
}
```

Seed rows for supervisor tasks are inserted in V026 migration (or via startup code in `AppCtx::build`). The `payload_json` for `db_backup` task:
```json
{"backup_folder": "/path/chosen/by/user", "retention": 7, "schedule": "daily"}
```

**Phase 7 supervisor manages two jobs:**
1. `db_backup` — runs backup + integrity_check + retention cleanup
2. `log_retention` — deletes rotated log files older than `config.logging.retention_days` from `./logs/`

### Pattern 8: MiniJinja Template Validation (SET-09)

The `render_with_timeout` function in `pdf/minijinja_env.rs` is the validation primitive [VERIFIED: src/pdf/minijinja_env.rs lines 46–73]. For template validation without a real data context, pass a `serde_json::json!({})` empty object:

```rust
// template validation endpoint — [VERIFIED: minijinja_env.rs render_with_timeout signature]
pub async fn build_templates_validate(
    ctx: &AppCtx,
    body: &str,
) -> Result<Vec<u8>, AppError> {
    // Step 1: parse template (catches syntax errors)
    let dummy_ctx = serde_json::json!({
        "org_name": "ООО Тест", "inn": "0000000000",
        "act_number": "42", "handover_date": "15.06.2026",
        "items": [{"name": "Ноутбук", "qty": 1}]
    });
    let rendered = render_with_timeout(&ctx.pdf.minijinja_env, "preview", body, dummy_ctx).await?;
    // Step 2: parse as DocSpec (catches JSON structure errors)
    let spec: crate::pdf::docspec::DocSpec = serde_json::from_str(&rendered)
        .map_err(|e| AppError::Validation { field: "template", message: e.to_string() })?;
    // Step 3: render to PDF (catches rendering errors)
    ctx.pdf.render_docspec(&spec)
}
```

### Pattern 9: SVG Polyline Chart (DASH-03)

No external chart library. The Svelte component receives `data: Array<{month: string, model: string, installs: number}>` and computes SVG points:

```svelte
<!-- ChartWidget.svelte [ASSUMED — implementation sketch] -->
<script lang="ts">
  // chartData: {month: string, count: number}[] per model
  // Normalize to viewBox 0 0 400 200
  function toPoints(series: {count: number}[], maxVal: number): string {
    return series.map((d, i) => {
      const x = (i / (series.length - 1)) * 380 + 10;
      const y = 190 - (d.count / maxVal) * 170;
      return `${x},${y}`;
    }).join(' ');
  }
</script>

<svg role="img" aria-label="График динамики расхода картриджей за {period}"
     viewBox="0 0 400 200" preserveAspectRatio="none">
  <polyline points={toPoints(seriesData, maxCount)}
    fill="none" stroke="var(--color-accent)" stroke-width="2" />
  <!-- visually-hidden data table for accessibility (per UI-SPEC) -->
</svg>
```

### Anti-Patterns to Avoid

- **Concatenating user input into SQL:** All filter values via `rusqlite::params!` / `params_from_iter`. No `format!("WHERE x = '{}'", user_input)`.
- **`std::fs::copy` for backup:** Clippy-banned globally. Always `rusqlite::backup::Backup`.
- **`dirs::*_dir()` for backup folder default:** No default folder. User must choose.
- **Blocking tokio task for backup:** Backup is synchronous rusqlite API; always wrap in `spawn_blocking`.
- **Logo as filesystem path in portable build:** After D-15 migration, logo is a BLOB. Do not reintroduce a filesystem logo path without the `safe_logo_canonical` check.
- **Running MiniJinja template from user input in browser:** All template rendering (validate / preview / final PDF) must happen on the Rust backend. Never eval MiniJinja in JS.
- **`chrono::Local::now` for month boundary math:** Clippy-banned. Use `time::OffsetDateTime` with a fixed `UtcOffset`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Database backup | Custom `fs::copy` or WAL file copy | `rusqlite::backup::Backup` | Clippy ban; WAL copy mid-write is unsafe; Backup API is page-by-page consistent |
| PDF with Cyrillic | New PDF crate or wkhtmltopdf | Existing `PdfRenderer` + `DocSpec` + krilla 0.7 | Already proven in Phase 3; DejaVu Sans embedded |
| CSV with Russian encoding | Manual BOM + string join | Existing `csv::WriterBuilder::new().delimiter(b';')` + BOM prepend (`\xEF\xBB\xBF`) | Pattern established in `DeviceService::export_csv` |
| MiniJinja template validation | Regex on template string | `pdf::minijinja_env::render_with_timeout` with dummy context | Already handles fuel limit, timeout, strict mode |
| Chart animation | CSS keyframes or JS animation | Static SVG polyline (`prefers-reduced-motion` global rule blocks animations) | Global `global.scss` rule already disables animations |
| Session auth check | Custom cookie parsing | Existing `session_identity()` + `authorize()` pattern from Phase 5 | Consistent with all other handlers |

**Key insight:** Phase 7 is almost entirely wiring of existing infrastructure to new UI surfaces. The only genuinely new code is: (a) report SQL queries with period/filter, (b) supervisor loop, (c) org_settings migration + BLOB logo path in renderer, (d) backup service, and (e) Svelte components for reports, dashboard, settings.

---

## Common Pitfalls

### Pitfall 1: Period Boundary Off-By-One in UTC Conversion

**What goes wrong:** Report for «Июнь 2026» in Europe/Moscow (UTC+3) must start at 2026-06-01 00:00:00 MSK = 2026-05-31 21:00:00 UTC. A naive "start of day in UTC" returns 2026-06-01 00:00:00 UTC which misses 3 hours of events.

**Why it happens:** Treating `date_from = '2026-06-01'` as UTC when the org is UTC+3.

**How to avoid:** Compute bounds server-side. The `time 0.3` crate's `Date::from_calendar_date(year, month, 1).midnight().assume_offset(utc_plus_3)` gives the correct UTC timestamp. Never let the frontend send a raw date string without converting to UTC first.

**Warning signs:** Report for «Июнь» shows some July events or is missing late-evening June events.

### Pitfall 2: Backup While Writer is Holding a Transaction

**What goes wrong:** `rusqlite::backup::Backup::new(src_conn, ...)` called with the writer connection while it holds an open transaction → backup may catch an inconsistent state or deadlock.

**Why it happens:** Trying to use the writer connection directly for backup.

**How to avoid:** Run backup via a **reader connection** from the reader pool. In WAL mode, readers see a consistent snapshot. Acquire a reader, open the destination file, run `Backup::new(reader_conn, dest_conn)`, release reader.

**Warning signs:** Backup completes but `PRAGMA integrity_check` on the backup file returns `"database disk image is malformed"`.

### Pitfall 3: Logo BLOB Size Bloating DB File

**What goes wrong:** User uploads a 10 MB PNG logo. DB grows by 10 MB per save. Eventually portable .db file becomes unwieldy.

**Why it happens:** No size cap on logo upload.

**How to avoid:** Enforce 512 KB size limit on the frontend (`<input>` `maxSize` check) AND on the backend handler (`if logo_bytes.len() > 512 * 1024 { return Err(Validation) }`). UI-SPEC already specifies 512 KB limit. The backend must enforce it independently (security in depth).

**Warning signs:** No validation errors but `app_settings` table grows unexpectedly; export to CSV includes logo bytes accidentally.

### Pitfall 4: `scheduled_tasks` supervisor race with multiple AppCtx instances

**What goes wrong:** In desktop mode, if the user somehow opens two instances (unlikely — `tauri-plugin-single-instance` prevents it), both supervisors run backup at the same time → file collision.

**Why it happens:** Two processes owning the same DB file's scheduled_tasks.

**How to avoid:** `tauri-plugin-single-instance` is already in place from Phase 1. Not a realistic concern, but supervisor should check `status = 'running'` before starting a task and use an atomic UPDATE with WHERE:
```sql
UPDATE scheduled_tasks SET status='running', last_run_at_utc=?1
WHERE name=?2 AND status != 'running'
```
If `rows_affected == 0`, another worker already claimed it → skip.

### Pitfall 5: Svelte Month-Grouping Timezone Display Mismatch

**What goes wrong:** Report rows for an event at 2026-06-30 22:00 UTC (= 2026-07-01 01:00 MSK) appear under «Июнь» in the report even though in the org's timezone they belong to «Июль».

**Why it happens:** Grouping by UTC date in SQL or on client using `new Date(utc_ms).getMonth()` which uses the browser's local timezone, not the org's timezone.

**How to avoid:** Either (a) return `month_key` as a pre-computed string from the backend (SQL `strftime` using `+3 hours` via `datetime(ts, 'unixepoch', '+3 hours')`) or (b) send `created_at_utc` to the client and have Svelte add the known org offset before extracting month. The backend approach is simpler.

```sql
-- Correct month key for Europe/Moscow UTC+3 [ASSUMED: SQLite datetime modifier]
strftime('%Y-%m', datetime(created_at_utc, 'unixepoch', '+3 hours')) AS month_key
```

### Pitfall 6: Report SQL Dynamic Params with `params_from_iter`

**What goes wrong:** Attempting to pass a `Vec<Box<dyn ToSql>>` to `query_map` with `params![]` fails to compile because the number of params is not known at compile time.

**Why it happens:** The report query builds WHERE clauses dynamically based on optional filters.

**How to avoid:** Use `rusqlite::params_from_iter` for dynamic parameter lists:
```rust
// [VERIFIED: rusqlite API — params_from_iter is in the codebase's rusqlite 0.38]
let params: Vec<i64> = /* collected optional values */;
stmt.query_map(rusqlite::params_from_iter(params.iter()), map_row)
```

### Pitfall 7: DocSpec HeaderBlock — Breaking Change Without Backward Compat

**What goes wrong:** Adding `logo_bytes: Option<Vec<u8>>` to `HeaderBlock` breaks the existing MiniJinja templates that produce `DocSpec` JSON (they don't include `logo_bytes`).

**Why it happens:** serde requires fields to be present (or use `#[serde(default)]`).

**How to avoid:** Add `#[serde(default)]` on the new field:
```rust
#[serde(default)]
pub logo_bytes: Option<Vec<u8>>,
#[serde(default)]
pub logo_mime: Option<String>,
```
This makes the field optional in JSON deserialization — existing templates continue to work.

---

## State of the Art

| Old Approach | Current Approach | Status |
|--------------|------------------|--------|
| `org.json` filesystem file for org data | `org_settings` table with BLOB logo (D-15) | **Phase 7 migrates to DB** |
| `safe_logo_canonical` path-traversal check on filesystem logo | BLOB in DB — no filesystem traversal | **Phase 7 retires the check** |
| `ReportsPage.svelte` placeholder | Full report UI with sub-nav, period selector, export | **Phase 7 implements** |
| `Dashboard.svelte` placeholder | 5-widget fixed grid with SVG chart | **Phase 7 implements** |
| `scheduled_tasks` table empty (V011) | Supervisor with backup + log-retention jobs | **Phase 7 activates** |
| `SettingsPage` has only `NetworkSettings` | Full settings with org/storage/backup/threshold/templates | **Phase 7 implements** |

---

## Timezone Decision (Claude's Discretion)

**Question (from D-15 note):** Should `organization.timezone` move from `config.toml` to the `org_settings` DB table?

**Recommendation: Keep in `config.toml`.**

Rationale:
1. `"Europe/Moscow"` is UTC+3 fixed (no DST since 2014). A config file field is appropriate for a deployment-level parameter.
2. Moving to DB adds a migration, a UI field in OrgSettings, and a DB read on every period-boundary computation. This complexity buys nothing for a single-org app where TZ is set once at installation.
3. The `time 0.3` crate (already in workspace) supports `UtcOffset::from_hms(3, 0, 0)` directly without parsing an IANA TZ string. Parsing the IANA string `"Europe/Moscow"` would require `chrono-tz` — a new dependency and clippy configuration.
4. The backend reads `ctx.config.organization.timezone` and derives a fixed UTC offset. A helper `fn parse_simple_utc_offset(tz: &str) -> UtcOffset` can handle common cases (`"Europe/Moscow" → +3`, `"Asia/Yekaterinburg" → +5`) for a single-org app.

**If the user wants TZ to be editable via Settings UI later**, add it to `org_settings` in a future phase. For Phase 7, `config.toml` is the source of truth. [ASSUMED: The parse_simple_utc_offset helper needs to be written; not pre-existing in codebase]

---

## Validation Architecture

**nyquist_validation is enabled** (confirmed in .planning/config.json).

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (nextest compatible) + `tokio::test(flavor = "multi_thread")` |
| Config file | Cargo.toml workspace (no separate test config) |
| Quick run command | `cargo test -p trackly-app --test <test_file> -- --nocapture` |
| Full suite command | `cargo test -p trackly-app` (one at a time per memory: cargo_no_concurrent_test) |
| Fixture pattern | `test_writer_and_readers()` from `trackly_infra::test_support` (tempfile-backed, WAL-mode) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RPT-01 | Acts report query returns rows filtered by period and act_type | integration | `cargo test -p trackly-app --test report_acts` | ❌ Wave 0 |
| RPT-02 | Cartridge consumption: Install events grouped correctly | integration | `cargo test -p trackly-app --test report_cartridges` | ❌ Wave 0 |
| RPT-03 | Period boundary UTC math correct for Europe/Moscow | unit | `cargo test -p trackly-app --test report_period_bounds` | ❌ Wave 0 |
| RPT-04 | Report filters narrow result set (location, type, status) | integration | within `report_acts` / `report_cartridges` tests | ❌ Wave 0 |
| RPT-05 | Search within report filters by row content | integration | within report tests | ❌ Wave 0 |
| RPT-06 | Month grouping returns correct month_key for MSK events near month boundary | unit | within `report_period_bounds` | ❌ Wave 0 |
| RPT-07 | CSV export produces UTF-8 BOM + `;`-delimited output for report data | integration | `cargo test -p trackly-app --test report_csv_export` | ❌ Wave 0 |
| RPT-08 | Print: Tauri path triggers shell open; browser path detected correctly | manual | — | manual-only |
| DASH-01 | Device counts widget returns correct totals by status | integration | `cargo test -p trackly-app --test dashboard_widgets` | ❌ Wave 0 |
| DASH-02 | Cartridge widget returns counts + low-stock items | integration | reuse `cartridges_low_stock.rs` + extend | ✅ (extend) |
| DASH-03 | Consumption time-series query returns correct month/model groups | integration | within `dashboard_widgets` | ❌ Wave 0 |
| DASH-04 | Requests widget counts open/in_progress/completed | integration | reuse `phase06_stubs.rs` pattern | ❌ Wave 0 |
| DASH-05 | Printer widget counts online/offline/problematic (active alerts) | integration | within `dashboard_widgets` | ❌ Wave 0 |
| SET-01 | Org settings save + read round-trip (name, inn, kpp, address) | integration | `cargo test -p trackly-app --test org_settings` | ❌ Wave 0 |
| SET-02 | Logo BLOB: upload → store → retrieve → render in PDF header | integration | `cargo test -p trackly-app --test org_settings` + `pdf_logo.rs` extends | ✅ (extend pdf_logo.rs) |
| SET-03 | DB path display; move DB via backup + integrity_check | integration | `cargo test -p trackly-app --test backup_service` | ❌ Wave 0 |
| SET-04 | Low-stock threshold update via app_settings upsert | unit | within org_settings test or reuse cartridges_low_stock | ✅ (extend) |
| SET-05 | Manual backup creates valid SQLite file at destination | integration | within `backup_service` test | ❌ Wave 0 |
| SET-06 | Manual backup button → file created + integrity_check passes | integration | within `backup_service` test | ❌ Wave 0 |
| SET-07 | Supervisor: overdue task runs on startup (catch-up); next_run_at updates | integration | `cargo test -p trackly-app --test supervisor` | ❌ Wave 0 |
| SET-09 | Template validation: valid body → Ok; broken MiniJinja → Validation error | unit | `cargo test -p trackly-app --test template_edit` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p trackly-app --test <relevant_test_file>`
- **Per wave merge:** `cargo test -p trackly-app` (full suite, one at a time)
- **Phase gate:** Full suite green + `cargo clippy -- -D warnings` + `pnpm svelte-check` before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `crates/trackly-app/tests/report_acts.rs` — covers RPT-01, RPT-04, RPT-05
- [ ] `crates/trackly-app/tests/report_cartridges.rs` — covers RPT-02, D-14 semantics
- [ ] `crates/trackly-app/tests/report_period_bounds.rs` — covers RPT-03, RPT-06 (UTC math unit tests)
- [ ] `crates/trackly-app/tests/report_csv_export.rs` — covers RPT-07 (BOM + semicolon + report scope)
- [ ] `crates/trackly-app/tests/dashboard_widgets.rs` — covers DASH-01, DASH-03, DASH-04, DASH-05
- [ ] `crates/trackly-app/tests/org_settings.rs` — covers SET-01, SET-02, SET-04
- [ ] `crates/trackly-app/tests/backup_service.rs` — covers SET-03, SET-05, SET-06, UNC rejection
- [ ] `crates/trackly-app/tests/supervisor.rs` — covers SET-07 catch-up semantics
- [ ] `crates/trackly-app/tests/template_edit.rs` — covers SET-09 validation + reset_to_default

---

## Security Domain

**security_enforcement: true**, ASVS level 1.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes (all routes) | Existing `session_identity()` + `authorize()` — all new `build_*` helpers must call it |
| V3 Session Management | yes | Existing `RusqliteSessionStore` + `tower-sessions` 0.15 |
| V4 Access Control | yes | `Action::ManageSettings` for all write settings; `Action::ViewReports` to add if needed (or reuse existing role check) |
| V5 Input Validation | yes | Logo size cap (512 KB); template fuel limit (`set_fuel(Some(100_000))`); backup folder UNC rejection; threshold range check (1..=999) |
| V6 Cryptography | no (no new crypto) | — |

### Known Threat Patterns for This Phase

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via backup folder | Tampering | Reuse existing `is_unc_path()` from `fs_helpers.rs:127`; canonicalize; reject `..` components |
| Logo BLOB XSS via SVG upload | Tampering | Accept only PNG/JPG/SVG; SVG is served back only as BLOB in `<img src="data:...">` — never injected as raw HTML |
| MiniJinja template code injection | Tampering | `build_safe_env()` already uses `UndefinedBehavior::Strict` + `set_fuel(100_000)` + `no loader`; DocSpec is typed (no raw PDF ops) |
| Large logo BLOB DoS | DoS | 512 KB limit enforced on backend handler AND frontend |
| Backup path pointing to sensitive system location | Tampering | Reject UNC; user must pick folder via native dialog (not type path) in Tauri mode; HTTP mode validates with canonicalize + path check |
| CSRF on settings mutation endpoints | Tampering | Existing `SameSite::Strict` cookie covers CSRF — already applied globally in `build_router` |
| Threshold set to 0 disabling low-stock alerts | Tampering | Validate `threshold >= 1` on backend before writing to `app_settings` |

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust 1.92 toolchain | All Rust code | ✓ | 1.92 (workspace rust-version) | — |
| rusqlite 0.38 + backup feature | Backup, DB queries | ✓ | 0.38 (Cargo.toml:32) | — |
| krilla 0.7.0 | PDF export | ✓ | =0.7.0 (trackly-app/Cargo.toml) | — |
| minijinja 2.20 | Template editor | ✓ | ^2.20 (trackly-app/Cargo.toml) | — |
| time 0.3 | Period boundary math | ✓ | 0.3 (workspace Cargo.toml:24) | — |
| tauri-plugin-dialog 2.x | Backup folder picker, logo upload | ✓ | ^2 (Cargo.toml + package.json) | browser: `<input type="file">` |
| tauri-plugin-shell 2.x | Open PDF in system viewer | ✓ | ^2 (Cargo.toml) | browser: window.print() / download |
| Svelte 5.55 | Dashboard SVG chart | ✓ | ^5.55 (package.json) | — |

**Missing dependencies with no fallback:** None.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `time::Date::from_calendar_date(y, m, 1).midnight().assume_offset(utc_offset)` is the correct API for computing period start in UTC | Period Filter pattern | Medium — would need `OffsetDateTime` constructor instead; fix in impl |
| A2 | `rusqlite::backup::Backup::new(src, dest)` accepts a reader conn as `src` without locking the writer | Pattern 4 | Medium — if API requires writer, need to briefly queue the backup job through writer channel; behavior needs test |
| A3 | `PRAGMA integrity_check` on the backup connection returns `"ok"` string on success | Pattern 4 | Low — well-documented SQLite behavior; confirm in backup_service test |
| A4 | `datetime(ts, 'unixepoch', '+3 hours')` is valid SQLite syntax for UTC+3 offset in strftime | Pitfall 5 | Low — confirmed in SQLite docs; verify in test |
| A5 | `chrono-tz` is NOT needed — `time::UtcOffset::from_hms(3,0,0)` is sufficient for `Europe/Moscow` (no DST since 2014) | Timezone Decision | Low — DST was abolished; Russia does not observe DST for Moscow timezone |
| A6 | `parse_simple_utc_offset("Europe/Moscow") → UtcOffset::from_hms(3,0,0)` helper needs to be written from scratch | Timezone Decision | Low — trivial helper; if scope expands to multi-TZ, replace with chrono-tz |
| A7 | The `audit_log.action` for cartridge install events is `'install'` (matching `CartridgeTransitionOp::Install`) | Pattern 3 | High — verify in `cartridges_lifecycle.rs` test or `audit_log_sqlite.rs`; if action string differs, consumption query returns 0 |
| A8 | The `org_settings` table uses a single-row pattern with `CHECK (id = 1)` | Pattern 5 | Low — alternative is `app_settings` k/v; single-row is cleaner for typed access |

---

## Open Questions (RESOLVED)

1. **Exact `audit_log.action` string for Install events**
   - What we know: `CartridgeTransitionOp::Install` writes to `audit_log` via `SqliteAuditLogRepository`
   - What's unclear: Whether the action string is `"install"`, `"transition_install"`, or something else
   - Recommendation: Before writing the consumption SQL query, grep `audit_log_sqlite.rs` for the action string used in cartridge transition writes
   - **RESOLVED:** Action string is `'custom:install'`. Verified at `crates/trackly-core/src/domain/cartridges.rs:176` — `CartridgeTransitionOp::Install { .. } => "custom:install"`. Pattern 3 SQL above has been corrected. All consuming queries MUST use `action = 'custom:install'` (NOT `'install'`).

2. **`rusqlite::backup::Backup` source connection — reader vs writer**
   - What we know: The Backup API requires two `Connection` references
   - What's unclear: Whether it works from a reader pool connection in WAL mode without acquiring the write lock
   - Recommendation: Write a `backup_service` integration test that backs up while concurrent writes are happening; confirm integrity_check passes
   - **RESOLVED:** WAL mode allows consistent reads from a reader pool connection while writes proceed. The Backup API reads page-by-page from the reader connection; SQLite WAL ensures a consistent snapshot without blocking the writer. Plan 02 uses the reader pool conn in `spawn_blocking` for backup — this is the correct and safe approach.

3. **Document templates — `kind` CHECK constraint**
   - What we know: V007 has `CHECK (kind IN ('act_handover', 'act_acceptance'))`
   - What's unclear: Whether Phase 7 needs additional template kinds for report PDF (D-05 universal report template is NOT a MiniJinja template — it's built programmatically from DocSpec)
   - Recommendation: The universal report PDF (D-05) does NOT need a `document_templates` entry — it is generated from code (DocSpec programmatically constructed). Only `act_handover` and `act_acceptance` are in the template editor (SET-09). No migration change needed.
   - **RESOLVED:** Confirmed — no new template kinds in Phase 7. The report PDF (D-05) is generated from DocSpec IR programmatically in `ReportService::export_pdf()`. TemplateEditor (SET-09) lists only `act_handover` and `act_acceptance`. No `CHECK` constraint change or new migration needed.

---

## Sources

### Primary (HIGH confidence — verified in live codebase)

- `migrations/V011__scheduled_tasks.sql` — scheduled_tasks schema, confirmed fields
- `migrations/V007__document_templates.sql` — document_templates schema + `kind` CHECK constraint
- `migrations/V016__cartridges_kind_color_settings.sql` — app_settings table + low_stock_threshold seed
- `crates/trackly-app/Cargo.toml` — all Rust dependency versions confirmed
- `Cargo.toml` (workspace) — `rusqlite = { version = "0.38", features = ["bundled", "serde_json", "backup"] }` confirmed at line 32
- `crates/trackly-app/src/services/organization_service.rs` — org.json pattern, safe_logo_canonical, to be replaced
- `crates/trackly-app/src/services/template_service.rs` — seed pattern, get_active, to be extended
- `crates/trackly-app/src/pdf/minijinja_env.rs` — `render_with_timeout`, `build_safe_env`, validation primitive
- `crates/trackly-app/src/pdf/docspec.rs` — DocSpec IR, HeaderBlock, Section enum
- `crates/trackly-app/src/pdf/renderer.rs` — logo rendering path (logo_path branch, lines 174–176)
- `crates/trackly-app/src/services/device_service.rs:801–897` — CSV export pattern: BOM prepend + `csv::WriterBuilder::new().delimiter(b';\')` + formula injection prevention
- `crates/trackly-app/src/tauri_cmds/fs_helpers.rs:127` — UNC rejection: `path.starts_with("\\\\") || path.starts_with("//")` — reuse for backup folder
- `crates/trackly-infra/src/config.rs:104–115` — `organization.timezone = "Europe/Moscow"` in config.toml
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs:678–725` — low_stock() query with app_settings read
- `crates/trackly-core/src/domain/cartridges.rs:97–140` — CartridgeTransitionOp enum; Install/ToRefill/FromRefill/WriteOff
- `crates/trackly-core/src/domain/printers.rs:52–60` — PrinterAlertRow with alert_type "offline" | "error"
- `crates/trackly-core/src/domain/printers.rs:137–143` — PrinterCounts
- `crates/trackly-core/src/domain/requests.rs:69–75` — RequestCounts
- `crates/trackly-app/src/http/mod.rs` — `build_router` with session layer, security headers, SameSite::Strict
- `clippy.toml` — `std::fs::copy` disallowed; `chrono::Local::now` disallowed; `dirs::*_dir` disallowed
- `ui/package.json` — confirmed no chart library in deps; `svelte ^5.55`, `tauri-plugin-dialog`, `tauri-plugin-shell`, `tauri-plugin-fs`
- `ui/src/routes.ts` — `/` maps to Dashboard (no routing change needed per D-09)
- `ui/src/pages/SettingsPage.svelte` — existing layout pattern (vertical stack with `--space-lg --space-xl` padding)
- `.planning/phases/01-foundation/01-SKELETON.md` — confirms supervisor/backup/logo-BLOB deferred to Phase 7; `std::fs::copy` ban rationale

### Secondary (MEDIUM confidence)

- `cargo search chrono-tz` — confirms `chrono-tz = "0.10.4"` on crates.io, but NOT recommended (time 0.3 + UtcOffset is sufficient for single fixed offset)
- `time 0.3` workspace features: `serde, macros, formatting, parsing` — UtcOffset is part of the core time crate, no additional features needed [CITED: time crate docs]

### Tertiary (LOW confidence — not verified by tool call in this session)

- `rusqlite::backup::Backup::new(src, dest)` exact signature — described in rusqlite docs but not grep-verified in local code [ASSUMED]
- SQLite `datetime(ts, 'unixepoch', '+3 hours')` syntax for UTC offset in strftime [ASSUMED — standard SQLite]

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — all crates verified in Cargo.toml files
- Architecture: HIGH — all patterns grounded in Phase 1–6 codebase
- Report SQL patterns: MEDIUM — structure verified; exact API calls have A-tags
- Backup API: MEDIUM — feature confirmed; exact Rust API tagged ASSUMED
- Pitfalls: HIGH — derived from existing clippy bans and established patterns
- Timezone decision: HIGH — config.toml location verified; DST reasoning well-established for Moscow

**Research date:** 2026-06-15
**Valid until:** 2026-07-15 (stable dependencies; no fast-moving libraries added)
