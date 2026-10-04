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
use trackly_core::domain::groups::{GroupNew, GroupRow};
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
