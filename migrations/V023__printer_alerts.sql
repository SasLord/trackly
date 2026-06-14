-- V023: printer_alerts — one active alert per printer (D-Alert-01).
--
-- UNIQUE on printer_id enforces dedup: only one active alert per printer at a time.
-- Alert types: 'offline' (SNMP unreachable) or 'error' (hrPrinterStatus = error).
-- CASCADE delete: removing a printer removes its alert row.

CREATE TABLE printer_alerts (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  printer_id          INTEGER NOT NULL UNIQUE REFERENCES printers(id) ON DELETE CASCADE,
  alert_type          TEXT    NOT NULL CHECK (alert_type IN ('offline', 'error')),
  first_seen_utc      INTEGER NOT NULL,
  last_seen_utc       INTEGER NOT NULL,
  acknowledged_at_utc INTEGER NULL
);

PRAGMA user_version = 23;
