---
phase: 3
slug: pdf
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-28
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source of truth for **what** is validated lives in `03-RESEARCH.md` → "## Validation Architecture".

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust) + `vitest` (Svelte) — established Phase 1/2 |
| **Config file** | `Cargo.toml` workspace test config; `ui/vitest.config.ts` |
| **Quick run command** | `cargo test -p trackly-core acts:: -- --nocapture` |
| **Full suite command** | `cargo test --workspace && pnpm -C ui test run` |
| **Estimated runtime** | ~45 seconds (cargo) + ~15 seconds (vitest) |

---

## Sampling Rate

- **After every task commit:** Run targeted quick command for the touched layer (e.g. `cargo test -p trackly-core acts::numbering::`)
- **After every plan wave:** Run `cargo test --workspace && pnpm -C ui test run`
- **Before `/gsd-verify-work`:** Full suite green + PDF hash fixture test passes on dev box
- **Max feedback latency:** 60 seconds for quick run, 120 seconds for full suite

---

## Per-Task Verification Map

> Populated by the planner from PLAN.md `<automated>` blocks after planning completes.
> Each task in PLAN.md MUST emit a row here, or be listed under "Manual-Only Verifications" below.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| _to be filled from PLAN.md after planning_ |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/trackly-app/tests/pdf_determinism.rs` — fixture SHA256 hash test for Акт + Документ приёма (governs ACT-11, DEV-14, success criterion 4)
- [ ] `crates/trackly-infra/tests/acts_numbering.rs` — atomic counter UPDATE...RETURNING under concurrency (ACT-14)
- [ ] `crates/trackly-infra/tests/acts_returns.rs` — sub_number sequencing per parent_act_id; auto-archive trigger correctness (ACT-07, ACT-09)
- [ ] `crates/trackly-infra/tests/acts_undo.rs` — undo restore from `audit_log.before_json` for handover delete and return delete (ACT-06, ACT-10)
- [ ] `crates/trackly-core/tests/acts_display_rule.rs` — «в»/«в1»/«в2» formatting rule including retroactive promotion (D-Numbering-01)
- [ ] `ui/src/features/acts/__tests__/` — Vitest stubs for switch-bar counts, master-detail navigation, create-modal validation, return-modal bulk-apply (ACT-02, ACT-03, ACT-08)
- [ ] DejaVu Sans TTF embedded via `include_bytes!` — verified by determinism test
- [ ] `org.json` fixture beside `.exe` in test target dir — verified by PDF render test

*Infrastructure note:* `cargo test` already configured Phase 1; `vitest` already configured Phase 2. No new test framework install required.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Cyrillic glyphs render in PDF preview modal | ACT-11, DEV-14 | Visual confirmation of font subset on real Tauri webview | Открыть приложение, создать акт с ФИО «Сидоров-Петроградский Иван Александрович (ё)», нажать «Печать», убедиться что в preview-модале видны все буквы без квадратиков. Сохранить PDF, открыть в Preview/SumatraPDF, повторить визуальную проверку. |
| Print dialog opens with PDF preselected | ACT-11, DEV-14 | OS-native dialog cannot be asserted from cargo tests | Тот же сценарий + кнопка «Печать»; убедиться что открывается системный диалог печати с правильным документом. |
| PDF file size sanity (< 500 KB for single-page act) | ACT-11 | File-size assertion is flaky in CI due to font subset variance | `ls -la` after save; expect < 500KB. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies (verified during plan-checker pass)
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter (flipped by planner after Per-Task table is populated)

**Approval:** pending
