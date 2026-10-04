//! Phase 41 Plan 10: журнал пакета группового переноса — 7 строк с общим `batch_id`,
//! история устройства и группы, событие D-30 (`audit_log` вместо строки журнала),
//! картриджи принтера-члена и первое размещение группы без места.
//! Реальный `AppCtx` на временном каталоге; имена вымышленные.
//!
//! Префикс тестов: `journal_`.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::cartridge::{CartridgeCreateDto, CartridgeModelCreateDto};
use trackly_app::dto::number_template::NumberFieldInput;
use trackly_core::auth::Identity;
use trackly_infra::error_conversions::map_rusqlite;

async fn build_ctx_in(dir: &std::path::Path) -> anyhow::Result<AppCtx> {
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.to_path_buf())?;
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    })?;
    AppCtx::build(paths, config, log_guard).await
}

async fn make_test_ctx() -> (AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let ctx = build_ctx_in(dir.path()).await.expect("build ctx");
    (ctx, dir)
}

fn admin() -> Identity {
    Identity::trusted_admin()
}

async fn scalar_i64(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("scalar query")
}

async fn opt_i64(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| {
                r.get::<_, Option<i64>>(0)
            })
            .map_err(map_rusqlite)
        })
        .await
        .expect("opt query")
}

async fn scalar_string(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) -> String {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| {
                r.get::<_, String>(0)
            })
            .map_err(map_rusqlite)
        })
        .await
        .expect("string query")
}

fn int(v: i64) -> rusqlite::types::Value {
    rusqlite::types::Value::Integer(v)
}

async fn type_id_by_code(ctx: &AppCtx, code: &str) -> i64 {
    ctx.group_types
        .list_types(&admin(), false)
        .await
        .expect("list_types")
        .into_iter()
        .find(|t| t.code == code)
        .unwrap_or_else(|| panic!("тип {code} не найден"))
        .id
}

/// Сеет место (`kind` — токен вида места) с необязательным родителем.
async fn seed_place(ctx: &AppCtx, kind: &'static str, name: &str, parent: Option<i64>) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, parent_id, is_storage, created_at_utc, \
                 updated_at_utc, version) VALUES (?1, ?2, ?3, 0, 1700000000, 1700000000, 1)",
                params![kind, name, parent],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

/// Сеет живую группу напрямую (мимо сервиса).
async fn seed_group(
    ctx: &AppCtx,
    type_id: i64,
    name: &str,
    seq: i64,
    place_id: Option<i64>,
    parent: Option<i64>,
) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 1700000000, 1700000000, 1)",
                params![type_id, name, seq, place_id, parent],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group")
}

/// Сеет живое устройство и (при `group_id`) сразу кладёт его в группу.
async fn seed_device(
    ctx: &AppCtx,
    name: &str,
    inv: &str,
    serial: &str,
    place_id: Option<i64>,
    group_id: Option<i64>,
) -> i64 {
    let (name, inv, serial) = (name.to_string(), inv.to_string(), serial.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, inventory_number, serial_number, place_id, \
                 status_id, created_at_utc, updated_at_utc, version) \
                 VALUES (1, ?1, ?2, ?3, ?4, 1, 1700000000, 1700000000, 1)",
                params![name, inv, serial, place_id],
            )
            .map_err(map_rusqlite)?;
            let id = conn.last_insert_rowid();
            if let Some(g) = group_id {
                conn.execute(
                    "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
                     VALUES (?1, ?2, 1700000000)",
                    params![id, g],
                )
                .map_err(map_rusqlite)?;
            }
            Ok(id)
        })
        .await
        .expect("seed device")
}

/// Версия группы напрямую из БД (CAS).
async fn group_version(ctx: &AppCtx, id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT version FROM groups WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

/// Состав для переноса: корневая «АРМ #3» (4 прямых устройства) и вложенная «АРМ #4»
/// (2 устройства), всё на «Складе А». Ids устройств растут: 4-е — последнее прямое.
struct Fixture {
    root: i64,
    nested: i64,
    devices: Vec<i64>,
    place_a: i64,
    place_b: i64,
}

async fn seed_fixture(ctx: &AppCtx) -> Fixture {
    let arm = type_id_by_code(ctx, "workstation").await;
    let place_a = seed_place(ctx, "room", "Склад А", None).await;
    let place_b = seed_place(ctx, "room", "Склад Б", None).await;
    let root = seed_group(ctx, arm, "АРМ #3", 3, Some(place_a), None).await;
    let nested = seed_group(ctx, arm, "АРМ #4", 4, Some(place_a), Some(root)).await;
    let mut devices = Vec::new();
    for i in 0..4 {
        devices.push(
            seed_device(
                ctx,
                &format!("Системный блок {i}"),
                &format!("INV-M-{i}"),
                &format!("SN-M-{i}"),
                Some(place_a),
                Some(root),
            )
            .await,
        );
    }
    for i in 4..6 {
        devices.push(
            seed_device(
                ctx,
                &format!("Монитор {i}"),
                &format!("INV-M-{i}"),
                &format!("SN-M-{i}"),
                Some(place_a),
                Some(nested),
            )
            .await,
        );
    }
    Fixture {
        root,
        nested,
        devices,
        place_a,
        place_b,
    }
}

async fn device_place(ctx: &AppCtx, id: i64) -> Option<i64> {
    opt_i64(
        ctx,
        "SELECT place_id FROM devices WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

fn text(v: &str) -> rusqlite::types::Value {
    rusqlite::types::Value::Text(v.to_string())
}

/// Единственное значение `batch_id` среди строк журнала источника `group`.
async fn single_group_batch(ctx: &AppCtx) -> String {
    let distinct = scalar_i64(
        ctx,
        "SELECT COUNT(DISTINCT batch_id) FROM place_movements WHERE source = 'group'",
        vec![],
    )
    .await;
    assert_eq!(distinct, 1, "у пакета ровно один batch_id");
    scalar_string(
        ctx,
        "SELECT DISTINCT batch_id FROM place_movements WHERE source = 'group'",
        vec![],
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn journal_7_rows() {
    let (ctx, _dir) = make_test_ctx().await;
    let f = seed_fixture(&ctx).await;
    let v = group_version(&ctx, f.root).await;

    let out = ctx
        .groups
        .move_group(&admin(), f.root, v, f.place_b)
        .await
        .expect("move");

    let batch = single_group_batch(&ctx).await;
    assert_eq!(out.batch_id.as_deref(), Some(batch.as_str()));
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE batch_id = ?1",
            vec![text(&batch)]
        )
        .await,
        7,
        "1 строка группы + 6 строк устройств"
    );
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE batch_id = ?1 AND entity_type = 'group'",
            vec![text(&batch)]
        )
        .await,
        1
    );
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE batch_id = ?1 AND entity_type = 'device'",
            vec![text(&batch)]
        )
        .await,
        6
    );
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE batch_id = ?1 AND source = 'group'",
            vec![text(&batch)]
        )
        .await,
        7
    );
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(DISTINCT entity_label) FROM place_movements WHERE batch_id = ?1",
            vec![text(&batch)]
        )
        .await,
        1
    );
    assert_eq!(
        scalar_string(
            &ctx,
            "SELECT DISTINCT entity_label FROM place_movements WHERE batch_id = ?1",
            vec![text(&batch)]
        )
        .await,
        "АРМ #3"
    );
    // group_id — id перенесённой (корневой) группы в КАЖДОЙ строке, вложенная строк не пишет
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE batch_id = ?1 AND group_id = ?2",
            vec![text(&batch), int(f.root)]
        )
        .await,
        7
    );
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'group' AND entity_id = ?1",
            vec![int(f.nested)]
        )
        .await,
        0
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn journal_device_history() {
    let (ctx, _dir) = make_test_ctx().await;
    let f = seed_fixture(&ctx).await;
    let v = group_version(&ctx, f.root).await;
    ctx.groups
        .move_group(&admin(), f.root, v, f.place_b)
        .await
        .expect("move");

    let nested_device = f.devices[5];
    let tl = ctx
        .place_movements
        .get_timeline(&admin(), "device", nested_device)
        .await
        .expect("timeline device");
    assert_eq!(tl.len(), 1);
    assert_eq!(tl[0].group_label.as_deref(), Some("АРМ #3"));
    assert_eq!(tl[0].group_id, Some(f.root));
    assert!(tl[0].batch_id.is_some());
    assert_eq!(tl[0].source, "group");
    assert_eq!(tl[0].from_place_id, f.place_a);
    assert_eq!(tl[0].to_place_id, f.place_b);

    let tl_group = ctx
        .place_movements
        .get_timeline(&admin(), "group", f.root)
        .await
        .expect("timeline group");
    assert_eq!(tl_group.len(), 1, "у группы только своя строка");
    assert_eq!(tl_group[0].entity_type, "group");
    assert_eq!(tl_group[0].entity_id, f.root);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn journal_null_place_d30() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let a = seed_place(&ctx, "room", "Склад А", None).await;
    let b = seed_place(&ctx, "room", "Склад Б", None).await;
    let g = seed_group(&ctx, arm, "АРМ #5", 5, Some(a), None).await;
    let with_place = seed_device(
        &ctx,
        "Системный блок",
        "INV-D-1",
        "SN-D-1",
        Some(a),
        Some(g),
    )
    .await;
    let without_place = seed_device(&ctx, "Монитор", "INV-D-2", "SN-D-2", None, Some(g)).await;
    let v = group_version(&ctx, g).await;

    ctx.groups
        .move_group(&admin(), g, v, b)
        .await
        .expect("move");

    assert_eq!(device_place(&ctx, without_place).await, Some(b));
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE entity_id = ?1",
            vec![int(without_place)]
        )
        .await,
        0,
        "у устройства без места строки журнала нет"
    );
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM audit_log WHERE entity_type = 'device' AND entity_id = ?1 \
             AND action = 'custom:group_place_assigned'",
            vec![int(without_place)]
        )
        .await,
        1
    );
    let payload = scalar_string(
        &ctx,
        "SELECT payload_json FROM audit_log WHERE action = 'custom:group_place_assigned'",
        vec![],
    )
    .await;
    let payload: serde_json::Value = serde_json::from_str(&payload).expect("json");
    assert_eq!(payload["group_id"], g);
    assert_eq!(payload["place_id"], b);
    // у устройства с местом аудита D-30 нет
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM audit_log WHERE action = 'custom:group_place_assigned' \
             AND entity_id = ?1",
            vec![int(with_place)]
        )
        .await,
        0
    );
    // пакет: 1 группа + 1 устройство с местом
    let batch = single_group_batch(&ctx).await;
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE batch_id = ?1",
            vec![text(&batch)]
        )
        .await,
        2
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn journal_cartridge_batch() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let a = seed_place(&ctx, "room", "Склад А", None).await;
    let b = seed_place(&ctx, "room", "Склад Б", None).await;
    let g = seed_group(&ctx, arm, "АРМ #6", 6, Some(a), None).await;
    let printer = seed_device(&ctx, "Принтер", "INV-C-1", "SN-C-1", Some(a), Some(g)).await;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE devices SET type_id = 2 WHERE id = ?1",
                params![printer],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("make printer");
    let model = ctx
        .cartridges
        .model_create(CartridgeModelCreateDto {
            brand: "HP".into(),
            model: "CE285A".into(),
            kind_id: 1,
            color: Some("Чёрный".into()),
            notes: None,
            compatibility: vec![],
        })
        .await
        .expect("model")
        .id;
    let cartridge = ctx
        .cartridges
        .create(CartridgeCreateDto {
            model_id: model,
            number_input: NumberFieldInput {
                value: "C-JRNL-00001".into(),
                template_id: None,
                confirm_mismatch: false,
                confirm_script_mix: false,
            },
            state_id: Some(1),
            place_id: Some(a),
            notes: None,
        })
        .await
        .expect("cartridge")
        .expect_created("cartridge")
        .id;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE cartridges SET current_printer_device_id = ?1 WHERE id = ?2",
                params![printer, cartridge],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("attach");
    let before_rows = scalar_i64(&ctx, "SELECT COUNT(*) FROM place_movements", vec![]).await;

    let v = group_version(&ctx, g).await;
    ctx.groups
        .move_group(&admin(), g, v, b)
        .await
        .expect("move");

    let batch = single_group_batch(&ctx).await;
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE batch_id = ?1 AND entity_type = 'cartridge' \
             AND entity_id = ?2 AND source = 'group' AND group_id = ?3 AND entity_label = 'АРМ #6'",
            vec![text(&batch), int(cartridge), int(g)]
        )
        .await,
        1,
        "строка картриджа в том же пакете"
    );
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE batch_id = ?1",
            vec![text(&batch)]
        )
        .await,
        3,
        "группа + принтер + картридж"
    );
    assert_eq!(
        scalar_i64(&ctx, "SELECT COUNT(*) FROM place_movements", vec![]).await,
        before_rows + 3
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn journal_first_assignment() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let a = seed_place(&ctx, "room", "Склад А", None).await;
    let b = seed_place(&ctx, "room", "Склад Б", None).await;
    let g = seed_group(&ctx, arm, "АРМ #9", 9, None, None).await;
    let d1 = seed_device(
        &ctx,
        "Системный блок",
        "INV-F-1",
        "SN-F-1",
        Some(a),
        Some(g),
    )
    .await;
    let d2 = seed_device(&ctx, "Монитор", "INV-F-2", "SN-F-2", Some(a), Some(g)).await;
    let v = group_version(&ctx, g).await;

    let out = ctx
        .groups
        .move_group(&admin(), g, v, b)
        .await
        .expect("move");

    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'group'",
            vec![]
        )
        .await,
        0,
        "NULL -> место: строки группы нет"
    );
    let batch = single_group_batch(&ctx).await;
    assert_eq!(out.batch_id.as_deref(), Some(batch.as_str()));
    assert_eq!(
        scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE batch_id = ?1 AND group_id = ?2 \
             AND entity_label = 'АРМ #9' AND entity_type = 'device'",
            vec![text(&batch), int(g)]
        )
        .await,
        2,
        "строки устройств несут batch_id, group_id и имя группы без строки-заголовка"
    );
    for d in [d1, d2] {
        let tl = ctx
            .place_movements
            .get_timeline(&admin(), "device", d)
            .await
            .expect("timeline");
        assert_eq!(tl.len(), 1);
        assert_eq!(tl[0].group_id, Some(g));
        assert_eq!(tl[0].group_label.as_deref(), Some("АРМ #9"));
        assert_eq!(tl[0].batch_id.as_deref(), Some(batch.as_str()));
        assert_eq!(tl[0].from_place_id, a);
        assert_eq!(tl[0].to_place_id, b);
    }
}
