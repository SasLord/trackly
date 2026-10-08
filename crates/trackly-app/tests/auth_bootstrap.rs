//! INT-03 (фаза 41.7, план 03): публичный маршрут первоначальной настройки
//! `POST /api/v1/auth_bootstrap`.
//!
//! Граница безопасности маршрута целиком держится на условии «в БД нет ни
//! одного активного администратора» (`needs_bootstrap`). Тесты проверяют обе
//! стороны этой границы:
//!
//! - открыто: `bootstrap_route_open_when_needs_bootstrap`,
//!   `bootstrap_route_open_when_only_non_admins_exist`;
//! - закрыто: `bootstrap_route_closed_after_first_user` (отказ 409),
//!   `bootstrap_route_race_exactly_one_wins` (отказ второму из двух параллельных).
//!
//! Плюс: роль навязывается сервером, аудит без автора, governor, невалидный
//! ввод не закрывает окно, `users_create` по-прежнему требует сессию, Tauri-путь.
//!
//! Данные вымышленные.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_app::http::auth::{build_auth_bootstrap, BootstrapPayload};
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::tauri_cmds::auth::build_auth_bootstrap_tauri;
use trackly_core::auth::Identity;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

const PEER: &str = "203.0.113.7:54321";

async fn build_test_components() -> anyhow::Result<(axum::Router, AppCtx)> {
    let dir = tempfile::TempDir::new()?;
    let dir_path = dir.keep(); // не дропаем TempDir — путь нужен живым
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir_path)?;
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    })?;
    let ctx = AppCtx::build(paths, config, log_guard).await?;
    let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
    let router = build_router(&ctx, session_store);
    Ok((router, ctx))
}

/// Тело запроса. Роль «employee» присылается СПЕЦИАЛЬНО неверной: сервер обязан
/// её проигнорировать.
fn body_with(login: &str, password: &str) -> String {
    serde_json::json!({
        "userNew": {
            "login": login,
            "full_name": "Иванов И.И.",
            "password": password,
            "role": "employee",
            "email": null
        }
    })
    .to_string()
}

async fn post(router: &axum::Router, uri: &str, body: String) -> (StatusCode, Vec<u8>) {
    // Без ConnectInfo GovernorLayer отвечает 500 «Unable To Extract Key!».
    let addr: std::net::SocketAddr = PEER.parse().unwrap();
    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .extension(axum::extract::ConnectInfo(addr))
                .body(Body::from(body))
                .expect("build request"),
        )
        .await
        .expect("oneshot");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 64 * 1024)
        .await
        .expect("read body")
        .to_vec();
    (status, bytes)
}

async fn scalar_i64(ctx: &AppCtx, sql: &'static str) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, [], |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("scalar query")
}

async fn admin_count(ctx: &AppCtx) -> i64 {
    scalar_i64(
        ctx,
        "SELECT COUNT(*) FROM users WHERE role = 'admin' AND deleted_at_utc IS NULL",
    )
    .await
}

fn user_new(login: &str) -> UserNew {
    UserNew {
        login: login.to_string(),
        full_name: "Петров П.П.".to_string(),
        password: "password123".to_string(),
        role: "employee".to_string(),
        email: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bootstrap_route_open_when_needs_bootstrap() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (router, ctx) = build_test_components().await.expect("components");
        assert!(ctx.auth.needs_bootstrap().await.expect("needs_bootstrap"));

        let (status, bytes) = post(
            &router,
            "/api/v1/auth_bootstrap",
            body_with("bootstrap_admin", "password123"),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "открытое окно: ждали 200, тело: {}",
            String::from_utf8_lossy(&bytes)
        );
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).expect("ответ обязан быть JSON с пользователем");
        assert_eq!(json["login"], "bootstrap_admin");
        assert_eq!(
            json["role"], "admin",
            "роль навязывается сервером, присланная employee игнорируется"
        );

        assert!(!ctx.auth.needs_bootstrap().await.expect("needs_bootstrap"));
        assert_eq!(admin_count(&ctx).await, 1);

        let audit = scalar_i64(
            &ctx,
            "SELECT COUNT(*) FROM audit_log \
             WHERE entity_type = 'user' AND action = 'custom:bootstrap' AND user_id IS NULL",
        )
        .await;
        assert_eq!(audit, 1, "ровно одна запись аудита без автора");

        ctx.shutdown.cancel();
    })
    .await
    .expect("bootstrap_route_open exceeded budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bootstrap_route_closed_after_first_user() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (router, ctx) = build_test_components().await.expect("components");
        assert!(ctx.auth.needs_bootstrap().await.expect("needs_bootstrap"));

        let (first, _) = post(
            &router,
            "/api/v1/auth_bootstrap",
            body_with("bootstrap_admin", "password123"),
        )
        .await;
        assert_eq!(first, StatusCode::OK);

        let (second, bytes) = post(
            &router,
            "/api/v1/auth_bootstrap",
            body_with("second_admin", "password123"),
        )
        .await;
        assert_eq!(
            second,
            StatusCode::CONFLICT,
            "закрытое окно: ждали 409, тело: {}",
            String::from_utf8_lossy(&bytes)
        );
        assert_eq!(admin_count(&ctx).await, 1, "второго админа быть не должно");
        assert_eq!(
            scalar_i64(&ctx, "SELECT COUNT(*) FROM users").await,
            1,
            "отказанный запрос не оставляет пользователя"
        );

        ctx.shutdown.cancel();
    })
    .await
    .expect("bootstrap_route_closed exceeded budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bootstrap_route_open_when_only_non_admins_exist() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (router, ctx) = build_test_components().await.expect("components");
        assert!(ctx.auth.needs_bootstrap().await.expect("needs_bootstrap"));

        // Предикат — «нет активного админа», а не «нуль пользователей».
        ctx.auth
            .create_user(user_new("plain_employee"), &Identity::trusted_admin())
            .await
            .expect("create employee");
        assert!(
            ctx.auth.needs_bootstrap().await.expect("needs_bootstrap"),
            "сотрудник не закрывает окно"
        );

        let (status, bytes) = post(
            &router,
            "/api/v1/auth_bootstrap",
            body_with("bootstrap_admin", "password123"),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "тело: {}",
            String::from_utf8_lossy(&bytes)
        );
        assert_eq!(admin_count(&ctx).await, 1);
        assert!(!ctx.auth.needs_bootstrap().await.expect("needs_bootstrap"));

        ctx.shutdown.cancel();
    })
    .await
    .expect("bootstrap_route_open_non_admins exceeded budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bootstrap_route_race_exactly_one_wins() {
    tokio::time::timeout(std::time::Duration::from_secs(120), async {
        let (router, ctx) = build_test_components().await.expect("components");
        assert!(ctx.auth.needs_bootstrap().await.expect("needs_bootstrap"));

        let (a, b) = tokio::join!(
            post(
                &router,
                "/api/v1/auth_bootstrap",
                body_with("racer_one", "password123")
            ),
            post(
                &router,
                "/api/v1/auth_bootstrap",
                body_with("racer_two", "password123")
            ),
        );
        let mut statuses = [a.0, b.0];
        statuses.sort_by_key(|s| s.as_u16());
        assert_eq!(
            statuses,
            [StatusCode::OK, StatusCode::CONFLICT],
            "ровно один 200 и один 409"
        );
        assert_eq!(admin_count(&ctx).await, 1, "в БД ровно один администратор");

        ctx.shutdown.cancel();
    })
    .await
    .expect("bootstrap_route_race exceeded budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bootstrap_route_rate_limited_per_peer() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (router, ctx) = build_test_components().await.expect("components");
        assert!(ctx.auth.needs_bootstrap().await.expect("needs_bootstrap"));

        // Невалидный ввод (короткий пароль) -> быстрые 400 без argon2 и без
        // создания пользователя: окно остаётся открытым, считается только governor.
        let mut statuses = Vec::new();
        for _ in 0..12 {
            let (s, _) = post(
                &router,
                "/api/v1/auth_bootstrap",
                body_with("bootstrap_admin", "short"),
            )
            .await;
            statuses.push(s);
        }
        assert!(
            statuses[..5]
                .iter()
                .all(|s| *s != StatusCode::TOO_MANY_REQUESTS),
            "первые пять (burst) не лимитируются: {statuses:?}"
        );
        assert!(
            statuses[5..].contains(&StatusCode::TOO_MANY_REQUESTS),
            "после burst обязан появиться 429: {statuses:?}"
        );

        ctx.shutdown.cancel();
    })
    .await
    .expect("bootstrap_route_rate_limited exceeded budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn users_create_still_requires_session() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (router, ctx) = build_test_components().await.expect("components");
        assert!(ctx.auth.needs_bootstrap().await.expect("needs_bootstrap"));

        let (status, bytes) = post(
            &router,
            "/api/v1/users_create",
            body_with("sneaky_admin", "password123"),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "users_create без сессии обязан давать 401 даже при needs_bootstrap, тело: {}",
            String::from_utf8_lossy(&bytes)
        );
        assert_eq!(scalar_i64(&ctx, "SELECT COUNT(*) FROM users").await, 0);

        ctx.shutdown.cancel();
    })
    .await
    .expect("users_create_still_requires_session exceeded budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bootstrap_invalid_input_rejected() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (router, ctx) = build_test_components().await.expect("components");
        assert!(ctx.auth.needs_bootstrap().await.expect("needs_bootstrap"));

        let (status, bytes) = post(
            &router,
            "/api/v1/auth_bootstrap",
            body_with("bootstrap_admin", "short"),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "тело: {}",
            String::from_utf8_lossy(&bytes)
        );
        assert_eq!(admin_count(&ctx).await, 0, "админ не создан");
        assert!(
            ctx.auth.needs_bootstrap().await.expect("needs_bootstrap"),
            "маршрут остаётся открытым после невалидного ввода"
        );

        ctx.shutdown.cancel();
    })
    .await
    .expect("bootstrap_invalid_input exceeded budget");
}

/// WR-01: публичный bootstrap обязан ТОЛЬКО вставлять — никогда не ревайвить.
///
/// `bootstrap_first_admin` делит запись строки с `create_user` через
/// `insert_user_in_tx`, а у того есть ветвь ревайва: строка с тем же логином,
/// мягко удалённая, обновляется НА МЕСТЕ с сохранением первичного ключа.
/// Для `create_user` это сделано осознанно (вызывает аутентифицированный
/// админ, FK на акты/историю остаются целыми). Для bootstrap вызывающий
/// АНОНИМЕН, и ревайв означает захват исторической личности: анонимный клиент
/// из LAN, знающий удалённый логин, получает его `id` с ролью `admin` и своим
/// паролем — вместе со всем, что на этот `id` ссылается.
///
/// Состояние достижимо без удалённого администратора (т.е. это НЕ принятый риск
/// T-41.7-09): достаточно того, что благословляет
/// `bootstrap_route_open_when_only_non_admins_exist` — есть не-админы, нет
/// активного админа.
///
/// Данные вымышленные.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bootstrap_refuses_to_revive_soft_deleted_login() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (router, ctx) = build_test_components().await.expect("components");
        assert!(ctx.auth.needs_bootstrap().await.expect("needs_bootstrap"));

        // Мягко удалённый НЕ-админ — окно bootstrap остаётся открытым.
        let ghost = ctx
            .auth
            .create_user(user_new("ghost_employee"), &Identity::trusted_admin())
            .await
            .expect("create employee");
        ctx.auth
            .delete_user(ghost.id, ghost.version, &Identity::trusted_admin())
            .await
            .expect("soft-delete employee");
        assert!(
            ctx.auth.needs_bootstrap().await.expect("needs_bootstrap"),
            "мягко удалённый сотрудник не закрывает окно"
        );

        // Анонимный POST с ИЗВЕСТНЫМ удалённым логином.
        let (status, bytes) = post(
            &router,
            "/api/v1/auth_bootstrap",
            body_with("ghost_employee", "password123"),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::CONFLICT,
            "анонимный bootstrap обязан отказать, а не ревайвить удалённый логин; тело: {}",
            String::from_utf8_lossy(&bytes)
        );

        // Старая строка не тронута: та же роль, всё ещё удалена.
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM users WHERE login = 'ghost_employee' \
                 AND deleted_at_utc IS NOT NULL AND role = 'employee' AND is_active = 1",
            )
            .await,
            1,
            "строка удалённого сотрудника обязана остаться удалённой с исходной ролью"
        );
        assert_eq!(
            admin_count(&ctx).await,
            0,
            "администратор не создан — личность не захвачена"
        );
        assert_eq!(
            scalar_i64(
                &ctx,
                "SELECT COUNT(*) FROM audit_log WHERE action = 'custom:bootstrap'",
            )
            .await,
            0,
            "отказ не должен оставлять запись bootstrap в аудите"
        );
        assert!(
            ctx.auth.needs_bootstrap().await.expect("needs_bootstrap"),
            "отказ не закрывает окно первоначальной настройки"
        );

        ctx.shutdown.cancel();
    })
    .await
    .expect("bootstrap_refuses_revive exceeded budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bootstrap_tauri_and_http_share_one_path() {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let (_router, ctx) = build_test_components().await.expect("components");
        assert!(ctx.auth.needs_bootstrap().await.expect("needs_bootstrap"));

        let created = build_auth_bootstrap_tauri(&ctx, user_new("bootstrap_admin"))
            .await
            .expect("первый вызов на чистой БД");
        assert_eq!(created.role, "admin", "Tauri-путь тоже навязывает роль");

        let again = build_auth_bootstrap_tauri(&ctx, user_new("second_admin")).await;
        assert!(
            matches!(again, Err(AppError::Conflict { .. })),
            "второй вызов обязан дать Conflict: {again:?}"
        );

        // HTTP-билдер делит тот же сервисный метод и закрыт тем же условием.
        let http = build_auth_bootstrap(
            &ctx,
            BootstrapPayload {
                user_new: user_new("third_admin"),
            },
        )
        .await;
        assert!(matches!(http, Err(AppError::Conflict { .. })));
        assert_eq!(admin_count(&ctx).await, 1);

        ctx.shutdown.cancel();
    })
    .await
    .expect("bootstrap_tauri exceeded budget");
}
