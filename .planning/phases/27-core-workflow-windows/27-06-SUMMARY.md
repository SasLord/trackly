---
phase: 27-core-workflow-windows
plan: 06
subsystem: ui
tags: [svelte, scss, design-tokens, tabs-primitive, cartridges]

# Dependency graph
requires:
  - phase: 26-windows-with-mockup
    provides: DeviceFormBody/DeviceContextMenu/DeviceFilters/PrinterAlertBanner as re-tokenization reference patterns
  - phase: 24-base-components
    provides: Tabs/Select/Input/Textarea/Button primitives + var(--tr-*) token layer
provides:
  - CartridgeFormModal/CartridgeFormBody confirmed fully on tokens/primitives (audit only, no code change needed)
  - CompatibilityEditor/CartridgeContextMenu/LowStockBanner confirmed fully on tokens/primitives (audit only, no code change needed)
  - CartridgeFilters status switch-bar migrated from bespoke <button class="tab"> to Tabs primitive (D-05)
affects: [28-support-and-admin-windows]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Tabs primitive string-key adapter for number|null domain ids (String(id) in / Number(key) out), mirrored from DeviceFilters.svelte"

key-files:
  created: []
  modified:
    - ui/src/features/cartridges/CartridgeFilters.svelte

key-decisions:
  - "Tasks 1 и 2 (CartridgeFormModal/CartridgeFormBody, CompatibilityEditor, CartridgeContextMenu, LowStockBanner) не потребовали правок — повторный аудит подтвердил: все контролы уже на примитивах (Input/Select/Textarea/LocationAutocomplete/Modal/Button), все цвета через var(--tr-*), CartridgeContextMenu структурно идентичен DeviceContextMenu, LowStockBanner структурно идентичен близнецу PrinterAlertBanner (тот же color-mix warning-фон)"
  - "CompatibilityEditor использует raw <input> с кастомным inline-autocomplete dropdown (не Input-примитив) — это согласуется с установленным паттерном LocationAutocomplete.svelte, где кастомная listbox/aria-логика не покрывается стандартным Input; не является bespoke-дублированием примитива"

patterns-established: []

requirements-completed: []  # WIN-04 не закрывается этим планом целиком — оконный план 27-04/27-08 закрывает WIN-04 суммарно; этот план — часть D-04/D-05 объёма

# Metrics
duration: ~35min (продолжение после прерывания предыдущего executor'а)
completed: 2026-07-21
---

# Phase 27 Plan 06: Форма картриджа + виджеты + CartridgeFilters→Tabs Summary

**Ре-токенизация формы экземпляра картриджа и виджетов (CompatibilityEditor/CartridgeContextMenu/LowStockBanner) подтверждена аудитом без правок; CartridgeFilters переведён на примитив Tabs (D-05) — удалено 116 строк мёртвого bespoke-CSS.**

## Performance

- **Duration:** ~35 мин (продолжение прерванной сессии)
- **Started:** предыдущая сессия (script+template swap CartridgeFilters уже был выполнен)
- **Completed:** 2026-07-21
- **Tasks:** 3/3
- **Files modified:** 1 (CartridgeFilters.svelte)

## Accomplishments
- Подтверждено (повторным аудитом, не на слово): CartridgeFormModal/CartridgeFormBody уже полностью на примитивах (Input/Select/Textarea/LocationAutocomplete) и токенах — 0 правок нужно.
- Подтверждено: CompatibilityEditor/CartridgeContextMenu/LowStockBanner уже полностью на токенах; CartridgeContextMenu 1:1 совпадает по классам/структуре с эталоном DeviceContextMenu; LowStockBanner 1:1 совпадает по структуре CSS с близнецом PrinterAlertBanner (idénтичный warning-SVG + `color-mix(in srgb, var(--tr-warning) 10%, transparent)` фон) — 0 правок нужно.
- CartridgeFilters.svelte (D-05): switch-bar статусов переведён с самописных `<button class="tab">` на примитив `<Tabs variant="underline">` по образцу `DeviceFilters.svelte` (строковый key-адаптер для `number | null` id статусов); удалены 4 мёртвых CSS-блока после свапа разметки — `.status-bar`, `.status-tab`, `.count-badge` (со вложенным `.count-active`), `.filter-select` (raw `<select>` стиль, замещённый примитивом `Select`).
- Фильтры типа/модели (Select) и логика фильтрации (`onStatusChange`/`onKindChange`/`onModelChange`, `visibleModels`, счётчики из `CartridgeCountsDto`) не тронуты — контракт D-05 (SC #4) сохранён.

## Task Commits

Task 1 (CartridgeFormModal/CartridgeFormBody) и Task 2 (CompatibilityEditor/CartridgeContextMenu/LowStockBanner) — без коммитов: аудит подтвердил, что файлы уже соответствуют acceptance criteria плана без изменений.

1. **Task 3: CartridgeFilters → примитив Tabs (D-05)** - `7b0510f` (refactor)

**Plan metadata:** (данный коммит SUMMARY/STATE/ROADMAP)

## Files Created/Modified
- `ui/src/features/cartridges/CartridgeFilters.svelte` - switch-bar статусов на `Tabs`; удалён мёртвый CSS после свапа разметки, выполненного в прерванной сессии

## Decisions Made
- Тасков 1 и 2 не потребовали изменений кода — повторный аудит (не доверие к отчёту прошлого executor'а) подтвердил соответствие acceptance criteria: `grep -cE "background: *#|color: *#"` == 0 во всех пяти файлах, `var(--tr-` присутствует, все контролы — примитивы.
- CompatibilityEditor оставлен с raw `<input>` для inline-autocomplete строк совместимости (не `Input`-примитив) — согласовано с уже существующим паттерном `LocationAutocomplete.svelte`, где кастомная listbox/aria-логика (arrow-key навигация, debounce suggestions, click-outside close) не покрывается стандартным `Input`. Это не bespoke-дублирование примитива, а обоснованное расширение контракта, полностью на `var(--tr-*)`.

## Deviations from Plan

None - plan executed exactly as written. Отклонений сверх штатного продолжения прерванной сессии (см. `<current_partial_state>`) не возникло.

## Issues Encountered
Предыдущая сессия executor'а была прервана лимитом на середине Task 3 — после swap script+template, но до удаления мёртвого CSS. Эта сессия: провалидировала существующую правку сверкой с эталоном `DeviceFilters.svelte`, удалила 4 неиспользуемых CSS-блока (`.status-bar`, `.status-tab`, `.count-badge`, `.filter-select`), прогнала все гейты (`check-tokens.mjs`, `svelte-check`, `lint`, `build`) — все зелёные, закоммитила Task 3, затем провела независимый повторный аудит Tasks 1–2 (не доверяя утверждению прошлой сессии) и подтвердила их корректность.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Все 6 файлов плана 27-06 на токенах/примитивах без bespoke-остатков; функция/раскладка не изменены (SC #4).
- WIN-04 (окно Картриджей) продолжает закрываться остальными планами фазы 27 (детали/списки/master-detail и т.д.) — этот план закрывал только D-04 (форма+виджеты) и перенесённый сюда D-05 (CartridgeFilters).
- `check-tokens.mjs` closed-world гейт зелёный — нет ссылок на несуществующие токены.

---
*Phase: 27-core-workflow-windows*
*Completed: 2026-07-21*

## Self-Check: PASSED

- FOUND: ui/src/features/cartridges/CartridgeFilters.svelte
- FOUND: .planning/phases/27-core-workflow-windows/27-06-SUMMARY.md
- FOUND commit: 7b0510f
- FOUND commit: 0a51c99
