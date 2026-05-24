-- V002: Core entities — users + locations.
--
-- User-mutable tables (D-Schema-03/04): include the four standard columns
-- (`created_at_utc`, `updated_at_utc`, `deleted_at_utc`, `version`).

CREATE TABLE users (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  login           TEXT    NOT NULL UNIQUE,
  full_name       TEXT    NOT NULL,
  password_hash   TEXT    NULL,                                    -- NULL for AD users (bind-only)
  role            TEXT    NOT NULL DEFAULT 'employee',             -- 'admin' | 'manager' | 'employee'
  ad_user         INTEGER NOT NULL DEFAULT 0,                      -- 0/1 boolean
  email           TEXT    NULL,
  notes           TEXT    NULL,
  created_at_utc  INTEGER NOT NULL,
  updated_at_utc  INTEGER NOT NULL,
  deleted_at_utc  INTEGER NULL,
  version         INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE locations (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  name            TEXT    NOT NULL UNIQUE,
  kind            TEXT    NULL,                                    -- 'office' | 'warehouse' | 'repair' | freeform
  address         TEXT    NULL,
  notes           TEXT    NULL,
  created_at_utc  INTEGER NOT NULL,
  updated_at_utc  INTEGER NOT NULL,
  deleted_at_utc  INTEGER NULL,
  version         INTEGER NOT NULL DEFAULT 1
);

PRAGMA user_version = 2;
