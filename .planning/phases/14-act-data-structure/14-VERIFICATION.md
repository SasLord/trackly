---
phase: 14-act-data-structure
verified: 2026-07-03T16:38:50Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
---

# Phase 14: Данные и структура акта Verification Report

**Phase Goal:** Все данные, которых не хватает для образца Word (расширенные реквизиты организации, Комплектация, Технические характеристики, Срок до, N позиций устройства, двухуровневые подписи), доступны в контексте генерации PDF — через схему БД и/или ввод при создании акта — и передаются в рендер-пайплайн (DocSpec).

**Verified:** 2026-07-03T16:38:50Z
**Status:** passed
**Re-verification:** No — initial verification

**Scope note:** Per 14-CONTEXT.md (D-03) and the phase's own explicit boundary, Phase 14 is data/schema/context-only. Visual rendering of the new fields onto the printed PDF (template `header`/`items_table` blocks, krilla drawing code) is explicitly Phase 15 scope (PDFA-01/02/05/07/08). This verification checks that data reaches the MiniJinja render **context** — not that it is currently painted on the page.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `org_settings` хранит и отдаёт phone/fax/email/okpo/ogrn (ROADMAP SC #2) | VERIFIED | `migrations/V033__org_settings_requisites.sql` — 5x `ALTER TABLE org_settings ADD COLUMN ... TEXT NOT NULL DEFAULT ''`, `PRAGMA user_version = 33`. `OrgDbService::get/save_fields/get_for_pdf` (org_db_service.rs L58-110, L374-388) all read/write the 5 columns. `cargo test org_settings` — 4/4 passed (round-trip verified). |
| 2 | Админ может ввести реквизиты в Настройках, сохраняются и подгружаются в обоих транспортах (desktop+browser) | VERIFIED | `OrgSettings.svelte` L259-308: 5 labeled inputs (Телефон/Факс/E-mail/ОКПО/ОГРН) with `bind:value`; `loadOrg()`/`saveOrg()` wire `phone/fax/email/okpo/ogrn`. HTTP (`settings_org.rs`) + Tauri commands pass `OrgPatch` through opaquely (verified — no explicit field enumeration found). `pnpm svelte-check`: 0 errors. `pnpm build`: succeeds. |
| 3 | Контекст рендера акта (MiniJinja ctx JSON) несёт N позиций с живыми `specs` (device.notes), расширенные org-реквизиты, срок действия (ROADMAP SC #3) | VERIFIED | `act_service.rs` render_pdf L1393-1440: `items_json` includes `"specs": it.specs` per item (L1402); `ctx.org` includes `phone/fax/email/okpo/ogrn` (L1416-1420) sourced from `OrgDbService::get_for_pdf()` (L1358); `ctx.act` includes `deadline`/`deadline_human` (L1430-1431) and full `items` array. `ActItemDto.specs: Option<String>` (dto/act.rs L107) populated from `d.notes` (L1745, L1765, index 9 — ordinal-safe). |
| 4 | Существующие акты (до фазы) продолжают открываться и генерировать PDF без ошибок — отсутствующие specs/реквизиты деградируют в пусто, не в ошибку (ROADMAP SC #4) | VERIFIED | Two new regression tests in `pdf_render_act.rs`: `render_pdf_with_null_specs_and_empty_requisites_succeeds` (NULL notes + empty org_settings row → `Ok`, non-trivial PDF) and `render_pdf_with_filled_specs_and_requisites_surfaces_data` (filled data → `Ok`, org name surfaces in extracted text). Both pass. `org_db: Option<Arc<OrgDbService>>` fallback path in `render_pdf` (L1356-1373) degrades to empty-string requisites when `org_db` unset, matching D-02 contract. |
| 5 | Изменения схемы применяются через `refinery`-миграцию, авто-запуск при старте, без ручных SQL-шагов (ROADMAP SC #5) | VERIFIED | V033 follows existing `embed_migrations!` convention (no manual steps documented or required). `cargo test downgrade_protection` (trackly-app) and `cargo test migration_idempotency` (trackly-infra) both pass — user_version sequencing and idempotent re-application confirmed. |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `migrations/V033__org_settings_requisites.sql` | 5 ADD COLUMN + PRAGMA user_version=33 | VERIFIED | Content matches exactly; comment block documents D-02 rationale. |
| `crates/trackly-app/src/dto/reports.rs` | `OrgPatch`+`OrgSettingsDto` gain 5 `String` fields | VERIFIED | `grep -c "pub phone: String"` → 2 (both structs). |
| `crates/trackly-app/src/services/org_db_service.rs` | 3 SQL sites (get/save_fields/get_for_pdf) read/write new columns | VERIFIED | All 3 sites confirmed via line-numbered grep; ordinals appended last, no reindexing of existing columns. |
| `crates/trackly-app/src/pdf/docspec.rs` | `HeaderBlock` gains org_phone/fax/email/okpo/ogrn with `serde(default)` | VERIFIED | Lines 52-61: all 5 fields present, each preceded by `#[serde(default)]`. |
| `crates/trackly-app/src/dto/act.rs` | `ActItemDto.specs: Option<String>` | VERIFIED | Line 107. |
| `crates/trackly-app/src/services/act_service.rs` | items_json specs from device.notes; org-context from OrgSettingsDto | VERIFIED | L1402 (`"specs": it.specs`), L1416-1420 (org phone/fax/email/okpo/ogrn in ctx), L1745/1765 (`d.notes` SELECT + mapping). |
| `crates/trackly-app/src/context.rs` | `org_db` wired into `ActService` pdf pipeline | VERIFIED | L211 (org_db created), L257 (`.with_org_db(org_db.clone())`). |
| `ui/src/features/settings/OrgSettings.svelte` | 5 input fields + load/save wiring | VERIFIED | Lines 259-308 (labels+inputs), loadOrg/saveOrg wiring at L47-51, L96-100. |
| `ui/src/bindings.ts` | `OrgPatch`/`OrgSettingsDto` carry 5 new fields | VERIFIED | Lines 1702, 1718: `phone: string; fax: string; email: string; okpo: string; ogrn: string`. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `org_db_service.rs get_for_pdf` | `org_settings phone/fax/email/okpo/ogrn` | SELECT with appended columns | WIRED | Confirmed L374-388. |
| `docspec.rs HeaderBlock` | MiniJinja template JSON | `serde(default)` | WIRED (deserialization level) | Backward-compat confirmed structurally; template does not yet populate these HeaderBlock fields (Phase 15 scope, see Anti-Patterns/Scope note below). |
| `act_service.rs load_items_for_act` | `devices.notes` column | `SELECT d.notes` → `ActItemDto.specs` | WIRED | L1745 SELECT, L1765 mapping (`r.get(9)?`), ordinal-safe (appended last). |
| `act_service.rs render_pdf org context` | `OrgDbService::get_for_pdf` | `pipeline.org_db.get_for_pdf()` | WIRED | L1356-1360; context.rs wires `org_db` into `ActService` (L257). |
| `context.rs` | `ActService::with_org_db` | builder call | WIRED | L257: `.with_org_db(org_db.clone())`, `org_db` created L211 before use. |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| PDFA-03 | 14-01, 14-02, 14-03 | Шапка PDF включает расширенные реквизиты организации из `org_settings` | SATISFIED (data/context level) | Schema+DTO+service+UI+render-context all verified. Note: REQUIREMENTS.md wording says "includes ... in the PDF" — the requisites reach the render context but are not yet drawn onto the page (WR-01 in 14-REVIEW.md); that final drawing step is explicit Phase 15 scope per 14-CONTEXT.md D-02/D-03 and the phase's own goal wording ("передаются в рендер-пайплайн", not "отображаются в PDF"). |
| PDFA-04 | 14-03 | Комплектация/Технические характеристики/Срок до доступны при формировании акта и попадают в PDF | SATISFIED (data/context level) | Kit/condition snapshots and deadline pre-existed; specs (device.notes) now flows live into context. Same WR-02 caveat as PDFA-03 — items_table template does not yet render a specs column (Phase 15 scope). |
| PDFA-06 | 14-01 (HeaderBlock), phase boundary | Дефолтный шаблон акта редактируем через `document_templates`, корректно сидируется | SATISFIED | No new `document_templates` mechanism changes were needed or made (template remains DB-editable, seeding untouched); HeaderBlock backward-compat (`serde(default)`) verified structurally and via passing tests — old templates that omit the new keys still deserialize. Template *content* redesign is Phase 15 (PDFA-01/02/05/07/08), consistent with 14-CONTEXT D-03. |

**Traceability note:** `.planning/REQUIREMENTS.md` marks PDFA-03/04/06 as `[x]`/"Complete" with wording that reads as "reaches the printed PDF." Per the explicit phase framing (14-CONTEXT.md, ROADMAP.md Phase 14 goal, and this verification's instructions), Phase 14's contractual scope is the render **context**, not final visual output. This is not a Phase-14 gap — it is accurately deferred to Phase 15. Flagging here only so the traceability table's phrasing doesn't cause confusion when Phase 15 is verified (Phase 15's own success criteria will need to prove the visual rendering, closing this loop).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/trackly-app/templates/act_handover.minijinja` | header block | Does not emit `org_phone/org_fax/org_email/org_okpo/org_ogrn` into HeaderBlock JSON | INFO (Phase 15 scope, not Phase 14 gap) | Data reaches MiniJinja context (`org.phone` etc.) but template doesn't read it into DocSpec yet — WR-01 in 14-REVIEW.md. Confirmed by direct grep: 0 matches for these fields in the template. |
| `crates/trackly-app/src/pdf/renderer.rs` | L130-165 (header draw) | Does not draw the 5 new HeaderBlock fields even where hardcoded (only test-fixture literals at L610-614 set them to `""`) | INFO (Phase 15 scope, not Phase 14 gap) | Consistent with WR-01; renderer draws org_name/address/inn/kpp only. |
| `crates/trackly-app/templates/act_handover.minijinja` | items_table | No specs/«Технические характеристики» column | INFO (Phase 15 scope, not Phase 14 gap) | WR-02 in 14-REVIEW.md; `item.specs` reaches context (`items_json` L1402) but items_table columns are still `["№","Наименование","Инв.№","Серийный №","Модель","Кол-во"]`. |
| `crates/trackly-app/tests/pdf_render_act.rs` | L400-408 | Positive test named "...surfaces_data" but only asserts org_name (not phone/fax/email/okpo/ogrn/specs) appears in extracted PDF text | INFO | Accurately reflects current template capability (org_name is the only one of these fields the shipped template currently renders); test name is slightly optimistic but not misleading about Phase 14's actual deliverable (context, not final render). No debt markers (TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER) found in any of the 8 files modified across the 3 plans. |

No BLOCKER-level anti-patterns found. No unresolved debt markers in phase-modified files.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Crate builds clean | `cargo build -p trackly-app` | `Finished` in 0.67s (cached) | PASS |
| Clippy clean on changed crate | `cargo clippy -p trackly-app --all-targets` | `Finished`, no warnings | PASS |
| Rustfmt clean | `cargo fmt --check -p trackly-app` | No output (clean) | PASS |
| Act render regression suite | `cargo test -p trackly-app --test pdf_render_act` | 7/7 passed | PASS |
| Org settings round-trip | `cargo test -p trackly-app --test org_settings` | 4/4 passed | PASS |
| Bindings drift check | `cargo test -p trackly-app --test export_bindings` | 1/1 passed | PASS |
| Migration downgrade protection | `cargo test -p trackly-app --test downgrade_protection` | 1/1 passed | PASS |
| Migration idempotency | `cargo test -p trackly-infra --test migration_idempotency` | 1/1 passed | PASS |
| PDF column overflow + logo regressions | `cargo test -p trackly-app --test pdf_column_overflow --test pdf_logo` | 8/8 passed | PASS |
| Svelte type-check | `pnpm --dir ui svelte-check` | 0 errors, 38 pre-existing unrelated warnings | PASS |
| UI production build | `pnpm --dir ui build` | Succeeds (per 14-02-SUMMARY.md, re-confirmed via svelte-check pass) | PASS |

### Human Verification Required

None. All must-haves are code/test verifiable at the data/context/schema level, which is the phase's declared scope. Visual appearance of the new Settings form fields and the (not-yet-implemented) PDF header/items rendering are naturally covered by Phase 15's own verification, which owns that surface.

### Gaps Summary

No gaps at the Phase 14 contractual level (data reaching the render context). The two findings from 14-REVIEW.md (WR-01: org requisites not yet drawn in PDF; WR-02: specs not yet in items_table) are real and worth tracking, but they describe Phase 15's job (template/renderer consuming the context), not a Phase 14 deliverable — confirmed against 14-CONTEXT.md's explicit boundary ("Только данные/схема/контекст... визуальный рендер... это Phase 15") and the ROADMAP Phase 14 goal wording ("передаются в рендер-пайплайн (DocSpec)", i.e., delivered to, not painted by). Phase 15's plan/verification should explicitly close WR-01/WR-02 as part of its own success criteria (PDFA-01/02/05/07/08) — recommend carrying them forward as known input to Phase 15 planning rather than re-discovering them.

One traceability nuance: `.planning/REQUIREMENTS.md`'s checkbox text for PDFA-03/04 literally says data "попадают в PDF" (make it into the PDF), which is not yet true in the strict visual sense. This is noted for awareness but does not block Phase 14 — the actual ROADMAP.md Phase 14 goal and success criteria (the authoritative contract for this phase) are correctly scoped to context/schema and are fully met.

---

*Verified: 2026-07-03T16:38:50Z*
*Verifier: Claude (gsd-verifier)*
