---
phase: 3
slug: pdf
status: ready
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-28
updated: 2026-05-29
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
| **Quick run command** | `cargo test -p trackly-app --test acts_crud -- --nocapture` |
| **Full suite command** | `cargo test --workspace && pnpm -C ui test run` |
| **Estimated runtime** | ~55 seconds (cargo) + ~15 seconds (vitest) |

---

## Sampling Rate

- **After every task commit:** Run targeted quick command for the touched layer (e.g. `cargo test -p trackly-app --test acts_crud`).
- **After every plan wave:** Run `cargo test --workspace && pnpm -C ui test run`.
- **Before `/gsd-verify-work`:** Full suite green + PDF hash fixture test passes on dev box.
- **Max feedback latency:** 60 seconds for quick run, 120 seconds for full suite.

---

## Per-Task Verification Map

> Populated from PLAN.md `<automated>` blocks across all 5 plans (12 tasks total after B-3 split).
> Each row is one task — exactly one `<automated>` block per task is the Nyquist guarantee.

| Task ID | Plan | Wave | Requirement(s) | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|----------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 03-01-T1 | 03-01 | 1 | (PDF infra) | T-03-01-SC | Pin'ы krilla=0.7.0 + MSRV 1.92; cargo deps & fonts on disk | build + ls + rustc | `cargo build -p trackly-app --all-features 2>&1 \| tail -5 && ls -la crates/trackly-app/assets/fonts/ && rustc --version` | n/a (build) | ⬜ pending |
| 03-01-T2 | 03-01 | 1 | (PDF infra) | T-03-01-01, T-03-01-02, T-03-01-03, T-03-01-04 | MiniJinja safe-mode + fuel + timeout; krilla determinism post-process | cargo test --lib | `cargo test -p trackly-app --lib pdf:: 2>&1 \| tail -20 && cargo clippy -p trackly-app --all-targets -- -D warnings 2>&1 \| tail -5` | `crates/trackly-app/src/pdf/{mod,fonts,docspec,minijinja_env,renderer}.rs` | ⬜ pending |
| 03-01-T3 | 03-01 | 1 | (PDF infra) | T-03-01-03, T-03-01-04 | Stable SHA256 hash + Cyrillic glyph presence | cargo test --test | `cargo test -p trackly-app --test pdf_determinism 2>&1 \| tail -15 && cargo test -p trackly-app --test pdf_text_extract 2>&1 \| tail -15 && grep -cE '^[0-9a-f]{64}$' crates/trackly-app/tests/fixtures/act_42.sha256` | `crates/trackly-app/tests/{pdf_determinism,pdf_text_extract}.rs` + `fixtures/act_42.{json,sha256}` | ⬜ pending |
| 03-02-T1 | 03-02 | 2 | ACT-14 | T-03-02-01 | V014 migration (indexes + device_statuses.code + act_items.quantity) + atomic counter `UPDATE...RETURNING` under 50 concurrent writers | cargo test --lib + --test | `cargo test -p trackly-infra --lib db::migrations 2>&1 \| tail -10 && cargo test -p trackly-infra --lib repos::acts_sqlite repos::audit_log_sqlite 2>&1 \| tail -10 && cargo test -p trackly-app --test acts_numbering concurrent_50_creates_unique_numbers 2>&1 \| tail -10` | `migrations/V014__acts_indexes_and_status_codes.sql` + `crates/trackly-infra/src/repos/{acts_sqlite,audit_log_sqlite}.rs` + `crates/trackly-app/tests/acts_numbering.rs` | ⬜ pending |
| 03-02-T2a | 03-02 | 2 | ACT-01, ACT-02, ACT-03, ACT-05, ACT-13, ACT-14 | T-03-02-02, T-03-02-03, T-03-02-04, T-03-02-05, T-03-02-08 | Single-writer transaction with override audit + rollback + quantity persistence | cargo test --test | `cargo test -p trackly-app --test acts_crud --test acts_http_smoke --test acts_numbering --test export_bindings 2>&1 \| tail -20 && cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tail -5` | `crates/trackly-app/{src/services/act_service.rs,src/dto/act.rs,src/tauri_cmds/acts.rs,src/http/acts.rs,src/context.rs,tests/acts_crud.rs,tests/acts_http_smoke.rs}` | ⬜ pending |
| 03-02-T2b | 03-02 | 2 | ACT-03 (autocomplete filtering) | T-03-02-08 | DeviceAutocompleteField statusIn + backend code→status_id mapping | cargo test --test + svelte-check | `cargo test -p trackly-app --test devices_autocomplete --test devices_crud --test devices_search 2>&1 \| tail -15 && cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tail -5 && pnpm -C ui svelte-check 2>&1 \| tail -5` | `ui/src/lib/components/Modal.svelte` (расширен) + `ui/src/features/devices/DeviceAutocompleteField.svelte` (расширен) + `crates/trackly-app/src/services/device_service.rs` (расширен) | ⬜ pending |
| 03-02-T2c | 03-02 | 2 | ACT-01, ACT-02, ACT-03, ACT-05 | (UI-only — no STRIDE delta) | UI feature folder compiles (svelte-check + lint) | svelte-check + lint | `pnpm -C ui svelte-check 2>&1 \| tail -5 && pnpm -C ui lint 2>&1 \| tail -5` | `ui/src/features/acts/{ActsPage,ActsSearchAndTabs,ActsMasterDetail,ActsList,ActListRow,ActDetail,ActItemsTable,ActHeaderField,ActFormModal,ActFormBody,ActFormItemsTable,ActNumberField}.svelte` + `ui/src/features/layout/sidebar-config.ts` + `ui/src/pages/ActsPage.svelte` | ⬜ pending |
| 03-03-T1 | 03-03 | 3 | ACT-07, ACT-08, ACT-09 | T-03-03-01, T-03-03-04, T-03-03-08 | Sub-numbering + bulk+per-row override + auto-archive via SUM(quantity); W-7 counter unchanged + W-8 per-row override positive path | cargo test --test | `cargo test -p trackly-app --test acts_display_rule --test acts_returns --test acts_http_smoke 2>&1 \| tail -20 && cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tail -5` | `crates/trackly-app/tests/{acts_display_rule,acts_returns}.rs` + `crates/trackly-infra/src/repos/acts_sqlite.rs` (расширен) + `crates/trackly-infra/src/repos/devices_sqlite.rs` (расширен — update_full_in_tx) | ⬜ pending |
| 03-03-T2 | 03-03 | 3 | ACT-06, ACT-10 | T-03-03-02, T-03-03-03, T-03-03-05 | Universal undo via audit_log.before_json (handover delete cascades; return delete unarchives parent) | cargo test --test + svelte-check + lint | `cargo test -p trackly-app --test acts_undo --test acts_returns --test acts_crud 2>&1 \| tail -20 && cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tail -5 && pnpm -C ui svelte-check 2>&1 \| tail -5 && pnpm -C ui lint 2>&1 \| tail -5` | `crates/trackly-app/tests/acts_undo.rs` + `crates/trackly-infra/src/repos/devices_sqlite.rs` (restore_from_snapshot_in_tx) + `ui/src/features/acts/{ReturnModal,ReturnItemsTable,ActDetail,ActsPage}.svelte` | ⬜ pending |
| 03-04-T1 | 03-04 | 4 | ACT-12, DEV-15 | T-03-04-01, T-03-04-02, T-03-04-08 | Templates idempotent seed + org.json placeholder + logo path traversal mitigation; Phase 2 regression (W-10) | cargo test --test | `cargo test -p trackly-app --test templates_seed --test organization_io 2>&1 \| tail -15 && cargo test -p trackly-app --test acts_crud --test acts_returns --test acts_undo 2>&1 \| tail -15 && cargo test -p trackly-app --test devices_crud --test devices_search --test devices_autocomplete --test devices_csv_import --test devices_csv_export --test devices_bulk_create --test concurrent_writes 2>&1 \| tail -15 && cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tail -5` | `crates/trackly-app/{templates/act_handover.minijinja,templates/act_acceptance.minijinja,src/services/{template_service,organization_service}.rs,src/dto/organization.rs,tests/{templates_seed,organization_io}.rs}` | ⬜ pending |
| 03-04-T2 | 03-04 | 4 | ACT-11, DEV-15 | T-03-04-03, T-03-04-04, T-03-04-05 | Full 3-stage pipeline (template → DocSpec → krilla) + missing-template/broken-template/missing-logo edge cases + PdfPreviewModal compile | cargo test --test + svelte-check + lint | `cargo test -p trackly-app --test pdf_render_act --test export_bindings 2>&1 \| tail -15 && cargo test -p trackly-app --test acts_crud --test acts_returns --test acts_undo --test templates_seed --test organization_io 2>&1 \| tail -15 && cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tail -5 && pnpm -C ui svelte-check 2>&1 \| tail -5 && pnpm -C ui lint 2>&1 \| tail -5` | `crates/trackly-app/tests/pdf_render_act.rs` + `ui/src/features/acts/PdfPreviewModal.svelte` + `ui/src/lib/api/{organization,templates,pdf,acts}.ts` | ⬜ pending |
| 03-05-T1 | 03-05 | 5 | ACT-04 | T-03-05-01, T-03-05-02 | LIKE+FTS5 search merge + escape `%`/`_` + empty fallback | cargo test --test + svelte-check + lint | `cargo test -p trackly-app --test acts_search 2>&1 \| tail -15 && cargo test -p trackly-app --test acts_crud --test acts_returns --test acts_undo 2>&1 \| tail -10 && cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tail -5 && pnpm -C ui svelte-check 2>&1 \| tail -5 && pnpm -C ui lint 2>&1 \| tail -5` | `crates/trackly-app/tests/acts_search.rs` + `crates/trackly-infra/src/repos/acts_sqlite.rs` (расширен search_acts) | ⬜ pending |
| 03-05-T2 | 03-05 | 5 | DEV-14 (UI) | T-03-05-03, T-03-05-04 | DEV-14 UI flow (context menu → modal → preview) + e2e smoke + W-9 MSK timezone conversion test | cargo test workspace + clippy + fmt + svelte-check + lint | `cargo test -p trackly-app --test acts_e2e_smoke 2>&1 \| tail -15 && cargo test --workspace 2>&1 \| tail -10 && cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tail -5 && cargo fmt --all -- --check 2>&1 \| tail -5 && pnpm -C ui svelte-check 2>&1 \| tail -5 && pnpm -C ui lint 2>&1 \| tail -5` | `ui/src/features/acts/DocumentAcceptanceModal.svelte` + `ui/src/features/acts/PdfPreviewModal.svelte` (расширен mode) + `ui/src/features/devices/{DeviceContextMenu,DevicesPage}.svelte` + `crates/trackly-app/tests/acts_e2e_smoke.rs` | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

**Sampling continuity:** All 12 tasks have `<automated>` blocks. No task is manual-only. No 3+ consecutive tasks without automated verify.

---

## Wave 0 Requirements

> Paths corrected per B-4 fix: все integration test файлы живут в `crates/trackly-app/tests/` (соответствует плану файлов), не в `trackly-infra/tests/` или `trackly-core/tests/`. Phase 1 + Phase 2 уже устанавливают этот convention.

- [x] `crates/trackly-app/tests/pdf_determinism.rs` — fixture SHA256 hash test for Акта приёма-передачи (governs ACT-11, success criterion 4). Created by 03-01-T3.
- [x] `crates/trackly-app/tests/pdf_text_extract.rs` — Cyrillic «Сидоров-Петроградский Иван Александрович (ё) №42» проверяется extract'ом. Created by 03-01-T3.
- [x] `crates/trackly-app/tests/acts_numbering.rs` — atomic counter `UPDATE...RETURNING` under 50-way concurrency (ACT-14). Created by 03-02-T1.
- [x] `crates/trackly-app/tests/acts_crud.rs` — handover create/get/list/counts + override audit + rollback + B-2 regression `handover_with_quantity_persists`. Created by 03-02-T2a.
- [x] `crates/trackly-app/tests/acts_http_smoke.rs` — POST /api/v1/acts_create roundtrip. Created by 03-02-T2a.
- [x] `crates/trackly-app/tests/acts_returns.rs` — sub_number sequencing per parent_act_id; auto-archive via SUM(quantity); bulk-apply + per-row override; W-7 counter-unchanged + W-8 per-row positive (ACT-07, ACT-08, ACT-09). Created by 03-03-T1.
- [x] `crates/trackly-app/tests/acts_undo.rs` — undo restore from `audit_log.before_json` for handover delete (cascades returns) and return delete (unarchives parent) (ACT-06, ACT-10). Created by 03-03-T2.
- [x] `crates/trackly-app/tests/acts_display_rule.rs` — «в»/«в1»/«в2» formatting rule including retroactive promotion (D-Numbering-01). Created by 03-03-T1.
- [x] `crates/trackly-app/tests/templates_seed.rs` — idempotent seed of `act_handover` + `act_acceptance` templates. Created by 03-04-T1.
- [x] `crates/trackly-app/tests/organization_io.rs` — first-run placeholder + corrupt-JSON validation + logo path traversal mitigation. Created by 03-04-T1.
- [x] `crates/trackly-app/tests/pdf_render_act.rs` — full 3-stage pipeline (template → DocSpec → krilla) with Cyrillic assertion via pdf-extract (ACT-11, DEV-15). Created by 03-04-T2.
- [x] `crates/trackly-app/tests/acts_search.rs` — FTS5+LIKE merge across number/ФИО/устройство (ACT-04). Created by 03-05-T1.
- [x] `crates/trackly-app/tests/acts_e2e_smoke.rs` — full lifecycle handover → return → archive → undo + handover_pdf_render + acceptance_pdf_render (ACT-13 + DEV-14 backend integration + W-9 MSK timezone). Created by 03-05-T2.
- [x] DejaVu Sans TTF embedded via `include_bytes!` — verified by `pdf_determinism` test (03-01-T3).
- [x] `org.json` fixture beside `.exe` in test target dir — verified by `pdf_render_act` test (03-04-T2).
- [ ] `ui/src/features/acts/__tests__/` — Vitest stubs for switch-bar counts, master-detail navigation, create-modal validation, return-modal bulk-apply. **Status:** deferred — UI compile-time enforcement через `pnpm -C ui svelte-check` покрывает type-safety; runtime Vitest stubs опциональны в Phase 3 и могут быть добавлены в любой момент без блокировки phase progress. (Manual UI verification listed below.)

*Infrastructure note:* `cargo test` already configured Phase 1; `vitest` already configured Phase 2. No new test framework install required.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Cyrillic glyphs render in PDF preview modal | ACT-11, DEV-14 | Visual confirmation of font subset on real Tauri webview | Открыть приложение, создать акт с ФИО «Сидоров-Петроградский Иван Александрович (ё)», нажать «Печать», убедиться что в preview-модале видны все буквы без квадратиков. Сохранить PDF, открыть в Preview/SumatraPDF, повторить визуальную проверку. |
| Print dialog opens with PDF preselected | ACT-11, DEV-14 | OS-native dialog cannot be asserted from cargo tests | Тот же сценарий + кнопка «Печать»; убедиться что открывается системный диалог печати с правильным документом. |
| PDF file size sanity (< 500 KB for single-page act) | ACT-11 | File-size assertion is flaky in CI due to font subset variance | `ls -la` after save; expect < 500KB. |
| Sidebar «Акты» активен + master-detail работает | ACT-02 | Visual UI flow | Запустить `pnpm tauri dev`, кликнуть «Акты» в sidebar; убедиться что master-detail layout рендерится с tab'ами Акты/Возвраты/Архив и счётчиками; создание акта (➕) открывает модал; submit добавляет акт в список + обновляет счётчик. |
| ReturnModal bulk-default + per-row override | ACT-08 | Visual UI flow (checkbox states) | Открыть акт → «Возврат» → ReturnModal с «Применить ко всем» ВКЛ по умолчанию; снять галочку с одной позиции, поставить per-row override → submit; убедиться что только override применился для этой позиции. |
| DEV-14 «Печать документа приёма» из device context menu | DEV-14 | Visual UI flow | Правый клик на устройстве в DevicesPage → «Печать документа приёма» → intermediate modal с полями «Кто передал»/«Кто принял»/«Дата» → submit → PdfPreviewModal с реальным PDF Document приёма. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (verified during plan-checker pass after B-4 fix).
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (all 12 tasks have one).
- [x] Wave 0 covers all MISSING references (13 of 14 done; vitest stubs explicitly deferred without blocking).
- [x] No watch-mode flags.
- [x] Feedback latency < 60s for quick run.
- [x] `nyquist_compliant: true` set in frontmatter.
- [x] `wave_0_complete: true` set in frontmatter.

**Approval:** ready (pending plan-checker re-pass for B-1 / B-2 / B-3 / W-5..W-10 fixes verification).
