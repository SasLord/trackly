---
phase: 42-link-monorepo-foundation
plan: 14
subsystem: kit-components
tags: [components, icons, gates, LNK-01]
requires: [42-13]
provides:
  - packages/kit/src/components - AuthShell, Checkbox, DetailField, DetailSection, FormField, Input, Placeholder, Radio, Select, Textarea, ThemeSwitcher, Toast, ToastHost
  - packages/kit/src/components/icons - HistoryKindIcon, IconHistory, IconInsertTemplate, RowReload
key-files:
  modified:
    - ui/scripts/fixtures/{move-map.json,gate-coverage.baseline.json}
metrics:
  tasks: 2
  completed: 2026-11-26
---

# Phase 42 Plan 14: остальные компоненты и иконки в packages/kit Summary

Семнадцать доменно-независимых файлов (13 компонентов и 4 иконки) переехали в `packages/kit` кодмодом (67 правок импортов, без прокладок). Spinner уже был в kit с плана 13. Коммит: 57b3c1b4.

## Классификация

Все 17 перенесённых файлов дали «чисто». `AuthShell` не тянет auth-стор (только `Snippet`), поэтому переехал. Баррель `index.ts` не создавался: механизм (wildcard `exports`) его не требует; `exports` в `package.json` менять не пришлось (`./components/*` покрывает и `icons/`).

## Остаток в `ui/src/lib/components` и причины

| Файл | Блокирующий импорт / причина |
|------|------------------------------|
| CartridgeSelect | bindings |
| MovementTimeline | bindings |
| NumberTemplateField | $lib/api/client, $lib/api/ws, bindings |
| PageHeader | features/layout/layout-state, stores/auth |
| PersonAutocomplete | $lib/api/acts |
| PlacePicker | $lib/api/client, stores/auth, bindings |
| Table | features/devices/deviceListMode |
| GroupedPrinterSelect | type-импорт из bindings-phase6 (DTO принтера) |
| PrinterSelect | type-импорт из bindings-phase6 (DTO принтера) |
| HistoryKindChips | доменная семантика видов событий истории (как решено планом, D-17) |
| NumberMismatchPopup, NumberScriptWarningPopup, NumberTakenPopup | доменная семантика номеров актов (как решено планом, D-17); кодмод --classify «чисто», оставлены осознанно |

Ровно один `Button.svelte` в репозитории; ссылок на перенесённые файлы по `$lib/components/` в `ui/src` нет.

## Гейты

- `check-gate-coverage` [C7]: `place-tree-invalidation` pinned `ui/src` 279 -> 262 (ушли 17 файлов). Обоснование: гейт ищет write-site'ы доменных API (`$lib/api/*`), перенесённые файлы их не импортируют (классификатор «чисто»), на гейт они не влияли. Остальные счётчики не менялись; скан 308 файлов, 18 обходчиков, 4267 файлов.
- Ссылок гейтов на старые пути перенесённых файлов нет (grep по `ui/scripts`), перенацеливание не потребовалось. `check-tokens` и `check-focus-outline` обходят `packages/*/src` через `walk()`.

## Мутации реальных исходников kit (красный, возврат, зелёный)

- `Input.svelte` (строка 105, `box-shadow` в `&:focus-visible`) заменён -> `check-focus-outline` красный («bare outline: none»), возврат -> PASS.
- `Toast.svelte` (строка 37) `border-radius` -> `3px; color: #ff0000` -> `check-tokens` красный («hex literal»), возврат -> PASS.

Якоря заданы номером строки, возврат `git checkout --`.

## Проверки

`pnpm --dir ui lint` exit 0, `svelte-check` 441 файл 0/0, `vite build` зелёный, privacy-гейт pre-commit PASS.

## Отклонения

Только пересчёт pinned-счётчика (выше). Заглушек и новых поверхностей угроз нет.

## Self-Check: PASSED
