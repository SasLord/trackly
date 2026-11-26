---
phase: 42-link-monorepo-foundation
plan: 11
subsystem: design-system
tags: [scss, tokens, vite-plugin, gates, LNK-01]
requires: [42-09, 42-10]
provides:
  - packages/design с токенами, брейкпоинтами, global.scss и theme-init.js
  - Vite-плагин @trackly/design/vite-plugin (theme-init в dev и dist)
key-files:
  created:
    - packages/design/vite-plugin-theme-init.mjs
  modified:
    - packages/design/package.json
    - packages/README.md
    - ui/vite.config.ts
    - ui/src/main.ts
    - ui/eslint.config.js
    - ui/scripts/{check-tokens,check-contrast,check-action-menu-portal,probe-date-picker-chromium,verify-value-map}.mjs
    - ui/scripts/fixtures/{move-map.json,gate-coverage.baseline.json}
metrics:
  tasks: 2
  completed: 2026-11-26
---

# Phase 42 Plan 11: дизайн-система в packages/design Summary

Токены, брейкпоинты, глобальные стили и theme-init переехали в `packages/design`; ui собирается, типизируется и проходит все гейты. Коммит: 67773370.

## Принятая форма @use (спайк A2)

Ни один из запасных вариантов (loadPaths, алиас, относительный путь) не понадобился: Sass внутри Vite резолвит `@use '@trackly/design/breakpoints' as bp;` через `exports` workspace-зависимости. Глобально: `import '@trackly/design/global.scss'` в `main.ts` (`scss.prependData` не вводился, комментарий-запрет сохранён). Подтверждено `vite build` (токены `--tr-*` в CSS) и `svelte-check` (406 файлов, 0/0). Записано в `packages/README.md`.

## theme-init

`vite-plugin-theme-init.mjs` без новых зависимостей: middleware в dev, `emitFile` в `generateBundle`. `ui/dist/theme-init.js` есть, `index.html` ссылается на внешний `/theme-init.js` (CSP не тронут). Каталог `ui/public` опустел и удалён, сборка не падает.

## Гейты

- check-tokens: токены берутся из `packages/design` (явный `--src` сохраняет прежнюю схему для тестовых деревьев); check-contrast, check-action-menu-portal (ключ `GLOBAL_SCSS_FILE` = репозиторный путь), probe-date-picker-chromium, verify-value-map перенацелены. check-focus-outline читает через `walk()`, `packages/design/src` входит в корни автоматически.
- Мутация: переименование токена `--tr-space-2xs` -> check-tokens FAIL (176 нарушений, включая `packages/kit/.../Button.svelte`, то есть пакеты сканируются); возврат -> PASS. Отсутствие файла токенов -> exit 1 у check-tokens и check-contrast (не SKIP).
- `pnpm --dir ui lint` (74 гейта), `svelte-check`, `vite build` зелёные.

## Сниженные счётчики

- `place-tree-invalidation` pinned (`ui/src`) 307 -> 304: три `.scss` (токены, брейкпоинты, global) ушли из `ui/src`; гейт ищет write-site'ы в `.svelte/.ts`, к стилям отношения не имеет.

## Отклонения

**1. [Rule 3] `ui/eslint.config.js`: игнор `**/public/`.** `_eslint-workspaces` линтует `packages/` целиком, а `public/theme-init.js` — браузерный скрипт (прежде исключался паттерном `public/` в ui). Добавлен `**/public/`.

**2. [Rule 3] verify-value-map:** pathspec-исключение переписано на `:(exclude,glob)**/styles/_tokens.scss` (путь вне cwd git не принимает).

## Self-Check: PASSED
