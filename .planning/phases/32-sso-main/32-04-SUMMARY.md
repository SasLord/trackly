---
plan: 32-04
phase: 32-sso-main
status: complete
completed: 2026-08-04
requirements: [SSO-02]
---

# Plan 32-04 Summary — merge-readiness gate + ci-full dry-run PR

## What was done

Fixed the pre-existing merge-blocking rustfmt drift, ran the full workspace
verification gate, and opened the PR that triggers a real `ci-full` run before the
human-gated merge (Plan 05).

## Commits

- `bfb77a0` chore(32-04): fix pre-existing workspace rustfmt drift (14 files + new test file normalized; AST-preserving, zero semantic change)
- `8cb04f3` fix(32-04): use `mem::take` over `mem::replace` with default in config test (clippy `mem_replace_with_default`, surfaced only under `--all-targets`)

## Verification (all local gates green)

| Gate | Command | Result |
|------|---------|--------|
| Format | `cargo fmt --all -- --check` | ✓ exit 0 (was failing on 14 files) |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | ✓ clean (after the `mem::take` fix) |
| Full tests | `cargo test --workspace --no-fail-fast -- --test-threads=1` | ✓ ran to completion through doc-tests, 0 failures (~71 min; pre-existing `auth_remember_cookie` slow but completed under `--test-threads=1`) |
| Phase-32 targeted | config / auth unit / `ad_admin_logins` | ✓ 5 / 5 / 9 |

## PR / ci-full

- Branch pushed: `spike/ad-sso-kerberos` → origin (`fbc8de8..8cb04f3`).
- PR opened: **https://github.com/SasLord/trackly/pull/1** (base `main`).
- Triggered `ci-full` (pull_request) — first `ci-full` run on this branch (3-OS matrix + ProcMon Windows check); previously only `ci-fast` (ubuntu) ran.

## D-12 note

Confirmed RESEARCH.md's premise correction: there is NO `gssapi`/`ntlm` Cargo feature — SPNEGO uses the pure-Rust `sspi` crate, unconditionally compiled. So no feature-gating work was needed; the merge-readiness gate is "fmt/clippy/test green + a real ci-full run," which this plan delivered.

## Handoff to Plan 05 (human-gated)

Plan 05 waits for **ci-full to go green** on PR #1, then merges `spike/ad-sso-kerberos` → `main` and pushes tag `v1.3.0` (three-segment — `release.yml` triggers only on `v*.*.*`). This is an outward, irreversible step requiring explicit human confirmation.
