---
phase: 42-link-monorepo-foundation
plan: 17
subsystem: attachments
tags: [attachments, packages, gates, LNK-01]
requires: [42-15]
provides:
  - packages/attachments/src - 9 перенесённых файлов + host.ts (setAttachmentsHost) + types.ts (структурные DTO)
  - ui/src/packages-compat.ts - проверка взаимной присваиваемости 5 DTO вложений bindings <-> пакет
key-files:
  created:
    - packages/attachments/src/{host,types}.ts
  modified:
    - packages/attachments/package.json
    - ui/src/main.ts
    - ui/src/packages-compat.ts
    - ui/scripts/check-{attachments-block,employee-api,user-scoped-reset,workspace-boundaries}.mjs
    - ui/scripts/fixtures/{move-map,gate-coverage.baseline}.json
metrics:
  tasks: 2
  completed: 2026-11-26
---

# Phase 42 Plan 17: вложения в packages/attachments Summary

Подсистема вложений (10 файлов) переехала из `ui/src/features/attachments` в `packages/attachments/src`; пакет не импортирует `bindings*`, `auth` и `features/*`. Права текущего пользователя приходят через внедряемый хост (`setAttachmentsHost`, регистрация в `ui/src/main.ts` до `mount`). Коммиты: 9ee438a2 (перенос), 00da922b (гейты).

## Что сделано

- `host.ts`: интерфейс `AttachmentsHost { currentUser() }`; без регистрации возвращает `null` (минимум прав). `AttachmentsBlock` берёт `currentUser()?.id` (свои вложения), `AttachmentHistory` - `currentUser()?.role === 'admin'`. Адаптер в `main.ts` читает `authStore.user`, реактивность сохранена.
- `types.ts`: структурные `AttachmentDto`, `AttachmentListDto`, `AttachmentHistoryEventDto`, `AttachmentLimitDto`, `IntegrityReportDto` (последние два нужны `api.ts`). `packages-compat.ts` проверяет все пять (больше трёх из плана). Мутация `sizeBytes: number -> string` дала 7 ошибок svelte-check, возврат -> 0/0.
- `package.json`: `exports` - явный `./attachmentEvents.svelte` (иначе шаблон `./*.svelte` резолвил бы его в несуществующий `.svelte`), `./*.svelte`, `./*`.
- Потребители (в т.ч. `features/requests/chat/*`, D-19) перенацелены на `@trackly/attachments/...`. Чат не перемещён, в его диффе только строки импортов (плюс перенос строк импортов prettier).
- Гейт границ: `packages/attachments/src/api.ts` добавлен в `TAURI_IMPORT_ALLOWLIST` (`convertFileSrc` схемы `trackly-att`).

## Гейты

- `check-attachments-block`: `DIR` -> `packages/attachments/src` (G0 падает, если файла блока нет); `pinned`-запись baseline перенацелена в том же коммите (`ui/src/features/attachments` -> `packages/attachments/src`, count 6). Правила G1-G15 не менялись по смыслу: опоры на `authStore` у гейта не было. Мутация `ev.type === 'attachments_changed' -> 'other'` в `packages/attachments/src/AttachmentsBlock.svelte` -> красный [G2]; возврат -> зелёный. Selftest 26 мутантов.
- `check-employee-api`: резолвер замыкания не знал `@trackly/*` (skip), из-за чего файлы вложений и обёртка `attachments.*` выпали бы из проверки вызовов. Добавлено отображение `@trackly/attachments/*` -> `../packages/attachments/src/*`; замыкание 89 файлов/89 вызовов, `MIN_FILES` 86 -> 89 (вверх). Selftest-мутация перенацелена на путь пакета; мутация снятия `canManageDeleted` красная.
- `check-user-scoped-reset`: EXEMPT-ключ -> `../../packages/attachments/src/attachmentEvents.svelte.ts`. Мутация (в `labels.ts` пакета импорт `./api` + модульный `Map`) -> красный [A], возврат -> зелёный: walk() покрывает пакет.
- `check-gate-coverage` PASS (selftest 17), `lint-selftest-wiring` 75 гейтов, `check-workspace-boundaries` PASS.

Понижение замороженного счётчика: `place-tree-invalidation` ui/src 259 -> 250 (9 файлов вложений ушли из ui/src; гейт следит за записью дерева мест, вложения его write-site'ов не содержат). Других понижений нет.

Проверки: `pnpm --dir ui lint` 0, `svelte-check` 485 файлов 0/0, `vite build` зелёный, `pnpm --dir link-ui build` и `lint` зелёные, privacy-гейт PASS.

## Отклонения от плана

**1. [Rule 3]** Кодмод `--apply` отказывается переносить файлы с одноуровневыми импортами внутри `features/` (классифицирует их как непакетные), поэтому перенос сделан `git mv` + скриптовая замена спецификаторов потребителей; записи в `move-map.json` дописаны вручную (11 строк).
**2. [Rule 2]** Резолвер `check-employee-api` расширен на `@trackly/attachments/*` (иначе потеря покрытия, см. выше).

## UNVERIFIED

- Поведение прав (свои вложения, «Удалённые» у администратора) под реальным пользователем в десктопе и LAN-браузере: стенд D-21 (план 19). Статически проверено только присваивание и гейты.

## Self-Check: PASSED
