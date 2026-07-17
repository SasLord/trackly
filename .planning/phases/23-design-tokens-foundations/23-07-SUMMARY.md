---
phase: 23-design-tokens-foundations
plan: 07
subsystem: ui
tags: [design-tokens, css-custom-properties, lint-tooling, svelte, scss]

# Dependency graph
requires:
  - phase: 23-design-tokens-foundations
    provides: "Единый слой --tr-* токенов (плана 23-01..23-06), постоянный CI-гейт check-tokens.mjs (Правила 1-3), one-shot верификатор verify-value-map.mjs"
provides:
  - "check-tokens.mjs Правило 4 — детекция rgba()/rgb()/hsl()/hsla() литералов внутри <style>-блоков .svelte-файлов, включено в дефолтный набор правил"
  - "--tr-danger-ring токен (light + dark) в _tokens.scss, готов к потреблению планом 23-08"
  - "verify-value-map.mjs без CR-01 (все токены многотокенной строки, не только первый) + run-if-main guard, named exports tokensOnSide/checkHunk импортируемы"
affects: [23-08-call-site-migration]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "check-tokens.mjs Rule N: STYLE_BLOCK_RE + .svelte-only фильтр + per-block detection regex — Правило 4 копирует структуру Правила 2 (hex-in-style), меняя только регэксп детекции"
    - "run-if-main guard: process.argv[1] === fileURLToPath(import.meta.url) — паттерн для CLI-скриптов, которые нужно и запускать напрямую, и импортировать как модуль без побочных process.exit()"

key-files:
  created: []
  modified:
    - ui/scripts/check-tokens.mjs
    - ui/scripts/verify-value-map.mjs
    - ui/src/styles/_tokens.scss

key-decisions:
  - "Правило 4 намеренно ловит rgba()/hsl() где угодно внутри style-блока, включая внутри var(--tr-x, rgba(...))-fallback'ов — закрытая модель токенов (D-01) делает такие fallback'и мёртвым кодом"
  - "--tr-danger-ring = alpha 0.2 от --tr-danger каждой темы (rgb-компоненты скопированы дословно из существующих --tr-danger-soft/-text без пересчёта) — канонизирует большинство из 9 дублированных 'invalid'-focus-ring сайтов; Button.svelte сегодня использует 0.3, план 23-08 намеренно сводит к единому значению"
  - "tokensOnSide() применяет глобальный не-анкорённый паттерн к каждой строке отдельно (без m-флага) вместо одного анкорённого + ленивого паттерна на весь текст хунка — фикс CR-01"

patterns-established: []

requirements-completed: [DS-01, DS-04]

# Metrics
duration: 15min
completed: 2026-07-17
---

# Phase 23 Plan 07: Gap-Closure Tooling — Rule 4 + --tr-danger-ring + CR-01 Fix Summary

**Расширил постоянный CI-гейт check-tokens.mjs новым Правилом 4 (rgba/rgb/hsl/hsla-в-style), добавил --tr-danger-ring токен в обе темы _tokens.scss, и исправил CR-01 в verify-value-map.mjs (регекс терял второй+ токен на многотокенной строке) + добавил run-if-main guard для безопасного импорта.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-07-17T18:45:00Z
- **Completed:** 2026-07-17T19:00:23Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- `check-tokens.mjs` теперь ловит 17 существующих rgba()-нарушений (Modal.svelte overlay, Button.svelte focus ring и др.) — закрывает подтверждённый в 23-VERIFICATION.md слепой участок Правила 2 (hex-only)
- `--tr-danger-ring` определён в обеих темах, готов к потреблению планом 23-08 (миграция 9 дублированных "invalid"-focus-ring сайтов)
- `verify-value-map.mjs`: CR-01 закрыт — все токены на многотокенной строке извлекаются с обеих сторон хунка; named exports (`tokensOnSide`, `checkHunk`) безопасно импортируемы благодаря run-if-main guard

## Task Commits

Each task was committed atomically:

1. **Task 1: Исправить CR-01 в verify-value-map.mjs — все токены на строке + run-if-main guard** - `eee8f07` (fix)
2. **Task 2: check-tokens.mjs Правило 4 (rgba/rgb/hsl/hsla-в-style) + --tr-danger-ring токен** - `3e063a7` (feat)

**Plan metadata:** (pending — final commit below)

## Files Created/Modified
- `ui/scripts/verify-value-map.mjs` — `tokensOnSide()` helper заменяет анкорённый matchAll-паттерн, `main()` под run-if-main guard, `tokensOnSide`/`checkHunk` экспортированы
- `ui/scripts/check-tokens.mjs` — `checkColorFunctionsInStyle()` (Правило 4), дефолт `args.rules` расширен до `[1, 2, 3, 4]`, `printHelp()`/шапка-комментарий обновлены
- `ui/src/styles/_tokens.scss` — `--tr-danger-ring: rgba(207, 59, 59, 0.2)` (light), `--tr-danger-ring: rgba(242, 101, 101, 0.2)` (dark)

## Decisions Made
- Правило 4 матчится где угодно внутри style-блока (включая `var(--tr-x, rgba(...))`-fallback'ы) — намеренно, т.к. закрытая модель токенов делает такие fallback'ы мёртвым кодом для удаления, не сохранения
- `--tr-danger-ring` alpha зафиксирована на 0.2 (большинство из 9 сайтов), Button.svelte's 0.3 конвертируется планом 23-08, не этим планом
- tokensOnSide() не использует `m`-флаг — паттерн применяется к уже выделенной одной строке после `split('\n')`, а не к многострочному тексту хунка

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. Все acceptance criteria обоих задач подтверждены командами из `<verify>`/`<acceptance_criteria>` плана:
- Task 1: inline-репродюсер (commit 16244e2) → `PASS ["--space-sm","--space-md"] ["--tr-space-xs","--tr-space-md"]`; `node scripts/verify-value-map.mjs HEAD` → exit 0, 0 хунков; `node scripts/verify-value-map.mjs 6425d30c` → exit 0, 578 хунков, 0 нарушений; оба grep-совпадения (`function tokensOnSide`, `fileURLToPath(import.meta.url)) main()`) — по 1
- Task 2: `node ui/scripts/check-tokens.mjs --rules=4` → exit 1, 17 нарушений (≥16), включает Modal.svelte и Button.svelte; `grep -c "rules: \[1, 2, 3, 4\]"` → 1; `grep -c "tr-danger-ring" _tokens.scss` → 2; `node ui/scripts/check-tokens.mjs --rules=1,2,3` → exit 0 (правила 1-3 не задеты)

**Ожидаемое (не дефект):** `node ui/scripts/check-tokens.mjs` (дефолтный набор правил, без `--rules`) теперь завершается exit-кодом 1 — 17 rgba()-нарушений на call-site'ах, которые ещё не мигрированы. Это Wave 1 плана (только tooling + токен, без изменения компонентов) — `pnpm lint` останется красным до плана 23-08 (Wave 2), который мигрирует все 17 сайтов на `var(--tr-danger-ring)`/`var(--tr-overlay)`/`var(--tr-elev-*)` и восстановит зелёный `pnpm lint`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

План 23-08 (call-site migration, Wave 2) может немедленно потреблять:
- `--tr-danger-ring` (обе темы) для 9 "invalid"-focus-ring сайтов
- `check-tokens.mjs --rules=4` как объективный gate для подтверждения полноты миграции (0 нарушений после 23-08)
- Исправленный `verify-value-map.mjs` доступен для регрессионной проверки, если 23-08 затронет space/radius токены косвенно

Блокеров нет. `pnpm lint` намеренно красный до завершения 23-08 — ожидаемое промежуточное состояние gap-closure раунда, задокументированное выше.

---
*Phase: 23-design-tokens-foundations*
*Completed: 2026-07-17*

## Self-Check: PASSED

- FOUND: ui/scripts/check-tokens.mjs
- FOUND: ui/scripts/verify-value-map.mjs
- FOUND: ui/src/styles/_tokens.scss
- FOUND commit: eee8f07 (Task 1)
- FOUND commit: 3e063a7 (Task 2)
