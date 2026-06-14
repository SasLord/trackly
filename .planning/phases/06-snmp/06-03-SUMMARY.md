---
phase: 06-snmp
plan: "03"
subsystem: api
tags: [axum, websocket, tauri, snmp, tower-sessions, broadcast, middleware]

requires:
  - phase: 06-02
    provides: PrinterService, RequestService, DTO layer (PrinterDto, RequestDto, WsEvent)

provides:
  - Tauri commands printers_list/get/create/discover/admit/refresh/acknowledge_alert
  - Tauri commands requests_list/get/create/transition/list_categories/counts
  - axum HTTP routes /api/v1/printers_* and /api/v1/requests_*
  - axum WebSocket endpoint /api/v1/ws with session auth gate middleware
  - AppCtx extended with printers, requests, ws_broadcast, poll_task
  - Runtime SNMP mock switch via TRACKLY_SNMP_MOCK env var

affects: [06-04, 06-05, 06-06, 05-ui]

tech-stack:
  added: []
  patterns:
    - "ws_auth_middleware: from_fn middleware checks Session BEFORE WebSocket upgrade (Pitfall 6 mitigation, T-06-09-E ASVS V4)"
    - "Extension<Identity> passed from middleware to ws_handler — clean separation auth/transport"
    - "Lagged(n) broadcast recv error -> continue (not break) — Pitfall 5 mitigation"
    - "WsEvent::is_visible_to(&identity) visibility filter per role (T-06-06-I)"
    - "ctx.ws_broadcast.send(...).ok() — fire-and-forget push on mutation handlers"

key-files:
  created:
    - crates/trackly-app/src/http/printers.rs
    - crates/trackly-app/src/http/requests.rs
    - crates/trackly-app/src/http/ws.rs
  modified:
    - crates/trackly-app/src/http/mod.rs
    - crates/trackly-app/tests/phase06_stubs.rs

key-decisions:
  - "ws_auth_middleware via axum::middleware::from_fn — Option<WebSocketUpgrade> не реализует OptionalFromRequestParts в axum-core 0.5.6, поэтому middleware-паттерн с Extension<Identity> выбран вместо Option-параметра"
  - "axum::routing::any() вместо get() для /api/v1/ws — ws_handler возвращает impl IntoResponse (ответ зависит от наличия WS-заголовков), any() принимает любой HTTP метод"
  - "route_layer(from_fn(ws_auth_middleware)) — middleware применяется только к /api/v1/ws, не ко всем маршрутам"

patterns-established:
  - "Pattern: auth middleware before WS — route_layer(from_fn(ws_auth_middleware)) + Extension<Identity> в handler"
  - "Pattern: mutation handlers send WsEvent через ctx.ws_broadcast.send(...).ok() после успешной операции"

requirements-completed: [PRN-01, PRN-02, PRN-04, PRN-07, REQ-01, REQ-03, REQ-04, REQ-05, REQ-07]

duration: 25min
completed: 2026-06-15
---

# Phase 6 Plan 03: Transport Layer (Tauri + HTTP + WebSocket) Summary

**Tauri commands, axum HTTP handlers и WebSocket endpoint для принтеров и заявок с WS auth middleware (T-06-09-E ASVS V4) и runtime SNMP mock switch через TRACKLY_SNMP_MOCK env**

## Performance

- **Duration:** 25 min
- **Started:** 2026-06-15T00:00:00Z
- **Completed:** 2026-06-15T00:25:00Z
- **Tasks:** 2 (Task 1 — с предыдущей сессии; Task 2 — текущий)
- **Files modified:** 7

## Accomplishments

- Tauri commands printers_*/requests_* скомпилированы и экспортированы в collect_commands!
- axum HTTP handlers для /api/v1/printers_* и /api/v1/requests_* реализуют S-2 паттерн (один DTO, два транспорта)
- WebSocket endpoint /api/v1/ws: auth middleware через from_fn проверяет Session ДО upgrade; Pitfall 5 (Lagged -> continue) и Pitfall 6 (401 до on_upgrade) митигированы
- AppCtx расширен: printers, requests, ws_broadcast (capacity 128), run_poll_task запущен
- test_ws_unauth_401 и test_snmp_mock_switch зелёные (убраны #[ignore] из 06-00)
- CSP обновлён: connect-src 'self' wss: (T-06-12-I)

## Task Commits

1. **Task 1: Tauri commands + AppCtx wire-up** — `d7c4eab` (feat) + `2b3ba83` (test, failing stubs)
2. **Task 2: axum HTTP handlers + WebSocket + build_router** — `3391fd6` (feat)

## Files Created/Modified

- `crates/trackly-app/src/http/printers.rs` — axum handlers list/get/create/discover/refresh/acknowledge_alert + router /api/v1/printers_*
- `crates/trackly-app/src/http/requests.rs` — axum handlers list/get/create/transition/counts/list_categories; WS push на create/transition
- `crates/trackly-app/src/http/ws.rs` — ws_auth_middleware (Session check → Extension<Identity>) + ws_handler (WebSocketUpgrade) + handle_ws_socket loop
- `crates/trackly-app/src/http/mod.rs` — build_router merges printers/requests/ws; pub mod printers/requests/ws; CSP wss:
- `crates/trackly-app/tests/phase06_stubs.rs` — test_ws_unauth_401 и test_snmp_mock_switch реализованы (не #[ignore])

## Decisions Made

**Option<WebSocketUpgrade> не работает в axum 0.8 — выбран middleware паттерн**

`Option<WebSocketUpgrade>` не реализует `OptionalFromRequestParts` в axum-core 0.5.6, поэтому компилятор отклоняет такой хендлер. Решение: `axum::middleware::from_fn(ws_auth_middleware)` через `route_layer` на маршруте `/api/v1/ws`. Middleware проверяет `Session`, при успехе пишет `Identity` в `req.extensions_mut()` и передаёт запрос в `ws_handler` через `Extension<Identity>`. При ошибке возвращает `StatusCode::UNAUTHORIZED` ДО WebSocket upgrade — Pitfall 6 митигирован корректно, тест `test_ws_unauth_401` подтверждает.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Option<WebSocketUpgrade> не компилируется в axum 0.8**
- **Found during:** Task 2 (ws.rs реализация)
- **Issue:** `Option<WebSocketUpgrade>` не реализует `OptionalFromRequestParts` — план предписывал использовать Option для обхода 426-ответа, но в axum-core 0.5.6 этот путь закрыт
- **Fix:** Заменён на middleware паттерн: `ws_auth_middleware` (from_fn) → `Extension<Identity>` → `ws_handler` принимает `WebSocketUpgrade` напрямую; router использует `any()` вместо `get()` и `route_layer(from_fn(ws_auth_middleware))`
- **Files modified:** `crates/trackly-app/src/http/ws.rs`
- **Verification:** `cargo check -p trackly-app` зелёный; `test_ws_unauth_401` зелёный — GET без сессии → 401
- **Committed in:** 3391fd6

---

**Total deviations:** 1 auto-fixed (Rule 1 — bug fix)
**Impact on plan:** Fix корректен с точки зрения безопасности — 401 до WS upgrade гарантирован. Семантика теста не изменилась.

## Issues Encountered

None — после переключения на middleware паттерн всё скомпилировалось и тесты прошли.

## User Setup Required

None — конфигурация не требуется.

## Next Phase Readiness

- Transport layer полностью готов: Tauri + HTTP + WebSocket для принтеров и заявок
- AppCtx расширен — готов к использованию в Phase 06-04 (SNMP poll task)
- WS broadcast готов — Phase 06-05 (UI) может подписываться на события
- test_ws_unauth_401 и test_snmp_mock_switch зелёные — security gate пройден

---
*Phase: 06-snmp*
*Completed: 2026-06-15*
