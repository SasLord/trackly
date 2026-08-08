-- V036: Organisation full legal name (DOC-05).
--
-- Adds full_name to org_settings for the full legal name of the organisation
-- (e.g. "Общество с ограниченной ответственностью «Пример»"), rendered in the
-- shared document header alongside the existing short org_name. Multiline
-- (D-02): a legal-form prefix and quoted name commonly sit on separate lines
-- in printed requisites.
--
-- Design decision (34-CONTEXT D-01/D-02/D-04): DEFAULT '' (empty string) —
-- same rationale as V033/V035: historic rows degrade to "no full-name line
-- shown" via an independent `{% if %}` guard (D-04), never a misleading
-- placeholder.
--
-- Appended at the end of the column list — existing SELECT/UPDATE ordinal
-- positions in org_db_service.rs are unaffected.
--
-- PRAGMA user_version = 36.

ALTER TABLE org_settings ADD COLUMN full_name TEXT NOT NULL DEFAULT '';

PRAGMA user_version = 36;
