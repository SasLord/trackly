//! Протаскивание места группы на состав (Phase 41, GRP-06, D-17/D-20/D-30).
//!
//! Здесь только примитивы на `&Transaction`: коммит делает вызывающий сервис, чтобы
//! перенос, запись пакета в журнал и аудит жили в ОДНОЙ транзакции writer'а.
//! Примитивы `pub(crate)`: их переиспользуют добавление в состав (план 11) и
//! массовый перенос места (план 15, D-23).

use rusqlite::Transaction;
use trackly_core::domain::place_movements::{MovementEntityKind, MovementSource};
use trackly_core::error::AppError;
use trackly_core::ports::places::PlaceRepository;
use trackly_infra::repos::audit_log_sqlite::{AuditEntry, SqliteAuditLogRepository};
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

/// Итог переноса группы.
#[derive(Debug, Default)]
pub(crate) struct GroupMoveOutcome {
    pub moved_devices: i32,
    pub moved_nested_groups: i32,
    /// Старые и новые места без дублей; пусто, если ничего не менялось.
    pub changed_place_ids: Vec<i64>,
    pub batch_id: Option<String>,
}

/// Итог применения места группы к одному устройству.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DevicePlaceChange {
    /// Место устройства действительно поменялось.
    pub changed: bool,
    /// Прежнее место (имеет смысл только при `changed`).
    pub previous_place: Option<i64>,
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
) -> Result<DevicePlaceChange, AppError> {
    let before = deps.devices.get_in_tx(tx, device_id)?;
    if before.place_id == Some(target_place) {
        return Ok(DevicePlaceChange {
            changed: false,
            previous_place: before.place_id,
        });
    }

    deps.devices.update_status_and_place_in_tx(
        tx,
        device_id,
        before.status_id,
        Some(target_place),
        now,
    )?;

    deps.movements.record_batch_movement_if_applicable(
        tx,
        &deps.places,
        MovementEntityKind::Device,
        device_id,
        before.place_id,
        Some(target_place),
        MovementSource::Group,
        None,
        None,
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
        MovementSource::Group,
        "вместе с принтером",
        user_id,
        now,
        batch_id,
        entity_label,
        Some(group_id),
    )?;

    Ok(DevicePlaceChange {
        changed: true,
        previous_place: before.place_id,
    })
}

fn push_unique(out: &mut Vec<i64>, id: i64) {
    if !out.contains(&id) {
        out.push(id);
    }
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

    if group.parent_group_id.is_some() {
        let root_id = deps.groups.root_group_id_in_tx(tx, group_id)?;
        let root = deps.groups.get_group_in_tx(tx, root_id)?;
        return Err(AppError::Validation {
            field: "place_id".to_string(),
            message: format!(
                "Место вложенной группы задаётся корневой группой «{}».",
                root.name
            ),
        });
    }

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

    let group_ids = deps.groups.subtree_group_ids_in_tx(tx, group_id)?;
    let device_ids = deps.groups.subtree_device_ids_in_tx(tx, group_id)?;
    let old_place = group.place_id;
    let group_moves = old_place != Some(target_place_id);

    let batch_id = uuid::Uuid::new_v4().to_string();
    let mut changed_places: Vec<i64> = Vec::new();

    if group_moves {
        // Место групп поддерева меняется ДО цикла устройств (денормализация, Pitfall 4).
        deps.groups
            .set_subtree_place_in_tx(tx, group_id, Some(target_place_id), now)?;
        deps.movements.record_batch_movement_if_applicable(
            tx,
            &deps.places,
            MovementEntityKind::Group,
            group_id,
            old_place,
            Some(target_place_id),
            MovementSource::Group,
            None,
            None,
            user_id,
            now,
            Some(&batch_id),
            Some(&group.name),
            Some(group_id),
        )?;
        if let Some(p) = old_place {
            push_unique(&mut changed_places, p);
        }
    }

    let mut moved_devices = 0;
    for device_id in &device_ids {
        let change = apply_group_place_to_device_in_tx(
            tx,
            deps,
            *device_id,
            target_place_id,
            Some(&batch_id),
            Some(&group.name),
            group_id,
            user_id,
            now,
        )?;
        if change.changed {
            moved_devices += 1;
            if let Some(p) = change.previous_place {
                push_unique(&mut changed_places, p);
            }
        }
    }

    if !group_moves && moved_devices == 0 {
        return Ok(GroupMoveOutcome::default());
    }
    push_unique(&mut changed_places, target_place_id);

    let payload = serde_json::json!({
        "batch_id": batch_id,
        "moved_devices": moved_devices,
        "from": old_place,
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

    Ok(GroupMoveOutcome {
        moved_devices,
        moved_nested_groups: if group_moves {
            (group_ids.len() as i32 - 1).max(0)
        } else {
            0
        },
        changed_place_ids: changed_places,
        batch_id: Some(batch_id),
    })
}
