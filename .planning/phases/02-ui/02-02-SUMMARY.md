---
phase: 02-ui
plan: "02"
subsystem: ui-shell
tags:
  - ui
  - shell
  - theme
  - sidebar
  - transport
  - routing
dependency_graph:
  requires:
    - 02-01 (backend scaffold — AppCtx, domain types)
  provides:
    - ui-shell (App.svelte + Layout + Sidebar + Router)
    - theme-store (initTheme/setTheme/themeStore)
    - toast-store (pushToast + TTL auto-remove)
    - api-client (apiCall with __TAURI_INTERNALS__ transport-detect)
    - 11 hand-rolled UI primitives
  affects:
    - ci-fast.yml + ci-full.yml (svelte-check now blocking)
    - .planning/phases/01-foundation/deferred-items.md (item closed)
tech_stack:
  added:
    - "@tauri-apps/api ^2.11.0 (runtime dep)"
    - "@tauri-apps/plugin-dialog ^2.7.1 (runtime dep)"
    - "svelte-spa-router ^5.1.0 (hash routing)"
  patterns:
    - "Svelte 5 runes ($state, $derived, $props) in all stores + components"
    - "transport-detect: __TAURI_INTERNALS__ runtime check"
    - "CSS custom properties via _tokens.scss auto-prepend"
    - "Hash router with use:link + use:active from svelte-spa-router"
key_files:
  created:
    - ui/src/lib/api/client.ts
    - ui/src/lib/api/errors.ts
    - ui/src/lib/api/index.ts
    - ui/src/lib/stores/theme.svelte.ts
    - ui/src/lib/stores/toast.svelte.ts
    - ui/src/lib/stores/transport.svelte.ts
    - ui/src/lib/utils/date.ts
    - ui/src/lib/components/Button.svelte
    - ui/src/lib/components/Input.svelte
    - ui/src/lib/components/Select.svelte
    - ui/src/lib/components/Textarea.svelte
    - ui/src/lib/components/Modal.svelte
    - ui/src/lib/components/Toast.svelte
    - ui/src/lib/components/ToastHost.svelte
    - ui/src/lib/components/ThemeSwitcher.svelte
    - ui/src/lib/components/Placeholder.svelte
    - ui/src/lib/components/Spinner.svelte
    - ui/src/lib/components/Badge.svelte
    - ui/src/features/layout/Layout.svelte
    - ui/src/features/layout/Sidebar.svelte
    - ui/src/features/layout/sidebar-config.ts
    - ui/src/routes.ts
    - ui/src/pages/Dashboard.svelte
    - ui/src/pages/MapPage.svelte
    - ui/src/pages/DevicesPlaceholder.svelte
    - ui/src/pages/ActsPage.svelte
    - ui/src/pages/PrintersPage.svelte
    - ui/src/pages/CartridgesPage.svelte
    - ui/src/pages/RequestsPage.svelte
    - ui/src/pages/ReportsPage.svelte
    - ui/src/pages/UsersPage.svelte
    - ui/src/pages/SettingsPage.svelte
    - ui/src/pages/NotFound.svelte
  modified:
    - ui/package.json (runtime deps added)
    - ui/pnpm-lock.yaml (lockfile updated)
    - ui/index.html (inline no-flash theme script)
    - ui/src/styles/_tokens.scss (full light+dark palette + spacing + typography)
    - ui/src/styles/global.scss (base styles, reduced-motion, focus-ring, skip-link)
    - ui/src/App.svelte (Router + Layout + ToastHost)
    - ui/src/main.ts (initTheme before mount, global.scss import)
    - ui/tsconfig.json ($lib paths alias)
    - ui/vite.config.ts ($lib resolve alias)
    - ui/eslint.config.js (ConfFile + matchMedia global added)
    - .github/workflows/ci-fast.yml (svelte-check now blocking)
    - .github/workflows/ci-full.yml (svelte-check now blocking)
    - .planning/phases/01-foundation/deferred-items.md (item marked resolved)
decisions:
  - "DevicesPlaceholder.svelte временный; Plan 03 заменит на features/devices/DevicesPage.svelte"
  - "Sidebar active link: :global(.nav-link.is-active) вместо :global(&.is-active) — Svelte CSS nesting restriction"
  - "initTheme() вызывается в main.ts ДО mount, чтобы document.documentElement.dataset.theme был установлен до первого рендера"
  - "toast max-10 cap: при превышении лимита oldest toast вытесняется (D-DoS-05 mitigated)"
metrics:
  duration: "~50 min (including interrupted previous session)"
  completed_date: "2026-05-26"
  tasks_completed: 4
  tasks_total: 4
  files_created: 33
  files_modified: 13
---

# Phase 2 Plan 02: UI Shell — Hash routing + sidebar + theme + primitives

**One-liner:** Svelte 5 SPA shell с sidebar 10-пунктов + hash-routing (svelte-spa-router) + transport-detect apiCall (__TAURI_INTERNALS__) + 11 hand-rolled UI-примитивов + no-flash theme persistence — полный навигационный каркас без конкретного feature-контента.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | JS deps + index.html + design tokens + global styles | f5d7311 | ui/package.json, ui/pnpm-lock.yaml, ui/index.html, ui/src/styles/_tokens.scss, ui/src/styles/global.scss |
| 2 | Stores + API client + 11 hand-rolled primitives | e9cb12a | 18 new files: api/*, stores/*, utils/date.ts, 11 components |
| 3 | Layout/Sidebar + sidebar-config + routes + 10 pages + App.svelte/main.ts | 7c7d01a | 17 files: features/layout/*, pages/*, routes.ts, App.svelte, main.ts |
| 4 | CI cleanup — снять continue-on-error + закрыть deferred-item | df9dde3 | ci-fast.yml, ci-full.yml, deferred-items.md |

## What Was Built

### Transport-detect API client
`ui/src/lib/api/client.ts` — `apiCall<R>(name, args)` с runtime-детектом `__TAURI_INTERNALS__`. В Tauri-режиме использует lazy `import('@tauri-apps/api/core')` + `invoke<R>`. В browser-режиме — `fetch('/api/v1/${name}')`. Ошибки нормализуются через `parseAppError`.

### Theme system (no-flash)
Inline script в `<head>` index.html читает `localStorage.getItem('trackly:theme')` и устанавливает `document.documentElement.dataset.theme` СИНХРОННО до Vite-модуля. `initTheme()` вызывается в main.ts ДО `mount()`. CSS custom properties переключаются через `[data-theme="dark"]` в `_tokens.scss`.

### UI Primitives (11 компонентов)
Button (5 вариантов), Input (invalid-state), Select (styled native), Textarea, Modal (Escape-close + focus trap), Toast (kind colors + aria-role), ToastHost (fixed bottom-right), ThemeSwitcher (3 segments на русском), Placeholder (section + phase subline), Spinner (CSS-only SVG), Badge (5 вариантов).

### Sidebar (14 entries — 10 items + 4 dividers)
Per UI-SPEC §Copywriting Sidebar: Дашборд, Карта | Устройства, Акты | Принтеры, Картриджи, Заявки | Отчёты, Пользователи | Настройки. Active link через `use:active` (is-active class). ThemeSwitcher в footer.

### Hash routing (svelte-spa-router)
10 маршрутов + `*` → NotFound. Все разделы открываются Placeholder-компонентом «Раздел в разработке». `/devices` — DevicesPlaceholder.svelte (Plan 03 заменит).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] CSS nesting selector `:global(&.is-active)` в Sidebar.svelte**
- **Found during:** Task 3 (svelte-check)
- **Issue:** Svelte 5 не поддерживает `&` nesting внутри `:global()` — ошибка "Nesting selectors can only be used inside a rule or as the first selector inside a lone `:global(...)`"
- **Fix:** Переместил стиль активной ссылки в `<style>` как `:global(.nav-link.is-active)` на верхнем уровне
- **Files modified:** ui/src/features/layout/Sidebar.svelte
- **Commit:** 7c7d01a

## Known Stubs

| Stub | File | Phase |
|------|------|-------|
| DevicesPlaceholder — section="Устройства" | ui/src/pages/DevicesPlaceholder.svelte | Plan 03 заменит на DevicesPage |
| Dashboard placeholder | ui/src/pages/Dashboard.svelte | Phase 7 |
| MapPage placeholder | ui/src/pages/MapPage.svelte | v2 |
| ActsPage placeholder | ui/src/pages/ActsPage.svelte | Phase 3 |
| PrintersPage placeholder | ui/src/pages/PrintersPage.svelte | Phase 6 |
| CartridgesPage placeholder | ui/src/pages/CartridgesPage.svelte | Phase 4 |
| RequestsPage placeholder | ui/src/pages/RequestsPage.svelte | Phase 6 |
| ReportsPage placeholder | ui/src/pages/ReportsPage.svelte | Phase 7 |
| UsersPage placeholder | ui/src/pages/UsersPage.svelte | Phase 5 |
| SettingsPage placeholder | ui/src/pages/SettingsPage.svelte | Phase 7 |

Все placeholder-страницы рендерят `<Placeholder>` компонент с section name и phase — это намеренный stub до соответствующих фаз реализации. Plan 03 заполняет Устройства.

## Verification Results

- `pnpm svelte-check`: 0 errors, 0 warnings (129 files) — PASS
- `pnpm lint`: ESLint + Prettier — PASS
- `pnpm build`: 169 modules, 0 errors — PASS
- `dist/index.html`: trackly:theme inline script preserved — PASS
- 14 sidebar entries (10 items + 4 dividers) — PASS
- `continue-on-error: true` removed from both CI workflows — PASS

## Awaiting Manual Verification (Task 5 — checkpoint:human-verify)

Task 5 is a `gate="blocking-human"` checkpoint requiring visual smoke test:
1. `pnpm tauri dev` — sidebar with 10 items, dividers, ThemeSwitcher footer
2. Theme switching (Светлая/Тёмная/Системная) — no-flash on reload
3. Hash routing — placeholder pages for all 10 sections

## Self-Check: PASSED

All created files verified to exist, all commits verified in git log.
