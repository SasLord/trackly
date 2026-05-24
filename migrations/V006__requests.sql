-- V006: Requests (заявки) — incoming work items from employees.
--
-- Three request kinds in v1: cartridge replacement (printer + model),
-- free-form (description only), AD registration (user wants AD-account binding).
-- Status transitions are enforced by service layer; the DB only constrains the
-- allowed values via CHECK.
--
-- User-mutable: standard4 columns.

CREATE TABLE requests (
  id                        INTEGER PRIMARY KEY AUTOINCREMENT,
  request_type              TEXT    NOT NULL CHECK (request_type IN ('cartridge_replace', 'free_form', 'ad_register')),
  status                    TEXT    NOT NULL CHECK (status IN ('open', 'in_progress', 'completed', 'rejected')) DEFAULT 'open',
  requested_by_user_id      INTEGER NOT NULL REFERENCES users(id),
  assigned_to_user_id       INTEGER NULL REFERENCES users(id),
  printer_device_id         INTEGER NULL REFERENCES devices(id),
  cartridge_model_id        INTEGER NULL REFERENCES cartridge_models(id),
  description               TEXT    NULL,
  resolution_notes          TEXT    NULL,
  created_at_utc            INTEGER NOT NULL,
  updated_at_utc            INTEGER NOT NULL,
  deleted_at_utc            INTEGER NULL,
  version                   INTEGER NOT NULL DEFAULT 1
);

PRAGMA user_version = 6;
