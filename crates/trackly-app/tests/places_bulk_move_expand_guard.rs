//! Phase 41.2 Plan 05 (GAM-04, P-09, T-41.2-21): массовый перенос обязан читать
//! содержимое места в ФИЗИЧЕСКОМ режиме (`expand_groups = true`).
//!
//! Режим «группа вместо членов» (по умолчанию в UI) прячет членов групп. Если бы
//! `PlaceService::move_subtree_contents` просил свёрнутый режим, группы находились
//! бы только через скрытых членов и «Перенести всё содержимое» молча перестало бы
//! их переносить. Две защиты:
//!
//! 1. поведенческий тест — группа переезжает целиком;
//! 2. страж по исходнику — вызов в теле метода ровно один и с `true, true`.
//!
//! Страж проверен мутацией: на копии исходника в памяти (тест (c)) и на файле на
//! диске (запись в SUMMARY плана).
//!
//! Реальный `AppCtx` на временном каталоге; имена вымышленные.

use std::time::Duration;

use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::auth::UserNew;
use trackly_core::auth::{Identity, Role};
use trackly_infra::error_conversions::map_rusqlite;

/// Единственный допустимый вызов в теле `move_subtree_contents`.
const EXPECTED_CALL: &str = "list_subtree_contents(&tx, root_id, true, true)";

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

async fn manager(ctx: &AppCtx) -> Identity {
    let dto = ctx
        .auth
        .create_user(
            UserNew {
                login: "manager_expand_guard".to_string(),
                full_name: "Петров П.П.".to_string(),
                password: "password123".to_string(),
                role: "manager".to_string(),
                email: None,
            },
            &Identity::trusted_admin(),
        )
        .await
        .expect("create manager");
    Identity {
        user_id: Some(dto.id),
        role: Role::Manager,
    }
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

async fn workstation_type(ctx: &AppCtx) -> i64 {
    ctx.group_types
        .list_types(&Identity::trusted_admin(), false)
        .await
        .expect("list_types")
        .into_iter()
        .find(|t| t.code == "workstation")
        .expect("тип workstation")
        .id
}

async fn seed_group(ctx: &AppCtx, type_id: i64, place: i64) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, created_at_utc, \
                 updated_at_utc, version) VALUES (?1, 'АРМ #9', 9, ?2, 1700000000, 1700000000, 1)",
                params![type_id, place],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group")
}

async fn seed_member(ctx: &AppCtx, type_id: i64, name: &str, place: i64, group: i64) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, place_id, status_id, version, \
                 created_at_utc, updated_at_utc) VALUES (?1, ?2, ?3, 1, 1, 1700000000, 1700000000)",
                params![type_id, name, place],
            )
            .map_err(map_rusqlite)?;
            let id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
                 VALUES (?1, ?2, 1700000000)",
                params![id, group],
            )
            .map_err(map_rusqlite)?;
            Ok(id)
        })
        .await
        .expect("seed member")
}

async fn scalar(ctx: &AppCtx, sql: &'static str, p: Vec<i64>) -> i64 {
    ctx.writer
        .execute(move |conn| {
            conn.query_row(sql, rusqlite::params_from_iter(p), |r| r.get::<_, i64>(0))
                .map_err(map_rusqlite)
        })
        .await
        .expect("scalar query")
}

// ---------------------------------------------------------------------------
// (a) поведение: при скрытом режиме по умолчанию группа всё равно едет целиком
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bulk_move_still_moves_the_whole_group_although_collapsed_mode_hides_members() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (ctx, _dir) = make_test_ctx().await;
        let mgr = manager(&ctx).await;
        let arm = workstation_type(&ctx).await;

        let from = seed_place(&ctx, "Склад А").await;
        let to = seed_place(&ctx, "Склад Б").await;
        let group = seed_group(&ctx, arm, from).await;
        let block = seed_member(&ctx, 1, "Системный блок", from, group).await;
        let printer = seed_member(&ctx, 2, "Принтер", from, group).await;

        // Предпосылка расхождения фикстуры: свёрнутый режим ДЕЙСТВИТЕЛЬНО прячет
        // обоих членов (иначе тест вакуумен — перенос работал бы и при `false`).
        let collapsed = ctx
            .places
            .list_subtree_contents(&Identity::trusted_admin(), from, true, false)
            .await
            .expect("collapsed listing");
        assert!(
            collapsed
                .iter()
                .all(|r| !matches!(r.kind.as_str(), "device" | "printer")),
            "в свёрнутом режиме члены группы скрыты"
        );
        assert_eq!(collapsed.iter().filter(|r| r.kind == "group").count(), 1);

        ctx.places
            .move_subtree_contents(&mgr, from, to, None)
            .await
            .expect("bulk move");

        assert_eq!(
            scalar(
                &ctx,
                "SELECT place_id FROM groups WHERE id = ?1",
                vec![group]
            )
            .await,
            to,
            "место группы изменилось"
        );
        for d in [block, printer] {
            assert_eq!(
                scalar(&ctx, "SELECT place_id FROM devices WHERE id = ?1", vec![d]).await,
                to,
                "член {d} переехал вместе с группой"
            );
            assert_eq!(
                scalar(
                    &ctx,
                    "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'device' \
                     AND entity_id = ?1 AND source = 'group' AND batch_id IS NOT NULL",
                    vec![d]
                )
                .await,
                1,
                "журнал: одна строка source='group' в пакете"
            );
        }
        assert_eq!(
            scalar(
                &ctx,
                "SELECT COUNT(DISTINCT batch_id) FROM place_movements WHERE batch_id IS NOT NULL",
                vec![]
            )
            .await,
            1,
            "один пакет на корневую группу"
        );
    })
    .await
    .expect("test exceeded 60 s budget");
}

// ---------------------------------------------------------------------------
// (b) страж по исходнику
// ---------------------------------------------------------------------------

/// Область тела метода: от `start` до первого `end` после него.
fn body_between<'a>(src: &'a str, start: &str, end: &str) -> &'a str {
    let s = src
        .find(start)
        .unwrap_or_else(|| panic!("не найдено начало {start:?}"));
    let rest = &src[s..];
    let e = rest
        .find(end)
        .unwrap_or_else(|| panic!("не найден конец {end:?}"));
    &rest[..e]
}

/// Тело `move_subtree_contents` внутри исходника сервиса.
fn move_body(src: &str) -> &str {
    body_between(
        src,
        "pub async fn move_subtree_contents(",
        "/// Cyrillic-safe full-path substring search",
    )
}

/// Страж: в теле ровно один вызов `list_subtree_contents` и он равен ожидаемому.
/// Возвращает `Err` с причиной (а не паникует), чтобы самопроверка могла его
/// проверить на мутированной копии.
fn check_guard(src: &str) -> Result<(), String> {
    let body = move_body(src);
    let any_calls = body.matches(".list_subtree_contents(").count();
    if any_calls != 1 {
        return Err(format!(
            "в move_subtree_contents ожидался ровно один вызов list_subtree_contents, найдено {any_calls}"
        ));
    }
    let exact = body.matches(EXPECTED_CALL).count();
    if exact != 1 {
        return Err(format!(
            "вызов должен быть {EXPECTED_CALL:?} (физический режим), найдено совпадений: {exact}"
        ));
    }
    Ok(())
}

const PLACE_SERVICE_SRC: &str = include_str!("../src/services/place_service.rs");

#[test]
fn bulk_move_requests_the_physical_listing() {
    if let Err(why) = check_guard(PLACE_SERVICE_SRC) {
        panic!("{why}");
    }
}

// ---------------------------------------------------------------------------
// (c) самопроверка стража на мутированной копии в памяти
// ---------------------------------------------------------------------------

#[test]
fn guard_selfcheck_detects_the_collapsed_mode_mutation() {
    // Якорь обязан быть уникальным: иначе `replace(.., 1)` мог бы тронуть соседа.
    assert_eq!(
        PLACE_SERVICE_SRC.matches(EXPECTED_CALL).count(),
        1,
        "якорь мутации неуникален во всём файле"
    );
    assert_eq!(
        move_body(PLACE_SERVICE_SRC).matches(EXPECTED_CALL).count(),
        1
    );

    assert!(
        check_guard(PLACE_SERVICE_SRC).is_ok(),
        "чистая копия обязана быть зелёной"
    );

    let mutated = PLACE_SERVICE_SRC.replace(
        EXPECTED_CALL,
        "list_subtree_contents(&tx, root_id, true, false)",
    );
    assert_ne!(mutated, PLACE_SERVICE_SRC, "мутация ничего не изменила");
    assert!(
        check_guard(&mutated).is_err(),
        "страж не заметил переход на свёрнутый режим"
    );

    // Второй вид нарушения: добавлен ещё один вызов (например, для подсчёта).
    let doubled = PLACE_SERVICE_SRC.replace(
        EXPECTED_CALL,
        &format!("{EXPECTED_CALL}?;\n let _extra = places_repo.list_subtree_contents(&tx, root_id, true, false)"),
    );
    assert!(
        check_guard(&doubled).is_err(),
        "страж не заметил второй вызов"
    );
}
