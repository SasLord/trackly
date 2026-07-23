---
quick_id: 260723-syw
slug: wr01-user-edit-password
status: complete
date: 2026-07-23
commits:
  - a30b360  # fix(auth): rotate password on user edit (WR-01)
  - c4df18c  # fix(users): forward new password from edit form (WR-01)
---

# Quick Task 260723-syw — Summary

## What / Why

Fixed **WR-01** from the Phase 28 code review (`28-REVIEW.md`): editing a user
and typing a new password was a **silent no-op**. `UserFormModal` collected and
validated «Новый пароль», but `UsersPage.handleSave` never forwarded it — the
`UserPatch` DTO had no password field. The admin saw the «Пользователь
обновлён» success toast while the stored `password_hash` stayed unchanged: a
misleading failure on a security action.

## Backend contract check (finding)

`AuthService::update_user` did a single atomic COALESCE-based `UPDATE` with no
password path. An admin `reset_password` already existed (argon2id) but is
HTTP-only and unused by the edit form. Chosen fix (per the review's WR-01
guidance): add the password to `UserPatch` and hash it **inside** the existing
atomic, version-bumping UPDATE so the edit stays one write and both transports
(Tauri `users_update` + HTTP `/api/v1/users_update`, which share the service)
are fixed at once. The HTTP handler already forwards `patch` verbatim — no
change there.

## Changes

- **`dto/auth.rs`** — `UserPatch` gains `#[serde(default)] pub password:
  Option<String>` (None/absent/empty = no change; non-empty hashed).
- **`services/auth.rs`** — `update_user` now, before the writer closure,
  validates a non-empty new password (`len >= 8`, same message as create) and
  hashes it via `spawn_blocking(hash_password)` (argon2id, off the writer
  thread — mirrors `create_user`). The UPDATE gained
  `password_hash = COALESCE(?, password_hash)`; empty/None ⇒ no rotation.
- **`tests/users_crud.rs`** — new `users_update_password_change` (rotation
  works, old password rejected with `Unauthorized`, empty-password edit leaves
  the credential intact while other fields still apply, too-short password ⇒
  `Validation{field:"password"}`). Updated the 7 existing `UserPatch` literals
  for the new field.
- **`ui/.../UsersPage.svelte`** — `handleSave` edit branch adds
  `password: data.password ? data.password : null`.
- **`ui/src/bindings.ts`** (gitignored, regenerated via `prebuild`) — `UserPatch`
  now carries `password?: string | null`.

## Verification

- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test users_crud` → **8/8 pass** (incl. new test).
- `cargo test -p trackly-app --test export_bindings` → pass; `UserPatch.password` present in `bindings.ts`.
- `cargo clippy -p trackly-app --tests` → clean (no warnings).
- `pnpm svelte-check` → **0 errors** (48 pre-existing warnings, none in touched files).
- `pnpm build` → succeeds; `ui/dist` rebuilt.

## Notes

- `ui/src/bindings.ts` and `ui/dist` are gitignored build artifacts regenerated
  by the `prebuild` hook (`cargo test --test export_bindings`) — not committed.
- Follow-up (optional, out of scope): `reset_password` remains HTTP-only and now
  redundant with this path for the edit form; could be Tauri-wired or removed if
  no other caller needs it.
