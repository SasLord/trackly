---
phase: 41-groups-model-and-editor
plan: 28
subsystem: ui
tags: [copywriting, gap-closure, groups]
requires: []
provides:
  - нейтральный хинт неизменяемости типа группы для всех ролей
affects: [GroupTypePanel, 41-UI-SPEC, 41-VALIDATION]
tech-stack:
  added: []
  patterns: []
key-files:
  modified:
    - ui/src/features/groups/GroupTypePanel.svelte
    - .planning/phases/41-groups-model-and-editor/41-UI-SPEC.md
    - .planning/phases/41-groups-model-and-editor/41-VALIDATION.md
    - .planning/phases/41-groups-model-and-editor/deferred-items.md
key-decisions:
  - "Хинт панели типа — «Код и поведение типа изменить нельзя.» для admin и manager (решение пользователя, W-F03)"
requirements-completed: [GRP-01, GRP-09]
metrics:
  duration: 1 мин
  completed: 2026-10-05
---

# Phase 41 Plan 28: нейтральный хинт панели типа Summary

Хинт панели типа группы заменён на «Код и поведение типа изменить нельзя.» для всех ролей; UI-SPEC §9.1, таблица копирайта, VALIDATION и deferred-items синхронизированы. Закрыт GAP-6 (W-F03).

## Выполнено

| Задача | Коммит | Что сделано |
|--------|--------|-------------|
| 1 | 267249b8 | Одна строка в `GroupTypePanel.svelte`; условий по роли нет |
| 2 | 6d844fb2 | UI-SPEC §9.1 и таблица копирайта; VALIDATION: строка отклонений закрыта, из H20-3 убрана просьба решить копирайт; из deferred-items удалён пункт про `GroupTypePanel` (пункт `MovementTimeline` оставлен) |

## Проверки

- `grep -rn "Название и набор" ui/src` пусто; в UI-SPEC / VALIDATION / deferred-items старой фразы нет.
- Новая фраза в UI-SPEC встречается 2 раза (§9.1 и таблица).
- `pnpm --dir ui svelte-check`: 0 ошибок, 68 предупреждений (прежняя база); `pnpm --dir ui lint`: код 0 (включая gate ActionMenu из 41-27); prettier --check чисто.
- `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256`: 0 нарушений.
- Исторические протоколы (VERIFICATION, REVIEW*, 41-20-PLAN, 41-26-SUMMARY) не менялись.

## Отклонения от плана

В ходе работы две правки пришлось слегка переформулировать ради критериев приёмки: абзац §9.1 перенесён так, чтобы фраза стояла в одной строке (критерий считает строки), а в строке отклонений VALIDATION описание старого текста перефразировано без дословной цитаты. Иных отклонений нет.

## Ожидает ручной проверки (НЕ ПРОВЕРЕНО в запущенном приложении)

Живая проверка из критериев задачи 1 не выполнялась: приложение не запускалось.
- Под admin и отдельной сессией под manager (десктоп и LAN-браузер после `pnpm --dir ui build`) открыть тип «АРМ».
- Ожидание: над таблицей «Свойства» у обеих ролей одна и та же строка «Код и поведение типа изменить нельзя.», без слова «можно».
Этот пункт совпадает с H20-3 в 41-VALIDATION.md.

## Known Stubs

Нет.

## Self-Check: PASSED

Коммиты 267249b8 и 6d844fb2 существуют; изменённые файлы на месте.
