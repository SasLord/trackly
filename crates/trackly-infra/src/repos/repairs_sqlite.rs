//! SQLite-слой ремонтных заявок (Phase 41.4): единый источник SQL для предиката
//! «открытая ремонтная заявка устройства» (D-19), заходов в сервис (D-13), участников
//! (D-20), сканов актов, настроек и области устройств сотрудника.
//!
//! Все значения идут плейсхолдерами; `format!` допустим только для списков `?` в `IN`.

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use trackly_core::domain::repairs::{RepairVisitNew, RepairVisitRow};
use trackly_core::error::AppError;

use crate::error_conversions::map_rusqlite;

/// Заявка в истории ремонтов устройства (строка запроса по заходам).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairHistoryRequestRow {
    pub id: i64,
    pub status: String,
    pub description: Option<String>,
    pub created_at_utc: i64,
    pub requester_name: Option<String>,
}

/// SQLite-репозиторий ремонтов (zero-sized).
#[derive(Debug, Default, Clone)]
pub struct SqliteRepairRepository;

/// D-19: условие «ремонтная заявка открыта». Единственное определение; значения
/// статусов совпадают с `OPEN_REPAIR_STATUSES` и частичным индексом V051.
pub const OPEN_REPAIR_CLAUSE: &str = "r.request_type = 'repair' \
     AND r.status IN ('open', 'in_progress', 'in_repair') \
     AND r.deleted_at_utc IS NULL";

/// Видимость заявки не-админу: автор или участник (D-20). Подставляется как
/// `AND (…)` в запросы над `requests r`; параметр `?N` — id пользователя (подставляет вызывающий).
pub fn request_visibility_clause(user_param: &str) -> String {
    format!(
        "(r.requested_by_user_id = {p} OR EXISTS (SELECT 1 FROM request_participants rp \
         WHERE rp.request_id = r.id AND rp.user_id = {p}))",
        p = user_param
    )
}

const VISIT_COLUMNS: &str = "id, request_id, device_id, seq, service_place_id, from_place_id, \
     sent_at_utc, returned_at_utc, giver_name, receiver_name, fault_description, \
     returned_to_place_id, received_by_name, works_done, cost_kopecks, act_number, \
     created_by_user_id, created_at_utc, updated_at_utc, version";

fn visit_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<RepairVisitRow> {
    Ok(RepairVisitRow {
        id: r.get(0)?,
        request_id: r.get(1)?,
        device_id: r.get(2)?,
        seq: r.get(3)?,
        service_place_id: r.get(4)?,
        from_place_id: r.get(5)?,
        sent_at_utc: r.get(6)?,
        returned_at_utc: r.get(7)?,
        giver_name: r.get(8)?,
        receiver_name: r.get(9)?,
        fault_description: r.get(10)?,
        returned_to_place_id: r.get(11)?,
        received_by_name: r.get(12)?,
        works_done: r.get(13)?,
        cost_kopecks: r.get(14)?,
        act_number: r.get(15)?,
        created_by_user_id: r.get(16)?,
        created_at_utc: r.get(17)?,
        updated_at_utc: r.get(18)?,
        version: r.get(19)?,
    })
}

fn placeholders(n: usize) -> String {
    vec!["?"; n].join(",")
}

impl SqliteRepairRepository {
    // ---------------------------------------------------------------- предикат

    /// Id открытой ремонтной заявки устройства (D-19, D-21, D-22 используют только его).
    pub fn open_repair_request_id(
        &self,
        conn: &Connection,
        device_id: i64,
    ) -> Result<Option<i64>, AppError> {
        let sql = format!(
            "SELECT r.id FROM requests r WHERE r.printer_device_id = ?1 AND {OPEN_REPAIR_CLAUSE} \
             ORDER BY r.id LIMIT 1"
        );
        conn.query_row(&sql, params![device_id], |r| r.get(0))
            .optional()
            .map_err(map_rusqlite)
    }

    /// Пары `(device_id, request_id)` открытых ремонтов среди `device_ids`.
    pub fn open_repair_requests_for_devices(
        &self,
        conn: &Connection,
        device_ids: &[i64],
    ) -> Result<Vec<(i64, i64)>, AppError> {
        if device_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT r.printer_device_id, r.id FROM requests r \
             WHERE {OPEN_REPAIR_CLAUSE} AND r.printer_device_id IN ({}) \
             ORDER BY r.printer_device_id, r.id",
            placeholders(device_ids.len())
        );
        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(device_ids.iter()), |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
            })
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    // ------------------------------------------------------- поля заявки

    /// `(repair_origin_place_id, repair_moved)` заявки.
    pub fn get_repair_fields(
        &self,
        conn: &Connection,
        request_id: i64,
    ) -> Result<(Option<i64>, bool), AppError> {
        let row: Option<(Option<i64>, i64)> = conn
            .query_row(
                "SELECT repair_origin_place_id, repair_moved FROM requests \
                 WHERE id = ?1 AND deleted_at_utc IS NULL",
                params![request_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(map_rusqlite)?;
        match row {
            Some((o, m)) => Ok((o, m != 0)),
            None => Err(AppError::NotFound {
                entity: "request",
                id: request_id,
            }),
        }
    }

    pub fn set_repair_fields_in_tx(
        &self,
        tx: &Transaction<'_>,
        request_id: i64,
        origin_place_id: Option<i64>,
        moved: bool,
        now_utc: i64,
    ) -> Result<(), AppError> {
        let n = tx
            .execute(
                "UPDATE requests SET repair_origin_place_id = ?1, repair_moved = ?2, \
                 updated_at_utc = ?3 WHERE id = ?4 AND deleted_at_utc IS NULL",
                params![origin_place_id, i64::from(moved), now_utc, request_id],
            )
            .map_err(map_rusqlite)?;
        if n == 0 {
            return Err(AppError::NotFound {
                entity: "request",
                id: request_id,
            });
        }
        Ok(())
    }

    fn lock_mismatch(tx: &Transaction<'_>, request_id: i64, version: i64) -> Result<(), AppError> {
        let actual: Option<i64> = tx
            .query_row(
                "SELECT version FROM requests WHERE id = ?1 AND deleted_at_utc IS NULL",
                params![request_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(map_rusqlite)?;
        Err(match actual {
            None => AppError::NotFound {
                entity: "request",
                id: request_id,
            },
            Some(actual) => AppError::OptimisticLockMismatch {
                entity: "request",
                id: request_id,
                expected: version,
                actual,
            },
        })
    }

    /// Смена статуса заявки с CAS по `version`.
    pub fn set_request_status_in_tx(
        &self,
        tx: &Transaction<'_>,
        request_id: i64,
        version: i64,
        new_status: &str,
        assigned_to: Option<i64>,
        now_utc: i64,
    ) -> Result<(), AppError> {
        let n = tx
            .execute(
                "UPDATE requests SET status = ?1, \
                 assigned_to_user_id = COALESCE(?2, assigned_to_user_id), \
                 updated_at_utc = ?3, version = version + 1 \
                 WHERE id = ?4 AND version = ?5 AND deleted_at_utc IS NULL",
                params![new_status, assigned_to, now_utc, request_id, version],
            )
            .map_err(map_rusqlite)?;
        if n == 0 {
            return Self::lock_mismatch(tx, request_id, version);
        }
        Ok(())
    }

    /// «Указать устройство» в заявке. Нарушение `uq_requests_open_repair_device` -> `Conflict`
    /// (карта `map_rusqlite` переводит ConstraintViolation).
    pub fn set_request_device_in_tx(
        &self,
        tx: &Transaction<'_>,
        request_id: i64,
        device_id: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
        let n = tx
            .execute(
                "UPDATE requests SET printer_device_id = ?1, updated_at_utc = ?2, \
                 version = version + 1 WHERE id = ?3 AND deleted_at_utc IS NULL",
                params![device_id, now_utc, request_id],
            )
            .map_err(map_rusqlite)?;
        if n == 0 {
            return Err(AppError::NotFound {
                entity: "request",
                id: request_id,
            });
        }
        Ok(())
    }

    // ------------------------------------------------------------ заходы

    /// Вставка захода; `seq = MAX(seq) + 1` по заявке. Второй открытый заход отклоняет
    /// `uq_repair_visits_one_open` (-> `Conflict`). Возвращает id.
    pub fn insert_visit_in_tx(
        &self,
        tx: &Transaction<'_>,
        new: &RepairVisitNew,
        now_utc: i64,
    ) -> Result<i64, AppError> {
        tx.execute(
            "INSERT INTO repair_visits (request_id, device_id, seq, service_place_id, \
             from_place_id, sent_at_utc, giver_name, receiver_name, fault_description, \
             act_number, created_by_user_id, created_at_utc, updated_at_utc, version) \
             VALUES (?1, ?2, \
                     (SELECT COALESCE(MAX(seq), 0) + 1 FROM repair_visits WHERE request_id = ?1), \
                     ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, 1)",
            params![
                new.request_id,
                new.device_id,
                new.service_place_id,
                new.from_place_id,
                new.sent_at_utc,
                new.giver_name,
                new.receiver_name,
                new.fault_description,
                new.act_number,
                new.created_by_user_id,
                now_utc,
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(tx.last_insert_rowid())
    }

    pub fn get_visit(
        &self,
        conn: &Connection,
        visit_id: i64,
    ) -> Result<Option<RepairVisitRow>, AppError> {
        let sql = format!("SELECT {VISIT_COLUMNS} FROM repair_visits WHERE id = ?1");
        conn.query_row(&sql, params![visit_id], visit_from_row)
            .optional()
            .map_err(map_rusqlite)
    }

    pub fn list_visits_for_request(
        &self,
        conn: &Connection,
        request_id: i64,
    ) -> Result<Vec<RepairVisitRow>, AppError> {
        let sql =
            format!("SELECT {VISIT_COLUMNS} FROM repair_visits WHERE request_id = ?1 ORDER BY seq");
        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![request_id], visit_from_row)
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// Ремонтные заявки, у которых есть заходы именно по этому устройству (по
    /// `repair_visits.device_id`, не по `requests.printer_device_id`): без дубликатов, без
    /// помеченных удалёнными, новые первыми.
    pub fn list_repair_requests_with_visits_for_device(
        &self,
        conn: &Connection,
        device_id: i64,
    ) -> Result<Vec<RepairHistoryRequestRow>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT r.id, r.status, r.description, r.created_at_utc, u.full_name \
                 FROM requests r \
                 LEFT JOIN users u ON u.id = r.requested_by_user_id \
                 WHERE r.request_type = 'repair' AND r.deleted_at_utc IS NULL \
                   AND r.id IN (SELECT request_id FROM repair_visits WHERE device_id = ?1) \
                 ORDER BY r.created_at_utc DESC, r.id DESC",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![device_id], |r| {
                Ok(RepairHistoryRequestRow {
                    id: r.get(0)?,
                    status: r.get(1)?,
                    description: r.get(2)?,
                    created_at_utc: r.get(3)?,
                    requester_name: r.get(4)?,
                })
            })
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    pub fn open_visit_for_request(
        &self,
        conn: &Connection,
        request_id: i64,
    ) -> Result<Option<RepairVisitRow>, AppError> {
        let sql = format!(
            "SELECT {VISIT_COLUMNS} FROM repair_visits \
             WHERE request_id = ?1 AND returned_at_utc IS NULL"
        );
        conn.query_row(&sql, params![request_id], visit_from_row)
            .optional()
            .map_err(map_rusqlite)
    }

    pub fn count_visits(&self, conn: &Connection, request_id: i64) -> Result<i64, AppError> {
        conn.query_row(
            "SELECT COUNT(*) FROM repair_visits WHERE request_id = ?1",
            params![request_id],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)
    }

    /// Закрыть открытый заход (CAS по `version`).
    #[allow(clippy::too_many_arguments)]
    pub fn close_visit_in_tx(
        &self,
        tx: &Transaction<'_>,
        visit_id: i64,
        version: i64,
        returned_at_utc: i64,
        returned_to_place_id: Option<i64>,
        received_by_name: Option<&str>,
        works_done: Option<&str>,
        cost_kopecks: Option<i64>,
        now_utc: i64,
    ) -> Result<(), AppError> {
        let n = tx
            .execute(
                "UPDATE repair_visits SET returned_at_utc = ?1, returned_to_place_id = ?2, \
                 received_by_name = ?3, works_done = ?4, cost_kopecks = ?5, \
                 updated_at_utc = ?6, version = version + 1 \
                 WHERE id = ?7 AND version = ?8 AND returned_at_utc IS NULL",
                params![
                    returned_at_utc,
                    returned_to_place_id,
                    received_by_name,
                    works_done,
                    cost_kopecks,
                    now_utc,
                    visit_id,
                    version
                ],
            )
            .map_err(map_rusqlite)?;
        if n == 0 {
            let actual: Option<i64> = tx
                .query_row(
                    "SELECT version FROM repair_visits WHERE id = ?1",
                    params![visit_id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(map_rusqlite)?;
            return Err(match actual {
                None => AppError::NotFound {
                    entity: "repair_visit",
                    id: visit_id,
                },
                Some(actual) if actual != version => AppError::OptimisticLockMismatch {
                    entity: "repair_visit",
                    id: visit_id,
                    expected: version,
                    actual,
                },
                Some(_) => AppError::Conflict {
                    reason: "Заход уже закрыт.".to_string(),
                },
            });
        }
        Ok(())
    }

    // ------------------------------------------------------- участники

    /// Идемпотентное добавление участника (D-20).
    pub fn add_participant_in_tx(
        &self,
        tx: &Transaction<'_>,
        request_id: i64,
        user_id: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
        tx.execute(
            "INSERT OR IGNORE INTO request_participants (request_id, user_id, joined_at_utc) \
             VALUES (?1, ?2, ?3)",
            params![request_id, user_id, now_utc],
        )
        .map(|_| ())
        .map_err(map_rusqlite)
    }

    pub fn participant_user_ids(
        &self,
        conn: &Connection,
        request_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT user_id FROM request_participants WHERE request_id = ?1 ORDER BY user_id",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![request_id], |r| r.get::<_, i64>(0))
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    // ------------------------------------------------------------ сканы

    pub fn link_scan_in_tx(
        &self,
        tx: &Transaction<'_>,
        visit_id: i64,
        attachment_id: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
        tx.execute(
            "INSERT OR IGNORE INTO repair_visit_scans (visit_id, attachment_id, linked_at_utc) \
             VALUES (?1, ?2, ?3)",
            params![visit_id, attachment_id, now_utc],
        )
        .map(|_| ())
        .map_err(map_rusqlite)
    }

    /// Число НЕ удалённых вложений-сканов захода.
    pub fn scan_count_for_visit(&self, conn: &Connection, visit_id: i64) -> Result<i64, AppError> {
        conn.query_row(
            "SELECT COUNT(*) FROM repair_visit_scans s \
             JOIN attachments a ON a.id = s.attachment_id AND a.deleted_at_utc IS NULL \
             WHERE s.visit_id = ?1",
            params![visit_id],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)
    }

    /// `(visit_id, число живых сканов)` по всем заходам заявки (захода без сканов — с нулём).
    pub fn scan_counts_for_request(
        &self,
        conn: &Connection,
        request_id: i64,
    ) -> Result<Vec<(i64, i64)>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT v.id, COUNT(a.id) FROM repair_visits v \
                 LEFT JOIN repair_visit_scans s ON s.visit_id = v.id \
                 LEFT JOIN attachments a ON a.id = s.attachment_id AND a.deleted_at_utc IS NULL \
                 WHERE v.request_id = ?1 GROUP BY v.id ORDER BY v.seq",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![request_id], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
            })
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    // ------------------------------------------------------- настройки

    /// Значение из `app_settings` (D-25; `backup_service` не затрагивается).
    pub fn read_setting(&self, conn: &Connection, key: &str) -> Result<Option<String>, AppError> {
        conn.query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .optional()
        .map_err(map_rusqlite)
    }

    pub fn upsert_setting(
        &self,
        conn: &Connection,
        key: &str,
        value: &str,
        now_utc: i64,
    ) -> Result<(), AppError> {
        conn.execute(
            "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
             VALUES (?1, ?2, ?3, ?3) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, \
             updated_at_utc = excluded.updated_at_utc",
            params![key, value, now_utc],
        )
        .map(|_| ())
        .map_err(map_rusqlite)
    }

    // ------------------------------------------------------ справочники

    pub fn device_status_id_by_code(&self, conn: &Connection, code: &str) -> Result<i64, AppError> {
        conn.query_row(
            "SELECT id FROM device_statuses WHERE code = ?1",
            params![code],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)
    }

    // ------------------------------------------------- область сотрудника

    /// Устройства области пользователя (основа BOLA, REP-01): состав (с вложенными) групп,
    /// где он указан в свойстве типа `users`; устройства, на которые эти группы ссылаются
    /// свойством `device_refs`; принтеры, USB-привязанные к устройствам состава.
    /// Только живые группы и устройства. Логика повторяет карточку группы
    /// (`member_printers_for_group` / `usb_printers_for_group` / `ref_devices_for_group`).
    pub fn employee_device_ids(
        &self,
        conn: &Connection,
        user_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        const SQL: &str = "
            WITH RECURSIVE bound(id) AS (
                SELECT DISTINCT g.id FROM group_property_values v
                JOIN group_type_properties p ON p.id = v.property_id AND p.data_type = 'users'
                JOIN groups g ON g.id = v.group_id AND g.deleted_at_utc IS NULL
                WHERE v.value_ref = ?1
            ),
            sub(id) AS (
                SELECT id FROM bound
                UNION
                SELECT g.id FROM groups g JOIN sub ON g.parent_group_id = sub.id
                WHERE g.deleted_at_utc IS NULL
            ),
            members(id) AS (
                SELECT gd.device_id FROM group_devices gd
                JOIN sub ON gd.group_id = sub.id
                JOIN devices d ON d.id = gd.device_id AND d.deleted_at_utc IS NULL
            )
            SELECT id FROM members
            UNION
            SELECT d.id FROM group_property_values v
            JOIN group_type_properties p ON p.id = v.property_id AND p.data_type = 'device_refs'
            JOIN bound b ON b.id = v.group_id
            JOIN devices d ON d.id = v.value_ref AND d.deleted_at_utc IS NULL
            UNION
            SELECT d.id FROM printers pr
            JOIN devices d ON d.id = pr.device_id AND d.deleted_at_utc IS NULL
            WHERE pr.usb_host_device_id IN (SELECT id FROM members)
            ORDER BY 1";
        let mut stmt = conn.prepare(SQL).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![user_id], |r| r.get::<_, i64>(0))
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }
}
