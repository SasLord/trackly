---
phase: 03-pdf
plan: 01
subsystem: pdf
tags: [krilla, minijinja, pdf-extract, dejavu-sans, docspec, msrv-1.92, structural-spike]

requires:
  - phase: 01-foundation
    provides: trackly-core AppError shape (Validation/Internal), workspace toolchain pin, crate split (core/infra/app)
  - phase: 02-ui
    provides: trackly-app::csv as the precedent for self-contained subsystem modules, snake_case JSON conventions, integration-test layout under tests/

provides:
  - trackly-app::pdf::PdfRenderer public API (new() + render_docspec()) — ready to be wired into AppCtx in plan 04
  - trackly-app::pdf::docspec::{DocSpec, Section, HeaderBlock, KvRow, TextStyle} typed AST — frozen contract that downstream templates target
  - trackly-app::pdf::minijinja_env::{build_safe_env, render_with_timeout} — safe-mode template engine + 5s timeout + 100k fuel cap
  - trackly-app::pdf::fonts::{DEJAVU_SANS_REGULAR, DEJAVU_SANS_BOLD} — embedded Cyrillic-safe fonts via include_bytes!
  - Canonical PDF fixture (act_42.json + act_42.sha256) — known-good byte-stream for cross-OS CI determinism gating
  - PDF determinism guard pattern — pinned Metadata + xmp_metadata=false + regex post-process; documented for plan 04 reuse
  - MSRV bump 1.88 → 1.92 (workspace + rust-toolchain.toml); Win7 32-bit deferred to v2

affects:
  - 03-02 (Acts CRUD) — will compose ActService over the same writer/reader pool patterns as Phase 2 services
  - 03-03 (Returns) — will rely on AppError::Validation field=template for template-rendering failures
  - 03-04 (Templates + PDF endpoints) — will plug DocSpec output into real MiniJinja templates loaded from document_templates table
  - 03-05 (Search + DEV-14 polish) — re-uses DEV-14 acceptance document via same PDF pipeline
  - Phase 6 (Заявки печать) — REQ-04 print path reuses the same DocSpec → krilla pipeline
  - Phase 7 (Отчёты) — report PDF output uses the same renderer

tech-stack:
  added:
    - krilla = "=0.7.0" (PDF rendering with OpenType subsetting, default-engine per CLAUDE.md)
    - minijinja = "^2.20" (no-default-features + json + fuel + serde) — safe template engine
    - pdf-extract = "^0.10" (dev-dep) — text extraction for Cyrillic glyph regression test
    - regex = "1.12.3" — used by renderer's determinism post-process
    - sha2 (already dev-dep from Phase 1) — used by determinism hash test
  patterns:
    - Three-stage PDF pipeline (MiniJinja safe-mode → serde<DocSpec> → krilla render) — D-PDF-Render-Path-01
    - Pinned Metadata + xmp_metadata=false + regex post-process for byte-stream determinism — Pitfall 4 mitigation
    - Embedded binary assets via include_bytes! from crates/trackly-app/assets/fonts/
    - tokio::time::timeout(5s) + spawn_blocking + JoinError mapping for CPU-bound async wrappers
    - Tagged enum (serde tag="type", rename_all="snake_case") for typed IR with frontend-friendly discriminator

key-files:
  created:
    - crates/trackly-app/src/pdf/mod.rs (subsystem re-export + module docs)
    - crates/trackly-app/src/pdf/fonts.rs (DEJAVU_SANS_REGULAR/BOLD constants)
    - crates/trackly-app/src/pdf/docspec.rs (DocSpec + 6 Section variants + tests)
    - crates/trackly-app/src/pdf/minijinja_env.rs (build_safe_env + render_with_timeout + 4 tests)
    - crates/trackly-app/src/pdf/renderer.rs (PdfRenderer + determinism guard + 4 tests)
    - crates/trackly-app/assets/fonts/DejaVuSans.ttf (757 KB)
    - crates/trackly-app/assets/fonts/DejaVuSans-Bold.ttf (705 KB)
    - crates/trackly-app/assets/fonts/README.md (license + version + selection rationale)
    - crates/trackly-app/tests/pdf_determinism.rs (2 acceptance tests)
    - crates/trackly-app/tests/pdf_text_extract.rs (1 Cyrillic glyph test)
    - crates/trackly-app/tests/fixtures/act_42.json (canonical DocSpec)
    - crates/trackly-app/tests/fixtures/act_42.sha256 (pinned hash)
  modified:
    - rust-toolchain.toml (1.88 → 1.92.0)
    - Cargo.toml (workspace rust-version 1.88 → 1.92)
    - Cargo.lock (new transitive deps from krilla + minijinja + pdf-extract)
    - crates/trackly-app/Cargo.toml (+krilla, +minijinja, +regex, +pdf-extract dev)
    - crates/trackly-app/src/lib.rs (+pub mod pdf;)
    - .planning/phases/01-foundation/deferred-items.md (Windows 7 32-bit moved to "Closed / promoted")

key-decisions:
  - "krilla determinism path: pin Metadata { creation_date=2026-01-01, producer='Trackly Phase 3' } + SerializeSettings { xmp_metadata: false } + regex safety net over /CreationDate, /ModDate, /Producer in raw bytes"
  - "Section enum tag chosen as serde(tag='type', rename_all='snake_case') — matches frontend convention and PATTERNS.md §E"
  - "DocSpec lives in crates/trackly-app/src/pdf/docspec.rs (NOT crates/trackly-app/src/dto/) per plan 01 task 2 read_first guidance — avoids duplicate type for the same content tree"
  - "MiniJinja features explicitly include 'fuel' (required for set_fuel) and 'serde' (required for tmpl.render(serde_json::Value)) — these were absent from the original plan but are required by minijinja 2.20 to expose those APIs"
  - "Win7 32-bit definitively closed in v1 — MSRV 1.92 closes the toolchain door, recorded in deferred-items.md under Closed/promoted"
  - "Determinism hash 88df7f9d69c5db10a4685f0aa5d390caec90b045067e35cc1caba33efdd15d1f captured on macOS aarch64 1.92.0 + krilla 0.7.0 + DejaVu Sans 2.37 — same-machine stability proven; cross-OS validated when CI matrix runs the same test"

patterns-established:
  - "Pattern PDF-1: three-stage rendering pipeline (template → serde → krilla) keeps each stage testable in isolation and prevents arbitrary PDF-operator injection from user templates"
  - "Pattern PDF-2: byte-stream determinism via Metadata pin + XMP disable + regex post-process — defends against future krilla version drift"
  - "Pattern PDF-3: embedded binary assets via include_bytes! from per-crate assets/ subdir; canonical place for similar future embeds (e.g. logo defaults, signature templates)"
  - "Pattern PDF-4: integration-test fixtures live under crates/trackly-app/tests/fixtures/ with a canonical naming convention act_42.* — extensible to cartridge/printer fixtures in later phases"

requirements-completed: []  # Plan 01 has no direct user-facing requirements; it's the D-PDF-Engine-01 structural spike

duration: 25min
completed: 2026-05-29
---

# Phase 3 Plan 01: PDF Foundation Summary

**krilla 0.7 + DejaVu Sans embedded + MiniJinja safe-mode + DocSpec typed IR + deterministic byte-stream guard pinning the canonical Cyrillic fixture at sha256 88df7f9d…**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-05-29T18:36:00Z
- **Completed:** 2026-05-29T19:00:00Z
- **Tasks:** 3
- **Files created/modified:** 19

## Accomplishments

- Toolchain bumped to Rust 1.92.0 (krilla 0.7 prerequisite); Win7 32-bit definitively closed in v1
- Three new pinned dependencies (krilla =0.7.0, minijinja ^2.20, pdf-extract ^0.10) + regex 1.x + dev-dep sha2 wired into trackly-app; workspace builds clean
- DejaVu Sans Regular + Bold (DejaVu 2.37, public-domain-derived) embedded directly into the trackly-app binary; portable bundle does not have to ship .ttf files
- Full PDF subsystem (`crates/trackly-app/src/pdf/`) in place: docspec.rs + fonts.rs + minijinja_env.rs + renderer.rs + mod.rs — 12 unit tests + 3 integration tests, all green
- Determinism proven on macOS aarch64 / 1.92.0: two consecutive renders of the canonical fixture produce byte-identical output, sha256 pinned at `88df7f9d69c5db10a4685f0aa5d390caec90b045067e35cc1caba33efdd15d1f`
- Cyrillic glyphs survive end-to-end: `pdf-extract` recovers «Сидоров-Петроградский», «№42», and «(ё)» from the rendered fixture

## Task Commits

Each task was committed atomically:

1. **Task 1: MSRV bump + dependency add + fonts** — `1593c0e` (chore)
2. **Task 2: PDF subsystem skeleton (fonts/docspec/minijinja_env/renderer)** — `71dc25f` (feat)
3. **Task 3: Integration fixture (act_42.json + sha256) + Cyrillic extract test** — `83f3db8` (test)

## Public API Signatures

```rust
// crates/trackly-app/src/pdf/renderer.rs
pub struct PdfRenderer { /* ... */ }
impl PdfRenderer {
    pub fn new() -> Self;
    pub fn render_docspec(&self, spec: &DocSpec) -> Result<Vec<u8>, AppError>;
}

// crates/trackly-app/src/pdf/minijinja_env.rs
pub fn build_safe_env() -> minijinja::Environment<'static>;
pub async fn render_with_timeout(
    env: &minijinja::Environment<'static>,
    name: &str,
    template_src: &str,
    ctx: serde_json::Value,
) -> Result<String, AppError>;
```

## DocSpec Variants (serde tag = "type", rename_all = "snake_case")

| Variant         | JSON discriminator   | Required fields                                 | Optional         |
|-----------------|----------------------|-------------------------------------------------|------------------|
| Paragraph       | `paragraph`          | `text: String`                                  | `style` (default Regular) |
| Heading         | `heading`            | `level: u8`, `text: String`                     | —                |
| KeyValueTable   | `key_value_table`    | `rows: Vec<KvRow {key, value}>`                 | —                |
| ItemsTable      | `items_table`        | `columns: Vec<String>`, `rows: Vec<Vec<String>>`| —                |
| Signature       | `signature`          | `left_label: String`, `right_label: String`     | `spacer_pt` (default 24.0) |
| Spacer          | `spacer`             | `height_pt: f32`                                | —                |

`HeaderBlock`: `org_name`, `org_inn`, `org_kpp`, `org_address`, `act_label`, `date_label` (all `String`); `logo_path: Option<String>`.

`TextStyle`: `regular | bold`.

## Pinned Fixture Hash

`crates/trackly-app/tests/fixtures/act_42.sha256`:

```
88df7f9d69c5db10a4685f0aa5d390caec90b045067e35cc1caba33efdd15d1f
```

This is the SHA256 of the rendered PDF byte-stream when the canonical fixture
`act_42.json` is fed through `PdfRenderer::render_docspec`. If a downstream
plan changes the renderer's geometry, the determinism guard, or the embedded
font version, this hash will need to be updated as part of that change.

## Determinism Notes (Pitfall 4 status)

- **krilla 0.7 `Document::new()`** does NOT inject a creation date by default; the
  /Info dict's `/CreationDate` only appears if `Metadata::creation_date` is set.
- **xmp_metadata** is `true` by default in `SerializeSettings`; we set it to
  `false`. Without this, the XMP packet would carry an embedded instance-id
  hash that could shift between krilla minor versions.
- **Font subset prefix** (e.g. `ABCDEF+DejaVuSans`) is currently stable across
  same-machine consecutive renders on macOS aarch64; no normalization required.
- **`/Producer` string** is pinned via `Metadata::producer("Trackly Phase 3")`.
- **Regex safety net** in `normalize_pdf_for_determinism` covers `/CreationDate`,
  `/ModDate`, and `/Producer` in case future krilla versions reintroduce
  timestamps in dictionary positions we don't currently see.

These pins are likely sufficient for cross-OS determinism, but the formal
verification will come when Phase 5's CI matrix runs the same fixture on linux
and windows runners. If that test reveals platform-specific drift, the
mitigation order is (in increasing severity): (1) extend the regex set, (2)
override the document_id to a fixed UUID via `Metadata::document_id`, (3)
revisit Section-level layout for floating-point reproducibility.

## Files Created/Modified

- `rust-toolchain.toml` — toolchain 1.88 → 1.92.0
- `Cargo.toml` — workspace rust-version 1.88 → 1.92
- `Cargo.lock` — krilla + minijinja + pdf-extract + transitive crates
- `crates/trackly-app/Cargo.toml` — krilla =0.7.0, minijinja ^2.20 (+json+fuel+serde), regex 1.x, pdf-extract ^0.10 dev-dep
- `crates/trackly-app/src/lib.rs` — added `pub mod pdf;`
- `crates/trackly-app/src/pdf/mod.rs` — subsystem module doc + re-exports
- `crates/trackly-app/src/pdf/fonts.rs` — DEJAVU_SANS_REGULAR/BOLD via include_bytes!
- `crates/trackly-app/src/pdf/docspec.rs` — DocSpec + 6 Section variants + 4 unit tests
- `crates/trackly-app/src/pdf/minijinja_env.rs` — build_safe_env + render_with_timeout + 4 unit tests
- `crates/trackly-app/src/pdf/renderer.rs` — PdfRenderer + determinism guard + 4 unit tests
- `crates/trackly-app/assets/fonts/DejaVuSans.ttf` — 757 KB
- `crates/trackly-app/assets/fonts/DejaVuSans-Bold.ttf` — 705 KB
- `crates/trackly-app/assets/fonts/README.md` — license + provenance
- `crates/trackly-app/tests/pdf_determinism.rs` — 2 acceptance tests
- `crates/trackly-app/tests/pdf_text_extract.rs` — Cyrillic glyph acceptance test
- `crates/trackly-app/tests/fixtures/act_42.json` — canonical DocSpec
- `crates/trackly-app/tests/fixtures/act_42.sha256` — pinned hash
- `.planning/phases/01-foundation/deferred-items.md` — Win7 32-bit moved to Closed / promoted

## Decisions Made

- **DocSpec location:** placed in `crates/trackly-app/src/pdf/docspec.rs` rather than
  duplicating in `crates/trackly-app/src/dto/`. Plan task 2 read_first explicitly
  ruled out the dto/ duplication; one canonical place keeps serde/typeck simple
  and avoids drift if the schema evolves.
- **MiniJinja features:** the original task spec said
  `--no-default-features --features json`, but `set_fuel(Some(100_000))` requires
  the `fuel` feature, and `tmpl.render(serde_json::Value)` requires the `serde`
  feature in minijinja 2.20. Both were added as a Rule 3 (blocking) fix so the
  acceptance tests `env_render_timeout_returns_validation` and
  `env_renders_simple_template` can actually compile.
- **krilla API path corrections:** the plan referenced
  `krilla::interchange::metadata::Metadata`, `krilla::serialize::SerializeSettings`,
  but krilla 0.7 makes `interchange` and `serialize` private modules. The correct
  imports are `krilla::metadata::{Metadata, DateTime}` and
  `krilla::SerializeSettings` (re-exported at crate root). Applied as Rule 3
  (blocking import-path fix).
- **Hash captured on dev machine:** the canonical hash
  `88df7f9d69c5db10a4685f0aa5d390caec90b045067e35cc1caba33efdd15d1f` was
  captured on macOS aarch64 / Rust 1.92.0. Cross-OS validation deferred to when
  CI matrix runs Phase 3 tests (Phase 5+).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `fuel` and `serde` MiniJinja features**

- **Found during:** Task 2 (PDF subsystem skeleton)
- **Issue:** Plan specified `minijinja@^2.20 --no-default-features --features json`,
  but `Environment::set_fuel` requires the `fuel` cargo feature and
  `Template::render(serde_json::Value)` requires the `serde` feature in
  minijinja 2.20. Build failed with `no method named set_fuel`.
- **Fix:** Updated `crates/trackly-app/Cargo.toml` to
  `features = ["json", "fuel", "serde"]`. Both are necessary for the safe-mode
  contract.
- **Files modified:** `crates/trackly-app/Cargo.toml`
- **Verification:** `cargo build -p trackly-app` succeeds; `set_fuel` invocation
  in `pdf/minijinja_env.rs` compiles; `env_render_timeout_returns_validation`
  test asserts the fuel cap path.
- **Committed in:** `71dc25f` (Task 2 commit)

**2. [Rule 3 - Blocking] Corrected krilla 0.7 module paths**

- **Found during:** Task 2 (PDF subsystem skeleton)
- **Issue:** Plan referenced
  `krilla::interchange::metadata::{Metadata, DateTime}` and
  `krilla::serialize::SerializeSettings`, but krilla 0.7 declares both
  `interchange` and `serialize` as private modules at the crate root, with
  the actual exports re-routed via `pub use interchange::*` and
  `pub use serialize::SerializeSettings`.
- **Fix:** Imports rewritten to `krilla::metadata::{Metadata, DateTime}` and
  `krilla::SerializeSettings`.
- **Files modified:** `crates/trackly-app/src/pdf/renderer.rs`
- **Verification:** `cargo build` clean; PDF renders correctly with pinned
  metadata; determinism tests pass.
- **Committed in:** `71dc25f` (Task 2 commit)

**3. [Rule 1 - Bug] Removed unused `static mut CACHED` leftover**

- **Found during:** Task 2 (PDF subsystem skeleton)
- **Issue:** Initial renderer.rs draft kept a `static mut CACHED` placeholder
  that triggered the Rust 2024 `static_mut_refs` warning, which `-D warnings`
  promotes to error.
- **Fix:** Removed the `static mut` and the `let _ = unsafe { &CACHED };`
  guard; regexes are now recompiled per call (cost negligible vs PDF write).
- **Files modified:** `crates/trackly-app/src/pdf/renderer.rs`
- **Verification:** `cargo clippy -p trackly-app --all-targets -- -D warnings`
  passes.
- **Committed in:** `71dc25f` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (2 blocking, 1 bug)
**Impact on plan:** All three were necessary for the acceptance tests to
compile and pass. No scope creep; all corrections are within the plan's
stated subsystem.

## Issues Encountered

None beyond the three deviations above.

## User Setup Required

None — all changes are in-tree.

## Next Phase Readiness

- **PDF foundation ready for plan 03-02 (Acts CRUD).** `PdfRenderer::new()` will
  become an `Arc<PdfRenderer>` field on `AppCtx` in plan 03-04.
- **Determinism hash captured on macOS aarch64.** Cross-OS validation deferred
  to Phase 5 CI matrix; if linux/windows produce a different hash, the
  determinism guard's regex set may need expansion or `Metadata::document_id`
  may need to be pinned to a fixed UUID.
- **Phase 3 D-PDF-Engine-01 structural spike PASSED.** krilla 0.7 produces
  byte-stable, Cyrillic-correct output for the canonical fixture. The
  fallback Typst-as-lib spike is NOT triggered; plans 03-02..03-05 proceed
  on the krilla 0.7 path.

## Self-Check: PASSED

All 11 created files verified present on disk; all 3 task commits found in
`git log`. No missing artifacts.

---
*Phase: 03-pdf*
*Completed: 2026-05-29*
