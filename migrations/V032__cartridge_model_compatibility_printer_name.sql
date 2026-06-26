-- V032: Collapse `cartridge_model_compatibility` printer_brand+printer_model
-- into a single free-text `printer_name` column, and DROP `printer_cartridge_models`
-- (V029) entirely (13-SPEC.md R1/R2 — Phase 13 compatibility redesign).
--
-- Rationale: Phase 13 redesigns Printer<->Cartridge compatibility to key off
-- a unique printer NAME/TYPE (devices.name), not a per-device junction table.
-- The V029 `printer_cartridge_models` junction (device_id <-> cartridge_model_id
-- by numeric FK) is superseded by matching `cartridge_model_compatibility
-- .printer_name` against `devices.name` (case-insensitive, TRIM'd) at query
-- time in `CartridgeRepository::list()`/`compatible_model_aggregates()`
-- (crates/trackly-infra/src/repos/cartridges_sqlite.rs). No replacement table
-- is needed for that junction — the V005 free-text table now does both jobs.
--
-- Step A — rebuild `cartridge_model_compatibility` (V005): drop the
-- printer_brand/printer_model TEXT pair, add a single printer_name TEXT
-- column. Existing rows are NOT lost — printer_name is populated from
-- TRIM(printer_brand || ' ' || printer_model) for every existing row (D-02).
--
-- Step B — DROP TABLE printer_cartridge_models (V029) along with its indexes
-- (idx_printer_cartridge_models_unique, idx_printer_cartridge_models_model
-- are dropped automatically by SQLite when the table is dropped — no
-- separate DROP INDEX needed). printer_cartridge_models references
-- devices(id), NOT printers(id) or cartridge_model_compatibility — its
-- removal does not intersect with Step A.
--
-- SQLite has no `ALTER TABLE ... DROP COLUMN` combined with a data transform
-- in one statement, so the only way to do this is the standard rebuild
-- pattern used by V030/V031: create a replacement table, copy+transform
-- rows, drop the old table, rename the new one into place. Both steps are
-- combined in this single file (one PRAGMA foreign_keys OFF/ON block,
-- refinery runs one file per transaction — set_grouped(false) — so this
-- window never overlaps user traffic).
--
-- This migration is safe to run on a DB where both V005 and V029 have
-- already been applied (the normal upgrade path) — idempotency of the full
-- migration chain is verified by the existing dynamic
-- db::migrations::tests / migration_idempotency.rs tests, which compute
-- max_known_version() at runtime and require no changes here.

PRAGMA foreign_keys = OFF;

CREATE TABLE cartridge_model_compatibility_new (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  cartridge_model_id  INTEGER NOT NULL REFERENCES cartridge_models(id) ON DELETE CASCADE,
  printer_name        TEXT    NOT NULL
);

-- Only migrate rows that produce a usable (non-empty) printer_name. A legacy
-- row with printer_brand='' AND printer_model='' collapses to TRIM(' ')='',
-- which would never match any devices.name and would silently suppress the
-- D-05 "no compatibility configured => compatible with any printer"
-- pass-through (CR-01). Dropping such empty rows restores that pass-through.
INSERT INTO cartridge_model_compatibility_new (id, cartridge_model_id, printer_name)
SELECT id, cartridge_model_id, TRIM(printer_brand || ' ' || printer_model)
  FROM cartridge_model_compatibility
 WHERE TRIM(printer_brand || ' ' || printer_model) <> '';

DROP TABLE cartridge_model_compatibility;

ALTER TABLE cartridge_model_compatibility_new RENAME TO cartridge_model_compatibility;

DROP TABLE printer_cartridge_models;

PRAGMA foreign_keys = ON;

PRAGMA user_version = 32;
