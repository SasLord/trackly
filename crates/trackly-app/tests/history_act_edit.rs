//! Фаза 41.10, ревью CR-01 (D-24 (a)): правка ЖИВОГО акта пишет обычные события
//! состава и вложенности. Когда правка снимает позицию, а устройство или группа
//! возвращаются в состав, последняя запись ленты должна совпадать с текущим составом.
//! «Выведено … актом №N» не должно оставаться последней записью, если устройство или
//! группа уже снова в составе. Префикс тестов: `history_act_edit_`. Фикстуры вымышленные.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::act::{
    ActCreateDto, ActItemNewDto, ActNumberEditInput, ActSaveOutcome, ActUpdateDto, ActUpdateItemDto,
};
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

/// Последнее событие состава/вложенности субъекта: (kind, act_id, группа «из», группа «в»).
type LastEv = (String, Option<i64>, Option<i64>, Option<i64>);

async fn last_membership_event(ctx: &AppCtx, subject_type: &'static str, id: i64) -> LastEv {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT kind, act_id, from_group_id, to_group_id FROM history_events \
                 WHERE subject_type = ?1 AND subject_id = ?2 AND kind <> 'status_changed' \
                 ORDER BY created_at_utc DESC, id DESC LIMIT 1",
                params![subject_type, id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("last membership event")
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

/// CR-01: акт вывел члена группы (`group_left` с act_id); правка сняла позицию, и устройство
/// вернулось в группу. Последнее событие состава — `group_joined` этого же акта.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_edit_restored_member_writes_group_joined() {
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
    assert_eq!(
        last_membership_event(&ctx, "device", d1).await.0,
        "group_left"
    );

    edit_keep(&ctx, act, &[free]).await;

    assert_eq!(group_of(&ctx, d1).await, Some(g), "правка вернула в группу");
    let (kind, act_id, _, to_group) = last_membership_event(&ctx, "device", d1).await;
    assert_eq!(
        (kind.as_str(), act_id, to_group),
        ("group_joined", Some(act), Some(g)),
        "последняя запись состава обязана совпадать с составом"
    );
}

/// CR-01: правка сняла позицию, восстановленное место разошлось с местом группы, в которую
/// устройство вошло уже после акта. Устройство выходит из группы, и это пишется событием.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn history_act_edit_locked_release_writes_group_left() {
    let (ctx, _d) = make_test_ctx().await;
    let a = seed_place(&ctx, "Склад А").await;
    let b = seed_place(&ctx, "Кабинет Б").await;
    let d = seed_device(&ctx, "Монитор #22", a, None).await;
    let free = seed_device(&ctx, "Принтер #22", a, None).await;
    let act = create(&ctx, create_dto("HAE-2", &[d, free], b)).await;
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
    let (kind, act_id, from_group, _) = last_membership_event(&ctx, "device", d).await;
    assert_eq!(
        (kind.as_str(), act_id, from_group),
        ("group_left", Some(act), Some(h)),
        "вывод при правке акта не может быть молчаливым"
    );
}
