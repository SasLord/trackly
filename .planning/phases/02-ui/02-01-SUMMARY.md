---
phase: 02-ui
plan: "01"
subsystem: backend-scaffold
tags:
  - foundation
  - scaffolding
  - migration
  - schema-version

dependency_graph:
  requires:
    - 01-04 (AppCtx, WriterHandle, ReaderPool)
    - 01-05 (HealthDto pattern, build_health pattern, specta_export)
    - 01-03 (refinery migrations, embed_migrations!)
  provides:
    - V013 migration (FTS sync triggers + 5 partial autocomplete indexes)
    - DeviceRepository trait (trackly-core::ports::devices)
    - DeviceNew/Patch/Filter/Row/GroupRow domain types (trackly-core::domain::devices)
    - SqliteDeviceRepository stub (trackly-infra::repos::devices_sqlite)
    - DeviceService scaffold + constructor (trackly-app::services::device_service)
    - ImportSessionStore full impl (trackly-app::csv::session_store)
    - STATE_HINTS const (6 RU strings) (trackly-app::dto::device)
    - AppCtx.devices: Arc<DeviceService> field
  affects:
    - All Phase 2 plans (02-02..02-05) build on these scaffolds
    - All schema_version callsites updated from 12 to 13

tech_stack:
  added:
    - chardetng = "0.1" (workspace dep, used in Plan 05 CSV import)
    - encoding_rs = "0.8" (workspace dep, used in Plan 05 CSV import)
    - csv = "1.3" (workspace dep, used in Plan 05 CSV import/export)
    - uuid = { version = "1", features = ["v4", "serde"] } (workspace dep, used in ImportSessionStore)
    - tauri-plugin-dialog = "2" (workspace dep, used in Plan 05 file picker)
    - tauri-plugin-single-instance = "2" (moved to workspace from direct dep in trackly-app)
  patterns:
    - Hexagonal port-adapter: DeviceRepository trait in core, SqliteDeviceRepository in infra
    - Path B column mapping: SQL uses V003 names (inventory_number, condition), domain uses UI names (inventory_no, state)
    - ImportSessionStore lazy TTL sweep on put() — no background task needed

key_files:
  created:
    - migrations/V013__devices_fts_triggers.sql
    - crates/trackly-core/src/domain/mod.rs
    - crates/trackly-core/src/domain/devices.rs
    - crates/trackly-core/src/ports/mod.rs
    - crates/trackly-core/src/ports/devices.rs
    - crates/trackly-infra/src/repos/mod.rs
    - crates/trackly-infra/src/repos/devices_sqlite.rs
    - crates/trackly-app/src/services/mod.rs
    - crates/trackly-app/src/services/device_service.rs
    - crates/trackly-app/src/csv/mod.rs
    - crates/trackly-app/src/csv/sniff.rs
    - crates/trackly-app/src/csv/decode.rs
    - crates/trackly-app/src/csv/parse.rs
    - crates/trackly-app/src/csv/session_store.rs
    - crates/trackly-app/src/dto/device.rs
    - crates/trackly-app/src/tauri_cmds/devices.rs
    - crates/trackly-app/src/http/devices.rs
  modified:
    - Cargo.toml (workspace deps: +6 new entries)
    - crates/trackly-app/Cargo.toml (+6 workspace deps)
    - crates/trackly-core/src/lib.rs (added pub mod domain, pub mod ports)
    - crates/trackly-infra/src/lib.rs (added pub mod repos)
    - crates/trackly-infra/src/db/migrations.rs (tests 12→13)
    - crates/trackly-infra/src/test_support/test_db.rs (assert 12→13)
    - crates/trackly-infra/tests/migration_idempotency.rs (asserts 12→13)
    - crates/trackly-app/src/lib.rs (added pub mod csv, pub mod services)
    - crates/trackly-app/src/dto/mod.rs (added pub mod device)
    - crates/trackly-app/src/tauri_cmds/mod.rs (added pub mod devices)
    - crates/trackly-app/src/http/mod.rs (added pub mod devices)
    - crates/trackly-app/src/context.rs (added devices field + DeviceService construction)
    - crates/trackly-app/src/tauri_cmds/health.rs (schema_version 12→13 + devices field in test ctx)
    - crates/trackly-app/src/http/health.rs (schema_version 12→13 + devices field in test ctx)
    - crates/trackly-app/tests/health_smoke.rs (schema_version 12→13)
    - crates/trackly-app/tests/downgrade_protection.rs (binary version 12→13)
    - crates/trackly-app/tests/specta_roundtrip.rs (schema_version 12→13 + devices field)

decisions:
  - "Path B column mapping chosen: domain types use UI names (inventory_no, state, kit, specs); SQL columns remain as V003 (inventory_number, condition, complectation, notes); DTO layer maps between them"
  - "DeviceRepository trait uses associated type Conn to keep rusqlite out of trackly-core (option-2 per PATTERNS.md)"
  - "ImportSessionStore lazy sweep on put() only — no background task; acceptable for LAN-scale use"
  - "DeviceService fields are pub(crate) with #[allow(dead_code)] for Plan 01 scaffold; methods land in Plans 03-05"
  - "V013 FTS triggers use AFTER INSERT/UPDATE/DELETE (not content-table rebuild) for incremental sync"

metrics:
  duration: "~20 min"
  completed: "2026-05-26"
  tasks: 2
  files_changed: 29
---

# Phase 02 Plan 01: Backend Scaffold + V013 Migration Summary

**One-liner:** Backend scaffold для Phase 2 — workspace deps, V013 FTS-triggers + autocomplete indexes, hexagonal модули ports/domain/repos/services/csv, AppCtx.devices, schema_version 12→13.

## What Was Built

### Task 1: Workspace deps + V013 migration + schema_version bump

**Workspace dependencies added** (`Cargo.toml [workspace.dependencies]`):
- `chardetng = "0.1"` — детект кодировки CP1251/UTF-8 для CSV
- `encoding_rs = "0.8"` — декодирование байтов в строку
- `csv = "1.3"` — CSV парсер
- `uuid = { version = "1", features = ["v4", "serde"] }` — токены ImportSessionStore
- `tauri-plugin-dialog = "2"` — нативный file-picker для CSV
- `tauri-plugin-single-instance = "2"` — перенесён из прямой зависимости в workspace

**V013 SQL** (`migrations/V013__devices_fts_triggers.sql`):
- 3 AFTER-триггера на таблицу `devices` для синхронизации `devices_fts` (INSERT/DELETE/UPDATE)
- 5 partial indexes `WHERE deleted_at_utc IS NULL` для autocomplete DISTINCT-запросов
- Используются V003 column names (`inventory_number`, `serial_number`, `condition`, `complectation`) — Path B
- Финальная строка: `PRAGMA user_version = 13;`

**Schema-version bump 12→13** во всех callsites:
- `crates/trackly-infra/src/db/migrations.rs` — тесты
- `crates/trackly-infra/src/test_support/test_db.rs`
- `crates/trackly-infra/tests/migration_idempotency.rs`
- `crates/trackly-app/src/tauri_cmds/health.rs`
- `crates/trackly-app/src/http/health.rs`
- `crates/trackly-app/tests/health_smoke.rs`
- `crates/trackly-app/tests/downgrade_protection.rs`
- `crates/trackly-app/tests/specta_roundtrip.rs`

### Task 2: Scaffold modules + AppCtx extension

**trackly-core новые модули:**
- `domain/devices.rs` — `DeviceNew`, `DevicePatch`, `DeviceFilter`, `Pagination`, `DeviceRow`, `DeviceGroupRow` (только `#[derive(Debug, Clone, PartialEq, Eq)]`, без serde)
- `ports/devices.rs` — `DeviceRepository` trait с 8 методами, `type Conn` для изоляции rusqlite

**trackly-infra новые модули:**
- `repos/devices_sqlite.rs` — `SqliteDeviceRepository` (Default + Clone), полный `impl DeviceRepository` с `todo!()` телами

**trackly-app новые модули:**
- `services/device_service.rs` — `DeviceService { writer, readers, clock, repo, csv_sessions }` + `#[allow(dead_code)]` + `new()` конструктор
- `csv/session_store.rs` — **полная реализация** `ImportSessionStore` с TTL 5 мин, lazy sweep
- `csv/{sniff,decode,parse}.rs` — заглушки с `#[allow(dead_code)]` + TODO Plan 05
- `dto/device.rs` — `STATE_HINTS: &[&str]` с 6 русскими строками (per DEV-10)
- `tauri_cmds/devices.rs`, `http/devices.rs` — module scaffolds

**AppCtx расширен** (`context.rs`):
- Поле `pub devices: Arc<DeviceService>`
- `AppCtx::build` конструирует `DeviceService::new(writer, readers, clock)` после reader pool

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Hardcoded schema_version = 12 в дополнительных тестовых файлах**
- **Found during:** Task 2 — `cargo test --workspace` упал на 3 теста
- **Issue:** `migration_idempotency.rs`, `test_db.rs`, `downgrade_protection.rs` не были в списке планируемых callsites; все ассертировали старое значение 12
- **Fix:** Обновил все три файла с `12` → `13`
- **Files modified:** `crates/trackly-infra/tests/migration_idempotency.rs`, `crates/trackly-infra/src/test_support/test_db.rs`, `crates/trackly-app/tests/downgrade_protection.rs`, `crates/trackly-app/tests/specta_roundtrip.rs`
- **Commit:** ecf03b4

**2. [Rule 2 - Missing] #[allow(dead_code)] на DeviceService struct**
- **Found during:** Task 2 — `cargo clippy -- -D warnings` падал на unused fields
- **Issue:** Scaffold-поля `writer`, `readers`, `clock`, `repo`, `csv_sessions` правомерно помечены как dead_code для Plan 01
- **Fix:** Добавил `#[allow(dead_code)]` на struct-уровне с документирующим комментарием
- **Files modified:** `crates/trackly-app/src/services/device_service.rs`

## Known Stubs

| Stub | File | Reason |
|------|------|--------|
| `csv/sniff.rs` — пустой модуль | `crates/trackly-app/src/csv/sniff.rs` | Реализация в Plan 05 (CSV import/export) |
| `csv/decode.rs` — пустой модуль | `crates/trackly-app/src/csv/decode.rs` | Реализация в Plan 05 |
| `csv/parse.rs` — пустой модуль | `crates/trackly-app/src/csv/parse.rs` | Реализация в Plan 05 |
| `SqliteDeviceRepository` методы — `todo!()` | `crates/trackly-infra/src/repos/devices_sqlite.rs` | CRUD в Plan 03, search/autocomplete в Plan 04 |
| `tauri_cmds/devices.rs` — пустой | `crates/trackly-app/src/tauri_cmds/devices.rs` | Команды в Plans 03-05 |
| `http/devices.rs` — пустой | `crates/trackly-app/src/http/devices.rs` | Routes в Plans 03-05 |

Все stubs намеренны и задокументированы — они НЕ препятствуют цели Plan 01 (scaffold + миграция).

## Verification Results

- `cargo build --workspace` — PASS
- `cargo clippy --workspace --all-targets -- -D warnings` — PASS (чист)
- `cargo test --workspace --no-fail-fast` — PASS (все тесты зелёные)
- `cargo test -p trackly-core --test no_io_deps` — PASS (core без I/O deps)
- `cargo test -p trackly-infra --lib db::migrations` — PASS (3 теста, schema_version = 13)
- `cargo test -p trackly-app --test health_smoke` — PASS
- `grep -c "PRAGMA user_version = 13" migrations/V013__devices_fts_triggers.sql` — 1

## Self-Check: PASSED

All created files verified to exist:
- `migrations/V013__devices_fts_triggers.sql` — EXISTS
- `crates/trackly-core/src/domain/devices.rs` — EXISTS
- `crates/trackly-core/src/ports/devices.rs` — EXISTS
- `crates/trackly-infra/src/repos/devices_sqlite.rs` — EXISTS
- `crates/trackly-app/src/services/device_service.rs` — EXISTS
- `crates/trackly-app/src/csv/session_store.rs` — EXISTS
- `crates/trackly-app/src/dto/device.rs` — EXISTS

Commits verified:
- `4523368` — feat(02-01): declare workspace deps + V013 migration + schema_version bump 12→13
- `ecf03b4` — feat(02-01): scaffold ports/domain/repos/services/csv modules + AppCtx.devices
