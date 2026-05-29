# Embedded Fonts (Trackly Phase 3)

This directory contains TrueType fonts embedded into the `trackly-app` binary via `include_bytes!`
for use by the PDF subsystem (`crates/trackly-app/src/pdf/`).

## Files

- `DejaVuSans.ttf` — Regular cut
- `DejaVuSans-Bold.ttf` — Bold cut

## Version

**DejaVu Sans 2.37** — the last upstream release from the DejaVu Fonts project (2016-07-30).
Download source: https://dejavu-fonts.github.io/Download.html

## License

DejaVu Sans is **public-domain-derived** (descended from Bitstream Vera Fonts).
The Bitstream Vera Fonts License grants free use, modification, and embedding without
attribution in user-facing UI. See the upstream license document:
https://dejavu-fonts.github.io/License.html

These properties allow Trackly to embed both cuts directly into the binary and ship
them as part of the portable bundle without any runtime download or external
font installation step.

## Why DejaVu Sans (per D-PDF-Engine-01)

- **Cyrillic coverage:** all Russian glyphs including `ё` and Latin diacritics — exactly
  what the canonical acceptance fixture `«Сидоров-Петроградский Иван Александрович (ё) №42»`
  exercises.
- **Subsetting friendly:** krilla 0.7's OpenType subsetting only embeds the glyphs actually
  used by each rendered PDF, keeping output sizes small even though the source TTFs are
  ~700 KB each.
- **Stable license:** public-domain-derived removes any attribution burden in the
  application's printed output.

## What NOT to add here

- **No additional cuts** (Oblique, Mono, Serif, Condensed) — Phase 3 acts and acceptance
  documents only need Regular + Bold. Adding more cuts inflates the binary without
  rendering benefit.
- **No other font families** — if a future phase needs a second family (e.g. for monospace
  device serial numbers), add it in that phase with its own license review.
