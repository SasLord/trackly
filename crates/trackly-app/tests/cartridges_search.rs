//! Cartridge FTS + LIKE search integration tests — Plan 04-03 (GREEN phase).
//!
//! Covers:
//!   - search_by_code: LIKE match on C-NNNNNN code
//!   - search_by_model_brand: FTS/LIKE on cartridge_models.brand + model
//!   - search_by_place: place-path substring match (D-29/PLC-05, place-tree
//!     rename) — Rust-side `place_full_paths` scan, not SQL LIKE on a dropped
//!     `cartridges.location` column
//!   - empty_query_returns_all: empty / whitespace-only query falls back to list

use std::sync::Arc;
use std::time::Duration;

use trackly_infra::clock_impl::SystemClock;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;

use trackly_app::dto::cartridge::{CartridgeCreateDto, CartridgeFilter, CartridgeModelCreateDto};
use trackly_app::services::CartridgeService;

fn make_cartridge_service() -> (CartridgeService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let svc = CartridgeService::new(writer, readers, clock);
    (svc, dir)
}

async fn seed_model(svc: &CartridgeService, brand: &str, model_name: &str) -> i64 {
    svc.model_create(CartridgeModelCreateDto {
        brand: brand.into(),
        model: model_name.into(),
        kind_id: 1,
        color: None,
        notes: None,
        compatibility: vec![],
    })
    .await
    .expect("seed model")
    .id
}

/// Seed a real `places` row — FK-valid `place_id` (`REFERENCES places(id)`,
/// V038) — mirrors the `seed_place()` precedent from Plan 09's
/// `cartridges_sqlite.rs` inline test module.
async fn seed_place(svc: &CartridgeService, name: &str) -> i64 {
    let name = name.to_string();
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, is_storage, created_at_utc, updated_at_utc, version) \
                 VALUES ('room', ?1, 0, ?2, ?2, 1)",
                rusqlite::params![name, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

/// Create a cartridge whose `place_id` points at a real `places` row named
/// `place_name` — search matches the place's `full_path` substring, not a
/// freeform `location` string (D-29/PLC-05).
async fn create_with_place(svc: &CartridgeService, model_id: i64, place_name: &str) -> String {
    let place_id = seed_place(svc, place_name).await;
    svc.create(CartridgeCreateDto {
        model_id,
        code_override: None,
        state_id: None,
        place_id: Some(place_id),
        notes: None,
    })
    .await
    .expect("create")
    .code
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_by_code() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc, "Pantum", "TL-5120X").await;

        // Create a cartridge to get its auto-code.
        let code = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: None,
                state_id: None,
                place_id: None,
                notes: None,
            })
            .await
            .expect("create")
            .code;

        // Search by the first 5 chars of the code (e.g. "C-000").
        let prefix = &code[..5];
        let result = svc
            .search(prefix.to_string(), CartridgeFilter::default())
            .await
            .expect("search");

        assert!(
            result.items.iter().any(|c| c.code == code),
            "search by code prefix must find the cartridge"
        );
    })
    .await
    .expect("search_by_code budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_by_model_brand() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc, "KyoceraUnique", "TK-8375K").await;

        let code = svc
            .create(CartridgeCreateDto {
                model_id,
                code_override: None,
                state_id: None,
                place_id: None,
                notes: None,
            })
            .await
            .expect("create")
            .code;

        // Search by brand prefix
        let result = svc
            .search("KyoceraUnique".to_string(), CartridgeFilter::default())
            .await
            .expect("search by brand");

        assert!(
            result.items.iter().any(|c| c.code == code),
            "search by brand must find the cartridge"
        );
    })
    .await
    .expect("search_by_model_brand budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_by_place() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc, "Canon", "FX-9").await;

        let unique_place_name = "Уникальный склад 7Б";
        let code = create_with_place(&svc, model_id, unique_place_name).await;

        let result = svc
            .search("7Б".to_string(), CartridgeFilter::default())
            .await
            .expect("search by place");

        assert!(
            result.items.iter().any(|c| c.code == code),
            "search by place-path substring must find the cartridge"
        );
    })
    .await
    .expect("search_by_place budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn empty_query_returns_all() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc, "Epson", "S050418").await;

        // Create 3 cartridges.
        for _ in 0..3 {
            svc.create(CartridgeCreateDto {
                model_id,
                code_override: None,
                state_id: None,
                place_id: None,
                notes: None,
            })
            .await
            .expect("create");
        }

        // Empty query falls back to list — should return all 3.
        let result = svc
            .search("".to_string(), CartridgeFilter::default())
            .await
            .expect("search empty");
        assert_eq!(
            result.items.len(),
            3,
            "empty query must return all 3 cartridges"
        );

        // Whitespace-only query also falls back.
        let result2 = svc
            .search("   ".to_string(), CartridgeFilter::default())
            .await
            .expect("search whitespace");
        assert_eq!(
            result2.items.len(),
            3,
            "whitespace query must return all 3 cartridges"
        );
    })
    .await
    .expect("empty_query_returns_all budget")
}
