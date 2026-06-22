---
phase: 12
slug: cartridge-request-interconnection
status: planned
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-22
---

# Phase 12 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust: `cargo test` (workspace integration tests); Frontend: `pnpm svelte-check` + `pnpm lint` |
| **Config file** | Cargo workspace (`Cargo.toml`); `ui/` Vite/Svelte config |
| **Quick run command** | `cargo test -p trackly-app cartridge request` |
| **Full suite command** | `cargo test && pnpm --dir ui svelte-check && pnpm --dir ui lint` |
| **Estimated runtime** | ~120–240 seconds (full) |

---

## Sampling Rate

- **After every task commit:** Run quick command (scoped cargo test for touched crate)
- **After every plan wave:** Run full suite
- **Before `/gsd-verify-work`:** Full suite green + `bindings*.ts` regenerated if DTO changed
- **Max feedback latency:** ~240 seconds

> Note (project memory): run only ONE `cargo test` at a time — concurrent runs contend on the `target/` lock and look like a multi-minute hang.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 12-01-T1 | 01 | 1 | D-01/D-02 (installable+model filter) | T-12-01 | List returns only status=На складе AND charge ∈ {1,2} AND model=request model | integration (tdd) | `cargo test -p trackly-app --test cartridges_lifecycle installable` | ✅ created (RED→GREEN in-task) | ⬜ pending |
| 12-01-T2 | 01 | 1 | D-05 (printer location on RequestDto) | — | RequestDto exposes printer_location via LEFT JOIN locations, NULL-safe | integration (tdd) | `cargo test -p trackly-app --test phase06_stubs printer_location` | ✅ created (RED→GREEN in-task) | ⬜ pending |
| 12-02-T1 | 02 | 2 | D-06 (completed_cartridge_id link) / D-07 (history snapshot) | — | Complete{linked_cartridge_id} persists completed_cartridge_id + enriches audit notes with code+model | integration (tdd) | `cargo test -p trackly-app --test phase06_stubs cart_link` / `history_` | ✅ created (test_req_cart_link de-ignored) | ⬜ pending |
| 12-02-T2 | 02 | 2 | RBAC (employee cannot install / transition) | T-12-01 | Employee → cartridges_transition / requests_transition → 403 on HTTP transport | integration | `cargo test -p trackly-app --test role_endpoint_matrix role_endpoint_matrix_test` | ✅ existing matrix + 2 new cases | ⬜ pending |
| 12-03-T1 | 03 | 3 | D-01/D-02/D-03 (selector component) | — | CartridgeSelect renders flat list + empty state | type-check | `pnpm --dir ui svelte-check` | ✅ created | ⬜ pending |
| 12-03-T2 | 03 | 3 | D-01..D-05/D-08 (modal selector + prefill + dual-entry) | — | OperationModal supports both cartridge-centric and request-centric install entry | type-check + build | `pnpm --dir ui svelte-check && pnpm --dir ui build` | ✅ modified | ⬜ pending |
| 12-03-T3 | 03 | 3 | D-06 (linkedCartridgeId wiring) | — | RequestDetail passes real cartridgeId to complete instead of null | type-check + build | `pnpm --dir ui svelte-check && pnpm --dir ui build` | ✅ modified | ⬜ pending |
| 12-03-T4 | 03 | 3 | D-01..D-08 (end-to-end UX + D-08 regression) | — | Manual happy path + empty state (DISC-02) + old cartridge-centric entry unchanged | manual (checkpoint:human-verify) | n/a — see `<how-to-verify>` in 12-03-PLAN.md | ✅ checkpoint task | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements (satisfied via task-level TDD, no separate Wave 0 plan)

- [x] `test_req_cart_link` (`phase06_stubs.rs`, previously `#[ignore]`) — activated in 12-02-T1 as real `#[tokio::test]`, covers D-06 install→complete linkage
- [x] Integration test scaffolds for installable-cartridge filter (status + charge + model) — written RED-first inside 12-01-T1 (`tdd="true"`, 4 tests)
- [x] Test scaffold asserting RequestDto carries printer location (JOIN) — written RED-first inside 12-01-T2 (`tdd="true"`, 2 tests)

*Existing role×endpoint matrix (`role_endpoint_matrix.rs`) covers RBAC; extended with 2 new cases in 12-02-T2 for the install/transition employee-deny gap (T-12-01).*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Selector renders, prefilled location/requester are editable, install completes request | D-01..D-08 (UX) | Svelte UI interaction in Tauri webview + LAN browser | См. `<how-to-verify>` в 12-03-PLAN.md Task 4 (checkpoint:human-verify) — полный сценарий заявка → принять → установить картридж → проверить предзаполнение → завершение → история |
| Empty state when no compatible stock cartridge | DISC-02 | Visual/UX | Заявка на модель без подходящих картриджей → "Нет подходящих картриджей на складе", форма не блокируется |
| D-08 regression: old cartridge-centric entry unchanged | D-08 | Visual/UX | Прямой вход через карточку картриджа → «Установить в принтер» — без нового селектора, форма работает как раньше |

---

## Validation Sign-Off

- [x] All tasks have automated verify or Wave 0 dependencies (Task 4/12-03 is the sole manual checkpoint, explicitly typed `checkpoint:human-verify`)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (only the final UX checkpoint is manual, preceded by 6 automated tasks)
- [x] Wave 0 covers all MISSING references (installable filter, link test, printer-location read) — satisfied via task-level TDD in 12-01-T1/T2 and 12-02-T1
- [x] No watch-mode flags
- [x] Feedback latency < 240s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** auto-approved by planner — task-level TDD (tdd="true" + `<behavior>` blocks) in 12-01/12-02 satisfies the Wave 0 RED-first requirement without a dedicated Wave 0 plan, since each scaffold is created and turned green within the same task that implements the feature (no plan ships with a permanently-red test).
