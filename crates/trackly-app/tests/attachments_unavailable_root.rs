//! Phase 41.3 (verification WARNING): an unreachable `attachments_path`
//! (UNC share / removable disk, D-28) must not stop the whole app from
//! starting — only attachments degrade (D-20 "unavailable").
//!
//! The path is made uncreatable by placing it under a regular file.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn app_starts_when_attachments_dir_cannot_be_created() -> anyhow::Result<()> {
    let dir = tempfile::TempDir::new()?;
    let blocker = dir.path().join("not-a-dir");
    std::fs::write(&blocker, b"x")?;
    let unreachable = blocker.join("attachments");

    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf())?;
    let mut config = trackly_infra::AppConfig::default();
    config.paths.attachments_path = unreachable.to_string_lossy().to_string();
    let log_guard = trackly_app::logging::init(&paths, &config)?;

    let ctx = trackly_app::context::AppCtx::build(paths, config, log_guard)
        .await
        .expect("startup must survive an unreachable attachments directory");

    // The rest of the app works.
    ctx.devices
        .list(Default::default(), Default::default())
        .await
        .expect("devices list");

    // Attachments report the root as unavailable instead of failing.
    let report = ctx.attachments.run_integrity_scan().await?;
    assert_eq!(report.status, "unavailable");

    // An upload cannot start, and says so with an error rather than a panic.
    assert!(ctx.attachments.begin_staging().is_err());
    Ok(())
}
