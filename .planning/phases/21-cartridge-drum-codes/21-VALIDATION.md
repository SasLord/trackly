---
phase: 21
slug: cartridge-drum-codes
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-07-15
---

# Phase 21 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Reconstructed retroactively from phase artifacts (State B: no prior VALIDATION.md, SUMMARY present).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (workspace), inline `#[cfg(test)]` + `tests/` integration crates |
| **Config file** | `Cargo.toml` (workspace root) — no separate test-runner config |
| **Quick run command** | `cargo test -p trackly-infra --lib assign_code` |
| **Full suite command** | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test cartridges_numbering -- --test-threads=1` |
| **Estimated runtime** | ~10–30 seconds (single crate/target; full workspace build cold ~minutes) |

> Note: run one `cargo test` at a time (project convention — concurrent runs contend on the `target/` lock and look like a multi-minute hang).

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p trackly-infra --lib assign_code`
- **After every plan wave:** Run the full `cartridges_numbering` integration command above
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** ~30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 21-01-01 | 01 | 1 | CRT-01 | T-21-01 / T-21-02 (both accept) | Auto-code stays server-generated; retry-loop re-checks UNIQUE before returning any candidate; `code_override` UNIQUE-validation path unchanged | unit | `cargo test -p trackly-infra --lib assign_code_auto_increments` | ✅ | ✅ green |
| 21-01-01 | 01 | 1 | CRT-01 | — | Drum uses separate `drum_seq` counter, `D-` prefix, exact `D-0001`/`D-0002` | unit | `cargo test -p trackly-infra --lib assign_code_drum_uses_d_prefix_and_separate_counter` | ✅ | ✅ green |
| 21-01-01 | 01 | 1 | CRT-01 | T-21-02 | Custom `code_override` round-trips unchanged (branch untouched) | unit | `cargo test -p trackly-infra --lib assign_code_custom_roundtrip` | ✅ | ✅ green |
| 21-01-01 | 01 | 1 | CRT-01 | T-21-01 | 50 concurrent real-service creates → 50 unique codes, `C-` + all-digit suffix (via real `CartridgeService::create` path) | integration | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test cartridges_numbering concurrent_50_unique_codes -- --test-threads=1` | ✅ | ⚠️ green (weak assertion — see Manual-Only / WR-02) |
| 21-01-01 | 01 | 1 | CRT-01 | T-21-01 | Sequential creates → monotonically increasing numeric suffix (counter never rolled back) | integration | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test cartridges_numbering collision_retry_does_not_lose_counter -- --test-threads=1` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky/weak*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. The two files this phase modified (`crates/trackly-infra/src/repos/cartridges_sqlite.rs`, `crates/trackly-app/tests/cartridges_numbering.rs`) already carry the inline unit tests and integration tests; no framework install or new stub files were needed.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Integration test detects a regression back to the old 6-digit format | CRT-01 | **WR-02** (user-elected `Skip — manual-only`, 2026-07-15). The flagship integration test `concurrent_50_unique_codes` asserts only `code.len() >= 6` (lower bound); `C-000001` satisfies this identically to `C-0001`, so this test cannot observe a regression to `{seq:06}`. The exact-width guard exists — via the inline unit tests that pin `C-0001`/`D-0001` exactly — but not in the integration test nominally responsible. Not filled by choice; recorded as a known limitation. | If touching auto-code format, confirm the exact format at unit level: `cargo test -p trackly-infra --lib assign_code_auto_increments` (must assert `C-0001`, `C-0002`) and `...drum_uses_d_prefix...` (must assert `D-0001`). To strengthen the integration test opportunistically: add `assert_eq!(code[2..].len(), 4)` for the `seq < 10000` codes it produces. |
| Live-upgrade mixed-width display (old 6-digit rows + continuing counter → new 4-digit codes) | CRT-01 | **IN-02** — cosmetic only. No behavioral test exercises a pre-existing DB whose `cartridge_seq`/`drum_seq` counters are already advanced past a 6-digit-era value; uniqueness is guaranteed by the retry-loop regardless. Backfill/renumber is out of scope. | On an upgraded DB, create a new cartridge without `code_override`; confirm the new code is accepted and unique alongside older `C-000xxx` codes (mixed width is expected, not a defect). |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies — CRT-01 covered by 3 unit + 2 integration tests
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (single-task phase, fully automated)
- [x] Wave 0 covers all MISSING references — none MISSING; existing infra sufficient
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter — exact-format behavior of CRT-01 is pinned by automated unit tests

**Approval:** approved 2026-07-15

---

## Validation Audit 2026-07-15

| Metric | Count |
|--------|-------|
| Gaps found | 1 (WR-02 — integration-test regression-guard weakness) |
| Resolved | 0 |
| Escalated / manual-only | 1 (user-elected skip; recorded as known limitation) |

**Verdict:** Nyquist-compliant. CRT-01's exact-format behavior is verified by automated unit tests (`assign_code_auto_increments`, `assign_code_drum_uses_d_prefix_and_separate_counter`) that pin `C-0001`/`D-0001` and would fail on a regression to `{seq:06}`. The one open gap (WR-02) is a test-quality caveat on the integration test, not a coverage gap — the requirement remains guarded by a different, exact-value test.
