-- V012: Cross-table indexes + FTS5 virtual tables.
--
-- All indexes that depend on tables created in V001..V011 land here so that
-- domain-specific migrations stay focused on their own schema. FTS5 virtual
-- tables use the `unicode61 remove_diacritics 2` tokenizer to handle cyrillic
-- `ё`/`е` normalisation transparently.
--
-- Triggers that keep FTS5 tables in sync with live data are NOT created in
-- Phase 1 — Phase 2 (devices_fts), Phase 3 (acts_fts), Phase 4 (cartridges_fts)
-- own those triggers when the corresponding write operations land.

-- audit_log indexes (D-Schema-05).
CREATE INDEX idx_audit_log_entity ON audit_log(entity_type, entity_id, created_at_utc);
CREATE INDEX idx_audit_log_user   ON audit_log(user_id, created_at_utc);

-- acts + act_items.
CREATE INDEX idx_acts_parent       ON acts(parent_act_id);
CREATE INDEX idx_act_items_act     ON act_items(act_id);
CREATE INDEX idx_act_items_device  ON act_items(device_id);

-- cartridges + compatibility.
CREATE INDEX idx_cartridges_model           ON cartridges(model_id);
CREATE INDEX idx_cartridge_compat_model     ON cartridge_model_compatibility(cartridge_model_id);

-- devices.
CREATE INDEX idx_devices_location ON devices(location_id);
CREATE INDEX idx_devices_status   ON devices(status_id);
CREATE INDEX idx_devices_type     ON devices(type_id);

-- requests.
CREATE INDEX idx_requests_status   ON requests(status);
CREATE INDEX idx_requests_assigned ON requests(assigned_to_user_id);

-- sessions / scheduled_tasks.
CREATE INDEX idx_sessions_expiry           ON sessions(expiry_date);
CREATE INDEX idx_scheduled_tasks_next_run  ON scheduled_tasks(next_run_at_utc);

-- FTS5 virtual tables. `content=` external-content mode means the virtual
-- table mirrors a rowid in the source table; triggers land in later phases.
CREATE VIRTUAL TABLE devices_fts USING fts5(
  name, inventory_number, serial_number, model,
  content='devices', content_rowid='id',
  tokenize='unicode61 remove_diacritics 2'
);

CREATE VIRTUAL TABLE acts_fts USING fts5(
  number, giver_name, receiver_name,
  content='acts', content_rowid='id',
  tokenize='unicode61 remove_diacritics 2'
);

CREATE VIRTUAL TABLE cartridges_fts USING fts5(
  code, location, holder_name,
  content='cartridges', content_rowid='id',
  tokenize='unicode61 remove_diacritics 2'
);

PRAGMA user_version = 12;
