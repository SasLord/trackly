-- V028: Add ad_subtype discriminator column to requests table.
--
-- D-REG-03: restoration-of-access requests reuse request_type='ad_register'
-- rather than a new CHECK value (avoids a CHECK-constraint table rebuild).
-- ad_subtype distinguishes 'register' (new/unknown AD user) from 'restore'
-- (blocked/soft-deleted AD user requesting reactivation). NULL for
-- non-ad_register requests (cartridge_replace, free_form).
--
-- Nullable, no DEFAULT — SQLite ADD COLUMN with NULL is always safe
-- regardless of existing rows (Pitfall 2 from V019 does not apply here).

ALTER TABLE requests
  ADD COLUMN ad_subtype TEXT NULL;

PRAGMA user_version = 28;
