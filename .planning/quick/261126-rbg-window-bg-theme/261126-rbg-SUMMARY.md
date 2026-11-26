---
phase: quick-261126-rbg
plan: 01
subsystem: frontend-shell
tags: [tauri, theme, webview2, capabilities, gate]
requires: []
provides:
  - стартовый backgroundColor нативного окна Trackly и Trackly Link
  - синхронизация цвета окна с --tr-bg при смене темы (только Tauri)
  - поведенческий гейт check-window-bg-theme в pnpm --dir ui lint
affects: [packages/kit theme store, capabilities обоих приложений, check-link-layout L2/L3]
key-files:
  created:
    - ui/scripts/check-window-bg-theme.mjs
  modified:
    - crates/trackly-app/tauri.conf.json
    - crates/link-app/tauri.conf.json
    - crates/trackly-app/capabilities/main.json
    - crates/link-app/capabilities/main.json
    - packages/kit/package.json
    - packages/kit/src/stores/theme.svelte.ts
    - pnpm-lock.yaml
    - ui/package.json
    - ui/scripts/check-workspace-boundaries.mjs
    - link-ui/scripts/check-link-layout.mjs
key-decisions:
  - "Право расширено ровно на core:window:allow-set-background-color; L3 теперь ждёт точно три права"
  - "Динамический импорт @tauri-apps/api/window за проверкой __TAURI_INTERNALS__ при каждом вызове; вынесен в отдельный чанк"
  - "Ошибка setBackgroundColor молча проглатывается: окно без права не должно ронять UI"
metrics:
  completed: 2026-11-26
  tasks: 3
---

# Quick 261126-rbg: фон окна следует теме

Нативный фон окна обоих приложений окрашен в цвет темы: стартово `#eef1f6` (светлый `--tr-bg`) из `tauri.conf.json`,
далее `theme.svelte.ts` при каждом `initTheme`/`setTheme`/смене системной темы вызывает
`getCurrentWindow().setBackgroundColor(<вычисленный --tr-bg>)`. В браузере (LAN) ничего не вызывается и API окна не импортируется.

## Коммиты

- c14fd054 feat: конфиги окна, право, стор темы, границы, link-layout
- 6b57fbf4 test: гейт check-window-bg-theme в `pnpm lint`
- 80d9e31e fix: `window.getComputedStyle` (eslint no-undef), якорь мутации не зависит от переноса строк prettier

## Гейты (каждый отдельной командой)

| Команда | Exit |
|---|---|
| `pnpm --dir ui lint` (включая новый гейт, selftest-wiring 77 гейтов) | 0 |
| `pnpm --dir ui svelte-check` | 0 |
| `pnpm --dir ui exec vite build` | 0 |
| `pnpm --dir link-ui lint` | 0 |
| `pnpm --dir link-ui build` | 0 |
| `pnpm --dir link-ui svelte-check` | 0 |
| `cargo clippy -p link-app -p trackly-app --all-targets -- -D warnings` | 0 |

`cargo test` не запускался (по плану). `@tauri-apps/api/window` в обоих билдах ушёл в отдельный чанк `window-*.js`.

## Доказательство гейта на реальном исходнике

Кроме `--selftest` (5 мутантов: убран вызов, снят Tauri-guard, проброс ошибки, захардкоженный цвет, без `.trim()`), на настоящем
`packages/kit/src/stores/theme.svelte.ts` вручную: удалён `void syncWindowBackground();` -> гейт exit 1 (W1, W2, W4);
снят guard Tauri -> exit 1 (W3); после возврата exit 0. `git diff` по файлу после возврата чист.

## Deviations from Plan

**1. [Rule 1 - Bug] `getComputedStyle` не определён для eslint (no-undef).** Найдено в `pnpm --dir ui lint`.
Исправлено на `window.getComputedStyle(...)` без правки eslint-конфига (правила не ослаблялись). Коммит 80d9e31e.

**2. [Rule 1 - Bug] Якорь мутации ломался от prettier.** Цепочка `.getPropertyValue(...).trim()` переносилась на несколько строк,
якорь встречался 0 раз. Код разбит на `const style = ...; const color = style.getPropertyValue('--tr-bg').trim();`,
якорь привязан к `style.getPropertyValue(...)`. Коммит 80d9e31e.

**3. Отклонение от плана:** L2 в check-link-layout дополнительно проверяет `backgroundColor` окна (как и предписывал шаг 6, добавлены мутанты L2/L3).
Иных отклонений нет. Дополнительно `pnpm install` обновил `pnpm-lock.yaml` (только запись `@tauri-apps/api` для packages/kit, версия 2.12.5 из уже установленной в ui/link-ui).

## Наблюдение

Один раз `check-lint-selftest-wiring.mjs --selftest` вернул exit 1 сразу после подключения нового гейта; три последующих прогона и полный `pnpm lint`
зелёные, воспроизвести не удалось (вероятно, гонка с только что созданным файлом). Если всплывёт в CI — разобрать отдельно.

## Требует проверки человеком (НЕ одобрено исполнителем)

Визуальная проверка на Windows (WebView2): ресайз окна Trackly и Trackly Link в светлой и тёмной теме (включая переключение
темы вручную и смену системной) — полоса при растягивании не белая, цвет совпадает с фоном приложения. Проверить также, что
первый кадр запуска в тёмной теме не мигает светлым (стартовый цвет конфига светлый, тёмный выставляется стором после загрузки).

## Known Stubs

Нет.

## Threat Flags

Нет новой поверхности сверх плана (T-rbg-01: добавлено единственное право, зафиксировано гейтом L3).

## Self-Check: PASSED

Файлы и коммиты c14fd054, 6b57fbf4, 80d9e31e существуют в `git log`.

## Доработка после UAT на macOS (2026-11-26)

На macOS фон NSWindow перекрашивает и шапку окна, а цвет заголовка остаётся от системной темы:
у Link тёмный текст на тёмной шапке, у Trackly (процесс без новых прав) — светлая шапка из
конфига. WKWebView ресайзится синхронно, маскировать на macOS нечего. Исправление:
`backgroundColor` убран из обоих `tauri.conf.json` (на Windows фон ставит стор темы при старте),
стор пропускает macOS по userAgent; гейт W5 + мутант, L2 теперь требует отсутствия backgroundColor.
