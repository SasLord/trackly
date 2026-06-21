# Phase 11: Заявки/employee UX gap-closure - Pattern Map

**Mapped:** 2026-06-21
**Files analyzed:** 14 (modify) + 1 (new component) + bindings regen
**Analogs found:** 14 / 14 (every change has a verified in-repo analog)

This phase is "fill the grooves already cut" — every new field, endpoint, and handler
copies an existing sibling. No new crates/npm packages. No DB migration (V024 already
seeds `request_categories`; `devices`/`locations` exist).

## File Classification

| File | New/Mod | Role | Data Flow | Closest Analog | Match |
|------|---------|------|-----------|----------------|-------|
| `crates/trackly-infra/src/repos/requests_sqlite.rs` | mod | repo (read) | CRUD/read-JOIN | self (`requester_name`/`printer_name` joins) | exact |
| `crates/trackly-core/src/domain/requests.rs` (`RequestRow`) | mod | model | transform | self (`requester_name` field) | exact |
| `crates/trackly-app/src/dto/request.rs` (`RequestDto`, new `RequestPrinterOptionDto`) | mod | DTO | transform | self (`requester_name`) + `PrinterDto` | exact |
| `crates/trackly-app/src/dto/printer.rs` (`WsEvent`, `is_visible_to`) | mod | DTO+guard | event-driven | self (`is_visible_to` match arms) | exact |
| `crates/trackly-app/src/services/request_service.rs` (3 send sites + new `printer_options`) | mod | service | event-driven + read | self (`transition`/`list`) | exact |
| `crates/trackly-app/src/tauri_cmds/requests.rs` (new `request_printer_options`; maybe categories `{id,name}`) | mod | controller (Tauri) | request-response | self (`build_requests_list` + `requests_list`) | exact |
| `crates/trackly-app/src/http/requests.rs` (new handler+route) | mod | controller (axum) | request-response | self (`handler_list` + router) | exact |
| `crates/trackly-app/src/specta_export.rs` | mod | config | — | self (`collect_commands!` entries) | exact |
| `crates/trackly-infra/src/repos/devices_sqlite.rs` | read-only ref | repo (read) | read-JOIN | `SELECT_DEVICES` LEFT JOIN locations | exact (source for printer query) |
| `ui/src/bindings*.ts` | regen | config | — | tooling (`cargo test export_bindings`) | n/a |
| `ui/src/features/requests/RequestDetail.svelte` | mod | component | render | self (lines 387-391 category block) | exact |
| `ui/src/features/requests/RequestFormModal.svelte` | mod | component | request-response | self (`loadPrinters`, Select usage) | exact |
| `ui/src/features/requests/RequestsPage.svelte` | mod (or move) | component | event-driven | self (`handleWsEvent`, `onMount` connectWs) | exact |
| `ui/src/features/requests/api.ts` | mod | client | request-response | self (`requests.create`) | exact |
| `ui/src/features/layout/EmployeeLayout.svelte` | mod | component (shell) | event-driven | `RequestsPage` onMount WS pattern | role-match |
| `ui/src/lib/components/GroupedPrinterSelect.svelte` | NEW | component | render | `Select.svelte` (markup+SCSS tokens) | role-match |

---

## Pattern Assignments

### `requests_sqlite.rs` — add `category_name` via LEFT JOIN (D-CAT-01)

**Analog:** itself — the existing `requester_name`/`printer_name` joins.

**Current SELECT** (lines 26-39): joins `users u` (→`u.full_name AS requester_name`, idx 11)
and `devices d` (→`d.name AS printer_name`, idx 12). `map_row_request` (lines 44-65) maps
idx 0..17; last existing column is `r.ad_subtype` at **idx 17**.

**Apply (append LAST — never insert mid-list; Pitfall 2):**
- In `SELECT_REQUESTS` add column AFTER `r.ad_subtype`: `rc.name AS category_name`
- Add join: `LEFT JOIN request_categories rc ON rc.id = r.category_id`
- In `map_row_request` add: `category_name: row.get(18)?,` (new last index)
- One edit covers all read paths — `get`, `list`, `fetch_in_tx` all reuse `SELECT_REQUESTS` + `map_row_request`.

**Test pattern to extend** (lines 435-477 `test_request_repo_create`): seed a `free_form`
request with `category_id = Some(3)`, assert `row.category_name == Some("Программное обеспечение")`;
seed one with `None` → assert `None`. Helper `fresh_conn()` + `seed_user()` already exist.

---

### `RequestRow` (domain) + `RequestDto` — carry `category_name` (D-CAT-01)

**Analog:** the `requester_name`/`printer_name` fields already on both structs.

- `trackly-core/src/domain/requests.rs` `RequestRow`: add `pub category_name: Option<String>,` (mirror `requester_name`). Update the 5 test `RequestNew` constructors in `requests_sqlite.rs` only if `RequestNew` changes — it does NOT (write path untouched), so no test churn there.
- `dto/request.rs` `RequestDto` (lines 17-48): add `pub category_name: Option<String>,` next to `printer_name` (line 37). In `From<RequestRow>` (lines 50-73) add `category_name: r.category_name,`. camelCase → `categoryName` in bindings automatically.

---

### `dto/request.rs` — new `RequestPrinterOptionDto` (D-PRN-01)

**Analog:** `PrinterDto` (printer.rs lines 16-50) for the `#[specta(type = i32)]` on `i64` id + camelCase.

```rust
/// Минимальные опции принтера для формы заявки (D-PRN-01).
/// НЕ раскрывает SNMP/community/ip — только id/name/location (Security V4 BOLA).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RequestPrinterOptionDto {
    #[specta(type = i32)]
    pub id: i64,        // device id → printer_device_id in RequestCreateDto
    pub name: String,
    pub location: Option<String>,
}
```

---

### `dto/printer.rs` — `WsEvent::RequestStatusChanged` + `is_visible_to` (D-WS-01)

**Analog:** the existing enum + `is_visible_to` in the same file (lines 175-217). **Make both edits as ONE atomic change** (Pitfall 1).

**Add field** (current variant lines 187-191):
```rust
RequestStatusChanged {
    #[specta(type = i32)]
    request_id: i64,
    new_status: String,
    #[specta(type = i32)]
    requested_by_user_id: i64,   // NEW — needed by is_visible_to + client filter
},
```

**Edit `is_visible_to`** (current arm, lines 212-214, lumps NewRequest + RequestStatusChanged together — SPLIT them):
```rust
WsEvent::NewRequest { .. } => matches!(identity.role, Role::Admin | Role::Manager),
WsEvent::RequestStatusChanged { requested_by_user_id, .. } => {
    matches!(identity.role, Role::Admin | Role::Manager)
        || identity.user_id == Some(*requested_by_user_id)
}
```
`Identity { user_id: Option<i64>, role }` confirmed in `auth.rs` lines 64-70.

**New unit test** (Wave 0): in `dto/printer.rs` `#[cfg(test)]` — employee-author→true, employee-other→false, admin/manager→true, manager-author-irrelevant→true.

---

### `services/request_service.rs` — fill new field at 3 send sites + new `printer_options` (D-WS-01, D-PRN-01)

**Analog:** itself — the 3 existing `self.ws_tx.send(WsEvent::RequestStatusChanged {...})` calls.

| Send site | Line | Source of `requested_by_user_id` |
|-----------|------|----------------------------------|
| `transition` | 410-413 | `dto.requested_by_user_id` (the `dto` from `self.get` on line 406 is in scope) |
| `approve_ad_register` | 543-546 | `dto.requested_by_user_id` (`dto` from `self.get` line 541) |
| `reject_ad_register` | 661-664 | `dto.requested_by_user_id` (`dto` from `self.get` line 659) |

All three have a `let dto = self.get(request_id, caller).await?;` immediately before the
send, so `dto.requested_by_user_id` is available at every site — uniform edit.

**New `printer_options` read method** — copy the `build_requests_list_categories` reader pattern
(tauri_cmds/requests.rs lines 99-123): `authorize(caller, &Action::CreateRequest)?` then
`tokio::task::spawn_blocking` over `ctx.readers.acquire()`. Query copies `devices_sqlite.rs`
LEFT JOIN locations:
```sql
SELECT d.id, d.name, l.name AS location
  FROM devices d
  LEFT JOIN locations l ON d.location_id = l.id
 WHERE d.type_id = 2 AND d.deleted_at_utc IS NULL
 ORDER BY l.name IS NULL, l.name, d.name
```
Parameterized; no user input (no filter "на первое время"). Reader pool + `spawn_blocking`
per CLAUDE.md single-writer discipline. (`ctx.readers: Arc<ReaderPool>` confirmed, context.rs line 50.)

> **Discretion (RESEARCH §171):** put `printer_options` on `RequestService` (gate = CreateRequest, semantically "options for the request form") rather than on device/printer service.

---

### `tauri_cmds/requests.rs` — `build_request_printer_options` + thin wrapper (D-PRN-01)

**Analog:** `build_requests_list` (lines 32-41) + the thin `requests_list` wrapper (lines 131-138),
and the reader/gate shape of `build_requests_list_categories` (lines 99-123).

```rust
pub async fn build_request_printer_options(
    ctx: &AppCtx, caller: &Identity,
) -> Result<Vec<RequestPrinterOptionDto>, AppError> {
    authorize(caller, &Action::CreateRequest)?;
    ctx.requests.printer_options(caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn request_printer_options(
    state: tauri::State<'_, AppCtx>,
) -> Result<Vec<RequestPrinterOptionDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_request_printer_options(state.inner(), &caller).await
}
```
`#[specta::specta]` MUST follow `#[tauri::command]` (module doc, line 6).

**Optional categories `{id,name}`** (Discretion / A3): change `build_requests_list_categories`
return to `Vec<CategoryDto{id,name}>`, query `SELECT id, name FROM request_categories ORDER BY name`.
If done, replace the hardcoded `CATEGORIES` array in `RequestFormModal.svelte` (lines 41-46) —
do NOT keep both (anti-pattern). RESEARCH says this is optional; the JOIN alone fixes display.

---

### `http/requests.rs` — handler + route (D-PRN-01)

**Analog:** `handler_counts` (lines 161-173, the no-payload variant — printer-options takes no body)
+ router (lines 208-224).

```rust
pub async fn handler_request_printer_options(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<Vec<RequestPrinterOptionDto>>, AppErrorResponse> {
    let identity = session_identity(&session).await.map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_request_printer_options(&ctx, &identity).await.map_err(AppErrorResponse::from)?,
    ))
}
```
Add to `router()`: `.route("/api/v1/request_printer_options", post(handler_request_printer_options))`.
Add `build_request_printer_options` + `RequestPrinterOptionDto` to the `use` lists (lines 16-26).
Router is merged in `http/mod.rs` line 119 via `requests::router()` — no extra wiring.

> **Do NOT** add a `ws_broadcast.send` in this handler (it's a read). The transition/create
> handlers re-broadcast (lines 110-116, 132-137) but a read endpoint must not.

---

### `specta_export.rs` — register new command (D-PRN-01)

**Analog:** lines 114-122 (existing `crate::tauri_cmds::requests::*` entries in `collect_commands!`).
Add: `crate::tauri_cmds::requests::request_printer_options,`. Regenerates `bindings.ts` on
`cargo test` (tests/export_bindings.rs).

---

### `RequestDetail.svelte` — render category name (D-CAT-01)

**Analog:** itself — the `field-label`/`field-value` rows (lines 376-391), and the `printerName ?? '—'` idiom (line 378).

**Replace** lines 387-391 (currently renders `{request.categoryId}`):
```svelte
{#if request.categoryName}
  <div class="field">
    <span class="field-label">Категория</span>
    <span class="field-value">{request.categoryName}</span>
  </div>
{/if}
```
Svelte auto-escapes text (XSS mitigation, Security V5).

---

### `RequestFormModal.svelte` — switch printer source + use grouped dropdown (D-PRN-01)

**Analog:** itself — `loadPrinters` (lines 69-83) and the `<Select>` printer block (lines 174-198).

- Add to `api.ts` client: `printerOptions: () => apiCall<RequestPrinterOptionDto[]>('request_printer_options')` (mirror `listCategories`, api.ts line 29).
- Rewrite `loadPrinters` to call `requests.printerOptions()` (server already sorts by location); drop the `devices.list({type_id:2,...})` call (lines 73-77) and the `devices`/`DeviceDto` imports (lines 12-13).
- Change `availablePrinters` type from `DeviceDto[]` to `RequestPrinterOptionDto[]`.
- Replace the `<Select>` (lines 178-191) with `<GroupedPrinterSelect>` (below). Empty-list copy: "Принтеры не найдены" (discretion).
- **Notification permission gate (D-WS-01, Pitfall 4):** in `handleSubmit` success branch (after `pushToast('success', 'Заявка отправлена')`, line 117), call `maybeRequestNotifyPermission()` — request only after this user gesture, only if `'Notification' in window && window.isSecureContext && Notification.permission === 'default'`.

---

### `GroupedPrinterSelect.svelte` — NEW grouped dropdown (D-PRN-01)

**Analog:** `Select.svelte` (full file) for markup conventions + SCSS design tokens
(`var(--color-border)`, `var(--radius-sm)`, `var(--space-*)`, focus-visible box-shadow).
Group-header background = `var(--color-surface-sunken)` (already used in RequestFormModal
`.type-btn:hover`, line 300 — confirmed token exists).

Grouping (server already sorts by location; group on client for headers):
```ts
const groups = $derived.by(() => {
  const map = new Map<string, RequestPrinterOptionDto[]>();
  for (const p of options) {
    const key = p.location ?? 'Без расположения';
    (map.get(key) ?? map.set(key, []).get(key)!).push(p);
  }
  return [...map.entries()];   // [location, printers[]]
});
```
Group header = small strip with grey background + location text; printers listed beneath
(per user's literal spec, CONTEXT line 112). Props mirror `Select` (`value`, `onchange`, `id`, `invalid`).

---

### `RequestsPage.svelte` / `EmployeeLayout.svelte` — employee WS toast/notification (D-WS-01)

**Analog:** `RequestsPage.svelte` `handleWsEvent` (lines 106-133) + `onMount` connectWs/onWsEvent (lines 135-151). The reconnect/backoff lives in `ws.ts` — DO NOT re-roll it; just `connectWs()` + `onWsEvent(handler)` and return cleanup.

Employee branch in the handler:
```ts
if (event.type === 'request_status_changed') {
  // server is_visible_to already guarantees employee sees only own; client check = UX only
  const text = statusToastText(event.newStatus); // 'Ваша заявка принята в работу' / '... выполнена' / '... отклонена'
  const canNotify = 'Notification' in window && window.isSecureContext && Notification.permission === 'granted';
  if (document.hidden && canNotify) new Notification('Trackly', { body: text });
  else pushToast(event.newStatus === 'rejected' ? 'info' : 'success', text);
}
```

> **Placement (Discretion A2 / Pitfall 5):** RESEARCH recommends hoisting the employee
> subscription to `EmployeeLayout.svelte` (lives the whole session) so notification works
> regardless of current screen. `EmployeeLayout` already imports `apiCall`/`authStore`
> (lines 10-11) and has no WS yet — add `onMount` connectWs/onWsEvent there for employees.
> Avoid double-subscribe: `RequestsPage` already subscribes for admin/manager "new request"
> toasts — guard by role so employee path lives in one place.

RU toast texts are discretion. Graceful-degrade to toast whenever not secure-context (Pitfall 3 — HTTP first-run fallback has no Notification API).

---

## Shared Patterns

### "Один DTO, два транспорта" (dual-transport)
**Source:** `tauri_cmds/requests.rs` `build_*` helpers + thin `#[tauri::command]` wrappers;
`http/requests.rs` handlers delegate to the SAME `build_*`.
**Apply to:** new `request_printer_options` (both transports), optional categories `{id,name}`.
Business logic + `authorize()` live in the service / `build_*`; handlers are thin adapters.

### Read via reader-pool + spawn_blocking (single-writer discipline)
**Source:** `build_requests_list_categories` (tauri_cmds/requests.rs lines 102-123) —
`ctx.readers.acquire()` inside `tokio::task::spawn_blocking`.
**Apply to:** new `printer_options` read. Never touch the writer for reads.

### Server-side authorization is the source of truth (D-RBAC-03)
**Source:** `authorize(caller, &Action::CreateRequest)` (auth matrix auth.rs lines 136-158,
Employee has CreateRequest+ReadRequests only) and `WsEvent::is_visible_to` (dto/printer.rs).
**Apply to:** printer-picker gate = `CreateRequest` (NOT ReadData/ReadPrinters, which Phase 10
closed from employee — do not regress the matrix). WS author-filter MUST be server-side in
`is_visible_to`, never client-only (BOLA). Client `requestedByUserId` check is UX text only.

### Append-LAST when extending a column-indexed SELECT
**Source:** `requests_sqlite.rs` `map_row_request` index discipline; `devices_sqlite.rs`
comment "LEFT JOIN locations добавляет l.name как последний столбец (индекс 15)".
**Apply to:** `category_name` → new last index 18. Mid-list insert silently shifts all `row.get(n)`.

### Dual-transport WS client (browser WS + Tauri listen)
**Source:** `ui/src/lib/api/ws.ts` — `connectWs()` branches on `__TAURI_INTERNALS__`; browser
path does exponential backoff + single reconnect toast (debug fix baked in).
**Apply to:** employee subscription. Reuse `connectWs`/`onWsEvent`; do not build a new channel
or reconnect logic.

### apiCall (dual-transport client) + 403 toast
**Source:** `ui/src/lib/api/client.ts` — `apiCall<R>(name, args)` invokes Tauri or POSTs
`/api/v1/{name}`; 403 → "Недостаточно прав…" toast (D-DENY-01).
**Apply to:** new `requests.printerOptions()` in `api.ts` (mirror `listCategories`, no args).

---

## No Analog Found

None. Every change maps to a verified in-repo sibling. The only genuinely new artifact is
the `GroupedPrinterSelect.svelte` component, which still copies `Select.svelte`'s markup
conventions and SCSS design tokens (analog = role-match, not exact).

## Metadata

**Analog search scope:** `crates/trackly-app/src/{dto,services,tauri_cmds,http,specta_export.rs}`,
`crates/trackly-infra/src/repos`, `crates/trackly-core/src/auth.rs`, `migrations/V024`,
`ui/src/{features/requests,features/layout,lib/api,lib/components}`.
**Files scanned:** ~18 read in full or targeted.
**No DB migration required** (V024 confirmed: `request_categories` seeded with 4 RU names; `category_id`/`printer_device_id` columns exist).
**Pattern extraction date:** 2026-06-21
