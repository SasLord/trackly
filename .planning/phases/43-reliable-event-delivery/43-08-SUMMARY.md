---
phase: 43-reliable-event-delivery
plan: 08
subsystem: api
tags: [im, websocket, http, resync, parity-gate]
requires:
  - phase: 43-02
    provides: "WS-кадры hello/ping/resync_required"
  - phase: 43-04
    provides: "ImHub, ImService send/sync"
  - phase: 43-06
    provides: "идемпотентность чата и заявок"
provides:
  - "POST /api/v1/im_send и /api/v1/im_sync (авторизация по членству)"
  - "ветка ImHub в WS-цикле: кадр im_changed {conversationId, rev}"
  - "Lagged канала IM -> resync_required"
affects: [44, 47, 48]
tech-stack:
  added: []
  patterns: ["IM-сигнал мимо is_visible_to: получатели определены сервисом", "отрицательные проверки сокета через контрольный сигнал вместо sleep"]
key-files:
  created:
    - crates/trackly-app/src/http/im.rs
    - crates/trackly-app/tests/im_http.rs
    - crates/trackly-app/tests/im_ws_fanout.rs
  modified:
    - crates/trackly-app/src/http/mod.rs
    - crates/trackly-app/src/http/ws.rs
    - crates/trackly-app/tests/http_route_parity.rs
    - crates/trackly-app/tests/im_support/mod.rs
    - crates/trackly-app/tests/suites/server.rs
    - crates/trackly-app/tests/entities_support/registry.rs
    - crates/trackly-infra/tests/chat_migration.rs
key-decisions:
  - "D-02: наружу только im_send/im_sync; в ROUTE_WITHOUT_CMD с обоснованием, Tauri-команд IM нет"
  - "D-04: im_changed из ImHub отдельной веткой select!; пользователь без user_id (системный администратор десктопа) подписки IM не имеет"
  - "D-07: Lagged IM даёт resync_required, Closed канала IM только снимает подписку, соединение остаётся"
requirements-completed: [DLV-01, DLV-02, DLV-03, DLV-04]
completed: 2026-11-27
---

# Phase 43 Plan 08: транспорт IM на сервере Summary

Маршруты im_send и im_sync, ветка ImHub в WebSocket-цикле и resync_required при переполнении канала IM; DLV-02 доказан на настоящем сокете с мутациями.

## Задачи

1. Маршруты im_send/im_sync, записи паритета, HTTP-тесты — `77da8626`
2. Ветка ImHub в WS, Lagged IM, тест im_ws_fanout — `e74276dd`
3. Прогон целевых наборов; починка двух регрессий фазы — `beb9f4e6`, `b4d3d36c`

## Проверки (по одной, с TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1)

- `app_server` 83/83; `app_chat` 48/48; `app_gates` 291/291 (1 ignored); `app_auth role_endpoint_matrix` 35/35; `infra_tests` 303/303.
- `cargo clippy -p trackly-app -p trackly-infra -p trackly-core --all-targets -- -D warnings` и `cargo fmt --check` — чисто.
- Tauri-команд IM нет (`tauri_cmds` без im_*).

## Мутационные проверки (исходник восстановлен после каждой)

- handler_send с фиксированной идентичностью вместо session_identity: краснеют `im_send_session_membership_and_replay` (401) и sync-тест.
- ImHub::publish во все каналы без учёта получателей: краснеют `im_signal_reaches_members_only_without_body` и `removed_member_gets_no_further_signals`.
- Ветка Lagged IM заменена на `continue`: краснеет `im_lagged_yields_resync_required_and_keeps_connection`.
- Подмешан текст в кадр сигнала: краснеет проверка «нет текста в кадре».

## Отклонения от плана

1. [Rule 3 - блокер] `im_service.rs` (План 04) не был внесён в `OUT_OF_SCOPE_WRITERS` — красный `entities_changed_gate::writer_owning_services_outside_scope_are_listed`. Внесён с обоснованием.
2. [Rule 1 - регрессия] `infra_tests::chat_migration::v054_*` не знал о колонке `client_msg_id` из V059 (План 01) — ожидаемый список колонок дополнен.
3. Хелпер `cookie` добавлен в общий `im_support`; ветвь ws_loop вынесена в отдельную функцию, чтобы `prune()` вызывался после выхода при любой причине.
4. role_endpoint_matrix и fixture employee-allowed-commands правок не потребовали (маршруты вне матрицы зелёные).

## Known Stubs

Нет.

## Threat Flags

Новых поверхностей сверх threat_model плана нет; T-43-08-01..05 закрыты тестами.

## Self-Check: PASSED

Файлы из key-files существуют; коммиты 77da8626, e74276dd, beb9f4e6, b4d3d36c в `git log`.
