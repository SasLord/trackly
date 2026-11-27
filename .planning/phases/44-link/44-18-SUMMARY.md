---
phase: 44-link
plan: 18
subsystem: ui-admin-link
tags: [link, admin-ui, svelte, ws, dropdown]
requires: [44-16]
provides: [route /link, LinkPage, check-link-section gate]
affects: [44-20]
requirements: [LNK-07]
key-files:
  created:
    - ui/src/features/link/LinkPage.svelte
    - ui/src/features/link/LoginsTab.svelte
    - ui/src/features/link/InvitesTab.svelte
    - ui/src/features/link/InviteModal.svelte
    - ui/src/features/link/linkApi.ts
    - ui/scripts/check-link-section.mjs
  modified:
    - ui/src/routes.ts
    - ui/src/features/layout/sidebar-config.ts
    - ui/package.json
    - ui/scripts/check-ws-resync-registry.mjs
decisions:
  - "Модалки (подтверждение отзыва входа, попап кода) рендерятся в LinkPage, не в ячейках таблицы; вкладки получают только колбэки."
  - "InviteModal монтируется через {#if} в LinkPage: закрытие размонтирует компонент и стирает код (T-44-70); наружу код не передаётся, onIssued только перечитывает список."
  - "Активная вкладка хранится в hash-query (?tab=codes) через history.replaceState: без перемонтирования страницы и без push в историю."
  - "Состояние «сервер не запущен» определяется по коду ошибки CONFLICT от link_invites_create (контракт плана 16), а не по server_url настроек сети."
  - "Колонка «Связь» опущена (сервер не отдаёт состояние WS; UI-SPEC §8.2.1 это допускает)."
metrics:
  tasks: 3
  completed: 2026-11-28
---

# Phase 44 Plan 18: раздел Link в Trackly Summary

Раздел «Link» для администратора: активные входы с отзывом, запрещённые компьютеры с кнопкой «Разрешить», коды приглашения и двухшаговый попап выпуска кода; список обновляется по `link_changed` без тоста.

## Что сделано

- **Маршрут и сайдбар:** `'/link': LinkPage` только в `routes` (не в `employeeRoutes`); пункт «Link» с `roles: ['admin']` сразу после «Пользователи», комментарий-счётчик обновлён (14 пунктов + 4 разделителя).
- **linkApi.ts:** обёртки `apiCall` для 9 команд плана 16 (типы из `bindings.ts`, в рантайме он не импортируется).
- **LinkPage:** `height:100%; min-height:0`, внутренняя область скроллится сама; `Tabs underline` («Входы» со счётчиком, «Коды приглашения»); подписка `onWsEvent(link_changed)` и `onWsResync` с отпиской в cleanup (реестровый гейт `check-ws-resync-registry` покрывает новый файл; `MIN_SUBSCRIBERS` поднят 13 -> 14).
- **LoginsTab:** таблица активных входов (Сотрудник/логин, Компьютер, Способ входа, Вход выполнен), «Отозвать» (цвет danger-text); Spinner / пусто / ошибка по UI-SPEC; секция «Запрещённые компьютеры» только при непустом списке, Вариант А (D-13): «Разрешить» с loading, без подтверждения.
- **InvitesTab:** таблица кодов, бейджи состояния, инлайн-подтверждение отзыва по образцу `UserListRow`, пустое состояние с кнопкой выпуска.
- **InviteModal:** шаг 1 (адрес, предзаполнен хостом из `settings_get_network`; warning при IP; срок на `Dropdown variant="select"` 1 час / 24 часа / 7 дней; ошибка `role="alert"`; danger-callout «сервер не запущен» и disabled кнопка), шаг 2 (код `.tr-mono`, `user-select:all`, «Скопировать код» -> «Скопировано» 2 с, «Готово»).
- **Гейт `check-link-section.mjs`** (+ `--selftest`, 8 мутантов реальных файлов, подключён в `pnpm lint`): S1 admin-only и позиция после `/users`, S2 нет в `employeeRoutes`, S3 нет `<select`/`Select.svelte` и есть `Dropdown variant="select"`, S4 Modal вне td/TableRow и вне вкладок, S5 нет литеральных цветов.

## Гейты

| Команда | Результат |
|---------|-----------|
| `pnpm --dir ui lint` (включая link-ui, boundaries, check-link-section, check-tokens, check-contrast, check-ws-resync-registry, check-gate-coverage, check-lint-selftest-wiring) | exit 0 |
| `pnpm --dir ui svelte-check` | 0 errors, 0 warnings |
| `pnpm --dir ui build` | зелёный (ui/dist обновлён) |

## Deviations from Plan

**1. [Rule 3 - Blocking] Digit-prefix regex.** Гейт `check-act-number-no-regex-split` запрещает литерал `/^\d…/` в любом файле; определение IP-адреса в `InviteModal` переписано без него (`split('.')` + проверка символов). Поведение прежнее.

**2. [Rule 2 - Registry] `MIN_SUBSCRIBERS` 13 -> 14** в `check-ws-resync-registry.mjs`: LinkPage стал четырнадцатым подписчиком (по правилу гейта порог только растёт).

**3. Коммит один, а не три.** Задачи 1-3 взаимозависимы (`routes.ts` импортирует LinkPage, LinkPage импортирует обе вкладки и попап, гейт читает все файлы раздела), раздельные коммиты не собирались бы и не проходили `pnpm lint`. Все файлы в `54ec0b03`.

**4. Ранее зафиксированный красный `pnpm lint` (`queueMicrotask` в link-ui, deferred плана 16) в этом прогоне зелёный** (исправлено в промежутке другими коммитами); в этом плане link-ui не менялся.

## Авто-одобренные чекпоинты

Чекпоинтов в плане нет. Рантайм Svelte-рун компиляцией не проверяется; в запущенном приложении (план 20) человеку выполнить (auto-approved, NOT observed live):

1. Войти администратором, открыть «Link» в сайдбаре; войти сотрудником и убедиться, что пункта нет, а `#/link` даёт «Доступ запрещён».
2. Нажать «Выпустить код приглашения», оставить «24 часа», выпустить; код виден один раз, «Скопировать код» меняется на «Скопировано», «Готово» закрывает; повторно код нигде не виден, во вкладке «Коды приглашения» строка «Активен».
3. Ввести в поле адреса IP (например 192.0.2.10) и увидеть warning-callout; выбрать срок в Dropdown клавиатурой и мышью.
4. При выключенном сервере выпуск показывает danger-callout и блокирует кнопку.
5. С клиентского Link войти, увидеть строку во «Входах» без перезагрузки и без тоста; «Отозвать» -> Modal с фокусом на «Отмена»; подтвердить -> клиент отключён, компьютер в «Запрещённых»; «Разрешить» -> строка пропала, toast.
6. Отозвать активный код (инлайн «Отозвать?»); пустые состояния обеих вкладок; страница скроллится внутри себя на низком окне.
7. Проверить в LAN-браузере (после `pnpm --dir ui build`) и в десктопном окне.

## Known Stubs

Нет.

## Threat Flags

Новой серверной поверхности нет. T-44-69: `/link` только в admin-карте, гейт S2, серверный 403 (план 16). T-44-70: код только в локальном состоянии модалки, стирается размонтированием. T-44-71: принято.

## Self-Check: PASSED

Файлы раздела и `ui/scripts/check-link-section.mjs` существуют; коммит `54ec0b03` в истории.
