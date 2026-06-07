---
phase: "04"
plan: "02"
subsystem: cartridges
tags: [domain, ports, infra, repository, hexagonal, sql, fts, lifecycle]
dependency_graph:
  requires:
    - V016 migration (04-01) — cartridge_kinds, color, app_settings, FTS triggers
    - crates/trackly-core (domain/acts.rs, ports/acts.rs as patterns)
    - crates/trackly-infra (acts_sqlite increment_counter_in_tx, audit_log_sqlite)
  provides:
    - trackly-core::domain::cartridges (CartridgeRow, CartridgeModelRow, CartridgeNew, CartridgeModelNew, CartridgeTransitionOp, CartridgeFilter, CartridgeCounts, LowStockItem, Pagination)
    - trackly-core::ports::cartridges (CartridgeRepository trait)
    - trackly-infra::repos::cartridges_sqlite (SqliteCartridgeRepository + all SQL helpers)
  affects:
    - crates/trackly-core/src/domain/mod.rs (pub mod cartridges added)
    - crates/trackly-core/src/ports/mod.rs (pub mod cartridges added)
    - crates/trackly-infra/src/repos/mod.rs (pub mod + pub use cartridges_sqlite added)
tech_stack:
  added: []
  patterns:
    - "Hexagonal: CartridgeRepository trait in core, impl in infra — rusqlite never touches trackly-core"
    - "CartridgeTransitionOp domain enum with validate_from_status + audit_action + target_status_id"
    - "assign_code_in_tx: C-NNNNNN auto-code with retry loop on UNIQUE collision (counter never lost)"
    - "FTS5 MATCH + LIKE UNION CTE search with double-quote escaping (T-04-02-01)"
    - "INSERT OR IGNORE location round-trip pattern for shared autocomplete"
    - "transition_in_tx: fetch snapshot → validate op → UPDATE → location round-trip → audit_log"
    - "low_stock: GROUP BY HAVING with threshold read from app_settings"
key_files:
  created:
    - crates/trackly-core/src/domain/cartridges.rs
    - crates/trackly-core/src/ports/cartridges.rs
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
  modified:
    - crates/trackly-core/src/domain/mod.rs
    - crates/trackly-core/src/ports/mod.rs
    - crates/trackly-infra/src/repos/mod.rs
decisions:
  - "CartridgeTransitionOp placed in domain layer (not dto) with validate_from_status helper — domain logic stays close to domain types"
  - "transition_in_tx validates status transition via CartridgeTransitionOp::validate_from_status (domain rule) before any SQL UPDATE"
  - "FTS query double-quotes escaped via str::replace before MATCH — prevents FTS5 parse errors on user input (T-04-02-01)"
  - "assign_code_in_tx retry loop: counter increment is unconditional, code uniqueness checked after — counter never rolls back on collision (D-Code-Override-01)"
  - "soft_delete_model_in_tx has live-instance guard — Conflict returned if cartridges reference the model being deleted"
  - "AuditEntryRow struct defined in cartridges_sqlite.rs for history read path — keeps audit_log_sqlite generic"
metrics:
  duration: "6 min"
  completed: "2026-06-07"
  tasks: 2
  files: 6
---

# Phase 04 Plan 02: Hexagonal Layer — Domain + Port + Infra Repo

Domain structs, CartridgeRepository port trait, and SqliteCartridgeRepository with full SQL helpers for the cartridge lifecycle — all in 6 minutes.

## Summary

CartridgeRow/CartridgeModelRow/CartridgeTransitionOp in trackly-core domain layer (rusqlite-free), CartridgeRepository trait in ports, SqliteCartridgeRepository in trackly-infra with assign_code_in_tx (retry loop), transition_in_tx (status validation + audit), search (FTS5+LIKE UNION CTE), low_stock, and get_history.

## Task Completion

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Domain structs + Port trait (trackly-core) | 9a71b50 | domain/cartridges.rs, ports/cartridges.rs, domain/mod.rs, ports/mod.rs |
| 2 | SqliteCartridgeRepository SQL helpers (trackly-infra) | 9de02f3 | repos/cartridges_sqlite.rs, repos/mod.rs |

## Verification Results

- `cargo check -p trackly-core` — 0 errors
- `cargo check -p trackly-infra` — 0 errors
- `rusqlite` NOT present in `crates/trackly-core/src/domain/cartridges.rs` — hexagonal boundary intact
- 26 `params![]` usages in cartridges_sqlite.rs — all SQL parameterized (T-04-02-01 through T-04-02-05 mitigated)
- 5 domain tests + 8 infra tests = 13 tests passing

## Threat Mitigations Applied

| Threat ID | Status | Implementation |
|-----------|--------|----------------|
| T-04-02-01 | mitigated | FTS MATCH param via params![]; double-quotes escaped before MATCH |
| T-04-02-02 | mitigated | assign_code_in_tx: parameterized SELECT EXISTS before INSERT; AppError::Conflict on collision |
| T-04-02-03 | mitigated | single-writer + BEGIN IMMEDIATE (conn.transaction() default) + retry loop |
| T-04-02-04 | mitigated | validate_from_status() called in transition_in_tx before UPDATE |
| T-04-02-05 | mitigated | location is plain text value stored via params![] — no path traversal surface |

## Deviations from Plan

### Auto-added correctness improvements

**1. [Rule 2 - Critical] validate_from_status on CartridgeTransitionOp domain type**
- Found during: Task 1 implementation
- Issue: Plan specified validation only in transition_in_tx (infra layer); domain types had no self-validation
- Fix: Added `validate_from_status`, `audit_action`, `target_status_id` helper methods on CartridgeTransitionOp in domain layer — keeps domain rules in domain code
- Files modified: crates/trackly-core/src/domain/cartridges.rs
- Commit: 9a71b50

**2. [Rule 2 - Critical] AuditEntryRow struct for history read path**
- Found during: Task 2 implementation
- Issue: Plan referenced returning `Vec<AuditEntryDto>` from get_history, but no DTO exists yet (plan 04-03)
- Fix: Defined `AuditEntryRow` struct in cartridges_sqlite.rs for the repo layer — CartridgeService in plan 04-03 will map to DTO
- Files modified: crates/trackly-infra/src/repos/cartridges_sqlite.rs
- Commit: 9de02f3

**3. [Rule 2 - Security] soft_delete_model_in_tx live-instance guard**
- Found during: Task 2 implementation
- Issue: Plan mentioned conflict on model delete but didn't specify guard location
- Fix: Added `SELECT COUNT(*) FROM cartridges WHERE model_id=? AND deleted_at_utc IS NULL` check in soft_delete_model_in_tx before UPDATE — returns AppError::Conflict if live instances exist
- Files modified: crates/trackly-infra/src/repos/cartridges_sqlite.rs
- Commit: 9de02f3

## Known Stubs

None — this plan creates a data layer with no UI rendering paths.

## Threat Flags

None — no new network endpoints, auth paths, or file access patterns introduced.

## Self-Check: PASSED

- crates/trackly-core/src/domain/cartridges.rs — FOUND (342 lines)
- crates/trackly-core/src/ports/cartridges.rs — FOUND (52 lines)
- crates/trackly-infra/src/repos/cartridges_sqlite.rs — FOUND (1221 lines)
- Commit 9a71b50 — FOUND
- Commit 9de02f3 — FOUND
