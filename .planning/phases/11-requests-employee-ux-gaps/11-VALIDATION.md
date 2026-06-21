---
phase: 11
slug: requests-employee-ux-gaps
status: approved
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-21
---

# Phase 11 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source: 11-RESEARCH.md `## Validation Architecture`. Frontend (render/toast/notification/dropdown) is MANUAL by design — no FE test runner in scope (CONTEXT deferred ideas).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` (cargo); FE runner out of scope |
| **Config file** | none for FE; Rust — Cargo workspace |
| **Quick run command** | `cargo test -p trackly-app -p trackly-infra -p trackly-core <module>` |
| **Full suite command** | `cargo test` (one at a time — MEMORY: no concurrent cargo test) |
| **Estimated runtime** | ~60–120 seconds (full workspace) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p <crate> <module>` (the touched module)
- **After every plan wave:** Run `cargo test` (full suite, single process)
- **Before `/gsd-verify-work`:** Full `cargo test` green + manual check of all 3 findings in LAN browser (after `pnpm --dir ui build`) and in desktop mode
- **Max feedback latency:** ~120 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 11-01-01 | 01 | 1 | D-CAT-01 | — | N/A (read JOIN; param query, no user input in SQL) | unit | `cargo test -p trackly-infra requests_sqlite` | ❌ W0 (extend existing request-repo test) | ⬜ pending |
| 11-01-02 | 01 | 1 | D-CAT-01 | — | free_form без категории → прочерк, никогда null/число | manual | — (render) | ✅ | ⬜ pending |
| 11-02-01 | 02 | 2 | D-PRN-01 | T-11-02-E / T-11-02-I | endpoint gated `Action::CreateRequest` (NOT ReadData/ReadPrinters); DTO strictly `{id,name,location}` — no snmp/community/ip | integration | `cargo test -p trackly-app request_printer_options` | ❌ W0 (extend Phase 5/10 role×endpoint matrix) | ⬜ pending |
| 11-02-02 | 02 | 2 | D-PRN-01 | — | N/A (UI dropdown grouped by location) | manual | — (dropdown visual) | ✅ | ⬜ pending |
| 11-03-01 | 03 | 3 | D-WS-01 | T-11-03-I / T-11-03-E | `is_visible_to(RequestStatusChanged)`: author-employee→true, other-employee→false, admin/manager→true; payload `requested_by_user_id` set at all 3 send-sites | unit | `cargo test -p trackly-app ws_event_visibility request_service` | ❌ W0 (new test in dto/printer.rs) | ⬜ pending |
| 11-03-02 | 03 | 3 | D-WS-01 | — | permission requested gently (after first submit), not on load; secure-context guard → degrade to toast | manual | — (toast/notification, Page Visibility) | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/trackly-app/src/dto/printer.rs` — `is_visible_to` test for `RequestStatusChanged` (3 role/authorship cases) + NewRequest regression case
- [ ] `crates/trackly-infra/src/repos/requests_sqlite.rs` — extend test: `category_name` for a request with and without a category
- [ ] role×endpoint matrix (Phase 5/10) — add the new printer-picker endpoint (employee 200, minimal-DTO JSON-shape assertion forbidding snmp/community/ip/serial)
- [ ] `bindings.ts` regeneration verified by existing `tests/export_bindings.rs` (runs under `cargo test`)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Деталь заявки показывает имя категории, free_form → прочерк | D-CAT-01 | UI render, no FE runner | Открыть деталь заявки с категорией → видно название; открыть free_form-заявку → прочерк, не число/null |
| Employee видит тост по СВОЕЙ заявке; ничего по чужой | D-WS-01 | Live WS + 2 browser sessions | Войти employee A и employee B; админ меняет статус заявки A → тост только у A |
| document.hidden → системная нотификация; иначе тост; degrade на HTTP | D-WS-01 | Page Visibility + secure-context, runtime-dependent | Свернуть вкладку employee, сменить статус → системная нотификация (на `:8443`); на HTTP-fallback → тост |
| Permission запрашивается деликатно после первой отправки | D-WS-01 | Browser permission prompt | Первая успешная отправка заявки → один промпт; при `denied` не повторять |
| Дропдаун принтеров сгруппирован по Расположению (серый заголовок) | D-PRN-01 | UI render, no FE runner | Открыть форму заявки employee → непустой список, группы по Расположению с серой полоской-заголовком; пустой список → корректное состояние |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (backend) / explicit manual steps (frontend, by design)
- [x] Sampling continuity: no 3 consecutive backend tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-06-21
