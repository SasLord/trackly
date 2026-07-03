---
phase: 15
slug: render-word-fidelity
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-07-04
---

# Phase 15 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust builtin; `cargo nextest` optional) |
| **Config file** | `crates/trackly-app/Cargo.toml` (`[[test]]` integration tests in `crates/trackly-app/tests/`) |
| **Quick run command** | `cargo test -p trackly-app --test pdf_render_act` |
| **Full suite command** | `cargo test -p trackly-app` |
| **Estimated runtime** | ~60–180 seconds (bundled SQLite + krilla compile amortized) |

> NOTE: one `cargo test` at a time — concurrent runs contend on the `target/` lock and look like a multi-minute hang.

---

## Sampling Rate

- **After every task commit:** Run the relevant focused `--test <file>` (e.g. `pdf_render_act`, `pdf_column_overflow`, `pdf_logo`)
- **After every plan wave:** Run `cargo test -p trackly-app`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** ~180 seconds

---

## Per-Task Verification Map

> Populated by the planner / gsd-nyquist-auditor from PLAN.md tasks. Each task must map to an
> automated `cargo test` assertion or declare a Wave 0 dependency.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | TBD | TBD | PDFA-01/02/05/07/08 | — | N/A (local doc render) | integration | `cargo test -p trackly-app --test pdf_render_act` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Extend `crates/trackly-app/tests/pdf_render_act.rs` — assert new block order + intro + «Выдал/Получил» labels (PDFA-01, PDFA-05)
- [ ] Extend `crates/trackly-app/tests/pdf_column_overflow.rs` — assert long fields (Комплектация/Тех.характеристики) wrap without truncation (PDFA-02)
- [ ] New multi-device test (1 vs N positions) — assert all positions render (PDFA-02)
- [ ] `crates/trackly-app/tests/pdf_logo.rs` — assert logo sourced from org_settings BLOB (PDFA-01 / D-08)
- [ ] Regenerate `pdf_determinism.rs` pinned-hash fixture (`act_42.sha256`) as an explicit planned step (expected break, not a regression)

*Existing PDF test infrastructure covers the harness; new assertions extend it.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Visual fidelity to Word sample (spacing/weights/overall look) | PDFA-01 | Pixel-level "looks like the sample" is subjective; automated tests assert content/structure, not aesthetic match | Generate an act PDF, compare side-by-side with `.planning/reference/act-word-source/act-sample.docx` |

*All structural/content behaviors have automated verification; only aesthetic match is manual.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 180s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
