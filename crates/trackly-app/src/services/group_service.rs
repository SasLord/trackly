//! `GroupService` — группы устройств: чтения и CRUD (Phase 41, GRP-04, GRP-05, GRP-08, GRP-10).
//!
//! Бизнес-правила живут здесь, чтобы оба транспорта (Tauri invoke и HTTP) были
//! тонкими адаптерами. Авторизация — ПЕРВОЙ строкой каждого метода: чтения —
//! `ReadGroups`, мутации — `MutateGroups` (admin|manager). Чтения идут через
//! пул читателей (`spawn_blocking`), мутации — через единственный writer, одна
//! `conn.transaction()` на мутацию вместе с аудитом.
//!
//! Имя группы по умолчанию строится ТОЛЬКО из целочисленной колонки `seq`;
//! существующие имена обратно не разбираются. Поиск вложенных кандидатов и
//! фильтры по подстроке идут в Rust (`to_lowercase().contains`): сопоставление шаблоном
//! в SQLite не сворачивает регистр кириллицы.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use rusqlite::Connection;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::domain::group_values;
use trackly_core::domain::groups::{
    validate_name, GroupNew, GroupRow, GroupTypeRow, GroupValueRow, PropertyDataType,
    MAX_DEVICES_PER_BATCH, MAX_REFS_PER_PROPERTY,
};
use trackly_core::error::AppError;
use trackly_core::ports::group_types::GroupTypeRepository;
use trackly_core::ports::groups::GroupRepository;
use trackly_core::ports::places::PlaceRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::audit_log_sqlite::{AuditEntry, SqliteAuditLogRepository};
use trackly_infra::repos::{
    SqliteGroupRepository, SqliteGroupTypeRepository, SqlitePlaceRepository,
};

use crate::dto::groups::{
    DeviceMembershipDto, GroupAddDevicesResultDto, GroupCardDto, GroupCompositionDto,
    GroupCreateDto, GroupDeleteResultDto, GroupDto, GroupMemberDeviceDto, GroupMoveResultDto,
    GroupPrinterDto, GroupPropertyValueDto, GroupSearchHitDto, GroupUserDto, GroupValueInputDto,
    UserOptionDto,
};
use crate::services::group_membership::release_device_in_tx;
use crate::services::group_place::{
    apply_group_place_to_device_in_tx, move_group_in_tx, propagate_group_place_in_tx,
    GroupMoveDeps, GroupMoveOutcome,
};
use crate::services::place_path_display::compute_place_path_short_with_conn;
use crate::services::plural::ru_plural;

/// Максимальная длина поискового запроса (как в `PlaceService::search`).
const SEARCH_QUERY_MAX_CHARS: usize = 100;
/// Потолок числа кандидатов в выдаче поиска.
const SEARCH_RESULT_LIMIT: usize = 50;

/// Сервис групп. `Arc`-поля делают `Clone` дешёвым.
#[derive(Clone)]
pub struct GroupService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) repo: Arc<SqliteGroupRepository>,
    pub(crate) type_repo: Arc<SqliteGroupTypeRepository>,
    pub(crate) places_repo: Arc<SqlitePlaceRepository>,
    pub(crate) audit_repo: Arc<SqliteAuditLogRepository>,
}

/// Снимок всех групп с производными данными: один набор запросов вместо N+1.
struct GroupSnapshot {
    rows: Vec<GroupRow>,
    types: HashMap<i64, GroupTypeRow>,
    counts: HashMap<i64, i64>,
    direct: HashMap<i64, i64>,
    paths: HashMap<i64, String>,
    nested: HashMap<i64, i64>,
}

impl GroupSnapshot {
    fn load(
        conn: &Connection,
        repo: &SqliteGroupRepository,
        type_repo: &SqliteGroupTypeRepository,
    ) -> Result<Self, AppError> {
        let rows = repo.list_groups(conn)?;
        let types = type_repo
            .list_types(conn)?
            .into_iter()
            .map(|t| (t.id, t))
            .collect();
        let counts = repo.tree_counts(conn)?.into_iter().collect();
        let direct = repo.direct_device_counts(conn)?.into_iter().collect();
        let paths = repo.group_place_paths(conn)?.into_iter().collect();
        let mut nested: HashMap<i64, i64> = HashMap::new();
        for r in &rows {
            if let Some(p) = r.parent_group_id {
                *nested.entry(p).or_insert(0) += 1;
            }
        }
        Ok(Self {
            rows,
            types,
            counts,
            direct,
            paths,
            nested,
        })
    }

    fn row(&self, id: i64) -> Option<&GroupRow> {
        self.rows.iter().find(|r| r.id == id)
    }

    /// Корень цепочки вложенности; обход ограничен числом групп (защита от цикла в испорченных данных).
    fn root_of<'a>(&'a self, row: &'a GroupRow) -> &'a GroupRow {
        let mut cur = row;
        for _ in 0..self.rows.len() {
            match cur.parent_group_id.and_then(|p| self.row(p)) {
                Some(parent) => cur = parent,
                None => break,
            }
        }
        cur
    }

    fn dto(&self, row: &GroupRow) -> Result<GroupDto, AppError> {
        let t = self
            .types
            .get(&row.type_id)
            .ok_or_else(|| AppError::Internal {
                source_chain: format!("group {}: type {} not found", row.id, row.type_id),
            })?;
        let root = self.root_of(row);
        Ok(GroupDto {
            id: row.id,
            type_id: row.type_id,
            type_code: t.code.clone(),
            type_name: t.name.clone(),
            type_behavior: t.behavior.clone(),
            name: row.name.clone(),
            seq: row.seq,
            place_id: row.place_id,
            place_path: self.paths.get(&row.id).cloned(),
            parent_group_id: row.parent_group_id,
            root_group_id: root.id,
            root_group_name: root.name.clone(),
            version: row.version,
            device_count: self.counts.get(&row.id).copied().unwrap_or(0),
            direct_device_count: self.direct.get(&row.id).copied().unwrap_or(0),
            nested_group_count: self.nested.get(&row.id).copied().unwrap_or(0),
        })
    }
}

/// DTO одной группы на переданном соединении; `NotFound`, если группы нет.
fn group_dto_on(
    conn: &Connection,
    repo: &SqliteGroupRepository,
    type_repo: &SqliteGroupTypeRepository,
    id: i64,
) -> Result<GroupDto, AppError> {
    let snap = GroupSnapshot::load(conn, repo, type_repo)?;
    let row = snap
        .row(id)
        .ok_or(AppError::NotFound {
            entity: "group",
            id,
        })?
        .clone();
    snap.dto(&row)
}

fn to_json<T: serde::Serialize>(v: &T) -> Result<String, AppError> {
    serde_json::to_string(v).map_err(|e| AppError::Internal {
        source_chain: format!("audit_log json: {e}"),
    })
}

fn join_err(e: tokio::task::JoinError) -> AppError {
    AppError::Internal {
        source_chain: format!("spawn_blocking: {e}"),
    }
}

impl GroupService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            repo: Arc::new(SqliteGroupRepository),
            type_repo: Arc::new(SqliteGroupTypeRepository),
            places_repo: Arc::new(SqlitePlaceRepository),
            audit_repo: Arc::new(SqliteAuditLogRepository),
        }
    }

    // ---- чтения --------------------------------------------------------

    /// Все живые группы для дерева: `ORDER BY type_id, seq`.
    pub async fn list_groups(&self, caller: &Identity) -> Result<Vec<GroupDto>, AppError> {
        authorize(caller, &Action::ReadGroups)?;
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let type_repo = self.type_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let snap = GroupSnapshot::load(&conn, &repo, &type_repo)?;
            snap.rows.iter().map(|r| snap.dto(r)).collect()
        })
        .await
        .map_err(join_err)?
    }

    /// Карточка группы; несуществующий id -> `NotFound`.
    pub async fn get_group(&self, caller: &Identity, id: i64) -> Result<GroupDto, AppError> {
        authorize(caller, &Action::ReadGroups)?;
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let type_repo = self.type_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            group_dto_on(&conn, &repo, &type_repo, id)
        })
        .await
        .map_err(join_err)?
    }

    /// Состав группы: прямые устройства и прямые вложенные группы (D-03, D-05).
    pub async fn composition(
        &self,
        caller: &Identity,
        group_id: i64,
    ) -> Result<GroupCompositionDto, AppError> {
        authorize(caller, &Action::ReadGroups)?;
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let type_repo = self.type_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let snap = GroupSnapshot::load(&conn, &repo, &type_repo)?;
            if snap.row(group_id).is_none() {
                return Err(AppError::NotFound {
                    entity: "group",
                    id: group_id,
                });
            }
            let devices = repo
                .member_devices(&conn, group_id)?
                .into_iter()
                .map(|d| GroupMemberDeviceDto {
                    place_path_short: compute_place_path_short_with_conn(
                        &conn,
                        d.place_id,
                        d.place_path.clone(),
                    ),
                    device_id: d.device_id,
                    type_name: d.type_name,
                    name: d.name,
                    inventory_number: d.inventory_number,
                    serial_number: d.serial_number,
                    place_id: d.place_id,
                    place_path: d.place_path,
                    status_name: d.status_name,
                })
                .collect();
            let child_groups = repo
                .direct_child_groups(&conn, group_id)?
                .iter()
                .map(|r| snap.dto(r))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(GroupCompositionDto {
                devices,
                child_groups,
            })
        })
        .await
        .map_err(join_err)?
    }

    /// Кандидаты во вложенные группы (D-04): без самой группы и её потомков,
    /// подстрока по имени без учёта регистра (в Rust, не средствами SQL), не более 50.
    pub async fn search(
        &self,
        caller: &Identity,
        query: String,
        exclude_group_id: Option<i64>,
    ) -> Result<Vec<GroupSearchHitDto>, AppError> {
        authorize(caller, &Action::ReadGroups)?;
        if query.chars().count() > SEARCH_QUERY_MAX_CHARS {
            return Err(AppError::Validation {
                field: "query".to_string(),
                message: format!(
                    "Запрос слишком длинный (макс. {SEARCH_QUERY_MAX_CHARS} символов)"
                ),
            });
        }
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let type_repo = self.type_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let snap = GroupSnapshot::load(&conn, &repo, &type_repo)?;

            // Исключаемое поддерево: сама группа и все потомки (в памяти, по parent_group_id).
            let mut excluded: HashSet<i64> = HashSet::new();
            if let Some(root) = exclude_group_id {
                excluded.insert(root);
                let mut grew = true;
                while grew {
                    grew = false;
                    for r in &snap.rows {
                        if let Some(p) = r.parent_group_id {
                            if excluded.contains(&p) && excluded.insert(r.id) {
                                grew = true;
                            }
                        }
                    }
                }
            }

            let needle = query.trim().to_lowercase();
            let mut out = Vec::new();
            for r in &snap.rows {
                if excluded.contains(&r.id) || !r.name.to_lowercase().contains(&needle) {
                    continue;
                }
                let dto = snap.dto(r)?;
                out.push(GroupSearchHitDto {
                    id: dto.id,
                    name: dto.name,
                    type_name: dto.type_name,
                    device_count: dto.device_count,
                    has_parent: dto.parent_group_id.is_some(),
                });
                if out.len() >= SEARCH_RESULT_LIMIT {
                    break;
                }
            }
            Ok(out)
        })
        .await
        .map_err(join_err)?
    }

    /// Членство устройств пачкой (D-01/D-19); только устройства-члены, потолок 500 id.
    pub async fn for_devices(
        &self,
        caller: &Identity,
        device_ids: Vec<i64>,
    ) -> Result<Vec<DeviceMembershipDto>, AppError> {
        authorize(caller, &Action::ReadGroups)?;
        if device_ids.len() > MAX_DEVICES_PER_BATCH {
            return Err(AppError::Validation {
                field: "device_ids".to_string(),
                message: format!(
                    "Слишком много устройств в одном запросе (не более {MAX_DEVICES_PER_BATCH})."
                ),
            });
        }
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            Ok(repo
                .groups_for_devices(&conn, &device_ids)?
                .into_iter()
                .map(|(device_id, g)| DeviceMembershipDto {
                    device_id,
                    group_id: g.id,
                    group_name: g.name,
                    group_has_place: g.place_id.is_some(),
                })
                .collect())
        })
        .await
        .map_err(join_err)?
    }
}

// ---- мутации -------------------------------------------------------------

impl GroupService {
    /// Создать группу. Без имени — «{имя типа} #{seq}», где `seq = MAX(seq)+1` по
    /// колонке типа (GRP-04); явное имя проходит `validate_name`. Место необязательно
    /// (D-21). Строк `place_movements` создание не пишет.
    pub async fn create_group(
        &self,
        caller: &Identity,
        dto: GroupCreateDto,
    ) -> Result<GroupDto, AppError> {
        authorize(caller, &Action::MutateGroups)?;
        let explicit_name = dto
            .name
            .as_deref()
            .map(|n| validate_name(n, "name"))
            .transpose()?;

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let repo = self.repo.clone();
        let type_repo = self.type_repo.clone();
        let places_repo = self.places_repo.clone();
        let audit_repo = self.audit_repo.clone();
        let type_id = dto.type_id;
        let place_id = dto.place_id.map(i64::from);

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let gtype = type_repo.get_type(&tx, type_id)?;
                if let Some(pid) = place_id {
                    let place = match places_repo.get(&tx, pid) {
                        Ok(p) => p,
                        Err(AppError::NotFound { .. }) => {
                            return Err(place_error("Место не найдено."))
                        }
                        Err(e) => return Err(e),
                    };
                    if place.archived_at_utc.is_some() {
                        return Err(place_error("Место в архиве: выберите другое."));
                    }
                }
                let seq = repo.next_seq_in_tx(&tx, type_id)?;
                let name = explicit_name.unwrap_or_else(|| format!("{} #{}", gtype.name, seq));
                let id = repo.insert_group_in_tx(
                    &tx,
                    &GroupNew {
                        type_id,
                        name,
                        seq,
                        place_id,
                    },
                    now,
                )?;
                let out = group_dto_on(&tx, &repo, &type_repo, id)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "group",
                        entity_id: id,
                        action: "create",
                        user_id,
                        before_json: None,
                        after_json: Some(to_json(&out)?),
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(out)
            })
            .await
    }

    /// Переименовать группу (CAS по `version`). Счётчик `seq` не меняется.
    pub async fn update_group(
        &self,
        caller: &Identity,
        id: i64,
        version: i64,
        name: String,
    ) -> Result<GroupDto, AppError> {
        authorize(caller, &Action::MutateGroups)?;
        let name = validate_name(&name, "name")?;

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let repo = self.repo.clone();
        let type_repo = self.type_repo.clone();
        let audit_repo = self.audit_repo.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let before = to_json(&group_dto_on(&tx, &repo, &type_repo, id)?)?;
                repo.rename_group_in_tx(&tx, id, version, &name, now)?;
                let out = group_dto_on(&tx, &repo, &type_repo, id)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "group",
                        entity_id: id,
                        action: "update",
                        user_id,
                        before_json: Some(before),
                        after_json: Some(to_json(&out)?),
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(out)
            })
            .await
    }

    /// Удалить группу (GRP-10). Прямые устройства освобождаются (членство исчезает),
    /// `devices.place_id` не трогается; вложенные группы становятся корневыми с тем же
    /// местом. Ответ несёт число освобождённых устройств для подтверждения в UI.
    pub async fn delete_group(
        &self,
        caller: &Identity,
        id: i64,
    ) -> Result<GroupDeleteResultDto, AppError> {
        authorize(caller, &Action::MutateGroups)?;

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let repo = self.repo.clone();
        let type_repo = self.type_repo.clone();
        let audit_repo = self.audit_repo.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let before = group_dto_on(&tx, &repo, &type_repo, id)?;
                let released = before.direct_device_count;
                let nested = before.nested_group_count;
                repo.delete_group_in_tx(&tx, id)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "group",
                        entity_id: id,
                        action: "delete",
                        user_id,
                        before_json: Some(to_json(&before)?),
                        after_json: None,
                        payload_json: Some(to_json(&serde_json::json!({
                            "released_devices": released,
                            "nested_groups": nested,
                        }))?),
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(GroupDeleteResultDto {
                    released_devices: released as i32,
                })
            })
            .await
    }

    /// Перенести группу вместе с составом (GRP-06, D-17/D-20/D-24). Одна транзакция:
    /// место группы и вложенных, место каждого устройства, пакет журнала и аудит.
    /// `version` — CAS по версии группы. Вложенную группу перенести нельзя.
    pub async fn move_group(
        &self,
        caller: &Identity,
        id: i64,
        version: i64,
        target_place_id: i64,
    ) -> Result<GroupMoveResultDto, AppError> {
        authorize(caller, &Action::MutateGroups)?;

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;

        let outcome = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let outcome = move_group_in_tx(
                    &tx,
                    &GroupMoveDeps::new(),
                    id,
                    Some(version),
                    target_place_id,
                    user_id,
                    now,
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(outcome)
            })
            .await?;

        let summary = move_summary(outcome.moved_devices);

        Ok(GroupMoveResultDto {
            moved_devices: outcome.moved_devices,
            moved_nested_groups: outcome.moved_nested_groups,
            changed_place_ids: outcome.changed_place_ids,
            summary,
            batch_id: outcome.batch_id,
        })
    }
}

// ---- состав --------------------------------------------------------------

/// Проверка списка id устройств: непустой, не больше потолка, без дублей (порядок сохранён).
fn normalize_device_ids(device_ids: Vec<i64>) -> Result<Vec<i64>, AppError> {
    let mut seen = HashSet::new();
    let ids: Vec<i64> = device_ids
        .into_iter()
        .filter(|id| seen.insert(*id))
        .collect();
    if ids.is_empty() {
        return Err(AppError::Validation {
            field: "device_ids".to_string(),
            message: "Выберите хотя бы одно устройство.".to_string(),
        });
    }
    if ids.len() > MAX_DEVICES_PER_BATCH {
        return Err(AppError::Validation {
            field: "device_ids".to_string(),
            message: format!(
                "Слишком много устройств в одном запросе (не более {MAX_DEVICES_PER_BATCH})."
            ),
        });
    }
    Ok(ids)
}

fn device_ids_error(message: String) -> AppError {
    AppError::Validation {
        field: "device_ids".to_string(),
        message,
    }
}

fn push_unique_place(out: &mut Vec<i64>, id: i64) {
    if !out.contains(&id) {
        out.push(id);
    }
}

impl GroupService {
    /// Добавить устройства в группу (GRP-05, D-01, D-21). Один обработчик для обоих
    /// путей UI; пакет атомарен: занятое, удалённое или несуществующее устройство
    /// отменяет весь пакет. Группа С местом присваивает его устройствам (журнал
    /// `source='group'` с `group_id` для ссылки D-28); группа БЕЗ места место не трогает.
    pub async fn add_devices(
        &self,
        caller: &Identity,
        group_id: i64,
        device_ids: Vec<i64>,
    ) -> Result<GroupAddDevicesResultDto, AppError> {
        authorize(caller, &Action::MutateGroups)?;
        let device_ids = normalize_device_ids(device_ids)?;

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let deps = GroupMoveDeps::new();
                let group = deps.groups.get_group_in_tx(&tx, group_id)?;

                let mut added: Vec<i64> = Vec::new();
                let mut changed_places: Vec<i64> = Vec::new();
                for device_id in &device_ids {
                    let device = match deps.devices.get_in_tx(&tx, *device_id) {
                        Ok(d) => d,
                        Err(AppError::NotFound { .. }) => {
                            return Err(device_ids_error("Устройство не найдено.".to_string()))
                        }
                        Err(e) => return Err(e),
                    };
                    if device.deleted_at_utc.is_some() {
                        return Err(device_ids_error(format!(
                            "Устройство «{}» удалено и не может входить в группу.",
                            device.name
                        )));
                    }
                    if let Some(existing) = deps.groups.group_of_device_in_tx(&tx, *device_id)? {
                        if existing.id == group_id {
                            continue;
                        }
                        return Err(device_ids_error(format!(
                            "Устройство «{}» уже входит в группу «{}». \
                             Сначала выведите его из состава.",
                            device.name, existing.name
                        )));
                    }
                    deps.groups
                        .add_device_in_tx(&tx, group_id, *device_id, now)?;
                    added.push(*device_id);

                    // D-21: место присваивает только группа С местом.
                    if let Some(place) = group.place_id {
                        let change = apply_group_place_to_device_in_tx(
                            &tx,
                            &deps,
                            *device_id,
                            place,
                            None,
                            Some(&group.name),
                            group_id,
                            user_id,
                            now,
                        )?;
                        if change.changed {
                            if let Some(p) = change.previous_place {
                                push_unique_place(&mut changed_places, p);
                            }
                            push_unique_place(&mut changed_places, place);
                        }
                    }
                }

                if !added.is_empty() {
                    deps.audit.insert(
                        &tx,
                        AuditEntry {
                            entity_type: "group",
                            entity_id: group_id,
                            action: "add_devices",
                            user_id,
                            before_json: None,
                            after_json: None,
                            payload_json: Some(to_json(&serde_json::json!({
                                "device_ids": added,
                            }))?),
                            created_at_utc: now,
                        },
                    )?;
                }
                tx.commit().map_err(map_rusqlite)?;
                Ok(GroupAddDevicesResultDto {
                    added: added.len() as i32,
                    changed_place_ids: changed_places,
                })
            })
            .await
    }

    /// Вывести устройства из группы (D-02): без подтверждения, место не меняется.
    /// Идемпотентно: устройства, которых нет в ЭТОЙ группе, игнорируются.
    /// Возвращает, сколько устройств действительно выведено.
    pub async fn remove_devices(
        &self,
        caller: &Identity,
        group_id: i64,
        device_ids: Vec<i64>,
    ) -> Result<i32, AppError> {
        authorize(caller, &Action::MutateGroups)?;
        let device_ids = normalize_device_ids(device_ids)?;

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let deps = GroupMoveDeps::new();
                deps.groups.get_group_in_tx(&tx, group_id)?;
                let mut released = 0;
                for device_id in &device_ids {
                    let member_of = deps.groups.group_of_device_in_tx(&tx, *device_id)?;
                    if member_of.map(|g| g.id) != Some(group_id) {
                        continue;
                    }
                    if release_device_in_tx(
                        &tx,
                        &deps.groups,
                        &deps.audit,
                        *device_id,
                        None,
                        user_id,
                        now,
                    )?
                    .is_some()
                    {
                        released += 1;
                    }
                }
                tx.commit().map_err(map_rusqlite)?;
                Ok(released)
            })
            .await
    }
}

// ---- вложенность ---------------------------------------------------------

impl GroupService {
    /// Вложить группу в группу либо вывести в корень (GRP-05, D-04). Цикл и вложение
    /// в «Разбор» отклоняет `set_parent_in_tx`. Место вложенной группы производное:
    /// родитель С местом протаскивает своё место на группу, её вложенные группы и весь
    /// состав (пакет журнала); родитель БЕЗ места обнуляет место групп поддерева, места
    /// устройств остаются («запрет спит»); выход в корень место не меняет.
    pub async fn set_parent(
        &self,
        caller: &Identity,
        id: i64,
        version: i64,
        parent_group_id: Option<i64>,
    ) -> Result<GroupMoveResultDto, AppError> {
        authorize(caller, &Action::MutateGroups)?;

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;

        let outcome = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let deps = GroupMoveDeps::new();
                let before = deps.groups.get_group_in_tx(&tx, id)?;
                deps.groups
                    .set_parent_in_tx(&tx, id, version, parent_group_id, now)?;

                let mut outcome = GroupMoveOutcome::default();
                if let Some(parent_id) = parent_group_id {
                    let parent = deps.groups.get_group_in_tx(&tx, parent_id)?;
                    match parent.place_id {
                        Some(parent_place) => {
                            let batch_id = uuid::Uuid::new_v4().to_string();
                            outcome = propagate_group_place_in_tx(
                                &tx,
                                &deps,
                                id,
                                parent_place,
                                before.place_id,
                                &batch_id,
                                user_id,
                                now,
                            )?;
                        }
                        None => {
                            if before.place_id.is_some() {
                                deps.groups.set_subtree_place_in_tx(&tx, id, None, now)?;
                                if let Some(p) = before.place_id {
                                    outcome.changed_place_ids.push(p);
                                }
                            }
                        }
                    }
                }

                let payload = serde_json::json!({
                    "from_parent": before.parent_group_id,
                    "to_parent": parent_group_id,
                    "batch_id": outcome.batch_id,
                    "moved_devices": outcome.moved_devices,
                });
                deps.audit.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "group",
                        entity_id: id,
                        action: "set_parent",
                        user_id,
                        before_json: None,
                        after_json: None,
                        payload_json: Some(to_json(&payload)?),
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(outcome)
            })
            .await?;

        Ok(GroupMoveResultDto {
            moved_devices: outcome.moved_devices,
            moved_nested_groups: outcome.moved_nested_groups,
            changed_place_ids: outcome.changed_place_ids,
            summary: move_summary(outcome.moved_devices),
            batch_id: outcome.batch_id,
        })
    }
}

/// Текст итога для тоста: «группа» либо «группа и 6 устройств».
fn move_summary(moved_devices: i32) -> String {
    let mut summary = "группа".to_string();
    if moved_devices > 0 {
        summary.push_str(&format!(
            " и {} {}",
            moved_devices,
            ru_plural(
                i64::from(moved_devices),
                "устройство",
                "устройства",
                "устройств"
            )
        ));
    }
    summary
}

fn place_error(message: &str) -> AppError {
    AppError::Validation {
        field: "place_id".to_string(),
        message: message.to_string(),
    }
}

// ---- значения свойств, пользователи для выбора, карточка (план 41-12) ----

/// Максимальная длина запроса и потолок выдачи в `user_options` (как в поиске групп).
const USER_OPTIONS_LIMIT: usize = 50;

const USER_NOT_FOUND_MESSAGE: &str = "Выбрать можно только пользователя, который уже заходил \
                                      в приложение. Поиска по каталогу AD нет.";

fn values_error(property_id: i64, message: impl Into<String>) -> AppError {
    AppError::Validation {
        field: format!("values.{property_id}"),
        message: message.into(),
    }
}

/// Проверенное значение одного свойства, готовое к записи.
struct PreparedValue {
    property_id: i64,
    rows: Vec<GroupValueRow>,
    /// Ссылки на пользователей/устройства для проверки живости одним запросом.
    user_refs: Vec<i64>,
    device_refs: Vec<i64>,
}

/// Разобрать и нормализовать одно значение формы. Живость ссылок проверяется отдельно.
fn prepare_value(
    group_id: i64,
    data_type: PropertyDataType,
    input: &GroupValueInputDto,
) -> Result<PreparedValue, AppError> {
    let pid = input.property_id;
    let mut prepared = PreparedValue {
        property_id: pid,
        rows: Vec::new(),
        user_refs: Vec::new(),
        device_refs: Vec::new(),
    };
    match data_type {
        PropertyDataType::Text
        | PropertyDataType::Number
        | PropertyDataType::Ip
        | PropertyDataType::Mac => {
            if !input.refs.is_empty() {
                return Err(values_error(
                    pid,
                    "Это свойство хранит одно значение, а не список связей.",
                ));
            }
            let raw = input.text.as_deref().unwrap_or("");
            let normalized =
                group_values::normalize_scalar(data_type, raw).map_err(|e| match e {
                    AppError::Validation { message, .. } => values_error(pid, message),
                    other => other,
                })?;
            if let Some(text) = normalized {
                prepared.rows.push(GroupValueRow {
                    group_id,
                    property_id: pid,
                    position: 0,
                    value_text: Some(text),
                    value_ref: None,
                    is_primary: false,
                });
            }
        }
        PropertyDataType::Users | PropertyDataType::DeviceRefs => {
            if input.text.as_deref().is_some_and(|t| !t.trim().is_empty()) {
                return Err(values_error(pid, "Это свойство хранит связи, а не текст."));
            }
            if input.refs.len() > MAX_REFS_PER_PROPERTY {
                return Err(values_error(
                    pid,
                    format!("Слишком много значений: не более {MAX_REFS_PER_PROPERTY}."),
                ));
            }
            let mut seen = HashSet::new();
            let mut primaries = 0;
            for (pos, r) in input.refs.iter().enumerate() {
                if !seen.insert(r.ref_id) {
                    return Err(values_error(pid, "Значение указано дважды."));
                }
                if r.is_primary {
                    primaries += 1;
                }
                prepared.rows.push(GroupValueRow {
                    group_id,
                    property_id: pid,
                    position: pos as i64,
                    value_text: None,
                    value_ref: Some(r.ref_id),
                    is_primary: r.is_primary,
                });
            }
            if primaries > 1 {
                return Err(values_error(pid, "Основным может быть только один."));
            }
            let ids: Vec<i64> = input.refs.iter().map(|r| r.ref_id).collect();
            if data_type == PropertyDataType::Users {
                prepared.user_refs = ids;
            } else {
                if primaries > 0 {
                    return Err(values_error(
                        pid,
                        "Для ссылок на устройства основное значение не задаётся.",
                    ));
                }
                prepared.device_refs = ids;
            }
        }
    }
    Ok(prepared)
}

/// Карточка группы на переданном соединении (общая для чтения и ответа `set_values`).
fn build_card(
    conn: &Connection,
    repo: &SqliteGroupRepository,
    type_repo: &SqliteGroupTypeRepository,
    id: i64,
) -> Result<GroupCardDto, AppError> {
    let snap = GroupSnapshot::load(conn, repo, type_repo)?;
    let row = snap
        .row(id)
        .ok_or(AppError::NotFound {
            entity: "group",
            id,
        })?
        .clone();
    let group = snap.dto(&row)?;
    let props = type_repo.list_properties(conn, row.type_id, false)?;
    let values = repo.list_values(conn, id)?;

    let type_of = |property_id: i64| {
        props
            .iter()
            .find(|p| p.id == property_id)
            .and_then(|p| PropertyDataType::from_str_lenient(&p.data_type))
    };
    let user_ids: Vec<i64> = values
        .iter()
        .filter(|v| type_of(v.property_id) == Some(PropertyDataType::Users))
        .filter_map(|v| v.value_ref)
        .collect();
    let live_users: HashMap<i64, String> = repo
        .live_users_by_ids(conn, &user_ids)?
        .into_iter()
        .collect();
    let ref_prop_ids: Vec<i64> = props
        .iter()
        .filter(|p| {
            PropertyDataType::from_str_lenient(&p.data_type) == Some(PropertyDataType::DeviceRefs)
        })
        .map(|p| p.id)
        .collect();
    let live_refs = repo.ref_devices_for_group(conn, id, &ref_prop_ids)?;
    let live_ref_ids: HashSet<i64> = live_refs.iter().map(|r| r.device_id).collect();

    let mut properties = Vec::with_capacity(props.len());
    for p in &props {
        let dt = PropertyDataType::from_str_lenient(&p.data_type);
        let own: Vec<&GroupValueRow> = values.iter().filter(|v| v.property_id == p.id).collect();
        let mut text = None;
        let mut users = Vec::new();
        let mut ref_device_ids = Vec::new();
        match dt {
            Some(PropertyDataType::Users) => {
                for v in own {
                    if let Some(uid) = v.value_ref {
                        if let Some(name) = live_users.get(&uid) {
                            users.push(GroupUserDto {
                                user_id: uid,
                                full_name: name.clone(),
                                is_primary: v.is_primary,
                            });
                        }
                    }
                }
            }
            Some(PropertyDataType::DeviceRefs) => {
                for v in own {
                    if let Some(did) = v.value_ref {
                        if live_ref_ids.contains(&did) {
                            ref_device_ids.push(did);
                        }
                    }
                }
            }
            _ => {
                text = own.iter().find_map(|v| v.value_text.clone());
            }
        }
        properties.push(GroupPropertyValueDto {
            property_id: p.id,
            name: p.name.clone(),
            data_type: p.data_type.clone(),
            sort_order: p.sort_order,
            is_required: p.is_required,
            show_on_map: p.show_on_map,
            text,
            users,
            ref_device_ids,
        });
    }

    // Единый список принтеров: USB-производные + явные ссылки на принтеры, дедуп по device_id.
    let link_candidates: Vec<i64> = live_refs.iter().map(|r| r.device_id).collect();
    let link_printers = repo.live_printer_device_ids(conn, &link_candidates)?;
    let explicit: HashSet<i64> = live_refs
        .iter()
        .map(|r| r.device_id)
        .filter(|d| link_printers.contains(d))
        .collect();
    let mut printers: Vec<GroupPrinterDto> = Vec::new();
    let mut seen: HashSet<i64> = HashSet::new();
    for r in repo.usb_printers_for_group(conn, id)? {
        if seen.insert(r.device_id) {
            printers.push(GroupPrinterDto {
                device_id: r.device_id,
                name: r.name,
                inventory_number: r.inventory_number,
                serial_number: r.serial_number,
                origin: "usb".to_string(),
                has_explicit_link: explicit.contains(&r.device_id),
            });
        }
    }
    for r in live_refs {
        if explicit.contains(&r.device_id) && seen.insert(r.device_id) {
            printers.push(GroupPrinterDto {
                device_id: r.device_id,
                name: r.name,
                inventory_number: r.inventory_number,
                serial_number: r.serial_number,
                origin: "link".to_string(),
                has_explicit_link: true,
            });
        }
    }
    printers.sort_by(|a, b| a.name.cmp(&b.name).then(a.device_id.cmp(&b.device_id)));

    let link_property_id = ref_prop_ids.first().copied();
    Ok(GroupCardDto {
        group,
        properties,
        printers,
        link_property_id,
    })
}

impl GroupService {
    /// Пользователи для выбора (D-11): живые активные, только `id`, `full_name`, `login`,
    /// подстрока без учёта регистра в Rust, не более 50. Под `ReadGroups` — manager
    /// не имеет права на `users_list`, а выбирать людей в группу ему можно.
    pub async fn user_options(
        &self,
        caller: &Identity,
        query: String,
    ) -> Result<Vec<UserOptionDto>, AppError> {
        authorize(caller, &Action::ReadGroups)?;
        if query.chars().count() > SEARCH_QUERY_MAX_CHARS {
            return Err(AppError::Validation {
                field: "query".to_string(),
                message: format!(
                    "Запрос слишком длинный (макс. {SEARCH_QUERY_MAX_CHARS} символов)"
                ),
            });
        }
        let needle = query.trim().to_lowercase();
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let rows = repo.list_user_options(&conn)?;
            Ok(rows
                .into_iter()
                .filter(|(_, full_name, login)| {
                    needle.is_empty()
                        || full_name.to_lowercase().contains(&needle)
                        || login.to_lowercase().contains(&needle)
                })
                .take(USER_OPTIONS_LIMIT)
                .map(|(id, full_name, login)| UserOptionDto {
                    id,
                    full_name,
                    login,
                })
                .collect())
        })
        .await
        .map_err(join_err)?
    }

    /// Карточка группы: свойства типа со значениями и единый список принтеров (D-12).
    pub async fn card(&self, caller: &Identity, id: i64) -> Result<GroupCardDto, AppError> {
        authorize(caller, &Action::ReadGroups)?;
        let readers = self.readers.clone();
        let repo = self.repo.clone();
        let type_repo = self.type_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            build_card(&conn, &repo, &type_repo, id)
        })
        .await
        .map_err(join_err)?
    }

    /// Записать значения свойств группы (GRP-03). Проверка и нормализация — на сервере,
    /// одна транзакция: либо записано всё, либо ничего. `version` — CAS по группе.
    /// Переданное свойство полностью заменяется; непереданные не трогаются.
    pub async fn set_values(
        &self,
        caller: &Identity,
        id: i64,
        version: i64,
        values: Vec<GroupValueInputDto>,
    ) -> Result<GroupCardDto, AppError> {
        authorize(caller, &Action::MutateGroups)?;

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let repo = self.repo.clone();
        let type_repo = self.type_repo.clone();
        let audit_repo = self.audit_repo.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                let group = repo.get_group_in_tx(&tx, id)?;
                if group.version != version {
                    return Err(AppError::OptimisticLockMismatch {
                        entity: "group",
                        id,
                        expected: version,
                        actual: group.version,
                    });
                }
                let props = type_repo.list_properties(&tx, group.type_id, false)?;

                let mut prepared: Vec<PreparedValue> = Vec::with_capacity(values.len());
                let mut seen = HashSet::new();
                for input in &values {
                    let pid = input.property_id;
                    let prop = props
                        .iter()
                        .find(|p| p.id == pid)
                        .ok_or_else(|| values_error(pid, "Свойство не относится к этой группе."))?;
                    if !seen.insert(pid) {
                        return Err(values_error(pid, "Свойство указано дважды."));
                    }
                    let data_type = PropertyDataType::from_str_lenient(&prop.data_type)
                        .ok_or_else(|| values_error(pid, "Неизвестный тип свойства."))?;
                    prepared.push(prepare_value(id, data_type, input)?);
                }

                // Живость ссылок — в той же транзакции, одним запросом на вид ссылки.
                let all_users: Vec<i64> =
                    prepared.iter().flat_map(|p| p.user_refs.iter().copied()).collect();
                let live_users = repo.live_user_ids_in_tx(&tx, &all_users)?;
                let all_devices: Vec<i64> =
                    prepared.iter().flat_map(|p| p.device_refs.iter().copied()).collect();
                let live_devices = repo.live_device_ids_in_tx(&tx, &all_devices)?;
                for p in &prepared {
                    if p.user_refs.iter().any(|u| !live_users.contains(u)) {
                        return Err(values_error(p.property_id, USER_NOT_FOUND_MESSAGE));
                    }
                    if p.device_refs.iter().any(|d| !live_devices.contains(d)) {
                        return Err(values_error(
                            p.property_id,
                            "Выбрать можно только существующее устройство.",
                        ));
                    }
                }

                for p in &prepared {
                    repo.replace_property_values_in_tx(&tx, id, p.property_id, &p.rows, now)?;
                }

                // Обязательность — по итоговому состоянию всех живых обязательных свойств.
                for prop in props.iter().filter(|p| p.is_required) {
                    if !repo.property_filled_in_tx(&tx, id, prop.id)? {
                        return Err(values_error(
                            prop.id,
                            format!("Заполните обязательное свойство «{}».", prop.name),
                        ));
                    }
                }

                repo.bump_group_version_in_tx(&tx, id, version, now)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "group",
                        entity_id: id,
                        action: "set_values",
                        user_id,
                        before_json: None,
                        after_json: None,
                        payload_json: Some(to_json(&serde_json::json!({
                            "property_ids": prepared.iter().map(|p| p.property_id).collect::<Vec<_>>(),
                        }))?),
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                build_card(conn, &repo, &type_repo, id)
            })
            .await
    }
}
