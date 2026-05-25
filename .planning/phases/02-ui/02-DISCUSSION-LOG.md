# Phase 2 Discussion Log

**Mode:** `/gsd-discuss-phase 2 --auto` — fully autonomous, no AskUserQuestion prompts.
**Date:** 2026-05-25

> This log is for human reference only (audits, retrospectives) and is NOT consumed by downstream agents (researcher, planner, executor). The canonical artefact is `02-CONTEXT.md`.

## Prior context loaded

- `PROJECT.md`, `REQUIREMENTS.md`, `STATE.md`, `.planning/phases/01-foundation/01-CONTEXT.md`, 6× Phase 1 SUMMARY.md, `.planning/phases/01-foundation/deferred-items.md`.
- `.planning/research/ARCHITECTURE.md`, `.planning/research/STACK.md`, `.planning/research/PITFALLS.md`, `.planning/research/SUMMARY.md`.

## Cross-referenced todos

`gsd-sdk query todo.match-phase 2` → 0 matches. No external TODO items folded.

## Codebase scout

Phase 1 source tree enumerated. Reusable assets + integration points captured in `02-CONTEXT.md` `<code_context>` section.

## Gray areas — auto-resolved (single pass)

All gray areas auto-selected in `--auto` mode (per `workflows/discuss-phase/modes/auto.md`). Decisions taken with best-practice defaults from Phase 1 patterns + research notes.

### Backend layer

| Area | Decision | Default rationale |
|------|----------|-------------------|
| Repo/Service layout | hexagonal: port в core, repo в infra, service в app | Matches Phase 1 ARCHITECTURE.md + Plan 04 AppCtx pattern. |
| Migrations for Phase 2 | NEW `V013__devices_fts_triggers.sql` only — base schema already in V003 + V012 from Phase 1 | Plan 03 explicitly deferred FTS triggers to phases that own the CRUD paths. |
| Autocomplete strategy | DISTINCT + partial indexes (NO precomputed table) | YAGNI; sub-50ms on ~10k rows. Premature optimisation avoided. |
| Non-unique device grouping | `(type, name, model, specs, kit, state, location, status)` as group key; server-side `GROUP BY` | REQ-DEV-03/11 explicit; server-side cheaper than client agg. |
| FTS5 search | trigger-synced index, `MATCH` operator, prefix-search via `term*` sanitiser | Native SQLite pattern. |
| CSV import encoding sniff | BOM check → `chardetng` heuristic → `encoding_rs` decode | Standard Rust trio; covers UTF-8/UTF-8-BOM/CP1251. |
| CSV import delimiter | count `,` vs `;` in first non-empty line outside quotes | Heuristic enough for v1. |
| CSV import workflow | preview-then-commit, in-memory state with 5-min TTL token | Avoids server-side temp files; aligns with portable mode. |
| CSV export delimiter + encoding | UTF-8 BOM + `;` (RU Excel-friendly) | Standard для RU-locale Excel. |
| Autocomplete endpoint | one `devices_autocomplete(field, prefix, ctx_name=None)` | One UI hook on the frontend. |
| Device state hints (DEV-10) | static `const` array in DTO + `device_state_hints()` command | Not data — it's a UI affordance. |

### Frontend layer

| Area | Decision | Default rationale |
|------|----------|-------------------|
| Folder structure | feature-folders + shared `lib/` | Scales across 8 phases without restructuring. |
| Routing | `svelte-spa-router` (hash-routing) | Identical behaviour in Tauri webview + browser; no axum rewrites needed. |
| State management | Svelte 5 runes (`$state` at module level for app-state) | Modern Svelte 5 idiom; no legacy stores. |
| Theme switcher | `data-theme` attr + inline no-flash script in `<head>` + localStorage | Standard pattern, zero JS deps, works pre-hydration. |
| Transport detection | runtime check `'__TAURI_INTERNALS__' in window` (Tauri 2's marker) | Plan 1 ARCHITECTURE.md mandates dual-transport from single bundle. |
| Error UX | hand-rolled `Toast` + `ToastHost`, parses `AppError.message` (russian, server-side) | UI-06; ~80 LoC, no library. |
| Form validation | manual via runes; sserver-side validation via `AppError::Validation` | Simplicity > generality for 4 required fields. |
| Pagination | server-side, 50 per page, simple page-jump (virtual scroll deferred) | Sweet spot для 1280×720. |
| Responsive | fixed sidebar 240px, content overflow, min-target 1280×720 (no mobile breakpoints) | Scope is desktop + LAN browser. |
| i18n | hard-coded Russian; Paraglide deferred | YAGNI for one-locale v1. |
| Sidebar structure | exact UI-01 ordering with dividers; placeholders for non-Phase-2 sections | UI-01 explicit. |

### Composition + Tests

| Area | Decision |
|------|----------|
| `AppCtx` extension | `devices: Arc<DeviceService>` — 9th field after Phase 1's 8 |
| Integration tests | extend `test_writer_and_readers` fixture → `test_device_service()`; per-command `tests/devices_*.rs` files |
| CSV tests | real fixture files in `tests/fixtures/devices/` (UTF-8, UTF-8 BOM, CP1251, both `,` and `;`); cyrillic fixture string from Phase 1 specifics |
| `specta_export::builder()` | extend `collect_commands![...]` with ~12 device-related commands |

### Phase 1 deferred-items closure

| Item | Action in Phase 2 |
|------|-------------------|
| `@tauri-apps/api` missing from `ui/package.json` | ADD as runtime dep (`"^2"`) |
| `pnpm svelte-check` `continue-on-error: true` | REMOVE from both workflows (svelte-check теперь должен быть зелёным) |
| `tests/export_bindings.rs` Windows-skip | NOT addressed (still blocked on stable-Rust specta upgrade) |

## Scope creep — none flagged

No suggestions outside phase boundary surfaced during auto-resolution.

## Next step

Auto-advance to `/gsd-plan-phase 2 --auto` (chain mode).
