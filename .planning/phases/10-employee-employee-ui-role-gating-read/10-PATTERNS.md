# Phase 10: Ограничение роли employee — Pattern Map

**Mapped:** 2026-06-21
**Files analyzed:** 19 (10 backend modify, 2 frontend new, 5 frontend modify, 1 test modify, 1 matrix-only)
**Analogs found:** 19 / 19

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/trackly-core/src/auth.rs` | service (domain) | transform (pure fn) | itself (matrix edit only) | exact — edit in place |
| `crates/trackly-app/src/services/device_service.rs` (`get/list/search/list_grouped/list_by_ids/status_counts/locations_autocomplete/autocomplete/export_csv`) | service | CRUD-read | `request_service.rs::list` (caller-threaded read, same crate) | exact (mechanism), role-match (resource) |
| `crates/trackly-app/src/services/act_service.rs` (`get/search/list/counts`, + peek_next_number/suggest_person if reachable read-only) | service | CRUD-read | `request_service.rs::list` | exact (mechanism) |
| `crates/trackly-app/src/services/cartridge_service.rs` (`get/list/status_counts/search/get_history`, model_list/model_get/suggest_*) | service | CRUD-read | `request_service.rs::list` + `request_service.rs::get_history` (for the cartridge `get_history` ownership-shaped method) | exact (mechanism) |
| `crates/trackly-app/src/services/printer_service.rs` (`list/get/current_cartridge_for_printer`) | service | CRUD-read | `printer_service.rs::create_from_device`/`discover` (same file, mutation already calls `authorize(&Action::ReadPrinters` is the matrix target, pattern is `MutatePrinters`) | exact (mechanism), already in-file |
| `crates/trackly-app/src/services/report_service.rs` (`list_device_*`, `list_cartridge_*`, `get_report_counts`, `export_csv`, `export_pdf`) | service | CRUD-read / batch | `request_service.rs::list` | exact (mechanism) |
| `crates/trackly-app/src/services/request_service.rs::list` | service | CRUD-read | itself — already threads `caller`, add ownership override | exact — in-place edit |
| `crates/trackly-app/src/services/request_service.rs::counts/get_history/get` | service | CRUD-read | `request_service.rs::list` (caller param) + `printer_service.rs::acknowledge_alert` (id+caller shape) | exact (mechanism) |
| `crates/trackly-app/src/services/dashboard_service.rs::get_all_widgets` (employee branch) | service | batch/aggregate | itself + `request_service.rs::counts` (request-scoped aggregate query shape) | role-match |
| `crates/trackly-app/src/http/devices.rs`, `acts.rs`, `cartridges.rs`, `printers.rs`, `reports.rs` (read handlers) | route (HTTP) | request-response | `http/requests.rs::handler_list` (already passes `&identity` through) vs `http/devices.rs::handler_list` (currently discards `_identity`) | exact — same file is both the bug and the fix template |
| `crates/trackly-app/src/tauri_cmds/devices.rs`, `acts.rs`, `cartridges.rs`, `printers.rs`, `reports.rs` (`build_*` + `#[tauri::command]` wrappers for reads) | route (Tauri) | request-response | `tauri_cmds/requests.rs::requests_list`/`build_requests_list` (already calls `resolve_tauri_identity` + passes caller) | exact |
| `crates/trackly-app/src/http/requests.rs` (`handler_get`, `handler_counts`, `handler_get_history`) | route (HTTP) | request-response | `http/requests.rs::handler_list` (same file, already-correct sibling) | exact — in-file |
| `crates/trackly-app/src/tauri_cmds/requests.rs` (`requests_get`, `requests_counts`, `requests_get_history`) | route (Tauri) | request-response | `tauri_cmds/requests.rs::requests_list` (same file, already-correct sibling) | exact — in-file |
| `crates/trackly-app/tests/role_endpoint_matrix.rs` | test | integration | itself — extend in place | exact — in-place edit |
| `ui/src/features/layout/EmployeeLayout.svelte` (NEW) | component (layout shell) | request-response (renders) | `ui/src/features/layout/Layout.svelte` (structural template: skip-link + landmark) | role-match (deliberately NOT a branch of it, per UI-SPEC) |
| `ui/src/pages/AccessDenied.svelte` (NEW) | component (page) | request-response (renders) | `ui/src/pages/NotFound.svelte` | exact (UI-SPEC names it the structural template) |
| `ui/src/App.svelte` (shell selection by role) | component (root) | request-response | itself — add role branch next to existing `<Layout>` render | exact — in-place edit |
| `ui/src/routes.ts` / route-guard mechanism | route (frontend) | request-response | `ui/src/App.svelte` (role read from `authStore.user.role`) + `sidebar-config.ts::getVisibleItems` (allowlist pattern) | role-match |
| `ui/src/lib/api/client.ts` (403 branch) | utility (API transport) | request-response | itself — symmetric addition next to existing 401 branch (both HTTP and Tauri code paths) | exact — in-place edit |
| Employee dashboard summary card (folded into `RequestsPage.svelte`/landing) | component | request-response | `ui/src/features/dashboard/StatWidget.svelte` (reused unchanged, no new component) | exact (per UI-SPEC) |

## Pattern Assignments

### Backend: `crates/trackly-core/src/auth.rs` (matrix edit)

**Analog:** itself, current matrix block.

**Current state** (lines 136–157, confirmed by direct read):
```rust
pub fn authorize(identity: &Identity, action: &Action) -> Result<(), AppError> {
    let allowed = match action {
        Action::ManageUsers | Action::ManageSettings => {
            matches!(identity.role, Role::Admin)
        }
        Action::MutateDevices
        | Action::MutateActs
        | Action::MutateCartridges
        | Action::MutatePrinters
        | Action::TransitionRequests
        | Action::ReadPrinters => {
            matches!(identity.role, Role::Admin | Role::Manager)
        }
        Action::ReadData | Action::CreateRequest | Action::ReadRequests => true,
    };
    if allowed { Ok(()) } else { Err(AppError::Forbidden) }
}
```

**Required edit:** move `Action::ReadData` into the `Admin | Manager` arm. `Action::ReadPrinters` is
**already** correctly restricted — do not touch that arm (Pitfall 1 in RESEARCH.md). Leave
`Action::CreateRequest | Action::ReadRequests => true` unchanged — those stay available to Employee.

```rust
Action::MutateDevices
| Action::MutateActs
| Action::MutateCartridges
| Action::MutatePrinters
| Action::TransitionRequests
| Action::ReadPrinters
| Action::ReadData => {                  // ReadData moved here
    matches!(identity.role, Role::Admin | Role::Manager)
}
Action::CreateRequest | Action::ReadRequests => true,   // unchanged
```

Existing test to update: `authorize_employee_read_data_ok` (lines 266–273) currently asserts `is_ok()`
for Employee + `ReadData` — must flip to `Err(Forbidden)` after the matrix edit, mirroring how
`authorize_employee_mutate_devices_forbidden` (lines 252–262) is already structured. Copy that test's
shape verbatim for the new `ReadData` assertion.

---

### Backend: read-method gating — device/act/cartridge/printer/report services

**Analog:** `crates/trackly-app/src/services/request_service.rs::list` (lines 84–106) — the ONE
existing caller-threaded read method in the codebase. This is the template for every newly-gated read
method.

**Imports pattern** (top of `request_service.rs`, lines 13–24):
```rust
use std::sync::Arc;

use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::domain::printers::RequestTransitionOp;
use trackly_core::domain::requests::{Pagination, RequestFilter, RequestNew};
use trackly_core::error::AppError;
use trackly_core::ports::requests::RequestRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
```
`device_service.rs` currently lacks the `use trackly_core::auth::{authorize, Action, Identity};` line
entirely (confirmed — its only `trackly_core` imports are `error::AppError`, `ports::devices`,
`primitives::clock`) — this import must be added to every service file being gated.

**Core read-gate pattern** (the exact shape to replicate, `request_service.rs` lines 84–106 — the
`exclude_ad_register` line is the *existing* role-branch precedent; `authorize()` would be the *new*
addition right before it):
```rust
pub async fn list(
    &self,
    filter: RequestFilter,
    page: Pagination,
    caller: &Identity,
) -> Result<RequestListResponse, AppError> {
    let exclude_ad_register = !matches!(caller.role, trackly_core::auth::Role::Admin);
    let readers = self.readers.clone();
    let repo = self.request_repo.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        let (rows, total) = repo.list(&conn, &filter, &page, exclude_ad_register)?;
        let items = rows.into_iter().map(RequestDto::from).collect();
        Ok(RequestListResponse { items, total: total as i64 })
    })
    .await
    .map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking: {e}") })?
}
```

Applied to `device_service.rs::list` (currently lines 170–212, NO `caller` param, NO `authorize` call —
confirmed by direct read), the gated version becomes:
```rust
pub async fn list(
    &self,
    filter: DeviceFilter,
    page: Pagination,
    caller: &Identity,                       // NEW param — add to every read method
) -> Result<DeviceListResponse, AppError> {
    authorize(caller, &Action::ReadData)?;   // NEW — first line, before any pagination validation
    if page.limit > 200 {
        return Err(AppError::Validation { /* unchanged */ });
    }
    let readers = self.readers.clone();
    let repo = self.repo.clone();
    // ... unchanged spawn_blocking body
}
```
**Critical ordering point** confirmed from RESEARCH.md Architecture Patterns: `authorize()` must run
**before** `self.readers.clone()`/`spawn_blocking`/`readers.acquire()` — a forbidden caller must never
touch the reader pool. In `device_service.rs::list`, that means `authorize()?` goes before even the
existing `page.limit > 200` validation check, as the very first statement in the method body.

**Error handling pattern:** identical across all read methods — `authorize()` returns
`Result<(), AppError>`; propagate with `?`. No new error variant needed; `AppError::Forbidden` already
maps to HTTP 403 (confirmed via `error.rs` `code()` = `"FORBIDDEN"`, `error_axum.rs` mapping cited in
RESEARCH.md).

**Apply this exact pattern to every currently-ungated read method** (confirmed via direct grep — none
of these currently call `authorize` nor accept `caller`):
- `device_service.rs`: `get`, `list`, `search`, `list_grouped`, `list_by_ids`, `status_counts`,
  `locations_autocomplete`, `autocomplete`, `export_csv` — all gated with `Action::ReadData`.
- `act_service.rs`: `get` (line 909), `search` (943), `list` (1006), plus `counts`-style aggregate
  methods if present — gated with `Action::ReadData`.
- `cartridge_service.rs`: `get` (325), `list` (339), `status_counts` (367), `search` (411),
  `get_history` (440) — gated with `Action::ReadData`.
- `printer_service.rs`: `list` (144), `get` (167), `current_cartridge_for_printer` (208) — gated with
  `Action::ReadPrinters` (matrix arm already correct; only the call sites are missing — see Pitfall 1).
- `report_service.rs`: `list_device_acts` (222), `list_device_returns` (241), `list_device_in_use`
  (260), `list_device_in_stock` (276), `list_cartridge_consumption` (296), `list_cartridge_refills`
  (315), `list_cartridge_in_use` (340), `list_cartridge_in_stock` (356), `get_report_counts` (380) —
  gated with `Action::ReadData`.

---

### Backend: `request_service.rs` — D-REQ-01 own-requests filter + ownership checks

**Analog:** itself, `list()` (exists, needs ownership override added) + `printer_service.rs::acknowledge_alert`
(lines 279–299, the simplest example of an `id + caller: &Identity` signature shape in the codebase,
for the `get`/`get_history` ownership-check pattern).

**Server-side override pattern** (RESEARCH.md Pattern 2, builds directly on the existing
`exclude_ad_register` precedent already in this method):
```rust
pub async fn list(
    &self,
    mut filter: RequestFilter,                 // mut — server overrides fields
    page: Pagination,
    caller: &Identity,
) -> Result<RequestListResponse, AppError> {
    authorize(caller, &Action::ReadRequests)?;  // ReadRequests stays true for Employee
    // D-REQ-01: force, don't merely default — ignore whatever client sent.
    if matches!(caller.role, trackly_core::auth::Role::Employee) {
        filter.requested_by_user_id = caller.user_id;
    }
    let exclude_ad_register = !matches!(caller.role, trackly_core::auth::Role::Admin);
    // ... unchanged body — repo.list() already accepts requested_by_user_id via
    //     parameterized SQL ("(?4 IS NULL OR r.requested_by_user_id = ?4)",
    //     crates/trackly-infra/src/repos/requests_sqlite.rs ~line 95).
}
```

**Ownership check for single-resource reads** (`get`/`get_history`, currently lines 66–78 and 130–164
— neither takes `caller` today):
```rust
pub async fn get_history(
    &self,
    request_id: i64,
    caller: &Identity,                          // NEW param
) -> Result<Vec<RequestHistoryEntryDto>, AppError> {
    authorize(caller, &Action::ReadRequests)?;
    if matches!(caller.role, trackly_core::auth::Role::Employee) {
        let owner_id = /* fetch owner_id of request_id, e.g. via self.get(request_id).await?.requested_by_user_id */;
        if Some(owner_id) != caller.user_id {
            return Err(AppError::Forbidden);
        }
    }
    // ... unchanged body
}
```
Apply the identical branch to `get(id)` (Pitfall 2 / Assumption A3 in RESEARCH.md — treat as in-scope
even though CONTEXT.md's text only names `list`/`counts`/`get_history`).

**`counts()`** (lines 109–127, currently no `caller` param) needs the same own-requests scoping as
`list()` if employee-scoped counts are surfaced anywhere outside the dashboard — confirm against
D-GATE-03's dashboard design (the dashboard gets its own narrower query per the next section, so
`counts()` may only need the `caller` param + `authorize(ReadRequests)` without an ownership filter, OR
the filter, depending on whether any UI path calls bare `counts()` for an employee — verify call sites
during planning).

---

### Backend: `dashboard_service.rs` — D-GATE-03 employee-scoped widgets

**Analog:** itself (`get_all_widgets`, lines 51–276 — single `spawn_blocking` aggregate) + the request
counts shape already inside it (lines 170–225, the `request_counts_open/in_progress/completed` query)
as the literal SQL to reuse for an employee-only narrower path.

**Anti-pattern explicitly flagged** (RESEARCH.md Pitfall 4 / Common Pitfalls): do NOT compute the
full `DashboardWidgetDto` (devices_by_status, cartridge_by_status, printer_* fields) and merely hide
fields in the frontend — the org-wide data would already have crossed the network. The employee path
must be a genuinely separate query that never selects from `devices`, `cartridges`, `cartridge_models`,
or `printers`/`printer_alerts` tables.

**Recommended shape** — early branch inside `get_all_widgets()` (or a new sibling method
`get_employee_widgets(caller)`), reusing only the request-counts SQL block already present:
```rust
pub async fn get_all_widgets(
    &self,
    period: Option<PeriodDto>,
    caller: &Identity,                         // NEW param
) -> Result<DashboardWidgetDto, AppError> {
    if matches!(caller.role, trackly_core::auth::Role::Employee) {
        // D-GATE-03: never touch devices/cartridges/printers tables for Employee.
        return self.get_employee_widgets(caller, period).await;
    }
    // ... existing Admin/Manager body, unchanged
}

async fn get_employee_widgets(
    &self,
    caller: &Identity,
    period: Option<PeriodDto>,
) -> Result<DashboardWidgetDto, AppError> {
    let readers = self.readers.clone();
    let user_id = caller.user_id;
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        // SELECT r.status, COUNT(r.id) FROM requests r
        //   WHERE r.deleted_at_utc IS NULL AND r.requested_by_user_id = ?1
        //   GROUP BY r.status   (mirrors existing request_counts_* block, lines 196-202,
        //                        but with the requested_by_user_id filter added)
        // ... zero/None for devices_total, cartridge_by_status, printer_* fields.
    })
    .await
    .map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking: {e}") })?
}
```
Whether to reuse the same `DashboardWidgetDto` shape (zeroed non-request fields) or introduce a
narrower `EmployeeDashboardDto` is planner's discretion per CONTEXT.md — either way, the SQL query
itself must never reference the gated tables for an Employee caller.

---

### Backend: HTTP transport — stop discarding `_identity` on reads

**Analog (the bug, in the same file as the fix):** `crates/trackly-app/src/http/devices.rs::handler_list`
(lines 138–151) vs `crates/trackly-app/src/http/requests.rs::handler_list` (lines 67–80) — the latter is
already correct and is the literal template.

**Current buggy pattern** (`http/devices.rs`, confirmed lines 138-151):
```rust
pub async fn handler_list(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(payload): Json<ListPayload>,
) -> Result<Json<DeviceListResponse>, AppErrorResponse> {
    let _identity = session_identity(&session)          // computed, discarded
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_devices_list(&ctx, payload.filter, payload.pagination)   // no caller passed
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}
```

**Already-correct template** (`http/requests.rs::handler_list`, lines 67–80):
```rust
pub async fn handler_list(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<ListPayload>,
) -> Result<Json<RequestListResponse>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_requests_list(&ctx, &identity, p.filter, p.pagination)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}
```
**Fix:** rename `_identity` → `identity` and thread it into every `build_devices_*`/`build_acts_*`/
`build_cartridges_*`/`build_printers_*`/`build_reports_*` read call, mirroring this exact diff shape,
across `http/devices.rs`, `http/acts.rs`, `http/cartridges.rs`, `http/printers.rs`, `http/reports.rs`.
Also fix the three already-broken `request_service` siblings in `http/requests.rs` itself:
`handler_get` (lines 82–95), `handler_counts` (161–173), `handler_get_history` (189–202) — all three
currently discard `_identity` and call the un-threaded `build_requests_get`/`build_requests_counts`/
`build_requests_get_history`.

---

### Backend: Tauri transport — add missing `resolve_tauri_identity()` calls on reads

**Analog (correct sibling in same file):** `tauri_cmds/requests.rs::requests_list` (lines 121–130) vs
`tauri_cmds/devices.rs::build_devices_list`/no Tauri wrapper shown with identity resolution at all for
reads (confirmed: `devices.rs`'s `build_devices_list`, lines 27-33, takes no `caller` and is called from
a Tauri wrapper with **zero** `resolve_tauri_identity()` call — distinct gap from the HTTP side, per
RESEARCH.md Pitfall 5).

**Already-correct template** (`tauri_cmds/requests.rs`, lines 121–130):
```rust
#[tauri::command]
#[specta::specta]
pub async fn requests_list(
    state: tauri::State<'_, AppCtx>,
    filter: RequestFilter,
    pagination: Pagination,
) -> Result<RequestListResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_requests_list(state.inner(), &caller, filter, pagination).await
}
```

**Required fix shape for `devices.rs` (and act/cartridge/printer/report siblings)** — both the `build_*`
helper signature AND the `#[tauri::command]` wrapper need the new param:
```rust
// build_* helper — shared with axum, gains caller param
pub async fn build_devices_list(
    ctx: &AppCtx,
    caller: &Identity,                          // NEW
    filter: DeviceFilter,
    pagination: Pagination,
) -> Result<DeviceListResponse, AppError> {
    ctx.devices.list(filter, pagination, caller).await   // pass through to service
}

// Tauri wrapper — NEW resolve_tauri_identity() call (entirely absent today)
#[tauri::command]
#[specta::specta]
pub async fn devices_list(
    state: tauri::State<'_, AppCtx>,
    filter: DeviceFilter,
    pagination: Pagination,
) -> Result<DeviceListResponse, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;   // NEW — was entirely missing
    build_devices_list(state.inner(), &caller, filter, pagination).await
}
```
The `resolve_tauri_identity` import already exists in every `tauri_cmds/*.rs` file (`use
crate::tauri_cmds::users::resolve_tauri_identity;` — confirmed present in `devices.rs` line 13), so no
new import is needed — only the call site is missing on read commands.

Also fix the three request-side Tauri siblings that don't call `resolve_tauri_identity` for reads:
`requests_get` (134–139), `requests_counts` (178–182), `requests_get_history` (192–199) — none
currently resolve an identity at all (confirmed by direct read).

---

### Test: `crates/trackly-app/tests/role_endpoint_matrix.rs` — extend + flip Case 9

**Analog:** itself — existing 9-case matrix, `new_app!()` macro (lines 243–248), `post_with_cookie`
helper (lines 86–107), session-cookie bootstrap (lines 53–83). This is the only test file touched.

**Flip required** (Case 9, lines 392–408 — currently asserts the bug as correct behavior):
```rust
// BEFORE (current, asserts the bug):
assert_eq!(
    status,
    StatusCode::OK,
    "Case 9: Employee → devices_list (read) → expected 200, got {status}"
);
// AFTER (post-Phase-10, asserts the fix):
assert_eq!(
    status,
    StatusCode::FORBIDDEN,
    "Case 9 (post-Phase-10): Employee → devices_list (read) → expected 403, got {status}"
);
```

**Body-aware helper needed** (RESEARCH.md Wave 0 Gaps) — current `post_with_cookie` (lines 86–107)
discards the response body, returning only `StatusCode`. D-REQ-01 ("only own requests returned") and
D-GATE-03 ("no org-wide fields present") both require asserting response **content**, not just status.
New helper, same shape, added alongside the existing one:
```rust
async fn post_with_cookie_json(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
    cookie: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(c) = cookie {
        builder = builder.header("cookie", c);
    }
    let req = builder.body(Body::from(serde_json::to_string(&body).unwrap())).unwrap();
    let res = app.oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}
```

**New cases to add** (same `macro_rules! new_app!()` + cookie fixtures already in scope): Employee→403
on `acts_list`/`cartridges_list`/`printers_list`/a representative `reports_list_*` endpoint (mirrors
Case 2's shape exactly, just swapping the URI and using a read-payload instead of a create-payload);
Employee→200 on `requests_list` with body assertion (own requests only, via
`post_with_cookie_json`); Employee→403/404 on `requests_get_history` for another user's `request_id`;
Employee→200 on `dashboard_get_all_widgets` with body assertion (no `devices_total`/`cartridge_by_status`/
`printer_*` fields present, or present-but-zeroed depending on chosen DTO shape).

---

### Frontend: `ui/src/features/layout/EmployeeLayout.svelte` (NEW)

**Analog:** `ui/src/features/layout/Layout.svelte` (structural template — skip-link + landmark pattern,
NOT extended/branched, per UI-SPEC explicit decision for "separate dedicated shell").

**Structural template to copy from** (`Layout.svelte`, full file, lines 1–61):
```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Sidebar from './Sidebar.svelte';

  interface Props {
    children?: Snippet;
  }

  const { children }: Props = $props();
</script>

<a href="#main" class="skip-link">Перейти к основному содержимому</a>

<div class="app-layout">
  <aside class="sidebar-container">
    <Sidebar />
  </aside>
  <main id="main" class="content">
    {@render children?.()}
  </main>
</div>
```
The `.skip-link` CSS block (lines 44–60) must be copied **verbatim** into `EmployeeLayout.svelte` — UI-SPEC
explicitly requires accessibility parity. Replace the `.app-layout` grid (`grid-template-columns:
var(--sidebar-width) 1fr`) with a single-column flex layout per UI-SPEC's exact markup:
```svelte
<a class="skip-link" href="#main">Перейти к основному содержимому</a>
<div class="employee-shell">
  <header class="employee-header">
    <span class="employee-brand">Trackly</span>
    <div class="employee-header-actions">
      <span class="user-name">{fullName}</span>
      <span class="user-role">Сотрудник</span>
      <ThemeSwitcher />
      <Button variant="ghost" size="sm" onclick={logout}>Выйти</Button>
    </div>
  </header>
  <main id="main" class="employee-content">
    {@render children?.()}
  </main>
</div>
```

**Logout pattern to copy** (`Sidebar.svelte`, lines 25–38 — the `logout()` function body, NOT the
hand-rolled `.logout-btn` CSS, since UI-SPEC directs EmployeeLayout to use the shared `Button.svelte`
component instead):
```typescript
async function logout() {
  if (loggingOut) return;
  loggingOut = true;
  try {
    await apiCall<null>('auth_logout', {});
  } catch {
    // Even if the server call fails, drop the local session so the user can re-authenticate.
  } finally {
    authStore.user = null;
    loggingOut = false;
    window.location.hash = '#/login';
  }
}
```
`canLogout`/`loggingOut` state pattern (`Sidebar.svelte` lines 21–23) and `ROLE_LABELS` constant
(`Sidebar.svelte` lines 12–16, reuse the `Сотрудник` value, no need to import the whole map for a
single-role shell) also transfer directly.

**ThemeSwitcher import** — reused unchanged: `import ThemeSwitcher from '$lib/components/ThemeSwitcher.svelte';`
(confirmed import path, `Sidebar.svelte` line 7).

---

### Frontend: `ui/src/pages/AccessDenied.svelte` (NEW)

**Analog:** `ui/src/pages/NotFound.svelte` — UI-SPEC names this explicitly as the structural template
("near-exact copy... changing only heading/body/CTA copy and the CTA's destination hash").

**Full template to copy from** (`NotFound.svelte`, complete file, lines 1–52):
```svelte
<script lang="ts">
  import Button from '$lib/components/Button.svelte';

  interface Props {
    location?: { hash: string };
  }

  const { location }: Props = $props();
  const hashPath = $derived(location?.hash ?? '');
</script>

<div class="not-found">
  <h2 class="not-found-heading">Страница не найдена</h2>
  <p class="not-found-body">
    Раздел <code>{hashPath}</code> не существует. Откройте навигацию слева.
  </p>
  <Button variant="secondary" onclick={() => (window.location.hash = '/')}>На главную</Button>
</div>

<style lang="scss">
  .not-found {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-2xl);
    text-align: center;
    min-height: 300px;
    gap: var(--space-md);
  }

  .not-found-heading {
    margin: 0;
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .not-found-body {
    margin: 0;
    font-size: var(--font-size-body);
    color: var(--color-text-secondary);

    code {
      font-family: monospace;
      background: var(--color-surface-sunken);
      padding: 2px 6px;
      border-radius: var(--radius-sm);
    }
  }
</style>
```

**Required diff per UI-SPEC** — rename classes `.not-found*` → `.access-denied*` (CSS values stay
byte-identical, just selector renamed), drop the `location`/`hashPath` prop (not needed — copy is
static), change heading/body copy, change button `variant` (stays `secondary`) and `onclick` target:
```svelte
<div class="access-denied">
  <h2 class="access-denied-heading">Нет доступа</h2>
  <p class="access-denied-body">
    У вашей роли («Сотрудник») нет доступа к этому разделу. Доступны только заявки.
  </p>
  <Button variant="secondary" onclick={() => (window.location.hash = '#/requests')}>
    К заявкам
  </Button>
</div>
```
No icon, text-only — matches `NotFound.svelte`'s existing icon-free convention (UI-SPEC explicit).

---

### Frontend: `App.svelte` — role-conditional shell selection

**Analog:** itself (lines 54–65) — single unconditional `<Layout>` render today, needs a role branch.

**Current pattern** (lines 54–65):
```svelte
{#if appLoading}
  <div class="app-loading">Загрузка...</div>
{:else if bootstrapNeeded && !authStore.user}
  <FirstRunWizard />
{:else if !authStore.user}
  <LoginPage />
{:else}
  <Layout>
    <Router {routes} />
  </Layout>
{/if}
```

**Required edit** — branch the final `{:else}` on `authStore.user.role`, mirroring the existing
if/else-if chain shape (no new control-flow construct, just one more branch):
```svelte
{:else if authStore.user.role === 'employee'}
  <EmployeeLayout>
    <Router routes={employeeRoutes} />
  </EmployeeLayout>
{:else}
  <Layout>
    <Router {routes} />
  </Layout>
{/if}
```
Import `EmployeeLayout` from `./features/layout/EmployeeLayout.svelte` alongside the existing `Layout`
import (line 5). `authStore.user.role` is already read elsewhere in this exact file (line 31,
`role: status.user.role as UserRole`) — same `UserRole` type import (line 11) covers the new branch,
no new type needed.

---

### Frontend: `routes.ts` — employee route allowlist / route-guard

**Analog:** `routes.ts` itself (the route map to reduce) + `sidebar-config.ts::getVisibleItems` (lines
38–45, the existing allowlist-filter mechanism pattern, even though UI-SPEC's D-UI-01 decision is a
separate shell rather than a filtered sidebar — the *allowlist* idea generalizes to the route map).

**Current full route map** (`routes.ts`, lines 14–27):
```typescript
export const routes = {
  '/': Dashboard,
  '/login': LoginPage,
  '/map': MapPage,
  '/devices': DevicesPage,
  '/acts': ActsPage,
  '/printers': PrintersPage,
  '/cartridges': CartridgesPage,
  '/requests': RequestsPage,
  '/reports': ReportsPage,
  '/users': UsersPage,
  '/settings': SettingsPage,
  '*': NotFound,
} as const;
```

**Recommended employee route map** (new export, same file or a sibling `employee-routes.ts`, per
UI-SPEC §Layout & Component Contract "Route map for employee"):
```typescript
export const employeeRoutes = {
  '/': RequestsPage,            // employee landing = "Мои заявки", same component, reused
  '/access-denied': AccessDenied,
  '*': AccessDenied,            // every other hash → "Нет доступа", NOT NotFound
} as const;
```
Per UI-SPEC: a forbidden route is a deliberately different message from an unmapped route — `'*'` for
the employee map points at `AccessDenied`, not `NotFound` (which stays reserved for the
admin/manager route map's truly-unmapped-hash case, unchanged).

**Allowlist precedent to reference for the guard mechanism** (`sidebar-config.ts`, lines 38–45):
```typescript
export function getVisibleItems(role: UserRole | null): SidebarEntry[] {
  return SIDEBAR_ITEMS.filter((entry) => {
    if (entry.kind === 'divider') return true;
    if (!entry.roles) return true;
    if (role === null) return false;
    return entry.roles.includes(role);
  });
}
```
This `roles?: UserRole[]` + filter-by-role shape is the existing project convention for role-based
visibility — even though D-UI-01 chose a separate route map over filtering the existing one, the
underlying "check role before granting access" idiom is the same one already used here. RESEARCH.md's
Open Question #3 (exact `svelte-spa-router` guard API — `wrap()`/`conditionsFailed`) is unresolved;
recommend the simpler `App.svelte`-level branch (swap `routes` prop based on role, as shown above)
over a per-route `wrap()` guard, since it requires zero new router API surface and is directly
analogous to the existing `{#if authStore.user.role === 'employee'}` branch already being added.

---

### Frontend: `ui/src/lib/api/client.ts` — 403 handling

**Analog:** itself — symmetric addition next to the existing 401 branch in both code paths (Tauri
`invoke` catch block, lines 11–22; HTTP `fetch` branch, lines 30–40).

**Current full file** (confirmed, all 42 lines):
```typescript
import { parseAppError } from './errors';
import { authStore } from '$lib/stores/auth.svelte';

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export async function apiCall<R>(name: string, args: Record<string, unknown> = {}): Promise<R> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      return await invoke<R>(name, args);
    } catch (e) {
      const err = parseAppError(e);
      // Tauri errors don't have HTTP status codes; check error code for auth errors.
      if (err && typeof err === 'object' && 'code' in err) {
        const code = (err as { code: string }).code;
        if (code === 'UNAUTHORIZED' || code === 'Unauthorized') {
          authStore.user = null;
          if (typeof window !== 'undefined') window.location.hash = '#/login';
        }
      }
      throw err;
    }
  }
  // Phase 5+ HTTP path.
  const res = await fetch(`/api/v1/${name}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(args),
  });
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    const err = parseAppError(body);
    // 401 → clear auth and redirect to login.
    if (res.status === 401) {
      authStore.user = null;
      if (typeof window !== 'undefined') window.location.hash = '#/login';
    }
    // 403 → throw only (user is authenticated, just forbidden).
    throw err;
  }
  return res.json();
}
```

**Required additions** — symmetric `code === 'FORBIDDEN'` check in the Tauri branch (mirroring the
existing `'UNAUTHORIZED' || 'Unauthorized'` dual-string check shape, confirmed exact backend string
`AppError::Forbidden.code()` = `"FORBIDDEN"` per `crates/trackly-core/src/error.rs` line 167), and a
`res.status === 403` branch in the HTTP path — both toast-only, no redirect (UI-SPEC explicit: a
mid-session 403 should not yank the user off a page they're legitimately on):
```typescript
// Tauri branch — add alongside the existing UNAUTHORIZED check:
if (code === 'FORBIDDEN' || code === 'Forbidden') {
  pushToast('error', 'Недостаточно прав для этого действия');
}

// HTTP branch — add alongside the existing res.status === 401 check:
if (res.status === 403) {
  pushToast('error', 'Недостаточно прав для этого действия');
}
```
New import required: `import { pushToast } from '$lib/stores/toast.svelte';` (confirmed export exists
at `ui/src/lib/stores/toast.svelte.ts` line 22 — `export function pushToast(kind: ToastKind, message:
string): void`). No circular-import risk — `RequestsPage.svelte` already imports both `authStore` and
the toast store side-by-side without issue (per UI-SPEC's explicit confirmation note).

---

### Frontend: Employee dashboard summary card — reuse `StatWidget.svelte` unchanged

**Analog:** `ui/src/features/dashboard/StatWidget.svelte` — UI-SPEC mandates exact reuse, zero new
component.

**Props contract to satisfy** (confirmed, lines 11–20):
```typescript
interface Props {
  id: string;
  title: string;
  mainNumber: number | null;
  mainLabel: string;
  breakdown: BreakdownRow[];   // { label: string; count: number }[]
  loading: boolean;
  error: string | null;
  warningItems?: string[];     // DO NOT use for employee card — that's the low-stock mechanism
}
```
Usage in the employee landing page (per UI-SPEC §D-GATE-03):
```svelte
<StatWidget
  id="employee-requests"
  title="Мои заявки"
  mainNumber={openPlusInProgressCount}
  mainLabel="активных заявок"
  breakdown={[
    { label: 'Новые', count: openCount },
    { label: 'В работе', count: inProgressCount },
    { label: 'Выполнено', count: completedCount },
  ]}
  loading={dashboardLoading}
  error={dashboardError}
/>
```
Omit `warningItems` entirely (don't pass an empty array either — UI-SPEC: "do not wire it even if
empty"). Data source: the new employee-scoped `dashboard_service` method/branch (see backend section
above), not the org-wide `get_all_widgets()` response.

## Shared Patterns

### Service-layer `authorize()` gate (the single most-replicated pattern this phase)
**Source:** `crates/trackly-app/src/services/request_service.rs::create`/`transition` (lines 178, 245)
— the two existing call sites of `authorize()` in a service method.
**Apply to:** every read method in device/act/cartridge/printer/report services, plus
`request_service::get`/`counts`/`get_history`.
```rust
authorize(caller, &Action::ReadData)?;   // first statement in the method body,
                                          // before any spawn_blocking/readers.acquire()
```

### `caller: &Identity` threading through both transports
**Source:** `request_service.rs::list` (service) + `http/requests.rs::handler_list` (HTTP) +
`tauri_cmds/requests.rs::requests_list`/`build_requests_list` (Tauri) — the one complete,
already-correct three-layer example in the codebase.
**Apply to:** all newly-gated read methods, their HTTP handlers, and their Tauri command wrappers. The
HTTP side needs only a rename (`_identity` → `identity`) since `session_identity()` is already called
everywhere; the Tauri side needs a brand-new `resolve_tauri_identity()` call on most read commands
(distinct gap, see Pitfall 5).

### Server-side ownership override (never trust client-supplied scoping fields)
**Source:** `request_service.rs::list`'s `exclude_ad_register` line (existing precedent for
role-derived query modification) — D-REQ-01 extends this exact idiom to
`filter.requested_by_user_id`.
**Apply to:** `request_service::list`/`counts`/`get_history`/`get`, and the employee dashboard branch
in `dashboard_service`.

### 401/403 chokepoint in `client.ts`
**Source:** `ui/src/lib/api/client.ts` — single function, both transports, already has the 401
precedent.
**Apply to:** add 403 toast-only handling symmetric to the existing 401 redirect handling; no per-page
boilerplate needed anywhere else in the frontend.

### Russian-only copy + existing role-label conventions
**Source:** `Sidebar.svelte`'s `ROLE_LABELS` map (`Сотрудник` for employee) — reused verbatim, not
redefined, in `EmployeeLayout.svelte`.
**Apply to:** `EmployeeLayout.svelte`, `AccessDenied.svelte` — all copy is final per UI-SPEC §Copywriting
Contract, use verbatim.

## No Analog Found

None. Every file in scope for this phase has a same-codebase analog — this phase is explicitly
described in RESEARCH.md as "mechanical replication of an established pattern," not new design, and
that holds for both backend and frontend surfaces.

## Metadata

**Analog search scope:** `crates/trackly-core/src/`, `crates/trackly-app/src/services/`,
`crates/trackly-app/src/http/`, `crates/trackly-app/src/tauri_cmds/`, `crates/trackly-app/tests/`,
`ui/src/App.svelte`, `ui/src/routes.ts`, `ui/src/features/layout/`, `ui/src/features/dashboard/`,
`ui/src/features/requests/`, `ui/src/pages/`, `ui/src/lib/api/`, `ui/src/lib/stores/`.
**Files scanned:** 19 source files read directly (full or targeted ranges) + grep sweeps across
`act_service.rs`/`cartridge_service.rs`/`report_service.rs` to confirm zero existing `authorize()` call
sites (consistent with RESEARCH.md's finding).
**Pattern extraction date:** 2026-06-21
