//! Phase 41.2 Plan 06: `PlaceService::group_members` — состав группы строками
//! содержимого места (D-19). Реальный `AppCtx`; имена вымышленные.
//! Префикс тестов: `members_`.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::groups::GroupCreateDto;
use trackly_core::auth::{Identity, Role};
use trackly_infra::error_conversions::map_rusqlite;

async fn make_test_ctx() -> (AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let paths =
        trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("resolve paths");
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    });
    let ctx = AppCtx::build(paths, config, log_guard.expect("log guard"))
        .await
        .expect("build ctx");
    (ctx, dir)
}

fn admin() -> Identity {
    Identity::trusted_admin()
}

async fn seed_place(ctx: &AppCtx, name: &str) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, parent_id, is_storage, created_at_utc, \
                 updated_at_utc, version) VALUES ('room', ?1, NULL, 0, 1700000000, 1700000000, 1)",
                params![name],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

async fn seed_device(ctx: &AppCtx, type_id: i64, name: &str, inv: &str, place: i64) -> i64 {
    let (name, inv) = (name.to_string(), inv.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, inventory_number, place_id, status_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, 1, 1700000000, 1700000000, 1)",
                params![type_id, name, inv, place],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
}

/// USB-принтер (сетевой в группу войти не может): хост — обычное устройство.
async fn make_printer(ctx: &AppCtx, name: &str, inv: &str, place: i64) -> i64 {
    let id = seed_device(ctx, 2, name, inv, place).await;
    let host = seed_device(ctx, 1, &format!("Хост {name}"), &format!("{inv}-H"), place).await;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO printers (device_id, ip_address, usb_host_device_id, \
                 created_at_utc, updated_at_utc) VALUES (?1, NULL, ?2, 1700000000, 1700000000)",
                params![id, host],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("seed printer");
    id
}

async fn container_type(ctx: &AppCtx) -> i64 {
    ctx.writer
        .execute(|conn| {
            let found: Option<i64> = conn
                .query_row(
                    "SELECT id FROM group_types WHERE behavior = 'container' LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .ok();
            if let Some(id) = found {
                return Ok(id);
            }
            conn.execute(
                "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc) \
                 VALUES ('members_t', 'АРМ', 'container', 1700000000, 1700000000)",
                [],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("container type")
}

async fn make_group(
    ctx: &AppCtx,
    name: &str,
    place: Option<i32>,
) -> trackly_app::dto::groups::GroupDto {
    let type_id = container_type(ctx).await;
    ctx.groups
        .create_group(
            &admin(),
            GroupCreateDto {
                type_id,
                name: Some(name.to_string()),
                place_id: place,
                source_device_id: None,
            },
        )
        .await
        .expect("create_group")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn members_flat_device_and_printer_rows_ordered_with_path_and_status() {
    let (ctx, _dir) = make_test_ctx().await;
    let place = seed_place(&ctx, "Кабинет 214").await;
    let root = make_group(&ctx, "АРМ #3", Some(place as i32)).await;
    let inner = make_group(&ctx, "Периферия #1", None).await;

    let monitor = seed_device(&ctx, 1, "Монитор #2", "INV-001", place).await;
    let block = seed_device(&ctx, 1, "Системный блок #4", "INV-002", place).await;
    let printer = make_printer(&ctx, "Принтер #7", "INV-003", place).await;
    ctx.groups
        .add_devices(&admin(), root.id, vec![block, monitor])
        .await
        .unwrap();
    ctx.groups
        .add_devices(&admin(), inner.id, vec![printer])
        .await
        .unwrap();
    let v = ctx
        .groups
        .get_group(&admin(), inner.id)
        .await
        .unwrap()
        .version;
    ctx.groups
        .set_parent(&admin(), inner.id, v, Some(root.id))
        .await
        .unwrap();

    let rows = ctx.places.group_members(&admin(), root.id).await.unwrap();
    let got: Vec<(&str, &str)> = rows
        .iter()
        .map(|r| (r.kind.as_str(), r.name.as_str()))
        .collect();
    // плоско, ORDER BY name, id: вложенная группа не отдельной строкой
    assert_eq!(
        got,
        vec![
            ("device", "Монитор #2"),
            ("printer", "Принтер #7"),
            ("device", "Системный блок #4"),
        ]
    );
    for r in &rows {
        assert!(
            r.full_path.contains("Кабинет 214"),
            "путь места заполнен: {r:?}"
        );
        assert!(
            r.status_name.is_some() && r.status_code.is_some(),
            "статус заполнен: {r:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn members_empty_and_missing_group_give_empty_list() {
    let (ctx, _dir) = make_test_ctx().await;
    let empty = make_group(&ctx, "Пустой АРМ #1", None).await;
    assert!(ctx
        .places
        .group_members(&admin(), empty.id)
        .await
        .unwrap()
        .is_empty());
    assert!(ctx
        .places
        .group_members(&admin(), 987_654)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn members_second_call_after_removal_excludes_removed_device() {
    let (ctx, _dir) = make_test_ctx().await;
    let place = seed_place(&ctx, "Кабинет 3").await;
    let g = make_group(&ctx, "АРМ #5", Some(place as i32)).await;
    let a = seed_device(&ctx, 1, "Монитор #5", "INV-010", place).await;
    let b = seed_device(&ctx, 1, "Мышь #5", "INV-011", place).await;
    ctx.groups
        .add_devices(&admin(), g.id, vec![a, b])
        .await
        .unwrap();
    assert_eq!(
        ctx.places
            .group_members(&admin(), g.id)
            .await
            .unwrap()
            .len(),
        2
    );

    ctx.groups
        .remove_devices(&admin(), g.id, vec![a])
        .await
        .unwrap();
    let rows = ctx.places.group_members(&admin(), g.id).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, b);
    assert!(rows.iter().all(|r| r.id != a));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn members_requires_read_places_role() {
    let (ctx, _dir) = make_test_ctx().await;
    let employee = Identity {
        user_id: Some(77),
        role: Role::Employee,
    };
    let err = ctx.places.group_members(&employee, 1).await.unwrap_err();
    assert!(
        matches!(err, trackly_core::error::AppError::Forbidden),
        "got {err:?}"
    );
}
