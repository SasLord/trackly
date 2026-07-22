---
phase: 28-support-admin-windows
plan: 02
subsystem: ui-requests
tags: [design-system, detail-panel, page-header, requests]
dependency-graph:
  requires:
    - "27-01: DetailPanel/DetailSection/DetailField primitives extracted"
    - "26 D-07: PageHeader primitive extracted"
    - "28-01: RequestsMasterDetail/RequestsSearchAndTabs/RequestsList/RequestListRow migrated to design system (this plan builds on that structural wave)"
  provides:
    - "Заявки (WIN-06): RequestDetail on DetailPanel/DetailSection/DetailField"
    - "Заявки (WIN-06): RequestsPage on PageHeader"
  affects:
    - ui/src/features/requests/RequestDetail.svelte
    - ui/src/features/requests/RequestsPage.svelte
tech-stack:
  added: []
  patterns:
    - "DetailPanel header with two-badge title-row + meta-row as bespoke first children content (PrinterDetail precedent), panelTitle as plain typeLabel string"
    - "Lifecycle buttons without an attached inline field moved to DetailPanel actions snippet; inline-mini-form buttons (Выполнить + completeNotes Textarea) stay together in the body"
    - "DetailField reused for resolution-notes display (label+value box), replacing bespoke field-label/field-value spans"
key-files:
  created: []
  modified:
    - ui/src/features/requests/RequestDetail.svelte
    - ui/src/features/requests/RequestsPage.svelte
decisions:
  - "panelTitle = typeLabel (simple string) per plan instruction; the two-badge + meta-row header content renders as bespoke markup inside DetailPanel's children, not via the title prop"
  - "RequestFormModal.svelte required no changes — audit confirmed 0 raw <input> tags, already 100% on Select/Textarea/GroupedPrinterSelect primitives from Phase 6"
metrics:
  duration: "20 min"
  completed: 2026-07-22
---

# Phase 28 Plan 02: Заявки — деталь, форма, шапка Summary

Перевели `RequestDetail` на общий `DetailPanel`/`DetailSection`/`DetailField` по прецеденту заголовка `PrinterDetail` (title-row с двумя Badge + meta-row вместо простого строкового title), а `RequestsPage` — на примитив `PageHeader`; `RequestFormModal` прошла аудит D-04 без правок (уже на 100% примитивах).

## What Was Built

**Task 1 — RequestDetail → DetailPanel/DetailSection/DetailField (D-01):**
- `RequestDetail.svelte` переписан на общие примитивы (`DetailPanel`, `DetailSection`, `DetailField`), извлечённые в 27-01.
- Заголовок — по прецеденту `PrinterDetail.svelte`, а не `ActDetail.svelte`: `panelTitle` = `typeLabel` (простая строка для `DetailPanel`'s `title`-пропа), а `title-row` (два `Badge`: тип + статус) и `meta-row` (автор/дата) скопированы дословно как первый bespoke-контент внутри `children`, сразу после `{#snippet actions()}`.
- Секция «Информация» переведена на `<DetailSection heading="Информация">` с `.fields-grid` (2-колоночный grid) внутри которого — условные `DetailField` по типу заявки (`ad_register`/`cartridge_replace`/`free_form`), включая широкие поля (`.field-wide`) для комментария/описания.
- Все lifecycle-кнопки без прикреплённого инлайн-поля (Подтвердить, Принять в работу, Отклонить, Установить картридж, Отменить заявку, Удалить) перенесены в `{#snippet actions()}` `DetailPanel` с СОХРАНЕНИЕМ той же условной логики видимости по роли (`isAdRegister`/`isAdmin`/`isSpecialist`/`isOwnRequest`) и статусу заявки.
- Кнопка «Выполнить» (in_progress + free_form) осталась в теле вместе с `completeNotes`-Textarea внутри `<DetailSection>` — инлайн-мини-форма не разделена; при этом соседняя кнопка «Отклонить» (не связанная с инлайн-полем) перенесена в header actions.
- Отображение `resolutionNotes` (admin/specialist/employee terminal-ветки) переведено с bespoke `field-label`/`field-value` span на `DetailField` внутри `.resolution`-контейнера (сохранены padding/background/border).
- Секция «История» → `<DetailSection heading="История">` с существующей `{#each historyEntries}`-разметкой без изменений внутри.
- Удалены bespoke CSS-классы `.detail-header`/`.section`/`.section-heading` (заменены примитивами). 4 confirm-`<Modal>` (Отклонить/Удалить/Отменить/Подтвердить AD) не тронуты — D-04 территория.

**Task 2 — RequestFormModal аудит (D-04) + RequestsPage → PageHeader:**
- `RequestFormModal.svelte` — финальный аудит на предмет остаточного raw HTML: `grep '<input'` вернул 0 совпадений. Компонент уже полностью на `Select`/`Textarea`/`GroupedPrinterSelect` (Phase 6), правок не потребовалось.
- `RequestsPage.svelte`: bespoke `<header class="page-header"><h1 class="page-title">Заявки</h1></header>` заменён на `<PageHeader title="Заявки" />` (без `actions`-snippet — «Создать заявку» остаётся в `RequestsSearchAndTabs`, не дублируется). Удалены `.page-header`/`.page-title` CSS-классы.

## Verification

- `node ui/scripts/check-tokens.mjs` → `PASS — 0 нарушений` (запущено дважды, после каждой задачи).
- `pnpm --dir ui svelte-check` → `0 ERRORS, 48 WARNINGS` (все 48 предупреждений pre-existing, не в изменённых файлах — подтверждено сверкой warning-списка до/после).
- Acceptance grep-гейты (все выполнены):
  - `RequestDetail.svelte`: `import DetailPanel`/`<DetailPanel`/`import DetailSection`/`import DetailField` — присутствуют.
  - `class="detail-header"` count == 0.
  - «История пуста»/«Выберите заявку»/«Выберите заявку слева» — 3 совпадающие строки (>= 2).
  - Lifecycle-кнопки (Подтвердить/Принять в работу/Выполнить/Установить картридж/Отклонить/Отменить заявку/Удалить) — все присутствуют.
  - `<Modal` count == 4 (reject/delete/cancel/approve).
  - `RequestFormModal.svelte`: raw `<input` count == 0.
  - `RequestsPage.svelte`: `import PageHeader`/`<PageHeader title="Заявки"` присутствуют; `class="page-header"` count == 0.

## Deviations from Plan

None — plan executed exactly as written. `RequestFormModal.svelte` required no code changes (plan anticipated this: "если аудит не находит остаточного raw HTML, задача сводится к подтверждению 100%-покрытия").

## Known Stubs

None.

## Threat Flags

None — no new network endpoints, auth paths, or trust-boundary changes. Both threat-register mitigations (T-28-02-01 role/status condition literalness, T-28-02-02 confirm-Modal preservation) verified via the grep gates above and manual condition-by-condition review during the rewrite.

## Self-Check: PASSED

- FOUND: `ui/src/features/requests/RequestDetail.svelte`
- FOUND: `ui/src/features/requests/RequestsPage.svelte`
- FOUND commit `11664c7` (feat(28-02): RequestDetail on DetailPanel/DetailSection/DetailField (D-01))
- FOUND commit `53ee0c9` (feat(28-02): RequestsPage adopts PageHeader; RequestFormModal audit (D-04))
