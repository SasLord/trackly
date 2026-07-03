---
phase: 14-act-data-structure
status: secured
asvs_level: 1
block_on: high
threats_total: 12
threats_closed: 12
threats_open: 0
audit_date: 2026-07-04
---

# SECURITY — Phase 14: act-data-structure

**Audit date:** 2026-07-04
**ASVS Level:** 1
**Block-on:** high
**Result:** SECURED — 12/12 threats resolved (10 mitigate CLOSED, 2 accept CLOSED)

Verification method: every `mitigate` threat was confirmed by locating the
actual mitigation call in the implementation (not documentation/intent);
every `accept` threat is logged below in the Accepted Risks section.
Implementation files were treated as read-only.

---

## Threat Verification

### Plan 14-01 (schema + HeaderBlock)

| Threat ID | Category | Disposition | Evidence |
|-----------|----------|-------------|----------|
| T-14-01 | Tampering | mitigate | `authorize(caller, &Action::ManageSettings)` at `crates/trackly-app/src/services/org_db_service.rs:90` (in `save_fields`); write via single-writer `self.writer.execute(...)` at :92 |
| T-14-02 | Injection | mitigate | `save_fields` UPDATE uses `params![...]` (org_db_service.rs:100-112); `get`/`get_for_pdf` bind no user input; no string concatenation into any SQL literal |
| T-14-03 | Information Disclosure | accept | See Accepted Risks AR-14-03 — org requisites are non-secret, intended for print |
| T-14-SC | Tampering (supply chain) | mitigate | `git diff 08bcddc^..2aa0698` shows no `Cargo.toml`/`Cargo.lock`/`package.json`/`pnpm-lock` changes — zero new dependencies |

### Plan 14-02 (transport + Settings UI)

| Threat ID | Category | Disposition | Evidence |
|-----------|----------|-------------|----------|
| T-14-02-01 | Elevation of Privilege | mitigate | HTTP: `authorize(&caller, &Action::ManageSettings)` at `crates/trackly-app/src/http/settings_org.rs:184`. Tauri: `settings_save_org_fields` (tauri_cmds/settings_org.rs:305) → `build_settings_save_org_fields` (:33) → `org_db.save_fields` which re-enforces authorize at org_db_service.rs:90. Both entry points gated. |
| T-14-02-02 | Tampering | mitigate | OrgPatch forwarded opaquely to the same `params!` single-writer UPDATE; no new privileged operation introduced (build helper tauri_cmds/settings_org.rs:28-34) |
| T-14-02-03 | Cross-Site Scripting | accept | See Accepted Risks AR-14-02-03 — Svelte auto-escapes; no `@html` in `ui/src/features/settings/OrgSettings.svelte` (values use `bind:value` at :264/275/286/297/308); PDF path renders via structured DocSpec IR, not raw HTML |
| T-14-02-SC | Tampering (supply chain) | mitigate | No dependency manifest changes in phase commit range (same evidence as T-14-SC) |

### Plan 14-03 (act render context)

| Threat ID | Category | Disposition | Evidence |
|-----------|----------|-------------|----------|
| T-14-03-01 | Denial of Service | mitigate | `ActItemDto.specs: Option<String>` (`crates/trackly-app/src/dto/act.rs:107`) serializes NULL notes to JSON `null`, not error; org requisites default to `""` (V033 `DEFAULT ''` + `None` fallback branch act_service.rs:1361-1372). Regression test `render_pdf_with_null_specs_and_empty_requisites_succeeds` (tests/pdf_render_act.rs) asserts `Ok(non-empty PDF)`. |
| T-14-03-02 | Information Disclosure | accept | See Accepted Risks AR-14-03-02 — intended D-05 unification; `org_settings` is the canonical source |
| T-14-03-03 | Injection | mitigate | Requisites/specs passed as JSON **data** via `serde_json::json!` context (act_service.rs:1410-1440), never spliced into template source; template output parsed into typed `DocSpec` via `serde_json::from_str` (:1450) then `render_docspec` (:1456). `load_items_for_act` reads `d.notes` via `params![act_id]` parameterized query (act_service.rs:1744-1753). |
| T-14-03-SC | Tampering (supply chain) | mitigate | No dependency manifest changes in phase commit range (same evidence as T-14-SC) |

---

## Accepted Risks Log

| ID | Threat | Category | Rationale |
|----|--------|----------|-----------|
| AR-14-03 | New org requisites (phone/fax/email/OKPO/OGRN) appear in generated PDF | Information Disclosure | Org requisites are corporate reference data, not PII/secrets; their whole purpose is to be printed on the act. No confidentiality boundary crossed. |
| AR-14-02-03 | Requisite strings rendered in Settings UI / PDF | Cross-Site Scripting | Svelte auto-escapes interpolated values; the Settings form uses `bind:value` on `<input>` elements with no `@html` sink. PDF output is built from a structured DocSpec IR (typed JSON → renderer), not raw HTML/markup, so injected markup cannot execute. |
| AR-14-03-02 | Switching act-render org source to `org_settings` may show requisites in old acts that previously read from `org.json` | Information Disclosure | Expected unification per design decision D-05. `org_settings` (editable in Settings by ManageSettings admins) is the single canonical source. Historic acts re-render against current canonical org data by design; no unauthorized disclosure — same admin-controlled data. |

---

## Unregistered Flags

None. No Phase 14 SUMMARY contains a `## Threat Flags` section; no new attack
surface was reported by the executor during implementation. All requisite and
specs data flows through pre-existing, already-gated write/read paths.

---

## Auditor Notes

- Both write transports (HTTP + Tauri) converge on the single
  `OrgDbService::save_fields` mutation, which is the enforcement point for
  `ManageSettings`. The Tauri command does not call `authorize` inline but the
  guard is unconditional inside `save_fields`, so the mutation cannot execute
  unauthorized regardless of transport. Verified end-to-end, not assumed.
- V033 columns are `TEXT NOT NULL DEFAULT ''` — no NULL requisites on historic
  rows, and empty requisites degrade to blank in rendered output rather than
  producing render errors (backing T-14-03-01).
- Supply-chain claim verified by diffing the actual phase commit range
  (`08bcddc^..2aa0698`) for manifest/lockfile changes — none found.
