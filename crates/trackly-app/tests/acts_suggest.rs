//! Acts suggest_person integration tests — Phase 3.1 Plan 02 (G-5).
//!
//! Separated from `acts_search.rs` per W-3: suggest_person — отдельная
//! feature (autocomplete для giver/receiver, не общий поиск по актам).
//!
//! tokio timeout 30s каждый.

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto};
use trackly_app::dto::suggest::SuggestPersonField;
use trackly_app::services::ActService;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;

fn make_acts_service() -> (ActService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = ActService::new(writer, readers, clock);
    (svc, dir)
}

async fn seed_device(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    name: &str,
) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            tx.execute(
                "INSERT INTO devices \
                 (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (1, ?1, 1, 1, ?2, ?2)",
                params![name, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            let id = tx.last_insert_rowid();
            tx.commit().map_err(map_rusqlite)?;
            Ok(id)
        })
        .await
        .expect("seed device")
}

/// Создаёт handover-акт с заданными giver/receiver. Каждый акт занимает
/// свой device (по одному device на акт — frequency группируется по имени).
async fn make_handover_with_giver_receiver(
    svc: &ActService,
    giver: &str,
    receiver: &str,
    device_id: i64,
) {
    svc.create(ActCreateDto {
        number_override: None,
        giver_name: giver.into(),
        receiver_name: receiver.into(),
        location_id: None,
        location_name: None,
        notes: None,
        deadline_utc: None,
        handover_date_utc: None,
        items: vec![ActItemNewDto {
            device_id,
            device_ids: Vec::new(),
            quantity: 1,
        }],
    })
    .await
    .expect("create handover");
}

// ---------------------------------------------------------------------------
// Test 1: empty prefix → frequency DESC ordering.
// Иванов x3, Сидоров x2, Петров x1 → ['Иванов', 'Сидоров', 'Петров'].
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_empty_prefix_orders_by_frequency() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        // 6 devices (один на акт).
        let mut dids = Vec::new();
        for i in 0..6 {
            dids.push(seed_device(&svc.writer, &format!("D{i}")).await);
        }
        // Иванов x3
        make_handover_with_giver_receiver(&svc, "Иванов И.И.", "Anyone", dids[0]).await;
        make_handover_with_giver_receiver(&svc, "Иванов И.И.", "Anyone", dids[1]).await;
        make_handover_with_giver_receiver(&svc, "Иванов И.И.", "Anyone", dids[2]).await;
        // Сидоров x2
        make_handover_with_giver_receiver(&svc, "Сидоров С.С.", "Anyone", dids[3]).await;
        make_handover_with_giver_receiver(&svc, "Сидоров С.С.", "Anyone", dids[4]).await;
        // Петров x1
        make_handover_with_giver_receiver(&svc, "Петров П.П.", "Anyone", dids[5]).await;

        let result = svc
            .suggest_person(SuggestPersonField::Giver, "", 20)
            .await
            .expect("suggest_person");
        assert_eq!(
            result,
            vec![
                "Иванов И.И.".to_string(),
                "Сидоров С.С.".to_string(),
                "Петров П.П.".to_string(),
            ]
        );
    })
    .await
    .expect("budget");
}

// ---------------------------------------------------------------------------
// Test 2: prefix match.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_prefix_match() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let d1 = seed_device(&svc.writer, "DA").await;
        let d2 = seed_device(&svc.writer, "DB").await;
        let d3 = seed_device(&svc.writer, "DC").await;
        make_handover_with_giver_receiver(&svc, "Иванов И.И.", "X", d1).await;
        make_handover_with_giver_receiver(&svc, "Иваненко", "X", d2).await;
        make_handover_with_giver_receiver(&svc, "Петров", "X", d3).await;

        let result = svc
            .suggest_person(SuggestPersonField::Giver, "Ив", 20)
            .await
            .expect("suggest_person");
        // Both Иванов и Иваненко начинаются с 'Ив' — каждый freq=1 → alpha sort.
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"Иванов И.И.".to_string()));
        assert!(result.contains(&"Иваненко".to_string()));
        assert!(!result.contains(&"Петров".to_string()));
    })
    .await
    .expect("budget");
}

// ---------------------------------------------------------------------------
// Test 3: escape_like защищает от LIKE injection через `%`.
// Префикс `%adm` не должен «расширяться» в wildcard и выдать чужие имена.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_escape_like_blocks_percent_injection() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let d1 = seed_device(&svc.writer, "DA").await;
        let d2 = seed_device(&svc.writer, "DB").await;
        make_handover_with_giver_receiver(&svc, "admin@example", "X", d1).await;
        make_handover_with_giver_receiver(&svc, "%admin", "X", d2).await;

        // prefix='%' raw был бы wildcard match-everything; escape должен
        // оставить literal '%'.
        let result = svc
            .suggest_person(SuggestPersonField::Giver, "%adm", 20)
            .await
            .expect("suggest_person");
        // Только literal "%admin" должен совпасть как prefix.
        assert_eq!(result, vec!["%admin".to_string()]);
    })
    .await
    .expect("budget");
}

// ---------------------------------------------------------------------------
// Test 4: hard LIMIT (suggest_person clamps к 20).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_limit_clamped_to_20() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        // 25 distinct giver names.
        for i in 0..25 {
            let did = seed_device(&svc.writer, &format!("DX{i}")).await;
            make_handover_with_giver_receiver(&svc, &format!("Person{i:02}"), "X", did).await;
        }

        let result = svc
            .suggest_person(SuggestPersonField::Giver, "", 100)
            .await
            .expect("suggest_person");
        assert_eq!(result.len(), 20, "limit clamped to 20 (T-03.1-02-02)");
    })
    .await
    .expect("budget");
}

// ---------------------------------------------------------------------------
// Test 5: prefix.len > 100 → Validation error.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_rejects_too_long_prefix() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let too_long: String = "А".repeat(101);
        let err = svc
            .suggest_person(SuggestPersonField::Giver, &too_long, 20)
            .await
            .expect_err("must reject");
        match err {
            trackly_core::error::AppError::Validation { field, message } => {
                assert_eq!(field, "prefix");
                assert!(message.contains("слишком длинный"));
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    })
    .await
    .expect("budget");
}

// ---------------------------------------------------------------------------
// Test 6: soft-deleted акт не учитывается в frequency.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_excludes_soft_deleted_acts() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let d1 = seed_device(&svc.writer, "DA").await;
        let d2 = seed_device(&svc.writer, "DB").await;

        // 2 акта с одинаковым giver: один — soft-delete'нем.
        svc.create(ActCreateDto {
            number_override: None,
            giver_name: "Soft Иванов".into(),
            receiver_name: "X".into(),
            location_id: None,
            location_name: None,
            notes: None,
            deadline_utc: None,
            handover_date_utc: None,
            items: vec![ActItemNewDto {
                device_id: d1,
                device_ids: Vec::new(),
                quantity: 1,
            }],
        })
        .await
        .expect("create 1");
        let act2 = svc
            .create(ActCreateDto {
                number_override: None,
                giver_name: "Soft Иванов".into(),
                receiver_name: "X".into(),
                location_id: None,
                location_name: None,
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                items: vec![ActItemNewDto {
                    device_id: d2,
                    device_ids: Vec::new(),
                    quantity: 1,
                }],
            })
            .await
            .expect("create 2");

        // Pre-delete: должно вернуть имя.
        let pre = svc
            .suggest_person(SuggestPersonField::Giver, "Soft", 20)
            .await
            .expect("pre-delete suggest");
        assert_eq!(pre, vec!["Soft Иванов".to_string()]);

        // Delete one act → frequency = 1 (другой акт остался).
        svc.delete_soft(act2.id, act2.version)
            .await
            .expect("delete soft");
        let mid = svc
            .suggest_person(SuggestPersonField::Giver, "Soft", 20)
            .await
            .expect("mid suggest");
        assert_eq!(mid, vec!["Soft Иванов".to_string()]);

        // Delete the remaining act → empty result (no live acts left).
        let act1 = svc
            .list(Default::default(), Default::default())
            .await
            .expect("list")
            .items
            .into_iter()
            .find(|a| a.giver_name == "Soft Иванов")
            .expect("find act1");
        svc.delete_soft(act1.id, act1.version)
            .await
            .expect("delete soft 2");
        let post = svc
            .suggest_person(SuggestPersonField::Giver, "Soft", 20)
            .await
            .expect("post-delete suggest");
        assert!(
            post.is_empty(),
            "after deleting all live acts, frequency=0 → empty result, got {post:?}"
        );
    })
    .await
    .expect("budget");
}

// ---------------------------------------------------------------------------
// Test 7: Receiver field — independent column.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_receiver_field_independent() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let d1 = seed_device(&svc.writer, "DA").await;
        let d2 = seed_device(&svc.writer, "DB").await;
        make_handover_with_giver_receiver(&svc, "GiverOnly", "ReceiverA", d1).await;
        make_handover_with_giver_receiver(&svc, "GiverOnly", "ReceiverB", d2).await;

        // Giver suggest: 1 result.
        let g = svc
            .suggest_person(SuggestPersonField::Giver, "Giv", 20)
            .await
            .expect("giver suggest");
        assert_eq!(g.len(), 1, "single distinct giver");

        // Receiver suggest: 2 results.
        let r = svc
            .suggest_person(SuggestPersonField::Receiver, "Receiver", 20)
            .await
            .expect("receiver suggest");
        assert_eq!(r.len(), 2, "two distinct receivers");
        assert!(r.contains(&"ReceiverA".to_string()));
        assert!(r.contains(&"ReceiverB".to_string()));
    })
    .await
    .expect("budget");
}
