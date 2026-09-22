-- V044: `devices.inventory_number` uniqueness — introduced for the FIRST
-- TIME (NUM-09/NUM-15/D-18). This column was always free text with no
-- uniqueness constraint of any kind. Before creating a case-insensitive,
-- trimmed, live-only partial UNIQUE INDEX, any pre-existing duplicate must
-- be renamed away or the index creation itself would fail.
--
-- Devices and printers share this same physical column (a printer is a
-- `devices` row with `type_id = 2`) — the index/dedup below covers both.
--
-- Choice of SQL (not a Rust `refinery` migration) — Claude's Discretion
-- (D-18): this project's `crates/trackly-infra/src/db/migrations.rs` embeds
-- migrations via `embed_migrations!("../../migrations")`, and every
-- migration in that directory today is a plain `.sql` file discovered by
-- the `V{n}__{name}.sql` naming convention. Introducing the project's FIRST
-- Rust-based migration would mean verifying, for the first time, that this
-- refinery version's directory-scanning macro correctly mixes `.rs` and
-- `.sql` migration files in one embedded set — an unverified, higher-risk
-- path for a one-time, data-critical dedup rename, with no existing
-- precedent anywhere in this codebase to copy from. SQLite's window
-- functions (`ROW_NUMBER() OVER (...)`) are already used in this same
-- directory's CTEs (see V015/V037/V039) and are fully sufficient to express
-- ONE continuous sequence across ALL duplicate groups (not restarting per
-- group) — see the temp-table staging below.
--
-- ## Case-insensitivity pitfall (T-40.2-08, discovered during this plan's
-- own execution, not anticipated by the plan text)
--
-- SQLite's BUILT-IN `LOWER()`/`UPPER()` fold ONLY ASCII by default — this is
-- a documented SQLite limitation (full Unicode case folding needs the ICU
-- extension, which the `bundled` amalgamation this project ships does NOT
-- include). Empirically verified: `sqlite3 :memory: "SELECT
-- LOWER('ОРГ-00-000007')"` returns the string UNCHANGED — Cyrillic `О` is
-- NOT folded to `о`. This app's entire UI/data is Russian — a plain
-- `LOWER(TRIM(inventory_number))` expression (what a literal reading of
-- D-08 might suggest) would be case-SENSITIVE for exactly the alphabet this
-- app actually uses, silently failing the "NUM-15 same number, different
-- case" acceptance scenario.
--
-- Fix chosen: a pure-SQL Cyrillic case-fold — `LOWER()` first (correctly
-- folds ASCII, e.g. Latin-script inventory numbers), then a fixed chain of
-- 33 `REPLACE()` calls mapping each Cyrillic uppercase letter (А-Я + Ё) to
-- its lowercase counterpart. This is safe because Cyrillic upper/lowercase
-- pairs are always a 1:1 single-codepoint mapping (no expansions, unlike
-- e.g. German ß) and none of the target (lowercase) characters collide with
-- any later REPLACE's search pattern, so chain order does not matter.
--
-- A Rust-registered custom SQL scalar function (`create_scalar_function`
-- wrapping `str::to_lowercase()`) was considered and REJECTED: it would need
-- to be registered on EVERY connection that ever opens this database file
-- BEFORE this migration runs (the migration-runner connection, both
-- writer/reader pools, and every test helper across the whole test suite) —
-- an unbounded, easy-to-miss blast radius for a feature a pure-SQL
-- expression already covers completely for this app's ONLY target alphabet
-- (Russian, v1 scope — see CLAUDE.md "только русский в v1"). The exact same
-- 33-REPLACE expression is used BOTH in the dedup grouping key below AND in
-- the final index, so the two are guaranteed consistent with each other.
--
-- Staging via a TEMP TABLE (not a bare `UPDATE ... SET x = (SELECT ...
-- WHERE id = table.id)`) avoids any ambiguity about whether a self-
-- referencing correlated subquery inside an `UPDATE` sees pre- or
-- post-update values row-by-row — the original trimmed number and the
-- assigned sequence number are captured ONCE, before any row is mutated.
CREATE TEMP TABLE _v044_dedup_renames AS
WITH dedup_key AS (
  -- Live rows only, non-empty after trim, Cyrillic-aware case-insensitive
  -- grouping key — matches `NumberTemplateService::is_occupied`'s own
  -- T-40.2-08 semantics (Rust `.to_lowercase()`, full Unicode folding).
  SELECT id,
         TRIM(inventory_number) AS original_number,
         REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(
           LOWER(TRIM(inventory_number)),
           'А', 'а'), 'Б', 'б'), 'В', 'в'), 'Г', 'г'), 'Д', 'д'), 'Е', 'е'), 'Ж', 'ж'), 'З', 'з'),
           'И', 'и'), 'Й', 'й'), 'К', 'к'), 'Л', 'л'), 'М', 'м'), 'Н', 'н'), 'О', 'о'), 'П', 'п'),
           'Р', 'р'), 'С', 'с'), 'Т', 'т'), 'У', 'у'), 'Ф', 'ф'), 'Х', 'х'), 'Ц', 'ц'), 'Ч', 'ч'),
           'Ш', 'ш'), 'Щ', 'щ'), 'Ъ', 'ъ'), 'Ы', 'ы'), 'Ь', 'ь'), 'Э', 'э'), 'Ю', 'ю'), 'Я', 'я'),
           'Ё', 'ё') AS key_val
    FROM devices
   WHERE deleted_at_utc IS NULL
     AND inventory_number IS NOT NULL
     AND TRIM(inventory_number) <> ''
),
group_stats AS (
  -- Only groups with more than one live member are duplicates. The
  -- earliest member (MIN(id)) keeps its number untouched (SPEC NUM-15).
  SELECT key_val, MIN(id) AS first_id
    FROM dedup_key
   GROUP BY key_val
  HAVING COUNT(*) > 1
)
-- `dup_seq` is ONE continuous sequence across ALL duplicate groups, ordered
-- first by each group's earliest id (so groups are processed in the order
-- they first appear), then by id within a group — the literal SPEC NUM-15
-- example (id 3/8/12 sharing "ОРГ-00-000007") produces id 8 -> dup_seq 1,
-- id 12 -> dup_seq 2, id 3 excluded entirely (kept as-is).
SELECT dk.id                                                AS id,
       dk.original_number                                   AS original_number,
       ROW_NUMBER() OVER (ORDER BY gs.first_id, dk.id)       AS dup_seq
  FROM dedup_key dk
  JOIN group_stats gs ON gs.key_val = dk.key_val
 WHERE dk.id <> gs.first_id;

-- Rename every non-earliest duplicate to `ДУБЛЬ-NNNNNN (<исходный>)`, width 6
-- (SPEC NUM-15's exact format), reading ONLY from the staged snapshot above.
UPDATE devices
   SET inventory_number =
         'ДУБЛЬ-' || printf('%06d', (
           SELECT dup_seq FROM _v044_dedup_renames r WHERE r.id = devices.id
         )) || ' (' || (
           SELECT original_number FROM _v044_dedup_renames r WHERE r.id = devices.id
         ) || ')'
 WHERE id IN (SELECT id FROM _v044_dedup_renames);

-- D-18: one audit_log row per renamed device, `user_id = NULL` (system-
-- initiated, not a user action), `action = 'custom:inventory_number_dedup'`,
-- `payload_json` carries both the old and new number so an administrator can
-- recover the original value from the existing audit trail without a
-- separate recovery mechanism.
INSERT INTO audit_log
  (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc)
SELECT
  'device',
  r.id,
  'custom:inventory_number_dedup',
  NULL,
  NULL,
  NULL,
  json_object('old_number', r.original_number, 'new_number', d.inventory_number),
  CAST(strftime('%s', 'now') AS INTEGER)
FROM _v044_dedup_renames r
JOIN devices d ON d.id = r.id;

DROP TABLE _v044_dedup_renames;

-- BE-WR-05: blank numbers mean "no number". The dedup above skips them
-- (`TRIM(...) <> ''`), so two live devices holding '' or '  ' (possible via
-- the old `COALESCE(?3, inventory_number)` update path with an empty string
-- from an HTTP client) would make the UNIQUE INDEX below fail and the app
-- refuse to start. Normalise them to NULL first; the index also excludes
-- blanks explicitly so a stray '' written later can never collide either.
UPDATE devices
   SET inventory_number = NULL
 WHERE inventory_number IS NOT NULL AND TRIM(inventory_number) = '';

-- Case-insensitive (Cyrillic-aware, see above), trimmed, live-only partial
-- UNIQUE INDEX — the actual NUM-09 enforcement. Expression-based (unlike
-- `cartridges.code`'s case-preserving `idx_cartridges_code_live`, D-08
-- requires case-insensitivity here) — the SAME 33-REPLACE expression as the
-- dedup grouping key above, so the index cannot disagree with what was just
-- deduplicated.
CREATE UNIQUE INDEX idx_devices_inventory_number_live
  ON devices (
    REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(
      LOWER(TRIM(inventory_number)),
      'А', 'а'), 'Б', 'б'), 'В', 'в'), 'Г', 'г'), 'Д', 'д'), 'Е', 'е'), 'Ж', 'ж'), 'З', 'з'),
      'И', 'и'), 'Й', 'й'), 'К', 'к'), 'Л', 'л'), 'М', 'м'), 'Н', 'н'), 'О', 'о'), 'П', 'п'),
      'Р', 'р'), 'С', 'с'), 'Т', 'т'), 'У', 'у'), 'Ф', 'ф'), 'Х', 'х'), 'Ц', 'ц'), 'Ч', 'ч'),
      'Ш', 'ш'), 'Щ', 'щ'), 'Ъ', 'ъ'), 'Ы', 'ы'), 'Ь', 'ь'), 'Э', 'э'), 'Ю', 'ю'), 'Я', 'я'),
      'Ё', 'ё')
  )
  WHERE deleted_at_utc IS NULL AND inventory_number IS NOT NULL
    AND TRIM(inventory_number) <> '';

PRAGMA user_version = 44;
