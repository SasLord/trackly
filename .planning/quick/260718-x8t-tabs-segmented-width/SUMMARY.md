---
quick_id: 260718-x8t
slug: tabs-segmented-width
subsystem: ui
tags: [svelte, scss, flexbox, showcase, tabs]

requires: []
provides:
  - Segmented-вариант вкладок в витрине компонентов (#/showcase) обжимает подложку по содержимому, как в эталоне Tabs.dc.html:64
affects: [24-base-components]

tech-stack:
  added: []
  patterns:
    - "Flex column wrappers around inline-flex children must set align-items: flex-start explicitly — default align-items: stretch silently stretches inline-flex descendants along the cross axis"

key-files:
  created: []
  modified:
    - ui/src/features/showcase/sections/TabsSection.svelte

key-decisions:
  - "align-self: stretch fallback on .tabs-underline NOT applied — headless-Chrome screenshot diff (before/after) showed underline variant is pixel-identical, so Tabs.svelte was left untouched per the plan's stated preference"

requirements-completed: []

duration: ~20min
completed: 2026-07-18
---

# Quick Task 260718-x8t: Segmented-вариант вкладок обжимает подложку по содержимому Summary

**Одна строка CSS (`align-items: flex-start` на `.variant-block` в TabsSection.svelte) убрала неявное `align-items: stretch`, которое растягивало inline-flex подложку `.tabs-segmented` на всю ширину секции витрины.**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-07-18
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Segmented-подложка вкладок в витрине (`#/showcase` → «Вкладки» → «Сегментированный») теперь обжимается по содержимому вместо растягивания на всю ширину секции, соответствуя эталону `Tabs.dc.html:64`.
- Подтверждено визуально (headless Chrome screenshot diff, до/после фикса): underline-вариант («Switch-bar») не изменился — регресса не произошло, `align-self: stretch` fallback не понадобился, `Tabs.svelte` не тронут.
- `Tabs.svelte` компонент оставлен без изменений — фикс полностью локализован в обёртке витрины.

## Task Commits

1. **Task 1: CSS-фикс — обжать segmented-подложку по содержимому в витрине** - `402f15d` (fix)

**Plan metadata:** committed together with this SUMMARY (see final commit below).

## Files Created/Modified

- `ui/src/features/showcase/sections/TabsSection.svelte` — добавлено `align-items: flex-start;` в правило `.variant-block`.

## Decisions Made

- Fallback (`align-self: stretch` на `.tabs-underline` в `Tabs.svelte`) не применён — проверка headless-Chrome screenshot diff (сборка временного preview-харнесса, mount `TabsSection.svelte` напрямую, скриншот до и после фикса) показала, что underline-вариант визуально идентичен в обоих состояниях. Его полноширинный вид обеспечивается `border-bottom` внутри `.tab`, а не растяжением родителем — как и предполагал план.

## Deviations from Plan

None - plan executed exactly as written. Fallback branch (align-self: stretch) was evaluated per plan's verification requirement and found unnecessary — this is the plan's own designed decision point, not a deviation.

## Issues Encountered

- Визуальная проверка через реальный dev-сервер приложения оказалась невозможна напрямую (`#/showcase` защищён auth-гейтом `App.svelte` → без backend-сессии показывается `LoginPage`). Решение: временный изолированный preview-харнесс (`ui/dev-preview.html` + `ui/src/dev-preview-main.ts`), который mount'ит `TabsSection.svelte` напрямую с реальными глобальными стилями (`global.scss`, `initTheme()`). Скриншоты сделаны через headless Chrome (`google-chrome --headless --screenshot`) до и после фикса для объективного pixel-diff вместо словесного описания. Временные файлы удалены перед коммитом (не входят в диф, не закоммичены).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Одна из 2 gap-задач UAT фазы 24 (`24-UAT.md`, тест 6) закрыта. Вторая — «D-02 route gating» — остаётся отдельной quick-задачей/фиксом.
- `ui/dist` пересобран (`pnpm --dir ui build`), сервер-режим/LAN-браузер отдаёт актуальный бандл.

---
*Quick task: 260718-x8t-tabs-segmented-width*
*Completed: 2026-07-18*

## Self-Check: PASSED
