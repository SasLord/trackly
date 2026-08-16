---
phase: 37-data-privacy
plan: 03
subsystem: infra
tags: [privacy, durable-gate, node-esm, tokenizer, self-test]

requires:
  - phase: 37-data-privacy plan 01
    provides: "Marker-shape checklist (class + structural form + count) for future production hash population"
  - phase: 37-data-privacy plan 02
    provides: "Marker-shape checklist continuation; HEAD scrubbed of dangling references"
provides:
  - "scripts/check-privacy.mjs — unified privacy gate: mode 1 (allowlist, ported verbatim from check-privacy-requisites.sh), mode 2 (SHA-256 n-gram token hashes), binary-extension control, path exclusions, fail-closed --hashes loading, --add interactive population"
  - "scripts/check-privacy.selftest.mjs — fixture-driven regression suite (6 assertions) provable entirely against fictional data"
  - "scripts/fixtures/privacy/* — 6 fictional fixtures exercising D-05/D-06/R7/C-02/R8"
affects: [37-04]

tech-stack:
  added: []
  patterns:
    - "Zero-dependency Node ESM gate pattern (only node:fs/path/url/crypto/child_process/readline), mirroring ui/scripts/check-print-isolation.mjs's header-comment/self-test-by-argument conventions but at repo-root scripts/"
    - "Explicit positional file-argument scan mode (bypassing git plumbing) so a fixture-driven self-test can target specific files without requiring them to be staged/committed first"
    - "charCode-based control-character comparison in raw-mode stdin input handling (avoids literal control bytes in source, which several string-based edit tools silently mangle)"

key-files:
  created:
    - scripts/check-privacy.mjs
    - scripts/check-privacy.selftest.mjs
    - scripts/fixtures/privacy/tokens.fixture.sha256
    - scripts/fixtures/privacy/with-marker.md
    - scripts/fixtures/privacy/without-marker.md
    - scripts/fixtures/privacy/empty.sha256
    - scripts/fixtures/privacy/allowlist-regression.rs.txt
    - scripts/fixtures/privacy/binary-regression.docx
    - scripts/fixtures/privacy/README.md
  modified: []

key-decisions:
  - "Widened the mode-1 file filter from a strict `\\.(rs|html)$` (ends-with) match to `\\.(rs|html)(\\.|$)` so it also recognizes a `.rs.txt`/`.html.txt` extension chain — required because the plan's own allowlist-regression fixture needs a trailing `.txt` to stay out of `cargo build` while still being scanned as Rust-shaped content by the self-test"
  - "Added an explicit positional-file-argument scan mode (not spelled out in Task 1's CLI behavior list) alongside --staged/head, since the fixture-driven self-test needs to target specific uncommitted fixture files directly rather than scanning all of staged/HEAD"
  - "Binary-extension violations (R8) are labeled class D in the D-16 output format (`путь — маркер класса D`), distinct from mode 1's `requisite` label and mode 2's A/B/C hash-file classes — matches 37-RESEARCH.md's class taxonomy (A/B/C/D) where D is binary reference artifacts"
  - "Deduplicated mode-2 hash violations per (path, line, class) rather than per matching n-gram, to keep D-16 output readable without changing detection coverage — a line with multiple overlapping n-gram hits of the same class reports once, not N times"
  - "--add's class prompt (A/B/C) uses normal readline with echo; only the value prompt uses raw-mode stdin with suppressed echo — the class label alone is not sensitive, only the raw value is (D-15's 'no echo' requirement applies to the value, not the non-sensitive class selector)"

patterns-established:
  - "Self-test-by-child-process pattern: check-privacy.selftest.mjs shells out to check-privacy.mjs via execFileSync per fixture and asserts on exit code + absence of the fixture's raw token string in stdout+stderr, never on stdout content directly — keeps the self-test itself from ever needing to know the gate's exact message wording"

requirements-completed: [PRIV-02]

duration: ~40min
completed: 26-08-17
---

# Phase 37 Plan 03: Unified durable privacy gate — check-privacy.mjs (mode 1 + mode 2 + binary control) Summary

**Built scripts/check-privacy.mjs, a zero-dependency Node ESM gate that absorbs the existing bash allowlist regression verbatim and adds a new SHA-256 n-gram token-hash mode plus binary-extension path control, together with a 6-fixture self-test — all provable against fictional data only, with no dependency on the not-yet-populated production hash file.**

## Performance

- **Duration:** ~40 min
- **Tasks:** 2/2 completed
- **Files created:** 9 (2 scripts, 7 fixtures/README)

## Accomplishments

- Implemented `scripts/check-privacy.mjs` with three independent scan functions (mode 1 allowlist, mode 2 n-gram hash, binary-extension control) plus fail-closed `--hashes` loading and hardcoded, reviewable path exclusions.
- Ported mode 1's `ALLOWED` array (23 entries) and `PATTERN` regex verbatim from `scripts/check-privacy-requisites.sh`, translating only the POSIX `[[:space:]]` class to `\s` — verified 0 violations against all 23 original literals and exactly 1 against a 24th non-matching literal.
- Implemented the D-05/D-06 tokenizer: `WORD_RE` extracts letter/digit runs, a sliding 1–3-word n-gram window builds candidate phrases, `normalize()` (lowercase + ё→е + Unicode NFC) matches the scanner's own normalization so a stored hash and a scanned n-gram always agree.
- Implemented binary-extension control (R8): `.docx/.xlsx/.pdf/.png/.jpg/.jpeg` outside a hardcoded `BINARY_ALLOWLIST` (seeded with `crates/trackly-app/icons/*` and `crates/trackly-app/tests/fixtures/logo_test.png`) is a violation, labeled class D.
- Implemented `--add`: TTY check before any raw-mode call (Pitfall 4 — no uncaught `TypeError` off a pipe/CI), readline prompt for class (A/B/C), raw-mode suppressed-echo prompt for the value, normalize + SHA-256 + sorted append to the `--hashes` target file (D-07).
- Built 6 fictional fixtures (`scripts/fixtures/privacy/`) and `check-privacy.selftest.mjs`, which shells out to the gate per fixture via `execFileSync` and asserts both exit code and that the fixture's raw token string never appears in the child process's combined stdout+stderr (D-16). All 6 assertions pass.
- Confirmed all 5 hard prohibitions held: `scripts/privacy-tokens.sha256` was not created; `scripts/check-privacy-requisites.sh` was not touched or deleted; `.githooks/`, `scripts/setup-hooks.sh`, `CONTRIBUTING.md`, `.github/workflows/ci-fast.yml` were not touched; no inline per-token disable mechanism exists anywhere in the source; every git invocation uses `execFileSync('git', [argsArray])`, never a string-interpolated shell command.
- Re-ran the old `scripts/check-privacy-requisites.sh` against the final tree — still passes, confirming the new gate's fixtures introduced no real-looking requisite literals.

## Task Commits

Each task was committed atomically.

1. **Task 1: Core scanner (mode 1 + mode 2 + binary control + fail-closed `--hashes`)** — `fd0c39d` (feat)
2. **Task 2: `--add` implementation + fixtures + `check-privacy.selftest.mjs`** — `85bad7a` (feat)

**Plan metadata:** (this commit, docs)

## Files Created/Modified

- `scripts/check-privacy.mjs` — unified gate; mode 1 (allowlist, C-02), mode 2 (n-gram hash, D-05/D-06/D-07), binary control (R8), path exclusions (D-13/D-14), fail-closed `--hashes` (R7), `--add` (D-15)
- `scripts/check-privacy.selftest.mjs` — 6 fixture-driven assertions, self-contained, zero production-file dependency (C-01)
- `scripts/fixtures/privacy/tokens.fixture.sha256` — test-only hash file, one fictional class-B entry (D-07 format + D-08-style header)
- `scripts/fixtures/privacy/with-marker.md` — fictional two-word marker adjacent in one line (D-05 positive case)
- `scripts/fixtures/privacy/without-marker.md` — same words, split across separate lines (D-05 negative case)
- `scripts/fixtures/privacy/empty.sha256` — zero-byte file (R7 fail-closed case)
- `scripts/fixtures/privacy/allowlist-regression.rs.txt` — `.rs`-shaped, `.txt` extension, unlisted inn/ogrn literals (C-02 regression)
- `scripts/fixtures/privacy/binary-regression.docx` — zero-byte file at a disallowed path (R8 negative case)
- `scripts/fixtures/privacy/README.md` — per-fixture explanation, mirrors `crates/trackly-app/tests/fixtures/devices/README.md`'s convention

## Decisions Made

See `key-decisions` in frontmatter for the full list. Most consequential: widening the mode-1 file filter to accept a `.rs.txt`/`.html.txt` extension chain (not just a terminal `.rs`/`.html`) so the plan's own regression fixture could satisfy two competing constraints simultaneously — staying out of `cargo build` while still being recognized as Rust-shaped content by the gate.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Mode-1 file filter did not recognize the plan's own `.rs.txt` regression fixture**
- **Found during:** Task 2, while writing `allowlist-regression.rs.txt` per the plan's exact naming instruction
- **Issue:** The plan specifies `allowlist-regression.rs.txt` — real extension `.txt`, so `cargo build` never picks it up — but Task 1's file filter (`\.(rs|html)$`, strict ends-with) would never match a `.txt`-terminated path, meaning mode 1 would silently skip the fixture and the C-02 regression assertion could never fail as designed.
- **Fix:** Widened `REQUISITE_FILE_RE` from `\.(rs|html)$` to `\.(rs|html)(\.|$)`, matching either a terminal `.rs`/`.html` extension or an internal `.rs.`/`.html.` extension-chain segment. Verified this does not create false positives (e.g. `foo.rst` does not match) and does not affect any real committed `.rs`/`.html` file's detection.
- **Files modified:** `scripts/check-privacy.mjs`
- **Verification:** `check-privacy.selftest.mjs` assertion 5 (mode-1 regression) passes; re-ran the Task 1 all-23-`ALLOWED`-literals regression check after the change — still 0 violations for allowed values, 1 for a 24th unlisted value.
- **Committed in:** `85bad7a` (Task 2 commit)

**2. [Rule 3 - Blocking] Explicit positional file-argument scan mode not in Task 1's CLI spec, but required by Task 2's self-test**
- **Found during:** Task 2, while implementing `check-privacy.selftest.mjs`'s fixture invocations (`--hashes tokens.fixture.sha256 with-marker.md`)
- **Issue:** Task 1's `<behavior>` CLI section only describes `--staged` (mode = staged, default = head) — scanning the whole repository. The self-test needs to target one specific fixture file at a time so each assertion is isolated and deterministic; scanning all of HEAD or staged would mix in unrelated files and make the fixture-driven assertions meaningless.
- **Fix:** Added a third target-collection mode — explicit positional file arguments, read directly via `fs.readFileSync` (bypassing git plumbing entirely so it works on uncommitted fixtures) — that takes priority over `--staged`/head when any positional args are present. This was added during Task 1's own implementation (before Task 2's fixtures existed) since it is foundational scanning infrastructure, not an `--add`-specific feature.
- **Files modified:** `scripts/check-privacy.mjs` (`collectExplicitTargets`, CLI dispatch in `main()`)
- **Verification:** All 6 self-test assertions pass using this mode; `--staged`/head modes remain unaffected (untested by this plan's own acceptance criteria, but code paths are unchanged).
- **Committed in:** `fd0c39d` (Task 1 commit, since the capability was added before Task 2's fixtures existed)

---

**Total deviations:** 2 auto-fixed (both Rule 3 — blocking issues discovered while satisfying the plan's own fixture/self-test design, not scope creep)
**Impact on plan:** Both changes are additive/widening (broader file-extension recognition, an additional scan mode) and do not narrow or weaken any existing detection path. No plan acceptance criterion was relaxed to accommodate either fix.

## Issues Encountered

- While authoring `--add`'s raw-mode stdin key handling, initial attempts to write literal control-character bytes (CR/LF/EOF/backspace/Ctrl+C) directly into the source via string-based edit tools resulted in the control bytes being silently stripped to empty strings, breaking the `Edit` tool's exact-match requirement and producing dead comparison branches. Resolved by comparing `chunk.toString().charCodeAt(0)` against numeric character codes instead of literal control-character string comparisons — avoids any literal non-printable byte appearing in the source file at all, sidestepping the tooling issue entirely rather than working around it.

## User Setup Required

None — no external service configuration required. `scripts/check-privacy.mjs` is not yet wired into `.githooks/pre-commit` or CI (that is plan 37-04's job), so there is nothing for a user to activate yet.

## Next Phase Readiness

- `scripts/check-privacy.mjs` and `scripts/check-privacy.selftest.mjs` are complete, self-testing, and provably free of any dependency on real values or the not-yet-existing production hash file.
- Plan 37-04 can now: (1) populate `scripts/privacy-tokens.sha256` using the marker-shape checklists from plans 37-01/37-02 plus its own `--add` flow, (2) wire the gate into `.githooks/pre-commit` (`--staged` mode) and `.github/workflows/ci-fast.yml` (head mode, replacing the `check-privacy-requisites.sh` step), and (3) retire `scripts/check-privacy-requisites.sh` once the new gate's mode 1 is confirmed to cover its exact regression surface (already proven equivalent in this plan via the 23-literal round-trip test).
- `scripts/check-privacy.mjs --hashes <path>` has no default production path by design (documented inline) — plan 37-04 should decide whether to hardcode `scripts/privacy-tokens.sha256` as a default once that file exists, or keep `--hashes` explicit at every call site (pre-commit hook, CI step).

## Self-Check: PASSED

- FOUND: scripts/check-privacy.mjs
- FOUND: scripts/check-privacy.selftest.mjs
- FOUND: scripts/fixtures/privacy/tokens.fixture.sha256
- FOUND: scripts/fixtures/privacy/with-marker.md
- FOUND: scripts/fixtures/privacy/without-marker.md
- FOUND: scripts/fixtures/privacy/empty.sha256
- FOUND: scripts/fixtures/privacy/allowlist-regression.rs.txt
- FOUND: scripts/fixtures/privacy/binary-regression.docx
- FOUND: scripts/fixtures/privacy/README.md
- FOUND commit fd0c39d (Task 1)
- FOUND commit 85bad7a (Task 2)
- `node scripts/check-privacy.selftest.mjs` — exit 0, all 6 assertions pass (re-verified after all edits)
- `grep -E "^import" scripts/check-privacy.mjs` — only `node:` builtin specifiers
- `scripts/privacy-tokens.sha256` confirmed absent (hard prohibition #1 honored)
- `scripts/check-privacy-requisites.sh` confirmed unmodified and still passing (hard prohibitions #2 honored, and re-run as a sanity check)
- `.githooks/`, `scripts/setup-hooks.sh`, `CONTRIBUTING.md`, `.github/workflows/ci-fast.yml` confirmed untouched (`git diff --stat` against these paths since before this plan's first commit — empty)

## TDD Gate Compliance

Both tasks carry `tdd="true"`, but this plan's own frontmatter is `type: execute` (not `type: tdd`), so the whole-plan RED/GREEN/REFACTOR gate enforcement does not apply here — only the per-task guidance does. In practice: Task 1 has no dedicated test file of its own (the fixture-driven regression suite is Task 2's deliverable, since it needs fixtures that do not exist until Task 2), so its four acceptance criteria (fail-closed on missing/empty `--hashes`, zero non-`node:` imports, `EXCLUDED_PATHS` excludes `37-data-privacy`, 23-literal ALLOWED round-trip) were used directly as executable specifications and verified via ad hoc shell invocations before the `feat` commit — functioning as the task's real (if informally structured) test-first gate, given no pre-existing test framework/harness for this directory. Task 2's fixtures + `check-privacy.selftest.mjs` largely exercise scanning behavior Task 1 had already implemented correctly (so the self-test did not fail on first run) — a genuine `test`-then-`feat` RED/GREEN split was not meaningfully achievable there without fabricating an artificially-broken intermediate state; both fixtures/self-test and `--add`'s implementation were verified together (`node scripts/check-privacy.selftest.mjs` exit 0, `--add < /dev/null` exits 1 with the expected message) before the single Task 2 `feat` commit. No RED failures were observed at any point that required investigation per the tdd_execution error-handling guidance.

---
*Phase: 37-data-privacy*
*Completed: 26-08-17*
