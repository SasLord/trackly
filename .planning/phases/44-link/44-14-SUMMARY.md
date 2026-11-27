---
phase: 44-link
plan: 14
subsystem: server-http
tags: [link, revocation, ws, guard]
requires: [44-11]
provides: [link_guard, ws teardown 4401, user-block hooks]
affects: [44-20]
requirements: [LNK-07]
key-files:
  created:
    - crates/trackly-app/tests/link_revoke.rs
    - crates/trackly-app/tests/link_revoked_device.rs
  modified:
    - crates/trackly-app/src/http/mod.rs
    - crates/trackly-app/src/http/ws.rs
    - crates/trackly-app/src/http/link.rs
    - crates/trackly-app/src/services/auth.rs
    - crates/trackly-app/src/services/request_service.rs
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/tests/suites/server.rs
decisions:
  - "link_guard пропускает публичные маршруты /api/v1/link_*: устаревшая cookie не должна ломать повторный вход (иначе flush + 401 на самом входе)."
  - "Хук отзыва получает сервис Link через builder with_link (AuthService, RequestService), без ссылки на AppCtx; сбой хука логируется и не откатывает операцию, guard независимо проверяет users.is_active/deleted_at_utc."
  - "Админ-идентификатор хука = caller.user_id.unwrap_or(0) (колонка revoked_by_user_id без FK)."
metrics:
  tasks: 2
  completed: 2026-11-28
---

# Phase 44 Plan 14: отзыв Link на лету Summary

Guard Link-сессий на каждом запросе (401 `LINK_REVOKED` + сброс сессии), закрытие WS кадром 4401 без нового запроса и хуки блокировки/удаления пользователя, отзывающие его входы.

## Что сделано

- **Task 1** (`a94723dc`): `link_guard` в `http/mod.rs` (внутри session-слоя, снаружи `refresh_session_role`); `ws_handler` регистрирует соединение Link-устройства в `LinkHub` и `ws_loop` закрывает его `CloseFrame` с `REVOKED_CLOSE_CODE` (4401).
- **Task 2** (`497c0a4b`): `AuthService::with_link` / `RequestService::with_link`, подключены в `context.rs`; хуки `revoke_user_logins(.., ban_devices=false)` после коммита.

## Инвентарь мест записи is_active / deleted_at_utc (grep от репозитория)

| Место | Вердикт |
|-------|---------|
| auth.rs `update_user` (`is_active = COALESCE(?6,..)`) | хук добавлен (при `is_active=Some(false)`) |
| auth.rs `delete_user` (`deleted_at_utc = ?1`) | хук добавлен |
| request_service.rs `transition` Reject (~стр. 1122, `deleted_at_utc`, `is_active = 0`) | хук добавлен (ветка register + активный пользователь) |
| auth.rs 664/718, request_service.rs 978/985, auth.rs 2278/2518 (`is_active = 1`, активация/восстановление) | не применимо: активация, не блокировка |
| auth.rs INSERT INTO users (create/AD-регистрация/system) | не применимо: новые строки, входов нет |
| прочие `deleted_at_utc` (requests, cartridges, devices, acts, attachments, places) | не применимо: не таблица users |

Независимая страховка (T-44-52): guard проверяет `users.is_active` и `deleted_at_utc` сам (тест `inactive_user_link_session_gets_link_revoked` пишет в БД напрямую, минуя хуки).

## Мутационные якоря для плана 20 (уникальные строки)

- `http/mod.rs`, `link_guard`: единственное выражение `if !valid {` после `ctx.link.is_login_valid(login_id)`. Снятие -> красные `revoked_login_gets_401_on_next_request_and_session_is_flushed`, `inactive_user_link_session_gets_link_revoked`.
- `http/ws.rs`: единственная ветка `() = &mut link_cancelled => {` в `select!` `ws_loop`. Снятие -> красные `ws_of_revoked_device_closes_with_4401_without_new_request`, `ws_of_another_device_survives_foreign_revoke`.
- `services/auth.rs`: `if deactivates_user {` (хук блокировки) и вызов `self.revoke_link_logins(id, caller).await;` в `delete_user`. Снятие -> красные `blocking_user_revokes_logins_but_not_device_and_closes_ws`, `deleting_user_revokes_logins`.
- `services/request_service.rs`: `if ad_subtype_was_register && user_is_active {`. Снятие -> красный `rejecting_accepted_ad_registration_revokes_logins`.

Мутации в этом плане не выполнялись (по плану: план 20).

## Гейты

| Гейт | Результат |
|------|-----------|
| `cargo test -p trackly-app --test app_server -- link_revoke` (включает link_revoked_device) | 13/13 |
| `cargo test -p trackly-app --test app_server -- ws` | 22/22 |
| `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_server` | 124/124 |
| `... --test app_auth` | 138/138 |
| `... --test app_gates` (целиком) | 291 passed, 0 failed, 1 ignored |
| `cargo clippy -p trackly-app --all-targets -- -D warnings` | чисто |
| `cargo fmt --check -p trackly-app` | чисто |

## Deviations from Plan

**1. [Rule 1 - Bug] Публичные маршруты Link не проходят через guard.** Без пропуска `/api/v1/link_*` вход с устаревшей cookie давал flush и 401 на самом запросе входа. Проверка по `strip_prefix("/api/v1/")` + `starts_with("link_")`, а не литералом `"/api/v1/link_"`: сканер `http_routes_covered_by_role_matrix` принимает такой литерал за маршрут (поймано прогоном app_auth).

**2. [Rule 3 - Blocking] `#[allow(clippy::too_many_arguments)]` на `handle_ws_socket`:** добавлен восьмой аргумент (токен отзыва), каждый аргумент независим.

**3. Новый writer в services/ не добавлялся:** реестр `entities_support/registry.rs` не менялся; app_gates зелёный.

**4. SSO-тест запрета устройства** идёт через заголовок Negotiate с мусорным билетом: устройство проверяется до билета, ответ 403 `LINK_DEVICE_BANNED`.

## Авто-одобренные чекпоинты

Нет (план автономный). Живая проверка не выполнялась; для человека: на стенде войти в Link под сотрудником, в админке заблокировать этого пользователя и убедиться, что клиент мгновенно выброшен на экран входа без ожидания следующего действия; затем отозвать вход другого клиента и увидеть то же.

## Known Stubs

Нет.

## Threat Flags

Новой сетевой поверхности нет (guard и хуки сужают доступ).

## Self-Check: PASSED

Файлы link_revoke.rs, link_revoked_device.rs существуют; коммиты a94723dc и 497c0a4b в истории.
