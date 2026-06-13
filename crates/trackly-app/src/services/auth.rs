//! `AuthService` — application service для управления пользователями и аутентификации.
//!
//! Обеспечивает:
//! - bootstrap-проверку (первый запуск без пользователей)
//! - login с argon2id верификацией
//! - CRUD пользователей с optimistic locking
//! - desktop_identity attribution (D-Desktop-01)
//! - desktop_lock_enabled r/w (D-Desktop-02)
//!
//! **Безопасность:** все argon2 операции (hash + verify) выполняются в
//! `spawn_blocking` (T-05-03 mitigate — CPU-bound, не блокируют async).

use std::sync::Arc;

use argon2::{
    Argon2, Algorithm, Version,
    password_hash::{
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
        rand_core::OsRng,
    },
    Params,
};
use tracing::warn;

use trackly_core::auth::{Action, Identity, Role, authorize};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_core::primitives::secret::Secret;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;

use crate::dto::auth::{
    ChangePasswordRequest, LoginRequest, UserDto, UserFilter, UserListResponse, UserNew, UserPatch,
};
use crate::dto::device::Pagination;

// ---------------------------------------------------------------------------
// Free functions (CPU-bound crypto — always spawn_blocking)
// ---------------------------------------------------------------------------

/// Хэширует пароль через argon2id (OWASP 2024+ params: m=19456 KiB, t=2, p=1).
///
/// Возвращает PHC-formatted string (includes salt + params).
pub fn hash_password(password: &Secret<String>) -> Result<String, AppError> {
    let params = Params::new(19456, 2, 1, None).map_err(|e| AppError::Internal {
        source_chain: format!("argon2 params: {e}"),
    })?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let salt = SaltString::generate(&mut OsRng);
    let hash = argon2
        .hash_password(password.expose().as_bytes(), &salt)
        .map_err(|e| AppError::Internal {
            source_chain: format!("argon2 hash: {e}"),
        })?;
    Ok(hash.to_string())
}

/// Верифицирует пароль против argon2 hash-строки.
///
/// Возвращает `true` если пароль совпадает.
pub fn verify_password(password: &Secret<String>, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        warn!("verify_password: не удалось распарсить hash");
        return false;
    };
    Argon2::default()
        .verify_password(password.expose().as_bytes(), &parsed)
        .is_ok()
}

// ---------------------------------------------------------------------------
// AuthService
// ---------------------------------------------------------------------------

/// Application service для аутентификации и управления пользователями.
///
/// `Arc`-wrapped fields → Clone O(1) (Tauri State + axum State).
#[derive(Clone)]
pub struct AuthService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
}

impl AuthService {
    /// Создать новый `AuthService`.
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self { writer, readers, clock }
    }

    // -----------------------------------------------------------------------
    // Bootstrap
    // -----------------------------------------------------------------------

    /// Проверяет, нужна ли начальная настройка (нет ни одного admin-пользователя).
    ///
    /// Возвращает `true` если в таблице `users` нет активных admin-пользователей.
    pub async fn needs_bootstrap(&self) -> Result<bool, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<bool, AppError> {
            let conn = readers.acquire();
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM users \
                     WHERE role = 'admin' AND deleted_at_utc IS NULL",
                    [],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;
            Ok(count == 0)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking needs_bootstrap: {e}"),
        })?
    }

    // -----------------------------------------------------------------------
    // Login / auth
    // -----------------------------------------------------------------------

    /// Получить хэш пароля для активного пользователя.
    async fn get_password_hash(&self, login: &str) -> Result<String, AppError> {
        let readers = self.readers.clone();
        let login = login.to_string();
        tokio::task::spawn_blocking(move || -> Result<String, AppError> {
            let conn = readers.acquire();
            let result: rusqlite::Result<String> = conn.query_row(
                "SELECT password_hash FROM users \
                 WHERE login = ?1 AND deleted_at_utc IS NULL AND is_active = 1",
                rusqlite::params![login],
                |r| r.get(0),
            );
            match result {
                Ok(h) => Ok(h),
                Err(rusqlite::Error::QueryReturnedNoRows) => Err(AppError::Unauthorized),
                Err(e) => Err(map_rusqlite(e)),
            }
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking get_password_hash: {e}"),
        })?
    }

    /// Аутентифицировать пользователя по логину и паролю.
    ///
    /// Верификация argon2 выполняется в `spawn_blocking` (T-05-03).
    pub async fn login(&self, req: LoginRequest) -> Result<UserDto, AppError> {
        let hash = self.get_password_hash(&req.login).await?;
        let password = Secret::new(req.password.clone());

        // CPU-bound verify — в spawn_blocking (T-05-03)
        let verified = tokio::task::spawn_blocking(move || verify_password(&password, &hash))
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("spawn_blocking verify_password: {e}"),
            })?;

        if !verified {
            return Err(AppError::Unauthorized);
        }

        self.get_by_login(&req.login).await
    }

    // -----------------------------------------------------------------------
    // User CRUD
    // -----------------------------------------------------------------------

    /// Создать нового пользователя.
    ///
    /// Требует права `ManageUsers`. Пароль хэшируется через argon2id в `spawn_blocking`.
    pub async fn create_user(&self, new: UserNew, caller: &Identity) -> Result<UserDto, AppError> {
        authorize(caller, &Action::ManageUsers)?;
        Self::validate_user_new(&new)?;

        let now = self.clock.unix_seconds();
        let caller_id = caller.user_id;
        let password = Secret::new(new.password.clone());

        // CPU-bound hash — в spawn_blocking (T-05-03)
        let hash = tokio::task::spawn_blocking(move || hash_password(&password))
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("spawn_blocking hash_password: {e}"),
            })??;

        let login = new.login.clone();
        let full_name = new.full_name.clone();
        let role = new.role.clone();
        let email = new.email.clone();

        let id = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                tx.execute(
                    "INSERT INTO users \
                     (login, full_name, password_hash, role, email, \
                      is_active, created_at_utc, updated_at_utc, version) \
                     VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6, 1)",
                    rusqlite::params![login, full_name, hash, role, email, now],
                )
                .map_err(map_rusqlite)?;

                let id = tx.last_insert_rowid();

                // Audit log
                tx.execute(
                    "INSERT INTO audit_log \
                     (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
                     VALUES ('user', ?1, 'create', ?2, NULL, NULL, ?3, ?4)",
                    rusqlite::params![id, caller_id, login, now],
                )
                .map_err(map_rusqlite)?;

                tx.commit().map_err(map_rusqlite)?;
                Ok(id)
            })
            .await?;

        self.get_user_by_id(id).await
    }

    /// Получить пользователя по ID.
    pub async fn get_user_by_id(&self, id: i64) -> Result<UserDto, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<UserDto, AppError> {
            let conn = readers.acquire();
            let result = conn.query_row(
                "SELECT id, version, login, full_name, role, email, is_active, \
                        created_at_utc, updated_at_utc \
                 FROM users \
                 WHERE id = ?1 AND deleted_at_utc IS NULL",
                rusqlite::params![id],
                row_to_user_dto,
            );
            match result {
                Ok(dto) => Ok(dto),
                Err(rusqlite::Error::QueryReturnedNoRows) => Err(AppError::NotFound {
                    entity: "user",
                    id,
                }),
                Err(e) => Err(map_rusqlite(e)),
            }
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking get_user_by_id: {e}"),
        })?
    }

    /// Получить пользователя по логину.
    pub async fn get_by_login(&self, login: &str) -> Result<UserDto, AppError> {
        let readers = self.readers.clone();
        let login = login.to_string();
        tokio::task::spawn_blocking(move || -> Result<UserDto, AppError> {
            let conn = readers.acquire();
            let result = conn.query_row(
                "SELECT id, version, login, full_name, role, email, is_active, \
                        created_at_utc, updated_at_utc \
                 FROM users \
                 WHERE login = ?1 AND deleted_at_utc IS NULL",
                rusqlite::params![login],
                row_to_user_dto,
            );
            match result {
                Ok(dto) => Ok(dto),
                Err(rusqlite::Error::QueryReturnedNoRows) => Err(AppError::Unauthorized),
                Err(e) => Err(map_rusqlite(e)),
            }
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking get_by_login: {e}"),
        })?
    }

    /// Список пользователей с опциональным фильтром поиска и пагинацией.
    pub async fn list_users(
        &self,
        filter: UserFilter,
        pagination: Pagination,
    ) -> Result<UserListResponse, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<UserListResponse, AppError> {
            let conn = readers.acquire();

            let (items, total) = if let Some(ref search) = filter.search {
                let pattern = format!("%{}%", search);
                let total: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM users \
                         WHERE deleted_at_utc IS NULL \
                           AND (login LIKE ?1 OR full_name LIKE ?1)",
                        rusqlite::params![pattern],
                        |r| r.get(0),
                    )
                    .map_err(map_rusqlite)?;

                let mut stmt = conn
                    .prepare(
                        "SELECT id, version, login, full_name, role, email, is_active, \
                                created_at_utc, updated_at_utc \
                         FROM users \
                         WHERE deleted_at_utc IS NULL \
                           AND (login LIKE ?1 OR full_name LIKE ?1) \
                         ORDER BY created_at_utc DESC \
                         LIMIT ?2 OFFSET ?3",
                    )
                    .map_err(map_rusqlite)?;

                let limit = pagination.limit as i64;
                let offset = pagination.offset as i64;
                let rows: Vec<UserDto> = stmt
                    .query_map(
                        rusqlite::params![pattern, limit, offset],
                        row_to_user_dto,
                    )
                    .map_err(map_rusqlite)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(map_rusqlite)?;

                (rows, total)
            } else {
                let total: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM users WHERE deleted_at_utc IS NULL",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(map_rusqlite)?;

                let mut stmt = conn
                    .prepare(
                        "SELECT id, version, login, full_name, role, email, is_active, \
                                created_at_utc, updated_at_utc \
                         FROM users \
                         WHERE deleted_at_utc IS NULL \
                         ORDER BY created_at_utc DESC \
                         LIMIT ?1 OFFSET ?2",
                    )
                    .map_err(map_rusqlite)?;

                let limit = pagination.limit as i64;
                let offset = pagination.offset as i64;
                let rows: Vec<UserDto> = stmt
                    .query_map(
                        rusqlite::params![limit, offset],
                        row_to_user_dto,
                    )
                    .map_err(map_rusqlite)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(map_rusqlite)?;

                (rows, total)
            };

            Ok(UserListResponse { items, total })
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking list_users: {e}"),
        })?
    }

    /// Обновить пользователя с optimistic-lock.
    pub async fn update_user(
        &self,
        id: i64,
        version: i64,
        patch: UserPatch,
        caller: &Identity,
    ) -> Result<UserDto, AppError> {
        authorize(caller, &Action::ManageUsers)?;

        if let Some(ref role_str) = patch.role {
            // Validate role string
            Role::from_str(role_str)?;
        }

        let now = self.clock.unix_seconds();
        let caller_id = caller.user_id;

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // Check version (optimistic lock)
                let current_version: i64 = tx
                    .query_row(
                        "SELECT version FROM users WHERE id = ?1 AND deleted_at_utc IS NULL",
                        rusqlite::params![id],
                        |r| r.get(0),
                    )
                    .map_err(|e| {
                        if e == rusqlite::Error::QueryReturnedNoRows {
                            AppError::NotFound {
                                entity: "user",
                                id,
                            }
                        } else {
                            map_rusqlite(e)
                        }
                    })?;

                if current_version != version {
                    return Err(AppError::Conflict {
                        reason: format!(
                            "optimistic lock: version {version} != current {current_version}"
                        ),
                    });
                }

                // Build partial update
                let mut sets = vec!["updated_at_utc = ?1", "version = version + 1"];
                let new_version_val = now; // placeholder reuse

                if patch.full_name.is_some() {
                    sets.push("full_name = ?3");
                }
                if patch.role.is_some() {
                    sets.push("role = ?4");
                }
                if patch.email.is_some() {
                    sets.push("email = ?5");
                }
                if patch.is_active.is_some() {
                    sets.push("is_active = ?6");
                }

                let _ = new_version_val; // suppress unused warning

                // Execute update directly with all fields (simpler than dynamic SQL)
                let rows_changed = tx
                    .execute(
                        "UPDATE users SET \
                         updated_at_utc = ?1, \
                         version = version + 1, \
                         full_name = COALESCE(?2, full_name), \
                         role = COALESCE(?3, role), \
                         email = CASE WHEN ?4 = 1 THEN ?5 ELSE email END, \
                         is_active = COALESCE(?6, is_active) \
                         WHERE id = ?7 AND version = ?8 AND deleted_at_utc IS NULL",
                        rusqlite::params![
                            now,
                            patch.full_name,
                            patch.role,
                            patch.email.is_some() as i64,
                            patch.email.flatten(),
                            patch.is_active.map(|b| b as i64),
                            id,
                            version
                        ],
                    )
                    .map_err(map_rusqlite)?;

                let _ = sets; // used above in comment

                if rows_changed == 0 {
                    return Err(AppError::Conflict {
                        reason: "optimistic lock: version mismatch".to_string(),
                    });
                }

                // Audit log
                tx.execute(
                    "INSERT INTO audit_log \
                     (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
                     VALUES ('user', ?1, 'update', ?2, NULL, NULL, NULL, ?3)",
                    rusqlite::params![id, caller_id, now],
                )
                .map_err(map_rusqlite)?;

                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await?;

        self.get_user_by_id(id).await
    }

    /// Мягкое удаление пользователя (soft-delete) с optimistic-lock.
    pub async fn delete_user(
        &self,
        id: i64,
        version: i64,
        caller: &Identity,
    ) -> Result<(), AppError> {
        authorize(caller, &Action::ManageUsers)?;

        let now = self.clock.unix_seconds();
        let caller_id = caller.user_id;

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                let rows_changed = tx
                    .execute(
                        "UPDATE users SET deleted_at_utc = ?1, version = version + 1 \
                         WHERE id = ?2 AND version = ?3 AND deleted_at_utc IS NULL",
                        rusqlite::params![now, id, version],
                    )
                    .map_err(map_rusqlite)?;

                if rows_changed == 0 {
                    return Err(AppError::Conflict {
                        reason: "optimistic lock: version mismatch or user not found".to_string(),
                    });
                }

                // Audit log
                tx.execute(
                    "INSERT INTO audit_log \
                     (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
                     VALUES ('user', ?1, 'delete', ?2, NULL, NULL, NULL, ?3)",
                    rusqlite::params![id, caller_id, now],
                )
                .map_err(map_rusqlite)?;

                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await
    }

    /// Сменить пароль пользователя (пользователь меняет себе).
    pub async fn change_password(
        &self,
        user_id: i64,
        req: ChangePasswordRequest,
    ) -> Result<(), AppError> {
        // Validate new password length
        if req.new_password.len() < 8 {
            return Err(AppError::Validation {
                field: "new_password".to_string(),
                message: "Пароль должен быть не менее 8 символов".to_string(),
            });
        }

        // Load current hash
        let readers = self.readers.clone();
        let uid = user_id;
        let current_hash = tokio::task::spawn_blocking(move || -> Result<String, AppError> {
            let conn = readers.acquire();
            let result: rusqlite::Result<String> = conn.query_row(
                "SELECT password_hash FROM users WHERE id = ?1 AND deleted_at_utc IS NULL",
                rusqlite::params![uid],
                |r| r.get(0),
            );
            match result {
                Ok(h) => Ok(h),
                Err(rusqlite::Error::QueryReturnedNoRows) => Err(AppError::NotFound {
                    entity: "user",
                    id: uid,
                }),
                Err(e) => Err(map_rusqlite(e)),
            }
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking change_password load: {e}"),
        })??;

        // Verify old password (T-05-03)
        let old_password = Secret::new(req.old_password.clone());
        let hash_clone = current_hash.clone();
        let verified = tokio::task::spawn_blocking(move || verify_password(&old_password, &hash_clone))
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("spawn_blocking verify old: {e}"),
            })?;

        if !verified {
            return Err(AppError::Unauthorized);
        }

        // Hash new password (T-05-03)
        let new_password = Secret::new(req.new_password.clone());
        let new_hash = tokio::task::spawn_blocking(move || hash_password(&new_password))
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("spawn_blocking hash new: {e}"),
            })??;

        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE users SET password_hash = ?1, updated_at_utc = ?2, version = version + 1 \
                     WHERE id = ?3 AND deleted_at_utc IS NULL",
                    rusqlite::params![new_hash, now, user_id],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
    }

    /// Сбросить пароль пользователя (admin-операция).
    pub async fn reset_password(
        &self,
        user_id: i64,
        new_password: String,
        caller: &Identity,
    ) -> Result<(), AppError> {
        authorize(caller, &Action::ManageUsers)?;

        if new_password.len() < 8 {
            return Err(AppError::Validation {
                field: "new_password".to_string(),
                message: "Пароль должен быть не менее 8 символов".to_string(),
            });
        }

        let password = Secret::new(new_password);
        let new_hash = tokio::task::spawn_blocking(move || hash_password(&password))
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("spawn_blocking reset hash: {e}"),
            })??;

        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE users SET password_hash = ?1, updated_at_utc = ?2, version = version + 1 \
                     WHERE id = ?3 AND deleted_at_utc IS NULL",
                    rusqlite::params![new_hash, now, user_id],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
    }

    // -----------------------------------------------------------------------
    // Desktop attribution (D-Desktop-01)
    // -----------------------------------------------------------------------

    /// Возвращает идентификатор для десктоп-режима без входа.
    ///
    /// **D-Desktop-01:** если в БД ровно один активный admin — атрибутирует
    /// его (`user_id = Some(id)`). При 0 или 2+ admin'ах — `trusted_admin()`
    /// (user_id = None). Используется LIMIT 2 для эффективности.
    pub async fn desktop_identity(&self) -> Identity {
        let readers = self.readers.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<Vec<i64>, AppError> {
            let conn = readers.acquire();
            let mut stmt = conn
                .prepare(
                    "SELECT id FROM users \
                     WHERE role = 'admin' AND deleted_at_utc IS NULL AND is_active = 1 \
                     LIMIT 2",
                )
                .map_err(map_rusqlite)?;
            let ids: Vec<i64> = stmt
                .query_map([], |r| r.get(0))
                .map_err(map_rusqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_rusqlite)?;
            Ok(ids)
        })
        .await;

        match result {
            Ok(Ok(ids)) if ids.len() == 1 => Identity {
                user_id: Some(ids[0]),
                role: Role::Admin,
            },
            _ => Identity::trusted_admin(),
        }
    }

    // -----------------------------------------------------------------------
    // Desktop lock (D-Desktop-02)
    // -----------------------------------------------------------------------

    /// Читает флаг `desktop_lock_enabled` из таблицы `app_settings`.
    ///
    /// '1' → true, любое другое значение или отсутствие записи → false.
    pub async fn get_desktop_lock_enabled(&self) -> Result<bool, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<bool, AppError> {
            let conn = readers.acquire();
            let result: rusqlite::Result<String> = conn.query_row(
                "SELECT value FROM app_settings WHERE key = 'desktop_lock_enabled'",
                [],
                |r| r.get(0),
            );
            match result {
                Ok(v) => Ok(v == "1"),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
                Err(e) => Err(map_rusqlite(e)),
            }
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking get_desktop_lock_enabled: {e}"),
        })?
    }

    /// Устанавливает флаг `desktop_lock_enabled` в таблице `app_settings`.
    ///
    /// Требует права `ManageSettings` (D-Desktop-02).
    pub async fn set_desktop_lock_enabled(
        &self,
        enabled: bool,
        caller: &Identity,
    ) -> Result<(), AppError> {
        authorize(caller, &Action::ManageSettings)?;

        let value = if enabled { "1" } else { "0" };
        let now = self.clock.unix_seconds();

        self.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE app_settings SET value = ?1, updated_at_utc = ?2 \
                     WHERE key = 'desktop_lock_enabled'",
                    rusqlite::params![value, now],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
    }

    // -----------------------------------------------------------------------
    // Validation helpers
    // -----------------------------------------------------------------------

    fn validate_user_new(new: &UserNew) -> Result<(), AppError> {
        if new.login.len() < 3 {
            return Err(AppError::Validation {
                field: "login".to_string(),
                message: "Логин должен быть не менее 3 символов".to_string(),
            });
        }
        if new.password.len() < 8 {
            return Err(AppError::Validation {
                field: "password".to_string(),
                message: "Пароль должен быть не менее 8 символов".to_string(),
            });
        }
        // Validate role
        Role::from_str(&new.role)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Row mapper
// ---------------------------------------------------------------------------

fn row_to_user_dto(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserDto> {
    let is_active_i64: i64 = row.get(6)?;
    Ok(UserDto {
        id: row.get(0)?,
        version: row.get(1)?,
        login: row.get(2)?,
        full_name: row.get(3)?,
        role: row.get(4)?,
        email: row.get(5)?,
        is_active: is_active_i64 != 0,
        created_at_utc: row.get(7)?,
        updated_at_utc: row.get(8)?,
    })
}
