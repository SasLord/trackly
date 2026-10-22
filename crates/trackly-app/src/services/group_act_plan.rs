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

// Первые вызовы — планы 41.2-06, -07, -09, -11, -13.
#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, Transaction};
use trackly_core::domain::groups::PlanGroupRow;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::SqliteGroupRepository;

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
    pub root_name: String,
    pub root_version: i64,
    pub from_place_id: Option<i64>,
    pub detach: Option<GroupDetach>,
    pub subtree_group_ids: Vec<i64>,
    pub subtree_device_ids: Vec<i64>,
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

/// Итог планирования: что переезжает целиком и что выносится из состава.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ActGroupPlan {
    pub moves: Vec<GroupMove>,
    pub releases: Vec<DeviceRelease>,
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
            root_name: g.name.clone(),
            root_version: g.version,
            from_place_id: g.place_id,
            detach,
            subtree_group_ids,
            subtree_device_ids,
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

    ActGroupPlan { moves, releases }
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

/// План для списка устройств акта внутри транзакции writer'а: загрузка + `plan` +
/// имена выносимых устройств одним запросом. Клиентский список не определяет, какие
/// группы переезжают: план всегда пересчитывается здесь (T-41.2-13).
pub(crate) fn plan_in_tx(
    tx: &Transaction<'_>,
    act_device_ids: &[i64],
) -> Result<ActGroupPlan, AppError> {
    let snapshot = load_snapshot(tx, &SqliteGroupRepository)?;
    let mut result = plan(&snapshot, act_device_ids);
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
