---
phase: 43-reliable-event-delivery
plan: 02
subsystem: websocket
tags: [websocket, heartbeat, resync, axum, tauri-bridge]
requires: []
provides:
  - "WS-протокол служебных кадров: hello, ping/pong, resync_required"
  - "WsTimings + константы PING_INTERVAL/IDLE_TIMEOUT"
  - "desktop_event_bridge::resync_on_lagged и RESYNC_EVENT (trackly-resync)"
affects: [43-03, 43-08, 44]
tech-stack:
  added: []
  patterns: ["служебные кадры не входят в WsEvent", "исходный гейт на ветку main.rs с уникальным якорем"]
key-files:
  created:
    - crates/trackly-app/tests/ws_heartbeat.rs
    - crates/trackly-app/tests/ws_resync_required.rs
    - crates/trackly-app/tests/desktop_resync_bridge.rs
  modified:
    - crates/trackly-app/src/http/ws.rs
    - crates/trackly-app/src/desktop_event_bridge.rs
    - crates/trackly-app/src/main.rs
    - crates/trackly-app/tests/chat_ws_fanout.rs
    - crates/trackly-app/tests/entities_changed_ws.rs
    - crates/trackly-app/tests/ws_close_on_server_stop.rs
    - crates/trackly-app/tests/suites/server.rs
    - crates/trackly-app/tests/suites/chat.rs
key-decisions:
  - "D-13: hello первым кадром (pingIntervalSecs/idleTimeoutSecs в секундах, округление вверх), ping 25 с, idle 60 с — константы"
  - "D-07: Lagged веб-сокета -> {\"type\":\"resync_required\"}; Lagged моста десктопа -> событие trackly-resync; буфер 128 не тронут"
requirements-completed: [DLV-01, DLV-04]
completed: 2026-11-26
---

# Phase 43 Plan 02: протокол WS и ресинк при Lagged Summary

Сервер шлёт hello, пингует, закрывает молчащих и сообщает клиентам о потере событий и в веб-сокете, и в мосте десктопа; формат кадров закреплён тестами на настоящем сокете.

## Задачи

1. hello, ping/pong, idle-закрытие (WsTimings) — `97b74215`
2. Lagged общего канала -> resync_required — `97b74215` (один коммит: обе задачи правят одну функцию ws.rs)
3. Мост десктопа: resync_on_lagged + trackly-resync — `e0baa943`

## Проверки

- `cargo test -p trackly-app --test app_server -- ws_ chat_ws_fanout` — 14/14 (heartbeat 6, resync 1, close_on_server_stop, fanout чата).
- `cargo test -p trackly-app --test app_chat desktop_resync_bridge` — 2/2.
- `cargo test -p trackly-app --test app_gates -- entities_changed_ws` — 1/1 (тест затронут hello).
- `cargo clippy -p trackly-app --all-targets -- -D warnings`, `cargo check --bins`, `cargo fmt --check` — чисто.

## Мутационные проверки

- Ветка idle заменена на `pending()`: `dead_peer_is_closed_after_idle` краснеет (парный `live_peer` остаётся зелёным); восстановлено.
- Отправка resync_required отключена (`if false`): `ws_resync_required` краснеет; восстановлено.
- Вызов resync_on_lagged в main.rs заменён на warn: исходный гейт краснеет; восстановлено.

## Отклонения от плана

1. [Rule 3] Существующие тесты на настоящем хендлере (`chat_ws_fanout`, `entities_changed_ws`, `ws_close_on_server_stop`) обновлены: hello потребляется/пропускается; утверждения не ослаблены. Остальные из перечисленных в плане (ws_broadcast_fanout, ws_http_single_broadcast, ws_upgrade_serve_connection, tls_server_smoke) настоящий хендлер не используют — не менялись.
2. Задачи 1 и 2 в одном коммите (общая функция).

## Known Stubs

Нет. Ветка Lagged канала IM — План 08 (отмечено в докблоке).

## Threat Flags

Нет новых поверхностей; T-43-02-01/02/04 закрыты (idle, SEND_TIMEOUT, разбор только короткого JSON ping).

## Self-Check: PASSED

Файлы и коммиты 97b74215, e0baa943 существуют.
