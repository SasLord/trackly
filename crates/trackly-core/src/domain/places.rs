//! Domain value types for the Places entity (adjacency-list location tree, Phase 39).
//!
//! `places` replaces the flat `locations` table (V037/V038): every device/cartridge/act
//! now points at a `place_id` in a tree of territories/zones/buildings/floors/rooms/outdoor
//! objects instead of a flat freeform-name `locations` row.
//!
//! NO serde::Serialize/Deserialize or specta::Type derives here — those live in the DTO
//! layer in trackly-app (mirrors `domain::devices` convention). Only
//! `#[derive(Debug, Clone, PartialEq, Eq)]`.

use crate::error::AppError;

/// The six closed place-kind values (D-02). Not a freeform string — an unrecognized
/// token is rejected with `AppError::Validation` at the domain boundary, before it can
/// reach the repository/SQL layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceKind {
    /// Территория (внешний периметр, кампус).
    Territory,
    /// Зона (участок территории).
    Zone,
    /// Здание.
    Building,
    /// Этаж (уровень внутри здания — может быть 0 или отрицательным, PLC-02).
    Floor,
    /// Помещение (кабинет, склад).
    Room,
    /// Уличный объект (вне здания, но привязан к территории/зоне).
    Outdoor,
}

impl PlaceKind {
    /// Parse from the DB token. Returns `AppError::Validation` for unknown values,
    /// with a Russian-language message listing all six permitted values.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "territory" => Ok(Self::Territory),
            "zone" => Ok(Self::Zone),
            "building" => Ok(Self::Building),
            "floor" => Ok(Self::Floor),
            "room" => Ok(Self::Room),
            "outdoor" => Ok(Self::Outdoor),
            other => Err(AppError::Validation {
                field: "kind".to_string(),
                message: format!(
                    "Неизвестный тип места: «{other}». Допустимые значения: \
                     territory, zone, building, floor, room, outdoor."
                ),
            }),
        }
    }

    /// Returns the DB token corresponding to this kind (inverse of `from_str`).
    /// Russian labels are a UI-layer concern, not domain.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Territory => "territory",
            Self::Zone => "zone",
            Self::Building => "building",
            Self::Floor => "floor",
            Self::Room => "room",
            Self::Outdoor => "outdoor",
        }
    }
}

/// A single place row as returned from the repository read path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceRow {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub kind: PlaceKind,
    pub name: String,
    /// Floor level. May be 0 or negative (PLC-02). `None` for kinds where level
    /// is not meaningful (territory/zone/building/room/outdoor).
    pub level: Option<i64>,
    pub is_storage: bool,
    /// Manual sibling-ordering override (D-05). When set, wins over `level`/name.
    pub sort_order: Option<i64>,
    pub archived_at_utc: Option<i64>,
    pub notes: Option<String>,
    /// Resolved root-to-leaf path via `LEFT JOIN place_full_paths` — populated on
    /// read paths only, never stored (mirrors `DeviceRow.location` convention).
    pub full_path: Option<String>,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub deleted_at_utc: Option<i64>,
    pub version: i64,
}

/// Data needed to create a new place record. `parent_id: None` creates a root node —
/// place assignment on devices/cartridges is optional (D-07), and so is having a parent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceNew {
    pub parent_id: Option<i64>,
    pub kind: PlaceKind,
    pub name: String,
    pub level: Option<i64>,
    pub is_storage: bool,
    pub sort_order: Option<i64>,
    pub notes: Option<String>,
}

/// Partial update for a place — all fields optional (mirrors `DevicePatch`'s
/// all-optional shape). Tree-structural changes (rename, move) also have their own
/// dedicated `PlaceRepository` methods with optimistic-lock CAS.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlacePatch {
    pub parent_id: Option<i64>,
    pub kind: Option<PlaceKind>,
    pub name: Option<String>,
    pub level: Option<i64>,
    pub is_storage: Option<bool>,
    pub sort_order: Option<i64>,
    pub notes: Option<String>,
}

/// Filter parameters for place list queries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlaceFilter {
    pub include_archived: bool,
    /// `None` — no parent filter. `Some(None)` — only root nodes. `Some(Some(id))` —
    /// only direct children of `id`.
    pub parent_id: Option<Option<i64>>,
}

/// Subtree counts under a root place, inclusive of the root itself (Pattern 2 —
/// reused by D-14 delete-block, D-21 consequences preview, D-25 tree counters, PLC-06).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SubtreeStats {
    pub direct_children: i64,
    pub nested_places: i64,
    pub device_count: i64,
    pub cartridge_count: i64,
    /// Distinct live (non-soft-deleted) acts referencing any place in the subtree
    /// through `acts.place_id`, `acts.bulk_place_id`, or `act_items.place_id_override`
    /// (CR-01, phase 39 review): D-16 freezes these references even after every
    /// device has moved away, so a place can be otherwise-empty yet still
    /// undeletable — `ON DELETE RESTRICT` on all three columns enforces this at
    /// the schema level (V038), this counter surfaces it in the pre-flight check.
    pub referencing_act_count: i64,
}

/// A single row in the "content of place" listing (PLC-06 / D-23 — one table, column
/// «Тип»). `kind` values are application-layer literals (`"device"`, `"printer"`,
/// `"cartridge"`), not a closed Rust enum — Phase 41 adds `"workstation"` without
/// touching this struct's shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceContentRow {
    pub kind: String,
    pub id: i64,
    pub name: String,
    pub inventory_or_code: Option<String>,
    pub full_path: String,
    pub status_name: Option<String>,
}

/// Default sibling ordering (D-05): `sort_order` wins if both siblings have it set,
/// else `level` (floors, including 0 and negatives — PLC-02) wins if both have it set,
/// else fall back to natural name comparison.
pub fn sibling_cmp(a: &PlaceRow, b: &PlaceRow) -> std::cmp::Ordering {
    if let (Some(sa), Some(sb)) = (a.sort_order, b.sort_order) {
        return sa.cmp(&sb);
    }
    if let (Some(la), Some(lb)) = (a.level, b.level) {
        return la.cmp(&lb);
    }
    natural_name_cmp(&a.name, &b.name)
}

/// Natural-order string comparison (D-05): splits both strings into alternating
/// ASCII-digit/non-digit runs, compares digit runs as `u64` (so "2" < "10"), and
/// compares non-digit runs as plain string slices.
pub fn natural_name_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();

    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(ca), Some(cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let na = take_digit_run(&mut ai);
                    let nb = take_digit_run(&mut bi);
                    match na.cmp(&nb) {
                        std::cmp::Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    let sa = take_non_digit_run(&mut ai);
                    let sb = take_non_digit_run(&mut bi);
                    match sa.cmp(&sb) {
                        std::cmp::Ordering::Equal => continue,
                        other => return other,
                    }
                }
            }
        }
    }
}

/// Consumes a run of ASCII digits from `it` and parses it as `u64`.
fn take_digit_run(it: &mut std::iter::Peekable<std::str::Chars>) -> u64 {
    let mut s = String::new();
    while let Some(&c) = it.peek() {
        if c.is_ascii_digit() {
            s.push(c);
            it.next();
        } else {
            break;
        }
    }
    s.parse().unwrap_or(0)
}

/// Consumes a run of non-digit characters from `it`.
fn take_non_digit_run(it: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut s = String::new();
    while let Some(&c) = it.peek() {
        if !c.is_ascii_digit() {
            s.push(c);
            it.next();
        } else {
            break;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(id: i64, level: Option<i64>, sort_order: Option<i64>, name: &str) -> PlaceRow {
        PlaceRow {
            id,
            parent_id: None,
            kind: PlaceKind::Room,
            name: name.to_string(),
            level,
            is_storage: false,
            sort_order,
            archived_at_utc: None,
            notes: None,
            full_path: None,
            created_at_utc: 0,
            updated_at_utc: 0,
            deleted_at_utc: None,
            version: 1,
        }
    }

    // PlaceKind::from_str / as_str

    #[test]
    fn place_kind_from_str_floor() {
        assert_eq!(PlaceKind::from_str("floor").unwrap(), PlaceKind::Floor);
    }

    #[test]
    fn place_kind_from_str_unknown_lists_all_six_values_in_russian() {
        let err = PlaceKind::from_str("attic").expect_err("должна быть ошибка");
        match err {
            AppError::Validation { field, message } => {
                assert_eq!(field, "kind");
                for token in ["territory", "zone", "building", "floor", "room", "outdoor"] {
                    assert!(
                        message.contains(token),
                        "message should list '{token}': {message}"
                    );
                }
                assert!(
                    message
                        .chars()
                        .any(|c| ('а'..='я').contains(&c.to_ascii_lowercase())
                            || ('А'..='Я').contains(&c)),
                    "message should contain Russian text: {message}"
                );
            }
            other => panic!("ожидали AppError::Validation, получили {other:?}"),
        }
    }

    #[test]
    fn place_kind_as_str_roundtrips_all_six_variants() {
        for kind in [
            PlaceKind::Territory,
            PlaceKind::Zone,
            PlaceKind::Building,
            PlaceKind::Floor,
            PlaceKind::Room,
            PlaceKind::Outdoor,
        ] {
            let s = kind.as_str();
            assert_eq!(PlaceKind::from_str(s).unwrap(), kind);
        }
    }

    // sibling_cmp / natural_name_cmp

    #[test]
    fn sibling_cmp_orders_negative_zero_positive_levels() {
        let a = place(1, Some(-1), None, "Подвал");
        let b = place(2, Some(0), None, "Первый этаж");
        let c = place(3, Some(2), None, "Третий этаж");
        assert_eq!(sibling_cmp(&a, &b), std::cmp::Ordering::Less);
        assert_eq!(sibling_cmp(&b, &c), std::cmp::Ordering::Less);
        assert_eq!(sibling_cmp(&a, &c), std::cmp::Ordering::Less);
    }

    #[test]
    fn sibling_cmp_falls_back_to_natural_name_cmp_without_level_or_sort_order() {
        let a = place(1, None, None, "2");
        let b = place(2, None, None, "10");
        assert_eq!(sibling_cmp(&a, &b), std::cmp::Ordering::Less);
    }

    #[test]
    fn natural_name_cmp_compares_numeric_runs_as_integers() {
        assert_eq!(natural_name_cmp("2", "10"), std::cmp::Ordering::Less);
        assert_eq!(
            natural_name_cmp("Каб. 2", "Каб. 10"),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn sibling_cmp_sort_order_wins_regardless_of_level_or_name() {
        // b has a "later" name and "later" level but an earlier sort_order — must win.
        let a = place(1, Some(5), Some(2), "Zzz");
        let b = place(2, Some(-5), Some(1), "Aaa");
        assert_eq!(sibling_cmp(&b, &a), std::cmp::Ordering::Less);
    }
}
