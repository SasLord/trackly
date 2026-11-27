---
phase: 43-reliable-event-delivery
plan: 20
subsystem: ui-chat
tags: [chat, pending-queue, reconcile, WR-04]
requires: [43-16, 43-19]
provides: [reconcileWithFeed]
affects: [ui/src/features/requests/chat]
key-files:
  modified:
    - ui/src/features/requests/chat/pendingStore.svelte.ts
    - ui/src/features/requests/chat/RequestChat.svelte
    - ui/scripts/check-chat-pending-behavior.mjs
decisions:
  - "Сверка только по clientMsgId из ленты, снимаются лишь записи 'failed' той же заявки"
requirements: [DLV-03]
completed: 2026-11-27
---

# Phase 43 Plan 20: сверка очереди неотправленных с лентой

Клиентская половина WR-04: `reconcileWithFeed(requestId, clientMsgIds)` снимает пузырь «Не отправлено», если сервер уже вернул сообщение с тем же clientMsgId (потерянный ответ), и вызывается из `load()` после `feed = res` на любом пути загрузки.

## Задачи

1. reconcileWithFeed + кейсы P15/P15b/P15c, мутанты (1e0a1c50). RED подтверждён: до реализации гейт падал с «st.reconcileWithFeed is not a function» по P15/P15b/P15c.
2. Вызов из RequestChat.load(), правило C5 и мутант (bfa4e635).

## Проверка

- `check-chat-pending-behavior.mjs` и `--selftest`: OK, 18 мутантов красные (новые: любой статус -> P15b, игнор заявки -> P15, лишнее переприсвоение -> P15c, нет вызова в load -> C5).
- `check-chat-ui.mjs`, `check-chat-effects.mjs`: PASS. `pnpm svelte-check`: 0 ошибок, 0 предупреждений. `pnpm lint` (после prettier): зелёный.
- Число `$effect` в RequestChat.svelte: 5 до и после.
- Если массив не изменился, `pending.items` не переприсваивается.

## Отклонения

Нет. Замечание: коммит 1 содержит правило C5, которое зеленеет только с коммитом 2 (гейт между ними красный по C5).

## Self-Check: PASSED
