-- V040: place-movement history journal (Phase 40, HST-01).
--
-- New standalone table, NOT a view/query over `audit_log` (D-01) — the
-- HST-04 two-filter report (period + place, on either side of the move)
-- needs to run plain indexed WHERE clauses against `from_place_id` /
-- `to_place_id` / `created_at_utc`; `audit_log` stores generic before/after
-- JSON blobs that would force a JSON-parse per row to answer the same
-- question. A dedicated table with its own indexes is the only shape that
-- keeps that report cheap at scale (up to 5000 devices).
--
-- Append-only shape, mirrors `migrations/V008__audit_log.sql` (D-Schema-03
-- precedent), NOT the `standard4` soft-delete convention used by `places`
-- (V037) or most editable entities: NO `deleted_at_utc`, NO `version`. This
-- is a journal — rows are written once and never edited or (outside the
-- act-undo path below) deleted.
--
-- D-02: this migration performs NO backfill from `audit_log`. The table is
-- empty immediately after upgrade — history starts accumulating from the
-- first write-site call in a later plan, it is not reconstructed from past
-- activity.
--
-- Column shapes:
--   - `entity_type` / `source` (D-07/D-21): bare `TEXT NOT NULL`, no SQL
--     `CHECK` constraint enumerating the token set. This mirrors the
--     existing `places.kind`-vs-`path_variant_override` split precedent
--     (V037/V039): `entity_type` stores only 'device'|'cartridge' (D-21 —
--     a printer is recorded as 'device', it has no separate token) and is
--     effectively closed, so it could take a CHECK; `source` stores
--     'manual'|'act'|'map'|'workstation', two of which (map, workstation)
--     are not written by any code that exists yet. Per Pitfall 6 in
--     40-RESEARCH.md (IN-01: a strict SQL/Rust parse on an evolving token
--     column has already crashed a whole screen once), validation for BOTH
--     columns is deliberately Rust-side only, via
--     `MovementSource::from_str_lenient` / `MovementEntityKind::from_str_lenient`
--     (crates/trackly-core/src/domain/place_movements.rs, Plan 40-01 Task 2)
--     — never a SQL CHECK, so an unrecognized value degrades safely at read
--     time instead of being rejected at write time or crashing a report.
--   - `note` (D-08): optional, `TEXT NULL`.
--   - `act_id` (D-03): `NULL` for movements not caused by an act; the undo
--     path removes an act's movement rows via
--     `DELETE FROM place_movements WHERE act_id = ?`, which is why this
--     column gets its own partial index below.
--   - `user_id` / `actor_name_snapshot` (D-09/D-11): `user_id NULL` means a
--     system-initiated move (no human actor). `actor_name_snapshot` freezes
--     the ФИО at write time — a login-reuse-safe snapshot, so a movement
--     row's displayed actor never silently changes if the `users` row is
--     later reassigned to a different person or the user is deleted.

CREATE TABLE place_movements (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type         TEXT    NOT NULL,  -- 'device' | 'cartridge' (D-21: printer stored as 'device')
  entity_id           INTEGER NOT NULL,
  from_place_id       INTEGER NOT NULL REFERENCES places(id) ON DELETE RESTRICT,
  from_place_path     TEXT    NOT NULL,
  to_place_id         INTEGER NOT NULL REFERENCES places(id) ON DELETE RESTRICT,
  to_place_path       TEXT    NOT NULL,
  source              TEXT    NOT NULL,  -- 'manual' | 'act' | 'map' | 'workstation' (D-07, no CHECK)
  note                TEXT    NULL,      -- optional (D-08)
  act_id              INTEGER NULL REFERENCES acts(id) ON DELETE SET NULL,
  user_id             INTEGER NULL REFERENCES users(id) ON DELETE SET NULL,  -- NULL = система (D-09/D-11)
  actor_name_snapshot TEXT    NULL,      -- ФИО at write time, login-reuse-safe (D-09)
  created_at_utc      INTEGER NOT NULL
);

-- HST-02 timeline query: all movements of one entity, newest first.
CREATE INDEX idx_place_movements_entity
  ON place_movements(entity_type, entity_id, created_at_utc DESC);

-- HST-04 period filter.
CREATE INDEX idx_place_movements_created
  ON place_movements(created_at_utc);

-- HST-04 «Откуда» filter.
CREATE INDEX idx_place_movements_from_place
  ON place_movements(from_place_id);

-- HST-04 «Куда» filter.
CREATE INDEX idx_place_movements_to_place
  ON place_movements(to_place_id);

-- D-03: act-undo path deletes all movement rows tied to an act
-- (`DELETE FROM place_movements WHERE act_id = ?`). Partial index — most
-- rows have `act_id IS NULL` (manual/map/workstation moves), so indexing
-- only the act-linked subset keeps this small.
CREATE INDEX idx_place_movements_act
  ON place_movements(act_id) WHERE act_id IS NOT NULL;

PRAGMA user_version = 40;
