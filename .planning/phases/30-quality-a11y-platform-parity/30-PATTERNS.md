# Phase 30: Качество — доступность и паритет платформ - Pattern Map

**Mapped:** 2026-07-24
**Files analyzed:** 5 (1 new script-gate, 1 lint heuristic slot, 1 optional checklist doc, 2 concrete audit fixes confirmed + N pointwise candidates)
**Analogs found:** 5 / 5 (this is an audit/gap-closure phase — most "files" are audit findings, not blank-slate creates)

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `ui/scripts/check-contrast.mjs` | utility (script-gate) | batch (parse SCSS → compute → exit code) | `ui/scripts/check-tokens.mjs` | exact (same script-gate skeleton, same repo, zero-dep node) |
| bare-`outline:none` lint (new rule inside `check-contrast.mjs` OR sibling script) | utility (script-gate) | batch (regex scan → exit code) | `ui/scripts/check-tokens.mjs` Rule 1–4 pattern | exact |
| `ui/package.json` → `scripts.lint` (1-line edit) | config | n/a | `ui/package.json:16` (existing line) | exact — literal edit target |
| `ui/src/lib/components/Dropdown.svelte` (2 confirmed defects) | component | request-response (focus/keyboard interaction) | itself (compare `.tr-dropdown-search-input` vs `.tr-dropdown-field` in same file) | exact — in-file sibling pattern to copy from |
| `ui/src/features/cartridges/ModelListRow.svelte` (1 confirmed defect: non-inset ring inside `overflow:hidden` table) | component (table row) | request-response | `ui/src/features/acts/ActListRow.svelte:106-109` / `ui/src/features/cartridges/CartridgeListRow.svelte:148-151` (sibling row components, already fixed) | exact — same component family, already-correct sibling |
| `ui/src/lib/components/TableRow.svelte` `.tr-row-chevron` (no own `focus-visible`, relies on clippable global baseline) | component | request-response | same file's `.tr-row.selected` inset pattern (line 124-126) for the *technique*; `ActListRow.svelte:106-109` for the *ring* | role-match |
| `30-...WINDOWS-PARITY.md` (optional Windows/WebView2 best-effort checklist, D-03) | doc / checkpoint artifact | n/a | `.planning/phases/26-windows-with-mockup/26-08-*.md` + `26-CONTEXT.md` D-17/D-18 (both-theme UAT checkpoint precedent) | role-match |

**Correction to CONTEXT.md's stated candidate:** `ui/src/lib/components/Tabs.svelte` — CONTEXT says "своего `focus-visible` нет." This is **stale**: reading the file (2026-07-24) shows both variants already have `&:focus-visible { box-shadow: 0 0 0 3px var(--tr-focus-ring); }` (lines 95-98 underline variant, 161-163 segmented variant). **No action needed on Tabs.svelte** — the planner should re-verify empirically rather than trust this CONTEXT line, and should not spend a plan-task "fixing" it.

---

## Pattern Assignments

### `ui/scripts/check-contrast.mjs` (utility, batch script-gate)

**Analog:** `ui/scripts/check-tokens.mjs` (full file read, 279 lines)

**Header/doc-comment pattern** (lines 1-21):
```javascript
#!/usr/bin/env node
// [check-tokens] Постоянный CI-гейт дизайн-токенов (Phase 23, план 02, D-04).
//
// Four independently runnable checks over `ui/src`: ...
//
// Zero-dependency: только node:fs/node:path. `fs.readdirSync(dir, { recursive: true })`
// требует Node >= 20.1 (CI пинит node-version: '20' через actions/setup-node@v4, ...
//
// Usage: node scripts/check-tokens.mjs [--rules=1,2,3] [--src=<dir>]

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
```
`check-contrast.mjs` should open the same way: zero-dependency (`node:fs`, `node:path`, `node:url` only — no npm contrast-calc library is installed, see "No Analog Found" below), a `[check-contrast]` log prefix, and a doc comment citing Phase 30 / D-01.

**File collection + safe-read helpers** (lines 60-89) — copy verbatim, same signatures:
```javascript
function collectSourceFiles(srcDir) { /* recursive readdirSync, filter .svelte/.scss, tolerate missing dir */ }
function readFileSafe(filePath) { try { return fs.readFileSync(filePath, 'utf8'); } catch { return null; } }
```
For `check-contrast.mjs` you only need to read ONE file (`ui/src/styles/_tokens.scss`), so this can be simplified to a single `readFileSafe(tokensScssPath)` call — no need for the recursive scan unless the bare-`outline:none` lint is folded into the same script (in which case reuse `collectSourceFiles` verbatim for that half).

**Token-parsing regex idiom** (Rule 3, lines 172-216) — this is the closest existing code to "parse `_tokens.scss` into a name→value map," reuse the *shape*:
```javascript
const DEFINE_RE = /(--tr-[a-z0-9-]+)\s*:/gi;
```
`check-contrast.mjs` needs a **value-capturing** variant (check-tokens only captures the *name*, not the value, since Rule 3 only checks existence). Extend to also capture the RHS up to `;`, and do it **per theme block** — `_tokens.scss` has two selector blocks (`:root, [data-theme='light'] { ... }` and `[data-theme='dark'] { ... }`, see the exact ranges below) that must be parsed independently since the same token name resolves to a different hex/rgba per theme.

**Comment-stripping guard** (lines 176-190) — reuse verbatim; `_tokens.scss` has prose comments (e.g. `// #242c38 — one step lighter than --tr-surface-raised...`) that could otherwise false-match a token-like pattern:
```javascript
function stripCommentsForRule3(content) {
  let stripped = content.replace(/\/\*[\s\S]*?\*\//g, '');
  stripped = stripped.replace(/(^|[^:])\/\/.*$/gm, '$1');
  return stripped;
}
```

**exit-code / violation-accumulation pattern** (lines 222-278) — copy the `main()` shape exactly: accumulate `violations[]` with `{file/pair, line, detail}`, `console.error` each with a `[check-contrast]` prefix, sum a `totalViolations` counter, `process.exit(1)` if > 0 else `process.exit(0)` with a `PASS` line. This is the contract the planner's plan-tasks should assert against (script must exit non-zero on any sub-AA pair).

**CLI arg parsing** (lines 30-58) — same `--help`/`-h`, `parseArgs(argv)` shape if `check-contrast.mjs` needs flags (e.g. `--rules=contrast,outline` if the two D-01/D-04 checks are folded into one script) or omit entirely if kept as two separate zero-arg scripts (simpler, matches `verify-value-map.mjs`'s one-job-one-script philosophy — see Discretion note below).

---

### Exact token values available for contrast pairing (input data for `check-contrast.mjs`)

Source: `ui/src/styles/_tokens.scss` (full file, 267 lines). **Copied verbatim — do not recompute.**

**Light theme block** (`:root, [data-theme='light'] { ... }`, lines 12-88):
```scss
--tr-accent: #2b5fd9;
--tr-accent-hover: #2350bd;
--tr-accent-active: #1c4299;
--tr-accent-soft: rgba(43, 95, 217, 0.1);
--tr-accent-text: #2350bd;
--tr-on-accent: #ffffff;
--tr-focus-ring: rgba(43, 95, 217, 0.35);

--tr-bg: #eef1f6;
--tr-surface: #ffffff;
--tr-surface-raised: #ffffff;
--tr-surface-sunken: #e4e8f0;
--tr-overlay: rgba(20, 26, 38, 0.45);

--tr-text-primary: #1b2230;
--tr-text-secondary: #586074;
--tr-text-tertiary: #8891a4;
--tr-text-disabled: #aab2c1;
--tr-text-inverse: #ffffff;

--tr-border: #e1e6ef;
--tr-border-strong: #c9d0dd;

--tr-success: #12894e;
--tr-success-soft: rgba(18, 137, 78, 0.12);
--tr-success-text: #0f7343;

--tr-warning: #b9720c;
--tr-warning-soft: rgba(185, 114, 12, 0.14);
--tr-warning-text: #8f590a;

--tr-danger: #cf3b3b;
--tr-danger-hover: #b83232;
--tr-danger-active: #9d2929;
--tr-danger-soft: rgba(207, 59, 59, 0.12);
--tr-danger-text: #b02f2f;
--tr-danger-ring: rgba(207, 59, 59, 0.2);

--tr-info: #2b5fd9;
--tr-info-soft: rgba(43, 95, 217, 0.1);
--tr-info-text: #2350bd;

--tr-row-hover: #f1f4fa;
--tr-row-selected: rgba(43, 95, 217, 0.09);
--tr-group: #e9edf5;
```

**Dark theme block** (`[data-theme='dark'] { ... }`, lines 90-170):
```scss
--tr-accent: #5b8bff;
--tr-accent-hover: #7099ff;
--tr-accent-active: #4b78e8;
--tr-accent-soft: rgba(91, 139, 255, 0.16);
--tr-accent-text: #8fb0ff;
--tr-on-accent: #0e1218;
--tr-focus-ring: rgba(91, 139, 255, 0.45);

--tr-bg: #0e1218;
--tr-surface: #161b23;
--tr-surface-raised: #1c222c;
--tr-surface-sunken: #0a0d12;
--tr-overlay: rgba(0, 0, 0, 0.6);

--tr-text-primary: #e7ebf2;
--tr-text-secondary: #9aa3b4;
--tr-text-tertiary: #6b7486;
--tr-text-disabled: #4d5566;
--tr-text-inverse: #0e1218;

--tr-border: #272e3a;
--tr-border-strong: #39414f;

--tr-success: #2fbf74;
--tr-success-soft: rgba(47, 191, 116, 0.16);
--tr-success-text: #4fd08c;

--tr-warning: #e5a13a;
--tr-warning-soft: rgba(229, 161, 58, 0.16);
--tr-warning-text: #f0b45a;

--tr-danger: #f26565;
--tr-danger-hover: #ff7d7d;
--tr-danger-active: #e05555;
--tr-danger-soft: rgba(242, 101, 101, 0.16);
--tr-danger-text: #ff8080;
--tr-danger-ring: rgba(242, 101, 101, 0.2);

--tr-info: #5b8bff;
--tr-info-soft: rgba(91, 139, 255, 0.16);
--tr-info-text: #8fb0ff;

--tr-row-hover: #242c38;
--tr-row-selected: rgba(91, 139, 255, 0.14);
--tr-group: #1a212b;
```

**Focus-ring tokens exist in both themes** and are `rgba(..., alpha)` overlays, not solid text/bg colors — `check-contrast.mjs`'s D-01 scope is "text-on-background" pairs, so `--tr-focus-ring`/`--tr-danger-ring` are **not** contrast-check subjects themselves (they're visual affordance, not text), but the planner should note their line refs since CONTEXT cites them: light `--tr-focus-ring` line 21, `--tr-danger-ring` line 55; dark `--tr-focus-ring` line 98, `--tr-danger-ring` line 132 (line numbers confirmed against the read above — light block starts at `:root, [data-theme='light'] {` = line 13, dark block starts at line 91, both matching CONTEXT's citation).

**Candidate text↔background pairs to seed the script's canonical list** (planner should confirm against `23-UI-SPEC.md` and actual component usage, per D-01's "Действие для планировщика," but these are the highest-confidence pairs from grep of `--tr-text-*`/`--tr-surface-*`/`--tr-bg` co-occurring in the same component):
- `--tr-text-primary` on `--tr-bg`, `--tr-surface`, `--tr-surface-raised`, `--tr-surface-sunken`
- `--tr-text-secondary` on the same 4 surfaces (this pair is the AA risk: secondary is deliberately lower-contrast)
- `--tr-text-tertiary` on the same 4 surfaces (likely the large-text-only exception candidate, D-01's 3:1 carve-out)
- `--tr-text-inverse` on `--tr-accent`, `--tr-danger` (button/badge text-on-solid pairs — `--tr-on-accent` is literally this pair pre-named)
- `--tr-accent-text` on `--tr-accent-soft`, `--tr-surface` (soft-badge and link-on-surface pairs)
- `--tr-success-text`/`--tr-warning-text`/`--tr-danger-text`/`--tr-info-text` on their respective `*-soft` background AND on `--tr-surface` (semantic badges appear on both)
- `--tr-text-primary`/`--tr-text-secondary` on `--tr-row-hover`, `--tr-row-selected`, `--tr-group` (table states)

---

### Bare-`outline:none` lint (utility, batch script-gate — D-04)

**Analog:** same `check-tokens.mjs` skeleton; heuristic-rule shape closest to Rule 2 (`checkHexInStyle`, lines 119-142) — scan `<style>` blocks of `.svelte` files with a regex, collect violations with file+line.

**Concrete heuristic requirement (from live grep, not guesswork):** a bare/undefended `outline: none;` is one **not** immediately followed (within a few lines, same rule block) by a `box-shadow` declaration. Live evidence of the "safe" pattern (39 total `outline:\s*none` hits in `ui/src/`, **37 of 39 are already paired** with a `box-shadow` on an adjacent line, order-insensitive — box-shadow can appear either immediately before OR immediately after `outline: none;`):
```scss
/* ui/src/features/acts/ActListRow.svelte:106-109 — box-shadow AFTER */
&:focus-visible {
  outline: none;
  box-shadow: inset 0 0 0 2px var(--tr-accent);
}

/* ui/src/lib/components/Modal.svelte:245-248 — box-shadow BEFORE */
&:focus-visible {
  box-shadow: 0 0 0 3px var(--tr-focus-ring);
  outline: none;
}
```
The heuristic must tolerate **both orderings** and skip intervening property lines like `border-color: ...;` that commonly sit between them (see `Input.svelte:93-95`, `DeviceFormBody.svelte:429-431`). A safe implementation: within each `&:focus-visible`/`&:focus` rule body (or within N=4 lines of the `outline: none;` line, bounded by the enclosing `{ }`), require at least one `box-shadow` line.

**Confirmed REAL violations found by manual audit (use these as the lint's self-test fixtures / first fix targets):**
1. `ui/src/lib/components/Dropdown.svelte:922-935` — `.tr-dropdown-search-input` (search box inside the portaled dropdown panel) has `outline: none;` (line 927) with **no** `box-shadow`/`:focus-visible` replacement anywhere in that rule. This is a genuine defect — the search `<input>` loses all focus indication.
   ```scss
   :global(.tr-dropdown-panel .tr-dropdown-search-input) {
     flex: 1 1 auto;
     min-width: 0;
     background: transparent;
     border: none;
     outline: none;
     color: var(--tr-text-primary);
     font-family: var(--tr-font-family);
     font-size: var(--tr-font-size-label);
     &::placeholder { color: var(--tr-text-tertiary); }
   }
   ```
   Fix idiom to copy: any of the other `Dropdown.svelte` field patterns, e.g. `.tr-dropdown-field:focus-visible` (lines 719-722): `outline: none; border-color: var(--tr-accent); box-shadow: 0 0 0 3px var(--tr-focus-ring);` — add an equivalent `&:focus-visible { box-shadow: 0 0 0 2px var(--tr-focus-ring); }` block (2px, not 3px, since the box is small/inline — match `Sidebar.svelte:238-240`'s 2px inset-adjacent scale) to `.tr-dropdown-search-input`.
2. `features/settings/TemplateEditor.svelte:466-470` uses `&:focus` (not `&:focus-visible`) — not a "no replacement" bug (box-shadow IS present), but an inconsistency worth flagging in the audit since every other primitive in the codebase standardized on `:focus-visible` (keyboard-only ring, no mouse-click ring). Confirm whether this is intentional (textarea may want click-ring too) or a drift the lint should also catch as a secondary heuristic (`:focus` without `-visible` on an interactive rule).

**2 files with `outline: none;` NOT preceded by `&:focus-visible`/`&:focus` at all** (base-style resets, not focus rules — do NOT flag, but the lint must not false-positive on these):
- `ui/src/lib/components/Tabs.svelte:79` — inside the base `.tab { ... }` rule (browser-default outline reset applied unconditionally); the actual ring is defined in a **separate** `&:focus-visible { box-shadow: ...; }` block 16 lines later (95-98 underline variant, 161-163 segmented variant). A same-rule-body heuristic would false-flag this — the lint needs to also check "does this selector (or `&:focus-visible` under the same parent selector) exist anywhere later in the same file," not just the same rule block. Cheapest safe option: whitelist/allow multi-block pairing by tracking the enclosing parent selector name, not just brace-nesting.
- `ui/src/lib/components/Dropdown.svelte:927` — see violation #1 above (this one genuinely IS a defect, unlike Tabs).

---

### `ui/package.json` → `scripts.lint` wiring

**Analog / exact edit target:** `ui/package.json:16` (verbatim, current state):
```json
"lint": "eslint . --ext .ts,.svelte && prettier --check . && node scripts/check-tokens.mjs"
```
Append `&& node scripts/check-contrast.mjs` (and the outline-lint invocation, if it's a separate script) in the same `&&`-chained style — this is the only integration point; `check-tokens.mjs` is the direct precedent for "how a new script-gate joins `pnpm lint`." Do **not** add it to `prebuild`/`build` — those are reserved for `cargo test ... export_bindings` (see `scripts.prebuild:13`) and Vite build respectively; script-gates live in `lint` per existing convention.

---

### `ui/src/features/cartridges/ModelListRow.svelte` (component, table row) — confirmed clip-risk defect

**Analog (already-correct sibling in the SAME component family):** `ui/src/features/acts/ActListRow.svelte:102-110` and `ui/src/features/cartridges/CartridgeListRow.svelte:145-151`

**The defect** — `ModelListRow.svelte:179-182` (current, non-inset):
```scss
&:focus-visible {
  outline: none;
  box-shadow: 0 0 0 3px var(--tr-focus-ring);
}
```
This row renders via `TableRow.svelte` inside `Table.svelte`, whose framed wrapper clips content:
```scss
/* ui/src/lib/components/Table.svelte:92-97 */
.tr-table-framed.framed {
  border: 1px solid var(--tr-border);
  border-radius: 8px;
  overflow: hidden;
  box-shadow: var(--tr-elev-1);
}
```
A non-inset `box-shadow: 0 0 0 3px` extends 3px **outward** from the row's cell border and gets truncated on the left/right/top/bottom by this `overflow: hidden` frame — exactly the D-02 dimension-2 defect. `ModelListRow.svelte` is used by `ModelsList.svelte:41` (`<Table ...>`), confirmed via `TableRow` import at `ModelListRow.svelte:10`.

**The fix pattern to copy verbatim** — `ActListRow.svelte:102-110`:
```scss
.cell-number {
  width: 72px;
  cursor: pointer;

  &:focus-visible {
    outline: none;
    box-shadow: inset 0 0 0 2px var(--tr-accent);
  }
}
```
And `CartridgeListRow.svelte:145-151` (identical idiom, different cell class name). Both siblings already migrated to `inset 0 0 0 2px var(--tr-accent)` — this is the established, repo-wide idiom for "focus ring on a table-row cell inside a `overflow: hidden` framed table." `ModelListRow.svelte` should be changed to match (`inset 0 0 0 2px var(--tr-accent)`, replacing the outward 3px ring). `PrinterListRow.svelte:132-136` and `RequestListRow.svelte:164-169` (with its explanatory UAT comment) are two more instances of the same already-fixed idiom, useful as additional reference.

---

### `ui/src/lib/components/TableRow.svelte` `.tr-row-chevron` (component, group-row toggle button) — clip-risk candidate

**Analog:** same file's own `.tr-row.selected :global(> td:first-child)` inset technique (lines 118-126) for the *inset idiom*; `ActListRow.svelte:106-109` for the ring value.

**Current state** (`TableRow.svelte:137-157`, full rule reproduced) — **no own `&:focus-visible` at all**:
```scss
.tr-row-chevron {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  padding: 0;
  margin-right: var(--tr-space-2xs);
  background: transparent;
  border: none;
  color: var(--tr-text-secondary);
  cursor: pointer;
  transform: none;
  transition: transform 0.15s;

  &.expanded {
    transform: rotate(90deg);
  }
}
```
This button relies purely on the global baseline (`global.scss:40-43`, non-inset `box-shadow: 0 0 0 3px`). Since `TableRow` is rendered inside `Table.svelte`'s `overflow: hidden` framed wrapper (same as ModelListRow above) and the chevron sits at the row's left edge (`.tr-row-group-name` first cell), the 3px outward ring is clip-risk on rows near the table edge. **Add** an inset rule following the `ActListRow`/`CartridgeListRow` idiom:
```scss
&:focus-visible {
  outline: none;
  box-shadow: inset 0 0 0 2px var(--tr-accent);
}
```
This is a `TableRow.svelte`-owned primitive (used by every grouped table in the app — Devices, at minimum, per the file's own doc comment referencing `DeviceGroupRow precedent`), so fixing it here has the widest blast-radius payoff of any single-file change in this phase.

---

### `ui/src/lib/components/Dropdown.svelte` `.tr-dropdown-option` (portaled panel items) — clip-risk candidate, needs UAT confirmation

**Context:** `Dropdown.svelte:793-826` — the panel is portaled to `<body>` (`use:portal`, see the file's own Plan-18-04 comment) and has its own `overflow: auto` (not `hidden`, but still a scroll-clipping box):
```scss
:global(.tr-dropdown-panel) {
  position: fixed;
  z-index: 1000;
  overflow: auto;
  max-height: 280px;
  ...
}
:global(.tr-dropdown-panel .tr-dropdown-option) {
  display: flex;
  ...
  background: transparent;
  border: none;
  border-bottom: 1px solid var(--tr-border);
  cursor: pointer;
  color: var(--tr-text-primary);
  font-family: var(--tr-font-family);
  font-size: var(--tr-font-size-body);
}
```
`.tr-dropdown-option` has **no own `&:focus-visible`** — relies on the global baseline's outward 3px ring. Because the panel scrolls (`overflow: auto`, `max-height: 280px`), an option row focused near the very top/bottom edge of the visible scroll window can have its ring clipped by the scroll boundary — this is the literal "Dropdown panel" clip risk named in CONTEXT/D-02. **This is a genuine candidate but needs visual UAT confirmation** (script-gates can't detect scroll-clipping) — flag for the both-theme UAT checkpoint (D-04), not necessarily a blind code fix. If confirmed, the fix idiom is the same inset technique as above, scoped to `:global(.tr-dropdown-panel .tr-dropdown-option:focus-visible)`.

---

## Shared Patterns

### Canonical `&:focus-visible` ring idiom (non-clipped context)
**Source:** `ui/src/lib/components/Button.svelte:87-89` (also identical in Input, Checkbox, Select, PrinterSelect, PersonAutocomplete, LocationAutocomplete, DatePicker, ActionMenu, PeriodToggle, DashboardPage, DeviceAutocompleteField, DeviceFormBody, UserFormModal, TemplateEditor, ModelFormModal, CompatibilityEditor, CartridgeContextMenu, DeviceContextMenu — 20+ occurrences, this IS the house style):
```scss
&:focus-visible {
  box-shadow: 0 0 0 3px var(--tr-focus-ring);
}
```
Sometimes paired with `border-color: var(--tr-accent);` for bordered fields (Input/Textarea/Select-style components). Apply this idiom to any NEW interactive element that is **not** inside a clipping ancestor.

### Canonical `&:focus-visible` inset-ring idiom (clipped context — table rows, D-02 fix)
**Source:** `ui/src/features/acts/ActListRow.svelte:106-109`, `ui/src/features/cartridges/CartridgeListRow.svelte:148-151`, `ui/src/features/printers/PrinterListRow.svelte:133-136`, `ui/src/features/requests/RequestListRow.svelte:165-168`, `ui/src/features/layout/Sidebar.svelte:152-155` (nav-link) and `:237-240` (logout-btn, non-inset 2px variant):
```scss
&:focus-visible {
  outline: none;
  box-shadow: inset 0 0 0 2px var(--tr-accent);
}
```
Apply this idiom whenever a focusable element sits inside an ancestor with `overflow: hidden`/`overflow: auto` where an outward ring would be clipped (ModelListRow, TableRow chevron — see above). Note: table-row variants use `var(--tr-accent)` (solid), while Sidebar uses `var(--tr-focus-ring)` (the alpha-overlay token) — both are attested in the codebase; prefer `--tr-focus-ring` for new inset fixes unless matching an existing row family that already uses `--tr-accent` (stay consistent within the same component family).

### Global focus-visible baseline (KEEP, do not remove)
**Source:** `ui/src/styles/global.scss:40-43`
```scss
*:focus-visible {
  outline: none;
  box-shadow: 0 0 0 3px var(--tr-focus-ring);
}
```
This is the safety net (D-02) — every interactive element gets SOME ring by default; the phase's job is finding where this gets locally **overridden without replacement** or **clipped**, not rebuilding it.

### Script-gate wiring convention
**Source:** `ui/package.json:16`, `ui/scripts/check-tokens.mjs` (whole file)
- Zero-dependency Node scripts (`node:fs`, `node:path`, `node:url` only — no npm packages added).
- Exit 0 + `console.error('[prefix] PASS — 0 нарушений')` on success; exit 1 + one `console.error` line per violation + a summary `FAIL` line on failure. (Uses `console.error` even for the PASS line — matches existing convention, keeps stdout clean for potential piping.)
- Registered into `pnpm lint` via `&&`-chaining, never into `build`/`prebuild`.
- Doc-comment header cites the originating Phase/plan/decision (e.g. `// [check-contrast] Постоянный CI-гейт контраста (Phase 30, D-01).`).

---

## No Analog Found

| File/Concern | Role | Data Flow | Reason |
|---|---|---|---|
| WCAG relative-luminance / contrast-ratio math inside `check-contrast.mjs` | utility (pure function) | transform | No existing contrast-calculation code or npm dependency (`package.json` deps list confirmed — no `wcag-contrast`/`color`/`chroma-js` etc.) exists in the repo. Must be hand-written: parse hex → sRGB → linearize (gamma 2.4 piecewise) → relative luminance `L = 0.2126*R + 0.7152*G + 0.0722*B` → contrast ratio `(L1+0.05)/(L2+0.05)` with L1 ≥ L2 (standard WCAG 2.x formula, zero-dependency, ~20 lines). `rgba()` alpha-blended tokens (`--tr-accent-soft`, `--tr-*-soft`, `--tr-focus-ring`) additionally need alpha-compositing against their actual background token before luminance — flag this as a design decision for the planner (D-01's "Действие для планировщика": which pairs are meaningful) since blending changes per background. |
| `30-...WINDOWS-PARITY.md` exact structure | doc | n/a | No file in `.planning/phases/26-windows-with-mockup/` or `29-login-and-employee-shell/` is a pure "checklist for a machine we don't have" — those phases' UAT checkpoints (`26-CONTEXT.md` D-17/D-18, `26-08-SUMMARY.md`) assume same-machine testing. The planner has discretion (per CONTEXT `<specifics>` Discretion list) on exact form/placement; closest structural precedent is a `26-08-PLAN.md`-style final-wave checkpoint doc, adapted into a static checklist (screens × themes × "expected identical / known-divergent" columns) rather than a live-executed plan. |

---

## Metadata

**Analog search scope:** `ui/scripts/`, `ui/src/styles/`, `ui/src/lib/components/`, `ui/src/features/**/*.svelte`, `ui/package.json`, `.planning/phases/26-windows-with-mockup/`, `.planning/phases/29-login-and-employee-shell/`
**Files scanned:** `check-tokens.mjs`, `verify-value-map.mjs`, `_tokens.scss`, `global.scss`, `Tabs.svelte`, `Button.svelte`, `Table.svelte`, `TableRow.svelte`, `Dropdown.svelte`, `Modal.svelte`, `Toast.svelte`, `PageHeader.svelte`, `Sidebar.svelte`, `DashboardPage.svelte`, `PeriodToggle.svelte`, `ActListRow.svelte`, `RequestListRow.svelte`, `ModelListRow.svelte`, `CartridgeListRow.svelte`, `CartridgeContextMenu.svelte`, `TemplateEditor.svelte`, `package.json`; plus full-tree grep of `outline:\s*none` (39 hits, all triaged above) and `overflow:\s*hidden` in `lib/components/` (9 hits, all triaged above).
**Pattern extraction date:** 2026-07-24
