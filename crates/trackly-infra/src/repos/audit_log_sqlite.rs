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
    /// No caller yet — `ActService::update` (Plan 19-03) is the first and
    /// only caller.
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
             ORDER BY created_at_utc DESC, id DESC LIMIT 1",
            params![act_id, device_id],
            |r| r.get::<_, String>(0),
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
}
