-- V039: place-path display format (Phase 39.1, PLC-07/PLC-08).
--
-- Moves the "how much of a place's path to show" choice out of
-- `trackly.config.toml` (removed, D-22) and into the app: an organization-wide
-- default (`app_settings`) plus a per-place override that inherits from the
-- nearest ancestor override, or falls back to the organization default when no
-- ancestor (including the place itself) has one set (D-01..D-04).
--
-- Mirrors two existing conventions rather than inventing new ones:
--   - `migrations/V037__places.sql` (`place_full_paths`) for the recursive-CTE
--     view shape and the "recompute on every read, never cache" discipline
--     (D-02) — this migration's view is the upward-walk twin of that
--     downward-walk view.
--   - `migrations/V016__cartridges_kind_color_settings.sql` for the
--     `app_settings` key/value seeding pattern — that migration created the
--     `app_settings` table; this one only INSERTs new keys into it.
--
-- `path_variant_override` is intentionally NOT constrained by a SQLite CHECK
-- clause enumerating 'ends'/'last_two'/'last' — same choice already made for
-- `places.kind` (V037), which validates its token set in Rust
-- (`PlaceKind::from_str`) rather than in SQL. `PathDisplayVariant::from_str`
-- (crates/trackly-core/src/domain/places.rs) is the single source of truth
-- for the three permitted values; `NULL` means "Как у родителя" (D-06).

ALTER TABLE places ADD COLUMN path_variant_override TEXT NULL;

-- Organization-wide defaults (D-07..D-10). Mirrors V016's
-- `low_stock_threshold` seed literally (same table, same column shapes,
-- `unixepoch()` for both timestamp columns).
--
-- D-23: a freshly migrated database gets the organization default 'ends'
-- ("Крайние") and zero place-level overrides (every existing place keeps its
-- `path_variant_override` at NULL from the ALTER TABLE default above) — the
-- system behaves exactly as it did under the old config-file default.
--
-- D-09: separator defaults carry significant leading/trailing spaces
-- (' // ' and ' / ') — SQLite string literals are NOT trimmed, so these two
-- single-quoted literals must be typed with the spaces exactly as shown.
INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) VALUES
  ('place_path_variant',    'ends', unixepoch(), unixepoch()),
  ('place_path_sep_ends',   ' // ', unixepoch(), unixepoch()),
  ('place_path_sep_last_two', ' / ', unixepoch(), unixepoch());

-- Effective variant per place: walk UP the `parent_id` chain (the opposite
-- direction from `place_full_paths`, which walks down from root to leaf),
-- stopping at the first ancestor (including the place itself) that has a
-- non-NULL `path_variant_override`. If no ancestor in the chain (up to and
-- including a root node) has an override, fall back to the organization
-- default read from `app_settings` (D-02/D-23).
--
-- Recursion correctness: for a given starting place `id`, `walk` grows one
-- row per step while `w.variant IS NULL AND p.deleted_at_utc IS NULL` holds
-- (join to the next parent). It stops naturally by one of two conditions:
--   (a) an ancestor with a non-NULL override is reached — the recursive
--       member's `WHERE w.variant IS NULL` becomes false for that `id`, so no
--       further row is generated for it, or
--   (b) `w.parent_id` is NULL (root reached) — the JOIN to `places p` finds no
--       match, so recursion for that `id` stops on its own.
-- At every intermediate step the row just produced has `parent_id NOT NULL`
-- (we only just joined to a parent) AND `variant IS NULL` (otherwise
-- recursion would already have stopped) — the terminal predicate
-- `variant IS NOT NULL OR parent_id IS NULL` excludes exactly those
-- intermediate rows, leaving exactly one terminal row per starting `id`.
--
-- Filters `deleted_at_utc IS NULL` at every step, mirroring `place_full_paths`.
CREATE VIEW place_effective_variant AS
WITH RECURSIVE walk(id, parent_id, variant) AS (
  SELECT id, parent_id, path_variant_override
  FROM places WHERE deleted_at_utc IS NULL
  UNION ALL
  SELECT w.id, p.parent_id, p.path_variant_override
  FROM walk w
  JOIN places p ON p.id = w.parent_id
  WHERE w.variant IS NULL AND p.deleted_at_utc IS NULL
)
SELECT id AS place_id,
       COALESCE(
         variant,
         (SELECT value FROM app_settings WHERE key = 'place_path_variant')
       ) AS effective_variant
FROM walk
WHERE variant IS NOT NULL OR parent_id IS NULL;

PRAGMA user_version = 39;
