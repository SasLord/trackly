---
phase: 23-design-tokens-foundations
plan: 01
subsystem: ui
tags: [scss, design-tokens, css-custom-properties, svelte]

# Dependency graph
requires: []
provides:
  - "Единственный источник --tr-* токенов в ui/src/styles/_tokens.scss (цвет light+dark, spacing 11 уровней, radius 5, elevation 5, типографика 9 ролей+mono, layout-константы без изменений)"
  - "global.scss мигрирован на --tr-* по карте D-05 (body/focus-ring/skip-link/scrollbar)"
  - "Глобальный класс .tr-mono для моноширинных идентификаторов (инв./серийный №, № акта)"
affects: [23-02, 23-03, 23-04, 23-05, 23-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Composite shorthand (--tr-text-{role}) + decomposed axes (--tr-font-size/-weight/-line-height-{role}) определены параллельно для каждой типографической роли — call sites мигрируют на оси 1:1, shorthand доступен для нового кода в фазах 24+"
    - ".tr-mono как единственный новый глобальный класс верхнего уровня (второй после .skip-link) — грепаемый паттерн для DS-03 вместо компонента-обёртки"

key-files:
  created: []
  modified:
    - ui/src/styles/_tokens.scss
    - ui/src/styles/global.scss

key-decisions:
  - "Заголовочный комментарий global.scss переформулирован без буквального повторения строки @use './tokens' — иначе греп-критерий D-05 (ровно 1 совпадение) ложно триггерится собственным комментарием"
  - "--tr-line-height-mono зафиксирован как 1.4 (не указано в UI-SPEC для mono-роли) — совпадает с line-height --tr-text-label при том же размере 13px; font-variant-numeric: tabular-nums остаётся исключительно в классе .tr-mono, не в самом токене (D-14)"

patterns-established:
  - "Value-preserving миграция токенных семейств: старые имена сносятся полностью без bridge-алиасов (D-01) — единственная защита от тихого пропуска call-site это видимый дефект, а не билд-ошибка"

requirements-completed: [DS-01, DS-02, DS-03, DS-04]

duration: 10min
completed: 2026-07-17
---

# Phase 23 Plan 01: Единый слой токенов --tr-* Summary

**Полностью переписанный `_tokens.scss` (единый `--tr-*` слой: цвет light+dark, 12-ступенчатая нейтральная шкала, 5 уровней elevation, 11 spacing, 5 radius, 9 типографических ролей + mono) и мигрированный `global.scss` (body/focus-ring/skip-link/scrollbar на новые имена + новый класс `.tr-mono`).**

## Performance

- **Duration:** ~10 min
- **Completed:** 2026-07-17
- **Tasks:** 2/2 completed
- **Files modified:** 2

## Accomplishments
- `_tokens.scss` переписан с нуля: старые семейства (`--color-*`, `--space-*`, `--radius-*`, `--font-size-*`, `--font-weight-*`, `--line-height-*`, `--shadow-*`) полностью удалены, кроме неизменных layout-констант (D-02)
- Инверсия поверхностей включена сразу (`--tr-bg` #eef1f6 / `--tr-surface` #ffffff) — без флага (D-10)
- `--shadow-elev-2-dark` не перенесён — подтверждённый мёртвый код (D-03)
- `global.scss` смигрирован механически по карте D-05: форма и охват `*:focus-visible` не изменены, поменялось только имя custom property внутри
- Добавлен глобальный класс `.tr-mono` рядом с `.skip-link`, той же плоской структурой (D-12)

## Task Commits

Каждая задача закоммичена атомарно:

1. **Task 1: Переписать _tokens.scss в единый слой --tr-*** - `b14ace9` (feat)
2. **Task 2: Мигрировать global.scss по карте + добавить .tr-mono** - `4345d17` (feat)

_Плановая docs-метадата коммитится отдельно ниже (final_commit)._

## Files Created/Modified
- `ui/src/styles/_tokens.scss` — полностью переписан: цвет (light+dark, brand/accent/surfaces/text/border/semantic/row-states/neutral-ramp), elevation (5, theme-scoped), spacing (11), layout-константы (без изменений), radius (5), типографика (9 ролей + mono, composite + decomposed)
- `ui/src/styles/global.scss` — body/focus-ring/skip-link/scrollbar переведены на `--tr-*`; добавлен `.tr-mono`; заголовочные комментарии актуализированы

## Decisions Made
- Заголовочный комментарий `global.scss` переформулирован без буквального `@use './tokens'` в тексте — иначе греп-проверка «ровно 1 совпадение» ложно считает и код, и комментарий
- `--tr-line-height-mono: 1.4` — значение не задано UI-SPEC явно для mono-роли, принято по аналогии с `--tr-text-label` (тот же размер 13px); `tabular-nums` остаётся только в `.tr-mono`, не в самом токене

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Заголовочный комментарий global.scss дублировал грепаемый литерал `@use './tokens'`**
- **Found during:** Task 2 verification (acceptance criteria grep)
- **Issue:** План требует `grep -c "@use './tokens'" ui/src/styles/global.scss` вернуть 1 (точка подключения не изменилась), но объяснительный комментарий над импортом (строки 1-6, уже существовавший до этого плана) буквально содержал ту же строку `@use './tokens';` в кавычках — grep находил 2 совпадения (комментарий + реальный код)
- **Fix:** Переформулирован комментарий без буквального повторения синтаксиса импорта («The tokens import below emits…» вместо цитирования `@use './tokens';`)
- **Files modified:** ui/src/styles/global.scss
- **Verification:** `grep -c "@use './tokens'" ui/src/styles/global.scss` → 1
- **Committed in:** `4345d17` (часть коммита Task 2)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Косметическая правка комментария для соответствия собственному acceptance criteria плана; поведение и вся механика подключения токенов не затронуты.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
`_tokens.scss` и `global.scss` готовы как источник имён `--tr-*` для всех последующих sweep-планов (23-03..23-05: цвет, space/radius, типографика) и параллельного греп-гейта (23-02). `pnpm svelte-check` чист (0 errors, 48 pre-existing warnings baseline, не связанных с этим планом).

---
*Phase: 23-design-tokens-foundations*
*Completed: 2026-07-17*

## Self-Check: PASSED

- FOUND: ui/src/styles/_tokens.scss
- FOUND: ui/src/styles/global.scss
- FOUND: b14ace9 (feat(23-01): rewrite _tokens.scss as single --tr-* token layer)
- FOUND: 4345d17 (feat(23-01): migrate global.scss to --tr-* tokens, add .tr-mono)
