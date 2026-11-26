//! Помощники для тестов ядра мессенджера (Phase 43). Имена вымышленные.
#![allow(dead_code)]

use rusqlite::params;
use tokio::sync::broadcast::{error::TryRecvError, Receiver};
use trackly_app::context::AppCtx;
use trackly_app::services::im_hub::ImSignal;
use trackly_core::auth::{Identity, Role};
use trackly_infra::error_conversions::map_rusqlite;

pub fn ident(id: i64, role: Role) -> Identity {
    Identity {
        user_id: Some(id),
        role,
    }
}

pub async fn seed_user(ctx: &AppCtx, login: &str, full: &str, role: &str) -> i64 {
    let (login, full, role) = (login.to_string(), full.to_string(), role.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, role, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, ?3, 1700000000, 1700000000)",
                params![login, full, role],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed user")
}

/// Настоящий AppCtx во временном каталоге; каталог живёт, пока жив возвращённый TempDir.
pub async fn build_ctx() -> (AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().unwrap();
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).unwrap();
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    });
    let ctx = AppCtx::build(paths, config, log_guard.unwrap())
        .await
        .unwrap();
    (ctx, dir)
}

/// Все накопленные сигналы без ожидания.
pub fn drain(rx: &mut Receiver<ImSignal>) -> Vec<ImSignal> {
    let mut v = Vec::new();
    loop {
        match rx.try_recv() {
            Ok(s) => v.push(s),
            Err(TryRecvError::Empty) | Err(TryRecvError::Closed) => return v,
            Err(TryRecvError::Lagged(_)) => continue,
        }
    }
}

pub async fn count_messages(ctx: &AppCtx, conv: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM im_messages WHERE conversation_id = ?1",
                [conv],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .unwrap()
}

pub async fn last_seq(ctx: &AppCtx, conv: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT last_seq FROM im_conversations WHERE id = ?1",
                [conv],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .unwrap()
}
