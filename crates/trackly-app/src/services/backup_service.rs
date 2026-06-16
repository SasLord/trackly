//! `BackupService` — создание SQLite-бэкапов через `rusqlite::backup::Backup`.
//!
//! Безопасность (T-07-02-02):
//!   - UNC-пути (`\\server\share` или `//...`) отклоняются.
//!   - Canonicalize()-ация после создания директории проверяет реальный путь ФС.
//!
//! Запрещено использовать `std::fs::copy` (clippy banned per CLAUDE.md).
//! Используем только `rusqlite::backup::Backup::new(...).run_to_completion(...)`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use rusqlite::{backup::Backup, Connection};
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;

use crate::dto::reports::BackupConfigPatch;

/// Результат успешного ручного бэкапа.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct BackupResult {
    #[specta(type = i32)]
    pub timestamp_utc: i64,
    pub file_path: String,
}

/// Конфигурация автоматического бэкапа (читается из app_settings).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct BackupConfigDto {
    pub backup_folder: Option<String>,
    pub schedule: String,
    #[specta(type = i32)]
    pub retention: i64,
}

#[derive(Clone)]
pub struct BackupService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub clock: Arc<dyn Clock + Send + Sync>,
    pub db_path: PathBuf,
}

impl BackupService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
        db_path: PathBuf,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            db_path,
        }
    }

    /// Проверяет что путь не является UNC (\\server\share или //...).
    /// Воспроизводит точную логику из fs_helpers.rs (D-UNC-01).
    fn reject_unc(path: &str) -> Result<(), AppError> {
        if path.starts_with("\\\\") || path.starts_with("//") {
            return Err(AppError::Validation {
                field: "path".to_string(),
                message: "UNC-пути не поддерживаются".to_string(),
            });
        }
        Ok(())
    }

    /// Запускает ручной бэкап в `dest_folder`.
    ///
    /// Имя файла: `trackly-backup-{unix_timestamp}.db`.
    /// После создания запускает `PRAGMA integrity_check` на копии.
    /// Применяет retention-политику (удаляет самые старые копии, оставляет ≤ retention).
    pub async fn run_backup(&self, dest_folder: &str) -> Result<BackupResult, AppError> {
        Self::reject_unc(dest_folder)?;

        let timestamp = self.clock.unix_seconds();
        let dest_folder_owned = dest_folder.to_string();
        let readers = self.readers.clone();

        // Читаем retention из конфига
        let config = self.get_config().await?;
        let retention = config.retention;

        tokio::task::spawn_blocking(move || -> Result<BackupResult, AppError> {
            // Создаём папку если не существует
            std::fs::create_dir_all(&dest_folder_owned).map_err(|e| AppError::Validation {
                field: "backup_folder".to_string(),
                message: format!("Не удалось создать папку бэкапов: {e}"),
            })?;

            // Canonicalize — проверяем что путь реально существует
            let canonical_folder =
                std::fs::canonicalize(&dest_folder_owned).map_err(|e| AppError::Validation {
                    field: "backup_folder".to_string(),
                    message: format!("Путь бэкапа невалиден: {e}"),
                })?;

            // Проверяем отсутствие `..` в canonical пути (defense-in-depth T-07-02-02)
            use std::path::Component;
            if canonical_folder
                .components()
                .any(|c| matches!(c, Component::ParentDir))
            {
                return Err(AppError::Validation {
                    field: "backup_folder".to_string(),
                    message: "Путь не должен содержать «..»".to_string(),
                });
            }

            let filename = format!("trackly-backup-{timestamp}.db");
            let dest_path = canonical_folder.join(&filename);
            let dest_path_str = dest_path.to_string_lossy().to_string();

            // Backup через rusqlite::backup::Backup (NOT fs::copy — clippy ban)
            run_rusqlite_backup(&readers, &dest_path)?;

            // Retention cleanup
            apply_retention(&canonical_folder, retention)?;

            Ok(BackupResult {
                timestamp_utc: timestamp,
                file_path: dest_path_str,
            })
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking BackupService::run_backup: {e}"),
        })?
    }

    /// Копирует БД в конкретный путь (без авто-имени и retention).
    /// Используется при перемещении файла БД.
    pub async fn backup_to_path(&self, dest_path: &Path) -> Result<(), AppError> {
        let dest_str = dest_path.to_str().unwrap_or("");
        Self::reject_unc(dest_str)?;

        let dest_path_owned = dest_path.to_path_buf();
        let readers = self.readers.clone();

        tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            run_rusqlite_backup(&readers, &dest_path_owned)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking BackupService::backup_to_path: {e}"),
        })?
    }

    /// Читает конфигурацию бэкапа из `app_settings`.
    pub async fn get_config(&self) -> Result<BackupConfigDto, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<BackupConfigDto, AppError> {
            let conn = readers.acquire();

            let backup_folder = read_setting(&conn, "backup_folder")?;
            let schedule = read_setting(&conn, "backup_schedule")?
                .unwrap_or_else(|| "disabled".to_string());
            let retention_str = read_setting(&conn, "backup_retention")?;
            let retention: i64 = retention_str
                .as_deref()
                .unwrap_or("7")
                .parse()
                .unwrap_or(7);

            Ok(BackupConfigDto {
                backup_folder,
                schedule,
                retention,
            })
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking BackupService::get_config: {e}"),
        })?
    }

    /// Сохраняет конфигурацию бэкапа в `app_settings`.
    pub async fn set_config(
        &self,
        caller: &Identity,
        patch: BackupConfigPatch,
    ) -> Result<(), AppError> {
        authorize(caller, &Action::ManageSettings)?;

        if let Some(ref folder) = patch.backup_folder {
            Self::reject_unc(folder)?;
        }

        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                if let Some(folder) = patch.backup_folder {
                    upsert_setting(conn, "backup_folder", &folder, now)?;
                }
                if let Some(schedule) = patch.schedule {
                    upsert_setting(conn, "backup_schedule", &schedule, now)?;
                }
                if let Some(retention) = patch.retention {
                    upsert_setting(conn, "backup_retention", &retention.to_string(), now)?;
                }
                Ok(())
            })
            .await
    }
}

/// Внутренняя функция: выполняет rusqlite::backup::Backup + integrity_check.
fn run_rusqlite_backup(readers: &ReaderPool, dest_path: &Path) -> Result<(), AppError> {
    // Открываем destination connection
    let mut dest_conn = Connection::open(dest_path).map_err(|e| AppError::Internal {
        source_chain: format!("backup dest open {}: {e}", dest_path.display()),
    })?;

    // Выполняем backup в блоке чтобы grd и Backup дропнулись до integrity_check
    {
        // Acquire reader для backup source
        let reader_guard = readers.acquire();

        // rusqlite::backup::Backup требует &Connection и &mut Connection
        // SAFETY: acquire() возвращает RAII guard с Deref<Target=Connection>
        let backup = Backup::new(&reader_guard, &mut dest_conn).map_err(|e| AppError::Internal {
            source_chain: format!("Backup::new failed: {e}"),
        })?;

        backup
            .run_to_completion(500, Duration::from_millis(250), None)
            .map_err(|e| AppError::Internal {
                source_chain: format!("Backup::run_to_completion failed: {e}"),
            })?;

        // reader_guard и backup дропаются здесь — освобождает заимствования
    }

    // Проверяем целостность скопированной БД (после дропа Backup)
    let integrity_result: String = dest_conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(map_rusqlite)?;

    if integrity_result != "ok" {
        return Err(AppError::Validation {
            field: "backup".to_string(),
            message: format!("Целостность бэкапа нарушена: {integrity_result}"),
        });
    }

    Ok(())
}

/// Удаляет старые файлы бэкапа, оставляя не более `retention` штук.
fn apply_retention(folder: &Path, retention: i64) -> Result<(), AppError> {
    let retention = retention.max(1) as usize;

    let mut entries: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(folder)
        .map_err(|e| AppError::Internal {
            source_chain: format!("read_dir for retention: {e}"),
        })?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if !name_str.starts_with("trackly-backup-") || !name_str.ends_with(".db") {
                return None;
            }
            let metadata = entry.metadata().ok()?;
            let modified = metadata.modified().ok()?;
            Some((modified, entry.path()))
        })
        .collect();

    // Сортируем: самые старые — в начале
    entries.sort_by_key(|(time, _)| *time);

    // Удаляем пока не достигнем retention limit
    while entries.len() > retention {
        let (_, path) = entries.remove(0);
        if let Err(e) = std::fs::remove_file(&path) {
            tracing::warn!(
                "BackupService retention: failed to delete {}: {e}",
                path.display()
            );
        } else {
            tracing::info!("BackupService retention: deleted old backup {}", path.display());
        }
    }

    Ok(())
}

/// Читает одно значение из `app_settings` по ключу. Возвращает None если нет.
fn read_setting(conn: &Connection, key: &str) -> Result<Option<String>, AppError> {
    let result: rusqlite::Result<String> = conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        rusqlite::params![key],
        |r| r.get(0),
    );
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(map_rusqlite(e)),
    }
}

/// Upsert значения в `app_settings` (pattern из V016 migration).
fn upsert_setting(
    conn: &Connection,
    key: &str,
    value: &str,
    now: i64,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
         VALUES (?1, ?2, ?3, ?3) \
         ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at_utc=excluded.updated_at_utc",
        rusqlite::params![key, value, now],
    )
    .map(|_| ())
    .map_err(map_rusqlite)
}
