---
phase: 43-reliable-event-delivery
plan: 07
subsystem: ui-resync
tags: [websocket, resync, registry-gate, svelte, chat]
requires: [43-05]
provides:
  - "onWsResync во всех 13 подписчиках onWsEvent (12 с перечитыванием, chatNotifications освобождён)"
  - "check-ws-resync-registry.mjs подключён в pnpm lint и зелёный"
  - "кейсы U8 (unreadStore) и N10 (chatNotifications): ресинк тихий"
affects: [43-08]
key-files:
  modified:
    - ui/src/features/requests/RequestDetail.svelte
    - ui/src/features/requests/RequestsPage.svelte
    - ui/src/features/requests/chat/RequestChat.svelte
    - ui/src/features/requests/chat/unreadStore.svelte.ts
    - ui/src/features/settings/OrgSettings.svelte
    - ui/src/lib/components/NumberTemplateField.svelte
    - packages/attachments/src/attachmentEvents.svelte.ts
    - packages/attachments/src/AttachmentsBlock.svelte
    - ui/scripts/check-ws-resync-registry.mjs
    - ui/scripts/check-unread-store-behavior.mjs
    - ui/scripts/check-chat-notify-behavior.mjs
    - ui/scripts/check-chat-ui.mjs
    - ui/package.json
    - ui/scripts/fixtures/gate-coverage.baseline.json
key-decisions:
  - "D-08: chatNotifications.svelte.ts освобождён записью в EXEMPTIONS (держит только карту тостов, перечитывать нечего; счётчики перечитывает unreadStore)"
  - "D-09: ресинк unreadStore только вызывает refreshUnread, без pushToast"
  - "D-10: RequestChat на ресинк зовёт load(false); черновик не трогается. OrgSettings перечитывает только шаблоны, не поля организации"
requirements-completed: [DLV-01]
completed: 2026-11-27
---

# Phase 43 Plan 07: ресинк в оставшихся подписчиках и подключение реестра в lint Summary

Все слушатели WS-событий перечитывают данные после обрыва или Lagged; ресинк чата тихий (без тостов и ОС-уведомлений) и не трогает ввод; реестровый гейт живёт в `pnpm lint`.

## Задачи

1. Заявки и чат (RequestDetail, RequestsPage, RequestChat, unreadStore; chatNotifications в EXEMPTIONS), стабы `onWsResync` и кейсы U8/N10 — `a66d135d`
2. Настройки номеров и вложения (OrgSettings, NumberTemplateField, attachmentEvents, AttachmentsBlock) — `1dfeab58`
3. Гейт реестра в lint, ключ perGate в baseline, правка check-chat-ui — `440dab1e`

## Проверки

- `check-unread-store-behavior` и `check-chat-notify-behavior`: selftest PASS (по 9 мутантов, новые «ресинк не зарегистрирован» [U8] и «ресинк показывает тост» [N10] пойманы), боевой OK.
- `check-ws-resync-registry` selftest OK, боевой OK.
- Мутация на настоящем исходнике: из `AttachmentsBlock.svelte` убран вызов onWsResync (и импорт) — гейт exit 1 с «packages/attachments/src/AttachmentsBlock.svelte: подписчиков 1, ресинков 0»; после восстановления из копии зелёный, `git diff` по файлу пуст.
- `pnpm --dir ui lint` — exit 0 (79 гейтов в wiring); `pnpm --dir ui svelte-check` — 0 ошибок, 0 предупреждений; `check-attachments-block` PASS; `check-gate-coverage` зелёный; privacy-гейт PASS на каждом коммите.
- `pnpm --dir ui build` — OK.

## Отклонения от плана

1. [Rule 3 - Blocking] `check-gate-coverage` потребовал ключ `ws-resync-registry` в `perGate` baseline (реестровый гейт использует walk()) — добавлен со значением 303 (факт 311).
2. [Rule 3 - Blocking] `check-chat-ui` C16 требовал дословно `return off;` в эффекте RequestChat; новая форма cleanup `return () => { off(); offResync(); }`. Правило и его мутант обновлены (90 мутантов красные).

## Known Stubs

Нет.

## Self-Check: PASSED

Коммиты a66d135d, 1dfeab58, 440dab1e существуют.
