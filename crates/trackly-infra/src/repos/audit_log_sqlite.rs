//! SQLite adapter for the `audit_log` table.
//!
//! Thin repository with insert + JSON1-based selection. Used by:
//!   - `ActService::create` / `do_return` to record device mutations
//!     (`payload_json = {act_id, kind}`) and the act-level create event.
//!   - Future undo path (plan 03): `select_device_mutations_for_act`
//!     reads `before_json` rows to restore device snapshots.
//!
//! NB: V008 declares `audit_log` as a hard-delete table (no `deleted_at_utc`
//! / no `version`). Retention is owned by Phase 7 scheduled tasks.

use rusqlite::{params, OptionalExtension, Transaction};
use trackly_core::error::AppError;

use crate::error_conversions::map_rusqlite;

/// SQLite-backed audit_log repository adapter (zero-sized).
#[derive(Debug, Default, Clone)]
pub struct SqliteAuditLogRepository;

/// A single audit_log entry, ready to be inserted.
///
/// `'a` ties the `action` borrow to the caller's frame; strings carried
/// inside (`before_json`, `after_json`, `payload_json`) are owned because
/// they typically come from `serde_json::to_string(...)`.
#[derive(Debug, Clone)]
pub struct AuditEntry<'a> {
    pub entity_type: &'static str,
    pub entity_id: i64,
    pub action: &'a str,
    pub user_id: Option<i64>,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    pub payload_json: Option<String>,
    pub created_at_utc: i64,
}

impl SqliteAuditLogRepository {
    /// Insert a single audit_log row inside an open transaction.
    pub fn insert(&self, tx: &Transaction<'_>, e: AuditEntry<'_>) -> Result<(), AppError> {
        tx.execute(
            "INSERT INTO audit_log \
             (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                e.entity_type,
                e.entity_id,
                e.action,
                e.user_id,
                e.before_json,
                e.after_json,
                e.payload_json,
                e.created_at_utc,
            ],
        )
        .map_err(map_rusqlite)?;
        Ok(())
    }

    /// SELECT device-mutation rows linked to a given act via
    /// `json_extract(payload_json, '$.act_id') = act_id`.
    ///
    /// Returns pairs `(device_id, before_json)` in chronological insert
    /// order. Used by the undo path (delete handover → restore each
    /// device from its snapshot) — plan 03 will consume this.
    pub fn select_device_mutations_for_act(
        &self,
        tx: &Transaction<'_>,
        act_id: i64,
    ) -> Result<Vec<(i64, String)>, AppError> {
        let mut stmt = tx
            .prepare(
                "SELECT entity_id, before_json FROM audit_log \
                 WHERE entity_type = 'device' \
                   AND json_extract(payload_json, '$.act_id') = ?1 \
                   AND before_json IS NOT NULL \
                 ORDER BY created_at_utc ASC, id ASC",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map(params![act_id], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(map_rusqlite)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(map_rusqlite)?);
        }
        Ok(out)
    }

    /// SELECT the single most-recent device-mutation snapshot linked to a
    /// given act, for one specific device (Phase 19, ACT-02).
    ///
    /// Unlike `select_device_mutations_for_act` (which returns ALL rows in
    /// chronological ASC order for full-act LIFO undo), this returns just the
    /// single row immediately preceding the most recent edit — `ORDER BY
    /// created_at_utc DESC, id DESC LIMIT 1` (Pitfall 2: taking the FIRST
    /// (ASC) row instead would restore to the ORIGINAL creation-time
    /// snapshot rather than the state right before the most recent edit).
    ///
    /// Callers: `ActService::update`'s handover-edit removed-device restore,
    /// and `ActService::update_return`'s un-return restore (Phase 22).
    ///
    /// CR-02 (Phase 22 gap-closure): excludes rows tagged `action =
    /// 'custom:return_item_edit'` — those rows are written by
    /// `update_return`'s retained-edit loop and capture an intermediate
    /// within-act mutation snapshot, not the act's own original device
    /// mutation. A caller restoring a device's pre-mutation state (un-return,
    /// handover-edit-removal) must never pick up that intermediate snapshot
    /// instead of the act's true original mutation. This exclusion is inert
    /// for the handover-edit caller, which never writes rows with that
    /// action tag.
    pub fn select_latest_device_mutation(
        &self,
        tx: &Transaction<'_>,
        act_id: i64,
        device_id: i64,
    ) -> Result<Option<String>, AppError> {
        tx.query_row(
            "SELECT before_json FROM audit_log \
             WHERE entity_type = 'device' \
               AND entity_id = ?2 \
               AND json_extract(payload_json, '$.act_id') = ?1 \
               AND before_json IS NOT NULL \
               AND action != 'custom:return_item_edit' \
             ORDER BY created_at_utc DESC, id DESC LIMIT 1",
            params![act_id, device_id],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(map_rusqlite)
    }

    /// SELECT the single most-recent device-mutation `(before_json,
    /// after_json)` pair linked to a given act, for one specific device
    /// (Phase 22, ACT-03 — D-11 safety check).
    ///
    /// Sibling of `select_latest_device_mutation`, same `ORDER BY
    /// created_at_utc DESC, id DESC LIMIT 1` predicate, but returns both
    /// columns in one query round-trip: `before_json` is the un-return
    /// restore basis, `after_json` is the D-11 drift-comparison basis
    /// ("what this act's own most-recent mutation set the device to").
    pub fn select_latest_device_mutation_pair(
        &self,
        tx: &Transaction<'_>,
        act_id: i64,
        device_id: i64,
    ) -> Result<Option<(String, String)>, AppError> {
        tx.query_row(
            "SELECT before_json, after_json FROM audit_log \
             WHERE entity_type = 'device' \
               AND entity_id = ?2 \
               AND json_extract(payload_json, '$.act_id') = ?1 \
               AND before_json IS NOT NULL \
             ORDER BY created_at_utc DESC, id DESC LIMIT 1",
            params![act_id, device_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(map_rusqlite)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use crate::db::pragmas::apply_writer_pragmas;
    use rusqlite::Connection;
    use tempfile::TempDir;

    fn fresh_conn() -> (Connection, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("audit-test.db");
        let mut conn = Connection::open(&path).expect("open");
        apply_writer_pragmas(&conn).expect("pragmas");
        migrations::run(&mut conn).expect("migrations");
        (conn, dir)
    }

    #[test]
    fn round_trip_insert_and_select_by_act_id() {
        let (mut conn, _g) = fresh_conn();
        let repo = SqliteAuditLogRepository;

        let tx = conn.transaction().expect("tx");
        repo.insert(
            &tx,
            AuditEntry {
                entity_type: "device",
                entity_id: 7,
                action: "update",
                user_id: None,
                before_json: Some("{\"name\":\"A\"}".into()),
                after_json: Some("{\"name\":\"B\"}".into()),
                payload_json: Some("{\"act_id\":42,\"kind\":\"handover\"}".into()),
                created_at_utc: 1_700_000_000,
            },
        )
        .expect("insert device");

        // Unrelated row: different act_id, should NOT be returned.
        repo.insert(
            &tx,
            AuditEntry {
                entity_type: "device",
                entity_id: 9,
                action: "update",
                user_id: None,
                before_json: Some("{\"name\":\"X\"}".into()),
                after_json: Some("{\"name\":\"Y\"}".into()),
                payload_json: Some("{\"act_id\":99,\"kind\":\"handover\"}".into()),
                created_at_utc: 1_700_000_001,
            },
        )
        .expect("insert unrelated");

        let rows = repo
            .select_device_mutations_for_act(&tx, 42)
            .expect("select");
        assert_eq!(rows.len(), 1, "exactly one row for act_id=42");
        assert_eq!(rows[0].0, 7);
        assert!(rows[0].1.contains("\"name\":\"A\""));

        tx.commit().expect("commit");
    }

    #[test]
    fn select_latest_device_mutation_pair_returns_newest_row_for_act() {
        let (mut conn, _g) = fresh_conn();
        let repo = SqliteAuditLogRepository;

        let tx = conn.transaction().expect("tx");

        // Two rows for the SAME device_id under two DIFFERENT act_ids.
        repo.insert(
            &tx,
            AuditEntry {
                entity_type: "device",
                entity_id: 7,
                action: "update",
                user_id: None,
                before_json: Some("{\"status_id\":1}".into()),
                after_json: Some("{\"status_id\":2}".into()),
                payload_json: Some("{\"act_id\":42,\"kind\":\"handover\"}".into()),
                created_at_utc: 1_700_000_000,
            },
        )
        .expect("insert act 42 row");

        repo.insert(
            &tx,
            AuditEntry {
                entity_type: "device",
                entity_id: 7,
                action: "update",
                user_id: None,
                before_json: Some("{\"status_id\":2}".into()),
                after_json: Some("{\"status_id\":3}".into()),
                payload_json: Some("{\"act_id\":50,\"kind\":\"return\"}".into()),
                created_at_utc: 1_700_000_001,
            },
        )
        .expect("insert act 50 row (older, for act 50)");

        // Newer THIRD row for the same device_id AND the same act_id as the
        // first row (act 42) — must be the one returned for act_id=42.
        repo.insert(
            &tx,
            AuditEntry {
                entity_type: "device",
                entity_id: 7,
                action: "custom:update_remove",
                user_id: None,
                before_json: Some("{\"status_id\":2}".into()),
                after_json: Some("{\"status_id\":4}".into()),
                payload_json: Some("{\"act_id\":42,\"kind\":\"return\"}".into()),
                created_at_utc: 1_700_000_002,
            },
        )
        .expect("insert newest act 42 row");

        let pair = repo
            .select_latest_device_mutation_pair(&tx, 42, 7)
            .expect("select")
            .expect("row exists");
        assert_eq!(pair.0, "{\"status_id\":2}", "before_json of newest act-42 row");
        assert_eq!(pair.1, "{\"status_id\":4}", "after_json of newest act-42 row");

        tx.commit().expect("commit");
    }

    #[test]
    fn select_latest_device_mutation_pair_returns_none_when_no_match() {
        let (mut conn, _g) = fresh_conn();
        let repo = SqliteAuditLogRepository;

        let tx = conn.transaction().expect("tx");
        let pair = repo
            .select_latest_device_mutation_pair(&tx, 999, 999)
            .expect("select");
        assert!(pair.is_none(), "no matching audit rows → Ok(None)");
        tx.commit().expect("commit");
    }

    #[test]
    fn select_latest_device_mutation_excludes_return_item_edit_action() {
        let (mut conn, _g) = fresh_conn();
        let repo = SqliteAuditLogRepository;

        let tx = conn.transaction().expect("tx");

        // Older row: the return's own original device mutation.
        repo.insert(
            &tx,
            AuditEntry {
                entity_type: "device",
                entity_id: 7,
                action: "update",
                user_id: None,
                before_json: Some("{\"status_id\":2}".into()),
                after_json: Some("{\"status_id\":1}".into()),
                payload_json: Some("{\"act_id\":42,\"kind\":\"return\"}".into()),
                created_at_utc: 1_700_000_000,
            },
        )
        .expect("insert older 'update' row");

        // Newer row: a within-return retained-edit mutation — must be
        // EXCLUDED from select_latest_device_mutation's restore lookup
        // (CR-02).
        repo.insert(
            &tx,
            AuditEntry {
                entity_type: "device",
                entity_id: 7,
                action: "custom:return_item_edit",
                user_id: None,
                before_json: Some("{\"status_id\":3}".into()),
                after_json: Some("{\"status_id\":1}".into()),
                payload_json: Some("{\"act_id\":42,\"kind\":\"return\"}".into()),
                created_at_utc: 1_700_000_001,
            },
        )
        .expect("insert newer 'custom:return_item_edit' row");

        let before_json = repo
            .select_latest_device_mutation(&tx, 42, 7)
            .expect("select")
            .expect("row exists");
        assert_eq!(
            before_json, "{\"status_id\":2}",
            "must skip the newer excluded-action row and return the older 'update' row"
        );

        tx.commit().expect("commit");
    }
}
