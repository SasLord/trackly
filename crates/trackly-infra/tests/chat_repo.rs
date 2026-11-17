//! Integration test: `SqliteChatRepository` (V054, Phase 41.5 Plan 03).
//! Только вымышленные данные.

use rusqlite::Connection;
use tempfile::TempDir;

use trackly_core::error::AppError;
use trackly_infra::db::migrations;
use trackly_infra::db::pragmas::apply_writer_pragmas;
use trackly_infra::repos::attachments_sqlite::AttachmentNew;
use trackly_infra::repos::chat_sqlite::{ChatMessageNew, ReadMark};
use trackly_infra::repos::{SqliteAttachmentRepository, SqliteChatRepository};

const NOW: i64 = 1_700_000_000;
const H: &str = "1111111111111111111111111111111111111111111111111111111111111111";

/// users: 1 Иванов (admin), 2 Петров (employee), 3 Сидоров (employee).
/// requests: 1 (автор 2), 2 (автор 3), 3 ad_register (автор 2), 4 удалена (автор 2).
fn fresh_db() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let mut conn = Connection::open(dir.path().join("chat-repo.db")).expect("open");
    apply_writer_pragmas(&conn).expect("pragmas");
    migrations::run(&mut conn).expect("migrate");
    conn.execute_batch(&format!(
        "INSERT INTO users (id, login, full_name, role, created_at_utc, updated_at_utc)
           VALUES (1, 'ivanov', 'Иванов И.И.', 'admin', {NOW}, {NOW}),
                  (2, 'petrov', 'Петров П.П.', 'employee', {NOW}, {NOW}),
                  (3, 'sidorov', 'Сидоров С.С.', 'employee', {NOW}, {NOW});
         INSERT INTO requests (id, request_type, status, requested_by_user_id, description,
                               created_at_utc, updated_at_utc, deleted_at_utc)
           VALUES (1, 'free_form', 'open', 2, 'вопрос', {NOW}, {NOW}, NULL),
                  (2, 'free_form', 'open', 3, 'вопрос', {NOW}, {NOW}, NULL),
                  (3, 'ad_register', 'open', 2, 'учётка', {NOW}, {NOW}, NULL),
                  (4, 'free_form', 'open', 2, 'удалена', {NOW}, {NOW}, {NOW});"
    ))
    .expect("seed");
    (conn, dir)
}

fn msg(c: &Connection, req: i64, author: Option<i64>, body: &str) -> i64 {
    SqliteChatRepository
        .insert_message_on(
            c,
            &ChatMessageNew {
                request_id: req,
                author_user_id: author,
                body,
                created_at_utc: NOW,
                moved_from_request_id: None,
            },
        )
        .unwrap()
}

fn audit(
    c: &Connection,
    et: &str,
    eid: i64,
    action: &str,
    user: Option<i64>,
    payload: Option<&str>,
) {
    c.execute(
        "INSERT INTO audit_log (entity_type, entity_id, action, user_id, payload_json, created_at_utc)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![et, eid, action, user, payload, NOW],
    )
    .unwrap();
}

fn set_baseline(c: &Connection, v: i64) {
    c.execute(
        "UPDATE app_settings SET value = ?1 WHERE key = 'chat_unread_baseline_audit_id'",
        [v.to_string()],
    )
    .unwrap();
}

fn draft(c: &Connection, parent_id: i64, user: i64) -> i64 {
    SqliteAttachmentRepository
        .insert_on(
            c,
            &AttachmentNew {
                parent_kind: "message_draft",
                parent_id,
                original_name: "фото.png",
                mime: "image/png",
                size_bytes: 10,
                content_hash: H,
                uploaded_by_user_id: Some(user),
                created_at_utc: NOW,
                message_id: None,
            },
        )
        .unwrap()
}

#[test]
fn messages_roundtrip_with_author_and_order() {
    let (c, _g) = fresh_db();
    let r = SqliteChatRepository;
    let a = msg(&c, 1, Some(2), "Принтер снова мнёт бумагу");
    let b = r
        .insert_message_on(
            &c,
            &ChatMessageNew {
                request_id: 1,
                author_user_id: Some(1),
                body: "Посмотрим",
                created_at_utc: NOW + 1,
                moved_from_request_id: Some(9),
            },
        )
        .unwrap();
    msg(&c, 2, Some(3), "другая заявка");
    let one = r.get_message_on(&c, a).unwrap().unwrap();
    assert_eq!(one.author_name.as_deref(), Some("Петров П.П."));
    assert_eq!(one.author_role.as_deref(), Some("employee"));
    let list = r.list_messages_on(&c, 1).unwrap();
    assert_eq!(list.iter().map(|m| m.id).collect::<Vec<_>>(), [a, b]);
    assert_eq!(list[1].moved_from_request_id, Some(9));
    assert!(r.get_message_on(&c, 999).unwrap().is_none());
}

#[test]
fn events_filter() {
    let (c, _g) = fresh_db();
    let r = SqliteChatRepository;
    audit(&c, "request", 1, "update", Some(1), None);
    audit(&c, "request", 1, "list", Some(1), None);
    audit(&c, "request", 1, "get", Some(1), None);
    audit(&c, "request", 1, "custom:attachment_added", Some(1), None);
    audit(&c, "request", 1, "custom:attachment_deleted", Some(1), None);
    audit(
        &c,
        "request",
        1,
        "custom:something",
        Some(1),
        Some("{\"message_id\": 5}"),
    );
    audit(&c, "request", 2, "update", Some(1), None);
    audit(&c, "device", 1, "update", Some(1), None);
    audit(
        &c,
        "request",
        1,
        "custom:status",
        Some(1),
        Some("{\"to\":\"x\"}"),
    );
    let ev = r.list_events_on(&c, 1).unwrap();
    let actions: Vec<_> = ev.iter().map(|e| e.action.as_str()).collect();
    assert_eq!(actions, ["update", "custom:status"]);
    assert_eq!(ev[0].user_name.as_deref(), Some("Иванов И.И."));
}

#[test]
fn message_attachment_ids_include_deleted() {
    let (c, _g) = fresh_db();
    let a = SqliteAttachmentRepository;
    let r = SqliteChatRepository;
    let mk = |msg_id: Option<i64>| AttachmentNew {
        parent_kind: "request",
        parent_id: 1,
        original_name: "f.png",
        mime: "image/png",
        size_bytes: 1,
        content_hash: H,
        uploaded_by_user_id: Some(1),
        created_at_utc: NOW,
        message_id: msg_id,
    };
    let plain = a.insert_on(&c, &mk(None)).unwrap();
    let m1 = a.insert_on(&c, &mk(Some(1))).unwrap();
    let m2 = a.insert_on(&c, &mk(Some(2))).unwrap();
    a.soft_delete_on(&c, m2, Some(1), NOW).unwrap();
    let ids = r.message_attachment_ids_on(&c, 1).unwrap();
    assert!(ids.contains(&m1) && ids.contains(&m2) && !ids.contains(&plain));
    let listed = a.list_message_attachments_on(&c, 1).unwrap();
    assert_eq!(listed.iter().map(|x| x.id).collect::<Vec<_>>(), [m1, m2]);
}

#[test]
fn read_marks_upsert_and_effective() {
    let (c, _g) = fresh_db();
    let r = SqliteChatRepository;
    set_baseline(&c, 7);
    let e = r.effective_read_on(&c, 1, 1).unwrap();
    assert_eq!((e.last_message_id, e.last_audit_id), (0, 7));
    assert!(r.get_read_on(&c, 1, 1).unwrap().is_none());
    let m = ReadMark {
        last_message_id: 3,
        last_audit_id: 9,
    };
    r.upsert_read_on(&c, 1, 1, m, NOW).unwrap();
    assert_eq!(r.effective_read_on(&c, 1, 1).unwrap(), m);
    let m2 = ReadMark {
        last_message_id: 4,
        last_audit_id: 10,
    };
    r.upsert_read_on(&c, 1, 1, m2, NOW + 1).unwrap();
    assert_eq!(r.get_read_on(&c, 1, 1).unwrap().unwrap(), m2);
    let n: i64 = c
        .query_row("SELECT COUNT(*) FROM chat_reads", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1);
}

#[test]
fn max_ids_empty_and_filled() {
    let (c, _g) = fresh_db();
    let r = SqliteChatRepository;
    assert_eq!(r.max_ids_on(&c, 1).unwrap(), (0, 0));
    let m = msg(&c, 1, Some(2), "x");
    audit(&c, "request", 1, "update", Some(1), None);
    let (mm, ma) = r.max_ids_on(&c, 1).unwrap();
    assert_eq!(mm, m);
    assert!(ma > 0);
}

#[test]
fn unread_own_vs_foreign_and_read() {
    let (c, _g) = fresh_db();
    let r = SqliteChatRepository;
    set_baseline(&c, 0);
    msg(&c, 1, Some(1), "чужое");
    msg(&c, 1, Some(2), "своё");
    audit(&c, "request", 1, "update", Some(1), None);
    audit(&c, "request", 1, "update", Some(2), None);
    assert_eq!(r.unread_summary_on(&c, 2, false).unwrap(), [(1, 2)]);
    let (mm, ma) = r.max_ids_on(&c, 1).unwrap();
    r.upsert_read_on(
        &c,
        2,
        1,
        ReadMark {
            last_message_id: mm,
            last_audit_id: ma,
        },
        NOW,
    )
    .unwrap();
    assert!(r.unread_summary_on(&c, 2, false).unwrap().is_empty());
}

#[test]
fn unread_file_message_not_double_counted() {
    let (c, _g) = fresh_db();
    let r = SqliteChatRepository;
    set_baseline(&c, 0);
    audit(
        &c,
        "request",
        1,
        "custom:attachment_added",
        Some(1),
        Some("{\"message_id\": 5}"),
    );
    assert!(r.unread_summary_on(&c, 2, false).unwrap().is_empty());
    audit(
        &c,
        "request",
        1,
        "custom:attachment_added",
        Some(1),
        Some("{\"name\":\"a\"}"),
    );
    assert_eq!(r.unread_summary_on(&c, 2, false).unwrap(), [(1, 1)]);
}

#[test]
fn unread_baseline_watermark() {
    let (c, _g) = fresh_db();
    let r = SqliteChatRepository;
    audit(&c, "request", 1, "update", Some(1), None);
    audit(&c, "request", 1, "update", Some(1), None);
    let (_, max) = r.max_ids_on(&c, 1).unwrap();
    set_baseline(&c, max - 1);
    assert_eq!(r.unread_summary_on(&c, 2, false).unwrap(), [(1, 1)]);
    set_baseline(&c, max);
    assert!(r.unread_summary_on(&c, 2, false).unwrap().is_empty());
}

#[test]
fn unread_visibility_staff_vs_employee() {
    let (c, _g) = fresh_db();
    let r = SqliteChatRepository;
    set_baseline(&c, 0);
    msg(&c, 1, Some(1), "а");
    msg(&c, 2, Some(1), "б");
    msg(&c, 3, Some(1), "ad");
    msg(&c, 4, Some(1), "удалена");
    // Staff видит все живые не-ad_register заявки.
    assert_eq!(r.unread_summary_on(&c, 3, true).unwrap(), [(1, 1), (2, 1)]);
    // Employee Сидоров: только заявка 2.
    assert_eq!(r.unread_summary_on(&c, 3, false).unwrap(), [(2, 1)]);
    // Участник заявки 1 видит её.
    c.execute(
        "INSERT INTO request_participants (request_id, user_id, joined_at_utc) VALUES (1, 3, ?1)",
        [NOW],
    )
    .unwrap();
    assert_eq!(r.unread_summary_on(&c, 3, false).unwrap(), [(1, 1), (2, 1)]);
}

#[test]
fn promote_drafts_valid_and_tampering() {
    let (c, _g) = fresh_db();
    let r = SqliteChatRepository;
    let ok = draft(&c, 1, 2);
    let foreign = draft(&c, 1, 3);
    let other_req = draft(&c, 2, 2);
    let done = draft(&c, 1, 2);
    r.promote_drafts_on(&c, 1, 2, &[done], 11).unwrap();
    let deleted = draft(&c, 1, 2);
    c.execute(
        "UPDATE attachments SET deleted_at_utc = ?1 WHERE id = ?2",
        [NOW, deleted],
    )
    .unwrap();

    let rows = r.promote_drafts_on(&c, 1, 2, &[ok], 12).unwrap();
    assert_eq!(rows[0].parent_kind, "request");
    assert_eq!(rows[0].parent_id, 1);
    assert_eq!(rows[0].message_id, Some(12));

    for bad in [foreign, other_req, done, deleted] {
        let fresh = draft(&c, 1, 2);
        c.execute_batch("SAVEPOINT t").unwrap();
        let res = r.promote_drafts_on(&c, 1, 2, &[fresh, bad], 13);
        assert!(matches!(res, Err(AppError::Forbidden)), "bad id {bad}");
        c.execute_batch("ROLLBACK TO t; RELEASE t").unwrap();
        let kind: String = c
            .query_row(
                "SELECT parent_kind FROM attachments WHERE id = ?1",
                [fresh],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kind, "message_draft", "откат: валидный черновик не повышен");
    }
}

#[test]
fn list_and_delete_drafts() {
    let (c, _g) = fresh_db();
    let r = SqliteChatRepository;
    let mine = draft(&c, 1, 2);
    draft(&c, 1, 3);
    draft(&c, 2, 2);
    let list = r.list_drafts_on(&c, 1, 2).unwrap();
    assert_eq!(list.iter().map(|x| x.id).collect::<Vec<_>>(), [mine]);
    r.delete_draft_rows_on(&c, &[mine]).unwrap();
    assert!(r.list_drafts_on(&c, 1, 2).unwrap().is_empty());
}
