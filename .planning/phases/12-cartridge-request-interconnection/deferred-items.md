# Deferred Items — Phase 12

Out-of-scope discoveries logged during plan execution. Not fixed (per scope boundary rule).

## Plan 12-04

- **`restore_request_visibility_http.rs::blocked_user_restore_request_visible_to_admin_and_marks_pending_http`** —
  fails in this dev environment with `503 service unavailable: ad` instead of the expected `403`.
  Root cause: test relies on AD reachability (`ad_mode="real"` default, no `TRACKLY_AD_MODE=mock`
  env var set for this invocation) and no AD/LDAP server is reachable from the macOS dev box
  (documented constraint — see project memory `dev_environment_constraints`). Pre-existing,
  unrelated to `act_service.rs`/`suggest_person()` — last touched in Phase 9 (`2a029f1`,
  `344a6fc`), well before this plan. Not fixed; out of scope for 12-04's `suggest_person` change.

## Plan 12-07

- **`svelte-check` pre-existing errors in `OperationModal.svelte:143` and `CartridgesPage.svelte:60`** —
  both construct a `CartridgeFilter` object literal missing the
  `compatible_with_printer_device_id` field added in Plan 12-05 (D-13/D-14). Confirmed via
  `git stash` diff that both errors exist identically before and after this plan's changes —
  unrelated to 12-07's two new editor components / API wrappers. Not fixed; belongs to whichever
  plan touches `OperationModal.svelte`/`CartridgesPage.svelte` filter construction next.
  **Resolved for `OperationModal.svelte` by Plan 12-08** (added `compatible_with_printer_device_id`
  to the install picker's filter object as part of the D-13/D-14 compatibility-aware filtering
  feature). `CartridgesPage.svelte:60` remains unresolved — out of scope for 12-08
  (`files_modified` only listed `OperationModal.svelte`); still belongs to whichever plan touches
  `CartridgesPage.svelte`'s filter construction next.

## Plan 12-10

- **`restore_request_visibility_http.rs::blocked_user_restore_request_visible_to_admin_and_marks_pending_http`** —
  same pre-existing `503 service unavailable: ad` failure documented under Plan 12-04, reconfirmed
  via `git stash`/`git stash pop` around this plan's commits (identical failure before and after).
  Unrelated to the V030 `printers` CHECK-constraint removal or the new
  `test_printer_no_ip_no_usb` regression test. Not fixed; out of scope for 12-10.
