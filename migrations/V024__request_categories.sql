-- V024: Request categories for free_form requests (D-Req-Categories-01).
--
-- Lookup table (not a CHECK enum) — easier to extend in Phase 7 without migration.
-- 4 categories seeded per D-Req-Categories-01: Ремонт / Расходники / ПО / Прочее.
--
-- Also adds two columns to requests (table created in V006):
--   category_id        — FK to request_categories (only set for free_form requests)
--   completed_cartridge_id — FK to cartridges (set on Complete transition, REQ-05)

CREATE TABLE request_categories (
  id    INTEGER PRIMARY KEY AUTOINCREMENT,
  name  TEXT NOT NULL UNIQUE
);

INSERT INTO request_categories (name) VALUES
  ('Ремонт техники'),
  ('Расходные материалы'),
  ('Программное обеспечение'),
  ('Прочее');

-- Add category_id to requests (nullable — only for free_form type)
ALTER TABLE requests ADD COLUMN category_id INTEGER NULL REFERENCES request_categories(id);

-- PRN-07 / REQ-05: link completed cartridge replacement to the cartridge installed
ALTER TABLE requests ADD COLUMN completed_cartridge_id INTEGER NULL REFERENCES cartridges(id);

PRAGMA user_version = 24;
