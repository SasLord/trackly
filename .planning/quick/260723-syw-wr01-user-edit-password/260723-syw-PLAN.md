---
quick_id: 260723-syw
slug: wr01-user-edit-password
description: "Fix WR-01 — password change on user EDIT is a silent no-op"
status: in-progress
created: 2026-07-23
must_haves:
  truths:
    - "UserPatch carries an optional password field (None/empty = no change)."
    - "AuthService::update_user hashes a non-empty new password via argon2id and writes password_hash atomically in the same UPDATE that bumps version."
    - "UsersPage.handleSave forwards data.password into the patch only when non-empty."
    - "A non-empty password shorter than 8 chars is rejected with a Validation error (field=password)."
    - "bindings.ts UserPatch includes the password field."
    - "A Rust test proves: after edit with a new password, login with the new password succeeds and the old password fails; an empty-password edit leaves the password unchanged."
  artifacts:
    - crates/trackly-app/src/dto/auth.rs
    - crates/trackly-app/src/services/auth.rs
    - crates/trackly-app/tests/users_crud.rs
    - ui/src/features/users/UsersPage.svelte
    - ui/src/bindings.ts
  key_links:
    - crates/trackly-app/src/dto/auth.rs:73
    - crates/trackly-app/src/services/auth.rs:1214
    - ui/src/features/users/UsersPage.svelte:62
---

# Quick Task 260723-syw: Fix WR-01 — silent password no-op on user edit

## Problem (WR-01, Phase 28 code review)

In edit mode `UserFormModal` collects and validates «Новый пароль», but
`UsersPage.handleSave` never forwards `data.password` into the `UserPatch`
(the DTO has no password field). An admin enters a new password, sees the
«Пользователь обновлён» toast, but the DB `password_hash` is unchanged —
a silent no-op on a security action. Both transports (Tauri `users_update`
and HTTP `/api/v1/users_update`) funnel through `AuthService::update_user`,
so the fix lives in the shared service + DTO.

Backend contract check: `update_user` (services/auth.rs:1214) does a single
atomic UPDATE with COALESCE and has **no** password path. An admin
`reset_password` exists (auth.rs:1486) but is HTTP-only and unused by the
edit form. Chosen fix mirrors `create_user`'s argon2id path inside the
existing atomic UPDATE so the edit stays a single versioned write.

## Tasks

### T1 — Backend: optional password on UserPatch + hashing path in update_user

**Files:** `crates/trackly-app/src/dto/auth.rs`, `crates/trackly-app/src/services/auth.rs`

- Add `pub password: Option<String>` to `UserPatch` (doc: None/empty = no
  change; non-empty hashed via argon2id like create). `Option<_>` fields
  deserialize to `None` when absent, so existing HTTP/Tauri payloads without
  `password` keep working.
- In `update_user`, before the writer closure: if `patch.password` is
  `Some(non-empty)`, validate `len >= 8` (else `AppError::Validation {
  field: "password", .. }`, message identical to create), then hash via
  `tokio::task::spawn_blocking(move || hash_password(&Secret::new(..)))`
  (CPU-bound, off the writer thread — same pattern as `create_user`).
  Empty string or `None` → `new_password_hash = None`.
- Add `password_hash = COALESCE(?, password_hash)` to the atomic UPDATE and
  bind `new_password_hash`; shift the `id`/`version` param indices.

**Verify:** `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test users_crud` (one cargo test at a time).
**Done:** UserPatch has password; update_user hashes+writes it atomically; validation rejects <8 non-empty.

### T2 — Test: password change on edit

**Files:** `crates/trackly-app/tests/users_crud.rs`

- Add `users_update_password_change` test: create user (2nd admin as keeper),
  edit with a new 8+ char password, assert login with new password succeeds
  and login with old password fails (`Unauthorized`); then edit with empty
  password and assert login still works with the (unchanged) new password.
- Update the 6 existing `UserPatch { .. }` struct literals to add
  `password: None` (compile fix for the new field).

**Verify:** `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test users_crud`.
**Done:** new test passes; existing users_crud tests still green.

### T3 — Frontend: forward password + regenerate bindings

**Files:** `ui/src/features/users/UsersPage.svelte`, `ui/src/bindings.ts`

- In `handleSave` edit branch, add `password: data.password ? data.password : null`
  to the patch object.
- Regenerate `ui/src/bindings.ts` via `cargo test --test export_bindings`
  (the `pnpm --dir ui build` prebuild hook), confirming `UserPatch` gains the
  `password` field.

**Verify:** `pnpm --dir ui build` succeeds; `grep password ui/src/bindings.ts` shows UserPatch.password; `pnpm --dir ui check` (svelte-check) passes.
**Done:** edit form sends password; bindings in sync; frontend type-checks.
