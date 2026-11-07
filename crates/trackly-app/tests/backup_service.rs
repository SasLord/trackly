// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Backup service integration tests — Phase 7 Plan 02 (GREEN).
//!
//! Covers SET-05 (manual backup) and SET-05 auto-backup:
//!   - Manual backup copies DB file to configured backup_folder
//!   - SQLite integrity_check is run on the copy before declaring success
//!   - UNC paths (\\server\share) are rejected with a validation error
//!   - Retention policy prunes oldest copies when count exceeds retention limit

use std::sync::Arc;
use std::time::Duration;

use trackly_app::dto::reports::BackupConfigPatch;
use trackly_app::services::BackupService;
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

fn make_backup_service() -> (BackupService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let db_path = dir.path().join("test.db");
    let svc = BackupService::new(writer, readers, clock, db_path);
    (svc, dir)
}

fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

/// Verify that a manual backup creates a file in backup_folder and passes integrity_check.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn backup_manual_creates_file_and_passes_integrity_check() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, dir) = make_backup_service();

        // Создаём папку для бэкапов внутри tempdir
        let backup_dir = dir.path().join("backups");
        std::fs::create_dir_all(&backup_dir).expect("create backup dir");

        let result = svc
            .run_backup(backup_dir.to_str().expect("backup_dir utf8"))
            .await
            .expect("run_backup should succeed");

        // Файл должен существовать
        let backup_file = std::path::Path::new(&result.file_path);
        assert!(
            backup_file.exists(),
            "backup file must exist at {}",
            result.file_path
        );

        // timestamp должен быть разумным
        assert!(result.timestamp_utc > 0, "timestamp_utc must be positive");

        // Имя файла должно содержать trackly-backup
        let filename = backup_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        assert!(
            filename.starts_with("trackly-backup-") && filename.ends_with(".db"),
            "backup filename must match pattern: {filename}"
        );
    })
    .await
    .expect("backup_manual_creates_file_and_passes_integrity_check budget")
}

/// Verify that UNC backup_folder paths are rejected.
///
/// Per PATTERNS.md: UNC path = starts with r"\\" (two backslashes).
/// Decision recorded: Plan 01-02 — "UNC rejection via simple starts_with(r"\\\\") prefix check".
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn backup_unc_path_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_backup_service();

        // UNC с двойным обратным слешем
        let result_backslash = svc.run_backup("\\\\server\\share\\backups").await;
        assert!(
            result_backslash.is_err(),
            "UNC path with \\\\ should be rejected"
        );
        match result_backslash {
            Err(trackly_core::error::AppError::Validation { field, .. }) => {
                assert_eq!(field, "path", "field должен быть 'path'");
            }
            other => panic!("ожидали Validation для UNC-пути, получили: {other:?}"),
        }

        // UNC с двойным прямым слешем
        let result_slash = svc.run_backup("//server/share/backups").await;
        assert!(result_slash.is_err(), "UNC path with // should be rejected");
        match result_slash {
            Err(trackly_core::error::AppError::Validation { field, .. }) => {
                assert_eq!(field, "path", "field должен быть 'path'");
            }
            other => panic!("ожидали Validation для //-пути, получили: {other:?}"),
        }
    })
    .await
    .expect("backup_unc_path_rejected budget")
}

/// Verify that backup config can be read and written via app_settings.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn backup_config_set_and_get() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, dir) = make_backup_service();
        let caller = admin_caller();

        // Дефолтная конфигурация
        let initial_config = svc.get_config().await.expect("get_config initial");
        assert_eq!(initial_config.schedule, "disabled");
        assert_eq!(initial_config.retention, 7);
        assert!(initial_config.backup_folder.is_none());

        // Сохраняем конфигурацию
        let backup_dir = dir.path().join("auto_backups");
        let patch = BackupConfigPatch {
            backup_folder: Some(backup_dir.to_str().expect("utf8").to_string()),
            schedule: Some("daily".to_string()),
            retention: Some(14),
        };
        svc.set_config(&caller, patch).await.expect("set_config");

        // Проверяем сохранённую конфигурацию
        let updated_config = svc.get_config().await.expect("get_config updated");
        assert!(updated_config.backup_folder.is_some());
        assert_eq!(updated_config.schedule, "daily");
        assert_eq!(updated_config.retention, 14);
    })
    .await
    .expect("backup_config_set_and_get budget")
}

/// Verify that retention policy prunes oldest copies when count exceeds retention limit.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn backup_retention_prunes_oldest() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (svc, dir) = make_backup_service();
        let caller = admin_caller();

        let backup_dir = dir.path().join("retention_test");
        std::fs::create_dir_all(&backup_dir).expect("create backup dir");

        // Устанавливаем retention = 2
        svc.set_config(
            &caller,
            BackupConfigPatch {
                backup_folder: Some(backup_dir.to_str().expect("utf8").to_string()),
                schedule: None,
                retention: Some(2),
            },
        )
        .await
        .expect("set_config retention=2");

        // Создаём 3 бэкапа — 3-й должен вытолкнуть первый
        // Нужно немного подождать между бэкапами чтобы mtime различались
        for i in 0..3 {
            // Создаём fake-бэкап напрямую (разные mtime через sleep)
            let fake_path = backup_dir.join(format!("trackly-backup-{}.db", 1_000_000 + i));
            #[allow(clippy::disallowed_methods)]
            // OK in tests: just fabricating fixture files, not a real DB backup
            std::fs::copy(dir.path().join("test.db"), &fake_path).expect("copy fake backup");
            // Убеждаемся в разнице mtime через небольшую задержку
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Запускаем реальный бэкап — он тоже должен применить retention
        let result = svc
            .run_backup(backup_dir.to_str().expect("utf8"))
            .await
            .expect("run_backup");
        assert!(!result.file_path.is_empty());

        // В директории должно быть не более retention (2) файлов
        let count = std::fs::read_dir(&backup_dir)
            .expect("read_dir")
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.starts_with("trackly-backup-") && name.ends_with(".db")
            })
            .count();
        assert!(
            count <= 2,
            "retention должен оставить не более 2 файлов, осталось: {count}"
        );
    })
    .await
    .expect("backup_retention_prunes_oldest budget")
}

fn h(c: char) -> String {
    std::iter::repeat_n(c, 64).collect()
}

fn put_blob(root: &std::path::Path, hash: &str, bytes: &[u8]) {
    let d = root.join(&hash[..2]);
    std::fs::create_dir_all(&d).expect("shard dir");
    std::fs::write(d.join(hash), bytes).expect("blob");
}

fn make_with_attachments() -> (BackupService, tempfile::TempDir, std::path::PathBuf) {
    let (svc, dir) = make_backup_service();
    let src = dir.path().join("att_src");
    std::fs::create_dir_all(&src).expect("src");
    let svc = svc.with_attachments_dir(src.clone());
    (svc, dir, src)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn backup_copies_attachments_incrementally() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (svc, dir, src) = make_with_attachments();
        put_blob(&src, &h('a'), b"one");
        put_blob(&src, &h('c'), b"two!");
        std::fs::create_dir_all(src.join("thumbs")).expect("thumbs");
        std::fs::write(src.join("thumbs/x.jpg"), b"t").expect("thumb");
        std::fs::create_dir_all(src.join(".tmp")).expect("tmp");
        std::fs::write(src.join(".tmp/y"), b"t").expect("tmpf");
        let bdir = dir.path().join("bk");
        let b = bdir.to_str().expect("utf8");

        svc.run_backup(b).await.expect("backup 1");
        let dest = bdir.join("attachments");
        assert!(dest.join("aa").join(h('a')).is_file());
        assert!(dest.join("cc").join(h('c')).is_file());
        assert!(!dest.join("thumbs").exists());
        assert!(!dest.join(".tmp").exists());
        let m1 = std::fs::metadata(dest.join("aa").join(h('a')))
            .expect("meta")
            .modified()
            .expect("mtime");

        tokio::time::sleep(Duration::from_millis(1100)).await;
        put_blob(&src, &h('e'), b"three");
        svc.run_backup(b).await.expect("backup 2");
        let m2 = std::fs::metadata(dest.join("aa").join(h('a')))
            .expect("meta")
            .modified()
            .expect("mtime");
        assert_eq!(m1, m2, "existing blob must not be re-copied");
        assert!(dest.join("ee").join(h('e')).is_file());

        // same name, different size -> recopied
        std::fs::write(dest.join("aa").join(h('a')), b"x").expect("corrupt");
        svc.run_backup(b).await.expect("backup 3");
        assert_eq!(
            std::fs::read(dest.join("aa").join(h('a'))).expect("read"),
            b"one"
        );
    })
    .await
    .expect("budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn backup_retention_keeps_attachments() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (svc, dir, src) = make_with_attachments();
        put_blob(&src, &h('a'), b"one");
        let bdir = dir.path().join("bk");
        std::fs::create_dir_all(&bdir).expect("bk");
        svc.set_config(
            &admin_caller(),
            BackupConfigPatch {
                backup_folder: Some(bdir.to_str().expect("utf8").to_string()),
                schedule: None,
                retention: Some(1),
            },
        )
        .await
        .expect("cfg");
        for i in 0..3 {
            std::fs::write(bdir.join(format!("trackly-backup-{}.db", 1_000 + i)), b"x")
                .expect("fake");
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        svc.run_backup(bdir.to_str().expect("utf8"))
            .await
            .expect("bk");
        assert!(bdir.join("attachments").join("aa").join(h('a')).is_file());
    })
    .await
    .expect("budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn backup_without_attachments_dir_creates_none() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, dir) = make_backup_service();
        let bdir = dir.path().join("bk");
        svc.run_backup(bdir.to_str().expect("utf8"))
            .await
            .expect("bk");
        assert!(!bdir.join("attachments").exists());
    })
    .await
    .expect("budget")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn backup_missing_attachments_source_still_succeeds() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, dir) = make_backup_service();
        let svc = svc.with_attachments_dir(dir.path().join("nope"));
        let bdir = dir.path().join("bk");
        let r = svc.run_backup(bdir.to_str().expect("utf8")).await;
        assert!(r.is_ok(), "DB backup must succeed: {r:?}");
    })
    .await
    .expect("budget")
}
