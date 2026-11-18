-- Phase 41.5 gap-closure (UAT test 3, D-G2/D-G3): per-user chat notification mute.
-- A row means "muted"; unmuting deletes the row. No foreign keys (as chat_reads and
-- chat_messages: the row outlives its request). Only CREATE TABLE, no rebuilds.
CREATE TABLE chat_mutes (
    user_id      INTEGER NOT NULL,
    request_id   INTEGER NOT NULL,
    muted_at_utc INTEGER NOT NULL,
    PRIMARY KEY (user_id, request_id)
);

PRAGMA user_version = 55;
