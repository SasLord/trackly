//! UD-2 (41.2-26): предварительный вопрос и выбор разбиения возврата по HTTP.
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
use trackly_app::dto::act::{
    ActCreateDto, ActItemNewDto, ActReturnDto, ActReturnItemDto, ActSaveOutcome,
    ActUpdateReturnDto, ReturnGroupSplitWarningDto,
};
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

/// Контейнер `name` из трёх устройств на `place`.
async fn seed_arm(ctx: &AppCtx, place: i64, name: &str, seq: i64) -> (i64, Vec<i64>) {
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
                 VALUES (?1, ?2, ?3, ?4, NULL, 1700000000, 1700000000, 1)",
                params![type_id, name, seq, place],
            )
            .map_err(map_rusqlite)?;
            let group = conn.last_insert_rowid();
            let mut devices = Vec::new();
            for n in ["Системный блок #4", "Монитор #2", "Клавиатура #5"] {
                conn.execute(
                    "INSERT INTO devices (type_id, name, place_id, status_id, created_at_utc, \
                     updated_at_utc, version) VALUES (1, ?1, ?2, 1, 1700000000, 1700000000, 1)",
                    params![n, place],
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

async fn group_place(ctx: &AppCtx, group: i64) -> Option<i64> {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(
                "SELECT place_id FROM groups WHERE id = ?1",
                params![group],
                |r| r.get(0),
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
                |r| r.get(0),
            )
            .map_err(map_rusqlite)
        })
        .await
        .unwrap()
}

async fn act_count(ctx: &AppCtx) -> i64 {
    ctx.writer
        .execute(|conn| {
            conn.query_row("SELECT COUNT(*) FROM acts", [], |r| r.get(0))
                .map_err(map_rusqlite)
        })
        .await
        .unwrap()
}

struct Fixture {
    act: i64,
    group: i64,
    devices: Vec<i64>,
    /// act_item_id по порядку devices.
    items: Vec<i64>,
    place_c: i64,
}

/// Группа выдана целиком на «Склад Б»; «Склад В» — место возврата.
async fn issued_arm(ctx: &AppCtx, tag: &str, seq: i64) -> Fixture {
    let a = seed_place(ctx, &format!("Склад А {tag}")).await;
    let b = seed_place(ctx, &format!("Склад Б {tag}")).await;
    let c = seed_place(ctx, &format!("Склад В {tag}")).await;
    let (group, devices) = seed_arm(ctx, a, &format!("АРМ #{seq}"), seq).await;
    let payload = ActCreateDto {
        number_input: NumberFieldInput {
            value: format!("RS-{tag}"),
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
        items: devices
            .iter()
            .map(|&d| ActItemNewDto {
                device_id: d,
                device_ids: vec![d],
                quantity: 1,
            })
            .collect(),
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
    let full = ctx.acts.get(act.id).await.expect("get act");
    let items = devices
        .iter()
        .map(|d| full.items.iter().find(|i| i.device_id == *d).unwrap().id)
        .collect();
    Fixture {
        act: act.id,
        group,
        devices,
        items,
        place_c: c,
    }
}

fn ret_item(act_item_id: i64, device: i64, place_override: Option<i64>) -> ActReturnItemDto {
    ActReturnItemDto {
        act_item_id,
        device_id: device,
        device_ids: vec![device],
        quantity: 1,
        condition_override: Some("Исправно".into()),
        place_id_override: place_override,
    }
}

fn ret_dto(f: &Fixture, idx: &[usize]) -> ActReturnDto {
    ActReturnDto {
        bulk_condition: None,
        bulk_place_id: Some(f.place_c),
        apply_to_all: true,
        items: idx
            .iter()
            .map(|&i| ret_item(f.items[i], f.devices[i], None))
            .collect(),
        giver_name: None,
        receiver_name: None,
        handover_date_utc: None,
    }
}

async fn post(
    app: &Router,
    cookie: &str,
    route: &str,
    body: serde_json::Value,
) -> (StatusCode, Option<String>, Vec<u8>) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/{route}"))
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

fn assert_json(ct: &Option<String>, bytes: &[u8]) {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(32)]).to_lowercase();
    assert!(
        !head.starts_with("<!doctype") && !head.starts_with("<html"),
        "получили index.html от spa_fallback"
    );
    assert!(
        ct.as_deref().unwrap_or("").starts_with("application/json"),
        "content-type: {ct:?}"
    );
}

fn parse_preview(r: (StatusCode, Option<String>, Vec<u8>)) -> ReturnGroupSplitWarningDto {
    assert_eq!(r.0, StatusCode::OK, "{}", String::from_utf8_lossy(&r.2));
    assert_json(&r.1, &r.2);
    serde_json::from_slice(&r.2).expect("тело — ReturnGroupSplitWarningDto")
}

async fn setup() -> (AppCtx, tempfile::TempDir, Router, String, String) {
    let (ctx, dir) = build_ctx().await;
    let admin_id = make_user(&ctx, "admin_rs", "admin").await;
    let emp_id = make_user(&ctx, "emp_rs", "employee").await;
    let store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
    let admin_cookie = session_cookie(&store, admin_id, "admin").await;
    let emp_cookie = session_cookie(&store, emp_id, "employee").await;
    let app = build_router(&ctx, store);
    (ctx, dir, app, admin_cookie, emp_cookie)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_return_preview_partial_and_full() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (ctx, _dir, app, cookie, _emp) = setup().await;
        let f = issued_arm(&ctx, "p", 3).await;

        let partial = parse_preview(
            post(
                &app,
                &cookie,
                "acts_return_group_split_preview",
                serde_json::json!({"actId": f.act, "payload": ret_dto(&f, &[0])}),
            )
            .await,
        );
        assert_eq!(partial.items.len(), 1, "частичный возврат: вопрос нужен");
        assert_eq!(partial.items[0].group_name, "АРМ #3");
        assert!(partial.place_conflicts.is_empty());

        let full = parse_preview(
            post(
                &app,
                &cookie,
                "acts_return_group_split_preview",
                serde_json::json!({"actId": f.act, "payload": ret_dto(&f, &[0, 1, 2])}),
            )
            .await,
        );
        assert!(full.items.is_empty(), "полный возврат: вопроса нет");
        ctx.shutdown.cancel();
    })
    .await
    .expect("budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_update_return_preview_asks_for_added_partial() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (ctx, _dir, app, cookie, _emp) = setup().await;
        let f = issued_arm(&ctx, "u", 4).await;
        let ret = ctx
            .acts
            .do_return(&Identity::trusted_admin(), f.act, ret_dto(&f, &[0]))
            .await
            .expect("return");
        let ur = ActUpdateReturnDto {
            id: ret.id,
            expected_version: ret.version,
            giver_name: ret.giver_name.clone(),
            receiver_name: ret.receiver_name.clone(),
            place_id: ret.place_id,
            notes: None,
            deadline_utc: None,
            handover_date_utc: ret.handover_date_utc,
            bulk_condition: None,
            bulk_place_id: Some(f.place_c),
            apply_to_all: true,
            items: vec![
                ret_item(f.items[0], f.devices[0], None),
                ret_item(f.items[1], f.devices[1], None),
            ],
        };
        let p = parse_preview(
            post(
                &app,
                &cookie,
                "acts_update_return_group_split_preview",
                serde_json::json!({"payload": ur}),
            )
            .await,
        );
        assert_eq!(p.items.len(), 1, "правка с добавлением: вопрос нужен");
        ctx.shutdown.cancel();
    })
    .await
    .expect("budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_return_applies_group_split_choice() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (ctx, _dir, app, cookie, _emp) = setup().await;

        // move_group: группа с возвращаемым переезжает.
        let f = issued_arm(&ctx, "m", 5).await;
        let (s, ct, b) = post(
            &app,
            &cookie,
            "acts_return",
            serde_json::json!({
                "actId": f.act, "payload": ret_dto(&f, &[0]), "groupSplit": "move_group"
            }),
        )
        .await;
        assert_eq!(s, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
        assert_json(&ct, &b);
        assert_eq!(group_place(&ctx, f.group).await, Some(f.place_c));

        // без groupSplit: Release — группа на месте, возвращённый вышел из неё.
        let g = issued_arm(&ctx, "r", 6).await;
        let before = group_place(&ctx, g.group).await;
        let (s, ct, b) = post(
            &app,
            &cookie,
            "acts_return",
            serde_json::json!({"actId": g.act, "payload": ret_dto(&g, &[0])}),
        )
        .await;
        assert_eq!(s, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
        assert_json(&ct, &b);
        assert_eq!(group_place(&ctx, g.group).await, before, "группа на месте");
        assert_eq!(member_rows(&ctx, g.devices[0]).await, 0, "вышел из группы");
        assert_eq!(member_rows(&ctx, g.devices[1]).await, 1);

        // невалидное значение — отказ десериализации.
        let h = issued_arm(&ctx, "x", 7).await;
        let (s, _, _) = post(
            &app,
            &cookie,
            "acts_return",
            serde_json::json!({
                "actId": h.act, "payload": ret_dto(&h, &[0]), "groupSplit": "bogus"
            }),
        )
        .await;
        assert!(s.is_client_error(), "статус {s}");
        ctx.shutdown.cancel();
    })
    .await
    .expect("budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_return_whole_group_with_different_places_is_validation_error() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (ctx, _dir, app, cookie, _emp) = setup().await;
        let f = issued_arm(&ctx, "c", 8).await;
        let other = {
            let n = "Склад Г".to_string();
            ctx.writer
                .execute(move |conn| {
                    conn.execute(
                        "INSERT INTO places (kind, name, parent_id, is_storage, created_at_utc, \
                         updated_at_utc, version) VALUES ('room', ?1, NULL, 0, 1700000000, 1700000000, 1)",
                        params![n],
                    )
                    .map_err(map_rusqlite)?;
                    Ok(conn.last_insert_rowid())
                })
                .await
                .unwrap()
        };
        let mut dto = ret_dto(&f, &[0, 1, 2]);
        dto.items[0].place_id_override = Some(other);
        let acts_before = act_count(&ctx).await;

        let (s, ct, b) = post(
            &app,
            &cookie,
            "acts_return",
            serde_json::json!({"actId": f.act, "payload": dto}),
        )
        .await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_json(&ct, &b);
        let text = String::from_utf8_lossy(&b).to_string();
        assert!(text.contains("place_id"), "{text}");
        assert!(text.contains("АРМ #8"), "{text}");
        assert_eq!(act_count(&ctx).await, acts_before, "акт не создан");

        let p = parse_preview(
            post(
                &app,
                &cookie,
                "acts_return_group_split_preview",
                serde_json::json!({"actId": f.act, "payload": dto}),
            )
            .await,
        );
        assert_eq!(p.place_conflicts.len(), 1);
        assert_eq!(p.place_conflicts[0].group_name, "АРМ #8");
        ctx.shutdown.cancel();
    })
    .await
    .expect("budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_preview_denied_for_employee() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (ctx, _dir, app, _admin, emp) = setup().await;
        let f = issued_arm(&ctx, "e", 9).await;
        let (s, ct, b) = post(
            &app,
            &emp,
            "acts_return_group_split_preview",
            serde_json::json!({"actId": f.act, "payload": ret_dto(&f, &[0])}),
        )
        .await;
        assert_eq!(s, StatusCode::FORBIDDEN);
        assert_json(&ct, &b);
        let (s, ct, b) = post(
            &app,
            &emp,
            "acts_update_return_group_split_preview",
            serde_json::json!({"payload": ActUpdateReturnDto::default()}),
        )
        .await;
        assert_eq!(s, StatusCode::FORBIDDEN);
        assert_json(&ct, &b);
        ctx.shutdown.cancel();
    })
    .await
    .expect("budget");
}
