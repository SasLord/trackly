# Phase 1: Фундамент - Context

**Gathered:** 2026-05-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Заложить фундамент проекта: workspace из 3 крейтов, портативный режим (с дисциплиной путей и WebView2), полная схема БД для всех v1-сущностей (devices/acts/cartridges/users/requests/templates + cross-cutting audit_log/counters/sessions/scheduled_tasks), refinery-миграции, single-writer pattern, общие cross-cutting типы (`Secret<T>`, `AppError`, `Clock`), `tauri-specta` pipeline, и CI с ProcMon-тестом. **UI не строим** — это делает Phase 2. На выходе должна быть запускаемая «пустая» Tauri-оболочка, которая создаёт БД, прогоняет миграции, открывает соединения и держит инфраструктуру для следующих фаз.

Пользователь указал: «делай как считаешь правильным следуя лучшим практикам». Все серые зоны разрешены best-practice решениями, перечисленными ниже. Каждое из них — отменяемое в Phase-плане (через `--power` или новый CONTEXT-апдейт), но downstream-агенты должны исходить из них.

</domain>

<decisions>
## Implementation Decisions

### D-Schema-01: Идентификаторы — INTEGER PRIMARY KEY AUTOINCREMENT
- Все таблицы используют `id INTEGER PRIMARY KEY AUTOINCREMENT` (rowid с гарантией монотонности и без переиспользования).
- **Человеко-видимые номера** (`act.number INTEGER`, `cartridge.code TEXT GENERATED AS '...'` или поле `code` + counter) — отдельные колонки, не PK.
- UUID v7 НЕ используем для PK в v1 — нет шаринга наружу, нет distributed-сценариев; INTEGER экономит место (важно для FTS и индексов) и упрощает joins.
- **Rationale:** SUMMARY.md упоминает UUID v7 как «recommended for primary keys» с оговоркой «keep human numbers as a separate column» — но для single-org single-process приложения это преждевременная сложность. Если в будущем понадобятся стабильные public-facing IDs (например, ссылки на акты), добавим `public_id TEXT UNIQUE` отдельной миграцией.

### D-Schema-02: Timestamps — INTEGER (unix seconds, UTC only)
- Все timestamp-колонки: `INTEGER NOT NULL` (или `NULL` для опциональных), хранят unix epoch seconds.
- Колонки именуются с суффиксом `_at_utc`: `created_at_utc`, `updated_at_utc`, `deleted_at_utc`.
- В Rust: `time::OffsetDateTime` или `i64` (unix), сериализация через `serde_with::TimestampSeconds`.
- Запрет `chrono::Local::now()` через clippy `disallowed-methods` (уже в success criteria #3).
- **Rationale:** SQLite не имеет native datetime — INTEGER компактнее TEXT ISO-8601 в индексах, быстрее сравнивается, без неоднозначности TZ. TZ-форматирование — только на UI-слое через chrono-tz (PITFALLS #15).

### D-Schema-03: Soft-delete scope — все user-mutable сущности; system-таблицы — hard delete
- **Soft-delete (`deleted_at_utc INTEGER NULL`):** `devices`, `acts`, `cartridges`, `cartridge_models`, `users`, `requests`, `document_templates`, `locations`.
- **Hard delete:** `audit_log` (ретенция отдельно), `counters`, `sessions`, `scheduled_tasks`, `device_types`, `device_statuses`, `cartridge_states`, `cartridge_statuses` (lookup-таблицы — никогда не удаляем, только добавляем через миграции).
- `deleted_at_utc IS NULL` означает «живая запись»; все SELECT в репозиториях по умолчанию фильтруют это через helper в трейте репозитория.
- **Rationale:** SUMMARY.md рекомендует Acts/Devices/Cartridges как минимум; распространяем на все user-mutable — soft-delete дёшев на схеме, но дорог как retrofit (придётся переписывать репозитории и индексы).

### D-Schema-04: Optimistic lock — `version INTEGER NOT NULL DEFAULT 1` на всех user-mutable
- На тех же сущностях, что и soft-delete (см. D-Schema-03).
- Инкремент через `UPDATE ... SET version = version + 1 WHERE id = ? AND version = ?`; 0 affected rows → `AppError::OptimisticLockMismatch` → 409 в HTTP, 409-эквивалент в Tauri-invoke.
- Запись в `audit_log` — после успешного UPDATE, в той же транзакции.
- **Rationale:** SUMMARY.md прямо: «without it the 20-concurrent-user LAN scenario produces silent overwrites».

### D-Schema-05: audit_log — полный before/after JSON, без отдельной ретенции в Phase 1
- Схема: `id INTEGER PK AUTOINCREMENT, entity_type TEXT NOT NULL, entity_id INTEGER NOT NULL, action TEXT NOT NULL ('create'|'update'|'delete'|'restore'|'custom:xxx'), user_id INTEGER NULL, before_json TEXT NULL, after_json TEXT NULL, payload_json TEXT NULL, created_at_utc INTEGER NOT NULL`.
- `before_json`/`after_json` — JSON-сериализация всей записи (не diff). Diff делается на чтении при отображении истории.
- Индексы: `(entity_type, entity_id, created_at_utc)` для «история сущности» и `(user_id, created_at_utc)` для аудита по пользователю.
- **Ретенция:** не настраивается в v1 — пишем всё, чистим только если потребуется (отдельная фаза, scheduled_tasks-job). FOUND-10 говорит о записи, не о ретенции.
- **Rationale:** Полный JSON позволяет точное «undo return» (ACT-06, ACT-10, CART-06) без накопления edge-кейсов на diff-алгоритмах. JSON весит больше, но 20-юзеровый LAN не упрётся в IO; пересмотрим если БД пойдёт за 1 ГБ.

### D-Workspace-01: Crate layout и binary name
```
trackly/
├── Cargo.toml                 # workspace root
├── crates/
│   ├── trackly-core/          # домен + traits (ports) + сервисы. БЕЗ tokio, БЕЗ rusqlite, БЕЗ serde-feature на DTO.
│   ├── trackly-infra/         # adapters: SqliteRepos, тестовые mocks, paths.rs, refinery embed_migrations!
│   └── trackly-app/            # bin "trackly": tauri + axum + AppCtx + tracing setup + tauri-specta export bin
├── ui/                        # Svelte 5 SPA (vanilla, не SvelteKit). Vite root.
├── migrations/                # refinery .sql files. embed_migrations!() в trackly-infra.
├── .github/workflows/         # CI
└── tools/
    └── procmon-check/         # Windows-only утилита для CI ProcMon-теста (см. D-CI-03)
```
- **Binary name:** `trackly` (bin target в `trackly-app`).
- **UI folder:** `ui/` в корне (не `frontend/`, не внутри `trackly-app/src-ui/`). Vite root = `ui/`, build output = `ui/dist/`. Tauri конфиг указывает `frontendDist = "../ui/dist"`.
- **Rationale:** 3-крейтная раскладка — обязательная (ARCHITECTURE.md). `ui/` в корне — короче, ясно отделено от Rust-кода, не конфликтует с Tauri 2 конвенцией (раньше всё лежало в `src/`, теперь `frontendDist` явно настраивается).

### D-Workspace-02: tauri-specta — generate в `cargo test`, gitignored, pnpm prebuild
- `trackly-app` экспортирует `#[tauri::command]`-функции и `#[derive(specta::Type)]`-DTO через `tauri_specta::collect_commands!` + `Builder::export`.
- Bindings генерируются в `ui/src/bindings.ts` через test: `cargo test --package trackly-app --test export_bindings`.
- `ui/src/bindings.ts` в `.gitignore`.
- `package.json` script: `"prebuild": "cargo test -p trackly-app --test export_bindings"` (vite build зависит от prebuild).
- CI gate: `cargo test` запускается до svelte-check — если DTO рассинхронизированы, svelte-check падает раньше vite build.
- **Smoke-тест из success criteria #5:** один DTO (например, `HealthDto { version: String, db_ready: bool }`) + один command (`#[tauri::command] fn health(...) -> HealthDto`) + один axum-handler `GET /api/v1/health`, оба возвращают идентичный JSON. Тест: десериализовать ответ обоих в один и тот же rust-тип.
- **Rationale:** SUMMARY.md Open Question #5 — выбран generated-in-cargo-test вариант (всегда in-sync с Rust-кодом, не засоряет git diff'ы при изменениях DTO).

### D-Migrations-01: split по доменам, refinery convention, seed — отдельной миграцией
- Структура файлов:
  ```
  migrations/
    V001__init_pragmas_and_lookups.sql      # PRAGMA + lookup tables (device_types, device_statuses, cartridge_states, cartridge_statuses) + seed
    V002__core_entities.sql                  # users, locations
    V003__devices.sql
    V004__acts.sql                            # с parent_act_id + sub_number
    V005__cartridges.sql                      # cartridge_models, cartridges (с counter)
    V006__requests.sql
    V007__document_templates.sql
    V008__audit_log.sql
    V009__counters.sql                        # generic numbering: act_number, cartridge_seq
    V010__sessions.sql                        # tower-sessions backend
    V011__scheduled_tasks.sql
    V012__indexes_and_fts.sql                 # FTS5 виртуальные таблицы + всё, что зависит от наличия всех таблиц
  ```
- **Forward-only.** Refinery convention `V{n}__{description}.sql`, embed_migrations!() в trackly-infra.
- **Seed** в V001 (лookup-таблицы создаются и заполняются вместе — это атомарно и идеально для рестарта).
- **Эволюция seeded справочников:** новые типы/статусы добавляются отдельными миграциями (`V013__add_monitor_device_type.sql`), идемпотентно через `INSERT OR IGNORE`.
- **device_types seed:** `Устройство` (id=1, default), `Принтер` (id=2). НЕТ `Расходник` (см. SUMMARY.md Resolved Decisions).
- **device_statuses seed:** `На складе`, `В работе`, `На ремонте`, `Списано`.
- **cartridge_states seed:** `Полный`, `Частичный`, `Пустой`.
- **cartridge_statuses seed:** `На складе`, `В работе`, `На заправке`, `Списано`.
- **Rationale:** Split по доменам = понятный history в git blame, легко ревьюить, легко искать «когда добавлен такой-то индекс». Single 001_initial.sql на 600 строк — нечитаемый. Refinery всё равно бежит в одной транзакции по умолчанию.

### D-Migrations-02: PRAGMA user_version + ProcMon discipline
- Каждая миграция оканчивается `PRAGMA user_version = N;` где N = номер миграции.
- На старте `trackly-app::main()` после открытия write-пула, до открытия read-пула:
  1. `PRAGMA user_version` (текущая в БД).
  2. Если выше, чем embedded last migration → `AppError::DatabaseFromNewerVersion` → graceful shutdown с понятным сообщением (success criteria #4).
  3. Если ниже → `refinery::Runner::run(&mut conn)`.
  4. Если equal → пропуск.
- **Тест восстановления (success criteria #4):** fixture с user_version=999, попытка открыть, ассерт на ошибку + ассерт что файл побайтово идентичен после неудачи (не повреждён).
- **Rationale:** Защищает портабельность от downgrade-сценария (юзер запустил новую версию, потом откатился на старую — БД не должна повредиться).

### D-WriterChannel-01: bounded mpsc, capacity 256, backpressure через timeout
- `tokio::sync::mpsc::channel::<WriteJob>(256)`.
- Writer-task: `tokio::task::spawn_blocking` с одним owned `rusqlite::Connection`, в loop `while let Some(job) = rx.blocking_recv()`.
- Job-payload: `enum WriteJob { ... }` + `oneshot::Sender<Result<R, AppError>>` для ответа.
- **Backpressure:** если канал полон, `tx.send_timeout(job, Duration::from_secs(5))`. По timeout → `AppError::WriteQueueBusy` (HTTP 503 / Tauri-invoke эквивалент). 5 сек на LAN с 20 юзерами достаточно для пиков; индикатор перегрузки появится в логах.
- Capacity 256: один акт-возврат с 50 устройствами = ~50 jobs; 5 одновременных таких операций = 250. Запас.
- **Не unbounded:** unbounded маскирует утечку памяти при back-pressure-инциденте (зависшая транзакция).
- **Rationale:** SUMMARY.md прямо требует single-writer; backpressure-policy не специфицирована — выбираем bounded + timeout как операционно-безопасный baseline. Если в нагрузочных тестах увидим частые WriteQueueBusy — повысим capacity или будем разносить crud на batched jobs.

### D-AppError-01: единый flat enum, идентичный JSON shape в Tauri и axum
- Один `AppError` в `trackly-core::error` (variants по доменам):
  ```
  enum AppError {
    NotFound { entity: &'static str, id: i64 },
    Conflict { reason: String },
    OptimisticLockMismatch { entity: &'static str, id: i64, expected: i64, actual: i64 },
    WriteQueueBusy,
    DatabaseFromNewerVersion { binary: u32, file: u32 },
    Validation { field: String, message: String },
    Unauthorized,
    Forbidden,
    Internal { source_chain: String }, // serde_json::to_string на anyhow::Error chain
  }
  ```
- `impl Serialize` — единый JSON shape:
  ```json
  { "code": "OPTIMISTIC_LOCK_MISMATCH", "message": "Ru-сообщение", "details": { "entity": "device", "id": 42 } }
  ```
- `impl IntoResponse for AppError` (axum): mapping code → HTTP status.
- В Tauri: `#[tauri::command]` возвращает `Result<T, AppError>`; tauri-specta генерирует совместимый TS-тип `Result<T, AppError>`.
- **Rationale:** PITFALLS #5 / SUMMARY.md «AppError defined once, Serialize for Tauri + IntoResponse for axum, identical JSON shape in both transports». Flat enum (а не nested) — проще для tauri-specta и для frontend-обработчиков ошибок.

### D-Config-01: `trackly.config.toml` (НЕ JSON), минимальный набор полей
- Имя файла: `trackly.config.toml`. Лежит рядом с .exe (или с БД, см. `paths.rs`).
- Содержимое v1 (всё опционально, дефолты в коде):
  ```toml
  [server]
  enabled = false
  host = "127.0.0.1"
  port = 8443
  cert_path = ""           # пусто = self-signed на первом запуске

  [paths]
  db_path = ""             # пусто = <exe_dir>/trackly.db

  [logging]
  level = "info"           # trace|debug|info|warn|error
  format = "compact"       # compact|json
  retention_days = 14

  [organization]
  timezone = "Europe/Moscow"  # для отображения; в БД всегда UTC
  ```
- **TOML, не JSON** — комментарии разрешены, нет trailing-comma-footguns, стандарт в Rust ecosystem. SUMMARY.md упоминала `.json` в casual mention — но это не залоченное решение.
- **Маркер портативности:** наличие `portable.txt` ИЛИ `trackly.config.toml` рядом с .exe — оба сигналят portable mode (ARCHITECTURE.md).
- **Парсинг:** `toml` crate (sync, маленький, без зависимостей от serde-derive runtime).
- **Rationale:** TOML — best practice для hand-edited Rust-проектов config'ов. JSON неудобен админу-человеку (нет комментариев).

### D-Logging-01: tracing + tracing-appender, daily rotation, compact human по умолчанию
- Subscriber: `tracing_subscriber::Registry::default().with(EnvFilter).with(fmt_layer).with(file_layer)`.
- Stdout layer: compact format с цветами для dev.
- File layer: `tracing_appender::rolling::daily("<exe_dir>/logs", "trackly.log")`, non-blocking, формат из `[logging.format]` (compact | json).
- `WorkerGuard` хранится на `AppCtx`, drop'ается при graceful shutdown.
- **Retention:** background task в scheduled_tasks-supervisor (Phase 1 закладывает таблицу + минимальную инфру; реальная задача удаления старых логов — Phase 7). На Phase 1 — просто daily rotation без чистки.
- **Default level:** `info` для самого приложения, `warn` для зависимостей (через EnvFilter directive `info,hyper=warn,tower_http=warn`). Переопределяется через `TRACKLY_LOG` env var (стандарт RUST_LOG-like).
- **Rationale:** Стандартная конфигурация tracing-стека для production Rust сервиса; format-toggle через config даёт админу возможность переключиться на JSON для агрегации.

### D-CI-01: GitHub Actions matrix — fast checks на каждый push, full matrix на каждый PR + main
- **Workflow `ci-fast.yml`** (на каждый push в любую ветку, на PR):
  - ubuntu-latest: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --no-fail-fast`, `cd ui && pnpm install && pnpm svelte-check && pnpm lint`.
  - Кэш через `Swatinem/rust-cache@v2` и `actions/setup-node@v4` с pnpm cache.
- **Workflow `ci-full.yml`** (только на PR + push в main):
  - matrix: `[ubuntu-latest, macos-latest, windows-latest]`.
  - Steps: те же что fast + `cargo build --release -p trackly-app` (sanity-check сборки).
  - Windows runner дополнительно: ProcMon-тест (см. D-CI-03).
- **`cargo-deny`** — отдельный nightly workflow (`schedule: cron '0 6 * * *'`).
- **Rationale:** Solo dev + GitHub Free может упереться в минуты на private repo. Разделение fast/full даёт быстрый feedback на feature-ветке и тщательный gate на PR. ProcMon-тест дорогой (запуск VM) — поэтому только на PR.

### D-CI-02: clippy.toml — disallowed-methods list
```toml
# clippy.toml в корне workspace
disallowed-methods = [
  { path = "dirs::data_dir", reason = "portable mode: use trackly_infra::paths" },
  { path = "dirs::data_local_dir", reason = "portable mode: use trackly_infra::paths" },
  { path = "dirs::config_dir", reason = "portable mode: use trackly_infra::paths" },
  { path = "dirs::cache_dir", reason = "portable mode: use trackly_infra::paths" },
  { path = "dirs::home_dir", reason = "portable mode: use trackly_infra::paths" },
  { path = "tauri::Manager::path", reason = "use trackly_infra::paths, not tauri's path resolver" },
  { path = "chrono::Local::now", reason = "UTC only; use time::OffsetDateTime::now_utc" },
  { path = "chrono::offset::Local::now", reason = "UTC only" },
  { path = "std::fs::copy", reason = "for DB backup use rusqlite::backup::Backup; otherwise OK in tests" },
]
disallowed-types = [
  { path = "chrono::DateTime<chrono::Local>", reason = "UTC only" },
]
```
- `.to_str().unwrap()` на путях — нельзя бан через disallowed-methods (это generic метод OsStr/Path); вместо этого делаем clippy `path-with-unwrap`-стиль через custom dylint позже или ручной review-пункт в gsd-code-review.
- **Rationale:** Полностью покрывает success criteria #3 запреты. `std::fs::copy` бан мягкий (можно в тестах) — на самом деле bool через `clippy.toml` так не выставляется, поэтому будет в архитектурном README + ручной review.

### D-CI-03: ProcMon-тест — Windows-only, headless, через `procmon-check` утилиту
- В `tools/procmon-check/` — небольшая Rust-программа, которая:
  1. Создаёт временный sandbox-каталог `%TEMP%\trackly_procmon_<uuid>\`, копирует туда release-сборку `trackly.exe`.
  2. Запускает ProcMon с фильтром на `Process Name = trackly.exe` и `Operation = WriteFile|CreateFile (write access)`, output в CSV.
  3. Запускает `trackly.exe --self-test` (специальный режим: создать БД, прогнать миграции, открыть/закрыть, выйти).
  4. Останавливает ProcMon, парсит CSV.
  5. Assert: все WriteFile-paths начинаются с `%TEMP%\trackly_procmon_<uuid>\` ИЛИ `%TEMP%\<windows tmp paths>` (ProcessHacker'у разрешено).
  6. Запрещённые префиксы: `%APPDATA%`, `%LOCALAPPDATA%`, `~\AppData\` — fail-fast.
- В CI: `Action: download ProcMon` (Sysinternals), `Action: run procmon-check`.
- **Test fixture:** sandbox path содержит кириллицу: `%TEMP%\Документы\Trackly\` — покрывает success criteria #1 (cyrillic path) одновременно.
- **Rationale:** FOUND-11 / BLD-06 / success criteria #1 — критичны для core constraint «portable». Без ProcMon-теста любая регрессия (новая зависимость, обновление WebView2 SDK) тихо ломает портативность. Один из главных deliverable'ов фазы.

### D-Test-01: тестовая БД — tempfile per test (НЕ `:memory:`)
- Хелпер `test_db()` в `trackly-infra::test_support`: создаёт `tempfile::NamedTempFile` (auto-cleanup), открывает `rusqlite::Connection`, применяет refinery-миграции, возвращает `(conn, _guard)`.
- Для интеграционных тестов с writer-каналом — `test_app_ctx()` создаёт полный AppCtx с tempfile-БД и in-process mpsc.
- **Не `:memory:`:** `:memory:` НЕ моделирует WAL-поведение (нет .db-wal/.db-shm файлов), а мы хотим тестировать именно WAL-инварианты (single-writer, busy_timeout). Tempfile с auto-delete — стандарт для SQLite-тестов с WAL.
- **Concurrent-тест (success criteria #2):** integration test в `trackly-app/tests/concurrent_writes.rs`: spawn'ит 25 task'ов (имитация Tauri-invoke pat) + 25 task'ов (имитация axum handler pattern); каждый шлёт WriteJob через writer-channel. Все 50 успешно завершаются, нет «database is locked», нет таймаутов канала.
- **Rationale:** Тестируем WAL — тестируем на реальном WAL. `:memory:` обманет нас.

### Claude's Discretion
Пользователь сказал «делай как считаешь правильным» — следующие места имеют best-practice defaults, но downstream-агенты могут их пересмотреть, если найдётся конкретная причина:
- Точные имена fields в DTO (`HealthDto` smoke-тест) — на усмотрение планировщика.
- Структура `paths.rs` API (например, `Paths::db()`, `Paths::config()`, `Paths::logs_dir()`) — общая идея ясна, детали — у планировщика.
- Конкретный wire-format `AppError.details` — общий shape залочен, поля под domain — на усмотрение.
- Имена тестовых файлов и модулей.
- Конкретные индексы в `V012__indexes_and_fts.sql` (помимо очевидных PK/FK/UNIQUE) — планировщик решает по плану запросов из Phase 2+.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-Level (must-read)
- `CLAUDE.md` — стек, constraints, что НЕ использовать, version compatibility таблица.
- `.planning/PROJECT.md` — vision, core value, key decisions, существующие болевые точки.
- `.planning/REQUIREMENTS.md` — все 120 v1-требований с traceability; Phase 1 покрывает FOUND-01..12 + BLD-01 + BLD-06.
- `.planning/ROADMAP.md` §«Phase 1: Фундамент» — goal + 5 success criteria.

### Research (Phase 1 specific)
- `.planning/research/SUMMARY.md` — **главный документ.** Resolved Decisions (rusqlite vs sqlx, krilla vs Typst, Расходник question), Recommended Phase 1 Schema Must-Haves checklist (12 пунктов схемы + per-record invariants + pool/pragma discipline + cross-cutting Rust invariants), Phase 1 implications.
- `.planning/research/STACK.md` — pinned versions, supporting libraries, alternatives considered, stack patterns by variant (portable mode, server mode, win7).
- `.planning/research/ARCHITECTURE.md` — hexagonal core layout, dual-transport pattern, split read/write pools, single-writer task, tauri-specta integration, PRAGMA discipline, paths.rs design, AppCtx.
- `.planning/research/PITFALLS.md` — top 15 pitfalls с prevention; для Phase 1 особо важны #1 (portable leak), #2 (SQLite locked), #5 (auth gap), #6 (Cyrillic paths), #15 (TZ).
- `.planning/research/FEATURES.md` — gaps in PROJECT.md, рекомендованные дополнения к v1-схеме (audit_log, soft delete, optimistic lock, denormalized assigned_to, tables-not-enums).

### External (для research-агента, если копать глубже)
- Tauri 2 docs: https://v2.tauri.app/ — особо `WEBVIEW2_USER_DATA_FOLDER`, plugin v2, capability model.
- Refinery README: https://github.com/rust-db/refinery — `embed_migrations!`, naming convention, transaction behavior.
- rusqlite docs: https://docs.rs/rusqlite/0.39/ — connection options, PRAGMA usage, `backup` API.
- SQLite WAL: https://sqlite.org/wal.html — обоснование single-writer.
- Evan Schwartz, «Write Transactions are a Footgun with SQLx and SQLite»: https://emschwartz.me/psa-write-transactions-are-a-footgun-with-sqlx-and-sqlite/ — обоснование rusqlite vs sqlx.
- tauri-specta v2: https://github.com/specta-rs/tauri-specta — bindings generation pipeline.
- Sysinternals ProcMon CLI: https://learn.microsoft.com/en-us/sysinternals/downloads/procmon — для D-CI-03.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
**Нет.** Кодовой базы пока нет — greenfield. В репозитории только `CLAUDE.md` и `.planning/`.

### Established Patterns
**Нет** (greenfield). Patterns, которые мы устанавливаем В этой фазе, становятся обязательными для всех последующих:
- Hexagonal: `trackly-core` без I/O, `trackly-infra` — adapters, `trackly-app` — composition root.
- Single-writer mpsc + spawn_blocking — единственный путь записи в БД.
- `AppCtx` — single source of truth для всех сервисов и репозиториев; шарится между Tauri и axum.
- Все timestamps — UTC unix seconds, форматирование только на UI.
- `Secret<T>` для любых credentials.
- Все пути — через `trackly_infra::paths::Paths`, никогда через `dirs::*` или tauri path resolver.

### Integration Points
**Greenfield.** Phase 1 создаёт ВСЕ integration points для будущих фаз:
- `AppCtx` — handle, который получит каждый Tauri command и каждый axum handler начиная с Phase 2.
- `WriteJob`-enum + writer-channel — все будущие write-операции пойдут через него.
- DTO-крейт (часть `trackly-core`) + tauri-specta export — каждый новый DTO добавляется сюда.
- `AppError` — каждая фаза добавляет свои варианты (расширение enum) или маппит в существующие.
- `migrations/V0XX_*.sql` — каждая schema-эволюция = новый файл, никогда не правим существующие.

</code_context>

<specifics>
## Specific Ideas

- **«Сидоров-Петроградский Иван Александрович (ё) №42»** — фикстурная строка из SUMMARY.md PITFALLS #4. Используется в Phase 3 (PDF hash test) и в Phase 1 для cyrillic-path-теста (CI ProcMon).
- **Cyrillic install path для тестов:** `%TEMP%\Документы\Учёт\Trackly\` — модельный реальный кейс из success criteria #1.
- **`trackly --self-test` flag** — специальный режим запуска для CI (см. D-CI-03): прогнать full lifecycle (create DB → migrations → open pools → close → exit) без открытия UI и без запуска сервера. Удобно для smoke-тестов и для будущих deployment-проверок.
- **Smoke DTO для tauri-specta:** `HealthDto { version: String, db_ready: bool, schema_version: u32 }` (на ум) — тривиально, покрывает success criteria #5 без премutaры реального бизнес-DTO.

</specifics>

<deferred>
## Deferred Ideas

- **Корзина UI поверх soft-delete** — Phase 7 (Настройки) или отдельная фаза по UX-уборке. Схема готова с Phase 1.
- **Custom fields на устройствах** (`device_custom_fields(device_id, key, value)`) — SUMMARY.md Open Question #2: «live with freeform в v1; revisit only if users complain». НЕ добавляем в Phase 1.
- **Логотип BLOB в БД** (SET-02) — Phase 7. Phase 1 закладывает `document_templates` версионированную таблицу, BLOB-логотип — separate row в settings/organization-таблице, добавляется в Phase 7-миграции.
- **Backup retention policy + scheduled_tasks worker** — Phase 7. Phase 1 создаёт `scheduled_tasks` таблицу, но supervisor запускается с Phase 7.
- **Cleanup audit_log retention** — отложено, schema поддерживает любую ретенцию.
- **`activeCodePage=UTF-8` Windows manifest** — относится к Phase 8 (release pipeline) когда настраиваем NSIS-bundling. Но если в Phase 1 встанет в CI Windows-тест с cyrillic-путями — поднимаем раньше.
- **mDNS `.local` hostname для HTTPS в server mode** — SUMMARY.md Open Question #3 — Phase 5.
- **`tauri-plugin-single-instance`** — нужен в release (предотвратить две инстанции на одной БД), но УЖЕ в Phase 1 имеет смысл подключить чтобы dev-окружение не давало конкурирующих процессов. Если планировщик решит включить — best practice, согласовано.

</deferred>

---

*Phase: 1-Фундамент*
*Context gathered: 2026-05-24*
