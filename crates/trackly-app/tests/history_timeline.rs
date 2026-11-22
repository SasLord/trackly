//! Лента истории (фаза 41.10, HST-05): серверная склейка `place_movements` и
//! `history_events` в `HistoryService::get_timeline`.
//!
//! События сеются прямой вставкой в `history_events` (читающая сторона). Данные
//! вымышленные (privacy gate).

use rusqlite::params;
use trackly_app::services::HistoryService;
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

fn assert_validation(r: Result<Vec<trackly_app::dto::history::HistoryEntryDto>, AppError>) {
    assert!(
        matches!(r, Err(AppError::Validation { .. })),
        "ожидали Validation"
    );
}

async fn seed_movement(
    writer: &WriterHandle,
    entity_type: &str,
    entity_id: i64,
    from_place: i64,
    to_place: i64,
    source: &str,
    created_at: i64,
) -> i64 {
    let entity_type = entity_type.to_string();
    let source = source.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO place_movements \
                 (entity_type, entity_id, from_place_id, from_place_path, to_place_id, \
                  to_place_path, source, created_at_utc) \
                 VALUES (?1, ?2, ?3, 'Здание А / Каб. 101', ?4, 'Здание Б / Склад', ?5, ?6)",
                params![
                    entity_type,
                    entity_id,
                    from_place,
                    to_place,
                    source,
                    created_at
                ],
            )
            .map_err(|e| AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed movement")
}

#[derive(Default, Clone)]
struct Ev {
    op: &'static str,
    subject_type: &'static str,
    subject_id: i64,
    kind: &'static str,
    from_group: Option<i64>,
    to_group: Option<i64>,
    status_to: Option<&'static str>,
    source: &'static str,
    movement_id: Option<i64>,
    created_at: i64,
}

async fn seed_event(writer: &WriterHandle, e: Ev) -> i64 {
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO history_events \
                 (operation_id, subject_type, subject_id, subject_label, kind, \
                  from_group_id, from_group_label, to_group_id, to_group_label, \
                  status_to_code, status_to_label, source, movement_id, created_at_utc) \
                 VALUES (?1, ?2, ?3, 'АРМ-3', ?4, ?5, 'АРМ-5', ?6, 'АРМ-5', ?7, ?7, ?8, ?9, ?10)",
                params![
                    e.op,
                    e.subject_type,
                    e.subject_id,
                    e.kind,
                    e.from_group,
                    e.to_group,
                    e.status_to,
                    e.source,
                    e.movement_id,
                    e.created_at
                ],
            )
            .map_err(|er| AppError::Internal {
                source_chain: format!("{er}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed event")
}

fn kinds(e: &trackly_app::dto::history::HistoryEntryDto) -> Vec<String> {
    e.segments.iter().map(|s| s.kind.clone()).collect()
}

#[tokio::test]
async fn history_timeline_orders_newest_first() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let m = seed_manager_caller(&writer).await;
    let a = seed_place(&writer, "Каб. 101").await;
    let b = seed_place(&writer, "Склад").await;
    seed_movement(&writer, "device", 7, a, b, "manual", 1_700_000_100).await;
    seed_event(
        &writer,
        Ev {
            op: "op-new",
            subject_type: "device",
            subject_id: 7,
            kind: "group_joined",
            to_group: Some(2),
            source: "manual",
            created_at: 1_700_000_300,
            ..Default::default()
        },
    )
    .await;
    seed_movement(&writer, "device", 7, b, a, "manual", 1_700_000_200).await;
    let t = HistoryService::new(readers)
        .get_timeline(&m, "device", 7)
        .await
        .unwrap();
    let times: Vec<i64> = t.iter().map(|e| e.created_at_utc).collect();
    assert_eq!(times, vec![1_700_000_300, 1_700_000_200, 1_700_000_100]);
}

#[tokio::test]
async fn history_timeline_joins_by_movement_id() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let m = seed_manager_caller(&writer).await;
    let a = seed_place(&writer, "Каб. 101").await;
    let b = seed_place(&writer, "Склад").await;
    let mv = seed_movement(&writer, "device", 7, a, b, "group", 1_700_000_100).await;
    seed_event(
        &writer,
        Ev {
            op: "op-1",
            subject_type: "device",
            subject_id: 7,
            kind: "status_changed",
            status_to: Some("in_repair"),
            source: "group_move",
            movement_id: Some(mv),
            created_at: 1_700_000_100,
            ..Default::default()
        },
    )
    .await;
    let t = HistoryService::new(readers)
        .get_timeline(&m, "device", 7)
        .await
        .unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!(kinds(&t[0]), vec!["move", "status"]);
    assert!(t[0].reason_is_event);
    assert_eq!(t[0].reason_source, "group_move");
}

#[tokio::test]
async fn history_timeline_joins_events_with_shared_movement_id_across_operations() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let m = seed_manager_caller(&writer).await;
    let a = seed_place(&writer, "Каб. 101").await;
    let b = seed_place(&writer, "Склад").await;
    let mv = seed_movement(&writer, "device", 7, a, b, "group", 1_700_000_100).await;
    seed_event(
        &writer,
        Ev {
            op: "op-1",
            subject_type: "device",
            subject_id: 7,
            kind: "group_transferred",
            from_group: Some(1),
            to_group: Some(2),
            source: "group_move",
            movement_id: Some(mv),
            created_at: 1_700_000_100,
            ..Default::default()
        },
    )
    .await;
    seed_event(
        &writer,
        Ev {
            op: "op-2",
            subject_type: "device",
            subject_id: 7,
            kind: "status_changed",
            status_to: Some("in_repair"),
            source: "group_move",
            movement_id: Some(mv),
            created_at: 1_700_000_100,
            ..Default::default()
        },
    )
    .await;
    let t = HistoryService::new(readers)
        .get_timeline(&m, "device", 7)
        .await
        .unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!(kinds(&t[0]), vec!["membership", "move", "status"]);
}

#[tokio::test]
async fn history_timeline_no_gluing_without_movement_id() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let m = seed_manager_caller(&writer).await;
    let a = seed_place(&writer, "Каб. 101").await;
    let b = seed_place(&writer, "Склад").await;
    seed_movement(&writer, "device", 7, a, b, "manual", 1_700_000_100).await;
    seed_event(
        &writer,
        Ev {
            op: "op-1",
            subject_type: "device",
            subject_id: 7,
            kind: "status_changed",
            status_to: Some("in_repair"),
            source: "manual",
            created_at: 1_700_000_100,
            ..Default::default()
        },
    )
    .await;
    let t = HistoryService::new(readers)
        .get_timeline(&m, "device", 7)
        .await
        .unwrap();
    assert_eq!(
        t.len(),
        2,
        "одна секунда без movement_id — разные записи (D-25)"
    );
    assert!(t.iter().all(|e| e.segments.len() == 1));
}

#[tokio::test]
async fn history_timeline_transfer_single_row() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let m = seed_manager_caller(&writer).await;
    seed_event(
        &writer,
        Ev {
            op: "op-1",
            subject_type: "device",
            subject_id: 7,
            kind: "group_transferred",
            from_group: Some(1),
            to_group: Some(2),
            source: "manual",
            created_at: 1_700_000_100,
            ..Default::default()
        },
    )
    .await;
    let t = HistoryService::new(readers)
        .get_timeline(&m, "device", 7)
        .await
        .unwrap();
    assert_eq!(t.len(), 1);
    let s = &t[0].segments[0];
    assert_eq!(s.event_kind.as_deref(), Some("group_transferred"));
    assert_eq!((s.from_group_id, s.to_group_id), (Some(1), Some(2)));
}

#[tokio::test]
async fn history_timeline_unknown_kind_degrades() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let m = seed_manager_caller(&writer).await;
    seed_event(
        &writer,
        Ev {
            op: "op-1",
            subject_type: "device",
            subject_id: 7,
            kind: "future_kind",
            source: "garbage",
            created_at: 1_700_000_100,
            ..Default::default()
        },
    )
    .await;
    let t = HistoryService::new(readers)
        .get_timeline(&m, "device", 7)
        .await
        .unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].reason_source, "garbage");
    assert_eq!(t[0].segments[0].event_kind.as_deref(), Some("future_kind"));
}

#[tokio::test]
async fn history_group_feed_membership_and_no_status() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let m = seed_manager_caller(&writer).await;
    let a = seed_place(&writer, "Каб. 101").await;
    let b = seed_place(&writer, "Склад").await;
    // образование группы 5 (subject = group)
    seed_event(
        &writer,
        Ev {
            op: "op-f",
            subject_type: "group",
            subject_id: 5,
            kind: "group_formed",
            source: "group_create",
            created_at: 1_700_000_010,
            ..Default::default()
        },
    )
    .await;
    // вход и выход участника
    seed_event(
        &writer,
        Ev {
            op: "op-j",
            subject_type: "device",
            subject_id: 7,
            kind: "group_joined",
            to_group: Some(5),
            source: "manual",
            created_at: 1_700_000_020,
            ..Default::default()
        },
    )
    .await;
    seed_event(
        &writer,
        Ev {
            op: "op-l",
            subject_type: "device",
            subject_id: 7,
            kind: "group_left",
            from_group: Some(5),
            source: "manual",
            created_at: 1_700_000_030,
            ..Default::default()
        },
    )
    .await;
    // вложение
    seed_event(
        &writer,
        Ev {
            op: "op-n",
            subject_type: "group",
            subject_id: 6,
            kind: "group_nested",
            to_group: Some(5),
            source: "manual",
            created_at: 1_700_000_040,
            ..Default::default()
        },
    )
    .await;
    // смена статуса участника — НЕ должна попасть
    seed_event(
        &writer,
        Ev {
            op: "op-s",
            subject_type: "device",
            subject_id: 7,
            kind: "status_changed",
            status_to: Some("in_repair"),
            source: "group_move",
            created_at: 1_700_000_050,
            ..Default::default()
        },
    )
    .await;
    // перемещение самой группы
    seed_movement(&writer, "group", 5, a, b, "group", 1_700_000_060).await;
    let t = HistoryService::new(readers)
        .get_timeline(&m, "group", 5)
        .await
        .unwrap();
    assert_eq!(t.len(), 5);
    assert!(t
        .iter()
        .flat_map(|e| &e.segments)
        .all(|s| s.event_kind.as_deref() != Some("status_changed")));
    assert_eq!(t[0].segments[0].kind, "move");
}

#[tokio::test]
async fn history_cartridge_only_movements() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let m = seed_manager_caller(&writer).await;
    let a = seed_place(&writer, "Каб. 101").await;
    let b = seed_place(&writer, "Склад").await;
    seed_movement(&writer, "cartridge", 3, a, b, "manual", 1_700_000_100).await;
    // событие с тем же subject_id у устройства — картриджу не принадлежит
    seed_event(
        &writer,
        Ev {
            op: "op-1",
            subject_type: "device",
            subject_id: 3,
            kind: "group_joined",
            to_group: Some(2),
            source: "manual",
            created_at: 1_700_000_200,
            ..Default::default()
        },
    )
    .await;
    let t = HistoryService::new(readers)
        .get_timeline(&m, "cartridge", 3)
        .await
        .unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!(kinds(&t[0]), vec!["move"]);
    assert!(!t[0].reason_is_event);
}

#[tokio::test]
async fn history_timeline_rejects_unknown_entity_type() {
    let (writer, readers, _dir) = test_writer_and_readers();
    let m = seed_manager_caller(&writer).await;
    assert_validation(
        HistoryService::new(readers)
            .get_timeline(&m, "foo", 1)
            .await,
    );
}

#[tokio::test]
async fn history_timeline_employee_forbidden() {
    let (_writer, readers, _dir) = test_writer_and_readers();
    let emp = Identity {
        user_id: Some(1),
        role: Role::Employee,
    };
    let r = HistoryService::new(readers)
        .get_timeline(&emp, "device", 1)
        .await;
    assert!(matches!(r, Err(AppError::Forbidden)), "{r:?}");
}
