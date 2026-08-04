---
phase: 33
slug: print-preview-polish
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-08-04
---

# Phase 33 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust: `cargo test` (existing, `crates/trackly-app/tests/`). Frontend: **none** — `ui/` has no Vitest/Jest (verified in RESEARCH.md); existing gates are `svelte-check` + zero-dependency Node lint scripts in `ui/scripts/`. |
| **Config file** | Rust: `crates/trackly-app/Cargo.toml`. Frontend: `ui/package.json` (`lint`, `check` scripts). |
| **Quick run command** | `pnpm --dir ui check` (svelte-check + lint) |
| **Full suite command** | `cargo test -p trackly-app` (needs `TRACKLY_AD_MOCK` / `TRACKLY_SNMP_MOCK` env + a real `pnpm --dir ui build` output in `ui/dist`) |
| **Estimated runtime** | ~30 s frontend, ~2–4 min Rust |

**Planner note:** do NOT introduce a frontend test framework for this phase unless a plan
explicitly justifies it — that is a stack decision beyond the phase scope. Prefer the existing
Rust integration-test location (precedent: `crates/trackly-app/tests/html_act_render.rs`) for
D-13's structural `@page`-parity assertion, since the artifact under test is a Rust-crate asset.

---

## Sampling Rate

- **After every task commit:** `pnpm --dir ui check`
- **After every plan wave:** `cargo test -p trackly-app` (with mock env vars set)
- **Before `/gsd-verify-work`:** both green
- **Max feedback latency:** 240 seconds

---

## Per-Task Verification Map

*To be filled by the planner — one row per task, mapped to PRV-01/PRV-02/PRV-03 and to the
threat model entry for D-14 (CSP change).*

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| — | — | — | — | — | — | — | — | — | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] No new test framework — existing Rust `cargo test` + `svelte-check` cover what is
      automatable here.
- [ ] `crates/trackly-app/tests/` — new integration test asserting all three shipped templates
      declare identical `@page` `size` and `margin` (D-13).

---

## Manual-Only Verifications

Visual and print fidelity cannot be proven by text-extraction tests — a recorded project lesson.
These require a real render and human eyes.

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Лист A4 на сероватой подложке, поля видны | PRV-01, PRV-02 | Визуальная композиция; никакой строковый assert её не докажет | Открыть предпросмотр акта в desktop-режиме и в LAN-браузере, сверить обе темы |
| Превью совпадает с бумагой | PRV-03 | Требует реальной печати/Save-as-PDF при дефолтных настройках диалога | Напечатать длинный акт (N≥2 устройств) и длинный отчёт; сверить число страниц и точки разрыва с превью |
| Paged.js грузится в LAN-режиме (CSP) | PRV-01 | Проявляется только под реальным axum-сервером с CSP-заголовком, не в dev-сборке | `pnpm --dir ui build`, поднять server mode, открыть превью в браузере, проверить консоль на CSP-ошибки |
| Fit-to-width на узком окне | PRV-01 | Зависит от реальной ширины вьюпорта | Сузить окно/открыть в LAN-браузере на ноутбуке — горизонтального скролла быть не должно |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or are listed under Manual-Only above
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 240s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
