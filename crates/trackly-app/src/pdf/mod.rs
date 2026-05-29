//! PDF rendering subsystem (Phase 3).
//!
//! 3-stage pipeline (D-PDF-Render-Path-01):
//! 1. `minijinja_env::render_with_timeout(template_src, ctx) -> String` (JSON)
//! 2. `serde_json::from_str::<DocSpec>(rendered)` (validation)
//! 3. `renderer::PdfRenderer::render_docspec(&spec) -> Vec<u8>`
//!
//! The MiniJinja stage runs in safe-mode (`UndefinedBehavior::Strict`,
//! `set_fuel(Some(100_000))`, no loader, `tokio::time::timeout(5s)` wrapping a
//! `spawn_blocking` join). The serde stage rejects anything that isn't a valid
//! `DocSpec` tree. The krilla stage walks a typed `Section` enum and emits
//! draw calls against embedded DejaVu Sans Regular/Bold cuts.
//!
//! Determinism (Pitfall 4 from 03-RESEARCH.md): krilla 0.7 produces stable
//! bytes when the input DocSpec is stable AND we either avoid timestamp
//! injection or pin it to a constant. The renderer pins both `producer` and
//! `creation_date` via `krilla::interchange::metadata::Metadata`, disables XMP
//! metadata via `SerializeSettings { xmp_metadata: false, .. }`, and applies a
//! regex-based post-process as a safety net against `/CreationDate`,
//! `/ModDate`, and `/Producer` PDF dictionary entries.

pub mod docspec;
pub mod fonts;
pub mod minijinja_env;
pub mod renderer;

pub use renderer::PdfRenderer;
