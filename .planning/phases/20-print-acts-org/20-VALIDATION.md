---
phase: 20
slug: print-acts-org
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-07-14
reconstructed: true
---

# Phase 20 — Validation Strategy

> Per-phase validation contract. Reconstructed retroactively from PLAN/SUMMARY/VERIFICATION artifacts (State B) — phase was already executed and verified before this audit.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`), integration tests under `crates/trackly-app/tests/` + `#[cfg(test)]` unit modules |
| **Config file** | none — Cargo-native; test env requires `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1` and a real `pnpm`-built `ui/dist` for SPA-serving tests (see CI test requirements memory) |
| **Quick run command** | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test html_act_render -- --test-threads=1` |
| **Full suite command** | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --test-threads=1` |
| **Estimated runtime** | ~1 second for the Phase-20 test binaries; single `cargo test` process only (no concurrent runs — target/ lock contention) |

---

## Sampling Rate

- **After every task commit:** Run the relevant `--test` binary (e.g. `html_act_render`)
- **After every plan wave:** Run all four Phase-20 test binaries + the `pdf::html_templates::tests` unit module
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** ~5 seconds

---

## Per-Task Verification Map

| Plan | Wave | Requirement | Behavior | Test Type | Automated Command | Test Name | File Exists | Status |
|------|------|-------------|----------|-----------|-------------------|-----------|-------------|--------|
| 20-01 | 1 | ORG-02 | `address_line2` column stored + threaded through DTOs/SQL (round-trip) | integration | `cargo test -p trackly-app --test org_settings` | `org_settings_save_and_load_round_trip` | ✅ | ✅ green |
| 20-02 | 2 | PRN-01, ORG-02 | `render_acceptance_pdf` sources full org ctx from `org_db.get_for_pdf()` | integration | `cargo test -p trackly-app --test pdf_render_act` | `render_acceptance_pdf_for_device_works` | ✅ | ✅ green |
| 20-03 | 2 | PRN-01, ORG-02 | `act_acceptance.html` requisites block at full parity with handover | integration | `cargo test -p trackly-app --test html_act_render` | `html_acceptance_full_org_parity_with_handover` | ✅ | ✅ green |
| 20-03 | 2 | ORG-02 | `report.html` header renders guarded `address_line2` line | integration | `cargo test -p trackly-app --test html_report_render` | `html_report_org_header_shows_address_line2` | ✅ | ✅ green |
| 20-04 | 3 | ORG-02 | UI «Адрес (2-я строка)» field wired end-to-end; bindings regenerated | integration | `cargo test -p trackly-app --test export_bindings` | binding export (asserts `address_line2` in generated DTOs) | ✅ | ✅ green |
| 20-05 | 3 | ORG-01 | SVG logo with `<script>` embeds as `data:` URI in `<img>` only, never inlines `<script>` | integration | `cargo test -p trackly-app --test html_act_render` | `html_svg_logo_with_script_embeds_img_only_no_inline_script` | ✅ | ✅ green |
| 20-05 | 3 | PRN-01, ORG-02 | Report header offline-safe + address_line2 parity | integration | `cargo test -p trackly-app --test html_report_render` | `html_report_org_header_present`, `html_report_disallowed_logo_mime_drops_logo` | ✅ | ✅ green |
| 20-06 | 4 | PRN-01, ORG-02 (D-12) | Untouched legacy default templates auto-upgrade to current bundled body on startup (delivery to existing installs) | unit | `cargo test -p trackly-app --lib pdf::html_templates::tests` | `upgrade_replaces_untouched_legacy_default_with_current_bundled_body` | ✅ | ✅ green |
| 20-06 | 4 | PRN-01, ORG-02 (D-12) | User-customized template left untouched (fail-closed) | unit | `cargo test -p trackly-app --lib pdf::html_templates::tests` | `upgrade_leaves_user_customized_file_untouched` | ✅ | ✅ green |
| 20-06 | 4 | PRN-01, ORG-02 (D-12) | Already-current template is a no-op | unit | `cargo test -p trackly-app --lib pdf::html_templates::tests` | `upgrade_is_noop_when_file_already_current` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Requirement Coverage Summary

| Requirement | Description | Coverage | Evidence |
|-------------|-------------|----------|----------|
| **PRN-01** | Полный org-контекст в шапке acceptance-печати | COVERED | `html_acceptance_full_org_parity_with_handover` + `render_acceptance_pdf_for_device_works`; delivered to existing installs via D-12 upgrade tests |
| **ORG-01** | SVG-логотип безопасно встраивается, скрипты не исполняются | COVERED | `html_svg_logo_with_script_embeds_img_only_no_inline_script` (real `<script>`-bearing SVG fixture `logo_test_with_script.svg`) |
| **ORG-02** | Вторая строка адреса во всех печатных формах | COVERED | `org_settings_save_and_load_round_trip` + `html_report_org_header_shows_address_line2` + acceptance parity test; UI half via `export_bindings` |

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. No new framework, config, or stub scaffolding was needed — the Rust `cargo test` harness and the pre-existing `html_act_render.rs` / `html_report_render.rs` / `pdf_render_act.rs` / `org_settings.rs` integration binaries were extended in-place. New fixture added: `crates/trackly-app/tests/fixtures/logo_test_with_script.svg` (ORG-01).

---

## Manual-Only Verifications

All phase behaviors have automated verification. The three code-review warnings (WR-01 read-time mime allowlist gap, WR-02 authorize-ordering in `save_logo`, WR-03 unconditional legacy org read) documented in 20-REVIEW.md / 20-SECURITY.md are non-blocking defense-in-depth items, not uncovered functional requirements.

---

## Validation Sign-Off

- [x] All tasks have automated verify or existing-infrastructure coverage
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (none — no gaps)
- [x] No watch-mode flags
- [x] Feedback latency < 5s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-07-14 (reconstructed; all 10 mapped tests independently re-run green during audit)

---

## Validation Audit 2026-07-14

| Metric | Count |
|--------|-------|
| Requirements audited | 3 (PRN-01, ORG-01, ORG-02) |
| Gaps found | 0 |
| Resolved (tests generated) | 0 (none needed) |
| Escalated to manual-only | 0 |
| Existing tests re-run green | 31 (html_act_render 10, html_report_render 8, org_settings 4, pdf_render_act 11 — plus 3 D-12 unit tests) |

Reconstructed from artifacts (State B). No VALIDATION.md existed; phase was executed and verified (20-VERIFICATION.md, 8/8 truths). All requirement→test mappings confirmed present in the codebase and independently re-executed green during this audit — no gap-filling required.
