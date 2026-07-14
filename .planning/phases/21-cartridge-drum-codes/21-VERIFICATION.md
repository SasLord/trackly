---
phase: 21-cartridge-drum-codes
verified: 2026-07-15T00:00:00Z
status: passed
score: 4/4 must-haves verified
overrides_applied: 0
---

# Phase 21: Точечные фиксы — коды картриджей/фотобарабанов Verification Report

**Phase Goal:** Автоматически присваиваемые коды новых картриджей и фотобарабанов используют укороченный, согласованный формат (Новый картридж без явного кода → C-XXXX, новый фотобарабан → D-XXXX, 4 цифры минимум).
**Verified:** 2026-07-15
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Новый картридж без явного кода получает автокод в формате `C-XXXX` (мин. 4 цифры) | ✓ VERIFIED | `crates/trackly-infra/src/repos/cartridges_sqlite.rs:151` — `format!("{prefix}-{seq:04}")`. Unit test `assign_code_auto_increments` asserts exact output `C-0001`, `C-0002` (line 1434, 1440), passes (`cargo test -p trackly-infra --lib assign_code` → 3 passed). Integration test `concurrent_50_unique_codes` (50 concurrent creates via the real `CartridgeService::create` path) confirms all codes match `C-` + all-digit suffix, passes. |
| 2 | Новый фотобарабан без явного кода получает автокод в формате `D-XXXX` (мин. 4 цифры) | ✓ VERIFIED | Same format string, `kind_id == 2` branch selects `("drum_seq", 'D')` (line 142-146, unchanged mapping). Unit test `assign_code_drum_uses_d_prefix_and_separate_counter` asserts exact output `D-0001`, `D-0002` on a separate counter from `C-0001` in the same transaction (line 1460-1462), passes. |
| 3 | Существующие 6-значные коды (например `C-000123`) остаются валидными строками, коллизий с новым форматом нет | ✓ VERIFIED | `{:04}` is a *minimum*-width specifier (not fixed-width), confirmed by source read. Old 6-digit codes (`C-000001`..) and new 4-digit codes (`C-0001`..) are distinct strings for the same counter value range (different digit counts/padding) as long as the counter is never reset — no migration was added (`grep` confirms no new migration file), so `cartridge_seq`/`drum_seq` continue from their prior value. The retry-loop (`assign_code_in_tx` lines 149-163) re-checks `EXISTS` before returning any candidate regardless, providing a functional safety net even in edge cases. No behavioral test exercises a live upgrade scenario (old 6-digit rows + continuing counter) directly, but the logic is sound and reviewed (code review IN-02: "not a correctness or data-loss issue — only cosmetic"). |
| 4 | Кастомный `code_override` и retry-loop механизм счётчика не затронуты | ✓ VERIFIED | Diff (`166e540`, `344ca7e`) touches only the format string and comments/tests; `code_override` branch (lines 124-138) and retry-loop structure (lines 149-163) are byte-identical to pre-phase code apart from the single format specifier. Unit test `assign_code_custom_roundtrip` (unaffected by this phase) still passes, confirming the override path. |

**Score:** 4/4 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs` | `fn assign_code_in_tx` generates via `format!("{prefix}-{seq:04}")` | ✓ VERIFIED | Line 151 contains exactly this. `grep -n '{seq:06}'` in the file returns nothing (old format fully removed). Doc-comment (lines 109-117) and inline comment (lines 140-141) updated to `C-NNNN`/`D-NNNN`. Inline unit tests (`assign_code_auto_increments`, `assign_code_drum_uses_d_prefix_and_separate_counter`) updated to pin exact `C-0001`/`D-0001`/`D-0002` (fixed via post-merge gate commit `344ca7e`, not present in the executor's original plan scope). |
| `crates/trackly-app/tests/cartridges_numbering.rs` | Integration test verifying 4-digit (minimum) auto-code format | ✓ VERIFIED | `concurrent_50_unique_codes` asserts `code.len() >= 6 && code.starts_with("C-") && code[2..].all(is_ascii_digit)` — a minimum-width check per plan spec. Module/inline comments updated to `C-NNNN` (fixed via `2cb14ef`). `collision_retry_does_not_lose_counter` unaffected (parses suffix numerically, width-independent). Both tests pass. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `cartridges_sqlite.rs::assign_code_in_tx` | `cartridge_seq` / `drum_seq` counters | `increment_counter_in_tx` + `format!("{prefix}-{seq:04}")` | ✓ WIRED | Line 150: `let seq = increment_counter_in_tx(tx, counter_name)?;` immediately followed by line 151's format call. Pattern `\{seq:04\}` present exactly once, at the single generation site. |
| `crates/trackly-app/src/services/cartridge_service.rs` | `assign_code_in_tx` | direct call in `create()` | ✓ WIRED | `grep` confirms `cartridge_service.rs:120` calls `SqliteCartridgeRepository::assign_code_in_tx(...)` — the format change is exercised by the real create path used by the integration test, not just isolated unit tests. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| New cartridge auto-code is 4-digit-minimum `C-XXXX` | `cargo test -p trackly-infra --lib assign_code_auto_increments` | `C-0001`, `C-0002` asserted, 1 passed | ✓ PASS |
| New drum auto-code is 4-digit-minimum `D-XXXX`, separate counter | `cargo test -p trackly-infra --lib assign_code_drum_uses_d_prefix_and_separate_counter` | `D-0001`, `D-0002` asserted, 1 passed | ✓ PASS |
| 50 concurrent real-service creates all unique + correct format | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test cartridges_numbering -- --test-threads=1` | 2 passed, 0 failed | ✓ PASS |
| No stray old-format generation string remains | `grep -n '{seq:06}' crates/trackly-infra/src/repos/cartridges_sqlite.rs` | no matches | ✓ PASS |
| No new clippy warnings introduced | `cargo clippy -p trackly-infra -- -D warnings` | clean, exit 0 | ✓ PASS |
| Full `trackly-app` integration suite unaffected | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app` | 0 FAILED lines in output | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| CRT-01 | 21-01-PLAN.md | Автоматически присваиваемый Код нового картриджа имеет формат `C-XXXX`, нового фотобарабана — `D-XXXX` | ✓ SATISFIED | Format string change verified in code, unit + integration tests pin/exercise it, wired into the real create service path. |

No orphaned requirements: `REQUIREMENTS.md` traceability table maps `CRT-01 → Phase 21` only, and the phase's single plan declares `requirements: [CRT-01]` — full 1:1 coverage.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/trackly-app/tests/cartridges_numbering.rs` | 74-83 | `concurrent_50_unique_codes` asserts only `len() >= 6` (a lower bound), which is satisfied identically by both the old 6-digit and new 4-digit format — this test cannot detect a regression back to `{seq:06}` | ⚠️ Warning (pre-existing, flagged in code review WR-02, not fixed) | Low — the exact-format guarantee is covered by the inline unit tests (`assign_code_auto_increments`/`assign_code_drum_uses_d_prefix_and_separate_counter`), which do pin `C-0001`/`D-0001` exactly and would fail on regression. The integration test's weak assertion is a test-quality gap, not a functional gap — goal is still achieved and guarded, just by a different test than the one nominally responsible for it. |
| `crates/trackly-app/src/dto/cartridge.rs`, `crates/trackly-app/src/dto/reports.rs`, `crates/trackly-core/src/domain/cartridges.rs`, `crates/trackly-infra/tests/seed_data.rs`, `ui/src/bindings.ts` (generated) | multiple | Stale `C-NNNNNN`/`C-000001`/`D-NNNNNN` references in doc comments (outside the two files this phase's plan scoped for editing) | ℹ️ Info | Documentation drift only — these are Rust doc comments (and their auto-generated TS mirror in `bindings.ts`, produced by `tauri-specta`) describing the code format in prose; they do not affect runtime behavior. Not part of the plan's declared `files_modified` scope. Does not block the phase goal. |
| `crates/trackly-app/tests/phase06_stubs.rs:529`, `crates/trackly-infra/src/repos/printers_sqlite.rs:892` | — | Literal `'C-000001'` fixture strings | ℹ️ Info (expected) | Explicitly excluded by the plan ("НЕ трогать литеральные коды-фикстуры... это ручные фикстуры с явным code_override, не сгенерированный вывод"). Correctly left untouched. |

No BLOCKER-level anti-patterns found. No `TBD`/`FIXME`/`XXX` debt markers in the two files this phase modified.

### Human Verification Required

None. The phase is a pure backend format-string change with deterministic, fully automatable test coverage (exact-value unit tests + a real-service-path integration test). No visual, real-time, or external-service behavior is involved. The one UI-facing artifact (placeholder text in `CartridgeFormBody.svelte`) is a static string verified by direct source inspection (`codePlaceholder = $derived(kindId === 2 ? 'D-XXXX' : 'C-XXXX')`), not a rendering/interaction concern needing a human check.

### Gaps Summary

No gaps. All 4 must-have truths verified, both required artifacts present/substantive/wired, both key links confirmed, requirement CRT-01 satisfied with no orphans. Two issues raised during code review (`21-REVIEW.md`) were fixed post-review by follow-up commits before this verification ran:

- **WR-01** (UI placeholder still showed 6-digit width) → fixed in `2cb14ef`.
- **Inline unit tests pinning old 6-digit format** (caught by the post-merge/regression gate, not originally flagged by the plan's own scope) → fixed in `344ca7e`.

One review finding remains open by design and is downgraded to a non-blocking warning in this report: **WR-02** (the integration test's assertion is a lower bound, not an exact-width check, so it alone cannot detect a regression to the old format). This does not block phase-goal achievement because the inline unit tests independently pin the exact format and would fail on such a regression — the safety net exists, just via a different test file than the one nominally responsible. Recommend addressing WR-02 opportunistically in a future touch of this test file, but it is not required to close Phase 21.

---

_Verified: 2026-07-15_
_Verifier: Claude (gsd-verifier)_
