---
quick_id: 260614-60l
slug: http-camelcase-payloads
date: 2026-06-14
---

# Quick Task: HTTP transport camelCase payload parity (S-5)

## Problem

Latent bug in browser/HTTP transport (server mode), invisible on desktop.

Convention S-5 (ui/src/lib/api/acts.ts:3-5): frontend `apiCall(...)` args are
camelCase; for the Tauri transport tauri-specta auto-converts to snake_case Rust
params. Works on desktop.

In browser mode `apiCall` (ui/src/lib/api/client.ts) does
`fetch('/api/v1/<name>', { body: JSON.stringify(args) })` — sends the same
camelCase keys verbatim. But axum payload structs in crates/trackly-app/src/http/*.rs
deserialize snake_case serde fields WITHOUT `#[serde(rename_all = "camelCase")]`,
so every endpoint with a multi-word top-level arg breaks over HTTP (422).

Affected multi-word top-level wrapper keys:
- users.rs CreatePayload.user_new, ResetPasswordPayload.{user_id,new_password}
- acts.rs ReturnPayload.act_id, RenderPdfPayload.act_id,
  RenderAcceptancePdfPayload.{device_id,giver_name,receiver_name,date_utc}
- devices.rs AutocompletePayload.{ctx_name,ctx_status_id,status_in}
- templates.rs RenderPreviewPayload.sample_act_id

## Key findings from codebase analysis

1. Nested DTOs (UserNew, ActCreateDto, DeviceNew…) are **snake_case on BOTH
   transports** (dto/auth.rs header: "snake_case JSON"; specta generates
   snake_case field names → frontend sends snake_case nested fields). So nested
   DTOs must **NOT** get camelCase — only the http/ wrapper structs (the
   top-level arg names) differ between transports.
2. No role-check middleware: auth/role checks run **inside** handler bodies via
   `session_identity()`, AFTER the `Json` extractor. So JSON deserialization
   (422) happens BEFORE 401/403. Consequence: changing a wrapper key breaks any
   existing test that sends the old snake key and asserts a non-422 status.
3. Existing tests sending affected snake keys over HTTP:
   - role_endpoint_matrix.rs:216 `"user_new"` (cases 6/7 assert exactly 403 →
     would become 422) → must update to `"userNew"`.
   - acts_http_smoke.rs:213 `"act_id"` (asserts 200) → must update to `"actId"`.
   - devices_autocomplete.rs uses the service directly (not HTTP) → unaffected.
4. Only `StatusPayload {}` (empty) derives Serialize among payloads; it is
   skipped (empty body). `SessionIdentity` is session storage, not a request
   payload → not touched.

## Approach

Add `#[serde(rename_all = "camelCase")]` to all axum request `*Payload`
deserialize structs in http/ (acts, auth, cartridges, devices, fs_helpers,
settings, templates, users). Single-word keys (filter, payload, patch, req, id,
version, prefix, login, password, path…) are unaffected (no-op); uniform
application future-proofs new multi-word fields. Skip empty `StatusPayload`.

Update the two existing HTTP tests to camelCase keys. Add a new strict
integration test (users_http_camelcase.rs): admin session + camelCase
`{"userNew": {...}}` → 200; old snake `{"user_new": {...}}` → 422.

## Verification
- cargo test -p trackly-app passes.
- Closes the hidden gap before UAT #5/#6 (browser HTTPS access) in
  .planning/phases/05-auth-server-mode/05-VERIFICATION.md.

## Out of scope
- ui/* files (desktop fix user_new→userNew already done separately).
- Response-body camelCase parity (separate concern).
