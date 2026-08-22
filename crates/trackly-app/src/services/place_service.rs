//! `PlaceService` — application service for the Places entity (Phase 39).
//!
//! Mutation half (this plan, 39-05): `create`/`rename`/`move_node`/`archive`/
//! `unarchive`/`delete_hard` — each `authorize()`-gated (Admin-only per D-20,
//! `Action::MutatePlaces`), audit-logged (`entity_type = 'place'`), routed through
//! the single-writer task (`self.writer.execute(...)`).
//!
//! Read half (`get`/`list_children`/`list_all`/`subtree_stats`/`search`/`contents`)
//! is Plan 08's territory, same file, next wave — not implemented here.
//!
//! `PlaceRepository`'s mutating methods (`create`/`rename`/`archive`/`unarchive`)
//! take `&mut Self::Conn` (`&mut rusqlite::Connection`) directly — Plan 04
//! deliberately did NOT add `_in_tx`-style `&Transaction` variants (see
//! 39-04-SUMMARY.md's "one query definition per shape" decision). Because
//! `rusqlite::Transaction` only implements `Deref<Target = Connection>` (no
//! `DerefMut`), a `&mut Transaction` can never satisfy `&mut Self::Conn` — so
//! every mutation here calls the repo method directly on the writer closure's
//! own `&mut Connection` (each call is its own autocommitted SQLite statement,
//! matching Plan 02/04's port design), then opens a short-lived `conn.transaction()`
//! purely to insert the `audit_log` row via the shared `SqliteAuditLogRepository`
//! (mirrors `CartridgeService`/`PrinterService`'s `audit_repo.insert(&tx, AuditEntry
//! {...})` convention). `move_node`/`delete_hard` (Task 2) are internally atomic
//! already (Plan 04's `SqlitePlaceRepository` opens its own transaction for the
//! cycle-check+UPDATE / subtree-stats+DELETE compound operations) — this service
//! calls them as a single unit, then separately audit-logs the result.

use std::sync::Arc;

use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::domain::places::{PlaceNew, PlaceRow};
use trackly_core::error::AppError;
use trackly_core::ports::places::PlaceRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::audit_log_sqlite::{AuditEntry, SqliteAuditLogRepository};
use trackly_infra::repos::SqlitePlaceRepository;

use crate::dto::place::PlaceDto;

/// The `CREATE UNIQUE INDEX` name for D-04's sibling-name constraint
/// (`migrations/V037__places.sql`) — used to recognize the raw SQLite
/// constraint-violation message and translate it into UI-SPEC §11.2's
/// friendly Russian copy, rather than surfacing the raw SQLite text.
const DUPLICATE_NAME_INDEX: &str = "idx_places_parent_name_unique";

/// Application service for place-tree management. `Arc`-fields keep `Clone` O(1).
#[derive(Clone)]
pub struct PlaceService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) repo: Arc<SqlitePlaceRepository>,
    pub(crate) audit_repo: Arc<SqliteAuditLogRepository>,
}

impl PlaceService {
    /// Construct a new `PlaceService`. Called from `AppCtx::build` — same
    /// three-argument shape as `CartridgeService::new`/`DeviceService::new`.
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            repo: Arc::new(SqlitePlaceRepository),
            audit_repo: Arc::new(SqliteAuditLogRepository),
        }
    }

    // -----------------------------------------------------------------------
    // Validation helpers
    // -----------------------------------------------------------------------

    fn validate_name(name: &str) -> Result<(), AppError> {
        if name.trim().is_empty() {
            return Err(AppError::Validation {
                field: "name".to_string(),
                message: "Название обязательно для заполнения".to_string(),
            });
        }
        Ok(())
    }

    /// Translates the raw `idx_places_parent_name_unique` SQLite constraint
    /// violation (surfaced by `error_conversions::map_rusqlite` as a generic
    /// `AppError::Conflict { reason: <raw sqlite text> }`) into UI-SPEC §11.2's
    /// friendly Russian validation message: «В «{родитель}» уже есть место
    /// «{имя}». Укажите другое имя.» (or a root-list variant when `parent_id`
    /// is `None` — §11.2 does not specify root-node copy explicitly, this is
    /// the reachable equivalent, Rule 2).
    fn duplicate_name_error(
        conn: &rusqlite::Connection,
        repo: &SqlitePlaceRepository,
        parent_id: Option<i64>,
        name: &str,
    ) -> AppError {
        let message = match parent_id.and_then(|pid| repo.full_path(conn, pid).ok()) {
            Some(parent_path) => {
                format!("В «{parent_path}» уже есть место «{name}». Укажите другое имя.")
            }
            None => format!(
                "Место «{name}» уже существует в списке корневых мест. Укажите другое имя."
            ),
        };
        AppError::Validation {
            field: "name".to_string(),
            message,
        }
    }

    /// `reason` is the raw SQLite constraint message from `map_rusqlite`
    /// (T-39-04's `AppError::Conflict { reason }` mapping) — recognized by the
    /// unique-index name it always contains.
    fn is_duplicate_name_conflict(err: &AppError) -> bool {
        matches!(err, AppError::Conflict { reason } if reason.contains(DUPLICATE_NAME_INDEX))
    }

    fn to_after_json(dto: &PlaceDto) -> Result<String, AppError> {
        serde_json::to_string(dto).map_err(|e| AppError::Internal {
            source_chain: format!("audit_log after-json: {e}"),
        })
    }

    // -----------------------------------------------------------------------
    // Mutations
    // -----------------------------------------------------------------------

    /// Create a new place. Admin-only (D-20). `new.parent_id: None` creates a
    /// root node (D-03 — multiple roots allowed).
    pub async fn create(&self, caller: &Identity, new: PlaceNew) -> Result<PlaceDto, AppError> {
        authorize(caller, &Action::MutatePlaces)?;
        Self::validate_name(&new.name)?;

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();

        let row: PlaceRow = self
            .writer
            .execute(move |conn| {
                let id = match repo.create(conn, &new, now) {
                    Ok(id) => id,
                    Err(err) if Self::is_duplicate_name_conflict(&err) => {
                        return Err(Self::duplicate_name_error(conn, &repo, new.parent_id, &new.name));
                    }
                    Err(other) => return Err(other),
                };
                let row = repo.get(conn, id)?;

                let after_json = Self::to_after_json(&PlaceDto::from(row.clone()))?;
                let tx = conn.transaction().map_err(map_rusqlite)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "place",
                        entity_id: id,
                        action: "create",
                        user_id,
                        before_json: None,
                        after_json: Some(after_json),
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;

                Ok(row)
            })
            .await?;

        Ok(PlaceDto::from(row))
    }

    /// Rename a place (optimistic-lock CAS via `version`). Admin-only (D-20).
    pub async fn rename(
        &self,
        caller: &Identity,
        id: i64,
        name: String,
        version: i64,
    ) -> Result<PlaceDto, AppError> {
        authorize(caller, &Action::MutatePlaces)?;
        Self::validate_name(&name)?;

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();

        let row: PlaceRow = self
            .writer
            .execute(move |conn| {
                let before = repo.get(conn, id)?;
                let before_parent_id = before.parent_id;
                let before_json = Self::to_after_json(&PlaceDto::from(before))?;

                let row = match repo.rename(conn, id, &name, version, now) {
                    Ok(row) => row,
                    Err(err) if Self::is_duplicate_name_conflict(&err) => {
                        return Err(Self::duplicate_name_error(conn, &repo, before_parent_id, &name));
                    }
                    Err(other) => return Err(other),
                };

                let after_json = Self::to_after_json(&PlaceDto::from(row.clone()))?;
                let tx = conn.transaction().map_err(map_rusqlite)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "place",
                        entity_id: id,
                        action: "rename",
                        user_id,
                        before_json: Some(before_json),
                        after_json: Some(after_json),
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;

                Ok(row)
            })
            .await?;

        Ok(PlaceDto::from(row))
    }

    /// Archive a place (soft, reversible — D-15). Admin-only (D-20). Does NOT
    /// remove the node from the tree/cards/history — only hides it from
    /// `PlacePicker` (a read-path concern, Plan 08).
    pub async fn archive(&self, caller: &Identity, id: i64, version: i64) -> Result<(), AppError> {
        authorize(caller, &Action::MutatePlaces)?;
        self.set_archived(caller.user_id, id, version, true, "archive").await
    }

    /// Reverse `archive`. Admin-only (D-20).
    pub async fn unarchive(&self, caller: &Identity, id: i64, version: i64) -> Result<(), AppError> {
        authorize(caller, &Action::MutatePlaces)?;
        self.set_archived(caller.user_id, id, version, false, "unarchive").await
    }

    /// Shared archive/unarchive body — called only after the public method's
    /// own `authorize()` gate has already passed.
    async fn set_archived(
        &self,
        user_id: Option<i64>,
        id: i64,
        version: i64,
        archived: bool,
        action: &'static str,
    ) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();

        self.writer
            .execute(move |conn| {
                let before = repo.get(conn, id)?;
                let before_json = Self::to_after_json(&PlaceDto::from(before))?;

                if archived {
                    repo.archive(conn, id, version, now)?;
                } else {
                    repo.unarchive(conn, id, version, now)?;
                }

                let after = repo.get(conn, id)?;
                let after_json = Self::to_after_json(&PlaceDto::from(after))?;

                let tx = conn.transaction().map_err(map_rusqlite)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "place",
                        entity_id: id,
                        action,
                        user_id,
                        before_json: Some(before_json),
                        after_json: Some(after_json),
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;

                Ok(())
            })
            .await
    }
}
