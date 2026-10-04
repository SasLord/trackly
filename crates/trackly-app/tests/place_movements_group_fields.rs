//! Phase 41 Plan 05 (GRP-06, D-26/D-28/D-29): поля пакета и группы в DTO таймлайна.
//!
//! Строки журнала сеются прямым SQL (колонки V046 batch_id / entity_label / group_id);
//! проверяется чтение через `PlaceMovementService::get_timeline`. Только вымышленные
//! данные («АРМ #3», «Склад А», «Склад Б», «Петров П.П.»).

use rusqlite::params;
use trackly_app::services::PlaceMovementService;
use trackly_core::auth::{Identity, Role};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::writer_worker::WriterHandle;
use trackly_infra::test_support::test_writer_and_readers;

/// Сеет реальную строку `users` (FK-цель для `place_movements.user_id`) и
/// возвращает `Identity` менеджера. Вымышленное имя — privacy gate (CLAUDE.md).
async fn seed_manager_caller(writer: &WriterHandle) -> Identity {
    let now = SystemClock.unix_seconds();
    let user_id = writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users \
                 (login, full_name, password_hash, role, ad_user, is_active, \
                  created_at_utc, updated_at_utc, version) \
                 VALUES ('petrov.pp', 'Петров П.П.', NULL, 'manager', 0, 1, ?1, ?1, 1)",
                params![now],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed manager user");
    Identity {
        user_id: Some(user_id),
        role: Role::Manager,
    }
}

/// Сеет строку `places` напрямую. Вымышленное название.
async fn seed_place(writer: &WriterHandle, name: &str) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, is_storage, created_at_utc, updated_at_utc, version) \
                 VALUES ('room', ?1, 0, ?2, ?2, 1)",
                params![name, 1_700_000_000_i64],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

/// Сеет строку `devices` с заданным `type_id` (1 = "Устройство", 2 = "Принтер" —
/// V001__init_pragmas_and_lookups.sql). Вымышленное название.
async fn seed_device(writer: &WriterHandle, name: &str, type_id: i64, place_id: i64) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, status_id, place_id, version, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, 1, ?3, 1, ?4, ?4)",
                params![type_id, name, place_id, 1_700_000_000_i64],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
}

/// Сеет строку `place_movements` с колонками пакета V046.
#[allow(clippy::too_many_arguments)]
async fn seed_group_row(
    writer: &WriterHandle,
    entity_type: &str,
    entity_id: i64,
    from_place_id: i64,
    to_place_id: i64,
    source: &str,
    batch_id: Option<&str>,
    entity_label: Option<&str>,
    group_id: Option<i64>,
    created_at_utc: i64,
) {
    let entity_type = entity_type.to_string();
    let source = source.to_string();
    let batch_id = batch_id.map(str::to_string);
    let entity_label = entity_label.map(str::to_string);
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO place_movements \
                 (entity_type, entity_id, from_place_id, from_place_path, to_place_id, \
                  to_place_path, source, note, act_id, user_id, actor_name_snapshot, \
                  batch_id, entity_label, group_id, created_at_utc) \
                 VALUES (?1, ?2, ?3, 'Склад А', ?4, 'Склад Б', ?5, NULL, NULL, NULL, NULL, \
                         ?6, ?7, ?8, ?9)",
                params![
                    entity_type,
                    entity_id,
                    from_place_id,
                    to_place_id,
                    source,
                    batch_id,
                    entity_label,
                    group_id,
                    created_at_utc,
                ],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(())
        })
        .await
        .expect("seed group movement row");
}

const BATCH: &str = "6f1c2a9e-8d3b-4e47-a5c1-2b9f0d7e4a63";
const LABEL: &str = "АРМ #3";
const GROUP_ID: i64 = 303;

#[tokio::test]
async fn device_row_in_batch_carries_batch_group_and_label() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let manager = seed_manager_caller(&writer).await;
    let a = seed_place(&writer, "Склад А").await;
    let b = seed_place(&writer, "Склад Б").await;
    let dev = seed_device(&writer, "Системный блок инв.301", 1, b).await;
    // Пакет: строка устройства + строка группы, один batch_id.
    seed_group_row(
        &writer,
        "device",
        dev,
        a,
        b,
        "group",
        Some(BATCH),
        Some(LABEL),
        Some(GROUP_ID),
        1_700_000_100,
    )
    .await;
    seed_group_row(
        &writer,
        "group",
        GROUP_ID,
        a,
        b,
        "group",
        Some(BATCH),
        Some(LABEL),
        Some(GROUP_ID),
        1_700_000_100,
    )
    .await;

    let svc = PlaceMovementService::new(readers);
    let tl = svc
        .get_timeline(&manager, "device", dev)
        .await
        .expect("timeline");
    assert_eq!(tl.len(), 1);
    assert_eq!(tl[0].batch_id.as_deref(), Some(BATCH));
    assert_eq!(tl[0].group_id, Some(GROUP_ID));
    assert_eq!(tl[0].group_label.as_deref(), Some(LABEL));
    assert_eq!(tl[0].source, "group");
}

#[tokio::test]
async fn batch_without_group_row_still_has_group_id_and_label() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let manager = seed_manager_caller(&writer).await;
    let a = seed_place(&writer, "Склад А").await;
    let b = seed_place(&writer, "Склад Б").await;
    let dev = seed_device(&writer, "Монитор инв.302", 1, b).await;
    // Первое размещение группы: строки группы в журнале нет, у устройства — есть.
    seed_group_row(
        &writer,
        "device",
        dev,
        a,
        b,
        "group",
        Some(BATCH),
        Some(LABEL),
        Some(GROUP_ID),
        1_700_000_200,
    )
    .await;

    let svc = PlaceMovementService::new(readers);
    let tl = svc
        .get_timeline(&manager, "device", dev)
        .await
        .expect("timeline");
    assert_eq!(tl.len(), 1);
    assert_eq!(tl[0].batch_id.as_deref(), Some(BATCH));
    assert_eq!(
        tl[0].group_id,
        Some(GROUP_ID),
        "group_id приходит из колонки, а не из строки группы"
    );
    assert_eq!(tl[0].group_label.as_deref(), Some(LABEL));

    let group_tl = svc
        .get_timeline(&manager, "group", GROUP_ID)
        .await
        .expect("group timeline");
    assert!(
        group_tl.is_empty(),
        "строки группы нет — таймлайн группы пуст"
    );
}

#[tokio::test]
async fn add_devices_row_has_group_without_batch() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let manager = seed_manager_caller(&writer).await;
    let a = seed_place(&writer, "Склад А").await;
    let b = seed_place(&writer, "Склад Б").await;
    let dev = seed_device(&writer, "Клавиатура инв.303", 1, b).await;
    seed_group_row(
        &writer,
        "device",
        dev,
        a,
        b,
        "group",
        None,
        Some(LABEL),
        Some(GROUP_ID),
        1_700_000_300,
    )
    .await;

    let svc = PlaceMovementService::new(readers);
    let tl = svc
        .get_timeline(&manager, "device", dev)
        .await
        .expect("timeline");
    assert_eq!(tl.len(), 1);
    assert_eq!(tl[0].batch_id, None);
    assert_eq!(tl[0].group_id, Some(GROUP_ID));
    assert_eq!(tl[0].group_label.as_deref(), Some(LABEL));
}

#[tokio::test]
async fn manual_row_has_no_batch_or_group_fields() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let manager = seed_manager_caller(&writer).await;
    let a = seed_place(&writer, "Склад А").await;
    let b = seed_place(&writer, "Склад Б").await;
    let dev = seed_device(&writer, "Мышь инв.304", 1, b).await;
    seed_group_row(
        &writer,
        "device",
        dev,
        a,
        b,
        "manual",
        None,
        None,
        None,
        1_700_000_400,
    )
    .await;

    let svc = PlaceMovementService::new(readers);
    let tl = svc
        .get_timeline(&manager, "device", dev)
        .await
        .expect("timeline");
    assert_eq!(tl.len(), 1);
    assert_eq!(tl[0].batch_id, None);
    assert_eq!(tl[0].group_id, None);
    assert_eq!(tl[0].group_label, None);
}

#[tokio::test]
async fn group_timeline_returns_group_rows_and_employee_is_forbidden() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let manager = seed_manager_caller(&writer).await;
    let a = seed_place(&writer, "Склад А").await;
    let b = seed_place(&writer, "Склад Б").await;
    seed_group_row(
        &writer,
        "group",
        GROUP_ID,
        a,
        b,
        "group",
        Some(BATCH),
        Some(LABEL),
        Some(GROUP_ID),
        1_700_000_500,
    )
    .await;

    let svc = PlaceMovementService::new(readers);
    let tl = svc
        .get_timeline(&manager, "group", GROUP_ID)
        .await
        .expect("group timeline");
    assert_eq!(tl.len(), 1);
    assert_eq!(tl[0].entity_type, "group");
    assert_eq!(tl[0].group_id, Some(GROUP_ID));

    let employee = Identity {
        user_id: None,
        role: Role::Employee,
    };
    let err = svc
        .get_timeline(&employee, "group", GROUP_ID)
        .await
        .expect_err("employee denied");
    assert!(matches!(err, AppError::Forbidden), "got {err:?}");
}
