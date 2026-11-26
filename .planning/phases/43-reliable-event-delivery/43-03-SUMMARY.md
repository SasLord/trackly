---
phase: 43-reliable-event-delivery
plan: 03
subsystem: transport-client
tags: [websocket, resync, reconnect, toast, runtime-gate]
requires: [43-02]
provides:
  - "ws.ts: onWsResync, resyncAll (дебаунс), служебные кадры hello/pong/resync_required"
  - "отложенный (5 с) отменяемый тост обрыва"
  - "переподключение по online/visibilitychange + проба ping"
  - "исполняющий гейт check-ws-resync.mjs (R1-R11, 10 мутантов)"
affects: [43-08, 44, 48]
tech-stack:
  patterns: ["гейт исполняет настоящий ws.ts с фейковыми WebSocket и часами", "таймеры клиента хранятся в модульных переменных и снимаются в activeCleanup"]
key-files:
  created:
    - ui/scripts/check-ws-resync.mjs
  modified:
    - packages/transport/src/ws.ts
    - ui/scripts/check-ws-disconnect-toast.mjs
    - ui/scripts/check-transport-tauri.mjs
    - ui/package.json
key-decisions:
  - "D-08/D-09: resyncAll — первый вызов сразу, повторные в окне 500 мс схлопываются в один хвостовой; ресинк тостов не показывает"
  - "D-14: тост обрыва через RECONNECT_TOAST_DELAY_MS=5000, таймер снимается на onopen и в activeCleanup"
  - "D-15: online/visibilitychange открывают сокет сразу либо шлют ping и рвут сокет при молчании 5 с"
  - "D-03: hello/pong/resync_required разбираются до dispatch; WsEvent и packages-compat.ts не менялись"
requirements-completed: [DLV-01, DLV-04]
completed: 2026-11-26
---

# Phase 43 Plan 03: клиент транспорта — ресинк, отложенный тост, проба Summary

Клиент WS догружает пропущенное после обрыва и Lagged, не мигает тостом на коротких обрывах, мгновенно переподключается при пробуждении и рвёт молчащий сокет; поведение закреплено исполняющим гейтом с мутантами.

## Задачи

1. ws.ts: служебные кадры, onWsResync/resyncAll, отложенный тост, online/visibilitychange, проба ping — `7ddc4cf5`
2. Исполняющий гейт check-ws-resync.mjs (R1-R11, 10 мутантов) — коммит `8cdeffd8`
3. check-ws-disconnect-toast: инвариант W7 и мутанты M16-M19, подключение в `pnpm lint` — `b684919d`

## Проверки

- `node ui/scripts/check-ws-resync.mjs [--selftest]` — OK, 10/10 мутантов обнаружены.
- `node ui/scripts/check-ws-disconnect-toast.mjs [--selftest]` — OK, 20 мутантов (M1-M19 + M10-дубль) пойманы.
- Мутационная проверка на копии реального ws.ts без `clearTimeout(reconnectToastTimer)` в onopen: гейт exit 1 (W7).
- `check-transport-tauri.mjs --selftest` — 10/10; `check-lint-selftest-wiring` (78 гейтов), `check-gate-coverage` — зелёные.
- `pnpm --dir ui lint` — exit 0; `pnpm --dir ui svelte-check` — 0 ошибок, 0 предупреждений.

## Отклонения от плана

1. [Rule 3 - Blocking] `ui/scripts/check-transport-tauri.mjs` (не в files_modified) стал красным: он считал одну подписку Tauri на соединение, а теперь их две (`trackly-event` + `trackly-resync`). Гейт обновлён: ожидания 2 подписки, `hold` удерживает только первый listen, добавлен кейс W5 (trackly-resync -> один ресинк) и мутант.
2. R4 трактован так: два кадра в окне дают 1 вызов сразу и 1 хвостовой после окна (leading + trailing), далее дебаунс не залипает.
3. В `check-ws-resync.mjs` добавлен импорт `setImmediate` из `node:timers` (eslint no-undef).

## Known Stubs

Нет.

## Threat Flags

Нет новых поверхностей. T-43-03-01/02/03 закрыты: дебаунс, JSON.parse в try/catch, снятие слушателей и таймеров (кейсы R8, R11, мутанты M5, M8).

## Self-Check: PASSED

Файлы и коммиты 7ddc4cf5, b684919d существуют; коммит гейта check-ws-resync присутствует в git log.
