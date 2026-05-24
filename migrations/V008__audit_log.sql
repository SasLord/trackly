-- V008: Audit log — append-only history of all entity mutations.
--
-- Hard-delete table (D-Schema-03): NO `deleted_at_utc`, NO `version`.
-- Retention is out of scope in Phase 1; Phase 7 scheduled-tasks owns cleanup.
-- Schema per D-Schema-05.
--
-- Indexes live in V012 (alongside all other cross-table indexes).

CREATE TABLE audit_log (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type     TEXT    NOT NULL,                                 -- 'device' | 'act' | 'cartridge' | …
  entity_id       INTEGER NOT NULL,
  action          TEXT    NOT NULL,                                 -- 'create' | 'update' | 'delete' | 'restore' | 'custom:xxx'
  user_id         INTEGER NULL REFERENCES users(id),                -- NULL for system-initiated changes
  before_json     TEXT    NULL,                                     -- full record snapshot before the change
  after_json      TEXT    NULL,                                     -- full record snapshot after the change
  payload_json    TEXT    NULL,                                     -- extra context (e.g., act-link IDs)
  created_at_utc  INTEGER NOT NULL
);

PRAGMA user_version = 8;
