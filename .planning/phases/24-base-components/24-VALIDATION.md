---
phase: 24
slug: base-components
status: approved
nyquist_compliant: true
wave_0_complete: true
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

> Automated coverage for this phase is structural (component exists, lint/svelte-check/token-gate pass). Behavioral state/variant coverage is visual via the admin-only showcase (D-01) — see Manual-Only Verifications. Every `auto` task across the 7 plans carries a fast static `<automated>` gate; no watch-mode/E2E flags, no sampling gaps (plan-checker confirmed Dimension 8 passes).

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 24-01-* | 01 | 1 | CMP-03 (doc), D-06/D-09 | — | N/A | static | `node ui/scripts/check-tokens.mjs && pnpm --dir ui check` | ✅ | ⬜ pending |
| 24-02-* | 02 | 1 | CMP-01 | — | N/A | static | `pnpm --dir ui check && node ui/scripts/check-tokens.mjs` | ✅ | ⬜ pending |
| 24-03-* | 03 | 1 | CMP-02 | — | N/A | static | `pnpm --dir ui check && node ui/scripts/check-tokens.mjs` | ✅ | ⬜ pending |
| 24-04-* | 04 | 1 | CMP-05 | — | N/A | static | `pnpm --dir ui check && node ui/scripts/check-tokens.mjs` | ✅ | ⬜ pending |
| 24-05-* | 05 | 2 | CMP-03 | T-24-05-01 | Default render preserves current per-tone CSS (0/21 call-sites regress) | static | `grep -A2 '.badge-accent {' && pnpm --dir ui check` | ✅ | ⬜ pending |
| 24-06-* | 06 | 2 | CMP-04 | — | N/A | static | `pnpm --dir ui check && node ui/scripts/check-tokens.mjs` | ✅ | ⬜ pending |
| 24-07-* | 07 | 3 | CMP-01..05 (showcase) | T-24-07-01 | Showcase route admin-gated via existing sidebar-role filter | static + manual UAT | `pnpm --dir ui build` + admin login → `/showcase` | ✅ | ⬜ pending |

### Gap-closure plans (added after 24-VERIFICATION.md found 3 BLOCKER gaps)

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 24-08-T1 | 08 | 1 | CMP-02 (bind:value) | T-24-08-01 | Two-way bind fixes data fidelity; backend validation unaffected | static | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ | ⬜ pending |
| 24-08-T2 | 08 | 1 | D-09 (theme transition) | T-24-08-02 | N/A (CSS selector syntax fix) | build + grep | `pnpm --dir ui build && grep -c ":global(" ui/dist/assets/*.css` | ✅ | ⬜ pending |
| 24-09-T1 | 09 | 1 | CMP-03 (Badge count 5 tones) | T-24-09-01 | Legacy 21 call-sites render byte-identically | static | `node ui/scripts/check-tokens.mjs && pnpm --dir ui svelte-check` | ✅ | ⬜ pending |
| 24-09-T2 | 09 | 1 | CMP-03 | T-24-09-01 | N/A | build + grep | `pnpm --dir ui build && grep -c "badge-m-success\|badge-m-warning\|badge-m-danger" ui/dist/assets/*.css` | ✅ | ⬜ pending |
| 24-10-T1 | 10 | 2 | CMP-05 (CR-03 Modal focus) | T-24-10-01 | Empty-focusable-list guard prevents UX lockout | static | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ | ⬜ pending |
| 24-10-T2 | 10 | 2 | CMP-05 | T-24-10-02 | prevFocus sourced from same-DOM activeElement only | build + manual keyboard walkthrough | `pnpm --dir ui build` | ✅ | ⬜ pending |
| 24-11-T1 | 11 | 3 | CMP-01..05 (checkpoint gate) | T-24-11-02 | Prior auto_advance recorded before mutation; restored in T3 | CLI assertion | `test "$(gsd-sdk query check auto-mode --pick active)" = "false"` | ✅ | ⬜ pending |
| 24-11-T2 | 11 | 3 | CMP-01..05 | T-24-11-01 | /showcase route-gating gap re-affirmed as accepted (deferred) | manual UAT (blocking) | human sign-off vs 5 `.dc.html` refs | ✅ | ⬜ pending |
| 24-11-T3 | 11 | 3 | — (restoration) | T-24-11-02 | Config mutation does not outlive the plan | CLI + git diff | `gsd-sdk query config-get workflow.auto_advance && git diff --stat .planning/config.json` | ✅ | ⬜ pending |

> **24-11-T1 is the gate that makes 24-11-T2 real.** `autonomous: false` and `gate="blocking"` do not prevent
> auto-approval — auto-approval keys on checkpoint TYPE (`checkpoints.md:11`), and `execute-phase.md:963-975`
> has no carve-out for either attribute. The `gate="blocking-human"` value referenced in `12-03-SUMMARY.md`
> does not exist anywhere in the GSD sources. Only flipping `workflow.auto_advance` to `false` (T1) actually
> stops the auto-approval path; T3 restores it.

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

- [x] All tasks have a static automated gate (lint/svelte-check/token) OR a listed manual verification
- [x] Sampling continuity: no 3 consecutive tasks without an automated static gate
- [x] Wave 0 covers all MISSING references (N/A — no framework install)
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-07-18
