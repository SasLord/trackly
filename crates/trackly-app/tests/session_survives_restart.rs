//! Интеграционные тесты `RusqliteSessionStore`.
//!
//! GREEN после Plan 02 Task 2.

use std::time::Duration;

use time::OffsetDateTime;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_infra::test_support::test_writer_and_readers;

fn make_store() -> (RusqliteSessionStore, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let store = RusqliteSessionStore::new(writer, readers);
    (store, dir)
}

fn make_record() -> Record {
    Record {
        id: Id::default(),
        data: Default::default(),
        expiry_date: OffsetDateTime::now_utc() + Duration::from_secs(3600),
    }
}

// ---------------------------------------------------------------------------
// session_persists_across_store_recreate (D-Session-01)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_persists_across_store_recreate() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (writer, readers, _dir) = test_writer_and_readers();
        let store1 = RusqliteSessionStore::new(writer.clone(), readers.clone());

        // Create session
        let mut record = make_record();
        let original_id = record.id;
        store1.create(&mut record).await.expect("create session");

        // Drop store and recreate from same writer/readers
        drop(store1);
        let store2 = RusqliteSessionStore::new(writer, readers);

        // Load from new store — must still find it (D-Session-01 survives restart)
        let loaded = store2.load(&original_id).await.expect("load session");
        assert!(
            loaded.is_some(),
            "session должна переживать пересоздание store"
        );
        assert_eq!(loaded.unwrap().id, original_id);
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// session_delete_removes_session
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_delete_removes_session() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (store, _dir) = make_store();

        let mut record = make_record();
        let id = record.id;
        store.create(&mut record).await.expect("create");

        // Verify it exists
        let loaded = store.load(&id).await.expect("load before delete");
        assert!(loaded.is_some(), "сессия должна существовать до удаления");

        // Delete
        store.delete(&id).await.expect("delete");

        // After delete — None
        let loaded_after = store.load(&id).await.expect("load after delete");
        assert!(loaded_after.is_none(), "сессия должна быть удалена");
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// expired_session_not_returned
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn expired_session_not_returned() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (store, _dir) = make_store();

        // Create session with expiry in the past
        let record = Record {
            id: Id::default(),
            data: Default::default(),
            // Expired 1 hour ago
            expiry_date: OffsetDateTime::now_utc() - Duration::from_secs(3600),
        };
        let id = record.id;

        // Use save() to bypass create() collision check and force expired entry
        store.save(&record).await.expect("save expired session");

        // load() must return None for expired session
        let loaded = store.load(&id).await.expect("load expired");
        assert!(loaded.is_none(), "истёкшая сессия не должна возвращаться");
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// session_save_updates_existing
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_save_updates_existing() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (store, _dir) = make_store();

        let mut record = make_record();
        let id = record.id;
        store.create(&mut record).await.expect("create");

        // Modify and save
        record.expiry_date = OffsetDateTime::now_utc() + Duration::from_secs(7200);
        store.save(&record).await.expect("save update");

        let loaded = store.load(&id).await.expect("load after save");
        assert!(loaded.is_some(), "сессия должна существовать после save");
    })
    .await
    .expect("test exceeded 30s budget");
}
