---
phase: 43-reliable-event-delivery
plan: 19
subsystem: chat
tags: [gap-closure, chat, client_msg_id, WR-04]
requires: [43-13]
provides:
  - "ChatMessageDto.clientMsgId в ленте чата заявки, только автору (WR-04, сервер)"
affects: [43-20]
key-files:
  modified:
    - crates/trackly-infra/src/repos/chat_sqlite.rs
    - crates/trackly-app/src/dto/chat.rs
    - crates/trackly-app/src/services/chat_service.rs
    - crates/trackly-app/tests/chat_feed.rs
    - crates/trackly-app/tests/chat_idempotent_send.rs
key-decisions:
  - "clientMsgId заполняется только при is_own; админ и другие участники получают null (T-43-19-01)"
requirements-completed: [DLV-03]
completed: 2026-11-27
---

# Phase 43 Plan 19: clientMsgId в ленте чата заявки Summary

Лента чата заявки отдаёт автору клиентский id его сообщений (clientMsgId), остальным участникам и администратору null; replay chat_send возвращает то же сообщение с id автора.

## Что сделано
- ChatMessageRow и MSG_SELECT читают `m.client_msg_id` (параметризованный SELECT), message_dto отдаёт значение только при is_own.
- IN-03: у replay_of убран неиспользуемый параметр repo. IN-02: комментарий defence-in-depth у ветки «страховка UNIQUE» в chat_service::send.
- Биндинги регенерированы (`export_bindings`), `clientMsgId: string | null` присутствует в ChatMessageDto (bindings.ts не отслеживается git).

## RED/GREEN
- RED: тесты `feed_exposes_client_msg_id_to_author_only` и `replay_returns_client_msg_id` не компилировались (поля client_msg_id нет у ChatMessageDto). GREEN: оба проходят после правки.

## Проверка
- app_chat (chat_feed, chat_idempotent_send, chat_send, chat_http): зелёные; infra_tests chat_repo 12/12; export_bindings; app_gates chat_changed_gate 14/14; app_server chat_ws_fanout 2/2.
- cargo fmt --check и clippy -p trackly-app -p trackly-infra --tests -D warnings: чисто.

## Задача 2
Файлы chat_http, chat_changed_gate, role_endpoint_matrix менять не пришлось: связанные тесты зелёные без правок.

## Deviations from Plan
None - plan executed exactly as written.

## Self-Check: PASSED
Коммит b37df56a существует; файлы изменены.
