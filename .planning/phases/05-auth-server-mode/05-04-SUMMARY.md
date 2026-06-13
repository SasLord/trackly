---
phase: 05-auth-server-mode
plan: 04
subsystem: auth
tags: [rbac, axum, tower-sessions, tauri, authorize, role-enforcement]

requires:
  - phase: 05-03
    provides: build_router() с SessionManagerLayer, session_identity() helper, resolve_tauri_identity()

provides:
  - RBAC enforcement (authorize()) на всех mutation HTTP-эндпоинтах devices/acts/cartridges/users
  - Tauri mutation commands используют resolve_tauri_identity() для D-Desktop-01/02
  - role_endpoint_matrix CI тест GREEN (ROADMAP success criterion #3)
  - 9-case role×endpoint матрица: no-session→401, employee→403, manager→200, admin→200

affects:
  - 05-05-PLAN.md (server-mode и финальные тесты опираются на RBAC)

tech-stack:
  added: []
  patterns:
    - "build_* helpers принимают caller: &Identity и вызывают authorize() внутри — единая точка авторизации для обоих транспортов"
    - "HTTP handlers: session_identity(&session) → extract Identity → передать в build_*"
    - "Tauri commands: resolve_tauri_identity(ctx) → extract Identity → передать в build_*"
    - "Тесты создают сессии программно через RusqliteSessionStore::create() — обход GovernorLayer"

key-files:
  created:
    - crates/trackly-app/tests/role_endpoint_matrix.rs
  modified:
    - crates/trackly-app/src/http/devices.rs
    - crates/trackly-app/src/http/acts.rs
    - crates/trackly-app/src/http/cartridges.rs
    - crates/trackly-app/src/tauri_cmds/devices.rs
    - crates/trackly-app/src/tauri_cmds/acts.rs
    - crates/trackly-app/src/tauri_cmds/cartridges.rs
    - crates/trackly-app/tests/acts_http_smoke.rs
    - crates/trackly-app/tests/devices_http_smoke.rs

key-decisions:
  - "authorize() вызывается в build_* helpers, а не в HTTP handlers напрямую — это гарантирует, что оба транспорта (HTTP и Tauri) проходят одну и ту же проверку"
  - "Тесты обходят /auth_login (GovernorLayer требует реальный TCP peer IP) и создают сессии напрямую через RusqliteSessionStore"
  - "role_endpoint_matrix использует macro_rules! new_app! для создания свежего router на каждый test case (oneshot потребляет router)"

patterns-established:
  - "Mutation build_* signature: pub async fn build_X_create(ctx: &AppCtx, caller: &Identity, ...) -> Result<...>"
  - "HTTP mutation handler pattern: let identity = session_identity(&session).await.map_err(...)?; build_*(ctx, &identity, ...).await"
  - "Tauri mutation command pattern: let caller = resolve_tauri_identity(state.inner()).await?; build_*(state.inner(), &caller, ...).await"
  - "Test session creation: RusqliteSessionStore::create() с Record {id: Id::default(), data: {identity: SessionIdentity}, expiry_date: +1day}"

requirements-completed: [USR-02, USR-06, SRV-03]

duration: 180min
completed: 2026-06-13
---

# Phase 05 Plan 04: RBAC Retrofit — HTTP handlers и Tauri commands Summary

**authorize() принудительно применён во всех mutation-эндпоинтах devices/acts/cartridges через build_* helpers; role_endpoint_matrix (9 cases) GREEN — ROADMAP criterion #3 выполнен**

## Performance

- **Duration:** ~180 min
- **Started:** 2026-06-13T08:00:00Z
- **Completed:** 2026-06-13T11:29:41Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Все mutation HTTP-эндпоинты для devices, acts, cartridges и users теперь требуют аутентифицированной сессии и роль ≥ Manager (или Admin для users)
- Tauri mutation commands для devices/acts/cartridges используют resolve_tauri_identity() — D-Desktop-01/02 compliance
- role_endpoint_matrix CI тест с 9 случаями полностью GREEN: employees получают 403 на mutation-операциях, managers и admins получают 200, неаутентифицированные запросы получают 401
- acts_http_smoke и devices_http_smoke обновлены для работы с full build_router() + session layer

## Task Commits

1. **Task 1: Retrofit authorize() в HTTP handlers и Tauri commands** - `c139c31` (feat)
2. **Task 2: role_endpoint_matrix CI тест GREEN** - `c874cfd` (feat)

## Files Created/Modified
- `crates/trackly-app/tests/role_endpoint_matrix.rs` — создан: 9-case role×endpoint матрица CI тест
- `crates/trackly-app/src/http/devices.rs` — изменён: все handlers принимают Session, mutations передают &identity в build_*
- `crates/trackly-app/src/http/acts.rs` — изменён: то же что devices, для acts mutations
- `crates/trackly-app/src/http/cartridges.rs` — изменён: то же, для cartridges mutations
- `crates/trackly-app/src/tauri_cmds/devices.rs` — изменён: build_* helpers принимают caller: &Identity + authorize(), Tauri wrappers используют resolve_tauri_identity
- `crates/trackly-app/src/tauri_cmds/acts.rs` — изменён: то же для acts
- `crates/trackly-app/src/tauri_cmds/cartridges.rs` — изменён: то же для cartridges
- `crates/trackly-app/tests/acts_http_smoke.rs` — изменён: переключён на build_router() + programmatic admin session
- `crates/trackly-app/tests/devices_http_smoke.rs` — изменён: переключён на build_router() + assert 401 without session

## Decisions Made
- **Точка авторизации — build_* helpers:** authorize() вызывается не в HTTP handler и не в Tauri command, а в общем build_* helper. Это гарантирует, что оба транспорта не могут обойти проверку.
- **Обход GovernorLayer в тестах:** /auth_login использует PeerIpKeyExtractor, который требует реальный TCP peer IP. В tower oneshot-тестах его нет. Решение — создавать сессии напрямую через RusqliteSessionStore::create(), минуя login endpoint.
- **macro_rules! new_app! для test isolation:** oneshot() потребляет router, поэтому каждый из 9 test cases должен получить свежий router. Макрос создаёт новый RusqliteSessionStore + build_router на месте.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] devices_http_smoke: 500 вместо ожидаемого 200 — Session extractor без SessionManagerLayer**
- **Found during:** Task 1 (retrofit HTTP handlers)
- **Issue:** После добавления Session в devices handlers, старый тест использовал devices_router() без SessionManagerLayer → Session extractor возвращал 500
- **Fix:** Переключён на build_router() + assert 401 (без сессии mutation должен возвращать 401, не 200)
- **Files modified:** crates/trackly-app/tests/devices_http_smoke.rs
- **Verification:** cargo test devices_http_smoke — PASSED
- **Committed in:** c139c31 (Task 1 commit)

**2. [Rule 1 - Bug] acts_http_smoke: 500 из-за отсутствия SessionManagerLayer**
- **Found during:** Task 1 (retrofit acts handlers)
- **Issue:** acts_http_smoke использовал acts_router() без session layer
- **Fix:** Переключён на build_router() + создание admin сессии программно через create_admin_session() helper
- **Files modified:** crates/trackly-app/tests/acts_http_smoke.rs
- **Verification:** cargo test acts_http_smoke — PASSED (2 tests)
- **Committed in:** c874cfd (Task 2 commit)

**3. [Rule 1 - Bug] role_endpoint_matrix: /auth_login возвращал 500 "Unable To Extract Key!"**
- **Found during:** Task 2 (role_endpoint_matrix implementation)
- **Issue:** GovernorLayer на /auth_login требует реальный TCP peer IP через PeerIpKeyExtractor. В tower oneshot-тестах нет socket с IP адресом
- **Fix:** Полностью обойти /auth_login. Сессии создаются программно через RusqliteSessionStore::create() с tower_sessions::session::Record
- **Files modified:** crates/trackly-app/tests/role_endpoint_matrix.rs
- **Verification:** cargo test role_endpoint_matrix — PASSED (1 test, 9 assertions)
- **Committed in:** c874cfd (Task 2 commit)

**4. [Rule 1 - Bug] role_endpoint_matrix: Case 4 (acts_create) возвращал 422 вместо 403**
- **Found during:** Task 2 (role_endpoint_matrix green pass)
- **Issue:** Тест использовал неправильные имена полей (`act_number` вместо `number_override`) и пропускал обязательные поля (`giver_name`, `receiver_name`). axum JSON extractor возвращал 422 Unprocessable Entity ещё до того, как handler мог выполнить authorize()
- **Fix:** Исправлена структура payload: `number_override: null, giver_name: "...", receiver_name: "..."` и все обязательные поля
- **Files modified:** crates/trackly-app/tests/role_endpoint_matrix.rs
- **Verification:** Case 4 → 403 как ожидалось
- **Committed in:** c874cfd (Task 2 commit)

**5. [Rule 1 - Bug] role_endpoint_matrix: Case 9 (devices_list) возвращал 422 вместо 200**
- **Found during:** Task 2 (role_endpoint_matrix green pass)
- **Issue:** `DeviceFilter` имеет поля `include_deleted: bool` и `group_by_condition: bool` без `#[serde(default)]`. Отправка `{}` JSON → 422 из-за missing fields
- **Fix:** Предоставлен полный filter объект с `"include_deleted": false, "group_by_condition": false` и остальными nullable полями
- **Files modified:** crates/trackly-app/tests/role_endpoint_matrix.rs
- **Verification:** Case 9 → 200 как ожидалось
- **Committed in:** c874cfd (Task 2 commit)

---

**Total deviations:** 5 auto-fixed (5 bugs)
**Impact on plan:** Все авто-фиксы необходимы для корректности тестов. Изменение архитектуры (build_router вместо отдельных routers в тестах) было неизбежным следствием добавления Session layer. Scope не изменился.

## Issues Encountered

**GovernorLayer + tower oneshot incompatibility:** Фундаментальное ограничение — PeerIpKeyExtractor требует реальный TCP-адрес клиента, недоступный в unit-тестах на базе tower::ServiceExt::oneshot(). Правильное решение — обход /auth_login в тестах и программное создание сессий. Это рабочий pattern для всех будущих тестов с аутентификацией.

## User Setup Required

None — изменения только в backend Rust коде и тестах. Внешних сервисов не требуется.

## Next Phase Readiness

- RBAC полностью применён ко всем mutation-эндпоинтам — ready для Plan 05 (financial/auth final integration)
- role_endpoint_matrix GREEN — ROADMAP success criterion #3 выполнен
- Паттерн programmatic session creation установлен для будущих integration тестов
- Нет открытых блокеров

---
*Phase: 05-auth-server-mode*
*Completed: 2026-06-13*

## Self-Check: PASSED

- `crates/trackly-app/tests/role_endpoint_matrix.rs` — FOUND
- `crates/trackly-app/src/http/devices.rs` — FOUND
- `crates/trackly-app/src/http/acts.rs` — FOUND
- `crates/trackly-app/src/http/cartridges.rs` — FOUND
- `crates/trackly-app/src/tauri_cmds/devices.rs` — FOUND
- `crates/trackly-app/src/tauri_cmds/acts.rs` — FOUND
- `crates/trackly-app/src/tauri_cmds/cartridges.rs` — FOUND
- Commit `c139c31` — FOUND
- Commit `c874cfd` — FOUND
