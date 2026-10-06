# Deferred items (phase 41)

## 41-18 — svelte-check: 1 error outside plan scope — RESOLVED (commit 17d26df2, orchestrator)
- `ui/src/features/showcase/sections/MovementTimelineSection.svelte:16` — the fixture object lacks `batch_id`
  (`MovementEntryDto.batch_id: string | null` is required in the regenerated `bindings.ts`; the fixture gives `undefined`).
- Not caused by plan 41-18 (files untouched); appears since bindings gained `batch_id` for the group batch (D-25/D-26).
- Fix: add `batch_id: null` to the showcase fixture entries. Belongs to whichever plan owns MovementTimeline batch UI.
- DONE: the orchestrator added `batch_id`/`group_id`/`group_label` to the fixture factory in commit 17d26df2;
  svelte-check is back to the 0-errors / 68-warnings baseline. Root cause for the record: the backend waves'
  post-merge gates were cargo-only, so this frontend regression (introduced by plan 41-05) went unnoticed for
  seven waves until plan 41-18 ran the UI chain.

## 41-23 — вне границ плана
- `MovementTimeline.svelte`: пустой блок «Перемещений ещё не было…» остаётся устройство-ориентированным (для группы GroupPanel рисует свой); добавлен только `showInitialPlacementNote` для сноски под списком.

## 41-27 — вне границ плана

- `DeviceContextMenu.svelte` и `CartridgeContextMenu.svelte` — самостоятельные контекстные меню по правому клику со своей портальной реализацией (`.ctx-menu-portal`, `use:portal` в <body>, собственный click-outside через `closest('.ctx-menu-portal')`). Уже выводятся порталом и не раздвигают контейнеры, поэтому жалобу UAT (тест 5) не порождают; в гейт `check-action-menu-portal.mjs` и инвентарь 13 вызовов `ActionMenu` не входят. Унификация с `ActionMenu` — отдельная задача: иной триггер (правый клик, позиционирование по координатам курсора) и иной набор клавиш.

## UAT R5 — счётчики отчётов не работают по LAN (дефект ВНЕ фазы 41, отложен решением пользователя 2026-10-06)

Найден при живой приёмке пункта R5. Сам R5 (баннер усечения отчёта
«Перемещения», планы 41-33/41-30) отработал верно; это побочная находка.

**Симптом:** только в LAN-браузере все счётчики на вкладках «Отчётов» равны 0 —
«Устройства» (Акты, Возвраты, В работе, На складе), «Картриджи» (Расход,
История заправок, В работе, На складе), «Заявки» (Все, Открытые, В работе,
Выполненные), «Перемещения» (Все перемещения). В десктопе через
`cargo tauri dev` счётчики верные.

**Происхождение:** не фаза 41. Счётчики введены в G2-5b (фаза 28) и по HTTP не
работали с самого начала — на них просто не смотрели из браузера.

**Корневая причина — наложение трёх дефектов:**

1. У команды `reports_get_report_counts` нет HTTP-маршрута.
   `crates/trackly-app/src/http/reports.rs:359` регистрирует 15 маршрутов, этого
   среди них нет, хотя `build_reports_get_report_counts`
   (`tauri_cmds/reports.rs:832`) помечен «callable from both Tauri and HTTP».
   Сверка показала: из 184 команд Tauri без маршрута только 4, и три прочих
   (`app_restart`, `settings_move_db`, `settings_open_db_folder`) десктопные по
   смыслу и закрыты проверкой `isTauri` в `StorageSettings.svelte:42-44`.
   `reports_get_report_counts` — единственный случайный пропуск, ничем не закрытый.
2. `spa_fallback` (`http/mod.rs:252`) не исключает `/api/`: POST на
   несуществующий `/api/v1/*` возвращает `index.html` с **HTTP 200** и
   `text/html` вместо 404. Отсутствующий маршрут отвечает «успехом» — поэтому
   дыра и прожила незамеченной.
3. Клиент глотает отказ: пустой `.catch(() => {})` в
   `ReportsPage.svelte:449` не логирует ничего, `statusCounts` остаётся `{}`,
   а в `ReportSubNav.svelte:132` пустой объект truthy — ветка отдаёт `0`
   вместо задуманного отката на `rowCount`.

**Что нужно при починке:**
- маршрут `/api/v1/reports_get_report_counts` поверх `build_reports_get_report_counts`;
- исключить `/api/` из `spa_fallback` — несуществующий API-путь должен давать 404 JSON;
- логировать отказ загрузки счётчиков вместо пустого `.catch()`;
- гейт: тест, сверяющий список команд Tauri со списком HTTP-маршрутов, с явным
  allowlist десктопных (`app_restart`, `settings_move_db`, `settings_open_db_folder`).

Полная запись с artifacts — в `41-HUMAN-UAT.md`, раздел `## Gaps`, тест 24.

## Находки аудита /gsd-validate-phase (2026-10-06, вне границ фазы 41)

- **`settings_open_db_folder` не закрыт `isTauri`.** `openFolder()` в
  `ui/src/features/settings/StorageSettings.svelte:29-39` гарда не имеет, кнопка
  «Открыть папку с базой данных» рисуется и в LAN-браузере (`:103-105`); `isTauri`
  (`:41-47`) закрывает только `settings_move_db` и `app_restart`. Отказ виден тостом,
  данные не страдают. Нужно: гард или скрытие кнопки в браузере.
- **`users_reset_password` — HTTP-only сирота.** `handler_reset_password` и
  `build_users_reset_password` есть в `crates/trackly-app/src/http/users.rs`, но ни
  команды Tauri, ни вызывающей стороны в `ui/src` нет — админский сброс пароля
  недостижим из интерфейса на обоих транспортах. Решить: доделать UI или удалить маршрут.
- Оба зафиксированы вердиктами в новом гейте `crates/trackly-app/tests/http_route_parity.rs`,
  поэтому молча не разойдутся.
