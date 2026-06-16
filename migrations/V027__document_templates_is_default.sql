-- V027: Add is_default column to document_templates.
--
-- is_default tracks whether the current body_minijinja equals the bundled
-- default template (seeded at startup). Used by TemplateService::update_body
-- (sets is_default=0) and reset_to_default (sets is_default=1).
--
-- NOT NULL DEFAULT 1 — existing rows created by seed_defaults_on_startup are
-- the defaults; SQLite requires DEFAULT for ADD COLUMN on non-empty tables.

ALTER TABLE document_templates
  ADD COLUMN is_default INTEGER NOT NULL DEFAULT 1;

PRAGMA user_version = 27;
