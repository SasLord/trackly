---
phase: 13
slug: per-device-junction-chip-drum-state
status: validated
nyquist_compliant: false
wave_0_complete: true
created: 2026-06-26
---

# Phase 13 — Validation Strategy

> Per-phase validation contract. Reconstructed retroactively from phase artifacts (State B) — no execution-time VALIDATION.md existed.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (two layers: `trackly-infra` repo unit tests in `#[cfg(test)]` modules; `trackly-app` integration tests under `crates/trackly-app/tests/`) |
| **Config file** | none — Cargo workspace; `cargo nextest` available but tests authored for plain `cargo test` |
| **Quick run command** | `cargo test -p trackly-infra --lib` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~60–120 seconds (cold build longer; one `cargo test` at a time — concurrent invocations contend on the `target/` lock) |

**Frontend note:** `ui/` has **no test runner** (no vitest/jest). UI-only behaviors (R5, R6, and the visual side of R3/R4) are intentionally manual-only — see Manual-Only section. Frontend gates are `pnpm exec svelte-check` (type) and `pnpm --dir ui build`.

---

## Sampling Rate

- **After every task commit:** Run the relevant targeted test (`cargo test -p <crate> --lib <name>` or `--test <file> <name>`)
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite green + `cargo clippy --workspace -- -D warnings` + `pnpm exec svelte-check`
- **Max feedback latency:** ~120 seconds

---

## Per-Task Verification Map

Requirements are tracked in phase-local `13-SPEC.md` as SPEC-13-R1..R8 (lightweight spec flow; not in milestone `REQUIREMENTS.md`).

| Req | Source Plan(s) | Behavior | Test Type | Automated Command | File Exists | Status |
|-----|----------------|----------|-----------|-------------------|-------------|--------|
| R1 | 13-01/02/03/08 | V029 `printer_cartridge_models` dropped; full migration chain V1..V32 applies clean | integration | `cargo test -p trackly-infra --test migration_idempotency` | ✅ | ✅ green |
| R1 | 13-01/02/03/08 | Fresh DB boots through V32 (RBAC matrix) | integration | `cargo test -p trackly-app --test role_endpoint_matrix` | ✅ | ✅ green |
| R2 | 13-01/02/05/08 | V005 free-text compatibility narrows cartridge list to linked model | integration | `cargo test -p trackly-app --test cartridges_crud printer_compatib_list_narrows_to_linked_model` | ✅ | ✅ green |
| R2 | 13-01/02/05/08 | Unconfigured device does not narrow (D-05 pass-through) | integration | `cargo test -p trackly-app --test cartridges_crud printer_compatib_unconfigured_device_does_not_narrow` | ✅ | ✅ green |
| R2 | 13-01/02/05/08 | Case-insensitive + trim match against `devices.name` (D-03) | integration | `cargo test -p trackly-app --test cartridges_crud printer_compatib_case_insensitive_match` | ✅ | ✅ green |
| R3 | 13-02/06 | `suggest_compat_printer` returns DISTINCT type_id=2 names, prefix-matched, soft-deleted excluded **(gap filled this audit)** | integration | `cargo test -p trackly-app --test cartridges_crud suggest_compat_printer_returns_distinct_printer_names` | ✅ | ✅ green |
| R4 | 13-01/03/07 | `compatible_model_aggregates` RAW per-status counts (1/3/2), no D-05 pass-through, soft-delete excluded **(gap filled this audit)** | unit | `cargo test -p trackly-infra --lib compatible_model_aggregates_counts_raw_statuses_and_omits_unmatched` | ✅ | ✅ green |
| R4 | 13-01/03/07 | Aggregate command is RBAC-gated (Employee → 403, Case 41) | integration | `cargo test -p trackly-app --test role_endpoint_matrix` | ✅ | ✅ green |
| R7 | 13-04/08 | Kind-aware auto-return: drum (kind=2) → state 5 «Изношенный», not 3 | unit | `cargo test -p trackly-infra --lib auto_return_uses_kind_aware_default_state_for_drum` | ✅ | ✅ green |
| R7 | 13-04/08 | Regular cartridge (kind=1) auto-return keeps state 3 default | unit | `cargo test -p trackly-infra --lib auto_return_keeps_state_3_default_for_regular_cartridge` | ✅ | ✅ green |
| R8 | 13-04 | Printer-list cap removed: 250-printer seed returns all rows above old 200 cutoff | unit | `cargo test -p trackly-infra --lib list_returns_all_printers_above_old_cap` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure (`cargo test` across both crate layers) covers all automatable phase requirements. No new framework needed. Two backend gaps (R3 `suggest_compat_printer`, R4 aggregate-count correctness) were closed retroactively by this validation audit:

- [x] `crates/trackly-app/tests/cartridges_crud.rs` — `suggest_compat_printer_returns_distinct_printer_names` (R3)
- [x] `crates/trackly-infra/src/repos/cartridges_sqlite.rs` — `compatible_model_aggregates_counts_raw_statuses_and_omits_unmatched` (R4)

---

## Manual-Only Verifications

These are UI-rendering behaviors with no frontend test runner; their **backend data sources are automated** (see map above), so only the visual rendering remains manual.

| Behavior | Req | Why Manual | Test Instructions |
|----------|-----|------------|-------------------|
| Cartridge-model form shows exactly ONE «Совместимые принтеры» block; autocomplete dropdown lists DISTINCT printer names and accepts free-text entry («будет сохранено как есть») | R3 (UI) | Svelte render; no UI test runner. Backend `suggest_compat_printer` is automated. | Open «Новая модель»/«Редактирование модели» картриджа → confirm one compat block; type a prefix → dropdown of existing printer names; type an unknown name → free-entry hint; save + reload → value persists. |
| Printer card compatibility block is strictly read-only aggregates (D-07 order, no «Списано», no add/remove controls) | R4 (UI) | Svelte render; backend `compatible_model_aggregates` counts are automated. | Open a printer card → compat block shows «brand model: На складе N, На заправке N, В работе N»; confirm zero edit controls. |
| Printer card device-data block (Инвентарный №, Серийный №, Расположение, Состояние) + «Редактировать» → «Редактирование устройства» dialog (reuses `DeviceFormModal`) | R5 | Pure UI; reuses existing component; no isolated unit-testable backend delta. | Open printer card → four device fields render from `devices` row; click «Редактировать» → DeviceFormModal opens; save → fields refetch and update. |
| Installed cartridge shown as code (C-XXXXXX) + model name, never raw numeric id | R6 | Svelte render of existing `cartridges.get` data. | Open printer card with an installed cartridge → confirm `C-000xxx — brand model`; confirm no internal id shown. |

*Frontend gates that did run green at verification: `pnpm exec svelte-check` (0 errors), `pnpm --dir ui build` (dist produced).*

---

## Validation Sign-Off

- [x] All automatable requirements (R1, R2, R3-backend, R4-backend, R7, R8) have green automated verification
- [x] Sampling continuity: no run of requirements left without automated backend coverage
- [x] Wave 0 gaps closed (R3 suggest, R4 aggregate counts) — both green
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [ ] `nyquist_compliant: true` — **withheld**: R5 and R6 are UI-only with no frontend test runner; their backend data sources are automated but the visual rendering is manual-only. Phase is **VALIDATED (PARTIAL)**.

**Approval:** validated (partial) 2026-06-26 — 8 automated tests green; R5/R6 manual-only by infrastructure constraint (no UI test runner).

---

## Validation Audit 2026-06-26

| Metric | Count |
|--------|-------|
| Requirements (SPEC-13-R1..R8) | 8 |
| Fully automated (backend) | 6 (R1, R2, R3, R4, R7, R8) |
| Manual-only (UI render, no test runner) | 2 (R5, R6) |
| Gaps found | 2 (R3 suggest, R4 aggregate counts) |
| Gaps resolved | 2 |
| Gaps escalated | 0 |

Tests added this audit:
- `crates/trackly-app/tests/cartridges_crud.rs::suggest_compat_printer_returns_distinct_printer_names` (R3)
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs::compatible_model_aggregates_counts_raw_statuses_and_omits_unmatched` (R4)

No implementation files modified; no implementation bugs surfaced.
