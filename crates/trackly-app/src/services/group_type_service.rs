//! `GroupTypeService` — типы групп и их свойства (Phase 41, GRP-01..03, GRP-09).
//!
//! Бизнес-правила живут здесь, чтобы оба транспорта (Tauri invoke и HTTP) были
//! тонкими адаптерами: неизменяемость `code`/`behavior`, защита встроенных типов
//! и типов с группами, одноразовый засев свойств по умолчанию.
//!
//! Авторизация — ПЕРВОЙ строкой каждого метода: мутации — `ManageGroupTypes`
//! (только admin), чтения — `ReadGroups` (admin|manager). Чтения идут через
//! пул читателей (`spawn_blocking`), мутации — через единственный writer.
//! Репозиторий (`SqliteGroupTypeRepository`) — единственный слой SQL.
//! Мутация и её строка `audit_log` — одна транзакция writer (идиом
//! `GroupService::delete_group`): сбой аудита откатывает изменение (W-B02).

use std::sync::Arc;

use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::domain::groups::{
    validate_name, GroupBehavior, GroupTypeNew, GroupTypePatch, GroupTypeRow, PropertyDataType,
    PropertyNew, PropertyPatch, PropertyRow, NAME_MAX_CHARS,
};
use trackly_core::error::AppError;
use trackly_core::ports::group_types::GroupTypeRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::audit_log_sqlite::{AuditEntry, SqliteAuditLogRepository};
use trackly_infra::repos::group_types_sqlite::DefaultProperty;
use trackly_infra::repos::SqliteGroupTypeRepository;

use crate::dto::group_types::{
    GroupRefDto, GroupTypeCreateDto, GroupTypeDto, GroupTypePropertyDto, GroupTypeUpdateDto,
    PropertyCreateDto, PropertyDeleteOutcomeDto, PropertyUpdateDto,
};

/// Свойства по умолчанию встроенных типов (D-31), `(code, свойства)`.
///
/// У АРМ — пять необязательных свойств в порядке `sort_order = индекс`;
/// «На карте» включён только у «Пользователи». У «Системного блока» и «Разбора»
/// набор в SPEC/UI-SPEC не назван и не додумывается: пустой список, маркер
/// `default_props_seeded` остаётся 0, чтобы будущая фаза могла определить свойства.
const BUILTIN_DEFAULT_PROPERTIES: &[(&str, &[DefaultProperty])] = &[
    (
        "workstation",
        &[
            DefaultProperty {
                name: "Пользователи",
                data_type: PropertyDataType::Users,
                show_on_map: true,
            },
            DefaultProperty {
                name: "Подключённые принтеры",
                data_type: PropertyDataType::DeviceRefs,
                show_on_map: false,
            },
            DefaultProperty {
                name: "Хост",
                data_type: PropertyDataType::Text,
                show_on_map: false,
            },
            DefaultProperty {
                name: "IP",
                data_type: PropertyDataType::Ip,
                show_on_map: false,
            },
            DefaultProperty {
                name: "MAC",
                data_type: PropertyDataType::Mac,
                show_on_map: false,
            },
        ],
    ),
    ("system_unit", &[]),
    ("teardown", &[]),
];

/// Склонение «N группа/группы/групп».
pub(crate) fn plural_groups(n: i64) -> String {
    let m100 = n % 100;
    let m10 = n % 10;
    let word = if (11..=14).contains(&m100) {
        "групп"
    } else if m10 == 1 {
        "группа"
    } else if (2..=4).contains(&m10) {
        "группы"
    } else {
        "групп"
    };
    format!("{n} {word}")
}

/// Потолок числа свойств одного типа (скрытые учитываются) — защита от раздувания схемы.
const MAX_PROPERTIES_PER_TYPE: usize = 50;

/// «в N группах» (предложный падеж) для сообщения о заполненном свойстве.
fn in_groups(n: i64) -> String {
    if n % 10 == 1 && n % 100 != 11 {
        format!("{n} группе")
    } else {
        format!("{n} группах")
    }
}

fn duplicate_property_name_error() -> AppError {
    AppError::Validation {
        field: "name".to_string(),
        message: "Свойство с таким названием уже есть.".to_string(),
    }
}

/// Скрытое свойство из форм групп выпадает: обязательным его сделать нельзя (W-B01).
fn hidden_property_required_error() -> AppError {
    AppError::Validation {
        field: "is_required".to_string(),
        message: "Скрытое свойство нельзя сделать обязательным. \
                  Сначала верните его в формы групп."
            .to_string(),
    }
}

/// Перехват нарушения `idx_gtp_name_live` (сырой `Conflict` от SQLite).
fn is_duplicate_property_conflict(err: &AppError) -> bool {
    matches!(err, AppError::Conflict { reason } if reason.contains("idx_gtp_name_live")
        || reason.contains("group_type_properties.type_id, group_type_properties.name"))
}

/// Нарушители обязательности: до 5 имён, остальное «и ещё N».
fn required_violation_error(violators: &[(i64, String)]) -> AppError {
    let shown: Vec<String> = violators
        .iter()
        .take(5)
        .map(|(_, n)| format!("«{n}»"))
        .collect();
    let mut list = shown.join(", ");
    if violators.len() > 5 {
        list.push_str(&format!(" и ещё {}", violators.len() - 5));
    }
    AppError::Validation {
        field: "is_required".to_string(),
        message: format!(
            "Нельзя сделать свойство обязательным: у групп нет значения: {list}. \
             Заполните его в этих группах."
        ),
    }
}

/// Живое свойство типа с таким именем (без учёта регистра, кириллица тоже),
/// кроме `except_id`. SQLite `lower()` кириллицу не сворачивает — сравнение в Rust.
fn live_name_taken(
    conn: &rusqlite::Connection,
    repo: &SqliteGroupTypeRepository,
    type_id: i64,
    name: &str,
    except_id: Option<i64>,
) -> Result<bool, AppError> {
    let needle = name.to_lowercase();
    Ok(repo
        .list_properties(conn, type_id, false)?
        .iter()
        .any(|p| Some(p.id) != except_id && p.name.to_lowercase() == needle))
}

fn property_dto(
    conn: &rusqlite::Connection,
    repo: &SqliteGroupTypeRepository,
    row: PropertyRow,
) -> Result<GroupTypePropertyDto, AppError> {
    let filled = repo.filled_group_count(conn, row.id)?;
    Ok(GroupTypePropertyDto::from_row(row, filled))
}

/// Сервис типов групп. `Arc`-поля делают `Clone` дешёвым.
#[derive(Clone)]
pub struct GroupTypeService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) repo: Arc<SqliteGroupTypeRepository>,
    pub(crate) audit_repo: Arc<SqliteAuditLogRepository>,
}

/// Собрать DTO типа (свойства + счётчики) на переданном соединении.
fn build_type_dto(
    conn: &rusqlite::Connection,
    repo: &SqliteGroupTypeRepository,
    row: GroupTypeRow,
    include_archived: bool,
) -> Result<GroupTypeDto, AppError> {
    let group_count = repo.count_groups_of_type(conn, row.id)?;
    let mut props = Vec::new();
    for p in repo.list_properties(conn, row.id, include_archived)? {
        let filled = repo.filled_group_count(conn, p.id)?;
        props.push(GroupTypePropertyDto::from_row(p, filled));
    }
    Ok(GroupTypeDto::from_row(row, group_count, props))
}

fn to_json<T: serde::Serialize>(v: &T) -> Result<String, AppError> {
    serde_json::to_string(v).map_err(|e| AppError::Internal {
        source_chain: format!("audit_log json: {e}"),
    })
}

impl GroupTypeService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            repo: Arc::new(SqliteGroupTypeRepository),
            audit_repo: Arc::new(SqliteAuditLogRepository),
        }
    }

    // ---- чтения --------------------------------------------------------

    /// Все типы со свойствами; скрытые свойства — только при `include_archived`.
    pub async fn list_types(
        &self,
        caller: &Identity,
        include_archived: bool,
    ) -> Result<Vec<GroupTypeDto>, AppError> {
        authorize(caller, &Action::ReadGroups)?;
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let mut out = Vec::new();
            for row in repo.list_types(&conn)? {
                out.push(build_type_dto(&conn, &repo, row, include_archived)?);
            }
            Ok::<_, AppError>(out)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    // ---- мутации типов -------------------------------------------------

    /// Создать пользовательский тип. `code` генерируется (`custom_<8 hex>`), не из имени.
    pub async fn create_type(
        &self,
        caller: &Identity,
        dto: GroupTypeCreateDto,
    ) -> Result<GroupTypeDto, AppError> {
        authorize(caller, &Action::ManageGroupTypes)?;
        let name = validate_name(&dto.name, "name")?;
        let behavior = GroupBehavior::from_str(&dto.behavior)?;
        let code = format!("custom_{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let next_sort = repo
                    .list_types(&tx)?
                    .iter()
                    .map(|t| t.sort_order)
                    .max()
                    .map_or(0, |m| m + 1);
                let id = repo.create_type_on(
                    &tx,
                    &GroupTypeNew {
                        code,
                        name,
                        behavior,
                        is_builtin: false,
                        sort_order: next_sort,
                        quick_action_enabled: false,
                        quick_action_label: None,
                    },
                    now,
                )?;
                let out = build_type_dto(&tx, &repo, repo.get_type(&tx, id)?, true)?;
                let after = to_json(&out)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "group_type",
                        entity_id: id,
                        action: "create",
                        user_id,
                        before_json: None,
                        after_json: Some(after),
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(out)
            })
            .await
    }

    /// Обновить тип (CAS по `version`). `code`/`behavior` в DTO допускаются,
    /// только если совпадают с текущими (Pitfall 10).
    pub async fn update_type(
        &self,
        caller: &Identity,
        id: i64,
        version: i64,
        dto: GroupTypeUpdateDto,
    ) -> Result<GroupTypeDto, AppError> {
        authorize(caller, &Action::ManageGroupTypes)?;
        let name = dto
            .name
            .as_deref()
            .map(|n| validate_name(n, "name"))
            .transpose()?;
        let label = match &dto.quick_action_label {
            None => None,
            Some(None) => Some(None),
            Some(Some(s)) => {
                let t = s.trim();
                if t.chars().count() > NAME_MAX_CHARS {
                    return Err(AppError::Validation {
                        field: "quick_action_label".to_string(),
                        message: format!(
                            "Подпись слишком длинная: не более {NAME_MAX_CHARS} символов."
                        ),
                    });
                }
                Some(if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                })
            }
        };
        let patch = GroupTypePatch {
            name,
            sort_order: dto.sort_order,
            quick_action_enabled: dto.quick_action_enabled,
            quick_action_label: label,
        };

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();
        let want_code = dto.code;
        let want_behavior = dto.behavior;

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let current = repo.get_type(&tx, id)?;
                let before = to_json(&build_type_dto(&tx, &repo, current.clone(), true)?)?;
                if want_code.as_deref().is_some_and(|c| c != current.code) {
                    return Err(Self::immutable_error("code"));
                }
                if want_behavior
                    .as_deref()
                    .is_some_and(|b| b != current.behavior)
                {
                    return Err(Self::immutable_error("behavior"));
                }
                let row = repo.update_type_on(&tx, id, version, &patch, now)?;
                let out = build_type_dto(&tx, &repo, row, true)?;
                let after = to_json(&out)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "group_type",
                        entity_id: id,
                        action: "update",
                        user_id,
                        before_json: Some(before),
                        after_json: Some(after),
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?; // W-B02:update_type:commit
                Ok(out)
            })
            .await
    }

    fn immutable_error(field: &str) -> AppError {
        AppError::Validation {
            field: field.to_string(),
            message: "Код и поведение типа после создания не меняются.".to_string(),
        }
    }

    /// Удалить тип: встроенный и тип с группами удалить нельзя.
    pub async fn delete_type(&self, caller: &Identity, id: i64) -> Result<(), AppError> {
        authorize(caller, &Action::ManageGroupTypes)?;
        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let current = repo.get_type(&tx, id)?;
                if current.is_builtin {
                    return Err(AppError::Validation {
                        field: "id".to_string(),
                        message: "Встроенный тип удалить нельзя. Его можно переименовать \
                                  и дополнить свойствами."
                            .to_string(),
                    });
                }
                let groups = repo.count_groups_of_type(&tx, id)?;
                if groups > 0 {
                    return Err(AppError::Validation {
                        field: "id".to_string(),
                        message: format!(
                            "Тип нельзя удалить: у него {}. Удалите или перенесите их.",
                            plural_groups(groups)
                        ),
                    });
                }
                let before = to_json(&build_type_dto(&tx, &repo, current, true)?)?;
                repo.delete_type_on(&tx, id)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "group_type",
                        entity_id: id,
                        action: "delete",
                        user_id,
                        before_json: Some(before),
                        after_json: None,
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await
    }

    // ---- свойства типа -------------------------------------------------

    /// Создать свойство в конце списка типа.
    pub async fn create_property(
        &self,
        caller: &Identity,
        dto: PropertyCreateDto,
    ) -> Result<GroupTypePropertyDto, AppError> {
        authorize(caller, &Action::ManageGroupTypes)?;
        let name = validate_name(&dto.name, "name")?;
        let data_type = PropertyDataType::from_str(&dto.data_type)?;

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();
        let type_id = dto.type_id;
        let want_required = dto.is_required;
        let show_on_map = dto.show_on_map;

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                repo.get_type(&tx, type_id)?;
                if repo.list_properties(&tx, type_id, true)?.len() >= MAX_PROPERTIES_PER_TYPE {
                    return Err(AppError::Validation {
                        field: "type_id".to_string(),
                        message: format!(
                            "У типа не может быть больше {MAX_PROPERTIES_PER_TYPE} свойств."
                        ),
                    });
                }
                if live_name_taken(&tx, &repo, type_id, &name, None)? {
                    return Err(duplicate_property_name_error());
                }
                // Создаём необязательным: «обязательное» проверяется по уже
                // существующим группам типа тем же правилом, что и при правке.
                let id = match repo.create_property_on(
                    &tx,
                    &PropertyNew {
                        type_id,
                        name,
                        data_type,
                        is_required: false,
                        show_on_map,
                    },
                    now,
                ) {
                    Ok(id) => id,
                    Err(e) if is_duplicate_property_conflict(&e) => {
                        return Err(duplicate_property_name_error());
                    }
                    Err(e) => return Err(e),
                };
                if want_required {
                    let violators = repo.groups_missing_required(&tx, type_id, id)?;
                    if !violators.is_empty() {
                        return Err(required_violation_error(&violators));
                    }
                    repo.update_property_on(
                        &tx,
                        id,
                        1,
                        &PropertyPatch {
                            is_required: Some(true),
                            ..Default::default()
                        },
                        now,
                    )?;
                }
                let out = property_dto(&tx, &repo, repo.get_property(&tx, id)?)?;
                let after = to_json(&out)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "group_type_property",
                        entity_id: id,
                        action: "create",
                        user_id,
                        before_json: None,
                        after_json: Some(after),
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(out)
            })
            .await
    }

    /// Обновить свойство (CAS по `version`). Смена типа данных заполненного
    /// свойства и включение «обязательное» при пустых группах отклоняются на сервере;
    /// скрытое свойство обязательным сделать нельзя.
    pub async fn update_property(
        &self,
        caller: &Identity,
        id: i64,
        version: i64,
        dto: PropertyUpdateDto,
    ) -> Result<GroupTypePropertyDto, AppError> {
        authorize(caller, &Action::ManageGroupTypes)?;
        let name = dto
            .name
            .as_deref()
            .map(|n| validate_name(n, "name"))
            .transpose()?;
        let data_type = dto
            .data_type
            .as_deref()
            .map(PropertyDataType::from_str)
            .transpose()?;
        let patch = PropertyPatch {
            name,
            data_type,
            is_required: dto.is_required,
            show_on_map: dto.show_on_map,
        };

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let current = repo.get_property(&tx, id)?;
                let before = to_json(&property_dto(&tx, &repo, current.clone())?)?;

                if let Some(new_type) = patch.data_type {
                    if new_type.as_str() != current.data_type {
                        let filled = repo.filled_group_count(&tx, id)?;
                        if filled > 0 {
                            return Err(AppError::Validation {
                                field: "data_type".to_string(),
                                message: format!(
                                    "Тип данных изменить нельзя: свойство заполнено в {}. \
                                     Создайте новое свойство.",
                                    in_groups(filled)
                                ),
                            });
                        }
                    }
                }
                if patch.is_required == Some(true) && current.archived_at_utc.is_some() {
                    // W-B01:1
                    return Err(hidden_property_required_error());
                }
                if patch.is_required == Some(true)
                    && !current.is_required
                    && current.archived_at_utc.is_none()
                {
                    let violators = repo.groups_missing_required(&tx, current.type_id, id)?;
                    if !violators.is_empty() {
                        return Err(required_violation_error(&violators));
                    }
                }
                if let Some(new_name) = &patch.name {
                    if current.archived_at_utc.is_none()
                        && live_name_taken(&tx, &repo, current.type_id, new_name, Some(id))?
                    {
                        return Err(duplicate_property_name_error());
                    }
                }

                let row = match repo.update_property_on(&tx, id, version, &patch, now) {
                    Ok(row) => row,
                    Err(e) if is_duplicate_property_conflict(&e) => {
                        return Err(duplicate_property_name_error());
                    }
                    Err(e) => return Err(e),
                };
                let out = property_dto(&tx, &repo, row)?;
                let after = to_json(&out)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "group_type_property",
                        entity_id: id,
                        action: "update",
                        user_id,
                        before_json: Some(before),
                        after_json: Some(after),
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(out)
            })
            .await
    }

    /// Удалить свойство: с заполненными значениями — скрыть (значения остаются, D-13),
    /// без значений — удалить физически.
    pub async fn delete_property(
        &self,
        caller: &Identity,
        id: i64,
    ) -> Result<PropertyDeleteOutcomeDto, AppError> {
        authorize(caller, &Action::ManageGroupTypes)?;
        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let current = repo.get_property(&tx, id)?;
                let before = to_json(&property_dto(&tx, &repo, current)?)?;
                let archived = repo.filled_group_count(&tx, id)? > 0;
                if archived {
                    repo.archive_property_on(&tx, id, now)?;
                } else {
                    repo.delete_property_hard_on(&tx, id)?;
                }
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "group_type_property",
                        entity_id: id,
                        action: if archived { "archive" } else { "delete" },
                        user_id,
                        before_json: Some(before),
                        after_json: None,
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(PropertyDeleteOutcomeDto { archived })
            })
            .await
    }

    /// Вернуть скрытое свойство. Занятое живое имя в типе — `Validation`; наследное
    /// обязательное свойство при нарушителях — `Validation` `is_required`.
    pub async fn unarchive_property(
        &self,
        caller: &Identity,
        id: i64,
    ) -> Result<GroupTypePropertyDto, AppError> {
        authorize(caller, &Action::ManageGroupTypes)?;
        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let current = repo.get_property(&tx, id)?;
                if current.archived_at_utc.is_none() {
                    return property_dto(&tx, &repo, current);
                }
                if live_name_taken(&tx, &repo, current.type_id, &current.name, Some(id))? {
                    return Err(duplicate_property_name_error());
                }
                // W-B01:2
                // Наследное «скрыто + обязательное» (строки до плана 41-29) не
                // отклоняется, а нормализуется: `unarchive_property_on` снимает
                // обязательность тем же UPDATE, которым возвращает свойство. Отказ
                // (как было до ревью WR-01) запирал пользователя — скрытое свойство
                // не заполнить, а в меню строки есть только «Показать». Живое
                // обязательное свойство без значений поэтому не появляется, и
                // set_values у групп не ломается.
                match repo.unarchive_property_on(&tx, id, now) {
                    Ok(()) => {}
                    Err(e) if is_duplicate_property_conflict(&e) => {
                        return Err(duplicate_property_name_error());
                    }
                    Err(e) => return Err(e),
                }
                let out = property_dto(&tx, &repo, repo.get_property(&tx, id)?)?;
                let after = to_json(&out)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "group_type_property",
                        entity_id: id,
                        action: "unarchive",
                        user_id,
                        before_json: None,
                        after_json: Some(after),
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(out)
            })
            .await
    }

    /// Переставить живые свойства типа одной операцией (D-10).
    pub async fn reorder_properties(
        &self,
        caller: &Identity,
        type_id: i32,
        ordered_ids: Vec<i32>,
    ) -> Result<Vec<GroupTypePropertyDto>, AppError> {
        authorize(caller, &Action::ManageGroupTypes)?;
        let type_id = i64::from(type_id);
        let ordered: Vec<i64> = ordered_ids.into_iter().map(i64::from).collect();
        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                repo.get_type(&tx, type_id)?;
                repo.reorder_properties_on(&tx, type_id, &ordered, now)?;
                let mut out = Vec::new();
                for p in repo.list_properties(&tx, type_id, false)? {
                    out.push(property_dto(&tx, &repo, p)?);
                }
                let payload = to_json(&ordered)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "group_type",
                        entity_id: type_id,
                        action: "reorder_properties",
                        user_id,
                        before_json: None,
                        after_json: None,
                        payload_json: Some(payload),
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(out)
            })
            .await
    }

    /// Полный список групп-нарушителей обязательности для попапа (D-14).
    /// Скрытое свойство нарушителей не имеет.
    pub async fn empty_groups_for_property(
        &self,
        caller: &Identity,
        property_id: i64,
    ) -> Result<Vec<GroupRefDto>, AppError> {
        authorize(caller, &Action::ReadGroups)?;
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let prop = repo.get_property(&conn, property_id)?;
            if prop.archived_at_utc.is_some() {
                return Ok(Vec::new());
            }
            Ok::<_, AppError>(
                repo.groups_missing_required(&conn, prop.type_id, property_id)?
                    .into_iter()
                    .map(|(id, name)| GroupRefDto { id, name })
                    .collect(),
            )
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    // ---- засев ---------------------------------------------------------

    /// Засев встроенных типов и их свойств по умолчанию при старте (GRP-01, D-31).
    ///
    /// Системный вызов без `caller`. Типы — идемпотентно по `code`
    /// (`ON CONFLICT DO NOTHING` в репозитории), свойства — однократно по маркеру
    /// `default_props_seeded`; всё одной транзакцией writer.
    pub async fn seed_builtin_types_on_startup(&self) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
        self.writer
            .execute(move |conn| {
                let builtin = |code: &str,
                               name: &str,
                               behavior: GroupBehavior,
                               label: &str,
                               sort_order: i64| GroupTypeNew {
                    code: code.to_string(),
                    name: name.to_string(),
                    behavior,
                    is_builtin: true,
                    sort_order,
                    quick_action_enabled: true,
                    quick_action_label: Some(label.to_string()),
                };
                let types = [
                    builtin(
                        "workstation",
                        "АРМ",
                        GroupBehavior::Container,
                        "Сформировать группу",
                        0,
                    ),
                    builtin(
                        "system_unit",
                        "Системный блок",
                        GroupBehavior::Substitute,
                        "Замещение группой",
                        1,
                    ),
                    builtin(
                        "teardown",
                        "Разбор",
                        GroupBehavior::Teardown,
                        "На разбор",
                        2,
                    ),
                ];
                let tx = conn.transaction().map_err(map_rusqlite)?;
                repo.seed_builtin_types_in_tx(&tx, &types, now)?;
                for (code, defaults) in BUILTIN_DEFAULT_PROPERTIES {
                    if defaults.is_empty() {
                        continue;
                    }
                    repo.seed_default_properties_in_tx(&tx, code, defaults, now)?;
                }
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await
    }
}
