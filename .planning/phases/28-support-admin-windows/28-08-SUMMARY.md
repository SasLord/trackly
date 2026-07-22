---
phase: 28-support-admin-windows
plan: 08
subsystem: ui-settings
tags: [design-system, tokens, select-primitive, template-editor]
dependency-graph:
  requires: []
  provides:
    - "TemplateEditor.svelte kind-select on Select primitive"
  affects:
    - "ui/src/features/settings/TemplateEditor.svelte"
tech-stack:
  added: []
  patterns:
    - "Select primitive wrapped in .select-shrink (width: fit-content; min-width: 220px) for label+select flex rows — matches BackupSettings.svelte precedent"
key-files:
  created: []
  modified:
    - "ui/src/features/settings/TemplateEditor.svelte"
decisions:
  - "Reused .select-shrink wrapper pattern from BackupSettings.svelte (fit-content + min-width) instead of inventing new CSS, since Select's internal width:100% would otherwise stretch to fill the flex row"
metrics:
  duration: "8 min"
  completed: 2026-07-22
---

# Phase 28 Plan 08: TemplateEditor kind-select → Select Summary

Ре-токенизация единственного raw-контрола редактора Шаблонов (WIN-08, D-08): `<select id="template-kind" class="form-select">` заменён на `Select` примитив, без единого изменения в `textarea` тела шаблона, `iframe sandbox=""` превью или save/reset-логике.

## What Was Built

- `ui/src/features/settings/TemplateEditor.svelte`: `import Select from '$lib/components/Select.svelte'` добавлен; raw `<select class="form-select" bind:value={selectedKind}>` заменён на `<Select id="template-kind" value={selectedKind} onchange={(v) => (selectedKind = v)}>` с той же `{#each templates as tmpl (tmpl.kind)}<option>` разметкой внутри (children snippet, без изменений содержимого опций).
- Bespoke `.form-select` CSS-блок удалён, заменён на `.select-shrink { width: fit-content; min-width: 220px; }` — обёртка вокруг `Select`, воспроизводящая прежнее визуальное поведение (компактная ширина в flex-row `.template-selector-row`, min-width 220px как раньше). Паттерн скопирован из `BackupSettings.svelte` (`select-shrink`), где та же задача (label + Select в flex-row без растягивания на всю ширину) уже решена — `Select`'s внутренний `width: 100%` иначе растянул бы контрол на всю доступную ширину строки.
- Область редактирования (`textarea.template-textarea`, `bind:value={body}`), механика предпросмотра (`iframe sandbox="" srcdoc={previewHtml}`), `validateAndPreview`/`saveTemplate`/`resetTemplate`-логика и confirm-`Modal` «Сбросить шаблон?» — **не тронуты** (побайтово идентичны, подтверждено grep-гейтами).

## Deviations from Plan

None - plan executed exactly as written. Единственное отклонение от буквального текста плана — обёртка `<div class="select-shrink">` вокруг `<Select>` вместо голого `<Select class="form-select">` (Select не принимает произвольный `class` проп на корневой элемент), что является стандартным паттерном, уже применённым в `BackupSettings.svelte` для той же ситуации (label + Select в узкой flex-строке). Не архитектурное изменение — не требует Rule 4.

## Verification

- `node ui/scripts/check-tokens.mjs` — PASS, 0 нарушений
- `pnpm --dir ui svelte-check` — 0 ERRORS (48 pre-existing warnings в других файлах, не связаны с этой правкой)
- `grep -n "import Select"` → найден
- `grep -n '<Select id="template-kind"'` → найден
- `grep -v '^\s*//' ... | grep -c 'class="form-select"'` → 0
- `grep -c 'class="template-textarea"'` → 1 (не изменена)
- `grep -c 'sandbox=""'` → 1 (не изменена)

**Human-check (визуальная проверка светлая/тёмная тема, поведение выбора/редактирования/превью/сохранения/сброса шаблона) — не выполнена в рамках автономного execute; требуется на этапе `/gsd-verify-phase` или ручного UAT.**

## Threat Flags

None — правка ограничена markup-адаптером селектора, ни новой сетевой поверхности, ни новых путей записи/чтения не добавлено.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: `ui/src/features/settings/TemplateEditor.svelte` (modified)
- FOUND commit `90dc398`
