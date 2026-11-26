---
phase: quick-261126-rbg
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-app/tauri.conf.json
  - crates/link-app/tauri.conf.json
  - crates/trackly-app/capabilities/main.json
  - crates/link-app/capabilities/main.json
  - packages/kit/package.json
  - packages/kit/src/stores/theme.svelte.ts
  - ui/scripts/check-window-bg-theme.mjs
  - ui/scripts/check-workspace-boundaries.mjs
  - ui/package.json
  - link-ui/scripts/check-link-layout.mjs
autonomous: true
requirements: [QUICK-261126-rbg]
must_haves:
  truths:
    - "Нативное окно обоих приложений стартует с backgroundColor под --tr-bg светлой темы (#eef1f6)"
    - "При initTheme и setTheme (в т.ч. смена системной темы) в Tauri вызывается setBackgroundColor с вычисленным --tr-bg текущей темы"
    - "В браузере (нет __TAURI_INTERNALS__) setBackgroundColor не вызывается и @tauri-apps/api/window не импортируется"
    - "Отказ setBackgroundColor (нет прав) не роняет UI"
    - "Права окна расширены только на core:window:allow-set-background-color"
  artifacts:
    - path: "packages/kit/src/stores/theme.svelte.ts"
      provides: "syncWindowBackground в applyResolved"
      contains: "setBackgroundColor"
    - path: "ui/scripts/check-window-bg-theme.mjs"
      provides: "поведенческий гейт + selftest-мутанты"
  key_links:
    - from: "ui/package.json lint"
      to: "ui/scripts/check-window-bg-theme.mjs"
      via: "node scripts/check-window-bg-theme.mjs --selftest && ..."
      pattern: "check-window-bg-theme"
---

<objective>
Окрасить фон нативного окна в цвет темы (--tr-bg), чтобы полоса при ресайзе WebView2 на Windows не была белой.
Output: стартовый backgroundColor в обеих tauri.conf.json, синхронизация в сторе темы, право в capabilities, поведенческий гейт в `pnpm lint`.
Визуальная проверка на Windows — пункт для пользователя; исполнитель её не одобряет.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@packages/kit/src/stores/theme.svelte.ts
@packages/README.md
@ui/scripts/_rune-harness.mjs
@ui/scripts/check-unread-store-behavior.mjs
@ui/scripts/check-workspace-boundaries.mjs
@link-ui/scripts/check-link-layout.mjs

<interfaces>
theme.svelte.ts экспортирует themeStore, initTheme(), setTheme(p); applyResolved() приватна, ставит documentElement.dataset.theme.
Гард Tauri в kit: `typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window` (как в saveFile.ts; kit не зависит от transport).
Токены: light --tr-bg #eef1f6, dark #0e1218 (packages/design/src/styles/_tokens.scss).
Харнесс: gateMain({tag, scriptUrl, mutants:[{name,file,from,to,expect}], runCases(overrides)}), installHarness({stubs, overrides}), pkgUrl('kit','stores/theme.svelte.ts'); якорь мутации должен встречаться в исходнике ровно один раз.
check-link-layout.mjs [L3] требует ровно 2 права (core:default, window-state:default) — расширить осознанно; мутанты L3 в selftest (~строки 150+) тоже поправить.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: конфиги окна, право, сторе темы, границы</name>
  <files>crates/trackly-app/tauri.conf.json, crates/link-app/tauri.conf.json, crates/trackly-app/capabilities/main.json, crates/link-app/capabilities/main.json, packages/kit/package.json, packages/kit/src/stores/theme.svelte.ts, ui/scripts/check-workspace-boundaries.mjs, link-ui/scripts/check-link-layout.mjs</files>
  <action>
1. В объект окна обеих tauri.conf.json добавить `"backgroundColor": "#eef1f6"` (светлая тема --tr-bg; тёмную выставит стор при старте).
2. В обоих capabilities/main.json дописать в permissions строку `"core:window:allow-set-background-color"`, больше ничего.
3. packages/kit/package.json: добавить dependency `"@tauri-apps/api": "^2.11.0"` (как в ui/link-ui), затем `pnpm install` в корне воркспейса (проверь, как устроен workspace; lockfile обновить, если он есть) — иначе динамический импорт из packages/kit не резолвится.
4. theme.svelte.ts: в конце applyResolved вызвать `void syncWindowBackground()`. Новая функция: если `typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)` — return (проверка при каждом вызове, не при загрузке модуля). Иначе в try/catch: прочитать `getComputedStyle(document.documentElement).getPropertyValue('--tr-bg').trim()`; пусто — return; `const { getCurrentWindow } = await import('@tauri-apps/api/window'); await getCurrentWindow().setBackgroundColor(color);` catch — молча проглотить (комментарий почему: окно без прав не должно ронять UI). dataset.theme выставляется синхронно до вызова, так что computed уже под новую тему. Динамический импорт обязателен.
5. check-workspace-boundaries.mjs: добавить 'packages/kit/src/stores/theme.svelte.ts' в TAURI_IMPORT_ALLOWLIST с комментарием (quick 261126-rbg: setBackgroundColor окна за guard'ом Tauri, динамический импорт). Проверить, не режет ли B4 (link-ui разрешает только api/core) путь через пакет kit — B4 относится к исходникам link-ui, не kit; если гейт всё же красный, расширить осознанно.
6. check-link-layout.mjs: [L3] — ожидать ровно три права: core:default, window-state:default, core:window:allow-set-background-color (длина 3, includes всех трёх), обновить сообщение и заголовок-комментарий L3; в selftest сохранить мутанты (подмена window-state, добавление fs:default) рабочими и добавить мутант: убрать "core:window:allow-set-background-color" → L3 красный. Дополнительно в L2 проверить, что у окна backgroundColor задан (`w.backgroundColor` непустая строка), и добавить мутант на его удаление.
Без хардкода реальных данных; цвета — только токены дизайна.
  </action>
  <verify>
    <automated>node link-ui/scripts/check-link-layout.mjs --selftest && node link-ui/scripts/check-link-layout.mjs && node ui/scripts/check-workspace-boundaries.mjs --selftest && node ui/scripts/check-workspace-boundaries.mjs</automated>
  </verify>
  <done>Конфиги и права обновлены ровно на одно новое право; гейты link-layout и boundaries зелёные вместе со своими selftest; theme.svelte.ts синхронизирует фон только в Tauri.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: поведенческий гейт check-window-bg-theme и подключение в lint</name>
  <files>ui/scripts/check-window-bg-theme.mjs, ui/package.json</files>
  <behavior>
    - [W1] Tauri + initTheme (preference 'light', нет сохранённого): getCurrentWindow().setBackgroundColor вызван с цветом, который fake getComputedStyle вернул для data-theme=light (#eef1f6)
    - [W2] после setTheme('dark') вызов с #0e1218 (fake getComputedStyle читает documentElement.dataset.theme и отдаёт цвет по теме)
    - [W3] браузер (нет __TAURI_INTERNALS__): ни одного вызова, импорт @tauri-apps/api/window не запрашивался
    - [W4] setBackgroundColor бросает/отклоняется: setTheme не бросает и нет unhandled rejection
  </behavior>
  <action>
Создать ui/scripts/check-window-bg-theme.mjs по образцу check-unread-store-behavior.mjs (шапка с описанием кейсов, `gateMain`, `installHarness`). Стабы: '@tauri-apps/api/window' → data-модуль, экспортирующий getCurrentWindow, который делегирует в globalThis.__h (считает запросы импорта и вызовы setBackgroundColor с аргументом; режим «бросать»). В runCases до импорта модуля поднять fake-глобалы: window (с/без __TAURI_INTERNALS__; matchMedia → { matches:false, addEventListener(){} }), document.documentElement ({ classList:{add,remove}, dataset:{} }), localStorage (Map), requestAnimationFrame (cb сразу), getComputedStyle (по documentElement.dataset.theme → getPropertyValue('--tr-bg') возвращает ' #eef1f6' / ' #0e1218' с пробелом — проверка trim). Сценарии выполнять при свежем импорте модуля с query-строкой (pkgUrl(..., '?t=N')), для Tauri и браузера отдельно; ждать вызова через `until`, для W3 — после небольшого ожидания микротасков (until на отрицательное условие не годится: дождаться завершения через `await new Promise(r => setTimeout(r, 50))`). Не использовать реальные данные.
Мутанты (file 'theme.svelte.ts' относительно packages? — `file` в gateMain считается от SRC ui, поэтому для пакета передавать относительный путь вида '../../packages/kit/src/stores/theme.svelte.ts' и проверить, что path.join(SRC, ...) даёт верный файл; иначе расширить gateMain осторожно, не ломая остальных): (а) убрать вызов `void syncWindowBackground()` → expect W1; (б) снять guard `'__TAURI_INTERNALS__' in window` (заменить на true-условие) → expect W3; (в) убрать `.trim()` не обязательно; (г) убрать try/catch вокруг setBackgroundColor (замена `catch {` на пробрасывающий вариант) → expect W4; (д) захардкодить цвет вместо computed → expect W2. Якоря в theme.svelte.ts должны быть уникальны — при необходимости добавить в исходник комментарии-метки не нужно, подбирай уникальные фрагменты.
Подключить в ui/package.json lint: добавить `&& node scripts/check-window-bg-theme.mjs --selftest && node scripts/check-window-bg-theme.mjs` рядом с check-unread-store-behavior (до check-gate-coverage/wiring; мета-гейты W1/W2 требуют и простой, и --selftest прогон). Если check-gate-coverage требует регистрации гейта где-то ещё — зарегистрировать.
  </action>
  <verify>
    <automated>node ui/scripts/check-window-bg-theme.mjs --selftest && node ui/scripts/check-lint-selftest-wiring.mjs --selftest && node ui/scripts/check-lint-selftest-wiring.mjs && node ui/scripts/check-gate-coverage.mjs</automated>
  </verify>
  <done>Гейт зелёный на реальном theme.svelte.ts, все мутанты (убрать вызов, снять Tauri-guard, снять try/catch, хардкод цвета) краснеют; гейт подключён в lint с --selftest; мета-гейты зелёные.</done>
</task>

<task type="auto">
  <name>Task 3: полный набор гейтов по отдельности</name>
  <files>ui/package.json</files>
  <action>
Прогнать по отдельности (каждый отдельной командой, не цепочкой в одну, фиксируя результат): `pnpm --dir ui lint`, `pnpm --dir ui svelte-check`, `pnpm --dir ui exec vite build`, `pnpm --dir link-ui lint`, `pnpm --dir link-ui build`, `pnpm --dir link-ui svelte-check`, `cargo clippy -p link-app -p trackly-app --all-targets -- -D warnings` (при exit 69 префикс `DEVELOPER_DIR=/Library/Developer/CommandLineTools`). Не запускать cargo test. Исправить найденное (prettier на новых файлах: `pnpm --dir ui exec prettier --write` для изменённых). Проверить, что vite build обоих приложений не тянет @tauri-apps/api/window в основной чанк (динамический импорт → отдельный чанк допустим). Перед коммитом (если коммит нужен) убедиться: нет реальных данных организации в изменениях (`git diff` на цвета/имена). Одобрение визуала на Windows не давать — отметить в SUMMARY как пункт пользователю: ресайз окна Trackly и Trackly Link на Windows в светлой и тёмной теме, полоса не белая.
  </action>
  <verify>
    <automated>pnpm --dir ui lint && pnpm --dir link-ui lint</automated>
  </verify>
  <done>Все перечисленные гейты зелёные по отдельности; в SUMMARY записан пункт ручной проверки на Windows для пользователя.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| webview JS -> Tauri window API | UI-код вызывает привилегированную команду окна |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-rbg-01 | E | capabilities/main.json | mitigate | добавлено только core:window:allow-set-background-color; гейт link-layout [L3] фиксирует точный список прав |
| T-rbg-02 | T | значение --tr-bg из CSS | accept | цвет приходит из собственных токенов дизайна, не из пользовательского ввода; ошибка применения проглатывается |
| T-rbg-SC | T | npm install | mitigate | добавляется только официальный @tauri-apps/api, уже используемый в ui и link-ui той же версии |
</threat_model>

<verification>
Гейты из Task 3 зелёные по отдельности; selftest нового гейта краснит мутантов; link-layout и workspace-boundaries зелёные.
</verification>

<success_criteria>
backgroundColor задан в обеих конфигурациях; стор темы синхронизирует цвет окна только в Tauri; право добавлено в оба capabilities; поведенческий гейт в `pnpm --dir ui lint`; визуальная проверка на Windows оставлена пользователю.
</success_criteria>

<output>
Create `.planning/quick/261126-rbg-window-bg-theme/261126-rbg-SUMMARY.md` when done
</output>
