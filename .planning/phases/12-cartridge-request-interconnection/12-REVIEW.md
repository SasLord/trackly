---
phase: 12-cartridge-request-interconnection
reviewed: 2026-06-22T00:00:00Z
depth: standard
files_reviewed: 14
files_reviewed_list:
  - crates/trackly-core/src/domain/cartridges.rs
  - crates/trackly-core/src/domain/requests.rs
  - crates/trackly-app/src/dto/cartridge.rs
  - crates/trackly-app/src/dto/request.rs
  - crates/trackly-app/src/services/request_service.rs
  - crates/trackly-infra/src/repos/cartridges_sqlite.rs
  - crates/trackly-infra/src/repos/requests_sqlite.rs
  - crates/trackly-app/tests/cartridges_lifecycle.rs
  - crates/trackly-app/tests/phase06_stubs.rs
  - crates/trackly-app/tests/role_endpoint_matrix.rs
  - ui/src/bindings-phase6.ts
  - ui/src/features/cartridges/OperationModal.svelte
  - ui/src/features/requests/RequestDetail.svelte
  - ui/src/lib/components/CartridgeSelect.svelte
findings:
  critical: 1
  warning: 7
  info: 4
  total: 12
status: issues_found
---

# Phase 12: Code Review Report

**Reviewed:** 2026-06-22
**Depth:** standard
**Files Reviewed:** 14
**Status:** issues_found

## Summary

Phase 12 interconnects the cartridge-replace request flow with cartridge installation: an `installable_only` filter on the cartridge list, a request↔cartridge history link in `RequestService::transition`, a compatible-cartridge selector on the frontend, and printer-location auto-fill.

The security posture is solid: all SQL is parameterised (verified across both repos), RBAC gates are enforced at the builder layer (`build_cartridges_transition` → `Action::MutateCartridges`, `transition` → `Action::TransitionRequests`), the new role-matrix Cases 31/32 lock the Employee-denial path, and optimistic locking is consistently applied.

The headline correctness defect is the `installable_only` filter: it whitelists only charge states `IN (1, 2)`, which are **cartridge** states. Photo-drums (kind 2) use states 4/5/6, so the request-centric install picker silently excludes every drum — a hard data-correctness bug for any drum-replace request. There are also several robustness gaps in the install→complete frontend handshake (double toasts, fire-and-forget completion, no compatibility scoping when the request has no model).

## Critical Issues

### CR-01: `installable_only` filter excludes all photo-drums via cartridge-only charge states

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:966` and `:985`
**Issue:** The new `installable_only` predicate is hardcoded to `(?5 = 0 OR c.state_id IN (1, 2))`. Per `migrations/V001` and `migrations/V017`, states `1=Полный, 2=Частичный, 3=Пустой` belong to cartridges (kind 1), while drums (kind 2) use `4=Новый, 5=Изношенный, 6=Отработанный`. A photo-drum can never have `state_id IN (1, 2)`, so when the request-centric install picker (`OperationModal` with `installable_only: true`) is opened for a drum-model request, the filter returns zero rows regardless of available stock — the operator sees "Нет подходящих картриджей на складе" even when full new drums exist. The domain doc comment (`cartridges.rs:209`) only describes cartridge semantics ("заряд Полный(1)/Частичный(2)") and the test suite (`cartridges_lifecycle.rs`) only exercises kind-1 cartridges, so the gap is unguarded. The installable kind-2 equivalent (exclude only `6=Отработанный`, which `transition_in_tx` already refuses to install at line 375) is never expressed.
**Fix:** Make the predicate kind-aware so drums are installable in their non-spent states:
```sql
AND (?5 = 0 OR (
        (m.kind_id = 1 AND c.state_id IN (1, 2))
     OR (m.kind_id = 2 AND c.state_id IN (4, 5))
))
```
Add a `cartridges_lifecycle.rs` case that seeds a kind-2 drum with `state_id = 4` and asserts `installable_only: true` returns it. (If drums are intentionally out of scope for request-install, gate the picker on `kind_id = 1` explicitly and document it — but silent exclusion is the bug.)

## Warnings

### WR-01: `installable_only` does not constrain `status_id`; relies entirely on the caller

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:966`, `:985`
**Issue:** The domain doc (`cartridges.rs:209`, DTO `cartridge.rs:336`) states `installable_only` means "Только статус «На складе» (1) И заряд ...". But the SQL only filters `state_id`, not `status_id`. It works today purely because the frontend always co-sends `status_id: 1` (`OperationModal.svelte:128`). A different caller (or a future refactor that drops the explicit status) passing `installable_only: true` without `status_id` would return in-use/at-refill/written-off cartridges as "installable", which then fail `validate_from_status` at install time. The filter does not enforce its own documented invariant.
**Fix:** Fold the status requirement into the `installable_only` branch so the predicate is self-contained:
```sql
AND (?5 = 0 OR (c.status_id = 1 AND <kind-aware state check from CR-01>))
```

### WR-02: Install picker shows model-incompatible cartridges when the request has no `cartridge_model_id`

**File:** `ui/src/features/requests/RequestDetail.svelte:588`, `ui/src/features/cartridges/OperationModal.svelte:130`
**Issue:** `cartridgeModelId={request.cartridgeModelId ?? undefined}` — `cartridge_model_id` is optional on a `cartridge_replace` request (`requests.rs:60`). When it is `None`, `OperationModal` calls `cartridges.list({ model_id: cartridgeModelId ?? null, ... })`, i.e. **no** model filter, so the picker lists every installable cartridge of every model. The operator can then install a cartridge that does not fit the request's printer, and the backend performs no model/printer compatibility check (`transition_in_tx` validates only status/kind, never compatibility against `cartridge_model_compatibility`). The phase intent ("compatible-cartridge selector") is silently defeated whenever the model is unset.
**Fix:** When `request.cartridgeModelId` is null, either (a) derive compatible model(s) from the printer via `cartridge_model_compatibility` before listing, or (b) require a model selection / show an explicit "модель не указана — выберите вручную" warning. At minimum, document that no compatibility guarantee holds in this path.

### WR-03: Install success toast fires even if request completion fails (fire-and-forget `onSuccess`)

**File:** `ui/src/features/cartridges/OperationModal.svelte:281-283`, `ui/src/features/requests/RequestDetail.svelte:301-323`
**Issue:** `handleSubmit` calls `onSuccess(effectiveCartridge.id)` then unconditionally `pushToast('success', 'Операция выполнена успешно.')`. `onSuccess` is the async `handleInstallSuccess`, which is **not awaited** — it returns a floating promise. If the subsequent `requests.transition({op:'complete', linkedCartridgeId})` rejects, the user sees both the green "Операция выполнена успешно." (from the modal) and a red "Не удалось завершить заявку. Проверьте вручную." (from the handler). The request is left in `in_progress` with the cartridge already installed, but the success toast implies the whole flow worked.
**Fix:** Make `onSuccess` awaited (`onSuccess: (id) => Promise<void>`) and only emit the modal-level success toast after it resolves, or move the success toast into `handleInstallSuccess` so it reflects the true end state.

### WR-04: Stale request `version` used for the post-install Complete transition

**File:** `ui/src/features/requests/RequestDetail.svelte:306-312`
**Issue:** `handleInstallSuccess` sends `version: request.version` from the prop captured at render time. Installing the cartridge does not bump the request row, so this is correct in the common case — but if any other actor transitions the request between modal open and completion (e.g. another specialist accepts/rejects, or a WS-driven refresh has not yet propagated), the Complete fires with a stale version and returns `OptimisticLockMismatch`, surfacing as the generic "Не удалось завершить заявку" with the cartridge already installed and unlinked. There is no re-fetch of the current version before completing.
**Fix:** Re-read the request (or thread the latest version from `onTransition`/the list store) immediately before the Complete call, or have the backend expose a single "install-and-complete" operation so the cartridge install and request completion share one optimistic-lock boundary.

### WR-05: `printer_options` join can return duplicate rows if a device has >1 matching location (LEFT JOIN unguarded)

**File:** `crates/trackly-app/src/services/request_service.rs:249-257`
**Issue:** The query `LEFT JOIN locations l ON d.location_id = l.id` is 1:1 only if `locations.id` is unique (it is, as PK) — so this is low-risk — but the `WHERE d.type_id = (SELECT id FROM device_types WHERE name = 'Принтер')` subquery returns NULL if the seed name is ever renamed, silently making `d.type_id = NULL` always-false and returning an empty printer list with no error. The "resilience" comment (WR-04 inline) assumes the name is stable; a localized rename of the seed would break the dropdown invisibly.
**Fix:** Either assert the subquery resolves (`COALESCE((SELECT id ...), -1)` won't help; better to fail loudly) or keep a unit test that the `'Принтер'` device-type seed exists so a rename breaks CI rather than production.

### WR-06: `relativeDate` / `formatFullDate` render history timestamps in UTC, not local time

**File:** `ui/src/features/requests/RequestDetail.svelte:135-142`
**Issue:** Both helpers use `getUTCDate/getUTCMonth/getUTCHours`. For a RU/Moscow single-tz deployment (UTC+3) every history line and "Создана" date is shown 3 hours behind wall-clock. The cartridge `OperationModal` builds `date_utc` from `new Date(iso + 'T00:00:00Z')` (also UTC midnight), so the round-trip is internally consistent but user-visibly wrong by the local offset. This is a correctness issue for an audit/history view where timestamps are load-bearing.
**Fix:** Use local accessors (`getDate/getMonth/getHours`) for display, or format with an explicit Europe/Moscow offset. Confirm the intended tz convention against the act/cartridge history views for consistency.

### WR-07: `get_history` notes-extraction silently swallows malformed payload JSON

**File:** `crates/trackly-app/src/services/request_service.rs:205-213`
**Issue:** The `notes` extraction does `serde_json::from_str::<Value>(p).ok().and_then(...)`. A payload that is non-JSON or has a non-string `notes` value yields `None` with no trace/log. Given Phase 12 newly writes `{"notes": "<plain>; Установлен C-... (Brand Model)"}` into this same field, any future format drift (e.g. someone stores a structured object under `notes`) would make completed-with-cartridge history lines silently lose their text. Not a security issue, but a debuggability gap on a freshly-touched code path.
**Fix:** Keep the graceful fallback but add a `tracing::debug!` (or `warn!`) when `payload_json` is present yet fails to parse / lacks a string `notes`, so the drop is observable.

## Info

### IN-01: Duplicate success toast on the install→complete happy path

**File:** `ui/src/features/cartridges/OperationModal.svelte:283`, `ui/src/features/requests/RequestDetail.svelte:313`
**Issue:** On a successful request-centric install, the user sees "Операция выполнена успешно." (modal) immediately followed by "Заявка выполнена" (handler). Two stacked success toasts for one logical action.
**Fix:** Suppress the modal-level toast when invoked from the request flow (e.g. pass a `silentSuccess` flag), letting the caller own the single user-facing message.

### IN-02: `printerContextHint` shows raw device id, not printer name

**File:** `ui/src/features/cartridges/OperationModal.svelte:109-113`
**Issue:** `Устанавливается в принтер #${preFillPrinterId}` displays the numeric `printer_device_id`, which is meaningless to an operator. The request already carries `printerName`, but it is not threaded into the modal.
**Fix:** Pass `request.printerName` as a prop and render the human name (fall back to `#id` only if absent).

### IN-03: `actionLabel` maps `custom:*` actions but request audit writes mixed prefixes

**File:** `ui/src/features/requests/RequestDetail.svelte:144-156`, `crates/trackly-app/src/services/request_service.rs:361,543,797`
**Issue:** `create` is written without a prefix (`action: "create"`), `transition` writes `op.audit_action()` → `"custom:accept"|"custom:complete"|"custom:reject"`, and `reject_ad_register` writes the literal `"custom:reject"`, while `approve_ad_register` writes `"ad_register_approve"`. The `actionLabel` map covers `create`, the four bare verbs, and the four `custom:` verbs, but **not** `ad_register_approve` — an AD-approve history row renders the raw string `ad_register_approve`. Minor (admin-only screen) but inconsistent.
**Fix:** Add `ad_register_approve` (and any other action strings actually produced) to the `labels` map, or normalize action strings server-side.

### IN-04: `buildPayload` uses non-null assertions on `effectiveCartridge`

**File:** `ui/src/features/cartridges/OperationModal.svelte:191-192`
**Issue:** `effectiveCartridge!.id` / `!.version` rely on the runtime guard in `handleSubmit` (`if (!effectiveCartridge ... return`). The assertions are safe today because `buildPayload` is only reached after that guard, but the coupling is implicit; a future caller of `buildPayload` would get a null deref.
**Fix:** Pass the resolved cartridge into `buildPayload(cartridge)` as a non-null parameter, or early-return inside `buildPayload`.

---

_Reviewed: 2026-06-22_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
