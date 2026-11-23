//! Протаскивание места группы на состав (Phase 41, GRP-06, D-17/D-20/D-30).
//!
//! Здесь только примитивы на `&Transaction`: коммит делает вызывающий сервис, чтобы
//! перенос, запись пакета в журнал и аудит жили в ОДНОЙ транзакции writer'а.
//! Примитивы `pub(crate)`: их переиспользуют добавление в состав (план 11) и
//! массовый перенос места (план 15, D-23).

use crate::services::history_events::{new_operation_id, record_status_change_in_tx, HistoryCause};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use trackly_core::domain::group_move_status::{
    status_after_move, storage_transition, StorageTransition,
};
use trackly_core::domain::history_events::HistoryReason;
use trackly_core::domain::place_movements::{MovementEntityKind, MovementSource};
use trackly_core::domain::repairs::is_repair_exempt_status_code;
use trackly_core::error::AppError;
use trackly_core::ports::places::PlaceRepository;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::audit_log_sqlite::{AuditEntry, SqliteAuditLogRepository};
use trackly_infra::repos::repairs_sqlite::SqliteRepairRepository;
use trackly_infra::repos::{
    SqliteCartridgeRepository, SqliteDeviceRepository, SqliteGroupRepository,
    SqlitePlaceMovementsRepository, SqlitePlaceRepository,
};

/// Репозитории, нужные протаскиванию (все без состояния).
pub(crate) struct GroupMoveDeps {
    pub groups: SqliteGroupRepository,
    pub devices: SqliteDeviceRepository,
    pub movements: SqlitePlaceMovementsRepository,
    pub places: SqlitePlaceRepository,
    pub cartridges: SqliteCartridgeRepository,
    pub audit: SqliteAuditLogRepository,
}

impl GroupMoveDeps {
    pub(crate) fn new() -> Self {
        Self {
            groups: SqliteGroupRepository,
            devices: SqliteDeviceRepository,
            movements: SqlitePlaceMovementsRepository,
            places: SqlitePlaceRepository,
            cartridges: SqliteCartridgeRepository,
            audit: SqliteAuditLogRepository,
        }
    }
}

/// Причина записи журнала при протаскивании места: источник и (для акта) его id.
///
/// Phase 41.2 (D-15): параметризация вместо форка. Существующие вызовы передают
/// [`GroupMoveCause::GROUP`] и поведения не меняют; акт передаст
/// [`GroupMoveCause::act`], чтобы откат акта находил строки журнала по `act_id`
/// (в том числе строки картриджей принтера-члена, D-14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GroupMoveCause {
    pub source: MovementSource,
    pub act_id: Option<i64>,
}

impl GroupMoveCause {
    /// Операция над группой без акта (состав, место группы).
    pub(crate) const GROUP: Self = Self {
        source: MovementSource::Group,
        act_id: None,
    };

    /// Перенос как следствие акта `act_id`.
    pub(crate) const fn act(act_id: i64) -> Self {
        Self {
            source: MovementSource::Act,
            act_id: Some(act_id),
        }
    }
}

/// Итог переноса группы.
#[derive(Debug, Default)]
pub(crate) struct GroupMoveOutcome {
    pub moved_devices: i32,
    pub moved_nested_groups: i32,
    /// Старые и новые места без дублей; пусто, если ничего не менялось.
    pub changed_place_ids: Vec<i64>,
    pub batch_id: Option<String>,
    /// D-34: устройства, у которых вместе с местом сменился статус.
    pub status_changed_device_ids: Vec<i64>,
    /// Phase 41.10 (D-04, D-25, ревью WR-04): устройство -> id строки журнала места,
    /// записанной пакетом группы. Вызывающий связывает с ней событие статуса той же операции.
    pub device_movement_ids: std::collections::HashMap<i64, i64>,
    /// Id строки журнала места самой группы (если перенос записан): событие вложения той же
    /// операции несёт его (D-25).
    pub group_movement_id: Option<i64>,
}

/// D-34: сдвиг статуса при переносе через границу «склад <-> не склад».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StatusShift {
    pub transition: StorageTransition,
}

/// Итог применения места группы к одному устройству.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DevicePlaceChange {
    /// Phase 41.10 (D-04): id записи журнала места, если перемещение записано.
    pub movement_id: Option<i64>,
    /// Место устройства действительно поменялось.
    pub changed: bool,
    /// Прежнее место (имеет смысл только при `changed`).
    pub previous_place: Option<i64>,
    /// D-34: вместе с местом сменился и статус.
    pub status_changed: bool,
}

/// Единый признак «склад» (D-09, D-34): место или любой его предок с `is_storage`;
/// удалённое складское место не считается, несуществующее место - не склад.
pub(crate) fn place_is_storage_in_tx(conn: &Connection, place_id: i64) -> Result<bool, AppError> {
    let n: i64 = conn
        .query_row(
            "WITH RECURSIVE anc(id) AS ( \
                 SELECT id FROM places WHERE id = ?1 \
                 UNION ALL \
                 SELECT p.parent_id FROM places p JOIN anc a ON p.id = a.id \
                 WHERE p.parent_id IS NOT NULL \
             ) \
             SELECT COUNT(*) FROM places p2 JOIN anc ON p2.id = anc.id \
             WHERE p2.is_storage = 1 AND p2.deleted_at_utc IS NULL",
            params![place_id],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)?;
    Ok(n > 0)
}

/// Поедет ли устройство с таким статусом и местом (общее правило примитива и
/// перечисления для предпросмотра): исключённый по ремонту статус едет только вместе с
/// группой, и место обязано действительно поменяться.
fn device_will_move(
    before_status_code: &str,
    before_place: Option<i64>,
    target_place: i64,
    group_old_place: Option<i64>,
) -> bool {
    let rides = group_old_place.is_some() && before_place == group_old_place;
    if is_repair_exempt_status_code(before_status_code) && !rides {
        return false;
    }
    before_place != Some(target_place)
}

fn status_id_by_code_in_tx(tx: &Transaction<'_>, code: &str) -> Result<i64, AppError> {
    tx.query_row(
        "SELECT id FROM device_statuses WHERE code = ?1",
        params![code],
        |r| r.get(0),
    )
    .optional()
    .map_err(map_rusqlite)?
    .ok_or_else(|| AppError::Validation {
        field: "status".to_string(),
        message: format!("Статус «{code}» не найден в справочнике."),
    })
}

/// Поставить устройству место группы.
///
/// Журнал: строка пишется только при `Some -> Some` (общий гейт D-04/D-06). Если у
/// устройства места не было, строки нет («откуда» NOT NULL) — вместо неё запись в
/// `audit_log` (D-30). Картриджи, прикреплённые к устройству, едут тем же пакетом.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_group_place_to_device_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    device_id: i64,
    target_place: i64,
    batch_id: Option<&str>,
    entity_label: Option<&str>,
    group_id: i64,
    user_id: Option<i64>,
    now: i64,
    cause: GroupMoveCause,
) -> Result<DevicePlaceChange, AppError> {
    apply_group_place_inner(
        tx,
        deps,
        device_id,
        target_place,
        batch_id,
        entity_label,
        group_id,
        user_id,
        now,
        cause,
        None,
        None,
    )
}

/// Ядро примитива. `group_old_place`: `Some(место)` — вызов из переноса группы с прежним
/// местом группы; устройство в исключённом статусе едет с группой только если его место
/// совпадает с ним (D-15), иначе остаётся на своём месте. `None` — вступление/возврат:
/// исключённое устройство не стягивается никогда.
#[allow(clippy::too_many_arguments)]
fn apply_group_place_inner(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    device_id: i64,
    target_place: i64,
    batch_id: Option<&str>,
    entity_label: Option<&str>,
    group_id: i64,
    user_id: Option<i64>,
    now: i64,
    cause: GroupMoveCause,
    group_old_place: Option<i64>,
    status_shift: Option<StatusShift>,
) -> Result<DevicePlaceChange, AppError> {
    let before = deps.devices.get_in_tx(tx, device_id)?;
    let rides_with_group = group_old_place.is_some() && before.place_id == group_old_place;
    // Phase 41.4 (D-14): устройство в исключённом статусе (на ремонте, в ремонте, на
    // списание) не стягивается в место группы; членство при этом сохраняется. Общий
    // примитив: правило действует и при переносе группы, и при вступлении/возврате.
    if is_repair_exempt_status_code(&before.status_code) && !rides_with_group {
        return Ok(DevicePlaceChange {
            movement_id: None,
            changed: false,
            previous_place: before.place_id,
            status_changed: false,
        });
    }
    if before.place_id == Some(target_place) {
        return Ok(DevicePlaceChange {
            movement_id: None,
            changed: false,
            previous_place: before.place_id,
            status_changed: false,
        });
    }

    debug_assert!(device_will_move(
        &before.status_code,
        before.place_id,
        target_place,
        group_old_place
    ));

    // D-34: новый статус решает чистое правило; устройство с открытой ремонтной заявкой
    // статус не меняет (его ведёт ремонт).
    let mut new_status_id = before.status_id;
    let mut status_to: Option<&'static str> = None;
    if let Some(shift) = status_shift {
        if let Some(code) = status_after_move(Some(shift.transition), &before.status_code) {
            if SqliteRepairRepository
                .open_repair_request_id(tx, device_id)?
                .is_none()
            {
                new_status_id = status_id_by_code_in_tx(tx, code)?;
                status_to = Some(code);
            }
        }
    }

    deps.devices.update_status_and_place_in_tx(
        tx,
        device_id,
        new_status_id,
        Some(target_place),
        now,
    )?;

    let movement_id = deps.movements.record_batch_movement_if_applicable(
        tx,
        &deps.places,
        MovementEntityKind::Device,
        device_id,
        before.place_id,
        Some(target_place),
        cause.source,
        None,
        cause.act_id,
        user_id,
        now,
        batch_id,
        entity_label,
        Some(group_id),
    )?;

    if before.place_id.is_none() {
        let payload = serde_json::json!({ "group_id": group_id, "place_id": target_place });
        deps.audit.insert(
            tx,
            AuditEntry {
                entity_type: "device",
                entity_id: device_id,
                action: "custom:group_place_assigned",
                user_id,
                before_json: None,
                after_json: None,
                payload_json: Some(payload.to_string()),
                created_at_utc: now,
            },
        )?;
    }

    // Картриджи принтера едут за принтером; без прикреплённых картриджей — no-op.
    deps.cartridges.cascade_place_for_printer_batch_in_tx(
        tx,
        device_id,
        Some(target_place),
        cause.source,
        "вместе с принтером",
        user_id,
        now,
        batch_id,
        entity_label,
        Some(group_id),
        cause.act_id,
    )?;

    if let Some(to) = status_to {
        let payload = serde_json::json!({
            "group_id": group_id,
            "batch_id": batch_id,
            "status_from": before.status_code,
            "status_to": to,
        });
        deps.audit.insert(
            tx,
            AuditEntry {
                entity_type: "device",
                entity_id: device_id,
                action: "custom:group_move_status_changed",
                user_id,
                before_json: None,
                after_json: None,
                payload_json: Some(payload.to_string()),
                created_at_utc: now,
            },
        )?;
        // Phase 41.10 (D-04, D-25): перемещение уже записано, теперь событие статуса.
        let group_label: String = deps
            .groups
            .get_group_in_tx(tx, group_id)
            .map(|g| g.name)
            .unwrap_or_default();
        let operation_id = new_operation_id();
        let cause_ev = HistoryCause {
            operation_id: &operation_id,
            source: HistoryReason::GroupMove,
            act_id: cause.act_id,
            request_id: None,
            ref_group: Some((group_id, group_label.as_str())),
            movement_id,
            user_id,
            now,
        };
        record_status_change_in_tx(tx, device_id, before.status_id, new_status_id, &cause_ev)?;
    }

    Ok(DevicePlaceChange {
        movement_id,
        changed: true,
        previous_place: before.place_id,
        status_changed: status_to.is_some(),
    })
}

fn push_unique(out: &mut Vec<i64>, id: i64) {
    if !out.contains(&id) {
        out.push(id);
    }
}

/// Целевое место переноса: существует и не в архиве.
fn check_target_place(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    target_place_id: i64,
) -> Result<(), AppError> {
    let place_err = |message: &str| AppError::Validation {
        field: "place_id".to_string(),
        message: message.to_string(),
    };
    let target = match deps.places.get(tx, target_place_id) {
        Ok(p) => p,
        Err(AppError::NotFound { .. }) => return Err(place_err("Место не найдено.")),
        Err(e) => return Err(e),
    };
    if target.archived_at_utc.is_some() {
        return Err(place_err("Место в архиве: выберите другое."));
    }
    Ok(())
}

/// Перенести корневую группу и весь её состав (включая вложенные группы) в `target_place_id`.
///
/// `expected_version`: `Some` — CAS по версии группы (UI), `None` — внутренний вызов (D-23).
pub(crate) fn move_group_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    group_id: i64,
    expected_version: Option<i64>,
    target_place_id: i64,
    user_id: Option<i64>,
    now: i64,
) -> Result<GroupMoveOutcome, AppError> {
    move_group_with_status_in_tx(
        tx,
        deps,
        group_id,
        expected_version,
        target_place_id,
        user_id,
        now,
        false,
    )
}

/// Проверки переноса группы, общие для переноса и предпросмотра: группа корневая,
/// целевое место существует и не в архиве.
pub(crate) fn ensure_group_movable_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    group: &trackly_core::domain::groups::GroupRow,
    target_place_id: i64,
) -> Result<(), AppError> {
    if group.parent_group_id.is_some() {
        let root_id = deps.groups.root_group_id_in_tx(tx, group.id)?;
        let root = deps.groups.get_group_in_tx(tx, root_id)?;
        return Err(AppError::Validation {
            field: "place_id".to_string(),
            message: format!(
                "Место вложенной группы задаётся корневой группой «{}».",
                root.name
            ),
        });
    }
    check_target_place(tx, deps, target_place_id)
}

/// D-34: переход «склад <-> не склад» по месту группы и целевому месту; у группы без
/// места «старого места» нет, перехода нет.
pub(crate) fn group_move_transition_in_tx(
    tx: &Transaction<'_>,
    group_place: Option<i64>,
    target_place_id: i64,
) -> Result<Option<StorageTransition>, AppError> {
    match group_place {
        Some(from) => Ok(storage_transition(
            place_is_storage_in_tx(tx, from)?,
            place_is_storage_in_tx(tx, target_place_id)?,
        )),
        None => Ok(None),
    }
}

/// То же, что [`move_group_in_tx`], но при `change_status` и переходе «склад <-> не склад»
/// устройства с «обычным» статусом получают новый (D-34); решение и перечень - на сервере.
#[allow(clippy::too_many_arguments)]
pub(crate) fn move_group_with_status_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    group_id: i64,
    expected_version: Option<i64>,
    target_place_id: i64,
    user_id: Option<i64>,
    now: i64,
    change_status: bool,
) -> Result<GroupMoveOutcome, AppError> {
    let group = deps.groups.get_group_in_tx(tx, group_id)?;

    if let Some(expected) = expected_version {
        if expected != group.version {
            return Err(AppError::OptimisticLockMismatch {
                entity: "group",
                id: group_id,
                expected,
                actual: group.version,
            });
        }
    }

    ensure_group_movable_in_tx(tx, deps, &group, target_place_id)?;

    let status_shift = if change_status {
        group_move_transition_in_tx(tx, group.place_id, target_place_id)?
            .map(|transition| StatusShift { transition })
    } else {
        None
    };

    let batch_id = uuid::Uuid::new_v4().to_string();
    let outcome = propagate_group_place_with_shift_in_tx(
        tx,
        deps,
        group_id,
        target_place_id,
        group.place_id,
        &batch_id,
        user_id,
        now,
        GroupMoveCause::GROUP,
        status_shift,
    )?;
    if outcome.batch_id.is_none() {
        return Ok(outcome);
    }

    let payload = serde_json::json!({
        "batch_id": batch_id,
        "moved_devices": outcome.moved_devices,
        "status_changed": outcome.status_changed_device_ids.len(),
        "from": group.place_id,
        "to": target_place_id,
    });
    deps.audit.insert(
        tx,
        AuditEntry {
            entity_type: "group",
            entity_id: group_id,
            action: "move",
            user_id,
            before_json: None,
            after_json: None,
            payload_json: Some(payload.to_string()),
            created_at_utc: now,
        },
    )?;

    Ok(outcome)
}

/// Перенести ВЛОЖЕННУЮ группу вместе с её блоком (сама группа, её вложенные группы и
/// устройства состава) в `target_place_id`; место корневой группы и её прочих членов не
/// меняется (Phase 41.4, D-18/D-32). Внутренний вызов: версия группы не проверяется.
///
/// Блок может оказаться «в отъезде» (его место отличается от места корня): перенос
/// корня такой блок не трогает (D-15), возврат на место родителя отъезд снимает.
pub(crate) fn move_nested_block_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    group_id: i64,
    target_place_id: i64,
    user_id: Option<i64>,
    now: i64,
) -> Result<GroupMoveOutcome, AppError> {
    let group = deps.groups.get_group_in_tx(tx, group_id)?;
    if group.parent_group_id.is_none() {
        return Err(AppError::Validation {
            field: "place_id".to_string(),
            message: "Для корневой группы используйте перенос группы.".to_string(),
        });
    }
    check_target_place(tx, deps, target_place_id)?;

    let batch_id = uuid::Uuid::new_v4().to_string();
    let outcome = propagate_group_place_in_tx(
        tx,
        deps,
        group_id,
        target_place_id,
        group.place_id,
        &batch_id,
        user_id,
        now,
        GroupMoveCause::GROUP,
    )?;
    if outcome.batch_id.is_none() {
        return Ok(outcome);
    }

    let payload = serde_json::json!({
        "batch_id": batch_id,
        "moved_devices": outcome.moved_devices,
        "from": group.place_id,
        "to": target_place_id,
        "nested": true,
    });
    deps.audit.insert(
        tx,
        AuditEntry {
            entity_type: "group",
            entity_id: group_id,
            action: "move",
            user_id,
            before_json: None,
            after_json: None,
            payload_json: Some(payload.to_string()),
            created_at_utc: now,
        },
    )?;

    Ok(outcome)
}

/// Ядро протаскивания: поставить `target_place` группе поддерева `group_id`, вложенным
/// группам и каждому устройству состава (с каскадом картриджей). Общее для переноса
/// корневой группы и вложения группы под родителя с местом (план 11).
///
/// Порядок важен: место групп поддерева меняется ДО цикла устройств (денормализация,
/// Pitfall 4). Строка группы в журнале пишется общим гейтом (только `Some -> Some`);
/// `group_id` и `entity_label` (имя группы) уходят в КАЖДУЮ строку пакета. Если не
/// изменилось ничего, возвращается `GroupMoveOutcome::default()` (`batch_id = None`).
/// Аудит пишет вызывающий: у переноса и вложения разные действия.
#[allow(clippy::too_many_arguments)]
pub(crate) fn propagate_group_place_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    group_id: i64,
    target_place: i64,
    old_place: Option<i64>,
    batch_id: &str,
    user_id: Option<i64>,
    now: i64,
    cause: GroupMoveCause,
) -> Result<GroupMoveOutcome, AppError> {
    propagate_group_place_with_shift_in_tx(
        tx,
        deps,
        group_id,
        target_place,
        old_place,
        batch_id,
        user_id,
        now,
        cause,
        None,
    )
}

/// Группы и устройства поддерева за вычетом блоков «в отъезде» (D-15/D-32).
fn group_move_sets(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    group_id: i64,
    old_place: Option<i64>,
) -> Result<(Vec<i64>, Vec<i64>), AppError> {
    let mut group_ids = deps.groups.subtree_group_ids_in_tx(tx, group_id)?;
    let mut device_ids = deps.groups.subtree_device_ids_in_tx(tx, group_id)?;
    // Phase 41.4 (D-15/D-32): вложенный блок «в отъезде» (его место отличается от
    // места переносимой группы) остаётся на месте вместе со всем поддеревом.
    if old_place.is_some() {
        for away in deps
            .groups
            .away_nested_group_ids_in_tx(tx, group_id, old_place)?
        {
            let away_groups = deps.groups.subtree_group_ids_in_tx(tx, away)?;
            let away_devices = deps.groups.subtree_device_ids_in_tx(tx, away)?;
            group_ids.retain(|id| !away_groups.contains(id));
            device_ids.retain(|id| !away_devices.contains(id));
        }
    }
    Ok((group_ids, device_ids))
}

/// D-34: устройства, статус которых примитив поменяет при тех же аргументах (без записи).
/// Основа числа в подписи предпросмотра: то же множество, что у фактического переноса.
pub(crate) fn status_shift_candidates_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    group_id: i64,
    old_place: Option<i64>,
    target_place: i64,
    transition: StorageTransition,
) -> Result<Vec<i64>, AppError> {
    let (_, device_ids) = group_move_sets(tx, deps, group_id, old_place)?;
    let mut out = Vec::new();
    for device_id in device_ids {
        let before = deps.devices.get_in_tx(tx, device_id)?;
        if !device_will_move(
            &before.status_code,
            before.place_id,
            target_place,
            old_place,
        ) {
            continue;
        }
        if status_after_move(Some(transition), &before.status_code).is_none() {
            continue;
        }
        if SqliteRepairRepository
            .open_repair_request_id(tx, device_id)?
            .is_some()
        {
            continue;
        }
        out.push(device_id);
    }
    Ok(out)
}

/// [`propagate_group_place_in_tx`] со сдвигом статуса (D-34); `None` - прежнее поведение.
#[allow(clippy::too_many_arguments)]
pub(crate) fn propagate_group_place_with_shift_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    group_id: i64,
    target_place: i64,
    old_place: Option<i64>,
    batch_id: &str,
    user_id: Option<i64>,
    now: i64,
    cause: GroupMoveCause,
    status_shift: Option<StatusShift>,
) -> Result<GroupMoveOutcome, AppError> {
    let group = deps.groups.get_group_in_tx(tx, group_id)?;
    let (group_ids, device_ids) = group_move_sets(tx, deps, group_id, old_place)?;
    let group_moves = old_place != Some(target_place);

    let mut changed_places: Vec<i64> = Vec::new();
    let mut group_movement_id: Option<i64> = None;

    if group_moves {
        deps.groups
            .set_groups_place_in_tx(tx, &group_ids, Some(target_place), now)?;
        group_movement_id = deps.movements.record_batch_movement_if_applicable(
            tx,
            &deps.places,
            MovementEntityKind::Group,
            group_id,
            old_place,
            Some(target_place),
            cause.source,
            None,
            cause.act_id,
            user_id,
            now,
            Some(batch_id),
            Some(&group.name),
            Some(group_id),
        )?;
        if let Some(p) = old_place {
            push_unique(&mut changed_places, p);
        }
    }

    let mut moved_devices = 0;
    let mut status_changed_device_ids: Vec<i64> = Vec::new();
    let mut device_movement_ids: std::collections::HashMap<i64, i64> =
        std::collections::HashMap::new();
    for device_id in &device_ids {
        let change = apply_group_place_inner(
            tx,
            deps,
            *device_id,
            target_place,
            Some(batch_id),
            Some(&group.name),
            group_id,
            user_id,
            now,
            cause,
            old_place,
            status_shift,
        )?;
        if let Some(m) = change.movement_id {
            device_movement_ids.insert(*device_id, m);
        }
        if change.changed {
            moved_devices += 1;
            if change.status_changed {
                status_changed_device_ids.push(*device_id);
            }
            if let Some(p) = change.previous_place {
                push_unique(&mut changed_places, p);
            }
        }
    }

    if !group_moves && moved_devices == 0 {
        return Ok(GroupMoveOutcome::default());
    }
    push_unique(&mut changed_places, target_place);

    Ok(GroupMoveOutcome {
        moved_devices,
        moved_nested_groups: if group_moves {
            (group_ids.len() as i32 - 1).max(0)
        } else {
            0
        },
        changed_place_ids: changed_places,
        batch_id: Some(batch_id.to_string()),
        status_changed_device_ids,
        device_movement_ids,
        group_movement_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::group_act_plan::test_support::{fresh_db, NOW};
    use rusqlite::{params, Connection, TransactionBehavior};

    const OFFICE: i64 = 1;
    const STORE: i64 = 2;
    const ROOM: i64 = 3;
    const ROOT: i64 = 1;
    const NESTED: i64 = 2;
    const ROOT_DEV: i64 = 1;
    const NESTED_ANCHOR: i64 = 2;
    const NESTED_PART: i64 = 3;

    /// Корень «АРМ-1» (Кабинет 12) с членом; вложенная «АРМ-1.1» с двумя устройствами.
    fn seed(conn: &Connection) {
        for (id, name) in [
            (OFFICE, "Кабинет 12"),
            (STORE, "Склад А"),
            (ROOM, "Склад Б"),
        ] {
            conn.execute(
                "INSERT INTO places (id, parent_id, kind, name, is_storage, created_at_utc, \
                 updated_at_utc) VALUES (?1, NULL, 'room', ?2, 0, ?3, ?3)",
                params![id, name, NOW],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO group_types (code, name, behavior, created_at_utc, updated_at_utc) \
             VALUES ('rack_t', 'Стойка', 'container', ?1, ?1)",
            params![NOW],
        )
        .unwrap();
        let ty = conn.last_insert_rowid();
        for (id, name, seq, parent) in
            [(ROOT, "АРМ-1", 1, None), (NESTED, "АРМ-1.1", 2, Some(ROOT))]
        {
            conn.execute(
                "INSERT INTO groups (id, type_id, name, seq, place_id, parent_group_id, \
                 created_at_utc, updated_at_utc, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, 1)",
                params![id, ty, name, seq, OFFICE, parent, NOW],
            )
            .unwrap();
        }
        for (id, group) in [
            (ROOT_DEV, ROOT),
            (NESTED_ANCHOR, NESTED),
            (NESTED_PART, NESTED),
        ] {
            conn.execute(
                "INSERT INTO devices (id, type_id, name, inventory_number, place_id, status_id, \
                 created_at_utc, updated_at_utc, version) VALUES (?1, 1, ?2, ?3, ?4, 1, ?5, ?5, 1)",
                params![
                    id,
                    format!("ПК-{id:05}"),
                    format!("ИНВ-{id:05}"),
                    OFFICE,
                    NOW
                ],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO group_devices (device_id, group_id, added_at_utc) VALUES (?1, ?2, ?3)",
                params![id, group, NOW],
            )
            .unwrap();
        }
    }

    fn dev_place(conn: &Connection, id: i64) -> Option<i64> {
        conn.query_row(
            "SELECT place_id FROM devices WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn grp_place(conn: &Connection, id: i64) -> Option<i64> {
        conn.query_row(
            "SELECT place_id FROM groups WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn nested_block_moves_alone_and_root_stays() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let out = move_nested_block_in_tx(&tx, &GroupMoveDeps::new(), NESTED, STORE, None, NOW)
            .expect("move nested");
        tx.commit().unwrap();

        assert_eq!(grp_place(&conn, NESTED), Some(STORE));
        assert_eq!(dev_place(&conn, NESTED_ANCHOR), Some(STORE));
        assert_eq!(dev_place(&conn, NESTED_PART), Some(STORE));
        assert_eq!(grp_place(&conn, ROOT), Some(OFFICE), "корень не тронут");
        assert_eq!(
            dev_place(&conn, ROOT_DEV),
            Some(OFFICE),
            "член корня не тронут"
        );
        assert_eq!(out.moved_devices, 2);
        let batch = out.batch_id.expect("пакет");
        let rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM place_movements WHERE batch_id = ?1",
                params![batch],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rows, 3, "группа + 2 устройства одним пакетом");
        let audit: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_log WHERE entity_type = 'group' \
                 AND entity_id = ?1 AND action = 'move' AND payload_json LIKE '%\"nested\":true%'",
                params![NESTED],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(audit, 1);
    }

    #[test]
    fn nested_block_returns_to_parent_place() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let deps = GroupMoveDeps::new();
        move_nested_block_in_tx(&tx, &deps, NESTED, STORE, None, NOW).unwrap();
        move_nested_block_in_tx(&tx, &deps, NESTED, OFFICE, None, NOW).unwrap();
        tx.commit().unwrap();
        assert_eq!(grp_place(&conn, NESTED), Some(OFFICE));
        assert_eq!(dev_place(&conn, NESTED_PART), Some(OFFICE));
    }

    #[test]
    fn move_nested_block_rejects_root_group() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let err = move_nested_block_in_tx(&tx, &GroupMoveDeps::new(), ROOT, STORE, None, NOW)
            .expect_err("корень");
        match err {
            AppError::Validation { field, message } => {
                assert_eq!(field, "place_id");
                assert!(message.contains("корневой группы"), "{message}");
            }
            other => panic!("{other:?}"),
        }
    }
    // ---------------------------------------------------------------- D-34

    const SHELF: i64 = 4;
    const WORKSHOP: i64 = 5;
    const ALL_CODES: [&str; 6] = [
        "на_складе",
        "в_работе",
        "на_ремонте",
        "в_ремонте",
        "на_списание",
        "списано",
    ];

    /// `seed` + склад «Склад А» (STORE) с дочерним местом «Полка 3» и «Мастерская»
    /// (WORKSHOP); группы и три исходных устройства переставлены на `base`.
    fn seed_d34(conn: &Connection, base: i64) {
        seed(conn);
        conn.execute(
            "UPDATE places SET is_storage = 1 WHERE id = ?1",
            params![STORE],
        )
        .unwrap();
        for (id, parent, name) in [
            (SHELF, Some(STORE), "Полка 3"),
            (WORKSHOP, None, "Мастерская"),
        ] {
            conn.execute(
                "INSERT INTO places (id, parent_id, kind, name, is_storage, created_at_utc, \
                 updated_at_utc) VALUES (?1, ?2, 'room', ?3, 0, ?4, ?4)",
                params![id, parent, name, NOW],
            )
            .unwrap();
        }
        conn.execute("UPDATE groups SET place_id = ?1", params![base])
            .unwrap();
        conn.execute("UPDATE devices SET place_id = ?1", params![base])
            .unwrap();
    }

    fn add_device(conn: &Connection, id: i64, group: i64, place: i64, code: &str) {
        conn.execute(
            "INSERT INTO devices (id, type_id, name, inventory_number, place_id, status_id, \
             created_at_utc, updated_at_utc, version) VALUES (?1, 1, ?2, ?3, ?4, \
             (SELECT id FROM device_statuses WHERE code = ?5), ?6, ?6, 1)",
            params![
                id,
                format!("ПК-{id:05}"),
                format!("ИНВ-{id:05}"),
                place,
                code,
                NOW
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO group_devices (device_id, group_id, added_at_utc) VALUES (?1, ?2, ?3)",
            params![id, group, NOW],
        )
        .unwrap();
    }

    fn dev_code(conn: &Connection, id: i64) -> String {
        conn.query_row(
            "SELECT s.code FROM devices d JOIN device_statuses s ON s.id = d.status_id \
             WHERE d.id = ?1",
            params![id],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn snapshot(conn: &Connection) -> Vec<(i64, Option<i64>, String)> {
        let ids: Vec<i64> = conn
            .prepare("SELECT id FROM devices ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        ids.into_iter()
            .map(|id| (id, dev_place(conn, id), dev_code(conn, id)))
            .collect()
    }

    /// Устройства 10..21: по одному на каждую пару (код, «на месте группы»/«в мастерской»);
    /// чётные в корневой группе, нечётные во вложенной. Возвращает (id, код, у места группы).
    fn add_matrix(conn: &Connection, base: i64) -> Vec<(i64, &'static str, bool)> {
        let mut out = Vec::new();
        let mut id = 10;
        for code in ALL_CODES {
            for at_group in [true, false] {
                let group = if id % 2 == 0 { ROOT } else { NESTED };
                add_device(
                    conn,
                    id,
                    group,
                    if at_group { base } else { WORKSHOP },
                    code,
                );
                out.push((id, code, at_group));
                id += 1;
            }
        }
        out
    }

    fn expected_status(t: StorageTransition, code: &str) -> String {
        status_after_move(Some(t), code).unwrap_or(code).to_string()
    }

    fn run_matrix(from: i64, to: i64, t: StorageTransition) {
        let (mut conn, _g) = fresh_db();
        seed_d34(&conn, from);
        let matrix = add_matrix(&conn, from);
        let before = snapshot(&conn);
        let deps = GroupMoveDeps::new();

        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let predicted = status_shift_candidates_in_tx(&tx, &deps, ROOT, Some(from), to, t).unwrap();
        let out = move_group_with_status_in_tx(&tx, &deps, ROOT, None, to, None, NOW, true)
            .expect("move");
        tx.commit().unwrap();

        let after = snapshot(&conn);
        let mut actual_changed: Vec<i64> = before
            .iter()
            .zip(after.iter())
            .filter(|(b, a)| b.2 != a.2)
            .map(|(b, _)| b.0)
            .collect();
        actual_changed.sort();
        let mut p = predicted.clone();
        p.sort();
        let mut o = out.status_changed_device_ids.clone();
        o.sort();
        assert_eq!(p, actual_changed, "перечисление == фактическое");
        assert_eq!(o, actual_changed, "outcome == фактическое");

        for (id, code, at_group) in matrix {
            let exempt = is_repair_exempt_status_code(code);
            let moves = !exempt || at_group;
            let (_, place, new_code) = after.iter().find(|r| r.0 == id).unwrap();
            let (_, old_place, _) = before.iter().find(|r| r.0 == id).unwrap();
            if moves {
                assert_eq!(*place, Some(to), "{code} at_group={at_group}: место");
                assert_eq!(*new_code, expected_status(t, code), "{code}: статус");
            } else {
                assert_eq!(place, old_place, "{code}: остаётся на месте");
                assert_eq!(new_code, code, "{code}: статус не меняется");
            }
        }
        let audits: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_log WHERE action = 'custom:group_move_status_changed'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(audits as usize, actual_changed.len());
    }

    #[test]
    fn shift_to_work_table() {
        run_matrix(STORE, OFFICE, StorageTransition::ToWork);
    }

    #[test]
    fn shift_to_work_from_child_of_storage() {
        run_matrix(SHELF, OFFICE, StorageTransition::ToWork);
    }

    #[test]
    fn shift_to_storage_table() {
        run_matrix(OFFICE, STORE, StorageTransition::ToStorage);
    }

    #[test]
    fn shift_to_storage_child_place() {
        run_matrix(OFFICE, SHELF, StorageTransition::ToStorage);
    }

    #[test]
    fn exempt_status_at_foreign_place_stays_incl_write_off() {
        let (mut conn, _g) = fresh_db();
        seed_d34(&conn, STORE);
        add_device(&conn, 30, ROOT, WORKSHOP, "на_списание");
        add_device(&conn, 31, ROOT, STORE, "на_списание");
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        move_group_with_status_in_tx(
            &tx,
            &GroupMoveDeps::new(),
            ROOT,
            None,
            OFFICE,
            None,
            NOW,
            true,
        )
        .unwrap();
        tx.commit().unwrap();
        assert_eq!(dev_place(&conn, 30), Some(WORKSHOP));
        assert_eq!(dev_code(&conn, 30), "на_списание");
        assert_eq!(dev_place(&conn, 31), Some(OFFICE));
        assert_eq!(dev_code(&conn, 31), "на_списание");
    }

    #[test]
    fn open_repair_request_keeps_status_but_moves_place() {
        let (mut conn, _g) = fresh_db();
        seed_d34(&conn, STORE);
        add_device(&conn, 40, ROOT, STORE, "на_складе");
        add_device(&conn, 41, ROOT, STORE, "на_складе");
        conn.execute(
            "INSERT INTO users (login, full_name, role, created_at_utc, updated_at_utc) \
             VALUES ('ivanov', 'Иванов И.И.', 'employee', ?1, ?1)",
            params![NOW],
        )
        .unwrap();
        let user = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO requests (request_type, status, requested_by_user_id, \
             printer_device_id, created_at_utc, updated_at_utc) \
             VALUES ('repair', 'open', ?1, 40, ?2, ?2)",
            params![user, NOW],
        )
        .unwrap();
        let deps = GroupMoveDeps::new();
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let cands = status_shift_candidates_in_tx(
            &tx,
            &deps,
            ROOT,
            Some(STORE),
            OFFICE,
            StorageTransition::ToWork,
        )
        .unwrap();
        assert!(!cands.contains(&40) && cands.contains(&41), "{cands:?}");
        let out =
            move_group_with_status_in_tx(&tx, &deps, ROOT, None, OFFICE, None, NOW, true).unwrap();
        tx.commit().unwrap();
        assert_eq!(dev_place(&conn, 40), Some(OFFICE));
        assert_eq!(dev_code(&conn, 40), "на_складе");
        assert_eq!(dev_code(&conn, 41), "в_работе");
        assert!(!out.status_changed_device_ids.contains(&40));
        assert!(out.status_changed_device_ids.contains(&41));
    }

    #[test]
    fn no_storage_transition_or_flag_off_changes_no_status() {
        // Оба места не склад / оба склад / флаг выключен.
        for (from, to, flag) in [
            (OFFICE, WORKSHOP, true),
            (STORE, SHELF, true),
            (STORE, OFFICE, false),
            (OFFICE, STORE, false),
        ] {
            let (mut conn, _g) = fresh_db();
            seed_d34(&conn, from);
            add_matrix(&conn, from);
            let before: Vec<String> = snapshot(&conn).into_iter().map(|r| r.2).collect();
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            let out = move_group_with_status_in_tx(
                &tx,
                &GroupMoveDeps::new(),
                ROOT,
                None,
                to,
                None,
                NOW,
                flag,
            )
            .unwrap();
            tx.commit().unwrap();
            let after: Vec<String> = snapshot(&conn).into_iter().map(|r| r.2).collect();
            assert_eq!(before, after, "{from}->{to} flag={flag}");
            assert!(out.status_changed_device_ids.is_empty());
        }
    }

    #[test]
    fn legacy_wrappers_never_shift_status() {
        let (mut conn, _g) = fresh_db();
        seed_d34(&conn, STORE);
        add_device(&conn, 50, ROOT, STORE, "на_складе");
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let out =
            move_group_in_tx(&tx, &GroupMoveDeps::new(), ROOT, None, OFFICE, None, NOW).unwrap();
        tx.commit().unwrap();
        assert_eq!(dev_place(&conn, 50), Some(OFFICE));
        assert_eq!(dev_code(&conn, 50), "на_складе");
        assert!(out.status_changed_device_ids.is_empty());
    }

    #[test]
    fn missing_target_status_rolls_everything_back() {
        let (mut conn, _g) = fresh_db();
        seed_d34(&conn, STORE);
        add_device(&conn, 60, ROOT, STORE, "на_складе");
        add_device(&conn, 61, NESTED, STORE, "на_складе");
        conn.execute(
            "UPDATE device_statuses SET code = 'нет_такого' WHERE code = 'в_работе'",
            [],
        )
        .unwrap();
        let before = snapshot(&conn);
        let groups_before = (grp_place(&conn, ROOT), grp_place(&conn, NESTED));
        {
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .unwrap();
            let err = move_group_with_status_in_tx(
                &tx,
                &GroupMoveDeps::new(),
                ROOT,
                None,
                OFFICE,
                None,
                NOW,
                true,
            )
            .expect_err("нет статуса");
            assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
            // tx сбрасывается без commit: откат
        }
        assert_eq!(before, snapshot(&conn));
        assert_eq!(
            groups_before,
            (grp_place(&conn, ROOT), grp_place(&conn, NESTED))
        );
    }

    #[test]
    fn place_is_storage_table() {
        let (conn, _g) = fresh_db();
        seed_d34(&conn, OFFICE);
        assert!(place_is_storage_in_tx(&conn, STORE).unwrap());
        assert!(
            place_is_storage_in_tx(&conn, SHELF).unwrap(),
            "через предка"
        );
        assert!(!place_is_storage_in_tx(&conn, OFFICE).unwrap());
        assert!(!place_is_storage_in_tx(&conn, 9999).unwrap());
        conn.execute(
            "UPDATE places SET deleted_at_utc = ?1 WHERE id = ?2",
            params![NOW, STORE],
        )
        .unwrap();
        assert!(
            !place_is_storage_in_tx(&conn, SHELF).unwrap(),
            "удалённый склад не считается"
        );
    }
}
