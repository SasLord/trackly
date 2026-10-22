//! Примитивы состава группы (Phase 41, GRP-05, D-22).
//!
//! Только функции на `&Transaction`: коммит делает вызывающий сервис, чтобы
//! освобождение устройства, запись аудита и основная операция жили в ОДНОЙ
//! транзакции writer'а. Примитивы `pub(crate)`: их переиспользуют акты и удаление
//! устройства (планы 14 и 16).

use rusqlite::{OptionalExtension, Transaction};
use trackly_core::domain::groups::{ActGroupEffectKind, ActGroupEffectNew};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::audit_log_sqlite::{AuditEntry, SqliteAuditLogRepository};
use trackly_infra::repos::SqliteGroupRepository;

/// Режим следа при освобождении устройства на restore-путях акта (Phase 41.2).
///
/// `Skip` — обычный restore: вывод на restore-пути НЕ есть исходное членство, повторный
/// откат не должен возвращать устройство в группу, куда оно вошло уже после акта.
/// `Record` — вывод при убирании позиции из акта, когда группа устройства переехала
/// этим же актом: откат акта обязан вернуть такое устройство в группу.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReleaseEffect {
    Record,
    Skip,
}

/// Вывести устройство из группы (D-22: состав не врёт, акт проходит).
///
/// Возвращает id группы, если членство было; тогда пишет запись `audit_log`
/// о выводе из состава с `{group_id, act_id}`. Если членства не было —
/// `None`, без записей. Место устройства не меняется.
///
/// Путь вперёд (создание, правка, возврат акта): при `act_id = Some(..)` пишет ещё и
/// след `release` в `act_group_effects`, по которому откат акта вернёт членство.
/// Вызовы без акта (удаление устройства, вывод из состава) следа не пишут.
pub(crate) fn release_device_in_tx(
    tx: &Transaction<'_>,
    groups: &SqliteGroupRepository,
    audit: &SqliteAuditLogRepository,
    device_id: i64,
    act_id: Option<i64>,
    user_id: Option<i64>,
    now: i64,
) -> Result<Option<i64>, AppError> {
    release_inner(
        tx,
        groups,
        audit,
        device_id,
        act_id,
        user_id,
        now,
        act_id.is_some(),
    )
}

/// Тело освобождения: общее для пути вперёд и для restore-путей. Определено между двумя
/// публичными release-функциями намеренно: страж `s11_*` берёт область от первой до
/// второй и требует в ней ровно одну охрану якоря выше первого чтения членства.
#[allow(clippy::too_many_arguments)]
fn release_inner(
    tx: &Transaction<'_>,
    groups: &SqliteGroupRepository,
    audit: &SqliteAuditLogRepository,
    device_id: i64,
    act_id: Option<i64>,
    user_id: Option<i64>,
    now: i64,
    record_effect: bool,
) -> Result<Option<i64>, AppError> {
    // Phase 41.1 (D-14, D-36): якорь группы нельзя вывести из состава ни одним путём
    // (акт, restore, удаление). Phase 41.2 (D-13): акт якоря больше не отклоняется — путь акта
    // направляет якорь в перенос группы целиком ДО этого вызова, поэтому охрана остаётся
    // внутренним инвариантом и срабатывает только на путях, где якорь не покрыт планом.
    if let Some(anchor_group) = groups.anchor_group_of_device_in_tx(tx, device_id)? {
        let device_name: String = tx
            .query_row(
                "SELECT name FROM devices WHERE id = ?1",
                rusqlite::params![device_id],
                |r| r.get(0),
            )
            .unwrap_or_default();
        return Err(AppError::Validation {
            field: "id".to_string(),
            message: format!(
                "Устройство «{}» — якорь группы «{}»: из состава его вывести нельзя, он передаётся только вместе с группой.",
                device_name, anchor_group.name
            ),
        });
    }
    let Some(group) = groups.group_of_device_in_tx(tx, device_id)? else {
        return Ok(None);
    };
    if !groups.remove_device_in_tx(tx, device_id)? {
        return Ok(None);
    }
    let payload = serde_json::json!({ "group_id": group.id, "act_id": act_id });
    audit.insert(
        tx,
        AuditEntry {
            entity_type: "device",
            entity_id: device_id,
            action: "custom:group_member_released",
            user_id,
            before_json: None,
            after_json: None,
            payload_json: Some(payload.to_string()),
            created_at_utc: now,
        },
    )?;
    if record_effect {
        if let Some(act_id) = act_id {
            groups.insert_act_group_effect_in_tx(
                tx,
                &ActGroupEffectNew {
                    act_id,
                    kind: ActGroupEffectKind::Release,
                    group_id: group.id,
                    group_label: group.name.clone(),
                    device_id: Some(device_id),
                    from_place_id: None,
                    to_place_id: None,
                    from_parent_group_id: None,
                    created_at_utc: now,
                },
            )?;
        }
    }
    Ok(Some(group.id))
}

/// Вывести устройство из группы ТОЛЬКО если оно сейчас член группы с местом
/// И его место РАСХОДИТСЯ с местом группы.
///
/// Для restore-путей актов (убрали устройство из акта, правка возврата, undo
/// при удалении акта): restore из снимка возвращает старое место и затёр бы
/// «место принадлежит группе», поэтому членство снимается; если устройство не
/// в запирающей группе (нет членства либо у группы нет места, D-21) — `None`,
/// без записей. Если место устройства РАВНО месту группы (например, откат акта
/// только что вернул саму группу и её состав на место), расхождения нет и
/// собственную возвращённую группу разбирать нельзя: `None`, членство цело
/// (ловушка P-05, D-08, D-14). Сравнение живёт здесь, а не на трёх сайтах акта.
///
/// `effect` выбирает режим следа: см. [`ReleaseEffect`].
#[allow(clippy::too_many_arguments)]
pub(crate) fn release_if_locked_device_in_tx(
    tx: &Transaction<'_>,
    groups: &SqliteGroupRepository,
    audit: &SqliteAuditLogRepository,
    device_id: i64,
    act_id: Option<i64>,
    user_id: Option<i64>,
    now: i64,
    effect: ReleaseEffect,
) -> Result<Option<i64>, AppError> {
    let Some(locked) = groups.locked_group_for_device_in_tx(tx, device_id)? else {
        return Ok(None);
    };
    let device_place: Option<i64> = tx
        .query_row(
            "SELECT place_id FROM devices WHERE id = ?1",
            rusqlite::params![device_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(map_rusqlite)?
        .flatten();
    if device_place == locked.place_id {
        return Ok(None);
    }
    release_inner(
        tx,
        groups,
        audit,
        device_id,
        act_id,
        user_id,
        now,
        effect == ReleaseEffect::Record,
    )
}

/// Предупреждение для менеджера: что откат или правка акта сделали НЕ так, как
/// обещало бы механическое обращение (Phase 41.2). Тексты — фиксированные русские
/// шаблоны без внутренних токенов.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroupNotice {
    pub message: String,
    /// Группа, о которой речь (тост называет её и ведёт к ней); `None`, если её нет.
    pub group_id: Option<i64>,
}

/// Итог возврата устройства в группу при откате акта.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RestoreOutcome {
    Restored,
    Skipped(GroupNotice),
}

/// Вернуть устройство в группу, из которой его вынес акт (откат, Phase 41.2, D-08).
///
/// Вызывается ПОСЛЕ восстановления снимка устройства: `restored_place_id` — место,
/// на котором устройство оказалось после восстановления. Правила проверяются по порядку:
/// (a) группы нет или она мягко удалена; (b) устройство уже состоит в другой живой группе;
/// (c) место группы не равно `restored_place_id` — сравниваются `Option<i64>` ЦЕЛИКОМ,
/// включая `Some` против `None` и `None` против `Some` (иначе устройство вернулось бы в
/// группу с местом группы и без собственного места и нарушило бы «член живёт в месте
/// группы», D-13/D-14 фазы 41.1); (d) иначе устройство добавлено в группу.
/// Нарушение правила — не ошибка: откат проходит, менеджер получает уведомление.
#[allow(clippy::too_many_arguments)]
pub(crate) fn restore_membership_in_tx(
    tx: &Transaction<'_>,
    groups: &SqliteGroupRepository,
    audit: &SqliteAuditLogRepository,
    device_id: i64,
    group_id: i64,
    group_label: &str,
    device_name: &str,
    restored_place_id: Option<i64>,
    act_id: i64,
    user_id: Option<i64>,
    now: i64,
) -> Result<RestoreOutcome, AppError> {
    let skipped = |reason: &str, group_id: Option<i64>| {
        RestoreOutcome::Skipped(GroupNotice {
            message: format!("«{device_name}» не вернулся в «{group_label}»: {reason}"),
            group_id,
        })
    };
    let group = match groups.get_group_in_tx(tx, group_id) {
        Ok(g) => g,
        Err(AppError::NotFound { .. }) => return Ok(skipped("группа удалена.", None)),
        Err(e) => return Err(e),
    };
    if let Some(other) = groups.group_of_device_in_tx(tx, device_id)? {
        if other.id == group.id {
            // Уже в этой группе (вернули руками после акта): возвращать нечего.
            return Ok(RestoreOutcome::Restored);
        }
        return Ok(skipped(
            &format!("устройство уже входит в «{}».", other.name),
            Some(other.id),
        ));
    }
    if group.place_id != restored_place_id {
        return Ok(skipped("группа теперь в другом месте.", Some(group.id)));
    }
    groups.add_device_in_tx(tx, group.id, device_id, now)?;
    let payload = serde_json::json!({ "group_id": group.id, "act_id": act_id });
    audit.insert(
        tx,
        AuditEntry {
            entity_type: "device",
            entity_id: device_id,
            action: "custom:group_member_restored",
            user_id,
            before_json: None,
            after_json: None,
            payload_json: Some(payload.to_string()),
            created_at_utc: now,
        },
    )?;
    Ok(RestoreOutcome::Restored)
}

/// Конфликт восстановления снимка для устройства якорной группы (Phase 41.2, D-09, D-13).
///
/// Если якорную группу (или её деталь) переместили ПОСЛЕ акта, восстановление снимка
/// вернуло бы устройству старое место и разорвало бы «место якоря и деталей = место
/// группы», а вывести якорь из состава нельзя. Тогда восстановление пропускается:
/// аудит `custom:group_restore_skipped` и уведомление с причиной. Охрана якоря в
/// `release_inner` остаётся неизменной; операция проходит.
///
/// Контракт вызывающих: вызывается на ВСЕХ restore-путях (правка акта, правка возврата,
/// откат) ДО восстановления снимка; при `Some` вызывающий НЕ восстанавливает снимок
/// устройства, НЕ вызывает release и НЕ пишет возвратную строку журнала места, а всё
/// прочее на пути (удаление позиции, аудит, удаление акта) продолжается.
///
/// Члены контейнерных групп не затрагиваются: их выводит `release_if_locked` при
/// расхождении места. «Спящая» якорная группа (без места) конфликта не даёт.
#[allow(clippy::too_many_arguments)]
pub(crate) fn anchor_restore_conflict_in_tx(
    tx: &Transaction<'_>,
    groups: &SqliteGroupRepository,
    audit: &SqliteAuditLogRepository,
    device_id: i64,
    snapshot_place_id: Option<i64>,
    act_id: i64,
    user_id: Option<i64>,
    now: i64,
) -> Result<Option<GroupNotice>, AppError> {
    let Some(group) = groups.group_of_device_in_tx(tx, device_id)? else {
        return Ok(None);
    };
    if group.anchor_device_id.is_none() {
        return Ok(None);
    }
    let Some(group_place) = group.place_id else {
        return Ok(None);
    };
    if snapshot_place_id == Some(group_place) {
        return Ok(None);
    }
    let device_name: String = tx
        .query_row(
            "SELECT name FROM devices WHERE id = ?1",
            rusqlite::params![device_id],
            |r| r.get(0),
        )
        .unwrap_or_default();
    let payload = serde_json::json!({
        "group_id": group.id,
        "act_id": act_id,
        "reason": "group_moved_after_act",
    });
    audit.insert(
        tx,
        AuditEntry {
            entity_type: "device",
            entity_id: device_id,
            action: "custom:group_restore_skipped",
            user_id,
            before_json: None,
            after_json: None,
            payload_json: Some(payload.to_string()),
            created_at_utc: now,
        },
    )?;
    Ok(Some(GroupNotice {
        message: format!(
            "Группа «{}» осталась на месте: после акта её переместили, поэтому место и состояние «{}» не возвращены.",
            group.name, device_name
        ),
        group_id: Some(group.id),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::group_act_plan::test_support::{fresh_db, NOW};
    use rusqlite::{Connection, TransactionBehavior};

    /// Места 1 и 2; контейнеры: 1 «АРМ #3» (место 1), 2 «АРМ #4» (место 2),
    /// 4 «Спящая #1» (без места); якорные: 3 «Системный блок #4» (место 1, якорь 10,
    /// деталь 11), 5 «Спящий блок #5» (без места, якорь 20, деталь 21).
    /// Свободные устройства 1..=6 стоят на месте 1.
    fn seed(conn: &Connection) {
        conn.execute_batch(&format!(
            "INSERT INTO places (id, parent_id, kind, name, is_storage, created_at_utc, updated_at_utc)
               VALUES (1, NULL, 'building', 'Склад А', 1, {NOW}, {NOW}),
                      (2, NULL, 'building', 'Склад Б', 1, {NOW}, {NOW});
             INSERT INTO group_types (id, code, name, behavior, created_at_utc, updated_at_utc)
               VALUES (100, 'workstation_test', 'АРМ', 'container', {NOW}, {NOW}),
                      (101, 'assembly_test', 'Сборка', 'substitute', {NOW}, {NOW});
             INSERT INTO devices (id, type_id, name, inventory_number, status_id, place_id,
                                  created_at_utc, updated_at_utc)
               VALUES (1, 1, 'Монитор #2', 'ИНВ-000001', 2, 1, {NOW}, {NOW}),
                      (2, 1, 'Клавиатура #1', 'ИНВ-000002', 2, 1, {NOW}, {NOW}),
                      (3, 1, 'Мышь #1', 'ИНВ-000003', 2, 1, {NOW}, {NOW}),
                      (4, 1, 'Колонки #1', 'ИНВ-000004', 2, 1, {NOW}, {NOW}),
                      (5, 1, 'Веб-камера #1', 'ИНВ-000005', 2, 1, {NOW}, {NOW}),
                      (6, 1, 'Наушники #1', 'ИНВ-000006', 2, 1, {NOW}, {NOW}),
                      (10, 1, 'Системный блок #4', 'ИНВ-000010', 2, 1, {NOW}, {NOW}),
                      (11, 1, 'Планка памяти #1', 'ИНВ-000011', 2, 1, {NOW}, {NOW}),
                      (20, 1, 'Системный блок #5', 'ИНВ-000020', 2, NULL, {NOW}, {NOW}),
                      (21, 1, 'Планка памяти #2', 'ИНВ-000021', 2, NULL, {NOW}, {NOW});
             INSERT INTO groups (id, type_id, name, seq, place_id, anchor_device_id,
                                 created_at_utc, updated_at_utc)
               VALUES (1, 100, 'АРМ #3', 3, 1, NULL, {NOW}, {NOW}),
                      (2, 100, 'АРМ #4', 4, 2, NULL, {NOW}, {NOW}),
                      (3, 101, 'Системный блок #4', 4, 1, 10, {NOW}, {NOW}),
                      (4, 100, 'Спящая #1', 5, NULL, NULL, {NOW}, {NOW}),
                      (5, 101, 'Спящий блок #5', 5, NULL, 20, {NOW}, {NOW});
             INSERT INTO group_devices (device_id, group_id, added_at_utc)
               VALUES (10, 3, {NOW}), (11, 3, {NOW}), (20, 5, {NOW}), (21, 5, {NOW});"
        ))
        .expect("seed");
    }

    fn member_of(conn: &Connection, device_id: i64) -> Option<i64> {
        conn.query_row(
            "SELECT group_id FROM group_devices WHERE device_id = ?1",
            [device_id],
            |r| r.get(0),
        )
        .optional()
        .unwrap()
    }

    fn count(conn: &Connection, sql: &str, id: i64) -> i64 {
        conn.query_row(sql, [id], |r| r.get(0)).unwrap()
    }

    fn effects_for(conn: &Connection, device_id: i64) -> i64 {
        count(
            conn,
            "SELECT COUNT(*) FROM act_group_effects WHERE device_id = ?1 AND kind = 'release'",
            device_id,
        )
    }

    fn audit_rows(conn: &Connection, action: &str, device_id: i64) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM audit_log WHERE action = ?1 AND entity_id = ?2",
            rusqlite::params![action, device_id],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn add_member(conn: &Connection, device_id: i64, group_id: i64) {
        conn.execute(
            "INSERT INTO group_devices (device_id, group_id, added_at_utc) VALUES (?1, ?2, ?3)",
            rusqlite::params![device_id, group_id, NOW],
        )
        .unwrap();
    }

    macro_rules! with_tx {
        ($conn:ident, $tx:ident, $body:block) => {{
            let $tx = $conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .expect("tx");
            let r = $body;
            $tx.commit().expect("commit");
            r
        }};
    }

    fn restore(
        tx: &Transaction<'_>,
        device_id: i64,
        group_id: i64,
        place: Option<i64>,
    ) -> RestoreOutcome {
        restore_membership_in_tx(
            tx,
            &SqliteGroupRepository,
            &SqliteAuditLogRepository,
            device_id,
            group_id,
            "АРМ #3",
            "Монитор #2",
            place,
            7,
            None,
            NOW,
        )
        .expect("restore")
    }

    #[test]
    fn restore_deleted_group_is_skipped() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        conn.execute("UPDATE groups SET deleted_at_utc = ?1 WHERE id = 1", [NOW])
            .unwrap();
        let out = with_tx!(conn, tx, { restore(&tx, 1, 1, Some(1)) });
        assert_eq!(
            out,
            RestoreOutcome::Skipped(GroupNotice {
                message: "«Монитор #2» не вернулся в «АРМ #3»: группа удалена.".to_string(),
                group_id: None,
            })
        );
        assert_eq!(member_of(&conn, 1), None);
        // Группы с таким id не было вовсе — то же правило.
        let out = with_tx!(conn, tx, { restore(&tx, 1, 999, Some(1)) });
        assert!(matches!(out, RestoreOutcome::Skipped(_)));
    }

    #[test]
    fn restore_other_group_is_skipped() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        add_member(&conn, 1, 2);
        let out = with_tx!(conn, tx, { restore(&tx, 1, 1, Some(1)) });
        assert_eq!(
            out,
            RestoreOutcome::Skipped(GroupNotice {
                message: "«Монитор #2» не вернулся в «АРМ #3»: устройство уже входит в «АРМ #4»."
                    .to_string(),
                group_id: Some(2),
            })
        );
        assert_eq!(member_of(&conn, 1), Some(2), "чужое членство не тронуто");
    }

    #[test]
    fn restore_moved_group_is_skipped() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        // Группа 1 стоит на месте 1, устройство после восстановления — на месте 2.
        let out = with_tx!(conn, tx, { restore(&tx, 1, 1, Some(2)) });
        assert_eq!(
            out,
            RestoreOutcome::Skipped(GroupNotice {
                message: "«Монитор #2» не вернулся в «АРМ #3»: группа теперь в другом месте."
                    .to_string(),
                group_id: Some(1),
            })
        );
        assert_eq!(member_of(&conn, 1), None);
    }

    #[test]
    fn restore_restored_when_group_is_alive_and_in_place() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let out = with_tx!(conn, tx, { restore(&tx, 1, 1, Some(1)) });
        assert_eq!(out, RestoreOutcome::Restored);
        assert_eq!(member_of(&conn, 1), Some(1));
        assert_eq!(audit_rows(&conn, "custom:group_member_restored", 1), 1);
    }

    #[test]
    fn restore_some_none_skips_when_group_has_place_and_device_has_none() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let out = with_tx!(conn, tx, { restore(&tx, 1, 1, None) });
        assert!(matches!(out, RestoreOutcome::Skipped(_)), "{out:?}");
        assert_eq!(member_of(&conn, 1), None);
    }

    #[test]
    fn restore_none_some_skips_when_group_has_no_place_and_device_has_one() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let out = with_tx!(conn, tx, { restore(&tx, 1, 4, Some(1)) });
        assert!(matches!(out, RestoreOutcome::Skipped(_)), "{out:?}");
        assert_eq!(member_of(&conn, 1), None);
    }

    #[test]
    fn restore_none_none_restores_when_neither_side_has_a_place() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let out = with_tx!(conn, tx, { restore(&tx, 1, 4, None) });
        assert_eq!(out, RestoreOutcome::Restored);
        assert_eq!(member_of(&conn, 1), Some(4));
    }

    /// Расходящаяся пара на ОДНОМ устройстве: путь вперёд пишет след, restore со Skip — нет,
    /// restore с Record — да.
    #[test]
    fn release_modes_record_effect_only_when_asked_on_the_same_device() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let groups = SqliteGroupRepository;
        let audit = SqliteAuditLogRepository;

        // Путь вперёд с актом: след пишется.
        add_member(&conn, 1, 1);
        with_tx!(conn, tx, {
            assert_eq!(
                release_device_in_tx(&tx, &groups, &audit, 1, Some(7), None, NOW).unwrap(),
                Some(1)
            );
        });
        assert_eq!(effects_for(&conn, 1), 1);

        // Путь вперёд без акта: следа нет.
        add_member(&conn, 1, 1);
        with_tx!(conn, tx, {
            assert_eq!(
                release_device_in_tx(&tx, &groups, &audit, 1, None, None, NOW).unwrap(),
                Some(1)
            );
        });
        assert_eq!(effects_for(&conn, 1), 1, "без акта след не пишется");

        // Restore-путь со Skip: место устройства расходится с местом группы, устройство
        // выведено, но следа нет.
        add_member(&conn, 1, 1);
        conn.execute("UPDATE devices SET place_id = 2 WHERE id = 1", [])
            .unwrap();
        with_tx!(conn, tx, {
            assert_eq!(
                release_if_locked_device_in_tx(
                    &tx,
                    &groups,
                    &audit,
                    1,
                    Some(7),
                    None,
                    NOW,
                    ReleaseEffect::Skip
                )
                .unwrap(),
                Some(1)
            );
        });
        assert_eq!(member_of(&conn, 1), None);
        assert_eq!(effects_for(&conn, 1), 1, "Skip следа не пишет");

        // Тот же restore-путь с Record: след пишется.
        add_member(&conn, 1, 1);
        with_tx!(conn, tx, {
            assert_eq!(
                release_if_locked_device_in_tx(
                    &tx,
                    &groups,
                    &audit,
                    1,
                    Some(7),
                    None,
                    NOW,
                    ReleaseEffect::Record
                )
                .unwrap(),
                Some(1)
            );
        });
        assert_eq!(effects_for(&conn, 1), 2, "Record пишет след");
    }

    #[test]
    fn release_if_locked_keeps_member_when_places_match_and_releases_when_they_differ() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let groups = SqliteGroupRepository;
        let audit = SqliteAuditLogRepository;
        add_member(&conn, 1, 1);
        // Устройство 1 стоит на месте 1, группа 1 — тоже: расхождения нет.
        with_tx!(conn, tx, {
            assert_eq!(
                release_if_locked_device_in_tx(
                    &tx,
                    &groups,
                    &audit,
                    1,
                    Some(7),
                    None,
                    NOW,
                    ReleaseEffect::Skip
                )
                .unwrap(),
                None
            );
        });
        assert_eq!(member_of(&conn, 1), Some(1), "членство цело");
        assert_eq!(audit_rows(&conn, "custom:group_member_released", 1), 0);

        // Место устройства стало другим: расхождение, устройство выведено.
        conn.execute("UPDATE devices SET place_id = 2 WHERE id = 1", [])
            .unwrap();
        with_tx!(conn, tx, {
            assert_eq!(
                release_if_locked_device_in_tx(
                    &tx,
                    &groups,
                    &audit,
                    1,
                    Some(7),
                    None,
                    NOW,
                    ReleaseEffect::Skip
                )
                .unwrap(),
                Some(1)
            );
        });
        assert_eq!(member_of(&conn, 1), None);
        assert_eq!(audit_rows(&conn, "custom:group_member_released", 1), 1);
    }

    fn conflict(
        conn: &mut Connection,
        device_id: i64,
        snapshot_place: Option<i64>,
    ) -> Option<GroupNotice> {
        with_tx!(conn, tx, {
            anchor_restore_conflict_in_tx(
                &tx,
                &SqliteGroupRepository,
                &SqliteAuditLogRepository,
                device_id,
                snapshot_place,
                7,
                None,
                NOW,
            )
            .expect("conflict")
        })
    }

    #[test]
    fn anchor_conflict_when_anchor_group_was_moved_after_the_act() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        // Снимок якоря говорит «место 2», группа 3 стоит на месте 1.
        let notice = conflict(&mut conn, 10, Some(2)).expect("конфликт");
        assert_eq!(
            notice.message,
            "Группа «Системный блок #4» осталась на месте: после акта её переместили, поэтому место и состояние «Системный блок #4» не возвращены."
        );
        assert_eq!(notice.group_id, Some(3));
        assert_eq!(audit_rows(&conn, "custom:group_restore_skipped", 10), 1);
        let payload: String = conn
            .query_row(
                "SELECT payload_json FROM audit_log WHERE action = 'custom:group_restore_skipped'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(payload.contains("group_moved_after_act"), "{payload}");
    }

    #[test]
    fn anchor_without_conflict_when_places_match() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        assert_eq!(conflict(&mut conn, 10, Some(1)), None);
        assert_eq!(audit_rows(&conn, "custom:group_restore_skipped", 10), 0);
    }

    #[test]
    fn anchor_group_part_conflicts_like_the_anchor() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let notice = conflict(&mut conn, 11, Some(2)).expect("конфликт у детали");
        assert_eq!(notice.group_id, Some(3));
        assert_eq!(audit_rows(&conn, "custom:group_restore_skipped", 11), 1);
    }

    #[test]
    fn container_member_never_conflicts() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        add_member(&conn, 2, 1);
        assert_eq!(conflict(&mut conn, 2, Some(2)), None);
        // Устройство вне групп тоже.
        assert_eq!(conflict(&mut conn, 3, Some(2)), None);
    }

    #[test]
    fn dormant_anchor_group_never_conflicts() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        assert_eq!(conflict(&mut conn, 20, Some(1)), None);
        assert_eq!(conflict(&mut conn, 21, Some(2)), None);
    }
}
