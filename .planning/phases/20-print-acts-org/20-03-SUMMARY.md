---
phase: 20-print-acts-org
plan: 03
subsystem: pdf-render
tags: [minijinja, html-render, org-settings, templates]

requires:
  - phase: 20-print-acts-org (plan 02)
    provides: address_line2 present in ctx for all three render paths (render_pdf, render_acceptance_pdf, report_service::export_pdf)
provides:
  - act_acceptance.html .requisites block at full field parity with act_handover.html (name/inn+kpp/address/address_line2/phone/fax/email/okpo/ogrn)
  - address_line2 guarded display line in all three printed templates (act_handover.html, act_acceptance.html, report.html)
affects: [20-04, 20-05, 20-06]

tech-stack:
  added: []
  patterns:
    - "address_line2 empty-guard rendering idiom ({%- if org.address_line2 %}<div>{{ org.address_line2 }}</div>{%- endif %}) now present identically in all three print templates, positioned right after org.address"

key-files:
  created: []
  modified:
    - crates/trackly-app/templates/act_acceptance.html
    - crates/trackly-app/templates/act_handover.html
    - crates/trackly-app/templates/report.html

key-decisions: []

requirements-completed: [PRN-01, ORG-02]

duration: ~10min
completed: 2026-07-14
---

# Phase 20 Plan 03: act_acceptance.html requisites parity + address_line2 template display Summary

**Brought act_acceptance.html's `.requisites` header block to full field parity with act_handover.html (name/inn+kpp/address/phone/fax/email/okpo/ogrn), and added the `address_line2` guarded display line to all three printed templates (act_handover.html, act_acceptance.html, report.html), closing the visual side of PRN-01/ORG-02 now that Plan 20-02 already threads the data into ctx.**

## Performance

- **Duration:** ~10 min
- **Completed:** 2026-07-14
- **Tasks:** 2 completed
- **Files modified:** 3

## Accomplishments
- `act_acceptance.html`'s header ("Печать документа приёма") now shows the exact same set of organizational requisites as `act_handover.html` — phone, fax, email, ОКПО, ОГРН join the pre-existing name/ИНН/КПП/address fields, closing the visual root cause of PRN-01.
- `address_line2` renders as a guarded second address line (`{% if %}`-hidden when empty) immediately under the main address in all three print templates: `act_handover.html`, `act_acceptance.html`, `report.html` — uniform D-06 idiom, verified byte-for-byte identical insertion point in all three.
- Logo `<img src="{{ org.logo_data_uri | safe }}">` embedding left untouched in `act_acceptance.html` (ORG-01/D-08 invariant preserved, out of scope for this plan).
- Doc-comment context-variable lists updated in all three templates to mention the new/expanded field set.

## Task Commits

Each task was committed atomically:

1. **Task 1: act_acceptance.html — full .requisites parity block + address_line2 (D-01/D-06)** - `4564111` (feat)
2. **Task 2: act_handover.html + report.html — insert address_line2 line (D-06)** - `1343ec3` (feat)

**Plan metadata:** (this commit, see below)

## Files Created/Modified
- `crates/trackly-app/templates/act_acceptance.html` - `.requisites` block extended to full 10-field org header (name/inn/kpp/address/address_line2/phone/fax/email/okpo/ogrn), copied verbatim from `act_handover.html`; doc-comment updated
- `crates/trackly-app/templates/act_handover.html` - one-line `address_line2` guard inserted between `org.address` and `org.phone`; doc-comment context list updated
- `crates/trackly-app/templates/report.html` - identical one-line `address_line2` insertion + doc-comment update, mirroring `act_handover.html`

## Decisions Made
None — plan executed exactly as written; all edits followed the plan's verbatim block/idiom guidance from `20-PATTERNS.md`.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Verification

- `grep -c "org.phone"` / `grep -c "org.address_line2"` / `grep -c '<img src="{{ org.logo_data_uri | safe }}"'` on `act_acceptance.html` → 1/1/1 (all fields present, logo line unchanged).
- `grep -n` on `act_handover.html` and `report.html` confirms `org.address_line2` line sits directly between `org.address` and `org.phone` in both files' `.requisites` block.
- `cargo test -p trackly-app --test html_act_render` — 8/8 passed (including `html_acceptance_contains_required_blocks`, `html_handover_contains_required_blocks_and_logo`, `html_is_offline_safe_no_external_links`).
- `cargo test -p trackly-app --test html_report_render` — 7/7 passed (including `html_report_org_header_present`).

## Known Stubs

None — all three templates render the full org-header field set with real data flowing from ctx (Plan 20-02's backend work); no placeholder/empty values introduced.

## Threat Flags

None. T-20-03-01 (address_line2 interpolated without `| safe`, autoescape-protected) and T-20-03-02 (logo `<img>` embedding, unchanged pre-existing accept-disposition) are both satisfied by the implementation as written — no new surface introduced beyond what the plan's threat model already accounted for.

## Next Phase Readiness

- All three print templates now visually match on org-header content; PRN-01's visual parity requirement and ORG-02's address_line2 display requirement are both satisfied at the template layer.
- Existing installs still need Plan 20-06's `upgrade_untouched_defaults_on_startup` mechanism (D-12) to receive these bundle-default template edits automatically — that remains a downstream plan, not a blocker for this one (fresh installs and file-absent cases already pick up these changes via `materialize_defaults_on_startup`).
- No blockers for Plan 20-04/20-05/20-06.

---
*Phase: 20-print-acts-org*
*Completed: 2026-07-14*

## Self-Check: PASSED

- FOUND: crates/trackly-app/templates/act_acceptance.html
- FOUND: crates/trackly-app/templates/act_handover.html
- FOUND: crates/trackly-app/templates/report.html
- FOUND commit 4564111 (Task 1)
- FOUND commit 1343ec3 (Task 2)
