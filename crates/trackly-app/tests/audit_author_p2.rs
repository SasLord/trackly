//! Долг v1.4, пакет П2 (WARNING-8, 41.1-08): создание, массовое создание и мягкое
//! удаление устройства, удаление акта и SNMP-приёмка пишут автора
//! (`audit_log.user_id`) из `Identity` вызывающего; приёмка берёт статус по коду.
//! Реальный `AppCtx` на временном каталоге; имена вымышленные.

use std::time::Duration;

use trackly_app::context::AppCtx;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto};
use trackly_app::dto::device::{DeviceNew, DeviceSaveOutcome};
use trackly_app::dto::number_template::{NumberFieldInput, TemplateContextDto};
use trackly_app::tauri_cmds::acts::build_acts_delete;
use trackly_app::tauri_cmds::devices::{
    build_devices_bulk_create, build_devices_create, build_devices_create_single_with_number_check,
    build_devices_delete,
};
use trackly_app::tauri_cmds::printers::build_printers_admit;
use trackly_core::auth::{Identity, Role};
use trackly_infra::error_conversions::map_rusqlite;

async fn make_ctx() -> (AppCtx, tempfile::TempDir) {
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

/// Реальная строка `users` (FK-цель `audit_log.user_id`) и её Identity.
async fn manager(ctx: &AppCtx) -> (Identity, i64) {
    let uid = ctx
        .writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, password_hash, role, ad_user, is_active, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES ('sidorov.ss', 'Сидоров С.С.', NULL, 'manager', 0, 1, 1700000000, 1700000000, 1)",
                [],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed user");
    (
        Identity {
            user_id: Some(uid),
            role: Role::Manager,
        },
        uid,
    )
}

async fn scalar(ctx: &AppCtx, sql: &'static str, p: Vec<rusqlite::types::Value>) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| {
                r.get::<_, Option<i64>>(0)
            })
            .map_err(map_rusqlite)
        })
        .await
        .expect("scalar")
}

fn int(v: i64) -> rusqlite::types::Value {
    rusqlite::types::Value::Integer(v)
}

fn txt(v: &str) -> rusqlite::types::Value {
    rusqlite::types::Value::Text(v.to_string())
}

/// `user_id` единственной строки журнала `(entity, id, action)`.
async fn audit_user(ctx: &AppCtx, entity: &'static str, id: i64, action: &'static str) -> i64 {
    let n = scalar(
        ctx,
        "SELECT COUNT(*) FROM audit_log WHERE entity_type = ?1 AND entity_id = ?2 AND action = ?3",
        vec![txt(entity), int(id), txt(action)],
    )
    .await
    .unwrap();
    assert_eq!(
        n, 1,
        "{entity}#{id} {action}: ожидалась одна строка журнала"
    );
    scalar(
        ctx,
        "SELECT user_id FROM audit_log WHERE entity_type = ?1 AND entity_id = ?2 AND action = ?3",
        vec![txt(entity), int(id), txt(action)],
    )
    .await
    .expect("user_id в журнале не NULL")
}

async fn default_status(ctx: &AppCtx) -> i64 {
    scalar(
        ctx,
        "SELECT id FROM device_statuses WHERE code = 'на_складе'",
        vec![],
    )
    .await
    .expect("status")
}

fn new_device(name: &str, status_id: i64) -> DeviceNew {
    DeviceNew {
        type_id: 1,
        name: name.to_string(),
        inventory_no: None,
        serial_no: None,
        model: None,
        specs: None,
        kit: None,
        state: None,
        place_id: None,
        status_id,
        origin_group_id: None,
    }
}

fn created(o: DeviceSaveOutcome) -> trackly_app::dto::device::DeviceDto {
    match o {
        DeviceSaveOutcome::Created(d) => *d,
        other => panic!("ожидалось Created, получено {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_delete_bulk_write_caller_to_audit_log() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (ctx, _dir) = make_ctx().await;
        let (caller, uid) = manager(&ctx).await;
        let st = default_status(&ctx).await;

        // create
        let d = created(
            build_devices_create(&ctx, &caller, new_device("Монитор А", st))
                .await
                .expect("create"),
        );
        assert_eq!(audit_user(&ctx, "device", d.id, "create").await, uid);

        // create_single_with_number_check
        let d2 = created(
            build_devices_create_single_with_number_check(
                &ctx,
                &caller,
                new_device("Монитор Б", st),
                NumberFieldInput {
                    value: String::new(),
                    template_id: None,
                    confirm_mismatch: false,
                    confirm_script_mix: false,
                },
                TemplateContextDto::DeviceCreate,
                None,
            )
            .await
            .expect("create single"),
        );
        assert_eq!(audit_user(&ctx, "device", d2.id, "create").await, uid);

        // bulk_create
        let bulk = build_devices_bulk_create(&ctx, &caller, new_device("Мышь", st), 2)
            .await
            .expect("bulk");
        assert_eq!(bulk.len(), 2);
        for b in &bulk {
            assert_eq!(audit_user(&ctx, "device", b.id, "create").await, uid);
        }

        // delete
        build_devices_delete(&ctx, &caller, d.id, d.version)
            .await
            .expect("delete");
        assert_eq!(audit_user(&ctx, "device", d.id, "delete").await, uid);
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn act_delete_writes_caller_to_audit_log() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (ctx, _dir) = make_ctx().await;
        let (caller, uid) = manager(&ctx).await;
        let st = default_status(&ctx).await;
        let d = created(
            build_devices_create(&ctx, &caller, new_device("Ноутбук А", st))
                .await
                .expect("create"),
        );
        let act = ctx
            .acts
            .create(
                &caller,
                ActCreateDto {
                    number_input: NumberFieldInput {
                        value: "1".into(),
                        template_id: None,
                        confirm_mismatch: false,
                        confirm_script_mix: false,
                    },
                    giver_name: "Иванов И.И.".into(),
                    receiver_name: "Петров П.П.".into(),
                    place_id: None,
                    notes: None,
                    deadline_utc: None,
                    handover_date_utc: None,
                    items: vec![ActItemNewDto {
                        device_id: d.id,
                        device_ids: Vec::new(),
                        quantity: 1,
                    }],
                    confirm_group_release: false,
                },
            )
            .await
            .expect("act create")
            .expect_created("act create");

        build_acts_delete(&ctx, &caller, act.id, act.version, Vec::new(), None)
            .await
            .expect("act delete");

        let rows = scalar(
            &ctx,
            "SELECT COUNT(*) FROM audit_log WHERE entity_type = 'act' AND entity_id = ?1 \
             AND action = 'delete'",
            vec![int(act.id)],
        )
        .await
        .unwrap();
        assert!(rows >= 1, "удаление акта должно оставить строку журнала");
        let null_authors = scalar(
            &ctx,
            "SELECT COUNT(*) FROM audit_log WHERE entity_type = 'act' AND entity_id = ?1 \
             AND action = 'delete' AND user_id IS NULL",
            vec![int(act.id)],
        )
        .await
        .unwrap();
        assert_eq!(null_authors, 0, "у удаления акта должен быть автор");
        let authored = scalar(
            &ctx,
            "SELECT COUNT(*) FROM audit_log WHERE entity_type = 'act' AND entity_id = ?1 \
             AND action = 'delete' AND user_id = ?2",
            vec![int(act.id), int(uid)],
        )
        .await
        .unwrap();
        assert_eq!(authored, rows);
    })
    .await
    .expect("timeout");
}

/// 41.1-08: приёмка берёт статус по коду «на_складе», а не литералом 1, и пишет автора.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn snmp_admit_uses_status_by_code_and_author() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (ctx, _dir) = make_ctx().await;
        let (_caller, uid) = manager(&ctx).await;
        // Меняем нумерацию справочника: статус «на_складе» получает id, не равный 1.
        // PRAGMA foreign_keys отключаем на время перестановки id.
        ctx.writer
            .execute(move |conn| {
                conn.execute_batch(
                    "PRAGMA foreign_keys = OFF; \
                     UPDATE device_statuses SET id = id + 1000; \
                     PRAGMA foreign_keys = ON;",
                )
                .map_err(map_rusqlite)
            })
            .await
            .expect("renumber statuses");
        let expected = scalar(
            &ctx,
            "SELECT id FROM device_statuses WHERE code = 'на_складе'",
            vec![],
        )
        .await
        .unwrap();
        assert!(expected > 1000);

        let admitted = build_printers_admit(
            &ctx,
            &Identity {
                user_id: Some(uid),
                role: Role::Admin,
            },
            vec!["192.168.1.100".to_string()],
            "comm-alpha".to_string(),
        )
        .await
        .expect("admit");
        assert_eq!(admitted.len(), 1);

        let status = scalar(
            &ctx,
            "SELECT status_id FROM devices WHERE type_id = 2",
            vec![],
        )
        .await
        .unwrap();
        assert_eq!(status, expected, "статус приёмки — по коду, не литерал 1");
        let author = scalar(
            &ctx,
            "SELECT user_id FROM audit_log WHERE entity_type = 'device' AND action = 'create'",
            vec![],
        )
        .await
        .unwrap();
        assert_eq!(author, uid);
    })
    .await
    .expect("timeout");
}
