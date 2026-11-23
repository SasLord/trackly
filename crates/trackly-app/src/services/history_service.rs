//! `HistoryService` — читающая сторона ленты истории (фаза 41.10, HST-05).
//!
//! Склеивает `place_movements` и `history_events` на сервере в один DTO (D-02).
//! Связь «место + статус» — ТОЧНО по `history_events.movement_id = place_movements.id`
//! (D-25); корреляции по времени нет. События объединяются в одну запись общим
//! `operation_id` ИЛИ общим `movement_id` (D-04). Только чтение через пул читателей.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::domain::history_events::HistoryEventKind;
use trackly_core::error::AppError;
use trackly_infra::db::pools::ReaderPool;
use trackly_infra::repos::history_events_sqlite::HistoryEventRow;
use trackly_infra::repos::{SqliteHistoryEventsRepository, SqlitePlaceMovementsRepository};

use crate::dto::history::{HistoryEntryDto, HistorySegmentDto};
use crate::dto::place_movements::MovementEntryDto;
use crate::services::place_movement_service::{movement_row_to_dto, resolve_actor_display};

/// Сервис ленты истории.
pub struct HistoryService {
    pub readers: Arc<ReaderPool>,
}

impl HistoryService {
    pub fn new(readers: Arc<ReaderPool>) -> Self {
        Self { readers }
    }

    /// Полная лента сущности, новые сверху (D-22). `entity_type` — `device|group|cartridge`.
    pub async fn get_timeline(
        &self,
        caller: &Identity,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Vec<HistoryEntryDto>, AppError> {
        authorize(caller, &Action::ReadPlaces)?;
        if !matches!(entity_type, "device" | "group" | "cartridge") {
            return Err(AppError::Validation {
                field: "entity_type".to_string(),
                message: "допустимы device, group, cartridge".to_string(),
            });
        }
        let readers = self.readers.clone();
        let entity_type = entity_type.to_string();
        tokio::task::spawn_blocking(move || -> Result<Vec<HistoryEntryDto>, AppError> {
            let conn = readers.acquire();
            let movement_rows =
                SqlitePlaceMovementsRepository.get_history(&conn, &entity_type, entity_id)?;
            let events = match entity_type.as_str() {
                "device" => SqliteHistoryEventsRepository.list_for_device(&conn, entity_id)?,
                "group" => SqliteHistoryEventsRepository.list_for_group(&conn, entity_id)?,
                _ => Vec::new(), // D-12: у картриджа только перемещения
            };
            let movements: Vec<MovementEntryDto> = movement_rows
                .into_iter()
                .map(|r| movement_row_to_dto(&conn, r))
                .collect();
            Ok(assemble(&conn, movements, events))
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }
}

fn find(parent: &mut [usize], i: usize) -> usize {
    let mut r = i;
    while parent[r] != r {
        r = parent[r];
    }
    let mut c = i;
    while parent[c] != r {
        let n = parent[c];
        parent[c] = r;
        c = n;
    }
    r
}

fn union(parent: &mut [usize], a: usize, b: usize) {
    let (ra, rb) = (find(parent, a), find(parent, b));
    if ra != rb {
        parent[ra.max(rb)] = ra.min(rb);
    }
}

fn segment_rank(kind: &str) -> u8 {
    match kind {
        "membership" => 0,
        "move" => 1,
        "status" => 2,
        _ => 3,
    }
}

/// IN-02 (41.10): вид сегмента выводится из `HistoryEventKind` исчерпывающим `match` без
/// `_` — новый вид события не компилируется, пока ему не назначен сегмент. Неизвестный токен
/// из БД (событие более новой версии) — "other".
fn event_segment_kind(kind: &str) -> &'static str {
    match HistoryEventKind::from_str_lenient(kind) {
        Some(HistoryEventKind::StatusChanged) => "status",
        Some(
            HistoryEventKind::GroupJoined
            | HistoryEventKind::GroupLeft
            | HistoryEventKind::GroupTransferred
            | HistoryEventKind::GroupFormed
            | HistoryEventKind::GroupDisbanded
            | HistoryEventKind::GroupNested
            | HistoryEventKind::GroupUnnested,
        ) => "membership",
        None => "other",
    }
}

fn event_segment(e: &HistoryEventRow) -> HistorySegmentDto {
    HistorySegmentDto {
        kind: event_segment_kind(&e.kind).to_string(),
        event_kind: Some(e.kind.clone()),
        movement: None,
        subject_type: Some(e.subject_type.clone()),
        subject_id: Some(e.subject_id),
        subject_label: e.subject_label.clone(),
        from_group_id: e.from_group_id,
        from_group_label: e.from_group_label.clone(),
        to_group_id: e.to_group_id,
        to_group_label: e.to_group_label.clone(),
        status_from_code: e.status_from_code.clone(),
        status_from_label: e.status_from_label.clone(),
        status_to_code: e.status_to_code.clone(),
        status_to_label: e.status_to_label.clone(),
    }
}

fn move_segment(m: MovementEntryDto) -> HistorySegmentDto {
    HistorySegmentDto {
        kind: "move".to_string(),
        event_kind: None,
        subject_type: Some(m.entity_type.clone()),
        subject_id: Some(m.entity_id),
        subject_label: m.group_label.clone(),
        movement: Some(m),
        from_group_id: None,
        from_group_label: None,
        to_group_id: None,
        to_group_label: None,
        status_from_code: None,
        status_from_label: None,
        status_to_code: None,
        status_to_label: None,
    }
}

/// Склейка: события -> компоненты по `operation_id` / `movement_id`; перемещения
/// присоединяются только по точному `movement_id`; остальные — отдельные записи.
fn assemble(
    conn: &rusqlite::Connection,
    movements: Vec<MovementEntryDto>,
    events: Vec<HistoryEventRow>,
) -> Vec<HistoryEntryDto> {
    let n = events.len();
    let mut parent: Vec<usize> = (0..n).collect();
    let mut by_op: HashMap<&str, usize> = HashMap::new();
    let mut by_mv: HashMap<i64, usize> = HashMap::new();
    for (i, e) in events.iter().enumerate() {
        if let Some(&j) = by_op.get(e.operation_id.as_str()) {
            union(&mut parent, i, j);
        } else {
            by_op.insert(e.operation_id.as_str(), i);
        }
        if let Some(mv) = e.movement_id {
            if let Some(&j) = by_mv.get(&mv) {
                union(&mut parent, i, j);
            } else {
                by_mv.insert(mv, i);
            }
        }
    }
    let mut comps: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n {
        let r = find(&mut parent, i);
        comps.entry(r).or_default().push(i);
    }

    // movement_id -> индекс компонента (только для загруженных перемещений).
    let mut mv_to_comp: HashMap<i64, usize> = HashMap::new();
    for (root, idxs) in &comps {
        for &i in idxs {
            if let Some(mv) = events[i].movement_id {
                if movements.iter().any(|m| m.id == mv) {
                    mv_to_comp.insert(mv, *root);
                }
            }
        }
    }

    // (created, rank, id, entry)
    let mut out: Vec<(i64, u8, i64, HistoryEntryDto)> = Vec::new();
    let mut attached: HashMap<usize, Vec<MovementEntryDto>> = HashMap::new();
    for m in movements {
        match mv_to_comp.get(&m.id) {
            Some(root) => attached.entry(*root).or_default().push(m),
            None => {
                let created = m.created_at_utc;
                let id = m.id;
                out.push((
                    created,
                    1,
                    id,
                    HistoryEntryDto {
                        key: format!("m:{id}"),
                        created_at_utc: created,
                        actor_display: m.actor_display.clone(),
                        reason_source: m.source.clone(),
                        reason_is_event: false,
                        act_id: m.act_id,
                        act_number: m.act_number.clone(),
                        request_id: m.request_id,
                        ref_group_id: None,
                        ref_group_label: None,
                        segments: vec![move_segment(m)],
                    },
                ));
            }
        }
    }

    for (root, idxs) in comps {
        let mut evs: Vec<&HistoryEventRow> = idxs.iter().map(|&i| &events[i]).collect();
        evs.sort_by_key(|e| e.id);
        let first = evs[0];
        let mut segs: Vec<(u8, i64, i64, HistorySegmentDto)> = Vec::new();
        for e in &evs {
            let s = event_segment(e);
            segs.push((segment_rank(&s.kind), e.created_at_utc, e.id, s));
        }
        let mut mvs = attached.remove(&root).unwrap_or_default();
        mvs.sort_by_key(|m| m.id);
        let mv_act = mvs.iter().find_map(|m| m.act_id);
        let mv_act_number = mvs.iter().find_map(|m| m.act_number.clone());
        let mv_request = mvs.iter().find_map(|m| m.request_id);
        for m in mvs {
            segs.push((1, m.created_at_utc, m.id, move_segment(m)));
        }
        segs.sort_by_key(|s| (s.0, s.1, s.2));
        let created = evs.iter().map(|e| e.created_at_utc).max().unwrap_or(0);
        let max_id = evs.iter().map(|e| e.id).max().unwrap_or(0);
        let act_id = evs.iter().find_map(|e| e.act_id).or(mv_act);
        let ref_ev = evs.iter().find(|e| e.ref_group_id.is_some());
        out.push((
            created,
            0,
            max_id,
            HistoryEntryDto {
                key: format!("e:{}", first.operation_id),
                created_at_utc: created,
                actor_display: resolve_actor_display(
                    conn,
                    first.user_id,
                    first.actor_name_snapshot.as_deref(),
                ),
                reason_source: first.source.clone(),
                reason_is_event: true,
                act_id,
                act_number: if evs.iter().any(|e| e.act_id.is_some()) {
                    crate::services::act_number_display::resolve_movement_act_number(conn, act_id)
                } else {
                    mv_act_number
                },
                request_id: evs.iter().find_map(|e| e.request_id).or(mv_request),
                ref_group_id: ref_ev.and_then(|e| e.ref_group_id),
                ref_group_label: ref_ev.and_then(|e| e.ref_group_label.clone()),
                segments: segs.into_iter().map(|s| s.3).collect(),
            },
        ));
    }

    // D-22: новые сверху; tie-break — событие раньше перемещения, затем id DESC.
    out.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(a.1.cmp(&b.1))
            .then(b.2.cmp(&a.2))
            .then(b.3.key.cmp(&a.3.key))
    });
    out.into_iter().map(|t| t.3).collect()
}

#[cfg(test)]
mod segment_kind_tests {
    use super::*;

    /// IN-02: каждый вид из `HistoryEventKind::ALL` получает осмысленный сегмент, а не "other".
    #[test]
    fn every_known_event_kind_has_a_segment() {
        for k in HistoryEventKind::ALL {
            let seg = event_segment_kind(k.as_str());
            assert!(
                seg == "status" || seg == "membership",
                "вид {} попал в {seg}",
                k.as_str()
            );
        }
        assert_eq!(event_segment_kind("unknown_future_kind"), "other");
    }
}
