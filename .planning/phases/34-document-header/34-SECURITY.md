---
phase: 34
slug: document-header
status: verified
threats_open: 0
asvs_level: 1
created: 2026-08-11
---

# Phase 34 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register source: `<threat_model>` blocks in `34-01-PLAN.md` … `34-06-PLAN.md`
(18 threats: 8 `mitigate`, 9 `accept`, 1 `transfer`). Verification is
disposition-driven — no blind vulnerability scan was performed.

**Privacy note:** this repository is public. No organization requisite value,
address, phone, e-mail or personal name is reproduced anywhere in this
document; findings are described structurally (`file:line`, "hardcoded literal
present/absent") only.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Settings UI (authenticated, `ManageSettings`) → `org_settings.full_name` → rendered act/report → LAN browser preview | Admin-written, broadcast-read: any authenticated LAN user who opens a print preview renders whatever an admin typed. Same boundary class as the pre-existing `org.logo_data_uri`. | Free-text org legal name, rendered into a `| safe` sink |
| Hand-edited reference bodies under `target/debug/templates/` (gitignored, contained a real org name) → `crates/trackly-app/templates/_header.html` (public repo) | Highest-risk boundary of the phase: a naive whole-file copy would have committed a real organization name to a public repo, permanently. | Organization identity data |
| `crates/trackly-app/templates/*.html` (shipped defaults) → any install's `templates/` directory | Pre-existing (Phase 16/20) materialize + D-16 auto-upgrade mechanism; this phase adds a 4th file (`_header.html`) and a second legacy snapshot generation (`v21`). | Template bodies, upgrade-eligibility decisions |
| Authenticated LAN session → `POST /api/v1/templates_status` → filesystem read of `templates_dir` | New in 34-05: read-only, but discloses which template files an admin hand-customized plus the resolved templates directory path. | Customization status, absolute filesystem path |
| Rust source literals in `template_service::demo_context_for_kind` → TemplateEditor preview → public repo | Not runtime-user-controlled, but compiled into a public repository — subject to the project privacy constraint. | Demo requisites |
| Human sign-off (34-06 checkpoints 1–2) → destructive deletion of the rescued reference files (34-06 Task 3) | Process boundary: an irreversible-in-effect cleanup gated on human visual confirmation. | Workflow authority |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-34-01-01 | Tampering / EoP (stored XSS) | `org_full_name_html` | mitigate | `crates/trackly-app/src/pdf/minijinja_env.rs:44-47` — `format!("{}", HtmlEscape(normalized))` runs FIRST, `.replace('\n', "<br />")` SECOND; CRLF/lone-CR normalized before escaping (does not disturb the ordering, `HtmlEscape` ignores `\r`/`\n`). Unit tests at `minijinja_env.rs:310,318,332,338,345,389` cover order, empty input, CRLF, lone CR, `&`. End-to-end render-through case in `tests/pdf_render_act.rs:661`. Consuming env keeps `AutoEscape::Html` + `UndefinedBehavior::Strict` (`minijinja_env.rs:118-126`). | closed |
| T-34-01-02 | Information Disclosure | `org_settings.full_name` storage | accept | Write path guarded unchanged: `services/org_db_service.rs:105` `authorize(caller, &Action::ManageSettings)`, plus a character cap at `org_db_service.rs:106-112`. Column added by `migrations/V036__org_settings_full_name.sql` with `NOT NULL DEFAULT ''` (no misleading placeholder). Rationale holds — no new sensitivity class. | closed |
| T-34-01-SC | Tampering (supply chain) | package installs | accept | Verified by `git diff <phase-start>~1..HEAD`: `Cargo.lock` unchanged, no `package.json`/`pnpm-lock.yaml` change. See UF-01 for the one manifest edit. | closed |
| T-34-02-01 | Information Disclosure | `_header.html` authoring | mitigate | `crates/trackly-app/templates/_header.html:104-107` — the `.orgName` node contains only Jinja expressions (`org.full_name | safe`, `org.name`) and markup; no hardcoded literal name anywhere in the file. Permanent regression guard: `tests/html_header_parity.rs:61` `header_partial_org_name_node_has_no_hardcoded_literal` (strips Jinja spans, asserts zero alphabetic characters remain). Repo-wide CI privacy gate `scripts/check-privacy-requisites.sh` (wired at `.github/workflows/ci-fast.yml:86`) executed during this audit: exit 0. | closed |
| T-34-02-02 | Repudiation / regression | v21 legacy snapshot timing | mitigate | Snapshots are provably PRE-rewrite: `git show ed87bea~1:crates/trackly-app/templates/{act_handover,act_acceptance,report}.html` is byte-identical to `templates/_legacy_defaults/v21/*.html` (diff exit 0 for all three), while `diff v21/x.html templates/x.html` is non-empty for all three (70–77 lines). Wired into the upgrade-detection registry at `src/pdf/html_templates.rs:76-102` (`KNOWN_LEGACY_DEFAULTS`, v20 + v21 entries per file); `_header.html` carries the required present-but-empty slice at `html_templates.rs:102` (WR-06 invariant enforced by `every_default_template_has_a_known_legacy_defaults_entry`). | closed |
| T-34-02-03 | Tampering | `{% include "_header.html" %}` resolution | accept | Rationale verified, not contradicted: repo-wide grep shows `set_loader` appears only in doc comments (`minijinja_env.rs:9,138`) and is never called. Includes resolve exclusively from the in-memory registry populated in `render_with_timeout` (`minijinja_env.rs:157-170`) — D-13 intact. | closed |
| T-34-02-SC | Tampering (supply chain) | package installs | accept | `tests/html_header_parity.rs` uses `regex`, already a `trackly-app` dependency; no manifest change from this plan. | closed |
| T-34-03-01 | Tampering / EoP | ctx assembly (act / report / template services) | mitigate | Exhaustive enumeration of org-ctx assembly sites (grep `"logo_data_uri"` → 4 sites) matched 1:1 against grep `"full_name"` → all four route through the helper: `services/act_service.rs:2641`, `act_service.rs:2809`, `services/report_service.rs:705`, `services/template_service.rs:451`. Zero raw `org_dto.full_name` / `org.full_name` interpolations into a `| safe` context anywhere in `crates/trackly-app/src/`. Cross-form byte-identity render gate: `tests/html_header_parity.rs:288`. | closed |
| T-34-03-02 | Tampering / availability | `render_with_timeout` `extra_templates` | mitigate | All four production sites source the header via `html_templates::load_template` with the embedded default resolved from `DEFAULT_HTML_TEMPLATES`: `act_service.rs:2589`, `act_service.rs:2751`, `report_service.rs:657`, `template_service.rs:394`. No raw `std::fs::read_to_string` for `_header.html` on any render path. `load_template` (`html_templates.rs:248-263`) returns the embedded default on `NotFound` AND on any other IO/UTF-8 error (warn-logged) — a deleted or Notepad-ANSI-mangled header degrades instead of failing every render. Registration ordering (extras before main template, both before `get_template`/`render`) at `minijinja_env.rs:157-176`. | closed |
| T-34-03-03 | Information Disclosure | `demo_context_for_kind` org block | mitigate | `services/template_service.rs:449-464` — every requisite (org name, full legal name, INN, KPP, address, address_line2, phone, fax, e-mail, OKPO, OGRN) is a fictional placeholder in the same style as the existing fictional test fixtures; `logo_data_uri` is `null`. Personal names in the demo blocks use the project's sanctioned fictional forms. Confirmed by the CI privacy gate passing at HEAD. Residual: pre-phase values persist in git history — see AR-02. | closed |
| T-34-03-SC | Tampering (supply chain) | package installs | accept | See UF-01: a feature flag on an already-present crate, not a new dependency; `Cargo.lock` unchanged across the phase. | closed |
| T-34-04-01 | Tampering | `fullName` textarea value | transfer | Transfer target verified to exist and to be effective. Client performs no escaping by design: `ui/src/features/settings/OrgSettings.svelte:283-287` passes the raw value through one-way `value`/`oninput`, `:118` submits it as `full_name`; no `{@html}` sink exists anywhere under `ui/src/features/settings/`. Server-side enforcement it transfers to: `org_db_service.rs:105` (`ManageSettings`) for write, `minijinja_env.rs:44-47` (escape-then-`<br>`) for render. | closed |
| T-34-04-SC | Tampering (supply chain) | package installs | accept | `Textarea.svelte` is a pre-existing shared component; no frontend manifest/lockfile change in the phase diff. | closed |
| T-34-05-01 | Information Disclosure | `/api/v1/templates_status` | mitigate | BOTH transports guarded. Tauri: `src/tauri_cmds/settings_org.rs:532-534` — `resolve_tauri_identity` then `authorize(&caller, &Action::ManageSettings)`. HTTP: `src/http/settings_org.rs:306-309` — `session_identity(&session)` (returns `AppError::Unauthorized` on absent/invalid session, `http/auth.rs:99-106` → 401) then `authorize(..., &Action::ManageSettings)` (→ 403), both BEFORE `build_templates_status`. Route registered inside the `/api/v1` namespace at `http/settings_org.rs:392`, not in the public-route set (`http/mod.rs:95-149`: only `auth_login`, `request_ad_restore`, `auth_status`). See UF-04 for a test-coverage observation. | closed |
| T-34-05-02 | Tampering | `build_templates_status` filesystem read | accept | Rationale verified: `tauri_cmds/settings_org.rs:308-355` only calls `read_template_if_present` (read-only) inside `spawn_blocking`; no write call. The D-16 write path `upgrade_untouched_defaults_on_startup` has exactly one caller — `src/context.rs:224` (startup) — and is unreachable from the endpoint. | closed |
| T-34-05-SC | Tampering (supply chain) | package installs | accept | No manifest change from this plan. | closed |
| T-34-06-01 | Tampering (process) | deletion gated behind human verification | mitigate | Documentation-level verification (as declared). `34-06-PLAN.md:56` and `:102` are both `<task type="checkpoint:human-verify" gate="blocking">`; the deletion is `34-06-PLAN.md:132-145` (Task 3, last task, action text "Only after BOTH preceding checkpoints are approved"). `34-06-SUMMARY.md:61,67,68,70` records Task 1 APPROVED (desktop) and Task 2 APPROVED (LAN browser), both in prior sessions, before the Task 3 deletion session. Independently corroborated by `34-VERIFICATION.md:36`. | closed |
| T-34-06-SC | Tampering (supply chain) | package installs | accept | No manifest change from this plan. | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Unregistered Flags

No `## Threat Flags` section exists in any of `34-01-SUMMARY.md` … `34-06-SUMMARY.md`, so the
following new attack surface was identified from the implementation diff rather than from the
executor's own declarations. None is a blocker; all are recorded here for the register.

| Flag | Surface | Assessment |
|------|---------|------------|
| UF-01 | `minijinja`'s `multi_template` Cargo feature enabled (`crates/trackly-app/Cargo.toml`, the phase's only manifest change) — `{% include %}` / `{% import %}` / `{% extends %}` now parse in user-editable template bodies | Not a new dependency (`Cargo.lock` unchanged across `edfdb94~1..HEAD`), so the `*-SC` "no new package" claims are accurate as written — but the feature does widen the template language available to a `ManageSettings` admin editing `templates/*.html`. Contained by the unchanged no-loader invariant (`set_loader` never called, grep-verified): includes resolve only against names explicitly registered in-memory, so no filesystem or path-traversal reach. Recommend registering this surface explicitly in a future phase's threat model rather than under an `-SC` row. |
| UF-02 | Registering `_header.html` into `DEFAULT_HTML_TEMPLATES` (34-02) briefly made the shared partial addressable as an editor "kind" — `templates_update_body {"kind":"_header"}` would have overwritten the header shared by all three print forms, invisibly and unrevertably from the UI | Surfaced and closed within the phase (34-03, Rule 1 fix): `services/template_service.rs:66-71` `is_editable_template_filename` requires allowlist membership AND no `_` prefix, and is applied on all three paths — `list_all_for_editor:220`, `update_body:269`, `reset_to_default:316`. Regression test at `template_service.rs:719-733`. Closed, but never appeared in any plan's threat register. |
| UF-03 | `TemplateStatusDto.templates_dir` (`src/dto/reports.rs:331-334`) returns the resolved absolute templates directory path to the client | Sub-aspect of T-34-05-01, covered by the same `ManageSettings` guard on both transports. Informational: path disclosure is limited to admins, who can already read `settings_get_db_path`. |
| UF-04 | No automated authorization regression test for the new endpoint | `crates/trackly-app/tests/templates_status.rs` exercises `build_templates_status` directly (status derivation only) and never asserts 401/403 on `handler_templates_status` / `templates_status`. The guards themselves are present in code (evidence under T-34-05-01), so the threat is mitigated — but the guard is not test-pinned and a future refactor could drop it silently. Recommended follow-up, not a blocker. |

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-01 | T-34-01-02 | `org_settings.full_name` sits at the same trust level as every other org requisite: admin-written under `ManageSettings`, readable by any authenticated user who prints a document. No new sensitivity class; write guard and length cap verified in code. | Phase 34 plan author (34-01-PLAN) | 2026-08-09 |
| AR-02 | T-34-03-03 (residual) | Real-looking organization requisites scrubbed from HEAD by this phase remain in the public git history of earlier commits. A history rewrite is destructive to every existing clone/fork and was deliberately deferred pending separate user authorization (PRIV-01 in `.planning/STATE.md`; WR-11 in `34-REVIEW-FIX.md:193-205`). Compensating control shipped this phase: `scripts/check-privacy-requisites.sh` wired into `.github/workflows/ci-fast.yml:86` — verified passing at HEAD during this audit. Out of scope for DOC-04/05/06. | User (PRIV-01 decision, recorded in STATE.md) | 2026-08-09 |
| AR-03 | T-34-02-03 | `{% include %}` resolution relies on the in-memory registry only; D-13 (`env.set_loader` never called) is unchanged code, grep-verified. | Phase 34 plan author (34-02-PLAN) | 2026-08-09 |
| AR-04 | T-34-05-02 | `build_templates_status` is read-only and structurally cannot reach the D-16 upgrade-write path (single caller: startup). Worst case is a stale read of an informational endpoint. | Phase 34 plan author (34-05-PLAN) | 2026-08-09 |
| AR-05 | T-34-01-SC … T-34-06-SC | No new package-manager dependency was introduced. Verified: `Cargo.lock` and the frontend lockfile are unchanged across the phase; the sole manifest edit enables an additional feature on the already-vetted `minijinja` crate (see UF-01). | Security audit, 2026-08-11 | 2026-08-11 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-08-11 | 18 | 18 | 0 | gsd-security-auditor |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-08-11
