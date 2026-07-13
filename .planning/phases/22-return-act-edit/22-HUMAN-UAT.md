---
status: partial
phase: 22-return-act-edit
source: [22-VERIFICATION.md]
started: 2026-07-13T15:04:33Z
updated: 2026-07-13T15:04:33Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Visual prefill fidelity of the return-edit dialog
expected: Open the edit dialog for an existing return act. The title reads «Возврат по акту №XXX», and every prefilled value — состав возвращаемых устройств, состояние (condition) и локация каждой позиции, ФИО сдавшего/принявшего, дата возврата — matches exactly what was saved at the time the return was created (un-swapped giver/receiver, the return's own date, not the parent handover's).
result: [pending]

### 2. Reactive save-and-refresh UX after editing a return
expected: Change a return in the edit dialog (e.g. edit a device's condition, un-return a device, or add an outstanding one) and save. The save completes without errors, and the act detail view updates immediately to reflect the new state (device statuses/locations, archived flag on the parent) without a manual reload. Verify in a live browser/webview session (LAN server mode uses ui/dist — rebuild with `pnpm --dir ui build` if testing that path).
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
