---
phase: 42-link-monorepo-foundation
plan: 16
subsystem: link-ui
tags: [svelte, vite, tauri, design-tokens, gate]
requires: [42-10, 42-14, 42-15]
provides:
  - link-ui собирается (vite build) на общих пакетах @trackly/design и @trackly/kit
  - каркас окна Link (логотип, вкладка «Чат», сайдбар, область содержимого, служебная строка)
  - гейт check-link-layout (+selftest) в link-ui lint
affects: [ci-fast (plan 18), 42-19]
tech-stack:
  added: []
  patterns: [source-package импорты @trackly/*, theme-init через vite-plugin, тема из kit]
key-files:
  created:
    - link-ui/index.html
    - link-ui/vite.config.ts
    - link-ui/tsconfig.json
    - link-ui/src/main.ts
    - link-ui/src/App.svelte
    - link-ui/src/components/LinkLogo.svelte
    - link-ui/src/components/StatusLine.svelte
    - link-ui/src/lib/linkInfo.ts
    - link-ui/scripts/check-link-layout.mjs
  modified:
    - link-ui/package.json
decisions:
  - "Гейт проверяет присутствие <LinkLogo / <StatusLine как тегов, а не имён: иначе импорт оставляет мутанта живым"
metrics:
  tasks: 3
  completed: 2026-11-26
---

# Phase 42 Plan 16: UI-каркас Link Summary

link-ui собирается на общих пакетах (Button, Tabs, ThemeSwitcher из kit; токены и theme-init из design), показывает каркас окна LNK-10 и защищён структурным гейтом check-link-layout с мутационным selftest.

## Что сделано

- Конфигурация: Vite (порт 1421, плагин theme-init, без алиасов и prependData), tsconfig, index.html без inline-скриптов, main.ts (global.scss, initTheme до mount). Скрипты `svelte-check` и `lint` в package.json; `prebuild` нет.
- Каркас: сетка с верхней панелью (логотип «Link», вкладка «Чат», ThemeSwitcher), сайдбар, область содержимого с заглушкой «Здесь скоро будет чат» и служебной строкой (версия Link, WebView2, путь установки; вне Tauri «—»). Кнопка «Скопировать диагностику» — единственный Button.svelte в репозитории. Только токены `--tr-*`, без `{@html}`.
- Гейт check-link-layout: правила L0–L4, 10 мутантов, 33 признака на чистом дереве; ручная мутация (`<aside`) даёт красный.

## Проверки (все выполнены локально)

- `pnpm --dir link-ui build` — зелёный (создаёт dist/index.html и theme-init.js)
- `pnpm --dir link-ui svelte-check` — 0 ошибок, 0 предупреждений
- `pnpm --dir link-ui lint` — зелёный
- `pnpm --dir ui lint` (включая check-workspace-boundaries, link-ui/src просканирован) — exit 0
- `pnpm --dir ui svelte-check`, `pnpm --dir ui exec vite build` — зелёные
- `cargo clippy -p link-app --all-targets -- -D warnings` — зелёный
- prettier по link-ui — зелёный; eslint по link-ui идёт через обёртку `_eslint-workspaces` в `ui lint`
- Button.svelte в репозитории: 1 файл

## Deviations from Plan

**[Rule 1 - Bug]** LinkLogo.svelte без `<script>` распознавался svelte-check как `any`; добавлен пустой `<script lang="ts">`.
**[Rule 1 - Bug]** Первый вариант L1 проверял имя `StatusLine`, мутант с удалением тега выживал (импорт оставался); правило переведено на проверку тега.

## Для оркестратора: human-verify (не одобрено самостоятельно)

Визуальная проверка окна Link (логотип, вкладка «Чат», сайдбар, заглушка, служебная строка с реальными значениями, светлая/тёмная тема, минимум 720x500, запоминание размера) — UNVERIFIED, нужен запуск `link-app` (WebView2 на Windows / dev на macOS).

## Known Stubs

Сайдбар («Беседы появятся здесь») и область чата («Здесь скоро будет чат») — намеренные заглушки по D-16; чат — последующие фазы v1.5.

## Self-Check: PASSED

Файлы созданы, коммиты 85a8acbe, 8a24f421, 3fb024d3 существуют.
