//! D-20 print-path-shortening regression tests (Phase 39.1 Plan 06).
//!
//! Covers:
//!  1. `render_pdf` shortens the frozen `place_path_snapshot` by the CURRENT
//!     `place_effective_variant` for `acts.place_id` (not the variant at
//!     act-create time) — proven by changing the variant AFTER act creation
//!     and observing the printed form pick up the new variant on next
//!     render.
//!  2. An act whose place was later soft-deleted (Pitfall 4: `place_id`
//!     still set, but `place_effective_variant` has no row for it) falls
//!     back to the organization default variant and renders without panic.
//!  3. An act with no place at all (`place_id`/`place_path_snapshot` both
//!     `NULL`) still renders the existing blank-underline fallback.
//!
//! Only invented place names ("Здание А", "Территория Б") — never real
//! organization data, per the project's hard privacy constraint.

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto};
use trackly_app::pdf::PdfRenderer;
use trackly_app::services::{ActService, OrganizationService, TemplateService};
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::ports::places::PlaceRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::SqlitePlaceRepository;
use trackly_infra::test_support::test_writer_and_readers;
use trackly_infra::Paths;

struct Pipeline {
    acts: ActService,
    writer: Arc<WriterHandle>,
    _readers: Arc<ReaderPool>,
    _dir: tempfile::TempDir,
}

async fn make_full_pipeline() -> Pipeline {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);

    let paths = Arc::new(Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("paths"));
    let organization = Arc::new(OrganizationService::new(paths));
    let templates = Arc::new(TemplateService::new(
        writer.clone(),
        readers.clone(),
        clock.clone(),
    ));
    let pdf = Arc::new(PdfRenderer::new());
    templates.seed_defaults_on_startup().await.expect("seed");

    let acts = ActService::new(writer.clone(), readers.clone(), clock.clone()).with_pdf_pipeline(
        templates.clone(),
        organization.clone(),
        pdf.clone(),
    );

    Pipeline {
        acts,
        writer,
        _readers: readers,
        _dir: dir,
    }
}

async fn create_place(writer: &Arc<WriterHandle>, name: &str) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            let repo = SqlitePlaceRepository;
            let new_place = PlaceNew {
                parent_id: None,
                kind: PlaceKind::Building,
                name: name.clone(),
                level: None,
                is_storage: false,
                sort_order: None,
                notes: None,
            };
            repo.create(conn, &new_place, 1_700_000_000)
        })
        .await
        .expect("create root place")
}

async fn create_child_place(writer: &Arc<WriterHandle>, parent_id: i64, name: &str) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            let repo = SqlitePlaceRepository;
            let new_place = PlaceNew {
                parent_id: Some(parent_id),
                kind: PlaceKind::Room,
                name: name.clone(),
                level: None,
                is_storage: false,
                sort_order: None,
                notes: None,
            };
            repo.create(conn, &new_place, 1_700_000_000)
        })
        .await
        .expect("create child place")
}

async fn set_override(writer: &Arc<WriterHandle>, place_id: i64, variant: Option<&str>) {
    let variant = variant.map(|v| v.to_string());
    writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE places SET path_variant_override = ?1 WHERE id = ?2",
                params![variant, place_id],
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("set path_variant_override");
}

async fn set_org_default(writer: &Arc<WriterHandle>, variant: &str) {
    let variant = variant.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE app_settings SET value = ?1 WHERE key = 'place_path_variant'",
                params![variant],
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("set org default");
}

/// Soft-deletes `place_id` directly (raw SQL — mirrors Pitfall 4's "place
/// disappeared" scenario: `acts.place_id` still points at the row, but
/// `place_effective_variant`'s base case filters `deleted_at_utc IS NULL`,
/// so the view has no row for it).
async fn soft_delete_place(writer: &Arc<WriterHandle>, place_id: i64) {
    writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE places SET deleted_at_utc = 1700000200 WHERE id = ?1",
                params![place_id],
            )
            .map_err(map_rusqlite)
        })
        .await
        .expect("soft-delete place");
}

async fn seed_device(writer: &Arc<WriterHandle>, name: &str) -> i64 {
    let name = name.to_string();
    writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices \
                 (type_id, name, status_id, version, created_at_utc, updated_at_utc) \
                 VALUES (1, ?1, 1, 1, ?2, ?2)",
                params![name, 1_700_000_000_i64],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed device")
}

async fn create_handover_at_place(
    svc: &ActService,
    device_id: i64,
    place_id: Option<i64>,
    giver: &str,
    receiver: &str,
) -> trackly_app::dto::act::ActDto {
    svc.create(ActCreateDto {
        number_override: None,
        giver_name: giver.to_string(),
        receiver_name: receiver.to_string(),
        place_id,
        notes: None,
        deadline_utc: None,
        handover_date_utc: None,
        items: vec![ActItemNewDto {
            device_id,
            device_ids: Vec::new(),
            quantity: 1,
        }],
    })
    .await
    .expect("create handover")
}

// ---------------------------------------------------------------------------
// Test 1: render_pdf_uses_current_variant_not_create_time_variant (D-20)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_pdf_uses_current_variant_not_create_time_variant() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let building = create_place(&p.writer, "Здание А").await;
        let floor = create_child_place(&p.writer, building, "1 этаж").await;
        let room = create_child_place(&p.writer, floor, "1-05").await;
        let device_id = seed_device(&p.writer, "Вариант-Ноутбук").await;

        // Org default is 'ends' at create time (V039 seed) — snapshot is
        // captured regardless of variant (it freezes the full path, D-16).
        let act =
            create_handover_at_place(&p.acts, device_id, Some(room), "Иванов И.И.", "Петров П.П.")
                .await;
        assert_eq!(
            act.place_path_snapshot.as_deref(),
            Some("Здание А / 1 этаж / 1-05"),
            "fixture invariant: snapshot captured at create time"
        );

        // Switch the room's OWN override to 'last_two' AFTER the act was
        // created — D-20 requires render_pdf to pick up the CURRENT variant
        // on every render, not the variant at create time.
        set_override(&p.writer, room, Some("last_two")).await;

        let html = p.acts.render_pdf(act.id).await.expect("render_pdf");
        // Autoescape encodes '/' as '&#x2f;' (T-16-01).
        let expected_row = "<div class=\"field-row\">Расположение: 1 этаж &#x2f; 1-05</div>";
        assert!(
            html.contains(expected_row),
            "expected D-20 shortened 'last_two' place field-row {expected_row:?} — \
             render_pdf must resolve the CURRENT effective variant, not the one at \
             act-create time. Head: {:?}",
            html.chars().take(2000).collect::<String>()
        );

        // The frozen full-path snapshot must still be present verbatim
        // elsewhere in the render context (Pitfall 3 — place_path itself is
        // never removed, only place_path_short is newly added).
        set_override(&p.writer, room, Some("ends")).await;
        let html2 = p.acts.render_pdf(act.id).await.expect("render_pdf");
        let expected_ends_row = "<div class=\"field-row\">Расположение: Здание А // 1-05</div>";
        assert!(
            html2.contains(expected_ends_row),
            "expected D-20 shortened 'ends' place field-row {expected_ends_row:?} on a second \
             render after switching the variant again. Head: {:?}",
            html2.chars().take(2000).collect::<String>()
        );
    })
    .await
    .expect("render_pdf_uses_current_variant_not_create_time_variant budget");
}

// ---------------------------------------------------------------------------
// Test 2: render_pdf_falls_back_to_org_default_when_place_disappeared
// (Pitfall 4)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_pdf_falls_back_to_org_default_when_place_disappeared() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let building = create_place(&p.writer, "Территория Б").await;
        let floor = create_child_place(&p.writer, building, "Объект Х").await;
        let room = create_child_place(&p.writer, floor, "помещение 3").await;
        let device_id = seed_device(&p.writer, "Исчезло-Ноутбук").await;

        let act = create_handover_at_place(
            &p.acts,
            device_id,
            Some(room),
            "Сидоров С.С.",
            "Кузнецов К.К.",
        )
        .await;
        assert_eq!(
            act.place_path_snapshot.as_deref(),
            Some("Территория Б / Объект Х / помещение 3"),
            "fixture invariant: snapshot captured at create time"
        );

        // Org default is 'last_two' — the place itself disappears (soft
        // delete), so `place_effective_variant` has no row for `place_id`
        // (Pitfall 4). render_pdf must fall back to the org default and
        // must NOT panic.
        set_org_default(&p.writer, "last_two").await;
        soft_delete_place(&p.writer, room).await;

        let html = p
            .acts
            .render_pdf(act.id)
            .await
            .expect("render_pdf must not fail/panic when the place has disappeared");
        let expected_row =
            "<div class=\"field-row\">Расположение: Объект Х &#x2f; помещение 3</div>";
        assert!(
            html.contains(expected_row),
            "expected org-default 'last_two' fallback shortening on a disappeared place \
             {expected_row:?}. Head: {:?}",
            html.chars().take(2000).collect::<String>()
        );
    })
    .await
    .expect("render_pdf_falls_back_to_org_default_when_place_disappeared budget");
}

// ---------------------------------------------------------------------------
// Test 3: render_pdf_with_no_place_renders_blank_underline (sanity — the
// existing D-27 fallback branch keeps working unchanged)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn render_pdf_with_no_place_renders_blank_underline() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let device_id = seed_device(&p.writer, "Без-Места-Ноутбук").await;

        let act =
            create_handover_at_place(&p.acts, device_id, None, "Иванов И.И.", "Петров П.П.").await;
        assert!(
            act.place_path_snapshot.is_none(),
            "fixture invariant: no place -> no snapshot"
        );

        let html = p.acts.render_pdf(act.id).await.expect("render_pdf");
        assert!(
            html.contains("Расположение: <span class=\"value-blank\"></span>"),
            "an act with no place must keep rendering the blank-underline fallback. \
             Head: {:?}",
            html.chars().take(2000).collect::<String>()
        );
    })
    .await
    .expect("render_pdf_with_no_place_renders_blank_underline budget");
}
