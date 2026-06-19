---
phase: 08-windows-macos-linux
plan: "02"
subsystem: release-pipeline
tags: [github-actions, release, tauri, portable, windows, macos, linux, sha256]
dependency_graph:
  requires:
    - 08-01 (portable-zip-staging-files: README-portable.md, trackly.config.toml.example)
    - tauri-bundle-active (from 08-01)
  provides:
    - release-yml-pipeline
    - three-job-release-structure
    - portable-zip-assembly
    - sha256sums-aggregation
  affects:
    - .github/workflows/release.yml
tech_stack:
  added: []
  patterns:
    - "three-job GitHub Actions release pipeline (create-release → build matrix → checksums)"
    - "tauri-apps/tauri-action@v0 with releaseId (race-condition-free)"
    - "perl -0pi cross-platform version injection (Cargo.toml + tauri.conf.json via jq)"
    - "PowerShell Compress-Archive portable ZIP assembly (D-08)"
    - "actions/upload-artifact@v4 merge-multiple for SHA256SUMS aggregation (Pitfall 8)"
key_files:
  created:
    - .github/workflows/release.yml
  modified: []
decisions:
  - "Rust toolchain 1.92 in release.yml (MSRV from Cargo.toml rust-version; ci-full.yml uses 1.88 but release MUST match MSRV)"
  - "perl -0pi over sed -i: BSD sed (macOS runner) incompatible with sed -i without suffix; perl portable on all runners"
  - "context.eventName check in create-release: workflow_dispatch derives tag from inputs.version, not context.ref"
  - "GITHUB_EVENT_NAME check in upload steps: workflow_dispatch GITHUB_REF_NAME = branch name, not tag"
  - "uploadUpdaterJson: false — portable discipline, no updater (D-09)"
  - "releaseDraft: true — manual publish after artifact verification (D-12)"
  - "retryAttempts: 1 — mitigates macOS DMG intermittent bundle_dmg.sh failure (Pitfall 4)"
  - "gh release upload $TAG (tag_name, not release_id) — gh CLI accepts tag, not numeric ID (Open Question 2)"
  - "No i686-pc-windows-msvc, no embedBootstrapper, no webviewInstallMode (D-01, D-07)"
metrics:
  duration: "1 min"
  completed_date: "2026-06-19"
  tasks: 3
  files: 1
---

# Phase 08 Plan 02: Release Pipeline (release.yml) — Summary

**One-liner:** Three-job GitHub Actions release pipeline: create-release (draft + release_id) → build matrix (Windows NSIS + portable ZIP, macOS aarch64 dmg, Linux AppImage+deb, Rust 1.92, perl version injection) → checksums (SHA256SUMS aggregation via merge-multiple).

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1+2 | Create .github/workflows/release.yml (all three jobs combined) | a1f5bb3 | .github/workflows/release.yml |
| 3 | Dry-run pre-checks (automated portion) | — | see "Требует ручной проверки" below |

## Verification Results

| Check | Result |
|-------|--------|
| YAML valid (python yaml.safe_load) | PASS |
| uploadUpdaterJson: false (count >= 1) | PASS (count=2) |
| toolchain: '1.92' | PASS |
| Three jobs: create-release, build, checksums | PASS |
| perl -0pi + context.eventName + GITHUB_EVENT_NAME (count >= 3) | PASS (count=7) |
| workflow_dispatch trigger present | PASS |
| releaseDraft: true | PASS |
| SHA256SUMS + sha256sum + merge-multiple present | PASS |
| i686 / embedBootstrapper / webviewInstallMode absent | PASS (count=0) |
| Matrix: windows-latest --bundles nsis | PASS |
| Matrix: macos-latest --target aarch64-apple-darwin --bundles dmg | PASS |
| Matrix: ubuntu-22.04 --bundles appimage,deb | PASS |
| permissions: contents: write | PASS |
| portable.txt in Assemble portable ZIP step | PASS |
| needs: [create-release, build] in checksums | PASS |

## Deviations from Plan

### Auto-merged Tasks 1 and 2

Task 1 specified jobs `create-release` and `build`; Task 2 specified job `checksums`. Both were implemented in a single Write pass producing the complete `release.yml`. This is not a behavioral deviation — all specified content was created correctly; the two commits were merged into one because the file was written atomically. All Task 1 and Task 2 done criteria are met.

## Требует ручной проверки (отложено)

Task 3 is a `checkpoint:human-verify` task with a `blocking` gate. In auto mode, the automated structural pre-checks were executed (all PASS — see Verification Results above). The following human-required steps were deferred:

| Step | Action | Expected Result |
|------|--------|-----------------|
| A | `git push origin main` (already pushed via commit a1f5bb3 in prior session) | release.yml available on GitHub |
| B | `gh workflow run release.yml --field version=0.1.0-test` (or GitHub UI: Actions → release → Run workflow) | workflow_dispatch dry-run starts |
| C | Monitor run: `gh run watch` | create-release → three build jobs → checksums — all green (~20–35 min) |
| D | Verify draft Release: `gh release list` / `gh release view v0.1.0-test` | Files present: Trackly_*_x64-setup.exe (NSIS), *.dmg, *.AppImage, *.deb, trackly-v0.1.0-test-windows-x64-portable.zip, SHA256SUMS |
| E | Cleanup: `gh release delete v0.1.0-test --yes` | draft deleted |
| F (optional) | Full tag-push test: `git tag v0.0.99-test && git push origin v0.0.99-test`; verify; `gh release delete v0.0.99-test --yes && git tag -d v0.0.99-test && git push origin --delete v0.0.99-test` | push-tag path confirmed |

These items require GitHub Actions runners (real cross-OS builds take ~20–35 min) and cannot be performed from macOS dev without pushing to GitHub. They will be surfaced by phase verification as `human_needed` items.

## Known Stubs

None. The release.yml is complete and functional per all automated checks.

## Threat Flags

No new security-relevant surface beyond the planned threat model (T-08-02-01..07, T-08-02-SC).

Key mitigations in place:
- **T-08-02-01 (Tampering / artifacts):** SHA256SUMS uploaded to Release alongside artifacts.
- **T-08-02-04 (Tampering / VERSION injection):** VERSION extracted only from `GITHUB_REF_NAME#v` (parameter substitution, no eval, no shell expansion of user data); on.push.tags pattern restricts to `v*.*.*`.
- **T-08-02-05 (EoP / updater):** `uploadUpdaterJson: false` explicitly set; `tauri-plugin-updater` not in Cargo.toml.
- **T-08-02-06 (Repudiation / draft):** `releaseDraft: true` — publish only after human review.

## Self-Check

### Файлы существуют:
- .github/workflows/release.yml: checked below

### Коммиты существуют:
- a1f5bb3: checked below
