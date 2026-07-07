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
use crate::pdf::PdfRenderer;
use crate::services::organization_service::OrganizationService;

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
    /// D-13-style freeze (Phase 17): the krilla renderer handle is no longer
    /// invoked on this service's active path — `validate_preview` renders
    /// HTML via `build_safe_html_env` instead. Kept only because
    /// `TemplateService::new`'s constructor signature is used by ~10 existing
    /// call sites (context.rs, http/health.rs, tauri_cmds/health.rs, and
    /// numerous test fixtures).
    pub pdf: Arc<PdfRenderer>,
    /// Phase 17: source of `Paths` for `templates/*.html` file-first
    /// resolution used by the editor-facing methods (`list_all_for_editor`,
    /// `update_body`, `reset_to_default`, `validate_preview`). Mirrors
    /// `ActService::organization` / `ReportService::organization`. Optional
    /// so the existing 3-arg `TemplateService::new(...)` call sites
    /// (context.rs, http/health.rs, tauri_cmds/health.rs, and test fixtures)
    /// keep compiling unchanged.
    pub(crate) organization: Option<Arc<OrganizationService>>,
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
            organization: None,
        }
    }

    /// Builder: подключить `OrganizationService` (Phase 17) — источник
    /// `Paths` для `templates/*.html` file-first resolution. Mirrors
    /// `ActService::with_pdf_pipeline` / `ReportService::with_organization`.
    pub fn with_organization(mut self, organization: Arc<OrganizationService>) -> Self {
        self.organization = Some(organization);
        self
    }

    /// Resolves the `templates/` directory via `self.organization`'s
    /// `Paths`, or returns `AppError::Internal` if the service was
    /// constructed without `with_organization` (should never happen in
    /// production — `AppCtx::build` always chains it).
    fn templates_dir(&self) -> Result<std::path::PathBuf, AppError> {
        let organization = self
            .organization
            .as_ref()
            .ok_or_else(|| AppError::Internal {
                source_chain: "TemplateService::templates_dir called without with_organization"
                    .into(),
            })?;
        Ok(crate::pdf::html_templates::resolve_templates_dir(
            &organization.paths,
        ))
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

    /// Возвращает все шаблоны для редактора (SET-07).
    ///
    /// Phase 17: retargeted from the DB-backed `document_templates` table
    /// onto `templates/*.html` files — the same files the acts (Phase 16)
    /// and report (Plan 17-01) render pipelines read via
    /// `html_templates::{resolve_templates_dir, load_template,
    /// DEFAULT_HTML_TEMPLATES}`. `id` is always `0` — file-backed items have
    /// no numeric row id.
    pub async fn list_all_for_editor(&self) -> Result<Vec<TemplateEditorItem>, AppError> {
        let templates_dir = self.templates_dir()?;
        Ok(crate::pdf::html_templates::DEFAULT_HTML_TEMPLATES
            .iter()
            .map(|(filename, default_body)| {
                let kind = filename.trim_end_matches(".html").to_string();
                let body = crate::pdf::html_templates::load_template(
                    &templates_dir,
                    filename,
                    default_body,
                );
                let is_default = body == *default_body;
                TemplateEditorItem {
                    id: 0,
                    kind,
                    body,
                    is_default,
                }
            })
            .collect())
    }

    /// Обновляет тело шаблона. Требует `ManageSettings`.
    ///
    /// Валидирует синтаксис MiniJinja перед записью на диск.
    ///
    /// Phase 17: writes `templates/{kind}.html` on disk instead of
    /// `UPDATE document_templates`. `kind` is checked against the fixed
    /// `DEFAULT_HTML_TEMPLATES` allowlist BEFORE any path join (T-17-02-01 —
    /// no path-traversal surface, unrecognized `kind` never reaches
    /// `templates_dir.join(...)`).
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

        let filename = format!("{kind}.html");
        if !crate::pdf::html_templates::DEFAULT_HTML_TEMPLATES
            .iter()
            .any(|(f, _)| *f == filename)
        {
            return Err(AppError::NotFound {
                entity: "document_template",
                id: 0,
            });
        }

        let templates_dir = self.templates_dir()?;
        tokio::fs::write(templates_dir.join(&filename), body)
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("write {filename}: {e}"),
            })
    }

    /// Сбрасывает шаблон к встроенному дефолту. Требует `ManageSettings`.
    ///
    /// Phase 17: overwrites `templates/{kind}.html` with the embedded
    /// `DEFAULT_HTML_TEMPLATES` body instead of `UPDATE document_templates`.
    /// Same fixed-allowlist gate as `update_body` (T-17-02-01).
    pub async fn reset_to_default(&self, caller: &Identity, kind: &str) -> Result<(), AppError> {
        authorize(caller, &Action::ManageSettings)?;

        let filename = format!("{kind}.html");
        let default_body = crate::pdf::html_templates::DEFAULT_HTML_TEMPLATES
            .iter()
            .find(|(f, _)| *f == filename)
            .map(|(_, body)| *body)
            .ok_or(AppError::NotFound {
                entity: "default_template",
                id: 0,
            })?;

        let templates_dir = self.templates_dir()?;
        tokio::fs::write(templates_dir.join(&filename), default_body)
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("reset write {filename}: {e}"),
            })
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

    /// Validate template syntax + render an HTML preview with demo context.
    ///
    /// Used by TemplateEditor to let the user see the rendered output before
    /// saving. Phase 17: retargeted from the previous krilla round-trip
    /// (MiniJinja render to a JSON string, parsed into an intermediate spec,
    /// then rendered to PDF bytes) onto the same `build_safe_html_env` +
    /// `render_with_timeout` pipeline the acts (Phase 16) and report (Plan
    /// 17-01) render paths use — returns the rendered HTML string directly,
    /// zero krilla references in this method's body.
    ///
    /// `kind` selects the per-kind demo context (`act_handover`,
    /// `act_acceptance`, `report`) via `demo_context_for_kind` — any other
    /// value degrades gracefully to the `act_handover` context rather than
    /// erroring (preview should never crash on an unrecognized kind).
    pub async fn validate_preview(&self, kind: &str, body: &str) -> Result<String, AppError> {
        let demo_ctx = demo_context_for_kind(kind);
        crate::pdf::minijinja_env::render_with_timeout(
            &crate::pdf::minijinja_env::build_safe_html_env(),
            "_preview",
            body,
            demo_ctx,
        )
        .await
    }
}

/// Per-kind demo context for `TemplateService::validate_preview` (D-11/D-12
/// from `17-CONTEXT.md`): the editor preview must work on an empty DB, using
/// embedded sample data that covers every variable each `templates/*.html`
/// file's doc-comment references. Any `kind` not matching one of the 3 known
/// branches falls through to the `act_handover` context — preview degrades
/// gracefully rather than erroring on an unrecognized kind.
fn demo_context_for_kind(kind: &str) -> serde_json::Value {
    // Shared org block — matches org_settings requisites referenced by all
    // 3 templates' header blocks (org.name/inn/kpp/address/phone/fax/email/
    // okpo/ogrn/logo_data_uri). `logo_data_uri: null` (D-11/D-08 — replaces
    // the old krilla-era `org.logo_path` key, since act_handover.html /
    // act_acceptance.html / report.html now all expect `logo_data_uri`).
    let org = serde_json::json!({
        "name": "ООО Демо Организация",
        "inn": "7700000000",
        "kpp": "770000000",
        "address": "г. Москва, ул. Примерная, д. 1",
        "logo_data_uri": null,
        "phone": "(3919) 75-90-98",
        "fax": "(3919) 75-08-59",
        "email": "info@demo-org.ru",
        "okpo": "10176125",
        "ogrn": "1122452000714"
    });

    match kind {
        "act_acceptance" => serde_json::json!({
            "org": org,
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
        }),
        "report" => serde_json::json!({
            "org": org,
            "report_name": "Демо-отчёт: Акты приёма-передачи",
            "period_label": "Сентябрь 2026",
            "columns": ["Номер", "Устройство", "Сдал", "Принял", "Расположение"],
            "groups": [
                {
                    "month_label": "Сентябрь 2026",
                    "rows": [
                        [
                            "42",
                            "HP LaserJet Pro M404n",
                            "Иванов И.И.",
                            "Петров П.П.",
                            "Офис 101"
                        ]
                    ]
                }
            ]
        }),
        // "act_handover" and any unrecognized kind — degrade gracefully to
        // the act_handover demo context rather than erroring.
        _ => serde_json::json!({
            "org": org,
            "act": {
                "number": "42",
                "suffix": null,
                "date": "2026-06-17",
                "date_human": "17 июня 2026",
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
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::{Mutex, MutexGuard};
    use trackly_infra::{
        clock_impl::SystemClock,
        db::{pools::ReaderPool, writer_worker::WriterHandle},
    };

    /// Serializes tests that touch `TRACKLY_TEMPLATES_DIR` — `std::env` is
    /// process-global and Rust test threads run in parallel by default
    /// (mirrors the `ENV_GUARD` pattern in `pdf/html_templates.rs`). Uses
    /// `tokio::sync::Mutex` (not `std::sync::Mutex`) because these tests hold
    /// the guard across `.await` points (`clippy::await_holding_lock`).
    static ENV_GUARD: Mutex<()> = Mutex::const_new(());

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

    /// Helper: build a `TemplateService` wired with `OrganizationService`
    /// pointed at a fresh tempdir (via `TRACKLY_TEMPLATES_DIR`), mirroring
    /// production's `with_organization` wiring in `context.rs`. Returns the
    /// `ENV_GUARD` lock alongside the service/tempdir — the caller must keep
    /// the guard alive for the duration of the test (held via the returned
    /// tuple binding) so no other test thread can race-override the env var
    /// while this test's `TemplateService` still reads it. `async` (not
    /// sync) because `tokio::sync::Mutex::lock` is async-aware — this lets
    /// the returned guard be held safely across the caller's `.await` points.
    async fn build_test_svc_with_organization(
    ) -> (TemplateService, tempfile::TempDir, MutexGuard<'static, ()>) {
        let guard = ENV_GUARD.lock().await;
        let (writer, readers) = build_test_db();
        let clock =
            Arc::new(SystemClock) as Arc<dyn trackly_core::primitives::clock::Clock + Send + Sync>;
        let templates_tmp = tempfile::tempdir().unwrap();
        // SAFETY: guarded by ENV_GUARD for the duration of the test (guard
        // held via the returned tuple binding) — no other thread touches
        // TRACKLY_TEMPLATES_DIR concurrently.
        unsafe {
            std::env::set_var("TRACKLY_TEMPLATES_DIR", templates_tmp.path());
        }
        let paths = Arc::new(
            trackly_infra::paths::Paths::resolve_for_exe_dir(std::path::PathBuf::from(
                "/does/not/matter",
            ))
            .unwrap(),
        );
        let organization =
            Arc::new(crate::services::organization_service::OrganizationService::new(paths));
        let svc = TemplateService::new(writer, readers, clock).with_organization(organization);
        (svc, templates_tmp, guard)
    }

    /// Test 1 (Plan 17-02 Task 2 behavior): validate_preview("act_handover", body)
    /// where body is the current act_handover.html file content returns
    /// Ok(html) containing the act title marker — proves the demo context
    /// covers every variable the real act_handover.html template references.
    #[tokio::test]
    async fn validate_preview_act_handover_returns_html_with_title_marker() {
        let (svc, _tmp, _guard) = build_test_svc_with_organization().await;

        let body = crate::pdf::html_templates::DEFAULT_HTML_TEMPLATES
            .iter()
            .find(|(f, _)| *f == "act_handover.html")
            .map(|(_, body)| *body)
            .expect("act_handover.html must exist in DEFAULT_HTML_TEMPLATES");

        let result = svc.validate_preview("act_handover", body).await;
        match result {
            Ok(html) => {
                assert!(
                    html.contains("Акт приема-передачи"),
                    "rendered HTML must contain the act title marker; got: {html}"
                );
            }
            Err(e) => panic!("validate_preview failed for act_handover: {e:?}"),
        }
    }

    /// Test 2 (Plan 17-02 Task 2 behavior): validate_preview("report", body)
    /// where body is the report.html content from Plan 17-01 returns
    /// Ok(html) containing a non-empty month label — proves the report
    /// demo-context branch supplies report_name/period_label/columns/groups.
    #[tokio::test]
    async fn validate_preview_report_returns_html_with_month_label() {
        let (svc, _tmp, _guard) = build_test_svc_with_organization().await;

        let body = crate::pdf::html_templates::DEFAULT_HTML_TEMPLATES
            .iter()
            .find(|(f, _)| *f == "report.html")
            .map(|(_, body)| *body)
            .expect("report.html must exist in DEFAULT_HTML_TEMPLATES");

        let result = svc.validate_preview("report", body).await;
        match result {
            Ok(html) => {
                assert!(
                    html.contains("Сентябрь 2026"),
                    "rendered HTML must contain the demo month label; got: {html}"
                );
            }
            Err(e) => panic!("validate_preview failed for report: {e:?}"),
        }
    }

    /// Test 3 (Plan 17-02 Task 2 behavior): validate_preview with a body
    /// referencing an undefined variable returns Err(AppError::Validation)
    /// — Strict undefined behavior propagates as a render error, not a panic.
    #[tokio::test]
    async fn validate_preview_undefined_variable_returns_validation_error() {
        let (svc, _tmp, _guard) = build_test_svc_with_organization().await;

        let result = svc
            .validate_preview("act_handover", "{{ this_variable_does_not_exist }}")
            .await;

        match result {
            Err(AppError::Validation { .. }) => {}
            other => panic!("expected AppError::Validation, got {other:?}"),
        }
    }

    /// act_acceptance branch of demo_context_for_kind must also render
    /// without error (device/document keys present) — companion coverage
    /// to Test 1/2 above for the third known kind.
    #[tokio::test]
    async fn validate_preview_act_acceptance_returns_html() {
        let (svc, _tmp, _guard) = build_test_svc_with_organization().await;

        let body = crate::pdf::html_templates::DEFAULT_HTML_TEMPLATES
            .iter()
            .find(|(f, _)| *f == "act_acceptance.html")
            .map(|(_, body)| *body)
            .expect("act_acceptance.html must exist in DEFAULT_HTML_TEMPLATES");

        let result = svc.validate_preview("act_acceptance", body).await;
        match result {
            Ok(html) => assert!(!html.is_empty(), "rendered HTML must be non-empty"),
            Err(e) => panic!("validate_preview failed for act_acceptance: {e:?}"),
        }
    }

    /// R3-3 / CR-02 (Phase 17 retarget): update_body on a kind not in the
    /// fixed DEFAULT_HTML_TEMPLATES allowlist must return AppError::NotFound
    /// — T-17-02-01 mitigation, unrecognized kind never reaches the
    /// templates_dir path join.
    #[tokio::test]
    async fn update_body_unknown_kind_returns_not_found() {
        let (svc, _tmp, _guard) = build_test_svc_with_organization().await;
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

    /// list_all_for_editor reads the 3 known kinds from disk (file-first +
    /// embedded fallback) instead of the DB — Phase 17 retarget.
    #[tokio::test]
    async fn list_all_for_editor_returns_all_known_kinds_from_files() {
        let (svc, _tmp, _guard) = build_test_svc_with_organization().await;

        let items = svc.list_all_for_editor().await.unwrap();
        assert_eq!(items.len(), 3, "must return exactly 3 known kinds");
        let kinds: Vec<&str> = items.iter().map(|i| i.kind.as_str()).collect();
        assert!(kinds.contains(&"act_handover"));
        assert!(kinds.contains(&"act_acceptance"));
        assert!(kinds.contains(&"report"));
        // No templates written to the fresh tempdir yet — every item must
        // report is_default = true (body equals the embedded default).
        assert!(items.iter().all(|i| i.is_default));
    }

    /// update_body writes to templates/{kind}.html on disk (not the DB) —
    /// verified via a subsequent list_all_for_editor read showing the new
    /// body and is_default = false.
    #[tokio::test]
    async fn update_body_writes_file_and_list_reflects_it() {
        let (svc, _tmp, _guard) = build_test_svc_with_organization().await;
        let admin = Identity::trusted_admin();

        svc.update_body(&admin, "act_handover", "CUSTOM BODY".to_string())
            .await
            .unwrap();

        let items = svc.list_all_for_editor().await.unwrap();
        let item = items
            .iter()
            .find(|i| i.kind == "act_handover")
            .expect("act_handover item must exist");
        assert_eq!(item.body, "CUSTOM BODY");
        assert!(!item.is_default);
    }

    /// reset_to_default overwrites templates/{kind}.html with the embedded
    /// default body — verified via a subsequent list_all_for_editor read.
    #[tokio::test]
    async fn reset_to_default_restores_embedded_default() {
        let (svc, _tmp, _guard) = build_test_svc_with_organization().await;
        let admin = Identity::trusted_admin();

        svc.update_body(&admin, "act_handover", "CUSTOM BODY".to_string())
            .await
            .unwrap();
        svc.reset_to_default(&admin, "act_handover").await.unwrap();

        let items = svc.list_all_for_editor().await.unwrap();
        let item = items
            .iter()
            .find(|i| i.kind == "act_handover")
            .expect("act_handover item must exist");
        assert!(item.is_default);
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
