---
status: diagnosed
trigger: "Операция «Возвращён на склад» с незаполненным полем «Место (предыдущий картридж)» не меняет место картриджа, поэтому перемещение не фиксируется; подпись поля вводит пользователя в заблуждение."
created: 2026-09-03T00:00:00Z
updated: 2026-09-03T00:00:00Z
---

## Current Focus

hypothesis: "Пустое поле «Место (предыдущий картридж)» уходит на сервер как place_id=NULL, ветка авто-возврата безусловно ПЕРЕЗАПИСЫВАЕТ место картриджа этим NULL (не сохраняет прежнее), а гард D-06 (место → NULL) пропускает запись перемещения. Поле не валидируется, в отличие от такого же поля прямой операции «Возврат на склад»."
test: "Чтение transition_in_tx + validate() в OperationModal + дифференциальный подсчёт по audit_log dev-БД: у прямых возвратов на текущем коде place_id всегда заполнен, у авто-возвратов — нет."
expecting: "Подтверждено: resolved_place_id = *previous_cartridge_place_id (без fallback), UPDATE ... place_id=?2; validate() не проверяет previousCartridgePlaceId."
next_action: "Диагностика завершена — вернуть ROOT CAUSE FOUND оркестратору (goal: find_root_cause_only, фикс не применяем)."

reasoning_checkpoint:
  hypothesis: "Пустое поле previous_cartridge_place_id записывается в cartridges.place_id как NULL; is_reportable_place_change(Some(x), None) = false → перемещение не пишется. Место не «остаётся прежним», оно СТИРАЕТСЯ."
  confirming_evidence:
    - "cartridges_sqlite.rs:614 `let resolved_place_id = *previous_cartridge_place_id;` — прямое присваивание Option без unwrap_or/fallback, в отличие от соседнего resolved_state_id, у которого fallback есть (unwrap_or_else, kind-aware)."
    - "cartridges_sqlite.rs:645-655 UPDATE ... place_id=?2 с resolved_place_id — безусловная перезапись."
    - "place_movements.rs:100 is_reportable_place_change + place_movements_sqlite.rs:133 ранний return Ok(()) — место → NULL не пишется (D-06)."
    - "dev-БД: audit_log id=557 (cartridge 13, auto-return) before place_id=9, payload place_id=null; в place_movements для entity 13 между 1788369839 и 1788370002 записи нет."
    - "OperationModal.svelte:566-570 требует placeId для op='return_to_stock', но previousCartridgePlaceId в validate() не упомянут вообще."
  falsification_test: "Если бы место сохранялось (а не стиралось), у cartridge 13 после auto-return place_id остался бы 9 и следующая операция стартовала бы с 9. Фактически следующая операция (to_refill) стартовала с 4 — значения, проставленного промежуточным ручным `update` из NULL."
  fix_rationale: "n/a — режим find_root_cause_only, фикс не применяется. Требуется продуктовое решение (см. Resolution.fix)."
  blind_spots:
    - "Не воспроизводил инцидент вживую в UI — вывод собран из кода + аудита dev-БД."
    - "Не проверял HTTP/LAN-транспорт отдельно: серверная валидация previous_cartridge_place_id отсутствует в обоих (валидация только клиентская), но LAN-путь глазами не гонял."

## Symptoms

expected: |
  При замене картриджа в принтере снимаемый картридж возвращается на склад, его место
  становится складским, и это фиксируется записью в истории перемещений.
actual: |
  UAT фазы 40, тест 16: одна операция «Возвращён на склад» не породила записи перемещения.
  Пользователь СОЗНАТЕЛЬНО не заполнил поле «Место (предыдущий картридж)», ожидая
  автоподстановки прежнего складского места. Контроль: другая операция «Возвращён на склад»
  у того же картриджа место поменяла и перемещение дала — путь зависит от заполненности поля.
errors: None reported
reproduction: Test 16 в .planning/phases/40-movement-history/40-UAT.md
started: Обнаружено во время UAT фазы 40

## Eliminated

- hypothesis: "Место картриджа осталось прежним (местом принтера), т.е. UPDATE не тронул place_id"
  evidence: |
    Реконструкция в 40-UAT.md::analysis неточна. UPDATE в ветке авто-возврата ВСЕГДА пишет
    place_id = resolved_place_id, т.е. при пустом поле ставит NULL. Подтверждено аудитом:
    audit_log id=557 (cartridge 13) — before place_id=9, payload place_id=null; далее
    ручной `update` (id=559) выставил место заново. Симптом «перемещение не записалось»
    объясняется не «место не изменилось», а «место стёрто в NULL» — гард D-06 одинаково
    молчит в обоих случаях, поэтому со стороны UI отличить нельзя.
  timestamp: 2026-09-03

- hypothesis: "Дефект в журнале перемещений / гарде is_reportable_place_change"
  evidence: |
    Гард отработал строго по D-06 (место → NULL не пишется). Дефект выше по стеку —
    в самой операции авто-возврата и в отсутствии валидации поля.
  timestamp: 2026-09-03

- hypothesis: "Сбой воспроизводится случайно / зависит от таймингов"
  evidence: |
    Дифференциальный подсчёт по dev-БД показал детерминированное расщепление по типу
    операции, а не случайность (см. Evidence #6).
  timestamp: 2026-09-03

## Evidence

- timestamp: 2026-09-03
  checked: crates/trackly-infra/src/repos/cartridges_sqlite.rs:601-680 (ветка авто-возврата)
  found: |
    `let resolved_place_id = *previous_cartridge_place_id;` (стр. 614) — Option копируется
    как есть, без fallback. Соседнее поле состояния fallback ИМЕЕТ:
    `resolved_state_id = previous_cartridge_state_id.unwrap_or_else(|| if kind==2 {5} else {3})`.
    Далее `UPDATE cartridges SET status_id=1, state_id=?1, place_id=?2, holder_name=NULL,
    current_printer_device_id=NULL ...` с resolved_place_id.
  implication: |
    При пустом поле место снимаемого картриджа стирается в NULL. Асимметрия внутри одной
    ветки: у состояния продуманный дефолт, у места — нет.

- timestamp: 2026-09-03
  checked: crates/trackly-infra/src/repos/cartridges_sqlite.rs:507-513 (основная операция)
  found: |
    `CartridgeTransitionOp::ReturnToStock { state_id, place_id, .. } => (Some(*state_id), *place_id, None)`
    — прямой возврат ведёт себя так же: place_id=None → NULL.
  implication: Поведение «пусто = стереть место» общее для всех 5 операций картриджа, не только для авто-возврата.

- timestamp: 2026-09-03
  checked: crates/trackly-core/src/domain/place_movements.rs:100 + place_movements_sqlite.rs:133
  found: |
    `if !is_reportable_place_change(before, after) { return Ok(()); }`; юнит-тесты фиксируют
    is_reportable_place_change(Some(1), None) == false.
  implication: Стирание места в NULL по определению не даёт записи перемещения — D-06 by design, не баг журнала.

- timestamp: 2026-09-03
  checked: ui/src/features/cartridges/OperationModal.svelte:107-113, 146, 505, 711-717
  found: |
    `previousCartridgePlaceId = $state<number | null>(null)`, сбрасывается в null при
    открытии модалки, никогда не преднаполняется. Комментарий-исходник (стр. 107-110)
    прямо фиксирует: «editable charge state (default Пустой/3) and place (default none)».
    Подпись поля — «Место (предыдущий картридж)», соседняя — «Состояние заряда
    (предыдущий картридж)»: скобки здесь паттерн различения двух картриджей формы,
    а не маркер необязательности. Хинта у поля нет.
  implication: «default none» был осознанным решением Plan 12-09 и с тех пор не пересматривался; подпись о нём не сообщает.

- timestamp: 2026-09-03
  checked: ui/src/features/cartridges/OperationModal.svelte:547-575 (validate) + 230-270 (autofill)
  found: |
    validate() требует placeId для op='install'/'to_refill' (стр. 563) и для
    op='return_to_stock'/'from_refill' (стр. 567-569) — «Заполните это поле».
    previousCartridgePlaceId в validate() НЕ УПОМЯНУТ.
    Отдельно: место НОВОГО картриджа автозаполняется из места принтера
    (`if (printer.devicePlaceId !== null && (placeId === null || placeAutofilled))`,
    стр. 267-270), у предыдущего картриджа автозаполнения нет.
  implication: |
    Ключевая асимметрия. Одна и та же логическая операция «возврат на склад» обязательна
    по месту, когда пользователь вызывает её напрямую, и не обязательна, когда система
    выполняет её неявно внутри установки. Плюс в той же форме одно поле места
    автозаполняется, другое — нет, что закрепляет ожидание пользователя.

- timestamp: 2026-09-03
  checked: target/debug/trackly.db — audit_log, только счётчики/флаги NULL, без содержимого текстовых полей
  found: |
    Разделение по наличию actor-ключей в payload (авто-возврат пишет given_by_name,
    прямой возврат — нет):
      прямой возврат,  place_id NULL: 17 шт, все 2026-06-12 .. 2026-06-24 (эпоха free-text location, до place_id)
      прямой возврат,  place_id есть:  3 шт, все 2026-09-02 (текущий код)
      авто-возврат,    place_id NULL:  7 шт, 2026-06-25 .. 2026-09-02 17:26:02  ← включая текущий код
      авто-возврат,    place_id есть:  2 шт, 2026-08-25 и 2026-09-02 17:27:12
    Записей возврата, где before place_id NOT NULL и payload place_id NULL (т.е. место реально
    стёрто): ровно 1, и она — авто-возврат.
  implication: |
    На текущем коде прямой возврат НИКОГДА не приходит без места (валидация работает),
    авто-возврат — приходит. Это и есть «путь воспроизводится не всегда» из UAT:
    отличается не случайность, а тип операции.

- timestamp: 2026-09-03
  checked: target/debug/trackly.db — цепочка cartridge 13 (audit_log + place_movements), только id/时间
  found: |
    audit: create → install(place=7) → return_to_stock(before 7 → 4) → to_refill(4→10) →
    from_refill(10→4) → install(4→3) → [bulk move 3→9, movement id 24] →
    return_to_stock(before place_id=9, payload place_id=NULL, авто-возврат, 17:26:02) →
    update (ручное «Изменён», before/after/payload все NULL — этот путь снапшотов не пишет) →
    to_refill(before 4 → 10) → from_refill(10→4) → install(4→9).
    place_movements для entity 13: id 17,18,19,20,24,25,26,27 — между 20/24 и 25 записи
    авто-возврата НЕТ, как и записи для восстановления места ручным update (NULL→4, D-06).
  implication: |
    Инцидент воспроизведён по данным: авто-возврат стёр место 9 → NULL (перемещение не
    записано), затем ручная правка вернула место из NULL (перемещение снова не записано,
    уже по причине D-06 «первичное заполнение»). Пользователь увидел ДВА пропуска подряд —
    отсюда ощущение «события пропадают».

- timestamp: 2026-09-03
  checked: Понятие «склад» в системе — places.is_storage, cartridge_storage_place_ids
  found: |
    migrations/V037__places.sql:11 — `is_storage INTEGER NOT NULL DEFAULT 0` (D-08), флаг на месте.
    В dev-БД: 10 мест, из них ровно 1 с is_storage=1.
    Есть готовая команда `cartridge_storage_place_ids` на ОБОИХ транспортах
    (tauri_cmds/cartridges.rs:415, http/cartridges.rs:451 `/api/v1/cartridge_storage_place_ids`),
    реализация — services/cartridge_service.rs:936, комментарий: «exposed for the frontend's
    ReturnToStock place-suggestion UX (D-11.3). The backend does not pick a default — a UI concern».
    Потребители в UI: ui/src/features/acts/ReturnModal.svelte:111 и
    ui/src/features/devices/DeviceFormBody.svelte:181. В ui/src/features/cartridges/ —
    НИ ОДНОГО потребителя.
    Понятия «склад по умолчанию» (одно выделенное место) в системе НЕТ: нигде нет настройки
    default_place/stock_place, is_storage — множественный флаг, используется как фильтр отчётов
    и как источник chip-подсказок.
  implication: |
    Для варианта (а) бэкенд-API уже есть и уже применён на двух других поверхностях —
    цена вопроса UI-only. Но «склад по умолчанию» пришлось бы либо вводить как новое понятие,
    либо выводить из истории (последнее складское место картриджа восстановимо из
    place_movements: последняя запись с to_place_id из множества складских).

- timestamp: 2026-09-03
  checked: ui/src/lib/components/PlacePicker.svelte:29-31
  found: "`value: number | null` — «Выбранный place_id (или null — место не выбрано)». Семантики «не менять» у контрола нет."
  implication: Пустое значение принципиально неотличимо от «оставить как есть» — на уровне контрола выразить «не менять» сейчас нечем.

- timestamp: 2026-09-03
  checked: crates/trackly-app/tests/cartridges_lifecycle.rs:786-790, 1113-1117
  found: |
    Текущее поведение ЗАФИКСИРОВАНО тестами:
    - стр. 789: `assert_eq!(a_place_id, None, "A's place_id must be cleared to NULL
      (no previous_cartridge_place_id override supplied)")`
    - стр. 1113-1117 (doc-comment теста install_auto_return_falls_back_to_defaults_when_overrides_absent):
      «falls back to 12-06's original hardcoded defaults (state_id=3 Пустой, place_id=NULL)»
  implication: Любой фикс варианта (а) или (б) обязан переписать эти два теста — иначе гейт красный, и это не регрессия, а смена контракта.

## Resolution

root_cause: |
  Ветка авто-возврата предыдущего картриджа (crates/trackly-infra/src/repos/cartridges_sqlite.rs:614)
  берёт `previous_cartridge_place_id` как есть, без fallback, и безусловно записывает его в
  `cartridges.place_id`. Пустое поле «Место (предыдущий картридж)» приходит как NULL, поэтому
  операция не «оставляет место прежним», а СТИРАЕТ его. Переход «место → NULL» по D-06 не
  порождает записи перемещения, и операция выглядит бесследной.
  Усугубляющий фактор (он же причина «не всегда воспроизводится»): в
  ui/src/features/cartridges/OperationModal.svelte::validate() поле места обязательно для
  прямой операции `return_to_stock` (стр. 567-569), но для `previousCartridgePlaceId`
  валидации нет вообще. Та же логическая операция обязательна при явном вызове и
  необязательна при неявном — прямые возвраты на текущем коде всегда приходят с местом,
  авто-возвраты приходят без.
  Подпись «Место (предыдущий картридж)» скобками различает два картриджа формы, а не
  сообщает о необязательности; хинта нет, а место НОВОГО картриджа в той же форме
  автозаполняется из места принтера — это закрепляет ожидание автоподстановки.

fix: |
  Не применяется (goal: find_root_cause_only). Требуется продуктовое решение, факты для выбора:
  (а) автоподстановка складского места: API `cartridge_storage_place_ids` уже есть на обоих
      транспортах и уже используется в ReturnModal.svelte и DeviceFormBody.svelte — работа
      UI-only. Но «склада по умолчанию» в системе нет (is_storage — множественный флаг;
      в текущей БД он случайно единственный). Прежнее складское место конкретного картриджа
      восстановимо из place_movements. Требует переписать 2 теста.
  (б) сделать поле обязательным: симметрично уже существующей валидации `return_to_stock`
      в том же validate(), правка в одну ветку if. Требует переписать те же 2 теста.
  (в) только подпись/хинт: дешевле всего, но оставляет молчаливое стирание места живым и
      не чинит уже накопленные NULL (в dev-БД 12 из 15 картриджей без места, из них 8 «На складе»).
  Смежное, вне этой сессии: пропуск D-06 при NULL→место означает, что и восстановление места
  ручной правкой тоже не попадает в историю — второй пропуск подряд в той же цепочке.

verification: n/a — диагностика без фикса
files_changed: []
