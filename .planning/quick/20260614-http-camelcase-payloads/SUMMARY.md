---
quick_id: 260614-60l
slug: http-camelcase-payloads
date: 2026-06-14
status: complete
---

# Summary: HTTP transport camelCase payload parity (S-5)

## What changed

Added `#[serde(rename_all = "camelCase")]` to **49** axum request `*Payload`
deserialize structs across `crates/trackly-app/src/http/`:
acts.rs (+9), auth.rs (+1, LoginPayload), cartridges.rs (+12), devices.rs (+14),
fs_helpers.rs (+2), settings.rs (+3), templates.rs (+2), users.rs (+6).

The empty `StatusPayload {}` (Serialize+Deserialize) was skipped; the non-request
`SessionIdentity` (session storage) was not touched.

Now the browser/HTTP transport accepts the camelCase top-level arg keys the
frontend sends verbatim via `fetch` (S-5), matching the Tauri path where
tauri-specta converts camelCase → snake_case Rust params. Previously every
endpoint with a multi-word arg (`userNew`, `actId`, `userId`/`newPassword`,
`deviceId`/`giverName`/`receiverName`/`dateUtc`, `ctxName`/`ctxStatusId`/`statusIn`,
`sampleActId`) returned 422 over HTTP while working on desktop.

Single-word keys (filter, payload, patch, req, id, version, prefix, login,
password, path…) are unaffected — rename_all is a no-op for them, applied
uniformly to future-proof new multi-word fields.

## Why nested DTOs were NOT touched

`dto/auth.rs` (and device/act DTOs) are explicitly **snake_case JSON on both
transports** — specta generates snake_case field names, so the frontend already
sends snake_case nested object fields. Only the http/ wrapper structs (top-level
arg names) differed between transports. Adding camelCase to nested DTOs would
have broken nested deserialization.

## Tests

- New: `tests/users_http_camelcase.rs` — admin session + camelCase
  `{"userNew": {...}}` → **200**; old snake `{"user_new": {...}}` → **422**.
  (No role-check middleware exists; auth runs inside the handler body, after the
  `Json` extractor — so a deserialization failure surfaces as 422 before
  200/401/403. An authenticated admin session is created programmatically.)
- Updated 2 existing HTTP tests that sent now-invalid snake wrapper keys:
  - `tests/role_endpoint_matrix.rs`: `"user_new"` → `"userNew"` (cases 6/7 assert
    exactly 403 — would have become 422 after the fix).
  - `tests/acts_http_smoke.rs`: `"act_id"` → `"actId"`.
- `tests/devices_autocomplete.rs` uses the service directly (not HTTP) → unaffected.

## Verification

- `cargo test -p trackly-app`: **293 passed, 0 failed** (50 test binaries).
- `cargo clippy -p trackly-app --tests`: no new warnings (2 pre-existing,
  unrelated).

Closes the hidden gap before UAT #5/#6 (browser HTTPS access) in
`.planning/phases/05-auth-server-mode/05-VERIFICATION.md`.

## Note (process)

A ~36-min apparent "hang" during full-suite runs was self-inflicted: two
concurrent `cargo test` invocations contended on the target/ build lock. Running
a single invocation completes the suite in ~2 min. (See decision log: prefer one
cargo invocation at a time.)

## Files NOT touched (out of scope)

ui/* — desktop fix (`user_new` → `userNew` in FirstRunWizard.svelte,
UsersPage.svelte) was done separately and is already correct.
