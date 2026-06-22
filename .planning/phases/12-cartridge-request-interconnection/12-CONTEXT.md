# Phase 12: Взаимосвязь картриджной заявки - Context

**Gathered:** 2026-06-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Сделать установку картриджа из заявки «Замена картриджа» полнофункциональной и
взаимосвязанной. Сейчас `OperationModal` (ui/src/features/cartridges/OperationModal.svelte)
открывается из `RequestDetail` с `cartridge={null}` — выбора физического картриджа нет,
поэтому установка из заявки фактически нерабочая (`handleSubmit` выходит на
`if (!cartridge) return`). Фаза замыкает четыре связи:

1. **Заявка → инвентарь:** выбор физического картриджа из БД прямо в потоке установки.
2. **Заявка → принтер/расположение:** авто-подстановка Расположения из принтера заявки.
3. **Заявка → заявитель:** авто-подстановка «Кому отдал» из автора заявки.
4. **Установка → заявка:** запись установленного картриджа в `completedCartridgeId`
   заявки + отражение в истории.

**В scope:** request-centric установка картриджа (выбор картриджа, авто-локация,
авто-заявитель, запись связи). Старый cartridge-centric вход (меню картриджа →
«Установить») сохраняется без изменений.

**Не в scope (новые возможности → отдельные фазы):** массовая установка нескольких
картриджей по одной заявке; создание/заправка картриджа прямо из заявки; изменение
самого lifecycle заявок (статусы, переходы); изменение compatibility-матрицы моделей.

</domain>

<decisions>
## Implementation Decisions

### Выбор картриджа в потоке установки из заявки
- **D-01:** В `OperationModal` при `op='install'`, открытом из заявки, добавить
  селектор физического картриджа из БД. Список фильтруется: только статус
  «На складе» **И** заряд «устанавливаемый» = `Полный(1)` / `Частичный(2)` (для
  картриджей; фотобарабаны не относятся к этому потоку — заявка «Замена картриджа»).
- **D-02:** Список дополнительно фильтруется по совместимости с моделью из заявки —
  `request.cartridgeModelId` (поле уже есть в `RequestDto`). Показываем экземпляры
  именно запрошенной модели картриджа.
- **D-03:** После выбора картриджа форма установки работает как раньше (Дата / Кто
  выдал / Кому выдал / Расположение), но `cartridge` теперь приходит из селектора, а
  не из пропа. Submit вызывает `cartridges.transition({op:'install', cartridge_id, …})`.

### «Кому отдал» (given_to_name)
- **D-04:** Поле `Кому выдал` предзаполняется из автора заявки (`request.requesterName`),
  но остаётся **редактируемым** через существующий `PersonAutocomplete`. Заявку мог
  создать один человек, а картридж забрать другой — специалист может поправить.

### Авто-подстановка «Расположение»
- **D-05:** Поле `Расположение` предзаполняется из расположения принтера заявки
  (location устройства типа «Принтер», `request.printerDeviceId`), остаётся
  **редактируемым** через `LocationAutocomplete`. Если у принтера расположение пустое —
  поле остаётся пустым (обычный ручной ввод).

### Связь установленного картриджа с заявкой
- **D-06:** При завершении заявки после установки записывать выбранный картридж в
  `completedCartridgeId` заявки. Поток уже завершает заявку через
  `requests.transition({op:'complete', …, linkedCartridgeId})` — сейчас передаётся
  `null`, нужно передавать `id` установленного картриджа.
- **D-07:** Установленный картридж отражается в истории заявки (REQ-07 история) —
  человекочитаемо (код картриджа `C-000001` + модель), чтобы из карточки заявки было
  видно, что именно установили.

### Сосуществование двух входов установки
- **D-08:** Сохранить **оба** входа установки картриджа: новый request-centric (из
  `RequestDetail`, с выбором картриджа) и старый cartridge-centric (меню картриджа
  «На складе» → «Установить в принтер», `cartridge` уже выбран). Старый вход не
  меняется и служит fallback'ом, когда установка идёт вне заявки.

### Claude's Discretion (граничные случаи — разумные дефолты)
- **DISC-01:** Если `request.cartridgeModelId` = `null` (заявка без выбранной модели) —
  fallback: показать все картриджи «На складе» с устанавливаемым зарядом, без фильтра
  совместимости. Реализационная деталь, оставлена планировщику.
- **DISC-02:** Если совместимых картриджей на складе нет — показать понятное
  пустое состояние («Нет подходящих картриджей на складе»); специалист может
  использовать старый cartridge-centric вход (D-08) или отклонить заявку. Без блокировки.
- **DISC-03:** Точная форма селектора (выпадающий список / поиск по коду+модели) и где
  именно он рендерится в `OperationModal` — на усмотрение планировщика, по образцу
  существующих автокомплитов/Select.
- **DISC-04:** `requesterName` иногда может быть логином/AD-учёткой, а не ФИО —
  редактируемое поле (D-04) это покрывает, отдельной логики не требуется.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Поток установки картриджа (точка изменений)
- `ui/src/features/cartridges/OperationModal.svelte` — параметризованная модалка 5
  lifecycle-операций; ветка `op='install'`, `buildPayload()`, `validate()`,
  `preFillPrinterId`-хинт (REQ-05). Сюда добавляется выбор картриджа + авто-подстановки.
- `ui/src/features/requests/RequestDetail.svelte` — вход в установку из заявки
  (`operationModalOpen`, `handleInstallSuccess`, передача `preFillPrinterId`,
  `complete` с `linkedCartridgeId: null`). Здесь меняется проброс данных заявки.
- `ui/src/features/cartridges/api.ts` — клиент `cartridges.transition` и списки
  картриджей; нужен фильтр «на складе + устанавливаемый заряд + модель».
- `ui/src/features/requests/api.ts` — клиент `requests.transition` (complete с
  `linkedCartridgeId`).

### Backend (контракты и сервисы)
- `crates/trackly-app/src/dto/request.rs` — `RequestDto` (`cartridgeModelId`,
  `printerDeviceId`, `completedCartridgeId`, `requesterName`, `printerName`),
  `RequestTransitionPayload`/complete с `linkedCartridgeId`.
- `crates/trackly-app/` cartridge service — `CartridgeService` (read/transition,
  list по фильтру статуса/заряда/модели) и `CartridgeTransitionPayload` (`op='install'`).
- `crates/trackly-app/` request service — `RequestService` complete-переход,
  запись `completed_cartridge_id`, история заявки (REQ-07).
- `ui/src/bindings.ts` / `ui/src/bindings-phase6.ts` — генерируемые типы; при
  изменении DTO/команд регенерировать через `tauri-specta`.

### Доменные решения и предыстория
- `.planning/ROADMAP.md` §«Phase 6» (REQ-05: «можно сразу запустить операцию установки
  картриджа из контекста заявки») и §«Phase 12».
- `.planning/phases/06-*/06-CONTEXT.md` — исходные решения по заявкам и REQ-05 link
  (если присутствует; контекст cartridge_replace ↔ принтер ↔ модель).
- `.planning/phases/04-*/04-CONTEXT.md` — решения по lifecycle картриджей, статусам
  заряда (Полный(1)/Частичный(2)/Пустой(3)) и контекстным операциям.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `PersonAutocomplete.svelte` — уже используется для «Кому выдал»; предзаполнение
  значением `requesterName` через `bind:value`.
- `LocationAutocomplete.svelte` — уже в форме установки; предзаполнение location принтера.
- `CartridgeListRow`/`CartridgesList`/`Select` — образцы рендера списка картриджей для
  нового селектора.
- `cartridges.transition({op:'install', …})` + `requests.transition({op:'complete', …,
  linkedCartridgeId})` — оба контракта уже существуют; новые поля не требуются для
  записи связи (нужно лишь передать реальный id вместо `null`).

### Established Patterns
- Single-writer + optimistic version: операции картриджа и заявки несут `version`;
  установка → транзакция картриджа, затем complete заявки. Сохранять порядок и
  обработку конфликтов версий.
- Фильтрация картриджей по статусу/заряду уже реализована (свитч-бар «На складе»);
  переиспользовать существующий фильтр на стороне backend, не плодить новый SQL.
- DTO определяются один раз, биндинги генерируются `tauri-specta` (правка типов →
  регенерация `bindings*.ts`).

### Integration Points
- `RequestDetail` → `OperationModal`: расширить проброс (модель из заявки,
  расположение принтера, имя заявителя) вместо одного `preFillPrinterId`.
- `OperationModal.handleSubmit` success → `RequestDetail.handleInstallSuccess`:
  прокинуть id установленного картриджа, чтобы complete передал `linkedCartridgeId`.
- Расположение принтера: получить location устройства по `request.printerDeviceId`
  (devices type=Принтер) — уточнить, отдаётся ли оно уже в `RequestDto`/`printerName`
  или нужен дополнительный read.

</code_context>

<specifics>
## Specific Ideas

- Устанавливаемый заряд = строго `Полный(1)` и `Частичный(2)` (заправленные/частично
  заправленные на складе). `Пустой(3)` и списанные — не предлагать.
- Связь, которую пользователь хочет видеть замкнутой: Заявка ↔ инвентарь картриджей ↔
  принтер/расположение ↔ заявитель.

</specifics>

<deferred>
## Deferred Ideas

- Массовая установка нескольких картриджей по одной заявке — отдельная фаза при
  появлении потребности.
- Создание/отправка на заправку картриджа прямо из потока заявки — вне scope.
- Изменение lifecycle/статусов самих заявок — вне scope.

</deferred>

---

*Phase: 12-cartridge-request-interconnection*
*Context gathered: 2026-06-22*
