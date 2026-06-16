// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Document template editing integration test scaffold — Phase 7 Plan 01 (RED).
//!
//! Covers SET-07 (шаблоны документов: Акт, Документ приёма):
//!   - Template body update (Jinja2/minijinja source stored in app_settings or dedicated table)
//!   - Validation: template must be parseable by minijinja (no syntax errors)
//!   - reset_to_default restores the bundled template from the binary
//!
//! Implemented in plan 07 (TemplateService::update / reset_to_default).

use std::time::Duration;

/// Verify that a template update stores the new body and subsequent render uses it.
///
/// RED: TemplateService does not exist yet — todo!() causes runtime failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn template_update_and_render_uses_new_body() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 07")
    })
    .await
    .expect("template_update_and_render_uses_new_body budget")
}

/// Verify that a template with Jinja2 syntax errors is rejected with a validation error.
///
/// RED: TemplateService does not exist yet — todo!() causes runtime failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn template_invalid_syntax_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 07")
    })
    .await
    .expect("template_invalid_syntax_rejected budget")
}

/// Verify that reset_to_default restores the bundled template, discarding user edits.
///
/// RED: TemplateService does not exist yet — todo!() causes runtime failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn template_reset_to_default_restores_builtin() {
    tokio::time::timeout(Duration::from_secs(30), async {
        todo!("implement in plan 07")
    })
    .await
    .expect("template_reset_to_default_restores_builtin budget")
}
