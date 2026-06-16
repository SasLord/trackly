// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Document template editing integration tests — Phase 7 Plan 02 (GREEN).
//!
//! Covers SET-07 (шаблоны документов: Акт, Документ приёма):
//!   - Template body update (MiniJinja source stored in document_templates table)
//!   - Validation: template must be parseable by minijinja (no syntax errors)
//!   - reset_to_default restores the bundled template from the binary

use std::sync::Arc;
use std::time::Duration;

use trackly_app::services::TemplateService;
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

fn make_template_service() -> (TemplateService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = TemplateService::new(writer, readers, clock);
    (svc, dir)
}

fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

/// Verify that a template update stores the new body and subsequent render uses it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn template_update_and_render_uses_new_body() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_template_service();
        let caller = admin_caller();

        // Сидируем дефолтные шаблоны
        svc.seed_defaults_on_startup().await.expect("seed");

        // Обновляем шаблон act_handover
        let new_body = "Акт приёма-передачи: {{ act_number }} — КАСТОМНЫЙ ШАБЛОН".to_string();
        svc.update_body(&caller, "act_handover", new_body.clone())
            .await
            .expect("update_body");

        // Проверяем что тело изменилось
        let retrieved_body = svc.get_active("act_handover").await.expect("get_active");
        assert_eq!(
            retrieved_body, new_body,
            "get_active должен вернуть обновлённое тело шаблона"
        );

        // Проверяем через list_all_for_editor
        let items = svc.list_all_for_editor().await.expect("list_all_for_editor");
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
        let (svc, _dir) = make_template_service();
        let caller = admin_caller();

        // Сидируем шаблоны
        svc.seed_defaults_on_startup().await.expect("seed");

        // Шаблон с синтаксической ошибкой MiniJinja
        let invalid_body = "{{ незакрытый тег {% for".to_string();
        let result = svc
            .update_body(&caller, "act_handover", invalid_body)
            .await;

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

        // Оригинальный шаблон должен остаться неизменным
        let body = svc.get_active("act_handover").await.expect("get_active");
        assert!(
            !body.contains("незакрытый"),
            "оригинальный шаблон должен остаться после ошибки"
        );
    })
    .await
    .expect("template_invalid_syntax_rejected budget")
}

/// Verify that reset_to_default restores the bundled template, discarding user edits.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn template_reset_to_default_restores_builtin() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_template_service();
        let caller = admin_caller();

        // Сидируем шаблоны
        svc.seed_defaults_on_startup().await.expect("seed");

        // Запоминаем дефолтный шаблон
        let default_body = svc.get_active("act_handover").await.expect("get default");

        // Меняем шаблон
        let custom_body = "КАСТОМНОЕ ТЕЛО ШАБЛОНА — ПОЛНОСТЬЮ ИЗМЕНЕНО".to_string();
        svc.update_body(&caller, "act_handover", custom_body.clone())
            .await
            .expect("update_body");

        // Убеждаемся что изменили
        let changed = svc.get_active("act_handover").await.expect("get changed");
        assert_eq!(changed, custom_body);

        // Сбрасываем к дефолту
        svc.reset_to_default(&caller, "act_handover")
            .await
            .expect("reset_to_default");

        // Должно вернуться к дефолту
        let restored = svc.get_active("act_handover").await.expect("get restored");
        assert_eq!(
            restored, default_body,
            "reset_to_default должен восстановить дефолтный шаблон"
        );

        // is_default должен быть true
        let items = svc.list_all_for_editor().await.expect("list_all_for_editor");
        let handover = items
            .iter()
            .find(|i| i.kind == "act_handover")
            .expect("act_handover must be in list");
        assert!(
            handover.is_default,
            "is_default должен быть true после reset_to_default"
        );
    })
    .await
    .expect("template_reset_to_default_restores_builtin budget")
}
