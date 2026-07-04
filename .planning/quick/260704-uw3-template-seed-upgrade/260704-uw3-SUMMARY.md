---
quick_id: 260704-uw3
slug: template-seed-upgrade
subsystem: templates
tags: [rusqlite, minijinja, document_templates, seed, migration-adjacent]

# Dependency graph
requires:
  - phase: 07 (settings)
    provides: document_templates schema + is_default column (V027), TemplateService
  - phase: 15 (render-word-fidelity)
    provides: rewritten act_handover.minijinja bundled template (Phase 15-02)
provides:
  - seed_defaults_on_startup auto-upgrade branch for stale is_default=1 bundled templates
affects: [template_service, act rendering, any future bundled template rewrite]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Seed-on-startup auto-upgrade: is_default flag as the safety key distinguishing 'safe to auto-sync with bundle' rows from 'user customized, never touch' rows"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/template_service.rs

key-decisions:
  - "Auto-upgrade UPDATE branch mirrors reset_to_default's exact UPDATE shape (same SET/WHERE clause) instead of introducing a new query pattern"
  - "Write only fires when stored body_minijinja differs from the bundled DEFAULT_TEMPLATES body — avoids needless version bump on every app startup"

patterns-established:
  - "Bundled-default drift detection: compare stored body_minijinja against include_str!'d DEFAULT_TEMPLATES at every seed_defaults_on_startup call, gated on is_default=1"

requirements-completed: []

# Metrics
duration: 25min
completed: 2026-07-04
---

# Quick Task 260704-uw3: Auto-upgrade bundled default act templates on existing DBs Summary

**`seed_defaults_on_startup` теперь при каждом запуске сверяет `body_minijinja` активной `is_default=1` записи с текущим встроенным `DEFAULT_TEMPLATES` и обновляет её на месте (version+1), если тело устарело — существующие БД подхватывают правки бандл-шаблона (например, рендер-фикс Phase 15-02) без ручного вмешательства.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-07-04T15:33:00Z (approx, per plan creation timestamp)
- **Completed:** 2026-07-04T15:58:45Z
- **Tasks:** 3 (2 code/test commits + 1 verification-only gate)
- **Files modified:** 1 (`crates/trackly-app/src/services/template_service.rs`)

## Accomplishments

- Диагностирован и закрыт баг: `ActService::render` рендерит через `TemplateService::get_active`, читающий тело шаблона из БД — а `seed_defaults_on_startup` раньше сидировал дефолт только при `active_count == 0`, поэтому все БД, созданные до Phase 15-02, никогда не получали переписанный `act_handover.minijinja` (новые поля Phase 14: Комплектация, Технические характеристики, Срок до, мультиустройство).
- `seed_defaults_on_startup` расширен третьей веткой: для каждого `kind` теперь читаются `(is_default, body_minijinja)` активной записи и логика ветвится на 3 исхода — INSERT (нет активной записи, поведение не изменилось), auto-upgrade UPDATE (`is_default=1` и тело устарело — обновляем `body_minijinja`, `version+1`, зеркалируя UPDATE-форму `reset_to_default`), no-op (`is_default=0`, т.е. пользователь кастомизировал шаблон через `update_body`, либо тело уже совпадает с бандлом).
- 3 новых regression-теста в `template_service.rs`'s `tests`-модуле: bug-репро апгрейда, no-clobber кастомного шаблона, идемпотентность повторных вызовов.
- Обновлён doc-comment модуля (Seed-семантика block) с описанием новой auto-upgrade ветки (D-Templates-Seed-02).

## Task Commits

Each task was committed atomically:

1. **Task 1: Extend `seed_defaults_on_startup` with the upgrade branch** - `20fb879` (fix)
2. **Task 2: Regression tests (3 new `#[tokio::test]` fns)** - `1a7a1d7` (test)
3. **Task 3: Final verification gate** - no separate commit (verification-only task; `cargo test`/`clippy -D warnings`/`fmt --check` all green, no code changes required)

**Plan metadata:** committed together with Task 1 (`20fb879` includes `260704-uw3-PLAN.md`)

## Files Created/Modified

- `crates/trackly-app/src/services/template_service.rs` - `seed_defaults_on_startup` extended with the auto-upgrade branch (fetches `is_default`+`body_minijinja` for the active row, branches INSERT/UPDATE/no-op); module doc-comment updated; 3 new regression tests added to the co-located `tests` module.

## Decisions Made

- Auto-upgrade UPDATE mirrors `reset_to_default`'s exact UPDATE shape (`template_service.rs:196-206` — same SET clause: `body_minijinja`, `is_default=1`, `updated_at_utc`, `version=version+1`; same WHERE clause: `kind=?1 AND is_active=1 AND deleted_at_utc IS NULL`) instead of inventing a new query pattern — keeps the two code paths in sync by construction.
- Write only fires when the stored body genuinely differs from the bundled default — avoids inflating `version` on every restart once already synced (explicit idempotency requirement called out in both the plan and the file's existing doc-comment).

## Deviations from Plan

None — plan executed exactly as written. Task 3 was a pure verification gate (test/clippy/fmt), all three commands exited 0 on the first attempt with no fixes needed.

## Issues Encountered

None — the change compiled cleanly on first attempt, all 6 tests in the `template_service::tests` module (3 new + 3 pre-existing) passed on first run, and the full `trackly-app` test suite (all ~150 integration test binaries), `cargo clippy -p trackly-app -- -D warnings`, and `cargo fmt --check` were all green with no follow-up fixes required.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Existing pre-Phase-15 installations will now auto-upgrade their `act_handover`/`act_acceptance` default templates to the current bundled body on next app startup, provided the admin never customized them via the Template Editor (`is_default=1` preserved).
- No blockers. This closes the "existing DBs never see bundled template rewrites" gap permanently for all future bundled-template updates, not just the Phase 15-02 one.

---
*Quick task: 260704-uw3-template-seed-upgrade*
*Completed: 2026-07-04*

## Self-Check: PASSED

- FOUND: `.planning/quick/260704-uw3-template-seed-upgrade/260704-uw3-SUMMARY.md`
- FOUND: `.planning/quick/260704-uw3-template-seed-upgrade/260704-uw3-PLAN.md`
- FOUND: `crates/trackly-app/src/services/template_service.rs`
- FOUND commit: `20fb879` (fix — auto-upgrade branch)
- FOUND commit: `1a7a1d7` (test — 3 regression tests)
- FOUND commit: `b4d8f10` (docs — this summary)
