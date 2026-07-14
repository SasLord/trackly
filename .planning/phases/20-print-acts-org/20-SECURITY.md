---
phase: 20
slug: print-acts-org
status: verified
threats_open: 0
asvs_level: 1
created: 2026-07-14
---

# Phase 20 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Register authored at plan-time (all 6 PLANs carried `<threat_model>` blocks) — this
> audit **verifies** each mitigation against the implementation; it does not construct
> a new register.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| HTTP/Tauri client → `org_db_service.rs::save_fields` | Authenticated settings-write path; caller-supplied OrgPatch (incl. new `address_line2` free-text) crosses into SQL UPDATE | Org requisites (free-text, admin-authored) |
| `org_settings` (DB) → render ctx → HTML template string | Full org requisites (incl. logo BLOB as `data:` URI) flow into acceptance/handover/report documents | Org business info + logo bytes |
| Malicious SVG logo → `save_logo` → `render_pdf` → rendered HTML | Adversarial admin-uploaded SVG; validates img-only embedding invariant | Untrusted image bytes (admin-supplied) |
| On-disk template file (user-customizable via Settings editor) → `upgrade_untouched_defaults_on_startup` → conditional overwrite | Startup overwrite of files that may hold user-authored content; byte-identity check is the only guard | User-authored template bodies |
| `OrgSettings.svelte` form input → `settings_save_org_fields` | Free-text `address_line2` crosses from browser/webview into authenticated write path | UI form value |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-20-01-01 | Tampering | `org_db_service.rs::save_fields` (address_line2) | accept | Free-text TEXT param, same shape as phone/fax/email; write gated by `authorize(caller, &Action::ManageSettings)` (org_db_service.rs:91,99,113) | closed |
| T-20-01-02 | Elevation of Privilege | `org_db_service.rs::save_fields` | accept | Pre-existing authorize() gate untouched (org_db_service.rs:91) | closed |
| T-20-01-03 | Tampering | migrations/V035 | accept | Additive-only `ALTER TABLE org_settings ADD COLUMN address_line2 TEXT NOT NULL DEFAULT ''` | closed |
| T-20-02-01 | Information Disclosure | `render_acceptance_pdf` ctx | accept | Full org requisites in acceptance print is intended PRN-01 behavior, parity with handover/report (act_service.rs:2546-2549) | closed |
| T-20-02-02 | Tampering | `render_acceptance_pdf` (legacy logo path) | mitigate | Legacy `pipeline.organization.read_logo_bytes` removed — `grep -c read_logo_bytes act_service.rs` == **0**; sole source is `org_db.get_for_pdf()` | closed |
| T-20-02-03 | Tampering | `logo_data_uri` construction | accept | `logo_bytes`/`logo_mime` originate exclusively from `org_db.get_for_pdf()` (org_settings BLOB, authenticated-write-gated) — act_service.rs:2548,2699 | closed |
| T-20-03-01 | Injection (XSS) | `{{ org.address_line2 }}` in 3 templates | mitigate | Interpolated WITHOUT `\| safe` (act_acceptance.html:108, act_handover.html:135, report.html:139); autoescape ON via `AutoEscape::Html` (minijinja_env.rs:56) | closed |
| T-20-03-02 | Injection (XSS) | logo `<img src="{{ org.logo_data_uri \| safe }}">` | accept | img-only `data:` URI embedding, pre-existing/unchanged (act_acceptance.html:101, act_handover.html:128, report.html:132) | closed |
| T-20-04-01 | Tampering | `OrgSettings.svelte` address_line2 input | accept | Client value is UI state; real trust boundary is server-side `save_fields` authorize() gate (org_db_service.rs:91) | closed |
| T-20-04-02 | Information Disclosure | `ui/src/bindings.ts` regeneration | accept | Exposes only type shapes (field names/types), not data | closed |
| T-20-05-01 | Injection (Script Exec / XSS) | logo `<img>` embedding via malicious SVG | mitigate | Regression test `html_svg_logo_with_script_embeds_img_only_no_inline_script` (tests/html_act_render.rs:327) asserts no literal `<script>` (:351), non-vacuous data-URI embed (:359), img-only embedding (:367) | closed |
| T-20-05-02 | Information Disclosure | PRN-01 parity test assertions | accept | Test-only synthetic fictional org data | closed |
| T-20-06-01 | Tampering (destructive overwrite) | `html_templates.rs::upgrade_untouched_defaults_on_startup` | mitigate | Exact byte-for-byte equality (`&on_disk == current_default` :141; `*legacy == on_disk` :149) — NOT hash/fuzzy; else-branch fail-closed leaves user-customized files untouched (:158-159); regression test `upgrade_leaves_user_customized_file_untouched` (:263) | closed |
| T-20-06-02 | Tampering (stale registry) | `html_templates.rs::KNOWN_LEGACY_DEFAULTS` | accept | Documented extension point (:60); worst case = missed upgrade, never wrongful overwrite (asymmetric fail-closed risk) | closed |
| T-20-06-03 | Denial of Service | `include_str!` of `_legacy_defaults/v20/*.html` | accept | Compile-time embed (html_templates.rs:63-75) — missing snapshot fails BUILD, never runtime | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-20-01 | T-20-01-01/02/03 | Free-text column + authorize(ManageSettings) gate + additive-only migration — no new attack surface vs. Phase 14 requisites | gsd-security-auditor | 2026-07-14 |
| AR-20-02 | T-20-02-01, T-20-02-03 | Org requisites/logo in acceptance print is intended PRN-01 business info (not end-user PII); bytes only from authenticated-write-gated org_db | gsd-security-auditor | 2026-07-14 |
| AR-20-03 | T-20-03-02 | img-only `data:` URI logo embedding pre-existing/unchanged; regression-locked by T-20-05-01 | gsd-security-auditor | 2026-07-14 |
| AR-20-04 | T-20-04-01, T-20-04-02 | Client input is UI state (server gate is trust boundary); bindings.ts exposes type shapes only | gsd-security-auditor | 2026-07-14 |
| AR-20-05 | T-20-05-02 | Test-only synthetic fictional org data | gsd-security-auditor | 2026-07-14 |
| AR-20-06 | T-20-06-02, T-20-06-03 | Stale registry worst-case = missed upgrade (fail-closed); missing snapshot fails build, not runtime | gsd-security-auditor | 2026-07-14 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-07-14 | 16 | 16 | 0 | gsd-security-auditor (verify mode) |

Unregistered flags: none. All three SUMMARY.md `## Threat Flags` sections (20-01/02/03) explicitly report "None" — no new attack surface introduced beyond the plan-time register.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-07-14
