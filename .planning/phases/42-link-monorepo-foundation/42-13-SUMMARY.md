---
phase: 42-link-monorepo-foundation
plan: 13
subsystem: kit-components-gates
tags: [components, gates, mutation, LNK-01]
requires: [42-12]
provides:
  - packages/kit/src/components - ActionMenu, ActionSubmenu, TableRow, Modal, DatePicker, Badge, Tabs, DetailPanel, Dropdown, actionMenuContext.ts
  - перенацеленные гейты action-menu-portal, modal-isolation, date-picker-layout, device-status-labels, chat-ui, movements-type-filter, group-vocabulary
key-files:
  modified:
    - ui/scripts/check-{action-menu-portal,modal-isolation,date-picker-layout,device-status-labels,chat-ui,movements-type-filter,group-vocabulary}.mjs
    - ui/scripts/fixtures/{move-map.json,gate-coverage.baseline.json}
metrics:
  tasks: 2
  completed: 2026-11-26
---

# Phase 42 Plan 13: компоненты гейтов в packages/kit Summary

Девять запланированных компонентов и `actionMenuContext.ts` переехали в `packages/kit` кодмодом (114 правок импортов, без прокладок); семь гейтов перенацелены и подтверждены мутациями реальных исходников kit. Коммит: d896fdb4.

## Классификация

`--classify` дал «чисто» для всех десяти файлов; остались в `ui/` ноль. Кодмод отказался переносить `Modal` (импорт `icons/IconClose`), поэтому по Rule 3 вместе с ними переехали транзитивные зависимости: `UnreadBadge.svelte` (импорт из `Tabs`), `icons/IconClose.svelte`, `icons/RowChevron.svelte` (из `TableRow`). Итого 13 файлов. Внутрипакетный импорт `./actionMenuContext` оставлен без расширения (кодмод поставил `.ts`, исправлено).

## Гейты

- action-menu-portal: `MENU_FILE`/`TABLE_ROW_FILE`/`SUBMENU_FILE` = `../packages/kit/src/components/*` (ключи `files` строятся `path.relative(root, abs)`, walk() включает пакеты); импорты в фикстурах - `@trackly/kit/components/ActionMenu.svelte`. 29 фикстур.
- modal-isolation: `MODAL_FILE` = `../packages/kit/src/components/Modal.svelte` (путь через `path.join(root, ...)`, абсолютный сюда не годится). 13 мутантов.
- date-picker-layout: `COMPONENT` = `KIT_SRC/components/DatePicker.svelte`.
- device-status-labels: `BADGE_PATH` в kit, литералы `@trackly/kit/components/{Badge,Tabs}.svelte` в фикстурах. 62 фикстуры.
- chat-ui: путь DetailPanel = `../../packages/kit/src/components/DetailPanel.svelte` (относительно `SRC`). 90 мутантов.
- movements-type-filter: INV-1 теперь требует `import Dropdown from '@trackly/kit/components/Dropdown.svelte'` (смысл правила сохранён).
- group-vocabulary: путь TableRow в kit.
- act-delete-notices, place-path-short: на переехавшие файлы по пути не ссылаются, изменений не потребовали (остались зелёными, selftest зелёный).

## Мутации реальных исходников kit (красный, возврат, зелёный)

- Modal: `modal-container` -> `modal-box` -> modal-isolation красный.
- ActionMenu: снят `use:portal` у подложки -> action-menu-portal красный.
- DatePicker: убран импорт `needsOwnCalendarIcon` -> date-picker-layout красный.
- ReportFilters (потребитель): импорт Dropdown вернули на `$lib/...` -> movements-type-filter красный.
- DetailPanel: у `.with-subheader` `border-bottom` -> `none` -> chat-ui красный.
- Badge: снят inset `box-shadow` у `.badge-m-soft.badge-m-muted` -> device-status-labels красный (F6).

Первая попытка на DetailPanel (снят `class:with-subheader`) и на Badge (снят `background`) гейтами не ловится: они этого не проверяют, это не вакуумность; перешёл на защищаемые правилами строки. Все якоря уникальны (проверено grep -c / номером строки); после каждой мутации `git checkout --` на закоммиченную версию, повторный прогон зелёный.

## Сниженные и перенацеленные счётчики

- `place-tree-invalidation` pinned (`ui/src`) 292 -> 279: ушли 13 файлов общих компонентов; гейт ищет write-site'ы доменных API, на компоненты не смотрит.
- `chat-ui` pinned: вход DetailPanel перенацелен на `packages/kit/src/components/DetailPanel.svelte` (счётчик не менялся).
- Остальные per-gate счётчики не менялись; `check-gate-coverage`: 308 файлов в скане, 18 обходчиков, 4233 файла.

## Проверки

`pnpm --dir ui lint` (74 гейта) exit 0, `svelte-check` 422 файла 0/0, `vite build` зелёный, privacy-гейт pre-commit PASS.

## Отклонения

**[Rule 3] транзитивные зависимости:** UnreadBadge, IconClose, RowChevron перенесены вместе с компонентами (без них kit импортировал бы из `ui/`).

Заглушек и новых поверхностей угроз нет.

## Self-Check: PASSED
