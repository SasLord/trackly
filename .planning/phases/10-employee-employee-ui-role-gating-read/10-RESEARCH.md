# Phase 10: Ограничение роли employee — Research

**Researched:** 2026-06-21
**Domain:** Internal RBAC enforcement (Rust service-layer authorization) + Svelte role-based UI shell
**Confidence:** HIGH

## Summary

This phase closes a real security gap, not a UX nicety: read methods across `device_service`,
`act_service`, `cartridge_service`, `printer_service`, `report_service`, and most of
`request_service`/`dashboard_service` call `authorize()` **zero times**. Both transports (axum HTTP
and Tauri commands) resolve an `Identity` for these reads (`session_identity()` on the HTTP side) but
then discard it as `_identity` — the identity is computed and thrown away. On the Tauri side it's
worse: most read commands don't even call `resolve_tauri_identity()`. The `authorize()` matrix itself
is *mostly* correct already — `Action::ReadPrinters` is already bundled with `Mutate*` actions and
restricted to `Admin | Manager`. Only `Action::ReadData` is the literal `true`-for-everyone catch-all
that needs narrowing. **The bug is overwhelmingly "missing enforcement calls," not "wrong matrix
entries."** This distinction matters for planning: don't spend effort re-deriving a matrix that's
already 90% right — spend it threading `caller: &Identity` into ~25 read methods and calling
`authorize()` before each one touches the reader pool.

`request_service.list()` already threads `caller: &Identity` and branches on role for
`exclude_ad_register` — D-REQ-01's "own requests only" filter is a one-line addition reusing the
*already-wired* `RequestFilter.requested_by_user_id: Option<i64>` SQL parameter
(`requests_sqlite.rs` line ~95, parameterized as `(?4 IS NULL OR r.requested_by_user_id = ?4)`).
No DB migration needed — confirmed via `migrations/V006__requests.sql`, which already has
`requested_by_user_id INTEGER NOT NULL REFERENCES users(id)`. `counts()` and `get_history()` on
`request_service` do **not** thread `caller` today and need it added — `get_history()` in particular
has a real authorization gap: even after `list()` is scoped, an employee can call
`requests_get_history(request_id)` with an arbitrary ID and read another user's audit trail, because
the method takes only `request_id` with no ownership check.

On the frontend: `App.svelte` renders exactly one `<Layout>` shell for every authenticated role; there
is no role-based shell branching and no per-route guard. `sidebar-config.ts`'s `getVisibleItems(role)`
already supports `roles: ['admin']` restriction on individual `SidebarItem`s (used today only for
`/users` and `/settings`) — this mechanism generalizes directly to building an employee allowlist, but
CONTEXT.md's D-UI-01 explicitly wants a **separate minimal shell**, not a filtered version of the
admin/manager sidebar. Interestingly, `features/requests/RequestsPage.svelte` *already* computes a
client-side `requestedByUserId` filter for `identity.role === 'employee'` — this is exactly the kind
of UI-only convenience that D-RBAC-03 (Phase 5) warns is not a security boundary: today an employee
who calls the API directly (bypassing this UI filter) sees everyone's requests, because the backend
service does not enforce the restriction itself. D-REQ-01 makes the backend the actual enforcement
point; the existing frontend filter becomes redundant-but-harmless UX once the backend filters
unconditionally for `Role::Employee`.

**Primary recommendation:** Add `authorize()` calls + `caller: &Identity` parameters to all read
service methods (reusing existing `Action::ReadData`/`ReadPrinters`/`ReadRequests` — no new Action
variants needed unless the planner wants per-resource granularity for future-proofing), narrow
`Action::ReadData`'s matrix entry to `Admin | Manager`, add the `requested_by_user_id` filter
unconditionally inside `request_service` for `Role::Employee` (not as a client-supplied filter param),
build a dedicated `EmployeeDashboardDto`/method scoped to the caller's own requests, and build a
genuinely separate `EmployeeLayout.svelte` shell gated by `authStore.user.role === 'employee'` in
`App.svelte`, with a route-guard component and 403-handling added to `client.ts`. Extend
`role_endpoint_matrix.rs`'s existing 9-case matrix with read-endpoint cases — and flip Case 9 (which
currently asserts employee→`devices_list`→200 OK) to 403, since that assertion describes exactly the
bug this phase fixes.

## Project Constraints (from CLAUDE.md)

- Rust backend, Tauri desktop shell, Svelte 5 (runes) frontend, SCSS, SQLite — fixed stack, no new
  languages/frameworks needed for this phase.
- **Dual-transport mandate**: "один DTO, два транспорта" — every capability must work identically via
  Tauri `invoke` and axum HTTP. All `authorize()` calls and ownership filters MUST live in the
  service layer (`crates/trackly-app/src/services/*.rs`), never duplicated separately in
  `http/*.rs` or `tauri_cmds/*.rs` handlers. This phase's entire backend mechanism already follows
  this pattern (see `request_service.create()`/`transition()` as the existing reference
  implementation) — no architectural deviation needed.
  `[VERIFIED: codebase read — crates/trackly-app/src/tauri_cmds/requests.rs, http/requests.rs]`
- **SQLite single-writer / reader-pool**: reads go through `ReaderPool::acquire()` inside
  `tokio::task::spawn_blocking`. `authorize()` checks (and ownership filters) must happen **before**
  `readers.acquire()` is called, so a forbidden caller never touches the connection pool at all.
  `[VERIFIED: codebase read — every *_service.rs read method follows spawn_blocking+acquire pattern]`
- **Session middleware gates `/api/*` except `/auth_login`** — already in place; this phase adds
  *authorization* (role check) on top of already-enforced *authentication* (session presence).
  `[CITED: CLAUDE.md Critical Architectural Notes]`
- No `dirs` crate, no APPDATA paths — not relevant to this phase (no new file/path handling).
- Russian-only UI strings — all new UI copy (employee shell, "Нет доступа" screen) must be Russian,
  matching existing `ROLE_LABELS`-style conventions in `Sidebar.svelte`.
- CI gates: `cargo clippy -D warnings`, `cargo fmt`, `cargo test --workspace --no-fail-fast --
  --test-threads=1` (already single-threaded — consistent with the project's "one `cargo test` at a
  time" rule; no special sequencing needed in CI, but local dev must not run two `cargo test`
  invocations concurrently). `[VERIFIED: .github/workflows/ci-fast.yml:71-72, ci-full.yml:77-78]`

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-UI-01 — Отдельная упрощённая employee-оболочка:** Employee получает СВОЙ минимальный layout, а не
отфильтрованный общий. Стартовая страница — «Мои заявки»; доступные действия — «Новая заявка»,
профиль/выход. Без полного sidebar и без навигации на остальные разделы. Discretion планировщику: тонкая
ветка над существующим `Layout.svelte` vs полностью отдельный shell-компонент — на усмотрение, в рамках
UI-SPEC паттернов. Главное требование: сотрудник физически не видит и не может навигировать на чужие
разделы из своей оболочки.

**D-GATE-01 — закрыть от employee чтение всего, кроме Заявок и (упрощённого) Дашборда:** Employee
сохраняет ТОЛЬКО: `ReadRequests`, `CreateRequest`, упрощённый дашборд (D-GATE-03). Закрываются для
employee: чтение devices, acts, cartridges, printers, reports, users.

**D-GATE-02 — механизм: authorize() в read-сервисах + изменение матрицы:** Источник истины —
`authorize(caller, &Action::…)` в сервис-слое (переиспользуем D-RBAC-01 Phase 5). Сейчас read-методы
(`device_service.get/list/search/list_grouped`, аналогично act/cartridge/printer/report) не вызывают
`authorize()` — добавить явные read-гейты. Это требует протянуть `caller: &Identity` в те read-методы,
где его сейчас нет в сигнатуре. Изменить матрицу `authorize()` в `crates/trackly-core/src/auth.rs`:
`ReadData` и `ReadPrinters` больше НЕ возвращают `true` для `Role::Employee` (Admin|Manager).
`ReadRequests` + `CreateRequest` остаются доступны employee. Discretion планировщику: точные сигнатуры
протяжки `caller`, гранулярность Action-ов (использовать существующие `ReadData`/`ReadPrinters`/
`ReadRequests` vs добавить более узкие) — на усмотрение, лишь бы каждый закрытый read-эндпоинт реально
проверял роль.

**D-GATE-03 — employee-дашборд только request-scoped данные сотрудника:** Для роли employee дашборд
отдаёт ТОЛЬКО данные его собственных заявок (счётчики по статусам, последние заявки), БЕЗ org-wide
метрик устройств/картриджей/принтеров. Причина: `dashboard_service` сейчас агрегирует org-wide данные —
если employee увидит их, это утечка тех самых read-данных, которые гейтим.

**D-REQ-01 — employee видит только свои заявки:** Фильтр по `requested_by_user_id == caller.user_id`
для роли Employee в `request_service.list` (и соответствующих status_counts / get_history). Реализует
существующий комментарий в `auth.rs` («сотрудник видит только свои»), который пока не реализован в
`list()`. Manager/Admin — без этого фильтра; существующее скрытие `ad_register` для не-admin
сохраняется.

**D-DENY-01 — экран «Нет доступа» (403) + обработка 403 в client.ts:** Прямой переход employee на
запрещённый роут (по URL) → route-guard показывает страницу-заглушку «Нет доступа» с пояснением и
кнопкой «К Заявкам». 403 от API обрабатывается в `ui/src/lib/api/client.ts` рядом с существующим
401-редиректом на логин (сейчас обрабатывается только 401). UI-gating (скрытие навигации) остаётся
UX-слоем; источник истины безопасности — backend `authorize()` (переиспользуем D-RBAC-03 Phase 5: на
UI-скрытие не полагаемся для защиты).

**D-TEST-01 — расширить CI-матрицу role × endpoint на read-эндпоинты:** Расширить существующую
тест-матрицу Phase 5 (мутации → 403) на READ-эндпоинты: Employee → 403 на чтение devices / acts /
cartridges / printers / reports / users; Employee → 200 на свои заявки; Employee не видит чужие заявки
(D-REQ-01); Employee → корректный упрощённый дашборд (D-GATE-03). Manager/Admin — read разрешён.

### Claude's Discretion

- Точная вёрстка/копирайт employee-оболочки и экрана «Нет доступа» — в рамках UI-SPEC паттернов.
- Тонкая ветка над `Layout.svelte` vs отдельный shell-компонент для employee (D-UI-01).
- Точный состав упрощённого employee-дашборда в пределах request-scoped данных юзера (D-GATE-03).
- Гранулярность `Action`-ов и сигнатуры протяжки `caller: &Identity` в read-методы (D-GATE-02).
- Нужна ли миграция БД — скорее всего НЕТ (всё на уровне authorize + сервисных фильтров + UI);
  подтвердить при планировании. **Research confirms: NO migration needed** (see Open Questions §7
  resolution below).

### Deferred Ideas (OUT OF SCOPE)

- Org-метрики/виджеты в employee-дашборде — намеренно не делаем (request-scoped only).
- Гибкая настройка «manager видит свои/все заявки» — вне scope; manager как сейчас.
- Более узкие/гранулярные `Action`-ы для каждого ресурса (если решат не использовать существующие
  `ReadData`/`ReadPrinters`) — на усмотрение планировщика, не обязательно.

None из обсуждения не ушло за пределы scope фазы.
</user_constraints>

<phase_requirements>
## Phase Requirements

This phase has no new REQUIREMENTS.md IDs — it closes enforcement gaps in already-shipped
requirements.

| ID | Description | Research Support |
|----|-------------|------------------|
| USR-02 | Три роли (admin/manager/employee); «Сотрудник — только создание заявок» | Backend read-gating (D-GATE-01/02) makes this UI-documented constraint an actually-enforced one; see Architecture Patterns §authorize() Matrix and Don't Hand-Roll table. |
| USR-06 | Enforcement роли нельзя обойти через API | Directly addressed by adding `authorize()` to every read service method (see Common Pitfalls §1 and Code Examples). |
| REQ-06 | Заявки (requests) feature | D-REQ-01's own-requests filter and D-GATE-03's employee dashboard build directly on `request_service`'s existing `caller`-threading pattern. |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Role-based read authorization | API/Backend (service layer) | — | `authorize()` must be the single source of truth; this is exactly the architectural principle D-RBAC-01 (Phase 5) already established and this phase extends to reads. |
| Own-requests filtering (D-REQ-01) | API/Backend (service layer) | Database (SQL `WHERE` clause, already exists) | Filtering must happen in `request_service`/`RequestRepository` SQL, not as a client-supplied parameter — an employee-supplied `requestedByUserId` must be ignored/overridden server-side for `Role::Employee`. |
| Employee dashboard scoping (D-GATE-03) | API/Backend (new service method or branch) | — | Same reasoning: aggregation must happen server-side on the caller's own data; a client-side filter over the org-wide payload would mean the org-wide data was already sent to the browser. |
| Employee shell rendering (D-UI-01) | Frontend (Svelte SPA) | — | Browser tier owns navigation/UI structure. This is explicitly UX-only per D-RBAC-03 — backend `authorize()` remains the real boundary regardless of what the shell renders. |
| Route-guard / 403 screen (D-DENY-01) | Frontend (Svelte SPA, `svelte-spa-router` route component + `client.ts`) | — | Browser-tier concern; must not be treated as a security control — it is UX that reacts to a backend that is the actual enforcement point. |
| CI role×endpoint matrix (D-TEST-01) | Test infrastructure (`trackly-app/tests/`) | — | Integration tests exercise the real axum `Router` end-to-end (no mocking of `authorize()`), verifying the backend tier's actual behavior — not the frontend's UX layer. |

## Standard Stack

No new external libraries are needed for this phase. All work uses already-installed dependencies.

### Core (already in use, no version change)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `axum` | 0.8.x | HTTP transport for LAN read endpoints | Already the project's HTTP framework (CLAUDE.md-fixed); no change needed for this phase. `[VERIFIED: codebase — Cargo.toml / http/*.rs]` |
| `tauri` | 2.x | Desktop transport for read commands | Already the project's desktop framework; no change needed. `[VERIFIED: codebase — tauri_cmds/*.rs]` |
| `svelte-spa-router` | ^5.1.0 | Frontend router, used for D-DENY-01's route guard | Already the actual router in use (confirmed in `App.svelte` import and `ui/package.json`), despite CLAUDE.md's "Alternatives Considered" table listing `svelte-routing` as one option — the codebase has already settled on `svelte-spa-router`. `[VERIFIED: ui/package.json:27, ui/src/App.svelte:3]` |
| `tower-sessions` | 0.13+ | Session identity resolution (`session_identity()`) | Already wired; this phase stops discarding its return value in read handlers, no new integration needed. `[VERIFIED: crates/trackly-app/src/http/auth.rs]` |

### Supporting
None needed — this phase is pure application-logic plumbing (threading an existing `Identity` type
through existing service methods, and existing SQL filter columns), not new infrastructure.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Reusing `Action::ReadData`/`ReadPrinters`/`ReadRequests` | Adding narrower `Action::ReadActs`, `Action::ReadCartridges`, `Action::ReadReports`, `Action::ReadUsers` per-resource | CONTEXT.md explicitly defers this to planner discretion. Narrower actions cost more enum variants + more matrix rows but give finer audit/future-flexibility (e.g., if a future role needs read-acts-but-not-read-cartridges). Given the locked scope is binary (employee gets Requests+Dashboard, nothing else), reusing the existing catch-all `ReadData` for "everything an employee can't read" is lower-effort and sufficient for this phase's actual requirement. Recommend: reuse existing Actions; only introduce new ones if the planner identifies a near-term need for per-resource differentiation among Admin/Manager (there is none in current CONTEXT.md scope). |
| Server-side own-requests filter (always applied for `Role::Employee`, ignoring client `requestedByUserId`) | Trusting the client-supplied `requestedByUserId` field already in `RequestFilter` | The frontend (`RequestsPage.svelte`) already sends `requestedByUserId: identity.id` for employees as a UI nicety — but this is attacker-controlled input from the employee's own browser session. Must NOT be trusted as the enforcement mechanism; the service must override/ignore the client value and always inject `caller.user_id` server-side when `caller.role == Employee`, regardless of what the request body contains. |

**Installation:** No new packages — nothing to install.

**Version verification:** Not applicable — no new package versions to verify. Existing versions
confirmed via direct `Cargo.toml`/`package.json` reads, not registry lookups, since no new
dependencies are introduced.

## Package Legitimacy Audit

**Not applicable for this phase** — no new external packages are introduced. This phase modifies
existing Rust service-layer code and existing Svelte components using only already-installed
dependencies (`axum`, `tauri`, `svelte-spa-router`, `tower-sessions`, etc.). The Package Legitimacy
Gate protocol is skipped per its own scope condition ("whenever this phase installs external
packages" — it does not).

## Architecture Patterns

### System Architecture Diagram

```
Browser (LAN employee)              Tauri Desktop (employee, unlocked)
        │                                       │
        │ fetch('/api/v1/requests_list', …)     │ invoke('requests_list', …)
        ▼                                       ▼
┌───────────────────┐                ┌────────────────────────┐
│ axum http/requests │                │ tauri_cmds/requests.rs │
│  .rs handler        │                │  thin wrapper          │
│  session_identity() │                │  resolve_tauri_identity│
│  → caller: Identity │                │  → caller: Identity    │
└─────────┬──────────┘                └───────────┬────────────┘
          │                                        │
          └───────────────┬────────────────────────┘
                           ▼
                 build_requests_list(ctx, caller, filter, page)
                           │
                           ▼
              ┌─────────────────────────────┐
              │  RequestService::list()      │
              │  1. (NEW) if caller.role ==   │
              │     Employee: force filter.   │
              │     requested_by_user_id =    │
              │     Some(caller.user_id)      │
              │  2. exclude_ad_register =     │
              │     !matches!(caller.role,    │
              │     Role::Admin)  (existing)  │
              │  3. spawn_blocking {           │
              │       readers.acquire()        │
              │       repo.list(...)           │
              │     }                          │
              └─────────────┬───────────────┘
                             ▼
              RequestRepository::list() SQL
              "... WHERE (?4 IS NULL OR
                r.requested_by_user_id = ?4) ..."
              (?4 already wired — just feed
               caller.user_id instead of
               trusting client filter)
                             │
                             ▼
                      SQLite (WAL, reader pool)


Contrast — currently-ungated read path (device/act/cartridge/printer/report):

Browser/Desktop → http/devices.rs::handler_list
                     let _identity = session_identity(&session)…   ◄── COMPUTED, DISCARDED
                  → build_devices_list(ctx, filter, page)            ◄── no caller param at all
                  → DeviceService::list(filter, page)                ◄── no authorize() call
                  → spawn_blocking { readers.acquire(); repo.list() } ◄── reached unconditionally,
                                                                          regardless of caller role
```

### Recommended Project Structure

No new directories needed. Changes land in existing files:

```
crates/trackly-core/src/
└── auth.rs                  # Narrow Action::ReadData matrix arm to Admin|Manager

crates/trackly-app/src/services/
├── device_service.rs        # Thread caller: &Identity into get/list/search/list_grouped/
│                             #   list_by_ids/status_counts/locations_autocomplete/
│                             #   autocomplete/export_csv; call authorize(caller, &Action::ReadData)
├── act_service.rs            # Same pattern: get/search/list/counts/peek_next_number/suggest_person
├── cartridge_service.rs      # Same pattern: get/list/status_counts/search/get_history/low_stock/
│                             #   model_list/model_get/suggest_* methods
├── printer_service.rs        # Same pattern: list/get/current_cartridge_for_printer
│                             #   (mutations already call authorize(MutatePrinters) — no change there)
├── report_service.rs          # Same pattern: list_device_*/list_cartridge_*/get_report_counts/
│                             #   export_csv/export_pdf
├── request_service.rs         # D-REQ-01: force requested_by_user_id filter for Employee in list();
│                             #   thread caller into counts() and get_history(); add ownership check
│                             #   in get_history() (fetch request, verify requested_by_user_id ==
│                             #   caller.user_id for Employee, else AppError::Forbidden/NotFound)
└── dashboard_service.rs       # D-GATE-03: new method (e.g. get_employee_widgets(caller)) or branch
                              #   inside get_all_widgets() returning request-scoped-only data for
                              #   Role::Employee

crates/trackly-app/src/http/*.rs        # Stop discarding _identity; pass caller through to service
crates/trackly-app/src/tauri_cmds/*.rs  # Call resolve_tauri_identity() where currently absent;
                                         # pass caller through to service

ui/src/features/layout/
├── EmployeeLayout.svelte     # NEW — D-UI-01 separate minimal shell (or thin branch in Layout.svelte
│                             #   — planner's discretion)
└── AccessDenied.svelte        # NEW — D-DENY-01 "Нет доступа" screen with "К Заявкам" button

ui/src/App.svelte              # Branch shell selection on authStore.user.role
ui/src/routes.ts               # Possibly a reduced employee route map, or route-guard wrapper
ui/src/lib/api/client.ts        # Add 403 handling next to existing 401 handling

crates/trackly-app/tests/
└── role_endpoint_matrix.rs    # Extend with read-endpoint cases; flip existing Case 9 to 403
```

### Pattern 1: authorize()-before-spawn_blocking (the gating pattern to replicate)

**What:** Every gated method calls `authorize(caller, &Action::X)?` synchronously, before cloning
`Arc` handles into the `spawn_blocking` closure. If `authorize` returns `Err(AppError::Forbidden)`,
the method returns immediately via `?` — the reader pool is never touched, so no connection is
wasted on an already-rejected caller.

**When to use:** Every read method in device/act/cartridge/printer/report services, and the
counts/get_history methods in request_service.

**Example (already-correct reference, from `request_service.rs`'s `create`/`transition`, applied to
the read case being added):**
```rust
// Source: crates/trackly-app/src/services/printer_service.rs (existing mutation pattern,
// generalized here to the read case this phase adds)
pub async fn list(
    &self,
    filter: DeviceFilter,
    page: Pagination,
    caller: &Identity,                      // NEW param
) -> Result<DeviceListResponse, AppError> {
    authorize(caller, &Action::ReadData)?;  // NEW — gate before touching the pool
    let readers = self.readers.clone();
    let repo = self.device_repo.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        let (rows, total) = repo.list(&conn, &filter, &page)?;
        let items = rows.into_iter().map(DeviceDto::from).collect();
        Ok(DeviceListResponse { items, total: total as i64 })
    })
    .await
    .map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking: {e}") })?
}
```

### Pattern 2: Server-side ownership override (D-REQ-01)

**What:** Never trust a client-supplied "whose data do I want" filter field for a restricted role.
Force the filter value server-side, ignoring/overwriting whatever the client sent.

**When to use:** `request_service.list()`'s `requested_by_user_id` field, which the frontend
(`RequestsPage.svelte`) already populates client-side — that must become advisory-only once
server-side enforcement lands.

```rust
// Source: crates/trackly-app/src/services/request_service.rs (existing list(), modified)
pub async fn list(
    &self,
    mut filter: RequestFilter,                 // mut — server may override fields
    page: Pagination,
    caller: &Identity,
) -> Result<RequestListResponse, AppError> {
    authorize(caller, &Action::ReadRequests)?;  // ReadRequests stays true for Employee — no
                                                  // matrix change needed for this action.
    // D-REQ-01: Employee can ONLY ever see their own requests — override whatever the
    // client sent in filter.requested_by_user_id, do not merely default it.
    if matches!(caller.role, trackly_core::auth::Role::Employee) {
        filter.requested_by_user_id = caller.user_id;
    }
    let exclude_ad_register = !matches!(caller.role, trackly_core::auth::Role::Admin);
    // ... unchanged spawn_blocking body, repo.list() already accepts
    //     filter.requested_by_user_id via the existing parameterized SQL.
}
```

Note: `RequestFilter` (the domain-layer one in `trackly_core::domain::requests`) carries
`requested_by_user_id: Option<i64>`; `Identity.user_id` is `Option<i64>` (None = trusted desktop
admin). For Employee role, `caller.user_id` is always `Some(_)` in practice (employees only exist as
real DB users with AD or local login — `Identity::trusted_admin()` is Admin role, never Employee), but
the planner should decide explicitly whether to treat a theoretical `Employee` with `user_id: None` as
an error condition (defensive `unwrap_or` / early `Forbidden`) rather than silently passing `None`
through (which would defeat the filter, returning unfiltered results).

### Pattern 3: Ownership check for single-resource reads (get_history gap)

**What:** List-level filtering is not enough when a sibling "get a single item by ID" method exists.
`request_service.get_history(request_id)` has no caller parameter at all — an employee who knows or
guesses another user's `request_id` can read that request's full audit history today, and will still
be able to after D-REQ-01's list-level filter ships, unless this method is independently patched.

**When to use:** `request_service.get_history()`, and arguably `request_service.get()` (used by
`RequestDetail.svelte` when a request is selected) — both take only an ID, no caller.

```rust
// Source: crates/trackly-app/src/services/request_service.rs (get_history, modified)
pub async fn get_history(
    &self,
    request_id: i64,
    caller: &Identity,                          // NEW param
) -> Result<Vec<RequestHistoryEntryDto>, AppError> {
    authorize(caller, &Action::ReadRequests)?;
    if matches!(caller.role, trackly_core::auth::Role::Employee) {
        // Fetch the request first to check ownership — reuse self.get() or a lighter
        // "owner_id of request X" repo query to avoid double-fetching the full DTO.
        let owner_id = self.request_repo_owner_id(request_id).await?;
        if Some(owner_id) != caller.user_id {
            return Err(AppError::Forbidden);
        }
    }
    // ... unchanged body
}
```

The same reasoning applies to `request_service.get()` if it is reachable by employees for arbitrary
IDs (it is — `requests_get` Tauri command and the corresponding HTTP handler take only `id`, no
caller, today). The planner should decide whether `get()` needs the identical ownership check, or
whether D-REQ-01's intent ("Заявки только свои") implies it should — recommend yes, for consistency
with `get_history()` and to avoid leaving a parallel single-resource leak.

### Anti-Patterns to Avoid

- **Filtering only in the UI:** `RequestsPage.svelte` already does this for `requestedByUserId` —
  it is explicitly called out in D-RBAC-03 (Phase 5) as **not a security boundary**. Do not consider
  the frontend filter "done" work for D-REQ-01; the backend filter is the actual deliverable.
- **Authorizing in the HTTP/Tauri handler instead of the service:** Both transports must reach
  identical behavior. Putting `authorize()` calls in `http/devices.rs::handler_list` but not in
  `tauri_cmds/devices.rs::devices_list` (or vice versa) re-creates exactly the kind of
  transport-asymmetric bug this phase is meant to eliminate. Put the check in `DeviceService::list()`
  itself, once, and let both thin wrappers inherit it for free.
  `[CITED: CLAUDE.md Critical Architectural Notes — "Dual access path must share business logic"]`
- **Trusting client-supplied scoping parameters:** see Pattern 2 above — `requested_by_user_id`
  arriving from the request body must be overridden, not merely validated, for `Role::Employee`.
- **Building the employee dashboard by filtering the existing org-wide DTO in the frontend:** This
  would require the backend to send the org-wide aggregate to the employee's browser in the first
  place, then hide parts of it client-side — re-creating the exact "leak via dashboard" failure mode
  D-GATE-03 is written to prevent. The new employee-scoped data must never leave the server in
  unfiltered form.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Role-permission matrix | A new ad-hoc per-route permission check sprinkled across `http/*.rs` and `tauri_cmds/*.rs` | The existing `authorize(identity, action)` function in `trackly-core::auth` | Single source of truth already exists and is unit-testable in isolation (no I/O deps — `trackly-core` is gated by a `no_io_deps` compile check). Adding parallel ad-hoc checks in transport handlers defeats this and reintroduces the dual-transport asymmetry bug this phase fixes. |
| "Is this caller allowed to see this specific row" checks | Bespoke per-service ownership-comparison helper functions, one per service | A small, consistent pattern: `if matches!(caller.role, Role::Employee) { check/force ownership }` inlined at the top of each affected method, mirroring the existing `exclude_ad_register` pattern already in `request_service.list()` | The codebase already has exactly one such precedent (`exclude_ad_register`); replicating its shape (a `let` binding derived from `caller.role`, used to alter the query/filter) keeps the diff small and reviewable, rather than introducing a new abstraction layer for a single phase's needs. |
| 403 detection on the frontend | Custom per-page try/catch blocks checking `res.status === 403` in every Svelte component that calls `apiCall()` | A single check inside `client.ts`'s `apiCall()`, symmetric with the existing 401 check | `client.ts` already centralizes 401 handling exactly once; 403 handling belongs in the same chokepoint so every caller of `apiCall()` gets consistent behavior without per-page boilerplate. |
| Route-level access control | A custom router replacement or manual `window.location.hash` parsing in `App.svelte` | `svelte-spa-router`'s existing route map (`routes.ts`) plus a wrapper/guard component, or a reduced employee-specific route map swapped in based on role | The project has already standardized on `svelte-spa-router`; introducing a second routing mechanism for one role would fragment navigation logic across two systems. |

**Key insight:** This phase's "Don't Hand-Roll" list is short because almost nothing new needs
inventing — the dominant risk is *not adding the missing `authorize()` calls consistently across both
transports for every read method*, not architectural novelty. The existing patterns
(`authorize()`, `caller: &Identity` threading, `exclude_ad_register`-style role branching, the 401
chokepoint in `client.ts`) already cover every shape of problem this phase needs to solve; the work is
mechanical replication of an established pattern across ~25 currently-ungated methods, not new design.

## Common Pitfalls

### Pitfall 1: Treating "ReadData/ReadPrinters returns true for Employee" as the bug, when ReadPrinters is already correct

**What goes wrong:** CONTEXT.md's phrasing ("`ReadData`/`ReadPrinters` больше НЕ возвращают `true`
для Employee") suggests both Actions need a matrix change. Direct code reading
(`crates/trackly-core/src/auth.rs`) shows `Action::ReadPrinters` is **already** bundled with the
`Mutate*` arm, restricted to `Admin | Manager` — it is NOT in the `true`-for-everyone arm today.
Only `Action::ReadData` is the actual universal-`true` catch-all.
**Why it happens:** The CONTEXT.md framing was written before someone re-read the current matrix
code; the bug as experienced (employee CAN read printers via the API) is real, but its root cause for
printers specifically is "no service method calls `authorize(ReadPrinters)` at all" — not "the matrix
says `true`."
**How to avoid:** When implementing D-GATE-02, confirm with `cargo test` / a quick manual check that
the matrix change for `ReadData` is the only matrix-level edit needed; the `ReadPrinters` fix is
entirely about adding the missing `authorize(caller, &Action::ReadPrinters)` calls into
`printer_service.list()`/`get()`/`current_cartridge_for_printer()`, not about touching the matrix arm
that already excludes Employee.
**Warning signs:** If a plan task says "change `ReadPrinters` matrix entry," that task is based on a
stale premise and should be corrected to "add missing `authorize(ReadPrinters)` calls in
`printer_service`."

### Pitfall 2: Fixing `list()` but leaving sibling single-resource `get()`/`get_history()` methods open

**What goes wrong:** D-REQ-01 explicitly calls out `list`/`status_counts`/`get_history`, but
`request_service.get(id)` (used when a row is selected in the master-detail UI) is structurally
identical to `get_history` — it takes only an `id`, no `caller`, no ownership check. An employee can
list only their own requests (after the fix) but might still be able to `GET` an arbitrary other
request's full detail by ID via direct API call, if `get()` isn't patched too.
**Why it happens:** It's easy to fix the method named explicitly in CONTEXT.md and miss its sibling
because list-level filtering "feels like" it solves the access problem.
**How to avoid:** Audit every method on `request_service` that accepts a bare `id: i64` (`get`,
`get_history`) and add the same ownership check pattern to both, not just the ones named in CONTEXT.md.
**Warning signs:** A CI test that exercises `requests_get_history` with an employee session and a
foreign `request_id` returning 200 instead of 403/404.

### Pitfall 3: Flipping the matrix/adding authorize() calls without updating the existing CI test that currently expects the OLD (insecure) behavior

**What goes wrong:** `crates/trackly-app/tests/role_endpoint_matrix.rs` Case 9 currently asserts
`Employee session → POST /api/v1/devices_list → 200 OK ("reads allowed")`. Once D-GATE-02 ships, this
assertion is wrong and the test will fail (correctly, but confusingly if not anticipated) — or worse,
if the planner doesn't touch this test at all, CI will catch the regression but the task sequencing
might not have anticipated needing to *edit* an existing passing test as part of the read-gating work.
**Why it happens:** It's easy to think of "extend the test matrix" as purely additive (D-TEST-01's
phrasing) and not notice that one existing case's assertion is the literal behavior being changed.
**How to avoid:** Explicitly include "flip Case 9's expected status from 200 to 403" as a plan task,
not just "add new read-endpoint cases."
**Warning signs:** `cargo test` failing on `role_endpoint_matrix_test` with `Case 9` in the panic
message after the backend changes land but before the test file is updated.

### Pitfall 4: Building the employee dashboard as a role-conditional branch inside the existing aggregate SQL query, rather than a genuinely separate query path

**What goes wrong:** `dashboard_service.get_all_widgets()` runs ONE `spawn_blocking` closure that
computes device/cartridge/request/printer widgets together from several repo calls. A tempting
shortcut is to keep this single method, compute everything as today, and just *not render* the
non-request fields in the employee version of `DashboardPage.svelte`. This re-creates the exact
"computed-but-hidden" leak pattern called out in Architecture Patterns' Anti-Patterns section — the
org-wide data still crosses the network to the employee's browser/webview, retrievable via browser
devtools or a modified frontend build.
**Why it happens:** Reusing one query path is less code than writing a second, narrower one.
**How to avoid:** Add a distinct method (or an early role-branch *inside* `get_all_widgets()` that
returns before reaching the org-wide repo calls) so the SQL/data computed for an `Employee` caller
genuinely never includes device/cartridge/printer aggregates — verify by reading the response JSON in
a test, not just the rendered UI.
**Warning signs:** Network tab / HTTP response body for `dashboard_get_all_widgets` called with an
employee session still contains `devices_total`, `cartridge_by_status`, etc.

### Pitfall 5: Forgetting the Tauri side has TWO different gaps — missing `authorize()` calls AND missing `resolve_tauri_identity()` calls entirely

**What goes wrong:** On HTTP, every read handler already calls `session_identity()` — it's just
discarded (`let _identity = …`). On Tauri, several read commands (`requests_get`, `requests_counts`,
`requests_get_history`, `requests_list_categories`, and the equivalent for devices/acts/cartridges/
printers/reports) don't call `resolve_tauri_identity()` **at all**. A plan task phrased only as "stop
discarding `_identity`" will silently fail to fix the Tauri side, because there's no identity variable
to "stop discarding" there — a new call to `resolve_tauri_identity()` must be added first.
**Why it happens:** The two transports' current code shape looks similar (`_identity` vs missing
entirely) but requires textually different fixes.
**How to avoid:** Treat "thread caller through HTTP handler" and "thread caller through Tauri command"
as two distinct sub-steps per endpoint, not one shared fix.
**Warning signs:** `cargo build` succeeding (Tauri commands without `caller` still compile fine — they
just don't pass one to the service) while the Tauri desktop app's behavior for an employee differs
from the LAN browser's behavior for the same role, defeating the dual-transport parity goal.

## Code Examples

### Current authorize() matrix (the file D-GATE-02 edits)
```rust
// Source: crates/trackly-core/src/auth.rs (current state, confirmed by direct read)
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
        | Action::ReadPrinters => {                       // already Admin|Manager only
            matches!(identity.role, Role::Admin | Role::Manager)
        }
        Action::ReadData | Action::CreateRequest | Action::ReadRequests => true,
        //         ^^^^^^^^ this is the ONE arm that needs narrowing
    };
    if allowed { Ok(()) } else { Err(AppError::Forbidden) }
}
```

Recommended post-Phase-10 shape (illustrative — exact Action grouping is planner's discretion):
```rust
Action::MutateDevices
| Action::MutateActs
| Action::MutateCartridges
| Action::MutatePrinters
| Action::TransitionRequests
| Action::ReadPrinters
| Action::ReadData => {                  // ReadData moved into this arm
    matches!(identity.role, Role::Admin | Role::Manager)
}
Action::CreateRequest | Action::ReadRequests => true,   // unchanged — Employee keeps these
```

### Existing reference pattern for caller-threaded read method (request_service.list)
```rust
// Source: crates/trackly-app/src/services/request_service.rs lines 84-105 (current, unmodified)
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
This is the template to replicate for `device_service.list()`, `act_service.list()`, etc. — add
`caller: &Identity` param, add `authorize(caller, &Action::ReadData)?` as the first line of the
method body (before any `.clone()` calls), keep everything else unchanged.

### Existing CI test scaffolding (role_endpoint_matrix.rs) — programmatic session + fresh router per case
```rust
// Source: crates/trackly-app/tests/role_endpoint_matrix.rs (current, confirmed by direct read)
macro_rules! new_app {
    () => {{
        let ss = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        build_router(&ctx, ss)
    }};
}

async fn post_with_cookie(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
    cookie: Option<&str>,
) -> StatusCode {
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(c) = cookie {
        builder = builder.header("cookie", c);
    }
    let req = builder.body(Body::from(serde_json::to_string(&body).unwrap())).unwrap();
    app.oneshot(req).await.unwrap().status()
}

// Existing Case 9 — MUST be flipped from StatusCode::OK to StatusCode::FORBIDDEN post-Phase-10:
{
    let status = post_with_cookie(
        new_app!(),
        "/api/v1/devices_list",
        device_list_payload.clone(),
        Some(&employee_cookie),
    ).await;
    assert_eq!(status, StatusCode::FORBIDDEN, /* was StatusCode::OK */
        "Case 9 (post-Phase-10): Employee → devices_list (read) → expected 403, got {status}");
}
```
Sessions are created directly via `RusqliteSessionStore::create()` (bypassing `/auth_login`'s
`GovernorLayer`, which needs a real TCP peer IP unavailable in unit tests) — this exact mechanism
extends unchanged to new read-endpoint test cases; no new test infrastructure is needed, only more
cases following the same shape.

### Frontend: existing 401 handling in client.ts (the chokepoint D-DENY-01 extends)
```typescript
// Source: ui/src/lib/api/client.ts (current, confirmed by direct read)
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
```
D-DENY-01 needs a symmetric `if (res.status === 403) { /* navigate to #/access-denied or similar */ }`
branch here, alongside the Tauri-path equivalent (which currently only checks
`code === 'UNAUTHORIZED'` and has no 403/`'FORBIDDEN'` branch at all — confirm exact Tauri error code
string for forbidden errors before wiring this, likely `'FORBIDDEN'` or `'Forbidden'` mirroring the
existing `'UNAUTHORIZED'`/`'Unauthorized'` dual-check pattern already present for the 401 case).

### Frontend: existing (UI-only) employee request filter — becomes redundant-but-harmless after backend fix
```typescript
// Source: ui/src/features/requests/RequestsPage.svelte lines 27-35 (current, confirmed by direct read)
// Role-based filter: employees see only their own requests (D-RBAC-02).
// Backend enforces this — UI passes requestedByUserId for employee role.
const baseFilter = $derived<RequestFilter>({
  status: null,
  requestType: null,
  assignedToUserId: null,
  requestedByUserId:
    identity?.role === 'employee' ? (identity?.id ?? null) : null,
});
```
The comment "Backend enforces this" is currently **aspirational, not true** — this is exactly the gap
D-REQ-01 closes. No frontend change is strictly required here (the existing code already sends the
right filter value), but the planner should note this UI code becomes correct-by-coincidence once the
backend enforces independently, rather than being the actual enforcement mechanism it currently reads
as.

## State of the Art

| Old Approach (current code) | New Approach (this phase) | When Changed | Impact |
|--------------------------|---------------------------|---------------|--------|
| Read handlers call `session_identity()`/no-op and discard the result | Read handlers/commands resolve `caller: &Identity` and pass it into the service, which calls `authorize()` | This phase | Closes the "employee can read everything via direct API call" gap (USR-06). |
| `Action::ReadData` matrix arm returns `true` unconditionally | `Action::ReadData` restricted to `Admin \| Manager` | This phase | Employee loses blanket read access to devices/acts/cartridges/reports data via this Action. |
| `request_service.list()` accepts client-supplied `requested_by_user_id` filter at face value | Service forces `requested_by_user_id = caller.user_id` for `Role::Employee`, ignoring client input | This phase | Closes the gap where an employee could supply `requestedByUserId: null` or another user's ID directly via API and see others' requests, bypassing the existing frontend-only filter. |
| One shared `<Layout>` shell for every authenticated role | Role-conditional shell selection in `App.svelte` (employee gets a separate/branched minimal shell) | This phase | Employee navigation surface area shrinks to Requests + Dashboard + profile/logout only. |
| `client.ts` handles only 401 specially | `client.ts` handles 401 (redirect to login) and 403 (this phase adds — likely redirect to an access-denied screen or simply ensure the thrown error is catchable by route guards) | This phase | Consistent UX for "authenticated but forbidden" across both transports. |

**Deprecated/outdated:** None — this phase does not deprecate any existing pattern; it completes a
pattern (`authorize()` + `caller` threading) that was already established in Phase 5 for mutations but
never extended to reads.

## Runtime State Inventory

Not applicable — this is not a rename/refactor/migration phase. No stored data, live service config,
OS-registered state, secrets, or build artifacts carry names/identifiers that this phase changes. The
phase only adds authorization checks and a new UI shell; it does not rename, move, or restructure any
existing identifiers, tables, or external integrations.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `Identity.user_id` is always `Some(_)` for any real `Role::Employee` caller in practice (no legitimate "trusted desktop admin with Employee role" exists) | Architecture Patterns, Pattern 2 | If wrong, a defensive `unwrap_or`/early-return path is needed where currently assumed unnecessary; without it, a theoretical `None`-user_id Employee caller would see unfiltered results. Low likelihood given `Identity::trusted_admin()` is hardcoded to `Role::Admin`, but not directly proven by a type-level invariant — worth a planner-level confirmation or a defensive guard regardless. |
| A2 | Reusing existing `Action::ReadData`/`ReadPrinters`/`ReadRequests` (rather than adding narrower per-resource Actions) is sufficient for this phase's locked scope | Standard Stack, Alternatives Considered | If the planner or a future phase needs Admin/Manager to have differentiated read access to, say, reports-but-not-cartridges, the coarse `ReadData` grouping would need retrofitting later. CONTEXT.md explicitly defers this choice to planner discretion, so this is a recommendation, not a verified constraint. |
| A3 | `request_service.get()` (single-request fetch, not just `get_history()`) also needs the ownership check, even though CONTEXT.md's D-REQ-01 text names only `list`/`status_counts`/`get_history` | Common Pitfalls #2, Architecture Patterns Pattern 3 | If the planner takes CONTEXT.md's method list as exhaustive and skips `get()`, an employee could still fetch another user's single request detail by ID after this phase ships, leaving a narrower but real version of the same gap D-REQ-01 is meant to close. |

**If this table is empty:** N/A — see entries above. All other claims in this research were verified
directly by reading the actual source files referenced (service implementations, `auth.rs`, migration
files, CI workflow YAML, `client.ts`, Svelte components) rather than from training-data assumptions
about typical Rust/Tauri/Svelte RBAC patterns.

## Open Questions

1. **Should `request_service.get()` get the same ownership check as `get_history()`?**
   - What we know: CONTEXT.md's D-REQ-01 text names `list`/`status_counts`/`get_history` explicitly;
     `get()` is structurally identical (takes only `id`, no `caller`) and is reachable by the
     `RequestDetail.svelte` UI when an employee selects a row.
   - What's unclear: Whether the omission from CONTEXT.md's text is intentional (e.g., because the
     UI-level `list()` filter already prevents an employee from ever knowing a foreign `request_id`
     to select) or an oversight.
   - Recommendation: Treat it as in-scope for consistency and defense-in-depth — an employee could
     still guess/enumerate IDs via direct API calls even if the UI never surfaces a foreign ID. Cost
     of adding the same check to `get()` is small (mirrors `get_history()`'s pattern exactly).

2. **Exact Tauri-side error code string for `AppError::Forbidden`, for the frontend D-DENY-01 fix**
   - What we know: HTTP path returns 403 with a JSON body via `AppErrorResponse`; the Tauri path
     (`client.ts`'s `isTauri` branch) currently checks `err.code === 'UNAUTHORIZED' || 'Unauthorized'`
     for the 401-equivalent case, implying `AppError`'s Tauri-serialized shape has a `code` field.
   - What's unclear: This research did not trace `AppError`'s `Serialize`/specta-generated `code`
     field values for the `Forbidden` variant specifically (only confirmed the HTTP `StatusCode`
     mapping via `error_axum.rs`). The exact string (`'FORBIDDEN'` vs `'Forbidden'` vs something else)
     needs a quick grep of `trackly-core/src/error.rs` or the generated TS bindings
     (`ui/src/bindings*.ts`) before writing the `client.ts` 403 branch.
   - Recommendation: Planner should have an early task to grep `AppError::Forbidden`'s serialized
     `code` value (likely in `crates/trackly-core/src/error.rs` and/or generated specta bindings)
     before finalizing the `client.ts` 403-handling code.

3. **Where exactly should the employee 403/access-denied UI navigate to, mechanically?**
   - What we know: D-DENY-01 specifies a "Нет доступа" screen with a "К Заявкам" button, triggered by
     (a) direct URL navigation to a forbidden route, and (b) presumably also reachable if an API call
     returns 403 mid-session (e.g., stale UI state pointing at a now-restricted resource).
   - What's unclear: Whether the route-guard approach should be a wrapper component checked per-route
     in `routes.ts`, a `$effect` in `App.svelte` reacting to `location` changes, or `svelte-spa-router`
     hooks (`wrap()` / `conditionsFailed` event) — `svelte-spa-router` v5 supports a `conditionsFailed`
     event and route `wrap()` helper for exactly this kind of guard, but this research did not fetch
     `svelte-spa-router`'s official docs to confirm the v5 API surface for guards (no Context7 entry
     checked; this would be a good target for the planner or a follow-up doc lookup before
     implementation, since getting the guard mechanism right affects task structure).
   - Recommendation: Planner should do a focused doc check (`svelte-spa-router` README/wrap() API) for
     v5.1, since this research confirms the dependency is present but did not verify its guard API
     surface against current docs — flagged here as `[ASSUMED]`-adjacent rather than `[VERIFIED]`.

## Environment Availability

Not applicable — this phase has no new external tool/service/runtime dependencies. All work happens
within the existing Rust workspace and existing `pnpm`-managed frontend, both already confirmed present
and functional by the existing CI pipelines (`ci-fast.yml`, `ci-full.yml`) and by this research's own
direct reads of the codebase.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[tokio::test]` (via `cargo test`), integration-style test exercising the real `axum::Router` with `tower::ServiceExt::oneshot` |
| Config file | None — test behavior is hardcoded in `crates/trackly-app/tests/role_endpoint_matrix.rs`; CI invocation is `cargo test --workspace --no-fail-fast -- --test-threads=1` (`.github/workflows/ci-fast.yml:71-72`, `ci-full.yml:77-78`) |
| Quick run command | `cargo test --test role_endpoint_matrix` (single test binary, single-threaded by project convention — never run two `cargo test` invocations concurrently, per project memory note) |
| Full suite command | `cargo test --workspace --no-fail-fast -- --test-threads=1` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| D-GATE-01/02 (devices) | Employee → POST `/api/v1/devices_list` → 403 | integration | `cargo test --test role_endpoint_matrix` | ✅ existing file, ❌ new case (currently asserts 200 — must flip) |
| D-GATE-01/02 (acts) | Employee → POST `/api/v1/acts_list` → 403 | integration | `cargo test --test role_endpoint_matrix` | ❌ new case needed |
| D-GATE-01/02 (cartridges) | Employee → POST `/api/v1/cartridges_list` → 403 | integration | `cargo test --test role_endpoint_matrix` | ❌ new case needed |
| D-GATE-01/02 (printers) | Employee → POST `/api/v1/printers_list` → 403 | integration | `cargo test --test role_endpoint_matrix` | ❌ new case needed |
| D-GATE-01/02 (reports) | Employee → POST `/api/v1/reports_list_device_acts` (or representative report endpoint) → 403 | integration | `cargo test --test role_endpoint_matrix` | ❌ new case needed |
| D-GATE-01/02 (users) | Employee → POST `/api/v1/users_list` → 403 (already covered? — `users_create` is Case 6, but `users_list` read is unconfirmed; likely already gated via `authorize(ManageUsers)` in `auth.rs::list_users`, so may already pass — verify, don't assume new gating is needed) | integration | `cargo test --test role_endpoint_matrix` | ✅ likely already enforced (`list_users` already calls `authorize(caller, &Action::ManageUsers)` per direct code read), recommend adding a case anyway for explicit regression coverage |
| D-REQ-01 (own requests only) | Employee → POST `/api/v1/requests_list` with no filter → only own requests returned (assert response body, not just status) | integration | `cargo test --test role_endpoint_matrix` (extend to assert response JSON content, not just status code — existing matrix only asserts `StatusCode`) | ❌ new case needed; note this requires asserting on response body, a new pattern for this test file |
| D-REQ-01 (get_history ownership) | Employee → POST `/api/v1/requests_get_history` for another user's request_id → 403/404 | integration | `cargo test --test role_endpoint_matrix` | ❌ new case needed |
| D-GATE-03 (employee dashboard) | Employee → POST `/api/v1/dashboard_get_all_widgets` → response contains only request-derived fields, no devices/cartridges/printers fields (or those fields are zeroed/absent per chosen DTO shape) | integration | `cargo test --test role_endpoint_matrix` (or a new dedicated test file if the DTO shape changes enough to warrant separation) | ❌ new case needed |
| D-UI-01 (employee shell) | Employee sees only Requests-related navigation, no sidebar to other sections | manual / Svelte component test (no existing frontend test runner detected in this codebase) | manual-only — no `vitest`/`@testing-library/svelte` config found in `ui/` during this research | ❌ manual verification only — flag for human-verify checkpoint |
| D-DENY-01 (access denied screen) | Employee navigating directly to `#/devices` (or similar) sees "Нет доступа" screen with "К Заявкам" button | manual / no frontend test runner | manual-only | ❌ manual verification only — flag for human-verify checkpoint |

### Sampling Rate
- **Per task commit:** `cargo test --test role_endpoint_matrix` (fast, single integration test file,
  exercises the real router end-to-end)
- **Per wave merge:** `cargo test --workspace --no-fail-fast -- --test-threads=1` (full suite,
  matches CI exactly)
- **Phase gate:** Full suite green before `/gsd-verify-work`, plus manual verification of the two
  frontend-only behaviors (D-UI-01 shell visibility, D-DENY-01 access-denied screen) since no
  frontend test runner exists in this codebase to automate them.

### Wave 0 Gaps
- [ ] No new test file needed — extend existing `crates/trackly-app/tests/role_endpoint_matrix.rs` in
      place. Confirm during planning whether response-body assertions (needed for D-REQ-01's
      "only own requests returned" and D-GATE-03's "no org-wide fields present") require adding a JSON
      body parse step to the test file's helpers (currently `post_with_cookie` only returns
      `StatusCode`, discarding the body) — this is a real gap: the existing helper needs a body-aware
      variant, e.g. `post_with_cookie_json() -> (StatusCode, serde_json::Value)`, before D-REQ-01/
      D-GATE-03 cases can assert response content rather than just status.
- [ ] Frontend test runner: no `vitest`, `@testing-library/svelte`, or `playwright` config detected in
      `ui/` during this research. D-UI-01 and D-DENY-01 are frontend-only behaviors with no automated
      coverage path available — recommend the planner gate these behind `checkpoint:human-verify`
      tasks rather than inventing test infrastructure as a side effect of this phase (out of scope per
      CONTEXT.md's deferred-ideas framing, which does not mention frontend testing infra).

## Security Domain

`security_enforcement: true`, `security_asvs_level: 1` (confirmed via `.planning/config.json`) — this
section is required and is in fact the central subject of this entire phase.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V4 Access Control | **yes — primary focus of this phase** | Single `authorize(identity, action)` function as the sole access-control decision point (ASVS V4.1.1 "verify that the application enforces access control rules on a trusted service layer" — this phase moves enforcement from "absent" to "present at the service layer," matching this control directly). |
| V2 Authentication | yes (already satisfied, unchanged by this phase) | `tower-sessions` cookie-based sessions (HTTP) / desktop-lock + Tauri state (desktop) — out of scope for this phase's changes, but the precondition this phase's authorization checks build on top of. |
| V3 Session Management | yes (already satisfied, unchanged by this phase) | Session middleware already gates `/api/*` except `/auth_login`; this phase does not touch session lifecycle. |
| V1 Architecture/Design | yes | V1.4.x "verify that there is a centralized access-control mechanism" — directly the architectural fix this phase implements (single `authorize()` call site reused by all transports, per CLAUDE.md's "один DTO, два транспорта" mandate). |
| V5 Input Validation | partial | `requested_by_user_id` from client input must be treated as untrusted and overridden, not merely validated, for `Role::Employee` (see Architecture Patterns Pattern 2) — this is an access-control concern expressed as an input-handling discipline, not a new validation library need. |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Broken Object Level Authorization (BOLA) / Insecure Direct Object Reference — caller supplies an `id` for a resource they don't own (e.g., `request_id` in `get_history`) and the service returns it without an ownership check | Elevation of Privilege / Information Disclosure | Server-side ownership check comparing the resource's owner field to `caller.user_id` before returning data — exactly Pattern 3 in this document. This is the single most relevant OWASP API Security Top 10 category (API1:2023 Broken Object Level Authorization) for this phase's `get`/`get_history` gap. |
| Missing Function-Level Authorization — an endpoint exists and is reachable by an authenticated-but-unauthorized role because no check is present at all | Elevation of Privilege | Centralized `authorize()` call at the top of every service method, verified by the CI role×endpoint matrix (D-TEST-01) — this is the API5:2023 Broken Function Level Authorization pattern, and is the literal subject of D-GATE-01/02. |
| Mass data exposure via an aggregate/dashboard endpoint that wasn't designed with the most-restricted caller in mind | Information Disclosure | D-GATE-03's requirement that the employee dashboard be a genuinely separate, narrower query path rather than a filtered view of the same org-wide query — see Common Pitfalls #4. |
| Trusting client-supplied scoping/filter parameters as if they were access-control decisions | Tampering / Elevation of Privilege | Server-side override of `requested_by_user_id` for `Role::Employee`, ignoring client input — see Architecture Patterns Pattern 2 and V5 row above. |

## Sources

### Primary (HIGH confidence — direct codebase reads, this session)
- `crates/trackly-core/src/auth.rs` — current `authorize()` matrix, `Action`/`Role`/`Identity` definitions
- `crates/trackly-app/src/services/device_service.rs`, `act_service.rs`, `cartridge_service.rs`, `printer_service.rs`, `report_service.rs`, `request_service.rs`, `dashboard_service.rs` — read-method signatures, presence/absence of `authorize()` and `caller` params
- `crates/trackly-app/src/http/devices.rs`, `acts.rs`, `cartridges.rs`, `printers.rs`, `reports.rs`, `requests.rs`, `dashboard.rs`, `users.rs`, `auth.rs` — `session_identity()` usage patterns
- `crates/trackly-app/src/tauri_cmds/devices.rs`, `requests.rs`, `dashboard.rs`, `users.rs` — `resolve_tauri_identity()` usage patterns
- `crates/trackly-app/src/error_axum.rs` — `AppError` → `StatusCode` mapping (confirms `Forbidden` → 403 already wired)
- `crates/trackly-infra/src/repos/requests_sqlite.rs` — confirms `requested_by_user_id` SQL parameterization already exists
- `migrations/V006__requests.sql` — confirms `requested_by_user_id` column already exists (no migration needed)
- `crates/trackly-app/tests/role_endpoint_matrix.rs` — full existing CI test matrix structure, macros, helpers, all 9 cases
- `ui/src/App.svelte`, `ui/src/routes.ts`, `ui/src/features/layout/sidebar-config.ts`, `Layout.svelte`, `Sidebar.svelte` — current frontend shell/routing/sidebar structure
- `ui/src/lib/stores/auth.svelte.ts`, `ui/src/lib/api/client.ts` — current auth store shape and 401-handling chokepoint
- `ui/src/features/requests/RequestsPage.svelte`, `ui/src/features/dashboard/DashboardPage.svelte` — current request-filtering and dashboard-widget-rendering behavior
- `.planning/config.json` — confirms `nyquist_validation: true`, `security_enforcement: true`, `security_asvs_level: 1`
- `.github/workflows/ci-fast.yml`, `ci-full.yml` — confirms `cargo test --workspace --no-fail-fast -- --test-threads=1` CI invocation
- `.planning/phases/10-employee-employee-ui-role-gating-read/10-CONTEXT.md` — locked decisions, source of all `D-*` IDs cited throughout

### Secondary (MEDIUM confidence)
- None required for this phase — the domain is entirely internal-codebase archaeology, not external library research, so no WebSearch/Context7 lookups were necessary or performed.

### Tertiary (LOW confidence)
- `svelte-spa-router` v5's exact guard/`wrap()`/`conditionsFailed` API surface for implementing
  D-DENY-01's route guard — not verified against current official docs in this session (see Open
  Question #3). Recommend a focused doc check before implementation.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; all existing library usage confirmed by direct reads.
- Architecture: HIGH — every read method's current authorize()/caller-threading status confirmed by
  direct source reads across all six affected services and both transport layers.
- Pitfalls: HIGH — all five pitfalls are derived from directly-observed code (the actual current state
  of `auth.rs`, the actual existing test assertions, the actual `dashboard_service` aggregation shape),
  not speculation about generic RBAC pitfalls.
- Frontend routing guard mechanism (Open Question #3): MEDIUM-LOW — `svelte-spa-router`'s presence
  and version confirmed, but its v5 guard API was not independently verified against official docs in
  this session.

**Research date:** 2026-06-21
**Valid until:** 2026-07-21 (30 days — this is internal-codebase research with no external library
version drift risk; the only decay vector is if the codebase itself changes materially before
planning/execution begins, which the planner should re-verify with a quick diff check if there's a
gap between this research and execution).
