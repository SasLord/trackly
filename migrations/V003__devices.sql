-- V003: Devices — primary asset entity.
--
-- FKs: `type_id` → device_types, `status_id` → device_statuses,
-- `location_id` → locations. User-mutable: standard4 columns.

CREATE TABLE devices (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  type_id           INTEGER NOT NULL REFERENCES device_types(id),
  name              TEXT    NOT NULL,
  inventory_number  TEXT    NULL,
  serial_number     TEXT    NULL,
  model             TEXT    NULL,
  condition         TEXT    NULL,                                  -- состояние ("новое", "б/у", "под списание")
  complectation     TEXT    NULL,                                  -- комплектация (текстом)
  location_id       INTEGER NULL REFERENCES locations(id),
  status_id         INTEGER NOT NULL REFERENCES device_statuses(id) DEFAULT 1,
  notes             TEXT    NULL,
  created_at_utc    INTEGER NOT NULL,
  updated_at_utc    INTEGER NOT NULL,
  deleted_at_utc    INTEGER NULL,
  version           INTEGER NOT NULL DEFAULT 1
);

PRAGMA user_version = 3;
