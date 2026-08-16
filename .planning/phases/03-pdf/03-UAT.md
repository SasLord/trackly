---
status: complete
phase: 03-pdf
source:
  - 03-VERIFICATION.md (human_verification block)
started: 2026-05-30T21:25:00Z
status: complete
updated: 2026-05-30T22:10:00Z
---

## Current Test

[testing complete]

## Tests

### 1. DEV-14 flow на запущенном Tauri-приложении
expected: Правый клик на устройство в DevicesPage → «Печать документа приёма» → DocumentAcceptanceModal с полями «Кто передал/Кто принял/Дата» → submit → PdfPreviewModal с реальным PDF (кириллица читаема, ФИО видны, дата правильная)
result: issue
reported: "yes, но есть замечание: Диалоговое окно DocumentAcceptanceModal закрывается на mouse up вне окна. Если выделять текст в окне и с зажатой кнопкой мыши выйти за границу окна и отпустить кнопку, окно закроется — это не правильно!"
severity: major
note: "DEV-14 happy path работает; обнаружен cross-cutting bug в shared Modal.svelte — backdrop-dismiss срабатывает на mouseup вне модала, даже если mousedown был внутри (text selection drag разрушает форму)"

### 2. PDF рендеринг handover-акта с шапкой и кириллицей
expected: Открыть существующий handover-акт → «Печать» → видна шапка с реквизитами организации (название, ИНН, КПП, адрес), таблица позиций с кириллическими наименованиями, подписи Сдал/Принял с правильными ФИО включая «ё»/«-» в составе
result: issue
reported: |
  yes (Cyrillic, ё, дефис в фамилии — читаемы; шапка с placeholder org-data
  отображается; ФИО Сдал/Принял видны). НО есть 7 замечаний — 5 по модалам
  создания/возврата и 2 по PDF (column overlap + отсутствие save/print/open).
  Screenshot: визуально текст "Сидоров-Петроградский Иван Александрович"
  переползает через колонки Инв.№/Серийный №/Модель; "(ё) №42" накладывается
  на "Кол-во".
severity: blocker
note: |
  Test 2 surface area вскрыл 7 cross-cutting issues. Перечислены в Gaps как
  отдельные truth-entries (G-2 .. G-8). Главные blocker'ы:
  • G-7: partial return → суффикс «в» + полный archive (regression ACT-07+ACT-09)
  • G-8: PDF column overlap + нет save/print/system-open в PdfPreviewModal
  • G-3: quantity в handover/return модалах не ограничен наличием на складе

### 3. Визуальная проверка логотипа в шапке PDF (ACT-11 closure)
expected: Настроить org с реальным logo.png (или JPG) в exe_dir → render handover-PDF → в правом-верхнем углу шапки виден логотип ≤80×40pt с сохранённым aspect-ratio. Тест с битым/несуществующим logo_path — PDF рендерится без логотипа без ошибки.
result: issue
reported: "yes, но логотип немного сжат по ширине."
severity: minor
note: "Логотип отображается на корректной позиции (правый-верхний угол), правильного размера, читаем. НО aspect-ratio не идеально сохранён — наблюдается сжатие по ширине vs. оригинал. Баг в draw_logo_top_right (renderer.rs:325) — расчёт scale_fit в bbox 80×40 не учитывает реальный aspect оригинала."

### 4. Full lifecycle (ACT-06..10) через UI
expected: Создать handover → partial return через ReturnModal (галочка «Применить ко всем» по умолчанию ВКЛ) → full return → handover автоматически в архив (вкладка «Архив», счётчик +1) → удалить return → handover возвращается в активные → удалить handover → все devices снова «На складе»
result: issue
reported: |
  При частичном возврате не добавляется цифра ("в1", а просто "в"). Когда
  делаешь второй возврат, то предлагает вернуть то, что уже возвращалось!
  Надо учитывать возвраты и давать возможность вернуть только то, что он
  брал с учётом Акта и уже выполненых Возвратов. Когда я, например, по
  акту отдал 3 мышки с состоянием Новое, потом оформляю возврат 2 мышек,
  то возврат считается полным и отправляется в Архив. Новое Состояние
  применяется только к одной мышке! Наведи порядок с учётом колличества и
  правильновсти возвратов с учётом возвращаемого количества.
severity: blocker
note: |
  Корневая причина — quantity-unaware accounting в acts lifecycle. Backend
  model работает корректно ТОЛЬКО при quantity=1 per act_item (фикстуры
  всех integration-тестов используют quantity=1, потому baga не вскрыта).
  При quantity > 1:
  • G-7 подтверждён: partial → суффикс «в», parent → Архив.
  • G-10: ReturnModal не учитывает already_returned — повторно предлагает
    вернуть уже-возвращённые позиции (UI showing handover_qty, не
    handover_qty - already_returned).
  • G-11: recompute_parent_archived считает по distinct device_id, а не по
    SUM(quantity) — handover c 1 строкой qty=3 уходит в Архив после
    возврата с qty=2.
  • G-12: condition/location update применяется к 1 device row на act_item,
    игнорируя quantity > 1 (нужно либо clone N-1 device rows, либо
    redesign модели «quantity на act_item ≠ N independent devices»).

### 5. Поиск по актам (ACT-04) через UI
expected: В ActsSearchAndTabs ввести часть номера акта / ФИО / наименования устройства → через ~250ms список фильтруется; backend acts.search вызывается с debounce
result: pass

### 6. ACT-13 cross-tx race на запущенной системе
expected: Открыть два окна с одним handover-актом → в одном оформить полный возврат (success) → во втором (с устаревшим cache) попытаться оформить возврат тех же позиций → backend отвергает с понятным сообщением (HTTP 409 Conflict / AppError::Conflict «уже не в работе»); UI показывает toast/alert, не «тихое сохранение»
result: blocked
blocked_by: prior-phase
reason: |
  User reported: При открытии второго окна по адресу 'http://localhost:1420/'
  появляется тоаст: «Не удалось связаться с приложением. Попробуйте
  перезапустить.» и не видит БД.
  Архитектурно: cross-tx race невозможно воспроизвести в Phase 3 — backend
  доступен только из Tauri webview, axum HTTP API не bind'ится (отложено на
  Phase 8 LAN/server-mode), tauri-plugin-single-instance блокирует второй
  Tauri-экземпляр. Тест нужно прогнать после Phase 8.
note: |
  Заодно зафиксирован UX-баг (G-13) — toast «Попробуйте перезапустить»
  misleading: перезапуск не поможет, потому что browser-runtime в Phase 3
  не имеет ни Tauri invoke, ни HTTP API. Сообщение должно явно говорить:
  «Браузерный доступ будет доступен в Phase 8; пока используйте desktop-окно».

## Summary

total: 6
passed: 1
issues: 4
pending: 0
skipped: 0
blocked: 1

## Gaps

- id: G-1
  truth: "DocumentAcceptanceModal (и shared Modal.svelte) не должен закрываться на mouseup-вне-модала, если mousedown произошёл внутри модала — это типичный паттерн text selection drag, который сейчас выкидывает пользователя из формы"
  status: failed
  reason: "User reported: yes, но есть замечание: Диалоговое окно DocumentAcceptanceModal закрывается на mouse up вне окна. Если выделять текст в окне и с зажатой кнопкой мыши выйти за границу окна и отпустить кнопку, окно закроется — это не правильно!"
  severity: major
  test: 1
  scope: "cross-cutting — затрагивает ВСЕ модалы приложения (общий Modal.svelte / backdrop click handler)"
  fix_pattern: "разделить mousedown/mouseup: backdrop dismiss срабатывает только если ОБА события произошли на backdrop. Альтернатива — слушать click (composed mousedown+mouseup на одном элементе) вместо mouseup."
  artifacts: []
  missing: []

- id: G-2
  truth: "ActFormModal: поле «Когда отдали» (handover date) обязательно при создании акта, default = сегодня, редактируемое; «Дата» и «Сроком до» — DatePicker-компонент, а не plain text YYYY-MM-DD input"
  status: failed
  reason: "User reported: При создании акта должно быть поле с датой Когда отдали, по умолчанию сегодняшний день и возможностью изменить. Дата (когда) и Сроком до - не просто текстовое окно для записи формата YYYY-MM-DD, а с DatePicker'ом."
  severity: major
  test: 2
  scope: "ActFormModal.svelte — все date-инпуты в форме создания акта"
  fix_pattern: "Заменить <input type='text'> на DatePicker-компонент (нужен ui/src/lib/components/DatePicker.svelte — потенциально новый shared component). Добавить поле handover_date в ActService.create payload + DB acts.handover_date_utc или вычислить из first audit_log entry."
  artifacts: []
  missing: []

- id: G-3
  truth: "ActFormModal: «Кол-во» позиции при handover должно быть ограничено фактическим количеством устройства на складе (status='на_складе'); особенно если устройство уникально (qty=1) — нельзя выбрать «Кол-во: 5»"
  status: failed
  reason: "User reported: Устройству можно указать любое количество - это не правильно, особенно если устрройство всего одно на складе."
  severity: major
  test: 2
  scope: "ActFormModal.svelte qty input + backend ActService::create validation"
  fix_pattern: "Backend: добавить cross-check в validate_handover_create — для каждого item.device_id вычислить stock_qty = devices.quantity (или count по status='на_складе') и assert item.quantity <= stock_qty → AppError::Validation. UI: max attribute на qty input."
  artifacts: []
  missing: []

- id: G-4
  truth: "DeviceAutocompleteField: после выбора устройства из dropdown автокомплит должен закрыться, а не показываться снова до повторного клика"
  status: failed
  reason: "User reported: Когда в списке устройств вводишь наименование устройства и выбираешь нужное через автокомплит, почему-то автокомплит снова показывается, пока не выберешь его снова. Такая проблема уже была при создании автокомплита в добавлении устройств."
  severity: major
  test: 2
  scope: "ui/src/features/devices/DeviceAutocompleteField.svelte — regression: тот же баг уже был в device-add flow"
  fix_pattern: "После onSelect: явно установить showDropdown=false + blur input ИЛИ переключить in-progress флаг чтобы $effect / debounce не пересоздавал список. Покрыть тестом svelte/testing-library — input.fill + select item → dropdown not visible."
  artifacts: []
  missing: []

- id: G-5
  truth: "DocumentAcceptanceModal: поля «Кто сдал» и «Кто принял» должны иметь автокомплит (из ранее введённых имён / справочника людей)"
  status: failed
  reason: "User reported: При создании нового акта и Печати документа приёма у полей Кто сдал и Кто принял - должен быть автокомплит."
  severity: minor
  test: 2
  scope: "DocumentAcceptanceModal.svelte + ActFormModal.svelte signature fields"
  fix_pattern: "Новый shared component PersonAutocomplete.svelte — источник: SELECT DISTINCT giver / receiver FROM acts WHERE … ORDER BY frequency DESC LIMIT 20. Backend tauri command: acts.suggest_person(field, prefix). Phase 3 deferred — может уйти в Phase 5 (Auth) или follow-up."
  artifacts: []
  missing: []

- id: G-6
  truth: "ReturnModal: поля «Кто сдал» и «Кто принял» должны быть; default — swap из handover (тот кто сдавал → теперь принимает; кто брал → теперь сдаёт); с автокомплитом; редактируемые. При снятии «Применить ко всем» — поле «Состояние» блокируется неверно (а «Расположение на складе» — нет); inconsistency. Поле «Состояние» должно быть с автокомплитом (из справочника condition values)."
  status: failed
  reason: "User reported: При возврате: должны быть поля Кто сдал и Кто принял с автокомплитом и по умолчанию должны быть заполненые значениями из акта (тот кто сдавал, теперь принимает и кто брал, теперь возвращает) с возможностью изменения значений. Также при снятии галочки с «Применить ко всем (по умолчанию)», Состояние - блокируется для ввода, а Расположение на складе - не блокируется. Опять же, в Кол-во к возврату - можно указать любое значение, даже больше чем отдавалось по кату. Поле Состояние тоже должно быть с автокомплитом."
  severity: major
  test: 2
  scope: "ReturnModal.svelte + ReturnItemsTable.svelte + backend ReturnPayload schema"
  fix_pattern: "1) Добавить giver_name/receiver_name в ReturnPayload + ReturnModal; default swap значений из parent handover (через acts.get(parent_act_id)). 2) Fix inconsistency: при apply_to_all=false — оба поля (condition И location) должны быть разблокированы для per-row override (или оба заблокированы — спросить дизайн-решение). 3) Backend CR-04 уже добавил quantity-bound assertion — UI должен показывать max и подсвечивать invalid input до отправки. 4) ConditionAutocomplete component (из devices.conditions enum или DISTINCT по devices.condition)."
  artifacts: []
  missing: []

- id: G-7
  truth: "ACT-07 + ACT-09 регрессия: при ЧАСТИЧНОМ возврате (не все позиции вернулись) display-rule даёт суффикс «в» (формат full-return) И handover ушёл в Архив (поведение full-return). Должно быть: суффикс «в1» (sub_number=1), parent.archived=0."
  status: failed
  reason: "User reported: При частичном возврате, возврат оформился просто с окончанием 'в' и Акт ушёл полностью в Архив (поведение для ПОЛНОГО, а не частичного возврата)."
  severity: blocker
  test: 2
  scope: "act_service.rs::do_return → recompute_parent_archived + act_service.rs::format_act_number (display rule)"
  fix_pattern: |
    Двойная проверка нужна с реальным фикстурным сценарием:
    1) Воспроизвести: handover с 2+ устройствами → return 1 устройства (≠ все). Ожидание: parent.archived=0 + display «42в1».
    2) Если воспроизводится — баг в recompute_parent_archived (SQL не считает оставшиеся в работе) ИЛИ в format_act_number (логика "это единственный return AND закрыл все позиции" слишком жадная).
    3) Если НЕ воспроизводится в фикстуре — спросить у пользователя точный сценарий (возможно handover был с одним устройством, и user воспринимает return одного устройства как partial).
    Существующие тесты acts_returns::partial_return_keeps_handover_active и acts_display_rule покрывают это; их зелёный статус говорит что либо backend OK и баг в UI, либо тестовый сценарий не покрывает реальный путь user.
  artifacts:
    - "crates/trackly-app/src/services/act_service.rs (do_return + recompute_parent_archived)"
    - "crates/trackly-infra/src/repos/acts_sqlite.rs::recompute_parent_archived"
  missing:
    - "Воспроизводимый user-сценарий (количество устройств в handover, какие именно возвращены)"
    - "Возможно integration-test, точно повторяющий пользовательский путь"

- id: G-13
  truth: "Toast «Не удалось связаться с приложением. Попробуйте перезапустить» — misleading, когда пользователь открывает http://localhost:1420 в обычном браузере. Перезапуск не поможет: в Phase 3 backend доступен только через Tauri invoke; axum HTTP API не bind'ится (отложено на Phase 8). Сообщение должно honest указывать architectural ограничение."
  status: failed
  reason: "User reported (test 6 attempt): При открытии второго окна по адресу 'http://localhost:1420/' появляется тоаст: «Не удалось связаться с приложением. Попробуйте перезапустить.» и не видит БД."
  severity: minor
  test: 6
  scope: "ui/src/lib/api/* (transport detection) + toast/error UI"
  fix_pattern: |
    1) Detect runtime в transport layer: если window.__TAURI__ undefined →
       runtime = 'browser'; otherwise → 'tauri'.
    2) Browser runtime в Phase 3: показывать info-banner (не error toast)
       «Этот режим работает только в desktop-приложении Trackly. Веб-доступ
       будет добавлен в Phase 8.»
    3) После Phase 8 — переключение на HTTP-API.
  artifacts:
    - "ui/src/lib/api/* (apiCall / transport detection)"
    - "ui/src/lib/components/Toast / error component"
  missing:
    - "Решение по first-run UX в browser mode (Phase 8 scope)"

- id: G-10
  truth: "ReturnModal должен показывать к возврату только handover_qty - already_returned для каждой позиции (вычислено per (parent_act_id, device_id)). Сейчас показывает полное handover_qty — после первого возврата 2 из 3 в повторном Возврате снова предлагает все 3, а должен предлагать 1."
  status: failed
  reason: "User reported (test 4): Когда делаешь второй возврат, то предлагает вернуть то, что уже возвращалось! Надо учитывать возвраты и давать возможность вернуть только то, что он брал с учётом Акта и уже выполненых Возвратов."
  severity: blocker
  test: 4
  scope: "ui/src/features/acts/ReturnModal.svelte + ReturnItemsTable.svelte (data source) + потенциально acts.get / ActService для добавления already_returned в ActItemDto"
  fix_pattern: |
    Backend: ActService::get(act_id) — для каждого item добавить computed поле
    already_returned_qty = SELECT COALESCE(SUM(rai.quantity), 0) FROM act_items rai
    JOIN acts ra ON ra.id = rai.act_id WHERE ra.parent_act_id = ?1 AND
    rai.device_id = ?2 AND ra.deleted_at_utc IS NULL.
    (Тот же SQL, что в CR-04 quantity-bound check.)
    DTO ActItemDto: добавить already_returned_qty: i64.
    UI ReturnModal: для каждой строки показывать max=handover_qty-already_returned;
    если remaining=0 — строка disabled / скрыта.
  artifacts:
    - "crates/trackly-app/src/services/act_service.rs::get (или load_items)"
    - "crates/trackly-app/src/dto/act.rs::ActItemDto"
    - "ui/src/features/acts/ReturnModal.svelte + ReturnItemsTable.svelte"
  missing:
    - "Integration test: handover qty=3 → return qty=2 → re-open ReturnModal → assert max=1"

- id: G-11
  truth: "recompute_parent_archived должен считать остаток в работе по SUM(act_items.quantity) - SUM(return_items.quantity), а не по count(distinct device_id). Сейчас handover c одной строкой qty=3 уходит в Архив после возврата qty=2."
  status: failed
  reason: "User reported (test 4): по акту отдал 3 мышки … оформляю возврат 2 мышек, то возврат считается полным и отправляется в Архив."
  severity: blocker
  test: 4
  scope: "crates/trackly-infra/src/repos/acts_sqlite.rs::recompute_parent_archived"
  fix_pattern: |
    Текущий SQL (D-Archive-01 из CONTEXT.md):
      SELECT COALESCE(SUM(ai.quantity), 0) FROM act_items ai
      JOIN devices d ON d.id = ai.device_id
      WHERE ai.act_id = ? AND d.status_id = (SELECT id FROM device_statuses WHERE code='в_работе')
    Проблема: JOIN на devices.status='в_работе' — после возврата 2 из 3
    устройство имеет ОДИН status_id (на_складе после full update), потому
    JOIN не находит → SUM=0 → archived=1.
    Правильная формула должна быть quantity-based, независимо от device.status:
      handover_total = SUM(ai.quantity) FROM act_items WHERE act_id = ?
      returned_total = SUM(rai.quantity) FROM act_items rai
                       JOIN acts ra ON ra.id = rai.act_id
                       WHERE ra.parent_act_id = ? AND ra.deleted_at_utc IS NULL
      archived = (handover_total - returned_total) = 0
    Связано с G-12 — модель «один device_id с qty > 1» противоречит
    «devices.status_id это per-device-row». Нужно либо: (a) рассматривать
    devices как количественные позиции (devices.quantity column), (b) при
    handover clone device row N раз. Архитектурное решение требуется до
    исправления G-11.
  artifacts:
    - "crates/trackly-infra/src/repos/acts_sqlite.rs::recompute_parent_archived"
    - "crates/trackly-app/src/services/act_service.rs::do_return"
  missing:
    - "Решение по quantity-модели (G-12)"
    - "Integration test: handover qty=3 + 1 device → partial return qty=2 → assert parent.archived=0"

- id: G-12
  truth: "Модель «act_item.quantity > 1 представляет N штук одного device» несовместима с «devices.status_id это per-row». При возврате 2 из 3 — condition/location меняется ТОЛЬКО на 1 device row (а должно — на 2 виртуальных «копий»)."
  status: failed
  reason: "User reported (test 4): Новое Состояние применяется только к одной мышке!"
  severity: blocker
  test: 4
  scope: "Архитектурный — затрагивает devices schema, acts schema, ActService, DeviceService, ReturnModal data flow"
  fix_pattern: |
    Два candidate решения — нужен ARCHITECTURE DECISION:

    Решение A: «devices.quantity column»
    + devices.quantity INTEGER DEFAULT 1 (для расходников / non-unique)
    + act_items.quantity stays
    + return_items.quantity stays
    + condition/location update применяется к одной device row, qty считается
      как агрегат
    - Теряется per-unit tracking (если 2 мышки из 3 сломаны, а 1 ОК — модель не
      различит)

    Решение B: «clone-on-handover»
    + При handover с qty=3 — создаём 3 device rows с независимыми status_id
    + Каждый return обновляет конкретные device rows (выбор UI per-row)
    + ACT-13 quantity-bound становится «count check» а не «SUM check»
    - Размножение rows; usability для расходников (картриджи) ухудшается
    - Требует UI «выбрать какие именно из 3 мышек вернуть»

    Решение C: «гибрид — devices.is_unique flag»
    + Уникальное устройство (компьютер, принтер) — quantity всегда 1, qty>1
      запрещён в UI
    + Расходник (мышка, картридж) — quantity > 1 разрешён, но условие/location
      едины для всей позиции (упрощение бизнес-логики)
    + В UI handover/return для non-unique — выбор «сколько штук вернуть» без
      per-unit attributes
  artifacts:
    - "crates/trackly-core/src/domain/devices.rs"
    - "crates/trackly-core/src/domain/acts.rs"
    - "crates/trackly-app/src/services/act_service.rs"
    - "ui/src/features/acts/ReturnModal.svelte"
  missing:
    - "ARCHITECTURE DECISION: A | B | C (требует discuss-phase или ADR)"
    - "Возможно отдельная Phase 3.5 или ингредиент Phase 4 (картриджи — там quantity-aware будет критичен)"

- id: G-9
  truth: "draw_logo_top_right (renderer.rs:325) должен сохранять aspect-ratio оригинального изображения при scale-fit в bbox 80×40pt — сейчас наблюдается заметное сжатие по ширине логотипа организации vs. оригинал"
  status: failed
  reason: "User reported: yes, но логотип немного сжат по ширине."
  severity: minor
  test: 3
  scope: "crates/trackly-app/src/pdf/renderer.rs::draw_logo_top_right (helper added in commit b1298eb)"
  fix_pattern: |
    Проверить логику scale в draw_logo_top_right:
    ```
    let scale = (max_w / orig_w).min(max_h / orig_h);
    let final_w = orig_w * scale;
    let final_h = orig_h * scale;
    Size::from_wh(final_w, final_h)
    ```
    Если уже так — баг возможно в том, что Image::from_png не возвращает intrinsic dimensions, и сейчас используется hardcoded (80, 40) Size без расчёта. В этом случае: вытащить width/height из image data (PNG IHDR chunk parsing ИЛИ через image::ImageReader перед krilla wrap) и пересчитать scaled Size.
  artifacts:
    - "crates/trackly-app/src/pdf/renderer.rs:325-385 (draw_logo_top_right)"
  missing:
    - "Pixel-perfect сравнение rendered vs. оригинала (Visual regression test)"
    - "Возможно нужен unit test: load 200×100 PNG → assert final size = 80×40 (limited by width) с сохранённым 2:1 aspect"

- id: G-8
  truth: "PdfPreviewModal: PDF рендерится с column overlap (текст «Сидоров-Петроградский Иван Александрович» переползает через колонки Инв.№/Серийный №/Модель; «(ё) №42» накладывается на «Кол-во»). Также отсутствуют действия «Сохранить», «Печать», «Открыть в системном просмотрщике» — preview доступен только как просмотр."
  status: failed
  reason: "User reported: При формировании PDF некоторые значения столбиков со значениями залезают друг на друга. При просмотре ПДФ его не получается сохранить, отправить на печать и открыть в системном просмотре."
  severity: blocker
  test: 2
  scope: "1) pdf/renderer.rs — table layout (column widths не учитывают реальный max-width текста). 2) PdfPreviewModal.svelte — отсутствуют action buttons. 3) tauri-plugin-dialog (save) + tauri-plugin-shell (open) wiring."
  fix_pattern: |
    1) Renderer: column widths должны быть либо (а) fixed-pt с text-wrap внутри cell, либо (б) computed по max(measure_text_width(cell)) с min/max. Сейчас, видимо, абсолютные позиции — длинный текст просто рисуется поверх соседних колонок. Нужно: либо реализовать word-wrap внутри cell (DejaVu Sans + krilla text shaping), либо truncate с ellipsis.
    2) PdfPreviewModal: добавить три кнопки: «Сохранить как...» (tauri-plugin-dialog::save → fs::write), «Печать» (window.print() ИЛИ tauri-plugin-shell::open для системного PDF-просмотрщика), «Открыть» (tauri-plugin-shell::open temp-file).
    3) В browser-mode (LAN server в будущем) — fallback на iframe download attribute.
  artifacts:
    - "crates/trackly-app/src/pdf/renderer.rs (render_table_section)"
    - "ui/src/features/acts/PdfPreviewModal.svelte"
    - "Tauri plugin-dialog + plugin-shell capabilities config"
  missing:
    - "Решение column-overflow: word-wrap или ellipsis (UX decision)"
    - "Save filename pattern (например 'Акт_приёма-передачи_№42_2026-05-30.pdf')"
