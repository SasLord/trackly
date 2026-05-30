//! `TemplateService` — seed дефолтных шаблонов на первом запуске + чтение
//! активного шаблона для рендера.
//!
//! Schema (V007): `document_templates` хранит шаблоны MiniJinja с
//! партициированным unique-index по `(kind)` для активных записей.
//!
//! Seed-семантика (D-Templates-Seed-01):
//! - При `AppCtx::build` вызывается `seed_defaults_on_startup`.
//! - Для каждого `kind` (`act_handover`, `act_acceptance`) проверяем — есть
//!   ли активная (non-deleted) запись. Если нет — INSERT дефолта.
//! - Идемпотентно: повторный запуск никогда не дублирует.
//! - При soft-delete всех версий — следующий запуск восстанавливает дефолт.

use std::sync::Arc;

use rusqlite::params;
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;

/// Дефолтные шаблоны, embed'нутые в бинарь (`include_str!`). Сидятся в БД
/// при первом запуске и при полной soft-delete всех версий kind'а.
pub const DEFAULT_TEMPLATES: &[(&str, &str, &str)] = &[
    (
        "act_handover",
        "Дефолтный шаблон акта приёма-передачи",
        include_str!("../../templates/act_handover.minijinja"),
    ),
    (
        "act_acceptance",
        "Дефолтный шаблон документа приёма",
        include_str!("../../templates/act_acceptance.minijinja"),
    ),
];

#[derive(Clone)]
pub struct TemplateService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub clock: Arc<dyn Clock + Send + Sync>,
}

impl TemplateService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
        }
    }

    /// Sees missing default templates. Идемпотентно: проверяет COUNT активных
    /// (`is_active = 1 AND deleted_at_utc IS NULL`) и INSERT'ит дефолт только
    /// если 0.
    pub async fn seed_defaults_on_startup(&self) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                for (kind, name, body) in DEFAULT_TEMPLATES.iter() {
                    let active_count: i64 = tx
                        .query_row(
                            "SELECT COUNT(*) FROM document_templates \
                             WHERE kind = ?1 AND is_active = 1 AND deleted_at_utc IS NULL",
                            params![kind],
                            |r| r.get(0),
                        )
                        .map_err(map_rusqlite)?;
                    if active_count == 0 {
                        tx.execute(
                            "INSERT INTO document_templates \
                             (kind, name, body_minijinja, is_active, version, \
                              created_at_utc, updated_at_utc) \
                             VALUES (?1, ?2, ?3, 1, 1, ?4, ?4)",
                            params![kind, name, body, now],
                        )
                        .map_err(map_rusqlite)?;
                        tracing::info!(
                            "Seeded default template kind={kind} name={name}"
                        );
                    }
                }
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await
    }

    /// Возвращает body активного шаблона для kind. Если нет активных
    /// (всё soft-deleted) — `AppError::NotFound`.
    pub async fn get_active(&self, kind: &str) -> Result<String, AppError> {
        let readers = self.readers.clone();
        let kind_owned = kind.to_string();
        tokio::task::spawn_blocking(move || -> Result<String, AppError> {
            let conn = readers.acquire();
            let body: Option<String> = conn
                .query_row(
                    "SELECT body_minijinja FROM document_templates \
                     WHERE kind = ?1 AND is_active = 1 AND deleted_at_utc IS NULL \
                     ORDER BY updated_at_utc DESC, id DESC LIMIT 1",
                    params![kind_owned],
                    |r| r.get(0),
                )
                .map(Some)
                .or_else(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => Ok(None),
                    other => Err(map_rusqlite(other)),
                })?;
            body.ok_or(AppError::NotFound {
                entity: "document_template",
                id: 0,
            })
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking get_active: {e}"),
        })?
    }
}
