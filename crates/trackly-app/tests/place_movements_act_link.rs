//! Wave 0 integration coverage: act family write sites for `place_movements`
//! (Plan 40-09, HST-01/HST-03).
//!
//! Verifies:
//! - `ActService::create` (handover) records one `place_movements` row per
//!   device on a real place change, with `act_id` set to the new act's id
//!   and `source='act'` — HST-03 (the timeline links back to the act number).
//! - `ActService::do_return` correctly records ZERO rows when no place
//!   override is supplied — the documented DEF-3 `place -> NULL` code path
//!   (Pitfall 4 / D-06), never a NOT NULL constraint violation or a panic.
//!
//! - `ActService::delete_soft` correctly and precisely un-happens a deleted
//!   act's own movement rows (D-03), scoped per-act inside the LIFO cascade
//!   loop for a nested handover+return, leaving an unrelated control act's
//!   rows untouched (`place_movements_act_undo_deletes`, Plan 40-20).
//!
//! Harness mirrors `acts_returns.rs` / `place_movements_write_sites_devices.rs`
//! — real tempfile SQLite DB via `test_writer_and_readers`, invented device/
//! place/person names only (CLAUDE.md privacy gate).

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto, ActReturnDto, ActReturnItemDto};
use trackly_app::services::ActService;
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::writer_worker::WriterHandle;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;

fn make_acts_service() -> (ActService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = ActService::new(writer, readers, clock);
    (svc, dir)
}

/// Seed a real `places` row, returns its id. Invented names only.
async fn seed_place(writer: &WriterHandle, name: &str) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, is_storage, created_at_utc, updated_at_utc, version) \
                 VALUES ('room', ?1, 0, ?2, ?2, 1)",
                params![name, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

/// Seed real `devices` rows at a given `place_id` (may be `None`). Invented
/// names only.
async fn seed_devices_at_place(
    writer: &WriterHandle,
    names: &[&str],
    place_id: Option<i64>,
) -> Vec<i64> {
    let names: Vec<String> = names.iter().map(|s| s.to_string()).collect();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            let mut out = Vec::with_capacity(names.len());
            for name in &names {
                tx.execute(
                    "INSERT INTO devices \
                     (type_id, name, status_id, place_id, version, created_at_utc, updated_at_utc) \
                     VALUES (1, ?1, 1, ?2, 1, ?3, ?3)",
                    params![name, place_id, 1_700_000_000_i64],
                )
                .map_err(map_rusqlite)?;
                out.push(tx.last_insert_rowid());
            }
            tx.commit().map_err(map_rusqlite)?;
            Ok(out)
        })
        .await
        .expect("seed devices")
}

async fn count_movements_for_act(
    readers: Arc<trackly_infra::db::pools::ReaderPool>,
    act_id: i64,
) -> i64 {
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT COUNT(*) FROM place_movements WHERE act_id = ?1",
            params![act_id],
            |r| r.get(0),
        )
    })
    .await
    .expect("spawn_blocking")
    .expect("count place_movements by act_id")
}

async fn count_movements_for_entities(
    readers: Arc<trackly_infra::db::pools::ReaderPool>,
    entity_ids: Vec<i64>,
) -> i64 {
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        let placeholders = entity_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT COUNT(*) FROM place_movements \
             WHERE entity_type = 'device' AND entity_id IN ({placeholders})"
        );
        let params: Vec<Box<dyn rusqlite::ToSql>> = entity_ids
            .iter()
            .map(|id| Box::new(*id) as Box<dyn rusqlite::ToSql>)
            .collect();
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.query_row(&sql, param_refs.as_slice(), |r| r.get(0))
    })
    .await
    .expect("spawn_blocking")
    .expect("count place_movements by entity_id")
}

// ---------------------------------------------------------------------------
// place_movements_act_link
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_movements_act_link() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let place_a = seed_place(&svc.writer, "Каб. 101").await;
        let place_b = seed_place(&svc.writer, "Склад №2").await;
        let device_ids =
            seed_devices_at_place(&svc.writer, &["Ноутбук Dell", "Монитор LG"], Some(place_a))
                .await;

        let payload = ActCreateDto {
            number_override: None,
            giver_name: "Иванов И.И.".into(),
            receiver_name: "Петров П.П.".into(),
            place_id: Some(place_b),
            notes: None,
            deadline_utc: None,
            handover_date_utc: None,
            items: device_ids
                .iter()
                .map(|&id| ActItemNewDto {
                    device_id: id,
                    device_ids: Vec::new(),
                    quantity: 1,
                })
                .collect(),
        };

        let handover = svc
            .create(&Identity::trusted_admin(), payload)
            .await
            .expect("create handover with real place change");

        // One row per device, each linked to the act's id.
        let act_linked = count_movements_for_act(svc.readers.clone(), handover.id).await;
        assert_eq!(
            act_linked,
            device_ids.len() as i64,
            "HST-03: expected one place_movements row per device linked to the act, got {act_linked}"
        );

        let by_entity = count_movements_for_entities(svc.readers.clone(), device_ids.clone()).await;
        assert_eq!(
            by_entity,
            device_ids.len() as i64,
            "expected exactly one row per device entity"
        );

        // Verify source + from/to place ids on one representative row.
        let dev_id = device_ids[0];
        let readers = svc.readers.clone();
        let row: (String, i64, i64, Option<i64>) = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT source, from_place_id, to_place_id, act_id \
                 FROM place_movements WHERE entity_type='device' AND entity_id=?1",
                params![dev_id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, Option<i64>>(3)?,
                    ))
                },
            )
        })
        .await
        .expect("spawn_blocking")
        .expect("query place_movements row");

        let (source, from_place_id, to_place_id, act_id) = row;
        assert_eq!(source, "act", "handover-driven move must have source='act'");
        assert_eq!(from_place_id, place_a);
        assert_eq!(to_place_id, place_b);
        assert_eq!(act_id, Some(handover.id));
    })
    .await
    .expect("place_movements_act_link exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// place_movements_null_place_skip
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_movements_null_place_skip() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let place_a = seed_place(&svc.writer, "Каб. 202").await;
        let device_ids = seed_devices_at_place(&svc.writer, &["Принтер HP"], Some(place_a)).await;
        let dev_id = device_ids[0];

        // Handover at the SAME place — D-04 no-op, zero movements from create.
        let handover_payload = ActCreateDto {
            number_override: None,
            giver_name: "Сидоров С.С.".into(),
            receiver_name: "Кузнецов К.К.".into(),
            place_id: Some(place_a),
            notes: None,
            deadline_utc: None,
            handover_date_utc: None,
            items: vec![ActItemNewDto {
                device_id: dev_id,
                device_ids: Vec::new(),
                quantity: 1,
            }],
        };
        let handover = svc
            .create(&Identity::trusted_admin(), handover_payload)
            .await
            .expect("create handover, place unchanged");

        let after_create = count_movements_for_entities(svc.readers.clone(), vec![dev_id]).await;
        assert_eq!(
            after_create, 0,
            "D-04: unchanged place at handover must record zero movements, got {after_create}"
        );

        // DEF-3: return with NO place override at all (no bulk_place_id, no
        // per-row place_id_override) — `update_full_in_tx` writes
        // `place_id = NULL`. The guard inside `record_movement_if_applicable`
        // must skip this (before=Some, after=None), never panic or violate
        // the `place_movements.to_place_id NOT NULL` constraint.
        let return_payload = ActReturnDto {
            bulk_condition: Some("Хорошее".into()),
            bulk_place_id: None,
            apply_to_all: true,
            giver_name: None,
            receiver_name: None,
            handover_date_utc: None,
            items: vec![ActReturnItemDto {
                act_item_id: handover.items[0].id,
                device_id: dev_id,
                device_ids: vec![dev_id],
                quantity: 1,
                condition_override: None,
                place_id_override: None,
            }],
        };
        let ret = svc
            .do_return(&Identity::trusted_admin(), handover.id, return_payload)
            .await
            .expect("do_return with no place override must not panic or crash");

        let after_return = count_movements_for_entities(svc.readers.clone(), vec![dev_id]).await;
        assert_eq!(
            after_return, 0,
            "Pitfall 4/D-06: place -> NULL return must record zero movements, got {after_return}"
        );

        // Also confirm no row was linked to the return act itself.
        let by_return_act = count_movements_for_act(svc.readers.clone(), ret.id).await;
        assert_eq!(by_return_act, 0);
    })
    .await
    .expect("place_movements_null_place_skip exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// place_movements_act_undo_deletes (Plan 40-20, D-03/Pitfall 5)
// ---------------------------------------------------------------------------

/// Deleting a handover act with a nested cascaded return must delete exactly
/// the movement rows belonging to each deleted act's own `act_id` (the
/// handover's own rows AND the nested return's own rows), each removed at its
/// own point in the existing LIFO undo loop — never a single blanket delete
/// at the end (D-03, Pitfall 5). A control act's rows, untouched by this
/// cascade, must survive.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_movements_act_undo_deletes() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let place_a = seed_place(&svc.writer, "Каб. 301").await;
        let place_b = seed_place(&svc.writer, "Склад №3").await;
        let place_c = seed_place(&svc.writer, "Каб. 302").await;
        let control_place_from = seed_place(&svc.writer, "Каб. 401").await;
        let control_place_to = seed_place(&svc.writer, "Склад №4").await;

        // Handover H: device moves place_a -> place_b, one movement row
        // linked to H.id.
        let device_ids = seed_devices_at_place(&svc.writer, &["Ноутбук Asus"], Some(place_a)).await;
        let dev_id = device_ids[0];
        let handover_payload = ActCreateDto {
            number_override: None,
            giver_name: "Смирнов С.С.".into(),
            receiver_name: "Николаев Н.Н.".into(),
            place_id: Some(place_b),
            notes: None,
            deadline_utc: None,
            handover_date_utc: None,
            items: vec![ActItemNewDto {
                device_id: dev_id,
                device_ids: Vec::new(),
                quantity: 1,
            }],
        };
        let handover = svc
            .create(&Identity::trusted_admin(), handover_payload)
            .await
            .expect("create handover H");

        // Return R nested under H: device moves place_b -> place_c, one
        // movement row linked to R.id.
        let return_payload = ActReturnDto {
            bulk_condition: Some("Хорошее".into()),
            bulk_place_id: None,
            apply_to_all: true,
            giver_name: None,
            receiver_name: None,
            handover_date_utc: None,
            items: vec![ActReturnItemDto {
                act_item_id: handover.items[0].id,
                device_id: dev_id,
                device_ids: vec![dev_id],
                quantity: 1,
                condition_override: None,
                place_id_override: Some(place_c),
            }],
        };
        let ret = svc
            .do_return(&Identity::trusted_admin(), handover.id, return_payload)
            .await
            .expect("do_return with a real place override, nested under H");

        // Control handover C: unrelated device, unrelated place change — its
        // movement row must survive H's cascade-delete untouched.
        let control_device_ids =
            seed_devices_at_place(&svc.writer, &["Монитор Acer"], Some(control_place_from)).await;
        let control_dev_id = control_device_ids[0];
        let control_payload = ActCreateDto {
            number_override: None,
            giver_name: "Волков В.В.".into(),
            receiver_name: "Зайцев З.З.".into(),
            place_id: Some(control_place_to),
            notes: None,
            deadline_utc: None,
            handover_date_utc: None,
            items: vec![ActItemNewDto {
                device_id: control_dev_id,
                device_ids: Vec::new(),
                quantity: 1,
            }],
        };
        let control = svc
            .create(&Identity::trusted_admin(), control_payload)
            .await
            .expect("create control handover C");

        // Sanity: each act has exactly one movement row of its own before
        // the delete.
        assert_eq!(count_movements_for_act(svc.readers.clone(), handover.id).await, 1);
        assert_eq!(count_movements_for_act(svc.readers.clone(), ret.id).await, 1);
        assert_eq!(count_movements_for_act(svc.readers.clone(), control.id).await, 1);

        // Delete H — cascades: undo+soft-delete R first (LIFO), then undo+
        // soft-delete H itself. Both acts' own movement rows must be gone;
        // the control act's row must remain untouched.
        //
        // `do_return` bumps the parent handover's `version` (via
        // `recompute_parent_archived`) — re-fetch H to get its current
        // version rather than the stale one captured at `create` time.
        let handover_current = svc.get(handover.id).await.expect("re-fetch H before delete");
        svc.delete_soft(handover.id, handover_current.version)
            .await
            .expect("delete_soft on H with nested cascade must succeed");

        assert_eq!(
            count_movements_for_act(svc.readers.clone(), handover.id).await,
            0,
            "D-03: handover's own movement rows must be deleted"
        );
        assert_eq!(
            count_movements_for_act(svc.readers.clone(), ret.id).await,
            0,
            "D-03/Pitfall 5: nested return's own movement rows must be deleted, scoped to its own act_id"
        );
        assert_eq!(
            count_movements_for_act(svc.readers.clone(), control.id).await,
            1,
            "Pitfall 5: an unrelated control act's movement rows must survive the cascade untouched"
        );
    })
    .await
    .expect("place_movements_act_undo_deletes exceeded 30s budget");
}
