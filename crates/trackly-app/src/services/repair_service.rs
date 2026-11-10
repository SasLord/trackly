//! `RepairService` — ремонтные заявки (Phase 41.4).
//!
//! Этот план: чтение (опции устройств, занятые устройства, последние места, состояние
//! карточки) и создание или присоединение к открытой заявке (D-19, D-20, REP-01).
//! Переходы цикла (перемещение, отправка в сервис, возврат) добавляют следующие планы.
//!
//! Single-writer: каждая запись идёт через `WriterHandle::execute` одной транзакцией.
//! BOLA: область устройств сотрудника определяется только на сервере
//! (`employee_device_ids`), клиентский идентификатор устройства не доверяется.

use std::sync::Arc;

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use trackly_core::auth::{authorize, Action, Identity, Role};
use trackly_core::domain::place_movements::{MovementEntityKind, MovementSource};
use trackly_core::domain::printers::RequestTransitionOp;
use trackly_core::domain::repairs::{
    format_kopecks, parse_cost_kopecks, RepairVisitNew, DEVICE_STATUS_CODE_AWAITING_REPAIR,
    DEVICE_STATUS_CODE_IN_REPAIR, DEVICE_STATUS_CODE_IN_STORAGE, DEVICE_STATUS_CODE_IN_USE,
    REPAIR_REQUEST_TYPE,
};
use trackly_core::domain::requests::RequestNew;
use trackly_core::error::AppError;
use trackly_core::ports::requests::RequestRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::audit_log_sqlite::{AuditEntry, SqliteAuditLogRepository};
use trackly_infra::repos::repairs_sqlite::SqliteRepairRepository;
use trackly_infra::repos::requests_sqlite::SqliteRequestRepository;

use crate::dto::printer::WsEvent;
use crate::dto::repair::{
    OpenRepairRefDto, RepairAssignDeviceDto, RepairAssignResultDto, RepairCancelDto,
    RepairCompleteDto, RepairCreateDto, RepairCreateResultDto, RepairDeviceOptionDto,
    RepairLastPlacesDto, RepairMoveChoiceDto, RepairReceiveDto, RepairRejectDto, RepairSendDto,
    RepairStateDto, RepairTransitionResultDto, RepairVisitDto,
};
use crate::dto::request::RequestDto;
use crate::services::group_place::{move_group_in_tx, GroupMoveDeps};
use crate::services::number_template_service::issue_repair_act_number_in_tx;

/// Ключи `app_settings` для последних выбранных мест (D-25).
const SETTING_LAST_MOVE_PLACE: &str = "repair_last_move_place_id";
const SETTING_LAST_SERVICE_PLACE: &str = "repair_last_service_place_id";

/// Предел длины описания (символов).
const MAX_DESCRIPTION_CHARS: usize = 4000;
/// Предел длины имени (символов) в актах захода.
const MAX_NAME_CHARS: usize = 200;

fn is_staff(caller: &Identity) -> bool {
    matches!(caller.role, Role::Admin | Role::Manager)
}

fn join_err(e: tokio::task::JoinError) -> AppError {
    AppError::Internal {
        source_chain: format!("spawn_blocking: {e}"),
    }
}

/// Полный путь места (`place_full_paths`), `None` если места нет.
fn place_path(conn: &Connection, place_id: Option<i64>) -> Result<Option<String>, AppError> {
    let Some(id) = place_id else {
        return Ok(None);
    };
    conn.query_row(
        "SELECT full_path FROM place_full_paths WHERE place_id = ?1",
        params![id],
        |r| r.get::<_, Option<String>>(0),
    )
    .optional()
    .map_err(map_rusqlite)
    .map(Option::flatten)
}

/// Место существует, не удалено и не в архиве.
fn place_is_live(conn: &Connection, id: i64) -> Result<bool, AppError> {
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM places \
             WHERE id = ?1 AND deleted_at_utc IS NULL AND archived_at_utc IS NULL",
            params![id],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)?;
    Ok(n > 0)
}

/// Application service ремонтных заявок. `Arc`-поля: `Clone` O(1).
#[derive(Clone)]
pub struct RepairService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) repo: Arc<SqliteRepairRepository>,
    pub(crate) request_repo: Arc<SqliteRequestRepository>,
    pub(crate) audit_repo: Arc<SqliteAuditLogRepository>,
    /// Общий WS-отправитель; `None` в фикстурах без подписчиков.
    pub(crate) ws_tx: Option<Arc<tokio::sync::broadcast::Sender<WsEvent>>>,
}

impl RepairService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            repo: Arc::new(SqliteRepairRepository),
            request_repo: Arc::new(SqliteRequestRepository),
            audit_repo: Arc::new(SqliteAuditLogRepository),
            ws_tx: None,
        }
    }

    /// Builder: подключить общий WS-отправитель.
    pub fn with_ws_tx(mut self, ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>) -> Self {
        self.ws_tx = Some(ws_tx);
        self
    }

    /// `EntitiesChanged` после коммита: единственный владелец правила отправки в
    /// `entities_broadcast`; литеральная метка операции равна имени функции-мутации.
    fn broadcast_entities(
        &self,
        op: &'static str,
        place_ids: Vec<i64>,
        device_ids: Vec<i64>,
        group_ids: Vec<i64>,
    ) {
        crate::services::entities_broadcast::send_entities_changed(
            &self.ws_tx,
            op,
            place_ids,
            device_ids,
            group_ids,
        );
    }

    /// Итог перехода: свежая заявка и состояние ремонта; `RequestStatusChanged` участникам.
    async fn finish_transition(
        &self,
        request_id: i64,
        caller: &Identity,
    ) -> Result<RepairTransitionResultDto, AppError> {
        let readers = self.readers.clone();
        let request_repo = self.request_repo.clone();
        let row = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            request_repo.get(&conn, request_id)
        })
        .await
        .map_err(join_err)??;
        let request = RequestDto::from(row);
        if let Some(ws) = &self.ws_tx {
            let participants = self.participants_of(request_id).await;
            let _ = ws.send(WsEvent::RequestStatusChanged {
                request_id,
                new_status: request.status.clone(),
                requested_by_user_id: request.requested_by_user_id,
                participant_user_ids: participants,
            });
        }
        let state = self.get_state(request_id, caller).await?;
        Ok(RepairTransitionResultDto { request, state })
    }

    /// Участники заявки для маршрутизации WS (читается после коммита).
    async fn participants_of(&self, id: i64) -> Vec<i64> {
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.participant_user_ids(&conn, id).unwrap_or_default()
        })
        .await
        .unwrap_or_default()
    }

    // -----------------------------------------------------------------------
    // Чтение
    // -----------------------------------------------------------------------

    /// Опции выпадающего списка устройств формы «Ремонт» (REP-01).
    ///
    /// Администратор и Специалист видят все живые устройства, сотрудник только
    /// устройства своей области (`employee_device_ids`). Поля минимальные (BOPLA):
    /// `in_repair` булев, без id чужой заявки. Сортировка: тип, затем название.
    pub async fn device_options(
        &self,
        caller: &Identity,
    ) -> Result<Vec<RepairDeviceOptionDto>, AppError> {
        authorize(caller, &Action::CreateRequest)?;
        let staff = is_staff(caller);
        let user_id = caller.user_id;
        if !staff && user_id.is_none() {
            return Err(AppError::Forbidden);
        }

        let readers = self.readers.clone();
        let repo = self.repo.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<RepairDeviceOptionDto>, AppError> {
            let conn = readers.acquire();
            let scope: Option<std::collections::HashSet<i64>> = if staff {
                None
            } else {
                Some(
                    repo.employee_device_ids(&conn, user_id.unwrap_or(-1))?
                        .into_iter()
                        .collect(),
                )
            };
            let mut stmt = conn
                .prepare(
                    "SELECT d.id, d.name, d.inventory_number, dt.name, pfp.full_path \
                     FROM devices d \
                     JOIN device_types dt ON dt.id = d.type_id \
                     LEFT JOIN place_full_paths pfp ON pfp.place_id = d.place_id \
                     WHERE d.deleted_at_utc IS NULL \
                     ORDER BY dt.name, d.name, d.id",
                )
                .map_err(map_rusqlite)?;
            let mut items = stmt
                .query_map([], |r| {
                    Ok(RepairDeviceOptionDto {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        inventory_no: r.get(2)?,
                        type_name: r.get(3)?,
                        place: r.get(4)?,
                        in_repair: false,
                    })
                })
                .map_err(map_rusqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_rusqlite)?;
            if let Some(scope) = scope {
                items.retain(|o| scope.contains(&o.id));
            }
            let ids: Vec<i64> = items.iter().map(|o| o.id).collect();
            let busy: std::collections::HashSet<i64> = repo
                .open_repair_requests_for_devices(&conn, &ids)?
                .into_iter()
                .map(|(d, _)| d)
                .collect();
            for o in &mut items {
                o.in_repair = busy.contains(&o.id);
            }
            Ok(items)
        })
        .await
        .map_err(join_err)?
    }

    /// Открытые ремонтные заявки среди `device_ids` (пометка «уже в ремонте» персоналу).
    pub async fn open_requests_for_devices(
        &self,
        device_ids: Vec<i64>,
        caller: &Identity,
    ) -> Result<Vec<OpenRepairRefDto>, AppError> {
        authorize(caller, &Action::TransitionRequests)?;
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            Ok(repo
                .open_repair_requests_for_devices(&conn, &device_ids)?
                .into_iter()
                .map(|(device_id, request_id)| OpenRepairRefDto {
                    device_id,
                    request_id,
                })
                .collect())
        })
        .await
        .map_err(join_err)?
    }

    /// D-25: последние выбранные места. Удалённое или архивное место даёт `None`.
    pub async fn last_places(&self, caller: &Identity) -> Result<RepairLastPlacesDto, AppError> {
        authorize(caller, &Action::TransitionRequests)?;
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let read = |key: &str| -> Result<Option<i64>, AppError> {
                let Some(raw) = repo.read_setting(&conn, key)? else {
                    return Ok(None);
                };
                let Ok(id) = raw.trim().parse::<i64>() else {
                    return Ok(None);
                };
                Ok(place_is_live(&conn, id)?.then_some(id))
            };
            Ok(RepairLastPlacesDto {
                move_place_id: read(SETTING_LAST_MOVE_PLACE)?,
                service_place_id: read(SETTING_LAST_SERVICE_PLACE)?,
            })
        })
        .await
        .map_err(join_err)?
    }

    /// Состояние ремонта для карточки заявки. Доступно автору, участникам,
    /// Администратору и Специалисту; только для `request_type = repair`.
    pub async fn get_state(
        &self,
        request_id: i64,
        caller: &Identity,
    ) -> Result<RepairStateDto, AppError> {
        authorize(caller, &Action::CreateRequest)?;
        let staff = is_staff(caller);
        let user_id = caller.user_id;
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let request_repo = self.request_repo.clone();
        tokio::task::spawn_blocking(move || -> Result<RepairStateDto, AppError> {
            let conn = readers.acquire();
            let row = request_repo.get(&conn, request_id)?;
            let participants = repo.participant_user_ids(&conn, request_id)?;
            if !staff {
                let me = user_id.unwrap_or(-1);
                if row.requested_by_user_id != me && !participants.contains(&me) {
                    return Err(AppError::Forbidden);
                }
            }
            if row.request_type != REPAIR_REQUEST_TYPE {
                return Err(AppError::Validation {
                    field: "request_id".into(),
                    message: "Заявка не является ремонтной.".into(),
                });
            }
            let (origin_place_id, moved) = repo.get_repair_fields(&conn, request_id)?;
            let device_id = row.printer_device_id;

            let mut device_name = None;
            let mut status_code = None;
            let mut current_place_id = None;
            let mut is_anchor = false;
            let mut group_nested = false;
            let mut block_size = 0_i64;
            let mut member_group_place: Option<i64> = None;
            if let Some(dev) = device_id {
                let d: Option<(String, Option<String>, Option<i64>)> = conn
                    .query_row(
                        "SELECT d.name, ds.code, d.place_id FROM devices d \
                         LEFT JOIN device_statuses ds ON ds.id = d.status_id WHERE d.id = ?1",
                        params![dev],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                    )
                    .optional()
                    .map_err(map_rusqlite)?;
                if let Some((n, s, p)) = d {
                    device_name = Some(n);
                    status_code = s;
                    current_place_id = p;
                }
                let anchor: Option<(i64, Option<i64>)> = conn
                    .query_row(
                        "SELECT id, parent_group_id FROM groups \
                         WHERE anchor_device_id = ?1 AND deleted_at_utc IS NULL",
                        params![dev],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()
                    .map_err(map_rusqlite)?;
                if let Some((gid, parent)) = anchor {
                    is_anchor = true;
                    group_nested = parent.is_some();
                    block_size = conn
                        .query_row(
                            "WITH RECURSIVE sub(id) AS ( \
                                 SELECT ?1 \
                                 UNION \
                                 SELECT g.id FROM groups g JOIN sub ON g.parent_group_id = sub.id \
                                 WHERE g.deleted_at_utc IS NULL \
                             ) \
                             SELECT COUNT(*) FROM group_devices gd \
                             JOIN sub ON gd.group_id = sub.id \
                             JOIN devices d ON d.id = gd.device_id AND d.deleted_at_utc IS NULL \
                             WHERE gd.device_id <> ?2",
                            params![gid, dev],
                            |r| r.get(0),
                        )
                        .map_err(map_rusqlite)?;
                } else {
                    member_group_place = conn
                        .query_row(
                            "SELECT g.place_id FROM group_devices gd \
                             JOIN groups g ON g.id = gd.group_id AND g.deleted_at_utc IS NULL \
                             WHERE gd.device_id = ?1",
                            params![dev],
                            |r| r.get::<_, Option<i64>>(0),
                        )
                        .optional()
                        .map_err(map_rusqlite)?
                        .flatten();
                }
            }

            // D-16 / D-18: якорь возвращается блоком на сохранённое место; член группы,
            // не якорный, на текущее место группы; прочие на сохранённое.
            let suggested_return_place_id = if is_anchor {
                origin_place_id
            } else {
                member_group_place.or(origin_place_id)
            };

            let scan_counts: std::collections::HashMap<i64, i64> = repo
                .scan_counts_for_request(&conn, request_id)?
                .into_iter()
                .collect();
            let visit_rows = repo.list_visits_for_request(&conn, request_id)?;
            let has_open_visit = visit_rows.iter().any(|v| v.returned_at_utc.is_none());
            let visit_count = visit_rows.len();
            let mut visits = Vec::with_capacity(visit_count);
            for v in visit_rows {
                visits.push(RepairVisitDto {
                    id: v.id,
                    request_id: v.request_id,
                    device_id: v.device_id,
                    seq: v.seq,
                    service_place_id: v.service_place_id,
                    service_place_path: place_path(&conn, Some(v.service_place_id))?,
                    from_place_id: v.from_place_id,
                    from_place_path: place_path(&conn, v.from_place_id)?,
                    sent_at_utc: v.sent_at_utc,
                    returned_at_utc: v.returned_at_utc,
                    giver_name: v.giver_name,
                    receiver_name: v.receiver_name,
                    fault_description: v.fault_description,
                    returned_to_place_id: v.returned_to_place_id,
                    returned_to_place_path: place_path(&conn, v.returned_to_place_id)?,
                    received_by_name: v.received_by_name,
                    works_done: v.works_done,
                    cost_kopecks: v.cost_kopecks,
                    cost_display: v.cost_kopecks.map(format_kopecks),
                    act_number: v.act_number,
                    scan_count: scan_counts.get(&v.id).copied().unwrap_or(0),
                    version: v.version,
                });
            }

            let can_cancel_repair =
                staff && row.status == "in_progress" && moved && visit_count == 0;

            Ok(RepairStateDto {
                request_id,
                device_id,
                device_name,
                device_status_code: status_code,
                current_place_id,
                current_place_path: place_path(&conn, current_place_id)?,
                origin_place_id,
                origin_place_path: place_path(&conn, origin_place_id)?,
                moved,
                is_anchor,
                group_nested,
                group_block_size: block_size,
                suggested_return_place_id,
                suggested_return_place_path: place_path(&conn, suggested_return_place_id)?,
                has_open_visit,
                visits,
                can_cancel_repair,
            })
        })
        .await
        .map_err(join_err)?
    }

    // -----------------------------------------------------------------------
    // Создание
    // -----------------------------------------------------------------------

    /// Создать ремонтную заявку или присоединиться к открытой (D-19, D-20, D-24).
    ///
    /// Единственный путь создания `request_type = repair`: `RequestService::create`
    /// такой тип отклоняет allowlist'ом.
    pub async fn create(
        &self,
        payload: RepairCreateDto,
        caller: &Identity,
    ) -> Result<RepairCreateResultDto, AppError> {
        authorize(caller, &Action::CreateRequest)?;

        let description = payload.description.trim().to_string();
        if description.is_empty() {
            return Err(AppError::Validation {
                field: "description".into(),
                message: "Опишите неисправность.".into(),
            });
        }
        if description.chars().count() > MAX_DESCRIPTION_CHARS {
            return Err(AppError::Validation {
                field: "description".into(),
                message: format!("Описание длиннее {MAX_DESCRIPTION_CHARS} символов."),
            });
        }

        let staff = is_staff(caller);
        let user_id = caller.user_id;
        if !staff && user_id.is_none() {
            return Err(AppError::Forbidden);
        }
        let device_id = payload.device_id;
        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
        let request_repo = self.request_repo.clone();
        let audit_repo = self.audit_repo.clone();

        let new = RequestNew {
            request_type: REPAIR_REQUEST_TYPE.to_string(),
            requested_by_user_id: user_id.unwrap_or(1), // trusted_admin fallback
            printer_device_id: device_id,
            cartridge_model_id: None,
            category_id: None, // D-24: категория «Ремонт техники» не назначается
            description: Some(description.clone()),
            ad_subtype: None,
        };

        let (request_id, joined) = self
            .writer
            .execute(move |conn| {
                if let Some(dev) = device_id {
                    if staff {
                        let live: i64 = conn
                            .query_row(
                                "SELECT COUNT(*) FROM devices WHERE id = ?1 \
                                 AND deleted_at_utc IS NULL",
                                params![dev],
                                |r| r.get(0),
                            )
                            .map_err(map_rusqlite)?;
                        if live == 0 {
                            return Err(AppError::NotFound {
                                entity: "device",
                                id: dev,
                            });
                        }
                    } else {
                        // BOLA: область сотрудника определяется на сервере.
                        let scope = repo.employee_device_ids(conn, user_id.unwrap_or(-1))?;
                        if !scope.contains(&dev) {
                            return Err(AppError::Forbidden);
                        }
                    }
                }
                let attempt = |conn: &mut Connection, allow_insert: bool| {
                    try_create(
                        conn,
                        &repo,
                        &request_repo,
                        &audit_repo,
                        &new,
                        &description,
                        user_id,
                        now,
                        allow_insert,
                    )
                };
                match attempt(conn, true) {
                    // D-19: проигравший гонку присоединяется, а не получает ошибку.
                    Err(AppError::Conflict { .. }) if device_id.is_some() => attempt(conn, false),
                    other => other,
                }
            })
            .await?;

        let readers = self.readers.clone();
        let request_repo = self.request_repo.clone();
        let row = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            request_repo.get(&conn, request_id)
        })
        .await
        .map_err(join_err)??;
        let dto = RequestDto::from(row);

        if let Some(ws) = &self.ws_tx {
            if joined {
                let participants = self.participants_of(request_id).await;
                let _ = ws.send(WsEvent::RequestStatusChanged {
                    request_id,
                    new_status: dto.status.clone(),
                    requested_by_user_id: dto.requested_by_user_id,
                    participant_user_ids: participants,
                });
            } else {
                let _ = ws.send(WsEvent::NewRequest {
                    request_id,
                    request_type: dto.request_type.clone(),
                    requester_name: dto.requester_name.clone().unwrap_or_default(),
                });
            }
        }

        Ok(RepairCreateResultDto {
            request: dto,
            joined,
        })
    }

    // -----------------------------------------------------------------------
    // Переходы цикла: принятие и «Забрать на склад»
    // -----------------------------------------------------------------------

    /// Принять заявку в работу (REP-02, D-01, D-03, D-24).
    ///
    /// С устройством: статус «Ожидает ремонта» при любом выборе (D-23: прежний статус
    /// не блокирует), перемещение по выбору. Без устройства: только смена статуса заявки.
    pub async fn accept(
        &self,
        request_id: i64,
        version: i64,
        choice: RepairMoveChoiceDto,
        caller: &Identity,
    ) -> Result<RepairTransitionResultDto, AppError> {
        authorize(caller, &Action::TransitionRequests)?;
        let user_id = caller.user_id;
        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
        let request_repo = self.request_repo.clone();
        let audit_repo = self.audit_repo.clone();

        let (place_ids, device_ids, group_ids) = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let row = request_repo.get(&tx, request_id)?;
                ensure_repair(&row.request_type)?;
                if row.status != "open" {
                    return Err(AppError::Validation {
                        field: "status".into(),
                        message: "Заявка уже принята в работу.".into(),
                    });
                }
                repo.set_request_status_in_tx(
                    &tx,
                    request_id,
                    version,
                    "in_progress",
                    user_id,
                    now,
                )?;
                let mut ids = (Vec::new(), Vec::new(), Vec::new());
                let mut payload = serde_json::json!({ "device_id": row.printer_device_id });
                if let Some(dev) = row.printer_device_id {
                    let target = match choice {
                        RepairMoveChoiceDto::Move { place_id } => Some(place_id),
                        RepairMoveChoiceDto::Stay => None,
                    };
                    let deps = GroupMoveDeps::new();
                    let out = apply_device_state_in_tx(
                        &tx,
                        &deps,
                        &repo,
                        dev,
                        Some(DEVICE_STATUS_CODE_AWAITING_REPAIR),
                        target,
                        request_id,
                        user_id,
                        now,
                    )?;
                    if let Some(place) = target {
                        remember_first_move(&tx, &repo, request_id, &out, now)?;
                        repo.upsert_setting(&tx, SETTING_LAST_MOVE_PLACE, &place.to_string(), now)?;
                        payload["moved_to"] = serde_json::json!(place);
                    }
                    ids = (out.places, out.devices, out.groups);
                }
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "request",
                        entity_id: request_id,
                        action: "custom:repair_accept",
                        user_id,
                        before_json: None,
                        after_json: None,
                        payload_json: Some(payload.to_string()),
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(ids)
            })
            .await?;

        self.broadcast_entities("accept", place_ids, device_ids, group_ids);
        self.finish_transition(request_id, caller).await
    }

    /// «Забрать на склад» (REP-03): переместить устройство, пока оно не в сервисе.
    /// Статус устройства не меняется.
    pub async fn move_to_storage(
        &self,
        request_id: i64,
        version: i64,
        place_id: i64,
        caller: &Identity,
    ) -> Result<RepairTransitionResultDto, AppError> {
        authorize(caller, &Action::TransitionRequests)?;
        let user_id = caller.user_id;
        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
        let request_repo = self.request_repo.clone();

        let (place_ids, device_ids, group_ids) = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let row = request_repo.get(&tx, request_id)?;
                ensure_repair(&row.request_type)?;
                if row.version != version {
                    return Err(AppError::OptimisticLockMismatch {
                        entity: "request",
                        id: request_id,
                        expected: version,
                        actual: row.version,
                    });
                }
                if row.status != "in_progress" {
                    return Err(AppError::Validation {
                        field: "status".into(),
                        message: "Перемещение доступно только для заявки в работе.".into(),
                    });
                }
                let Some(dev) = row.printer_device_id else {
                    return Err(AppError::Validation {
                        field: "device_id".into(),
                        message: "В заявке не указано устройство.".into(),
                    });
                };
                if repo.open_visit_for_request(&tx, request_id)?.is_some() {
                    return Err(AppError::Validation {
                        field: "status".into(),
                        message: "Устройство находится в сервисе.".into(),
                    });
                }
                let deps = GroupMoveDeps::new();
                let out = apply_device_state_in_tx(
                    &tx,
                    &deps,
                    &repo,
                    dev,
                    None,
                    Some(place_id),
                    request_id,
                    user_id,
                    now,
                )?;
                remember_first_move(&tx, &repo, request_id, &out, now)?;
                repo.upsert_setting(&tx, SETTING_LAST_MOVE_PLACE, &place_id.to_string(), now)?;
                tx.commit().map_err(map_rusqlite)?;
                Ok((out.places, out.devices, out.groups))
            })
            .await?;

        self.broadcast_entities("move_to_storage", place_ids, device_ids, group_ids);
        self.finish_transition(request_id, caller).await
    }

    // -----------------------------------------------------------------------
    // Заход в сервис: отправка, забор, сканы
    // -----------------------------------------------------------------------

    /// «Отправить в ремонт» (REP-04, D-04, D-05, D-13): заход со своим номером акта.
    pub async fn send(
        &self,
        dto: RepairSendDto,
        caller: &Identity,
    ) -> Result<RepairTransitionResultDto, AppError> {
        authorize(caller, &Action::TransitionRequests)?;
        let giver = required_text(&dto.giver_name, "giver_name", "Укажите, кто передал.")?;
        let receiver = required_text(&dto.receiver_name, "receiver_name", "Укажите, кто принял.")?;
        let fault = required_text(
            &dto.fault_description,
            "fault_description",
            "Опишите неисправность.",
        )?;
        let user_id = caller.user_id;
        let now = self.clock.unix_seconds();
        check_date(dto.date_utc, now)?;
        let (request_id, version, date, service_place) = (
            dto.request_id,
            dto.version,
            dto.date_utc,
            dto.service_place_id,
        );
        let repo = self.repo.clone();
        let request_repo = self.request_repo.clone();
        let audit_repo = self.audit_repo.clone();

        let (place_ids, device_ids, group_ids) = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let row = request_repo.get(&tx, request_id)?;
                ensure_repair(&row.request_type)?;
                if row.status != "in_progress" {
                    return Err(AppError::Validation {
                        field: "status".into(),
                        message: "Отправить в ремонт можно только заявку в работе.".into(),
                    });
                }
                let Some(dev) = row.printer_device_id else {
                    return Err(AppError::Validation {
                        field: "device_id".into(),
                        message: "Сначала укажите устройство.".into(),
                    });
                };
                if repo.open_visit_for_request(&tx, request_id)?.is_some() {
                    return Err(AppError::Validation {
                        field: "status".into(),
                        message: "Устройство уже находится в сервисе.".into(),
                    });
                }
                if !place_is_live(&tx, service_place)? {
                    return Err(AppError::Validation {
                        field: "service_place_id".into(),
                        message: "Место сервиса не найдено или в архиве: выберите другое.".into(),
                    });
                }
                let from_place: Option<i64> = tx
                    .query_row(
                        "SELECT place_id FROM devices WHERE id = ?1",
                        params![dev],
                        |r| r.get(0),
                    )
                    .map_err(map_rusqlite)?;
                let act_number = issue_repair_act_number_in_tx(&tx, now)?;
                let visit_id = repo.insert_visit_in_tx(
                    &tx,
                    &RepairVisitNew {
                        request_id,
                        device_id: dev,
                        service_place_id: service_place,
                        from_place_id: from_place,
                        sent_at_utc: date,
                        giver_name: giver,
                        receiver_name: receiver,
                        fault_description: fault,
                        act_number: act_number.clone(),
                        created_by_user_id: user_id,
                    },
                    now,
                )?;
                let deps = GroupMoveDeps::new();
                let out = apply_device_state_in_tx(
                    &tx,
                    &deps,
                    &repo,
                    dev,
                    Some(DEVICE_STATUS_CODE_IN_REPAIR),
                    Some(service_place),
                    request_id,
                    user_id,
                    now,
                )?;
                remember_first_move(&tx, &repo, request_id, &out, now)?;
                repo.set_request_status_in_tx(&tx, request_id, version, "in_repair", user_id, now)?;
                repo.upsert_setting(
                    &tx,
                    SETTING_LAST_SERVICE_PLACE,
                    &service_place.to_string(),
                    now,
                )?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "request",
                        entity_id: request_id,
                        action: "custom:repair_send",
                        user_id,
                        before_json: None,
                        after_json: None,
                        payload_json: Some(
                            serde_json::json!({
                                "visit_id": visit_id,
                                "service_place_id": service_place,
                                "act_number": act_number,
                            })
                            .to_string(),
                        ),
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok((out.places, out.devices, out.groups))
            })
            .await?;

        self.broadcast_entities("send", place_ids, device_ids, group_ids);
        self.broadcast_number_space_changed();
        self.finish_transition(request_id, caller).await
    }

    /// Номер акта выдан в транзакции `send`: открытые формы с превью номера обновляются.
    fn broadcast_number_space_changed(&self) {
        if let Some(ws) = &self.ws_tx {
            let _ = ws.send(WsEvent::NumberSpaceChanged {
                contexts: vec!["repair_act_create".to_string()],
            });
        }
    }

    /// «Забрать из ремонта» (REP-05, D-07, D-08, D-09, D-29).
    ///
    /// С отметкой `return_to_workplace` забор и завершение заявки выполняются одной
    /// транзакцией writer: сбой на любом шаге ничего не оставляет.
    pub async fn receive(
        &self,
        dto: RepairReceiveDto,
        caller: &Identity,
    ) -> Result<RepairTransitionResultDto, AppError> {
        authorize(caller, &Action::TransitionRequests)?;
        let received_by = required_text(
            &dto.received_by_name,
            "received_by_name",
            "Укажите, кто принял.",
        )?;
        let works_done = optional_text(dto.works_done.as_deref(), "works_done")?;
        let cost = parse_cost_kopecks(dto.cost.as_deref().unwrap_or("")).map_err(|message| {
            AppError::Validation {
                field: "cost".into(),
                message,
            }
        })?;
        let user_id = caller.user_id;
        let now = self.clock.unix_seconds();
        check_date(dto.date_utc, now)?;
        let (request_id, version, date, place_id, to_workplace) = (
            dto.request_id,
            dto.version,
            dto.date_utc,
            dto.place_id,
            dto.return_to_workplace,
        );
        let repo = self.repo.clone();
        let request_repo = self.request_repo.clone();
        let audit_repo = self.audit_repo.clone();

        let (place_ids, device_ids, group_ids) = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let row = request_repo.get(&tx, request_id)?;
                ensure_repair(&row.request_type)?;
                if row.status != "in_repair" {
                    return Err(AppError::Validation {
                        field: "status".into(),
                        message: "Забрать из ремонта можно только заявку «В ремонте».".into(),
                    });
                }
                let Some(visit) = repo.open_visit_for_request(&tx, request_id)? else {
                    return Err(AppError::Validation {
                        field: "status".into(),
                        message: "У заявки нет открытого захода в сервис.".into(),
                    });
                };
                if date < visit.sent_at_utc {
                    return Err(AppError::Validation {
                        field: "date_utc".into(),
                        message: "Дата возврата раньше даты отправки.".into(),
                    });
                }
                repo.close_visit_in_tx(
                    &tx,
                    visit.id,
                    visit.version,
                    date,
                    Some(place_id),
                    Some(&received_by),
                    works_done.as_deref(),
                    cost,
                    now,
                )?;
                let status_code = if to_workplace {
                    status_code_for_place(&tx, place_id)?
                } else {
                    DEVICE_STATUS_CODE_AWAITING_REPAIR
                };
                let deps = GroupMoveDeps::new();
                let out = apply_device_state_in_tx(
                    &tx,
                    &deps,
                    &repo,
                    visit.device_id,
                    Some(status_code),
                    Some(place_id),
                    request_id,
                    user_id,
                    now,
                )?;
                let new_status = if to_workplace {
                    "completed"
                } else {
                    "in_progress"
                };
                repo.set_request_status_in_tx(&tx, request_id, version, new_status, user_id, now)?;
                if !to_workplace {
                    repo.upsert_setting(&tx, SETTING_LAST_MOVE_PLACE, &place_id.to_string(), now)?;
                }
                let mut entry = AuditEntry {
                    entity_type: "request",
                    entity_id: request_id,
                    action: "custom:repair_receive",
                    user_id,
                    before_json: None,
                    after_json: None,
                    payload_json: Some(
                        serde_json::json!({
                            "visit_id": visit.id,
                            "returned_to_place_id": place_id,
                            "return_to_workplace": to_workplace,
                        })
                        .to_string(),
                    ),
                    created_at_utc: now,
                };
                audit_repo.insert(&tx, entry.clone())?;
                if to_workplace {
                    entry.action = "custom:repair_complete";
                    audit_repo.insert(&tx, entry)?;
                }
                tx.commit().map_err(map_rusqlite)?;
                Ok((out.places, out.devices, out.groups))
            })
            .await?;

        self.broadcast_entities("receive", place_ids, device_ids, group_ids);
        self.finish_transition(request_id, caller).await
    }

    // -----------------------------------------------------------------------
    // Выход из цикла: завершить, отменить ремонт, отклонить, отмена автором,
    // «Указать устройство»
    // -----------------------------------------------------------------------

    /// «Завершить» (REP-06, D-09, D-10, D-16, D-18): статус устройства по месту возврата.
    pub async fn complete(
        &self,
        dto: RepairCompleteDto,
        caller: &Identity,
    ) -> Result<RepairTransitionResultDto, AppError> {
        authorize(caller, &Action::TransitionRequests)?;
        let notes = optional_text(dto.notes.as_deref(), "notes")?;
        let user_id = caller.user_id;
        let now = self.clock.unix_seconds();
        let (request_id, version, place_arg) = (dto.request_id, dto.version, dto.place_id);
        let repo = self.repo.clone();
        let request_repo = self.request_repo.clone();
        let audit_repo = self.audit_repo.clone();

        let (place_ids, device_ids, group_ids) = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let row = request_repo.get(&tx, request_id)?;
                ensure_repair(&row.request_type)?;
                if row.status == "in_repair" {
                    return Err(AppError::Validation {
                        field: "status".into(),
                        message: "Сначала заберите устройство из ремонта.".into(),
                    });
                }
                if row.status != "in_progress" {
                    return Err(AppError::Validation {
                        field: "status".into(),
                        message: "Завершить можно только заявку в работе.".into(),
                    });
                }
                if row.version != version {
                    return Err(AppError::OptimisticLockMismatch {
                        entity: "request",
                        id: request_id,
                        expected: version,
                        actual: row.version,
                    });
                }
                let (_, moved) = repo.get_repair_fields(&tx, request_id)?;
                let mut ids = (Vec::new(), Vec::new(), Vec::new());
                let mut payload = serde_json::json!({ "device_id": row.printer_device_id });
                if let Some(dev) = row.printer_device_id {
                    let deps = GroupMoveDeps::new();
                    let out = if moved {
                        let Some(place) = place_arg else {
                            return Err(AppError::Validation {
                                field: "place_id".into(),
                                message: "Укажите место возврата устройства.".into(),
                            });
                        };
                        let code = status_code_for_place(&tx, place)?;
                        payload["returned_to_place_id"] = serde_json::json!(place);
                        apply_device_state_in_tx(
                            &tx,
                            &deps,
                            &repo,
                            dev,
                            Some(code),
                            Some(place),
                            request_id,
                            user_id,
                            now,
                        )?
                    } else {
                        let code = status_code_for_current_place(&tx, dev)?;
                        apply_device_state_in_tx(
                            &tx,
                            &deps,
                            &repo,
                            dev,
                            Some(code),
                            None,
                            request_id,
                            user_id,
                            now,
                        )?
                    };
                    ids = (out.places, out.devices, out.groups);
                }
                request_repo.transition_in_tx(
                    &tx,
                    request_id,
                    version,
                    &RequestTransitionOp::Complete {
                        notes,
                        linked_cartridge_id: None,
                    },
                    None,
                    None,
                    now,
                )?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "request",
                        entity_id: request_id,
                        action: "custom:repair_complete",
                        user_id,
                        before_json: None,
                        after_json: None,
                        payload_json: Some(payload.to_string()),
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(ids)
            })
            .await?;

        self.broadcast_entities("complete", place_ids, device_ids, group_ids);
        self.finish_transition(request_id, caller).await
    }

    /// «Отменить ремонт» (REP-07, D-12): только после перемещения и без заходов.
    pub async fn cancel_repair(
        &self,
        dto: RepairCancelDto,
        caller: &Identity,
    ) -> Result<RepairTransitionResultDto, AppError> {
        authorize(caller, &Action::TransitionRequests)?;
        let user_id = caller.user_id;
        let now = self.clock.unix_seconds();
        let (request_id, version) = (dto.request_id, dto.version);
        let repo = self.repo.clone();
        let request_repo = self.request_repo.clone();
        let audit_repo = self.audit_repo.clone();

        let (place_ids, device_ids, group_ids) = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let row = request_repo.get(&tx, request_id)?;
                ensure_repair(&row.request_type)?;
                let visits = repo.count_visits(&tx, request_id)?;
                if visits > 0 || row.status == "in_repair" {
                    return Err(AppError::Validation {
                        field: "status".into(),
                        message: "Отмена ремонта недоступна после передачи в сервис; \
                                  завершите заявку."
                            .into(),
                    });
                }
                if row.status != "in_progress" {
                    return Err(AppError::Validation {
                        field: "status".into(),
                        message: "Отменить ремонт можно только заявку в работе.".into(),
                    });
                }
                let (origin, moved) = repo.get_repair_fields(&tx, request_id)?;
                if !moved {
                    return Err(AppError::Validation {
                        field: "status".into(),
                        message: "Устройство не перемещалось: используйте «Отклонить».".into(),
                    });
                }
                let mut ids = (Vec::new(), Vec::new(), Vec::new());
                if let Some(dev) = row.printer_device_id {
                    let deps = GroupMoveDeps::new();
                    let code = match origin {
                        Some(p) => status_code_for_place(&tx, p)?,
                        None => status_code_for_current_place(&tx, dev)?,
                    };
                    let out = apply_device_state_in_tx(
                        &tx,
                        &deps,
                        &repo,
                        dev,
                        Some(code),
                        origin,
                        request_id,
                        user_id,
                        now,
                    )?;
                    ids = (out.places, out.devices, out.groups);
                }
                repo.set_request_status_in_tx(&tx, request_id, version, "cancelled", None, now)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "request",
                        entity_id: request_id,
                        action: "custom:repair_cancel",
                        user_id,
                        before_json: None,
                        after_json: None,
                        payload_json: Some(
                            serde_json::json!({
                                "device_id": row.printer_device_id,
                                "returned_to_place_id": origin,
                            })
                            .to_string(),
                        ),
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(ids)
            })
            .await?;

        self.broadcast_entities("cancel_repair", place_ids, device_ids, group_ids);
        self.finish_transition(request_id, caller).await
    }

    /// «Отклонить» ремонтную заявку (D-11): до первого перемещения.
    pub async fn reject(
        &self,
        dto: RepairRejectDto,
        caller: &Identity,
    ) -> Result<RepairTransitionResultDto, AppError> {
        authorize(caller, &Action::TransitionRequests)?;
        let notes = optional_text(dto.notes.as_deref(), "notes")?;
        let user_id = caller.user_id;
        let now = self.clock.unix_seconds();
        let (request_id, version) = (dto.request_id, dto.version);
        let repo = self.repo.clone();
        let request_repo = self.request_repo.clone();
        let audit_repo = self.audit_repo.clone();

        let (place_ids, device_ids, group_ids) = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let row = request_repo.get(&tx, request_id)?;
                ensure_repair(&row.request_type)?;
                if row.status != "open" && row.status != "in_progress" {
                    return Err(AppError::Validation {
                        field: "status".into(),
                        message: "Отклонить можно только новую заявку или заявку в работе.".into(),
                    });
                }
                let ids = release_device_in_tx(&tx, &repo, &row, user_id, now)?;
                request_repo.transition_in_tx(
                    &tx,
                    request_id,
                    version,
                    &RequestTransitionOp::Reject { notes },
                    None,
                    None,
                    now,
                )?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "request",
                        entity_id: request_id,
                        action: "custom:reject",
                        user_id,
                        before_json: None,
                        after_json: None,
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(ids)
            })
            .await?;

        self.broadcast_entities("reject", place_ids, device_ids, group_ids);
        self.finish_transition(request_id, caller).await
    }

    /// Отмена автором (D-11): владение проверяется на сервере.
    pub async fn cancel_by_author(
        &self,
        dto: RepairCancelDto,
        caller: &Identity,
    ) -> Result<RepairTransitionResultDto, AppError> {
        authorize(caller, &Action::CancelOwnRequest)?;
        let staff = is_staff(caller);
        let user_id = caller.user_id;
        let now = self.clock.unix_seconds();
        let (request_id, version) = (dto.request_id, dto.version);
        let repo = self.repo.clone();
        let request_repo = self.request_repo.clone();
        let audit_repo = self.audit_repo.clone();

        let (place_ids, device_ids, group_ids) = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let row = request_repo.get(&tx, request_id)?;
                ensure_repair(&row.request_type)?;
                if !staff && user_id != Some(row.requested_by_user_id) {
                    return Err(AppError::Forbidden);
                }
                if row.status != "open" && row.status != "in_progress" {
                    return Err(AppError::Validation {
                        field: "status".into(),
                        message: "Отменить можно только новую заявку или заявку в работе.".into(),
                    });
                }
                let ids = release_device_in_tx(&tx, &repo, &row, user_id, now)?;
                repo.set_request_status_in_tx(&tx, request_id, version, "cancelled", None, now)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "request",
                        entity_id: request_id,
                        action: "custom:cancel",
                        user_id,
                        before_json: None,
                        after_json: None,
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(ids)
            })
            .await?;

        self.broadcast_entities("cancel_by_author", place_ids, device_ids, group_ids);
        self.finish_transition(request_id, caller).await
    }

    /// «Указать устройство» для заявки «Другое» в работе (D-24).
    pub async fn assign_device(
        &self,
        dto: RepairAssignDeviceDto,
        caller: &Identity,
    ) -> Result<RepairAssignResultDto, AppError> {
        authorize(caller, &Action::TransitionRequests)?;
        let user_id = caller.user_id;
        let now = self.clock.unix_seconds();
        let (request_id, version, device_id) = (dto.request_id, dto.version, dto.device_id);
        let repo = self.repo.clone();
        let request_repo = self.request_repo.clone();
        let audit_repo = self.audit_repo.clone();

        let (conflict, place_ids, device_ids, group_ids) = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let row = request_repo.get(&tx, request_id)?;
                ensure_repair(&row.request_type)?;
                if row.status != "in_progress" {
                    return Err(AppError::Validation {
                        field: "status".into(),
                        message: "Указать устройство можно только в заявке в работе.".into(),
                    });
                }
                if row.printer_device_id.is_some() {
                    return Err(AppError::Validation {
                        field: "device_id".into(),
                        message: "В заявке уже указано устройство.".into(),
                    });
                }
                if row.version != version {
                    return Err(AppError::OptimisticLockMismatch {
                        entity: "request",
                        id: request_id,
                        expected: version,
                        actual: row.version,
                    });
                }
                let live: i64 = tx
                    .query_row(
                        "SELECT COUNT(*) FROM devices WHERE id = ?1 AND deleted_at_utc IS NULL",
                        params![device_id],
                        |r| r.get(0),
                    )
                    .map_err(map_rusqlite)?;
                if live == 0 {
                    return Err(AppError::NotFound {
                        entity: "device",
                        id: device_id,
                    });
                }
                if let Some(other) = repo.open_repair_request_id(&tx, device_id)? {
                    return Ok((Some(other), Vec::new(), Vec::new(), Vec::new()));
                }
                repo.set_request_device_in_tx(&tx, request_id, device_id, now)?;
                let deps = GroupMoveDeps::new();
                let out = apply_device_state_in_tx(
                    &tx,
                    &deps,
                    &repo,
                    device_id,
                    Some(DEVICE_STATUS_CODE_AWAITING_REPAIR),
                    None,
                    request_id,
                    user_id,
                    now,
                )?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "request",
                        entity_id: request_id,
                        action: "custom:repair_assign_device",
                        user_id,
                        before_json: None,
                        after_json: None,
                        payload_json: Some(
                            serde_json::json!({ "device_id": device_id }).to_string(),
                        ),
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok((None, out.places, out.devices, out.groups))
            })
            .await?;

        if let Some(other) = conflict {
            return Ok(RepairAssignResultDto {
                request: None,
                conflict_request_id: Some(other),
            });
        }
        self.broadcast_entities("assign_device", place_ids, device_ids, group_ids);
        let done = self.finish_transition(request_id, caller).await?;
        Ok(RepairAssignResultDto {
            request: Some(done.request),
            conflict_request_id: None,
        })
    }

    /// D-06: связать вложения заявки с заходом явной записью. Всё или ничего.
    pub async fn link_scans(
        &self,
        visit_id: i64,
        attachment_ids: Vec<i64>,
        caller: &Identity,
    ) -> Result<RepairStateDto, AppError> {
        authorize(caller, &Action::TransitionRequests)?;
        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
        let request_id = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let Some(visit) = repo.get_visit(&tx, visit_id)? else {
                    return Err(AppError::NotFound {
                        entity: "repair_visit",
                        id: visit_id,
                    });
                };
                for att in &attachment_ids {
                    let ok: i64 = tx
                        .query_row(
                            "SELECT COUNT(*) FROM attachments WHERE id = ?1 \
                             AND parent_kind = 'request' AND parent_id = ?2 \
                             AND deleted_at_utc IS NULL",
                            params![att, visit.request_id],
                            |r| r.get(0),
                        )
                        .map_err(map_rusqlite)?;
                    if ok == 0 {
                        return Err(AppError::Validation {
                            field: "attachment_ids".into(),
                            message: "Файл не относится к этой заявке или удалён.".into(),
                        });
                    }
                    repo.link_scan_in_tx(&tx, visit_id, *att, now)?;
                }
                tx.commit().map_err(map_rusqlite)?;
                Ok(visit.request_id)
            })
            .await?;
        self.get_state(request_id, caller).await
    }
}

/// Одна транзакция «присоединиться или создать». `allow_insert = false` — повтор после
/// проигранной гонки: если открытой заявки всё же нет, возвращается ошибка вставки.
#[allow(clippy::too_many_arguments)]
fn try_create(
    conn: &mut Connection,
    repo: &SqliteRepairRepository,
    request_repo: &SqliteRequestRepository,
    audit_repo: &SqliteAuditLogRepository,
    new: &RequestNew,
    description: &str,
    user_id: Option<i64>,
    now: i64,
    allow_insert: bool,
) -> Result<(i64, bool), AppError> {
    let tx = conn.transaction().map_err(map_rusqlite)?;
    if let Some(dev) = new.printer_device_id {
        if let Some(rid) = repo.open_repair_request_id(&tx, dev)? {
            repo.add_participant_in_tx(&tx, rid, new.requested_by_user_id, now)?;
            audit_repo.insert(
                &tx,
                AuditEntry {
                    entity_type: "request",
                    entity_id: rid,
                    action: "repair_join",
                    user_id,
                    before_json: None,
                    after_json: None,
                    payload_json: Some(
                        serde_json::json!({ "description": description }).to_string(),
                    ),
                    created_at_utc: now,
                },
            )?;
            tx.commit().map_err(map_rusqlite)?;
            return Ok((rid, true));
        }
        if !allow_insert {
            return Err(AppError::Conflict {
                reason: "Не удалось создать заявку: повторите попытку.".into(),
            });
        }
    }
    let id = request_repo.insert_in_tx(&tx, new, now)?;
    audit_repo.insert(
        &tx,
        AuditEntry {
            entity_type: "request",
            entity_id: id,
            action: "create",
            user_id,
            before_json: None,
            after_json: None,
            payload_json: None,
            created_at_utc: now,
        },
    )?;
    tx.commit().map_err(map_rusqlite)?;
    Ok((id, false))
}

fn ensure_repair(request_type: &str) -> Result<(), AppError> {
    if request_type != REPAIR_REQUEST_TYPE {
        return Err(AppError::Validation {
            field: "request_id".into(),
            message: "Заявка не является ремонтной.".into(),
        });
    }
    Ok(())
}

/// Что затронула запись устройства: для рассылки и для D-16.
struct DeviceOutcome {
    places: Vec<i64>,
    devices: Vec<i64>,
    groups: Vec<i64>,
    /// Место устройства действительно поменялось.
    changed: bool,
    /// Место устройства до операции.
    previous_place: Option<i64>,
}

fn push_unique(v: &mut Vec<i64>, id: i64) {
    if !v.contains(&id) {
        v.push(id);
    }
}

/// D-16: при первом реальном перемещении сохранить место ДО в заявке; дальнейшие
/// перемещения `repair_origin_place_id` не перезаписывают.
fn remember_first_move(
    tx: &Transaction<'_>,
    repo: &SqliteRepairRepository,
    request_id: i64,
    out: &DeviceOutcome,
    now: i64,
) -> Result<(), AppError> {
    if !out.changed {
        return Ok(());
    }
    let (_, moved) = repo.get_repair_fields(tx, request_id)?;
    if !moved {
        repo.set_repair_fields_in_tx(tx, request_id, out.previous_place, true, now)?;
    }
    Ok(())
}

/// Места живых картриджей, прикреплённых к устройствам (для рассылки при каскаде).
fn attached_cartridge_places(
    tx: &Transaction<'_>,
    device_ids: &[i64],
) -> Result<Vec<i64>, AppError> {
    let mut out = Vec::new();
    for dev in device_ids {
        let mut stmt = tx
            .prepare(
                "SELECT DISTINCT place_id FROM cartridges \
                 WHERE current_printer_device_id = ?1 AND deleted_at_utc IS NULL \
                 AND place_id IS NOT NULL",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![dev], |r| r.get::<_, i64>(0))
            .map_err(map_rusqlite)?;
        for r in rows {
            push_unique(&mut out, r.map_err(map_rusqlite)?);
        }
    }
    Ok(out)
}

/// Единственная точка записи `status_id` и `place_id` устройства сервиса.
///
/// `status_code`: `Some` ставит статус по коду из справочника (D-02), `None` оставляет.
/// `target_place`: `Some` перемещает, `None` оставляет на месте. Якорное устройство
/// перемещает весь блок через `move_group_in_tx` (D-18), статус меняется только у
/// него; якорь вложенной группы переместить нельзя. Журнал Manual с note «по заявке №N»,
/// аудит `custom:repair_move`, каскад места картриджей принтера (только при `Some` и
/// смене места, CR-03) выполняются в той же транзакции.
#[allow(clippy::too_many_arguments)]
fn apply_device_state_in_tx(
    tx: &Transaction<'_>,
    deps: &GroupMoveDeps,
    repo: &SqliteRepairRepository,
    device_id: i64,
    status_code: Option<&str>,
    target_place: Option<i64>,
    request_id: i64,
    user_id: Option<i64>,
    now: i64,
) -> Result<DeviceOutcome, AppError> {
    let before = deps.devices.get_in_tx(tx, device_id)?;
    if before.deleted_at_utc.is_some() {
        return Err(AppError::NotFound {
            entity: "device",
            id: device_id,
        });
    }
    let status_id = match status_code {
        Some(code) => repo.device_status_id_by_code(tx, code)?,
        None => before.status_id,
    };
    let mut out = DeviceOutcome {
        places: Vec::new(),
        devices: vec![device_id],
        groups: Vec::new(),
        changed: false,
        previous_place: before.place_id,
    };
    let mut final_place = before.place_id;

    if let Some(target) = target_place {
        if !place_is_live(tx, target)? {
            return Err(AppError::Validation {
                field: "place_id".into(),
                message: "Место не найдено или в архиве: выберите другое.".into(),
            });
        }
        let anchor: Option<(i64, Option<i64>)> = tx
            .query_row(
                "SELECT id, parent_group_id FROM groups \
                 WHERE anchor_device_id = ?1 AND deleted_at_utc IS NULL",
                params![device_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(map_rusqlite)?;
        if let Some((gid, parent)) = anchor {
            if parent.is_some() {
                return Err(AppError::Validation {
                    field: "place_id".into(),
                    message: "Место вложенной группы задаётся корневой группой; \
                              переместите блок через неё."
                        .into(),
                });
            }
            let members = deps.groups.subtree_device_ids_in_tx(tx, gid)?;
            let carts = attached_cartridge_places(tx, &members)?;
            let outcome = move_group_in_tx(tx, deps, gid, None, target, user_id, now)?;
            if outcome.batch_id.is_some() {
                for p in outcome.changed_place_ids {
                    push_unique(&mut out.places, p);
                }
                for p in carts {
                    push_unique(&mut out.places, p);
                }
            }
            for m in members {
                push_unique(&mut out.devices, m);
            }
            out.groups = deps.groups.subtree_group_ids_in_tx(tx, gid)?;
        }
        final_place = Some(target);
    }

    // Место после переноса блока (якорь мог поехать с группой) и до записи ниже.
    let mid_place = deps.devices.get_in_tx(tx, device_id)?.place_id;
    deps.devices
        .update_status_and_place_in_tx(tx, device_id, status_id, final_place, now)?;

    if mid_place != final_place {
        deps.movements.record_movement_if_applicable(
            tx,
            &deps.places,
            MovementEntityKind::Device,
            device_id,
            mid_place,
            final_place,
            MovementSource::Manual,
            Some(&format!("по заявке №{request_id}")),
            None,
            user_id,
            now,
        )?;
        if final_place.is_some() {
            for p in attached_cartridge_places(tx, &[device_id])? {
                push_unique(&mut out.places, p);
            }
            deps.cartridges.cascade_place_for_printer_in_tx(
                tx,
                device_id,
                final_place,
                MovementSource::Manual,
                "вместе с принтером",
                user_id,
                now,
            )?;
        }
    }

    if before.place_id != final_place {
        out.changed = true;
        if let Some(p) = before.place_id {
            push_unique(&mut out.places, p);
        }
        if let Some(p) = final_place {
            push_unique(&mut out.places, p);
        }
        deps.audit.insert(
            tx,
            AuditEntry {
                entity_type: "device",
                entity_id: device_id,
                action: "custom:repair_move",
                user_id,
                before_json: None,
                after_json: None,
                payload_json: Some(
                    serde_json::json!({
                        "request_id": request_id,
                        "from": before.place_id,
                        "to": final_place,
                    })
                    .to_string(),
                ),
                created_at_utc: now,
            },
        )?;
    }
    Ok(out)
}

/// Обязательный текст: обрезка, непустота, предел длины.
fn required_text(raw: &str, field: &str, empty_msg: &str) -> Result<String, AppError> {
    let t = raw.trim();
    if t.is_empty() {
        return Err(AppError::Validation {
            field: field.into(),
            message: empty_msg.into(),
        });
    }
    let limit = if field == "fault_description" {
        MAX_DESCRIPTION_CHARS
    } else {
        MAX_NAME_CHARS
    };
    if t.chars().count() > limit {
        return Err(AppError::Validation {
            field: field.into(),
            message: format!("Текст длиннее {limit} символов."),
        });
    }
    Ok(t.to_string())
}

/// Необязательный текст (D-29): пусто даёт `None`.
fn optional_text(raw: Option<&str>, field: &str) -> Result<Option<String>, AppError> {
    let t = raw.map(str::trim).unwrap_or("");
    if t.is_empty() {
        return Ok(None);
    }
    if t.chars().count() > MAX_DESCRIPTION_CHARS {
        return Err(AppError::Validation {
            field: field.into(),
            message: format!("Текст длиннее {MAX_DESCRIPTION_CHARS} символов."),
        });
    }
    Ok(Some(t.to_string()))
}

/// Дата захода: положительная и не позже «сейчас плюс сутки».
fn check_date(date_utc: i64, now: i64) -> Result<(), AppError> {
    if date_utc <= 0 || date_utc > now + 86_400 {
        return Err(AppError::Validation {
            field: "date_utc".into(),
            message: "Укажите корректную дату, не позже сегодняшней.".into(),
        });
    }
    Ok(())
}

/// D-09 на текущем месте устройства (без перемещения).
fn status_code_for_current_place(
    conn: &Connection,
    device_id: i64,
) -> Result<&'static str, AppError> {
    let place: Option<i64> = conn
        .query_row(
            "SELECT place_id FROM devices WHERE id = ?1",
            params![device_id],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)?;
    match place {
        Some(p) => status_code_for_place(conn, p),
        None => Ok(DEVICE_STATUS_CODE_IN_USE),
    }
}

/// Снять «Ожидает ремонта» при отклонении или отмене автором (D-09, D-11): только до
/// первого перемещения и вне сервиса; иначе Validation. Без устройства ничего не делает.
fn release_device_in_tx(
    tx: &Transaction<'_>,
    repo: &SqliteRepairRepository,
    row: &trackly_core::domain::requests::RequestRow,
    user_id: Option<i64>,
    now: i64,
) -> Result<(Vec<i64>, Vec<i64>, Vec<i64>), AppError> {
    if row.status == "open" {
        return Ok((Vec::new(), Vec::new(), Vec::new()));
    }
    let (_, moved) = repo.get_repair_fields(tx, row.id)?;
    if moved {
        return Err(AppError::Validation {
            field: "status".into(),
            message: "Устройство уже перемещалось: завершите заявку.".into(),
        });
    }
    let Some(dev) = row.printer_device_id else {
        return Ok((Vec::new(), Vec::new(), Vec::new()));
    };
    let code = status_code_for_current_place(tx, dev)?;
    let deps = GroupMoveDeps::new();
    let out =
        apply_device_state_in_tx(tx, &deps, repo, dev, Some(code), None, row.id, user_id, now)?;
    Ok((out.places, out.devices, out.groups))
}

/// D-09: статус устройства при возврате по месту считается на сервере. Складское
/// место (оно само или любой предок с `is_storage`) даёт «На складе», остальное
/// «В работе».
fn status_code_for_place(conn: &Connection, place_id: i64) -> Result<&'static str, AppError> {
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
    Ok(if n > 0 {
        DEVICE_STATUS_CODE_IN_STORAGE
    } else {
        DEVICE_STATUS_CODE_IN_USE
    })
}
