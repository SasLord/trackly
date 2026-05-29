-- V014: Phase 3 supporting indexes + device_statuses.code + act_items.quantity.
--
-- Indexes accelerate Phase 3 read paths:
--   - act_items lookups by act_id / device_id (act detail load + undo)
--   - acts lookup by parent_act_id (return list for handover)
--   - audit_log scans by (entity_type, entity_id, created_at_utc) for undo
--
-- B-1 fix: `device_statuses.code` adds a machine-stable identifier so the
-- service layer can resolve statuses without depending on Russian display
-- names (V001 stored only `name = 'На складе' / 'В работе' / ...`).
--
-- B-2 fix: `act_items.quantity` enables ACT-03 quantity persistence and
-- ACT-08 partial-return remaining math. SQLite ALTER TABLE requires a
-- NOT NULL column to have a DEFAULT when the table already has rows —
-- `DEFAULT 1` is the canonical choice (each existing act_item is one unit).

-- Phase 3 indexes
CREATE INDEX IF NOT EXISTS idx_act_items_act_id ON act_items(act_id);
CREATE INDEX IF NOT EXISTS idx_act_items_device_id ON act_items(device_id);
CREATE INDEX IF NOT EXISTS idx_acts_parent_act_id ON acts(parent_act_id) WHERE parent_act_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_audit_log_entity ON audit_log(entity_type, entity_id, created_at_utc);

-- B-1 fix: device_statuses.code (machine-stable identifier).
ALTER TABLE device_statuses ADD COLUMN code TEXT;
UPDATE device_statuses SET code = 'на_складе'  WHERE name = 'На складе';
UPDATE device_statuses SET code = 'в_работе'   WHERE name = 'В работе';
UPDATE device_statuses SET code = 'на_ремонте' WHERE name = 'На ремонте';
UPDATE device_statuses SET code = 'списано'    WHERE name = 'Списано';
CREATE UNIQUE INDEX idx_device_statuses_code_unique ON device_statuses(code) WHERE code IS NOT NULL;

-- B-2 fix: act_items.quantity (ACT-03 / ACT-08 persistence).
ALTER TABLE act_items ADD COLUMN quantity INTEGER NOT NULL DEFAULT 1;

-- ACT-03: «Сроком до» (deadline) — Unix seconds, nullable.
-- Required by ActFormModal header field per D-Acts-Create-01.
ALTER TABLE acts ADD COLUMN deadline_utc INTEGER NULL;

PRAGMA user_version = 14;
