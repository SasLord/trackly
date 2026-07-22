---
phase: 28-support-admin-windows
plan: 09
subsystem: ui
tags: [svelte, design-system, table, form-primitives, badge, page-header]

# Dependency graph
requires:
  - phase: 25-dropdown
    provides: Table/TableRow primitives (D-09/D-10)
  - phase: 24-base-components
    provides: Input/Select/Checkbox/Badge primitives
  - phase: 26-windows-with-mockup
    provides: PageHeader primitive (D-07)
provides:
  - "Окно Пользователей (WIN-09) целиком на дизайн-системе: список на Table/TableRow, статус-бейдж на Badge, форма создания/редактирования на Input/Select/Checkbox, шапка на PageHeader"
affects: [30-quality]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Password field D-04 exception: raw <input type=\"password\"> retained when Input.svelte's type contract (text|number|search) has no password variant — masking is a security requirement, not a stylistic choice"
    - "Email field renders via Input type=\"text\" — HTML5 native email validation is lost when the primitive lacks an email type; server-side validation remains authoritative"

key-files:
  created: []
  modified:
    - ui/src/features/users/UsersList.svelte
    - ui/src/features/users/UserListRow.svelte
    - ui/src/features/users/UserFormModal.svelte
    - ui/src/features/users/UsersPage.svelte

key-decisions:
  - "Пароль остаётся raw <input type=\"password\"> (T-28-09-01) — единственное осознанное исключение из D-04 в фазе, т.к. Input.svelte не поддерживает type=\"password\"; рендер через type=\"text\" снял бы маскировку"
  - "Email-поле рендерится через Input type=\"text\" (Input.svelte не поддерживает type=\"email\") — нативная HTML5-валидация email потеряна, серверная валидация остаётся авторитетной"
  - "Inline-подтверждение удаления (Удалить?/Да/Нет) в UserListRow сохранено дословно — UI-SPEC §7.4 запрещает замену модалкой"
  - "Кнопки Изменить/Удалить/Да/Нет в UserListRow оставлены на bespoke .btn-action классе (Claude's Discretion из плана) — Button-примитив рассчитан на более крупные CTA"

patterns-established: []

requirements-completed: [WIN-09]

# Metrics
duration: 8min
completed: 2026-07-22
---

# Phase 28 Plan 09: Окно Пользователей на дизайн-системе Summary

**Список Пользователей на Table/TableRow/Badge, форма на Input/Select/Checkbox с обязательным raw-исключением для пароля, шапка на PageHeader**

## Performance

- **Duration:** 8 min
- **Started:** 2026-07-22T06:52:00Z
- **Completed:** 2026-07-22T06:59:35Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- `UsersList.svelte`/`UserListRow.svelte` переведены на `Table`/`TableRow`; статус-бейдж на `Badge` (variant success/default); inline-подтверждение удаления сохранено дословно; empty-state только с `emptyTitle`, без `emptyBody`
- `UserFormModal.svelte`: 5 из 6 полей (Логин/ФИО/Роль/Email/Активен) переведены на `Input`/`Select`/`Checkbox`; поле пароля — обязательное raw-исключение из D-04
- `UsersPage.svelte`: bespoke `<header class="page-header">` заменена на `PageHeader` с actions-snippet, кнопка «+ Добавить пользователя» сохранена

## Task Commits

Each task was committed atomically:

1. **Task 1: UsersList + UserListRow → Table/TableRow + Badge (D-03)** - `e15a0a7` (feat)
2. **Task 2: UserFormModal → Input/Select/Checkbox (D-04) + UsersPage → PageHeader** - `349b2d4` (feat)

## Files Created/Modified
- `ui/src/features/users/UsersList.svelte` - список пользователей на `Table` (columns=6, emptyTitle only)
- `ui/src/features/users/UserListRow.svelte` - строка на `TableRow`/`Badge`, inline-удаление сохранено дословно
- `ui/src/features/users/UserFormModal.svelte` - форма на `Input`/`Select`/`Checkbox`; пароль — raw-исключение
- `ui/src/features/users/UsersPage.svelte` - шапка на `PageHeader`

## Decisions Made
- Пароль остаётся raw `<input type="password">` (T-28-09-01) — единственное осознанное исключение из D-04 в фазе; `Input.svelte` не поддерживает `type="password"`, а рендер через `type="text"` снял бы маскировку и показал бы вводимые символы открытым текстом
- Email-поле рендерится через `Input type="text"` — `Input.svelte` не поддерживает `type="email"`; нативная HTML5-валидация email потеряна, серверная валидация остаётся авторитетной (задокументировано по требованию плана)
- Кнопки «Изменить»/«Удалить»/«Да»/«Нет» в `UserListRow` оставлены на bespoke `.btn-action` классе (Claude's Discretion из плана — `Button`-примитив рассчитан на более крупные CTA, не на мелкие inline-действия таблицы)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. Первая попытка acceptance-гейта на `type="password"` count==1 не прошла (4 совпадения — 3 в комментариях, объясняющих T-28-09-01, + 1 реальное использование); переформулировал текст комментариев без изменения смысла, гейт прошёл (count==1).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Последнее окно фазы 28 (WIN-06..09) на дизайн-системе. Human-визуальная проверка (светлая/тёмная тема, поведение inline-удаления, маскировка пароля) откладывается до конца фазы согласно `human_verify_mode: end-of-phase` в конфиге проекта — батчится вместе с остальными планами фазы 28.

---
*Phase: 28-support-admin-windows*
*Completed: 2026-07-22*

## Self-Check: PASSED
