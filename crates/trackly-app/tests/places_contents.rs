//! Интеграционные тесты `PlaceService`'s read half (Phase 39 Plan 08, Task 1):
//! `list_subtree_contents` D-24 nested-vs-"Только здесь" toggle, `list_children`
//! natural sibling ordering (D-05), `subtree_stats` nested-inclusive counters
//! (D-25).
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)` — защита от Linux-CI
//! deadlock (PATTERNS.md §Pattern 4), mirrors `places_delete_blocked.rs`'s
//! idiom. Все имена мест/устройств вымышленные (приватность репозитория).

use std::sync::Arc;
use std::time::Duration;

use trackly_app::services::PlaceService;
use trackly_core::auth::Identity;
use trackly_core::domain::places::{sibling_cmp, PlaceKind, PlaceNew};
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

fn make_service() -> (PlaceService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = PlaceService::new(writer, readers, clock);
    (svc, dir)
}

fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

fn new_place(kind: PlaceKind, name: &str, parent_id: Option<i64>) -> PlaceNew {
    PlaceNew {
        parent_id,
        kind,
        name: name.to_string(),
        level: None,
        is_storage: false,
        sort_order: None,
        notes: None,
    }
}

/// Inserts a minimal device row directly (device_types id=1 / device_statuses
/// id=1 are seeded by V001) — mirrors `places_delete_blocked.rs`'s fixture
/// style. No `DeviceService` call needed; this is a raw fixture insert via
/// the writer.
async fn insert_device(svc: &PlaceService, place_id: i64, name: &str) {
    let name = name.to_string();
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, place_id, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (1, ?1, ?2, 1, 1, 0, 0)",
                rusqlite::params![name, place_id],
            )
            .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("insert fixture device");
}

// ---------------------------------------------------------------------------
// list_subtree_contents — D-24 nested-vs-"Только здесь" toggle
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_subtree_contents_nested_true_includes_nested_place_devices() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let building = svc
            .create(&admin, new_place(PlaceKind::Building, "Здание А", None))
            .await
            .expect("create building");
        let floor = svc
            .create(
                &admin,
                new_place(PlaceKind::Floor, "2 этаж", Some(building.id)),
            )
            .await
            .expect("create nested floor");

        // 2 direct devices on the building, 1 nested device on the floor beneath it.
        insert_device(&svc, building.id, "Ноутбук №1").await;
        insert_device(&svc, building.id, "Ноутбук №2").await;
        insert_device(&svc, floor.id, "Ноутбук №3").await;

        let nested = svc
            .list_subtree_contents(&admin, building.id, true, true)
            .await
            .expect("list_subtree_contents nested=true");
        assert_eq!(
            nested.len(),
            3,
            "nested=true должен вернуть все 3 устройства (2 прямых + 1 вложенное)"
        );

        let direct_only = svc
            .list_subtree_contents(&admin, building.id, false, true)
            .await
            .expect("list_subtree_contents nested=false");
        assert_eq!(
            direct_only.len(),
            2,
            "nested=false («Только здесь») должен вернуть только 2 прямых устройства"
        );
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// place_path_short — D-17 (Phase 39.1 Plan 07): PlaceContents carries the
// shortened path, symmetric with device/cartridge list `place_path_short`.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_subtree_contents_computes_place_path_short_per_effective_variant() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let building = svc
            .create(&admin, new_place(PlaceKind::Building, "Здание А", None))
            .await
            .expect("create building");
        // Override on the top-level place (D-06) — floor/room below inherit it
        // dynamically via place_effective_variant's upward walk.
        svc.set_path_variant(
            &admin,
            building.id,
            Some("last".to_string()),
            building.version,
        )
        .await
        .expect("set_path_variant last");

        let floor = svc
            .create(
                &admin,
                new_place(PlaceKind::Floor, "1 этаж", Some(building.id)),
            )
            .await
            .expect("create floor");
        let room = svc
            .create(&admin, new_place(PlaceKind::Room, "1-05", Some(floor.id)))
            .await
            .expect("create room");
        insert_device(&svc, room.id, "Ноутбук №1").await;

        let contents = svc
            .list_subtree_contents(&admin, building.id, true, true)
            .await
            .expect("list_subtree_contents");
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].full_path, "Здание А / 1 этаж / 1-05");
        assert_eq!(
            contents[0].place_path_short, "1-05",
            "«Последнее» на 3-сегментном пути должен вернуть только последний сегмент"
        );
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// list_children — sibling_cmp natural ordering, not DB insertion order
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_children_sorted_by_sibling_cmp_not_insertion_order() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let building = svc
            .create(&admin, new_place(PlaceKind::Building, "Здание Б", None))
            .await
            .expect("create building");

        // Insert rooms out of natural order: "10" before "2" — natural_name_cmp
        // must still order them "2" < "10" (not lexicographic "10" < "2").
        let _room_10 = svc
            .create(&admin, new_place(PlaceKind::Room, "10", Some(building.id)))
            .await
            .expect("create room 10");
        let _room_2 = svc
            .create(&admin, new_place(PlaceKind::Room, "2", Some(building.id)))
            .await
            .expect("create room 2");

        let children = svc
            .list_children(&admin, Some(building.id))
            .await
            .expect("list_children");

        assert_eq!(children.len(), 2);
        assert_eq!(
            children.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            vec!["2", "10"],
            "ожидали натуральный порядок «2» перед «10», а не порядок вставки в БД"
        );
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// subtree_stats — D-25 nested-inclusive counters
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn subtree_stats_counts_nested_places_and_devices_inclusive() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let building = svc
            .create(&admin, new_place(PlaceKind::Building, "Здание В", None))
            .await
            .expect("create building");
        let floor = svc
            .create(
                &admin,
                new_place(PlaceKind::Floor, "3 этаж", Some(building.id)),
            )
            .await
            .expect("create nested floor");

        insert_device(&svc, building.id, "Ноутбук №1").await;
        insert_device(&svc, building.id, "Ноутбук №2").await;
        insert_device(&svc, floor.id, "Ноутбук №3").await;

        let stats = svc
            .subtree_stats(&admin, building.id)
            .await
            .expect("subtree_stats");

        assert_eq!(stats.nested_places, 1, "1 вложенное место (floor)");
        assert_eq!(
            stats.device_count, 3,
            "device_count должен учитывать вложенные места (D-25): 2 прямых + 1 вложенное"
        );
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// quick 260827-rzq: list_children/list_all must not panic on a partial
// sort_order set that contradicts name/level order — the shape produced by
// drag-and-drop reordering a subset of siblings, which crashed
// places_list_all/places_list_children in production before the sibling_cmp fix.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_children_and_list_all_survive_partial_sort_order_without_panicking() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let building = svc
            .create(&admin, new_place(PlaceKind::Building, "Здание Г", None))
            .await
            .expect("create building");

        // ~15 rooms under the same building. Names deliberately contradict both
        // level and sort_order. Roughly a third get an explicit sort_order that
        // runs in reverse of insertion order (mirrors drag-and-drop reordering a
        // subset of siblings); the rest get None (natural order applies).
        for i in 0..15i64 {
            let sort_order = if i % 3 == 0 { Some(15 - i) } else { None };
            let level = Some((i % 4) - 1);
            let name = format!("Каб. {}", 15 - i);
            let place_new = PlaceNew {
                parent_id: Some(building.id),
                kind: PlaceKind::Room,
                name,
                level,
                is_storage: false,
                sort_order,
                notes: None,
            };
            svc.create(&admin, place_new)
                .await
                .expect("create room with partial sort_order/level");
        }

        // Neither call must panic (production regression: sort_by(sibling_cmp)
        // paniced with "user-provided comparison function does not correctly
        // implement a total order" on exactly this partial-sort_order shape).
        let children = svc
            .list_children(&admin, Some(building.id))
            .await
            .expect("list_children must not panic on partial sort_order");
        assert_eq!(children.len(), 15);
        for w in children.windows(2) {
            assert_ne!(
                sibling_cmp(&w[0], &w[1]),
                std::cmp::Ordering::Greater,
                "list_children: non-decreasing sibling order violated between {:?} and {:?}",
                w[0].name,
                w[1].name
            );
        }

        let all = svc
            .list_all(&admin, false)
            .await
            .expect("list_all must not panic on partial sort_order");
        assert!(all.len() >= 16, "building + 15 rooms");

        // list_all groups by parent_id then applies sibling_cmp within the group —
        // verify every adjacent pair sharing a parent_id is non-decreasing under
        // sibling_cmp (non-adjacent-parent pairs are not meaningfully comparable
        // and are not asserted on).
        for w in all.windows(2) {
            if w[0].parent_id == w[1].parent_id {
                assert_ne!(
                    sibling_cmp(&w[0], &w[1]),
                    std::cmp::Ordering::Greater,
                    "list_all: non-decreasing sibling order violated within parent_id={:?} between {:?} and {:?}",
                    w[0].parent_id,
                    w[0].name,
                    w[1].name
                );
            }
        }
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// status_code (Phase 41.1 Plan 04, D-06/D-07/D-09): строка содержимого несёт
// машинный ключ статуса рядом с подписью — UI выбирает тон бейджа по нему,
// а не по русскому имени. Картриджи остаются вне справочника кодов (None).
// ---------------------------------------------------------------------------

/// Вставляет устройство с заданным типом и статусом, выбранным по `code`
/// (числовые id статусов не зашиты — V047 добавляет новые через MAX+1).
async fn insert_device_with_status(
    svc: &PlaceService,
    place_id: i64,
    name: &str,
    type_id: i64,
    status_code: &str,
) {
    let name = name.to_string();
    let status_code = status_code.to_string();
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, place_id, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, ?3, (SELECT id FROM device_statuses WHERE code = ?4), 1, 0, 0)",
                rusqlite::params![type_id, name, place_id, status_code],
            )
            .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("insert fixture device with status");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_subtree_contents_devices_carry_status_code_next_to_status_name() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let room = svc
            .create(&admin, new_place(PlaceKind::Room, "Каб. 101", None))
            .await
            .expect("create room");
        insert_device_with_status(&svc, room.id, "Монитор №1", 1, "на_списание").await;
        insert_device_with_status(&svc, room.id, "Монитор №2", 1, "на_складе").await;
        insert_device_with_status(&svc, room.id, "Монитор №3", 1, "в_ремонте").await;

        let contents = svc
            .list_subtree_contents(&admin, room.id, false, true)
            .await
            .expect("list_subtree_contents");
        assert_eq!(contents.len(), 3);

        let by_name = |n: &str| {
            contents
                .iter()
                .find(|r| r.name == n)
                .unwrap_or_else(|| panic!("нет строки {n}"))
        };
        assert_eq!(by_name("Монитор №1").kind, "device");
        assert_eq!(
            by_name("Монитор №1").status_code.as_deref(),
            Some("на_списание")
        );
        assert_eq!(
            by_name("Монитор №1").status_name.as_deref(),
            Some("На списание")
        );
        assert_eq!(
            by_name("Монитор №2").status_code.as_deref(),
            Some("на_складе")
        );
        assert_eq!(
            by_name("Монитор №2").status_name.as_deref(),
            Some("На складе")
        );
        assert_eq!(
            by_name("Монитор №3").status_code.as_deref(),
            Some("в_ремонте")
        );
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_subtree_contents_status_code_survives_status_rename() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let room = svc
            .create(&admin, new_place(PlaceKind::Room, "Каб. 102", None))
            .await
            .expect("create room");
        insert_device_with_status(&svc, room.id, "Монитор №1", 1, "на_ремонте").await;

        // Подпись статуса меняется, машинный ключ — нет (D-01, D-02).
        svc.writer
            .execute(|conn| {
                conn.execute(
                    "UPDATE device_statuses SET name = 'Переименован' WHERE code = 'на_ремонте'",
                    [],
                )
                .map_err(trackly_infra::error_conversions::map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("rename status");

        let contents = svc
            .list_subtree_contents(&admin, room.id, false, true)
            .await
            .expect("list_subtree_contents");
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].status_name.as_deref(), Some("Переименован"));
        assert_eq!(contents[0].status_code.as_deref(), Some("на_ремонте"));
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_subtree_contents_printer_carries_status_code() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let room = svc
            .create(&admin, new_place(PlaceKind::Room, "Каб. 103", None))
            .await
            .expect("create room");
        // device_types id=2 = «Принтер» (сид V001); ветка printer выбирается по типу.
        insert_device_with_status(&svc, room.id, "Принтер №1", 2, "в_работе").await;

        let contents = svc
            .list_subtree_contents(&admin, room.id, false, true)
            .await
            .expect("list_subtree_contents");
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].kind, "printer");
        assert_eq!(contents[0].status_code.as_deref(), Some("в_работе"));
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_subtree_contents_cartridge_has_no_status_code() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();

        let room = svc
            .create(&admin, new_place(PlaceKind::Room, "Каб. 104", None))
            .await
            .expect("create room");
        let place_id = room.id;
        svc.writer
            .execute(move |conn| {
                conn.execute(
                    "INSERT INTO cartridge_models \
                     (brand, model, created_at_utc, updated_at_utc, version) \
                     VALUES ('Бренд', 'Модель-1', 0, 0, 1)",
                    [],
                )
                .map_err(trackly_infra::error_conversions::map_rusqlite)?;
                let model_id = conn.last_insert_rowid();
                conn.execute(
                    "INSERT INTO cartridges \
                     (code, model_id, status_id, place_id, created_at_utc, updated_at_utc, version) \
                     VALUES ('C-TEST-1', ?1, 1, ?2, 0, 0, 1)",
                    rusqlite::params![model_id, place_id],
                )
                .map_err(trackly_infra::error_conversions::map_rusqlite)?;
                Ok(())
            })
            .await
            .expect("insert fixture cartridge");

        let contents = svc
            .list_subtree_contents(&admin, room.id, false, true)
            .await
            .expect("list_subtree_contents");
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].kind, "cartridge");
        assert!(
            contents[0].status_name.is_some(),
            "подпись статуса картриджа остаётся"
        );
        assert_eq!(contents[0].status_code, None);
    })
    .await
    .expect("test timed out");
}

// ---------------------------------------------------------------------------
// Phase 41.2 Plan 05 (GAM-04): режим «группа вместо членов».
//
// ОДНА расходящаяся фикстура: место с группой из трёх устройств (одно из них
// принтер с прикреплённым картриджем), «спящая» группа без места (её члены живут
// в собственных местах), одиночное устройство вне групп, вложенное место с ещё
// одним устройством. Члены с разными статусами.
// ---------------------------------------------------------------------------

struct GroupFixture {
    root: i64,
    nested: i64,
    group: i64,
    member_block: i64,
    member_monitor: i64,
    member_printer: i64,
    cartridge: i64,
    dormant_group: i64,
    dormant_a: i64,
    dormant_b: i64,
    single: i64,
    nested_device: i64,
}

/// Сырой посев устройства: `type_id` 1 — обычное, 2 — принтер.
async fn seed_device_row(
    svc: &PlaceService,
    type_id: i64,
    name: &str,
    place_id: i64,
    status_id: i64,
    group_id: Option<i64>,
) -> i64 {
    let name = name.to_string();
    svc.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, place_id, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, ?3, ?4, 1, 0, 0)",
                rusqlite::params![type_id, name, place_id, status_id],
            )
            .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            let id = conn.last_insert_rowid();
            if let Some(g) = group_id {
                conn.execute(
                    "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
                     VALUES (?1, ?2, 0)",
                    rusqlite::params![id, g],
                )
                .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            }
            Ok(id)
        })
        .await
        .expect("seed device row")
}

async fn seed_group_row(svc: &PlaceService, name: &str, seq: i64, place_id: Option<i64>) -> i64 {
    let name = name.to_string();
    svc.writer
        .execute(move |conn| {
            // Тип создаёт сам тест: справочник типов групп сеет AppCtx, не миграция.
            conn.execute(
                "INSERT OR IGNORE INTO group_types \
                 (code, name, behavior, created_at_utc, updated_at_utc) \
                 VALUES ('ws_fixture', 'АРМ', 'repair', 0, 0)",
                [],
            )
            .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, version, \
                 created_at_utc, updated_at_utc) \
                 VALUES ((SELECT id FROM group_types WHERE code = 'ws_fixture'), \
                         ?1, ?2, ?3, 1, 0, 0)",
                rusqlite::params![name, seq, place_id],
            )
            .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group row")
}

async fn group_fixture(svc: &PlaceService) -> GroupFixture {
    let admin = admin_caller();
    let root = svc
        .create(&admin, new_place(PlaceKind::Room, "Кабинет 201", None))
        .await
        .expect("create root")
        .id;
    let nested = svc
        .create(
            &admin,
            new_place(PlaceKind::Room, "Подсобка 201", Some(root)),
        )
        .await
        .expect("create nested")
        .id;
    let group = seed_group_row(svc, "АРМ #3", 3, Some(root)).await;
    let member_block = seed_device_row(svc, 1, "Системный блок №1", root, 1, Some(group)).await;
    let member_monitor = seed_device_row(svc, 1, "Монитор №1", root, 2, Some(group)).await;
    let member_printer = seed_device_row(svc, 2, "Принтер №1", root, 3, Some(group)).await;
    // Картридж, установленный в принтер-член группы.
    let cartridge = svc
        .writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO cartridge_models (brand, model, created_at_utc, updated_at_utc, version) \
                 VALUES ('Бренд', 'Модель-7', 0, 0, 1)",
                [],
            )
            .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            let model_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO cartridges \
                 (code, model_id, status_id, place_id, current_printer_device_id, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES ('C-GRP-1', ?1, 1, ?2, ?3, 0, 0, 1)",
                rusqlite::params![model_id, root, member_printer],
            )
            .map_err(trackly_infra::error_conversions::map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed cartridge");
    // «Спящая» группа: место NULL, члены живут в собственных местах.
    let dormant_group = seed_group_row(svc, "АРМ #4", 4, None).await;
    let dormant_a = seed_device_row(svc, 1, "Спящий член А", root, 1, Some(dormant_group)).await;
    let dormant_b = seed_device_row(svc, 1, "Спящий член Б", nested, 1, Some(dormant_group)).await;
    let single = seed_device_row(svc, 1, "Ноутбук Одиночный", root, 1, None).await;
    let nested_device = seed_device_row(svc, 1, "Ноутбук Вложенный", nested, 1, None).await;
    GroupFixture {
        root,
        nested,
        group,
        member_block,
        member_monitor,
        member_printer,
        cartridge,
        dormant_group,
        dormant_a,
        dormant_b,
        single,
        nested_device,
    }
}

fn ids_of(rows: &[trackly_core::domain::places::PlaceContentRow], kind: &str) -> Vec<i64> {
    let mut v: Vec<i64> = rows
        .iter()
        .filter(|r| r.kind == kind)
        .map(|r| r.id)
        .collect();
    v.sort_unstable();
    v
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn expand_false_collapses_group_members_into_one_group_row() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let f = group_fixture(&svc).await;
        let rows = svc
            .list_subtree_contents(&admin_caller(), f.root, true, false)
            .await
            .expect("list collapsed");

        let groups: Vec<_> = rows.iter().filter(|r| r.kind == "group").collect();
        assert_eq!(groups.len(), 1, "одна строка группы (спящая — без места)");
        let g = groups[0];
        assert_eq!(g.id, f.group);
        assert_eq!(g.name, "АРМ #3", "чистое имя группы, без склейки счётчика");
        assert_eq!(g.group_type_name.as_deref(), Some("АРМ"));
        assert_eq!(g.member_count, Some(3));
        assert_eq!(g.printer_member_count, Some(1));
        assert_eq!(g.inventory_or_code, None);
        assert_eq!(g.status_name, None);
        assert_eq!(g.status_code, None);
        assert!(g.full_path.contains("Кабинет 201"));

        let dev = ids_of(&rows, "device");
        let prn = ids_of(&rows, "printer");
        for hidden in [f.member_block, f.member_monitor] {
            assert!(!dev.contains(&hidden), "член группы скрыт: device {hidden}");
        }
        assert!(
            !prn.contains(&f.member_printer),
            "принтер-член скрыт тем же предикатом"
        );
        assert_eq!(
            ids_of(&rows, "cartridge"),
            vec![f.cartridge],
            "картридж принтера остаётся своей строкой"
        );
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn expand_true_keeps_every_member_and_has_no_group_rows() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let f = group_fixture(&svc).await;
        let rows = svc
            .list_subtree_contents(&admin_caller(), f.root, true, true)
            .await
            .expect("list physical");
        assert!(ids_of(&rows, "group").is_empty(), "строк группы нет");
        let dev = ids_of(&rows, "device");
        for id in [f.member_block, f.member_monitor, f.single, f.nested_device] {
            assert!(
                dev.contains(&id),
                "устройство {id} видно в физическом режиме"
            );
        }
        assert_eq!(ids_of(&rows, "printer"), vec![f.member_printer]);
        assert!(rows.iter().all(|r| r.group_type_name.is_none()
            && r.member_count.is_none()
            && r.printer_member_count.is_none()));
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dormant_group_members_stay_visible_without_group_row() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let f = group_fixture(&svc).await;
        let rows = svc
            .list_subtree_contents(&admin_caller(), f.root, true, false)
            .await
            .expect("list collapsed");
        let dev = ids_of(&rows, "device");
        assert!(
            dev.contains(&f.dormant_a),
            "член спящей группы виден (P-10)"
        );
        assert!(
            dev.contains(&f.dormant_b),
            "член спящей группы виден (P-10)"
        );
        assert!(
            !rows
                .iter()
                .any(|r| r.kind == "group" && r.id == f.dormant_group),
            "у группы без места строки нет"
        );
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn physical_counts_equal_in_both_modes_while_row_count_differs() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();
        let f = group_fixture(&svc).await;

        let stats = svc.subtree_stats(&admin, f.root).await.expect("stats");
        // 3 члена + 2 спящих + одиночное + вложенное = 7, физически, в обоих режимах.
        assert_eq!(stats.device_count, 7);

        let physical = svc
            .list_subtree_contents(&admin, f.root, true, true)
            .await
            .expect("physical");
        let collapsed = svc
            .list_subtree_contents(&admin, f.root, true, false)
            .await
            .expect("collapsed");
        assert!(
            collapsed.len() < physical.len(),
            "строк при expand_groups=false строго меньше: {} против {}",
            collapsed.len(),
            physical.len()
        );
        // Строка группы не прибавляет единицу к device_count: счётчик не зависит от режима.
        let stats_after = svc
            .subtree_stats(&admin, f.root)
            .await
            .expect("stats again");
        assert_eq!(stats_after.device_count, stats.device_count);
        let device_like = |rows: &[trackly_core::domain::places::PlaceContentRow]| {
            rows.iter()
                .filter(|r| matches!(r.kind.as_str(), "device" | "printer"))
                .count()
        };
        assert_eq!(device_like(&physical), 7);
        assert_eq!(device_like(&collapsed), 4, "7 минус 3 скрытых члена");
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn nested_false_returns_group_row_only_in_the_groups_own_place() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();
        let f = group_fixture(&svc).await;

        let here = svc
            .list_subtree_contents(&admin, f.root, false, false)
            .await
            .expect("only here");
        assert_eq!(ids_of(&here, "group"), vec![f.group]);

        let child = svc
            .list_subtree_contents(&admin, f.nested, false, false)
            .await
            .expect("child only");
        assert!(
            ids_of(&child, "group").is_empty(),
            "во вложенном месте строки группы нет"
        );
    })
    .await
    .expect("test timed out");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn expand_false_keeps_member_outside_group_place_visible() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        let admin = admin_caller();
        let f = group_fixture(&svc).await;
        // Старые данные: член живёт не в месте группы (нарушенный инвариант).
        let stray =
            seed_device_row(&svc, 1, "Монитор Расхождение", f.nested, 1, Some(f.group)).await;

        let rows = svc
            .list_subtree_contents(&admin, f.root, true, false)
            .await
            .expect("collapsed");
        assert!(
            ids_of(&rows, "device").contains(&stray),
            "член с расходящимся местом не прячется"
        );
        let group_row = rows.iter().find(|r| r.kind == "group").expect("group row");
        assert_eq!(group_row.member_count, Some(4), "счётчик физический: 3 + 1");
    })
    .await
    .expect("test timed out");
}
