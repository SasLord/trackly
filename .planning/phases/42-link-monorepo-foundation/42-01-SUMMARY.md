---
phase: 42-link-monorepo-foundation
plan: 01
subsystem: ui-gates
tags: [gates, coverage, monorepo, LNK-01]
requires: []
provides:
  - ui/scripts/_roots.mjs (общий обходчик walk/scanRoots с отчётом GATE_COVERAGE_REPORT)
  - baseline набора файлов фронтенда (309 файлов) до переезда
  - move-map.json (пополняется планами 09-17)
  - гейт check-gate-coverage в pnpm lint
affects: [планы 07, 09-17 фазы 42]
key-files:
  created:
    - ui/scripts/_roots.mjs
    - ui/scripts/snapshot-gate-coverage.mjs
    - ui/scripts/check-gate-coverage.mjs
    - ui/scripts/fixtures/move-map.json
    - ui/scripts/fixtures/gate-coverage.baseline.json
  modified:
    - ui/package.json
decisions:
  - "C3 сравнивает скан с baseline без ui/public (public в скан не входит): 308 против 309 файлов"
metrics:
  tasks: 2
  files: 6
  completed: 2026-11-26
---

# Phase 42 Plan 01: ведомость покрытия гейтов Summary

Baseline из 309 файлов фронтенда снят до переездов, а гейт check-gate-coverage краснеет, если файл baseline пропал или не учтён в move-map (правила C0-C3, без SKIP).

## Задачи

1. Общий обходчик `_roots.mjs`, пустая move-map и baseline: 93bf7dff.
2. Гейт `check-gate-coverage.mjs` с мутационным `--selftest` (6 случаев) и подключением парой перед `check-lint-selftest-wiring`: fc715d17.

## Проверка

`pnpm --dir ui lint` зелёный; wiring-гейт отчитывается о 73 гейтах (было 72). Privacy-гейт зелёный. svelte-check, clippy и fmt план не затрагивает.

## Отклонения от плана

Ручная проверка мутации (переименование реального .svelte) не выполнялась: её покрывает мутационный selftest на виртуальных наборах путей. Больше отклонений нет.

## Известные заглушки

Нет.

## Self-Check: PASSED
