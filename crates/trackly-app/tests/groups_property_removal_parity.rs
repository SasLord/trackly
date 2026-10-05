//! Phase 41 Plan 31 (GAP-2): серверная сторона golden-фикстуры
//! `ui/scripts/fixtures/property-removal/cases.json`.
//!
//! JS-зеркало `propertyRemovalCopy` решает «скрыть или удалить» по
//! `filled_group_count > 0`; здесь то же решение сверяется с настоящим
//! `delete_property`: `archived == (expected.kind == "hide")`. Фикстуру читаем с
//! диска — кейсы в Rust не дублируются. Данные вымышленные.

use rusqlite::params;

use trackly_app::context::AppCtx;
use trackly_app::dto::group_types::{GroupTypeCreateDto, PropertyCreateDto, PropertyUpdateDto};
use trackly_core::auth::Identity;
use trackly_infra::error_conversions::map_rusqlite;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../ui/scripts/fixtures/property-removal/cases.json"
);

const MIN_CASES: usize = 6;

async fn build_ctx_in(dir: &std::path::Path) -> anyhow::Result<AppCtx> {
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.to_path_buf())?;
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    })?;
    AppCtx::build(paths, config, log_guard).await
}

fn admin() -> Identity {
    Identity::trusted_admin()
}

/// Сеет живую группу типа `type_id` с порядковым номером `seq`.
async fn seed_group(ctx: &AppCtx, type_id: i64, name: &str, seq: i64) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, created_at_utc, updated_at_utc, version)
                 VALUES (?1, ?2, ?3, 1700000000, 1700000000, 1)",
                params![type_id, name, seq],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group")
}

async fn seed_value(ctx: &AppCtx, group_id: i64, property_id: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO group_property_values (group_id, property_id, value_text, updated_at_utc)
                 VALUES (?1, ?2, 'значение', 1700000000)",
                params![group_id, property_id],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("seed value");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn property_removal_matches_golden_fixture() {
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(&std::fs::read_to_string(FIXTURE).expect("фикстура читается"))
            .expect("фикстура — JSON-массив");

    let dir = tempfile::TempDir::new().expect("tempdir");
    let ctx = build_ctx_in(dir.path()).await.expect("build ctx");
    let mut checked = 0usize;
    for (i, c) in cases.iter().enumerate() {
        let filled = c["filled_group_count"]
            .as_i64()
            .expect("filled_group_count");
        let required = c["is_required"].as_bool().expect("is_required");
        let kind = c["expected"]["kind"].as_str().expect("expected.kind");
        let want_archived = match kind {
            "hide" => true,
            "delete" => false,
            other => panic!("неизвестный kind «{other}» в фикстуре"),
        };

        // Каждый кейс — свой пользовательский тип: группы прошлых кейсов не
        // влияют ни на счёт заполненности, ни на проверку обязательности.
        let t = ctx
            .group_types
            .create_type(
                &admin(),
                GroupTypeCreateDto {
                    name: format!("Стойка {i}"),
                    behavior: "container".to_string(),
                },
            )
            .await
            .expect("create_type");
        let prop = ctx
            .group_types
            .create_property(
                &admin(),
                PropertyCreateDto {
                    type_id: t.id,
                    name: "Хост".to_string(),
                    data_type: "text".to_string(),
                    is_required: false,
                    show_on_map: false,
                },
            )
            .await
            .expect("create_property");

        // «Заполнено в N группах»: N групп типа, в каждой значение свойства.
        for g in 0..filled {
            let gid = seed_group(&ctx, t.id, &format!("Стойка {i}-{g}"), g + 1).await;
            seed_value(&ctx, gid, prop.id).await;
        }

        if required {
            // Все группы типа заполнены — сервер разрешит флаг (при N=0 групп нет).
            ctx.group_types
                .update_property(
                    &admin(),
                    prop.id,
                    prop.version,
                    PropertyUpdateDto {
                        name: None,
                        data_type: None,
                        is_required: Some(true),
                        show_on_map: None,
                    },
                )
                .await
                .expect("включить обязательность");
        }

        let out = ctx
            .group_types
            .delete_property(&admin(), prop.id)
            .await
            .expect("delete_property");
        assert_eq!(
            out.archived, want_archived,
            "кейс {i} (filled={filled}, required={required}): сервер archived={}, фикстура ждёт kind={kind}",
            out.archived
        );
        checked += 1;
    }

    assert!(
        checked >= MIN_CASES,
        "прошло {checked} кейсов, ожидалось не меньше {MIN_CASES} — фикстуру урезали?"
    );
}
