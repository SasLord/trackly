//! Интеграционные тесты для ImportSessionStore (Plan 05).
//!
//! Проверяем:
//! - put() возвращает UUID-токен, take() возвращает сессию
//! - double-take возвращает None (однократное использование)
//! - TTL истечение (используем force-expire через внутренний хелпер)
//!
//! Каждый тест обёрнут в `tokio::time::timeout(30s)`.

use std::time::Duration;

use trackly_app::csv::session_store::{ImportSession, ImportSessionStore};

fn make_session() -> ImportSession {
    use encoding_rs::UTF_8;
    ImportSession {
        encoding: UTF_8,
        delimiter: b',',
        headers: vec!["Наименование".to_string(), "Тип".to_string()],
        all_rows: vec![
            vec!["Ноутбук Lenovo".to_string(), "Устройство".to_string()],
            vec!["Принтер HP".to_string(), "Устройство".to_string()],
        ],
        created: std::time::Instant::now(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_put_returns_token_and_take_retrieves() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let store = ImportSessionStore::new();
        let session = make_session();
        let expected_headers = session.headers.clone();
        let expected_rows = session.all_rows.clone();
        let expected_delimiter = session.delimiter;

        let token = store.put(session);

        // Token must be a valid UUID v4.
        let parsed = uuid::Uuid::parse_str(&token.to_string());
        assert!(parsed.is_ok(), "token must be a valid UUID, got: {token}");

        // take() must return the session.
        let retrieved = store.take(token);
        assert!(retrieved.is_some(), "take() should return the stored session");

        let s = retrieved.unwrap();
        assert_eq!(s.headers, expected_headers, "headers must match");
        assert_eq!(s.all_rows, expected_rows, "rows must match");
        assert_eq!(s.delimiter, expected_delimiter, "delimiter must match");
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_double_take_returns_none() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let store = ImportSessionStore::new();
        let token = store.put(make_session());

        // First take: should succeed.
        let first = store.take(token);
        assert!(first.is_some(), "first take should return session");

        // Second take with same token: must return None (single-use).
        let second = store.take(token);
        assert!(second.is_none(), "second take with same token should return None");
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_unknown_token_returns_none() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let store = ImportSessionStore::new();
        // Random UUID that was never put.
        let random_token = uuid::Uuid::new_v4();
        let result = store.take(random_token);
        assert!(result.is_none(), "unknown token should return None");
    })
    .await
    .expect("timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_multiple_tokens_independent() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let store = ImportSessionStore::new();

        let mut session_a = make_session();
        session_a.headers = vec!["A".to_string()];

        let mut session_b = make_session();
        session_b.headers = vec!["B".to_string()];

        let token_a = store.put(session_a);
        let token_b = store.put(session_b);

        // Taking B doesn't affect A.
        let b = store.take(token_b);
        assert!(b.is_some());
        assert_eq!(b.unwrap().headers, vec!["B".to_string()]);

        // A is still retrievable.
        let a = store.take(token_a);
        assert!(a.is_some());
        assert_eq!(a.unwrap().headers, vec!["A".to_string()]);
    })
    .await
    .expect("timeout");
}

/// TTL expiry test: creates a session with a `created` time backdated by 6 minutes.
/// This uses the `ImportSession.created` field directly (Instant arithmetic).
///
/// Note: this test does NOT sleep 5 minutes — it backdates the created timestamp.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn session_expired_returns_none() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let store = ImportSessionStore::new();

        // Create a session with a backdated `created` field (6 minutes in the past).
        let mut expired_session = make_session();
        expired_session.created =
            std::time::Instant::now() - Duration::from_secs(6 * 60); // 6 min ago

        let token = store.put(expired_session);

        // take() should return None because the session is expired (TTL = 5 min).
        let result = store.take(token);
        assert!(
            result.is_none(),
            "expired session (created 6 min ago) should return None on take()"
        );
    })
    .await
    .expect("timeout");
}
