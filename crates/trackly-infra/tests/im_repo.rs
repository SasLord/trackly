//! Интеграционные тесты `SqliteImRepository` (Phase 43 Plan 01, DLV-02/DLV-03).
//! Только вымышленные данные.

use rusqlite::Connection;
use trackly_core::domain::im::member_window;
use trackly_infra::repos::im_sqlite::ImMessageNew;
use trackly_infra::repos::SqliteImRepository;
use trackly_infra::test_support::test_db;

const NOW: i64 = 1_700_000_000;

fn setup() -> (Connection, tempfile::TempDir, i64) {
    let (conn, dir) = test_db();
    conn.execute_batch(&format!(
        "INSERT INTO users (id, login, full_name, role, created_at_utc, updated_at_utc) VALUES
           (1, 'ivanov', 'Иванов И.И.', 'admin', {NOW}, {NOW}),
           (2, 'petrov', 'Петров П.П.', 'admin', {NOW}, {NOW});"
    ))
    .unwrap();
    let repo = SqliteImRepository;
    let conv = repo
        .create_conversation_on(&conn, "room", Some("Комната"), None, 1, NOW)
        .unwrap();
    repo.insert_member_on(&conn, conv, 1, "creator", NOW, 0)
        .unwrap();
    repo.insert_member_on(&conn, conv, 2, "member", NOW, 0)
        .unwrap();
    (conn, dir, conv)
}

fn post(conn: &Connection, conv: i64, author: i64, client: Option<&str>) -> i64 {
    let repo = SqliteImRepository;
    let seq = repo.bump_last_seq_on(conn, conv).unwrap();
    repo.insert_message_on(
        conn,
        &ImMessageNew {
            conversation_id: conv,
            seq,
            rev: seq,
            kind: "user",
            author_user_id: Some(author),
            client_msg_id: client,
            body_json: "{}",
            body_text: "привет",
            created_at_utc: NOW,
        },
    )
    .unwrap()
}

#[test]
fn bump_last_seq_increments_and_rolls_back() {
    let (mut conn, _d, conv) = setup();
    let repo = SqliteImRepository;
    assert_eq!(repo.bump_last_seq_on(&conn, conv).unwrap(), 1);
    assert_eq!(repo.bump_last_seq_on(&conn, conv).unwrap(), 2);
    let tx = conn.transaction().unwrap();
    assert_eq!(repo.bump_last_seq_on(&tx, conv).unwrap(), 3);
    tx.rollback().unwrap();
    assert_eq!(repo.last_seq_on(&conn, conv).unwrap(), 2, "нет дыр в seq");
    assert_eq!(repo.bump_last_seq_on(&conn, conv).unwrap(), 3);
}

#[test]
fn find_by_client_id_matches_only_full_triple() {
    let (conn, _d, conv) = setup();
    let repo = SqliteImRepository;
    post(&conn, conv, 1, Some("c-1"));
    let hit = repo.find_by_client_id_on(&conn, conv, 1, "c-1").unwrap();
    assert_eq!(hit.unwrap().seq, 1);
    assert!(repo
        .find_by_client_id_on(&conn, conv, 2, "c-1")
        .unwrap()
        .is_none());
    assert!(repo
        .find_by_client_id_on(&conn, conv, 1, "c-2")
        .unwrap()
        .is_none());
}

#[test]
fn duplicate_client_id_is_distinguishable_conflict() {
    let (conn, _d, conv) = setup();
    let repo = SqliteImRepository;
    post(&conn, conv, 1, Some("c-1"));
    let seq = repo.bump_last_seq_on(&conn, conv).unwrap();
    let err = repo
        .insert_message_on(
            &conn,
            &ImMessageNew {
                conversation_id: conv,
                seq,
                rev: seq,
                kind: "user",
                author_user_id: Some(1),
                client_msg_id: Some("c-1"),
                body_json: "{}",
                body_text: "дубль",
                created_at_utc: NOW,
            },
        )
        .unwrap_err();
    assert!(SqliteImRepository::is_unique_conflict(&err), "{err:?}");
}

#[test]
fn left_member_is_excluded_from_active_and_keeps_boundary() {
    let (conn, _d, conv) = setup();
    let repo = SqliteImRepository;
    assert_eq!(repo.active_member_ids_on(&conn, conv).unwrap(), vec![1, 2]);
    repo.mark_left_on(&conn, conv, 2, NOW + 5, 4).unwrap();
    assert_eq!(repo.active_member_ids_on(&conn, conv).unwrap(), vec![1]);
    let row = repo.member_row_on(&conn, conv, 2).unwrap().unwrap();
    assert_eq!(row.left_at_utc, Some(NOW + 5));
    assert_eq!(row.left_seq, Some(4));
    // вышедшая беседа остаётся в списке пользователя
    assert_eq!(repo.conversations_of_user_on(&conn, 2).unwrap().len(), 1);
    repo.rejoin_member_on(&conn, conv, 2, 4).unwrap();
    let row = repo.member_row_on(&conn, conv, 2).unwrap().unwrap();
    assert_eq!(
        (row.left_at_utc, row.left_seq, row.visible_from_seq),
        (None, None, 4)
    );
}

#[test]
fn window_hides_edits_after_leaving() {
    let (conn, _d, conv) = setup();
    let repo = SqliteImRepository;
    let m1 = post(&conn, conv, 1, None);
    post(&conn, conv, 1, None);
    let w = member_window(0, Some(2));

    // без правки сообщение возвращается
    let before = repo.messages_window_on(&conn, conv, &w, 50).unwrap();
    assert!(before.iter().any(|m| m.id == m1));

    // после выхода сообщение «правят»: rev поднимается выше left_seq
    conn.execute("UPDATE im_messages SET rev = 9 WHERE id = ?1", [m1])
        .unwrap();
    let after = repo.messages_window_on(&conn, conv, &w, 50).unwrap();
    assert!(!after.iter().any(|m| m.id == m1), "копия заморожена");
    // активный участник правку видит
    let active = member_window(0, None);
    let all = repo.messages_window_on(&conn, conv, &active, 50).unwrap();
    assert!(all.iter().any(|m| m.id == m1));
}

#[test]
fn window_bounds_seq() {
    let (conn, _d, conv) = setup();
    let repo = SqliteImRepository;
    for _ in 0..5 {
        post(&conn, conv, 1, None);
    }
    let w = member_window(2, Some(4));
    let seqs: Vec<i64> = repo
        .messages_window_on(&conn, conv, &w, 50)
        .unwrap()
        .iter()
        .map(|m| m.seq)
        .collect();
    assert_eq!(seqs, vec![2, 3, 4]);
}

#[test]
fn after_rev_and_last_page_and_count() {
    let (conn, _d, conv) = setup();
    let repo = SqliteImRepository;
    for _ in 0..5 {
        post(&conn, conv, 1, None);
    }
    let w = member_window(0, None);
    let revs: Vec<i64> = repo
        .messages_after_rev_on(&conn, conv, 2, &w, 50)
        .unwrap()
        .iter()
        .map(|m| m.rev)
        .collect();
    assert_eq!(revs, vec![3, 4, 5]);
    assert_eq!(repo.count_after_rev_on(&conn, conv, 2, &w).unwrap(), 3);
    let last: Vec<i64> = repo
        .messages_last_page_on(&conn, conv, &w, 2)
        .unwrap()
        .iter()
        .map(|m| m.seq)
        .collect();
    assert_eq!(last, vec![4, 5]);
}
