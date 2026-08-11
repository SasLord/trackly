---
status: resolved
phase: 34-document-header
source: [34-VERIFICATION.md]
started: 2026-08-11T06:45:00Z
updated: 2026-08-11T07:30:00Z
---

## Current Test

[complete — both items confirmed by the user on 2026-08-11]

## Tests

### 1. Re-confirm visual header parity on the CURRENT code (desktop Tauri webview + LAN browser)

Cover both the empty-`org.full_name` case (C-01) and a filled-`org.full_name` case, on all three
print forms (акт приёма-передачи, акт приёмки, отчёт).

expected: No stray leading `<br />` and no orphan `()` around the short name when `full_name` is
empty; when `full_name` is set, the full name renders with the short name in parentheses on its own
line below it; identical geometry and typography across all three forms; no overflow on a long
`full_name` or address.

why_human: The only completed human checkpoint (34-06 Tasks 1/2, approved at commit `2038f4e`)
predates the CR-01 fix (commit `e306b77`) by ~9 hours. CR-01 was a real shipped visual defect —
unguarded `org.full_name` producing a stray `<br />` and orphan parentheses on every upgraded
install with an empty `full_name` — which directly contradicts the C-01 behaviour the sign-off
describes. The sign-off was made against pre-fix rendering (and, in the first round, against the
stale hand-edited `target/debug/templates/act_handover.html`), not against the code as it exists
today. An automated regression test now encodes the correct behaviour and is designed to fail on
the pre-fix template, but the phase's own Success Criterion #2 explicitly requires confirmation via
a real rendered PDF/preview rather than a text/DOM assertion — overlap, overflow and font
rendering are invisible to extraction-based tests (project convention, `act-pdf-word-fidelity`).

result: passed — confirmed by the user in the running app (desktop + LAN browser) after the CR-01 fix landed

### 2. Look at the `templates_status` badge in Settings → Шаблоны in the running app

The WR-05 badge («изменён вручную» / «файл не читается») beside the template selector.

expected: Badge renders correctly for each of the three editable kinds, updates after save/reset,
and does not block editing when the status fetch fails.

why_human: WR-05 was added during the code-review fix pass, after the 34-06 UAT checkpoints had
already been approved. It is a scope addition beyond plan 34-05's explicit "no UI is added in this
phase" boundary — a reasonable response to a dead-API-surface review finding, not a defect, but it
has only been type-checked, linted and built, never looked at running.

result: passed — confirmed by the user in the running app (desktop + LAN browser) after the CR-01 fix landed

## Summary

total: 2
passed: 2
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps
