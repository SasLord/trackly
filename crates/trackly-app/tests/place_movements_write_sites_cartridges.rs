//! Wave 0 integration coverage: cartridge family write sites for
//! `place_movements` (Plan 40-08, HST-01).
//!
//! Verifies BOTH cartridge write sites call
//! `SqlitePlaceMovementsRepository::record_movement_if_applicable` correctly:
//! - `transition_in_tx`'s main mutation records exactly one row with an
//!   operation-derived Russian `note` (D-05's "meaningful reason" lives in
//!   `note`, never in `source` — D-07's closed enum stays `manual`)
//! - `transition_in_tx`'s nested auto-return branch (Pitfall 3, RESEARCH.md)
//!   records a SECOND, separate row for the previously-installed cartridge —
//!   the single easiest write site to miss in the whole phase
//! - `cartridge_service::update` (plain manual `PlacePicker` edit) still
//!   records `note = None`, proving a transition-driven row is distinguishable
//!   from a manual-edit row, not byte-identical
//!
//! Harness mirrors `place_movements_write_sites_devices.rs` / `cartridges_lifecycle.rs`
//! — real tempfile SQLite DB via `test_writer_and_readers`, invented place/printer
//! names only (CLAUDE.md privacy gate).

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::cartridge::{
    CartridgeCreateDto, CartridgeModelCreateDto, CartridgeTransitionPayload,
};
use trackly_app::services::CartridgeService;
use trackly_core::auth::Identity;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::pools::ReaderPool;
use trackly_infra::db::writer_worker::WriterHandle;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;

/// `Identity::trusted_admin()` — unlocked-desktop identity (D-Desktop-01),
/// `user_id: None`. Mirrors `cartridges_lifecycle.rs::admin_caller()`.
fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

fn make_cartridge_service() -> (CartridgeService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let svc = CartridgeService::new(writer, readers, clock);
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

/// Seed a printer device (type_id=2) at a given `place_id`. D-13: `transition`
/// resolves an Install's missing `place_id` from the target printer's own
/// `devices.place_id` — this is what drives the "real place change" scenarios
/// below without the test needing to pass an explicit `place_id` override.
async fn seed_printer_device_at_place(writer: &WriterHandle, name: &str, place_id: i64) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            tx.execute(
                "INSERT INTO devices \
                 (type_id, name, status_id, place_id, version, created_at_utc, updated_at_utc) \
                 VALUES (2, ?1, 2, ?2, 1, ?3, ?3)",
                params![name, place_id, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            let id = tx.last_insert_rowid();
            tx.commit().map_err(map_rusqlite)?;
            Ok(id)
        })
        .await
        .expect("seed printer device")
}

async fn seed_model(svc: &CartridgeService) -> i64 {
    svc.model_create(CartridgeModelCreateDto {
        brand: "HP".into(),
        model: "CE285A".into(),
        kind_id: 1,
        color: Some("Чёрный".into()),
        notes: None,
        compatibility: vec![],
    })
    .await
    .expect("seed model")
    .id
}

async fn create_cartridge_at_place(
    svc: &CartridgeService,
    model_id: i64,
    place_id: Option<i64>,
) -> trackly_app::dto::cartridge::CartridgeDto {
    svc.create(CartridgeCreateDto {
        model_id,
        code_override: None,
        state_id: Some(1), // Полный
        place_id,
        notes: None,
    })
    .await
    .expect("create cartridge")
}

#[allow(clippy::type_complexity)]
async fn movement_rows(
    readers: Arc<ReaderPool>,
    entity_id: i64,
) -> Vec<(i64, i64, Option<String>)> {
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        let mut stmt = conn
            .prepare(
                "SELECT from_place_id, to_place_id, note FROM place_movements \
                 WHERE entity_type='cartridge' AND entity_id=?1 ORDER BY id ASC",
            )
            .expect("prepare");
        let rows = stmt
            .query_map(params![entity_id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })
            .expect("query_map")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect rows");
        rows
    })
    .await
    .expect("spawn_blocking")
}

// ---------------------------------------------------------------------------
// place_movements_cartridge_transition_install
// ---------------------------------------------------------------------------

/// A plain Install (no printer occupied by another cartridge) that changes
/// place from A (stock) to B (printer's own place, resolved via D-13) records
/// exactly one movement row with an operation-derived `note` — never `None`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_movements_cartridge_transition_install() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let place_a = seed_place(&svc.writer, "Каб. 101 (склад)").await;
        let place_b = seed_place(&svc.writer, "Каб. 202 (принтер)").await;
        let printer_id =
            seed_printer_device_at_place(&svc.writer, "Pantum BM5100ADN", place_b).await;

        let cart = create_cartridge_at_place(&svc, model_id, Some(place_a)).await;

        svc.transition(
            &admin_caller(),
            CartridgeTransitionPayload::Install {
                cartridge_id: cart.id,
                version: cart.version,
                date_utc: 1_700_000_000,
                given_by_name: "Иванов".into(),
                given_to_name: "Петров".into(),
                place_id: None,
                printer_device_id: Some(printer_id),
                previous_cartridge_state_id: None,
                previous_cartridge_place_id: None,
            },
        )
        .await
        .expect("transition Install");

        let rows = movement_rows(svc.readers.clone(), cart.id).await;
        assert_eq!(
            rows.len(),
            1,
            "install with a real place change must produce exactly one row, got {rows:?}"
        );
        let (from_place_id, to_place_id, note) = &rows[0];
        assert_eq!(*from_place_id, place_a);
        assert_eq!(*to_place_id, place_b);
        assert_eq!(
            note.as_deref(),
            Some("автоматически при установке в принтер"),
            "note must be the operation-derived string, never None"
        );
    })
    .await
    .expect("place_movements_cartridge_transition_install exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// place_movements_cartridge_transition_nested_auto_return
// ---------------------------------------------------------------------------

/// Installing a SECOND cartridge into a printer that already holds a first
/// one (В работе) auto-returns the first — Pitfall 3: this must produce a
/// SECOND, separate movement row for the auto-returned (first) cartridge,
/// with `entity_id` matching the FIRST cartridge's id (not the second's), and
/// a note distinct from the main mutation's own note.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_movements_cartridge_transition_nested_auto_return() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let place_a = seed_place(&svc.writer, "Каб. 101 (склад)").await;
        let place_b = seed_place(&svc.writer, "Каб. 202 (принтер)").await;
        let place_c = seed_place(&svc.writer, "Каб. 303 (заправка)").await;
        let printer_id = seed_printer_device_at_place(&svc.writer, "Kyocera ECOSYS", place_b).await;

        let cart_a = create_cartridge_at_place(&svc, model_id, Some(place_a)).await;
        let cart_b = create_cartridge_at_place(&svc, model_id, Some(place_a)).await;

        // Install A into the printer first — A: place_a -> place_b (main mutation row #1).
        svc.transition(
            &admin_caller(),
            CartridgeTransitionPayload::Install {
                cartridge_id: cart_a.id,
                version: cart_a.version,
                date_utc: 1_700_000_000,
                given_by_name: "Иванов".into(),
                given_to_name: "Петров".into(),
                place_id: None,
                printer_device_id: Some(printer_id),
                previous_cartridge_state_id: None,
                previous_cartridge_place_id: None,
            },
        )
        .await
        .expect("install A");

        // Install B into the SAME printer — must auto-return A to place_c.
        svc.transition(
            &admin_caller(),
            CartridgeTransitionPayload::Install {
                cartridge_id: cart_b.id,
                version: cart_b.version,
                date_utc: 1_700_000_100,
                given_by_name: "Сидоров".into(),
                given_to_name: "Кузнецов".into(),
                place_id: None,
                printer_device_id: Some(printer_id),
                previous_cartridge_state_id: None,
                previous_cartridge_place_id: Some(place_c),
            },
        )
        .await
        .expect("install B into same printer, auto-returns A");

        // B: its own main mutation row (place_a -> place_b).
        let b_rows = movement_rows(svc.readers.clone(), cart_b.id).await;
        assert_eq!(
            b_rows.len(),
            1,
            "B's own install must produce exactly one row, got {b_rows:?}"
        );
        assert_eq!(
            b_rows[0].2.as_deref(),
            Some("автоматически при установке в принтер")
        );

        // A: TWO rows — its own install, THEN the auto-return, both attributed
        // to A's entity_id (cart_a.id), never cart_b.id (Pitfall 3's exact
        // regression class).
        let a_rows = movement_rows(svc.readers.clone(), cart_a.id).await;
        assert_eq!(
            a_rows.len(),
            2,
            "A must have TWO movement rows (its own install + the auto-return), got {a_rows:?}"
        );

        let (a_first_from, a_first_to, a_first_note) = &a_rows[0];
        assert_eq!(*a_first_from, place_a);
        assert_eq!(*a_first_to, place_b);
        assert_eq!(
            a_first_note.as_deref(),
            Some("автоматически при установке в принтер"),
            "A's own install row must carry the install note"
        );

        let (a_second_from, a_second_to, a_second_note) = &a_rows[1];
        assert_eq!(
            *a_second_from, place_b,
            "auto-return's from must be A's place right before being auto-returned"
        );
        assert_eq!(*a_second_to, place_c);
        assert_eq!(
            a_second_note.as_deref(),
            Some("автоматически возвращён на склад при установке другого картриджа"),
            "auto-return's own row must carry a note DISTINCT from the main mutation's"
        );
        assert_ne!(
            a_first_note, a_second_note,
            "the two rows on the SAME entity must be distinguishable by note"
        );
    })
    .await
    .expect("place_movements_cartridge_transition_nested_auto_return exceeded 30 s budget");
}

// ---------------------------------------------------------------------------
// place_movements_cartridge_transition_note_distinguishes_from_manual
// ---------------------------------------------------------------------------

/// Contrast case (the blocker's own regression test): a transition-driven row
/// carries a non-empty, operation-derived `note`; a plain manual
/// `cartridge_service::update` edit on a DIFFERENT cartridge still produces
/// `note == None` — the two must never be byte-identical.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_movements_cartridge_transition_note_distinguishes_from_manual() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;
        let place_a = seed_place(&svc.writer, "Каб. 101 (склад)").await;
        let place_b = seed_place(&svc.writer, "Каб. 202 (принтер)").await;
        let printer_id = seed_printer_device_at_place(&svc.writer, "Xerox Phaser", place_b).await;

        // Transition-driven cartridge.
        let cart_transition = create_cartridge_at_place(&svc, model_id, Some(place_a)).await;
        svc.transition(
            &admin_caller(),
            CartridgeTransitionPayload::Install {
                cartridge_id: cart_transition.id,
                version: cart_transition.version,
                date_utc: 1_700_000_000,
                given_by_name: "Иванов".into(),
                given_to_name: "Петров".into(),
                place_id: None,
                printer_device_id: Some(printer_id),
                previous_cartridge_state_id: None,
                previous_cartridge_place_id: None,
            },
        )
        .await
        .expect("transition Install");

        // Plain manual edit on a DIFFERENT cartridge (place_a -> place_b via update()).
        let cart_manual = create_cartridge_at_place(&svc, model_id, Some(place_a)).await;
        svc.update(
            &admin_caller(),
            cart_manual.id,
            cart_manual.version,
            Some(place_b),
            None,
        )
        .await
        .expect("manual update place A -> B");

        let transition_rows = movement_rows(svc.readers.clone(), cart_transition.id).await;
        assert_eq!(transition_rows.len(), 1);
        assert!(
            transition_rows[0].2.is_some(),
            "transition-driven row must carry a non-empty, operation-derived note"
        );

        let manual_rows = movement_rows(svc.readers.clone(), cart_manual.id).await;
        assert_eq!(manual_rows.len(), 1);
        assert_eq!(
            manual_rows[0].2, None,
            "a plain manual PlacePicker edit must keep note == None — exclusive to transition_in_tx"
        );
    })
    .await
    .expect(
        "place_movements_cartridge_transition_note_distinguishes_from_manual exceeded 30 s budget",
    );
}
