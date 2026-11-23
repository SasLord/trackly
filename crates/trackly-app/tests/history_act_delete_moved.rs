//! UAT-3 (фаза 41.10): удаление акта, когда устройства акта перемещали после акта.
//!
//! Акт №N переместил устройство А -> Б, потом его вручную перевели Б -> В. Раньше удаление
//! акта молча возвращало устройство в А, стирало строку акта и оставляло ручную строку
//! «Б -> В»: лента говорила «в В», устройство стояло в А. Теперь сервер без выбора ничего не
//! меняет и возвращает `needs_moved_after_choice`; выбор применяется ко всем перечисленным
//! устройствам. Префикс тестов: `history_act_delete_moved_`. Фикстуры вымышленные.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::act::{
    ActCreateDto, ActDeleteResultDto, ActItemNewDto, ActReturnDto, ActReturnItemDto,
    GroupRestoreChoiceDto, MovedAfterActChoiceDto,
};
use trackly_app::dto::device::DevicePatch;
use trackly_app::dto::number_template::NumberFieldInput;
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

async fn seed_device(ctx: &AppCtx, name: &str, place: i64) -> i64 {
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
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
}

fn create_dto(number: &str, place: i64, devices: &[i64]) -> ActCreateDto {
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
        items: devices
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

async fn create(ctx: &AppCtx, number: &str, place: i64, devices: &[i64]) -> (i64, i64) {
    let act = ctx
        .acts
        .create(&admin(), create_dto(number, place, devices))
        .await
        .expect("create act")
        .expect_created("create act");
    (act.id, act.version)
}

async fn act_version(ctx: &AppCtx, act: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT version FROM acts WHERE id = ?1",
                params![act],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("act version")
}

async fn act_deleted(ctx: &AppCtx, act: i64) -> bool {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT deleted_at_utc IS NOT NULL FROM acts WHERE id = ?1",
                params![act],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("act deleted")
}

/// (place_id, status_id, version)
async fn device_state(ctx: &AppCtx, device: i64) -> (Option<i64>, i64, i64) {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT place_id, status_id, version FROM devices WHERE id = ?1",
                params![device],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("device state")
}

/// Отдельный статус справочника для «после акта» (не совпадает ни с одним штатным).
async fn seed_status(ctx: &AppCtx, name: &'static str) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO device_statuses (name, code) VALUES (?1, NULL)",
                params![name],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("status")
}

fn patch(place_id: i64, status_id: Option<i64>) -> DevicePatch {
    DevicePatch {
        type_id: None,
        name: None,
        number_input: None,
        serial_no: None,
        model: None,
        specs: None,
        kit: None,
        state: None,
        place_id: Some(Some(place_id)),
        status_id,
        usb_host_device_id: None,
        printer_ip_address: None,
        printer_community: None,
        group_write_off: None,
    }
}

/// Ручное перемещение (форма карточки): пишет строку `manual` и, при смене статуса, событие.
async fn move_manually(ctx: &AppCtx, device: i64, place: i64, status: Option<i64>) {
    let (_, _, version) = device_state(ctx, device).await;
    ctx.devices
        .update(&admin(), device, version, patch(place, status))
        .await
        .expect("ручное перемещение");
}

/// Строки журнала мест устройства по порядку: (id, from, to, source, act_id, note).
type Mv = (i64, i64, i64, String, Option<i64>, Option<String>);

async fn device_movements(ctx: &AppCtx, device: i64) -> Vec<Mv> {
    ctx.writer
        .execute(move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, from_place_id, to_place_id, source, act_id, note FROM place_movements \
                     WHERE entity_type = 'device' AND entity_id = ?1 ORDER BY id",
                )
                .map_err(map_rusqlite)?;
            let rows = stmt
                .query_map(params![device], |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                })
                .map_err(map_rusqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_rusqlite)?;
            Ok(rows)
        })
        .await
        .expect("device movements")
}

/// (kind, source, act_id, movement_id)
async fn device_events(
    ctx: &AppCtx,
    device: i64,
) -> Vec<(String, String, Option<i64>, Option<i64>)> {
    ctx.writer
        .execute(move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT kind, source, act_id, movement_id FROM history_events \
                     WHERE subject_type = 'device' AND subject_id = ?1 ORDER BY id",
                )
                .map_err(map_rusqlite)?;
            let rows = stmt
                .query_map(params![device], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
                })
                .map_err(map_rusqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_rusqlite)?;
            Ok(rows)
        })
        .await
        .expect("device events")
}

async fn count(ctx: &AppCtx, sql: &'static str) -> i64 {
    ctx.writer
        .execute(move |conn| conn.query_row(sql, [], |r| r.get(0)).map_err(map_rusqlite))
        .await
        .expect("count")
}

async fn dangling_movement_refs(ctx: &AppCtx) -> i64 {
    count(
        ctx,
        "SELECT COUNT(*) FROM history_events e WHERE e.movement_id IS NOT NULL \
         AND NOT EXISTS (SELECT 1 FROM place_movements m WHERE m.id = e.movement_id)",
    )
    .await
}

/// Журнал мест не противоречит месту: строки идут цепочкой (откуда = куда предыдущей), и
/// последняя кончается там, где устройство стоит.
async fn assert_journal_consistent(ctx: &AppCtx, device: i64) {
    let rows = device_movements(ctx, device).await;
    for w in rows.windows(2) {
        assert_eq!(
            w[1].1, w[0].2,
            "строка {:?} начинается не там, где кончилась предыдущая {:?}",
            w[1], w[0]
        );
    }
    if let Some(last) = rows.last() {
        let (place, _, _) = device_state(ctx, device).await;
        assert_eq!(
            Some(last.2),
            place,
            "журнал кончается в {}, устройство стоит в {place:?}: {rows:?}",
            last.2
        );
    }
}

async fn delete_with(
    ctx: &AppCtx,
    act: i64,
    groups: &[GroupRestoreChoiceDto],
    moved: Option<MovedAfterActChoiceDto>,
) -> ActDeleteResultDto {
    let version = act_version(ctx, act).await;
    ctx.acts
        .delete_soft_with_choices(act, version, groups, moved)
        .await
        .expect("delete")
}

struct Fixture {
    a: i64,
    c: i64,
    moved: i64,
    still: i64,
    act: i64,
    stock: i64,
    later_status: i64,
}

/// Акт «UAT3-…» переводит два устройства из «Склад А» в «Кабинет Б»; потом одно вручную
/// переводят в «Кабинет В» со сменой статуса, второе не трогают.
async fn fixture(ctx: &AppCtx, number: &str) -> Fixture {
    let a = seed_place(ctx, "Склад А").await;
    let b = seed_place(ctx, "Кабинет Б").await;
    let c = seed_place(ctx, "Кабинет В").await;
    let moved = seed_device(ctx, "Монитор #31", a).await;
    let still = seed_device(ctx, "Принтер #31", a).await;
    let (_, stock, _) = device_state(ctx, moved).await;
    let (act, _) = create(ctx, number, b, &[moved, still]).await;
    let later_status = seed_status(ctx, "Резерв (тест)").await;
    move_manually(ctx, moved, c, Some(later_status)).await;
    let rows = device_movements(ctx, moved).await;
    assert_eq!(
        rows.iter().map(|m| (m.1, m.2)).collect::<Vec<_>>(),
        vec![(a, b), (b, c)],
        "предусловие: А -> Б актом, Б -> В вручную"
    );
    Fixture {
        a,
        c,
        moved,
        still,
        act,
        stock,
        later_status,
    }
}

/// Без выбора: ответ с перечнем, НИЧЕГО не изменено (акт жив, места, статусы, журнал, события).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_delete_moved_without_choice_changes_nothing() {
    let (ctx, _d) = make_test_ctx().await;
    let f = fixture(&ctx, "UAT3-1").await;
    let moved_before = device_state(&ctx, f.moved).await;
    let still_before = device_state(&ctx, f.still).await;
    let movements_before = count(&ctx, "SELECT COUNT(*) FROM place_movements").await;
    let events_before = count(&ctx, "SELECT COUNT(*) FROM history_events").await;

    let r = delete_with(&ctx, f.act, &[], None).await;

    let w = r
        .needs_moved_after_choice
        .expect("нужен выбор по устройствам, перемещённым после акта");
    assert!(r.needs_group_restore.is_none());
    assert!(r.changed_place_ids.is_empty());
    assert_eq!(w.devices.len(), 1, "в перечне только перемещённое: {w:?}");
    assert_eq!(w.devices[0].device_id, f.moved);
    assert_eq!(w.devices[0].name, "Монитор #31");
    assert_eq!(
        w.devices[0].current_place_path.as_deref(),
        Some("Кабинет В")
    );
    assert_eq!(w.devices[0].before_place_path.as_deref(), Some("Склад А"));
    assert!(!act_deleted(&ctx, f.act).await, "акт жив");
    assert_eq!(device_state(&ctx, f.moved).await, moved_before);
    assert_eq!(device_state(&ctx, f.still).await, still_before);
    assert_eq!(
        count(&ctx, "SELECT COUNT(*) FROM place_movements").await,
        movements_before
    );
    assert_eq!(
        count(&ctx, "SELECT COUNT(*) FROM history_events").await,
        events_before
    );
}

/// «Вернуть на место до акта» + «Стереть из истории перемещения после акта»: устройство в А
/// со статусом до акта; ни строки акта, ни ручной строки, ни событий с их movement_id.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_delete_moved_return_erase() {
    let (ctx, _d) = make_test_ctx().await;
    let f = fixture(&ctx, "UAT3-2").await;
    assert!(
        device_events(&ctx, f.moved)
            .await
            .iter()
            .any(|e| e.1 == "manual" && e.3.is_some()),
        "предусловие: ручная смена статуса приклеена к ручной строке"
    );

    let r = delete_with(&ctx, f.act, &[], Some(MovedAfterActChoiceDto::ReturnErase)).await;

    assert!(r.needs_moved_after_choice.is_none());
    assert!(act_deleted(&ctx, f.act).await);
    let (place, status, _) = device_state(&ctx, f.moved).await;
    assert_eq!(
        (place, status),
        (Some(f.a), f.stock),
        "место и статус до акта"
    );
    assert_eq!(
        device_movements(&ctx, f.moved).await,
        Vec::new(),
        "история кончается состоянием до акта"
    );
    assert!(
        device_events(&ctx, f.moved)
            .await
            .iter()
            .all(|e| e.3.is_none()),
        "событий со ссылкой на стёртые строки нет"
    );
    let (still_place, _, _) = device_state(&ctx, f.still).await;
    assert_eq!(
        still_place,
        Some(f.a),
        "неперемещённое откатилось как раньше"
    );
    assert_eq!(device_movements(&ctx, f.still).await, Vec::new());
    assert_eq!(dangling_movement_refs(&ctx).await, 0);
}

/// «Вернуть на место до акта» без стирания: строка акта забыта (ручная стала А -> В), одна
/// строка «при удалении акта» В -> А без act_id, с номером акта в примечании, и событие
/// статуса после неё с её movement_id (D-25).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_delete_moved_return_mark() {
    let (ctx, _d) = make_test_ctx().await;
    let f = fixture(&ctx, "UAT3-3").await;

    let r = delete_with(&ctx, f.act, &[], Some(MovedAfterActChoiceDto::ReturnMark)).await;

    assert!(r.needs_moved_after_choice.is_none());
    assert!(act_deleted(&ctx, f.act).await);
    let (place, status, _) = device_state(&ctx, f.moved).await;
    assert_eq!(
        (place, status),
        (Some(f.a), f.stock),
        "место и статус до акта"
    );
    let rows = device_movements(&ctx, f.moved).await;
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(
        (rows[0].1, rows[0].2, rows[0].3.as_str()),
        (f.a, f.c, "manual"),
        "ручная строка начинается с места до акта"
    );
    let (ret_id, from, to, source, act_id, note) = rows[1].clone();
    assert_eq!(
        (from, to, source.as_str(), act_id, note.as_deref()),
        (f.c, f.a, "act_delete", None, Some("UAT3-3"))
    );
    let events = device_events(&ctx, f.moved).await;
    let last = events.last().expect("событие статуса возврата");
    assert_eq!(
        (last.0.as_str(), last.1.as_str(), last.2, last.3),
        ("status_changed", "act_delete", None, Some(ret_id)),
        "{events:?}"
    );
    assert!(events.iter().all(|e| e.2 != Some(f.act)));
    assert_journal_consistent(&ctx, f.moved).await;
    let timeline = ctx
        .history
        .get_timeline(&admin(), "device", f.moved)
        .await
        .expect("лента");
    assert_eq!(
        timeline.first().map(|e| e.reason_source.as_str()),
        Some("act_delete"),
        "последняя запись ленты — возврат при удалении акта: {timeline:?}"
    );
    let (still_place, _, _) = device_state(&ctx, f.still).await;
    assert_eq!(still_place, Some(f.a));
    assert_eq!(device_movements(&ctx, f.still).await, Vec::new());
    assert_eq!(dangling_movement_refs(&ctx).await, 0);
}

/// «Оставить на текущем месте»: место, статус и состав не восстанавливаются, строка акта
/// забыта (лента: А -> В), новых строк нет. Неперемещённое устройство откатилось.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_delete_moved_keep() {
    let (ctx, _d) = make_test_ctx().await;
    let f = fixture(&ctx, "UAT3-4").await;

    let r = delete_with(&ctx, f.act, &[], Some(MovedAfterActChoiceDto::Keep)).await;

    assert!(r.needs_moved_after_choice.is_none());
    assert!(act_deleted(&ctx, f.act).await);
    let (place, status, _) = device_state(&ctx, f.moved).await;
    assert_eq!(
        (place, status),
        (Some(f.c), f.later_status),
        "место и статус не восстановлены"
    );
    let rows = device_movements(&ctx, f.moved).await;
    assert_eq!(
        rows.iter()
            .map(|m| (m.1, m.2, m.3.clone(), m.4))
            .collect::<Vec<_>>(),
        vec![(f.a, f.c, "manual".to_string(), None)],
        "перемещение показано с места до акта сразу на текущее"
    );
    assert!(device_events(&ctx, f.moved)
        .await
        .iter()
        .all(|e| e.2 != Some(f.act) && e.1 != "act_delete"));
    assert_journal_consistent(&ctx, f.moved).await;
    let (still_place, _, _) = device_state(&ctx, f.still).await;
    assert_eq!(still_place, Some(f.a), "неперемещённое откатилось");
    assert_eq!(dangling_movement_refs(&ctx).await, 0);
}

/// Устройства, которые после акта не перемещали, — как раньше: вопроса нет, откат молча.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_delete_moved_unmoved_act_has_no_question() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Кабинет Б").await;
    let d = seed_device(&ctx, "Ноутбук #32", a).await;
    let (act, _) = create(&ctx, "UAT3-5", b, &[d]).await;
    let r = delete_with(&ctx, act, &[], None).await;
    assert!(r.needs_moved_after_choice.is_none());
    assert!(act_deleted(&ctx, act).await);
    assert_eq!(device_state(&ctx, d).await.0, Some(a));
    assert_eq!(device_movements(&ctx, d).await, Vec::new());
}

/// Возврат по акту, потом ручное перемещение: каскад удаления (акт и его возврат) спрашивает;
/// «Оставить» оставляет устройство на месте, журнал сведён с местом.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_delete_moved_cascade_keep_and_mark() {
    for choice in [
        MovedAfterActChoiceDto::Keep,
        MovedAfterActChoiceDto::ReturnMark,
        MovedAfterActChoiceDto::ReturnErase,
    ] {
        let (ctx, _d) = make_test_ctx().await;
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Кабинет Б").await;
        let w = seed_place(&ctx, "Склад Возвратов").await;
        let c = seed_place(&ctx, "Кабинет В").await;
        let d = seed_device(&ctx, "Монитор #33", a).await;
        let (act, _) = create(&ctx, "UAT3-6", b, &[d]).await;
        let item = ctx.acts.get(act).await.expect("акт").items[0].id;
        ctx.acts
            .do_return(
                &admin(),
                act,
                ActReturnDto {
                    items: vec![ActReturnItemDto {
                        act_item_id: item,
                        device_id: d,
                        device_ids: vec![d],
                        quantity: 1,
                        condition_override: Some("исправно".into()),
                        place_id_override: Some(w),
                    }],
                    apply_to_all: false,
                    bulk_condition: None,
                    bulk_place_id: None,
                    giver_name: Some("Петров П.П.".into()),
                    receiver_name: Some("Иванов И.И.".into()),
                    handover_date_utc: None,
                },
            )
            .await
            .expect("возврат");
        assert_eq!(
            device_state(&ctx, d).await.0,
            Some(w),
            "предусловие: возврат на склад"
        );
        move_manually(&ctx, d, c, None).await;

        let asked = delete_with(&ctx, act, &[], None).await;
        let wn = asked.needs_moved_after_choice.expect("каскад спрашивает");
        assert_eq!(wn.devices.len(), 1);
        assert_eq!(
            wn.devices[0].current_place_path.as_deref(),
            Some("Кабинет В")
        );
        assert_eq!(wn.devices[0].before_place_path.as_deref(), Some("Склад А"));
        assert!(!act_deleted(&ctx, act).await);

        let done = delete_with(&ctx, act, &[], Some(choice)).await;
        assert!(done.needs_moved_after_choice.is_none(), "{choice:?}");
        assert!(act_deleted(&ctx, act).await);
        let expected = match choice {
            MovedAfterActChoiceDto::Keep => c,
            _ => a,
        };
        assert_eq!(device_state(&ctx, d).await.0, Some(expected), "{choice:?}");
        assert_journal_consistent(&ctx, d).await;
        assert!(
            device_movements(&ctx, d)
                .await
                .iter()
                .all(|m| m.4.is_none()),
            "строк удалённых актов нет ({choice:?})"
        );
        assert_eq!(dangling_movement_refs(&ctx).await, 0);
        ctx.shutdown.cancel();
    }
}

// ---------------------------------------------------------------------------
// Оба конфликта сразу: группа перенесена после акта (D-42) и устройство перемещали.
// ---------------------------------------------------------------------------

/// Группа «АРМ #34» из двух устройств на месте `place`; (group, [devices]).
async fn seed_arm(ctx: &AppCtx, place: i64) -> (i64, Vec<i64>) {
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
                        "INSERT INTO group_types (code, name, behavior, created_at_utc, \
                         updated_at_utc) VALUES ('uat3_t', 'Тип теста', 'container', 1700000000, 1700000000)",
                        [],
                    )
                    .map_err(map_rusqlite)?;
                    conn.last_insert_rowid()
                }
            };
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, 'АРМ #34', 1, ?2, NULL, 1700000000, 1700000000, 1)",
                params![type_id, place],
            )
            .map_err(map_rusqlite)?;
            let group = conn.last_insert_rowid();
            let mut devices = Vec::new();
            for name in ["Системный блок #34", "Монитор #34"] {
                conn.execute(
                    "INSERT INTO devices (type_id, name, place_id, status_id, created_at_utc, \
                     updated_at_utc, version) VALUES (1, ?1, ?2, 1, 1700000000, 1700000000, 1)",
                    params![name, place],
                )
                .map_err(map_rusqlite)?;
                let d = conn.last_insert_rowid();
                conn.execute(
                    "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
                     VALUES (?1, ?2, 1700000000)",
                    params![d, group],
                )
                .map_err(map_rusqlite)?;
                devices.push(d);
            }
            Ok((group, devices))
        })
        .await
        .expect("seed arm")
}

async fn group_version(ctx: &AppCtx, group: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT version FROM groups WHERE id = ?1",
                params![group],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("group version")
}

async fn member_rows(ctx: &AppCtx, device: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM group_devices WHERE device_id = ?1",
                params![device],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("member rows")
}

/// Акт выводит члена группы и переводит отдельное устройство; затем группу переносят, а
/// отдельное устройство перемещают вручную. Первый ответ несёт оба перечня; ответ с одним
/// выбором по перемещённым всё ещё просит выбор по группе; оба выбора одним вызовом удаляют.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_delete_moved_with_group_conflict() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Кабинет Б").await;
    let c = seed_place(&ctx, "Кабинет В").await;
    let g_new = seed_place(&ctx, "Склад Г").await;
    let (group, members) = seed_arm(&ctx, a).await;
    let released = members[1];
    let solo = seed_device(&ctx, "Ноутбук #34", a).await;
    let (act, _) = create(&ctx, "UAT3-7", b, &[released, solo]).await;
    assert_eq!(member_rows(&ctx, released).await, 0, "акт вывел из группы");
    let gv = group_version(&ctx, group).await;
    ctx.groups
        .move_group(&admin(), group, gv, g_new)
        .await
        .expect("перенос группы после акта");
    move_manually(&ctx, solo, c, None).await;

    let first = delete_with(&ctx, act, &[], None).await;
    assert!(first.needs_group_restore.is_some(), "конфликт группы");
    let moved = first
        .needs_moved_after_choice
        .expect("конфликт перемещения");
    assert_eq!(
        moved
            .devices
            .iter()
            .map(|d| d.device_id)
            .collect::<Vec<_>>(),
        vec![solo]
    );
    assert!(!act_deleted(&ctx, act).await);

    let second = delete_with(&ctx, act, &[], Some(MovedAfterActChoiceDto::Keep)).await;
    assert!(second.needs_moved_after_choice.is_none());
    assert!(
        second.needs_group_restore.is_some(),
        "выбор по группе ещё нужен"
    );
    assert!(
        !act_deleted(&ctx, act).await,
        "без выбора по группе ничего не удалено"
    );

    let done = delete_with(
        &ctx,
        act,
        &[GroupRestoreChoiceDto {
            group_id: group,
            restore: true,
        }],
        Some(MovedAfterActChoiceDto::Keep),
    )
    .await;
    assert!(done.needs_group_restore.is_none() && done.needs_moved_after_choice.is_none());
    assert!(act_deleted(&ctx, act).await);
    assert_eq!(member_rows(&ctx, released).await, 1, "возвращён в группу");
    assert_eq!(
        device_state(&ctx, solo).await.0,
        Some(c),
        "оставлен на месте"
    );
    assert_journal_consistent(&ctx, solo).await;
    assert_eq!(dangling_movement_refs(&ctx).await, 0);
}
