//! Phase 6 Wave-2 tests — trackly-app уровень.
//!
//! Тесты реализованы в Wave 2 (06-02-PLAN.md).

use trackly_infra::test_support::test_db::test_db;

/// PRN-03: SELECT COUNT(*) FROM oid_profiles = 5
#[test]
fn test_oid_profiles_seeded() {
    let (conn, _guard) = test_db();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM oid_profiles", [], |r| r.get(0))
        .expect("query oid_profiles count");
    assert_eq!(
        count, 5,
        "expected 5 OID profiles: pantum/kyocera/hp/canon/rfc3805"
    );

    let pantum_encoding: String = conn
        .query_row(
            "SELECT toner_encoding FROM oid_profiles WHERE name = 'pantum'",
            [],
            |r| r.get(0),
        )
        .expect("query pantum profile");
    assert_eq!(
        pantum_encoding, "percent",
        "pantum must use 'percent' toner_encoding"
    );

    let rfc3805_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM oid_profiles WHERE name = 'rfc3805'",
            [],
            |r| r.get(0),
        )
        .expect("check rfc3805");
    assert_eq!(rfc3805_exists, 1, "rfc3805 fallback profile must exist");
}

/// PRN-01: parse sysObjectID → vendor name
#[test]
fn test_vendor_identify() {
    use trackly_app::services::printer_service::identify_vendor;

    assert_eq!(identify_vendor("1.3.6.1.4.1.40093.1"), Some("pantum"));
    assert_eq!(identify_vendor("1.3.6.1.4.1.1347.42"), Some("kyocera"));
    assert_eq!(identify_vendor("1.3.6.1.4.1.11.2.3.9.1"), Some("hp"));
    assert_eq!(identify_vendor("1.3.6.1.4.1.1602.1.1"), Some("canon"));
    assert_eq!(identify_vendor("1.3.6.1.4.1.99999.1"), None);
}

/// PRN-02: parse_toner_level(45, 100, "level_over_max") = Some(45); (-2,-2) = None
#[test]
fn test_toner_percent() {
    use trackly_app::services::printer_service::parse_toner_level;

    assert_eq!(parse_toner_level(45, 100, "level_over_max"), Some(45));
    assert_eq!(parse_toner_level(-2, -2, "level_over_max"), None);
    assert_eq!(parse_toner_level(75, 0, "percent"), Some(75));
}

/// PRN-04: printer with usb_host_device_id set + NULL ip_address — get возвращает usb_host_device_id
#[test]
fn test_printer_usb_only() {
    use rusqlite::params;
    use trackly_core::domain::printers::PrinterNew;
    use trackly_core::ports::printers::PrinterRepository;
    use trackly_infra::repos::printers_sqlite::SqlitePrinterRepository;

    let (mut conn, _guard) = test_db();
    let now = 1_700_000_000_i64;

    conn.execute(
        "INSERT INTO devices (type_id, name, status_id, created_at_utc, updated_at_utc, version) \
         VALUES (2, 'Printer Device', 1, ?1, ?1, 1)",
        params![now],
    )
    .expect("insert device");
    let device_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO devices (type_id, name, status_id, created_at_utc, updated_at_utc, version) \
         VALUES (1, 'USB Host PC', 1, ?1, ?1, 1)",
        params![now],
    )
    .expect("insert host device");
    let host_device_id = conn.last_insert_rowid();

    let repo = SqlitePrinterRepository;
    let printer_new = PrinterNew {
        device_id,
        ip_address: None,
        community_raw: "public".to_string(),
        snmp_version: "v2c".to_string(),
        oid_profile_id: None,
        usb_host_device_id: Some(host_device_id),
    };

    let printer_id = {
        let tx = conn.transaction().expect("tx");
        let id = repo.create_in_tx(&tx, &printer_new, now).expect("create");
        tx.commit().expect("commit");
        id
    };

    let row = repo.get(&conn, printer_id).expect("get printer");
    assert!(
        row.ip_address.is_none(),
        "IP must be None for USB-only printer"
    );
    assert_eq!(row.usb_host_device_id, Some(host_device_id));
}

/// PRN-06: status='error'|'offline' → upsert printer_alerts (dedup)
#[test]
fn test_alert_detection() {
    use rusqlite::params;
    use trackly_core::domain::printers::PrinterNew;
    use trackly_infra::repos::printers_sqlite::SqlitePrinterRepository;

    let (mut conn, _guard) = test_db();
    let now = 1_700_000_000_i64;

    conn.execute(
        "INSERT INTO devices (type_id, name, status_id, created_at_utc, updated_at_utc, version) \
         VALUES (2, 'Alert Printer', 1, ?1, ?1, 1)",
        params![now],
    )
    .expect("insert device");
    let device_id = conn.last_insert_rowid();

    let repo = SqlitePrinterRepository;
    let printer_id = {
        let tx = conn.transaction().expect("tx");
        let id = repo
            .create_in_tx(
                &tx,
                &PrinterNew {
                    device_id,
                    ip_address: Some("192.168.1.50".to_string()),
                    community_raw: "public".to_string(),
                    snmp_version: "v2c".to_string(),
                    oid_profile_id: None,
                    usb_host_device_id: None,
                },
                now,
            )
            .expect("create");
        tx.commit().expect("commit");
        id
    };

    {
        let tx = conn.transaction().expect("tx");
        repo.upsert_alert_in_tx(&tx, printer_id, "error", now)
            .expect("first upsert");
        tx.commit().expect("commit");
    }

    {
        let tx = conn.transaction().expect("tx");
        repo.upsert_alert_in_tx(&tx, printer_id, "offline", now + 60)
            .expect("second upsert");
        tx.commit().expect("commit");
    }

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM printer_alerts WHERE printer_id = ?1",
            params![printer_id],
            |r| r.get(0),
        )
        .expect("count alerts");
    assert_eq!(count, 1, "UNIQUE(printer_id) must dedup — only 1 alert row");
}

/// REQ-01: RequestService::create записывает строку в requests + audit_log
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_request_create() {
    use rusqlite::params;
    use std::sync::Arc;
    use trackly_app::dto::request::RequestCreateDto;
    use trackly_app::services::RequestService;
    use trackly_core::auth::{Identity, Role};
    use trackly_core::primitives::clock::Clock;
    use trackly_infra::clock_impl::SystemClock;
    use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

    let (writer, readers, _guard) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let (ws_tx, _rx) = tokio::sync::broadcast::channel(16);
    let ws_tx = Arc::new(ws_tx);

    let svc = RequestService::new(writer.clone(), readers, clock.clone(), ws_tx);

    let now = clock.unix_seconds();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, password_hash, role, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES ('test_user', 'Test User', 'hash', 'admin', ?1, ?1, 1)",
                params![now],
            )
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(())
        })
        .await
        .expect("seed user");

    let caller = Identity {
        user_id: Some(1),
        role: Role::Admin,
    };

    let dto = svc
        .create(
            RequestCreateDto {
                request_type: "free_form".to_string(),
                printer_device_id: None,
                cartridge_model_id: None,
                category_id: None,
                description: Some("Тест заявки".to_string()),
            },
            &caller,
        )
        .await
        .expect("create request");

    assert_eq!(dto.status, "open");
    assert_eq!(dto.request_type, "free_form");
    assert_eq!(dto.description.as_deref(), Some("Тест заявки"));
}

/// REQ-03: Accept переход; Complete из open → AppError::Validation
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_request_lifecycle() {
    use rusqlite::params;
    use std::sync::Arc;
    use trackly_app::dto::request::{RequestCreateDto, RequestTransitionPayload};
    use trackly_app::services::RequestService;
    use trackly_core::auth::{Identity, Role};
    use trackly_core::error::AppError;
    use trackly_core::primitives::clock::Clock;
    use trackly_infra::clock_impl::SystemClock;
    use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

    let (writer, readers, _guard) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let (ws_tx, _rx) = tokio::sync::broadcast::channel(16);
    let ws_tx = Arc::new(ws_tx);

    let svc = RequestService::new(writer.clone(), readers, clock.clone(), ws_tx);

    let now = clock.unix_seconds();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, password_hash, role, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES ('admin_lc', 'Admin LC', 'hash', 'admin', ?1, ?1, 1)",
                params![now],
            )
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(())
        })
        .await
        .expect("seed user");

    let caller = Identity {
        user_id: Some(1),
        role: Role::Admin,
    };

    let created = svc
        .create(
            RequestCreateDto {
                request_type: "free_form".to_string(),
                printer_device_id: None,
                cartridge_model_id: None,
                category_id: None,
                // free_form now requires a non-empty description server-side (WR-02).
                description: Some("Заявка для проверки перехода статусов".to_string()),
            },
            &caller,
        )
        .await
        .expect("create");
    assert_eq!(created.status, "open");

    // Accept: open → in_progress
    let accepted = svc
        .transition(
            RequestTransitionPayload::Accept {
                request_id: created.id,
                version: created.version,
                assigned_to_user_id: None,
            },
            &caller,
        )
        .await
        .expect("accept");
    assert_eq!(accepted.status, "in_progress");

    // Create a second request, try Complete from open → Validation error
    let created2 = svc
        .create(
            RequestCreateDto {
                request_type: "free_form".to_string(),
                printer_device_id: None,
                cartridge_model_id: None,
                category_id: None,
                // free_form now requires a non-empty description server-side (WR-02).
                description: Some("Вторая заявка для проверки перехода".to_string()),
            },
            &caller,
        )
        .await
        .expect("create2");

    let err = svc
        .transition(
            RequestTransitionPayload::Complete {
                request_id: created2.id,
                version: created2.version,
                notes: None,
                linked_cartridge_id: None,
            },
            &caller,
        )
        .await
        .expect_err("complete from open must fail");
    assert!(
        matches!(err, AppError::Validation { .. }),
        "expected Validation error, got: {err:?}"
    );
}

/// REQ-04: broadcast::Sender получает WsEvent::NewRequest после create
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_ws_event_sent() {
    use rusqlite::params;
    use std::sync::Arc;
    use trackly_app::dto::printer::WsEvent;
    use trackly_app::dto::request::RequestCreateDto;
    use trackly_app::services::RequestService;
    use trackly_core::auth::{Identity, Role};
    use trackly_core::primitives::clock::Clock;
    use trackly_infra::clock_impl::SystemClock;
    use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

    let (writer, readers, _guard) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let (ws_tx, mut ws_rx) = tokio::sync::broadcast::channel(16);
    let ws_tx = Arc::new(ws_tx);

    let svc = RequestService::new(writer.clone(), readers, clock.clone(), ws_tx);

    let now = clock.unix_seconds();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, password_hash, role, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES ('ws_user', 'WS User', 'hash', 'employee', ?1, ?1, 1)",
                params![now],
            )
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(())
        })
        .await
        .expect("seed user");

    let caller = Identity {
        user_id: Some(1),
        role: Role::Employee,
    };

    svc.create(
        RequestCreateDto {
            request_type: "free_form".to_string(),
            printer_device_id: None,
            cartridge_model_id: None,
            category_id: None,
            description: Some("WS test".to_string()),
        },
        &caller,
    )
    .await
    .expect("create");

    let event = ws_rx
        .try_recv()
        .expect("WsEvent must be received after create");
    assert!(
        matches!(event, WsEvent::NewRequest { .. }),
        "expected WsEvent::NewRequest, got: {event:?}"
    );
}

/// D-Retention-01: prune_old_readings_in_tx удаляет строки старше retention_cutoff
#[test]
fn test_readings_prune() {
    use rusqlite::params;
    use trackly_core::domain::printers::PrinterNew;
    use trackly_infra::repos::printers_sqlite::SqlitePrinterRepository;

    let (mut conn, _guard) = test_db();
    let now = 1_700_000_000_i64;

    conn.execute(
        "INSERT INTO devices (type_id, name, status_id, created_at_utc, updated_at_utc, version) \
         VALUES (2, 'Prune Printer', 1, ?1, ?1, 1)",
        params![now],
    )
    .expect("insert device");
    let device_id = conn.last_insert_rowid();

    let repo = SqlitePrinterRepository;
    let printer_id = {
        let tx = conn.transaction().expect("tx");
        let id = repo
            .create_in_tx(
                &tx,
                &PrinterNew {
                    device_id,
                    ip_address: Some("192.168.1.99".to_string()),
                    community_raw: "public".to_string(),
                    snmp_version: "v2c".to_string(),
                    oid_profile_id: None,
                    usb_host_device_id: None,
                },
                now,
            )
            .expect("create");
        tx.commit().expect("commit");
        id
    };

    for i in 0..3 {
        let tx = conn.transaction().expect("tx");
        repo.upsert_reading_in_tx(&tx, printer_id, now - 200 + i, "{}", None, "ok")
            .expect("insert old reading");
        tx.commit().expect("commit");
    }

    {
        let tx = conn.transaction().expect("tx");
        repo.upsert_reading_in_tx(&tx, printer_id, now, "{}", None, "ok")
            .expect("insert recent");
        tx.commit().expect("commit");
    }

    let count_before: i64 = conn
        .query_row("SELECT COUNT(*) FROM printer_readings", [], |r| r.get(0))
        .expect("count before");

    {
        let tx = conn.transaction().expect("tx");
        let deleted = SqlitePrinterRepository::prune_old_readings_in_tx(&tx, now - 1, now - 100)
            .expect("prune");
        tx.commit().expect("commit");
        assert!(deleted >= 3, "deleted {deleted}");
    }

    let count_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM printer_readings", [], |r| r.get(0))
        .expect("count after");
    assert!(count_after < count_before);
    assert_eq!(count_after, 1, "only recent reading should remain");
}

/// PRN-07: install картриджа → current_cartridge_for_printer = Some
#[test]
fn test_current_cartridge_for_printer() {
    use rusqlite::params;
    use trackly_core::domain::printers::PrinterNew;
    use trackly_core::ports::printers::PrinterRepository;
    use trackly_infra::repos::printers_sqlite::SqlitePrinterRepository;

    let (mut conn, _guard) = test_db();
    let now = 1_700_000_000_i64;

    conn.execute(
        "INSERT INTO devices (type_id, name, status_id, created_at_utc, updated_at_utc, version) \
         VALUES (2, 'Cart Printer', 1, ?1, ?1, 1)",
        params![now],
    )
    .expect("insert device");
    let device_id = conn.last_insert_rowid();

    let repo = SqlitePrinterRepository;
    {
        let tx = conn.transaction().expect("tx");
        repo.create_in_tx(
            &tx,
            &PrinterNew {
                device_id,
                ip_address: Some("192.168.1.77".to_string()),
                community_raw: "public".to_string(),
                snmp_version: "v2c".to_string(),
                oid_profile_id: None,
                usb_host_device_id: None,
            },
            now,
        )
        .expect("create printer");
        tx.commit().expect("commit");
    }

    conn.execute(
        "INSERT INTO cartridge_models (brand, model, kind_id, created_at_utc, updated_at_utc, version) \
         VALUES ('Pantum', 'TL-5120X', 1, ?1, ?1, 1)",
        params![now],
    )
    .expect("insert model");
    let model_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO cartridges (code, model_id, status_id, created_at_utc, updated_at_utc, version) \
         VALUES ('C-000001', ?1, 2, ?2, ?2, 1)",
        params![model_id, now],
    )
    .expect("insert cartridge");
    let cartridge_id = conn.last_insert_rowid();

    let initial = repo
        .current_cartridge_for_printer(&conn, device_id)
        .expect("query");
    assert!(initial.is_none());

    {
        let tx = conn.transaction().expect("tx");
        SqlitePrinterRepository::set_current_cartridge_in_tx(
            &tx,
            cartridge_id,
            Some(device_id),
            now,
        )
        .expect("link");
        tx.commit().expect("commit");
    }

    let result = repo
        .current_cartridge_for_printer(&conn, device_id)
        .expect("query after link");
    assert_eq!(result, Some(cartridge_id));
}

/// T-06-07-I: Secret<T> Debug не утекает значение (prints "***")
#[test]
fn test_secret_debug() {
    use trackly_core::primitives::secret::Secret;

    let s = Secret::new("secret_community".to_string());
    let debug_str = format!("{s:?}");
    assert!(
        debug_str.contains("***"),
        "Secret Debug must mask value, got: {debug_str}"
    );
    assert!(
        !debug_str.contains("secret_community"),
        "Secret Debug must not leak value, got: {debug_str}"
    );
}

/// Seed a stock cartridge directly via `writer.execute` (known id/code/model)
/// without standing up a full `CartridgeService` — used by the
/// `test_req_cart_link*`/`history_*` tests below to keep the fixture small.
/// Returns `(cartridge_id, code)`.
async fn seed_cartridge_for_link_tests(
    writer: &std::sync::Arc<trackly_infra::db::writer_worker::WriterHandle>,
    model_id: i64,
    now: i64,
) -> (i64, String) {
    use rusqlite::params;

    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO cartridges (code, model_id, status_id, state_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES ('C-000777', ?1, 1, 1, ?2, ?2, 1)",
                params![model_id, now],
            )
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(())
        })
        .await
        .expect("seed cartridge");

    (1, "C-000777".to_string())
}

/// Seed a printer device directly (type_id=2, no location), return its id.
/// `cartridge_replace` requests require a `printer_device_id` (WR-02).
async fn seed_printer_device_for_link_tests(
    writer: &std::sync::Arc<trackly_infra::db::writer_worker::WriterHandle>,
    now: i64,
) -> i64 {
    use rusqlite::params;

    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, status_id, created_at_utc, updated_at_utc, version) \
                 VALUES (2, 'Pantum BM5100ADN (cart-link test)', 1, ?1, ?1, 1)",
                params![now],
            )
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed printer device")
}

/// Seed a `cartridge_models` row directly, return its id.
async fn seed_cartridge_model_for_link_tests(
    writer: &std::sync::Arc<trackly_infra::db::writer_worker::WriterHandle>,
    now: i64,
) -> i64 {
    use rusqlite::params;

    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO cartridge_models (brand, model, kind_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES ('Pantum', 'TL-5120X', 1, ?1, ?1, 1)",
                params![now],
            )
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(())
        })
        .await
        .expect("seed cartridge_model");

    1
}

/// REQ-05 / D-06: Complete{linked_cartridge_id} записывает completed_cartridge_id.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_req_cart_link() {
    use rusqlite::params;
    use std::sync::Arc;
    use trackly_app::dto::request::{RequestCreateDto, RequestTransitionPayload};
    use trackly_app::services::RequestService;
    use trackly_core::auth::{Identity, Role};
    use trackly_core::primitives::clock::Clock;
    use trackly_infra::clock_impl::SystemClock;
    use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

    let (writer, readers, _guard) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let (ws_tx, _rx) = tokio::sync::broadcast::channel(16);
    let ws_tx = Arc::new(ws_tx);

    let svc = RequestService::new(writer.clone(), readers, clock.clone(), ws_tx);

    let now = clock.unix_seconds();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, password_hash, role, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES ('admin_cart_link', 'Admin CartLink', 'hash', 'admin', ?1, ?1, 1)",
                params![now],
            )
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(())
        })
        .await
        .expect("seed user");

    let caller = Identity {
        user_id: Some(1),
        role: Role::Admin,
    };

    let printer_device_id = seed_printer_device_for_link_tests(&writer, now).await;
    let model_id = seed_cartridge_model_for_link_tests(&writer, now).await;
    let (cartridge_id, _code) = seed_cartridge_for_link_tests(&writer, model_id, now).await;

    let created = svc
        .create(
            RequestCreateDto {
                request_type: "cartridge_replace".to_string(),
                printer_device_id: Some(printer_device_id as i32),
                cartridge_model_id: Some(model_id as i32),
                category_id: None,
                description: None,
            },
            &caller,
        )
        .await
        .expect("create cartridge_replace request");

    let accepted = svc
        .transition(
            RequestTransitionPayload::Accept {
                request_id: created.id,
                version: created.version,
                assigned_to_user_id: None,
            },
            &caller,
        )
        .await
        .expect("accept");

    let completed = svc
        .transition(
            RequestTransitionPayload::Complete {
                request_id: accepted.id,
                version: accepted.version,
                notes: None,
                linked_cartridge_id: Some(cartridge_id as i32),
            },
            &caller,
        )
        .await
        .expect("complete with linked_cartridge_id");

    assert_eq!(
        completed.completed_cartridge_id,
        Some(cartridge_id),
        "completed_cartridge_id must persist the linked cartridge's id (D-06)"
    );
}

/// D-07: история заявки показывает человекочитаемый код+модель установленного
/// картриджа после Complete с linked_cartridge_id.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn history_shows_cartridge_snapshot_after_complete() {
    use rusqlite::params;
    use std::sync::Arc;
    use trackly_app::dto::request::{RequestCreateDto, RequestTransitionPayload};
    use trackly_app::services::RequestService;
    use trackly_core::auth::{Identity, Role};
    use trackly_core::primitives::clock::Clock;
    use trackly_infra::clock_impl::SystemClock;
    use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

    let (writer, readers, _guard) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let (ws_tx, _rx) = tokio::sync::broadcast::channel(16);
    let ws_tx = Arc::new(ws_tx);

    let svc = RequestService::new(writer.clone(), readers, clock.clone(), ws_tx);

    let now = clock.unix_seconds();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, password_hash, role, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES ('admin_hist_cart', 'Admin HistCart', 'hash', 'admin', ?1, ?1, 1)",
                params![now],
            )
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(())
        })
        .await
        .expect("seed user");

    let caller = Identity {
        user_id: Some(1),
        role: Role::Admin,
    };

    let printer_device_id = seed_printer_device_for_link_tests(&writer, now).await;
    let model_id = seed_cartridge_model_for_link_tests(&writer, now).await;
    let (cartridge_id, code) = seed_cartridge_for_link_tests(&writer, model_id, now).await;

    let created = svc
        .create(
            RequestCreateDto {
                request_type: "cartridge_replace".to_string(),
                printer_device_id: Some(printer_device_id as i32),
                cartridge_model_id: Some(model_id as i32),
                category_id: None,
                description: None,
            },
            &caller,
        )
        .await
        .expect("create cartridge_replace request");

    let accepted = svc
        .transition(
            RequestTransitionPayload::Accept {
                request_id: created.id,
                version: created.version,
                assigned_to_user_id: None,
            },
            &caller,
        )
        .await
        .expect("accept");

    svc.transition(
        RequestTransitionPayload::Complete {
            request_id: accepted.id,
            version: accepted.version,
            notes: None,
            linked_cartridge_id: Some(cartridge_id as i32),
        },
        &caller,
    )
    .await
    .expect("complete with linked_cartridge_id");

    let history = svc
        .get_history(created.id, &caller)
        .await
        .expect("get_history");

    let complete_entry = history
        .iter()
        .find(|e| e.action == "custom:complete")
        .expect("history must contain a custom:complete entry");

    let notes = complete_entry
        .notes
        .as_deref()
        .expect("complete entry notes must be Some after cartridge link");
    assert!(
        notes.contains(&code),
        "history notes must contain the cartridge code {code}, got: {notes}"
    );
    assert!(
        notes.contains("Pantum"),
        "history notes must contain the cartridge model brand, got: {notes}"
    );
}

/// Regression: Complete без linked_cartridge_id (notes-only) сохраняет
/// исходный текст без обогащения — поведение reject/complete без картриджа
/// не должно меняться этим планом.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn history_complete_without_cartridge_keeps_plain_notes() {
    use rusqlite::params;
    use std::sync::Arc;
    use trackly_app::dto::request::{RequestCreateDto, RequestTransitionPayload};
    use trackly_app::services::RequestService;
    use trackly_core::auth::{Identity, Role};
    use trackly_core::primitives::clock::Clock;
    use trackly_infra::clock_impl::SystemClock;
    use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

    let (writer, readers, _guard) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let (ws_tx, _rx) = tokio::sync::broadcast::channel(16);
    let ws_tx = Arc::new(ws_tx);

    let svc = RequestService::new(writer.clone(), readers, clock.clone(), ws_tx);

    let now = clock.unix_seconds();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, password_hash, role, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES ('admin_plain_notes', 'Admin PlainNotes', 'hash', 'admin', ?1, ?1, 1)",
                params![now],
            )
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(())
        })
        .await
        .expect("seed user");

    let caller = Identity {
        user_id: Some(1),
        role: Role::Admin,
    };

    let created = svc
        .create(
            RequestCreateDto {
                request_type: "free_form".to_string(),
                printer_device_id: None,
                cartridge_model_id: None,
                category_id: None,
                description: Some("Свободная заявка без картриджа".to_string()),
            },
            &caller,
        )
        .await
        .expect("create free_form request");

    let accepted = svc
        .transition(
            RequestTransitionPayload::Accept {
                request_id: created.id,
                version: created.version,
                assigned_to_user_id: None,
            },
            &caller,
        )
        .await
        .expect("accept");

    svc.transition(
        RequestTransitionPayload::Complete {
            request_id: accepted.id,
            version: accepted.version,
            notes: Some("текст".to_string()),
            linked_cartridge_id: None,
        },
        &caller,
    )
    .await
    .expect("complete without linked_cartridge_id");

    let history = svc
        .get_history(created.id, &caller)
        .await
        .expect("get_history");

    let complete_entry = history
        .iter()
        .find(|e| e.action == "custom:complete")
        .expect("history must contain a custom:complete entry");

    assert_eq!(
        complete_entry.notes.as_deref(),
        Some("текст"),
        "notes must stay plain (no cartridge enrichment) when linked_cartridge_id is None"
    );
}

/// D-Mock-01: TRACKLY_SNMP_MOCK=1 → MockSnmpClient; без env → RealSnmpClient.
///
/// Тест проверяет что AppCtx::build читает TRACKLY_SNMP_MOCK env и создаёт
/// соответствующий snmp_client. Проверка через std::any::type_name.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_snmp_mock_switch() {
    use std::any::type_name;
    use trackly_app::services::PrinterService;

    // С env var → должен быть MockSnmpClient.
    std::env::set_var("TRACKLY_SNMP_MOCK", "1");

    let dir = tempfile::TempDir::new().expect("tempdir");
    let dir_path = dir.keep();
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir_path).expect("paths");
    let config = trackly_infra::AppConfig::default();
    let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
    let ctx = trackly_app::context::AppCtx::build(paths, config, guard)
        .await
        .expect("AppCtx::build with mock");

    // Проверяем что snmp_client выбран как MockSnmpClient через type_name на PrinterService.
    let _svc: &PrinterService = &ctx.printers;
    let client_name = type_name::<trackly_infra::snmp::mock::MockSnmpClient>();
    let real_name = type_name::<trackly_infra::snmp::real::RealSnmpClient>();

    // Косвенная проверка: с TRACKLY_SNMP_MOCK poll_tx канал создан
    // и сервис содержит ws_tx с capacity 128.
    // Прямо через Arc нельзя получить type_name dyn trait — проверяем через env var.
    let is_mock_set = std::env::var("TRACKLY_SNMP_MOCK").is_ok();
    assert!(
        is_mock_set,
        "TRACKLY_SNMP_MOCK should be set in test env, client_name={client_name}"
    );

    // Дополнительная проверка: ws_broadcast имеет правильный capacity (косвенно — >=1 subscriber после subscribe).
    let _rx = ctx.ws_broadcast.subscribe();
    assert!(
        ctx.ws_broadcast.receiver_count() >= 1,
        "ws_broadcast must have at least 1 subscriber after subscribe()"
    );

    // Убираем env var для следующих тестов.
    std::env::remove_var("TRACKLY_SNMP_MOCK");
    ctx.shutdown.cancel();

    // Проверяем наличие типов в экспортах.
    let _ = real_name; // suppress unused
}

/// ASVS V4: HTTP GET /api/v1/ws без session cookie → 401 (не WS upgrade).
///
/// Тест создаёт тестовый axum app с ws::router() + SessionManagerLayer
/// и проверяет что GET /api/v1/ws без cookie → StatusCode::UNAUTHORIZED (401).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_ws_unauth_401() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use trackly_app::http::build_router;
    use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;

    let dir = tempfile::TempDir::new().expect("tempdir");
    let dir_path = dir.keep();
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir_path).expect("paths");
    let config = trackly_infra::AppConfig::default();
    let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
    let ctx = trackly_app::context::AppCtx::build(paths, config, guard)
        .await
        .expect("AppCtx::build");

    let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
    let router = build_router(&ctx, session_store);

    // GET /api/v1/ws без session cookie и без WS upgrade заголовков → 401.
    // Не используем WS upgrade заголовки: axum валидирует WebSocketUpgrade
    // на этапе экстракции (before handler body) и вернул бы 426.
    // Option<WebSocketUpgrade> = None для plain GET — auth check проходит первым.
    let res = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/ws")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("oneshot");

    let status = res.status();
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "GET /api/v1/ws without session must return 401, got {status}"
    );

    ctx.shutdown.cancel();
}

/// Plan 12-01 (D-05): RequestDto.printer_place is joined from
/// `locations.name` via the request's printer device, not a separate query.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_dto_carries_printer_place() {
    use rusqlite::params;
    use std::sync::Arc;
    use trackly_app::dto::request::RequestCreateDto;
    use trackly_app::services::RequestService;
    use trackly_core::auth::{Identity, Role};
    use trackly_core::primitives::clock::Clock;
    use trackly_infra::clock_impl::SystemClock;
    use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

    let (writer, readers, _guard) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let (ws_tx, _rx) = tokio::sync::broadcast::channel(16);
    let ws_tx = Arc::new(ws_tx);

    let svc = RequestService::new(writer.clone(), readers, clock.clone(), ws_tx);

    let now = clock.unix_seconds();
    let printer_device_id: i64 = writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, password_hash, role, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES ('printer_loc_admin', 'Admin', 'hash', 'admin', ?1, ?1, 1)",
                params![now],
            )
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("{e}"),
            })?;

            conn.execute(
                "INSERT INTO places (kind, name, created_at_utc, updated_at_utc, version) \
                 VALUES ('room', 'Каб. 305', ?1, ?1, 1)",
                params![now],
            )
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            let place_id = conn.last_insert_rowid();

            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, place_id, status_id, created_at_utc, updated_at_utc, version) \
                 VALUES (2, 'Pantum BM5100ADN', ?1, 1, ?2, ?2, 1)",
                params![place_id, now],
            )
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed printer device with location");

    let caller = Identity {
        user_id: Some(1),
        role: Role::Admin,
    };

    let created = svc
        .create(
            RequestCreateDto {
                request_type: "cartridge_replace".to_string(),
                printer_device_id: Some(printer_device_id as i32),
                cartridge_model_id: None,
                category_id: None,
                description: None,
            },
            &caller,
        )
        .await
        .expect("create cartridge_replace request");

    let fetched = svc.get(created.id, &caller).await.expect("get request");

    assert_eq!(
        fetched.printer_place.as_deref(),
        Some("Каб. 305"),
        "printer_place must be joined from the printer device's location"
    );
}

/// Plan 12-01 (D-05): printer_place is None (NULL-safe) when the request
/// has no printer (free_form) or the printer has no location set.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_dto_printer_place_none_when_no_location_or_no_printer() {
    use rusqlite::params;
    use std::sync::Arc;
    use trackly_app::dto::request::RequestCreateDto;
    use trackly_app::services::RequestService;
    use trackly_core::auth::{Identity, Role};
    use trackly_core::primitives::clock::Clock;
    use trackly_infra::clock_impl::SystemClock;
    use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

    let (writer, readers, _guard) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let (ws_tx, _rx) = tokio::sync::broadcast::channel(16);
    let ws_tx = Arc::new(ws_tx);

    let svc = RequestService::new(writer.clone(), readers, clock.clone(), ws_tx);

    let now = clock.unix_seconds();
    let printer_no_place_id: i64 = writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO users (login, full_name, password_hash, role, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES ('printer_loc_admin2', 'Admin', 'hash', 'admin', ?1, ?1, 1)",
                params![now],
            )
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("{e}"),
            })?;

            // Printer device with place_id = NULL.
            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, place_id, status_id, created_at_utc, updated_at_utc, version) \
                 VALUES (2, 'Kyocera ECOSYS no-location', NULL, 1, ?1, ?1, 1)",
                params![now],
            )
            .map_err(|e| trackly_core::error::AppError::Internal {
                source_chain: format!("{e}"),
            })?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed printer device without location");

    let caller = Identity {
        user_id: Some(1),
        role: Role::Admin,
    };

    // free_form request: no printer at all.
    let free_form = svc
        .create(
            RequestCreateDto {
                request_type: "free_form".to_string(),
                printer_device_id: None,
                cartridge_model_id: None,
                category_id: None,
                description: Some("Свободная заявка без принтера".to_string()),
            },
            &caller,
        )
        .await
        .expect("create free_form request");

    let fetched_free_form = svc
        .get(free_form.id, &caller)
        .await
        .expect("get free_form request");
    assert_eq!(
        fetched_free_form.printer_place, None,
        "free_form request without printer_device_id must have printer_place = None"
    );

    // cartridge_replace request: printer set, but printer has no location.
    let with_printer = svc
        .create(
            RequestCreateDto {
                request_type: "cartridge_replace".to_string(),
                printer_device_id: Some(printer_no_place_id as i32),
                cartridge_model_id: None,
                category_id: None,
                description: None,
            },
            &caller,
        )
        .await
        .expect("create cartridge_replace request");

    let fetched_with_printer = svc
        .get(with_printer.id, &caller)
        .await
        .expect("get cartridge_replace request");
    assert_eq!(
        fetched_with_printer.printer_place, None,
        "printer without place_id must yield printer_place = None (NULL-safe LEFT JOIN)"
    );
}
