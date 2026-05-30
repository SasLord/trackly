//! OrganizationService integration tests — Phase 3 Plan 04 Task 1.
//!
//! Covers:
//!   - first_run_creates_placeholder
//!   - read_returns_existing
//!   - read_corrupt_json_returns_validation
//!   - logo_path_traversal_rejected
//!   - logo_not_existing_returns_none

use std::sync::Arc;
use std::time::Duration;

use tempfile::TempDir;
use trackly_app::services::{OrgData, OrganizationService};
use trackly_core::error::AppError;
use trackly_infra::Paths;

fn make_service() -> (OrganizationService, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let paths = Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("resolve paths");
    let svc = OrganizationService::new(Arc::new(paths));
    (svc, dir)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn first_run_creates_placeholder() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, dir) = make_service();
        let org_path = dir.path().join("org.json");
        assert!(!org_path.exists(), "precondition: org.json should not exist");

        let org = svc.read().await.expect("read");
        assert_eq!(org.name, "Ваша организация");
        assert!(org_path.exists(), "org.json placeholder must have been written");
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_returns_existing() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, dir) = make_service();
        let custom = OrgData {
            name: "ООО Ромашка".into(),
            inn: "7700000000".into(),
            kpp: "770001001".into(),
            address: "г. Москва, ул. Ленина, 1".into(),
            logo_path: String::new(),
        };
        let json = serde_json::to_string_pretty(&custom).unwrap();
        std::fs::write(dir.path().join("org.json"), json).expect("write custom");

        let got = svc.read().await.expect("read");
        assert_eq!(got, custom);
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_corrupt_json_returns_validation() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, dir) = make_service();
        std::fs::write(dir.path().join("org.json"), "{ not valid json ").expect("write");
        let err = svc.read().await.expect_err("should fail");
        match err {
            AppError::Validation { field, .. } => assert_eq!(field, "org.json"),
            other => panic!("expected Validation, got {other:?}"),
        }
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logo_path_traversal_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        // Setup: создать lure-target /tmp файл, на который ../ резолвится
        let outside = TempDir::new().expect("outside tempdir");
        let lure = outside.path().join("lure.png");
        std::fs::write(&lure, b"PNG").expect("write lure");

        // Относительный путь от exe_dir в "outside" tempdir
        let traversal = lure.to_string_lossy().to_string();
        let org = OrgData {
            name: "X".into(),
            inn: "1".into(),
            kpp: "2".into(),
            address: "A".into(),
            logo_path: traversal,
        };
        let err = svc
            .safe_logo_canonical(&org)
            .await
            .expect_err("traversal should be rejected");
        match err {
            AppError::Validation { field, message } => {
                assert_eq!(field, "org.logo_path");
                assert!(
                    message.contains("вне рабочей папки"),
                    "expected mitigation message, got: {message}"
                );
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logo_not_existing_returns_none() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let org = OrgData {
            name: "X".into(),
            inn: "1".into(),
            kpp: "2".into(),
            address: "A".into(),
            logo_path: "missing.png".into(),
        };
        let got = svc
            .safe_logo_canonical(&org)
            .await
            .expect("non-existing logo не должен быть ошибкой");
        assert!(got.is_none(), "missing logo → None");
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logo_empty_path_returns_none() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let org = OrgData::placeholder();
        let got = svc.safe_logo_canonical(&org).await.expect("empty path ok");
        assert!(got.is_none());
    })
    .await
    .expect("timeout");
}
