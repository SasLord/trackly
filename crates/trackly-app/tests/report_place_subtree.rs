//! D-28 (Phase 39): «Фильтр по месту — включая вложенные».
//!
//! Выбор места в фильтре отчёта должен захватывать это место И ВСЁ, что
//! вложено под ним (рекурсивный CTE `subtree` в `report_service.rs`), а не
//! только строки с точным совпадением `place_id`.
//!
//! В `report_service.rs` этот фильтр реализован ЧЕТЫРЬМЯ независимыми
//! построителями SQL (каждый со своей копией CTE), и все четыре достижимы
//! через публичный API `ReportService`:
//!
//! | Построитель             | Публичный вход                                  |
//! |-------------------------|-------------------------------------------------|
//! | `query_acts_inner`      | `list_device_acts` / `list_device_returns`      |
//! | `query_device_snapshot` | `list_device_in_stock` / `list_device_in_use`   |
//! | `count_acts_inner`      | `get_report_counts("devices")` → `acts`/`returns` |
//! | `count_device_snapshot` | `get_report_counts("devices")` → `in_use`/`in_stock` |
//!
//! Каждый тест проверяет ДВЕ стороны контракта:
//!   1. строка на самом глубоком уровне («Здание А / 2 этаж / Кабинет 214»)
//!      возвращается при фильтре по КОРНЮ «Здание А»;
//!   2. строка из СОСЕДНЕГО поддерева («Корпус Б / …») НЕ возвращается —
//!      иначе CTE был бы no-op, отдающим вообще всё.
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)` (PATTERNS.md §Pattern 4).
//!
//! Все названия мест, устройств и ФИО — ВЫМЫШЛЕННЫЕ (жёсткое условие
//! приватности: репозиторий публичный).

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

/// Период, заведомо накрывающий «сейчас» — акты создаются `SystemClock`'ом.
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

/// Трёхуровневое дерево + СОСЕДНЕЕ поддерево того же уровня вложенности.
struct Tree {
    building_a: i64,
    room_a: i64,
    building_b: i64,
    room_b: i64,
}

async fn seed_tree(writer: &Arc<WriterHandle>) -> Tree {
    let building_a = create_place(writer, None, PlaceKind::Building, "Здание А").await;
    let floor_a = create_place(writer, Some(building_a), PlaceKind::Floor, "2 этаж").await;
    let room_a = create_place(writer, Some(floor_a), PlaceKind::Room, "Кабинет 214").await;

    let building_b = create_place(writer, None, PlaceKind::Building, "Корпус Б").await;
    let floor_b = create_place(writer, Some(building_b), PlaceKind::Floor, "1 этаж").await;
    let room_b = create_place(writer, Some(floor_b), PlaceKind::Room, "Кабинет 101").await;

    Tree {
        building_a,
        room_a,
        building_b,
        room_b,
    }
}

/// `status_id = 1` = «На складе» (сид V001).
async fn seed_device(writer: &Arc<WriterHandle>, name: &str, place_id: i64) -> i64 {
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

fn count_for(counts: &trackly_app::dto::reports::ReportCountsDto, key: &str) -> i64 {
    counts
        .counts
        .iter()
        .find(|e| e.key == key)
        .unwrap_or_else(|| panic!("no count entry for key {key}: {:?}", counts.counts))
        .count
}

// ---------------------------------------------------------------------------
// 1. query_acts_inner — list_device_acts
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn acts_report_root_place_filter_returns_deeply_nested_act() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let ctx = make_ctx();
        let tree = seed_tree(&ctx.writer).await;

        let dev_a = seed_device(&ctx.writer, "Ноутбук А-214", tree.room_a).await;
        let dev_b = seed_device(&ctx.writer, "Ноутбук Б-101", tree.room_b).await;
        create_handover(&ctx.acts, dev_a, tree.room_a, "Иванов И.И.").await;
        create_handover(&ctx.acts, dev_b, tree.room_b, "Сидоров С.С.").await;

        // Sanity: без фильтра видны оба акта — иначе тест ниже проходил бы
        // «зелёным» просто потому, что данных нет.
        let all = ctx
            .reports
            .list_device_acts(ReportFilter::default(), wide_period())
            .await
            .expect("list all acts");
        assert_eq!(all.rows.len(), 2, "фикстура: должно быть 2 акта, {all:?}");

        // D-28: фильтр по КОРНЮ должен вернуть акт с 3-го уровня вложенности.
        let filtered = ctx
            .reports
            .list_device_acts(
                ReportFilter {
                    place_id: Some(tree.building_a),
                    ..Default::default()
                },
                wide_period(),
            )
            .await
            .expect("list acts filtered by root place");

        assert_eq!(
            filtered.rows.len(),
            1,
            "фильтр по «Здание А» должен вернуть РОВНО 1 акт (вложенный, \
             из «Кабинет 214»), а не 0 (точное совпадение place_id) и не 2 \
             (CTE-no-op): {filtered:?}"
        );
        let row = &filtered.rows[0];
        assert_eq!(row.giver_name.as_deref(), Some("Иванов И.И."));
        assert_eq!(
            row.place_path.as_deref(),
            Some("Здание А / 2 этаж / Кабинет 214"),
            "снимок пути в отчёте — полный путь из place_full_paths"
        );
        assert_eq!(filtered.total, 1, "total должен совпадать с числом строк");
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn acts_report_place_filter_excludes_sibling_subtree() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let ctx = make_ctx();
        let tree = seed_tree(&ctx.writer).await;

        let dev_a = seed_device(&ctx.writer, "Ноутбук А-214", tree.room_a).await;
        let dev_b = seed_device(&ctx.writer, "Ноутбук Б-101", tree.room_b).await;
        create_handover(&ctx.acts, dev_a, tree.room_a, "Иванов И.И.").await;
        create_handover(&ctx.acts, dev_b, tree.room_b, "Сидоров С.С.").await;

        let filtered = ctx
            .reports
            .list_device_acts(
                ReportFilter {
                    place_id: Some(tree.building_b),
                    ..Default::default()
                },
                wide_period(),
            )
            .await
            .expect("list acts filtered by sibling root");

        assert_eq!(filtered.rows.len(), 1, "«Корпус Б» → ровно 1 акт");
        assert_eq!(
            filtered.rows[0].giver_name.as_deref(),
            Some("Сидоров С.С."),
            "поддерево «Корпус Б» не должно захватывать акт из «Здание А»"
        );
        assert!(
            filtered.rows.iter().all(|r| !r
                .place_path
                .as_deref()
                .unwrap_or("")
                .contains("Здание А")),
            "ни одна строка «Корпуса Б» не должна ссылаться на «Здание А»: {filtered:?}"
        );
    })
    .await
    .expect("timeout");
}

// ---------------------------------------------------------------------------
// 2. query_device_snapshot — list_device_in_stock
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn device_snapshot_root_place_filter_returns_deeply_nested_device() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let ctx = make_ctx();
        let tree = seed_tree(&ctx.writer).await;

        seed_device(&ctx.writer, "Ноутбук А-214", tree.room_a).await;
        seed_device(&ctx.writer, "Ноутбук Б-101", tree.room_b).await;

        let all = ctx
            .reports
            .list_device_in_stock(ReportFilter::default())
            .await
            .expect("snapshot without filter");
        assert_eq!(all.rows.len(), 2, "фикстура: 2 устройства «На складе»");

        let filtered = ctx
            .reports
            .list_device_in_stock(ReportFilter {
                place_id: Some(tree.building_a),
                ..Default::default()
            })
            .await
            .expect("snapshot filtered by root place");

        assert_eq!(
            filtered.rows.len(),
            1,
            "фильтр по «Здание А» должен вернуть вложенное устройство из \
             «Кабинет 214» и только его: {filtered:?}"
        );
        assert_eq!(
            filtered.rows[0].device_name.as_deref(),
            Some("Ноутбук А-214")
        );
        assert_eq!(
            filtered.rows[0].place_path.as_deref(),
            Some("Здание А / 2 этаж / Кабинет 214")
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn device_snapshot_middle_level_place_filter_captures_only_its_own_branch() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let ctx = make_ctx();
        let tree = seed_tree(&ctx.writer).await;

        // Третий, ЗАВЕДОМО ПУСТОЙ корень — контрольная группа: если бы CTE
        // был no-op, фильтр по нему вернул бы все строки, а не ноль.
        let empty_building = create_place(&ctx.writer, None, PlaceKind::Building, "Корпус В").await;

        seed_device(&ctx.writer, "Ноутбук А-214", tree.room_a).await;
        seed_device(&ctx.writer, "Ноутбук Б-101", tree.room_b).await;

        // Фильтр по листу «Кабинет 214» — самый узкий случай (поддерево = сам узел).
        let leaf = ctx
            .reports
            .list_device_in_stock(ReportFilter {
                place_id: Some(tree.room_a),
                ..Default::default()
            })
            .await
            .expect("snapshot filtered by leaf place");
        assert_eq!(leaf.rows.len(), 1, "лист «Кабинет 214» → своё устройство");

        // Фильтр по пустому корню — 0 строк (доказывает, что CTE не no-op).
        let empty = ctx
            .reports
            .list_device_in_stock(ReportFilter {
                place_id: Some(empty_building),
                ..Default::default()
            })
            .await
            .expect("snapshot filtered by empty place");
        assert_eq!(
            empty.rows.len(),
            0,
            "пустое место «Корпус В» не должно возвращать ничего — иначе CTE \
             был бы no-op, отдающим весь список: {empty:?}"
        );
    })
    .await
    .expect("timeout");
}

// ---------------------------------------------------------------------------
// 3. count_acts_inner + count_device_snapshot — get_report_counts("devices")
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_counts_devices_domain_place_filter_is_subtree_inclusive() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let ctx = make_ctx();
        let tree = seed_tree(&ctx.writer).await;

        // Устройства, участвующие в актах: `ActService::create` переводит их
        // в «В работе», поэтому для счётчика `in_stock` нужна ОТДЕЛЬНАЯ пара
        // устройств, которых акты не касаются.
        let dev_a = seed_device(&ctx.writer, "Ноутбук А-214", tree.room_a).await;
        let dev_b = seed_device(&ctx.writer, "Ноутбук Б-101", tree.room_b).await;
        create_handover(&ctx.acts, dev_a, tree.room_a, "Иванов И.И.").await;
        create_handover(&ctx.acts, dev_b, tree.room_b, "Сидоров С.С.").await;

        seed_device(&ctx.writer, "Монитор А-214", tree.room_a).await;
        seed_device(&ctx.writer, "Монитор Б-101", tree.room_b).await;

        let unfiltered = ctx
            .reports
            .get_report_counts("devices", ReportFilter::default(), wide_period(), false)
            .await
            .expect("counts without filter");
        assert_eq!(count_for(&unfiltered, "acts"), 2, "фикстура: 2 акта");
        assert_eq!(
            count_for(&unfiltered, "in_stock"),
            2,
            "фикстура: 2 устройства «На складе» (мониторы, не тронутые актами)"
        );

        // count_acts_inner + count_device_snapshot, оба с D-28 CTE.
        let filtered = ctx
            .reports
            .get_report_counts(
                "devices",
                ReportFilter {
                    place_id: Some(tree.building_a),
                    ..Default::default()
                },
                wide_period(),
                false,
            )
            .await
            .expect("counts filtered by root place");

        assert_eq!(
            count_for(&filtered, "acts"),
            1,
            "count_acts_inner: «Здание А» должно посчитать вложенный акт из \
             «Кабинет 214» (не 0 — точное совпадение, не 2 — no-op)"
        );
        assert_eq!(
            count_for(&filtered, "in_stock"),
            1,
            "count_device_snapshot: «Здание А» должно посчитать вложенный \
             «Монитор А-214» из «Кабинет 214»"
        );

        // Соседнее поддерево — свои счётчики, не общие.
        let sibling = ctx
            .reports
            .get_report_counts(
                "devices",
                ReportFilter {
                    place_id: Some(tree.building_b),
                    ..Default::default()
                },
                wide_period(),
                false,
            )
            .await
            .expect("counts filtered by sibling root");
        assert_eq!(count_for(&sibling, "acts"), 1);
        assert_eq!(count_for(&sibling, "in_stock"), 1);
    })
    .await
    .expect("timeout");
}

// ---------------------------------------------------------------------------
// 4. Четырёхуровневая глубина — CTE должен рекурсировать, а не «на один шаг»
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_filter_walks_more_than_one_level_of_nesting() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let ctx = make_ctx();

        // Здание А / 2 этаж / Кабинет 214 / Шкаф 3 — четыре уровня.
        let building = create_place(&ctx.writer, None, PlaceKind::Building, "Здание А").await;
        let floor = create_place(&ctx.writer, Some(building), PlaceKind::Floor, "2 этаж").await;
        let room = create_place(&ctx.writer, Some(floor), PlaceKind::Room, "Кабинет 214").await;
        let cabinet = create_place(&ctx.writer, Some(room), PlaceKind::Zone, "Шкаф 3").await;

        seed_device(&ctx.writer, "Ноутбук в шкафу", cabinet).await;

        for (level_name, place_id) in [
            ("корень «Здание А»", building),
            ("«2 этаж»", floor),
            ("«Кабинет 214»", room),
            ("«Шкаф 3»", cabinet),
        ] {
            let resp = ctx
                .reports
                .list_device_in_stock(ReportFilter {
                    place_id: Some(place_id),
                    ..Default::default()
                })
                .await
                .expect("snapshot by ancestor");
            assert_eq!(
                resp.rows.len(),
                1,
                "фильтр по уровню {level_name} должен захватить устройство на 4-м \
                 уровне вложенности: {resp:?}"
            );
            assert_eq!(
                resp.rows[0].place_path.as_deref(),
                Some("Здание А / 2 этаж / Кабинет 214 / Шкаф 3")
            );
        }
    })
    .await
    .expect("timeout");
}
