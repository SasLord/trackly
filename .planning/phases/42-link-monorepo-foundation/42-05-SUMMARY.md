---
phase: 42-link-monorepo-foundation
plan: 05
subsystem: ui-gates
tags: [gates, walker, monorepo, LNK-01]
requires: [42-01]
provides:
  - 10 гейтов группы A обходят дерево через walk() из _roots.mjs
affects: [план 07, планы 09-17]
key-files:
  modified:
    - ui/scripts/check-act-delete-confirm.mjs
    - ui/scripts/check-act-number-no-regex-split.mjs
    - ui/scripts/check-action-menu-portal.mjs
    - ui/scripts/check-date-picker-layout.mjs
    - ui/scripts/check-device-list-mode.mjs
    - ui/scripts/check-device-row-menu.mjs
    - ui/scripts/check-device-status-labels.mjs
    - ui/scripts/check-device-type-labels.mjs
    - ui/scripts/check-employee-device-menu.mjs
    - ui/scripts/check-focus-outline.mjs
metrics:
  tasks: 2
  completed: 2026-11-26
---

# Phase 42 Plan 05: гейты группы A на общий walk() Summary

10 из 13 гейтов переведены на `walk()` из `_roots.mjs`: после переезда они увидят `packages/*/src` и `link-ui/src` без правок. Правила и мутанты selftest не менялись. Коммит: см. `git log` (refactor(42-05)).

## Переведены (10)

act-delete-confirm (subdirs features/acts), action-menu-portal, act-number-no-regex-split, date-picker-layout, device-status-labels, device-row-menu, device-list-mode, device-type-labels, employee-device-menu (subdirs features/employee), focus-outline.

Для `--src=<root>` и временных каталогов selftest передаётся `roots: [<root>/src]`, для реального `UI_ROOT` — полный обход всех корней. Локальные walk/collectFiles удалены; в PASS-строки act-number и focus-outline добавлено число файлов.

## Исключения (3) — для плана 07

check-attachments-block, check-chat-effects, check-chat-ui не обходят `src`: они читают именованные файлы и нерекурсивно листают конкретные каталоги (features/attachments, features/chat, features/requests) с жёсткими путями и ключами `dir + имя`. Перевод на walk() изменил бы семантику; после переезда их пути/константы придётся править точечно (план 07 / планы переезда).

## Проверка

- `--selftest` и обычный прогон всех 10 зелёные; `pnpm --dir ui lint` зелёный (73 гейта).
- GATE_COVERAGE_REPORT: count > 0 у всех (21, 223, 303, 223, 303, 303, 9, 303, 303, 226). Единственная строка count=0 у act-delete-confirm — кейс selftest с пустым каталогом.
- Мутация «пустой src»: `--src=<пустой src>` даёт exit 1 у device-type-labels, device-row-menu, device-status-labels, action-menu-portal (пустой скан не проходит зелёным).
- svelte-check, clippy, fmt не затрагиваются (только .mjs).

## Отклонения от плана

[Rule 3] Три гейта из 13 не переведены (см. исключения): план допускал это при несовпадении предпосылки. Приватных данных нет.

## Известные заглушки

Нет.

## Self-Check: PASSED
