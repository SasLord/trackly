-- V007: Document templates — versioned printable form templates.
--
-- Templates are MiniJinja-rendered (Phase 3). Body stored as TEXT;
-- `is_active = 1` flags the current template per kind, only one active per kind
-- enforced by partial unique index.
--
-- User-mutable: standard4 columns.

CREATE TABLE document_templates (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  kind              TEXT    NOT NULL CHECK (kind IN ('act_handover', 'act_acceptance')),
  name              TEXT    NOT NULL,
  body_minijinja    TEXT    NOT NULL,
  is_active         INTEGER NOT NULL DEFAULT 1,                     -- 0/1 boolean
  created_at_utc    INTEGER NOT NULL,
  updated_at_utc    INTEGER NOT NULL,
  deleted_at_utc    INTEGER NULL,
  version           INTEGER NOT NULL DEFAULT 1
);

-- Only one active template per kind among live (non-soft-deleted) rows.
CREATE UNIQUE INDEX idx_document_templates_kind_active_unique
  ON document_templates(kind)
  WHERE is_active = 1 AND deleted_at_utc IS NULL;

PRAGMA user_version = 7;
