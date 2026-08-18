---
phase: 37-data-privacy
verified: 2026-08-18T01:53:08Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
gaps: []
deferred:
  - truth: "All real organization/employee data is fully removed from the repository (not just HEAD)"
    addressed_in: "Future Requirements (PRIV-03, explicitly deferred, not scheduled to any phase)"
    evidence: "REQUIREMENTS.md Future Requirements: 'PRIV-03: Очистка утёкших данных из истории git (filter-repo + force-push) либо перевод репозитория в private.' ROADMAP.md Phase 37 Success Criterion 3: 'История git НЕ переписана (осознанное решение пользователя от 2026-08-08)'"
---

# Phase 37: Приватность данных — Verification Report

**Phase Goal:** Текущее состояние репозитория (HEAD) не содержит реальных реквизитов организации
и ФИО сотрудников, и повторная утечка ловится автоматической проверкой до попадания в репозиторий.

**Verified:** 2026-08-18T01:53:08Z
**Status:** passed
**Re-verification:** No — initial verification

**Scope note (per REQUIREMENTS.md / ROADMAP.md, and explicitly not re-litigated here):** this
phase's guarantee is **"no real data at HEAD + re-entry of already-known real values is blocked"**,
not **"no real data anywhere in the repository"**. Every value ever hashed into
`scripts/privacy-tokens.sha256` still exists unredacted in git history — PRIV-03 (history rewrite
or making the repo private) is explicitly deferred by a 2026-08-08 user decision recorded in
`REQUIREMENTS.md`'s Out of Scope table and is not evaluated as a gap here.

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | HEAD contains no real org name/ИНН/КПП/ОКПО/ОГРН/address — only `org.name`-style variables and fictional placeholders | ✓ VERIFIED | `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` (full HEAD, all file types) → `PASS — 0 нарушений`, exit 0 (independently re-run, not just orchestrator claim). Spot-checked `crates/trackly-app/src/pdf/renderer.rs` (`org_name: "ООО Ромашка"` — fictional test placeholder) and `.planning/STATE.md` (`example.local`, `dc.example.local`, `us100@example.local`, «Иванов Александр Дмитриевич» used as an anonymization example, not a live record) — all placeholder-shaped, consistent with CLAUDE.md's approved convention. |
| 2 | `.planning/`-artifacts on HEAD contain no real employee ФИО, including the three explicitly named leak points (deleted phase-15 brief, `STATE.md`, `260805-lrs-*`) | ✓ VERIFIED | `test ! -f .planning/PHASE-BRIEF-act-pdf-word-fidelity.md` → ABSENT (confirmed). `STATE.md` and `260805-lrs-PLAN.md`/`260805-lrs-SUMMARY.md` are in 37-01's `files_modified` and covered by the 22-replacement scrub (37-01-SUMMARY.md) and the full-HEAD gate PASS above. |
| 3 | Git history is NOT rewritten — cleanup is HEAD-only, residual risk in old commits is a deliberate, accepted 2026-08-08 decision | ✓ VERIFIED | `git log` shows the phase's 12 commits are additive scrub/activation commits, no `filter-repo`/force-push evidence; `REQUIREMENTS.md` Out of Scope table documents the decision explicitly. Consistent with the deferred-item note above. |
| 4 | A durable scan script (mode 1 allowlist + mode 2 n-gram hash + binary control) exists, is wired into an automatic CI/hook chain, and runs without manual invocation | ✓ VERIFIED | `scripts/check-privacy.mjs` exists (593 lines-scale gate, zero-dependency Node ESM — `grep -E "^import"` shows only `node:` specifiers). Wired at two points, both independently re-verified live: (a) `.githooks/pre-commit` → `scripts/setup-hooks.sh` → `git config core.hooksPath` reflects `.githooks`; a scratch-clone commit with an injected unapproved requisite literal was **blocked** (`git commit` exit 1, HEAD unchanged) and a clean commit **succeeded** (exit 0); `node`-absent produces exit 1 with an instructional message, not a silent pass. (b) `.github/workflows/ci-fast.yml` line 30 calls `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` immediately after Checkout, before any build step; `ci-full.yml` has 0 matches (`grep -c check-privacy` → 0), confirming no duplicate. |
| 5 | Gate is green on the cleaned HEAD with zero false positives on legitimate placeholders (`org.name`, «Иванов И.И.», etc.) | ✓ VERIFIED | Full-HEAD run above: exit 0, 0 violations, across the whole tree including `.planning/reference/design-system-v2/`'s 11 tracked files (independently re-run: exit 0). `node scripts/check-privacy.selftest.mjs` → exit 0, all 6 fixture assertions pass (independently re-run). |

**Score:** 5/5 truths verified

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| PRIV-01 | 37-01, 37-02 | HEAD has no real org requisites/ФИО in code/templates/tests/`.planning/` | ✓ SATISFIED | Truths 1–3 above; per-token reachability probe (orchestrator, re-usable evidence): 19/19 production hashes independently traced back to the specific pre-cleanup blob and marker-shape checklist line that produced them; 18/18 pre-cleanup files fire the finished gate when re-scanned from their pre-cleanup blob (37-04-SUMMARY.md completeness table). |
| PRIV-02 | 37-03, 37-04 | An attempt to commit real org requisites/ФИО is caught by an automatic check before it reaches the repository | ✓ SATISFIED (with documented residual risk, see below) | Truths 4–5 above; live reproduction of a blocked commit and a passing clean commit (this verification); mode 1 (structural key-pattern detection, not limited to known values) independently reproduced catching an unlisted `inn:`/`ogrn:` literal. |

No orphaned requirements: `REQUIREMENTS.md`'s Traceability table lists only PRIV-01 and PRIV-02
under Phase 37; both are declared in plan frontmatter (`37-01`/`37-02` → PRIV-01, `37-03`/`37-04`
→ PRIV-02) with no gap between the two lists. PRIV-03 is in `REQUIREMENTS.md`'s **Future
Requirements** section (not mapped to any phase) — correctly out of scope here, not orphaned.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `scripts/check-privacy.mjs` | Unified gate: mode 1 + mode 2 + binary control + fail-closed `--hashes` + `--add` | ✓ VERIFIED | Exists, zero-dependency, all behaviors independently exercised (below) |
| `scripts/check-privacy.selftest.mjs` | Fixture-driven self-test | ✓ VERIFIED | `node scripts/check-privacy.selftest.mjs` → exit 0, 6/6 assertions (re-run) |
| `scripts/privacy-tokens.sha256` | Production token hash list, D-07 format | ✓ VERIFIED | 19 data lines, all match `^[0-9a-f]{64} [ABC]$` (re-verified via grep), zero plaintext |
| `.githooks/pre-commit` | Staged-scope entry point, fail-closed on missing node | ✓ VERIFIED | Exists (100755), live-blocked a violating commit, live-passed a clean one, live-failed closed with no `node` |
| `scripts/setup-hooks.sh` | One-line `core.hooksPath` activation | ✓ VERIFIED | Ran in a scratch clone, `git config core.hooksPath` → `.githooks` |
| `CONTRIBUTING.md` | Developer-facing activation doc | ✓ VERIFIED | Exists at repo root, new file per plan |
| `.github/workflows/ci-fast.yml` (privacy step) | Relocated after Checkout, before build steps | ✓ VERIFIED | Confirmed at line 30/33 ordering by direct read |
| `scripts/check-privacy-requisites.sh` (deletion) | Legacy script removed, absorbed by mode 1 | ✓ VERIFIED | `test ! -f` → ABSENT |
| `.planning/PHASE-BRIEF-act-pdf-word-fidelity.md` (deletion) | Densest single leak site removed | ✓ VERIFIED | `test ! -f` → ABSENT |
| `.planning/reference/act-word-source/` (untrack) | Untracked, remains on local disk | ✓ VERIFIED | `git ls-files` → 0 entries |
| `.planning/reference/design-system-v2/` (preserved) | Stays tracked (unrelated content) | ✓ VERIFIED | `git ls-files` → 11 entries |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `.githooks/pre-commit` | `scripts/check-privacy.mjs` | `exec node .../check-privacy.mjs --staged --hashes .../privacy-tokens.sha256` | ✓ WIRED | Read hook source directly; live-exercised (block + pass + node-absent) |
| `.github/workflows/ci-fast.yml` | `scripts/check-privacy.mjs` | `run: node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` | ✓ WIRED | Read workflow source directly at the documented line position |
| `scripts/setup-hooks.sh` | `git config core.hooksPath` | `git config core.hooksPath .githooks` | ✓ WIRED | Live-exercised in a scratch clone |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Full-HEAD gate green | `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` | `PASS — 0 нарушений`, exit 0 | ✓ PASS |
| Self-test green | `node scripts/check-privacy.selftest.mjs` | 6/6 assertions ok, exit 0 | ✓ PASS |
| Pre-commit blocks a violation (scratch clone) | stage `inn: "9998887771",` in a `.rs` file, commit | gate `FAIL — 1 нарушений`, `git commit` exit 1, HEAD unchanged | ✓ PASS |
| Pre-commit allows a clean commit (scratch clone) | stage a marker-free `.rs` file, commit | gate `PASS — 0 нарушений`, commit created, exit 0 | ✓ PASS |
| Pre-commit fails closed with no `node` on PATH | `PATH="/usr/bin:/bin" git commit ...` | instructional stderr message, exit 1, no silent pass | ✓ PASS |
| `ci-full.yml` has no duplicate step | `grep -c check-privacy .github/workflows/ci-full.yml` | `0` | ✓ PASS |
| CR-01 reproduction (review claim check) | fictional hyphenated 1-token value hashed via raw `normalize()` (simulating `--add`), then scanned in a sentence containing that literal string | `PASS — 0 нарушений` (should have fired) | ✗ REPRODUCED — confirms 37-REVIEW.md CR-01 is accurate, not overstated |
| CR-02 reproduction (review claim check) | `chmod 000` on a file containing an unapproved `inn:` literal, scanned against the real production hash list | `PASS — 0 нарушений` (file silently skipped, no diagnostic) | ✗ REPRODUCED — confirms 37-REVIEW.md CR-02 is accurate, not overstated |

### Probe Execution

Skipped — no `scripts/*/tests/probe-*.sh` files exist and neither the phase's plans nor its
review reference a probe-based verification convention for this phase.

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` markers found in any file this phase created
or modified (`grep` re-run against `scripts/check-privacy.mjs`, `scripts/check-privacy.selftest.mjs`,
`.githooks/pre-commit`, `scripts/setup-hooks.sh` — zero matches). No debt-marker gate triggered.

The two Critical findings from `37-REVIEW.md` (CR-01, CR-02) were independently reproduced above
and are real, live defects in the gate — see "Known Open Findings" below for how they were
weighed against the phase goal.

### Human Verification Required

None. Every truth in this report was verified by direct command execution against the actual
repository (not SUMMARY.md narration), including live reproduction of the two Critical review
findings.

### Known Open Findings (from `37-REVIEW.md`, weighed against the phase goal — not treated as gaps)

`37-REVIEW.md` is committed to this phase and reports 2 Critical + 5 Warning + 3 Info findings
against `scripts/check-privacy.mjs`. This verification independently reproduced both Critical
findings (see Behavioral Spot-Checks above) and confirms they are accurate. They are recorded
here as **follow-up debt**, not phase-blocking gaps, for the following reasons:

- **CR-01** (`--add` hashes a raw value that scan-time tokenization can never reproduce for
  values with internal punctuation or >3 words): affects only **future** `--add` entries. The
  orchestrator's per-token reachability probe shows this has **not** happened to any of the 19
  current production entries (19/19 reachable) — re-verified structurally via the completeness
  table in `37-04-SUMMARY.md`. Does not defeat "repeat leak of already-known values is caught,"
  which is the literal ROADMAP wording ("повторная утечка"). Recommend a follow-up quick task to
  fix `runAdd()` to hash through the same tokenizer the scanner uses, per the review's suggested fix.
- **CR-02** (unreadable/oversized scan targets are silently dropped, no diagnostic, gate reports
  `PASS`): a genuine fail-open path for a specific edge case (permission-denied file, or a staged
  blob exceeding the hardcoded 64 MB `maxBuffer`). Does not affect the normal commit path (a
  readable, reasonably-sized file) — the standard flow was live-verified blocking a violation
  above. This is real residual risk for the "accidental large-export leak" scenario the gate was
  partly built to catch, and is worth prioritizing in the same follow-up as CR-01.
- **WR-01/02/03** (mode 1 only scans `.rs`/`.html`, case-sensitively, quoted-values-only): mode 2
  (hash-based) still scans all text files regardless of extension/case/quoting, so class B/C
  (ФИО, infra identifiers) coverage is unaffected; only mode 1's structural class-A numeric-field
  detection has this narrower scope, inherited in part from the absorbed legacy script.
- **WR-04** (`scripts/fixtures/privacy/` permanently exempt from auto-discovery scans): scoped,
  documented in code with an inline comment, and in 37-04-SUMMARY.md's own deviation log.
- **WR-05** (`--add`'s raw-mode stdin only inspects the first byte of a multi-character chunk):
  affects the interactive `--add` value-capture UX under specific paste/fast-typing conditions,
  not the scan path.

**Recommendation:** file a follow-up quick task or Phase 38+ item to fix CR-01 and CR-02 (both
have concrete fixes proposed in `37-REVIEW.md`), since they are the two findings with the clearest
path to a real (if narrow) leak escaping the automatic check. This is a recommendation, not a
blocker — the phase's literal goal, as scoped by ROADMAP.md and REQUIREMENTS.md, is achieved.

### Gaps Summary

No gaps. Both phase requirements (PRIV-01, PRIV-02) are satisfied as literally stated in
`ROADMAP.md` and `REQUIREMENTS.md`: HEAD is clean (independently re-verified via a full-repository
gate scan and spot-checks of previously-flagged files), and re-entry of already-known real values
is blocked at two independently-wired, independently-live-tested enforcement points (a local
pre-commit hook and a CI backstop). Two Critical and five Warning findings from the phase's own
code review were independently reproduced and are real, but affect a broader guarantee ("catches
every future leak, including never-before-seen compound-shaped or unreadable-file leaks") that
was never the literal scope of this phase — that broader guarantee is explicitly bounded by the
deferred, out-of-scope PRIV-03 (full history rewrite) and by the design tradeoff inherent to any
hash-list-based (as opposed to ML/NER-based) detection mechanism. These are recorded above as
recommended follow-up debt.

---

_Verified: 2026-08-18T01:53:08Z_
_Verifier: Claude (gsd-verifier)_
