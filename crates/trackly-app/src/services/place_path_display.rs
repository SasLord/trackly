//! Single owner of `compute_place_path_short` — the path-shortening formula
//! used to render a `from`/`to` place path everywhere it appears cosmetically
//! shortened (acts today; Phase 40's timeline (Plan 40-10) and report
//! (Plan 40-11) import it from here, not a re-derived copy).
//!
//! Promoted out of `act_service.rs` (D-18, Phase 40 Plan 02) per
//! `40-RESEARCH.md` Open Question 3, to pre-empt the exact WR-08 duplication
//! anti-pattern (5 duplicated copies of an org-default formula) that Phase
//! 39.2 spent an entire plan eliminating. `compute_place_path_short` needs
//! `&ReaderPool` (an app-level type), so it cannot live in `trackly-core`
//! alongside `PathDisplayVariant`/`shorten_place_path` — that crate's
//! `no_io_deps.rs` boundary gate forbids I/O-capable types. It also does not
//! belong in `trackly_infra::repos::place_path_settings`, which stays
//! narrowly scoped to `&Connection`-level settings reads.

use rusqlite::{params, Connection};
use trackly_core::domain::places::{shorten_place_path, PathDisplayVariant};
use trackly_infra::db::pools::ReaderPool;
use trackly_infra::repos::place_path_settings::{
    read_org_default_variant_token, read_path_display_separators,
};

/// Shortens `snapshot` (the frozen `place_path_snapshot`, D-16) by the
/// CURRENT effective path-display variant for `place_id` (D-20, Phase 39.1
/// Plan 06) — never the variant at act-create time.
///
/// Resolution order (org-wide defaults and separators come from
/// `trackly_infra::repos::place_path_settings` — the single owner since
/// Phase 39.2 / WR-08; this function no longer keeps its own copy):
///   1. `snapshot` is `None`/absent → `None` (nothing to shorten — a
///      genuinely place-less act, existing D-27 blank-underline fallback).
///   2. `place_id` present AND `place_effective_variant` has a row for it →
///      use that row's `effective_variant`.
///   3. Otherwise (no `place_id`, OR `place_id` set but the place has since
///      disappeared — soft-deleted, Pitfall 4) → fall back to the
///      organization default via `read_org_default_variant_token`, which in
///      turn falls back to `DEFAULT_VARIANT` if even the setting is missing.
///
/// Entirely `Option`/`Result`-chained with `.ok()`/`.unwrap_or()` — no
/// `.expect()`/`.unwrap()`/`?` anywhere on this path. A printed act must
/// never fail to render because of this cosmetic field.
pub fn compute_place_path_short(
    readers: &ReaderPool,
    place_id: Option<i64>,
    snapshot: Option<String>,
) -> Option<String> {
    let snapshot = snapshot?;
    let conn = readers.acquire();
    compute_place_path_short_with_conn(&conn, place_id, Some(snapshot))
}

/// `&Connection` sibling of [`compute_place_path_short`] — for callers that
/// already hold a `Connection` (from a `ReaderPool::acquire()` done ONCE at
/// the top of their own read, e.g. `PlaceMovementService::get_timeline`,
/// `report_service.rs::query_movements_inner`) and must not take a SECOND
/// connection from the same pool inside a per-row loop (Phase 40 gap-closure
/// CR-01 — a nested `acquire()` on an exhausted pool blocks forever on an
/// untimed `Condvar`, risking a whole-app read deadlock under LAN
/// concurrency). Identical logic to `compute_place_path_short`, just without
/// the `readers.acquire()` step.
pub fn compute_place_path_short_with_conn(
    conn: &Connection,
    place_id: Option<i64>,
    snapshot: Option<String>,
) -> Option<String> {
    let snapshot = snapshot?;

    let variant_token: String = place_id
        .and_then(|pid| {
            conn.query_row(
                "SELECT effective_variant FROM place_effective_variant WHERE place_id = ?1",
                params![pid],
                |r| r.get::<_, String>(0),
            )
            .ok()
        })
        .unwrap_or_else(|| read_org_default_variant_token(conn));
    // Unexpected/corrupt token → fall back to Ends rather than dropping the
    // field-row entirely — this is a non-critical visual element.
    let variant = PathDisplayVariant::from_str(&variant_token).unwrap_or(PathDisplayVariant::Ends);

    let (sep_ends, sep_last_two) = read_path_display_separators(conn);

    Some(shorten_place_path(
        &snapshot,
        variant,
        &sep_ends,
        &sep_last_two,
    ))
}
