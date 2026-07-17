---
phase: 24
slug: base-components
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-07-18
---

# Phase 24 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None (no vitest/playwright) — frontend gates are static checks + manual showcase (D-01) |
| **Config file** | `ui/eslint.config.js`, `ui/tsconfig.json` (svelte-check); token gate `ui/scripts/check-tokens.mjs` |
| **Quick run command** | `pnpm --dir ui lint` |
| **Full suite command** | `pnpm --dir ui lint && pnpm --dir ui check` (svelte-check) + `node ui/scripts/check-tokens.mjs` |
| **Estimated runtime** | ~30–60 seconds |

---

## Sampling Rate

- **After every task commit:** Run `pnpm --dir ui lint`
- **After every plan wave:** Run full suite (`lint` + `svelte-check` + token gate)
- **Before `/gsd-verify-work`:** Full suite green AND `pnpm --dir ui build` succeeds (server-mode UAT needs fresh `ui/dist`)
- **Max feedback latency:** ~60 seconds

---

## Per-Task Verification Map

> Automated coverage for this phase is structural (component exists, lint/svelte-check/token-gate pass). Behavioral state/variant coverage is visual via the admin-only showcase (D-01) — see Manual-Only Verifications. The Nyquist auditor fills exact task IDs after plans exist.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 24-XX-XX | XX | X | CMP-01..05 | — | N/A (no security-sensitive behavior — presentational primitives) | static | `pnpm --dir ui lint && pnpm --dir ui check && node ui/scripts/check-tokens.mjs` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- No test framework is being installed this phase (deliberate — showcase replaces storybook, per D-01 / CONTEXT code_context).
- Static gates already exist: `pnpm --dir ui lint`, `pnpm --dir ui check`, `node ui/scripts/check-tokens.mjs`.

*Existing static infrastructure (lint + svelte-check + token gate) covers all automated verification for this phase.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Button: 5 variants × 2 sizes × 6 states visually distinct | CMP-01 | No screenshot-diff harness; pixel fidelity to `Buttons.dc` is a human judgment | Log in as admin → open showcase route → verify every Button cell against `Buttons.dc.html` |
| Input/Select/Textarea/Checkbox/Radio states (normal/focus/error/disabled) | CMP-02 | Same — visual state fidelity to `Fields.dc` | Showcase Fields section → tab through focus rings, toggle error/disabled |
| Badge: 5 tones × 4 appearances (soft/solid/dot/count) | CMP-03 | Visual tone/appearance grid vs `Badges.dc` | Showcase Badge grid; verify 15/21 existing call-sites unchanged (D-08) |
| Tabs: underline (counters + active underline) AND segmented | CMP-04 | Visual + interaction fidelity to `Tabs.dc` | Showcase Tabs section; switch active tab, confirm counter badges |
| Modal: overlay + header + body + footer, elev-3, radius 12px | CMP-05 | Visual fidelity to `Modal.dc` | Showcase Modal trigger; confirm overlay token, shadow, radius |
| Theme-switch transition suppression (D-09) | CMP-01..05 | Frame-timing behavior not statically assertable | Toggle theme rapidly; confirm no color "smear" on primitives |

---

## Validation Sign-Off

- [ ] All tasks have a static automated gate (lint/svelte-check/token) OR a listed manual verification
- [ ] Sampling continuity: no 3 consecutive tasks without an automated static gate
- [ ] Wave 0 covers all MISSING references (N/A — no framework install)
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
