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
-- V015. This UPDATE is a ONE-TIME historical backfill. It is safe ONLY
-- because refinery never re-runs an already-applied migration — it is
-- NOT safe to run manually after Phase 22 ships, because
-- `ActService::update_return` (D-05) intentionally lets a return's
-- handover_date_utc diverge from its created_at_utc (the user can edit
-- «Дата возврата» independently of when the return row was created). A
-- manual re-run of this exact UPDATE statement would silently overwrite
-- every user-edited «Дата возврата» back to the row's creation timestamp.

UPDATE acts SET handover_date_utc = created_at_utc WHERE act_type = 'return';

PRAGMA user_version = 34;
