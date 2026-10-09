---
phase: quick-261009-ul8
plan: 01
subsystem: test-infrastructure / registry gate EntitiesChanged
tags: [gate, scanner, mutation-testing, WR-06, WR-07, D-04, D-08]
requires: [41.7 registry gate (41.7-14)]
provides:
  - "scan.rs: Kind::BroadcastBeforeWriter / BroadcastWithoutWrite / ExemptBroadcasts, transitive write points, scan_with_stats"
  - "gate tests: selftests, sweeps over all 31 Broadcasts and 21 Exempt rows, named moves, measured-something test"
affects: [phases 43-45 (rely on the gate)]
key-files:
  modified:
    - crates/trackly-app/tests/entities_support/scan.rs
    - crates/trackly-app/tests/entities_support/registry.rs (doc-comment only)
    - crates/trackly-app/tests/entities_changed_gate.rs
    - .planning/phases/41.7-v1-4-ws-bootstrap-lan/41.7-REVIEW.md
    - .planning/phases/41.7-v1-4-ws-bootstrap-lan/41.7-SECURITY.md
    - .planning/todos/pending/2026-10-09-review-41.7-residual-findings.md
decisions:
  - "A write point is writer.execute(..) OR a call to a same-file self./Self:: method that transitively owns a write (fixpoint); recursion is not a write point"
  - "Broadcasts row with no visible write point fails loudly (BroadcastWithoutWrite) instead of passing"
  - "Exempt row may not call broadcast_entities at all (ExemptBroadcasts; inside writer parens -> BroadcastInsideWriter)"
metrics:
  tests_before: 34
  tests_after: 49
  registry_rows: "118 (31 Broadcasts / 21 Exempt / 66 ReadOnly), unchanged"
---

# Quick 261009-ul8: WR-06 / WR-07 registry gate strengthening Summary

Registry gate now locks the D-04 contract (broadcast strictly AFTER the last write point, outside the writer closure) and gives `Exempt` rows a mechanical check (no `broadcast_entities` at all), with write points resolved transitively inside a file so `archive` (via `set_archived`) and `device::create` (via `create_without_broadcast` -> `insert_new_and_get`) do not false-positive.

## Commits

- `0db2d02b` test(41.7): гейт EntitiesChanged - порядок рассылки после записи и механика Exempt (WR-06, WR-07) - scan.rs, registry.rs (doc only), entities_changed_gate.rs
- `f266ccc2` docs(41.7): WR-06 и WR-07 закрыты - REVIEW, SECURITY, todo

## What changed

- `scan.rs`: new `Kind`s `BroadcastBeforeWriter`, `BroadcastWithoutWrite`, `ExemptBroadcasts`; `writing_fns` (fixpoint over same-file self-calls), `write_points`, `writer_call_ranges` (pub), `ScanStats` and `scan_with_stats` (`scan` is a thin wrapper). Existing `MissingBroadcast` / `BroadcastInsideWriter` logic untouched; a broadcast already reported as inside-writer is not double-reported.
- `registry.rs`: Exempt doc-comment only. 118 rows unchanged.
- `entities_changed_gate.rs`: 10 new selftests (synthetic, fictional identifiers), 3 real-source tests (`mutation_early_broadcast_in_every_broadcasts_row_turns_gate_red`, `mutation_broadcast_moved_before_write_on_real_sources_is_red`, `mutation_broadcast_in_every_exempt_row_turns_gate_red`) and `gate_new_checks_measured_something`. Anchors asserted unique in code (`replace_unique` asserts `matches(anchor).count() == 1` before replacing; sweeps assert function-name uniqueness and `{` at the body offset). The file still has no `fs::write`.

## Evidence

Scoped run, foreground: `cargo test -p trackly-app --test entities_changed_gate` -> **49 passed / 0 failed / 0 filtered out** (baseline 34/0/0; +15 tests).

Sweep sizes asserted equal to registry counts: 31 Broadcasts rows ordered, 21 Exempt rows checked; Exempt sweep asserts `group_place.rs` and `group_membership.rs` were covered and >= 3 writer-owning Exempt functions. `gate_new_checks_measured_something` asserts `broadcasts_ordered == 31`, `exempt_checked == 21`, floors 30 / 20, zero violations.

### On-disk RED -> revert -> GREEN (real service files)

Three simultaneous mutations (anchor uniqueness asserted in the script):
1. `place_service.rs::create` - `broadcast_entities` inserted right after the signature (WR-06)
2. `place_service.rs::set_archived` - local `broadcast_entities` closure call inserted before `repo.archive(..)` inside the writer closure (WR-07, inside writer)
3. `group_place.rs::apply_group_place_to_device_in_tx` - same pair right after the body `{` (WR-07, outside writer)

`every_service_mutation_has_verdict_and_broadcasts` -> RED, exit 101, exactly three violations:
```
гейт слоя (1) красный, нарушений 3:
  - place_service.rs::create [BroadcastBeforeWriter] ...
  - place_service.rs::set_archived [BroadcastInsideWriter] ...
  - group_place.rs::apply_group_place_to_device_in_tx [ExemptBroadcasts] ...
test result: FAILED. 0 passed; 1 failed; ... 48 filtered out
```
After `git checkout -- place_service.rs group_place.rs` (`git status --short crates/trackly-app/src/` empty): same test `1 passed`, full file `49 passed; 0 failed`.

## Нарушения, вскрытые укреплением

Нет. Порядок верен на всех 31 точке Broadcasts, ни одна из 21 Exempt-функций не рассылает; боевой скан зелёный на неизменённых исходниках с первого прогона. Ни правило, ни реестр, ни сканер не смягчались.

## Deviations from Plan

- **Order of TDD steps (minor):** the scanner was written before the selftests were added, so a formal "tests red / not compiling" step was not recorded. Bite is instead proven on real sources (sweeps over all rows plus the on-disk RED/GREEN above), which the plan names as the binding requirement.
- Task 1 and Task 2 share one file (`entities_changed_gate.rs`), so they went into a single code commit as the plan's Part B prescribes, not two.
- Doc fix beyond the plan text: in `41.7-SECURITY.md` the Residual intro sentence and the Approval line were reworded (R-3, R-4 remain; R-1, R-2 closed); status `verified` / `threats_open: 0` unchanged.

## Known limitation (recorded, not a stub)

Write points are searched only within the same file. A `Broadcasts` row whose write is entirely in another file/object fails loudly (`BroadcastWithoutWrite`); a mixed body (some visible write, some external write after the broadcast) is not caught - layer (2) behavioural scenarios cover that.

## Hygiene

`rustfmt --edition 2021 --check` on the touched test files is clean (no `cargo fmt -p trackly-app` was run). `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` PASS, 0 violations, before both commits. No push performed.

## Self-Check: PASSED

- scan.rs, registry.rs, entities_changed_gate.rs modified: FOUND
- commits `0db2d02b`, `f266ccc2`: FOUND
- `crates/trackly-app/src/` working tree clean after revert: confirmed
