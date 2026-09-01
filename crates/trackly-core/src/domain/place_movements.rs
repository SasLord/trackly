//! Domain value types for Phase 40's movement-history journal.
//!
//! Pure domain layer — no I/O dependencies (enforced by `tests/no_io_deps.rs`), mirrors
//! the `domain::places` boundary convention (crate-local `PlaceKind`/`PathDisplayVariant`).
//!
//! `MovementSource` and `MovementEntityKind` back the `place_movements.source` /
//! `entity_type` columns (migration V040), which are deliberately UNCONSTRAINED TEXT in
//! SQL (no `CHECK`, Pitfall 6 / IN-01). Unlike `PlaceKind::from_str` (strict `Result`,
//! used at write time where the value is always server-controlled and valid), both enums
//! here expose only a lenient `from_str_lenient(&str) -> Option<Self>` — the read-side
//! soft-degradation entry point every history read site must call: an unrecognized token
//! degrades to `None` (safe fallback label), it never panics or errors.

/// The four movement-source tokens (D-07). `Map` and `Workstation` are not written by any
/// write-site that exists yet in this phase — they are reserved for later plans (map
/// drag-and-drop moves, workstation assignment moves) so the column never needs a schema
/// change when those land.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementSource {
    /// Ручное перемещение (форма "Переместить").
    Manual,
    /// Перемещение как побочный эффект акта приёма-передачи.
    Act,
    /// Перемещение через drag-and-drop на карте (Phase 42+).
    Map,
    /// Перемещение как часть назначения/переназначения АРМ (Phase 41+).
    Workstation,
}

impl MovementSource {
    /// Returns the DB token corresponding to this source (inverse of `from_str_lenient`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Act => "act",
            Self::Map => "map",
            Self::Workstation => "workstation",
        }
    }

    /// Parse from the DB token. Returns `None` for unknown values — never panics, never
    /// errors. This is the read-side soft-degrade entry point (Pitfall 6 / IN-01): a
    /// value written by a future binary version that this binary doesn't recognize yet
    /// must not crash the history screen, it should just fall back to a safe label.
    pub fn from_str_lenient(s: &str) -> Option<Self> {
        match s {
            "manual" => Some(Self::Manual),
            "act" => Some(Self::Act),
            "map" => Some(Self::Map),
            "workstation" => Some(Self::Workstation),
            _ => None,
        }
    }
}

/// The two movement entity-kind tokens (D-21). A printer is stored as `Device` — it has
/// no separate `entity_type` token of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementEntityKind {
    Device,
    Cartridge,
}

impl MovementEntityKind {
    /// Returns the DB token corresponding to this kind (inverse of `from_str_lenient`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Device => "device",
            Self::Cartridge => "cartridge",
        }
    }

    /// Russian label for the HST-04 report's «Тип» column.
    pub fn label_ru(&self) -> &'static str {
        match self {
            Self::Device => "Устройство",
            Self::Cartridge => "Картридж",
        }
    }

    /// Parse from the DB token. Returns `None` for unknown values (D-21: "printer" is
    /// never a stored `entity_type` — only "device"/"cartridge" are valid tokens; a
    /// printer's movements are recorded as `Device`).
    pub fn from_str_lenient(s: &str) -> Option<Self> {
        match s {
            "device" => Some(Self::Device),
            "cartridge" => Some(Self::Cartridge),
            _ => None,
        }
    }
}

/// D-04/D-06 guard: is a place change from `before` to `after` worth recording as a
/// history row?
///
/// - Both `Some` and different → `true` (a real move between two known places).
/// - Both `Some` and equal → `false` (D-04: no actual change, e.g. a no-op edit).
/// - Either side `None` → `false` (D-06: first assignment `NULL -> place` and clearing
///   `place -> NULL` are not "movements" — there is no "from" or "to" place to record).
pub fn is_reportable_place_change(before: Option<i64>, after: Option<i64>) -> bool {
    matches!((before, after), (Some(b), Some(a)) if b != a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_source_as_str_round_trips_all_variants() {
        assert_eq!(MovementSource::Manual.as_str(), "manual");
        assert_eq!(MovementSource::Act.as_str(), "act");
        assert_eq!(MovementSource::Map.as_str(), "map");
        assert_eq!(MovementSource::Workstation.as_str(), "workstation");
    }

    #[test]
    fn movement_source_from_str_lenient_parses_known_tokens() {
        assert_eq!(
            MovementSource::from_str_lenient("manual"),
            Some(MovementSource::Manual)
        );
        assert_eq!(
            MovementSource::from_str_lenient("act"),
            Some(MovementSource::Act)
        );
        assert_eq!(
            MovementSource::from_str_lenient("map"),
            Some(MovementSource::Map)
        );
        assert_eq!(
            MovementSource::from_str_lenient("workstation"),
            Some(MovementSource::Workstation)
        );
    }

    #[test]
    fn movement_source_from_str_lenient_returns_none_for_garbage() {
        assert_eq!(MovementSource::from_str_lenient("garbage"), None);
    }

    #[test]
    fn movement_entity_kind_as_str_and_label_ru() {
        assert_eq!(MovementEntityKind::Device.as_str(), "device");
        assert_eq!(MovementEntityKind::Cartridge.as_str(), "cartridge");
        assert_eq!(MovementEntityKind::Device.label_ru(), "Устройство");
        assert_eq!(MovementEntityKind::Cartridge.label_ru(), "Картридж");
    }

    #[test]
    fn movement_entity_kind_from_str_lenient_rejects_printer() {
        // D-21: printer is never a stored entity_type — only device/cartridge.
        assert_eq!(MovementEntityKind::from_str_lenient("printer"), None);
        assert_eq!(
            MovementEntityKind::from_str_lenient("device"),
            Some(MovementEntityKind::Device)
        );
        assert_eq!(
            MovementEntityKind::from_str_lenient("cartridge"),
            Some(MovementEntityKind::Cartridge)
        );
    }

    #[test]
    fn is_reportable_place_change_true_when_both_some_and_differ() {
        assert!(is_reportable_place_change(Some(1), Some(2)));
    }

    #[test]
    fn is_reportable_place_change_false_when_both_some_and_equal() {
        // D-04: no real change.
        assert!(!is_reportable_place_change(Some(1), Some(1)));
    }

    #[test]
    fn is_reportable_place_change_false_on_first_assignment() {
        // D-06: NULL -> place is a first assignment, not a movement.
        assert!(!is_reportable_place_change(None, Some(2)));
    }

    #[test]
    fn is_reportable_place_change_false_on_place_cleared() {
        // D-06: place -> NULL is a clear, not a movement.
        assert!(!is_reportable_place_change(Some(1), None));
    }

    #[test]
    fn is_reportable_place_change_false_when_both_none() {
        assert!(!is_reportable_place_change(None, None));
    }
}
