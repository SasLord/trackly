---
phase: 11-requests-employee-ux-gaps
verified: 2026-06-22T00:20:28Z
status: human_needed
score: 11/11 must-haves verified
overrides_applied: 0
---

# Phase 11: Requests/Employee UX Gap-Closure Verification Report

**Phase Goal:** Закрыть находки UAT после Phase 9/10 по заявкам и опыту сотрудника: (1) категория заявки отображается текстом, а не числом; (2) ответ администратора приходит сотруднику по WebSocket с тостом, а при свёрнутой/неактивной вкладке — системной нотификацией; (3) сотрудник снова может завести заявку на замену картриджа — отдельный доступный ему эндпоинт списка принтеров + кастомный дропдаун, сгруппированный по Расположению.
**Verified:** 2026-06-22T00:20:28Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Деталь заявки показывает название категории текстом, не число | ✓ VERIFIED | `requests_sqlite.rs:40,44,70` LEFT JOIN + idx 18; `RequestDetail.svelte:387-391` renders `request.categoryName` as text |
| 2 | Заявка без категории (free_form) не рисует блок категории — никогда null/число | ✓ VERIFIED | `RequestDetail.svelte:387` truthy-gated `{#if request.categoryName}` — renders nothing when null, never "null"/number |
| 3 | Эндпоинт списка категорий возвращает `{id, name}` (оба транспорта); форма шлёт корректный category_id | ✓ VERIFIED | `dto/request.rs:85` `RequestCategoryDto{id,name}`; `tauri_cmds/requests.rs:103-127` `SELECT id, name`; `http/requests.rs:168,173` delegates to same `build_requests_list_categories`; `RequestFormModal.svelte` options iterate `categories` by `cat.id`/`cat.name` |
| 4 | RequestFormModal не содержит хардкод-массив CATEGORIES | ✓ VERIFIED | `grep -n "const CATEGORIES"` → no match; replaced by `$state<RequestCategoryDto[]>([])` + `listCategories()` |
| 5 | Сотрудник видит непустой список принтеров, сгруппированный по Расположению | ✓ VERIFIED | `GroupedPrinterSelect.svelte` (149 lines) groups by location with `var(--color-surface-sunken)` header; `RequestFormModal.svelte:77` calls `requests.printerOptions()` |
| 6 | Сотрудник может выбрать принтер и отправить заявку на замену картриджа (регрессия Phase 10 устранена) | ✓ VERIFIED | `request_printer_options` gated on `Action::CreateRequest` (not `ReadData`/`ReadPrinters`); `tests/request_printer_options.rs` employee→200; `tests/role_endpoint_matrix.rs` green |
| 7 | Новый эндпоинт возвращает строго {id, name, location} — без SNMP/community/ip | ✓ VERIFIED | `dto/request.rs:99-108` `RequestPrinterOptionDto{id,name,location}`; integration test asserts minimal-key JSON (`employee_gets_printer_options_minimal_dto` passes) |
| 8 | Сотрудник получает событие изменения статуса ТОЛЬКО своей заявки — никогда чужой | ✓ VERIFIED | `dto/printer.rs:217-231` split-arm `is_visible_to`; 4 unit tests pass (author→true, other-employee→false, admin/manager→true, NewRequest regression) |
| 9 | При активной вкладке — тост; при свёрнутой/неактивной (document.hidden) в secure-context — системная нотификация; иначе graceful-degrade на тост | ✓ VERIFIED | `EmployeeLayout.svelte:45-58` `canNotify` gate + `document.hidden` branch → `new Notification(...)` else `pushToast(...)` |
| 10 | Разрешение на нотификации запрашивается деликатно (после первой успешной отправки заявки), не при загрузке | ✓ VERIFIED | `RequestFormModal.svelte:121-129,144-145` `maybeRequestNotifyPermission()` called only inside `handleSubmit` success branch, gated on `permission === 'default'`; no call at module/onMount level |
| 11 | Полный cargo build + затронутые тесты зелёные; биндинги без дрейфа | ✓ VERIFIED | See "Behavioral Spot-Checks" — all targeted test suites pass; `export_bindings` test passes; `bindings.ts`/`bindings-phase6.ts` both carry the new fields |

**Score:** 11/11 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-infra/src/repos/requests_sqlite.rs` | LEFT JOIN request_categories + category_name idx 18 | ✓ VERIFIED | Line 40/44/70; indices 0-17 unchanged (verified by reading map_row_request) |
| `crates/trackly-core/src/domain/requests.rs` | `RequestRow.category_name: Option<String>` | ✓ VERIFIED | Line 45 |
| `crates/trackly-app/src/dto/request.rs` | `RequestDto.category_name` + `From` mapping; `RequestCategoryDto{id,name}`; `RequestPrinterOptionDto{id,name,location}` | ✓ VERIFIED | Lines 50, 74, 85, 99-108 |
| `crates/trackly-app/src/tauri_cmds/requests.rs` | `build_requests_list_categories` returns `Vec<RequestCategoryDto>`; `build_request_printer_options` + `request_printer_options` command | ✓ VERIFIED | Lines 103-127, 138, 237 |
| `crates/trackly-app/src/http/requests.rs` | Categories handler typed `Json<Vec<RequestCategoryDto>>`; `handler_request_printer_options` + route; no duplicate `ws_broadcast` after CR-01 fix | ✓ VERIFIED | Lines 168/173 (categories), 196/204/231 (printer options); 112-116/130-131/146-147 show CR-01 fix comments, no `ws_broadcast`/`ws_tx.send` present |
| `crates/trackly-app/src/specta_export.rs` | `request_printer_options` registered | ✓ VERIFIED | Line 123 |
| `crates/trackly-app/src/dto/printer.rs` | `WsEvent::RequestStatusChanged += requested_by_user_id`; split-arm `is_visible_to` | ✓ VERIFIED | Lines 190-231; 4 unit tests pass |
| `crates/trackly-app/src/services/request_service.rs` | `printer_options` (CreateRequest-gated, type_id resolved by name) + 3 send-sites filling `requested_by_user_id` | ✓ VERIFIED | Lines 224-268 (printer_options); 516/650/769 (send-sites) |
| `ui/src/features/requests/RequestDetail.svelte` | Renders `categoryName` text, not `categoryId` | ✓ VERIFIED | Lines 387-391; no remaining `categoryId` render |
| `ui/src/features/requests/RequestFormModal.svelte` | Hardcoded `CATEGORIES` removed; consumes `listCategories()`; uses `GroupedPrinterSelect` + `printerOptions()`; delicate permission request | ✓ VERIFIED | No `const CATEGORIES` match; `categories` state + `loadCategories`; `printerOptions()`; `maybeRequestNotifyPermission()` in submit-success branch |
| `ui/src/lib/components/GroupedPrinterSelect.svelte` | New component, ≥40 lines, gray group header, empty-state text | ✓ VERIFIED | 149 lines; `var(--color-surface-sunken)`; "Принтеры не найдены" |
| `ui/src/features/layout/EmployeeLayout.svelte` | WS subscription, toast/notification dispatch by `document.hidden` | ✓ VERIFIED | Lines 13, 32-58, 60-77; gated to `role === 'employee'` |
| `ui/src/bindings-phase6.ts` / `ui/src/bindings.ts` | `categoryName`, `RequestCategoryDto`, `RequestPrinterOptionDto`, `WsEvent.requestedByUserId` present | ✓ VERIFIED | Confirmed in both files via grep; `bindings.ts` regenerated by `export_bindings` test, exit 0 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `requests_sqlite.rs SELECT_REQUESTS` | `request_categories.name` | `LEFT JOIN` | ✓ WIRED | Confirmed; idx 18 mapped in `map_row_request` |
| `RequestDetail.svelte` | `RequestDto.categoryName` | render | ✓ WIRED | Confirmed conditional render |
| `build_requests_list_categories` (Tauri) + http handler | `RequestCategoryDto{id,name}` | shared `build_*` | ✓ WIRED | Both transports delegate to the same function — no duplicated SQL |
| `RequestFormModal.svelte` | `RequestCategoryDto[]` | `api.listCategories()` | ✓ WIRED | Confirmed; hardcode removed |
| `RequestService::printer_options` | `Action::CreateRequest` | authorize gate | ✓ WIRED | Confirmed; regression-tested via `role_endpoint_matrix.rs` |
| `RequestFormModal.svelte` | `request_printer_options` endpoint | `requests.printerOptions()` | ✓ WIRED | Confirmed; old `devices.list({type_id` call removed |
| `specta_export.rs collect_commands!` | `request_printer_options` | command registration | ✓ WIRED | Confirmed line 123 |
| `WsEvent::is_visible_to` | `identity.user_id == requested_by_user_id` | split-arm `RequestStatusChanged` | ✓ WIRED | Confirmed; 4 unit tests green |
| `EmployeeLayout.svelte` | `ws.ts connectWs/onWsEvent` | `onMount` subscription | ✓ WIRED | Confirmed; cleanup returned |
| `RequestFormModal.svelte success` | `Notification.requestPermission` | `maybeRequestNotifyPermission` gesture-gated | ✓ WIRED | Confirmed; called only after submit success, gated on `permission === 'default'` |
| `RequestService::create/transition/approve_ad_register` | single WS broadcast | `ws_tx.send` (HTTP handler no longer re-sends) | ✓ WIRED | CR-01 regression test `ws_http_single_broadcast.rs` passes — exactly one event delivered |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| D-CAT-01 repo tests (Some/None category_name, categories list invariant) | `cargo test -p trackly-infra --lib repos::requests_sqlite` | 6 passed, 0 failed | ✓ PASS |
| D-PRN-01 endpoint integration tests | `cargo test -p trackly-app --test request_printer_options` | 3 passed (401, empty-list, minimal-DTO) | ✓ PASS |
| D-PRN-01 auth-matrix regression | `cargo test -p trackly-app --test role_endpoint_matrix` | 1 passed | ✓ PASS |
| D-WS-01 visibility split-arm unit tests | `cargo test -p trackly-app --lib dto::printer::tests` | 4 passed | ✓ PASS |
| D-WS-01 broadcast fan-out regression | `cargo test -p trackly-app --test ws_broadcast_fanout` | 1 passed | ✓ PASS |
| CR-01 single-broadcast regression (review-fix) | `cargo test -p trackly-app --test ws_http_single_broadcast` | 1 passed | ✓ PASS |
| WR-01/WR-02 create() validation regression | `cargo test -p trackly-app --test phase06_stubs` | 13 passed, 1 ignored | ✓ PASS |
| AD-register lifecycle regression | `cargo test -p trackly-app --test requests_ad_register_http` | 3 passed | ✓ PASS |
| Bindings drift check | `cargo test -p trackly-app --test export_bindings` | 1 passed | ✓ PASS |
| Frontend type-check | `pnpm --dir ui exec svelte-check --threshold error` | 0 errors, 36 warnings (pre-existing) | ✓ PASS |
| Frontend LAN bundle build | `pnpm --dir ui build` | succeeded, `dist/` produced | ✓ PASS |
| Backend build (both transports) | `cargo build -p trackly-app` | succeeded | ✓ PASS |
| Lint on modified crates | `cargo clippy -p trackly-infra -p trackly-app -- -D warnings` | 0 warnings | ✓ PASS |
| Environmental pre-existing failure (documented, not a regression) | `cargo test -p trackly-app --test restore_request_visibility_http` | FAILED — `ad_mode="real"`, AD unreachable from dev macOS (503 vs expected 403) | ? SKIP (documented dev-environment constraint, file untouched by this phase) |

### Probe Execution

No probes declared for this phase and no conventional `scripts/*/tests/probe-*.sh` found. SKIPPED (no runnable entry points of this kind).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|--------------|--------|----------|
| D-CAT-01 | 11-01 | Категория заявки текстом, не числом; список категорий {id,name} | ✓ SATISFIED | LEFT JOIN, RequestCategoryDto, RequestDetail render, RequestFormModal hardcode removal — all verified above |
| D-PRN-01 | 11-02 | Отдельный CreateRequest-гейтед эндпоинт списка принтеров + сгруппированный дропдаун | ✓ SATISFIED | RequestPrinterOptionDto, printer_options service method, GroupedPrinterSelect — all verified above |
| D-WS-01 | 11-03 | WS-тост/нотификация сотруднику по своей заявке; деликатный permission-запрос | ✓ SATISFIED | requested_by_user_id field, split-arm is_visible_to, EmployeeLayout subscription, gesture-gated permission — all verified above |

**Note on REQUIREMENTS.md cross-reference:** D-CAT-01, D-WS-01, D-PRN-01 are phase-local gap-closure requirement IDs declared in `11-CONTEXT.md` and `ROADMAP.md` (line 409) — they are UAT-driven follow-up findings from Phase 9/10, not part of the original v1 traceability table in `.planning/REQUIREMENTS.md` (which only tracks FOUND/DEV/ACT/CART/PRN/REQ/RPT/USR/DASH/SET/UI/SRV/BLD IDs). This is consistent with the project's established gap-closure convention (same pattern as prior `D-*` IDs referenced in 11-RESEARCH.md, e.g. `D-RBAC-03`, `D-REQ-01`, `D-UI-01` from earlier phases). No orphaned requirements found — all 3 declared IDs are claimed by exactly one plan each, and ROADMAP.md's phase-11 entry lists exactly these 3.

### Anti-Patterns Found

No debt markers (`TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER`) found in any of the 13 files modified across the 3 plans. No stub patterns (`return null`/`=> {}`/empty-array-only renders) found in the modified Svelte components. No blockers or warnings from this scan.

The independent code-review pass (`11-REVIEW.md`) found 1 Critical (CR-01, duplicate WS broadcast) and 6 Warnings (WR-01 through WR-06); the fix pass (`11-REVIEW-FIX.md`) addresses all 7 with commits `e500b6c`, `202816d`, `e098afc`, `b9e1cc7`, `a4e27f6` — all confirmed present in git log and all associated regression tests pass (verified independently above, not just trusted from the fix report).

### Human Verification Required

The following items require a live LAN-browser session and cannot be verified by static analysis or automated tests. These are explicitly called out as MANUAL in the plans' own `<acceptance_criteria>`/`<verification>` blocks (harvested per workflow #3309) and are appropriate to defer to end-of-phase human testing.

### 1. Category name renders correctly in both desktop and LAN browser

**Test:** Open a request with category "Программное обеспечение" in the Tauri desktop webview AND in a LAN browser (after `pnpm --dir ui build`, already done).
**Expected:** Detail view shows the Russian category name as text, not the number "3"; a free_form request without a category shows no category block (not "—", not "null", not "0").
**Why human:** Requires visual confirmation in a running webview/browser session against seeded data; grep confirms the code path but not the rendered pixel output.

### 2. Create-request form dropdown is server-populated and submits correct category_id

**Test:** Open the create-request form (desktop + LAN browser) and confirm the category dropdown is populated from the server and that creating a request sends the correct `category_id`.
**Expected:** Dropdown shows all 4 seeded categories; selecting one and submitting creates a request whose detail view later shows the matching category name.
**Why human:** End-to-end UI interaction + form submission; not observable via static code reading alone.

### 3. Employee sees non-empty grouped printer dropdown and can submit a cartridge-replace request

**Test:** Log in as an employee in a LAN browser, open the create-request form, select "Замена картриджа".
**Expected:** Printer dropdown is non-empty, grouped by Расположение with gray group-header strips, sorted by location; selecting a printer and submitting succeeds.
**Why human:** Requires a live employee session against seeded device/location data and visual confirmation of the grouping UI.

### 4. Realtime toast/notification delivery scoped to the request's own author

**Test:** Open two employee LAN-browser sessions (Employee A, Employee B). Employee A submits a request. Admin changes Employee A's request status while A's tab is active (expect toast) and while A's tab is hidden with Notification permission granted on a secure context `https://...:8443` (expect system Notification). Confirm Employee B receives nothing for A's request.
**Expected:** A sees the correctly-worded RU toast/notification depending on tab visibility; B receives no event for A's request.
**Why human:** Requires two concurrent live browser sessions, an admin actor, and Page Visibility/Notification API browser behavior that cannot be exercised by an automated test runner in this codebase (per project convention, frontend test runner is out of scope — confirmed by `11-RESEARCH.md`).

### 5. Delicate permission prompt timing

**Test:** As a fresh employee session (Notification.permission === 'default'), load the request page (no prompt expected), then submit a request (prompt expected once).
**Expected:** No browser permission prompt appears on page load; exactly one prompt appears immediately after the first successful request submission; it does not reappear on subsequent submissions.
**Why human:** Browser permission-prompt UI cannot be triggered or observed by grep/cargo test; requires a live browser session.

### 6. HTTP first-run fallback graceful-degrade

**Test:** Access the app over plain HTTP (first-run, no HTTPS cert yet) as an employee; trigger a status change from admin.
**Expected:** No console errors; notification gracefully degrades to a toast since `window.isSecureContext` is false.
**Why human:** Requires a non-secure-context browser session; `isSecureContext` behavior is environment-dependent and not exercisable from the Rust/Vitest-less test suite.

### 7. WR-03 error-semantics behavior change confirmation

**Test:** Confirm no other part of the system (frontend error handling, other tests) depended on receiving `OptimisticLockMismatch` specifically (rather than `NotFound`) when a request is concurrently soft-deleted during a transition.
**Expected:** No regression in user-facing error messaging for this rare concurrent-delete race.
**Why human:** This was explicitly flagged by the executor itself in `11-REVIEW-FIX.md` as needing human confirmation; no existing test exercised this branch before the change, and grep alone cannot prove the absence of a downstream behavioral dependency on the old error variant.

### Gaps Summary

No blocking gaps. All 11 must-have truths derived from the phase goal (3 D-requirements) are verified against actual running code: passing tests, clean builds, confirmed wiring (LEFT JOIN → DTO → render; gated endpoint → grouped dropdown; WS payload + split-arm visibility → employee subscription). The independent code review's 1 Critical + 6 Warning findings were all fixed in a documented follow-up pass, and this verification independently re-ran the regression tests for each fix (CR-01, WR-01/02, WR-03, WR-04/06) rather than trusting the fix report's claims — all green.

Status is `human_needed` rather than `passed` solely because the phase's own plans defer several acceptance-criteria items to live-browser manual testing (visual rendering, two-session WS isolation, Notification permission UX, secure-context fallback) — these are correctly un-automatable and are surfaced above for the developer to execute.

---

*Verified: 2026-06-22T00:20:28Z*
*Verifier: Claude (gsd-verifier)*
