-- V042: acts.number → TEXT (NUM-14).
--
-- Free-text/templated act numbers (NUM-13/NUM-14, Phase 40.2) can no longer
-- be represented by an INTEGER column — a rendered template like
-- "2026/0009" or a hand-typed value like "42а" is not a valid integer.
-- SQLite has no `ALTER TABLE ... ALTER COLUMN`, so widening the type is done
-- via the standard 12-step table-rebuild pattern already used by
-- `V030__printers_drop_connectivity_check.sql`: create a replacement table,
-- copy rows across (casting the one changed column), drop the old table,
-- rename the replacement into place, and manually recreate anything that
-- gets dropped along with the old table (indexes are NOT renamed with their
-- table — they must be recreated explicitly).
--
-- Data preservation: `CAST(number AS TEXT)` on every existing INTEGER value
-- round-trips byte-for-byte (e.g. `42` -> `"42"`, `1` -> `"1"`) — no act
-- loses or changes its visible number across this migration. `id` is copied
-- verbatim (not re-assigned), so every foreign key pointing at an act row
-- (`act_items.act_id`, `place_movements.act_id`, and this table's own
-- self-referencing `parent_act_id`) keeps resolving to the exact same
-- logical row after the rename below.
--
-- FK integrity note (mirrors V030's doc-comment): SQLite resolves foreign
-- keys BY TABLE NAME at check time, not by an internal rowid/oid captured
-- at CREATE TABLE time. `act_items` and `place_movements` declare
-- `REFERENCES acts(id)` — once `acts_new` is renamed to `acts` below, those
-- constraints transparently point at the rebuilt table without any changes
-- to `act_items`/`place_movements` themselves. This table's own
-- self-referencing `parent_act_id` column is declared below as
-- `REFERENCES acts(id)` (the FINAL name, not `acts_new`) for the same
-- reason — no post-rename rewrite is needed. `PRAGMA foreign_keys = OFF`
-- is set for the whole migration (refinery runs one file per transaction,
-- so this window never overlaps user traffic) precisely so that the
-- momentary self-reference to a not-yet-renamed "acts" causes no premature
-- validation error while `acts_new` and the old `acts` briefly coexist.
--
-- Unique index note (D-06): `idx_acts_number_sub_unique` enforces
-- uniqueness of the RAW `number` column (plus `sub_number`) among live rows
-- — this is a low-level DB invariant on the stored column only. The
-- broader requirement that a NEW act number must also not collide with the
-- DISPLAYED number of a live RETURN act (which carries a "в"/"вN" suffix
-- computed at read time via `format_act_number`, never stored in a column)
-- cannot be expressed by a SQL index — it is implemented in application
-- code (`act_service.rs::is_act_number_occupied_including_returns`, Plan 06
-- Task 3), layered on top of this index as the final DB-level backstop
-- against a race between two concurrent writers (T-40.2-14).

PRAGMA foreign_keys = OFF;

CREATE TABLE acts_new (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  number              TEXT    NOT NULL,
  sub_number          INTEGER NULL,
  parent_act_id       INTEGER NULL REFERENCES acts(id) ON DELETE RESTRICT,
  act_type            TEXT    NOT NULL CHECK (act_type IN ('handover', 'return')),
  giver_name          TEXT    NOT NULL,
  receiver_name       TEXT    NOT NULL,
  notes               TEXT    NULL,
  archived            INTEGER NOT NULL DEFAULT 0,
  created_at_utc      INTEGER NOT NULL,
  updated_at_utc      INTEGER NOT NULL,
  deleted_at_utc      INTEGER NULL,
  version             INTEGER NOT NULL DEFAULT 1,
  deadline_utc        INTEGER NULL,
  handover_date_utc   INTEGER NOT NULL DEFAULT 0,
  place_id            INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT,
  bulk_place_id       INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT,
  place_path_snapshot TEXT NULL
);

INSERT INTO acts_new (
  id, number, sub_number, parent_act_id, act_type, giver_name, receiver_name,
  notes, archived, created_at_utc, updated_at_utc, deleted_at_utc, version,
  deadline_utc, handover_date_utc, place_id, bulk_place_id, place_path_snapshot
)
SELECT
  id, CAST(number AS TEXT), sub_number, parent_act_id, act_type, giver_name,
  receiver_name, notes, archived, created_at_utc, updated_at_utc,
  deleted_at_utc, version, deadline_utc, handover_date_utc, place_id,
  bulk_place_id, place_path_snapshot
FROM acts;

DROP TABLE acts;

ALTER TABLE acts_new RENAME TO acts;

-- Recreate every index that was dropped along with the old `acts` table
-- (indexes are NOT renamed with their table — V004/V012/V014 originals).
CREATE UNIQUE INDEX idx_acts_number_sub_unique
  ON acts(number, COALESCE(sub_number, 0))
  WHERE deleted_at_utc IS NULL;

CREATE INDEX idx_acts_parent ON acts(parent_act_id);

CREATE INDEX idx_acts_parent_act_id ON acts(parent_act_id) WHERE parent_act_id IS NOT NULL;

PRAGMA foreign_keys = ON;

PRAGMA user_version = 42;
