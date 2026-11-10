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

use rusqlite::{params, Connection, OptionalExtension};
use trackly_core::auth::{authorize, Action, Identity, Role};
use trackly_core::domain::repairs::{format_kopecks, REPAIR_REQUEST_TYPE};
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
    OpenRepairRefDto, RepairCreateDto, RepairCreateResultDto, RepairDeviceOptionDto,
    RepairLastPlacesDto, RepairStateDto, RepairVisitDto,
};
use crate::dto::request::RequestDto;

/// Ключи `app_settings` для последних выбранных мест (D-25).
const SETTING_LAST_MOVE_PLACE: &str = "repair_last_move_place_id";
const SETTING_LAST_SERVICE_PLACE: &str = "repair_last_service_place_id";

/// Предел длины описания (символов).
const MAX_DESCRIPTION_CHARS: usize = 4000;

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

    /// Одно `WsEvent::EntitiesChanged` на мутацию, после коммита и вне замыкания writer.
    /// Используется переходами цикла (планы 10-13); создание заявки места/устройства/групп
    /// не меняет.
    #[allow(dead_code)]
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
