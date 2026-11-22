//! Гвард HST-07 (фаза 41.10): отчёт о перемещениях (HST-04) читает только
//! `place_movements`; события истории (`history_events`) в нём не появляются.
//!
//! Данные вымышленные (privacy gate).

use std::sync::Arc;

use rusqlite::params;
use trackly_app::dto::reports::{PeriodDto, ReportFilter, ReportResponse};
use trackly_app::pdf::PdfRenderer;
use trackly_app::services::ReportService;
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::writer_worker::WriterHandle;
use trackly_infra::test_support::test_writer_and_readers;

fn wide_period() -> PeriodDto {
    PeriodDto {
        mode: "range".to_string(),
        year: None,
        month: None,
        date_from: Some("2000-01-01".to_string()),
        date_to: Some("2099-12-31".to_string()),
    }
}

fn internal(e: rusqlite::Error) -> AppError {
    AppError::Internal {
        source_chain: format!("{e}"),
    }
}

async fn seed_place(writer: &WriterHandle, name: &str) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, is_storage, created_at_utc, updated_at_utc, version) \
                 VALUES ('room', ?1, 0, 1700000000, 1700000000, 1)",
                params![name],
            )
            .map_err(internal)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

async fn seed_device(writer: &WriterHandle, name: &str) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (1, ?1, 1, 1, 1700000000, 1700000000)",
                params![name],
            )
            .map_err(internal)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
}

async fn seed_movement(writer: &WriterHandle, device: i64, from: i64, to: i64, at: i64) {
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO place_movements \
                 (entity_type, entity_id, from_place_id, from_place_path, to_place_id, \
                  to_place_path, source, created_at_utc) \
                 VALUES ('device', ?1, ?2, 'Склад', ?3, 'Кабинет 101', 'manual', ?4)",
                params![device, from, to, at],
            )
            .map_err(internal)?;
            Ok(())
        })
        .await
        .expect("seed movement");
}

/// События истории на те же устройства и в тот же период.
async fn seed_history_events(writer: &WriterHandle, device: i64, at: i64) {
    for (i, (kind, status)) in [
        ("group_joined", None),
        ("group_left", None),
        ("status_changed", Some("in_repair")),
    ]
    .into_iter()
    .enumerate()
    {
        writer
            .execute(move |conn| {
                conn.execute(
                    "INSERT INTO history_events \
                     (operation_id, subject_type, subject_id, subject_label, kind, \
                      from_group_id, from_group_label, to_group_id, to_group_label, \
                      status_to_code, status_to_label, source, movement_id, created_at_utc) \
                     VALUES (?1, 'device', ?2, 'АРМ-3', ?3, NULL, 'АРМ-5', NULL, 'АРМ-5', \
                             ?4, ?4, 'manual', NULL, ?5)",
                    params![format!("op-guard-{i}"), device, kind, status, at + i as i64],
                )
                .map_err(internal)?;
                Ok(())
            })
            .await
            .expect("seed history event");
    }
}

fn service(
    writer: Arc<WriterHandle>,
    readers: Arc<trackly_infra::db::pools::ReaderPool>,
) -> ReportService {
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    ReportService::new(
        writer,
        readers,
        clock,
        Arc::new(trackly_infra::AppConfig::default()),
        Arc::new(PdfRenderer::new()),
    )
}

async fn report(svc: &ReportService) -> ReportResponse {
    svc.list_movements(ReportFilter::default(), wide_period())
        .await
        .expect("movements report")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_not_in_movements_report_unchanged() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let svc = service(writer.clone(), readers.clone());
    let a = seed_place(&writer, "Склад").await;
    let b = seed_place(&writer, "Кабинет 101").await;
    let d1 = seed_device(&writer, "Ноутбук 1").await;
    let d2 = seed_device(&writer, "Ноутбук 2").await;
    seed_movement(&writer, d1, a, b, 1_700_000_100).await;
    seed_movement(&writer, d2, a, b, 1_700_000_200).await;

    let before = report(&svc).await;
    assert_eq!(before.total, 2, "фикстура: два перемещения");

    seed_history_events(&writer, d1, 1_700_000_100).await;
    seed_history_events(&writer, d2, 1_700_000_200).await;

    let after = report(&svc).await;
    assert_eq!(
        serde_json::to_value(&before).unwrap(),
        serde_json::to_value(&after).unwrap(),
        "события истории изменили отчёт о перемещениях"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_not_in_movements_report_no_event_rows() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let svc = service(writer.clone(), readers.clone());
    let a = seed_place(&writer, "Склад").await;
    let b = seed_place(&writer, "Кабинет 101").await;
    let moved = seed_device(&writer, "Ноутбук 1").await;
    let only_events = seed_device(&writer, "Ноутбук 2").await;
    seed_movement(&writer, moved, a, b, 1_700_000_100).await;
    seed_history_events(&writer, only_events, 1_700_000_150).await;
    seed_history_events(&writer, moved, 1_700_000_100).await;

    let res = report(&svc).await;
    assert_eq!(res.total, 1, "событие без перемещения попало в отчёт");
    assert_eq!(res.rows.len(), 1);
    let dump = serde_json::to_string(&res.rows).unwrap();
    assert!(
        !dump.contains("Ноутбук 2"),
        "устройство только с событиями присутствует в отчёте: {dump}"
    );
}

/// Исходник отчёта не должен упоминать `history_events` вне комментариев.
fn code_mentions(src: &str, needle: &str) -> usize {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .map(|l| l.matches(needle).count())
        .sum()
}

#[test]
fn history_not_in_movements_report_source_scan() {
    let report_src = include_str!("../src/services/report_service.rs");
    let dto_src = include_str!("../src/dto/reports.rs");
    assert_eq!(
        code_mentions(report_src, "history_events"),
        0,
        "report_service.rs читает history_events (HST-07)"
    );
    assert_eq!(
        code_mentions(dto_src, "history_events"),
        0,
        "dto/reports.rs упоминает history_events (HST-07)"
    );
    // Проба несущая: сканер действительно видит упоминание в коде.
    assert_eq!(
        code_mentions("let q = \"history_events\";", "history_events"),
        1
    );
    assert_eq!(code_mentions("// history_events", "history_events"), 0);
}
