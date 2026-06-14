-- V021: OID profiles table + seed data for 4 vendors + RFC3805 fallback (D-OID-01).
--
-- Data-driven OID profiles allow matching a printer's sysObjectID prefix to the
-- correct polling strategy (toner encoding, OIDs, page counter).
-- 5 profiles seeded: pantum / kyocera / hp / canon / rfc3805 (fallback).
-- UI editor for OID profiles is deferred to Phase 7.

CREATE TABLE oid_profiles (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  name              TEXT    NOT NULL UNIQUE,
  vendor_prefix     TEXT    NOT NULL,          -- sysObjectID prefix for vendor matching
  toner_level_oid   TEXT    NULL,
  toner_max_oid     TEXT    NULL,              -- NULL for 'percent' encoding (value is already %)
  toner_encoding    TEXT    NOT NULL DEFAULT 'level_over_max'
                            CHECK (toner_encoding IN ('percent', 'level_over_max')),
  page_counter_oid  TEXT    NULL,
  status_oid        TEXT    NOT NULL,          -- hrPrinterStatus OID
  serial_oid        TEXT    NULL,
  notes             TEXT    NULL
);

INSERT INTO oid_profiles (name, vendor_prefix, toner_level_oid, toner_max_oid,
    toner_encoding, page_counter_oid, status_oid, notes) VALUES
('pantum',   '1.3.6.1.4.1.40093',  '1.3.6.1.4.1.40093.6.3.1',       NULL,
    'percent',        '1.3.6.1.4.1.40093.10.3.1.1', '1.3.6.1.2.1.25.3.5.1.1.1',
    'Pantum BM5100ADN и аналоги'),
('kyocera',  '1.3.6.1.4.1.1347',   '1.3.6.1.2.1.43.11.1.1.9.1.1',   '1.3.6.1.2.1.43.11.1.1.8.1.1',
    'level_over_max', '1.3.6.1.2.1.43.10.2.1.4.1.1', '1.3.6.1.2.1.25.3.5.1.1.1',
    'Kyocera ECOSYS'),
('hp',       '1.3.6.1.4.1.11',     '1.3.6.1.2.1.43.11.1.1.9.1.1',   '1.3.6.1.2.1.43.11.1.1.8.1.1',
    'level_over_max', '1.3.6.1.2.1.43.10.2.1.4.1.1', '1.3.6.1.2.1.25.3.5.1.1.1',
    'HP LaserJet'),
('canon',    '1.3.6.1.4.1.1602',   '1.3.6.1.2.1.43.11.1.1.9.1.1',   '1.3.6.1.2.1.43.11.1.1.8.1.1',
    'level_over_max', '1.3.6.1.2.1.43.10.2.1.4.1.1', '1.3.6.1.2.1.25.3.5.1.1.1',
    'Canon iR/imageRUNNER'),
('rfc3805',  '',                    '1.3.6.1.2.1.43.11.1.1.9.1.1',   '1.3.6.1.2.1.43.11.1.1.8.1.1',
    'level_over_max', '1.3.6.1.2.1.43.10.2.1.4.1.1', '1.3.6.1.2.1.25.3.5.1.1.1',
    'RFC 3805 fallback — any printer');

PRAGMA user_version = 21;
