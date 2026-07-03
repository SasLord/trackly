---
phase: 14-act-data-structure
plan: 03
subsystem: pdf-render
tags: [rusqlite, minijinja, docspec, org-settings, act-render, specta]

# Dependency graph
requires:
  - phase: 14-act-data-structure (plan 01)
    provides: "org_settings extended with phone/fax/email/okpo/ogrn (V033); OrgSettingsDto/OrgDbService::get_for_pdf carry them"
  - phase: 14-act-data-structure (plan 02)
    provides: "Settings UI writes the 5 requisites into org_settings via settings_save_org_fields"
provides:
  - "ActItemDto.specs (live devices.notes) reaches the act_handover render context — no longer hardcoded Null"
  - "act_service.rs render_pdf reads org requisites from OrgDbService::get_for_pdf() (org_settings), not org.json"
  - "ActService.with_org_db() builder wires OrgDbService into the PDF pipeline (Option-aware, backward-compat fallback to legacy org.json values)"
  - "Backward-compat proven: NULL specs + empty org_settings requisites render Ok, non-empty PDF"
affects: [15-render-fidelity]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Option-aware PdfPipelineRefs field (org_db) with a fallback branch inside render_pdf, so pre-existing test fixtures / helper constructions that don't call the new builder still compile and degrade gracefully instead of erroring"
    - "New DTO field appended as the LAST SELECT column (d.notes at index 9) to preserve existing ordinal r.get(N) positions in load_items_for_act"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/dto/act.rs
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/tests/pdf_render_act.rs

key-decisions:
  - "org_db wired via a separate with_org_db() builder method, not folded into with_pdf_pipeline()'s existing 3-arg signature — avoids breaking every pre-existing 3-arg call site across the test suite; org_db is genuinely Optional at the type level (PdfPipelineRefs.org_db: Option<&Arc<OrgDbService>>)"
  - "render_pdf's org-context fallback (when org_db is None) reads the legacy OrganizationService (org.json) name/inn/kpp/address and leaves the 5 new requisites as empty strings — matches D-02's 'missing requisites degrade to blank' contract even for callers who haven't wired org_db yet"
  - "render_acceptance_pdf intentionally left untouched (D-03: phase semantics scoped to act_handover only); confirmed it still compiles and its existing test still passes"

patterns-established: []

requirements-completed: [PDFA-04, PDFA-06]

# Metrics
duration: 30min
completed: 2026-07-03
---

# Phase 14 Plan 03: Act render context — live specs + org_settings requisites Summary

**act_handover PDF render context now carries live `devices.notes` as `item.specs` (was hardcoded `Null`) and reads org requisites (name/inn/kpp/address/phone/fax/email/okpo/ogrn) from `OrgDbService::get_for_pdf()` instead of `org.json`, with a backward-compat fallback and two new regression tests proving NULL/empty data degrades to a valid PDF rather than an error.**

## Performance

- **Duration:** 30 min
- **Started:** 2026-07-03T15:06:00Z (approx, first Read call)
- **Completed:** 2026-07-03T15:36:32Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments

- `ActItemDto` gained `specs: Option<String>`; `load_items_for_act`'s SELECT now joins `d.notes` (last column, ordinal-safe) and maps it into the DTO — the single real data-gap this phase set out to close (D-01)
- `render_pdf`'s `items_json` no longer hardcodes `"specs": Null` — it emits the live device value, degrading to `null`/`None` (not an error) when `devices.notes` is unset
- `ActService` gained an `org_db: Option<Arc<OrgDbService>>` field + `with_org_db()` builder; `PdfPipelineRefs` carries it as `Option<&Arc<OrgDbService>>` so pre-existing 3-arg `with_pdf_pipeline(...)` call sites (test fixtures) keep compiling unchanged
- `render_pdf`'s org context now sources name/inn/kpp/address **and** the 5 new requisites (phone/fax/email/okpo/ogrn) from `OrgDbService::get_for_pdf()` (D-05) — the same store Settings UI writes to — with a fallback to legacy `org.json` values (empty requisites) when `org_db` isn't wired
- `context.rs` wires the already-constructed `org_db` into `ActService` via the new `.with_org_db(org_db.clone())` call, alongside the existing `with_pdf_pipeline(...)`
- Two new integration tests in `pdf_render_act.rs` prove backward-compat end-to-end: (1) NULL `devices.notes` + all-default-empty `org_settings` row → `render_pdf` returns `Ok` with a valid, non-trivial PDF; (2) filled `devices.notes` + `org_settings` requisites saved via the real `OrgDbService::save_fields` path → the org name surfaces in the extracted PDF text, proving the data actually flows through the context (not just "doesn't crash")
- Full render/PDF regression suite (85+ integration test binaries filtered by `render`) green, 0 failures; `export_bindings` confirms `ui/src/bindings.ts` picked up `ActItemDto.specs` automatically; `clippy -D warnings` clean on both lib and tests; `cargo fmt --check` clean

## Task Commits

Each task was committed atomically:

1. **Task 1: specs↔notes — расхардкодить Технические характеристики (D-01)** - `8d61318` (feat)
2. **Task 2: Переключить источник org-реквизитов act-рендера на org_settings (D-05) + org-контекст** - `2d44b85` (feat)
3. **Task 3: Backward-compat — старый акт генерирует PDF, регресс-тесты и bindings** - `2aa0698` (test)

**Plan metadata:** (this commit, pending)

## Files Created/Modified

- `crates/trackly-app/src/dto/act.rs` - `ActItemDto` gains `pub specs: Option<String>` (live device value, not a snapshot)
- `crates/trackly-app/src/services/act_service.rs` - `load_items_for_act` SELECT extended with `d.notes` (last column); `items_json` emits `it.specs`; `ActService` gains `org_db` field + `with_org_db()` builder; `PdfPipelineRefs` gains `org_db: Option<&Arc<OrgDbService>>`; `render_pdf`'s org-context block now reads `OrgDbService::get_for_pdf()` with a legacy-`org.json` fallback when `org_db` is `None`
- `crates/trackly-app/src/context.rs` - Wires the existing `org_db` instance into `ActService` via `.with_org_db(org_db.clone())`
- `crates/trackly-app/tests/pdf_render_act.rs` - New `make_full_pipeline_with_org_db()` test helper + 2 new tests (`render_pdf_with_null_specs_and_empty_requisites_succeeds`, `render_pdf_with_filled_specs_and_requisites_surfaces_data`)

## Decisions Made

- `with_org_db()` kept as a separate builder method rather than extending `with_pdf_pipeline()`'s signature, to avoid touching every existing 3-arg call site in the test suite (multiple test files construct `ActService` via the old signature) — `org_db` is genuinely `Option` at the type level, and `pdf_pipeline()` passes it through as `self.org_db.as_ref()` without erroring when absent
- `render_pdf`'s fallback branch (no `org_db` wired) reads legacy `org.json` name/inn/kpp/address and defaults the 5 new requisites to empty strings — consistent with D-02's "missing requisites degrade to blank, not error" contract, and keeps pre-existing tests (which don't wire `org_db`) passing unchanged as a genuine backward-compat proof, not just an incidental side effect
- `render_acceptance_pdf` left untouched per D-03 (phase semantics scoped to `act_handover`); confirmed via existing `render_acceptance_pdf_for_device_works` test still passing

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] cargo fmt reformatted a multi-line method chain in act_service.rs**
- **Found during:** Task 3 (running `cargo fmt --check` before committing)
- **Issue:** `pipeline.organization.safe_logo_canonical(&org_legacy).await?` (introduced in Task 2) exceeded the project's line-length wrapping convention
- **Fix:** Ran `cargo fmt -p trackly-app`; the single affected line was reflowed across 4 lines by rustfmt with no logic change
- **Files modified:** crates/trackly-app/src/services/act_service.rs
- **Verification:** `cargo fmt --check -p trackly-app` clean after; `cargo build`/`clippy` unaffected
- **Committed in:** 2aa0698 (Task 3 commit, folded in since Task 3 was the next commit checkpoint after discovering it)

---

**Total deviations:** 1 auto-fixed (Rule 1 — cosmetic formatting fix, no logic change)
**Impact on plan:** No scope creep; purely a formatting correction required to keep `cargo fmt --check` green, which is part of this project's CI gate.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required. This plan is data/context-wiring only (Phase 14 boundary); no new settings, migrations, or user-facing UI were introduced (those were Plan 01/02's scope).

## Next Phase Readiness

- `act_handover` render context now carries everything Phase 15's redesigned template needs: live `item.specs`, and `org.phone`/`org.fax`/`org.email`/`org.okpo`/`org.ogrn` alongside the existing `org.name`/`org.inn`/`org.kpp`/`org.address`. The current shipped `act_handover.minijinja` template does not yet consume `specs` or the 5 new org fields in its `HeaderBlock`/`ItemsTable` output — that wiring is explicitly Phase 15's scope (PDFA-01/02/05/07/08), not a gap left by this plan.
- Phase 14 (`act-data-structure`) is now complete: all 3 plans done. `PDFA-03`, `PDFA-04`, `PDFA-06` requirements are satisfied at the schema/context level; visual rendering fidelity to the Word sample is Phase 15.
- No blockers. `org_db`'s `Option`-aware design means any future test fixture or lightweight `ActService` construction that doesn't need org requisites can continue to omit `with_org_db()` safely.

## Self-Check: PASSED

- FOUND: crates/trackly-app/src/dto/act.rs (specs field present)
- FOUND: crates/trackly-app/src/services/act_service.rs (org_db, get_for_pdf, d.notes all present)
- FOUND: crates/trackly-app/src/context.rs (with_org_db wiring present)
- FOUND: crates/trackly-app/tests/pdf_render_act.rs (2 new tests present, both passing)
- FOUND commit: 8d61318 (Task 1)
- FOUND commit: 2d44b85 (Task 2)
- FOUND commit: 2aa0698 (Task 3)

---
*Phase: 14-act-data-structure*
*Completed: 2026-07-03*
