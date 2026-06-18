---
phase: 8
slug: windows-macos-linux
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-19
---

# Phase 8 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Release-pipeline phase: there is no unit-test framework for CI YAML. Validation
> relies on workflow-dispatch dry-runs, throwaway tags, and artifact inspection.
> See `08-RESEARCH.md` §"Validation Architecture" for full rationale.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | GitHub Actions workflow validation (no unit-test framework for CI YAML) |
| **Config file** | `.github/workflows/release.yml` |
| **Quick run command** | `gh workflow run release.yml` (requires `workflow_dispatch` trigger) |
| **Full suite command** | `git tag v0.0.99-test && git push origin v0.0.99-test` (throwaway tag, delete after) |
| **Estimated runtime** | ~15–30 min per cross-OS matrix run |

---

## Sampling Rate

- **After every task commit:** `actionlint .github/workflows/release.yml` (YAML/syntax lint) where applicable
- **After every plan wave:** `gh workflow run release.yml` via `workflow_dispatch` dry-run (artifacts-only, no Release publish)
- **Before `/gsd-verify-work`:** One full throwaway-tag run produces a complete draft Release with all OS artifacts + SHA256SUMS
- **Max feedback latency:** ~30 min (cross-OS matrix run)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 8-bundle | bundle | 1 | BLD-02 | — | bundler emits per-OS targets | manual | inspect Actions run artifacts | ❌ W0 | ⬜ pending |
| 8-version | bundle | 1 | BLD-02 | — | tag `v1.2.3` → version `1.2.3` in Cargo.toml + tauri.conf.json | smoke | `grep version` after injection step | ❌ W0 | ⬜ pending |
| 8-trigger | release | 2 | BLD-02 | — | workflow fires on `v*.*.*` tag push | smoke | `git tag v0.0.99-test && git push --tags` | ❌ W0 | ⬜ pending |
| 8-checksums | release | 2 | BLD-03 | — | SHA256SUMS aggregates all artifacts, verifies | manual | `sha256sum -c SHA256SUMS` locally | ❌ W0 | ⬜ pending |
| 8-portable | release | 2 | BLD-04 | T-8 (no updater, portable discipline) | ZIP = trackly.exe + portable.txt + README + config.example, no updater, no %APPDATA% writes | manual | unzip + inspect; run on Win10, check no %APPDATA% writes (procmon-check) | ❌ W0 | ⬜ pending |
| 8-readme | readme | 1 | BLD-05 | — | root README.md (RU) with per-OS run, portable, WebView2, server-mode self-signed, SmartScreen/Gatekeeper | smoke | `ls README.md` + content grep | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `.github/workflows/release.yml` — main pipeline file (BLD-02..05)
- [ ] `crates/trackly-app/tauri.conf.json` — `bundle.active: true`, `bundle.icon` expanded (.ico/.icns/.png)
- [ ] `README.md` (root, RU) — BLD-05
- [ ] portable ZIP staging assets — `README` (portable run instructions) + `trackly.config.toml.example`
- [ ] `workflow_dispatch` trigger added alongside `push: tags: v*.*.*` for dry-run validation

*No existing unit-test framework covers the release pipeline — it is a new CI layer. Validation is workflow-run + artifact inspection.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| NSIS installer present + installs on Win10 x64 | BLD-02 | Requires real Windows + GUI install | Download from draft Release, run installer, launch app |
| macOS .dmg mounts + app launches (Gatekeeper override) | BLD-02 | Requires macOS + Gatekeeper interaction | Mount .dmg, right-click→Open, confirm |
| Linux .AppImage + .deb install/run | BLD-02 | Requires Linux desktop | `chmod +x *.AppImage && ./*.AppImage`; `dpkg -i *.deb` |
| Portable ZIP writes data beside .exe, not %APPDATA% | BLD-04 | Requires real Win10 filesystem observation | Run from ZIP, use `tools/procmon-check`, confirm no %APPDATA% writes |
| Draft Release contents complete before publish | BLD-02/03 | Human gate before manual publish | `gh release view --json assets` + visual check |

---

## Validation Sign-Off

- [ ] All tasks have automated verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without verification path
- [ ] Wave 0 covers all MISSING references (release.yml, tauri.conf.json, README, portable assets)
- [ ] No watch-mode flags
- [ ] Feedback latency documented (~30 min for full matrix)
- [ ] `nyquist_compliant: true` set in frontmatter after planner maps all tasks

**Approval:** pending
