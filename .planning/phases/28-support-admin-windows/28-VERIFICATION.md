---
phase: 28-support-admin-windows
verified: 2026-07-22T13:32:17Z
status: gaps_found
score: 2/4 roadmap Success Criteria verified (SC #1 Заявки and SC #2 Отчёты fail live UAT); automated gates 4/4 green
overrides_applied: 0
source: live human both-theme UAT (cargo tauri dev) at plan 28-10 Task 2 checkpoint
---

# Phase 28: Окна поддержки и администрирования — Verification Report

**Phase Goal:** Четыре окна поддержки/администрирования (Заявки WIN-06, Отчёты WIN-07,
Настройки WIN-08, Пользователи WIN-09) переходят на дизайн-систему Фаз 23–25 и оболочку Фазы 26.
SC #1 Заявки, SC #2 Отчёты, SC #3 Настройки, SC #4 Пользователи — новые токены/компоненты
повсеместно; каждое поле/действие/workflow сохранено (изменение чисто визуальное).

**Verified:** 2026-07-22 (live human UAT, both themes, desktop)
**Status:** gaps_found — automated gates pass, but live UAT surfaced systematic visual/UX gaps
plus one functional regression.

## Automated Gates (all green)

- `node ui/scripts/check-tokens.mjs` → PASS, 0 нарушений
- `pnpm --dir ui svelte-check` → 0 errors (48 pre-existing warnings, unrelated files)
- `pnpm --dir ui lint` → clean
- `pnpm --dir ui build` → success

The gaps below are visual/interaction/functional — none break the build, which is why they only
surface under human UAT (phase has no frontend tests — D-18 of Фаза 26).

## Goal Achievement — Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Окно Заявок использует новые токены/компоненты повсеместно; поля/действия сохранены | ✗ GAPS | Structural migration landed (Tabs/Table/DetailPanel/PageHeader), but master/detail panels overflow off-screen and are not stretched to full height with bottom padding; list table overflows horizontally, hiding the «Автор» column so rows are indistinguishable; category picker still on **native** `Select` instead of custom `Dropdown`. See GAP-1, GAP-2. |
| 2 | Окно Отчётов использует новые токены/компоненты повсеместно | ✗ GAPS | Sub-nav on Tabs ✓, but report table lost rounded/card styling and is not aligned to bottom of screen with padding like Акты; month/year on **native** `Select` with the selected value not displayed; range С/По inputs misaligned; **Возвраты fails to load** («Не удалось загрузить отчёт»). See GAP-1, GAP-3, GAP-4. |
| 3 | Окно Настроек использует новые токены/компоненты повсеместно; граница D-08 TemplateEditor соблюдена | ◐ PARTIAL | Structure/panels accepted by UAT ("всё хорошо"), but Bind-адрес / частота бэкапа / тип шаблона still on **native** `Select` instead of custom `Dropdown`. See GAP-1. |
| 4 | Окно Пользователей использует новые токены/компоненты; поля/действия сохранены; пароль замаскирован | ◐ PARTIAL | Window accepted by UAT ("пойдёт"), password masked ✓, inline delete ✓; only the Роль picker still on **native** `Select` instead of custom `Dropdown`. See GAP-1. |

**Score:** 2/4 roadmap SCs fail live UAT (SC #1, SC #2); SC #3 and SC #4 pass except the shared
dropdown gap (GAP-1).

## Root Causes

1. **Wrong dropdown primitive (design-contract error).** The phase migrated raw `<select>` to the
   **native** `Select.svelte` (a thin wrapper over the browser `<select>` → OS combobox). Phase 27
   (the Acts/Cartridges/Printers reference) established the opposite standard: native `<select>` →
   custom `Dropdown.svelte` "for full visual consistency" (Phase 27 Batch G, commit `80d0b41`;
   see 27-VERIFICATION.md SC #2 evidence). The in-repo positive reference the user cited is
   `RequestFormModal`'s printer picker, which already uses `GroupedPrinterSelect` (a wrapper over
   the custom `Dropdown`).
2. **Unfaithful layout port from Акты.** Заявки and Отчёты were meant to mechanically reuse the
   Фаза 27 Acts playbook (`ActsMasterDetail`/`ActsList`/`ActListRow` + shared `Table`), but the
   port dropped the full-height/overflow handling and (for Reports) the card/rounding table
   styling.

## Gaps

### GAP-1 — Native `Select` → custom `Dropdown` across all four windows (SC #1–#4)
**Severity:** high (recurring user feedback; repeats a Фаза 27 correction)
**What:** Replace every remaining **native** `Select.svelte` usage with the custom `Dropdown.svelte`
(flat "select" variant), matching the `GroupedPrinterSelect`/`Dropdown` pattern already used in
`RequestFormModal` (printer picker) and the Phase 27 Cartridges window (commit `80d0b41`).
**Where (7 sites):**
- `ui/src/features/requests/RequestDetail.svelte:673` — Роль (подтверждение)
- `ui/src/features/requests/RequestFormModal.svelte:238` — Категория
- `ui/src/features/users/UserFormModal.svelte:192` — Роль
- `ui/src/features/settings/BackupSettings.svelte:150` — частота бэкапа
- `ui/src/features/settings/NetworkSettings.svelte:194` — Bind-адрес
- `ui/src/features/settings/TemplateEditor.svelte:220` — тип шаблона (respect D-08: only the control, editor/preview untouched)
- `ui/src/features/reports/PeriodSelector.svelte:114,119,127` — Месяц / Год (also fixes the "selected value not displayed" regression — native value binding is broken; custom Dropdown shows the value explicitly)
**Reference:** `ui/src/lib/components/Dropdown.svelte`, `ui/src/lib/components/GroupedPrinterSelect.svelte`, and Cartridges-window usage from Фаза 27 (commit `80d0b41`).
**Acceptance:** No `import Select from '$lib/components/Select.svelte'` remaining in the four admin
windows (unless a specific field genuinely needs the native control — justify inline); each list is
the custom Dropdown; PeriodSelector Месяц/Год display the current value; same options, same selected
id/value, same onchange side-effects (component swap, not workflow change).

### GAP-2 — Заявки master/detail layout must match Акты (SC #1, D-02)
**Severity:** high
**What:** `RequestsMasterDetail` panels overflow off-screen and are not stretched to full height
with bottom padding; the list table overflows horizontally, clipping the «Автор» / «Статус»
columns, so rows read as "куча одинаковых строчек" and it is unclear who created each request.
**Where:** `ui/src/features/requests/RequestsMasterDetail.svelte`, `RequestsList.svelte`,
`RequestListRow.svelte`.
**Reference:** `ui/src/features/acts/ActsMasterDetail.svelte`, `ActsList.svelte`, `ActListRow.svelte`
(Фаза 27) — copy the height/overflow/flex-fill behavior and column layout faithfully.
**Acceptance:** master and detail panels fill available height with bottom padding (no off-screen
overflow); the list has no horizontal overflow/clipping; each row clearly shows who created the
request (author visible), visually consistent with the Акты list.

### GAP-3 — Отчёты tables must match shared Table + Акты height (SC #2)
**Severity:** medium-high
**What:** `ReportTable` lost rounded/card styling; across all report sections the table must adopt
the shared `Table` look and stretch to the bottom of the screen with padding, as in Акты; range
С/По date inputs are misaligned and unstyled.
**Where:** `ui/src/features/reports/ReportTable.svelte`, `ReportsPage.svelte`, `ReportFilters.svelte`.
**Reference:** shared `ui/src/lib/components/Table.svelte`/`TableRow.svelte` as used in Акты; Акты
page full-height layout.
**Acceptance:** report tables use consistent shared-Table styling (rounding/borders) across all
report types; the table area fills height to the bottom with padding; С/По date inputs are aligned
and styled.

### GAP-4 — Возвраты report fails to load (functional regression — investigate)
**Severity:** high (functional, not visual)
**What:** Отчёты → Устройства → Возвраты shows «Не удалось загрузить отчёт» (toast + inline error).
User is unsure whether it predates the phase. Determine whether phase-28 `ReportTable`/`ReportsPage`
changes broke the returns data path (e.g., a changed column/shape assumption) or it is a pre-existing
backend/data issue.
**Where:** `ui/src/features/reports/ReportsPage.svelte`, `ReportTable.svelte`, reports API/backend
returns path.
**Route:** if the cause is data/backend, hand to `/gsd-debug`; if it is a frontend shape/column
assumption introduced in 28-04, fix in the gap plan.
**Acceptance:** Возвраты loads without error (both device and cartridge domains where applicable),
matching prior behavior.

## Notes

- Not a regression of the OTHER windows: Устройства/Акты/Картриджи/Принтеры/витрина were not
  reported as visually affected (Tabs/Table/TableRow/Dropdown primitives themselves were not
  modified in this phase).
- The password-masking (28-09, T-28-09-01) and inline-delete (UI-SPEC §7.4) requirements passed UAT.
- Automated gates remain green — gap plans should preserve `check-tokens`/`svelte-check`/`lint`/`build`
  exit 0.
