-- V013: FTS5 sync triggers + autocomplete partial indexes for devices.
--
-- Purpose: D-Schema-Phase2-01 + D-Autocomplete-01 from Phase 2 CONTEXT.md.
--
-- Three AFTER-triggers keep devices_fts (external-content FTS5 table from V012)
-- in sync with the live `devices` table on every INSERT / UPDATE / DELETE.
--
-- Column names match V003__devices.sql exactly (Path B from PATTERNS.md):
--   inventory_number, serial_number, condition, complectation, notes
-- The domain layer maps these to UI names (inventory_no, serial_no, state, kit, specs)
-- inside the DTO layer in trackly-app; SQL stays as V003.
--
-- Five partial indexes (WHERE deleted_at_utc IS NULL) support autocomplete
-- DISTINCT queries from DeviceService::autocomplete per D-Autocomplete-01.

-- FTS sync: INSERT
CREATE TRIGGER devices_fts_ai
AFTER INSERT ON devices
WHEN NEW.deleted_at_utc IS NULL
BEGIN
  INSERT INTO devices_fts(rowid, name, inventory_number, serial_number, model)
  VALUES (NEW.id, NEW.name, NEW.inventory_number, NEW.serial_number, NEW.model);
END;

-- FTS sync: DELETE
CREATE TRIGGER devices_fts_ad
AFTER DELETE ON devices
BEGIN
  INSERT INTO devices_fts(devices_fts, rowid, name, inventory_number, serial_number, model)
  VALUES ('delete', OLD.id, OLD.name, OLD.inventory_number, OLD.serial_number, OLD.model);
END;

-- FTS sync: UPDATE (remove old entry, add new entry only if not soft-deleted)
CREATE TRIGGER devices_fts_au
AFTER UPDATE ON devices
BEGIN
  INSERT INTO devices_fts(devices_fts, rowid, name, inventory_number, serial_number, model)
  VALUES ('delete', OLD.id, OLD.name, OLD.inventory_number, OLD.serial_number, OLD.model);
  INSERT INTO devices_fts(rowid, name, inventory_number, serial_number, model)
  SELECT NEW.id, NEW.name, NEW.inventory_number, NEW.serial_number, NEW.model
  WHERE NEW.deleted_at_utc IS NULL;
END;

-- Partial indexes for autocomplete DISTINCT queries (D-Autocomplete-01).
-- All filtered to WHERE deleted_at_utc IS NULL to skip soft-deleted devices.
CREATE INDEX idx_devices_autocomplete_name
  ON devices(name)
  WHERE deleted_at_utc IS NULL;

CREATE INDEX idx_devices_autocomplete_name_model
  ON devices(name, model)
  WHERE deleted_at_utc IS NULL AND model IS NOT NULL;

CREATE INDEX idx_devices_autocomplete_name_location
  ON devices(name, location_id)
  WHERE deleted_at_utc IS NULL AND location_id IS NOT NULL;

CREATE INDEX idx_devices_autocomplete_name_condition
  ON devices(name, condition)
  WHERE deleted_at_utc IS NULL AND condition IS NOT NULL;

CREATE INDEX idx_devices_autocomplete_name_complectation
  ON devices(name, complectation)
  WHERE deleted_at_utc IS NULL AND complectation IS NOT NULL;

PRAGMA user_version = 13;
