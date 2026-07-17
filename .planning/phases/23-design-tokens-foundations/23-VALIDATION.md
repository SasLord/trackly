---
phase: 23
slug: design-tokens-foundations
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-07-17
---

# Phase 23 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `23-RESEARCH.md` §Validation Architecture. Values and maps are locked by `23-UI-SPEC.md`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None for frontend — no vitest, no playwright, no jest, no stylelint (confirmed in `ui/package.json`). This phase creates its own static gates. Backend `cargo test` is untouched (frontend-only phase). |
| **Config file** | none — this phase creates `ui/scripts/check-tokens.mjs` and `ui/scripts/verify-value-map.mjs` (zero new dev dependencies, `node:fs` / `node:child_process` only) |
| **Quick run command** | `node ui/scripts/check-tokens.mjs` |
| **Full suite command** | `cd ui && pnpm svelte-check && node scripts/check-tokens.mjs` |
| **Estimated runtime** | ~15–25 seconds (svelte-check dominates; check-tokens.mjs is a single-pass read of `ui/src`) |

> **Do NOT gate on `pnpm lint` as a whole.** `pnpm lint` is already RED on `main` with 5
> pre-existing eslint errors unrelated to tokens (RESEARCH §Pitfall 2). `svelte-check` is
> clean (0 errors, 48 warnings) and IS a valid baseline gate.

---

## Sampling Rate

- **After every task commit:** `node ui/scripts/check-tokens.mjs` — run only the rules applicable to the token families already migrated at that point. Before the `_tokens.scss` plan lands, the gate is meaningless and must not be run.
- **After every plan wave:** `pnpm svelte-check` + full `check-tokens.mjs`. For the space/radius wave additionally `node ui/scripts/verify-value-map.mjs <base-ref>` against that wave's diff.
- **Before `/gsd-verify-work`:** all static gates green, AND the explicit "not automatically provable" checklist (below) handed to the user — never silently counted as done.
- **Max feedback latency:** ~25 seconds

---

## Per-Task Verification Map

Task IDs are assigned by the planner. This map binds each phase requirement to its automated
command; the planner MUST attach the matching command to every task touching that family.

| Requirement | Wave | Test Type | Automated Command | File Exists | Status |
|-------------|------|-----------|-------------------|-------------|--------|
| DS-01 (colour on `--tr-*`, no hardcoded hex) | after gate plan | static-grep | `node ui/scripts/check-tokens.mjs` (rules 1+2) | ❌ W0 — created this phase | ⬜ pending |
| DS-02 (theme switch, no artefacts) | — | manual UAT | — (not automatable, D-09) | N/A | ⬜ pending |
| DS-03 (9-level type scale + mono on identifiers) | after typography plan | static-grep (mono) + manual (visual hierarchy) | `git grep -n 'class="tr-mono"' ui/src` | ❌ W0 | ⬜ pending |
| DS-04 (space/radius migrated by value, no layout shift) | after space/radius plan | git-diff verifier | `node ui/scripts/verify-value-map.mjs <base-ref>` | ❌ W0 — created this phase | ⬜ pending |
| QA-01 (undefined-token bugs fixed, incl. new `--shadow-md`) | after colour plan | static-grep | `git grep -n -- '--font-size-sm\|--radius-lg\|--shadow-md' ui/src` → 0 matches | N/A — grep, not a file | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

**Closed-world existence check (additional, part of `check-tokens.mjs`):** 0 `var(--tr-*)`
references to names not defined in `_tokens.scss`. This is the only automated catch for typos
in the new layer — CSS custom properties resolve to nothing *silently*, and the build never
sees it.

---

## Wave 0 Requirements

- [ ] `ui/scripts/check-tokens.mjs` — does not exist; created this phase. Three rules: (1) old token names in `ui/src` excluding the D-02 layout constants, (2) hex literals inside `<style>` blocks of .svelte files, (3) closed-world `--tr-*` existence.
- [ ] `ui/scripts/verify-value-map.mjs` — does not exist; created this phase. Parses the space/radius migration diff and asserts every replacement is value-preserving per the UI-SPEC value map, with the `--radius-sm` split (D-07) as the single allowed exception.
- [ ] Explicit hand-off checklist of what is physically not provable automatically (DS-02 visual, DS-03 visual hierarchy, the toast/modal-title mono grey area) — must reach `/gsd-verify-work`, not be silently marked covered.

*No existing frontend test infrastructure covers any of this — confirmed by `package.json` and by running both commands directly.*

---

## Manual-Only Verifications

Hard constraint from D-09 (locked): the executor does NOT launch a browser. Most of the UI is
behind login and needs a backend with real data. This is architectural, not a tooling gap.

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Theme switch shows no artefacts (flash, unstyled surfaces, unreadable text) | DS-02 | Requires a real render with data behind login | Both themes, 3–4 dense screens minimum: Устройства, форма акта, Настройки. Run `pnpm --dir ui build` first if checking via LAN browser — server mode serves `ui/dist`, and `cargo tauri dev` only hot-reloads the desktop webview. |
| No layout shift after space/radius migration | DS-04 | `verify-value-map.mjs` proves the *replacement* is logically value-preserving, not how the cascade renders it | Before/after comparison on the same dense screens. Surface inversion (`--tr-bg` #eef1f6, `--tr-surface` #ffffff) is INTENTIONAL (D-10) — not a migration defect. |
| Typography level looks right per text block | DS-03 (visual part) | Level choice is locked by UI-SPEC; "looks correct" is a visual judgement | Spot-check headings/body/captions against the 9-level scale on the same screens. |
| Contrast lost to surface inversion (white-on-white) | DS-02 / D-11 | Only visible in a render | Fix pointwise via `--tr-border`, `--tr-elev-1` or `--tr-surface-sunken` (D-11). Do NOT re-triage every surface call-site by meaning — that is phases 24–28. |

---

## Validation Sign-Off

- [ ] All tasks carry an automated verify command or an explicit Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without an automated verify
- [ ] Wave 0 covers both missing scripts (`check-tokens.mjs`, `verify-value-map.mjs`)
- [ ] No watch-mode flags
- [ ] Feedback latency < 25s
- [ ] Manual-only items handed to `/gsd-verify-work` as an explicit checklist
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
