---
status: diagnosed
trigger: "grouped-device-list-place-inversion — В сгруппированном списке устройств колонка «Место» показывает путь у строки-группы и прочерк у самих устройств"
created: 2026-09-03
updated: 2026-09-03
---

## Current Focus

hypothesis: "ПОДТВЕРЖДЕНО. Не одна инверсия, а два независимых дефекта на бэкенде: (A) РЕГРЕССИЯ фазы 39.1 — list_by_ids (источник детей развёрнутой группы) маппится голым from_row, который жёстко ставит place_path_short: None, а фронтенд с коммита 3909ae92 читает именно это поле → «—» у каждого устройства внутри группы; (B) ДАВНИЙ ПРОБЕЛ (с фазы 02-04) — list_grouped берёт место группы как MAX(place_id) произвольного члена, проверки однородности нет ни в SQL, ни в DTO → строка-группы показывает место одного случайного устройства."
test: "Инспекция кода обоих путей запроса + git-археология + счётчики по дев-БД."
expecting: "—"
next_action: "Передать диагноз оркестратору (goal: find_root_cause_only). Фикс НЕ применяется."

## Symptoms

expected: |
  Колонка «Место» в сгруппированном списке «Устройства»:
  - у устройства внутри группы — его собственное место;
  - у строки-группы — место только если оно одинаково у ВСЕХ устройств группы, иначе «—».
actual: |
  Инверсия: строка-группы показывает реальный путь; устройства внутри группы — «—».
  Отдельные (негруппированные, count==1) строки место показывают.
errors: нет
reproduction: Раздел «Устройства», группировка включена, развернуть группу (count > 1). UAT фазы 40, тест 13.
started: вероятная регрессия фаз 39 / 39.1 (place_path_short, place_effective_variant)

## Eliminated

- hypothesis: "Строка-группы и строка-устройства читают РАЗНЫЕ поля во фронтенде (перепутаны после переименования)"
  evidence: "Оба компонента читают одно и то же поле: DeviceGroupRow.svelte — `group.repr.place_path_short ?? '—'`, DeviceListRow.svelte:68 — `device.place_path_short ?? '—'`. Фронтенд симметричен; расхождение целиком на бэкенде."
  timestamp: 2026-09-03

## Evidence

- checked: "ui/src/features/devices/DeviceGroupRow.svelte + DeviceListRow.svelte:68"
  found: "Оба рендерят `place_path_short ?? '—'`, title = full_path. Разметка идентична."
  implication: "Разница исключительно в том, чем бэкенд наполняет place_path_short на каждом пути запроса."

- checked: "ui/src/features/devices/DeviceList.svelte:120-145"
  found: "count > 1 → DeviceGroupRow (данные group.repr из list_grouped); count == 1 → DeviceListRow, но тоже с group.repr из list_grouped. Дети развёрнутой группы грузятся отдельным вызовом devices.listByIds(group.ids)."
  implication: "Одиночки берут данные из list_grouped (место есть — совпадает с симптомом), дети — из list_by_ids."

- checked: "crates/trackly-infra/src/repos/devices_sqlite.rs:30-70 (SELECT_DEVICES, from_row, from_row_with_short_path)"
  found: "from_row жёстко ставит `place_path_short: None`. Короткий путь вычисляет только from_row_with_short_path, который по прямому решению D-19 используется ТОЛЬКО в list()/search_fts(); doc-комментарий буквально перечисляет list_by_ids среди тех, кто «продолжает использовать голый from_row и всегда получает place_path_short: None»."
  implication: "ДЕФЕКТ A подтверждён по коду: любые дети развёрнутой группы приходят с place_path_short = None → «—» всегда, независимо от данных."

- checked: "crates/trackly-infra/src/repos/devices_sqlite.rs:1353 list_by_ids + crates/trackly-app/src/services/device_service.rs:979"
  found: "list_by_ids = `{SELECT_DEVICES} WHERE d.id IN (...)` с маппером `from_row` (не from_row_with_short_path). Сервис делает голый `DeviceDto::from(row)` — постобработки нет ни на одном слое (Tauri-команда и axum-хендлер — тонкие адаптеры)."
  implication: "Оба транспорта (десктоп и LAN-браузер) поражены одинаково."

- checked: "crates/trackly-infra/src/repos/devices_sqlite.rs:1020-1210 (три SQL-ветки list_grouped)"
  found: "Во всех трёх ветках место группы приходит через `LEFT JOIN place_full_paths pfp ON pfp.place_id = (SELECT MAX(d2.place_id) FROM devices d2 WHERE <ключ группы> ...)`. MAX() = произвольный (наибольший) place_id среди членов группы; проверки однородности (COUNT(DISTINCT place_id)) нет ни в SELECT, ни в DTO."
  implication: "ДЕФЕКТ B подтверждён по коду: строка-группы показывает место одного случайного члена как место всей группы. Нет данных, чтобы фронтенд мог показать «—» при разнородности (аналог condition_distinct_count для места отсутствует)."

- checked: "ui/src/features/devices/DevicesPage.svelte:63-72 — baseFilter"
  found: "group_by_condition: false → в list_grouped работает ветка `sql_without_condition`: GROUP BY d.type_id, d.name; place-подзапрос коррелирует по (type_id, name)."
  implication: "Ключ группы НЕ включает место — устройства с разными местами обязаны попадать в одну группу by design; значит место группы принципиально может быть неоднородным."

- checked: "target/debug/trackly.db (только счётчики, без содержимого полей)"
  found: "groups_total(count>1)=8; из них с неоднородным place_id=3; однородных=5, и все 5 однородных — это группы, где place_id NULL у ВСЕХ членов (groups_multi_all_place_null=5). Устройств с местом всего 7 из 41."
  implication: "Полностью объясняет скриншот: 3 группы показывают реальный путь произвольного члена (дефект B), а «другая группа», где и группа, и дети показывают «—», — это группа полностью без мест, т.е. корректное поведение, а не третий дефект."

- checked: "git log -L на ячейке «Место» в DeviceListRow.svelte"
  found: "Коммит 3909ae92 «feat(39.1-10): 4 компонента читают place_path_short вместо локального сокращения» заменил `device.full_path ? shortenPlacePath(...) : '—'` на `device.place_path_short ?? '—'`."
  implication: "ТОЧКА РЕГРЕССИИ дефекта A. До 3909ae92 дети группы показывали место: SELECT_DEVICES всегда джойнит place_full_paths, поэтому full_path у list_by_ids заполнен, а сокращение делалось на клиенте. После — компонент читает поле, которое этот путь запроса не заполняет никогда."

- checked: ".planning/phases/39.1-place-path-display/39.1-CONTEXT.md D-17/D-19 + 39.1-03-SUMMARY.md"
  found: "D-17 относит список «Устройства» к зоне сокращения. D-19 перечисляет как «остаются полными» только детальные экраны, PlacePicker/ReturnModal и поиск в дереве. list_by_ids в D-19 не упомянут, но SUMMARY 39.1-03 записал его в исключения вместе с get/get_in_tx/restore_from_snapshot_in_tx («unaffected by construction, not by an added exclusion check»)."
  implication: "list_by_ids классифицирован как «не список», хотя это ровно раскрытие группы в списке «Устройства» (зона D-17). Хуже того, деградация выбрана неверная: голый from_row даёт None (= «места нет»), а не полный путь, — то самое, от чего явно предостерегает doc-комментарий from_row_with_short_path (WR-01 фазы 39.2)."

- checked: "git log -S \"MAX(d2.location_id)\" -- devices_sqlite.rs"
  found: "Подзапрос MAX(...) в place-джойне тянется с 05a639ae (фаза 02-04) → 575fac32 → c3a52374; a1663a99 (фаза 39-06) лишь переименовал location_id → place_id."
  implication: "Дефект B НЕ регрессия фаз 39/39.1 — это исходное поведение, которое пользователь заметил только сейчас, на контрасте с пустыми детьми."

- checked: "crates/trackly-app/src/dto/device.rs:249-262 (DeviceGroup) + trackly-core/src/domain/devices.rs:116 (DeviceGroupRow)"
  found: "Есть condition_distinct_count, аналога place_distinct_count нет ни в core-row, ни в DTO."
  implication: "Починить дефект B чисто во фронтенде нельзя — у клиента физически нет признака однородности места. Нужна колонка в SQL + поле в DeviceGroupRow/DeviceGroup."

- checked: "ui/scripts/check-place-path-short.mjs (INV-1..INV-4)"
  found: "Гейт структурный: проверяет, что ячейка ЧИТАЕТ place_path_short, что title несёт full_path и что полный путь не рендерится в тексте. Он не проверяет, что источник данных это поле ЗАПОЛНЯЕТ."
  implication: "Дефект A прошёл сквозь гейт по построению: гейт держит контракт компонента, а сломан контракт репозитория. Фикс обязан быть на бэкенде (INV-3 запрещает вернуть рендер full_path в Svelte)."

## Resolution

root_cause: |
  Два независимых дефекта, дающих в сумме видимость «инверсии».

  (A) Устройства внутри группы всегда «—» — РЕГРЕССИЯ фазы 39.1.
  Дети развёрнутой группы грузятся через devices.listByIds → DeviceService::list_by_ids →
  DevicesSqliteRepo::list_by_ids, который маппит строки функцией `from_row`. `from_row`
  жёстко присваивает `place_path_short: None` (devices_sqlite.rs:67); короткий путь
  считает только `from_row_with_short_path`, применяемый исключительно в `list()` и
  `search_fts()` (граница D-19, зафиксированная в 39.1-03-SUMMARY). Пока фронтенд рендерил
  `full_path`, это было безвредно — SELECT_DEVICES всегда джойнит place_full_paths.
  Коммит 3909ae92 (39.1-10) переключил ячейку на `place_path_short ?? '—'`, и путь запроса,
  который это поле не заполняет, начал печатать «—». Постобработки нет ни в сервисе, ни в
  адаптерах — поражены оба транспорта (Tauri и LAN-браузер).

  (B) Строка-группы показывает место, даже когда оно у членов разное — ДАВНИЙ ПРОБЕЛ,
  не регрессия. Во всех трёх SQL-ветках list_grouped место группы приходит через
  `LEFT JOIN place_full_paths pfp ON pfp.place_id = (SELECT MAX(d2.place_id) ...)`, то есть
  берётся место ПРОИЗВОЛЬНОГО (наибольшего по place_id) члена. Проверки однородности нет:
  в DeviceGroupRow/DeviceGroup есть condition_distinct_count, но аналога place_distinct_count
  не существует. Паттерн тянется с фазы 02-04 (тогда MAX(location_id)); фаза 39-06 только
  переименовала колонку.

  Наблюдение «у другой группы и группа, и устройства показывают «—»» третьим дефектом
  не является: по дев-БД 5 из 8 многоэлементных групп состоят целиком из устройств без места.

fix: "не применялся (goal: find_root_cause_only)"
verification: "не применялась"
files_changed: []

## Suggested Fix Direction

- (A) В `DevicesSqliteRepo::list_by_ids` перейти с `from_row` на `from_row_with_short_path`
  (SELECT_DEVICES уже джойнит place_effective_variant — колонка 16 доступна, менять SQL не
  нужно; отдельно прочитать сепараторы через `read_path_display_separators`, как это делают
  `list`/`search_fts`/`list_grouped`). Это переносит list_by_ids на правильную сторону
  границы D-17/D-19 — раскрытие группы есть часть списка «Устройства», а не детальный экран.
  Пересмотреть заодно, не поражены ли тем же способом другие «не-списочные» пути, которые
  всё же питают списочные ячейки.
- (B) Добавить в три SQL-ветки `list_grouped` колонку
  `COUNT(DISTINCT COALESCE(d.place_id, -1)) AS place_distinct_count`, протянуть её через
  GroupRowTuple → `DeviceGroupRow` → `DeviceGroup` (симметрично condition_distinct_count) и
  при значении > 1 не отдавать place_path_short/full_path у repr (либо гасить ячейку на
  фронтенде) — ровно поведение, которого ждёт пользователь.
- Регрессионный гейт: гейт `check-place-path-short.mjs` структурный и по построению слеп к
  этому классу дефектов. Нужен интеграционный тест на уровне репозитория/сервиса:
  «устройство с местом, полученное через list_by_ids, несёт непустой place_path_short» —
  плюс тест на неоднородную группу.
