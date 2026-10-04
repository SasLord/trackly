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
use trackly_core::domain::groups::{GroupRow, GroupTypeRow, MAX_DEVICES_PER_BATCH};
use trackly_core::error::AppError;
use trackly_core::ports::group_types::GroupTypeRepository;
use trackly_core::ports::groups::GroupRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::repos::audit_log_sqlite::SqliteAuditLogRepository;
use trackly_infra::repos::{
    SqliteGroupRepository, SqliteGroupTypeRepository, SqlitePlaceRepository,
};

use crate::dto::groups::{
    DeviceMembershipDto, GroupCompositionDto, GroupDto, GroupMemberDeviceDto, GroupSearchHitDto,
};
use crate::services::place_path_display::compute_place_path_short_with_conn;

/// Максимальная длина поискового запроса (как в `PlaceService::search`).
const SEARCH_QUERY_MAX_CHARS: usize = 100;
/// Потолок числа кандидатов в выдаче поиска.
const SEARCH_RESULT_LIMIT: usize = 50;

/// Сервис групп. `Arc`-поля делают `Clone` дешёвым.
// TEMP-T1: clock/places_repo/audit_repo используются мутациями (задача 2).
#[allow(dead_code)]
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
