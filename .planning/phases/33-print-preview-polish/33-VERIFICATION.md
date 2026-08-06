---
phase: 33-print-preview-polish
status: passed
verified: 2026-08-05
requirements: [PRV-01, PRV-02, PRV-03]
verifier: orchestrator (evidence-based — live user UAT is the authoritative signal for visual/print fidelity)
score: 4/4 success criteria verified
---

# Phase 33 — Verification (goal-backward)

**Goal (ROADMAP):** Предпросмотр печати документов (Акты, Приёмка/DocumentAcceptance, Отчёты)
выглядит как «вордовский» предпросмотр — лист A4 на подложке с полями — и один-в-один совпадает
с тем, что уходит на печать.

**Verdict: PASSED** — all 4 Success Criteria demonstrably true; PRV-01/02/03 covered.

## Why this file was written after the fact

The phase's automated gates were all green at execution time, but the goal here is **visual and
print fidelity** — a property no string assertion in this codebase can prove (a recorded project
lesson: text-extraction tests cannot see overlap, overflow, or a wrong margin). The authoritative
signal is therefore a real render in front of a human, which arrived across a debug round and six
same-day quick-fixes rather than in a single scheduled verification pass. `/gsd-verify-work 33`
was deliberately not run afterwards: it would have re-asked a checklist the user had already
answered by using the product. The sign-off is recorded in
`33-VALIDATION.md` § «Manual UAT Sign-Off 2026-08-05»; this file restates it in the shape the
milestone audit expects.

## Success Criteria → Evidence

| # | Criterion | Verdict | Evidence |
|---|-----------|---------|----------|
| 1 | Документ отображается как лист A4 над визуально отделённой сероватой подложкой | ✅ | Живой UAT в desktop-режиме и в LAN-браузере, обе темы. Реализация — `PdfPreviewModal.svelte`: backdrop `--tr-surface-sunken`, лист без рамки, только `box-shadow` (D-08/D-09) |
| 2 | Лист имеет видимые внутренние поля, соответствующие реальным полям печати | ✅ | Живой UAT. Структурно подкреплено `html_page_parity.rs` (1/1 green): все три шаблона объявляют идентичный `@page { size; margin }` (D-13) |
| 3 | Экранное превью совпадает с печатью (WYSIWYG), единый источник стилей | ✅ | Пользователь напечатал **двухстраничный** акт — число страниц и точки разрыва совпали с превью. Оба пути печати (desktop temp-`.html` и LAN top-level) прогоняют тот же Paged.js и ждут завершения пагинации, а не события `load` (D-06/C-03) |
| 4 | Поведение одинаково для всех документов общей модалки (Акты, Приёмка, Отчёты) | ✅ | Один компонент `PdfPreviewModal.svelte` обслуживает все три; `html_page_parity.rs` пиннит равенство `@page` у `act_handover.html`, `act_acceptance.html`, `report.html` |

## Requirements

| REQ | Status | Evidence |
|-----|--------|----------|
| PRV-01 | satisfied | Критерии 1 и 4 выше; `33-01`…`33-03` |
| PRV-02 | satisfied | Критерий 2; `html_page_parity.rs` |
| PRV-03 | satisfied | Критерий 3; `33-04` + быстрофиксы `260805-*` |

## Automated gates (re-run during the 2026-08-05 validation audit)

| Gate | Result |
|------|--------|
| `pnpm --dir ui svelte-check` | 0 errors (48 pre-existing warnings) |
| `pnpm --dir ui lint` (вкл. `check-pagedjs-csp-hash`, `check-print-isolation`) | pass |
| `cargo test -p trackly-app --test html_page_parity` | 1/1 |
| `cargo test -p trackly-app --test security_headers` | 4/4 |

## Honest limits of this verification

- **Visual fidelity rests on human observation, not automation.** There is no frontend rendering
  harness in this project, and a synthetic Playwright/Chromium harness is explicitly not accepted
  as verification here — the app runs in WKWebView/WebView2.
- **Multi-page was verified once**, on a two-page act. Deeper documents (many pages, long reports)
  remain unobserved. Prior to 2026-08-05 the project had never observed multi-page preview working
  at all, so this is the first datapoint, not a broad sample.
- **CSP-in-LAN-mode is inferred, not directly observed:** the quick-fixes `har`/`ifj`/`jwf` were
  developed and checked in a real LAN browser, so Paged.js demonstrably loaded there, but the
  browser console was not audited specifically for CSP violations.
- **`printViaSystemBrowser` (desktop) is not covered by `check-print-isolation.mjs`** — deliberate:
  it ships no app stylesheet, so the cascade-leak defect class cannot occur on that path.

## Post-phase work folded into this verification

Seven fix commits landed after the plans completed and are part of what was verified:
`9a66ff8` (pagination actually runs), `c77ab6c` (`260805-edd` — stylesheet argument shape),
`4b7f96f`/`8a06587` (`gdz` — swallowed error, print-root geometry), `2f296b2`/`3162320`/`1f868ad`
(`har`/`ifj`/`jwf` — app cascade leaking into LAN print). The last three are one defect class that
recurred three times; it is now guarded by `ui/scripts/check-print-isolation.mjs`
(commit `d66316b`), whose three invariants were mutation-tested in both directions.
