---
phase: 29-login-and-employee-shell
reviewed: 2026-07-24T00:00:00Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - ui/src/features/auth/BlockedScreen.svelte
  - ui/src/features/auth/FirstRunWizard.svelte
  - ui/src/features/auth/LoginPage.svelte
  - ui/src/features/auth/PendingScreen.svelte
  - ui/src/features/layout/EmployeeLayout.svelte
  - ui/src/lib/components/AuthShell.svelte
  - ui/src/lib/components/FormField.svelte
  - ui/src/lib/components/Input.svelte
findings:
  critical: 0
  warning: 2
  info: 3
  total: 5
status: issues_found
---

# Phase 29: Code Review Report

**Reviewed:** 2026-07-24T00:00:00Z
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Summary

This phase migrates the four auth screens (Login, FirstRunWizard, Pending, Blocked) and the employee shell onto shared design-system primitives (`AuthShell`, `FormField`, `Input`, `Button`, `Checkbox`) with SCSS design tokens. I traced the migration diffs against their pre-migration versions and the auth/WS/error backend contracts.

The migration is faithful: auth-routing logic (`screen` state, `REGISTRATION_PENDING` / `ACCESS_BLOCKED` / `SERVICE_UNAVAILABLE` / `GENERIC_AUTH_ERROR`) is untouched, all referenced CSS tokens (`--tr-danger-text`, `--tr-danger-ring`, `--tr-surface-raised`, `--header-height`, etc.) resolve in `_tokens.scss`, aria wiring (`FormField` → `describedBy`/`invalid` → `Input`) is correct and label `for`/`id` pairs match, and no security defects were found (no secrets, no `innerHTML`/`eval`; the WS `Notification` body is plain text per T-11-03-T).

No BLOCKER-class defects. The findings are edge-case correctness and consistency issues, two of which (WS refcount leak, empty-string `rejection_reason`) are latent contract mismatches rather than markup regressions.

## Warnings

### WR-01: WS refcount leak when EmployeeLayout unmounts before `connectWs()` resolves

**File:** `ui/src/features/layout/EmployeeLayout.svelte:63-80`
**Issue:** `connectWs()` (see `ui/src/lib/api/ws.ts:115`) increments the shared `refCount` **synchronously** on its first line and resolves asynchronously with the teardown function. In `onMount`, the teardown is captured into `unlisten` only inside `.then(...)`. The `onMount` cleanup closure captures `unlisten` by reference, but if the component unmounts before the `connectWs()` promise resolves, the cleanup runs while `unlisten` is still `undefined` — so `unlisten?.()` is a no-op. The later-resolved teardown is then stored into `unlisten` but never invoked. Because `connectWs` already bumped `refCount`, the refcounted singleton never gets decremented back to zero, so the WebSocket / Tauri listener is never torn down (leaked subscription; on the browser path it also keeps the reconnect machinery alive). Repeated fast mount/unmount cycles ratchet `refCount` upward permanently.
**Fix:** Guard with a disposed flag so a teardown that arrives after unmount is invoked immediately:
```ts
onMount(() => {
  if (authStore.user?.role !== 'employee') return;
  let disposed = false;
  let unlisten: (() => void) | undefined;
  connectWs()
    .then((fn) => {
      if (disposed) fn(); // unmounted before resolve — tear down now
      else unlisten = fn;
    })
    .catch(() => {});
  const unsubscribe = onWsEvent(handleEmployeeWsEvent);
  return () => {
    disposed = true;
    unsubscribe();
    unlisten?.();
  };
});
```

### WR-02: BlockedScreen misclassifies a rejected request when `rejection_reason` is an empty string

**File:** `ui/src/features/auth/BlockedScreen.svelte:78` (and `ui/src/features/auth/LoginPage.svelte:79-80`)
**Issue:** BlockedScreen selects its state with a truthiness test: `{:else if blockedDetails.rejection_reason}`. LoginPage normalizes the payload with `typeof details?.rejection_reason === 'string' ? details.rejection_reason : null`, which **preserves the empty string `""`**. The backend derives `rejection_reason` from `resolution_notes` (`crates/trackly-app/src/services/auth.rs:676-679`), which is free-form and can be an empty string when an admin rejects without meaningful notes. In that case `pending === false` and `rejection_reason === ""` (falsy), so BlockedScreen skips the "Запрос отклонён" branch and falls through to the first-time `{:else}` branch — rendering "Доступ закрыт" with a «Запросить восстановление доступа» CTA instead of "Запрос отклонён" + «Запросить снова». The user is shown a first-time-request screen for an already-rejected request. Impact is cosmetic-to-misleading (both branches still call `handleRestoreRequest`), but the displayed state is wrong and the null/empty ambiguity is a loose contract boundary.
**Fix:** Distinguish "no request" (`null`) from "rejected without a reason" (`""`) explicitly. Either treat "has a rejected request" as its own boolean from the backend, or test for presence rather than truthiness:
```svelte
{:else if blockedDetails.rejection_reason !== null}
  <h1 class="login-title">Запрос отклонён</h1>
  <p class="screen-body">
    Запрос на восстановление доступа отклонён.{#if blockedDetails.rejection_reason} Причина: {blockedDetails.rejection_reason}{/if}
  </p>
```
And in LoginPage keep the `null` vs `""` distinction rather than collapsing them.

## Info

### IN-01: LoginPage dropped input placeholders during migration (inconsistent with FirstRunWizard)

**File:** `ui/src/features/auth/LoginPage.svelte:115-123, 129-137`
**Issue:** The pre-migration login/password `<input>`s had `placeholder="Логин"` / `placeholder="Пароль"`; the migrated `<Input>` calls pass no `placeholder`. `Input.svelte` still supports the prop, and FirstRunWizard *does* pass placeholders (e.g. "Логин (не менее 3 символов)"). This is an unintended, inconsistent behavior change for a phase described as markup/CSS-only. Removing placeholder-as-label is defensible a11y-wise (visible `FormField` labels exist), so this is cosmetic — flagging for intentional confirmation, not correctness.
**Fix:** Either re-add placeholders to LoginPage for parity, or drop them from FirstRunWizard too so the auth screens are consistent. Confirm the removal was intended.

### IN-02: FirstRunWizard can orphan an admin user if auto-login fails after creation

**File:** `ui/src/features/auth/FirstRunWizard.svelte:56-84`
**Issue:** `handleSubmit` calls `users_create` then `auth_login` in the same `try`. If `users_create` succeeds but `auth_login` throws (transient error), the admin account is already persisted, but the UI shows a generic error and stays on the wizard. A retry re-runs `users_create` with the same login, which will now fail on duplicate-login, permanently trapping the user on the wizard with a confusing error despite a valid account existing. Pre-existing logic (unchanged by this phase), but worth noting.
**Fix:** On `auth_login` failure after a successful create, either redirect to the login screen with an informational message, or make the wizard retry `auth_login` alone (skip re-create when the login already exists). Distinguish "duplicate login" from other create errors in the catch.

### IN-03: `AuthShell` `maxWidth` is an unvalidated raw number interpolated into a style

**File:** `ui/src/lib/components/AuthShell.svelte:17,21`
**Issue:** `style:max-width="{maxWidth}px"` interpolates the numeric prop directly. All current call sites pass literals (360 default, 400 in FirstRunWizard), so there is no live risk, but a non-finite or negative value would produce an invalid CSS declaration silently. Low priority — noted for defensiveness only.
**Fix:** None required for current usage. If the component becomes more widely reused, clamp/validate: `const w = Number.isFinite(maxWidth) && maxWidth > 0 ? maxWidth : 360;`.

---

_Reviewed: 2026-07-24T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
