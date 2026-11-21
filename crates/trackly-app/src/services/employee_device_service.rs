//! `EmployeeDeviceService` — раздел «Устройства» сотрудника (Phase 41.6, EMP-02/EMP-03).
//!
//! Только чтение: без писателя, вне области реестрового гейта записей.
//! BOLA: область строится ТОЛЬКО по `SqliteRepairRepository::employee_device_ids`
//! для любой роли (без staff-обхода); второй реализации области нет, эндпоинта
//! «по id» нет. BOPLA: DTO минимальный (см. `dto::employee_device`).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use rusqlite::{params, params_from_iter, Connection};
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::domain::repairs::DEVICE_STATUS_CODE_WRITTEN_OFF;
use trackly_core::error::AppError;
use trackly_infra::db::pools::ReaderPool;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::repairs_sqlite::{
    request_visibility_clause, DeviceOriginSource, SqliteRepairRepository,
};

use crate::dto::employee_device::{
    EmployeeDeviceDto, EmployeeDevicesDto, EmployeeRequestHistoryDto, EmployeeRequestRefDto,
    EmployeeTonerDto, EmployeeTonerLevelDto,
};

/// Верхняя граница закрытых заявок в карточке устройства (T-41.6-24).
const CLOSED_REQUESTS_LIMIT: usize = 10;

fn join_err(e: tokio::task::JoinError) -> AppError {
    AppError::Internal {
        source_chain: format!("spawn_blocking: {e}"),
    }
}

/// Открытая заявка: статусы совпадают с `OPEN_REPAIR_CLAUSE` и «замена картриджа».
fn is_open_status(status: &str) -> bool {
    matches!(status, "open" | "in_progress" | "in_repair")
}

pub struct EmployeeDeviceService {
    readers: Arc<ReaderPool>,
    repo: Arc<SqliteRepairRepository>,
}

impl EmployeeDeviceService {
    pub fn new(readers: Arc<ReaderPool>, repo: Arc<SqliteRepairRepository>) -> Self {
        Self { readers, repo }
    }

    /// Список устройств области вызывающего сотрудника (D-14).
    pub async fn list_for_employee(
        &self,
        caller: &Identity,
    ) -> Result<EmployeeDevicesDto, AppError> {
        authorize(caller, &Action::CreateRequest)?;
        // Скоуп по user_id для ЛЮБОЙ роли; без user_id — отказ.
        let user_id = caller.user_id.ok_or(AppError::Forbidden)?;
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            build_list(&conn, &repo, user_id)
        })
        .await
        .map_err(join_err)?
    }
}

fn build_list(
    conn: &Connection,
    repo: &SqliteRepairRepository,
    user_id: i64,
) -> Result<EmployeeDevicesDto, AppError> {
    let has_groups = repo.employee_has_groups(conn, user_id)?;
    let scope: HashSet<i64> = repo
        .employee_device_ids(conn, user_id)?
        .into_iter()
        .collect();

    // (c) основной запрос. NULL-статус не отбрасывается; «Списано» исключено (D-20).
    // NEW-5/INT-31 (quick-261121-heh): `is_printer` — тот же признак, что варианты
    // принтеров заявки (`request_printer_options`): `devices.type_id` =
    // PRINTER_TYPE_ID, а не наличие строки `printers` (она производна от type_id).
    let mut stmt = conn
        .prepare(concat!(
            "SELECT d.id, d.name, dt.name, d.model, d.serial_number, d.inventory_number, \
                    pfp.full_path, ds.code, ds.name, \
                    (d.type_id = ",
            trackly_core::printer_type_id_sql!(),
            ") \
             FROM devices d \
             JOIN device_types dt ON dt.id = d.type_id \
             LEFT JOIN place_full_paths pfp ON pfp.place_id = d.place_id \
             LEFT JOIN device_statuses ds ON ds.id = d.status_id \
             WHERE d.deleted_at_utc IS NULL AND (ds.code IS NULL OR ds.code <> ?1) \
             ORDER BY dt.name, d.name, d.id"
        ))
        .map_err(map_rusqlite)?;
    let mut items: Vec<EmployeeDeviceDto> = stmt
        .query_map(params![DEVICE_STATUS_CODE_WRITTEN_OFF], |r| {
            Ok(EmployeeDeviceDto {
                id: r.get(0)?,
                name: r.get(1)?,
                type_name: r.get(2)?,
                model: r.get(3)?,
                serial_number: r.get(4)?,
                inventory_number: r.get(5)?,
                place_full: r.get(6)?,
                status_code: r.get(7)?,
                status_name: r.get::<_, Option<String>>(8)?.unwrap_or_default(),
                is_printer: r.get(9)?,
                arm_names: Vec::new(),
                printer_origin: None,
                toner: None,
                in_repair: false,
                my_repair_request_id: None,
                my_open_requests: Vec::new(),
                other_open_request: false,
                my_requests: Vec::new(),
            })
        })
        .map_err(map_rusqlite)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_rusqlite)?;
    items.retain(|d| scope.contains(&d.id));
    let ids: Vec<i64> = items.iter().map(|d| d.id).collect();

    // (d) АРМ и происхождение принтера (D-04, D-05).
    let mut arms: HashMap<i64, Vec<String>> = HashMap::new();
    let mut best: HashMap<i64, DeviceOriginSource> = HashMap::new();
    for o in repo.employee_device_origins(conn, user_id)? {
        let names = arms.entry(o.device_id).or_default();
        if !names.contains(&o.group_name) {
            names.push(o.group_name.clone());
        }
        best.entry(o.device_id)
            .and_modify(|s| {
                if o.source.rank() < s.rank() {
                    *s = o.source;
                }
            })
            .or_insert(o.source);
    }

    // (e) тонер: последнее чтение по printers.device_id.
    let toner = load_toner(conn)?;

    // (f)/(g) заявки.
    let reqs = load_requests(conn, user_id, &ids)?;
    let busy: HashSet<i64> = repo
        .open_repair_requests_for_devices(conn, &ids)?
        .into_iter()
        .map(|(d, _)| d)
        .collect();

    for d in &mut items {
        d.arm_names = arms.remove(&d.id).unwrap_or_default();
        if d.is_printer {
            d.printer_origin = best.get(&d.id).map(|s| {
                match s {
                    DeviceOriginSource::Usb => "usb",
                    DeviceOriginSource::Link => "link",
                    DeviceOriginSource::Member => "member",
                }
                .to_string()
            });
            d.toner = toner.get(&d.id).cloned();
        }
        d.in_repair = busy.contains(&d.id);
        if let Some(rows) = reqs.get(&d.id) {
            for q in rows {
                let open = is_open_status(&q.status);
                if open && !q.mine {
                    d.other_open_request = true;
                }
                if !q.mine {
                    continue;
                }
                if open {
                    d.my_open_requests.push(EmployeeRequestRefDto {
                        id: q.id,
                        request_type: q.request_type.clone(),
                        status: q.status.clone(),
                    });
                    if q.request_type == "repair" && d.my_repair_request_id.is_none() {
                        d.my_repair_request_id = Some(q.id);
                    }
                }
            }
            // История: открытые сверху, затем закрытые по дате убыв. (лимит серверный).
            let mut hist: Vec<&ReqRow> = rows.iter().filter(|q| q.mine).collect();
            hist.sort_by(|a, b| {
                is_open_status(&b.status)
                    .cmp(&is_open_status(&a.status))
                    .then(b.created_at_utc.cmp(&a.created_at_utc))
                    .then(b.id.cmp(&a.id))
            });
            let mut closed_taken = 0usize;
            for q in hist {
                if !is_open_status(&q.status) {
                    if closed_taken >= CLOSED_REQUESTS_LIMIT {
                        continue;
                    }
                    closed_taken += 1;
                }
                d.my_requests.push(EmployeeRequestHistoryDto {
                    id: q.id,
                    request_type: q.request_type.clone(),
                    status: q.status.clone(),
                    created_at_utc: q.created_at_utc,
                });
            }
        }
    }

    Ok(EmployeeDevicesDto { has_groups, items })
}

struct ReqRow {
    id: i64,
    request_type: String,
    status: String,
    created_at_utc: i64,
    mine: bool,
}

/// Заявки «Ремонт» и «Замена картриджа» по устройствам; `mine` — сотрудник автор/участник.
fn load_requests(
    conn: &Connection,
    user_id: i64,
    ids: &[i64],
) -> Result<HashMap<i64, Vec<ReqRow>>, AppError> {
    let mut out: HashMap<i64, Vec<ReqRow>> = HashMap::new();
    if ids.is_empty() {
        return Ok(out);
    }
    let ph = (2..ids.len() + 2)
        .map(|i| format!("?{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT r.printer_device_id, r.id, r.request_type, r.status, r.created_at_utc, \
                {vis} \
         FROM requests r \
         WHERE r.deleted_at_utc IS NULL \
           AND r.request_type IN ('repair', 'cartridge_replace') \
           AND r.printer_device_id IN ({ph}) \
         ORDER BY r.id",
        vis = request_visibility_clause("?1"),
    );
    let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
    let mut args: Vec<i64> = Vec::with_capacity(ids.len() + 1);
    args.push(user_id);
    args.extend_from_slice(ids);
    let rows = stmt
        .query_map(params_from_iter(args.iter()), |r| {
            Ok((
                r.get::<_, i64>(0)?,
                ReqRow {
                    id: r.get(1)?,
                    request_type: r.get(2)?,
                    status: r.get(3)?,
                    created_at_utc: r.get(4)?,
                    mine: r.get(5)?,
                },
            ))
        })
        .map_err(map_rusqlite)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_rusqlite)?;
    for (dev, row) in rows {
        out.entry(dev).or_default().push(row);
    }
    Ok(out)
}

/// Последнее чтение тонера по каждому принтеру: `device_id -> EmployeeTonerDto`.
fn load_toner(conn: &Connection) -> Result<HashMap<i64, EmployeeTonerDto>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT p.device_id, pr.ts_utc, COALESCE(pr.toner_levels, '{}') \
             FROM printers p \
             JOIN printer_readings pr ON pr.id = ( \
                 SELECT id FROM printer_readings WHERE printer_id = p.id \
                 ORDER BY ts_utc DESC, id DESC LIMIT 1)",
        )
        .map_err(map_rusqlite)?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(map_rusqlite)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_rusqlite)?;
    let mut out = HashMap::new();
    for (device_id, ts, json) in rows {
        out.insert(
            device_id,
            EmployeeTonerDto {
                levels: parse_levels(&json),
                polled_at_utc: Some(ts),
            },
        );
    }
    Ok(out)
}

/// Разбор `{label: level}` (допускается и `{label: {pct: N}}`); `< 0` или не число -> `None`.
fn parse_levels(json: &str) -> Vec<EmployeeTonerLevelDto> {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    map.into_iter()
        .map(|(label, v)| {
            let n = match &v {
                serde_json::Value::Object(o) => o.get("pct").and_then(|x| x.as_f64()),
                other => other.as_f64(),
            };
            let percent = n
                .filter(|x| *x >= 0.0)
                .map(|x| x.round().clamp(0.0, 100.0) as i32);
            EmployeeTonerLevelDto { label, percent }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_levels_unknown_and_percent() {
        let v = parse_levels(r#"{"black":45,"cyan":-2,"drum":{"pct":7}}"#);
        let get = |l: &str| v.iter().find(|x| x.label == l).unwrap().percent;
        assert_eq!(get("black"), Some(45));
        assert_eq!(get("cyan"), None);
        assert_eq!(get("drum"), Some(7));
        assert!(parse_levels("not json").is_empty());
    }
}
