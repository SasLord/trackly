//! Справочник типов устройств `device_types` (Phase 41.12, INT-14, D-01, D-02).
//!
//! Данные вымышленные. Назначение файла — любая вновь появившаяся карта
//! `type_id` -> подпись (в экспорте или в импорте CSV) должна сделать тест красным:
//! для этого в справочник вставляется тип «Монитор» (id 3), которого нет ни в одной
//! карте кода.

use std::collections::HashMap;

use trackly_app::context::AppCtx;
use trackly_app::dto::device::{DeviceFilter, DeviceNew};
use trackly_core::auth::Identity;
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::ports::places::PlaceRepository;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::SqlitePlaceRepository;

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

/// Вставляет в справочник тип, отсутствующий в прежних картах кода (id 3).
async fn insert_monitor_type(ctx: &AppCtx) {
    ctx.devices
        .writer
        .execute(|conn| {
            conn.execute("INSERT INTO device_types (name) VALUES ('Монитор')", [])
                .map(|_| ())
                .map_err(map_rusqlite)
        })
        .await
        .expect("insert Монитор");
}

fn type_id_of(ctx: &AppCtx, device_name: &str) -> i64 {
    let conn = ctx.devices.readers.acquire();
    conn.query_row(
        "SELECT type_id FROM devices WHERE name = ?1",
        [device_name],
        |r| r.get::<_, i64>(0),
    )
    .unwrap_or_else(|e| panic!("нет устройства «{device_name}»: {e}"))
}

fn printers_rows(ctx: &AppCtx, device_name: &str) -> i64 {
    let conn = ctx.devices.readers.acquire();
    conn.query_row(
        "SELECT COUNT(*) FROM printers WHERE device_id = \
         (SELECT id FROM devices WHERE name = ?1)",
        [device_name],
        |r| r.get::<_, i64>(0),
    )
    .expect("count printers")
}

/// Собирает CSV `;` с колонками «Тип», «Наименование» (+ необязательно «Место»).
fn csv_bytes(rows: &[(&str, &str, Option<&str>)]) -> Vec<u8> {
    let with_place = rows.iter().any(|(_, _, p)| p.is_some());
    let mut out = String::new();
    out.push_str(if with_place {
        "Тип;Наименование;Место\n"
    } else {
        "Тип;Наименование\n"
    });
    for (type_label, name, place) in rows {
        if with_place {
            out.push_str(&format!("{type_label};{name};{}\n", place.unwrap_or("")));
        } else {
            out.push_str(&format!("{type_label};{name}\n"));
        }
    }
    out.into_bytes()
}

fn mapping(with_place: bool) -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("Тип".to_string(), "type".to_string());
    m.insert("Наименование".to_string(), "name".to_string());
    if with_place {
        m.insert("Место".to_string(), "place".to_string());
    }
    m
}

async fn import(
    ctx: &AppCtx,
    rows: &[(&str, &str, Option<&str>)],
) -> trackly_app::dto::device::CsvImportReport {
    let with_place = rows.iter().any(|(_, _, p)| p.is_some());
    let preview = ctx
        .devices
        .import_csv_preview(csv_bytes(rows))
        .await
        .expect("preview");
    ctx.devices
        .import_csv_commit(preview.token, mapping(with_place))
        .await
        .expect("commit")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_types_returns_dictionary_rows() {
    let (ctx, _dir) = make_test_ctx().await;
    let list = ctx.devices.list_types().await.expect("list_types");
    let direct: Vec<(i64, String)> = {
        let conn = ctx.devices.readers.acquire();
        let mut stmt = conn
            .prepare("SELECT id, name FROM device_types ORDER BY id")
            .expect("prepare");
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .expect("query")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect");
        rows
    };
    let got: Vec<(i64, String)> = list.iter().map(|t| (t.id, t.name.clone())).collect();
    assert_eq!(got, direct);
    assert_eq!(
        got,
        vec![(1, "Устройство".to_string()), (2, "Принтер".to_string())]
    );
}

/// Выгрузка CSV не имеет собственной таблицы подписей типа: для КАЖДОГО типа
/// справочника устройство выгружается с подписью из справочника.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn csv_export_label_equals_dictionary_name_for_every_type() {
    let (ctx, _dir) = make_test_ctx().await;
    insert_monitor_type(&ctx).await;
    let types = ctx.devices.list_types().await.expect("list_types");

    // Невырожденность: фикстура обязана расходиться с прежними картами.
    assert!(
        types.iter().any(|t| t.name == "Монитор"),
        "в справочнике нет типа «Монитор»"
    );
    let t2 = types.iter().find(|t| t.id == 2).expect("тип 2");
    assert_ne!(t2.name, "Расходник");
    let t3 = types.iter().find(|t| t.id == 3).expect("тип 3");
    assert_ne!(t3.name, "Устройство");

    let statuses = ctx.devices.list_statuses().await.expect("statuses");
    let stock = statuses
        .iter()
        .find(|s| s.code == "на_складе")
        .expect("статус на_складе")
        .id;

    for t in &types {
        ctx.devices
            .create(DeviceNew {
                type_id: t.id,
                name: format!("Проверка типа {}", t.id),
                inventory_no: None,
                serial_no: None,
                model: None,
                specs: None,
                kit: None,
                state: None,
                place_id: None,
                status_id: stock,
            })
            .await
            .expect("create");
    }

    let csv = ctx
        .devices
        .export_csv(DeviceFilter::default())
        .await
        .expect("export");
    for t in &types {
        let line = csv
            .lines()
            .find(|l| l.contains(&format!("Проверка типа {}", t.id)))
            .unwrap_or_else(|| panic!("нет строки для типа {}", t.id));
        let cells: Vec<&str> = line.trim_start_matches('\u{FEFF}').split(';').collect();
        assert_eq!(cells[0], t.name, "подпись типа {}", t.id);
    }
}

/// Импорт читает тот же справочник: подпись каждого типа даёт его id.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn import_resolves_every_dictionary_label() {
    let (ctx, _dir) = make_test_ctx().await;
    insert_monitor_type(&ctx).await;
    let types = ctx.devices.list_types().await.expect("list_types");

    let names: Vec<String> = types
        .iter()
        .map(|t| format!("Импорт типа {}", t.id))
        .collect();
    let rows: Vec<(&str, &str, Option<&str>)> = types
        .iter()
        .zip(names.iter())
        .map(|(t, n)| (t.name.as_str(), n.as_str(), None))
        .collect();
    let report = import(&ctx, &rows).await;

    assert!(
        report.failed.is_empty(),
        "ошибки строк: {:?}",
        report.failed
    );
    assert_eq!(report.inserted, types.len() as u64);
    for (t, n) in types.iter().zip(names.iter()) {
        assert_eq!(type_id_of(&ctx, n), t.id, "тип для подписи «{}»", t.name);
    }
}

/// Строка «Принтер» создаёт принтер: type_id 2, строка `printers`, вид printer в месте.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn imported_printer_is_visible_as_printer_kind() {
    let (ctx, _dir) = make_test_ctx().await;
    let place_id = ctx
        .devices
        .writer
        .execute(|conn| {
            let repo = SqlitePlaceRepository;
            repo.create(
                conn,
                &PlaceNew {
                    parent_id: None,
                    kind: PlaceKind::Room,
                    name: "Кабинет 1".to_string(),
                    level: None,
                    is_storage: false,
                    sort_order: None,
                    notes: None,
                },
                1_700_000_000,
            )
        })
        .await
        .expect("seed place");

    let report = import(&ctx, &[("Принтер", "Импорт принтера", Some("Кабинет 1"))]).await;
    assert!(
        report.failed.is_empty(),
        "ошибки строк: {:?}",
        report.failed
    );
    assert_eq!(report.inserted, 1);

    assert_eq!(type_id_of(&ctx, "Импорт принтера"), 2);
    assert_eq!(printers_rows(&ctx, "Импорт принтера"), 1);

    let contents = ctx
        .places
        .list_subtree_contents(&Identity::trusted_admin(), place_id, true)
        .await
        .expect("contents");
    let row = contents
        .iter()
        .find(|r| r.name == "Импорт принтера")
        .expect("устройство в содержимом места");
    assert_eq!(row.kind, "printer");
}

// D-01 (41.12): до фазы 41.12 экспорт печатал «Расходник» на принтерах, и round-trip
// старых выгрузок работал по нему; псевдоним в импорте сохранён. Это единственное
// место вне device_service.rs, где слово допустимо.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn legacy_consumable_label_imports_as_printer() {
    let (ctx, _dir) = make_test_ctx().await;
    let report = import(&ctx, &[("Расходник", "Импорт старой выгрузки", None)]).await;
    assert!(
        report.failed.is_empty(),
        "ошибки строк: {:?}",
        report.failed
    );
    assert_eq!(report.inserted, 1);
    assert_eq!(type_id_of(&ctx, "Импорт старой выгрузки"), 2);
    assert_eq!(printers_rows(&ctx, "Импорт старой выгрузки"), 1);
}

// D-02 (41.12): неизвестная непустая подпись и пустая ячейка дают «Устройство» без
// ошибки строки; регистр и пробелы подписи значения не имеют.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_label_imports_as_default_device_without_row_error() {
    let (ctx, _dir) = make_test_ctx().await;
    let report = import(
        &ctx,
        &[
            ("Планшет", "Импорт неизвестного", None),
            ("", "Импорт пустого", None),
            (" принтер ", "Импорт регистра", None),
        ],
    )
    .await;
    assert!(
        report.failed.is_empty(),
        "ошибки строк: {:?}",
        report.failed
    );
    assert_eq!(report.inserted, 3);
    assert_eq!(type_id_of(&ctx, "Импорт неизвестного"), 1);
    assert_eq!(type_id_of(&ctx, "Импорт пустого"), 1);
    assert_eq!(type_id_of(&ctx, "Импорт регистра"), 2);
}
