//! Админ-API Link (Phase 44-16, D-05/D-12/D-13): 9 маршрутов, роль Admin,
//! выпуск кода с SPKI запущенного сервера, отзыв и допуск, `LinkChanged`.
//! Имена вымышленные. Роутер собран настоящим `build_router`.

mod im_support;
use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{header, Request, StatusCode};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use im_support::{build_ctx, cookie, seed_user};
use rusqlite::params;
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;
use trackly_app::context::AppCtx;
use trackly_app::dto::printer::WsEvent;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::http::link::{SESSION_DEVICE_KEY, SESSION_LOGIN_KEY};
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::server::{ServerHandle, TlsInfo};
use trackly_app::services::link_service::{DeviceMeta, LoginMethod};
use trackly_core::auth::{Identity, Role};
use trackly_infra::error_conversions::map_rusqlite;

const DEV_A: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const SPKI: [u8; 32] = [9u8; 32];

struct Fx {
    ctx: AppCtx,
    _dir: tempfile::TempDir,
    app: axum::Router,
    admin: i64,
    admin_cookie: String,
    manager_cookie: String,
    employee_cookie: String,
}

async fn fx() -> Fx {
    let (ctx, dir) = build_ctx().await;
    let admin = seed_user(&ctx, "la_adm", "Смирнов С.С.", "admin").await;
    let mgr = seed_user(&ctx, "la_mgr", "Кузнецов К.К.", "manager").await;
    let emp = seed_user(&ctx, "la_emp", "Орлов О.О.", "employee").await;
    let admin_cookie = cookie(&ctx, admin, Role::Admin).await;
    let manager_cookie = cookie(&ctx, mgr, Role::Manager).await;
    let employee_cookie = cookie(&ctx, emp, Role::Employee).await;
    let store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
    let app = build_router(&ctx, store);
    Fx {
        ctx,
        _dir: dir,
        app,
        admin,
        admin_cookie,
        manager_cookie,
        employee_cookie,
    }
}

/// Имитирует запущенный сервер: в `server_ctl` кладётся handle с известным SPKI.
async fn fake_server(ctx: &AppCtx) {
    let tls = TlsInfo {
        fingerprint_hex: link_proto::fmt_colon_upper(&[1u8; 32]),
        spki_fingerprint_hex: link_proto::fmt_colon_upper(&SPKI),
        host: "0.0.0.0".into(),
        port: 8443,
    };
    *ctx.server_ctl.lock().await = Some(ServerHandle {
        tls,
        cancel: tokio_util::sync::CancellationToken::new(),
        task: tokio::spawn(async {}),
    });
}

struct Resp {
    status: StatusCode,
    json: Value,
}

async fn call(app: &axum::Router, name: &str, cookie: Option<&str>, body: Value) -> Resp {
    let mut b = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/{name}"))
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    let mut req = b.body(Body::from(body.to_string())).unwrap();
    req.extensions_mut().insert(ConnectInfo::<SocketAddr>(
        "192.0.2.55:4000".parse().unwrap(),
    ));
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
        .await
        .unwrap();
    Resp {
        status,
        json: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    }
}

async fn enroll(ctx: &AppCtx, admin: i64, device: &str) -> String {
    let issued = ctx
        .link
        .create_invite("trackly.example.test", 8443, SPKI, 3600, admin)
        .await
        .unwrap();
    let decoded = link_proto::InviteCode::decode(&issued.code).unwrap();
    ctx.link
        .enroll(
            &URL_SAFE_NO_PAD.encode(decoded.secret),
            DeviceMeta {
                device_id: device.to_string(),
                hostname: "PC-EXAMPLE-01".into(),
                os: "windows".into(),
                arch: "x86_64".into(),
                client_version: "1.0.0".into(),
                ip: None,
            },
        )
        .await
        .unwrap()
}

/// Link-сессия сотрудника на устройстве (cookie).
async fn link_cookie(ctx: &AppCtx, user_id: i64, device: &str, login_id: i64) -> String {
    let id = Id::default();
    let si = SessionIdentity {
        user_id: Some(user_id),
        role: "employee".to_string(),
    };
    let mut record = Record {
        id,
        data: Default::default(),
        expiry_date: time::OffsetDateTime::now_utc() + time::Duration::days(1),
    };
    record
        .data
        .insert("identity".into(), serde_json::to_value(&si).unwrap());
    record.data.insert(SESSION_DEVICE_KEY.into(), json!(device));
    record
        .data
        .insert(SESSION_LOGIN_KEY.into(), json!(login_id));
    RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone())
        .create(&mut record)
        .await
        .unwrap();
    format!("id={id}")
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn drain_link_changed(rx: &mut tokio::sync::broadcast::Receiver<WsEvent>) -> usize {
    let mut n = 0;
    while let Ok(ev) = rx.try_recv() {
        if matches!(ev, WsEvent::LinkChanged {}) {
            n += 1;
        }
    }
    n
}

/// Все 9 маршрутов с валидными телами (иначе тело отвергнется раньше проверки роли).
fn all_routes() -> Vec<(&'static str, Value)> {
    vec![
        (
            "link_invites_create",
            json!({ "req": { "host": "", "ttlHours": 24 } }),
        ),
        ("link_invites_list", json!({})),
        ("link_invites_revoke", json!({ "id": 999_999 })),
        ("link_logins_list", json!({})),
        ("link_logins_revoke", json!({ "loginId": 999_999 })),
        ("link_devices_list", json!({})),
        ("link_devices_allow", json!({ "deviceId": "nope" })),
        ("users_link_status", json!({ "userId": 1 })),
        ("users_revoke_link", json!({ "userId": 1 })),
    ]
}

// --- Test 1 (D-05): выпуск кода ---------------------------------------------

#[tokio::test]
async fn invite_is_issued_with_running_server_spki_and_requested_host() {
    let f = fx().await;
    fake_server(&f.ctx).await;
    let r = call(
        &f.app,
        "link_invites_create",
        Some(&f.admin_cookie),
        json!({ "req": { "host": "trackly.example.test", "ttlHours": 24 } }),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let code = r.json["code"].as_str().unwrap();
    let decoded = link_proto::InviteCode::decode(code).unwrap();
    assert_eq!(decoded.spki, SPKI, "SPKI берётся у запущенного сервера");
    assert_eq!(decoded.host, "trackly.example.test");
    assert_eq!(decoded.port, 8443);
    assert_eq!(r.json["host"], "trackly.example.test");
    assert!(decoded.exp_unix - now() > 23 * 3600);
}

#[tokio::test]
async fn invite_host_defaults_to_server_display_host() {
    let f = fx().await;
    fake_server(&f.ctx).await;
    let r = call(
        &f.app,
        "link_invites_create",
        Some(&f.admin_cookie),
        json!({ "req": { "host": "  ", "ttlHours": 1 } }),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let decoded = link_proto::InviteCode::decode(r.json["code"].as_str().unwrap()).unwrap();
    assert_eq!(
        decoded.host,
        trackly_app::server::tls::display_host("0.0.0.0")
    );
}

#[tokio::test]
async fn invite_without_running_server_is_409() {
    let f = fx().await;
    let r = call(
        &f.app,
        "link_invites_create",
        Some(&f.admin_cookie),
        json!({ "req": { "host": "x.example.test", "ttlHours": 24 } }),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.json);
}

#[tokio::test]
async fn invite_ttl_outside_allowed_set_is_rejected() {
    let f = fx().await;
    fake_server(&f.ctx).await;
    for bad in [0, 2, 25, 1000, -1] {
        let r = call(
            &f.app,
            "link_invites_create",
            Some(&f.admin_cookie),
            json!({ "req": { "host": "x.example.test", "ttlHours": bad } }),
        )
        .await;
        assert!(r.status.is_client_error(), "ttl {bad}: {}", r.status);
    }
    for ok in [1, 24, 168] {
        let r = call(
            &f.app,
            "link_invites_create",
            Some(&f.admin_cookie),
            json!({ "req": { "host": "x.example.test", "ttlHours": ok } }),
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "ttl {ok}");
    }
}

// --- Test 2 (D-05): роли на всех 9 маршрутах ---------------------------------

#[tokio::test]
async fn all_nine_routes_401_without_session_and_403_for_non_admin() {
    let f = fx().await;
    fake_server(&f.ctx).await;
    let routes = all_routes();
    assert_eq!(routes.len(), 9);
    for (name, body) in &routes {
        let r = call(&f.app, name, None, body.clone()).await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{name} без сессии");
        for (who, c) in [
            ("manager", &f.manager_cookie),
            ("employee", &f.employee_cookie),
        ] {
            let r = call(&f.app, name, Some(c), body.clone()).await;
            assert_eq!(r.status, StatusCode::FORBIDDEN, "{name} для {who}");
            assert!(!r.json.is_null(), "{name}: тело 403 должно быть JSON");
        }
        // Админ проходит границу роли (не 401/403).
        let r = call(&f.app, name, Some(&f.admin_cookie), body.clone()).await;
        assert!(
            r.status != StatusCode::UNAUTHORIZED && r.status != StatusCode::FORBIDDEN,
            "{name} для admin: {}",
            r.status
        );
    }
    // Отказ не-админу не оставил побочных эффектов: ни одного кода не выпущено.
    let n: i64 = f
        .ctx
        .readers
        .acquire()
        .query_row("SELECT COUNT(*) FROM link_invites", [], |r| r.get(0))
        .unwrap();
    // 1 — выпуск админом выше (остальные два отказа).
    assert_eq!(n, 1);
}

#[test]
fn build_helpers_gate_every_route_with_manage_users() {
    // Серверная граница в хелперах: каждый build_* зовёт require_admin.
    let src = include_str!("../src/tauri_cmds/link.rs");
    let helpers = src.matches("pub async fn build_").count();
    assert_eq!(helpers, 9);
    assert_eq!(src.matches("require_admin(caller)?").count(), 9);
    assert!(src.contains("Action::ManageUsers"));
}

// --- Test 3: список / отзыв кодов --------------------------------------------

#[tokio::test]
async fn invites_list_and_revoke_block_enrollment() {
    let f = fx().await;
    fake_server(&f.ctx).await;
    let issued = call(
        &f.app,
        "link_invites_create",
        Some(&f.admin_cookie),
        json!({ "req": { "host": "x.example.test", "ttlHours": 24 } }),
    )
    .await;
    let code = issued.json["code"].as_str().unwrap().to_string();
    let id = issued.json["id"].as_i64().unwrap();

    let list = call(
        &f.app,
        "link_invites_list",
        Some(&f.admin_cookie),
        json!({}),
    )
    .await;
    assert_eq!(list.status, StatusCode::OK);
    let rows = list.json.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["status"], "active");
    assert_eq!(rows[0]["createdByFullName"], "Смирнов С.С.");
    // T-44-61: ни кода, ни хэша в списке.
    let raw = list.json.to_string();
    assert!(!raw.contains(&code) && !raw.to_lowercase().contains("hash"));

    let rv = call(
        &f.app,
        "link_invites_revoke",
        Some(&f.admin_cookie),
        json!({ "id": id }),
    )
    .await;
    assert_eq!(rv.status, StatusCode::OK, "{}", rv.json);
    let list = call(
        &f.app,
        "link_invites_list",
        Some(&f.admin_cookie),
        json!({}),
    )
    .await;
    assert_eq!(list.json[0]["status"], "revoked");

    let decoded = link_proto::InviteCode::decode(&code).unwrap();
    let res = f
        .ctx
        .link
        .enroll(
            &URL_SAFE_NO_PAD.encode(decoded.secret),
            DeviceMeta {
                device_id: DEV_A.into(),
                ..Default::default()
            },
        )
        .await;
    assert!(res.is_err(), "отозванный код не погашается");
}

#[tokio::test]
async fn invites_older_than_30_days_are_hidden() {
    let f = fx().await;
    let admin = f.admin;
    let n = now();
    f.ctx
        .writer
        .execute(move |c| {
            for (h, exp) in [
                (vec![1u8], n - 40 * 86_400),
                (vec![2u8], n - 5 * 86_400),
                (vec![3u8], n + 3600),
            ] {
                c.execute(
                    "INSERT INTO link_invites (code_hash, created_by_user_id, created_at_utc, \
                     expires_at_utc) VALUES (?1, ?2, ?3, ?4)",
                    params![h, admin, exp - 3600, exp],
                )
                .map_err(map_rusqlite)?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let list = f.ctx.link.list_invite_dtos().await.unwrap();
    let statuses: Vec<&str> = list.iter().map(|r| r.status.as_str()).collect();
    assert_eq!(list.len(), 2, "{statuses:?}");
    assert!(statuses.contains(&"active") && statuses.contains(&"expired"));
}

// --- Test 4 (D-12): активные входы и отзыв -----------------------------------

#[tokio::test]
async fn logins_list_and_revoke_bans_device_and_kills_client_session() {
    let f = fx().await;
    enroll(&f.ctx, f.admin, DEV_A).await;
    let uid = seed_user(&f.ctx, "petrov", "Петров П.П.", "employee").await;
    let issued = f
        .ctx
        .link
        .record_login(DEV_A, uid, LoginMethod::Password)
        .await
        .unwrap();
    let client = link_cookie(&f.ctx, uid, DEV_A, issued.login_id).await;

    let list = call(&f.app, "link_logins_list", Some(&f.admin_cookie), json!({})).await;
    assert_eq!(list.status, StatusCode::OK);
    let rows = list.json.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["fullName"], "Петров П.П.");
    assert_eq!(rows[0]["login"], "petrov");
    assert_eq!(rows[0]["hostname"], "PC-EXAMPLE-01");
    assert_eq!(rows[0]["method"], "password");
    assert_eq!(rows[0]["loginId"], issued.login_id);

    let ok = call(&f.app, "auth_status", Some(&client), json!({})).await;
    assert!(!ok.json["user"].is_null());

    let rv = call(
        &f.app,
        "link_logins_revoke",
        Some(&f.admin_cookie),
        json!({ "loginId": issued.login_id }),
    )
    .await;
    assert_eq!(rv.status, StatusCode::OK, "{}", rv.json);

    let after = call(&f.app, "auth_status", Some(&client), json!({})).await;
    assert_eq!(after.status, StatusCode::UNAUTHORIZED);
    assert_eq!(after.json["code"], "LINK_REVOKED");
    let banned = call(
        &f.app,
        "link_devices_list",
        Some(&f.admin_cookie),
        json!({}),
    )
    .await;
    assert_eq!(banned.json.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn revoke_unknown_login_is_404() {
    let f = fx().await;
    let r = call(
        &f.app,
        "link_logins_revoke",
        Some(&f.admin_cookie),
        json!({ "loginId": 424242 }),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

// --- Test 5 (D-13): запрещённые устройства и допуск --------------------------

#[tokio::test]
async fn devices_list_and_allow_restores_access_without_new_code() {
    let f = fx().await;
    let secret = enroll(&f.ctx, f.admin, DEV_A).await;
    let uid = seed_user(&f.ctx, "sidorov", "Сидоров С.С.", "employee").await;
    let l = f
        .ctx
        .link
        .record_login(DEV_A, uid, LoginMethod::Password)
        .await
        .unwrap();
    f.ctx.link.revoke_login(l.login_id, f.admin).await.unwrap();
    assert!(f.ctx.link.verify_device(DEV_A, &secret).await.is_err());

    let list = call(
        &f.app,
        "link_devices_list",
        Some(&f.admin_cookie),
        json!({}),
    )
    .await;
    assert_eq!(list.status, StatusCode::OK);
    let rows = list.json.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["deviceId"], DEV_A);
    assert_eq!(rows[0]["hostname"], "PC-EXAMPLE-01");
    assert_eq!(rows[0]["lastUserFullName"], "Сидоров С.С.");
    assert!(rows[0]["revokedAt"].as_i64().unwrap() > 0);

    let al = call(
        &f.app,
        "link_devices_allow",
        Some(&f.admin_cookie),
        json!({ "deviceId": DEV_A }),
    )
    .await;
    assert_eq!(al.status, StatusCode::OK, "{}", al.json);
    f.ctx
        .link
        .verify_device(DEV_A, &secret)
        .await
        .expect("после «Разрешить» устройство снова допущено");
    let list = call(
        &f.app,
        "link_devices_list",
        Some(&f.admin_cookie),
        json!({}),
    )
    .await;
    assert!(list.json.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn allow_unknown_device_is_404() {
    let f = fx().await;
    let r = call(
        &f.app,
        "link_devices_allow",
        Some(&f.admin_cookie),
        json!({ "deviceId": "no-such" }),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

// --- Test 6 (D-12): входы пользователя ---------------------------------------

#[tokio::test]
async fn user_link_status_counts_and_revoke_all_bans_devices() {
    let f = fx().await;
    enroll(&f.ctx, f.admin, DEV_A).await;
    enroll(&f.ctx, f.admin, "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb").await;
    let uid = seed_user(&f.ctx, "ivanov", "Иванов И.И.", "employee").await;
    for d in [DEV_A, "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"] {
        f.ctx
            .link
            .record_login(d, uid, LoginMethod::Sso)
            .await
            .unwrap();
    }
    let st = call(
        &f.app,
        "users_link_status",
        Some(&f.admin_cookie),
        json!({ "userId": uid }),
    )
    .await;
    assert_eq!(st.status, StatusCode::OK);
    assert_eq!(st.json["activeLogins"], 2);

    let rv = call(
        &f.app,
        "users_revoke_link",
        Some(&f.admin_cookie),
        json!({ "userId": uid }),
    )
    .await;
    assert_eq!(rv.status, StatusCode::OK, "{}", rv.json);
    assert_eq!(rv.json["revoked"], 2);

    let st = call(
        &f.app,
        "users_link_status",
        Some(&f.admin_cookie),
        json!({ "userId": uid }),
    )
    .await;
    assert_eq!(st.json["activeLogins"], 0);
    let banned = call(
        &f.app,
        "link_devices_list",
        Some(&f.admin_cookie),
        json!({}),
    )
    .await;
    assert_eq!(banned.json.as_array().unwrap().len(), 2);
}

// --- Test 7: маршруты не маскируются spa_fallback -----------------------------

#[tokio::test]
async fn routes_answer_json_and_unknown_link_route_is_json_404() {
    let f = fx().await;
    for (name, body) in all_routes() {
        let r = call(&f.app, name, Some(&f.admin_cookie), body).await;
        assert!(
            !r.json.is_null() || r.status == StatusCode::OK,
            "{name}: ответ не JSON ({})",
            r.status
        );
    }
    let r = call(
        &f.app,
        "link_no_such_route",
        Some(&f.admin_cookie),
        json!({}),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

// --- Test 1/2/6 задачи 1: DTO -------------------------------------------------

#[tokio::test]
async fn login_dtos_sorted_by_created_at_desc() {
    let f = fx().await;
    enroll(&f.ctx, f.admin, DEV_A).await;
    let a = seed_user(&f.ctx, "u_a", "Антонов А.А.", "employee").await;
    let b = seed_user(&f.ctx, "u_b", "Борисов Б.Б.", "employee").await;
    let la = f
        .ctx
        .link
        .record_login(DEV_A, a, LoginMethod::Sso)
        .await
        .unwrap();
    let lb = f
        .ctx
        .link
        .record_login(DEV_A, b, LoginMethod::Password)
        .await
        .unwrap();
    f.ctx
        .writer
        .execute(move |c| {
            c.execute(
                "UPDATE link_logins SET created_at_utc = 1000 WHERE id = ?1",
                params![la.login_id],
            )
            .map_err(map_rusqlite)?;
            c.execute(
                "UPDATE link_logins SET created_at_utc = 2000 WHERE id = ?1",
                params![lb.login_id],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .unwrap();
    let rows = f.ctx.link.list_login_dtos().await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].user_id, b);
    assert_eq!(rows[1].user_id, a);
    assert!(rows[0].created_at > rows[1].created_at);
    assert_eq!(rows[0].login, "u_b");
}

#[test]
fn dtos_do_not_expose_code_or_hash_fields() {
    let src = include_str!("../src/dto/link.rs");
    // Единственное поле `code` — в InviteIssuedDto (выдача один раз).
    assert_eq!(src.matches("pub code:").count(), 1);
    let issued = src.split("pub struct InviteIssuedDto").nth(1).unwrap();
    assert!(issued.split('}').next().unwrap().contains("pub code:"));
    assert!(!src.to_lowercase().contains("hash"));
}

// --- Test 2 задачи 3: LinkChanged после коммита -------------------------------

#[tokio::test]
async fn link_changed_sent_once_per_mutation_and_not_on_rollback() {
    let f = fx().await;
    let mut rx = f.ctx.ws_broadcast.subscribe();

    let issued = f
        .ctx
        .link
        .create_invite("trackly.example.test", 8443, SPKI, 3600, f.admin)
        .await
        .unwrap();
    assert_eq!(drain_link_changed(&mut rx), 1, "create_invite");

    let decoded = link_proto::InviteCode::decode(&issued.code).unwrap();
    f.ctx
        .link
        .enroll(
            &URL_SAFE_NO_PAD.encode(decoded.secret),
            DeviceMeta {
                device_id: DEV_A.into(),
                hostname: "PC-EXAMPLE-01".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(drain_link_changed(&mut rx), 1, "enroll");

    let uid = seed_user(&f.ctx, "petrov2", "Петров П.П.", "employee").await;
    let l1 = f
        .ctx
        .link
        .record_login(DEV_A, uid, LoginMethod::Password)
        .await
        .unwrap();
    assert_eq!(drain_link_changed(&mut rx), 1, "record_login");
    f.ctx.link.logout(l1.login_id).await.unwrap();
    assert_eq!(drain_link_changed(&mut rx), 1, "logout");

    let l2 = f
        .ctx
        .link
        .record_login(DEV_A, uid, LoginMethod::Password)
        .await
        .unwrap();
    drain_link_changed(&mut rx);
    f.ctx.link.revoke_login(l2.login_id, f.admin).await.unwrap();
    assert_eq!(drain_link_changed(&mut rx), 1, "revoke_login");
    f.ctx.link.allow_device(DEV_A).await.unwrap();
    assert_eq!(drain_link_changed(&mut rx), 1, "allow_device");
    f.ctx
        .link
        .record_login(DEV_A, uid, LoginMethod::Sso)
        .await
        .unwrap();
    drain_link_changed(&mut rx);
    f.ctx
        .link
        .revoke_user_logins(uid, f.admin, true)
        .await
        .unwrap();
    assert_eq!(drain_link_changed(&mut rx), 1, "revoke_user_logins");

    let second = f
        .ctx
        .link
        .create_invite("trackly.example.test", 8443, SPKI, 3600, f.admin)
        .await
        .unwrap();
    drain_link_changed(&mut rx);
    f.ctx.link.revoke_invite(second.id, f.admin).await.unwrap();
    assert_eq!(drain_link_changed(&mut rx), 1, "revoke_invite");

    // Неудачные операции (запись не прошла) событий не порождают.
    assert!(f.ctx.link.revoke_invite(999_999, f.admin).await.is_err());
    assert!(f.ctx.link.revoke_login(999_999, f.admin).await.is_err());
    assert!(f.ctx.link.allow_device("no-such").await.is_err());
    assert!(f.ctx.link.revoke_invite(second.id, f.admin).await.is_err());
    assert_eq!(drain_link_changed(&mut rx), 0, "откат/ошибка без события");
}

#[tokio::test]
async fn link_changed_is_visible_only_to_admin_subscribers() {
    let f = fx().await;
    let mut rx = f.ctx.ws_broadcast.subscribe();
    f.ctx
        .link
        .create_invite("trackly.example.test", 8443, SPKI, 3600, f.admin)
        .await
        .unwrap();
    let ev = rx.try_recv().expect("событие отправлено");
    let id = |role| Identity {
        user_id: Some(1),
        role,
    };
    // Тот же предикат, что применяет ws_loop к каждому кадру перед отправкой клиенту.
    assert!(ev.is_visible_to(&id(Role::Admin)));
    assert!(!ev.is_visible_to(&id(Role::Manager)));
    assert!(!ev.is_visible_to(&id(Role::Employee)));
}
