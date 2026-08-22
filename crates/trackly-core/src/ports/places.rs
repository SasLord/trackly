//! `PlaceRepository` port — repository trait for the Places entity (adjacency-list
//! location tree, Phase 39).
//!
//! Pattern: associated `type Conn` keeps rusqlite out of trackly-core. The concrete
//! type (`rusqlite::Connection`) is specified in the adapter impl in
//! `trackly-infra::repos::places_sqlite` (a later Phase 39 plan).
//!
//! Every mutating method (`create`, `rename`, `move_node`, `archive`, `unarchive`,
//! `delete_hard`) takes `&mut Self::Conn` — it writes. Every read method (`get`,
//! `list_children`, `list_all`, `subtree_stats`, `list_subtree_contents`,
//! `list_storage_place_ids`, `full_path`) takes `&Self::Conn` — it only reads.
//! This mirrors the existing `DeviceRepository` port's mut/non-mut split.
//!
//! `PlaceRepository` is the SOLE contract for creating/reading/renaming/moving/
//! archiving/deleting a place — unlike the freeform `locations` table's
//! `resolve_location_id_in_tx` helper it replaces, no method here auto-creates a
//! place by name. Place creation is always an explicit, Admin-gated call to
//! `create()` (D-18).

use crate::domain::places::{PlaceContentRow, PlaceNew, PlaceRow, SubtreeStats};
use crate::error::AppError;

/// Repository port for places. Implemented by `SqlitePlaceRepository` in trackly-infra.
///
/// `type Conn` is the connection type — kept generic so that trackly-core does not
/// take a hard dependency on rusqlite.
pub trait PlaceRepository {
    /// The connection type provided by the adapter (e.g. `rusqlite::Connection`).
    type Conn;

    /// Create a new place. Returns the new place's `id`.
    fn create(&self, conn: &mut Self::Conn, new: &PlaceNew, now_utc: i64) -> Result<i64, AppError>;

    /// Get a single place by ID. Returns `AppError::NotFound` if absent or soft-deleted.
    fn get(&self, conn: &Self::Conn, id: i64) -> Result<PlaceRow, AppError>;

    /// Direct children only, in DB order (caller applies natural sort — `sibling_cmp`,
    /// `domain::places::sibling_cmp`, Pattern 4). `parent_id: None` lists root nodes.
    fn list_children(&self, conn: &Self::Conn, parent_id: Option<i64>) -> Result<Vec<PlaceRow>, AppError>;

    /// Whole tree, flattened, for initial `PlacePicker`/tree-view hydration.
    fn list_all(&self, conn: &Self::Conn, include_archived: bool) -> Result<Vec<PlaceRow>, AppError>;

    /// Rename a place. Optimistic-lock CAS via `version` (mirrors `ActPatch`/`DevicePatch`
    /// pattern — reuse `expected_version` + `WHERE version = ?`, no new locking scheme).
    fn rename(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        name: &str,
        version: i64,
        now_utc: i64,
    ) -> Result<PlaceRow, AppError>;

    /// Move a place to a new parent (or to root, if `new_parent_id` is `None`).
    ///
    /// The implementation MUST run the Pattern 3 cycle check (39-RESEARCH.md) inside
    /// the same transaction as the `UPDATE`, before it executes: reject if
    /// `new_parent_id` is `id` itself, or a descendant of `id` — walking the
    /// `new_parent_id` ancestor chain via a recursive CTE and rejecting if `id`
    /// appears in it. Optimistic-lock CAS via `version`.
    fn move_node(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        new_parent_id: Option<i64>,
        version: i64,
        now_utc: i64,
    ) -> Result<PlaceRow, AppError>;

    /// Archive a place (soft, reversible — sets `archived_at_utc`). Optimistic-lock
    /// CAS via `version`.
    fn archive(&self, conn: &mut Self::Conn, id: i64, version: i64, now_utc: i64) -> Result<(), AppError>;

    /// Reverse `archive`. Optimistic-lock CAS via `version`.
    fn unarchive(&self, conn: &mut Self::Conn, id: i64, version: i64, now_utc: i64) -> Result<(), AppError>;

    /// Hard-delete a place (irreversible).
    ///
    /// The implementation MUST run the Pattern 2 subtree-stats query first and
    /// surface a conflict distinguishable from a plain not-found when the subtree
    /// is non-empty (D-14: block delete with exact counts) — the exact `AppError`
    /// variant used for that conflict is the implementing plan's choice; this port
    /// documents the contract (must reject non-empty subtrees, must be distinguishable
    /// from `NotFound`), not the enum variant.
    fn delete_hard(&self, conn: &mut Self::Conn, id: i64, version: i64) -> Result<(), AppError>;

    /// Subtree counts under `root_id`, inclusive of the root itself (Pattern 2 —
    /// shared by D-14 delete-block, D-21 consequences preview, D-25 tree counters,
    /// PLC-06 content screen).
    fn subtree_stats(&self, conn: &Self::Conn, root_id: i64) -> Result<SubtreeStats, AppError>;

    /// PLC-06 "content of place" listing: devices/printers/cartridges under `root_id`.
    /// `nested: true` includes the whole subtree (default per D-24); `nested: false`
    /// restricts to items whose `place_id` is exactly `root_id` ("Только здесь").
    fn list_subtree_contents(
        &self,
        conn: &Self::Conn,
        root_id: i64,
        nested: bool,
    ) -> Result<Vec<PlaceContentRow>, AppError>;

    /// All place IDs where `is_storage = true` on the node itself OR any ancestor
    /// (D-11.4).
    fn list_storage_place_ids(&self, conn: &Self::Conn) -> Result<Vec<i64>, AppError>;

    /// Resolve the root-to-leaf, `' / '`-joined full path of a place (via
    /// `place_full_paths`, always live — never cached).
    fn full_path(&self, conn: &Self::Conn, id: i64) -> Result<String, AppError>;
}
