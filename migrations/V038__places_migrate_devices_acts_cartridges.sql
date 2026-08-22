-- V038: point devices/cartridges/acts at places, drop `locations`.
-- Schema-only migration — no data preserved (confirmed decision, see
-- 39-CONTEXT.md "Удаление и архивация" preamble / REQUIREMENTS.md Out of
-- Scope: app is pre-production, all acts in the DB are test data).
--
-- Ordering note: every object that references a soon-to-be-dropped column
-- (indexes on devices.location_id, FTS5 sync triggers on cartridges.location)
-- must be dropped BEFORE the ALTER TABLE ... DROP COLUMN that removes it.
-- SQLite's DROP COLUMN dependency check for triggers has been observed to
-- vary across SQLite point releases (verified empirically: the bundled CLI
-- silently allows it, a newer library version raises "no such column" the
-- first time the trigger fires) — dropping the dependents first sidesteps
-- the ambiguity entirely rather than relying on version-specific leniency.

ALTER TABLE devices    ADD COLUMN place_id INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT;
ALTER TABLE cartridges ADD COLUMN place_id INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT;
ALTER TABLE acts       ADD COLUMN place_id INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT;
ALTER TABLE acts       ADD COLUMN bulk_place_id INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT;
ALTER TABLE acts       ADD COLUMN place_path_snapshot TEXT NULL; -- D-16, act-level granularity
ALTER TABLE act_items  ADD COLUMN place_id_override INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT;

-- Drop dependents of devices.location_id before the column itself.
DROP INDEX idx_devices_location;
DROP INDEX idx_devices_autocomplete_name_location;

-- Drop dependents of cartridges.location before the column itself.
-- cartridges_fts previously indexed a freeform-text FTS5 column sourced
-- from `cartridges.location`. FTS5 external-content tables read the
-- backing content table by column name at query and rebuild time, so
-- leaving that column declared would make
-- `INSERT INTO cartridges_fts(cartridges_fts) VALUES('rebuild')` fail with
-- "no such column" once the source column is gone. The virtual table is
-- therefore dropped here and recreated further below without that column,
-- then rebuilt from the (now two-column) `cartridges` content table — this
-- both fixes the schema mismatch and purges any stale freeform-place
-- tokens from the index in one step. No place-related column is added to
-- devices_fts or the recreated FTS5 table below (place search resolves
-- live via `place_full_paths`, per RESEARCH Common Pitfall 1).
DROP TRIGGER cartridges_fts_ai;
DROP TRIGGER cartridges_fts_ad;
DROP TRIGGER cartridges_fts_au;
DROP TABLE cartridges_fts;

-- Old columns dropped. SQLite ALTER TABLE DROP COLUMN is supported since
-- 3.35.0 (2021) — comfortably inside rusqlite's bundled SQLite version.
ALTER TABLE devices    DROP COLUMN location_id;
ALTER TABLE cartridges DROP COLUMN location;
ALTER TABLE acts       DROP COLUMN location_id;
-- NOTE: acts.bulk_location_id / act_items.location_id_override were never
-- schema columns — they are Rust-only domain/DTO struct fields resolved
-- against `locations` at write time (verified: no migration ever created
-- them). Only the Rust struct fields need renaming; no DROP COLUMN needed.

CREATE INDEX idx_devices_place    ON devices(place_id)    WHERE deleted_at_utc IS NULL AND place_id IS NOT NULL;
CREATE INDEX idx_cartridges_place ON cartridges(place_id) WHERE deleted_at_utc IS NULL AND place_id IS NOT NULL;

DROP TABLE locations;  -- PLC-04

CREATE VIRTUAL TABLE cartridges_fts USING fts5(
  code, holder_name,
  content='cartridges', content_rowid='id',
  tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER cartridges_fts_ai
AFTER INSERT ON cartridges
WHEN NEW.deleted_at_utc IS NULL
BEGIN
  INSERT INTO cartridges_fts(rowid, code, holder_name)
  VALUES (NEW.id, NEW.code, NEW.holder_name);
END;

CREATE TRIGGER cartridges_fts_ad
AFTER DELETE ON cartridges
BEGIN
  INSERT INTO cartridges_fts(cartridges_fts, rowid, code, holder_name)
  VALUES ('delete', OLD.id, OLD.code, OLD.holder_name);
END;

CREATE TRIGGER cartridges_fts_au
AFTER UPDATE ON cartridges
BEGIN
  INSERT INTO cartridges_fts(cartridges_fts, rowid, code, holder_name)
  VALUES ('delete', OLD.id, OLD.code, OLD.holder_name);
  INSERT INTO cartridges_fts(rowid, code, holder_name)
  SELECT NEW.id, NEW.code, NEW.holder_name
  WHERE NEW.deleted_at_utc IS NULL;
END;

INSERT INTO cartridges_fts(cartridges_fts) VALUES('rebuild');

PRAGMA user_version = 38;
