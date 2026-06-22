---
phase: 12
slug: cartridge-request-interconnection
status: draft
nyquist_compliant: false
wave_0_complete: false
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

> Planner fills exact task IDs. Seams identified by research (12-RESEARCH.md §Validation Architecture):

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 12-01-xx | 01 | 1 | D-01/D-02 (installable+model filter) | — | List returns only status=На складе AND charge ∈ {1,2} AND model=request model | integration | `cargo test -p trackly-app cartridge` | ❌ W0 | ⬜ pending |
| 12-0x-xx | 0x | x | D-06 (completed_cartridge_id link) | — | install→complete persists completed_cartridge_id = chosen cartridge | integration | `cargo test -p trackly-app request_cart_link` | ❌ W0 (test_req_cart_link stub) | ⬜ pending |
| 12-0x-xx | 0x | x | RBAC (employee cannot install) | T-12-01 | Employee transition/install → 403 on both transports | integration | `cargo test rbac` (role×endpoint matrix) | ✅ existing | ⬜ pending |
| 12-0x-xx | 0x | x | D-05 (printer location on RequestDto) | — | RequestDto exposes printer_location via locations JOIN | integration | `cargo test -p trackly-app request` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Activate/implement `test_req_cart_link` (currently `#[ignore]` stub at `phase06_stubs.rs:578`) — covers D-06 install→complete linkage
- [ ] Add integration test scaffolds for installable-cartridge filter (status + charge + model) — RED first
- [ ] Add test scaffold asserting RequestDto carries printer location (JOIN) — RED first

*Existing role×endpoint matrix covers RBAC; reuse it for the install/transition employee-deny case.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Selector renders, prefilled location/requester are editable, install completes request | D-01..D-08 (UX) | Svelte UI interaction in Tauri webview + LAN browser | Создать заявку «Замена картриджа» от сотрудника → принять в работу → «Установить картридж» → выбрать совместимый картридж со склада → проверить предзаполнение Расположения и «Кому отдал» (редактируемы) → установить → заявка завершена, в истории виден код+модель установленного картриджа |
| Empty state when no compatible stock cartridge | DISC-02 | Visual/UX | Заявка на модель без подходящих картриджей → понятное пустое состояние, без блокировки |

---

## Validation Sign-Off

- [ ] All tasks have automated verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (installable filter, link test, printer-location read)
- [ ] No watch-mode flags
- [ ] Feedback latency < 240s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
