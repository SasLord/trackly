---
phase: 08-windows-macos-linux
plan: "01"
subsystem: release-config
tags: [tauri, bundle, portable, readme, documentation]
dependency_graph:
  requires: []
  provides:
    - tauri-bundle-active
    - portable-zip-staging-files
    - root-readme-ru
  affects:
    - crates/trackly-app/tauri.conf.json
    - README.md
    - README-portable.md
    - trackly.config.toml.example
tech_stack:
  added: []
  patterns:
    - tauri bundle configuration (active, icon list, macOS signingIdentity)
    - portable ZIP staging pattern (portable.txt marker + README-portable.md + .example config)
key_files:
  modified:
    - crates/trackly-app/tauri.conf.json
  created:
    - README.md
    - README-portable.md
    - trackly.config.toml.example
decisions:
  - "bundle.active: true — bundler включён для всех ОС (D-14)"
  - "bundle.icon: 5 форматов (32x32.png, 128x128.png, 128x128@2x.png, icon.icns, icon.ico) — Pitfall 3 устранён"
  - "bundle.macOS.signingIdentity: \"-\" — ad-hoc подпись без Apple Developer ID (D-04)"
  - "portable.txt маркер документирован в README-portable.md; логика в trackly_infra::paths (Phase 1)"
  - "trackly.config.toml.example содержит только закомментированные поля — нет секретов (T-08-01-02 accepted)"
metrics:
  duration: "2 min"
  completed_date: "2026-06-19"
  tasks: 3
  files: 4
---

# Phase 08 Plan 01: Bundle Config и Portable Staging — Summary

**One-liner:** Включён Tauri bundler (active:true, 5 форматов иконок, ad-hoc macOS signingIdentity), созданы portable ZIP staging-файлы и корневой README.md на русском.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Включить Tauri bundler и расширить bundle.icon | efde862 | crates/trackly-app/tauri.conf.json |
| 2 | Создать portable ZIP staging файлы | 9fd890b | README-portable.md, trackly.config.toml.example |
| 3 | Создать корневой README.md на русском (BLD-05) | e0da321 | README.md |

## Verification Results

| Check | Result |
|-------|--------|
| `bundle.active` = true | PASS |
| `bundle.icon` содержит 5 элементов | PASS |
| `bundle.macOS.signingIdentity` = "-" | PASS |
| README.md, README-portable.md, trackly.config.toml.example существуют | PASS |
| README.md: SmartScreen, Gatekeeper, WebView2, portable, серверный (>= 4 строк) | PASS (12 строк) |
| README.md >= 80 строк | PASS (161 строка) |
| trackly.config.toml.example содержит закомментированный `db_path` | PASS |
| README-portable.md упоминает `portable.txt` | PASS |

## Decisions Made

- **D-04 (ad-hoc подпись):** `signingIdentity: "-"` в bundle.macOS — подпись без Apple Developer ID.
- **D-08 (portable ZIP):** staging-файлы готовы; `portable.txt` добавляет CI-пайплайн Plan 02.
- **D-09 (без updater):** не добавлен updater в tauri.conf.json — соответствует portable-дисциплине.
- **D-14 (bundle.active):** `active: true` — bundler выдаёт NSIS/dmg/AppImage/deb при `tauri build`.
- **D-16 (README):** README.md на русском с инструкциями по каждой ОС, portable-режиму, серверному режиму и обходу SmartScreen/Gatekeeper.

## Deviations from Plan

Нет — план выполнен точно по спецификации.

## Known Stubs

Нет. Все файлы содержат реальные инструкции и конфигурацию.

## Threat Flags

Нет новой угрозы-поверхности вне плановой threat model (T-08-01-01..03 отражены в файлах).

## Self-Check

### Файлы существуют:
- crates/trackly-app/tauri.conf.json: FOUND
- README.md: FOUND
- README-portable.md: FOUND
- trackly.config.toml.example: FOUND

### Коммиты существуют:
- efde862: FOUND
- 9fd890b: FOUND
- e0da321: FOUND

## Self-Check: PASSED
