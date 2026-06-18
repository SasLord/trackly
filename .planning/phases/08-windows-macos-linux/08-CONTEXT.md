# Phase 8: Релизный пайплайн (Windows/macOS/Linux) - Context

**Gathered:** 2026-06-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Релизный CI/CD пайплайн: при push git-тега `v*.*.*` GitHub Actions собирает
распространяемые артефакты Trackly для трёх ОС (приоритет — Windows 10 x64),
прикладывает SHA256-checksums и заливает всё в **draft** GitHub Release для
ручной публикации. Плюс корневой README на русском с инструкциями по запуску.

Покрывает требования **BLD-02, BLD-03, BLD-04, BLD-05**.

**В scope:**
- GitHub Actions workflow `release.yml`, триггер по тегу `v*.*.*`
- Windows 10 x64: NSIS installer + portable ZIP (с маркером `portable.txt`, без updater)
- macOS aarch64: `.dmg` (ad-hoc, без подписи)
- Linux x86_64: `.AppImage` + `.deb`
- SHA256-checksums на все артефакты
- Корневой `README.md` (RU) с инструкциями запуска по ОС

**НЕ в scope (вынесено/отложено):**
- AD-вход, заявки на регистрацию (вынесено в будущую Phase 9 — SPIDR-split 2026-06-18)
- Windows 7 / 32-bit (i686) — пользователь явно отказался, цель только Win10 x64
- Code-signing сертификаты и нотаризация (отложено до появления сертов)
- Bundled fixed-version WebView2 runtime (используем системный Evergreen)

</domain>

<decisions>
## Implementation Decisions

### Целевые платформы и объём матрицы
- **D-01:** Windows-цель = **только Windows 10 x64**. Windows 7 и 32-bit (i686) полностью убраны из scope — НЕ добавлять `i686-pc-windows-msvc`, НЕ настраивать `embedBootstrapper`/Win7-специфику из CLAUDE.md.
- **D-02:** Linux собираем в обоих форматах **`.AppImage` + `.deb`** (как в BLD-02, без изменений).
- **D-03:** macOS — `.dmg` для aarch64, низкий приоритет (dev-платформа разработчика).

### Подпись и checksums (BLD-03)
- **D-04:** **Без code-signing во всех ОС в v1.** Windows .exe/installer — не подписываем; macOS — ad-hoc (без Apple Developer ID); Linux — без подписи. BLD-03 «подписи по возможности» закрывается через checksums; подпись отложена.
- **D-05:** На все артефакты — **SHA256-checksums**. Формат — единый файл `SHA256SUMS` на весь релиз (Claude's discretion, см. ниже; per-file `.sha256` допустим если так проще с tauri-action).
- **D-06:** В README документировать обход предупреждений **SmartScreen** (Windows) и **Gatekeeper** (macOS), раз артефакты неподписанные.

### Portable ZIP (BLD-04)
- **D-07:** WebView2 — **системный Evergreen**, runtime НЕ бандлим (на Win10 x64 практически всегда присутствует). НЕ использовать fixed-version/offline installer вариант.
- **D-08:** Содержимое portable ZIP: `trackly.exe` (raw release-бинарник) + пустой `portable.txt` (маркер portable-режима) + краткий README по запуску + закомментированный шаблон `trackly.config.toml` для ручной настройки пути БД/порта.
- **D-09:** Portable ZIP — **без updater** (соответствует portable-дисциплине из CLAUDE.md; `tauri-plugin-updater` не включать).
- **D-10:** Portable-маркер уже определён в Phase 1: наличие `portable.txt` ИЛИ `trackly.config.toml` рядом с .exe сигналит portable-режим (`trackly_infra::paths`). Пайплайн только кладёт `portable.txt` в ZIP — логика уже есть.

### Триггер и версионирование (BLD-02)
- **D-11:** Триггер — **push git-тега `v*.*.*`**.
- **D-12:** Релиз создаётся как **draft** (tauri-action `releaseDraft: true`); публикация — вручную после проверки артефактов. НЕ авто-publish.
- **D-13:** **Тег — источник истины для версии.** CI извлекает версию из тега (`v1.2.3` → `1.2.3`) и подставляет её в `Cargo.toml` (workspace `version`) и `tauri.conf.json` перед сборкой. Не требуется руками править версии в файлах перед тегированием. (Сейчас оба = `0.1.0`.)
- **D-14:** Сборку вести через **`tauri-apps/tauri-action`** (рекомендация CLAUDE.md), а не hand-rolled bundling. Включить `bundle.active` в `tauri.conf.json` (сейчас `false`) и задать per-OS targets.

### Release notes
- **D-15:** **GitHub auto-generated release notes** (из PR/коммитов между тегами). Ручной CHANGELOG.md не ведём. Описание при необходимости редактируется в draft перед публикацией.

### Документация (BLD-05)
- **D-16:** Создать **корневой `/README.md`** на русском (сейчас корневого README нет). Полный: описание проекта, запуск по каждой ОС (Win/macOS/Linux), portable-режим, требование WebView2 на Windows, серверный режим + подсказки по доверию self-signed сертификату в локальной сети, обход SmartScreen/Gatekeeper.

### Claude's Discretion
- Точный формат checksums-файла (единый `SHA256SUMS` vs per-file `.sha256`) — на усмотрение, ориентир — единый `SHA256SUMS`.
- Структура `release.yml` (отдельный workflow vs расширение существующего) — рекомендуется **отдельный `release.yml`**, т.к. `ci-full.yml`/`ci-fast.yml` уже заняты PR/push-проверками.
- Конкретный механизм подстановки версии из тега (sed/скрипт/`cargo set-version`).
- Retention/имена артефактов в Release.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Требования и план фазы
- `.planning/ROADMAP.md` §"Phase 8: Релизный пайплайн" — goal, success criteria (release matrix + README), requirements BLD-02..05.
- `.planning/REQUIREMENTS.md` — BLD-02 (release matrix), BLD-03 (checksums/подписи), BLD-04 (portable без updater + `portable.txt`), BLD-05 (README RU).

### Стек и release-практики (ОБЯЗАТЕЛЬНО)
- `CLAUDE.md` — разделы "CI / Release", "Tauri Plugins" (updater = skip в portable), "Stack Patterns by Variant" (portable mode discipline), "What NOT to Use" (updater в portable; `native-tls`/OpenSSL → rustls). Примечание: Win7/32-bit/`embedBootstrapper` рекомендации CLAUDE.md в этой фазе НЕ применяем (D-01).
- `.planning/research/STACK.md` — pinned версии, stack patterns по вариантам (portable mode, server mode).

### Существующий код для расширения/переиспользования
- `.github/workflows/ci-full.yml` — рабочая matrix (ubuntu/macos/windows-latest, Rust 1.88, pnpm 10, Node 20, Tauri 2 Linux system deps). Источник правды по toolchain/зависимостям для release-сборки.
- `.github/workflows/ci-fast.yml` — нюансы тестов (`--test-threads=1`).
- `crates/trackly-app/tauri.conf.json` — `bundle.active: false` (включить), `productName: Trackly`, `identifier: org.trackly.app`, `version: 0.1.0`.
- `.planning/phases/01-foundation/01-CONTEXT.md` §"Маркер портативности" — `portable.txt`/`trackly.config.toml` логика, `trackly_infra::paths`.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`ci-full.yml` matrix**: готовый набор шагов установки (Rust 1.88 toolchain, `Swatinem/rust-cache@v2`, Tauri 2 Linux apt-deps, `pnpm/action-setup@v4` v10, `actions/setup-node@v4` Node 20, frozen lockfile). Release workflow переиспользует те же setup-шаги.
- **`procmon-check` крейт** (`tools/procmon-check`): валидирует portable-режим (no writes вне каталога .exe) на Windows — может использоваться как пост-сборочная проверка portable ZIP.

### Established Patterns
- **Portable-маркер** (Phase 1): `portable.txt` ИЛИ `trackly.config.toml` рядом с .exe; пути резолвятся через `trackly_infra::paths` от `current_exe()`, не через `dirs`. Pipeline лишь добавляет `portable.txt` в ZIP.
- **Версия из workspace**: `crates/trackly-app/Cargo.toml` использует `version.workspace = true`; `tauri.conf.json` дублирует `0.1.0`. D-13 требует синхронной подстановки из тега в оба места.

### Integration Points
- `tauri.conf.json` `bundle` блок — точка включения сборки дистрибутивов (`active: true`, per-OS `targets`, иконки — сейчас только `icon.png`, для bundling могут понадобиться `.ico`/`.icns`).
- Новый `.github/workflows/release.yml` — точка входа пайплайна.

</code_context>

<specifics>
## Specific Ideas

- Пользователь ведёт разработку на macOS вне корпоративной сети → Windows-сборку из этой фазы он будет ставить на реальный Win10 x64 в домене для последующего теста AD-входа (Phase 9). Поэтому release-пайплайн идёт ПЕРЕД AD-фазой (обоснование reordering, SPIDR 2026-06-18).
- Артефакты должны быть «готовы к раздаче»: пользователь скачивает из Release, проверяет checksum, запускает — без установки доп. зависимостей (кроме системного WebView2 на Windows).

</specifics>

<deferred>
## Deferred Ideas

- **Code-signing Windows (OV/EV cert)** — добавить шаг подписи в `release.yml`, когда появится сертификат (через GitHub secret: base64 .pfx + пароль). Сейчас отложено (D-04).
- **macOS подпись + нотаризация (Apple Developer ID + notarytool)** — отложено до появления Developer ID (D-04).
- **Windows 7 / 32-bit (i686)** — явно вне scope, не возвращать без запроса пользователя (D-01).
- **Bundled fixed-version WebView2 runtime** — на случай оффлайн-машин без Evergreen; сейчас полагаемся на системный (D-07).
- **AD-вход, заявки на регистрацию, авто-приём (USR-08..12, REQ-06, SET-10)** — будущая Phase 9 (создать через `/gsd add-phase`, затем `/gsd mvp-phase 9`). Авто-SSO (не только simple_bind) — см. память проекта `phase8_split_ad_sso`.

</deferred>

---

*Phase: 08-windows-macos-linux*
*Context gathered: 2026-06-19*
