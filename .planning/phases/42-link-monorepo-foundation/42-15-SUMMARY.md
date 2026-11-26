---
phase: 42-link-monorepo-foundation
plan: 15
subsystem: transport
tags: [transport, hooks, gates, LNK-01]
requires: [42-14]
provides:
  - packages/transport/src - client (apiCall, HTML_COMMANDS, setApiHooks), errors, ws, transport.svelte (isTauri)
  - ui/src/packages-compat.ts - проверка взаимной присваиваемости WsEvent bindings <-> пакет
  - ui/scripts/check-transport-hooks.mjs - поведенческий гейт хуков 401/403
key-files:
  created:
    - packages/transport/src/{client,errors,ws,transport.svelte}.ts
    - ui/src/packages-compat.ts
    - ui/scripts/check-transport-hooks.mjs
  modified:
    - ui/src/main.ts
    - ui/scripts/check-{html-commands,ws-disconnect-toast,employee-api,user-scoped-reset,chat-notify-behavior,unread-store-behavior,store-response-shape,workspace-boundaries}.mjs
    - ui/scripts/fixtures/{move-map.json,gate-coverage.baseline.json}
metrics:
  tasks: 2
  completed: 2026-11-26
---

# Phase 42 Plan 15: транспорт в packages/transport Summary

apiCall, ошибки, WebSocket-клиент и признак isTauri переехали в `packages/transport` кодмодом (75 правок импортов, без прокладок). Реакции на 401/403 развязаны с auth/toast: пакет вызывает внедряемые хуки `onUnauthorized`/`onForbidden` (`setApiHooks`), приложение регистрирует их в `ui/src/main.ts` до `mount`. Коммит: 34c94c74.

## Что сделано

- `client.ts`: убраны `authStore` и `pushToast`; без хуков ветка 401/403 только бросает ошибку. `HTML_COMMANDS` остался в `client.ts`.
- `ws.ts`: `WsEvent` объявлен структурно в пакете; `ui/src/packages-compat.ts` проверяет взаимную присваиваемость с bindings-phase6 (мутация `userId: number -> string` дала 9 ошибок svelte-check, возврат -> 0).
- Доменные обёртки `api/{acts,devices,groups,organization,templates,adSso}.ts` остались в ui; barrel `api/index.ts` оставил только `devices`.
- `packages/transport/package.json`: `exports: ./* -> ./src/*.ts`, зависимость `@trackly/kit` (workspace) для тоста обрыва WS.
- Гейт границ: `packages/transport/src/{client,ws}.ts` добавлены в `TAURI_IMPORT_ALLOWLIST` (Tauri-ветка транспорта: invoke и listen).

## Гейты

Обновлены и зелёные (прогон и `--selftest`): html-commands (читает `packages/transport/src/client.ts`; baseline `html-commands` в gate-coverage и snapshot перенацелены в том же коммите), ws-disconnect-toast, employee-api, user-scoped-reset (EXEMPT-ключ ws), chat-notify-behavior и unread-store-behavior (stub `@trackly/transport/ws`), open-external, store-response-shape, place-tree-invalidation, workspace-boundaries, gate-coverage, lint-selftest-wiring.

Мутации реальных исходников (красный, возврат, зелёный):
- удалена `templates_validate_preview` из `HTML_COMMANDS` -> `check-html-commands` [H2];
- `sticky: true -> false` в `packages/transport/src/ws.ts` -> `check-ws-disconnect-toast` W1.

Новый гейт `check-transport-hooks` (подключён в `pnpm lint`, 74 -> baseline `lint-selftest-wiring` count): исполняет настоящий `client.ts`, 5 мутантов красные (401 без хука, 403 не тем хуком, хук на любой ошибке, ошибка глотается без хуков, снята регистрация в main.ts).

Понижение замороженных счётчиков:
- `place-tree-invalidation` ui/src 262 -> 259: четыре файла транспорта ушли из ui/src, плюс один новый `packages-compat.ts`; доменных write-site'ов в них нет, гейт на них не опирался.
- `check-employee-api` MIN_FILES 90 -> 86 (замыкание 92 -> 88): обход идёт только по ui/src, вызовов команд в перенесённых файлах нет (MIN_CALLS не менялся).
- `lint-selftest-wiring` count 73 -> 74 (рост, новый гейт).

Проверки: `pnpm --dir ui lint` 0, `svelte-check` 443 файла 0/0, `vite build` зелёный, privacy-гейт PASS.

## Отклонения от плана

**1. [Отступление] Тост обрыва WS остался в пакете через `@trackly/kit/stores/toast.svelte`.** Истина плана «пакет не импортирует pushToast» выполнена для `client.ts` (auth/тосты 401/403 вынесены хуками), но `ws.ts` по-прежнему лениво импортирует kit-тост: гейт ws-disconnect-toast (W1-W6, 16 мутантов) закреплён на тексте этой логики, а kit не является доменом Trackly (граница B1 зелёная). Вынос в хук потребовал бы переписать гейт; отложено. Гейт границ не запрещает `@trackly/kit/stores/toast` из transport.
**2. [Rule 3]** Кодмод не резолвит новые внутрипакетные импорты до переезда, поэтому `client.ts`/`ws.ts` временно ссылались на старый путь; кодмод записал `./x.ts` с расширением, заменено на `./x`.

## UNVERIFIED

- Tauri-ветка `invoke` в `apiCall` и `listen` в WS: поведенческий гейт покрывает только HTTP-ветку (isTauri читается при загрузке модуля). Живая проверка входа/выхода и 403 на десктопе и в LAN-браузере - стенд D-21 (план 19).

## Self-Check: PASSED
