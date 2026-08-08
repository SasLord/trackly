---
phase: 260808-np4-unify-ad-register-visibility-predicate
verified: 2026-08-08T00:00:00Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
---

# Quick Task 260808-np4: Unify ad_register visibility predicate Verification Report

**Task Goal:** Вынести дублирующийся предикат видимости `ad_register` в одно место + закрепить регрессионным тестом, БЕЗ изменения наблюдаемого поведения. Видимость остаётся admin-only.
**Verified:** 2026-08-08
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Admin sees ad_register in list()/counts(); Manager/Employee see zero — identical behavior, now via one shared predicate | ✓ VERIFIED | `trackly_core::auth::excludes_ad_register` (auth.rs:173-175) is the sole role-rule function, called from both `RequestService::list` (request_service.rs:120) and `counts` (request_service.rs:164). `git show fea0ef3` confirms the diff is exactly the two `let exclude_ad_register = ...` lines, nothing else touched. |
| 2 | `get_employee_widgets` still unconditionally excludes ad_register, now sourced from shared SQL helper | ✓ VERIFIED | dashboard_service.rs:328 calls `trackly_infra::repos::requests_sqlite::ad_register_predicate("r.")` directly into the `clauses` vec — no new bound parameter, no role check inside the function (still reached only via the `Role::Employee` dispatch at get_all_widgets:63-65, D-GATE-03 unchanged). |
| 3 | New regression test drives `RequestService::list`/`counts` through a Manager Identity via the service layer, with a control assertion | ✓ VERIFIED | `requests_ad_register_visibility_manager.rs` calls `svc.list(...)`/`svc.counts(...)` (real `RequestService`, real `WriterHandle`/reader pool via `test_writer_and_readers`) with `manager(manager_id)` — a real `Identity{role: Role::Manager}`, not a repo-level precomputed flag. Asserts manager sees 0 ad_register rows, `counts.all == 1`, and that the control `free_form` request stays visible/counted; companion Admin assertions (`counts.all == 2`) prove role-specificity. |
| 4 | Test proven non-vacuous via a real mutation-check cycle with captured RED evidence | ✓ VERIFIED | SUMMARY's captured RED text (`panicked at ...requests_ad_register_visibility_manager.rs:166:5: manager list must not contain any ad_register requests`) matches the actual assertion at test file line 166 exactly. This is a genuine assertion panic (not a compile error), and `git diff crates/trackly-core/src/auth.rs` currently shows no drift from the Task-1-committed state, consistent with the claimed clean revert. |
| 5 | All 8 SQL literal occurrences + 2 duplicated role-check expressions consolidated to one shared implementation each; zero remaining duplicates anywhere in crates/*/src/ | ✓ VERIFIED | `git show 210cee3` diff confirms all 8 `list()`/`counts()` literals now interpolate `{ad_register_clause}`, with byte-identical resulting SQL text (`?5`/`?2` placeholders and param order/count unchanged). Exhaustive grep across `crates/*/src/` for `ad_register` found only: the 2 helper functions' own literal, the 2 call sites using them, `exclude_ad_register: bool` trait/param plumbing, and two unrelated occurrences in `auth.rs` (AD-bind auto-approve completion UPDATE and ad_register INSERT) that are not visibility-exclusion logic — a different concern (existence/creation, not read-side hiding). |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-core/src/auth.rs` | `pub fn excludes_ad_register` + 3 unit tests | ✓ VERIFIED | Present at auth.rs:166-175, doc-commented with REQ-06/T-09-11 rationale; 3 new tests `excludes_ad_register_admin_is_false`/`_manager_is_true`/`_employee_is_true` present in `mod tests` (auth.rs:350-363). |
| `crates/trackly-app/src/services/request_service.rs` | Both call sites use `excludes_ad_register(&caller.role)` | ✓ VERIFIED | Confirmed at lines 120 and 164. |
| `crates/trackly-infra/src/repos/requests_sqlite.rs` | `ad_register_predicate`/`ad_register_exclude_clause` reused across 2+6 queries | ✓ VERIFIED | Both fns present (lines 84-95); all 8 call sites (list: 2, counts: 6) use `ad_register_clause` interpolation. |
| `crates/trackly-app/src/services/dashboard_service.rs` | Sources exclusion from shared helper, still unconditional | ✓ VERIFIED | Line 328: `ad_register_predicate("r.")`, unconditional, no new param, gate unchanged (D-GATE-03 dispatch still at get_all_widgets:63-65). |
| `crates/trackly-app/tests/requests_ad_register_visibility_manager.rs` | Manager-role regression test via service layer | ✓ VERIFIED | Created, exercises `svc.list`/`svc.counts` with real `Identity{role: Role::Manager}`, plus Admin comparison. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `request_service.rs::list()`/`counts()` | `trackly_core::auth::excludes_ad_register` | direct fn call | ✓ WIRED | Grep confirms both call sites present. |
| `requests_sqlite.rs::list()`/`counts()` SQL builders | `ad_register_exclude_clause`/`ad_register_predicate` | `format!` interpolation | ✓ WIRED | `ad_register_clause` local var computed via `ad_register_exclude_clause("r.","?5")` (list) / `ad_register_exclude_clause("","?2")` (counts), interpolated into every query string. |
| `dashboard_service.rs::get_employee_widgets` | `requests_sqlite::ad_register_predicate` | `clauses.push(...)` | ✓ WIRED | Line 328, fully-qualified call, matches existing file convention. |
| `requests_ad_register_visibility_manager.rs` | `RequestService::list`/`counts` | `svc.list(...)`/`svc.counts(...)` with `Role::Manager` Identity | ✓ WIRED | Confirmed test body calls both methods through the real service with a real Manager Identity. |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| REQ-06 | 260808-np4-PLAN.md | Only Admin sees `ad_register` requests | ✓ SATISFIED | Rule consolidated to `excludes_ad_register` + `ad_register_predicate`/`ad_register_exclude_clause`; all 4 call sites wired; behavior confirmed unchanged via byte-identical SQL diffs and pre-existing test suites (`requests_ad_register.rs`, `dashboard_widgets.rs`) untouched by this task's commits. |

### Anti-Patterns Found

None. Grep for `TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER` across all 5 files modified/created by this task returned zero matches.

### Data-Flow / Behavioral Notes

- SQL parameter numbering verified byte-for-byte identical pre/post refactor via direct `git show 210cee3` diff inspection: `list()` total query and paginated query both use `?5` bound to `exclude_ad_register_i64` before `?6`/`?7` (limit/offset); `counts()`'s 6 queries all use `?2` bound to `exclude_ad_register_i64` after `?1` (`requested_by_user_id`) — no shift, no renumbering.
- `dashboard_service.rs::get_employee_widgets` reachability unchanged: still gated behind `matches!(caller.role, Role::Employee)` in `get_all_widgets` (D-GATE-03), so the "unconditional" exclusion inside `get_employee_widgets` never needs a role branch — Admin/Manager never reach this function at all.
- Mutation-check evidence in SUMMARY.md cross-checked against actual test file: captured panic location `requests_ad_register_visibility_manager.rs:166:5` and message text match the real assertion at that line exactly — this is strong evidence the RED-run capture was not fabricated from memory.
- Commit history verified: `fea0ef3`, `210cee3`, `5d77a8f`, `1c7f73b` all exist in `git log`, and each commit's diff matches the corresponding task's claimed scope exactly (no extraneous changes).

### Human Verification Required

None — this is a pure backend refactor with no UI/UX surface; all claims are verifiable via static code inspection, git diff, and prior orchestrator-confirmed test runs.

### Gaps Summary

No gaps found. All must-haves (5 truths, 5 artifacts, 4 key links) verified against the actual codebase, not just SUMMARY.md claims. The refactor genuinely collapses the previously-triplicated REQ-06 rule into 2 shared functions, SQL parameter numbering is provably unchanged, the dashboard exclusion remains unconditional, and the new Manager-role regression test is genuinely non-vacuous with cross-verified mutation-check evidence.

---

_Verified: 2026-08-08_
_Verifier: Claude (gsd-verifier)_
