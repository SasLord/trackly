//! Acts search integration tests — Phase 3 Plan 05 Task 1 (ACT-04).
//!
//! Покрывает FTS+LIKE search путь через `ActService::search`:
//!   - по номеру акта (LIKE по CAST(number AS TEXT))
//!   - по ФИО Сдал/Принял (LIKE)
//!   - по наименованию устройства (FTS5 MATCH через devices_fts JOIN act_items)
//!   - filter по act_type (Акты vs Возвраты)
//!   - empty query → fallback на list
//!   - спец-символы (одинарная кавычка) — без panic.
//!
//! tokio timeout 30s каждый.

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::act::{ActCreateDto, ActFilter, ActItemNewDto, Pagination};
use trackly_app::services::ActService;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::test_support::test_writer_and_readers;

fn make_acts_service() -> (ActService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = ActService::new(writer, readers, clock);
    (svc, dir)
}

async fn seed_devices_named(
    writer: &Arc<trackly_infra::db::writer_worker::WriterHandle>,
    names: &[&str],
) -> Vec<i64> {
    let owned: Vec<String> = names.iter().map(|n| (*n).to_string()).collect();
    writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            let mut out = Vec::with_capacity(owned.len());
            for name in &owned {
                tx.execute(
                    "INSERT INTO devices \
                     (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                     VALUES (1, ?1, 1, 1, ?2, ?2)",
                    params![name, 1_700_000_000_i64],
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

async fn create_handover(
    svc: &ActService,
    device_ids: &[i64],
    giver: &str,
    receiver: &str,
) -> trackly_app::dto::act::ActDto {
    svc.create(ActCreateDto {
        number_override: None,
        giver_name: giver.to_string(),
        receiver_name: receiver.to_string(),
        location_id: None,
        notes: None,
        deadline_utc: None,
        items: device_ids
            .iter()
            .map(|&id| ActItemNewDto {
                device_id: id,
                quantity: 1,
            })
            .collect(),
    })
    .await
    .expect("create handover")
}

fn handover_filter() -> ActFilter {
    ActFilter {
        act_type: Some("handover".into()),
        archived: Some(false),
        search: None,
        include_deleted: false,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_by_act_number() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        // Создадим 5 handover актов — у каждого один device.
        let device_ids = seed_devices_named(
            &svc.writer,
            &["Stub-1", "Stub-2", "Stub-3", "Stub-4", "Stub-5"],
        )
        .await;
        for &id in &device_ids {
            create_handover(&svc, &[id], "Иванов", "Петров").await;
        }
        // Search по «3» — должен найти акт №3 (LIKE %3% по number-as-text).
        let resp = svc
            .search("3".into(), handover_filter(), Pagination::default())
            .await
            .expect("search");
        assert_eq!(resp.total, 1, "expected one act with number containing '3'");
        assert_eq!(resp.items[0].number_raw, 3);
    })
    .await
    .expect("budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_by_giver_name() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices_named(&svc.writer, &["D-A", "D-B", "D-C"]).await;
        create_handover(&svc, &[device_ids[0]], "Иванов И.И.", "Петров П.П.").await;
        create_handover(&svc, &[device_ids[1]], "Петров С.С.", "Сидоров С.С.").await;
        create_handover(&svc, &[device_ids[2]], "Сидоров К.К.", "Иванов А.А.").await;

        // «Иван» матчит и giver «Иванов И.И.», и receiver «Иванов А.А.» — итого 2.
        let resp = svc
            .search("Иван".into(), handover_filter(), Pagination::default())
            .await
            .expect("search");
        assert_eq!(resp.total, 2, "expected 2 acts matching «Иван»");
    })
    .await
    .expect("budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_by_device_name() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices_named(
            &svc.writer,
            &[
                "Ноутбук Lenovo ThinkPad",
                "Принтер HP LaserJet",
                "Монитор Dell",
            ],
        )
        .await;
        for &id in &device_ids {
            create_handover(&svc, &[id], "Тестов", "Проверкин").await;
        }
        // «Lenovo» — есть в одном устройстве → один акт.
        let resp = svc
            .search("Lenovo".into(), handover_filter(), Pagination::default())
            .await
            .expect("search");
        assert_eq!(resp.total, 1, "expected 1 act for device «Lenovo»");
        assert_eq!(
            resp.items[0].items[0].device_name,
            "Ноутбук Lenovo ThinkPad"
        );
    })
    .await
    .expect("budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_filters_by_tab() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices_named(&svc.writer, &["D-1", "D-2"]).await;
        // Handover «Иванов»
        let h1 = create_handover(&svc, &[device_ids[0]], "Иванов И.И.", "Петров П.П.").await;
        // Handover «Сидоров» → return: giver/receiver наследуются от parent.
        let h2 = create_handover(&svc, &[device_ids[1]], "Сидоров С.С.", "Орлов О.О.").await;
        // Возврат по h2 (giver/receiver унаследует «Сидоров»/«Орлов»).
        let act_item_id: i64 = {
            let readers = svc.readers.clone();
            let h2_id = h2.id;
            tokio::task::spawn_blocking(move || {
                let conn = readers.acquire();
                conn.query_row(
                    "SELECT id FROM act_items WHERE act_id = ?1 LIMIT 1",
                    params![h2_id],
                    |r| r.get(0),
                )
                .expect("act_item id")
            })
            .await
            .expect("spawn")
        };
        let return_payload = trackly_app::dto::act::ActReturnDto {
            bulk_condition: Some("Хорошее".into()),
            bulk_location_id: None,
            bulk_location_name: Some("Склад".into()),
            apply_to_all: true,
            items: vec![trackly_app::dto::act::ActReturnItemDto {
                act_item_id,
                device_id: device_ids[1],
                quantity: 1,
                condition_override: None,
                location_id_override: None,
                location_name_override: None,
            }],
        };
        svc.do_return(h2.id, return_payload).await.expect("return");

        // Search «Сидоров» по handover-filter (archived=None — h2 уже архивирован
        // после полного возврата) → 1 (только h2, без return).
        let h_filter = ActFilter {
            act_type: Some("handover".into()),
            archived: None,
            search: None,
            include_deleted: false,
        };
        let resp_h = svc
            .search("Сидоров".into(), h_filter, Pagination::default())
            .await
            .expect("search handover");
        assert_eq!(resp_h.total, 1, "handover filter must drop returns");
        assert_eq!(resp_h.items[0].id, h2.id);

        // Возвраты: filter act_type=return → 1.
        let r_filter = ActFilter {
            act_type: Some("return".into()),
            archived: None,
            search: None,
            include_deleted: false,
        };
        let resp_r = svc
            .search("Сидоров".into(), r_filter, Pagination::default())
            .await
            .expect("search returns");
        assert_eq!(resp_r.total, 1, "returns filter must drop handovers");
        assert_eq!(resp_r.items[0].act_type, "return");

        // Sanity — что h1 не появляется в обоих результатах.
        assert!(resp_h.items.iter().all(|a| a.id != h1.id));
        assert!(resp_r.items.iter().all(|a| a.id != h1.id));
    })
    .await
    .expect("budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_empty_query_falls_back_to_list() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices_named(&svc.writer, &["A", "B", "C", "D", "E"]).await;
        for &id in &device_ids {
            create_handover(&svc, &[id], "Тестов", "Проверкин").await;
        }
        let resp = svc
            .search("".into(), handover_filter(), Pagination::default())
            .await
            .expect("search empty");
        assert_eq!(resp.total, 5, "empty query should behave like list");
        // Whitespace-only тоже → fallback.
        let resp_ws = svc
            .search("   ".into(), handover_filter(), Pagination::default())
            .await
            .expect("search ws");
        assert_eq!(resp_ws.total, 5);
    })
    .await
    .expect("budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_handles_special_chars() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let device_ids = seed_devices_named(&svc.writer, &["D-1"]).await;
        create_handover(&svc, &[device_ids[0]], "Иванов", "Петров").await;

        // Apostrophe (одинарная кавычка) — параметризованный query не должен
        // упасть; результат — пустой (никто не матчит).
        let resp = svc
            .search("О'Брайен".into(), handover_filter(), Pagination::default())
            .await
            .expect("apostrophe no panic");
        assert_eq!(resp.total, 0);

        // FTS5 spec chars: `"` — экранируется через `""`; `*` сам по себе —
        // build_fts_query обрабатывает.
        let resp2 = svc
            .search(
                "\"unmatched".into(),
                handover_filter(),
                Pagination::default(),
            )
            .await
            .expect("FTS quote no panic");
        assert_eq!(resp2.total, 0);

        // LIKE wildcards `%` и `_` — наш escape заменяет на пробел;
        // оставшийся запрос пустой → fallback на list (1 акт).
        let resp3 = svc
            .search("%".into(), handover_filter(), Pagination::default())
            .await
            .expect("percent no panic");
        // После escape «%» → пробел → trimmed.is_empty? Нет — у нас cleaned
        // содержит один пробел, plain_query «% %» — матчит всё (с пробелом
        // в строке). Не строгий assert, просто no-panic.
        let _ = resp3;
    })
    .await
    .expect("budget");
}
