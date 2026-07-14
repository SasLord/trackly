---
phase: 20-print-acts-org
verified: 2026-07-14T00:00:00Z
status: passed
score: 8/8 must-haves verified
overrides_applied: 0
---

# Phase 20: Печать актов и организация Verification Report

**Phase Goal:** Печать акта приёма-передачи из раздела Устройства выводит полный организационный контекст в шапке (логотип, название, ИНН, реквизиты), а настройки организации поддерживают безопасный SVG-логотип и вторую строку адреса (address_line2), которые попадают во все печатные шаблоны — включая существующие установки, а не только fresh installs.

**Verified:** 2026-07-14
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `render_acceptance_pdf` sources org data exclusively from `org_db.get_for_pdf()`, not legacy org.json/`read_logo_bytes` (PRN-01/D-02/D-11) | VERIFIED | `act_service.rs:2697-2719` — `match pipeline.org_db { Some(org_db) => org_db.get_for_pdf().await? ... }`; `grep -c read_logo_bytes` → 0 in the whole file |
| 2 | `render_acceptance_pdf` ctx carries full 11-field org requisite set (name/inn/kpp/address/address_line2/phone/fax/email/okpo/ogrn/logo_data_uri), parity with `render_pdf` (PRN-01/D-01/D-03) | VERIFIED | `act_service.rs:2788-2801` — all 11 keys present, matches `render_pdf`'s ctx (lines 2630-2643) field-for-field |
| 3 | `act_acceptance.html` `.requisites` block renders all fields at parity with `act_handover.html` (D-01) | VERIFIED | `templates/act_acceptance.html:104-114` — name/inn+kpp/address/address_line2/phone/fax/email/okpo/ogrn, all present with identical `{% if %}` guard idiom to `act_handover.html` |
| 4 | `address_line2` stored in DB (`org_settings` table), threaded through DTOs, and rendered as a guarded line in all three print templates (ORG-02) | VERIFIED | Migration `V035__org_settings_address_line2.sql` applies `ALTER TABLE ... ADD COLUMN address_line2 TEXT NOT NULL DEFAULT ''`; `OrgPatch`/`OrgSettingsDto` carry the field (`dto/reports.rs:189,224`); all 3 SQL sites in `org_db_service.rs` (get/save_fields/get_for_pdf) read+write it; `act_handover.html:135`, `act_acceptance.html:108`, `report.html:139` all render `{%- if org.address_line2 %}<div>{{ org.address_line2 }}</div>{%- endif %}` |
| 5 | UI exposes «Адрес (2-я строка)» field in Settings, loads/saves via production write path (ORG-02/D-05/D-07) | VERIFIED | `OrgSettings.svelte:262-271` — labeled exactly «Адрес (2-я строка)», wired to interface/state/load/save; `bindings.ts` regenerated with `address_line2` (2 occurrences) |
| 6 | SVG logo with embedded `<script>` embeds ONLY as `data:` URI inside `<img>`, `<script>` never appears literally in rendered HTML (ORG-01/D-08/D-09) | VERIFIED | Regression test `html_svg_logo_with_script_embeds_img_only_no_inline_script` — independently re-run, PASSED. Asserts `!html.contains("<script>")` AND non-vacuous `data:image/svg+xml;base64,` presence AND `<img src="data:image/svg+xml;base64,` presence |
| 7 | Automated tests prove PRN-01 parity and ORG-02 address_line2 rendering end-to-end via production write paths, not hand-built ctx | VERIFIED | `html_acceptance_full_org_parity_with_handover` and `html_report_org_header_shows_address_line2` — both independently re-run, PASSED; use `OrgDbService::save_fields`/`save_logo`, not hand-constructed ctx |
| 8 | Existing installs (act_handover.html/act_acceptance.html/report.html already materialized pre-Phase-20 on disk, untouched by user) receive the new bundled content on next startup — not only fresh installs (D-12) | VERIFIED | `upgrade_untouched_defaults_on_startup` wired into `context.rs` immediately after `materialize_defaults_on_startup`; `KNOWN_LEGACY_DEFAULTS` snapshots verified byte-identical (via `diff` against `git show 8f82339...`) to the pinned pre-Phase-20 commit; 3 regression tests independently re-run, PASSED (legacy→upgraded, customized→untouched, already-current→no-op) |

**Score:** 8/8 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `migrations/V035__org_settings_address_line2.sql` | `ALTER TABLE org_settings ADD COLUMN address_line2 TEXT NOT NULL DEFAULT ''` | VERIFIED | Exact content confirmed; `PRAGMA user_version = 35` present |
| `crates/trackly-app/src/dto/reports.rs` | `OrgPatch.address_line2` + `OrgSettingsDto.address_line2` | VERIFIED | Both fields present (lines 189, 224) |
| `crates/trackly-app/src/services/org_db_service.rs` | get/save_fields/get_for_pdf all read+write address_line2 | VERIFIED | All 3 SQL sites confirmed (lines 58,73,99,113,377,392) |
| `crates/trackly-app/src/services/act_service.rs` | `render_acceptance_pdf` rewritten to org_db parity | VERIFIED | `org_db.get_for_pdf()` called at both render_pdf and render_acceptance_pdf sites; ctx fully expanded |
| `crates/trackly-app/templates/act_acceptance.html` | Full `.requisites` parity block | VERIFIED | All 9 org fields + address_line2 present |
| `crates/trackly-app/templates/act_handover.html`, `report.html` | address_line2 guarded line | VERIFIED | Both present |
| `ui/src/features/settings/OrgSettings.svelte` | address_line2 field wired end-to-end | VERIFIED | Interface/state/load/save/markup all present |
| `ui/src/bindings.ts` | regenerated with address_line2 | VERIFIED | 2 occurrences found (gitignored build artifact, present in working tree) |
| `crates/trackly-app/tests/html_act_render.rs` | PRN-01 parity test + D-09 security test | VERIFIED | Both tests present and passing |
| `crates/trackly-app/tests/fixtures/logo_test_with_script.svg` | malicious SVG fixture | VERIFIED | Contains literal `<script>alert('xss')</script>` |
| `crates/trackly-app/tests/html_report_render.rs` | address_line2 rendering assertion | VERIFIED | `html_report_org_header_shows_address_line2` present and passing |
| `crates/trackly-app/src/pdf/html_templates.rs` | `KNOWN_LEGACY_DEFAULTS` + `upgrade_untouched_defaults_on_startup` | VERIFIED | Both present, fully implemented per D-12 fail-closed decision tree |
| `crates/trackly-app/src/context.rs` | startup wiring for upgrade pass | VERIFIED | Called immediately after `materialize_defaults_on_startup`, same `html_templates_dir` binding |
| `crates/trackly-app/templates/_legacy_defaults/v20/*.html` | byte-for-byte pre-Phase-20 snapshots | VERIFIED | `diff` against `git show 8f82339497fe820f4b4487dc160524ce9da9d002:...` confirms byte-identical for all 3 files |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `act_service.rs::render_acceptance_pdf` | `org_db_service.rs::get_for_pdf` | `org_db.get_for_pdf().await?` | WIRED | Confirmed at line 2699 |
| `act_service.rs::render_acceptance_pdf` ctx | `act_acceptance.html` | `org.phone/fax/email/okpo/ogrn/address_line2` | WIRED | Template consumes all ctx keys, confirmed by passing parity test |
| `OrgSettings.svelte::saveOrg` | `settings_save_org_fields` → `OrgPatch.address_line2` | `apiCall(..., { patch: { ..., address_line2: addressLine2 } })` | WIRED | Confirmed at line 99 |
| `context.rs` | `html_templates.rs::upgrade_untouched_defaults_on_startup` | called right after `materialize_defaults_on_startup(&html_templates_dir)?` | WIRED | Confirmed at context.rs:215/221 |
| `upgrade_untouched_defaults_on_startup` | `KNOWN_LEGACY_DEFAULTS` | byte-comparison against legacy snapshot bodies before overwrite | WIRED | Confirmed in function body (html_templates.rs:144-157); fail-closed path confirmed by passing "customized file untouched" test |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| `act_acceptance.html` `org.*` | `org_dto` (11 fields) | `OrgDbService::get_for_pdf()` → real SQL `SELECT ... FROM org_settings` | Yes — SQL query confirmed, not static stub | FLOWING |
| `act_acceptance.html` `org.logo_data_uri` | `logo_bytes`/`logo_mime` | `org_settings.logo_blob` BLOB, written via authenticated `save_logo` | Yes | FLOWING |
| `OrgSettings.svelte` `addressLine2` | `dto.address_line2` | `settings_get_org` → `OrgDbService::get()` → real SQL SELECT | Yes | FLOWING |
| `html_templates.rs` upgrade decision | on-disk file content vs `KNOWN_LEGACY_DEFAULTS` | real `std::fs::read_to_string` + byte comparison, not a stub | Yes | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `cargo check -p trackly-app --all-targets` compiles cleanly | `cargo check -p trackly-app --all-targets` | Finished, exit 0 | PASS |
| D-12 auto-upgrade unit tests pass independently | `cargo test -p trackly-app --lib pdf::html_templates::tests -- --test-threads=1` | 8/8 passed (3 new: upgrade_replaces_untouched_legacy_default_with_current_bundled_body, upgrade_leaves_user_customized_file_untouched, upgrade_is_noop_when_file_already_current) | PASS |
| PRN-01/ORG-01/ORG-02 regression tests pass independently | `cargo test -p trackly-app --test html_act_render --test html_report_render -- --test-threads=1` | 10/10 + 8/8 passed (incl. html_acceptance_full_org_parity_with_handover, html_svg_logo_with_script_embeds_img_only_no_inline_script, html_report_org_header_shows_address_line2) | PASS |
| Legacy template snapshots are byte-identical to the pinned pre-Phase-20 commit | `diff <(git show 8f82339...:path) _legacy_defaults/v20/path` × 3 | IDENTICAL for all 3 files | PASS |

All spot-checks were independently re-executed by the verifier (not taken from SUMMARY.md claims) and produced the same green results the summaries described.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| PRN-01 | 20-02, 20-03, 20-05, 20-06 | Полный org-контекст в шапке acceptance-печати | SATISFIED | `render_acceptance_pdf` rewritten to org_db parity; template parity confirmed; regression test passing; D-12 delivers to existing installs |
| ORG-01 | 20-05 (D-08/D-09 already-satisfied + regression lock) | SVG-логотип безопасно встраивается, скрипты не исполняются | SATISFIED | img-only `data:` URI embedding confirmed in all 3 templates (unchanged, pre-existing correct behavior); regression test with real `<script>`-bearing SVG payload passes |
| ORG-02 | 20-01, 20-02, 20-03, 20-04, 20-05, 20-06 | Вторая строка адреса, попадающая во все печатные формы | SATISFIED | Migration+DTO+SQL+UI+templates+tests+D-12 upgrade mechanism all confirmed |

No orphaned requirements — all three IDs (PRN-01, ORG-01, ORG-02) declared across plan frontmatter and covered by verified evidence.

**Note on REQUIREMENTS.md checkbox state:** `.planning/REQUIREMENTS.md` shows PRN-01 `[x]` and ORG-02 `[x]` checked (updated in commit `b6672f1`), but ORG-01 remains `[ ]` unchecked and the Traceability table still lists all three as "Planned" rather than "Complete". This is a documentation bookkeeping gap, not a code gap — the codebase evidence for ORG-01 (img-only SVG embedding + D-09 regression test) is fully verified above. Flagged as an informational cleanup item for phase close, not a functional blocker.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `act_service.rs:2545,2696` | WR-03 (code review) | `render_pdf`/`render_acceptance_pdf` unconditionally call `pipeline.organization.read().await?` even though `org_legacy` is only consumed in the unreachable-in-production `None` branch | Info/Warning (pre-existing in render_pdf, newly duplicated into render_acceptance_pdf) | Latent risk: malformed org.json could fail acceptance render even though org_settings is authoritative. Does not affect PRN-01/ORG-02 goal achievement — logo/requisites data is correctly sourced from org_db in the `Some` branch, which is always the production path |
| `org_db_service.rs:132-155` | WR-02 (code review) | `save_logo` validates size/mime before `authorize()` — inconsistent ordering vs sibling mutators | Warning (least-privilege ordering) | Low impact — write is still blocked by the later authorize() call; does not affect ORG-01's core "img-only, no script execution" guarantee |
| `act_service.rs:2572-2579,2724-2731` | WR-01 (code review) | Read-time logo-mime allowlist enforced in `report_service.rs` but not in either act render path | Warning (defense-in-depth gap) | Currently unexploitable — write-side allowlist in `save_logo` is the only mime-write path; does not compromise ORG-01's "script never executes" guarantee, which rests on img-only embedding (verified), not on mime revalidation |

No BLOCKER-level anti-patterns found (code review confirms 0 critical, 3 warnings, 3 info — none touching the core PRN-01/ORG-01/ORG-02 delivery paths verified above). No unresolved `TBD`/`FIXME`/`XXX` debt markers found in phase-modified files.

### Human Verification Required

None. All truths were verifiable programmatically: SQL/DTO/service wiring via grep+read, template rendering via passing automated tests (re-run independently, not trusted from SUMMARY), and the D-12 upgrade mechanism via byte-diff against the pinned git snapshot plus independently re-run regression tests covering the exact "pre-existing install" scenario the plan-checker originally flagged as a gap.

### Gaps Summary

No gaps. All 8 derived observable truths for PRN-01/ORG-01/ORG-02 are verified in the codebase, not merely claimed in SUMMARY.md. Every plan's `must_haves` (artifacts, key_links, truths) checked against actual file content and passing test output, independently re-executed by the verifier. The three code-review warnings (WR-01/WR-02/WR-03) are defense-in-depth/consistency gaps explicitly disclosed in 20-REVIEW.md as non-blocking (0 critical findings) — they do not undermine the phase's delivered goal. The single documentation gap (REQUIREMENTS.md ORG-01 checkbox not ticked) is cosmetic and does not reflect a code deficiency.

---

*Verified: 2026-07-14*
*Verifier: Claude (gsd-verifier)*
</content>
