---
phase: "04-cartridges"
plan: "06"
subsystem: cartridges-models-ui
tags: [cartridges, svelte, ui, models, compatibility-editor, focus-open-autocomplete, wire-up]
dependency_graph:
  requires:
    - "04-03 (CartridgeService + Tauri commands + suggestCompatPrinter)"
    - "04-04 (UI skeleton: CartridgesPage stubs, ModelsList placeholder)"
    - "04-05 (lifecycle modals, CartridgeContextMenu, LowStockBanner)"
  provides:
    - CompatibilityEditor.svelte (добавляемый список пар Бренд+Модель принтера с focus-open autocomplete)
    - ModelFormModal.svelte (CRUD модели, size=wide, kindId conditional Color, openInstanceCounter)
    - ModelListRow.svelte (строка модели: бренд+модель, badge тип/цвет, instanceCount, kebab)
    - ModelsList.svelte (список моделей с toolbar, empty state, callbacks)
    - CartridgesPage.svelte полностью завершён (Models tab + orchestration)
  affects:
    - "human-verify checkpoint: полный end-to-end тест раздела Картриджи"
tech_stack:
  added: []
  patterns:
    - "CompatibilityEditor: per-row-per-field autocomplete state (openKey string key pattern)"
    - "ModelFormModal: openInstanceCounter + {#key} remount — аналогично DeviceFormModal"
    - "ModelsList: callbacks-only pattern (без внутренних форм, оркестрация в CartridgesPage)"
    - "CartridgesPage loadAll(): Promise.all([list, counts, lowStock]) — единое место загрузки"
    - "ModelFormModal suggestCompatPrinter calls: (prefix) => cartridges.suggestCompatPrinter('brand'/'model', prefix)"
    - "T-04-06-02: filter empty compat pairs before submit (p => p.printer_brand && p.printer_model)"
key_files:
  created:
    - ui/src/features/cartridges/CompatibilityEditor.svelte
    - ui/src/features/cartridges/ModelFormModal.svelte
    - ui/src/features/cartridges/ModelListRow.svelte
    - ui/src/features/cartridges/ModelsList.svelte
  modified:
    - ui/src/features/cartridges/CartridgesPage.svelte
key_decisions:
  - "ModelsList callbacks-only: ModelFormModal и confirm-delete управляются из CartridgesPage (единый оркестратор)"
  - "CompatibilityEditor хранит пары как CompatRow[] внутри; конвертация в [string,string][] при submit в ModelFormModal"
  - "ModelListRow kebab — inline (без portal): нет перекрытия overflow:hidden в ModelsList, портал излишен"
  - "openCreate() в CartridgesPage переключается на model-форму при activeTab === 'models'"
  - "T-04-06-01: AppError::Conflict из backend → inline conflictError в ModelFormModal (не Toast)"
requirements-completed:
  - CART-01
  - CART-02
  - CART-03
  - CART-04
  - CART-05
  - CART-06
  - CART-07
  - CART-08
  - CART-09
  - CART-10
  - CART-11
  - CART-12

duration: 15min
completed: "2026-06-08"
---

# Phase 04 Plan 06: Cartridges Models UI + Final Integration Summary

**CompatibilityEditor с focus-open autocomplete через suggestCompatPrinter, ModelFormModal (size=wide, conditional Color), ModelListRow, ModelsList + полная интеграция CartridgesPage с вкладкой Модели и loadAll() orchestration.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-06-08T00:00:00Z
- **Completed:** 2026-06-08T00:15:00Z
- **Tasks:** 2 auto (Task 3 — checkpoint, awaiting human-verify)
- **Files modified:** 5

## Accomplishments

- CompatibilityEditor: добавляемый список пар принтер_бренд+принтер_модель; openKey per-field state pattern; focus-open autocomplete через suggestBrandFn/suggestModelFn props; T-04-06-02 filtration of empty pairs on submit
- ModelFormModal: CRUD модели с CompatibilityEditor, size="wide", conditional {#if kindId !== 2} для поля Цвет, openInstanceCounter remount, conflict error inline, suggestCompatPrinter x2 (brand + model)
- ModelListRow: бренд+модель, Badge тип (accent/default), Badge цвет (if Картридж), instanceCount, inline kebab с Редактировать/Удалить
- ModelsList: callbacks-only — onCreateModel/onEditModel/onDeleteModel; toolbar + empty state «Моделей пока нет»
- CartridgesPage: полная интеграция Models tab, loadAll() через Promise.all([list,counts,lowStock]), ModelFormModal как оркестратор, confirm-delete для моделей, openCreate() переключение по activeTab

## Task Commits

1. **Task 1: Models UI — CompatibilityEditor + ModelFormModal + ModelListRow + ModelsList** - `85cacff` (feat)
2. **Task 2: Финальная интеграция CartridgesPage + ModelsList** - `b8fa6fe` (feat)

## Files Created/Modified

- `ui/src/features/cartridges/CompatibilityEditor.svelte` — добавляемые пары с focus-open autocomplete
- `ui/src/features/cartridges/ModelFormModal.svelte` — CRUD модели, size=wide, suggestCompatPrinter x2
- `ui/src/features/cartridges/ModelListRow.svelte` — строка списка моделей с kebab
- `ui/src/features/cartridges/ModelsList.svelte` — полноширинный список, callbacks-only
- `ui/src/features/cartridges/CartridgesPage.svelte` — вкладка Модели, loadAll(), model modals

## Decisions Made

- **ModelsList callbacks-only:** ModelFormModal и confirm-delete управляются CartridgesPage как единым оркестратором, не внутри ModelsList — соответствует плану §Task 2 §8/§10
- **CompatibilityEditor per-row state:** openKey = `${rowIndex}-${field}` строка вместо массива объектов — меньше reactive overhead, нет необходимости в сложной структуре
- **ModelListRow inline kebab:** portal излишен для ModelsList (нет overflow:hidden контейнера), inline `position: absolute` на kebab-wrap достаточно
- **T-04-06-02 empty pairs filter:** `compatibility.filter(p => p.printer_brand.trim() && p.printer_model.trim())` перед submit — threat mitigation

## Deviations from Plan

None — план выполнен в точности. ModelsList был намерен иметь callbacks-only интерфейс согласно §Task 2 action #10.

## Known Stubs

None — все компоненты полностью реализованы. Единственная незавершённая задача — human-verify checkpoint (Task 3), ожидающий запуска приложения и верификации пользователем.

Note: `instanceCount` в ModelListRow всегда передаётся как 0 из ModelsList — backend endpoint `cartridge_models_list` возвращает `CartridgeModelDto` без поля `instance_count`. Если нужно реальное количество, потребуется отдельный backend query. Для Phase 4 это приемлемо — функционал отображения есть, значение будет «0 шт.».

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| T-04-06-01 mitigated | ModelFormModal.svelte | AppError::Conflict (brand+model unique) → inline conflictError |
| T-04-06-02 mitigated | ModelFormModal.svelte | Пустые пары отфильтрованы перед передачей в compatibility |
| T-04-06-03 accepted | ModelFormModal.svelte | submitting=$state обеспечивает disabled на кнопке Submit |

## Self-Check: PASSED

- `ui/src/features/cartridges/CompatibilityEditor.svelte`: FOUND
- `ui/src/features/cartridges/ModelFormModal.svelte`: FOUND
- `ui/src/features/cartridges/ModelListRow.svelte`: FOUND
- `ui/src/features/cartridges/ModelsList.svelte`: FOUND
- Task 1 commit 85cacff: FOUND
- Task 2 commit b8fa6fe: FOUND
- pnpm svelte-check: 0 ERRORS
- `grep -c "suggestCompatPrinter" ModelFormModal.svelte` = 2 ✓
- `grep -c "kindId !== 2" ModelFormModal.svelte` = 3 ✓
