-- V035: Organisation address second line (ORG-02).
--
-- Adds address_line2 to org_settings for a free-form second address line
-- (e.g. "офис 305, корпус 2") displayed under the main address in all
-- printed documents (act_handover.html, act_acceptance.html, report.html).
--
-- Design decision (20-CONTEXT D-04): DEFAULT '' (empty string) — same
-- rationale as V033: historic rows degrade to "no second line shown"
-- (D-06's `{% if %}` guard), never a misleading placeholder.
--
-- Appended at the end of the column list — existing SELECT/UPDATE ordinal
-- positions in org_db_service.rs are unaffected.
--
-- PRAGMA user_version = 35.

ALTER TABLE org_settings ADD COLUMN address_line2 TEXT NOT NULL DEFAULT '';

PRAGMA user_version = 35;
