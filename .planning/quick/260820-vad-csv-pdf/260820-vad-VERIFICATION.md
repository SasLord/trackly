---
phase: 260820-vad
verified: 2026-08-21T02:55:00Z
status: human_needed
score: 9/9 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Открыть «Отчёты» → домен «Заявки» → проверить 4 вкладки, счётчики, переключение периода, CSV-экспорт и печать/предпросмотр в реальном запущенном приложении (десктоп и LAN-браузер)."
    expected: "4 вкладки (Все/Открытые/В работе/Выполненные) с реальными счётчиками из reports_get_report_counts; PeriodSelector активен и перезагружает список при смене периода; колонки Тип/Статус на русском; «Принтер / Локация» пустая (не тире) для заявок без принтера; CSV и печать показывают те же переведённые значения, что и экран."
    why_human: "Синтетические харнессы (svelte-check/vite build/cargo test) не ловят рантайм-ошибки Svelte 5 рун (см. проектный урок «Compile gates miss Svelte runtime») — компилируемость подтверждена автоматически, живое поведение $effect/$derived в реальном UI не проверялось в рамках этого исполнения."
  - test: "Убедиться, что существующие отчёты «Устройства» и «Картриджи» визуально не изменились после добавления домена «Заявки» (вкладки, счётчики, CSV, печать)."
    expected: "Поведение и вид этих двух доменов идентичны состоянию до задачи."
    why_human: "Регресс на уровне backend/тестов подтверждён (полный `cargo test` зелёный), но визуальный regression в живом UI не покрыт automated gates."
---

# Quick Task 260820-vad: Домен «Заявки» в разделе «Отчёты» Verification Report

**Task Goal:** Добавить домен «Заявки» в раздел Отчёты (рядом с существующими «Устройства» и «Картриджи») — просмотр отчётов по заявкам на экране, экспорт CSV и печать/PDF, по тому же паттерну, что уже работает для существующих доменов.

**Verified:** 2026-08-21T02:55:00Z
**Status:** human_needed (all automated must-haves passed; live UAT still required — executor honestly flagged this as UNVERIFIED)
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Admin/Manager видит третий домен «Заявки» с 4 вкладками (Все/Открытые/В работе/Выполненные) (VAD-01) | VERIFIED | `ui/src/features/reports/ReportSubNav.svelte:53-74` — `REQUEST_REPORTS` (4 entries) + `DOMAINS` includes `{key:'requests', label:'Заявки'}` as 3rd domain. `ReportsPage.svelte:105-120` mirrors the same config. |
| 2 | «Все» без фильтра по статусу — включает `rejected`, нет отдельной вкладки «Отклонённые» | VERIFIED | `report_service.rs::list_requests_all` calls `query_requests_inner(..., None, ...)` (no status filter). Test `report_requests_all_includes_every_status_translated_including_rejected` asserts total==4 including `"Отклонена"`. |
| 3 | Все 4 вкладки периодические по `created_at_utc`, снимков нет | VERIFIED | `tauri_cmds/reports.rs::PERIOD_BASED_REPORT_TYPES` is `[&str;8]` including all 4 `requests_*` keys (line 298-307). `query_requests_inner` filters `r.created_at_utc >= / <=`. `ReportsPage.svelte::isSnapshot()` unaffected (only `in_use`/`in_stock` match) — confirmed no key collision with requests domain. |
| 4 | 6 одинаковых колонок (№, Дата, Тип, Статус, Заявитель, Принтер / Локация) на экране/CSV/печати для всех 4 вкладок | VERIFIED | `tauri_cmds/reports.rs::columns_for`/`column_labels_for` (lines 40-49, 77-79) define the same 6 keys/labels for all 4 `requests_*` types. `ReportsPage.svelte::REQUEST_COLUMNS` (123-130) matches; `COLUMNS_MAP` wires all 4 tab keys to it (184-187). Index-alignment enforced by test `column_labels_for_is_index_aligned_with_columns_for` (passes, includes requests_* keys). |
| 5 | Тип/Статус переведены на русский одинаково на экране/CSV/печати, вычислено один раз на бэкенде; неизвестное значение → raw-ключ, не пустая ячейка | VERIFIED | `translate_request_type`/`translate_request_status` (`report_service.rs:251-273`) implement exact mapping incl. `cancelled → "Отменена"`; unit tests `translate_request_type_known_values`, `translate_request_type_unknown_falls_back_to_raw_key`, `translate_request_status_known_values`, `translate_request_status_unknown_falls_back_to_raw_key` all pass. Integration test `report_requests_csv_export_uses_translated_values_not_raw_enum_keys` confirms CSV output contains translated strings and NOT the raw `cartridge_replace` key. |
| 6 | «Принтер / Локация» пустая (не тире) для заявки без принтера | VERIFIED | `combine_printer_and_location` (`report_service.rs:279-288`) returns `None` when `printer_name` is `None`. Unit test `combine_printer_and_location_none_without_printer` and integration test `report_requests_printer_location_blank_when_no_printer` both pass. |
| 7 | Manager не видит `ad_register` заявки (строки + счётчики); Admin видит все — RBAC REQ-06/T-09-11 | VERIFIED | `exclude_ad_register = trackly_core::auth::excludes_ad_register(&caller.role)` computed and threaded through `list_requests_*`, `fetch_report`, `get_report_counts` (all 3 entry points: list/export/counts). Applies `ad_register_predicate("r.")` in WHERE. Integration test `report_requests_manager_role_excludes_ad_register_admin_sees_all` passes (Manager sees 3/4, Admin sees 4/4). |
| 8 | И Tauri-команды, и `/api/v1/reports_list_requests_*` HTTP-роуты работают одинаково | VERIFIED | 4 Tauri commands (`tauri_cmds/reports.rs:499-539`) and 4 HTTP handlers + routes (`http/reports.rs:181-239, 319-334`) both delegate to the same `build_reports_list_requests_*` helpers — single source of business logic, dual thin adapters (matches CLAUDE.md dual access path pattern). Confirmed by direct code read, not just grep. |
| 9 | Существующие отчёты «Устройства»/«Картриджи» не регрессировали | VERIFIED | Ran full `cargo test -p trackly-app -- --skip login_remember_persistent_cookie --test-threads=1` independently: all test binaries report "0 failed" (197+ unit tests plus every integration test file, including `report_acts`, `report_cartridges`, `report_csv_export`, `html_report_render`, `html_header_parity`, `reports_period_required` — all green). |

**Score:** 9/9 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-app/src/dto/reports.rs` | `request_type_label: Option<String>` field | VERIFIED | Present with doc comment; all `ReportRow{...}` literals in the codebase updated (compiles clean). |
| `crates/trackly-app/src/services/report_service.rs` | `query_requests_inner`/`count_requests_inner`, translators, `list_requests_*`, `get_report_counts(requests)` | VERIFIED | All present, follow `query_acts_inner` pattern (parameterized `?N`, `spawn_blocking`, `next_idx`). |
| `crates/trackly-app/src/tauri_cmds/reports.rs` | `columns_for`/`column_labels_for`/`report_display_name`/`PERIOD_BASED_REPORT_TYPES` extended + 4 Tauri commands | VERIFIED | Confirmed by direct read, index-alignment test passes. |
| `crates/trackly-app/src/http/reports.rs` | 4 handlers + 4 `/api/v1/reports_list_requests_*` routes | VERIFIED | Confirmed present and wired to `build_reports_list_requests_*`. |
| `crates/trackly-app/src/specta_export.rs` | 4 commands registered | VERIFIED | `reports_list_requests_all/open/in_progress/completed` present in `collect_commands!`. |
| `crates/trackly-app/tests/report_requests.rs` | 6 integration tests | VERIFIED | File exists, all 6 tests pass independently (re-run by verifier). |
| `ui/src/features/reports/ReportSubNav.svelte` | domain 'requests' + 4 tabs | VERIFIED | Confirmed by direct read. |
| `ui/src/features/reports/ReportsPage.svelte` | `REQUEST_REPORTS`, `COLUMNS_MAP` requests entries, `currentCmd`/`reportTypeKey` extended | VERIFIED | Confirmed by direct read. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `ReportsPage.svelte reportTypeKey()` | `tauri_cmds/reports.rs columns_for()/fetch_report()` | `requests_(all\|open\|in_progress\|completed)` key | WIRED | `reportTypeKey()` switch statement (lines 310-321) returns exactly these 4 keys; backend `columns_for`/`fetch_report` match arms consume them identically. |
| `report_service.rs query_requests_inner` | `dto/reports.rs ReportRow` | `translate_request_type`/`translate_request_status` computed once at row-read time | WIRED | Confirmed in `query_requests_inner` row-mapping closure (lines 1338-1354) — single computation point feeding screen/CSV/print via `row_field()`. |
| `tauri_cmds/reports.rs build_reports_list_requests_*` | `ad_register_predicate` / `excludes_ad_register` | `exclude_ad_register: bool` threaded to service | WIRED | Confirmed at all 3 entry points (list, export via `fetch_report`, counts via `build_reports_get_report_counts`). |
| `ReportSubNav.svelte DOMAINS/REQUEST_REPORTS` | `http/reports.rs router()` | command name == HTTP route suffix | WIRED | `reports_list_requests_all` (Tauri cmd name) ↔ `/api/v1/reports_list_requests_all` (route) — same string convention as existing domains, confirmed present. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| New report_requests integration tests | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test report_requests -- --test-threads=1` | 6 passed, 0 failed | PASS |
| Index-alignment regression guard | `cargo test -p trackly-app --lib tauri_cmds::reports::tests::` | 1 passed (covers requests_* keys) | PASS |
| Translator unit tests | `cargo test -p trackly-app --lib services::report_service::tests::` | 30 passed, 0 failed | PASS |
| Existing report regression | `cargo test -p trackly-app --test report_acts --test report_cartridges --test report_csv_export --test html_report_render --test html_header_parity` | All green (2+2+2+8+5) | PASS |
| Period-required regression | `cargo test -p trackly-app --test reports_period_required` | 2 passed | PASS |
| Full crate regression | `cargo test -p trackly-app -- --skip login_remember_persistent_cookie --test-threads=1` | Every test binary reports "0 failed" | PASS |
| Backend compile gate | `cargo check -p trackly-app --all-targets` | 0 errors | PASS |
| Frontend type-check | `pnpm --dir ui run svelte-check` | 0 errors, 269 files, only pre-existing unrelated warnings | PASS |
| Frontend build | `pnpm --dir ui build` | Success, `bindings.ts` regenerated with 4 new `reports_list_requests_*` bindings | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| VAD-01 | 260820-vad-PLAN.md | 4 status tabs, «Все» incl. rejected, period-based | SATISFIED | Truths 1-3 verified |
| VAD-02 | 260820-vad-PLAN.md | 6-column parity screen/CSV/print, empty printer/location | SATISFIED | Truths 4, 6 verified |
| VAD-03 | 260820-vad-PLAN.md | RU translation Type/Status, backend-computed, raw fallback | SATISFIED | Truth 5 verified |
| VAD-04 | 260820-vad-PLAN.md | RBAC ad_register exclusion, dual Tauri+HTTP, no regression | SATISFIED | Truths 7-9 verified |

No orphaned requirements found — all 4 requirements declared in the PLAN frontmatter are addressed.

### Anti-Patterns Found

None. No TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER markers, no empty stub implementations, no hardcoded empty data flowing to render paths in the modified files. `_filter: ReportFilter` unused-parameter pattern in `list_requests_*` is documented via doc-comment as intentional (uniform signature with `fetch_report()` dispatch), not a stub.

### Privacy Check (CLAUDE.md hard constraint)

Scanned the 3 task commits for real organizational data, real names, or credentials — none found. Test fixtures in `report_requests.rs` use only fictional data («Иванов И.И.», «Склад тест», «Принтер HP LaserJet», login `us501`), matching the project convention. Git author lines show only the user's own committer identity, not leaked PII.

### Human Verification Required

Two items, both explicitly and honestly flagged by the executor as UNVERIFIED in SUMMARY.md (not a gap — a documented limitation of synthetic compile gates against Svelte 5 runes, per project lesson «Compile gates miss Svelte runtime»):

1. **Live UAT of the «Заявки» domain in the running app** — tab switching, real per-tab counters, PeriodSelector reactivity, CSV download, print/preview modal — on both desktop (Tauri) and LAN browser.
2. **Visual regression check** of «Устройства»/«Картриджи» domains — confirm no visual/behavioral change from adding the third domain.

### Gaps Summary

No gaps found. All 9 derived must-have truths (roadmap goal + PLAN frontmatter must_haves) are backed by code that was independently re-read and re-tested by the verifier (not just SUMMARY claims): domain layer, wiring (Tauri + HTTP dual path), frontend config, RBAC, translation, column parity, and full regression suite all check out. The only outstanding item is live-application UAT, which requires a human and cannot be verified by static analysis or automated tests — correctly classified as `human_needed`, not `gaps_found`.

---

_Verified: 2026-08-21T02:55:00Z_
_Verifier: Claude (gsd-verifier)_
