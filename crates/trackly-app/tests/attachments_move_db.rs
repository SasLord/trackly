//! NEW-1 (аудит v1.4 раунд 6): перенос БД обязан забирать вложения с собой.
//!
//! Пустой `attachments_path` означает «рядом с БД», поэтому каталог вложений копируется
//! в `<новая папка>/attachments` ДО переписывания `db_path`. Данные вымышленные.

use std::path::{Path, PathBuf};

use trackly_app::context::AppCtx;
use trackly_app::tauri_cmds::settings_org::build_settings_move_db;
use trackly_core::auth::Identity;

const SHARD: &str = "ab";

fn blob_name() -> String {
    "c".repeat(64)
}

/// Строит `AppCtx` во временной папке; `explicit` — значение `attachments_path`.
async fn make_ctx(dir: &Path, explicit: &str) -> anyhow::Result<AppCtx> {
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.to_path_buf())?;
    let mut config = trackly_infra::AppConfig::default();
    config.paths.attachments_path = explicit.to_string();
    std::fs::write(
        paths.config_file(),
        format!("[paths]\nattachments_path = {explicit:?}\n"),
    )?;
    let log_guard = trackly_app::logging::init(&paths, &config)?;
    Ok(AppCtx::build(paths, config, log_guard).await?)
}

fn seed_blob(att_dir: &Path) -> PathBuf {
    let shard = att_dir.join(SHARD);
    std::fs::create_dir_all(&shard).unwrap();
    let f = shard.join(blob_name());
    std::fs::write(&f, b"fictional attachment bytes").unwrap();
    f
}

fn config_text(ctx: &AppCtx) -> String {
    std::fs::read_to_string(ctx.paths.config_file()).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_db_copies_attachments_when_path_empty() -> anyhow::Result<()> {
    let dir = tempfile::TempDir::new()?;
    let ctx = make_ctx(dir.path(), "").await?;
    seed_blob(&dir.path().join("attachments"));

    let target = dir.path().join("moved");
    std::fs::create_dir_all(&target)?;
    let new_db = target.join("trackly.db");
    build_settings_move_db(
        &ctx,
        &Identity::trusted_admin(),
        new_db.to_string_lossy().to_string(),
    )
    .await?;

    let copied = target.join("attachments").join(SHARD).join(blob_name());
    assert_eq!(std::fs::read(copied)?, b"fictional attachment bytes");
    assert!(config_text(&ctx).contains("moved"), "db_path переписан");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_db_leaves_explicit_attachments_path_alone() -> anyhow::Result<()> {
    let dir = tempfile::TempDir::new()?;
    let explicit = dir.path().join("shared-att");
    std::fs::create_dir_all(&explicit)?;
    let explicit_str = explicit.to_string_lossy().to_string();
    let ctx = make_ctx(dir.path(), &explicit_str).await?;
    seed_blob(&explicit);

    let target = dir.path().join("moved");
    std::fs::create_dir_all(&target)?;
    build_settings_move_db(
        &ctx,
        &Identity::trusted_admin(),
        target.join("trackly.db").to_string_lossy().to_string(),
    )
    .await?;

    assert!(!target.join("attachments").exists());
    let cfg: toml::Value = toml::from_str(&config_text(&ctx))?;
    assert_eq!(
        cfg["paths"]["attachments_path"].as_str(),
        Some(explicit_str.as_str())
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_db_copy_failure_keeps_db_path() -> anyhow::Result<()> {
    let dir = tempfile::TempDir::new()?;
    let ctx = make_ctx(dir.path(), "").await?;
    seed_blob(&dir.path().join("attachments"));

    let target = dir.path().join("moved");
    std::fs::create_dir_all(&target)?;
    // На месте целевого каталога вложений — обычный файл: копия обязана упасть.
    std::fs::write(target.join("attachments"), b"blocker")?;
    let before = config_text(&ctx);

    let res = build_settings_move_db(
        &ctx,
        &Identity::trusted_admin(),
        target.join("trackly.db").to_string_lossy().to_string(),
    )
    .await;

    assert!(res.is_err(), "сбой копии вложений должен вернуть ошибку");
    assert_eq!(config_text(&ctx), before, "конфиг не изменён");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_db_without_source_attachments_succeeds() -> anyhow::Result<()> {
    let dir = tempfile::TempDir::new()?;
    let ctx = make_ctx(dir.path(), "").await?;
    // Исходного каталога вложений нет.
    let _ = std::fs::remove_dir_all(dir.path().join("attachments"));

    let target = dir.path().join("moved");
    std::fs::create_dir_all(&target)?;
    build_settings_move_db(
        &ctx,
        &Identity::trusted_admin(),
        target.join("trackly.db").to_string_lossy().to_string(),
    )
    .await?;
    assert!(config_text(&ctx).contains("moved"));
    Ok(())
}
