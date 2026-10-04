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

use crate::domain::groups::{
    GroupTypeNew, GroupTypePatch, GroupTypeRow, PropertyNew, PropertyPatch, PropertyRow,
};
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

    // ---- свойства типа -------------------------------------------------

    /// Свойства типа, `ORDER BY sort_order, id`; скрытые — только при `include_archived`.
    fn list_properties(
        &self,
        conn: &Self::Conn,
        type_id: i64,
        include_archived: bool,
    ) -> Result<Vec<PropertyRow>, AppError>;

    /// Свойство по id; `AppError::NotFound`, если его нет.
    fn get_property(&self, conn: &Self::Conn, id: i64) -> Result<PropertyRow, AppError>;

    /// Создать свойство в конце списка типа (`sort_order = MAX + 1`, скрытые учитываются).
    /// Живое свойство с тем же именем в типе -> `AppError::Conflict`.
    fn create_property(
        &self,
        conn: &mut Self::Conn,
        new: &PropertyNew,
        now_utc: i64,
    ) -> Result<i64, AppError>;

    /// Обновить name/data_type/is_required/show_on_map с CAS по `version`.
    fn update_property(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        version: i64,
        patch: &PropertyPatch,
        now_utc: i64,
    ) -> Result<PropertyRow, AppError>;

    /// Скрыть свойство (проставить `archived_at_utc`). Повторное скрытие — не ошибка.
    fn archive_property(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        now_utc: i64,
    ) -> Result<(), AppError>;

    /// Вернуть скрытое свойство. Занятое живое имя в типе -> `AppError::Conflict`.
    fn unarchive_property(
        &self,
        conn: &mut Self::Conn,
        id: i64,
        now_utc: i64,
    ) -> Result<(), AppError>;

    /// Жёстко удалить свойство вместе со значениями (каскад).
    fn delete_property_hard(&self, conn: &mut Self::Conn, id: i64) -> Result<(), AppError>;

    /// Число различных групп с заполненным значением свойства (D-16).
    fn filled_group_count(&self, conn: &Self::Conn, property_id: i64) -> Result<i64, AppError>;

    /// Группы типа без значения по свойству — нарушители обязательности (D-14),
    /// `(id, name)`, `ORDER BY seq`. Скрытость свойства проверяет сервис.
    fn groups_missing_required(
        &self,
        conn: &Self::Conn,
        type_id: i64,
        property_id: i64,
    ) -> Result<Vec<(i64, String)>, AppError>;

    /// Переставить живые свойства типа: `sort_order = 0..n-1` по списку, одна транзакция.
    /// Список должен в точности совпадать с множеством живых свойств типа,
    /// иначе `AppError::Validation { field: "ordered_ids" }` и порядок не меняется.
    fn reorder_properties(
        &self,
        conn: &mut Self::Conn,
        type_id: i64,
        ordered_ids: &[i64],
        now_utc: i64,
    ) -> Result<(), AppError>;
}
