---
phase: 24-base-components
plan: 09
subsystem: ui
tags: [svelte5, scss, badge, css-selectors, gap-closure]

requires:
  - phase: 24-base-components (plans 01-08)
    provides: Badge.svelte opt-in appearance matrix (soft/solid/dot/count) across 5 tones, tokens layer
provides:
  - Tone-correct count-appearance CSS for success/warning/danger tones on Badge (CMP-03 gap closed)
affects: [25-tables-dropdown, 26-window-dashboard-devices, 27-window-acts-cartridges-printers, 28-window-requests-reports-settings-users, 29-window-login-employee]

tech-stack:
  added: []
  patterns:
    - "Badge.svelte's opt-in appearance matrix nests all 4 appearance variants (&.badge-m-soft/&.badge-m-solid/&.badge-m-dot/&.badge-m-count) inside each tone block for consistency — the accent tone's older flat .badge-m-accent.badge-m-count selector was pre-existing and untouched, but success/warning/danger now follow the nested-& convention"

key-files:
  created: []
  modified:
    - ui/src/lib/components/Badge.svelte

key-decisions:
  - "Added &.badge-m-count as a nested rule inside .badge-m-success/.badge-m-warning/.badge-m-danger (matching the existing &.badge-m-soft/&.badge-m-solid/&.badge-m-dot nesting already present in those same blocks), rather than adding new flat .badge-m-X.badge-m-count top-level selectors like the pre-existing accent one — same computed specificity and output, cleaner source structure"
  - "Left .badge-m-neutral, .badge-m-accent, the base .badge-m-count, and .badge-m-accent.badge-m-count completely untouched, per plan constraint — verified byte-identical via diff of the untouched block ranges"

requirements-completed: [CMP-03]

duration: 8min
completed: 2026-07-18
---

# Phase 24 Plan 09: Badge count-tone CSS gap-closure Summary

**Added tone-correct `&.badge-m-count` CSS rules inside `.badge-m-success`/`.badge-m-warning`/`.badge-m-danger`, closing the confirmed CMP-03 blocker where count-appearance badges fell through to neutral grey for non-accent tones.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-07-18T~15:22:00Z (approx.)
- **Completed:** 2026-07-18T~15:30:00Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- `.badge-m-success`, `.badge-m-warning`, `.badge-m-danger` each gained a `&.badge-m-count` nested rule, modeled property-for-property on the existing `.badge-m-accent.badge-m-count` override (`background`/`color` from the tone's soft/text tokens, `border: 1px solid var(--tr-{tone})`, `border-radius: 11px`, `padding: 0 9px`, `height: 20px`, `font-size: 11px`)
- Rebuilt `ui/dist` and confirmed in the compiled CSS that all three new tone-count rules are present with the correct tone-specific tokens (`--tr-success-soft`/`--tr-success-text`, `--tr-warning-soft`/`--tr-warning-text`, `--tr-danger-soft`/`--tr-danger-text`), distinct from the neutral fallback
- `node ui/scripts/check-tokens.mjs` (0 violations) and `pnpm --dir ui svelte-check` (0 errors, only pre-existing unrelated warnings) both pass
- Verified the legacy `.badge` rules, `.badge-m-neutral`, `.badge-m-accent`, base `.badge-m-count`, and `.badge-m-accent.badge-m-count` blocks are unchanged — no regression to the 21 existing legacy call-sites or the 2 previously-working count tones

## Task Commits

Each task was committed atomically:

1. **Task 1: Add count-appearance CSS for success/warning/danger tones (CMP-03)** - `ee6f0f8` (fix)
2. **Task 2: Rebuild and verify all 5 tones' count CSS ships correctly** - no commit (verification-only task; `ui/dist` is gitignored, no tracked files changed by the rebuild)

_No TDD — both tasks are `tdd="false"` per plan frontmatter._

## Files Created/Modified
- `ui/src/lib/components/Badge.svelte` - Added `&.badge-m-count { ... }` nested rule to `.badge-m-success`, `.badge-m-warning`, and `.badge-m-danger` blocks (27 lines added, 0 removed)

## Decisions Made
- Nested `&.badge-m-count` inside each tone block (matching the existing `&.badge-m-soft`/`&.badge-m-solid`/`&.badge-m-dot` nesting pattern already used in those same blocks) rather than introducing new top-level flat selectors — SCSS compiles this to the identical `.badge-m-success.badge-m-count { ... }` output as the pre-existing accent override, confirmed in the built CSS.
- Did not touch `.badge-m-neutral`, `.badge-m-accent`, the base `.badge-m-count` fallback, or `.badge-m-accent.badge-m-count` — all four were already correct per the plan's explicit "do not modify" constraint.

## Deviations from Plan

### Auto-fixed Issues

None - plan executed exactly as written for Task 1's code change.

### Note on acceptance-criteria grep literalism (not a deviation, documentation only)

Task 2's acceptance criteria specify `grep -c "badge-m-success\|badge-m-warning\|badge-m-danger" ui/dist/assets/*.css` returning "at least 3." Because Vite's production build emits fully minified single-line CSS, `grep -c` (which counts matching *lines*, not occurrences) returns `1` against the minified file rather than a per-occurrence count. Using `grep -o ... | wc -l` instead (which counts individual matches regardless of line-wrapping) confirms **12** occurrences of the three tone class names in the compiled CSS, and a targeted extraction of the three exact rule bodies (`.badge-m-success.badge-m-count{...}`, `.badge-m-warning.badge-m-count{...}`, `.badge-m-danger.badge-m-count{...}`) confirms all three compiled correctly with their tone-specific tokens. The functional intent of the acceptance criterion (three new tone-count combinations present in the bundle) is fully satisfied; the literal `grep -c` command in the plan just doesn't account for minification. No code change was needed — this is purely a verification-command nuance.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Manual Spot-Check (documented, not automatable)

Per the plan's acceptance criteria, opening the built showcase's Бейджи section and visually confirming `success`/`warning`/`destructive` count badges render in their own tone color (not neutral grey) is a manual/visual step. As a build-time substitute for the live browser check, the compiled CSS was directly inspected and confirmed to contain the exact expected rule bodies:

```css
.badge-m-success.badge-m-count.svelte-dtbgkf{background:var(--tr-success-soft);color:var(--tr-success-text);border:1px solid var(--tr-success);border-radius:11px;padding:0 9px;height:20px;font-size:11px}
.badge-m-warning.badge-m-count.svelte-dtbgkf{background:var(--tr-warning-soft);color:var(--tr-warning-text);border:1px solid var(--tr-warning);border-radius:11px;padding:0 9px;height:20px;font-size:11px}
.badge-m-danger.badge-m-count.svelte-dtbgkf{background:var(--tr-danger-soft);color:var(--tr-danger-text);border:1px solid var(--tr-danger);border-radius:11px;padding:0 9px;height:20px;font-size:11px}
```

These match the tokens and shape used by the pre-existing, known-working `.badge-m-accent.badge-m-count` rule, giving high confidence the visual rendering will be tonally correct. A live-browser visual confirmation in the showcase (`BadgeSection.svelte` lines 34/44/54) remains recommended at end-of-phase human verification per `human_verify_mode: end-of-phase` in config.

## Next Phase Readiness

Badge's `appearance="count"` now renders correctly for all 5 tones (neutral/accent/success/warning/danger), closing the last confirmed BLOCKER gap from 24-VERIFICATION.md for CMP-03. No blockers for continuing Phase 24 or entering Phase 25 (Tables and Dropdown).

---
*Phase: 24-base-components*
*Completed: 2026-07-18*

## Self-Check: PASSED
