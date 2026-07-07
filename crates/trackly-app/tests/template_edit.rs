// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Document template editing integration tests — Phase 7 Plan 02 (GREEN),
//! retargeted onto file-backed `templates/*.html` I/O by Phase 17 Plan 02.
//!
//! Covers SET-07 (шаблоны документов: Акт, Документ приёма):
//!   - Template body update (MiniJinja/HTML source stored in templates/*.html)
//!   - Validation: template must be parseable by minijinja (no syntax errors)
//!   - reset_to_default restores the bundled template from the binary
//!
//! Phase 17 note: `list_all_for_editor`/`update_body`/`reset_to_default` no
//! longer touch the DB-backed `document_templates` table (frozen, D-13) —
//! they read/write `templates/*.html` files via `OrganizationService`'s
//! `Paths`. `seed_defaults_on_startup`/`get_active` remain DB-backed and are
//! intentionally NOT exercised here as an editor-state proxy anymore; these
//! tests assert file-backed state exclusively via `list_all_for_editor`.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, MutexGuard};
use trackly_app::services::organization_service::OrganizationService;
use trackly_app::services::TemplateService;
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

/// Serializes tests that touch `TRACKLY_TEMPLATES_DIR` — `std::env` is
/// process-global and this integration test binary runs its `#[tokio::test]`
/// functions in parallel by default (mirrors the `ENV_GUARD` pattern in
/// `pdf/html_templates.rs` / `services/template_service.rs`). Uses
/// `tokio::sync::Mutex` (not `std::sync::Mutex`) because these tests hold
/// the guard across `.await` points (`clippy::await_holding_lock`).
static ENV_GUARD: Mutex<()> = Mutex::const_new(());

/// Phase 17: `list_all_for_editor`/`update_body`/`reset_to_default` now read
/// and write `templates/*.html` files via `OrganizationService`'s `Paths`
/// (file-first + embedded fallback) instead of the DB-backed
/// `document_templates` table — wire `with_organization` pointed at a fresh
/// tempdir via `TRACKLY_TEMPLATES_DIR`, mirroring production's
/// `context.rs` wiring. Returns the `ENV_GUARD` lock alongside the service
/// fixtures — the caller must keep the guard alive for the duration of the
/// test so no other test thread can race-override the env var. `async` (not
/// sync) because `tokio::sync::Mutex::lock` is async-aware.
async fn make_template_service() -> (
    TemplateService,
    tempfile::TempDir,
    tempfile::TempDir,
    MutexGuard<'static, ()>,
) {
    let guard = ENV_GUARD.lock().await;
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let templates_dir = tempfile::tempdir().expect("templates tempdir");
    // SAFETY: guarded by ENV_GUARD for the duration of the test (guard held
    // via the returned tuple binding) — no other thread touches
    // TRACKLY_TEMPLATES_DIR concurrently.
    unsafe {
        std::env::set_var("TRACKLY_TEMPLATES_DIR", templates_dir.path());
    }
    let paths = Arc::new(
        trackly_infra::paths::Paths::resolve_for_exe_dir(std::path::PathBuf::from(
            "/does/not/matter",
        ))
        .expect("resolve_for_exe_dir"),
    );
    let organization = Arc::new(OrganizationService::new(paths));
    let svc = TemplateService::new(writer, readers, clock).with_organization(organization);
    (svc, dir, templates_dir, guard)
}

fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

/// Verify that update_body writes the new body to templates/act_handover.html
/// and list_all_for_editor reflects it (body + is_default=false).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn template_update_and_render_uses_new_body() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir, _templates_dir, _guard) = make_template_service().await;
        let caller = admin_caller();

        // Обновляем шаблон act_handover
        let new_body = "Акт приёма-передачи: {{ act_number }} — КАСТОМНЫЙ ШАБЛОН".to_string();
        svc.update_body(&caller, "act_handover", new_body.clone())
            .await
            .expect("update_body");

        // Проверяем через list_all_for_editor (file-backed read)
        let items = svc
            .list_all_for_editor()
            .await
            .expect("list_all_for_editor");
        let handover = items
            .iter()
            .find(|i| i.kind == "act_handover")
            .expect("act_handover must be in list");
        assert_eq!(handover.body, new_body);
        assert!(
            !handover.is_default,
            "is_default должен быть false после update_body"
        );
    })
    .await
    .expect("template_update_and_render_uses_new_body budget")
}

/// Verify that a template with Jinja2 syntax errors is rejected with a validation error.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn template_invalid_syntax_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir, _templates_dir, _guard) = make_template_service().await;
        let caller = admin_caller();

        // Шаблон с синтаксической ошибкой MiniJinja
        let invalid_body = "{{ незакрытый тег {% for".to_string();
        let result = svc.update_body(&caller, "act_handover", invalid_body).await;

        assert!(
            result.is_err(),
            "шаблон с синтаксической ошибкой должен быть отклонён"
        );
        match result {
            Err(trackly_core::error::AppError::Validation { field, .. }) => {
                assert_eq!(field, "body", "field должен быть 'body'");
            }
            other => panic!("ожидали Validation, получили: {other:?}"),
        }

        // Оригинальный шаблон должен остаться неизменным (embedded default,
        // never written since update_body rejected before any file write).
        let items = svc
            .list_all_for_editor()
            .await
            .expect("list_all_for_editor");
        let handover = items
            .iter()
            .find(|i| i.kind == "act_handover")
            .expect("act_handover must be in list");
        assert!(
            !handover.body.contains("незакрытый"),
            "оригинальный шаблон должен остаться после ошибки"
        );
        assert!(
            handover.is_default,
            "is_default должен остаться true — write never happened"
        );
    })
    .await
    .expect("template_invalid_syntax_rejected budget")
}

/// Verify that reset_to_default restores the bundled template, discarding user edits.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn template_reset_to_default_restores_builtin() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir, _templates_dir, _guard) = make_template_service().await;
        let caller = admin_caller();

        // Запоминаем дефолтный шаблон (embedded default from
        // DEFAULT_HTML_TEMPLATES, via a fresh list_all_for_editor read).
        let default_body = svc
            .list_all_for_editor()
            .await
            .expect("list_all_for_editor")
            .into_iter()
            .find(|i| i.kind == "act_handover")
            .expect("act_handover must be in list")
            .body;

        // Меняем шаблон
        let custom_body = "КАСТОМНОЕ ТЕЛО ШАБЛОНА — ПОЛНОСТЬЮ ИЗМЕНЕНО".to_string();
        svc.update_body(&caller, "act_handover", custom_body.clone())
            .await
            .expect("update_body");

        // Убеждаемся что изменили
        let changed = svc
            .list_all_for_editor()
            .await
            .expect("list_all_for_editor")
            .into_iter()
            .find(|i| i.kind == "act_handover")
            .expect("act_handover must be in list")
            .body;
        assert_eq!(changed, custom_body);

        // Сбрасываем к дефолту
        svc.reset_to_default(&caller, "act_handover")
            .await
            .expect("reset_to_default");

        // Должно вернуться к дефолту
        let items = svc
            .list_all_for_editor()
            .await
            .expect("list_all_for_editor");
        let handover = items
            .iter()
            .find(|i| i.kind == "act_handover")
            .expect("act_handover must be in list");
        assert_eq!(
            handover.body, default_body,
            "reset_to_default должен восстановить дефолтный шаблон"
        );
        assert!(
            handover.is_default,
            "is_default должен быть true после reset_to_default"
        );
    })
    .await
    .expect("template_reset_to_default_restores_builtin budget")
}
