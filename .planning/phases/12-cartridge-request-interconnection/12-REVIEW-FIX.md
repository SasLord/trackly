---
phase: 12-cartridge-request-interconnection
fixed_at: 2026-06-22T11:21:39Z
review_path: .planning/phases/12-cartridge-request-interconnection/12-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 12: Code Review Fix Report

**Fixed at:** 2026-06-22T11:21:39Z
**Source review:** .planning/phases/12-cartridge-request-interconnection/12-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4 (CR-01/WR-01 combined, WR-02, WR-03, WR-04)
- Fixed: 4
- Skipped: 0

## Fixed Issues

### CR-01 / WR-01: `installable_only` filter excludes all photo-drums; does not constrain `status_id`

**Files modified:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs`, `crates/trackly-core/src/domain/cartridges.rs`, `crates/trackly-app/src/dto/cartridge.rs`, `crates/trackly-app/tests/cartridges_lifecycle.rs`
**Commit:** `6ac7dfb`
**Applied fix:** Made the `installable_only` SQL predicate kind-aware and self-contained in both the COUNT and SELECT branches of `CartridgeRepository::list`:

```sql
AND (?5 = 0 OR (c.status_id = 1 AND (
      (m.kind_id = 1 AND c.state_id IN (1, 2))
   OR (m.kind_id = 2 AND c.state_id IN (4, 5))
)))
```

Confirmed `cartridge_models m` is in scope via `LEFT JOIN cartridge_models m ON m.id = c.model_id` in both branches (the COUNT query's inline join and `SELECT_CARTRIDGES`'s join). Folded `c.status_id = 1` into the same branch (WR-01) so `installable_only` no longer depends on the caller co-sending `status_id`. Updated the doc comments on `CartridgeFilter::installable_only` (domain + DTO) to describe the kind-aware semantics. Added a regression test `installable_only_includes_new_drum_excludes_spent_drum` seeding a kind-2 drum at `state_id=4` (asserted present) and `state_id=6` Отработанный (asserted excluded, matching the existing install-time refusal in `transition_in_tx`).

### WR-02: Install picker shows model-incompatible cartridges when the request has no `cartridge_model_id`

**Files modified:** `ui/src/features/cartridges/OperationModal.svelte`
**Commit:** `74ac172`
**Applied fix:** Added a derived `noModelScopeWarning` shown next to the cartridge picker in the request-centric install flow (`op === 'install' && cartridge === null`) whenever `cartridgeModelId` is `undefined`. Per the finding's "minimum acceptable fix" guidance, this surfaces an explicit warning ("Модель не указана — проверьте совместимость вручную") rather than deriving compatible models from the printer (no existing binding makes that derivation straightforward without further backend work, so the explicit-warning path was chosen to avoid over-engineering). `RequestDetail.svelte`'s existing `cartridgeModelId={request.cartridgeModelId ?? undefined}` wiring required no change — it already converts `null` to `undefined` correctly for this check.

### WR-03: Install success toast fires even if request completion fails (fire-and-forget `onSuccess`)

**Files modified:** `ui/src/features/cartridges/OperationModal.svelte`, `ui/src/features/requests/RequestDetail.svelte`
**Commit:** `b7bd42a`
**Applied fix:** `OperationModal.handleSubmit` now `await`s `onSuccess` (type widened to `(_cartridgeId: number) => void | Promise<void>`) and only shows the modal-level "Операция выполнена успешно." toast after it resolves; on rejection the modal just closes (no duplicate/contradictory toast — the caller owns its own error message). `RequestDetail.handleInstallSuccess` now rethrows after its own catch block (which already pushes a specific error toast and refreshes), so the awaited `onSuccess` in the modal sees the rejection. Verified the cartridge-centric caller (`CartridgesPage.svelte`'s `handleOperationSuccess`, a synchronous void function) is unaffected — `await`ing a non-Promise-returning function resolves immediately with no behavior change.

### WR-04: Stale request `version` used for the post-install Complete transition

**Files modified:** `ui/src/features/requests/RequestDetail.svelte`
**Commit:** `fcadacd`
**Applied fix:** `handleInstallSuccess` now calls `requests.get(requestId)` immediately before the `complete` transition and uses the freshly-fetched `version` instead of the `request` prop's value captured at render time. This closes the window where a concurrent transition (another specialist accept/reject, or a WS-driven refresh not yet propagated) would cause `OptimisticLockMismatch` with the cartridge already installed.

## Deferred Issues (out of scope for this fix run)

The following findings from `12-REVIEW.md` were explicitly out of scope per the fix task and were **not** touched:

- **WR-05** — `printer_options` join can return duplicate rows / silent empty list if the `'Принтер'` device-type seed name is renamed (`crates/trackly-app/src/services/request_service.rs:249-257`).
- **WR-06** — `relativeDate` / `formatFullDate` render history timestamps in UTC instead of local (Europe/Moscow) time (`ui/src/features/requests/RequestDetail.svelte:135-142`).
- **WR-07** — `get_history` notes-extraction silently swallows malformed payload JSON with no trace/log (`crates/trackly-app/src/services/request_service.rs:205-213`).
- **IN-01** — Duplicate success toast on the install→complete happy path (modal toast + handler toast both fire on success).
- **IN-02** — `printerContextHint` shows raw device id instead of printer name.
- **IN-03** — `actionLabel` map missing `ad_register_approve` action string.
- **IN-04** — `buildPayload` uses non-null assertions on `effectiveCartridge`.

These remain in `12-REVIEW.md` for a follow-up fix pass.

## Verification

- `cargo build --workspace`: clean.
- `cargo test -p trackly-app --test cartridges_lifecycle`: 11/11 passed, including the new `installable_only_includes_new_drum_excludes_spent_drum` regression test.
- `pnpm --dir ui svelte-check` (via `npx svelte-check --output machine` in the worktree): 0 errors, 36 warnings (all pre-existing, none in files touched by this fix run).
- `cargo fmt --check` (via `cargo fmt -p trackly-app -p trackly-core -p trackly-infra`, scoped diff inspected per-file): clean for all 4 touched Rust files. Note: running `cargo fmt` at the package level also reformats two pre-existing, unrelated files (`crates/trackly-app/tests/request_printer_options.rs`, `crates/trackly-app/tests/ws_http_single_broadcast.rs`) that already had formatting drift before this fix run — those reformats were discarded (`git checkout --`) both times they appeared, since they are out of scope for this fix run and not part of any finding.

---

_Fixed: 2026-06-22T11:21:39Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
