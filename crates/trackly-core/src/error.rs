//! `AppError` — типизированная ошибка приложения (D-AppError-01).
//!
//! Plan 04: полный набор из 9 вариантов с единым JSON-shape:
//! `{code: "SCREAMING_SNAKE", message: String, details: Value}`.
//!
//! Конверсии `From<rusqlite::Error>`, `From<refinery::Error>`,
//! `From<tokio::sync::mpsc::error::SendTimeoutError<T>>` и
//! `From<tokio::sync::oneshot::error::RecvError>` живут в `trackly-infra`
//! (`error_conversions.rs`) — `trackly-core` не имеет I/O-зависимостей.
//!
//! `axum::IntoResponse` для AppError — Plan 05 (`trackly-app/error_axum.rs`).
//!
//! `specta::Type` для tauri-specta — Plan 05. Реализовано как **manual impl**
//! (не derive), потому что:
//!
//! - Сериализация уже ручная (manual `impl Serialize` выше), и shape `{code,
//!   message, details}` НЕ соответствует тому, что специальный derive выдал бы
//!   для tagged enum с 9 вариантами и разнотипными payload'ами.
//! - `details` — это `serde_json::Value` (произвольная форма), что
//!   `specta::Type` derive не умеет вывести из enum-варианта.
//!
//! Manual impl даёт type `{code: string, message: string, details: any}` —
//! ровно та форма, которую видит фронтенд в bindings.ts.

use serde::{Serialize, Serializer};
use serde_json::{json, Value};

/// Главный тип ошибки приложения. См. D-AppError-01.
///
/// `specta::Type` реализован **вручную** ниже (а не derive) — у нас уже manual
/// `impl Serialize` с формой `{code: string, message: string, details: any}`,
/// и frontend bindings должны отражать ровно её, а не Rust-enum tagged shape.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Сущность не найдена.
    #[error("{entity} {id} not found")]
    NotFound {
        /// Имя сущности (`"device"`, `"act"`, …).
        entity: &'static str,
        /// Идентификатор.
        id: i64,
    },

    /// Конфликт (например, нарушение unique constraint).
    #[error("conflict: {reason}")]
    Conflict {
        /// Причина конфликта (человекочитаемая).
        reason: String,
    },

    /// Оптимистическая блокировка: версия в БД не совпадает с ожидаемой.
    #[error("optimistic lock mismatch on {entity} {id}: expected v{expected}, found v{actual}")]
    OptimisticLockMismatch {
        /// Имя сущности.
        entity: &'static str,
        /// Идентификатор.
        id: i64,
        /// Версия, которую ожидал клиент.
        expected: i64,
        /// Версия, которая фактически в БД.
        actual: i64,
    },

    /// Очередь записей переполнена (mpsc send_timeout сработал).
    #[error("write queue busy (5s timeout)")]
    WriteQueueBusy,

    /// Файл БД создан более новой версией бинаря, чем текущая.
    #[error("database from newer version: binary={binary}, file={file}")]
    DatabaseFromNewerVersion {
        /// `user_version`, который знает текущий бинарь.
        binary: u32,
        /// `user_version`, фактически записанный в файле.
        file: u32,
    },

    /// Валидация входных данных (TOML, путь, поле формы).
    #[error("validation [{field}]: {message}")]
    Validation {
        /// Имя поля или источника, который не прошёл валидацию.
        field: String,
        /// Сообщение, пригодное к показу администратору.
        message: String,
    },

    /// Запрос без аутентификации.
    #[error("unauthorized")]
    Unauthorized,

    /// Аутентификация есть, но прав не хватает.
    #[error("forbidden")]
    Forbidden,

    /// Внутренняя ошибка (I/O, рантайм, неожиданное состояние).
    #[error("internal: {source_chain}")]
    Internal {
        /// Цепочка причин (обычно отформатированный source error).
        source_chain: String,
    },

    /// Внешний сервис недоступен (например, AD/LDAP сервер не отвечает).
    ///
    /// Distinct from `Unauthorized` (Phase 9, AD login fallback): an
    /// unreachable AD server is an infra fault, not a bad-credentials
    /// outcome — the UI must show "AD недоступен", not "неверный логин
    /// или пароль" (no enumeration leak either way, see `AuthOutcome`).
    #[error("service unavailable: {service}")]
    ServiceUnavailable {
        /// Имя недоступного сервиса (например, `"ad"`).
        service: &'static str,
    },

    /// AD bind успешен, но учётная запись ещё не создана/активирована —
    /// заявка `ad_register` создана и ждёт решения администратора
    /// (Phase 9 Plan 03, USR-09/USR-11/SET-10 "pending" mode).
    ///
    /// Не `Unauthorized` — UI должен показать `PendingScreen`, а не форму
    /// логина с ошибкой (D-REG-01).
    #[error("registration pending admin approval (request {request_id})")]
    RegistrationPending {
        /// ID созданной заявки `ad_register`.
        request_id: i64,
    },

    /// AD bind успешен, но локальная учётная запись блокирована
    /// (`is_active=0`) или soft-deleted (D-REG-03 "blocked" mode).
    ///
    /// Не `Unauthorized` — UI должен показать `BlockedScreen`, а не форму
    /// логина с ошибкой.
    ///
    /// **09-AD-GAPS restoration-flow UX:** начиная с gap-closure plan plain
    /// login НЕ создаёт заявку восстановления (read-only) — он только
    /// сообщает состояние МОСТ-РЕЦЕНТНОЙ заявки восстановления (если она
    /// есть), чтобы UI мог показать одно из трёх состояний:
    /// - нет заявки вообще → `pending=false`, `rejection_reason=None`.
    /// - есть открытая заявка → `pending=true`, `rejection_reason=None`.
    /// - последняя заявка отклонена → `pending=false`,
    ///   `rejection_reason=Some(причина)`.
    ///
    /// Явное создание новой заявки — отдельный сервисный метод
    /// `AuthService::request_ad_restore` (EXPLICIT re-request action).
    #[error("account blocked (pending={pending}, rejection_reason={rejection_reason:?})")]
    AccessBlocked {
        /// `true`, если для пользователя уже существует ОТКРЫТАЯ заявка
        /// восстановления (никакого нового запроса создавать не нужно).
        pending: bool,
        /// Причина отклонения последней заявки восстановления, если
        /// последняя (по времени) заявка была отклонена и сейчас нет
        /// открытой заявки. `None`, если заявок не было вообще или
        /// последняя заявка была одобрена (что не должно приводить
        /// пользователя на эту ветку, но защищаемся явно).
        rejection_reason: Option<String>,
    },
}

impl AppError {
    /// Стабильный SCREAMING_SNAKE_CASE код для фронтенда.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "NOT_FOUND",
            Self::Conflict { .. } => "CONFLICT",
            Self::OptimisticLockMismatch { .. } => "OPTIMISTIC_LOCK_MISMATCH",
            Self::WriteQueueBusy => "WRITE_QUEUE_BUSY",
            Self::DatabaseFromNewerVersion { .. } => "DATABASE_FROM_NEWER_VERSION",
            Self::Validation { .. } => "VALIDATION",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::Internal { .. } => "INTERNAL",
            Self::ServiceUnavailable { .. } => "SERVICE_UNAVAILABLE",
            Self::RegistrationPending { .. } => "REGISTRATION_PENDING",
            Self::AccessBlocked { .. } => "ACCESS_BLOCKED",
        }
    }

    /// Варианто-специфичные поля для JSON-shape `details`.
    fn details_value(&self) -> Value {
        match self {
            Self::NotFound { entity, id } => json!({ "entity": entity, "id": id }),
            Self::Conflict { reason } => json!({ "reason": reason }),
            Self::OptimisticLockMismatch {
                entity,
                id,
                expected,
                actual,
            } => json!({
                "entity": entity,
                "id": id,
                "expected": expected,
                "actual": actual,
            }),
            Self::WriteQueueBusy => json!({}),
            Self::DatabaseFromNewerVersion { binary, file } => {
                json!({ "binary": binary, "file": file })
            }
            Self::Validation { field, message } => {
                json!({ "field": field, "message": message })
            }
            Self::Unauthorized => json!({}),
            Self::Forbidden => json!({}),
            Self::Internal { source_chain } => json!({ "source_chain": source_chain }),
            Self::ServiceUnavailable { service } => json!({ "service": service }),
            Self::RegistrationPending { request_id } => json!({ "request_id": request_id }),
            Self::AccessBlocked {
                pending,
                rejection_reason,
            } => json!({ "pending": pending, "rejection_reason": rejection_reason }),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = ser.serialize_struct("AppError", 3)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
        s.serialize_field("details", &self.details_value())?;
        s.end()
    }
}

/// Sibling marker-структура: ровно та форма, которую `AppError::serialize`
/// выдаёт на проводе. Используется для генерации `bindings.ts` через
/// `specta::Type` derive, чтобы frontend-тип `AppError` соответствовал
/// runtime JSON. См. `impl specta::Type for AppError` ниже — он делегирует
/// `AppErrorRepr::inline` / `AppErrorRepr::reference`.
///
/// Поле `details` объявлено как `serde_json::Value` (= specta `any`) —
/// варианто-специфичные shapes (`{entity, id}`, `{reason}`, ...) задокументированы
/// в коде, но не выражаются в bindings (фронт получает discriminated union
/// через поле `code`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename = "AppError")]
pub struct AppErrorRepr {
    pub code: String,
    pub message: String,
    pub details: serde_json::Value,
}

impl specta::Type for AppError {
    fn inline(
        type_map: &mut specta::TypeCollection,
        generics: specta::Generics,
    ) -> specta::datatype::DataType {
        <AppErrorRepr as specta::Type>::inline(type_map, generics)
    }

    fn reference(
        type_map: &mut specta::TypeCollection,
        generics: &[specta::datatype::DataType],
    ) -> specta::datatype::reference::Reference {
        <AppErrorRepr as specta::Type>::reference(type_map, generics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn ser(e: &AppError) -> Value {
        serde_json::from_str(&serde_json::to_string(e).expect("serialize")).expect("parse")
    }

    #[test]
    fn code_for_all_variants() {
        assert_eq!(
            AppError::NotFound {
                entity: "device",
                id: 1
            }
            .code(),
            "NOT_FOUND"
        );
        assert_eq!(
            AppError::Conflict {
                reason: "dup".into()
            }
            .code(),
            "CONFLICT"
        );
        assert_eq!(
            AppError::OptimisticLockMismatch {
                entity: "device",
                id: 42,
                expected: 1,
                actual: 2,
            }
            .code(),
            "OPTIMISTIC_LOCK_MISMATCH"
        );
        assert_eq!(AppError::WriteQueueBusy.code(), "WRITE_QUEUE_BUSY");
        assert_eq!(
            AppError::DatabaseFromNewerVersion {
                binary: 12,
                file: 99
            }
            .code(),
            "DATABASE_FROM_NEWER_VERSION"
        );
        assert_eq!(
            AppError::Validation {
                field: "f".into(),
                message: "m".into()
            }
            .code(),
            "VALIDATION"
        );
        assert_eq!(AppError::Unauthorized.code(), "UNAUTHORIZED");
        assert_eq!(AppError::Forbidden.code(), "FORBIDDEN");
        assert_eq!(
            AppError::Internal {
                source_chain: "x".into()
            }
            .code(),
            "INTERNAL"
        );
        assert_eq!(
            AppError::ServiceUnavailable { service: "ad" }.code(),
            "SERVICE_UNAVAILABLE"
        );
        assert_eq!(
            AppError::RegistrationPending { request_id: 5 }.code(),
            "REGISTRATION_PENDING"
        );
        assert_eq!(
            AppError::AccessBlocked {
                pending: true,
                rejection_reason: None
            }
            .code(),
            "ACCESS_BLOCKED"
        );
    }

    #[test]
    fn serialize_shape_has_code_message_details() {
        let v = ser(&AppError::NotFound {
            entity: "device",
            id: 42,
        });
        assert_eq!(v["code"], "NOT_FOUND");
        assert_eq!(v["message"], "device 42 not found");
        assert_eq!(v["details"]["entity"], "device");
        assert_eq!(v["details"]["id"], 42);
    }

    #[test]
    fn serialize_optimistic_lock_mismatch_details() {
        let v = ser(&AppError::OptimisticLockMismatch {
            entity: "act",
            id: 7,
            expected: 1,
            actual: 2,
        });
        assert_eq!(v["code"], "OPTIMISTIC_LOCK_MISMATCH");
        assert_eq!(v["details"]["entity"], "act");
        assert_eq!(v["details"]["id"], 7);
        assert_eq!(v["details"]["expected"], 1);
        assert_eq!(v["details"]["actual"], 2);
    }

    #[test]
    fn serialize_write_queue_busy_empty_details() {
        let v = ser(&AppError::WriteQueueBusy);
        assert_eq!(v["code"], "WRITE_QUEUE_BUSY");
        assert!(v["details"].is_object());
        assert_eq!(v["details"].as_object().expect("obj").len(), 0);
    }

    #[test]
    fn serialize_database_from_newer_version_details() {
        let v = ser(&AppError::DatabaseFromNewerVersion {
            binary: 12,
            file: 999,
        });
        assert_eq!(v["code"], "DATABASE_FROM_NEWER_VERSION");
        assert_eq!(v["details"]["binary"], 12);
        assert_eq!(v["details"]["file"], 999);
    }

    #[test]
    fn serialize_validation_details() {
        let v = ser(&AppError::Validation {
            field: "host".into(),
            message: "empty".into(),
        });
        assert_eq!(v["code"], "VALIDATION");
        assert_eq!(v["details"]["field"], "host");
        assert_eq!(v["details"]["message"], "empty");
    }

    #[test]
    fn serialize_unauthorized_forbidden_empty_details() {
        let u = ser(&AppError::Unauthorized);
        assert_eq!(u["code"], "UNAUTHORIZED");
        assert_eq!(u["details"].as_object().expect("obj").len(), 0);

        let f = ser(&AppError::Forbidden);
        assert_eq!(f["code"], "FORBIDDEN");
        assert_eq!(f["details"].as_object().expect("obj").len(), 0);
    }

    #[test]
    fn serialize_internal_details() {
        let v = ser(&AppError::Internal {
            source_chain: "oops".into(),
        });
        assert_eq!(v["code"], "INTERNAL");
        assert_eq!(v["details"]["source_chain"], "oops");
    }

    #[test]
    fn serialize_conflict_details() {
        let v = ser(&AppError::Conflict {
            reason: "unique violation".into(),
        });
        assert_eq!(v["code"], "CONFLICT");
        assert_eq!(v["details"]["reason"], "unique violation");
    }

    #[test]
    fn serialize_service_unavailable_details() {
        let v = ser(&AppError::ServiceUnavailable { service: "ad" });
        assert_eq!(v["code"], "SERVICE_UNAVAILABLE");
        assert_eq!(v["details"]["service"], "ad");
    }

    #[test]
    fn serialize_registration_pending_details() {
        let v = ser(&AppError::RegistrationPending { request_id: 5 });
        assert_eq!(v["code"], "REGISTRATION_PENDING");
        assert_eq!(v["details"]["request_id"], 5);
    }

    #[test]
    fn serialize_access_blocked_pending_details() {
        let v = ser(&AppError::AccessBlocked {
            pending: true,
            rejection_reason: None,
        });
        assert_eq!(v["code"], "ACCESS_BLOCKED");
        assert_eq!(v["details"]["pending"], true);
        assert!(v["details"]["rejection_reason"].is_null());
    }

    #[test]
    fn serialize_access_blocked_rejected_details() {
        let v = ser(&AppError::AccessBlocked {
            pending: false,
            rejection_reason: Some("дубликат заявки".to_string()),
        });
        assert_eq!(v["code"], "ACCESS_BLOCKED");
        assert_eq!(v["details"]["pending"], false);
        assert_eq!(v["details"]["rejection_reason"], "дубликат заявки");
    }

    #[test]
    fn serialize_access_blocked_none_details() {
        let v = ser(&AppError::AccessBlocked {
            pending: false,
            rejection_reason: None,
        });
        assert_eq!(v["code"], "ACCESS_BLOCKED");
        assert_eq!(v["details"]["pending"], false);
        assert!(v["details"]["rejection_reason"].is_null());
    }
}
