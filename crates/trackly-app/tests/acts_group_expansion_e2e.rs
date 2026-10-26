//! Phase 41.2 Plan 16: сквозная проверка «разворот группы -> акт» (D-03, D-04, D-05).
//!
//! Единицы, которые отдаёт `groups_act_expansion` (план 06), подаются в создание акта
//! (план 07) как позиции. Проверка не делается ни в плане 06 (там нет акта), ни в
//! плане 07 (он не зависит от плана 06): её место здесь, в плане, который зависит от
//! обоих. Реальный `AppCtx` на временном каталоге; имена вымышленные.
//!
//! Префикс тестов: `e2e_`.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto, ActSaveOutcome};
use trackly_app::dto::groups::GroupCreateDto;
use trackly_app::dto::number_template::NumberFieldInput;
use trackly_core::auth::Identity;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::SqliteGroupRepository;

async fn make_test_ctx() -> (AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let paths =
        trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("resolve paths");
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    });
    let ctx = AppCtx::build(paths, config, log_guard.expect("log guard"))
        .await
        .expect("build ctx");
    (ctx, dir)
}

fn admin() -> Identity {
    Identity::trusted_admin()
}

async fn seed_place(ctx: &AppCtx, name: &str, parent: Option<i64>) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, parent_id, is_storage, created_at_utc, \
                 updated_at_utc, version) VALUES ('room', ?1, ?2, 0, 1700000000, 1700000000, 1)",
                params![name, parent],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

async fn seed_device(ctx: &AppCtx, name: &str, inv: &str, place: Option<i64>) -> i64 {
    let (name, inv) = (name.to_string(), inv.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, inventory_number, serial_number, place_id, \
                 status_id, created_at_utc, updated_at_utc, version) \
                 VALUES (1, ?1, ?2, ?3, ?4, 1, 1700000000, 1700000000, 1)",
                params![name, inv, format!("SN-{inv}"), place],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
}

async fn type_by_behavior(ctx: &AppCtx, behavior: &'static str) -> i64 {
    ctx.writer
        .execute(move |conn| {
            let found: Option<i64> = conn
                .query_row(
                    "SELECT id FROM group_types WHERE behavior = ?1 LIMIT 1",
                    params![behavior],
                    |r| r.get(0),
                )
                .ok();
            if let Some(id) = found {
                return Ok(id);
            }
            conn.execute(
                "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, ?3, 1700000000, 1700000000)",
                params![format!("exp_t_{behavior}"), "Тип теста", behavior],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("type by behavior")
}

async fn make_group(
    ctx: &AppCtx,
    behavior: &'static str,
    name: &str,
    place: Option<i32>,
) -> trackly_app::dto::groups::GroupDto {
    let type_id = type_by_behavior(ctx, behavior).await;
    ctx.groups
        .create_group(
            &admin(),
            GroupCreateDto {
                type_id,
                name: Some(name.to_string()),
                place_id: place,
                source_device_id: None,
            },
        )
        .await
        .expect("create_group")
}

/// Группа-якорь: якорное устройство входит в состав, затем ставится якорем.
async fn make_anchor_group(
    ctx: &AppCtx,
    behavior: &'static str,
    name: &str,
    anchor_name: &str,
    inv: &str,
) -> (i64, i64) {
    let g = make_group(ctx, behavior, name, None).await;
    let anchor = seed_device(ctx, anchor_name, inv, None).await;
    ctx.groups
        .add_devices(&admin(), g.id, vec![anchor])
        .await
        .expect("add anchor");
    let gid = g.id;
    ctx.writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            SqliteGroupRepository.set_anchor_in_tx(&tx, gid, anchor)?;
            tx.commit().map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set anchor");
    (gid, anchor)
}

async fn nest(ctx: &AppCtx, child: i64, parent: i64) {
    let dto = ctx.groups.get_group(&admin(), child).await.expect("child");
    ctx.groups
        .set_parent(&admin(), child, dto.version, Some(parent))
        .await
        .expect("set_parent");
}

/// Фикстура пункта (a): контейнер «АРМ #3» с двумя прямыми устройствами, вложенным
/// контейнером (одно устройство) и вложенной якорной группой (якорь + две детали).
struct Fixture {
    root: i64,
    inner: i64,
    anchor_group: i64,
    direct: Vec<i64>,
    nested_dev: i64,
    anchor: i64,
    parts: Vec<i64>,
}

async fn build_fixture(ctx: &AppCtx) -> Fixture {
    let root = make_group(ctx, "container", "АРМ #3", None).await;
    let d1 = seed_device(ctx, "Системный блок #4", "INV-001", None).await;
    let d2 = seed_device(ctx, "Монитор #2", "INV-002", None).await;
    ctx.groups
        .add_devices(&admin(), root.id, vec![d1, d2])
        .await
        .unwrap();

    let inner = make_group(ctx, "container", "Периферия #1", None).await;
    let n1 = seed_device(ctx, "Клавиатура #1", "INV-003", None).await;
    ctx.groups
        .add_devices(&admin(), inner.id, vec![n1])
        .await
        .unwrap();
    nest(ctx, inner.id, root.id).await;

    let (anchor_group, anchor) =
        make_anchor_group(ctx, "substitute", "Замена #5", "Ноутбук #5", "INV-004").await;
    let p1 = seed_device(ctx, "Адаптер #5", "INV-005", None).await;
    let p2 = seed_device(ctx, "Мышь #5", "INV-006", None).await;
    ctx.groups
        .add_devices(&admin(), anchor_group, vec![p1, p2])
        .await
        .unwrap();
    nest(ctx, anchor_group, root.id).await;

    Fixture {
        root: root.id,
        inner: inner.id,
        anchor_group,
        direct: vec![d1, d2],
        nested_dev: n1,
        anchor,
        parts: vec![p1, p2],
    }
}

fn dto(number: &str, ids: &[i64], place: Option<i64>, confirm: bool) -> ActCreateDto {
    ActCreateDto {
        number_input: NumberFieldInput {
            value: number.into(),
            template_id: None,
            confirm_mismatch: false,
            confirm_script_mix: false,
        },
        giver_name: "Иванов И.И.".into(),
        receiver_name: "Петров П.П.".into(),
        place_id: place,
        notes: None,
        deadline_utc: None,
        handover_date_utc: None,
        items: ids
            .iter()
            .map(|&id| ActItemNewDto {
                device_id: id,
                device_ids: vec![id],
                quantity: 1,
            })
            .collect(),
        confirm_group_release: confirm,
    }
}

async fn scalar(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("scalar")
}

fn int(v: i64) -> rusqlite::types::Value {
    rusqlite::types::Value::Integer(v)
}

/// Ставит всем живым группам и устройствам фикстуры место `place` (группы и устройства
/// фикстуры создаются «спящими»: без места).
async fn put_everything_on(ctx: &AppCtx, place: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute("UPDATE groups SET place_id = ?1", params![place])
                .map_err(map_rusqlite)?;
            conn.execute("UPDATE devices SET place_id = ?1", params![place])
                .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("put everything on a place");
}

/// Счётчики шести таблиц, которые запись акта меняет первой.
async fn counters(ctx: &AppCtx) -> Vec<i64> {
    ctx.writer
        .execute(move |conn| {
            let mut out = Vec::new();
            for t in [
                "acts",
                "act_items",
                "group_devices",
                "place_movements",
                "audit_log",
                "act_group_effects",
            ] {
                let n: i64 = conn
                    .query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))
                    .map_err(map_rusqlite)?;
                out.push(n);
            }
            Ok(out)
        })
        .await
        .expect("counters")
}

async fn expansion_ids(ctx: &AppCtx, root: i64) -> Vec<i64> {
    ctx.groups
        .act_expansion(&admin(), root)
        .await
        .expect("act_expansion")
        .units
        .iter()
        .map(|u| u.device_id)
        .collect()
}

/// (a) Единицы разворота, поданные в акт целиком, переносят всю группу без подтверждения.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn e2e_expansion_units_fed_to_create_move_the_whole_group() {
    let (ctx, _dir) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А", None).await;
    let b = seed_place(&ctx, "Кабинет Б", None).await;
    let fx = build_fixture(&ctx).await;
    put_everything_on(&ctx, a).await;

    let units = expansion_ids(&ctx, fx.root).await;
    assert_eq!(units.len(), 4, "две прямые + вложенная + один якорь");
    for d in fx.direct.iter().chain([&fx.nested_dev, &fx.anchor]) {
        assert!(units.contains(d), "единица {d} есть в развороте");
    }
    for p in &fx.parts {
        assert!(!units.contains(p), "деталей якорной группы в единицах нет");
    }
    let members_before = scalar(&ctx, "SELECT COUNT(*) FROM group_devices", vec![]).await;

    let act = match ctx
        .acts
        .create(&admin(), dto("E2E-1", &units, Some(b), false))
        .await
        .expect("create")
    {
        ActSaveOutcome::Created(act) => act,
        other => panic!("единицы разворота целиком обязаны пройти без подтверждения: {other:?}"),
    };

    for g in [fx.root, fx.inner, fx.anchor_group] {
        assert_eq!(
            scalar(
                &ctx,
                "SELECT place_id FROM groups WHERE id = ?1",
                vec![int(g)]
            )
            .await,
            b,
            "группа {g} переехала на место акта"
        );
    }
    assert_eq!(
        scalar(&ctx, "SELECT COUNT(*) FROM group_devices", vec![]).await,
        members_before,
        "все строки group_devices на месте"
    );
    for p in &fx.parts {
        assert_eq!(
            scalar(
                &ctx,
                "SELECT COUNT(*) FROM act_items WHERE act_id = ?1 AND device_id = ?2",
                vec![int(act.id), int(*p)]
            )
            .await,
            0,
            "деталь {p} не стала позицией акта"
        );
        assert_eq!(
            scalar(
                &ctx,
                "SELECT place_id FROM devices WHERE id = ?1",
                vec![int(*p)]
            )
            .await,
            b,
            "деталь едет вместе с якорной группой"
        );
    }
    assert_eq!(
        scalar(
            &ctx,
            "SELECT COUNT(*) FROM act_group_effects WHERE act_id = ?1 AND kind = 'move'",
            vec![int(act.id)]
        )
        .await,
        1,
        "ровно один эффект move"
    );
}

/// (b) Те же единицы минус одно устройство: подтверждение выноса БЕЗ единой записи.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn e2e_expansion_units_minus_one_raise_confirmation_without_any_write() {
    let (ctx, _dir) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А", None).await;
    let b = seed_place(&ctx, "Кабинет Б", None).await;
    let fx = build_fixture(&ctx).await;
    put_everything_on(&ctx, a).await;

    let mut units = expansion_ids(&ctx, fx.root).await;
    let dropped = units.pop().expect("есть единицы");
    let before = counters(&ctx).await;

    let warning = match ctx
        .acts
        .create(&admin(), dto("E2E-2", &units, Some(b), false))
        .await
        .expect("create")
    {
        ActSaveOutcome::NeedsGroupRelease(w) => w,
        other => panic!("неполный состав обязан поднять подтверждение выноса: {other:?}"),
    };
    // Корень «АРМ #3» покрыт не целиком, поэтому из его состава выносятся ВСЕ оставшиеся
    // в акте единицы (D-05): три устройства, а не одно.
    assert_eq!(
        warning.items.len(),
        units.len(),
        "выносятся все оставшиеся единицы покрытого не целиком корня"
    );
    assert_eq!(
        counters(&ctx).await,
        before,
        "acts, act_items, group_devices, place_movements, audit_log, act_group_effects не менялись"
    );
    assert_eq!(
        scalar(
            &ctx,
            "SELECT place_id FROM devices WHERE id = ?1",
            vec![int(dropped)]
        )
        .await,
        a,
        "невыбранное устройство на прежнем месте"
    );
    for g in [fx.root, fx.inner, fx.anchor_group] {
        assert_eq!(
            scalar(
                &ctx,
                "SELECT place_id FROM groups WHERE id = ?1",
                vec![int(g)]
            )
            .await,
            a,
            "группа {g} не двигалась"
        );
    }
}

/// (b2) Минимальная расходящаяся пара: контейнер из двух устройств. Оба в акте — перенос
/// целиком; одно из двух — подтверждение со списком РОВНО из одного выносимого устройства.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn e2e_expansion_two_device_container_minus_one_lists_exactly_one() {
    let (ctx, _dir) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А", None).await;
    let b = seed_place(&ctx, "Кабинет Б", None).await;
    let g = make_group(&ctx, "container", "АРМ #8", None).await;
    let x = seed_device(&ctx, "Монитор #8", "INV-E8A", None).await;
    let y = seed_device(&ctx, "Клавиатура #8", "INV-E8B", None).await;
    ctx.groups
        .add_devices(&admin(), g.id, vec![x, y])
        .await
        .expect("add devices");
    put_everything_on(&ctx, a).await;

    let units = expansion_ids(&ctx, g.id).await;
    assert_eq!(units.len(), 2);
    let before = counters(&ctx).await;

    let one = &units[..1];
    let warning = match ctx
        .acts
        .create(&admin(), dto("E2E-3", one, Some(b), false))
        .await
        .expect("create")
    {
        ActSaveOutcome::NeedsGroupRelease(w) => w,
        other => panic!("одно из двух обязано поднять подтверждение: {other:?}"),
    };
    assert_eq!(warning.items.len(), 1, "выносимое устройство одно");
    assert_eq!(warning.items[0].group_name, "АРМ #8");
    assert_eq!(
        counters(&ctx).await,
        before,
        "ни одной записи до подтверждения"
    );

    // Контроль расхождения: те же два устройства целиком — без подтверждения.
    match ctx
        .acts
        .create(&admin(), dto("E2E-3", &units, Some(b), false))
        .await
        .expect("create")
    {
        ActSaveOutcome::Created(_) => {}
        other => panic!("полный состав обязан пройти без подтверждения: {other:?}"),
    }
}
