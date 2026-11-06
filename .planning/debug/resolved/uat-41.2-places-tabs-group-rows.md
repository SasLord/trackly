---
status: resolved
trigger: "uat-41.2-places-tabs-group-rows: в «Местах» при выключенном «Разворачивать группы» строка группы и раскрытый состав не учитывают вкладку"
created: 2026-11-05
updated: 2026-11-05
---

## Current Focus

hypothesis: CONFIRMED — клиентский фильтр вкладок в PlaceContents.svelte реализует D-21/R-08 буквально: группа только на «Все»/«Устройства», на «Принтеры» никогда; раскрытие и подпись счётчика не фильтруются по вкладке.
next_action: вернуть диагноз; нужна правка решения D-21/R-08/R-09 (продуктовое решение уже дано UAT)

## Symptoms

expected: «АРМ #3» = 1 устройство + 2 принтера. «Устройства»: «АРМ #3 (1 устройство)», в раскрытии только устройство. «Принтеры»: «АРМ #3 (2 принтера)», в раскрытии 2 принтера.
actual: «Устройства»: «АРМ #3 (3 устройства)», раскрытие показывает всех 3. «Принтеры»: таблица пустая. Счётчики вкладок верны; режим с разворотом работает.
errors: нет
reproduction: Тест 5, .planning/phases/41.2-groups-acts-places/41.2-UAT.md
started: найдено на UAT 2026-11-05

## Eliminated

- hypothesis: places_group_members отдаёт kind "device" для принтеров, и поэтому раскрытие нельзя отфильтровать
  evidence: list_group_members_impl (places_sqlite.rs:465+) отдаёт CASE type_id='Принтер' THEN 'printer' ELSE 'device'; принтерность у ветки группы (printer_member_count) и у состава одна — тип устройства. Счётчик «Принтеры»=2 это подтверждает.
  timestamp: 2026-11-05

- hypothesis: бэкенд не отдаёт строку группы для вкладки «Принтеры»
  evidence: вкладки — чисто клиентский фильтр; places_contents отдаёт строку group независимо от вкладки (41.2-05-PLAN truth). Отсекает клиент.
  timestamp: 2026-11-05

## Evidence

- timestamp: 2026-11-05
  checked: ui/src/features/places/PlaceContents.svelte:347-358 (filteredRows)
  found: для r.kind==='group' возвращается `activeTab === 'device' && member_count - printer_member_count > 0` — на «Принтеры» группа отсекается всегда.
  implication: пустая вкладка «Принтеры» — прямое следствие; комментарий ссылается на D-21/R-08.

- timestamp: 2026-11-05
  checked: 41.2-CONTEXT.md D-21, 41.2-UI-SPEC.md :675-690, R-08, R-09
  found: спецификация намеренно ставит группу только на «Все»/«Устройства»; расхождение счётчика и пустой вкладки «Принтеры» объявлено «законным» и закрыто текстом пустого состояния (emptyTabHint, :370-382).
  implication: код соответствует спеке; дефект — в решении D-21/R-08/R-09, UAT его отвергла.

- timestamp: 2026-11-05
  checked: PlaceContents.svelte:629 и :637-656
  found: подпись `deviceCountLabel(row.member_count)` — полный состав без учёта вкладки; раскрытие рендерит `membersByGroup[row.id]` целиком, без фильтра по member.kind и activeTab.
  implication: «(3 устройства)» и 3 строки в раскрытии на «Устройствах».

- timestamp: 2026-11-05
  checked: places_sqlite.rs:368-405 (group_branch), :465-500 (list_group_members_impl)
  found: member_count/printer_member_count и kind членов считаются по одному признаку (device_types.name='Принтер'); данных клиенту хватает для фильтра без изменений бэкенда.
  implication: правка — только фронтенд (+ спека/тесты).

## Resolution

root_cause: Фильтр вкладок в PlaceContents.svelte реализует решение D-21/R-08 (группа видна только на «Все» и «Устройства»; на «Принтеры» — никогда, расхождение «прикрыто» подсказкой R-09). Подпись счётчика группы берёт полный member_count, раскрытие рендерит весь состав без фильтра по вкладке.
fix:
verification:
files_changed: []


## Закрыто 2026-11-07
Подтверждено живой UAT 41.2 (G1–G12).
