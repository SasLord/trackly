---
phase: 28-support-admin-windows
plan: 11
subsystem: ui
tags: [svelte, dropdown, select, gap-closure, requests]

# Dependency graph
requires:
  - phase: 25-dropdown
    provides: "Dropdown.svelte generic primitive (flat + variant=\"select\")"
  - phase: 27-windows-workflow-cartridges-printers
    provides: "CartridgeFormBody.svelte canonical flat+select Dropdown wiring pattern (Phase 27-G1)"
provides:
  - "RequestDetail.svelte Роль (confirm-registration) picker on custom Dropdown"
  - "RequestFormModal.svelte Категория picker on custom Dropdown"
  - "Zero native Select imports remaining in ui/src/features/requests/"
affects: [28-VERIFICATION, 29-login-employee-ui, 30-quality-parity]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Dropdown flat+variant=\"select\" component swap for native Select — third
       and fourth application of the CartridgeFormBody (Phase 27-G1) pattern"
    - "String sentinel ('none') to round-trip a nullable id through Dropdown's
       string|number getGroupId contract"

key-files:
  created: []
  modified:
    - ui/src/features/requests/RequestDetail.svelte
    - ui/src/features/requests/RequestFormModal.svelte

key-decisions:
  - "Роль options kept as module-adjacent const array (ROLE_OPTIONS), identical
     labels/order to the removed <option> list — no behavior change (SC #1)"
  - "Категория 'Без категории' encoded as NONE_CATEGORY_ID = 'none' string
     sentinel, decoded back to categoryId = null in onPickGroup — same
     semantics as the native Select's empty-string option value"
  - "GroupedPrinterSelect (Принтер field) left untouched — out of GAP-1's
     7-site scope, still wraps a native <select> internally by design"

patterns-established: []

requirements-completed: [WIN-06]

# Metrics
duration: ~20min
completed: 2026-07-22
---

# Phase 28 Plan 11: Заявки Роль/Категория Select → Dropdown Summary

**Closed GAP-1 (28-VERIFICATION.md) for the last two native-`Select` sites in
the Заявки window — `RequestDetail.svelte`'s confirm-registration Роль picker
and `RequestFormModal.svelte`'s free-form Категория picker — both now render
via the custom `Dropdown.svelte` primitive (flat + `variant="select"`), with
identical options/values/side-effects.**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-07-22
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `RequestDetail.svelte`: Роль confirm-registration picker replaced —
  `ROLE_OPTIONS` const (employee/Сотрудник, manager/Специалист,
  admin/Администратор), `approveRoleLabel` derived, `noExpandRole` typed
  no-op, `.dropdown-label` wrapping-label CSS — identical 3 options and
  identical `approveRole` write via `onPickGroup`.
- `RequestFormModal.svelte`: Категория picker replaced — `NONE_CATEGORY_ID`
  string sentinel for "Без категории", `categoryOptions`/
  `selectedCategoryKey`/`selectedCategoryLabel` derived, `noExpandCategory`
  typed no-op — identical server-driven category list and identical
  `categoryId` write semantics (`null` for "Без категории").
- Zero `import Select from '$lib/components/Select.svelte'` remains in either
  file; `GroupedPrinterSelect` (Принтер field, out of GAP-1 scope) untouched.
- `pnpm --dir ui lint` (eslint + prettier + check-tokens.mjs) passes clean on
  both files after a Prettier reformat pass.
- `pnpm --dir ui svelte-check` reports zero errors/warnings for either
  modified file (whole-project run is blocked by a pre-existing, unrelated
  backend compile error — see Issues Encountered).

## Task Commits

Each task was committed atomically:

1. **Task 1: RequestDetail.svelte — Роль (подтверждение) native Select -> Dropdown** - `b9cd443` (feat)
2. **Task 2: RequestFormModal.svelte — Категория native Select -> Dropdown** - `7eaa845` (feat)
3. **Prettier formatting fix (Rule 3 — lint gate)** - `2049ff3` (style)
4. **Deferred-item documentation (out-of-scope discovery)** - `0c1dc6f` (docs)

_No refactor commit needed — plan is not TDD-gated (`type="auto"` tasks)._

## Files Created/Modified

- `ui/src/features/requests/RequestDetail.svelte` — Роль picker (confirm-registration modal) migrated to Dropdown
- `ui/src/features/requests/RequestFormModal.svelte` — Категория picker (free-form request) migrated to Dropdown
- `.planning/phases/28-support-admin-windows/deferred-items.md` — new: logs the pre-existing SpaAssets compile error found during verification (out of scope)

## Decisions Made

- `ROLE_OPTIONS`/`categoryOptions` are plain arrays/derived, not extracted to
  a shared module — each site's option set is small and screen-specific,
  matching the CartridgeFormBody precedent (KIND_OPTIONS/modelOptions are
  also per-component).
- `NONE_CATEGORY_ID = 'none'` chosen as the sentinel string (never collides
  with a real numeric category id encoded via `String(c.id)`).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Prettier line-wrap formatting on both modified files**
- **Found during:** Post-task verification (`pnpm --dir ui lint`)
- **Issue:** `prettier --check` (part of the `lint` script, a plan-level
  verification gate) flagged both files for line-wrap style (single-line
  `$derived` calls that Prettier's printer wraps differently at 100 cols).
- **Fix:** Ran `npx prettier --write` on both files; no logic change.
- **Files modified:** `ui/src/features/requests/RequestDetail.svelte`,
  `ui/src/features/requests/RequestFormModal.svelte`
- **Verification:** `pnpm --dir ui lint` now exits 0; re-ran all
  acceptance-criteria greps after reformatting — all still pass; `svelte-check`
  still reports zero issues for either file.
- **Committed in:** `2049ff3`

---

**Total deviations:** 1 auto-fixed (1 blocking/lint)
**Impact on plan:** Cosmetic only. No scope creep — both files' logic is
identical to what Tasks 1/2 implemented; only whitespace/line-wrapping
changed.

## Issues Encountered

- Attempted to regenerate `ui/src/bindings.ts` (gitignored, generated by
  `cargo test -p trackly-app --test export_bindings`) in order to run a
  clean whole-project `pnpm --dir ui svelte-check` / `pnpm --dir ui build`
  per the plan's `<verification>` block. The backend crate `trackly-app`
  fails to compile: `crates/trackly-app/src/http/mod.rs:185,190` —
  `SpaAssets::get(...)` — `error[E0599]: no function or associated item
  named 'get' found for struct 'SpaAssets' in the current scope`. This is
  **pre-existing and unrelated** to this plan's two Svelte files (confirmed:
  neither `RequestDetail.svelte` nor `RequestFormModal.svelte` produced any
  svelte-check error even before this attempt — the "Cannot find module
  '../../bindings'" errors that appear project-wide when `bindings.ts` is
  absent do not touch either of these two files, which only import from the
  already-present `bindings-phase6`/`bindings-phase9`). Logged to
  `.planning/phases/28-support-admin-windows/deferred-items.md` per the
  scope-boundary rule rather than fixed — out of scope for GAP-1's Заявки
  Select→Dropdown swap. Task-level acceptance criteria (all grep checks +
  targeted `svelte-check` on the two files) all pass.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- GAP-1 is now closed for all sites listed in 28-VERIFICATION.md that fell
  within the Заявки window's remit — `RequestDetail.svelte` and
  `RequestFormModal.svelte` no longer import `Select.svelte`.
- Deferred: `crates/trackly-app/src/http/mod.rs`'s `SpaAssets::get()`
  trait-scope compile error blocks `ui/src/bindings.ts` regeneration and,
  transitively, a clean whole-project `svelte-check`/`build`/`export_bindings`
  test run. Recommend a follow-up fix (likely a one-line missing `use`
  import) before the next plan that needs a full backend build.

---
*Phase: 28-support-admin-windows*
*Completed: 2026-07-22*
