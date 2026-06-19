---
phase: 08-windows-macos-linux
reviewed: 2026-06-19T10:00:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - .github/workflows/release.yml
  - crates/trackly-app/tauri.conf.json
  - README.md
  - README-portable.md
  - trackly.config.toml.example
findings:
  critical: 2
  warning: 4
  info: 1
  total: 7
status: issues_found
---

# Phase 8: Code Review Report

**Reviewed:** 2026-06-19T10:00:00Z
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

Ревью охватывает пайплайн релиза (GitHub Actions), конфиг бандла Tauri, корневой README, README portable-режима и шаблон конфигурации. Основная масса ценности — в `release.yml`.

Структура пайплайна в целом корректна: трёхзадачная цепочка (create-release → build matrix → checksums) логична, `fail-fast: false` позволяет частичным платформам не блокировать остальные, `uploadUpdaterJson: false` и `draft: true` соответствуют portable-дисциплине. Инъекция версии через perl + jq работает правильно на практике при нормальных входных данных.

Обнаружены два BLOCKER-уровня: канонический GH Actions shell-injection через `${{ github.event.inputs.version }}` в `run:` блоках, и плавающий mutable-тег `dtolnay/rust-toolchain@stable` как supply-chain риск. Четыре WARNING: отключённый CSP в webview, все остальные actions без SHA-pin, непроверенный параметр `retryAttempts`, и потенциальная ловушка с `"targets": "all"` в tauri.conf.json.

---

## Critical Issues

### CR-01: Shell injection через `${{ github.event.inputs.version }}` в `run:` блоках

**File:** `.github/workflows/release.yml:132, 199, 256`

**Issue:** GitHub Actions вычисляет выражения `${{ ... }}` и подставляет результат в текст shell-скрипта **до** его запуска. Это означает, что если `inputs.version` содержит специальные символы оболочки (`"`, `` ` ``, `$(`, `\n` и др.), они выполнятся в контексте bash. Строки 199 и 256 вставляют значение напрямую в `TAG="v${{ github.event.inputs.version }}"` без какой-либо прослойки. Строка 132 также уязвима (`VERSION="${{ github.event.inputs.version }}"`), после чего VERSION попадает в команду `perl -0pi -e 's/.../${1}'"${VERSION}"'${2}/m'`: если VERSION содержит `/`, это разобьёт разделитель подстановки perl и вызовет ошибку или неожиданное поведение.

`workflow_dispatch` может запустить только пользователь с правом `write` на репозиторий, что снижает практический риск. Тем не менее это канонический anti-pattern, задокументированный в руководстве GitHub по безопасности Actions, и он ломается даже при случайно введённых спецсимволах.

**Fix:** Передавайте `inputs.version` через `env:` как переменную окружения, а в shell читайте из неё. Пример для всех трёх мест:

```yaml
# В каждом затронутом шаге добавить секцию env:
env:
  INPUT_VERSION: ${{ github.event.inputs.version }}

# В теле run: заменить ${{ github.event.inputs.version }} на $INPUT_VERSION
- name: Extract version and inject into Cargo.toml + tauri.conf.json
  shell: bash
  env:
    INPUT_VERSION: ${{ github.event.inputs.version }}
  run: |
    if [[ "$GITHUB_EVENT_NAME" == "workflow_dispatch" ]]; then
      VERSION="${INPUT_VERSION}"
    else
      VERSION="${GITHUB_REF_NAME#v}"
    fi
    # ...остальное без изменений...

- name: Upload portable ZIP to release
  shell: bash
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
    INPUT_VERSION: ${{ github.event.inputs.version }}
  run: |
    if [[ "$GITHUB_EVENT_NAME" == "workflow_dispatch" ]]; then
      TAG="v${INPUT_VERSION}"
    else
      TAG="$GITHUB_REF_NAME"
    fi
    gh release upload "$TAG" "$PORTABLE_ZIP" --repo "$GITHUB_REPOSITORY"
```

---

### CR-02: `dtolnay/rust-toolchain@stable` — mutable action tag, supply-chain риск

**File:** `.github/workflows/release.yml:79`

**Issue:** Тег `@stable` в `dtolnay/rust-toolchain@stable` — это **мутируемая ссылка** на ветку/тег репозитория экшена. Если репозиторий экшена будет скомпрометирован или тег переписан, следующий релизный билд исполнит произвольный код в контексте релизного пайплайна с токеном `GITHUB_TOKEN` (`contents: write`). Это позволяет злоумышленнику загрузить модифицированные артефакты в черновой релиз.

Для сравнения: все `actions/*` (checkout, upload-artifact и т.д.) тоже не закреплены SHA, но они первой стороны и обновляются по контролируемым тегам. `dtolnay/rust-toolchain` — сторонний экшен, доверять его мутируемому тегу в релизном пайплайне рискованнее.

**Fix:** Закрепите все сторонние actions по commit SHA:

```yaml
# Найти текущий SHA:
# gh api repos/dtolnay/rust-toolchain/git/ref/tags/stable --jq '.object.sha'
# Затем:
uses: dtolnay/rust-toolchain@<sha>  # например: @a54c7afa936d91f02eedd3ef79fd22fa4c97c085

# Аналогично для остальных сторонних actions:
uses: Swatinem/rust-cache@<sha>
uses: pnpm/action-setup@<sha>
uses: tauri-apps/tauri-action@<sha>
```

---

## Warnings

### WR-01: `"csp": null` отключает Content Security Policy в Tauri webview

**File:** `crates/trackly-app/tauri.conf.json:21`

**Issue:** `csp: null` полностью отключает CSP для webview. В режиме десктоп-приложения это означает, что если пользовательские данные (например, названия устройств, имена сотрудников) рендерятся в DOM без должного экранирования на стороне Svelte-компонентов, XSS может вызвать Tauri `invoke()` с произвольными командами. Tauri 2 использует capability-модель как дополнительный барьер, но отсутствие CSP убирает первый эшелон защиты.

**Fix:** Установить минимальный restrictive CSP:

```json
"security": {
  "csp": "default-src 'self'; script-src 'self'; connect-src 'self' http://localhost:1420"
}
```

Скорректируйте `connect-src` под реальные источники данных (например, SNMP/LDAP идут через Tauri invoke, а не прямые fetch). Если какие-то стили или скрипты требуют `'unsafe-inline'`, добавьте их явно и задокументируйте почему.

---

### WR-02: Все GitHub Actions привязаны к мутируемым тегам (не только dtolnay)

**File:** `.github/workflows/release.yml:32, 36, 85, 102, 107, 159, 210, 231, 234`

**Issue:** Ни один из используемых actions не закреплён по commit SHA. Тег `@v4`, `@v7`, `@v0` и т.д. — мутируемые ссылки. Особенно опасен `tauri-apps/tauri-action@v0` в release pipeline, так как именно он загружает финальные артефакты в GitHub Release и имеет доступ к `GITHUB_TOKEN`. Компрометация `tauri-apps/tauri-action` или любого из `actions/*` в момент релизного прогона позволяет подменить исполняемые файлы.

**Fix:** Закрепить все actions по SHA. Для `actions/*` GitHub публикует SHA в changelog. Используйте инструменты типа `pinact` или `renovate` с `pinDigests: true` для автоматического управления.

```yaml
# Пример (актуальные SHA нужно проверить):
uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af68  # v4.2.2
uses: actions/github-script@60a0d83039c74a4aee543508d2ffcb1c3799cdea  # v7
uses: actions/upload-artifact@65c4c4a1ddee5b72f698fdd19549f0f0fb30fc4  # v4
uses: actions/download-artifact@95815c38cf2ff2b2b7e07beacba8a1a2a1fc48e  # v4
uses: actions/setup-node@cdca7365b2dadb8aad0a33bc7601856ffabcc48e  # v4
```

---

### WR-03: Параметр `retryAttempts` в `tauri-apps/tauri-action@v0` не задокументирован

**File:** `.github/workflows/release.yml:167`

**Issue:** Параметр `retryAttempts: 1` передан в `tauri-apps/tauri-action@v0`, однако этого параметра нет в официальной документации `tauri-action`. GitHub Actions молча игнорирует неизвестные поля в `with:`, поэтому если параметр не поддерживается, retry-логика просто не работает, а CI не выдаст ошибки. Комментарий в workflow ссылается на «Pitfall 4» (intermittent macOS DMG failure), но если `retryAttempts` игнорируется, эта ловушка не закрыта.

**Fix:** Проверить поддержку параметра:

```bash
# Просмотреть action.yml для нужного SHA tauri-action:
gh api repos/tauri-apps/tauri-action/contents/action.yml --ref <sha> | jq -r '.content' | base64 -d | grep -A5 retry
```

Если параметр не поддерживается, реализовать retry явно через `uses: nick-fields/retry@v3` или bash-цикл в шаге.

---

### WR-04: `"targets": "all"` в `tauri.conf.json` — ловушка при локальном `tauri build`

**File:** `crates/trackly-app/tauri.conf.json:26`

**Issue:** `"targets": "all"` означает, что при локальном `tauri build` (без `--bundles`) Tauri попытается собрать все форматы для текущей платформы: на Windows — NSIS + MSI (если WiX установлен); на macOS — .app + .dmg. Это неожиданное поведение для разработчика, который хочет просто получить бинарник для тестирования. CI переопределяет это через `--bundles nsis` / `--bundles dmg` / `--bundles appimage,deb`, поэтому релизы корректны. Но локальные сборки могут зависать или падать из-за недостающих зависимостей (WiX на Windows, create-dmg на macOS).

**Fix:** Указать явные targets или документировать ожидаемое поведение:

```json
"bundle": {
  "active": true,
  "targets": ["nsis"],
  "icon": [ ... ],
  "macOS": { "signingIdentity": "-" }
}
```

Или — если `"all"` задумано намеренно — добавить комментарий в CLAUDE.md или README для разработчиков, что локальная сборка всегда требует `tauri build --bundles <target>`.

---

## Info

### IN-01: README.md — инструкция проверки SHA256SUMS для Windows неполная

**File:** `README.md:150-155`

**Issue:** Инструкция для Windows/PowerShell показывает `Get-FileHash` для одного файла, но не объясняет как сравнить результат с `SHA256SUMS` (текстовым файлом). Пользователи Windows, скорее всего, не смогут воспроизвести проверку в формате `sha256sum -c SHA256SUMS`.

**Fix:** Добавить пример полного сравнения:

```powershell
# Скачайте SHA256SUMS и нужный артефакт в одну папку, затем:
$expected = (Get-Content SHA256SUMS | Select-String "trackly_N.N.N_x64-setup.exe").ToString().Split(" ")[0]
$actual = (Get-FileHash "trackly_N.N.N_x64-setup.exe" -Algorithm SHA256).Hash
if ($expected.ToUpper() -eq $actual) { "OK" } else { "MISMATCH" }
```

---

_Reviewed: 2026-06-19T10:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
