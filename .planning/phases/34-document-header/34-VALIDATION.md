---
phase: 34
slug: document-header
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-08-08
audited: 2026-08-11
---

# Phase 34 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `34-RESEARCH.md` § Validation Architecture (line 891+).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (workspace). No separate test config — integration targets live in `crates/trackly-app/tests/*.rs`, unit tests inline in `src/pdf/*.rs`. |
| **Config file** | none — workspace `Cargo.toml` |
| **Quick run command** | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib pdf:: -- --test-threads=1` |
| **Full suite command** | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --test-threads=1` (requires a real `pnpm --dir ui build` output in `ui/dist` — placeholder fails the `security_headers` SPA test) |
| **Estimated runtime** | quick ~20 s · full ~3–5 min |

**Hard constraints (project memory):**
- Never run two `cargo test` processes concurrently — they contend on the `target/` lock and look like a multi-minute hang (`cargo-no-concurrent-test`).
- `cargo test --workspace` hangs on the pre-existing `auth_remember_cookie` test — use the targeted `-p trackly-app` commands above (`workspace-test-hangs-auth-remember-cookie`).

---

## Sampling Rate

- **After every task commit:** `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib pdf:: -- --test-threads=1`
- **After every plan wave:** full suite command above, with `pnpm --dir ui build` run first
- **Before `/gsd-verify-work`:** full suite green **AND** the Level-2 human visual pass (below) done on both transports
- **Max feedback latency:** ~20 s (quick) / ~300 s (full)

---

## Per-Task Verification Map

> Resolved against the phase as actually executed (audit 2026-08-11). Every row was re-run,
> not read — see the Validation Audit section for the commands and counts.

| Source | Plan | Requirement | Threat Ref | Secure Behavior | Test Type | Test | Status |
|--------|------|-------------|------------|-----------------|-----------|------|--------|
| Task 3 | 34-02 | DOC-04 | — | N/A | structural | `html_header_parity::all_three_templates_include_header_partial` — all three templates contain `{% include "_header.html" %}` | ✅ green |
| Task 3 | 34-03 | DOC-04 | — | N/A | integration | `html_header_parity::header_fragment_identical_across_all_three_forms` — rendered header fragment byte-identical across the three forms | ✅ green |
| Tasks 1–2 | 34-06 | DOC-04 | — | N/A | **manual UAT** | 34-HUMAN-UAT test 1 — real preview on desktop webview + LAN browser, re-confirmed after the CR-01 fix | ✅ passed |
| Task 2 | 34-02 | DOC-05 | T-34-PRIV | Shipped templates carry no literal org name — the name node must be a Jinja expression | structural | `html_header_parity::header_partial_org_name_node_has_no_hardcoded_literal` — privacy-safe positive form (asserts the node is a Jinja expression); the real name is **not** in the test | ✅ green |
| Task 3 | 34-03 | DOC-05 | — | N/A | integration | `pdf_render_act::render_pdf_with_multiline_full_name_renders_br_not_raw_newline` — non-empty multiline `full_name` through the real render pipeline | ✅ green |
| Task 3 | 34-01 | DOC-05 / D-03 | T-34-XSS | HTML-escape runs **before** `<br>` insertion; `<script>` in the org field renders inert | unit | `pdf::minijinja_env::tests::org_full_name_html_*` — 6 cases: escape-before-`<br>`, `<script>` payload, `&`, empty, CRLF, lone CR (IN-04) | ✅ green |
| Task 1 | 34-01 | DOC-05 | — | N/A | integration | `org_settings::org_settings_save_and_load_round_trip` — multiline Cyrillic value round-trips byte-for-byte | ✅ green |
| IN-03 fix | review | DOC-05 | T-34-PRIV | `full_name` feeds every printed header — length is bounded server-side | integration | `org_settings::org_full_name_length_is_bounded` — at-bound accepted, one over rejected on `field=full_name`, stored value unchanged | ✅ green |
| CR-01 fix | review | DOC-05 / D-04 | — | N/A | integration | `html_header_parity::empty_full_name_renders_bare_short_name_without_stray_br_or_orphan_parens` — encodes the C-01 behavior the shipped defect violated | ✅ green |
| Task 2 | 34-04 | DOC-05 (UI) | — | N/A | **manual only** | No frontend test runner exists in `ui/` (no vitest/jest/playwright) — see Manual-Only | ✅ passed (UAT) |
| Task 2 | 34-02 | DOC-06 | — | Fail-closed preserved: unrecognized file is never overwritten | unit | `pdf::html_templates::tests::upgrade_replaces_v21_legacy_default_with_current_bundled_body` + 4 siblings (untouched-legacy, user-customized, already-current, read-only dir) | ✅ green |
| WR-06 fix | review | DOC-06 | — | Every shipped default has a registered legacy predecessor (or is explicitly exempt) | unit | `pdf::html_templates::tests::every_default_template_has_a_known_legacy_defaults_entry` | ✅ green |
| Audit | validate | DOC-06 / D-16 | — | Silent skip becomes observable | unit | `pdf::html_templates::tests::upgrade_warns_when_it_skips_a_user_customized_file` — thread-local scoped `tracing` subscriber (no new dependency), asserts the `warn` fires and names the skipped file, with an already-current file as negative control | ✅ green **(added by this audit)** |
| Tasks 1–2 | 34-05 | DOC-06 / D-17 | — | `templates_status` is `ManageSettings`-gated on both transports | integration | `templates_status::{fresh_materialized_dir_reports_current_for_all_four, hand_edited_file_reports_customized_others_unaffected, non_utf8_file_reports_unreadable_not_current}` | ✅ green |
| WR-05 fix | review | DOC-06 (UI badge) | — | N/A | **manual only** | No frontend test runner — see Manual-Only | ✅ passed (UAT) |
| WR-11 fix | review | PRIV-01 | T-34-PRIV | Requisite literals in `*.rs`/`*.html` must be on an explicit allowlist | CI gate | `bash scripts/check-privacy-requisites.sh` (wired into `ci-fast` before the build) | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `crates/trackly-app/tests/html_header_parity.rs` — substring + render gate for DOC-04, modeled on the existing `html_page_parity.rs` (5 tests)
- [x] Privacy-safe "no literal org name in shipped templates" structural test for DOC-05 — positive-assertion form; confirmed by `/gsd-secure-phase` (`34-SECURITY.md`) and by the `check-privacy-requisites.sh` gate
- [x] Unit test for the Rust `full_name` escaping helper (escape → `<br>` order, D-03) — 6 cases
- [x] v21-specific upgrade unit test in `html_templates.rs` (DOC-06)
- [x] Framework install: **not required** — `cargo test` was already configured; the audit closed D-16 with zero new dependencies

---

## Manual-Only Verifications

> Outcome column filled by the 2026-08-11 audit against `34-HUMAN-UAT.md` (status: `resolved`, 2/2 passed).

| Behavior | Requirement | Why Manual | Test Instructions | Outcome |
|----------|-------------|------------|-------------------|---------|
| `full_name` field in Settings → Организация saves and reloads; `templates_status` badge («изменён вручную» / «файл не читается») renders beside the template selector | DOC-05 (34-04) / DOC-06 (WR-05) | `ui/` has no test runner at all — no vitest, jest or playwright config, zero `*.test.*`/`*.spec.*` files. Standing up a frontend harness is out of scope for this phase; the backend round-trip (`org_settings`) and status derivation (`templates_status`) are covered automatically, only the Svelte binding is not. | Open Settings → Организация, enter a multiline fictional name, save, reload. Then Settings → Шаблоны: check the badge per kind, after save and after reset, and with the status fetch failing. | ✅ passed — 34-HUMAN-UAT test 2, both transports |
| Header geometry and typography render identically across all three forms | DOC-04 (Success Criterion #2) | Text-extraction tests cannot see overlap or overflow (project memory `act-pdf-word-fidelity`). Success Criterion #2 explicitly demands a real rendered PDF/preview, not a text test. | Render a real preview/PDF of act_handover, act_acceptance and report. Compare logo → name → requisites block visually. | ✅ passed — 34-HUMAN-UAT test 1 (re-run after the CR-01 fix) |
| Same, in the LAN browser transport | DOC-04 (Success Criterion #2) | Server mode serves `ui/dist`; the desktop webview HMRs separately (project memory `dev-browser-testing-needs-ui-build`). | Run `pnpm --dir ui build`, start server mode, open each form's print preview in a LAN browser, repeat the visual comparison. | ✅ passed — 34-HUMAN-UAT test 1, LAN browser |
| Long legal name / long address does not overflow the 80 mm block | DOC-04 / D-06 | `overflow-wrap` / `hyphens` behavior depends on the engine's hyphenation dictionary (RESEARCH assumptions A2, A3 — `[ASSUMED]`, unverified). | Enter a deliberately long fictional multi-line name and a long fictional address, render on both transports, confirm no horizontal overflow. | ✅ passed — covered by 34-HUMAN-UAT test 1's explicit no-overflow expectation |
| Cyrillic serif fallback on Windows | DOC-04 / D-10 | Windows is not reachable from the dev macOS box (project memory `dev-environment-constraints`); RESEARCH assumption A1 is `[ASSUMED]`. | On the Windows test machine, render each form and confirm the text is serif and Cyrillic renders correctly. | ⬜ **outstanding** — no Windows box in this session; carry into the next Windows release check |
| Upgrade reaches an already-installed copy | DOC-06 (Success Criterion #4) | Requires a real on-disk `templates/` dir matching the v21 snapshot plus an app restart. | Seed a templates dir with the v21 bodies, launch, confirm the new header appears; repeat with a hand-edited file, confirm it is left alone and a `warn` is logged. | ✅ passed — 34-06 confirmed **live** on the user's own pre-Phase-34 install (both branches: untouched upgraded, hand-edited preserved), a stronger proof than the scripted synthetic setup |
| Post-upgrade appearance with an empty `full_name` (C-01) | DOC-05 | Consequence of D-04 that only a human can judge as acceptable. | After upgrade with `full_name` empty and short name set, confirm the header shows only the short name in parentheses; decide whether that is acceptable. | ✅ passed — accepted by the user in 34-06, then re-confirmed post-CR-01 (stray `<br />` / orphan parens gone); now also regression-locked automatically |

---

## Validation Sign-Off

- [x] All tasks have an `<automated>` verify or a Wave 0 dependency
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all ❌ MISSING references above
- [x] No watch-mode flags; `--test-threads=1` used; no concurrent `cargo test`
- [x] Feedback latency < 300 s (quick gate measured at ~0.8 s; 8 integration targets ~1.7 s total)
- [x] Level-2 human visual pass completed on **both** transports (`34-HUMAN-UAT.md`, status `resolved`, 2/2)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-08-11 (audit — every row re-run, not read)

**Carried forward (not blocking):** Cyrillic serif rendering on Windows (DOC-04 / D-10) is
unverifiable from the dev macOS box and stays open for the next Windows release check.

---

## Validation Audit 2026-08-11

| Metric | Count |
|--------|-------|
| Requirement rows audited | 16 |
| Already covered (re-run green) | 15 |
| Gaps found | 1 (DOC-06 / D-16 — skipped auto-upgrade not observable in any test) |
| Resolved | 1 |
| Escalated to manual-only | 0 |
| Manual-only rows | 7 (6 passed via `34-HUMAN-UAT.md` / 34-06; 1 outstanding — Windows serif) |

**Commands run (sequentially, never concurrently):**

```
TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib pdf:: -- --test-threads=1
  → 60 passed; 0 failed  (was 59 before the audit's new test)

TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test <T> -- --test-threads=1
  html_header_parity 5 · templates_status 3 · org_settings 5 · pdf_render_act 12 ·
  html_act_render 11 · html_report_render 8 · template_edit 6 · templates_seed 4
  → 54 passed; 0 failed

cargo clippy -p trackly-app --all-targets -- -D warnings   → clean
bash scripts/check-privacy-requisites.sh                   → gate OK
```

**Gap closed:** `pdf::html_templates::tests::upgrade_warns_when_it_skips_a_user_customized_file`.
Captures `tracing` output through a thread-local scoped subscriber writing into an
`Arc<Mutex<Vec<u8>>>` — no new crate dependency, since `tracing-subscriber` is already a direct
dependency of `trackly-app`. Non-vacuity was proven by mutation: deleting the `warn!` in the
`else` branch turns the new test red while the sibling `upgrade_leaves_user_customized_file_untouched`
stays green, so only the new test covers observability. The mutation was reverted and the
production half of `html_templates.rs` is byte-identical to its pre-audit state (the diff is a
pure insertion inside `#[cfg(test)] mod tests`).

**Known trade-off:** the new test asserts on the human-readable log text (`"Skipped auto-upgrade"`),
not a structured field — a future reword of that message will need the test updated alongside it.
That is deliberate: D-16's requirement is that a human operator can *see* the skip.
