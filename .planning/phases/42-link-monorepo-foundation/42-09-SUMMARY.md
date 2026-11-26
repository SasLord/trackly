---
phase: 42-link-monorepo-foundation
plan: 09
subsystem: monorepo
tags: [codemod, source-package, gates, LNK-01]
requires: [42-07, 42-08]
provides:
  - кодмод переезда ui/scripts/_move-codemod.mjs (--classify / --apply)
  - Button и Spinner в packages/kit, механизм импортов (wildcard-exports) задокументирован
  - eslint/prettier/svelte-check покрывают packages/ и link-ui/
  - гейт check-workspace-boundaries ([B0]-[B5], selftest)
key-files:
  created:
    - ui/scripts/_move-codemod.mjs
    - ui/scripts/_eslint-workspaces.mjs
    - ui/scripts/check-workspace-boundaries.mjs
    - packages/README.md
    - packages/{svelte.config.js,eslint.config.mjs,prettier.config.mjs}
    - link-ui/{svelte.config.js,eslint.config.mjs,prettier.config.mjs}
  modified:
    - packages/kit/package.json
    - ui/package.json
    - ui/tsconfig.json
    - ui/scripts/check-act-delete-confirm.mjs
    - ui/scripts/check-act-delete-choice.mjs
    - ui/scripts/check-gate-coverage.mjs
    - ui/scripts/fixtures/{move-map.json,gate-coverage.baseline.json}
metrics:
  tasks: 3
  completed: 2026-11-26
---

# Phase 42 Plan 09: спайк переезда Button и гейты границ Summary

Button доказал, что source-package собирается и типизируется в Trackly без алиасов; кодмод и гейт границ готовы к массовому переносу (планы 11-17). Коммиты: cad43908, 11c5fd24, 01938bd3.

## Принятый механизм импортов (A1)

Вариант (a): wildcard-`exports` в `packages/kit/package.json` (`./components/*`, `./utils/*` -> `.ts`, `./stores/*` -> `.ts`), без алиасов в `vite.config.ts` и без `paths` в `tsconfig.json`. Vite компилирует `.svelte` из workspace-симлинка как исходники. Спецификатор: `@trackly/kit/components/Button.svelte`. Записано в `packages/README.md`.

## Кодмод

`--classify` отказывает на файлах с импортами bindings/auth/features/`$lib/api`; `--apply from=to` делает `git mv`, переписывает `$lib` и относительные спецификаторы во всех ui/src, packages, link-ui, дописывает move-map, прокладок не оставляет. На спайке: 2 файла перенесено, 106 файлов поправлено.

## Покрытие гейтов кода (A4) и мутации

Результат: `svelte-check` читает `tsconfig.include` с `../packages/*/src`, но каждому каталогу нужен свой `svelte.config.js` (иначе ошибка fallback `svelte-preprocess`); eslint 9 не видит файлы вне каталога конфига, поэтому `_eslint-workspaces.mjs` запускает его изнутри `packages/` и `link-ui/`; prettier требует конфиг рядом (`prettier.config.mjs` с плагином из `ui/node_modules`). Мутации в `packages/kit/src/components/Button.svelte`: TS-ошибка — svelte-check 2 ошибки; сдвиг отступа — prettier красный; неиспользуемая `var` — eslint красный. После возврата всё зелёное.

## Гейт границ

`check-workspace-boundaries` — 13 случаев selftest (мутанты [B0]-[B5], контроль, два «проходит»), подключён парой перед `check-lint-selftest-wiring` (в нём 74 гейта, было 73). `TAURI_IMPORT_ALLOWLIST` пуст.

## Отклонения

**1. [Rule 3 - Blocking] Spinner.svelte переехал вместе с Button.** Button импортирует `./Spinner.svelte`; без Spinner в пакете импорт ломался бы (пакет не вправе тянуть `ui/`). Кодмод перенёс оба (move-map: 2 записи, Button — одна).

**2. [Rule 3] `place-tree-invalidation` pinned-счётчик 309 -> 307** в `gate-coverage.baseline.json`: гейт сканирует все файлы `ui/src`, два файла переехали ([C7] требовал осознанного обновления). Переехавшие файлы write-site'ов не содержат.

**3. [Rule 3] `check-act-delete-choice` (E)** проверял регэкспом только `$lib/components/X.svelte`; теперь принимает `@trackly/kit/components/X.svelte` (план говорил об `act-delete-confirm` — там менялись литералы мутантов; фикстура `device-list-mode/DevicesPage.pre-fix.svelte.txt` тоже обновлена).

**4. [Rule 3] `check-gate-coverage` LOCAL_WALK_EXEMPT** расширен на `check-workspace-boundaries.mjs` с обоснованием в комментарии: гейт обходит `packages/*` намеренно (границы, копия дерева в selftest); вакуум закрывает [B0].

## Известное

- `ui/scripts/verify-value-map.mjs` (one-shot историческая сверка) всё ещё называет `ui/src/lib/components/Button.svelte`: не гейт, не в lint, не правился.
- Ограничение D-20 из плана 07 остаётся: фильтр внутри гейта после `walk()` не ловится.
- Прогонялись: `pnpm --dir ui lint`, `svelte-check` (406 файлов, 0/0), `vite build`; rune-харнесса для Button в плане нет.

## Self-Check: PASSED
