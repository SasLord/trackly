//! `Supervisor` — фоновый планировщик задач для Phase 7.
//!
//! `run_supervisor(ctx)` — долгоживущая tokio-задача, которая:
//! 1. Каждую минуту проверяет `scheduled_tasks` на просроченные задачи.
//! 2. Атомарно захватывает задачу (UPDATE WHERE status != 'running').
//! 3. Запускает логику задачи в `spawn_blocking`.
//! 4. Обновляет статус и `next_run_at_utc`.
//! 5. Завершается по `ctx.shutdown.cancelled()`.
//!
//! Catch-up семантика (D-17): задачи, просроченные при запуске, выполняются
//! немедленно (next_run_at_utc <= now при первом тике).
//!
//! Атомарное требование (T-07-02-05): `UPDATE WHERE status != 'running'` →
//! если `rows_affected == 0`, другой воркер уже захватил задачу — пропускаем.

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use tokio::select;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

use crate::context::AppCtx;
use crate::services::backup_service::BackupService;

/// Длительность тика планировщика.
const TICK_INTERVAL: Duration = Duration::from_secs(60);

/// 24 часа в секундах — интервал для daily задач.
const DAILY_SECS: i64 = 86_400;
/// Неделя в секундах — интервал для weekly задач.
const WEEKLY_SECS: i64 = 604_800;
/// 30 дней в секундах — возраст лог-файлов для удаления.
const LOG_RETENTION_SECS: u64 = 30 * 24 * 3600;

pub async fn run_supervisor(ctx: AppCtx) {
    let mut interval = tokio::time::interval(TICK_INTERVAL);
    // Первый тик срабатывает немедленно (catch-up семантика D-17)
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        select! {
            _ = interval.tick() => {
                let now = ctx.clock.unix_seconds();
                tick(&ctx, now).await;
            }
            _ = ctx.shutdown.cancelled() => {
                tracing::info!("Supervisor: shutdown signal received — exiting");
                break;
            }
        }
    }
}

/// Один тик планировщика: находит просроченные задачи и выполняет их.
async fn tick(ctx: &AppCtx, now: i64) {
    // Находим все задачи с next_run_at_utc <= now AND status != 'running'
    let readers = ctx.readers.clone();
    let overdue_names: Vec<String> = match tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        let mut stmt = conn
            .prepare(
                "SELECT name FROM scheduled_tasks \
                 WHERE next_run_at_utc IS NOT NULL \
                   AND next_run_at_utc <= ?1 \
                   AND status != 'running'",
            )
            .map_err(map_rusqlite)?;
        let names: Vec<String> = stmt
            .query_map(params![now], |r| r.get(0))
            .map_err(map_rusqlite)?
            .filter_map(|r| r.ok())
            .collect();
        Ok::<Vec<String>, AppError>(names)
    })
    .await
    {
        Ok(Ok(names)) => names,
        Ok(Err(e)) => {
            tracing::error!("Supervisor tick query error: {e}");
            return;
        }
        Err(e) => {
            tracing::error!("Supervisor tick spawn_blocking error: {e}");
            return;
        }
    };

    for task_name in overdue_names {
        let claimed = claim_task(ctx, &task_name, now).await;
        if !claimed {
            tracing::debug!("Supervisor: task {task_name} already claimed by another worker — skip");
            continue;
        }
        dispatch_task(ctx, &task_name, now).await;
    }
}

/// Атомарно захватывает задачу: UPDATE WHERE status != 'running'.
/// Возвращает true если захватили (rows_affected > 0).
async fn claim_task(ctx: &AppCtx, task_name: &str, now: i64) -> bool {
    let task_name_owned = task_name.to_string();
    match ctx
        .writer
        .execute(move |conn| {
            let rows_affected = conn
                .execute(
                    "UPDATE scheduled_tasks \
                     SET status='running', last_run_at_utc=?1 \
                     WHERE name=?2 AND status != 'running'",
                    params![now, task_name_owned],
                )
                .map_err(map_rusqlite)?;
            Ok(rows_affected)
        })
        .await
    {
        Ok(rows) => rows > 0,
        Err(e) => {
            tracing::error!("Supervisor claim_task error for {task_name}: {e}");
            false
        }
    }
}

/// Диспетчер задач по имени.
async fn dispatch_task(ctx: &AppCtx, task_name: &str, now: i64) {
    match task_name {
        "db_backup" => run_db_backup(ctx, now).await,
        "log_retention" => run_log_retention(ctx, now).await,
        unknown => {
            tracing::warn!("Supervisor: unknown task name '{unknown}'");
            mark_task_status(ctx, unknown, "failed", None).await;
        }
    }
}

/// Задача db_backup: читает backup_folder из payload_json или app_settings,
/// запускает BackupService::run_backup.
async fn run_db_backup(ctx: &AppCtx, now: i64) {
    // Читаем payload_json (там может быть backup_folder и retention)
    let readers = ctx.readers.clone();
    let payload_opt: Option<String> = match tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        let result: rusqlite::Result<Option<String>> = conn.query_row(
            "SELECT payload_json FROM scheduled_tasks WHERE name = 'db_backup'",
            [],
            |r| r.get(0),
        );
        result.map_err(map_rusqlite)
    })
    .await
    {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => {
            tracing::error!("db_backup: read payload_json error: {e}");
            mark_task_status(ctx, "db_backup", "failed", None).await;
            return;
        }
        Err(e) => {
            tracing::error!("db_backup: spawn_blocking error: {e}");
            mark_task_status(ctx, "db_backup", "failed", None).await;
            return;
        }
    };

    // Получаем backup_folder из конфига
    let db_path = ctx.paths.db_path().to_path_buf();
    let backup_svc = BackupService::new(
        ctx.writer.clone(),
        ctx.readers.clone(),
        ctx.clock.clone(),
        db_path,
    );

    let config = match backup_svc.get_config().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("db_backup: get_config error: {e}");
            mark_task_status(ctx, "db_backup", "failed", None).await;
            return;
        }
    };

    // Проверяем payload_json — может переопределять backup_folder
    let backup_folder = if let Some(payload) = payload_opt {
        let parsed: serde_json::Value = serde_json::from_str(&payload).unwrap_or_default();
        parsed
            .get("backup_folder")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or(config.backup_folder)
    } else {
        config.backup_folder
    };

    let Some(folder) = backup_folder else {
        tracing::warn!("db_backup: no backup_folder configured — task skipped");
        // Не считаем это ошибкой, просто нет конфига
        mark_task_status(ctx, "db_backup", "idle", None).await;
        return;
    };

    match backup_svc.run_backup(&folder).await {
        Ok(result) => {
            tracing::info!(
                "db_backup: backup created at {} (ts={})",
                result.file_path,
                result.timestamp_utc
            );
            // Следующий запуск: +24 часа (daily по умолчанию)
            let schedule = backup_svc
                .get_config()
                .await
                .map(|c| c.schedule)
                .unwrap_or_else(|_| "daily".to_string());
            let next_interval = match schedule.as_str() {
                "weekly" => WEEKLY_SECS,
                _ => DAILY_SECS, // daily / disabled / cron — default daily
            };
            let next_run = now + next_interval;
            mark_task_status(ctx, "db_backup", "succeeded", Some(next_run)).await;
        }
        Err(e) => {
            tracing::error!("db_backup: backup failed: {e}");
            mark_task_status(ctx, "db_backup", "failed", None).await;
        }
    }
}

/// Задача log_retention: удаляет ротированные лог-файлы старше 30 дней.
async fn run_log_retention(ctx: &AppCtx, now: i64) {
    let logs_dir = ctx.paths.exe_dir().join("logs");
    let cutoff_secs = LOG_RETENTION_SECS;

    let result = tokio::task::spawn_blocking(move || -> Result<usize, AppError> {
        if !logs_dir.exists() {
            return Ok(0);
        }

        let mut deleted = 0usize;
        let entries = std::fs::read_dir(&logs_dir).map_err(|e| AppError::Internal {
            source_chain: format!("log_retention read_dir: {e}"),
        })?;

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            // Пропускаем текущий лог (без timestamp-суффикса, просто .log)
            // Ротированные файлы имеют вид `trackly.YYYY-MM-DD` или аналогичный
            let name = entry
                .file_name()
                .to_string_lossy()
                .to_string();

            // Пропускаем текущий активный файл
            if name == "trackly.log" {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let modified = match metadata.modified() {
                Ok(t) => t,
                Err(_) => continue,
            };

            let age_secs = modified
                .elapsed()
                .unwrap_or(Duration::ZERO)
                .as_secs();

            if age_secs > cutoff_secs {
                if let Err(e) = std::fs::remove_file(&path) {
                    tracing::warn!(
                        "log_retention: failed to delete {}: {e}",
                        path.display()
                    );
                } else {
                    tracing::info!(
                        "log_retention: deleted old log file {} (age {}d)",
                        path.display(),
                        age_secs / 86400
                    );
                    deleted += 1;
                }
            }
        }
        Ok(deleted)
    })
    .await;

    match result {
        Ok(Ok(deleted)) => {
            tracing::info!("log_retention: deleted {deleted} old log file(s)");
            // Следующий запуск через 24 часа
            let next_run = now + DAILY_SECS;
            mark_task_status(ctx, "log_retention", "succeeded", Some(next_run)).await;
        }
        Ok(Err(e)) => {
            tracing::error!("log_retention: error: {e}");
            mark_task_status(ctx, "log_retention", "failed", None).await;
        }
        Err(e) => {
            tracing::error!("log_retention: spawn_blocking error: {e}");
            mark_task_status(ctx, "log_retention", "failed", None).await;
        }
    }
}

/// Обновляет status и next_run_at_utc задачи после выполнения.
async fn mark_task_status(ctx: &AppCtx, task_name: &str, status: &str, next_run: Option<i64>) {
    let task_name_owned = task_name.to_string();
    let status_owned = status.to_string();
    if let Err(e) = ctx
        .writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE scheduled_tasks \
                 SET status=?1, next_run_at_utc=?2 \
                 WHERE name=?3",
                params![status_owned, next_run, task_name_owned],
            )
            .map(|_| ())
            .map_err(map_rusqlite)
        })
        .await
    {
        tracing::error!("Supervisor mark_task_status error for {task_name}: {e}");
    }
}

/// Seed строк задач при старте AppCtx (INSERT OR IGNORE).
/// Вызывается из `context.rs` после прогона миграций.
pub async fn seed_supervisor_tasks(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    now: i64,
) -> Result<(), AppError> {
    writer
        .execute(move |conn| {
            // db_backup: next_run_at_utc = NULL (не активен пока не задан backup_folder)
            conn.execute(
                "INSERT OR IGNORE INTO scheduled_tasks (name, status, next_run_at_utc) \
                 VALUES ('db_backup', 'idle', NULL)",
                [],
            )
            .map_err(map_rusqlite)?;

            // log_retention: следующий запуск = сейчас (catch-up — фаилит на первом тике)
            conn.execute(
                "INSERT OR IGNORE INTO scheduled_tasks (name, status, next_run_at_utc) \
                 VALUES ('log_retention', 'idle', ?1)",
                params![now],
            )
            .map_err(map_rusqlite)?;

            Ok(())
        })
        .await
}
