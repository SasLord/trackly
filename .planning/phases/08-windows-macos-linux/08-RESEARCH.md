# Phase 8: Релизный пайплайн (Windows/macOS/Linux) - Research

**Researched:** 2026-06-19
**Domain:** GitHub Actions CI/CD, tauri-apps/tauri-action, Tauri 2 bundler, portable ZIP, SHA256SUMS aggregation
**Confidence:** HIGH (основной стек — tauri-action, структура workflow, icon gen); MEDIUM (версионирование из тега, portable ZIP вручную, macOS ad-hoc intermittency)

---

## Summary

Эта фаза добавляет GitHub Actions workflow `release.yml`, который по пушу тега `v*.*.*` собирает дистрибутивы Trackly для Windows 10 x64, macOS aarch64 и Linux x86_64. Основной инструмент — `tauri-apps/tauri-action@v0` (текущая версия 0.6.2, март 2026). Ключевая сложность — три проблемы, которые tauri-action не решает из коробки: portable ZIP для Windows, агрегация SHA256SUMS из артефактов нескольких matrix-джобов, и race condition при параллельном создании draft Release несколькими джобами.

**Рекомендуемый паттерн:** three-job структура — `create-release` (создаёт draft, отдаёт `release_id`) → `build` (matrix по трём ОС, tauri-action с `releaseId`) → `checksums` (downloads все артефакты, генерирует SHA256SUMS, заливает). Portable ZIP собирается дополнительным шагом в Windows-джобе: raw `trackly.exe` + `portable.txt` + `README-portable.md` + закомментированный `trackly.config.toml.example` архивируются PowerShell-командой и заливаются через `gh release upload`.

Версия подставляется из тега (`v1.2.3` → `1.2.3`) через sed в корневой `Cargo.toml` (поле `workspace.package.version`) и jq в `tauri.conf.json` — до запуска tauri-action. Это надёжнее cargo-edit (не требует установки в CI) и надёжнее `__VERSION__` заглушки tauri-action (которая читает из tauri.conf.json, а не из тега).

**Primary recommendation:** Использовать three-job workflow (create-release → build matrix → checksums), подставлять версию из тега через sed+jq в шаге перед tauri-action, portable ZIP — ручная сборка PowerShell на Windows-джобе.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Windows-цель = **только Windows 10 x64**. Windows 7 и 32-bit (i686) полностью убраны из scope — НЕ добавлять `i686-pc-windows-msvc`, НЕ настраивать `embedBootstrapper`/Win7-специфику из CLAUDE.md.
- **D-02:** Linux собираем в обоих форматах **`.AppImage` + `.deb`** (как в BLD-02, без изменений).
- **D-03:** macOS — `.dmg` для aarch64, низкий приоритет (dev-платформа разработчика).
- **D-04:** **Без code-signing во всех ОС в v1.** Windows .exe/installer — не подписываем; macOS — ad-hoc (без Apple Developer ID); Linux — без подписи. BLD-03 «подписи по возможности» закрывается через checksums; подпись отложена.
- **D-05:** На все артефакты — **SHA256-checksums**. Формат — единый файл `SHA256SUMS` на весь релиз (per-file `.sha256` допустим если так проще с tauri-action).
- **D-06:** В README документировать обход предупреждений **SmartScreen** (Windows) и **Gatekeeper** (macOS), раз артефакты неподписанные.
- **D-07:** WebView2 — **системный Evergreen**, runtime НЕ бандлим (на Win10 x64 практически всегда присутствует). НЕ использовать fixed-version/offline installer вариант.
- **D-08:** Содержимое portable ZIP: `trackly.exe` (raw release-бинарник) + пустой `portable.txt` (маркер portable-режима) + краткий README по запуску + закомментированный шаблон `trackly.config.toml` для ручной настройки пути БД/порта.
- **D-09:** Portable ZIP — **без updater** (соответствует portable-дисциплине из CLAUDE.md; `tauri-plugin-updater` не включать).
- **D-10:** Portable-маркер уже определён в Phase 1: наличие `portable.txt` ИЛИ `trackly.config.toml` рядом с .exe сигналит portable-режим (`trackly_infra::paths`). Пайплайн только кладёт `portable.txt` в ZIP — логика уже есть.
- **D-11:** Триггер — **push git-тега `v*.*.*`**.
- **D-12:** Релиз создаётся как **draft** (tauri-action `releaseDraft: true`); публикация — вручную после проверки артефактов. НЕ авто-publish.
- **D-13:** **Тег — источник истины для версии.** CI извлекает версию из тега (`v1.2.3` → `1.2.3`) и подставляет её в `Cargo.toml` (workspace `version`) и `tauri.conf.json` перед сборкой. Не требуется руками править версии в файлах перед тегированием. (Сейчас оба = `0.1.0`.)
- **D-14:** Сборку вести через **`tauri-apps/tauri-action`** (рекомендация CLAUDE.md), а не hand-rolled bundling. Включить `bundle.active` в `tauri.conf.json` (сейчас `false`) и задать per-OS targets.
- **D-15:** **GitHub auto-generated release notes** (из PR/коммитов между тегами). Ручной CHANGELOG.md не ведём. Описание при необходимости редактируется в draft перед публикацией.
- **D-16:** Создать **корневой `/README.md`** на русском (сейчас корневого README нет).

### Claude's Discretion

- Точный формат checksums-файла (единый `SHA256SUMS` vs per-file `.sha256`) — на усмотрение, ориентир — единый `SHA256SUMS`.
- Структура `release.yml` (отдельный workflow vs расширение существующего) — рекомендуется **отдельный `release.yml`**.
- Конкретный механизм подстановки версии из тега (sed/скрипт/`cargo set-version`).
- Retention/имена артефактов в Release.

### Deferred Ideas (OUT OF SCOPE)

- **Code-signing Windows (OV/EV cert)** — отложено до появления сертификата.
- **macOS подпись + нотаризация (Apple Developer ID + notarytool)** — отложено.
- **Windows 7 / 32-bit (i686)** — явно вне scope.
- **Bundled fixed-version WebView2 runtime** — отложено.
- **AD-вход, заявки на регистрацию, авто-приём (USR-08..12, REQ-06, SET-10)** — будущая Phase 9.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BLD-02 | GitHub Actions Release: при push-тега `v*.*.*` собирать релизы для Windows 64-bit (NSIS installer + portable ZIP), macOS aarch64 (.dmg), Linux x86_64 (.AppImage + .deb) | tauri-action matrix workflow, bundle.targets конфигурация, three-job структура |
| BLD-03 | Артефакты релиза включают checksums (SHA256) и подписи (по возможности) | SHA256SUMS в финальном checksums-джобе; подписи — D-04 закрывает через checksums |
| BLD-04 | Сборка portable варианта без updater, с включённым маркером `portable.txt` | raw exe из `target/release/`, PowerShell zip, portable.txt, uploadUpdaterJson: false |
| BLD-05 | Документация по запуску (README на русском) с инструкциями для каждой ОС | Корневой README.md с инструкциями по ОС, WebView2, SmartScreen/Gatekeeper |
</phase_requirements>

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Сборка бинарей (Tauri bundle) | CI Runner (OS-specific) | — | Tauri требует нативных runners: Windows для .exe/NSIS, macOS для .dmg, Linux для AppImage/.deb |
| Draft Release creation | CI Job: create-release | — | Единый job предотвращает race condition при параллельных matrix-джобах |
| Artifact upload | CI Job: build (matrix) | tauri-action | tauri-action управляет загрузкой через GitHub API по releaseId |
| Portable ZIP assembly | CI Job: build (Windows) | PowerShell step | tauri-action не умеет portable ZIP; собираем вручную после tauri build |
| SHA256SUMS aggregation | CI Job: checksums | actions/download-artifact | Скачивает все артефакты из matrix-джобов, генерирует единый файл |
| Версионирование из тега | CI Step: pre-build | sed + jq | Подстановка версии до запуска tauri-action; tauri-action читает из tauri.conf.json |
| README.md | Source tree | — | Статический файл в корне репо, пишется один раз |
| Icon generation | Local dev step | pnpm tauri icon | Иконки уже сгенерированы в `crates/trackly-app/icons/`; tauri.conf.json надо расширить |

---

## Standard Stack

### Core

| Tool/Action | Version | Purpose | Why Standard |
|-------------|---------|---------|--------------|
| `tauri-apps/tauri-action` | `@v0` (0.6.2, март 2026) | Запуск `tauri build`, загрузка артефактов в GitHub Release | Официальный action от tauri-apps; поддерживает matrix, draft, releaseId [CITED: github.com/tauri-apps/tauri-action] |
| `actions/checkout` | `@v4` | Checkout | Синхронизировать с ci-full.yml |
| `dtolnay/rust-toolchain@stable` | `toolchain: '1.88'` | Rust toolchain | Синхронизировать с ci-full.yml (pinned 1.88) |
| `Swatinem/rust-cache` | `@v2` | Cargo кэш | Синхронизировать с ci-full.yml |
| `pnpm/action-setup` | `@v4` + version `10` | pnpm | Синхронизировать с ci-full.yml |
| `actions/setup-node` | `@v4` + node `20` | Node.js | Синхронизировать с ci-full.yml |
| `actions/upload-artifact` | `@v4` | Сохранение артефактов между джобами | Нужен для агрегации в checksums-джобе |
| `actions/download-artifact` | `@v4` | Скачивание артефактов в checksums-джобе | Merge-multiple: true поддерживается в v4 [CITED: github.com/actions/upload-artifact] |
| `actions/github-script` | `@v7` | Создание draft release через GitHub API | Возвращает release_id для дальнейшего использования |

### Supporting

| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| `jq` | системный (1.7+ на ubuntu-latest) | Обновление version в tauri.conf.json | В шаге подстановки версии на каждом runner |
| `sed` | системный GNU | Обновление version в Cargo.toml workspace | Только Linux/macOS runners; Windows использует PowerShell |
| PowerShell | системный | Сборка portable ZIP, SHA256 на Windows | windows-latest runner |
| `shasum -a 256` / `sha256sum` | системный | SHA256 вычисление на macOS/Linux | В checksums-джобе |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| sed + jq для версии | `cargo set-version` (cargo-edit) | cargo-edit не установлен в CI по умолчанию; требует `cargo install cargo-edit` (~2 мин). sed+jq надёжнее и быстрее. |
| sed + jq для версии | tauri-action `__VERSION__` placeholder | `__VERSION__` читает из tauri.conf.json, не из тега — не решает D-13 «тег = источник истины». |
| actions/github-script для create-release | `gh release create` | `gh` доступен на всех runner'ах, но труднее получить release_id в YAML-выходе; github-script возвращает ID напрямую. Оба варианта рабочие. |
| three-job workflow | Все в matrix без create-release | Race condition — несколько matrix-джобов иногда создают duplicate release для того же тега (issue #914). Отдельный create-release job исключает race. |

---

## Package Legitimacy Audit

> Эта фаза не добавляет новых npm/cargo зависимостей в приложение. Добавляются только GitHub Actions (YAML). Package Legitimacy Gate не применяется.

**Packages removed due to slopcheck:** none
**Packages flagged as suspicious:** none

---

## Architecture Patterns

### System Architecture Diagram

```
git push tag v1.2.3
        |
        v
[on: push tags v*.*.*]
        |
        +---> Job: create-release (ubuntu-latest)
        |         |
        |         |  actions/github-script → GitHub API
        |         |  → creates draft Release for tag
        |         |  → outputs: release_id
        |         |
        v         v
[needs: create-release]
        |
        +---> Job: build (matrix: windows / macos / linux)
        |         |
        |         |  Step: Extract version (sed + jq)
        |         |  → Cargo.toml workspace.package.version = "1.2.3"
        |         |  → tauri.conf.json .version = "1.2.3"
        |         |
        |         |  Step: tauri-action@v0
        |         |  args per OS: (see matrix below)
        |         |  releaseId: ${{ needs.create-release.outputs.release_id }}
        |         |  uploadUpdaterJson: false   ← D-09 (no updater)
        |         |  releaseDraft: true
        |         |
        |         |  [Windows only] Step: Assemble portable ZIP
        |         |  target\release\trackly.exe
        |         |  + portable.txt (empty)
        |         |  + README-portable.md
        |         |  + trackly.config.toml.example
        |         |  → PowerShell Compress-Archive → trackly-v1.2.3-windows-portable.zip
        |         |  → gh release upload <release_id> ...zip
        |         |
        |         |  Step: Upload artifacts to workflow
        |         |  (upload-artifact для checksums-джоба)
        |         |
        v         v
[needs: build]
        |
        +---> Job: checksums (ubuntu-latest)
                  |
                  |  Step: download-artifact (merge-multiple: true)
                  |  → все файлы в ./artifacts/
                  |
                  |  Step: sha256sum ./* → SHA256SUMS
                  |
                  |  Step: gh release upload SHA256SUMS
```

### Recommended Project Structure

```
.github/
└── workflows/
    ├── ci-fast.yml          # существующий — без изменений
    ├── ci-full.yml          # существующий — без изменений
    ├── cargo-deny.yml       # существующий — без изменений
    └── release.yml          # НОВЫЙ — эта фаза

crates/trackly-app/
├── tauri.conf.json          # bundle.active: true, targets, icons, macOS signingIdentity
└── icons/                   # уже содержит .ico, .icns, .png (32x32, 128x128, etc.)

README.md                    # НОВЫЙ корневой файл (BLD-05)
```

### Pattern 1: Three-Job Workflow (create-release → build → checksums)

**What:** Разделение на три последовательных job устраняет race condition и позволяет агрегировать артефакты с разных ОС в единый SHA256SUMS файл.

**When to use:** Всегда при matrix-сборке с несколькими параллельными джобами, загружающими в один Release.

**Example:**

```yaml
# Source: github.com/tauri-apps/tauri-action (publish-to-manual-release.yml example)
jobs:
  create-release:
    runs-on: ubuntu-latest
    outputs:
      release_id: ${{ steps.create-release.outputs.result }}
    steps:
      - uses: actions/checkout@v4
      - name: Create draft release
        id: create-release
        uses: actions/github-script@v7
        with:
          script: |
            const tag = context.ref.replace('refs/tags/', '')
            const version = tag.replace(/^v/, '')
            const { data } = await github.rest.repos.createRelease({
              owner: context.repo.owner,
              repo: context.repo.repo,
              tag_name: tag,
              name: `Trackly ${version}`,
              draft: true,
              generate_release_notes: true,
            })
            return data.id

  build:
    needs: create-release
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: windows-latest
            args: '--bundles nsis'
          - platform: macos-latest
            args: '--target aarch64-apple-darwin --bundles dmg'
          - platform: ubuntu-22.04
            args: '--bundles appimage,deb'
    runs-on: ${{ matrix.platform }}
    steps:
      # ... setup steps ...
      - name: Extract version from tag
        shell: bash
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          # Cargo.toml workspace version
          sed -i "s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml
          # tauri.conf.json version
          jq --arg v "$VERSION" '.version = $v' \
            crates/trackly-app/tauri.conf.json > /tmp/tauri.conf.json.tmp
          mv /tmp/tauri.conf.json.tmp crates/trackly-app/tauri.conf.json

      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          APPLE_SIGNING_IDENTITY: '-'    # ad-hoc, macOS only (D-04)
        with:
          releaseId: ${{ needs.create-release.outputs.release_id }}
          uploadUpdaterJson: false
          projectPath: crates/trackly-app
          args: ${{ matrix.args }}

      # Windows portable ZIP (отдельный шаг, см. Pattern 2)
```

### Pattern 2: Portable ZIP Assembly (Windows)

**What:** tauri-action не умеет создавать portable ZIP — это надо делать отдельным шагом после tauri build. Используем PowerShell Compress-Archive.

**When to use:** Только на Windows runner, после завершения шага tauri-action.

**Where is the raw .exe:** `target\release\trackly.exe` (без Rust target triple prefix, т.к. Windows x64 — нативная цель runner'а). [VERIFIED: v2.tauri.app/distribute/windows-installer]

**Example:**

```yaml
# Source: [ASSUMED] — паттерн из community + tauri-action issue #302
- name: Assemble portable ZIP (Windows only)
  if: matrix.platform == 'windows-latest'
  shell: pwsh
  run: |
    $VERSION = $env:GITHUB_REF_NAME -replace '^v', ''
    $ZIP_NAME = "trackly-v${VERSION}-windows-x64-portable.zip"
    $STAGE = "portable-stage"
    New-Item -ItemType Directory -Force -Path $STAGE

    Copy-Item "target\release\trackly.exe" "$STAGE\trackly.exe"
    New-Item -ItemType File -Path "$STAGE\portable.txt" -Force  # пустой маркер
    Copy-Item "README-portable.md" "$STAGE\README.md"
    Copy-Item "trackly.config.toml.example" "$STAGE\trackly.config.toml.example"

    Compress-Archive -Path "$STAGE\*" -DestinationPath $ZIP_NAME
    echo "PORTABLE_ZIP=$ZIP_NAME" >> $env:GITHUB_ENV

- name: Upload portable ZIP to release
  if: matrix.platform == 'windows-latest'
  shell: bash
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
    gh release upload \
      "${{ needs.create-release.outputs.release_id }}" \
      "$PORTABLE_ZIP" \
      --repo "${{ github.repository }}"
```

> **Примечание:** `gh release upload` принимает release_id как числовой ID или tag_name. По умолчанию принимает tag. Проверить синтаксис: `gh release upload <tag> <file>`.

### Pattern 3: Version Injection из тега

**What:** Подстановка `1.2.3` из `refs/tags/v1.2.3` в два места: workspace `Cargo.toml` и `tauri.conf.json`.

**Why sed+jq, не cargo-edit:** cargo-edit требует `cargo install cargo-edit` — +2 мин на каждый runner, нет в стандартных image. jq всегда присутствует на ubuntu-latest/macos-latest. На Windows: PowerShell.

**Pitfall — Windows sed:** На `windows-latest` GNU tools не установлены по умолчанию. Использовать PowerShell для замены версии на Windows runner:

```powershell
# Cargo.toml (workspace version)
$content = Get-Content "Cargo.toml" -Raw
$content = $content -replace 'version = "0\.\d+\.\d+"', "version = `"$VERSION`""
Set-Content "Cargo.toml" $content

# tauri.conf.json
$json = Get-Content "crates\trackly-app\tauri.conf.json" -Raw | ConvertFrom-Json
$json.version = $VERSION
$json | ConvertTo-Json -Depth 10 | Set-Content "crates\trackly-app\tauri.conf.json"
```

**Альтернатива — shell: bash работает на всех runner'ах в GitHub Actions (Git Bash на Windows):** `shell: bash` в шаге позволяет использовать одни и те же sed/jq-команды на всех ОС.

### Pattern 4: SHA256SUMS Aggregation

**What:** Финальный job скачивает артефакты из всех matrix-джобов, генерирует единый `SHA256SUMS`, заливает в Release.

**Example:**

```yaml
  checksums:
    needs: [create-release, build]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          pattern: release-artifacts-*
          merge-multiple: true
          path: ./artifacts

      - name: Generate SHA256SUMS
        working-directory: ./artifacts
        run: |
          sha256sum * > SHA256SUMS
          cat SHA256SUMS

      - name: Upload SHA256SUMS to release
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          TAG="${GITHUB_REF_NAME}"
          gh release upload "$TAG" ./artifacts/SHA256SUMS \
            --repo "${{ github.repository }}"
```

**Upload-artifact в build job (каждый matrix):**

```yaml
      - name: Upload artifacts for checksums job
        uses: actions/upload-artifact@v4
        with:
          name: release-artifacts-${{ matrix.platform }}
          path: |
            target/release/bundle/**/*.exe
            target/release/bundle/**/*.msi
            target/release/bundle/**/*.deb
            *.AppImage
            *.dmg
            *-portable.zip
          if-no-files-found: warn
```

### Anti-Patterns to Avoid

- **Нет create-release job — все matrix-джобы пытаются создать Release:** Приводит к race condition (duplicate releases, issue #914). Всегда использовать отдельный create-release job.
- **uploadUpdaterJson: true (по умолчанию):** Генерирует `latest.json` — не нужен в portable-режиме и запутывает релиз. Явно ставить `uploadUpdaterJson: false`.
- **Использование `uploadPlainBinary: true` как замена portable ZIP:** uploadPlainBinary заливает raw .exe без portable.txt — пользователь получит .exe без маркера, приложение не войдёт в portable mode. Не использовать.
- **`bundle.active: false` в tauri.conf.json:** Текущее значение — сборка дистрибутивов не происходит. ОБЯЗАТЕЛЬНО включить `bundle.active: true` перед запуском tauri-action.
- **`bundle.icon` только `icons/icon.png`:** Текущее значение — на Windows может не собрать правильную иконку в NSIS. Расширить до полного списка (ico, icns, png).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Сборка NSIS installer | Собственный NSIS скрипт | tauri-action@v0 + `--bundles nsis` | tauri-bundler уже содержит готовый NSIS шаблон с WebView2 runtime detect |
| Загрузка артефактов в GitHub Release | curl + GitHub API вручную | tauri-action@v0 `releaseId` | tauri-action управляет загрузкой, retry, asset naming |
| Кросс-компиляция Windows из macOS | cargo-zigbuild или cross | Нативный `windows-latest` runner | Tauri WebView2 требует MSVC target; zig может *-gnu но не MSVC |
| macOS DMG creation | Ручная diskutil | tauri-action@v0 + `--bundles dmg` | tauri-bundler содержит bundle_dmg.sh скрипт |
| Icon generation | Ручные convert/inkscape | `pnpm tauri icon` | Генерирует все форматы (.ico, .icns, множество .png) из одного исходника [CITED: v2.tauri.app/develop/icons] |

**Key insight:** tauri-action@v0 покрывает 90% release pipeline; portable ZIP — единственное, что нужно делать вручную.

---

## Common Pitfalls

### Pitfall 1: Race condition при создании Draft Release

**What goes wrong:** Несколько matrix-джобов запускаются параллельно, каждый пытается создать release для одного тега — иногда создаётся 2+ duplicate release, артефакты расходятся по разным релизам. [CITED: github.com/tauri-apps/tauri-action/issues/914]

**Why it happens:** tauri-action проверяет: «есть ли release для этого тега?» При старте всех джобов почти одновременно, проверка может вернуть «нет» для всех, и каждый создаёт свой.

**How to avoid:** Отдельный `create-release` job, который создаёт draft **до** запуска matrix; matrix-джобы получают `release_id` через `needs.create-release.outputs.release_id` и используют параметр `releaseId` tauri-action.

**Warning signs:** В GitHub Release появляются две записи с одинаковым тегом; одна из них пустая.

### Pitfall 2: bundle.active: false — tauri-action не создаёт дистрибутивы

**What goes wrong:** tauri-action запускается, но выходные файлы — только raw .exe, без NSIS installer, .dmg, AppImage. CI завершается успешно, но артефактов нет.

**Why it happens:** `bundle.active: false` в `crates/trackly-app/tauri.conf.json` — текущее состояние проекта.

**How to avoid:** Установить `bundle.active: true` в tauri.conf.json в рамках этой фазы.

**Warning signs:** В Release нет `.exe` installer или `.dmg` файлов, только raw binary.

### Pitfall 3: tauri.conf.json bundle.icon — только icon.png

**What goes wrong:** На Windows NSIS installer использует неправильную иконку (или generic иконку Tauri). На macOS .dmg может использовать fallback иконку.

**Why it happens:** `bundle.icon: ["icons/icon.png"]` — текущее значение. Нужны `.ico` и `.icns`. Иконки уже сгенерированы в `crates/trackly-app/icons/` (icon.ico, icon.icns, 32x32.png, 128x128.png, 128x128@2x.png).

**How to avoid:** Обновить `bundle.icon` в tauri.conf.json на полный список.

**Warning signs:** NSIS installer показывает generic иконку Tauri.

### Pitfall 4: macOS ad-hoc signing — intermittent DMG bundling failure

**What goes wrong:** Build завершается ошибкой `failed to run bundle_dmg.sh` примерно в 5-10% случаев. Re-run без изменений проходит. [CITED: github.com/tauri-apps/tauri/issues/13804]

**Why it happens:** Non-deterministic flakiness в процессе подписи/создания DMG на macOS GitHub Actions runner; закрыто как дубликат #3055.

**How to avoid:** Добавить `retryAttempts: 1` в tauri-action inputs для macOS job. Это встроенная функция action.

**Warning signs:** macOS job падает с `bundle_dmg.sh` ошибкой при первой попытке, но проходит при retry.

### Pitfall 5: portable ZIP не содержит portable.txt — приложение не входит в portable mode

**What goes wrong:** Пользователь скачивает и распаковывает ZIP, запускает trackly.exe — приложение пишет данные в `%APPDATA%` вместо рядом с .exe.

**Why it happens:** portable.txt не был включён в ZIP при сборке; или `uploadPlainBinary: true` залил только raw .exe без сопутствующих файлов.

**How to avoid:** Явно создавать `portable.txt` (пустой файл) в stage-директории перед `Compress-Archive`. Никогда не использовать `uploadPlainBinary: true` вместо явной сборки portable ZIP.

**Warning signs:** В `%APPDATA%\org.trackly.app` или `%LOCALAPPDATA%` появляются файлы после запуска из portable ZIP.

### Pitfall 6: Версия не обновляется в Cargo.toml или tauri.conf.json перед сборкой

**What goes wrong:** Бинарь собирается с `0.1.0` вместо версии из тега; `tauri build` выдаёт артефакты с неверной версией в имени файла.

**Why it happens:** Шаг подстановки версии не выполнен или failed silently.

**How to avoid:** Шаг версии должен быть **до** pnpm install / cargo build / tauri-action. Явно проверять результат: `grep 'version = "' Cargo.toml` после sed.

**Warning signs:** Имя артефакта в Release содержит `0.1.0` вместо ожидаемой версии.

### Pitfall 7: Linux system deps — AppImage/deb не собирается

**What goes wrong:** Cargo build падает с ошибкой `pkg-config: package webkit2gtk-4.1 not found` на ubuntu-22.04 runner.

**Why it happens:** ci-full.yml уже устанавливает нужные пакеты, но release.yml нужно их воспроизвести.

**How to avoid:** Скопировать шаг `Install Tauri 2 Linux system dependencies` из ci-full.yml 1-в-1 в release.yml для Linux runner.

**Warning signs:** Ошибка компиляции вида `error: failed to run custom build command for 'webkit2gtk-sys'`.

### Pitfall 8: Aggregating checksums — артефакты tauri-action не видны в следующем job

**What goes wrong:** checksums-джоб запускает `sha256sum *` в пустой директории.

**Why it happens:** tauri-action загружает артефакты **напрямую в GitHub Release**, но не сохраняет их как GitHub Actions artifacts (workflow artifacts). Для checksums-джоба нужно явно сохранить артефакты через `actions/upload-artifact@v4`.

**How to avoid:** После tauri-action и после сборки portable ZIP явно использовать `actions/upload-artifact@v4` для сохранения всех артефактов с уникальным именем (`release-artifacts-${{ matrix.platform }}`). checksums-джоб скачивает через `actions/download-artifact@v4` с `merge-multiple: true`.

---

## Code Examples

### tauri.conf.json — обновлённый bundle блок

```json
// Source: [CITED: v2.tauri.app/reference/config/#bundleconfig] + project icons/ contents
{
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "macOS": {
      "signingIdentity": "-"
    }
  }
}
```

> `"targets": "all"` работает корректно т.к. каждый runner генерирует только платформенно-совместимые targets. Windows runner не будет пытаться создать .dmg; Linux runner не будет создавать NSIS. Альтернатива — явный массив через `args: --bundles nsis` в tauri-action.

### Извлечение версии из тега (shell: bash, работает на всех runner'ах)

```bash
# Source: [ASSUMED] — стандартный bash паттерн
VERSION="${GITHUB_REF_NAME#v}"  # v1.2.3 → 1.2.3

# 1. Обновить workspace version в Cargo.toml
# Ищем строку `version = "X.Y.Z"` в блоке [workspace.package] — только первое вхождение
sed -i "0,/^version = \"[0-9]*\.[0-9]*\.[0-9]*\"/{s/^version = \"[0-9]*\.[0-9]*\.[0-9]*\"/version = \"${VERSION}\"/}" Cargo.toml

# 2. Обновить version в tauri.conf.json
jq --arg v "$VERSION" '.version = $v' \
  crates/trackly-app/tauri.conf.json > /tmp/_tauri.conf.json
mv /tmp/_tauri.conf.json crates/trackly-app/tauri.conf.json

# Проверка
grep 'version = "' Cargo.toml | head -1
jq .version crates/trackly-app/tauri.conf.json
```

> **Важно:** `Cargo.toml` содержит строку `version = "0.1.0"` в секции `[workspace.package]`. sed с `0,...` заменяет только первое вхождение (остальные `version = ...` в `[workspace.dependencies]` — это версии зависимостей, трогать нельзя). Проверить после замены.

### tauri-action inputs полная спецификация

```yaml
# Source: [CITED: github.com/tauri-apps/tauri-action README]
- uses: tauri-apps/tauri-action@v0
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
    APPLE_SIGNING_IDENTITY: ${{ runner.os == 'macOS' && '-' || '' }}
  with:
    releaseId: ${{ needs.create-release.outputs.release_id }}
    releaseDraft: true          # safety net — если create-release не сработал
    uploadUpdaterJson: false    # D-09: no updater
    retryAttempts: 1            # Pitfall 4: macOS DMG flakiness
    projectPath: crates/trackly-app
    args: ${{ matrix.args }}
```

### permissions блок в release.yml

```yaml
# Source: [CITED: v2.tauri.app/distribute/pipelines/github]
permissions:
  contents: write   # ОБЯЗАТЕЛЬНО для создания Release и загрузки артефактов
```

> Без `contents: write` — ошибка "Resource not accessible by integration".

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `tauri-apps/tauri-action@v0.5` | `tauri-apps/tauri-action@v0.6.2` | март 2026 | Добавлен `uploadPlainBinary`, `releaseAssetNamePattern`, retry. Используем `@v0` — floating tag на latest 0.x |
| `actions/upload-artifact@v3` | `actions/upload-artifact@v4` | 2024 | v4: нельзя писать в одноимённый artifact дважды; нужны уникальные имена в matrix |
| Tauri 1 `bundle.targets` | Tauri 2 `bundle.targets` | oct 2024 | Таргеты те же, структура конфига немного изменилась (macOS sub-object для signingIdentity) |
| `releaseName: "v__VERSION__"` placeholder | Явное имя через github-script | — | Placeholder работает только если tauri.conf.json уже содержит правильную версию |

**Deprecated/outdated:**
- `actions/upload-artifact@v3`: не использовать — merge-multiple: true только в v4.
- `tauri-apps/tauri-action@v0` с floating `tagName` без `releaseId`: допустимо но создаёт race condition риск.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | PowerShell `Compress-Archive` создаёт валидный ZIP из stage-директории на windows-latest runner | Pattern 2: Portable ZIP | Portable ZIP будет пустым или повреждённым; обнаруживается при тестировании pipeline |
| A2 | `gh release upload <tag_name> <file>` работает для загрузки в существующий draft release, созданный через github-script | Pattern 1, Pattern 2 | Загрузка portable ZIP упадёт с ошибкой; альтернатива — использовать release_id вместо tag |
| A3 | tauri-action artifacts в `target/release/bundle/**` доступны в рабочей директории после завершения action для upload-artifact шага | Pitfall 8, Pattern 4 | Нужно уточнить точные glob patterns для каждой ОС |
| A4 | `sed` с паттерном `0,...` корректно заменяет только первое вхождение `version = "..."` в Cargo.toml без затрагивания dep versions | Pattern 3: Version Injection | При неверном sed версии зависимостей могут быть повреждены; fallback — использовать python/node скрипт для точечной замены |
| A5 | macOS runner `macos-latest` в 2026 — это Apple Silicon (aarch64) runner, нативно собирающий aarch64-apple-darwin без --target | Code Examples | Если runner x86_64, нужно явно указать `--target aarch64-apple-darwin` через matrix args |

---

## Open Questions

1. **Точные glob patterns для upload-artifact после tauri-action**
   - What we know: NSIS в `target/release/bundle/nsis/*.exe`, AppImage в `target/release/bundle/appimage/*.AppImage`, deb в `target/release/bundle/deb/*.deb`, dmg в `target/release/bundle/dmg/*.dmg`
   - What's unclear: Изменяется ли путь при явном `--bundles` vs `"targets": "all"` в tauri.conf.json
   - Recommendation: Добавить `ls target/release/bundle/ -R` шаг после tauri-action в первом тестовом прогоне для верификации

2. **`gh release upload` принимает ли числовой release_id или только tag?**
   - What we know: `gh release upload` принимает tag по документации; release_id — числовой ID из API
   - What's unclear: Можно ли передать release_id напрямую или нужен tag
   - Recommendation: Использовать tag (`GITHUB_REF_NAME`) для gh release upload; release_id передавать только в tauri-action через параметр `releaseId`

3. **macOS macos-latest — ARM или x86 runner в 2026?**
   - What we know: GitHub в 2024-2025 начал переход macos-latest → macos-14 (M1)
   - What's unclear: Текущий дефолт macos-latest для публичных репо
   - Recommendation: Явно указать `macos-14` или `macos-latest` + matrix args `--target aarch64-apple-darwin`

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `jq` | Version injection step (Linux/macOS) | ✓ (ubuntu-latest, macos-latest) | 1.7+ | PowerShell ConvertFrom-Json на Windows |
| `gh` (GitHub CLI) | Portable ZIP upload, SHA256SUMS upload | ✓ (все standard runners) | — | actions/upload-release-asset |
| `sha256sum` | Checksums generation | ✓ (ubuntu-latest) | — | `shasum -a 256` на macOS |
| `pnpm` | Frontend build (tauri-action вызывает beforeBuildCommand) | Нужно установить (step) | 10 | — |
| Tauri Linux system deps | tauri build на ubuntu-22.04 | Нет (нужна установка) | — | apt-get (из ci-full.yml) |
| Windows WebView2 (системный Evergreen) | Runtime для NSIS installer | ✓ (Win10 x64 в домене пользователя) | Evergreen | Нет fallback — D-07 |

**Missing dependencies with no fallback:**
- Системный WebView2 Evergreen на машине пользователя (Win10 x64 — практически всегда есть; D-07 это допускает)

**Missing dependencies with fallback:**
- jq на Windows runner → PowerShell JSON manipulation

---

## Validation Architecture

> nyquist_validation: true (из .planning/config.json)

### Test Framework

| Property | Value |
|----------|-------|
| Framework | GitHub Actions workflow validation (нет unit test framework для CI YAML) |
| Config file | `.github/workflows/release.yml` |
| Quick run command | `gh workflow run release.yml --ref main` (потребует workflow_dispatch trigger) |
| Full suite command | `git tag v0.1.0-test && git push origin v0.1.0-test` (одноразово на test branch) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BLD-02 | Release workflow запускается по тегу v*.*.* | smoke | `git tag v0.0.1-rc1 && git push --tags` (test branch) | ❌ Wave 0 |
| BLD-02 | Windows NSIS installer присутствует в Release artifacts | manual | Проверить Release в GitHub UI | ❌ Wave 0 |
| BLD-02 | macOS .dmg присутствует в Release artifacts | manual | Проверить Release в GitHub UI | ❌ Wave 0 |
| BLD-02 | Linux .AppImage и .deb присутствуют | manual | Проверить Release в GitHub UI | ❌ Wave 0 |
| BLD-03 | SHA256SUMS файл присутствует в Release | manual | `gh release view --json assets` + jq | ❌ Wave 0 |
| BLD-03 | SHA256 контрольная сумма NSIS installer верна | manual | `sha256sum -c SHA256SUMS` локально | ❌ Wave 0 |
| BLD-04 | Portable ZIP содержит trackly.exe + portable.txt | manual | Распаковать ZIP, проверить наличие файлов | ❌ Wave 0 |
| BLD-04 | Запуск trackly.exe из portable ZIP → данные рядом с .exe | manual | Запустить на Win10, убедиться нет записей в %APPDATA% | ❌ Wave 0 |
| BLD-05 | README.md присутствует в корне репо | smoke | `ls README.md` | ❌ Wave 0 |

### Dry-Run Стратегии (как тестировать без реального тега на main)

1. **workflow_dispatch trigger (рекомендуется для Wave 0):** Добавить `workflow_dispatch` trigger параллельно с `push tags v*.*.*`. Позволяет `gh workflow run release.yml` без пуша тега. Для версии: передавать как `inputs.version`.

2. **Throwaway tag на feature branch:** `git tag v0.0.99-test && git push origin v0.0.99-test`. После проверки удалить тег и draft release. Не загрязняет main history.

3. **act локально:** `act push -e tests/fixtures/tag_push_event.json` — локальный runner GitHub Actions. Ограничение: не может реально создать GitHub Release без токена.

4. **Проверка артефактов без release:** `actions/upload-artifact@v4` в release.yml с `uploadWorkflowArtifacts: true` в tauri-action — артефакты видны в Actions run без Release. Использовать для быстрой проверки что bundler работает.

### Sampling Rate

- **Per task commit:** CI: `gh workflow run release.yml` с workflow_dispatch (если добавлен)
- **Per wave merge:** Push throwaway tag на main (v0.0.x-rc1, удалить после)
- **Phase gate:** Успешный run с тегом v0.1.0 на main, проверка всех артефактов в draft Release

### Wave 0 Gaps

- [ ] `.github/workflows/release.yml` — основной файл (весь BLD-02..05)
- [ ] `README.md` (корневой) — BLD-05
- [ ] `README-portable.md` — для portable ZIP
- [ ] `trackly.config.toml.example` — шаблон конфига для portable ZIP
- [ ] `crates/trackly-app/tauri.conf.json` bundle.active: true, bundle.icon расширен

*(Существующая test infrastructure покрывает unit/integration; release pipeline — это новый слой без существующего покрытия)*

---

## Security Domain

> security_enforcement: true (из .planning/config.json)

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — (CI использует GITHUB_TOKEN, не user auth) |
| V3 Session Management | no | — |
| V4 Access Control | yes (partially) | `permissions: contents: write` — минимально необходимые права; не использовать `permissions: write-all` |
| V5 Input Validation | yes | Тег `v*.*.*` — валидируется через on.push.tags pattern; VERSION экстрактируется через `${GITHUB_REF_NAME#v}` без eval |
| V6 Cryptography | yes | SHA256SUMS — криптографические хэши для integrity verification |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Poisoned supply chain (actions) | Tampering | Пинить actions по hash (`@v4` — floating, допустимо для trusted orgs; для paranoid mode — `@sha256:...`) |
| Secrets в логах CI | Information Disclosure | GITHUB_TOKEN автоматически маскируется; APPLE_SIGNING_IDENTITY='-' — не секрет |
| Arbitary code execution через tag message | Tampering | Версия извлекается только из GITHUB_REF_NAME (tag name), не из tag message/annotation |
| Подменный артефакт в Release | Repudiation | SHA256SUMS закрывает integrity; пользователь должен проверить хэш перед запуском |
| `bundle.active: true` + updater случайно включён | Elevation of Privilege | `uploadUpdaterJson: false` явно; tauri-plugin-updater не подключён в Cargo.toml |

---

## Project Constraints (from CLAUDE.md)

- **Portable дисциплина:** `tauri-plugin-updater` НЕ включать. Paths через `current_exe()`. D-09 это подтверждает.
- **No native-tls/OpenSSL:** Не релевантно для release pipeline (нет runtime зависимостей), но portable ZIP должен содержать бинарь без OpenSSL DLL.
- **WebView2 Evergreen:** `webviewInstallMode` НЕ настраивать на embedBootstrapper/fixed-version (D-07). По умолчанию Tauri 2 использует Evergreen.
- **Windows 7/32-bit из CLAUDE.md:** Игнорируем — D-01 явно исключает. Не добавлять `i686-pc-windows-msvc`.
- **MSRV 1.92** (из STATE.md — krilla bump): Toolchain pinned 1.88 в ci-full.yml. Нужно обновить до 1.92 или убедиться что release.yml использует `toolchain: '1.92'`. [ASSUMED — проверить актуальный MSRV в Cargo.toml]

---

## Sources

### Primary (HIGH confidence)
- [tauri-apps/tauri-action README.md](https://github.com/tauri-apps/tauri-action/blob/dev/README.md) — полный список inputs, releaseId/tagName interaction
- [Tauri v2 GitHub Actions guide](https://v2.tauri.app/distribute/pipelines/github/) — официальный рекомендованный workflow
- [Tauri Bundle Config reference](https://v2.tauri.app/reference/config/#bundleconfig) — bundle.targets, bundle.icon, bundle.macOS.signingIdentity
- [Tauri Windows Installer docs](https://v2.tauri.app/distribute/windows-installer/) — NSIS output location `target/release/bundle/nsis/`
- [Tauri Icons docs](https://v2.tauri.app/develop/icons/) — `pnpm tauri icon`, generated files list
- [Tauri macOS signing docs](https://v2.tauri.app/distribute/sign/macos/) — APPLE_SIGNING_IDENTITY env var, ad-hoc `-`
- [actions/upload-artifact@v4](https://github.com/actions/upload-artifact) — merge-multiple: true, unique names в matrix

### Secondary (MEDIUM confidence)
- [Ship Your Tauri v2 App Like a Pro (dev.to)](https://dev.to/tomtomdu73/ship-your-tauri-v2-app-like-a-pro-github-actions-and-release-automation-part-22-2ef7) — create-release job pattern, releaseId flow
- [tauri-action publish-to-manual-release.yml example](https://github.com/tauri-apps/tauri-action/blob/dev/examples/publish-to-manual-release.yml) — три-джоб паттерн с create-release → build → publish

### Tertiary (LOW confidence / flagged)
- [tauri-action issue #914](https://github.com/tauri-apps/tauri-action/issues/914) — race condition duplicate releases
- [tauri issue #13804](https://github.com/tauri-apps/tauri/issues/13804) — macOS ad-hoc signing intermittent DMG failure
- [tauri-action issue #302](https://github.com/tauri-apps/tauri-action/issues/302) — portable builds request (не реализовано в action)

---

## Metadata

**Confidence breakdown:**
- tauri-action inputs/API: HIGH — verified via official README
- Three-job workflow pattern: HIGH — from official example + dev.to article
- Version injection (sed+jq): MEDIUM — standard bash, but sed regex for Cargo.toml needs testing
- Portable ZIP assembly: MEDIUM — community pattern, not officially documented
- SHA256SUMS aggregation: MEDIUM — upload-artifact@v4 docs verified, specific globs [ASSUMED]
- macOS ad-hoc reliability: LOW — known intermittent issue

**Research date:** 2026-06-19
**Valid until:** ~2026-09-19 (tauri-action minor updates frequent; check @v0 changelog before implementation)
