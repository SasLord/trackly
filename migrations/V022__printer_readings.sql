-- V022: printer_readings — one row per poll snapshot (D-History-01).
--
-- toner_levels stored as JSON: {"black":{"level":45,"max":100,"pct":45},"drum":...}
-- Retention/downsample managed by a background task (D-Retention-01).
-- CASCADE delete: removing a printer removes all its readings.

CREATE TABLE printer_readings (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  printer_id  INTEGER NOT NULL REFERENCES printers(id) ON DELETE CASCADE,
  ts_utc      INTEGER NOT NULL,
  toner_levels TEXT   NULL,          -- JSON object (see D-History-01)
  page_count  INTEGER NULL,
  status      TEXT    NOT NULL DEFAULT 'unknown'
                      CHECK (status IN ('ok', 'warning', 'error', 'offline', 'unknown'))
);

CREATE INDEX idx_printer_readings_printer_ts
  ON printer_readings(printer_id, ts_utc DESC);

PRAGMA user_version = 22;
