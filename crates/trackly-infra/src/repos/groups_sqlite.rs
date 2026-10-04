//! SQLite-адаптер `GroupRepository` (Phase 41).
//!
//! Единственное место SQL по `groups`, `group_devices` и `group_property_values`.
//! Все запросы параметризованы через `rusqlite::params![...]` (T-41-06-03):
//! ввод вызывающего никогда не конкатенируется в текст SQL. Единственная сборка
//! текста — список `?`-заглушек для `IN (...)` по числу элементов.
//!
//! Составные операции — inherent-методы `*_in_tx(&self, tx: &Transaction<'_>, ...)`:
//! мутация, журнал перемещений и аудит обязаны лежать в одной транзакции
//! вызывающего сервиса. Инварианты, которые нельзя нарушить даже при ошибке
//! сервиса, живут здесь: одна группа на устройство (PK `group_devices.device_id`),
//! отсутствие циклов вложенности, запрет вложения в группу «Разбор».

use rusqlite::{Connection, OptionalExtension, Transaction};
use trackly_core::domain::groups::{
    GroupNew, GroupRow, GroupValueRow, MemberDeviceRow, PrinterRefRow,
};
use trackly_core::error::AppError;
use trackly_core::ports::groups::GroupRepository;

use crate::error_conversions::map_rusqlite;

/// SQLite-реализация репозитория групп (zero-sized, как `SqlitePlaceRepository`).
#[derive(Debug, Default, Clone)]
pub struct SqliteGroupRepository;

/// Колонки группы (алиас `g`) в порядке, который ждёт `group_from_row_at`.
const GROUP_COLS: &str = "g.id, g.type_id, g.name, g.seq, g.place_id, g.parent_group_id, \
                          g.version, g.created_at_utc, g.updated_at_utc";

/// Сообщение о цикле вложенности.
const CYCLE_MESSAGE: &str = "Нельзя вложить группу в саму себя или в свою вложенную группу.";

/// Рекурсивный CTE `sub(id)`: корень (параметр `?1`) и все его живые потомки.
const SUBTREE_CTE: &str = "WITH RECURSIVE sub(id) AS (
        SELECT id FROM groups WHERE id = ?1 AND deleted_at_utc IS NULL
        UNION
        SELECT g.id FROM groups g JOIN sub ON g.parent_group_id = sub.id
        WHERE g.deleted_at_utc IS NULL
     )";

fn group_from_row_at(row: &rusqlite::Row<'_>, o: usize) -> rusqlite::Result<GroupRow> {
    Ok(GroupRow {
        id: row.get(o)?,
        type_id: row.get(o + 1)?,
        name: row.get(o + 2)?,
        seq: row.get(o + 3)?,
        place_id: row.get(o + 4)?,
        parent_group_id: row.get(o + 5)?,
        version: row.get(o + 6)?,
        created_at_utc: row.get(o + 7)?,
        updated_at_utc: row.get(o + 8)?,
    })
}

fn group_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GroupRow> {
    group_from_row_at(row, 0)
}

fn get_group_impl(conn: &Connection, id: i64) -> Result<GroupRow, AppError> {
    let sql = [
        "SELECT ",
        GROUP_COLS,
        " FROM groups g WHERE g.id = ?1 AND g.deleted_at_utc IS NULL",
    ]
    .concat();
    conn.query_row(&sql, rusqlite::params![id], group_from_row)
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
                entity: "group",
                id,
            },
            other => map_rusqlite(other),
        })
}

/// Нулевое число затронутых строк CAS -> `NotFound` или `OptimisticLockMismatch`.
fn resolve_group_cas_failure(conn: &Connection, id: i64, expected: i64) -> AppError {
    let actual: Option<i64> = conn
        .query_row(
            "SELECT version FROM groups WHERE id = ?1 AND deleted_at_utc IS NULL",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .optional()
        .unwrap_or(None);
    match actual {
        None => AppError::NotFound {
            entity: "group",
            id,
        },
        Some(actual) => AppError::OptimisticLockMismatch {
            entity: "group",
            id,
            expected,
            actual,
        },
    }
}

fn collect_groups(
    conn: &Connection,
    sql: &str,
    params: impl rusqlite::Params,
) -> Result<Vec<GroupRow>, AppError> {
    let mut stmt = conn.prepare(sql).map_err(map_rusqlite)?;
    let rows = stmt
        .query_map(params, group_from_row)
        .map_err(map_rusqlite)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_rusqlite)?;
    Ok(rows)
}

impl SqliteGroupRepository {
    /// Следующий порядковый номер внутри типа: `MAX(seq) + 1` по целочисленной
    /// колонке (GRP-04). Имя группы обратно не разбирается. Учитывает и
    /// мягко удалённые строки — `UNIQUE(type_id, seq)` покрывает их тоже.
    /// Номер последней удалённой (жёстко) группы выдаётся повторно: принятое поведение.
    pub fn next_seq_in_tx(&self, tx: &Transaction<'_>, type_id: i64) -> Result<i64, AppError> {
        tx.query_row(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM groups WHERE type_id = ?1",
            rusqlite::params![type_id],
            |r| r.get(0),
        )
        .map_err(map_rusqlite)
    }

    /// Вставить группу (`version = 1`, без родителя). Повторный `(type_id, seq)` -> `Conflict`.
    pub fn insert_group_in_tx(
        &self,
        tx: &Transaction<'_>,
        new: &GroupNew,
        now_utc: i64,
    ) -> Result<i64, AppError> {
        tx.execute(
            "INSERT INTO groups
               (type_id, name, seq, place_id, parent_group_id, created_at_utc, updated_at_utc, version)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?5, 1)",
            rusqlite::params![new.type_id, new.name, new.seq, new.place_id, now_utc],
        )
        .map_err(map_rusqlite)?;
        Ok(tx.last_insert_rowid())
    }

    /// Группа по id в транзакции; `NotFound`, если её нет.
    pub fn get_group_in_tx(&self, tx: &Transaction<'_>, id: i64) -> Result<GroupRow, AppError> {
        get_group_impl(tx, id)
    }

    /// Переименовать группу с CAS по `version`. Имя уже провалидировано сервисом.
    pub fn rename_group_in_tx(
        &self,
        tx: &Transaction<'_>,
        id: i64,
        version: i64,
        name: &str,
        now_utc: i64,
    ) -> Result<GroupRow, AppError> {
        let affected = tx
            .execute(
                "UPDATE groups SET name = ?1, updated_at_utc = ?2, version = version + 1
                 WHERE id = ?3 AND version = ?4 AND deleted_at_utc IS NULL",
                rusqlite::params![name, now_utc, id, version],
            )
            .map_err(map_rusqlite)?;
        if affected == 0 {
            return Err(resolve_group_cas_failure(tx, id, version));
        }
        get_group_impl(tx, id)
    }

    /// Жёстко удалить группу. Каскады БД: строки `group_devices` и значения
    /// свойств исчезают, вложенные группы получают `parent_group_id = NULL`;
    /// `devices.place_id` бывших членов не меняется. Нет строки -> `NotFound`.
    pub fn delete_group_in_tx(&self, tx: &Transaction<'_>, id: i64) -> Result<(), AppError> {
        let affected = tx
            .execute("DELETE FROM groups WHERE id = ?1", rusqlite::params![id])
            .map_err(map_rusqlite)?;
        if affected == 0 {
            return Err(AppError::NotFound {
                entity: "group",
                id,
            });
        }
        Ok(())
    }

    /// Добавить устройство в группу. Устройство уже состоит в какой-либо группе
    /// (PK `group_devices.device_id`) -> `Conflict`, причина содержит `device_id`;
    /// сервис заменяет её русским сообщением с именем группы.
    pub fn add_device_in_tx(
        &self,
        tx: &Transaction<'_>,
        group_id: i64,
        device_id: i64,
        now_utc: i64,
    ) -> Result<(), AppError> {
        if let Some(existing) = self.group_of_device_in_tx(tx, device_id)? {
            return Err(AppError::Conflict {
                reason: format!(
                    "device {device_id} already belongs to group {}",
                    existing.id
                ),
            });
        }
        tx.execute(
            "INSERT INTO group_devices (device_id, group_id, added_at_utc) VALUES (?1, ?2, ?3)",
            rusqlite::params![device_id, group_id, now_utc],
        )
        .map_err(|e| match map_rusqlite(e) {
            // Гонка за PK, не замеченная предпроверкой.
            AppError::Conflict { reason } => AppError::Conflict {
                reason: format!("device {device_id}: {reason}"),
            },
            other => other,
        })?;
        Ok(())
    }

    /// Убрать устройство из группы; `true`, если членство было.
    pub fn remove_device_in_tx(
        &self,
        tx: &Transaction<'_>,
        device_id: i64,
    ) -> Result<bool, AppError> {
        let affected = tx
            .execute(
                "DELETE FROM group_devices WHERE device_id = ?1",
                rusqlite::params![device_id],
            )
            .map_err(map_rusqlite)?;
        Ok(affected > 0)
    }

    /// Группа, в которой состоит устройство.
    pub fn group_of_device_in_tx(
        &self,
        tx: &Transaction<'_>,
        device_id: i64,
    ) -> Result<Option<GroupRow>, AppError> {
        let sql = [
            "SELECT ",
            GROUP_COLS,
            " FROM group_devices gd JOIN groups g ON g.id = gd.group_id
              WHERE gd.device_id = ?1 AND g.deleted_at_utc IS NULL",
        ]
        .concat();
        tx.query_row(&sql, rusqlite::params![device_id], group_from_row)
            .optional()
            .map_err(map_rusqlite)
    }

    /// Группа, «запирающая» устройство: только с `place_id IS NOT NULL`. Так D-21
    /// («запрет спит», пока у группы нет места) реализован на уровне запроса.
    pub fn locked_group_for_device_in_tx(
        &self,
        tx: &Transaction<'_>,
        device_id: i64,
    ) -> Result<Option<GroupRow>, AppError> {
        let sql = [
            "SELECT ",
            GROUP_COLS,
            " FROM group_devices gd JOIN groups g ON g.id = gd.group_id
              WHERE gd.device_id = ?1 AND g.deleted_at_utc IS NULL AND g.place_id IS NOT NULL",
        ]
        .concat();
        tx.query_row(&sql, rusqlite::params![device_id], group_from_row)
            .optional()
            .map_err(map_rusqlite)
    }

    /// Создаст ли `new_parent_id` цикл для `group_id`: родитель — сама группа
    /// либо один из её потомков (то есть `group_id` среди предков родителя).
    pub fn would_create_cycle_in_tx(
        &self,
        tx: &Transaction<'_>,
        group_id: i64,
        new_parent_id: i64,
    ) -> Result<bool, AppError> {
        // UNION (не ALL): даже уже испорченные данные с циклом не зациклят запрос.
        let is_cycle: i64 = tx
            .query_row(
                "WITH RECURSIVE ancestors(id) AS (
                    SELECT parent_group_id FROM groups WHERE id = ?1
                    UNION
                    SELECT g.parent_group_id FROM groups g
                    JOIN ancestors a ON g.id = a.id
                    WHERE g.parent_group_id IS NOT NULL
                 )
                 SELECT EXISTS(
                   SELECT 1 WHERE ?1 = ?2
                   UNION ALL
                   SELECT 1 FROM ancestors WHERE id = ?2
                 )",
                rusqlite::params![new_parent_id, group_id],
                |r| r.get(0),
            )
            .map_err(map_rusqlite)?;
        Ok(is_cycle != 0)
    }

    /// Поведение типа группы-родителя (`group_types.behavior`); `NotFound`, если группы нет.
    pub fn parent_behavior_in_tx(
        &self,
        tx: &Transaction<'_>,
        parent_id: i64,
    ) -> Result<String, AppError> {
        tx.query_row(
            "SELECT t.behavior FROM groups g JOIN group_types t ON t.id = g.type_id
             WHERE g.id = ?1 AND g.deleted_at_utc IS NULL",
            rusqlite::params![parent_id],
            |r| r.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
                entity: "group",
                id: parent_id,
            },
            other => map_rusqlite(other),
        })
    }

    /// Сменить родителя с CAS. Перед `UPDATE` в той же транзакции: цикл ->
    /// `Validation{parent_group_id}`; родитель с `behavior = 'teardown'` ->
    /// `Validation{parent_group_id}`. `None` — вынос в корень, проверок не требует.
    pub fn set_parent_in_tx(
        &self,
        tx: &Transaction<'_>,
        id: i64,
        version: i64,
        parent: Option<i64>,
        now_utc: i64,
    ) -> Result<GroupRow, AppError> {
        if let Some(p) = parent {
            if self.would_create_cycle_in_tx(tx, id, p)? {
                return Err(AppError::Validation {
                    field: "parent_group_id".to_string(),
                    message: CYCLE_MESSAGE.to_string(),
                });
            }
            if self.parent_behavior_in_tx(tx, p)? == "teardown" {
                return Err(AppError::Validation {
                    field: "parent_group_id".to_string(),
                    message: "Группа типа «Разбор» не может содержать другие группы.".to_string(),
                });
            }
        }
        let affected = tx
            .execute(
                "UPDATE groups SET parent_group_id = ?1, updated_at_utc = ?2, version = version + 1
                 WHERE id = ?3 AND version = ?4 AND deleted_at_utc IS NULL",
                rusqlite::params![parent, now_utc, id, version],
            )
            .map_err(map_rusqlite)?;
        if affected == 0 {
            return Err(resolve_group_cas_failure(tx, id, version));
        }
        get_group_impl(tx, id)
    }

    /// Корневая группа цепочки вложенности (для корня — он сам). Имя корня
    /// показывается в поле места вложенной группы.
    pub fn root_group_id_in_tx(&self, tx: &Transaction<'_>, id: i64) -> Result<i64, AppError> {
        tx.query_row(
            "WITH RECURSIVE up(id, parent) AS (
                SELECT id, parent_group_id FROM groups WHERE id = ?1 AND deleted_at_utc IS NULL
                UNION
                SELECT g.id, g.parent_group_id FROM groups g JOIN up ON g.id = up.parent
                WHERE g.deleted_at_utc IS NULL
             )
             SELECT id FROM up WHERE parent IS NULL LIMIT 1",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound {
                entity: "group",
                id,
            },
            other => map_rusqlite(other),
        })
    }

    /// Id группы-корня и всех её живых вложенных групп, `ORDER BY id`.
    pub fn subtree_group_ids_in_tx(
        &self,
        tx: &Transaction<'_>,
        root: i64,
    ) -> Result<Vec<i64>, AppError> {
        let sql = [SUBTREE_CTE, " SELECT id FROM sub ORDER BY id"].concat();
        let mut stmt = tx.prepare(&sql).map_err(map_rusqlite)?;
        let ids = stmt
            .query_map(rusqlite::params![root], |r| r.get::<_, i64>(0))
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(ids)
    }

    /// Состав группы с вложенными: id живых устройств (`deleted_at_utc IS NULL`) корня
    /// и всех вложенных групп, `ORDER BY device_id` (D-03/D-20).
    pub fn subtree_device_ids_in_tx(
        &self,
        tx: &Transaction<'_>,
        root: i64,
    ) -> Result<Vec<i64>, AppError> {
        let sql = [
            SUBTREE_CTE,
            " SELECT gd.device_id FROM group_devices gd
              JOIN sub ON gd.group_id = sub.id
              JOIN devices d ON d.id = gd.device_id AND d.deleted_at_utc IS NULL
              ORDER BY gd.device_id",
        ]
        .concat();
        let mut stmt = tx.prepare(&sql).map_err(map_rusqlite)?;
        let ids = stmt
            .query_map(rusqlite::params![root], |r| r.get::<_, i64>(0))
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(ids)
    }

    /// Проставить место всем группам поддерева — денормализация места вложенных
    /// групп (Pitfall 4). Возвращает число затронутых групп.
    pub fn set_subtree_place_in_tx(
        &self,
        tx: &Transaction<'_>,
        root: i64,
        place: Option<i64>,
        now_utc: i64,
    ) -> Result<usize, AppError> {
        let sql = [
            SUBTREE_CTE,
            " UPDATE groups SET place_id = ?2, updated_at_utc = ?3, version = version + 1
              WHERE id IN (SELECT id FROM sub)",
        ]
        .concat();
        tx.execute(&sql, rusqlite::params![root, place, now_utc])
            .map_err(map_rusqlite)
    }
}

fn printer_ref_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PrinterRefRow> {
    Ok(PrinterRefRow {
        device_id: row.get(0)?,
        name: row.get(1)?,
        inventory_number: row.get(2)?,
        serial_number: row.get(3)?,
    })
}

impl SqliteGroupRepository {
    /// Значения свойств группы, `ORDER BY property_id, position`.
    pub fn list_values(
        &self,
        conn: &Connection,
        group_id: i64,
    ) -> Result<Vec<GroupValueRow>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT group_id, property_id, position, value_text, value_ref, is_primary
                 FROM group_property_values
                 WHERE group_id = ?1
                 ORDER BY property_id, position, id",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(rusqlite::params![group_id], |r| {
                let is_primary: i64 = r.get(5)?;
                Ok(GroupValueRow {
                    group_id: r.get(0)?,
                    property_id: r.get(1)?,
                    position: r.get(2)?,
                    value_text: r.get(3)?,
                    value_ref: r.get(4)?,
                    is_primary: is_primary != 0,
                })
            })
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// Атомарно заменить значения `(group_id, property_id)`: удалить прежние
    /// строки и вставить новые. Пустой набор -> строк нет («пусто» = отсутствие
    /// строки). `group_id`/`property_id` берутся из аргументов, а не из строк.
    /// Нарушение частичных индексов (два `is_primary`, повтор ссылки) -> `Conflict`,
    /// при этом прежние значения остаются на месте (откат к savepoint).
    pub fn replace_property_values_in_tx(
        &self,
        tx: &Transaction<'_>,
        group_id: i64,
        property_id: i64,
        values: &[GroupValueRow],
        now_utc: i64,
    ) -> Result<(), AppError> {
        tx.execute_batch("SAVEPOINT gpv_replace")
            .map_err(map_rusqlite)?;
        let result = (|| -> Result<(), AppError> {
            tx.execute(
                "DELETE FROM group_property_values WHERE group_id = ?1 AND property_id = ?2",
                rusqlite::params![group_id, property_id],
            )
            .map_err(map_rusqlite)?;
            for v in values {
                tx.execute(
                    "INSERT INTO group_property_values
                       (group_id, property_id, position, value_text, value_ref, is_primary,
                        updated_at_utc)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![
                        group_id,
                        property_id,
                        v.position,
                        v.value_text,
                        v.value_ref,
                        v.is_primary as i64,
                        now_utc
                    ],
                )
                .map_err(map_rusqlite)?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => tx
                .execute_batch("RELEASE gpv_replace")
                .map_err(map_rusqlite),
            Err(e) => {
                // Откат к savepoint и его снятие; исходная ошибка важнее ошибки отката.
                let _ = tx.execute_batch("ROLLBACK TO gpv_replace; RELEASE gpv_replace");
                Err(e)
            }
        }
    }

    /// Принтеры, USB-привязанные к устройствам состава группы (включая вложенные
    /// группы): производные, ручной записи нет (GRP-08, D-12). Только живые
    /// устройства — и принтер, и хост. Дедупликацию с явными ссылками делает сервис.
    pub fn usb_printers_for_group(
        &self,
        conn: &Connection,
        group_id: i64,
    ) -> Result<Vec<PrinterRefRow>, AppError> {
        let sql = [
            SUBTREE_CTE,
            " SELECT d.id, d.name, d.inventory_number, d.serial_number
              FROM printers p
              JOIN devices d ON d.id = p.device_id AND d.deleted_at_utc IS NULL
              WHERE p.usb_host_device_id IN (
                SELECT gd.device_id FROM group_devices gd
                JOIN sub ON gd.group_id = sub.id
                JOIN devices h ON h.id = gd.device_id AND h.deleted_at_utc IS NULL
              )
              ORDER BY d.name, d.id",
        ]
        .concat();
        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(rusqlite::params![group_id], printer_ref_from_row)
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// Живые устройства, на которые группа ссылается через значения свойств
    /// `property_ids` (тип `device_refs`). Одно устройство — одна строка.
    pub fn ref_devices_for_group(
        &self,
        conn: &Connection,
        group_id: i64,
        property_ids: &[i64],
    ) -> Result<Vec<PrinterRefRow>, AppError> {
        if property_ids.is_empty() {
            return Ok(Vec::new());
        }
        // Только `?`-заглушки по числу элементов: ввод в текст SQL не попадает.
        let placeholders = vec!["?"; property_ids.len()].join(",");
        let sql = [
            "SELECT DISTINCT d.id, d.name, d.inventory_number, d.serial_number
             FROM group_property_values v
             JOIN devices d ON d.id = v.value_ref AND d.deleted_at_utc IS NULL
             WHERE v.group_id = ?1 AND v.value_ref IS NOT NULL AND v.property_id IN (",
            &placeholders,
            ") ORDER BY d.name, d.id",
        ]
        .concat();
        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let params = std::iter::once(group_id).chain(property_ids.iter().copied());
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params), printer_ref_from_row)
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// Для каждой живой группы — число живых устройств состава ВКЛЮЧАЯ вложенные
    /// группы. Один рекурсивный запрос по индексам `idx_groups_parent` /
    /// `idx_group_devices_group`, не N+1. `(group_id, device_count)`, `ORDER BY group_id`.
    pub fn tree_counts(&self, conn: &Connection) -> Result<Vec<(i64, i64)>, AppError> {
        let mut stmt = conn
            .prepare(
                "WITH RECURSIVE closure(root, id) AS (
                    SELECT id, id FROM groups WHERE deleted_at_utc IS NULL
                    UNION ALL
                    SELECT c.root, g.id FROM groups g JOIN closure c ON g.parent_group_id = c.id
                    WHERE g.deleted_at_utc IS NULL
                 )
                 SELECT c.root, COUNT(d.id)
                 FROM closure c
                 LEFT JOIN group_devices gd ON gd.group_id = c.id
                 LEFT JOIN devices d ON d.id = gd.device_id AND d.deleted_at_utc IS NULL
                 GROUP BY c.root
                 ORDER BY c.root",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// Прямые живые устройства группы (без вложенных групп) со столбцами таблицы
    /// состава (D-05), `ORDER BY name, id`. Путь места — из `place_full_paths`;
    /// сокращение пути считает сервис (единственный владелец формулы).
    pub fn member_devices(
        &self,
        conn: &Connection,
        group_id: i64,
    ) -> Result<Vec<MemberDeviceRow>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT d.id, COALESCE(dt.name, ''), d.name, d.inventory_number,
                        d.serial_number, d.place_id, pfp.full_path, ds.name
                 FROM group_devices gd
                 JOIN devices d ON d.id = gd.device_id AND d.deleted_at_utc IS NULL
                 LEFT JOIN device_types dt ON dt.id = d.type_id
                 LEFT JOIN device_statuses ds ON ds.id = d.status_id
                 LEFT JOIN place_full_paths pfp ON pfp.place_id = d.place_id
                 WHERE gd.group_id = ?1
                 ORDER BY d.name, d.id",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(rusqlite::params![group_id], |r| {
                Ok(MemberDeviceRow {
                    device_id: r.get(0)?,
                    type_name: r.get(1)?,
                    name: r.get(2)?,
                    inventory_number: r.get(3)?,
                    serial_number: r.get(4)?,
                    place_id: r.get(5)?,
                    place_path: r.get(6)?,
                    status_name: r.get(7)?,
                })
            })
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// Для каждой живой группы — число её ПРЯМЫХ живых устройств (без вложенных):
    /// `(group_id, count)`; группы без устройств в выдачу не попадают.
    pub fn direct_device_counts(&self, conn: &Connection) -> Result<Vec<(i64, i64)>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT gd.group_id, COUNT(d.id)
                 FROM group_devices gd
                 JOIN groups g ON g.id = gd.group_id AND g.deleted_at_utc IS NULL
                 JOIN devices d ON d.id = gd.device_id AND d.deleted_at_utc IS NULL
                 GROUP BY gd.group_id
                 ORDER BY gd.group_id",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// Полный путь места каждой живой группы, у которой место задано:
    /// `(group_id, full_path)`.
    pub fn group_place_paths(&self, conn: &Connection) -> Result<Vec<(i64, String)>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT g.id, pfp.full_path
                 FROM groups g
                 JOIN place_full_paths pfp ON pfp.place_id = g.place_id
                 WHERE g.deleted_at_utc IS NULL
                 ORDER BY g.id",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// Число живых групп по типам: `(type_id, group_count)`, `ORDER BY type_id`.
    pub fn group_counts_by_type(&self, conn: &Connection) -> Result<Vec<(i64, i64)>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT type_id, COUNT(*) FROM groups
                 WHERE deleted_at_utc IS NULL GROUP BY type_id ORDER BY type_id",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }
}

/// Подмножество `ids`, которое есть в результате запроса `SELECT id ... IN (?, ...)`.
/// В текст SQL попадают только `?`-заглушки по числу элементов (T-41-12-01).
fn existing_ids(
    conn: &Connection,
    head: &str,
    tail: &str,
    ids: &[i64],
) -> Result<std::collections::HashSet<i64>, AppError> {
    if ids.is_empty() {
        return Ok(std::collections::HashSet::new());
    }
    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = [head, &placeholders, tail].concat();
    let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(ids.iter().copied()), |r| {
            r.get::<_, i64>(0)
        })
        .map_err(map_rusqlite)?
        .collect::<Result<std::collections::HashSet<_>, _>>()
        .map_err(map_rusqlite)?;
    Ok(rows)
}

/// Живой и активный пользователь: не удалён мягко и не отключён.
const LIVE_USERS_HEAD: &str =
    "SELECT id FROM users WHERE deleted_at_utc IS NULL AND is_active = 1 AND id IN (";
const LIVE_DEVICES_HEAD: &str = "SELECT id FROM devices WHERE deleted_at_utc IS NULL AND id IN (";

impl SqliteGroupRepository {
    /// Увеличить `version` группы на 1 с CAS (запись значений меняет группу).
    pub fn bump_group_version_in_tx(
        &self,
        tx: &Transaction<'_>,
        id: i64,
        version: i64,
        now_utc: i64,
    ) -> Result<GroupRow, AppError> {
        let affected = tx
            .execute(
                "UPDATE groups SET updated_at_utc = ?1, version = version + 1
                 WHERE id = ?2 AND version = ?3 AND deleted_at_utc IS NULL",
                rusqlite::params![now_utc, id, version],
            )
            .map_err(map_rusqlite)?;
        if affected == 0 {
            return Err(resolve_group_cas_failure(tx, id, version));
        }
        get_group_impl(tx, id)
    }

    /// Из `ids` — только живые активные пользователи.
    pub fn live_user_ids_in_tx(
        &self,
        tx: &Transaction<'_>,
        ids: &[i64],
    ) -> Result<std::collections::HashSet<i64>, AppError> {
        existing_ids(tx, LIVE_USERS_HEAD, ")", ids)
    }

    /// Из `ids` — только живые устройства.
    pub fn live_device_ids_in_tx(
        &self,
        tx: &Transaction<'_>,
        ids: &[i64],
    ) -> Result<std::collections::HashSet<i64>, AppError> {
        existing_ids(tx, LIVE_DEVICES_HEAD, ")", ids)
    }

    /// Из `ids` — живые устройства, у которых есть строка `printers`.
    pub fn live_printer_device_ids(
        &self,
        conn: &Connection,
        ids: &[i64],
    ) -> Result<std::collections::HashSet<i64>, AppError> {
        existing_ids(
            conn,
            "SELECT d.id FROM devices d JOIN printers p ON p.device_id = d.id
             WHERE d.deleted_at_utc IS NULL AND d.id IN (",
            ")",
            ids,
        )
    }

    /// Живые активные пользователи по id: `(id, full_name)`.
    pub fn live_users_by_ids(
        &self,
        conn: &Connection,
        ids: &[i64],
    ) -> Result<Vec<(i64, String)>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; ids.len()].join(",");
        let sql = [
            "SELECT id, full_name FROM users
             WHERE deleted_at_utc IS NULL AND is_active = 1 AND id IN (",
            &placeholders,
            ")",
        ]
        .concat();
        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(ids.iter().copied()), |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    /// Есть ли у группы непустое значение свойства (строка со ссылкой или непустой текст).
    pub fn property_filled_in_tx(
        &self,
        tx: &Transaction<'_>,
        group_id: i64,
        property_id: i64,
    ) -> Result<bool, AppError> {
        tx.query_row(
            "SELECT EXISTS(
               SELECT 1 FROM group_property_values
               WHERE group_id = ?1 AND property_id = ?2
                 AND (value_ref IS NOT NULL OR (value_text IS NOT NULL AND value_text <> '')))",
            rusqlite::params![group_id, property_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|v| v != 0)
        .map_err(map_rusqlite)
    }

    /// Пользователи для выбора в свойстве «Пользователи» (D-11): только живые активные,
    /// только `(id, full_name, login)`. Фильтр по подстроке делает сервис в Rust.
    pub fn list_user_options(
        &self,
        conn: &Connection,
    ) -> Result<Vec<(i64, String, String)>, AppError> {
        let mut stmt = conn
            .prepare(
                "SELECT id, full_name, login FROM users
                 WHERE deleted_at_utc IS NULL AND is_active = 1
                 ORDER BY full_name, id",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }
}

impl GroupRepository for SqliteGroupRepository {
    type Conn = Connection;

    fn get_group(&self, conn: &Connection, id: i64) -> Result<GroupRow, AppError> {
        get_group_impl(conn, id)
    }

    fn list_groups(&self, conn: &Connection) -> Result<Vec<GroupRow>, AppError> {
        let sql = [
            "SELECT ",
            GROUP_COLS,
            " FROM groups g WHERE g.deleted_at_utc IS NULL ORDER BY g.type_id, g.seq",
        ]
        .concat();
        collect_groups(conn, &sql, [])
    }

    fn groups_for_devices(
        &self,
        conn: &Connection,
        device_ids: &[i64],
    ) -> Result<Vec<(i64, GroupRow)>, AppError> {
        if device_ids.is_empty() {
            return Ok(Vec::new());
        }
        // Только `?`-заглушки по числу элементов: ввод в текст SQL не попадает.
        let placeholders = vec!["?"; device_ids.len()].join(",");
        let sql = [
            "SELECT gd.device_id, ",
            GROUP_COLS,
            " FROM group_devices gd JOIN groups g ON g.id = gd.group_id
              WHERE g.deleted_at_utc IS NULL AND gd.device_id IN (",
            &placeholders,
            ") ORDER BY gd.device_id",
        ]
        .concat();
        let mut stmt = conn.prepare(&sql).map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(device_ids.iter()), |r| {
                Ok((r.get::<_, i64>(0)?, group_from_row_at(r, 1)?))
            })
            .map_err(map_rusqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_rusqlite)?;
        Ok(rows)
    }

    fn direct_child_groups(
        &self,
        conn: &Connection,
        parent_id: i64,
    ) -> Result<Vec<GroupRow>, AppError> {
        let sql = [
            "SELECT ",
            GROUP_COLS,
            " FROM groups g WHERE g.parent_group_id = ?1 AND g.deleted_at_utc IS NULL
              ORDER BY g.type_id, g.seq",
        ]
        .concat();
        collect_groups(conn, &sql, rusqlite::params![parent_id])
    }
}
