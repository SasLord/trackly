---
phase: 43-reliable-event-delivery
plan: 10
subsystem: ui-chat
tags: [idempotency, retry, chat, pending-store, gates]
requires:
  - phase: 43-06
    provides: "серверная идемпотентность chat_send по client_msg_id"
  - phase: 43-09
    provides: "sendWithRetry, newClientId, isNetworkFailure"
provides:
  - "pendingStore.svelte.ts: submitMessage, retryMessage, pendingFor, pending"
  - "ChatPendingMessage.svelte: пузырь «Не отправлено» / «Повторить»"
  - "check-chat-pending-behavior.mjs (исполняющий, 6 мутантов) в pnpm lint; правило C17 в check-chat-ui"
affects: [43-12]
key-files:
  created:
    - ui/src/features/requests/chat/pendingStore.svelte.ts
    - ui/src/features/requests/chat/sendOutcomeText.ts
    - ui/src/features/requests/chat/ChatPendingMessage.svelte
    - ui/scripts/check-chat-pending-behavior.mjs
  modified:
    - ui/src/features/requests/chat/api.ts
    - ui/src/features/requests/chat/ChatComposer.svelte
    - ui/src/features/requests/chat/RequestChat.svelte
    - ui/scripts/check-chat-ui.mjs
    - ui/package.json
key-decisions:
  - "D-12: статус failed только при сетевом сбое после 4 попыток; доменные отказы и AppError снимают запись"
  - "D-12: очередь только в памяти, registerUserScopedReset очищает при смене пользователя"
  - "D-11: все попытки и ручной повтор идут с одним clientMsgId (замыкание в dispatch)"
requirements-completed: [DLV-03]
completed: 2026-11-27
---

# Phase 43 Plan 10: неотправленные сообщения чата Summary

Сообщение чата при сбое сети не теряется и не дублируется: тихие автоповторы с одним clientMsgId, затем пузырь «Не отправлено» с кнопкой «Повторить» (тот же id); доменные отказы идут прежними строками ошибок композера.

## Задачи

1. Стор неотправленных, `chat.send(..., clientMsgId)`, `rejectText`, исполняющий гейт — `15ab0f23`
2. Пузырь, композер через `submitMessage`, вывод очереди в ленте, правило C17 — `621a6bd6`

## Проверки

- `check-chat-pending-behavior --selftest`: 6 мутантов пойманы (P2, P2, P3, P5, P8, P7), боевой OK. P2 сверяет, что множество clientMsgId среди 4 вызовов ровно одно.
- `check-chat-ui --selftest`: 94 мутанта красные (4 новых для C17), боевой PASS.
- `pnpm lint` exit 0 (82 гейта в wiring, включая check-gate-coverage, contrast, tokens), `svelte-check` 0/0, `pnpm build` OK, privacy PASS.

## Отклонения

1. Мутант M6 сначала вешал стенд (второй retry ждал неснятый gate): кейс P7 переписан на одновременный запуск двух retry и освобождение gate после.
2. Правило C3 в check-chat-ui переведено с `chat.send(` на `submitMessage(` (прямой вызов теперь запрещён правилом C17); перестановочный мутант C3 обновлён.
3. Пустое состояние ленты «Сообщений пока нет» скрывается и при наличии неотправленных (малая правка, не из плана).

## Ограничения

- При обычной быстрой отправке в ленте на мгновение может мелькнуть «Отправляется…» до прихода настоящего сообщения; поведение не проверялось в живом приложении (WKWebView/WebView2), только компиляторные и исполняющие гейты. Живая проверка — план 12.
- Автоповторы длятся до ~7 с, пока текст ещё в композере; после исчерпания композер очищается и сообщение остаётся в ленте.

## Known Stubs

Нет.

## Self-Check: PASSED

Коммиты 15ab0f23, 621a6bd6 существуют.
