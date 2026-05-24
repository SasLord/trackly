-- V009: Counters — generic monotonic counters for human-visible numbering.
--
-- `act_number` produces act numbers (1, 2, 3, …); `cartridge_seq` produces the
-- numeric portion of cartridge codes (formatted as C-NNNNNN by the service
-- layer). Service-layer code does `UPDATE counters SET current_value = current_value + 1
-- WHERE name = ? RETURNING current_value`.
--
-- Hard-delete system table (D-Schema-03): NO standard4 columns.

CREATE TABLE counters (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  name            TEXT    NOT NULL UNIQUE,
  current_value   INTEGER NOT NULL DEFAULT 0
);

INSERT INTO counters (name, current_value) VALUES
  ('act_number', 0),
  ('cartridge_seq', 0);

PRAGMA user_version = 9;
