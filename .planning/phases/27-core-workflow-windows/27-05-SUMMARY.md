---
phase: 27-core-workflow-windows
plan: 05
subsystem: ui
tags: [svelte5, scss, design-tokens, cartridges]

# Dependency graph
requires:
  - phase: 23-design-tokens-foundations
    provides: "слой токенов --tr-* (surfaces/text/accent/semantic/neutrals/shadows/spacing/radius/typography)"
  - phase: 24-base-components
    provides: "примитивы Button/Input/Select/Textarea/Checkbox/Modal на новой токен-системе"
provides:
  - "OperationModal.svelte (887 стр.) подтверждён полностью на токенах/примитивах — аудит без изменений кода"
  - "ModelFormModal.svelte (580 стр.) — устранены 3 остаточных хардкод-px в scoped-стилях (gap/margin-top/dropdown-offset), переведены на var(--tr-space-2xs|3xs)"
affects: [28-support-admin-windows, 30-quality-a11y-parity]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Автокомплит бренда/модели в ModelFormModal — inline bespoke-input паттерн (идентичен LocationAutocomplete до Phase 18 portal-миграции), НЕ переведён на Input-примитив осознанно: примитив не поддерживает keyboard-nav/dropdown-логику; логика подсказок вне SC #4"

key-files:
  created: []
  modified:
    - ui/src/features/cartridges/ModelFormModal.svelte

key-decisions:
  - "OperationModal.svelte не изменён — уже полностью на var(--tr-*) и примитивах Select/DatePicker/Textarea/PersonAutocomplete/LocationAutocomplete/CartridgeSelect/PrinterSelect/Button после sweep фазы 23 (batch 2/4); Task 1 выполнен как чистый аудит без коммита кода"
  - "ModelFormModal.brand/model автокомплит (inline raw <input>+<button>-dropdown) оставлен как есть — не переведён на portal+dropdownAnchor (паттерн Phase 18 AUTO-01..05): это функциональное изменение вне границы SC #4 (purely-visual re-tokenization); только 3 хардкод-px значения (gap:4px→var(--tr-space-2xs), margin-top:2px→var(--tr-space-3xs), calc(100%+2px)→calc(100%+var(--tr-space-3xs))) заменены на токены — визуально идентично, риск нулевой"

patterns-established: []

requirements-completed: []  # WIN-04 доставляется window-планом, не этим внутренним re-token planом (per phase_context)

duration: 12min
completed: 2026-07-21
---

# Phase 27 Plan 05: Re-tokenization OperationModal + ModelFormModal Summary

**Аудит и точечная доработка двух крупнейших модалок Картриджей (887 и 580 строк) — обе уже почти полностью на `var(--tr-*)`/примитивах благодаря sweep фазы 23; исправлены 3 остаточных хардкод-px значения в ModelFormModal.**

## Performance

- **Duration:** 12 min
- **Tasks:** 2
- **Files modified:** 1 (ModelFormModal.svelte)

## Accomplishments

- Полный аудит `OperationModal.svelte` (887 строк): 0 raw-hex цветов, 15 использований `var(--tr-*)`, все контролы уже на примитивах (`Select`/`DatePicker`/`Textarea`/`PersonAutocomplete`/`LocationAutocomplete`/`CartridgeSelect`/`PrinterSelect`/`Button`) — многошаговая логика (prevStateOptions/stateOptions, previous-cartridge блок, printer-context lookup) не тронута, файл уже соответствует D-04 целиком.
- Полный аудит `ModelFormModal.svelte` (580 строк): 0 raw-hex, 42 использования `var(--tr-*)`, найдены и устранены 3 остаточных хардкод-px значения в scoped-стилях, не покрытых механическим sweep'ом фазы 23 (тот шёл по старым `--space-*`/`--radius-*`/`--color-*` именам, эти значения были «голыми» px изначально).
- Верифицирован closed-world token-гейт (`check-tokens.mjs`), `svelte-check` (0 ошибок), `pnpm lint` (eslint+prettier+check-tokens), `pnpm build` — все зелёные.

## Task Commits

1. **Task 1: OperationModal (887) — ре-токенизация (D-04)** — аудит без изменений кода (файл уже полностью соответствует D-04 после sweep фазы 23, batch 2/4 — коммиты `473358a`/`66705ee`/`fe4685b`); отдельного коммита нет.
2. **Task 2: ModelFormModal (580) — ре-токенизация (D-04)** — `6d60eaf` (refactor)

**Plan metadata:** (текущий коммит — docs)

## Files Created/Modified

- `ui/src/features/cartridges/ModelFormModal.svelte` — `.field { gap: 4px }` → `var(--tr-space-2xs)`; `.field-error { margin-top: 2px }` → `var(--tr-space-3xs)`; `.dropdown { top: calc(100% + 2px) }` → `calc(100% + var(--tr-space-3xs))`. Значения идентичны (space-2xs=4px, space-3xs=2px) — визуальный ноль-diff, только источник значения меняется с литерала на токен.

## Decisions Made

- **Task 1 — без изменений кода.** `OperationModal.svelte` уже был полностью ре-токенизирован в рамках фазы 23 (типографика/space/radius sweep по значению, а не по имени переменной — это специально ловило «голые» токен-совместимые значения). Повторная проверка (`grep -cE "background: *#|color: *#"` = 0, `grep -c "var(--tr-"` = 15, полное отсутствие raw `<input>/<select>/<button>/<textarea>` в шаблоне) подтвердила соответствие всем acceptance criteria плана без необходимости правок.
- **Task 2 — минимальная точечная доработка.** `ModelFormModal.svelte` тоже прошёл sweep фазы 23, но три значения (`gap: 4px`, `margin-top: 2px`, `calc(100% + 2px)`) не были захвачены механическим sed-проходом (тот таргетировал старые имена `--space-*`, а не «голые» px-литералы) — исправлены вручную под букву задачи «все scoped-стили → var(--tr-*)».
- **Автокомплит бренда/модели НЕ переведён на `Input`-примитив и НЕ мигрирован на `portal`+`dropdownAnchor`.** Установленный в кодовой базе паттерн (идентичный `LocationAutocomplete`/`PersonAutocomplete`/`DeviceAutocompleteField` ДО их Phase-18 portal-миграции) — inline raw `<input class="autocomplete-input">` + bespoke `<div class="dropdown">`+`<button class="dropdown-item">` с ручной keyboard-навигацией (Escape/ArrowUp/ArrowDown/Enter/Tab) и debounce-логикой подсказок. `Input`-примитив не поддерживает такую комбинацию (нет hook-ов под onfocus/onkeydown/aria-autocomplete в этой форме), а перевод на `portal`+`dropdownAnchor` (как в мигрированных аналогах) — это функциональное изменение позиционирования дропдауна (потенциально влияет на клиппинг внутри модалки), выходящее за границу SC #4 («многошаговый workflow, поля, валидация и логика не меняются»). Это тот же bespoke-input паттерн, что и в `CompatibilityEditor.svelte` (соседний файл, не входящий в `files_modified` этого плана) — согласованность оставлена как есть, вне явного скоупа плана 27-05.

## Deviations from Plan

None — план выполнен как написано. Task 1 не потребовал изменений кода (уже соответствовал D-04 из прошлой фазы) — задокументировано выше как ключевое решение, не как отклонение от процесса (acceptance criteria плана проверены и подтверждены).

## Issues Encountered

None.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Обе крупнейшие модалки Картриджей (`OperationModal`, `ModelFormModal`) подтверждены закрытыми под D-04 — bespoke-классов, дублирующих примитивы, не осталось; hex-цветов нет; все контролы на примитивах или на установленном bespoke-autocomplete паттерне.
- `check-tokens.mjs`/`svelte-check`/`lint`/`build` зелёные — блокеров для следующих планов фазы 27 (окна Картриджей/Принтеров, D-01..D-03/D-05) нет.
- WIN-04 не помечен complete в REQUIREMENTS.md — по `phase_context` это доставляется window-планом (страница `CartridgesPage`/`CartridgesMasterDetail`), не этим внутренним re-token планом.

---
*Phase: 27-core-workflow-windows*
*Completed: 2026-07-21*
