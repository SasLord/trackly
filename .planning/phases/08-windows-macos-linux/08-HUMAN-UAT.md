---
status: partial
phase: 08-windows-macos-linux
source: [08-VERIFICATION.md]
started: 2026-06-19
updated: 2026-06-19
---

## Current Test

[awaiting human testing — requires a CI-capable / Windows setup; cannot run from macOS dev box]

## Tests

### 1. workflow_dispatch dry-run собирает артефакты во все три ОС
expected: `gh workflow run release.yml --field version=0.1.0-test` → три build-job (windows/macos/ubuntu) зелёные; в draft Release появляются NSIS .exe + portable .zip (Windows), .dmg (macOS aarch64), .AppImage + .deb (Linux).
result: [pending]

### 2. Portable ZIP — содержимое и portable-режим на Win10
expected: распаковать `trackly-v*-windows-x64-portable.zip` → внутри `trackly.exe` + `portable.txt` + `README.md` + `trackly.config.toml.example`, без updater; запуск `trackly.exe` создаёт данные рядом с .exe, НЕ в %APPDATA% (проверить через `tools/procmon-check`).
result: [pending]

### 3. SHA256SUMS верификация
expected: `SHA256SUMS` присутствует в Release; `sha256sum -c SHA256SUMS` (или `certutil -hashfile` на Windows) совпадает со скачанными артефактами.
result: [pending]

### 4. Tag-push путь (триггер v*.*.*)
expected: `git tag v0.0.99-test && git push origin v0.0.99-test` → release.yml запускается по тегу, создаётся draft Release; затем удалить тестовый тег и draft.
result: [pending]

### 5. Установка/запуск на каждой ОС (опционально, при наличии машин)
expected: Windows NSIS installer ставится и запускается (обход SmartScreen по README); macOS .dmg монтируется и приложение открывается (обход Gatekeeper по README); Linux .AppImage/.deb запускается.
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0
blocked: 0

## Gaps
