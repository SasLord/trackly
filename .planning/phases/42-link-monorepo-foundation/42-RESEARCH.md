# Phase 42: Основа монорепо, ADR и пилот на доменных ПК - Research

**Researched:** 2026-11-26
**Domain:** pnpm-монорепо (Svelte 5 / Vite 6), второй Tauri-2-крейт, отдельный release-workflow, ADR, набор для пилота на доменных Windows-ПК
**Confidence:** MEDIUM (состояние репозитория и версии - HIGH; платформенные факты Windows - MEDIUM до живого пилота)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Целевые рабочие ПК - Windows 10 **только 64-бит**: Link собирается одной сборкой x64, сборка x86 не нужна. Плюс терминальный сервер RDS (версия Windows Server уточняется в пилоте); RDS поддерживается в вехе (решение 2026-11-26).
- **D-02:** Пилот: 2-3 обычных ПК под сотрудниками **без прав администратора** + 1 RDS-сервер с несколькими пользовательскими сеансами.
- **D-03:** Формат пилота - **набор для пилота**, который готовит Claude: тестовая сборка каркаса Link из CI (NSIS `currentUser`), скрипт тихой установки (как будет вызывать скрипт входа GPO), PowerShell-скрипт диагностики (версия WebView2 Runtime, события AppLocker/SRP из журнала, путь установки, разрядность, сеанс RDS) и чек-лист. Пользователь прогоняет его в домене и присылает итог; итог записывается в документацию **обезличенно** (без названия организации, имён ПК, пользователей, IP).
- **D-04:** В домене действуют ограничения запуска (AppLocker/SRP), пользователь настраивает их сам. Если запуск из `%LOCALAPPDATA%` запрещён - **добавляем разрешающее правило** (предпочтительно по издателю/подписи), остаёмся на варианте A (установка на пользователя). Переход на вариант B не рассматривается без нового решения пользователя.
- **D-05:** Наличие AD CS неизвестно - набор пилота покрывает **оба** варианта подписи тестовой сборки: сертификатом подписи кода из AD CS (если есть) и самоподписанным сертификатом с раздачей доверия через GPO; чек-лист проверяет правило AppLocker «по издателю» для обоих. Закрытые ключи и сертификаты в репозиторий не попадают.
- **D-06:** Чек-лист пилота проверяет минимум: тихая установка без прав админа; запуск из `%LOCALAPPDATA%\Programs\Trackly Link\trackly-link.exe` под AppLocker/SRP; наличие/версия WebView2; окно каркаса открывается; на RDS два пользователя одновременно запускают свои экземпляры; удаление.
- **D-07:** Пилот **не блокирует закрытие Фазы 42** - если не пройден к концу фазы, он оформляется как отложенная human-UAT. Но он **блокирует планирование Фазы 45** (дистрибуция): без итогов пилота Фазу 45 не планировать.
- **D-08:** Системное имя приложения - **«Trackly Link»** везде, где его видит ОС: меню «Пуск», «Приложения и возможности», подсказка иконки в трее, заголовок системных уведомлений. Внутри окна логотип - просто **«Link»**.
- **D-09:** Каталог установки - `%LOCALAPPDATA%\Programs\Trackly Link\`, исполняемый файл - **`trackly-link.exe`** (не `link.exe` - совпадает с линковщиком MSVC). Идентификатор приложения (bundle identifier) - на усмотрение Claude, но стабильный с первой сборки (от него зависят AUMID уведомлений и пути данных).
- **D-10:** Версии Link - **собственная нумерация, независимая от Trackly**: начинается с **0.1.0**; 0.x - пилот и ранние сборки; **1.0.0** - когда веха v1.5 закрыта и Link раскатывается на всех. Крейт Link не наследует `version.workspace` (иначе инъекция версии в `release.yml` перепишет и его).
- **D-11:** Выпуск Link - **отдельный CI-процесс** (`release-link.yml`) на собственные теги (например, `link-v*.*.*`), который не срабатывает на теги Trackly `v*.*.*` и не запускает `release.yml`. В Фазе 42 - dry-run (сборка артефакта без публикации релиза).
- **D-12:** Иконка и логотип - **от знака Trackly с меткой связи** (семейное сходство), читаемы в трее при 16 px. Финальная прорисовка может уточниться в Фазе 46, но каркас уже идёт с этим знаком, не с заглушкой.
- **D-13:** Цвета - **токены дизайн-системы Trackly**, светлая/тёмная тема следует системной, переключатель темы как в Trackly. Своего акцентного цвета у Link нет.
- **D-14:** Окно - **обычное окно**, по умолчанию ~1000x700, минимум ~720x500, **запоминает размер и положение** между запусками.
- **D-15:** Раскладка (LNK-10): узкий логотип «Link» слева вверху, справа от него вкладки (в v1.5 одна - «Чат»), сайдбар слева, область содержимого справа.
- **D-16:** Пустой каркас (до появления чата) показывает **заглушку «Здесь скоро будет чат»** и **служебную строку для пилота**: версия Link, версия WebView2, путь установки. Служебная строка - не постоянная функция; её судьбу решит Фаза 46/48.
- **D-17:** В Фазе 42 в общие пакеты выносится **сразу всё общее**, а не только минимум для каркаса: токены/стили, базовые компоненты (kit), API-клиент/транспорт, вложения (просмотр и тайлы) и прочее, что будет нужно Link. Раскладка пакетов - по исследованию (`packages/design`, `kit`, `transport`, `attachments`, ...) на усмотрение планировщика.
- **D-18:** Фронтенд Trackly **остаётся в `ui/`** (пути, сборка, `release.yml`, стенды, документация не меняются); рядом появляются `packages/` и UI-сборка Link (например, `link-ui/`); pnpm workspace на корне.
- **D-19:** **Чат заявок (`ui/src/features/requests/chat/`) не переносится** - остаётся в `ui/`. Новый чат строится на отдельной модели; чат заявок переедет в v1.6 вместе с вкладкой «Заявки» в Link.
- **D-20:** Все существующие гейты `ui/scripts/check-*.mjs` (~70, с `--selftest`) **должны продолжить проверять тот же код** после переезда: пути в гейтах обновляются вместе с файлами, а гейт, который после переезда молча находит 0 файлов, считается сломанным. `pnpm lint`, `svelte-check`, `pnpm build` и CI (ci-fast) - зелёные.
- **D-21:** Проверка, что переезд ничего не сломал в Trackly, - **автоматические гейты + агентский прогон на стенде**: агент поднимает настоящий сервер с собранным `ui/dist` (одноразовый `#[ignore]`-стенд, как в прошлых фазах) и прокликивает ключевые разделы Trackly во встроенном браузере, проверяя консоль на ошибки. Ручной проход пользователя не требуется; десктопная сборка Trackly при этом остаётся непроверенной живьём - это допущение, а не «проверено».
- **D-22:** Пишется ADR (один или несколько, в `docs/`) с решениями, принятыми в обсуждении вехи 2026-11-25/26 и исследовании: вся сеть Link - в Rust-части (webview без сети), закрепление SPKI-отпечатка сертификата сервера, задаваемого установщиком/конфигом; хранение токена в хранилище ОС; отдельная модель чата `im_*`, не наращивание `chat_*` заявок; доставка «сигнал + догрузка по курсору»; откат версии = выпуск с большим номером (запасной - `version_comparator`); каналы обновлений только на сервере (компьютер приоритетнее пользователя); вариант A распространения; поддержка RDS (single-instance на пользователя сеанса). Расхождения исследования (`keyring-core` vs `keyring`, имя крейта, токенайзер FTS) закрываются в ADR.

### Claude's Discretion
- Имена крейтов и пакетов (`link-app`/`trackly-link`, `packages/*`), bundle identifier (стабильный с первой сборки), точная раскладка общих пакетов, формат ADR (один файл или несколько).
- Механика pnpm workspace и Vite-конфигов, способ, которым `tauri build` Link получает свою UI-сборку.
- Состав и формат PowerShell-диагностики пилота.

### Deferred Ideas (OUT OF SCOPE)
- Маскот «Васька» (заставка пустого чата, «О программе») - к дизайну Фазы 46, если пользователь захочет.
- Перенос чата заявок в общий пакет - v1.6 (вкладка «Заявки» в Link).
- Не входит в фазу: трей, автозапуск, single-instance, вход, сеть, закрепление сертификата (Ф.44); установщик для GPO, подпись, автообновление, каналы (Ф.45); доставка событий (Ф.43); любой чат (Ф.47-51).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LNK-01 | Link собирается из этого же репозитория отдельным приложением со своей версией; компоненты чата и вложений общие | Разделы «pnpm-монорепо», «Второй Tauri-крейт», «release-link.yml», «Перенос гейтов», «Валидация» |
| LNK-10 | Окно Link: узкий логотип слева вверху, вкладка «Чат» справа, сайдбар слева, содержимое справа | «Каркас окна Link», window-state, служебная строка D-16 |
| DST-06 | Пилот на реальных доменных ПК, итог обезличенно | «Набор для пилота», ADR, формат отложенной UAT (D-07) |
</phase_requirements>

## Summary

Репозиторий сейчас НЕ монорепо: pnpm-workspace нет, весь фронтенд - один пакет `ui/` (`trackly-ui`, lockfile `ui/pnpm-lock.yaml`). Токены подключаются глобально через `@use './tokens'` в `global.scss` (не через `prependData`), `$lib` - алиас на `ui/src/lib` в `vite.config.ts` и `tsconfig.json`. 72 гейта `ui/scripts/check-*.mjs` написаны от `UI_ROOT` и захардкожены на пути `src/lib/...`, `src/features/...`, `src/styles/...`; 25 из них обходят дерево `src` рекурсивно. Главный риск фазы - не сам перенос файлов, а **тихая потеря покрытия**: гейт, обходящий `ui/src`, после выноса компонентов в `packages/` увидит меньше файлов, но не нуль - и останется зелёным (D-20 ловит только «0 файлов»).

Критическое наблюдение по раскладке: «общее» не равно «лежит в `lib/components`». Из 40 компонентов около 12 тянут домен (`../../bindings`, `$lib/api/acts`, `features/devices/...`, `auth`-стор). Общие пакеты не могут импортировать `ui/src/bindings.ts` (файл генерируется cargo-тестом и в `.gitignore`; у Link его нет). Перенос «всего» по D-17 надо трактовать как «всего, что не зависит от домена Trackly»; доменно-связанные компоненты остаются в `ui/`.

Второе критическое наблюдение: **Tauri NSIS `currentUser` ставит в `%LOCALAPPDATA%\<productName>`, а не в `%LOCALAPPDATA%\Programs\<productName>`** (шаблон `installer.nsi`: `StrCpy $INSTDIR "$LOCALAPPDATA\${PRODUCTNAME}"`). D-09/D-06 предполагают `Programs\`. Решение: скрипт тихой установки передаёт `/D=%LOCALAPPDATA%\Programs\Trackly Link` явно; чек-лист пилота фиксирует фактический путь. Обновления (Ф.45, `/UPDATE`) берут путь из реестра предыдущей установки и каталог не меняют.

**Primary recommendation:** корень = `pnpm-workspace.yaml` + единый `pnpm-lock.yaml` в корне; `packages/{design,kit,transport,attachments}` как «source packages» (экспортируют `.svelte`/`.ts`/`.scss` напрямую, без собственной сборки; компилирует Vite потребителя); `link-ui/` (Vite+Svelte) и `crates/link-app` (бинарь `trackly-link`); гейты не трогать «по месту», а перевести на общий модуль корней сканирования + ведомость покрытия «файлов до/после».

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Дизайн-токены, темы, глобальные стили | Browser/Client (`packages/design`) | - | Чистый SCSS; подключается в `main.ts` каждого приложения |
| Базовые компоненты (kit) | Browser/Client (`packages/kit`) | - | Без домена и без сети; рендерятся в двух webview |
| Транспорт (контракт `apiCall`/события) | Browser/Client (`packages/transport`) | API/Backend (реализация в Rust Link, Фаза 44) | Webview Link сети не видит (ADR); пакет несёт интерфейс и реализацию Trackly (invoke/fetch) |
| Вложения (просмотр, тайлы) | Browser/Client (`packages/attachments`) | API/Backend | UI общий; источник байт задаётся транспортом |
| Окно, размер/положение, версия, путь установки, версия WebView2 | Desktop shell (Rust, `link-app`) | Browser/Client | Это знание ОС; webview получает через Tauri-команду |
| Сборка, версия, выпуск Link | CI (`release-link.yml`) | - | Изоляция от `release.yml` |
| Пилот (установка, AppLocker/SRP, WebView2) | Среда заказчика (домен) | CI даёт артефакт | Проверяется только живьём |
| ADR | `docs/` | - | Документ |

## Standard Stack

### Core (всё уже в репозитории - версии из Cargo.lock/package.json)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tauri | 2.12.7 (Cargo.lock) | оболочка Link | та же, что у Trackly; `[VERIFIED: Cargo.lock]` |
| tauri-build | 2.7.6 (Cargo.lock) | `build.rs` Link | `[VERIFIED: Cargo.lock]` |
| @tauri-apps/cli | `^2.11.0` в ui/package.json (npm latest 2.12.6) | `tauri build` | release.yml закрепляет CLI из lockfile `[VERIFIED: ui/package.json, npm view]` |
| svelte | ^5.55.0 | UI | `[VERIFIED: ui/package.json]` |
| vite | ^6 + @sveltejs/vite-plugin-svelte ^5.1.1 | сборка | `[VERIFIED: ui/package.json]` |
| sass | ^1.80 | SCSS | `[VERIFIED: ui/package.json]` |
| pnpm | 10.17.1 (`packageManager`), CI `version: 10` | workspace | `[VERIFIED: ui/package.json, pnpm --version]` |

### Supporting (НОВОЕ в фазе)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tauri-plugin-window-state | 2.5.0 (стабильная; 3.0.0-alpha.2 - НЕ брать) | D-14: запоминание размера/положения | `link-app`; MSRV плагина проверить `cargo check` под 1.92 `[VERIFIED: crates.io API]` |
| @tauri-apps/plugin-window-state | 2.5.0 | JS-часть - НЕ нужна, если регистрация только из Rust | не добавлять в npm без причины |

Плагины single-instance/autostart/notification/updater/opener - НЕ в Фазе 42 (Ф.44/45).

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Source-packages (без сборки пакета) | `svelte-package`/dist-пакеты | Source-packages: нет отдельного шага сборки, HMR сквозной, SCSS/runes компилирует один Vite. dist-пакеты добавляют шаг и рассинхрон. Брать source-packages |
| Единый корневой lockfile | `shared-workspace-lockfile=false` | Per-package lockfile сохранил бы `ui/pnpm-lock.yaml`, но ломает `workspace:*` и сам смысл монорепо. Брать корневой, поправить 4 workflow |
| Крейт `link-app` | `trackly-link` | См. «Расхождения ADR» - брать `link-app` + `[[bin]] name = "trackly-link"` (прецедент: `trackly-app` -> bin `trackly`) |

**Installation (новое):**
```bash
cargo add -p link-app tauri-plugin-window-state@2.5.0   # через workspace.dependencies
pnpm install                                            # из корня, после создания pnpm-workspace.yaml
```
**Version verification:** `cargo search tauri-plugin-window-state`, `npm view @tauri-apps/plugin-window-state version` (2.5.0, обновлён 2026-09-30) выполнены 2026-11-26.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| tauri-plugin-window-state | crates.io | с 2022-06 | ~5.9M | github.com/tauri-apps/plugins-workspace | [OK] | Approved |
| @tauri-apps/plugin-window-state | npm | с 2022 | - | github.com/tauri-apps/plugins-workspace | [OK] | Approved (не нужен, если только Rust) |
| keyring-core | crates.io | - | ~9.3M | github.com/open-source-cooperative/keyring-core | [OK] | Approved - ТОЛЬКО для ADR, в Ф.42 не ставится |
| windows-native-keyring-store | crates.io | - | ~5.0M | open-source-cooperative | [OK] | то же (ADR) |
| apple-native-keyring-store | crates.io | - | ~4.9M | open-source-cooperative | [OK] | то же (ADR) |
| zbus-secret-service-keyring-store | crates.io | - | ~5.4M | open-source-cooperative | [OK] | то же (ADR) |

**Packages removed ([SLOP]):** none. **Packages flagged ([SUS]):** none. `postinstall` у npm-пакета не обнаружен (`npm view ... scripts.postinstall` пусто). Новых npm-зависимостей кроме workspace-ссылок фаза не добавляет. Запись в `THIRD-PARTY-NOTICES.md` для `tauri-plugin-window-state` (MIT/Apache-2.0) - добавить.

## Architecture Patterns

### System Architecture Diagram

```
                       pnpm-workspace.yaml (корень)
        ┌──────────────────────────────────────────────────────────┐
        │ packages/design  packages/kit  packages/transport        │
        │ packages/attachments        (source-packages, workspace:*)│
        └───────┬───────────────────────────┬──────────────────────┘
                │ import (Vite компилирует  │
        ┌───────▼────────┐          ┌───────▼─────────┐
        │ ui/  (Trackly) │          │ link-ui/ (Link) │
        │ vite build     │          │ vite build      │
        │ -> ui/dist     │          │ -> link-ui/dist │
        └───────┬────────┘          └───────┬─────────┘
                │ rust-embed + frontendDist │ frontendDist
        ┌───────▼────────────┐      ┌───────▼───────────────┐
        │ crates/trackly-app │      │ crates/link-app       │
        │ tauri.conf (v=wspc)│      │ tauri.conf (v=0.1.0)  │
        │ release.yml  v*.*.*│      │ release-link.yml      │
        └────────────────────┘      │ tag link-v*.*.*       │
                                    └───────┬───────────────┘
                                            │ NSIS currentUser artifact
                                  пилотный набор (docs/pilot/)
                                  install.ps1 -> diag.ps1 -> checklist
                                            │
                                  домен: AppLocker/SRP, WebView2, RDS
```
Потоки: SCSS-токены -> `design` -> `main.ts` обоих приложений; компонент `kit` -> оба `dist`; служебная строка Link: Rust-команда (версия, путь `current_exe`, `tauri::webview_version()`) -> webview.

### Recommended Project Structure
```
pnpm-workspace.yaml          # packages: [ui, link-ui, packages/*]
package.json (корень)        # private, scripts-агрегаторы; packageManager pnpm@10.17.1
pnpm-lock.yaml               # единственный (переехал из ui/)
packages/
  design/    src/styles/{_tokens,_breakpoints,global}.scss, public/theme-init.js
  kit/       src/{Button,Modal,...}.svelte, icons/, utils(portal,dropdownAnchor,...), stores(theme,toast)
  transport/ src/{contract.ts,client.ts,ws.ts,errors.ts}
  attachments/ src/{AttachmentViewer,Tile,TypeIcon,...}.svelte, labels.ts, rejectText.ts
ui/                          # Trackly - остаётся (D-18)
link-ui/                     # Vite+Svelte каркас Link
crates/link-app/             # Tauri 2, bin "trackly-link"
docs/adr/, docs/pilot/       # ADR; набор для пилота
.github/workflows/release-link.yml
```

### Pattern 1: source-package
`packages/kit/package.json`: `{ "name": "@trackly/kit", "private": true, "type": "module", "svelte": "./src/index.ts", "exports": { ".": { "svelte": "./src/index.ts", "default": "./src/index.ts" }, "./*.svelte": "./src/*.svelte" } }`. Потребитель: `"@trackly/kit": "workspace:*"`. `@sveltejs/vite-plugin-svelte` компилирует `.svelte` из workspace-пакетов по условию `svelte` в exports. `[ASSUMED]` точная форма exports - подтвердить сборкой в первом плане (Wave 0 спайк «kit в двух приложениях»).

### Pattern 2: SCSS из пакета
Токены сейчас НЕ через `prependData` (комментарий в `vite.config.ts`: scoped `:root` ломает переменные). Значит: `packages/design/src/styles/global.scss` + `_tokens.scss`, импорт `import '@trackly/design/global.scss'` в `main.ts` обоих приложений. Четыре компонента делают `@use '../../styles/_breakpoints' as bp` - заменить на `@use '@trackly/design/breakpoints' as bp` с `sass.loadPaths`/алиасом в `vite.config.ts` (`css.preprocessorOptions.scss.loadPaths: [<packages/design/src/styles>]`) `[ASSUMED]` - подтвердить спайком. `public/theme-init.js` (внешний скрипт для CSP `script-src 'self'`) остаётся в `public/` каждого приложения: либо копия-ссылка на файл пакета в `vite.config` (`publicDir`/плагин копирования), либо оставить в `ui/public` и положить в `link-ui/public` из пакета скриптом сборки. Проще: вынести в `packages/design/public/theme-init.js` и копировать `vite-plugin-static-copy`-подобным мини-плагином (без новой зависимости: `generateBundle` + `fs.readFileSync`).

### Pattern 3: транспорт как контракт
`apiCall` сейчас сшит с `authStore` и `pushToast` (401 -> `authStore.user = null` + `#/login`; 403 -> тост) и ветвится по `__TAURI_INTERNALS__`/fetch. В Link сеть - только Rust. Поэтому `packages/transport` = интерфейс `Transport { call(name,args), onEvent(...) }` + `setTransport()`, реализация Trackly (invoke/fetch с текущей логикой) остаётся в пакете, а реакции на 401/403 внедряются через хуки (`onUnauthorized`, `onForbidden`), которые `ui/` регистрирует при старте. Иначе пакет тянет `auth`/`toast` и логику `#/login`.

### Pattern 4: пакеты без bindings
Пакеты не импортируют `bindings*.ts`. Нужные DTO - структурные интерфейсы внутри пакета; в `ui/` файл-проверка `packages-compat.ts` с `satisfies`/присваиванием типов из `bindings` -> типы пакета, чтобы svelte-check ловил дрейф. `[ASSUMED]` подход; альтернатива - alias `$bindings`, но у Link bindings нет до `proto-ts` (Ф.43+).

### Что переносить, а что нет (по фактическим импортам)
| Группа | Переносить (общее) | Оставить в `ui/` (домен) |
|--------|--------------------|---------------------------|
| design | `styles/_tokens.scss`, `_breakpoints.scss`, `global.scss`, `public/theme-init.js` | - |
| kit | ActionMenu, ActionSubmenu, AuthShell, Badge, Button, Checkbox, DatePicker (+`utils/calendarIcon.js`), DetailField/Panel/Section, Dropdown, FormField, Input, Modal, Placeholder, Radio, Select, Spinner, TableRow, Tabs, Textarea, ThemeSwitcher, Toast, ToastHost, UnreadBadge, `icons/*`, `actionMenuContext.ts`; utils: portal, dropdownAnchor, date, formatFileSize, pluralize, saveFile, hashId; stores: `theme.svelte.ts`, `toast.svelte.ts` | CartridgeSelect, GroupedPrinterSelect, PrinterSelect (bindings), MovementTimeline (bindings, historyEvents, deviceStatuses), NumberTemplateField (api+ws+bindings), PersonAutocomplete (`api/acts`), PlacePicker (api+auth+bindings), PageHeader (`layout-state`, auth), Table (`features/devices/deviceListMode`), HistoryKindChips, Number*Popup (домен номеров - решить по тексту) |
| transport | `api/client.ts`, `api/ws.ts`, `api/errors.ts`, `stores/transport.svelte.ts` (isTauri) | `api/{acts,devices,groups,organization,templates,adSso}.ts` (доменные обёртки; импортируют `apiCall` из пакета) |
| attachments | `features/attachments/*` (10 файлов); тянет `auth`, `toast`, `ws`, `client`, `bindings`, `Modal`, `Button`... | `features/requests/chat/*` (D-19) - его импорты `features/attachments/` придётся перенаправить на пакет |
| stores | `userScoped.svelte.ts` (3 зависимых), `auth.svelte.ts` - СПОРНО: оставить в `ui/`, в пакет только если Link нужен тот же `authStore` (Ф.44 вводит свой вход) | `deviceStatuses`, `deviceTypes`, `placeContentEvents`, `historyPanel` |

Число затрагиваемых импортов (по `grep` в `ui/src`): `$lib/components` - 495 строк, `$lib/stores` - 159, `$lib/api` - 116, `$lib/utils` - 55, `$lib/platform` - 2; относительных `../lib` импортов нет (всё через `$lib`). Масштаб - кодмод (скрипт-замена спецификаторов по списку «файл -> пакет»), а не ручная правка. `[VERIFIED: grep]`

### Anti-Patterns to Avoid
- **Реэкспорт-прокладки в `ui/src/lib/...`** (`export * from '@trackly/kit'`): гейты увидят прокладку, а не код - это ровно «молча зелёное». Менять импорты и пути гейтов, прокладки не оставлять.
- **`prependData` для токенов** - уже отвергнут в проекте (ломает `:root`).
- **Общий пакет импортирует `ui/src/bindings.ts`** - ломает сборку Link и чистый checkout.
- **`version.workspace = true` в `link-app`** - инъекция release.yml перепишет версию.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Запоминание размера/положения окна | свой JSON в app-dir | `tauri-plugin-window-state` 2.5.0 | экраны, максимизация, DPI, восстановление за экран |
| Версия WebView2 | чтение реестра в Rust | `tauri::webview_version()` (экспортируется из `tauri_runtime_wry`) | `[VERIFIED: tauri-2.12.7/src/lib.rs]`; реестр - только в PowerShell-диагностике |
| Путь установки | вычисление из реестра | `std::env::current_exe()` | на Link нет portable-режима |
| Установщик/ярлыки/деинсталляция | свой NSIS-скрипт | штатный Tauri NSIS `installMode: currentUser` (это значение по умолчанию) | `[CITED: v2.tauri.app/reference/config]` |
| Подпись | самописные скрипты в репо | `signtool`/`Set-AuthenticodeSignature` на машине оператора; в CI - секреты | ключи в репо запрещены (D-05) |
| Токены/pin/keyring | в Ф.42 не реализуется | решения - в ADR | Ф.44 |

**Key insight:** в этой фазе нет ни одной задачи, где нужен новый крупный код: это перераскладка + конфигурация + документы. Риск - в регрессиях, а не в сложности.

## Runtime State Inventory

> Фаза - перераскладка путей (рефакторинг), поэтому раздел обязателен.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None - БД/хранилища не содержат путей к `ui/src/...`; `ui/dist` встраивается `rust-embed` (`#[folder = "../../ui/dist"]`) и путь `ui/dist` не меняется (D-18) `[VERIFIED: release.yml/ci комментарии]` | нет |
| Live service config | None - сервисов с настройками «в UI» нет; GitHub: секретов Link в этой фазе нет | нет |
| OS-registered state | None - Link не установлен нигде; пилот создаст записи (ярлыки, HKCU Uninstall) на тестовых ПК - удаление входит в чек-лист D-06 | чек-лист «удаление» |
| Secrets/env vars | `GITHUB_TOKEN` (release) - без изменений; ключи подписи Link появятся в Ф.45; в 42 не создавать | none |
| Build artifacts | `ui/node_modules` (локальный) и `ui/pnpm-lock.yaml` устареют после переезда lockfile в корень; `crates/trackly-app/gen/` игнорируется; для Link добавить `crates/link-app/gen/` в `.gitignore`; `release.yml`: `tauriScript: node .../ui/node_modules/@tauri-apps/cli/tauri.js` остаётся верным, т.к. pnpm кладёт симлинк прямых зависимостей в `ui/node_modules` | удалить `ui/pnpm-lock.yaml`, переустановить; обновить 4 workflow (cache-dependency-path) |

## Перенос гейтов (D-20): инвентарь

**Факты:** `pnpm lint` локально зелёный сейчас (прогон 2026-11-26), `check-lint-selftest-wiring` отчитывается «72 гейтов подключены». 25 гейтов обходят дерево (`readdirSync`/`walk`): act-delete-confirm, action-menu-portal, act-number-no-regex-split, attachments-block, chat-effects, date-picker-layout, chat-ui, device-status-labels, device-row-menu, employee-device-menu, device-list-mode, device-type-labels, focus-outline, group-vocabulary, history-timeline-refresh, lint-selftest-wiring, html-commands, path-settings-form, place-path-short, place-tree-invalidation, printer-identity, store-response-shape, repair-modal-focus, user-scoped-reset, tokens.

**Гейты, чьи ПУТИ затрагивает перенос по рекомендованной раскладке** (гейт -> что сканирует из переезжающего):

| Гейт | Путь(и) в переезжающем | Особенность |
|------|------------------------|-------------|
| check-tokens | `src/styles/_tokens.scss`; сканирует ВСЕ `.svelte` `ui/src` (Rule 1-4) | обязан сканировать и `packages/*/src` - иначе kit уйдёт из-под Rule 2/4 (hex/rgba в `<style>`) |
| check-contrast | `src/styles/_tokens.scss` | читает файл токенов |
| check-focus-outline | обход `src`, `global.scss` | покрытие `.scss` пакета design |
| check-action-menu-portal | `lib/components/ActionMenu`, `ActionSubmenu`, `TableRow`, `styles/global.scss`, `lib/utils/portal`; обход всех `src/**/*.svelte` | селф-тест правит реальные файлы |
| check-modal-isolation | `lib/components/Modal.svelte` | |
| check-date-picker-layout | `lib/components/DatePicker.svelte`, `lib/utils/calendarIcon.js` | |
| check-device-status-labels | `lib/components/Badge/Tabs.svelte`, `lib/stores/deviceStatuses` (остаётся), `lib/utils/labels.ts` | Badge/Tabs уезжают, store остаётся |
| check-attachments-block | `features/attachments/`, обход `src/features` | пакет attachments - добавить корень |
| check-employee-api | `features/attachments/AttachmentsBlock.svelte`, `lib/api/client` | |
| check-user-scoped-reset | `features/attachments/attachmentEvents.svelte.ts`, `lib/api/ws.ts`, `lib/stores/{auth,theme,toast,userScoped}.svelte.ts` | инвентарь сбросов user-scoped: файлы в пакетах |
| check-ws-disconnect-toast | `lib/api/ws.ts`, `lib/stores/toast.svelte.ts` | |
| check-html-commands | `lib/api/client.ts` (`HTML_COMMANDS`) | парсит именно этот файл |
| check-chat-notify-behavior, check-unread-store-behavior, check-open-external | `lib/api/ws`, `lib/stores/{toast,auth,userScoped}`, `lib/platform/systemNotify` | **используют `_rune-harness.mjs`**: хук резолвера `$lib/...` -> `ui/src/lib/...`, заглушки по спецификаторам - надо научить резолвить `@trackly/*` или менять заглушки |
| check-chat-ui | `lib/components/DetailPanel.svelte`, `lib/platform/systemNotify.ts` | |
| check-place-path-short, check-group-vocabulary, check-act-delete-notices | `lib/components/{MovementTimeline,PlacePicker,TableRow}` | MovementTimeline/PlacePicker остаются; TableRow уезжает |
| check-act-delete-confirm, check-act-delete-choice | `$lib/components/Button.svelte` (импорт-проверка) | проверка спецификатора |
| check-movements-type-filter | запрет импорта `$lib/components/Dropdown.svelte` | строка спецификатора меняется на `@trackly/kit` |
| check-place-tree-invalidation, check-store-response-shape | обход `src/lib/api`, `src/lib/stores`, `src/features` | `client.ts` уходит в пакет; доменные api остаются |
| check-reorder, check-placepath-parity, check-printer-identity, check-pagedjs-csp-hash | `lib/utils/{reorder,placePath}.ts`, `lib/deviceTypeIds`, `lib/pdfPreview/*` | **остаются** в `ui/` (домен/печать) - не трогать |

**Не затронуты** (чат заявок/домен остаётся в `ui/`): все `check-chat-dates/linkify/effects`, `check-request-*`, `check-repair-*`, `check-group-*`, `check-device-*` (кроме перечисленных), `check-act-*` (кроме перечисленных), `check-history-*`, `check-report-*`, `check-employee-*`.

### Рекомендованная механика
1. **Wave 0 - ведомость покрытия ДО переезда:** скрипт собирает для каждого гейта список/число просканированных файлов (инструментировать общий хелпер обхода) и коммитит как `ui/scripts/fixtures/gate-coverage.baseline.json` (имена файлов - вымышленных данных тут нет, только пути кода). После переезда суммарное множество (ui + packages) должно совпасть с baseline по набору путей модуло переименование каталогов. Это закрывает «сканируется меньше, но не 0».
2. Общий модуль `ui/scripts/_roots.mjs`: `REPO`, `UI_ROOT`, `SCAN_ROOTS = [ui/src, packages/*/src, link-ui/src]`, `walk()`; гейты, обходящие `src`, переводятся на него (а не на локальные `walk(path.join(UI_ROOT,'src'))`).
3. Гейты с точечными путями - обновить константы на новые пути (таблица выше).
4. `check-lint-selftest-wiring` - инвентарь «от каталога», новый общий модуль `_roots.mjs`/`_strip.mjs` не называется `check-*` и не попадёт в W1; если появятся новые гейты (покрытие, workspace-границы) - подключить в `pnpm lint` вместе с `--selftest`.
5. `eslint.config.js` и `prettier --check .` работают от `ui/` - не увидят `packages/`: добавить в `lint`-цепочку `eslint ../packages ../link-ui` или перенести конфиг в корень (`ignores` из текущего конфига - `public/`, `src/bindings.ts`). `[VERIFIED: ui/eslint.config.js]`
6. `svelte-check` (`--fail-on-warnings`) - добавить `packages/*/src` и `link-ui/src` в охват: у `svelte-check` рабочая область = каталог запуска; `tsconfig.include` расширить или запускать из корня с `--workspace`. `[ASSUMED]` точная форма - проверить в Wave 0.
7. Новый гейт границ: пакеты не импортируют `ui/`, `bindings*`, `@tauri-apps/*` (кроме `transport`), `link-ui` не импортирует `ui/` (selftest с мутантом).

## Tauri: второй крейт

**Образец `crates/trackly-app`:** `tauri.conf.json` -> `frontendDist: ../../ui/dist`, `devUrl: http://localhost:1420`, `beforeDevCommand: pnpm --dir ../ui dev`, `beforeBuildCommand: pnpm --dir ../ui build`; `build.rs` = `tauri_build::build()`; capabilities `capabilities/main.json` (`windows: ["main"]`, `core:default`, dialog/fs/shell/notification); иконки `icons/*`. Версия берётся из `[workspace.package]` + инъекция.

**`crates/link-app` (рекомендуется):**
- `Cargo.toml`: `name = "link-app"`, **`version = "0.1.0"` литералом** (не `version.workspace`), `edition/rust-version/license.workspace = true`, `publish = false`, `[lints] workspace = true`, `[[bin]] name = "trackly-link"`; зависимости `tauri`, `tauri-plugin-window-state` через `workspace = true`; `tauri-build` в build-dependencies. Добавить в `members` корневого Cargo.toml. Perl-регэксп release.yml правит ПЕРВУЮ `version = "X.Y.Z"` в КОРНЕВОМ `Cargo.toml` и `jq` - только `crates/trackly-app/tauri.conf.json` -> Link не затрагивается `[VERIFIED: release.yml]`.
- `tauri.conf.json`: `productName: "Trackly Link"`, `mainBinaryName: "trackly-link"` (переименовывает бинарь; без расширения `[CITED: v2.tauri.app/reference/config]`), `version: "0.1.0"`, `identifier: "org.trackly.link"` (рекомендация, стабилен навсегда: от него AUMID, `%APPDATA%\<id>` данные window-state и `%LOCALAPPDATA%\<id>` при удалении; не заканчивать на `.app`), `build.frontendDist: "../../link-ui/dist"`, `devUrl: "http://localhost:1421"` (порт отличается от 1420, чтобы не конфликтовать с Trackly), `beforeDevCommand: "pnpm --dir ../../link-ui dev"`, `beforeBuildCommand: "pnpm --dir ../../link-ui build"`; окно: `title: "Trackly Link"`, `width: 1000, height: 700, minWidth: 720, minHeight: 500, visible: false` (плагин показывает после восстановления - нет мигания `[CITED: v2.tauri.app/plugin/window-state]`), `security.csp` - задать строгий (`default-src 'self'; connect-src 'self' ipc: http://ipc.localhost`; сети нет) `[ASSUMED]` значения ipc для Windows - проверить сборкой; `bundle.targets: ["nsis"]`, `bundle.windows.nsis.installMode: "currentUser"` (значение по умолчанию, но прописать явно), `languages: ["Russian"]`, `displayLanguageSelector: false`, `bundle.windows.webviewInstallMode` - см. ниже, `bundle.resources` - `THIRD-PARTY-NOTICES.md` по образцу.
- Capabilities `capabilities/main.json`: `core:default` + `window-state:default`; НЕ подключать `shell`/`fs`/`dialog` (ADR: минимальная поверхность). Команда для служебной строки (`link_info`: версия, `current_exe`, `tauri::webview_version()`) - свои разрешения не нужны для собственных `#[tauri::command]`.
- Скрипты: добавить `link-ui` скрипт `tauri` по образцу `ui`: `cd ../crates/link-app && node ../../link-ui/node_modules/@tauri-apps/cli/tauri.js`; `@tauri-apps/cli` - devDependency `link-ui` (закреплена lockfile). `tauriScript` в release-link.yml - путь `link-ui/node_modules/@tauri-apps/cli/tauri.js`.
- `.gitignore`: `link-ui/dist/`, `link-ui/node_modules/`, `crates/link-app/gen/`.
- **Установочный путь (расхождение с D-09):** по шаблону `installer.nsi` (ветка `dev` tauri-bundler) `currentUser` -> `$LOCALAPPDATA\${PRODUCTNAME}` = `%LOCALAPPDATA%\Trackly Link`, переопределяется `/D=` и реестровым путём прежней установки `[CITED: raw.githubusercontent.com/tauri-apps/tauri/dev/.../installer.nsi]`; соответствие версии CLI 2.12.x не подтверждено побайтно - `[ASSUMED]`, ПОДТВЕРДИТЬ собранным установщиком в CI-артефакте (в dry-run добавить шаг, который распаковывает NSIS через `7z l`/ищет строку, либо проверка в пилоте). Скрипт тихой установки: `trackly-link_x64-setup.exe /S /D=<путь>` (`/D=` - последним, без кавычек даже при пробелах - правило NSIS).
- Ключи установщика (из шаблона): `/S` (silent, штатный NSIS), `/P` пассивный, `/R` перезапуск после установки (только вместе с `/P`/`/S`), `/ARGS`, `/UPDATE`, `/NS` без ярлыков, `/D=`; при silent/passive создаётся ярлык на рабочем столе; при `/S` проверка «приложение запущено» (CheckIfAppIsRunning) выполняется - установка поверх запущенного экземпляра ведёт себя особо (проверить в пилоте) `[CITED: installer.nsi]`.
- Запись удаления: `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Trackly Link`, `DisplayName` = productName (D-08 «Приложения и возможности») `[CITED: installer.nsi]`. Ярлык «Пуск»: `$SMPROGRAMS\...\Trackly Link.lnk`.
- **WebView2 в тихой установке:** по умолчанию `webviewInstallMode = {"type":"downloadBootstrapper","silent":true}`: нужен доступ в интернет с ПК; на Win10 с WebView2 уже установленным не требуется. В закрытом контуре без интернета - `offlineInstaller` (+127 МБ) или предустановка Runtime через GPO/WSUS. Для 42: оставить `downloadBootstrapper`, пилот определяет (диагностика фиксирует наличие `pv` до установки) - итог решает режим в Ф.45 `[CITED: v2.tauri.app/distribute/windows-installer]`.
- **Чтение версии WebView2 / пути для D-16:** `tauri::webview_version()` (Result<String>) и `std::env::current_exe()` в команде `link_info`; в webview - `invoke('link_info')`. Версия Link - `app.package_info().version` (из `tauri.conf.json`), не `CARGO_PKG_VERSION` workspace.
- **Tray/single-instance/autostart** - в 42 не добавлять (Ф.44). Иконки `.ico` 16/32/48/256 из знака Trackly с меткой связи (D-12) - сгенерировать `tauri icon` из исходного PNG; финальная прорисовка - Ф.46.
- Данные `window-state`: плагин хранит файл состояния в app-config-каталоге (`%APPDATA%\<identifier>`). Правило «не писать в %APPDATA%» из CLAUDE.md относится к portable Trackly; Link - устанавливаемый клиент. `[ASSUMED]` что это допустимо - зафиксировать в ADR и подтвердить у пользователя (Open Question 3).

## release-link.yml и CI

**Факты (`release.yml`):** триггер `push.tags: ['v*.*.*']` и `workflow_dispatch(version)`; tag = `'v'+version`; шаг создания релиза удаляет прежние DRAFT-релизы с тем же тегом и создаёт `Trackly ${version}`; matrix windows/macos/ubuntu; инъекция версии perl+jq; `tauri-action@v0` с `projectPath: crates/trackly-app`; `gh release upload "$TAG"`; job `checksums`.

- Совпадение тегов: паттерн GitHub-фильтра `v*.*.*` сопоставляется со ВСЕМ именем тега и должен начинаться с `v`; `link-v0.1.0` начинается с `l` и **не** совпадает `[ASSUMED: по правилам glob фильтров GitHub Actions; подтвердить dry-run'ом и/или `act`-аналогом]`. Обратно: `release-link.yml` с `tags: ['link-v*.*.*']` не сработает на `v1.4.0`. Дополнительная защита: в начале job Link - `if: startsWith(github.ref_name, 'link-v') || github.event_name == 'workflow_dispatch'`.
- Dry-run в 42 = ТОЛЬКО `workflow_dispatch` (input `version`, по умолчанию `0.1.0-dryrun`), **без** публикации релиза: собрать NSIS и загрузить как `actions/upload-artifact` (retention 7 дней). Не использовать `tauri-action` с `releaseId`/`releaseDraft` в dry-run (иначе создаётся драфт). Это «не задевает релизы Trackly» буквально: нет ни тега, ни Release.
- Версия Link в dry-run: input `version` -> `jq` правит ТОЛЬКО `crates/link-app/tauri.conf.json` и `crates/link-app/Cargo.toml` (`perl -0pi -e 's/^(version = ")...(")/.../m'` по файлу `crates/link-app/Cargo.toml` - первая `version =` в `[package]`). Проверка эха: `grep -Fq` / `jq -r .version` - по образцу release.yml (проверять точную строку, не `X.Y.Z`-регэксп - допускает `0.1.0-dryrun`).
- Один job Windows x64 (D-01), `runs-on: windows-latest`, `args: --bundles nsis`; **без** `--features heic`, **без** vcpkg (Link не линкует libheif - существенно быстрее); предварительная подкладка NSIS-тулчейна с повторами из release.yml нужна (504 с GitHub CDN `[VERIFIED: комментарии release.yml]`) - скопировать шаг, URL/SHA1 синхронизировать с версией CLI (`@tauri-apps/cli` 2.11.2 в release.yml; у Link CLI из своего lockfile - после объединения lockfile версия одна).
- Composite action (`.github/actions/setup-link-build`) - опционально; в 42 допустимо дублирование шагов, вынос не обязателен.
- **Права:** `permissions: contents: read` для dry-run.
- **Приватность (M-17):** в артефакте и логах - никаких адресов сервера/отпечатков; их в 42 и нет.
- **ci-fast/ci-full/heic-spike** ссылаются на `ui/pnpm-lock.yaml` (`cache-dependency-path`) - после переезда lockfile -> `pnpm-lock.yaml`; `pnpm install --frozen-lockfile` с `working-directory: ui` продолжит работать (pnpm находит workspace вверх по дереву), но лучше запускать из корня. ci-fast добавить шаги: `pnpm --dir link-ui build`, `pnpm --dir link-ui svelte-check`, `cargo clippy --workspace` уже подхватит `link-app` (workspace member) - **учесть**: `cargo test --workspace` соберёт `link-app` на Ubuntu - нужны те же системные пакеты Tauri (уже ставятся), и `frontendDist` `link-ui/dist` обязан существовать при компиляции (`tauri::generate_context!` проверяет) -> «seed placeholder» для `link-ui/dist` по образцу `ui/dist` (иначе ci-fast красный сразу, как было при `rust-embed`).
- `Swatinem/rust-cache`, `rust-toolchain.toml` (1.92) - как есть.
- Расхождение toolchain: ci-fast ставит `1.88`, release - `1.92`; `rust-toolchain.toml` = 1.92.0 приоритетен у rustup (`[ASSUMED]` - не менять, не относится к фазе).

## Стенд живой проверки в браузере (D-21)

Прецедент: одноразовый `#[ignore]`-тест поднимает настоящий `build_router` + `ui/dist` и открывается во встроенном браузере (память проекта «Стенд живой проверки в браузере»; конкретный образец искать в `crates/trackly-app/tests/` по `#[ignore]` + `build_router`). Для фазы: (1) `pnpm build` (вместе с `prebuild` export_bindings; нужен `ui/dist`-seed на чистом чекауте); (2) запустить стенд; (3) обойти ключевые разделы (вход, Устройства, Акты, Принтеры, Заявки, Отчёты, Настройки, Места/Группы, вложения - используют kit/attachments/transport), собрать ошибки консоли. Критерий: ноль `console.error` и ноль 404 на ассеты; сверка с прогоном ДО переезда (снимок консоли ДО - в Wave 0, иначе нечего сравнивать). Ограничение (честно): WKWebView/WebView2-десктоп не проверяется; это допущение D-21, помечать «UNVERIFIED в десктопе». Стенд ходит по HTTP-пути `apiCall` (fetch), не по `invoke` - ветка Tauri-invoke в пакете `transport` остаётся без живой проверки.

## Набор для пилота (DST-06)

Расположение: `docs/pilot/` (README-чек-лист, `install-silent.ps1`, `diag.ps1`, `sign-notes.md`). Все скрипты - без данных организации; результаты оператор присылает обезличенно (шаблон `RESULT-TEMPLATE.md` с полями без имён: «ОС: Windows 10 22H2 x64», «AppLocker: Enforce/Audit/None», «WebView2: версия»).

### `diag.ps1` (только чтение; запускается пользователем-сотрудником, без админа)
- Версия ОС/разрядность: `[Environment]::OSVersion`, `[Environment]::Is64BitOperatingSystem`, `(Get-CimInstance Win32_OperatingSystem).Caption/BuildNumber/ProductType` (`ProductType` 3 = сервер).
- **WebView2 Runtime:** значение `pv` (REG_SZ) в `HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}` и `HKCU:\Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}`; «установлен» = хотя бы одно значение > `0.0.0.0` `[CITED: learn.microsoft.com WebView2 distribution]`. Вывести оба значения и что именно использовано.
- **Путь установки Link:** `$env:LOCALAPPDATA\Programs\Trackly Link\trackly-link.exe` и `$env:LOCALAPPDATA\Trackly Link\trackly-link.exe` (проверить оба - из-за расхождения с D-09); `HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Trackly Link` (`InstallLocation`, `DisplayVersion`); разрядность exe (чтение PE-заголовка `Machine`: 0x8664 = x64).
- **Подпись:** `Get-AuthenticodeSignature` для exe и установщика -> `Status`, `SignerCertificate.Subject` (в отчёт - только `Status` и «подписан/не подписан»; Subject может содержать название организации - **не** выводить в файл-итог, оставить на экране и пометить «не пересылать»).
- **AppLocker:** `Get-WinEvent -LogName 'Microsoft-Windows-AppLocker/EXE and DLL'` и `.../MSI and Script`, фильтр по `Message -like '*trackly-link*'`; ID: 8002 разрешено, 8003 audit (`would have been prevented`), **8004 заблокировано** (EXE/DLL), 8005-8007 для script/MSI; 8001 - политика применена `[CITED: learn.microsoft.com AppLocker events]`. Эффективная политика: `Get-AppLockerPolicy -Effective -Xml` (кол-во правил по коллекциям и режим EnforcementMode; без имён путей организации в отчёт).
- **SRP:** события источника `Microsoft-Windows-SoftwareRestrictionPolicies` в журнале Application (ID 865-868, 882 - `[ASSUMED]` значения, фильтровать по провайдеру, а не только по ID); ключ `HKLM:\SOFTWARE\Policies\Microsoft\Windows\Safer\CodeIdentifiers` (`DefaultLevel`, `PolicyScope`).
- **RDS/сеанс:** `$env:SESSIONNAME` (`RDP-Tcp#N` - RDP; `Console` - локально), `(Get-CimInstance Win32_ComputerSystem).Roles`/`ProductType`, `query user`/`qwinsta` (число активных сеансов - только число), `[System.Windows.Forms.SystemInformation]::TerminalServerSession` `[ASSUMED]`, ключ `HKLM:\SYSTEM\CurrentControlSet\Control\Terminal Server` (`TSAppCompat`).
- **Запуск exe:** `Start-Process ... -PassThru`, подождать 5 с, проверить `HasExited`/`ExitCode`, затем AppLocker-события за последнюю минуту.
- Вывод: JSON-файл в `$env:TEMP`, содержащий ТОЛЬКО обезличиваемые поля (без `$env:COMPUTERNAME`, `$env:USERNAME`, IP, домена - явно исключить; privacy-гейт репо этого не проверит, т.к. файл не коммитится - но шаблон результата вручную сверять).

### `install-silent.ps1` (имитация вызова из logon-скрипта GPO)
`Start-Process -FilePath $installer -ArgumentList '/S',"/D=$env:LOCALAPPDATA\Programs\Trackly Link" -Wait -PassThru`; проверка `ExitCode` (0), наличие exe, запись HKCU Uninstall, ярлык. Идемпотентность: повторный запуск при установленной версии - не ломается. Деинсталляция: `"<InstallLocation>\uninstall.exe" /S`. Для RDS: два сеанса под разными пользователями одновременно запускают exe (проверка D-06). `[ASSUMED]` имя установщика `Trackly Link_<ver>_x64-setup.exe` - взять фактическое из артефакта.

### Подпись тестовой сборки (D-05) - «без ключей в репозитории»
- **Вариант AD CS:** оператор запрашивает сертификат «Code Signing» из шаблона AD CS, подписывает `signtool sign /fd SHA256 /tr <timestamp-url> /td SHA256 /sha1 <thumbprint> trackly-link.exe` и установщик; правило AppLocker «издатель» по этому сертификату.
- **Вариант самоподписанный:** `New-SelfSignedCertificate -Type CodeSigningCert -Subject 'CN=Link Pilot Test' -CertStoreLocation Cert:\CurrentUser\My -NotAfter (Get-Date).AddMonths(6)`; экспорт `.cer` (публичный) -> раздать GPO в «Доверенные корневые» и «Доверенные издатели»; подпись `Set-AuthenticodeSignature` или `signtool`. Без доверия в корневых AppLocker-правило по издателю не сработает `[ASSUMED]`.
- **Проблема:** exe внутри NSIS-установщика подписывается ДО упаковки. Подпись готового установщика не подписывает установленный `trackly-link.exe`. Пилотные варианты: (a) оператор подписывает уже установленный exe на тестовом ПК (подходит для проверки «правило по издателю разрешает запуск»); (b) пилотная сборка в CI с необязательной подписью через `bundle.windows.signCommand`, команда читает PFX из GitHub-секрета, заданного пользователем (секрет в репо не попадает) - это территория Ф.45; в 42 достаточно (a) + описание (b) в `sign-notes.md`. Рекомендуется (a).
- Дополнительные риски AppLocker: NSIS-установщик при запуске распаковывает DLL-плагины в `%TEMP%\nsXXXX.tmp` - при включённых DLL-правилах возможна блокировка (событие 8004 для `.dll` в `%TEMP%`); отметить в чек-листе.

### Чек-лист (минимум D-06, по пунктам с «Ожидаемо / Фактически / Событие»)
1. Диагностика ДО (WebView2, разрядность, политика). 2. Тихая установка без админа (`exit 0`, путь, запись удаления, WebView2 не потребовал админа или интернета/потребовал - что именно). 3. Запуск из пользовательского каталога под AppLocker/SRP (Enforce/Audit; при 8004 - применить разрешающее правило по издателю/пути и повторить - зафиксировать какое сработало). 4. Окно каркаса открывается: логотип, «Чат», сайдбар, служебная строка (версия Link, WebView2, путь). 5. Тема: светлая/тёмная следует системной. 6. Положение/размер запоминаются (закрыть-открыть). 7. RDS: два пользователя одновременно запускают свои экземпляры (оба видят окно, у каждого своя установка/`%LOCALAPPDATA%`). 8. Удаление (`uninstall.exe /S`, ярлыки/запись реестра исчезли, каталог пуст). 9. Итог-карточка (обезличенно).
Результат записывается в `docs/pilot/RESULT.md` по шаблону; пока нет - файл-заглушка «ОТЛОЖЕНО: human-UAT (D-07)» + запись в `42-HUMAN-UAT.md`/STATE (по принятой в проекте форме отложенных UAT).

## ADR

**Формат/место:** `docs/adr/NNNN-<slug>.md`, по одному решению на файл (MADR-подобно: Статус / Контекст / Решение / Последствия / Отвергнутые варианты / Источники). Индекс `docs/adr/README.md`. `docs/` сейчас содержит только `AD-SETUP.md`, `TESTING.md` - каталог `adr/` новый. Рекомендуется один файл на группу решений D-22 (7-8 файлов), статус «Accepted» после принятия пользователем; пункт про пилот - «Accepted (provisional)» до итога пилота.

**Закрытие расхождений (решать в ADR):**
| Расхождение | Что нашёл | Рекомендованное решение |
|-------------|-----------|--------------------------|
| `keyring-core` vs `keyring` | `keyring` 4.2.0 - тонкая обёртка: `keyring-core` обязательная зависимость, store-крейты опциональные (`windows-native-keyring-store`, `apple-native-keyring-store`, `zbus-secret-service-keyring-store`, ...); README авторов: приложения с полным набором должны зависеть от `keyring-core` + нужных store-крейтов `[CITED: github keyring-rs README, crates.io API]`. keyring-core 1.1.0 MSRV 1.85 `[VERIFIED: crates.io]` | `keyring-core` 1.1.0 + store-крейты под `cfg(target_os)`; Linux: `zbus-secret-service-keyring-store` (без libdbus), без Secret Service - токен только в памяти. Не ставить в 42 |
| Имя крейта | ARCHITECTURE/SUMMARY: `link-app`; STACK: `trackly-client` (в примере cargo add); CONTEXT D-09: на усмотрение | Пакет `link-app` (каталог `crates/link-app`), `[[bin]] name = "trackly-link"`, `mainBinaryName = "trackly-link"`; `trackly-client` отбросить |
| Токенайзер FTS | ARCHITECTURE: `trigram`; PITFALLS: `unicode61 remove_diacritics 2`. **Проверено экспериментом** на `rusqlite 0.41` bundled (SQLite 3.54.1, 2026-11-26): FTS5 доступен; `trigram` находит подстроки по-русски (`коллег`, `ривет`, `картр`) без учёта регистра (`ПРИВЕТ` находит); `unicode61` не находит подстроки (`коллег` -> 0), только префиксы `картр*`; запрос короче 3 символов (`ка`) с trigram -> 0 строк; `ё` и `е` не склеиваются ни в одном токенайзере (`ежик` не находит `ёжик`) | `trigram` (подстрока без стеммера) + на уровне приложения: нормализация `ё->е` и в индексируемом тексте, и в запросе; для запроса < 3 символов - отдельный путь (`LIKE`/запрет поиска, решить в Ф.47); запрос всегда соединяется с членством внутри SQL (из ARCHITECTURE). Включить тест миграции «FTS5+trigram доступен» как задел Ф.47 |
| Откат версии | ARCHITECTURE: `version_comparator` | Основной - выпуск с большим номером; `version_comparator` - запасной (D-22) |
| Каталог установки | D-09 vs шаблон NSIS | См. выше: `/D=` в скрипте установки; отдельное решение в ADR о каноническом пути |
| Данные в `%APPDATA%` | CLAUDE.md portable vs window-state | ADR: portable относится к серверу Trackly; Link - per-user установка (ожидаемо подтвердить) |

Эксперимент (FTS5) - выполнен в scratch-каталоге, в репозиторий не коммитится; для ADR привести вывод в виде таблицы результатов.

## Common Pitfalls

### Pitfall 1: гейт «зелёный», но сканирует меньше
**What goes wrong:** компоненты уехали в `packages/`, гейт обходит `ui/src`, находит остаток и PASS. **How to avoid:** `_roots.mjs` + ведомость покрытия «до/после»; для гейтов с `readdirSync` - проверка «просканировано N >= baseline». **Warning signs:** строка PASS без числа файлов; общее число `.svelte` в `SCAN_ROOTS` меньше, чем было в `ui/src`.

### Pitfall 2: селф-тесты мутируют «реальные» файлы по старым путям
Мутационные selftest'ы (`replace(...,1)` по неуникальному якорю - см. память «Якорь мутационной проверки») читают исходник по константе-пути; при неверном пути selftest падает (хорошо) или, хуже, мутирует соседний файл. **Avoid:** после правки путей прогонять КАЖДЫЙ `--selftest` и пару мутаций вручную.

### Pitfall 3: `_rune-harness.mjs` не резолвит пакеты
Хук резолвера знает только `$lib/...` и относительные `.ts`; стор `toast`/`auth`/`userScoped` и `ws` переезжают. **Avoid:** научить хук `@trackly/<pkg>` -> `packages/<pkg>/src`, обновить `stubs`; прогнать 3 поведенческих гейта + их мутанты.

### Pitfall 4: lockfile и CI
`ui/pnpm-lock.yaml` исчезает; `cache-dependency-path` в 4 workflow, `pnpm/action-setup`, шаг `pnpm install --frozen-lockfile` в release.yml. **Avoid:** одним коммитом с переездом lockfile; D-18 («release.yml не меняется») трактовать как «пути артефактов и логика выпуска», а кеш-путь править.

### Pitfall 5: `frontendDist` не существует при `cargo clippy/test --workspace`
`tauri::generate_context!` в `link-app` требует `link-ui/dist`. **Avoid:** placeholder-шаг в ci-fast/ci-full (как для `ui/dist`) ИЛИ `[build-dependencies]`-проверка; иначе красный ci-fast.

### Pitfall 6: `prebuild` у `ui` тянет cargo
`"prebuild": "cargo test -p trackly-app --test export_bindings"`. В `link-ui` такого хука быть не должно (сборка UI Link не должна компилировать Rust). **Avoid:** не копировать скрипты `ui` в `link-ui`.

### Pitfall 7: унификация feature-ов в workspace
Прецедент `snmp2`/`zip` (Windows-only поломка из-за унификации). Link в 42 тянет только `tauri` + window-state, риск мал, но: собирать релиз строго `-p link-app`; `cargo tree -d`/`cargo deny` без новых предупреждений.

### Pitfall 8: путь установки
`%LOCALAPPDATA%\Trackly Link` vs `...\Programs\Trackly Link` - AppLocker path-правило пользователя должно соответствовать фактическому. **Avoid:** `/D=` в скрипте; чек-лист фиксирует фактический путь.

### Pitfall 9: приватность
Репозиторий публичный: в ADR/чек-листе/скриптах/результатах пилота - никаких названий организации, хостов, доменов AD, имён, IP, отпечатков сертификатов, Subject'ов реальных сертификатов. Тестовый CN - «Link Pilot Test». Перед коммитом - `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` (гейт ловит только занесённое известное, не ФИО - см. память) и ручная сверка.

### Pitfall 10: `window-state` и `visible:false`
Если окно создаётся `visible:false`, а плагин не восстановит состояние (первый запуск, ошибка) - окно может не показаться. **Avoid:** явный `window.show()` в `setup` как страховка; тест первого запуска без файла состояния.

### Pitfall 11: смена порта dev
Vite Trackly - 1420 `strictPort`; Link dev - 1421 (в конфиге и `devUrl`), иначе параллельный `tauri dev` конфликтует.

## Code Examples

### Корневой workspace
```yaml
# pnpm-workspace.yaml
packages:
  - ui
  - link-ui
  - packages/*
```
```json
// корневой package.json (private)
{ "name": "trackly-monorepo", "private": true, "packageManager": "pnpm@10.17.1", "engines": { "node": ">=20" } }
```

### Версия WebView2 и путь для служебной строки (Rust)
```rust
// Source: tauri-2.12.7/src/lib.rs (pub use tauri_runtime_wry::webview_version) [VERIFIED]
#[derive(serde::Serialize)]
struct LinkInfo { version: String, webview: Option<String>, install_path: String }

#[tauri::command]
fn link_info(app: tauri::AppHandle) -> LinkInfo {
    LinkInfo {
        version: app.package_info().version.to_string(),
        webview: tauri::webview_version().ok(),
        install_path: std::env::current_exe()
            .ok().and_then(|p| p.parent().map(|d| d.display().to_string()))
            .unwrap_or_default(),
    }
}
```

### Плагин состояния окна
```rust
// Source: https://v2.tauri.app/plugin/window-state/ [CITED]
tauri::Builder::default()
    .plugin(tauri_plugin_window_state::Builder::default().build())
    .invoke_handler(tauri::generate_handler![link_info])
    .run(tauri::generate_context!())
    .expect("error while running Trackly Link");
```
Capability: `"permissions": ["core:default", "window-state:default"]`.

### Фрагмент `link-app/tauri.conf.json`
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Trackly Link",
  "mainBinaryName": "trackly-link",
  "version": "0.1.0",
  "identifier": "org.trackly.link",
  "build": {
    "frontendDist": "../../link-ui/dist",
    "devUrl": "http://localhost:1421",
    "beforeDevCommand": "pnpm --dir ../../link-ui dev",
    "beforeBuildCommand": "pnpm --dir ../../link-ui build"
  },
  "app": { "windows": [ { "label": "main", "title": "Trackly Link", "width": 1000, "height": 700,
            "minWidth": 720, "minHeight": 500, "visible": false, "dragDropEnabled": false } ] },
  "bundle": { "active": true, "targets": ["nsis"],
    "windows": { "nsis": { "installMode": "currentUser", "languages": ["Russian"] } } }
}
```

### Каркас окна (структура, Svelte 5)
Сетка: колонка 1 - узкий логотип («Link» + знак) в верхней левой ячейке; в верхней строке справа - `Tabs` из kit с единственной вкладкой «Чат»; ниже слева - сайдбар (пустой/заглушка), справа - область содержимого с «Здесь скоро будет чат» и служебной строкой (`link_info`). Использовать только токены `--tr-*` (гейт `check-tokens` Rule 2/4 запрещает hex/rgba в `<style>`), `ThemeSwitcher` из kit, `.content`-паттерн «каждая страница - height:100% + min-height:0, скроллит свою область» (память проекта «App shell scroll pattern»). Тема следует системной: `theme-init.js` + `initTheme()` из пакета kit.

### Критерий «компонент из пакета рендерится в обоих» (SC-3)
Один и тот же импорт `import { Button } from '@trackly/kit'` в `ui/` (в существующем экране, например, замена `$lib/components/Button.svelte`) и в `link-ui/App.svelte`; проверка - гейт границ (нет второй копии файла: `find . -name Button.svelte -not -path '*/node_modules/*'` -> ровно 1) + в обоих `dist` присутствует один и тот же хэш чанка компонента `[ASSUMED]`; достаточно проверки «ровно один исходник» + сборки обоих.

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| Один пакет `ui/`, `$lib` | pnpm workspace + `workspace:*` source-пакеты | общий код для двух клиентов |
| `keyring` 3 (монолит) | `keyring` 4 = фасад над `keyring-core` + store-крейтами | модульные зависимости |
| window-state 2.x | 2.5.0 стабильная; 3.0.0-alpha.* - не использовать | брать 2.5.0 |

**Deprecated/outdated:** `nsis.minimumWebview2Version` - устарел, использовать `bundle > windows > minimumWebview2Version` `[CITED: reference/config]`.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Точная форма `exports`/`svelte`-условия source-пакетов собирается vite-plugin-svelte 5 без доп. настроек | Pattern 1 | Нужен `optimizeDeps`/`ssr.noExternal`-эквивалент; решить спайком Wave 0 |
| A2 | `sass.loadPaths`/алиас для `@use` из пакета design работает в Vite 6 + svelte-preprocess/vitePreprocess и в `svelte-check` | Pattern 2 | Пути `@use` придётся оставить относительными (`../../design/...`) |
| A3 | Пакеты без bindings + compat-файл с `satisfies` ловит дрейф типов | Pattern 4 | Нужен общий пакет типов |
| A4 | `svelte-check` охватит `packages/*` и `link-ui` расширением include/запуском из корня | Перенос гейтов п.6 | Нужен отдельный запуск на каждый пакет |
| A5 | Тег `link-v0.1.0` не совпадает с `v*.*.*` | release-link | Dry-run проверит; запасной `if:` guard |
| A6 | Шаблон NSIS в CLI 2.12.x ставит в `%LOCALAPPDATA%\<productName>` (подтверждено по ветке `dev`) | Tauri/путь | Путь уже `Programs\...`; `/D=` всё равно детерминирует |
| A7 | События SRP 865-868/882 и сигнал `TerminalServerSession` | Набор для пилота | Диагностика пропустит событие; фильтр по провайдеру страхует |
| A8 | Самоподписанный сертификат работает для AppLocker-правила по издателю только при доверии в корневых хранилищах | Подпись | Иная схема раздачи доверия |
| A9 | Данные window-state в `%APPDATA%` допустимы для Link | Tauri/ADR | Нужен каталог рядом с exe; смена до Ф.44 |
| A10 | CSP `connect-src 'self' ipc: http://ipc.localhost` достаточна в WebView2 | tauri.conf | Белый экран/ошибки IPC; проверить первой сборкой Windows |
| A11 | `identifier` `org.trackly.link` приемлем | Tauri | Менять нельзя после первой раскатки - подтвердить |

## Open Questions (RESOLVED)

1. **Канонический путь установки: `...\Programs\Trackly Link` (D-09) или штатный `...\Trackly Link`?**
   - Известно: штатно Tauri кладёт без `Programs`; `/D=` переопределяет. Неизвестно: предпочтение пользователя для AppLocker-правил.
   - Рекомендация: оставить D-09 (`/D=` в скрипте GPO), зафиксировать в ADR; пилот проверит.
   - RESOLVED: D-09 — `%LOCALAPPDATA%\Programs\Trackly Link` через `/D=` в скрипте тихой установки; ADR 0005 (план 42-03), скрипт и чек-лист (план 42-04).
2. **Как ограничить охват `svelte-check`/eslint на packages?** Решается спайком в первом плане (A4).
   - RESOLVED: спайк плана 42-09 подключает eslint/svelte-check к `packages/`; гейты обязаны покрывать пакеты.
3. **Допустимы ли данные Link (`window-state`) в `%APPDATA%`?** Ожидаемо да (не portable); подтвердить у пользователя.
   - RESOLVED (оркестратор, 2026-11-26): да — portable-правило относится к серверу/десктопу Trackly, Link — устанавливаемый клиент на пользователя; данные window-state в каталоге данных ОС пользователя; фиксируется в ADR 0005 (план 42-03).
4. **Где остаются `auth.svelte.ts`/`userScoped.svelte.ts`?** Рекомендация: `userScoped` - в kit (универсальный механизм сброса), `auth` - в `ui/`, в Link свой вход (Ф.44).
   - RESOLVED: так и сделано в планах 42-12 (kit) и 42-15 (transport через хуки 401/403).
5. **Пилотная сборка подписана?** Рекомендация (a): подпись установленного exe на ПК оператора; CI-подпись - Ф.45.
   - RESOLVED: вариант (a) — подпись на ПК оператора, ключи вне репозитория и CI; план 42-04, ADR 0004/0005.
6. **Включать ли в 42 `trackly-proto`/`proto-ts` и гейт дрейфа** (SUMMARY перечисляет в Ф.42, CONTEXT - нет)? CONTEXT не требует; рекомендация - НЕ включать (нет новых DTO до Ф.43/47), отметить в ADR.
   - RESOLVED: не включать в Ф.42 (нет в CONTEXT); отметка в ADR (план 42-03).
7. **Спайки ROADMAP (pin SPKI на reqwest/ring+tungstenite, keyring на 3 ОС):** CONTEXT сводит фазу к ADR; рекомендация - FTS5 проверен здесь; pin и keyring оформить в ADR как «проверить в Ф.44», без кода в 42.
   - RESOLVED: FTS5 trigram проверен экспериментом исследования; pin SPKI и keyring-core — в ADR как «проверить в Ф.44» (план 42-03), кода в 42 нет.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Node | UI-сборка/гейты | ✓ | v24.21.0 локально (CI: 20) | - |
| pnpm | workspace | ✓ | 10.17.1 | - |
| Rust/cargo | crates | ✓ | 1.92 (rust-toolchain.toml) | - |
| Windows-ПК (домен, AppLocker/SRP, RDS) | пилот DST-06, проверка NSIS | ✗ (dev на macOS) | - | отложенная human-UAT (D-07) |
| Windows-CI (windows-latest) | сборка NSIS Link | ✓ в GitHub Actions | - | - |
| slopcheck | аудит пакетов | ✓ | /opt/homebrew/bin | - |
| ctx7 | документация | ✗ | - | WebFetch (использован) |

**Missing without fallback:** живой домен для пилота - блокирует DST-06 acceptance, не закрытие фазы (D-07).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Node-скрипты `ui/scripts/check-*.mjs` (+`--selftest`), `svelte-check`, `eslint`/`prettier`, `cargo test/clippy/fmt` |
| Config file | `ui/package.json` (`lint`, `svelte-check`), `.github/workflows/ci-fast.yml` |
| Quick run command | `pnpm --dir ui lint` (быстро, ~минуты); `pnpm --dir ui svelte-check` |
| Full suite command | команда `ci-full`/ci-fast: `pnpm build` -> `pnpm svelte-check` -> `pnpm lint` -> `cargo fmt --check` -> `cargo clippy --workspace --all-targets -- -D warnings` -> `cargo test --workspace` (долго, один раз на границе волны, в фоне; одновременно один `cargo test`) |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LNK-01 | Trackly собирается и все гейты зелёные после переезда | gate+build | `pnpm --dir ui build && pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ |
| LNK-01 | ни один гейт не потерял покрытие | gate (новый) | `node ui/scripts/check-gate-coverage.mjs --selftest && node ui/scripts/check-gate-coverage.mjs` | ❌ Wave 0 |
| LNK-01 | пакеты не импортируют `ui/`/bindings; Link не импортирует `ui/` | gate (новый) | `node ui/scripts/check-workspace-boundaries.mjs --selftest && ...` | ❌ Wave 0 |
| LNK-01 | компонент в одном исходнике, собирается в обоих | build+gate | `pnpm --dir link-ui build` + `pnpm --dir ui build`; гейт «один Button.svelte» | ❌ Wave 0 |
| LNK-01 | версия Link независима | unit(shell) | `grep` в `crates/link-app/Cargo.toml` (нет `version.workspace`) + `jq -r .version crates/link-app/tauri.conf.json`; ложится в гейт границ | ❌ Wave 0 |
| LNK-01 | `release-link.yml` не срабатывает на `v*.*.*` и не создаёт Release | static+CI | `node scripts/check-release-isolation.mjs` (парсит оба workflow: триггеры не пересекаются, в dry-run нет `releaseId`/`createRelease`) + dry-run `gh workflow run release-link.yml --field version=0.1.0-dryrun` | ❌ Wave 0 |
| LNK-01 | Trackly: поведение UI не сломано | browser stand (D-21) | `cargo test -p trackly-app --test <stand> -- --ignored` + прогон агента; консоль без ошибок | ✅ образец, ❌ под фазу |
| LNK-10 | раскладка: логотип, вкладка «Чат», сайдбар, область | gate (новый, структурный) + build | `node link-ui/scripts/check-link-layout.mjs --selftest && ...` (читает `App.svelte`: присутствуют логотип «Link», `Tabs` с «Чат», `aside`/сайдбар, область контента, заглушка, служебная строка) | ❌ Wave 0 |
| LNK-10 | окно 1000x700, min 720x500, window-state подключён | static | проверка `tauri.conf.json` + `capabilities/main.json` (`window-state:default`) в том же гейте | ❌ Wave 0 |
| LNK-10 | визуально: окно показывается, размер запоминается | manual (Windows) | чек-лист пилота п.4, п.6 | manual-only: WebView2/WKWebView |
| DST-06 | набор пилота полон и без приватных данных | static | `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256`; парсинг `.ps1`: `[System.Management.Automation.Language.Parser]::ParseFile` (на macOS - `pwsh` при наличии, иначе пропуск с пометкой) | ❌ Wave 0 |
| DST-06 | итог пилота, ADR принят | manual | `docs/pilot/RESULT.md`, `docs/adr/*` статус Accepted; либо отложенная UAT | manual-only (живой домен) |

### Sampling Rate
- **Per task commit:** `pnpm --dir ui lint` + затронутые `--selftest` + `pnpm --dir ui svelte-check` по затронутым пакетам.
- **Per wave merge:** `pnpm build`, `svelte-check`, `lint`, `cargo clippy --workspace`, `cargo fmt --check`; сплошной `cargo test --workspace` - один раз на границе волны, в фоне (память: ~4,5-6 ч на старой схеме; ветка с suite-бинарниками ~97 с - уточнить актуальность).
- **Phase gate:** полный набор + стенд D-21 + dry-run `release-link.yml` зелёные перед `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] ведомость покрытия гейтов (baseline до переезда) и `check-gate-coverage.mjs` (+selftest, подключить в `lint`)
- [ ] `check-workspace-boundaries.mjs` (+selftest, подключить в `lint`)
- [ ] `check-release-isolation.mjs` (корневой `scripts/`, шаг ci-fast + selftest по образцу двух корневых гейтов)
- [ ] `check-link-layout.mjs` (+selftest)
- [ ] снимок консоли браузерного стенда ДО переезда
- [ ] спайк A1/A2/A4 (source-package + SCSS + svelte-check) на одном компоненте `Button`
- [ ] `_roots.mjs` и перевод `_rune-harness.mjs` на `@trackly/*`

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | нет (вход - Ф.44) | - |
| V3 Session Management | нет | - |
| V4 Access Control | нет | - |
| V5 Input Validation | минимально | служебная строка выводит строки из Rust (экранирование Svelte по умолчанию; без `{@html}`) |
| V6 Cryptography | нет в коде; ADR фиксирует pin/keyring | не изобретать |
| V10/V14 Config/Supply chain | да | закреплённый CLI, секреты только в GitHub, `cargo deny`, slopcheck, строгий CSP, минимум capabilities (без shell/fs/dialog) |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Утечка приватных данных организации в публичный репозиторий (ADR, результаты пилота, скрипты) | Information Disclosure | вымышленные значения, `check-privacy` до коммита, шаблон результата без имён хостов/пользователей/отпечатков |
| Подмена/утечка ключа подписи | Tampering | ключи только вне репо (секреты Actions); в 42 ключей нет |
| Расширенные права webview Link | Elevation | capabilities без `shell`/`fs`/`dialog`; CSP `default-src 'self'`; сеть - только Rust (Ф.44) |
| Выпуск Link перезаписывает релиз Trackly | Tampering | раздельные теги/workflow, dry-run без Release, гейт изоляции |
| Вредоносный пакет (slopsquatting) | Tampering | slopcheck [OK], официальные `tauri-apps` пакеты, минимум новых зависимостей |
| Установка под AppLocker обходит политику | - | не обходить: разрешающее правило по издателю (D-04), без хаков |

## Sources

### Primary (HIGH)
- Репозиторий (прочитано/выполнено 2026-11-26): `ui/package.json`, `ui/vite.config.ts`, `ui/svelte.config.js`, `ui/tsconfig.json`, `ui/eslint.config.js`, `ui/scripts/*` (пути, обходчики, `_rune-harness.mjs`), `crates/trackly-app/{tauri.conf.json,build.rs,capabilities/main.json}`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.github/workflows/{release,ci-fast,ci-full,heic-spike}.yml`, `pnpm lint` (зелёный).
- crates.io API: tauri-plugin-window-state (2.5.0 стабильная), keyring 4.2.0 (зависимости), keyring-core 1.1.0 (MSRV 1.85), store-крейты.
- `tauri-2.12.7/src/lib.rs` (`webview_version`).
- Эксперимент FTS5: rusqlite 0.41 bundled, SQLite 3.54.1 (scratch, не в репо).

### Secondary (MEDIUM)
- https://v2.tauri.app/reference/config/ (mainBinaryName, productName, NSIS installMode default `currentUser`, languages).
- https://v2.tauri.app/distribute/windows-installer/ (режимы WebView2, per-user установка).
- https://v2.tauri.app/plugin/window-state/ (регистрация, `window-state:default`, `visible:false`).
- tauri-apps/tauri `dev`: `crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi` (INSTDIR, `/P /R /UPDATE /NS /D=`, ключи реестра).
- https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution (реестр `pv`, per-user/per-machine).
- https://learn.microsoft.com/.../applocker/using-event-viewer-with-applocker (ID 8002-8007).
- https://github.com/open-source-cooperative/keyring-rs (README: модульный путь).

### Tertiary (LOW)
- ID событий SRP, сигналы RDS-сеанса, поведение самоподписанного издателя в AppLocker - `[ASSUMED]`, подтвердить пилотом.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - всё уже в репозитории; одна новая зависимость проверена.
- Architecture (монорепо/Vite/SCSS/svelte-check): MEDIUM - требует спайка Wave 0 (A1-A4).
- Перенос гейтов: HIGH по инвентарю путей (grep/чтение), MEDIUM по точной механике.
- NSIS/WebView2/AppLocker: MEDIUM - по документации и шаблону; живой результат только пилотом.
- Pitfalls: HIGH для CI/гейтов (из чтения кода), MEDIUM для Windows.

**Research date:** 2026-11-26
**Valid until:** 2026-12-26 (Tauri 2.12.x и плагины быстро меняются - перепроверить версии перед сборкой)
