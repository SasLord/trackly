---
phase: 21-cartridge-drum-codes
reviewed: 2026-07-14T00:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - crates/trackly-infra/src/repos/cartridges_sqlite.rs
  - crates/trackly-app/tests/cartridges_numbering.rs
findings:
  critical: 0
  warning: 2
  info: 3
  total: 5
status: issues_found
---

# Phase 21: Code Review Report

**Reviewed:** 2026-07-14
**Depth:** standard
**Files Reviewed:** 2
**Status:** issues_found

## Summary

The core change of CRT-01 — switching the auto-generated cartridge/drum code
format in `assign_code_in_tx` from `format!("{prefix}-{seq:06}")` to
`format!("{prefix}-{seq:04}")` — is **correct**. `:04` is a *minimum* width
(zero-padded to 4, growing past 4 digits automatically once `seq >= 10000`), so
there is no truncation or overflow risk. The critical adjacent paths are
verified untouched and sound:

- **Retry loop / counter** (lines 148–163): `increment_counter_in_tx` is still
  called before every candidate, existence is re-checked, and the counter is
  never rolled back on collision. The format change does not interact with this.
  Because the loop re-checks uniqueness, a shortened auto-code that happens to
  collide with a pre-existing custom/override code (or an old 6-digit code) is
  safely skipped — no duplicate-code bug is introduced.
- **`code_override` path** (lines 124–138): unchanged; still validates UNIQUE
  and returns `(s, false)`.
- **Inline unit tests** (`assign_code_auto_increments`,
  `assign_code_drum_uses_d_prefix_and_separate_counter`): correctly updated to
  pin the exact 4-digit format (`C-0001`, `C-0002`, `D-0001`, `D-0002`). These
  are the tests that actually lock the new format.

No BLOCKER-level defects were found. The remaining findings are format-consistency
gaps left behind by the change and a weakness in the integration test's assertion.

## Warnings

### WR-01: UI code placeholder still advertises the old 6-digit width

**File:** `ui/src/features/cartridges/CartridgeFormBody.svelte:54`
(cross-referenced; outside the two reviewed files, but a direct consequence of
this phase's change)
**Issue:** The form placeholder still shows the pre-change width:
```svelte
const codePlaceholder = $derived(kindId === 2 ? 'D-XXXXXX' : 'C-XXXXXX');
```
Six `X`s imply a 6-digit code, but the backend now generates 4-digit codes
(`C-0001`). After changing the numbering format, this user-facing hint was not
updated, so the UI misrepresents the format the system actually assigns. The
format change is effectively incomplete at the presentation layer.
**Fix:** Align the placeholder with the new minimum width, e.g.
`kindId === 2 ? 'D-XXXX' : 'C-XXXX'`.

### WR-02: Integration-test assertion cannot detect a regression to the old format

**File:** `crates/trackly-app/tests/cartridges_numbering.rs:76-82`
**Issue:** The `concurrent_50_unique_codes` assertion validates only a *lower
bound* on length:
```rust
code.len() >= 6
    && code.starts_with("C-")
    && code[2..].chars().all(|c| c.is_ascii_digit())
```
`C-000001` (old `:06`, len 8) satisfies `len() >= 6` just as well as `C-0001`
(new `:04`, len 6). This integration test — the primary numbering test for
CRT-01 — therefore passes identically before and after the change and provides
**no regression guard** for the 4-digit format. The exact-format guarantee rests
entirely on the inline unit tests. Given the phase's whole purpose is the width
change, the flagship test not being able to observe it is a real test-validity
gap.
**Fix:** Assert the exact width for the small sequence values this test produces
(seq 1–50 all render as 4 digits), e.g. check `code.len() == 6` for these codes,
or assert the numeric suffix width is exactly 4 when `seq < 10000`:
```rust
let digits = &code[2..];
assert_eq!(digits.len(), 4, "seq < 10000 must render as exactly 4 digits, got: {}", code);
```

## Info

### IN-01: Stale doc comment claims "C-NNNNNN" format

**File:** `crates/trackly-app/tests/cartridges_numbering.rs:38`
**Issue:** `/// Spawn 50 concurrent creates; verify all 50 codes are unique and
in C-NNNNNN format.` — six `N`s, describing the pre-change format. The
module-level comment (line 4) and the inline comment (line 74) were correctly
updated to "C-NNNN / min 4 digits", so this line is inconsistent with its own
file.
**Fix:** Change `C-NNNNNN` to `C-NNNN` on line 38.

### IN-02: Mixed-width codes across the upgrade boundary (cosmetic)

**File:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs:151`
**Issue:** On a database created by an earlier version, existing rows carry
6-digit codes (`C-000001`) and the shared `cartridge_seq`/`drum_seq` counters are
already advanced. After upgrade, newly generated codes use 4-digit padding
(`C-0005`), so a single DB will display mixed-width codes. Uniqueness is
preserved by the retry loop, so this is not a correctness or data-loss issue —
only a cosmetic inconsistency in existing installs. Worth a conscious
acknowledgement in the phase notes rather than a code change.
**Fix:** None required for correctness. If uniform display matters, document the
behavior; a backfill/renumber is out of scope and risky.

### IN-03: `collision_retry_does_not_lose_counter` name overstates what it checks

**File:** `crates/trackly-app/tests/cartridges_numbering.rs:92-126`
**Issue:** The test performs three *sequential* creates and asserts the numeric
suffix strictly increases. It never induces an actual UNIQUE collision, so it
verifies monotonicity of sequential allocation, not the "counter not lost on
collision" retry branch its name implies. The suffix parse (`c[2..].parse::<u64>()`)
does correctly handle leading zeros, so the test is valid for what it actually
does — the name/comment just promise more coverage than exists.
**Fix:** Either rename to reflect "sequential codes are monotonic", or add a case
that pre-inserts a colliding auto-format code (e.g. seed `C-0001`) and asserts
the next auto-create skips it — exercising the retry loop directly.

---

_Reviewed: 2026-07-14_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
