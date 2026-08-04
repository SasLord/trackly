# Phase 32: Авто-админ по списку логинов + релиз SSO в main - Pattern Map

**Mapped:** 2026-08-03
**Files analyzed:** 5 (+1 CI/merge operational item, not a code pattern)
**Analogs found:** 5 / 5

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|--------------------|------|-----------|-----------------|---------------|
| `crates/trackly-infra/src/config.rs` (add `admin_logins: Vec<String>` to `AdConfig`) | config | transform (TOML→struct, deploy-time) | same file, `role_mapping: Vec<RoleMappingEntry>` field (`:229-233`) + its `Default`/`Debug`/tests | exact |
| `crates/trackly-app/src/services/auth.rs` (`with_admin_logins` builder + `is_admin_login` + forced-admin state machine + injection in `on_ad_bind_success`) | service | CRUD (state-transition on `users`/`requests`/`audit_log`) | same file: `on_ad_bind_success` (`:404`), `auto_register_ad_user` (`:531`), `create_pending_registration` (`:604`), `find_user_any_state` (`:1049`); builder analog `act_service.rs` `with_pdf_pipeline`/`with_org_db` (`:83-109`); UPDATE-shape analog `request_service.rs::approve_ad_register` (`:747-865`) | exact (provisioning) / role-match (builder, borrowed from a sibling service) |
| `crates/trackly-app/src/context.rs` (one builder call on `AuthService::new(...)` chain) | config/wiring | request-response (app bootstrap) | same file: `ActService::new(...).with_pdf_pipeline(...).with_org_db(...)` (`:275-279`) immediately preceding the `AuthService::new(...)` call (`:325-332`) | exact |
| new integration test file `crates/trackly-app/tests/ad_admin_logins.rs` (name TBD) | test | request-response / CRUD | `crates/trackly-app/tests/ad_directory_sso.rs` (helper + test shape); `ad_register.rs` (state-matrix coverage style); mock analog `crates/trackly-infra/src/ad/directory_mock.rs` `MockAdDirectory` | exact |
| `trackly.config.toml.example` (document `admin_logins`) | config/docs | — | same file, `role_mapping` block (`:69-79`) + `bind_password` privacy-warning block (`:52-60`) | exact |

## Pattern Assignments

### `crates/trackly-infra/src/config.rs` (config, transform)

**Analog:** same file, `role_mapping: Vec<RoleMappingEntry>` (field `:233`, `Default` `:285`, `Debug` `:262`, tests `:392-418`)

**Field + doc-comment pattern** (mirror exactly, but flat `Vec<String>` — simpler than `role_mapping`'s array-of-tables, no intermediate struct needed):
```rust
// crates/trackly-infra/src/config.rs:229-233 (existing role_mapping, the shape to mirror)
/// Таблица «AD-группа → роль», приоритет Admin > Manager > Employee
/// (highest-privilege-wins) независимо от порядка записей. Пустой список —
/// валидное стационарное состояние («маппинг не настроен»).
#[serde(default)]
pub role_mapping: Vec<RoleMappingEntry>,
```
New field to add right after it, same struct (`AdConfig`), same attribute:
```rust
#[serde(default)]
pub admin_logins: Vec<String>,
```
TOML shape is a **flat array**, not `[[ad.admin_logins]]` table-array (simpler than `role_mapping` since there's no per-entry struct):
```toml
[ad]
admin_logins = ["us100", "us777"]
```

**Default impl pattern** (`:267-288`, add one line):
```rust
impl Default for AdConfig {
    fn default() -> Self {
        Self {
            // ...existing fields...
            role_mapping: Vec::new(),
            admin_logins: Vec::new(),   // <- new, same style
        }
    }
}
```

**Debug impl pattern** (`:236-265`) — `admin_logins` has NO secrets (same as `role_mapping`), safe to print as-is; add one `.field(...)` line to the existing manual `impl std::fmt::Debug for AdConfig`:
```rust
.field("role_mapping", &self.role_mapping)
.field("admin_logins", &self.admin_logins)   // <- new, no redaction needed
.finish()
```

**Test pattern** (mirror `role_mapping_array_of_tables_deserializes`, `:392-418`, and the two default-empty tests `:329-337`/`:343-360`) — this exact style, adapted for the flat-array shape:
```rust
// New test, same file, same #[cfg(test)] mod tests block (:322-419)
#[test]
fn admin_logins_flat_array_deserializes_and_defaults_empty() {
    let empty: AppConfig = toml::from_str("").expect("empty config parses");
    assert_eq!(empty.ad.admin_logins, Vec::<String>::new());

    let toml_str = "[ad]\n\
         enabled = true\n use_mock = false\n host = \"dc1.example.local\"\n \
         port = 636\n domain = \"example.local\"\n base_dn = \"dc=example,dc=local\"\n \
         name_attr = \"displayName\"\n no_tls_verify = false\n \
         admin_logins = [\"us100\", \"us777\"]\n";
    let cfg: AppConfig = toml::from_str(toml_str).expect("admin_logins parses");
    assert_eq!(cfg.ad.admin_logins, vec!["us100".to_string(), "us777".to_string()]);
}
```

**Error handling:** none needed here — `serde`'s `#[serde(default)]` already handles the absent-field/empty-list case identically to `role_mapping`; `AppConfig::load_or_default` (`:298-319`) is unchanged (parse errors already routed to `AppError::Validation { field, message }` for the whole file, not per-field).

---

### `crates/trackly-app/src/services/auth.rs` (service, CRUD)

**Imports** (no new imports needed — `rusqlite::OptionalExtension`, `trackly_core::auth::Role`, `Arc` are all already imported at the top of the file, `:13-35`). Add `std::collections::HashSet` if not already present as a bare import (currently only `std::sync::Arc` is imported at top-level — use fully-qualified `std::collections::HashSet` in the struct field type or add the import).

**Struct + constructor pattern to extend** (`:143-179`):
```rust
// crates/trackly-app/src/services/auth.rs:143-179 (existing, extend with one field)
#[derive(Clone)]
pub struct AuthService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) ad_client: Arc<dyn AdClient + Send + Sync>,
    pub(crate) directory: Arc<dyn AdDirectory + Send + Sync>,
    pub(crate) ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>,
    // NEW: pub(crate) admin_logins: Arc<std::collections::HashSet<String>>,
}

impl AuthService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
        ad_client: Arc<dyn AdClient + Send + Sync>,
        ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>,
        directory: Arc<dyn AdDirectory + Send + Sync>,
    ) -> Self {
        Self {
            writer, readers, clock, ad_client, ws_tx, directory,
            // NEW: admin_logins: Arc::new(std::collections::HashSet::new()),
        }
    }
}
```
**Do NOT add a 7th positional constructor argument** — this is the exact problem `ActService`'s builder methods were introduced to avoid (see Builder pattern below). 9 existing `AuthService::new(...)` call sites (`context.rs` + 8 test files including `ad_directory_sso.rs:40-47` shown above) must keep compiling unchanged.

**Builder pattern** (copy the precedent, `crates/trackly-app/src/services/act_service.rs:83-109`):
```rust
// crates/trackly-app/src/services/act_service.rs:103-109 (existing precedent to mirror)
/// Builder: подключить `OrgDbService` (D-05) — источник org-реквизитов для
/// act-рендера. Отдельный builder-метод, чтобы не ломать существующие
/// call sites `with_pdf_pipeline(templates, organization, pdf)`.
pub fn with_org_db(mut self, org_db: Arc<OrgDbService>) -> Self {
    self.org_db = Some(org_db);
    self
}
```
New method on `AuthService`, same shape (note: field is `Arc<HashSet<...>>`, not `Option<...>`, since empty set is itself a valid "off" default — matches `role_mapping`'s "empty = off" convention, not `act_service`'s `Option`-for-not-yet-wired convention):
```rust
/// Builder: настроить список доверенных доменных логинов, получающих
/// принудительную роль admin при AD-bind (Phase 32, SSO-02). Пустой список
/// (дефолт из `new()`) = фича выключена — существующие call sites/тесты не
/// затронуты.
pub fn with_admin_logins(mut self, logins: Vec<String>) -> Self {
    self.admin_logins = Arc::new(
        logins.iter().map(|l| normalize_login_for_admin_check(l)).collect(),
    );
    self
}
```

**Normalization helper pattern** (independent copy, per this codebase's established convention — mirror `crates/trackly-infra/src/ad/directory.rs:64-79`'s `cache_key` function verbatim, do NOT import it — it's private to `trackly-infra` and D-10 requires this check to stay structurally decoupled from the directory adapter):
```rust
// crates/trackly-infra/src/ad/directory.rs:69-79 (existing — logic to duplicate, not import)
fn cache_key(sam_account_name: &str) -> String {
    let without_upn_suffix = sam_account_name
        .split('@')
        .next()
        .unwrap_or(sam_account_name);
    let without_netbios_prefix = without_upn_suffix
        .rsplit('\\')
        .next()
        .unwrap_or(without_upn_suffix);
    without_netbios_prefix.to_lowercase()
}
```
Same file (`crates/trackly-infra/src/ad/directory_mock.rs:69-79`) has a second independent copy (`MockAdDirectory::lookup_key`) confirming this is the established "small independent adapters" convention, not an oversight — write a 3rd/4th copy in `auth.rs` as a free function, e.g. `normalize_login_for_admin_check`.

**Core injection-point pattern** (`on_ad_bind_success`, `:404-439` — inject a new branch at the TOP, existing match UNCHANGED per D-08):
```rust
// crates/trackly-app/src/services/auth.rs:404-439 (existing — the match to wrap, not rewrite)
async fn on_ad_bind_success(
    &self,
    login: &str,
    display_name: &str,
    role_hint: Option<Role>,
) -> Result<UserDto, AppError> {
    match self.find_user_any_state(login).await? {
        Some(found) if found.is_active && !found.deleted => self.get_by_login(login).await,
        Some(pending)
            if !pending.is_active && !pending.deleted && pending.has_open_register_request =>
        {
            self.reuse_or_create_pending_registration(pending.id, login, display_name).await
        }
        Some(blocked_or_deleted) => self.report_blocked_access(blocked_or_deleted.id).await,
        None => {
            if self.ad_auto_accept().await? {
                self.auto_register_ad_user(login, display_name, role_hint).await
            } else {
                self.create_pending_registration(login, display_name, role_hint).await
            }
        }
    }
}
```
Both call sites feed into it unchanged: `sso_login` (`:292-323`, passwordless SSO — `self.on_ad_bind_success(ad_username, &resolved_display_name, role_hint).await` at `:321-322`) and `try_ad_login` (`:366-390`, LDAPS bind — `self.on_ad_bind_success(&req.login, &display_name, None).await` at `:386-387`). Injecting the `is_admin_login` check at the top of `on_ad_bind_success` itself (not inside either caller) makes both entry points get identical treatment for free — this is the DRY option research recommends (Open Question #1 in RESEARCH.md, still open for planner/user to lock explicitly).

**Read-seam pattern to reuse for state detection** — `find_user_any_state` (`:1049-1087`) already returns exactly the shape needed to branch the 5-state matrix (unknown/pending/blocked-or-deleted/active-non-admin/active-admin):
```rust
// crates/trackly-app/src/services/auth.rs:1049-1087 (existing, reuse as-is)
pub async fn find_user_any_state(&self, login: &str) -> Result<Option<UserAnyState>, AppError> {
    // ... returns Option<UserAnyState { id, role, is_active, deleted, has_open_register_request }>
}
```
`UserAnyState` struct (`:120-134`) already has every field the forced-admin branch needs (`role: String`, `is_active: bool`, `deleted: bool`, `has_open_register_request: bool`) — no new read query required, just new branching logic on the existing return shape.

**Writer-transaction + INSERT shape for the "unknown login" branch** (mirror `auto_register_ad_user`, `:531-598`, but with `role` hardcoded to `"admin"` and `is_active=1`, and — per D-04 — WITHOUT the extra `ad_register`/`requests` INSERT that normal auto-accept does, since this is a bypass path, not an auto-accept-with-info-request path):
```rust
// crates/trackly-app/src/services/auth.rs:547-554 (existing INSERT shape to mirror,
// omit the requests/ad_register block below it for the forced-admin unknown case)
tx.execute(
    "INSERT INTO users \
     (login, full_name, password_hash, role, ad_user, is_active, \
      created_at_utc, updated_at_utc, version) \
     VALUES (?1, ?2, NULL, ?4, 1, 1, ?3, ?3, 1)",
    rusqlite::params![login_owned, display_name_owned, now, role],
)
.map_err(map_rusqlite)?;
let user_id = tx.last_insert_rowid();
```

**UPDATE shapes for the pending/blocked/active-non-admin branches** — mirror `request_service.rs::approve_ad_register`'s existing "restore"/"register" branches EXACTLY (this is the single most important copy-not-invent instruction from research):
```rust
// crates/trackly-app/src/services/request_service.rs:783-788 (existing —
// "restore" branch: blocked/soft-deleted → active + role change + revive)
UPDATE users SET role = ?1, is_active = 1, deleted_at_utc = NULL, \
     updated_at_utc = ?2, version = version + 1 WHERE id = ?3

// crates/trackly-app/src/services/request_service.rs:790-795 (existing —
// "register" branch: pending (never active) → active + role change, no deleted_at_utc touch)
UPDATE users SET role = ?1, is_active = 1, \
     updated_at_utc = ?2, version = version + 1 WHERE id = ?3
```
For the "active, role≠admin" escalation case (D-06), the same shape minus `is_active`:
```sql
UPDATE users SET role = ?1, updated_at_utc = ?2, version = version + 1 WHERE id = ?3
```
For "active, role=admin already" — **NO-OP**, skip the write entirely (idempotency — do not bump `version`/`updated_at_utc` on every login for an already-admin user; see Anti-Patterns in RESEARCH.md).

**Closing the dangling pending request** (Pitfall 2 — mirror the exact completion UPDATE, `request_service.rs:820-826`):
```rust
// crates/trackly-app/src/services/request_service.rs:819-827 (existing shape to mirror
// in the SAME writer tx that promotes a "pending" user to forced-admin)
let affected = tx
    .execute(
        "UPDATE requests SET status = 'completed', updated_at_utc = ?1, \
         version = version + 1 \
         WHERE id = ?2 AND version = ?3 AND status = 'open' \
           AND deleted_at_utc IS NULL",
        rusqlite::params![now, request_id, version],
    )
    .map_err(map_rusqlite)?;
```
For the forced-admin path there is no caller-supplied `version` to optimistic-lock against (this is a system-triggered transition, not an admin UI action) — use an unconditional `WHERE ... AND status='open' AND deleted_at_utc IS NULL` (no `version = ?` clause) to close whatever open `ad_register`/`register` request exists for that user, in the same transaction as the `users` UPDATE.

**`audit_log` INSERT shape** (mirror `auto_register_ad_user`'s pattern, `:557-564`, and `approve_ad_register`'s pattern, `:798-810`, which additionally captures `payload_json`):
```rust
// crates/trackly-app/src/services/auth.rs:557-564 (existing — no-payload variant)
tx.execute(
    "INSERT INTO audit_log \
     (entity_type, entity_id, action, user_id, before_json, after_json, \
      payload_json, created_at_utc) \
     VALUES ('user', ?1, 'ad_auto_register', ?1, NULL, NULL, NULL, ?2)",
    rusqlite::params![user_id, now],
)
.map_err(map_rusqlite)?;

// crates/trackly-app/src/services/request_service.rs:798-810 (existing —
// WITH payload_json, the shape to prefer here per D-07's "security-significant,
// needs a durable trail" requirement — capture prior state)
tx.execute(
    "INSERT INTO audit_log \
     (entity_type, entity_id, action, user_id, before_json, after_json, \
      payload_json, created_at_utc) \
     VALUES ('user', ?1, 'ad_register_approve', ?2, NULL, NULL, ?3, ?4)",
    rusqlite::params![
        target_user_id,
        user_id,
        serde_json::json!({ "role": role_for_tx }).to_string(),
        now
    ],
)
.map_err(map_rusqlite)?;
```
Recommended for the new path: action string `'ad_auto_admin'` (per CONTEXT.md discretion), `payload_json: {"prior_state": "unknown"|"pending"|"blocked"|"active_employee"|"active_manager"}` (RESEARCH.md Open Question #2 recommendation), `user_id` param = the affected user's own id (no separate "caller" identity exists for this system-triggered path — unlike `approve_ad_register` where `user_id` is the *admin* who approved; here there is no human admin actor, use the target `user_id` itself, consistent with `auto_register_ad_user`'s own `?1, ?1` self-referential pattern at `:561-562`).

**Error handling:** all writer-tx code follows the exact same `.map_err(map_rusqlite)?` + `tx.commit().map_err(map_rusqlite)?` convention seen in every example above — no new error type needed. `self.writer.execute(move |conn| { ... }).await?` is the outer wrapper (see `auto_register_ad_user:542-588` for the full closure shape including the `move` capture pattern for owned `String`/`i64` locals).

**Test file location for inline unit test of `normalize_login_for_admin_check`/`is_admin_login`:** this file already has a `#[cfg(test)] mod tests` block with async `sso_login_*` tests at `:1836` onward (`sso_login_resolves_known_user_and_role_via_mock_directory`, `sso_login_degrades_role_when_directory_unreachable`) — add unit tests for the pure normalization function alongside these, or as plain `#[test]` (non-async) functions since normalization has no I/O.

---

### `crates/trackly-app/src/context.rs` (wiring, request-response)

**Analog:** same file, `ActService::new(...).with_pdf_pipeline(...).with_org_db(...)` builder chain (`:275-279`), immediately followed by the `AuthService::new(...)` call this phase must extend (`:325-332`).

**Pattern to copy** (one new line on the existing chain):
```rust
// crates/trackly-app/src/context.rs:275-279 (existing precedent — multi-builder chain style)
let acts = Arc::new(
    ActService::new(writer.clone(), readers.clone(), clock.clone())
        .with_pdf_pipeline(templates.clone(), organization.clone(), pdf.clone())
        .with_org_db(org_db.clone()),
);

// crates/trackly-app/src/context.rs:325-332 (existing — the call site to extend)
let auth = Arc::new(AuthService::new(
    writer.clone(),
    readers.clone(),
    clock.clone(),
    ad_client,
    ws_broadcast.clone(),
    directory,
));
```
New version (wrap the constructor call in a builder chain, same style as `acts` above):
```rust
let auth = Arc::new(
    AuthService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        ad_client,
        ws_broadcast.clone(),
        directory,
    )
    .with_admin_logins(config.ad.admin_logins.clone()),
);
```
`config` is already in scope at this point in `AppCtx::build` (it's the source of `config.ad.use_mock`, `config.ad.clone()` used just above at `:291-314` for `ad_client`/`directory` selection) — no new plumbing needed to get `admin_logins` here.

---

### `crates/trackly-app/tests/ad_admin_logins.rs` (new, test/integration)

**Analog:** `crates/trackly-app/tests/ad_directory_sso.rs` (helper + fixture-injection style) and `ad_register.rs` (state-matrix coverage breadth).

**Helper pattern to copy** (mirror `ad_directory_sso.rs:33-64` — independent helper, do NOT reuse `ad_auth.rs`'s helper per that file's own documented "small independent fixtures" convention):
```rust
// crates/trackly-app/tests/ad_directory_sso.rs:33-64 (existing — copy this shape,
// add a builder call for with_admin_logins)
fn make_auth_service_with_directory(
    ad_client: Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync>,
    directory: Arc<dyn AdDirectory + Send + Sync>,
) -> (AuthService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let (ws_tx, _) = tokio::sync::broadcast::channel(128);
    let svc = AuthService::new(writer, readers, clock, ad_client, Arc::new(ws_tx), directory);
    (svc, dir)
}
```
New helper for this test file adds `.with_admin_logins(vec!["us100".to_string()])` (or similar) on the constructed `AuthService` before returning it — this is exactly the isolation the builder pattern was designed for: only tests that explicitly opt in get a non-empty `admin_logins` set, every other existing test (all 8 other call sites) is unaffected by default.

**Header doc-comment convention to copy** (mirror `ad_directory_sso.rs:1-12` — Phase/plan attribution + explicit "no real org data" privacy note, and reuse of existing placeholder identities `us100`/`us200`/`us300` from `directory_mock.rs`, not new invented names):
```rust
//! Интеграционные тесты SSO-01/SSO-03 (Phase 31 Plan 04).
//! ...
//! Используются ТОЛЬКО уже существующие placeholder-идентичности из
//! `directory_mock.rs` (us100/us200) + новый неиспользуемый в фикстурах
//! placeholder-логин us300 ... — никаких реальных имён/доменов.
```

**Mock fixtures to reuse** (`crates/trackly-infra/src/ad/directory_mock.rs`, `MockAdDirectory::default_fixtures()` — `us100`→Manager, `us200`→no role — and `MockAdDirectory::unreachable()` for the D-10 "works even when directory unreachable" test case):
```rust
// crates/trackly-infra/src/ad/directory_mock.rs:38-58 (existing fixtures to reuse as-is)
pub fn default_fixtures() -> Self { /* us100 -> Manager, us200 -> None */ }
pub fn unreachable() -> Self { /* always Err(DirectoryError::Unreachable) */ }
```
For the "admin_logins forces admin even when directory unreachable" test, combine `MockAdDirectory::unreachable()` with `.with_admin_logins(vec!["us100"])` — `us100` is not in the mock's fixture map when using `unreachable()`, but that's fine: D-10 says the admin_logins check must NOT depend on `directory.resolve` succeeding at all.

**Test naming/assertion style to copy** (mirror `ad_directory_sso.rs:74-97`'s `#[tokio::test] async fn sso_login_resolves_known_user_display_name()` shape — section-comment banners per scenario, `.expect("...")` with descriptive messages, `assert_eq!` on `UserDto` fields):
```rust
#[tokio::test]
async fn admin_logins_unknown_user_becomes_active_admin_no_pending_request() {
    let (svc, _dir) = make_auth_service_with_admin_logins(vec!["us100".to_string()]);
    svc.set_ad_enabled(true, &admin_caller()).await.expect("enable AD");
    let dto = svc.sso_login("us100", "us100").await
        .expect("admin_logins bypass must not require ad_auto_accept");
    assert_eq!(dto.role, "admin");
    assert!(dto.is_active);
}
```

**Regression coverage to add to the SAME file (not modify existing files):** one new "not-in-list, admin_logins non-empty" case proving Phase 31 behavior is fully unchanged for logins outside the list (D-08) — construct the service with a non-empty `admin_logins` that does NOT contain the test login, and assert the existing Phase-31 branch outcome (e.g. `us200` still gets role `None`/`employee` default per existing `role_mapping` behavior).

---

### `trackly.config.toml.example` (config/docs)

**Analog:** same file, `role_mapping` block (`:69-79`) + the `bind_password` privacy-warning block (`:52-60`) for the "security-significant, document explicitly" tone.

**Pattern to copy** (comment style: Phase/req-id attribution, plain-language default, explicit warning where relevant):
```
# crates/../trackly.config.toml.example:69-79 (existing role_mapping block — style to mirror)
# Таблица «AD-группа → роль» (Phase 31, SSO-03). Приоритет при попадании
# пользователя в НЕСКОЛЬКО групп: Admin > Manager > Employee
# (highest-privilege-wins) НЕЗАВИСИМО от порядка записей ниже.
# Указывайте ПОЛНЫЙ distinguished name группы (не короткое имя) — избегает
# лишнего LDAP round-trip на резолв имени в DN.
# [[ad.role_mapping]]
# group_dn = "CN=IT-Admins,OU=Groups,DC=example,DC=local"
# role = "admin"
```
New block to add right after `role_mapping` (same `[ad]` section), combining `role_mapping`'s attribution style with `bind_password`'s explicit security-warning tone (D-07/D-11 explicitly flag this as security-significant, and Pitfall 3 requires the "no live reload" note):
```
# Список доменных логинов (Phase 32, SSO-02) — sAMAccountName, case-insensitive
# (us100, US100@example.local, EXAMPLE\us100 — все матчатся на "us100").
# Любой логин из этого списка получает роль admin И немедленную активацию
# на КАЖДОМ AD-входе, В ОБХОД ad_auto_accept и ручной блокировки/pending-
# заявки (побеждает над group→role маппингом role_mapping). Пустой список
# (по умолчанию) = фича полностью выключена.
# БЕЗОПАСНОСТЬ: кто редактирует этот файл — фактически может создать
# администратора; относитесь к доступу на запись в trackly.config.toml так
# же серьёзно, как к доступу на подмену .exe.
# Требует ПЕРЕЗАПУСКА процесса — изменения не подхватываются "на лету"
# (как и все остальные TOML-only [ad]-настройки).
# admin_logins = ["us100", "us777"]
```

---

## Shared Patterns

### Single-writer transaction discipline
**Source:** `crates/trackly-app/src/services/auth.rs::auto_register_ad_user` (`:542-588`) and `request_service.rs::approve_ad_register` (`:776-854`)
**Apply to:** the entire forced-admin state machine in `auth.rs` — every branch (INSERT for unknown, UPDATE for pending/blocked/active-non-admin) must go through `self.writer.execute(move |conn| { let tx = conn.transaction()...; ...; tx.commit()...; Ok(...) }).await?`, never a direct `conn.execute` outside a writer job. No exceptions — this is enforced project-wide (CLAUDE.md "SQLite WAL + single-writer pattern").
```rust
self.writer
    .execute(move |conn| {
        let tx = conn.transaction().map_err(map_rusqlite)?;
        // ...INSERT/UPDATE users + UPDATE requests (if closing pending) + INSERT audit_log...
        tx.commit().map_err(map_rusqlite)?;
        Ok(result)
    })
    .await?
```

### `version` optimistic-lock bump convention
**Source:** every `UPDATE users`/`UPDATE requests` in `request_service.rs` (e.g. `:784-785`, `:821-823`)
**Apply to:** every UPDATE branch in the new state machine EXCEPT the "already active admin" no-op case
```sql
... version = version + 1 WHERE id = ?n
```

### `audit_log` INSERT shape (mandatory for this phase per D-07/V9 ASVS)
**Source:** `request_service.rs::approve_ad_register` (`:798-810`, the payload_json-carrying variant)
**Apply to:** every branch of the forced-admin state machine, in the SAME transaction as the write it documents — never best-effort/fire-and-forget for this action (RESEARCH.md Security Domain section)
```rust
tx.execute(
    "INSERT INTO audit_log \
     (entity_type, entity_id, action, user_id, before_json, after_json, \
      payload_json, created_at_utc) \
     VALUES ('user', ?1, 'ad_auto_admin', ?1, NULL, NULL, ?2, ?3)",
    rusqlite::params![
        user_id,
        serde_json::json!({ "prior_state": prior_state_label }).to_string(),
        now
    ],
)
.map_err(map_rusqlite)?;
```

### Independent normalization copies (do not share private helpers across crates)
**Source:** `crates/trackly-infra/src/ad/directory.rs::cache_key` (`:69-79`) and `crates/trackly-infra/src/ad/directory_mock.rs::MockAdDirectory::lookup_key` (`:73-79`) — already two independent copies of the same UPN/NetBIOS-stripping + lowercase logic
**Apply to:** the new `normalize_login_for_admin_check` free function in `auth.rs` — write a 3rd independent copy, do not attempt to import/share (D-10 structural-decoupling requirement + established codebase convention)

### Builder-method extension of existing constructors (do not add new positional args)
**Source:** `crates/trackly-app/src/services/act_service.rs::with_pdf_pipeline`/`with_org_db` (`:91-109`)
**Apply to:** `AuthService::with_admin_logins` — protects the 9 existing `AuthService::new(...)` call sites (`context.rs` + `ad_auth.rs`, `ad_directory_sso.rs`, `ad_register.rs`, `auth_smoke.rs`, `specta_roundtrip.rs`, `users_crud.rs`, both `health.rs` files, one inline `#[cfg(test)]` site in `auth.rs`)

## No Analog Found

None — every file in this phase has a strong, exact analog already in the codebase. The only non-code deliverable without a "pattern" in the usual sense is the merge/release operational work (D-11/D-12):

| File/Task | Role | Data Flow | Reason no code analog applies |
|-----------|------|-----------|-------------------------------|
| `cargo fmt --all` drift fix (~15 files, pre-existing, verified failing) | chore/CI | — | Mechanical `rustfmt` re-format, not a hand-written pattern; must land as its own commit before/at merge (RESEARCH.md Pitfall 1) |
| Merge `spike/ad-sso-kerberos` → `main` + tag `v1.3.0` | release/CI | — | Git/GitHub Actions mechanics (`.github/workflows/release.yml` `v*.*.*` trigger, already verified in RESEARCH.md), not a code file to pattern-map |

## Metadata

**Analog search scope:** `crates/trackly-infra/src/config.rs`, `crates/trackly-app/src/services/auth.rs`, `crates/trackly-app/src/services/request_service.rs`, `crates/trackly-app/src/services/act_service.rs`, `crates/trackly-app/src/context.rs`, `crates/trackly-infra/src/ad/directory.rs`, `crates/trackly-infra/src/ad/directory_mock.rs`, `crates/trackly-app/tests/ad_directory_sso.rs`, `crates/trackly-core/src/auth.rs`, `trackly.config.toml.example`
**Files scanned:** 10 read directly (targeted ranges, no re-reads of overlapping lines) + line-number index via `grep -n` on `auth.rs`/`request_service.rs`
**Pattern extraction date:** 2026-08-03

---

*Phase: 32-sso-main*
