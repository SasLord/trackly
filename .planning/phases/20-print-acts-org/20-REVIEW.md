---
phase: 20-print-acts-org
reviewed: 2026-07-14T00:00:00Z
depth: standard
files_reviewed: 16
files_reviewed_list:
  - crates/trackly-app/src/context.rs
  - crates/trackly-app/src/dto/reports.rs
  - crates/trackly-app/src/pdf/html_templates.rs
  - crates/trackly-app/src/services/act_service.rs
  - crates/trackly-app/src/services/org_db_service.rs
  - crates/trackly-app/src/services/report_service.rs
  - crates/trackly-app/src/services/template_service.rs
  - crates/trackly-app/templates/act_acceptance.html
  - crates/trackly-app/templates/act_handover.html
  - crates/trackly-app/templates/report.html
  - crates/trackly-app/tests/html_act_render.rs
  - crates/trackly-app/tests/html_report_render.rs
  - crates/trackly-app/tests/org_settings.rs
  - crates/trackly-app/tests/pdf_render_act.rs
  - migrations/V035__org_settings_address_line2.sql
  - ui/src/features/settings/OrgSettings.svelte
findings:
  critical: 0
  warning: 3
  info: 3
  total: 6
status: issues_found
---

# Phase 20: Code Review Report

**Reviewed:** 2026-07-14T00:00:00Z
**Depth:** standard
**Files Reviewed:** 16
**Status:** issues_found

## Summary

Reviewed the three Phase-20 deliverables (PRN-01 acceptance-PDF org-header parity,
ORG-01 SVG-logo-as-data-URI XSS invariant, ORG-02 `address_line2` end-to-end).
The core correctness and security invariants the phase set out to establish all
**hold**:

- **SQL binding correctness (org_db_service.rs):** `get`, `get_for_pdf`, and
  `save_fields` column/ordinal positions all line up (verified index-by-index).
  `address_line2` is appended last in every SELECT/UPDATE, matching V035's
  append-only migration and leaving pre-existing ordinals untouched. The `?1 =
  1i64` no-op bind idiom is consistent with the rest of the module (WHERE uses
  the literal `id=1`).
- **Byte-identity fail-closed upgrade (html_templates.rs):** the
  `upgrade_untouched_defaults_on_startup` logic is sound — it overwrites only
  when on-disk content is byte-identical to a `KNOWN_LEGACY_DEFAULTS` snapshot,
  leaves anything else (user-customized) untouched, and no-ops when already
  current. Confirmed the committed `_legacy_defaults/v20/*` snapshots differ
  from the current bundled bodies **only** by the Phase-20 `address_line2`
  line + doc-comments, so genuine pre-20 installs are correctly recognized.
  Line-ending drift (a theoretical byte-mismatch source) is neutralized by the
  repo's `.gitattributes` (`* text=auto eol=lf`), and the worst case is a
  *missed* upgrade, never a wrongful overwrite.
- **XSS invariant (templates + render paths):** the logo is embedded only as
  `<img src="{{ org.logo_data_uri | safe }}">`, where the value is
  server-constructed base64 (RFC-4648 alphabet) + a write-time mime allowlist —
  never user HTML. All org text fields (`address_line2` included) use plain
  autoescaped `{{ }}`. The adversarial `<script>`-in-SVG test locks this.

No BLOCKER-level defects found. The findings below are defense-in-depth /
consistency gaps and minor quality issues.

## Warnings

### WR-01: Read-time logo-mime allowlist enforced in `report_service` but NOT in `act_service` render paths

**File:** `crates/trackly-app/src/services/act_service.rs:2572-2579` (render_pdf) and `:2724-2731` (render_acceptance_pdf)
**Issue:** `ReportService::export_pdf` deliberately re-validates the stored logo
mime against the `image/png | image/jpeg | image/svg+xml` allowlist on *read*
before interpolating it into the `data:{mime};base64,...` URI (the WR-05 fix,
guarded by `html_report_disallowed_logo_mime_drops_logo`). Both act render
paths omit this identical read-time check and pass `logo_mime` straight through:

```rust
let mime = logo_mime.as_deref().unwrap_or("image/png");
format!("data:{mime};base64,{}", ...encode(bytes))
```

The `mime` string is placed into an HTML attribute under `| safe` (autoescape
off). Today this is not exploitable — the only write paths (`OrgDbService::save_logo`
exact-match allowlist, and `migrate_from_org_json`'s hardcoded literals) cannot
store a hostile mime — so the invariant currently rests entirely on the write
side. But the two render paths make asymmetric trust assumptions about the same
column, and `report.html`/`act_handover.html` both carry doc-comments claiming
the allowlist is enforced "in the service." For acts that claim is only true on
write, not on read.
**Fix:** Mirror the `report_service.rs:573-582` guard in both act render paths —
drop the logo (set `logo_bytes = None`) when an explicit `logo_mime` is present
and not in the allowlist, before building the data URI:

```rust
let logo_mime_ok = logo_mime.as_deref()
    .map(|m| matches!(m.to_lowercase().as_str(),
        "image/png" | "image/jpeg" | "image/svg+xml"))
    .unwrap_or(true);
let logo_bytes = if logo_mime_ok { logo_bytes } else { None };
```

### WR-02: `OrgDbService::save_logo` runs input validation before authorization

**File:** `crates/trackly-app/src/services/org_db_service.rs:126-155`
**Issue:** `save_logo` performs the 512-KiB size check (`:132`) and the mime
allowlist check (`:142-154`) **before** calling `authorize(caller, &Action::ManageSettings)`
(`:155`). Every sibling mutator in the same file authorizes first: `save_fields`
(`:91`), `remove_logo` (`:174`). This lets an unauthorized caller distinguish
"acceptable size+mime" from "rejected" via the returned `Validation` error
before the RBAC gate ever runs — a least-privilege ordering violation and an
inconsistency within the module. Impact is low (the write itself is still
blocked by the later `authorize`), but the check order should match the rest of
the service.
**Fix:** Move `authorize(caller, &Action::ManageSettings)?;` to the first line
of `save_logo`, ahead of the size and mime validation.

### WR-03: `render_acceptance_pdf` / `render_pdf` unconditionally read `org.json` (legacy) with `?` even when `org_db` is the real source

**File:** `crates/trackly-app/src/services/act_service.rs:2545` (render_pdf), `:2696` (render_acceptance_pdf)
**Issue:** Both paths execute `let org_legacy = pipeline.organization.read().await?;`
unconditionally, but `org_legacy` is consumed **only** inside the `None` branch
(the org_db-not-wired fallback). In production `org_db` is always `Some`, so the
legacy `org.json` read is pure overhead — and worse, it propagates with `?`, so
a malformed/locked `org.json` on disk would fail the entire acceptance/handover
render even though `org_settings` (the authoritative source) is available. This
pattern predates Phase 20 in `render_pdf`, but Phase 20 newly copied it verbatim
into `render_acceptance_pdf`, doubling the surface. `OrganizationService::read()`
tolerates a missing file today (tests rely on it), so this is latent rather than
live, but it is needless coupling to the frozen legacy path.
**Fix:** Read `org_legacy` lazily only inside the `None` match arm, e.g. move the
`.read().await?` into that branch so the org_db-Some path never touches
`org.json`.

## Info

### IN-01: `get_for_pdf` redundantly selects `has_logo` alongside `logo_blob`

**File:** `crates/trackly-app/src/services/org_db_service.rs:373-397`
**Issue:** The query selects both `(logo_blob IS NOT NULL) as has_logo` (index 4)
and the full `logo_blob` (index 5). Since the caller already receives the raw
blob and can trivially derive presence, `has_logo` in the PDF tuple's DTO is
never meaningfully consumed by the render paths (they branch on
`logo_bytes: Option<Vec<u8>>`). Harmless but dead-ish column.
**Fix:** Optional — either drop the `has_logo` expression from the `get_for_pdf`
SELECT, or leave a comment noting it is only populated to satisfy the shared
`OrgSettingsDto` shape.

### IN-02: Duplicated `logo_data_uri` construction block across three render sites

**File:** `crates/trackly-app/src/services/act_service.rs:2572-2579`, `:2724-2731`; `crates/trackly-app/src/services/report_service.rs:583-590`
**Issue:** The `Option<Vec<u8>> + Option<String>` → `data:{mime};base64,...`
conversion is copy-pasted in three places. Because WR-01 above shows the three
copies have already drifted (report has the mime re-check, acts don't), the
duplication is actively causing the inconsistency, not just cosmetic.
**Fix:** Extract a single helper, e.g.
`fn logo_data_uri(bytes: Option<Vec<u8>>, mime: Option<String>) -> Option<String>`
that enforces the allowlist once, and call it from all three sites. This
resolves WR-01 structurally.

### IN-03: `KNOWN_LEGACY_DEFAULTS` extension discipline depends on manual snapshotting

**File:** `crates/trackly-app/src/pdf/html_templates.rs:60-79`
**Issue:** The auto-upgrade correctness depends on every future bundled-template
change also capturing the pre-change body into a new `_legacy_defaults/vNN/`
snapshot and registering it here (the module doc-comment at `:53-59` states
this). This is correct for Phase 20 but is an easy step to forget in a later
phase; a miss silently degrades to "existing installs stop receiving upgrades"
with no test or compile-time signal. Consider a test that asserts each
registered legacy body differs from the current default only in expected ways,
or a CI check that fails when `DEFAULT_HTML_TEMPLATES` bodies change without a
corresponding new `KNOWN_LEGACY_DEFAULTS` entry.
**Fix:** Add a guard test/CI check tying default-body changes to legacy-snapshot
registration (nice-to-have; not blocking).

---

_Reviewed: 2026-07-14T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
