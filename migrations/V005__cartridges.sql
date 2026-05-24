-- V005: Cartridge models, model-printer compatibility, cartridges.
--
-- `cartridges.code` (human-visible C-XXXXXX) is populated from V009's
-- `cartridge_seq` counter at INSERT time by the service layer; not a
-- generated column to keep the format change flexible.
--
-- `cartridge_model_compatibility` is a junction table — NO standard4.

CREATE TABLE cartridge_models (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  brand           TEXT    NOT NULL,
  model           TEXT    NOT NULL,
  notes           TEXT    NULL,
  created_at_utc  INTEGER NOT NULL,
  updated_at_utc  INTEGER NOT NULL,
  deleted_at_utc  INTEGER NULL,
  version         INTEGER NOT NULL DEFAULT 1
);

-- Unique (brand, model) among live (non-soft-deleted) rows.
CREATE UNIQUE INDEX idx_cartridge_models_brand_model_unique
  ON cartridge_models(brand, model)
  WHERE deleted_at_utc IS NULL;

CREATE TABLE cartridge_model_compatibility (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  cartridge_model_id  INTEGER NOT NULL REFERENCES cartridge_models(id) ON DELETE CASCADE,
  printer_brand       TEXT    NOT NULL,
  printer_model       TEXT    NOT NULL
);

CREATE TABLE cartridges (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  code            TEXT    NOT NULL UNIQUE,                         -- e.g. 'C-000001' from cartridge_seq counter
  model_id        INTEGER NOT NULL REFERENCES cartridge_models(id),
  status_id       INTEGER NOT NULL REFERENCES cartridge_statuses(id) DEFAULT 1,
  state_id        INTEGER NULL REFERENCES cartridge_states(id),
  location        TEXT    NULL,                                    -- freeform; locations table is for devices
  holder_name     TEXT    NULL,                                    -- denormalised current holder
  notes           TEXT    NULL,
  created_at_utc  INTEGER NOT NULL,
  updated_at_utc  INTEGER NOT NULL,
  deleted_at_utc  INTEGER NULL,
  version         INTEGER NOT NULL DEFAULT 1
);

PRAGMA user_version = 5;
