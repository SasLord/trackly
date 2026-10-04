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
use trackly_core::domain::groups::{
    validate_name, GroupNew, GroupRow, GroupTypeRow, MAX_DEVICES_PER_BATCH,
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
    DeviceMembershipDto, GroupAddDevicesResultDto, GroupCompositionDto, GroupCreateDto,
    GroupDeleteResultDto, GroupDto, GroupMemberDeviceDto, GroupMoveResultDto, GroupSearchHitDto,
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
