# Phase 23: Токены и основы дизайн-системы — карта паттернов

**Составлено:** 2026-07-17
**Тип фазы:** механическая CSS/token-миграция, не построение фич — карта организована **по категориям
семейств**, а не по 118 отдельным файлам (см. `<scoping_guidance>` в задании).
**Категорий:** 6 · **Аналогов найдено:** 4/6 (для двух категорий аналога в проекте нет — см. «Аналогов нет»).

---

## Классификация по категориям

| Категория | Роль | Data flow | Ближайший аналог | Качество совпадения |
|---|---|---|---|---|
| 1. `ui/src/styles/_tokens.scss` (полный rewrite) | config/design-tokens | transform (CSS custom properties) | Сам файл (текущая версия, 89 строк) — эталон «что было» | exact (это же файл, до/после) |
| 2. `ui/src/styles/global.scss` (мех. миграция + `.tr-mono`) | config/global-styles | transform | Сам файл (текущая версия) + `.skip-link` как эталон объявления глобального класса | exact |
| 3. Sweep ~118 `.svelte` файлов (scoped `<style lang="scss">`) | component (mixed) | transform | `ActItemsTable.svelte`, `PersonAutocomplete.svelte`, `Button.svelte` (представители трёх правил сразу) | role-match (представительная выборка, не 1:1) |
| 4. `ui/scripts/check-tokens.mjs` + `verify-value-map.mjs` | utility/CI-gate | batch (static analysis) | **Нет аналога в проекте** (см. ниже) | no-analog |
| 5. `ui/package.json` + CI workflow wiring | config | request-response (CI pipeline) | Сам `ui/package.json` (текущий `lint` script) + `.github/workflows/ci-fast.yml`/`ci-full.yml` (уже вызывают `pnpm lint`) | exact (расширение существующей точки) |
| 6. `ui/eslint.config.js` (D-15 fix) | config | transform | Сам файл, `browserGlobals` объект | exact |

---

## Категория 1: `ui/src/styles/_tokens.scss` — полный rewrite

**Аналог:** сам файл, текущее состояние (89 строк) — это одновременно «эталон структуры файла» (как
организован `:root`/`[data-theme='dark']` блок) и «эталон старых значений» (source of value-map).

**Текущая структура (полностью, для справки — она же то, что сносится по D-01):**
```scss
// ui/src/styles/_tokens.scss (СНОСИТСЯ ЦЕЛИКОМ, D-01)
:root,
[data-theme='light'] {
  --color-bg: #ffffff;
  --color-surface: #f5f6f8;
  --color-surface-raised: #ffffff;
  --color-surface-sunken: #eaecef;
  --color-accent: #2563eb;
  --color-accent-hover: #1d4ed8;
  --color-accent-focus: rgba(37, 99, 235, 0.3);
  --color-destructive: #dc2626;
  --color-success: #16a34a;
  --color-warning: #d97706;
  --color-text-primary: #111827;
  --color-text-secondary: #4b5563;
  --color-text-muted: #9ca3af;
  --color-text-inverse: #ffffff;
  --color-border: #e5e7eb;
  --color-border-strong: #d1d5db;
  color-scheme: light;
}

[data-theme='dark'] {
  /* … зеркальный блок … */
  color-scheme: dark;
}

:root {
  --space-xs: 4px;
  --space-sm: 8px;
  --space-md: 16px;
  --space-lg: 24px;
  --space-xl: 32px;
  --space-2xl: 48px;
  --space-3xl: 64px;

  // ── Layout constants (D-02: НЕ переименовывать, остаются как есть) ──────
  --sidebar-width: 240px;
  --header-height: 56px;
  --modal-max-width: 640px;
  --modal-max-width-wide: 960px;
  --touch-target-min: 36px;
  --row-height: 40px;
  --row-height-dense: 32px;

  --radius-sm: 4px;
  --radius-md: 8px;

  --shadow-elev-1: 0 1px 2px rgba(0, 0, 0, 0.06), 0 1px 1px rgba(0, 0, 0, 0.04);
  --shadow-elev-2: 0 4px 12px rgba(0, 0, 0, 0.08), 0 2px 4px rgba(0, 0, 0, 0.06);
  --shadow-elev-2-dark: 0 4px 16px rgba(0, 0, 0, 0.5), 0 2px 8px rgba(0, 0, 0, 0.3); // D-03: удалить, 0 call-site

  --font-family-base: -apple-system, 'Segoe UI', 'Roboto', 'Helvetica Neue', 'Arial', sans-serif;
  --font-size-body: 14px;
  --font-size-label: 13px;
  --font-size-heading: 20px;
  --font-size-display: 28px;
  --font-weight-regular: 400;
  --font-weight-medium: 500;
  --font-weight-semibold: 600;
  --line-height-body: 1.5;
  --line-height-label: 1.4;
  --line-height-heading: 1.3;
  --line-height-display: 1.2;
}
```

**Что копировать из этой структуры в новый файл:**
- Форма блоков `:root, [data-theme='light'] { … }` / `[data-theme='dark'] { … }` — механика переключения
  темы через атрибут на корне **не меняется**, меняется только содержимое (см. UI-SPEC цветовые таблицы).
- Layout-константы (строки со `--sidebar-width` и далее) переносятся **дословно, без изменений**
  (D-02) — буквально copy-paste блока в новый файл.
- `--shadow-elev-2-dark` — единственная строка, которую нужно **не переносить** (D-03).
- Всё остальное (цвет/space/radius/typography) переписывается с нуля по значениям из `23-UI-SPEC.md`
  (не пересчитывать, копировать таблицы дословно — см. UI-SPEC §Color Tokens, §Typography Scale,
  §Spacing Scale, §Radii).
- Новая типографика вводит **и** composite shorthand (`--tr-text-{role}`), **и** decomposed axes
  (`--tr-font-size-{role}`, `--tr-font-weight-{role}`, `--tr-line-height-{role}`) — см. UI-SPEC пример:
  ```scss
  --tr-text-body: 400 14px/1.5 var(--tr-font-family);
  --tr-font-size-body: 14px;
  --tr-font-weight-body: 400;
  --tr-line-height-body: 1.5;
  ```

**Комментарий-шапка файла** (Claude's Discretion — обновить под новую реальность):
текущая шапка `// Design tokens — Phase 2. Full palette …` — заменить на актуальную (Phase 23,
`--tr-*`-слой, ссылку на UI-SPEC как источник значений).

---

## Категория 2: `ui/src/styles/global.scss` — механическая миграция + `.tr-mono`

**Аналог:** сам файл, текущее состояние (103 строки). `.skip-link` — единственный существующий
глобальный класс в проекте, это прямой эталон того, **как** здесь объявляется глобальный класс
(D-12 добавляет `.tr-mono` по тому же паттерну).

**Точка подключения токенов (не менять, D-05):**
```scss
// ui/src/styles/global.scss:1-7
@use './tokens';
```
Комментарий над этой строкой (строки 1-6) объясняет, почему НЕ `scss.prependData` — актуализировать
под новую реальность (Claude's Discretion), сам факт (`@use './tokens'` — единственная точка) не менять.

**Механическая карта имён для этого файла (дословно из D-05):**
| Было (текущий global.scss) | Строка | Станет |
|---|---|---|
| `var(--color-bg)` | 29 | `var(--tr-bg)` |
| `var(--color-text-primary)` | 30 | `var(--tr-text-primary)` |
| `var(--font-family-base)` | 31 | `var(--tr-font-family)` |
| `var(--font-size-body)` | 32 | `var(--tr-font-size-body)` |
| `var(--line-height-body)` | 33 | `var(--tr-line-height-body)` |
| `var(--color-accent-focus)` | 42 | `var(--tr-focus-ring)` |
| `var(--space-md)` (внутри `.skip-link:focus`) | 75 | `var(--tr-space-md)` |
| `var(--color-accent)` (фон `.skip-link:focus`) | 76 | `var(--tr-accent)` |
| `var(--color-text-inverse)` (текст `.skip-link:focus`) | 77 | `var(--tr-text-inverse)` |
| `var(--font-size-body)` (внутри `.skip-link:focus`) | 78 | `var(--tr-font-size-body)` |
| `var(--font-weight-medium)` (внутри `.skip-link:focus`) | 79 | `var(--tr-font-weight-medium)` |
| `var(--color-border-strong)` (скроллбар) | 96 | `var(--tr-border-strong)` |
| `var(--color-text-muted)` (скроллбар hover) | 100 | `var(--tr-text-tertiary)` |

**Форма/охват focus-ring НЕ меняется** (только имя токена внутри — QA-02 это фаза 30):
```scss
// ui/src/styles/global.scss:40-43 — правило и охват (*:focus-visible) остаются как есть
*:focus-visible {
  outline: none;
  box-shadow: 0 0 0 3px var(--color-accent-focus);  // → var(--tr-focus-ring)
}
```

**Эталон для объявления `.tr-mono` — структура `.skip-link` (единственный существующий global-класс,
D-12 явно ссылается на этот паттерн):**
```scss
// ui/src/styles/global.scss:60-82 — паттерн: селектор верхнего уровня (не .scoped),
// живёт в global.scss (не в компоненте), использует токены через var()
.skip-link {
  position: absolute;
  /* … */
  &:focus {
    /* … */
    padding: var(--space-md);
    background: var(--color-accent);
    color: var(--color-text-inverse);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-medium);
    text-decoration: none;
  }
}
```
`.tr-mono` по D-12 добавляется рядом, той же плоской структурой (без вложенности `&:focus`, это не
интерактивный элемент):
```scss
.tr-mono {
  font: var(--tr-text-mono);
  font-variant-numeric: tabular-nums;
}
```

---

## Категория 3: Sweep ~118 `.svelte`-файлов — три правила миграции

Это НЕ per-file список (118 почти одинаковых записей не нужны — см. `<scoping_guidance>`). Ниже — три
репрезентативных файла, показывающих, как цвет/space+radius/типографика сталкиваются в одном
scoped `<style lang="scss">` блоке, плюс общая форма компонента.

### Общая форма (везде одинаковая — не меняется этой фазой)
Каждый компонент — `<script lang="ts">` + разметка + `<style lang="scss">` (scoped, без `:global`
кроме явных dropdown-порталов). Sweep трогает **только** содержимое `<style>`-блока (+ добавление
`class="tr-mono"` в разметке точечно по D-13/D-16) — `<script>`-логика и структура разметки не
меняются (Out of Scope: «изменение поведения/логики»).

### Аналог A — `ui/src/features/acts/ActItemsTable.svelte` (все три правила в одном файле)

```scss
// ui/src/features/acts/ActItemsTable.svelte:46-107 — типичный scoped-блок ДО миграции
.items-table {
  width: 100%;
  border: 1px solid var(--color-border);        // цвет: по роли → var(--tr-border)
  border-radius: var(--radius-sm);               // radius: по значению → var(--tr-radius-xs) (не поле/кнопка → 4px)
  overflow: hidden;
}
.thead {
  background: var(--color-surface-sunken);       // цвет → var(--tr-surface-sunken)
  border-bottom: 1px solid var(--color-border);  // → var(--tr-border)
}
.th {
  padding: var(--space-sm) var(--space-md);      // space: по значению → var(--tr-space-xs) var(--tr-space-md)
  font-size: var(--font-size-label);             // типографика: по роли → декомпозированная ось --tr-font-size-label
  font-weight: 500;                              // литерал 500 — можно заменить на var(--tr-font-weight-medium) по смыслу роли
  color: var(--color-text-secondary);            // → var(--tr-text-secondary)
}
.td {
  padding: var(--space-sm) var(--space-md);      // → var(--tr-space-xs) var(--tr-space-md)
  font-size: var(--font-size-body);              // → var(--tr-font-size-body)
  color: var(--color-text-primary);              // → var(--tr-text-primary)
}
.col-qty.tabular {
  font-variant-numeric: tabular-nums;            // уже делает то, что теперь инкапсулирует .tr-mono — НЕ дублировать вручную, обычный CSS-текст здесь не идентификатор (кол-во, не инв./серийный №)
}
.muted {
  color: var(--color-text-muted);                // → var(--tr-text-tertiary)
}
.empty {
  padding: var(--space-xl);                      // → var(--tr-space-2xl) (24px→32px карта: --space-xl(32px) → --tr-space-2xl(32px))
  color: var(--color-text-muted);                // → var(--tr-text-tertiary)
  font-size: var(--font-size-body);              // → var(--tr-font-size-body)
}
```

**Пример точки применения `.tr-mono` (D-13, «списки/таблицы» — чистый in-scope case) в этом же файле:**
```svelte
<!-- ui/src/features/acts/ActItemsTable.svelte:34-35 — данные-идентификаторы в табличной ячейке -->
<div class="td col-inv" class:muted={!item.inventory_no}>{item.inventory_no ?? '—'}</div>
<div class="td col-serial" class:muted={!item.serial_no}>{item.serial_no ?? '—'}</div>
<!-- ПОСЛЕ: добавить class="tr-mono" рядом с существующими классами (col-inv/muted не убирать) -->
```
Аналогичный паттерн (уже изолированный `<span>`, тривиально): `ActListRow.svelte:62`
`<span class="number">№{act.number}</span>` → добавить `tr-mono` на этот `<span>`. Контрастный
пример, требующий частичного оборачивания (не весь заголовок): `ActDetail.svelte:69`
`<h2 class="detail-title">№{act.number} от {headerDate}</h2>` — только сегмент `№{act.number}`
оборачивается в `<span class="tr-mono">`, `{headerDate}` — нет (не идентификатор).

### Аналог B — `ui/src/lib/components/PersonAutocomplete.svelte` (undefined-token баги + dropdown)

```scss
// ui/src/lib/components/PersonAutocomplete.svelte:276-330
&:disabled {
  background: var(--color-surface-muted);   // BUG: не определён нигде сегодня → var(--tr-surface-sunken) (UI-SPEC карта)
  color: var(--color-text-muted);           // → var(--tr-text-tertiary)
  cursor: not-allowed;
}
/* … */
:global(.dropdown--person .dropdown-empty) {
  padding: var(--space-sm) var(--space-md); // → var(--tr-space-xs) var(--tr-space-md)
  color: var(--color-text-muted);           // → var(--tr-text-tertiary)
  font-size: var(--font-size-sm);           // QA-01 BUG: не определён нигде → var(--tr-font-size-caption) (роль caption)
}
:global(.dropdown--person .dropdown-item:hover),
:global(.dropdown--person .dropdown-item.active) {
  background: var(--color-surface-hover);   // BUG: не определён нигде → var(--tr-row-hover)
}
```
Тот же `--color-surface-muted`-баг — в `LocationAutocomplete.svelte:203`, `DatePicker.svelte:71`.
Тот же `--color-surface-hover`-баг — в `LocationAutocomplete.svelte:244`, `CompatibilityEditor.svelte:309`,
`ModelFormModal.svelte:558`. Все закрываются одной и той же строкой карты — не отдельная задача, но
пропускать эти два имени при sweep нельзя (см. RESEARCH.md §Ground-truth).

### Аналог C — `ui/src/lib/components/Button.svelte` (radius-sm allowlist + hardcoded hex + motion)

```scss
// ui/src/lib/components/Button.svelte:38-46 — Button — ОДИН из 4 файлов allowlist D-07
.btn {
  border-radius: var(--radius-sm);   // Button В allowlist → var(--tr-radius-sm) (6px, НАМЕРЕННЫЙ сдвиг 4→6)
  font-family: var(--font-family-base);      // → var(--tr-font-family)
  font-weight: var(--font-weight-semibold);  // → var(--tr-font-weight-semibold)
  transition: none; // Theme switch: no transitions per UI-SPEC §Motion  ← НЕ ТРОГАТЬ (мотив, вне scope)
}
```
```scss
// Button.svelte:78 и :103 — hardcoded hex внутри <style> (DS-01 нарушение, ловится hex-гейтом)
.btn-primary {
  color: #ffffff;   // → var(--tr-on-accent)
}
/* … */
.btn-destructive {
  color: #ffffff;   // → var(--tr-on-accent)
}
```
Для сравнения — тот же `--radius-sm` в файле, которого НЕТ в allowlist (пример из sweep-списка
RESEARCH.md §D-07): `ActItemsTable.svelte:50` (см. Аналог A) идёт в `--tr-radius-xs` (4px), а не
`--tr-radius-sm` — тот же старый токен, два разных целевых имени в зависимости от файла. Реальный
allowlist на 6px — ровно 4 файла: `Button.svelte`, `Input.svelte`, `Select.svelte`, `Textarea.svelte`
(подтверждено research; `Checkbox.svelte` не существует до фазы 24).

---

## Категория 4: `ui/scripts/check-tokens.mjs` + `ui/scripts/verify-value-map.mjs`

### Аналогов нет — подтверждено

Проверено дерево репозитория целиком (не только `ui/`): нет каталога `scripts/` ни в `ui/`, ни в
корне; нет ни одного `.mjs`/`.cjs`-файла вне `node_modules`; `.github/workflows/` содержит только
YAML (никаких inline/attached node-скриптов); фронтенд-тестов нет вообще (`package.json` — только
`svelte-check`/`eslint`/`prettier`, без vitest/playwright/stylelint) — это прямо зафиксировано в
CONTEXT.md/RESEARCH.md и подтверждено повторно в этой сессии.

Единственный намёк на «house style standalone CLI-скрипта» в репозитории — `tools/procmon-check/`
(**Rust**, не Node — другой язык, не прямой аналог по синтаксису, но полезен как образец **конвенции
поведения** CLI-инструментов в этом проекте):
```rust
// tools/procmon-check/src/main.rs:21-53 — конвенции этого репо для standalone-инструментов:
// - eprintln!("[tool-name] ...") с префиксом имени инструмента в квадратных скобках — для прогресса
// - явный usage-error при отсутствии обязательного аргумента (не паника)
// - "PASS — <человеко-читаемое подтверждение>" на успешный exit
// - Result-based / process::exit с ненулевым кодом на провал (Rust: возврат Err; Node-эквивалент:
//   process.exit(1) + console.error)
```
**Рекомендация для планировщика:** переносить конвенцию `[имя-скрипта] сообщение` в stderr/stdout и
чёткий финальный PASS/FAIL с ненулевым exit-кодом при провале — единственное, что можно взять из
существующего кода репозитория «по духу», а не по синтаксису. Сама структура ESM-скрипта
(`fs.readdirSync(dir, {recursive:true})`, regex-паттерны, `execSync('git diff …')`) — RESEARCH.md
уже спроектировал конкретный, готовый к использованию дизайн (см. `23-RESEARCH.md` §Pattern 1 —
`OLD_FAMILY_RE`/`STYLE_BLOCK_RE`/`HEX_RE`/closed-world gate, и §Pattern 2 — `SPACE_MAP`/
`RADIUS_EXCEPTION_FILES`/`parseHunkTokenPairs`). Это готовый design, не архитектурный аналог из
существующего кода — планировщик должен просто перенести эти сниппеты в реальные файлы.

**Точка вызова (D-04, из package.json — см. Категория 5):** оба скрипта запускаются через `node`
напрямую (`node scripts/check-tokens.mjs`), без транспиляции — проект уже на чистом ESM
(`"type": "module"` в `ui/package.json:5`).

---

## Категория 5: `ui/package.json` + CI workflow wiring

**Аналог:** сам `ui/package.json`, текущий `scripts`-блок — единственная точка, куда встраивается
новый шаг.

```jsonc
// ui/package.json:10-18 — ТЕКУЩЕЕ состояние (до фазы 23)
"scripts": {
  "dev": "vite",
  "build": "vite build",
  "prebuild": "cargo test -p trackly-app --test export_bindings",
  "preview": "vite preview",
  "svelte-check": "svelte-check --tsconfig ./tsconfig.json",
  "lint": "eslint . --ext .ts,.svelte && prettier --check .",
  "tauri": "cd ../crates/trackly-app && node ../../ui/node_modules/@tauri-apps/cli/tauri.js"
}
```
**D-04 wiring** (добавить один шаг через `&&`, без новых полей package.json — RESEARCH.md уже
спроектировал точную строку):
```jsonc
"lint": "eslint . --ext .ts,.svelte && prettier --check . && node scripts/check-tokens.mjs"
```
`verify-value-map.mjs` (D-08) **не** встраивается в `lint` (one-shot по решению D-08/RESEARCH.md
Open Question #2) — запускается вручную: `node scripts/verify-value-map.mjs <base-ref>`.

**CI wiring — изменений YAML не требуется**, обе точки вызова `pnpm lint` уже существуют:
```yaml
# .github/workflows/ci-fast.yml:111-113
- name: pnpm lint
  working-directory: ui
  run: pnpm lint
```
```yaml
# .github/workflows/ci-full.yml:126-129 (не гоняется на Windows-раннере)
- name: pnpm lint
  if: runner.os != 'Windows'
  working-directory: ui
  run: pnpm lint
```
Оба step'а уже стоят ПОСЛЕ `pnpm svelte-check` и ПОСЛЕ `pnpm build` (который тянет `prebuild` →
`cargo test -p trackly-app --test export_bindings` — см. память проекта «gotcha»: `pnpm --dir ui build`
тянет cargo, учитывать при оценке времени задач).

**Важно (Pitfall 2 из RESEARCH.md):** `pnpm lint` **уже красный сегодня** на 5 pre-existing
eslint-ошибках, не связанных с токенами — исполнитель не должен полагаться на «весь `pnpm lint`
зелёный = моя часть готова» до того, как категория 6 (D-15) их починит. До этого — гонять
`node scripts/check-tokens.mjs` отдельно как validation-шаг.

---

## Категория 6: `ui/eslint.config.js` — D-15 fix (5 pre-existing ошибок)

**Аналог:** сам файл, `browserGlobals`-объект (строки 9-49) — точка, куда добавляются 4 недостающих
имени.

```js
// ui/eslint.config.js:9-49 — ТЕКУЩИЙ browserGlobals (не включает 4 нужных имени)
const browserGlobals = {
  document: 'readonly',
  window: 'readonly',
  console: 'readonly',
  /* … */
  HTMLElement: 'readonly',
  HTMLDivElement: 'readonly',
  HTMLButtonElement: 'readonly',
  HTMLInputElement: 'readonly',
  HTMLTextAreaElement: 'readonly',
  HTMLSelectElement: 'readonly',
  HTMLStyleElement: 'readonly',
  // ОТСУТСТВУЮТ: HTMLUListElement, SVGRectElement, SVGSVGElement, btoa
  /* … */
};
```
**Фикс (D-15, zero-risk, 4 строки):** добавить в тот же объект, рядом с существующими `HTML*Element`
записями (тот же стиль — `'readonly'`, алфавитный/тематический порядок не строгий, судя по текущему
списку):
```js
HTMLUListElement: 'readonly',
SVGRectElement: 'readonly',
SVGSVGElement: 'readonly',
btoa: 'readonly',
```
Ошибки, которые это закрывает (подтверждено прямым запуском `pnpm lint` в research-сессии):
```
ActFormItemsTable.svelte:85   'HTMLUListElement' is not defined   no-undef
ChartWidget.svelte:260,261    'SVGRectElement'/'SVGSVGElement' is not defined  no-undef
OrgSettings.svelte:93         'btoa' is not defined  no-undef
```
(5-я ошибка — `ActFormItemsTable.svelte:186` `no-useless-assignment` — НЕ globals-проблема, отдельный
маленький фикс в самом файле, не в `eslint.config.js`; вне текущей категории, но тот же D-15 task.)

---

## Общие (cross-cutting) паттерны

### Переключение темы (не меняется, только имена внутри)
Источник: `_tokens.scss` (Категория 1). Механика `:root, [data-theme='light']` / `[data-theme='dark']`
+ `color-scheme: light|dark` — применяется ко всем новым `--tr-*`-блокам один в один, включая
theme-scoped elevation (`--tr-elev-*` резолвится по-разному в `[data-theme='dark']`, заменяя ручной
`--shadow-elev-2-dark`).

### `<style lang="scss">` — единственное место, где живут токены call-site
Применяется ко всем 118 файлам категории 3 без исключений — `<script>`/разметка не трогаются
(кроме точечных `class="tr-mono"` вставок по D-13/D-16).

### Radius-sm split allowlist (D-07) — ровно 4 файла на 6px
`Button.svelte`, `Input.svelte`, `Select.svelte`, `Textarea.svelte` → `--tr-radius-sm` (6px).
Все остальные 102 из 106 call sites → `--tr-radius-xs` (4px), без исключений и без «похоже на кнопку»
триажа (см. Категория 3, Аналог C).

### `.tr-mono` (D-12/D-13/D-16) — грепаемый охват
Применять `class="tr-mono"` только к: табличным ячейкам/карточкам-деталям/dropdown-labels с
`inventory_no`/`serial_no`/`act.number` (реальные имена полей — см. RESEARCH.md §DS-03, НЕ
`inventory_number`/`serial_number`/`act_number`, как ошибочно предложено в CONTEXT.md/UI-SPEC).
НЕ применять к: полям ввода, печатным HTML-шаблонам, toast/confirm/modal-title интерполяциям
(`` `Акт №${act.number}` `` — D-16), справочнику плейсхолдеров `TemplateEditor.svelte` (grey area,
research-рекомендация — не мочить). 7 «чистых» in-scope файлов: `ActItemsTable.svelte`,
`DeviceListRow.svelte`, `PrinterDetail.svelte`, `DocumentAcceptanceModal.svelte`,
`ActFormItemsTable.svelte`, `ReturnModal.svelte`, `ActDetail.svelte`/`ActListRow.svelte`.

---

## Аналогов нет

| Файл | Роль | Data flow | Причина |
|---|---|---|---|
| `ui/scripts/check-tokens.mjs` | utility/CI-gate | batch | Нет фронтенд-тест/lint-тулинга за пределами eslint/prettier/svelte-check в проекте; stylelint/playwright явно отклонены (D-04, deferred ideas). Ближайшее — `tools/procmon-check` (Rust, другой язык, полезен только как конвенция логирования). RESEARCH.md уже дал готовый code design — использовать его. |
| `ui/scripts/verify-value-map.mjs` | utility/CI-gate | batch (git-diff based) | Та же причина — нет прецедента git-diff-парсинга скриптом нигде в репозитории. RESEARCH.md дал готовый дизайн (`SPACE_MAP`/`RADIUS_EXCEPTION_FILES`/`parseHunkTokenPairs`). |

---

## Метаданные

**Область поиска аналогов:** `ui/src/styles/`, `ui/src/**/*.svelte` (118 файлов), `ui/package.json`,
`ui/eslint.config.js`, `ui/vite.config.ts`, `.github/workflows/*.yml`, корень репозитория (`tools/`,
поиск любых `scripts/`-каталогов и `.mjs`/`.cjs`-файлов вне `node_modules`).
**Файлов прочитано напрямую:** `_tokens.scss`, `global.scss`, `package.json`, `eslint.config.js`,
`vite.config.ts`, `ActItemsTable.svelte`, `ActListRow.svelte`, `ActDetail.svelte`,
`PersonAutocomplete.svelte`, `LocationAutocomplete.svelte`, `DatePicker.svelte`, `Button.svelte`,
`tools/procmon-check/src/main.rs`, `ci-fast.yml`, `ci-full.yml`.
**Дата составления:** 2026-07-17
