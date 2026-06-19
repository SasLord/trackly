# Phase 9: AD-аутентификация и заявки на регистрацию пользователей - Pattern Map

**Mapped:** 2026-06-19
**Files analyzed:** 22 (new + modified)
**Analogs found:** 22 / 22 (every new file has a verified in-repo analog)

> All analogs below were read and excerpts extracted from the real codebase. The SNMP mock triad, `AuthService::login`, `app_settings` upsert, `requests` lifecycle, DTO/transport split, and the Svelte settings/login/requests components all exist and map 1:1 onto the Phase 9 work. There are **no greenfield files without a pattern source**.

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/trackly-core/src/ports/ad.rs` (new) | port (trait) | request-response | `crates/trackly-core/src/ports/snmp.rs` | exact |
| `crates/trackly-core/src/ports/mod.rs` (modify) | module index | — | same file (existing `pub mod snmp;`) | exact |
| `crates/trackly-infra/src/ad/mod.rs` (new) | module index | — | `crates/trackly-infra/src/snmp/mod.rs` | exact |
| `crates/trackly-infra/src/ad/real.rs` (new) | infra adapter | request-response (network I/O) | `crates/trackly-infra/src/snmp/real.rs` | exact |
| `crates/trackly-infra/src/ad/mock.rs` (new) | infra adapter | request-response (fixtures) | `crates/trackly-infra/src/snmp/mock.rs` | exact |
| `crates/trackly-infra/src/ad/discovery.rs` (new) | utility | transform (env/DNS → config) | `crates/trackly-infra/src/config.rs` (pure-derive style) | role-match |
| `crates/trackly-infra/src/config.rs` (modify) | config | — | `ServerConfig` in same file | exact |
| `crates/trackly-infra/src/lib.rs` (modify) | crate index | — | existing `pub mod snmp;` | exact |
| `crates/trackly-app/src/context.rs` (modify) | provider/wiring | — | SNMP switch (context.rs:282-294) | exact |
| `crates/trackly-app/src/services/auth.rs` (modify) | service | request-response + CRUD | `AuthService::login` (auth.rs:180) + `app_settings` (auth.rs:799-847) + `create_user` (auth.rs:214) | exact |
| `crates/trackly-app/src/services/request_service.rs` (modify) | service | CRUD/event-driven | `RequestService::create`/`transition` (same file) | exact |
| `crates/trackly-core/src/domain/requests.rs` (modify) | model | — | `RequestRow`/`RequestNew`/`RequestFilter` (same file) | exact |
| `crates/trackly-app/src/dto/auth.rs` (modify) | dto | — | `LoginRequest`/`NetworkSettingsDto` (same file) | exact |
| `crates/trackly-app/src/http/auth.rs` (modify) | route/transport | request-response | `build_auth_login`/`public_router` (same file) | exact |
| `crates/trackly-app/src/tauri_cmds/auth.rs` (modify) | route/transport | request-response | `build_*_tauri` helpers (same file) | exact |
| `migrations/V028__ad_register_subtype.sql` (new, IF chosen) | migration | — | `migrations/V019__users_is_active.sql` | exact |
| `ui/src/features/auth/LoginPage.svelte` (modify) | component | request-response | same file (extend `.login-card` shell) | exact |
| `ui/src/features/auth/PendingScreen.svelte` (new) | component | — | `LoginPage.svelte` `.login-card` shell | exact |
| `ui/src/features/auth/BlockedScreen.svelte` (new) | component | request-response | `LoginPage.svelte` shell + restore submit | exact |
| `ui/src/features/settings/ActiveDirectorySettings.svelte` (new) | component | CRUD (load/save) | `ui/src/features/settings/NetworkSettings.svelte` | exact |
| `ui/src/features/settings/SettingsSubNav.svelte` (modify) | component | — | same file (`SECTIONS` array) | exact |
| `ui/src/features/requests/{RequestListRow,RequestDetail,RequestsList/MasterDetail}.svelte` (modify) | component | event-driven | same files (`typeLabel`/approve modal) | exact |

---

## Pattern Assignments

### `crates/trackly-core/src/ports/ad.rs` (port trait, request-response)

**Analog:** `crates/trackly-core/src/ports/snmp.rs` (whole file, 72 lines — read in full)

**I/O-free invariant + imports** (snmp.rs:1-13, 43-48):
```rust
//! Pattern: like `Clock`, this trait lives in trackly-core but has NO tokio/snmp2
//! imports — I/O-free invariant enforced by `tests/no_io_deps.rs`.
use async_trait::async_trait;
use crate::error::AppError;

/// CRITICAL: This trait MUST NOT import tokio or snmp2 — those are infra-layer deps.
/// `async_trait` is the only allowed external dependency here (pure-data crate).
#[async_trait]
pub trait SnmpClient: Send + Sync { ... }
```
> COPY EXACTLY: only `async_trait` + `crate::error::AppError` + `crate::primitives::secret::Secret`. NO `ldap3`/`hickory`/`tokio`. The `no_io_deps.rs` test (`crates/trackly-core/tests/no_io_deps.rs`) will fail the build otherwise.

**Outcome-not-error pattern** (snmp.rs:59-65): `get_oids` returns `Result<Option<...>>` — unreachable is `Ok(None)`, not `Err`. For AD, mirror this with `AuthOutcome { Ok{display_name}, BadCreds, Unreachable }` returned as `Ok(AuthOutcome)`; reserve `AppError` for genuine infra faults (RESEARCH §Pattern 3). `Secret` is already I/O-free (`primitives/secret.rs:24`) so the trait can take `&Secret<String>` directly.

---

### `crates/trackly-infra/src/ad/mod.rs` (module index)

**Analog:** `crates/trackly-infra/src/snmp/mod.rs` (whole file, 19 lines)

**Copy structure verbatim** (snmp/mod.rs:1-19):
```rust
//! SNMP adapters for Trackly.
//! Runtime switching in `AppCtx::build`:
//! ```ignore
//! let snmp_client: Arc<dyn SnmpClient + Send + Sync> =
//!     if config.snmp.use_mock || std::env::var("TRACKLY_SNMP_MOCK").is_ok() {
//!         Arc::new(MockSnmpClient::default_fixtures())
//!     } else { Arc::new(RealSnmpClient) };
//! ```
pub mod mock;
pub mod real;
```
> AD version adds `pub mod discovery;` for the auto-detect helper (RESEARCH §structure).

---

### `crates/trackly-infra/src/ad/mock.rs` (infra adapter, fixtures)

**Analog:** `crates/trackly-infra/src/snmp/mock.rs` (whole file, 233 lines — read in full)

**Fixture map + `default_fixtures()` constructor** (mock.rs:29-74):
```rust
pub struct MockSnmpClient { pub fixtures: HashMap<String, PrinterFixture> }
impl MockSnmpClient {
    pub fn default_fixtures() -> Self {
        let mut map = HashMap::new();
        map.insert("192.168.1.100".into(), PrinterFixture { ... });   // ok
        map.insert("192.168.1.101".into(), PrinterFixture { ... });   // warning
        map.insert("192.168.1.102".into(), PrinterFixture { ... });   // offline (alert)
        Self { fixtures: map }
    }
}
```
> AD analog: `default_fixtures()` seeds 2 domain users (`us100` / `us200` with RU display names per RESEARCH Code Example), plus an `unreachable()` constructor for the server-down scenario.

**`#[async_trait] impl` returning outcome-not-error** (mock.rs:82-138): unknown key → `Ok(None)` for SNMP; for AD, unknown/wrong-password → `Ok(AuthOutcome::BadCreds)` (generic, no enumeration — RESEARCH note after Code Example). Reject empty password BEFORE lookup (Pitfall 1).

**`#[cfg(test)] mod tests` with `#[tokio::test]`** (mock.rs:179-232): one test per scenario (`known_returns_some`, `offline_returns_none`, `unknown_returns_none`). AD analog: `success`, `wrong_password`, `not_found`, `unreachable`, `empty_password_rejected`, `display_name_fallback` (RESEARCH Test Map → Wave 0).

---

### `crates/trackly-infra/src/ad/real.rs` (infra adapter, network I/O)

**Analog:** `crates/trackly-infra/src/snmp/real.rs` (lines 1-60 read; pattern clear)

**"Only place that imports the I/O crate" header** (real.rs:1-8):
```rust
//! CRITICAL: This module is the ONLY place in the codebase that imports `snmp2`.
//! `trackly_core::ports::snmp::SnmpClient` trait must remain snmp2-free.
//! Always wraps SNMP calls in `tokio::time::timeout` ... Timeout/error → `Ok(None)`.
```
> AD analog: the ONLY place importing `ldap3`. Wrap connect in `set_conn_timeout`/`drive!`; connect/TLS error → `Ok(AuthOutcome::Unreachable)`, rc=49 → `Ok(AuthOutcome::BadCreds)`. Full skeleton in RESEARCH §Code Examples (use `ldap_escape` on login before filter — Pitfall 5; `default-features=false, features=["tls-rustls-ring"]` — Pitfall 2).

**Struct + `#[async_trait] impl`** (real.rs:27-38): `pub struct RealSnmpClient;` → `RealAdClient { cfg: AdConfig }` (carries config, unlike the unit-struct SNMP client).

---

### `crates/trackly-infra/src/config.rs` (config section)

**Analog:** `ServerConfig` (config.rs:39-68)

**Section struct + manual `Default`** (config.rs:40-68):
```rust
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub enabled: bool, pub host: String, pub port: u16,
    pub cert_path: String,
    #[serde(default)] pub key_path: String,
}
impl Default for ServerConfig {
    fn default() -> Self { Self { enabled: false, host: "127.0.0.1".into(), port: 8443, ... } }
}
```
**Wire into root with `#[serde(default)]`** (config.rs:23-37): add `#[serde(default)] pub ad: AdConfig` to `AppConfig`. `AdConfig` fields + defaults in RESEARCH §AdConfig (port 636, name_attr "displayName", no_tls_verify false; empty host/domain/base_dn → auto-detect). Note: TOML is bootstrap-only; live AD settings live in `app_settings` (see below).

---

### `crates/trackly-app/src/context.rs` (provider/wiring)

**Analog:** SNMP runtime switch (context.rs:282-294) + `AuthService::new` (context.rs:266-270)

**Env/config mock switch** (context.rs:282-294):
```rust
let use_mock = std::env::var("TRACKLY_SNMP_MOCK").is_ok();
tracing::info!(snmp_mode = if use_mock { "mock" } else { "real" }, "SNMP client selected");
let snmp_client: Arc<dyn trackly_core::ports::snmp::SnmpClient + Send + Sync> =
    if use_mock { Arc::new(MockSnmpClient::default_fixtures()) } else { Arc::new(RealSnmpClient) };
```
> AD analog: `let use_mock = config.ad.use_mock || std::env::var("TRACKLY_AD_MOCK").is_ok();` then build `Arc<dyn AdClient>` the same way, and **inject it into `AuthService::new`** (currently auth has 3 args at context.rs:266-270 — extend the constructor; see auth.rs below). Import line to mirror: context.rs:41 `use trackly_infra::snmp::{mock::MockSnmpClient, real::RealSnmpClient};`.

---

### `crates/trackly-app/src/services/auth.rs` (service — login fallback + settings + create)

**Analog:** itself — three proven patterns in one file.

**(1) Constructor to extend** (auth.rs:108-120): `AuthService::new(writer, readers, clock)` → add `ad_client: Arc<dyn AdClient + Send + Sync>`. The struct (auth.rs:101-106) is `#[derive(Clone)]` with `Arc` fields — add an `Arc` field, Clone stays O(1).

**(2) login() + constant-time dummy-hash to PRESERVE** (auth.rs:180-205):
```rust
pub async fn login(&self, req: LoginRequest) -> Result<UserDto, AppError> {
    // CR-05: устранение user-enumeration timing oracle.
    let (hash, user_known) = match self.get_password_hash(&req.login).await {
        Ok(h) => (h, true),
        Err(AppError::Unauthorized) => (dummy_password_hash().to_string(), false),
        Err(e) => return Err(e),
    };
    let password = Secret::new(req.password.clone());
    let verified = tokio::task::spawn_blocking(move || verify_password(&password, &hash)).await...?;
    if !user_known || !verified { return Err(AppError::Unauthorized); }  // ← AD fallback inserts HERE
    self.get_by_login(&req.login).await
}
```
> CRITICAL (RESEARCH §Pattern 4): keep this local path verbatim, ADD the AD branch after it. Do not short-circuit so the dummy-hash always runs. Reject empty password BEFORE bind (Pitfall 1). On `AuthOutcome::Ok{display_name}` → `on_ad_bind_success` which needs `find_user_any_state(login)` (new read, NO `is_active=1 AND deleted_at_utc IS NULL` filter — RESEARCH Open Q3) to branch active→session / blocked→BlockedScreen / none→register-mode.

**(3) app_settings get/set upsert** (auth.rs:799-847) — model for `ad_enabled`, `ad_auto_accept`, and AD connection fields:
```rust
pub async fn get_desktop_lock_enabled(&self) -> Result<bool, AppError> {
    // SELECT value FROM app_settings WHERE key = 'desktop_lock_enabled'
    // QueryReturnedNoRows → Ok(false)  (default)
}
pub async fn set_desktop_lock_enabled(&self, enabled, caller) -> Result<(), AppError> {
    authorize(caller, &Action::ManageSettings)?;
    self.writer.execute(move |conn| {
        conn.execute(
          "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
           VALUES ('desktop_lock_enabled', ?1, ?2, ?2) \
           ON CONFLICT(key) DO UPDATE SET value = ?1, updated_at_utc = ?2",
          rusqlite::params![value, now]).map(|_| ()).map_err(map_rusqlite)
    }).await
}
```
> Reuse the exact upsert SQL + `ManageSettings` authorize gate for every AD setting key.

**(4) create_user / revive single-writer pattern** (auth.rs:214-302) — model for auto-register + approve + restoration:
```rust
let id = self.writer.execute(move |conn| {
    let tx = conn.transaction().map_err(map_rusqlite)?;
    // SELECT id, deleted_at_utc FROM users WHERE login = ?1  → revive vs insert vs Conflict
    // ... UPDATE ... is_active=1, deleted_at_utc=NULL, version = version + 1   (revive)
    // ... INSERT INTO users (... is_active, ...) VALUES (..., 1, ...)          (create)
    // INSERT INTO audit_log (entity_type='user', entity_id, action, user_id, ...)
    tx.commit()...; Ok(id)
}).await?;
```
> AD auto-register: same writer txn but `password_hash=NULL`, `ad_user=1`, `role='employee'`. Pending mode: insert `is_active=0` AD user so the FK `requests.requested_by_user_id` resolves (Pitfall 4 / Open Q2). Approve = revive/activate path; Reject(auto) = soft-delete. ALL through `self.writer.execute` (single-writer invariant).

---

### `crates/trackly-app/src/services/request_service.rs` (service — ad_register lifecycle)

**Analog:** `RequestService::create` (request_service.rs:168-221) + `transition` (228+)

**create() with authorize + writer txn + audit + WS broadcast** (request_service.rs:168-221):
```rust
pub async fn create(&self, payload: RequestCreateDto, caller: &Identity) -> Result<RequestDto, AppError> {
    authorize(caller, &Action::CreateRequest)?;
    let new = RequestNew { request_type: payload.request_type.clone(),
                           requested_by_user_id: user_id.unwrap_or(1), ... };
    let request_id = self.writer.execute(move |conn| {
        let tx = conn.transaction()?;
        let id = request_repo.insert_in_tx(&tx, &new, now)?;
        audit_repo.insert(&tx, AuditEntry { entity_type: "request", action: "create", ... })?;
        tx.commit()?; Ok(id)
    }).await?;
    let _ = self.ws_tx.send(WsEvent::NewRequest { request_id, request_type, requester_name });
    Ok(dto)
}
```
> AD `ad_register` request reuses this verbatim (`request_type="ad_register"`). The approve path reuses `transition()` (Accept/Reject ops, request_service.rs:228+, optimistic-lock via `version`). Admin-only visibility = filter `request_type='ad_register'` at the **list** level + `identity.role=='admin'` (REQ-06) — extend `RequestFilter` / list query, do not rely on row hide.

---

### `crates/trackly-core/src/domain/requests.rs` (model)

**Analog:** `RequestRow` / `RequestNew` / `RequestFilter` (same file, read in full)

- `RequestRow.request_type` already documents `"ad_register"` (requests.rs:16). `RequestNew.request_type` doc (requests.rs:43) must be widened to include it.
- For the **restoration sub-flag** (D-REG-03): RESEARCH recommends a nullable discriminator. If a column is added, mirror the field-doc style here (`pub ad_subtype: Option<String>` — `register`/`restore`). Pure data only — NO serde derives in this file (header rule, requests.rs:3-4).

---

### `crates/trackly-app/src/dto/auth.rs` (dto, one-DTO-two-transports)

**Analog:** `LoginRequest` (dto/auth.rs:19-23) + `NetworkSettingsDto` (dto/auth.rs:108-126)

**Login DTO to extend** (dto/auth.rs:19-23):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LoginRequest { pub login: String, pub password: String }
```
> Add `#[serde(default)] pub remember: bool` for «Запомнить меня» (D-UX-02). Password stays plain in DTO; `AuthService` wraps in `Secret` (existing comment dto/auth.rs:17-18).

**Settings DTO pattern** (dto/auth.rs:108-126): mirror `NetworkSettingsDto` for an `AdSettingsDto` (enabled, auto_accept, host, port→`#[specta(type = i32)]`, domain, base_dn, name_attr, no_tls_verify). `i64`→`#[specta(type = i32)]` rule (header dto/auth.rs:1-9). Serde roundtrip + snake_case + no-secret tests already templated (dto/auth.rs:159-243).

---

### `crates/trackly-app/src/http/auth.rs` (axum transport) + `tauri_cmds/auth.rs` (tauri transport)

**Analog:** `build_auth_login` + `public_router` (http/auth.rs:99-125, 215-219) and `build_auth_login_tauri` (tauri_cmds/auth.rs:24-26)

**Thin adapter + session-fixation flush-before-insert** (http/auth.rs:99-125):
```rust
pub async fn build_auth_login(ctx, session, payload) -> Result<UserDto, AppError> {
    let user = ctx.auth.login(payload.req).await?;
    session.flush().await...?;                 // T-05-SF: flush BEFORE insert
    session.insert("identity", SessionIdentity::from(&identity)).await...?;
    Ok(user)
}
```
> AD login flows through the SAME `ctx.auth.login` — no new handler needed for the happy path (D-UX-01). «Запомнить меня» wires the cookie policy here (persistent vs session) per D-UX-02. New endpoints needed: AD settings get/set, restoration-request create, approve-with-role — add to both `public_router`/`protected_router` (http/auth.rs:215-226) AND the matching `build_*_tauri` helper (tauri_cmds/auth.rs:24-40) so one DTO serves both transports. NOTE: AD login is web-only (D-AD-01) but the registration/approval/settings commands are admin desktop+web → expose on both.

**Tauri thin wrapper** (tauri_cmds/auth.rs:6-7 header): `build_*` helper + `#[tauri::command] #[specta::specta]` wrapper; `#[specta::specta]` AFTER `#[tauri::command]`.

---

### `migrations/V028__ad_register_subtype.sql` (new — ONLY IF column chosen for D-REG-03)

**Analog:** `migrations/V019__users_is_active.sql` (whole file)

```sql
-- V019: Add is_active column to users table.
ALTER TABLE users ADD COLUMN is_active INTEGER NOT NULL DEFAULT 1;
PRAGMA user_version = 19;
```
> If the planner picks the column option for restoration, mirror exactly: `ALTER TABLE requests ADD COLUMN ad_subtype TEXT NULL;` + `PRAGMA user_version = 28;` (next free number — last is V027). NULLABLE → no DEFAULT needed. This AVOIDS a CHECK-constraint rebuild (the `request_type` CHECK already allows `ad_register`, V006). RESEARCH recommends this over a new `request_type`. **No migration is needed for users or the base ad_register type** (V002 + V006 already support them).

---

### `ui/src/features/settings/ActiveDirectorySettings.svelte` (new component)

**Analog:** `ui/src/features/settings/NetworkSettings.svelte` (read lines 1-90; full structure clear)

**load/save + apiCall + pushToast lifecycle** (NetworkSettings.svelte:38-81):
```svelte
import { apiCall } from '$lib/api/client';
import { pushToast } from '$lib/stores/toast.svelte';
let settings = $state<NetworkSettingsDto>({ ... });
async function loadSettings() { settings = await apiCall<...>('settings_get_network', {}); }
onMount(() => loadSettings());
async function saveSettings() {
  await apiCall<void>('settings_set_network', { patch: {...} });
  pushToast('success', 'Настройки сохранены');
}
```
> COPY this load/save/toast skeleton. Endpoints become `settings_get_ad` / `settings_set_ad`. UI-SPEC Screen 4: enable toggle, registration-mode radios, `<details>` «Расширенные», save + «Проверить подключение» (toast). Reuse `.checkbox-label` (accent-color), `.settings-section`, `.section-title`, `.helper-text`, `.form-grid`, `.save-row` classes from this file. `max-width: 640px`.

---

### `ui/src/features/settings/SettingsSubNav.svelte` (modify)

**Analog:** itself (SECTIONS array, SettingsSubNav.svelte:5-12)
```js
const SECTIONS = [ { key: 'network', label: 'Сеть' }, ... ] as const;
```
> Insert `{ key: 'ad', label: 'Active Directory' }`. One-line change; tab rendering + active-tab accent styling (lines 22-74) unchanged. Parent that switches on `activeSection` must render `<ActiveDirectorySettings>` for `'ad'`.

---

### `ui/src/features/auth/{LoginPage,PendingScreen,BlockedScreen}.svelte`

**Analog:** `ui/src/features/auth/LoginPage.svelte` (`.login-container`/`.login-card` shell, lines 52-94 + styles 102-184)

**Login shell + apiCall + error block** (LoginPage.svelte:2,32,52-94):
```svelte
import { apiCall } from '$lib/api/client';
const user = await apiCall<UserDto>('auth_login', { req: { login: login.trim(), password } });
<div class="login-container"><div class="login-card">
  <h1 class="login-title">Вход в систему</h1>
  <input class="form-input" .../>  <span class="field-error">...</span>
  <div class="server-error">{serverError}</div>
  <button class="btn-submit" type="submit" disabled={loading}>...</button>
</div></div>
```
> Screen 1: add «Запомнить меня» checkbox (`.checkbox-label` pattern from NetworkSettings, `accent-color: var(--color-accent)`) → send `remember` in the `req`; add format hint helper; generic error `Неверный логин или пароль` vs distinct `Сервер аутентификации недоступен` for `Unreachable`; reserved disabled SSO button (muted, no handler — D-UX-03). Screens 2 (Pending) & 3 (Blocked) REUSE the same `.login-card` shell (360px centered); Blocked has a primary `.btn-submit` «Запросить восстановление доступа» → restoration apiCall, then in-place confirmation state + success toast (UI-SPEC Screens 2-3).

---

### `ui/src/features/requests/{RequestListRow,RequestDetail,RequestsList,RequestsMasterDetail}.svelte`

**Analog:** `ui/src/features/requests/RequestListRow.svelte` (typeLabel/shortDesc/Badge, lines 4,15-45,81-85)

**typeLabel + Badge derivation to extend** (RequestListRow.svelte:38-44, 81-85):
```svelte
import Badge from '$lib/components/Badge.svelte';
const typeLabel = $derived(
  request.requestType === 'cartridge_replace' ? 'Замена картриджа' : 'Свободная форма');
const shortDesc = $derived(...);
<Badge variant="default" size="sm">{typeLabel}</Badge>
<Badge variant={statusVariant}>{statusLabel}</Badge>
```
> Extend `typeLabel`: `ad_register` → `Регистрация AD`; add a `Восстановление доступа` chip (`Badge variant="warning"`) for the restore sub-flag (UI-SPEC Screen 5). `shortDesc` → requested ФИО. Admin-only filter at list level (RequestsList/MasterDetail) — `identity.role === 'admin'` (REQ-06). RequestDetail: approve→Modal with role `Select` (default `Сотрудник`), reject→existing reject-confirm Modal with destructive copy (UI-SPEC Screens 5/5b + Destructive table).

---

## Shared Patterns

### Mock-via-trait runtime switch (config flag + env)
**Source:** `crates/trackly-core/src/ports/snmp.rs` + `crates/trackly-infra/src/snmp/{mod,real,mock}.rs` + `crates/trackly-app/src/context.rs:282-294`
**Apply to:** the entire `AdClient` triad.
```rust
let use_mock = config.ad.use_mock || std::env::var("TRACKLY_AD_MOCK").is_ok();
let ad_client: Arc<dyn AdClient + Send + Sync> =
    if use_mock { Arc::new(MockAdClient::default_fixtures()) } else { Arc::new(RealAdClient::new(config.ad.clone())) };
```

### Single-writer for all mutations
**Source:** `auth.rs:234-299` (`self.writer.execute(|conn| { let tx = conn.transaction()?; ...; tx.commit() })`)
**Apply to:** AD user auto-create, pending inactive-user insert, approve (activate/revive), reject (soft-delete), every `app_settings` write. NEVER write outside `WriterHandle::execute` (CLAUDE.md single-writer invariant).

### app_settings key/value upsert
**Source:** `auth.rs:799-847` (`ON CONFLICT(key) DO UPDATE SET value=?1, updated_at_utc=?2`) gated by `authorize(caller, &Action::ManageSettings)`.
**Apply to:** `ad_enabled`, `ad_auto_accept`, and AD connection fields (DB is the live source-of-truth; TOML is bootstrap only).

### One DTO, two transports
**Source:** `dto/auth.rs` (shared types) → `http/auth.rs` (`build_*` + `public_router`/`protected_router`) → `tauri_cmds/auth.rs` (`build_*_tauri` + `#[tauri::command]`).
**Apply to:** AD settings get/set, restoration request, approve-with-role. Login itself is web-only (D-AD-01) but flows through the shared `ctx.auth.login`.

### Secret<T> for the AD password
**Source:** `crates/trackly-core/src/primitives/secret.rs:24-39` (`Secret::new` / `.expose()`; no Debug/Serialize; zeroize-on-drop).
**Apply to:** wrap the AD password in `Secret<String>` at the service boundary; pass `&Secret<String>` into `AdClient::authenticate`; only `.expose()` at the `simple_bind` call site. `password_hash=NULL` in DB. Already used in `login` (auth.rs:191) and documented for LDAP in secret.rs:16-17.

### Constant-time anti-enumeration (preserve, don't rebuild)
**Source:** `auth.rs:64-75` (`dummy_password_hash`) + `auth.rs:180-205` (login dual-branch).
**Apply to:** keep the local dummy-hash verify intact; the AD fallback is ADDED after it. Generic `Unauthorized` for both bad-creds and unknown.

### I/O-free core gate
**Source:** `crates/trackly-core/tests/no_io_deps.rs` (enforces snmp.rs has no tokio/snmp2).
**Apply to:** `ports/ad.rs` — only `async_trait` + `crate::error::AppError` + `crate::primitives::secret::Secret`. ldap3/hickory/tokio must stay in `trackly-infra`.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| — | — | — | None. Every Phase 9 file maps to a verified in-repo analog. |

**Partial-analog note:** `crates/trackly-infra/src/ad/discovery.rs` (env + DNS SRV → derived domain/DC/base-DN) has no exact functional twin. Closest model is the pure-transform + manual-`Default` style in `config.rs`; the base-DN derivation (`corp.local` → `dc=corp,dc=local`) is a unit-testable string transform (RESEARCH §Pattern 5). The hickory `srv_lookup` call is genuinely new — follow RESEARCH §Pattern 5 directly; on dev macOS this code is never exercised (`TRACKLY_AD_MOCK=1`).

---

## Metadata

**Analog search scope:** `crates/trackly-core/src/{ports,domain,primitives}`, `crates/trackly-infra/src/{snmp,config}`, `crates/trackly-app/src/{services,dto,http,tauri_cmds,context}`, `migrations/`, `ui/src/features/{auth,settings,requests}`.
**Files scanned:** ~18 read in full or in targeted ranges; directory listings for migrations + ui features.
**Pattern extraction date:** 2026-06-19
