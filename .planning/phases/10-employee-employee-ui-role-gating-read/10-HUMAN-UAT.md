---
status: partial
phase: 10-employee-employee-ui-role-gating-read
source: [10-VERIFICATION.md]
started: 2026-06-21
updated: 2026-06-21
---

## Current Test

[awaiting human testing]

## Tests

### 1. Employee shell renders (D-UI-01)
expected: After logging in as an `employee`-role user (server mode, browser), the user lands on a minimal **header-based** EmployeeLayout shell (Trackly brand + user name + «Сотрудник» + theme switcher + logout) — NOT the admin/manager sidebar shell. Landing page shows «Заявки» (own requests) with a «Мои заявки» StatWidget summary card (Новые / В работе / Выполнено) reflecting only this employee's requests, and a working «Новая заявка» / «Создать заявку» action. No navigation to other sections is visible.
result: [pending]

### 2. Access-denied screen + 403 handling (D-DENY-01)
expected: As the employee, directly editing the URL hash to `#/devices`, `#/acts`, `#/cartridges`, `#/printers`, `#/reports`, `#/users`, `#/settings`, `#/map` each shows the «Нет доступа» screen («У вашей роли («Сотрудник») нет доступа к этому разделу. Доступны только заявки.») with a «К заявкам» button that returns to requests. A gated API call returning 403 surfaces a toast («Недостаточно прав для этого действия») without crashing/blanking/redirecting to login. Admin/Manager are unaffected (full sidebar, no «Мои заявки» card).
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps

## Setup note

Run `pnpm --dir ui build` before testing in server/LAN-browser mode — `ui/dist` is served there and is stale until rebuilt (the executor already ran this once after the last frontend change). For AD-based employee login, set `TRACKLY_AD_MOCK=1` on the dev box (AD is unreachable here), or create a local employee user via the admin Users page.
