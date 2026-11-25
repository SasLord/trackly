# Phase 42: Основа монорепо, ADR и пилот на доменных ПК - Pattern Map

**Mapped:** 2026-11-26
**Files analyzed:** 27 (новые/изменяемые группы файлов)
**Analogs found:** 24 / 27

Приватность: репозиторий публичный. Все примеры ниже - код/конфиги проекта; в новых файлах (ADR, чек-лист, скрипты пилота) только вымышленные значения («Link Pilot Test», «Иванов И.И.»).

## File Classification

| Новый/изменяемый файл | Роль | Поток данных | Ближайший аналог | Качество |
|---|---|---|---|---|
| `pnpm-workspace.yaml`, корневой `package.json`, `pnpm-lock.yaml` | config | - | `ui/package.json` (поля `packageManager`, `engines`) | role-match |
| `packages/design/*` (tokens, breakpoints, global.scss, theme-init.js) | config/style | transform | `ui/src/styles/*`, `ui/public/theme-init.js` | exact (перенос) |
| `packages/kit/*` (компоненты, utils, stores) | component | request-response | `ui/src/lib/components/*`, `lib/stores/{theme,toast,userScoped}.svelte.ts` | exact (перенос) |
| `packages/transport/*` | service | request-response + event-driven | `ui/src/lib/api/{client,ws,errors}.ts` | exact (перенос + хуки) |
| `packages/attachments/*` | component | CRUD/file-I/O | `ui/src/features/attachments/*` | exact (перенос) |
| `ui/vite.config.ts`, `ui/tsconfig.json`, `ui/package.json` (изм.) | config | - | они же | exact |
| `link-ui/` (vite.config, svelte.config, tsconfig, index.html, main.ts, App.svelte, package.json) | component/config | request-response | `ui/vite.config.ts`, `ui/svelte.config.js`, `ui/tsconfig.json`, `ui/index.html`, `ui/src/main.ts` | exact |
| `crates/link-app/Cargo.toml` | config | - | `crates/trackly-app/Cargo.toml` | role-match (версия литералом) |
| `crates/link-app/tauri.conf.json` | config | - | `crates/trackly-app/tauri.conf.json` | exact |
| `crates/link-app/build.rs` | config | - | `crates/trackly-app/build.rs` | exact |
| `crates/link-app/capabilities/main.json` | config | - | `crates/trackly-app/capabilities/main.json` | role-match (сильно урезать) |
| `crates/link-app/src/main.rs` (+ команда `link_info`) | controller/shell | request-response | `crates/trackly-app/src/main.rs` (шапка, `windows_subsystem`) | partial |
| корневой `Cargo.toml` (members, workspace.dependencies) | config | - | он же | exact |
| `.gitignore` (изм.) | config | - | он же | exact |
| `ui/scripts/_roots.mjs` (общий обход) | utility | transform | `ui/scripts/_strip.mjs`, `_rune-harness.mjs` (экспорт `UI`,`SRC`) | role-match |
| `ui/scripts/check-gate-coverage.mjs` + baseline-фикстура | test/gate | batch | `ui/scripts/check-modal-isolation.mjs`, `scripts/fixtures`/`ui/scripts/fixtures` | role-match |
| `ui/scripts/check-workspace-boundaries.mjs` | test/gate | batch | `check-modal-isolation.mjs` (мутанты `--selftest`) | role-match |
| `link-ui/scripts/check-link-layout.mjs` | test/gate | batch | `check-modal-isolation.mjs` | role-match |
| ~25 существующих гейтов `ui/scripts/check-*.mjs` (изм. путей) | test/gate | batch | они же | exact |
| `ui/scripts/_rune-harness.mjs` (изм.: резолвер `@trackly/*`) | utility | transform | он же | exact |
| `ui/package.json` `scripts.lint` (изм.) | config | - | он же | exact |
| `scripts/check-release-isolation.mjs` (+selftest) | test/gate | batch | `scripts/check-write-site-gate-untouched.mjs`, `scripts/check-privacy.selftest.mjs` | role-match |
| `.github/workflows/release-link.yml` | config/CI | batch | `.github/workflows/release.yml` | exact |
| `.github/workflows/ci-fast.yml`, `ci-full.yml`, `release.yml`, `heic-spike.yml` (изм. cache-path/шаги) | config/CI | batch | они же | exact |
| `docs/adr/*.md`, `docs/adr/README.md` | docs | - | `docs/AD-SETUP.md`, `docs/TESTING.md` (только тон/язык) | partial |
| `docs/pilot/{README,install-silent.ps1,diag.ps1,sign-notes.md,RESULT-TEMPLATE.md}` | docs/script | batch | `scripts/setup-hooks.sh`, `.githooks/pre-commit` (стиль шапки) | partial |
| Стенд D-21 (одноразовый `#[ignore]`-тест) | test | request-response | `crates/trackly-app/tests/security_headers.rs::server_serves_embedded_spa_index` | role-match |
| `THIRD-PARTY-NOTICES.md` (изм.) | docs | - | он же | exact |

## Pattern Assignments

### `crates/link-app/Cargo.toml` (config)

**Analog:** `crates/trackly-app/Cargo.toml` (строки 1-33)

Скопировать шапку, но ЗАМЕНИТЬ `version.workspace = true` литералом `version = "0.1.0"` (иначе инъекция `release.yml` перепишет Link, D-10). Perl в release.yml правит только первую `version = "X.Y.Z"` в КОРНЕВОМ `Cargo.toml`, а файл `crates/link-app/Cargo.toml` не трогает.

```toml
[package]
name = "link-app"
version = "0.1.0"            # НЕ version.workspace
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish = false              # нужен для cargo-deny allow-wildcard-paths
description = "Trackly Link: Tauri 2 desktop client shell."

[lints]
workspace = true

[[bin]]
name = "trackly-link"        # прецедент: trackly-app -> bin "trackly"
path = "src/main.rs"

[dependencies]
tauri = { workspace = true }
tauri-plugin-window-state = { workspace = true }   # добавить в [workspace.dependencies] версию 2.5.0
serde = { workspace = true }

[build-dependencies]
tauri-build = { workspace = true }
```

Корневой `Cargo.toml` (строки 1-8): добавить `"crates/link-app"` в `members`; в `[workspace.dependencies]` `tauri`/`tauri-build` уже есть (строки ~38-39) - не дублировать.

---

### `crates/link-app/tauri.conf.json` (config)

**Analog:** `crates/trackly-app/tauri.conf.json` (весь файл, 40 строк)

Ключи, повторяемые один в один: `$schema`, `build.{frontendDist,devUrl,beforeDevCommand,beforeBuildCommand}`, `app.windows[].dragDropEnabled:false`, `bundle.{active,icon,resources}`. Меняются: пути на `link-ui`, порт 1421 (у Trackly 1420), `productName`, `mainBinaryName`, `identifier`, окно, `security.csp` (у Trackly `null` - Link делает строгий), `bundle.targets: ["nsis"]`, нет `macOS.signingIdentity`, нет `plugins.shell`.

Пример из аналога (структура `build`):
```json
"build": {
  "frontendDist": "../../ui/dist",
  "devUrl": "http://localhost:1420",
  "beforeDevCommand": "pnpm --dir ../ui dev",
  "beforeBuildCommand": "pnpm --dir ../ui build"
}
```
Для Link: `"../../link-ui/dist"`, `1421`, `pnpm --dir ../../link-ui dev|build` (RESEARCH: «Фрагмент link-app/tauri.conf.json», готовый вариант). Ресурс `"../../THIRD-PARTY-NOTICES.md": "THIRD-PARTY-NOTICES.md"` взять из аналога как есть. `visible:false` + явный `window.show()` в `setup` как страховка (Pitfall 10).

### `crates/link-app/build.rs` (config)

**Analog:** `crates/trackly-app/build.rs` (весь файл):
```rust
fn main() {
    tauri_build::build()
}
```

### `crates/link-app/capabilities/main.json` (config)

**Analog:** `crates/trackly-app/capabilities/main.json` (строки 1-9 структура; остальное НЕ копировать)
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "main",
  "description": "Trackly Link main window capabilities",
  "windows": ["main"],
  "permissions": ["core:default", "window-state:default"]
}
```
Не переносить `dialog/fs/shell/notification` (ADR: минимальная поверхность).

### `crates/link-app/src/main.rs` (shell, request-response)

**Analog:** `crates/trackly-app/src/main.rs` строки 1-5 (подавление консоли в release):
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
```
Остальное (Paths::resolve, WEBVIEW2_USER_DATA_FOLDER, AppCtx, tokio) НЕ переносить - это portable-серверная логика Trackly. Ядро Link берётся из RESEARCH «Версия WebView2 и путь…» и «Плагин состояния окна» (`link_info`, `tauri_plugin_window_state::Builder::default().build()`, `tauri::generate_handler![link_info]`). Версия - `app.package_info().version`, не `CARGO_PKG_VERSION` workspace.

---

### `link-ui/vite.config.ts`, `svelte.config.js`, `tsconfig.json`, `index.html`, `src/main.ts` (config/component)

**Analogs:** `ui/vite.config.ts`, `ui/svelte.config.js`, `ui/tsconfig.json`, `ui/index.html`, `ui/src/main.ts`

**vite.config.ts** (ui, строки 1-37) - копировать целиком, сменив `port: 1420 -> 1421`, оставив `strictPort: true`, `envPrefix`, `clearScreen:false`, `outDir:'dist'`, `emptyOutDir:true`; убрать alias `$lib` (у Link его нет; `@trackly/*` разрешаются через workspace). Сохранить комментарий про запрет `prependData` (токены только глобально):
```ts
plugins: [svelte({ preprocess: vitePreprocess() })],
resolve: { alias: { $lib: path.resolve(__dirname, 'src/lib') } },   // у Link - убрать
server: { port: 1420, strictPort: true },
build: { outDir: 'dist', emptyOutDir: true },
```
Для `@use '@trackly/design/breakpoints'` добавить `css.preprocessorOptions.scss.loadPaths` (допущение A2 - подтвердить спайком на `Button` в Wave 0).

**svelte.config.js** (ui) - 1:1: `export default { preprocess: vitePreprocess() }`.

**tsconfig.json** (ui, строки 1-28): копировать `compilerOptions` (strict, `verbatimModuleSyntax`, `types: ["svelte","vite/client"]`); `paths` и `include` свои.

**index.html** (ui): сохранить внешний `<script src="/theme-init.js">` в `<head>` (CSP `script-src 'self'`, без inline), `<div id="app">`, `<script type="module" src="/src/main.ts">`; `<title>Trackly Link</title>`.

**main.ts** (ui, строки 1-17) - образец порядка: глобальные стили -> `initTheme()` ДО `mount` -> `mount(App, { target })`, throw если нет `#app`:
```ts
import './styles/global.scss';          // у Link: import '@trackly/design/global.scss'
import { mount } from 'svelte';
import App from './App.svelte';
import { initTheme } from './lib/stores/theme.svelte';   // у Link: из '@trackly/kit'
initTheme();
const target = document.getElementById('app');
if (!target) { throw new Error('Элемент #app не найден в index.html'); }
const app = mount(App, { target });
export default app;
```
**НЕ копировать** из `ui/package.json`: `prebuild` (тянет cargo, Pitfall 6), `pagedjs`, `pdfjs-dist`, `svelte-spa-router`. Скрипт `tauri` - по образцу `ui/package.json`:
```json
"tauri": "cd ../crates/link-app && node ../../link-ui/node_modules/@tauri-apps/cli/tauri.js"
```

**`App.svelte`** (каркас): токены только `--tr-*` (check-tokens Rule 2/4 запрещает hex/rgba в `<style>`); `Tabs`, `ThemeSwitcher` из `@trackly/kit`; паттерн «каждая страница height:100% + min-height:0, скроллит свой регион» (память «App shell scroll pattern»; опорный компонент - `ui/src/App.svelte` / `.content`).

---

### `packages/*` (kit / design / transport / attachments)

**Pattern 1 (source-package)** - манифест из RESEARCH «Pattern 1»; `"name": "@trackly/kit"`, `"private": true`, `exports` с условием `svelte`. Допущение A1: подтвердить сборкой.

**packages/design** - переносить файлы как есть:
- `ui/src/styles/_tokens.scss`, `_breakpoints.scss`, `global.scss`; `global.scss` начинается с `@use './tokens';` (строки 1-8, комментарий про `:root` обязателен к сохранению).
- `ui/public/theme-init.js` -> `packages/design/public/theme-init.js`, копировать в `dist` обоих приложений мини-плагином Vite (`generateBundle` + `fs.readFileSync`, без новой зависимости).

**packages/transport** - исходник `ui/src/lib/api/client.ts`:
```ts
import { parseAppError, type AppError } from './errors';
import { authStore } from '$lib/stores/auth.svelte';       // УБРАТЬ из пакета
import { pushToast } from '$lib/stores/toast.svelte';      // УБРАТЬ из пакета
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
export const HTML_COMMANDS: ReadonlySet<string> = new Set([...]);   // парсится гейтом check-html-commands
export async function apiCall<R>(name, args = {}): Promise<R> { if (isTauri) { ...invoke...; UNAUTHORIZED -> authStore.user=null + '#/login'; FORBIDDEN -> pushToast } ... }
```
Правка: реакции на 401/403 вынести в хуки `onUnauthorized`/`onForbidden`, регистрируемые из `ui/` при старте (Pattern 3). `HTML_COMMANDS` остаётся в `client.ts` (гейт парсит именно этот файл - обновить его путь). Доменные обёртки `api/{acts,devices,groups,organization,templates,adSso}.ts` остаются в `ui/` и импортируют `apiCall` из `@trackly/transport`.

**packages/kit** - переносить компоненты без доменных импортов (список - RESEARCH «Что переносить»). Критерий: файл не импортирует `bindings`, `$lib/api/*` (кроме transport), `auth.svelte`, `features/*`. Четыре компонента с `@use '../../styles/_breakpoints' as bp` переводятся на `@use '@trackly/design/breakpoints' as bp`.

**packages/attachments** - переносить `ui/src/features/attachments/*` (10 файлов). `features/requests/chat/*` остаётся в `ui/`; его импорты `features/attachments/` перенаправить на `@trackly/attachments` (D-19). Типы DTO - структурные интерфейсы внутри пакета + файл `ui/src/packages-compat.ts` с `satisfies` на типы из `bindings` (Pattern 4, допущение A3).

**Антипаттерн:** не оставлять реэкспорт-прокладки `export * from '@trackly/kit'` в `ui/src/lib/...` - гейты будут сканировать прокладку. Замену специфайеров делать кодмодом (495 импортов `$lib/components`, 159 `$lib/stores`, 116 `$lib/api`, 55 `$lib/utils`).

---

### Гейты: общий шаблон нового `check-*.mjs`

**Analog:** `ui/scripts/check-modal-isolation.mjs` (шапка 1-45, импорты 36-44, мутационный `--selftest`)

**Структура шапки/импортов** (строки 36-44):
```js
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';
export { stripComments };

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-modal-isolation]';
```
Правила оформления, обязательные для новых гейтов:
- Шапка-комментарий: ЗАЧЕМ, нумерованные правила `[X1]`, чего гейт НЕ видит, Usage (`--src=<root>`, `--selftest`).
- Только `node:*` (zero-dependency); комментарии вырезаются `stripComments` из `_strip.mjs` (однопроходный, длина и переводы строк сохраняются).
- `--selftest`: мутанты РЕАЛЬНОГО файла; якорь замены обязан встречаться ровно один раз (память «Якорь мутационной проверки»); сверяется ожидаемый код нарушения.
- SKIP недопустим: отсутствие файла/нулевой скан - нарушение (D-20: «0 файлов = сломанный гейт»); PASS печатает число просканированных файлов.
- Новый гейт обязан быть подключён в `ui/package.json` `scripts.lint` ДВУМЯ вызовами: `node scripts/<f>.mjs --selftest && node scripts/<f>.mjs` - иначе `check-lint-selftest-wiring` [W1]/[W2] упадёт.

**Подключение в lint** (`ui/package.json`, пример пары, конец цепочки):
```
... && node scripts/check-import-automap.mjs --selftest && node scripts/check-import-automap.mjs && node scripts/check-lint-selftest-wiring.mjs --selftest && node scripts/check-lint-selftest-wiring.mjs
```
Новые пары вставлять ПЕРЕД `check-lint-selftest-wiring`. Если гейт живёт вне `ui/scripts` (`link-ui/scripts/check-link-layout.mjs`, `scripts/check-release-isolation.mjs`) - его подключение `check-lint-selftest-wiring` не держит; добавить шаг в `ci-fast.yml` по образцу шага «Root gate selftests».

### `ui/scripts/_roots.mjs` (utility)

**Analog:** `ui/scripts/_rune-harness.mjs` строки 22-24 (экспорт корней) и `_strip.mjs` (имя с `_` - не попадает под `GATE_RE = /^check-[\w.-]+\.mjs$/`):
```js
const __dirname = path.dirname(fileURLToPath(import.meta.url));
export const UI = path.resolve(__dirname, '..');
export const SRC = path.join(UI, 'src');
```
Расширить: `REPO`, `UI_ROOT`, `SCAN_ROOTS = [ui/src, packages/*/src, link-ui/src]`, `walk()`. 25 гейтов, обходящих `src` (список - RESEARCH «Перенос гейтов»), переводятся на него. Точечные константы путей (`MODAL_FILE = 'src/lib/components/Modal.svelte'` в `check-modal-isolation`) обновляются по таблице RESEARCH.

### `ui/scripts/check-gate-coverage.mjs` + `fixtures/gate-coverage.baseline.json`

**Analog:** `check-lint-selftest-wiring.mjs` (инвентарь ОТ каталога, а не вкомпилированный список; `[W0]` защита от вакуума, строки 55-72):
```js
const gates = fs.readdirSync(scriptsDir).filter((f) => GATE_RE.test(f)).sort();
if (gates.length === 0) { failures.push('[W0] ... разбор пуст'); }
```
Берётся тот же приём: список гейтов из каталога, N файлов в скане >= baseline, множество путей (ui + packages) совпадает с baseline модуло переименование каталогов. Baseline снимается ДО переезда (Wave 0). Фикстуры гейтов лежат в `ui/scripts/fixtures/<gate-name>/` - baseline положить рядом как один JSON.

### `ui/scripts/check-workspace-boundaries.mjs`

**Analog:** `check-modal-isolation.mjs` (правила + мутанты). Правила: пакеты не импортируют `ui/`, `bindings*`, `@tauri-apps/*` (кроме `transport`); `link-ui` не импортирует `ui/`; ровно один `Button.svelte` вне `node_modules`; `crates/link-app/Cargo.toml` без `version.workspace`; `jq -r .version` для `tauri.conf.json` Link не равен версии workspace-инъекции.

### `ui/scripts/_rune-harness.mjs` (изменение)

**Analog:** он же, хук резолвера (строки 33-52):
```js
if (spec.startsWith('$lib/')) base = path.join(cfg.src, 'lib', spec.slice(5));
else if ((spec.startsWith('./') || spec.startsWith('../')) && ctx.parentURL && ctx.parentURL.startsWith('file:'))
  base = path.resolve(path.dirname(fileURLToPath(ctx.parentURL)), spec);
if (base !== null) { ...; const p = withTs(...); if (p !== null && p.startsWith(cfg.src)) return {...}; }
```
Добавить ветку `@trackly/<pkg>` -> `packages/<pkg>/src`, учесть условие `p.startsWith(cfg.src)` (пакеты вне `cfg.src` - расширить проверку на список корней), обновить `stubs`. Затрагивает 3 поведенческих гейта: `check-chat-notify-behavior`, `check-unread-store-behavior`, `check-open-external`; после правки прогнать их `--selftest` и мутанты.

---

### `.github/workflows/release-link.yml` (CI, batch)

**Analog:** `.github/workflows/release.yml`

Что ПЕРЕНЕСТИ дословно:
- `workflow_dispatch` с `input version` (release.yml, строки 11-16), `permissions` (у Link `contents: read`).
- Шаг NSIS pre-fetch с ретраями (release.yml ~ строки 186-216): URL/SHA1 синхронизировать с версией CLI.
- Шаги `pnpm/action-setup@v4 (version: 10)`, `setup-node@v4 (node 20, cache: pnpm)`, `pnpm install --frozen-lockfile`; `cache-dependency-path` - корневой `pnpm-lock.yaml`.
- Паттерн «INPUT_VERSION через env, не inline `${{ }}`» (CR-01, защита от shell-инъекции) и проверка точной строки, а не X.Y.Z-регэкспа:
```bash
perl -0pi -e 's/^(version = ")[0-9]+\.[0-9]+\.[0-9]+(")/${1}'"${VERSION}"'${2}/m' crates/link-app/Cargo.toml
jq --arg v "$VERSION" '.version = $v' crates/link-app/tauri.conf.json > /tmp/_t.json && mv /tmp/_t.json crates/link-app/tauri.conf.json
grep -Fq "version = \"${VERSION}\"" crates/link-app/Cargo.toml || { echo "ERROR"; exit 1; }
test "$(jq -r .version crates/link-app/tauri.conf.json)" = "${VERSION}" || { echo "ERROR"; exit 1; }
```
(Perl-регэксп допускает только `X.Y.Z`; для `0.1.0-dryrun` в dry-run подправить регэксп или шаблон, как в комментарии release.yml про pre-release.)
- Тулчейн `dtolnay/rust-toolchain@stable` с `toolchain: '1.92'`, `Swatinem/rust-cache@v2`.
- `tauri-action` с `projectPath` и `tauriScript` (закреплённый CLI): у Link `projectPath: crates/link-app`, `tauriScript: node ${{ github.workspace }}/link-ui/node_modules/@tauri-apps/cli/tauri.js`.

Что НЕ переносить: job `create-release`, `releaseId`, `releaseDraft`, matrix macOS/Linux, vcpkg/`--features heic`, Linux-пакеты, job `checksums`. Dry-run = `upload-artifact` (retention 7 дней), без Release. Триггеры: ТОЛЬКО `workflow_dispatch` в Ф.42, плюс при необходимости `tags: ['link-v*.*.*']` с guard `if: startsWith(github.ref_name,'link-v') || github.event_name == 'workflow_dispatch'`.

Образец изоляции в release.yml, который гейт проверяет: `on.push.tags: ['v*.*.*']` (строки 7-10).

### `.github/workflows/ci-fast.yml` и др. (изменения)

**Analog:** сам `ci-fast.yml`. Точки правки:
- `cache-dependency-path: ui/pnpm-lock.yaml` -> `pnpm-lock.yaml` (ci-fast, ci-full, release, heic-spike - Pitfall 4; одним коммитом с переездом lockfile). Шаг `pnpm install --frozen-lockfile` с `working-directory: ui` продолжает работать (workspace ищется вверх), лучше запускать из корня.
- Шаг «Seed ui/dist placeholder» - скопировать дубль для `link-ui/dist` (Pitfall 5; `tauri::generate_context!` проверяет `frontendDist` при `cargo clippy/test --workspace`):
```bash
mkdir -p link-ui/dist
[ -f link-ui/dist/index.html ] || printf '<!doctype html><title>placeholder</title>\n' > link-ui/dist/index.html
```
- Добавить шаги `pnpm --dir link-ui build`, `svelte-check` для `link-ui`/`packages`, и в блок «Root gate selftests» - `node scripts/check-release-isolation.mjs --selftest && node scripts/check-release-isolation.mjs`.
- `release.yml`: `tauriScript: .../ui/node_modules/@tauri-apps/cli/tauri.js` остаётся верным (pnpm кладёт симлинк прямых зависимостей в `ui/node_modules`); правится только кеш-путь. Триггер `v*.*.*` и инъекция версии НЕ менять.

### `scripts/check-release-isolation.mjs` (корневой гейт)

**Analog:** `scripts/check-privacy.selftest.mjs` и `scripts/check-write-site-gate-untouched.mjs --selftest` (самодостаточные, временные каталоги). Парсит оба workflow: триггеры не пересекаются, в `release-link.yml` нет `releaseId`/`createRelease`/`softprops`, у Link нет `v*.*.*`, у Trackly нет `link-v`. Шапка и CLI (`--selftest`) - как у `check-privacy.mjs` (строки 1-60: режимы, fail-closed, печать только «путь:строка - маркер», zero-dependency).

---

### Стенд D-21 (одноразовый `#[ignore]`)

**Analog:** `crates/trackly-app/tests/security_headers.rs` строки 264-314 (`server_serves_embedded_spa_index`) - единственный живой образец `build_test_components()` + `router.oneshot(...)` против встроенного `ui/dist`. Предыдущих `#[ignore]`-стендов в HEAD нет (одноразовые, не коммитились) - писать вне репозитория или помечать `#[ignore]` и не подключать в suite.

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn server_serves_embedded_spa_index() {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (router, ctx) = build_test_components().await.expect("build_test_components failed");
        let res = router.oneshot(Request::builder().method("GET").uri("/").body(Body::empty()).expect("build request")).await.expect("oneshot");
        ...
        ctx.shutdown.cancel();
    }).await.expect("... exceeded 30s budget");
}
```
Для живого браузера вместо `oneshot` нужен настоящий листенер (`start_server_on_addr` из `trackly_app::server`, импортируется в `main.rs`, строки 33-35). Тестовые пользователи - вымышленные: `login: "hdr_admin"`, `full_name: "Иванов И.И."` (строки 327-335). Новый файл теста НЕ вносить в `tests/suites/*.rs` (suite-бинарники подключают файлы через `#[path]`), иначе `suite_wiring` потребует учёта; стенд запускается точечно `-- --ignored`. На macOS - префикс `DEVELOPER_DIR=/Library/Developer/CommandLineTools`.

Нужен снимок консоли ДО переезда (Wave 0): без него «ноль ошибок» не с чем сравнивать. Стенд идёт по HTTP-пути `apiCall` (fetch), ветка Tauri-`invoke` в `@trackly/transport` остаётся без живой проверки - помечать UNVERIFIED в десктопе.

---

### `docs/pilot/*` (docs/scripts)

**Analog (стиль):** `scripts/setup-hooks.sh`, `.githooks/pre-commit` (шапка-объяснение «зачем / чего не делает», fail-closed). Структуры в репозитории для PowerShell нет - содержимое строить по RESEARCH разделу «Набор для пилота» (ключи реестра WebView2 `pv`, события AppLocker 8002-8007, `Start-Process /S "/D=..."`). Вывод `diag.ps1` - JSON в `$env:TEMP` БЕЗ `COMPUTERNAME`/`USERNAME`/IP/домена/Subject сертификата. Тестовый CN - «Link Pilot Test». Проверка перед коммитом:
```
node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256
```

### `docs/adr/*.md`

**Analog:** `docs/AD-SETUP.md`, `docs/TESTING.md` (русский, заголовки `#`/`##`, блоки кода). Формата ADR в репозитории нет - использовать RESEARCH «ADR» (MADR-подобно: Статус / Контекст / Решение / Последствия / Отвергнутые варианты / Источники; индекс `docs/adr/README.md`; нумерация `NNNN-<slug>.md`). Таблица закрытия расхождений (keyring-core vs keyring; `link-app` vs `trackly-client`; FTS `trigram` + нормализация `ё->е`; откат версии; путь установки `/D=`; `%APPDATA%` для window-state; `trackly-proto` вне Ф.42) - в RESEARCH, переносится в ADR без изменений.

---

## Shared Patterns

### Гейты проходят в `pnpm lint` парами `--selftest` + прогон
**Source:** `ui/package.json` `scripts.lint`; контроль - `ui/scripts/check-lint-selftest-wiring.mjs` (строки 55-72: `[W0]`, `[W1]`, `[W2]`)
**Apply to:** каждый новый `ui/scripts/check-*.mjs`. `lint` НЕ включает `svelte-check`, `cargo clippy`, `cargo fmt` - это отдельные шаги (память «UI-гейты - разные команды»); исполнители обязаны гонять все.

### Одноимённые приёмы мутационного selftest
**Source:** `ui/scripts/check-modal-isolation.mjs` (шапка, строки 28-31)
**Apply to:** все новые гейты и все 25 гейтов с правленными путями: после правки путей прогнать КАЖДЫЙ `--selftest` и пару ручных мутаций (Pitfall 2); якорь `replace(...,1)` уникален.

### Токены дизайн-системы, не `prependData`
**Source:** `ui/vite.config.ts` (комментарий строки 8-13), `ui/src/styles/global.scss` (строки 1-8)
**Apply to:** `link-ui`, `packages/design`. Токены подключаются один раз глобально через `import '@trackly/design/global.scss'`; в `<style>` компонентов только `var(--tr-*)`; hex/rgba запрещены гейтом `check-tokens` (он должен сканировать и `packages/*/src`).

### Порядок старта UI
**Source:** `ui/src/main.ts` (строки 1-17) - стили -> `initTheme()` до `mount` -> проверка `#app`.
**Apply to:** `link-ui/src/main.ts`.

### Изоляция версии Link от инъекции release.yml
**Source:** `.github/workflows/release.yml` (шаг «Extract version and inject», perl по КОРНЕВОМУ `Cargo.toml` + `jq` по `crates/trackly-app/tauri.conf.json`)
**Apply to:** `crates/link-app/Cargo.toml` (версия литералом), `release-link.yml` (своя инъекция только по файлам Link), гейт границ.

### Приватность публичного репозитория
**Source:** `scripts/check-privacy.mjs` (режим 2, хэши n-грамм), `.githooks/pre-commit`, шаг «Privacy gate» в `ci-fast.yml`
**Apply to:** все новые документы/скрипты фазы (ADR, `docs/pilot/*`, `.planning/`). Гейт ловит только известное, не ФИО - ручная сверка обязательна; в тестах/примерах только «Иванов И.И.», «Петров П.П.».

### Placeholder для `frontendDist`
**Source:** `ci-fast.yml` шаг «Seed ui/dist placeholder»
**Apply to:** ci-fast, ci-full для `link-ui/dist` (иначе `generate_context!` валит `cargo clippy/test --workspace`).

## No Analog Found

| Файл | Роль | Поток | Причина |
|---|---|---|---|
| `pnpm-workspace.yaml` | config | - | В репозитории нет workspace; брать RESEARCH «Корневой workspace» (`packages: [ui, link-ui, packages/*]`) |
| `docs/pilot/diag.ps1`, `install-silent.ps1` | script | batch | PowerShell в репозитории отсутствует; источник - RESEARCH «Набор для пилота»; синтаксис проверять `pwsh` Parser при наличии, иначе пометка |
| Иконки Link (`crates/link-app/icons/*`) | asset | - | Генерируются `tauri icon` из исходного PNG (знак Trackly + метка связи, D-12); аналог - `crates/trackly-app/icons/` только по составу набора (`32x32.png`, `128x128.png`, `icon.ico`) |
| Стенд браузерной проверки (коммитимая версия) | test | request-response | Прежние стенды одноразовые и в HEAD не сохранились; ближайший - `security_headers.rs` (см. выше) |

## Metadata

**Analog search scope:** `crates/trackly-app/{src/main.rs,Cargo.toml,tauri.conf.json,build.rs,capabilities,tests}`, `ui/{package.json,vite.config.ts,svelte.config.js,tsconfig.json,index.html,public,src/main.ts,src/lib/api,src/lib/stores,src/styles,scripts}`, `scripts/`, `.github/workflows/`, `docs/`, корневой `Cargo.toml`, `.gitignore`
**Files scanned:** ~30 прочитано, ~320 просмотрено списком (в том числе 72 гейта `check-*.mjs`)
**Pattern extraction date:** 2026-11-26
