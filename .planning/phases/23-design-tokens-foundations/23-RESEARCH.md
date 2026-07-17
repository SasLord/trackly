# Phase 23: Токены и основы дизайн-системы — Research

**Researched:** 2026-07-17
**Domain:** SCSS custom-property миграция в vanilla Svelte 5 SPA (без новых зависимостей), grep-based lint-гейты, git-diff-based верификация
**Confidence:** HIGH (все числа — прямой замер кодовой базы на момент research; ни одна цифра не взята из training data)

## Summary

Это чисто механическая фаза без новых библиотек: единственная неопределённость — **как** проверить
сама себя (grep-гейт D-04, скрипт-верификатор D-08), не имея браузера и без stylelint/playwright.
Ground-truth замеры (см. ниже) в целом подтверждают числа из REQUIREMENTS.md с небольшим дрейфом
(+2..+9 на семейство — код чуть вырос с 2026-07-16) **и добавляют один ранее не задокументированный
баг неопределённого токена** (`--shadow-md`, 3 сайта, не определён нигде в `_tokens.scss`).
Обнаружены также две конкретные ошибки в предложенных grep-паттернах DS-03 из CONTEXT.md/UI-SPEC —
реальные имена полей в кодовой базе `inventory_no`/`serial_no` (не `inventory_number`/`serial_number`)
и `act.number` (не `act_number`) — с них нужно грепать, иначе покрытие мониторинга DS-03 будет
близко к нулю.

Для D-04/D-08 предлагается два новых zero-dependency Node-скрипта (ESM, встроенные `fs`/`path`,
без glob-пакетов — `fs.readdirSync(dir, {recursive:true})`, доступен с Node 20.1+, CI пинит Node 20)
плюс третий, не запрошенный явно, но структурно необходимый: **closed-world токен-чекер** —
сверяет каждый `var(--tr-*)`, реально встреченный в `ui/src`, с множеством токенов, реально
*определённых* в новом `_tokens.scss`. Это единственный автоматический способ поймать опечатку в
новом имени токена (типа `--tr-spce-md`) для семейства **цвета**, где value-map-скрипт (D-08) не
применим (значения меняются намеренно, сверять не с чем).

**Primary recommendation:** `ui/scripts/check-tokens.mjs` — единый файл с тремя независимо
запускаемыми проверками (old-name gate / hex-in-style gate / closed-world existence gate),
подключаемый в `pnpm lint` через `&&`. Отдельный, разовый `ui/scripts/verify-value-map.mjs`
для D-08 — принимает git ref на вход, не встраивается в постоянный CI (D-08 явно допускает
one-shot использование).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Токен-слой (`_tokens.scss`) | Browser/Client (CSS custom properties) | — | Чистый CSS, резолвится браузером/webview, никакой серверной логики |
| Механический sweep call-sites | Browser/Client (Svelte `<style>` компиляция) | — | Компилируется Vite в статический CSS, без рантайм-зависимости от бэкенда |
| Grep-гейт (D-04) | Dev tooling / CI (Node script) | — | Статический анализ исходников, не часть runtime-приложения |
| Value-map верификатор (D-08) | Dev tooling / CI (Node script, git-diff based) | — | То же — работает на git-истории, не на рантайме |
| `.tr-mono` покрытие (DS-03) | Browser/Client (CSS класс + Svelte template) | — | Применяется в template-разметке компонентов |

Эта фаза не затрагивает Backend/API/Database — 100% Frontend Server(SSR отсутствует, SPA)/Client tier.

## Project Constraints (from CLAUDE.md)

- Стек фиксирован: Rust/Tauri/**Svelte 5/SCSS**/SQLite — фаза 23 работает строго в `ui/` (SCSS +
  Svelte), бэкенд не трогается вообще.
- Frontend — vanilla Svelte 5 SPA (без SvelteKit), `pnpm` — пакетный менеджер (`ui/package.json`
  подтверждает `packageManager: pnpm@10.17.1`).
- UI — только русская локализация (не относится напрямую к токенам, но подтверждает, что вся
  UI-строка/копирайт не меняется в этой фазе — см. UI-SPEC §Copywriting Contract: N/A).
- Никаких новых npm/cargo зависимостей без крайней необходимости — CLAUDE.md явно не запрещает,
  но D-04 отдельно фиксирует «без новых dev-зависимостей» — совпадает с общим духом проекта
  (минимальный тулинг, `rusqlite` вместо `sqlx`, `time` вместо `chrono` и т.д. — везде выбор в
  пользу меньшей поверхности зависимостей).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Старые токены сносятся целиком, без bridge-алиасов. `_tokens.scss` переписывается с
  нуля — в нём остаются ТОЛЬКО `--tr-*` (+ layout-константы D-02). Обоснование: CSS custom
  properties резолвятся в пустоту молча — пропуск не поймает сборка.
- **D-02:** Layout-константы (`--sidebar-width`, `--header-height`, `--modal-max-width`,
  `--modal-max-width-wide`, `--touch-target-min`, `--row-height`, `--row-height-dense`) остаются
  под старыми именами/значениями, вне семейств DS-01..04, греп-гейт их не трогает.
- **D-03:** `--shadow-elev-2-dark` удаляется как мёртвый код (0 call-sites).
- **D-04:** Греп-гейт в `pnpm lint`/CI, node/sh-скрипт без новых dev-зависимостей, падает на:
  (1) `var(--color-*|--space-*|--radius-*|--font-size-*|--font-weight-*|--line-height-*|--shadow-*)`
  кроме layout-констант; (2) hex-литерал внутри `<style>`-блока svelte-файла. Живёт до конца v1.2.
  Точная реализация — на усмотрение планировщика.
- **D-05:** `global.scss` мигрируется механически по карте наравне с компонентами; форма/охват
  focus-ring не меняются (QA-02, фаза 30), меняется только имя токена.
- **D-06:** Дробить планы по семействам токенов (цвет / space+radius / типографика / гейты), не по
  каталогам/экранам — один план = одно правило = один способ проверки. Точное число планов/волн —
  на усмотрение планировщика.
- **D-07:** Split `--radius-sm`: безопасный дефолт `--tr-radius-xs` (4px). 6px (`--tr-radius-sm`)
  получают ТОЛЬКО файлы из явного списка (`Button.svelte` + chrome полей ввода
  Input/Select/Textarea/Checkbox). Всё остальное, включая спорные и кнопкоподобные элементы вне
  списка, — в `--tr-radius-xs`.
- **D-08:** Скрипт-верификатор git-диффа space/radius миграции против value-map из UI-SPEC, с
  одним разрешённым исключением (split `--radius-sm`). Против playwright screenshot-diff — цвета
  меняются намеренно, pixel-diff проверял бы не то. One-shot vs постоянный CI — на усмотрение
  планировщика.
- **D-09:** Визуальную проверку делает пользователь на UAT — обе темы, 3–4 плотных экрана
  (Устройства, форма акта, Настройки). Исполнитель НЕ поднимает браузер — отвечает за
  скрипт-гейт/греп-гейт/`pnpm lint`/`svelte-check`. Гоча: серверный режим отдаёт `ui/dist`, нужен
  `pnpm --dir ui build` перед LAN-браузер UAT.
- **D-10:** Новые значения включаются сразу, «полуготовый» вид принят на несколько фаз. Инверсия
  поверхностей (`--tr-bg` #eef1f6 был белый / `--tr-surface` #ffffff был серый) — намеренная, не
  ошибка.
- **D-11:** Инверсия поверхностей мигрирует строго по карте UI-SPEC. Места с пропавшим контрастом
  выявляются на UAT (D-09) и чинятся точечно — не территория исполнителя фазы 23.
- **D-12:** Глобальный класс `.tr-mono` в `global.scss`, применяется как `class="tr-mono"` в
  компонентах. Компонент-обёртка `<Mono>` отклонена.
- **D-13:** Охват моношрифта: списки + карточки/детали, где идентификатор отображается как данные
  (строки таблиц, детали акта/устройства, выпадайки автокомплитов). НЕ в полях ввода, НЕ в печатных
  HTML-шаблонах. Точки применения ищутся грепом по биндингам `inventory_number`, `serial_number`,
  номера актов *(см. Research Focus #5 ниже — реальные имена полей в коде отличаются)*.
- **D-14:** Типографика мигрируется на декомпозированные оси 1:1 (`--font-size-body` →
  `--tr-font-size-body` и т.д.), без переписывания на composite shorthand `font:` в этой фазе.
  Исключение: `--font-size-sm` (баг QA-01) → `--tr-text-caption`/её оси.

### Claude's Discretion

- Точная структура `_tokens.scss`: один файл vs партиалы.
- Точное число планов/волн (D-06).
- Форма греп-гейта (D-04) и скрипта-верификатора (D-08): отдельные файлы vs расширение `lint`;
  расположение в дереве.
- Порядок семейств между собой (важна только зависимость всех от плана с `_tokens.scss`).
- Обновление шапки-комментария `_tokens.scss` и комментария про `prependData` в `global.scss`.

### Deferred Ideas (OUT OF SCOPE)

- Playwright / screenshot-diff — отклонена для фазы 23 (намеренная смена цветов делает pixel-diff
  бесполезным здесь); возможна для фаз 24–30, отдельной задачей.
- stylelint + postcss-svelte — отклонён как избыточная dev-зависимость ради двух правил.
- Компонент-обёртка `<Mono>` — территория фазы 24.
- Перевод типографики на composite shorthand `font: var(--tr-text-{role})` целиком — возможен позже.
- Форма focus-ring по новому дизайну и AA-контраст — QA-02, фаза 30.
- Layout-константы под `--tr-*` — сознательно оставлены как есть (D-02).

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DS-01 | Единый слой `--tr-*`, без захардкоженных hex в компонентах | Ground-truth: 47 hex-литералов внутри `<style>`-блоков (не ~40) — точный per-file/per-value список ниже. Hex-гейт дизайн подтверждён нулём false positives вне `<style>`-блоков. |
| DS-02 | Светлая/тёмная тема без артефактов при переключении | Не автоматизируемо (D-09) — см. Validation Architecture. Undefined-token баги (QA-01 + новый `--shadow-md`) — главный источник артефактов «в одной теме норм, в другой пусто» — уже не актуально: undefined resolves к пустоте одинаково в обеих темах, реальный риск — value-map ошибка меняющая контраст только в одной теме. |
| DS-03 | Типографика по 9 уровням, идентификаторы моноширинным | Ground-truth call-site inventory для `.tr-mono`: реальные имена полей — `inventory_no`/`serial_no`/`act.number` (см. §DS-03 Mono Coverage), не те, что предложены в CONTEXT.md/UI-SPEC. Даны конкретные in-scope/out-of-scope/grey-area файлы. |
| DS-04 | Space/radius мигрированы по значению, без сдвига вёрстки | Ground-truth: 651 `--space-*` (не 642), 134 `--radius-*` (не 132), 106 `var(--radius-sm)` call sites (не 103) — конкретные числа для sizing волн. Value-map верификатор (D-08) спроектирован конкретно, включая эвристику парсинга диффа и failure modes. |
| QA-01 | Устранены undefined-токен баги | Подтверждены оба документированных бага (font-size-sm × 1 сайт, radius-lg × 4 сайта) + найден **новый недокументированный** `--shadow-md` × 3 сайта. |

</phase_requirements>

## Standard Stack

Новых библиотек эта фаза не вводит (соответствует CLAUDE.md — фиксированный стек, минимальная
поверхность зависимостей). Единственный «инструмент» — два/три Node-скрипта на встроенных
ES-модулях (`node:fs`, `node:path`), запускаемые через существующий `pnpm lint`/npm-скрипт
механизм. `fs.readdirSync(dir, { recursive: true })` доступен с **Node 20.1+** [VERIFIED: локальный
запуск, Node v22.18.0]; CI (`ci-fast.yml`/`ci-full.yml`) пинит `node-version: '20'` через
`actions/setup-node@v4`, что по умолчанию тянет последний патч 20.x → recursive readdir доступен.
Явной верхней/нижней границы патч-версии в CI YAML нет — риск минимальный, но стоит знать: если
когда-нибудь CI зафиксируют на Node 20.0.0 ровно, `recursive: true` отсутствует и скрипт упадёт с
`TypeError`. Не проблема данной фазы, но стоит одной строкой прокомментировать в самом скрипте.

### Core
Нет новых runtime/dev-зависимостей.

### Supporting
Нет.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Ручной `fs.readdirSync(recursive)` walk | `glob`/`fast-glob` npm-пакет | Новая dev-зависимость — прямо противоречит D-04 «без новых dev-зависимостей»; `recursive: true` встроен в Node ≥20.1, покрывает потребность без пакета |
| Регулярка на сырой текст файла | Полноценный CSS/SCSS AST-парсер (`postcss`) | Новая зависимость + оверинжиниринг для двух grep-правил; UI-SPEC/D-04 уже отклонили stylelint по этой же причине |
| Ручной git-diff parsing (D-08) | `simple-git`/`isomorphic-git` npm-пакет | Не нужен — `git diff --unified=0` + `execSync('git ...')` из Node `child_process` достаточно, без новой зависимости |

**Installation:** не требуется — все скрипты используют только `node:fs`, `node:path`,
`node:child_process` (`git diff`).

## Package Legitimacy Audit

**Не применимо.** Фаза не устанавливает никаких npm/cargo пакетов — все проверочные скрипты
написаны на встроенных Node-модулях. Package Legitimacy Gate пропущен по этой причине.

## Architecture Patterns

### System Architecture Diagram

```
                     ┌────────────────────────────┐
                     │   ui/src/styles/_tokens.scss │  ← единственный источник --tr-*
                     │   (переписан с нуля, D-01)   │
                     └──────────────┬───────────────┘
                                    │ @use './tokens' (единственная точка, D-05)
                                    ▼
                     ┌────────────────────────────┐
                     │   ui/src/styles/global.scss │  ← body/focus-ring/scrollbar/.tr-mono
                     └──────────────┬───────────────┘
                                    │ import 'global.scss' в main.ts (до mount())
                                    ▼
        ┌───────────────────────────────────────────────────────┐
        │      118 × *.svelte  (scoped <style lang="scss">)       │
        │  105 файлов уже var(--color-*|--space-*|...) → sweep     │
        │  13 файлов без токенов (только --tr-* при новом коде)    │
        └───────────────────────────┬───────────────────────────┘
                                    │ pnpm build (vite + vite-plugin-svelte)
                                    ▼
                     ┌────────────────────────────┐
                     │   Статический CSS в бандле  │  ← браузер/webview резолвит
                     └────────────────────────────┘

  ── Проверочный контур (не runtime) ──────────────────────────────────────
   git diff (space/radius commits) → verify-value-map.mjs (D-08, one-shot)
   ui/src/**/*.{svelte,scss}       → check-tokens.mjs (D-04, постоянный CI-гейт)
                                       ├─ old-name gate
                                       ├─ hex-in-<style>-block gate
                                       └─ closed-world --tr-* existence gate (доп., см. ниже)
   pnpm lint (eslint+prettier+check-tokens.mjs) → CI (ci-fast.yml, ci-full.yml non-Windows)
```

### Recommended Project Structure

```
ui/
├── scripts/
│   ├── check-tokens.mjs        # D-04: постоянный гейт, встраивается в `lint`
│   └── verify-value-map.mjs    # D-08: git-diff верификатор, ручной/one-shot запуск
├── src/
│   └── styles/
│       ├── _tokens.scss        # переписан с нуля (D-01)
│       └── global.scss         # мигрирован по карте (D-05) + .tr-mono (D-12)
└── package.json                # "lint": "eslint ... && prettier ... && node scripts/check-tokens.mjs"
```

### Pattern 1: Grep-гейт (D-04) — три независимые проверки в одном файле

**What:** Node-скрипт без зависимостей, обходит `ui/src` рекурсивно, извлекает содержимое каждого
`.svelte`/`.scss` файла, применяет 2 обязательных (D-04) + 1 рекомендованную проверку.

**When to use:** Как финальный шаг `pnpm lint`, начиная с плана, где `_tokens.scss` уже переписан.
До этого момента запуск гейта бессмысленен — он будет падать на каждом ещё не мигрированном файле.

**Design — Rule 1 (old-name gate):**
```js
// Source: разработано в этой research-сессии по D-04, без внешних références —
// паттерн проверен на реальном дереве ui/src (см. §Ground-truth Measurements).
const OLD_FAMILY_RE =
  /--(?:color|space|radius|font-size|font-weight|line-height|shadow)-[a-z0-9-]+/gi;

// Важно: слой layout-констант (--sidebar-width, --header-height, --modal-max-width,
// --modal-max-width-wide, --touch-target-min, --row-height, --row-height-dense) НЕ пересекается
// с этими шестью префиксами ни по одному имени — подтверждено грепом (38 живых call-site,
// ни один не матчится OLD_FAMILY_RE). Явный exclude-список не нужен.

function findOldTokenViolations(filePath, content) {
  const matches = [...content.matchAll(OLD_FAMILY_RE)];
  return matches.map(m => ({ file: filePath, token: m[0], index: m.index }));
}
```
Эта проверка применяется **ко всему файлу целиком** (включая `.scss`), не только к `<style>`-блокам
— в отличие от hex-проверки. Обоснование: старые токены-имена реалистично появляются только внутри
`<style>` в текущей кодовой базе (проверено — ни одного `var(--color-...)` вне `<style>` не найдено
при ручном обзоре), но ограничивать паттерн только `<style>`-блоком не даёт выигрыша в точности и
добавляет риск пропустить случай, если кто-то вставит токен в inline `style={...}` атрибут в
разметке (в текущей кодовой базе таких нет, но гейт не должен на это полагаться).

**Design — Rule 2 (hex-in-`<style>`-block gate):**
```js
const STYLE_BLOCK_RE = /<style[^>]*>([\s\S]*?)<\/style>/g;
const HEX_RE = /#[0-9a-fA-F]{3,4}\b|#[0-9a-fA-F]{6}\b|#[0-9a-fA-F]{8}\b/g;
// {3,4}|{6}|{8} вместо {3,8} — ограничивает совпадения валидными CSS-длинами hex
// (3/4/6/8 симв.), устраняя единственный теоретический false-positive класс
// (5- или 7-значные "хвосты" совпадений). На текущем дереве {3,8} тоже не дал ни одного
// false positive (проверено скриптом), но {3,4}|{6}|{8} — более строгий инвариант на будущее.

function findHexInStyleBlocks(filePath, content) {
  if (!filePath.endsWith('.svelte')) return []; // .scss не имеет <style>-тегов — вне правила
  const violations = [];
  for (const styleMatch of content.matchAll(STYLE_BLOCK_RE)) {
    const block = styleMatch[1];
    for (const hexMatch of block.matchAll(HEX_RE)) {
      violations.push({ file: filePath, hex: hexMatch[0] });
    }
  }
  return violations;
}
```
Подтверждено (см. §Ground-truth): 0 false positives — весь hex вне `<style>`-блоков (2 совпадения,
`#13451`/`#3066`) — номера GitHub-issue в JS-комментарии, они физически вне `<style>` и не
матчатся этим правилом. Внутри `<style>`-блоков ID-селекторов (`#app { }`) в текущем дереве нет —
не источник false positive, но если появятся в будущем, длина ID-строки (`#app` = 4 симв.) может
случайно совпасть с валидной hex-длиной (rgba/hex 4-значный — `#fff8` формат с альфой). Это
теоретический edge case, не встреченный в реальном дереве — стоит одной строкой упомянуть в
комментарии скрипта, не более.

**Design — Rule 3 (доп., closed-world `--tr-*` existence gate — рекомендация, не входит в D-04
буквально, но закрывает дыру, которую ничто другое не закрывает):**
```js
// Извлекает всё множество РЕАЛЬНО ОПРЕДЕЛЁННЫХ --tr-* custom properties из финального
// _tokens.scss (после того как план 1 его переписал), затем проверяет, что каждый
// var(--tr-*) call-site в ui/src ссылается на существующее имя. Ловит опечатки в НОВЫХ
// именах (--tr-spce-md вместо --tr-space-md) — единственный класс ошибки, который НЕ ловит
// ни old-name gate (--tr-spce-md не является старым именем), ни D-08 (D-08 применим только
// к space/radius семейству, у color/typography нет "old value == new value" инварианта).
const DEFINE_RE = /(--tr-[a-z0-9-]+)\s*:/gi;
const USE_RE = /var\((--tr-[a-z0-9-]+)/gi;

function closedWorldCheck(tokensScssContent, allSvelteAndScssContent) {
  const defined = new Set([...tokensScssContent.matchAll(DEFINE_RE)].map(m => m[1]));
  const used = new Set();
  for (const content of allSvelteAndScssContent) {
    for (const m of content.matchAll(USE_RE)) used.add(m[1]);
  }
  return [...used].filter(name => !defined.has(name)); // используется, но не определено
}
```
**Важно:** это правило можно включить в `check-tokens.mjs` только ПОСЛЕ того как весь `_tokens.scss`
переписан (план 1 из D-06) — до этого момента оно будет ложно ругаться на все ещё не введённые
`--tr-*` имена, которые появляются в компонентах раньше, чем в токен-файле, при параллельной работе
над несколькими планами. Планировщику стоит явно завести это правило как gate только с волны,
следующей за планом `_tokens.scss`.

### Pattern 2: Value-map верификатор (D-08) — git-diff based

**What:** Одноразовый (или CI, на усмотрение) скрипт, который берёт `git diff <base>..<head>` для
изменённых `.svelte`/`.scss` файлов (кроме `_tokens.scss` — файл-источник новых значений, к нему
инвариант «старое значение == новое» не применяется по определению) и проверяет, что каждая замена
`--space-*`/`--radius-*` → `--tr-space-*`/`--tr-radius-*` соответствует value-map из UI-SPEC.

**Pairing heuristic (устойчивая к построчным сед-заменам):**
```js
// Source: разработано в этой research-сессии специально под D-08 — нет готового
// эталона в экосистеме для "проверить что git diff — value-preserving rename".
import { execSync } from 'node:child_process';

const SPACE_MAP = { // скопировано дословно из UI-SPEC/REQUIREMENTS.md — не пересчитывать
  '--space-xs': '--tr-space-2xs',
  '--space-sm': '--tr-space-xs',
  '--space-md': '--tr-space-md',
  '--space-lg': '--tr-space-xl',
  '--space-xl': '--tr-space-2xl',
  '--space-2xl': '--tr-space-4xl',
  '--space-3xl': '--tr-space-5xl',
};
const RADIUS_EXCEPTION_FILES = new Set([
  'src/lib/components/Button.svelte',
  'src/lib/components/Input.svelte',
  'src/lib/components/Select.svelte',
  'src/lib/components/Textarea.svelte',
  // Checkbox.svelte НЕ существует в кодовой базе на момент фазы 23 — см. §Ground-truth,
  // Checkbox — компонент, который построит Phase 24 (CMP-02). Native <input type="checkbox">
  // разбросаны по 8 файлам без собственного --radius-sm на самом чекбоксе.
]);
function expectedRadiusTarget(oldToken, filePath) {
  if (oldToken === '--radius-md') return '--tr-radius-md';
  if (oldToken === '--radius-lg') return '--tr-radius-lg'; // QA-01 fix, "old" был undefined
  if (oldToken === '--radius-sm') {
    return RADIUS_EXCEPTION_FILES.has(filePath) ? '--tr-radius-sm' : '--tr-radius-xs';
  }
  return null;
}

function parseHunkTokenPairs(diffText) {
  // Плоское сравнение токенов внутри каждого @@ hunk, а не построчная пара —
  // устойчиво к тому, что prettier/reflow может сдвинуть многотокенные строки
  // (напр. `padding: var(--space-sm) var(--space-md);`) на другое число строк.
  const hunks = diffText.split(/^@@/m).slice(1);
  const violations = [];
  for (const hunk of hunks) {
    const removedTokens = [...hunk.matchAll(/^-.*?(--(?:space|radius)-[a-z0-9]+)/gm)]
      .map(m => m[1]);
    const addedTokens = [...hunk.matchAll(/^\+.*?(--tr-(?:space|radius)-[a-z0-9]+)/gm)]
      .map(m => m[1]);
    if (removedTokens.length !== addedTokens.length) {
      violations.push({ reason: 'count-mismatch', removedTokens, addedTokens, hunk });
      continue;
    }
    for (let i = 0; i < removedTokens.length; i++) {
      // сопоставление по позиции — валидно только если план НЕ мешает токен-сдвиг с другим
      // рефакторингом в одном хануке (см. Failure modes ниже)
    }
  }
  return violations;
}
```

**Failure modes (честно, не приукрашено):**
1. **Смешанный коммит.** Если план сваливает token-sweep и несвязанный рефакторинг в один diff-хук,
   позиционное сопоставление «i-й removed ↔ i-й added» ломается без явной ошибки — скрипт может
   пропустить реальную регрессию или дать ложный violation. Митигация — процессное требование к
   планам: коммиты/задачи, выполняющие sweep, не должны содержать других правок в том же diff-хуке.
2. **Reflow, меняющий порядок токенов внутри строки.** Если сед-скрипт или ручная правка
   переставляет местами `padding: var(--space-md) var(--space-sm)` → `var(--space-sm) var(--space-md)`
   (той же природы, но другой порядок) — позиционное сопоставление даст false positive. Не
   наблюдалось в текущей кодовой базе (все multi-token строки сохраняют порядок при простом
   find&replace), но стоит знать.
3. **`--radius-sm` split — единственное разрешённое исключение (D-07).** Скрипт обязан принимать
   путь файла как параметр, иначе не отличит легитимный 4→6 (Button/Input/Select/Textarea) от
   реальной регрессии (тот же 4→6 в любом другом файле).
4. **Скрипт НЕ ловит «токен убрали и не заменили ничем»** (совсем другая проблема — строка правила
   пропала целиком) — но это ловит `count-mismatch` (removedTokens.length !== addedTokens.length),
   так что фактически ловится, просто под другой причиной в отчёте.
5. **`_tokens.scss` вне scope верификатора** — файл легитимно вводит НОВЫЕ значения под новыми
   именами (это не rename, это redefinition) — гонять по нему value-map-проверку бессмысленно.

### Anti-Patterns to Avoid

- **Полноценный CSS-парсер для двух grep-правил** — оверинжиниринг; UI-SPEC/D-04 уже отклонили
  stylelint по этой причине, тот же аргумент применим к самодельному AST-парсеру.
- **`fs.readdirSync` без `{recursive: true}` + ручная рекурсия через `readdirSync`+`statSync`** —
  работает, но лишний код при живом штатном API с Node 20.1+; не стоит писать рекурсию руками.
- **Построчное 1:1-сопоставление git diff без учёта хуков** — ломается на любом reflow; плоское
  сравнение токенов по хуку — единственный практичный вариант без полноценного diff3-алгоритма.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Обход директории `ui/src` | Ручная рекурсия `readdirSync`+`statSync` | `fs.readdirSync(dir, {recursive:true})` (Node ≥20.1, встроен) | Меньше кода, тот же результат, ноль зависимостей |
| Git diff parsing | Свой diff-алгоритм | `execSync('git diff --unified=0 ...')` + построчный regex-парсинг unified-diff формата | git уже делает diff правильно; скрипту нужно только распарсить готовый unified-формат |
| CSS/SCSS токенизация | Собственный SCSS-лексер | Плоский regex по тексту файла (`--(family)-[a-z0-9-]+`) | Домен узкий (два конкретных правила), полноценный лексер — избыточная сложность, которую сам D-04 explicitly отверг (stylelint) |

**Key insight:** Вся инфраструктура проверки этой фазы — намеренно «grep, а не парсер». Это не
недостаток, а прямое следствие явного решения пользователя (D-04: stylelint отклонён). Ключевой
риск такого подхода — false negatives на нестандартном форматировании (multi-line CSS-правила,
неожиданные пробелы) — минимизируется тем, что паттерны совпадения (`--family-name`) не зависят от
позиции в строке/файле, только от буквального текстового вхождения.

## Ground-truth Measurements (2026-07-17, this session — supersedes 2026-07-16 numbers where they differ)

Все числа получены прямым `grep`/Python-скриптом на `ui/src` в этой сессии — не training data,
не экстраполяция.

| Метрика | REQUIREMENTS.md (2026-07-16) | Замерено сейчас (2026-07-17) | Дрейф |
|---|---|---|---|
| svelte-файлов всего | 118 | **118** | 0 |
| файлов уже на токенах | 105/118 | **105/118** (13 без токенов — список ниже) | 0, точное совпадение |
| `--space-*` call sites | 642 | **651** | +9 |
| `--radius-*` call sites | 132 | **134** | +2 |
| `--font-size-*` call sites | ~344 | **350** | +6 |
| hardcoded hex (всего, `~40`) | ~40 | **47** внутри `<style>`-блоков (2 доп. вне блоков — false positives, GH issue номера в коммент.) | +7 |
| `#c0392b` count | ×22 | **×22** | 0, точное совпадение |
| `var(--radius-sm)` call sites | 103 | **106** | +3 |
| `--radius-lg` (undefined bug) сайтов | 3 названо в REQUIREMENTS, 4-й найден UI-SPEC | **4** (LoginPage:175, BlockedScreen:125, FirstRunWizard:190, PendingScreen:37) | подтверждено |
| `--font-size-sm` (undefined bug) сайтов | 1 (PersonAutocomplete.svelte:312) | **1**, тот же файл/строка | подтверждено |
| `--color-surface-hover` (bug sweep) | 4 | **4** | подтверждено |
| `--color-surface-muted` (bug sweep) | 3 | **3** | подтверждено |

**Дрейф в пределах 1-3% для всех метрик, кроме hex (+17%) и radius-sm (+3%)** — не критично для
sizing волн, но планировщик должен перезамерить непосредственно перед началом плана (код мог
измениться ещё раз между research и planning/execution).

### Новый, ранее не задокументированный баг: `--shadow-md` (undefined)

`_tokens.scss` определяет только `--shadow-elev-1`, `--shadow-elev-2`, `--shadow-elev-2-dark` — имя
`--shadow-md` **нигде не определено**, но используется в 3 местах без fallback:

```
features/cartridges/ModelListRow.svelte:169      box-shadow: var(--shadow-md);
features/cartridges/CompatibilityEditor.svelte:283  box-shadow: var(--shadow-md);
features/cartridges/ModelFormModal.svelte:539     box-shadow: var(--shadow-md);
```

Это тот же класс бага, что QA-01 (`--font-size-sm`, `--radius-lg`) — «резолвится в ничего, тень
незаметно пропадает». Не упомянут ни в REQUIREMENTS.md, ни в CONTEXT.md, ни в UI-SPEC.md.
**Рекомендация:** добавить в scope QA-01-подобных фиксов — маппинг на `--tr-elev-2` (по UI-SPEC
§Elevation, роль «Dropdowns, popovers, tooltips» — визуально ближе всего к текущему намерению
`shadow-md` на карточках моделей картриджей/совместимости).

### `--font-size-page-title`/`--font-size-subheading`/`--font-size-caption` — НЕ undefined-баги

В отличие от `--font-size-sm`/`--radius-lg`/`--shadow-md`, эти три имени **уже безопасны сегодня** —
используются с explicit CSS var()-fallback:
```
font-size: var(--font-size-page-title, var(--font-size-heading));   // 7 сайтов
font-size: var(--font-size-subheading, var(--font-size-body));      // 2 сайта
font-size: var(--font-size-caption, 12px);                          // 1 сайт
```
Не ломается сейчас, но всё равно требует sweep: после D-01 сноса `--font-size-heading`/
`--font-size-body` (fallback-цели) исчезнут, и весь `var(...)`-вызов целиком — включая fallback —
нужно переписать на новую роль (`--tr-text-h2`/`--tr-text-body`/`--tr-text-caption` — по смыслу
роли, не по букве старого имени). **Пропустить нельзя** — greр-гейт (D-04) поймает
`--font-size-heading` внутри fallback-аргумента точно так же, как поймал бы его вне var(), так что
пропуск гарантированно всплывёт на гейте, но лучше знать заранее, а не тратить цикл на дебаг.

### 13 файлов без единого токена (подтверждено, точное совпадение с "105/118")

```
pages/CartridgesPage.svelte, pages/ReportsPage.svelte, pages/UsersPage.svelte,
pages/DevicesPlaceholder.svelte, pages/PrintersPage.svelte, pages/ActsPage.svelte,
pages/Dashboard.svelte, pages/MapPage.svelte, pages/RequestsPage.svelte,
features/cartridges/CartridgeFormModal.svelte, features/acts/ActFormModal.svelte,
features/devices/DeviceFormModal.svelte, lib/components/Spinner.svelte
```
Большинство — тонкие page-обёртки/модалки, вероятно не содержащие `<style>` блока вообще (просто
делегируют вёрстку дочерним компонентам) — не требуют работы в этой фазе, но полезно знать при
финальном грепе (нулевые файлы не должны давать false «пропуск»).

### `--radius-sm` split (D-07) — конкретный allowlist, готовый к использованию

Ровно 4 файла содержат `--radius-sm` внутри списка допустимых «field chrome» компонентов:
`Button.svelte` (1 call site), `Input.svelte` (1), `Select.svelte` (1), `Textarea.svelte` (1) —
**итого 4 из 106 сайтов идут на `--tr-radius-sm` (6px), оставшиеся 102 — на `--tr-radius-xs` (4px)**.

**Важное уточнение по `Checkbox`:** отдельного `Checkbox.svelte`-компонента **не существует** в
кодовой базе на момент фазы 23 (его построит Phase 24 / CMP-02). Нативные `<input type="checkbox">`
разбросаны по 8 файлам (`NetworkSettings`, `ActiveDirectorySettings`, `LoginPage`, `ReturnModal`,
`ReturnItemsTable`, `DiscoveryResultsTable`, `UserFormModal`, `DeviceFilters`) — ни в одном из них
`--radius-sm` не применяется непосредственно к самому чекбоксу (нативный рендер без кастомного
border-radius). D-07's «Checkbox» пункт списка на сегодня — **пустое множество call sites**, ничего
делать не нужно; актуализируется только когда Phase 24 построит компонент.

**Остальные 55 файлов, использующих `--radius-sm` вне 4 shared-компонентов** (полный список ниже,
включая `.form-input`/`.form-select`/`.btn-*`/`.autocomplete-input` и т.п., которые ВИЗУАЛЬНО похожи
на поля/кнопки, но физически не являются экземплярами shared-компонентов) — **все они по D-07's
явной формулировке («включая спорные и кнопкоподобные элементы вне этого списка») идут в
`--tr-radius-xs`, без исключений.** Это НЕ открытый вопрос — CONTEXT.md явно предвидел и разрешил
именно этот сценарий. Полный список для справки при написании плана:

```
CartridgeFilters(.status-tab,.filter-select) ModelListRow(.kebab-btn,.ctx-menu)
OperationModal(.previous-cartridge-block) CartridgesSearchAndTabs(.tab)
CompatibilityEditor(.autocomplete-input,.dropdown,.remove-btn) CartridgeContextMenu(.kebab-btn)
ModelFormModal(.conflict-error,.autocomplete-input,.dropdown) BackupSettings(.folder-code,
.form-select,.form-input) ThresholdSettings(.form-input) TemplateEditor(.form-select,
.variables-panel,.var-item,.template-textarea,.preview-wrapper) NetworkSettings(.status-badge,
.server-info-block,.form-input,.form-select) ActiveDirectorySettings(.form-input)
OrgSettings(.form-input,.logo-img,.logo-placeholder) SettingsSubNav(.tab)
StorageSettings(.db-path-code) LoginPage(.form-input,.btn-sso-reserved,.server-error,.btn-submit)
BlockedScreen(.server-error,.btn-submit) FirstRunWizard(.form-input,.server-error,.btn-submit)
EmployeeLayout/Layout(.skip-link) Sidebar(.logout-btn) ReturnModal(.persons-section,.bulk-section)
ActItemsTable(.items-table) DocumentAcceptanceModal(.date-input) ActsSearchAndTabs(.tab)
ReturnItemsTable(.rows) ActFormItemsTable(.items,.qty-input,.device-input)
PdfPreviewModal(.pdf-page-frame) RequestsSearchAndTabs(.tab) RequestFormModal(.type-toggle)
RequestDetail(.resolution) DashboardPage(.period-select) ChartWidget(.chart-tooltip)
PeriodToggle(.toggle-btn) StatWidget(.widget-warning) PrintersSearchAndTabs(.tab)
TonerGauge(.gauge-track,.gauge-fill) UserListRow(.badge,.btn-action)
UserFormModal(.form-input,.form-select,.server-error) ReportSubNav(.tab)
PeriodSelector(.period-btn,.period-select) DeviceAutocompleteField(.autocomplete-input)
DeviceGroupRow(.chevron-btn) DeviceList(.skeleton-block) DeviceFormBody(.input)
DeviceContextMenu(.kebab-btn) DeviceFilters(.search-input,.status-tab)
DeviceImportCsvModal(.warning-banner,.preview-table-wrap,.error-list) Modal(.modal-close)
LocationAutocomplete(.autocomplete-input) CartridgeSelect/GroupedPrinterSelect/PrinterSelect(.select)
ThemeSwitcher(.theme-switcher) PersonAutocomplete(.autocomplete-input) DatePicker(.date-picker)
NotFound(.not-found-body)
```

## DS-03 Mono Coverage — реальные grep-термины и call-site inventory

**Критичная поправка к CONTEXT.md/UI-SPEC формулировке.** Предложенные там термины для грепа —
`inventory_number`, `serial_number`, `act_number` — **не существуют в кодовой базе**. Реальные
имена полей (из `bindings.ts`, DTO Rust-стороны):
- `inventory_no` (не `inventory_number`) — 11 файлов
- `serial_no` (не `serial_number`) — 8 файлов
- `act.number`/`.number` (не `act_number`) — используется как `act.number`, `saved.number`,
  `editTarget?.number`, `ret.number` в зависимости от контекста

Грепать нужно **`inventory_no`**, **`serial_no`**, **`\.number\b`** (последнее менее специфично —
даст шум, нужна ручная фильтрация по контексту акта).

### Call-site категоризация (по правилу D-13: списки + карточки/детали + автокомплиты; НЕ инпуты, НЕ print-шаблоны)

| Файл : строки | Что там | Категория |
|---|---|---|
| `ActItemsTable.svelte:34-35` | `<div class="td col-inv">{item.inventory_no}` / `col-serial` | **IN SCOPE** — табличная ячейка |
| `DeviceListRow.svelte:52-53` | `<td class="cell cell-numeric">{device.inventory_no}` / `serial_no` | **IN SCOPE** — табличная ячейка |
| `PrinterDetail.svelte:327,331` | `<span class="meta-value">{deviceData?.inventory_no}` / `serial_no` | **IN SCOPE** — карточка деталей |
| `DocumentAcceptanceModal.svelte:100-101` | `{#if device.inventory_no}(инв. № {device.inventory_no})` | **IN SCOPE** — это Svelte-модалка UI (НЕ печатный HTML-шаблон в `crates/trackly-app/templates/`), детали устройства перед подтверждением |
| `ActFormItemsTable.svelte:240-316` | Построение `label` для grouped dropdown (`${d.name} (SN ${d.serial_no}, инв. ${d.inventory_no})`) | **IN SCOPE** — это именно «выпадайка автокомплита» из D-13, НЕ инпут; сам `<input>` для текстового поиска рядом — НЕ трогать |
| `ReturnModal.svelte:104-151` | Построение `deviceLabel` для dropdown-опций возврата | **IN SCOPE** — та же природа, dropdown label |
| `ActDetail.svelte:69` | `<h2 class="detail-title">№{act.number} от {headerDate}</h2>` | **IN SCOPE** — заголовок карточки детали акта, но потребует обернуть только сегмент `№{act.number}` в `<span class="tr-mono">`, не весь `<h2>` |
| `ActListRow.svelte:62` | `<span class="number">№{act.number}</span>` | **IN SCOPE** — уже изолированный `<span>`, тривиально добавить класс |
| `DeviceFormBody.svelte:56-141` | `let inventoryNo = $state(...)`, `bind:` на `<input>` | **NOT IN SCOPE** — явно поле ввода (D-13 exclusion) |
| `PrinterCreateModal.svelte:62-63` | `inventory_no: null` — инициализация объекта | **NOT IN SCOPE** — не рендерится вообще |
| `DeviceImportCsvModal.svelte:39-72` | `{ value: 'inventory_no', label: 'Инвентарный №' }` — CSV column-mapping select | **NOT IN SCOPE** — это UI для настройки импорта (список ИМЁН полей), не отображение значения устройства |
| `TemplateEditor.svelte:48,61-62` | `{ code: 'device.inventory_no', desc: 'инвентарный номер' }` — справочник переменных шаблона | **GREY AREA** — это список доступных плейсхолдеров (мета-текст «`device.inventory_no` — инвентарный номер»), а не отображение реального значения устройства; технически не подпадает под «идентификатор отображается как данные». Рекомендация: НЕ мочить mono, но решение — на усмотрение планировщика/UAT |
| `DeviceContextMenu.svelte:146` | `«{device.name}» (инв. № {device.inventory_no ?? '—'}) будет помечено...` — текст confirm-диалога | **GREY AREA** — номер встроен в полное предложение подтверждения удаления, не отдельное поле; технически "данные", но изоляция в `<span>` внутри строки-предложения — субъективная косметика вне жёсткого D-13 определения |
| `ActFormModal.svelte:38`, `ActsPage.svelte:224-334`, `ReturnModal.svelte:178-331` (toast/confirm/modal-title строки) | `` `Редактировать акт №${initialAct?.number}` ``, toast-сообщения с `${act.number}` | **NOT PRACTICALLY IN SCOPE** — номер интерполирован внутри полноценного русского предложения (заголовок модалки, toast, confirm-диалог), а не отдельного поля данных; изоляция потребовала бы переписывать структуру каждой строки под `<span>`, что не является «мехническим» изменением и граничит с редизайном текста. Рекомендация для планировщика: явно исключить toast/confirm/modal-title интерполяции из DS-03 scope этой фазы, зафиксировать как осознанное решение (не молчаливый пропуск) |

**Итог для планировщика:** «чистых» call sites (табличные ячейки + карточки деталей + автокомплит
dropdown labels) — **7 файлов** (`ActItemsTable`, `DeviceListRow`, `PrinterDetail`,
`DocumentAcceptanceModal`, `ActFormItemsTable`, `ReturnModal`, `ActDetail`/`ActListRow`). Toast/modal
title/confirm-dialog интерполяции (ещё ~5 файлов) — рекомендуется явно задокументировать как
осознанно вне scope, а не молчаливо пропустить при верификации DS-03.

## Common Pitfalls

### Pitfall 1: «Резолвится в ничто» не ловится сборкой — единственная защита это гейты + UAT

**What goes wrong:** Опечатка в новом имени токена (`--tr-spce-md` вместо `--tr-space-md`) проходит
`pnpm build`/`cargo build` без единой ошибки — CSS custom properties, ссылающиеся на неопределённое
имя, просто резолвятся в `unset`/initial value молча.
**Why it happens:** Это фундаментальное свойство CSS custom properties, не баг тулинга.
**How to avoid:** D-04's old-name gate ловит только СТАРЫЕ имена. D-08's value-map верификатор
ловит только space/radius (где есть строгий "старое значение == новое значение" инвариант). Для
ЦВЕТА и ТИПОГРАФИКИ (где значения меняются намеренно) единственная автоматическая защита — closed-
world existence gate (Pattern 1, Rule 3 выше), плюс визуальный UAT (D-09).
**Warning signs:** Элемент «пропадает» (нулевой padding, чёрный/дефолтный цвет текста, browser-
default радиус) в ОДНОЙ конкретной точке экрана — типичный признак typo, а не намеренного дизайна.

### Pitfall 2: `pnpm lint` сегодня уже RED — не полагаться на него как на чистый baseline

**What goes wrong:** `pnpm lint` (eslint+prettier) **падает уже сейчас**, до какой-либо работы фазы
23, с 5 pre-existing ошибками, никак не связанными с токенами:
```
ActFormItemsTable.svelte:85   'HTMLUListElement' is not defined   no-undef
ActFormItemsTable.svelte:186  'This assigned value is not used...' no-useless-assignment
ChartWidget.svelte:260,261    'SVGRectElement'/'SVGSVGElement' is not defined  no-undef
OrgSettings.svelte:93         'btoa' is not defined  no-undef
```
Если план 23-XX добавляет `check-tokens.mjs` в `pnpm lint` через `&&` и рассчитывает на «`pnpm lint`
зелёный = моя часть готова», он унаследует этот pre-existing failure и не сможет отличить свой
регресс от чужого.
**Why it happens:** `eslint.config.js`'s `browserGlobals` объект не включает `HTMLUListElement`,
`SVGRectElement`, `SVGSVGElement`, `btoa` — простой пробел в списке globals, не связан с design
tokens.
**How to avoid:** Executor должен запускать `node scripts/check-tokens.mjs` **отдельно** как
validation-шаг для этой фазы, не полагаясь на весь `pnpm lint` как единый green/red сигнал. Отдельно
стоит решить (не в этой research-сессии — планировщик/CONTEXT решает): чинить ли эти 5
pre-existing ошибок попутно (тривиальный fix — добавить 4 имени в `browserGlobals`, zero behavior
change) — это разблокирует `pnpm lint` как честный гейт для фаз 24–30 тоже.
**Warning signs:** `pnpm lint` возвращает non-zero exit code сразу после запуска, до того как
`check-tokens.mjs` вообще выполнился (порядок `&&` означает eslint-ошибки блокируют даже запуск
нового скрипта).

### Pitfall 3: `svelte-check`, наоборот, чист — не путать два гейта

**What goes wrong:** Легко предположить, что раз `pnpm lint` красный, то и весь frontend-тулинг
сломан.
**Why it happens:** `pnpm svelte-check` (типы + a11y + Svelte 5 rune warnings) — **отдельная
команда**, независимая от eslint/prettier. Замер этой сессии: `0 ERRORS, 48 WARNINGS` — чистый
baseline (все 48 warnings — pre-existing `state_referenced_locally`/`a11y`/`css_unused_selector`,
не блокирующие).
**How to avoid:** `svelte-check` можно и нужно использовать как честный gate для фазы 23 (в
Validation Architecture — см. ниже), `pnpm lint` (eslint часть) — нет, пока pre-existing ошибки не
починены отдельно.

### Pitfall 4: три семейства — три разных инварианта проверки, смешение = ложные срабатывания

**What goes wrong:** Один и тот же скрипт, применённый одинаково к цвету/space/radius/типографике,
даст либо слишком много ложных срабатываний (цвет ведь МЕНЯЕТСЯ намеренно), либо пропустит реальные
регрессии (space/radius НЕ должны меняться, но скрипт «прощает» любое изменение, думая что это
дизайн-намерение).
**Why it happens:** Ровно тот футган, который явно называет REQUIREMENTS.md/CONTEXT.md — но стоит
явно перепроверить на уровне реализации скрипта, не только на уровне решения D-06/D-08.
**How to avoid:** D-08's value-map верификатор запускается ТОЛЬКО на диффах space/radius-планов, с
явным списком старое→новое из UI-SPEC. Никогда не запускать его на диффе color/typography-планов —
он немедленно даст false positives (там значения ДОЛЖНЫ отличаться).
**Warning signs:** Верификатор рапортует «нарушение» на diff, где меняется цвет — признак, что
скрипт запущен не на том коммите/диапазоне.

## Code Examples

### `ui/package.json` изменение (D-04 wiring)

```jsonc
// Source: изменение существующего ui/package.json (см. текущее содержимое выше) —
// добавление одного шага в существующую цепочку `&&`, без новых полей package.json.
{
  "scripts": {
    "lint": "eslint . --ext .ts,.svelte && prettier --check . && node scripts/check-tokens.mjs"
  }
}
```

### CI wiring — уже готово, изменений в workflow YAML не требуется

`ci-fast.yml` (строка 111-113) и `ci-full.yml` (строка 126-129, `if: runner.os != 'Windows'`) уже
вызывают `pnpm lint` в `ui/` рабочей директории. Добавление шага внутрь `lint` npm-скрипта
автоматически подхватывается обоими workflow **без единой правки `.yml`-файла**. Единственный нюанс
(см. Pitfall 2) — pre-existing eslint errors уже делают этот шаг красным на `main` независимо от
работы этой фазы.

## Validation Architecture

> `workflow.nyquist_validation` не установлен в `false` в `.planning/config.json` (проверено — ключ
> либо отсутствует, либо `true`) → секция обязательна.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Нет фронтенд test-фреймворка (ни vitest, ни playwright, ни jest) — подтверждено `package.json`: только `svelte-check`, `eslint`, `prettier`. Backend — `cargo test` (не затрагивается этой фазой, чисто-фронтенд phase). |
| Config file | Нет (frontend); `.github/workflows/ci-fast.yml`/`ci-full.yml` — CI-уровень |
| Quick run command | `node ui/scripts/check-tokens.mjs` (после появления в plan 1) |
| Full suite command | `cd ui && pnpm svelte-check && node scripts/check-tokens.mjs` (НЕ включать `pnpm lint` целиком, см. Pitfall 2 — eslint часть red независимо от фазы) |

### Честная оценка: что МОЖНО и что НЕЛЬЗЯ проверить автоматически

По D-09 (locked): исполнитель НЕ поднимает браузер — большая часть UI за логином, требует бэкенда с
данными. Это жёсткое архитектурное ограничение, не временная нехватка тулинга. Разбивка:

**Может проверить исполнитель (автоматически, каждый task/commit):**
- **Old-name gate (D-04, rule 1)** — 0 упоминаний `--color-*`/`--space-*`/`--radius-*`/
  `--font-size-*`/`--font-weight-*`/`--line-height-*`/`--shadow-*` (кроме layout-констант) в
  `ui/src` → доказывает DS-01/DS-04's «не осталось старых имён» буквально.
- **Hex-in-`<style>` gate (D-04, rule 2)** — 0 hex-литералов внутри `<style>`-блоков → напрямую
  доказывает SC1 (DS-01) «захардкоженных hex не остаётся».
- **Closed-world existence gate (доп.)** — 0 `var(--tr-*)` ссылок на неопределённые имена → ловит
  опечатки, которые не поймать иначе автоматически.
- **Value-map верификатор (D-08)** — git diff space/radius-планов value-preserving (с учётом
  radius-sm exception) → прямое доказательство SC4 (DS-04) «вёрстка не сдвигается», настолько,
  насколько это доказуемо БЕЗ рендера.
- **`svelte-check`** (0 errors baseline подтверждён) — ловит TS/Svelte 5 rune-ошибки, которые могли
  бы возникнуть при неаккуратном рефакторинге разметки вокруг `.tr-mono` вставок.
- **Мono call-site grep** — подтверждение, что все 7 «чистых» call sites из §DS-03 Mono Coverage
  реально несут класс `tr-mono` (простой grep по каждому конкретному file:line после правки).
- **`git grep` на конкретные QA-01/новый `--shadow-md` баг-файлы** — прямая проверка, что все 4+4+3
  известных сайта резолвятся в определённое имя (не в старое неопределённое).

**НЕ может проверить исполнитель (по архитектурному ограничению D-09, не по лени):**
- Реальный визуальный результат переключения темы (SC2/DS-02) — артефакты типа «вспышка»,
  «нечитаемый текст» физически требуют рендера в браузере/webview с реальными данными за логином.
- Реальное отсутствие сдвига вёрстки (SC4/DS-04) «на глаз» — value-map верификатор доказывает
  **логическую корректность замены**, но не то, как это выглядит с учётом каскада/наследования CSS
  (напр. `!important` где-то дальше по каскаду мог бы теоретически всё равно сдвинуть пиксель, хотя
  в этой кодовой базе `!important` не встречается — не проверялось специально, вне scope research).
- 9-уровневая типографика «на глаз» (часть SC3/DS-03, помимо мono) — какой уровень выбран для
  каждого текстового блока это дизайн-решение (уже принято по value-мар в UI-SPEC), но
  соответствие «выглядит правильно» — визуальная проверка.

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DS-01 | Нет hex в `<style>`, всё на `--tr-*` | static-grep | `node ui/scripts/check-tokens.mjs` | ❌ Wave 0/1 (создать в плане 1 или гейт-плане) |
| DS-02 | Тема переключается без артефактов | manual UAT | — (не автоматизируется, D-09) | N/A |
| DS-03 | 9 уровней типографики + mono на идентификаторах | static-grep (mono) + manual (визуальная иерархия) | `git grep -n 'class="tr-mono"' ui/src` + manual UAT | ❌ Wave 0/1 |
| DS-04 | Space/radius по значению, без сдвига | git-diff verifier | `node ui/scripts/verify-value-map.mjs <base-ref>` | ❌ создать в гейт-плане (D-06) |
| QA-01 | Undefined-токен баги устранены (+ новый `--shadow-md`) | static-grep | `git grep -n -- '--font-size-sm\|--radius-lg\|--shadow-md' ui/src` → 0 совпадений (все переписаны на `--tr-*`) | N/A — grep, не файл |

### Sampling Rate

- **Per task commit:** `node ui/scripts/check-tokens.mjs` (только те правила, что применимы к уже
  переписанным на данный момент семействам — до плана 1 (`_tokens.scss`) гейт не имеет смысла
  запускать вообще).
- **Per wave merge:** `pnpm svelte-check` + полный `check-tokens.mjs` + (для space/radius волны)
  `verify-value-map.mjs` против diff всей волны.
- **Phase gate:** Все static-проверки зелёные + явный список «вне автоматической проверки» пунктов
  (DS-02 визуал, часть DS-03 визуальной иерархии, «на глаз» DS-04) передаётся пользователю на UAT
  (D-09) как явный чеклист, не молчаливо считается done.

### Wave 0 Gaps

- [ ] `ui/scripts/check-tokens.mjs` — не существует, создаётся этой фазой (нет аналогов в проекте —
      REQUIREMENTS.md/CONTEXT.md это явно фиксируют: «Фронтенд-тестов нет вообще»).
- [ ] `ui/scripts/verify-value-map.mjs` — не существует, создаётся этой фазой.
- [ ] Явный список файлов/строк, где токен-миграция физически недоказуема автоматически (DS-02
      визуал, DS-03 визуальная иерархия, toast/modal-title mono-decision из §DS-03) — должен попасть
      в финальный чеклист для `/gsd-verify-work`, а не молчаливо считаться покрытым.

*(Если бы существующая test-инфраструктура покрывала что-то из этого — но она не покрывает вообще
ничего фронтендового, кроме статических eslint/svelte-check/prettier проверок, что подтверждено
`package.json` и прямым запуском обеих команд в этой сессии.)*

## Security Domain

Не применимо в узком смысле ASVS-категорий — фаза не вводит auth/input-validation/crypto изменений
(чисто CSS/визуальный слой, «Изменение поведения/логики» — явно Out of Scope в REQUIREMENTS.md).
Единственная смежная область — `.tr-mono` применяется через `class="tr-mono"` в Svelte-шаблонах, что
не вводит новый XSS-вектор (класс — статическая строка, не пользовательский ввод). SVG-логотип/
XSS-санитизация (ORG-01, Phase 20) уже закрыты в предыдущей фазе и не пересекаются с этой.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `--shadow-md` (3 сайта) должен маппиться на `--tr-elev-2` — рекомендация основана на визуальном сходстве роли («Dropdowns, popovers, tooltips» из UI-SPEC), не на прямом указании в UI-SPEC/CONTEXT (эти файлы там не упомянуты вообще) | §Ground-truth — новый баг `--shadow-md` | Низкий — это косметический выбор уровня тени на 3 карточках картриджных модалок; если планировщик выберет `--tr-elev-1` вместо `--tr-elev-2`, визуальная разница минимальна и легко правится на UAT |
| A2 | Toast/confirm-dialog/modal-title интерполяции номера акта (`` `Акт №${act.number}` ``) НЕ должны получать `.tr-mono` в этой фазе — рекомендация основана на практической сложности изоляции подстроки внутри шаблонного литерала, не на explicit пункте D-13 | §DS-03 Mono Coverage | Средний — если UAT-пользователь ожидает mono ВЕЗДЕ, где встречается номер акта (включая toast), это будет воспринято как gap на верификации; стоит явно подтвердить трактовку на этапе `/gsd-plan-phase` или через checkpoint |
| A3 | `TemplateEditor.svelte`'s справочник переменных шаблона (`device.inventory_no` как текст-подсказка) НЕ является «отображением идентификатора как данных» в смысле D-13 | §DS-03 Mono Coverage | Низкий — это мета-UI (список доступных плейсхолдеров для шаблонов документов), не реальные данные устройства; ошибка в любую сторону не влияет на реальный пользовательский workflow |
| A4 | `DeviceContextMenu.svelte:146` (confirm-диалог с инв.№ внутри предложения) — grey area, не жёстко in/out of scope | §DS-03 Mono Coverage | Низкий — единственное появление, легко добавить или пропустить без больших последствий |

**Если у планировщика уже есть трактовка A2 (самая рискованная позиция) — она должна победить
research-рекомендацию; здесь дана осторожная default-позиция, не финальное решение.**

## Open Questions (RESOLVED)

> Оба вопроса разрешены пользователем 2026-07-17 в ходе `/gsd-plan-phase 23`, до написания планов.

1. **RESOLVED → чинить. См. [D-15](23-CONTEXT.md) + план 23-02, Task 2.**
   **Чинить ли 5 pre-existing eslint errors (Pitfall 2) в рамках этой фазы?**
   - What we know: они блокируют `pnpm lint` целиком уже сегодня, никак не связаны с токенами,
     тривиальный фикс (4 строки в `eslint.config.js`'s `browserGlobals`).
   - What's unclear: входит ли это в «Изменение поведения/логики» (Out of Scope) — формально нет
     (это добавление typing globals, ноль поведенческого изменения), но CONTEXT.md явно не
     затрагивал этот вопрос.
   - Recommendation: чинить как отдельный zero-risk micro-task в гейт-плане (D-06's «план: греп-гейт
     + верификатор»), т.к. без этого `pnpm lint` бесполезен как гейт для ВСЕХ фаз 24–30, не только
     этой.

2. **RESOLVED → one-shot, привязанный к `BASE_SHA` из `23-04-SUMMARY.md`. Не встраивается в
   `pnpm lint`. Реализовано в плане 23-02, Task 2; запускается в 23-04 Task 2 и 23-06 Task 1.**
   **`verify-value-map.mjs` (D-08) — one-shot artifact или постоянный CI-гейт?**
   - What we know: D-08 явно оставляет это на усмотрение планировщика («ценность прежде всего
     одноразовая, во время миграции»).
   - What's unclear: если постоянный — на каком diff-диапазоне его гонять в CI (нет отдельной
     «space/radius-only» ветки после мержа, весь sweep уже будет в history)?
   - Recommendation: one-shot, запускаемый вручную/как часть конкретного плана верификации,
     привязанный к конкретному git-диапазону (`git diff <sha до space/radius-плана>..<sha после>`),
     не встраивать в постоянный `pnpm lint` — там ему после мержа нечего будет проверять (diff
     против main пуст).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Node.js | `check-tokens.mjs`/`verify-value-map.mjs`, `fs.readdirSync({recursive:true})` | ✓ (локально), ✓ (CI, `node-version: '20'`) | локально v22.18.0; CI пинит '20' (последний патч 20.x) | — (recursive readdir доступен с 20.1+, риск теоретический — см. §Standard Stack) |
| pnpm | `pnpm lint`/`pnpm svelte-check`/`pnpm build` | ✓ | 10.17.1 (`packageManager` в package.json) | — |
| git | D-08 value-map верификатор (git diff parsing) | ✓ | — (не проверялась версия, `git diff --unified=0` — стабильный флаг, доступен в любой современной версии) | — |
| cargo | `prebuild` hook (`cargo test -p trackly-app --test export_bindings`) при `pnpm build`/`pnpm --dir ui build` | ✓ | 1.92.0 | Известная гоча из памяти проекта: `pnpm --dir ui build` тянет cargo — учитывать при оценке времени выполнения задач этой фазы, если план требует свежей `ui/dist` для LAN-браузер UAT |

**Missing dependencies with no fallback:** нет.

**Missing dependencies with fallback:** нет — все зависимости присутствуют и подтверждены.

## Sources

### Primary (HIGH confidence — прямой замер в этой сессии)
- `ui/src/**/*.svelte`, `ui/src/styles/*.scss` — прямой grep/Python-скрипт замер (все числа в
  §Ground-truth Measurements, §DS-03 Mono Coverage, §D-07 allowlist)
- `ui/package.json` — скрипты `lint`/`svelte-check`/`build`/`prebuild`
- `ui/vite.config.ts` — подтверждение отсутствия `scss.prependData`
- `.github/workflows/ci-fast.yml`, `.github/workflows/ci-full.yml` — точные строки, где `pnpm lint`
  вызывается (ci-fast:111-113, ci-full:126-129 с `if: runner.os != 'Windows'`)
- Прямой запуск `pnpm lint` (5 pre-existing errors) и `pnpm svelte-check` (0 errors, 48 warnings) в
  этой сессии
- Прямой запуск `node -e "fs.readdirSync('.', {recursive:true})"` — подтверждение API доступности

### Secondary (MEDIUM confidence)
- `.planning/phases/23-design-tokens-foundations/23-CONTEXT.md`, `23-UI-SPEC.md` — источник всех
  locked decisions и value-карт (не пересчитывались, копировались дословно где требовалось)
- `.planning/REQUIREMENTS.md` — baseline-числа замера 2026-07-16, сверены с текущим замером

### Tertiary (LOW confidence)
- Нет — вся research-работа этой сессии опиралась на прямые инструменты (grep/Python/bash), не на
  training data о конкретной кодовой базе

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — новых зависимостей нет, `fs.readdirSync(recursive)` проверен напрямую
- Architecture (grep-gate/value-map verifier design): HIGH — спроектировано и логически провалидировано
  на реальных данных кодовой базы этой сессии (0 false positives подтверждено скриптом)
- DS-03 mono coverage: HIGH для фактов (реальные имена полей/call sites подтверждены grep), MEDIUM
  для категоризации grey-area случаев (toast/confirm — суждение, не факт)
- Pitfalls: HIGH — оба (`pnpm lint` red, `svelte-check` green) подтверждены прямым запуском

**Research date:** 2026-07-17
**Valid until:** численные замеры (§Ground-truth) валидны ~7 дней (кодовая база активно меняется,
как показывает дрейф +2..+9 всего за 1 день с 2026-07-16 до 2026-07-17) — планировщику стоит
перезамерить непосредственно перед написанием планов, если разрыв между research и planning больше
нескольких дней. Архитектурные решения (grep-gate/value-map verifier дизайн) валидны на весь
milestone v1.2 (фазы 24–30 используют тот же гейт).
