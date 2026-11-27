---
phase: 44-link
plan: 16
subsystem: server-admin-api
tags: [link, admin, invites, revoke, ws, specta]
requires: [44-04, 44-06, 44-11, 44-14]
provides: [link_invites_*, link_logins_*, link_devices_*, users_link_status, users_revoke_link, WsEvent::LinkChanged]
affects: [44-18, 44-19, 44-20]
requirements: [LNK-07]
key-files:
  created:
    - crates/trackly-app/src/dto/link.rs
    - crates/trackly-app/src/tauri_cmds/link.rs
    - crates/trackly-app/tests/link_admin.rs
  modified:
    - crates/trackly-app/src/services/link_service.rs
    - crates/trackly-app/src/http/link.rs
    - crates/trackly-app/src/http/mod.rs
    - crates/trackly-app/src/dto/printer.rs
    - crates/trackly-app/src/dto/mod.rs
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/src/tauri_cmds/mod.rs
    - crates/trackly-app/tests/suites/server.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs
    - ui/src/bindings-phase6.ts
    - packages/transport/src/ws.ts
decisions:
  - "Общие build_link_* лежат в tauri_cmds/link.rs и принимают &Identity: Tauri-команда резолвит identity десктопа, HTTP-обработчик берёт session_identity; Action::ManageUsers проверяется в единой require_admin внутри каждого хелпера."
  - "Код выпускается по TlsInfo запущенного сервера (SPKI и порт только оттуда); host из запроса, пустой -> tls::display_host(bind-host); сервер не запущен -> 409 Conflict."
  - "link_guard теперь пропускает только пять публичных имён (link::PUBLIC_ROUTES), а не любой префикс link_: админ-маршруты идут под guard как обычные, отозванная Link-сессия админа не получит доступа."
  - "Допуск устройства — Вариант А (D-13): link_devices_allow снимает запрет, нового кода не нужно."
  - "users_revoke_link отзывает все входы пользователя И банит их устройства (ban_devices=true), в отличие от блокировки пользователя (44-14), где устройства остаются."
metrics:
  tasks: 3
  completed: 2026-11-28
---

# Phase 44 Plan 16: админ-API Link Summary

Девять парных маршрутов и Tauri-команд для раздела «Link» (коды-приглашения, активные входы, запрещённые компьютеры, отзыв/допуск, входы пользователя), закрытых серверной проверкой роли Admin, плюс событие `WsEvent::LinkChanged` для живого обновления.

## Что сделано

- **Task 1** (`721a00be`): `dto/link.rs` (7 DTO, camelCase, specta Type); в `LinkService` добавлены `list_login_dtos`, `list_banned_dtos`, `list_invite_dtos` (JOIN users/link_devices; использованные/истёкшие/отозванные старше 30 дней скрыты), `user_link_status`, `ttl_hours_to_secs` (1/24/168, иначе Validation). Списки не содержат кода и хэшей.
- **Task 2** (`35160a73`): `tauri_cmds/link.rs` (9 `build_link_*`/`build_users_*` + 9 Tauri-команд), `http/link.rs::admin_router()`, регистрация в `specta_export.rs`. `main.rs` не менялся: `invoke_handler` берётся из того же `builder()`.
- **Task 3** (`721a00be` + `35160a73`): `WsEvent::LinkChanged {}` (`{"type":"link_changed"}`, виден только Admin); `LinkService::with_ws_tx` + приватный `broadcast_link_changed(op)` вызывается после успешной записи из `create_invite`, `revoke_invite`, `enroll`, `allow_device`, `record_login`, `logout`, `revoke_login`, `revoke_user_logins`; девять строк в `ROUTE_MATRIX_EXEMPT`.

Маршруты: `link_invites_create|list|revoke`, `link_logins_list|revoke`, `link_devices_list|allow`, `users_link_status`, `users_revoke_link`. Тела HTTP: `{ req }`, `{ id }`, `{ loginId }`, `{ deviceId }`, `{ userId }`.

## Гейты

| Гейт | Результат |
|------|-----------|
| `cargo test -p trackly-app --test app_server -- link_admin` | 18/18 |
| `cargo test -p trackly-app --test app_server` (целиком) | 142/142 |
| `cargo test -p trackly-app --test app_gates` (parity, entities_changed_gate, suite_wiring) | 291 passed, 0 failed, 1 ignored |
| `cargo test -p trackly-app --test app_auth` (role_endpoint_matrix) | 138/138 |
| `cargo test -p trackly-app --lib dto::printer` | 22/22 (включая link_changed) |
| `cargo test -p trackly-app --test export_bindings` | зелёный; в bindings.ts `linkInvitesCreate`, `linkLoginsRevoke` и остальные команды |
| `cargo clippy -p trackly-app --all-targets -- -D warnings` | чисто |
| `cargo fmt --check -p trackly-app` | чисто |
| `pnpm svelte-check` (ui) | 0 errors, 0 warnings |
| `check-ws-resync(-registry)`, prettier по затронутым TS | OK |
| `pnpm lint` (ui) | красный из-за ранее существовавшей ошибки `queueMicrotask is not defined` в `link-ui/src/components/LoginScreen.svelte:84` (не из этого плана, см. Deferred) |

## Deviations from Plan

**1. [Rule 1 - Bug] Широкий пропуск guard.** В 44-14 `link_guard` пропускал любой `/api/v1/link_*`. Новые админ-маршруты тоже начинаются с `link_`, поэтому Link-сессия администратора с отозванным входом обходила бы guard. Введён `link::PUBLIC_ROUTES` (пять имён), guard сверяется по нему. Коммит `35160a73`.

**2. Test 5 задачи 3 (bindings.ts содержит `link_changed`) неприменим в исходной форме:** `WsEvent` не попадает в сгенерированный `bindings.ts` (на него не ссылается ни одна команда). Вариант добавлен в рукописные зеркала `ui/src/bindings-phase6.ts` и `packages/transport/src/ws.ts` (они обязаны быть взаимно присваиваемы; svelte-check зелёный), JSON-форма закреплена юнит-тестом в `dto/printer.rs`.

**3. Test 3 задачи 3 (WS-клиенты по ролям) проверен на уровне предиката**, а не живым WebSocket: `link_changed_is_visible_only_to_admin_subscribers` берёт настоящее событие из `ctx.ws_broadcast` и применяет `is_visible_to` — тот же предикат, которым `ws_loop` фильтрует каждый кадр; десктопный мост (`desktop_event_visible`) использует его же. Живой WS-клиент по ролям — на плане 20.

**4. Тесты DTO/сервиса (Test 1-6 задачи 1) лежат в `tests/link_admin.rs`,** а не в `tests/link_service.rs`: так весь файл плана один и уже подключён в suite `app_server`. `verify` из плана (`-- link_service`) этих тестов не запустил бы, использовать `-- link_admin`.

**5. Коммиты:** задачи 1 и 3 (сервис, DTO, WsEvent) слиты в первый коммит, потому что сервис ссылается на `WsEvent::LinkChanged` и по отдельности они не собираются; маршруты, команды, тесты и реестры — второй коммит.

**6. Реестр `http_route_parity` не менялся:** все 9 маршрутов имеют одноимённые Tauri-команды и ссылки `build_*` из `http/link.rs`, вердикты не нужны (гейт зелёный). `role_endpoint_matrix` потребовал девять записей в `ROUTE_MATRIX_EXEMPT` (имя не упоминается в самой матрице); роль-тест вынесен в `tests/link_admin.rs`.

## Deferred Issues (вне области плана)

- `pnpm lint`: `link-ui/src/components/LoginScreen.svelte:84 'queueMicrotask' is not defined` (eslint no-undef, нужен `queueMicrotask` в globals конфига link-ui или `globalThis.queueMicrotask`). Файл из ранних планов UI Link, не трогался. Закрыть до плана 20 (сплошной прогон).
- В `LinkService::list_invites`/`list_active_logins` (сырые строки) оставлены как есть; админка использует новые `*_dtos`.

## Мутационные якоря для плана 20

- `tauri_cmds/link.rs`: `require_admin(caller)?` в каждом из 9 `build_*` (тест `build_helpers_gate_every_route_with_manage_users` считает ровно 9 вхождений); снятие любого краснит `all_nine_routes_401_without_session_and_403_for_non_admin`.
- `http/mod.rs`: `.is_some_and(|name| link::PUBLIC_ROUTES.contains(&name))` в `link_guard`.
- `services/link_service.rs`: `self.broadcast_link_changed("revoke_login");` и соседние — красит `link_changed_sent_once_per_mutation_and_not_on_rollback`.
- `tauri_cmds/link.rs`: `link_proto::parse_colon_hex(&info.spki_fingerprint_hex)` — SPKI из запущенного сервера (тест `invite_is_issued_with_running_server_spki_and_requested_host`).

## Авто-одобренные чекпоинты

Нет (план полностью автономный). Живая проверка не выполнялась; для человека: на стенде (режим сервера включён) войти администратором, выпустить код в разделе «Link» (UI появится в плане 18/19) или `POST /api/v1/link_invites_create` с телом `{"req":{"host":"","ttlHours":24}}`; убедиться, что код показывается один раз и не виден в `link_invites_list`; отозвать вход и увидеть мгновенный выход клиента; нажать «Разрешить» для запрещённого компьютера и убедиться, что он входит без нового кода.

## Known Stubs

Нет.

## Threat Flags

Новая поверхность — ровно девять маршрутов из threat_model (T-44-60..64), все с митигациями: ManageUsers в каждом хелпере, код только в ответе выпуска, `LinkChanged` только Admin, реестры и тест на каждый маршрут, SPKI только из `ServerHandle.tls`.

## Self-Check: PASSED

Файлы dto/link.rs, tauri_cmds/link.rs, tests/link_admin.rs существуют; коммиты 721a00be и 35160a73 в истории.
