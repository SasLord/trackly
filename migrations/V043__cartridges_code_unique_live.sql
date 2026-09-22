-- V043: `cartridges.code` uniqueness — global -> live-only (NUM-09 space "в").
--
-- Plan 07 (last of the three "rebuild+counter-removal" Wave 5 migrations,
-- after V042 for acts). Today `code TEXT NOT NULL UNIQUE` is a column-level
-- constraint that also blocks reusing a code after the cartridge/drum that
-- held it was soft-deleted. NUM-09 requires uniqueness to be scoped to LIVE
-- rows only, so a soft-deleted cartridge's code becomes free again.
--
-- SQLite has no `ALTER TABLE ... DROP CONSTRAINT`, so the standard 12-step
-- rebuild (V030 pattern) is used: create a replacement table without the
-- column-level UNIQUE, copy rows verbatim (no CAST — `code`'s type is
-- unchanged, only the constraint is), drop the old table, rename the new
-- one into place, then add a PARTIAL unique index that only sees live rows.
--
-- Full current column list confirmed by applying every migration V001..V042
-- to a scratch DB and running `PRAGMA table_info(cartridges)` (not
-- reconstructed by hand from scattered ALTER TABLEs — see V005, V025, V038):
--   id, code, model_id, status_id, state_id, holder_name, notes,
--   created_at_utc, updated_at_utc, deleted_at_utc, version,
--   current_printer_device_id, place_id.
--
-- Existing indexes on `cartridges` before this migration (`PRAGMA
-- index_list`): `idx_cartridges_place` (V038, partial), `idx_cartridges_model`
-- (V012, plain), plus the autoindex backing the UNIQUE constraint being
-- removed here. Both named indexes are recreated verbatim below.
--
-- Index note (D-08): `idx_cartridges_code_live` indexes `code` AS STORED
-- (case-preserved), NOT `LOWER(TRIM(code))`. The index exists purely to
-- defend against a byte-for-byte race at the DB layer; case-insensitive /
-- trimmed comparison for the human-facing "is this code occupied?" check is
-- the application layer's job (`cartridge_service.rs`, Task 3 of this plan),
-- run BEFORE the writer transaction opens, same pattern as
-- `NumberTemplateService::is_occupied`.
--
-- `DROP TABLE cartridges` also drops every trigger and index attached to it
-- (SQLite behavior — not a V030-style oversight): `cartridges_fts_ai/_ad/_au`
-- (V038) must be recreated after the rebuild, identical to their current
-- definitions, or FTS sync silently stops. The FTS5 virtual table itself
-- (`cartridges_fts`, external-content, `content_rowid='id'`) is untouched —
-- `id` values are preserved 1:1 by the explicit-column INSERT below, so no
-- `INSERT INTO cartridges_fts(cartridges_fts) VALUES('rebuild')` is needed.
--
-- FK note: only `requests.completed_cartridge_id` (V024/V031) references
-- `cartridges(id)`. SQLite resolves FKs by table name, not rowid/oid, so
-- renaming `cartridges_new` -> `cartridges` restores that FK transparently.
--
-- Foreign keys MUST be OFF while this file runs (BE-CR-02): with FKs ON,
-- `DROP TABLE cartridges` performs an implicit `DELETE` that fails
-- immediately on any `requests.completed_cartridge_id` reference (NO
-- ACTION). A `PRAGMA foreign_keys = OFF` line HERE is a no-op — refinery
-- runs each file inside a transaction and SQLite ignores the pragma there —
-- so the runner (`trackly_infra::db::migrations::run`) switches FKs off on
-- the connection before refinery starts and runs `PRAGMA foreign_key_check`
-- afterwards. The two pragma lines below are kept only as documentation.
--
-- AUTOINCREMENT note (BE-WR-11): same as V042 — the old `sqlite_sequence`
-- high-water mark is carried over to `cartridges_new` before the drop so
-- ids of physically deleted rows are never reissued (`main.` qualified,
-- see V042).

PRAGMA foreign_keys = OFF;

CREATE TABLE cartridges_new (
  id                          INTEGER PRIMARY KEY AUTOINCREMENT,
  code                        TEXT    NOT NULL,                      -- UNIQUE dropped: see idx_cartridges_code_live below
  model_id                    INTEGER NOT NULL REFERENCES cartridge_models(id),
  status_id                   INTEGER NOT NULL REFERENCES cartridge_statuses(id) DEFAULT 1,
  state_id                    INTEGER NULL REFERENCES cartridge_states(id),
  holder_name                 TEXT    NULL,
  notes                       TEXT    NULL,
  created_at_utc              INTEGER NOT NULL,
  updated_at_utc              INTEGER NOT NULL,
  deleted_at_utc              INTEGER NULL,
  version                     INTEGER NOT NULL DEFAULT 1,
  current_printer_device_id   INTEGER NULL REFERENCES devices(id),
  place_id                    INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT
);

INSERT INTO cartridges_new (
  id, code, model_id, status_id, state_id, holder_name, notes,
  created_at_utc, updated_at_utc, deleted_at_utc, version,
  current_printer_device_id, place_id
)
SELECT
  id, code, model_id, status_id, state_id, holder_name, notes,
  created_at_utc, updated_at_utc, deleted_at_utc, version,
  current_printer_device_id, place_id
FROM cartridges;

INSERT INTO main.sqlite_sequence (name, seq)
  SELECT 'cartridges_new', 0
   WHERE NOT EXISTS (SELECT 1 FROM main.sqlite_sequence WHERE name = 'cartridges_new');
UPDATE main.sqlite_sequence
   SET seq = MAX(seq, COALESCE((SELECT seq FROM main.sqlite_sequence WHERE name = 'cartridges'), 0))
 WHERE name = 'cartridges_new';

DROP TABLE cartridges;

ALTER TABLE cartridges_new RENAME TO cartridges;

-- Partial unique index: uniqueness scoped to LIVE (non-soft-deleted) rows only.
CREATE UNIQUE INDEX idx_cartridges_code_live ON cartridges(code) WHERE deleted_at_utc IS NULL;

-- Recreate the other two pre-existing indexes (dropped along with the table above).
CREATE INDEX idx_cartridges_model ON cartridges(model_id);
CREATE INDEX idx_cartridges_place ON cartridges(place_id) WHERE deleted_at_utc IS NULL AND place_id IS NOT NULL;

-- Recreate FTS sync triggers (dropped along with the table above) — verbatim from V038.
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

PRAGMA foreign_keys = ON;

PRAGMA user_version = 43;
