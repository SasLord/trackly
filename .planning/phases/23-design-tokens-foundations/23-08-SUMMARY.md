---
phase: 23-design-tokens-foundations
plan: 08
subsystem: ui
tags: [design-tokens, css-custom-properties, rgba-migration, svelte, scss]

# Dependency graph
requires:
  - phase: 23-design-tokens-foundations
    provides: "check-tokens.mjs Правило 4 (rgba/rgb/hsl/hsla-в-style, дефолтное) + --tr-danger-ring токен (план 23-07)"
provides:
  - "14 файлов мигрированы с rgba()-литералов на --tr-overlay/--tr-danger-ring/--tr-elev-* — 0 нарушений check-tokens.mjs (все 4 правила) на всём ui/src"
  - "Button.svelte danger-ring alpha 0.3 -> 0.2 (WR-01-санкционированный visual touch, handoff в фазу 24)"
affects: [24-base-components]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Modal overlay dark-override удалён после миграции на theme-scoped var(--tr-overlay) — токен сам разрешается по-разному в [data-theme='dark'], отдельный CSS-override стал избыточен"
    - "Мёртвые var(--tr-x, rgba(...))-fallback'ы удаляются при миграции (закрытая модель токенов, D-01) — не сохраняются как safety net"

key-files:
  created: []
  modified:
    - ui/src/lib/components/Modal.svelte
    - ui/src/lib/components/Button.svelte
    - ui/src/lib/components/Input.svelte
    - ui/src/lib/components/DatePicker.svelte
    - ui/src/lib/components/LocationAutocomplete.svelte
    - ui/src/lib/components/PersonAutocomplete.svelte
    - ui/src/features/devices/DeviceAutocompleteField.svelte
    - ui/src/features/acts/ActFormItemsTable.svelte
    - ui/src/features/cartridges/ModelFormModal.svelte
    - ui/src/features/dashboard/ChartWidget.svelte
    - ui/src/features/auth/LoginPage.svelte
    - ui/src/features/auth/FirstRunWizard.svelte
    - ui/src/features/auth/PendingScreen.svelte
    - ui/src/features/auth/BlockedScreen.svelte

key-decisions:
  - "Modal.svelte: убран отдельный [data-theme='dark'] override после замены обоих overlay-сайтов на var(--tr-overlay) — токен уже theme-scoped, второй сайт стал буквальным дублем первого"
  - "Все 9 danger-ring сайтов сведены к единому var(--tr-danger-ring); Button.svelte's rgba(220,38,38,0.3) конвертирован в 0.2 — единственное реальное визуальное изменение в этом плане, WR-01-санкционировано (см. handoff ниже)"
  - "ActFormItemsTable.svelte:909 — удалён мёртвый rgba-fallback из var(--tr-elev-1, rgba(0,0,0,0.08)); --tr-elev-1 определён в обеих темах, fallback никогда не срабатывал"

patterns-established: []

requirements-completed: [DS-01]

# Metrics
duration: 15min
completed: 2026-07-17
---

# Phase 23 Plan 08: Call-Site Migration — Modal/Danger-Ring/Elevation rgba() → --tr-* Tokens Summary

**Мигрировал все 17 оставшихся hardcoded rgba()-литералов (14 файлов) на --tr-overlay/--tr-danger-ring/--tr-elev-* — check-tokens.mjs (все 4 правила) впервые проходит на 0 нарушений, DS-01 закрыт буквально (hex И rgba).**

## Performance

- **Duration:** 15 min
- **Completed:** 2026-07-17T19:06:25Z
- **Tasks:** 3 (Tasks 1-2 code migration, Task 3 verification-only — 0 нарушений с первого прогона, доп. фиксов не потребовалось)
- **Files modified:** 14

## Accomplishments
- Modal.svelte: оба overlay-сайта (`rgba(0,0,0,0.4)` / `rgba(0,0,0,0.6)` dark-override) → `var(--tr-overlay)`; избыточный `[data-theme='dark']` override удалён (токен сам theme-scoped)
- 9 дублированных "invalid"-focus-ring сайтов (`rgba(220,38,38,0.2)`/`0.3`) в 8 файлах → `var(--tr-danger-ring)`: PersonAutocomplete, Button, Input, DatePicker, LocationAutocomplete, DeviceAutocompleteField, ModelFormModal, ActFormItemsTable (×2)
- Elevation-тени: ChartWidget-тултип (`rgba(0,0,0,0.15)`) → `var(--tr-elev-2)`; 4 auth-экрана (LoginPage/FirstRunWizard/PendingScreen/BlockedScreen, `rgba(0,0,0,0.08)`) → `var(--tr-elev-2)`; мёртвый rgba-fallback в ActFormItemsTable:909 удалён из `var(--tr-elev-1, ...)`
- `node ui/scripts/check-tokens.mjs` (все 4 правила по умолчанию) впервые за gap-closure раунд завершается `PASS — 0 нарушений` на всём `ui/src`
- `pnpm lint` / `pnpm svelte-check` / `pnpm build` — все зелёные (0 ошибок; предсуществующие warnings не изменились)

## Task Commits

Each task was committed atomically:

1. **Task 1: Modal overlay + danger-ring migration (9 файлов)** - `2e3accd` (fix)
2. **Task 2: Elevation-тени — ChartWidget + 4 auth-экрана + fallback-strip** - `4e54d3e` (fix)
3. **Task 3: Финальный полный гейт + pnpm lint/svelte-check/build + SUMMARY handoff** - verification-only, без кода (см. ниже); задокументировано в этом коммите (metadata)

**Plan metadata:** (pending — final commit below)

## Files Created/Modified
- `ui/src/lib/components/Modal.svelte` — overlay → `var(--tr-overlay)`, dark-override удалён
- `ui/src/lib/components/PersonAutocomplete.svelte` — danger-ring → токен
- `ui/src/lib/components/Button.svelte` — danger-ring → токен (alpha 0.3→0.2, см. Button.svelte visual-touch handoff ниже)
- `ui/src/lib/components/Input.svelte` — danger-ring → токен
- `ui/src/lib/components/DatePicker.svelte` — danger-ring → токен
- `ui/src/lib/components/LocationAutocomplete.svelte` — danger-ring → токен
- `ui/src/features/devices/DeviceAutocompleteField.svelte` — danger-ring → токен
- `ui/src/features/cartridges/ModelFormModal.svelte` — danger-ring → токен
- `ui/src/features/acts/ActFormItemsTable.svelte` — danger-ring ×2 (item-qty + qty-input invalid) → токен, elevation fallback-strip
- `ui/src/features/dashboard/ChartWidget.svelte` — tooltip-тень → `var(--tr-elev-2)`
- `ui/src/features/auth/LoginPage.svelte` — card-тень → `var(--tr-elev-2)`
- `ui/src/features/auth/FirstRunWizard.svelte` — card-тень → `var(--tr-elev-2)`
- `ui/src/features/auth/PendingScreen.svelte` — card-тень → `var(--tr-elev-2)`
- `ui/src/features/auth/BlockedScreen.svelte` — card-тень → `var(--tr-elev-2)`

## Decisions Made
- Modal.svelte dark-мode override удалён целиком (не сохранён как «на будущее») — второй overlay-сайт после замены на токен стал буквальным дублем первого; `var(--tr-overlay)` уже разрешается корректно в обеих темах через `[data-theme='dark']`-блок в `_tokens.scss`
- ActFormItemsTable.svelte:909 rgba-fallback удалён без замены на что-либо ещё — `--tr-elev-1` всегда определён (обе темы, `:root`/`[data-theme='light']`/`[data-theme='dark']`), fallback был мёртвым кодом
- Task 3 не потребовал дополнительных фиксов — `check-tokens.mjs` прошёл 0 нарушений с первого прогона после Tasks 1-2, что подтверждает полноту исходного списка из 17 сайтов (interfaces плана 23-08) без пропущенных мест на стыке

## Deviations from Plan

None - plan executed exactly as written. Task 3's verification step required no additional fixes — the 17-site list from the plan's `<interfaces>` section was complete and exhaustive; `check-tokens.mjs` passed cleanly on the first post-migration run.

## Button.svelte Visual-Touch Handoff (для Фазы 24)

**Button.svelte получил минимальный визуальный touch в фазе 23 (danger-ring alpha 0.3→0.2 при миграции на `var(--tr-danger-ring)`, WR-01-санкционировано).** Это НЕ layout-изменение, НЕ API-изменение, НЕ изменение поведения — единственное затронутое свойство: alpha-канал фокус-ring на `.btn-destructive:focus-visible` (было `rgba(220, 38, 38, 0.3)`, стало `var(--tr-danger-ring)` = `rgba(207, 59, 59, 0.2)` в light / `rgba(242, 101, 101, 0.2)` в dark). Причина: `--tr-danger-ring` (план 23-07) канонизирует большинство из 9 дублированных "invalid"-focus-ring сайтов на единое значение 0.2; Button.svelte был единственным сайтом-исключением с 0.3.

**CONTEXT.md резервирует полный визуальный редизайн Button.svelte за фазой 24** (CMP-01). Планирование фазы 24 должно знать, что файл уже частично тронут в фазе токенов — это не должно вызывать удивления при diff-обзоре или конфликтовать с ожидаемым «чистым» стартовым состоянием Button.svelte для редизайна.

## Issues Encountered

None. Все acceptance criteria обоих кодовых задач подтверждены командами из `<verify>`/`<acceptance_criteria>` плана:
- Task 1: `! grep -rq "rgba(220, 38, 38" ui/src && ! grep -Eq "rgba\(0, 0, 0, 0\.(4|6)\)" ui/src/lib/components/Modal.svelte` → CLEAN; `grep -c "var(--tr-danger-ring)" ActFormItemsTable.svelte` → 2; `grep -c "var(--tr-overlay)" Modal.svelte` → 1
- Task 2: `! grep -rEq "rgba\(0, 0, 0, 0\.(15|08)\)" ui/src` → CLEAN; `grep -c "var(--tr-elev-2)" ChartWidget.svelte` → 1; 4 auth-файла суммарно → 4; `grep -c "var(--tr-elev-1);" ActFormItemsTable.svelte` → 1
- Task 3: `node scripts/check-tokens.mjs` → exit 0, "PASS — 0 нарушений"; `pnpm lint` → exit 0; `pnpm svelte-check` → 0 ERRORS (48 pre-existing WARNINGS, baseline не изменился); `pnpm build` → exit 0, успешная сборка (pre-existing warnings те же: `state_referenced_locally` ×N, `css_unused_selector` ActFormItemsTable/ActFormBody — не относятся к этому плану, out of scope)

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Gap-closure раунд фазы 23 (планы 23-07 + 23-08) полностью закрывает единственный подтверждённый разрыв из 23-VERIFICATION.md: DS-01 буквально закрыт (hex И rgba/hsl литералов в компонентах не осталось), постоянный CI-гейт (`check-tokens.mjs`, все 4 правила) впервые зелёный на 0 нарушений.

Блокеров нет. Фаза 24 (Базовые компоненты — Button/Input/Select/Textarea/Checkbox/Badge/Tabs/Modal) может начинаться; единственная заметка для её планирования — Button.svelte visual-touch handoff выше.

---
*Phase: 23-design-tokens-foundations*
*Completed: 2026-07-17*

## Self-Check: PASSED

- FOUND: ui/src/lib/components/Modal.svelte
- FOUND: ui/src/lib/components/Button.svelte
- FOUND: ui/src/lib/components/Input.svelte
- FOUND: ui/src/lib/components/DatePicker.svelte
- FOUND: ui/src/lib/components/LocationAutocomplete.svelte
- FOUND: ui/src/lib/components/PersonAutocomplete.svelte
- FOUND: ui/src/features/devices/DeviceAutocompleteField.svelte
- FOUND: ui/src/features/acts/ActFormItemsTable.svelte
- FOUND: ui/src/features/cartridges/ModelFormModal.svelte
- FOUND: ui/src/features/dashboard/ChartWidget.svelte
- FOUND: ui/src/features/auth/LoginPage.svelte
- FOUND: ui/src/features/auth/FirstRunWizard.svelte
- FOUND: ui/src/features/auth/PendingScreen.svelte
- FOUND: ui/src/features/auth/BlockedScreen.svelte
- FOUND commit: 2e3accd (Task 1)
- FOUND commit: 4e54d3e (Task 2)
