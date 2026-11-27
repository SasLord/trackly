---
phase: 43-reliable-event-delivery
plan: 18
subsystem: ui
tags: [svelte, runes, ws-resync, place-tree, gate]
requires: []
provides:
  - "resetSeq / resetSeqNow() и notifyEntitiesChanged(ids, { all })"
  - "PlaceTree.resetStatsCache() и сброс кэша по resetSeq внутри эффекта загрузки"
  - "INV-12 в check-place-tree-invalidation.mjs + 4 мутанта"
affects: [43-21]
tech-stack:
  added: []
  patterns: ["сброс кэша внутри существующего эффекта загрузки, без нового $effect"]
key-files:
  created: []
  modified:
    - ui/src/lib/stores/placeContentEvents.svelte.ts
    - ui/src/features/places/PlaceTree.svelte
    - ui/src/features/layout/Layout.svelte
    - ui/scripts/check-place-tree-invalidation.mjs
key-decisions:
  - "Полный сброс — по флагу opts.all; обычное событие с placeIds остаётся точечным (D-10)"
  - "Новых $effect нет (INV-1/INV-11); seenResetSeq — обычная переменная, не $state"
requirements-completed: [DLV-01]
duration: 25min
completed: 2026-11-27
---

# Phase 43 Plan 18: ресинк сбрасывает кэш счётчиков мест Summary

Ресинк после обрыва или resync_required в Admin/Manager теперь не только перечитывает дерево мест, но и обнуляет statsCache через resetSeq в сторе и resetStatsCache() в эффекте загрузки PlaceTree (WR-05, D-08).

## Выполнено

1. **Task 1 (RED), cc795c77.** В гейт добавлен INV-12 (стор, PlaceTree, Layout) и 4 мутанта. Боевой прогон до правок: 6 нарушений с меткой INV-12 (RED подтверждён).
2. **Task 2 (GREEN), 7d9440e9.** Стор получил resetSeq, resetSeqNow(), флаг `all`; PlaceTree — resetStatsCache() (его же зовёт refreshTree) и проверку `seenResetSeq !== resetSeqNow()` внутри эффекта загрузки; Layout.handleResync зовёт `notifyEntitiesChanged([], { all: true })`.

## Верификация (реально прогнано)

- `check-place-tree-invalidation.mjs`: боевой PASS 0 нарушений; `--selftest` PASS 71 кейс.
- `check-user-scoped-reset.mjs`: OK.
- `pnpm --dir ui lint`: exit 0 (включая prettier).
- `pnpm --dir ui svelte-check`: 0 errors, 0 warnings.
- Число `$effect` в PlaceTree.svelte: 6 до правки, 6 после (новых нет).

## Отклонения

- Первая версия мутанта «сброс не вызывается» (`&& false`) не была убита структурным гейтом; заменена на удаление вызова resetStatsCache() из эффекта, после чего selftest зелёный (Rule 1, правка собственного мутанта).

## Не проверено

Живая проверка «счётчики после ресинка» в WKWebView — пункт плана 43-21; синтетические гейты рантайм рун не доказывают.

## Self-Check: PASSED

Коммиты cc795c77 и 7d9440e9 существуют; изменённые файлы на месте.
