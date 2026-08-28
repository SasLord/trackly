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
    /// Per-place override of the path-shortening variant (D-06). `None` means
    /// «Как у родителя» — the effective variant is resolved dynamically by the
    /// `place_effective_variant` SQL view (V039), never here.
    pub path_variant_override: Option<PathDisplayVariant>,
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
    /// Short path per the effective D-06 variant (D-17) — symmetric with the
    /// device/cartridge list `place_path_short` field. Not `Option`, mirroring
    /// `full_path`'s own non-nullability here: `shorten_place_path` on a
    /// non-empty `full_path` deterministically returns a non-empty string.
    pub place_path_short: String,
    pub status_name: Option<String>,
}

/// Default sibling ordering (D-05): `sort_order` wins if set, else `level` (floors,
/// including 0 and negatives — PLC-02) wins if set, else fall back to natural name
/// comparison.
///
/// **quick 260827-rzq fix:** the previous implementation only compared `sort_order`
/// (or `level`) when BOTH siblings had it set — a pair where only one side had a value
/// fell straight through to the next stage, silently skipping it. That means different
/// pairs in the same slice were being ordered by different rules depending on what
/// happened to be filled in on each side (exactly the shape produced by drag-and-drop
/// reordering, which sets `sort_order` only on the moved nodes). A comparator that
/// applies a different rule to different pairs is not a total order — transitivity can
/// break on mixed slices, and since Rust 1.81 `slice::sort_by` detects that and panics
/// with "user-provided comparison function does not correctly implement a total
/// order". That panic (no `CatchPanicLayer` in this app) is what surfaced in
/// production as `ERR_EMPTY_RESPONSE` on `places_list_all`/`places_list_children`.
///
/// The fix: every pair goes through the SAME three-stage chain, and every stage
/// explicitly decides Some-vs-None instead of skipping when only one side has a value.
/// Convention: a node WITH a value at a given stage sorts BEFORE a node without one —
/// this matches D-05 ("manual order if set, else automatic") by making manually
/// (drag-and-drop-)positioned nodes visibly take priority over naturally-ordered ones.
/// This intentionally changes ordering for mixed sibling sets versus the old
/// (non-deterministic) behavior — a deliberate decision, not a regression.
pub fn sibling_cmp(a: &PlaceRow, b: &PlaceRow) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    match (a.sort_order, b.sort_order) {
        (Some(sa), Some(sb)) => match sa.cmp(&sb) {
            Ordering::Equal => {}
            other => return other,
        },
        (Some(_), None) => return Ordering::Less,
        (None, Some(_)) => return Ordering::Greater,
        (None, None) => {}
    }

    match (a.level, b.level) {
        (Some(la), Some(lb)) => match la.cmp(&lb) {
            Ordering::Equal => {}
            other => return other,
        },
        (Some(_), None) => return Ordering::Less,
        (None, Some(_)) => return Ordering::Greater,
        (None, None) => {}
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

/// The three closed path-display-variant values (D-06, Phase 39.1). Not a freeform
/// string — an unrecognized token is rejected with `AppError::Validation` at the
/// domain boundary, mirroring `PlaceKind::from_str`/`as_str` above.
///
/// **Token note:** the two carried-over tokens `"ends"`/`"last_two"` keep their old
/// names from the removed `PlacePathDisplay` config enum; only the old `"full"` token
/// is retired and replaced by `"last"` — a SEMANTICALLY OPPOSITE meaning (old `full` =
/// "never shorten, show the whole path"; new `last` = "show ONLY the final segment").
/// Do not treat `last` as a synonym for the old `full` when reading historical code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathDisplayVariant {
    /// «Крайние» — first + last segment, joined by `sep_ends`.
    Ends,
    /// «Два последних» — last two segments, joined by `sep_last_two`.
    LastTwo,
    /// «Последнее» — only the final segment.
    Last,
}

impl PathDisplayVariant {
    /// Parse from the DB/app_settings token. Returns `AppError::Validation` for
    /// unknown values, with a Russian-language message listing all three permitted
    /// values. The old `"full"` token is deliberately NOT accepted here (D-Open Q3) —
    /// it is a different, retired variant, not an alias for `"last"`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "ends" => Ok(Self::Ends),
            "last_two" => Ok(Self::LastTwo),
            "last" => Ok(Self::Last),
            other => Err(AppError::Validation {
                field: "path_variant".to_string(),
                message: format!(
                    "Неизвестный вариант сокращения пути: «{other}». Допустимые \
                     значения: ends, last_two, last."
                ),
            }),
        }
    }

    /// Returns the DB token corresponding to this variant (inverse of `from_str`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ends => "ends",
            Self::LastTwo => "last_two",
            Self::Last => "last",
        }
    }
}

/// Shorten a `' / '`-joined full place path (as produced by the `place_full_paths`
/// SQL view) according to `variant`, using the given organization-wide separators.
///
/// Pure string transform — does NOT read the database, does NOT walk the place tree.
/// Tree-walking for inheritance lives exclusively in the `place_effective_variant` SQL
/// view (V039); callers resolve `variant` there first, then call this function once on
/// the already-resolved full-path string. Never call this in a loop over ancestors.
///
/// D-13/D-14: the variant's separator (`sep_ends`/`sep_last_two`) is used ONLY when it
/// actually stands in for something that was dropped from the path. A 2-segment path
/// under `Ends`/`LastTwo` has nothing to drop, so it is returned unchanged, joined by
/// the ordinary `' / '` — never by `sep_ends`/`sep_last_two`. D-15: `Last` on a
/// single-segment path returns that segment unchanged, with no shortening marker.
pub fn shorten_place_path(
    full_path: &str,
    variant: PathDisplayVariant,
    sep_ends: &str,
    sep_last_two: &str,
) -> String {
    if full_path.is_empty() {
        return full_path.to_string();
    }

    let segments: Vec<&str> = full_path.split(" / ").collect();

    match segments.len() {
        0 => full_path.to_string(),
        1 => segments[0].to_string(),
        2 => match variant {
            // Nothing to drop on a 2-segment path — D-14 keeps the ordinary
            // ' / ' join for Ends/LastTwo; only Last narrows to one segment.
            PathDisplayVariant::Ends | PathDisplayVariant::LastTwo => full_path.to_string(),
            PathDisplayVariant::Last => segments[1].to_string(),
        },
        n => match variant {
            PathDisplayVariant::Ends => format!("{}{sep_ends}{}", segments[0], segments[n - 1]),
            PathDisplayVariant::LastTwo => {
                format!("{}{sep_last_two}{}", segments[n - 2], segments[n - 1])
            }
            PathDisplayVariant::Last => segments[n - 1].to_string(),
        },
    }
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
            path_variant_override: None,
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

    // quick 260827-rzq: sibling_cmp must be a genuine total order (Rust 1.81+
    // `sort_by` panics otherwise), and natural_name_cmp must be one too.

    #[test]
    fn sibling_cmp_is_a_total_order_exhaustive() {
        // Cartesian product: sort_order x level x name (36 rows), all with distinct ids
        // so reflexivity/antisymmetry/transitivity are checked across every combination
        // of "which stage has a value" — exactly the shape that broke transitivity
        // before the fix (mixed Some/None across sort_order/level).
        let sort_orders: [Option<i64>; 3] = [None, Some(0), Some(1)];
        let levels: [Option<i64>; 4] = [None, Some(-1), Some(0), Some(1)];
        let names = ["2", "10", "Zzz"];

        let mut rows = Vec::new();
        let mut next_id = 1i64;
        for so in sort_orders {
            for lvl in levels {
                for name in names {
                    rows.push(place(next_id, lvl, so, name));
                    next_id += 1;
                }
            }
        }
        assert_eq!(rows.len(), 36);

        // Reflexivity.
        for a in &rows {
            assert_eq!(
                sibling_cmp(a, a),
                std::cmp::Ordering::Equal,
                "reflexivity failed for {a:?}"
            );
        }

        // Antisymmetry.
        for a in &rows {
            for b in &rows {
                assert_eq!(
                    sibling_cmp(a, b),
                    sibling_cmp(b, a).reverse(),
                    "antisymmetry failed for a={a:?} b={b:?}"
                );
            }
        }

        // Transitivity: if a<=b and b<=c then a<=c (using != Greater as "<=").
        for a in &rows {
            for b in &rows {
                if sibling_cmp(a, b) == std::cmp::Ordering::Greater {
                    continue;
                }
                for c in &rows {
                    if sibling_cmp(b, c) == std::cmp::Ordering::Greater {
                        continue;
                    }
                    assert_ne!(
                        sibling_cmp(a, c),
                        std::cmp::Ordering::Greater,
                        "transitivity failed for a={a:?} b={b:?} c={c:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn sibling_cmp_sorts_partial_sort_order_slice_without_panicking_case_c() {
        // Regression for the production panic: >=60 rows with PARTIAL sort_order
        // (some Some, some None) whose values contradict both name and level order —
        // this is exactly the shape produced by drag-and-drop reordering a subset of
        // siblings. Before the fix, `sort_by(sibling_cmp)` panicked here on Rust 1.81+
        // ("user-provided comparison function does not correctly implement a total
        // order"); this test asserts it merely completes and yields a non-decreasing
        // order.
        let mut rows = Vec::new();
        for i in 0..60i64 {
            // Every 3rd row gets a manual sort_order that runs in REVERSE of insertion
            // order (contradicts name/level); the rest are None (natural/level order).
            let sort_order = if i % 3 == 0 { Some(60 - i) } else { None };
            // Level contradicts name order too, and dips negative (PLC-02 coverage).
            let level = Some((i % 5) - 2);
            let name = format!("Каб. {}", 60 - i);
            rows.push(place(i, level, sort_order, &name));
        }
        assert!(rows.len() >= 60);

        rows.sort_by(sibling_cmp); // must not panic

        for w in rows.windows(2) {
            assert_ne!(
                sibling_cmp(&w[0], &w[1]),
                std::cmp::Ordering::Greater,
                "non-decreasing order violated between {:?} and {:?}",
                w[0],
                w[1]
            );
        }
    }

    #[test]
    fn natural_name_cmp_is_a_total_order() {
        let names = [
            "",
            "2",
            "10",
            "Каб. 2",
            "Каб. 10",
            "Кабинет",
            "Zzz",
            "Ааа",
            "10 этаж",
            "2 этаж",
        ];

        for a in names {
            assert_eq!(
                natural_name_cmp(a, a),
                std::cmp::Ordering::Equal,
                "reflexivity failed for {a:?}"
            );
        }

        for a in names {
            for b in names {
                assert_eq!(
                    natural_name_cmp(a, b),
                    natural_name_cmp(b, a).reverse(),
                    "antisymmetry failed for a={a:?} b={b:?}"
                );
            }
        }

        for a in names {
            for b in names {
                if natural_name_cmp(a, b) == std::cmp::Ordering::Greater {
                    continue;
                }
                for c in names {
                    if natural_name_cmp(b, c) == std::cmp::Ordering::Greater {
                        continue;
                    }
                    assert_ne!(
                        natural_name_cmp(a, c),
                        std::cmp::Ordering::Greater,
                        "transitivity failed for a={a:?} b={b:?} c={c:?}"
                    );
                }
            }
        }
    }

    // PathDisplayVariant::from_str / as_str

    #[test]
    fn path_display_variant_from_str_full_is_rejected() {
        // "full" is the retired old-config token — it must NOT be accepted as a
        // synonym for the new "last" variant (semantically opposite meanings).
        let err = PathDisplayVariant::from_str("full").expect_err("должна быть ошибка");
        match err {
            AppError::Validation { field, .. } => assert_eq!(field, "path_variant"),
            other => panic!("ожидали AppError::Validation, получили {other:?}"),
        }
    }

    #[test]
    fn path_display_variant_from_str_last_is_accepted() {
        assert_eq!(
            PathDisplayVariant::from_str("last").unwrap(),
            PathDisplayVariant::Last
        );
    }

    #[test]
    fn path_display_variant_from_str_unknown_lists_all_three_values_in_russian() {
        let err = PathDisplayVariant::from_str("bogus").expect_err("должна быть ошибка");
        match err {
            AppError::Validation { field, message } => {
                assert_eq!(field, "path_variant");
                for token in ["ends", "last_two", "last"] {
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
    fn path_display_variant_as_str_roundtrips_all_three_variants() {
        for variant in [
            PathDisplayVariant::Ends,
            PathDisplayVariant::LastTwo,
            PathDisplayVariant::Last,
        ] {
            let s = variant.as_str();
            assert_eq!(PathDisplayVariant::from_str(s).unwrap(), variant);
        }
    }

    // shorten_place_path — full "input -> output" table from RESEARCH.md § Pattern 3
    // (D-13..D-16). Uses the D-09 default separators (' // ' / ' / ') unless a test
    // specifically exercises D-09/D-10 whitespace significance.

    const SEP_ENDS: &str = " // ";
    const SEP_LAST_TWO: &str = " / ";

    #[test]
    fn shorten_empty_input_returns_empty() {
        assert_eq!(
            shorten_place_path("", PathDisplayVariant::Ends, SEP_ENDS, SEP_LAST_TWO),
            ""
        );
    }

    #[test]
    fn shorten_one_segment_any_variant_returns_it_unchanged() {
        for variant in [
            PathDisplayVariant::Ends,
            PathDisplayVariant::LastTwo,
            PathDisplayVariant::Last,
        ] {
            assert_eq!(
                shorten_place_path("Склад", variant, SEP_ENDS, SEP_LAST_TWO),
                "Склад"
            );
        }
    }

    #[test]
    fn shorten_two_segments_ends_keeps_ordinary_separator_d14() {
        assert_eq!(
            shorten_place_path(
                "Здание А / 1 этаж",
                PathDisplayVariant::Ends,
                SEP_ENDS,
                SEP_LAST_TWO
            ),
            "Здание А / 1 этаж"
        );
    }

    #[test]
    fn shorten_two_segments_last_two_keeps_ordinary_separator_d14() {
        assert_eq!(
            shorten_place_path(
                "Здание А / 1 этаж",
                PathDisplayVariant::LastTwo,
                SEP_ENDS,
                SEP_LAST_TWO
            ),
            "Здание А / 1 этаж"
        );
    }

    #[test]
    fn shorten_two_segments_last_returns_only_last_segment() {
        assert_eq!(
            shorten_place_path(
                "Здание А / 1 этаж",
                PathDisplayVariant::Last,
                SEP_ENDS,
                SEP_LAST_TWO
            ),
            "1 этаж"
        );
    }

    #[test]
    fn shorten_three_segments_ends_uses_sep_ends_d16() {
        assert_eq!(
            shorten_place_path(
                "Здание А / 1 этаж / 1-05",
                PathDisplayVariant::Ends,
                SEP_ENDS,
                SEP_LAST_TWO
            ),
            "Здание А // 1-05"
        );
    }

    #[test]
    fn shorten_three_segments_last_two_uses_sep_last_two_d16() {
        assert_eq!(
            shorten_place_path(
                "Здание А / 1 этаж / 1-05",
                PathDisplayVariant::LastTwo,
                SEP_ENDS,
                SEP_LAST_TWO
            ),
            "1 этаж / 1-05"
        );
    }

    #[test]
    fn shorten_three_segments_last_returns_only_last_segment() {
        assert_eq!(
            shorten_place_path(
                "Здание А / 1 этаж / 1-05",
                PathDisplayVariant::Last,
                SEP_ENDS,
                SEP_LAST_TWO
            ),
            "1-05"
        );
    }

    #[test]
    fn shorten_four_plus_segments_ends_uses_first_and_last() {
        assert_eq!(
            shorten_place_path(
                "Территория А / Объект Х / Здание 1 / помещение 3",
                PathDisplayVariant::Ends,
                SEP_ENDS,
                SEP_LAST_TWO
            ),
            "Территория А // помещение 3"
        );
    }

    #[test]
    fn shorten_four_plus_segments_last_two_uses_last_two() {
        assert_eq!(
            shorten_place_path(
                "Территория А / Объект Х / Здание 1 / помещение 3",
                PathDisplayVariant::LastTwo,
                SEP_ENDS,
                SEP_LAST_TWO
            ),
            "Здание 1 / помещение 3"
        );
    }

    #[test]
    fn shorten_four_plus_segments_last_uses_only_last() {
        assert_eq!(
            shorten_place_path(
                "Территория А / Объект Х / Здание 1 / помещение 3",
                PathDisplayVariant::Last,
                SEP_ENDS,
                SEP_LAST_TWO
            ),
            "помещение 3"
        );
    }

    #[test]
    fn shorten_separator_whitespace_is_not_trimmed_d09_d10() {
        // A comma-space separator must survive verbatim through the function —
        // no .trim() anywhere on the sep_ends/sep_last_two path.
        assert_eq!(
            shorten_place_path(
                "Здание А / 1 этаж / 1-05",
                PathDisplayVariant::Ends,
                ", ",
                SEP_LAST_TWO
            ),
            "Здание А, 1-05"
        );
    }
}
