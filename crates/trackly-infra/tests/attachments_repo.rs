//! Integration test: `SqliteAttachmentRepository` (V050, Phase 41.3 Plan 01).
//! Только вымышленные данные.

use rusqlite::Connection;
use tempfile::TempDir;

use trackly_infra::db::migrations;
use trackly_infra::db::pragmas::apply_writer_pragmas;
use trackly_infra::repos::attachments_sqlite::AttachmentNew;
use trackly_infra::repos::SqliteAttachmentRepository;

const NOW: i64 = 1_700_000_000;
const H1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const H2: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn fresh_db() -> (Connection, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let mut conn = Connection::open(dir.path().join("att-repo.db")).expect("open");
    apply_writer_pragmas(&conn).expect("pragmas");
    migrations::run(&mut conn).expect("migrate");
    conn.execute_batch(&format!(
        "INSERT INTO users (id, login, full_name, role, created_at_utc, updated_at_utc)
           VALUES (1, 'ivanov', 'Иванов И.И.', 'admin', {NOW}, {NOW}),
                  (2, 'petrov', 'Петров П.П.', 'admin', {NOW}, {NOW});"
    ))
    .expect("seed users");
    (conn, dir)
}

fn new<'a>(
    kind: &'a str,
    pid: i64,
    hash: &'a str,
    at: i64,
    user: Option<i64>,
) -> AttachmentNew<'a> {
    AttachmentNew {
        parent_kind: kind,
        parent_id: pid,
        original_name: "скан-акта.pdf",
        mime: "application/pdf",
        size_bytes: 100,
        content_hash: hash,
        uploaded_by_user_id: user,
        created_at_utc: at,
    }
}

#[test]
fn insert_and_list_ordered_with_names() {
    let (c, _g) = fresh_db();
    let r = SqliteAttachmentRepository;
    let b = r
        .insert_on(&c, &new("request", 5, H1, NOW + 10, Some(1)))
        .unwrap();
    let a = r.insert_on(&c, &new("request", 5, H2, NOW, None)).unwrap();
    r.insert_on(&c, &new("device", 5, H2, NOW, None)).unwrap();
    let rows = r.list_by_parent_on(&c, "request", 5, false).unwrap();
    assert_eq!(rows.iter().map(|x| x.id).collect::<Vec<_>>(), [a, b]);
    assert_eq!(rows[0].uploaded_by_name, None);
    assert_eq!(rows[1].uploaded_by_name.as_deref(), Some("Иванов И.И."));
    assert_eq!(r.get_by_id_on(&c, b).unwrap().unwrap().id, b);
    assert!(r.get_by_id_on(&c, 999).unwrap().is_none());
}

#[test]
fn soft_delete_restore_idempotent_and_listing() {
    let (c, _g) = fresh_db();
    let r = SqliteAttachmentRepository;
    let id = r
        .insert_on(&c, &new("request", 1, H1, NOW, Some(1)))
        .unwrap();
    assert!(r.soft_delete_on(&c, id, Some(2), NOW + 5).unwrap());
    assert!(!r.soft_delete_on(&c, id, Some(2), NOW + 6).unwrap());
    assert!(r
        .list_by_parent_on(&c, "request", 1, false)
        .unwrap()
        .is_empty());
    let all = r.list_by_parent_on(&c, "request", 1, true).unwrap();
    assert_eq!(all[0].deleted_by_name.as_deref(), Some("Петров П.П."));
    assert_eq!(all[0].deleted_at_utc, Some(NOW + 5));
    assert_eq!(r.count_deleted_with_bytes_on(&c).unwrap(), (1, 100));
    assert!(r.restore_on(&c, id).unwrap());
    assert!(!r.restore_on(&c, id).unwrap());
    assert_eq!(
        r.list_by_parent_on(&c, "request", 1, false).unwrap().len(),
        1
    );
}

#[test]
fn count_by_hash_includes_soft_deleted_and_excludes_id() {
    let (c, _g) = fresh_db();
    let r = SqliteAttachmentRepository;
    let a = r.insert_on(&c, &new("request", 1, H1, NOW, None)).unwrap();
    let b = r.insert_on(&c, &new("device", 2, H1, NOW, None)).unwrap();
    r.soft_delete_on(&c, b, None, NOW).unwrap();
    assert_eq!(r.count_by_hash_on(&c, H1, None).unwrap(), 2);
    assert_eq!(r.count_by_hash_on(&c, H1, Some(a)).unwrap(), 1);
    assert_eq!(r.count_by_hash_on(&c, H2, None).unwrap(), 0);
}

#[test]
fn delete_row_and_totals() {
    let (c, _g) = fresh_db();
    let r = SqliteAttachmentRepository;
    let a = r.insert_on(&c, &new("request", 1, H1, NOW, None)).unwrap();
    let b = r.insert_on(&c, &new("device", 2, H1, NOW, None)).unwrap();
    r.insert_on(&c, &new("device", 2, H2, NOW, None)).unwrap();
    r.soft_delete_on(&c, b, None, NOW).unwrap();
    assert_eq!(
        r.hashes_with_totals_on(&c).unwrap(),
        vec![(H1.to_string(), 2, 200), (H2.to_string(), 1, 100)]
    );
    assert_eq!(r.distinct_hash_sizes_on(&c).unwrap().len(), 2);
    assert!(r.delete_row_on(&c, a).unwrap());
    assert!(!r.delete_row_on(&c, a).unwrap());
    assert_eq!(r.count_by_hash_on(&c, H1, None).unwrap(), 1);
}
