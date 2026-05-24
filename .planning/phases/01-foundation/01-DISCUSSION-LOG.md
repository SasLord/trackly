# Phase 1: Фундамент - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-24
**Phase:** 1-Фундамент
**Areas discussed:** All gray areas resolved by Claude per user's «do as you think right following best practices» directive

---

## Initial gray-area selection

Claude presented 4 gray areas for selection (after collapsing initial 6 due to AskUserQuestion 4-option limit):

| Option | Description | Selected |
|--------|-------------|----------|
| Схема: ID, timestamps, soft-delete scope | INTEGER vs UUID v7 PK; timestamp storage; soft-delete scope; audit_log shape | (delegated) |
| Workspace и bindings.ts pipeline | Crate naming, UI folder location, tauri-specta generation strategy | (delegated) |
| Миграции и seed-данные | Single file vs split; seed location; lookup evolution | (delegated) |
| CI-матрица, тесты, config/логи | CI strategy; ProcMon test cadence; test DB strategy; config format; log defaults | (delegated) |

**User's choice:** «Делай как считаешь правильным следуя лучшим практикам. Если есть какие-то вопросы где сомневаешься, тогда задай этот вопрос мне.»

**Notes:** User delegated all decisions to Claude with the instruction to follow best practices and only ask if genuinely uncertain. Claude proceeded to resolve every area inline using research (SUMMARY.md, ARCHITECTURE.md, STACK.md, PITFALLS.md, FEATURES.md) and standard Rust/Tauri ecosystem conventions.

---

## Schema decisions (resolved inline)

| Option | Description | Selected |
|--------|-------------|----------|
| INTEGER PRIMARY KEY AUTOINCREMENT | Compact, fast joins, standard SQLite | ✓ |
| UUID v7 PK | Distributed-friendly, stable public ID | (deferred — no public sharing in v1) |
| Timestamps INTEGER unix seconds (`*_at_utc`) | Compact, fast comparison, no TZ ambiguity | ✓ |
| Timestamps TEXT ISO-8601 | Human-readable | (rejected — larger indexes, slower compare) |
| Soft-delete on Acts/Devices/Cartridges only | Minimum per SUMMARY.md | (extended) |
| Soft-delete on all user-mutable entities | Acts, Devices, Cartridges, Users, Requests, Templates, Locations | ✓ |
| audit_log full before/after JSON | Heavier, exact undo | ✓ |
| audit_log minimal delta | Lighter, complex undo reconstruction | (rejected — undo is core feature) |

**Notes:** Soft-delete extended beyond SUMMARY.md minimum because schema cost is cheap and retrofit is expensive. UUID PK deferred — adding `public_id TEXT UNIQUE` later is cheap if needed.

---

## Workspace and bindings.ts pipeline

| Option | Description | Selected |
|--------|-------------|----------|
| Binary name `trackly` in `trackly-app` crate | Matches project name | ✓ |
| UI folder `ui/` at workspace root | Clear separation, short path | ✓ |
| UI folder `frontend/` | Common naming convention | (rejected — `ui/` more concise) |
| UI folder inside `trackly-app/src-ui/` | Co-located with shell | (rejected — confuses build tooling) |
| bindings.ts generated in `cargo test`, gitignored | Always in-sync, no git noise | ✓ |
| bindings.ts committed to git | Visible diffs | (rejected — generated artifact churn) |
| tauri-specta via build.rs | Compile-time | (rejected — slows every cargo build) |
| tauri-specta via `npm prebuild` calling `cargo test` | Vite-driven, explicit | ✓ |

---

## Migrations and seeds

| Option | Description | Selected |
|--------|-------------|----------|
| Single `001_initial.sql` (~600 LoC) | One transaction, atomic | (rejected — unreadable, hard to review) |
| Split per domain (V001–V012) | Readable history, easy to blame | ✓ |
| Seed in V001 alongside lookup schema | Atomic, single startup path | ✓ |
| Seed in separate Vn | Visible separation | (rejected — lookups always go with schema) |
| Seed at runtime via INSERT OR IGNORE | Idempotent, evolvable | (rejected — opaque history) |
| Lookup evolution via new Vn migrations (idempotent) | Forward-only, auditable | ✓ |
| device_types seed: Устройство + Принтер | Per SUMMARY.md Resolved Decisions | ✓ |
| device_types seed: Устройство + Принтер + Расходник | Per original PROJECT.md | (rejected — see SUMMARY.md resolution) |

---

## CI matrix, tests, config, logs

| Option | Description | Selected |
|--------|-------------|----------|
| Fast checks on every push + full matrix on PR/main | Balanced cost vs feedback | ✓ |
| Full matrix on every push | Maximum coverage | (rejected — GH minutes cost, slow feedback) |
| Only PR checks | Cheap but loses pre-push signal | (rejected — too late) |
| ProcMon test on every PR (Windows runner) | Catches regressions early | ✓ |
| ProcMon test nightly only | Cheaper | (rejected — portable mode is core constraint) |
| Test DB: `:memory:` per test | Fast, no cleanup | (rejected — doesn't model WAL) |
| Test DB: tempfile per test | Real WAL behavior | ✓ |
| Config: TOML | Hand-editable, comments, Rust-standard | ✓ |
| Config: JSON | Lighter parser | (rejected — no comments, trailing-comma footguns) |
| Logs: compact human default, JSON via config toggle | Ops-friendly + agg-friendly | ✓ |
| Daily rotation, 14-day retention default | Standard | ✓ (cleanup worker deferred to Phase 7) |

---

## Single-writer channel and AppError

| Option | Description | Selected |
|--------|-------------|----------|
| Bounded mpsc capacity 256 + 5s send_timeout | Backpressure visible, no memory leak | ✓ |
| Unbounded mpsc | No timeouts | (rejected — masks runaway memory on hangs) |
| AppError as flat enum, identical JSON shape across transports | Simpler tauri-specta, simpler frontend | ✓ |
| AppError as nested per-domain enums | Modularity | (rejected — adds tagged-union complexity) |

---

## Claude's Discretion

The user explicitly delegated all decisions. Claude marked the following as «open to planner refinement» inside CONTEXT.md:
- Exact DTO field names (e.g., `HealthDto` smoke test fields).
- `Paths` struct method names and shape.
- `AppError.details` per-variant payload schema (overall shape locked).
- Test file naming conventions.
- Specific non-obvious indexes in `V012__indexes_and_fts.sql` (PK/FK/UNIQUE indexes are mandatory).

## Deferred Ideas

Recorded in CONTEXT.md `<deferred>` section:
- Корзина UI поверх soft-delete (Phase 7+).
- `device_custom_fields` table — defer to «if users complain».
- Логотип BLOB → Phase 7.
- Backup retention policy + scheduled_tasks worker → Phase 7.
- audit_log retention cleanup — schema ready, no policy in v1.
- Windows manifest `activeCodePage=UTF-8` → Phase 8 release pipeline (raise earlier if CI cyrillic test demands).
- mDNS `.local` HTTPS strategy → Phase 5.
- `tauri-plugin-single-instance` — noted as best-practice candidate for Phase 1 planning even though not in success criteria.
