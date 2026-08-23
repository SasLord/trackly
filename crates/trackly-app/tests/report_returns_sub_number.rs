//! GAP-4 regression test — Phase 28 Plan 15 (gap-closure).
//!
//! Covers: «Отчёты -> Устройства -> Возвраты» load failure ("Не удалось загрузить
//! отчёт"). Root cause (diagnosed via static analysis in 28-15-PLAN.md, cross-
//! referencing `migrations/V004__acts.sql`, `dto/reports.rs`, and `git blame`):
//!
//! `report_service.rs`'s `query_acts_inner` SQL (shared by BOTH `list_device_acts`
//! and `list_device_returns`) selects `a.sub_number` RAW — unlike the adjacent
//! `number` column, which IS explicitly cast: `CAST(a.number AS TEXT) as number`.
//! `acts.sub_number` is declared `INTEGER NULL` in the schema, but
//! `ReportRow.sub_number` is typed `Option<String>`. rusqlite's `FromSql` for
//! `String` requires the underlying SQLite value to be `Text`; handover acts
//! always have `sub_number = NULL` (so `Option<String>` sees `Null` and succeeds
//! trivially), but return acts have a real, non-NULL INTEGER `sub_number` — the
//! conversion fails at the driver level for those rows, exactly matching the
//! reported symptom (Возвраты report only).
//!
//! FIXED (28-15 Task 3, human decision `fix-now`): `query_acts_inner`'s SQL now
//! casts `a.sub_number` to TEXT (`CAST(a.sub_number AS TEXT) as sub_number`),
//! identical in kind to the pre-existing `CAST(a.number AS TEXT) as number`.
//! This test seeds a partial-return act (guaranteed non-NULL integer
//! `sub_number`) and calls `list_device_returns`, asserting it returns
//! `Ok(..)` with the row's `sub_number` round-tripped as `"1"`.

use std::sync::Arc;

use trackly_app::dto::act::{ActCreateDto, ActItemNewDto, ActReturnDto, ActReturnItemDto};
use trackly_app::dto::reports::{PeriodDto, ReportFilter};
use trackly_app::pdf::PdfRenderer;
use trackly_app::services::report_service::ReportService;
use trackly_app::services::ActService;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;
use trackly_infra::AppConfig;

async fn seed_devices(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    count: usize,
) -> Vec<i64> {
    let names: Vec<String> = (0..count).map(|i| format!("GapReturnDevice {i}")).collect();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            let mut out = Vec::with_capacity(names.len());
            for name in &names {
                tx.execute(
                    "INSERT INTO devices \
                     (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                     VALUES (1, ?1, 1, 1, ?2, ?2)",
                    rusqlite::params![name, 1_700_000_000_i64],
                )
                .map_err(map_rusqlite)?;
                out.push(tx.last_insert_rowid());
            }
            tx.commit().map_err(map_rusqlite)?;
            Ok(out)
        })
        .await
        .expect("seed devices")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn returns_report_loads_when_sub_number_is_set() {
    // One shared DB for both ActService (seeds the return act) and ReportService
    // (queries it) — mirrors `report_csv_export.rs`'s manual harness, but built
    // via the canonical `test_writer_and_readers` fixture so writer/readers are
    // guaranteed to point at the SAME file.
    let (writer, readers, _dir) = test_writer_and_readers();

    // --- Seed: handover + partial return (mirrors acts_returns.rs) ---
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let act_svc = ActService::new(writer.clone(), readers.clone(), clock.clone());

    let device_ids = seed_devices(&writer, 2).await;

    let handover = act_svc
        .create(ActCreateDto {
            number_override: None,
            giver_name: "Иванов И.И.".into(),
            receiver_name: "Петров П.П.".into(),
            place_id: None,
            notes: None,
            deadline_utc: None,
            handover_date_utc: None,
            items: device_ids
                .iter()
                .map(|&id| ActItemNewDto {
                    device_id: id,
                    device_ids: Vec::new(),
                    quantity: 1,
                })
                .collect(),
        })
        .await
        .expect("create handover");

    let first_item = &handover.items[0];
    let return_payload = ActReturnDto {
        bulk_condition: Some("Хорошее".into()),
        bulk_place_id: None,
        apply_to_all: true,
        giver_name: None,
        receiver_name: None,
        handover_date_utc: None,
        items: vec![ActReturnItemDto {
            act_item_id: first_item.id,
            device_id: first_item.device_id,
            device_ids: vec![first_item.device_id],
            quantity: 1,
            condition_override: None,
            place_id_override: None,
        }],
    };

    let ret = act_svc
        .do_return(handover.id, return_payload)
        .await
        .expect("do_return");

    // Precondition: the seeded return act really does have a non-NULL integer
    // sub_number — this is what makes the buggy raw `a.sub_number` SELECT fail.
    assert_eq!(
        ret.sub_number,
        Some(1),
        "precondition failed: seeded return act must have sub_number = Some(1)"
    );

    // --- Query: ReportService::list_device_returns against the SAME DB ---
    let config = Arc::new(AppConfig::default());
    let pdf = Arc::new(PdfRenderer::new());
    let report_svc = ReportService::new(writer, readers, clock, config, pdf);

    let period = PeriodDto {
        mode: "range".to_string(),
        year: None,
        month: None,
        date_from: Some("2000-01-01".to_string()),
        date_to: Some("2100-01-01".to_string()),
    };

    let result = report_svc
        .list_device_returns(ReportFilter::default(), period)
        .await;

    // Fixed (28-15 Task 3): `query_acts_inner` now casts sub_number to TEXT,
    // so this succeeds for return acts with a non-NULL integer sub_number —
    // resolving GAP-4 ("Не удалось загрузить отчёт" on Отчёты -> Устройства ->
    // Возвраты).
    let response = result.expect(
        "list_device_returns must succeed for a return act with a non-NULL sub_number \
         (GAP-4 reproduction: rusqlite column-type mismatch on a.sub_number)",
    );

    assert_eq!(
        response.rows.len(),
        1,
        "expected exactly one seeded return act row in the report"
    );
    assert_eq!(
        response.rows[0].sub_number,
        Some("1".to_string()),
        "sub_number must round-trip as \"1\" once properly cast to TEXT"
    );
}
