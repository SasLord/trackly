-- V031: Add 'cancelled' to the requests.status CHECK constraint (GAP-12-07/A4).
--
-- Plan 12-14 introduces RequestService::cancel() — an author-only self-cancel
-- of the CALLER'S OWN "open" request, distinct from the Admin/Manager-only
-- Reject. The new RequestTransitionOp::Cancel writes status = 'cancelled',
-- which the V006 CHECK (status IN ('open','in_progress','completed',
-- 'rejected')) would reject outright.
--
-- SQLite has no `ALTER TABLE ... DROP CONSTRAINT`, so the only way to widen a
-- CHECK is the standard rebuild pattern (same as V030): create a replacement
-- table with the new CHECK, copy rows verbatim (including every column added
-- by V024/V028 since V006), drop the old table, rename the new one into
-- place. FKs are resolved by table name in SQLite, so renaming
-- `requests_new` -> `requests` restores referential integrity for any table
-- that references `requests(id)` without touching those tables.
--
-- Foreign keys are checked immediately (not deferred) in this project's
-- connections (PRAGMA foreign_keys = ON), so the rebuild is wrapped in an
-- explicit OFF/ON pair scoped to this migration file only (refinery runs one
-- file per transaction, so this window never overlaps user traffic).

PRAGMA foreign_keys = OFF;

CREATE TABLE requests_new (
  id                        INTEGER PRIMARY KEY AUTOINCREMENT,
  request_type              TEXT    NOT NULL CHECK (request_type IN ('cartridge_replace', 'free_form', 'ad_register')),
  status                    TEXT    NOT NULL CHECK (status IN ('open', 'in_progress', 'completed', 'rejected', 'cancelled')) DEFAULT 'open',
  requested_by_user_id      INTEGER NOT NULL REFERENCES users(id),
  assigned_to_user_id       INTEGER NULL REFERENCES users(id),
  printer_device_id         INTEGER NULL REFERENCES devices(id),
  cartridge_model_id        INTEGER NULL REFERENCES cartridge_models(id),
  description               TEXT    NULL,
  resolution_notes          TEXT    NULL,
  created_at_utc            INTEGER NOT NULL,
  updated_at_utc            INTEGER NOT NULL,
  deleted_at_utc            INTEGER NULL,
  version                   INTEGER NOT NULL DEFAULT 1,
  category_id               INTEGER NULL REFERENCES request_categories(id),
  completed_cartridge_id    INTEGER NULL REFERENCES cartridges(id),
  ad_subtype                TEXT    NULL
);

INSERT INTO requests_new (
  id, request_type, status, requested_by_user_id, assigned_to_user_id,
  printer_device_id, cartridge_model_id, description, resolution_notes,
  created_at_utc, updated_at_utc, deleted_at_utc, version, category_id,
  completed_cartridge_id, ad_subtype
)
SELECT
  id, request_type, status, requested_by_user_id, assigned_to_user_id,
  printer_device_id, cartridge_model_id, description, resolution_notes,
  created_at_utc, updated_at_utc, deleted_at_utc, version, category_id,
  completed_cartridge_id, ad_subtype
FROM requests;

DROP TABLE requests;

ALTER TABLE requests_new RENAME TO requests;

PRAGMA foreign_keys = ON;

PRAGMA user_version = 31;
