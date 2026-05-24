-- V001: Initial schema — lookup tables + seed rows.
--
-- Runtime PRAGMAs (applied per-connection by `trackly_infra::db::pragmas`):
--   journal_mode = WAL          (persists to file header on first write)
--   synchronous = NORMAL        (per-connection)
--   busy_timeout = 5000         (per-connection)
--   foreign_keys = ON           (per-connection)
--   wal_autocheckpoint = 1000   (per-connection)
--   temp_store = MEMORY         (per-connection)
--   mmap_size = 128 MiB         (per-connection)
--
-- This migration intentionally performs DDL writes so that WAL mode persists
-- into the file header on first launch (Pitfall #4 in 01-RESEARCH.md).
--
-- Lookup tables (D-Schema-03: hard-delete; no `_at_utc` / `version` columns).

CREATE TABLE device_types (
  id   INTEGER PRIMARY KEY,
  name TEXT    NOT NULL UNIQUE
);
INSERT INTO device_types (id, name) VALUES
  (1, 'Устройство'),
  (2, 'Принтер');

CREATE TABLE device_statuses (
  id   INTEGER PRIMARY KEY,
  name TEXT    NOT NULL UNIQUE
);
INSERT INTO device_statuses (id, name) VALUES
  (1, 'На складе'),
  (2, 'В работе'),
  (3, 'На ремонте'),
  (4, 'Списано');

CREATE TABLE cartridge_states (
  id   INTEGER PRIMARY KEY,
  name TEXT    NOT NULL UNIQUE
);
INSERT INTO cartridge_states (id, name) VALUES
  (1, 'Полный'),
  (2, 'Частичный'),
  (3, 'Пустой');

CREATE TABLE cartridge_statuses (
  id   INTEGER PRIMARY KEY,
  name TEXT    NOT NULL UNIQUE
);
INSERT INTO cartridge_statuses (id, name) VALUES
  (1, 'На складе'),
  (2, 'В работе'),
  (3, 'На заправке'),
  (4, 'Списано');

PRAGMA user_version = 1;
