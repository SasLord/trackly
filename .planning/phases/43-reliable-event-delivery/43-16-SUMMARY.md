---
phase: 43-reliable-event-delivery
plan: 16
subsystem: ui-chat
tags: [gap-closure, CR-02, WR-03, DLV-03]
requires: []
provides:
  - "pendingStore: discardUnheldDrafts, returnToDraft, сохранение записи при отказе на повторе"
  - "draftPolicy.restoreText"
affects: [43-21]
key-files:
  created:
    - ui/src/features/requests/chat/draftPolicy.ts
  modified:
    - ui/src/features/requests/chat/pendingStore.svelte.ts
    - ui/src/features/requests/chat/ChatComposer.svelte
    - ui/src/features/requests/chat/ChatPendingMessage.svelte
    - ui/src/features/requests/chat/RequestChat.svelte
    - ui/scripts/check-chat-pending-behavior.mjs
    - ui/scripts/check-chat-ui.mjs
key-decisions:
  - "Серверный протокол не менялся: серверная очистка черновиков пропускается, если их держит запись очереди"
requirements-completed: [DLV-03]
completed: 2026-11-27
---

# Phase 43 Plan 16: CR-02 и WR-03, текст не теряется при повторе Summary

Отказ соседней отправки больше не стирает черновики вложений неотправленного сообщения, отказ на повторе не удаляет запись из очереди, а набранный за время автоповторов текст не затирается.

## Что сделано
- **pendingStore:** `dispatch` получил признак повтора; при `retryMessage` доменный отказ и не-сетевая ошибка оставляют запись `failed` (первичная отправка снимает запись как прежде). Добавлены `discardUnheldDrafts(requestId)` (пропускает серверную очистку, если очередь держит черновики этой заявки) и `returnToDraft(clientMsgId)`.
- **draftPolicy.ts:** чистая `restoreText(sent, current)`, склейка через перевод строки.
- **ChatComposer:** три вызова `chat.discardDrafts` заменены на `discardUnheldDrafts`; текст и чипы фиксируются до отправки, поле очищается сразу после постановки в очередь, `clearDraft()` в `send()` убран; при error/rejected текст возвращается через `restoreText`.
- **ChatPendingMessage / RequestChat:** кнопка «В поле ввода» (`onToDraft`); при наличии вложений в ошибки добавляется подсказка приложить файлы заново.
- **Гейты:** `check-chat-pending-behavior.mjs` получил кейсы P9-P14 и структурные C1-C4 (мутанты применяются и к ним), 14 мутантов красные. `check-chat-ui.mjs` обновлён под новую форму `send()` (C3 принимает `discardUnheldDrafts`, якоря мутантов).

## RED (коммит 393a6e5b)
До правки pendingStore боевой прогон красный: P9, P10, P11, P12, C1-C4 (P13 зелёный, т.к. draftPolicy создан в той же задаче; P14 зелёный по замыслу).

## Верификация
`check-chat-pending-behavior` (боевой и `--selftest`), `check-chat-ui` (selftest, 94 мутанта), `check-chat-effects`, `check-chat-notify-behavior`, `svelte-check` (0 ошибок), eslint и prettier по затронутым файлам чистые.

## Принятые последствия
- Осиротевшие черновики текущей отправки остаются на сервере, пока очередь держит черновики заявки, до серверной чистки устаревших (D-14); привязать их к чужому сообщению нельзя.
- Файлы неотправленного сообщения при «В поле ввода» не возвращаются в композер (нет File-объектов), пользователю показывается подсказка.

## Deviations from Plan
**1. [Rule 3] Обновлён check-chat-ui.mjs.** Его правила C3/C17 и якоря мутантов были привязаны к прежней форме `send()` (`discardDrafts(`, `await submitMessage(...)`); без обновления гейт краснел. Файла нет в списке плана.

## Deferred Issues
- `pnpm --dir ui lint` красный на уже существующей ошибке `'Response' is not defined` в `packages/transport/src/client.ts:59` (изменён планом 43-15, вне области этого плана). Нужна проверка в 43-21.
- Живая проверка поведения в приложении (композер, пузырь) отнесена к плану 43-21.

## Self-Check: PASSED
Коммиты 393a6e5b, 3de606d6, de810f65 существуют; draftPolicy.ts на месте.
