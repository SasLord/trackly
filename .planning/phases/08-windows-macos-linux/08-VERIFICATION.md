---
phase: 08-windows-macos-linux
verified: 2026-06-19T00:00:00Z
status: human_needed
score: 5/5 must-haves verified
overrides_applied: 0
re_verification: false
human_verification:
  - test: "Запустить workflow_dispatch dry-run"
    expected: "gh workflow run release.yml --field version=0.1.0-test → create-release создаёт draft Release v0.1.0-test → три параллельных build job завершаются → checksums job заливает SHA256SUMS. Итоговый Release содержит: Trackly_*_x64-setup.exe (NSIS), *.dmg, *.AppImage, *.deb, trackly-v0.1.0-test-windows-x64-portable.zip, SHA256SUMS."
    why_human: "Требует живые GitHub Actions runners (три ОС, ~20–35 минут). На macOS без push нельзя проверить реальную сборку tauri-action и работоспособность NSIS/dmg/AppImage артефактов."
  - test: "Проверить portable ZIP по факту сборки на Windows runner"
    expected: "ZIP содержит trackly.exe + portable.txt + README.md (из README-portable.md) + trackly.config.toml.example — без updater.json; запуск trackly.exe из распакованной папки создаёт trackly.db рядом с .exe."
    why_human: "Требует Windows runner и реальный tauri build. PowerShell-шаги корректны статически, но исполнение верифицируемо только при реальном прогоне."
  - test: "Проверить SHA256SUMS по факту прогона"
    expected: "Файл SHA256SUMS содержит строки для всех артефактов всех трёх ОС; sha256sum -c проходит для каждого артефакта."
    why_human: "SHA256SUMS генерируется в job checksums из реальных build-артефактов; без живого run не верифицируемо."
  - test: "Проверить push тега v*.*.* (полный путь)"
    expected: "git tag v0.0.99-test && git push origin v0.0.99-test → pipeline запускается по tag-trigger (не только по workflow_dispatch), создаёт draft Release с корректным именем 'Trackly 0.0.99-test'."
    why_human: "Tag-trigger (on.push.tags) отличается от workflow_dispatch; нужно подтверждение что оба пути создают Release с корректным тегом."
---

# Phase 8: Релизный пайплайн (Windows/macOS/Linux) — Verification Report

**Phase Goal:** При push git-тега `v*.*.*` GitHub Actions Release собирает Windows 64-bit (NSIS installer + portable ZIP с `portable.txt` и без updater), macOS aarch64 (.dmg), Linux x86_64 (.AppImage + .deb); артефакты содержат SHA256-checksums; релиз создаётся как draft. Плюс корневой README.md (RU) с инструкциями по запуску для каждой ОС, portable-режим, требование WebView2 на Windows, серверный режим + доверие self-signed сертификату, обход SmartScreen/Gatekeeper.

**Verified:** 2026-06-19
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Push тега `v*.*.*` запускает release.yml и создаёт draft GitHub Release | VERIFIED | `on.push.tags: ['v*.*.*']` + job create-release с `draft: true` в release.yml строки 9–11, 48–51 |
| 2 | Три ОС собирают артефакты: NSIS (Windows), .dmg (macOS aarch64), .AppImage + .deb (Linux) | VERIFIED | matrix.include: windows-latest `--bundles nsis`, macos-latest `--target aarch64-apple-darwin --bundles dmg`, ubuntu-22.04 `--bundles appimage,deb` — строки 61–67 |
| 3 | Portable ZIP содержит trackly.exe + portable.txt + README.md + trackly.config.toml.example, без updater | VERIFIED | `uploadUpdaterJson: false` + Assemble portable ZIP step (строки 179–191): все четыре файла, `New-Item portable.txt`, `Compress-Archive` |
| 4 | SHA256SUMS агрегирует хэши всех артефактов всех ОС в одном файле и загружается в Release | VERIFIED | job checksums: `download-artifact merge-multiple: true`, `sha256sum * > SHA256SUMS`, `gh release upload "$TAG" ./artifacts/SHA256SUMS` — строки 230–267 |
| 5 | Версия артефактов берётся из тега (v1.2.3 → 1.2.3), не из Cargo.toml заглушки | VERIFIED | `perl -0pi -e 's/^(version = ").../${1}'"${VERSION}"'${2}/m' Cargo.toml` + `jq --arg v "$VERSION" '.version = $v' tauri.conf.json` — строки 142–154 |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `.github/workflows/release.yml` | Three-job CI pipeline: create-release → build → checksums | VERIFIED | 268 строк, YAML-валиден (python yaml.safe_load PASS), три job, matrix из трёх ОС |
| `crates/trackly-app/tauri.conf.json` | `bundle.active: true`, 5 иконок, `macOS.signingIdentity: "-"` | VERIFIED | jq: `true`, `5`, `"-"` — все три поля в норме |
| `README.md` | Корневой README на русском ≥80 строк (BLD-05) | VERIFIED | 161 строка, grep даёт 13 строк с SmartScreen/Gatekeeper/WebView2/portable/серверный |
| `README-portable.md` | Краткая инструкция для portable ZIP, упоминает portable.txt | VERIFIED | Файл существует, содержит "portable.txt" в двух местах |
| `trackly.config.toml.example` | Закомментированный шаблон конфигурации, содержит `db_path` | VERIFIED | Файл существует, строка `# db_path = "D:\\trackly-data\\trackly.db"` присутствует |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| job create-release | job build | `needs.create-release.outputs.release_id` → `releaseId` в tauri-action | VERIFIED | `outputs: release_id: ${{ steps.create-release.outputs.result }}` + `releaseId: ${{ needs.create-release.outputs.release_id }}` |
| job build (windows) | README-portable.md + trackly.config.toml.example | `Copy-Item` в portable-stage | VERIFIED | строки 187–188: `Copy-Item "README-portable.md"`, `Copy-Item "trackly.config.toml.example"` |
| job build | job checksums | `upload-artifact release-artifacts-$platform` → `download-artifact merge-multiple` | VERIFIED | upload (строки 214–225) + download в checksums (строки 238–243): pattern `release-artifacts-*`, `merge-multiple: true` |
| `.github/workflows/release.yml` | `crates/trackly-app/icons/` | bundle.icon array в tauri.conf.json | VERIFIED | 5 иконок в json, все 5 файлов существуют в `crates/trackly-app/icons/` |

### Data-Flow Trace (Level 4)

Не применимо для данной фазы — нет компонентов, рендерящих динамические данные из БД. Все артефакты представляют собой CI-конфигурацию и документацию.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| YAML-синтаксис release.yml | `python3 -c "import yaml; yaml.safe_load(open('release.yml'))"` | YAML_VALID | PASS |
| tauri.conf.json: bundle.active=true, 5 иконок, signingIdentity="-" | `jq '.bundle.active, (.bundle.icon | length), .bundle.macOS.signingIdentity'` | `true`, `5`, `"-"` | PASS |
| release.yml: 3 job, matrix, checksums needs | python3 YAML parse | Jobs: create-release, build, checksums; matrix 3 platforms; needs: [create-release, build] | PASS |
| Запрещённые конструкции отсутствуют (i686, embedBootstrapper, webviewInstallMode) | grep + wc -l | 0 совпадений | PASS |
| README.md покрывает все темы | grep -c "SmartScreen\|Gatekeeper\|WebView2\|portable\|серверный" | 13 строк | PASS |
| Commit-хэши из SUMMARY реально существуют | git log --oneline | efde862, 9fd890b, e0da321, a1f5bb3 — все найдены | PASS |

### Probe Execution

В PLAN.md (08-02, Task 3) упоминается dry-run через `gh workflow run release.yml --field version=0.1.0-test`. Этот probe требует живых GitHub Actions runners и не может быть выполнен из macOS dev-среды. Вынесен в Human Verification.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| BLD-02 | 08-02 | GitHub Actions Release: push v*.*.* собирает Windows NSIS + portable ZIP, macOS dmg, Linux AppImage+deb | SATISFIED | release.yml: trigger `on.push.tags: ['v*.*.*']`, tauri-action matrix, per-OS bundle args |
| BLD-03 | 08-02 | Артефакты включают SHA256 checksums | SATISFIED | job checksums: `sha256sum * > SHA256SUMS`, upload в Release |
| BLD-04 | 08-01, 08-02 | Portable вариант без updater, с маркером portable.txt | SATISFIED | `uploadUpdaterJson: false`, portable.txt в Assemble-шаге, portable ZIP staging-файлы |
| BLD-05 | 08-01 | README на русском с инструкциями для каждой ОС | SATISFIED | README.md 161 строка, все требуемые разделы присутствуют |

**Замечание по orphaned requirements:** REQUIREMENTS.md в таблице трассируемости приписывает к Phase 8 следующие требования: USR-08..12, REQ-06, SET-10. Эти требования помечены как "Pending" и связаны с AD-логином. Ни один из двух PLAN.md Phase 8 их не объявляет в поле `requirements:`. Согласно memory-файлу проекта, AD-требования перенесены в Phase 9 (release first, AD testable on Windows). Phase 9 не описана в ROADMAP.md. Данные items не являются gaps для данной фазы, поскольку они намеренно отложены, однако в ROADMAP они не задокументированы как Phase 9.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | — | — | Нет TBD/FIXME/XXX/TODO/placeholder в модифицированных файлах |

Сканирование на debt-markers завершилось с нулевым результатом по всем пяти модифицированным файлам.

---

## Human Verification Required

### 1. Workflow_dispatch dry-run

**Test:** `gh workflow run release.yml --field version=0.1.0-test` (или через GitHub UI: Actions → release → Run workflow → version: 0.1.0-test)

**Expected:** Все три build job (windows-latest, macos-latest, ubuntu-22.04) завершаются успешно. Job checksums заливает SHA256SUMS. Draft Release v0.1.0-test содержит: `Trackly_*_x64-setup.exe` (NSIS), `*.dmg`, `*.AppImage`, `*.deb`, `trackly-v0.1.0-test-windows-x64-portable.zip`, `SHA256SUMS`. После верификации: `gh release delete v0.1.0-test --yes`.

**Why human:** Требует живые GitHub Actions runners на трёх ОС (~20–35 минут). macOS/Linux сборка tauri-action проверяема только в реальном прогоне.

### 2. Проверка содержимого portable ZIP

**Test:** Из Windows runner (или после скачивания артефакта) распаковать `trackly-v0.1.0-test-windows-x64-portable.zip` и проверить содержимое.

**Expected:** Архив содержит ровно 4 файла: `trackly.exe`, `portable.txt` (пустой), `README.md` (содержимое из README-portable.md), `trackly.config.toml.example`. Запуск `trackly.exe` создаёт `trackly.db` рядом с .exe, не в %APPDATA%.

**Why human:** Содержимое ZIP и portable-поведение проверяемы только при реальном запуске на Windows.

### 3. SHA256SUMS верификация

**Test:** После прогона скачать `SHA256SUMS` и все артефакты Release, выполнить `sha256sum -c SHA256SUMS` (Linux/macOS) или ручную проверку через `Get-FileHash` (Windows).

**Expected:** Все хэши совпадают. SHA256SUMS содержит строки для всех 6+ артефактов (NSIS, portable ZIP, dmg, AppImage, deb).

**Why human:** SHA256SUMS формируется из реальных build-артефактов — без живого run результат непроверяем.

### 4. Tag-push путь (опционально)

**Test:** `git tag v0.0.99-test && git push origin v0.0.99-test`

**Expected:** Pipeline запускается по tag-trigger (не только по workflow_dispatch). Create-release использует `context.ref.replace('refs/tags/', '')` для извлечения тега. Draft Release называется "Trackly 0.0.99-test". Cleanup: `gh release delete v0.0.99-test --yes && git tag -d v0.0.99-test && git push origin --delete v0.0.99-test`.

**Why human:** on.push.tags и workflow_dispatch — это два разных пути выполнения; оба задокументированы в workflow с условной логикой (`context.eventName`, `GITHUB_EVENT_NAME`). Статический анализ подтверждает корректность условий, но реальный tag-push требует CI-среды.

---

## Gaps Summary

Gaps не обнаружены. Все статически верифицируемые артефакты и связи присутствуют и содержательны. Единственные неверифицированные элементы — это реальный прогон GitHub Actions runners, который по природе проверяется только в CI-среде.

**Замечание: orphaned requirements в REQUIREMENTS.md.** Семь требований (USR-08..12, REQ-06, SET-10) числятся в трассируемости Phase 8 как "Pending" и фактически не входят в scope ни одного Plan Phase 8. Они были перенесены в будущую Phase 9 (AD-логин), которая пока не описана в ROADMAP.md. Это создаёт несоответствие между REQUIREMENTS.md traceability и фактическим ROADMAP. Рекомендуется либо добавить Phase 9 в ROADMAP.md, либо обновить трассируемость с явным указанием "Phase 9 (planned)".

---

_Verified: 2026-06-19_
_Verifier: Claude (gsd-verifier)_
