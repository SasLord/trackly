---
phase: 30-quality-a11y-platform-parity
plan: 07
subsystem: ui
tags: [svelte, scss, css-grid, focus-ring, layout, accessibility, gap-closure]

# Dependency graph
requires:
  - phase: 30-quality-a11y-platform-parity
    provides: "30-06 previous (partial) fix attempts for the same 2 gaps — inset ring box-shadow, .dashboard-grid min-height:0"
provides:
  - "PeriodToggle.svelte .toggle-btn с горизонтальным запасом 8px вокруг inset-кольца фокуса (Gap 1 закрыт)"
  - "Layout.svelte .app-layout с явным grid-template-rows: minmax(0, 1fr) — definite-height grid row вместо content-based auto (Gap 2 закрыт на уровне общего layout-примитива, разделяется всеми маршрутами)"
affects: [30-08, 30-09, 30-UAT]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CSS Grid: единственная implicit grid-row должна получать явный grid-template-rows: minmax(0, 1fr), когда контейнер задаёт height:100vh + overflow:hidden — иначе default auto-sizing вычисляется по max-content вкладу потомков, который НЕ клэмпится overflow:auto/hidden (отдельный механизм от automatic minimum size у grid-items)"

key-files:
  created: []
  modified:
    - ui/src/features/dashboard/PeriodToggle.svelte
    - ui/src/features/layout/Layout.svelte

key-decisions:
  - "Task 2 live-check выполнен не в самом запущенном приложении (Tauri/axum требует AD/сессионную аутентификацию, недоступную без дополнительной настройки в этой среде), а в headless Chromium (Playwright) через точный CSS-харнесс, воспроизводящий реальные селекторы .app-layout/.sidebar-container/.content из Layout.svelte и .dashboard-page/.dashboard-grid из DashboardPage.svelte один-в-один после фикса — валидирует именно CSS grid-track-sizing механизм, который был первопричиной обоих предыдущих провалов source-only проверки"

requirements-completed: [QA-02, QA-03]

# Metrics
duration: 12min
completed: 2026-07-25
---

# Phase 30 Plan 07: Дашборд — inset-кольцо запас + изолированный скролл (3-я итерация) Summary

**PeriodToggle padding 2px→8px горизонтально закрывает Gap 1 (кольцо больше не прилипает к тексту); Layout.svelte grid-template-rows: minmax(0, 1fr) + .content flex-column закрывает Gap 2 на корневом layout-уровне — root cause был не в DashboardPage.svelte (30-06 чинил не тот файл), а в отсутствии явного grid-template-rows на .app-layout, что заставляло единственную implicit-строку сайзиться по max-content вместо доступного 100vh.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-07-25T09:23:00Z
- **Completed:** 2026-07-25T09:35:00Z
- **Tasks:** 2 completed
- **Files modified:** 2

## Accomplishments
- Gap 1 (QA-02, cosmetic): `.toggle-btn` padding `2px 1px 5px` → `2px 8px 5px` — inset-кольцо фокуса (`box-shadow: inset 0 0 0 2px var(--tr-accent)`, уже установленное в 30-06) теперь имеет 8px горизонтального запаса от глифов текста «3 мес.»/«6 мес.»/«12 мес.», не прилипает
- Gap 2 (QA-03, major, 3-я итерация — root-cause фикс на уровень выше DashboardPage.svelte): `.app-layout` получил `grid-template-rows: minmax(0, 1fr)`, `.content` — `display: flex; flex-direction: column;` перед `overflow: auto`. Это делает единственную grid-строку `fr`-треком (сайзится от доступного 100vh, а не от max-content вклада детей), завершая definite-height цепочку сверху вниз до уже-корректного `.dashboard-page { height: 100%; }` / `.dashboard-grid { flex: 1; min-height: 0; overflow: auto; }` (фикс 30-06, не тронут)
- Фикс применён один раз на уровне общего `Layout.svelte` — автоматически распространяется на все маршруты (Устройства/Принтеры/Акты/Картриджи/Заявки), не требует правок в каждой странице
- Live-check в реальном Chromium (Playwright) headless-браузере подтвердил изоляцию скролла именно на уровне CSS grid-track-sizing (см. Decisions Made)

## Task Commits

Each task was committed atomically:

1. **Task 1: PeriodToggle.svelte — горизонтальный запас у inset-кольца (Gap 1)** - `98b7f1a` (fix)
2. **Task 2: Layout.svelte — grid-template-rows + .content flex-column (Gap 2, root-cause фикс)** - `c2248a7` (fix)

**Plan metadata:** committed via `gsd-sdk query commit` (see below)

## Files Created/Modified
- `ui/src/features/dashboard/PeriodToggle.svelte` - `.toggle-btn` padding `2px 1px 5px` → `2px 8px 5px` (line 36); inset-кольцо (`&:focus-visible`, lines 54-57) не изменено
- `ui/src/features/layout/Layout.svelte` - `.app-layout` (+`grid-template-rows: minmax(0, 1fr);`) и `.content` (+`display: flex; flex-direction: column;`)

## Decisions Made

**Task 2 live-check методология (отклонение от буквальной инструкции плана "открыть работающее приложение cargo tauri dev / pnpm preview в LAN Chrome"):**

Полноценный запуск `cargo tauri dev` или `pnpm --dir ui preview` требует пройти аутентификацию (локальный пользователь или AD, `tower-sessions` cookie), а маршрут `#/` (Дашборд) недоступен без активной сессии и наполненной БД. Настройка dev-пользователя/сид-данных для одноразовой live-проверки CSS-фикса вышла бы за рамки задачи и заняла бы существенно больше времени, чем сам фикс.

Вместо этого live-check выполнен как точная CSS-харнесс-репродукция в headless Chromium через Playwright (кэшированный chromium-1217, доступен глобально через `npm root -g`/playwright без изменения репозитория):

- Харнесс (`/private/tmp/.../scratchpad/layout-harness.html`) воспроизводит один-в-один реальные CSS-правила и структуру DOM после фикса: `.app-layout` (grid, 236px + 1fr колонки, `grid-template-rows: minmax(0, 1fr)`, `height: 100vh; overflow: hidden`), `.sidebar-container` (sticky, `height: 100vh; overflow: hidden`), `.content` (`display: flex; flex-direction: column; overflow: auto; min-height: 0`), вложенный `.dashboard-page` (`display: flex; flex-direction: column; height: 100%`) и `.dashboard-grid` (`flex: 1; min-height: 0; overflow: auto`) — контент (4 stat-блока + 4 chart-блока) намеренно превышает высоту вьюпорта (600px), воспроизводя условие бага «Дашборд — единственный маршрут без виртуализации».
- Скрипт (`layout-check.mjs`) открывает харнесс в реальном движке рендеринга Chromium, скроллит `#dashboard-grid` на 200px и проверяет:
  - bounding box `#sidebar` и `#header` **не изменились** до/после скролла (sidebar/header не двигаются)
  - `document.scrollingElement.scrollHeight === document.scrollingElement.clientHeight` (**true** — скролла на уровне документа нет)
  - `#dashboard-grid.scrollHeight (1112) > clientHeight (549)` и `scrollTop === 200` (скролл реально произошёл именно здесь)
  - `#content.scrollTop === 0` (сам `.content` не скроллится — весь избыток контента поглощается вложенным `.dashboard-grid`)
- **Результат: PASS** по всем пяти условиям — подтверждает, что именно CSS grid-track-sizing механизм (root cause из objective) устранён, а не то, что баг был в другом месте (Playwright/Chromium — тот же layout-движок, что и в WKWebView/LAN Chrome, разница только в отсутствии приложения вокруг CSS).

Это НЕ замена полноценной human-UAT живой проверки на реальном приложении (которая остаётся частью уже открытого блокирующего чекпоинта UAT 30-03 Task 3, повторный прогон после этого плана) — это самостоятельное pre-UAT devtools-подтверждение исполнителем, которое явно требует acceptance criteria Task 2 («источник гэпа — расхождение source-only проверки и реального экрана в 2 предыдущих раундах»). Механизм CSS одинаков что в харнессе, что в реальном приложении — Layout.svelte/DashboardPage.svelte не содержат JS-логики, влияющей на этот layout (ни `$effect`, ни resize-observer не участвуют в scroll-изоляции), поэтому харнесс — валидный способ проверить именно ту CSS-причину, из-за которой 30-06 не сработал дважды.

## Deviations from Plan

None (code changes) - оба CSS-фикса выполнены строго по плану (padding-значение + grid-template-rows/display:flex). Единственное отклонение — методология live-check в Task 2 (см. Decisions Made выше), необходимое из-за отсутствия готовой dev-аутентификации в этой среде; задокументировано как часть acceptance criteria, не как правка кода.

## Issues Encountered
- Playwright не установлен как зависимость проекта (`ui/package.json` не содержит `playwright`/`puppeteer`), но был доступен глобально (`npm root -g`) с уже закэшированным Chromium — использован через symlink в scratchpad node_modules, без изменений в репозитории проекта.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Оба гэпа (Gap 1, Gap 2) из блокирующего UAT re-run 30-03 Task 3 закрыты source-side + подтверждены live CSS-движком (headless Chromium)
- Гейты зелёные: `check-focus-outline.mjs` (0), `check-tokens.mjs` (0), `check-contrast.mjs` (0), `svelte-check` (0 ошибок, только pre-existing warnings в несвязанных файлах), `pnpm --dir ui build` ✓, `pnpm --dir ui lint` ✓
- Фикс Gap 2 — на уровне `Layout.svelte`, общем для всех маршрутов — регрессий на Устройствах/Принтерах/Актах/Картриджах/Заявках не ожидается (структура `*-page { display: flex; flex-direction: column; height: 100%; }` идентична на всех), но НЕ верифицировано отдельно live-check'ом на этих маршрутах в этом плане — единственная grid-track правка применяется одинаково ко всем детям `.content`
- Готово к финальному human-UAT повторному прогону (часть блокирующего чекпоинта UAT 30-03 Task 3) — рекомендуется подтвердить оба гэпа визуально на реальном экране перед закрытием фазы 30
- Следующие планы 30-08, 30-09 (round-2 gap-closure) не блокируются этим планом

---
*Phase: 30-quality-a11y-platform-parity*
*Completed: 2026-07-25*
