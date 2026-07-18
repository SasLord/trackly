---
phase: 24-base-components
plan: 05
subsystem: ui
tags: [svelte5, scss, design-tokens, badge, backward-compat]

# Dependency graph
requires:
  - phase: 24-base-components
    provides: "Plan 01's --tr-accent-text token, consumed by Badge's opt-in accent-soft/accent-solid matrix cells"
provides:
  - "Badge.svelte opt-in appearance prop (soft|solid|dot|count) rendering the full 5-tone x 4-appearance matrix from Badges.dc.html, fully isolated in a badge-m* CSS namespace"
  - "Badge.svelte's default (no-appearance) render path preserved byte-for-byte identical to the pre-plan CSS — accent stays SOLID, default text stays --tr-text-primary, success/warning/destructive keep color-mix soft"
  - "BadgeSection.svelte showcase gallery (self-contained, static demo content) ready for Plan 07 to wire into the showcase route"
affects: [24-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Opt-in prop + fully separate CSS namespace (badge-m* vs badge*) as the backward-compat pattern for extending a call-site-dense primitive without touching any existing call-sites — zero cascade/specificity risk between the two render paths"

key-files:
  created:
    - ui/src/features/showcase/sections/BadgeSection.svelte
  modified:
    - ui/src/lib/components/Badge.svelte

key-decisions:
  - "Default render path reproduces the current file's per-tone CSS verbatim instead of forcing every tone through a uniform 'soft' formula, per this plan's revision note (D-08's actual priority: backward-compat safety over API purity) — accent stays SOLID, default text stays --tr-text-primary"
  - "TONE_MAP (default->neutral, destructive->danger, others pass through) computed via $derived, used only by the opt-in matrix path — the legacy path never references tone or TONE_MAP"
  - "Matrix-path size overrides (badge-m-sm/badge-m-md) ordered before the five tone blocks, which are ordered before badge-m-count, so the count appearance's own height/padding wins over the sm/md size class regardless of size prop value"

patterns-established:
  - "badge-m* class namespace is now the reference shape for any future primitive needing an opt-in appearance/variant matrix alongside a byte-preserved legacy default"

requirements-completed: [CMP-03]

# Metrics
duration: 5min
completed: 2026-07-18
---

# Phase 24 Plan 05: Badge Component Extension (Opt-in Appearance Matrix) + Showcase Section Summary

**Badge.svelte gained an opt-in `appearance` prop covering the full 5-tone x 4-appearance `Badges.dc.html` matrix in a fully separate `badge-m*` CSS namespace, while the no-`appearance` default render path — reached by all 21 existing call-sites — was preserved byte-for-byte identical to the pre-plan file.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-07-18T06:28:53Z
- **Completed:** 2026-07-18T06:32:44Z
- **Tasks:** 2 completed
- **Files modified:** 2

## Accomplishments
- `Badge.svelte`: added `appearance?: 'soft' | 'solid' | 'dot' | 'count'` to `Props` (no default, undefined when unset), a `TONE_MAP` constant + `$derived(TONE_MAP[variant])` mapping the 5 existing `variant` values to the reference's 5 tone names (`default -> neutral`, `destructive -> danger`, others pass through)
- Template branches on `{#if appearance}`: matrix path renders `<span class="badge-m badge-m-{tone} badge-m-{appearance} badge-m-{size}">` (with a leading `.badge-m-dot-marker` span when `appearance === 'dot'`); `{:else}` renders the exact pre-plan markup unedited
- Legacy CSS block (`.badge`, `.badge-md`/`.badge-sm`, `.badge-default`/`.badge-accent`/`.badge-success`/`.badge-warning`/`.badge-destructive`) copied byte-for-byte with zero edits — verified via `diff` against the pre-plan file content
- New `badge-m*` CSS block appended below, covering 5 tone blocks (neutral/accent/success/warning/danger) x soft/solid/dot nested selectors, plus a `badge-m-count` base + `badge-m-accent.badge-m-count` accent-outlined override, plus the shared `badge-m-dot-marker` 7px-circle rule — all values transcribed verbatim from `Badges.dc.html`'s tone table (read via 24-RESEARCH.md's Badge section)
- `BadgeSection.svelte` created: "Бейджи" heading, 5 tone rows (one per `variant` value) x 4 appearance columns, all 20 cells passing `appearance` explicitly to reach the opt-in matrix path, mirroring `ButtonsSection.svelte`'s static-gallery structure

## Task Commits

Each task was committed atomically:

1. **Task 1: Extend Badge.svelte with an opt-in appearance matrix, preserving the default render verbatim (D-08, corrected)** - `e5e762d` (feat)
2. **Task 2: Create BadgeSection.svelte showcase section** - `388a29b` (feat)

**Plan metadata:** committed separately after this summary.

## Files Created/Modified
- `ui/src/lib/components/Badge.svelte` - Added opt-in `appearance` prop, `TONE_MAP`, matrix/legacy template branch, and the new `badge-m*` CSS block; legacy CSS block byte-for-byte unchanged
- `ui/src/features/showcase/sections/BadgeSection.svelte` - New: static showcase gallery, 5 tones x 4 appearances (20 `<Badge>` cells), not yet routed

## Decisions Made
- Reproduced the current file's per-tone CSS verbatim for the default path rather than a uniform "soft" formula, per the plan's explicit revision note — this is the corrected version of D-08 after checker feedback proved `variant="accent"` is currently SOLID (not soft) and is live on 6 screens
- Placed the `badge-m-sm`/`badge-m-md` size-override rules before the tone blocks and `badge-m-count` in source order, so `badge-m-count`'s own height/padding (18px/20px) always wins over the sm/md size class regardless of which `size` prop value is passed — avoids a CSS specificity fight between two same-specificity single-class selectors
- `BadgeSection.svelte` written as fully explicit static markup (5 tone-row blocks x 4 literal `<Badge>` instances each), matching `ButtonsSection.svelte`'s established showcase pattern and the plan's literal-string acceptance greps (`appearance="soft"` etc.)

## Deviations from Plan

None - plan executed exactly as written. `prettier --write` added a trailing comma to the new `TONE_MAP` object literal (cosmetic, part of the project's existing Prettier config) — no functional or CSS change.

### Note on plan-text staleness (not a deviation, informational only)

The plan's top-level `<verification>` section states `grep -rn "<Badge[ >]" ui/src | wc -l` "still returns 21" — this text is duplicated from Task 1's own acceptance criteria, which was correct when Task 1 ran (before `BadgeSection.svelte` existed). After Task 2 adds 20 new `<Badge appearance="...">` cells, the repo-wide grep now returns 41 (21 original + 20 new showcase cells), which is expected and intentional — Task 2's own acceptance criteria (`grep -c "<Badge" .../BadgeSection.svelte` = 20) is the authoritative check for the new file, and `git diff --stat HEAD~2 -- ui/src/features | grep -v BadgeSection` confirms zero changes to the original 21 call-site files.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `Badge.svelte` now exposes the full 5-tone x 4-appearance opt-in matrix from `Badges.dc.html` while every existing `<Badge variant="...">` call-site across `ui/src/features/*` renders identically to before this plan (verified: `git diff --stat HEAD~2 -- ui/src/features` shows only the new `BadgeSection.svelte` file, zero changes to the 21 original files)
- `BadgeSection.svelte` compiles standalone (`svelte-check`: 0 errors), ready for Plan 07 to import into the showcase page assembly
- No blockers for Wave 1 remaining plan (24-06) or Plan 07

---
*Phase: 24-base-components*
*Completed: 2026-07-18*

## Self-Check: PASSED

All 2 files verified present on disk (Badge.svelte, BadgeSection.svelte); both commits (e5e762d, 388a29b) verified in git log.
