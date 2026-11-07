//! Administrative side of the attachments subsystem (Phase 41.3 Plan 08):
//! size-limit setting, startup integrity scan and the persisted report.
//!
//! The scan is deliberately conservative (D-20, T-41.3-30): it never modifies
//! DB rows, deletes nothing when the root is unavailable, and touches only
//! orphan blobs older than 24 h and stale `.tmp` files.

use std::collections::HashSet;
use std::time::{Duration, SystemTime};

use rusqlite::{Connection, OptionalExtension};
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::domain::attachments::{
    DEFAULT_MAX_BYTES, MAX_BYTES_CEILING, SETTING_INTEGRITY_REPORT, SETTING_MAX_BYTES,
};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

use super::attachment_service::AttachmentService;
use crate::dto::attachment::{AttachmentLimitDto, IntegrityReportDto};

/// Age after which an orphan blob or temp file may be removed.
const GRACE_SECS: u64 = 24 * 60 * 60;
const MIN_LIMIT_BYTES: i64 = 1024 * 1024;

fn internal(e: tokio::task::JoinError) -> AppError {
    AppError::Internal {
        source_chain: format!("spawn_blocking: {e}"),
    }
}

fn json_err(e: serde_json::Error) -> AppError {
    AppError::Internal {
        source_chain: format!("attachments report json: {e}"),
    }
}

fn upsert_setting(conn: &Connection, key: &str, value: &str, now: i64) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
         VALUES (?1, ?2, ?3, ?3) \
         ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at_utc=excluded.updated_at_utc",
        rusqlite::params![key, value, now],
    )
    .map(|_| ())
    .map_err(map_rusqlite)
}

/// True when `mtime` is known and older than the grace period at `now`.
fn older_than_grace(mtime: Option<SystemTime>, now: SystemTime) -> bool {
    match mtime {
        Some(m) => now
            .duration_since(m)
            .map(|d| d > Duration::from_secs(GRACE_SECS))
            .unwrap_or(false),
        None => false,
    }
}

impl AttachmentService {
    /// Current limit and storage directory (Admin only).
    pub async fn get_limit(&self, caller: &Identity) -> Result<AttachmentLimitDto, AppError> {
        authorize(caller, &Action::ManageSettings)?;
        Ok(AttachmentLimitDto {
            max_bytes: self.max_bytes().await?,
            default_bytes: DEFAULT_MAX_BYTES as i64,
            ceiling_bytes: MAX_BYTES_CEILING as i64,
            path: self.store.root().to_string_lossy().into_owned(),
        })
    }

    /// Sets the maximum file size, 1 MiB..=100 MiB (D-07). Existing files stay.
    pub async fn set_max_bytes(&self, caller: &Identity, bytes: i64) -> Result<(), AppError> {
        authorize(caller, &Action::ManageSettings)?;
        if !(MIN_LIMIT_BYTES..=MAX_BYTES_CEILING as i64).contains(&bytes) {
            return Err(AppError::Validation {
                field: "attachment_max_bytes".to_string(),
                message: "Размер файла должен быть от 1 до 100 МБ".to_string(),
            });
        }
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| upsert_setting(conn, SETTING_MAX_BYTES, &bytes.to_string(), now))
            .await
    }

    /// Integrity scan (D-20). Never panics, never modifies rows.
    pub async fn run_integrity_scan(&self) -> Result<IntegrityReportDto, AppError> {
        let now_secs = self.clock.unix_seconds();
        let store = self.store.clone();
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let report =
            tokio::task::spawn_blocking(move || -> Result<IntegrityReportDto, AppError> {
                let mut report = IntegrityReportDto {
                    checked_at_utc: now_secs,
                    status: "ok".to_string(),
                    ..Default::default()
                };
                if !store.root_available() {
                    report.status = "unavailable".to_string();
                    return Ok(report);
                }
                let scan = store.scan().map_err(|e| AppError::Internal {
                    source_chain: format!("attachments scan: {e}"),
                })?;
                // Rows are read AFTER the directory scan: a blob moved in by an
                // upload that committed meanwhile already has its row visible.
                let db_hashes: HashSet<String> = {
                    let conn = readers.acquire();
                    repo.hashes_with_totals_on(&conn)?
                        .into_iter()
                        .map(|(h, _, _)| h)
                        .collect()
                };
                let now = SystemTime::UNIX_EPOCH + Duration::from_secs(now_secs.max(0) as u64);
                let mut on_disk: HashSet<&str> = HashSet::new();
                for b in &scan.blobs {
                    report.files += 1;
                    report.bytes += b.size as i64;
                    on_disk.insert(b.hash.as_str());
                    if db_hashes.contains(&b.hash) {
                        continue;
                    }
                    if older_than_grace(b.mtime, now) {
                        match store.remove_blob(&b.hash) {
                            Ok(()) => report.orphans_removed += 1,
                            Err(e) => tracing::warn!("attachments scan: orphan not removed: {e}"),
                        }
                    } else {
                        report.orphans_young += 1;
                    }
                }
                report.lost = db_hashes
                    .iter()
                    .filter(|h| !on_disk.contains(h.as_str()))
                    .count() as i64;
                for (path, mtime) in &scan.tmp_files {
                    if older_than_grace(*mtime, now) {
                        if let Err(e) = std::fs::remove_file(path) {
                            tracing::warn!("attachments scan: tmp not removed: {e}");
                        }
                    }
                }
                Ok(report)
            })
            .await
            .map_err(internal)??;

        let json = serde_json::to_string(&report).map_err(json_err)?;
        self.writer
            .execute(move |conn| upsert_setting(conn, SETTING_INTEGRITY_REPORT, &json, now_secs))
            .await?;
        if report.lost > 0 {
            tracing::warn!(
                lost = report.lost,
                orphans_removed = report.orphans_removed,
                orphans_young = report.orphans_young,
                status = %report.status,
                "attachments integrity scan finished"
            );
        } else {
            tracing::info!(
                files = report.files,
                orphans_removed = report.orphans_removed,
                orphans_young = report.orphans_young,
                status = %report.status,
                "attachments integrity scan finished"
            );
        }
        Ok(report)
    }

    /// Last persisted report with fresh deleted-attachment totals (Admin only).
    pub async fn get_report(
        &self,
        caller: &Identity,
    ) -> Result<Option<IntegrityReportDto>, AppError> {
        authorize(caller, &Action::ManageSettings)?;
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        tokio::task::spawn_blocking(move || -> Result<Option<IntegrityReportDto>, AppError> {
            let conn = readers.acquire();
            let raw: Option<String> = conn
                .query_row(
                    "SELECT value FROM app_settings WHERE key = ?1",
                    rusqlite::params![SETTING_INTEGRITY_REPORT],
                    |r| r.get(0),
                )
                .optional()
                .map_err(map_rusqlite)?;
            let Some(raw) = raw else { return Ok(None) };
            let mut report: IntegrityReportDto = serde_json::from_str(&raw).map_err(json_err)?;
            let (rows, bytes) = repo.count_deleted_with_bytes_on(&conn)?;
            report.deleted_rows = rows;
            report.deleted_bytes = bytes;
            Ok(Some(report))
        })
        .await
        .map_err(internal)?
    }
}
