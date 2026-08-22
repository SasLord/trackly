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
use trackly_core::domain::places::{PlaceNew, PlaceRow, SubtreeStats};
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

    /// Move a place to a new parent (or to root, if `new_parent_id` is `None`).
    /// Admin-only (D-20). The cycle-rejection `AppError::Validation` raised by
    /// `SqlitePlaceRepository::move_node` (Plan 04, Pattern 3) is propagated
    /// unchanged — its message is already the UI-SPEC §14.3-locked copy.
    pub async fn move_node(
        &self,
        caller: &Identity,
        id: i64,
        new_parent_id: Option<i64>,
        version: i64,
    ) -> Result<PlaceDto, AppError> {
        authorize(caller, &Action::MutatePlaces)?;

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();

        let row: PlaceRow = self
            .writer
            .execute(move |conn| {
                let before = repo.get(conn, id)?;
                let before_json = Self::to_after_json(&PlaceDto::from(before))?;

                // Cycle check + UPDATE run atomically inside SqlitePlaceRepository's
                // own transaction (Pattern 3, Plan 04) — not re-wrapped here.
                let row = repo.move_node(conn, id, new_parent_id, version, now)?;

                let after_json = Self::to_after_json(&PlaceDto::from(row.clone()))?;
                let tx = conn.transaction().map_err(map_rusqlite)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "place",
                        entity_id: id,
                        action: "move",
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

    /// Hard-delete a place (irreversible). Admin-only (D-20). D-14: no cascade,
    /// no auto-reparenting — the subtree must be empty (no direct/nested
    /// children, no devices, no cartridges). `subtree_stats` is checked on the
    /// READ path (reader pool, not the writer) first; if non-empty, an
    /// `AppError::Conflict` carrying the exact UI-SPEC §11.5/§14.3 counts is
    /// returned WITHOUT touching the writer at all.
    pub async fn delete_hard(&self, caller: &Identity, id: i64, version: i64) -> Result<(), AppError> {
        authorize(caller, &Action::MutatePlaces)?;

        let stats: SubtreeStats = {
            let readers = self.readers.clone();
            let repo = self.repo.clone();
            tokio::task::spawn_blocking(move || {
                let conn = readers.acquire();
                repo.subtree_stats(&conn, id)
            })
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("spawn_blocking: {e}"),
            })??
        };

        let total = stats.device_count + stats.nested_places + stats.cartridge_count;
        if total > 0 {
            return Err(AppError::Conflict {
                reason: build_delete_blocked_message(&stats),
            });
        }

        let now = self.clock.unix_seconds();
        let user_id = caller.user_id;
        let repo = self.repo.clone();
        let audit_repo = self.audit_repo.clone();

        self.writer
            .execute(move |conn| {
                let before = repo.get(conn, id)?;
                let before_json = Self::to_after_json(&PlaceDto::from(before))?;

                repo.delete_hard(conn, id, version)?;

                let tx = conn.transaction().map_err(map_rusqlite)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "place",
                        entity_id: id,
                        action: "delete",
                        user_id,
                        before_json: Some(before_json),
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
}

// ---------------------------------------------------------------------------
// D-14 delete-blocked message — UI-SPEC §11.5/§14.3 literal Russian copy,
// with §11.3's singular/plural agreement rule applied identically (plan Task 2).
// ---------------------------------------------------------------------------

/// Russian noun pluralization by count: `one` (1, 21, 31, …), `few` (2-4, 22-24, …),
/// `many` (0, 5-20, 25-30, …). The 11-14 exception (which would otherwise match
/// `few` via `n % 10 == 1..4`) always resolves to `many`.
fn ru_plural(n: i64, one: &'static str, few: &'static str, many: &'static str) -> &'static str {
    let n_abs = n.unsigned_abs();
    let mod100 = n_abs % 100;
    let mod10 = n_abs % 10;
    if (11..=14).contains(&mod100) {
        many
    } else {
        match mod10 {
            1 => one,
            2..=4 => few,
            _ => many,
        }
    }
}

fn join_with_and(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [only] => only.clone(),
        [rest @ .., last] => format!("{} и {}", rest.join(", "), last),
    }
}

/// Builds the exact D-14 message: «Место нельзя удалить: в нём {N} устройств и
/// {N} вложенных места. Перенесите содержимое или архивируйте место.» —
/// matching UI-SPEC §11.5/§14.3's literal template. Zero-count parts are
/// omitted (§11.3's rule, applied identically here). `cartridge_count` is not
/// part of the literal §11.5 example but is included as a third clause when
/// non-zero (Rule 2 — without it, a place containing ONLY cartridges would
/// otherwise produce an empty, broken message body).
fn build_delete_blocked_message(stats: &SubtreeStats) -> String {
    let mut parts = Vec::new();
    if stats.device_count > 0 {
        parts.push(format!(
            "{} {}",
            stats.device_count,
            ru_plural(stats.device_count, "устройство", "устройства", "устройств")
        ));
    }
    if stats.nested_places > 0 {
        parts.push(format!(
            "{} {}",
            stats.nested_places,
            ru_plural(
                stats.nested_places,
                "вложенное место",
                "вложенных места",
                "вложенных мест"
            )
        ));
    }
    if stats.cartridge_count > 0 {
        parts.push(format!(
            "{} {}",
            stats.cartridge_count,
            ru_plural(stats.cartridge_count, "картридж", "картриджа", "картриджей")
        ));
    }
    format!(
        "Место нельзя удалить: в нём {}. Перенесите содержимое или архивируйте место.",
        join_with_and(&parts)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ru_plural_device_word_matches_ui_spec_example() {
        assert_eq!(ru_plural(12, "устройство", "устройства", "устройств"), "устройств");
        assert_eq!(ru_plural(1, "устройство", "устройства", "устройств"), "устройство");
        assert_eq!(ru_plural(2, "устройство", "устройства", "устройств"), "устройства");
    }

    #[test]
    fn build_delete_blocked_message_matches_ui_spec_literal_example() {
        let stats = SubtreeStats {
            direct_children: 2,
            nested_places: 2,
            device_count: 12,
            cartridge_count: 0,
        };
        let msg = build_delete_blocked_message(&stats);
        assert_eq!(
            msg,
            "Место нельзя удалить: в нём 12 устройств и 2 вложенных места. \
             Перенесите содержимое или архивируйте место."
        );
    }

    #[test]
    fn build_delete_blocked_message_omits_zero_parts() {
        let stats = SubtreeStats {
            direct_children: 0,
            nested_places: 0,
            device_count: 1,
            cartridge_count: 0,
        };
        let msg = build_delete_blocked_message(&stats);
        assert_eq!(
            msg,
            "Место нельзя удалить: в нём 1 устройство. Перенесите содержимое или архивируйте место."
        );
    }

    #[test]
    fn build_delete_blocked_message_includes_cartridges_when_only_cartridges_present() {
        let stats = SubtreeStats {
            direct_children: 0,
            nested_places: 0,
            device_count: 0,
            cartridge_count: 3,
        };
        let msg = build_delete_blocked_message(&stats);
        assert_eq!(
            msg,
            "Место нельзя удалить: в нём 3 картриджа. Перенесите содержимое или архивируйте место."
        );
    }
}
