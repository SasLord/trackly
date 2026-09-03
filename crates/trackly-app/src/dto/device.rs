//! Device DTOs — полная реализация Plan 03.
//!
//! `STATE_HINTS` определён здесь (DEV-10 / D-DeviceHints-01).
//! `DeviceDto`, `DeviceNew`, `DevicePatch`, `DeviceFilter`, `Pagination`, `DeviceListResponse` —
//! с derives `Serialize + Deserialize + specta::Type`.
//!
//! Snake_case JSON — НИКАКИХ `rename_all = "camelCase"` (PATTERNS.md §Pattern 3).

use serde::{Deserialize, Serialize};
use specta::Type;
use trackly_core::domain::devices::DeviceRow;

/// Quick-pick hints for the device "Состояние" (condition/state) field.
///
/// Static UI affordances — not database-driven.
/// Per DEV-10 and D-DeviceHints-01.
pub const STATE_HINTS: &[&str] = &[
    "Новое",
    "Новый в заводской упаковке, не вскрытый",
    "Новый в заводской упаковке, вскрытый, настроенное рабочее окружение (ОС)",
    "Хорошее",
    "Среднее",
    "Б/У",
];

/// Device DTO — полный набор полей, возвращаемый frontend'у.
///
/// `#[specta(type = i32)]` на `i64`-полях — specta-typescript запрещает BigInt
/// (i64/u64) по умолчанию. ID-значения и версии в SQLite умещаются в i32,
/// timestamps (Unix-секунды ≤ ~2 млрд до 2038) тоже помещаются. TypeScript
/// получает `number` — JSON-числа передаются без потерь.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct DeviceDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub version: i64,
    #[specta(type = i32)]
    pub type_id: i64,
    pub name: String,
    pub inventory_no: Option<String>,
    pub serial_no: Option<String>,
    pub model: Option<String>,
    /// Технические характеристики (notes в БД).
    pub specs: Option<String>,
    /// Комплектация (complectation в БД).
    pub kit: Option<String>,
    /// Состояние (condition в БД).
    pub state: Option<String>,
    #[specta(type = Option<i32>)]
    pub place_id: Option<i64>,
    /// Resolved full path (from `place_full_paths` view via LEFT JOIN).
    pub full_path: Option<String>,
    /// Resolved short path per the organization/place display-variant setting
    /// (Phase 39.1 / PLC-08). `None` on read paths that don't join
    /// `place_effective_variant` (autocomplete, restore-from-snapshot, D-19)
    /// or when the device has no place.
    pub place_path_short: Option<String>,
    #[specta(type = i32)]
    pub status_id: i64,
    #[specta(type = i32)]
    pub created_at_utc: i64,
    #[specta(type = i32)]
    pub updated_at_utc: i64,
}

impl From<DeviceRow> for DeviceDto {
    fn from(row: DeviceRow) -> Self {
        Self {
            id: row.id,
            version: row.version,
            type_id: row.type_id,
            name: row.name,
            inventory_no: row.inventory_no,
            serial_no: row.serial_no,
            model: row.model,
            specs: row.specs,
            kit: row.kit,
            state: row.state,
            place_id: row.place_id,
            full_path: row.full_path,
            place_path_short: row.place_path_short,
            status_id: row.status_id,
            created_at_utc: row.created_at_utc,
            updated_at_utc: row.updated_at_utc,
        }
    }
}

/// DTO для создания нового устройства.
///
/// `place_id` — уже разрешённый caller'ом ID места, выбранный через
/// PlacePicker; создание нового места по имени больше не поддерживается на
/// этом пути (D-18).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct DeviceNew {
    #[specta(type = i32)]
    pub type_id: i64,
    pub name: String,
    pub inventory_no: Option<String>,
    pub serial_no: Option<String>,
    pub model: Option<String>,
    pub specs: Option<String>,
    pub kit: Option<String>,
    pub state: Option<String>,
    #[specta(type = Option<i32>)]
    pub place_id: Option<i64>,
    #[specta(type = i32)]
    pub status_id: i64,
}

impl From<DeviceNew> for trackly_core::domain::devices::DeviceNew {
    fn from(dto: DeviceNew) -> Self {
        Self {
            type_id: dto.type_id,
            name: dto.name,
            inventory_no: dto.inventory_no,
            serial_no: dto.serial_no,
            model: dto.model,
            specs: dto.specs,
            kit: dto.kit,
            state: dto.state,
            place_id: dto.place_id,
            status_id: dto.status_id,
        }
    }
}

/// DTO для частичного обновления устройства.
/// `Option<Option<T>>` — None означает «не менять», Some(None) — «установить NULL»,
/// Some(Some(v)) — «установить v». Для обязательных полей: None = «не менять».
///
/// `place_id` — уже разрешённый caller'ом ID места (PlacePicker); создание
/// нового места по имени на этом пути не поддерживается (D-18).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct DevicePatch {
    #[specta(type = Option<i32>)]
    pub type_id: Option<i64>,
    pub name: Option<String>,
    pub inventory_no: Option<Option<String>>,
    pub serial_no: Option<Option<String>>,
    pub model: Option<Option<String>>,
    pub specs: Option<Option<String>>,
    pub kit: Option<Option<String>>,
    pub state: Option<Option<String>>,
    #[specta(type = Option<Option<i32>>)]
    pub place_id: Option<Option<i64>>,
    #[specta(type = Option<i32>)]
    pub status_id: Option<i64>,
}

impl From<DevicePatch> for trackly_core::domain::devices::DevicePatch {
    fn from(dto: DevicePatch) -> Self {
        // Преобразуем плоские Option-поля в domain::DevicePatch
        // (domain использует простые Option<T>, не Option<Option<T>>)
        // Для nullable полей (inventory_no и пр.) передаём внутренний Option.
        let mut p = trackly_core::domain::devices::DevicePatch::default();
        if let Some(v) = dto.type_id {
            p.type_id = Some(v);
        }
        if let Some(v) = dto.name {
            p.name = Some(v);
        }
        if let Some(inner) = dto.inventory_no {
            p.inventory_no = inner;
        }
        if let Some(inner) = dto.serial_no {
            p.serial_no = inner;
        }
        if let Some(inner) = dto.model {
            p.model = inner;
        }
        if let Some(inner) = dto.specs {
            p.specs = inner;
        }
        if let Some(inner) = dto.kit {
            p.kit = inner;
        }
        if let Some(inner) = dto.state {
            p.state = inner;
        }
        if let Some(inner) = dto.place_id {
            // Phase 40-28: сохраняем различие "поле не передано" (внешний
            // None, ветка if не выполняется) vs "поле передано явно, значение
            // NULL" (inner = None, p.place_id = Some(None)) — уплощение до
            // одинарного Option ранее делало оба случая неразличимыми.
            p.place_id = Some(inner);
        }
        if let Some(v) = dto.status_id {
            p.status_id = Some(v);
        }
        p
    }
}

/// Фильтр для списка устройств.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct DeviceFilter {
    #[specta(type = Option<i32>)]
    pub type_id: Option<i64>,
    #[specta(type = Option<i32>)]
    pub place_id: Option<i64>,
    #[specta(type = Option<i32>)]
    pub status_id: Option<i64>,
    pub state: Option<String>,
    /// Многополевой FTS5-текстовый фильтр (Phase 18/AUTO-03).
    /// Используется ТОЛЬКО в `list_grouped` при `group_by_condition=true` —
    /// сопоставляет наименование/инвентарный №/серийный №/модель. В `list()`/
    /// `export_csv` не используется (pre-existing gap, вне скоупа Phase 18).
    pub name_prefix: Option<String>,
    /// Включать ли мягко-удалённые устройства. По умолчанию false.
    pub include_deleted: bool,
    /// Если true (акт-форма/пикер устройства, Phase 18/D-04/D-05) — группировка
    /// по (type_id, name, model), сортировка по count DESC (остаток по убыванию),
    /// текстовый фильтр по name_prefix активен.
    /// Если false (по умолчанию, страница Устройств) — группировка по
    /// (type_id, name), сортировка по имени; не изменено Phase 18.
    pub group_by_condition: bool,
}

/// Параметры пагинации.
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

/// Ответ на запрос списка устройств.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct DeviceListResponse {
    pub items: Vec<DeviceDto>,
    #[specta(type = u32)]
    pub total: u64,
}

/// Группа одинаковых устройств (DEV-11 / D-Group-01).
///
/// Только для не-уникальных устройств (без inventory_number и serial_number).
/// `repr` — представительная строка группы (с MIN(id)).
/// `ids` — все ID в группе (для expand через `devices_list_by_ids`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct DeviceGroup {
    pub repr: DeviceDto,
    #[specta(type = u32)]
    pub count: u64,
    pub ids: Vec<i32>,
    /// Количество различных значений condition в группе.
    /// При `group_by_condition=false`: > 1 означает смешанную группу (отображается
    /// как «разное» на фронтенде).
    /// При `group_by_condition=true` (Phase 18+): condition больше НЕ входит в
    /// ключ группировки (ключ — (type_id, name, model)) — поле сигнализирует
    /// фронтенду о необходимости drill-in подгруппировки по condition (D-07).
    #[specta(type = i32)]
    pub condition_distinct_count: i64,
    /// Количество различных значений place в группе (Phase 40 Plan 26 — Фикс B).
    /// Значение больше 1 означает, что члены группы расположены в разных
    /// местах — фронтенд должен погасить `repr.place_path_short` (показать
    /// «—») вместо места произвольного члена группы.
    #[specta(type = i32)]
    pub place_distinct_count: i64,
}

/// Счётчик устройств по статусу.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct StatusCount {
    #[specta(type = i32)]
    pub status_id: i64,
    #[specta(type = u32)]
    pub count: u64,
}

// ---------------------------------------------------------------------------
// CSV Import / Export DTOs (Plan 05)
// ---------------------------------------------------------------------------

/// Response from `import_csv_preview` — includes token + decoded preview data.
///
/// `token: String` — UUID v4 serialized as hyphenated string (not Uuid type,
/// to avoid specta::Type complexity with non-primitive UUID types).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CsvImportPreviewResponse {
    /// Session token (UUID v4, hyphenated). Use in `import_csv_commit`.
    pub token: String,
    /// Detected encoding label, e.g. "UTF-8" or "windows-1251".
    pub encoding: String,
    /// Delimiter character, either "," or ";".
    pub delimiter: String,
    /// Column headers from the first CSV row.
    pub headers: Vec<String>,
    /// First 5 decoded data rows (or fewer if file is short).
    pub preview_rows: Vec<Vec<String>>,
    /// Total number of data rows (excluding header).
    #[specta(type = u32)]
    pub total_rows: u64,
    /// True if encoding_rs encountered replacement characters during decode —
    /// surface a warning to the user (RESEARCH §Pitfall 7).
    pub had_replacements: bool,
}

/// Result of a successful (or partial) `import_csv_commit` call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CsvImportReport {
    /// Number of devices successfully inserted.
    #[specta(type = u32)]
    pub inserted: u64,
    /// Per-row errors (rows that were skipped due to validation failures).
    pub failed: Vec<RowError>,
}

/// A per-row import error.
///
/// Flattened struct (no nested AppError) for simpler specta::Type / JSON shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RowError {
    /// 1-based row index (matching what the user sees in a spreadsheet).
    #[specta(type = u32)]
    pub row_index: u64,
    /// AppError variant code (e.g. "Validation").
    pub error_code: String,
    /// Human-readable error message in Russian.
    pub error_message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_round_trip_device_dto() {
        let dto = DeviceDto {
            id: 42,
            version: 3,
            type_id: 1,
            name: "Ноутбук Lenovo".to_string(),
            inventory_no: Some("INV-001".to_string()),
            serial_no: None,
            model: Some("ThinkPad X1".to_string()),
            specs: None,
            kit: None,
            state: Some("Хорошее".to_string()),
            place_id: Some(5),
            full_path: Some("Здание А / Склад".to_string()),
            place_path_short: Some("Здание А / Склад".to_string()),
            status_id: 2,
            created_at_utc: 1_700_000_000,
            updated_at_utc: 1_700_001_000,
        };
        let json = serde_json::to_string(&dto).expect("serialize");
        let back: DeviceDto = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, dto);
    }

    #[test]
    fn serde_round_trip_device_new() {
        let new = DeviceNew {
            type_id: 1,
            name: "Принтер HP".to_string(),
            inventory_no: None,
            serial_no: Some("SN-999".to_string()),
            model: None,
            specs: None,
            kit: None,
            state: None,
            place_id: None,
            status_id: 1,
        };
        let json = serde_json::to_string(&new).expect("serialize");
        let back: DeviceNew = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, new);
    }

    #[test]
    fn snake_case_json_invariant() {
        let dto = DeviceDto {
            id: 1,
            version: 1,
            type_id: 1,
            name: "Test".to_string(),
            inventory_no: Some("INV".to_string()),
            serial_no: None,
            model: None,
            specs: None,
            kit: None,
            state: None,
            place_id: None,
            full_path: None,
            place_path_short: None,
            status_id: 1,
            created_at_utc: 0,
            updated_at_utc: 0,
        };
        let json = serde_json::to_string(&dto).expect("serialize");
        assert!(
            json.contains("inventory_no"),
            "должен содержать snake_case 'inventory_no'"
        );
        assert!(
            json.contains("type_id"),
            "должен содержать snake_case 'type_id'"
        );
        assert!(
            json.contains("status_id"),
            "должен содержать snake_case 'status_id'"
        );
        assert!(
            !json.contains("inventoryNo"),
            "НЕ должен содержать camelCase"
        );
    }
}
