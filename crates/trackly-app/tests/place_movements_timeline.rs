//! Wave 0 integration coverage: timeline read-side (`PlaceMovementService::get_timeline`,
//! Plan 40-10, HST-02).
//!
//! Verifies:
//! - `place_movements_system_actor`: a row with `user_id IS NULL` surfaces as
//!   `actor_display == "система"` (D-11).
//! - `place_movements_unknown_source_degrades`: an unrecognized `source` token does not
//!   crash the whole timeline read — both the garbage row and the normal row come back
//!   (Pitfall 6 / IN-01 recurrence risk).
//! - `place_movements_printer_is_device`: a device seeded with the printer `type_id`
//!   (device_types.id = 2, "Принтер") is queried with `entity_type = "device"` — same
//!   code path as any other device, no special "printer" branch (D-21).
//!
//! Harness mirrors `place_movements_write_sites_devices.rs` / `place_movements_act_link.rs`
//! — real tempfile SQLite DB via `test_writer_and_readers`, invented place/device/person
//! names only (CLAUDE.md privacy gate).

use rusqlite::params;
use trackly_app::services::PlaceMovementService;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::writer_worker::WriterHandle;
use trackly_infra::test_support::{test_writer_and_readers, test_writer_and_readers_sized};

/// Сеет реальную строку `users` (FK-цель для `place_movements.user_id`) и
/// возвращает `Identity` менеджера. Вымышленное имя — privacy gate (CLAUDE.md).
async fn seed_manager_caller(writer: &WriterHandle) -> Identity {
    let now = SystemClock.unix_seconds();
    let user_id = writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users \
                 (login, full_name, password_hash, role, ad_user, is_active, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES ('petrov.pp', 'Петров П.П.', NULL, 'manager', 0, 1, ?1, ?1, 1)",
                params![now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed manager user");
    Identity {
        user_id: Some(user_id),
        role: Role::Manager,
    }
}

/// Сеет строку `places` напрямую. Вымышленное название.
async fn seed_place(writer: &WriterHandle, name: &str) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, is_storage, created_at_utc, updated_at_utc, version) \
                 VALUES ('room', ?1, 0, ?2, ?2, 1)",
                params![name, 1_700_000_000_i64],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

/// Сеет строку `devices` с заданным `type_id` (1 = "Устройство", 2 = "Принтер" —
/// V001__init_pragmas_and_lookups.sql). Вымышленное название.
async fn seed_device(writer: &WriterHandle, name: &str, type_id: i64, place_id: i64) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, status_id, place_id, version, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, 1, ?3, 1, ?4, ?4)",
                params![type_id, name, place_id, 1_700_000_000_i64],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
}

/// Сеет строку `place_movements` напрямую (bypassing `record_movement_if_applicable` —
/// this test exercises the READ side, the write-side guard is covered by
/// `place_movements_write_sites_devices.rs` et al.).
#[allow(clippy::too_many_arguments)]
async fn seed_movement_row(
    writer: &WriterHandle,
    entity_type: &str,
    entity_id: i64,
    from_place_id: i64,
    from_place_path: &str,
    to_place_id: i64,
    to_place_path: &str,
    source: &str,
    user_id: Option<i64>,
    actor_name_snapshot: Option<&str>,
    created_at_utc: i64,
) -> i64 {
    let entity_type = entity_type.to_string();
    let from_place_path = from_place_path.to_string();
    let to_place_path = to_place_path.to_string();
    let source = source.to_string();
    let actor_name_snapshot = actor_name_snapshot.map(|s| s.to_string());
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO place_movements \
                 (entity_type, entity_id, from_place_id, from_place_path, to_place_id, \
                  to_place_path, source, note, act_id, user_id, actor_name_snapshot, \
                  created_at_utc) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, NULL, ?8, ?9, ?10)",
                params![
                    entity_type,
                    entity_id,
                    from_place_id,
                    from_place_path,
                    to_place_id,
                    to_place_path,
                    source,
                    user_id,
                    actor_name_snapshot,
                    created_at_utc,
                ],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place_movements row")
}

// ---------------------------------------------------------------------------
// place_movements_system_actor
// ---------------------------------------------------------------------------

#[tokio::test]
async fn place_movements_system_actor() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let manager = seed_manager_caller(&writer).await;
    let from_place = seed_place(&writer, "Здание А / Каб. 101").await;
    let to_place = seed_place(&writer, "Здание Б / Склад").await;
    let device_id = seed_device(&writer, "Ноутбук инв.001", 1, to_place).await;

    seed_movement_row(
        &writer,
        "device",
        device_id,
        from_place,
        "Здание А / Каб. 101",
        to_place,
        "Здание Б / Склад",
        "manual",
        None,
        None,
        1_700_000_100,
    )
    .await;

    let svc = PlaceMovementService::new(readers);
    let timeline = svc
        .get_timeline(&manager, "device", device_id)
        .await
        .expect("get_timeline");

    assert_eq!(timeline.len(), 1);
    assert_eq!(timeline[0].actor_display, "система");
}

// ---------------------------------------------------------------------------
// place_movements_unknown_source_degrades
// ---------------------------------------------------------------------------

#[tokio::test]
async fn place_movements_unknown_source_degrades() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let manager = seed_manager_caller(&writer).await;
    let from_place = seed_place(&writer, "Здание А / Каб. 102").await;
    let to_place = seed_place(&writer, "Здание Б / Каб. 202").await;
    let device_id = seed_device(&writer, "Монитор инв.002", 1, to_place).await;

    seed_movement_row(
        &writer,
        "device",
        device_id,
        from_place,
        "Здание А / Каб. 102",
        to_place,
        "Здание Б / Каб. 202",
        "manual",
        None,
        None,
        1_700_000_200,
    )
    .await;
    seed_movement_row(
        &writer,
        "device",
        device_id,
        to_place,
        "Здание Б / Каб. 202",
        from_place,
        "Здание А / Каб. 102",
        "garbage",
        None,
        None,
        1_700_000_300,
    )
    .await;

    let svc = PlaceMovementService::new(readers);
    let timeline = svc
        .get_timeline(&manager, "device", device_id)
        .await
        .expect("get_timeline must not error on an unrecognized source token");

    assert_eq!(
        timeline.len(),
        2,
        "both rows must come back, including the garbage one"
    );
    assert!(timeline.iter().any(|m| m.source == "manual"));
    assert!(timeline.iter().any(|m| m.source == "garbage"));
}

// ---------------------------------------------------------------------------
// place_movements_printer_is_device
// ---------------------------------------------------------------------------

#[tokio::test]
async fn place_movements_printer_is_device() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let manager = seed_manager_caller(&writer).await;
    let from_place = seed_place(&writer, "Здание А / Коридор").await;
    let to_place = seed_place(&writer, "Здание А / Каб. 103").await;
    // type_id = 2 ("Принтер") — the printer convention (V001 seed).
    let printer_device_id = seed_device(&writer, "Принтер инв.003", 2, to_place).await;

    seed_movement_row(
        &writer,
        "device",
        printer_device_id,
        from_place,
        "Здание А / Коридор",
        to_place,
        "Здание А / Каб. 103",
        "manual",
        None,
        None,
        1_700_000_400,
    )
    .await;

    let svc = PlaceMovementService::new(readers);
    // D-21: no separate entity_type='printer' — the caller passes "device" for a
    // printer's underlying device id, and it reads the exact same rows.
    let timeline = svc
        .get_timeline(&manager, "device", printer_device_id)
        .await
        .expect("get_timeline");

    assert_eq!(timeline.len(), 1);
    assert_eq!(timeline[0].entity_type, "device");
    assert_eq!(timeline[0].entity_id, printer_device_id);
}

// ---------------------------------------------------------------------------
// place_movements_act_number_resolves (CR-02 gap closure)
// ---------------------------------------------------------------------------

/// Seed a real `acts` row (minimal columns) and return its id. Invented
/// giver/receiver names only (CLAUDE.md privacy gate).
async fn seed_act(writer: &WriterHandle, number: i64) -> i64 {
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO acts \
                 (number, act_type, giver_name, receiver_name, created_at_utc, updated_at_utc) \
                 VALUES (?1, 'handover', 'Кузнецов К.К.', 'Смирнов С.С.', ?2, ?2)",
                params![number, 1_700_000_000_i64],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed act")
}

/// Seed a `place_movements` row with a real `act_id` set (bypassing
/// `seed_movement_row`'s hardcoded `act_id = NULL`).
#[allow(clippy::too_many_arguments)]
async fn seed_movement_row_with_act(
    writer: &WriterHandle,
    entity_type: &str,
    entity_id: i64,
    from_place_id: i64,
    from_place_path: &str,
    to_place_id: i64,
    to_place_path: &str,
    act_id: i64,
    created_at_utc: i64,
) -> i64 {
    let entity_type = entity_type.to_string();
    let from_place_path = from_place_path.to_string();
    let to_place_path = to_place_path.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO place_movements \
                 (entity_type, entity_id, from_place_id, from_place_path, to_place_id, \
                  to_place_path, source, note, act_id, user_id, actor_name_snapshot, \
                  created_at_utc) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'act', NULL, ?7, NULL, NULL, ?8)",
                params![
                    entity_type,
                    entity_id,
                    from_place_id,
                    from_place_path,
                    to_place_id,
                    to_place_path,
                    act_id,
                    created_at_utc,
                ],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place_movements row with act_id")
}

/// CR-02: `acts.number` is `INTEGER NOT NULL` — the timeline read must resolve
/// `act_number` to the act's REAL number, not silently degrade to `None` on
/// every act-linked row (the type-mismatch regression this test guards
/// against would have `act_number == None` here even though the linked act
/// row exists).
#[tokio::test]
async fn place_movements_act_number_resolves() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let manager = seed_manager_caller(&writer).await;
    let from_place = seed_place(&writer, "Здание А / Каб. 104").await;
    let to_place = seed_place(&writer, "Здание Б / Каб. 204").await;
    let device_id = seed_device(&writer, "Сканер инв.004", 1, to_place).await;
    let act_id = seed_act(&writer, 777).await;

    seed_movement_row_with_act(
        &writer,
        "device",
        device_id,
        from_place,
        "Здание А / Каб. 104",
        to_place,
        "Здание Б / Каб. 204",
        act_id,
        1_700_000_500,
    )
    .await;

    let svc = PlaceMovementService::new(readers);
    let timeline = svc
        .get_timeline(&manager, "device", device_id)
        .await
        .expect("get_timeline");

    assert_eq!(timeline.len(), 1);
    assert_eq!(timeline[0].act_id, Some(act_id));
    assert_eq!(
        timeline[0].act_number,
        Some("777".to_string()),
        "CR-02: act_number must resolve to the real act number, not degrade to None"
    );
}

// ---------------------------------------------------------------------------
// place_movements_act_number_resolves_return_act (Plan 40-24 gap closure)
// ---------------------------------------------------------------------------

/// Seed a real `acts` return row (parent_act_id + act_type='return' +
/// sub_number) and return its id. Invented giver/receiver names only
/// (CLAUDE.md privacy gate).
async fn seed_return_act(
    writer: &WriterHandle,
    parent_act_id: i64,
    number: i64,
    sub_number: i64,
) -> i64 {
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO acts                  (number, sub_number, parent_act_id, act_type, giver_name, receiver_name,                   created_at_utc, updated_at_utc)                  VALUES (?1, ?2, ?3, 'return', 'Кузнецов К.К.', 'Смирнов С.С.', ?4, ?4)",
                params![number, sub_number, parent_act_id, 1_700_000_000_i64],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed return act")
}

/// Plan 40-24 (gap closure): a movement linked to a RETURN act must show the
/// canonical return number ("777в" for a solo return, "777в2" once a
/// sibling return exists) — not the bare parent handover number ("777",
/// indistinguishable from the handover itself, the bug this test guards
/// against).
#[tokio::test]
async fn place_movements_act_number_resolves_return_act() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let manager = seed_manager_caller(&writer).await;
    let from_place = seed_place(&writer, "Здание А / Каб. 104").await;
    let to_place = seed_place(&writer, "Здание Б / Каб. 204").await;
    let device_id = seed_device(&writer, "Сканер инв.005", 1, to_place).await;
    let handover_id = seed_act(&writer, 777).await;

    // Case 1: a single (solo) return — sibling_return_count == 1 — displays
    // without the sub-number suffix.
    let return1_id = seed_return_act(&writer, handover_id, 777, 1).await;
    seed_movement_row_with_act(
        &writer,
        "device",
        device_id,
        from_place,
        "Здание А / Каб. 104",
        to_place,
        "Здание Б / Каб. 204",
        return1_id,
        1_700_000_500,
    )
    .await;

    let svc = PlaceMovementService::new(readers.clone());
    let timeline = svc
        .get_timeline(&manager, "device", device_id)
        .await
        .expect("get_timeline");

    assert_eq!(timeline.len(), 1);
    assert_eq!(
        timeline[0].act_number,
        Some("777в".to_string()),
        "solo return must display as \"777в\", not the bare parent number"
    );

    // Case 2: a second return sibling exists — sibling_return_count == 2 —
    // both returns now keep their sub-number suffix.
    let return2_id = seed_return_act(&writer, handover_id, 777, 2).await;
    seed_movement_row_with_act(
        &writer,
        "device",
        device_id,
        to_place,
        "Здание Б / Каб. 204",
        from_place,
        "Здание А / Каб. 104",
        return2_id,
        1_700_000_600,
    )
    .await;

    let timeline = svc
        .get_timeline(&manager, "device", device_id)
        .await
        .expect("get_timeline (after second return)");

    assert_eq!(timeline.len(), 2);
    let return2_entry = timeline
        .iter()
        .find(|e| e.act_id == Some(return2_id))
        .expect("return2 entry present");
    assert_eq!(
        return2_entry.act_number,
        Some("777в2".to_string()),
        "second sibling return must keep its sub-number suffix"
    );
}

// ---------------------------------------------------------------------------
// get_timeline_does_not_deadlock_with_single_reader_slot (Phase 40-29, CR-01)
// ---------------------------------------------------------------------------

/// CR-01 regression: `get_timeline` must not take a SECOND connection from
/// the `ReaderPool` while it already holds one — with a pool sized to
/// EXACTLY one connection, a nested `acquire()` would block forever on the
/// untimed `Condvar` inside `ReaderPool::acquire()` (no other thread can ever
/// return the second connection this call itself is holding).
///
/// Deliberately NOT a plain `#[tokio::test]` + `tokio::time::timeout`: that
/// combination does not actually protect the test run against a genuine
/// deadlock here. `get_timeline`'s nested `acquire()` parks a
/// `tokio::task::spawn_blocking` worker thread forever on `Condvar::wait`
/// (no timeout). An outer `tokio::time::timeout` DOES fire and the async
/// task returns — but dropping the `#[tokio::test]`-generated `Runtime`
/// afterwards blocks the whole process, because Tokio's `Runtime::drop`
/// waits for every outstanding blocking-pool task to finish, and the leaked
/// one never does. This was confirmed empirically while writing this test:
/// with the pre-fix code temporarily restored, the `tokio::time::timeout`
/// variant hung the entire `cargo test` invocation past the 5s test budget
/// with no failure ever reported — the harness had to be killed manually.
///
/// Instead: run the fixture + `get_timeline` call on its own **std** thread
/// (own `tokio::runtime::Runtime`, not the test-harness one) and hand the
/// result back over an `mpsc` channel. `recv_timeout` on the main test
/// thread bounds the wait WITHOUT taking ownership of (or needing to drop)
/// the worker's runtime — if the worker thread deadlocks, it is simply
/// leaked (parked, near-zero CPU) and the test still fails fast and cleanly;
/// leaked `std::thread`s do not block process exit the way a leaked Tokio
/// blocking-pool task blocks `Runtime::drop`.
#[test]
fn get_timeline_does_not_deadlock_with_single_reader_slot() {
    let (result_tx, result_rx) = std::sync::mpsc::channel::<Result<usize, String>>();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("build tokio runtime for worker thread");
        let outcome = rt.block_on(async {
            let (writer, readers, _dir) = test_writer_and_readers_sized(1);
            let manager = seed_manager_caller(&writer).await;
            let place_a = seed_place(&writer, "Здание А / Каб. 106").await;
            let place_b = seed_place(&writer, "Здание Б / Каб. 206").await;
            let device_id = seed_device(&writer, "Проектор инв.006", 1, place_b).await;

            // Two rows for the same device — guarantees at least 2 loop
            // iterations, i.e. at least 2x2 = 4 calls into
            // `compute_place_path_short_with_conn` during one `get_timeline`.
            seed_movement_row(
                &writer,
                "device",
                device_id,
                place_a,
                "Здание А / Каб. 106",
                place_b,
                "Здание Б / Каб. 206",
                "manual",
                None,
                None,
                1_700_000_700,
            )
            .await;
            seed_movement_row(
                &writer,
                "device",
                device_id,
                place_b,
                "Здание Б / Каб. 206",
                place_a,
                "Здание А / Каб. 106",
                "manual",
                None,
                None,
                1_700_000_800,
            )
            .await;

            let svc = PlaceMovementService::new(readers);
            svc.get_timeline(&manager, "device", device_id)
                .await
                .map(|timeline| timeline.len())
                .map_err(|e| format!("{e:?}"))
        });
        // Ignore send errors — if the receiver already gave up (timed out),
        // there is nothing left to report to.
        let _ = result_tx.send(outcome);
    });

    match result_rx.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(Ok(len)) => assert_eq!(len, 2),
        Ok(Err(e)) => panic!("get_timeline returned an error: {e}"),
        Err(_) => panic!(
            "get_timeline exceeded 5 s budget — nested reader-pool acquire regressed (CR-01)"
        ),
    }
}
