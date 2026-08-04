---
phase: 33
slug: print-preview-polish
status: approved
nyquist_compliant: true
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

Verified by gsd-plan-checker (2026-08-04): **every task across all four plans carries an
`<automated>` verify command**, no watch-mode flags, no E2E-latency issues. Sampling continuity
is therefore trivially satisfied — hence `nyquist_compliant: true` in the frontmatter.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 33-01-* | 01 | 1 | PRV-01 | — | N/A | typecheck/build | `pnpm --dir ui check` / `pnpm --dir ui build` | ✅ | ⬜ pending |
| 33-02-* | 02 | 2 | PRV-01, PRV-03 | D-14 (CSP `script-src` hash source) | Inline preview bootstrap runs under a hash source, never `'unsafe-inline'`; hash drift is caught by a lint gate rather than silently disabling preview | integration | `node ui/scripts/check-pagedjs-csp-hash.mjs && pnpm --dir ui build && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test security_headers` | ❌ W0 | ⬜ pending |
| 33-02-* (D-13) | 02 | 2 | PRV-02 | — | N/A | integration | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test html_page_parity` | ❌ W0 | ⬜ pending |
| 33-03-* | 03 | 2 | PRV-01, PRV-02 | — | Preview iframe stays `sandbox="allow-scripts"` without `allow-same-origin`; bridge validates `event.source`, not `event.origin` | typecheck | `pnpm --dir ui check` | ✅ | ⬜ pending |
| 33-04-* | 04 | 3 | PRV-03 | — | N/A | typecheck + manual | `pnpm --dir ui check` + Manual-Only table below | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*
*File Exists: ❌ W0 = the test file is created by the plan itself, not a pre-existing fixture.*

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
| **LAN-печать: поля и шрифты применились к `#act-print-root`** | PRV-03 | Отдельный риск, а не часть предыдущей строки. RESEARCH.md Open Question 2 — точная форма аргумента `Polisher.add()` не подтверждена; план 33-04 закладывает fallback (снять теги-обёртку), корректность которого `pnpm --dir ui check` проверить не может | В LAN-браузере нажать «Печать» на акте и на отчёте. В превью печати убедиться, что поля 20mm/15mm применились и шрифт документа не подменился шрифтом приложения. Если поля нулевые или шрифт чужой — сработал неверный вариант `Polisher.add()`, чинить в 33-04 Task 2 |
| Paged.js грузится в LAN-режиме (CSP) | PRV-01 | Проявляется только под реальным axum-сервером с CSP-заголовком, не в dev-сборке | `pnpm --dir ui build`, поднять server mode, открыть превью в браузере, проверить консоль на CSP-ошибки |
| Fit-to-width на узком окне | PRV-01 | Зависит от реальной ширины вьюпорта | Сузить окно/открыть в LAN-браузере на ноутбуке — горизонтального скролла быть не должно |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or are listed under Manual-Only above
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 240s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-08-04 (gsd-plan-checker: 0 blockers, 3 warnings — all three applied)
