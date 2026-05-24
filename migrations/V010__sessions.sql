-- V010: Sessions — backing store for tower-sessions (server mode).
--
-- Phase 5 wires tower-sessions through a custom `SessionStore` impl reading
-- and writing this table directly. Phase 1 ships the schema only.
--
-- Hard-delete system table (D-Schema-03): NO standard4 columns.
-- `id` is a BLOB (session-id bytes from tower-sessions); `data` is the
-- serialised session payload; `expiry_date` is unix epoch seconds.

CREATE TABLE sessions (
  id            BLOB    PRIMARY KEY,
  data          BLOB    NOT NULL,
  expiry_date   INTEGER NOT NULL
);

PRAGMA user_version = 10;
