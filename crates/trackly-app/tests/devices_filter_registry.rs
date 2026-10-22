//! Реестр фильтров устройств: матрица «поле x путь» (Phase 41.11 Plan 07, WR-02, D-08/D-08a).
//!
//! Корень дефекта WR-02: поле `DeviceFilter` принимается контрактом, но в WHERE не попадает,
//! и так проходят годы. Счётный гейт по исходнику не видит, работает ли предикат на данных,
//! поэтому страж здесь — тест по реальным данным.
//!
//! Три пути: `List` (`DeviceService::list`), `ListAll` (`DeviceService::export_csv`, он же
//! `DeviceRepository::list_all`) и `ListGrouped` (`DeviceService::list_grouped`).
//! Каждая пара (поле, путь) в [`VERDICTS`] имеет вердикт:
//! - `Narrows` / `Widens` — доказано на данных: нужные строки в выдаче и ненужных нет;
//! - `Ignored` / `Irrelevant` — с письменным обоснованием; «игнорируется» доказано тем, что
//!   результат с полем ИДЕНТИЧЕН результату без него, при том что на другом пути то же значение
//!   поля результат меняет (иначе проверка была бы вакуумной).
//!
//! Новое поле без вердикта краснит тест ДВАЖДЫ:
//! 1. ошибка компиляции исчерпывающей деструктуризации (без `..`) обоих типов `DeviceFilter`;
//! 2. если её обошли привязкой `новое_поле: _` — расхождение имён полей из `Debug`-вывода
//!    доменного и DTO-типов (и serde-ключей DTO) с ключами `VERDICTS`.
//!    Ограничения: `Debug` не видит `#[serde(skip)]`-семантики, serde-ключи не видят полей с
//!    `skip`; вместе эти проверки перекрываются.
//!
//! Все названия и номера ВЫМЫШЛЕННЫЕ (жёсткое условие приватности: репозиторий публичный).

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::device::{DeviceFilter, Pagination};
use trackly_app::services::DeviceService;
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::ports::places::PlaceRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::SqlitePlaceRepository;
use trackly_infra::test_support::test_writer_and_readers;

const NOW: i64 = 1_700_000_000;

// ---------------------------------------------------------------------------
// Матрица вердиктов
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Path {
    List,
    ListAll,
    ListGrouped,
}

const PATHS: [Path; 3] = [Path::List, Path::ListAll, Path::ListGrouped];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    /// Поле сужает выдачу (позитив и негатив на данных).
    Narrows,
    /// Поле расширяет выдачу (мягко-удалённые).
    Widens,
    /// Сужает только при `group_by_condition=true`; при `false` игнорируется.
    NarrowsOnlyWithGroupByCondition,
    /// Осознанно игнорируется: результат с полем равен результату без него.
    Ignored(&'static str),
    /// Не фильтр: выбор ветки/режима, проверяется отдельным тестом.
    Irrelevant(&'static str),
}

use Path::{List, ListAll, ListGrouped};
use Verdict::{Ignored, Irrelevant, Narrows, NarrowsOnlyWithGroupByCondition, Widens};

const GROUPED_COLLAPSE: &str = "list_grouped схлопывает одинаковые устройства по (тип, наименование[, модель]); \
     поле в его SQL не участвует (D-08a: реализация предикатов вынесена из долговой фазы), свёртка работает корректно";
const EXPORT_FORCED: &str =
    "export_csv выгружает ВСЕ устройства выборки: флаги якорей принудительно сброшены, \
     чтобы выгрузка не теряла детали членов якорных групп";

/// Поле, путь, вердикт. Ровно 12 полей x 3 пути.
const VERDICTS: &[(&str, Path, Verdict)] = &[
    ("type_id", List, Narrows),
    ("type_id", ListAll, Narrows),
    ("type_id", ListGrouped, Ignored(GROUPED_COLLAPSE)),
    ("place_id", List, Narrows),
    ("place_id", ListAll, Narrows),
    ("place_id", ListGrouped, Ignored(GROUPED_COLLAPSE)),
    ("status_id", List, Narrows),
    ("status_id", ListAll, Narrows),
    ("status_id", ListGrouped, Narrows),
    ("state", List, Narrows),
    ("state", ListAll, Narrows),
    ("state", ListGrouped, Ignored(GROUPED_COLLAPSE)),
    ("name_prefix", List, Narrows),
    ("name_prefix", ListAll, Narrows),
    ("name_prefix", ListGrouped, NarrowsOnlyWithGroupByCondition),
    ("include_deleted", List, Widens),
    ("include_deleted", ListAll, Widens),
    (
        "include_deleted",
        ListGrouped,
        Ignored("list_grouped всегда отбирает deleted_at_utc IS NULL: мягко-удалённые в свёртку не попадают"),
    ),
    (
        "group_by_condition",
        List,
        Irrelevant("сервис list принудительно ставит group_by_condition=false: поле на этом пути не читается"),
    ),
    (
        "group_by_condition",
        ListAll,
        Irrelevant("export_csv принудительно ставит group_by_condition=false: поле на этом пути не читается"),
    ),
    (
        "group_by_condition",
        ListGrouped,
        Irrelevant("не фильтр, а выбор ветки свёртки: (тип, наименование) против (тип, наименование, модель) и текстовый фильтр"),
    ),
    ("hide_anchor_members", List, Narrows),
    ("hide_anchor_members", ListAll, Ignored(EXPORT_FORCED)),
    ("hide_anchor_members", ListGrouped, Narrows),
    ("anchors_only", List, Narrows),
    ("anchors_only", ListAll, Ignored(EXPORT_FORCED)),
    ("anchors_only", ListGrouped, Ignored(GROUPED_COLLAPSE)),
    ("exclude_anchors", List, Narrows),
    ("exclude_anchors", ListAll, Ignored(EXPORT_FORCED)),
    ("exclude_anchors", ListGrouped, Narrows),
    ("member_of_group_id", List, Narrows),
    ("member_of_group_id", ListAll, Ignored(EXPORT_FORCED)),
    ("member_of_group_id", ListGrouped, Ignored(GROUPED_COLLAPSE)),
    ("group_members_only", List, Narrows),
    ("group_members_only", ListAll, Ignored(EXPORT_FORCED)),
    ("group_members_only", ListGrouped, Ignored(GROUPED_COLLAPSE)),
];

// ---------------------------------------------------------------------------
// Исчерпывающая деструктуризация (БЕЗ `..`): новое поле = ошибка компиляции
// ---------------------------------------------------------------------------

/// Новое поле DTO `DeviceFilter` ломает компиляцию здесь: добавить поле сюда значит
/// заметить, что вердикта у него нет.
fn destructure_dto(filter: DeviceFilter) {
    let DeviceFilter {
        type_id: _,
        place_id: _,
        status_id: _,
        state: _,
        name_prefix: _,
        include_deleted: _,
        group_by_condition: _,
        hide_anchor_members: _,
        anchors_only: _,
        exclude_anchors: _,
        member_of_group_id: _,
        group_members_only: _,
    } = filter;
}

/// То же для доменного `DeviceFilter`: он уже ломает три литерала в `device_service.rs`,
/// но DTO-тип сам по себе их не ломает.
fn destructure_domain(filter: trackly_core::domain::devices::DeviceFilter) {
    let trackly_core::domain::devices::DeviceFilter {
        type_id: _,
        place_id: _,
        status_id: _,
        state: _,
        name_prefix: _,
        include_deleted: _,
        group_by_condition: _,
        hide_anchor_members: _,
        anchors_only: _,
        exclude_anchors: _,
        member_of_group_id: _,
        group_members_only: _,
    } = filter;
}

/// Имена полей из `{:#?}` производного `Debug` (строки с отступом ровно в 4 пробела).
fn debug_field_names<T: std::fmt::Debug>(value: &T) -> BTreeSet<String> {
    let dump = format!("{value:#?}");
    dump.lines()
        .filter(|l| l.starts_with("    ") && !l.starts_with("     "))
        .filter_map(|l| l.trim().split(':').next().map(str::to_string))
        .collect()
}

fn verdict_fields() -> BTreeSet<String> {
    VERDICTS.iter().map(|(f, _, _)| f.to_string()).collect()
}

// ---------------------------------------------------------------------------
// Тесты полноты реестра
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_destructuring_compiles_for_both_types() {
    destructure_dto(DeviceFilter::default());
    destructure_domain(trackly_core::domain::devices::DeviceFilter::default());
}

#[test]
fn every_field_and_path_has_exactly_one_verdict() {
    let fields = verdict_fields();
    assert_eq!(
        fields.len(),
        12,
        "в реестре должно быть 12 полей: {fields:?}"
    );
    for field in &fields {
        for path in PATHS {
            let n = VERDICTS
                .iter()
                .filter(|(f, p, _)| f == field && *p == path)
                .count();
            assert_eq!(
                n, 1,
                "поле {field} x путь {path:?}: вердиктов {n}, ожидался ровно один"
            );
        }
    }
    assert_eq!(VERDICTS.len(), 12 * 3, "матрица обязана быть полной");
}

#[test]
fn verdict_keys_equal_domain_debug_field_names() {
    let names = debug_field_names(&trackly_core::domain::devices::DeviceFilter::default());
    assert_eq!(
        names,
        verdict_fields(),
        "поля доменного DeviceFilter (из Debug) разошлись с ключами VERDICTS: \
         у нового поля нет вердикта (или в таблице лишняя строка)"
    );
}

#[test]
fn verdict_keys_equal_dto_debug_and_serde_field_names() {
    let dto = DeviceFilter::default();
    assert_eq!(
        debug_field_names(&dto),
        verdict_fields(),
        "поля DTO DeviceFilter (из Debug) разошлись с ключами VERDICTS"
    );
    let serde_keys: BTreeSet<String> = serde_json::to_value(&dto)
        .expect("DTO сериализуется")
        .as_object()
        .expect("DTO сериализуется объектом")
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        serde_keys,
        verdict_fields(),
        "serde-ключи DTO DeviceFilter разошлись с ключами VERDICTS"
    );
}

#[test]
fn ignored_and_irrelevant_verdicts_carry_a_real_reason() {
    for (field, path, verdict) in VERDICTS {
        if let Ignored(why) | Irrelevant(why) = verdict {
            assert!(
                why.chars().count() >= 20,
                "{field} x {path:?}: обоснование короче 20 символов: {why:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Данные
// ---------------------------------------------------------------------------

// Ключ устройства = его инвентарный номер.
const A: &str = "РЕГ-01"; // Принтер Альфа, «Б/У», кабинет
const B: &str = "РЕГ-02"; // Принтер Бета, «б/у », этаж
const C: &str = "РЕГ-03"; // Монитор Гамма, «Новое», корпус Б, статус «ремонт»
const C2: &str = "РЕГ-04"; // Монитор Гамма (двойник C), корпус Б
const D: &str = "РЕГ-05"; // Принтер Дельта, тип 2
const E: &str = "РЕГ-06"; // Удалённый Эпсилон (мягко удалён), кабинет, «Б/У»
const F: &str = "РЕГ-07"; // Системный блок Дзета — якорь группы
const G: &str = "РЕГ-08"; // Планка Эта — член группы якоря
const H: &str = "РЕГ-09"; // Монитор Гамма другой модели

const ALL_LIVE: [&str; 8] = [A, B, C, C2, D, F, G, H];

struct Fx {
    svc: DeviceService,
    _dir: tempfile::TempDir,
    korpus_a: i64,
    kabinet: i64,
    repair_status: i64,
    group_id: i64,
    inv_by_id: HashMap<i64, String>,
}

struct Seed<'a> {
    name: &'a str,
    inv: &'a str,
    model: Option<&'a str>,
    type_id: i64,
    state: Option<&'a str>,
    place: Option<i64>,
    status: i64,
    deleted: bool,
}

async fn fixture() -> Fx {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = DeviceService::new(writer, readers, clock);

    type Seeded = (i64, i64, i64, i64, HashMap<i64, String>);
    let (korpus_a, kabinet, repair_status, group_id, inv_by_id): Seeded = svc
        .writer
        .execute(move |conn| {
            let stock: i64 = conn
                .query_row(
                    "SELECT id FROM device_statuses WHERE code = 'на_складе'",
                    [],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;
            let repair: i64 = conn
                .query_row(
                    "SELECT id FROM device_statuses WHERE code IS NOT NULL AND id <> ?1 \
                     ORDER BY id LIMIT 1",
                    params![stock],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;

            let repo = SqlitePlaceRepository;
            let mut mk = |parent: Option<i64>, kind: PlaceKind, name: &str| {
                repo.create(
                    conn,
                    &PlaceNew {
                        parent_id: parent,
                        kind,
                        name: name.to_string(),
                        level: None,
                        is_storage: false,
                        sort_order: None,
                        notes: None,
                    },
                    NOW,
                )
            };
            let korpus_a = mk(None, PlaceKind::Building, "Корпус А")?;
            let etazh = mk(Some(korpus_a), PlaceKind::Floor, "Этаж 1")?;
            let kabinet = mk(Some(etazh), PlaceKind::Room, "Кабинет 1")?;
            let korpus_b = mk(None, PlaceKind::Building, "Корпус Б")?;

            let seeds = [
                Seed { name: "Принтер Альфа", inv: A, model: Some("Модель-1"), type_id: 1, state: Some("Б/У"), place: Some(kabinet), status: stock, deleted: false },
                Seed { name: "Принтер Бета", inv: B, model: Some("Модель-2"), type_id: 1, state: Some("б/у "), place: Some(etazh), status: stock, deleted: false },
                Seed { name: "Монитор Гамма", inv: C, model: Some("Модель-3"), type_id: 1, state: Some("Новое"), place: Some(korpus_b), status: repair, deleted: false },
                Seed { name: "Монитор Гамма", inv: C2, model: Some("Модель-3"), type_id: 1, state: Some("Новое"), place: Some(korpus_b), status: stock, deleted: false },
                Seed { name: "Принтер Дельта", inv: D, model: None, type_id: 2, state: None, place: None, status: stock, deleted: false },
                Seed { name: "Удалённый Эпсилон", inv: E, model: None, type_id: 1, state: Some("Б/У"), place: Some(kabinet), status: stock, deleted: true },
                Seed { name: "Системный блок Дзета", inv: F, model: None, type_id: 1, state: None, place: None, status: stock, deleted: false },
                Seed { name: "Планка Эта", inv: G, model: None, type_id: 1, state: None, place: None, status: stock, deleted: false },
                Seed { name: "Монитор Гамма", inv: H, model: Some("Модель-4"), type_id: 1, state: None, place: None, status: stock, deleted: false },
            ];
            let mut ids: HashMap<&str, i64> = HashMap::new();
            let mut inv_by_id = HashMap::new();
            for s in &seeds {
                conn.execute(
                    "INSERT INTO devices (type_id, name, inventory_number, model, condition, \
                     place_id, status_id, deleted_at_utc, created_at_utc, updated_at_utc, version) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, 1)",
                    params![
                        s.type_id,
                        s.name,
                        s.inv,
                        s.model,
                        s.state,
                        s.place,
                        s.status,
                        if s.deleted { Some(NOW) } else { None },
                        NOW
                    ],
                )
                .map_err(map_rusqlite)?;
                let id = conn.last_insert_rowid();
                ids.insert(s.inv, id);
                inv_by_id.insert(id, s.inv.to_string());
            }

            conn.execute(
                "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc) \
                 VALUES ('reg_t', 'Тип реестра', 'repair', ?1, ?1)",
                params![NOW],
            )
            .map_err(map_rusqlite)?;
            let type_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, parent_group_id, version, \
                 created_at_utc, updated_at_utc) VALUES (?1, 'Разбор реестра', 1, NULL, 1, ?2, ?2)",
                params![type_id, NOW],
            )
            .map_err(map_rusqlite)?;
            let group_id = conn.last_insert_rowid();
            conn.execute(
                "UPDATE groups SET anchor_device_id = ?1 WHERE id = ?2",
                params![ids[F], group_id],
            )
            .map_err(map_rusqlite)?;
            for key in [F, G] {
                conn.execute(
                    "INSERT INTO group_devices (device_id, group_id, added_at_utc) VALUES (?1, ?2, ?3)",
                    params![ids[key], group_id, NOW],
                )
                .map_err(map_rusqlite)?;
            }
            Ok((korpus_a, kabinet, repair, group_id, inv_by_id))
        })
        .await
        .expect("seed registry fixture");

    Fx {
        svc,
        _dir: dir,
        korpus_a,
        kabinet,
        repair_status,
        group_id,
        inv_by_id,
    }
}

// ---------------------------------------------------------------------------
// Запуск пути
// ---------------------------------------------------------------------------

/// Результат пути: плоское множество инвентарных номеров и «форма» (список групп).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Out {
    flat: BTreeSet<String>,
    shape: Vec<Vec<String>>,
}

fn out_from_flat(flat: BTreeSet<String>) -> Out {
    let shape = flat.iter().map(|k| vec![k.clone()]).collect();
    Out { flat, shape }
}

async fn run(fx: &Fx, path: Path, filter: &DeviceFilter) -> Out {
    match path {
        List => {
            let r = fx
                .svc
                .list(
                    filter.clone(),
                    Pagination {
                        offset: 0,
                        limit: 200,
                    },
                )
                .await
                .expect("list");
            let flat: BTreeSet<String> = r
                .items
                .iter()
                .map(|d| d.inventory_no.clone().expect("inventory_no"))
                .collect();
            assert_eq!(
                r.total as usize,
                r.items.len(),
                "total обязан равняться числу строк (общий предикат COUNT и SELECT)"
            );
            assert_eq!(
                flat.len(),
                r.items.len(),
                "дубликатов в выдаче быть не должно"
            );
            out_from_flat(flat)
        }
        ListAll => {
            let csv_text = fx.svc.export_csv(filter.clone()).await.expect("export_csv");
            let mut rdr = csv::ReaderBuilder::new()
                .delimiter(b';')
                .from_reader(csv_text.trim_start_matches('\u{FEFF}').as_bytes());
            let flat: BTreeSet<String> = rdr
                .records()
                .map(|r| {
                    r.expect("csv row")
                        .get(2)
                        .expect("колонка «Инвентарный №»")
                        .to_string()
                })
                .collect();
            out_from_flat(flat)
        }
        ListGrouped => {
            let groups = fx
                .svc
                .list_grouped(
                    filter.clone(),
                    Pagination {
                        offset: 0,
                        limit: 200,
                    },
                )
                .await
                .expect("list_grouped");
            let mut shape: Vec<Vec<String>> = groups
                .iter()
                .map(|g| {
                    let mut v: Vec<String> = g
                        .ids
                        .iter()
                        .map(|id| fx.inv_by_id[&(*id as i64)].clone())
                        .collect();
                    v.sort();
                    v
                })
                .collect();
            shape.sort();
            let flat = shape.iter().flatten().cloned().collect();
            Out { flat, shape }
        }
    }
}

fn set(keys: &[&str]) -> BTreeSet<String> {
    keys.iter().map(|k| k.to_string()).collect()
}

fn all_live() -> BTreeSet<String> {
    set(&ALL_LIVE)
}

fn minus(base: BTreeSet<String>, drop: &[&str]) -> BTreeSet<String> {
    base.into_iter()
        .filter(|k| !drop.contains(&k.as_str()))
        .collect()
}

/// Значение поля, которое на «сужающем» пути даёт непустой, строго меньший (или больший) набор.
fn apply_probe(field: &str, fx: &Fx, f: &mut DeviceFilter) {
    match field {
        "type_id" => f.type_id = Some(2),
        "place_id" => f.place_id = Some(fx.korpus_a),
        "status_id" => f.status_id = Some(fx.repair_status),
        "state" => f.state = Some("б/у".to_string()),
        "name_prefix" => f.name_prefix = Some("Монитор".to_string()),
        "include_deleted" => f.include_deleted = true,
        "hide_anchor_members" => f.hide_anchor_members = true,
        "anchors_only" => f.anchors_only = true,
        "exclude_anchors" => f.exclude_anchors = true,
        "member_of_group_id" => f.member_of_group_id = Some(fx.group_id),
        "group_members_only" => f.group_members_only = true,
        other => panic!("для поля {other} нет зонда: добавьте его в apply_probe"),
    }
}

/// Ожидаемое множество при значении из `apply_probe` на пути, где поле действует.
fn expected_effect(field: &str) -> BTreeSet<String> {
    match field {
        "type_id" => set(&[D]),
        "place_id" => set(&[A, B]),
        "status_id" => set(&[C]),
        "state" => set(&[A, B]),
        "name_prefix" => set(&[C, C2, H]),
        "include_deleted" => {
            let mut s = all_live();
            s.insert(E.to_string());
            s
        }
        "hide_anchor_members" => minus(all_live(), &[G]),
        "anchors_only" => set(&[F]),
        "exclude_anchors" => minus(all_live(), &[F]),
        "member_of_group_id" => set(&[G]),
        // Якорь и член единственной живой группы фикстуры.
        "group_members_only" => set(&[F, G]),
        other => panic!("для поля {other} нет ожидаемого эффекта"),
    }
}

async fn timed<F: std::future::Future>(f: F) -> F::Output {
    tokio::time::timeout(Duration::from_secs(30), f)
        .await
        .expect("таймаут 30 с")
}

// ---------------------------------------------------------------------------
// Поведенческие проверки клеток
// ---------------------------------------------------------------------------

async fn check_narrows_or_widens(fx: &Fx, field: &str, path: Path, widens: bool, gc: bool) {
    let mut base_f = DeviceFilter {
        group_by_condition: gc,
        ..Default::default()
    };
    let base = run(fx, path, &base_f).await;
    assert_eq!(
        base.flat,
        all_live(),
        "{path:?}: базовая выдача без фильтра"
    );
    apply_probe(field, fx, &mut base_f);
    let got = run(fx, path, &base_f).await;
    let expected = expected_effect(field);

    // Позитив: нужные строки присутствуют.
    for k in &expected {
        assert!(
            got.flat.contains(k),
            "{field} x {path:?}: ожидаемая строка {k} отсутствует: {:?}",
            got.flat
        );
    }
    // Негатив: чужих строк нет.
    for k in &got.flat {
        assert!(
            expected.contains(k),
            "{field} x {path:?}: лишняя строка {k} (поле принято, но не сужает): {:?}",
            got.flat
        );
    }
    // Не вакуумно: результат действительно отличается от базового.
    assert_ne!(
        got.flat, base.flat,
        "{field} x {path:?}: зонд не меняет результат — проверка вакуумна"
    );
    if widens {
        assert!(
            got.flat.len() > base.flat.len(),
            "{field} x {path:?}: расширение"
        );
    } else {
        assert!(
            !got.flat.is_empty() && got.flat.len() < base.flat.len(),
            "{field} x {path:?}: сужение должно быть непустым и строгим"
        );
    }
}

/// Результат на пути, где поле действует, ОБЯЗАН отличаться от базового (эталон эффективности зонда).
async fn assert_probe_is_effective_on_list(fx: &Fx, field: &str) {
    let base = run(fx, List, &DeviceFilter::default()).await;
    let mut f = DeviceFilter::default();
    apply_probe(field, fx, &mut f);
    let got = run(fx, List, &f).await;
    assert_ne!(
        got.flat, base.flat,
        "{field}: зонд не меняет результат даже на list — «игнорируется» нечем доказать"
    );
}

async fn check_ignored(fx: &Fx, field: &str, path: Path, modes: &[bool]) {
    assert_probe_is_effective_on_list(fx, field).await;
    for &gc in modes {
        let base_f = DeviceFilter {
            group_by_condition: gc,
            ..Default::default()
        };
        let base = run(fx, path, &base_f).await;
        let mut f = base_f.clone();
        apply_probe(field, fx, &mut f);
        let got = run(fx, path, &f).await;
        assert_eq!(
            got, base,
            "{field} x {path:?} (group_by_condition={gc}): вердикт «игнорируется», но результат изменился"
        );
    }
}

/// Для `list_grouped` игнорирование проверяется в обеих ветках свёртки.
fn both_modes(path: Path) -> &'static [bool] {
    if path == ListGrouped {
        &[false, true]
    } else {
        &[false]
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn every_cell_of_the_matrix_matches_the_data() {
    timed(async {
        let fx = fixture().await;
        for (field, path, verdict) in VERDICTS {
            match verdict {
                Narrows => {
                    let gc = false;
                    check_narrows_or_widens(&fx, field, *path, false, gc).await;
                }
                Widens => check_narrows_or_widens(&fx, field, *path, true, false).await,
                NarrowsOnlyWithGroupByCondition => {
                    // Сужает при group_by_condition=true …
                    check_narrows_or_widens(&fx, field, *path, false, true).await;
                    // … и игнорируется при false.
                    check_ignored(&fx, field, *path, &[false]).await;
                }
                Ignored(_) => check_ignored(&fx, field, *path, both_modes(*path)).await,
                Irrelevant(_) => {}
            }
        }
    })
    .await;
}

// ---------------------------------------------------------------------------
// Именованные кейсы решений
// ---------------------------------------------------------------------------

/// D-02: поддерево места сквозь сервис и выгрузку CSV; по LAN отдаётся не вся организация.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_filter_reaches_csv_export_as_subtree() {
    timed(async {
        let fx = fixture().await;
        let whole = run(&fx, ListAll, &DeviceFilter::default()).await;
        assert_eq!(whole.flat, all_live(), "без фильтра выгружаются все живые");

        let subtree = run(
            &fx,
            ListAll,
            &DeviceFilter {
                place_id: Some(fx.korpus_a),
                ..Default::default()
            },
        )
        .await;
        assert_eq!(
            subtree.flat,
            set(&[A, B]),
            "корпус А: он сам, этаж и кабинет"
        );
        for foreign in [C, C2, D, F, G, H, E] {
            assert!(
                !subtree.flat.contains(foreign),
                "в выгрузке по месту оказался посторонний {foreign}"
            );
        }

        let leaf = run(
            &fx,
            ListAll,
            &DeviceFilter {
                place_id: Some(fx.kabinet),
                ..Default::default()
            },
        )
        .await;
        assert_eq!(
            leaf.flat,
            set(&[A]),
            "лист: родитель (этаж) в выгрузку не попадает"
        );
    })
    .await;
}

/// D-03: одно поле — одно поведение в трёх путях.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn name_prefix_gives_the_same_devices_on_all_three_paths() {
    timed(async {
        let fx = fixture().await;
        let text = Some("Монитор".to_string());
        let want = set(&[C, C2, H]);
        let list = run(
            &fx,
            List,
            &DeviceFilter {
                name_prefix: text.clone(),
                ..Default::default()
            },
        )
        .await;
        let export = run(
            &fx,
            ListAll,
            &DeviceFilter {
                name_prefix: text.clone(),
                ..Default::default()
            },
        )
        .await;
        let grouped = run(
            &fx,
            ListGrouped,
            &DeviceFilter {
                name_prefix: text.clone(),
                group_by_condition: true,
                ..Default::default()
            },
        )
        .await;
        assert_eq!(list.flat, want, "list");
        assert_eq!(export.flat, want, "выгрузка совпадает с экраном");
        assert_eq!(grouped.flat, want, "list_grouped(group_by_condition=true)");

        let grouped_off = run(
            &fx,
            ListGrouped,
            &DeviceFilter {
                name_prefix: text,
                group_by_condition: false,
                ..Default::default()
            },
        )
        .await;
        assert_eq!(
            grouped_off.flat,
            all_live(),
            "при group_by_condition=false текст игнорируется (вердикт реестра)"
        );
    })
    .await;
}

/// Форма фильтра, которую шлёт пикер принтеров группы (`GroupPrintersList.svelte`,
/// `fetchCandidates`): `type_id` + необязательный `name_prefix` + малое окно.
/// Поле-за-раз уже покрыто строками VERDICTS; здесь страхуется сочетание и порядок
/// «сначала тип, потом окно»: малый `limit` не должен вытеснять принтер
/// устройствами других типов. Для контраста зафиксировано, почему пикер не может
/// ходить через `list_grouped`: тот игнорирует `type_id` (D-08a), и окно режется
/// по всем типам.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn printer_picker_filter_shape_narrows_by_type_before_the_window() {
    timed(async {
        let fx = fixture().await;

        // Окно после фильтра типа: живых устройств типа 1 больше одного, но при
        // `type_id = 2` и `limit = 1` приходит единственное устройство типа 2.
        let r = fx
            .svc
            .list(
                DeviceFilter {
                    type_id: Some(2),
                    ..Default::default()
                },
                Pagination {
                    offset: 0,
                    limit: 1,
                },
            )
            .await
            .expect("list");
        assert_eq!(r.items.len(), 1, "в окно попало одно устройство");
        assert_eq!(
            r.items[0].inventory_no.as_deref(),
            Some(D),
            "окно режется ПОСЛЕ фильтра типа: принтер не вытеснен чужими типами"
        );
        assert_eq!(r.total, 1, "total считается по тому же предикату типа");

        // Предикаты комбинируются: текст не «перебивает» тип.
        let text = Some("Монитор".to_string());
        let same_type = run(
            &fx,
            List,
            &DeviceFilter {
                type_id: Some(1),
                name_prefix: text.clone(),
                ..Default::default()
            },
        )
        .await;
        assert_eq!(same_type.flat, set(&[C, C2, H]), "тип 1 + текст «Монитор»");
        let other_type = run(
            &fx,
            List,
            &DeviceFilter {
                type_id: Some(2),
                name_prefix: text.clone(),
                ..Default::default()
            },
        )
        .await;
        assert!(
            other_type.flat.is_empty(),
            "тип 2 + текст «Монитор»: мониторов типа 2 нет, текст не перебивает тип"
        );

        // Контраст: `list_grouped` с тем же типом отдаёт все живые устройства.
        let grouped = run(
            &fx,
            ListGrouped,
            &DeviceFilter {
                type_id: Some(2),
                group_by_condition: true,
                ..Default::default()
            },
        )
        .await;
        assert_eq!(
            grouped.flat,
            all_live(),
            "list_grouped игнорирует type_id (вердикт реестра): тип на нём не отфильтровать"
        );
    })
    .await;
}

/// D-08a: шесть полей `list_grouped` не меняют результат — «игнорируется» доказано данными.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_grouped_ignores_its_six_fields() {
    timed(async {
        let fx = fixture().await;
        let six = [
            "type_id",
            "include_deleted",
            "place_id",
            "state",
            "anchors_only",
            "member_of_group_id",
        ];
        for field in six {
            let verdict = VERDICTS
                .iter()
                .find(|(f, p, _)| *f == field && *p == ListGrouped)
                .map(|(_, _, v)| *v)
                .expect("вердикт есть");
            assert!(
                matches!(verdict, Ignored(_)),
                "{field}: в реестре для list_grouped должен стоять Ignored"
            );
            check_ignored(&fx, field, ListGrouped, &[false, true]).await;
        }
        // Свёртка при этом работает: три «Монитора Гамма» схлопываются по (тип, наименование).
        let off = run(&fx, ListGrouped, &DeviceFilter::default()).await;
        assert!(
            off.shape.iter().any(|g| g.len() == 3),
            "свёртка без модели схлопывает троих: {:?}",
            off.shape
        );
    })
    .await;
}

/// `include_deleted` расширяет `list` и выгрузку: мягко удалённое появляется только при `true`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn include_deleted_widens_list_and_export_only() {
    timed(async {
        let fx = fixture().await;
        for path in [List, ListAll] {
            let off = run(&fx, path, &DeviceFilter::default()).await;
            let on = run(
                &fx,
                path,
                &DeviceFilter {
                    include_deleted: true,
                    ..Default::default()
                },
            )
            .await;
            assert!(
                !off.flat.contains(E),
                "{path:?}: удалённое без флага не видно"
            );
            assert!(on.flat.contains(E), "{path:?}: удалённое с флагом видно");
            assert_eq!(
                on.flat.len(),
                off.flat.len() + 1,
                "{path:?}: добавилось ровно одно"
            );
        }
    })
    .await;
}

/// `group_by_condition` — выбор ветки свёртки, на `list`/выгрузку не влияет.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn group_by_condition_switches_only_the_grouped_branch() {
    timed(async {
        let fx = fixture().await;
        let off = DeviceFilter::default();
        let on = DeviceFilter {
            group_by_condition: true,
            ..Default::default()
        };
        for path in [List, ListAll] {
            assert_eq!(
                run(&fx, path, &off).await,
                run(&fx, path, &on).await,
                "{path:?}: поле не читается"
            );
        }
        let g_off = run(&fx, ListGrouped, &off).await;
        let g_on = run(&fx, ListGrouped, &on).await;
        assert_ne!(g_off.shape, g_on.shape, "ветка свёртки меняется");
        assert!(g_off
            .shape
            .contains(&vec![C.to_string(), C2.to_string(), H.to_string()]));
        assert!(g_on.shape.contains(&vec![C.to_string(), C2.to_string()]));
        assert!(g_on.shape.contains(&vec![H.to_string()]));
    })
    .await;
}

/// Согласованность `total` и числа строк при сужающих фильтрах (общий предикат COUNT/SELECT):
/// проверка сидит в `run(List)`, здесь она прогоняется на каждом сужающем поле явно.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn list_total_equals_row_count_for_every_narrowing_field() {
    timed(async {
        let fx = fixture().await;
        for (field, path, verdict) in VERDICTS {
            if *path == List && matches!(verdict, Narrows | Widens) {
                let mut f = DeviceFilter::default();
                apply_probe(field, &fx, &mut f);
                let got = run(&fx, List, &f).await;
                assert_eq!(got.flat, expected_effect(field), "{field}");
            }
        }
    })
    .await;
}
