-- V020: Printers — SNMP metadata table extending devices.
--
-- `printers` extends `devices` (type_id=Принтер) with SNMP-polling fields.
-- community stored as plain text — Secret<T> wrapping happens in the Rust service layer.
-- USB учёт (PRN-04): usb_host_device_id links printer to its host workstation device.
-- CHECK(ip_address IS NOT NULL OR usb_host_device_id IS NOT NULL) enforces at least
-- one connectivity method (SNMP via IP or USB via host).
--
-- oid_profiles table is created in V021; FK is deferred-safe in SQLite:
-- FK constraints are checked on INSERT, not on CREATE TABLE, so ordering is valid.

CREATE TABLE printers (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  device_id           INTEGER NOT NULL UNIQUE REFERENCES devices(id),
  ip_address          TEXT    NULL,                          -- NULL for USB-only printers
  community           TEXT    NOT NULL DEFAULT 'public',     -- SNMP community string
  snmp_version        TEXT    NOT NULL DEFAULT 'v2c'
                              CHECK (snmp_version IN ('v1', 'v2c', 'v3')),
  vendor              TEXT    NULL,                          -- detected at discovery
  oid_profile_id      INTEGER NULL REFERENCES oid_profiles(id),
  last_seen_utc       INTEGER NULL,
  usb_host_device_id  INTEGER NULL REFERENCES devices(id),  -- PRN-04 USB учёт
  created_at_utc      INTEGER NOT NULL,
  updated_at_utc      INTEGER NOT NULL,
  version             INTEGER NOT NULL DEFAULT 1,
  CHECK (ip_address IS NOT NULL OR usb_host_device_id IS NOT NULL)
);

PRAGMA user_version = 20;
