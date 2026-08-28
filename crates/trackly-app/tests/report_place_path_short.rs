//! Phase 39.1 Plan 05 — `place_path_short` on all 5 report domains (D-17).
//!
//! Each of the 5 `query_*_inner` builders in `report_service.rs`
//! (`query_acts_inner`, `query_device_snapshot`, `query_cartridge_audit`,
//! `query_cartridge_snapshot`, `query_requests_inner`) must LEFT JOIN
//! `place_effective_variant` and populate `ReportRow.place_path_short`
//! via the shared `shorten_place_path` formula (Plan 01).
//!
//! Fresh-DB org default variant is «Крайние» (`ends`, V039 seed) with
//! separator `" // "` — a 3-segment place `"Здание А / 2 этаж / Кабинет 214"`
//! therefore shortens to `"Здание А // Кабинет 214"`.
//!
//! Все имена мест/устройств/ФИО — ВЫМЫШЛЕННЫЕ (репозиторий публичный).

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto};
use trackly_app::dto::reports::{PeriodDto, ReportFilter};
use trackly_app::pdf::PdfRenderer;
use trackly_app::services::report_service::ReportService;
use trackly_app::services::ActService;
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::ports::places::PlaceRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::writer_worker::WriterHandle;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::SqlitePlaceRepository;
use trackly_infra::test_support::test_writer_and_readers;
use trackly_infra::AppConfig;

const NOW: i64 = 1_700_000_000;
const SHORT_PATH: &str = "Здание А // Кабинет 214";
const FULL_PATH: &str = "Здание А / 2 этаж / Кабинет 214";

struct Ctx {
    reports: ReportService,
    acts: ActService,
    writer: Arc<WriterHandle>,
    _dir: tempfile::TempDir,
}

fn make_ctx() -> Ctx {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let reports = ReportService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        Arc::new(AppConfig::default()),
        Arc::new(PdfRenderer::new()),
    );
    let acts = ActService::new(writer.clone(), readers.clone(), clock.clone());
    Ctx {
        reports,
        acts,
        writer,
        _dir: dir,
    }
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

async fn create_place(
    writer: &Arc<WriterHandle>,
    parent_id: Option<i64>,
    kind: PlaceKind,
    name: &str,
) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            let repo = SqlitePlaceRepository;
            repo.create(
                conn,
                &PlaceNew {
                    parent_id,
                    kind,
                    name: name.clone(),
                    level: None,
                    is_storage: false,
                    sort_order: None,
                    notes: None,
                },
                NOW,
            )
        })
        .await
        .expect("create place")
}

/// 3-segment place: «Здание А / 2 этаж / Кабинет 214».
async fn seed_room(writer: &Arc<WriterHandle>) -> i64 {
    let building = create_place(writer, None, PlaceKind::Building, "Здание А").await;
    let floor = create_place(writer, Some(building), PlaceKind::Floor, "2 этаж").await;
    create_place(writer, Some(floor), PlaceKind::Room, "Кабинет 214").await
}

async fn seed_device(writer: &Arc<WriterHandle>, name: &str, place_id: Option<i64>) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, place_id, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (1, ?1, ?2, 1, 1, ?3, ?3)",
                params![name, place_id, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
}

async fn create_handover(acts: &ActService, device_id: i64, place_id: i64, giver: &str) {
    acts.create(ActCreateDto {
        number_override: None,
        giver_name: giver.to_string(),
        receiver_name: "Петров П.П.".to_string(),
        place_id: Some(place_id),
        notes: None,
        deadline_utc: None,
        handover_date_utc: None,
        items: vec![ActItemNewDto {
            device_id,
            device_ids: Vec::new(),
            quantity: 1,
        }],
    })
    .await
    .expect("create handover act");
}

async fn seed_cartridge_model(writer: &Arc<WriterHandle>, brand: &str, model: &str) -> i64 {
    let brand = brand.to_string();
    let model = model.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO cartridge_models \
                 (brand, model, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?3, 1)",
                params![brand, model, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed cartridge model")
}

/// `status_id = 1` = «На складе» (сид V001).
async fn seed_cartridge(
    writer: &Arc<WriterHandle>,
    code: &str,
    model_id: i64,
    place_id: i64,
) -> i64 {
    let code = code.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO cartridges \
                 (code, model_id, status_id, place_id, created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, 1, ?3, ?4, ?4, 1)",
                params![code, model_id, place_id, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed cartridge")
}

async fn seed_audit_log(
    writer: &Arc<WriterHandle>,
    entity_id: i64,
    action: &str,
    created_at_utc: i64,
) {
    let action = action.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO audit_log (entity_type, entity_id, action, created_at_utc) \
                 VALUES ('cartridge', ?1, ?2, ?3)",
                params![entity_id, action, created_at_utc],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("seed audit log");
}

async fn seed_requester(writer: &Arc<WriterHandle>, login: &str, full_name: &str) -> i64 {
    let login = login.to_string();
    let full_name = full_name.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users \
                 (login, full_name, password_hash, role, ad_user, is_active, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, NULL, 'employee', 1, 0, ?3, ?3, 1)",
                params![login, full_name, NOW],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed requester")
}

async fn seed_request(
    writer: &Arc<WriterHandle>,
    requested_by_user_id: i64,
    printer_device_id: i64,
    created_at_utc: i64,
) {
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO requests \
                 (request_type, status, requested_by_user_id, printer_device_id, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES ('free_form', 'open', ?1, ?2, ?3, ?3, 1)",
                params![requested_by_user_id, printer_device_id, created_at_utc],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("seed request");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn acts_report_carries_shortened_place_path() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let ctx = make_ctx();
        let room = seed_room(&ctx.writer).await;
        let dev = seed_device(&ctx.writer, "Ноутбук А-214", Some(room)).await;
        create_handover(&ctx.acts, dev, room, "Иванов И.И.").await;

        let rows = ctx
            .reports
            .list_device_acts(ReportFilter::default(), wide_period())
            .await
            .expect("list_device_acts");
        assert_eq!(rows.rows.len(), 1);
        let row = &rows.rows[0];
        assert_eq!(row.place_path.as_deref(), Some(FULL_PATH));
        assert_eq!(
            row.place_path_short.as_deref(),
            Some(SHORT_PATH),
            "D-17: query_acts_inner должен нести сокращённый путь"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn device_snapshot_report_carries_shortened_place_path() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let ctx = make_ctx();
        let room = seed_room(&ctx.writer).await;
        // status_id=1 == «На складе».
        seed_device(&ctx.writer, "Принтер HP", Some(room)).await;

        let rows = ctx
            .reports
            .list_device_in_stock(ReportFilter::default())
            .await
            .expect("list_device_in_stock");
        assert_eq!(rows.rows.len(), 1);
        let row = &rows.rows[0];
        assert_eq!(row.place_path.as_deref(), Some(FULL_PATH));
        assert_eq!(
            row.place_path_short.as_deref(),
            Some(SHORT_PATH),
            "D-17: query_device_snapshot должен нести сокращённый путь"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn device_snapshot_report_no_place_yields_none() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let ctx = make_ctx();
        seed_device(&ctx.writer, "Принтер без места", None).await;

        let rows = ctx
            .reports
            .list_device_in_stock(ReportFilter::default())
            .await
            .expect("list_device_in_stock");
        assert_eq!(rows.rows.len(), 1);
        let row = &rows.rows[0];
        assert_eq!(row.place_path, None);
        assert_eq!(
            row.place_path_short, None,
            "устройство без места не должно нести place_path_short"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cartridge_audit_report_carries_shortened_place_path() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let ctx = make_ctx();
        let room = seed_room(&ctx.writer).await;
        let model = seed_cartridge_model(&ctx.writer, "HP", "CB435A").await;
        let cartridge = seed_cartridge(&ctx.writer, "C-000001", model, room).await;
        seed_audit_log(&ctx.writer, cartridge, "custom:install", NOW).await;

        let rows = ctx
            .reports
            .list_cartridge_consumption(ReportFilter::default(), wide_period())
            .await
            .expect("list_cartridge_consumption");
        assert_eq!(rows.rows.len(), 1);
        let row = &rows.rows[0];
        assert_eq!(row.place_path.as_deref(), Some(FULL_PATH));
        assert_eq!(
            row.place_path_short.as_deref(),
            Some(SHORT_PATH),
            "D-17: query_cartridge_audit должен нести сокращённый путь"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cartridge_snapshot_report_carries_shortened_place_path() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let ctx = make_ctx();
        let room = seed_room(&ctx.writer).await;
        let model = seed_cartridge_model(&ctx.writer, "HP", "CB435A").await;
        // status_id=1 == «На складе».
        seed_cartridge(&ctx.writer, "C-000002", model, room).await;

        let rows = ctx
            .reports
            .list_cartridge_in_stock(ReportFilter::default())
            .await
            .expect("list_cartridge_in_stock");
        assert_eq!(rows.rows.len(), 1);
        let row = &rows.rows[0];
        assert_eq!(row.place_path.as_deref(), Some(FULL_PATH));
        assert_eq!(
            row.place_path_short.as_deref(),
            Some(SHORT_PATH),
            "D-17: query_cartridge_snapshot должен нести сокращённый путь"
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn requests_report_carries_shortened_printer_place_path() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let ctx = make_ctx();
        let room = seed_room(&ctx.writer).await;
        let printer = seed_device(&ctx.writer, "Kyocera-01", Some(room)).await;
        let requester = seed_requester(&ctx.writer, "petrov", "Петров П.П.").await;
        seed_request(&ctx.writer, requester, printer, NOW).await;

        let rows = ctx
            .reports
            .list_requests_all(ReportFilter::default(), wide_period(), false)
            .await
            .expect("list_requests_all");
        assert_eq!(rows.rows.len(), 1);
        let row = &rows.rows[0];
        assert_eq!(row.place_path.as_deref(), Some(FULL_PATH));
        assert_eq!(
            row.place_path_short.as_deref(),
            Some(SHORT_PATH),
            "D-17: query_requests_inner должен нести сокращённый путь принтера"
        );
    })
    .await
    .expect("timeout");
}
