---
phase: 28-support-admin-windows
plan: 07
subsystem: ui-settings
tags: [design-system, tokens, settings, active-directory, org-settings]
dependency-graph:
  requires: []
  provides:
    - "ActiveDirectorySettings.svelte on Checkbox/Radio/Input primitives"
    - "OrgSettings.svelte on Input primitive (10 text fields)"
  affects:
    - ui/src/features/settings/ActiveDirectorySettings.svelte
    - ui/src/features/settings/OrgSettings.svelte
tech-stack:
  added: []
  patterns:
    - "Radio bind:group adapter for boolean state (regMode <-> settings.auto_accept via bidirectional $effect)"
key-files:
  created: []
  modified:
    - ui/src/features/settings/ActiveDirectorySettings.svelte
    - ui/src/features/settings/OrgSettings.svelte
decisions:
  - "org-email field uses Input type=\"text\" instead of native type=\"email\" — Input.svelte's type prop only supports 'text'|'number'|'search'; native HTML5 email validation is lost, server-side validation remains authoritative (documented deviation, low risk)"
metrics:
  duration: "5 min"
  completed: 2026-07-22
---

# Phase 28 Plan 07: Настройки AD и Организации — ре-токенизация Summary

Перевод внутренностей двух панелей окна Настроек на дизайн-систему: `ActiveDirectorySettings.svelte`
(чекбокс + boolean-radio-group адаптер + 5 read-only bootstrap-полей) и `OrgSettings.svelte`
(10 текстовых полей реквизитов организации), без изменения поведения (SC #4).

## What Was Built

### Task 1: ActiveDirectorySettings.svelte

- Чекбокс «Использовать Active Directory» → `Checkbox` (`checked`/`onchange`).
- Два radio «Автоматически принимать» / «Требовать подтверждения» → `Radio` с обязательным
  group-адаптером: `settings.auto_accept` (boolean) синхронизирован с производной строковой
  переменной `regMode: 'auto' | 'confirm'` через два `$effect`: один читает `settings.auto_accept`
  и обновляет `regMode` (внешняя загрузка через `loadSettings()`), другой читает `regMode` и
  обновляет `settings.auto_accept` (изменение через клик по Radio). Проверено вручную: нет
  зацикливания — второй effect устанавливает то же значение, которое первый уже прочитал, поэтому
  повторный проход не меняет `regMode`.
- Так как `Radio.svelte` сам рендерит `<label class="check-row">`, для текста рядом с
  переключателем текст передан как `children`-snippet внутрь `<Radio>`, а не как внешний
  `<label>`-обёртка (иначе получились бы вложенные `<label>` — невалидный HTML и риск
  двойного срабатывания клика). Внешний `<div class="radio-label">` оставлен только для
  вертикального отступа между двумя вариантами.
- 5 read-only bootstrap-полей (host:port/domain/base_dn/name_attr/no_tls_verify) → `Input`/`Checkbox`
  с `disabled`, без `onchange` (значения read-only, как раньше).
- Удалены bespoke `.checkbox-label`/`.checkbox-text`/`.radio-label input[...]`/`.form-input` CSS.
- `saveSettings`/`testConnection`-логика не изменена.

### Task 2: OrgSettings.svelte

- Все 10 raw `<input class="form-input">` (название/ИНН/КПП/адрес/адрес-2/телефон/факс/email/
  ОКПО/ОГРН) → `Input` через `bind:value` (Input.svelte's `value = $bindable('')` поддерживает
  прямой bind, ручной `oninput`-адаптер не понадобился).
- Поле `org-email` (было `type="email"`) → `Input type="text"` — `Input.svelte`'s `type` не
  включает `'email'` в контракте (`'text' | 'number' | 'search'`). Задокументировано как
  осознанное отклонение: теряется нативная HTML5 email-валидация в браузере, серверная валидация
  остаётся авторитетной (низкий риск, не блокирует SC #4).
- Секция логотипа (Button + hidden `<input type="file">` + `<img>`-рендер) не тронута.
- `saveOrg`-логика не изменена.
- Удалён bespoke `.form-input` CSS.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Radio-текст как children вместо внешнего `<label>`-обёртки**
- **Found during:** Task 1
- **Issue:** План предлагал обернуть `<Radio>` во внешний `<label class="radio-label">`, но
  `Radio.svelte` уже рендерит собственный `<label class="check-row">` — получился бы вложенный
  `<label>` (невалидный HTML5, потенциальный двойной toggle при клике по тексту).
- **Fix:** Текст (title+helper) передан как `children`-snippet внутрь `<Radio>`; внешняя обёртка
  заменена на `<div class="radio-label">` только для отступа между вариантами.
- **Files modified:** `ui/src/features/settings/ActiveDirectorySettings.svelte`
- **Commit:** 99b65bc

Прочее — plan executed as written (email-type deviation была явно предписана планом и
задокументирована как ожидаемая, не auto-fix).

### Auth Gates

None.

## Known Stubs

None.

## Threat Flags

None — обе панели используют существующие эндпоинты (`settings_get_ad`/`settings_set_ad`/
`ad_test_connection`/`settings_get_org`/`settings_save_org_fields`/logo-эндпоинты) без изменений.

## Self-Check: PASSED

- FOUND: ui/src/features/settings/ActiveDirectorySettings.svelte
- FOUND: ui/src/features/settings/OrgSettings.svelte
- FOUND commit 99b65bc (ActiveDirectorySettings)
- FOUND commit 21c7a3b (OrgSettings)
- `node ui/scripts/check-tokens.mjs` — PASS, 0 нарушений
- `pnpm --dir ui svelte-check` — 0 errors (48 pre-existing warnings in unrelated files)
