-- WARNING: V015 — clone-on-handover model shift. Forward-only data migration.
-- Recommend backup of trackly.db before apply.
-- The data-migration step below is UNCONDITIONAL but naturally no-op on databases
-- where no act_items has quantity > 1 (the recursive CTE expands to zero rows).
-- See 03.1-CONTEXT.md G-12 (Migration decision a).
--
-- Schema changes:
--   1. acts.handover_date_utc (consolidates G-2 future DatePicker requirement —
--      backfilled from created_at_utc for existing rows).
--   2. act_items.parent_act_item_id (clone provenance; NULL for originals,
--      references original act_item.id for clones — for reporting only,
--      business logic uses device_id+act_id JOIN).
--   3. Data migration: for every existing act_item with quantity > 1,
--      clone (quantity - 1) device-rows + insert (quantity - 1) additional
--      act_items rows (one per cloned device).
--   4. New partial index idx_act_items_parent_act_item_id (clones only).
--
-- Clone semantics (G-12 + W-5):
--   - Cloned device row inherits all attributes from source EXCEPT:
--       inventory_number := NULL (decision G-12 (b) — clones are anonymous)
--       serial_number    := NULL (W-5 — physical serial cannot duplicate)
--       version := 1, created_at_utc/updated_at_utc := now, deleted_at_utc := NULL
--   - Cloned act_item gets parent_act_item_id := original act_item.id, quantity := 1.
--   - Original act_item retains its original quantity for historical/reporting
--     purposes; canonical "how many devices on this act" = COUNT(*) FROM act_items.

-- 1. acts.handover_date_utc — NOT NULL with default 0 (intermediate),
--    backfilled immediately from created_at_utc.
ALTER TABLE acts ADD COLUMN handover_date_utc INTEGER NOT NULL DEFAULT 0;
UPDATE acts SET handover_date_utc = COALESCE(created_at_utc, strftime('%s','now'));

-- 2. act_items.parent_act_item_id — clone provenance.
ALTER TABLE act_items ADD COLUMN parent_act_item_id INTEGER NULL
  REFERENCES act_items(id) ON DELETE SET NULL;

-- 3. Partial index — only clones (parent_act_item_id IS NOT NULL).
CREATE INDEX IF NOT EXISTS idx_act_items_parent_act_item_id
  ON act_items(parent_act_item_id) WHERE parent_act_item_id IS NOT NULL;

-- 4. Data migration: split qty>1 act_items into N independent rows.
--
-- Strategy (pure SQL, no row-by-row iteration):
--   (a) Build a temp table `t_clones` with one row per "clone needed":
--       columns (orig_act_item_id, source_device_id, act_id,
--                condition_at_time, complectation_at_time, seq AUTOINCREMENT).
--       Rows produced via recursive CTE expanding clone_index 2..quantity
--       for each act_item with quantity > 1.
--   (b) Snapshot MAX(devices.id) BEFORE the bulk INSERT — call this id_floor.
--   (c) INSERT INTO devices SELECT ... FROM t_clones JOIN source devices
--       ORDER BY t_clones.seq. Because devices.id is AUTOINCREMENT and
--       t_clones rows are processed in seq order, the i-th inserted device
--       gets id = id_floor + i. We record id_floor in another temp table.
--   (d) INSERT INTO act_items by joining t_clones with the formula
--       device_id = id_floor + t_clones.seq.
--
-- Idempotency: refinery's version-table prevents V015 from being re-applied.
-- We do NOT content-sniff to skip work.

-- 4a. Recursive CTE → temp table with AUTOINCREMENT seq.
CREATE TEMP TABLE t_clones (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  orig_act_item_id INTEGER NOT NULL,
  source_device_id INTEGER NOT NULL,
  act_id INTEGER NOT NULL,
  condition_at_time TEXT NULL,
  complectation_at_time TEXT NULL
);

INSERT INTO t_clones (orig_act_item_id, source_device_id, act_id,
                      condition_at_time, complectation_at_time)
  WITH RECURSIVE ints(n, max_n) AS (
    -- Start at 2: clone_index 1 is the original device_id (no clone needed).
    SELECT 2, (SELECT COALESCE(MAX(quantity), 1) FROM act_items)
    UNION ALL
    SELECT n + 1, max_n FROM ints WHERE n < max_n
  )
  SELECT
    ai.id, ai.device_id, ai.act_id,
    ai.condition_at_time, ai.complectation_at_time
  FROM act_items ai
  JOIN ints ON ints.n <= ai.quantity
  WHERE ai.quantity > 1
  ORDER BY ai.id, ints.n;

-- 4b. Snapshot id_floor = MAX(devices.id) before bulk INSERT.
CREATE TEMP TABLE t_id_floor (
  id_floor INTEGER NOT NULL
);
INSERT INTO t_id_floor (id_floor)
  SELECT COALESCE(MAX(id), 0) FROM devices;

-- 4c. Bulk INSERT clone devices in seq order. AUTOINCREMENT assigns
--     id = id_floor + seq for each row inserted in order.
INSERT INTO devices (
  type_id, name, inventory_number, serial_number, model,
  condition, complectation, notes,
  location_id, status_id,
  version, created_at_utc, updated_at_utc, deleted_at_utc
)
SELECT
  d.type_id, d.name, NULL, NULL, d.model,
  d.condition, d.complectation, d.notes,
  d.location_id, d.status_id,
  1, strftime('%s','now'), strftime('%s','now'), NULL
FROM t_clones tc
JOIN devices d ON d.id = tc.source_device_id
ORDER BY tc.seq;

-- 4d. Insert clone act_items using device_id = id_floor + tc.seq.
INSERT INTO act_items (act_id, device_id, condition_at_time, complectation_at_time,
                       quantity, parent_act_item_id)
SELECT
  tc.act_id,
  (SELECT id_floor FROM t_id_floor) + tc.seq,
  tc.condition_at_time,
  tc.complectation_at_time,
  1,
  tc.orig_act_item_id
FROM t_clones tc
ORDER BY tc.seq;

-- 4e. Cleanup temp tables (TEMP auto-dropped at session close, explicit
--     drops free memory immediately).
DROP TABLE t_clones;
DROP TABLE t_id_floor;

PRAGMA user_version = 15;
