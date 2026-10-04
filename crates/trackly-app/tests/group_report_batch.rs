//! Phase 41 Plan 17: отчёт «Перемещения» (HST-04) и пакет группового переноса —
//! поля пакета в `ReportRow`, читаемая строка группы, причины, полный состав
//! в печати и CSV (D-25/D-26/D-27). Реальный `AppCtx` на временном каталоге;
//! имена вымышленные.
//!
//! Префикс тестов: `report_batch_`.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::cartridge::{CartridgeCreateDto, CartridgeModelCreateDto};
use trackly_app::dto::number_template::NumberFieldInput;
use trackly_app::dto::reports::{PeriodDto, ReportFilter, ReportRow};
use trackly_app::tauri_cmds::reports::{
    build_reports_export_csv, build_reports_export_pdf, build_reports_list_movements,
};
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

fn wide_period() -> PeriodDto {
    PeriodDto {
        mode: "range".to_string(),
        year: None,
        month: None,
        date_from: Some("2000-01-01".to_string()),
        date_to: Some("2099-12-31".to_string()),
    }
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

fn int(v: i64) -> rusqlite::types::Value {
    rusqlite::types::Value::Integer(v)
}

async fn exec(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(sql, rusqlite::params_from_iter(p))
                .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("exec");
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

async fn group_version(ctx: &AppCtx, id: i64) -> i64 {
    scalar_i64(
        ctx,
        "SELECT version FROM groups WHERE id = ?1",
        vec![int(id)],
    )
    .await
}

/// Корневая «АРМ #3» (4 прямых устройства) и вложенная «АРМ #4» (2 устройства),
/// всё на «Складе А».
struct Fixture {
    root: i64,
    devices: Vec<i64>,
    place_b: i64,
}

const GROUP_NAME: &str = "АРМ #3";

fn device_names() -> Vec<String> {
    let mut v = Vec::new();
    for i in 0..4 {
        v.push(format!("Системный блок {i}"));
    }
    for i in 4..6 {
        v.push(format!("Монитор {i}"));
    }
    v
}

async fn seed_fixture(ctx: &AppCtx) -> Fixture {
    let arm = type_id_by_code(ctx, "workstation").await;
    let place_a = seed_place(ctx, "Склад А").await;
    let place_b = seed_place(ctx, "Склад Б").await;
    let gp = Some(place_a);
    let root = seed_group(ctx, arm, GROUP_NAME, 3, gp, None).await;
    let nested = seed_group(ctx, arm, "АРМ #4", 4, gp, Some(root)).await;
    let mut devices = Vec::new();
    for (i, name) in device_names().iter().enumerate() {
        let g = if i < 4 { root } else { nested };
        devices.push(
            seed_device(
                ctx,
                name,
                &format!("INV-R-{i}"),
                &format!("SN-R-{i}"),
                gp,
                Some(g),
            )
            .await,
        );
    }
    Fixture {
        root,
        devices,
        place_b,
    }
}

async fn move_fixture(ctx: &AppCtx, f: &Fixture) {
    let v = group_version(ctx, f.root).await;
    ctx.groups
        .move_group(&admin(), f.root, v, f.place_b)
        .await
        .expect("move_group");
}

async fn report(ctx: &AppCtx, filter: ReportFilter) -> Vec<ReportRow> {
    build_reports_list_movements(ctx, &admin(), filter, wide_period())
        .await
        .expect("movements report")
        .rows
}

fn expected_member_reason() -> String {
    format!("в составе группы «{GROUP_NAME}»")
}

// ---------------------------------------------------------------------------
// Задача 1: поля пакета, строка группы, причины
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_batch_full_move_header_and_members() {
    let (ctx, _dir) = make_test_ctx().await;
    let f = seed_fixture(&ctx).await;
    move_fixture(&ctx, &f).await;

    let rows = report(&ctx, ReportFilter::default()).await;
    assert_eq!(rows.len(), 7, "строка группы + 6 устройств");

    let headers: Vec<&ReportRow> = rows
        .iter()
        .filter(|r| r.batch_role.as_deref() == Some("header"))
        .collect();
    assert_eq!(headers.len(), 1);
    let h = headers[0];
    assert_eq!(h.entity_type_label.as_deref(), Some("Группа"));
    assert_eq!(h.device_name.as_deref(), Some(GROUP_NAME));
    assert_eq!(h.reason.as_deref(), Some("перенос группы"));

    let members: Vec<&ReportRow> = rows
        .iter()
        .filter(|r| r.batch_role.as_deref() == Some("member"))
        .collect();
    assert_eq!(members.len(), 6);
    let reason = expected_member_reason();
    for m in &members {
        assert_eq!(m.entity_type_label.as_deref(), Some("Устройство"));
        assert_eq!(m.reason.as_deref(), Some(reason.as_str()));
    }

    let batch_id = h.batch_id.clone().expect("batch_id у заголовка");
    for r in &rows {
        assert_eq!(r.batch_id.as_deref(), Some(batch_id.as_str()));
        assert_eq!(r.batch_label.as_deref(), Some(GROUP_NAME));
        assert_eq!(r.batch_size, Some(6));
    }
    let mut names: Vec<String> = members
        .iter()
        .map(|m| m.device_name.clone().unwrap())
        .collect();
    names.sort();
    let mut expected = device_names();
    expected.sort();
    assert_eq!(names, expected);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_batch_printer_cartridge_row_not_counted() {
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Склад Б").await;
    let g = seed_group(&ctx, arm, "АРМ #8", 8, Some(a), None).await;
    let printer = seed_device(&ctx, "Принтер", "INV-P-1", "SN-P-1", Some(a), Some(g)).await;
    exec(
        &ctx,
        "UPDATE devices SET type_id = 2 WHERE id = ?1",
        vec![int(printer)],
    )
    .await;
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
                value: "C-RPT-00001".into(),
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
    exec(
        &ctx,
        "UPDATE cartridges SET current_printer_device_id = ?1 WHERE id = ?2",
        vec![int(printer), int(cartridge)],
    )
    .await;
    let v = group_version(&ctx, g).await;
    ctx.groups
        .move_group(&admin(), g, v, b)
        .await
        .expect("move");

    let rows = report(&ctx, ReportFilter::default()).await;
    // Заголовок группы + принтер + картридж.
    assert_eq!(rows.len(), 3, "{rows:#?}");
    let cart = rows
        .iter()
        .find(|r| r.entity_type_label.as_deref() == Some("Картридж"))
        .expect("строка картриджа в пакете");
    assert_eq!(cart.batch_role.as_deref(), Some("member"));
    assert!(cart.batch_id.is_some());
    for r in &rows {
        assert_eq!(
            r.batch_size,
            Some(1),
            "считаются только устройства, не картридж и не группа"
        );
        assert_eq!(r.batch_label.as_deref(), Some("АРМ #8"));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_batch_first_placement_has_no_header() {
    // Главный сценарий NULL -> место: у устройств место есть, у самой группы его нет
    // — строки группы в журнале нет (общий гейт Some->Some).
    let (ctx, _dir) = make_test_ctx().await;
    let arm = type_id_by_code(&ctx, "workstation").await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Склад Б").await;
    let g = seed_group(&ctx, arm, GROUP_NAME, 3, None, None).await;
    for (i, name) in device_names().iter().enumerate() {
        seed_device(
            &ctx,
            name,
            &format!("INV-F-{i}"),
            &format!("SN-F-{i}"),
            Some(a),
            Some(g),
        )
        .await;
    }
    let v = group_version(&ctx, g).await;
    ctx.groups
        .move_group(&admin(), g, v, b)
        .await
        .expect("move");

    let rows = report(&ctx, ReportFilter::default()).await;
    assert_eq!(rows.len(), 6, "строки группы нет");
    let first = rows[0].batch_id.clone().expect("batch_id");
    for r in &rows {
        assert_eq!(r.batch_role.as_deref(), Some("member"));
        assert_eq!(r.batch_id.as_deref(), Some(first.as_str()));
        assert_eq!(r.batch_label.as_deref(), Some(GROUP_NAME));
        assert_eq!(r.batch_size, Some(6));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_batch_manual_rows_have_no_batch_fields() {
    let (ctx, _dir) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Склад Б").await;
    let d = seed_device(&ctx, "Ноутбук", "INV-X-1", "SN-X-1", Some(a), None).await;
    exec(
        &ctx,
        "INSERT INTO place_movements (entity_type, entity_id, from_place_id, from_place_path, \
         to_place_id, to_place_path, source, note, created_at_utc) \
         VALUES ('device', ?1, ?2, 'Склад А', ?3, 'Склад Б', 'manual', 'проверка', 1700000100)",
        vec![int(d), int(a), int(b)],
    )
    .await;

    let rows = report(&ctx, ReportFilter::default()).await;
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.batch_id, None);
    assert_eq!(r.batch_role, None);
    assert_eq!(r.batch_size, None);
    assert_eq!(r.batch_label, None);
    assert_eq!(r.reason.as_deref(), Some("вручную · проверка"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_batch_type_filter_drops_header_keeps_size() {
    let (ctx, _dir) = make_test_ctx().await;
    let f = seed_fixture(&ctx).await;
    // Два монитора — «принтерного» типа (type_id = 2): фильтр по типу 1 оставит 4
    // из 6 устройств, и batch_size обязан остаться 6 (подзапрос по batch_id,
    // а не по видимым строкам — иначе тест вакуумный).
    exec(
        &ctx,
        "UPDATE devices SET type_id = 2 WHERE id IN (?1, ?2)",
        vec![int(f.devices[4]), int(f.devices[5])],
    )
    .await;
    move_fixture(&ctx, &f).await;

    let filter = ReportFilter {
        type_id: Some(1),
        ..ReportFilter::default()
    };
    let rows = report(&ctx, filter).await;
    assert_eq!(rows.len(), 4, "видимых строк меньше размера пакета");
    for r in &rows {
        assert_eq!(r.batch_role.as_deref(), Some("member"));
        assert_eq!(r.batch_size, Some(6), "размер по пакету, не по видимым");
        assert_eq!(r.batch_label.as_deref(), Some(GROUP_NAME));
        assert!(r.batch_id.is_some());
    }
    assert!(rows
        .iter()
        .all(|r| r.entity_type_label.as_deref() != Some("Группа")));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_batch_deleted_group_still_readable() {
    let (ctx, _dir) = make_test_ctx().await;
    let f = seed_fixture(&ctx).await;
    move_fixture(&ctx, &f).await;
    exec(&ctx, "DELETE FROM groups", vec![]).await;
    assert_eq!(
        scalar_i64(&ctx, "SELECT COUNT(*) FROM groups", vec![]).await,
        0
    );

    let rows = report(&ctx, ReportFilter::default()).await;
    assert_eq!(rows.len(), 7);
    let h = rows
        .iter()
        .find(|r| r.batch_role.as_deref() == Some("header"))
        .expect("заголовок читается без живой группы");
    assert_eq!(h.device_name.as_deref(), Some(GROUP_NAME));
    assert_eq!(h.entity_type_label.as_deref(), Some("Группа"));
    let reason = expected_member_reason();
    let member = rows
        .iter()
        .find(|r| r.batch_role.as_deref() == Some("member"))
        .expect("член");
    assert_eq!(member.reason.as_deref(), Some(reason.as_str()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_batch_unknown_source_shows_raw_token() {
    let (ctx, _dir) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Склад Б").await;
    let d = seed_device(&ctx, "Ноутбук", "INV-U-1", "SN-U-1", Some(a), None).await;
    exec(
        &ctx,
        "INSERT INTO place_movements (entity_type, entity_id, from_place_id, from_place_path, \
         to_place_id, to_place_path, source, created_at_utc) \
         VALUES ('device', ?1, ?2, 'Склад А', ?3, 'Склад Б', 'future_token', 1700000100)",
        vec![int(d), int(a), int(b)],
    )
    .await;
    let rows = report(&ctx, ReportFilter::default()).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].reason.as_deref(), Some("future_token"));
}

// ---------------------------------------------------------------------------
// Задача 2: печать и CSV — полный состав (D-27)
// ---------------------------------------------------------------------------

/// Число строк данных в HTML: все `<tr>` минус строки шапок (`<thead>`).
fn html_data_rows(html: &str) -> usize {
    html.matches("<tr>").count() - html.matches("<thead>").count()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_batch_print_full_composition() {
    let (ctx, _dir) = make_test_ctx().await;
    let f = seed_fixture(&ctx).await;
    move_fixture(&ctx, &f).await;

    let html = build_reports_export_pdf(
        &ctx,
        &admin(),
        "movements".to_string(),
        ReportFilter::default(),
        Some(wide_period()),
    )
    .await
    .expect("movements print html");

    assert_eq!(html_data_rows(&html), 7, "заголовок группы + 6 устройств");
    assert!(html.contains(&format!("<td>{GROUP_NAME}</td>")));
    assert!(html.contains("<td>Группа</td>"));
    assert!(html.contains("<td>перенос группы</td>"));
    for name in device_names() {
        assert_eq!(
            html.matches(&format!("<td>{name}</td>")).count(),
            1,
            "устройство {name} должно быть в печати ровно один раз"
        );
    }
    let reason = format!("<td>{}</td>", expected_member_reason());
    assert_eq!(html.matches(&reason).count(), 6);
    assert!(!html.contains("batch_"));
    assert!(!html.to_lowercase().contains("chevron"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_batch_csv_full_composition() {
    let (ctx, _dir) = make_test_ctx().await;
    let f = seed_fixture(&ctx).await;
    move_fixture(&ctx, &f).await;

    let bytes = build_reports_export_csv(
        &ctx,
        &admin(),
        "movements".to_string(),
        ReportFilter::default(),
        Some(wide_period()),
    )
    .await
    .expect("movements csv");
    let text = String::from_utf8(bytes).expect("utf8");
    let text = text.trim_start_matches('\u{FEFF}');
    let mut lines = text.lines().filter(|l| !l.is_empty());
    let header = lines.next().expect("заголовок CSV");
    assert_eq!(
        header, "Дата;Предмет;Тип;Откуда;Куда;Кем;Причина",
        "набор колонок CSV не менялся"
    );
    let data: Vec<&str> = lines.collect();
    assert_eq!(data.len(), 7);
    let group_rows: Vec<&&str> = data.iter().filter(|l| l.contains(";Группа;")).collect();
    assert_eq!(group_rows.len(), 1);
    assert!(group_rows[0].contains(GROUP_NAME));
    assert!(group_rows[0].ends_with("перенос группы"));
    for name in device_names() {
        assert_eq!(
            data.iter()
                .filter(|l| l.contains(&format!(";{name};")))
                .count(),
            1,
            "устройство {name}"
        );
    }
    let reason = expected_member_reason();
    assert_eq!(data.iter().filter(|l| l.ends_with(&reason)).count(), 6);
}

#[test]
fn report_batch_template_untouched() {
    // Шаблон отчёта не знает о пакетах: свёртка живёт только на экране (D-27).
    let tpl = include_str!("../templates/report.html");
    assert!(!tpl.contains("batch_"));
}
