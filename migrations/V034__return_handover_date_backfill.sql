-- V034: Backfill return rows' handover_date_utc (Phase 22, ACT-03, D-08).
--
-- Before this phase, a return act's handover_date_utc was a copy of its
-- PARENT handover act's own handover_date_utc (do_return's old write-site,
-- act_service.rs:1232) — it never meant "when this return happened".
--
-- After this phase, handover_date_utc on a return row means «Дата
-- возврата» (when the devices were actually returned): do_return's
-- write-site now persists the payload's own entered date, and the edit
-- path (ActService::update_return) lets it be changed independently of the
-- parent. The only available historical signal for "when it was actually
-- returned" for EXISTING return rows is the row's own created_at_utc (when
-- the return act itself was inserted) — no other timestamp on a return row
-- captures this.
--
-- No schema change — the handover_date_utc column already exists since
-- V015. Safe to run once (refinery never re-runs applied migrations); the
-- UPDATE is naturally idempotent (re-running it is a no-op once
-- handover_date_utc already equals created_at_utc for return rows).

UPDATE acts SET handover_date_utc = created_at_utc WHERE act_type = 'return';

PRAGMA user_version = 34;
