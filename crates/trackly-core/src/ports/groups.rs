//! `GroupRepository` port — хранилище групп устройств (Phase 41).
//!
//! Порт содержит только простое чтение. Составные операции (нумерация,
//! членство, вложенность, значения свойств) — inherent-методы адаптера на
//! `&Transaction`: мутация группы, журнал перемещений и аудит обязаны лежать
//! в ОДНОЙ транзакции, поэтому они не могут быть методами trait'а с
//! автокоммитом. Сервис групп собирает бизнес-правила поверх этих примитивов.

use crate::domain::groups::GroupRow;
use crate::error::AppError;

/// Порт хранилища групп. Реализация — `SqliteGroupRepository` в trackly-infra.
pub trait GroupRepository {
    /// Тип соединения адаптера (например, `rusqlite::Connection`).
    type Conn;

    /// Группа по id; `AppError::NotFound`, если её нет.
    fn get_group(&self, conn: &Self::Conn, id: i64) -> Result<GroupRow, AppError>;

    /// Все живые группы, `ORDER BY type_id, seq`.
    fn list_groups(&self, conn: &Self::Conn) -> Result<Vec<GroupRow>, AppError>;

    /// Пары `(device_id, группа)` только для устройств, состоящих в группе.
    fn groups_for_devices(
        &self,
        conn: &Self::Conn,
        device_ids: &[i64],
    ) -> Result<Vec<(i64, GroupRow)>, AppError>;

    /// Непосредственные вложенные группы, `ORDER BY type_id, seq`.
    fn direct_child_groups(
        &self,
        conn: &Self::Conn,
        parent_id: i64,
    ) -> Result<Vec<GroupRow>, AppError>;
}
