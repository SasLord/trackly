# Phase 5: Авторизация, локальные пользователи и серверный режим — Pattern Map

**Mapped:** 2026-06-13
**Files analyzed:** 28 (new/modified)
**Analogs found:** 26 / 28

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/trackly-core/src/auth.rs` | domain/utility | transform | `crates/trackly-core/src/error.rs` | role-match (pure domain type, no I/O) |
| `crates/trackly-app/src/services/auth.rs` | service | CRUD + request-response | `crates/trackly-app/src/services/device_service.rs` | exact |
| `crates/trackly-app/src/server/rusqlite_session_store.rs` | service | CRUD (async↔sync bridge) | `crates/trackly-infra/src/db/writer_worker.rs` + `pools.rs` | role-match (writer/reader pool pattern) |
| `crates/trackly-app/src/server/tls.rs` | utility | transform | `crates/trackly-core/src/primitives/secret.rs` | partial (utility module shape) |
| `crates/trackly-app/src/server/mod.rs` | service | event-driven | `crates/trackly-app/src/shutdown.rs` | role-match (CancellationToken lifecycle) |
| `crates/trackly-app/src/http/auth.rs` | controller | request-response | `crates/trackly-app/src/http/devices.rs` | exact |
| `crates/trackly-app/src/http/users.rs` | controller | CRUD | `crates/trackly-app/src/http/devices.rs` | exact |
| `crates/trackly-app/src/http/settings.rs` | controller | request-response | `crates/trackly-app/src/http/health.rs` | role-match |
| `crates/trackly-app/src/tauri_cmds/auth.rs` | controller | request-response | `crates/trackly-app/src/tauri_cmds/devices.rs` | exact |
| `crates/trackly-app/src/tauri_cmds/users.rs` | controller | CRUD | `crates/trackly-app/src/tauri_cmds/devices.rs` | exact |
| `crates/trackly-app/src/dto/auth.rs` | model | transform | `crates/trackly-app/src/dto/device.rs` | exact |
| `crates/trackly-app/src/context.rs` (modified) | config | — | self (add fields) | — |
| `crates/trackly-app/src/http/mod.rs` (modified) | config | — | self (add live bind) | — |
| `crates/trackly-app/src/main.rs` (modified) | config | event-driven | `crates/trackly-app/src/main.rs` + `shutdown.rs` | self |
| `migrations/V018__auth_settings.sql` | migration | — | `migrations/V016__cartridges_kind_color_settings.sql` | exact |
| `crates/trackly-app/tests/users_crud.rs` | test | CRUD | `crates/trackly-app/tests/devices_crud.rs` | exact |
| `crates/trackly-app/tests/role_endpoint_matrix.rs` | test | request-response | `crates/trackly-app/tests/devices_http_smoke.rs` | exact |
| `crates/trackly-app/tests/session_survives_restart.rs` | test | CRUD | `crates/trackly-app/tests/devices_http_smoke.rs` | role-match |
| `crates/trackly-app/tests/tls_server_smoke.rs` | test | event-driven | `crates/trackly-app/tests/acts_http_smoke.rs` | role-match |
| `crates/trackly-app/tests/server_hot_toggle.rs` | test | event-driven | `crates/trackly-app/tests/health_smoke.rs` | partial |
| `crates/trackly-app/tests/security_headers.rs` | test | request-response | `crates/trackly-app/tests/devices_http_smoke.rs` | role-match |
| `crates/trackly-app/tests/graceful_shutdown_drain.rs` | test | event-driven | `crates/trackly-app/shutdown.rs` | role-match |
| `ui/src/lib/stores/auth.svelte.ts` | store | event-driven | `ui/src/lib/stores/toast.svelte.ts` | exact |
| `ui/src/lib/api/client.ts` (modified) | utility | request-response | self | — |
| `ui/src/features/layout/sidebar-config.ts` (modified) | config | — | self | — |
| `ui/src/features/auth/LoginPage.svelte` | component | request-response | `ui/src/features/devices/DeviceFormModal.svelte` | role-match |
| `ui/src/features/auth/FirstRunWizard.svelte` | component | request-response | `ui/src/features/devices/DeviceFormModal.svelte` | role-match |
| `ui/src/features/users/UsersPage.svelte` + sub-components | component | CRUD | `ui/src/features/devices/DevicesPage.svelte` | exact |
| `ui/src/features/settings/NetworkSettings.svelte` | component | request-response | `ui/src/features/devices/DeviceFormModal.svelte` | role-match |
| `ui/src/pages/SettingsPage.svelte` (modified) | component | — | `ui/src/pages/UsersPage.svelte` (placeholder shape) | exact |
| `ui/src/routes.ts` (modified) | config | — | self | — |
| `ui/src/App.svelte` (modified) | component | event-driven | self | — |

---

## Pattern Assignments

### `crates/trackly-core/src/auth.rs` (domain, transform)

**Analog:** `crates/trackly-core/src/error.rs`

**Imports pattern** (lines 1-8 of error.rs):
```rust
use serde::{Serialize, Serializer};
use serde_json::{json, Value};
```

For auth.rs, no serde needed on domain types (Role, Action, Identity are internal). Pattern: pure enum + plain `fn authorize()` returning `Result<(), AppError>` — zero I/O dependencies, matching the `trackly-core` "no I/O" constraint (`crates/trackly-core/tests/no_io_deps.rs`).

**Core pattern** — derive shape from `error.rs` lines 33-100:
```rust
// New file: crates/trackly-core/src/auth.rs
use crate::error::AppError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Role {
    Admin,
    Manager,
    Employee,
}

impl Role {
    /// Parse from DB TEXT value ('admin' | 'manager' | 'employee').
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "admin" => Ok(Self::Admin),
            "manager" => Ok(Self::Manager),
            "employee" => Ok(Self::Employee),
            _ => Err(AppError::Validation {
                field: "role".to_string(),
                message: format!("Unknown role: {s}"),
            }),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Manager => "manager",
            Self::Employee => "employee",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Identity {
    pub user_id: Option<i64>, // None = trusted-desktop (unlocked, D-Desktop-01)
    pub role: Role,
}

impl Identity {
    /// Construct trusted-desktop identity (unlocked mode).
    pub fn trusted_admin() -> Self {
        Self { user_id: None, role: Role::Admin }
    }
}

#[derive(Clone, Debug)]
pub enum Action {
    ManageUsers,       // admin only
    ManageSettings,    // admin only
    MutateDevices,     // admin + manager
    MutateActs,        // admin + manager
    MutateCartridges,  // admin + manager
    ReadData,          // all authenticated
    CreateRequest,     // all authenticated
}

/// Source of truth for RBAC. Called by BOTH Tauri commands and axum handlers.
/// No I/O — pure function, fast, testable in isolation.
pub fn authorize(identity: &Identity, action: &Action) -> Result<(), AppError> {
    let allowed = match action {
        Action::ManageUsers | Action::ManageSettings => identity.role == Role::Admin,
        Action::MutateDevices | Action::MutateActs | Action::MutateCartridges =>
            matches!(identity.role, Role::Admin | Role::Manager),
        Action::ReadData | Action::CreateRequest => true,
    };
    if allowed { Ok(()) } else { Err(AppError::Forbidden) }
}
```

**Unit test pattern** — mirror `error.rs` lines 193-342 (inline `#[cfg(test)]` module):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_can_manage_users() {
        let id = Identity { user_id: Some(1), role: Role::Admin };
        assert!(authorize(&id, &Action::ManageUsers).is_ok());
    }

    #[test]
    fn employee_cannot_manage_users() {
        let id = Identity { user_id: Some(2), role: Role::Employee };
        assert_eq!(authorize(&id, &Action::ManageUsers), Err(AppError::Forbidden));
        // ... role×action matrix
    }
}
```

---

### `crates/trackly-app/src/services/auth.rs` (service, CRUD + request-response)

**Analog:** `crates/trackly-app/src/services/device_service.rs`

**Imports pattern** (lines 14-41 of device_service.rs):
```rust
use std::sync::Arc;
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
```

Add for auth.rs:
```rust
use trackly_core::auth::{Identity, Role};
use trackly_core::primitives::secret::Secret;
use crate::dto::auth::{LoginRequest, UserDto, UserNew, UserPatch};
```

**Service struct pattern** (lines 43-71 of device_service.rs):
```rust
#[derive(Clone)]
pub struct AuthService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
}

impl AuthService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self { writer, readers, clock }
    }
```

**Read pattern** (lines 155-166 of device_service.rs — `get` method):
```rust
pub async fn get_by_login(&self, login: &str) -> Result<UserDto, AppError> {
    let readers = self.readers.clone();
    let login = login.to_string();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        // query users by login
        conn.query_row(
            "SELECT id, login, full_name, role, email, is_active, ... \
             FROM users WHERE login = ?1 AND deleted_at_utc IS NULL",
            rusqlite::params![login],
            |row| { /* map to UserDto */ },
        )
        .map_err(|e| AppError::NotFound { entity: "user", id: 0 })
    })
    .await
    .map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking: {e}") })?
}
```

**Write pattern** (lines 109-152 of device_service.rs — `create` method):
```rust
pub async fn create_user(&self, new: UserNew, caller: &Identity) -> Result<UserDto, AppError> {
    // authorize before mutating
    trackly_core::auth::authorize(caller, &trackly_core::auth::Action::ManageUsers)?;
    Self::validate_new(&new)?;

    let now = self.clock.unix_seconds();
    let hash = tokio::task::spawn_blocking({
        let pw = new.password.clone(); // Secret<String> — expose only in spawn_blocking
        move || hash_password(&pw)
    }).await.map_err(|e| AppError::Internal { source_chain: e.to_string() })??;

    let id = self.writer.execute(move |conn| {
        let tx = conn.transaction().map_err(map_rusqlite)?;
        tx.execute(
            "INSERT INTO users (login, full_name, password_hash, role, email, is_active, \
             created_at_utc, updated_at_utc, version) \
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6, 1)",
            rusqlite::params![new.login, new.full_name, hash, new.role.as_str(),
                              new.email, now],
        ).map_err(map_rusqlite)?;
        let id = conn.last_insert_rowid();
        tx.commit().map_err(map_rusqlite)?;
        Ok(id)
    }).await?;

    self.get_by_id(id).await
}
```

**spawn_blocking for argon2 (critical — blocks tokio if called in async):**
```rust
// login() method — from RESEARCH Pattern 5
pub async fn login(&self, req: LoginRequest) -> Result<UserDto, AppError> {
    // 1. Load hash via reader pool
    let hash = self.get_password_hash(&req.login).await?;
    let pw = req.password; // Secret<String>
    // 2. verify in spawn_blocking — argon2 is CPU-bound ~50ms
    let ok = tokio::task::spawn_blocking(move || {
        verify_password(&pw, &hash)
    }).await.map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking: {e}") })?;
    if !ok { return Err(AppError::Unauthorized); }
    self.get_by_login(&req.login).await
}
```

---

### `crates/trackly-app/src/server/rusqlite_session_store.rs` (service, CRUD async↔sync bridge)

**Analog:** `crates/trackly-infra/src/db/writer_worker.rs` + `crates/trackly-infra/src/db/pools.rs`

**Writer pattern** (writer_worker.rs lines 76-118):
```rust
// All writes go through writer.execute(closure) — same as device_service.rs
self.writer.execute(move |conn| {
    conn.execute(
        "INSERT OR REPLACE INTO sessions (id, data, expiry_date) VALUES (?1, ?2, ?3)",
        rusqlite::params![id_bytes, data, expiry_ts],
    ).map_err(|e| AppError::Internal { source_chain: e.to_string() })?;
    Ok(())
}).await.map_err(|e| session_store::Error::Backend(e.to_string()))
```

**Reader pattern** (pools.rs lines 74-97 — `acquire()` inside `spawn_blocking`):
```rust
// Reads use spawn_blocking + readers.acquire() — same as device_service.rs get()
let readers = self.readers.clone();
tokio::task::spawn_blocking(move || {
    let conn = readers.acquire(); // RAII guard, returns to pool on drop
    conn.query_row(
        "SELECT data FROM sessions WHERE id = ?1 AND expiry_date > ?2",
        rusqlite::params![id_bytes, now],
        |row| row.get::<_, Vec<u8>>(0),
    ).optional()
}).await.map_err(|e| session_store::Error::Backend(e.to_string()))?
 .map_err(|e| session_store::Error::Backend(e.to_string()))
```

**Session ID serialization (from RESEARCH Pitfall 3):**
```rust
// Id is i128 — must use to_le_bytes(), NOT base64/string
let id_bytes = session_id.0.to_le_bytes().to_vec();
// Bind as: rusqlite::params![id_bytes] — stored as BLOB in V010 sessions table
```

---

### `crates/trackly-app/src/server/tls.rs` (utility, transform)

**Analog:** `crates/trackly-core/src/primitives/secret.rs` (utility module shape — no I/O deps in struct, pure transform)

**Module shape** (secret.rs lines 19-51):
```rust
// Utility struct + pure functions, no async
pub struct TlsBundle {
    pub acceptor: TlsAcceptor,
    pub fingerprint_hex: String,
    pub cert_pem: String,
    pub key_pem: String,
}

// Generate self-signed cert (rcgen 0.14 API — RESEARCH Pattern 3)
pub fn generate_self_signed(host: &str) -> anyhow::Result<TlsBundle> { ... }

// Load user-supplied cert from PEM file (rustls-pemfile 2.x)
pub fn load_from_pem(cert_path: &str, key_path: &str) -> anyhow::Result<TlsBundle> { ... }

// Build rustls::ServerConfig from TlsBundle
pub fn build_server_config(bundle: &TlsBundle) -> Arc<rustls::ServerConfig> { ... }
```

---

### `crates/trackly-app/src/server/mod.rs` (service, event-driven)

**Analog:** `crates/trackly-app/src/shutdown.rs`

**CancellationToken pattern** (shutdown.rs lines 7-25):
```rust
// shutdown.rs pattern: spawn async task that awaits cancellation
pub fn install_signal_handler(token: CancellationToken) {
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => token.cancel(),
            Err(e) => tracing::error!("failed to install Ctrl-C handler: {e}"),
        }
    });
}
```

**Server task lifecycle** — from RESEARCH Pattern 4, grounded in shutdown.rs shape:
```rust
// server/mod.rs — hot start/stop uses CHILD token (never cancel AppCtx.shutdown)
pub async fn start_server(
    app: Router,
    addr: SocketAddr,
    tls: TlsAcceptor,
    shutdown: CancellationToken, // child of AppCtx.shutdown
) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    loop {
        tokio::select! {
            result = listener.accept() => {
                let (stream, _) = result?;
                let tls = tls.clone();
                let app = app.clone();
                tokio::spawn(async move {
                    match tls.accept(stream).await {
                        Ok(tls_stream) => { /* serve with hyper */ }
                        Err(e) => tracing::warn!("TLS accept error: {e}"),
                    }
                });
            }
            _ = shutdown.cancelled() => {
                tracing::info!("server shutdown signal received");
                break;
            }
        }
    }
    Ok(())
}
```

---

### `crates/trackly-app/src/http/auth.rs` (controller, request-response)

**Analog:** `crates/trackly-app/src/http/devices.rs`

**Imports pattern** (devices.rs lines 8-26):
```rust
use axum::{extract::State, routing::post, Json, Router};
use crate::context::AppCtx;
use crate::error_axum::AppErrorResponse;
use crate::tauri_cmds::auth::{build_auth_login, build_auth_logout, build_auth_me, build_auth_status};
```

**Handler pattern** (devices.rs lines 119-128 — `handler_list`):
```rust
// Every axum handler: State(ctx) + Json(payload) → Result<Json<T>, AppErrorResponse>
pub async fn handler_login(
    State(ctx): State<AppCtx>,
    session: Session,               // tower-sessions extractor
    Json(payload): Json<LoginRequest>,
) -> Result<Json<UserDto>, AppErrorResponse> {
    Ok(Json(
        build_auth_login(&ctx, session, payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}
```

**Router pattern** (devices.rs lines 315-347):
```rust
pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/auth_login", post(handler_login))
        .route("/api/v1/auth_logout", post(handler_logout))
        .route("/api/v1/auth_me", post(handler_me))
        .route("/api/v1/auth_status", post(handler_status))
        // Note: login route is OUTSIDE session middleware (see http/mod.rs)
}
```

**Key difference from devices.rs:** auth handlers receive `Session` extractor from tower-sessions; logout calls `session.flush().await` (not just deleting cookie).

---

### `crates/trackly-app/src/http/users.rs` (controller, CRUD)

**Analog:** `crates/trackly-app/src/http/devices.rs` (exact match)

**Payload structs pattern** (devices.rs lines 30-113):
```rust
// Per-endpoint payload structs with #[derive(serde::Deserialize)]
#[derive(serde::Deserialize)]
pub struct ListPayload { pub filter: UserFilter, pub pagination: Pagination }

#[derive(serde::Deserialize)]
pub struct CreatePayload { pub user: UserNew }

#[derive(serde::Deserialize)]
pub struct UpdatePayload { pub id: i64, pub version: i64, pub patch: UserPatch }

#[derive(serde::Deserialize)]
pub struct DeletePayload { pub id: i64, pub version: i64 }
```

**Handler pattern** (devices.rs lines 141-151):
```rust
pub async fn handler_create(
    State(ctx): State<AppCtx>,
    session: Session,               // for identity extraction
    Json(payload): Json<CreatePayload>,
) -> Result<Json<UserDto>, AppErrorResponse> {
    Ok(Json(
        build_users_create(&ctx, session, payload.user)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}
```

**Router pattern** (devices.rs lines 315-347):
```rust
pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/users_list", post(handler_list))
        .route("/api/v1/users_create", post(handler_create))
        .route("/api/v1/users_update", post(handler_update))
        .route("/api/v1/users_delete", post(handler_delete))
        .route("/api/v1/users_change_password", post(handler_change_password))
}
```

---

### `crates/trackly-app/src/http/settings.rs` (controller, request-response)

**Analog:** `crates/trackly-app/src/http/health.rs`

**Handler pattern** (health.rs lines 14-16):
```rust
pub async fn handler_get_network(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<NetworkSettingsDto>, AppErrorResponse> {
    Ok(Json(build_settings_get_network(&ctx, session).await.map_err(AppErrorResponse::from)?))
}

pub async fn handler_set_network(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<NetworkSettingsPatch>,
) -> Result<Json<ServerStatusDto>, AppErrorResponse> {
    Ok(Json(build_settings_set_network(&ctx, session, payload).await.map_err(AppErrorResponse::from)?))
}
```

**Router pattern** (health.rs lines 21-22):
```rust
pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/settings_get_network", post(handler_get_network))
        .route("/api/v1/settings_set_network", post(handler_set_network))
        .route("/api/v1/server_toggle", post(handler_server_toggle))
        .route("/api/v1/server_status", post(handler_server_status))
}
```

---

### `crates/trackly-app/src/tauri_cmds/auth.rs` + `tauri_cmds/users.rs` (controllers, request-response + CRUD)

**Analog:** `crates/trackly-app/src/tauri_cmds/devices.rs`

**build_* helper pattern** (devices.rs lines 21-80):
```rust
// Every build_* function: takes &AppCtx + params, delegates to service
// Used by BOTH Tauri command wrapper AND axum handler
pub async fn build_auth_login(
    ctx: &AppCtx,
    session: Session,
    req: LoginRequest,
) -> Result<UserDto, AppError> {
    let user_dto = ctx.auth.login(req).await?;
    // Store identity in session
    session.insert("identity", Identity {
        user_id: Some(user_dto.id),
        role: Role::from_str(&user_dto.role)?,
    }).await.map_err(|e| AppError::Internal { source_chain: e.to_string() })?;
    Ok(user_dto)
}

pub async fn build_auth_logout(ctx: &AppCtx, session: Session) -> Result<(), AppError> {
    session.flush().await // removes from rusqlite sessions table
        .map_err(|e| AppError::Internal { source_chain: e.to_string() })
}

pub async fn build_auth_status(ctx: &AppCtx) -> Result<AuthStatusDto, AppError> {
    let needs_bootstrap = ctx.auth.needs_bootstrap().await?;
    Ok(AuthStatusDto { needs_bootstrap, user: None }) // user populated from session in axum
}
```

**Tauri command wrapper pattern** — from specta_export.rs / devices.rs:
```rust
#[tauri::command]
#[specta::specta]
pub async fn auth_login(
    ctx: tauri::State<'_, AppCtx>,
    req: LoginRequest,
) -> Result<UserDto, AppError> {
    // Tauri transport: identity from trusted-admin or from desktop session
    let identity = ctx.auth.desktop_identity().await;
    // For login itself, no prior authorize needed — it IS the auth
    build_auth_login_tauri(&ctx, req, identity).await
}
```

---

### `crates/trackly-app/src/dto/auth.rs` (model, transform)

**Analog:** `crates/trackly-app/src/dto/device.rs`

**DTO pattern** (device.rs lines 32-60):
```rust
// All DTOs: #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
// snake_case fields — NO rename_all = "camelCase" (PATTERNS.md project convention)
// i64 fields annotated with #[specta(type = i32)] per project convention

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct UserDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub version: i64,
    pub login: String,
    pub full_name: String,
    pub role: String,                  // 'admin' | 'manager' | 'employee'
    pub email: Option<String>,
    pub is_active: bool,
    #[specta(type = i32)]
    pub created_at_utc: i64,
    #[specta(type = i32)]
    pub updated_at_utc: i64,
    // NOTE: password_hash NEVER included in DTO
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LoginRequest {
    pub login: String,
    pub password: String, // plain — hashed in AuthService, never stored as plain
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UserNew {
    pub login: String,
    pub full_name: String,
    pub password: String, // AuthService wraps in Secret<String> before hashing
    pub role: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AuthStatusDto {
    pub needs_bootstrap: bool,
    pub user: Option<UserDto>,
}
```

---

### `crates/trackly-app/src/context.rs` (modified — add auth + server_ctl fields)

**Analog:** self (add to existing pattern)

**Extension pattern** (context.rs lines 60-77 — existing service fields):
```rust
// In AppCtx struct — add after cartridges field (line 76):
/// Auth service — login/logout, user CRUD, argon2id hashing, RBAC authorize().
/// Added in Phase 5.
pub auth: Arc<AuthService>,
/// Server lifecycle controller — sub-CancellationToken + JoinHandle.
/// None = server not running. Guarded by Mutex for hot start/stop (D-Server-01).
pub server_ctl: Arc<tokio::sync::Mutex<Option<ServerHandle>>>,
```

**In `AppCtx::build`** — follow lines 156-190 initialization pattern:
```rust
// After cartridges initialization (line 186):
let auth = Arc::new(AuthService::new(writer.clone(), readers.clone(), clock.clone()));

// In Ok(Self { ... }) block — add alongside cartridges:
auth,
server_ctl: Arc::new(tokio::sync::Mutex::new(None)),
```

---

### `crates/trackly-app/src/http/mod.rs` (modified — add live bind + middleware)

**Analog:** self (add to existing pattern in mod.rs + health.rs router shape)

**Router merge pattern** (http/health.rs lines 21-22):
```rust
// mod.rs will export a build_router() function merging all sub-routers:
pub fn build_router(ctx: &AppCtx, session_layer: SessionManagerLayer<RusqliteSessionStore>) -> Router {
    // Routes that bypass session check (login endpoint)
    let public = crate::http::auth::public_router();

    // Routes that require session
    let protected = Router::new()
        .merge(crate::http::auth::protected_router())
        .merge(crate::http::devices::router())
        .merge(crate::http::acts::router())
        .merge(crate::http::cartridges::router())
        .merge(crate::http::users::router())
        .merge(crate::http::settings::router())
        .merge(crate::http::templates::router())
        .merge(crate::http::organization::router())
        .layer(session_layer.clone()); // session gating on all protected routes

    Router::new()
        .merge(public)
        .merge(protected)
        .merge(crate::http::health::router()) // health always public
        .with_state(ctx.clone())
}
```

---

### `crates/trackly-app/src/main.rs` (modified — start server)

**Analog:** self + `shutdown.rs`

**Boot sequence addition** — after line 127 (`tauri::Builder::default()`):
```rust
// Phase 5: if server.enabled in config, start axum server under sub-CancellationToken
if ctx.config.server.enabled {
    let sub_token = ctx.shutdown.child_token(); // NOT the master token
    // ... build TLS, build router, tokio::spawn start_server(...)
    // Store handle in ctx.server_ctl
}

// Existing shutdown pattern (shutdown.rs):
trackly_app::shutdown::install_signal_handler(ctx.shutdown.clone());
```

---

### `migrations/V018__auth_settings.sql` (migration)

**Analog:** `migrations/V016__cartridges_kind_color_settings.sql`

Read the existing migration to verify pattern:
```sql
-- Pattern from V016: INSERT OR IGNORE into app_settings for new keys
-- (idempotent — safe to re-run)
INSERT OR IGNORE INTO app_settings (key, value, updated_at_utc)
VALUES ('desktop_lock_enabled', '0', unixepoch());
```

---

### Test files (CRUD, HTTP smoke, role matrix, server lifecycle)

**Analog:** `crates/trackly-app/tests/devices_crud.rs` + `tests/devices_http_smoke.rs`

**Test setup pattern** (devices_crud.rs lines 19-25):
```rust
// Every integration test: make_service() creates fresh tempfile DB + service
fn make_auth_service() -> (AuthService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers(); // from trackly-infra test_support
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = AuthService::new(writer, readers, clock);
    (svc, dir)
}
```

**Timeout wrapper pattern** (devices_crud.rs lines 48-51):
```rust
// Every test: 30s timeout to guard Linux-CI deadlock
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_name() {
    tokio::time::timeout(Duration::from_secs(30), async {
        // ...
    }).await.expect("test exceeded 30 s budget");
}
```

**HTTP role matrix pattern** (devices_http_smoke.rs lines 18-75):
```rust
// For role_endpoint_matrix.rs — build AppCtx + axum oneshot with auth header/session
// Pattern: build full router with .with_state(ctx), then oneshot per role×endpoint
let app = build_router(&ctx, session_layer).with_state(ctx.clone());
let res = app.clone().oneshot(
    Request::builder()
        .method("POST")
        .uri("/api/v1/users_create")
        .header("content-type", "application/json")
        // No session = 401; employee session = 403; admin session = 200
        .body(Body::from(serde_json::to_string(&payload)?))?,
).await?;
assert_eq!(res.status(), StatusCode::FORBIDDEN); // employee trying to create user
```

---

### `ui/src/lib/stores/auth.svelte.ts` (store, event-driven)

**Analog:** `ui/src/lib/stores/toast.svelte.ts`

**Runes store pattern** (toast.svelte.ts lines 1-44):
```typescript
// .svelte.ts extension REQUIRED — Svelte 5 runes only in .svelte/.svelte.ts
// $state at module level = reactive singleton

export type UserRole = 'admin' | 'manager' | 'employee';

export interface CurrentUser {
  id: number;
  login: string;
  fullName: string;
  role: UserRole;
}

// Module-level $state — reactive singleton (pattern from toast.svelte.ts line 20)
const _user = $state<{ value: CurrentUser | null }>({ value: null });

// Derived reactive value (pattern from theme.svelte.ts)
export const authStore = {
  get user() { return _user.value; },
  get role(): UserRole | null { return _user.value?.role ?? null; },
  get isAuthenticated() { return _user.value !== null; },
  setUser(u: CurrentUser | null) { _user.value = u; },
  clear() { _user.value = null; },
};
```

**Note on `$state` wrapping:** Svelte 5 module-level `$state` requires object wrapper for reactivity to propagate (plain `let _user = $state<CurrentUser | null>(null)` — the variable itself is reactive only inside `.svelte.ts` files). Follow `themeStore` pattern (theme.svelte.ts line 7): `export const themeStore = $state({ ... })`.

---

### `ui/src/lib/api/client.ts` (modified — add 401/403 handling)

**Analog:** self

**Current HTTP path** (client.ts lines 15-21):
```typescript
const res = await fetch(`/api/v1/${name}`, {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify(args),
});
if (!res.ok) throw parseAppError(await res.json().catch(() => ({})));
return res.json();
```

**Phase 5 extension — add 401/403 intercept:**
```typescript
if (!res.ok) {
  const err = parseAppError(await res.json().catch(() => ({})));
  // Redirect to login on auth errors
  if (err.code === 'UNAUTHORIZED' || err.code === 'FORBIDDEN') {
    authStore.clear();
    if (typeof window !== 'undefined') {
      window.location.hash = '#/login';
    }
  }
  throw err;
}
```

---

### `ui/src/features/layout/sidebar-config.ts` (modified — role filter)

**Analog:** self

**Current SIDEBAR_ITEMS shape** (sidebar-config.ts lines 7-22):
```typescript
// Add role field to SidebarItem type
export type SidebarItem = {
  kind: 'item';
  route: string;
  label: string;
  phase?: number | string;
  roles?: UserRole[];  // undefined = all roles; array = restrict to listed roles
};
```

**Filtered items function (new):**
```typescript
// Add to sidebar-config.ts:
import type { UserRole } from '$lib/stores/auth.svelte';

export function getVisibleItems(role: UserRole | null): SidebarEntry[] {
  return SIDEBAR_ITEMS.filter((entry) => {
    if (entry.kind === 'divider') return true;
    if (!entry.roles) return true;           // no restriction
    if (!role) return false;                 // not authenticated
    return entry.roles.includes(role);
  });
}
```

**Updated items with role restrictions:**
```typescript
{ kind: 'item', route: '/users', label: 'Пользователи', phase: 5, roles: ['admin'] },
{ kind: 'item', route: '/settings', label: 'Настройки', phase: 5, roles: ['admin'] },
{ kind: 'item', route: '/requests', label: 'Заявки', phase: 6 }, // all roles
```

---

### `ui/src/features/auth/LoginPage.svelte` + `FirstRunWizard.svelte` (components, request-response)

**Analog:** `ui/src/features/devices/DeviceFormModal.svelte`

**Svelte 5 $props() + $state pattern** (DeviceFormModal.svelte lines 19-26):
```svelte
<script lang="ts">
  import { apiCall } from '$lib/api/client';
  import { authStore } from '$lib/stores/auth.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';

  // No props for LoginPage (standalone page); FirstRunWizard also has no props
  let login = $state('');
  let password = $state('');
  let loading = $state(false);
  let error = $state<string | null>(null);

  async function handleSubmit() {
    loading = true;
    error = null;
    try {
      const user = await apiCall<UserDto>('auth_login', { login, password });
      authStore.setUser({ id: user.id, login: user.login, fullName: user.full_name, role: user.role });
      // Redirect to main page
      window.location.hash = '#/';
    } catch (e: unknown) {
      const appErr = e as { message?: string };
      error = appErr?.message ?? 'Ошибка входа';
    } finally {
      loading = false;
    }
  }
</script>
```

**Form validation pattern** (DeviceFormModal.svelte uses DeviceFormBody which validates before submit):
- Validate client-side (min 8 chars for password) before calling apiCall
- Display server-side errors in inline error block (not toast) for login forms

---

### `ui/src/features/users/UsersPage.svelte` + sub-components (component, CRUD)

**Analog:** `ui/src/features/devices/DevicesPage.svelte` (exact)

**Page state pattern** (DevicesPage.svelte lines 19-59):
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { apiCall } from '$lib/api/client';
  import { pushToast } from '$lib/stores/toast.svelte';
  import type { UserDto } from '../../bindings';

  let items = $state<UserDto[]>([]);
  let total = $state(0);
  let loading = $state(false);
  let modalOpen = $state(false);
  let editTarget = $state<UserDto | null>(null);
```

**Data loading pattern** (DevicesPage.svelte lines 66-96):
```svelte
  async function refresh() {
    loading = true;
    try {
      const resp = await apiCall<{ items: UserDto[], total: number }>('users_list', {
        filter: {}, pagination: { offset: 0, limit: 50 }
      });
      items = resp.items;
      total = resp.total;
    } catch (e: unknown) {
      const msg = e && typeof e === 'object' && 'message' in e
        ? String((e as { message: unknown }).message)
        : 'Не удалось загрузить пользователей';
      pushToast('error', msg);
    } finally {
      loading = false;
    }
  }

  onMount(() => { refresh(); });
```

**Sub-component structure** mirrors devices feature:
- `UsersPage.svelte` → page shell (analog: `DevicesPage.svelte`)
- `UsersList.svelte` → table/list (analog: `DeviceList.svelte`)
- `UserListRow.svelte` → single row (analog: `DeviceListRow.svelte`)
- `UserFormModal.svelte` → create/edit modal (analog: `DeviceFormModal.svelte`)

---

### `ui/src/features/settings/NetworkSettings.svelte` (component, request-response)

**Analog:** `ui/src/features/devices/DeviceFormModal.svelte` (form + async submit)

**Form + async submit pattern** (DeviceFormModal.svelte $props + state lines 19-66):
```svelte
<script lang="ts">
  import { apiCall } from '$lib/api/client';
  import { pushToast } from '$lib/stores/toast.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';

  // Settings loaded on mount
  let enabled = $state(false);
  let host = $state('127.0.0.1');
  let port = $state(8443);
  let certPath = $state('');
  let serverUrl = $state<string | null>(null);
  let fingerprint = $state<string | null>(null);
  let loading = $state(false);
  let saving = $state(false);

  async function loadSettings() {
    loading = true;
    try {
      const s = await apiCall<NetworkSettingsDto>('settings_get_network', {});
      enabled = s.enabled; host = s.host; port = s.port; certPath = s.cert_path;
      serverUrl = s.server_url; fingerprint = s.fingerprint;
    } finally { loading = false; }
  }

  async function toggleServer() {
    saving = true;
    try {
      const info = await apiCall<ServerStatusDto>('server_toggle', { enable: !enabled });
      enabled = info.running;
      serverUrl = info.url ?? null;
      fingerprint = info.fingerprint ?? null;
      pushToast('success', enabled ? 'Сервер запущен' : 'Сервер остановлен');
    } catch (e: unknown) {
      // ...
    } finally { saving = false; }
  }
```

---

### `ui/src/pages/SettingsPage.svelte` (modified — mini-version with Network tab)

**Analog:** `ui/src/pages/UsersPage.svelte` placeholder → replaced with real content

```svelte
<script lang="ts">
  import NetworkSettings from '../features/settings/NetworkSettings.svelte';
  // Phase 7 will add more tabs here
</script>

<div class="settings-page">
  <header class="page-header">
    <h1 class="page-title">Настройки</h1>
  </header>
  <div class="settings-tabs">
    <!-- Phase 5: only Network tab -->
    <NetworkSettings />
  </div>
</div>
```

---

### `ui/src/App.svelte` (modified — bootstrap guard + auth-aware layout)

**Analog:** self + RESEARCH Code Example (Svelte 5 first-run guard)

**Current App.svelte** (lines 1-11 — Router + Layout):
```svelte
<script lang="ts">
  import Router from 'svelte-spa-router';
  import { routes } from './routes';
  import Layout from './features/layout/Layout.svelte';
  import ToastHost from '$lib/components/ToastHost.svelte';
</script>

<Layout>
  <Router {routes} />
</Layout>
<ToastHost />
```

**Phase 5 extension** — wrap with bootstrap check (RESEARCH Pattern 6):
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { authStore } from '$lib/stores/auth.svelte';
  import { apiCall } from '$lib/api/client';
  // ... existing imports

  let bootstrapNeeded = $state(false);
  let appLoading = $state(true);

  onMount(async () => {
    try {
      const status = await apiCall<{ needs_bootstrap: boolean, user: UserDto | null }>('auth_status', {});
      bootstrapNeeded = status.needs_bootstrap;
      if (status.user) authStore.setUser({ /* map user */ });
    } finally { appLoading = false; }
  });
</script>

{#if appLoading}
  <Spinner />
{:else if bootstrapNeeded}
  <FirstRunWizard />
{:else}
  <Layout>
    <Router {routes} />
  </Layout>
{/if}
<ToastHost />
```

---

## Shared Patterns

### Single-writer / reader-pool pattern
**Source:** `crates/trackly-infra/src/db/writer_worker.rs` lines 76-120, `pools.rs` lines 74-97
**Apply to:** `services/auth.rs`, `server/rusqlite_session_store.rs`, ALL new service methods
```rust
// Writes:
self.writer.execute(move |conn| {
    // sync rusqlite operations
    Ok(result)
}).await?

// Reads:
let readers = self.readers.clone();
tokio::task::spawn_blocking(move || {
    let conn = readers.acquire(); // RAII, returns on drop
    // sync rusqlite operations
}).await.map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking: {e}") })?
```

### AppError / AppErrorResponse pattern
**Source:** `crates/trackly-core/src/error.rs` + `crates/trackly-app/src/error_axum.rs`
**Apply to:** all axum handlers (return `Result<Json<T>, AppErrorResponse>`), all services (return `Result<T, AppError>`)
- `Unauthorized` (401) and `Forbidden` (403) variants already exist in `AppError` (error.rs lines 88-98)
- `AppErrorResponse` already maps them to correct HTTP status codes (error_axum.rs lines 38-41)

### Secret<T> pattern
**Source:** `crates/trackly-core/src/primitives/secret.rs`
**Apply to:** `services/auth.rs` (password handling), `server/tls.rs` (private key in memory)
```rust
// Wrap sensitive value: Secret::new(password_string)
// Expose only when needed: secret.expose()
// NEVER: log, serialize, or clone the exposed value
```

### Dual-transport build_* pattern
**Source:** `crates/trackly-app/src/tauri_cmds/devices.rs` lines 21-80
**Apply to:** `tauri_cmds/auth.rs`, `tauri_cmds/users.rs` — all shared `build_*` functions
- `build_*` functions take `&AppCtx` (not `tauri::State`)
- Both axum handler and Tauri command call the same `build_*`
- Business logic (including `authorize()`) lives in `build_*` or service, NOT in transport layer

### CancellationToken child pattern
**Source:** `crates/trackly-app/src/shutdown.rs` + `context.rs` line 51
**Apply to:** `server/mod.rs`, `main.rs` server start, `tauri_cmds/auth.rs` server_toggle
```rust
// Always use child_token() for sub-systems — never cancel the master token
let server_token = ctx.shutdown.child_token();
// Cancel server without killing the whole app:
server_token.cancel(); // only server stops
// ctx.shutdown.cancel() kills everything (Ctrl-C)
```

### Svelte 5 $state runes store pattern
**Source:** `ui/src/lib/stores/toast.svelte.ts` + `ui/src/lib/stores/theme.svelte.ts`
**Apply to:** `ui/src/lib/stores/auth.svelte.ts`
- File extension MUST be `.svelte.ts` (not `.ts`)
- Module-level `$state` with object wrapper for singleton reactivity
- Expose getters, not raw `$state` variables

### apiCall + error propagation pattern
**Source:** `ui/src/lib/api/client.ts` + `ui/src/lib/api/devices.ts`
**Apply to:** `ui/src/lib/api/` new auth/users modules
```typescript
// Feature API module pattern (devices.ts lines 15-65):
export const auth = {
  login: (login: string, password: string) =>
    apiCall<UserDto>('auth_login', { login, password }),
  logout: () => apiCall<null>('auth_logout', {}),
  status: () => apiCall<AuthStatusDto>('auth_status', {}),
};
```

### Test setup pattern
**Source:** `crates/trackly-app/tests/devices_crud.rs` lines 19-25 + `crates/trackly-infra/src/test_support/test_app_ctx.rs`
**Apply to:** ALL new `crates/trackly-app/tests/*.rs` files
```rust
use trackly_infra::test_support::test_writer_and_readers;

fn make_auth_service() -> (AuthService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    (AuthService::new(writer, readers, clock), dir)
}

// Always 30s timeout:
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn my_test() {
    tokio::time::timeout(Duration::from_secs(30), async { ... })
        .await.expect("test exceeded 30 s budget");
}
```

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/trackly-app/src/server/tls.rs` (full TLS+rcgen logic) | utility | transform | No TLS code anywhere in codebase — use RESEARCH Pattern 3 excerpts directly |
| `crates/trackly-app/tests/graceful_shutdown_drain.rs` | test | event-driven | No existing server lifecycle tests — use RESEARCH Pattern 4 + tokio::time::sleep + TcpStream::connect to verify port freed |

---

## Metadata

**Analog search scope:** `crates/trackly-app/src/`, `crates/trackly-core/src/`, `crates/trackly-infra/src/`, `ui/src/`
**Files scanned:** 95
**Pattern extraction date:** 2026-06-13
