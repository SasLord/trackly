---
phase: 42-link-monorepo-foundation
plan: 06
subsystem: ui-gates
tags: [gates, walker, monorepo, LNK-01]
requires: [42-01]
provides:
  - 8 гейтов группы B обходят дерево через walk() из _roots.mjs
affects: [план 07, планы 09-17]
key-files:
  modified:
    - ui/scripts/check-group-vocabulary.mjs
    - ui/scripts/check-history-timeline-refresh.mjs
    - ui/scripts/check-path-settings-form.mjs
    - ui/scripts/check-place-path-short.mjs
    - ui/scripts/check-printer-identity.mjs
    - ui/scripts/check-store-response-shape.mjs
    - ui/scripts/check-user-scoped-reset.mjs
    - ui/scripts/check-tokens.mjs
metrics:
  tasks: 2
  completed: 2026-11-26
---

# Phase 42 Plan 06: гейты группы B на общий walk() Summary

8 из 12 гейтов переведены на `walk()`; локальные обходчики удалены. Без `--src` гейты видят все корни (`ui/src`, `packages/*/src`, `link-ui/src`), с `--src=<root>` передаётся `roots: [<root>/src]`. Правила и мутанты selftest не менялись. check-tokens без `--src` сканирует `.svelte` и `.scss` всех корней (kit остаётся под Rule 1-4). Коммит: refactor(42-06).

## Переведены (8)

group-vocabulary, history-timeline-refresh, path-settings-form, place-path-short, printer-identity, store-response-shape, user-scoped-reset, tokens.

## Исключения (4) — для плана 07

- check-lint-selftest-wiring: листает `scripts/`, не `src`.
- check-html-commands: читает `.rs`-каталог и точечный `client.ts`; путь обновит план 15.
- check-repair-modal-focus: нерекурсивный листинг одного каталога по маске `Repair*Modal.svelte`.
- check-place-tree-invalidation: инвентарь INV-10 обходит ВСЕ файлы `src` любых расширений, включая `bindings.ts` (на нём держатся вердикты NON_CODE_VERDICTS); `walk()` пропускает `bindings.ts` и фильтрует по расширениям — перевод изменил бы семантику. Нужен либо режим «все файлы» в `walk()`, либо точечная правка после переезда.

## Отклонения от плана

- [Rule 3] store-response-shape переведён БЕЗ `subdirs`: гейт обходит весь `src` (`.ts`) и фильтрует сам; `subdirs` сузили бы охват. printer-identity и store-response-shape и так пропускают `bindings*`, расхождения нет.

## Проверка

- `--selftest` и прогон всех 8 зелёные; `pnpm --dir ui lint` и `prettier --check scripts` зелёные.
- GATE_COVERAGE_REPORT, count: group-vocabulary 303, history-timeline-refresh 223, path-settings-form 223, place-path-short 303, printer-identity 303, store-response-shape 80, user-scoped-reset 303, tokens 226 (все > 0).
- Мутация «пустой src»: history-timeline-refresh, path-settings-form, place-path-short, printer-identity, store-response-shape дают exit 1. group-vocabulary, user-scoped-reset (фиксированный путь), tokens на пустом каталоге давали 0 и до правки (проверено на версии из HEAD) — предсуществующее свойство; для tokens мутант с hex в `<style>` временного src даёт exit 1.
- Исправление по ходу: в printer-identity и store-response-shape не хватало импорта `UI_ROOT` (поймано selftest/lint).

## Известные заглушки

Нет. Приватных данных нет.

## Self-Check: PASSED
