// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Document template editing integration tests — Phase 7 Plan 02 (GREEN),
//! retargeted onto file-backed `templates/*.html` I/O by Phase 17 Plan 02,
//! rewritten for the file-backed editor contract by Phase 17 Plan 04.
//!
//! Covers SET-07 (шаблоны документов: Акт, Документ приёма):
//!   - Template body update writes directly to `templates/{kind}.html`
//!   - `list_all_for_editor` reflects on-disk file state
//!   - `reset_to_default` restores the embedded default on disk
//!   - Validation: template must be parseable by minijinja (no syntax errors),
//!     and a rejected update must leave the on-disk file untouched
//!   - Unknown `kind` returns `NotFound` before any path is ever touched
//!
//! Phase 17 note: `list_all_for_editor`/`update_body`/`reset_to_default` no
//! longer touch the DB-backed `document_templates` table (frozen, D-13) —
//! they read/write `templates/*.html` files via `OrganizationService`'s
//! `Paths`. The DB-backed seed/read path remains untouched and is
//! intentionally NOT exercised here as an editor-state proxy anymore — these
//! tests assert file-backed state exclusively via direct filesystem reads
//! (`std::fs::read_to_string`) and `list_all_for_editor`.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, MutexGuard};
use trackly_app::pdf::html_templates::DEFAULT_HTML_TEMPLATES;
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
/// `context.rs` wiring. Returns the on-disk templates dir alongside the
/// service fixtures so tests can assert directly against
/// `templates_dir.join(...)` file contents. Returns the `ENV_GUARD` lock
/// alongside the service fixtures — the caller must keep the guard alive for
/// the duration of the test so no other test thread can race-override the
/// env var. `async` (not sync) because `tokio::sync::Mutex::lock` is
/// async-aware.
async fn make_template_service() -> (
    TemplateService,
    tempfile::TempDir,
    std::path::PathBuf,
    MutexGuard<'static, ()>,
) {
    let guard = ENV_GUARD.lock().await;
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let templates_dir = tempfile::tempdir().expect("templates tempdir");
    let templates_dir_path = templates_dir.path().to_path_buf();
    // SAFETY: guarded by ENV_GUARD for the duration of the test (guard held
    // via the returned tuple binding) — no other thread touches
    // TRACKLY_TEMPLATES_DIR concurrently.
    unsafe {
        std::env::set_var("TRACKLY_TEMPLATES_DIR", &templates_dir_path);
    }
    let paths = Arc::new(
        trackly_infra::paths::Paths::resolve_for_exe_dir(std::path::PathBuf::from(
            "/does/not/matter",
        ))
        .expect("resolve_for_exe_dir"),
    );
    let organization = Arc::new(OrganizationService::new(paths));
    let svc = TemplateService::new(writer, readers, clock).with_organization(organization);
    // Leak templates_dir's TempDir handle into `dir`'s return slot alongside
    // it — both `dir` (writer/readers tempdir) and `templates_dir` (the
    // on-disk templates tempdir) must outlive the test body. `dir` is
    // returned for that purpose; `templates_dir` itself is intentionally
    // NOT dropped here (its TempDir guard is kept alive by leaking it,
    // since only its path is needed by callers).
    std::mem::forget(templates_dir);
    (svc, dir, templates_dir_path, guard)
}

fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

fn embedded_default(kind: &str) -> &'static str {
    let filename = format!("{kind}.html");
    DEFAULT_HTML_TEMPLATES
        .iter()
        .find(|(f, _)| *f == filename)
        .map(|(_, body)| *body)
        .expect("kind must be a known DEFAULT_HTML_TEMPLATES entry")
}

/// Verify that `update_body` writes the new body directly to
/// `templates/act_handover.html` on disk — asserted via `std::fs::read_to_string`,
/// not the now-decoupled, frozen DB read path.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_body_writes_file_to_disk() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir, templates_dir, _guard) = make_template_service().await;
        let caller = admin_caller();

        let new_body = "Акт приёма-передачи: {{ act.number }} — КАСТОМНЫЙ ШАБЛОН".to_string();
        svc.update_body(&caller, "act_handover", new_body.clone())
            .await
            .expect("update_body");

        let on_disk = std::fs::read_to_string(templates_dir.join("act_handover.html"))
            .expect("act_handover.html must exist on disk after update_body");
        assert_eq!(
            on_disk, new_body,
            "on-disk file must equal the body passed to update_body"
        );
    })
    .await
    .expect("update_body_writes_file_to_disk budget")
}

/// Verify that `list_all_for_editor` reflects a body written directly to
/// disk (outside of `update_body`) — proves the read path is genuinely
/// file-backed, not cached or DB-shadowed.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_all_for_editor_reflects_disk_state() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir, templates_dir, _guard) = make_template_service().await;

        let known_body = "<html>ПРЯМАЯ ЗАПИСЬ НА ДИСК — report.html</html>".to_string();
        std::fs::write(templates_dir.join("report.html"), &known_body)
            .expect("direct write to report.html");

        let items = svc
            .list_all_for_editor()
            .await
            .expect("list_all_for_editor");
        let report = items
            .iter()
            .find(|i| i.kind == "report")
            .expect("report must be in list");

        assert_eq!(
            report.body, known_body,
            "list_all_for_editor must reflect the body written directly to disk"
        );
        assert!(
            !report.is_default,
            "is_default must be false — on-disk body differs from the embedded default"
        );
    })
    .await
    .expect("list_all_for_editor_reflects_disk_state budget")
}

/// Verify that `reset_to_default` restores the on-disk file to the embedded
/// default body from `DEFAULT_HTML_TEMPLATES`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reset_to_default_restores_embedded_body() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir, templates_dir, _guard) = make_template_service().await;
        let caller = admin_caller();

        let custom_body = "КАСТОМНОЕ ТЕЛО ШАБЛОНА — ПОЛНОСТЬЮ ИЗМЕНЕНО".to_string();
        svc.update_body(&caller, "act_acceptance", custom_body.clone())
            .await
            .expect("update_body");

        let changed_on_disk = std::fs::read_to_string(templates_dir.join("act_acceptance.html"))
            .expect("act_acceptance.html must exist after update_body");
        assert_eq!(changed_on_disk, custom_body, "sanity: update_body applied");

        svc.reset_to_default(&caller, "act_acceptance")
            .await
            .expect("reset_to_default");

        let restored = std::fs::read_to_string(templates_dir.join("act_acceptance.html"))
            .expect("act_acceptance.html must exist after reset_to_default");
        assert_eq!(
            restored,
            embedded_default("act_acceptance"),
            "on-disk file must equal the embedded default after reset_to_default"
        );
    })
    .await
    .expect("reset_to_default_restores_embedded_body budget")
}

/// Verify that an unrecognized `kind` string is rejected with `NotFound`
/// before any `templates_dir.join(...)` path is ever constructed — the
/// allowlist-check contract from Plan 17-02 Task 1 must still hold.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_body_unknown_kind_returns_not_found() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir, _templates_dir, _guard) = make_template_service().await;
        let caller = admin_caller();

        let result = svc
            .update_body(&caller, "nonexistent_kind", "{}".to_string())
            .await;

        match result {
            Err(trackly_core::error::AppError::NotFound { .. }) => {}
            other => panic!("ожидали NotFound, получили: {other:?}"),
        }
    })
    .await
    .expect("update_body_unknown_kind_returns_not_found budget")
}

/// Verify that a template with MiniJinja syntax errors is rejected with a
/// validation error, AND that the on-disk file is left completely untouched
/// (read before/after, assert unchanged) — replaces the old "DB row
/// unchanged" assertion with a "file unchanged" assertion.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_body_rejects_invalid_minijinja_syntax() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir, templates_dir, _guard) = make_template_service().await;
        let caller = admin_caller();

        let file_path = templates_dir.join("act_handover.html");
        let before = std::fs::read_to_string(&file_path);

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

        let after = std::fs::read_to_string(&file_path);
        assert_eq!(
            before.is_ok(),
            after.is_ok(),
            "file existence must not change (still absent, or still present)"
        );
        assert!(
            !after.as_deref().unwrap_or_default().contains("незакрытый"),
            "invalid body must never have been written to disk"
        );
        if let (Ok(before), Ok(after)) = (before, after) {
            assert_eq!(
                before, after,
                "on-disk file must remain unchanged after a rejected update_body call"
            );
        }
    })
    .await
    .expect("update_body_rejects_invalid_minijinja_syntax budget")
}

/// WR-01 gap-closure (Plan 17-06): a template that is syntactically valid
/// MiniJinja but references a top-level variable that does not exist in
/// `demo_context_for_kind` must be rejected on save — the old bare-env
/// validation (no `UndefinedBehavior::Strict`) let this through and it only
/// failed at real render/print time. Asserts the same "field=body" contract
/// and "file unchanged" invariant as `update_body_rejects_invalid_minijinja_syntax`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn update_body_rejects_undefined_top_level_variable() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir, templates_dir, _guard) = make_template_service().await;
        let caller = admin_caller();

        let file_path = templates_dir.join("act_handover.html");
        let before = std::fs::read_to_string(&file_path);

        let undefined_var_body = "{{ totally_undefined_marker }}".to_string();
        let result = svc
            .update_body(&caller, "act_handover", undefined_var_body)
            .await;

        assert!(
            result.is_err(),
            "шаблон с необъявленной переменной должен быть отклонён"
        );
        match result {
            Err(trackly_core::error::AppError::Validation { field, .. }) => {
                assert_eq!(field, "body", "field должен быть 'body'");
            }
            other => panic!("ожидали Validation, получили: {other:?}"),
        }

        let after = std::fs::read_to_string(&file_path);
        assert_eq!(
            before.is_ok(),
            after.is_ok(),
            "file existence must not change (still absent, or still present)"
        );
        if let (Ok(before), Ok(after)) = (before, after) {
            assert_eq!(
                before, after,
                "on-disk file must remain unchanged after a rejected update_body call"
            );
        }
    })
    .await
    .expect("update_body_rejects_undefined_top_level_variable budget")
}
