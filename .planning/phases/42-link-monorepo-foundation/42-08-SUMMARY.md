---
phase: 42-link-monorepo-foundation
plan: 08
subsystem: monorepo
tags: [pnpm, workspace, ci, LNK-01]
requires: [42-05, 42-06]
provides:
  - корневой pnpm-workspace (ui, link-ui, packages/*) и единый pnpm-lock.yaml
  - каркасы @trackly/design, kit, transport, attachments и link-ui
  - CI на корневом lockfile, placeholder link-ui/dist
key-files:
  created: [pnpm-workspace.yaml, package.json, packages/*/package.json, link-ui/package.json]
  modified: [ui/package.json, .gitignore, .github/workflows/ci-fast.yml, ci-full.yml, release.yml, heic-spike.yml]
metrics:
  tasks: 2
  completed: 2026-11-26
---

# Phase 42 Plan 08: pnpm-монорепо Summary

Репозиторий стал pnpm-монорепо с единым корневым lockfile; фронтенд Trackly остаётся в `ui/`. Коммиты: 24c3a030, d078b1f4.

## Что сделано

- Корневые `pnpm-workspace.yaml` и `package.json` (pnpm 10.17.1); каркасы четырёх пакетов с заранее объявленными зависимостями (по фактическим импортам: `plugin-dialog/fs/shell` у kit, `@tauri-apps/api` у transport/attachments, `workspace:*` между пакетами) и `link-ui` без `prebuild`, `pagedjs`, `pdfjs-dist`.
- `ui/package.json`: четыре зависимости `workspace:*`. `ui/pnpm-lock.yaml` удалён (переименован в корневой).
- Все четыре workflow: `cache-dependency-path: pnpm-lock.yaml`, `pnpm install` из корня; в ci-fast и ci-full (оба джоба) добавлен шаг-заглушка `link-ui/dist/index.html`. В release.yml изменены только cache-путь и место установки (триггер и инъекция версии не тронуты).
- `.gitignore`: `link-ui/dist/`, `link-ui/node_modules/`, `packages/*/node_modules/`, `crates/link-app/gen/`.

## Проверка

`pnpm install --frozen-lockfile` OK; `pnpm --dir ui lint` OK; `svelte-check` 0 ошибок/0 предупреждений; `vite build` OK; `ui/node_modules/@tauri-apps/cli/tauri.js` на месте; check-privacy PASS. YAML-парсера локально нет — workflow сверены по `git diff`.

## Отклонения

**[Rule 1 - Bug] Дрейф резолюций при чистом `pnpm install`.** Свежая генерация lockfile подняла prettier 3.8.3 -> 3.10.1, из-за чего `prettier --check` покраснел на 11 файлах. Исправлено: старый `ui/pnpm-lock.yaml` взят за основу корневого (ключ importer `.` -> `ui`), `pnpm install --lockfile-only` лишь достроил новые importers. Резолюции ui сохранены (prettier 3.8.3). Версии зависимостей ui не менялись.

## Известное

Блок `exports` пакетов пуст (заполнит спайк плана 09). Новых внешних npm-пакетов нет.

## Self-Check: PASSED
