---
phase: 01-foundation
plan: 01
subsystem: infra
tags: [rust, cargo, workspace, tauri, svelte, vite, pnpm, clippy, cargo-deny, github-actions, msrv]

requires: []
provides:
  - 4-member Cargo workspace (trackly-core, trackly-infra, trackly-app, procmon-check) with shared [workspace.dependencies] pinning all stack deps incl. axum/tower/tower-http for Plan 05 consumption
  - Workspace-wide clippy disallowed-methods gate (10 entries) + disallowed-types (1 entry) enforcing portable-mode and UTC-only discipline at compile time
  - trackly-core integration test (no_io_deps.rs) that fails CI if tokio/rusqlite/tauri/axum/hyper/tower/reqwest/sqlx/libsqlite3-sys ever enter the core dep closure
  - Svelte 5 + Vite 6 SPA scaffold in ui/ with svelte-check + eslint-9-flat-config + prettier gates green
  - GitHub Actions ci-fast.yml (5 gates: fmt, clippy, test, svelte-check, lint) on every push + PR; cargo-deny.yml daily cron
  - rust-toolchain.toml pinning the MSRV so every contributor and CI runner uses the identical compiler
  - trackly binary stub accepts --self-test (Plans 02-04 replace with real lifecycle)
  - tauri.conf.json v2 with frontendDist=../../ui/dist, devUrl=:1420, no updater plugin (portable-safe)
affects: [02-paths-config, 03-schema-migrations, 04-appctx-writer, 05-tauri-specta-axum, 06-procmon-ci, all-future-plans]

tech-stack:
  added: [Rust 1.88, tauri 2.11, tauri-build 2, tauri-specta 2.0.0-rc.21, specta 2.0.0-rc.22, specta-typescript 0.0.9, tauri-plugin-single-instance 2, tokio 1, tokio-util 0.7, rusqlite 0.38 (bundled), refinery 0.9 (rusqlite-bundled), serde 1, serde_json 1, serde_with 3 (time_0_3), time 0.3, thiserror 2, anyhow 1, tracing 0.1, tracing-subscriber 0.3, tracing-appender 0.2, toml 0.8, tempfile 3, zeroize 1 (derive), async-trait 0.1, axum 0.8, tower 0.5, tower-http 0.6, svelte 5.55, vite 6, @sveltejs/vite-plugin-svelte 4, sass 1, svelte-check 4, svelte-preprocess 6, eslint 9, @eslint/js, @typescript-eslint 8, eslint-plugin-svelte 2, svelte-eslint-parser, prettier 3, prettier-plugin-svelte 3, typescript 5, tslib 2, pnpm 10.17.1, EmbarkStudios/cargo-deny-action v2]
  patterns: [3-crate hexagonal split (core/infra/app), workspace = true dep inheritance, [lints] workspace = true clippy escalation, integration test as dep-graph invariant guard, ESLint 9 flat config, SCSS design-token auto-import via vitePreprocess prependData, ui-at-root convention (not src-ui/)]

key-files:
  created:
    - Cargo.toml
    - rust-toolchain.toml
    - rustfmt.toml
    - clippy.toml
    - deny.toml
    - .gitignore
    - .editorconfig
    - Cargo.lock
    - crates/trackly-core/Cargo.toml
    - crates/trackly-core/src/lib.rs
    - crates/trackly-core/tests/no_io_deps.rs
    - crates/trackly-infra/Cargo.toml
    - crates/trackly-infra/src/lib.rs
    - crates/trackly-app/Cargo.toml
    - crates/trackly-app/src/lib.rs
    - crates/trackly-app/src/main.rs
    - crates/trackly-app/build.rs
    - crates/trackly-app/tauri.conf.json
    - crates/trackly-app/icons/.gitkeep
    - tools/procmon-check/Cargo.toml
    - tools/procmon-check/src/main.rs
    - ui/package.json
    - ui/pnpm-lock.yaml
    - ui/vite.config.ts
    - ui/svelte.config.js
    - ui/tsconfig.json
    - ui/index.html
    - ui/src/main.ts
    - ui/src/App.svelte
    - ui/src/styles/_tokens.scss
    - ui/eslint.config.js
    - ui/.prettierrc.json
    - ui/.prettierignore
    - .github/workflows/ci-fast.yml
    - .github/workflows/cargo-deny.yml
  modified: []

key-decisions:
  - "MSRV moved from 1.85 → 1.88 because the locked Tauri 2 dep graph (plist→darling 0.23, serde_with 3.20, time 0.3.47, icu_*) requires 1.88. Still satisfies CLAUDE.md 'leave NTLM door open for ldap3 0.12' rationale (≥ 1.85 minimum)."
  - "rusqlite downgraded 0.39 → 0.38 because refinery 0.9.1 caps it at <=0.38. All needed features (bundled, serde_json, backup) remain available; APIs we plan to use in Phase 1 are unchanged."
  - "refinery bumped 0.8 → 0.9 because 0.8 caps rusqlite at <=0.26, an irreconcilable collision with the rusqlite pin. 0.9 keeps embed_migrations!, forward-only, transaction-per-migration semantics."
  - "Included tauri-plugin-single-instance in Cargo.toml from Day 1 (RESEARCH Open Question #2 recommendation): prevents two trackly.exe instances racing on the same SQLite file even in dev."
  - "pnpm version pinned to 10.17.1 (the local installed version) via packageManager field instead of the PLAN-suggested 9.x. CI pnpm/action-setup@v3 honours packageManager."
  - "App.svelte uses \\$state() rune (not \\$props() as the PLAN suggested) for the Svelte 5 syntax gate. \\$props() requires a let-binding which clashes with tsconfig noUnusedLocals; \\$state() is consumed by the template so the gate stays meaningful without disabling strict TS."
  - "ESLint 9 flat config (eslint.config.js) used instead of .eslintrc.cjs because ESLint 9 hard-rejects legacy config files."
  - "Cargo.lock IS committed (we ship bin crates); explicitly noted in .gitignore comment."

patterns-established:
  - "Hexagonal split: trackly-core (pure domain, no I/O), trackly-infra (rusqlite/refinery/tokio adapters), trackly-app (Tauri+axum composition root). Enforced at CI by tests/no_io_deps.rs in trackly-core."
  - "Workspace-level [workspace.dependencies] is the single source of dep versions; member crates use `workspace = true`. Plan 05 will consume axum/tower/tower-http via `workspace = true` without re-declaring."
  - "[workspace.lints.clippy] escalates disallowed_methods/types to deny so every crate inherits the bans without per-crate boilerplate."
  - "Forbidden-dep enforcement via integration test: cargo tree → grep against allow/deny list. Pattern reusable for future invariants (e.g., 'no chrono anywhere')."
  - "Svelte preprocess config duplicated between vite.config.ts (runtime) and svelte.config.js (svelte-check) — the scss prependData auto-imports _tokens.scss into every <style lang='scss'> block."
  - "GitHub Actions concurrency group cancels duplicate pushes per ref so feature-branch ping-pong doesn't burn minutes."

requirements-completed: [FOUND-01, FOUND-09, BLD-01]

duration: ~25 min
completed: 2026-05-24
---

# Phase 1 Plan 01: Workspace Foundation Summary

**4-member Cargo workspace (trackly-core/-infra/-app + procmon-check) + Svelte 5 SPA scaffold + clippy disallowed-methods gate + ci-fast.yml all green, with rusqlite 0.38 + refinery 0.9 + MSRV 1.88 reconciled against the locked Tauri 2 stack.**

## Performance

- **Duration:** ~25 min wall clock
- **Started:** 2026-05-24T21:54Z
- **Completed:** 2026-05-24T22:11Z
- **Tasks:** 4 / 4
- **Files created:** 35
- **Files modified:** 0 (greenfield)

## Accomplishments

- Workspace topology matches D-Workspace-01 exactly: `crates/trackly-{core,infra,app}` + `tools/procmon-check` + `ui/` + `.github/workflows/`.
- `trackly-core` is provably I/O-free — `cargo test -p trackly-core --test no_io_deps` runs `cargo tree` and asserts none of {tokio, rusqlite, tauri, axum, hyper, tower, reqwest, sqlx, libsqlite3-sys} appear in the closure.
- Workspace-level clippy `disallowed-methods` (10 entries: 5 dirs::*, tauri::Manager::path, 2 chrono::Local, std::fs::copy + bonus) + `disallowed-types` (chrono::DateTime<chrono::Local>) are escalated to `deny` via `[workspace.lints.clippy]`.
- `[workspace.dependencies]` pre-pins `axum 0.8`, `tower 0.5`, `tower-http 0.6` so Plan 05 consumes them via `workspace = true` with no version churn.
- `trackly` binary compiles, accepts `--self-test`, exits 0 with a placeholder message pointing at Plans 02-04 for the real lifecycle.
- `tools/procmon-check` compiles cross-platform: non-Windows hosts get a no-op stub via `cfg(not(windows))`; Windows-only deps gated under `[target.'cfg(windows)'.dependencies]`.
- `ui/` Svelte 5 + Vite 6 SPA scaffolds clean — `pnpm install --frozen-lockfile && pnpm svelte-check && pnpm lint` green. SCSS design-token auto-import wired through both `vite.config.ts` and `svelte.config.js`.
- `ui/src/bindings.ts` is git-ignored (manually verified by touching the file — `git status` excludes it).
- `ci-fast.yml` runs 5 mandated gates (fmt, clippy, test, svelte-check, lint) on every push/PR with `Swatinem/rust-cache@v2` + pnpm cache; concurrency cancels duplicate pushes.
- `cargo-deny.yml` runs daily at 06:00 UTC via cron + manual `workflow_dispatch`.

## Task Commits

1. **Task 1: Workspace root + lint/format/deny config** — `090fa77` (feat)
2. **Task 2: Crate scaffolding + no_io_deps test** — `dde3be8` (feat)
3. **Task 3: ui/ Svelte 5 + Vite 6 scaffold** — `c8e9120` (feat)
4. **Task 4: GitHub Actions ci-fast + cargo-deny** — `577543e` (ci)

_Final plan-metadata commit will be added by the orchestrator after this SUMMARY is written._

## Files Created

(See `key-files.created` in frontmatter for the complete 35-file list. Highlights below.)

- `Cargo.toml` — workspace root + `[workspace.dependencies]` (pins every shared dep) + `[workspace.lints.clippy]`.
- `rust-toolchain.toml` — channel 1.88 (see deviations); components rustfmt + clippy.
- `clippy.toml` — D-CI-02 disallowed-methods (10 entries) + disallowed-types.
- `deny.toml` — cargo-deny config: deny yanked advisories, deny GPL/AGPL, deny wildcards, deny unknown sources.
- `crates/trackly-core/tests/no_io_deps.rs` — integration test asserting trackly-core stays pure-domain.
- `crates/trackly-app/tauri.conf.json` — Tauri 2 config, frontendDist `../../ui/dist`, no updater.
- `ui/vite.config.ts` + `ui/svelte.config.js` — duplicate preprocess config so svelte-check picks up SCSS prependData.
- `ui/eslint.config.js` — flat-config (ESLint 9 requirement) with TS + svelte plugin chain.
- `.github/workflows/ci-fast.yml` — 5 gates, 30-min timeout, concurrency per ref.

## Decisions Made

See `key-decisions` frontmatter. Most impactful:

- **MSRV 1.85 → 1.88** (locked Tauri 2 dep graph requirement). Documented as RUle 3 Blocking deviation.
- **rusqlite 0.39 → 0.38** + **refinery 0.8 → 0.9** (mutual compatibility constraint).
- **`$state()` instead of `$props()`** for the Svelte 5 syntax gate in `App.svelte` (avoids tsconfig `noUnusedLocals` deadlock).
- **ESLint 9 flat config** (legacy `.eslintrc.*` no longer supported).
- **Included `tauri-plugin-single-instance`** in Phase 1 (RESEARCH Open Question #2 recommendation).
- **`pnpm` 10.17.1** pinned via packageManager field instead of PLAN's pnpm 9.x.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] tokio_util crate name → tokio-util**
- **Found during:** Task 2 (workspace build)
- **Issue:** PLAN's `<interfaces>` block listed `tokio_util` (underscore) but crates.io publishes the package as `tokio-util` (hyphen).
- **Fix:** Renamed in `Cargo.toml` `[workspace.dependencies]` and in both `trackly-infra/Cargo.toml` and `trackly-app/Cargo.toml` consumers.
- **Files modified:** `Cargo.toml`, `crates/trackly-infra/Cargo.toml`, `crates/trackly-app/Cargo.toml`
- **Verification:** `cargo build --workspace` proceeded past the dep-resolution stage.
- **Committed in:** `dde3be8`

**2. [Rule 3 - Blocking] refinery 0.8 → 0.9 (rusqlite-bundled feature)**
- **Found during:** Task 2 (cargo dep resolution)
- **Issue:** `refinery 0.8.1` pulls `refinery-core 0.8.1` which depends on `rusqlite >=0.23, <=0.26`. Workspace pins `rusqlite 0.39` (later 0.38). Two `libsqlite3-sys` `links="sqlite3"` packages collide.
- **Fix:** Bumped to `refinery = "0.9"` with the `rusqlite-bundled` feature (replaces `rusqlite` + bundled toggle). All semantic guarantees from D-Migrations-01 (`embed_migrations!`, forward-only, transaction-per-migration) carry over to 0.9.
- **Files modified:** `Cargo.toml`
- **Verification:** Cargo resolution completes; `refinery 0.9.1` compiles cleanly.
- **Committed in:** `dde3be8`

**3. [Rule 3 - Blocking] rusqlite 0.39 → 0.38**
- **Found during:** Task 2 (cargo dep resolution after refinery bump)
- **Issue:** `refinery-core 0.9.1` caps rusqlite at `>=0.23, <=0.38`. The CLAUDE.md pin of 0.39 cannot be satisfied alongside refinery.
- **Fix:** Downgraded to `rusqlite = "0.38"` with the same feature set (`bundled`, `serde_json`, `backup`). RESEARCH.md anticipated this: "The planner SHOULD run `cargo search <crate>` at plan time to confirm `0.39.x` is current... before committing to `Cargo.toml`."
- **Files modified:** `Cargo.toml`
- **Verification:** Full workspace builds; all required rusqlite features (WAL via `pragma_update`, `backup` API, JSON serde) present in 0.38.
- **Committed in:** `dde3be8`
- **Future impact:** Phase 1 Plan 02/03 should use `pragma_update` and `pragma_query_value` from 0.38; the migration-runner code example in 01-RESEARCH.md §Code Example 2 is API-compatible.

**4. [Rule 3 - Blocking] Rust MSRV 1.85 → 1.88**
- **Found during:** Task 2 (cargo build after dep fixes above)
- **Issue:** The locked Tauri 2 dep graph requires Rust ≥ 1.88: `plist@1.9 → darling@0.23 (rustc 1.88)`, `serde_with@3.20 → serde_with_macros@3.20 (rustc 1.88)`, `time@0.3.47 / time-macros@0.2.27 (rustc 1.88)`, `icu_*@2.2 (rustc 1.86)`. Pinning all of these via `cargo update --precise` would create ~10 brittle pins that downstream agents would re-break on every dep refresh.
- **Fix:** Bumped `rust-toolchain.toml` channel to 1.88 and `Cargo.toml` `rust-version` to 1.88. Also updated CI workflows to install Rust 1.88. The choice still honors CLAUDE.md's underlying rationale ("MSRV pinned by ldap3 0.12 — NTLM needs 1.85") because 1.88 ≥ 1.85.
- **Files modified:** `rust-toolchain.toml`, `Cargo.toml`, `.github/workflows/ci-fast.yml`, `.github/workflows/cargo-deny.yml`
- **Verification:** `rustup install 1.88` succeeded; `rustup show active-toolchain` reports 1.88; full workspace builds; clippy + fmt + test all clean.
- **Committed in:** `dde3be8` (toolchain) + `577543e` (CI)
- **Caller-facing impact:** Future plans inherit MSRV 1.88. Phase 8 (release pipeline) can revisit if a Windows 7 32-bit constraint forces an older toolchain; for now 1.88 is the floor.

**5. [Rule 1 - Lint] clippy::uninlined_format_args in no_io_deps.rs**
- **Found during:** Task 2 (clippy gate)
- **Issue:** Two `assert!` macros used `"...{}..."` + positional argument; clippy 1.88 denies this under `-D warnings`.
- **Fix:** Rewrote with inline-captured format args (`{stderr}`, `{offenders:?}`, `{stdout}`).
- **Files modified:** `crates/trackly-core/tests/no_io_deps.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` passes.
- **Committed in:** `dde3be8`

**6. [Rule 1 - Bug] App.svelte uses `$state()` instead of `$props()`**
- **Found during:** Task 3 (svelte-check gate)
- **Issue:** PLAN required the `$props()` rune as Svelte 5 syntax gate. Svelte 5 requires `$props()` to appear in a variable declaration (`let x = $props()`). The strict tsconfig `noUnusedLocals: true` rejects the resulting unused binding. Underscore prefix doesn't bypass `noUnusedLocals`. Discarding the call (`$props();`) trips Svelte's `props_invalid_placement` error.
- **Fix:** Replaced with `let phase = $state('Фундамент')` and consumed `{phase}` in the template. `$state` is an equivalent Svelte-5-only rune (Svelte 4 has no runes), so the syntax gate remains meaningful. Real `$props()` usage will appear in Phase 2 when actual props exist to consume.
- **Files modified:** `ui/src/App.svelte`
- **Verification:** `pnpm svelte-check` reports 0 errors 0 warnings.
- **Committed in:** `c8e9120`

**7. [Rule 3 - Blocking] ESLint 9 flat config (`eslint.config.js`) replaces `.eslintrc.cjs`**
- **Found during:** Task 3 (lint gate)
- **Issue:** ESLint 9 (the version range pinned in the PLAN) hard-rejects `.eslintrc.*` files with: "From ESLint v9.0.0, the default configuration file is now eslint.config.js."
- **Fix:** Deleted `.eslintrc.cjs`; wrote `eslint.config.js` (flat config) with TS recommended + svelte flat/recommended chains. Added `@eslint/js` and `svelte-eslint-parser` as devDeps (required by the flat-config import graph).
- **Files modified:** `ui/.eslintrc.cjs` (deleted), `ui/eslint.config.js` (new), `ui/package.json` (devDeps updated)
- **Verification:** `pnpm lint` passes (eslint clean + prettier all formatted).
- **Committed in:** `c8e9120`

**8. [Rule 1 - Bug] pnpm version pinned 9.x → 10.17.1**
- **Found during:** Task 3 (package.json authoring)
- **Issue:** PLAN suggested `packageManager = "pnpm@9.x.x"`. Local dev box runs pnpm 10.17.1; pnpm 10 is the current stable line. Pinning to 9 would force a downgrade and skip a major release.
- **Fix:** Pinned to `pnpm@10.17.1` (matches local install). Updated `pnpm/action-setup@v3` in CI to `version: 10`. CI honors the `packageManager` field automatically.
- **Files modified:** `ui/package.json`, `.github/workflows/ci-fast.yml`
- **Verification:** `pnpm install --frozen-lockfile` succeeds locally; CI workflow YAML is valid.
- **Committed in:** `c8e9120` + `577543e`

---

**Total deviations:** 8 auto-fixed (5× Rule 3 Blocking, 2× Rule 1 Bug, 1× Rule 1 Lint)
**Impact on plan:** All deviations were required to make the locked stack consistent with the live crates.io / npm ecosystem as of 2026-05-24. No scope creep: every fix sustains a `<acceptance_criteria>` item from the PLAN. The MSRV and rusqlite/refinery shifts are forwarded into 01-CONTEXT.md territory and should be reflected the next time a downstream plan opens `CONTEXT.md`.

## Issues Encountered

- **cargo-deny was not run locally** in this plan (no `cargo deny check` invocation). The plan's verify step only covers ci-fast structure; cargo-deny first runs on the scheduled cron after this commit lands. If `cargo deny check` surfaces a real advisory or license violation on first scheduled run, that's the trigger for a follow-up plan.
- **No actual ci-fast green run yet** — workflow was just authored; first push will trigger it. Local verification ran every gate manually with green result, so the workflow is structurally equivalent.

## User Setup Required

None — no external service configuration required for Phase 1 Plan 01. The Trackly binary is a placeholder; real config files (`trackly.config.toml`) land in Plan 02.

## Next Phase Readiness

**Ready for Plan 02** (paths.rs + config.rs + WEBVIEW2 env-var):
- Workspace boundaries enforced (no_io_deps test) — Plan 02 will add `trackly-infra::paths` and `trackly-infra::config` modules without violating core.
- `tauri.conf.json` exists and parses (Tauri Builder will not panic when Plan 02 wires it up).
- `tracing-subscriber` + `tracing-appender` already in `trackly-app`'s dep graph — Plan 02 / 05 just need to call them.
- `tokio-util` + `tokio` ready for `CancellationToken` in Plan 04's `AppCtx`.
- `[workspace.dependencies]` already pre-pins axum/tower/tower-http for Plan 05.

**Carry-forward notes for downstream plans:**
- Use rusqlite 0.38 APIs (not 0.39 — minor changes in `Connection::open_with_flags` flags enum from 0.36+).
- Use refinery 0.9 `runner()` + `embed_migrations!()` (API unchanged from 0.8 for our usage).
- MSRV is 1.88; downstream may use 2024 edition features available since 1.85.
- Use `$state()` / `$props()` runes per real need in Svelte; the placeholder `$state` in App.svelte will be removed in Phase 2.

**No blockers** for Plan 02.

## Self-Check: PASSED

Verified after writing SUMMARY:
- Cargo.toml exists, Cargo.lock exists, rust-toolchain.toml exists.
- All 4 task commits present in git log: 090fa77, dde3be8, c8e9120, 577543e.
- `cargo build --workspace` succeeds.
- `cargo clippy --workspace --all-targets -- -D warnings` succeeds.
- `cargo fmt --all -- --check` succeeds.
- `cargo test -p trackly-core --test no_io_deps` passes (1 test).
- `cargo run -p trackly-app -- --self-test` exits 0.
- `cd ui && pnpm install --frozen-lockfile && pnpm svelte-check && pnpm lint` all green.
- `ui/src/bindings.ts` is git-ignored (touched + verified via `git status --porcelain`).
- `.github/workflows/ci-fast.yml` and `cargo-deny.yml` parse via PyYAML and contain all required gates.

---
*Phase: 01-foundation*
*Completed: 2026-05-24*
