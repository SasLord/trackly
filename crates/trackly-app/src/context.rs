//! `AppCtx` — composition-root, который держит вместе writer worker, reader pool,
//! paths, config, clock, shutdown token и tracing WorkerGuard.
//!
//! Все Tauri commands и axum handlers в Phases 2-8 будут принимать `AppCtx`
//! (Clone-able через `Arc`), маршрутить чтения через `readers.acquire()`,
//! а записи — через `writer.execute(closure)`.
//!
//! Lifecycle `AppCtx::build` (Steps 6-10 из RESEARCH §Code Example 1):
//! 1. Resolve `db_path`: config override (если непустой), иначе `paths.db_path()`.
//! 2. **PROBE-READ** `user_version` через **`SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_URI`**
//!    connection, ЕСЛИ файл существует. Если `on_disk > max_known_version()`
//!    → `AppError::DatabaseFromNewerVersion` (файл побайтово не тронут — это
//!    locks ROADMAP success criterion #4).
//! 3. Open WRITER connection (создаст файл если нужно), apply writer pragmas,
//!    run миграции.
//! 4. Hand writer conn в `WriterHandle::spawn` (mpsc 256 + spawn_blocking worker).
//! 5. Open `ReaderPool::new(_, 8)`.
//! 6. Собрать `AppCtx`.

use std::path::PathBuf;
use std::sync::Arc;

use rusqlite::{Connection, OpenFlags};
use tokio_util::sync::CancellationToken;
use tracing_appender::non_blocking::WorkerGuard;

use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::{migrations, pools::ReaderPool, pragmas, writer_worker::WriterHandle};
use trackly_infra::{AppConfig, Paths};

use crate::pdf::PdfRenderer;
use crate::services::{ActService, CartridgeService, DeviceService, OrganizationService, TemplateService};

/// Composition-root. Cloneable; делится между Tauri commands и axum handlers.
#[derive(Clone)]
pub struct AppCtx {
    /// Single-writer handle. Все писать-запросы через `.execute(closure)`.
    pub writer: Arc<WriterHandle>,
    /// Read-only пул из 4 connections. `.acquire()` → RAII guard.
    pub readers: Arc<ReaderPool>,
    /// Portable-mode paths (exe_dir, db_path, logs_dir, …).
    pub paths: Arc<Paths>,
    /// Распарсенный `trackly.config.toml` (или дефолты).
    pub config: Arc<AppConfig>,
    /// Источник времени. В проде — `SystemClock`.
    pub clock: Arc<dyn Clock + Send + Sync>,
    /// Cooperative shutdown token. Phase 5+ слушает в axum-серверах и
    /// background tasks.
    pub shutdown: CancellationToken,
    /// Удерживает background writer-thread tracing-appender'а живым на
    /// всё время жизни AppCtx. Drop → flush + join thread.
    pub log_guard: Arc<WorkerGuard>,
    /// `PRAGMA user_version` после миграций. Совпадает с `max_known_version()`
    /// если миграции прошли успешно.
    pub schema_version: u32,
    /// Device service — CRUD, search, autocomplete, grouping, CSV import/export.
    /// Added in Phase 2 Plan 01 (D-AppCtx-Extension-01).
    pub devices: Arc<DeviceService>,
    /// Act service — handover create + list + counts + peek next number.
    /// Added in Phase 3 Plan 02 (D-AppCtx-Extension-03). Return lifecycle +
    /// undo land in plan 03; organization/templates/pdf services land in plan 04.
    pub acts: Arc<ActService>,
    /// Organization service — read org.json + logo path-traversal mitigation.
    /// Added in Phase 3 Plan 04.
    pub organization: Arc<OrganizationService>,
    /// Template service — seed defaults + get_active body for MiniJinja render.
    /// Added in Phase 3 Plan 04.
    pub templates: Arc<TemplateService>,
    /// PDF renderer — krilla 0.7 wrapper + embedded fonts + MiniJinja env.
    /// Added in Phase 3 Plan 04.
    pub pdf: Arc<PdfRenderer>,
    /// Cartridge service — lifecycle, models CRUD, low-stock, history.
    /// Added in Phase 4 Plan 03.
    pub cartridges: Arc<CartridgeService>,
}

impl AppCtx {
    /// Полный lifecycle: probe-read downgrade check → writer open → migrations
    /// → writer worker spawn → reader pool → assemble.
    ///
    /// **Probe-read pattern (W4):** перед открытием writer-connection (которая
    /// может легитимно мутировать файл через WAL-init) открываем
    /// `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_URI` connection и читаем
    /// `PRAGMA user_version`. Read-only open НЕ касается WAL и не мутирует
    /// main file — поэтому отказ из-за downgrade оставляет файл побайтово
    /// идентичным. Это и есть ROADMAP success criterion #4.
    ///
    /// **First-run case:** если DB файл ещё не существует, probe пропускаем
    /// (file-not-found ≠ downgrade), сразу идём в writer-open который
    /// создаст файл.
    pub async fn build(
        paths: Paths,
        config: AppConfig,
        log_guard: WorkerGuard,
    ) -> anyhow::Result<Self> {
        // Step 6a: resolve db_path (config override или paths default).
        let db_path: PathBuf = if !config.paths.db_path.is_empty() {
            PathBuf::from(&config.paths.db_path)
        } else {
            paths.db_path().to_path_buf()
        };

        // Step 6b: PROBE-READ user_version (read-only — НЕ мутирует файл).
        if db_path.exists() {
            let known = migrations::max_known_version();
            let probe = Connection::open_with_flags(
                &db_path,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("probe-read open: {e}"),
            })?;
            let on_disk_i64: i64 = probe
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .map_err(|e| AppError::Internal {
                    source_chain: format!("probe-read user_version: {e}"),
                })?;
            // Explicit drop — релизим read-only handle ДО открытия writer.
            drop(probe);
            let on_disk: u32 = u32::try_from(on_disk_i64).map_err(|_| AppError::Internal {
                source_chain: format!("user_version negative or too large: {on_disk_i64}"),
            })?;
            if on_disk > known {
                return Err(AppError::DatabaseFromNewerVersion {
                    binary: known,
                    file: on_disk,
                }
                .into());
            }
        }

        // Step 7: open WRITER conn (создаст файл если нужно; WAL-init легитимно
        // мутирует файл — приемлемо после прохождения downgrade-check выше).
        let mut writer_conn = Connection::open(&db_path).map_err(|e| AppError::Internal {
            source_chain: format!("writer open {}: {e}", db_path.display()),
        })?;
        pragmas::apply_writer_pragmas(&writer_conn)?;

        // Step 8: run migrations.
        let report = migrations::run(&mut writer_conn)?;
        let schema_version = report.schema_version;

        // Step 9: hand writer conn в spawn_blocking worker.
        let writer = Arc::new(WriterHandle::spawn(writer_conn));

        // Step 10: open reader pool. Size 8 (bumped from 4): a single page load
        // can fire several concurrent reads (e.g. CartridgesPage loadAll →
        // Promise.all([list, counts, lowStock]) + model_list); 4 was too tight.
        // acquire() now also queues-on-exhaust instead of panicking, so 8 is
        // headroom, not a hard ceiling.
        let readers = Arc::new(ReaderPool::new(&db_path, 8)?);

        // Step 11: build clock and Phase 2 services (D-AppCtx-Extension-01).
        let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
        let devices = Arc::new(DeviceService::new(
            writer.clone(),
            readers.clone(),
            clock.clone(),
        ));

        // Step 12: Phase 3 Plan 04 PDF pipeline services.
        let paths_arc = Arc::new(paths);
        let organization = Arc::new(OrganizationService::new(paths_arc.clone()));
        let templates = Arc::new(TemplateService::new(
            writer.clone(),
            readers.clone(),
            clock.clone(),
        ));
        let pdf = Arc::new(PdfRenderer::new());

        // Seed default templates on first run (idempotent).
        templates.seed_defaults_on_startup().await?;

        // ActService с подключённым PDF pipeline.
        let acts = Arc::new(
            ActService::new(writer.clone(), readers.clone(), clock.clone()).with_pdf_pipeline(
                templates.clone(),
                organization.clone(),
                pdf.clone(),
            ),
        );

        // Phase 4 Plan 03: cartridge service.
        let cartridges = Arc::new(CartridgeService::new(
            writer.clone(),
            readers.clone(),
            clock.clone(),
        ));

        Ok(Self {
            writer,
            readers,
            paths: paths_arc,
            config: Arc::new(config),
            clock,
            shutdown: CancellationToken::new(),
            log_guard: Arc::new(log_guard),
            schema_version,
            devices,
            acts,
            organization,
            templates,
            pdf,
            cartridges,
        })
    }
}
