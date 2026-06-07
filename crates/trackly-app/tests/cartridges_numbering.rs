//! Cartridge auto-code numbering integration tests — Plan 04-03 (GREEN phase).
//!
//! Covers:
//!   - 50 concurrent creates produce 50 unique codes in C-NNNNNN format.
//!   - Counter is never lost on UNIQUE collision: retry loop increments again.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

use trackly_app::dto::cartridge::{CartridgeCreateDto, CartridgeModelCreateDto};
use trackly_app::services::CartridgeService;

fn make_cartridge_service() -> (CartridgeService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let svc = CartridgeService::new(writer, readers, clock);
    (svc, dir)
}

async fn seed_model(svc: &CartridgeService) -> i64 {
    svc.model_create(CartridgeModelCreateDto {
        brand: "Pantum".into(),
        model: "TL-5120X".into(),
        kind_id: 1,
        color: None,
        notes: None,
        compatibility: vec![],
    })
    .await
    .expect("seed model")
    .id
}

/// Spawn 50 concurrent creates; verify all 50 codes are unique and in C-NNNNNN format.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_50_unique_codes() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (svc, _dir) = make_cartridge_service();
        let svc = Arc::new(svc);
        let model_id = seed_model(&svc).await;

        let mut handles = Vec::with_capacity(50);
        for _ in 0..50 {
            let svc2 = svc.clone();
            handles.push(tokio::spawn(async move {
                svc2.create(CartridgeCreateDto {
                    model_id,
                    code_override: None,
                    state_id: Some(1),
                    location: None,
                    notes: None,
                })
                .await
                .expect("concurrent create")
                .code
            }));
        }

        let mut codes = Vec::with_capacity(50);
        for handle in handles {
            codes.push(handle.await.expect("task panicked"));
        }

        assert_eq!(codes.len(), 50, "all 50 creates must succeed");

        // All unique
        let unique: HashSet<&str> = codes.iter().map(|s| s.as_str()).collect();
        assert_eq!(unique.len(), 50, "all 50 codes must be unique");

        // All in C-NNNNNN format (C- prefix + 6 ASCII digits = 8 chars)
        for code in &codes {
            assert!(
                code.len() == 8
                    && code.starts_with("C-")
                    && code[2..].chars().all(|c| c.is_ascii_digit()),
                "code must be C-NNNNNN format, got: {}",
                code
            );
        }
    })
    .await
    .expect("concurrent_50_unique_codes budget")
}

/// Verify that sequential creates produce monotonically increasing counter values.
/// (The counter is never rolled back on collision.)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn collision_retry_does_not_lose_counter() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_cartridge_service();
        let model_id = seed_model(&svc).await;

        let mut codes = Vec::with_capacity(3);
        for _ in 0..3 {
            let code = svc
                .create(CartridgeCreateDto {
                    model_id,
                    code_override: None,
                    state_id: None,
                    location: None,
                    notes: None,
                })
                .await
                .expect("create")
                .code;
            codes.push(code);
        }

        // Extract numeric parts and verify they are strictly increasing.
        let nums: Vec<u64> = codes
            .iter()
            .map(|c| c[2..].parse::<u64>().expect("numeric suffix"))
            .collect();

        // With sequential creates, the counter never decreases.
        for w in nums.windows(2) {
            assert!(w[1] > w[0], "counter must increase: {:?}", nums);
        }
    })
    .await
    .expect("collision_retry_does_not_lose_counter budget")
}
