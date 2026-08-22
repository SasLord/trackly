-- V037: places tree (adjacency list). No data migration from `locations`
-- is required (confirmed: app not in production, REQUIREMENTS.md Out of Scope).

CREATE TABLE places (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  parent_id       INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT,
  kind            TEXT    NOT NULL
                          CHECK (kind IN ('territory','zone','building','floor','room','outdoor')),
  name            TEXT    NOT NULL,
  level           INTEGER NULL,               -- floors only; NULL for other kinds (PLC-02: 0 and negatives OK)
  is_storage      INTEGER NOT NULL DEFAULT 0,  -- 0/1 boolean (D-08)
  sort_order      INTEGER NULL,                -- manual override (D-05); NULL = automatic
  archived_at_utc INTEGER NULL,                -- D-15: archived (hidden from PlacePicker), distinct from soft-delete
  notes           TEXT    NULL,
  created_at_utc  INTEGER NOT NULL,
  updated_at_utc  INTEGER NOT NULL,
  deleted_at_utc  INTEGER NULL,                -- hard-delete only allowed when empty (D-14), still soft-delete column per standard4
  version         INTEGER NOT NULL DEFAULT 1
);

-- D-04: unique name among live siblings. COALESCE(parent_id, 0) mirrors the
-- existing idx_acts_number_sub_unique pattern for nullable-column uniqueness.
CREATE UNIQUE INDEX idx_places_parent_name_unique
  ON places(COALESCE(parent_id, 0), name)
  WHERE deleted_at_utc IS NULL;

CREATE INDEX idx_places_parent ON places(parent_id) WHERE deleted_at_utc IS NULL;

-- Full path per node, from root to leaf, ' / '-joined (PLC-03 canonical format).
-- Recomputed on every query — cheap at ~300 rows, and crucially NEVER stale
-- (no cache to invalidate — this is what makes D-29/PLC-05 "instant, no
-- reindex" true by construction, not by trigger correctness).
CREATE VIEW place_full_paths AS
WITH RECURSIVE path_cte(id, path, parent_id) AS (
  SELECT id, name, parent_id FROM places WHERE deleted_at_utc IS NULL
  UNION ALL
  SELECT pc.id, p.name || ' / ' || pc.path, p.parent_id
  FROM path_cte pc
  JOIN places p ON p.id = pc.parent_id
  WHERE p.deleted_at_utc IS NULL
)
SELECT id AS place_id, path AS full_path
FROM path_cte
WHERE parent_id IS NULL;

PRAGMA user_version = 37;
