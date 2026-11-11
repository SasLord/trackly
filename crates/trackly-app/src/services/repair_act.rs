//! `RepairActRenderer` — HTML «Акта передачи в ремонт» (Phase 41.4, D-27/D-28).
//!
//! Только чтение: номер акта берётся из сохранённого `repair_visits.act_number`
//! (один на заход, повторная печать выдаёт тот же), таблица `acts` не затрагивается.
//! Рендер идёт тем же конвейером, что и акты приёма-передачи: файловый шаблон
//! `templates/repair_transfer_act.html` (встроенный дефолт как запасной),
//! `build_safe_html_env` с автоэкранированием, include `_header.html`.

use std::sync::Arc;

use rusqlite::{params, Connection, OptionalExtension};
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;
use trackly_infra::db::pools::ReaderPool;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::repairs_sqlite::SqliteRepairRepository;

use crate::services::act_service::format_ru_date;
use crate::services::org_db_service::OrgDbService;
use crate::services::organization_service::OrganizationService;
use crate::services::repair_service::RepairService;
use crate::services::report_service::translate_request_status;

const TEMPLATE_FILE: &str = "repair_transfer_act.html";
const DEVICE_REPORT_TEMPLATE_FILE: &str = "repair_device_report.html";

#[derive(Clone)]
pub struct RepairActRenderer {
    readers: Arc<ReaderPool>,
    organization: Arc<OrganizationService>,
    org_db: Arc<OrgDbService>,
}

/// Всё, что читается из БД для акта (без организации).
struct VisitActData {
    number: String,
    sent_at_utc: i64,
    giver_name: String,
    receiver_name: String,
    fault: String,
    service_place: String,
    device_name: String,
    inventory_no: Option<String>,
    serial_no: Option<String>,
    device_type: Option<String>,
    place_from: Option<String>,
}

fn place_label(conn: &Connection, place_id: i64) -> Result<Option<String>, AppError> {
    let path: Option<String> = conn
        .query_row(
            "SELECT full_path FROM place_full_paths WHERE place_id = ?1",
            params![place_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(map_rusqlite)?
        .flatten();
    if path.is_some() {
        return Ok(path);
    }
    conn.query_row(
        "SELECT name FROM places WHERE id = ?1",
        params![place_id],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .map_err(map_rusqlite)
}

impl RepairActRenderer {
    pub fn new(
        readers: Arc<ReaderPool>,
        organization: Arc<OrganizationService>,
        org_db: Arc<OrgDbService>,
    ) -> Self {
        Self {
            readers,
            organization,
            org_db,
        }
    }

    /// HTML акта для браузерной печати. Доступно Администратору и Специалисту.
    pub async fn render_html(&self, visit_id: i64, caller: &Identity) -> Result<String, AppError> {
        authorize(caller, &Action::TransitionRequests)?;

        let readers = self.readers.clone();
        let data = tokio::task::spawn_blocking(move || -> Result<VisitActData, AppError> {
            let conn = readers.acquire();
            let visit =
                SqliteRepairRepository
                    .get_visit(&conn, visit_id)?
                    .ok_or(AppError::NotFound {
                        entity: "repair_visit",
                        id: visit_id,
                    })?;
            let (name, inv, serial, dtype) = conn
                .query_row(
                    "SELECT d.name, d.inventory_number, d.serial_number, \
                            (SELECT t.name FROM device_types t WHERE t.id = d.type_id) \
                     FROM devices d WHERE d.id = ?1",
                    params![visit.device_id],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, Option<String>>(1)?,
                            r.get::<_, Option<String>>(2)?,
                            r.get::<_, Option<String>>(3)?,
                        ))
                    },
                )
                .optional()
                .map_err(map_rusqlite)?
                .ok_or(AppError::NotFound {
                    entity: "device",
                    id: visit.device_id,
                })?;
            let service_place = place_label(&conn, visit.service_place_id)?.unwrap_or_default();
            // UAT 6 / D-27: для первого захода «до отправки» = исходное место
            // устройства (до перемещения на склад по заявке); иначе — место на
            // момент отправки (from_place_id захода).
            let (origin, moved) =
                SqliteRepairRepository.get_repair_fields(&conn, visit.request_id)?;
            let place_from_id = match origin {
                Some(o) if visit.seq == 1 && moved => Some(o),
                _ => visit.from_place_id,
            };
            let place_from = match place_from_id {
                Some(id) => place_label(&conn, id)?,
                None => None,
            };
            Ok(VisitActData {
                number: visit.act_number,
                sent_at_utc: visit.sent_at_utc,
                giver_name: visit.giver_name,
                receiver_name: visit.receiver_name,
                fault: visit.fault_description,
                service_place,
                device_name: name,
                inventory_no: inv,
                serial_no: serial,
                device_type: dtype,
                place_from,
            })
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking load repair act: {e}"),
        })??;

        let (org_dto, logo_bytes, logo_mime) = self.org_db.get_for_pdf().await?;
        let logo_data_uri =
            crate::pdf::minijinja_env::logo_data_uri(logo_bytes, logo_mime.as_deref());

        let templates_dir =
            crate::pdf::html_templates::resolve_templates_dir(&self.organization.paths);
        let embedded = |file: &str| {
            crate::pdf::html_templates::DEFAULT_HTML_TEMPLATES
                .iter()
                .find(|(f, _)| *f == file)
                .map(|(_, body)| *body)
                .unwrap_or("")
        };
        let template_src = crate::pdf::html_templates::load_template(
            &templates_dir,
            TEMPLATE_FILE,
            embedded(TEMPLATE_FILE),
        );
        let header_src = crate::pdf::html_templates::load_template(
            &templates_dir,
            "_header.html",
            embedded("_header.html"),
        );

        let ctx = serde_json::json!({
            "org": {
                "name": org_dto.org_name,
                "inn": org_dto.inn,
                "kpp": org_dto.kpp,
                "address": org_dto.address,
                "address_line2": org_dto.address_line2,
                "phone": org_dto.phone,
                "fax": org_dto.fax,
                "email": org_dto.email,
                "okpo": org_dto.okpo,
                "ogrn": org_dto.ogrn,
                "full_name": crate::pdf::minijinja_env::org_full_name_html(&org_dto.full_name),
                "logo_data_uri": logo_data_uri,
            },
            "repair": {
                "number": data.number,
                "date_human": format_ru_date(data.sent_at_utc),
                "giver_name": data.giver_name,
                "receiver_name": data.receiver_name,
                "fault": data.fault,
                "service_place": data.service_place,
            },
            "device": {
                "name": data.device_name,
                "inventory_no": data.inventory_no,
                "serial_no": data.serial_no,
                "type": data.device_type,
                "place_from": data.place_from,
            },
        });

        crate::pdf::minijinja_env::render_with_timeout(
            &crate::pdf::minijinja_env::build_safe_html_env(),
            "repair_transfer_act_html",
            &template_src,
            ctx,
            &[("_header.html", &header_src)],
        )
        .await
    }

    /// HTML печатного «Отчёта о ремонтах устройства». Только чтение: номера не
    /// выдаются, журнал «Акты» не затрагивается. Данные — тот же
    /// `RepairService::device_history`, что и в попапе «Ремонты», чтобы экран
    /// и печать не расходились. Доступно Администратору и Специалисту.
    pub async fn render_device_report(
        &self,
        repairs: &RepairService,
        device_id: i64,
        caller: &Identity,
    ) -> Result<String, AppError> {
        authorize(caller, &Action::TransitionRequests)?;
        let history = repairs.device_history(device_id, caller).await?;

        let (org_dto, logo_bytes, logo_mime) = self.org_db.get_for_pdf().await?;
        let logo_data_uri =
            crate::pdf::minijinja_env::logo_data_uri(logo_bytes, logo_mime.as_deref());

        let templates_dir =
            crate::pdf::html_templates::resolve_templates_dir(&self.organization.paths);
        let embedded = |file: &str| {
            crate::pdf::html_templates::DEFAULT_HTML_TEMPLATES
                .iter()
                .find(|(f, _)| *f == file)
                .map(|(_, body)| *body)
                .unwrap_or("")
        };
        let template_src = crate::pdf::html_templates::load_template(
            &templates_dir,
            DEVICE_REPORT_TEMPLATE_FILE,
            embedded(DEVICE_REPORT_TEMPLATE_FILE),
        );
        let header_src = crate::pdf::html_templates::load_template(
            &templates_dir,
            "_header.html",
            embedded("_header.html"),
        );

        let requests: Vec<serde_json::Value> = history
            .requests
            .iter()
            .map(|r| {
                let visits: Vec<serde_json::Value> = r
                    .visits
                    .iter()
                    .map(|v| {
                        serde_json::json!({
                            "seq": v.seq,
                            "act_number": v.act_number,
                            "sent_date_human": format_ru_date(v.sent_at_utc),
                            "returned_date_human": v.returned_at_utc.map(format_ru_date),
                            "service_place": v.service_place_path,
                            "fault": v.fault_description,
                            "works_done": v.works_done,
                            "cost_display": v.cost_display,
                            "has_scan": v.scan_count > 0,
                        })
                    })
                    .collect();
                serde_json::json!({
                    "id": r.request_id,
                    "status_label": translate_request_status(&r.status),
                    "created_date_human": format_ru_date(r.created_at_utc),
                    "requester_name": r.requester_name,
                    "description": r.description,
                    "visits": visits,
                })
            })
            .collect();

        let printed = time::OffsetDateTime::now_utc().unix_timestamp();
        let ctx = serde_json::json!({
            "org": {
                "name": org_dto.org_name,
                "inn": org_dto.inn,
                "kpp": org_dto.kpp,
                "address": org_dto.address,
                "address_line2": org_dto.address_line2,
                "phone": org_dto.phone,
                "fax": org_dto.fax,
                "email": org_dto.email,
                "okpo": org_dto.okpo,
                "ogrn": org_dto.ogrn,
                "full_name": crate::pdf::minijinja_env::org_full_name_html(&org_dto.full_name),
                "logo_data_uri": logo_data_uri,
            },
            "device": {
                "name": history.device_name,
                "inventory_no": history.inventory_no,
                "serial_no": history.serial_no,
                "type_name": history.type_name,
            },
            "summary": {
                "visit_count": history.visit_count,
                "total_cost": history.total_cost_display,
                "printed_date_human": format_ru_date(printed),
            },
            "requests": requests,
        });

        crate::pdf::minijinja_env::render_with_timeout(
            &crate::pdf::minijinja_env::build_safe_html_env(),
            "repair_device_report_html",
            &template_src,
            ctx,
            &[("_header.html", &header_src)],
        )
        .await
    }
}
