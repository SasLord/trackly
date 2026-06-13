-- V019: Add is_active column to users table.
--
-- Phase 5 AuthService requires users.is_active to filter active accounts.
-- SQLite ADD COLUMN requires DEFAULT when NOT NULL and rows exist (Pitfall 2).
-- Existing rows: all set to active (1) by default — accounts were implicitly active.

ALTER TABLE users
  ADD COLUMN is_active INTEGER NOT NULL DEFAULT 1;

PRAGMA user_version = 19;
