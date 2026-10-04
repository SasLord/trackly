//! `GroupTypeRepository` port — хранилище типов групп и их свойств (Phase 41).
//!
//! Как и `PlaceRepository`: ассоциированный `type Conn` держит rusqlite вне
//! trackly-core; мутации берут `&mut Self::Conn`, чтения — `&Self::Conn`.
//!
//! Репозиторий — единственный слой SQL для `group_types` и
//! `group_type_properties`. Правил здесь нет (скрыть вместо удалить, запрет
//! смены `data_type`, нормализация имён без учёта регистра) — их держит сервис
//! типов поверх этих примитивов. Метода смены `code`/`behavior` нет намеренно
//! (GRP-01): неизменяемость дополнительно закреплена триггером БД.

use crate::domain::groups::{GroupTypeNew, GroupTypePatch, GroupTypeRow};
use crate::error::AppError;

/// Порт хранилища типов групп. Реализация — `SqliteGroupTypeRepository` в trackly-infra.
pub trait GroupTypeRepository {
    /// Тип соединения адаптера (например, `rusqlite::Connection`).
    type Conn;

    /// Все живые типы, `ORDER BY sort_order, id`.
    fn list_types(&self, conn: &Self::Conn) -> Result<Vec<GroupTypeRow>, AppError>;

    /// Тип по id; `AppError::NotFound`, если его нет.
    fn get_type(&self, conn: &Self::Conn, id: i64) -> Result<GroupTypeRow, AppError>;

    /// Тип по машинному коду; `None`, если его нет.
    fn get_type_by_code(
        &self,
        conn: &Self::Conn,
        code: &str,
    ) -> Result<Option<GroupTypeRow>, AppError>;

    /// Создать тип (`version = 1`). Повторный `code` -> `AppError::Conflict`.
    fn create_type(
        &self,
        conn: &mut Self::Conn,
        new: &GroupTypeNew,
        now_utc: i64,
    ) -> Result<i64, AppError>;

    /// Обновить name/sort_order/quick_action_* с CAS по `version`.
    /// Чужая версия -> `OptimisticLockMismatch`, нет строки -> `NotFound`.
    fn update_type(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        version: i64,
        patch: &GroupTypePatch,
        now_utc: i64,
    ) -> Result<GroupTypeRow, AppError>;

    /// Жёстко удалить тип. Ссылающиеся группы (FK RESTRICT) -> `AppError::Conflict`.
    fn delete_type(&self, conn: &mut Self::Conn, id: i64) -> Result<(), AppError>;

    /// Число живых групп данного типа.
    fn count_groups_of_type(&self, conn: &Self::Conn, type_id: i64) -> Result<i64, AppError>;
}
