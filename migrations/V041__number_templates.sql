-- V041: numbering templates — foundation for Phase 40.2.
--
-- D-15: `counters` (V009) is dropped IN THIS SAME MIGRATION, not in a
-- separate step — the numbering-template mechanism fully replaces it, and
-- shipping a window where both mechanisms coexist would let code
-- accidentally keep reading/writing the old table.
--
-- D-16: seeding reproduces today's numbering EXACTLY, width 4 (not 6, per
-- the 2026-09-18 clarification in 40.2-CONTEXT.md D-16): `act_number` gets
-- an unbounded `[X]` mask (acts today are plain incrementing integers,
-- unformatted); `cartridge_code`/`drum_code` get `C-[XXXX]`/`D-[XXXX]`,
-- matching the `C-XXXX`/`D-XXXX` format already produced by the service
-- layer (v1.1.2 CRT-01). Seeding is data-independent of `devices`/`acts`/
-- `cartridges` — it does not read or rely on existing row counts, unlike
-- `compute_next` (Plan 03+) which looks at live data at call time.
--
-- D-17: `number_template_contexts.template_id` uses `ON DELETE SET NULL`
-- so a physically-deleted template (D-12) can never leave a dangling id in
-- a context's memory — enforced structurally by the FK, not by service-layer
-- discipline (see threat T-40.2-01).

CREATE TABLE number_templates (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  type            TEXT    NOT NULL CHECK (type IN ('device_inventory', 'act_number', 'cartridge_code', 'drum_code')),
  mask            TEXT    NOT NULL,
  created_at_utc  INTEGER NOT NULL,
  updated_at_utc  INTEGER NOT NULL,
  version         INTEGER NOT NULL DEFAULT 1,
  UNIQUE(type, mask)
);

CREATE TABLE number_template_contexts (
  context         TEXT    PRIMARY KEY CHECK (context IN ('device_create', 'printer_create', 'act_create', 'cartridge_create', 'drum_create')),
  template_id     INTEGER NULL REFERENCES number_templates(id) ON DELETE SET NULL,
  updated_at_utc  INTEGER NOT NULL
);

-- All 5 contexts always exist as rows (service layer never needs to handle
-- a missing context row). `device_create`/`printer_create` start with no
-- default template (D-13) — inventory numbers stay free-text unless an
-- admin explicitly assigns a template later.
INSERT INTO number_template_contexts (context, template_id, updated_at_utc) VALUES
  ('device_create',    NULL, strftime('%s', 'now')),
  ('printer_create',   NULL, strftime('%s', 'now')),
  ('act_create',       NULL, strftime('%s', 'now')),
  ('cartridge_create', NULL, strftime('%s', 'now')),
  ('drum_create',      NULL, strftime('%s', 'now'));

-- Seed the three templates that reproduce today's numbering (D-16).
INSERT INTO number_templates (type, mask, created_at_utc, updated_at_utc, version) VALUES
  ('act_number',     '[X]',      strftime('%s', 'now'), strftime('%s', 'now'), 1),
  ('cartridge_code', 'C-[XXXX]', strftime('%s', 'now'), strftime('%s', 'now'), 1),
  ('drum_code',      'D-[XXXX]', strftime('%s', 'now'), strftime('%s', 'now'), 1);

-- Wire the seeded templates in as the default for their matching context.
UPDATE number_template_contexts
SET template_id = (SELECT id FROM number_templates WHERE type = 'act_number' AND mask = '[X]'),
    updated_at_utc = strftime('%s', 'now')
WHERE context = 'act_create';

UPDATE number_template_contexts
SET template_id = (SELECT id FROM number_templates WHERE type = 'cartridge_code' AND mask = 'C-[XXXX]'),
    updated_at_utc = strftime('%s', 'now')
WHERE context = 'cartridge_create';

UPDATE number_template_contexts
SET template_id = (SELECT id FROM number_templates WHERE type = 'drum_code' AND mask = 'D-[XXXX]'),
    updated_at_utc = strftime('%s', 'now')
WHERE context = 'drum_create';

-- D-15: drop the now-superseded generic counters table in this same migration.
DROP TABLE counters;

PRAGMA user_version = 41;
