---
phase: 44-link
plan: 06
subsystem: server-core
tags: [link, invites, devices, revoke, ws-hub]
requires: [44-02, 44-03, 44-04]
provides: [LinkService, LinkHub, AppCtx.link, AppCtx.link_hub]
affects: [44-11, 44-14, 44-16]
key-files:
  created:
    - crates/trackly-app/src/server/link_hub.rs
    - crates/trackly-app/src/services/link_service.rs
    - crates/trackly-app/tests/link_service.rs
  modified:
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/src/server/mod.rs
    - crates/trackly-app/src/services/mod.rs
    - crates/trackly-app/src/http/health.rs
    - crates/trackly-app/src/tauri_cmds/health.rs
    - crates/trackly-app/tests/suites/server.rs
    - "тесты с литералом AppCtx: printers_admit, report_movements, report_requests, reports_period_required, specta_roundtrip, templates_status"
decisions:
  - "Ошибки сервиса (EnrollError/DeviceError/ResumeError) переводятся в AppError явным into_app(), а не через From: impl From<_> for AppError ломает вывод типов в чужом коде (request_service)."
  - "verify_device обновляет last_seen/last_ip не чаще раза в 60 с (или при смене IP), чтобы не грузить единственного писателя на каждый запрос."
  - "logout тоже рвёт WS устройства через хаб: вход недействителен, соединение не должно жить."
  - "Секрет устройства/токен хэшируются как SHA-256 от base64url-строки; сравнение — ct_eq без раннего выхода."
metrics:
  tasks: 2
  completed: 2026-11-28
---

# Phase 44 Plan 06: серверное ядро Link Summary

LinkService (коды-приглашения, устройства, входы, отзыв/запрет) и LinkHub (разрыв WS на лету), подключённые к AppCtx; погашение кода атомарно в одной транзакции единственного писателя.

## Что сделано

- **Task 1** (`e0394682`): `LinkHub` — реестр `CancellationToken` по устройству/пользователю, poison-safe, prune; 5 unit-тестов.
- **Task 2** (`74aa8705`): `LinkService` с методами `create_invite`, `list_invites`, `revoke_invite`, `enroll`, `verify_device(_from)`, `allow_device`, `list_banned_devices`, `record_login`, `resume_by_token`, `logout`, `revoke_login` (запрет устройства), `revoke_user_logins(ban_devices)`, `list_active_logins`, `count_active_logins`, `is_login_valid`. Хаб вызывается только после `.await?` записи. 12 интеграционных тестов (suite `app_server`), включая гонку двух погашений.

## Гейты

| Гейт | Результат |
|------|-----------|
| `cargo test -p trackly-app --lib server::link_hub` | 5/5 |
| `cargo test -p trackly-app --test app_server -- link_service` | 12/12 |
| `cargo test -p trackly-app --test app_gates -- suite_wiring` | 5/5 |
| `cargo clippy -p trackly-app --all-targets -- -D warnings` | чисто |
| `cargo fmt --check` | чисто |

## Deviations from Plan

**1. [Rule 3 - Blocking] Литералы `AppCtx` в тестах и health-модулях** — новые поля `link`/`link_hub` сломали сборку 8 мест, где `AppCtx` собирается вручную (`http/health.rs`, `tauri_cmds/health.rs`, 6 тестовых файлов). Добавлены поля. Коммит `74aa8705`.

**2. [Rule 1 - Bug] `From<…Error> for AppError`** сломал вывод типов в `request_service.rs`; заменено на `into_app()`.

**3. Расхождение в команде проверки:** `cargo test --test suite_wiring` из плана не существует — `suite_wiring` живёт внутри бинарника `app_gates`; запущено `--test app_gates -- suite_wiring`.

**4.** Task 1 закоммичен без правок `context.rs` (они зависят от сервиса из Task 2) — коммит Task 1 собирается сам по себе, подключение к AppCtx — в коммите Task 2.

## Авто-одобренные чекпоинты

Нет (план полностью автономный).

## Known Stubs

Нет.

## Threat Flags

Нет новых поверхностей: маршруты появятся в планах 11/14/16.

## Self-Check: PASSED

Файлы link_hub.rs, link_service.rs, tests/link_service.rs существуют; коммиты e0394682 и 74aa8705 в истории.
