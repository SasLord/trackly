---
phase: 23-design-tokens-foundations
reviewed: 2026-07-17T15:40:10Z
depth: standard
files_reviewed: 14
files_reviewed_list:
  - ui/eslint.config.js
  - ui/package.json
  - ui/scripts/check-tokens.mjs
  - ui/scripts/verify-value-map.mjs
  - ui/src/features/acts/ActFormBody.svelte
  - ui/src/features/acts/ActFormItemsTable.svelte
  - ui/src/features/acts/PdfPreviewModal.svelte
  - ui/src/features/dashboard/ChartWidget.svelte
  - ui/src/lib/api/acts.ts
  - ui/src/lib/components/Badge.svelte
  - ui/src/lib/components/Button.svelte
  - ui/src/lib/components/PersonAutocomplete.svelte
  - ui/src/styles/_tokens.scss
  - ui/src/styles/global.scss
findings:
  critical: 2
  warning: 4
  info: 1
  total: 7
status: issues_found
---

# Phase 23: Code Review Report

**Reviewed:** 2026-07-17T15:40:10Z
**Depth:** standard
**Files Reviewed:** 14
**Status:** issues_found

## Summary

Reviewed the 14 high-signal files from the Phase 23 design-token migration: the two
Node verification scripts (`check-tokens.mjs` permanent gate, `verify-value-map.mjs`
one-shot), `_tokens.scss` + `global.scss`, and six representative Svelte
components/API files that went through the mechanical `--color-*/--shadow-*/
--space-*/--radius-*/--font-*` → `--tr-*` sweep.

`check-tokens.mjs` was executed directly (`node scripts/check-tokens.mjs`) and
correctly reports 0 violations against the current tree; `eslint . --ext .ts,.svelte`
also passes clean. The mechanical sweep itself (spot-checked via `git show` against
several sweep commits) is value-preserving everywhere it was actually verified.

However, `verify-value-map.mjs` — the tool whose entire job is to catch
value-preserving mapping mistakes in that sweep — has a confirmed logic bug that
silently drops every token after the first on any line with multiple
`--space-*`/`--radius-*` tokens (a very common pattern: `padding: var(--x) var(--y);`).
I reproduced this against a real historical hunk from the actual sweep
(`CartridgeListRow.svelte`) and confirmed the second token on the line was never
compared at all, despite the tool reporting "0 нарушений". 116+ lines in the current
tree have this shape, so the tool's "0 нарушений" pass result materially overstates
the coverage that was actually achieved.

Separately, `ActFormItemsTable.svelte`'s `removeRow()` (pre-existing, not introduced
by this phase, but present in a reviewed file) reindexes every per-row `$state` map
except the plain `debounceTimers` object, which can deliver a device search response
into the wrong row after a row is removed while another row's search is in flight.

Also found: a hardcoded, non-token `rgba(220, 38, 38, …)` "invalid" focus-ring color
duplicated across 9 files (including 2 of the reviewed files) that does not match the
`--tr-danger` token value and is invisible to the hex-only Rule 2 gate — a concrete,
already-realized instance of the gate's blind spot for non-hex color literals.

## Critical Issues

### CR-01: `verify-value-map.mjs` only compares the first `--space-*`/`--radius-*` token per line, silently ignoring the rest

**File:** `ui/scripts/verify-value-map.mjs:76-97`
**Issue:**
`checkHunk()` extracts tokens with:
```js
const removedTokens = [...hunkText.matchAll(/^-.*?(--(?:space|radius)-[a-z0-9]+)/gm)].map(m => m[1]);
const addedTokens = [...hunkText.matchAll(/^\+.*?(--tr-(?:space|radius)-[a-z0-9]+)/gm)].map(m => m[1]);
```
Because `^` anchors to line-start and `.*?` is lazy, `matchAll` with the `m` flag
produces **at most one match per line** — once the first `--space-*`/`--radius-*`
occurrence on a `-`/`+` line is consumed, the regex engine advances past that match
and the next attempt requires a fresh line start, so any *additional* token later on
the same line is never captured, on either the removed or the added side.

This is not theoretical. Multi-token lines are the majority pattern for spacing CSS
(`padding: var(--tr-space-xs) var(--tr-space-md);`). I confirmed 116+ such lines
exist in the current tree (`grep -c` across `src/**/*.svelte`), and reproduced the
bug directly against the real sweep commit `16244e2` (`CartridgeListRow.svelte`):
```
-  padding: var(--space-sm) var(--space-md);
+  padding: var(--tr-space-xs) var(--tr-space-md);
```
Running the script's own extraction logic against this exact hunk yields:
```
removed: [ '--space-sm' ]
added:   [ '--tr-space-xs' ]
```
`--space-md` → `--tr-space-md` is silently never compared on either side. Since
`removedTokens.length === addedTokens.length` (1 === 1) in this case, no
count-mismatch fires either — the second mapping is invisible to the tool, not
merely uncounted. Running `node scripts/verify-value-map.mjs 16244e2~1` against the
real historical base reports `PASS — 578 хунков проверено, 0 нарушений`, but the
"0 нарушений" result reflects only the subset of tokens the regex happened to see
(one per changed line), not the full set of renamed tokens. A wrong mapping on the
second (or third) token of a multi-token line would have passed silently.
**Fix:** Extract *all* tokens on each line, not just the first. Strip the leading
`-`/`+` marker per line and run a global, non-anchored match across the whole line
content:
```js
function tokensOnSide(hunkText, marker, re) {
  const out = [];
  for (const line of hunkText.split('\n')) {
    if (!line.startsWith(marker)) continue;
    for (const m of line.matchAll(re)) out.push(m[1]);
  }
  return out;
}
const removedTokens = tokensOnSide(hunkText, '-', /(--(?:space|radius)-[a-z0-9]+)/g);
const addedTokens = tokensOnSide(hunkText, '+', /(--tr-(?:space|radius)-[a-z0-9]+)/g);
```
This still preserves the "flat compare within hunk, not line-paired" design the
comment above `checkHunk` describes, but stops dropping same-line tokens.

### CR-02: `ActFormItemsTable.svelte` `removeRow()` does not reindex `debounceTimers`, so a wrong row can receive another row's search results

**File:** `ui/src/features/acts/ActFormItemsTable.svelte:141-157` (bug present since phase 18, `debounceTimers` declared line 89)
**Issue:** `removeRow(idx)` correctly reindexes every per-row `$state` map via
`shiftRowState()` (`suggestionsByRow`, `loadingByRow`, `openByRow`, `viewModeByRow`,
`drillGroupByRow`, `membersByRow`, `activeIndexByRow`, `showBackByRow`), but
`debounceTimers` — the plain (non-reactive) object keyed by row index that
`handleQueryInput`/`handleFocus` populate — is only cleared/deleted **at `idx`
itself**:
```js
if (debounceTimers[idx]) clearTimeout(debounceTimers[idx]);
delete debounceTimers[idx];
```
Timers belonging to rows *after* `idx` are never re-keyed. Concrete repro: with 4
rows (indices 0-3), type into row 3's device search (schedules
`debounceTimers[3] = setTimeout(() => fetchGroups(3, ...), 250)`), then remove row 0
before the timer fires. `items` now has 3 rows (old rows 1,2,3 → new indices 0,1,2);
all the `*ByRow` maps shift accordingly, but `debounceTimers[3]` is untouched. When
the timer fires 250ms later it still calls `fetchGroups(3, ...)`, which writes into
`suggestionsByRow[3]`/`openByRow[3]` — index 3 no longer corresponds to the row the
user was typing in (it's either out of range or, with 5+ original rows, a
*different* row entirely) — so a device search intended for one row can populate the
dropdown of an unrelated row. In an application whose core value proposition is
accurate device handover records, presenting the wrong device suggestions under the
wrong table row is a real correctness/data-integrity risk, not just a cosmetic glitch.
**Fix:** Reindex `debounceTimers` the same way as the other per-row maps (it isn't
`$state` so it needs a manual key-shift rather than `shiftRowState`, since that
helper returns a new object assigned via reactive `=`):
```ts
function shiftDebounceTimers(idx: number) {
  const keys = Object.keys(debounceTimers).map(Number).sort((a, b) => b - a);
  for (const k of keys) {
    if (k === idx) continue;
    if (k > idx) {
      debounceTimers[k - 1] = debounceTimers[k];
      delete debounceTimers[k];
    }
  }
}
// in removeRow, after clearing debounceTimers[idx]:
shiftDebounceTimers(idx);
```

## Warnings

### WR-01: Hardcoded `rgba(220, 38, 38, …)` "invalid" focus-ring color duplicated across 9 files, does not match `--tr-danger` and evades the token gate

**File:** `ui/src/lib/components/PersonAutocomplete.svelte:281`, `ui/src/features/acts/ActFormItemsTable.svelte:985,1010` (and 6 more files outside this review's file list: `Button.svelte:109`, `Input.svelte:67`, `DatePicker.svelte:67`, `LocationAutocomplete.svelte:200`, `DeviceAutocompleteField.svelte:416`, `ModelFormModal.svelte:526`)
**Issue:** All nine sites use the identical literal:
```scss
box-shadow: 0 0 0 3px rgba(220, 38, 38, 0.2); // (0.3 in Button.svelte)
```
`rgba(220, 38, 38, …)` is `#dc2626` — a *different* red than the design system's
`--tr-danger` token (`#cf3b3b` light / `#f26565` dark, defined in `_tokens.scss`).
This is a leftover pre-token-migration magic number: it was never swept to a
`--tr-danger`-derived value, is duplicated nine times instead of centralized, and —
because `check-tokens.mjs` Rule 2 only matches hex literals (`HEX_RE`), not
`rgba()`/`hsl()` — it is permanently invisible to the new permanent CI gate this
phase introduced. It also means the "invalid" state ring color is visually
inconsistent with every other danger-colored UI element (error text, error badges,
etc.) which all correctly use `--tr-danger`/`--tr-danger-soft`.
**Fix:** Add a token derived from `--tr-danger` (e.g. `--tr-danger-ring: rgba(207, 59,
59, 0.2)` per theme in `_tokens.scss`, alongside `--tr-focus-ring`) and replace all
nine occurrences with `box-shadow: 0 0 0 3px var(--tr-danger-ring);`.

### WR-02: `ChartWidget.svelte` tooltip uses a hardcoded `rgba()` shadow instead of the `--tr-elev-*` token used by equivalent overlay components

**File:** `ui/src/features/dashboard/ChartWidget.svelte:398`
**Issue:**
```scss
.chart-tooltip {
  ...
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
}
```
Every other reviewed overlay/dropdown element (`PdfPreviewModal.svelte:335`,
`ActFormItemsTable.svelte:818,909`, `PersonAutocomplete.svelte:308`) uses
`var(--tr-elev-1)`/`var(--tr-elev-2)` for exactly this kind of "floating element"
shadow, including dark-theme variants. This tooltip's shadow was left as a raw
literal and will not adapt to dark theme the way sibling components do, and — like
WR-01 — it is invisible to Rule 2's hex-only regex.
**Fix:** `box-shadow: var(--tr-elev-1);` (or `--tr-elev-2` to match the dropdown
components it visually resembles).

### WR-03: `check-tokens.mjs` Rule 2 only detects hex literals, not other CSS color-literal forms

**File:** `ui/scripts/check-tokens.mjs:113-136`
**Issue:** `HEX_RE` (`#[0-9a-fA-F]{3,4}\b|...`) is the sole detector for "raw color in
`<style>` block." WR-01 and WR-02 above are concrete, already-existing proof that
`rgba()`/`hsl()` literals slip through this gate undetected and will continue to for
any future regression — the gate cannot prevent new hardcoded `rgba(...)` colors
from being introduced going forward, defeating part of the purpose of a "closed-world
color token" gate.
**Fix:** Extend Rule 2 (or add a Rule 4) to also flag `rgba(`, `rgb(`, `hsl(`,
`hsla(` function calls inside `<style>` blocks that are not wrapped in
`var(--tr-...)` (a plain regex like `/\b(?:rgba?|hsla?)\(/g` scoped to the same
`STYLE_BLOCK_RE` blocks used by the existing hex check is enough for this repo's
"grep, not parser" philosophy).

### WR-04: `PdfPreviewModal.svelte` desktop print path uses a predictable filename in the shared OS temp directory with no cleanup

**File:** `ui/src/features/acts/PdfPreviewModal.svelte:194-198`
**Issue:**
```js
const fileName = `trackly-print-${Date.now()}.html`;
await writeTextFile(fileName, htmlWithAutoPrint, { baseDir: BaseDirectory.Temp });
```
The rendered act document — which contains personal names (giver/receiver),
device/inventory data, and internal act numbers — is written under a predictable,
millisecond-timestamp-based name into the OS-shared temp directory
(`%TEMP%`/`/tmp`), which on a shared multi-user Windows terminal-services-style
machine (plausible for this app's "Windows AD network" target environment) may be
world-readable by other local accounts. The file is also never cleaned up after the
system browser opens/prints it, so these act documents accumulate indefinitely in
temp.
**Fix:** Use an unpredictable name (`crypto.randomUUID()`-derived) and, since the
app can't reliably know when the external browser process is done with the file,
sweep/delete `trackly-print-*.html` files older than e.g. 1 hour on next app start
(or on `printViaSystemBrowser` invocation, before writing the new one).

## Info

### IN-01: `Badge.svelte` uses hardcoded pixel values not on the `--tr-*` scale

**File:** `ui/src/lib/components/Badge.svelte:23,24,35`
**Issue:** `border-radius: 10px;`, `font-size: 12px;` (`.badge`), and `font-size:
11px;` (`.badge-sm`) are literal pixel values that don't correspond to any
`--tr-radius-*`/`--tr-font-size-*` token (closest radius token is `--tr-radius-xs`
at 4px or `--tr-radius-full` at 999px; closest font-size tokens are
`--tr-font-size-caption` at 12px, matching one of the two, and
`--tr-font-size-micro` at 11px, matching the other). These aren't flagged by
Rule 1/Rule 2 (no hex, no old-family var name) and may be an intentional pill-badge
exception, but as-is they're an unexplained gap in an otherwise fully-tokenized
component and worth a one-line comment or conversion to
`var(--tr-font-size-caption)`/`var(--tr-font-size-micro)` for the two font-size
values, which are exact matches.
**Fix:** `font-size: var(--tr-font-size-caption);` in `.badge`, `font-size:
var(--tr-font-size-micro);` in `.badge-sm`; leave/comment the 10px radius if it's a
deliberate pill-shape choice outside the 5-level radius scale.

---

_Reviewed: 2026-07-17T15:40:10Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
