---
quick_id: 260805-nae
slug: employee-dashboard-widget-must-exclude-ad-register
phase: 260805-nae
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-app/src/services/dashboard_service.rs
  - crates/trackly-app/tests/dashboard_widgets.rs
autonomous: true
requirements: [NAE-01]
must_haves:
  truths:
    - "An employee whose ONLY request is the invisible auto-created `ad_register` row sees request_counts_open == 0 (and in_progress/completed == 0) from the dashboard widget, matching the empty list they see on the requests page — not the current phantom 1"
    - "An employee's own REAL (non-ad_register) requests are still counted normally by the same widget — the fix excludes `request_type = 'ad_register'` specifically, not all requests"
    - "The admin/manager dashboard branch (DASH-04, the `get_all_widgets` body above the Employee early-return) is untouched — this fix only edits `get_employee_widgets`, never the org-wide query path"
  artifacts:
    - path: "crates/trackly-app/src/services/dashboard_service.rs"
      provides: "get_employee_widgets's request-count SQL excludes ad_register rows, unconditionally (function is only ever reached for an Employee caller)"
      contains: "r.request_type != 'ad_register'"
    - path: "crates/trackly-app/tests/dashboard_widgets.rs"
      provides: "Regression test proving an employee's ad_register-only request yields zero widget counts"
      contains: "ad_register"
  key_links:
    - from: "crates/trackly-app/src/services/dashboard_service.rs (get_employee_widgets clauses vec)"
      to: "SQL WHERE clause of the request_counts_* query"
      via: "clauses.push(\"r.request_type != 'ad_register'\".to_string())"
      pattern: "r\\.request_type != 'ad_register'"
---

<objective>
Close the THIRD (and last) of three independently-written request-counting code paths that leak
the invisible, auto-created `ad_register` row into an employee-visible count. `RequestService::list`
(oldest) and `RequestService::counts` (fixed in quick task 260804-l22) already exclude
`request_type = 'ad_register'` from anything a non-admin caller sees. `DashboardService::
get_employee_widgets` — which feeds the "Мои заявки" summary card on the requests page via the
`dashboard_get_all_widgets` command/route — builds its own SQL from scratch and never got the
exclusion, so an employee auto-registered via AD-SSO sees "1 активная заявка, 1 новая заявка"
while their actual request list is empty (live Windows user report).

Purpose: the employee dashboard widget's counts must agree with what `list()` shows the same
employee — a caller must never see a nonzero count for a request row they cannot open, edit, or
even see exists.

Output: `get_employee_widgets`'s request-count query gains the same `request_type != 'ad_register'`
exclusion already proven in `list()`/`counts()`, applied unconditionally (not parameterised by
role) because this function is reached only for `Role::Employee` callers — see the `D-GATE-03`
dispatch note at the top of `get_all_widgets`. A regression test seeds an employee whose only
request is an `ad_register` row and asserts the widget reports zero, alongside a control case
proving real requests are still counted.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md

<interfaces>
<!-- Current (BEFORE this plan) get_employee_widgets request-count block —
crates/trackly-app/src/services/dashboard_service.rs, ~lines 316-372. This is the ONLY block this
plan touches; do not touch anything below it in the same function (the DashboardWidgetDto
construction) beyond what naturally follows from the SQL fix. -->
```rust
tokio::task::spawn_blocking(move || {
    let conn = readers.acquire();

    let (req_ts_from, req_ts_to) = match &period {
        Some(p) => compute_period_utc(p, tz),
        None => (None, None),
    };

    let (request_counts_open, request_counts_in_progress, request_counts_completed) = {
        let mut clauses = vec![
            "r.deleted_at_utc IS NULL".to_string(),
            "r.requested_by_user_id = ?1".to_string(),
        ];
        let mut owned: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(owner_user_id)];
        let mut pidx = 2usize;
        if let Some(from) = req_ts_from {
            clauses.push(format!("r.created_at_utc >= ?{pidx}"));
            owned.push(Box::new(from));
            pidx += 1;
        }
        if let Some(to) = req_ts_to {
            clauses.push(format!("r.created_at_utc <= ?{pidx}"));
            owned.push(Box::new(to));
            pidx += 1;
        }
        let _ = pidx;

        let sql = format!(
            "SELECT r.status, COUNT(r.id) \
             FROM requests r \
             WHERE {} \
             GROUP BY r.status",
            clauses.join(" AND ")
        );
        // ... query_map over r.status, accumulates into open/in_progress/completed
    };
    // ...
})
```

Note this function's SELECT already aliases the table as `r` (`FROM requests r`) — matching
`RequestService::list`'s aliased style (`r.request_type`), NOT `RequestService::counts`'s aliasless
style (`request_type`, no `r.`) in `requests_sqlite.rs`. Use the `r.`-prefixed form here to match
the alias actually in scope in THIS query.

<!-- D-GATE-03 dispatch note — crates/trackly-app/src/services/dashboard_service.rs, ~lines 53-64.
Confirms get_employee_widgets is reachable ONLY for an Employee caller: -->
```rust
/// D-GATE-03: an Employee caller is routed to [`Self::get_employee_widgets`]
/// — a structurally separate query path that never touches the
/// devices/cartridges/printers tables, not a filtered view of this
/// org-wide payload. Admin/Manager callers continue through the
/// unchanged body below.
pub async fn get_all_widgets(&self, caller: &Identity, period: Option<PeriodDto>) -> Result<DashboardWidgetDto, AppError> {
    if matches!(caller.role, trackly_core::auth::Role::Employee) {
        return self.get_employee_widgets(caller, period).await;
    }
    // ... admin/manager DASH-01..05 body, UNTOUCHED by this plan ...
}
```

<!-- The already-correct reference exclusion in RequestService::list — request_service.rs ~line 120,
proves the `r.request_type != 'ad_register'` predicate form (aliased) is the right one for a query
using `FROM requests r`: -->
```rust
let exclude_ad_register = !matches!(caller.role, trackly_core::auth::Role::Admin);
// repo.list(..., exclude_ad_register) -> requests_sqlite.rs list() appends:
//   AND (?5 = 0 OR r.request_type != 'ad_register')
```

<!-- Existing test file this plan extends — crates/trackly-app/tests/dashboard_widgets.rs.
Full current content (109 lines) already read; reproducing its fixture helper for reference: -->
```rust
fn build_test_db() -> (Arc<WriterHandle>, Arc<ReaderPool>) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let mut conn = rusqlite::Connection::open(&db_path).unwrap();
    trackly_infra::db::pragmas::apply_writer_pragmas(&conn).unwrap();
    trackly_infra::db::migrations::run(&mut conn).unwrap();
    let writer = Arc::new(WriterHandle::spawn(conn));
    let readers = Arc::new(ReaderPool::new(&db_path, 2).unwrap());
    (writer, readers)
}
```
This file has NO existing helper that inserts a `users`/`requests` row (its two existing tests only
assert on an empty DB) — the new test must add its own seed helper, following the INSERT shape
already proven in `crates/trackly-app/tests/requests_ad_register.rs`'s `seed_pending_register`
(same `users` and `requests` column lists — reproduced below for the values needed, since a
different test crate file cannot import another integration test file's private fn):
```rust
// users: (login, full_name, password_hash, role, ad_user, is_active, created_at_utc, updated_at_utc, version)
//   VALUES (?1, ?2, NULL, 'employee', 1, 0, ?3, ?3, 1)
// requests: (request_type, status, requested_by_user_id, description, ad_subtype, created_at_utc, updated_at_utc, version)
//   VALUES ('ad_register', 'open', ?1, ?2, 'register', ?3, ?3, 1)
```

`Identity` construction for an employee caller (from `trackly_core::auth::{Identity, Role}`):
```rust
Identity { user_id: Some(user_id), role: Role::Employee }
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Exclude ad_register rows from the employee dashboard widget's request counts</name>
  <files>crates/trackly-app/src/services/dashboard_service.rs</files>
  <action>
Re-read the current state of `get_employee_widgets` first (line numbers above are as of 2026-08-05
and may drift) — confirm the exact clauses vec and SQL before editing.

In `get_employee_widgets`, inside the `request_counts_open`/`in_progress`/`completed` block, add
one more entry to the `clauses` vec: `"r.request_type != 'ad_register'".to_string()`. Push it right
after the existing `"r.requested_by_user_id = ?1".to_string()` clause and before the optional
period-bound clauses, so the clause order stays deterministic and matches the reading order of the
generated SQL. This is a LITERAL string clause, not a bound parameter — do NOT add a new `?N`
placeholder or push anything to the `owned` vec, and do NOT touch `pidx`. This is a deliberate
deviation from `RequestRepository::list`/`counts`'s parameterised `(?N = 0 OR ...)` form: those two
are called for BOTH admin and non-admin callers and need the bool switch, but per D-GATE-03 (see
`<interfaces>`) `get_employee_widgets` is reached ONLY when `caller.role == Role::Employee` — there
is no admin/manager code path through this function to preserve, so an unconditional literal
predicate is correct and simpler. Record this reasoning in the SUMMARY.

Do not add a role check, a new function parameter, or a new bool flag — the existing `caller`
parameter already guarantees the Employee-only precondition via the dispatch in `get_all_widgets`.
Do not touch the admin/manager body of `get_all_widgets` (the code above the
`if matches!(caller.role, ... Employee) { return ...; }` early return) — that is the DASH-04 path
and is explicitly out of scope. Do not touch `RequestService::list` or `RequestService::counts` in
`request_service.rs`, or anything in `requests_sqlite.rs` — those are already correct per REQ-06
and 260804-l22.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && cargo check -p trackly-app 2>&1 | tail -40</automated>
  </verify>
  <done>Workspace compiles clean for trackly-app. `get_employee_widgets`'s request-count SQL clauses include `r.request_type != 'ad_register'` unconditionally, with no new bound parameter added. The admin/manager body of `get_all_widgets` is byte-for-byte unchanged (diff shows only the `get_employee_widgets` function touched).</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Regression test — employee dashboard widget excludes ad_register-only request</name>
  <files>crates/trackly-app/tests/dashboard_widgets.rs</files>
  <behavior>
    - Test A (the defect): seed an employee user whose ONLY request row is `request_type='ad_register'`, `status='open'` (mirrors `seed_pending_register`'s INSERT shape from `requests_ad_register.rs`, reproduced in `<interfaces>`). Call `svc.get_all_widgets(&employee_identity, None)`. Assert `dto.request_counts_open == 0`, `dto.request_counts_in_progress == 0`, `dto.request_counts_completed == 0` — all zero, matching the empty list the employee actually sees.
    - Test B (the control, same test or a second assertion after seeding one more row for the SAME employee): seed a second, REAL request for the same user with `request_type='free_form'` (the `requests.request_type` CHECK constraint in `migrations/V006__requests.sql` allows exactly `'cartridge_replace' | 'free_form' | 'ad_register'` — `free_form` is the simplest valid non-`ad_register` value, needs no `printer_device_id`/`cartridge_model_id` FK), `status='open'`. Re-call `svc.get_all_widgets(&employee_identity, None)` and assert `dto.request_counts_open == 1` — proving the fix excludes `ad_register` specifically, not all requests for that employee.
  </behavior>
  <action>
Add one new `#[tokio::test(flavor = "multi_thread", worker_threads = 4)]` async test function to
`crates/trackly-app/tests/dashboard_widgets.rs`, following the existing two tests' structure
(`build_test_db()`, `SystemClock`, `AppConfig::default()`, `DashboardService::new(...)`, wrapped in
`tokio::time::timeout(Duration::from_secs(30), async { ... }).await.expect(...)`).

Add a small local seed helper in this file (it cannot import `requests_ad_register.rs`'s private
`seed_pending_register` — different integration test binary) that inserts one `users` row and one
`requests` row via `writer.execute(move |conn| { ... })`, using the exact column lists shown in
`<interfaces>` for the `users` and `requests` tables. Parameterise it so it can insert either an
`ad_register` row (Test A) or a normal row (Test B) — e.g. take `request_type: &str` as a parameter
— and return `(user_id, request_id)` like `seed_pending_register` does.

Build the `Identity` for the seeded employee as `Identity { user_id: Some(user_id), role:
trackly_core::auth::Role::Employee }` (import `trackly_core::auth::{Identity, Role}` — note the
existing two tests in this file only import `trackly_core::auth::Identity` for
`Identity::trusted_admin()`, so add the `Role` import and the explicit-construction form alongside
it).

Name the new test function `dashboard_employee_widget_excludes_ad_register`.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test dashboard_widgets dashboard_employee_widget_excludes_ad_register -- --nocapture 2>&1 | tail -40</automated>
  </verify>
  <done>New test passes. It fails if Task 1's fix is reverted (mentally trace: it calls `svc.get_all_widgets` with an Employee identity, which dispatches to `get_employee_widgets` per D-GATE-03 — reverting the clause makes `request_counts_open` come back as 1 instead of 0 for Test A). The control assertion (Test B) proves the exclusion is scoped to `ad_register` only, not a blanket "employee sees nothing" regression.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Employee caller → `DashboardService::get_employee_widgets` | Employee-role `Identity` is caller-supplied context, already authenticated upstream (session/JWT validated before this service method runs) — the bug is a data-scoping/consistency leak between two count paths for the SAME already-trusted caller, not an authentication or authorization bypass. No untrusted input crosses this boundary. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-260805nae-01 | Information Disclosure | `DashboardService::get_employee_widgets` | mitigate | This IS the fix: exclude `request_type = 'ad_register'` from the employee widget's request counts so the aggregate no longer reflects the existence of a system-generated administrative row (their own AD auto-registration request) that the employee cannot see, open, or act on via `list()` — closes the third and last of three independently-implemented count paths that leaked this. |
| T-260805nae-02 | Tampering (supply chain) | N/A | accept | No new dependency, no package install — edits two existing Rust files (`dashboard_service.rs`, `dashboard_widgets.rs`) only. Package Legitimacy Gate not applicable. |
</threat_model>

<verification>
1. `cargo check -p trackly-app` compiles clean.
2. `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test dashboard_widgets` (full file, run alone — one cargo invocation at a time, no concurrent `cargo test` runs against the same target dir) — all 3 tests pass (2 pre-existing + the new regression test).
3. Manual trace: confirm the admin/manager body of `get_all_widgets` (everything above the
   `if matches!(caller.role, ... Employee) { return ...; }` early return) is byte-for-byte
   unchanged — `git diff crates/trackly-app/src/services/dashboard_service.rs` should show edits
   ONLY inside `get_employee_widgets`.
4. Do NOT run `cargo test -p trackly-app` (the full crate) in this task — the workspace has a
   pre-existing unrelated hang on `auth_remember_cookie` (documented in prior sessions); target
   `--test dashboard_widgets` specifically as shown above.
</verification>

<success_criteria>
- An employee whose only request is the auto-created `ad_register` row sees `request_counts_open
  == 0` (previously `1`) from `dashboard_get_all_widgets` — matching the empty list they already
  see on the requests page (closes the live Windows user report).
- An employee's real (non-`ad_register`) requests are still counted normally by the same widget —
  the exclusion is scoped to `ad_register`, not a blanket suppression.
- The admin/manager dashboard branch (DASH-04) is untouched — verified by diff scope.
- `RequestService::list` and `RequestService::counts` are untouched — the three duplicated count
  paths are not refactored into one in this task (that is recorded as a follow-up observation
  below, not acted on).
- `cargo fmt` applied only to the two touched files (not workspace-wide).

## Design observation (recorded, not acted on)

This defect recurred because the "exclude `ad_register` from anything a non-admin caller can see"
rule is independently reimplemented in three places: `RequestService::list` (oldest, correct),
`RequestService::counts` (fixed in quick task 260804-l22), and now
`DashboardService::get_employee_widgets` (fixed here). Fixing one did not fix the others, and nothing
enforced that they stay in sync. A candidate follow-up — NOT part of this task, deserves its own
review — is a single shared predicate or repository helper (e.g. a `RequestRepository` method or a
SQL fragment constant) that all three call sites use, so a future change to the exclusion rule
cannot be applied to only one of three query builders again.

Also recorded, NOT acted on here: `RequestService::counts` excludes `ad_register` for every
non-Admin caller (Employee AND Manager), while the admin/manager dashboard branch (DASH-04, the
`get_all_widgets` body this task does not touch) does not exclude `ad_register` at all for Manager
callers. Whether a Manager should see these rows in the dashboard is a product question, not
something this task decides.
</success_criteria>

<output>
Create `.planning/quick/260805-nae-employee-dashboard-widget-must-exclude-a/260805-nae-SUMMARY.md` when done
</output>
