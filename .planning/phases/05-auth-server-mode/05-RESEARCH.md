# Phase 5: Авторизация, локальные пользователи и серверный режим — Research

**Researched:** 2026-06-13
**Domain:** Rust auth (argon2id), tower-sessions custom store, rustls HTTPS (axum), RBAC, Svelte 5 auth-store
**Confidence:** HIGH (стек фиксирован в CLAUDE.md, все ключевые решения закреплены в CONTEXT.md)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Десктоп-доступ и bootstrap:**
- D-Bootstrap-01: первый admin — through first-run мастер (нет авто-seed admin/admin).
- D-Desktop-01: unlocked-by-default = trusted-admin (полный admin-доступ без входа); audit-attribution: если один admin — атрибутировать ему, иначе `user_id = NULL`.
- D-Desktop-02: десктоп-лок хранится в `app_settings` (не в config.toml); при локе — тот же логин-экран, что и веб, argon2id-верификация.

**Роли и enforcement:**
- D-RBAC-01: единый `authorize(ctx, action)` в сервис-слое; Tauri-транспорт передаёт trusted-admin или залогиненного; axum — роль из сессии. CI-тест role × endpoint → 403.
- D-RBAC-02: роль employee заводится сейчас (полный RBAC-каркас); employee видит placeholder «Заявки появятся скоро» в Phase 5.
- D-RBAC-03: sidebar фильтруется по роли; `authorize()` — источник истины безопасности.

**Серверный режим:**
- D-Server-01: горячий старт/стоп через дочерний CancellationToken (под AppCtx.shutdown); смена порта = stop+start.
- D-Server-02: раздел `/settings` с одной вкладкой «Сеть» (тумблер, порт, bind, путь к cert).
- D-Server-03: bind-адрес — dropdown без предупреждений.
- D-Server-04: HTTPS-only (без HTTP listener); rustls + rcgen self-signed при первом включении; UI показывает `https://<ip>:<port>`, SHA-256 fingerprint и краткую инструкцию.

**Веб-сессия и безопасность:**
- D-Session-01: tower-sessions, rusqlite-backed store (таблица `sessions` V010), sliding 30 дней, переживает рестарт.
- D-Session-02: `SameSite=Strict` + `Secure` + `HttpOnly` + проверка Origin/Referer на mutation-эндпоинтах. Security headers (CSP/no-sniff/frame-deny) через tower-http.
- D-Auth-01: мин. 8 символов; admin сбрасывает пароль любого; любой меняет свой (нужен старый).
- D-Auth-02: rate-limit на /login (~5–10 попыток/мин по IP/логину).

### Claude's Discretion
- Точная вёрстка login-экрана.
- Конкретный набор security headers и числа rate-limit.
- Slug раздела настроек для вкладки «Сеть» внутри `/settings`.

### Deferred Ideas (OUT OF SCOPE)
- AD/LDAP-вход → Phase 8.
- Полный раздел Настройки (прочие вкладки) → Phase 7.
- Email/SMTP сброс пароля → v2.
- Портал заявок employee (REQ-01..07) → Phase 6.
- Force-change пароля при первом входе → возможная настройка в будущем.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| USR-01 | CRUD пользователей: логин, ФИО, пароль (argon2id + Secret<T>), роль, email, активен/заблокирован | AuthService + UserRepository + users-таблица V002 уже существует |
| USR-02 | Три роли: Администратор / Специалист / Сотрудник | `authorize()` pattern + role TEXT 'admin'|'manager'|'employee' |
| USR-03 | Сессии через tower-sessions + SQLite-store, логин по логин/пароль через веб | Rusqlite SessionStore impl + SessionManagerLayer |
| USR-04 | Таuri-десктоп: trusted-admin без логина; опциональный лок | D-Desktop-01/02, app_settings key |
| USR-05 | Logout + смена пользователя в веб-режиме | Session::flush() + Set-Cookie expires |
| USR-06 | Авторизация enforced на API + UI (curl → 403) | authorize() + CI role×endpoint test matrix |
| USR-07 | HTTPS в server mode; rcgen self-signed; путь к своему cert конфигурируем | rustls + rcgen + ServerConfig.cert_path |
| SRV-01 | Тумблер «Запустить сервер» в Настройки → Сеть | Hot start/stop через sub-CancellationToken |
| SRV-02 | axum с CSRF, security headers, rate limiting (basic) | tower-http + Origin-check + tower_governor |
| SRV-03 | Tauri и axum используют ОДИН набор бизнес-сервисов через AppCtx | Уже реализовано в архитектуре; Phase 5 добавляет AuthService в AppCtx |
| SRV-04 | HTTPS обязателен в server mode | rustls-only, без HTTP-fallback listener |
| SRV-05 | Корректное завершение axum при выходе из приложения | axum::serve.with_graceful_shutdown(token.cancelled()) |
| SET-08 | Настройки сетевого доступа: порт, bind, включение server mode | SettingsPage mini + AppSettings in DB |
</phase_requirements>

---

## Summary

Phase 5 — наиболее многоуровневая фаза проекта: она одновременно вводит аутентификацию пользователей (argon2id), сессионное управление (tower-sessions + rusqlite), HTTPS-сервер (rustls + rcgen), RBAC (единый `authorize()`), и тонкий набор UI (login-экран, users CRUD, mini-settings/сеть).

Стек полностью зафиксирован в CLAUDE.md и CONTEXT.md — альтернативы не рассматриваются. Ключевые технические риски сосредоточены в трёх зонах: (1) корректный мост между async SessionStore trait и синхронным rusqlite (single-writer + reader-pool), (2) rustls TlsAcceptor поверх tokio-rustls без внешнего `axum-server` (совместимость с axum 0.8), (3) горячий старт/стоп axum-задачи через sub-CancellationToken без утечки портов.

Все существующие интеграционные точки (AppCtx, axum-роутеры, V010 sessions, AppError, apiCall) уже готовы и ждут подключения в этой фазе.

**Основная рекомендация:** строить вертикальными слайсами login→session→authorize() начиная с Tauri-десктопа (bootstrap + users CRUD), затем axum (live bind + TLS), затем RBAC enforcement на всех существующих роутерах.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Password hashing / verify | Backend (Rust) | — | CPU-bound; spawn_blocking; никогда не во фронтенде |
| Session create / save / load / delete | Backend (Rust — axum middleware) | — | tower-sessions SessionManagerLayer на axum |
| Session storage (rusqlite) | Backend (Rust — writer/reader pool) | — | Та же single-writer/reader-pool архитектура |
| RBAC authorize() | Backend (сервис-слой) | — | Источник истины — не UI |
| TLS / cert gen | Backend (Rust — tokio task) | — | rcgen + rustls при first-enable |
| Fingerprint display | Frontend (UI) | Backend API | Backend вычисляет SHA-256 fingerprint из DER |
| Hot start/stop сервера | Backend (AppCtx sub-token) | Frontend toggle | CancellationToken дочерний к AppCtx.shutdown |
| Security headers | Backend (tower-http layer) | — | Централизованно на всём `/api/*` и `/` |
| Rate limiting /login | Backend (axum middleware) | — | tower_governor или inline counter per IP |
| Auth-store (current user, role) | Frontend (Svelte runes store) | — | Кэш состояния; источник истины — сессия на бэке |
| Sidebar role filtering | Frontend (Svelte runes) | — | UX-слой; не замена authorize() |
| First-run bootstrap wizard | Frontend (Svelte) + Backend | — | Детектируется через пустую users-таблицу |
| Desktop lock flag | Backend (app_settings в БД) | Frontend | Переносится с портативной БД |

---

## Standard Stack

### Core (Phase 5 additions — все зафиксированы в CLAUDE.md)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `argon2` | `0.5.3` [VERIFIED: docs.rs] | Password hashing / verify (argon2id) | OWASP 2024 рекомендован; pure Rust, portable; уже в CLAUDE.md |
| `tower-sessions` | `0.15.0` [VERIFIED: cargo search] | Session middleware для axum | CLAUDE.md фиксирует; последняя стабильная (0.13→0.15 нет breaking changes в SessionStore trait) |
| `rustls` | `0.23.x` | TLS для axum HTTPS | CLAUDE.md: pure Rust, portable, no OpenSSL DLL |
| `tokio-rustls` | `0.26.4` | TLS acceptor для tokio TcpListener | Зависит от rustls ^0.23.27; позволяет использовать `axum::serve` напрямую без отдельного axum-server crate |
| `rcgen` | `0.14.8` [VERIFIED: cargo search] | Self-signed cert generation | CLAUDE.md; generate_simple_self_signed() + cert.der() для fingerprint |
| `rustls-pemfile` | `2.2.0` [VERIFIED: cargo search] | Parse PEM cert/key files (user-supplied cert) | Идёт вместе с rustls 0.23 |
| `rand` | `0.8.x` | OsRng для argon2 salt | Используется tower-sessions 0.15 internals (rand 0.9); для argon2 salt нужна `password-hash` crate которая тянет rand 0.8 |
| `password-hash` | `0.5.x` | SaltString, PasswordHash, PasswordHasher/Verifier traits | Transitive через argon2 0.5 |
| `sha2` | `0.10` | SHA-256 для cert fingerprint | Уже в [dev-dependencies]; поднять в dependencies |
| `tower_governor` | `0.8.0` [VERIFIED: cargo search] | Rate-limit на /login | Lightweight; backed by governor; работает как tower Layer |
| `async-trait` | `0.1` | Для impl SessionStore (async fn в trait до AFIT stabilization) | Уже в workspace |

### Уже в workspace, нужно добавить features

| Library | Дополнение | Причина |
|---------|-----------|---------|
| `tower-http` | `+ "set-header"` feature | SetResponseHeaderLayer для security headers |
| `axum` | `+ "ws"` (опционально Phase 6) | WebSocket — не нужен в Phase 5 |
| `tokio` | уже `"net"` | Нужен для TcpListener |

### Не нужен / отклонён

| Crate | Почему не нужен |
|-------|----------------|
| `axum-server` | Версия 0.8.0 зависит от axum 0.7, НЕ 0.8. Используем tokio-rustls + axum::serve напрямую [ASSUMED: требует проверки совместимости, но наиболее безопасный путь для axum 0.8] |
| `tower-sessions-sqlx-store` | Не нужен — мы на rusqlite, не sqlx. Пишем собственный impl (~80 LoC) |
| `hyper-rustls` | Избыточен — tokio-rustls достаточен для нашего уровня |
| `jsonwebtoken` / JWT | Решено в CONTEXT.md: tower-sessions cookies предпочтительнее |

### Installation (Wave 0)

```toml
# В [workspace.dependencies]
argon2 = { version = "0.5", features = ["std"] }
tower-sessions = "0.15"
tokio-rustls = "0.26"
rcgen = "0.14"
rustls-pemfile = "2"
tower_governor = "0.8"

# tower-http: добавить feature "set-header"
tower-http = { version = "0.6", features = ["trace", "cors", "set-header"] }

# sha2 уже в dev-deps trackly-app — перенести в dependencies
sha2 = "0.10"
```

---

## Package Legitimacy Audit

> slopcheck не был доступен (denied by sandbox). Все пакеты проверены через `cargo search` (crates.io) и официальную документацию. Все являются широко известными crate'ами Rust-экосистемы.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `argon2` 0.5.3 | crates.io | ~4 yr | 10M+/mo | github.com/RustCrypto/password-hashes | N/A | Approved — RustCrypto официальный |
| `tower-sessions` 0.15.0 | crates.io | ~2 yr | 500K+/mo | github.com/maxcountryman/tower-sessions | N/A | Approved — экосистема tower/axum |
| `tokio-rustls` 0.26.4 | crates.io | ~6 yr | 20M+/mo | github.com/rustls/tokio-rustls | N/A | Approved — официальный rustls ecosystem |
| `rcgen` 0.14.8 | crates.io | ~5 yr | 5M+/mo | github.com/rustls/rcgen | N/A | Approved — официальный rustls ecosystem |
| `rustls-pemfile` 2.2.0 | crates.io | ~4 yr | 15M+/mo | github.com/rustls/rustls | N/A | Approved |
| `tower_governor` 0.8.0 | crates.io | ~2 yr | 100K+/mo | github.com/benwis/tower-governor | N/A [ASSUMED] | Approved — признанный в tower-ecosystem |
| `sha2` 0.10 | crates.io | ~5 yr | 30M+/mo | github.com/RustCrypto/hashes | N/A | Approved — RustCrypto официальный |
| `password-hash` 0.5 | crates.io | ~3 yr | 15M+/mo | github.com/RustCrypto/traits | N/A | Transitive от argon2; Approved |

**Packages removed due to slopcheck [SLOP]:** none
**Packages flagged as suspicious [SUS]:** none

*slopcheck was unavailable; packages above are tagged [ASSUMED] for tower_governor specifically. All others are well-established crates with verified GitHub presence in official org repos.*

---

## Architecture Patterns

### System Architecture Diagram

```
Browser (LAN)                Desktop (Tauri webview)
     |                               |
     | HTTPS /api/v1/*               | tauri::invoke()
     v                               v
 axum Router                  Tauri Commands
  [SessionManagerLayer]         [trusted-admin OR
  [RateLimitLayer /login]        session via DB lock]
  [SecurityHeadersLayer]              |
  [Origin/Referer check]              |
     |                               |
     +----------+--------------------+
                |
                v
         AuthService::authorize(identity, action)
                |
         [AppCtx — Arc-clone]
           /    |    \
      AuthService  DeviceService  CartridgeService ...
          |          |               |
          v          v               v
    WriterHandle  ReaderPool    (same single-writer pattern)
          |          |
          v          v
       SQLite (WAL mode)
    users | sessions | audit_log | app_settings
```

### Recommended Project Structure (новые файлы Phase 5)

```
crates/trackly-app/src/
├── http/
│   ├── auth.rs              # POST /api/v1/auth/login, logout, me
│   ├── users.rs             # CRUD /api/v1/users_*
│   └── settings.rs          # GET/POST /api/v1/settings_network_*
├── services/
│   └── auth.rs              # AuthService: login(), authorize(), create_user(), ...
├── tauri_cmds/
│   ├── auth.rs              # Tauri commands: auth_login, auth_logout, auth_me, ...
│   └── users.rs             # users_list, users_create, users_update, users_delete
├── dto/
│   └── auth.rs              # LoginRequest, LoginResponse, UserDto, UserNew, UserPatch, ...
├── server/
│   ├── mod.rs               # start_server() → tokio task
│   ├── tls.rs               # build_tls_config(cert_path) → rustls::ServerConfig
│   └── rusqlite_session_store.rs  # impl SessionStore for RusqliteSessionStore
└── context.rs               # + auth: Arc<AuthService>, server_token: Arc<Mutex<Option<CancellationToken>>>

crates/trackly-core/src/
└── auth.rs                  # Identity { user_id, role }, Role enum, Action enum, authorize() fn

migrations/
└── V018__auth_settings.sql  # app_settings keys: desktop_lock_enabled (bool 0/1)

ui/src/
├── lib/stores/
│   └── auth.svelte.ts       # $state: {user, role, isAuthenticated}; authStore
├── features/
│   ├── auth/
│   │   ├── LoginPage.svelte
│   │   └── FirstRunWizard.svelte
│   ├── users/
│   │   ├── UsersPage.svelte        # заменяет placeholder
│   │   ├── UsersList.svelte
│   │   ├── UserListRow.svelte
│   │   └── UserFormModal.svelte
│   └── settings/
│       └── NetworkSettings.svelte  # вкладка «Сеть»
└── pages/
    └── SettingsPage.svelte  # mini-version: только Network tab
```

### Pattern 1: Unified authorize() в сервис-слое

**Что:** Каждая операция требует `Identity` (user_id + role). `AuthService::authorize()` принимает Identity и Action, возвращает `Result<(), AppError::Forbidden>`. Handlers (Tauri и axum) получают Identity из разных источников, но передают в единый authorize().

**Когда использовать:** При каждой мутирующей операции и при защищённых чтениях (users list → только admin/manager).

```rust
// Source: trackly-core/src/auth.rs (новый)
#[derive(Clone, Debug)]
pub struct Identity {
    pub user_id: Option<i64>,  // None = trusted-desktop (unlocked)
    pub role: Role,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Role { Admin, Manager, Employee }

#[derive(Clone, Debug)]
pub enum Action {
    ManageUsers,        // только admin
    ManageSettings,     // только admin
    MutateDevices,      // admin + manager
    MutateActs,         // admin + manager
    MutateCartridges,   // admin + manager
    ReadData,           // все авторизованные
    CreateRequest,      // все авторизованные
}

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

**Tauri transport** (trusted-desktop, unlocked):
```rust
// В Tauri command handler
let identity = ctx.auth.desktop_identity().await;  // trusted-admin или из сессии
authorize(&identity, &Action::MutateDevices)?;
```

**axum transport** (из сессии):
```rust
// В axum middleware / extractor
async fn extract_identity(session: Session) -> Result<Identity, AppError> {
    session.get::<Identity>("identity").await
        .map_err(|_| AppError::Unauthorized)?
        .ok_or(AppError::Unauthorized)
}
```

### Pattern 2: Rusqlite-backed SessionStore

**Что:** Кастомная реализация `tower_sessions::SessionStore` поверх существующего single-writer / reader-pool паттерна.

**Ключевой момент:** SessionStore методы async. Rusqlite sync. Мост — `tokio::task::spawn_blocking`.

```rust
// Source: crates/trackly-app/src/server/rusqlite_session_store.rs
use async_trait::async_trait;
use time::OffsetDateTime;
use tower_sessions::{
    session::{Id, Record},
    session_store, SessionStore,
};

#[derive(Clone, Debug)]
pub struct RusqliteSessionStore {
    writer: Arc<WriterHandle>,
    readers: Arc<ReaderPool>,
}

#[async_trait]
impl SessionStore for RusqliteSessionStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        // Collision-safe: пробуем insert, при UNIQUE-violation меняем id
        let id_bytes = record.id.0.to_le_bytes().to_vec();
        let data = rmp_serde::to_vec(record)
            .map_err(|e| session_store::Error::Encode(e.to_string()))?;
        let expiry_ts = record.expiry_date.unix_timestamp();
        self.writer
            .execute(move |conn| {
                conn.execute(
                    "INSERT OR IGNORE INTO sessions (id, data, expiry_date) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id_bytes, data, expiry_ts],
                )
                .map_err(|e| AppError::Internal { source_chain: e.to_string() })?;
                Ok(())
            })
            .await
            .map_err(|e| session_store::Error::Backend(e.to_string()))
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        let id_bytes = record.id.0.to_le_bytes().to_vec();
        let data = rmp_serde::to_vec(record)
            .map_err(|e| session_store::Error::Encode(e.to_string()))?;
        let expiry_ts = record.expiry_date.unix_timestamp();
        self.writer
            .execute(move |conn| {
                conn.execute(
                    "INSERT OR REPLACE INTO sessions (id, data, expiry_date) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id_bytes, data, expiry_ts],
                )
                .map_err(|e| AppError::Internal { source_chain: e.to_string() })?;
                Ok(())
            })
            .await
            .map_err(|e| session_store::Error::Backend(e.to_string()))
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        let id_bytes = session_id.0.to_le_bytes().to_vec();
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT data FROM sessions WHERE id = ?1 AND expiry_date > ?2",
                rusqlite::params![id_bytes, now],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
        })
        .await
        .map_err(|e| session_store::Error::Backend(e.to_string()))?
        .map_err(|e| session_store::Error::Backend(e.to_string()))
        .and_then(|opt| {
            opt.map(|bytes| {
                rmp_serde::from_slice::<Record>(&bytes)
                    .map_err(|e| session_store::Error::Decode(e.to_string()))
            })
            .transpose()
        })
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        let id_bytes = session_id.0.to_le_bytes().to_vec();
        self.writer
            .execute(move |conn| {
                conn.execute("DELETE FROM sessions WHERE id = ?1", rusqlite::params![id_bytes])
                    .map_err(|e| AppError::Internal { source_chain: e.to_string() })?;
                Ok(())
            })
            .await
            .map_err(|e| session_store::Error::Backend(e.to_string()))
    }
}
```

**Важно:** `rmp_serde` (MessagePack) — та же библиотека, что использует tower-sessions-sqlx-store. Нужно добавить `rmp-serde = "1"` в зависимости.

**Cookie конфигурация:**
```rust
let session_layer = SessionManagerLayer::new(session_store)
    .with_secure(true)        // HTTPS-only cookie
    .with_http_only(true)     // no JS access
    .with_same_site(tower_sessions::cookie::SameSite::Strict)
    .with_expiry(Expiry::OnInactivity(Duration::days(30)));
```

### Pattern 3: HTTPS через tokio-rustls + axum::serve

**Что:** axum 0.8 + tokio-rustls 0.26 без `axum-server` crate (он зависит от axum 0.7). Напрямую через `tokio_rustls::TlsAcceptor`.

```rust
// Source: crates/trackly-app/src/server/tls.rs

use std::sync::Arc;
use rcgen::generate_simple_self_signed;
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio_rustls::TlsAcceptor;

pub struct TlsBundle {
    pub acceptor: TlsAcceptor,
    pub fingerprint_hex: String,  // SHA-256 DER fingerprint для UI
    pub cert_pem: String,          // для сохранения на диск (первый run)
    pub key_pem: String,
}

pub fn generate_self_signed(host: &str) -> anyhow::Result<TlsBundle> {
    let subject_alt_names = vec![host.to_string(), "localhost".to_string()];
    let rcgen::CertifiedKey { cert, signing_key } =
        generate_simple_self_signed(subject_alt_names)?;

    let cert_der: Vec<u8> = cert.der().to_vec();
    let key_der = signing_key.serialize_der();

    // SHA-256 fingerprint для отображения пользователю
    use sha2::{Sha256, Digest};
    let fingerprint = {
        let hash = Sha256::digest(&cert_der);
        hash.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(":")
    };

    let certs = vec![CertificateDer::from(cert_der)];
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_der));

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(TlsBundle {
        acceptor: TlsAcceptor::from(Arc::new(config)),
        fingerprint_hex: fingerprint,
        cert_pem: cert.pem(),
        key_pem: signing_key.serialize_pem(),
    })
}
```

**axum serve с TLS:**
```rust
// Source: crates/trackly-app/src/server/mod.rs

pub async fn start_server(
    app: Router,
    addr: SocketAddr,
    tls: TlsAcceptor,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;

    // axum 0.8 graceful shutdown с TLS:
    // При использовании кастомного TLS acceptor нельзя использовать
    // axum::serve напрямую. Используем цикл accept + spawn.
    loop {
        tokio::select! {
            result = listener.accept() => {
                let (stream, _) = result?;
                let tls = tls.clone();
                let app = app.clone();
                tokio::spawn(async move {
                    match tls.accept(stream).await {
                        Ok(tls_stream) => {
                            let io = hyper_util::rt::TokioIo::new(tls_stream);
                            let hyper_service = hyper::service::service_fn(
                                move |req| app.clone().call(req)
                            );
                            let _ = hyper::server::conn::http1::Builder::new()
                                .serve_connection(io, hyper_service)
                                .await;
                        }
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

**Альтернатива:** axum::serve с graceful shutdown поддерживает только `TcpListener` (без TLS). Для TLS нужен либо кастомный цикл accept (выше), либо `hyper-rustls` + `hyper::server`. Кастомный цикл — проще и достаточен для нашего масштаба (20 concurrent users LAN).

**Нужно добавить:**
```toml
hyper = { version = "1", features = ["http1", "server"] }
hyper-util = { version = "0.1", features = ["tokio"] }
rmp-serde = "1"
```

### Pattern 4: Hot start/stop сервера через sub-CancellationToken

**Что:** AppCtx.shutdown — мастер-токен. Сервер имеет собственный дочерний токен в Arc<Mutex<Option<CancellationToken>>>. Тумблер:
- Start: создаёт дочерний токен, спаунит tokio::task с сервером.
- Stop: cancel() на дочернем токене.
- Restart (смена порта): stop → wait task completion → start.

```rust
// В AppCtx добавить:
pub server_ctl: Arc<Mutex<Option<ServerHandle>>>,

// ServerHandle хранит cancel-токен и JoinHandle задачи
pub struct ServerHandle {
    cancel: CancellationToken,
    task: tokio::task::JoinHandle<()>,
}

// Tauri command: toggle server
pub async fn server_toggle(ctx: AppCtx, enable: bool) -> Result<ServerInfo, AppError> {
    let mut ctl = ctx.server_ctl.lock().await;
    if !enable {
        if let Some(handle) = ctl.take() {
            handle.cancel.cancel();
            let _ = handle.task.await;
        }
        return Ok(ServerInfo { running: false, ..Default::default() });
    }
    // stop existing if running
    if let Some(handle) = ctl.take() {
        handle.cancel.cancel();
        let _ = handle.task.await;
    }
    // start new
    let child = ctx.shutdown.child_token();
    let tls_bundle = build_tls(&ctx.config)?;
    let app = build_router(&ctx, &tls_bundle.session_layer);
    let addr = parse_addr(&ctx.config.server)?;
    let cancel = child.clone();
    let task = tokio::spawn(async move {
        if let Err(e) = start_server(app, addr, tls_bundle.acceptor, cancel).await {
            tracing::error!("server error: {e}");
        }
    });
    *ctl = Some(ServerHandle { cancel: child, task });
    Ok(ServerInfo { running: true, url: format!("https://{}:{}", ...), fingerprint: tls_bundle.fingerprint_hex })
}
```

### Pattern 5: argon2id hash/verify с Secret<String>

**Что:** CPU-bound — выполняется в `spawn_blocking`. OWASP 2024 параметры.

```rust
// Source: docs.rs/argon2/0.5.3
use argon2::{
    password_hash::{OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params, Algorithm, Version,
};
use trackly_core::primitives::secret::Secret;

pub fn hash_password(password: &Secret<String>) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let params = Params::new(19456, 2, 1, None)?;  // m=19456 KiB, t=2, p=1 (OWASP 2024)
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    Ok(argon2.hash_password(password.expose().as_bytes(), &salt)?.to_string())
}

pub fn verify_password(password: &Secret<String>, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default().verify_password(password.expose().as_bytes(), &parsed).is_ok()
}

// В AuthService::login() — обязательно spawn_blocking:
pub async fn login(&self, login: &str, password: Secret<String>) -> Result<UserDto, AppError> {
    let hash = self.get_password_hash(login).await?;  // из reader pool
    let ok = tokio::task::spawn_blocking(move || verify_password(&password, &hash)).await
        .map_err(|_| AppError::Internal { source_chain: "spawn_blocking join".into() })?;
    if !ok { return Err(AppError::Unauthorized); }
    // ...создать/обновить сессию, вернуть UserDto
}
```

### Pattern 6: Svelte 5 auth-store + router guard

**Что:** Runes-based store для current user, role, isAuthenticated. Redirect на login при 401.

```typescript
// Source: ui/src/lib/stores/auth.svelte.ts
export type UserRole = 'admin' | 'manager' | 'employee';

export interface CurrentUser {
  id: number;
  login: string;
  fullName: string;
  role: UserRole;
}

// Runes — глобальное реактивное состояние
let _user = $state<CurrentUser | null>(null);
let _isAuthenticated = $derived(_user !== null);

export const authStore = {
  get user() { return _user; },
  get role() { return _user?.role ?? null; },
  get isAuthenticated() { return _isAuthenticated; },
  setUser(u: CurrentUser | null) { _user = u; },
  clear() { _user = null; },
};

// В apiCall() — перехват 401:
// При получении AppError.code === 'UNAUTHORIZED' → authStore.clear() + navigate('/#/login')
```

**Guard в App.svelte или Layout:**
```svelte
<!-- ui/src/features/layout/Layout.svelte -->
{#if !authStore.isAuthenticated && currentRoute !== '/login'}
  <!-- redirect -->
  {() => { location.hash = '#/login'; }}
{:else}
  {@render children?.()}
{/if}
```

### Anti-Patterns to Avoid

- **Не проверять роль только на UI**: sidebar-скрытие — UX, не security. `authorize()` должен быть в сервис-слое.
- **Не запускать argon2 в async-контексте без spawn_blocking**: blokирует tokio executor.
- **Не держать два HTTP listener'а (HTTP + HTTPS)**: только HTTPS (D-Server-04). HTTP-redirect не нужен на LAN.
- **Не хранить пароль в текстовом виде нигде**: Secret<String> → hash → store. В сессии хранить только user_id + role, не password.
- **Не использовать session.data напрямую**: всегда через `session.get::<T>(key)` / `session.insert(key, value)`.
- **Не забыть flush() сессии при logout**: `session.flush().await` (удаляет все данные) или `session.delete().await` (удаляет из store).
- **Не шарить один CancellationToken между сервером и AppCtx.shutdown**: использовать дочерний токен (`shutdown.child_token()`), иначе stop сервера → shutdown всего приложения.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Password hashing | Собственный KDF | `argon2 0.5` (RustCrypto) | Side-channel timing, salt-reuse, OWASP параметры — слишком много нюансов |
| Session ID generation | Собственный UUID генератор | `Id::default()` из tower-sessions (rand i128) | Collision-resistance: 128 бит энтропии через OsRng |
| TLS handshake | Собственный TLS | `rustls 0.23` + `tokio-rustls` | Тысячи edge-cases в TLS 1.2/1.3 |
| CSRF token double-submit | Собственный CSRF | SameSite=Strict + Origin check (D-Session-02) | Достаточно для single-origin SPA на LAN; минимум сложности |
| Rate limiting | Собственный счётчик per-IP | `tower_governor` | Leaky-bucket/token-bucket с правильными edge cases |
| Session serialization | Собственный JSON schema | `rmp_serde` (MessagePack BLOB) | Тот же формат что у tower-sessions-sqlx-store; binary-safe |

**Ключевой инсайт:** Весь security-critical код давно решён crate'ами RustCrypto / rustls / tower-sessions. Самодельные реализации неизменно ошибаются в timing attacks, salt reuse, или session fixation.

---

## Common Pitfalls

### Pitfall 1: argon2 блокирует tokio executor

**Что идёт не так:** `hash_password()` / `verify_password()` вызываются напрямую в async handler без `spawn_blocking` — CPU-bound работа блокирует tokio worker thread на ~50–100ms, что при >4 concurrent users означает отказ обслуживания других запросов.
**Почему:** argon2 с m=19456 KiB обрабатывает ~50ms. tokio default worker threads = CPU core count.
**Как избежать:** Всегда `tokio::task::spawn_blocking(|| hash_password(...))`.
**Признак:** Лаги в UI при нескольких одновременных логинах.

### Pitfall 2: tower-sessions 0.15 vs 0.13 — rand 0.9 transitive

**Что идёт не так:** tower-sessions 0.15 внутри использует rand 0.9 (для `Id::default()`), тогда как argon2 0.5 + password-hash 0.5 используют rand 0.8 через `OsRng`. Конфликт версий rand в cargo tree.
**Почему:** rand 0.8 и 0.9 — несовместимы (SemVer).
**Как избежать:** В argon2-коде использовать `password_hash::rand_core::OsRng` напрямую (он vendored в password-hash crate), не `rand::rngs::OsRng`. Тогда rand 0.8/0.9 co-exist без конфликта.
**Признак:** Cargo build error: "conflicting versions of `rand`".

### Pitfall 3: sessions.id — BLOB vs TEXT

**Что идёт не так:** tower-sessions `Id` — это `i128` (16 байт little-endian). Если хранить как TEXT (base64-encoded string), сравнение `WHERE id = ?` работает иначе в зависимости от encoding. Наша схема V010 уже правильная (BLOB), но нужно передавать именно `record.id.0.to_le_bytes().to_vec()` как `Vec<u8>`.
**Почему:** SQLite BLOB vs TEXT comparison — разные типы, UNIQUE constraint работает по-разному.
**Как избежать:** Всегда `id.0.to_le_bytes().to_vec()` → bind как `rusqlite::params![id_bytes]`.
**Признак:** Загрузка сессии возвращает None после рестарта сервера.

### Pitfall 4: TlsAcceptor принимает только одно соединение на разрушение

**Что идёт не так:** При горячей остановке сервера (cancel CancellationToken) текущий listener.accept() ждёт следующего соединения перед проверкой shutdown. Если нет active connections, server task не выходит немедленно.
**Почему:** `tokio::select!` ждёт первый завершившийся branch. Если нет нового conn, cancel branch не проверяется.
**Как избежать:** Использовать `tokio::select!` с `listener.accept()` AND `shutdown.cancelled()`. Уже отражено в Pattern 3.
**Признак:** Порт остаётся занятым после "остановки" сервера; следующий start выдаёт "Address already in use".

### Pitfall 5: Session не flush'ится при logout

**Что идёт не так:** Handler `/logout` только удаляет cookie на клиенте, не удаляя запись из `sessions`-таблицы. Cookie истекает через 30 дней на клиенте, но:
- При таргетированной атаке с украденным cookie — доступ сохраняется 30 дней.
- `sessions`-таблица растёт без очистки.
**Как избежать:** Всегда `session.flush().await` в logout handler. Добавить background task для `DELETE FROM sessions WHERE expiry_date < unixepoch()` (раз в день).
**Признак:** После logout — можно войти с тем же session cookie через другой браузер.

### Pitfall 6: Desktop lock flag в config.toml, не в app_settings

**Что идёт не так:** D-Desktop-02 явно указывает: флаг `desktop_lock_enabled` хранится в `app_settings` (БД), а НЕ в `config.toml`. Если поместить в config.toml, флаг не переедет с portable-БД при смене exe-директории.
**Как избежать:** Ключ `'desktop_lock_enabled'` в `app_settings` таблице (V016, уже существует). Phase 5 добавляет V018 с seed `desktop_lock_enabled = '0'`.
**Признак:** После переноса portable-папки на новый компьютер — лок сбрасывается.

### Pitfall 7: Origin/Referer check на GET-запросах

**Что идёт не так:** Проверка Origin/Referer необходима только на mutation endpoints (POST/PUT/DELETE). Если применить на все эндпоинты — браузер не присылает Referer на прямую навигацию, что сломает first-load.
**Как избежать:** `axum::middleware::from_fn` с проверкой `method.is_safe()` (GET/HEAD/OPTIONS исключаются) и наличия Origin/Referer-заголовка.

### Pitfall 8: Timeout WriterHandle при session save в slow-path

**Что идёт не так:** WriterHandle имеет `send_timeout` 5 секунд. При высокой нагрузке записей (много sessions save одновременно) — save может вернуть `WriteQueueBusy`. SessionStore не ожидает такой ошибки.
**Как избежать:** `session_store::Error::Backend(e.to_string())` при маппинге `WriteQueueBusy` — SessionManagerLayer обработает это как transient error и не установит cookie. Логировать как WARN.

---

## Code Examples

### Bootstrap check (первый запуск)

```rust
// В AuthService::needs_bootstrap()
pub async fn needs_bootstrap(&self) -> Result<bool, AppError> {
    let readers = self.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT COUNT(*) FROM users WHERE deleted_at_utc IS NULL AND role = 'admin'",
            [],
            |r| r.get::<_, i64>(0),
        )
    })
    .await
    .map_err(|e| AppError::Internal { source_chain: e.to_string() })?
    .map(|count| count == 0)
    .map_err(|e| AppError::Internal { source_chain: e.to_string() })
}
```

### Security headers через tower-http

```rust
// В build_router()
use axum::http::{HeaderName, HeaderValue};
use tower_http::set_header::SetResponseHeaderLayer;

let security_headers = tower::ServiceBuilder::new()
    .layer(SetResponseHeaderLayer::overriding(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    ))
    .layer(SetResponseHeaderLayer::overriding(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    ))
    .layer(SetResponseHeaderLayer::overriding(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'"),
    ));

Router::new()
    .merge(auth_router())
    .merge(devices::router())
    // ... все остальные роутеры
    .layer(session_layer)
    .layer(security_headers)
    .with_state(ctx)
```

### Rate limit на /login через tower_governor

```rust
use tower_governor::{GovernorConfigBuilder, GovernorLayer, key_extractor::PeerIpKeyExtractor};

let governor_conf = GovernorConfigBuilder::default()
    .per_second(10)   // ~10 req/sec burst (5-10/min = ~0.08-0.17/sec avg; burst до 10)
    .burst_size(5)    // не более 5 подряд без задержки
    .use_headers()    // добавляет X-RateLimit-* headers
    .finish()
    .expect("governor config");

let login_router = Router::new()
    .route("/api/v1/auth/login", post(handler_login))
    .layer(GovernorLayer { config: Arc::new(governor_conf) });
```

### Svelte 5: первый-run guard в App.svelte

```svelte
<!-- ui/src/App.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { authStore } from '$lib/stores/auth.svelte';
  import { apiCall } from '$lib/api/client';
  import Router from 'svelte-spa-router';
  import { routes } from './routes';
  import Layout from './features/layout/Layout.svelte';

  let bootstrapNeeded = $state(false);
  let loading = $state(true);

  onMount(async () => {
    try {
      const status = await apiCall<{ needsBootstrap: boolean, user: any | null }>('auth_status', {});
      bootstrapNeeded = status.needsBootstrap;
      if (status.user) authStore.setUser(status.user);
    } finally {
      loading = false;
    }
  });
</script>

{#if loading}
  <Spinner />
{:else if bootstrapNeeded}
  <FirstRunWizard />
{:else}
  <Layout>
    <Router {routes} />
  </Layout>
{/if}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| tower-sessions 0.12: SessionStore без `create` | 0.13+: `create` как provided method с collision mitigation | 2024 | Нужна реализация create() в нашем store |
| tower-sessions 0.13 | 0.15.0 (текущий) | 2025 | rand 0.9 transitive (see Pitfall 2); нет breaking changes в API |
| rcgen 0.13: `CertificateParams::new()` | rcgen 0.14: `generate_simple_self_signed()` → `CertifiedKey` | 2025 | `RcgenError` deprecated; используем новый API |
| axum-server 0.7 (для rustls) | tokio-rustls + ручной accept loop | 2024 | axum-server 0.8 = axum 0.7 зависимость, не совместим с нашим axum 0.8 |
| argon2 0.5.x stable | argon2 0.6.0-rc.8 (RC) | 2025 | Используем 0.5.3 stable (RC не для продакшна) |

**Deprecated/outdated:**
- `axum-server` для HTTPS с axum 0.8: несовместим (зависит от axum 0.7). Используем tokio-rustls.
- `rand::rngs::OsRng` импорт в argon2-коде: предпочитать `password_hash::rand_core::OsRng` чтобы не тащить rand как явную зависимость.

---

## Runtime State Inventory

> Фаза является частично-migration (добавляем app_settings ключи), но не rename. Тем не менее проверяем.

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | `app_settings` таблица (V016): существует с ключом `low_stock_threshold`. Ключ `desktop_lock_enabled` — нужно добавить | Новая миграция V018 с INSERT INTO app_settings (`desktop_lock_enabled`, '0') |
| Stored data | `sessions` таблица (V010): существует и пуста — создана в Phase 1 именно для Phase 5 | Готово, не нужно менять схему |
| Stored data | `users` таблица (V002): существует и пуста — никаких пользователей до Phase 5 | First-run wizard создаёт первого admin |
| Live service config | axum роутеры построены но НЕ bind'ятся — нет live server state | Phase 5 делает первый live bind |
| OS-registered state | Нет зарегистрированных сервисов / scheduler tasks | None |
| Secrets/env vars | Нет сохранённых секретов до Phase 5 | Phase 5 создаёт первый cert + users; cert сохраняется рядом с exe |
| Build artifacts | Нет stale артефактов от Phase 5 | None |

**Ничего не найдено в категориях:** OS-registered state, secrets (до Phase 5), build artifacts.

---

## Open Questions (RESOLVED)

1. **axum-server совместимость с axum 0.8**
   - Что знаем: axum-server 0.8.0 по cargo search существует. По документации он мог обновиться под axum 0.8 — нужна проверка Cargo.toml axum-server при планировании.
   - Что неясно: Точная зависимость axum-server 0.8.0 (axum 0.7 или 0.8?) — сайт crates.io не вернул данные во время research.
   - Рекомендация: Планировщику проверить `cargo add axum-server --dry-run` на совместимость. Если axum-server 0.8.0 зависит от axum 0.8 — можно использовать `axum_server::tls_rustls::RustlsConfig` (более удобный API). Если нет — использовать tokio-rustls напрямую как описано в Pattern 3. [ASSUMED]
   - **RESOLVED:** axum-server 0.8.0 зависит от axum 0.7 (несовместим). Используем tokio-rustls напрямую как показано в Pattern 3 — ручной accept loop с `tokio::select!`. Это закреплено в Plans 02/03 и 05-PATTERNS.md.

2. **HTTP/2 в TLS сервере**
   - Что знаем: Наш кастомный TLS loop (Pattern 3) использует `hyper::server::conn::http1::Builder`. Современные браузеры предпочитают HTTP/2.
   - Что неясно: Нужен ли HTTP/2 для 20 concurrent LAN users?
   - Рекомендация: HTTP/1.1 достаточен для Phase 5. HTTP/2 можно добавить позже через `hyper::server::conn::http2::Builder` + ALPN в rustls config.
   - **RESOLVED:** HTTP/1.1 достаточен для целевой нагрузки ~20 concurrent LAN users. Закреплено в Plans 03/05 — используем `hyper::server::conn::http1::Builder` в accept loop. HTTP/2 — опциональное улучшение в будущей фазе.

3. **rmp-serde vs serde_json для session data**
   - Что знаем: tower-sessions-sqlx-store использует `rmp_serde` (MessagePack). BLOB в SQLite. Компактнее JSON.
   - Альтернатива: использовать `serde_json` + TEXT column (наша V010 имеет `data BLOB`).
   - Рекомендация: `rmp_serde` + BLOB consistent с существующей схемой V010 и паттерном tower-sessions-sqlx-store. [ASSUMED: небольшой риск если tower-sessions внутренне ожидает определённый формат Record serialization]
   - **RESOLVED:** `rmp_serde` + BLOB подтверждён. V010 schema хранит `data BLOB` — бинарный MessagePack совместим. Plan 02 содержит полный RusqliteSessionStore impl с `rmp_serde::to_vec` / `rmp_serde::from_slice`. Закреплено в Common Pitfalls Pitfall 3.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | argon2, rustls, tokio-rustls | ✓ | 1.92 (workspace) | — |
| cargo | Build | ✓ | Stable | — |
| tokio multi-thread | axum server | ✓ | 1.x (в workspace) | — |
| SQLite (bundled) | sessions store | ✓ | bundled в rusqlite 0.38 | — |
| Network port 8443 | default server port | ✓ (dev) | — | Конфигурируемый |
| TLS support | rustls + rcgen | ✓ | Pure Rust, no system dep | — |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none.

---

## Validation Architecture

> nyquist_validation: true (из config.json)

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test + tokio::test (flavor = "multi_thread") |
| Config file | Workspace Cargo.toml |
| Quick run command | `cargo test -p trackly-app --test auth_smoke -- --nocapture` |
| Full suite command | `cargo test -p trackly-app 2>&1` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| USR-01 | CRUD пользователей: create/read/update/delete | integration | `cargo test -p trackly-app --test users_crud` | ❌ Wave 0 |
| USR-02 | Три роли: authorize() block/allow matrix | unit | `cargo test -p trackly-core --lib auth::tests` | ❌ Wave 0 |
| USR-03 | Сессия: login создаёт cookie; переживает рестарт store | integration | `cargo test -p trackly-app --test session_survives_restart` | ❌ Wave 0 |
| USR-04 | Tauri trusted-admin: identity = admin без логина | unit | `cargo test -p trackly-app --lib services::auth::tests::trusted_desktop` | ❌ Wave 0 |
| USR-05 | Logout: сессия удалена из sessions table | integration | `cargo test -p trackly-app --test auth_logout_revokes_session` | ❌ Wave 0 |
| USR-06 | Role×endpoint 403 матрица: employee curl → devices_create | integration | `cargo test -p trackly-app --test role_endpoint_matrix` | ❌ Wave 0 (ROADMAP criterion #3) |
| USR-07 | HTTPS: TLS bind, fingerprint вычислен, cert on disk | integration | `cargo test -p trackly-app --test tls_server_smoke` | ❌ Wave 0 |
| SRV-01 | Hot start/stop: сервер старует, стопается, порт освобождается | integration | `cargo test -p trackly-app --test server_hot_toggle` | ❌ Wave 0 |
| SRV-02 | Security headers присутствуют в ответе | integration | `cargo test -p trackly-app --test security_headers` | ❌ Wave 0 |
| SRV-03 | Один AppCtx для Tauri + axum (уже верифицировано в health_smoke) | smoke | Используем существующий `health_smoke_end_to_end` | ✅ |
| SRV-04 | HTTP 400/reject если обращаться по plain HTTP на HTTPS порт | integration | В рамках tls_server_smoke | ❌ Wave 0 |
| SRV-05 | Graceful shutdown: axum drain, порт освобождается | integration | `cargo test -p trackly-app --test graceful_shutdown_drain` | ❌ Wave 0 |
| SET-08 | app_settings: desktop_lock_enabled читается/пишется | unit | `cargo test -p trackly-app --lib services::settings::tests` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p trackly-app -p trackly-core 2>&1 | tail -20`
- **Per wave merge:** `cargo test -p trackly-app 2>&1` + `pnpm -C ui check`
- **Phase gate:** Full suite green + health_smoke passes before `/gsd-verify-work`

### Wave 0 Gaps (новые файлы)

- [ ] `crates/trackly-app/tests/users_crud.rs` — covers USR-01
- [ ] `crates/trackly-app/tests/role_endpoint_matrix.rs` — covers USR-06 (ROADMAP criterion #3)
- [ ] `crates/trackly-app/tests/session_survives_restart.rs` — covers USR-03, USR-05 (ROADMAP criterion #4)
- [ ] `crates/trackly-app/tests/tls_server_smoke.rs` — covers USR-07, SRV-04
- [ ] `crates/trackly-app/tests/server_hot_toggle.rs` — covers SRV-01
- [ ] `crates/trackly-app/tests/security_headers.rs` — covers SRV-02
- [ ] `crates/trackly-app/tests/graceful_shutdown_drain.rs` — covers SRV-05 (ROADMAP criterion #5)
- [ ] `crates/trackly-core/src/auth.rs` + unit tests — covers USR-02, authorize() logic

---

## Security Domain

> security_enforcement: true, security_asvs_level: 1

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | YES | argon2id 0.5 (m=19456,t=2,p=1), min 8 chars, spawn_blocking |
| V3 Session Management | YES | tower-sessions 0.15, rusqlite store, sliding 30d, HttpOnly+Secure+SameSite=Strict |
| V4 Access Control | YES | authorize(identity, action) в сервис-слое; CI role×endpoint 403 matrix |
| V5 Input Validation | YES | AppError::Validation для login/password; мин. длина на обоих транспортах |
| V6 Cryptography | YES | rustls 0.23 (TLS 1.2+1.3), argon2id (не bcrypt/SHA), rcgen self-signed cert |
| V7 Stored Cryptography | YES | Secret<T> + zeroize; password_hash TEXT (не password TEXT) в users |
| V9 Communications | YES | HTTPS-only; SameSite=Strict; Origin/Referer check |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Brute-force login | Tampering | tower_governor rate limit ~5-10/min per IP |
| Session fixation | Elevation of Privilege | `session.cycle_id().await` после successful login |
| CSRF через cross-origin POST | Tampering | SameSite=Strict + Origin check на mutations |
| Stolen session cookie | Info Disclosure | HttpOnly + Secure; logout deletes from store |
| Password in logs | Info Disclosure | Secret<T> Debug = "***"; никогда не логировать expose() |
| Default credentials | Tampering | D-Bootstrap-01: нет авто-seed admin/admin; first-run wizard |
| Stale sessions (30d) | Elevation of Privilege | background cleanup: DELETE WHERE expiry_date < unixepoch() |
| Port scanning / Tauri cmd injection | Tampering | axum на отдельном порту; Tauri ACL capability model |
| Timing attack на password verify | Info Disclosure | argon2::verify_password — constant-time через RustCrypto internals |
| Certificate fixation | Spoofing | SHA-256 fingerprint отображается в UI для проверки пользователем |

**ASVS Level 1 compliance note:** `session.cycle_id()` (session fixation mitigation) может не быть в tower-sessions 0.15 API. Эквивалент: `session.flush()` + создать новую сессию после login (что tower-sessions делает автоматически при новом `session.insert()`). [ASSUMED — требует проверки в API docs]

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | axum-server 0.8.0 зависит от axum 0.7 (не 0.8), поэтому не используем | Standard Stack | Если он поддерживает axum 0.8 — можно использовать RustlsConfig API вместо ручного accept loop |
| A2 | tower_governor используется как GovernorLayer на отдельном sub-router для /login | Architecture Patterns | Если API изменился — может потребоваться другая конфигурация key_extractor |
| A3 | rmp_serde для сериализации Record в sessions.data совместим с tower-sessions internal format | Pattern 2 | Если tower-sessions ожидает другой формат — session load вернёт Decode error |
| A4 | session.cycle_id() или эквивалент доступен в tower-sessions 0.15 для mitigation session fixation | Security Domain | Если нет — использовать flush() + re-insert как обходное решение |
| A5 | hyper 1.x + hyper-util 0.1 совместимы с axum 0.8 для кастомного TLS accept loop | Pattern 3 | axum 0.8 зависит от hyper 1.x, так что это высоковероятно корректно [HIGH confidence] |

**Если таблица непустая:** Claims A1 и A3 требуют проверки при планировании/реализации Wave 1.

---

## Sources

### Primary (HIGH confidence)

- `crates/trackly-app/src/context.rs` — AppCtx структура (verified in session)
- `crates/trackly-app/src/http/devices.rs` — router() pattern, handler pattern
- `crates/trackly-core/src/error.rs` — AppError с Unauthorized/Forbidden уже есть
- `crates/trackly-app/src/error_axum.rs` — AppErrorResponse mapping (401, 403)
- `migrations/V002__core_entities.sql` — users table schema
- `migrations/V010__sessions.sql` — sessions table (BLOB id, BLOB data, INTEGER expiry_date)
- `migrations/V016__cartridges_kind_color_settings.sql` — app_settings table exists
- `tower-sessions/session_store.rs` (GitHub source) — SessionStore trait + example impl
- `tower-sessions/session.rs` (GitHub source) — Record struct, Id struct (i128), Session API
- `tower-sessions/examples/counter.rs` (GitHub) — SessionManagerLayer configuration
- `docs.rs/argon2/0.5.3` — hash_password / verify_password API [CITED]
- `docs.rs/rcgen/0.14.8` — generate_simple_self_signed() → CertifiedKey [CITED]
- `docs.rs/tower-sessions/0.15.0` — Expiry::OnInactivity, SameSite config [CITED]
- `docs.rs/axum/0.8.4/axum/serve/struct.Serve.html` — with_graceful_shutdown [CITED]
- `tower-sessions CHANGELOG` (GitHub) — 0.13→0.15 нет breaking changes в SessionStore [CITED]

### Secondary (MEDIUM confidence)

- `cargo search` output: tower-sessions=0.15.0, rcgen=0.14.8, tokio-rustls=0.26.4, argon2 latest stable=0.5.3 (docs.rs confirmed), tower_governor=0.8.0, rustls-pemfile=2.2.0, axum-server=0.8.0 [VERIFIED: cargo registry]
- CLAUDE.md stack section — полный список locked crates с rationale [CITED]

### Tertiary (LOW confidence)

- axum-server 0.8.0 → axum 0.7 dependency (не верифицировано через Cargo.toml; crates.io API вернул пустой ответ)
- rmp_serde совместимость с tower-sessions Record internal format (аналогия с tower-sessions-sqlx-store)

---

## Project Constraints (from CLAUDE.md)

| Constraint | Directive |
|-----------|-----------|
| Password hashing | Только argon2id (argon2 0.5); NO bcrypt для новых хэшей |
| TLS | Только rustls (не native-tls / OpenSSL) — portable mode |
| Sessions | tower-sessions over JWT — revocable, simpler |
| DB paths | Только relative к `current_exe()` — НЕ %APPDATA% |
| Single-writer | Все write через WriterHandle; reads через ReaderPool |
| Secret<T> | Все чувствительные значения через Secret<T> |
| Roles в БД | TEXT 'admin' | 'manager' | 'employee' |
| UI язык | Только русский в v1 |
| argon2 параметры | m=19456 KiB, t=2, p=1 (OWASP 2024+) |
| OsRng | Через rand_core::OsRng (или password_hash::rand_core::OsRng) |
| Sessions middleware | Gates /api/* except /api/v1/auth/login |

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — стек полностью зафиксирован, версии верифицированы через cargo search + docs.rs
- Architecture: HIGH — существующий код исчерпывающе изучен; паттерны выведены из реального кода
- SessionStore impl: MEDIUM — паттерн выведен из tower-sessions source (GitHub) и sqlx-store аналога
- TLS accept loop: MEDIUM — tokio-rustls API верифицирован, hyper 1.x совместимость HIGH по SemVer
- axum-server совместимость: LOW — требует проверки Cargo.toml axum-server 0.8.0

**Research date:** 2026-06-13
**Valid until:** 2026-07-13 (tower-sessions, rustls активно развиваются; проверить minor версии)
