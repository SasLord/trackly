//! Integrity scan and admin settings of the attachments store
//! (Phase 41.3 Plan 08, D-20 / D-07). Hashes and names are synthetic.

use std::fs::{self, File};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

#[path = "entities_support/fixture.rs"]
#[allow(dead_code)]
mod fixture;

macro_rules! timed {
    ($body:expr) => {
        tokio::time::timeout(Duration::from_secs(30), $body)
            .await
            .expect("timeout")
    };
}

const HOUR: u64 = 3600;

fn hash(c: char) -> String {
    std::iter::repeat_n(c, 64).collect()
}

fn root(ctx: &AppCtx) -> PathBuf {
    ctx.attachments.store().root().to_path_buf()
}

fn put_blob(ctx: &AppCtx, h: &str, age_hours: u64) -> PathBuf {
    let dir = root(ctx).join(&h[..2]);
    fs::create_dir_all(&dir).unwrap();
    let p = dir.join(h);
    fs::write(&p, b"synthetic").unwrap();
    set_age(&p, age_hours);
    p
}

fn set_age(p: &PathBuf, age_hours: u64) {
    let f = File::options().write(true).open(p).unwrap();
    f.set_modified(SystemTime::now() - Duration::from_secs(age_hours * HOUR))
        .unwrap();
}

async fn seed_row(ctx: &AppCtx, h: &str, deleted: bool) {
    let h = h.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO attachments (parent_kind, parent_id, original_name, mime, \
                 size_bytes, content_hash, created_at_utc, deleted_at_utc) \
                 VALUES ('request', 1, 'doc.pdf', 'application/pdf', 9, ?1, 1700000000, ?2)",
                params![h, if deleted { Some(1700000100i64) } else { None }],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("seed row");
}

/// Waits for the startup scan spawned by `AppCtx::build`, so it cannot race
/// with files a test seeds afterwards.
async fn settle(ctx: &AppCtx) {
    for _ in 0..100 {
        if ctx
            .attachments
            .get_report(&fixture::admin())
            .await
            .unwrap()
            .is_some()
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("startup scan did not finish");
}

fn ident(role: Role) -> Identity {
    Identity {
        user_id: Some(1),
        role,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn old_orphan_removed_young_and_live_stay() {
    let (ctx, _d) = fixture::make_test_ctx().await;
    settle(&ctx).await;
    let old = put_blob(&ctx, &hash('a'), 25);
    let young = put_blob(&ctx, &hash('b'), 1);
    let live = put_blob(&ctx, &hash('c'), 48);
    seed_row(&ctx, &hash('c'), false).await;
    let r = timed!(ctx.attachments.run_integrity_scan()).unwrap();
    assert!(!old.exists());
    assert!(young.exists());
    assert!(live.exists());
    assert_eq!(r.orphans_removed, 1);
    assert_eq!(r.orphans_young, 1);
    assert_eq!(r.lost, 0);
    assert_eq!(r.files, 3);
    assert_eq!(r.status, "ok");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn soft_deleted_row_keeps_its_blob() {
    let (ctx, _d) = fixture::make_test_ctx().await;
    settle(&ctx).await;
    let p = put_blob(&ctx, &hash('d'), 100);
    seed_row(&ctx, &hash('d'), true).await;
    let r = timed!(ctx.attachments.run_integrity_scan()).unwrap();
    assert!(p.exists());
    assert_eq!(r.orphans_removed, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stale_tmp_removed_fresh_stays() {
    let (ctx, _d) = fixture::make_test_ctx().await;
    settle(&ctx).await;
    let tmp = root(&ctx).join(".tmp");
    fs::create_dir_all(&tmp).unwrap();
    let (old, fresh) = (tmp.join("old.tmp"), tmp.join("fresh.tmp"));
    fs::write(&old, b"x").unwrap();
    fs::write(&fresh, b"x").unwrap();
    set_age(&old, 30);
    set_age(&fresh, 1);
    timed!(ctx.attachments.run_integrity_scan()).unwrap();
    assert!(!old.exists());
    assert!(fresh.exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn missing_blob_counted_lost_rows_untouched() {
    let (ctx, _d) = fixture::make_test_ctx().await;
    settle(&ctx).await;
    seed_row(&ctx, &hash('e'), false).await;
    let r = timed!(ctx.attachments.run_integrity_scan()).unwrap();
    assert_eq!(r.lost, 1);
    let n: i64 = ctx
        .readers
        .acquire()
        .query_row("SELECT COUNT(*) FROM attachments", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unavailable_root_deletes_nothing() {
    let (ctx, _d) = fixture::make_test_ctx().await;
    settle(&ctx).await;
    seed_row(&ctx, &hash('f'), false).await;
    let moved = root(&ctx).with_extension("away");
    fs::rename(root(&ctx), &moved).unwrap();
    let r = timed!(ctx.attachments.run_integrity_scan()).unwrap();
    assert_eq!(r.status, "unavailable");
    assert_eq!(r.lost, 0);
    assert_eq!(r.orphans_removed, 0);
    fs::rename(&moved, root(&ctx)).unwrap();
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn junk_names_and_symlinks_are_ignored() {
    let (ctx, _d) = fixture::make_test_ctx().await;
    settle(&ctx).await;
    let shard = root(&ctx).join("aa");
    fs::create_dir_all(&shard).unwrap();
    let junk = shard.join("not-a-hash.txt");
    fs::write(&junk, b"x").unwrap();
    set_age(&junk, 100);
    let target = root(&ctx).join("outside.bin");
    fs::write(&target, b"x").unwrap();
    set_age(&target, 100);
    let link = shard.join(hash('a'));
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let r = timed!(ctx.attachments.run_integrity_scan()).unwrap();
    assert!(junk.exists());
    assert!(target.exists());
    assert!(fs::symlink_metadata(&link).is_ok());
    assert_eq!(r.orphans_removed, 0);
    assert_eq!(r.files, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_is_persisted_and_readable() {
    let (ctx, _d) = fixture::make_test_ctx().await;
    settle(&ctx).await;
    let admin = fixture::admin();
    seed_row(&ctx, &hash('1'), true).await;
    put_blob(&ctx, &hash('2'), 30);
    let r = timed!(ctx.attachments.run_integrity_scan()).unwrap();
    let got = timed!(ctx.attachments.get_report(&admin))
        .unwrap()
        .expect("report");
    assert_eq!(got.checked_at_utc, r.checked_at_utc);
    assert_eq!(got.orphans_removed, 1);
    assert_eq!(got.lost, 1);
    assert_eq!(got.deleted_rows, 1);
    assert_eq!(got.deleted_bytes, 9);
    let raw: String = ctx
        .readers
        .acquire()
        .query_row(
            "SELECT value FROM app_settings WHERE key='attachments_integrity_report'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(raw.contains("checkedAtUtc"));
    assert!(timed!(ctx.attachments.get_report(&ident(Role::Manager))).is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_max_bytes_range_and_roles() {
    let (ctx, _d) = fixture::make_test_ctx().await;
    let admin = fixture::admin();
    let mib = 1024 * 1024;
    timed!(ctx.attachments.set_max_bytes(&admin, mib)).unwrap();
    assert_eq!(ctx.attachments.max_bytes().await.unwrap(), mib);
    timed!(ctx.attachments.set_max_bytes(&admin, 100 * mib)).unwrap();
    for bad in [0, 101 * mib] {
        let e = timed!(ctx.attachments.set_max_bytes(&admin, bad)).unwrap_err();
        assert!(matches!(e, AppError::Validation { .. }));
    }
    for role in [Role::Manager, Role::Employee] {
        let e = timed!(ctx.attachments.set_max_bytes(&ident(role), mib)).unwrap_err();
        assert!(matches!(e, AppError::Forbidden), "{e:?}");
    }
    assert_eq!(ctx.attachments.max_bytes().await.unwrap(), 100 * mib);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_limit_returns_path_for_admin_only() {
    let (ctx, _d) = fixture::make_test_ctx().await;
    let l = timed!(ctx.attachments.get_limit(&fixture::admin())).unwrap();
    assert_eq!(l.path, root(&ctx).to_string_lossy());
    assert_eq!(l.ceiling_bytes, 100 * 1024 * 1024);
    assert!(timed!(ctx.attachments.get_limit(&ident(Role::Employee))).is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn startup_scan_runs_in_background() {
    let (ctx, _d) = fixture::make_test_ctx().await;
    // The spawn in AppCtx::build writes the report without anyone calling it.
    let mut found = false;
    for _ in 0..100 {
        if timed!(ctx.attachments.get_report(&fixture::admin()))
            .unwrap()
            .is_some()
        {
            found = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(found, "startup scan never stored a report");
}
