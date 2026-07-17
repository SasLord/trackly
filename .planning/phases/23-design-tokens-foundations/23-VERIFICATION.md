---
phase: 23-design-tokens-foundations
verified: 2026-07-17T23:10:00Z
status: gaps_found
score: 6/7 must-haves verified
overrides_applied: 0
gaps:
  - truth: "Захардкоженных цветов в компонентах не остаётся (DS-01, REQUIREMENTS.md verbatim text)"
    status: partial
    reason: >
      check-tokens.mjs Rule 2 only matches hex literals (HEX_RE), not rgba()/hsl()/hsla(). 17
      hardcoded rgba() color/shadow literals remain inside <style> blocks across 14 files after
      the full sweep — invisible to the phase's own permanent CI gate. Two categories are
      materially significant: (1) a duplicated "invalid-state" focus ring
      `rgba(220, 38, 38, 0.2)`/`0.3` in 9 files that does NOT match the --tr-danger token value
      (#cf3b3b light / #f26565 dark) — a different, un-tokenized red; (2) Modal.svelte's overlay
      background (`rgba(0, 0, 0, 0.4)` / `rgba(0, 0, 0, 0.6)`) does not use the --tr-overlay token
      that _tokens.scss defines for exactly this purpose (rgba(20, 26, 38, 0.45) light /
      rgba(0, 0, 0, 0.6) dark) — the single most widely-reused overlay component in the app
      bypasses the token layer it's supposed to consume. Also found: 4 auth-screen box-shadows
      (rgba(0,0,0,0.08)) and a ChartWidget tooltip shadow (rgba(0,0,0,0.15)) not using --tr-elev-*.
      This directly contradicts the REQUIREMENTS.md DS-01 clause "захардкоженных цветов в
      компонентах не остаётся" — the plan's own must_haves (23-03) scoped Rule 2 to hex-only,
      which is narrower than the roadmap-level DS-01 promise.
    artifacts:
      - path: "ui/scripts/check-tokens.mjs"
        issue: "Rule 2 (hex-in-style gate) has no detection for rgba()/hsl()/hsla() color-function literals — confirmed blind spot (WR-03 in 23-REVIEW.md)"
      - path: "ui/src/lib/components/Modal.svelte"
        issue: "Overlay background hardcoded as rgba(0,0,0,0.4)/rgba(0,0,0,0.6) instead of var(--tr-overlay), despite --tr-overlay existing in _tokens.scss for this exact purpose"
      - path: "ui/src/lib/components/PersonAutocomplete.svelte, ActFormItemsTable.svelte (x2), Button.svelte, Input.svelte, DatePicker.svelte, LocationAutocomplete.svelte, DeviceAutocompleteField.svelte, ModelFormModal.svelte"
        issue: "9 sites of hardcoded rgba(220, 38, 38, ...) 'invalid' focus ring that doesn't match --tr-danger's actual token value"
      - path: "ui/src/features/auth/BlockedScreen.svelte, FirstRunWizard.svelte, LoginPage.svelte, PendingScreen.svelte, ui/src/features/dashboard/ChartWidget.svelte"
        issue: "Hardcoded rgba() box-shadows not using --tr-elev-* tokens"
    missing:
      - "Extend check-tokens.mjs with a Rule 4 (or extend Rule 2) to flag rgba(/rgb(/hsl(/hsla( function calls inside <style> blocks not wrapped in var(--tr-...) — closes the permanent gate's blind spot for future regressions"
      - "Add a --tr-danger-ring token derived from --tr-danger and replace the 9 rgba(220,38,38,...) sites"
      - "Replace Modal.svelte's hardcoded overlay rgba() with var(--tr-overlay)"
      - "Replace the 4 auth-screen + ChartWidget hardcoded rgba() shadows with the appropriate --tr-elev-* level"
deferred: []
human_verification:
  - test: "Переключение темы (light/dark) без визуальных артефактов на 3-4 плотных экранах (Устройства, форма акта, Настройки) — DS-02"
    expected: "Нет флеша не той темы при загрузке, все поверхности стилизованы, текст читаем в обеих темах"
    why_human: "Требует живого рендера в браузере/Tauri webview; недоступно для grep-based верификации (D-09)"
  - test: "Отсутствие визуального сдвига вёрстки после space/radius миграции по значению — DS-04, сравнение до/после на тех же плотных экранах"
    expected: "Вёрстка визуально идентична прежней (кроме намеренной инверсии поверхностей --tr-bg/--tr-surface, D-10, которая не является дефектом)"
    why_human: "verify-value-map.mjs (даже с исправленной логикой, см. ниже) доказывает математическую сохранность px-значений на git-диффе, но не то, как каскад/layout это отрендерит визуально"
  - test: "Визуальная иерархия типографики — текст на правильном уровне 9-уровневой шкалы относительно соседних блоков — DS-03"
    expected: "Заголовки/тело/подписи визуально согласованы со шкалой"
    why_human: "Выбор роли статически проверен (check-tokens.mjs), но 'выглядит правильно' — вопрос визуального суждения"
  - test: "Точечная потеря контраста (белое-на-белом) от инверсии поверхностей — DS-02/D-11"
    expected: "Не должно быть невидимого текста/границ там, где старый код полагался на прежний порядок поверхностей"
    why_human: "Требует визуального обхода экранов; не обязательство этой фазы, но ожидаемый паттерн для UAT"
---

# Phase 23: Design Tokens Foundations Verification Report

**Phase Goal:** Интерфейс переходит на единый слой токенов `--tr-*` (поверхности, 5 уровней текста,
акцент с hover/active/soft, семантика с парами -soft/-text, нейтральная шкала n-0…n-950, 5 уровней
теней), типографика следует новой шкале из 9 уровней, отступы/радиусы мигрированы ПО ЗНАЧЕНИЮ
(вёрстка не сдвигается), устранены 2 бага неопределённых токенов.

**Verified:** 2026-07-17T23:10:00Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `_tokens.scss` defines single `--tr-*` layer: surfaces, 5 text levels, accent w/ hover/active/soft, semantic -soft/-text pairs, neutral n-0…n-950, 5 elevation levels, both light+dark themes | VERIFIED | Read full file — all listed tokens present in both `:root,[data-theme='light']` and `[data-theme='dark']` blocks; layout constants preserved verbatim (`--sidebar-width: 240px` etc.); `--shadow-elev-2-dark` absent (0 matches) |
| 2 | Typography follows new 9-level scale (composite + decomposed axes) | VERIFIED | 9 roles (display/h2/h3/subtitle/body/body-strong/label/caption/micro) + mono, each with `--tr-text-{role}` shorthand and `--tr-font-size-/-weight-/-line-height-{role}` decomposed axes, all present in `_tokens.scss` |
| 3 | All `--color-*`/`--shadow-*` call-sites in `ui/src` migrated to `--tr-*` (by role) | VERIFIED | `var(--color-` and `var(--shadow-` : 0 matches across all `.svelte`/`.scss` in `ui/src` (excl. `_tokens.scss`/`global.scss`, which legitimately define the new names) |
| 4 | All `--space-*`/`--radius-*` call-sites migrated to `--tr-*` by value (no layout shift) | VERIFIED (statically) | `var(--space-`/`var(--radius-` : 0 matches. Independently re-ran a corrected version of `verify-value-map.mjs` (fixing CR-01's single-token-per-line bug) against the full diff from `BASE_SHA 6425d30c` — 578 hunks checked, 0 real value-mismatches found. Spot-checked multiple multi-token lines (`CartridgeListRow.svelte`, `CartridgeFilters.svelte`) via `git log -p` against pre-migration values — confirmed pixel-exact preservation (e.g. `--space-sm` 8px → `--tr-space-xs` 8px). Visual "no shift" is UAT (see Human Verification) |
| 5 | All `--font-size-*`/`--font-weight-*`/`--line-height-*`/`--font-family-base` call-sites migrated to decomposed `--tr-*` axes by role | VERIFIED | 0 matches for all 4 old-name patterns across `ui/src` |
| 6 | 2 (+1 bonus) undefined-token bugs eliminated (QA-01): `--font-size-sm`, `--radius-lg`×4, bonus `--shadow-md`×3 | VERIFIED | `--tr-font-size-caption` confirmed in `PersonAutocomplete.svelte`; `--tr-radius-lg` confirmed in all 4 auth screens; `--tr-elev-2` confirmed in all 3 cartridge files; `git grep` for old names (`--font-size-sm`, `--radius-lg\b`, `--shadow-md`) returns 0 matches anywhere in `ui/src` |
| 7 | No hardcoded colors remain in components (DS-01, REQUIREMENTS.md: "захардкоженных цветов в компонентах не остаётся") | **FAILED (partial)** | 0 hex literals confirmed (`check-tokens.mjs --rules=2` PASS), but 17 hardcoded `rgba()` literals remain across 14 files, invisible to the gate (hex-only regex). Includes Modal.svelte's overlay not using the purpose-built `--tr-overlay` token, and a mismatched-red focus ring duplicated in 9 files. See Gaps below |

**Score:** 6/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `ui/src/styles/_tokens.scss` | Single `--tr-*` source of truth (color, spacing, radius, elevation, typography, layout constants) | VERIFIED | Read in full — matches UI-SPEC structure exactly, values copied verbatim per plan design |
| `ui/src/styles/global.scss` | body/focus-ring/scrollbar/.skip-link on `--tr-*`, plus `.tr-mono` | VERIFIED | All old names removed; `.tr-mono` defined; `@use './tokens'` single connection point intact; `*:focus-visible` scope unchanged |
| `ui/scripts/check-tokens.mjs` | Permanent CI gate: 3 rules (old-name / hex-in-style / closed-world) | VERIFIED (with known blind spot) | `node ui/scripts/check-tokens.mjs` → PASS — 0 нарушений (exit 0), all 3 rules confirmed independently. **Known gap:** Rule 2 only detects hex, not rgba()/hsl() (WR-03, confirmed) |
| `ui/scripts/verify-value-map.mjs` | One-shot value-preserving verifier for space/radius sweep | VERIFIED (tool bug found, but re-verification shows no missed real violations) | CR-01 confirmed: regex captures only first `--space-*`/`--radius-*` token per line, silently dropping subsequent tokens on multi-token lines (e.g. `padding: var(--x) var(--y);`). Reproduced the bug. However, independently re-ran a corrected extraction (all tokens per line) against the full 578-hunk diff and found 0 real violations — the underlying migration itself is not shown to be broken, only the diagnostic tool undercounted what it checked |
| `ui/package.json` lint wiring | `check-tokens.mjs` embedded in `pnpm lint` via `&&` | VERIFIED | `"lint": "eslint . --ext .ts,.svelte && prettier --check . && node scripts/check-tokens.mjs"` confirmed; `pnpm lint` exits 0 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `global.scss` | `_tokens.scss` | `@use './tokens'` | WIRED | Single connection point confirmed, unchanged from pre-phase |
| `ui/src/**/*.svelte <style>` | `_tokens.scss` | `var(--tr-*)` refs | WIRED | 0 remaining old-name refs of any of the 6 migrated families; closed-world rule 3 confirms every used `--tr-*` name is defined |
| `ActItemsTable.svelte`, `ActDetail.svelte`, `ActListRow.svelte`, `DeviceListRow.svelte`, `PrinterDetail.svelte`, `DocumentAcceptanceModal.svelte`, `ActFormItemsTable.svelte`, `ReturnItemsTable.svelte` | `global.scss` `.tr-mono` | `class="tr-mono"` | WIRED | 14 `class="tr-mono"` sites confirmed across 8 files (plan targeted 7 files/9 sites; `ReturnModal.svelte`'s target moved to child `ReturnItemsTable.svelte` per documented architectural deviation — functional goal (mono display of inv./serial numbers) achieved) |
| `ui/package.json` | `ui/scripts/check-tokens.mjs` | `"lint"` script `&&` chain | WIRED | Confirmed in package.json content |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|--------------|------------|--------------|--------|----------|
| DS-01 | 23-01, 23-03 | Единый слой токенов `--tr-*`; захардкоженных цветов не остаётся | **PARTIAL** | Token layer + call-site rename fully done (VERIFIED). "No hardcoded colors" sub-clause not fully met — 17 rgba() literals remain, invisible to the gate (see Gaps) |
| DS-02 | 23-01 (token layer), all sweep plans | Темы визуально согласованы, переключаются без артефактов | STATICALLY VERIFIED / NEEDS HUMAN | Both themes fully defined symmetrically in `_tokens.scss`; visual consistency requires human UAT (surfaced in hand-off checklist by 23-06-SUMMARY) |
| DS-03 | 23-01, 23-05 | Типографика на 9-уровневой шкале; идентификаторы моноширинным | VERIFIED (statically) + NEEDS HUMAN (visual hierarchy) | All call-sites migrated; `.tr-mono` applied to inv./serial/act-number sites; visual "correct level" needs human check |
| DS-04 | 23-01, 23-04 | Отступы/радиусы по значению, вёрстка не сдвигается | VERIFIED (statically, independently re-checked) + NEEDS HUMAN (visual) | 0 call-sites left, value-map re-verified with corrected logic (578 hunks, 0 real violations); visual layout-shift confirmation is UAT |
| QA-01 | 23-01 (fix in later plans), 23-03, 23-04, 23-05, 23-06 | Устранены неопределённые токены `--font-size-sm`, `--radius-lg`, (bonus `--shadow-md`) | VERIFIED | All 3 confirmed resolved and re-confirmed with 0 remaining old-name references |

No orphaned requirements — DS-01..04 and QA-01 all present in at least one plan's `requirements:` frontmatter, matching the phase requirement IDs given in the verification brief.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `ui/scripts/verify-value-map.mjs` | 76-97 | Regex only matches first `--space-*`/`--radius-*` token per line (CR-01, confirmed) | Warning | One-shot tool, not the permanent gate; independently re-verified 0 real violations exist despite the tool bug, so no false "PASS" concealed a real regression in this instance |
| `ui/scripts/check-tokens.mjs` | Rule 2 (HEX_RE) | No detection for `rgba()`/`hsl()`/`hsla()` literals (WR-03, confirmed) | Warning | Permanent gate blind spot — confirmed already-realized instances (see gap above); will not catch future regressions of this class |
| `ui/src/lib/components/Modal.svelte` | 83, 91 | Hardcoded `rgba(0,0,0,0.4)`/`rgba(0,0,0,0.6)` overlay instead of `var(--tr-overlay)` | Warning | Most widely-reused overlay component bypasses the token specifically defined for it |
| 9 files (PersonAutocomplete, ActFormItemsTable×2, Button, Input, DatePicker, LocationAutocomplete, DeviceAutocompleteField, ModelFormModal) | various | Duplicated `rgba(220, 38, 38, ...)` "invalid" focus ring, mismatched with `--tr-danger` token value | Warning | Visual inconsistency; un-tokenized magic number |
| `ui/src/features/acts/ActFormItemsTable.svelte` | 141-157 | `removeRow()` doesn't reindex `debounceTimers` (CR-02, pre-existing since Phase 18) | Info (out of phase scope) | Real data-integrity bug but not introduced by / not part of Phase 23's DS-01..04/QA-01 scope — noted for awareness, not counted against this phase's goal |
| `ui/src/features/acts/PdfPreviewModal.svelte` | 194-198 | Predictable temp filename, no cleanup (WR-04, pre-existing pattern) | Info (out of phase scope) | Unrelated to design tokens; noted for awareness |
| No TBD/FIXME/XXX debt markers found in any Phase 23 key-file | — | — | — | Debt-marker gate clean |

### Human Verification Required

See `human_verification` in frontmatter above — 4 items (DS-02 theme switching, DS-04 visual layout stability, DS-03 visual hierarchy, DS-02/D-11 contrast check), all already explicitly identified and handed off by the phase's own 23-06-SUMMARY.md hand-off checklist. Confirmed these are genuinely non-automatable (require live browser/Tauri render).

### Gaps Summary

The mechanical token-rename migration (the bulk of the phase's deliverable) is complete and correctly
verified: all 6 old token-name families (`--color-*`, `--space-*`, `--radius-*`, `--font-size-*`,
`--font-weight-*`, `--line-height-*`, `--font-family-base`, `--shadow-*`) are 100% migrated to
`--tr-*` across `ui/src`, the permanent `check-tokens.mjs` gate passes clean, `pnpm lint`/`svelte-check`/
`pnpm build` are all green, and all 3 QA-01 undefined-token bugs (plus the bonus `--shadow-md` find)
are confirmed fixed with zero remaining old-name references anywhere in the tree.

One real gap remains, identified by code review and independently confirmed here: **DS-01's explicit
"no hardcoded colors remain in components" clause is not fully met.** 17 hardcoded `rgba()` color/shadow
literals persist in 14 files, invisible to the phase's own permanent CI gate (`check-tokens.mjs` Rule 2
only detects hex, not rgba()/hsl()). Two instances are materially significant, not just cosmetic:
Modal.svelte's overlay background bypasses the `--tr-overlay` token defined specifically for that
purpose, and a duplicated "invalid" focus-ring color across 9 files doesn't match the `--tr-danger`
token value. This was a scoping choice baked into plan 23-03's must-haves (hex-only, matching
research's `HEX_RE` design) — narrower than the roadmap-level DS-01 promise — not a mistake introduced
by any executor deviation.

This looks like a scoping gap rather than a deliberate accepted trade-off, so I am not treating it as
an override candidate. It is small in surface area (17 sites, 2 recurring patterns) and does not block
the phase's primary mechanical deliverable, but it does mean DS-01 cannot be marked fully achieved as
literally worded in REQUIREMENTS.md.

**This looks like it could reasonably be deferred to Phase 24** (which rebuilds Button and Modal on the
new design system per its own goal), but Phase 24's stated goal/success-criteria do not explicitly
commit to "remove hardcoded color literals" — per the conservative Step 9b matching rule, I am not
auto-deferring it without clearer evidence, and am surfacing it as a gap for a human decision instead.

The `verify-value-map.mjs` regex bug (CR-01) is a tooling defect, not a goal-blocking gap — my
independent re-verification (corrected regex, same 578-hunk diff) found 0 real value-mismatches, so
the underlying space/radius migration's "by value" guarantee holds despite the tool's blind spot.
Recommend fixing the tool anyway since it may be reused/referenced in future phases with similar
migration risk.

---

*Verified: 2026-07-17T23:10:00Z*
*Verifier: Claude (gsd-verifier)*
