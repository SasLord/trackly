//! D-16 print-fidelity + Common Pitfall 5 upgrade-safety regression tests
//! (Phase 39 Plan 11).
//!
//! Covers:
//!  1. `place_path_snapshot` is captured server-side at act-CREATE time and
//!     equals the place's full path at that moment (`ActService::create`,
//!     Plan 07's pattern this plan's `do_return`/`update_return` mirrors).
//!  2. D-16's freeze guarantee: renaming the act's place AFTER the act was
//!     created does NOT change the already-captured `place_path_snapshot`,
//!     while a LIVE lookup for that same `place_id` DOES reflect the rename
//!     — snapshot (print) and navigation (live) diverge on purpose.
//!  3. Common Pitfall 5 regression: the shipped ("seeded"/unmodified) default
//!     `act_handover.html` print template renders the D-27 «Расположение:»
//!     field-row from `act.place_path` without minijinja's `default()`
//!     filter silently swallowing the value — exercised through the REAL
//!     `ActService::render_pdf` pipeline.
//!
//!     NOTE on scope: `act_handover.minijinja` (the frozen krilla-era
//!     sibling this plan also renamed) is confirmed DEAD CODE for this
//!     render — `render_pdf` reads `templates/act_handover.html` via
//!     `html_templates::load_template` exclusively (Phase 16 HTML pivot);
//!     `template_service.rs::validate_preview` was retargeted onto the same
//!     file-based HTML pipeline in Phase 17 and no longer reads
//!     `document_templates.body_minijinja` for `act_handover` either
//!     (verified: `pdf_render_act.rs`'s
//!     `render_falls_back_to_embedded_default_when_broken_template_row_present`
//!     proves corrupting that DB column has zero effect on `render_pdf`'s
//!     output). Testing the truly-inert `.minijinja` path would validate
//!     nothing about what an actual printed act shows — this test instead
//!     exercises the ACTIVE file-based HTML template, which is where D-27
//!     ("printed act shows the full place path") actually has to hold.
//!
//! Каждый тест обёрнут в `tokio::time::timeout` — защита от CI deadlock.
//!
//! Только вымышленные названия мест ("Здание А", "Здание Б") — никогда
//! реальные данные организации, по жёсткому условию приватности проекта.

use std::sync::Arc;
use std::time::Duration;

use rusqlite::params;
use trackly_app::dto::act::{ActCreateDto, ActItemNewDto};
use trackly_app::pdf::PdfRenderer;
use trackly_app::services::{ActService, OrganizationService, TemplateService};
use trackly_core::domain::places::{PlaceKind, PlaceNew};
use trackly_core::error::AppError as CoreAppError;
use trackly_core::ports::places::PlaceRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::SqlitePlaceRepository;
use trackly_infra::test_support::test_writer_and_readers;
use trackly_infra::Paths;

fn make_acts_service() -> (ActService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = ActService::new(writer, readers, clock);
    (svc, dir)
}

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

/// Creates a root place (kind=Building) directly via `SqlitePlaceRepository`
/// on the service's own writer connection (D-18: only an explicit,
/// Admin-gated `create()` call makes a place — no device/act write path does
/// this implicitly).
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

/// Creates a child place (kind=Room, the last-level example the plan's own
/// must_haves.truths uses: "Здание А / 2 этаж / 214") under `parent_id`.
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

/// Renames `place_id` to `new_name` via `PlaceRepository::rename` directly —
/// mirrors D-18: only `PlaceService`/`PlaceRepository` ever renames a place,
/// `ActService` never does (it only reads the place tree at write time to
/// capture `place_path_snapshot`).
async fn rename_place(writer: &Arc<WriterHandle>, place_id: i64, new_name: &str) {
    let new_name = new_name.to_string();
    writer
        .execute(move |conn| {
            let repo = SqlitePlaceRepository;
            let current = repo.get(&*conn, place_id)?;
            repo.rename(conn, place_id, &new_name, current.version, 1_700_000_100)?;
            Ok::<_, CoreAppError>(())
        })
        .await
        .expect("rename place");
}

/// Live-resolved full path for `place_id` via `PlaceRepository::full_path`
/// (`place_full_paths`, always recomputed — the D-16 "navigation" side of
/// the print/navigate divergence).
async fn live_full_path(writer: &Arc<WriterHandle>, place_id: i64) -> String {
    writer
        .execute(move |conn| {
            let repo = SqlitePlaceRepository;
            repo.full_path(&*conn, place_id)
        })
        .await
        .expect("live full_path")
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
    place_id: i64,
    giver: &str,
    receiver: &str,
) -> trackly_app::dto::act::ActDto {
    svc.create(ActCreateDto {
        number_override: None,
        giver_name: giver.to_string(),
        receiver_name: receiver.to_string(),
        place_id: Some(place_id),
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
// Test 1: create_captures_place_path_snapshot_at_write_time
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_captures_place_path_snapshot_at_write_time() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let building = create_place(&svc.writer, "Здание А").await;
        let floor = create_child_place(&svc.writer, building, "2 этаж").await;
        let room = create_child_place(&svc.writer, floor, "214").await;
        let device_id = seed_device(&svc.writer, "Снимок-Ноутбук").await;

        let act =
            create_handover_at_place(&svc, device_id, room, "Иванов И.И.", "Петров П.П.").await;

        assert_eq!(act.place_id, Some(room), "place_id must round-trip");
        assert_eq!(
            act.place_path_snapshot.as_deref(),
            Some("Здание А / 2 этаж / 214"),
            "place_path_snapshot must equal the place's full path at write time"
        );
    })
    .await
    .expect("create_captures_place_path_snapshot_at_write_time budget");
}

// ---------------------------------------------------------------------------
// Test 2: rename_after_handover_does_not_alter_printed_snapshot (D-16)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rename_after_handover_does_not_alter_printed_snapshot() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_acts_service();
        let building = create_place(&svc.writer, "Здание А").await;
        let floor = create_child_place(&svc.writer, building, "2 этаж").await;
        let room = create_child_place(&svc.writer, floor, "214").await;
        let device_id = seed_device(&svc.writer, "Переименование-Ноутбук").await;

        let act =
            create_handover_at_place(&svc, device_id, room, "Иванов И.И.", "Петров П.П.").await;
        assert_eq!(
            act.place_path_snapshot.as_deref(),
            Some("Здание А / 2 этаж / 214"),
            "fixture invariant: snapshot captured at create time"
        );

        // Rename the room AFTER the act was created — an unrelated place
        // mutation, not an act edit.
        rename_place(&svc.writer, room, "215").await;

        let act_after = svc.get(act.id).await.expect("re-fetch act");
        assert_eq!(
            act_after.place_path_snapshot.as_deref(),
            Some("Здание А / 2 этаж / 214"),
            "D-16: the printed snapshot must stay frozen after an unrelated place rename"
        );

        let live_path = live_full_path(&svc.writer, room).await;
        assert_eq!(
            live_path, "Здание А / 2 этаж / 215",
            "live navigation lookup must reflect the rename immediately (no reindex)"
        );

        assert_ne!(
            act_after.place_path_snapshot.as_deref().unwrap(),
            live_path,
            "D-16 core guarantee: snapshot (print) and live path (navigate) must diverge \
             after a rename — proving the snapshot is frozen, not re-derived on read"
        );
    })
    .await
    .expect("rename_after_handover_does_not_alter_printed_snapshot budget");
}

// ---------------------------------------------------------------------------
// Test 3: seeded_default_template_renders_place_path_field_row (Common
// Pitfall 5 regression, D-27)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn seeded_default_template_renders_place_path_field_row() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let building = create_place(&p.writer, "Здание Б").await;
        let floor = create_child_place(&p.writer, building, "3 этаж").await;
        let room = create_child_place(&p.writer, floor, "301").await;
        let device_id = seed_device(&p.writer, "Печатная-Форма-Ноутбук").await;

        let act =
            create_handover_at_place(&p.acts, device_id, room, "Сидоров С.С.", "Кузнецов К.К.")
                .await;
        assert_eq!(
            act.place_path_snapshot.as_deref(),
            Some("Здание Б / 3 этаж / 301"),
            "fixture invariant: snapshot captured at create time"
        );

        // Renders through the REAL production pipeline: `templates_dir` has
        // never had `act_handover.html` written to it by this test (no
        // `TemplateService::update_body` call), so `render_pdf` falls back
        // to the shipped embedded default — the "seeded"/unmodified body
        // this regression class (`260704-uw3`) targets.
        let html = p.acts.render_pdf(act.id).await.expect("render_pdf");

        // Autoescape is ON (build_safe_html_env, T-16-01) — minijinja's HTML
        // autoescaper encodes `/` as `&#x2f;` (OWASP-recommended, prevents a
        // `</script>`-style breakout in an HTML-embedding context), so the
        // rendered `/` path separators are entity-encoded, not literal.
        //
        // Phase 39.1 Plan 06 (D-20): the field-row now reads
        // `act.place_path_short`, which shortens the frozen snapshot by the
        // CURRENT effective variant. A fresh DB's organization default is
        // 'ends' (V039 seed), so a 3-segment path renders first+last joined
        // by `sep_ends` (" // ", two slashes, each individually escaped).
        let expected_row = "<div class=\"field-row\">Расположение: Здание Б &#x2f;&#x2f; 301</div>";
        assert!(
            html.contains(expected_row),
            "expected D-20 shortened place field-row {expected_row:?} in rendered HTML — \
             act.place_path_short must reach the shipped default template. Head: {:?}",
            html.chars().take(2000).collect::<String>()
        );

        // Sanity: the blank-underline fallback branch (used when
        // act.place_path is falsy) must NOT have fired instead — proves the
        // assertion above isn't vacuously passing on an empty/missing value.
        assert!(
            !html.contains("Расположение: <span class=\"value-blank\"></span>"),
            "the place field-row must not have fallen back to its blank-underline branch \
             (would mean act.place_path reached the template empty/missing). Head: {:?}",
            html.chars().take(2000).collect::<String>()
        );
    })
    .await
    .expect("seeded_default_template_renders_place_path_field_row budget");
}

// ---------------------------------------------------------------------------
// Test 4: create_with_no_place_renders_blank_underline (sanity: the D-27
// row degrades gracefully, matching the deadline row's existing convention,
// when an act genuinely has no place)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_with_no_place_renders_blank_underline() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let device_id = seed_device(&p.writer, "Без-Места-Ноутбук").await;

        let act = p
            .acts
            .create(ActCreateDto {
                number_override: None,
                giver_name: "Иванов И.И.".into(),
                receiver_name: "Петров П.П.".into(),
                place_id: None,
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
            .expect("create handover without a place");
        assert!(
            act.place_path_snapshot.is_none(),
            "fixture invariant: no place -> no snapshot"
        );

        let html = p.acts.render_pdf(act.id).await.expect("render_pdf");
        assert!(
            html.contains("Расположение: <span class=\"value-blank\"></span>"),
            "an act with no place must render the blank-underline fallback (D-27, mirrors the \
             deadline row's existing convention), never an empty/missing interpolation. \
             Head: {:?}",
            html.chars().take(2000).collect::<String>()
        );
    })
    .await
    .expect("create_with_no_place_renders_blank_underline budget");
}
