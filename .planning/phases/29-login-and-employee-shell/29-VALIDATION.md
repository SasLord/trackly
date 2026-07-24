---
phase: 29
slug: login-and-employee-shell
status: approved
nyquist_compliant: false
wave_0_complete: true
created: 2026-07-24
---

# Phase 29 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Reconstructed retroactively (State B) from PLAN/SUMMARY/VERIFICATION artifacts.

**Phase character:** Pure frontend markup/CSS migration onto design-system-v2 primitives.
8 Svelte files touched, **zero `.ts`/`.rs`**; all business logic (auth routing, `validate()`,
`handleSubmit()`, `handleRestoreRequest`, WS/logout) preserved byte-identical (confirmed via
git diff in `29-VERIFICATION.md`). No client-side logic introduced.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None — `ui/` has no unit-test runner (no vitest/jest/playwright). Verification is via static gates. |
| **Config file** | `ui/svelte.config.js`, `ui/tsconfig.json`, `ui/scripts/check-tokens.mjs` |
| **Quick run command** | `pnpm --dir ui svelte-check` |
| **Full suite command** | `pnpm --dir ui lint && pnpm --dir ui svelte-check && pnpm --dir ui build` |
| **Estimated runtime** | ~30–60 seconds |

**Decision (2026-07-24, user-approved):** Do **not** bootstrap vitest + @testing-library/svelte
for this phase. Rationale: (1) the real failure mode in a CSS/markup migration is *visual*
regression — jsdom component tests cannot assert layout/geometry (both UAT-found bugs were
CSS layout regressions); (2) structural prop contracts are already guarded by `svelte-check`
(it caught the missing `autocomplete` prop in 29-02) and token drift by `check-tokens.mjs`;
(3) no client-side business logic exists to unit-test — logic lives in the Rust backend.
Higher-leverage UI automation, if ever wanted, would be Playwright visual-regression, not
jsdom component tests — a separate infrastructure decision, not part of validating Phase 29.

---

## Sampling Rate

- **After every task commit:** `pnpm --dir ui svelte-check` (+ per-task grep acceptance criteria)
- **After every plan wave:** `pnpm --dir ui lint && pnpm --dir ui svelte-check && pnpm --dir ui build`
- **Before `/gsd-verify-work`:** Full static-gate suite green + blocking human-verify UAT
- **Max feedback latency:** ~60 seconds (static gates)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 29-01-01 | 01 | 1 | WIN-10 | T-29-01 | Additive `type` union; 19 consumers unaffected | static (type + grep + build) | `pnpm --dir ui build` + `grep -c "'password'" ui/src/lib/components/Input.svelte` | ✅ | ✅ green |
| 29-01-02 | 01 | 1 | WIN-10 | — | Chrome extraction, no logic | static (type + grep + build) | `pnpm --dir ui build` + grep `style:max-width`/`class:stack` | ✅ | ✅ green |
| 29-01-03 | 01 | 1 | WIN-10 | — | `aria-describedby` computed from `id` (a11y) | static (type + grep) | `pnpm --dir ui svelte-check` + grep `children({ describedBy` / `id="{id}-error"` | ✅ | ✅ green |
| 29-02-01 | 02 | 2 | WIN-10 | T-29-02 | Password masked via native `type="password"`; no value logging; zero bespoke form classes | static (grep + type + build) + human UAT | `pnpm --dir ui svelte-check` + grep sweep (`form-input`/`btn-submit`/… → 0) | ✅ | ✅ green |
| 29-02-02 | 02 | 2 | WIN-10 | — | FirstRunWizard on primitives; logic byte-identical | static (grep + build) + git-diff + human UAT | `pnpm --dir ui build` + grep sweep | ✅ | ✅ green |
| 29-03-01 | 03 | 2 | WIN-10 | — | PendingScreen on AuthShell/Button, zero bespoke classes | static (grep + build) + human UAT | `pnpm --dir ui build` + grep `btn-link`/`login-container` → 0 | ✅ | ✅ green |
| 29-03-02 | 03 | 2 | WIN-10 | — | BlockedScreen 4 branches on AuthShell(stack); `handleRestoreRequest` byte-identical | static (grep + build) + git-diff + human UAT | `pnpm --dir ui build` + grep sweep | ✅ | ✅ green |
| 29-04-01 | 04 | 3 | WIN-11 | — | Header sizing token-driven, no `56px` fallback | static (grep + build) | `pnpm --dir ui build` + `grep -c "56px" ui/src/features/layout/EmployeeLayout.svelte` → 0 | ✅ | ✅ green |
| 29-04-02 | 04 | 3 | WIN-11 | — | Content fills to viewport; header name not collapsed (both themes) | **human visual UAT (blocking)** | manual — see Manual-Only below | N/A | ✅ approved |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure (svelte-check + eslint + prettier + check-tokens.mjs + build) covers all
structural/contract phase requirements. No test-runner install performed (see Test Infrastructure
decision). No Wave 0 test stubs applicable to a no-runner frontend migration.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Visual-language parity of all 4 auth screens with redesigned main app | WIN-10 (SC3) | Visual parity cannot be asserted by static gates or jsdom (no layout/geometry, no visual-regression tooling in project) | Run app (desktop + LAN browser), open Login / FirstRunWizard / Pending / Blocked in both light & dark themes; confirm card chrome, spacing, typography match main app. **Done — approved 29-04 Task 3.** |
| EmployeeLayout content fills to viewport bottom | WIN-11 | CSS layout/geometry; jsdom does not compute layout | Open EmployeeLayout (RequestsPage); confirm list/detail cards reach viewport bottom with correct padding in both themes. **Done — fixed `61feaa7`, re-approved.** |
| EmployeeLayout header user-name not collapsed by ThemeSwitcher | WIN-11 | CSS flex geometry; not observable in jsdom | Confirm header shows full user-name (ellipsis only past `max-width:200px`), ThemeSwitcher bounded. **Done — fixed `e3bee15`, re-approved.** |
| Password field masks input | WIN-10 | Native browser masking behavior via `type="password"` | Visually confirm characters render as dots in Login + FirstRunWizard. **Done — UAT.** |

---

## Validation Sign-Off

- [x] All tasks have static-gate verification or are explicitly manual-only (visual parity)
- [x] Sampling continuity: every code task guarded by svelte-check/build + grep; no automatable gap left uncovered
- [x] Wave 0 covers all MISSING references — N/A (no runner; documented decision)
- [x] No watch-mode flags
- [x] Feedback latency < 60s (static gates)
- [ ] `nyquist_compliant: true` — **not set.** Visual-parity requirements (WIN-10 SC3, WIN-11) are inherently manual-only; no automated test can assert them without visual-regression tooling that the project does not have. Phase is validated PARTIAL: all automatable behaviors green via static gates; visual behaviors covered by completed blocking human UAT.

**Approval:** approved 2026-07-24 (manual-only classification, user-confirmed)
