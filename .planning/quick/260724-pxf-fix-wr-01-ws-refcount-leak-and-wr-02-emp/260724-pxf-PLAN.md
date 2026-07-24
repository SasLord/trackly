---
quick_id: 260724-pxf
slug: fix-wr-01-ws-refcount-leak-and-wr-02-emp
date: 2026-07-24
mode: quick
---

# Quick Task 260724-pxf: Fix WR-01 & WR-02 (phase-29 code review)

Fix two confirmed pre-existing logic bugs surfaced by the phase-29 code review
(`.planning/phases/29-login-and-employee-shell/29-REVIEW.md`). Both predate
phase 29 (a CSS/markup-only migration).

## Task 1 — WR-01: WS refcount leak on fast unmount

**File:** `ui/src/features/layout/EmployeeLayout.svelte` (onMount, ~63-80)

`connectWs()` bumps the shared `refCount` synchronously but resolves its teardown
(`unlisten`) asynchronously via `.then(...)`. If the component unmounts before the
promise resolves, the returned cleanup runs while `unlisten` is still `undefined`,
so the later-arriving release is never called and `refCount` never decrements —
the WS singleton/reconnect machinery leaks across fast mount/unmount cycles.

**Fix:** Add a `disposed` flag. If the promise resolves after unmount, call the
release fn immediately instead of storing it. The release fn from
`ui/src/lib/api/ws.ts` is idempotent (`released`/`refCount` guards), so this is safe.

- **action:** Add `let disposed = false;`; in `.then`, `if (disposed) fn(); else unlisten = fn;`; set `disposed = true` in the cleanup closure.
- **verify:** `pnpm --dir ui svelte-check` 0 errors; teardown always releases the refcount.
- **done:** Unmount before connect resolves still decrements `refCount`.

## Task 2 — WR-02: empty-string rejection_reason misclassified

**File:** `ui/src/features/auth/BlockedScreen.svelte` (~line 78)

State selection uses a truthiness test on `rejection_reason`. LoginPage
normalization (`ui/src/features/auth/LoginPage.svelte:79-80`) preserves `""`, and
the backend (`crates/trackly-app/src/services/auth.rs`) derives `rejection_reason`
from free-form `resolution_notes`, which can be empty. A rejected request with an
empty reason then renders the first-time "Доступ закрыт" screen instead of
"Запрос отклонён".

**Fix:** Distinguish `null` from `""` — `{:else if blockedDetails.rejection_reason !== null}`.

- **action:** Change the `{:else if}` condition to `blockedDetails.rejection_reason !== null`.
- **verify:** `pnpm --dir ui svelte-check` 0 errors.
- **done:** Rejected-with-empty-reason renders "Запрос отклонён".

## Verification
- `pnpm --dir ui lint`
- `pnpm --dir ui svelte-check` (expect 0 errors)
- `pnpm --dir ui build`
