//! Фаза 41.8 план 32 (D-34): паритет транспортов Tauri и HTTP для переноса группы со
//! сменой статуса и предпросмотра. Реальный `build_router`; тело ответа проверяется как
//! JSON: отсутствующий /api/ маршрут spa_fallback маскирует под index.html с 200.
//!
//! Префикс тестов: `groups_move_status_transport_`. Данные вымышленные.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use rusqlite::params;
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_app::dto::groups::{
    GroupMoveDto, GroupMoveResultDto, GroupMoveStatusPreviewDto, GroupMoveStatusPreviewQueryDto,
    GroupMoveStatusTransitionDto,
};
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::tauri_cmds::groups::{build_groups_move, build_groups_move_status_preview};
use trackly_core::auth::{Identity, Role};
use trackly_infra::error_conversions::map_rusqlite;

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

async fn seed_place(ctx: &AppCtx, name: &str, storage: bool) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, parent_id, is_storage, created_at_utc, \
                 updated_at_utc, version) VALUES ('room', ?1, NULL, ?2, 1700000000, 1700000000, 1)",
                params![name, i64::from(storage)],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

/// Группа `name` на `place` из двух устройств «На складе»; возвращает (group, devices).
async fn seed_group(ctx: &AppCtx, name: &str, seq: i64, place: i64) -> (i64, Vec<i64>) {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            let type_id: i64 = conn
                .query_row(
                    "SELECT id FROM group_types WHERE code = 'workstation'",
                    [],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, NULL, 1700000000, 1700000000, 1)",
                params![type_id, name, seq, place],
            )
            .map_err(map_rusqlite)?;
            let group = conn.last_insert_rowid();
            let mut devices = Vec::new();
            for (i, n) in ["Монитор", "Мышь"].iter().enumerate() {
                conn.execute(
                    "INSERT INTO devices (type_id, name, inventory_number, place_id, status_id, \
                     created_at_utc, updated_at_utc, version) \
                     VALUES (1, ?1, ?2, ?3, (SELECT id FROM device_statuses WHERE code = 'на_складе'), \
                     1700000000, 1700000000, 1)",
                    params![format!("{n} #{seq}"), format!("INV-{seq}-{i}"), place],
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
        .expect("seed group")
}

/// (версия группы, место группы, [(место, статус) устройств], число записей аудита).
async fn snapshot(
    ctx: &AppCtx,
    group: i64,
    devices: Vec<i64>,
) -> (i64, Option<i64>, Vec<(Option<i64>, String)>, i64) {
    ctx.writer
        .execute(move |conn| {
            let (v, p): (i64, Option<i64>) = conn
                .query_row(
                    "SELECT version, place_id FROM groups WHERE id = ?1",
                    params![group],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .map_err(map_rusqlite)?;
            let mut devs = Vec::new();
            for d in devices {
                devs.push(
                    conn.query_row(
                        "SELECT d.place_id, s.code FROM devices d \
                         JOIN device_statuses s ON s.id = d.status_id WHERE d.id = ?1",
                        params![d],
                        |r| Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, String>(1)?)),
                    )
                    .map_err(map_rusqlite)?,
                );
            }
            let audit: i64 = conn
                .query_row("SELECT COUNT(*) FROM audit_log", [], |r| r.get(0))
                .map_err(map_rusqlite)?;
            Ok((v, p, devs, audit))
        })
        .await
        .expect("snapshot")
}

async fn post(
    app: &Router,
    cookie: Option<&str>,
    route: &str,
    body: serde_json::Value,
) -> (StatusCode, Option<String>, Vec<u8>) {
    let mut b = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/{route}"))
        .header("content-type", "application/json");
    if let Some(c) = cookie {
        b = b.header("cookie", c);
    }
    let res = app
        .clone()
        .oneshot(
            b.body(Body::from(serde_json::to_string(&body).unwrap()))
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

struct Env {
    ctx: AppCtx,
    _dir: tempfile::TempDir,
    app: Router,
    admin: String,
    manager: String,
    employee: String,
    store: i64,
    room: i64,
}

async fn setup() -> Env {
    let (ctx, dir) = build_ctx().await;
    let a = make_user(&ctx, "admin_t", "admin").await;
    let m = make_user(&ctx, "manager_t", "manager").await;
    let e = make_user(&ctx, "emp_t", "employee").await;
    let sessions = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
    let admin = session_cookie(&sessions, a, "admin").await;
    let manager = session_cookie(&sessions, m, "manager").await;
    let employee = session_cookie(&sessions, e, "employee").await;
    let app = build_router(&ctx, sessions);
    let store = seed_place(&ctx, "Склад А", true).await;
    let room = seed_place(&ctx, "Кабинет 12", false).await;
    Env {
        ctx,
        _dir: dir,
        app,
        admin,
        manager,
        employee,
        store,
        room,
    }
}

fn admin_identity() -> Identity {
    Identity::trusted_admin()
}

async fn http_preview(
    env: &Env,
    cookie: &str,
    group: i64,
    target: i64,
) -> GroupMoveStatusPreviewDto {
    let r = post(
        &env.app,
        Some(cookie),
        "groups_move_status_preview",
        serde_json::json!({"dto": {"id": group, "target_place_id": target}}),
    )
    .await;
    assert_eq!(r.0, StatusCode::OK, "{}", String::from_utf8_lossy(&r.2));
    assert_json(&r.1, &r.2);
    let v: serde_json::Value = serde_json::from_slice(&r.2).unwrap();
    assert!(v.get("devices_affected").is_some(), "{v}");
    serde_json::from_value(v).expect("GroupMoveStatusPreviewDto")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn groups_move_status_transport_preview_parity_and_read_only() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let env = setup().await;
        let (g, devs) = seed_group(&env.ctx, "АРМ #9", 9, env.store).await;
        let before = snapshot(&env.ctx, g, devs.clone()).await;

        let tauri = build_groups_move_status_preview(
            &env.ctx,
            &admin_identity(),
            GroupMoveStatusPreviewQueryDto {
                id: g,
                target_place_id: env.room,
            },
        )
        .await
        .expect("tauri preview");
        let http = http_preview(&env, &env.admin, g, env.room).await;
        let http_mgr = http_preview(&env, &env.manager, g, env.room).await;

        assert_eq!(tauri, http, "Tauri и HTTP расходятся");
        assert_eq!(http, http_mgr, "manager видит то же, что admin");
        assert_eq!(tauri.transition, Some(GroupMoveStatusTransitionDto::ToWork));
        assert_eq!(tauri.devices_affected, 2);
        assert!(tauri.target_status_name.is_some());

        // Предпросмотр ничего не пишет (версия, места, статусы, аудит).
        assert_eq!(before, snapshot(&env.ctx, g, devs).await);
    })
    .await
    .expect("тест завис");
}

fn comparable(r: &GroupMoveResultDto) -> (i32, i32, i32, String) {
    (
        r.moved_devices,
        r.moved_nested_groups,
        r.status_changed_devices,
        r.summary.clone(),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn groups_move_status_transport_move_with_flag_parity() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let env = setup().await;
        // Два одинаковых АРМ: один переносим через Tauri-путь, другой через HTTP.
        let (g1, d1) = seed_group(&env.ctx, "АРМ #1", 1, env.store).await;
        let (g2, d2) = seed_group(&env.ctx, "АРМ #1", 2, env.store).await;

        let tauri = build_groups_move(
            &env.ctx,
            &admin_identity(),
            GroupMoveDto {
                id: g1,
                version: 1,
                target_place_id: env.room,
                change_status: Some(true),
            },
        )
        .await
        .expect("tauri move");

        let r = post(
            &env.app,
            Some(&env.admin),
            "groups_move",
            serde_json::json!({"dto": {
                "id": g2, "version": 1, "target_place_id": env.room, "change_status": true
            }}),
        )
        .await;
        assert_eq!(r.0, StatusCode::OK, "{}", String::from_utf8_lossy(&r.2));
        assert_json(&r.1, &r.2);
        let http: GroupMoveResultDto = serde_json::from_slice(&r.2).unwrap();

        assert_eq!(comparable(&tauri), comparable(&http));
        assert_eq!(http.status_changed_devices, 2);

        let (_, _, s1, _) = snapshot(&env.ctx, g1, d1).await;
        let (_, _, s2, _) = snapshot(&env.ctx, g2, d2).await;
        assert_eq!(s1, s2.iter().map(|_| s1[0].clone()).collect::<Vec<_>>());
        assert_ne!(s1[0].1, "на_складе", "статус должен был смениться");
        assert_eq!(s1[0].0, Some(env.room));
    })
    .await
    .expect("тест завис");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn groups_move_status_transport_move_without_flag_keeps_status() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let env = setup().await;
        let (g, devs) = seed_group(&env.ctx, "АРМ #3", 3, env.store).await;
        // Поле change_status в JSON отсутствует вовсе.
        let r = post(
            &env.app,
            Some(&env.manager),
            "groups_move",
            serde_json::json!({"dto": {"id": g, "version": 1, "target_place_id": env.room}}),
        )
        .await;
        assert_eq!(r.0, StatusCode::OK, "{}", String::from_utf8_lossy(&r.2));
        assert_json(&r.1, &r.2);
        let res: GroupMoveResultDto = serde_json::from_slice(&r.2).unwrap();
        assert_eq!(res.status_changed_devices, 0);
        let (_, place, devices, _) = snapshot(&env.ctx, g, devs).await;
        assert_eq!(place, Some(env.room));
        for (p, status) in devices {
            assert_eq!(p, Some(env.room));
            assert_eq!(status, "на_складе", "без флага статус не меняется");
        }
    })
    .await
    .expect("тест завис");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn groups_move_status_transport_employee_denied_in_both() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let env = setup().await;
        let (g, devs) = seed_group(&env.ctx, "АРМ #4", 4, env.store).await;
        let before = snapshot(&env.ctx, g, devs.clone()).await;
        let emp = Identity {
            user_id: None,
            role: Role::Employee,
        };

        assert!(build_groups_move_status_preview(
            &env.ctx,
            &emp,
            GroupMoveStatusPreviewQueryDto {
                id: g,
                target_place_id: env.room
            }
        )
        .await
        .is_err());
        assert!(build_groups_move(
            &env.ctx,
            &emp,
            GroupMoveDto {
                id: g,
                version: 1,
                target_place_id: env.room,
                change_status: Some(true)
            }
        )
        .await
        .is_err());

        for (route, body) in [
            (
                "groups_move_status_preview",
                serde_json::json!({"dto": {"id": g, "target_place_id": env.room}}),
            ),
            (
                "groups_move",
                serde_json::json!({"dto": {"id": g, "version": 1,
                    "target_place_id": env.room, "change_status": true}}),
            ),
        ] {
            let r = post(&env.app, Some(&env.employee), route, body).await;
            assert_eq!(
                r.0,
                StatusCode::FORBIDDEN,
                "{route}: {}",
                String::from_utf8_lossy(&r.2)
            );
            assert_json(&r.1, &r.2);
        }
        assert_eq!(before, snapshot(&env.ctx, g, devs).await);
    })
    .await
    .expect("тест завис");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn groups_move_status_transport_no_session_is_401() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let env = setup().await;
        let (g, _) = seed_group(&env.ctx, "АРМ #5", 5, env.store).await;
        for (route, body) in [
            (
                "groups_move_status_preview",
                serde_json::json!({"dto": {"id": g, "target_place_id": env.room}}),
            ),
            (
                "groups_move",
                serde_json::json!({"dto": {"id": g, "version": 1, "target_place_id": env.room}}),
            ),
        ] {
            let r = post(&env.app, None, route, body).await;
            assert_eq!(r.0, StatusCode::UNAUTHORIZED, "{route}");
        }
    })
    .await
    .expect("тест завис");
}
