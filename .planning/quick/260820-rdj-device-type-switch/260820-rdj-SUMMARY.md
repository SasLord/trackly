---
phase: 260820-rdj-device-type-switch
plan: 01
subsystem: devices
tags: [rusqlite, sqlite, svelte, svelte5-runes, tauri, printers, devices, modal]
status: complete

requires: []
provides:
  - "SqlitePrinterRepository::exists_for_device_in_tx / delete_by_device_id_in_tx tx-helpers"
  - "DeviceService::sync_printer_row_in_tx — атомарная идемпотентная синхронизация printers при create/update/bulk_create"
  - "Modal.svelte: module-level стек открытых модалов (Escape/Tab-trap/backdrop-dismiss только для верхнего, z-index по глубине) + titleExtra snippet-слот"
  - "ActionMenu.svelte: variant='ghost-sm' триггер (28px, без бордера)"
  - "DeviceFormModal.svelte: переключатель типа устройства (кебаб-меню), реактивный заголовок, владелец downgrade-подтверждения (вложенный Modal), onSaved(result?) с итоговым type_id"
affects: [devices-page, printers-page, printer-detail]

tech-stack:
  added: []
  patterns:
    - "Svelte 5: module-level $state как общий стек компонентных инстансов (Modal open-stack) — мутации ОБЯЗАТЕЛЬНО через untrack() внутри $effect, который читает и пишет тот же $state, иначе effect_update_depth_exceeded"
    - "Вложенные модалы как отдельные top-level <Modal> сиблинги, не вложенная разметка внутри .modal-body (backdrop-filter создаёт containing block для position:fixed — вложенный backdrop иначе прибивается к боксу внешнего)"
    - "Обратно-совместимое расширение callback-пропа (onSaved: () => void → onSaved: (result?: {...}) => void) — существующие вызовы без аргумента продолжают типизироваться и работать"

key-files:
  created:
    - crates/trackly-app/tests/devices_type_conversion.rs
  modified:
    - crates/trackly-infra/src/repos/printers_sqlite.rs
    - crates/trackly-app/src/services/device_service.rs
    - ui/src/lib/components/Modal.svelte
    - ui/src/lib/components/ActionMenu.svelte
    - ui/src/features/devices/DeviceFormModal.svelte
    - ui/src/features/devices/DeviceFormBody.svelte
    - ui/src/features/printers/PrinterDetail.svelte
    - ui/src/features/printers/PrintersPage.svelte

key-decisions:
  - "Полная конверсия типа (не soft-flag): Устройство→Принтер создаёт printers с дефолтами (ip=NULL, community='public', snmp_version='v2c'); Принтер→Устройство удаляет printers + каскадно printer_readings/printer_alerts (ON DELETE CASCADE V022/V023) — атомарно в той же writer-транзакции, что и devices INSERT/UPDATE"
  - "update() синхронизирует printers по финальному after.type_id (не по patch.type_id, обычно None) — идемпотентно при КАЖДОМ update, а не только когда тип явно меняется"
  - "Downgrade-подтверждение — решение и state (typeId, target) владеет DeviceFormModal, не DeviceFormBody; подтверждение — отдельный вложенный <Modal>, не inline-блок внутри тела формы (UAT round 1 gap-closure)"
  - "PrinterDetail.onRefresh (SNMP-опрос) и onDeviceSaved (реакция на сохранение формы устройства) — два разных пропа, не единый обработчик (UAT round 1 gap-closure) — смешение приводило к ложному тосту «Принтер не отвечает на SNMP» после downgrade"

requirements-completed: [RDJ-01, RDJ-02, RDJ-03, RDJ-04, RDJ-05, RDJ-06]

duration: ~70 min активной работы (реализация + 2 раунда UAT gap-closure); включает ожидание ручной UAT-проверки пользователем между раундами (не входит в активное время)
completed: 2026-08-20
---

# Quick Task 260820-rdj: Переключатель типа устройства (Устройство ⇄ Принтер) Summary

**Кебаб-меню в попапе создания/редактирования устройства переключает тип на «Принтер»/«Устройство» с полной атомарной конверсией записи на бэкенде (создание/удаление строки `printers` + каскад истории мониторинга) и явным подтверждением потери данных при downgrade через отдельный вложенный попап.**

## Performance

- **Duration:** ~70 min активной работы (2 автоматизированных задачи + 3 итерации UAT gap-closure)
- **Tasks:** 2/2 автоматизированных задачи + checkpoint:human-verify (2 раунда UAT, approved на втором)
- **Files modified:** 8 (1 новый тестовый файл, 7 изменённых)

## Accomplishments

- `SqlitePrinterRepository`: `exists_for_device_in_tx`/`delete_by_device_id_in_tx` tx-helpers + 2 unit-теста (идемпотентность delete, cascade на `printer_readings`/`printer_alerts`).
- `DeviceService::sync_printer_row_in_tx` — вызывается внутри одной writer-транзакции из `create()`, `update()` (по финальному `after.type_id`) и `bulk_create()`; полностью атомарна и идемпотентна. 6 новых интеграционных тестов покрывают upgrade/downgrade/идемпотентность/bulk-create.
- `Modal.svelte` получил переиспользуемый `titleExtra` snippet-слот и (по итогам UAT gap-closure) module-level **стек открытых модалов**: Escape/Tab-trap/backdrop-dismiss срабатывают только для верхнего инстанса, z-index backdrop'а — от глубины в стеке. Существующие потребители (`RequestDetail`, `PdfPreviewModal`, `OperationModal`, `PrinterCreateModal`) не затронуты — они всегда открывают не более одного модала одновременно.
- `ActionMenu.svelte`: новый `variant="ghost-sm"` (28px, без бордера) поверх существующего `default`-триггера (36px, бордер, используется в `DevicesPage` «Импорт и экспорт» — не тронут).
- `DeviceFormModal.svelte`: кебаб-меню «Устройство»/«Принтер» с галочкой (`var(--tr-accent)`) на выбранном, реактивный заголовок (4 варианта), владеет решением о downgrade-подтверждении и рендерит его как отдельный вложенный `<Modal>` («Сменить тип на «Устройство»?», только «Отмена»/«Да, сохранить»). `onSaved` расширен до `(result?: { typeId: number }) => void` — обратно совместимо с существующими вызовами без аргумента.
- `DeviceFormBody.svelte` — «тупая» форма: принимает `typeId` пропом, сохраняет с ним, ничего не знает о подтверждении конверсии.
- `PrinterDetail.svelte` — фикс бага «заголовок попапа редактирования принтера всегда показывал «Редактирование устройства»» (теперь заголовок вычисляется из `target.type_id` в `DeviceFormModal`); новый проп `onDeviceSaved`, отделённый от SNMP-опроса `onRefresh`.
- `PrintersPage.svelte` — обработчик `onDeviceSaved`: если тип изменился на «не принтер» — сбрасывает `selectedId` и перезагружает список; если тип не менялся — перечитывает деталь + список. Ни один путь больше не показывает ложный SNMP-тост.

## Task Commits

Итоговый набор коммитов задачи, по порядку:

1. **Task 1: Backend — атомарная синхронизация printers при конверсии типа устройства** - `c849322a` (feat)
2. **Task 2: Frontend — переключатель типа устройства в попапе + фикс заголовка PrinterDetail** - `9ff13d40` (feat)
3. **UAT round 1, дефект 1: downgrade-подтверждение — вложенный модал вместо inline** - `781d98de` (fix)
4. **UAT round 1, дефект 2: список принтеров обновляется после конверсии (SNMP-опрос отделён от reload)** - `c66e6bea` (fix)
5. **UAT round 2, регрессия: `untrack` в стеке модалов — `effect_update_depth_exceeded`** - `89413288` (fix)

**Plan metadata:** `485abedf` (docs: план, зафиксирован оркестратором до начала выполнения).
**Summary/state:** коммитятся оркестратором отдельно от этого исполнения.

## Files Created/Modified

- `crates/trackly-infra/src/repos/printers_sqlite.rs` - `exists_for_device_in_tx`/`delete_by_device_id_in_tx` + 2 unit-теста
- `crates/trackly-app/src/services/device_service.rs` - `printer_repo` поле, `sync_printer_row_in_tx`, 3 точки вызова (create/update/bulk_create)
- `crates/trackly-app/tests/devices_type_conversion.rs` - 6 интеграционных тестов (create×2 типа, update upgrade/downgrade/идемпотентность, bulk_create)
- `ui/src/lib/components/Modal.svelte` - `titleExtra` slot; module-level стек открытых модалов (isTop/z-index по глубине, `untrack` в push/cleanup)
- `ui/src/lib/components/ActionMenu.svelte` - `variant="ghost-sm"`
- `ui/src/features/devices/DeviceFormModal.svelte` - кебаб-меню типа, реактивный заголовок, владелец downgrade-решения + вложенный confirm-`Modal`, `onSaved(result?)`
- `ui/src/features/devices/DeviceFormBody.svelte` - принимает `typeId` пропом, сохраняет с ним; «тупая» форма (без знания о confirm-флоу)
- `ui/src/features/printers/PrinterDetail.svelte` - `onDeviceSaved` проп, отделён от SNMP `onRefresh`
- `ui/src/features/printers/PrintersPage.svelte` - обработчик `onDeviceSaved`: сброс выделения при уходе типа / reload детали+списка при сохранении без смены типа

## Decisions Made

- Полная конверсия (не мягкий флаг): смена типа синхронно создаёт/удаляет строку `printers` в той же транзакции, что и `devices` INSERT/UPDATE — конверсия никогда не оставляет `devices.type_id=2` без строки `printers` или наоборот, даже при крахе процесса.
- `update()` ключует синхронизацию от финального `after.type_id`, а не от `patch.type_id` (обычно `None`) — так вызов остаётся идемпотентным при каждом `update()`, а не только когда тип реально меняется в этом конкретном запросе.
- Downgrade-подтверждение вынесено из `DeviceFormBody` в `DeviceFormModal` и реализовано как настоящий вложенный `<Modal>` (не inline-блок в теле формы) — по итогам первого раунда UAT: пользователь указал на путаницу от одновременно интерактивных кебаб-меню/футера/inline-кнопок.
- `PrinterDetail`/`PrintersPage`: SNMP-опрос (`onRefresh`) и реакция на сохранение формы устройства (`onDeviceSaved`) — два разных callback-пропа, не единый обработчик — по итогам первого раунда UAT: смешение приводило к попытке SNMP-опроса уже несуществующего принтера и ложному тосту об ошибке.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Snippet-shadowing ошибка при первой реализации titleExtra**
- **Found during:** Task 2, первый прогон `svelte-check`
- **Issue:** Явная передача `{titleExtra}` в открывающий тег `<Modal>` конфликтовала с автоматическим forwarding'ом `{#snippet titleExtra()}`, объявленного как дочерний блок того же `<Modal>` — компилятор Svelte 5 сообщал "snippet shadowing prop titleExtra".
- **Fix:** Убрана явная передача `{titleExtra}` — snippet, объявленный как прямой child компонента, автоматически становится соответствующим именованным пропом (тот же паттерн, что уже использовался для `footer`).
- **Files modified:** `ui/src/features/devices/DeviceFormModal.svelte`
- **Verification:** `pnpm svelte-check` — 0 ошибок.
- **Committed in:** `9ff13d40`

**2. [UAT round 1, дефект 1 — Rule 4-style архитектурное изменение по прямому запросу пользователя] Inline-подтверждение → вложенный модал**
- **Found during:** Первый раунд ручной UAT-проверки (checkpoint Task 3)
- **Issue:** Downgrade-подтверждение рендерилось inline внутри `.modal-body` попапа редактирования — кебаб-меню, футер попапа И кнопки предупреждения были одновременно интерактивны; меню перекрывало текст предупреждения.
- **Fix:** `Modal.svelte` получил module-level стек открытых модалов (Escape/Tab-trap/backdrop-dismiss только для верхнего, z-index по глубине); `DeviceFormModal` теперь владеет решением о downgrade и рендерит вложенный `<Modal>` как top-level сиблинг (не внутри `DeviceFormBody`, чей родитель имеет `backdrop-filter`, создающий containing block для `position:fixed`); `DeviceFormBody` вернулся к роли «тупой» формы.
- **Files modified:** `ui/src/lib/components/Modal.svelte`, `ui/src/features/devices/DeviceFormModal.svelte`, `ui/src/features/devices/DeviceFormBody.svelte`
- **Verification:** `svelte-check`/`lint`/`build` чисто; повторная ручная UAT — approved (раунд 2, после исправления регрессии ниже).
- **Committed in:** `781d98de`

**3. [UAT round 1, дефект 2 — Rule 1 Bug] SNMP-опрос вместо перезагрузки списка принтеров**
- **Found during:** Первый раунд ручной UAT-проверки
- **Issue:** `onSaved` попапа редактирования устройства в `PrinterDetail` был подключён к `onRefresh` — SNMP-опросу конкретного принтера (`printers.refresh`). После downgrade строка `printers` уже удалена → опрос падает → ложный тост «Принтер не отвечает на SNMP», список не перечитывается.
- **Fix:** `DeviceFormModal.onSaved` расширен до `(result?: { typeId: number }) => void` (обратно совместимо); `PrinterDetail` получил отдельный проп `onDeviceSaved`; `PrintersPage` реализовал корректную логику (сброс выделения + reload при уходе типа, reload детали+списка иначе), без SNMP-тостов на этом пути.
- **Files modified:** `ui/src/features/devices/DeviceFormModal.svelte`, `ui/src/features/printers/PrinterDetail.svelte`, `ui/src/features/printers/PrintersPage.svelte`
- **Verification:** `svelte-check`/`lint`/`build` чисто; повторная ручная UAT — approved (раунд 2).
- **Committed in:** `c66e6bea`

**4. [UAT round 2 — Rule 1 Bug, критическая рантайм-регрессия] `effect_update_depth_exceeded` при открытии ЛЮБОГО модала**
- **Found during:** Второй раунд ручной UAT-проверки (после исправлений выше)
- **Issue:** Module-level стек модалов, добавленный в `781d98de`, читал и писал один и тот же `$state` (`openStack`) внутри одного `$effect`; `openStack = [...openStack, instanceId]` создаёт новую ссылку на каждом прогоне — эффект бесконечно переинвалидировал сам себя. Svelte падал с `effect_update_depth_exceeded` и убивал реактивность всего дерева компонентов — ломалось на первом же открытии ЛЮБОГО модала во всём приложении, не только вложенного confirm'а.
- **Fix:** Мутации стека (push и cleanup-remove) обёрнуты в `untrack()` из `svelte`.
- **Files modified:** `ui/src/lib/components/Modal.svelte`
- **Verification:** Воспроизведено и починено на живом временном Vite-харнессе с двумя вложенными `<Modal>` (харнесс удалён после проверки) — консоль чистая, вложенный модал открывается с корректным z-index (500/510), Escape закрывает только верхний. Финально подтверждено ручной UAT — **approved**.
- **Committed in:** `89413288`

---

**Total deviations:** 4 (1 Rule 1 компиляционный багфикс + 2 UAT-выявленных архитектурных/поведенческих правки по прямому запросу пользователя + 1 Rule 1 критическая рантайм-регрессия, пойманная только во втором раунде ручной UAT).
**Impact on plan:** Основная архитектура плана (полная конверсия, атомарная транзакция) не менялась. UI-флоу downgrade-подтверждения был пересмотрен по итогам живой UAT-проверки — итоговое поведение точнее соответствует пользовательскому ожиданию, чем исходный inline-вариант из плана.

## Issues Encountered

**Компиляционные гейты не ловят рантайм-ошибки реактивности Svelte 5.** `svelte-check`, `eslint`, `prettier` и `vite build` после каждого раунда правок отчитывались чисто (0 ошибок, тот же набор из 50 предсуществующих warning'ов) — но регрессия `effect_update_depth_exceeded` из дефекта 4 полностью ломала открытие ЛЮБОГО модала в приложении и была обнаружена только во втором раунде ручной UAT-проверки в живом `cargo tauri dev`. Это тот же урок, что уже зафиксирован в проектной памяти («Synthetic harness not verification») — применительно конкретно к рантайм-логике `$effect`/`$state`: заявлять «всё зелено» по компиляционным гейтам для изменений в этой области нельзя, финальная проверка поведения — только в реально работающем UI (либо, как в этом случае, на временном живом харнессе, воспроизводящем конкретный сценарий, если полный прогон приложения недоступен исполнителю).

## Known Stubs

None.

## Threat Flags

None — конверсия типа устройства работает через уже существующее поле `type_id` в `DeviceNew`/`DevicePatch` (пересекает ту же границу доверия, что и раньше); синхронизация `printers` — внутренняя операция в той же транзакции, без нового сетевого surface. См. `<threat_model>` в PLAN.md (T-rdj-01..T-rdj-04) — все mitigate/accept дispositions подтверждены реализацией без отклонений.

## User Setup Required

None - конфигурация внешних сервисов не требуется.

## Next Phase Readiness

Функциональность завершена и самодостаточна, прошла 2 раунда ручной UAT-проверки (round 1 — 2 дефекта UX/данных, round 2 — 1 критическая рантайм-регрессия), итог — **approved**. Открытых блокеров для последующих фаз нет.

---
*Quick task: 260820-rdj-device-type-switch*
*Completed: 2026-08-20*

## Self-Check: PASSED

Все 8 файлов из списка Files Created/Modified присутствуют на диске; все 5 хэшей коммитов задачи (`c849322a`, `9ff13d40`, `781d98de`, `c66e6bea`, `89413288`) присутствуют в `git log`.
