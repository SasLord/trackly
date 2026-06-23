-- V030: Drop the connectivity CHECK constraint on `printers`.
--
-- GAP-12-08 (UAT round 2, A5): the V020 CHECK(ip_address IS NOT NULL OR
-- usb_host_device_id IS NOT NULL) wrongly required at least one connectivity
-- method to create a printer row. IP is optional per requirements (a printer
-- may exist purely as an inventory record before SNMP/USB wiring is
-- configured) — the CHECK blocked that valid case with a constraint error.
--
-- SQLite has no `ALTER TABLE ... DROP CONSTRAINT`, so the only way to remove
-- a CHECK is the standard 12-step rebuild pattern: create a replacement table
-- without the constraint, copy rows, drop the old table, rename the new one
-- into place. `printer_readings.printer_id` and `printer_alerts.printer_id`
-- reference `printers(id)` via `ON DELETE CASCADE`; SQLite resolves FKs by
-- table name (not rowid/oid), so renaming `printers_new` -> `printers`
-- restores FK integrity for those tables without touching them.
--
-- Foreign keys are checked immediately (not deferred) in this project's
-- connections (PRAGMA foreign_keys = ON, see CLAUDE.md / pragmas.rs), so
-- `DROP TABLE printers` while reader rows still reference it via FK would be
-- blocked. The reconstruction is therefore wrapped in an explicit
-- `PRAGMA foreign_keys = OFF;` / `PRAGMA foreign_keys = ON;` pair scoped to
-- this migration file only (refinery runs one file per transaction —
-- set_grouped(false) — so this window never overlaps user traffic).
--
-- `printer_cartridge_models` (V029) references `devices(id)`, NOT
-- `printers(id)` — unaffected by this migration.

PRAGMA foreign_keys = OFF;

CREATE TABLE printers_new (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  device_id           INTEGER NOT NULL UNIQUE REFERENCES devices(id),
  ip_address          TEXT    NULL,                          -- NULL for USB-only or unconfigured printers
  community           TEXT    NOT NULL DEFAULT 'public',     -- SNMP community string
  snmp_version        TEXT    NOT NULL DEFAULT 'v2c'
                              CHECK (snmp_version IN ('v1', 'v2c', 'v3')),
  vendor              TEXT    NULL,                          -- detected at discovery
  oid_profile_id      INTEGER NULL REFERENCES oid_profiles(id),
  last_seen_utc       INTEGER NULL,
  usb_host_device_id  INTEGER NULL REFERENCES devices(id),  -- PRN-04 USB учёт
  created_at_utc      INTEGER NOT NULL,
  updated_at_utc      INTEGER NOT NULL,
  version             INTEGER NOT NULL DEFAULT 1
);

INSERT INTO printers_new (
  id, device_id, ip_address, community, snmp_version, vendor, oid_profile_id,
  last_seen_utc, usb_host_device_id, created_at_utc, updated_at_utc, version
)
SELECT
  id, device_id, ip_address, community, snmp_version, vendor, oid_profile_id,
  last_seen_utc, usb_host_device_id, created_at_utc, updated_at_utc, version
FROM printers;

DROP TABLE printers;

ALTER TABLE printers_new RENAME TO printers;

PRAGMA foreign_keys = ON;

PRAGMA user_version = 30;
