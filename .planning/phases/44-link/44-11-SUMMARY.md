---
phase: 44-link
plan: 11
subsystem: server-http
tags: [link, enroll, auth, sso, resume, logout, governor]
requires: [44-06]
provides: [http/link.rs router, sso::negotiate_identity, auth::establish_session]
affects: [44-12, 44-14, 44-16]
requirements: [LNK-06, LNK-07]
key-files:
  created:
    - crates/trackly-app/src/http/link.rs
    - crates/trackly-app/tests/link_enroll.rs
    - crates/trackly-app/tests/link_auth.rs
  modified:
    - crates/trackly-app/src/http/sso.rs
    - crates/trackly-app/src/http/auth.rs
    - crates/trackly-app/src/http/mod.rs
    - crates/trackly-app/tests/suites/server.rs
    - crates/trackly-app/tests/http_route_parity.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs
decisions:
  - "Ядро Negotiate (gate, разбор заголовка, keytab, accept_spnego) — negotiate_identity -> Result<Negotiated, SsoFailure>; handler_ad_sso стал тонкой обёрткой с прежними статусами и текстами."
  - "Общая выдача сессии — auth::establish_session (flush -> cycle_id -> insert identity -> expiry); build_auth_login и входы Link идут через неё."
  - "complete_sso_login вынесена в pub-функцию: настоящий Kerberos без KDC не воспроизвести, шаг после проверки билета тестируется отдельно."
  - "Отказ запрещённому устройству идёт раньше проверки пароля/токена на всех трёх входах (verify_device_headers)."
metrics:
  tasks: 3
  completed: 2026-11-28
---

# Phase 44 Plan 11: публичные маршруты Link Summary

Пять публичных маршрутов Link (`link_enroll`, `link_auth_login`, `link_auth_sso`, `link_resume`, `link_logout`) с проверкой устройства до пароля/токена, отдельными GovernorLayer у enroll и входа паролем, и общим ядром Negotiate, вынесенным из браузерного SSO без изменения его поведения.

## Что сделано

- **Task 1+2** (`0a559a2a`): `http/link.rs`, рефакторинг `sso.rs` и `auth.rs`, подключение `link::router()` в `build_router` рядом с `sso::router()` (вне защищённых маршрутов). Коды ошибок: enroll 400/410/409 (`LINK_INVITE_INVALID/EXPIRED/USED`), устройство 403 `LINK_DEVICE_BANNED` / 401 `LINK_DEVICE_UNKNOWN` (неизвестное и неверный секрет неразличимы), вход 401 `LINK_BAD_CREDENTIALS` / 403 `LINK_USER_BLOCKED`, SSO 503 `LINK_SSO_DISABLED` / 401 Negotiate (`LINK_SSO_CHALLENGE`) / 401 `LINK_SSO_DENIED`, resume 401 `LINK_REVOKED`. Все тела — `{ code, message }`.
- **Task 3** (`0cdad569`): пять вердиктов в `ROUTE_WITHOUT_CMD`, пять строк в `ROUTE_MATRIX_EXEMPT`, `link.rs` добавлен в список исходников матрицы ролей (список жёстко перечислен; без этого запись «устарела»). Мутация проверена: удаление вердикта `link_resume` краснит `every_http_api_route_maps_to_a_tauri_command`.

## Гейты

| Гейт | Результат |
|------|-----------|
| `cargo test -p trackly-app --test app_server -- link_enroll link_auth link_unknown` | 12/12 |
| `cargo test -p trackly-app --test app_gates -- http_route_parity` | 7/7 |
| `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_auth` (браузерный вход, SSO, матрица ролей) | 138/138 |
| `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_gates` | 290/291, красный — `entities_changed_gate::writer_owning_services_outside_scope_are_listed` (см. ниже, не из этого плана) |
| `cargo clippy -p trackly-app --all-targets -- -D warnings` | чисто |
| `cargo fmt --check` | чисто |
| `grep tracing::.*(password|token|Authorization)` в link.rs | пусто |

## Deviations from Plan

**1. [Rule 1 - Bug] flush не меняет идентификатор сессии.** В tower-sessions 0.15 `flush()` удаляет запись, но загруженная запись сохраняет старый `Id`, и следующий `insert` создаёт сессию под ним: подложенный до входа cookie оставался валидным (Test 9 красный). В `establish_session` после `flush()` добавлен `cycle_id()`. Исправление затрагивает и браузерный `auth_login`: теперь он тоже выдаёт новый идентификатор; браузерные тесты (138/138) зелёные. Коммит `0a559a2a`.

**2. Тест «заблокированный пользователь -> 403 LINK_USER_BLOCKED» для пароля воспроизвести нельзя:** локальный неактивный пользователь для `AuthService::login` неотличим от неверного пароля (Unauthorized — защита от перечисления). Код `LINK_USER_BLOCKED` из `AccessBlocked/RegistrationPending/Forbidden` покрыт через SSO-завершение (неизвестная учётка при выключенном автоприёме) и через resume с неактивным пользователем.

**3. Код `LINK_DEVICE_EXISTS`, `LINK_BAD_REQUEST`, `LINK_DEVICE_UNKNOWN`, `LINK_SSO_CHALLENGE`, `LINK_NO_SESSION`, `LINK_INTERNAL`** объявлены локально в `link.rs`: в `link-proto` их нет, а менять контракт вне списка файлов плана не стали. Отозванный код-приглашение отдаётся как `LINK_INVITE_INVALID`.

**4. Успешный SSO тестируется через `complete_sso_login`** (мини-роутер с сессией), а не через настоящий Kerberos-билет; сам Negotiate-путь покрыт тестами отказов (выключен, challenge, мусорный билет при непарсящемся keytab, запрещённое устройство).

**5. Коммиты Task 1 и Task 2 объединены** (`0a559a2a`): `link.rs` и `suites/server.rs` общие для двух задач и раздельно не собираются.

Ограничение тестов: rate limit проверяется через `oneshot` с вручную вставленным `ConnectInfo` (не через TCP).

## Deferred Issues (вне области плана)

- `entities_changed_gate::writer_owning_services_outside_scope_are_listed` красный: `link_service.rs` (план 44-06) владеет `writer.execute` и не внесён в `OUT_OF_SCOPE_WRITERS`. Нужна запись в реестр гейта (`tests/entities_changed_gate.rs`). Не чинилось: файл гейта вне списка плана; закрыть до сплошного прогона плана 20.

## Авто-одобренные чекпоинты

Нет (план полностью автономный). Живая проверка SSO по Kerberos на настоящем AD не выполнялась: для человека — на Windows-стенде с доменной машиной открыть Link с включённым SSO и убедиться, что `link_auth_sso` отдаёт 200 `method: "sso"` без пароля.

## Known Stubs

Нет.

## Threat Flags

Новая публичная поверхность — ровно пять маршрутов из threat_model плана (T-44-37..42), все с митигациями: governor на enroll и login, устройство до пароля, flush+cycle_id до insert, тела без деталей Kerberos/БД.

## Self-Check: PASSED

Файлы link.rs, link_enroll.rs, link_auth.rs существуют; коммиты 0a559a2a и 0cdad569 в истории.
