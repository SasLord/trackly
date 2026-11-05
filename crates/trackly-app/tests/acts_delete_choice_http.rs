//! UD-1 (41.2-21): выбор по группам при удалении акта доходит до сервиса по HTTP.
//!
//! Фикстура вымышленная. Тело ответа проверяется как JSON: отсутствующий /api/
//! маршрут spa_fallback маскирует под index.html с 200.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use rusqlite::params;
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use trackly_app::context::AppCtx;
use trackly_app::dto::act::{ActCreateDto, ActDeleteResultDto, ActItemNewDto, ActSaveOutcome};
use trackly_app::dto::auth::UserNew;
use trackly_app::dto::number_template::NumberFieldInput;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_core::auth::Identity;
use trackly_infra::error_conversions::map_rusqlite;

async fn session_cookie(store: &RusqliteSessionStore, user_id: i64, role: &str) -> String {
    let id = Id::default();
    let si = SessionIdentity {
        user_id: Some(user_id),
        role: role.to_string(),
    };
    let mut record = Record {
        id,
        data: Default::default(),
        expiry_date: OffsetDateTime::now_utc() + Duration::days(1),
    };
    record
        .data
        .insert("identity".to_string(), serde_json::to_value(&si).unwrap());
    store.create(&mut record).await.unwrap();
    format!("id={id}")
}

async fn make_user(ctx: &AppCtx, login: &str, role: &str) -> i64 {
    ctx.auth
        .create_user(
            UserNew {
                login: login.to_string(),
                full_name: "Иванов И.И.".to_string(),
                password: "password123".to_string(),
                role: role.to_string(),
                email: None,
            },
            &Identity::trusted_admin(),
        )
        .await
        .expect("create user")
        .id
}

async fn seed_place(ctx: &AppCtx, name: &str) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, parent_id, is_storage, created_at_utc, \
                 updated_at_utc, version) VALUES ('room', ?1, NULL, 0, 1700000000, 1700000000, 1)",
                params![name],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

/// Группа «АРМ #3» из двух устройств на месте `place`; возвращает (group, [devices]).
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
                         updated_at_utc) VALUES ('http_t', 'Тип теста', 'container', 1700000000, 1700000000)",
                        [],
                    )
                    .map_err(map_rusqlite)?;
                    conn.last_insert_rowid()
                }
            };
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, 'АРМ #3', 1, ?2, NULL, 1700000000, 1700000000, 1)",
                params![type_id, place],
            )
            .map_err(map_rusqlite)?;
            let group = conn.last_insert_rowid();
            let mut devices = Vec::new();
            for name in ["Системный блок #4", "Монитор #2"] {
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

async fn act_deleted(ctx: &AppCtx, act: i64) -> bool {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT deleted_at_utc IS NOT NULL FROM acts WHERE id = ?1",
                params![act],
                |r| r.get::<_, bool>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .unwrap()
}

async fn member_rows(ctx: &AppCtx, device: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM group_devices WHERE device_id = ?1",
                params![device],
                |r| r.get::<_, i64>(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .unwrap()
}

/// Акт, вывевший «Монитор #2» из группы, затем группа перенесена: (act_id, version, device, group).
async fn conflict_fixture(ctx: &AppCtx, number: &str) -> (i64, i64, i64, i64) {
    let a = seed_place(ctx, "Склад А").await;
    let b = seed_place(ctx, "Склад Б").await;
    let c = seed_place(ctx, "Склад В").await;
    let (group, devices) = seed_arm(ctx, a).await;
    let released = devices[1];
    let payload = ActCreateDto {
        number_input: NumberFieldInput {
            value: number.into(),
            template_id: None,
            confirm_mismatch: false,
            confirm_script_mix: false,
        },
        giver_name: "Иванов И.И.".into(),
        receiver_name: "Петров П.П.".into(),
        place_id: Some(b),
        notes: None,
        deadline_utc: None,
        handover_date_utc: None,
        items: vec![ActItemNewDto {
            device_id: released,
            device_ids: vec![released],
            quantity: 1,
        }],
        confirm_group_release: true,
    };
    let act = match ctx
        .acts
        .create(&Identity::trusted_admin(), payload)
        .await
        .expect("create act")
    {
        ActSaveOutcome::Created(a) => a,
        other => panic!("ожидали Created, получили {other:?}"),
    };
    assert_eq!(member_rows(ctx, released).await, 0);
    let gv: i64 = ctx
        .writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT version FROM groups WHERE id = ?1",
                params![group],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .unwrap();
    ctx.groups
        .move_group(&Identity::trusted_admin(), group, gv, c)
        .await
        .expect("move group after the act");
    (act.id, act.version, released, group)
}

async fn post_delete(
    app: &Router,
    cookie: &str,
    body: serde_json::Value,
) -> (StatusCode, Option<String>, Vec<u8>) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/acts_delete")
                .header("content-type", "application/json")
                .header("cookie", cookie)
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let ct = res
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap().to_string());
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap()
        .to_vec();
    (status, ct, bytes)
}

fn parse_json_result(status: StatusCode, ct: Option<String>, bytes: &[u8]) -> ActDeleteResultDto {
    assert_eq!(status, StatusCode::OK);
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(32)]).to_lowercase();
    assert!(
        !head.starts_with("<!doctype") && !head.starts_with("<html"),
        "получили index.html от spa_fallback"
    );
    assert!(
        ct.as_deref().unwrap_or("").starts_with("application/json"),
        "content-type: {ct:?}"
    );
    serde_json::from_slice(bytes).expect("тело — ActDeleteResultDto")
}

async fn build_ctx() -> (AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().unwrap();
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).unwrap();
    let config = trackly_infra::AppConfig::default();
    let guard = trackly_app::logging::init(&paths, &config).unwrap_or_else(|_| {
        let (_nb, g) = tracing_appender::non_blocking(std::io::sink());
        g
    });
    (AppCtx::build(paths, config, guard).await.unwrap(), dir)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_delete_act_asks_then_applies_group_choice() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (ctx, _dir) = build_ctx().await;
        let admin_id = make_user(&ctx, "admin_del", "admin").await;
        let store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let cookie = session_cookie(&store, admin_id, "admin").await;
        let app = build_router(&ctx, store);
        let (act, version, device, group) = conflict_fixture(&ctx, "HC-1").await;

        // Шаг 1: без выбора — просьба о выборе, акт жив.
        let (s, ct, b) = post_delete(
            &app,
            &cookie,
            serde_json::json!({"id": act, "version": version}),
        )
        .await;
        let r = parse_json_result(s, ct, &b);
        let w = r.needs_group_restore.expect("нужен выбор");
        assert_eq!(w.groups.len(), 1);
        assert_eq!(w.groups[0].devices.len(), 1);
        assert!(!act_deleted(&ctx, act).await, "акт жив");
        assert_eq!(member_rows(&ctx, device).await, 0);

        // Шаг 2: с выбором restore:true — успех, устройство в группе.
        let (s, ct, b) = post_delete(
            &app,
            &cookie,
            serde_json::json!({
                "id": act, "version": version,
                "groupChoices": [{"group_id": group, "restore": true}]
            }),
        )
        .await;
        let r = parse_json_result(s, ct, &b);
        assert!(
            r.needs_group_restore.is_none(),
            "выбор должен быть применён"
        );
        assert!(act_deleted(&ctx, act).await, "акт удалён");
        assert_eq!(member_rows(&ctx, device).await, 1, "устройство в группе");

        ctx.shutdown.cancel();
    })
    .await
    .expect("budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_delete_act_without_conflict_unchanged() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (ctx, _dir) = build_ctx().await;
        let admin_id = make_user(&ctx, "admin_del2", "admin").await;
        let store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let cookie = session_cookie(&store, admin_id, "admin").await;
        let app = build_router(&ctx, store);
        let a = seed_place(&ctx, "Склад А").await;
        let b = seed_place(&ctx, "Склад Б").await;
        let (_g, devices) = seed_arm(&ctx, a).await;
        let payload = ActCreateDto {
            number_input: NumberFieldInput {
                value: "HC-2".into(),
                template_id: None,
                confirm_mismatch: false,
                confirm_script_mix: false,
            },
            giver_name: "Иванов И.И.".into(),
            receiver_name: "Петров П.П.".into(),
            place_id: Some(b),
            notes: None,
            deadline_utc: None,
            handover_date_utc: None,
            items: vec![ActItemNewDto {
                device_id: devices[1],
                device_ids: vec![devices[1]],
                quantity: 1,
            }],
            confirm_group_release: true,
        };
        let act = match ctx
            .acts
            .create(&Identity::trusted_admin(), payload)
            .await
            .unwrap()
        {
            ActSaveOutcome::Created(a) => a,
            other => panic!("{other:?}"),
        };
        let (s, ct, bytes) = post_delete(
            &app,
            &cookie,
            serde_json::json!({"id": act.id, "version": act.version}),
        )
        .await;
        let r = parse_json_result(s, ct, &bytes);
        assert!(r.needs_group_restore.is_none());
        assert!(act_deleted(&ctx, act.id).await, "акт удалён");
        ctx.shutdown.cancel();
    })
    .await
    .expect("budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_delete_act_denied_for_employee() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (ctx, _dir) = build_ctx().await;
        let emp_id = make_user(&ctx, "emp_del", "employee").await;
        let store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let cookie = session_cookie(&store, emp_id, "employee").await;
        let app = build_router(&ctx, store);
        let (act, version, _d, _g) = conflict_fixture(&ctx, "HC-3").await;
        let (s, _ct, _b) = post_delete(
            &app,
            &cookie,
            serde_json::json!({"id": act, "version": version}),
        )
        .await;
        assert!(
            s == StatusCode::FORBIDDEN || s == StatusCode::UNAUTHORIZED,
            "статус {s}"
        );
        assert!(!act_deleted(&ctx, act).await, "акт жив");
        ctx.shutdown.cancel();
    })
    .await
    .expect("budget");
}
