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
//!
//! Auto-upgrade бандл-дефолта (D-Templates-Seed-02, quick task 260704-uw3):
//! - Если активная запись уже существует, `is_default = 1` (т.е. пользователь
//!   её не кастомизировал через `update_body`) и её `body_minijinja` не
//!   совпадает с текущим встроенным `DEFAULT_TEMPLATES` — она обновляется на
//!   месте (`UPDATE`, `version+1`), тем самым существующие БД подхватывают
//!   изменения бандл-шаблона из новых релизов без ре-сида с нуля.
//! - Строки с `is_default = 0` (пользователь вызывал `update_body`) этой
//!   веткой никогда не трогаются — они находятся вне области auto-upgrade.
//! - Если тело уже совпадает с бандлом — запись не трогаем (нет write, нет
//!   version bump) — повторные запуски idempotent.

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

    /// Seeds missing default templates and auto-upgrades stale ones.
    ///
    /// For each `kind`: looks up the active (non-deleted) row's
    /// `(is_default, body_minijinja)`, if any, then branches:
    /// - no active row → `INSERT` bundled default (`is_default=1, version=1`).
    /// - active row, `is_default=1`, body differs from bundled → `UPDATE`
    ///   in place (`is_default=1`, `version=version+1`) — mirrors
    ///   `reset_to_default`'s UPDATE shape.
    /// - active row, `is_default=0` (user-customized) or body already
    ///   matches bundled → no-op.
    pub async fn seed_defaults_on_startup(&self) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                for (kind, name, body) in DEFAULT_TEMPLATES.iter() {
                    let existing: Option<(bool, String)> = tx
                        .query_row(
                            "SELECT is_default, body_minijinja FROM document_templates \
                             WHERE kind = ?1 AND is_active = 1 AND deleted_at_utc IS NULL",
                            params![kind],
                            |r| Ok((r.get::<_, bool>(0)?, r.get::<_, String>(1)?)),
                        )
                        .map(Some)
                        .or_else(|e| match e {
                            rusqlite::Error::QueryReturnedNoRows => Ok(None),
                            other => Err(map_rusqlite(other)),
                        })?;

                    match existing {
                        None => {
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
                        Some((is_default, stored_body)) if is_default && &stored_body != body => {
                            tx.execute(
                                "UPDATE document_templates \
                                 SET body_minijinja=?2, is_default=1, \
                                     updated_at_utc=?3, version=version+1 \
                                 WHERE kind=?1 AND is_active=1 AND deleted_at_utc IS NULL",
                                params![kind, body, now],
                            )
                            .map_err(map_rusqlite)?;
                            tracing::info!(
                                "Auto-upgraded default template kind={kind} name={name}"
                            );
                        }
                        Some(_) => {
                            // is_default=0 (user-customized) or body already
                            // matches bundled default — no-op.
                        }
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
                let n = conn
                    .execute(
                        "UPDATE document_templates \
                         SET body_minijinja=?2, is_default=0, \
                             updated_at_utc=?3, version=version+1 \
                         WHERE kind=?1 AND is_active=1 AND deleted_at_utc IS NULL",
                        params![kind_owned, body, now],
                    )
                    .map_err(map_rusqlite)?;
                if n == 0 {
                    return Err(AppError::NotFound {
                        entity: "document_template",
                        id: 0,
                    });
                }
                Ok(())
            })
            .await
    }

    /// Сбрасывает шаблон к встроенному дефолту. Требует `ManageSettings`.
    pub async fn reset_to_default(&self, caller: &Identity, kind: &str) -> Result<(), AppError> {
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
                let n = conn
                    .execute(
                        "UPDATE document_templates \
                         SET body_minijinja=?2, is_default=1, \
                             updated_at_utc=?3, version=version+1 \
                         WHERE kind=?1 AND is_active=1 AND deleted_at_utc IS NULL",
                        params![kind_owned, body_owned, now],
                    )
                    .map_err(map_rusqlite)?;
                if n == 0 {
                    return Err(AppError::NotFound {
                        entity: "document_template",
                        id: 0,
                    });
                }
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

    /// Validate template syntax + render a preview PDF with demo context.
    ///
    /// Used by TemplateEditor to let the user see the rendered output before saving.
    /// The demo context mimics an act_handover document with placeholder data.
    pub async fn validate_preview(&self, body: &str) -> Result<Vec<u8>, AppError> {
        let env = build_safe_env();

        // Demo context — mirrors act_handover.minijinja nested variable schema:
        //   org.{name,inn,kpp,address,logo_path}
        //   act.{number,suffix,date,date_human,giver_name,receiver_name,
        //        location_name,deadline,deadline_human,parent,items[]}
        //   act.items[].{name,inventory_no,serial_no,model,quantity}
        // UndefinedBehavior::Strict requires every referenced variable to be present.
        let demo_ctx = serde_json::json!({
            "org": {
                "name": "ООО Демо Организация",
                "inn": "7700000000",
                "kpp": "770000000",
                "address": "г. Москва, ул. Примерная, д. 1",
                "logo_path": null,
                "phone": "(3919) 75-90-98",
                "fax": "(3919) 75-08-59",
                "email": "info@demo-org.ru",
                "okpo": "10176125",
                "ogrn": "1122452000714"
            },
            "act": {
                "number": "42",
                "suffix": null,
                "date": "2026-06-17",
                "date_human": "17 июня 2026",
                "giver_name": "Иванов И.И.",
                "receiver_name": "Петров П.П.",
                "location_name": "Офис 101",
                "deadline": null,
                "deadline_human": null,
                "parent": null,
                "items": [
                    {
                        "name": "HP LaserJet Pro M404n",
                        "inventory_no": "ИНВ-001",
                        "serial_no": "SN-001",
                        "model": "LaserJet Pro M404n",
                        "quantity": 1,
                        "specs": "Диагональ: 27 дюймов",
                        "kit": "Монитор, подставка, кабель питания",
                        "condition": "Новый в заводской упаковке"
                    }
                ]
            },
            "return": {
                "condition_default": "Рабочее",
                "location_default": "Склад"
            },
            // G2-4: device and document keys required by act_acceptance.minijinja
            "device": {
                "name": "HP LaserJet Pro M404n",
                "inventory_no": "ИНВ-001",
                "serial_no": "SN-001",
                "model": "LaserJet Pro M404n",
                "condition": "Рабочее"
            },
            "document": {
                "giver_name": "Иванов И.И.",
                "receiver_name": "Петров П.П.",
                "date_human": "17 июня 2026"
            }
        });

        // Render via MiniJinja (validates syntax + fuel)
        let rendered_json = render_with_timeout(&env, "_preview", body, demo_ctx).await?;

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
                    org_phone: "".to_string(),
                    org_fax: "".to_string(),
                    org_email: "".to_string(),
                    org_okpo: "".to_string(),
                    org_ogrn: "".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use trackly_infra::{
        clock_impl::SystemClock,
        db::{pools::ReaderPool, writer_worker::WriterHandle},
    };

    fn build_test_db() -> (Arc<WriterHandle>, Arc<ReaderPool>) {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let db_path = tmp.path().to_path_buf();
        std::mem::forget(tmp);
        let mut conn = rusqlite::Connection::open(&db_path).unwrap();
        trackly_infra::db::pragmas::apply_writer_pragmas(&conn).unwrap();
        trackly_infra::db::migrations::run(&mut conn).unwrap();
        let writer = Arc::new(WriterHandle::spawn(conn));
        let readers = Arc::new(ReaderPool::new(&db_path, 2).unwrap());
        (writer, readers)
    }

    /// G2-4: validate_preview with the default act_acceptance template must return
    /// valid PDF bytes. Previously failed because demo_ctx lacked device/document keys.
    #[tokio::test]
    async fn validate_preview_act_acceptance_returns_pdf_bytes() {
        let (writer, readers) = build_test_db();
        let clock =
            Arc::new(SystemClock) as Arc<dyn trackly_core::primitives::clock::Clock + Send + Sync>;
        let svc = TemplateService::new(writer, readers, clock);

        // Use the embedded default act_acceptance template body.
        let (_, _, body) = DEFAULT_TEMPLATES
            .iter()
            .find(|(k, _, _)| *k == "act_acceptance")
            .expect("act_acceptance default template must exist");

        let result = svc.validate_preview(body).await;
        match result {
            Ok(bytes) => {
                assert!(!bytes.is_empty(), "PDF bytes must be non-empty");
                assert!(
                    bytes.starts_with(b"%PDF"),
                    "output must be a valid PDF (starts with %PDF)"
                );
            }
            Err(e) => panic!("validate_preview failed for act_acceptance: {e:?}"),
        }
    }

    /// R3-3 / CR-02: update_body on a kind with no active row must return
    /// AppError::NotFound instead of silently Ok(()) on 0 rows_affected.
    #[tokio::test]
    async fn update_body_unknown_kind_returns_not_found() {
        let (writer, readers) = build_test_db();
        let clock =
            Arc::new(SystemClock) as Arc<dyn trackly_core::primitives::clock::Clock + Send + Sync>;
        let svc = TemplateService::new(writer, readers, clock);
        let admin = Identity::trusted_admin();

        let result = svc
            .update_body(&admin, "nonexistent_kind", "{}".to_string())
            .await;

        match result {
            Err(AppError::NotFound { entity, .. }) => {
                assert_eq!(entity, "document_template");
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    /// GAP-S6: validate_preview with the default act_handover template must return
    /// valid PDF bytes (len > 0). Previously failed with "undefined value" because
    /// demo_ctx used flat keys instead of nested org/act objects.
    #[tokio::test]
    async fn validate_preview_returns_pdf_bytes() {
        let (writer, readers) = build_test_db();
        let clock =
            Arc::new(SystemClock) as Arc<dyn trackly_core::primitives::clock::Clock + Send + Sync>;
        let svc = TemplateService::new(writer, readers, clock);

        // Use the embedded default act_handover template body.
        let (_, _, body) = DEFAULT_TEMPLATES
            .iter()
            .find(|(k, _, _)| *k == "act_handover")
            .expect("act_handover default template must exist");

        let result = svc.validate_preview(body).await;
        match result {
            Ok(bytes) => {
                assert!(!bytes.is_empty(), "PDF bytes must be non-empty");
                // PDF magic bytes: %PDF
                assert!(
                    bytes.starts_with(b"%PDF"),
                    "output must be a valid PDF (starts with %PDF)"
                );
            }
            Err(e) => panic!("validate_preview failed: {e:?}"),
        }
    }

    /// Quick task 260704-uw3 (the bug): an existing DB whose active
    /// `act_handover` row has `is_default=1` and a stale body must be
    /// auto-upgraded in place to the current bundled body on the next
    /// `seed_defaults_on_startup` call — no manual re-seed needed.
    #[tokio::test]
    async fn seed_upgrades_stale_default_body_to_bundled_current() {
        let (writer, readers) = build_test_db();
        let clock =
            Arc::new(SystemClock) as Arc<dyn trackly_core::primitives::clock::Clock + Send + Sync>;
        let svc = TemplateService::new(writer.clone(), readers, clock);

        writer
            .execute(|conn| {
                conn.execute(
                    "INSERT INTO document_templates \
                     (kind, name, body_minijinja, is_active, is_default, version, \
                      created_at_utc, updated_at_utc) \
                     VALUES ('act_handover', 'Дефолтный шаблон акта приёма-передачи', \
                             'STALE BODY', 1, 1, 1, 0, 0)",
                    [],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .unwrap();

        svc.seed_defaults_on_startup().await.unwrap();

        let (_, _, bundled_body) = DEFAULT_TEMPLATES
            .iter()
            .find(|(k, _, _)| *k == "act_handover")
            .expect("act_handover default template must exist");

        let active_body = svc.get_active("act_handover").await.unwrap();
        assert_eq!(
            &active_body, bundled_body,
            "stale body must be replaced by the current bundled default"
        );

        let version: i64 = writer
            .execute(|conn| {
                conn.query_row(
                    "SELECT version FROM document_templates \
                     WHERE kind = 'act_handover' AND is_active = 1 AND deleted_at_utc IS NULL",
                    [],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)
            })
            .await
            .unwrap();
        assert_eq!(version, 2, "version must be bumped to 2 after the upgrade");
    }

    /// No-clobber: a user-customized template (`is_default=0`) must never be
    /// overwritten by `seed_defaults_on_startup`, regardless of how the
    /// bundled default changes.
    #[tokio::test]
    async fn seed_does_not_clobber_user_customized_body() {
        let (writer, readers) = build_test_db();
        let clock =
            Arc::new(SystemClock) as Arc<dyn trackly_core::primitives::clock::Clock + Send + Sync>;
        let svc = TemplateService::new(writer.clone(), readers, clock);

        writer
            .execute(|conn| {
                conn.execute(
                    "INSERT INTO document_templates \
                     (kind, name, body_minijinja, is_active, is_default, version, \
                      created_at_utc, updated_at_utc) \
                     VALUES ('act_handover', 'Дефолтный шаблон акта приёма-передачи', \
                             'CUSTOM USER BODY', 1, 0, 1, 0, 0)",
                    [],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .unwrap();

        svc.seed_defaults_on_startup().await.unwrap();

        let active_body = svc.get_active("act_handover").await.unwrap();
        assert_eq!(
            active_body, "CUSTOM USER BODY",
            "user-customized body must remain untouched"
        );

        let version: i64 = writer
            .execute(|conn| {
                conn.query_row(
                    "SELECT version FROM document_templates \
                     WHERE kind = 'act_handover' AND is_active = 1 AND deleted_at_utc IS NULL",
                    [],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)
            })
            .await
            .unwrap();
        assert_eq!(version, 1, "version must stay 1 — no write should fire");
    }

    /// Idempotency: once a template's stored body already equals the
    /// bundled default, repeated `seed_defaults_on_startup` calls must not
    /// bump `version` (no needless write on every startup).
    #[tokio::test]
    async fn seed_is_idempotent_when_already_current() {
        let (writer, readers) = build_test_db();
        let clock =
            Arc::new(SystemClock) as Arc<dyn trackly_core::primitives::clock::Clock + Send + Sync>;
        let svc = TemplateService::new(writer.clone(), readers, clock);

        let (_, _, bundled_body) = DEFAULT_TEMPLATES
            .iter()
            .find(|(k, _, _)| *k == "act_handover")
            .expect("act_handover default template must exist");
        let bundled_body = bundled_body.to_string();

        writer
            .execute(move |conn| {
                conn.execute(
                    "INSERT INTO document_templates \
                     (kind, name, body_minijinja, is_active, is_default, version, \
                      created_at_utc, updated_at_utc) \
                     VALUES ('act_handover', 'Дефолтный шаблон акта приёма-передачи', \
                             ?1, 1, 1, 1, 0, 0)",
                    params![bundled_body],
                )
                .map_err(map_rusqlite)?;
                Ok(())
            })
            .await
            .unwrap();

        svc.seed_defaults_on_startup().await.unwrap();
        svc.seed_defaults_on_startup().await.unwrap();

        let version: i64 = writer
            .execute(|conn| {
                conn.query_row(
                    "SELECT version FROM document_templates \
                     WHERE kind = 'act_handover' AND is_active = 1 AND deleted_at_utc IS NULL",
                    [],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)
            })
            .await
            .unwrap();
        assert_eq!(
            version, 1,
            "version must stay 1 after two calls — no write fires when body already matches"
        );
    }
}
