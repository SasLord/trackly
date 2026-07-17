# Deferred Items — Phase 23

## Pre-existing Prettier formatting issues (out of scope for 23-02)

**Found during:** Task 2 verification (`pnpm lint` full run) of plan 23-02.

**Issue:** `prettier --check .` (the second `&&` step of `pnpm lint`) fails on 7 files. Confirmed
these failures **pre-date Phase 23 entirely** (verified via `git show b14ace9~1:<file> | prettier
--check --stdin-filepath <file>` on `ActFormBody.svelte` and `acts.ts` — both already fail before
plan 23-01's first commit):

- `src/features/acts/ActFormBody.svelte`
- `src/features/acts/ActFormItemsTable.svelte`
- `src/features/acts/PdfPreviewModal.svelte`
- `src/features/dashboard/ChartWidget.svelte`
- `src/lib/api/acts.ts`
- `src/lib/components/PersonAutocomplete.svelte`
- `src/styles/_tokens.scss` (rewritten in plan 23-01 — inherited the project's existing
  prettier-vs-scss formatting drift, not introduced by 23-01/23-02)

**Why deferred:** Plan 23-02's scope (D-15) was explicitly the 5 pre-existing **eslint** errors
listed in `23-RESEARCH.md` §Pitfall 2 (`no-undef` ×4 + `no-useless-assignment` ×1) — all 5 are now
fixed and confirmed (0 eslint errors). Prettier formatting drift is a separate, unrelated
pre-existing issue not named in the plan's must-haves or acceptance criteria. Per the executor's
scope boundary ("only auto-fix issues directly caused by the current task's changes"), these are
out of scope for 23-02.

**Impact:** `pnpm lint` still exits non-zero today — but the *reason* changed from "5 eslint
errors" to "prettier formatting drift on 7 unrelated files" (both pre-existing). `check-tokens.mjs`
(the actual new gate from 23-02 Task 1) is confirmed working correctly in isolation
(`node scripts/check-tokens.mjs` runs and reports real pre-migration violations, as designed).

**Recommendation:** A future quick/plan should run `prettier --write` on these 7 files (or the
whole `ui/src` tree) as a standalone formatting pass, reviewed separately from any behavioral
change, so `pnpm lint` becomes a fully honest gate for phases 24–30.
