---
phase: 37-data-privacy
plan: 04
subsystem: infra
tags: [privacy, git-hooks, ci, gate-activation, completeness-proof]

requires:
  - phase: 37-data-privacy plan 01
    provides: "Cleaned HEAD (14 files) + marker-shape checklist used to populate the production hash list and to derive the completeness proof set"
  - phase: 37-data-privacy plan 02
    provides: "Binary artifacts untracked, PHASE-BRIEF deleted, 3 overlap files scrubbed + their marker-shape checklist lines"
  - phase: 37-data-privacy plan 03
    provides: "scripts/check-privacy.mjs (incl. the --add flow) and check-privacy.selftest.mjs"
provides:
  - "scripts/privacy-tokens.sha256 — production token hash list (19 entries, hashes only, sorted, D-07/D-08 header)"
  - ".githooks/pre-commit — staged-scope gate entry point, fail-closed on missing node"
  - "scripts/setup-hooks.sh — one-line core.hooksPath activation"
  - "CONTRIBUTING.md — developer-facing activation doc (new file)"
  - ".github/workflows/ci-fast.yml — gate step relocated to immediately after Checkout, now calling the new gate"
  - ".planning/phases/37-data-privacy/37-VALIDATION.md — closed out (nyquist_compliant/wave_0_complete true, Sign-Off checked, Per-Task map filled)"
affects: []

tech-stack:
  added: []
  patterns:
    - "Two-point privacy enforcement: an opt-in, bypassable local pre-commit hook (fast feedback) plus a non-bypassable CI step (backstop) running the same single gate script — neither duplicates the other's logic"
    - "core.hooksPath activation kept explicit (scripts/setup-hooks.sh) rather than auto-wired through a package-manager postinstall hook — git config is developer-owned state (D-10)"
    - "Token-list completeness proof against pre-cleanup blobs still reachable in git history: extract blob to session scratchpad → scan with the finished gate → expect exit 1 → delete blob immediately; only path + pass/fail is ever recorded"

key-files:
  created:
    - scripts/privacy-tokens.sha256
    - .githooks/pre-commit
    - scripts/setup-hooks.sh
    - CONTRIBUTING.md
  modified:
    - .github/workflows/ci-fast.yml
    - scripts/check-privacy.mjs
    - scripts/fixtures/privacy/README.md
    - .planning/phases/37-data-privacy/37-VALIDATION.md
  deleted:
    - scripts/check-privacy-requisites.sh

key-decisions:
  - "Added AUTO_SCAN_EXCLUDED_PREFIXES = ['scripts/fixtures/privacy/'] as a NEW, separate, mode-aware constant in check-privacy.mjs rather than extending EXCLUDED_PATH_PREFIXES/EXCLUDED_PATH_EXACT — the gate's own regression fixtures are deliberately-violating synthetic data (an unapproved requisite literal; a .docx outside BINARY_ALLOWLIST) that must keep firing when named explicitly by the self-test, but would otherwise fail every auto-discovery scan (--staged / full HEAD) forever once committed"
  - "The hook-block demonstration uses a mode-1 unapproved requisite literal (synthetic digit strings) instead of the plan's literal instruction to 'reuse a 37-03 fixture marker' — fixture markers are fictional and live only in tokens.fixture.sha256, never in the production hash list, so they cannot be caught in hash mode by design; mode 1 fires independently of the hash list and demonstrates the same blocking behavior without touching a real value"
  - "The ci-fast.yml step passes --hashes scripts/privacy-tokens.sha256 explicitly rather than relying on a built-in default — keeps the gate's fail-closed contract (R7) visible at the call site and matches the pre-commit hook's invocation"

patterns-established:
  - "Provenance rule for interrupted executors: when an executor is cut off mid-plan with uncommitted work, the surviving artifacts' own claims are not evidence — every acceptance check is independently re-run before the work is committed, and any claim that cannot be substantiated is corrected rather than inherited"

requirements-completed: [PRIV-02]

duration: ~2h (incl. orchestrator re-verification after executor interruption)
completed: 26-08-17
---

# Phase 37 Plan 04: Activate the privacy gate Summary

**Switched the gate built in plan 37-03 on against the repository cleaned by plans 37-01/37-02: production hash list populated (19 entries, hashes only), pre-commit hook and CI step wired, the legacy bash gate retired, and the token list proven complete against all 18 pre-cleanup blobs still reachable in git history — zero holes.**

## Performance

- **Duration:** ~2 h (the original executor was terminated mid-Task-3 by a provider weekly usage limit; the orchestrator re-verified every check independently and completed the plan)
- **Tasks:** 3/3 completed
- **Files created:** 4 · **modified:** 4 · **deleted:** 1

## Accomplishments

- Populated `scripts/privacy-tokens.sha256` with 19 token hashes covering all 18 marker-shape checklist lines from `37-01-SUMMARY.md` (14) and `37-02-SUMMARY.md` (4). Every non-comment line matches `^[0-9a-f]{64} [ABC]$`; the file is sorted (D-07) and its header documents the D-08 residual risk and the R7 fail-closed recovery path. Zero plaintext anywhere in the file.
- Wired `.githooks/pre-commit` (mode `100755`): checks `command -v node`, exits 1 with an instructional message when node is absent (D-11 fail-closed), otherwise `exec`s the gate in `--staged` mode against the production hash list.
- Created `scripts/setup-hooks.sh` (mode `100755`) and `CONTRIBUTING.md`, documenting that git never enables a repo's `.githooks/` on its own and that the one-line fix is `./scripts/setup-hooks.sh` (D-10).
- Relocated the `ci-fast.yml` privacy step from after the SPA build to immediately after `Checkout` (line 30, before `Install Rust toolchain` at line 33) and swapped its `run:` to the new gate. `ci-full.yml` has zero privacy-step matches — no duplicate (D-12).
- Deleted `scripts/check-privacy-requisites.sh` (R5) — fully absorbed by mode 1, whose 23-literal equivalence was already proven in plan 37-03.
- **Token-list completeness proof (WARNING 3):** derived the 18-file proof set from the checklist lines themselves (not a hard-coded list), extracted each file's pre-cleanup blob from the commit preceding its cleanup commit into the session scratchpad, re-scanned it with the finished gate, and deleted the blob immediately. **All 18 fired; zero holes.**
- Closed out `37-VALIDATION.md`: `nyquist_compliant: true`, `wave_0_complete: true`, every Sign-Off item checked, a `Plan/Task` column added to the Per-Task Verification Map with real task IDs, and every row's Status set to ✅.

## Token-List Completeness Proof

Proof set = the 14 files with a marker-shape checklist line in `37-01-SUMMARY.md` + the 3 overlap files from `37-02` Task 3 + `PHASE-BRIEF-act-pdf-word-fidelity.md` (deleted wholesale, so no checklist line). Excludes, per plan: 37-02 Task 2's 11 dangling-reference-only files, `.gitignore`, and the 3 binary artifacts (covered by R8, not token hashing). Values are never recorded — only path, whether the gate fired, and the violation count/class.

| Pre-cleanup file | Gate fired | Violations |
|---|---|---|
| `.planning/phases/03-pdf/03-UAT.md` | ✅ | 1 (A) |
| `.planning/phases/34-document-header/34-REVIEW.md` | ✅ | 1 (A) |
| `crates/trackly-app/src/pdf/renderer.rs` | ✅ | 1 (A) |
| `.planning/phases/30-quality-a11y-platform-parity/30-09-SUMMARY.md` | ✅ | 2 (A) |
| `.planning/STATE.md` | ✅ | 3 (B, C) |
| `.planning/quick/260805-lrs-.../260805-lrs-PLAN.md` | ✅ | 3 (B) |
| `.planning/quick/260805-lrs-.../260805-lrs-SUMMARY.md` | ✅ | 1 (B) |
| `.planning/quick/260804-ire-.../260804-ire-PLAN.md` | ✅ | 1 (C) |
| `.planning/quick/260805-edd-.../260805-edd-PLAN.md` | ✅ | 2 (C) |
| `.planning/quick/260805-edd-.../260805-edd-SUMMARY.md` | ✅ | 1 (C) |
| `.planning/quick/260805-gdz-.../260805-gdz-PLAN.md` | ✅ | 1 (C) |
| `.planning/quick/260805-gdz-.../260805-gdz-SUMMARY.md` | ✅ | 1 (C) |
| `.planning/quick/260805-har-.../260805-har-PLAN.md` | ✅ | 1 (C) |
| `.planning/quick/260805-har-.../260805-har-SUMMARY.md` | ✅ | 1 (C) |
| `.planning/phases/15-render-word-fidelity/15-02-PLAN.md` | ✅ | 2 (A) |
| `.planning/phases/15-render-word-fidelity/15-CONTEXT.md` | ✅ | 1 (A) |
| `.planning/phases/15-render-word-fidelity/15-RESEARCH.md` | ✅ | 1 (A) |
| `.planning/PHASE-BRIEF-act-pdf-word-fidelity.md` | ✅ | 6 (A) |

**18/18 fired · 0 holes · 0 extracted blobs surviving** (`ls -A <scratchpad>/proof` empty; `git status --porcelain` before commit showed only this plan's own intended changes).

## Hook Activation Transcript (scratch clone, deleted afterwards)

Commands and exit codes only — no real value was used at any point.

| Check | Command | Result |
|---|---|---|
| D-10 activation | `./scripts/setup-hooks.sh` → `git config core.hooksPath` | `.githooks` |
| Negative control | commit a clean `.rs` file | gate `PASS`, commit created (exit 0) |
| R10 block | commit a `.rs` file with 2 unapproved requisite literals (synthetic digits) | gate `FAIL — 2 нарушений`, **`git commit` exit 1, HEAD unchanged** |
| D-11 fail-closed | run the hook with `node` off `PATH` | instructional message, **exit 1** (not a silent pass) |
| D-11 staged scope | staged blob clean, working tree dirty with a violation | gate `PASS`, commit created — confirms `git show :path`, not the working tree |

## Task Commits

Tasks 1–3 were committed together as the plan's Task 3 action specifies (the hash list and hook are only meaningful once the full green run proves them):

1. **Tasks 1–3: activate privacy gate** — `ebf8416` (chore)

## Files Created/Modified

**Created:** `scripts/privacy-tokens.sha256`, `.githooks/pre-commit` (100755), `scripts/setup-hooks.sh` (100755), `CONTRIBUTING.md`
**Modified:** `.github/workflows/ci-fast.yml`, `scripts/check-privacy.mjs`, `scripts/fixtures/privacy/README.md`, `.planning/phases/37-data-privacy/37-VALIDATION.md`
**Deleted:** `scripts/check-privacy-requisites.sh`

## Decisions Made

See `key-decisions` in frontmatter. The substantive one is the fixture auto-scan exclusion — recorded as a deviation below because it touches R9's letter.

## Deviations from Plan

1. **`AUTO_SCAN_EXCLUDED_PREFIXES` added to `check-privacy.mjs` (Rule 3 — blocking without it).** R9 states "the exclusion list remains the 4 constants from plan 37-03". Those 4 constants *are* untouched (verified by diffing `EXCLUDED_PATH_PREFIXES`/`EXCLUDED_PATH_EXACT` against plan 37-03's committed version — identical), and `.planning/phases/37-data-privacy/` was **not** added to them (D-13 holds; the source carries an explicit comment saying so). But once 37-03's fixtures are committed to HEAD, `allowlist-regression.rs.txt` (an intentionally unapproved requisite literal, C-02 regression) and `binary-regression.docx` (intentionally outside `BINARY_ALLOWLIST`, R8 regression) would fail every auto-discovery run of the gate — permanently red CI and a permanently blocked pre-commit hook. The fix is a separate, narrowly-scoped, mode-aware constant that applies **only** to auto-discovery (`--staged` / full HEAD) and **not** to explicit positional-file invocations, so the self-test's regression assertions still see the violations. It disables no token check for any real repository path. **Residual risk worth a reviewer's attention:** real data placed under `scripts/fixtures/privacy/` would not be caught by an auto-scan.
2. **Hook-block demo uses a mode-1 literal, not a "fixture marker" (Rule 1 — plan defect).** The plan asked to stage "a fictional marker already present in `scripts/privacy-tokens.sha256`'s test coverage (reuse a 37-03 fixture marker)". These are mutually exclusive: fixture markers are fictional and hashed only into `tokens.fixture.sha256`, never into the production list, so hash mode cannot fire on them — and putting one into the production list would be pointless noise. Demonstrated the identical blocking behavior via mode 1 instead, which fires independently of the hash list.
3. **Plan completed by the orchestrator, not the dispatched executor.** The `gsd-executor` agent was terminated mid-Task-3 by a provider weekly usage limit, leaving all of Tasks 1–3's file edits uncommitted and no SUMMARY. Rather than inherit its claims, every acceptance check in all three tasks was re-run independently before committing. One inherited claim was corrected: `37-VALIDATION.md`'s Approval paragraph asserted a hook demo shape and a green `cargo test` that had not been substantiated at the time it was written — the paragraph now describes exactly the five checks actually performed.

## Issues Encountered

- The first hook-block probe did not fire: the probe line placed a `"` immediately before `inn`, which `REQUISITE_PATTERN`'s `(^|[^A-Za-z0-9_"])` prefix class deliberately excludes. This was a malformed probe, not a gate defect — re-running with the fixture's own literal shape (`inn: "..."` as a struct field) blocked the commit as expected. Worth noting for anyone writing future probes: the pattern matches requisite keys as *fields*, not as substrings inside a quoted string.

## User Setup Required

**Every existing clone must run this once** — git does not enable `.githooks/` on its own, so the hook is inert until then:

```bash
./scripts/setup-hooks.sh
```

Verify with `git config core.hooksPath` (expected: `.githooks`). CI enforces the same gate regardless, so a developer who skips this is backstopped, not unguarded.

## Next Phase Readiness

- PRIV-02 is complete: the gate runs at two points, and the full-HEAD run is green.
- PRIV-03 (history rewrite) remains deferred by design — every value hashed into `scripts/privacy-tokens.sha256` still exists unredacted in git history. The hash list is a stop-word list preventing *re-entry*, not retroactive protection. This is the phase's known, accepted residual risk and the natural candidate for a follow-up milestone.
- The gate is only as complete as its token list. New categories of real data (a new AD domain, a new office address) need `node scripts/check-privacy.mjs --add --hashes scripts/privacy-tokens.sha256`, documented in the file's own header.

## Self-Check: PASSED

- FOUND: `scripts/privacy-tokens.sha256` — 19 entries, all matching `^[0-9a-f]{64} [ABC]$`, sorted, zero plaintext
- FOUND: `.githooks/pre-commit` (100755), `scripts/setup-hooks.sh` (100755), `CONTRIBUTING.md`
- FOUND commit `ebf8416`
- `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` — exit 0 (full HEAD)
- Same gate scoped to `.planning/reference/design-system-v2/`'s 11 tracked files — exit 0; `git ls-files` still reports 11 (R4b intact, nothing untracked, no exclusion added)
- `node scripts/check-privacy.selftest.mjs` — exit 0, all assertions pass after this plan's edits to the gate
- `test ! -f scripts/check-privacy-requisites.sh` — passes (R5)
- `grep -c check-privacy .github/workflows/ci-full.yml` — 0 (D-12)
- Privacy step at ci-fast.yml line 30, `Install Rust toolchain` at line 33 — correct ordering
- `EXCLUDED_PATH_PREFIXES`/`EXCLUDED_PATH_EXACT` byte-identical to plan 37-03's committed version (R9); `.planning/phases/37-data-privacy/` absent from both (D-13)
- 18/18 completeness-proof files fired; scratchpad empty afterwards
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test --workspace --no-fail-fast -- --test-threads=1 --skip login_remember_persistent_cookie` — **exit 0, 102 suites ok, 0 failed** (C-07: single cargo invocation, nothing else cargo running)
- `git status --porcelain` before commit — only this plan's intended changes, no extracted blob or scratch artifact

---
*Phase: 37-data-privacy*
*Completed: 26-08-17*
