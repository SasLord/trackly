//! `auth` — чистый domain-модуль авторизации (D-Auth-01).
//!
//! Определяет [`Role`], [`Identity`], [`Action`] и функцию [`authorize`].
//!
//! **Инвариант:** ни одного I/O-импорта — нет tokio, rusqlite, serde, axum.
//! Gate `no_io_deps` проверяет это при каждом `cargo test`.

use crate::error::AppError;

// ---------------------------------------------------------------------------
// Role
// ---------------------------------------------------------------------------

/// Роль пользователя в системе.
///
/// Три уровня доступа: Admin > Manager > Employee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    /// Полный доступ: управление пользователями, настройками, устройствами, актами, картриджами.
    Admin,
    /// Расширенный доступ: мутации устройств, актов, картриджей, чтение.
    Manager,
    /// Базовый доступ: чтение данных, создание заявок.
    Employee,
}

impl Role {
    /// Создаёт [`Role`] из строки.
    ///
    /// # Errors
    ///
    /// Возвращает [`AppError::Validation`] с `field = "role"` если строка не распознана.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "admin" => Ok(Self::Admin),
            "manager" => Ok(Self::Manager),
            "employee" => Ok(Self::Employee),
            other => Err(AppError::Validation {
                field: "role".to_string(),
                message: format!("unknown role: {other}"),
            }),
        }
    }

    /// Возвращает строковое представление роли (обратно от [`Role::from_str`]).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Manager => "manager",
            Self::Employee => "employee",
        }
    }
}

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

/// Идентификатор вошедшего пользователя (session subject).
///
/// `user_id = None` — «доверенный администратор» (D-Desktop-01 unlocked mode):
/// десктоп-приложение без включённого режима блокировки.
#[derive(Debug, Clone)]
pub struct Identity {
    /// ID пользователя из таблицы `users`. `None` — unlocked desktop mode.
    pub user_id: Option<i64>,
    /// Роль пользователя.
    pub role: Role,
}

impl Identity {
    /// Создаёт «доверенного администратора» для десктоп-режима без блокировки (D-Desktop-01).
    ///
    /// `user_id = None`, `role = Admin` — полный доступ без записи в audit_log.
    pub fn trusted_admin() -> Self {
        Self {
            user_id: None,
            role: Role::Admin,
        }
    }
}

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

/// Действия, для которых нужна авторизация.
#[derive(Debug, Clone)]
pub enum Action {
    /// Создание, изменение, удаление, блокировка пользователей. Admin only.
    ManageUsers,
    /// Изменение глобальных настроек приложения. Admin only.
    ManageSettings,
    /// Создание, изменение, удаление устройств. Admin | Manager.
    MutateDevices,
    /// Создание, изменение, удаление актов приёма-передачи. Admin | Manager.
    MutateActs,
    /// Создание, изменение, удаление операций с картриджами. Admin | Manager.
    MutateCartridges,
    /// Просмотр любых данных. All roles (always Ok).
    ReadData,
    /// Отправка заявки (через браузер). All roles (always Ok).
    CreateRequest,
}

// ---------------------------------------------------------------------------
// authorize
// ---------------------------------------------------------------------------

/// Проверяет, разрешено ли `action` для `identity`.
///
/// # Permission matrix
///
/// | Action             | Admin | Manager | Employee |
/// |--------------------|-------|---------|----------|
/// | ManageUsers        | ✓     | ✗       | ✗        |
/// | ManageSettings     | ✓     | ✗       | ✗        |
/// | MutateDevices      | ✓     | ✓       | ✗        |
/// | MutateActs         | ✓     | ✓       | ✗        |
/// | MutateCartridges   | ✓     | ✓       | ✗        |
/// | ReadData           | ✓     | ✓       | ✓        |
/// | CreateRequest      | ✓     | ✓       | ✓        |
///
/// # Errors
///
/// Возвращает [`AppError::Forbidden`] если роль не имеет прав.
pub fn authorize(identity: &Identity, action: &Action) -> Result<(), AppError> {
    let allowed = match action {
        Action::ManageUsers | Action::ManageSettings => {
            matches!(identity.role, Role::Admin)
        }
        Action::MutateDevices | Action::MutateActs | Action::MutateCartridges => {
            matches!(identity.role, Role::Admin | Role::Manager)
        }
        Action::ReadData | Action::CreateRequest => true,
    };

    if allowed {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Role::from_str

    #[test]
    fn role_from_str_admin() {
        assert_eq!(Role::from_str("admin").unwrap(), Role::Admin);
    }

    #[test]
    fn role_from_str_manager() {
        assert_eq!(Role::from_str("manager").unwrap(), Role::Manager);
    }

    #[test]
    fn role_from_str_employee() {
        assert_eq!(Role::from_str("employee").unwrap(), Role::Employee);
    }

    #[test]
    fn role_from_str_unknown_returns_validation_error() {
        let err = Role::from_str("unknown").expect_err("должна быть ошибка");
        match err {
            AppError::Validation { field, .. } => {
                assert_eq!(field, "role");
            }
            other => panic!("ожидали AppError::Validation, получили {other:?}"),
        }
    }

    // Role::as_str

    #[test]
    fn role_as_str_roundtrip() {
        assert_eq!(Role::Admin.as_str(), "admin");
        assert_eq!(Role::Manager.as_str(), "manager");
        assert_eq!(Role::Employee.as_str(), "employee");
    }

    // authorize — Admin

    #[test]
    fn authorize_admin_manage_users_ok() {
        let id = Identity { user_id: Some(1), role: Role::Admin };
        assert!(authorize(&id, &Action::ManageUsers).is_ok());
    }

    // authorize — Manager denials

    #[test]
    fn authorize_manager_manage_users_forbidden() {
        let id = Identity { user_id: Some(2), role: Role::Manager };
        assert!(matches!(authorize(&id, &Action::ManageUsers), Err(AppError::Forbidden)));
    }

    #[test]
    fn authorize_employee_manage_users_forbidden() {
        let id = Identity { user_id: Some(3), role: Role::Employee };
        assert!(matches!(authorize(&id, &Action::ManageUsers), Err(AppError::Forbidden)));
    }

    // authorize — Manager mutations

    #[test]
    fn authorize_manager_mutate_devices_ok() {
        let id = Identity { user_id: Some(2), role: Role::Manager };
        assert!(authorize(&id, &Action::MutateDevices).is_ok());
    }

    #[test]
    fn authorize_employee_mutate_devices_forbidden() {
        let id = Identity { user_id: Some(3), role: Role::Employee };
        assert!(matches!(authorize(&id, &Action::MutateDevices), Err(AppError::Forbidden)));
    }

    // authorize — ReadData (all roles)

    #[test]
    fn authorize_employee_read_data_ok() {
        let id = Identity { user_id: Some(3), role: Role::Employee };
        assert!(authorize(&id, &Action::ReadData).is_ok());
    }

    // Identity::trusted_admin

    #[test]
    fn trusted_admin_has_none_user_id_and_admin_role() {
        let id = Identity::trusted_admin();
        assert_eq!(id.user_id, None);
        assert_eq!(id.role, Role::Admin);
    }
}
