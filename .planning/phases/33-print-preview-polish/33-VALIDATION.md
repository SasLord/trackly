---
phase: 33
slug: print-preview-polish
status: audited
nyquist_compliant: true
wave_0_complete: true
created: 2026-08-04
audited: 2026-08-05
---

# Phase 33 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust: `cargo test` (existing, `crates/trackly-app/tests/`). Frontend: **none** — `ui/` has no Vitest/Jest (verified in RESEARCH.md); existing gates are `svelte-check` + zero-dependency Node lint scripts in `ui/scripts/`. |
| **Config file** | Rust: `crates/trackly-app/Cargo.toml`. Frontend: `ui/package.json` (`svelte-check`, `lint`, `build` scripts — there is **no** aggregate `check` script; confirmed during 33-01 execution). |
| **Quick run command** | `pnpm --dir ui svelte-check && pnpm --dir ui lint` (нет агрегирующего скрипта `check`) |
| **Frontend structural gates** | `ui/scripts/*.mjs`, zero-dependency Node, все встроены в `pnpm lint`: `check-tokens`, `check-contrast`, `check-focus-outline`, `check-pagedjs-csp-hash` (33-02), `check-print-isolation` (добавлен аудитом 2026-08-05) |
| **Full suite command** | `cargo test -p trackly-app` (needs `TRACKLY_AD_MOCK` / `TRACKLY_SNMP_MOCK` env + a real `pnpm --dir ui build` output in `ui/dist`) |
| **Estimated runtime** | ~30 s frontend, ~2–4 min Rust |

**Planner note:** do NOT introduce a frontend test framework for this phase unless a plan
explicitly justifies it — that is a stack decision beyond the phase scope. Prefer the existing
Rust integration-test location (precedent: `crates/trackly-app/tests/html_act_render.rs`) for
D-13's structural `@page`-parity assertion, since the artifact under test is a Rust-crate asset.

---

## Sampling Rate

- **After every task commit:** `pnpm --dir ui svelte-check && pnpm --dir ui lint`
- **After every plan wave:** `cargo test -p trackly-app` (with mock env vars set)
- **Before `/gsd-verify-work`:** both green
- **Max feedback latency:** 240 seconds

---

## Per-Task Verification Map

Verified by gsd-plan-checker (2026-08-04): **every task across all four plans carries an
`<automated>` verify command**, no watch-mode flags, no E2E-latency issues.

Statuses below are the **real post-execution results** measured during the 2026-08-05 audit
(all four command families re-run against the current tree), not the pre-execution projection.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 33-01-* | 01 | 1 | PRV-01 | — | N/A | typecheck/build | `pnpm --dir ui svelte-check && pnpm --dir ui lint` / `pnpm --dir ui build` | ✅ | ✅ green (0 errors, 48 pre-existing warnings) |
| 33-02-* | 02 | 2 | PRV-01, PRV-03 | D-14 (CSP `script-src` hash source) | Inline preview bootstrap runs under a hash source, never `'unsafe-inline'`; hash drift is caught by a lint gate rather than silently disabling preview | integration | `node ui/scripts/check-pagedjs-csp-hash.mjs && pnpm --dir ui build && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test security_headers` | ✅ | ✅ green (4/4 tests) |
| 33-02-* (D-13) | 02 | 2 | PRV-02 | — | N/A | integration | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test html_page_parity` | ✅ | ✅ green (1/1 test) |
| 33-03-* | 03 | 2 | PRV-01, PRV-02 | — | Preview iframe stays `sandbox="allow-scripts"` without `allow-same-origin`; bridge validates `event.source`, not `event.origin` | typecheck | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ | ✅ green |
| 33-04-* | 04 | 3 | PRV-03 | — | N/A | typecheck + manual | `pnpm --dir ui svelte-check && pnpm --dir ui lint` + Manual-Only table below | ✅ | ✅ green |
| **quick 260805-edd** | — | post | PRV-03 | — | N/A | typecheck | `pnpm --dir ui svelte-check` — форма аргумента `stylesheets` (объект, не строка) теперь **типизирована** в `ui/src/pagedjs.d.ts`, откат к строке = ошибка типов | ✅ | ✅ green |
| **quick 260805-ifj/-har/-jwf** | — | post | PRV-03 | — | Каскад приложения (`line-height`/`letter-spacing`/`word-spacing`/`background`) не протекает в LAN-печать; вставленные Paged.js стили шаблона не остаются в документе приложения после печати | structural | `node ui/scripts/check-print-isolation.mjs` (INV-1a/1b/1c/1d, INV-2) | ✅ | ✅ green |
| **quick 260805-gdz-02** | — | post | PRV-01 | — | `#act-print-root` сохраняет реальную геометрию во время пагинации (скрыт `position:absolute` + отрицательный `left`, **не** `display:none`) | structural | `node ui/scripts/check-print-isolation.mjs` (INV-3a/3b/3c) | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*
*File Exists: ❌ W0 = the test file is created by the plan itself, not a pre-existing fixture.*

### Известные границы `check-print-isolation.mjs`

Это **структурный гейт по исходнику, а не runtime-тест**. Он доказывает, что выстраданные
объявления всё ещё в коде, и не доказывает, что каскад реально нейтрализован в живом
WKWebView/WebView2 — фидельность рендера остаётся в Manual-Only ниже.

- Покрывает **только `printViaTopLevel` (LAN)**. `printViaSystemBrowser` (desktop, временный
  `.html`) намеренно не покрыт: он не тащит с собой стили приложения, этот класс дефекта там
  невозможен. Если будущая правка заставит desktop-путь рендерить в DOM приложения, гейт
  этого не заметит.
- INV-1c самоотключается, если идентификатор аргумента `stylesheets` не удаётся разрешить
  статически (выбрано осознанно — лучше пропустить, чем дать ложное падение).
- Ловит **удаление** инварианта, не семантически-эквивалентную неверную переписку (например,
  сброс `line-height` на обёртке внутри print-root вместо самого root пройдёт проверку).

---

## Wave 0 Requirements

- [x] No new test framework — existing Rust `cargo test` + `svelte-check` cover what is
      automatable here. *(Соблюдено: аудит 2026-08-05 тоже не вводил фреймворк — добавлен
      zero-dependency Node-скрипт в существующую цепочку `pnpm lint`.)*
- [x] `crates/trackly-app/tests/` — new integration test asserting all three shipped templates
      declare identical `@page` `size` and `margin` (D-13). *(`html_page_parity.rs`, 1/1 green.)*

---

## Manual-Only Verifications

Visual and print fidelity cannot be proven by text-extraction tests — a recorded project lesson.
These require a real render and human eyes.

**Статус: все строки подтверждены пользователем 2026-08-05** — живым UAT в ходе debug-раунда и
шести быстрофиксов того же дня (см. Sign-Off ниже). Инструкции сохранены как чек-лист для
будущих регрессий, но на сегодня ни одна строка не остаётся непроверенной.

| Behavior | Requirement | Why Manual | Test Instructions | Проверено |
|----------|-------------|------------|-------------------|-----------|
| Лист A4 на сероватой подложке, поля видны | PRV-01, PRV-02 | Визуальная композиция; никакой строковый assert её не докажет | Открыть предпросмотр акта в desktop-режиме и в LAN-браузере, сверить обе темы | ✅ 2026-08-05 |
| Превью совпадает с бумагой | PRV-03 | Требует реальной печати/Save-as-PDF при дефолтных настройках диалога | Напечатать длинный акт (N≥2 устройств) и длинный отчёт; сверить число страниц и точки разрыва с превью | ✅ 2026-08-05 |
| ~~**LAN-печать: поля и шрифты применились к `#act-print-root`**~~ **ЗАКРЫТО** | PRV-03 | ~~RESEARCH.md Open Question 2 — форма аргумента `Polisher.add()` не подтверждена~~ **Разрешено живым UAT + быстрофиксом `260805-edd`:** правильная форма — объект (`[{ 'act-preview.css': cssText }]`), не строка. Теперь зафиксировано типами в `ui/src/pagedjs.d.ts`, т.е. откат ловится `svelte-check` | — (проверять больше не нужно; регресс ловится автоматически) | ✅ 2026-08-05 |
| **Многостраничная пагинация превью (N≥2 страниц)** | PRV-01, PRV-02 | Требует реального многостраничного документа: пагинация, per-sheet тень и отсутствие reflow не доказываются строковым assert'ом. Фронтенд-харнесса рендера нет, а синтетический Playwright/Chromium харнесс в этом проекте не принимается как верификация (приложение работает в WKWebView/WebView2) | Открыть предпросмотр акта на **N≥2 устройств** (и длинный отчёт), сверить число листов и точки разрыва с печатью/Save-as-PDF | ✅ 2026-08-05 — **пользователь напечатал двухстраничный акт, результат корректный.** Это первое подтверждённое наблюдение работающего многостраничного рендера в проекте: до сегодняшнего дня все UAT шли на однолистовых актах (`.planning/debug/resolved/print-preview-always-degrades.md`, ~L1128, фиксировал это как полностью непроверенное) |
| Paged.js грузится в LAN-режиме (CSP) | PRV-01 | Проявляется только под реальным axum-сервером с CSP-заголовком, не в dev-сборке | `pnpm --dir ui build`, поднять server mode, открыть превью в браузере, проверить консоль на CSP-ошибки | ✅ 2026-08-05 (быстрофиксы `har`/`ifj`/`jwf` правились и проверялись именно в LAN-режиме) |
| Fit-to-width на узком окне | PRV-01 | Зависит от реальной ширины вьюпорта | Сузить окно/открыть в LAN-браузере на ноутбуке — горизонтального скролла быть не должно | ✅ 2026-08-05 |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or are listed under Manual-Only above
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 240s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-08-04 (gsd-plan-checker: 0 blockers, 3 warnings — all three applied)

---

## Validation Audit 2026-08-05

Повод: фаза закрыта живым UAT, но **после** закрытия прошли debug-раунд и шесть быстрофиксов
по пути печати (`260805-edd`, `-gdz-01`, `-gdz-02`, `-har`, `-ifj`, `-jwf`) — **без единого
регрессионного теста**. Три из них (`har`, `ifj`, `jwf`) — один и тот же класс дефекта:
каскад приложения протекает в вывод LAN-печати. Класс регрессировал трижды и не был защищён
ничем.

| Metric | Count |
|--------|-------|
| Gaps found | 4 |
| Resolved (автоматизировано) | 3 |
| Escalated (manual-only) | 1 |
| Manual-only rows closed | 1 |

| Gap | Требование | Итог |
|-----|-----------|------|
| G1 — нейтрализация утечки каскада в LAN-печать (`260805-ifj`, `-har`, `-jwf`) | PRV-03 | ✅ `check-print-isolation.mjs` INV-1a/1b/1c/1d |
| G2 — `previewer.polisher.destroy()` после печати (`260805-jwf`) | PRV-03 | ✅ INV-2 |
| G3 — `#act-print-root` не `display:none` во время пагинации (`260805-gdz-02`) | PRV-01 | ✅ INV-3a/3b/3c |
| G4 — многостраничное превью ни разу не наблюдалось работающим | PRV-01, PRV-02 | ⚠️ escalated → Manual-Only |

**Добавлено:** `ui/scripts/check-print-isolation.mjs` (zero-dependency Node, встроен последним
шагом в `pnpm --dir ui lint`). Скрипт бланкует комментарии перед матчингом — в файле есть
пояснительный комментарий, буквально содержащий текст `polisher.destroy()`, и наивный grep
проходил бы уже после удаления самого вызова.

**Доказательство двусторонности** (мутации применялись к копиям в scratchpad, реальный
`PdfPreviewModal.svelte` не изменялся — `git status --porcelain` по нему пуст):
13/13 мутаций дают exit 1 с правильным инвариантом и id быстрофикса; 6/6 безобидных
рефакторингов (переименования, минификация CSS, перестановка объявлений, инлайн-обработчик
`afterprint`, вырезанные комментарии) остаются молчаливыми. Оркестратор независимо
перепроверил control (exit 0) и по одной мутации на G1/G2/G3 — все три сработали.

**Реализация не изменялась.** Единственные изменённые файлы: `ui/scripts/check-print-isolation.mjs`
(новый) и `ui/package.json` (один добавленный шаг в `lint`).

---

## Manual UAT Sign-Off 2026-08-05

**Подтверждено пользователем (Alexander Platov) напрямую в сессии:** ручной UAT по всем строкам
Manual-Only выполнен живьём в ходе сегодняшнего debug-раунда и шести быстрофиксов
(`260805-edd`, `-gdz-01`, `-gdz-02`, `-har`, `-ifj`, `-jwf`) — каждый из них правился и
проверялся в работающем приложении, в desktop-режиме и в LAN-браузере.

**Отдельно подтверждено: печать акта на 2 страницы — результат корректный.**

Это закрывает единственный пробел, эскалированный аудитом того же дня (G4). Формулировка
«многостраничное превью ни разу не наблюдалось работающим», записанная в
`.planning/debug/resolved/print-preview-always-degrades.md` (~L1128) и перенесённая в таблицу
Manual-Only несколькими часами ранее, **устарела** — исправлено выше по месту.

`/gsd-verify-work 33` намеренно не запускался: он повторил бы тот же чек-лист вопрос за
вопросом, а ответы уже получены живым использованием. Отдельный `33-VERIFICATION.md` не
создавался — этот блок является записью верификации фазы.
