---
phase: 41-groups-model-and-editor
plan: 27
subsystem: ui
tags: [svelte, action-menu, portal, gate, gap-closure, uat]
requires: []
provides:
  - "ActionMenu с единственным, портальным режимом вывода панели (пропа portal нет)"
  - "структурный гейт check-action-menu-portal.mjs (правила A-F, --selftest с 9 мутантами) в pnpm lint"
affects: [41-31]
tech-stack:
  added: []
  patterns:
    - "чинить opt-in дефолт компонента, а не call-site; регрессию закреплять структурным гейтом с мутантами и защитой от вакуумности"
    - ":global внутри :has(...), когда селектор достаёт до внутренностей дочернего компонента"
key-files:
  created:
    - ui/scripts/check-action-menu-portal.mjs
  modified:
    - ui/src/lib/components/ActionMenu.svelte
    - ui/src/features/groups/GroupContentsTable.svelte
    - ui/src/features/groups/GroupPrintersList.svelte
    - ui/src/lib/components/NumberTemplateField.svelte
    - ui/src/features/groups/GroupTreeNode.svelte
    - ui/src/features/places/PlaceTreeNode.svelte
    - ui/package.json
key-decisions:
  - "Портал стал единственным режимом ActionMenu: проп portal удалён целиком, чтобы вызов с ним падал в svelte-check и в гейте"
  - "Tab внутри панели закрывает меню и возвращает фокус на триггер (без preventDefault) — панель теперь в конце body"
  - "DeviceContextMenu и CartridgeContextMenu вне границ плана: уже портальные, свои триггер и позиционирование"
requirements-completed: [GRP-03, GRP-04]
completed: 2026-10-05
---

# Phase 41 Plan 27: портал ActionMenu по умолчанию и гейт Summary

Панель ActionMenu всегда выводится в body через `use:portal` с `position: fixed`, проп `portal` удалён, а возврат непортального режима ловит гейт `check-action-menu-portal.mjs` в `pnpm lint`.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Гейт check-action-menu-portal.mjs с --selftest | de707f48 |
| 2 | Портал по умолчанию, удаление пропа у 4 вызовов, видимость триггера в деревьях, регистрация гейта | c93fcd68 |
| 3 | Сборка, красный сценарий правила F на реальных файлах, живая проверка | правок исходников нет (только прогоны) |

## Что сделано

- `ActionMenu.svelte`: одна панель `role="menu"` с `use:portal` + `use:actionMenuPortalPosition`; `onDown` безусловно проверяет `panelEl.contains`; ветка `Tab` закрывает меню с возвратом фокуса; стили `.action-menu-panel` стали `position: fixed` (+ max-width/max-height/overflow), блок `--portal` и `.action-menu { position: relative }` удалены; добавлен комментарий компонента.
- Убран атрибут `portal` у 4 вызовов (GroupContentsTable x2, GroupPrintersList, NumberTemplateField). Остальные 9 вызовов (включая источник жалобы GroupTypePropertiesTable:572) получили портал дефолтом без правок.
- Деревья групп и мест: `.row-actions` получили `&:has(:global([aria-expanded='true'])) { opacity: 1 }`, чтобы триггер не пропадал при открытом меню. Роль-гейты вокруг `<ActionMenu>` не тронуты.
- Гейт: правила A (одна портальная панель), B (нет usePortal/пропа), C (fixed, нет absolute), D (нет `portal` на вызовах, многострочные теги), E (минимум 13 вызовов — защита от вакуумности), F (`:has(:global(...))`, отдельное сообщение про мёртвую форму без `:global`). Комментарии всех трёх видов затираются с сохранением длины.

## Проверки

**Красный прогон гейта на дереве до задачи 2 (exit 1, 12 нарушений):** A — 2 элемента role="menu" в ActionMenu; B — `usePortal`, проп `portal`, `{#if usePortal}`; C — `position: absolute`, нет `position: fixed`; D — GroupContentsTable:459 и :487, GroupPrintersList:206, NumberTemplateField:702; F — GroupTreeNode:340 и PlaceTreeNode:317. Правило E молчало: 13 вызовов найдено.

**Зелёное после:** `node scripts/check-action-menu-portal.mjs --selftest` — 10 фикстур (позитив + M1-M9) совпали; гейт на репозитории exit 0; `pnpm --dir ui svelte-check` — 0 ERRORS, 68 WARNINGS (базовая линия), `css_unused_selector` в GroupTreeNode/PlaceTreeNode нет; `pnpm --dir ui lint` exit 0; `pnpm --dir ui build` exit 0; prettier и `check-privacy.mjs` PASS.

**Мутации на реальных файлах (якорь проверен на единственность, откат `git checkout`, после — гейт exit 0):**
- m1 (удалена строка `use:portal` в ActionMenu): exit 1
- m2 (возвращена `portal?: boolean;` в Props): exit 1
- m3 (`portal={false}` на вызове в GroupTypePropertiesTable): exit 1
- f1 (в PlaceTreeNode `&:has(:global(...))` заменён нерелевантным селектором — по эффекту равно удалению правила): exit 1, правило F
- f2 (в GroupTreeNode рабочая форма заменена мёртвой `:has([aria-expanded='true'])`): exit 1, в сообщении «мёртвая» и `:global`

**Правило дошло до выдачи:** `grep -ohE "row-actions[^{]*:has\(" ui/dist/assets/*.css | wc -l` = 2 (по одному на дерево).

## Живая проверка: НЕ ПРОВЕРЕНО

Приложение не запускалось (ни `cargo tauri dev`, ни LAN-браузер). Компиляционные гейты и гейт плана вёрстку, скролл, фокус и z-порядок не видят. Семь пунктов проверки (таблица «Свойства» АРМ, деревья «Группы»/«Места», меню в модалке «Новое устройство» с Esc/Tab, «Импорт и экспорт», шаблоны номеров, витрина, «Состав»/«Подключённые принтеры»/«Вставить номер по шаблону») перенесены в `41-HUMAN-UAT.md` как открытые, result: pending. Их нужно отдать пользователю ДО верификации; сводная проверка — в плане 41-34.

## Deviations from Plan

Отклонений правил 1-4 нет. Мелочи: f1 выполнен заменой селектора, а не удалением строки (эффект тот же); гейт строже плана в одном месте — в правиле E и selftest число вызовов вынесено в параметр (13 для репозитория, 2 для фикстуры).

## Known Stubs

Нет.

## Threat Flags

Нет новой поверхности: роль-гейты `{#if}` вокруг `<ActionMenu>` на месте (T-41-27-01), Tab/Esc/`[data-tr-portal]` сохраняют фокус-ловушку Modal (T-41-27-02, требует живой проверки), регрессию закрывает гейт (T-41-27-03).

## Self-Check: PASSED

- ui/scripts/check-action-menu-portal.mjs, ActionMenu.svelte — найдены; коммиты de707f48 и c93fcd68 найдены в git log.
