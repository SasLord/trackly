//! Единый записывающий модуль событий истории (фаза 41.10, HST-05/HST-06).
//!
//! Модуль не владеет писателем (WriterHandle) и не открывает транзакций: каждая функция
//! `record_*_in_tx` работает на транзакции вызывающего писателя (D-01). Снимки подписей
//! (имя устройства, подпись статуса, ФИО автора) читаются мягко (`.ok()`): сбой снимка
//! никогда не роняет мутацию (D-06, T-41.10-08).
//!
//! Контракт порядка (D-25): сначала `record_*_movement_*` (возвращает id перемещения),
//! затем событие истории с `cause.movement_id`.

use rusqlite::{params, OptionalExtension, Transaction};
use trackly_core::domain::history_events::{HistoryEventKind, HistoryReason};
use trackly_core::error::AppError;
use trackly_infra::repos::history_events_sqlite::{NewHistoryEvent, SqliteHistoryEventsRepository};

/// Общая причина всех событий одной операции (D-04).
#[derive(Debug, Clone, Copy)]
pub(crate) struct HistoryCause<'a> {
    pub operation_id: &'a str,
    pub source: HistoryReason,
    pub act_id: Option<i64>,
    pub request_id: Option<i64>,
    /// Группа-основание: «при переносе группы», «при списании группы».
    pub ref_group: Option<(i64, &'a str)>,
    pub movement_id: Option<i64>,
    pub user_id: Option<i64>,
    pub now: i64,
}

/// Субъект события формирования/расформирования группы (D-07).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HistorySubject {
    Device { id: i64 },
    Group { id: i64 },
}

/// Новый идентификатор операции (D-04).
pub(crate) fn new_operation_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn device_name(tx: &Transaction<'_>, device_id: i64) -> Option<String> {
    tx.query_row(
        "SELECT name FROM devices WHERE id = ?1",
        params![device_id],
        |r| r.get::<_, String>(0),
    )
    .ok()
}

fn actor_name(tx: &Transaction<'_>, user_id: Option<i64>) -> Option<String> {
    user_id.and_then(|uid| {
        tx.query_row(
            "SELECT full_name FROM users WHERE id = ?1",
            params![uid],
            |r| r.get::<_, String>(0),
        )
        .ok()
    })
}

/// (code, name) статуса; мягко.
fn status_snapshot(tx: &Transaction<'_>, status_id: i64) -> (Option<String>, Option<String>) {
    tx.query_row(
        "SELECT code, name FROM device_statuses WHERE id = ?1",
        params![status_id],
        |r| {
            Ok((
                r.get::<_, Option<String>>(0)?,
                r.get::<_, Option<String>>(1)?,
            ))
        },
    )
    .optional()
    .ok()
    .flatten()
    .unwrap_or((None, None))
}

/// Общая вставка: заполняет причину и автора, остальное задаёт вызывающий.
#[allow(clippy::too_many_arguments)]
fn insert<'a>(
    tx: &Transaction<'_>,
    cause: &HistoryCause<'a>,
    actor: Option<&'a str>,
    kind: HistoryEventKind,
    subject_type: &'a str,
    subject_id: i64,
    subject_label: Option<&'a str>,
    fill: impl FnOnce(&mut NewHistoryEvent<'a>),
) -> Result<(), AppError> {
    let mut e = NewHistoryEvent {
        operation_id: cause.operation_id,
        subject_type,
        subject_id,
        subject_label,
        kind: kind.as_str(),
        source: cause.source.as_str(),
        act_id: cause.act_id,
        request_id: cause.request_id,
        ref_group_id: cause.ref_group.map(|(id, _)| id),
        ref_group_label: cause.ref_group.map(|(_, l)| l),
        movement_id: cause.movement_id,
        user_id: cause.user_id,
        actor_name_snapshot: actor,
        created_at_utc: cause.now,
        ..Default::default()
    };
    fill(&mut e);
    SqliteHistoryEventsRepository.insert_in_tx(tx, e)?;
    Ok(())
}

/// Смена статуса устройства. При `before == after` ничего не пишет (D-06).
pub(crate) fn record_status_change_in_tx(
    tx: &Transaction<'_>,
    device_id: i64,
    before_status_id: i64,
    after_status_id: i64,
    cause: &HistoryCause<'_>,
) -> Result<(), AppError> {
    if before_status_id == after_status_id {
        return Ok(());
    }
    let name = device_name(tx, device_id);
    let (from_code, from_label) = status_snapshot(tx, before_status_id);
    let (to_code, to_label) = status_snapshot(tx, after_status_id);
    let actor = actor_name(tx, cause.user_id);
    insert(
        tx,
        cause,
        actor.as_deref(),
        HistoryEventKind::StatusChanged,
        "device",
        device_id,
        name.as_deref(),
        |e| {
            e.status_from_code = from_code.as_deref();
            e.status_from_label = from_label.as_deref();
            e.status_to_code = to_code.as_deref();
            e.status_to_label = to_label.as_deref();
        },
    )
}

/// Устройство вошло в группу (D-08, сторона устройства).
pub(crate) fn record_group_joined_in_tx(
    tx: &Transaction<'_>,
    device_id: i64,
    group_id: i64,
    group_label: &str,
    cause: &HistoryCause<'_>,
) -> Result<(), AppError> {
    let name = device_name(tx, device_id);
    let actor = actor_name(tx, cause.user_id);
    insert(
        tx,
        cause,
        actor.as_deref(),
        HistoryEventKind::GroupJoined,
        "device",
        device_id,
        name.as_deref(),
        |e| {
            e.to_group_id = Some(group_id);
            e.to_group_label = Some(group_label);
        },
    )
}

/// Устройство вышло из группы (D-08, сторона устройства).
pub(crate) fn record_group_left_in_tx(
    tx: &Transaction<'_>,
    device_id: i64,
    group_id: i64,
    group_label: &str,
    cause: &HistoryCause<'_>,
) -> Result<(), AppError> {
    let name = device_name(tx, device_id);
    let actor = actor_name(tx, cause.user_id);
    insert(
        tx,
        cause,
        actor.as_deref(),
        HistoryEventKind::GroupLeft,
        "device",
        device_id,
        name.as_deref(),
        |e| {
            e.from_group_id = Some(group_id);
            e.from_group_label = Some(group_label);
        },
    )
}

/// Перенос устройства из группы в группу: одна строка (D-05).
pub(crate) fn record_group_transferred_in_tx(
    tx: &Transaction<'_>,
    device_id: i64,
    from_group_id: i64,
    from_label: &str,
    to_group_id: i64,
    to_label: &str,
    cause: &HistoryCause<'_>,
) -> Result<(), AppError> {
    let name = device_name(tx, device_id);
    let actor = actor_name(tx, cause.user_id);
    insert(
        tx,
        cause,
        actor.as_deref(),
        HistoryEventKind::GroupTransferred,
        "device",
        device_id,
        name.as_deref(),
        |e| {
            e.from_group_id = Some(from_group_id);
            e.from_group_label = Some(from_label);
            e.to_group_id = Some(to_group_id);
            e.to_group_label = Some(to_label);
        },
    )
}

fn subject_parts(
    tx: &Transaction<'_>,
    subject: HistorySubject,
    group_label: &str,
) -> (&'static str, i64, Option<String>) {
    match subject {
        HistorySubject::Device { id } => ("device", id, device_name(tx, id)),
        HistorySubject::Group { id } => ("group", id, Some(group_label.to_string())),
    }
}

/// Группа сформирована (D-07): субъект — якорь-устройство или сама группа.
pub(crate) fn record_group_formed_in_tx(
    tx: &Transaction<'_>,
    subject: HistorySubject,
    group_id: i64,
    group_label: &str,
    cause: &HistoryCause<'_>,
) -> Result<(), AppError> {
    let (st, sid, label) = subject_parts(tx, subject, group_label);
    let actor = actor_name(tx, cause.user_id);
    insert(
        tx,
        cause,
        actor.as_deref(),
        HistoryEventKind::GroupFormed,
        st,
        sid,
        label.as_deref(),
        |e| {
            e.to_group_id = Some(group_id);
            e.to_group_label = Some(group_label);
        },
    )
}

/// Группа расформирована (D-07).
pub(crate) fn record_group_disbanded_in_tx(
    tx: &Transaction<'_>,
    subject: HistorySubject,
    group_id: i64,
    group_label: &str,
    cause: &HistoryCause<'_>,
) -> Result<(), AppError> {
    let (st, sid, label) = subject_parts(tx, subject, group_label);
    let actor = actor_name(tx, cause.user_id);
    insert(
        tx,
        cause,
        actor.as_deref(),
        HistoryEventKind::GroupDisbanded,
        st,
        sid,
        label.as_deref(),
        |e| {
            e.from_group_id = Some(group_id);
            e.from_group_label = Some(group_label);
        },
    )
}

/// Группа вложена в родителя (D-09, субъект — дочерняя группа).
pub(crate) fn record_group_nested_in_tx(
    tx: &Transaction<'_>,
    child_group_id: i64,
    child_label: &str,
    parent_group_id: i64,
    parent_label: &str,
    cause: &HistoryCause<'_>,
) -> Result<(), AppError> {
    let actor = actor_name(tx, cause.user_id);
    insert(
        tx,
        cause,
        actor.as_deref(),
        HistoryEventKind::GroupNested,
        "group",
        child_group_id,
        Some(child_label),
        |e| {
            e.to_group_id = Some(parent_group_id);
            e.to_group_label = Some(parent_label);
        },
    )
}

/// Группа извлечена из родителя (D-09).
pub(crate) fn record_group_unnested_in_tx(
    tx: &Transaction<'_>,
    child_group_id: i64,
    child_label: &str,
    parent_group_id: i64,
    parent_label: &str,
    cause: &HistoryCause<'_>,
) -> Result<(), AppError> {
    let actor = actor_name(tx, cause.user_id);
    insert(
        tx,
        cause,
        actor.as_deref(),
        HistoryEventKind::GroupUnnested,
        "group",
        child_group_id,
        Some(child_label),
        |e| {
            e.from_group_id = Some(parent_group_id);
            e.from_group_label = Some(parent_label);
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::group_act_plan::test_support::{fresh_db, NOW};
    use rusqlite::Connection;

    fn cause<'a>(op: &'a str, user: Option<i64>) -> HistoryCause<'a> {
        HistoryCause {
            operation_id: op,
            source: HistoryReason::Manual,
            act_id: Some(7),
            request_id: Some(9),
            ref_group: Some((3, "АРМ-5")),
            movement_id: Some(42),
            user_id: user,
            now: NOW,
        }
    }

    fn count(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM history_events", [], |r| r.get(0))
            .unwrap()
    }

    fn seed_device(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO devices (type_id, name, status_id, created_at_utc, updated_at_utc) \
             VALUES (1, 'Монитор-1', 1, ?1, ?1)",
            params![NOW],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn seed_user(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO users (login, full_name, role, created_at_utc, updated_at_utc) \
             VALUES ('ivanov', 'Иванов И.И.', 'admin', ?1, ?1)",
            params![NOW],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    type Row = (
        String,
        String,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    );

    fn last_row(conn: &Connection) -> Row {
        conn.query_row(
            "SELECT kind, subject_type, subject_id, subject_label, \
                    status_to_label, actor_name_snapshot, source \
             FROM history_events ORDER BY id DESC LIMIT 1",
            [],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        )
        .unwrap()
    }

    #[test]
    fn status_change_same_status_writes_nothing() {
        let (mut conn, _d) = fresh_db();
        let dev = seed_device(&conn);
        let tx = conn.transaction().unwrap();
        record_status_change_in_tx(&tx, dev, 2, 2, &cause("op", None)).unwrap();
        tx.commit().unwrap();
        assert_eq!(count(&conn), 0);
    }

    #[test]
    #[allow(clippy::type_complexity)]
    fn status_change_writes_snapshot_and_cause() {
        let (mut conn, _d) = fresh_db();
        let dev = seed_device(&conn);
        let uid = seed_user(&conn);
        let tx = conn.transaction().unwrap();
        record_status_change_in_tx(&tx, dev, 1, 2, &cause("op-1", Some(uid))).unwrap();
        tx.commit().unwrap();
        assert_eq!(count(&conn), 1);
        let (kind, st, sid, label, to_label, actor, source) = last_row(&conn);
        assert_eq!(kind, "status_changed");
        assert_eq!((st.as_str(), sid), ("device", dev));
        assert_eq!(label.as_deref(), Some("Монитор-1"));
        assert_eq!(to_label.as_deref(), Some("В работе"));
        assert_eq!(actor.as_deref(), Some("Иванов И.И."));
        assert_eq!(source.as_deref(), Some("manual"));
        let (code, from_code, act, req, rg, mv): (
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
        ) = conn
            .query_row(
                "SELECT status_to_code, status_from_code, act_id, request_id, \
                        ref_group_id, movement_id FROM history_events",
                [],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(code.as_deref(), Some("в_работе"));
        assert_eq!(from_code.as_deref(), Some("на_складе"));
        assert_eq!((act, req, rg, mv), (Some(7), Some(9), Some(3), Some(42)));
    }

    #[test]
    fn missing_user_and_status_do_not_fail() {
        let (mut conn, _d) = fresh_db();
        let tx = conn.transaction().unwrap();
        record_status_change_in_tx(&tx, 999, 1, 888, &cause("op", Some(12345))).unwrap();
        tx.commit().unwrap();
        let (_, _, _, label, to_label, actor, _) = last_row(&conn);
        assert_eq!((label, to_label, actor), (None, None, None));
    }

    #[test]
    fn transferred_writes_exactly_one_row_with_both_groups() {
        let (mut conn, _d) = fresh_db();
        let dev = seed_device(&conn);
        let tx = conn.transaction().unwrap();
        record_group_transferred_in_tx(&tx, dev, 1, "АРМ-5", 2, "АРМ-6", &cause("op", None))
            .unwrap();
        tx.commit().unwrap();
        assert_eq!(count(&conn), 1);
        let (f, fl, t, tl): (i64, String, i64, String) = conn
            .query_row(
                "SELECT from_group_id, from_group_label, to_group_id, to_group_label \
                 FROM history_events WHERE kind = 'group_transferred'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!((f, fl.as_str(), t, tl.as_str()), (1, "АРМ-5", 2, "АРМ-6"));
    }

    #[test]
    fn joined_and_left_are_device_side() {
        let (mut conn, _d) = fresh_db();
        let dev = seed_device(&conn);
        let tx = conn.transaction().unwrap();
        record_group_joined_in_tx(&tx, dev, 1, "АРМ-5", &cause("op", None)).unwrap();
        let (k, st, ..) = {
            tx.commit().unwrap();
            last_row(&conn)
        };
        assert_eq!((k.as_str(), st.as_str()), ("group_joined", "device"));
        let tx = conn.transaction().unwrap();
        record_group_left_in_tx(&tx, dev, 1, "АРМ-5", &cause("op", None)).unwrap();
        tx.commit().unwrap();
        let (k, st, ..) = last_row(&conn);
        assert_eq!((k.as_str(), st.as_str()), ("group_left", "device"));
    }

    #[test]
    fn formed_subject_device_or_group() {
        let (mut conn, _d) = fresh_db();
        let dev = seed_device(&conn);
        let tx = conn.transaction().unwrap();
        record_group_formed_in_tx(
            &tx,
            HistorySubject::Device { id: dev },
            1,
            "АРМ-5",
            &cause("op", None),
        )
        .unwrap();
        record_group_formed_in_tx(
            &tx,
            HistorySubject::Group { id: 2 },
            2,
            "АРМ-6",
            &cause("op", None),
        )
        .unwrap();
        record_group_disbanded_in_tx(
            &tx,
            HistorySubject::Group { id: 2 },
            2,
            "АРМ-6",
            &cause("op", None),
        )
        .unwrap();
        tx.commit().unwrap();
        let rows: Vec<(String, String, Option<String>)> = conn
            .prepare("SELECT kind, subject_type, subject_label FROM history_events ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(rows[0].1, "device");
        assert_eq!(rows[0].2.as_deref(), Some("Монитор-1"));
        assert_eq!(rows[1].1, "group");
        assert_eq!(rows[1].2.as_deref(), Some("АРМ-6"));
        assert_eq!(rows[2].0, "group_disbanded");
        assert_eq!(rows[2].1, "group");
    }

    #[test]
    fn nested_and_unnested_are_group_subject() {
        let (mut conn, _d) = fresh_db();
        let tx = conn.transaction().unwrap();
        record_group_nested_in_tx(&tx, 2, "Дочерняя", 1, "Родитель", &cause("op", None)).unwrap();
        record_group_unnested_in_tx(&tx, 2, "Дочерняя", 1, "Родитель", &cause("op", None)).unwrap();
        tx.commit().unwrap();
        let rows: Vec<(String, String, i64, Option<String>)> = conn
            .prepare(
                "SELECT kind, subject_type, subject_id, subject_label \
                 FROM history_events ORDER BY id",
            )
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(rows[0].0, "group_nested");
        assert_eq!(rows[1].0, "group_unnested");
        for r in &rows {
            assert_eq!((r.1.as_str(), r.2), ("group", 2));
            assert_eq!(r.3.as_deref(), Some("Дочерняя"));
        }
    }

    #[test]
    fn operation_ids_are_unique() {
        assert_ne!(new_operation_id(), new_operation_id());
    }
}
