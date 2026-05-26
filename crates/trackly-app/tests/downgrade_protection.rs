//! Phase 1 success criterion #4: newer-DB error + byte-identical file after rejection.
//!
//! Workflow:
//! 1. Pre-create tempfile DB; open writer, apply pragmas, run migrations.
//! 2. Set `PRAGMA user_version = 999` manually (force "newer than binary").
//! 3. Drop conn so file flushes.
//! 4. SHA256 of `.db` (+ `.db-wal` if present) → `before`.
//! 5. Call `AppCtx::build(...)`; expect `AppError::DatabaseFromNewerVersion { binary: 12, file: 999 }`.
//! 6. SHA256 again → `after`. Assert `before == after` via single `String == String`.
//!
//! The probe-read pattern in `AppCtx::build` (W4) guarantees the assertion holds:
//! read-only open does NOT touch WAL or main file.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use sha2::{Digest, Sha256};

use trackly_app::context::AppCtx;
use trackly_core::error::AppError;
use trackly_infra::db::{migrations, pragmas};
use trackly_infra::{AppConfig, Paths};

/// SHA256 хэш main `.db` файла + `.db-wal` файла (если существует).
/// Один digest на оба, чтобы единый `String == String` сравнил их вместе.
fn snapshot(db_path: &Path) -> String {
    let mut h = Sha256::new();
    let db_bytes = std::fs::read(db_path).expect("read .db");
    h.update(&db_bytes);
    let wal = db_path.with_extension("db-wal");
    if wal.exists() {
        let wal_bytes = std::fs::read(&wal).expect("read .db-wal");
        h.update(&wal_bytes);
    }
    format!("{:x}", h.finalize())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn appctx_build_rejects_newer_db_and_leaves_file_byte_identical() {
    // Step 1-3: pre-create DB with user_version = 999.
    let sandbox = tempfile::TempDir::new().expect("tempdir");
    let db_path: PathBuf = sandbox.path().join("trackly.db");

    {
        let mut conn = Connection::open(&db_path).expect("open writer");
        pragmas::apply_writer_pragmas(&conn).expect("apply writer pragmas");
        migrations::run(&mut conn).expect("run migrations");
        // Force a newer user_version.
        conn.pragma_update(None, "user_version", 999_u32)
            .expect("set user_version=999");
        // Drop closes the conn and flushes the WAL header.
        drop(conn);
    }

    // Sanity: confirm file & user_version setup.
    {
        let probe = Connection::open(&db_path).expect("re-open probe");
        let v: i64 = probe
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .expect("read user_version");
        assert_eq!(v, 999, "setup failed: user_version should be 999");
        drop(probe);
    }

    // Step 4: snapshot BEFORE.
    let before = snapshot(&db_path);

    // Step 5: invoke AppCtx::build with pointing config.
    let paths = Paths::resolve_for_exe_dir(sandbox.path().to_path_buf()).expect("resolve paths");
    let mut config = AppConfig::default();
    // Force the config to point at our tempfile.
    config.paths.db_path = db_path.display().to_string();

    // No-op WorkerGuard (writes to /dev/null).
    let (non_blocking, log_guard) = tracing_appender::non_blocking(std::io::sink());
    let _ = non_blocking;

    let result = AppCtx::build(paths, config, log_guard).await;
    let err = match result {
        Ok(_) => panic!("AppCtx::build should fail for user_version=999"),
        Err(e) => e,
    };

    // Downcast anyhow → AppError variant check.
    let app_err = err
        .downcast_ref::<AppError>()
        .expect("error should be AppError");
    match app_err {
        AppError::DatabaseFromNewerVersion { binary, file } => {
            assert_eq!(*binary, 13, "binary should be 13 (max_known_version)");
            assert_eq!(*file, 999, "file should be 999 (forced user_version)");
        }
        other => panic!("expected DatabaseFromNewerVersion, got {other:?}"),
    }

    // Step 6: snapshot AFTER and assert byte-identity.
    let after = snapshot(&db_path);
    assert_eq!(
        before, after,
        "DB file (and .db-wal if present) must be byte-identical after a rejected AppCtx::build"
    );
}
