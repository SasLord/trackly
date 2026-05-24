-- V004: Acts (акты приёма-передачи) + act_items junction.
--
-- An "act" represents a handover or return. Returns reference their parent
-- handover act via `parent_act_id`; partial returns use the same `number`
-- but distinct `sub_number` (composite uniqueness enforced via the partial
-- unique index below).
--
-- `act_items` is a junction table: NO standard4 columns (per D-Schema-03).

CREATE TABLE acts (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  number            INTEGER NOT NULL,                              -- human-visible act number from V009 counter
  sub_number        INTEGER NULL,                                  -- NULL for original handover; set on partial returns
  parent_act_id     INTEGER NULL REFERENCES acts(id) ON DELETE RESTRICT,
  act_type          TEXT    NOT NULL CHECK (act_type IN ('handover', 'return')),
  giver_name        TEXT    NOT NULL,                              -- denormalised; users may not exist long-term
  receiver_name     TEXT    NOT NULL,
  location_id       INTEGER NULL REFERENCES locations(id),
  notes             TEXT    NULL,
  archived          INTEGER NOT NULL DEFAULT 0,                    -- 0/1 boolean
  created_at_utc    INTEGER NOT NULL,
  updated_at_utc    INTEGER NOT NULL,
  deleted_at_utc    INTEGER NULL,
  version           INTEGER NOT NULL DEFAULT 1
);

-- Unique (number, sub_number) among live (non-soft-deleted) acts.
-- NULL sub_number is considered distinct from any other NULL by SQLite, so we
-- coalesce to 0 in the index expression to enforce true uniqueness.
CREATE UNIQUE INDEX idx_acts_number_sub_unique
  ON acts(number, COALESCE(sub_number, 0))
  WHERE deleted_at_utc IS NULL;

CREATE TABLE act_items (
  id                       INTEGER PRIMARY KEY AUTOINCREMENT,
  act_id                   INTEGER NOT NULL REFERENCES acts(id) ON DELETE CASCADE,
  device_id                INTEGER NOT NULL REFERENCES devices(id),
  condition_at_time        TEXT    NULL,
  complectation_at_time    TEXT    NULL
);

PRAGMA user_version = 4;
