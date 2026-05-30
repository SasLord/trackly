//! Templates seed integration tests — Phase 3 Plan 04 Task 1.
//!
//! Covers (per behavior list):
//!   - default_seeded_on_first_startup
//!   - seed_is_idempotent
//!   - seed_restores_after_full_soft_delete
//!   - acceptance_seeded_and_used

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::services::TemplateService;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;

fn make_template_service() -> (TemplateService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = TemplateService::new(writer, readers, clock);
    (svc, dir)
}

async fn count_active(svc: &TemplateService, kind: &str) -> i64 {
    let readers = svc.readers.clone();
    let kind_owned = kind.to_string();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT COUNT(*) FROM document_templates \
             WHERE kind = ?1 AND is_active = 1 AND deleted_at_utc IS NULL",
            params![kind_owned],
            |r| r.get::<_, i64>(0),
        )
        .expect("count")
    })
    .await
    .expect("join")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn default_seeded_on_first_startup() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_template_service();
        svc.seed_defaults_on_startup().await.expect("seed");
        assert_eq!(count_active(&svc, "act_handover").await, 1);
        assert_eq!(count_active(&svc, "act_acceptance").await, 1);
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seed_is_idempotent() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_template_service();
        svc.seed_defaults_on_startup().await.expect("seed 1");
        svc.seed_defaults_on_startup().await.expect("seed 2");
        svc.seed_defaults_on_startup().await.expect("seed 3");
        assert_eq!(count_active(&svc, "act_handover").await, 1);
        assert_eq!(count_active(&svc, "act_acceptance").await, 1);
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn seed_restores_after_full_soft_delete() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_template_service();
        svc.seed_defaults_on_startup().await.expect("seed 1");
        // Soft-delete handover.
        svc.writer
            .execute(|conn| {
                conn.execute(
                    "UPDATE document_templates SET deleted_at_utc = ?1, is_active = 0 \
                     WHERE kind = 'act_handover'",
                    params![1_900_000_000_i64],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
            .expect("soft delete");
        assert_eq!(count_active(&svc, "act_handover").await, 0);

        // Re-seed → новая активная запись создаётся.
        svc.seed_defaults_on_startup().await.expect("re-seed");
        assert_eq!(count_active(&svc, "act_handover").await, 1);
        // acceptance не пострадал.
        assert_eq!(count_active(&svc, "act_acceptance").await, 1);
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn acceptance_seeded_and_used() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_template_service();
        svc.seed_defaults_on_startup().await.expect("seed");
        let body = svc.get_active("act_acceptance").await.expect("get_active");
        assert!(
            body.contains("Документ приёма") || body.contains("Кто передал"),
            "Шаблон act_acceptance должен содержать русские маркеры. Body head: {:?}",
            body.chars().take(200).collect::<String>()
        );
    })
    .await
    .expect("timeout");
}
