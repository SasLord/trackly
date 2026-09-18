//! `NumberTemplateService` — единственный владелец бизнес-логики шаблонов
//! номеров (Phase 40.2, Plan 04): CRUD с пред-проверкой дубля и аудитом (по
//! образцу `cartridge_service.rs::model_create`), память последнего шаблона
//! на 5 контекстов (NUM-08, Task 3), проверка занятости номера в
//! пространстве (NUM-09, Task 3) и обнаружение предупреждений — смешение
//! алфавитов / двойник (NUM-12, Task 3).
//!
//! Single-writer discipline: every mutation goes through
//! `WriterHandle::execute(closure)` inside its own `BEGIN`/`COMMIT`
//! transaction — the same pattern as `CartridgeService`.
//!
//! ## T-40.2-07 (Elevation of Privilege) — authorization is NOT done here
//!
//! Every public method on this service is called EXCLUSIVELY from the
//! `build_*` handler functions of Plan 05 (Tauri commands / axum handlers),
//! which perform `authorize()` BEFORE calling into this service. This
//! service does not re-check the caller's role — callers (including Wave 5's
//! `act_service`/`cartridge_service`/`device_service`) MUST NOT expose these
//! methods to a transport boundary without gating them first.

use std::sync::Arc;

use rusqlite::{params, Connection, OptionalExtension};

use trackly_core::domain::number_templates::{NextNumberResult, NumberTemplateRow, TemplateType};
use trackly_core::error::AppError;
use trackly_core::ports::number_templates::NumberTemplateRepository;
use trackly_core::primitives::clock::Clock;
use trackly_core::text::mask::{self, ParsedMask};
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::audit_log_sqlite::AuditEntry;
use trackly_infra::repos::{SqliteAuditLogRepository, SqliteNumberTemplateRepository};

use crate::dto::number_template::{NextNumberDto, NumberTemplateDto, TemplateTypeDto};

/// Application service for numbering templates. `Arc`-fields keep `Clone` O(1).
#[derive(Clone)]
pub struct NumberTemplateService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) repo: Arc<SqliteNumberTemplateRepository>,
    pub(crate) audit_repo: Arc<SqliteAuditLogRepository>,
}

impl NumberTemplateService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            repo: Arc::new(SqliteNumberTemplateRepository),
            audit_repo: Arc::new(SqliteAuditLogRepository),
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Build a [`NumberTemplateDto`] from a saved row, rendering both preview
    /// numbers through `mask::render_number` — the client never renders a
    /// number from raw digits itself.
    fn row_to_dto(
        repo: &SqliteNumberTemplateRepository,
        conn: &Connection,
        row: NumberTemplateRow,
        today_utc: i64,
    ) -> Result<NumberTemplateDto, AppError> {
        let parsed = mask::parse_and_expand(&row.mask, today_utc)?;
        let next = repo.compute_next_for_template(conn, &row, today_utc)?;
        Ok(NumberTemplateDto {
            id: row.id,
            template_type: row.template_type.into(),
            mask: row.mask,
            next_first_free: mask::render_number(&parsed, next.first_free),
            next_max_plus_one: mask::render_number(&parsed, next.max_plus_one),
            has_gap: next.has_gap,
            overflowed: next.overflowed,
            version: row.version,
        })
    }

    /// Map a raw [`NextNumberResult`] into the wire DTO, applying the
    /// `alt_rendered` rule from `dto::number_template::NextNumberDto`'s
    /// doc-comment (NUM-07 gap toggle).
    fn next_number_dto(parsed: &ParsedMask, next: &NextNumberResult) -> NextNumberDto {
        let rendered = mask::render_number(parsed, next.first_free);
        let alt_rendered = if next.has_gap && !next.overflowed && next.max_plus_one_fits_width {
            Some(mask::render_number(parsed, next.max_plus_one))
        } else {
            None
        };
        NextNumberDto {
            rendered,
            alt_rendered,
            has_gap: next.has_gap,
            overflowed: next.overflowed,
        }
    }

    // -----------------------------------------------------------------------
    // CRUD
    // -----------------------------------------------------------------------

    pub async fn create(
        &self,
        template_type: TemplateTypeDto,
        mask_str: String,
    ) -> Result<NumberTemplateDto, AppError> {
        mask::validate_mask(&mask_str)?;
        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();
        let core_type: TemplateType = template_type.into();

        let id = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // Pre-check for a duplicate (type, mask) pair to return a
                // human-readable Russian conflict reason instead of the raw
                // SQLite UNIQUE(type, mask) violation (V041) — mirrors
                // `cartridge_service.rs::model_create`.
                let exists: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM number_templates WHERE type = ?1 AND mask = ?2)",
                        params![core_type.as_str(), mask_str],
                        |r| r.get(0),
                    )
                    .map_err(map_rusqlite)?;
                if exists {
                    return Err(AppError::Conflict {
                        reason: format!(
                            "Шаблон с такой маской для типа «{}» уже есть.",
                            core_type.display_name_ru()
                        ),
                    });
                }

                let id = repo.insert_in_tx(&tx, core_type.clone(), &mask_str, now)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "number_template",
                        entity_id: id,
                        action: "create",
                        user_id: None,
                        before_json: None,
                        after_json: None,
                        payload_json: Some(
                            serde_json::json!({"type": core_type.as_str(), "mask": mask_str})
                                .to_string(),
                        ),
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(id)
            })
            .await?;

        self.get(id).await
    }

    /// Update ONLY the `mask` of an existing template. `template_type` is
    /// intentionally NOT a parameter here — D-10 immutability enforced at
    /// the signature level (mirrors
    /// `SqliteNumberTemplateRepository::update_mask_in_tx`).
    pub async fn update_mask(
        &self,
        id: i64,
        mask_str: String,
        version: i64,
    ) -> Result<NumberTemplateDto, AppError> {
        mask::validate_mask(&mask_str)?;
        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                let type_str: String = tx
                    .query_row(
                        "SELECT type FROM number_templates WHERE id = ?1",
                        params![id],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(map_rusqlite)?
                    .ok_or(AppError::NotFound {
                        entity: "number_template",
                        id,
                    })?;
                let template_type = TemplateType::from_str(&type_str)?;

                // Duplicate pre-check excludes THIS row (editing a template's
                // mask to its own current value is never a conflict).
                let conflict: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM number_templates \
                          WHERE type = ?1 AND mask = ?2 AND id != ?3)",
                        params![type_str, mask_str, id],
                        |r| r.get(0),
                    )
                    .map_err(map_rusqlite)?;
                if conflict {
                    return Err(AppError::Conflict {
                        reason: format!(
                            "Шаблон с такой маской для типа «{}» уже есть.",
                            template_type.display_name_ru()
                        ),
                    });
                }

                repo.update_mask_in_tx(&tx, id, &mask_str, version, now)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "number_template",
                        entity_id: id,
                        action: "update",
                        user_id: None,
                        before_json: None,
                        after_json: None,
                        payload_json: Some(serde_json::json!({"mask": mask_str}).to_string()),
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await?;

        self.get(id).await
    }

    /// Physically DELETE the template (D-12). `ON DELETE SET NULL` (V041)
    /// clears any context's memory automatically — no extra step here.
    pub async fn delete(&self, id: i64) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                repo.delete_in_tx(&tx, id)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "number_template",
                        entity_id: id,
                        action: "delete",
                        user_id: None,
                        before_json: None,
                        after_json: None,
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await
    }

    pub async fn list(
        &self,
        template_type: Option<TemplateTypeDto>,
    ) -> Result<Vec<NumberTemplateDto>, AppError> {
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let now = self.clock.unix_seconds();
        let core_type = template_type.map(TemplateType::from);
        tokio::task::spawn_blocking(move || -> Result<Vec<NumberTemplateDto>, AppError> {
            let conn = readers.acquire();
            let rows = repo.list(&conn, core_type)?;
            rows.into_iter()
                .map(|row| Self::row_to_dto(&repo, &conn, row, now))
                .collect()
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    pub async fn get(&self, id: i64) -> Result<NumberTemplateDto, AppError> {
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let now = self.clock.unix_seconds();
        tokio::task::spawn_blocking(move || -> Result<NumberTemplateDto, AppError> {
            let conn = readers.acquire();
            let row = repo.get(&conn, id)?;
            Self::row_to_dto(&repo, &conn, row, now)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    // -----------------------------------------------------------------------
    // Preview
    // -----------------------------------------------------------------------

    /// Next number for an ALREADY SAVED template (drives the "Вставка" menu
    /// and the template list's preview column).
    pub async fn peek_next(&self, template_id: i64) -> Result<NextNumberDto, AppError> {
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let now = self.clock.unix_seconds();
        tokio::task::spawn_blocking(move || -> Result<NextNumberDto, AppError> {
            let conn = readers.acquire();
            let row = repo.get(&conn, template_id)?;
            let parsed = mask::parse_and_expand(&row.mask, now)?;
            let next = repo.compute_next_for_template(&conn, &row, now)?;
            Ok(Self::next_number_dto(&parsed, &next))
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    /// Preview the next number for a mask that is NOT (yet) saved as a
    /// `number_templates` row — needed by the create-template modal (Plan
    /// 10) before the user commits the mask.
    ///
    /// Reuses `SqliteNumberTemplateRepository::compute_next_for_template`
    /// (Plan 03) by constructing a transient, never-persisted
    /// `NumberTemplateRow` (`id: 0`) instead of duplicating its
    /// prefix/suffix/digit-extraction logic here a second time — this keeps
    /// the single source of truth for "how a mask turns into a next number"
    /// inside Plan 03's repository file, which is outside this plan's
    /// `files_modified` scope.
    pub async fn preview_mask(
        &self,
        template_type: TemplateTypeDto,
        mask_str: String,
        today_utc: i64,
    ) -> Result<NextNumberDto, AppError> {
        mask::validate_mask(&mask_str)?;
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let core_type: TemplateType = template_type.into();
        tokio::task::spawn_blocking(move || -> Result<NextNumberDto, AppError> {
            let conn = readers.acquire();
            let parsed = mask::parse_and_expand(&mask_str, today_utc)?;
            let synthetic = NumberTemplateRow {
                id: 0,
                template_type: core_type,
                mask: mask_str,
                created_at_utc: 0,
                updated_at_utc: 0,
                version: 0,
            };
            let next = repo.compute_next_for_template(&conn, &synthetic, today_utc)?;
            Ok(Self::next_number_dto(&parsed, &next))
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }
}
