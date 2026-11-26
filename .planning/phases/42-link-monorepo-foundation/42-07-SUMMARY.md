---
phase: 42-link-monorepo-foundation
plan: 07
subsystem: ui-gates
tags: [gates, coverage, monorepo, LNK-01]
requires: [42-05, 42-06]
provides:
  - perGate baseline (18 гейтов-обходчиков) и pinned-входы 7 гейтов вне walk()
  - правила [C4]-[C7] в check-gate-coverage
affects: [планы 09-17]
key-files:
  modified:
    - ui/scripts/snapshot-gate-coverage.mjs
    - ui/scripts/fixtures/gate-coverage.baseline.json
    - ui/scripts/check-gate-coverage.mjs
metrics:
  tasks: 2
  completed: 2026-11-26
---

# Phase 42 Plan 07: per-gate покрытие Summary

До переезда заморожены счётчики 18 гейтов на walk() (`perGate`) и входы 7 гейтов, не использующих walk() (`pinned`). `check-gate-coverage` сверяет их правилами [C4]-[C7]. Коммиты: e7932133, 3d1dd569.

## Что сделано

- `snapshot-gate-coverage.mjs --per-gate`: не трогает `files`; отказывает, если move-map не пуст или `ui/src` грязный; гейт с кодом != 0 или count 0 — ошибка. 18 гейтов, `files` = 309 как раньше.
- [C4] ключи perGate == check-*.mjs с `_roots.mjs` (кроме самого гейта); [C5] запуск каждого с GATE_COVERAGE_REPORT, count > 0 и >= baseline; [C6] запрет локального readdirSync по src вне `_roots.mjs`; [C7] pinned.
- 7 гейтов вне walk() (attachments-block, chat-effects, chat-ui, lint-selftest-wiring, html-commands, repair-modal-focus, place-tree-invalidation) покрыты [C7]: именованные файлы обязаны существовать по тому же пути, число файлов в каталоге (маска как у гейта) не должно падать; place-tree-invalidation — все файлы ui/src любых расширений (309). Гейт, у которого вход переехал, краснеет; план-переносчик обязан в том же коммите перенацелить гейт и осознанно обновить pinned.
- Прогон гейта 2.3 с (18 обходчиков последовательно), PASS: обходчиков 18, файлов 4181.

## Доказательство несущей способности (мутации на реальном дереве)

- Гейт-обходчик (check-tokens) с локальным readdirSync и без `_roots.mjs`: [C4], [C5], [C6] красные.
- Убран `linkify.ts` (chat-ui): [C7] chat-ui, [C1], [C3], [C5] красные. Убран `Repair*Modal.svelte`: [C7] для repair-modal-focus и chat-effects.
- Selftest: 17 случаев (мутанты [C4]-[C7], в т.ч. [C6] на временном каталоге). `pnpm --dir ui lint` зелёный.

## Известное ограничение

Мутация «гейт сам отфильтровал один файл ПОСЛЕ walk()» (printer-identity, `.filter` после обхода) НЕ ловится: счётчик фиксируется в walk(), до фильтра гейта. Per-gate счётчик гарантирует, что обход видит файлы, но не что гейт применил к ним правила. Это граница D-20, не закрыта.

## Отклонения

Нет. Добавление `pinned` и [C7] — расширение сверх текста плана по прямому указанию оркестратора (7 гейтов вне walk()).

## Self-Check: PASSED
