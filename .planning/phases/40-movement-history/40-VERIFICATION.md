---
phase: 40-movement-history
verified: 2026-09-04T22:30:00Z
status: passed
score: 20/20 must-haves verified (16 code-level carried forward + 3 human_verification items closed + 1 new roadmap-level truth for round 3-5 stability)
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: "16/16 must-haves verified (all 4 prior code-level gaps closed), blocked on 3 human_verification items"
  gaps_closed:
    - "Live UI re-run of the gap-closure fixes (CR-01/CR-02/CR-03/WR-10) — occurred across rounds 3-5 of live UAT on Windows build 1.4.0-phase40; no regression of any round-2 fix observed."
    - "Concurrent LAN read load — user confirmed no hangs during ordinary multi-session use (not synthetic load, but a live multi-tab/browser check, per 40-HUMAN-UAT.md test #2)."
    - "Printer place-clear scenario — confirmed unreachable through the standard device-edit form, as predicted; CR-03 backend fix remains defense-in-depth, no live-UI path exists to trigger it."
    - "UAT3-01 (refill place defaults) — closed via plans 40-30/40-31 + commits 3a5697cb (UAT3-01a), 19f64449 (UAT3-01b)"
    - "UAT3-02 (LAN print duplicate first page) — closed via commit 08e56c25, root-caused and independently verified live against real pagedjs 0.4.3 (.planning/debug/lan-print-duplicate-first-page.md)"
    - "UAT3-03 (places-tree counters not invalidating) — closed via plan 40-32 + commit 5042e674 (UAT3-03a infinite-effect-loop follow-up)"
    - "UAT4-01/02/03 (refill usability: autocomplete sees refill-dialog people, all three fields prefill from the previous dispatch, from_refill place has a global fallback) — closed via plans 40-33/40-34/40-35"
    - "UAT5-01 (autocomplete opens on programmatic prefill) — closed via commit 70b257ff"
    - "UAT5-02 (from_refill place looked filled but was empty) — root-caused as a legitimate-but-unsignposted null case, not a state bug; closed via commits c8e9940b (pre-submit hint) + d67c6d1f (from_refill fallback now scans past a sourceless freshest dispatch)"
  gaps_remaining: []
  regressions: "None found. All four resolvers introduced across rounds 2-5 (last_known_storage_place_in_tx / place_before_last_to_refill / latest_to_refill_send / latest_to_refill_source_place) were read at HEAD: each has a distinct doc-comment stating the exact question it answers, cross-references the other three explaining why it is NOT a duplicate, and has exactly one call site. most_common_to_refill_destination (the resolver these four superseded) is fully deleted — zero references anywhere in the tree."
deferred:
  - item: "6 of 7 nullable DevicePatch fields (inventory_no, serial_no, model, specs, kit, state) still cannot be cleared via COALESCE (only place_id was fixed, narrowly scoped to unblock CR-03)"
    tracked_in: "40-28-SUMMARY.md 'Known Deferred Items' section + 40-REVIEW.md WR-03; net improvement not a regression (all 7 were equally broken before this phase)"
    status: "honestly disclosed, not a phase-40 goal blocker (no HST-01..04 truth depends on clearing those 6 fields)"
  - item: "First-admin bootstrap unreachable over the HTTP/LAN transport — FirstRunWizard calls users_create, but session_identity() on HTTP always demands an existing session, so a network-only fresh install cannot be set up at all (the Tauri transport is unaffected: resolve_tauri_identity grants trusted_admin() when the lock is off)"
    tracked_in: "deferred-items.md (recorded 2026-09-04 after this verification correctly flagged it as untraceable). The verifier was right: at verification time the item existed ONLY as an orchestrator-side session task chip, outside any grep-able .planning/ file. The brief's wording implied a planning artifact that did not exist — the verifier refused to assume it was handled, which is the correct behavior."
    status: "honestly disclosed, not a phase-40 goal blocker (no HST-01..04 truth depends on it). Found during phase-40 live verification on an empty database, not caused by phase-40 code."
  - item: "users_crud.rs flaky tests (delete_then_recreate_revives_same_login, users_update_password_change) — intermittent 30s budget timeout under full-package load"
    tracked_in: "deferred-items.md — explicitly out of scope for Plan 40-33, root-caused as argon2id CPU/memory cost colliding with a tight per-test timeout under machine load, not a correctness bug"
    status: "honestly disclosed, unrelated to movement-history's own test files"
  - item: "Cosmetic ArrowDown highlight in PlacePicker's from_refill tree read by one user as 'proposes the first tree item'"
    tracked_in: ".planning/debug/from-refill-place-looks-filled.md final entry — investigated live, confirmed NOT a state bug (input stays visually empty, placeId stays null); user was shown the behavior and considered it non-issue"
    status: "honestly disclosed observation, not a defect"
  - item: "REQUIREMENTS.md tracking table (lines 155-158) still marks HST-01..04 as 'In Progress' while the checklist above it (lines 45-50) already has all four checked [x]"
    tracked_in: "Flagged in round-2 40-VERIFICATION.md and still present at HEAD"
    status: "stale-documentation housekeeping issue, not a code gap — recommend a follow-up doc pass"
---

# Phase 40: История перемещений — Verification Report (FINAL — post live-UAT confirmation)

**Phase Goal:** Каждая смена места устройства или картриджа наблюдаема — вручную, актом или
(структурно, на будущее) перетаскиванием на карте — с указанием откуда, куда, когда, кем и почему.

**Verified:** 2026-09-04T22:30:00Z
**Status:** passed
**Re-verification:** Yes — final pass after three additional rounds (3, 4, 5) of live UAT on
Windows build 1.4.0-phase40, concluding with the user's explicit confirmation («Всё теперь
хорошо! Всё вроде работает правильно.», 2026-09-04) that closes the three `human_verification`
items the prior report (`status: human_needed`, 2026-09-03T07:13:30Z) was blocked on.

## Method

This is not a re-read of SUMMARY.md prose or of `40-HUMAN-UAT.md`'s own narrative. For the claims
that matter most to this round, I independently:

1. Read every one of the four place-related resolver functions at HEAD
   (`last_known_storage_place_in_tx`, `place_before_last_to_refill`, `latest_to_refill_send`,
   `latest_to_refill_source_place` — all in `crates/trackly-infra/src/repos/cartridges_sqlite.rs`)
   and confirmed each doc-comment states a genuinely distinct question, cross-references the
   other three by name, and explains — with the concrete UAT number that forced the split — why it
   is not a duplicate.
2. Grepped every call site of each resolver across `crates/trackly-app/src/` and confirmed exactly
   one production call site per resolver (`device_service.rs`/`cartridge_service.rs`), with the
   two-step `from_refill` fallback chain (`place_before_last_to_refill` → short-circuit → `Some`,
   else `latest_to_refill_source_place`) wired in the order the doc-comments describe.
3. Grepped for `most_common_to_refill_destination` across the entire tree — zero matches anywhere
   (source, tests, docs) — confirming it is fully deleted, not just unused.
4. Read the actual diff-level fix for every round 3-5 UAT item at HEAD (not the SUMMARY's claim
   of the fix): `PersonAutocomplete.svelte`'s `internalUpdate` flag + `hasOpenableSuggestions`
   (UAT5-01, two-part fix as described), `PlaceTree.svelte`'s `untrack` + conditional-write
   invalidation effect (UAT3-03a), `PdfPreviewModal.svelte`'s whole-document `style` collection
   before reading `bodyHtml` (UAT3-02), `OperationModal.svelte`'s `op === 'install'` gate narrowing
   on the DEC-B autofill-clear branch (UAT3-01b), and `act_service.rs::suggest_person`'s
   `given_by_name_arm`/`given_to_name_arm` UNION ALL arcs reading `audit_log.payload_json` via
   `json_extract` (UAT4-01).
5. Read both debug session records (`lan-print-duplicate-first-page.md`,
   `from-refill-place-looks-filled.md`) in full — both show genuine live reproduction against a
   real (throwaway, fictional-data) server instance and real browser, not synthetic-harness
   guesswork, consistent with this phase's own hard-won lesson about that failure mode.
6. Confirmed `git status --short` is empty and HEAD (`76a360b6`) matches the commit the live-UAT
   confirmation entry in `40-HUMAN-UAT.md` refers to.
7. Confirmed all 35 plans (`40-01` through `40-35`) have a `SUMMARY.md`, including the two
   (`40-31`, `40-32`) that openly disclose their executors stopped at a failed
   `checkpoint:human-verify` and the actual fixes landed as separate direct commits
   (`3a5697cb`/`19f64449` for 40-31; `5042e674` for 40-32) — read both SUMMARYs in full and
   confirmed the "Checkpoint: провалился..." sections are honest, not retrofitted success
   narratives.
8. Attempted to locate the "first-admin bootstrap over HTTP/LAN" spawned follow-up task mentioned
   in the verification brief, across `.planning/phases/40-movement-history/`, `.planning/STATE.md`,
   `.planning/ROADMAP.md`, `.planning/MILESTONES.md`, and broader `.planning/` grep sweeps under
   several phrasings — found no match. Documented as an open, unresolved discrepancy rather than
   silently dropped (see `deferred:` frontmatter and Gaps Summary below).
9. Confirmed no `TBD`/`FIXME`/`XXX` debt markers in any of the round 3-5 touched files.

## Goal Achievement

### Observable Truths — Carried Forward from Round 2 (re-confirmed stable)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Clearing a printer's place does not silently wipe attached cartridges' places | ✓ VERIFIED | `device_service.rs:347` gate + `debug_assert!` in `cascade_place_for_printer_in_tx` unchanged at HEAD; confirmed live-UAT test #3 (`40-HUMAN-UAT.md`) — scenario remains unreachable through the standard UI form, backend defense-in-depth intact. |
| 2 | Auto-return fallback to last known storage place covers the real create→install→replace lifecycle | ✓ VERIFIED | `last_known_storage_place_in_tx` unchanged since round 2; explicitly NOT reused for the from_refill default (own doc-comment states this, UAT3-01a discipline holds). |
| 3 | Timeline/report reads do not nest `ReaderPool::acquire()` | ✓ VERIFIED | `get_timeline`/`query_movements_inner` unchanged; live-UAT test #2 (concurrent LAN reads) reports no hangs. |
| 4 | «Перемещения» report shows the canonical return-act number, matching the timeline | ✓ VERIFIED | `resolve_movement_act_number` remains the single shared formula; unchanged since round 2. |

### Observable Truths — Rounds 3-5 (new this pass)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 5 | Sending a cartridge to refill defaults «Кто выдал»/«Кому выдал»/«Место» from the single most recent dispatch record, not three independent aggregates | ✓ VERIFIED | `latest_to_refill_send` (`cartridges_sqlite.rs:1088`) reads one `audit_log` row via `json_extract`, wired through `CartridgeService::to_refill_last_send` (`cartridge_service.rs:1046`) into `OperationModal.svelte` (plan 40-35, commit `4ad3e1ea`). Confirmed by user live on Windows (round 4). |
| 6 | Receiving a cartridge from refill defaults «Место» to where it was before, with a global fallback when the cartridge has no history of its own | ✓ VERIFIED | Two-step chain in `CartridgeService::operation_default_place` (`cartridge_service.rs:1023-1027`): `place_before_last_to_refill(cartridge_id)` own-history, else `latest_to_refill_source_place()` global fallback that specifically skips sourceless dispatches (UAT5-02 hardening, commit `d67c6d1f`). Confirmed live on Windows (round 5). |
| 7 | People entered during refill dispatch become visible to the person-name autocomplete, not just people entered on acts | ✓ VERIFIED | `suggest_person`'s new `given_by_name_arm`/`given_to_name_arm` UNION ALL arcs in `act_service.rs:2504-2579`, reading `audit_log.payload_json` via `json_extract`, ranked by the same frequency-DESC ordering as the pre-existing arcs (plan 40-34, commit `a33a6cba`). Confirmed live on Windows (round 4). |
| 8 | The person-name autocomplete does not pop open on a programmatic prefill, only on real user focus/input | ✓ VERIFIED | `PersonAutocomplete.svelte`'s `internalUpdate` flag (set synchronously before `handleInput`/`select` assign `value`, read-and-reset at the top of the tracking `$effect`) distinguishes external/async prefill from user-driven change; `hasOpenableSuggestions` additionally suppresses a single self-matching suggestion (commit `70b257ff`). Confirmed live on Windows (round 5). |
| 9 | Bulk-moving a place's contents updates the places-tree device/cartridge counters for the source, destination, and all ancestors without freezing the page | ✓ VERIFIED | `PlaceTree.svelte`'s invalidation `$effect` reads `statsCache` inside `untrack()` and only writes back when at least one key was actually deleted — breaks the self-referential write-triggers-rerun cycle that caused `effect_update_depth_exceeded` (UAT3-03a, commit `5042e674`). Confirmed live on Windows (round 3). |
| 10 | LAN-browser PDF export/print of the «Перемещения» report does not duplicate the first page | ✓ VERIFIED | `PdfPreviewModal.svelte::printViaTopLevel` now collects `Array.from(parsed.querySelectorAll('style'))` (whole document, not just `head > style`) and removes each before building `bodyHtml`, mirroring pagedjs's own `removeStyles()` so a stray `<style>` tag from `_header.html` never flows into the paginated content stream (root-caused against real pagedjs 0.4.3, commit `08e56c25`). Confirmed live on Windows (round 3). |
| 11 | The four place-related resolvers introduced across this phase each answer a genuinely distinct question with exactly one owner | ✓ VERIFIED | See Method steps 1-3. `most_common_to_refill_destination` fully deleted (zero references). No duplicated formula found. |

**Score:** 11/11 truths verified (4 carried-forward + 7 new this round), 0 failed, 0 uncertain.

### Roadmap Success Criteria

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC1 | Пользователь видит таймлайн перемещений в карточке устройства и картриджа | ✓ VERIFIED | Unchanged, stable across rounds 3-5. |
| SC2 | Ручное изменение места фиксируется в истории с причиной «вручную» | ✓ VERIFIED | Unchanged. |
| SC3 | Акт приёма-передачи автоматически меняет место и создаёт запись в истории со ссылкой на номер акта | ✓ VERIFIED | Unchanged. |
| SC4 | Пользователь может получить отчёт о перемещениях за период с фильтром по месту и типу устройства | ✓ VERIFIED | Print/export defect (UAT3-02) that regressed this criterion's LAN-browser path is now closed and live-confirmed. |

### Required Artifacts (Rounds 3-5)

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs::place_before_last_to_refill` | Own-history from_refill default | ✓ VERIFIED | Present, single owner, tested (`place_before_last_to_refill_*` unit tests). |
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs::latest_to_refill_send` | Single-row source for all 3 to_refill dialog fields | ✓ VERIFIED | Present, single owner, tested (6 unit tests incl. `latest_to_refill_send_picks_most_recent_not_most_frequent`). |
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs::latest_to_refill_source_place` | Global from_refill fallback, skips sourceless rows | ✓ VERIFIED | Present, single owner, tested (3 unit tests incl. `latest_to_refill_source_place_skips_freshest_row_without_source`). |
| `crates/trackly-app/src/services/cartridge_service.rs::operation_default_place` | Wires the from_refill two-step chain | ✓ VERIFIED | Lines 1023-1027, matches doc-comment exactly. |
| `crates/trackly-app/src/services/cartridge_service.rs::to_refill_last_send` | Wires latest_to_refill_send into a DTO | ✓ VERIFIED | Lines 1046-1067. |
| `ui/src/lib/components/PersonAutocomplete.svelte` | internalUpdate + hasOpenableSuggestions gating | ✓ VERIFIED | Both present, both load-bearing per the UAT5-01 root-cause writeup. |
| `ui/src/features/places/PlaceTree.svelte` | untrack + conditional-write invalidation effect | ✓ VERIFIED | Present, doc-comment explicitly warns against removing either half. |
| `ui/src/features/acts/PdfPreviewModal.svelte::printViaTopLevel` | Whole-document style removal before bodyHtml | ✓ VERIFIED | Present, root-caused against real pagedjs. |
| `ui/src/features/cartridges/OperationModal.svelte` | DEC-B autofill-clear gated to `op === 'install'` only | ✓ VERIFIED | Line 373. |
| `crates/trackly-app/src/services/act_service.rs::suggest_person` | given_by_name_arm / given_to_name_arm UNION ALL | ✓ VERIFIED | Lines 2527-2557, both present and field-appropriate. |

### Key Link Verification

| From | To | Via | Status |
|------|-----|-----|--------|
| `OperationModal.svelte` (to_refill/from_refill open) | `cartridges.operationDefaultPlace` / `cartridges.toRefillLastSend` | API wrapper calls | ✓ WIRED |
| `cartridge_service.rs::operation_default_place` ("from_refill") | `cart_repo.place_before_last_to_refill` → `cart_repo.latest_to_refill_source_place` | short-circuit chain, single reader-pool acquire | ✓ WIRED |
| `act_service.rs::suggest_person` (Receiver field) | `audit_log.payload_json->given_to_name` | UNION ALL arm | ✓ WIRED |
| `PlaceContents.svelte` bulk-move | `PlaceTree.svelte` invalidation effect | `placeContentEventsStore` | ✓ WIRED |
| `PdfPreviewModal.svelte::printViaTopLevel` | pagedjs `Previewer.preview()` | explicit `stylesheets` arg + pre-stripped body | ✓ WIRED |

### Requirements Coverage

| Requirement | Description | Status | Evidence |
|---|---|---|---|
| HST-01 | Каждая смена места фиксируется в истории с откуда/куда/когда/кем/почему | ✓ SATISFIED | Write-site defects closed (rounds 1-2); refill-dispatch usability defaults (rounds 3-5) all live-confirmed on real hardware. |
| HST-02 | Пользователь видит таймлайн перемещений в карточке устройства и картриджа | ✓ SATISFIED | Read-path availability (CR-01) closed and now additionally confirmed under live (non-synthetic) concurrent LAN use. |
| HST-03 | Акт приёма-передачи автоматически меняет место и фиксирует ссылку на номер акта в истории | ✓ SATISFIED | Unchanged since round 2, stable. |
| HST-04 | Пользователь может получить отчёт о перемещениях с фильтром по месту и типу устройства | ✓ SATISFIED | LAN print/export duplicate-page regression (UAT3-02) closed and live-confirmed — this was the last open thread under this criterion. |

Note: `.planning/REQUIREMENTS.md`'s tracking table (lines 155-158) still shows all four HST-* rows
as "In Progress" while the checklist above it (lines 45-50) already has all four checked `[x]`.
Same stale-documentation issue flagged in the round-2 report, still present — recommend closing in
a housekeeping pass, not a phase-goal blocker.

### Anti-Patterns Found

None new. No `TBD`/`FIXME`/`XXX` in any file touched by plans 40-30 through 40-35 or the four
direct-commit fixes (`3a5697cb`, `19f64449`, `08e56c25`, `5042e674`, `70b257ff`, `c8e9940b`,
`d67c6d1f`). Working tree clean at HEAD.

### Behavioral / Live-UAT Spot-Checks

| Behavior | Method | Result | Status |
|----------|--------|--------|--------|
| Refill send/receive defaults | Live UAT rounds 3-5, real Windows build | All confirmed working, incl. edge cases (refill place itself marked storage, cartridge with no own history) | ✓ PASS |
| Autocomplete does not pop open on prefill | Live UAT round 5, screenshot-documented before/after | Confirmed fixed | ✓ PASS |
| Places-tree counters after bulk move | Live UAT round 3, page-freeze defect found and fixed | Confirmed fixed, no freeze | ✓ PASS |
| LAN PDF print/export duplicate page | Live UAT round 3 + independent debug-session reproduction against real pagedjs 0.4.3 | Confirmed fixed | ✓ PASS |
| Concurrent LAN reads | Live UAT round 2 re-run (test #2 in `40-HUMAN-UAT.md`) | No hangs observed during ordinary multi-session use | ✓ PASS |
| Printer place-clear via standard UI | Live UAT test #3 | Confirmed unreachable, as predicted; no defect | ✓ PASS |

### Human Verification Required

None remaining. All three items from the prior report's `human_verification:` section are closed,
confirmed by the user directly on the target platform (Windows, build 1.4.0-phase40), across three
additional rounds of live testing (2026-09-04), concluding with an explicit confirmation quote in
`40-HUMAN-UAT.md`'s "## Итог живого UAT" section.

## Gaps Summary

**No code-level or goal-blocking gaps remain.** All observable truths (11/11) hold at HEAD, all
four place-related resolvers are distinct with a single owner each, `most_common_to_refill_destination`
is fully deleted, and every fix from live-UAT rounds 3-5 is present, wired, and independently
confirmed by reading the actual code (not SUMMARY.md prose).

Three items are carried forward as **honest, non-blocking deferrals**, all already disclosed in
the phase's own artifacts:

1. **6 of 7 nullable `DevicePatch` fields** still can't be cleared (COALESCE bug) — logged in
   `40-28-SUMMARY.md` and `40-REVIEW.md` (WR-03) as a deliberate, transparent scope decision; not a
   regression (all 7 were equally broken before this phase touched any of them).
2. **`users_crud.rs` flaky tests** under full-package load — logged in `deferred-items.md`,
   root-caused as an argon2id-cost/test-timeout collision, unrelated to movement-history's own
   files.
3. **REQUIREMENTS.md tracking-table staleness** — cosmetic, flagged in round 2, still present.

**One item was flagged as untraceable and has since been made durable (orchestrator note,
2026-09-04):** the verification brief referenced a "spawned follow-up task" for first-admin
bootstrap over HTTP/LAN. The verifier searched `.planning/` exhaustively, found nothing, and
refused to assume it was handled — that was the correct call. The item genuinely existed only as
an orchestrator-side session task chip, outside any grep-able planning file, and the brief's
wording wrongly implied a planning artifact. It is now recorded in `deferred-items.md` with its
mechanism and the security consideration a fix must address (while the app sits in bootstrap
state, anyone who can reach the port could claim the admin account). It does not block the Phase
40 goal — no HST-01..04 truth depends on it — and it was found during phase-40 live verification
on an empty database rather than caused by phase-40 code.

`STATE.md`'s "Current Position" narrative (`last_updated: 2026-09-04T12:03:46Z`) also predates
rounds 4-5 and still describes 40-31/40-32 as "pending — проверить их фактический статус"; this is
stale relative to HEAD (both are complete and live-confirmed) and should be refreshed in the
phase-close housekeeping pass, but is not itself evidence of a code gap.

---

_Verified: 2026-09-04T22:30:00Z_
_Verifier: Claude (gsd-verifier)_
