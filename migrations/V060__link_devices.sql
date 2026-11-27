-- V060: устройства и входы Link (Phase 44, D-03 / D-12 / D-13).
--
-- Только CREATE TABLE / CREATE INDEX; ни одного DROP и ни одной перестройки
-- существующих таблиц (урок V042). Без внешних ключей на users: записи
-- переживают удаление учётной записи (как V050 / V054 / V058).
--
--   - link_devices: непустой revoked_at_utc = запрет устройства (D-13); он
--     отделён от отзыва входа (link_logins.revoked_at_utc).
--   - link_logins: каждый вход (D-12); token_hash NULL для SSO-входов.
--   - link_invites: коды-приглашения только хэшем (D-03).
-- Секреты хранятся только как SHA-256 хэши.

CREATE TABLE link_devices (
  device_id              TEXT PRIMARY KEY,
  secret_hash            BLOB NOT NULL,
  hostname               TEXT,
  os                     TEXT,
  arch                   TEXT,
  client_version         TEXT,
  enrolled_via_invite_id INTEGER NULL,
  first_seen_utc         INTEGER NOT NULL,
  last_seen_utc          INTEGER NULL,
  last_ip                TEXT NULL,
  last_user_id           INTEGER NULL,
  revoked_at_utc         INTEGER NULL,
  revoked_by_user_id     INTEGER NULL
);

CREATE TABLE link_logins (
  id                 INTEGER PRIMARY KEY AUTOINCREMENT,
  device_id          TEXT NOT NULL,
  user_id            INTEGER NOT NULL,
  method             TEXT NOT NULL CHECK (method IN ('sso','password')),
  token_hash         BLOB NULL UNIQUE,
  created_at_utc     INTEGER NOT NULL,
  last_used_utc      INTEGER NULL,
  revoked_at_utc     INTEGER NULL,
  revoked_by_user_id INTEGER NULL
);

CREATE INDEX idx_link_logins_user ON link_logins (user_id, revoked_at_utc);
CREATE INDEX idx_link_logins_device ON link_logins (device_id, revoked_at_utc);

CREATE TABLE link_invites (
  id                 INTEGER PRIMARY KEY AUTOINCREMENT,
  code_hash          BLOB NOT NULL UNIQUE,
  created_by_user_id INTEGER NOT NULL,
  created_at_utc     INTEGER NOT NULL,
  expires_at_utc     INTEGER NOT NULL,
  used_at_utc        INTEGER NULL,
  used_by_device_id  TEXT NULL,
  revoked_at_utc     INTEGER NULL,
  revoked_by_user_id INTEGER NULL
);

PRAGMA user_version = 60;
