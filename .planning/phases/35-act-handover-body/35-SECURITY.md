---
phase: 35
slug: act-handover-body
status: verified
threats_open: 0
asvs_level: 1
created: 2026-08-12
---

# Phase 35 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Audit scope: git range `77a1885~1..HEAD`, 12 implementation files under `crates/`
plus the phase's `.planning/` artifacts. Threat register authored at plan time
across seven PLAN files (`35-01` … `35-07`); this audit **verifies the declared
mitigations exist in the implementation** — it is not a retroactive STRIDE scan.
No SUMMARY declares a `## Threat Flags` section (verified: no match in
`.planning/phases/35-act-handover-body/*.md`), so the register below is complete.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| DB-stored act fields → MiniJinja HTML render | `act.giver_name` / `act.receiver_name` / `document.giver_name` / `document.receiver_name` / `item.name` are user-entered values interpolated into the printed act. This phase adds new **places** of interpolation, not a new sink — `build_safe_html_env` (`crates/trackly-app/src/pdf/minijinja_env.rs:118-126`) sets `AutoEscape::Html` unconditionally. | User-supplied ФИО, device names, inventory/serial numbers |
| Admin-editable template file on disk → MiniJinja render | `act_handover.html` / `act_acceptance.html` are re-read from disk per render. No loader is set (`minijinja_env.rs:124`), so `{% include %}` cannot reach the filesystem (T-16-02 baseline, untouched by this phase). | Template body authored by a local administrator |
| Phase diff → PUBLIC git repository | Repository is public. Any literal committed — fixture ФИО, org requisites, template text — is permanent in git history. Last control point is the pre-commit privacy gate. | Fictional fixtures only; real org/personal data forbidden |
| `_legacy_defaults/vNN/*.html` snapshots → binary (`include_str!`) | Snapshots drive the auto-upgrade of installed template copies. A snapshot taken *after* an edit silently voids the upgrade guarantee for existing installs. | Bundled default template bodies |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-35-01 | Information Disclosure | `demo_context_for_kind` new `giver_name` literal | mitigate | Reuses the already-approved fictional name «Иванов И.И.»; no new name introduced. `crates/trackly-app/src/services/template_service.rs:511` (diff adds exactly one line, adjacent to the pre-existing `receiver_name: "Петров П.П."`). | closed |
| T-35-02 | Tampering | `_legacy_defaults/v22/*.html` (new files) | accept | Premise verified mechanically, not trusted: both snapshots are **byte-identical** to the pre-phase templates. `diff <(git show 77a1885~1:crates/trackly-app/templates/act_handover.html) crates/trackly-app/templates/_legacy_defaults/v22/act_handover.html` → empty; same for `act_acceptance.html`. No hand-edit smuggled into the snapshot. | closed |
| T-35-03 | Tampering (stored XSS) | `{{ act.giver_name }}` / `{{ act.receiver_name }}` in handover signature block | mitigate | Plain interpolation under `AutoEscape::Html`. `crates/trackly-app/templates/act_handover.html:181,189`. Verified NO `\| safe` anywhere in the template body — `grep -n safe act_handover.html` returns only doc-comment lines 29-33. Env: `minijinja_env.rs:121` `set_auto_escape_callback(\|_\| AutoEscape::Html)`. | closed |
| T-35-04 | Information Disclosure | doc-comment and template text of `act_handover.html` | accept | Premise verified: every text literal added by 35-02 (commit `3904da9`) is a generic Russian label («Инвентарный номер:», «Серийный номер:», «Модель:», «Сроком до:», «Настоящим актом утверждаю, что мною:»). No organisation name, requisite, address or ФИО literal introduced; all values still arrive from the DB context. *Nuance recorded below in Accepted Risks (AR-2).* | closed |
| T-35-05 | Tampering (stored XSS) | `{{ document.giver_name }}` / `{{ document.receiver_name }}` in acceptance signature block | mitigate | Plain interpolation under `AutoEscape::Html`. `crates/trackly-app/templates/act_acceptance.html:138,146`. No `\| safe` in the template body (grep confirms doc-comment lines 20-24 only). | closed |
| T-35-06 | Information Disclosure | ФИО row de-duplication in `table.kv` | accept | Premise verified: commit `81b3d39` deletes exactly two `<tr>` rows («Кто передал» / «Кто принял») and updates the doc-comment. Data source unchanged — the same two context values are still rendered, once, in the signature block. Removal narrows, never widens, the disclosure surface. Regression assert `!html.contains("Кто передал")` at `tests/html_act_render.rs:250-253`. | closed |
| T-35-07 | Tampering | `tests/html_field_row_underline_gate.rs` (new file) | accept | Premise verified: the file contains no write API. `grep -nE "fs::write\|File::create\|OpenOptions\|remove_file\|std::fs"` → no match; the only file access is compile-time `include_str!` at lines 26-27. Read-only by construction. | closed |
| T-35-08 | Repudiation | weakened/removed test assertions in `html_act_render.rs` | mitigate | Coverage redirected, not reduced — verified in the diff: the dropped `"ФИО"` label assertion is replaced by (a) an explicit positive assert on the printed ФИО (`html.contains("Выдалов В.В.")`, `tests/html_act_render.rs:194-198`), (b) a new negative assert on the removed duplicate row (`tests/html_act_render.rs:250-253`), and (c) `"Выдал"`/`"Получил"` tightened to `"Выдал:"`/`"Получил:"`. Net assertion count increases. | closed |
| T-35-09 | Information Disclosure | manual UAT report (35-05 / 35-07 resume-signals) | mitigate | Both UAT write-ups are anonymised in fact, not just in instruction: `35-05-SUMMARY.md:62` and `35-07-SUMMARY.md:40,96` describe structural outcomes («длинное вымышленное ФИО полностью видно…») and never quote a real name. Repo-wide scan of the phase's `.planning/` diff for ФИО-shaped strings returns only formulaic fictional names. | closed |
| T-35-13 | Information Disclosure | `{{ item.name }}` now printed unconditionally per `.device-block` | accept | Premise verified: commit `d274e6b` removes only the `{%- if act.items \| length == 1 %}` / `{%- endif %}` pair around a pre-existing line. Same sink, same variable, no new interpolation point. `act_handover.html:144`. Escaping still governed by `AutoEscape::Html` (T-16-01 baseline). | closed |
| T-35-14 | Tampering | accidental `\| safe` around `item.name` | mitigate | Verified by diff, not by claim: `git show d274e6b -- templates/act_handover.html` is a pure 2-line deletion of the Jinja gate; the interpolation line itself is untouched context. Repo state confirms zero `\| safe` filters in either act template body. | closed |
| T-35-15 | Repudiation | assertion weakening in G-03/WR-02 | mitigate | The change is strictly **stricter**: commit `5ab29c1` replaces `"Выдал"`/`"Получил"` with `"Выдал:"`/`"Получил:"`. The old form was a prefix of the fixture ФИО («Выдалов В.В.» / «Получилов П.П.») and could pass vacuously; the colon removes that false-positive path. `tests/html_act_render.rs:188`. | closed |
| T-35-16 | Information Disclosure | CSS-only edit of `.signature-row .signature-name` | accept | Premise verified: the edit changes three declarations only (`min-width: 0`, `white-space: normal`, `overflow-wrap: break-word`) inside an existing rule body. No render sink altered; escaping unchanged. `act_handover.html:117-121`, `act_acceptance.html:103-107`. | closed |
| T-35-17 | Tampering | markup or unrelated CSS accidentally touched in 35-07 | mitigate | Verified by mechanical diff against the v23 snapshot: `diff _legacy_defaults/v23/act_handover.html templates/act_handover.html` yields exactly `118c118,120` (one declaration → three) and nothing else; `act_acceptance.html` yields exactly `104c104,106`. The `<div class="signatures">` markup is byte-identical. | closed |
| T-35-18 | Repudiation | v23 snapshot could have been taken AFTER the CSS edit | mitigate | Proven by git blob identity, the strongest available evidence: in commit `f162c79`, the new-file blob of `_legacy_defaults/v23/act_handover.html` is `5f5fdee`, which is exactly the *pre-image* blob of `templates/act_handover.html` in the same commit (`index 5f5fdee..e411cd2`). Same for acceptance (`f2f35fb`). Snapshot demonstrably predates the edit. Backed by the precondition-guard test `upgrade_replaces_v23_legacy_default_with_current_bundled_body` (`src/pdf/html_templates.rs:537-585`, `assert_ne!` at :559) — **executed and passing**. | closed |
| T-35-SC | Information Disclosure | full phase diff → PUBLIC git repository | mitigate | Independently re-verified by this audit, not accepted from SUMMARY. (1) `scripts/check-privacy-requisites.sh` exists and exits 0 («Privacy gate OK»). (2) ФИО-shaped scan of the `crates/` diff yields only: Иванов И.И., Петров П.П., Выдалов В.В., Получилов П.П., Морозов М.М., Сидоров-Петроградский-Константинов Иван Александрович — all on the approved fictional list or pre-dating this phase (`Выдалов`/`Получилов` introduced in phases 16/20 per `git log -S`). The long fixture is self-documenting: `const LONG_GIVER_NAME_FICTIONAL` at `tests/pdf_render_act.rs:163`. (3) Requisite/org/address-shaped scan (`[0-9]{9,13}`, e-mail, `ООО\|ЗАО\|ОАО\|АО`, `ИНН\|КПП\|ОКПО\|ОГРН`, `ул.`, `г. X`) of the `crates/` diff returns **zero** hits. (4) `.planning/` diff scan returns only formulaic fictional names and fictional placeholders quoted inside privacy analysis prose. | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Verification Evidence — executed gates

All targeted test binaries were run during this audit (workspace-wide `cargo test`
deliberately avoided: the pre-existing `login_remember_persistent_cookie` hang).

| Command | Result | Threats covered |
|---------|--------|-----------------|
| `cargo test -p trackly-app --lib html_templates` | 14 passed, 0 failed — incl. `upgrade_replaces_v22_…` and `upgrade_replaces_v23_legacy_default_with_current_bundled_body` | T-35-02, T-35-18 |
| `cargo test -p trackly-app --test html_field_row_underline_gate` | 3 passed, 0 failed — incl. `signature_name_css_permits_wrap_for_long_names` | T-35-07, T-35-16, T-35-17 |
| `cargo test -p trackly-app --test html_act_render --test pdf_render_act` | 11 + 15 passed, 0 failed — incl. `render_handover_multi_device_fields_attributable_to_own_device`, `render_handover_with_long_giver_name_preserves_full_name_in_signature_block` | T-35-03, T-35-05, T-35-08, T-35-13, T-35-15 |
| `./scripts/check-privacy-requisites.sh` | exit 0 — «Privacy gate OK: all requisite literals are approved placeholders.» | T-35-SC |

Escaping baseline (unchanged by this phase, re-confirmed):
`crates/trackly-app/src/pdf/minijinja_env.rs:118-126` — `UndefinedBehavior::Strict`,
`AutoEscape::Html`, recursion limit 64, fuel 100 000, **no loader**.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-1 | T-35-02 | `_legacy_defaults/v22/` snapshots were produced by mechanical `cp`; verified byte-identical to the pre-phase templates, so no unreviewed content entered the compile-time `include_str!` surface. | Phase 35 plan 35-01 | 2026-08-11 |
| AR-2 | T-35-04 | Template text edits carry no data-disclosure risk: all field values come from the DB context, never from template literals. **Wording nuance:** the plan's premise «текст интро и лейблов не меняется» is imprecise — 35-02 did restructure labels into plain-text `field-row`s. The security-relevant part of the premise (no real org/personal literal introduced) is verified and holds. | Phase 35 plan 35-02 / auditor | 2026-08-12 |
| AR-3 | T-35-06 | Removing the duplicate «Кто передал»/«Кто принял» table rows is structural; the same two values still render once in the signature block. Disclosure surface narrows. | Phase 35 plan 35-03 | 2026-08-11 |
| AR-4 | T-35-07 | `html_field_row_underline_gate.rs` reads templates via compile-time `include_str!` only; contains no filesystem write API. | Phase 35 plan 35-04 | 2026-08-11 |
| AR-5 | T-35-13 | `{{ item.name }}` printed unconditionally per device-block reuses the Phase 16 T-16-01 sink under `AutoEscape::Html`; only a Jinja condition was removed. | Phase 35 plan 35-06 | 2026-08-11 |
| AR-6 | T-35-16 | CSS-only change to `.signature-row .signature-name` (`min-width`/`white-space`/`overflow-wrap`); no interpolation structure or escaping affected. | Phase 35 plan 35-07 | 2026-08-12 |

---

## Unregistered Flags

None. No SUMMARY in this phase declares a `## Threat Flags` section, and the audit
found no new attack surface outside the register: the only non-test runtime change
in `crates/` is one demo-context string literal (`template_service.rs:511`) plus
four `include_str!` registrations of static snapshot files
(`html_templates.rs:81-82,90-91`). No new I/O sink, no new untrusted-input parser,
no new network or filesystem entry point was introduced by this phase.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-08-12 | 16 | 16 | 0 | gsd-security-auditor |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-08-12
