-- V016: Cartridge kinds lookup + color column + app_settings + FTS triggers.
--
-- Execution order (must be strictly followed to avoid FK errors):
--   1. CREATE TABLE cartridge_kinds   (referenced by ALTER below)
--   2. ALTER TABLE cartridge_models   (ADD kind_id FK + color column)
--   3. CREATE TABLE app_settings      (+ seed low_stock_threshold)
--   4. FTS sync triggers              (cartridges_fts_ai / _ad / _au)
--   5. PRAGMA user_version = 16
--
-- Index note: idx_audit_log_entity ON audit_log(entity_type, entity_id, created_at_utc)
-- already exists from V012 and fully covers get_history queries
-- (WHERE entity_type='cartridge' AND entity_id=? ORDER BY created_at_utc DESC).
-- Creating it again here would produce "already exists" error — DO NOT duplicate.

-- 1. Lookup table: cartridge kinds (Картридж / Фотобарабан).
--    Pattern: V001 cartridge_statuses — hard-delete, no _at_utc / version columns.
CREATE TABLE cartridge_kinds (
  id   INTEGER PRIMARY KEY,
  name TEXT    NOT NULL UNIQUE
);
INSERT INTO cartridge_kinds (id, name) VALUES
  (1, 'Картридж'),
  (2, 'Фотобарабан');

-- 2. Extend cartridge_models with kind + color.
--    NOT NULL DEFAULT 1 is mandatory: SQLite refuses ADD COLUMN NOT NULL without DEFAULT
--    when existing rows are present (Pitfall 2 from RESEARCH.md).
ALTER TABLE cartridge_models
  ADD COLUMN kind_id INTEGER NOT NULL DEFAULT 1
    REFERENCES cartridge_kinds(id);

ALTER TABLE cartridge_models
  ADD COLUMN color TEXT NULL;

-- 3. Application settings table.
--    Supports D-LowStock-01 (CART-12): low_stock_threshold read by CartridgeService.
CREATE TABLE app_settings (
  key            TEXT    NOT NULL PRIMARY KEY,
  value          TEXT    NOT NULL,
  created_at_utc INTEGER NOT NULL,
  updated_at_utc INTEGER NOT NULL
);
INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc)
  VALUES ('low_stock_threshold', '2', unixepoch(), unixepoch());

-- 4. FTS sync triggers for cartridges_fts (external-content FTS5 from V012).
--    Pattern: V013 devices_fts_* triggers (exact analog).
--    FTS columns match V012 cartridges_fts definition: code, location, holder_name.
--    Migrations run once via refinery — no IF NOT EXISTS needed.

-- FTS sync: INSERT (only for live rows, not soft-deleted)
CREATE TRIGGER cartridges_fts_ai
AFTER INSERT ON cartridges
WHEN NEW.deleted_at_utc IS NULL
BEGIN
  INSERT INTO cartridges_fts(rowid, code, location, holder_name)
  VALUES (NEW.id, NEW.code, NEW.location, NEW.holder_name);
END;

-- FTS sync: DELETE (FTS5 external-content delete protocol)
CREATE TRIGGER cartridges_fts_ad
AFTER DELETE ON cartridges
BEGIN
  INSERT INTO cartridges_fts(cartridges_fts, rowid, code, location, holder_name)
  VALUES ('delete', OLD.id, OLD.code, OLD.location, OLD.holder_name);
END;

-- FTS sync: UPDATE (remove old entry, add new entry only if not soft-deleted)
CREATE TRIGGER cartridges_fts_au
AFTER UPDATE ON cartridges
BEGIN
  INSERT INTO cartridges_fts(cartridges_fts, rowid, code, location, holder_name)
  VALUES ('delete', OLD.id, OLD.code, OLD.location, OLD.holder_name);
  INSERT INTO cartridges_fts(rowid, code, location, holder_name)
  SELECT NEW.id, NEW.code, NEW.location, NEW.holder_name
  WHERE NEW.deleted_at_utc IS NULL;
END;

PRAGMA user_version = 16;
