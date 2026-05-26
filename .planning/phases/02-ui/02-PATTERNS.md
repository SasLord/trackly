# Phase 2: Устройства и базовый UI — Pattern Map

**Mapped:** 2026-05-26
**Files analyzed:** ~46 новых / 7 модифицированных
**Analogs found:** 33 / 46 имеют сильный аналог в Phase 1; UI-файлы (13) — greenfield с минимальными подсказками от App.svelte/_tokens.scss

> Phase 2 — это **композиционная** фаза поверх Phase 1. Backend-плоскость почти полностью копирует паттерны Plan 04+05 (HealthDto / build_health / writer.execute / ReaderPool / map_rusqlite). UI-плоскость — greenfield, аналогов мало; для UI секция полагается на UI-SPEC.md + RESEARCH.md Pattern 5/8/9/10 как на источник истины.

---

## File Classification

### Rust backend (новые / модифицированные)

| Файл | Role | Data Flow | Closest Analog | Match Quality |
|------|------|-----------|----------------|---------------|
| `migrations/V013__devices_fts_triggers.sql` | migration | DDL | `migrations/V012__indexes_and_fts.sql` | exact (DDL + PRAGMA user_version) |
| `crates/trackly-core/src/domain/mod.rs` | module wiring | n/a | `crates/trackly-core/src/primitives/mod.rs` | role-match (модуль-агрегатор) |
| `crates/trackly-core/src/domain/devices.rs` | domain types | pure value | `crates/trackly-core/src/primitives/secret.rs` | role-match (pure types, no I/O) |
| `crates/trackly-core/src/ports/mod.rs` | module wiring | n/a | `crates/trackly-core/src/primitives/mod.rs` | role-match |
| `crates/trackly-core/src/ports/devices.rs` | port (trait) | n/a | `crates/trackly-core/src/primitives/clock.rs` (`trait Clock`) | role-match (trait в core, без I/O) |
| `crates/trackly-infra/src/repos/mod.rs` | module wiring | n/a | `crates/trackly-infra/src/db/mod.rs` | role-match |
| `crates/trackly-infra/src/repos/devices_sqlite.rs` | repository adapter | CRUD + read | `crates/trackly-infra/src/db/migrations.rs` (`run(conn: &mut Connection)`) + `crates/trackly-infra/src/db/pools.rs` (sync rusqlite) | role-match |
| `crates/trackly-app/src/services/mod.rs` | module wiring | n/a | `crates/trackly-app/src/dto/mod.rs` | role-match |
| `crates/trackly-app/src/services/device_service.rs` | service (composition) | request-response + spawn_blocking | `crates/trackly-app/src/context.rs::AppCtx::build` (writer.execute + readers.acquire usage) | role-match (новый слой — нет прямого аналога) |
| `crates/trackly-app/src/dto/device.rs` | DTO | serde | `crates/trackly-app/src/dto/health.rs` | **exact** (один и тот же паттерн `Serialize + Deserialize + Type`) |
| `crates/trackly-app/src/dto/mod.rs` | module wiring | n/a | существующий (модифицируется — добавить `pub mod device;`) | exact |
| `crates/trackly-app/src/tauri_cmds/devices.rs` | Tauri command | request-response | `crates/trackly-app/src/tauri_cmds/health.rs` | **exact** (build_* + #[tauri::command]) |
| `crates/trackly-app/src/tauri_cmds/mod.rs` | module wiring | n/a | существующий (мод.) | exact |
| `crates/trackly-app/src/http/devices.rs` | axum router | request-response | `crates/trackly-app/src/http/health.rs` | **exact** (Router<AppCtx>, State extractor) |
| `crates/trackly-app/src/http/mod.rs` | module wiring | n/a | существующий (мод.) | exact |
| `crates/trackly-app/src/csv/mod.rs` | module wiring | n/a | `crates/trackly-app/src/dto/mod.rs` | role-match |
| `crates/trackly-app/src/csv/sniff.rs` | utility (encoding detect) | transform | (нет аналога) | greenfield — следовать RESEARCH §Pattern 6 |
| `crates/trackly-app/src/csv/decode.rs` | utility | transform | (нет аналога) | greenfield — RESEARCH §Pattern 6 |
| `crates/trackly-app/src/csv/parse.rs` | utility | transform | (нет аналога) | greenfield — RESEARCH §Pattern 6 |
| `crates/trackly-app/src/csv/session_store.rs` | in-memory store (TTL) | state | (нет аналога) | greenfield — RESEARCH §Pattern 7 |
| `crates/trackly-app/src/context.rs` (MOD) | composition root | n/a | сам файл (расширение) | exact |
| `crates/trackly-app/src/specta_export.rs` (MOD) | bindings generator | n/a | сам файл (расширение) | exact |
| `crates/trackly-app/Cargo.toml` (MOD) | manifest | n/a | сам файл | exact |
| `Cargo.toml` (workspace, MOD) | manifest | n/a | сам файл (добавить chardetng/encoding_rs/csv/uuid pins) | exact |

### Rust tests (новые)

| Файл | Role | Data Flow | Closest Analog | Match Quality |
|------|------|-----------|----------------|---------------|
| `crates/trackly-app/tests/devices_crud.rs` | integration test | CRUD | `crates/trackly-app/tests/concurrent_writes.rs` | role-match (writer + readers fixture) |
| `crates/trackly-app/tests/devices_search.rs` | integration test | FTS read | `crates/trackly-app/tests/concurrent_writes.rs` | role-match |
| `crates/trackly-app/tests/devices_autocomplete.rs` | integration test | read | `crates/trackly-app/tests/concurrent_writes.rs` | role-match |
| `crates/trackly-app/tests/devices_grouping.rs` | integration test | read (GROUP BY) | `crates/trackly-app/tests/concurrent_writes.rs` | role-match |
| `crates/trackly-app/tests/devices_csv_import.rs` | integration test | CSV in | `crates/trackly-app/tests/concurrent_writes.rs` | role-match |
| `crates/trackly-app/tests/devices_csv_export.rs` | integration test | CSV out | `crates/trackly-app/tests/concurrent_writes.rs` | role-match |
| `crates/trackly-app/tests/devices_http_smoke.rs` | integration test | HTTP | `crates/trackly-app/tests/health_smoke.rs` | **exact** (full AppCtx::build + dual transport) |
| `crates/trackly-app/tests/export_bindings.rs` (MOD) | integration test | bindings | сам файл (расширить asserts) | exact |

### Frontend (Svelte 5 / TypeScript / SCSS) — все новые кроме помеченных

| Файл | Role | Data Flow | Closest Analog | Match Quality |
|------|------|-----------|----------------|---------------|
| `ui/index.html` (MOD) | shell HTML | n/a | сам файл (добавить inline no-flash script) | exact |
| `ui/package.json` (MOD) | manifest | n/a | сам файл (добавить deps) | exact |
| `ui/src/App.svelte` (REWRITE) | route shell | n/a | сам файл (текущий — placeholder из Phase 1) | partial (greenfield rewrite) |
| `ui/src/routes.ts` | router config | n/a | (нет аналога) | greenfield — RESEARCH §Pattern 10 |
| `ui/src/lib/api/client.ts` | transport adapter | request-response | `ui/src/bindings.ts` (генерируемый, паттерн `TAURI_INVOKE`) | partial (греет паттерн вызова) |
| `ui/src/lib/api/errors.ts` | utility | transform | (нет аналога) | greenfield |
| `ui/src/lib/api/devices.ts` | feature API client | request-response | (нет аналога) | greenfield — мапит на bindings.ts methods |
| `ui/src/lib/api/index.ts` | barrel | n/a | (нет аналога) | trivial |
| `ui/src/lib/stores/theme.svelte.ts` | runes store (module) | state | (нет аналога) | greenfield — RESEARCH §Pattern 9 |
| `ui/src/lib/stores/toast.svelte.ts` | runes store (module) | event-driven | (нет аналога) | greenfield — RESEARCH §Pattern 9 |
| `ui/src/lib/stores/transport.svelte.ts` | runes store (module) | state | (нет аналога) | greenfield — RESEARCH §Pattern 8 |
| `ui/src/lib/components/Button.svelte` | UI primitive | n/a | (нет аналога — `App.svelte` стилит сам) | greenfield — UI-SPEC §Component Inventory |
| `ui/src/lib/components/Input.svelte` | UI primitive | n/a | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/lib/components/Select.svelte` | UI primitive | n/a | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/lib/components/Textarea.svelte` | UI primitive | n/a | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/lib/components/Modal.svelte` | UI primitive | n/a | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/lib/components/Toast.svelte` | UI primitive | n/a | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/lib/components/ToastHost.svelte` | UI host | event-driven | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/lib/components/ThemeSwitcher.svelte` | UI primitive | event | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/lib/components/Placeholder.svelte` | UI primitive | n/a | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/lib/components/Spinner.svelte` | UI primitive | n/a | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/lib/components/Badge.svelte` | UI primitive | n/a | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/lib/utils/date.ts` | utility | transform | (нет аналога) | greenfield |
| `ui/src/features/layout/Layout.svelte` | layout shell | n/a | существующий `ui/src/App.svelte` (`<main>`) | partial |
| `ui/src/features/layout/Sidebar.svelte` | navigation | n/a | (нет аналога) | greenfield — UI-SPEC + RESEARCH §Pattern 10 |
| `ui/src/features/layout/sidebar-config.ts` | config | n/a | (нет аналога) | greenfield — UI-SPEC §Sidebar |
| `ui/src/features/devices/DevicesPage.svelte` | page | n/a | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/features/devices/DeviceList.svelte` | data view | request-response | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/features/devices/DeviceListRow.svelte` | row | n/a | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/features/devices/DeviceGroupRow.svelte` | row (expandable) | n/a | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/features/devices/DeviceFilters.svelte` | filters bar | event | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/features/devices/DeviceFormModal.svelte` | form | request-response | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/features/devices/DeviceAutocompleteField.svelte` | input + dropdown | request-response | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/features/devices/DeviceImportCsvModal.svelte` | wizard | request-response | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/features/devices/DeviceContextMenu.svelte` | kebab menu | event | (нет аналога) | greenfield — UI-SPEC |
| `ui/src/features/devices/api.ts` | feature API wrapper | request-response | (нет аналога) | greenfield |
| `ui/src/pages/{Dashboard,MapPage,ActsPage,PrintersPage,CartridgesPage,RequestsPage,ReportsPage,UsersPage,SettingsPage,NotFound}.svelte` | placeholder | n/a | (нет аналога) | trivial — 5-строчные `<Placeholder/>` обёртки |
| `ui/src/styles/_tokens.scss` (REWRITE) | tokens | n/a | сам файл (placeholder из Phase 1) | partial |
| `ui/src/styles/global.scss` | global styles | n/a | (нет аналога) | greenfield |

### CI / гигиена

| Файл | Role | Closest Analog | Match Quality |
|------|------|----------------|---------------|
| `.github/workflows/ci-fast.yml` (MOD) | CI | сам файл (убрать `continue-on-error` на svelte-check) | exact |
| `.github/workflows/ci-full.yml` (MOD) | CI | сам файл | exact |
| `.planning/phases/01-foundation/deferred-items.md` (MOD) | docs | сам файл (отметить resolved) | exact |

---

## Pattern Assignments

### `crates/trackly-app/src/dto/device.rs` (DTO, serde) — **exact match**

**Analog:** `crates/trackly-app/src/dto/health.rs`

**Imports + derive pattern** (analog lines 14, 18-19):
```rust
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct HealthDto {
    pub version: String,
    pub db_ready: bool,
    pub schema_version: u32,
}
```

**Применение к Phase 2:** копировать derive-набор для всех новых DTO (`DeviceDto`, `DeviceNew`, `DevicePatch`, `DeviceFilter`, `Pagination`, `DeviceGroup`, `CsvImportPreview`, `CsvImportReport`, `RowError`). Поле `STATE_HINTS` — `pub const STATE_HINTS: &[&str] = &[...]` (см. CONTEXT D-DeviceHints-01).

**Snake_case JSON invariant** (analog lines 47-58 в `#[test]` блоке) — frontend через bindings.ts ждёт snake_case. **Не добавлять** `#[serde(rename_all = "camelCase")]`.

**Каждый новый DTO должен иметь serde round-trip test** (analog lines 35-44):
```rust
#[test]
fn serde_round_trip_preserves_all_fields() {
    let dto = DeviceDto { /* ... */ };
    let json = serde_json::to_string(&dto).expect("serialize");
    let back: DeviceDto = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, dto);
}
```

---

### `crates/trackly-app/src/tauri_cmds/devices.rs` (Tauri command, request-response) — **exact match**

**Analog:** `crates/trackly-app/src/tauri_cmds/health.rs`

**Build helper + Tauri command pattern** (analog lines 21-37):
```rust
use crate::context::AppCtx;
use crate::dto::HealthDto;
use trackly_core::error::AppError;

pub async fn build_health(ctx: &AppCtx) -> HealthDto {
    HealthDto { /* ... */ }
}

#[tauri::command]
#[specta::specta]                // ← атрибут #[specta::specta] идёт ПОСЛЕ #[tauri::command]
pub async fn health(state: tauri::State<'_, AppCtx>) -> Result<HealthDto, AppError> {
    Ok(build_health(state.inner()).await)
}
```

**Применение к Phase 2:** на каждую из 12 device-команд (`devices_list`, `devices_get`, `devices_create`, `devices_update`, `devices_delete`, `devices_search`, `devices_autocomplete`, `devices_state_hints`, `devices_import_csv_preview`, `devices_import_csv_commit`, `devices_export_csv`, `devices_list_grouped`) — пара `build_<name>(ctx: &AppCtx, ...args) -> Result<Dto, AppError>` + thin `#[tauri::command] #[specta::specta]` wrapper.

**Test pattern для build_* helper** (analog lines 39-86):
```rust
async fn minimal_ctx() -> (AppCtx, TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf())
        .expect("resolve paths");
    let config = trackly_infra::AppConfig::default();
    let (_nb, log_guard) = tracing_appender::non_blocking(std::io::sink());
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let ctx = AppCtx {
        writer, readers,
        paths: Arc::new(paths),
        config: Arc::new(config),
        clock,
        shutdown: CancellationToken::new(),
        log_guard: Arc::new(log_guard),
        schema_version: 13,                          // ← bumped from 12 to 13 in Phase 2
        // devices: Arc::new(DeviceService::new(...)) — добавится в minimal_ctx
    };
    (ctx, dir)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn build_devices_get_returns_expected_fields() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        // ...
    })
    .await
    .expect("build_devices_get exceeded 30 s budget");
}
```

**Критично:** 30-секундный hard timeout вокруг `tokio::test` — обязательная защита против Linux-CI deadlock (analog line 77).

---

### `crates/trackly-app/src/http/devices.rs` (axum router, request-response) — **exact match**

**Analog:** `crates/trackly-app/src/http/health.rs`

**Router pattern** (analog lines 1-22):
```rust
use axum::{extract::State, routing::get, Json, Router};
use crate::context::AppCtx;
use crate::dto::HealthDto;
use crate::tauri_cmds::health::build_health;

pub async fn get_health(State(ctx): State<AppCtx>) -> Json<HealthDto> {
    Json(build_health(&ctx).await)
}

pub fn router() -> Router<AppCtx> {
    Router::new().route("/api/v1/health", get(get_health))
}
```

**Применение:** `http/devices.rs::router() -> Router<AppCtx>` с route'ами `POST /api/v1/devices_list`, `GET /api/v1/devices/{id}`, и т.д. Handler делегирует `build_<name>(&ctx, args).await` → `Json<Dto>` или `Result<Json<Dto>, AppError>` (для тех, что возвращают `Result`).

**axum 0.8 path syntax** — `{id}`, не `:id` (см. RESEARCH §Pattern 1 примечание).

**Router test pattern** (analog lines 24-86): `tower::ServiceExt::oneshot` + `axum::body::to_bytes`. См. также `tests/health_smoke.rs` для full-stack варианта.

---

### `crates/trackly-app/src/context.rs` (modification, +1 field)

**Analog:** сам файл (текущая структура — lines 33-55).

**Текущее объявление** (analog lines 33-55):
```rust
#[derive(Clone)]
pub struct AppCtx {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub paths: Arc<Paths>,
    pub config: Arc<AppConfig>,
    pub clock: Arc<dyn Clock + Send + Sync>,
    pub shutdown: CancellationToken,
    pub log_guard: Arc<WorkerGuard>,
    pub schema_version: u32,
}
```

**Изменение:** добавить поле `pub devices: Arc<DeviceService>` после `schema_version`. `DeviceService` должен либо имплементить `Clone` (cheap via `Arc<...>` fields внутри), либо мы держим его как `Arc<DeviceService>` чтобы `AppCtx: Clone` оставался дёшев.

**Конструкция в `AppCtx::build`** (после reader pool open, analog lines 124-138):
```rust
let writer = Arc::new(WriterHandle::spawn(writer_conn));
let readers = Arc::new(ReaderPool::new(&db_path, 4)?);
let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);

// NEW: device service composition.
let devices = Arc::new(DeviceService::new(
    writer.clone(),
    readers.clone(),
    clock.clone(),
));

Ok(Self {
    writer, readers,
    paths: Arc::new(paths),
    config: Arc::new(config),
    clock,
    shutdown: CancellationToken::new(),
    log_guard: Arc::new(log_guard),
    schema_version,
    devices,
})
```

---

### `crates/trackly-app/src/services/device_service.rs` (service, composition) — role-match

**Closest pattern carrier:** `crates/trackly-app/src/context.rs::AppCtx::build` (для использования writer/readers) + `crates/trackly-infra/src/db/writer_worker.rs::WriterHandle::execute` (для write closures) + `crates/trackly-infra/src/db/pools.rs` (для read paths).

**Write closure pattern** (writer_worker.rs lines 83-99 — сигнатура `execute`):
```rust
pub async fn execute<F, R>(&self, op: F) -> Result<R, AppError>
where
    F: FnOnce(&mut Connection) -> Result<R, AppError> + Send + 'static,
    R: Send + 'static,
```

**Применение в DeviceService::create** (см. RESEARCH §Pattern 2 — write closure + audit_log в той же транзакции):
```rust
pub async fn create(&self, new: DeviceNew) -> Result<DeviceDto, AppError> {
    let now = self.clock.unix_seconds();
    let repo = self.repo.clone();
    let user_id_opt: Option<i64> = None; // Phase 2 — no auth yet
    let id = self.writer.execute(move |conn| {
        let tx = conn.transaction().map_err(map_rusqlite)?;
        let id = repo.create(&tx, &new, now)?;
        let after = repo.get(&tx, id)?;
        let after_json = serde_json::to_string(&after).map_err(|e| AppError::Internal {
            source_chain: format!("audit_log after-json: {e}"),
        })?;
        tx.execute(
            "INSERT INTO audit_log (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
             VALUES ('device', ?1, 'create', ?2, NULL, ?3, NULL, ?4)",
            rusqlite::params![id, user_id_opt, after_json, now],
        ).map_err(map_rusqlite)?;
        tx.commit().map_err(map_rusqlite)?;
        Ok(id)
    }).await?;
    self.get(id).await
}
```

**Read path pattern** (pools.rs lines 56-67 — `acquire()` returns RAII guard, Deref to Connection):
```rust
pub async fn list(&self, filter: DeviceFilter, page: Pagination) -> Result<(Vec<DeviceDto>, u64), AppError> {
    let readers = self.readers.clone();
    let repo = self.repo.clone();
    tokio::task::spawn_blocking(move || {
        let guard = readers.acquire();      // RAII; Drop → returns to pool
        repo.list(&guard, &filter, &page)   // &guard derefs to &Connection
    })
    .await
    .map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking: {e}") })?
}
```

**Optimistic lock pattern** — see RESEARCH §Pattern 3; surface `AppError::OptimisticLockMismatch` (variant уже определён в `crates/trackly-core/src/error.rs` lines 52-62), не auto-retry.

---

### `crates/trackly-infra/src/repos/devices_sqlite.rs` (repository adapter, CRUD) — role-match

**Closest patterns:** `crates/trackly-infra/src/db/migrations.rs::run` (sync `fn(&mut Connection)`) + `crates/trackly-infra/src/db/pools.rs::ReaderHandle` (deref to `&Connection`).

**Function signature pattern:**
- Write methods: `fn create(&self, conn: &mut Connection, new: &DeviceNew, now_utc: i64) -> Result<i64, AppError>` — берут `&mut Connection` (или `&Transaction`) от вызывающего; не владеют коннекшном.
- Read methods: `fn list(&self, conn: &Connection, filter: &DeviceFilter, page: &Pagination) -> Result<(Vec<DeviceDto>, u64), AppError>`.

**Error mapping** (`crates/trackly-infra/src/error_conversions.rs` lines 31-47) — на каждый `rusqlite::Error` → `.map_err(map_rusqlite)`:
```rust
use crate::error_conversions::map_rusqlite;

let stmt = conn.prepare(SQL).map_err(map_rusqlite)?;
let rows = stmt.query_map(params, |row| { /* ... */ }).map_err(map_rusqlite)?;
```

`map_rusqlite` уже различает: `DatabaseBusy/Locked` → `WriteQueueBusy`, `ConstraintViolation` → `Conflict`, всё остальное → `Internal`. Это покрывает UNIQUE violations (имя устройства, etc.) автоматически.

**Column-name reconciliation (важно — несоответствие в существующей V003):**

V003 (`migrations/V003__devices.sql`) использует имена `inventory_number`, `serial_number`, `condition`, `complectation`, `notes`. CONTEXT.md (D-Schema-Phase2-01) и UI-SPEC говорят `inventory_no`, `serial_no`, `state`, `kit`, `specs`. **Решение для repo:** repo использует фактические имена колонок БД (`inventory_number`, `serial_number`, `condition`, `complectation`). DTO в `dto/device.rs` использует имена UI (`inventory_no`, `serial_no`, `state`, `kit`, `specs`), а repo делает mapping в `from_row`. Альтернативно — V013 добавляет столбцы; planner выбирает. **Recommendation:** в-памяти-mapping в repo (без миграции колонок) — дешевле, не ломает существующие fixture-данные V003.

---

### `crates/trackly-core/src/ports/devices.rs` (trait, pure port) — role-match

**Analog:** `crates/trackly-core/src/primitives/clock.rs` (trait в core, без I/O-deps).

**Pattern** (clock.rs lines 16-25):
```rust
pub trait Clock: Send + Sync {
    fn now(&self) -> OffsetDateTime;
    fn unix_seconds(&self) -> i64 {
        self.now().unix_timestamp()
    }
}
```

**Применение:** `DeviceRepository` trait без I/O-deps. Но: trait должен принимать `&rusqlite::Connection` — а `trackly-core` запрещено зависеть от `rusqlite` (см. `crates/trackly-core/tests/no_io_deps.rs`).

**Решение** (см. CONTEXT D-Repo-01): trait объявляет **только** domain типы (`DeviceNew`, `DevicePatch`, `DeviceFilter`, `DeviceRow`, `DeviceGroupRow`) и сигнатуры. Реальный rusqlite-conn передаётся в **impl** в `trackly-infra`, который добавляет в свою сигнатуру `&Connection`. Trait-объявление в core НЕ упоминает rusqlite. Два варианта:

1. **«Объявить trait в infra»** (проще): pure trait уехал в `trackly-infra/src/repos/devices_sqlite.rs` как inherent impl — нет trait-объявления в core вовсе; service в `trackly-app` напрямую держит `Arc<SqliteDeviceRepository>`. CONTEXT.md формально требует port в core, но если orphan-rule блокирует — этот fallback приемлем.
2. **«Generic trait с associated type»** (соблюсти hexagonal): объявить в core `trait DeviceRepository { type Conn; fn create(&self, conn: &mut Self::Conn, ...) -> Result<i64, AppError>; ... }`. Infra-impl задаёт `type Conn = rusqlite::Connection`. Это удовлетворяет core/no-rusqlite-dep тест.

Planner выбирает один из вариантов. Recommendation: вариант (1) — Phase 2 проще, тест-double для DeviceService в Phase 4+ можно сделать через `cfg_if!` или отдельный test-only trait wrapper.

---

### `crates/trackly-core/src/domain/devices.rs` (domain types) — role-match

**Analog:** `crates/trackly-core/src/primitives/secret.rs` (pure value types, без I/O).

**Pattern:**
- Types — `pub struct DeviceNew { ... }` с дженериковыми полями (`String`, `Option<String>`, `i64`).
- Никаких `serde::Serialize` derive (это для DTO в `trackly-app`).
- Никаких `specta::Type` derive (это тоже для DTO).
- domain типы могут иметь validation методы: `impl DeviceNew { pub fn validate(&self) -> Result<(), AppError> { ... } }`.

**Mapping core domain ↔ trackly-app DTO** — `From` impl'ы либо в `trackly-app/dto/device.rs`, либо в repo's `from_row`.

---

### `migrations/V013__devices_fts_triggers.sql` (migration, DDL) — **exact match**

**Analog:** `migrations/V012__indexes_and_fts.sql`

**File-layout pattern** (analog lines 1-58):
- Comment-блок описывает purpose + рамки.
- DDL statements.
- Последняя строка — `PRAGMA user_version = N;` (D-Migrations-02).

**FTS5 external-content trigger pattern** для devices_fts (создан в V012 с `content='devices', content_rowid='id'`) — см. RESEARCH §Pattern 4 для полного SQL. Ключевые моменты:

1. `AFTER INSERT` с `WHEN NEW.deleted_at_utc IS NULL` → `INSERT INTO devices_fts(rowid, name, inventory_number, serial_number, model)`.
2. `AFTER DELETE` → magic `INSERT INTO devices_fts(devices_fts, rowid, ...) VALUES ('delete', ...)`.
3. `AFTER UPDATE` → delete-then-conditional-insert, чтобы handling и content-change, и soft-delete transitions.

**Column names в trigger SQL** — `inventory_number`, `serial_number` (фактические колонки V003), не `inventory_no`/`serial_no`.

**Partial indexes** (analog V012 lines 26-28 для базовых indexes на devices):
```sql
CREATE INDEX idx_devices_autocomplete_name
  ON devices(name) WHERE deleted_at_utc IS NULL;
```
+ дополнительные `(name, model)`, `(name, location_id)`, `(name, condition)`, `(name, complectation)` для DEV-09 contextual.

**`max_known_version()`** автоматически подберёт V013 (см. `crates/trackly-infra/src/db/migrations.rs` lines 26-32, `runner().get_migrations()` читает embed_migrations!). Тесты, которые сейчас проверяют `schema_version == 12` (например `tests/health_smoke.rs:35`, `migrations.rs::tests::run_applies_all_twelve_migrations_on_fresh_db:93,99`, `tauri_cmds/health.rs:69`, `http/health.rs:53,82`), должны быть обновлены до 13 в плане Phase 2.

---

### `crates/trackly-app/src/specta_export.rs` (modification, +12 commands) — exact

**Analog:** сам файл (текущие lines 19-21).

**Текущее объявление:**
```rust
pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![crate::tauri_cmds::health::health])
}
```

**Расширение для Phase 2** (CONTEXT D-Bindings-01):
```rust
pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        crate::tauri_cmds::health::health,
        crate::tauri_cmds::devices::devices_list,
        crate::tauri_cmds::devices::devices_get,
        crate::tauri_cmds::devices::devices_create,
        crate::tauri_cmds::devices::devices_update,
        crate::tauri_cmds::devices::devices_delete,
        crate::tauri_cmds::devices::devices_search,
        crate::tauri_cmds::devices::devices_autocomplete,
        crate::tauri_cmds::devices::devices_state_hints,
        crate::tauri_cmds::devices::devices_import_csv_preview,
        crate::tauri_cmds::devices::devices_import_csv_commit,
        crate::tauri_cmds::devices::devices_export_csv,
        crate::tauri_cmds::devices::devices_list_grouped,
    ])
}
```

---

### `crates/trackly-app/tests/devices_*.rs` (integration tests) — role-match

**Closest analog:** `crates/trackly-app/tests/concurrent_writes.rs` (fixture usage) + `crates/trackly-app/tests/health_smoke.rs` (full AppCtx::build).

**Fixture pattern** (concurrent_writes.rs lines 6-19):
```rust
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ten_devices_create_and_list() {
    let (writer, readers, _guard) = test_writer_and_readers();
    // assemble a minimal DeviceService manually here (writer + readers + SystemClock),
    // or use a new test_support::test_device_service() helper (per D-Test-Phase2-01).
    // ...
}
```

**Full stack pattern** (health_smoke.rs lines 20-61) — для `tests/devices_http_smoke.rs`: полный `AppCtx::build` → Tauri-path (`build_devices_create`) + axum-path (`router().with_state(ctx).oneshot(...)`) → assert equality.

**CSV fixtures** (D-Test-Phase2-01) — реальные файлы в `crates/trackly-app/tests/fixtures/devices/`:
- `utf8.csv`
- `utf8_bom.csv` (3 байта BOM)
- `cp1251_comma.csv` (binary, точно CP1251-байты для `«Сидоров-Петроградский Иван Александрович (ё) №42»`)
- `cp1251_semicolon.csv`
- `malformed_mixed_rows.csv`

Fixture-строка — `«Сидоров-Петроградский Иван Александрович (ё) №42»` (наследуется из CONTEXT Phase 1; уже фигурирует в Phase 1 cyrillic-sandbox тестах).

**Hard 30s timeout** на каждый `#[tokio::test]` (см. health.rs:75-86, http/health.rs:61-87) — обязательно.

---

### `crates/trackly-app/tests/export_bindings.rs` (modification) — exact

**Текущие assert'ы** (lines 60-76):
```rust
assert!(contents.contains("HealthDto"), ...);
assert!(contents.contains("version"), ...);
assert!(contents.contains("schema_version") || contents.contains("schemaVersion"), ...);
assert!(contents.contains("AppError"), ...);
```

**Phase 2 расширение** (CONTEXT D-Bindings-01):
```rust
for substring in [
    "DeviceDto", "DeviceNew", "DevicePatch", "DeviceFilter",
    "CsvImportPreview", "CsvImportReport", "DeviceGroup",
] {
    assert!(contents.contains(substring), "bindings.ts missing {substring}");
}
```

**Skip-on-Windows** (line 20) **оставить** — не закрываем в Phase 2 (см. deferred-items.md).

---

### `crates/trackly-app/Cargo.toml` (modification) + workspace `Cargo.toml`

**Workspace Cargo.toml additions** (после line 45 в существующем `[workspace.dependencies]`):
```toml
chardetng = "0.1"
encoding_rs = "0.8"
csv = "1.3"
uuid = { version = "1", features = ["v4", "v7", "serde"] }
tauri-plugin-dialog = "2"
```

**crates/trackly-app/Cargo.toml additions** (`[dependencies]`, после line 39):
```toml
chardetng = { workspace = true }
encoding_rs = { workspace = true }
csv = { workspace = true }
uuid = { workspace = true }
tauri-plugin-dialog = { workspace = true }
```

**Pinned-versions discipline** (CONTEXT existing): `specta`/`tauri-specta`/`specta-typescript` остаются `=`-pinned (workspace lines 38, 41, 42). Не апгрейдить.

---

### `ui/index.html` (modification) — exact

**Текущее содержание** (lines 1-13):
```html
<!doctype html>
<html lang="ru">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Trackly</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

**Изменение:** добавить inline no-flash script в `<head>` ДО Vite-module script (см. UI-SPEC §Theme application; RESEARCH §Pattern 5). Точный snippet — RESEARCH lines 542-552 / UI-SPEC lines 547-559. **Critical:** inline `<script>` без `src` — Vite не трогает, шипится verbatim.

---

### `ui/package.json` (modification, D-Cleanup-01) — exact

**Текущее содержание** (lines 18-37) — dependencies только `svelte`.

**Изменение** (CONTEXT D-Cleanup-01 + RESEARCH §New JS deps):
```json
"dependencies": {
  "@tauri-apps/api": "^2.11.0",
  "@tauri-apps/plugin-dialog": "^2.7.1",
  "svelte": "^5.55.0",
  "svelte-spa-router": "^5.1.0"
}
```

**+ закрытие deferred item:** в `.github/workflows/ci-fast.yml` и `ci-full.yml` удалить `continue-on-error: true` со step `pnpm svelte-check`.

---

### `ui/src/lib/api/client.ts` (greenfield) — UI Pattern

**Полный листинг** в RESEARCH §Pattern 8 (lines 683-707) и UI-SPEC §Error rendering. Ключевые элементы:

```typescript
import { parseAppError } from './errors';

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export async function apiCall<R>(name: string, args: Record<string, unknown> = {}): Promise<R> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    try { return await invoke<R>(name, args); }
    catch (e) { throw parseAppError(e); }
  }
  // Phase 5+ HTTP path (никогда не выполняется в Tauri runtime в Phase 2)
  const res = await fetch(`/api/v1/${name}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(args),
  });
  if (!res.ok) throw parseAppError(await res.json().catch(() => ({})));
  return res.json();
}
```

**bindings.ts** (auto-generated) уже содержит `import { invoke as TAURI_INVOKE } from "@tauri-apps/api/core";` (lines 69-73) — паттерн совпадает.

---

### `ui/src/lib/stores/*.svelte.ts` (greenfield) — UI Pattern (Svelte 5 runes)

**Полный листинг** в RESEARCH §Pattern 9 (lines 716-755). Критические правила:

1. **Filename MUST end in `.svelte.ts`** — иначе rune-transform не применится.
2. `export const themeStore = $state({ ... })` — `const` корректен.
3. Не делать `export let x = $state(0)` — assignment к exported binding rejected.
4. Storage keys namespaced: `'trackly:theme'`, `'trackly:devices:grouped'` (UI-SPEC §Interaction Patterns).

**ToastStore minimum shape:**
```typescript
type ToastKind = 'success' | 'error' | 'info' | 'warning';
export const toastStore = $state({ items: [] as Array<{ id: string; kind: ToastKind; message: string }> });

export function pushToast(kind: ToastKind, message: string) { /* push + ttl-based auto-remove */ }
```

**TTLs** (UI-SPEC §Toast component): error 6000ms, warning 5000ms, success/info 4000ms.

---

### `ui/src/features/layout/Sidebar.svelte` (greenfield) — UI Pattern

**Полный pattern** — RESEARCH §Pattern 10 (lines 765-789). Двойной import:
```svelte
import { link } from 'svelte-spa-router';
import active from 'svelte-spa-router/active';
```

**Sidebar items source** — `sidebar-config.ts` array `SidebarItem | SidebarDivider` (10 items + 3 dividers строго по UI-SPEC §Sidebar / CONTEXT D-UI-Sidebar-01).

---

### `ui/src/styles/_tokens.scss` (REWRITE) — partial

**Текущий placeholder** (lines 1-3):
```scss
// Design tokens placeholder. Phase 2 fills in real palette, spacing, typography.
$placeholder: true;
```

**Phase 2 заполнение** — полные значения CSS variables перечислены в UI-SPEC §Color (lines 162-201), §Typography (lines 89-103), §Spacing Scale (lines 36-65).

**Контракт:** все токены — CSS custom properties в `:root` и `[data-theme="dark"]`. SCSS variables (`$placeholder` стиль) — НЕ использовать для tokens (UI должно переключать через `data-theme` атрибут).

**vite.config.ts** уже подключает `_tokens.scss` через `prependData` — `<style lang="scss">` блоки сразу видят токены.

---

## Shared Patterns

### Pattern 1: Single-helper-two-transport (build_* helper + Tauri command + axum handler)

**Source files:**
- `crates/trackly-app/src/tauri_cmds/health.rs` (lines 21-37): `build_health` helper + `#[tauri::command]`
- `crates/trackly-app/src/http/health.rs` (lines 14-22): axum handler делегирует тот же helper
- `crates/trackly-app/src/dto/health.rs`: DTO shared между обоими

**Apply to:** все 12 Phase 2 commands (devices_*).

**Invariant:** атрибут `#[specta::specta]` идёт **после** `#[tauri::command]` (требует tauri-specta v2 rc.21).

### Pattern 2: Error mapping discipline

**Source:** `crates/trackly-infra/src/error_conversions.rs` (free functions, lines 31-76)

**Apply to:** все callsite'ы repo + service. На каждый `.map_err(...)` — соответствующая `map_*` функция:
- `rusqlite::Error` → `.map_err(map_rusqlite)`
- `refinery::Error` → `.map_err(map_refinery)`
- `tokio::sync::mpsc::error::SendTimeoutError<T>` → автоматически через `WriterHandle::execute` (внутри)
- `tokio::sync::oneshot::error::RecvError` → аналогично

**`map_rusqlite` уже различает** UNIQUE/CHECK/FK violation → `Conflict`, busy/locked → `WriteQueueBusy`. Это покрывает большинство ошибок при `devices_create` (дубли inventory_no/serial_no).

### Pattern 3: Snake_case JSON discipline

**Source:** `crates/trackly-app/src/dto/health.rs` (lines 19, 47-58 test)

**Apply to:** все новые DTO — НЕ добавлять `#[serde(rename_all = "camelCase")]`. Bindings.ts экспортирует `inventory_no`, `db_ready` — frontend читает в snake_case.

### Pattern 4: hard 30s timeout на async-тесты

**Source:** `crates/trackly-app/src/tauri_cmds/health.rs::tests` (lines 75-86), `crates/trackly-app/src/http/health.rs::tests` (lines 61-87)

**Apply to:** каждый `#[tokio::test]` в Phase 2 integration tests. Защита от Linux-CI deadlock pattern.

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_name() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        // actual test body
    })
    .await
    .expect("<name> exceeded 30 s budget — Linux-CI deadlock pattern");
}
```

### Pattern 5: Single-writer + reader-pool через AppCtx

**Source:** `crates/trackly-app/src/context.rs::AppCtx` (lines 33-55), `crates/trackly-infra/src/db/writer_worker.rs` (lines 39-100), `crates/trackly-infra/src/db/pools.rs` (lines 23-73)

**Apply to:** `DeviceService` методы:
- writes → `self.writer.execute(move |conn| { ... }).await`
- reads → `tokio::task::spawn_blocking(move || { let g = self.readers.acquire(); /* ... */ })`

Frontend audit: НИ ОДИН repo-метод не должен открывать собственный `Connection::open(...)` — это нарушение Phase 1 success criterion #2.

### Pattern 6: Specta-export gate against bindings drift

**Source:** `crates/trackly-app/tests/export_bindings.rs`, `crates/trackly-app/src/specta_export.rs`

**Apply to:** каждая новая `#[tauri::command]` функция ОБЯЗАНА быть в `collect_commands![...]`. Code-review checklist. Существующий gate-тест (`export_bindings.rs`) расширяется substring-asserts (см. Pattern Assignment выше).

### Pattern 7: Schema-version bump propagates to all callsites

**Source files где `schema_version == 12` hardcoded:**
- `crates/trackly-app/src/tauri_cmds/health.rs:69` (test minimal_ctx)
- `crates/trackly-app/src/http/health.rs:53` (test minimal_ctx)
- `crates/trackly-app/src/http/health.rs:82` (test assert)
- `crates/trackly-app/tests/health_smoke.rs:34-35` (test assert)
- `crates/trackly-infra/src/db/migrations.rs:93,99,113` (tests)

**Apply to Phase 2:** bump каждое `12` → `13` в одном PR, не разрывать. `max_known_version()` (migrations.rs:25-33) автоматически возвращает 13 после добавления V013.

### Pattern 8: Cyrillic fixture string

**Source:** Phase 1 CONTEXT + cyrillic-sandbox tests (`tools/procmon-check/src/sandbox.rs`).

**Apply to:** все CSV/PDF фикстуры Phase 2 включают строку `«Сидоров-Петроградский Иван Александрович (ё) №42»` в обеих кодировках (UTF-8, CP1251 байты). Это и success-criterion-#1 регрессионный тест.

---

## No Analog Found

Phase 2 первая UI-фаза проекта. Следующие файлы НЕ имеют codebase-аналога; planner полагается на UI-SPEC + RESEARCH:

| Файл | Role | Reason | Fallback Source |
|------|------|--------|----------------|
| `ui/src/lib/components/*.svelte` (11 примитивов) | UI primitives | Phase 1 только placeholder `App.svelte` | UI-SPEC §Component Inventory (lines 386-540) |
| `ui/src/features/devices/*.svelte` (9 components) | feature UI | greenfield | UI-SPEC §Devices feature + REQUIREMENTS DEV-01..13 |
| `ui/src/lib/stores/*.svelte.ts` (3 stores) | rune-stores | greenfield | RESEARCH §Pattern 9 (lines 716-755) |
| `ui/src/lib/api/client.ts` | transport detect | greenfield (bindings.ts генерируется) | RESEARCH §Pattern 8 (lines 683-707) |
| `ui/src/features/layout/{Layout,Sidebar,sidebar-config}.{svelte,ts}` | layout | greenfield | UI-SPEC §Layout + RESEARCH §Pattern 10 |
| `ui/src/routes.ts` | router config | greenfield | RESEARCH §Pattern 10 |
| `ui/src/styles/global.scss` | global styles | greenfield | UI-SPEC (reset + focus-ring + reduced-motion) |
| `crates/trackly-app/src/csv/{sniff,decode,parse,session_store}.rs` | CSV pipeline | greenfield | RESEARCH §Pattern 6 (lines 561-617), §Pattern 7 (lines 631-672) |
| `crates/trackly-app/src/services/device_service.rs` | service layer | новый слой архитектуры | RESEARCH §Pattern 2 + §Pattern 3; CONTEXT D-Repo-01 |
| `crates/trackly-core/src/ports/devices.rs` | port trait | впервые появляются `ports/` | CONTEXT D-Repo-01 + RESEARCH §Architecture |
| `crates/trackly-core/src/domain/devices.rs` | domain types | впервые появляется `domain/` | CONTEXT D-Repo-01 |

---

## Metadata

**Analog search scope:**
- `crates/trackly-core/src/**` — все 6 файлов прочитаны (error.rs, lib.rs, primitives/{clock.rs, secret.rs, mod.rs}, plus tests)
- `crates/trackly-infra/src/**` — db/{writer_worker, pools, migrations, pragmas}.rs, error_conversions.rs, test_support/test_app_ctx.rs, lib.rs, clock_impl.rs
- `crates/trackly-app/src/**` — context.rs, lib.rs, specta_export.rs, error_axum.rs, tauri_cmds/{health,mod}.rs, http/{health,mod}.rs, dto/{health,mod}.rs
- `crates/trackly-app/tests/**` — health_smoke.rs, concurrent_writes.rs, export_bindings.rs
- `migrations/V001..V013` — V001, V002, V003, V008, V012 (полностью); V004-V007, V009-V011 (имена)
- `ui/**` — App.svelte, main.ts, bindings.ts, index.html, package.json, vite.config.ts, styles/_tokens.scss
- `Cargo.toml` (workspace), `crates/trackly-app/Cargo.toml`

**Files scanned:** 30+ Rust + 6 frontend + 7 SQL migrations + 3 manifests/configs

**Pattern extraction date:** 2026-05-26
