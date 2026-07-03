-- V033: Organisation extended requisites (PDFA-03).
--
-- Adds phone/fax/email/okpo/ogrn columns to org_settings so the act header
-- can display the full set of requisites required by the Word-fidelity
-- sample (see .planning/PHASE-BRIEF-act-pdf-word-fidelity.md).
--
-- Design decision (14-CONTEXT D-02): DEFAULT '' (empty string), NOT the
-- placeholder strings V026 used for name/inn/kpp — missing requisites on
-- historic rows must degrade to empty/"—" in rendered documents, not to a
-- misleading placeholder value. NOT NULL preserves the ADD-COLUMN-safe
-- pattern documented in V026 (no NULL in historic rows).
--
-- Appended at the end of the column list — existing SELECT/UPDATE ordinal
-- positions in org_db_service.rs are unaffected; new columns are added last
-- in every SQL site touching org_settings.
--
-- PRAGMA user_version = 33 (sequential; downgrade_protection test covers it).

ALTER TABLE org_settings ADD COLUMN phone TEXT NOT NULL DEFAULT '';
ALTER TABLE org_settings ADD COLUMN fax   TEXT NOT NULL DEFAULT '';
ALTER TABLE org_settings ADD COLUMN email TEXT NOT NULL DEFAULT '';
ALTER TABLE org_settings ADD COLUMN okpo  TEXT NOT NULL DEFAULT '';
ALTER TABLE org_settings ADD COLUMN ogrn  TEXT NOT NULL DEFAULT '';

PRAGMA user_version = 33;
