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
use trackly_core::auth::Identity;
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
    svc.create(
        &Identity::trusted_admin(),
        ActCreateDto {
            number_override: None,
            giver_name: giver.into(),
            receiver_name: receiver.into(),
            place_id: None,
            notes: None,
            deadline_utc: None,
            handover_date_utc: None,
            items: vec![ActItemNewDto {
                device_id,
                device_ids: Vec::new(),
                quantity: 1,
            }],
        },
    )
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
        svc.create(
            &Identity::trusted_admin(),
            ActCreateDto {
                number_override: None,
                giver_name: "Soft Иванов".into(),
                receiver_name: "X".into(),
                place_id: None,
                notes: None,
                deadline_utc: None,
                handover_date_utc: None,
                items: vec![ActItemNewDto {
                    device_id: d1,
                    device_ids: Vec::new(),
                    quantity: 1,
                }],
            },
        )
        .await
        .expect("create 1");
        let act2 = svc
            .create(
                &Identity::trusted_admin(),
                ActCreateDto {
                    number_override: None,
                    giver_name: "Soft Иванов".into(),
                    receiver_name: "X".into(),
                    place_id: None,
                    notes: None,
                    deadline_utc: None,
                    handover_date_utc: None,
                    items: vec![ActItemNewDto {
                        device_id: d2,
                        device_ids: Vec::new(),
                        quantity: 1,
                    }],
                },
            )
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

// ---------------------------------------------------------------------------
// GAP-12-01 / 12-04: suggest_person must also source names from
// cartridges.holder_name (set by OperationModal install/to_refill via
// given_to_name), not just acts.{giver_name|receiver_name}.
// ---------------------------------------------------------------------------

/// Inserts a `cartridge_models` row (kind_id defaults to 1 per V016) and
/// returns its id — minimal fixture for the cartridges.holder_name tests
/// below (mirrors the seeding style in `phase06_stubs.rs`).
async fn seed_cartridge_model(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    now: i64,
) -> i64 {
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO cartridge_models (brand, model, created_at_utc, updated_at_utc, version) \
                 VALUES ('Pantum', 'TL-5120X', ?1, ?1, 1)",
                params![now],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed cartridge model")
}

/// Inserts a `cartridges` row with the given `holder_name` (and optional
/// soft-delete), returns nothing — fixture only needs the row to exist for
/// `suggest_person()`'s UNION arm to find it.
async fn seed_cartridge_with_holder(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    model_id: i64,
    code: &str,
    holder_name: &str,
    deleted: bool,
    now: i64,
) {
    let code = code.to_string();
    let holder_name = holder_name.to_string();
    writer
        .execute(move |conn| {
            if deleted {
                conn.execute(
                    "INSERT INTO cartridges \
                     (code, model_id, status_id, holder_name, created_at_utc, updated_at_utc, deleted_at_utc, version) \
                     VALUES (?1, ?2, 1, ?3, ?4, ?4, ?4, 1)",
                    params![code, model_id, holder_name, now],
                )
                .map_err(map_rusqlite)?;
            } else {
                conn.execute(
                    "INSERT INTO cartridges \
                     (code, model_id, status_id, holder_name, created_at_utc, updated_at_utc, version) \
                     VALUES (?1, ?2, 1, ?3, ?4, ?4, 1)",
                    params![code, model_id, holder_name, now],
                )
                .map_err(map_rusqlite)?;
            }
            Ok(())
        })
        .await
        .expect("seed cartridge with holder");
}

/// Test 8 (GAP-12-01): name appearing in both an act AND a cartridge
/// holder_name is deduplicated — exactly one occurrence in the result, not
/// two identical rows.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_dedupes_name_present_in_acts_and_cartridges() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let now = 1_700_000_000_i64;
        let d1 = seed_device(&svc.writer, "DA").await;
        make_handover_with_giver_receiver(&svc, "Иванов И.И.", "X", d1).await;

        let model_id = seed_cartridge_model(&svc.writer, now).await;
        seed_cartridge_with_holder(&svc.writer, model_id, "C-100001", "Иванов И.И.", false, now)
            .await;

        let result = svc
            .suggest_person(SuggestPersonField::Giver, "Иван", 20)
            .await
            .expect("suggest_person");
        assert_eq!(
            result,
            vec!["Иванов И.И.".to_string()],
            "name present in both acts and cartridges must be deduplicated, got {result:?}"
        );
    })
    .await
    .expect("budget");
}

/// Test 9 (GAP-12-01): a name that exists ONLY in cartridges.holder_name
/// (no matching act row at all) must still surface — proves the cartridges
/// source is reachable independently of acts having any rows.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_finds_name_from_cartridges_only() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let now = 1_700_000_000_i64;

        let model_id = seed_cartridge_model(&svc.writer, now).await;
        seed_cartridge_with_holder(&svc.writer, model_id, "C-100002", "Петров П.П.", false, now)
            .await;

        let result = svc
            .suggest_person(SuggestPersonField::Receiver, "Петр", 20)
            .await
            .expect("suggest_person");
        assert_eq!(
            result,
            vec!["Петров П.П.".to_string()],
            "cartridges-only holder name must surface, got {result:?}"
        );
    })
    .await
    .expect("budget");
}

/// Test 10 (regression guard): soft-deleted cartridges must not leak into
/// suggestions, mirroring the existing acts `deleted_at_utc IS NULL` guard.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_excludes_soft_deleted_cartridges() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let now = 1_700_000_000_i64;

        let model_id = seed_cartridge_model(&svc.writer, now).await;
        seed_cartridge_with_holder(&svc.writer, model_id, "C-100003", "Скрытый С.С.", true, now)
            .await;

        let result = svc
            .suggest_person(SuggestPersonField::Giver, "Скрыт", 20)
            .await
            .expect("suggest_person");
        assert!(
            result.is_empty(),
            "soft-deleted cartridge holder_name must not leak into suggestions, got {result:?}"
        );
    })
    .await
    .expect("budget");
}

// ---------------------------------------------------------------------------
// GAP-12-06 (A3, part "a"): suggest_person must also source `given_by_name`
// ("Кто выдал") from audit_log.payload_json for cartridge install/to_refill
// operations — this value is currently written ONLY to the JSON payload,
// never to a queryable column, so it never reached the autocomplete
// suggestions (unlike `given_to_name`, which lands in cartridges.holder_name
// and is already aggregated by the existing UNION arm above).
// ---------------------------------------------------------------------------

/// Inserts an `audit_log` row with the given `entity_type`/`action` and a
/// `payload_json` containing `given_by_name` — minimal fixture mirroring the
/// real shape written by `CartridgesSqliteRepository::op_payload_json()`.
async fn seed_audit_log_given_by_name(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    entity_type: &str,
    action: &str,
    given_by_name: &str,
    now: i64,
) {
    let entity_type = entity_type.to_string();
    let action = action.to_string();
    let payload_json = serde_json::json!({
        "op": "install",
        "date_utc": now,
        "given_by_name": given_by_name,
        "given_to_name": "Кому Выдал",
        "location": "Склад",
    })
    .to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO audit_log \
                 (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
                 VALUES (?1, 1, ?2, NULL, NULL, NULL, ?3, ?4)",
                params![entity_type, action, payload_json, now],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("seed audit_log given_by_name");
}

/// Test 11 (GAP-12-06): `custom:install` audit row's `given_by_name`
/// surfaces in `Giver`-field suggestions.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_finds_given_by_name_from_install_audit_log() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let now = 1_700_000_000_i64;

        seed_audit_log_given_by_name(
            &svc.writer,
            "cartridge",
            "custom:install",
            "Иванов И.И.",
            now,
        )
        .await;

        let result = svc
            .suggest_person(SuggestPersonField::Giver, "Иван", 20)
            .await
            .expect("suggest_person");
        assert_eq!(
            result,
            vec!["Иванов И.И.".to_string()],
            "given_by_name from custom:install audit_log must surface in Giver suggestions, got {result:?}"
        );
    })
    .await
    .expect("budget");
}

/// Test 12 (GAP-12-06): `custom:to_refill` audit row's `given_by_name` also
/// surfaces (second relevant operation from the UAT text — «Отправка на
/// заправку»).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_finds_given_by_name_from_to_refill_audit_log() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let now = 1_700_000_000_i64;

        seed_audit_log_given_by_name(
            &svc.writer,
            "cartridge",
            "custom:to_refill",
            "Сидоров С.С.",
            now,
        )
        .await;

        let result = svc
            .suggest_person(SuggestPersonField::Giver, "Сидор", 20)
            .await
            .expect("suggest_person");
        assert_eq!(
            result,
            vec!["Сидоров С.С.".to_string()],
            "given_by_name from custom:to_refill audit_log must surface in Giver suggestions, got {result:?}"
        );
    })
    .await
    .expect("budget");
}

/// Test 13 (GAP-12-06): irrelevant actions (e.g. `custom:return_to_stock`)
/// must NOT contribute their `given_by_name` payload field — the `action IN
/// (...)` filter excludes anything outside install/to_refill.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_excludes_given_by_name_from_irrelevant_action() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let now = 1_700_000_000_i64;

        seed_audit_log_given_by_name(
            &svc.writer,
            "cartridge",
            "custom:return_to_stock",
            "Петров П.П.",
            now,
        )
        .await;

        let result = svc
            .suggest_person(SuggestPersonField::Giver, "Петр", 20)
            .await
            .expect("suggest_person");
        assert!(
            result.is_empty(),
            "given_by_name from a non install/to_refill action must not surface, got {result:?}"
        );
    })
    .await
    .expect("budget");
}

/// Test 14 (GAP-12-06 regression guard): the new `given_by_name` arm only
/// applies to the `Giver` field — `Receiver` suggestions must not pick it up
/// (given_by_name is semantically "Кто выдал", not "Кому выдал").
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_given_by_name_does_not_leak_into_receiver_field() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let now = 1_700_000_000_i64;

        seed_audit_log_given_by_name(
            &svc.writer,
            "cartridge",
            "custom:install",
            "ТолькоГивер",
            now,
        )
        .await;

        let result = svc
            .suggest_person(SuggestPersonField::Receiver, "ТолькоГивер", 20)
            .await
            .expect("suggest_person");
        assert!(
            result.is_empty(),
            "given_by_name must not surface in Receiver-field suggestions, got {result:?}"
        );
    })
    .await
    .expect("budget");
}

// ---------------------------------------------------------------------------
// UAT4-01 (Plan 40-34): suggest_person must also source `given_to_name`
// ("Кому выдал") from audit_log.payload_json for cartridge install/to_refill
// operations — symmetric to `given_by_name_arm` above (GAP-12-06). Without
// this arm, a person entered as "Кому выдал" when sending a cartridge to
// refill vanished from suggestions the moment cartridges.holder_name of the
// SAME cartridge was overwritten by a later operation — holder_name only
// ever reflects the CURRENT value, not history.
//
// `seed_audit_log_given_by_name` (above) is reused unmodified — its fixed
// payload_json literal already carries `given_to_name: "Кому Выдал"`
// alongside `given_by_name`, which is exactly what these tests assert on.
// ---------------------------------------------------------------------------

/// Test 15 (UAT4-01): `custom:install` audit row's `given_to_name` surfaces
/// in `Receiver`-field suggestions.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_finds_given_to_name_from_install_audit_log() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let now = 1_700_000_000_i64;

        seed_audit_log_given_by_name(
            &svc.writer,
            "cartridge",
            "custom:install",
            "Иванов И.И.",
            now,
        )
        .await;

        let result = svc
            .suggest_person(SuggestPersonField::Receiver, "Кому", 20)
            .await
            .expect("suggest_person");
        assert_eq!(
            result,
            vec!["Кому Выдал".to_string()],
            "given_to_name from custom:install audit_log must surface in Receiver suggestions, got {result:?}"
        );
    })
    .await
    .expect("budget");
}

/// Test 16 (UAT4-01): `custom:to_refill` audit row's `given_to_name` also
/// surfaces (the operation the original UAT4-01 symptom was reported
/// against — «Отправка на заправку»).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_finds_given_to_name_from_to_refill_audit_log() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let now = 1_700_000_000_i64;

        seed_audit_log_given_by_name(
            &svc.writer,
            "cartridge",
            "custom:to_refill",
            "Сидоров С.С.",
            now,
        )
        .await;

        let result = svc
            .suggest_person(SuggestPersonField::Receiver, "Кому", 20)
            .await
            .expect("suggest_person");
        assert_eq!(
            result,
            vec!["Кому Выдал".to_string()],
            "given_to_name from custom:to_refill audit_log must surface in Receiver suggestions, got {result:?}"
        );
    })
    .await
    .expect("budget");
}

/// Test 17 (UAT4-01): irrelevant actions (e.g. `custom:return_to_stock`)
/// must NOT contribute their `given_to_name` payload field — mirrors the
/// `given_by_name` action filter guard (Test 13).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_excludes_given_to_name_from_irrelevant_action() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let now = 1_700_000_000_i64;

        seed_audit_log_given_by_name(
            &svc.writer,
            "cartridge",
            "custom:return_to_stock",
            "Петров П.П.",
            now,
        )
        .await;

        let result = svc
            .suggest_person(SuggestPersonField::Receiver, "Кому", 20)
            .await
            .expect("suggest_person");
        assert!(
            result.is_empty(),
            "given_to_name from a non install/to_refill action must not surface, got {result:?}"
        );
    })
    .await
    .expect("budget");
}

/// Test 18 (UAT4-01 regression guard): the new `given_to_name` arm only
/// applies to the `Receiver` field — `Giver` suggestions must not pick it up
/// (given_to_name is semantically "Кому выдал", not "Кто выдал").
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn suggest_person_given_to_name_does_not_leak_into_giver_field() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let now = 1_700_000_000_i64;

        seed_audit_log_given_by_name(
            &svc.writer,
            "cartridge",
            "custom:install",
            "ТолькоРесивер источник",
            now,
        )
        .await;

        let result = svc
            .suggest_person(SuggestPersonField::Giver, "Кому", 20)
            .await
            .expect("suggest_person");
        assert!(
            result.is_empty(),
            "given_to_name must not surface in Giver-field suggestions, got {result:?}"
        );
    })
    .await
    .expect("budget");
}
