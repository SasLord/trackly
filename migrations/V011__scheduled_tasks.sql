-- V011: Scheduled tasks — placeholder schema for Phase 7 supervisor.
--
-- Phase 1 creates the table; Phase 7 implements the actual supervisor task
-- (cron-style execution, status reporting, retry policy).
--
-- Hard-delete system table (D-Schema-03): NO standard4 columns.

CREATE TABLE scheduled_tasks (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  name              TEXT    NOT NULL UNIQUE,
  cron              TEXT    NULL,                                   -- cron expression; NULL = manual trigger only
  last_run_at_utc   INTEGER NULL,
  next_run_at_utc   INTEGER NULL,
  status            TEXT    NOT NULL DEFAULT 'idle',                -- 'idle' | 'running' | 'succeeded' | 'failed'
  payload_json      TEXT    NULL                                    -- task-specific config
);

PRAGMA user_version = 11;
