# Phase 44: Link — транспорт, пин сертификата и вход - Pattern Map

**Mapped:** 2026-11-27
**Files analyzed:** 27 (новые и изменяемые)
**Analogs found:** 22 / 27 (5 без аналога в кодовой базе; нужно брать RESEARCH.md)

Приватность: репозиторий публичный. Все примеры вымышлены; реальных имён хостов, доменов, ФИО и отпечатков нет и в код/тесты они не попадают.

## File Classification

| Новый/изменяемый файл | Роль | Поток данных | Ближайший аналог | Качество |
|---|---|---|---|---|
| `crates/link-app/src/main.rs` (изменить) | shell/entry | event-driven | `crates/trackly-app/src/main.rs` (строки 240-265) + сам каркас | role-match |
| `crates/link-app/src/config.rs` | utility (чистые функции) | file-I/O | `build_link_info` в `crates/link-app/src/main.rs:13-28` (чистая функция + тесты) | partial |
| `crates/link-app/src/core/*` (автомат связи, backoff) | service | event-driven | `packages/transport/src/ws.ts` (логика backoff/5 с/сон на TS) | role-match (зеркало логики) |
| `crates/link-app/src/net/pin.rs` | utility (TLS-верификатор) | request-response | `crates/trackly-app/src/server/tls.rs` (`ensure_crypto_provider`, `compute_fingerprint`) | partial |
| `crates/link-app/src/net/http.rs`, `net/ws.rs` | service | request-response / streaming | клиентского аналога нет; серверная сторона: `crates/trackly-app/src/http/ws.rs` | partial |
| `crates/link-app/src/auth/{sso,password,store}.rs` | service | request-response | серверный `http/sso.rs`, `http/auth.rs`; mock-идея `TRACKLY_AD_MOCK` | partial |
| `crates/link-app/src/shell/{tray,autostart,window}.rs` | shell | event-driven | `crates/trackly-app/src/main.rs:253` (single-instance); трея в репо нет | no analog (tray) |
| `crates/link-app/src/commands.rs` | controller (Tauri commands) | request-response | `link_info` в `crates/link-app/src/main.rs:35-42` | exact |
| `crates/link-app/Cargo.toml`, `tauri.conf.json`, capabilities | config | — | `crates/link-app/Cargo.toml` | exact |
| `crates/link-proto/` (новый крейт: SPKI, код приглашения, DTO) | utility | transform | `compute_fingerprint` в `tls.rs:53-59` + golden-фикстура (память js_rust_mirror) | partial |
| `migrations/V060__link_devices.sql` | migration | CRUD | `migrations/V059__idempotency_keys.sql` | exact |
| `crates/trackly-app/src/server/tls.rs` (изменить: SPKI, сохранение ключа) | utility | file-I/O | сам файл, `load_or_generate_self_signed` (280-304) | exact |
| `crates/trackly-app/src/http/link.rs` (enroll, login, sso, resume, logout, admin) | controller | request-response | `http/sso.rs` + `http/auth.rs` | exact |
| `crates/trackly-app/src/services/link_service.rs` | service | CRUD | `services/chat_service.rs` (writer.execute) | exact |
| `crates/trackly-app/src/server/link_hub.rs` | service (pub-sub, разрыв WS) | pub-sub | `services/im_hub.rs` | role-match |
| `crates/trackly-app/src/http/ws.rs` (изменить: регистрация в LinkHub, ветка cancel) | controller | streaming | сам файл, ветка `shutdown.cancelled()` (стр. ~244) | exact |
| `crates/trackly-app/src/http/mod.rs` (изменить: link_guard, merge router) | middleware | request-response | `refresh_session_role` (mod.rs:63-85) | exact |
| `crates/trackly-app/src/services/auth.rs` (изменить: блокировка -> revoke_user) | service | CRUD | `update_user` (auth.rs:1797) | exact |
| `crates/trackly-app/src/tauri_cmds/link.rs` (админ-команды) | controller | request-response | существующие `tauri_cmds/*` | role-match |
| `crates/trackly-app/tests/link_*.rs` (http, revoke, enroll, tls) | test | request-response | `tests/chat_http.rs` | exact |
| `crates/trackly-app/tests/http_route_parity.rs`, `role_endpoint_matrix.rs` (изменить) | test/gate | — | записи `auth_ad_sso` (parity:183, matrix:7203) | exact |
| `link-ui/src/App.svelte` + новые экраны Link | component | request-response | `ui/src/features/auth/LoginPage.svelte`, `link-ui/src/App.svelte` | role-match |
| `link-ui/src/lib/*.ts` (invoke-обёртки, link-state) | utility | event-driven | `link-ui/src/lib/linkInfo.ts` | exact |
| `ui/src/features/link/LinkPage.svelte` (+ вкладки, попап кода) | component | CRUD | `ui/src/features/users/UsersPage.svelte` | role-match |
| `ui/src/routes.ts`, `ui/src/features/layout/sidebar-config.ts` (изменить) | config | — | строка `'/users'` в обоих | exact |
| `ui/src/features/settings/NetworkSettings.svelte` (изменить: SPKI) | component | request-response | сам файл, блок отпечатка (183-186) | exact |
| `ui/src/features/users/UserFormModal.svelte` / `UserListRow.svelte` (кнопка «Отозвать входы Link») | component | request-response | `UserListRow.svelte` (`btn-action`, инлайн-подтверждение 58-64) | exact |
| `docs/AD-SETUP.md` (дополнить) | doc | — | сам файл | exact |

## Pattern Assignments

### `crates/link-app/src/commands.rs` и точка входа `main.rs` (Tauri command)

**Analog:** `crates/link-app/src/main.rs`

**Паттерн команды + чистая функция + тесты** (стр. 13-42): логика в чистой `pub fn build_*`, команда `#[tauri::command]` тонкая обёртка, юнит-тесты на чистую функцию.
```rust
#[tauri::command]
fn link_info(app: tauri::AppHandle) -> LinkInfo {
    build_link_info(&app.package_info().version.to_string(),
        tauri::webview_version().ok(), std::env::current_exe().ok())
}
tauri::Builder::default()
    .plugin(tauri_plugin_window_state::Builder::default().build())
    .invoke_handler(tauri::generate_handler![link_info])
```
Изменения для 44: `window_state::Builder::default().with_state_flags(StateFlags::all() - StateFlags::VISIBLE)` (RESEARCH Pitfall 3); блок `setup` со «страховкой `w.show()`» (стр. 51-56) заменить решением автомата (D-18). Плагин single-instance — как в `crates/trackly-app/src/main.rs:253`, но колбэк делает `show+set_focus` (D-19), а не лог:
```rust
.plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
    tracing::info!("single-instance: second launch ignored");
}))
```
Cargo: версии единиц через `workspace = true` (как `tauri`, `tauri-plugin-window-state` в `crates/link-app/Cargo.toml`); `tauri-plugin-single-instance = "2"` уже в корневом `Cargo.toml:77`. `tauri` для link-app дополнить `features = ["tray-icon","image-png"]` отдельной записью (не раздувать trackly-app). Литеральная `version = "0.1.0"` у link-app сохраняется (D-10 Фазы 42).

Мост событий Rust -> webview: `emit('link-event' / 'trackly-event' / 'trackly-resync')`, как в `crates/trackly-app/src/desktop_event_bridge.rs:44-52` (константа `RESYNC_EVENT = "trackly-resync"`, `resync_on_lagged`). Для Link служебный кадр `resync_required` превращается в `emit("trackly-resync", ())`; `packages/transport` уже слушает это событие (`onWsResync`).

---

### `crates/link-app/src/net/pin.rs` (utility, TLS-пин)

**Analog:** `crates/trackly-app/src/server/tls.rs`

**Явный провайдер ring** (стр. 15-30) — не полагаться на process-default (в графе есть и aws-lc-rs):
```rust
static CRYPTO_PROVIDER_INIT: Once = Once::new();
pub fn ensure_crypto_provider() {
    CRYPTO_PROVIDER_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}
```
**Формат отпечатка** (стр. 53-59) — `AB:CD:...`, 95 символов; SPKI-версия должна давать тот же формат (в `link-proto`):
```rust
fn compute_fingerprint(der_bytes: &[u8]) -> String {
    let hash = Sha256::digest(der_bytes);
    hash.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":")
}
```
**ALPN http/1.1** (стр. 71-73, `pin_http1_alpn`) — Link-клиент тоже `http1_only()` (SPNEGO в одном TCP-соединении).

Сам `ServerCertVerifier` — аналога нет, брать RESEARCH.md «Pattern 1 / Верификатор пина (каркас)» (чистая функция сравнения тестируется без сети).

---

### `crates/trackly-app/src/server/tls.rs` (изменить: SPKI + сохранение ключа, D-06)

**Analog:** сам файл.

**Точка правки** — `load_or_generate_self_signed` (стр. 280-304): при несовпадении SAN сейчас вызывается `generate_self_signed(host)` целиком (новый ключ):
```rust
if let Some(bundle) = try_load_saved(&cert_path, &key_path, &sans_path, &wanted) { return Ok(bundle); }
let bundle = generate_self_signed(host)?;
save_self_signed(&cert_path, &key_path, &sans_path, &bundle, &wanted);
```
Изменение: если `server.key` читается (`rcgen::KeyPair::from_pem`), выпускать новый сертификат на старом ключе; новый ключ — только если ключа нет или он повреждён. `TlsBundle` (стр. 33-47) получает поле `spki_fingerprint_hex`; заполняется там же, где `fingerprint_hex` (стр. 234 и 459: `generate_self_signed`, `load_from_pem`).

**Тесты** — рядом, по образцу (стр. 519-546): `load_or_generate_reuses_saved_cert_for_same_host`, `load_or_generate_regenerates_when_host_changes`. Новый тест: смена SAN -> тот же SPKI, другой DER-отпечаток. Права ключа `0o600` (`write_private_key`, стр. 351) сохранять.

---

### `crates/link-proto/` (общие чистые функции)

**Analog:** `compute_fingerprint` (tls.rs:53) как форма; golden-фикстура — урок из памяти `js_rust_mirror_needs_fixture_gate`: тестовая PEM-пара + ожидаемый хэш в `crates/link-proto/tests/fixtures`, НЕ боевой сертификат. Клиент и сервер зовут одну `spki_sha256` (RESEARCH «Code Examples»). Код приглашения: кодек + тест на «круг» (encode/decode, истёкший срок, битая CRC). Серверный крейт подключает `link-proto` через workspace.

---

### `migrations/V060__link_devices.sql` (migration)

**Analog:** `migrations/V059__idempotency_keys.sql`

Правила (по памяти `migration_user_version_pragma`, `migration_fk_off_noop_in_refinery_tx`): только `CREATE`, FK на `users` не ставить, **последняя строка обязательно** `PRAGMA user_version = 60;`:
```sql
CREATE UNIQUE INDEX uq_requests_client
  ON requests (requested_by_user_id, client_request_id)
  WHERE client_request_id IS NOT NULL;

PRAGMA user_version = 59;
```
Схема таблиц `link_devices`, `link_logins`, `link_invites` — в RESEARCH Pattern 5. Миграции встраиваются через `embed_migrations` + `build.rs` (память `embed_migrations_stale_build`): после добавления файла проверить, что `user_version` в тестах = 60.

---

### `crates/trackly-app/src/http/link.rs` (controller: enroll, login, sso, resume, logout, админ-маршруты)

**Analog:** `crates/trackly-app/src/http/sso.rs` и `http/auth.rs`

**Imports** (sso.rs:21-38):
```rust
use axum::{extract::State, http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response}, routing::get, Json, Router};
use tower_sessions::Session;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use crate::context::AppCtx;
use crate::error_axum::AppErrorResponse;
use crate::http::auth::SessionIdentity;
```
**Выдача сессии (flush до insert, T-05-SF)** — копировать из `issue_sso_session` (sso.rs:68-95) и `build_auth_login` (auth.rs:~150-190), добавив в сессию `link_device_id`/`link_login_id`:
```rust
session.flush().await.map_err(|e| AppError::Internal { source_chain: format!("session flush (sso): {e}") })?;
session.insert("identity", SessionIdentity::from(&identity)).await.map_err(...)?;
session.set_expiry(Some(tower_sessions::Expiry::OnInactivity(time::Duration::days(30))));
```
**SSO-гейт 503 и Negotiate-заголовок** — `handler_ad_sso` (sso.rs:98-132): 503 при выключенном SSO, `401 + WWW-Authenticate: Negotiate`, `b64().decode`; ядро (после проверки токена) вынести в общую функцию, чтобы `link_auth_sso` и браузерный `handler_ad_sso` делили логику (поведение браузера не меняется).
**Тело запроса** — обёртка `{ req: ... }` (`LoginPayload`, auth.rs:~37-40) для маршрутов, у которых есть Tauri-аналог; HTTP-only маршруты Link (`link_enroll`, `link_auth_*`, `link_resume`, `link_logout`) идут как «только HTTP».
**Rate limit** — `GovernorLayer` через `route_layer` как у `auth_login` (http/mod.rs:~100-125, `GovernorConfigBuilder::default().per_second(1).burst_size(5)`); `link_enroll` и `link_auth_login` обернуть тем же приёмом, со своим конфигом.
**Ошибки** — `AppErrorResponse::from(e).into_response()`; наружу без деталей Kerberos/keytab (sso.rs:190-205).
**router()** — `pub fn router() -> Router<AppCtx>` и `merge` в `http::mod` (как `sso::router`, sso.rs:207-209).

---

### `crates/trackly-app/src/services/link_service.rs` (service, CRUD, единственный писатель)

**Analog:** `crates/trackly-app/src/services/chat_service.rs`

**Структура** (стр. 135-175): `Arc<WriterHandle>`, `Arc<ReaderPool>`, часы, `with_ws_tx` builder, `#[derive(Clone)]`.
**Запись** (стр. 388-405, `mark_read`): работа в `self.writer.execute(move |conn| { ... }).await?`; рассылка/побочные эффекты только ПОСЛЕ успешного коммита. Атомарное погашение кода приглашения (одно `UPDATE ... WHERE used_at_utc IS NULL AND expires_at_utc > ?` и проверка числа затронутых строк) — внутри одного writer-замыкания; TOCTOU читается в самом замыкании (комментарий chat_service.rs:519).
**Чтение**: `tokio::task::spawn_blocking(move || { let conn = readers.acquire(); ... })` (chat_service.rs:~415).
**Хэш токена** — SHA-256 (не Argon2); верификация пароля остаётся в `services/auth.rs` (argon2 в `spawn_blocking`). Регистрация в `AppCtx` — по образцу `im_hub` (context.rs:148, 240, 597).

---

### `crates/trackly-app/src/server/link_hub.rs` (pub-sub, разрыв WS на лету, D-14)

**Analog:** `crates/trackly-app/src/services/im_hub.rs`

```rust
#[derive(Debug, Default)]
pub struct ImHub { chans: Mutex<HashMap<i64, broadcast::Sender<ImSignal>>> }
fn lock(&self) -> MutexGuard<'_, HashMap<..>> { self.chans.lock().unwrap_or_else(|e| e.into_inner()) }
pub fn prune(&self) { self.lock().retain(|_, tx| tx.receiver_count() > 0); }
```
Тот же приём (poison-safe `lock`, `prune`, тесты в том же файле), но значение — `CancellationToken` на `device_id`; `revoke(device_id | user_id)` отменяет токены. Поле в `AppCtx` — как `im_hub`.

---

### `crates/trackly-app/src/http/ws.rs` (изменить)

**Analog:** сам файл — ветка `shutdown.cancelled()` в `ws_loop` (стр. ~244, `tokio::select!`), а также `ws_auth_middleware` (стр. 138) и `ws_handler` (стр. 157).
Новая ветка `select!`: `() = link_cancel.cancelled()` -> Close-кадр с отдельной причиной (`revoked`) через `send_msg` с `CLOSE_SEND_TIMEOUT` (стр. 70-83) и выход. Токен регистрируется в `ws_handler` после `ws_auth_middleware`, когда в сессии есть `link_device_id`. Служебные кадры `hello`/`ping`/`pong`/`resync_required` и константы `PING_INTERVAL`/`IDLE_TIMEOUT` (стр. 85-90) не менять; тестовые тайминги — через `Extension(WsTimings{..})`.

---

### `crates/trackly-app/src/http/mod.rs` (изменить: `link_guard`)

**Analog:** `refresh_session_role` (mod.rs:63-85)

```rust
async fn refresh_session_role(State(ctx): State<AppCtx>, session: Session, req: Request, next: Next) -> Response {
    if let Ok(Some(si)) = session.get::<SessionIdentity>("identity").await { ... }
    next.run(req).await
}
```
`link_guard` строить так же: `from_fn_with_state`, ВНУТРИ session-слоя; если в сессии есть `link_device_id` — проверить `link_devices.revoked_at`, `link_logins.revoked_at`, `users.is_active`; при провале `session.flush()` + `401` с кодом `LINK_REVOKED`. Принцип отсюда же: при ошибке БД сессию НЕ трогать (не разлогинивать из-за сбоя чтения). Подключить рядом с `refresh_session_role` в `build_router`.

---

### `crates/trackly-app/src/services/auth.rs` (изменить: блокировка -> отзыв)

**Analog:** `update_user` (auth.rs:1797) и точка soft-delete `request_service.rs:1122` (`UPDATE users SET deleted_at_utc = ?1, is_active = 0`). Исполнитель находит все места, где `is_active` -> 0 или `deleted_at_utc` ставится (поиск ОТ репозитория, память `write_site_gate_inventory_verdicts`), и после коммита зовёт `link.revoke_user(user_id)`. Тест: «заблокировать -> Link-сессия получает 401, WS закрыт».

---

### `crates/trackly-app/tests/link_*.rs` (тесты маршрутов)

**Analog:** `crates/trackly-app/tests/chat_http.rs`

Уровень роутера (`tower::ServiceExt::oneshot`), `AppCtx::build` на `TempDir`, вымышленные имена; сессия подкладывается напрямую в `RusqliteSessionStore`:
```rust
let store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
let mut record = Record { id, data: Default::default(), expiry_date: OffsetDateTime::now_utc() + Duration::days(1) };
record.data.insert("identity".to_string(), serde_json::to_value(&si).unwrap());
store.create(&mut record).await.expect("session");
format!("id={id}")
```
Вспомогательные `chat_support::{seed_user, ...}`. Каждый новый маршрут — тест статуса и тела, включая «404 JSON, а не index.html» (память `spa_fallback_masks_missing_api_routes`).

**Реестровые гейты:** `tests/http_route_parity.rs` — HTTP-only маршруты добавить в `ROUTE_WITHOUT_CMD` как `Verdict { name: "link_enroll", reason: "..." }` (формат, стр. 182-188); админ-маршруты Link — парой «команда Tauri + HTTP». `tests/role_endpoint_matrix.rs:7203` — запись вида `("auth_ad_sso", "публичный маршрут аутентификации ...")` для публичных `link_enroll`/`link_auth_*`/`link_resume`; админ-маршруты — admin-only строки матрицы. Также `entities_changed_gate` и инвентарь write-site — сверять ОТ репозитория.

---

### `link-ui` экраны (component)

**Analog:** `ui/src/features/auth/LoginPage.svelte` (экран пароля), каркас — `link-ui/src/App.svelte`

Импорты примитивов (LoginPage.svelte:1-14):
```ts
import AuthShell from '@trackly/kit/components/AuthShell.svelte';
import FormField from '@trackly/kit/components/FormField.svelte';
import Input from '@trackly/kit/components/Input.svelte';
import Button from '@trackly/kit/components/Button.svelte';
```
Паттерны для копирования: состояние `$state` (login/password/loading/serverError), единое обезличенное сообщение «Неверный логин или пароль» (D-Sec-01, стр. 16-19), кнопка автовхода и `ssoLoading` (стр. 49-72). НО: LoginPage ходит через `fetch('/api/v1/...')`; в Link сети в webview нет (CSP ipc-only), поэтому вместо `fetch` — `invoke('link_*')` (по образцу `link-ui/src/lib/linkInfo.ts:10-17`, включая проверку `'__TAURI_INTERNALS__' in window` и `try/catch` -> null) и `listen('link-state')`.
Каркас `App.svelte` (грид `.shell`, `.topbar`, `Tabs`, `ThemeSwitcher`, заглушка «Здесь скоро будет чат», `StatusLine`) сохраняется; высота шапки 56px и блок учётной записи — по UI-SPEC §3, §6.7. Литералы `gap: 12px`, `padding: 8px 16px` при правке перевести на токены. Конечный автомат экранов — один `$state` от события `link-state` (Setup | Connecting | NeedLogin | Online | Offline | Revoked | PinMismatch).

---

### `ui/src/features/link/*` (раздел «Link» в Trackly)

**Analog:** `ui/src/features/users/UsersPage.svelte` (страница + список + модал) и `UserListRow.svelte`
Регистрация: `ui/src/routes.ts:31` (`'/users': UsersPage,` -> добавить `'/link': LinkPage`), `ui/src/features/layout/sidebar-config.ts:32`:
```ts
{ kind: 'item', route: '/users', label: 'Пользователи', phase: 5, roles: ['admin'] },
```
(новая запись только `roles: ['admin']`; серверная проверка роли в каждом маршруте — граница безопасности). Транспорт — `apiCall` из `@trackly/transport/client` (как `LoginPage`). Инлайн-подтверждение «Да/Нет» и `.btn-action` — как `UserListRow.svelte:58-64`; «Отозвать» в таблице входов и «Разрешить» — тем же стилем. Попап выпуска кода — `Modal` из kit; вкладки — `Tabs`. Отпечаток/код — `.tr-mono` (UI-SPEC §4). Цветов/размеров литералами не писать (гейты `check-tokens.mjs`, `check-contrast.mjs`). После изменений фронтенда `pnpm --dir ui build` (память dev_browser_testing_needs_ui_build); запускать отдельно `svelte-check`, `pnpm lint`.

---

### `ui/src/features/settings/NetworkSettings.svelte` (изменить: SPKI, D-06)

**Analog:** сам файл, блок отпечатка (стр. 181-187):
```svelte
{#if formattedFingerprint}
  <div class="info-row">
    <span class="info-label">Отпечаток сертификата:</span>
    <code class="fingerprint">{formattedFingerprint}</code>
  </div>
{/if}
```
Добавить второй `info-row` «Отпечаток для Link (SPKI)» из нового поля статуса; поле `fingerprint` в интерфейсе (стр. 22, 33, 65, 108) продублировать полем `spkiFingerprint`. Серверная сторона — `http/settings.rs` + DTO + bindings (проверить specta-экспорт).

---

### `ui/src/features/users/*` (кнопка «Отозвать входы Link»)

**Analog:** `UserListRow.svelte` (инлайн-подтверждение, стр. 58-64) и `UserFormModal.svelte`. Серверный вызов — админ-маршрут «users_revoke_link» (пара Tauri + HTTP).

---

## Shared Patterns

### Сессия и защита от фиксации
**Source:** `crates/trackly-app/src/http/auth.rs` (`build_auth_login`) и `http/sso.rs::issue_sso_session`
**Apply to:** `link_auth_login`, `link_auth_sso`, `link_resume`
`session.flush()` ДО `session.insert("identity", ...)`, `set_expiry` ПОСЛЕ insert.

### Единственный писатель SQLite
**Source:** `services/chat_service.rs` (`self.writer.execute(move |conn| ...)`), чтение — `spawn_blocking` + `readers.acquire()`
**Apply to:** `link_service`, погашение кода, отзыв, `link_guard`. Побочные эффекты (WS-рассылка, `LinkHub.revoke`) только после Ok.

### Ошибки наружу без деталей
**Source:** `http/sso.rs:190-205`, `AppErrorResponse`
**Apply to:** все `link_*` маршруты и клиент Link (детали Kerberos — только в лог, без токенов и `Authorization: Negotiate`).

### Rate limit публичных маршрутов
**Source:** `http/mod.rs` (`GovernorConfigBuilder`, `route_layer(GovernorLayer::new(..))`)
**Apply to:** `link_enroll`, `link_auth_login` (свой конфиг, не общий с `auth_login`).

### Чистая функция + юнит-тест рядом
**Source:** `crates/link-app/src/main.rs` (`build_link_info` + `mod tests`), `tls.rs` (`mod tests`)
**Apply to:** `config.rs` (поиск конфига, валидация), backoff, классификация отказов SSO, сборка значения автозапуска, сравнение пина — всё тестируется на macOS без сети и AD. SSO — за трейтом `SsoProvider` + `MockSso`; «проверено» для SSPI/трея/RDS/автозапуска только после живого UAT на Windows (D-15, память auto_advance_auto_approves_human_verify).

### Реестровые гейты на дрейф
**Source:** `tests/http_route_parity.rs`, `tests/role_endpoint_matrix.rs`, `entities_changed_gate`
**Apply to:** все новые маршруты и мутации (инвентарь ОТ репозитория, не по именам).

### UI-гейты — разные команды
**Apply to:** фронтенд: `pnpm lint` НЕ включает `svelte-check`; clippy и fmt отдельно (память ui_gates_are_separate_commands). Рантайм Svelte-рун компиляция не ловит (compile_gates_miss_svelte_runtime).

## No Analog Found

| Файл | Роль | Поток | Причина |
|---|---|---|---|
| `link-app/src/shell/tray.rs` (иконки, меню, серый значок) | shell | event-driven | Трея в репозитории нет; брать RESEARCH Pattern 2 (`TrayIconBuilder`, `show_menu_on_left_click(false)`) |
| `link-app/src/shell/autostart.rs` | shell | file-I/O | Автозапуска нет; RESEARCH Pattern 2 + Pitfall 4 (`auto-launch` напрямую, путь в кавычках, `CurrentUser`) |
| `link-app/src/auth/sso.rs` (SSPI Negotiate) | service | request-response | Клиентского SPNEGO нет (сервер принимает через `sspi`); RESEARCH Pattern 4 (`cross-krb5`, спайк обязателен, запасной `windows` FFI), `cfg(windows)` |
| `link-app/src/auth/store.rs` (keyring-core + деградация) | service | CRUD | Хранилища секретов нет; RESEARCH «Keyring» + D-10/D-27 (файл `0600` для `device_secret`, токен входа только в памяти) |
| `link-app/src/net/pin.rs` — сам `ServerCertVerifier`, `net/http.rs`, `net/ws.rs` | service | request-response / streaming | Клиентской сети в Rust нет (только сервер tokio-tungstenite в workspace); RESEARCH Pattern 1 и Pattern 3 |

## Metadata

**Analog search scope:** `crates/link-app`, `crates/trackly-app/{src,tests}`, `migrations/`, `link-ui/src`, `packages/{kit,transport}`, `ui/src/{features,routes.ts}`
**Files scanned:** ~25 прочитано целиком или выборкой (остальное — grep)
**Pattern extraction date:** 2026-11-27
