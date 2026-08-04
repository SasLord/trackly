---
plan: 32-05
phase: 32-sso-main
status: complete
completed: 2026-08-04
requirements: [SSO-02]
---

# Plan 32-05 Summary — human-gated merge to main + v1.3.0 release

## What was done (human-gated Wave 5)

Confirmed `ci-full` green on PR #1, then merged the SSO work out of the spike line
into `main` and cut the first three-segment release tag.

## Gate: ci-full green (PR #1)

All checks CLEAN on head `079e0ee`:

| Check | Result |
|-------|--------|
| `fmt + clippy + test + ui` (ci-fast) | ✓ pass |
| `matrix (ubuntu-latest)` | ✓ pass |
| `matrix (macos-latest)` | ✓ pass |
| `matrix (windows-latest)` | ✓ pass |
| `procmon (windows-latest)` | ✓ pass |

First-ever `ci-full` run on this branch (3-OS matrix + ProcMon Windows portable check) — the authoritative cross-OS gate.

## Merge + release

- **Merged** PR https://github.com/SasLord/trackly/pull/1 into `main` — merge commit `ab25d4c` ("Merge pull request #1 from SasLord/spike/ad-sso-kerberos"). SSO-01/SSO-03 (Phase 31) + SSO-02 (Phase 32) now on `main`, out of the spike `0.0.x` line.
- **Tagged** `v1.3.0` (annotated) on the merge commit and pushed → triggered `release.yml` (which fires only on three-segment `v*.*.*`; `v1.3` would not build).
- Release workflow: `push | v1.3.0 | in_progress` (building at close time).

## Decisions honored

- D-11: merge after verify, three-segment `v1.3.0` tag. ✓
- D-12 (corrected): no `gssapi`/`ntlm` feature to gate — `sspi` compiled unconditionally; ci-full matrix green on all 3 OSes confirms the merge kept macOS/Linux/Windows green. ✓

## Deviations / notes

- The autonomous chain paused here (Wave 5 is `autonomous: false`); merge + tag executed after the user confirmed and ci-full went green.
- Two pre-existing merge-blockers surfaced by ci-full's `pnpm lint` were fixed under 32-04's merge-readiness umbrella (commit `079e0ee`): Prettier drift in `ActiveDirectorySettings.svelte` and a token-checker false-positive on a hex literal inside a comment in `DeviceContextMenu.svelte` — neither in Phase 32's functional scope.
