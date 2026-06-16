-- V026: Organisation settings (single-row) + logo BLOB.
--
-- Supports SET-01 / SET-02 requirements:
--   - Org name, INN, KPP, address — displayed in print templates
--   - logo_blob / logo_mime — stored as binary for template substitution
--
-- Design decisions:
--   CHECK (id = 1) — enforces single-row invariant at schema level (T-07-01-b).
--   NOT NULL DEFAULT '...' on textual fields — allows future ALTER TABLE ADD COLUMN
--   without risking NULL in historic rows.
--   logo_blob BLOB NULL / logo_mime TEXT NULL — optional; NULL means "no logo".
--   Seed row inserted with id=1 immediately so SET commands can always UPDATE (not INSERT).
--   version INTEGER NOT NULL DEFAULT 1 — optimistic lock for concurrent Tauri/HTTP writes.
--
-- PRAGMA user_version = 26 (sequential; T-07-01-a mitigated by downgrade_protection test).

CREATE TABLE org_settings (
    id              INTEGER  NOT NULL PRIMARY KEY CHECK (id = 1),
    org_name        TEXT     NOT NULL DEFAULT 'Ваша организация',
    inn             TEXT     NOT NULL DEFAULT '0000000000',
    kpp             TEXT     NOT NULL DEFAULT '000000000',
    address         TEXT     NOT NULL DEFAULT 'Адрес не указан',
    logo_blob       BLOB     NULL,
    logo_mime       TEXT     NULL,
    created_at_utc  INTEGER  NOT NULL,
    updated_at_utc  INTEGER  NOT NULL,
    version         INTEGER  NOT NULL DEFAULT 1
);

-- Seed row — always present; SET handler does UPDATE WHERE id = 1.
INSERT INTO org_settings (id, org_name, inn, kpp, address, logo_blob, logo_mime, created_at_utc, updated_at_utc, version)
VALUES (1, 'Ваша организация', '0000000000', '000000000', 'Адрес не указан', NULL, NULL, unixepoch(), unixepoch(), 1);

PRAGMA user_version = 26;
