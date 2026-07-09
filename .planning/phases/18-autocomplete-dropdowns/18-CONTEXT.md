# Phase 18: Автокомплит и дропдауны - Context

**Gathered:** 2026-07-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Все автокомплиты приложения выводят выпадающий список через portal в `body`, чтобы
он не обрезался и не ломал вёрстку внутри модальных окон. Дополнительно —
полноценный выбор устройства в форме акта (Акты → Позиции): открытие по фокусу,
рабочая фильтрация, группировка одинаковых устройств с раскрытием деталей экземпляра
и схлопывание единственной оставшейся группы в плоский список.

**В скоупе:** AUTO-01 (portal-дропдауны), AUTO-02 (focus-open пикера устройства),
AUTO-03 (фильтрация пикера), AUTO-04 (группы с drill-in по экземплярам),
AUTO-05 (единственная группа → плоский список).

**ВНЕ скоупа:** дата/редактирование актов (Phase 19), печать/организация (Phase 20),
коды картриджей (Phase 21). Никаких новых полей данных устройства — только UX выбора.
</domain>

<decisions>
## Implementation Decisions

### AUTO-01 — Portal-дропдауны
- **D-01:** Каждый автокомплит приложения выводит выпадающий список через portal в
  `body` (не обрезается overflow-контейнером модалки, не добавляет внутренний скролл,
  не искажает вёрстку диалога). Переиспользовать `ui/src/lib/utils/portal.ts`, добавив
  слой позиционирования (существующий use-action только перемещает узел, но НЕ
  позиционирует).
- **D-02:** Дропдаун якорится к инпуту (`getBoundingClientRect`) и **следует за ним**
  при скролле контейнера/окна и ресайзе (репозиция). Флип вверх у нижней кромки экрана.
  (Не «закрывать при скролле».)

### AUTO-02 / AUTO-03 — Focus-open + фильтрация пикера устройства
- **D-03:** Пикер устройства в `ActFormItemsTable` открывает список **сразу при фокусе**
  (без начала ввода). Переиспользовать канонический паттерн focus-open из
  `LocationAutocomplete.svelte` (`handleFocus` снимает suppress + fetch delay 0;
  ArrowDown при закрытом dropdown тоже триггерит открытие).
- **D-04:** На фокус показываются **top-20** групп, отсортированных **по остатку на
  складе** (count DESC). Ввод текста фильтрует по **наименованию + инвентарному № +
  серийному №** (сейчас фильтр только по `name_prefix` → требуется доработка
  backend-фильтра/сортировки `list_grouped`).

### AUTO-04 — Группировка с раскрытием экземпляров
- **D-05:** Группа верхнего уровня = **name + model** (model если есть).
  *(Уточняет более ранний ответ «группировать только по name» — итоговое решение:
  name+model.)*
- **D-06:** Раскрытие раскрываемой группы — **drill-in с кнопкой «← назад»**: клик по
  группе заменяет список её экземплярами; «назад» возвращает к списку групп.
- **D-07:** Представление внутри раскрытой группы (name+model):
  - серийные / инвентаризированные экземпляры — **отдельные строки**, показывают
    серийный № и/или инвентарный № (если есть) и состояние; выбор = qty 1;
  - несерийные И безынвентарные экземпляры — **подгруппируются по состоянию
    (condition)**, каждая подгруппа отображается одной строкой с ×count и вводом
    количества.
- **D-08:** Группа (name+model), состоящая **только** из несерийных/безынвентарных
  устройств, **не раскрывается**: клик по ней сразу выбирает устройство из группы с
  возможностью указать количество (текущая clone-семантика). При разных состояниях
  внутри — строки по состоянию (см. D-07).
- **Note (сохранить):** clone-on-handover qty-семантика (V015), hard-cap
  `MAX_CLONE_QTY = 1000`, ограничение qty доступным остатком (`stock_available`),
  и дедуп уже выбранных устройств (DEF-2A) — не регрессировать в новом пикере.

### AUTO-05 — Единственная группа → плоский список
- **D-09:** Если после фильтрации остаётся единственная группа — строка группы НЕ
  показывается; сразу выводится **плоский список экземпляров** этой группы (то же
  представление, что и внутри drill-in, но без кнопки «назад»).

### Связь с зафиксированным DEF-2B
AUTO-04 — это отложенный в Phase 03.2 «DEF-2B Вариант 1» (UI-раскрытие группы).
Настоящая фаза **уточняет** ключ группировки: верхний уровень становится
`name + model` (D-05), а `condition` из DEF-2B применяется как **подгруппировка внутри**
для несерийных/безынвентарных устройств (D-07). Планировщик/исследователь должны
согласовать это с текущим `list_grouped` (сейчас группирует по `name+model+condition`
при `group_by_condition:true`).

### Claude's Discretion
- Конкретный слой позиционирования portal-дропдауна (fixed-координаты + репозиция на
  scroll/resize + флип вверх) — деталь реализации.
- Как получить per-instance детали для drill-in: расширить `DeviceGroup` списком
  членов vs. дозагрузка устройств по `ids`.
- Точная сигнатура backend-фильтра для `name + инв.№ + SN` и сортировки по остатку.
- Единый переиспользуемый компонент дропдауна vs. точечные правки каждого автокомплита
  для AUTO-01.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Требования
- `.planning/REQUIREMENTS.md` — AUTO-01..05 (строки 14–18).

### История устройства-пикера и автокомплитов (прямые предшественники)
- `.planning/phases/03.2-deferred-uat-gap-closure/03.2-CONTEXT.md` — DEF-1
  (focus-open паттерн, реплицированный из LocationAutocomplete) и DEF-2B (grouping
  по name+model+condition). **AUTO-04 = отложенный там «Вариант 1» (раскрытие группы).**
- `.planning/phases/03.1-acts-quantity-model-uat-gap-closure/03.1-DEFERRED-UAT-ITEMS.md`
  — разбор вариантов DEF-2B (source of truth по grouping-семантике).
- `.planning/phases/03.3-device-list-ux-round-2-grouping-condition-column-cell-toolti/03.3-UI-SPEC.md`
  — дизайн флага `group_by_condition` (DevicesPage → false, ActFormItemsTable → true),
  подписи condition в группе.
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ui/src/lib/utils/portal.ts` — Svelte use-action, переносит узел в `body`. Уже
  применяется в `DeviceContextMenu`, `CartridgeContextMenu`, `PdfPreviewModal`.
  **Не позиционирует** — для AUTO-01 нужен слой anchoring (fixed-координаты +
  репозиция).
- `ui/src/lib/components/LocationAutocomplete.svelte` — канонический focus-open
  паттерн (`handleFocus`, ArrowDown-при-закрытом). Реплицировать для пикера устройства.

### Integration Points — пикер устройства (AUTO-02..05)
- `ui/src/features/acts/ActFormItemsTable.svelte` — целевой компонент. Сейчас:
  `Input` + `handleQueryInput` (нет `onfocus` → нет AUTO-02; `v.trim().length < 1`
  early-return; фильтрует через `devices.listGrouped(name_prefix, status_id:1,
  group_by_condition:true)`; `pickGroup` → repr + ×count). Дропдаун — `position:
  absolute` внутри `.col-device` → обрезается модалкой (AUTO-01). Требует переработки
  под drill-in (D-06/07/08) и portal (D-02).
- `devices.listGrouped(filter, pagination)` → `DeviceGroup { repr: DeviceDto; count:
  number; ids: number[] }` (`ui/src/bindings.ts:1549`). **Возвращает repr+count+ids,
  но НЕ детали каждого экземпляра** — для drill-in (SN/инв.№/состояние по каждому)
  нужны члены группы: расширить backend или дозагрузить по `ids`.
- Backend grouping: `crates/trackly-infra/src/repos/devices_sqlite.rs::list_grouped`
  + `crates/trackly-core/src/domain/devices.rs::DeviceGroupRow`. AUTO-03/04/05
  (фильтр name+инв.№+SN, сортировка по остатку, name+model верхний уровень) требуют
  правок здесь.

### Established Patterns — компоненты для portal (AUTO-01, «любой автокомплит»)
- Дропдауны с `position: absolute`, обрезаемые в модалках: `CartridgeSelect`,
  `GroupedPrinterSelect`, `PrinterSelect`, `LocationAutocomplete`, `PersonAutocomplete`,
  `Select`, `DeviceAutocompleteField`, `ActFormItemsTable`. Все — кандидаты на portal.
  Рассмотреть единый компонент/действие дропдауна, чтобы не дублировать anchoring.
</code_context>

<specifics>
## Specific Ideas

Дословная формулировка модели группы от пользователя (source для D-05..D-08):

> «Модель устройства должна быть в группе, сама группа должна состоять из наименования
> и модели (если есть). В строчках группы должно отображаться: серийные и инвентарные
> номера (если есть), состояние. Если нет серийного и инвентарного номера, то надо
> группировать по состоянию и отображать количество штук. Группа без инвентарного и
> серийного номера не должна раскрываться, а при нажатии по ней выбиралось устройство
> из этой группы с возможностью указать количество таких устройств.»
</specifics>

<deferred>
## Deferred Ideas

None — обсуждение осталось в границах фазы.
</deferred>

---

*Phase: 18-Автокомплит и дропдауны*
*Context gathered: 2026-07-09*
