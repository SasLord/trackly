//! Планировщик «целиком / часть» для передачи групп актами (Phase 41.2, GAM-01..GAM-03).
//!
//! Правило живёт ЗДЕСЬ и больше нигде: все пути актов (создание, правка, возврат,
//! удаление) спрашивают у этого модуля, что случится с группой, чьи устройства попали
//! в акт. Если пути ветвятся по группам сами, они расходятся (ловушка P-08).
//!
//! * Весь состав группы в акте (включая вложенные группы) — группа переезжает целиком,
//!   состав цел, выноса и подтверждения нет (D-04).
//! * Только часть состава — устройства акта выносятся из состава, нужно подтверждение (D-05).
//! * Якорная группа покрыта, когда в акте её якорь: группа едет за якорем с составом,
//!   якорь никогда не выносится (D-10, D-13).
//! * Вне CONTEXT (A9/OQ-1 в 41.2-RESEARCH.md): покрытая вложенная группа при непокрытом
//!   контейнере отцепляется от родителя и попадает в то же подтверждение. Это решение
//!   планировщика как расширение CONTEXT; пользователь его не подтверждал, оно вынесено
//!   в раздел «Решения вне CONTEXT» чек-листа 41.2-HUMAN-UAT.md.
//!
//! Файл НЕ входит в `SCOPE_FILES` реестра `entities_changed_gate`: гейт требует ровно
//! восемь файлов области и краснит строку реестра файла вне области. Модуль не владеет
//! `writer`-замыканием: все мутации идут на `&Transaction` внутри транзакции writer'а
//! сервиса актов (`act_service.rs`, он в области), и именно он рассылает
//! `EntitiesChanged`. Поэтому строк реестра для функций этого файла нет.
//!
//! Чистая часть (`plan`, `act_units`) не знает про БД и тестируется на вымышленных
//! снимках. Часть с БД работает только на `&Transaction`; коммит делает вызывающий.

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension, Transaction};
use trackly_core::domain::devices::DeviceRow;
use trackly_core::domain::groups::{
    ActGroupEffectKind, ActGroupEffectNew, ActGroupEffectRow, PlanGroupRow,
};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::audit_log_sqlite::{AuditEntry, SqliteAuditLogRepository};
use trackly_infra::repos::{SqliteGroupRepository, SqliteRepairRepository};

use crate::services::act_service::device_snapshot_json;
use crate::services::group_membership::{
    release_device_in_tx, restore_membership_with_policy_in_tx, GroupNotice, GroupRestoreConflict,
    PlaceConflictPolicy, ReleaseEffect, RestoreResult,
};
use crate::services::group_place::{propagate_group_place_in_tx, GroupMoveCause, GroupMoveDeps};

/// Снимок живых групп и членства для планировщика (читается одним набором запросов).
#[derive(Debug, Clone)]
pub(crate) struct ActGroupSnapshot {
    groups: Vec<PlanGroupRow>,
    by_id: HashMap<i64, usize>,
    children: HashMap<i64, Vec<i64>>,
    direct: HashMap<i64, Vec<i64>>,
    group_of_device: HashMap<i64, i64>,
    anchors: HashSet<i64>,
}

impl ActGroupSnapshot {
    /// Собрать снимок из живых групп и пар `(device_id, group_id)`.
    ///
    /// Группы упорядочиваются по id, устройства группы — по id устройства: порядок
    /// результатов планировщика детерминирован.
    pub(crate) fn new(mut groups: Vec<PlanGroupRow>, members: Vec<(i64, i64)>) -> Self {
        groups.sort_by_key(|g| g.id);
        let by_id: HashMap<i64, usize> =
            groups.iter().enumerate().map(|(i, g)| (g.id, i)).collect();
        let mut children: HashMap<i64, Vec<i64>> = HashMap::new();
        for g in &groups {
            if let Some(p) = g.parent_group_id {
                if by_id.contains_key(&p) {
                    children.entry(p).or_default().push(g.id);
                }
            }
        }
        let mut direct: HashMap<i64, Vec<i64>> = HashMap::new();
        let mut group_of_device: HashMap<i64, i64> = HashMap::new();
        let mut sorted_members = members;
        sorted_members.sort_unstable();
        for (device_id, group_id) in sorted_members {
            // Членство в мягко удалённой группе в снимок не попадает (читатель его не отдаёт),
            // но синтетический снимок в тестах мог бы его подать: пропускаем такие пары.
            if !by_id.contains_key(&group_id) {
                continue;
            }
            direct.entry(group_id).or_default().push(device_id);
            group_of_device.insert(device_id, group_id);
        }
        let anchors = groups
            .iter()
            .filter_map(|g| g.anchor_device_id)
            .collect::<HashSet<_>>();
        Self {
            groups,
            by_id,
            children,
            direct,
            group_of_device,
            anchors,
        }
    }

    fn group(&self, id: i64) -> Option<&PlanGroupRow> {
        self.by_id.get(&id).map(|i| &self.groups[*i])
    }

    fn children_of(&self, id: i64) -> &[i64] {
        self.children.get(&id).map(Vec::as_slice).unwrap_or(&[])
    }

    fn direct_of(&self, id: i64) -> &[i64] {
        self.direct.get(&id).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Группа и все её вложенные (id по возрастанию). Обход ограничен числом групп:
    /// цикл в испорченных данных не зацикливает.
    fn subtree_groups(&self, root: i64) -> Vec<i64> {
        let mut seen: HashSet<i64> = HashSet::new();
        let mut stack = vec![root];
        while let Some(g) = stack.pop() {
            if !seen.insert(g) {
                continue;
            }
            for c in self.children_of(g) {
                stack.push(*c);
            }
        }
        let mut out: Vec<i64> = seen.into_iter().collect();
        out.sort_unstable();
        out
    }
}

/// Отцепление вложенной группы от родителя (OQ-1): она делит место с корнем контейнера,
/// поэтому переезд невозможен без отцепления.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroupDetach {
    pub parent_group_id: i64,
    pub parent_name: String,
}

/// Группа, которая переезжает целиком.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroupMove {
    pub root_group_id: i64,
    /// Якорь переезжающей группы; `None` у контейнера. Возврат ищет по нему место возврата.
    pub anchor_device_id: Option<i64>,
    pub root_name: String,
    pub root_version: i64,
    pub from_place_id: Option<i64>,
    pub detach: Option<GroupDetach>,
    pub subtree_group_ids: Vec<i64>,
    pub subtree_device_ids: Vec<i64>,
    /// Место назначения, зафиксированное планировщиком возврата для КОНТЕЙНЕРА (D-44):
    /// общее место возврата его возвращаемых членов. `None` у якорных групп (цель берётся
    /// по месту возврата якоря) и у контейнера, чьи члены возвращаются без места.
    pub target_place_id: Option<i64>,
}

/// Устройство акта, выносимое из состава группы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeviceRelease {
    pub device_id: i64,
    /// Заполняет `plan_in_tx`; чистый `plan` оставляет пустую строку.
    pub device_name: String,
    pub group_id: i64,
    pub group_name: String,
}

/// Выбор менеджера для частично возвращаемой контейнерной группы (D-45).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroupSplitChoice {
    /// Возвращаемые члены выходят из группы, группа остаётся на месте (прежнее поведение).
    Release,
    /// Невозвращаемые прямые члены выходят из группы и остаются на месте, группа с
    /// возвращаемыми переезжает на место возврата.
    MoveGroup,
}

/// Вложенная непокрытая группа, которую надо отцепить при `GroupSplitChoice::MoveGroup`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SplitChild {
    pub group_id: i64,
    pub name: String,
    pub version: i64,
}

/// Контейнерная группа, у которой возвращена только часть состава (D-45).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroupSplit {
    pub group_id: i64,
    pub group_name: String,
    pub group_version: i64,
    pub returned_device_ids: Vec<i64>,
    pub kept_device_ids: Vec<i64>,
    /// Имена заполняет `plan_for_return_in_tx` (чистый планировщик оставляет пустые).
    pub returned_device_names: Vec<String>,
    pub kept_device_names: Vec<String>,
    /// Общее место возврата возвращаемых прямых членов и покрытых дочерних переносов, которые
    /// при `MoveGroup` едут вместе с группой (CR-02); `None`, если места не заданы или не совпали.
    pub target_place_id: Option<i64>,
    /// Места возвращаемых прямых членов и цели покрытых дочерних переносов совпали (все `None`
    /// тоже совпадение).
    pub places_match: bool,
    pub parent_group_id: Option<i64>,
    pub parent_name: Option<String>,
    pub from_place_id: Option<i64>,
    /// Глубина группы (корень = 0): `MoveGroup` применяет самые глубокие первыми.
    pub depth: usize,
    /// Непокрытые дочерние группы, которые `MoveGroup` отцепит и оставит на месте.
    pub uncovered_children: Vec<SplitChild>,
}

/// Целиком возвращаемая контейнерная группа с разными местами возврата членов (D-46).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroupPlaceConflict {
    pub group_id: i64,
    pub group_name: String,
    pub device_ids: Vec<i64>,
}

/// Итог планирования: что переезжает целиком и что выносится из состава.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ActGroupPlan {
    pub moves: Vec<GroupMove>,
    pub releases: Vec<DeviceRelease>,
    /// Только у плана возврата: частично возвращаемые контейнеры (D-45).
    pub splits: Vec<GroupSplit>,
    /// Только у плана возврата: целиком возвращаемые контейнеры с разными местами (D-46).
    pub place_conflicts: Vec<GroupPlaceConflict>,
}

impl ActGroupPlan {
    /// Нужно ли подтверждение менеджера: есть вынос ИЛИ отцепление вложенной группы.
    pub(crate) fn requires_confirmation(&self) -> bool {
        !self.releases.is_empty() || self.moves.iter().any(|m| m.detach.is_some())
    }

    /// Строки предупреждения (D-06, OQ-1): для выноса — имя устройства и имя группы;
    /// для отцепления — имя вложенной группы и имя родителя.
    pub(crate) fn warning_items(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = self
            .releases
            .iter()
            .map(|r| (r.device_name.clone(), r.group_name.clone()))
            .collect();
        for m in &self.moves {
            if let Some(d) = &m.detach {
                out.push((m.root_name.clone(), d.parent_name.clone()));
            }
        }
        out
    }

    /// D-46: целиком возвращаемая группа обязана возвращаться в одно место.
    pub(crate) fn ensure_places_match(&self) -> Result<(), AppError> {
        match self.place_conflicts.first() {
            None => Ok(()),
            Some(c) => Err(places_mismatch_error(&c.group_name)),
        }
    }
}

fn places_mismatch_error(group_name: &str) -> AppError {
    AppError::Validation {
        field: "place_id".to_string(),
        message: format!(
            "Группа «{group_name}» возвращается целиком: места возврата всех её устройств должны совпадать."
        ),
    }
}

/// Устройство, которое «разворачивает» группу в список позиций акта.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActUnit {
    pub device_id: i64,
    /// Имя вложенной группы, через которую пришло устройство; `None` для устройств корня.
    pub via_group_name: Option<String>,
}

fn covered(
    snap: &ActGroupSnapshot,
    group_id: i64,
    act: &HashSet<i64>,
    memo: &mut HashMap<i64, bool>,
    depth: usize,
) -> bool {
    if let Some(v) = memo.get(&group_id) {
        return *v;
    }
    // Защита от цикла в испорченных данных: глубже числа групп уйти нельзя.
    if depth > snap.groups.len() {
        return false;
    }
    let Some(group) = snap.group(group_id) else {
        return false;
    };
    let result = if let Some(anchor) = group.anchor_device_id {
        act.contains(&anchor)
    } else {
        let direct = snap.direct_of(group_id);
        let kids = snap.children_of(group_id);
        if direct.is_empty() && kids.is_empty() {
            false
        } else {
            direct.iter().all(|d| act.contains(d))
                && kids.iter().all(|c| covered(snap, *c, act, memo, depth + 1))
        }
    };
    memo.insert(group_id, result);
    result
}

/// Что делать с группами, если в акт попали устройства `act_device_ids`.
pub(crate) fn plan(snapshot: &ActGroupSnapshot, act_device_ids: &[i64]) -> ActGroupPlan {
    let act: HashSet<i64> = act_device_ids.iter().copied().collect();
    let mut memo: HashMap<i64, bool> = HashMap::new();
    let covered_ids: HashSet<i64> = snapshot
        .groups
        .iter()
        .filter(|g| covered(snapshot, g.id, &act, &mut memo, 0))
        .map(|g| g.id)
        .collect();

    let mut moves: Vec<GroupMove> = Vec::new();
    let mut moved_devices: HashSet<i64> = HashSet::new();
    for g in &snapshot.groups {
        if !covered_ids.contains(&g.id) {
            continue;
        }
        let live_parent = g
            .parent_group_id
            .and_then(|p| snapshot.group(p).map(|row| (p, row)));
        if let Some((p, _)) = live_parent {
            if covered_ids.contains(&p) {
                // Едет в поддереве покрытого родителя.
                continue;
            }
        }
        let detach = live_parent.map(|(p, row)| GroupDetach {
            parent_group_id: p,
            parent_name: row.name.clone(),
        });
        let subtree_group_ids = snapshot.subtree_groups(g.id);
        let mut subtree_device_ids: Vec<i64> = subtree_group_ids
            .iter()
            .flat_map(|gid| snapshot.direct_of(*gid).iter().copied())
            .collect();
        subtree_device_ids.sort_unstable();
        subtree_device_ids.dedup();
        moved_devices.extend(subtree_device_ids.iter().copied());
        moves.push(GroupMove {
            root_group_id: g.id,
            anchor_device_id: g.anchor_device_id,
            root_name: g.name.clone(),
            root_version: g.version,
            from_place_id: g.place_id,
            detach,
            subtree_group_ids,
            subtree_device_ids,
            target_place_id: None,
        });
    }

    let mut act_sorted: Vec<i64> = act.iter().copied().collect();
    act_sorted.sort_unstable();
    let mut releases: Vec<DeviceRelease> = Vec::new();
    for device_id in act_sorted {
        let Some(group_id) = snapshot.group_of_device.get(&device_id).copied() else {
            continue;
        };
        if moved_devices.contains(&device_id) {
            continue;
        }
        // Якорь не выносится никогда (D-13, D-14 фазы 41.1).
        if snapshot.anchors.contains(&device_id) {
            continue;
        }
        let group_name = snapshot
            .group(group_id)
            .map(|g| g.name.clone())
            .unwrap_or_default();
        releases.push(DeviceRelease {
            device_id,
            device_name: String::new(),
            group_id,
            group_name,
        });
    }

    ActGroupPlan {
        moves,
        releases,
        ..Default::default()
    }
}

/// Что делать с группами, если в возврат попали устройства `returned_device_ids`.
///
/// Покрытие считается так же, как у `plan` (якорная группа покрыта якорем, контейнер — когда
/// возвращены все его прямые члены и покрыты все дочерние). Корень перемещения — покрытая
/// группа без покрытого предка.
///
/// * Якорная группа едет к месту возврата якоря (цель считает `apply_return_plan_in_tx`).
/// * Целиком покрытый контейнер (D-44) едет как есть без подтверждения: цель — общее место
///   возврата его возвращаемых членов (все `None` — группа остаётся на месте). Разные места
///   (в том числе `None` против `Some`) — не split и не «место наименьшего id», а запись в
///   `place_conflicts` (D-46): вызывающий отклоняет возврат через `ensure_places_match`.
/// * Частично возвращаемый контейнер (D-45) не освобождается молча: он описан в `splits`,
///   применение зависит от `GroupSplitChoice`.
/// * Прочие возвращаемые члены групп (части якорных групп) — в `releases`, как раньше.
///   Покрытая вложенная группа при непокрытом родителе — корень с отцеплением (A9/OQ-1).
pub(crate) fn plan_for_return(
    snapshot: &ActGroupSnapshot,
    returned_device_ids: &[i64],
    return_places: &HashMap<i64, Option<i64>>,
) -> ActGroupPlan {
    let returned: HashSet<i64> = returned_device_ids.iter().copied().collect();
    let mut memo: HashMap<i64, bool> = HashMap::new();
    let covered_ids: HashSet<i64> = snapshot
        .groups
        .iter()
        .filter(|g| covered(snapshot, g.id, &returned, &mut memo, 0))
        .map(|g| g.id)
        .collect();

    let mut moves: Vec<GroupMove> = Vec::new();
    let mut place_conflicts: Vec<GroupPlaceConflict> = Vec::new();
    let mut riding_devices: HashSet<i64> = HashSet::new();
    for g in &snapshot.groups {
        if !covered_ids.contains(&g.id) {
            continue;
        }
        // Корень перемещения: ни один предок не является покрытой группой.
        let mut seen: HashSet<i64> = HashSet::new();
        let mut cur = g.parent_group_id;
        let mut under_covered = false;
        while let Some(p) = cur {
            if !seen.insert(p) {
                break;
            }
            if covered_ids.contains(&p) {
                under_covered = true;
                break;
            }
            cur = snapshot.group(p).and_then(|row| row.parent_group_id);
        }
        if under_covered {
            continue;
        }
        let live_parent = g
            .parent_group_id
            .and_then(|p| snapshot.group(p).map(|row| (p, row)));
        let detach = live_parent.map(|(p, row)| GroupDetach {
            parent_group_id: p,
            parent_name: row.name.clone(),
        });
        let subtree_group_ids = snapshot.subtree_groups(g.id);
        let mut subtree_device_ids: Vec<i64> = subtree_group_ids
            .iter()
            .flat_map(|gid| snapshot.direct_of(*gid).iter().copied())
            .collect();
        subtree_device_ids.sort_unstable();
        subtree_device_ids.dedup();
        riding_devices.extend(subtree_device_ids.iter().copied());

        let mut target_place_id = None;
        if g.anchor_device_id.is_none() {
            // Контейнер: места возврата всех возвращаемых устройств поддерева.
            let devs: Vec<i64> = subtree_device_ids
                .iter()
                .copied()
                .filter(|d| returned.contains(d))
                .collect();
            let places: HashSet<Option<i64>> = devs
                .iter()
                .map(|d| return_places.get(d).copied().flatten())
                .collect();
            if places.len() > 1 {
                place_conflicts.push(GroupPlaceConflict {
                    group_id: g.id,
                    group_name: g.name.clone(),
                    device_ids: devs,
                });
                continue;
            }
            target_place_id = places.into_iter().next().flatten();
        }
        moves.push(GroupMove {
            root_group_id: g.id,
            anchor_device_id: g.anchor_device_id,
            root_name: g.name.clone(),
            root_version: g.version,
            from_place_id: g.place_id,
            detach,
            subtree_group_ids,
            subtree_device_ids,
            target_place_id,
        });
    }

    // Частично возвращаемые контейнеры (не покрытые, вне переезжающих поддеревьев).
    let mut splits: Vec<GroupSplit> = Vec::new();
    let mut split_devices: HashSet<i64> = HashSet::new();
    for g in &snapshot.groups {
        if g.anchor_device_id.is_some() || covered_ids.contains(&g.id) {
            continue;
        }
        let returned_direct: Vec<i64> = snapshot
            .direct_of(g.id)
            .iter()
            .copied()
            .filter(|d| {
                returned.contains(d) && !snapshot.anchors.contains(d) && !riding_devices.contains(d)
            })
            .collect();
        if returned_direct.is_empty() {
            continue;
        }
        let kept: Vec<i64> = snapshot
            .direct_of(g.id)
            .iter()
            .copied()
            .filter(|d| !returned_direct.contains(d))
            .collect();
        let mut places: HashSet<Option<i64>> = returned_direct
            .iter()
            .map(|d| return_places.get(d).copied().flatten())
            .collect();
        // CR-02 (D-44/D-46): покрытые дочерние переносы при `MoveGroup` едут вместе с группой
        // (`apply_split_move_in_tx`) на её место. Их собственная цель (место возврата якоря у
        // якорной группы, общее место возвращаемых устройств у контейнера) входит в сравнение
        // мест: иначе место возврата, указанное для них пользователем, молча заменилось бы
        // местом родителя.
        for m in moves
            .iter()
            .filter(|m| m.detach.as_ref().is_some_and(|d| d.parent_group_id == g.id))
        {
            places.insert(return_move_target(m, return_places));
        }
        let places_match = places.len() == 1;
        let target = if places_match {
            places.into_iter().next().flatten()
        } else {
            None
        };
        let parent = g
            .parent_group_id
            .and_then(|p| snapshot.group(p).map(|row| (p, row)));
        let mut depth = 0usize;
        let mut seen: HashSet<i64> = HashSet::new();
        let mut cur = g.parent_group_id;
        while let Some(p) = cur {
            if !seen.insert(p) || snapshot.group(p).is_none() {
                break;
            }
            depth += 1;
            cur = snapshot.group(p).and_then(|row| row.parent_group_id);
        }
        split_devices.extend(returned_direct.iter().copied());
        splits.push(GroupSplit {
            group_id: g.id,
            group_name: g.name.clone(),
            group_version: g.version,
            returned_device_ids: returned_direct,
            kept_device_ids: kept,
            returned_device_names: Vec::new(),
            kept_device_names: Vec::new(),
            target_place_id: target,
            places_match,
            parent_group_id: parent.map(|(p, _)| p),
            parent_name: parent.map(|(_, row)| row.name.clone()),
            from_place_id: g.place_id,
            depth,
            uncovered_children: Vec::new(),
        });
    }
    // Непокрытые дочерние группы, которые MoveGroup отцепит. Ребёнок со своим переезжающим
    // разбиением отцепится сам при своём переносе.
    let moving_splits: HashSet<i64> = splits
        .iter()
        .filter(|s| s.target_place_id.is_some())
        .map(|s| s.group_id)
        .collect();
    for s in &mut splits {
        s.uncovered_children = snapshot
            .children_of(s.group_id)
            .iter()
            .filter(|c| !covered_ids.contains(c) && !moving_splits.contains(c))
            .filter_map(|c| snapshot.group(*c))
            .map(|row| SplitChild {
                group_id: row.id,
                name: row.name.clone(),
                version: row.version,
            })
            .collect();
    }

    let mut returned_sorted: Vec<i64> = returned.iter().copied().collect();
    returned_sorted.sort_unstable();
    let mut releases: Vec<DeviceRelease> = Vec::new();
    for device_id in returned_sorted {
        let Some(group_id) = snapshot.group_of_device.get(&device_id).copied() else {
            continue;
        };
        if riding_devices.contains(&device_id)
            || split_devices.contains(&device_id)
            || snapshot.anchors.contains(&device_id)
        {
            continue;
        }
        let group_name = snapshot
            .group(group_id)
            .map(|g| g.name.clone())
            .unwrap_or_default();
        releases.push(DeviceRelease {
            device_id,
            device_name: String::new(),
            group_id,
            group_name,
        });
    }
    ActGroupPlan {
        moves,
        releases,
        splits,
        place_conflicts,
    }
}

/// Плоский список позиций, который «разворачивает» группу в акт: прямые устройства,
/// затем вложенные группы рекурсивно; якорная группа — один юнит (якорь) вместо
/// всех её деталей. Пустая группа даёт пустой список.
///
/// Свойство согласованности: `plan(snapshot, ids(act_units(G)))` для непустой группы
/// даёт `moves == [G]` и пустые `releases`.
pub(crate) fn act_units(snapshot: &ActGroupSnapshot, root_group_id: i64) -> Vec<ActUnit> {
    let mut out: Vec<ActUnit> = Vec::new();
    let mut visited: HashSet<i64> = HashSet::new();
    collect_units(snapshot, root_group_id, None, &mut visited, &mut out);
    out
}

fn collect_units(
    snap: &ActGroupSnapshot,
    group_id: i64,
    via: Option<&str>,
    visited: &mut HashSet<i64>,
    out: &mut Vec<ActUnit>,
) {
    if !visited.insert(group_id) {
        return;
    }
    let Some(group) = snap.group(group_id) else {
        return;
    };
    if let Some(anchor) = group.anchor_device_id {
        out.push(ActUnit {
            device_id: anchor,
            via_group_name: via.map(str::to_string),
        });
        return;
    }
    for d in snap.direct_of(group_id) {
        out.push(ActUnit {
            device_id: *d,
            via_group_name: via.map(str::to_string),
        });
    }
    let mut kids: Vec<&PlanGroupRow> = snap
        .children_of(group_id)
        .iter()
        .filter_map(|c| snap.group(*c))
        .collect();
    kids.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    for kid in kids {
        collect_units(snap, kid.id, Some(&kid.name), visited, out);
    }
}

/// Прочитать снимок групп. Принимает `&Connection`, поэтому годится и читателю сервиса
/// разворота, и транзакции writer'а (`Transaction` разыменовывается в `Connection`).
pub(crate) fn load_snapshot(
    conn: &Connection,
    groups: &SqliteGroupRepository,
) -> Result<ActGroupSnapshot, AppError> {
    let rows = groups.list_live_groups_for_plan(conn)?;
    let members = groups.list_live_memberships(conn)?;
    Ok(ActGroupSnapshot::new(rows, members))
}

/// Текст отказа включения в акт устройства с открытой ремонтной заявкой (D-22).
pub(crate) const REPAIR_OPEN_ACT_MESSAGE: &str = "Открыта заявка на ремонт — в акт не включить.";
/// Текст отказа возврата и отката акта для такого устройства (D-22).
pub(crate) const REPAIR_OPEN_RETURN_MESSAGE: &str =
    "Нельзя вернуть или откатить акт: по устройству открыта заявка на ремонт.";

/// Отказ `Conflict` с `message`, если у любого из `device_ids` есть открытая ремонтная
/// заявка (предикат `open_repair_requests_for_devices`, единый для D-19/D-21/D-22).
pub(crate) fn ensure_no_open_repair_in_tx(
    tx: &Transaction<'_>,
    device_ids: &[i64],
    message: &str,
) -> Result<(), AppError> {
    let open = SqliteRepairRepository.open_repair_requests_for_devices(tx, device_ids)?;
    if open.is_empty() {
        return Ok(());
    }
    Err(AppError::Conflict {
        reason: message.to_string(),
    })
}

/// План для списка устройств акта внутри транзакции writer'а: загрузка + `plan` +
/// имена выносимых устройств одним запросом. Клиентский список не определяет, какие
/// группы переезжают: план всегда пересчитывается здесь (T-41.2-13).
pub(crate) fn plan_in_tx(
    tx: &Transaction<'_>,
    act_device_ids: &[i64],
) -> Result<ActGroupPlan, AppError> {
    let snapshot = load_snapshot(tx, &SqliteGroupRepository)?;
    let mut result = plan(&snapshot, act_device_ids);
    // Phase 41.4 (D-22): расширение группы в акте не должно втянуть устройство с открытой
    // ремонтной заявкой — отказ целого акта, а не молча неполный акт.
    let mut touched: Vec<i64> = act_device_ids.to_vec();
    for mv in &result.moves {
        touched.extend(mv.subtree_device_ids.iter().copied());
    }
    touched.extend(result.releases.iter().map(|r| r.device_id));
    ensure_no_open_repair_in_tx(tx, &touched, REPAIR_OPEN_ACT_MESSAGE)?;
    if result.releases.is_empty() {
        return Ok(result);
    }
    let ids: Vec<i64> = result.releases.iter().map(|r| r.device_id).collect();
    let ids_json = serde_json::to_string(&ids).map_err(|e| AppError::Internal {
        source_chain: format!("group_act_plan: ids json: {e}"),
    })?;
    let mut stmt = tx
        .prepare("SELECT id, name FROM devices WHERE id IN (SELECT value FROM json_each(?1))")
        .map_err(map_rusqlite)?;
    let names: HashMap<i64, String> = stmt
        .query_map(rusqlite::params![ids_json], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(map_rusqlite)?
        .collect::<rusqlite::Result<HashMap<_, _>>>()
        .map_err(map_rusqlite)?;
    for r in &mut result.releases {
        if let Some(n) = names.get(&r.device_id) {
            r.device_name = n.clone();
        }
    }
    Ok(result)
}

/// Имена устройств по id одним запросом (json_each).
fn device_names_in_tx(tx: &Transaction<'_>, ids: &[i64]) -> Result<HashMap<i64, String>, AppError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let ids_json = serde_json::to_string(ids).map_err(|e| AppError::Internal {
        source_chain: format!("group_act_plan: ids json: {e}"),
    })?;
    let mut stmt = tx
        .prepare("SELECT id, name FROM devices WHERE id IN (SELECT value FROM json_each(?1))")
        .map_err(map_rusqlite)?;
    let names = stmt
        .query_map(rusqlite::params![ids_json], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(map_rusqlite)?
        .collect::<rusqlite::Result<HashMap<_, _>>>()
        .map_err(map_rusqlite)?;
    Ok(names)
}

/// План групп для возврата внутри транзакции writer'а: загрузка снимка + `plan_for_return`
/// и имена устройств разбиений и выносов. Клиентский список не определяет, какие группы
/// переезжают (T-41.2-13).
pub(crate) fn plan_for_return_in_tx(
    tx: &Transaction<'_>,
    returned_device_ids: &[i64],
    return_places: &HashMap<i64, Option<i64>>,
) -> Result<ActGroupPlan, AppError> {
    let snapshot = load_snapshot(tx, &SqliteGroupRepository)?;
    let mut result = plan_for_return(&snapshot, returned_device_ids, return_places);
    // Phase 41.4 (D-22): возврат устройства с открытой ремонтной заявкой — отказ.
    let mut touched: Vec<i64> = returned_device_ids.to_vec();
    for mv in &result.moves {
        touched.extend(mv.subtree_device_ids.iter().copied());
    }
    ensure_no_open_repair_in_tx(tx, &touched, REPAIR_OPEN_RETURN_MESSAGE)?;
    let mut ids: Vec<i64> = result.releases.iter().map(|r| r.device_id).collect();
    for s in &result.splits {
        ids.extend(s.returned_device_ids.iter().copied());
        ids.extend(s.kept_device_ids.iter().copied());
    }
    if ids.is_empty() {
        return Ok(result);
    }
    let names = device_names_in_tx(tx, &ids)?;
    let name_of = |id: &i64| names.get(id).cloned().unwrap_or_default();
    for r in &mut result.releases {
        r.device_name = name_of(&r.device_id);
    }
    for s in &mut result.splits {
        s.returned_device_names = s.returned_device_ids.iter().map(name_of).collect();
        s.kept_device_names = s.kept_device_ids.iter().map(name_of).collect();
    }
    Ok(result)
}

/// Применить план возврата: якорные группы едут к месту возврата своего якоря, покрытые
/// контейнеры — к общему месту возврата членов (D-44), прочие возвращаемые члены групп
/// освобождаются (с эффектом `release` возвратного акта); частично возвращаемый контейнер
/// обрабатывается по `choice` (D-45).
///
/// `return_places` — место возврата каждого возвращаемого устройства из позиций акта.
/// Якорь без места возврата (и контейнер без общего места) не двигает группу: она остаётся
/// на месте, её устройства попадают в `stationary_device_ids`. Перенос идёт отдельным вызовом
/// `apply_plan_in_tx` на каждую группу, потому что у каждой своё место назначения.
///
/// `MoveGroup` при `places_match = false` отклоняется до любой записи (D-46). Порядок:
/// самые глубокие разбиения первыми; невозвращаемые прямые члены выходят из группы ДО
/// переноса (иначе группа унесла бы их с собой).
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_return_plan_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    act_id: i64,
    plan: &ActGroupPlan,
    return_places: &HashMap<i64, Option<i64>>,
    choice: GroupSplitChoice,
    item_device_ids: &[i64],
    user_id: Option<i64>,
    now: i64,
) -> Result<AppliedGroupPlan, AppError> {
    plan.ensure_places_match()?;
    if choice == GroupSplitChoice::MoveGroup {
        if let Some(s) = plan.splits.iter().find(|s| !s.places_match) {
            return Err(places_mismatch_error(&s.group_name));
        }
    }
    // Разбиения, которые реально переезжают: их покрытые дочерние переезды едут вместе.
    let moving_splits: HashSet<i64> = if choice == GroupSplitChoice::MoveGroup {
        plan.splits
            .iter()
            .filter(|s| s.target_place_id.is_some())
            .map(|s| s.group_id)
            .collect()
    } else {
        HashSet::new()
    };

    let mut total = AppliedGroupPlan::default();
    for mv in &plan.moves {
        if mv
            .detach
            .as_ref()
            .is_some_and(|d| moving_splits.contains(&d.parent_group_id))
        {
            continue; // едет с группой-разбиением
        }
        let Some(target) = return_move_target(mv, return_places) else {
            total
                .stationary_device_ids
                .extend(mv.subtree_device_ids.iter().copied());
            continue;
        };
        let single = ActGroupPlan {
            moves: vec![mv.clone()],
            ..Default::default()
        };
        let part = apply_plan_in_tx(
            tx,
            deps,
            act_id,
            &single,
            Some(target),
            item_device_ids,
            user_id,
            now,
        )?;
        absorb_applied(&mut total, part);
    }

    let mut releases = plan.releases.clone();
    match choice {
        GroupSplitChoice::Release => {
            for s in &plan.splits {
                for (i, d) in s.returned_device_ids.iter().enumerate() {
                    releases.push(DeviceRelease {
                        device_id: *d,
                        device_name: s.returned_device_names.get(i).cloned().unwrap_or_default(),
                        group_id: s.group_id,
                        group_name: s.group_name.clone(),
                    });
                }
            }
        }
        GroupSplitChoice::MoveGroup => {
            let mut ordered: Vec<&GroupSplit> = plan.splits.iter().collect();
            ordered.sort_by(|a, b| b.depth.cmp(&a.depth).then(a.group_id.cmp(&b.group_id)));
            for s in ordered {
                let Some(target) = s.target_place_id else {
                    total
                        .stationary_device_ids
                        .extend(s.returned_device_ids.iter().copied());
                    continue;
                };
                let part = apply_split_move_in_tx(
                    tx,
                    deps,
                    act_id,
                    plan,
                    s,
                    target,
                    item_device_ids,
                    user_id,
                    now,
                )?;
                absorb_applied(&mut total, part);
            }
        }
    }
    if !releases.is_empty() {
        let only_releases = ActGroupPlan {
            releases,
            ..Default::default()
        };
        let part = apply_plan_in_tx(
            tx,
            deps,
            act_id,
            &only_releases,
            None,
            item_device_ids,
            user_id,
            now,
        )?;
        absorb_applied(&mut total, part);
    }
    total.touched_place_ids.sort_unstable();
    total.touched_group_ids.sort_unstable();
    Ok(total)
}

/// Цель покрытого переноса при возврате: якорная группа едет к месту возврата якоря, контейнер —
/// к общему месту возврата своих возвращаемых устройств (`GroupMove::target_place_id`).
/// `None` — группа остаётся на месте. Общая для применения плана и для сравнения мест
/// разбиения, чьи покрытые дочерние переносы едут вместе с ним (CR-02).
fn return_move_target(mv: &GroupMove, return_places: &HashMap<i64, Option<i64>>) -> Option<i64> {
    match mv.anchor_device_id {
        Some(anchor) => return_places.get(&anchor).copied().flatten(),
        None => mv.target_place_id,
    }
}

/// `MoveGroup` для одного разбиения: kept-члены выходят (эффект `release`), непокрытые
/// дочерние и сама группа отцепляются (эффект `detach`), затем группа с возвращаемыми
/// членами и покрытыми дочерними переезжает на `target`.
#[allow(clippy::too_many_arguments)]
fn apply_split_move_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    act_id: i64,
    plan: &ActGroupPlan,
    split: &GroupSplit,
    target: i64,
    item_device_ids: &[i64],
    user_id: Option<i64>,
    now: i64,
) -> Result<AppliedGroupPlan, AppError> {
    let mut touched_groups: Vec<i64> = vec![split.group_id];
    for d in &split.kept_device_ids {
        release_device_in_tx(
            tx,
            &deps.groups,
            &deps.audit,
            *d,
            Some(act_id),
            user_id,
            now,
        )?;
    }
    for child in &split.uncovered_children {
        let mut effect = effect_new(
            act_id,
            ActGroupEffectKind::Detach,
            child.group_id,
            &child.name,
            now,
        );
        effect.from_parent_group_id = Some(split.group_id);
        deps.groups.insert_act_group_effect_in_tx(tx, &effect)?;
        deps.groups
            .set_parent_in_tx(tx, child.group_id, child.version, None, now)?;
        touched_groups.push(child.group_id);
    }
    if let Some(parent) = split.parent_group_id {
        let mut effect = effect_new(
            act_id,
            ActGroupEffectKind::Detach,
            split.group_id,
            &split.group_name,
            now,
        );
        effect.from_parent_group_id = Some(parent);
        deps.groups.insert_act_group_effect_in_tx(tx, &effect)?;
        deps.groups
            .set_parent_in_tx(tx, split.group_id, split.group_version, None, now)?;
        touched_groups.push(parent);
    }

    // Покрытые дочерние переезды едут вместе с группой.
    let mut device_ids: Vec<i64> = split.returned_device_ids.clone();
    let mut group_ids: Vec<i64> = vec![split.group_id];
    for mv in &plan.moves {
        if mv
            .detach
            .as_ref()
            .is_some_and(|d| d.parent_group_id == split.group_id)
        {
            device_ids.extend(mv.subtree_device_ids.iter().copied());
            group_ids.extend(mv.subtree_group_ids.iter().copied());
        }
    }
    device_ids.sort_unstable();
    device_ids.dedup();
    group_ids.sort_unstable();
    group_ids.dedup();
    let synthetic = GroupMove {
        root_group_id: split.group_id,
        anchor_device_id: None,
        root_name: split.group_name.clone(),
        root_version: split.group_version,
        from_place_id: split.from_place_id,
        detach: None,
        subtree_group_ids: group_ids,
        subtree_device_ids: device_ids,
        target_place_id: Some(target),
    };
    let single = ActGroupPlan {
        moves: vec![synthetic],
        ..Default::default()
    };
    let mut applied = apply_plan_in_tx(
        tx,
        deps,
        act_id,
        &single,
        Some(target),
        item_device_ids,
        user_id,
        now,
    )?;
    for g in touched_groups {
        push_unique(&mut applied.touched_group_ids, g);
    }
    Ok(applied)
}

fn absorb_applied(total: &mut AppliedGroupPlan, part: AppliedGroupPlan) {
    for (id, row) in part.pre_snapshots {
        total.pre_snapshots.entry(id).or_insert(row);
    }
    total.moved_device_ids.extend(part.moved_device_ids);
    for place in part.touched_place_ids {
        push_unique(&mut total.touched_place_ids, place);
    }
    for group in part.touched_group_ids {
        push_unique(&mut total.touched_group_ids, group);
    }
}

/// Итог применения плана внутри транзакции акта.
#[derive(Debug, Default)]
pub(crate) struct AppliedGroupPlan {
    /// Состояние устройств переехавших поддеревьев ДО переноса: `before_json` аудита
    /// позиций акта обязан хранить его (ловушка P-04), иначе откат вернёт уже новое место.
    pub pre_snapshots: HashMap<i64, DeviceRow>,
    /// Все устройства переехавших поддеревьев: вызывающий НЕ пишет им отдельную строку
    /// журнала места и НЕ вызывает release (их место уже поставила группа).
    pub moved_device_ids: HashSet<i64>,
    /// Устройства якорных групп, чей перенос возврат пропустил из-за незаданного места:
    /// группа осталась на месте, поэтому место устройства определяет она, а не позиция акта.
    pub stationary_device_ids: HashSet<i64>,
    /// Прежние и новое места групп и устройств (включая место ПУСТОЙ группы), без дублей.
    pub touched_place_ids: Vec<i64>,
    /// Корни переехавших групп, отцепленные группы и группы-источники выноса.
    pub touched_group_ids: Vec<i64>,
}

/// Итог обратных операций: уведомления и затронутые места и группы.
#[derive(Debug, Default)]
pub(crate) struct GroupRevertOutcome {
    pub notices: Vec<GroupNotice>,
    /// Конфликты «группа перенесена после акта», по которым нет выбора пользователя (D-42).
    pub conflicts: Vec<GroupRestoreConflict>,
    pub touched_place_ids: Vec<i64>,
    pub touched_group_ids: Vec<i64>,
    /// D-48: корни групп, чьё место после отката авторитетно для членов (перенос «забыт» и
    /// группа остаётся там, где стоит, либо группа возвращена на прежнее место): снимки
    /// их членов восстанавливаются с местом группы вместо записанного в снимке.
    pub kept_in_place_group_ids: Vec<i64>,
}

fn push_unique(out: &mut Vec<i64>, id: i64) {
    if !out.contains(&id) {
        out.push(id);
    }
}

/// Выполнить `f` в точке сохранения: ошибка откатывает только её запись, а не всю
/// транзакцию акта. Нужен обратным операциям: ошибка одного эффекта не должна
/// прерывать откат (она превращается в уведомление).
fn guarded<T>(
    tx: &Transaction<'_>,
    f: impl FnOnce() -> Result<T, AppError>,
) -> Result<T, AppError> {
    tx.execute_batch("SAVEPOINT group_act_effect")
        .map_err(map_rusqlite)?;
    match f() {
        Ok(v) => {
            tx.execute_batch("RELEASE group_act_effect")
                .map_err(map_rusqlite)?;
            Ok(v)
        }
        Err(e) => {
            let _ = tx.execute_batch("ROLLBACK TO group_act_effect; RELEASE group_act_effect");
            Err(e)
        }
    }
}

fn effect_new(
    act_id: i64,
    kind: ActGroupEffectKind,
    group_id: i64,
    label: &str,
    now: i64,
) -> ActGroupEffectNew {
    ActGroupEffectNew {
        act_id,
        kind,
        group_id,
        group_label: label.to_string(),
        device_id: None,
        from_place_id: None,
        to_place_id: None,
        from_parent_group_id: None,
        created_at_utc: now,
    }
}

/// Применить план групп внутри транзакции акта (коммит делает вызывающий сервис).
///
/// Порядок строго такой (ловушка P-04): (1) снимок «до» устройств поддеревьев;
/// (2) аудит `custom:group_moved_by_act` для деталей, которых нет среди позиций акта
/// (откат подхватит их по `select_device_mutations_for_act` без нового кода);
/// (3) отцепление и перенос каждой группы с причиной `GroupMoveCause::act`, так что строки
/// группы, устройств и картриджей принтера-члена несут `source = 'act'`, `act_id` и
/// общий `batch_id` (D-14, D-15); (4) вынос устройств из состава со следом `release`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_plan_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    act_id: i64,
    plan: &ActGroupPlan,
    target_place: Option<i64>,
    item_device_ids: &[i64],
    user_id: Option<i64>,
    now: i64,
) -> Result<AppliedGroupPlan, AppError> {
    let mut applied = AppliedGroupPlan::default();
    if !plan.moves.is_empty() && target_place.is_none() {
        return Err(AppError::Validation {
            field: "place_id".to_string(),
            message: "Для передачи группы укажите место".to_string(),
        });
    }
    let items: HashSet<i64> = item_device_ids.iter().copied().collect();

    // (1) снимки «до» — раньше любого переноса.
    for mv in &plan.moves {
        for device_id in &mv.subtree_device_ids {
            if !applied.pre_snapshots.contains_key(device_id) {
                let row = deps.devices.get_in_tx(tx, *device_id)?;
                applied.pre_snapshots.insert(*device_id, row);
            }
            applied.moved_device_ids.insert(*device_id);
        }
    }

    // (2) детали, не являющиеся позициями акта: аудит со снимком «до».
    let mut parts: Vec<i64> = applied
        .moved_device_ids
        .iter()
        .copied()
        .filter(|d| !items.contains(d))
        .collect();
    parts.sort_unstable();
    for device_id in parts {
        let row = &applied.pre_snapshots[&device_id];
        let before = device_snapshot_json(row).map_err(|e| AppError::Internal {
            source_chain: format!("group_act_plan: snapshot json: {e}"),
        })?;
        deps.audit.insert(
            tx,
            AuditEntry {
                entity_type: "device",
                entity_id: device_id,
                action: "custom:group_moved_by_act",
                user_id,
                before_json: Some(before),
                after_json: None,
                payload_json: Some(serde_json::json!({ "act_id": act_id }).to_string()),
                created_at_utc: now,
            },
        )?;
    }

    // (3) отцепление и перенос.
    for mv in &plan.moves {
        let target = target_place.expect("проверено выше: у плана с переносами место задано");
        if let Some(detach) = &mv.detach {
            let mut effect = effect_new(
                act_id,
                ActGroupEffectKind::Detach,
                mv.root_group_id,
                &mv.root_name,
                now,
            );
            effect.from_parent_group_id = Some(detach.parent_group_id);
            deps.groups.insert_act_group_effect_in_tx(tx, &effect)?;
            deps.groups
                .set_parent_in_tx(tx, mv.root_group_id, mv.root_version, None, now)?;
            push_unique(&mut applied.touched_group_ids, detach.parent_group_id);
        }
        let mut effect = effect_new(
            act_id,
            ActGroupEffectKind::Move,
            mv.root_group_id,
            &mv.root_name,
            now,
        );
        effect.from_place_id = mv.from_place_id;
        effect.to_place_id = Some(target);
        deps.groups.insert_act_group_effect_in_tx(tx, &effect)?;

        let batch_id = uuid::Uuid::new_v4().to_string();
        let outcome = propagate_group_place_in_tx(
            tx,
            deps,
            mv.root_group_id,
            target,
            mv.from_place_id,
            &batch_id,
            user_id,
            now,
            GroupMoveCause::act(act_id),
        )?;
        for place in outcome.changed_place_ids {
            push_unique(&mut applied.touched_place_ids, place);
        }
        if let Some(p) = mv.from_place_id {
            push_unique(&mut applied.touched_place_ids, p);
        }
        push_unique(&mut applied.touched_place_ids, target);
        push_unique(&mut applied.touched_group_ids, mv.root_group_id);
    }

    // (4) вынос из состава; след release пишет сам примитив (act_id задан).
    for release in &plan.releases {
        if release_device_in_tx(
            tx,
            &deps.groups,
            &deps.audit,
            release.device_id,
            Some(act_id),
            user_id,
            now,
        )?
        .is_some()
        {
            push_unique(&mut applied.touched_group_ids, release.group_id);
        }
    }

    applied.touched_place_ids.sort_unstable();
    applied.touched_group_ids.sort_unstable();
    Ok(applied)
}

/// Обратить переносы и отцепления групп актом (порядок отката: группы -> снимки -> членство,
/// P-05; вызывается ДО восстановления снимков устройств). Обходит эффекты акта по убыванию id.
///
/// `move`: группа жива, её место равно `to_place_id` (не двигалась с тех пор) и она корневая —
/// возвращается на `from_place_id` тем же протаскиванием с `GroupMoveCause::act` (строки
/// обратного переноса удалит `delete_by_act_id` вызывающего); иначе уведомление.
/// `detach` (ПОСЛЕ обращения `move` той же группы): родитель жив, группа корневая и её место
/// совпадает с местом родителя — вложенность восстанавливается; иначе уведомление. Ошибка
/// отдельного эффекта не прерывает откат: она превращается в уведомление.
pub(crate) fn revert_group_moves_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    act_id: i64,
    user_id: Option<i64>,
    now: i64,
) -> Result<GroupRevertOutcome, AppError> {
    let mut out = GroupRevertOutcome::default();
    let effects = deps.groups.list_act_group_effects_in_tx(tx, act_id)?;
    for effect in effects.iter().rev() {
        match effect.kind {
            ActGroupEffectKind::Move => {
                absorb(
                    &mut out,
                    revert_move_effect_in_tx(tx, deps, effect, user_id, now)?,
                );
            }
            ActGroupEffectKind::Detach => {
                absorb(
                    &mut out,
                    revert_detach_effect_in_tx(tx, deps, effect, user_id, now)?,
                );
            }
            ActGroupEffectKind::Release => {}
        }
    }
    out.touched_place_ids.sort_unstable();
    out.touched_group_ids.sort_unstable();
    Ok(out)
}

fn absorb(out: &mut GroupRevertOutcome, other: GroupRevertOutcome) {
    out.notices.extend(other.notices);
    out.conflicts.extend(other.conflicts);
    for place in other.touched_place_ids {
        push_unique(&mut out.touched_place_ids, place);
    }
    for group in other.touched_group_ids {
        push_unique(&mut out.touched_group_ids, group);
    }
    for group in other.kept_in_place_group_ids {
        push_unique(&mut out.kept_in_place_group_ids, group);
    }
}

/// D-48: если устройство входит в группу, чей перенос актом забыт, подменить в снимке
/// `place_id` текущим местом корня группы, чтобы восстановление не вывело устройство из
/// группы. Место берётся из живой группы. Возвращает `true`, если подмена выполнена.
pub(crate) fn snapshot_keeping_group_place_in_tx(
    tx: &Transaction<'_>,
    groups: &SqliteGroupRepository,
    device_id: i64,
    snapshot: &mut serde_json::Value,
    kept_group_ids: &[i64],
) -> Result<bool, AppError> {
    if kept_group_ids.is_empty() {
        return Ok(false);
    }
    let Some(group) = groups.group_of_device_in_tx(tx, device_id)? else {
        return Ok(false);
    };
    let root_id = groups.root_group_id_in_tx(tx, group.id)?;
    if !kept_group_ids.contains(&root_id) {
        return Ok(false);
    }
    let Some(place) = groups.get_group_in_tx(tx, root_id)?.place_id else {
        return Ok(false);
    };
    if let Some(obj) = snapshot.as_object_mut() {
        obj.insert("place_id".into(), serde_json::json!(place));
        return Ok(true);
    }
    Ok(false)
}

/// Обратить ОДИН эффект `move` (Phase 41.2): группа жива, её место равно `to_place_id`
/// (не двигалась с тех пор) и она корневая, иначе группа остаётся на месте и возвращается
/// уведомление. Строки обратного переноса несут `source = 'act'` и `act_id` эффекта.
/// Удаление самого эффекта и порядок относительно `detach` — у вызывающего.
pub(crate) fn revert_move_effect_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    effect: &ActGroupEffectRow,
    user_id: Option<i64>,
    now: i64,
) -> Result<GroupRevertOutcome, AppError> {
    let mut out = GroupRevertOutcome::default();
    let act_id = effect.act_id;
    let label = effect.group_label.clone();
    let result = guarded(tx, || {
        let group = match deps.groups.get_group_in_tx(tx, effect.group_id) {
            Ok(g) => g,
            Err(AppError::NotFound { .. }) => return Ok(None),
            Err(e) => return Err(e),
        };
        if group.parent_group_id.is_some() {
            return Ok(None);
        }
        if group.place_id != effect.to_place_id {
            // D-48 (UD-4): корневая группа стоит на другом месте - перенос актом «забывается»:
            // группа и члены остаются, журнал и следующий эффект переписываются.
            if group.place_id.is_none() {
                return Ok(None);
            }
            deps.movements
                .forget_group_move_rows_in_tx(tx, act_id, effect.group_id)?;
            deps.groups.rebase_later_move_effects_in_tx(
                tx,
                effect.group_id,
                effect.id,
                effect.to_place_id,
                effect.from_place_id,
            )?;
            let mut places: Vec<i64> = Vec::new();
            places.extend(effect.from_place_id);
            places.extend(effect.to_place_id);
            return Ok(Some(places));
        }
        let mut changed: Vec<i64> = Vec::new();
        match effect.from_place_id {
            Some(from) => {
                let batch_id = uuid::Uuid::new_v4().to_string();
                let outcome = propagate_group_place_in_tx(
                    tx,
                    deps,
                    effect.group_id,
                    from,
                    group.place_id,
                    &batch_id,
                    user_id,
                    now,
                    GroupMoveCause::act(act_id),
                )?;
                changed.extend(outcome.changed_place_ids);
                changed.push(from);
            }
            None => {
                deps.groups
                    .set_subtree_place_in_tx(tx, effect.group_id, None, now)?;
                if let Some(p) = group.place_id {
                    changed.push(p);
                }
            }
        }
        Ok(Some(changed))
    });
    match result {
        Ok(Some(changed)) => {
            for place in changed {
                push_unique(&mut out.touched_place_ids, place);
            }
            push_unique(&mut out.touched_group_ids, effect.group_id);
            // Место группы после отката авторитетно для её членов: при «забытом» переносе
            // это текущее место, при обычном возврате - прежнее (снимок члена может
            // устареть, если предыдущий перенос цепочки был забыт, D-48).
            push_unique(&mut out.kept_in_place_group_ids, effect.group_id);
        }
        Ok(None) | Err(_) => out.notices.push(GroupNotice {
            message: format!(
                "Группа «{label}» не возвращена на прежнее место: после акта её переместили."
            ),
            group_id: Some(effect.group_id),
        }),
    }
    Ok(out)
}

/// Обратить ОДИН эффект `detach` (вызывается ПОСЛЕ обращения `move` той же группы): родитель
/// жив, группа корневая и её место совпадает с местом родителя — вложенность
/// восстанавливается; иначе уведомление.
pub(crate) fn revert_detach_effect_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    effect: &ActGroupEffectRow,
    _user_id: Option<i64>,
    now: i64,
) -> Result<GroupRevertOutcome, AppError> {
    let mut out = GroupRevertOutcome::default();
    let notice = GroupNotice {
        message: format!(
            "Группа «{}» не возвращена в прежнюю группу: после акта изменились её место или состав.",
            effect.group_label
        ),
        group_id: Some(effect.group_id),
    };
    let Some(parent_id) = effect.from_parent_group_id else {
        out.notices.push(notice);
        return Ok(out);
    };
    let result = guarded(tx, || {
        let group = match deps.groups.get_group_in_tx(tx, effect.group_id) {
            Ok(g) => g,
            Err(AppError::NotFound { .. }) => return Ok(false),
            Err(e) => return Err(e),
        };
        let parent = match deps.groups.get_group_in_tx(tx, parent_id) {
            Ok(g) => g,
            Err(AppError::NotFound { .. }) => return Ok(false),
            Err(e) => return Err(e),
        };
        if group.parent_group_id.is_some() || group.place_id != parent.place_id {
            return Ok(false);
        }
        deps.groups
            .set_parent_in_tx(tx, group.id, group.version, Some(parent_id), now)?;
        Ok(true)
    });
    match result {
        Ok(true) => {
            push_unique(&mut out.touched_group_ids, effect.group_id);
            push_unique(&mut out.touched_group_ids, parent_id);
        }
        Ok(false) | Err(_) => out.notices.push(notice),
    }
    Ok(out)
}

/// Устройства, покрываемые группой при передаче актом (Phase 41.2): для контейнера — состав
/// со вложенными группами, для якорной группы — один якорь. Правка акта сверяет это
/// покрытие с убираемыми позициями: группа возвращается, когда из акта убрано ВСЁ покрытие.
pub(crate) fn units_of_group_in_tx(
    tx: &Transaction<'_>,
    group_id: i64,
) -> Result<Vec<i64>, AppError> {
    let snapshot = load_snapshot(tx, &SqliteGroupRepository)?;
    let mut ids: Vec<i64> = act_units(&snapshot, group_id)
        .into_iter()
        .map(|u| u.device_id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// Id эффекта `release` ЭТОГО акта по устройству, записанного РАНЬШЕ текущей операции
/// (при создании акта или прошлой правке), либо `None`. Читатель без записей: вызывать до
/// любых записей по устройству, иначе эффект, который запишет сама правка (режим
/// `Record`), будет принят за прежний и потрачен возвратом членства.
pub(crate) fn prior_release_effect_id_in_tx(
    tx: &Transaction<'_>,
    groups: &SqliteGroupRepository,
    act_id: i64,
    device_id: i64,
) -> Result<Option<i64>, AppError> {
    Ok(groups
        .list_act_group_effects_in_tx(tx, act_id)?
        .into_iter()
        .find(|e| e.kind == ActGroupEffectKind::Release && e.device_id == Some(device_id))
        .map(|e| e.id))
}

/// Отвязать от акта аудит `custom:group_moved_by_act` по деталям снятой с акта группы:
/// иначе позднее удаление акта восстановило бы из него устаревший снимок деталей поверх
/// их более поздних правок. Возвращает число отвязанных строк.
pub(crate) fn forget_moved_details_in_tx(
    tx: &Transaction<'_>,
    groups: &SqliteGroupRepository,
    audit: &SqliteAuditLogRepository,
    act_id: i64,
    group_id: i64,
) -> Result<usize, AppError> {
    let mut detached = 0;
    for device_id in groups.subtree_device_ids_in_tx(tx, group_id)? {
        detached +=
            audit.detach_act_link_in_tx(tx, act_id, "custom:group_moved_by_act", device_id)?;
    }
    Ok(detached)
}

/// Вернуть в группы устройства, вынесенные актом (след `release`). Вызывается ПОСЛЕ
/// восстановления снимков устройств: место читается текущее. Нарушение любого из четырёх
/// правил `restore_membership_in_tx` и ошибка записи превращаются в уведомление, ничего не падает.
/// Удаление самих следов — `delete_act_group_effects_in_tx` у вызывающего, после обеих функций.
///
/// `choices` (D-42): group_id -> «вернуть в группу на её место» (true) / «оставить вне
/// группы» (false). Конфликт по группе без выбора копится в `conflicts`, записей не делает.
pub(crate) fn restore_released_members_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    choices: &HashMap<i64, bool>,
    act_id: i64,
    user_id: Option<i64>,
    now: i64,
) -> Result<GroupRevertOutcome, AppError> {
    let mut out = GroupRevertOutcome::default();
    let effects = deps.groups.list_act_group_effects_in_tx(tx, act_id)?;
    for effect in effects
        .iter()
        .filter(|e| e.kind == ActGroupEffectKind::Release)
    {
        let Some(device_id) = effect.device_id else {
            continue;
        };
        let device: Option<(String, Option<i64>)> = tx
            .query_row(
                "SELECT name, place_id FROM devices WHERE id = ?1 AND deleted_at_utc IS NULL",
                rusqlite::params![device_id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<i64>>(1)?)),
            )
            .optional()
            .map_err(map_rusqlite)?;
        let Some((device_name, place)) = device else {
            continue;
        };
        let policy = match choices.get(&effect.group_id) {
            Some(true) => PlaceConflictPolicy::Restore,
            Some(false) => PlaceConflictPolicy::KeepOut,
            None => PlaceConflictPolicy::Ask,
        };
        let result = guarded(tx, || {
            restore_membership_with_policy_in_tx(
                tx,
                deps,
                device_id,
                effect.group_id,
                &effect.group_label,
                &device_name,
                place,
                act_id,
                user_id,
                now,
                policy,
            )
        });
        match result {
            Ok(RestoreResult::Restored) => push_unique(&mut out.touched_group_ids, effect.group_id),
            Ok(RestoreResult::RestoredToGroupPlace {
                previous_place,
                group_place,
            }) => {
                push_unique(&mut out.touched_group_ids, effect.group_id);
                push_unique(&mut out.touched_place_ids, group_place);
                if let Some(prev) = previous_place {
                    push_unique(&mut out.touched_place_ids, prev);
                }
            }
            Ok(RestoreResult::KeptOut) => push_unique(&mut out.touched_group_ids, effect.group_id),
            Ok(RestoreResult::Conflict(conflict)) => out.conflicts.push(conflict),
            Ok(RestoreResult::Skipped(notice)) => {
                if let Some(g) = notice.group_id {
                    push_unique(&mut out.touched_group_ids, g);
                }
                out.notices.push(notice);
            }
            Err(_) => out.notices.push(GroupNotice {
                message: format!(
                    "«{device_name}» не вернулся в «{}»: не удалось восстановить состав.",
                    effect.group_label
                ),
                group_id: Some(effect.group_id),
            }),
        }
    }
    out.touched_place_ids.sort_unstable();
    out.touched_group_ids.sort_unstable();
    Ok(out)
}

/// Кандидат на возврат в группу: (устройство, id следа `release`, подпись группы).
type RejoinCandidate = (i64, i64, String);

/// D-49 (G8, вариант Release): правка, делающая возврат ПОЛНЫМ возвратом группы, отменяет и
/// выбор «Вывести из группы». Удержанные позиции со следом `release` ЭТОГО акта (прочитан до
/// правки, W-7) возвращаются в свою группу, след снимается; вызывающий отдаёт их планировщику
/// вместе с добавленными, и группа, теперь покрытая целиком, едет на место возврата (D-44).
/// Места проверяет сам план (D-46, `ensure_places_match`) до применения; при отказе транзакция
/// не коммитится.
///
/// «Полный»: каждый юнит текущего состава группы (`units_of_group_in_tx`) есть в позициях правки.
/// Иначе ничего не меняется: частичная правка разбиение не отменяет. Члены без следа этого акта
/// (выведены вручную или другим актом) не трогаются. Возвращаются id вернувшихся устройств.
#[allow(clippy::too_many_arguments)]
pub(crate) fn rejoin_released_retained_for_whole_group_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    act_id: i64,
    retained: &[i64],
    payload_device_ids: &HashSet<i64>,
    prior_effects: &[ActGroupEffectRow],
    user_id: Option<i64>,
    now: i64,
) -> Result<RejoinedByAct, AppError> {
    let mut out = RejoinedByAct::default();
    let mut by_group: Vec<(i64, Vec<RejoinCandidate>)> = Vec::new();
    let mut sorted: Vec<i64> = retained.to_vec();
    sorted.sort_unstable();
    for device_id in sorted {
        let Some(effect) = prior_effects
            .iter()
            .find(|e| e.kind == ActGroupEffectKind::Release && e.device_id == Some(device_id))
        else {
            continue;
        };
        if deps.groups.group_of_device_in_tx(tx, device_id)?.is_some() {
            continue;
        }
        let entry = (device_id, effect.id, effect.group_label.clone());
        match by_group.iter_mut().find(|(g, _)| *g == effect.group_id) {
            Some((_, list)) => list.push(entry),
            None => by_group.push((effect.group_id, vec![entry])),
        }
    }
    by_group.sort_by_key(|(g, _)| *g);
    for (group_id, members) in by_group {
        let group = match deps.groups.get_group_in_tx(tx, group_id) {
            Ok(g) => g,
            Err(AppError::NotFound { .. }) => continue,
            Err(e) => return Err(e),
        };
        let whole = units_of_group_in_tx(tx, group.id)?
            .iter()
            .all(|d| payload_device_ids.contains(d));
        if !whole {
            continue;
        }
        let ids: Vec<i64> = members.iter().map(|(d, _, _)| *d).collect();
        let names = device_names_in_tx(tx, &ids)?;
        for (device_id, effect_id, label) in members {
            let name = names.get(&device_id).cloned().unwrap_or_default();
            // Место устройства не трогается: группу на место возврата переносит план. Место
            // группы передано как «место после восстановления», чтобы правило (c) не сработало.
            match restore_membership_with_policy_in_tx(
                tx,
                deps,
                device_id,
                group.id,
                &label,
                &name,
                group.place_id,
                act_id,
                user_id,
                now,
                PlaceConflictPolicy::Skip,
            )? {
                RestoreResult::Skipped(notice) => {
                    tracing::warn!(act_id, group_id = ?notice.group_id, "{}", notice.message);
                }
                _ => {
                    deps.groups.delete_act_group_effect_in_tx(tx, effect_id)?;
                    out.group_place_of.insert(device_id, group.place_id);
                    push_unique(&mut out.touched_group_ids, group.id);
                }
            }
        }
    }
    Ok(out)
}

/// Итог [`rejoin_released_by_act_in_tx`].
#[derive(Debug, Default)]
pub(crate) struct RejoinedByAct {
    /// Устройство, вернувшееся в группу -> место группы, на котором оно теперь стоит.
    pub group_place_of: HashMap<i64, Option<i64>>,
    pub touched_group_ids: Vec<i64>,
}

/// G8 (живая UAT 41.2, D-44/D-47/D-48): правка возврата добавляет позиции по членам, которых
/// ЭТОТ ЖЕ возврат раньше вывел из группы (`MoveGroup`: невозвращённые члены; либо позиция,
/// убранная прошлой правкой при переехавшей группе). Правка «переписывает» сделанное актом:
/// такое устройство возвращается в группу, след `release` снимается, и оно встаёт на место
/// группы — как если бы первая версия возврата включала его. Вызывающий исключает эти
/// устройства из плана добавленных (иначе группа частично покрыта и снова разбивается).
///
/// Условия возврата: след `release` этого акта по устройству был ДО правки (`prior_effects`,
/// прочитаны первым действием правки, W-7), группа перенесена этим же актом (эффект `move`
/// жив после шага `removed`), группа жива, устройство сейчас не входит ни в одну группу.
/// Члены, выведенные НЕ этим актом (другим актом, вручную), следа этого акта не имеют и не
/// трогаются.
///
/// D-46: место возврата вернувшихся (`None` = место группы) и места удержанных позиций той же
/// группы (`retained_places`) обязаны совпасть; иначе валидационная ошибка ДО любой записи.
#[allow(clippy::too_many_arguments)]
pub(crate) fn rejoin_released_by_act_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    act_id: i64,
    added: &[i64],
    prior_effects: &[ActGroupEffectRow],
    added_places: &HashMap<i64, Option<i64>>,
    retained_places: &HashMap<i64, Option<i64>>,
    user_id: Option<i64>,
    now: i64,
) -> Result<RejoinedByAct, AppError> {
    let mut out = RejoinedByAct::default();
    let moved: HashSet<i64> = deps
        .groups
        .list_act_group_effects_in_tx(tx, act_id)?
        .into_iter()
        .filter(|e| e.kind == ActGroupEffectKind::Move)
        .map(|e| e.group_id)
        .collect();
    if moved.is_empty() {
        return Ok(out);
    }
    // Кандидаты, сгруппированные по группе (порядок по id группы детерминирован).
    let mut by_group: Vec<(i64, Vec<RejoinCandidate>)> = Vec::new();
    for &device_id in added {
        let Some(effect) = prior_effects.iter().find(|e| {
            e.kind == ActGroupEffectKind::Release
                && e.device_id == Some(device_id)
                && moved.contains(&e.group_id)
        }) else {
            continue;
        };
        if deps.groups.group_of_device_in_tx(tx, device_id)?.is_some() {
            continue; // уже в группе: возвращать нечего или некуда
        }
        let entry = (device_id, effect.id, effect.group_label.clone());
        match by_group.iter_mut().find(|(g, _)| *g == effect.group_id) {
            Some((_, list)) => list.push(entry),
            None => by_group.push((effect.group_id, vec![entry])),
        }
    }
    by_group.sort_by_key(|(g, _)| *g);

    // Проверки до записи: группа жива, места совпадают (D-46).
    let mut live: Vec<(trackly_core::domain::groups::GroupRow, Vec<RejoinCandidate>)> = Vec::new();
    for (group_id, members) in by_group {
        let group = match deps.groups.get_group_in_tx(tx, group_id) {
            Ok(g) => g,
            Err(AppError::NotFound { .. }) => continue,
            Err(e) => return Err(e),
        };
        let mut places: HashSet<Option<i64>> = members
            .iter()
            .map(|(d, _, _)| added_places.get(d).copied().flatten().or(group.place_id))
            .collect();
        for d in deps.groups.subtree_device_ids_in_tx(tx, group.id)? {
            if let Some(place) = retained_places.get(&d) {
                places.insert(*place);
            }
        }
        if places.len() > 1 {
            return Err(places_mismatch_error(&group.name));
        }
        live.push((group, members));
    }

    let ids: Vec<i64> = live
        .iter()
        .flat_map(|(_, m)| m.iter().map(|(d, _, _)| *d))
        .collect();
    let names = device_names_in_tx(tx, &ids)?;
    for (group, members) in live {
        for (device_id, effect_id, label) in members {
            let name = names.get(&device_id).cloned().unwrap_or_default();
            match restore_membership_with_policy_in_tx(
                tx,
                deps,
                device_id,
                group.id,
                &label,
                &name,
                group.place_id,
                act_id,
                user_id,
                now,
                PlaceConflictPolicy::Skip,
            )? {
                RestoreResult::Skipped(notice) => {
                    tracing::warn!(act_id, group_id = ?notice.group_id, "{}", notice.message);
                }
                _ => {
                    deps.groups.delete_act_group_effect_in_tx(tx, effect_id)?;
                    out.group_place_of.insert(device_id, group.place_id);
                    push_unique(&mut out.touched_group_ids, group.id);
                }
            }
        }
    }
    Ok(out)
}

/// Режим освобождения для устройства, убираемого из акта (читатель, без записей).
///
/// `Record`, если устройство состоит в живой группе G и G либо её предок совпадает с группой
/// эффекта `move` ЭТОГО акта: группа переехала актом, и откат акта обязан вернуть
/// устройство в группу. Иначе `Skip`. Вызывающий сайт считает режим ВЫШЕ восстановления
/// снимка и передаёт последним аргументом `release_if_locked_device_in_tx`, чтобы между
/// восстановлением и освобождением не появлялось кода (окно гейта 14 строк).
pub(crate) fn release_effect_for_removed_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    act_id: i64,
    device_id: i64,
) -> Result<ReleaseEffect, AppError> {
    let Some(group) = deps.groups.group_of_device_in_tx(tx, device_id)? else {
        return Ok(ReleaseEffect::Skip);
    };
    let moved: HashSet<i64> = deps
        .groups
        .list_act_group_effects_in_tx(tx, act_id)?
        .into_iter()
        .filter(|e| e.kind == ActGroupEffectKind::Move)
        .map(|e| e.group_id)
        .collect();
    if moved.is_empty() {
        return Ok(ReleaseEffect::Skip);
    }
    // Обход предков ограничен: каждая группа посещается не более одного раза.
    let mut seen: HashSet<i64> = HashSet::new();
    let mut cur = Some(group);
    while let Some(g) = cur {
        if moved.contains(&g.id) {
            return Ok(ReleaseEffect::Record);
        }
        if !seen.insert(g.id) {
            break;
        }
        cur = match g.parent_group_id {
            Some(p) => match deps.groups.get_group_in_tx(tx, p) {
                Ok(parent) => Some(parent),
                Err(AppError::NotFound { .. }) => None,
                Err(e) => return Err(e),
            },
            None => None,
        };
    }
    Ok(ReleaseEffect::Skip)
}

#[cfg(test)]
pub(crate) mod test_support {
    use rusqlite::Connection;
    use tempfile::TempDir;
    use trackly_infra::db::migrations;
    use trackly_infra::db::pragmas::apply_writer_pragmas;

    pub(crate) const NOW: i64 = 1_700_000_000;

    /// Свежая БД с применёнными миграциями. Только вымышленные данные.
    pub(crate) fn fresh_db() -> (Connection, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("group-act-plan.db");
        let mut conn = Connection::open(&path).expect("open");
        apply_writer_pragmas(&conn).expect("writer pragmas");
        migrations::run(&mut conn).expect("run migrations");
        (conn, dir)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use rusqlite::TransactionBehavior;

    fn grp(
        id: i64,
        parent: Option<i64>,
        name: &str,
        place: Option<i64>,
        anchor: Option<i64>,
    ) -> PlanGroupRow {
        PlanGroupRow {
            id,
            parent_group_id: parent,
            name: name.to_string(),
            place_id: place,
            version: 1,
            anchor_device_id: anchor,
        }
    }

    /// Контейнер «АРМ #3» (id 1) из устройств 10, 11, 12.
    fn container_fixture() -> ActGroupSnapshot {
        ActGroupSnapshot::new(
            vec![grp(1, None, "АРМ #3", Some(5), None)],
            vec![(10, 1), (11, 1), (12, 1)],
        )
    }

    #[test]
    fn full_container_moves_as_a_whole_without_confirmation() {
        let p = plan(&container_fixture(), &[10, 11, 12]);
        assert_eq!(p.moves.len(), 1);
        assert_eq!(p.moves[0].root_group_id, 1);
        assert_eq!(p.moves[0].subtree_device_ids, vec![10, 11, 12]);
        assert_eq!(p.moves[0].from_place_id, Some(5));
        assert!(p.releases.is_empty());
        assert!(!p.requires_confirmation());
    }

    #[test]
    fn partial_container_releases_devices_and_requires_confirmation() {
        let p = plan(&container_fixture(), &[10, 11]);
        assert!(p.moves.is_empty());
        assert_eq!(
            p.releases
                .iter()
                .map(|r| (r.device_id, r.group_name.as_str()))
                .collect::<Vec<_>>(),
            vec![(10, "АРМ #3"), (11, "АРМ #3")]
        );
        assert!(p.requires_confirmation());
    }

    /// Расходящаяся пара: ОДНА фикстура, разный набор устройств акта, разный исход.
    #[test]
    fn full_versus_minus_one_device_diverge_on_the_same_fixture() {
        let snap = container_fixture();
        let full = plan(&snap, &[10, 11, 12]);
        let minus_one = plan(&snap, &[10, 11]);
        assert_eq!((full.moves.len(), full.releases.len()), (1, 0));
        assert_eq!((minus_one.moves.len(), minus_one.releases.len()), (0, 2));
        assert_ne!(full, minus_one);
    }

    #[test]
    fn nested_container_is_covered_only_when_every_part_is_covered() {
        // Корень 1 («АРМ #3»): устройство 10 и вложенная группа 2 («Монитор #2») с устройствами 20, 21.
        let snap = ActGroupSnapshot::new(
            vec![
                grp(1, None, "АРМ #3", Some(5), None),
                grp(2, Some(1), "Монитор #2", Some(5), None),
            ],
            vec![(10, 1), (20, 2), (21, 2)],
        );
        let all = plan(&snap, &[10, 20, 21]);
        assert_eq!(all.moves.len(), 1);
        assert_eq!(all.moves[0].root_group_id, 1);
        assert_eq!(all.moves[0].subtree_group_ids, vec![1, 2]);
        assert_eq!(all.moves[0].subtree_device_ids, vec![10, 20, 21]);
        assert!(all.moves[0].detach.is_none());
        assert!(all.releases.is_empty());

        // Не хватает одного устройства вложенной группы: корень не покрыт, вложенная тоже.
        let part = plan(&snap, &[10, 20]);
        assert!(part.moves.is_empty());
        assert_eq!(part.releases.len(), 2);
    }

    #[test]
    fn covered_nested_group_under_uncovered_container_is_detached() {
        let snap = ActGroupSnapshot::new(
            vec![
                grp(1, None, "АРМ #3", Some(5), None),
                grp(2, Some(1), "Монитор #2", Some(5), None),
            ],
            vec![(10, 1), (20, 2), (21, 2)],
        );
        // Вложенная группа покрыта целиком, устройство 10 корня — нет.
        let p = plan(&snap, &[20, 21]);
        assert_eq!(p.moves.len(), 1);
        assert_eq!(p.moves[0].root_group_id, 2);
        assert_eq!(
            p.moves[0].detach,
            Some(GroupDetach {
                parent_group_id: 1,
                parent_name: "АРМ #3".to_string()
            })
        );
        assert!(p.releases.is_empty());
        assert!(p.requires_confirmation());
        assert_eq!(
            p.warning_items(),
            vec![("Монитор #2".to_string(), "АРМ #3".to_string())]
        );
    }

    #[test]
    fn anchor_group_travels_with_anchor_and_parts_are_not_released() {
        // Якорная группа «Системный блок #4» (id 3), якорь 30, деталь 31.
        let snap = ActGroupSnapshot::new(
            vec![grp(3, None, "Системный блок #4", Some(7), Some(30))],
            vec![(30, 3), (31, 3)],
        );
        let only_anchor = plan(&snap, &[30]);
        assert_eq!(only_anchor.moves.len(), 1);
        assert_eq!(only_anchor.moves[0].root_group_id, 3);
        assert!(only_anchor.releases.is_empty());
        assert!(!only_anchor.requires_confirmation());

        let anchor_and_part = plan(&snap, &[30, 31]);
        assert_eq!(anchor_and_part.moves.len(), 1);
        assert!(
            anchor_and_part.releases.is_empty(),
            "деталь едет с группой и отдельной позицией выноса не получает"
        );
    }

    /// Вымышленный АРМ: контейнер 1 («АРМ #3», устройства 10, 11) с вложенной якорной
    /// группой 3 («Системный блок #4», якорь 30, деталь 31).
    fn nested_anchor_fixture() -> ActGroupSnapshot {
        ActGroupSnapshot::new(
            vec![
                grp(1, None, "АРМ #3", Some(5), None),
                grp(3, Some(1), "Системный блок #4", Some(5), Some(30)),
            ],
            vec![(10, 1), (11, 1), (30, 3), (31, 3)],
        )
    }

    #[test]
    fn return_of_nested_anchor_moves_only_the_anchor_group_with_detach() {
        let p = plan_for_return(&nested_anchor_fixture(), &[30], &places(&[(30, Some(9))]));
        assert_eq!(p.moves.len(), 1);
        assert_eq!(p.moves[0].root_group_id, 3);
        assert_eq!(p.moves[0].subtree_device_ids, vec![30, 31]);
        assert_eq!(
            p.moves[0].detach,
            Some(GroupDetach {
                parent_group_id: 1,
                parent_name: "АРМ #3".to_string()
            })
        );
        assert!(p.releases.is_empty());
    }

    fn places(pairs: &[(i64, Option<i64>)]) -> HashMap<i64, Option<i64>> {
        pairs.iter().copied().collect()
    }

    /// D-44: целиком возвращаемый контейнер едет к общему месту возврата, а не рассыпается.
    #[test]
    fn return_of_whole_container_moves_it_to_the_common_return_place() {
        let snap = container_fixture();
        let by_return = plan_for_return(
            &snap,
            &[10, 11, 12],
            &places(&[(10, Some(9)), (11, Some(9)), (12, Some(9))]),
        );
        assert_eq!(by_return.moves.len(), 1);
        assert_eq!(by_return.moves[0].root_group_id, 1);
        assert_eq!(by_return.moves[0].anchor_device_id, None);
        assert_eq!(by_return.moves[0].target_place_id, Some(9));
        assert_eq!(by_return.moves[0].subtree_device_ids, vec![10, 11, 12]);
        assert!(by_return.releases.is_empty());
        assert!(by_return.splits.is_empty());
        assert!(by_return.place_conflicts.is_empty());
        assert!(by_return.ensure_places_match().is_ok());
    }

    #[test]
    fn return_of_whole_container_without_places_keeps_the_group_in_place() {
        let by_return = plan_for_return(
            &container_fixture(),
            &[10, 11, 12],
            &places(&[(10, None), (11, None), (12, None)]),
        );
        assert_eq!(by_return.moves.len(), 1);
        assert_eq!(by_return.moves[0].target_place_id, None);
        assert!(by_return.ensure_places_match().is_ok());
    }

    /// D-46: разные места (включая None против Some) — конфликт, а не «место наименьшего id».
    #[test]
    fn return_of_whole_container_with_different_places_is_a_conflict() {
        for map in [
            places(&[(10, Some(9)), (11, Some(9)), (12, Some(8))]),
            places(&[(10, Some(9)), (11, Some(9)), (12, None)]),
        ] {
            let p = plan_for_return(&container_fixture(), &[10, 11, 12], &map);
            assert!(p.moves.is_empty() && p.splits.is_empty() && p.releases.is_empty());
            assert_eq!(
                p.place_conflicts,
                vec![GroupPlaceConflict {
                    group_id: 1,
                    group_name: "АРМ #3".to_string(),
                    device_ids: vec![10, 11, 12],
                }]
            );
            match p.ensure_places_match() {
                Err(AppError::Validation { field, message }) => {
                    assert_eq!(field, "place_id");
                    assert_eq!(
                        message,
                        "Группа «АРМ #3» возвращается целиком: места возврата всех её устройств должны совпадать."
                    );
                }
                other => panic!("{other:?}"),
            }
        }
    }

    /// Расходящаяся пара: тот же снимок, весь состав -> move, минус одно устройство -> split.
    #[test]
    fn return_of_container_minus_one_member_is_a_split_not_a_move() {
        let snap = container_fixture();
        let map = places(&[(10, Some(9)), (11, Some(9)), (12, Some(9))]);
        let full = plan_for_return(&snap, &[10, 11, 12], &map);
        let part = plan_for_return(&snap, &[10, 11], &map);
        assert_eq!((full.moves.len(), full.splits.len()), (1, 0));
        assert!(part.moves.is_empty() && part.releases.is_empty());
        assert_eq!(part.splits.len(), 1);
        let s = &part.splits[0];
        assert_eq!(s.group_id, 1);
        assert_eq!(s.returned_device_ids, vec![10, 11]);
        assert_eq!(s.kept_device_ids, vec![12]);
        assert_eq!(s.target_place_id, Some(9));
        assert!(s.places_match);
        assert_eq!(s.from_place_id, Some(5));
        assert_ne!(full, part);
    }

    #[test]
    fn split_with_different_places_has_no_target_and_does_not_match() {
        let p = plan_for_return(
            &container_fixture(),
            &[10, 11],
            &places(&[(10, Some(9)), (11, Some(8))]),
        );
        assert_eq!(p.splits.len(), 1);
        assert_eq!(p.splits[0].target_place_id, None);
        assert!(!p.splits[0].places_match);
        assert!(p.releases.is_empty());
        assert!(p.ensure_places_match().is_ok(), "для split это не конфликт");
    }

    #[test]
    fn split_without_places_matches_with_no_target() {
        let p = plan_for_return(
            &container_fixture(),
            &[10, 11],
            &places(&[(10, None), (11, None)]),
        );
        assert_eq!(p.splits[0].target_place_id, None);
        assert!(p.splits[0].places_match);
    }

    #[test]
    fn return_of_anchor_with_parts_keeps_parts_out_of_releases() {
        let p = plan_for_return(
            &nested_anchor_fixture(),
            &[30, 31],
            &places(&[(30, Some(9)), (31, Some(9))]),
        );
        assert_eq!(p.moves.len(), 1);
        assert!(p.releases.is_empty(), "детали едут с группой");
    }

    #[test]
    fn return_of_part_without_anchor_releases_the_part_only() {
        let p = plan_for_return(&nested_anchor_fixture(), &[31], &places(&[(31, Some(9))]));
        assert!(p.moves.is_empty());
        assert_eq!(p.releases.len(), 1);
        assert_eq!(p.releases[0].device_id, 31);
    }

    /// Контейнер 1 покрыт: прямые 10, 11 возвращены и вложенная якорная группа 3 (якорь 30).
    #[test]
    fn return_of_container_members_and_nested_anchor_moves_the_whole_container() {
        let p = plan_for_return(
            &nested_anchor_fixture(),
            &[10, 11, 30],
            &places(&[(10, Some(9)), (11, Some(9)), (30, Some(9))]),
        );
        assert_eq!(p.moves.len(), 1);
        assert_eq!(p.moves[0].root_group_id, 1);
        assert_eq!(p.moves[0].target_place_id, Some(9));
        assert!(p.releases.is_empty() && p.splits.is_empty());
    }

    /// Контейнер 1 не покрыт (член 12 остаётся): вложенная якорная группа уезжает сама с
    /// отцеплением, прямые возвращаемые члены описаны разбиением.
    #[test]
    fn nested_anchor_under_partially_returned_container_moves_and_container_splits() {
        let snap = ActGroupSnapshot::new(
            vec![
                grp(1, None, "АРМ #3", Some(5), None),
                grp(3, Some(1), "Системный блок #4", Some(5), Some(30)),
            ],
            vec![(10, 1), (11, 1), (12, 1), (30, 3), (31, 3)],
        );
        let p = plan_for_return(
            &snap,
            &[10, 11, 30],
            &places(&[(10, Some(9)), (11, Some(9)), (30, Some(7))]),
        );
        assert_eq!(p.moves.len(), 1);
        assert_eq!(p.moves[0].root_group_id, 3);
        assert!(p.moves[0].detach.is_some());
        assert_eq!(p.splits.len(), 1);
        assert_eq!(p.splits[0].returned_device_ids, vec![10, 11]);
        assert_eq!(p.splits[0].kept_device_ids, vec![12]);
        assert!(p.releases.is_empty());
    }

    /// CR-02: покрытая вложенная якорная группа при `MoveGroup` едет вместе с разбиением,
    /// поэтому место возврата её якоря входит в сравнение мест разбиения. Разные места —
    /// `places_match = false` (перенос группы недоступен), одно место — общая цель.
    #[test]
    fn split_places_match_counts_covered_nested_anchor_that_rides_along() {
        let snap = ActGroupSnapshot::new(
            vec![
                grp(1, None, "АРМ #3", Some(5), None),
                grp(3, Some(1), "Системный блок #4", Some(5), Some(30)),
            ],
            vec![(10, 1), (11, 1), (12, 1), (30, 3), (31, 3)],
        );
        let differ = plan_for_return(
            &snap,
            &[10, 11, 30],
            &places(&[(10, Some(9)), (11, Some(9)), (30, Some(7))]),
        );
        assert_eq!(differ.splits.len(), 1);
        assert!(
            !differ.splits[0].places_match,
            "якорь вложенной группы возвращается в другое место"
        );
        assert_eq!(differ.splits[0].target_place_id, None);

        let same = plan_for_return(
            &snap,
            &[10, 11, 30],
            &places(&[(10, Some(9)), (11, Some(9)), (30, Some(9))]),
        );
        assert!(same.splits[0].places_match);
        assert_eq!(same.splits[0].target_place_id, Some(9));

        // Якорь без места против прямых членов с местом — тоже расхождение (D-46: None != Some).
        let none = plan_for_return(
            &snap,
            &[10, 11, 30],
            &places(&[(10, Some(9)), (11, Some(9)), (30, None)]),
        );
        assert!(!none.splits[0].places_match);
    }

    /// Вложенная непокрытая группа не мешает покрытому корню: корень едет целиком.
    #[test]
    fn covered_nested_container_travels_with_its_covered_parent() {
        let snap = ActGroupSnapshot::new(
            vec![
                grp(1, None, "АРМ #3", Some(5), None),
                grp(2, Some(1), "Монитор #2", Some(5), None),
            ],
            vec![(10, 1), (20, 2), (21, 2)],
        );
        let p = plan_for_return(
            &snap,
            &[10, 20, 21],
            &places(&[(10, Some(9)), (20, Some(9)), (21, Some(9))]),
        );
        assert_eq!(p.moves.len(), 1);
        assert_eq!(p.moves[0].root_group_id, 1);
        assert_eq!(p.moves[0].subtree_group_ids, vec![1, 2]);
        // Вложенная покрыта, родитель нет: сама корень move с отцеплением.
        let q = plan_for_return(&snap, &[20, 21], &places(&[(20, Some(9)), (21, Some(9))]));
        assert_eq!(q.moves.len(), 1);
        assert_eq!(q.moves[0].root_group_id, 2);
        assert!(q.moves[0].detach.is_some());
        assert!(
            q.splits.is_empty(),
            "у родителя нет возвращаемых прямых членов"
        );
    }

    #[test]
    fn return_ignores_devices_outside_groups() {
        let p = plan_for_return(&nested_anchor_fixture(), &[99], &HashMap::new());
        assert!(p.moves.is_empty() && p.releases.is_empty());
    }

    #[test]
    fn anchor_group_part_without_anchor_is_released_and_anchor_never_is() {
        let snap = ActGroupSnapshot::new(
            vec![grp(3, None, "Системный блок #4", Some(7), Some(30))],
            vec![(30, 3), (31, 3)],
        );
        let only_part = plan(&snap, &[31]);
        assert!(only_part.moves.is_empty());
        assert_eq!(only_part.releases.len(), 1);
        assert_eq!(only_part.releases[0].device_id, 31);
        assert!(only_part.requires_confirmation());
        assert!(
            plan(&snap, &[30, 31])
                .releases
                .iter()
                .all(|r| r.device_id != 30),
            "якорь не выносится никогда"
        );
    }

    #[test]
    fn empty_container_is_never_covered() {
        let snap = ActGroupSnapshot::new(vec![grp(4, None, "Пустая #1", Some(5), None)], vec![]);
        assert_eq!(plan(&snap, &[]), ActGroupPlan::default());
        assert_eq!(plan(&snap, &[1, 2, 3]), ActGroupPlan::default());
    }

    #[test]
    fn dormant_group_without_place_moves_like_any_other() {
        let snap = ActGroupSnapshot::new(
            vec![grp(1, None, "АРМ #3", None, None)],
            vec![(10, 1), (11, 1)],
        );
        let p = plan(&snap, &[10, 11]);
        assert_eq!(p.moves.len(), 1);
        assert_eq!(p.moves[0].from_place_id, None);
    }

    #[test]
    fn device_outside_groups_does_not_take_part() {
        let p = plan(&container_fixture(), &[99]);
        assert_eq!(p, ActGroupPlan::default());
        // Устройство вне групп рядом с полным составом ничего не меняет.
        let with_extra = plan(&container_fixture(), &[10, 11, 12, 99]);
        assert_eq!(with_extra.moves.len(), 1);
        assert!(with_extra.releases.is_empty());
    }

    #[test]
    fn act_units_flatten_nested_containers_and_collapse_anchor_groups() {
        // Корень 1: устройства 10, 11; вложенный контейнер 2 («Монитор #2»): 20;
        // вложенная якорная группа 3 («Системный блок #4»): якорь 30, деталь 31.
        let snap = ActGroupSnapshot::new(
            vec![
                grp(1, None, "АРМ #3", Some(5), None),
                grp(2, Some(1), "Монитор #2", Some(5), None),
                grp(3, Some(1), "Системный блок #4", Some(5), Some(30)),
            ],
            vec![(10, 1), (11, 1), (20, 2), (30, 3), (31, 3)],
        );
        let units = act_units(&snap, 1);
        assert_eq!(
            units,
            vec![
                ActUnit {
                    device_id: 10,
                    via_group_name: None
                },
                ActUnit {
                    device_id: 11,
                    via_group_name: None
                },
                ActUnit {
                    device_id: 20,
                    via_group_name: Some("Монитор #2".to_string())
                },
                ActUnit {
                    device_id: 30,
                    via_group_name: Some("Системный блок #4".to_string())
                },
            ],
            "вместо деталей якорной группы — один якорь"
        );
        // Корень-якорная группа даёт один юнит без via.
        assert_eq!(
            act_units(&snap, 3),
            vec![ActUnit {
                device_id: 30,
                via_group_name: None
            }]
        );
    }

    #[test]
    fn act_units_of_empty_group_is_empty() {
        let snap = ActGroupSnapshot::new(vec![grp(4, None, "Пустая #1", Some(5), None)], vec![]);
        assert!(act_units(&snap, 4).is_empty());
        assert!(act_units(&snap, 999).is_empty());
    }

    #[test]
    fn consistency_act_units_fed_to_plan_move_exactly_that_group() {
        let snap = ActGroupSnapshot::new(
            vec![
                grp(1, None, "АРМ #3", Some(5), None),
                grp(2, Some(1), "Монитор #2", Some(5), None),
                grp(3, Some(1), "Системный блок #4", Some(5), Some(30)),
                grp(8, None, "Соседний АРМ", Some(5), None),
            ],
            vec![(10, 1), (20, 2), (30, 3), (31, 3), (80, 8)],
        );
        for gid in [1, 2, 3, 8] {
            let ids: Vec<i64> = act_units(&snap, gid)
                .into_iter()
                .map(|u| u.device_id)
                .collect();
            let p = plan(&snap, &ids);
            assert_eq!(
                p.moves.iter().map(|m| m.root_group_id).collect::<Vec<_>>(),
                vec![gid],
                "группа {gid}"
            );
            assert!(p.releases.is_empty(), "группа {gid}");
        }
    }

    #[test]
    fn cyclic_parents_do_not_hang_the_planner() {
        // Испорченные данные: 1 -> 2 -> 1.
        let snap = ActGroupSnapshot::new(
            vec![
                grp(1, Some(2), "Цикл #1", Some(5), None),
                grp(2, Some(1), "Цикл #2", Some(5), None),
            ],
            vec![(10, 1), (20, 2)],
        );
        let _ = plan(&snap, &[10, 20]);
        let _ = act_units(&snap, 1);
    }

    /// Группы, места, устройства — вымышленный посев для тестов с БД.
    fn seed(conn: &Connection) {
        conn.execute_batch(&format!(
            "INSERT INTO places (id, parent_id, kind, name, is_storage, created_at_utc, updated_at_utc)
               VALUES (1, NULL, 'building', 'Склад А', 1, {NOW}, {NOW});
             INSERT INTO group_types (id, code, name, behavior, created_at_utc, updated_at_utc)
               VALUES (100, 'workstation_test', 'АРМ', 'container', {NOW}, {NOW});
             INSERT INTO groups (id, type_id, name, seq, place_id, created_at_utc, updated_at_utc)
               VALUES (1, 100, 'АРМ #3', 3, 1, {NOW}, {NOW});
             INSERT INTO devices (id, type_id, name, inventory_number, status_id, place_id,
                                  created_at_utc, updated_at_utc)
               VALUES (1, 1, 'Монитор #2', 'ИНВ-000001', 2, 1, {NOW}, {NOW}),
                      (2, 1, 'Клавиатура #1', 'ИНВ-000002', 2, 1, {NOW}, {NOW}),
                      (3, 1, 'Мышь #1', 'ИНВ-000003', 2, 1, {NOW}, {NOW});
             INSERT INTO group_devices (device_id, group_id, added_at_utc)
               VALUES (1, 1, {NOW}), (2, 1, {NOW}), (3, 1, {NOW});"
        ))
        .expect("seed");
    }

    #[test]
    fn plan_in_tx_reads_the_database_and_fills_device_names() {
        let (mut conn, _guard) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .expect("tx");

        let partial = plan_in_tx(&tx, &[1, 2]).expect("plan");
        assert!(partial.moves.is_empty());
        assert_eq!(
            partial.warning_items(),
            vec![
                ("Монитор #2".to_string(), "АРМ #3".to_string()),
                ("Клавиатура #1".to_string(), "АРМ #3".to_string()),
            ]
        );

        let full = plan_in_tx(&tx, &[1, 2, 3]).expect("plan");
        assert_eq!(full.moves.len(), 1);
        assert_eq!(full.moves[0].root_name, "АРМ #3");
        assert_eq!(full.moves[0].from_place_id, Some(1));
        assert!(!full.requires_confirmation());
    }

    #[test]
    fn load_snapshot_skips_soft_deleted_groups_and_devices() {
        let (conn, _guard) = fresh_db();
        seed(&conn);
        conn.execute(
            "UPDATE devices SET deleted_at_utc = ?1 WHERE id = 3",
            rusqlite::params![NOW],
        )
        .unwrap();
        let snap = load_snapshot(&conn, &SqliteGroupRepository).expect("snapshot");
        // Мёртвое устройство 3 не нужно для покрытия: группа покрыта устройствами 1 и 2.
        let p = plan(&snap, &[1, 2]);
        assert_eq!(p.moves.len(), 1);
        assert!(p.releases.is_empty());

        conn.execute(
            "UPDATE groups SET deleted_at_utc = ?1 WHERE id = 1",
            rusqlite::params![NOW],
        )
        .unwrap();
        let snap = load_snapshot(&conn, &SqliteGroupRepository).expect("snapshot");
        assert_eq!(plan(&snap, &[1, 2]), ActGroupPlan::default());
    }
}

#[cfg(test)]
mod apply_tests {
    use super::test_support::*;
    use super::*;
    use rusqlite::TransactionBehavior;
    use trackly_infra::repos::audit_log_sqlite::SqliteAuditLogRepository;

    const ACT: i64 = 7;

    /// Места 1 «Склад А», 2 «Склад Б», 3 «Склад В»; контейнер 1 «АРМ #3» (место 1) из
    /// монитора 1, принтера 2 (с картриджем) и мыши 3; контейнер 2 «АРМ #4» (место 1) из
    /// устройств 4 и 5; контейнер 7 «Комплект #1» (место 1) с мышью 6 и вложенной
    /// якорной группой 8 «Системный блок #6» (место 1, якорь 30, деталь 31).
    fn seed(conn: &Connection) {
        conn.execute_batch(&format!(
            "INSERT INTO places (id, parent_id, kind, name, is_storage, created_at_utc, updated_at_utc)
               VALUES (1, NULL, 'building', 'Склад А', 1, {NOW}, {NOW}),
                      (2, NULL, 'building', 'Склад Б', 1, {NOW}, {NOW}),
                      (3, NULL, 'building', 'Склад В', 1, {NOW}, {NOW});
             INSERT INTO group_types (id, code, name, behavior, created_at_utc, updated_at_utc)
               VALUES (100, 'workstation_test', 'АРМ', 'container', {NOW}, {NOW}),
                      (101, 'assembly_test', 'Сборка', 'substitute', {NOW}, {NOW});
             INSERT INTO devices (id, type_id, name, inventory_number, status_id, place_id,
                                  created_at_utc, updated_at_utc)
               VALUES (1, 1, 'Монитор #2', 'ИНВ-000001', 2, 1, {NOW}, {NOW}),
                      (2, 1, 'Принтер #1', 'ИНВ-000002', 2, 1, {NOW}, {NOW}),
                      (3, 1, 'Мышь #1', 'ИНВ-000003', 2, 1, {NOW}, {NOW}),
                      (4, 1, 'Клавиатура #1', 'ИНВ-000004', 2, 1, {NOW}, {NOW}),
                      (5, 1, 'Колонки #1', 'ИНВ-000005', 2, 1, {NOW}, {NOW}),
                      (6, 1, 'Мышь #2', 'ИНВ-000006', 2, 1, {NOW}, {NOW}),
                      (9, 1, 'Веб-камера #1', 'ИНВ-000009', 2, 1, {NOW}, {NOW}),
                      (30, 1, 'Системный блок #6', 'ИНВ-000030', 2, 1, {NOW}, {NOW}),
                      (31, 1, 'Планка памяти #3', 'ИНВ-000031', 2, 1, {NOW}, {NOW});
             INSERT INTO groups (id, type_id, name, seq, place_id, parent_group_id, anchor_device_id,
                                 created_at_utc, updated_at_utc)
               VALUES (1, 100, 'АРМ #3', 3, 1, NULL, NULL, {NOW}, {NOW}),
                      (2, 100, 'АРМ #4', 4, 1, NULL, NULL, {NOW}, {NOW}),
                      (7, 100, 'Комплект #1', 5, 1, NULL, NULL, {NOW}, {NOW}),
                      (8, 101, 'Системный блок #6', 6, 1, 7, 30, {NOW}, {NOW});
             INSERT INTO group_devices (device_id, group_id, added_at_utc)
               VALUES (1, 1, {NOW}), (2, 1, {NOW}), (3, 1, {NOW}),
                      (4, 2, {NOW}), (5, 2, {NOW}),
                      (6, 7, {NOW}), (30, 8, {NOW}), (31, 8, {NOW});
             INSERT INTO acts (id, number, act_type, giver_name, receiver_name, created_at_utc, updated_at_utc)
               VALUES (7, '7', 'handover', 'Иванов И.И.', 'Петров П.П.', {NOW}, {NOW});
             INSERT INTO cartridge_models (id, brand, model, created_at_utc, updated_at_utc, version)
               VALUES (1, 'Бренд', 'Модель-1', {NOW}, {NOW}, 1);
             INSERT INTO cartridges (code, model_id, status_id, place_id, current_printer_device_id,
                                     created_at_utc, updated_at_utc, version)
               VALUES ('C-TEST-1', 1, 1, 1, 2, {NOW}, {NOW}, 1);"
        ))
        .expect("seed");
    }

    fn deps() -> GroupMoveDeps {
        GroupMoveDeps::new()
    }

    fn plan_for(tx: &Transaction<'_>, ids: &[i64]) -> ActGroupPlan {
        plan_in_tx(tx, ids).expect("plan")
    }

    fn device_place(conn: &Connection, id: i64) -> Option<i64> {
        conn.query_row("SELECT place_id FROM devices WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .unwrap()
    }

    fn group_place(conn: &Connection, id: i64) -> Option<i64> {
        conn.query_row("SELECT place_id FROM groups WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .unwrap()
    }

    fn group_parent(conn: &Connection, id: i64) -> Option<i64> {
        conn.query_row(
            "SELECT parent_group_id FROM groups WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .unwrap()
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

    fn scalar(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    /// (a) Полный перенос группы с принтером и картриджем: КАЖДАЯ строка журнала
    /// (группа, устройства, принтер, картридж) несёт source='act', act_id и общий batch_id.
    #[test]
    fn apply_full_move_stamps_every_journal_row_with_the_act_and_one_batch() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let p = plan_for(&tx, &[1, 2, 3]);
        assert_eq!(p.moves.len(), 1);
        let applied =
            apply_plan_in_tx(&tx, &deps(), ACT, &p, Some(2), &[1, 2, 3], None, NOW).expect("apply");
        tx.commit().unwrap();

        assert_eq!(applied.moved_device_ids, HashSet::from([1, 2, 3]));
        assert_eq!(
            applied.pre_snapshots[&1].place_id,
            Some(1),
            "снимок до переноса"
        );
        assert_eq!(applied.touched_place_ids, vec![1, 2]);
        assert_eq!(applied.touched_group_ids, vec![1]);
        assert_eq!(group_place(&conn, 1), Some(2));
        assert_eq!(device_place(&conn, 3), Some(2));

        // группа + 3 устройства + картридж = 5 строк
        let total = scalar(&conn, "SELECT COUNT(*) FROM place_movements");
        assert_eq!(total, 5);
        let stamped = scalar(
            &conn,
            "SELECT COUNT(*) FROM place_movements WHERE act_id = 7 AND source = 'act' AND batch_id IS NOT NULL",
        );
        assert_eq!(stamped, total, "все строки несут акт");
        assert_eq!(
            scalar(
                &conn,
                "SELECT COUNT(*) FROM place_movements WHERE act_id IS NULL"
            ),
            0
        );
        assert_eq!(
            scalar(
                &conn,
                "SELECT COUNT(DISTINCT batch_id) FROM place_movements"
            ),
            1,
            "общий batch_id"
        );
        assert_eq!(
            scalar(
                &conn,
                "SELECT COUNT(*) FROM place_movements WHERE entity_type = 'cartridge' AND act_id = 7"
            ),
            1,
            "картридж принтера-члена едет с актом (D-14)"
        );
    }

    /// Детали группы, которых нет среди позиций акта, получают аудит со снимком «до».
    #[test]
    fn apply_audits_parts_that_are_not_act_items() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let p = plan_for(&tx, &[30]);
        assert_eq!(p.moves.len(), 1);
        apply_plan_in_tx(&tx, &deps(), ACT, &p, Some(2), &[30], None, NOW).expect("apply");
        let mutations = SqliteAuditLogRepository
            .select_device_mutations_for_act(&tx, ACT)
            .unwrap();
        tx.commit().unwrap();
        // Деталь 31 едет с группой, но не позиция акта: её снимок «до» в аудите.
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].0, 31);
        assert!(
            mutations[0].1.contains("\"place_id\":1"),
            "{}",
            mutations[0].1
        );
    }

    /// (b) Вынос: след release записан, остальные члены на месте.
    #[test]
    fn apply_partial_releases_only_the_act_devices_and_records_the_effect() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let p = plan_for(&tx, &[4]);
        assert!(p.moves.is_empty() && p.releases.len() == 1);
        let applied =
            apply_plan_in_tx(&tx, &deps(), ACT, &p, None, &[4], None, NOW).expect("apply");
        tx.commit().unwrap();

        assert_eq!(member_of(&conn, 4), None);
        assert_eq!(member_of(&conn, 5), Some(2), "остальные члены на месте");
        assert_eq!(group_place(&conn, 2), Some(1));
        assert_eq!(applied.touched_group_ids, vec![2]);
        assert_eq!(
            scalar(
                &conn,
                "SELECT COUNT(*) FROM act_group_effects WHERE act_id = 7 AND kind = 'release' AND device_id = 4 AND group_id = 2"
            ),
            1
        );
        assert_eq!(
            scalar(&conn, "SELECT COUNT(*) FROM place_movements"),
            0,
            "вынос места не меняет"
        );
    }

    #[test]
    fn apply_without_target_place_rejects_a_plan_with_moves() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let p = plan_for(&tx, &[1, 2, 3]);
        let err = apply_plan_in_tx(&tx, &deps(), ACT, &p, None, &[1, 2, 3], None, NOW)
            .expect_err("нет места");
        match err {
            AppError::Validation { field, message } => {
                assert_eq!(field, "place_id");
                assert_eq!(message, "Для передачи группы укажите место");
            }
            other => panic!("{other:?}"),
        }
    }

    /// (c) apply -> revert -> restore: группа на прежнем месте, членство цело.
    #[test]
    fn apply_then_revert_and_restore_returns_place_and_membership() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        // Группа 1 целиком + устройство 4 из группы 2 (часть).
        let p = plan_for(&tx, &[1, 2, 3, 4]);
        assert_eq!((p.moves.len(), p.releases.len()), (1, 1));
        apply_plan_in_tx(&tx, &deps(), ACT, &p, Some(2), &[1, 2, 3, 4], None, NOW).expect("apply");
        // Устройство 4 как позиция акта уходит на место 2 (его двигает сервис актов).
        tx.execute("UPDATE devices SET place_id = 2 WHERE id = 4", [])
            .unwrap();
        // Откат: группы -> снимки -> членство.
        let reverted = revert_group_moves_in_tx(&tx, &deps(), ACT, None, NOW).expect("revert");
        assert!(reverted.notices.is_empty(), "{:?}", reverted.notices);
        tx.execute("UPDATE devices SET place_id = 1 WHERE id = 4", [])
            .unwrap();
        let restored =
            restore_released_members_in_tx(&tx, &deps(), &HashMap::new(), ACT, None, NOW)
                .expect("restore");
        assert!(restored.notices.is_empty(), "{:?}", restored.notices);
        tx.commit().unwrap();

        assert_eq!(group_place(&conn, 1), Some(1));
        for d in [1, 2, 3] {
            assert_eq!(device_place(&conn, d), Some(1), "устройство {d}");
            assert_eq!(member_of(&conn, d), Some(1));
        }
        assert_eq!(
            member_of(&conn, 4),
            Some(2),
            "членство вынесенного возвращено"
        );
        assert_eq!(restored.touched_group_ids, vec![2]);
        assert_eq!(reverted.touched_group_ids, vec![1]);
    }

    fn return_plan(
        tx: &Transaction<'_>,
        ids: &[i64],
        pairs: &[(i64, Option<i64>)],
    ) -> (ActGroupPlan, HashMap<i64, Option<i64>>) {
        let map: HashMap<i64, Option<i64>> = pairs.iter().copied().collect();
        (plan_for_return_in_tx(tx, ids, &map).expect("plan"), map)
    }

    /// Возврат 2 из 3 членов контейнера, выбор «Release»: прежнее поведение, группа на месте.
    #[test]
    fn return_split_with_release_choice_keeps_the_group_in_place() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let (p, map) = return_plan(&tx, &[1, 2], &[(1, Some(2)), (2, Some(2))]);
        assert_eq!(p.splits.len(), 1);
        assert_eq!(
            p.splits[0].returned_device_names,
            vec!["Монитор #2", "Принтер #1"]
        );
        apply_return_plan_in_tx(
            &tx,
            &deps(),
            ACT,
            &p,
            &map,
            GroupSplitChoice::Release,
            &[1, 2],
            None,
            NOW,
        )
        .expect("apply");
        tx.commit().unwrap();
        assert_eq!(group_place(&conn, 1), Some(1));
        assert_eq!(member_of(&conn, 1), None);
        assert_eq!(member_of(&conn, 2), None);
        assert_eq!(member_of(&conn, 3), Some(1));
        assert_eq!(
            scalar(
                &conn,
                "SELECT COUNT(*) FROM act_group_effects WHERE kind = 'release'"
            ),
            2
        );
    }

    /// MoveGroup: невозвращаемый член выходит и остаётся на месте, группа с возвращаемыми
    /// переезжает; откат акта возвращает группу и kept-члена.
    #[test]
    fn return_split_with_move_group_choice_moves_the_group_and_is_revertible() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let (p, map) = return_plan(&tx, &[1, 2], &[(1, Some(2)), (2, Some(2))]);
        let applied = apply_return_plan_in_tx(
            &tx,
            &deps(),
            ACT,
            &p,
            &map,
            GroupSplitChoice::MoveGroup,
            &[1, 2],
            None,
            NOW,
        )
        .expect("apply");
        assert_eq!(applied.moved_device_ids, HashSet::from([1, 2]));
        assert_eq!(group_place(&tx, 1), Some(2));
        assert_eq!(member_of(&tx, 3), None, "kept вышел из группы");
        assert_eq!(device_place(&tx, 3), Some(1), "kept остался на месте");
        assert_eq!(member_of(&tx, 1), Some(1));
        assert_eq!(member_of(&tx, 2), Some(1));
        assert_eq!(device_place(&tx, 1), Some(2));

        let reverted = revert_group_moves_in_tx(&tx, &deps(), ACT, None, NOW).expect("revert");
        assert!(reverted.notices.is_empty(), "{:?}", reverted.notices);
        let restored =
            restore_released_members_in_tx(&tx, &deps(), &HashMap::new(), ACT, None, NOW)
                .expect("restore");
        assert!(restored.notices.is_empty(), "{:?}", restored.notices);
        tx.commit().unwrap();
        assert_eq!(group_place(&conn, 1), Some(1));
        assert_eq!(member_of(&conn, 3), Some(1), "kept вернулся в группу");
        assert_eq!(device_place(&conn, 1), Some(1));
    }

    /// MoveGroup при разных местах возвращаемых: валидационная ошибка, записей нет.
    #[test]
    fn return_split_with_move_group_and_different_places_is_rejected_without_writes() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let (p, map) = return_plan(&tx, &[1, 2], &[(1, Some(2)), (2, Some(3))]);
        assert!(!p.splits[0].places_match);
        let err = apply_return_plan_in_tx(
            &tx,
            &deps(),
            ACT,
            &p,
            &map,
            GroupSplitChoice::MoveGroup,
            &[1, 2],
            None,
            NOW,
        )
        .expect_err("места не совпали");
        assert!(matches!(&err, AppError::Validation { field, .. } if field == "place_id"));
        assert_eq!(member_of(&tx, 3), Some(1));
        assert_eq!(scalar(&tx, "SELECT COUNT(*) FROM act_group_effects"), 0);
        // Release доступен всегда.
        apply_return_plan_in_tx(
            &tx,
            &deps(),
            ACT,
            &p,
            &map,
            GroupSplitChoice::Release,
            &[1, 2],
            None,
            NOW,
        )
        .expect("release");
        assert_eq!(member_of(&tx, 1), None);
    }

    /// Целиком возвращаемый контейнер с разными местами: ensure_places_match отклоняет, apply тоже.
    #[test]
    fn whole_container_with_different_places_is_rejected_by_apply_too() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let (p, map) = return_plan(&tx, &[1, 2, 3], &[(1, Some(2)), (2, Some(2)), (3, None)]);
        assert!(p.ensure_places_match().is_err());
        let err = apply_return_plan_in_tx(
            &tx,
            &deps(),
            ACT,
            &p,
            &map,
            GroupSplitChoice::Release,
            &[1, 2, 3],
            None,
            NOW,
        )
        .expect_err("конфликт");
        assert!(matches!(err, AppError::Validation { .. }));
        assert_eq!(group_place(&tx, 1), Some(1));
        assert_eq!(scalar(&tx, "SELECT COUNT(*) FROM act_group_effects"), 0);
    }

    /// Вложенность: контейнер 7 (мышь 6 + якорная группа 8, не возвращена) — MoveGroup
    /// отцепляет непокрытую дочернюю группу, она остаётся на месте.
    #[test]
    fn move_group_detaches_uncovered_children_and_leaves_them_in_place() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let (p, map) = return_plan(&tx, &[6], &[(6, Some(2))]);
        assert_eq!(p.splits.len(), 1);
        assert_eq!(p.splits[0].uncovered_children.len(), 1);
        apply_return_plan_in_tx(
            &tx,
            &deps(),
            ACT,
            &p,
            &map,
            GroupSplitChoice::MoveGroup,
            &[6],
            None,
            NOW,
        )
        .expect("apply");
        assert_eq!(group_place(&tx, 7), Some(2));
        assert_eq!(device_place(&tx, 6), Some(2));
        assert_eq!(group_parent(&tx, 8), None, "дочерняя отцеплена");
        assert_eq!(group_place(&tx, 8), Some(1), "и осталась на месте");
        assert_eq!(device_place(&tx, 30), Some(1));
        assert_eq!(
            scalar(
                &tx,
                "SELECT COUNT(*) FROM act_group_effects WHERE kind = 'detach' AND group_id = 8 AND from_parent_group_id = 7"
            ),
            1
        );
        let reverted = revert_group_moves_in_tx(&tx, &deps(), ACT, None, NOW).expect("revert");
        assert!(reverted.notices.is_empty(), "{:?}", reverted.notices);
        assert_eq!(group_place(&tx, 7), Some(1));
        assert_eq!(group_parent(&tx, 8), Some(7), "вложенность восстановлена");
    }

    /// (d) D-48: группу переместили после акта - перенос забыт: уведомления нет, группа
    /// остаётся, строки акта удалены, ручная строка получила «откуда» акта.
    #[test]
    fn revert_forgets_a_group_that_was_moved_after_the_act() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let p = plan_for(&tx, &[1, 2, 3]);
        apply_plan_in_tx(&tx, &deps(), ACT, &p, Some(2), &[1, 2, 3], None, NOW).expect("apply");
        // «После акта» группу перенесли на место 3, ручная строка журнала 2 -> 3.
        tx.execute("UPDATE groups SET place_id = 3 WHERE id = 1", [])
            .unwrap();
        tx.execute(
            "INSERT INTO place_movements (entity_type, entity_id, from_place_id, from_place_path, \
             to_place_id, to_place_path, source, act_id, group_id, created_at_utc) \
             VALUES ('group', 1, 2, 'П2', 3, 'П3', 'group', NULL, 1, ?1)",
            [NOW + 5],
        )
        .unwrap();
        let acts_rows_before = scalar(&tx, "SELECT COUNT(*) FROM place_movements WHERE act_id = 7");
        let reverted = revert_group_moves_in_tx(&tx, &deps(), ACT, None, NOW).expect("revert");
        let acts_rows_after = scalar(&tx, "SELECT COUNT(*) FROM place_movements WHERE act_id = 7");
        let successor_from = scalar(
            &tx,
            "SELECT from_place_id FROM place_movements WHERE source = 'group' AND entity_id = 1",
        );
        tx.commit().unwrap();

        assert!(reverted.notices.is_empty(), "{:?}", reverted.notices);
        assert_eq!(reverted.kept_in_place_group_ids, vec![1]);
        assert_eq!(group_place(&conn, 1), Some(3), "группа не двигалась");
        assert!(acts_rows_before > 0);
        assert_eq!(acts_rows_after, 0, "строки забытого переноса удалены");
        assert_eq!(successor_from, 1, "«откуда» ручной строки = место до акта");
    }

    /// (d2) D-48: перепись следующего эффекта - ровно один ближайший более поздний `move`.
    #[test]
    fn rebase_changes_exactly_the_nearest_later_move_effect() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        for (act, from, to) in [(7, 1, 2), (8, 2, 3), (9, 2, 3)] {
            tx.execute(
                "INSERT INTO act_group_effects (act_id, kind, group_id, group_label, \
                 from_place_id, to_place_id, created_at_utc) VALUES (?1, 'move', 1, 'АРМ #3', ?2, ?3, ?4)",
                [act, from, to, NOW],
            )
            .unwrap();
        }
        let first = scalar(&tx, "SELECT MIN(id) FROM act_group_effects");
        let changed = deps()
            .groups
            .rebase_later_move_effects_in_tx(&tx, 1, first, Some(2), Some(1))
            .expect("rebase");
        assert_eq!(changed, 1);
        assert_eq!(
            scalar(
                &tx,
                "SELECT COUNT(*) FROM act_group_effects WHERE from_place_id = 1"
            ),
            2,
            "первый эффект и ровно один следующий"
        );
        assert_eq!(
            scalar(
                &tx,
                "SELECT from_place_id FROM act_group_effects WHERE act_id = 9"
            ),
            2,
            "третий не тронут"
        );
    }

    /// (d3) D-48: строка без преемника удаляется; преемник другой сущности не трогается;
    /// строка другого акта по другой группе остаётся.
    #[test]
    fn forget_rows_without_successor_are_deleted_and_others_untouched() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        tx.execute(
            "INSERT INTO acts (id, number, act_type, giver_name, receiver_name, created_at_utc, updated_at_utc) \
             VALUES (8, '8', 'handover', 'Иванов И.И.', 'Петров П.П.', ?1, ?1)",
            [NOW],
        )
        .unwrap();
        let ins = |entity: i64, from: i64, to: i64, act: Option<i64>, group: i64, t: i64| {
            tx.execute(
                "INSERT INTO place_movements (entity_type, entity_id, from_place_id, \
                 from_place_path, to_place_id, to_place_path, source, act_id, group_id, \
                 created_at_utc) VALUES ('device', ?1, ?2, ?3, ?4, ?5, 'act', ?6, ?7, ?8)",
                rusqlite::params![
                    entity,
                    from,
                    format!("П{from}"),
                    to,
                    format!("П{to}"),
                    act,
                    group,
                    t
                ],
            )
            .unwrap();
        };
        ins(10, 1, 2, Some(7), 1, 1);
        ins(11, 1, 2, Some(7), 1, 2);
        ins(10, 2, 3, None, 1, 3); // преемник устройства 10
        ins(12, 2, 3, None, 1, 4); // другая сущность: не преемник
        ins(13, 1, 2, Some(8), 2, 5); // другой акт и группа
        let removed = deps()
            .movements
            .forget_group_move_rows_in_tx(&tx, 7, 1)
            .expect("forget");
        assert_eq!(removed, 2);
        assert_eq!(
            scalar(
                &tx,
                "SELECT from_place_id FROM place_movements WHERE entity_id = 10"
            ),
            1
        );
        assert_eq!(
            scalar(
                &tx,
                "SELECT from_place_id FROM place_movements WHERE entity_id = 12"
            ),
            2
        );
        assert_eq!(
            scalar(
                &tx,
                "SELECT COUNT(*) FROM place_movements WHERE entity_id = 13"
            ),
            1
        );
    }

    /// (e) Отцепление и возврат вложенной якорной группы туда и обратно.
    #[test]
    fn detach_and_reattach_a_nested_anchor_group() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let p = plan_for(&tx, &[30]);
        assert_eq!(p.moves[0].root_group_id, 8);
        assert!(p.moves[0].detach.is_some());
        let applied =
            apply_plan_in_tx(&tx, &deps(), ACT, &p, Some(2), &[30], None, NOW).expect("apply");
        assert_eq!(group_parent(&tx, 8), None, "группа отцеплена");
        assert_eq!(group_place(&tx, 8), Some(2));
        assert_eq!(group_place(&tx, 7), Some(1), "родитель остался на месте");
        assert_eq!(device_place(&tx, 30), Some(2));
        assert_eq!(device_place(&tx, 31), Some(2), "деталь едет с группой");
        assert_eq!(applied.touched_group_ids, vec![7, 8]);
        assert_eq!(
            scalar(
                &tx,
                "SELECT COUNT(*) FROM act_group_effects WHERE act_id = 7 AND kind = 'detach' AND from_parent_group_id = 7"
            ),
            1
        );

        let reverted = revert_group_moves_in_tx(&tx, &deps(), ACT, None, NOW).expect("revert");
        tx.commit().unwrap();
        assert!(reverted.notices.is_empty(), "{:?}", reverted.notices);
        assert_eq!(group_parent(&conn, 8), Some(7), "вложенность восстановлена");
        assert_eq!(group_place(&conn, 8), Some(1));
        assert_eq!(device_place(&conn, 30), Some(1));
        assert_eq!(device_place(&conn, 31), Some(1));
    }

    #[test]
    fn revert_does_not_reattach_when_the_parent_moved_away() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let p = plan_for(&tx, &[30]);
        apply_plan_in_tx(&tx, &deps(), ACT, &p, Some(2), &[30], None, NOW).expect("apply");
        // Родитель после акта уехал на место 3: место вложенной группы с ним уже не совпадёт.
        tx.execute("UPDATE groups SET place_id = 3 WHERE id = 7", [])
            .unwrap();
        let reverted = revert_group_moves_in_tx(&tx, &deps(), ACT, None, NOW).expect("revert");
        tx.commit().unwrap();
        assert_eq!(reverted.notices.len(), 1, "{:?}", reverted.notices);
        assert_eq!(group_parent(&conn, 8), None);
    }

    #[test]
    fn restore_released_members_reports_a_member_who_joined_another_group() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let p = plan_for(&tx, &[4]);
        apply_plan_in_tx(&tx, &deps(), ACT, &p, None, &[4], None, NOW).expect("apply");
        // После акта устройство 4 вошло в другую группу.
        tx.execute(
            "INSERT INTO group_devices (device_id, group_id, added_at_utc) VALUES (4, 1, ?1)",
            [NOW],
        )
        .unwrap();
        let restored =
            restore_released_members_in_tx(&tx, &deps(), &HashMap::new(), ACT, None, NOW)
                .expect("restore");
        tx.commit().unwrap();
        assert_eq!(restored.notices.len(), 1);
        assert_eq!(
            restored.notices[0].message,
            "«Клавиатура #1» не вернулся в «АРМ #4»: устройство уже входит в «АРМ #3»."
        );
        assert_eq!(member_of(&conn, 4), Some(1), "чужое членство не тронуто");
    }

    #[test]
    fn release_effect_for_removed_follows_the_group_moved_by_the_act() {
        let (mut conn, _g) = fresh_db();
        seed(&conn);
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let p = plan_for(&tx, &[1, 2, 3]);
        apply_plan_in_tx(&tx, &deps(), ACT, &p, Some(2), &[1, 2, 3], None, NOW).expect("apply");

        let mode =
            |device: i64| release_effect_for_removed_in_tx(&tx, &deps(), ACT, device).unwrap();
        assert_eq!(mode(1), ReleaseEffect::Record, "член переехавшей группы");
        assert_eq!(mode(4), ReleaseEffect::Skip, "член непереехавшей группы");
        assert_eq!(mode(9), ReleaseEffect::Skip, "устройство вне групп");

        // Вложенный член переехавшего корня: предок совпадает с группой эффекта.
        tx.execute("UPDATE groups SET parent_group_id = 1 WHERE id = 2", [])
            .unwrap();
        assert_eq!(mode(4), ReleaseEffect::Record, "предок переехал актом");
        tx.execute("UPDATE groups SET parent_group_id = NULL WHERE id = 2", [])
            .unwrap();

        // После удаления следа 'move' тот же член даёт Skip.
        let effects = deps()
            .groups
            .list_act_group_effects_in_tx(&tx, ACT)
            .unwrap();
        let move_id = effects
            .iter()
            .find(|e| e.kind == ActGroupEffectKind::Move)
            .unwrap()
            .id;
        deps()
            .groups
            .delete_act_group_effect_in_tx(&tx, move_id)
            .unwrap();
        assert_eq!(mode(1), ReleaseEffect::Skip);
    }
}
