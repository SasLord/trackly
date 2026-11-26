---
phase: 42-link-monorepo-foundation
plan: 12
subsystem: kit-stores-utils
tags: [stores, runes, harness, gates, LNK-01]
requires: [42-11]
provides:
  - packages/kit: сторы theme, toast, userScoped и утилиты portal, dropdownAnchor, date, formatFileSize, pluralize, saveFile, hashId, calendarIcon (+ .d.ts)
  - _rune-harness.mjs резолвит @trackly/<пакет>/... и исполняет файлы packages/*/src
key-files:
  modified:
    - packages/kit/package.json
    - ui/scripts/_rune-harness.mjs
    - ui/scripts/check-{user-scoped-reset,ws-disconnect-toast,date-picker-layout,place-tree-invalidation,workspace-boundaries,chat-notify-behavior,unread-store-behavior,open-external}.mjs
    - ui/scripts/fixtures/{move-map.json,gate-coverage.baseline.json}
metrics:
  tasks: 2
  completed: 2026-11-26
---

# Phase 42 Plan 12: сторы и утилиты в packages/kit Summary

Три стора и девять утилит переехали в `packages/kit` кодмодом (12 файлов, 101 правка импортов, без прокладок); поведенческие гейты исполняют настоящий код из пакета, мутации краснеют. Коммиты: e859016c, b8ba130c.

## Классификация

`--classify` дал «чисто» для всех 12 файлов; переехали все. `placePath.ts` и `reorder.ts` остались в `ui/` (домен). `saveFile.ts` тянет `@tauri-apps/plugin-dialog/-fs` динамически: файл внесён в `TAURI_IMPORT_ALLOWLIST` с обоснованием; зависимости уже объявлены в `packages/kit/package.json`. Для `calendarIcon.js` добавлена явная запись `exports` (`./utils/calendarIcon.js`, types -> `.d.ts`), так как wildcard `*.ts` для `.js` не подходит.

## Стенд _rune-harness

Добавлена ветка резолвера `@trackly/<пакет>/<подпуть>` -> `packages/<пакет>/src/<подпуть>`, условие `startsWith(cfg.src)` заменено на `inRoots` (ui/src или `packages/<пакет>/src`); экспортированы `PKGS`, `pkgUrl`. Три поведенческих гейта: `--selftest` и прогон зелёные (8/8/5 мутантов).

Доказательство, что стенд грузит файлы пакета (ручные мутации реальных исходников kit с уникальным якорем, затем возврат):
- `userScoped.svelte.ts`: `resetUserScoped` не вызывает сбросы -> chat-notify-behavior и unread-store-behavior красные;
- `toast.svelte.ts`: `removeToast` -> `return` -> chat-notify-behavior красный;
- `toast.svelte.ts`: `pushToast` не добавляет элемент -> open-external красный;
- (мутация дедупликации sticky-тоста ни одним из трёх гейтов не ловится: они её не проверяют, это не вакуумность стенда; её ловит check-ws-disconnect-toast E-серия).
После возврата все три гейта зелёные, `git diff` по kit пуст.

## Перенацеленные гейты

- check-user-scoped-reset: `REGISTRY` и ключи `EXEMPT` (theme, toast) = `../../packages/kit/src/stores/...` (walk() включает пакеты); selftest зелёный.
- check-ws-disconnect-toast: путь стора и W5-цепочки `import('@trackly/kit/stores/toast.svelte')`; 16 мутантов ловятся.
- check-date-picker-layout: путь `calendarIcon.js`, эталон `CLEAN` и мутант «нет импорта» на `@trackly/kit/utils/calendarIcon.js`.
- check-place-tree-invalidation: мутант 4j опирался на `utils/pluralize.ts`; перенацелен на реальный `utils/placePath.ts` (selftest 67 кейсов).

## Сниженные счётчики

- `place-tree-invalidation` pinned (`ui/src`) 304 -> 292: ушли 12 файлов generic-сторов/утилит; гейт ищет write-site'ы доменных API (devices.delete и т.п.), к ним они отношения не имеют, а пакетные файлы им не сканируются осознанно.

Больше ни один счётчик не менялся.

## Проверки

`pnpm --dir ui lint` (74 гейта) exit 0, `svelte-check` 406 файлов 0/0, `vite build` зелёный, privacy-гейт pre-commit PASS.

## Отклонения

**[Rule 1] prettier:** `check-date-picker-layout.mjs` после замены строки не проходил prettier --check; отформатирован.

Иначе план выполнен как написан. Заглушек и новых поверхностей угроз нет.

## Self-Check: PASSED
