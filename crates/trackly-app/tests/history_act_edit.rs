//! Фаза 41.10, UAT-1 (решение пользователя, уточняет D-24): событие с `act_id` живёт ровно
//! столько, сколько строка акта, которая его породила. Правка, исключающая устройство или
//! вложенную группу из акта, СТИРАЕТ события этого акта по субъекту, а восстановление
//! состава, вложенности и статуса идёт молча (как откат). Компенсирующих записей нет: в ленте
//! не остаётся пары «выведено актом №N» / «включено актом №N». Устройство, добавленное
//! правкой, получает обычные события акта. Префикс тестов: `history_act_edit_`. Фикстуры
//! вымышленные.
//!
//! UAT-2 (решение пользователя): строки `place_movements` акта живут по тому же правилу —
//! правка, исключившая позицию, удаляет их, а не дописывает возвратную строку. Исключение:
//! устройство перемещали после акта, и правка возвращает его на место «до акта» — это
//! настоящее изменение, оно пишется строкой с act_id, а вывод из группы — после неё (D-25).

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::act::{
    ActCreateDto, ActItemNewDto, ActNumberEditInput, ActSaveOutcome, ActUpdateDto, ActUpdateItemDto,
};
use trackly_app::dto::number_template::NumberFieldInput;
use trackly_app::dto::reports::{PeriodDto, ReportFilter};
use trackly_core::auth::Identity;
use trackly_infra::error_conversions::map_rusqlite;

async fn make_test_ctx() -> (AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("paths");
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).unwrap_or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        guard
    });
    let ctx = AppCtx::build(paths, config, log_guard).await.expect("ctx");
    (ctx, dir)
}

fn admin() -> Identity {
    Identity::trusted_admin()
}

async fn seed_place(ctx: &AppCtx, name: &str) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, is_storage, created_at_utc, updated_at_utc, version) \
                 VALUES ('room', ?1, 0, 1700000000, 1700000000, 1)",
                params![name],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

async fn seed_group(ctx: &AppCtx, name: &str, place: i64, parent: Option<i64>) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            let type_id: i64 = match conn.query_row(
                "SELECT id FROM group_types WHERE behavior = 'container' LIMIT 1",
                [],
                |r| r.get(0),
            ) {
                Ok(id) => id,
                Err(_) => {
                    conn.execute(
                        "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc) \
                         VALUES ('hae_t', 'Тип теста', 'container', 1700000000, 1700000000)",
                        [],
                    )
                    .map_err(map_rusqlite)?;
                    conn.last_insert_rowid()
                }
            };
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, (SELECT COALESCE(MAX(seq), 0) + 1 FROM groups WHERE type_id = ?1), \
                 ?3, ?4, 1700000000, 1700000000, 1)",
                params![type_id, name, place, parent],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group")
}

async fn seed_device(ctx: &AppCtx, name: &str, place: i64, group: Option<i64>) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, status_id, place_id, version, \
                 created_at_utc, updated_at_utc) VALUES (1, ?1, \
                 (SELECT id FROM device_statuses WHERE code = 'на_складе'), ?2, 1, 1700000000, 1700000000)",
                params![name, place],
            )
            .map_err(map_rusqlite)?;
            let id = conn.last_insert_rowid();
            if let Some(g) = group {
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

async fn set_anchor(ctx: &AppCtx, group: i64, device: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE groups SET anchor_device_id = ?1 WHERE id = ?2",
                params![device, group],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("set anchor");
}

fn create_dto(number: &str, ids: &[i64], place: i64) -> ActCreateDto {
    ActCreateDto {
        number_input: NumberFieldInput {
            value: number.into(),
            template_id: None,
            confirm_mismatch: false,
            confirm_script_mix: false,
        },
        giver_name: "Иванов И.И.".into(),
        receiver_name: "Петров П.П.".into(),
        place_id: Some(place),
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
        confirm_group_release: true,
    }
}

async fn create(ctx: &AppCtx, payload: ActCreateDto) -> i64 {
    match ctx.acts.create(&admin(), payload).await.expect("create") {
        ActSaveOutcome::Created(a) => a.id,
        other => panic!("ожидали Created, получили {other:?}"),
    }
}

/// Правка акта: оставить в нём только `keep`.
async fn edit_keep(ctx: &AppCtx, act_id: i64, keep: &[i64]) {
    let act = ctx.acts.get(act_id).await.expect("get act");
    let dto = ActUpdateDto {
        id: act.id,
        expected_version: act.version,
        number_input: ActNumberEditInput {
            value: act.number_raw.clone(),
            confirm_script_mix: false,
        },
        giver_name: act.giver_name.clone(),
        receiver_name: act.receiver_name.clone(),
        place_id: act.place_id,
        notes: act.notes.clone(),
        deadline_utc: act.deadline_utc,
        handover_date_utc: None,
        items: keep
            .iter()
            .map(|&id| ActUpdateItemDto {
                device_id: id,
                complectation_at_time: None,
            })
            .collect(),
        confirm_group_release: true,
    };
    match ctx.acts.update(&admin(), dto).await.expect("update") {
        ActSaveOutcome::Created(_) => {}
        other => panic!("ожидали сохранение правки, получили {other:?}"),
    }
}

/// Виды событий субъекта, порождённых актом `act` (по порядку записи).
async fn act_kinds(ctx: &AppCtx, subject_type: &'static str, id: i64, act: i64) -> Vec<String> {
    ctx.writer
        .execute(move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT kind FROM history_events \
                     WHERE subject_type = ?1 AND subject_id = ?2 AND act_id = ?3 \
                     ORDER BY created_at_utc, id",
                )
                .map_err(map_rusqlite)?;
            let rows = stmt
                .query_map(params![subject_type, id, act], |r| r.get::<_, String>(0))
                .map_err(map_rusqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_rusqlite)?;
            Ok(rows)
        })
        .await
        .expect("act kinds")
}

/// Число событий с `act_id = act` во всей таблице.
async fn count_act_events(ctx: &AppCtx, act: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM history_events WHERE act_id = ?1",
                params![act],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("count act events")
}

async fn group_of(ctx: &AppCtx, device: i64) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            match conn.query_row(
                "SELECT group_id FROM group_devices WHERE device_id = ?1",
                params![device],
                |r| r.get(0),
            ) {
                Ok(g) => Ok(Some(g)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(map_rusqlite(e)),
            }
        })
        .await
        .expect("group of")
}

async fn parent_of(ctx: &AppCtx, group: i64) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT parent_group_id FROM groups WHERE id = ?1",
                params![group],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("parent of")
}

/// UAT-1 (CR-01): акт по ошибке включил члена группы и вывел его (`group_left` и смена
/// статуса с act_id). Правка исключила устройство: состав восстановлен, а в ленте
/// устройства не осталось ни одного события этого акта — ни `group_left`, ни `group_joined`,
/// ни `status_changed`. События акта по оставшейся позиции живут.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_edit_excluded_member_leaves_no_act_events() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Кабинет Б").await;
    let g = seed_group(&ctx, "АРМ #21", a, None).await;
    let d1 = seed_device(&ctx, "Системный блок #21", a, Some(g)).await;
    seed_device(&ctx, "Монитор #21", a, Some(g)).await;
    seed_device(&ctx, "Клавиатура #21", a, Some(g)).await;
    let free = seed_device(&ctx, "Принтер #21", a, None).await;
    let act = create(&ctx, create_dto("HAE-1", &[d1, free], b)).await;
    assert_eq!(group_of(&ctx, d1).await, None, "акт вывел устройство");
    let before = act_kinds(&ctx, "device", d1, act).await;
    assert!(
        before.iter().any(|k| k == "group_left") && before.iter().any(|k| k == "status_changed"),
        "предусловие: акт записал вывод и смену статуса: {before:?}"
    );

    edit_keep(&ctx, act, &[free]).await;

    assert_eq!(group_of(&ctx, d1).await, Some(g), "правка вернула в группу");
    assert_eq!(
        act_kinds(&ctx, "device", d1, act).await,
        Vec::<String>::new(),
        "исключённое правкой устройство не несёт событий акта"
    );
    assert!(
        act_kinds(&ctx, "device", free, act)
            .await
            .iter()
            .any(|k| k == "status_changed"),
        "события акта по оставшейся позиции не тронуты"
    );
}

/// UAT-1: правка сняла позицию, восстановленное место разошлось с местом группы, в которую
/// устройство вошло уже после акта. Смена статуса акта стёрта (статус восстановлен молча),
/// а вывод из группы — новое следствие правки этого акта, а не восстановление: остаётся одно
/// событие `group_left` с act_id, и лента совпадает с составом.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_edit_locked_release_keeps_only_group_left() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Кабинет Б").await;
    let d = seed_device(&ctx, "Монитор #22", a, None).await;
    let free = seed_device(&ctx, "Принтер #22", a, None).await;
    let act = create(&ctx, create_dto("HAE-2", &[d, free], b)).await;
    assert!(
        act_kinds(&ctx, "device", d, act)
            .await
            .iter()
            .any(|k| k == "status_changed"),
        "предусловие: акт сменил статус"
    );
    // После акта устройство вошло в группу на месте Б (ввод вручную, с событием).
    let h = seed_group(&ctx, "АРМ #22", b, None).await;
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
                 VALUES (?1, ?2, 1800000000)",
                params![d, h],
            )
            .map_err(map_rusqlite)?;
            conn.execute(
                "INSERT INTO history_events (operation_id, subject_type, subject_id, kind, \
                 source, to_group_id, to_group_label, created_at_utc) \
                 VALUES ('manual-join', 'device', ?1, 'group_joined', 'manual', ?2, 'АРМ #22', 1700000001)",
                params![d, h],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("manual join");

    edit_keep(&ctx, act, &[free]).await;

    assert_eq!(group_of(&ctx, d).await, None, "место разошлось — вывод");
    assert_eq!(
        act_kinds(&ctx, "device", d, act).await,
        vec!["group_left".to_string()],
        "смена статуса акта стёрта, вывод из группы записан событием"
    );
}

/// UAT-1 (CR-02): акт отцепил вложенную группу (`group_unnested` с act_id); правка сняла всё
/// её покрытие. Вложенность восстановлена молча, событий этого акта у группы и у её якоря нет.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_edit_excluded_nested_group_leaves_no_act_events() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Кабинет Б").await;
    let container = seed_group(&ctx, "АРМ #23", a, None).await;
    seed_device(&ctx, "Монитор #23", a, Some(container)).await;
    let nested = seed_group(&ctx, "Замена #23", a, Some(container)).await;
    let laptop = seed_device(&ctx, "Ноутбук #23", a, Some(nested)).await;
    set_anchor(&ctx, nested, laptop).await;
    seed_device(&ctx, "Адаптер #23", a, Some(nested)).await;
    let free = seed_device(&ctx, "Принтер #23", a, None).await;
    let act = create(&ctx, create_dto("HAE-3", &[laptop, free], b)).await;
    assert_eq!(parent_of(&ctx, nested).await, None, "акт отцепил группу");
    assert_eq!(
        act_kinds(&ctx, "group", nested, act).await,
        vec!["group_unnested".to_string()],
        "предусловие: отцепление записано событием акта"
    );

    edit_keep(&ctx, act, &[free]).await;

    assert_eq!(
        parent_of(&ctx, nested).await,
        Some(container),
        "правка вернула вложенность"
    );
    assert_eq!(
        act_kinds(&ctx, "group", nested, act).await,
        Vec::<String>::new(),
        "у исключённой группы нет событий акта"
    );
    assert_eq!(
        act_kinds(&ctx, "device", laptop, act).await,
        Vec::<String>::new(),
        "у якоря исключённой группы нет событий акта"
    );
}

/// UAT-1: устройство, ДОБАВЛЕННОЕ правкой, получает обычные события акта (вывод из группы и
/// смену статуса с act_id).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_edit_added_device_gets_act_events() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Кабинет Б").await;
    let g = seed_group(&ctx, "АРМ #24", a, None).await;
    let d = seed_device(&ctx, "Системный блок #24", a, Some(g)).await;
    seed_device(&ctx, "Монитор #24", a, Some(g)).await;
    let free = seed_device(&ctx, "Принтер #24", a, None).await;
    let act = create(&ctx, create_dto("HAE-4", &[free], b)).await;
    assert_eq!(
        act_kinds(&ctx, "device", d, act).await,
        Vec::<String>::new()
    );

    edit_keep(&ctx, act, &[free, d]).await;

    assert_eq!(
        group_of(&ctx, d).await,
        None,
        "акт вывел добавленное устройство"
    );
    let kinds = act_kinds(&ctx, "device", d, act).await;
    assert!(
        kinds.iter().any(|k| k == "group_left") && kinds.iter().any(|k| k == "status_changed"),
        "добавленное правкой устройство получает события акта: {kinds:?}"
    );
}

/// UAT-1 / D-24: удаление акта после правок (исключение и добавление) не оставляет ни одного
/// события с его act_id.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_edit_then_delete_leaves_no_act_events() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Кабинет Б").await;
    let g = seed_group(&ctx, "АРМ #25", a, None).await;
    let d1 = seed_device(&ctx, "Системный блок #25", a, Some(g)).await;
    seed_device(&ctx, "Монитор #25", a, Some(g)).await;
    let d2 = seed_device(&ctx, "Ноутбук #25", a, None).await;
    let free = seed_device(&ctx, "Принтер #25", a, None).await;
    let act = create(&ctx, create_dto("HAE-5", &[d1, free], b)).await;
    edit_keep(&ctx, act, &[free, d2]).await;
    assert!(
        count_act_events(&ctx, act).await > 0,
        "предусловие: события акта есть"
    );

    let live = ctx.acts.get(act).await.expect("get act");
    ctx.acts
        .delete_soft(live.id, live.version)
        .await
        .expect("delete");

    assert_eq!(
        count_act_events(&ctx, act).await,
        0,
        "события удалённого акта стёрты"
    );
    assert_eq!(group_of(&ctx, d1).await, Some(g), "состав цел");
}

/// UAT-1: правка исключила устройство, но в группу оно не вернулось (группу перенесли после
/// акта). Смена статуса акта стёрта, а `group_left` акта остаётся: вывод по-прежнему правда.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_edit_skipped_rejoin_keeps_group_left() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Кабинет Б").await;
    let c = seed_place(&ctx, "Кабинет В").await;
    let g = seed_group(&ctx, "АРМ #26", a, None).await;
    let d1 = seed_device(&ctx, "Системный блок #26", a, Some(g)).await;
    seed_device(&ctx, "Монитор #26", a, Some(g)).await;
    let free = seed_device(&ctx, "Принтер #26", a, None).await;
    let act = create(&ctx, create_dto("HAE-6", &[d1, free], b)).await;
    // После акта группу перенесли в другое место.
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE groups SET place_id = ?1 WHERE id = ?2",
                params![c, g],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("move group");

    edit_keep(&ctx, act, &[free]).await;

    assert_eq!(group_of(&ctx, d1).await, None, "возврат в группу пропущен");
    assert_eq!(
        act_kinds(&ctx, "device", d1, act).await,
        vec!["group_left".to_string()],
        "вывод актом остаётся правдой, смена статуса стёрта"
    );
    // R8-03: строка акта забыта правкой — оставшийся вывод не ссылается на неё.
    assert_eq!(
        dangling_movement_refs(&ctx).await,
        0,
        "group_left не ссылается на стёртую строку перемещения"
    );
}

// ---------------------------------------------------------------------------
// UAT-2: строки перемещений акта по исключённой позиции
// ---------------------------------------------------------------------------

/// Строки журнала мест устройства по порядку записи: (id, from, to, source, act_id).
async fn device_movements(ctx: &AppCtx, device: i64) -> Vec<(i64, i64, i64, String, Option<i64>)> {
    ctx.writer
        .execute(move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, from_place_id, to_place_id, source, act_id FROM place_movements \
                     WHERE entity_type = 'device' AND entity_id = ?1 ORDER BY id",
                )
                .map_err(map_rusqlite)?;
            let rows = stmt
                .query_map(params![device], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
                })
                .map_err(map_rusqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_rusqlite)?;
            Ok(rows)
        })
        .await
        .expect("device movements")
}

async fn scalar(ctx: &AppCtx, sql: &'static str, act: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, params![act], |r| r.get(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("scalar")
}

/// События, ссылающиеся на несуществующую строку журнала мест.
async fn dangling_movement_refs(ctx: &AppCtx) -> i64 {
    scalar(
        ctx,
        "SELECT COUNT(*) FROM history_events e WHERE e.movement_id IS NOT NULL \
         AND ?1 = ?1 AND NOT EXISTS (SELECT 1 FROM place_movements m WHERE m.id = e.movement_id)",
        0,
    )
    .await
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

/// Имена устройств в отчёте «Перемещения» (HST-04).
async fn report_device_names(ctx: &AppCtx) -> Vec<String> {
    ctx.reports
        .list_movements(ReportFilter::default(), wide_period())
        .await
        .expect("отчёт перемещений")
        .rows
        .into_iter()
        .filter_map(|r| r.device_name)
        .collect()
}

/// UAT-2: акт переместил два устройства А -> Б, правка исключила одно. У исключённого нет ни
/// строки акта, ни возвратной строки: лента места и отчёт «Перемещения» о нём молчат. Строка
/// оставшейся позиции жива.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_edit_excluded_device_leaves_no_act_movements() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Кабинет Б").await;
    let d = seed_device(&ctx, "Монитор #27", a, None).await;
    let free = seed_device(&ctx, "Принтер #27", a, None).await;
    let act = create(&ctx, create_dto("HAE-7", &[d, free], b)).await;
    assert_eq!(
        device_movements(&ctx, d).await.len(),
        1,
        "предусловие: А -> Б актом"
    );

    edit_keep(&ctx, act, &[free]).await;

    assert_eq!(
        device_movements(&ctx, d).await,
        Vec::new(),
        "исключённое устройство «не перемещалось»: ни строки акта, ни возвратной"
    );
    let kept = device_movements(&ctx, free).await;
    assert_eq!(kept.len(), 1, "строка оставшейся позиции жива");
    assert_eq!((kept[0].1, kept[0].2, kept[0].4), (a, b, Some(act)));
    let timeline = ctx
        .history
        .get_timeline(&admin(), "device", d)
        .await
        .expect("лента");
    assert!(
        timeline.iter().all(|e| e.act_id != Some(act)),
        "в ленте исключённого устройства нет записей акта: {timeline:?}"
    );
    let names = report_device_names(&ctx).await;
    assert!(
        !names.iter().any(|n| n == "Монитор #27"),
        "отчёт «Перемещения» не показывает исключённое устройство: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "Принтер #27"),
        "отчёт показывает оставшуюся позицию: {names:?}"
    );
    assert_eq!(dangling_movement_refs(&ctx).await, 0);
}

/// UAT-2 (исключение, подтверждено пользователем): после акта устройство вручную ввели в группу
/// на новом месте В. Правка вернула его на место «до акта» А — это настоящее изменение: строка
/// акта А -> Б забыта (ручная строка стала А -> В), возврат В -> А пишется строкой с act_id, а
/// вывод из группы — после неё и с её movement_id (D-25).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_edit_real_change_writes_return_movement_then_group_left() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Кабинет Б").await;
    let c = seed_place(&ctx, "Кабинет В").await;
    let d = seed_device(&ctx, "Монитор #28", a, None).await;
    let free = seed_device(&ctx, "Принтер #28", a, None).await;
    let act = create(&ctx, create_dto("HAE-8", &[d, free], b)).await;
    let h = seed_group(&ctx, "АРМ #28", c, None).await;
    ctx.groups
        .add_devices(&admin(), h, vec![d])
        .await
        .expect("ввод в группу на месте В");
    assert_eq!(group_of(&ctx, d).await, Some(h));
    let before = device_movements(&ctx, d).await;
    assert_eq!(
        before.iter().map(|m| (m.1, m.2)).collect::<Vec<_>>(),
        vec![(a, b), (b, c)],
        "предусловие: А -> Б актом, Б -> В вводом в группу"
    );

    edit_keep(&ctx, act, &[free]).await;

    assert_eq!(
        group_of(&ctx, d).await,
        None,
        "место разошлось с группой — вывод"
    );
    let after = device_movements(&ctx, d).await;
    assert_eq!(
        after.len(),
        2,
        "строка акта забыта, возврат записан: {after:?}"
    );
    assert_eq!(
        (after[0].1, after[0].2),
        (a, c),
        "преемник забытой строки получил её «откуда»"
    );
    assert_ne!(after[0].4, Some(act));
    let (ret_id, from, to, source, act_id) = after[1].clone();
    assert_eq!(
        (from, to, source.as_str(), act_id),
        (c, a, "act", Some(act))
    );
    let left: (i64, Option<i64>) = ctx
        .writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT COUNT(*), MAX(movement_id) FROM history_events \
                 WHERE subject_type = 'device' AND subject_id = ?1 AND act_id = ?2 \
                   AND kind = 'group_left'",
                params![d, act],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("group_left");
    assert_eq!(
        left,
        (1, Some(ret_id)),
        "вывод из группы записан после возврата и несёт его movement_id (D-25)"
    );
    assert_eq!(
        act_kinds(&ctx, "device", d, act).await,
        vec!["group_left".to_string()],
        "события акта, кроме нового вывода, стёрты"
    );
    assert_eq!(dangling_movement_refs(&ctx).await, 0);
}

/// R8-03: правка с настоящим изменением записала возвратную строку и `group_left` с её
/// movement_id; повторное добавление и повторное исключение позиции забывают строки акта
/// по устройству, включая возвратную, — оставшийся `group_left` не должен ссылаться на неё.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_edit_reexclude_leaves_no_dangling_group_left_movement() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Кабинет Б").await;
    let c = seed_place(&ctx, "Кабинет В").await;
    let d = seed_device(&ctx, "Монитор #30", a, None).await;
    let free = seed_device(&ctx, "Принтер #30", a, None).await;
    let act = create(&ctx, create_dto("HAE-10", &[d, free], b)).await;
    let h = seed_group(&ctx, "АРМ #30", c, None).await;
    ctx.groups
        .add_devices(&admin(), h, vec![d])
        .await
        .expect("ввод в группу на месте В");
    edit_keep(&ctx, act, &[free]).await;
    assert_eq!(dangling_movement_refs(&ctx).await, 0, "предусловие");

    edit_keep(&ctx, act, &[free, d]).await;
    edit_keep(&ctx, act, &[free]).await;

    assert_eq!(
        dangling_movement_refs(&ctx).await,
        0,
        "group_left не ссылается на стёртую строку перемещения"
    );
}

/// UAT-2 / D-03: удаление акта после правки (исключение и добавление) не оставляет ни строк
/// журнала мест, ни событий с его act_id.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_edit_then_delete_leaves_no_act_movements() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Кабинет Б").await;
    let d1 = seed_device(&ctx, "Монитор #29", a, None).await;
    let d2 = seed_device(&ctx, "Ноутбук #29", a, None).await;
    let free = seed_device(&ctx, "Принтер #29", a, None).await;
    let act = create(&ctx, create_dto("HAE-9", &[d1, free], b)).await;
    edit_keep(&ctx, act, &[free, d2]).await;
    assert!(
        scalar(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE act_id = ?1",
            act
        )
        .await
            > 0,
        "предусловие: строки акта есть"
    );

    let live = ctx.acts.get(act).await.expect("get act");
    ctx.acts
        .delete_soft(live.id, live.version)
        .await
        .expect("delete");

    assert_eq!(
        scalar(
            &ctx,
            "SELECT COUNT(*) FROM place_movements WHERE act_id = ?1",
            act
        )
        .await,
        0,
        "строк удалённого акта нет"
    );
    assert_eq!(
        count_act_events(&ctx, act).await,
        0,
        "событий удалённого акта нет"
    );
    for dev in [d1, d2, free] {
        assert_eq!(
            device_movements(&ctx, dev).await,
            Vec::new(),
            "устройство «не перемещалось»"
        );
    }
    assert_eq!(dangling_movement_refs(&ctx).await, 0);
}
