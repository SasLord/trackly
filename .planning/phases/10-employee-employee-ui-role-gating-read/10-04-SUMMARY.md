---
phase: 10-employee-employee-ui-role-gating-read
plan: 04
subsystem: frontend
tags: [employee-ui, rbac, role-gating, svelte5, dashboard, ux]

# Dependency graph
requires:
  - phase: 10-employee-employee-ui-role-gating-read
    provides: "Plan 10-01 (ReadData Admin|Manager only), Plan 10-02 (read-path gating across 5 domains), Plan 10-03 (request ownership scope + BOLA close + employee-scoped dashboard_get_all_widgets branch, AppError code FORBIDDEN)"
provides:
  - "EmployeeLayout.svelte — standalone header-based shell for role=employee (D-UI-01), not a branch of Layout.svelte/Sidebar.svelte"
  - "AccessDenied.svelte — forbidden-route screen modeled on NotFound.svelte (D-DENY-01)"
  - "employeeRoutes route map in routes.ts — additive, existing routes export unchanged"
  - "App.svelte role branch selecting EmployeeLayout+employeeRoutes vs Layout+routes"
  - "«Мои заявки» StatWidget card in RequestsPage.svelte wired to the employee-scoped dashboard_get_all_widgets branch (D-GATE-03 frontend surface)"
  - "Symmetric 403 toast handling in client.ts (Tauri + HTTP transports)"
affects: [employee-ui, ui-role-gating]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Separate dedicated shell component per role (not a conditional branch inside the existing sidebar shell) when the role's permitted surface is small enough to warrant its own layout"
    - "Role-based route map switch in the root component ($effect-free if/else-if chain) rather than per-route wrap() guards — simpler, matches existing App.svelte if/else-if shape"
    - "Client-side route/shell selection documented explicitly as UX-only; backend 403 remains the sole security boundary"
    - "Symmetric error-code branch added next to an existing one (403 next to 401) in both transport paths of the same apiCall function"

key-files:
  created:
    - ui/src/features/layout/EmployeeLayout.svelte
    - ui/src/pages/AccessDenied.svelte
  modified:
    - ui/src/routes.ts
    - ui/src/App.svelte
    - ui/src/features/requests/RequestsPage.svelte
    - ui/src/lib/api/client.ts

key-decisions:
  - "employeeRoutes implemented as a plain route-map switch in App.svelte's if/else-if chain (role === 'employee' branch), not svelte-spa-router wrap() guards — simpler, and the existing App.svelte already gates shell selection on authStore.user the same way, so this reuses the established pattern instead of introducing a second gating mechanism (resolves RESEARCH Open Question 3 in favor of the simpler option)"
  - "AccessDenied.svelte destructures an empty Props object ({}) instead of binding the unused `location` prop — svelte-check flags unused destructured bindings as an error; the prop type is still declared for router-signature compatibility, just not bound to a name"
  - "Dashboard card fetch uses a bare $effect gated on isEmployee (not extending the existing onMount) — keeps the employee-only fetch logically separate from the role-agnostic refresh()/WS-connect onMount, avoids coupling an Admin/Manager-irrelevant fetch into shared mount logic"

requirements-completed: [D-UI-01, D-DENY-01, D-GATE-03]

# Metrics
duration: ~35min (autonomous tasks 1-3; checkpoint task 4 pending human verification)
completed: 2026-06-21
---

# Phase 10 Plan 04: Employee UI shell + access-denied + dashboard card + client.ts 403 Summary

**Built the three frontend-only Phase 10 decisions on top of the now-fully-enforced backend: a genuinely separate `EmployeeLayout.svelte` header shell, an `AccessDenied.svelte` screen for forbidden-route navigation, a «Мои заявки» dashboard summary card wired to the employee-scoped `dashboard_get_all_widgets` branch, and symmetric 403 toast handling in `client.ts` for both transports — all client-side gating is explicitly UX-only, the real boundary stays server-side.**

## Performance

- **Duration:** ~35 min for Tasks 1-3 (autonomous)
- **Completed:** 2026-06-21
- **Tasks:** 3/3 autonomous tasks complete; Task 4 (checkpoint:human-verify) reached and awaiting approval
- **Files modified:** 6 files (2 created, 4 modified)

## Accomplishments

- **D-UI-01:** `EmployeeLayout.svelte` is a standalone component (no import of `Layout.svelte`/`Sidebar.svelte`) — skip-link, header (brand "Trackly" left; user name, "Сотрудник" label, `ThemeSwitcher`, ghost logout button right), and a `<main id="main">` landmark. `App.svelte`'s if/else-if chain gained a new `{:else if authStore.user.role === 'employee'}` branch rendering `EmployeeLayout` + a new `employeeRoutes` Router; the existing `{:else}` branch (Layout + routes) is unchanged for admin/manager.
- **D-DENY-01:** `AccessDenied.svelte` modeled byte-for-byte on `NotFound.svelte`'s structure/SCSS shape, with the exact locked copy ("Нет доступа" / "У вашей роли («Сотрудник») нет доступа к этому разделу. Доступны только заявки." / "К заявкам" → `#/requests`). `routes.ts` gained an additive `employeeRoutes` export (`'/'` and `'/requests'` → `RequestsPage`, `'/access-denied'` and `'*'` → `AccessDenied`) — the existing `routes` export and all its entries are untouched.
- **D-GATE-03 (frontend surface):** `RequestsPage.svelte` now fetches `dashboard_get_all_widgets({ period: null })` in a `$effect` gated strictly on `isEmployee`, rendering a `StatWidget` titled "Мои заявки" (mainLabel "активных заявок", breakdown rows "Новые"/"В работе"/"Выполнено", no `warningItems`) above the existing search/tabs + master-detail block. A failed fetch sets a Russian error string and renders the `StatWidget` error state without blocking the rest of the page. Admin/Manager view of this page is byte-identical to before (the new block is inside `{#if isEmployee}`).
- **client.ts 403 handling:** added `import { pushToast } from '$lib/stores/toast.svelte'` and a sibling branch to the existing 401 checks on both transports — Tauri (`code === 'FORBIDDEN' || 'Forbidden'`) and HTTP (`res.status === 403`) — each calling `pushToast('error', 'Недостаточно прав для этого действия')` before the existing `throw err` (neither branch touches `authStore` or the location hash).
- `pnpm --dir ui exec svelte-check` ran after every task: 0 errors throughout (one transient error in `AccessDenied.svelte` from an unused destructured prop was fixed immediately in Task 1, before commit). Final state: 0 errors, 36 pre-existing warnings in unrelated files, unchanged from the pre-plan baseline.
- `pnpm --dir ui build` ran successfully after Task 3, producing a fresh `ui/dist` (required since server-mode/LAN-browser verification at the Task 4 checkpoint serves the build output, not HMR).

## Task Commits

1. **Task 1: EmployeeLayout.svelte + AccessDenied.svelte + App.svelte role branch + routes.ts employee route map** — `0667f1c` (feat) — `ui/src/features/layout/EmployeeLayout.svelte`, `ui/src/pages/AccessDenied.svelte`, `ui/src/App.svelte`, `ui/src/routes.ts`
2. **Task 2: «Мои заявки» StatWidget card wired to employee-scoped dashboard_get_all_widgets** — `2e5d286` (feat) — `ui/src/features/requests/RequestsPage.svelte`
3. **Task 3: client.ts symmetric 403 handling (Tauri + HTTP)** — `f482291` (feat) — `ui/src/lib/api/client.ts`

## Verification

- `pnpm --dir ui exec svelte-check` after each of Tasks 1, 2, 3 → 0 errors (same 36 pre-existing warnings in unrelated files each time; `FILES_WITH_PROBLEMS` count dropped from 12 to 11 after Task 1's fix, stayed at 11 through Tasks 2-3, confirming no regressions introduced).
- `pnpm --dir ui build` after Task 3 → succeeded, `ui/dist` rebuilt (385.54 kB main JS chunk, 138.80 kB CSS, gzip totals unchanged in character from before this plan — no new heavy dependencies).
- Task 4 (checkpoint:human-verify) — **NOT YET RUN.** The 7-step manual verification (employee shell visibility, dashboard card data, route-gating to AccessDenied on 8 forbidden hashes, 403 toast behavior, admin/manager regression) requires a human to log in as an employee-role user and a browser session; this is the only verification path per 10-UI-SPEC (no frontend test runner exists in this repo, by design).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] AccessDenied.svelte unused `location` destructure caused a svelte-check error**
- **Found during:** Task 1, immediately after writing `AccessDenied.svelte` and running `svelte-check`
- **Issue:** Destructuring `const { location }: Props = $props();` exactly as `NotFound.svelte` does triggers `'location' is declared but its value is never read.'` because, unlike `NotFound.svelte`, `AccessDenied.svelte`'s copy doesn't reference the current hash anywhere in the template (the message is role-generic, not route-specific).
- **Fix:** Changed to `const {}: Props = $props();` — the `Props` interface (and its `location?: { hash: string }` field) is kept for router-signature compatibility, but the binding is not named since it's unused.
- **Files modified:** `ui/src/pages/AccessDenied.svelte`
- **Commit:** `0667f1c` (fixed before commit, not a separate commit)

No other deviations. Plan executed as written for Tasks 1-3.

## Issues Encountered

None.

## User Setup Required

None for Tasks 1-3 (autonomous, complete). Task 4 requires the user (or an orchestrator-driven browser session) to perform the 7-step manual verification described in the plan's checkpoint — see the CHECKPOINT REACHED message below for exact steps.

## Next Phase Readiness

- All three frontend-only Phase 10 decisions (D-UI-01, D-DENY-01, D-GATE-03's frontend half) are implemented and type-check clean. `ui/dist` is freshly built.
- Phase 10 cannot be marked complete until Task 4's checkpoint is approved by a human — this SUMMARY documents Tasks 1-3 only; the checkpoint result (approved / failed-step) will need to be recorded by whichever agent resumes after the human responds.
- No backend changes in this plan — Phase 10's backend authorization boundary (10-01/10-02/10-03) is untouched and remains the actual security control; this plan is UX-only per its own threat model (T-10-04-01, accepted).

---
*Phase: 10-employee-employee-ui-role-gating-read*
*Completed: 2026-06-21 (Tasks 1-3; Task 4 checkpoint pending)*

## Self-Check: PASSED

All 6 claimed files verified present/modified in commits 0667f1c / 2e5d286 / f482291; svelte-check clean (0 errors) and `pnpm --dir ui build` succeeded after the final commit.
