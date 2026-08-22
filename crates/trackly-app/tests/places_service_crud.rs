//! Интеграционные тесты `PlaceService`'s mutation half (Phase 39 Plan 05):
//! create/rename/archive/unarchive — D-20 authorization, audit_log, D-04
//! uniqueness.
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)` — защита от Linux-CI
//! deadlock (PATTERNS.md §Pattern 4), mirrors `devices_crud.rs`'s idiom.

use std::sync::Arc;
use std::time::Duration;

use trackly_app::services::PlaceService;
use trackly_core::auth::{Identity, Role};
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

/// Создаёт тестовый `PlaceService` поверх свежего tempfile DB.
fn make_service() -> (PlaceService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = PlaceService::new(writer, readers, clock);
    (svc, dir)
}

fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

fn manager_caller() -> Identity {
    Identity {
        user_id: Some(1),
        role: Role::Manager,
    }
}

fn minimal_new_building(name: &str) -> PlaceNew {
    PlaceNew {
        parent_id: None,
        kind: PlaceKind::Building,
        name: name.to_string(),
        level: None,
        is_storage: false,
        sort_order: None,
        notes: None,
    }
}

fn minimal_new_room(name: &str, parent_id: Option<i64>) -> PlaceNew {
    PlaceNew {
        parent_id,
        kind: PlaceKind::Room,
        name: name.to_string(),
        level: None,
        is_storage: false,
        sort_order: None,
        notes: None,
    }
}

// ---------------------------------------------------------------------------
// create — success + audit_log
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_inserts_place_and_audit_log() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let dto = svc
            .create(&admin, minimal_new_building("Здание А"))
            .await
            .expect("create place");

        assert!(dto.id > 0, "id должен быть > 0, получили {}", dto.id);
        assert_eq!(dto.version, 1);
        assert_eq!(dto.name, "Здание А");
        assert_eq!(dto.kind, "building");
        assert!(dto.archived_at_utc.is_none());
        assert!(dto.created_at_utc > 0);

        let readers = svc.readers.clone();
        let entity_id = dto.id;
        let count: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT COUNT(*) FROM audit_log WHERE entity_type='place' AND entity_id=?1 AND action='create'",
                rusqlite::params![entity_id],
                |r| r.get(0),
            )
        })
        .await
        .expect("join")
        .expect("query audit_log");

        assert_eq!(count, 1, "ожидали ровно одну audit_log запись action='create'");
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// create — D-20 Manager forbidden, no DB write
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_forbidden_for_manager_before_any_db_write() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let manager = manager_caller();

        let err = svc
            .create(&manager, minimal_new_building("Здание Б"))
            .await
            .expect_err("Manager должен получить Forbidden");
        assert!(matches!(err, AppError::Forbidden), "ожидали Forbidden, получили {err:?}");

        let readers = svc.readers.clone();
        let count: i64 = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row("SELECT COUNT(*) FROM places", [], |r| r.get(0))
        })
        .await
        .expect("join")
        .expect("query places");

        assert_eq!(count, 0, "Manager's forbidden create не должен был писать в places");
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// rename — D-04 duplicate sibling name → validation-shaped AppError
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rename_duplicate_sibling_name_returns_validation_not_raw_sqlite() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let parent = svc
            .create(&admin, minimal_new_building("Здание А"))
            .await
            .expect("create parent");
        let _room_214 = svc
            .create(&admin, minimal_new_room("214", Some(parent.id)))
            .await
            .expect("create room 214");
        let room_215 = svc
            .create(&admin, minimal_new_room("215", Some(parent.id)))
            .await
            .expect("create room 215");

        let err = svc
            .rename(&admin, room_215.id, "214".to_string(), room_215.version)
            .await
            .expect_err("должен быть отклонён как дубликат имени у братьев (D-04)");

        match err {
            AppError::Validation { field, message } => {
                assert_eq!(field, "name");
                assert!(
                    message.contains("214") && message.contains("Здание А"),
                    "сообщение должно содержать имя и родителя (UI-SPEC §11.2): {message}"
                );
                assert!(
                    !message.to_uppercase().contains("UNIQUE"),
                    "сообщение НЕ должно содержать сырой текст SQLite-ошибки: {message}"
                );
            }
            other => panic!("ожидали AppError::Validation, получили {other:?}"),
        }
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// archive / unarchive — D-15 (soft, reversible)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn archive_sets_and_unarchive_clears_archived_at_utc() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let place = svc
            .create(&admin, minimal_new_building("Здание В"))
            .await
            .expect("create place");

        svc.archive(&admin, place.id, place.version)
            .await
            .expect("archive");

        let readers = svc.readers.clone();
        let id = place.id;
        let archived_at: Option<i64> = tokio::task::spawn_blocking({
            let readers = readers.clone();
            move || {
                let conn = readers.acquire();
                conn.query_row(
                    "SELECT archived_at_utc FROM places WHERE id = ?1",
                    rusqlite::params![id],
                    |r| r.get(0),
                )
            }
        })
        .await
        .expect("join")
        .expect("query archived_at_utc after archive");

        assert!(archived_at.is_some(), "archived_at_utc должен быть установлен после archive");

        // archive's UPDATE bumps version 1 -> 2.
        svc.unarchive(&admin, place.id, 2)
            .await
            .expect("unarchive");

        let archived_at_after: Option<i64> = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT archived_at_utc FROM places WHERE id = ?1",
                rusqlite::params![id],
                |r| r.get(0),
            )
        })
        .await
        .expect("join")
        .expect("query archived_at_utc after unarchive");

        assert!(
            archived_at_after.is_none(),
            "archived_at_utc должен быть очищен обратно в None после unarchive"
        );
    })
    .await
    .expect("test timed out");
}
