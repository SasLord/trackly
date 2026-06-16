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
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;

use crate::dto::reports::TemplateEditorItem;
use crate::pdf::{
    docspec::{DocSpec, HeaderBlock, Section},
    minijinja_env::{build_safe_env, render_with_timeout},
    PdfRenderer,
};

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
    pub pdf: Arc<PdfRenderer>,
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
            pdf: Arc::new(PdfRenderer::new()),
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
                        tracing::info!("Seeded default template kind={kind} name={name}");
                    }
                }
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await
    }

    /// Возвращает все активные шаблоны для редактора (SET-07).
    pub async fn list_all_for_editor(&self) -> Result<Vec<TemplateEditorItem>, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<TemplateEditorItem>, AppError> {
            let conn = readers.acquire();
            let mut stmt = conn
                .prepare(
                    "SELECT id, kind, is_default, body_minijinja \
                     FROM document_templates \
                     WHERE is_active = 1 AND deleted_at_utc IS NULL",
                )
                .map_err(map_rusqlite)?;
            let items: Vec<TemplateEditorItem> = stmt
                .query_map([], |r| {
                    Ok(TemplateEditorItem {
                        id: r.get(0)?,
                        kind: r.get(1)?,
                        is_default: r.get::<_, bool>(2)?,
                        body: r.get(3)?,
                    })
                })
                .map_err(map_rusqlite)?
                .filter_map(|r| r.ok())
                .collect();
            Ok(items)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking list_all_for_editor: {e}"),
        })?
    }

    /// Обновляет тело шаблона. Требует `ManageSettings`.
    ///
    /// Валидирует синтаксис MiniJinja перед записью в БД.
    pub async fn update_body(
        &self,
        caller: &Identity,
        kind: &str,
        body: String,
    ) -> Result<(), AppError> {
        authorize(caller, &Action::ManageSettings)?;

        // Валидация синтаксиса MiniJinja
        {
            let mut env = minijinja::Environment::new();
            env.add_template_owned("_validate", body.clone())
                .map_err(|e| AppError::Validation {
                    field: "body".to_string(),
                    message: format!("Синтаксическая ошибка в шаблоне: {e}"),
                })?;
        }

        let kind_owned = kind.to_string();
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE document_templates \
                     SET body_minijinja=?2, is_default=0, \
                         updated_at_utc=?3, version=version+1 \
                     WHERE kind=?1 AND is_active=1 AND deleted_at_utc IS NULL",
                    params![kind_owned, body, now],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
    }

    /// Сбрасывает шаблон к встроенному дефолту. Требует `ManageSettings`.
    pub async fn reset_to_default(
        &self,
        caller: &Identity,
        kind: &str,
    ) -> Result<(), AppError> {
        authorize(caller, &Action::ManageSettings)?;

        // Ищем дефолтный шаблон по kind в DEFAULT_TEMPLATES
        let default_body = DEFAULT_TEMPLATES
            .iter()
            .find(|(k, _, _)| *k == kind)
            .map(|(_, _, body)| *body)
            .ok_or(AppError::NotFound {
                entity: "default_template",
                id: 0,
            })?;

        let kind_owned = kind.to_string();
        let body_owned = default_body.to_string();
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE document_templates \
                     SET body_minijinja=?2, is_default=1, \
                         updated_at_utc=?3, version=version+1 \
                     WHERE kind=?1 AND is_active=1 AND deleted_at_utc IS NULL",
                    params![kind_owned, body_owned, now],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
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

    /// Validate template syntax + render a preview PDF with demo context.
    ///
    /// Used by TemplateEditor to let the user see the rendered output before saving.
    /// The demo context mimics an act_handover document with placeholder data.
    pub async fn validate_preview(&self, body: &str) -> Result<Vec<u8>, AppError> {
        let env = build_safe_env();

        // Demo context — mirrors act_handover.minijinja variable names
        let demo_ctx = serde_json::json!({
            "act_number": "42",
            "act_date": "16.06.2026",
            "org_name": "Ваша организация",
            "giver_name": "Иванов И.И.",
            "receiver_name": "Петров П.П.",
            "items": [
                {
                    "device_name": "Принтер HP LaserJet",
                    "serial_number": "SN-001",
                    "inventory_number": "ИНВ-001",
                    "condition": "Рабочее",
                    "quantity": 1
                }
            ],
            "notes": "",
            "location_name": "Офис 101"
        });

        // Render via MiniJinja (validates syntax + fuel)
        let rendered_json =
            render_with_timeout(&env, "_preview", body, demo_ctx).await?;

        // Parse rendered JSON into DocSpec and render PDF
        let spec = serde_json::from_str::<DocSpec>(&rendered_json).unwrap_or_else(|_| {
            // Fallback if body renders plain text (not DocSpec JSON)
            DocSpec {
                title: "Превью шаблона".to_string(),
                header: HeaderBlock {
                    org_name: "Организация".to_string(),
                    org_inn: "".to_string(),
                    org_kpp: "".to_string(),
                    org_address: "".to_string(),
                    logo_path: None,
                    logo_bytes: None,
                    logo_mime: None,
                    act_label: "Превью шаблона".to_string(),
                    date_label: "16.06.2026".to_string(),
                },
                sections: vec![Section::Paragraph {
                    text: rendered_json.clone(),
                    style: Default::default(),
                }],
            }
        });

        self.pdf.render_docspec(&spec)
    }
}
