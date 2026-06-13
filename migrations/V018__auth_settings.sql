-- V018: Phase 5 — desktop lock flag in app_settings (D-Desktop-02 / Pitfall 6).
--
-- desktop_lock_enabled controls whether the desktop Tauri window requires
-- a user to authenticate before accessing the application (D-Desktop-01/02).
--
-- INSERT OR IGNORE is idempotent — safe to re-run on existing DB.
INSERT OR IGNORE INTO app_settings (key, value, created_at_utc, updated_at_utc)
VALUES ('desktop_lock_enabled', '0', unixepoch(), unixepoch());

PRAGMA user_version = 18;
