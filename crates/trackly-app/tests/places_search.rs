//! Интеграционные тесты `PlaceService::search` (Phase 39 Plan 08, Task 2):
//! Cyrillic-safe full-path substring search — highest-value test in this
//! phase per VALIDATION.md (proves Rust-side `.to_lowercase()` matching works
//! where SQL `LIKE` would silently fail on Cyrillic).
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)` — защита от Linux-CI
//! deadlock (PATTERNS.md §Pattern 4). Все имена мест вымышленные (приватность
//! репозитория).

use std::sync::Arc;
use std::time::Duration;

use trackly_app::services::PlaceService;
use trackly_core::auth::Identity;
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

fn make_service() -> (PlaceService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = PlaceService::new(writer, readers, clock);
    (svc, dir)
}

fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

fn new_place(kind: PlaceKind, name: &str, parent_id: Option<i64>) -> PlaceNew {
    PlaceNew {
        parent_id,
        kind,
        name: name.to_string(),
        level: None,
        is_storage: false,
        sort_order: None,
        notes: None,
    }
}

// ---------------------------------------------------------------------------
// Cyrillic case-fold — the highest-value test in this phase (VALIDATION.md)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_lowercase_cyrillic_query_matches_uppercase_first_letter() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let building = svc
            .create(&admin, new_place(PlaceKind::Building, "Здание А", None))
            .await
            .expect("create building");
        svc.create(&admin, new_place(PlaceKind::Floor, "2 этаж", Some(building.id)))
            .await
            .expect("create nested floor");

        // Lowercase query "здание" must match "Здание А" — proves the search
        // is done in Rust via .to_lowercase(), not SQL LIKE (which case-folds
        // ASCII only and would silently miss this Cyrillic match).
        let results = svc.search(&admin, "здание".to_string()).await.expect("search");

        assert!(
            results.iter().any(|r| r.full_path.contains("Здание А")),
            "поиск «здание» (нижний регистр) должен найти «Здание А» \
             (кириллический case-fold через Rust .to_lowercase(), не SQL LIKE): {results:?}"
        );
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// No match — empty vec, not an error
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_non_matching_query_returns_empty_vec_not_error() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        svc.create(&admin, new_place(PlaceKind::Building, "Здание А", None))
            .await
            .expect("create building");

        let results = svc
            .search(&admin, "несуществующий-запрос-ёжик".to_string())
            .await
            .expect("search should not error on no match");

        assert!(results.is_empty(), "ожидали пустой результат, получили: {results:?}");
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// Query length validation — > 100 chars rejected
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_rejects_query_longer_than_100_chars() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let too_long = "а".repeat(101);
        let err = svc
            .search(&admin, too_long)
            .await
            .expect_err("запрос длиннее 100 символов должен быть отклонён");

        match err {
            AppError::Validation { field, .. } => assert_eq!(field, "query"),
            other => panic!("ожидали AppError::Validation, получили {other:?}"),
        }
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// Result cap — 50 rows even when more than 50 places match
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_caps_results_at_50_rows() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        for i in 0..60 {
            svc.create(
                &admin,
                new_place(PlaceKind::Territory, &format!("Полигон Тест {i}"), None),
            )
            .await
            .unwrap_or_else(|e| panic!("create place {i}: {e:?}"));
        }

        let results = svc.search(&admin, "полигон тест".to_string()).await.expect("search");

        assert_eq!(
            results.len(),
            50,
            "ожидали ровно 50 результатов (лимит), получили {}",
            results.len()
        );
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// Archived place excluded (D-15)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_excludes_archived_place() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let place = svc
            .create(&admin, new_place(PlaceKind::Territory, "Архивный полигон", None))
            .await
            .expect("create place");

        svc.archive(&admin, place.id, place.version)
            .await
            .expect("archive place");

        let results = svc
            .search(&admin, "архивный полигон".to_string())
            .await
            .expect("search");

        assert!(
            results.is_empty(),
            "архивное место не должно попадать в результаты поиска (D-15): {results:?}"
        );
    })
    .await
    .expect("test timed out");
}
