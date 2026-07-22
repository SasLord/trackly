---
phase: 28-support-admin-windows
plan: 13
subsystem: ui
tags: [svelte, dropdown, gap-closure, users, reports]

# Dependency graph
requires:
  - phase: 25-dropdown
    provides: Dropdown.svelte (flat + variant="select" combobox primitive)
  - phase: 28-support-admin-windows
    provides: 28-VERIFICATION.md GAP-1/GAP-3 findings from live UAT
provides:
  - UserFormModal.svelte Роль picker on custom Dropdown (last native Select site in Users admin)
  - PeriodSelector.svelte Месяц/Год pickers on custom Dropdown, value always displayed (fixes UAT regression)
  - PeriodSelector.svelte С/По range-group spacing fix (GAP-3 partial)
affects: [28-support-admin-windows, 28-VERIFICATION]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Dropdown flat+variant=select implicit-label wrapping for fields without native for/id association (CartridgeFormBody precedent, reused twice more)"

key-files:
  created: []
  modified:
    - ui/src/features/users/UserFormModal.svelte
    - ui/src/features/reports/PeriodSelector.svelte

key-decisions:
  - "Reused existing {value,label}/{id,label} option arrays directly as Dropdown groups instead of introducing new shapes"
  - "Re-targeted .period-controls CSS overrides from Select's .select-wrapper/.select classes to Dropdown's .tr-dropdown/.tr-dropdown-field-button classes to preserve the compact 28px filter-row height"

patterns-established: []

requirements-completed: [WIN-09, WIN-07]

# Metrics
duration: 6min
completed: 2026-07-22
---

# Phase 28 Plan 13: UserFormModal Роль + PeriodSelector Месяц/Год → Dropdown Summary

**Заменены последние 4 сайта native `<Select>` в Пользователи/Отчёты на кастомный Dropdown (flat + variant="select"); заодно исправлен баг "выбранное значение не отображается" в PeriodSelector и добавлен spacing-фикс для диапазона дат С/По.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-07-22T14:35:54Z
- **Completed:** 2026-07-22T14:43:28Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- UserFormModal.svelte: поле «Роль» переведено с `Select.svelte` на `Dropdown.svelte` (flat + `variant="select"`), с сохранением записи в `form.role`, invalid/disabled-состояний и всех 3 опций (Администратор/Специалист/Сотрудник)
- PeriodSelector.svelte: все 3 инстанса `<Select>` (месяц в режиме «Месяц», год в режиме «Месяц», год в режиме «Год») переведены на `Dropdown` — устраняет регрессию из живого UAT, когда выбранное значение не отображалось (Select's internal `bind:value` на невзаимно-связанный `$bindable` prop desync'ится от родительских реактивных апдейтов; Dropdown's явная односторонняя controlled-value конвенция не имеет этой проблемы)
- CSS-переопределения компактной высоты фильтр-строки (28px) перенацелены с устаревших классов `Select` (`.select-wrapper`/`.select`) на классы `Dropdown` (`.tr-dropdown`/`.tr-dropdown-field-button`) — без этого переноса переопределения молча не сработали бы
- GAP-3 partial: добавлен `gap: var(--tr-space-md)` в `.period-range` для visual breathing room между группами С/По (ранее наследовали тесный `var(--tr-space-2xs)` от `.period-controls`)

## Task Commits

Each task was committed atomically:

1. **Task 1: UserFormModal.svelte — Роль native Select -> Dropdown** - `61f4c05` (feat)
2. **Task 2: PeriodSelector.svelte — Месяц/Год native Select -> Dropdown + С/По range spacing** - `1b1e7fc` (feat)

**Plan metadata:** (pending — orchestrator commits final metadata separately)

_Note: no TDD tasks in this plan._

## Files Created/Modified
- `ui/src/features/users/UserFormModal.svelte` - Роль picker: `Select` → `Dropdown` (flat + variant="select"), `.dropdown-label` implicit-label wrapper CSS added
- `ui/src/features/reports/PeriodSelector.svelte` - 3× `Select` → `Dropdown` for Месяц/Год pickers, CSS re-targeted from `.select-wrapper`/`.select` to `.tr-dropdown`/`.tr-dropdown-field-button`, `.period-range` gap added

## Decisions Made
- Reused existing option-array shapes (`{value,label}` for roles, new `{id,label}` mapped from `MONTHS`/`years` arrays) directly as Dropdown `groups` — no new data shapes introduced, matching the CartridgeFormBody.svelte precedent already established in Plan 27-G1.
- Typed no-op `noExpandRole`/`noExpandMonth`/`noExpandYear` functions (each returning `[]`) satisfy Dropdown's `onExpandGroup` prop contract without ever being invoked (all three fields use `flat={true}`, `isGroupExpandable={() => false}`).

## Deviations from Plan

None - plan executed exactly as written. One post-implementation formatting fix (Prettier auto-wrapped a long derived-expression line in UserFormModal.svelte to satisfy `pnpm lint`'s `prettier --check` gate) — not a deviation from plan content, purely a line-wrap.

## Issues Encountered
- `pnpm --dir ui build` and whole-project `svelte-check` fail due to a **pre-existing, unrelated** backend compile error (`crates/trackly-app/src/http/mod.rs:185/190` — `SpaAssets::get` not found), which blocks generation of `ui/src/bindings.ts`. This is out of scope for this UI-only Select→Dropdown migration (per executor task instructions) and was not touched. Verification for this plan relied on: (1) file-level `grep` acceptance criteria (all passed), (2) `pnpm svelte-check` output scoped to the two modified files (no NEW errors introduced — only the pre-existing, project-wide "Cannot find module '../../bindings'" error and pre-existing unrelated warnings), (3) `pnpm lint` (ESLint + Prettier + check-tokens.mjs) passing clean.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- GAP-1 (28-VERIFICATION.md) closed for both remaining native-Select sites in Пользователи and Отчёты — no native `<select>` combobox usages should remain in these two admin windows.
- GAP-3 partial (С/По range spacing) closed in the same file where the markup lives.
- Full 28-VERIFICATION.md re-check of all gap-closure plans (28-11..28-16) still pending — this is one of six sibling plans addressing separate UAT gaps found in that verification pass.

---
*Phase: 28-support-admin-windows*
*Completed: 2026-07-22*
