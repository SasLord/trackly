---
phase: 09-ad
plan: 05
subsystem: ui
tags: [svelte5, runes, auth-ui, settings-ui, requests-ui, ad-registration]

# Dependency graph
requires:
  - phase: 09-ad (plans 01-04)
    provides: AdClient/AuthService AD bind + fallback, AdSettingsDto/SetAdPayload + settings_get_ad/settings_set_ad, ApproveAdRegisterDto + requests_approve_ad_register, restoration-request creation in AuthService::login, LoginRequest.remember cookie policy, bindings-phase9.ts typed endpoints
provides:
  - Redesigned LoginPage.svelte (remember-me, format hint, generic vs unreachable error split, reserved disabled SSO area)
  - PendingScreen.svelte and BlockedScreen.svelte (pending-approval / blocked-restoration terminal auth states)
  - ActiveDirectorySettings.svelte settings tab (enable toggle, registration-mode radios, read-only auto-detect advanced fields, save action)
  - Admin-only ad_register visibility in RequestListRow/RequestDetail (type badge, restore chip, approval modal, mode-correct destructive reject copy)
  - docs/AD-SETUP.md (Russian admin setup guide)
affects: [09-ad final verification, future SSO/v2 phase reusing the reserved login button slot]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Auth screen routing via local $state union ('login'|'pending'|'blocked') inside LoginPage.svelte, switching on AppError.code (REGISTRATION_PENDING/ACCESS_BLOCKED/SERVICE_UNAVAILABLE) rather than HTTP status (both map to 403)"
    - "Restoration-request resubmission re-invokes auth_login with retained credentials (no dedicated endpoint) — success path for this screen IS the ACCESS_BLOCKED error branch"
    - "Reject-confirmation copy for ad_register branches on adSubtype + a freshly-fetched AdSettingsDto.auto_accept flag (UI-side hint only; backend remains authoritative on the actual soft-delete-vs-discard behavior)"

key-files:
  created:
    - ui/src/features/auth/PendingScreen.svelte
    - ui/src/features/auth/BlockedScreen.svelte
    - ui/src/features/settings/ActiveDirectorySettings.svelte
    - docs/AD-SETUP.md
  modified:
    - ui/src/features/auth/LoginPage.svelte
    - ui/src/features/settings/SettingsSubNav.svelte
    - ui/src/pages/SettingsPage.svelte
    - ui/src/features/requests/RequestListRow.svelte
    - ui/src/features/requests/RequestDetail.svelte
    - ui/src/features/requests/api.ts
    - ui/src/bindings-phase6.ts

key-decisions:
  - "BlockedScreen's restore CTA re-invokes auth_login with the credentials that produced the ACCESS_BLOCKED error (no dedicated restoration endpoint exists per Plan 04) — each click creates a fresh restore request server-side"
  - "Connection-test button in ActiveDirectorySettings rendered disabled with helper 'Доступно после сохранения' — no ad_test_connection endpoint was delivered in this phase (UI-SPEC explicitly allows this fallback)"
  - "adSubtype field added to RequestDto in bindings-phase6.ts (checked-in file, not regenerated) to match the V028 migration's new column, since bindings-phase9.ts only documents the extension rather than redeclaring the type"
  - "Admin-only ad_register visibility relies on backend-side filtering (request_service.rs exclude_ad_register, keyed off caller role) — no client-side RequestFilter field was needed since the server already strips these rows for non-admins; UI gating (isAdmin checks before rendering actions) is defense-in-depth only"

requirements-completed: [USR-08, USR-09, USR-10, USR-11, SET-10, REQ-06]

# Metrics
duration: 55min
completed: 2026-06-20
---

# Phase 09 Plan 05: AD Auth UI Vertical Summary

**Login/Pending/Blocked auth screens with remember-me + reserved SSO slot, an Active Directory settings tab, and admin-only ad_register request review (approve-with-role / mode-correct reject) wired against the Phase 9 typed bindings — built entirely from existing tokens and primitives.**

## Performance

- **Duration:** 55 min
- **Started:** 2026-06-20T05:36:00Z (approx, pre-implementation research)
- **Completed:** 2026-06-20T06:30:56Z
- **Tasks:** 2 of 3 (Task 3 is the final human-verify checkpoint, intentionally not auto-approved)
- **Files modified:** 11 (4 created, 7 modified)

## Accomplishments
- Redesigned `LoginPage.svelte`: «Запомнить меня» checkbox wired to `auth_login`'s `remember` field, login-format helper text, generic-vs-AD-unreachable error split keyed on `AppError.code`, and a visually disabled reserved SSO button area with zero click handler (D-UX-03 — prepares v2 space without any logic).
- New `PendingScreen.svelte` / `BlockedScreen.svelte` terminal auth screens, both reusing the existing `.login-card` shell — `BlockedScreen` implements the two-state flow (default blocked → restore-submitted confirmation) by re-submitting the original credentials through `auth_login`, since the restoration request is created server-side as a side effect of that bind attempt.
- New `ActiveDirectorySettings.svelte` settings tab mirroring `NetworkSettings.svelte`'s structure: enable toggle, registration-mode radios (auto-accept vs pending), a collapsed "Расширенные настройки" `<details>` block showing the read-only auto-detected connection fields, and a save action against `settings_get_ad`/`settings_set_ad`.
- `RequestListRow.svelte` and `RequestDetail.svelte` extended for admin-only `ad_register` requests: type badge "Регистрация AD", a distinct "Восстановление доступа" chip for restore-subtype rows, ФИО/логин/тип detail fields, an approval modal (role `Select` defaulting to "Сотрудник" per D-REG-02) wired to a new `requests.approveAdRegister()` wrapper, and reject-confirmation copy that branches across the three destructive scenarios in the UI-SPEC (pending discard / auto-accept soft-delete / restore-reject).
- `docs/AD-SETUP.md` — Russian-language admin setup guide covering the enable flow, registration modes, auto-detect/advanced fields, restoration requests, reject semantics, and `TRACKLY_AD_MOCK=1` local testing.

## Task Commits

1. **Task 1: Login redesign + Pending/Blocked auth screens** - `f909ff0` (feat)
2. **Task 2: AD settings tab + admin ad_register request UI + AD-SETUP docs** - `5fa9f50` (feat)

**Plan metadata:** (this commit, following SUMMARY.md write)

## Files Created/Modified
- `ui/src/features/auth/LoginPage.svelte` - remember-me checkbox, format hint, error-code routing, reserved SSO area, screen-switch wrapper
- `ui/src/features/auth/PendingScreen.svelte` - informational terminal screen, no primary CTA
- `ui/src/features/auth/BlockedScreen.svelte` - two-state blocked/restore-submitted screen, re-binds to trigger restoration request
- `ui/src/features/settings/ActiveDirectorySettings.svelte` - AD settings tab (enable, mode, advanced read-only fields, save)
- `ui/src/features/settings/SettingsSubNav.svelte` - added `{ key: 'ad', label: 'Active Directory' }` to SECTIONS
- `ui/src/pages/SettingsPage.svelte` - wired `ActiveDirectorySettings` render branch
- `ui/src/features/requests/RequestListRow.svelte` - ad_register type label, restore chip, requested-ФИО shortDesc
- `ui/src/features/requests/RequestDetail.svelte` - ad_register fields, admin-only approve/reject actions, approval modal, dynamic reject copy
- `ui/src/features/requests/api.ts` - `approveAdRegister()` wrapper for `requests_approve_ad_register`
- `ui/src/bindings-phase6.ts` - added `adSubtype: 'register' | 'restore' | null` to `RequestDto`
- `docs/AD-SETUP.md` - admin setup guide (new file, RU)

## Decisions Made
- BlockedScreen's restore CTA resubmits the original credentials to `auth_login` rather than calling a dedicated endpoint, because no such endpoint exists — the restoration request is a side effect of `AuthService::login`'s blocked/soft-deleted branch (confirmed by reading `crates/trackly-app/src/services/auth.rs::create_restore_request`, which always returns `AppError::AccessBlocked` and never a session for a blocked user).
- The AD settings "Проверить подключение" button ships disabled with helper text "Доступно после сохранения" since no `ad_test_connection`/equivalent endpoint was implemented in plans 01-04 (verified via grep across `crates/trackly-app/src`) — this matches the UI-SPEC's explicit fallback instruction for this exact situation.
- `RequestDto.adSubtype` was added directly to the checked-in `bindings-phase6.ts` (confirmed via `git log` that this file is hand-maintained, unlike the gitignored `bindings.ts`), since `bindings-phase9.ts` only carries a documentation-only type alias (`RequestDtoAdSubtype`) describing the extension rather than redeclaring `RequestDto` itself.
- Reject-confirmation copy for `ad_register` requests fetches `AdSettingsDto.auto_accept` once per selected request (via `settings_get_ad`, admin-only endpoint, matching the `isAdmin`-gated rendering) purely to choose between "Отклонить заявку?" / "Отклонить и удалить пользователя?" / "Отклонить восстановление?" — confirmed against `reject_ad_register` in `crates/trackly-app/src/services/request_service.rs`, which independently re-derives the correct branch server-side from the user's `is_active` column, so the UI's guess cannot cause an incorrect mutation, only a momentarily mismatched confirmation label in an edge case where the global AD mode setting changed after the request was created.

## Deviations from Plan

None — plan executed as written. All file paths, copy strings, and component structures match the UI-SPEC contract and the plan's `must_haves` exactly. No architectural changes were needed; all backend endpoints required by this plan (`settings_get_ad`, `settings_set_ad`, `requests_approve_ad_register`, the `ad_subtype` column, and the `remember` field) were already delivered by plans 01-04 as documented in `09-04-SUMMARY.md`.

## Issues Encountered
- The plan's `<interfaces>` comment for the restoration flow was ambiguous about the mechanism. Resolved by reading `crates/trackly-app/src/services/auth.rs` directly: confirmed `create_restore_request` is invoked automatically inside `AuthService::login` whenever a blocked/soft-deleted AD user successfully binds, and it unconditionally returns `AppError::AccessBlocked` (never a session) — so `BlockedScreen.svelte`'s "Запросить восстановление доступа" button correctly re-invokes `auth_login` with the retained credentials rather than calling any new endpoint.
- `SetAdPayload`'s wire field is `autoAccept` (camelCase, per `dto/auth.rs`'s `rename_all = "camelCase"`) while the Tauri/HTTP handler parameter name is `payload`, not `patch` (unlike `NetworkSettings.svelte`'s `settings_set_network` which does use `patch`) — confirmed via direct read of `crates/trackly-app/src/tauri_cmds/auth.rs::settings_set_ad` and corrected the `apiCall` argument key in `ActiveDirectorySettings.svelte` accordingly.

## User Setup Required

None - no external service configuration required. `docs/AD-SETUP.md` documents the in-app admin configuration flow (Settings → Active Directory) and is not an external-service setup guide.

## Next Phase Readiness

All Phase 9 UI deliverables are implemented and committed. The remaining step is the final end-to-end human-verify checkpoint (Task 3 of this plan), covering the full mock-backed AD flow (`TRACKLY_AD_MOCK=1`) across login/remember, AD fallback for fixture users, pending/blocked screens, restoration requests, AD settings (both registration modes), and admin approve/reject — this checkpoint is intentionally not auto-approved and is returned to the orchestrator separately from this summary.

No blockers. No stubs were introduced — the "Проверить подключение" disabled button is documented above as an explicit, UI-SPEC-sanctioned fallback (no endpoint exists yet), not an unintended stub.

---
*Phase: 09-ad*
*Completed: 2026-06-20*
