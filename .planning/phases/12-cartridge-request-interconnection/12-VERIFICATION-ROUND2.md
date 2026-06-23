---
phase: 12-cartridge-request-interconnection
verified: 2026-06-24T01:30:00Z
status: resolved
resolved: 2026-06-24T02:10:00Z
resolution_note: >
  R3 gap (cancelled-status UI threading) closed inline during this execute-phase run.
  All enumerated sites fixed + committed (598561b, 2b93f52, and toast-variant follow-up):
  statusLabel/actionLabel (RequestDetail + RequestListRow), RequestCounts cancelled bucket
  (domain+repo+dto+service+bindings), EmployeeLayout toast text AND variant (info, not success),
  and a new «Отменённые» filter tab so cancelled requests are reachable. CR-01 blocker
  (test_db.rs stale schema version) also fixed (2b93f52). Verified: trackly-infra 74/74,
  request_lifecycle 7/7, role_endpoint_matrix green, svelte-check 0 errors, ui build OK.
score: 6/6 gap-closure truth-groups verified (GAP-12-04..08; GAP-12-07 presentation closed in R3)
scope: "Round 2 --gaps-only — plans 12-10..12-15 only (GAP-12-04..08). Plans 12-01..12-09 verified in prior rounds (12-VERIFICATION.md)."
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: "11/11 (programmatic) + 2 human-verify pending — predates Round 2 gap-closure batch"
  note: "Prior 12-VERIFICATION.md (2026-06-23) covered the ORIGINAL phase + GAP-12-01..03. This report is the FIRST verification of the Round 2 gap-closure batch (GAP-12-04..08, plans 12-10..12-15)."
  gaps_closed:
    - "GAP-12-08: V030 drops printers connectivity CHECK; printer without IP/USB creatable"
    - "GAP-12-04: WsEvent per-variant camelCase serialization + OperationModal duplicate-toast suppression"
    - "GAP-12-05: install dialog shows printer name+IP first; reversed-semantics hint for Кто/Кому"
    - "GAP-12-06: suggest_person sources given_by_name from audit_log for install/to_refill"
    - "GAP-12-07 (mechanism): reject-from-in_progress, soft-delete any-status, employee self-cancel — backend + endpoints + RBAC + UI buttons all wired and tested"
  gaps_remaining: []
  gaps_closed_r3:
    - "GAP-12-07 (presentation): `cancelled` now threaded through statusLabel (RequestDetail+RequestListRow → «Отменена»), actionLabel history map (cancel/custom:cancel), RequestCounts aggregation (cancelled bucket end-to-end), EmployeeLayout toast text+variant, and a new «Отменённые» filter tab"
    - "CR-01 (code review BLOCKER): test_db.rs schema-version assertion now derived from max_known_version() instead of hardcoded 30"
  regressions: []
gaps:
  - truth: "Сотрудник может отменить собственную заявку И ВИДЕТЬ ЕЁ КОРРЕКТНО ОТРАЖЁННОЙ как «Отменена» (не «Отклонена»)"
    status: resolved
    reason: >
      Механизм отмены работает end-to-end (status→'cancelled', persisted, BOLA-guarded,
      WS-broadcast — все backend-тесты зелёные). Но новый терминальный статус `cancelled`,
      созданный СПЕЦИАЛЬНО чтобы отличаться от `rejected` (12-14 objective:
      «семантически отмена самим автором отличается от отклонения специалистом»), нигде в
      презентационном слое не отделён от `rejected` — отменённая пользователем заявка
      отображается как «Отклонена», что прямо противоречит цели фичи. Это user-visible дефект
      доставленной фичи GAP-12-07, а не косметика: наблюдаемое отличие, ради которого статус
      и был введён, не реализовано в UI.
    artifacts:
      - path: "ui/src/features/requests/RequestDetail.svelte:98-108"
        issue: "statusLabel $derived: catch-all else → 'Отклонена'. Нет ветки для 'cancelled' → отменённая заявка показывается как «Отклонена» (Rejected). statusVariant (86-96) тоже сливает её в 'default'."
      - path: "ui/src/features/requests/RequestDetail.svelte:157-169"
        issue: "actionLabel map не содержит ключей 'cancel'/'custom:cancel'. RequestService::cancel пишет audit action='custom:cancel' → в History рендерится сырая строка «custom:cancel» через `?? action` fallback."
      - path: "crates/trackly-infra/src/repos/requests_sqlite.rs:303-361 + crates/trackly-app/src/dto/request.rs:276-286"
        issue: "RequestCounts/RequestCountsDto не имеет bucket 'cancelled'. Запрос `all` не фильтрует по статусу → включает cancelled-строки, но cancelled не учтён ни в одном bucket. После self-cancel: all += 1, а open+in_progress+completed+rejected больше не сходится в all → switch-bar/дашборд дрейфит, cancelled-заявки неисчислимы."
      - path: "ui/src/features/layout/EmployeeLayout.svelte:32-43"
        issue: "statusToastText() switch не имеет case 'cancelled' → WS-тост при self-cancel падает в default «Статус вашей заявки изменён» вместо специфичного «Ваша заявка отменена». Graceful, но снова cancelled не нитится."
    missing:
      - "Добавить ветку 'cancelled' → 'Отменена' в statusLabel (и подходящий statusVariant) в RequestDetail.svelte перед catch-all else."
      - "Добавить cancel: 'Отменена' и 'custom:cancel': 'Отменена' в actionLabel labels record."
      - "Добавить поле cancelled в RequestCounts + RequestCountsDto + count-запрос WHERE status='cancelled'."
      - "Опционально: case 'cancelled' в EmployeeLayout.statusToastText() для корректного текста тоста автору."
      - "Sibling-файл ui/src/features/requests/RequestListRow.svelte:28-36 имеет идентичный statusLabel-паттерн — проверить/обновить вместе (вне review-set, но тот же дефект)."
---

# Phase 12 (Round 2): Gap-Closure Verification Report

**Phase Goal (Round 2 scope):** Закрыть 6 Round-2 UAT-гэпов GAP-12-04..08 планами 12-10..12-15:
duplicate status notifications, install dialog polish, name autocomplete (given_by_name),
request lifecycle (reject-from-in_progress / soft-delete / employee self-cancel + UI), drop
printer connectivity CHECK.

**Verified:** 2026-06-24T01:30:00Z
**Status:** gaps_found
**Re-verification:** No — first verification of the Round 2 batch (prior 12-VERIFICATION.md predates these plans).
**Scope:** ONLY plans 12-10..12-15. Plans 12-01..12-09 verified in earlier rounds.

## Goal Achievement

### Observable Truths (per gap-closure plan)

| # | Gap / Plan | Truth | Status | Evidence |
|---|-----------|-------|--------|----------|
| 1 | GAP-12-08 / 12-10 | Принтер без IP и без USB создаётся (нет CHECK constraint) | ✓ VERIFIED | V030 пересобирает printers без CHECK; `test_printer_no_ip_no_usb` ✓ pass; `run_applies_all_known_migrations_on_fresh_db` ✓ pass |
| 2 | GAP-12-04 / 12-11 | Сотрудник видит ровно одно уведомление с корректным текстом; админ — один тост при установке | ✓ VERIFIED | WsEvent per-variant `rename_all="camelCase"` (printer.rs:190/202/211); 3 serialization-тесты ✓ pass (`*_serializes_camel_case_fields_snake_case_tag`); `suppressSuccessToast` проп + `suppressSuccessToast={true}` в RequestDetail:709 |
| 3 | GAP-12-05 / 12-12 | Принтер (имя+IP) первой строкой формы; подсказка о перевёрнутой семантике Кто/Кому | ✓ VERIFIED | `printerContext` $state + `printerContextHint` (OperationModal:145-160), рендерится первым (453-458); reversed-semantics hint (495-498); svelte-check 0 errors |
| 4 | GAP-12-06 / 12-13 | Имя «Кто выдал» появляется в автокомплите при след. вводе | ✓ VERIFIED | suggest_person third UNION ALL `json_extract(payload_json,'$.given_by_name')` для Giver-контекста (act_service.rs:1106-1118); 14 acts_suggest тестов ✓ pass (incl. install/to_refill/excludes-irrelevant/no-leak-into-receiver) |
| 5 | GAP-12-07 (mech) / 12-14+15 | Reject из in_progress; delete любой статус; employee self-cancel своей open-заявки; чужая/in_progress → отказ | ✓ VERIFIED | Domain Cancel op + Reject open\|in_progress (printers.rs:165/179); Action::DeleteRequests\|CancelOwnRequest (auth.rs:114/117); delete()/cancel() (request_service.rs:581/654); dual-transport (tauri:272/287, http:284/285); specta:136/137; RBAC Cases 36-39 + request_lifecycle 7 тестов ✓ pass; UI кнопки+модалки (RequestDetail:566/591/645/658) |
| 6 | GAP-12-07 (presentation) | Отменённая заявка ОТОБРАЖАЕТСЯ как «Отменена», отлична от «Отклонена» | ✗ FAILED (partial) | `cancelled` не нитится: statusLabel→«Отклонена», actionLabel→сырой «custom:cancel», RequestCounts без bucket, EmployeeLayout→default-текст. См. gaps. |

**Score:** 5/6 truth-groups verified. GAP-12-07 mechanism complete, presentation incomplete.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `migrations/V030__printers_drop_connectivity_check.sql` | printers rebuild без CHECK | ✓ VERIFIED | `CREATE TABLE printers_new`, нет connectivity CHECK, `user_version=30` |
| `migrations/V031__requests_status_add_cancelled.sql` | status CHECK включает 'cancelled' | ✓ VERIFIED | requests rebuild, CHECK incl 'cancelled', `user_version=31` |
| `crates/trackly-infra/.../test_db.rs` | CR-01 fix: динамич. version | ✓ VERIFIED | `expected = max_known_version()` (43); тест ✓ pass — BLOCKER устранён |
| `crates/trackly-core/src/domain/printers.rs` | Cancel op + Reject 2-status | ✓ VERIFIED | Cancel(165), validate/target/audit + unit-тесты ✓ |
| `crates/trackly-core/src/auth.rs` | DeleteRequests + CancelOwnRequest | ✓ VERIFIED | оба варианта + authorize ветки + тесты ✓ |
| `crates/trackly-app/.../request_service.rs` | delete() + cancel() | ✓ VERIFIED | оба метода, BOLA через self.get, audit, WS-broadcast |
| `crates/trackly-app/.../dto/printer.rs` | WsEvent camelCase per-variant | ✓ VERIFIED | rename_all на каждом из 3 вариантов |
| `crates/trackly-app/.../act_service.rs` | given_by_name UNION ALL | ✓ VERIFIED | json_extract arm, Giver-only, ESCAPE-safe |
| `ui/.../OperationModal.svelte` | printer hint + reversed hint + toast suppress | ✓ VERIFIED | все три присутствуют и рендерятся |
| `ui/.../requests/api.ts` | delete/cancel wrappers | ✓ VERIFIED | `requests.delete`/`requests.cancel` (34/36) |
| `ui/.../requests/RequestDetail.svelte` | Удалить/Отменить кнопки + модалки | ⚠️ PARTIAL | кнопки/модалки/handlers есть; НО statusLabel+actionLabel не нитят `cancelled` |
| `crates/trackly-infra/.../requests_sqlite.rs` | counts() | ⚠️ PARTIAL | counts работает, но нет cancelled-bucket (WR-03) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| tauri_cmds/requests.rs | http/requests.rs | build_requests_(delete\|cancel) dual-transport | ✓ WIRED | оба транспорта делегируют общим build_* |
| request_service.rs delete/cancel | role_endpoint_matrix Cases 36-39 | RBAC regression | ✓ WIRED | Cases 36-39 присутствуют и ✓ pass |
| dto/printer.rs WsEvent | EmployeeLayout.svelte | event.newStatus camelCase | ✓ WIRED (поле) / ⚠️ (cancelled-текст) | camelCase поле приходит корректно; но statusToastText не имеет ветки cancelled |
| RequestDetail.svelte | requests/api.ts | requests.delete()/cancel() | ✓ WIRED | handlers вызывают обёртки |
| cancel() | WsEvent::RequestStatusChanged | new_status='cancelled' broadcast | ✓ WIRED | request_service.rs:696-700 |

### Behavioral Spot-Checks (cargo test, executed in verifier process)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Domain Cancel/Reject + auth Actions | `cargo test -p trackly-core` | 48 + … passed, 0 failed | ✓ PASS |
| V030 + CR-01 fix + migrations | `cargo test -p trackly-infra` | test_printer_no_ip_no_usb, test_db_returns_fully_migrated_connection, run_applies_all ✓ | ✓ PASS |
| delete/cancel service + BOLA | `cargo test -p trackly-app --test request_lifecycle` | 7 passed, 0 failed | ✓ PASS |
| suggest_person given_by_name | `cargo test -p trackly-app --test acts_suggest` | 14 passed, 0 failed | ✓ PASS |
| RBAC Cases 36-39 | `cargo test -p trackly-app --test role_endpoint_matrix` | passed, 0 failed | ✓ PASS |
| WsEvent camelCase serialization | `cargo test -p trackly-app dto::printer` | 3 serialization tests passed | ✓ PASS |
| Frontend type-safety | `pnpm --dir ui exec svelte-check` | 0 ERRORS, 36 pre-existing warnings | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| GAP-12-04 | 12-11 | Duplicate status notifications | ✓ SATISFIED | WsEvent camelCase + toast suppress |
| GAP-12-05 | 12-12 | Install dialog polish | ✓ SATISFIED | printer hint + reversed hint |
| GAP-12-06 | 12-13 | Name autocomplete given_by_name | ✓ SATISFIED | suggest_person UNION ALL + 14 tests |
| GAP-12-07 | 12-14, 12-15 | Request lifecycle | ⚠️ PARTIAL | mechanism done; cancelled presentation incomplete |
| GAP-12-08 | 12-10 | Drop printer connectivity CHECK | ✓ SATISFIED | V030 + test |

**Note (per task brief):** GAP-12-04..08 are UAT-found, NOT in REQUIREMENTS.md traceability table —
phase driven by user decisions in 12-CONTEXT.md / 12-HUMAN-UAT.md. Absence of formal REQ-IDs is
expected and is NOT a verification failure.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | No TBD/FIXME/XXX/HACK/PLACEHOLDER in any batch-modified file | — | Clean |

(WR-04..06 / IN-01..03 from 12-REVIEW.md are pre-existing or non-blocking correctness/forensics
items — see "Other review findings" below. None gate the Round-2 gap goals.)

### Gaps Summary

The Round-2 batch is **mechanically complete and green**: every backend feature (V030/V031
migrations, Cancel/Reject domain ops, DeleteRequests/CancelOwnRequest auth, delete()/cancel()
service methods, dual-transport endpoints, RBAC Cases 36-39, WsEvent camelCase, suggest_person
given_by_name) is implemented, wired across both transports, and covered by passing tests. The
CR-01 BLOCKER (test_db hardcoded user_version==30) is already fixed and the test is green. All
UI artifacts (delete/cancel buttons + modals, printer hint, reversed-semantics hint, toast
suppression) exist and svelte-check is error-free.

The one real gap is **GAP-12-07 presentation**: the new terminal status `cancelled` — introduced
DELIBERATELY to be distinguishable from `rejected` (12-14's stated objective) — is not threaded
through any presentation/aggregation surface. A user who self-cancels sees the request labelled
"Отклонена" (Rejected by a specialist), its History shows the raw `custom:cancel` string, the
status-count switch-bar no longer reconciles (`all` includes cancelled but no bucket holds it),
and the employee's WS toast falls to a generic fallback. This is a user-visible defect of the
delivered feature, not cosmetics — the observable distinction the feature exists to create is
absent. Hence GAP-12-07 is scored partial, and overall status is gaps_found.

These four sites (RequestDetail statusLabel + actionLabel, RequestCounts/Dto, EmployeeLayout
toast text — plus the identical sibling RequestListRow.svelte) are small, mechanical additions
and are the actionable closure scope for a focused follow-up.

### Other 12-REVIEW.md findings (informational — do not gate Round-2 goals)

- **WR-04** (Manager can soft-delete Admin-only `ad_register` request, orphaning users row):
  real privilege-boundary asymmetry, but it concerns the `delete()` *authorization scope*, not
  the gap-closure truths. Worth a security follow-up; flagged here for the planner's awareness.
- **WR-05** (`printers_sqlite::list` status filter is a silent no-op): pre-existing, surfaced by
  V030 touching the file; not introduced by this batch.
- **WR-06** (`transition_in_tx` `affected==0` collapses lock-mismatch into NotFound): pre-existing
  UX-degradation; not gating.
- **IN-01..03** (community_configured constant; FK-PRAGMA invariant guard; delete() no
  before_json snapshot): info-level hardening suggestions.

---

_Verified: 2026-06-24T01:30:00Z_
_Verifier: Claude (gsd-verifier)_
_Scope: Round 2 --gaps-only (plans 12-10..12-15)_
