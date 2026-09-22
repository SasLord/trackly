//! Repository-level integration tests for `SqliteNumberTemplateRepository`
//! (Phase 40.2, Plan 03, Tasks 2+3) — a full stack of embedded migrations,
//! real seed data in `devices`/`acts`/`cartridges`, and
//! `compute_next_for_template` exercised against every literal numeric
//! example from `40.2-SPEC.md` (NUM-04).
//!
//! Mirrors `number_templates_migration.rs`'s `fresh_migrated_db` harness.
//! All seeded values are fictional placeholders (`ОРГ-00-…`, `C-…`, `D-…`,
//! generic act numbers) — no real organisation/person data
//! (`no_real_org_or_personal_data_in_repo`).

use rusqlite::{params, Connection};
use tempfile::TempDir;

use trackly_core::domain::number_templates::TemplateType;
use trackly_core::ports::number_templates::NumberTemplateRepository;
use trackly_infra::db::{migrations, pragmas};
use trackly_infra::repos::SqliteNumberTemplateRepository;

/// Fresh DB with all embedded migrations applied (same shape as
/// `number_templates_migration.rs::fresh_migrated_db`).
fn fresh_migrated_db(dir: &TempDir, file_name: &str) -> Connection {
    let db_path = dir.path().join(file_name);
    let mut conn = Connection::open(&db_path).expect("open");
    pragmas::apply_writer_pragmas(&conn).expect("pragmas");
    migrations::run(&mut conn).expect("run migrations");
    conn
}

/// Insert a live device row with the given `inventory_number` (type_id=1
/// "Устройство", status_id defaults to 1 "На складе" per V001 seed).
fn insert_device(conn: &Connection, inventory_number: &str, now_utc: i64) {
    conn.execute(
        "INSERT INTO devices (type_id, name, inventory_number, created_at_utc, updated_at_utc) \
         VALUES (1, 'Тестовое устройство', ?1, ?2, ?2)",
        params![inventory_number, now_utc],
    )
    .expect("insert device");
}

/// Insert a live handover act row with the given `number`.
fn insert_handover_act(conn: &Connection, number: i64, now_utc: i64) {
    conn.execute(
        "INSERT INTO acts (number, act_type, giver_name, receiver_name, created_at_utc, updated_at_utc) \
         VALUES (?1, 'handover', 'Иванов И.И.', 'Петров П.П.', ?2, ?2)",
        params![number, now_utc],
    )
    .expect("insert handover act");
}

/// Insert a live return act row with the given `number` — used to prove
/// returns never participate in the act-number sequence (SPEC NUM-14).
fn insert_return_act(conn: &Connection, number: i64, now_utc: i64) {
    conn.execute(
        "INSERT INTO acts (number, act_type, giver_name, receiver_name, created_at_utc, updated_at_utc) \
         VALUES (?1, 'return', 'Иванов И.И.', 'Петров П.П.', ?2, ?2)",
        params![number, now_utc],
    )
    .expect("insert return act");
}

/// Ensure a single `cartridge_models` row exists, returning its id
/// (kind_id defaults to 1 per V016 — irrelevant here, `fetch_matching_values`
/// never filters by it).
fn ensure_cartridge_model(conn: &Connection, now_utc: i64) -> i64 {
    conn.execute(
        "INSERT INTO cartridge_models (brand, model, created_at_utc, updated_at_utc) \
         VALUES ('ТестБренд', 'ТестМодель', ?1, ?1)",
        params![now_utc],
    )
    .expect("insert cartridge model");
    conn.last_insert_rowid()
}

/// Insert a live cartridge/drum row with the given `code`.
fn insert_cartridge(conn: &Connection, model_id: i64, code: &str, now_utc: i64) -> i64 {
    conn.execute(
        "INSERT INTO cartridges (code, model_id, created_at_utc, updated_at_utc) \
         VALUES (?1, ?2, ?3, ?3)",
        params![code, model_id, now_utc],
    )
    .expect("insert cartridge");
    conn.last_insert_rowid()
}

/// Soft-delete a cartridge row by id.
fn soft_delete_cartridge(conn: &Connection, id: i64, now_utc: i64) {
    conn.execute(
        "UPDATE cartridges SET deleted_at_utc = ?1 WHERE id = ?2",
        params![now_utc, id],
    )
    .expect("soft delete cartridge");
}

/// Fetch the single template V041 seeds for `template_type` (D-16) — used
/// instead of `insert_in_tx` when the scenario doesn't need a custom mask,
/// since re-inserting the same `(type, mask)` pair would collide with the
/// seed row (`UNIQUE(type, mask)`).
fn seeded_template(
    conn: &Connection,
    repo: &SqliteNumberTemplateRepository,
    template_type: TemplateType,
) -> trackly_core::domain::number_templates::NumberTemplateRow {
    let mut rows = repo
        .list(conn, Some(template_type))
        .expect("list seeded templates");
    assert_eq!(rows.len(), 1, "V041 seeds exactly one template per type");
    rows.remove(0)
}

const NOW: i64 = 1_758_000_000; // arbitrary fixed instant, unrelated to any real date-token test

/// Scenario 1 (SPEC NUM-04): a gap below the running max — live `…01..16` +
/// `…20` → `first_free=17`, `max_plus_one=21`.
#[test]
fn gap_in_the_middle_17_and_21() {
    let dir = TempDir::new().expect("tempdir");
    let mut conn = fresh_migrated_db(&dir, "gap-middle.db");
    let repo = SqliteNumberTemplateRepository;

    for n in 1..=16 {
        insert_device(&conn, &format!("ОРГ-00-{n:06}"), NOW);
    }
    insert_device(&conn, "ОРГ-00-000020", NOW);

    let tx = conn.transaction().expect("begin tx");
    let id = repo
        .insert_in_tx(&tx, TemplateType::DeviceInventory, "ОРГ-00-[XXXXXX]", NOW)
        .expect("insert template");
    tx.commit().expect("commit");

    let template = repo.get(&conn, id).expect("get template");
    let result = repo
        .compute_next_for_template(&conn, &template, NOW)
        .expect("compute next");

    assert_eq!(result.first_free, 17);
    assert_eq!(result.max_plus_one, 21);
    assert!(result.has_gap);
}

/// Scenario 2 (SPEC NUM-04): a dense tail with no gap below it — live
/// `…500..510` → `first_free=1`, `max_plus_one=511`. Also proves
/// `act_type='return'` rows never enter the act-number sequence (SPEC
/// NUM-14): a return act with a number that would otherwise extend the
/// sequence is seeded and must be ignored.
#[test]
fn dense_tail_500_to_510_and_returns_excluded() {
    let dir = TempDir::new().expect("tempdir");
    let conn = fresh_migrated_db(&dir, "dense-tail.db");
    let repo = SqliteNumberTemplateRepository;

    for n in 500..=510 {
        insert_handover_act(&conn, n, NOW);
    }
    // A return act with a number far outside the handover range — if this
    // leaked into the sequence, max_plus_one would jump to 1000+1.
    insert_return_act(&conn, 999, NOW);

    // V041 already seeds the canonical `act_number` / `[X]` template
    // (D-16) — reuse it rather than inserting a colliding duplicate
    // (`UNIQUE(type, mask)`).
    let template = seeded_template(&conn, &repo, TemplateType::ActNumber);
    let result = repo
        .compute_next_for_template(&conn, &template, NOW)
        .expect("compute next");

    assert_eq!(result.first_free, 1);
    assert_eq!(result.max_plus_one, 511);
    assert!(result.has_gap);
}

/// Scenario 3 (SPEC NUM-04): `[YYYY]/[MM]-[X]` — live records for
/// `2026/08-*` must not influence the count for `2026/09-*`: the expanded
/// prefix differs, so they are two entirely separate sequences even though
/// both live in the same physical column at the same time.
#[test]
fn month_prefix_isolates_independent_sequences() {
    let dir = TempDir::new().expect("tempdir");
    let mut conn = fresh_migrated_db(&dir, "month-isolation.db");
    let repo = SqliteNumberTemplateRepository;

    for n in 1..=5 {
        insert_device(&conn, &format!("2026/08-{n}"), NOW);
    }
    insert_device(&conn, "2026/09-1", NOW);
    insert_device(&conn, "2026/09-2", NOW);

    let tx = conn.transaction().expect("begin tx");
    let id = repo
        .insert_in_tx(&tx, TemplateType::DeviceInventory, "[YYYY]/[MM]-[X]", NOW)
        .expect("insert template");
    tx.commit().expect("commit");

    let template = repo.get(&conn, id).expect("get template");

    // "today" = 2026-09-18 (matches the `2026/09-*` prefix).
    let today_september = time::Date::from_calendar_date(2026, time::Month::September, 18)
        .expect("valid date")
        .midnight()
        .assume_utc()
        .unix_timestamp();
    let result_sep = repo
        .compute_next_for_template(&conn, &template, today_september)
        .expect("compute next for september");
    assert_eq!(
        result_sep.first_free, 3,
        "only 2026/09-1 and 2026/09-2 count toward the September sequence"
    );

    // "today" = 2026-08-01 (matches the `2026/08-*` prefix) — the 5 August
    // devices are visible here instead, unaffected by the 2 September ones.
    let today_august = time::Date::from_calendar_date(2026, time::Month::August, 1)
        .expect("valid date")
        .midnight()
        .assume_utc()
        .unix_timestamp();
    let result_aug = repo
        .compute_next_for_template(&conn, &template, today_august)
        .expect("compute next for august");
    assert_eq!(
        result_aug.first_free, 6,
        "all 5 August devices count toward the August sequence, September ones don't"
    );
}

/// Scenario 4 (SPEC NUM-04): soft-deleting the only cartridge occupying a
/// number frees it back up — `compute_next` holds no state, so a plain
/// re-read of the live set is enough.
#[test]
fn soft_deleted_cartridge_frees_its_number() {
    let dir = TempDir::new().expect("tempdir");
    let conn = fresh_migrated_db(&dir, "soft-delete-frees.db");
    let repo = SqliteNumberTemplateRepository;

    let model_id = ensure_cartridge_model(&conn, NOW);
    insert_cartridge(&conn, model_id, "C-0001", NOW);
    let second_id = insert_cartridge(&conn, model_id, "C-0002", NOW);

    // V041 already seeds the canonical `cartridge_code` / `C-[XXXX]`
    // template (D-16) — reuse it rather than inserting a colliding
    // duplicate (`UNIQUE(type, mask)`).
    let template = seeded_template(&conn, &repo, TemplateType::CartridgeCode);

    let before = repo
        .compute_next_for_template(&conn, &template, NOW)
        .expect("compute next before delete");
    assert_eq!(before.first_free, 3, "C-0001 and C-0002 both live");

    soft_delete_cartridge(&conn, second_id, NOW);

    let after = repo
        .compute_next_for_template(&conn, &template, NOW)
        .expect("compute next after delete");
    assert_eq!(
        after.first_free, 2,
        "soft-deleting C-0002 frees number 2 back up"
    );
}

/// Scenario 5 (must_haves.truths of this plan): "Картриджи" и
/// "Фотобарабаны" both read the SAME `cartridges.code` column, but two
/// templates with DIFFERENT literal prefixes (`C-[XXXX]` vs `D-[XXXX]`) see
/// DIFFERENT sequences — the isolation comes from the mask's literal
/// prefix via `extract_digits`, not from any `kind_id` filter in SQL.
#[test]
fn cartridge_and_drum_templates_see_isolated_sequences_despite_shared_column() {
    let dir = TempDir::new().expect("tempdir");
    let conn = fresh_migrated_db(&dir, "cartridge-drum-isolation.db");
    let repo = SqliteNumberTemplateRepository;

    let model_id = ensure_cartridge_model(&conn, NOW);
    insert_cartridge(&conn, model_id, "C-0001", NOW);
    insert_cartridge(&conn, model_id, "C-0002", NOW);
    insert_cartridge(&conn, model_id, "C-0003", NOW);
    insert_cartridge(&conn, model_id, "D-0001", NOW);

    // V041 already seeds both canonical templates (D-16) — reuse them
    // rather than inserting colliding duplicates (`UNIQUE(type, mask)`).
    let cartridge_template = seeded_template(&conn, &repo, TemplateType::CartridgeCode);
    let drum_template = seeded_template(&conn, &repo, TemplateType::DrumCode);

    let cartridge_next = repo
        .compute_next_for_template(&conn, &cartridge_template, NOW)
        .expect("compute next for cartridges");
    let drum_next = repo
        .compute_next_for_template(&conn, &drum_template, NOW)
        .expect("compute next for drums");

    assert_eq!(
        cartridge_next.first_free, 4,
        "C-0001..C-0003 taken -> next cartridge code is C-0004"
    );
    assert_eq!(
        drum_next.first_free, 2,
        "only D-0001 taken -> next drum code is D-0002, unaffected by the 3 cartridge rows"
    );
}

/// BE-WR-01/BE-WR-02: strict `set_context_in_tx` rejects missing/foreign
/// templates; lenient `remember_context_in_tx` (used inside the entity's own
/// transaction) degrades a vanished/foreign template to "no template"
/// instead of failing the already-validated create.
#[test]
fn context_setters_strict_and_lenient() {
    use trackly_core::domain::number_templates::TemplateContext;
    let dir = TempDir::new().expect("tempdir");
    let mut conn = fresh_migrated_db(&dir, "context-setters.db");
    let repo = SqliteNumberTemplateRepository;

    let tx = conn.transaction().expect("tx");
    let device_tpl = repo
        .insert_in_tx(&tx, TemplateType::DeviceInventory, "ИНВ-[XXX]", 1)
        .expect("insert device template");
    let strict_foreign =
        repo.set_context_in_tx(&tx, TemplateContext::ActCreate, Some(device_tpl), 2);
    assert!(
        matches!(
            strict_foreign,
            Err(trackly_core::error::AppError::Validation { .. })
        ),
        "got {strict_foreign:?}"
    );
    let strict_missing = repo.set_context_in_tx(&tx, TemplateContext::DeviceCreate, Some(9999), 2);
    assert!(
        matches!(
            strict_missing,
            Err(trackly_core::error::AppError::NotFound { .. })
        ),
        "got {strict_missing:?}"
    );

    repo.remember_context_in_tx(&tx, TemplateContext::DeviceCreate, Some(device_tpl), 3)
        .expect("lenient, valid");
    assert_eq!(
        repo.get_context_template_id(&tx, TemplateContext::DeviceCreate)
            .expect("get"),
        Some(device_tpl)
    );
    repo.remember_context_in_tx(&tx, TemplateContext::DeviceCreate, Some(9999), 4)
        .expect("lenient never fails on a vanished template");
    assert_eq!(
        repo.get_context_template_id(&tx, TemplateContext::DeviceCreate)
            .expect("get"),
        None
    );
    repo.remember_context_in_tx(&tx, TemplateContext::ActCreate, Some(device_tpl), 5)
        .expect("lenient never fails on a foreign template");
    assert_eq!(
        repo.get_context_template_id(&tx, TemplateContext::ActCreate)
            .expect("get"),
        None
    );
}
