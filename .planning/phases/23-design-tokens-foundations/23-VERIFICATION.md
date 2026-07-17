---
phase: 23-design-tokens-foundations
verified: 2026-07-18T02:15:00Z
status: human_needed
score: 7/7 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 6/7
  gaps_closed:
    - "Захардкоженных цветов в компонентах не остаётся (DS-01) — 17 rgba() literals across 14 files migrated to --tr-overlay/--tr-danger-ring/--tr-elev-*; check-tokens.mjs Rule 4 (new, permanent) now detects rgba()/rgb()/hsl()/hsla() in <style> blocks and is enabled by default; 0 violations across all 4 rules on ui/src"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Переключение темы (light/dark) без визуальных артефактов на 3-4 плотных экранах (Устройства, форма акта, Настройки)"
    expected: "Нет флеша не той темы при загрузке, все поверхности стилизованы, текст читаем в обеих темах"
    why_human: "Требует живого рендера в браузере/Tauri webview; недоступно для grep-based верификации (D-09) — carried over unchanged from initial verification, this gap-closure round did not touch DS-02"
  - test: "Отсутствие визуального сдвига вёрстки после space/radius миграции по значению — сравнение до/после на тех же плотных экранах"
    expected: "Вёрстка визуально идентична прежней (кроме намеренной инверсии поверхностей --tr-bg/--tr-surface, D-10, которая не является дефектом)"
    why_human: "verify-value-map.mjs (CR-01 now fixed, re-confirmed in this round) proves pixel-value preservation on the git diff, but not how cascade/layout renders it visually"
  - test: "Визуальная иерархия типографики — текст на правильном уровне 9-уровневой шкалы относительно соседних блоков"
    expected: "Заголовки/тело/подписи визуально согласованы со шкалой"
    why_human: "Роль статически проверена; 'выглядит правильно' — вопрос визуального суждения"
  - test: "Точечная потеря контраста (белое-на-белом) от инверсии поверхностей"
    expected: "Не должно быть невидимого текста/границ там, где старый код полагался на прежний порядок поверхностей"
    why_human: "Требует визуального обхода экранов; ожидаемый паттерн для UAT"
  - test: "Button.svelte danger-ring alpha 0.3→0.2 (WR-01-sanctioned micro visual touch from this gap-closure round) — confirm the slightly lighter focus ring on the destructive button reads acceptably next to the other 8 now-identical 0.2 danger-ring sites"
    expected: "No perceptible regression; ring is marginally more subtle but still clearly visible against both themes' backgrounds"
    why_human: "New in this round — a real, if intentionally sanctioned (WR-01), visual pixel change; disclosed in 23-08-SUMMARY.md as a Phase-24 handoff note, not previously covered by any prior human-verification item"
---

# Phase 23: Design Tokens Foundations Verification Report

**Phase Goal:** Интерфейс переходит на единый слой токенов `--tr-*` (поверхности, 5 уровней текста,
акцент hover/active/soft, семантика -soft/-text, нейтрали n-0…n-950, 5 уровней теней), типографика
по новой шкале из 9 уровней, отступы/радиусы мигрированы ПО ЗНАЧЕНИЮ (вёрстка не сдвигается),
устранены 2 бага неопределённых токенов.

**Verified:** 2026-07-18T02:15:00Z
**Status:** human_needed
**Re-verification:** Yes — after gap closure (plans 23-07, 23-08)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `_tokens.scss` defines single `--tr-*` layer: surfaces, 5 text levels, accent w/ hover/active/soft, semantic -soft/-text pairs, neutral n-0…n-950, 5 elevation levels, both light+dark themes | VERIFIED (regression check) | `check-tokens.mjs` Rule 1 (old-name gate) + Rule 3 (closed-world gate) both re-ran clean in the combined 0-violation result; `_tokens.scss` still contains all listed families plus the new `--tr-danger-ring` (light+dark) added by plan 23-07 |
| 2 | Typography follows new 9-level scale (composite + decomposed axes) | VERIFIED (regression check) | No files touched by gap closure affect typography tokens; `check-tokens.mjs` Rule 1/3 clean, unchanged from prior round |
| 3 | All `--color-*`/`--shadow-*` call-sites in `ui/src` migrated to `--tr-*` (by role) | VERIFIED (regression check) | `var(--color-`/`var(--shadow-` : 0 matches, unchanged |
| 4 | All `--space-*`/`--radius-*` call-sites migrated to `--tr-*` by value (no layout shift) | VERIFIED (statically, tooling now fixed) | 0 old-name matches; `verify-value-map.mjs` CR-01 fixed this round (tokensOnSide() now captures every token on a multi-token line) — independently re-ran `node scripts/verify-value-map.mjs HEAD` (trivial diff, PASS 0/0) and confirmed `tokensOnSide()` against the historical multi-token reproducer (commit 16244e2): both `--space-sm`/`--space-md` and `--tr-space-xs`/`--tr-space-md` captured, matching the plan's documented expectation. Visual "no shift" remains UAT |
| 5 | All `--font-size-*`/`--font-weight-*`/`--line-height-*`/`--font-family-base` call-sites migrated to decomposed `--tr-*` axes by role | VERIFIED (regression check) | Unaffected by gap closure, 0 old-name matches persists |
| 6 | 2 (+1 bonus) undefined-token bugs eliminated (QA-01) | VERIFIED (regression check) | Unaffected by gap closure; `check-tokens.mjs` Rule 3 (closed-world) still clean |
| 7 | No hardcoded colors remain in components (DS-01, REQUIREMENTS.md: "захардкоженных цветов в компонентах не остаётся") | **VERIFIED (gap closed)** | Independently re-ran `node ui/scripts/check-tokens.mjs` (no `--rules` flag, all 4 rules including the new Rule 4) → `PASS — 0 нарушений`, exit 0. Independently grepped `rgba(\|rgb(\|hsla(\|hsl(` inside every `.svelte` file's `<style>` block in `ui/src` → 0 matches. `git grep -c 'rgba(220, 38, 38' ui/src` → 0. Modal.svelte's overlay confirmed on `var(--tr-overlay)` (read lines 60-110, the redundant `[data-theme='dark']` override is gone as claimed). `--tr-danger-ring` confirmed defined in both theme blocks of `_tokens.scss` (`rgba(207, 59, 59, 0.2)` light / `rgba(242, 101, 101, 0.2)` dark). Button.svelte confirmed on `var(--tr-danger-ring)` (was `rgba(220, 38, 38, 0.3)`) |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `ui/src/styles/_tokens.scss` | Single `--tr-*` source of truth, now incl. `--tr-danger-ring` | VERIFIED | Read relevant sections — `--tr-danger-ring`/`--tr-overlay` both present in `:root,[data-theme='light']` and `[data-theme='dark']` blocks |
| `ui/scripts/check-tokens.mjs` | Permanent CI gate: 4 rules (old-name / hex-in-style / closed-world / color-func-in-style), all enabled by default | VERIFIED | Read full file: `Rule 4` (`checkColorFunctionsInStyle`) implemented, mirrors Rule 2's structure exactly as the plan specified; `parseArgs()` default `args.rules = [1, 2, 3, 4]` confirmed; `pnpm lint` (which calls the script with no `--rules` flag) independently re-run → exit 0, "PASS — 0 нарушений" |
| `ui/scripts/verify-value-map.mjs` | CR-01 fixed (all tokens per line, not just first); named exports importable without side effects | VERIFIED | `tokensOnSide` + `fileURLToPath(import.meta.url)) main()` guard + `export { tokensOnSide, checkHunk }` all present (grep confirmed, 1 match each). Independently imported `tokensOnSide` and re-ran the exact historical multi-token reproducer from the plan — both sides correctly return 2 tokens each, not 1. CLI path (`node scripts/verify-value-map.mjs HEAD`) still works, exit 0 |
| 14 component files (Modal, Button, Input, DatePicker, LocationAutocomplete, PersonAutocomplete, DeviceAutocompleteField, ActFormItemsTable, ModelFormModal, ChartWidget, LoginPage, FirstRunWizard, PendingScreen, BlockedScreen) | rgba() literals replaced with `--tr-*` tokens | VERIFIED | `git diff --stat` on the gap-closure commit range confirms exactly these 17 files changed (14 components + 3 tooling/token files), matching both plans' declared `files_modified` scope exactly — no scope creep |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `ui/package.json` `"lint"` script | `check-tokens.mjs` Rule 4 | call with no `--rules` flag → default `[1,2,3,4]` | WIRED | `pnpm lint` independently re-run, exit 0, `[check-tokens] PASS — 0 нарушений` printed as part of the lint chain |
| 8 danger-ring files | `_tokens.scss` `--tr-danger-ring` | `var(--tr-danger-ring)` | WIRED | Confirmed via `check-tokens.mjs` Rule 4 (0 violations = no unresolved rgba literals) + Rule 3 closed-world (0 violations = no reference to an undefined token name) |
| `Modal.svelte` | `_tokens.scss` `--tr-overlay` | `var(--tr-overlay)` | WIRED | Read Modal.svelte lines 60-110 directly — `background: var(--tr-overlay);` confirmed, no residual `[data-theme='dark']` override block remains (grep for `data-theme='dark'` inside Modal.svelte returns 0 matches, matching the claimed cleanup) |

### Data-Flow Trace (Level 4)

Not applicable — this phase's deliverable is static CSS custom-property definitions and lint tooling, not dynamic data rendering. No data-flow trace required.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Permanent gate (all 4 rules) is 0-violation on current tree | `node ui/scripts/check-tokens.mjs` | exit 0, "PASS — 0 нарушений" | PASS |
| Rule 4 alone (in isolation) is real, not a no-op | Source read of `checkColorFunctionsInStyle()` — same structure as Rule 2, wired into `main()` under `args.rules.includes(4)` | Rule 4 fired 17 times on the pre-migration tree per 23-07-SUMMARY; now 0 on the post-migration tree — confirms the rule is a real gate, not silently disabled | PASS |
| No `rgba(220, 38, 38` (mismatched invalid-red) anywhere in `ui/src` | `git grep -c 'rgba(220, 38, 38' ui/src \| wc -l` | `0` | PASS |
| `verify-value-map.mjs` CR-01 fix captures both tokens on a multi-token line | inline import + historical reproducer (commit 16244e2 pattern) | `removed ["--space-sm","--space-md"]`, `added ["--tr-space-xs","--tr-space-md"]` | PASS |
| `pnpm lint` | `cd ui && pnpm lint` | exit 0 (eslint clean, prettier clean, check-tokens.mjs clean) | PASS |
| `pnpm svelte-check` | `cd ui && pnpm svelte-check` | 242 files, 0 ERRORS, 48 WARNINGS (matches SUMMARY's claimed unchanged baseline) | PASS |
| `pnpm build` | `cd ui && pnpm build` | exit 0, dist emitted, only pre-existing unrelated warnings (unused CSS selector in ActFormItemsTable, dynamic-import chunking notice) | PASS |

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes declared or found for this phase. `check-tokens.mjs` and `verify-value-map.mjs` function as this phase's equivalent automated gates and were executed directly above (Behavioral Spot-Checks), not skipped.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|--------------|------------|--------------|--------|----------|
| DS-01 | 23-01, 23-03, 23-07, 23-08 | Единый слой токенов `--tr-*`; захардкоженных цветов не остаётся | **SATISFIED** (was PARTIAL) | Both the hex sub-clause (Rule 2, prior round) and the rgba/hsl sub-clause (Rule 4, this round) are now enforced by a 0-violation permanent gate. Gap fully closed |
| DS-02 | 23-01, all sweep plans | Темы визуально согласованы, переключаются без артефактов | STATICALLY VERIFIED / NEEDS HUMAN (unchanged) | Not in scope of this gap-closure round; still requires live UAT |
| DS-03 | 23-01, 23-05 | Типографика на 9-уровневой шкале; идентификаторы моноширинным | STATICALLY VERIFIED / NEEDS HUMAN (unchanged) | Not in scope of this gap-closure round |
| DS-04 | 23-01, 23-04, 23-07 | Отступы/радиусы по значению, вёрстка не сдвигается | VERIFIED (statically, tooling bug now fixed) / NEEDS HUMAN (visual) | CR-01 fix independently re-confirmed correct on both the historical reproducer and the live CLI path; underlying migration was already shown correct in the prior round (0 real violations even with the buggy tool) — this round removes the tooling doubt entirely. Visual layout-shift confirmation remains UAT |
| QA-01 | 23-01, 23-03, 23-04, 23-05, 23-06 | Устранены неопределённые токены | VERIFIED (unchanged) | Regression-checked, still 0 old-name references |

No orphaned requirements — DS-01..04 and QA-01 all present in at least one plan's `requirements:` frontmatter across the full phase (23-01 through 23-08), matching the phase requirement IDs given in the verification brief.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| None in the gap-closure diff scope | — | `grep -n -E "TBD\|FIXME\|XXX"` across all 17 files touched by 23-07/23-08 → 0 matches | — | Debt-marker gate clean |
| `ui/src/lib/components/Button.svelte` | ~109 | Intentional alpha value change (0.3 → 0.2) on danger-ring, disclosed as WR-01-sanctioned in 23-08-SUMMARY.md with an explicit Phase-24 handoff note | Info | Real, if small, visual pixel change — not silent scope creep since it is explicitly documented as a deliberate trade-off with rationale (converging 9 duplicated sites to one canonical alpha value) and flagged for the next phase's planner. Surfaced as a new human-verification item below rather than treated as a silent regression |
| Prior round's `verify-value-map.mjs` CR-01 regex bug | — | Fixed this round (`tokensOnSide()`) | — | No longer an anti-pattern; independently re-verified via import + historical reproducer |
| Prior round's `check-tokens.mjs` Rule 2 blind spot for rgba/hsl | — | Fixed this round (Rule 4) | — | No longer an anti-pattern; independently re-verified via full-tree 0-violation run |

### Human Verification Required

5 items require live browser/Tauri render — 4 carried over unchanged from the initial verification round (none of DS-02/DS-03's visual truths were touched by this gap-closure round, which was scoped exclusively to DS-01's rgba() literals) plus 1 new item introduced by this round's WR-01-sanctioned Button.svelte alpha change. See `human_verification` in frontmatter above for full detail.

### Gaps Summary

**The single gap from the initial verification round is closed.** DS-01's literal "захардкоженных цветов в компонентах не остаётся" clause is now fully met: 17 hardcoded `rgba()` literals across 14 files have been migrated to `--tr-overlay`, `--tr-danger-ring` (new token, added this round), and `--tr-elev-*`. The phase's permanent CI gate (`check-tokens.mjs`) gained a new Rule 4 that detects `rgba()`/`rgb()`/`hsl()`/`hsla()` inside `<style>` blocks, is enabled by default, and independently re-confirmed to pass 0-violation on the current tree — closing the blind spot permanently, not just for the 17 known sites. A pre-existing tooling bug (CR-01 in `verify-value-map.mjs`, which silently dropped the second-plus token on multi-token CSS lines) was also fixed and independently re-verified against the historical reproducer that exposed it.

Independent verification (not just trusting the SUMMARYs) confirms:
- `node ui/scripts/check-tokens.mjs` (default args, all 4 rules) → exit 0, 0 violations, re-run directly
- `git grep -c 'rgba(220, 38, 38' ui/src` → 0
- Direct grep for `rgba(`/`rgb(`/`hsla(`/`hsl(` in every `.svelte` file's `<style>` block across `ui/src` → 0 matches
- `pnpm lint` / `pnpm svelte-check` / `pnpm build` → all exit 0, warning baselines unchanged from pre-gap-closure state
- Git diff scope for the gap-closure commit range (`659f10d..HEAD`) touches exactly the 17 files declared across both plans' `files_modified` frontmatter — no undisclosed scope creep
- Modal.svelte's overlay and the 9 danger-ring sites read directly from source, confirmed on `var(--tr-overlay)`/`var(--tr-danger-ring)` respectively
- No TBD/FIXME/XXX debt markers introduced in any of the 17 touched files

**One disclosed, intentional visual change** (Button.svelte's danger-ring alpha 0.3 → 0.2) is present, confirmed as WR-01-sanctioned in 23-08-SUMMARY.md with an explicit rationale (canonicalizing 9 duplicated focus-ring sites to a single alpha value) and an explicit Phase-24 handoff note (Button.svelte's full visual redesign is reserved for Phase 24 per CONTEXT.md; this note ensures that phase's planner isn't surprised by an already-touched file). This is not silent scope creep — it is disclosed, small (a single alpha channel on one pseudo-class), and consistent with the phase's own goal of eliminating divergent hardcoded color values. It is surfaced as a new human-verification item (not a gap) because it is a genuine, if sanctioned, pixel-level visual change that hasn't been eyeballed live yet.

**Overall status is `human_needed`, not `passed`**, because 5 items require live browser/Tauri rendering that cannot be verified via static analysis: 4 carried over unchanged from the initial round (theme-switch artifact check, layout-shift visual comparison, typography visual hierarchy, surface-inversion contrast spot-check) plus 1 new item for the Button.svelte alpha change. None of these are gaps — they are expected, pre-identified UAT items per the phase's own 23-06-SUMMARY.md hand-off checklist and this round's disclosed WR-01 change. The mechanical/gate-level deliverable of the phase (100% of DS-01..04 + QA-01's statically-verifiable clauses) is now fully and correctly closed.

---

*Verified: 2026-07-18T02:15:00Z*
*Verifier: Claude (gsd-verifier)*
