//! Quick 261105-s6s (INT-26): приёмка SNMP-принтера `build_printers_admit`.
//!
//! До фикса шаг «создать устройство» уже создавал строку `printers` (через
//! `sync_printer_row_in_tx`), а следующий шаг `create_from_device` вставлял её
//! повторно и падал на `printers.device_id UNIQUE`, оставляя принтер без IP.
//!
//! Закрепляет:
//!   * a1 — admit проходит, строка `printers` ровно одна и несёт IP + community;
//!   * a2 — повторный admit того же IP отсекается стражем дубликата;
//!   * a3 — невалидный адрес (с портом) не оставляет ни устройства, ни строки
//!     `printers` (IP пишется в той же транзакции, что и устройство).
//!
//! Приватность (CLAUDE.md): адреса — фикстуры `MockSnmpClient` и
//! документационный диапазон 192.0.2.0/24, community вымышленная.

use std::sync::Arc;
use std::time::Duration;

use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use trackly_app::context::AppCtx;
use trackly_app::tauri_cmds::printers::build_printers_admit;
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_app_ctx::test_writer_and_readers;

/// IP из `MockSnmpClient::default_fixtures()` — probe отвечает.
const KNOWN_IP: &str = "192.168.1.100";
const COMMUNITY: &str = "comm-alpha";

fn minimal_ctx() -> (AppCtx, TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let paths =
        trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("resolve paths");
    let config = trackly_infra::AppConfig::default();
    let (_nb, log_guard) = tracing_appender::non_blocking(std::io::sink());
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let devices = Arc::new(trackly_app::services::DeviceService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let paths_arc = Arc::new(paths);
    let organization = Arc::new(trackly_app::services::OrganizationService::new(
        paths_arc.clone(),
    ));
    let templates = Arc::new(trackly_app::services::TemplateService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let pdf = Arc::new(trackly_app::pdf::PdfRenderer::new());
    let acts = Arc::new(
        trackly_app::services::ActService::new(writer.clone(), readers.clone(), clock.clone())
            .with_pdf_pipeline(templates.clone(), organization.clone(), pdf.clone()),
    );
    let cartridges = Arc::new(trackly_app::services::CartridgeService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let ad_client: Arc<dyn trackly_core::ports::ad::AdClient + Send + Sync> =
        Arc::new(trackly_infra::ad::mock::MockAdClient::default_fixtures());
    let (ws_tx, _) = tokio::sync::broadcast::channel::<trackly_app::dto::printer::WsEvent>(128);
    let ws_broadcast = Arc::new(ws_tx);
    let auth = Arc::new(trackly_app::services::AuthService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        ad_client,
        ws_broadcast.clone(),
        Arc::new(trackly_infra::ad::directory_mock::MockAdDirectory::default_fixtures()),
    ));
    let (poll_tx, _poll_rx) = tokio::sync::mpsc::channel::<i64>(64);
    let snmp_client: Arc<dyn trackly_core::ports::snmp::SnmpClient + Send + Sync> =
        Arc::new(trackly_infra::snmp::mock::MockSnmpClient::default_fixtures());
    let printers = Arc::new(trackly_app::services::PrinterService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        snmp_client,
        poll_tx,
        ws_broadcast.clone(),
    ));
    let requests = Arc::new(trackly_app::services::RequestService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        ws_broadcast.clone(),
    ));
    let org_db = Arc::new(trackly_app::services::OrgDbService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        paths_arc.clone(),
    ));
    let reports = Arc::new(trackly_app::services::ReportService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        Arc::new(config.clone()),
        pdf.clone(),
    ));
    let dashboard = Arc::new(trackly_app::services::DashboardService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        Arc::new(config.clone()),
    ));
    let backup = Arc::new(trackly_app::services::BackupService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        dir.path().join("trackly.db"),
    ));
    let places = Arc::new(trackly_app::services::PlaceService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let group_types = Arc::new(trackly_app::services::GroupTypeService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let groups = Arc::new(trackly_app::services::GroupService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let place_movements = Arc::new(trackly_app::services::PlaceMovementService::new(
        readers.clone(),
    ));
    let number_templates = Arc::new(trackly_app::services::NumberTemplateService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let attachments = Arc::new(trackly_app::services::AttachmentService::with_root(
        writer.clone(),
        readers.clone(),
        clock.clone(),
        dir.path().join("attachments"),
    ));
    let repairs = Arc::new(trackly_app::services::RepairService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let repair_acts = Arc::new(trackly_app::services::RepairActRenderer::new(
        readers.clone(),
        organization.clone(),
        org_db.clone(),
    ));
    let ctx = AppCtx {
        writer,
        readers,
        places,
        group_types,
        groups,
        place_movements,
        number_templates,
        attachments,
        paths: paths_arc,
        org_db,
        reports,
        dashboard,
        backup,
        config: Arc::new(config),
        clock,
        shutdown: CancellationToken::new(),
        log_guard: Arc::new(log_guard),
        schema_version: 15,
        devices,
        acts,
        organization,
        templates,
        pdf,
        cartridges,
        repairs,
        repair_acts,
        auth,
        server_ctl: Arc::new(tokio::sync::Mutex::new(None)),
        printers,
        requests,
        ws_broadcast,
    };
    (ctx, dir)
}

fn count(ctx: &AppCtx, sql: &str) -> i64 {
    let conn = ctx.readers.acquire();
    conn.query_row(sql, [], |r| r.get(0)).expect("count")
}

fn printer_net_by_ip(ctx: &AppCtx, ip: &str) -> Vec<(Option<String>, String)> {
    let conn = ctx.readers.acquire();
    let mut stmt = conn
        .prepare("SELECT ip_address, community FROM printers WHERE ip_address = ?1")
        .expect("prepare");
    stmt.query_map([ip], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("query")
        .map(|r| r.expect("row"))
        .collect()
}

#[tokio::test]
async fn a1_admit_creates_single_printer_row_with_network() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _dir) = minimal_ctx();
        let admitted = build_printers_admit(
            &ctx,
            &Identity::trusted_admin(),
            vec![KNOWN_IP.to_string()],
            COMMUNITY.to_string(),
        )
        .await
        .expect("admit must succeed (INT-26: no UNIQUE violation)");

        assert_eq!(admitted.len(), 1);
        assert_eq!(admitted[0].ip_address.as_deref(), Some(KNOWN_IP));
        assert!(admitted[0].community_configured);

        assert_eq!(
            count(&ctx, "SELECT COUNT(*) FROM devices WHERE type_id = 2"),
            1
        );
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM printers"), 1);
        assert_eq!(
            printer_net_by_ip(&ctx, KNOWN_IP),
            vec![(Some(KNOWN_IP.to_string()), COMMUNITY.to_string())]
        );
    })
    .await
    .expect("timeout");
}

#[tokio::test]
async fn a2_repeat_admit_is_skipped_as_duplicate() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _dir) = minimal_ctx();
        let caller = Identity::trusted_admin();
        let first = build_printers_admit(
            &ctx,
            &caller,
            vec![KNOWN_IP.to_string()],
            COMMUNITY.to_string(),
        )
        .await
        .expect("first admit");
        assert_eq!(first.len(), 1);

        let second = build_printers_admit(
            &ctx,
            &caller,
            vec![KNOWN_IP.to_string()],
            COMMUNITY.to_string(),
        )
        .await
        .expect("second admit");
        assert!(second.is_empty(), "duplicate IP must be skipped");
        assert_eq!(
            count(&ctx, "SELECT COUNT(*) FROM devices WHERE type_id = 2"),
            1
        );
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM printers"), 1);
    })
    .await
    .expect("timeout");
}

#[tokio::test]
async fn a3_invalid_ip_leaves_no_orphan_device() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _dir) = minimal_ctx();
        let res = build_printers_admit(
            &ctx,
            &Identity::trusted_admin(),
            vec!["192.0.2.10:161".to_string()],
            COMMUNITY.to_string(),
        )
        .await;
        assert!(res.is_err(), "address with port must be rejected");
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM devices"), 0);
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM printers"), 0);
    })
    .await
    .expect("timeout");
}
