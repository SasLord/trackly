//! Справочник статусов устройств `device_statuses_list` (Phase 41.1, D-06, D-08).
//!
//! Порядок задаёт `sort_order` (жизненный цикл), а не алфавит. Данные вымышленные.

use trackly_app::context::AppCtx;
use trackly_app::tauri_cmds::devices::build_device_statuses_list;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;

async fn make_test_ctx() -> (AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let paths =
        trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("resolve paths");
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    });
    let log_guard = log_guard.expect("log guard");
    let ctx = AppCtx::build(paths, config, log_guard)
        .await
        .expect("build ctx");
    (ctx, dir)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn statuses_come_in_lifecycle_order_with_labels() {
    let (ctx, _dir) = make_test_ctx().await;
    let list = build_device_statuses_list(&ctx, &Identity::trusted_admin())
        .await
        .expect("list");
    let codes: Vec<&str> = list.iter().map(|s| s.code.as_str()).collect();
    assert_eq!(
        codes,
        [
            "на_складе",
            "в_работе",
            "на_ремонте",
            "в_ремонте",
            "на_списание",
            "списано"
        ]
    );
    let waiting = list.iter().find(|s| s.code == "на_ремонте").unwrap();
    assert_eq!(waiting.name, "Ожидает ремонта");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codes_are_non_empty_and_sort_order_strictly_grows() {
    let (ctx, _dir) = make_test_ctx().await;
    let list = build_device_statuses_list(&ctx, &Identity::trusted_admin())
        .await
        .expect("list");
    assert!(!list.is_empty());
    assert!(list
        .iter()
        .all(|s| !s.code.is_empty() && !s.name.is_empty()));
    assert!(
        list.windows(2).all(|w| w[0].sort_order < w[1].sort_order),
        "sort_order должен строго расти: {:?}",
        list.iter().map(|s| s.sort_order).collect::<Vec<_>>()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn employee_is_forbidden_before_db_access() {
    let (ctx, _dir) = make_test_ctx().await;
    let employee = Identity {
        user_id: Some(999_999),
        role: Role::Employee,
    };
    let res = build_device_statuses_list(&ctx, &employee).await;
    assert!(
        matches!(res, Err(AppError::Forbidden)),
        "ожидался Forbidden, получено {res:?}"
    );
}
