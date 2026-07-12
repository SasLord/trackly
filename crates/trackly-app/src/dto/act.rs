//! Act DTOs — shared between Tauri command handlers and axum HTTP handlers.
//!
//! Snake_case JSON (S-2). All `i64` fields carry `#[specta(type = i32)]` so
//! TypeScript bindings see `number` rather than `bigint` — see `dto/device.rs`
//! module-doc for the rationale (S-3).
//!
//! `ActDto.number` is a `String` because the display rule «42» / «42в» /
//! «42в1» is applied at read time by the service layer
//! (`format_act_number`) — D-Numbering-01.

use serde::{Deserialize, Serialize};
use specta::Type;
use trackly_core::domain::acts::{ActCounts, ActFilter as DomainActFilter, ActRow, ActType};

/// Display-rule helper (D-Numbering-01).
///
/// - Handover: plain decimal — `"42"`
/// - Single return for a parent: drops the sub-number — `"42в"`
/// - Multiple returns for a parent: keeps it — `"42в1"`, `"42в2"`
///
/// `parent_number` and `sibling_return_count` come from the `ActRow` join
/// (see `SqliteActRepository::SELECT_ACTS`).
pub fn format_act_number(
    act_type: ActType,
    number: i64,
    sub_number: Option<i64>,
    parent_number: Option<i64>,
    sibling_return_count: Option<i64>,
) -> String {
    match act_type {
        ActType::Handover => number.to_string(),
        ActType::Return => {
            let sub = sub_number.unwrap_or(1);
            let parent = parent_number.unwrap_or(number);
            if sibling_return_count == Some(1) {
                format!("{parent}в")
            } else {
                format!("{parent}в{sub}")
            }
        }
    }
}

/// Public act DTO — what the UI receives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ActDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub version: i64,
    /// Formatted via `format_act_number` (D-Numbering-01).
    pub number: String,
    /// Raw counter value (without «в» suffix) — useful for sorting / re-display.
    #[specta(type = i32)]
    pub number_raw: i64,
    #[specta(type = Option<i32>)]
    pub sub_number: Option<i64>,
    /// `"handover"` | `"return"` (matches V004 CHECK constraint values).
    pub act_type: String,
    #[specta(type = Option<i32>)]
    pub parent_act_id: Option<i64>,
    pub giver_name: String,
    pub receiver_name: String,
    #[specta(type = Option<i32>)]
    pub location_id: Option<i64>,
    pub location: Option<String>,
    pub notes: Option<String>,
    #[specta(type = Option<i32>)]
    pub deadline_utc: Option<i64>,
    pub archived: bool,
    #[specta(type = i32)]
    pub created_at_utc: i64,
    #[specta(type = i32)]
    pub updated_at_utc: i64,
    #[specta(type = i32)]
    pub handover_date_utc: i64,
    pub items: Vec<ActItemDto>,
    /// IDs of linked return acts (for handover). Plan 02: always `[]`.
    /// Plan 03 (return lifecycle) populates this list; UI loads the full
    /// return rows separately via `acts_get` if needed.
    #[specta(type = Vec<i32>)]
    pub return_ids: Vec<i64>,
    /// D-07 (Phase 22) — «Дата архивации», derived read-time attribute,
    /// `MAX(handover_date_utc)` among the act's non-deleted `act_type='return'`
    /// children, populated ONLY when `archived == true`. No stored column,
    /// no migration (user decision 2026-07-12: compute-on-read over a stored
    /// `archived_at_utc` column — avoids a second source of truth that would
    /// need explicit recompute/clear on every un-return). Only
    /// `ActService::get()` populates this field in this phase; `list()`/
    /// `search()` leave it `None` (out of scope for this revision — only
    /// `ActDetail.svelte`'s single-act view needs to display it).
    #[specta(type = Option<i32>)]
    pub archived_at_utc: Option<i64>,
}

/// Single item line on an act (resolved with the joined device fields).
///
/// `outstanding_device_ids` (Phase 03.1, G-10/G-12): per handover act_item,
/// список device_id ещё НЕ возвращённых (canonical source для ReturnModal UI).
/// Для return-актов — всегда пустой vec (returns не возвращают сами себя).
/// Для handover-актов с qty=1 (single device): содержит `[device_id]` если не
/// возвращено, иначе `[]`. Для клонированных handover (qty>1): один device_id
/// в outstanding на каждый ещё-не-возвращённый clone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ActItemDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub device_id: i64,
    #[specta(type = i32)]
    pub quantity: i64,
    pub device_name: String,
    pub inventory_no: Option<String>,
    pub serial_no: Option<String>,
    pub model: Option<String>,
    /// Живое значение `devices.notes` («Технические характеристики»), НЕ снимок
    /// (D-01, Phase 14 plan 03): читается на момент рендера, а не на момент
    /// создания акта. `None` при отсутствующем значении — деградирует в шаблоне
    /// в «—»/пусто, не в ошибку.
    pub specs: Option<String>,
    pub condition_at_time: Option<String>,
    pub complectation_at_time: Option<String>,
    /// G-10 / G-12 (Phase 03.1): device_id'ы, ещё не возвращённые.
    /// Заполняется только для handover-актов; для return-актов всегда `[]`.
    #[specta(type = Vec<i32>)]
    pub outstanding_device_ids: Vec<i64>,
    /// Phase 22 (ACT-03, Pitfall 2): текущее расположение устройства
    /// (`devices.location_id`/joined `locations.name`), нужно для prefill
    /// «Расположение» в форме редактирования возврата. Populated by
    /// `load_items_for_act`'s `LEFT JOIN locations dl`; `None` when the
    /// device has no location set.
    #[specta(type = Option<i32>)]
    pub device_location_id: Option<i64>,
    pub device_location: Option<String>,
}

/// Payload sent by the UI when оформляет возврат по handover-акту.
///
/// Snapshot semantics (D-Acts-Return-01):
/// - per-row `condition_override` / `location_id_override` всегда побеждает.
/// - Если `apply_to_all = true` и override is `None` — используется bulk-значение.
/// - Если `apply_to_all = false` — каждый item обязан содержать override
///   (валидация на сервисе).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct ActReturnDto {
    pub bulk_condition: Option<String>,
    #[specta(type = Option<i32>)]
    pub bulk_location_id: Option<i64>,
    /// UX-friendly: имя расположения. Если задано — сервис резолвит в id
    /// (INSERT OR IGNORE → SELECT) и использует его. Имеет приоритет над
    /// `bulk_location_id` (UI обычно передаёт name, а не id).
    pub bulk_location_name: Option<String>,
    pub apply_to_all: bool,
    pub items: Vec<ActReturnItemDto>,
    /// Phase 22 (ACT-03, Pitfall 1 fix / D-12): who actually returned the
    /// device(s). Previously collected by `ReturnModal.svelte` but never
    /// sent to the backend — `do_return` hard-copied `parent.giver_name`
    /// instead. `None` = fall back to the existing parent-swap default
    /// (back-compat with not-yet-updated clients).
    #[serde(default)]
    pub giver_name: Option<String>,
    /// See `giver_name` doc — same rationale, `receiver_name` side.
    #[serde(default)]
    pub receiver_name: Option<String>,
    /// Phase 22 (ACT-03, D-05): «Дата возврата» — when devices were
    /// actually returned. `None` = fall back to `parent.handover_date_utc`
    /// (back-compat; existing V034-backfilled semantics).
    #[serde(default)]
    #[specta(type = Option<i32>)]
    pub handover_date_utc: Option<i64>,
}

/// Один пункт возврата в `ActReturnDto`.
///
/// Phase 03.1 (G-12 model shift): canonical новый contract — `device_ids: Vec<i64>`
/// (cardinality check vs `outstanding_device_ids`). Старые поля `device_id` +
/// `quantity` сохраняются для backward-compat с уже существующими тестами и
/// со старыми UI-версиями; backend применяет следующую логику:
///   - Если `device_ids` непустой → используется он (canonical).
///   - Иначе fallback: `device_ids := vec![device_id]` (legacy qty=1 path).
///   - `quantity` поле игнорируется backend-ом полностью в G-12 модели
///     (canonical count = device_ids.len()).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct ActReturnItemDto {
    #[specta(type = i32)]
    pub act_item_id: i64,
    /// Legacy single-device возврат — `device_ids` непустой имеет приоритет.
    #[specta(type = i32)]
    pub device_id: i64,
    /// Phase 03.1 canonical: список device_id для возврата по этой позиции.
    /// Используется UI-планами 03.1-03 (ReturnModal с чекбоксами per device).
    #[serde(default)]
    #[specta(type = Vec<i32>)]
    pub device_ids: Vec<i64>,
    /// Deprecated в Phase 03.1 G-12 model: остаётся в DTO для backward-compat,
    /// backend игнорирует (canonical count = `device_ids.len()` или 1 для legacy).
    #[specta(type = i32)]
    pub quantity: i64,
    pub condition_override: Option<String>,
    #[specta(type = Option<i32>)]
    pub location_id_override: Option<i64>,
    /// UX-friendly: per-row имя расположения; сервис резолвит. Приоритет
    /// над `location_id_override`.
    pub location_name_override: Option<String>,
}

/// Payload sent by the UI when creating a handover act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct ActCreateDto {
    /// `None` → service increments `counters.act_number`.
    /// `Some(n)` → service uses `n` (audited as `custom:act_number_override`).
    #[specta(type = Option<i32>)]
    pub number_override: Option<i64>,
    pub giver_name: String,
    pub receiver_name: String,
    #[specta(type = Option<i32>)]
    pub location_id: Option<i64>,
    /// UAT-fix: name-based location (UI обычно передаёт name через autocomplete).
    /// Если задано — backend resolves через `INSERT OR IGNORE locations → SELECT id`
    /// и использует. Приоритет над `location_id`.
    #[serde(default)]
    pub location_name: Option<String>,
    pub notes: Option<String>,
    #[specta(type = Option<i32>)]
    pub deadline_utc: Option<i64>,
    /// Phase 3.1 Plan 04 (G-2): дата фактической handover (когда отдали),
    /// независимая от `created_at_utc` (когда строка появилась в БД).
    /// `None` → service использует `clock.now_unix_seconds()` (backward-compat
    /// для clients, которые ещё не передают это поле).
    /// Persisted в `acts.handover_date_utc` (column из V015).
    #[serde(default)]
    #[specta(type = Option<i32>)]
    pub handover_date_utc: Option<i64>,
    pub items: Vec<ActItemNewDto>,
}

/// Single item line in `ActCreateDto.items`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct ActItemNewDto {
    #[specta(type = i32)]
    pub device_id: i64,
    /// UAT Fix #3/#4 (Phase 3.1): canonical путь — UI передаёт ВСЕ device_ids из
    /// одной "группы" (одинаковые name+model+inv_no=NULL+status=на_складе).
    /// Backend использует именно эти devices (без cloning'а — sparing stock).
    /// Если empty → fallback на legacy `device_id + quantity → clone`.
    #[serde(default)]
    #[specta(type = Vec<i32>)]
    pub device_ids: Vec<i64>,
    /// Defaults to 1 in the UI; persisted in `act_items.quantity` (V014).
    #[specta(type = i32)]
    pub quantity: i64,
}

/// Payload sent by the UI when editing an existing act's header + item set
/// (Phase 19, ACT-02). Consumed by `ActService::update` (Plan 19-03), which
/// destructures the header fields into `trackly_core::domain::acts::ActPatch`
/// and diffs `items` against the act's current `act_items`.
///
/// `items` is a **full replacement set**, not a delta — the service computes
/// added/retained/removed device_ids by comparing this list against what is
/// currently persisted for the act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct ActUpdateDto {
    #[specta(type = i32)]
    pub id: i64,
    /// CAS token — must match the act's current `version` or the update is
    /// rejected (`AppError::OptimisticLockMismatch`).
    #[specta(type = i32)]
    pub expected_version: i64,
    /// `None` = no rename requested (act keeps its current `number`).
    #[specta(type = Option<i32>)]
    pub number_override: Option<i64>,
    pub giver_name: String,
    pub receiver_name: String,
    #[specta(type = Option<i32>)]
    pub location_id: Option<i64>,
    /// UX-friendly: name-based location (mirrors `ActCreateDto.location_name`).
    #[serde(default)]
    pub location_name: Option<String>,
    pub notes: Option<String>,
    #[specta(type = Option<i32>)]
    pub deadline_utc: Option<i64>,
    /// Phase 19 (D-01/D-04): edit the act's «Когда отдали» date.
    /// `None` = no change requested (mirrors `ActCreateDto.handover_date_utc`).
    #[serde(default)]
    #[specta(type = Option<i32>)]
    pub handover_date_utc: Option<i64>,
    /// Full replacement set of items — see struct-level doc comment.
    pub items: Vec<ActUpdateItemDto>,
}

/// Single item line in `ActUpdateDto.items`.
///
/// `complectation_at_time` semantics (resolves RESEARCH.md Open Question 1 /
/// Pitfall 4):
/// - For a `device_id` **retained** across the edit (present both before and
///   after): `Some(v)` overwrites `act_items.complectation_at_time` for that
///   row; `None` leaves the existing value unchanged.
/// - For a **newly added** `device_id`: `None` falls back to the source
///   device's current `kit` value (mirrors `create`'s `source_before.kit`
///   default, see `act_service.rs:416`).
///
/// «Технические характеристики» (`ActItemDto.specs`, backed by the live
/// `devices.notes` column) has **no** corresponding field here and stays
/// read-only/out of scope for this phase — it is a live device attribute
/// read at render time, not an act-owned snapshot, and editing it would mean
/// mutating device data as an act-edit side effect (a materially different,
/// unreviewed security surface not covered by D-05/D-06).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct ActUpdateItemDto {
    #[specta(type = i32)]
    pub device_id: i64,
    pub complectation_at_time: Option<String>,
}

/// Payload sent by the UI when editing an existing **return** act (Phase 22,
/// ACT-03). Mirrors `ActUpdateDto`'s shape 1:1 EXCEPT:
/// - no `number_override` — return numbers never change (out of scope, D-10).
/// - adds `bulk_condition`/`bulk_location_id`/`bulk_location_name`/
///   `apply_to_all`, mirroring `ActReturnDto`'s own bulk-apply fields.
/// - `items: Vec<ActReturnItemDto>` — REUSES the existing return-item type
///   (not a new `ActUpdateReturnItemDto`); `device_ids[]`/`condition_override`/
///   `location_name_override` already match a full-replacement-set shape.
/// - `handover_date_utc` is a REQUIRED (non-`Option`) field — unlike
///   `ActUpdateDto`'s optional "no-change-requested" semantics, D-04 requires
///   the edit form to always show and submit a populated «Дата возврата».
///
/// `items` is a **full replacement set**, not a delta — `ActService::update_return`
/// (Plan 22-02) computes added/retained/removed device_ids by comparing this
/// list against what is currently persisted for the return act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct ActUpdateReturnDto {
    #[specta(type = i32)]
    pub id: i64,
    /// CAS token — must match the return act's current `version` or the
    /// update is rejected (`AppError::OptimisticLockMismatch`).
    #[specta(type = i32)]
    pub expected_version: i64,
    pub giver_name: String,
    pub receiver_name: String,
    #[specta(type = Option<i32>)]
    pub location_id: Option<i64>,
    #[serde(default)]
    pub location_name: Option<String>,
    pub notes: Option<String>,
    #[specta(type = Option<i32>)]
    pub deadline_utc: Option<i64>,
    /// «Дата возврата» — REQUIRED, D-04. See struct-level doc comment.
    #[specta(type = i32)]
    pub handover_date_utc: i64,
    pub bulk_condition: Option<String>,
    #[specta(type = Option<i32>)]
    pub bulk_location_id: Option<i64>,
    pub bulk_location_name: Option<String>,
    pub apply_to_all: bool,
    /// Full replacement set of items — see struct-level doc comment.
    pub items: Vec<ActReturnItemDto>,
}

/// Switch-bar counters returned by `acts_counts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct ActsCountsDto {
    #[specta(type = i32)]
    pub handover_active: i64,
    #[specta(type = i32)]
    pub returns: i64,
    #[specta(type = i32)]
    pub archived: i64,
}

impl From<ActCounts> for ActsCountsDto {
    fn from(c: ActCounts) -> Self {
        Self {
            handover_active: c.handover_active,
            returns: c.returns,
            archived: c.archived,
        }
    }
}

/// Filter passed by the UI to `acts_list`. `act_type` is the SQL string
/// (`"handover"` | `"return"` | absent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct ActFilter {
    pub act_type: Option<String>,
    pub archived: Option<bool>,
    pub search: Option<String>,
    pub include_deleted: bool,
}

impl ActFilter {
    /// Convert into the domain filter, validating the act_type string.
    pub fn into_domain(self) -> Result<DomainActFilter, trackly_core::error::AppError> {
        let act_type = match self.act_type.as_deref() {
            None => None,
            Some(s) => Some(ActType::from_str(s)?),
        };
        Ok(DomainActFilter {
            act_type,
            archived: self.archived,
            search: self.search,
            include_deleted: self.include_deleted,
        })
    }
}

/// Pagination — mirrors `dto::device::Pagination` (same shape, distinct type
/// to keep modules decoupled).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Pagination {
    #[specta(type = u32)]
    pub offset: u64,
    #[specta(type = u32)]
    pub limit: u64,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
        }
    }
}

/// Response of `acts_list` / `acts_search`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ActListResponse {
    pub items: Vec<ActDto>,
    #[specta(type = u32)]
    pub total: u64,
}

/// Builder used by service `get` / `list` after fetching items separately.
///
/// `items` should already be resolved (joined with devices). `returns` is
/// populated by plan 03 — in plan 02 callers always pass `vec![]`.
pub fn act_dto_from_row(row: ActRow, items: Vec<ActItemDto>, return_ids: Vec<i64>) -> ActDto {
    let number = format_act_number(
        row.act_type,
        row.number,
        row.sub_number,
        row.parent_number,
        row.sibling_return_count,
    );
    ActDto {
        id: row.id,
        version: row.version,
        number,
        number_raw: row.number,
        sub_number: row.sub_number,
        act_type: row.act_type.to_sql().to_string(),
        parent_act_id: row.parent_act_id,
        giver_name: row.giver_name,
        receiver_name: row.receiver_name,
        location_id: row.location_id,
        location: row.location,
        notes: row.notes,
        deadline_utc: row.deadline_utc,
        archived: row.archived,
        created_at_utc: row.created_at_utc,
        updated_at_utc: row.updated_at_utc,
        handover_date_utc: row.handover_date_utc,
        items,
        return_ids,
        archived_at_utc: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_handover_is_plain_number() {
        assert_eq!(
            format_act_number(ActType::Handover, 42, None, None, None),
            "42"
        );
    }

    #[test]
    fn format_single_return_drops_sub_suffix() {
        assert_eq!(
            format_act_number(ActType::Return, 999, Some(1), Some(42), Some(1)),
            "42в"
        );
    }

    #[test]
    fn format_multiple_returns_use_sub_suffix() {
        assert_eq!(
            format_act_number(ActType::Return, 999, Some(1), Some(42), Some(2)),
            "42в1"
        );
        assert_eq!(
            format_act_number(ActType::Return, 1000, Some(2), Some(42), Some(2)),
            "42в2"
        );
    }

    #[test]
    fn format_retroactive_promotion() {
        // Same row data — sub=1 — но при изменении sibling_count
        // отображение меняется с «42в» (один возврат) на «42в1» (два).
        // Это retroactive promotion из D-Numbering-01.
        let solo = format_act_number(ActType::Return, 999, Some(1), Some(42), Some(1));
        let with_sibling = format_act_number(ActType::Return, 999, Some(1), Some(42), Some(2));
        assert_eq!(solo, "42в");
        assert_eq!(with_sibling, "42в1");
    }

    #[test]
    fn snake_case_json_invariant() {
        let dto = ActCreateDto {
            number_override: Some(7),
            giver_name: "А".into(),
            receiver_name: "Б".into(),
            location_id: None,
            location_name: None,
            notes: None,
            deadline_utc: None,
            handover_date_utc: None,
            items: vec![],
        };
        let s = serde_json::to_string(&dto).expect("ser");
        assert!(
            s.contains("number_override"),
            "snake_case 'number_override'"
        );
        assert!(!s.contains("numberOverride"), "must NOT use camelCase");
    }

    #[test]
    fn act_update_dto_snake_case_json_invariant() {
        let dto = ActUpdateDto {
            id: 1,
            expected_version: 3,
            number_override: Some(7),
            giver_name: "А".into(),
            receiver_name: "Б".into(),
            location_id: None,
            location_name: None,
            notes: None,
            deadline_utc: None,
            handover_date_utc: Some(1_700_000_000),
            items: vec![ActUpdateItemDto {
                device_id: 5,
                complectation_at_time: Some("кабель, коробка".into()),
            }],
        };
        let s = serde_json::to_string(&dto).expect("ser");
        assert!(s.contains("expected_version"), "snake_case 'expected_version'");
        assert!(!s.contains("expectedVersion"), "must NOT use camelCase");
        assert!(s.contains("number_override"), "snake_case 'number_override'");
        assert!(!s.contains("numberOverride"), "must NOT use camelCase");
        assert!(
            s.contains("handover_date_utc"),
            "snake_case 'handover_date_utc'"
        );
        assert!(!s.contains("handoverDateUtc"), "must NOT use camelCase");
        assert!(
            s.contains("complectation_at_time"),
            "snake_case 'complectation_at_time'"
        );
        assert!(
            !s.contains("complectationAtTime"),
            "must NOT use camelCase"
        );
    }

    #[test]
    fn act_update_return_dto_snake_case_json_invariant() {
        let dto = ActUpdateReturnDto {
            id: 1,
            expected_version: 3,
            giver_name: "А".into(),
            receiver_name: "Б".into(),
            location_id: None,
            location_name: None,
            notes: None,
            deadline_utc: None,
            handover_date_utc: 1_700_000_000,
            bulk_condition: Some("Хорошее".into()),
            bulk_location_id: Some(9),
            bulk_location_name: Some("Склад".into()),
            apply_to_all: true,
            items: vec![ActReturnItemDto {
                act_item_id: 5,
                device_id: 10,
                device_ids: vec![10],
                quantity: 1,
                condition_override: None,
                location_id_override: None,
                location_name_override: None,
            }],
        };
        let s = serde_json::to_string(&dto).expect("ser");
        assert!(s.contains("expected_version"), "snake_case 'expected_version'");
        assert!(!s.contains("expectedVersion"), "must NOT use camelCase");
        assert!(
            s.contains("handover_date_utc"),
            "snake_case 'handover_date_utc'"
        );
        assert!(!s.contains("handoverDateUtc"), "must NOT use camelCase");
        assert!(s.contains("bulk_condition"), "snake_case 'bulk_condition'");
        assert!(!s.contains("bulkCondition"), "must NOT use camelCase");
        assert!(
            s.contains("bulk_location_id"),
            "snake_case 'bulk_location_id'"
        );
        assert!(!s.contains("bulkLocationId"), "must NOT use camelCase");
        assert!(
            s.contains("bulk_location_name"),
            "snake_case 'bulk_location_name'"
        );
        assert!(!s.contains("bulkLocationName"), "must NOT use camelCase");
        assert!(s.contains("apply_to_all"), "snake_case 'apply_to_all'");
        assert!(!s.contains("applyToAll"), "must NOT use camelCase");
        assert!(s.contains("\"items\""), "snake_case 'items'");
    }

    #[test]
    fn act_return_dto_back_compat_omitted_giver_receiver_date() {
        // Simulates an old/not-yet-updated client that omits giver_name,
        // receiver_name, and handover_date_utc entirely — proves the
        // #[serde(default)] back-compat contract holds (Pitfall 1 fix).
        let json = r#"{
            "bulk_condition": null,
            "bulk_location_id": null,
            "bulk_location_name": null,
            "apply_to_all": true,
            "items": []
        }"#;
        let dto: ActReturnDto = serde_json::from_str(json).expect("deserialize");
        assert_eq!(dto.giver_name, None);
        assert_eq!(dto.receiver_name, None);
        assert_eq!(dto.handover_date_utc, None);
    }
}
