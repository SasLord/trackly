---
status: passed
phase: 08-windows-macos-linux
source: [08-VERIFICATION.md]
started: 2026-06-19
updated: 2026-06-19
---

## Current Test

[complete — release pipeline validated via workflow_dispatch dry-run (version=0.1.0-test) on real CI; all three OS builds + checksums green; portable build runs cleanly on Windows]

## Tests

### 1. workflow_dispatch dry-run собирает артефакты во все три ОС
expected: `gh workflow run release.yml --field version=0.1.0-test` → три build-job (windows/macos/ubuntu) зелёные; в draft Release появляются NSIS .exe + portable .zip (Windows), .dmg (macOS aarch64), .AppImage + .deb (Linux).
result: passed — все 3 сборки зелёные, артефакты в draft Release (подтверждено пользователем 2026-06-19, после фиксов: version-grep, rust-embed/ui-dist cycle, idempotent create-release).

### 2. Portable ZIP — содержимое и portable-режим на Win10
expected: распаковать `trackly-v*-windows-x64-portable.zip` → внутри `trackly.exe` + `portable.txt` + `README.md` + `trackly.config.toml.example`, без updater; запуск `trackly.exe` создаёт данные рядом с .exe, НЕ в %APPDATA%.
result: passed — portable ZIP запускается на Win10; после фикса `windows_subsystem = "windows"` лишнего консольного окна нет. Поведение «данные рядом с .exe, не в %APPDATA%» закрыто Phase 1 procmon-check (CI).

### 3. SHA256SUMS верификация
expected: `SHA256SUMS` присутствует в Release; `sha256sum -c SHA256SUMS` совпадает со скачанными артефактами.
result: passed — checksums-job зелёный, `SHA256SUMS` залит в Release (после фикса рекурсивного сбора + basenames; round-trip `sha256sum -c` проверен локально).

### 4. Tag-push путь (триггер v*.*.*)
expected: `git tag v0.0.99-test && git push origin v0.0.99-test` → release.yml запускается по тегу.
result: deferred — отдельно не прогонялся. Использует тот же workflow, что и dry-run; различается только деривация тега, и обе ветки (workflow_dispatch / push-tag) покрыты в коде. Будет реально проверено при первом продакшн-релизе.

### 5. Установка/запуск на каждой ОС (опционально)
expected: Windows NSIS installer ставится; macOS .dmg монтируется (обход Gatekeeper); Linux .AppImage/.deb запускается.
result: deferred — опционально, по мере появления целевых машин. Portable .exe на Win10 запускается; NSIS/.dmg/.AppImage/.deb собираются, но установка на каждой ОС отдельно не прогонялась.

## Summary

total: 5
passed: 3
issues: 0
pending: 0
skipped: 2
blocked: 0

## Gaps

None blocking. Релизный пайплайн рабочий. Пять дефектов, найденных ТОЛЬКО реальным прогоном (невидимы статическим проверкам), исправлены и в `main`:
1. version-injection: verify-grep падал на pre-release версии (`0.1.0-test`) под `bash -e` → проверка точной строки.
2. rust-embed `ui/dist` циклическая зависимость на чистом checkout → seed placeholder перед сборкой.
3. `sha256sum *` спотыкался о каталог `target` → рекурсия по файлам + basenames.
4. дубликаты draft-релизов + коллизия ассетов на повторном dry-run → идемпотентный create-release (удаление прежних draft) + `--clobber`.
5. лишнее консольное окно на Windows → `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`.

Остаточные не-блокеры (follow-up): обновление/пиннинг GitHub Actions (Node 20 deprecation, SHA-pin — chip task_0752872f); macOS bundle-identifier `org.trackly.app` оканчивается на `.app` (Warn от Tauri).
