//! Reports, Dashboard and Organisation Settings DTOs — Phase 7.
//!
//! All structs derive `Debug + Clone + Serialize + Deserialize + specta::Type`.
//! JSON field names follow the project-wide convention: **snake_case** (no camelCase
//! rename_all, per PATTERNS.md §Pattern 3 — verified in device.rs).
//!
//! These types define the interface contract for plans 02–07; later plans implement
//! the services that produce / consume these types.

use serde::{Deserialize, Serialize};
use specta::Type;

// ---------------------------------------------------------------------------
// Report filter / response
// ---------------------------------------------------------------------------

/// Common filter for all tabular reports (acts, returns, cartridges, devices).
///
/// All filter fields are optional; None means "no restriction for this dimension".
/// Consumed by ReportService::query_acts / query_cartridges (plan 03).
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct ReportFilter {
    /// Lower bound (Unix seconds UTC, inclusive).
    #[specta(type = Option<i32>)]
    pub date_from_utc: Option<i64>,
    /// Upper bound (Unix seconds UTC, inclusive).
    #[specta(type = Option<i32>)]
    pub date_to_utc: Option<i64>,
    /// Filter by place ID (D-28: subtree-inclusive — matches this place AND
    /// everything nested under it, not just an exact `place_id` match).
    #[specta(type = Option<i32>)]
    pub place_id: Option<i64>,
    /// HST-04 «Откуда» — движения из этого места (D-24: независимый,
    /// subtree-inclusive фильтр, как `place_id`). Комбинируется с
    /// `to_place_id` по AND, когда оба заданы (пример из CONTEXT.md — «со
    /// склада в Здание Б»).
    #[specta(type = Option<i32>)]
    pub from_place_id: Option<i64>,
    /// HST-04 «Куда» — движения в это место (D-24: независимый,
    /// subtree-inclusive фильтр, как `place_id`). Комбинируется с
    /// `from_place_id` по AND, когда оба заданы.
    #[specta(type = Option<i32>)]
    pub to_place_id: Option<i64>,
    /// Filter by cartridge / device status ID.
    #[specta(type = Option<i32>)]
    pub status_id: Option<i64>,
    /// Filter by device type ID (for device-domain reports, e.g. D-04 «Устройства → тип»).
    #[specta(type = Option<i32>)]
    pub type_id: Option<i64>,
    /// Filter by act type (e.g. "handover" | "return").
    pub act_type: Option<String>,
    /// Filter by cartridge model ID.
    #[specta(type = Option<i32>)]
    pub model_id: Option<i64>,
    /// Filter by cartridge color (for D-04 «Картриджи → цвет»).
    pub color: Option<String>,
    /// Free-text search (applied to act number, device name, personnel names).
    pub search: Option<String>,
    /// VAD-05/CATF-02 — funnel-фильтр домена «Заявки» по типу/категории.
    /// `None` = «Все» (без ограничения). `Some(vec![])` = все чекбоксы сняты,
    /// явный пустой результат. `Some(keys)` = allow-list ключей
    /// ('ad_register'|'cartridge_replace'|'repair'|'consumables'|'software'|
    /// 'no_category'|'other'), см. `category_filter_clause` в
    /// `report_service.rs`. Игнорируется остальными доменами (devices/
    /// cartridges).
    pub request_category_filter: Option<Vec<String>>,
    /// D-11.2/D-11.4: геометрический быстрый фильтр «на складе» —
    /// `Some(true)` = место (или любой предок) помечено `is_storage`;
    /// `Some(false)` = «в эксплуатации» (не в складском месте); `None` = без
    /// ограничения. Отдельное измерение от `status_id`/статуса предмета «на
    /// складе» — см. D-11.5, никогда не смешивать.
    pub is_storage: Option<bool>,
}

/// A single row in a tabular report.
///
/// Sparse — only the columns relevant to the report type will be populated.
/// Frontend renders only non-None fields, grouping by `month_key` when present.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ReportRow {
    /// Primary key of the underlying record (act id, cartridge id, device id, etc.).
    #[specta(type = i32)]
    pub id: i64,
    /// "YYYY-MM" grouping key for month-separator rendering.
    pub month_key: Option<String>,
    /// Act / return number (human-readable, e.g. "42" or "42 в1").
    pub number: Option<String>,
    /// Sub-number for return acts (e.g. "в1", "в2").
    pub sub_number: Option<String>,
    /// Person who handed over the item.
    pub giver_name: Option<String>,
    /// Person who received the item.
    pub receiver_name: Option<String>,
    /// Handover / return date (Unix seconds UTC).
    #[specta(type = Option<i32>)]
    pub handover_date_utc: Option<i64>,
    /// Resolved place path (`place_full_paths.full_path`).
    pub place_path: Option<String>,
    /// Shortened place path per `place_effective_variant` (D-17) — populated
    /// alongside `place_path` in all 5 report domains; `None` when the row has
    /// no place (mirrors `place_path`'s own null semantics). Consumed by
    /// `row_field`'s `shorten: bool` switch — CSV (D-18) always reads
    /// `place_path`, PDF/print (D-17) always reads this field.
    pub place_path_short: Option<String>,
    /// Act type discriminator ("handover" | "return" | etc.).
    pub act_type: Option<String>,
    /// Device name or cartridge display name.
    pub device_name: Option<String>,
    /// Quantity (for non-unique devices).
    #[specta(type = Option<i32>)]
    pub quantity: Option<i64>,
    /// Cartridge code (C-000001 format).
    pub code: Option<String>,
    /// Human-readable cartridge model label (e.g. "HP CB435A / Чёрный").
    pub model_label: Option<String>,
    /// Status name (for device / cartridge status reports).
    pub status_name: Option<String>,
    /// Russian-translated `requests.request_type` (VAD-03). Populated only for
    /// the «Заявки» report domain (`requests_all`/`open`/`in_progress`/`completed`);
    /// rendered in the «Тип» column on screen, in CSV, and in print — computed
    /// once on the backend so all three outputs stay in sync.
    pub request_type_label: Option<String>,
    /// HST-04 «Откуда» для отчёта о перемещениях — снапшот полного пути
    /// (`place_movements.from_place_path`, D-16-style заморозка). «Куда»
    /// переиспользует существующие `place_path`/`place_path_short` (D-23) —
    /// НЕ отдельное поле, во избежание путаницы с колонкой «Место» других
    /// доменов отчётов (Pitfall 7).
    pub from_place_path: Option<String>,
    /// Сокращённый вариант `from_place_path` — та же формула
    /// (`place_path_display::compute_place_path_short`, D-18/D-20 — единственный
    /// владелец), что и `place_path_short`, применённая к «from»-снапшоту.
    pub from_place_path_short: Option<String>,
    /// HST-04 «Кем» — ФИО из `actor_name_snapshot`, иначе `users.login`,
    /// иначе «система» (D-11).
    pub actor_name: Option<String>,
    /// HST-04 «Причина» — готовая строка, сформированная на бэкенде
    /// («актом №42» / «вручную» / «вручную · с примечанием» / raw source
    /// token как soft-degrade), НЕ собирается из частей на фронте (D-23).
    pub reason: Option<String>,
    /// HST-04: тип сущности перемещения — «Устройство» / «Картридж»
    /// (`MovementEntityKind::label_ru`), для отчётной колонки «Тип».
    pub entity_type_label: Option<String>,
    /// D-25 — предмет (устройство/картридж), которого касалось это
    /// перемещение, с тех пор был мягко удалён. Строка остаётся в отчёте
    /// (прошлый период не должен «плыть» из-за списания задним числом),
    /// но помечается на экране/в CSV/PDF как «удалено».
    pub is_deleted: Option<bool>,
}

/// Paged response wrapper for report queries.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ReportResponse {
    pub rows: Vec<ReportRow>,
    /// Total matching rows (before paging) — used to render pagination controls.
    #[specta(type = i32)]
    pub total: i64,
}

// ---------------------------------------------------------------------------
// Cartridge consumption chart
// ---------------------------------------------------------------------------

/// A single data point in the cartridge consumption time-series chart (DASH-03).
///
/// `installs` counts audit_log rows where action = 'custom:install'
/// for the given model within the given month.
/// audit_log.action for cartridge install = 'custom:install'
/// (verified in CartridgeTransitionOp::audit_action).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ConsumptionPoint {
    /// "YYYY-MM" (e.g. "2026-06").
    pub month_key: String,
    /// Human-readable model label (e.g. "HP CB435A / Чёрный").
    pub model_label: String,
    /// Number of installs in this month.
    #[specta(type = i32)]
    pub installs: i64,
}

// ---------------------------------------------------------------------------
// Dashboard widget
// ---------------------------------------------------------------------------

/// Count for a single status bucket (used in dashboard status breakdowns).
///
/// Named `DashboardStatusCount` to avoid collision with `device.rs::StatusCount`
/// (which uses `status_id: i64`). See D-07-01 decision in STATE.md.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DashboardStatusCount {
    pub status_name: String,
    #[specta(type = i32)]
    pub count: i64,
}

/// Single aggregate response for the Dashboard main widget query (DASH-01..05).
///
/// One round-trip returns all widget data — avoids multiple concurrent queries
/// from the frontend on load.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DashboardWidgetDto {
    // --- Devices widget (DASH-01) ---
    #[specta(type = i32)]
    pub devices_total: i64,
    pub devices_by_status: Vec<DashboardStatusCount>,

    // --- Cartridges widget (DASH-02) ---
    pub cartridge_by_status: Vec<DashboardStatusCount>,
    #[specta(type = i32)]
    pub low_stock_count: i64,
    /// Model labels (human-readable) for low-stock models — shown in tooltip/list.
    pub low_stock_models: Vec<String>,

    // --- Requests widget (DASH-04) ---
    #[specta(type = i32)]
    pub request_counts_open: i64,
    #[specta(type = i32)]
    pub request_counts_in_progress: i64,
    #[specta(type = i32)]
    pub request_counts_completed: i64,

    // --- Printers widget (DASH-05) ---
    #[specta(type = i32)]
    pub printer_online: i64,
    #[specta(type = i32)]
    pub printer_offline: i64,
    #[specta(type = i32)]
    pub printer_problematic: i64,
}

// ---------------------------------------------------------------------------
// Organisation settings
// ---------------------------------------------------------------------------

/// Partial update payload for organisation settings (SET-01).
///
/// All fields are required in the UI form but transmitted as a replace-all
/// (not a JSON merge patch) to keep the handler simple.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OrgPatch {
    pub org_name: String,
    pub inn: String,
    pub kpp: String,
    pub address: String,
    /// Extended requisites (PDFA-03, Phase 14). Empty string = not filled in.
    pub phone: String,
    pub fax: String,
    pub email: String,
    pub okpo: String,
    pub ogrn: String,
    /// Second address line (ORG-02, Phase 20). Empty string = not filled in.
    pub address_line2: String,
    /// Full legal name, multiline (DOC-05, Phase 34). Empty = not filled in.
    pub full_name: String,
}

/// Logo upload/download DTO (SET-02).
///
/// `logo_bytes` is `None` when there is no logo stored.
/// `serde(skip_serializing_if = "Option::is_none")` keeps JSON payloads lean
/// when returning "no logo" state.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OrgLogoDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_bytes: Option<Vec<u8>>,
    pub logo_mime: Option<String>,
}

/// Read-only view of org settings returned to the frontend (SET-01/02).
///
/// Does NOT include logo_bytes — logo is large; frontend queries it separately
/// via `org_logo_get` command to avoid slowing the settings page load.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OrgSettingsDto {
    pub org_name: String,
    pub inn: String,
    pub kpp: String,
    pub address: String,
    /// True if a logo is stored (logo_blob IS NOT NULL). Frontend shows
    /// "Remove logo" button only when this is true.
    pub has_logo: bool,
    /// Extended requisites (PDFA-03, Phase 14). Empty string = not filled in.
    pub phone: String,
    pub fax: String,
    pub email: String,
    pub okpo: String,
    pub ogrn: String,
    /// Second address line (ORG-02, Phase 20). Empty string = not filled in.
    pub address_line2: String,
    /// Full legal name, multiline (DOC-05, Phase 34). Empty = not filled in.
    pub full_name: String,
}

/// Organisation-wide default for place-path shortening (PLC-07, Phase 39.1).
///
/// `variant` is one of `PathDisplayVariant::as_str()`'s tokens (`"ends"` /
/// `"last_two"` / `"last"`), validated server-side via `PathDisplayVariant::from_str`
/// on SET. `sep_ends`/`sep_last_two` are stored and returned byte-for-byte —
/// whitespace is significant (D-09) and never trimmed.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OrgPathDisplayDto {
    pub variant: String,
    pub sep_ends: String,
    pub sep_last_two: String,
}

// ---------------------------------------------------------------------------
// Backup configuration
// ---------------------------------------------------------------------------

/// Partial update for backup settings (SET-05).
///
/// All fields are optional — only provided fields are updated.
/// `schedule` is a cron-like string (e.g. "daily" | "weekly" | "0 3 * * *").
/// `retention` is the number of backup copies to keep (older are pruned).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BackupConfigPatch {
    /// Destination folder path. Must not be a UNC path (\\server\share).
    pub backup_folder: Option<String>,
    /// Schedule descriptor: "daily" | "weekly" | "monthly" | cron expression.
    pub schedule: Option<String>,
    /// Number of backup copies to retain (min 1).
    #[specta(type = Option<i32>)]
    pub retention: Option<i64>,
}

// ---------------------------------------------------------------------------
// Period selector
// ---------------------------------------------------------------------------

/// Period selector DTO — shared across Reports and Dashboard date pickers.
///
/// `mode` determines which optional fields are meaningful:
///   - "month" → `year` + `month` required
///   - "year"  → `year` required
///   - "range" → `date_from` + `date_to` required (ISO date strings "YYYY-MM-DD")
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PeriodDto {
    /// "month" | "year" | "range"
    pub mode: String,
    pub year: Option<i32>,
    pub month: Option<u8>,
    /// ISO date string "YYYY-MM-DD" (e.g. "2026-06-01"), inclusive lower bound.
    pub date_from: Option<String>,
    /// ISO date string "YYYY-MM-DD" (e.g. "2026-06-30"), inclusive upper bound.
    pub date_to: Option<String>,
}

// ---------------------------------------------------------------------------
// Template editor
// ---------------------------------------------------------------------------

/// Элемент списка шаблонов для редактора (SET-07).
///
/// Возвращается `TemplateService::list_all_for_editor`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TemplateEditorItem {
    #[specta(type = i32)]
    pub id: i64,
    /// Kind discriminator — "act_handover" | "act_acceptance".
    pub kind: String,
    /// Текущее тело шаблона (MiniJinja source).
    pub body: String,
    /// True если тело совпадает с дефолтным (seeded from binary).
    pub is_default: bool,
}

/// D-17: per-file upgrade status for the file-based HTML template mechanism
/// (`crate::pdf::html_templates`, Plan 34-02). Unlike `TemplateEditorItem`
/// (DB-backed `document_templates` table, `is_default: bool`), this is
/// FILE-backed: there is no `is_default` column, so status is derived
/// structurally by comparing on-disk bytes against
/// `DEFAULT_HTML_TEMPLATES`/`KNOWN_LEGACY_DEFAULTS`.
///
/// A missing on-disk file folds into `Current` (the SAME "not yet
/// materialized, no evidence of user customization" reasoning
/// `upgrade_untouched_defaults_on_startup` already applies). A file that is
/// present but UNREADABLE does not — see `Unreadable` (WR-03).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum TemplateFileStatus {
    /// On-disk content matches the current bundled default, OR matches a
    /// known prior (legacy) default that the next startup will
    /// auto-upgrade in place, OR the file is missing (not yet materialized).
    Current,
    /// On-disk content matches neither the current bundled default nor any
    /// `KNOWN_LEGACY_DEFAULTS` snapshot — hand-customized by the user.
    Customized,
    /// The file EXISTS but cannot be read as UTF-8 text (permissions, or —
    /// the realistic trigger on the target platform — a Windows admin
    /// editing it in Notepad and saving as ANSI/Windows-1251, which Cyrillic
    /// content guarantees is not valid UTF-8).
    ///
    /// WR-03: this used to be indistinguishable from `Current`, so the one
    /// endpoint whose purpose is flagging hand-edited files reported the
    /// mangled file as fine while the user's edits silently did nothing (the
    /// embedded default rendered instead) — undiagnosable from inside the app.
    Unreadable,
}

/// D-17 response entry — one per file in `DEFAULT_HTML_TEMPLATES`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TemplateStatusDto {
    /// e.g. "act_handover.html", "_header.html".
    pub filename: String,
    pub status: TemplateFileStatus,
    /// Resolved templates directory (`TRACKLY_TEMPLATES_DIR` override or
    /// `<exe_dir>/templates`), repeated per entry for a flat response shape.
    pub templates_dir: String,
}

// ---------------------------------------------------------------------------
// Report tab counts (G2-5b)
// ---------------------------------------------------------------------------

/// A single entry in the per-tab report count response.
///
/// Vec-based (not HashMap) so specta::Type derives without feature flags and the
/// TypeScript binding types it as `Array<{ key: string; count: number }>`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ReportCountEntry {
    pub key: String,
    #[specta(type = i32)]
    pub count: i64,
}

/// Response for `reports_get_report_counts` — one entry per report-type tab.
///
/// `counts` order matches the UI tab order for the active domain.
/// Uses `Vec<ReportCountEntry>` (not HashMap) consistent with all other DTOs
/// in this file — see file-level doc comment.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ReportCountsDto {
    pub counts: Vec<ReportCountEntry>,
}
