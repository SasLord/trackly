//! Примитивы состава группы (Phase 41, GRP-05, D-22).
//!
//! Только функции на `&Transaction`: коммит делает вызывающий сервис, чтобы
//! освобождение устройства, запись аудита и основная операция жили в ОДНОЙ
//! транзакции writer'а. Примитивы `pub(crate)`: их переиспользуют акты и удаление
//! устройства (планы 14 и 16).

use rusqlite::Transaction;
use trackly_core::error::AppError;
use trackly_infra::repos::audit_log_sqlite::{AuditEntry, SqliteAuditLogRepository};
use trackly_infra::repos::SqliteGroupRepository;

/// Вывести устройство из группы (D-22: состав не врёт, акт проходит).
///
/// Возвращает id группы, если членство было; тогда пишет `audit_log`
/// `custom:group_member_released` с `{group_id, act_id}`. Если членства не было —
/// `None`, без записей. Место устройства не меняется.
pub(crate) fn release_device_in_tx(
    tx: &Transaction<'_>,
    groups: &SqliteGroupRepository,
    audit: &SqliteAuditLogRepository,
    device_id: i64,
    act_id: Option<i64>,
    user_id: Option<i64>,
    now: i64,
) -> Result<Option<i64>, AppError> {
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
    Ok(Some(group.id))
}
