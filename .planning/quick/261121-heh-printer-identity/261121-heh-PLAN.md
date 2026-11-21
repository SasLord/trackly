---
phase: quick-261121-heh
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-core/src/domain/devices.rs
  - crates/trackly-app/src/services/device_service.rs
  - crates/trackly-app/src/services/request_service.rs
  - crates/trackly-app/src/services/employee_device_service.rs
  - crates/trackly-app/src/services/cartridge_service.rs
  - crates/trackly-app/src/services/number_template_service.rs
  - crates/trackly-app/src/tauri_cmds/printers.rs
  - crates/trackly-infra/src/repos/places_sqlite.rs
  - crates/trackly-infra/src/repos/groups_sqlite.rs
  - crates/trackly-infra/src/repos/cartridges_sqlite.rs
  - crates/trackly-app/tests/request_printer_options.rs
  - crates/trackly-app/tests/employee_devices_list.rs
  - crates/trackly-app/tests/places_contents.rs
  - crates/trackly-app/tests/printer_identity_gate.rs
  - crates/trackly-app/tests/suites/gates.rs
  - ui/src/lib/deviceTypeIds.ts
  - ui/src/features/devices/PrinterMark.svelte
  - ui/src/features/devices/DeviceFormBody.svelte
  - ui/src/features/devices/DeviceFormModal.svelte
  - ui/src/features/devices/DeviceRowMenu.svelte
  - ui/src/features/groups/GroupsPage.svelte
  - ui/src/features/groups/GroupPrintersList.svelte
  - ui/src/features/printers/PrintersPage.svelte
  - ui/scripts/check-printer-identity.mjs
  - ui/package.json
autonomous: true
requirements: [INT-31, NEW-5]

must_haves:
  truths:
    - "Признак «это принтер» на бэкенде определён ОДИН раз: `PRINTER_TYPE_ID` + `is_printer_type()` + SQL-макрос `printer_type_id_sql!()` в crates/trackly-core/src/domain/devices.rs; все 15 мест инвентаря B1–B15 используют его"
    - "Переименование типа id=2 в справочнике device_types (UPDATE device_types SET name = 'Печатающее устройство' WHERE id = 2) не ломает ни варианты принтеров в заявке, ни is_printer в списке сотрудника, ни kind='printer' в содержимом места"
    - "NEW-5 закрыт: employee_device_service (is_printer) и request_service (request_printer_options) используют один и тот же признак devices.type_id = PRINTER_TYPE_ID — устройство, помеченное сотруднику принтером, всегда есть в вариантах принтеров заявки, и RequestFormModal предзаполняет его"
    - "Rust-гейт printer_identity_gate краснеет на новом литерале `type_id = 2` / `type_id == 2` / `type_id: 2`, на сравнении с именем 'Принтер' в SQL или коде, и на новом `FROM printers`/`JOIN printers` вне реестра вердиктов «данные принтера»; проверено мутацией реального исходника"
    - "На фронте литерал 2 для типа принтера живёт только в ui/src/lib/deviceTypeIds.ts; гейт check-printer-identity.mjs краснеет на новой копии и сверяет значение с Rust-константой из trackly-core; входит в pnpm lint"
  artifacts:
    - path: "crates/trackly-core/src/domain/devices.rs"
      provides: "DEVICE_TYPE_ID, PRINTER_TYPE_ID, is_printer_type, #[macro_export] printer_type_id_sql!"
      contains: "pub const PRINTER_TYPE_ID: i64 = 2;"
    - path: "crates/trackly-app/tests/printer_identity_gate.rs"
      provides: "Сканер исходников crates/*/src: правила P1–P5 + фикстуры + мутации реальных файлов"
    - path: "ui/src/lib/deviceTypeIds.ts"
      provides: "DEVICE_TYPE_ID, PRINTER_TYPE_ID, isPrinterType — единственный фронтовый источник"
    - path: "ui/scripts/check-printer-identity.mjs"
      provides: "JS-гейт J1–J5 с --selftest"
  key_links:
    - from: "crates/trackly-app/src/services/request_service.rs"
      to: "trackly_core::printer_type_id_sql!"
      via: "concat! в SQL request_printer_options"
      pattern: "printer_type_id_sql!"
    - from: "crates/trackly-app/src/services/employee_device_service.rs"
      to: "trackly_core::printer_type_id_sql!"
      via: "is_printer = (d.type_id = <PRINTER>) вместо EXISTS(printers)"
      pattern: "printer_type_id_sql!"
    - from: "crates/trackly-app/tests/suites/gates.rs"
      to: "crates/trackly-app/tests/printer_identity_gate.rs"
      via: "#[path] mod printer_identity_gate"
      pattern: "printer_identity_gate"
    - from: "ui/package.json"
      to: "ui/scripts/check-printer-identity.mjs"
      via: "lint chain: --selftest и полный прогон"
      pattern: "check-printer-identity"
---

<objective>
Закрыть INT-31 (идентичность «это принтер» закодирована шестью способами, ~22 места) и NEW-5
(employee_device_service и request_service используют разные признаки → RequestFormModal молча
не предзаполняет принтер). Ввести ОДИН канонический признак — id типа `PRINTER_TYPE_ID` из сида
V001, определённый один раз в trackly-core, на SQL-стороне один макрос, на фронте один модуль,
сверяемый с Rust-константой; заменить все места; закрепить двумя гейтами, проверенными мутацией.

Purpose: Фаза 42 (умный подбор принтера) упрётся в INT-31 первой (F19 аудита v1.4).
Output: канон в trackly-core, 15 бэкенд-мест + 7 фронтовых файлов переведены, тесты-пробы
переименования, Rust-гейт printer_identity_gate, JS-гейт check-printer-identity.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@./CLAUDE.md
@.planning/v1.4-MILESTONE-AUDIT.md

## Почему канон — id типа, а не имя и не строка `printers`

* **Имя «Принтер» ошибочно в принципе.** `device_types` (V001: `id INTEGER PRIMARY KEY, name TEXT
  NOT NULL UNIQUE`, сид `(1,'Устройство'),(2,'Принтер')`) — по решению 41.12 (INT-14, D-01)
  единственный источник ПОДПИСЕЙ типа; подпись — данные, справочник «расширяется данными».
  Сегодня в коде нет команды переименования (в crates/*/src нет `UPDATE device_types`), но БД
  portable и редактируема, а подпись по замыслу не идентичность: переименование молча обнулит
  варианты принтеров в заявке и счётчики принтеров мест. Id — первичный ключ сида, стабилен.
* **Структурный признак (строка в `printers`) производен.** `DeviceService::sync_printer_row_in_tx`
  (device_service.rs ~666-700) в той же транзакции подтягивает строку `printers` за `type_id`:
  первичен `type_id`. Под инвариантом оба признака равны, но первичный — `type_id`.
  Следствие (принять, задокументировать в SUMMARY, НЕ лечить миграцией): в старых БД до quick
  260820-rdj возможен дрейф (строка `printers` при type_id≠2) — такое устройство перестанет
  считаться принтером в списке сотрудника и группах, как уже не считалось в заявке/картриджах/
  местах. Удалять строки `printers` миграцией нельзя (каскад показаний/ip).
* `printers_create` (N1) удалён в quick 261121-4nz — единственный путь расхождения уже закрыт;
  этот план закрывает само расхождение правил.

## Инвентарь (снят от репозитория, 2026-11-21; grep по crates/*/src и ui/src)

Классы: **L** — SQL-литерал `type_id = 2`; **C** — Rust-сравнение/инициализация числом;
**K** — локальная копия константы; **N** — по имени типа 'Принтер'; **S** — структурный признак
(EXISTS/JOIN printers как ФИЛЬТР идентичности); **F** — фронтовая копия `= 2`.

| # | Место | Класс | Действие |
|---|-------|-------|----------|
| B1 | crates/trackly-app/src/services/request_service.rs:308 (`request_printer_options`, комментарий WR-04 296-299) | N | `d.type_id = ` + макрос; комментарий WR-04 переписать (подзапрос по имени — ошибка, не страховка) |
| B2 | crates/trackly-infra/src/repos/places_sqlite.rs:394 (`printer_member_count`) | N | макрос |
| B3 | places_sqlite.rs:418 (`!=` — устройства) | N | макрос |
| B4 | places_sqlite.rs:432 (принтеры) | N | макрос |
| B5 | places_sqlite.rs:478 (`CASE … 'printer'` — источник `kind` для PlaceContents.svelte:97) | N | макрос; doc-комментарий 324 поправить |
| B6 | crates/trackly-app/src/services/employee_device_service.rs:85 `EXISTS(SELECT 1 FROM printers …)` → `is_printer` | S | `(d.type_id = <макрос>)` — это NEW-5 |
| B7 | crates/trackly-infra/src/repos/groups_sqlite.rs:699 `member_printers_for_group` (`JOIN printers p` только как фильтр) | S | заменить JOIN на `AND d.type_id = <макрос>`; doc 684-686 («строкой в printers, а не код типа») переписать |
| B8 | groups_sqlite.rs:982 `live_printer_device_ids` (`JOIN printers` как фильтр) | S | то же |
| B9 | crates/trackly-infra/src/repos/cartridges_sqlite.rs:441 | L | макрос; комментарий 424 поправить |
| B10 | cartridges_sqlite.rs:1838 | L | макрос |
| B11 | cartridges_sqlite.rs:1867 | L | макрос |
| B12 | crates/trackly-app/src/services/cartridge_service.rs:1205 (`suggest_compat_printer`, doc 1191) | L | макрос |
| B13 | crates/trackly-app/src/services/number_template_service.rs:794 `if type_id == 2 {"printer"}` (источник `kind` для NumberTakenPopup.svelte:48) | C | `is_printer_type(type_id)` |
| B14 | crates/trackly-app/src/tauri_cmds/printers.rs:150 `type_id: 2` (SNMP-приёмка; комментарии 75, 141) | C | `PRINTER_TYPE_ID` |
| B15 | crates/trackly-app/src/services/device_service.rs:664-665 `const DEVICE_TYPE_ID/PRINTER_TYPE_ID` (+ употребления 226, 681, 878, 943, 1467, 1495) | K | удалить ассоц. константы, импортировать из trackly-core; `match type_id { PRINTER_TYPE_ID => … }` работает с импортированной const |
| F1 | ui/src/features/devices/PrinterMark.svelte:12 | F | импорт; fallback `isPrinterType(typeId)`; приоритет серверного `isPrinter` сохранить |
| F2 | ui/src/features/devices/DeviceFormBody.svelte:224 (+ ~12 употреблений) | F | `import { PRINTER_TYPE_ID }` — имя идентификатора сохранить |
| F3 | ui/src/features/devices/DeviceFormModal.svelte:96-97 (DEVICE_TYPE_ID + PRINTER_TYPE_ID) | F | импорт обеих |
| F4 | ui/src/features/devices/DeviceRowMenu.svelte:110 `PRINTER_DEVICE_TYPE_ID = 2` | F | `import { PRINTER_TYPE_ID as PRINTER_DEVICE_TYPE_ID }` — имя держат якоря check-device-row-menu.mjs |
| F5 | ui/src/features/groups/GroupsPage.svelte:44 | F | импорт |
| F6 | ui/src/features/groups/GroupPrintersList.svelte:30 | F | импорт; `type_id: PRINTER_TYPE_ID` (якорь check-group-printer-search.mjs) не трогать |
| F7 | ui/src/features/printers/PrintersPage.svelte:124 | F | импорт (локальную const внутри функции убрать) |

**Следствия, кода не требуют** (чинятся B1/B6): RequestFormModal.svelte:199 (`availablePrinters.some`) —
после B6 множество `is_printer` ⊆ множества вариантов (с поправкой на видимость сотрудника).
**Уже серверные, оставить:** `EmployeeDeviceDto.is_printer` (dto/employee_device.rs:28 →
employeeDeviceMenu.ts:41, EmployeeDeviceViewModal/Tile/Row), `kind: 'printer'` (PlaceContents:97,
NumberTakenPopup:48) — производные B5/B13.
**Не признак — чтение данных принтера (реестр вердиктов гейта P3, оставить):**
employee_device_service.rs:277 (тонер), dashboard_service.rs:310/323/335 (SNMP-счётчики и алерты
по строкам `printers`), repairs_sqlite.rs:593/644 и groups_sqlite.rs:726 (связь USB-хоста —
`printers.usb_host_device_id`), printers_sqlite.rs:51/363/380/480 (собственный репозиторий
принтеров). Исполнитель пересчитывает точные числа вхождений по файлам после Task 1.
**Не трогать:** migrations/*.sql (сид — источник id), `#[cfg(test)]`-модули и crates/*/tests
(фикстуры `type_id = 2`/`PRINTER_TYPE_ID: i64 = 2` в тестах вне гейта), ui/src/bindings.ts
(генерат), deviceRowMenu.ts:65 `ANCHOR_DEVICE_TYPE_ID = 1` (иная семантика — тип-якорь, не принтер).

## Опасность фикстур (обязательно перед B6/B7/B8)

Тесты сеют строку `printers` прямым `INSERT INTO printers` без `type_id = 2` у устройства:
crates/trackly-app/tests/{employee_devices_list, groups_values_card, devices_container_printers,
groups_anchor, request_printer_scope, places_group_members, devices_group_children}.rs,
crates/trackly-infra/tests/{groups_repo, repairs_repo}.rs. После перевода B6/B7/B8 на `type_id`
такие фикстуры нарушают инвариант, и их тесты покраснеют. Чинить ФИКСТУРУ (сеять устройство с
`type_id = 2` / `UPDATE devices SET type_id = 2` рядом с INSERT printers), НЕ ослаблять продовый
признак и НЕ удалять ассерты. Каждую правку фикстуры перечислить в SUMMARY.

## Интерфейсы

device_type_label_gate.rs (crates/trackly-app/tests, 950 строк) — образец для нового гейта:
`strip_test_modules`, `lex` (комментарии/строки/сырые строки/char vs lifetime), `rust_sources_under`,
`gate_root`, `Exempt{file, rule, matched, …, count}`, `scan`, `mutate(src, anchor, replacement)`,
`MIN_FILES = 180`. Тест-бинарники друг друга не импортируют — копировать, как сделал этот файл.
ui/scripts/check-device-type-labels.mjs — образец JS-гейта: `_strip.mjs`, EXEMPT с причиной ≥20
символов, `--selftest` с мутантами по реальным исходникам, антивакуум.
suite_wiring.rs (app_gates) краснеет на неподключённом `tests/*.rs` — новый файл подключить в
tests/suites/gates.rs (алфавитный порядок, `#[path = "../printer_identity_gate.rs"] mod …;`).
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Канон в trackly-core + перевод бэкенд-мест B1–B15 + пробы переименования</name>
  <files>crates/trackly-core/src/domain/devices.rs, crates/trackly-app/src/services/device_service.rs, crates/trackly-app/src/services/request_service.rs, crates/trackly-app/src/services/employee_device_service.rs, crates/trackly-app/src/services/cartridge_service.rs, crates/trackly-app/src/services/number_template_service.rs, crates/trackly-app/src/tauri_cmds/printers.rs, crates/trackly-infra/src/repos/places_sqlite.rs, crates/trackly-infra/src/repos/groups_sqlite.rs, crates/trackly-infra/src/repos/cartridges_sqlite.rs, crates/trackly-app/tests/request_printer_options.rs, crates/trackly-app/tests/employee_devices_list.rs, crates/trackly-app/tests/places_contents.rs (+ фикстуры из раздела «Опасность фикстур» по мере покраснения)</files>
  <behavior>
    - request_printer_options.rs: новый тест — засеять 2 принтера (существующий `seed_printer_devices`), выполнить `UPDATE device_types SET name = 'Печатающее устройство' WHERE id = 2`, запросить варианты → оба принтера на месте. ДО фикса красный (подзапрос по имени даёт NULL).
    - places_contents.rs: новый тест — устройство type_id=2 в месте, переименовать тип так же → `list_subtree_contents` отдаёт kind = "printer" и счётчик принтеров места учитывает его. ДО фикса красный.
    - employee_devices_list.rs: новый тест NEW-5 — устройство с type_id=2 в зоне сотрудника: is_printer = true И его id присутствует в request_printer_options (тот же признак); устройство с строкой printers, но type_id=1 (дрейф) — is_printer = false. Вторая половина ДО фикса красная.
    - trackly-core unit-тесты в devices.rs: `printer_type_id_sql!()` == `PRINTER_TYPE_ID.to_string()`; `is_printer_type(PRINTER_TYPE_ID)` и `!is_printer_type(DEVICE_TYPE_ID)`; сид V001 (include_str! относительным путём до migrations/V001__init_pragmas_and_lookups.sql в корне репо) содержит `(PRINTER_TYPE_ID, '` и `(DEVICE_TYPE_ID, '` — константы согласованы с сидом.
  </behavior>
  <action>
RED: сначала добавить три интеграционных теста из behavior (имена вымышленные, «Принтер А», «Иванов И.И.» — никаких реальных данных), прогнать нужные suite и убедиться, что падают ожидаемые; закоммитить `test(quick-261121-heh): пробы переименования типа и паритет is_printer/вариантов заявки (INT-31, NEW-5) — RED`.

GREEN: в crates/trackly-core/src/domain/devices.rs добавить с doc-комментарием «единственное определение признака принтера (INT-31); значение — id сида V001» `pub const DEVICE_TYPE_ID: i64 = 1;`, `pub const PRINTER_TYPE_ID: i64 = 2;`, `pub fn is_printer_type(type_id: i64) -> bool`, и `#[macro_export] macro_rules! printer_type_id_sql` с ветвью `() => { "2" }` (раскрывается в строковый литерал, чтобы вставлять в статический SQL через `concat!("… d.type_id = ", trackly_core::printer_type_id_sql!(), " …")`; если `concat!` с пользовательским макросом не соберётся — использовать `format!` с константой там, где SQL уже собирается строкой, и описать выбор в SUMMARY). Плюс unit-тесты из behavior.
Перевести B1–B15 по таблице инвентаря (B6/B7/B8 — с `type_id` вместо `printers`; у B7/B8 убрать JOIN printers, если из него ничего не читается; в B6 SELECT-колонка 9 становится `(d.type_id = <макрос>)`). Комментарии, утверждающие старое правило (request_service 296-299, groups_sqlite 684-686, places_sqlite 324, cartridges_sqlite 424, cartridge_service 1191, tauri_cmds/printers.rs 75/141), переписать под канон со ссылкой на INT-31. В device_service: удалить ассоциированные константы, `use trackly_core::domain::devices::{DEVICE_TYPE_ID, PRINTER_TYPE_ID}`, поправить `DeviceService::PRINTER_TYPE_ID`/`Self::…` во всех употреблениях.
Затем прогнать затронутые suite; покрасневшие из-за фикстур (см. «Опасность фикстур») чинить в фикстуре. Коммит `fix(quick-261121-heh): единый признак принтера PRINTER_TYPE_ID в trackly-core, 15 мест (INT-31, NEW-5)`.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && pnpm --dir ui build && cargo test -p trackly-core && for t in app_server app_repairs app_places app_cartridges app_reports app_groups app_devices; do TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test $t || break; done && cargo test -p trackly-infra --test infra_tests</automated>
  </verify>
  <done>Три новых теста зелёные (и были красными до GREEN — зафиксировано в SUMMARY выводом); grep `type_id = 2|type_id == 2|type_id: 2|name = 'Принтер'` по crates/*/src вне `#[cfg(test)]` и devices.rs пуст; все перечисленные suite зелёные; правки фикстур перечислены. Cargo — по одному процессу, последовательно; при exit 69 на линковке — префикс DEVELOPER_DIR=/Library/Developer/CommandLineTools. Команды `timeout` на машине нет — не оборачивать.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Rust-гейт printer_identity_gate + проверка мутацией реального исходника</name>
  <files>crates/trackly-app/tests/printer_identity_gate.rs, crates/trackly-app/tests/suites/gates.rs</files>
  <behavior>
    - Реальное дерево crates/*/src (после Task 1) — зелёное; MIN_FILES ≥ 180 просканировано.
    - Фикстура на каждое правило: `"… WHERE d.type_id = 2 …"` → [P1]; `if type_id == 2 {` → [P1]; `DeviceNew { type_id: 2, … }` → [P1]; `"… (SELECT id FROM device_types WHERE name = 'Принтер') …"` → [P2]; `if t.name == "Принтер"` → [P2]; `"EXISTS(SELECT 1 FROM printers p WHERE …)"` в незарегистрированном файле → [P3]; `PRINTER_TYPE_ID: i64 = 2` вне канонического файла → [P4].
    - Мутации реальных файлов (include_str! + mutate с проверкой, что якорь встречается РОВНО один раз, иначе паника теста): cartridge_service.rs — вызов макроса в SQL B12 заменить на литерал `2` → красный [P1]; request_service.rs — B1 вернуть к подзапросу по имени → красный [P2]; employee_device_service.rs — B6 вернуть к `EXISTS(SELECT 1 FROM printers p WHERE p.device_id = d.id)` → красный [P3] (счёт вхождений в файле больше зарегистрированного).
    - Реестр P3: лишнее вхождение, пропавшее вхождение и запись реестра без срабатывания — красные.
  </behavior>
  <action>
Создать printer_identity_gate.rs по образцу device_type_label_gate.rs (скопировать лексер, `strip_test_modules`, обход `crates/*/src/**/*.rs`, `gate_root`, `mutate` — тест-бинарники друг друга не импортируют). Канонический файл — crates/trackly-core/src/domain/devices.rs. Правила (сообщение начинается с кода):
[P1] числовой признак вне канона: в строковых литералах (SQL) регэксп `type_id\s*(=|==|!=|<>)\s*2(?!\d)` (без регистра, допускает алиас `d.`); в очищенном коде `type_id\s*(==|!=)\s*2\b`, `\b2\s*(==|!=)\s*[\w.]*type_id`, `type_id\s*:\s*2\b`.
[P2] признак по имени: строковый литерал, содержащий `'принтер'` в одинарных кавычках (без регистра, SQL-сравнение), либо `==`/`!=` вплотную (через пробелы) к литералу «Принтер» (trim, без регистра) в коде. Подпись «Принтер» в выводе (toast, CSV) не флагается — только сравнение.
[P3] структурный признак: строковый литерал, содержащий `\b(from|join)\s+printers\b` (без регистра). Каждый файл с такими литералами обязан быть в реестре `PRINTER_DATA_READS: &[(file, count, verdict)]` с точным числом и вердиктом ≥20 символов («данные принтера: тонер», «связь USB-хоста», «SNMP-счётчики/алерты», «репозиторий принтеров»). Числа снять по дереву после Task 1. Новый файл или рост числа — красный с подсказкой «признак принтера = devices.type_id = printer_type_id_sql!(); JOIN printers только для чтения полей принтера — добавьте вердикт».
[P4] определение константы/макроса со значением 2 (`PRINTER_TYPE_ID\s*:\s*i64\s*=\s*2`, `printer_type_id_sql`-подобная ветвь `"2"`) вне канонического файла.
[P5] антивакуум: файлов ≥ MIN_FILES; канонический файл содержит `pub const PRINTER_TYPE_ID: i64 = 2;` и `macro_rules! printer_type_id_sql`; употреблений `PRINTER_TYPE_ID|is_printer_type|printer_type_id_sql!` вне канона ≥ 10; каждая запись реестра P3 сработала с точным числом.
В модульном doc-комментарии честно перечислить, чего гейт не видит (склейка строки из частей, число через переменную `let k = 2`, `IN (1, 2)`, сравнение `type_id` с `Some(2)`, 2 в match-ветке без `type_id` рядом).
Подключить в tests/suites/gates.rs по алфавиту. Затем РУЧНАЯ мутация реального исходника: временно вписать в crates/trackly-app/src/services/cartridge_service.rs литерал `type_id = 2` обратно в SQL B12, прогнать app_gates — убедиться в красном [P1] с путём файла; откатить (`git checkout -- <файл>`), прогнать снова — зелёный. Оба вывода (сокращённо) положить в SUMMARY. Коммит `test(quick-261121-heh): гейт printer_identity_gate — один признак принтера в crates/*/src (INT-31)`.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_gates</automated>
  </verify>
  <done>app_gates зелёный (включая suite_wiring и device_type_label_gate); фикстура на каждое из P1–P5; три мутации реальных файлов краснят ожидаемое правило; ручная мутация cartridge_service.rs зафиксирована в SUMMARY (красный → откат → зелёный); git status после отката чист по src.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Фронт — единый модуль deviceTypeIds.ts, перевод F1–F7, JS-гейт с кросс-сверкой Rust</name>
  <files>ui/src/lib/deviceTypeIds.ts, ui/src/features/devices/PrinterMark.svelte, ui/src/features/devices/DeviceFormBody.svelte, ui/src/features/devices/DeviceFormModal.svelte, ui/src/features/devices/DeviceRowMenu.svelte, ui/src/features/groups/GroupsPage.svelte, ui/src/features/groups/GroupPrintersList.svelte, ui/src/features/printers/PrintersPage.svelte, ui/scripts/check-printer-identity.mjs, ui/package.json</files>
  <behavior>
    - Полный прогон по ui/src — зелёный; файлов, импортирующих `$lib/deviceTypeIds`, ≥ 7.
    - [J1] `const|let <ИМЯ с TYPE_ID> = 2` вне deviceTypeIds.ts — красный (кроме `ANCHOR_DEVICE_TYPE_ID = 1`: значение 1, правило про принтерное 2).
    - [J2] `(type_id|typeId)\s*[!=]==?\s*2\b`, обратный порядок, `type_id\s*:\s*2\b` вне модуля — красный.
    - [J3] сравнение с литералом 'Принтер' (`=== 'Принтер'`, `typeName === "Принтер"`, без регистра) — красный.
    - [J4] значение `PRINTER_TYPE_ID` в deviceTypeIds.ts не равно `pub const PRINTER_TYPE_ID: i64 = N;` из crates/trackly-core/src/domain/devices.rs (или Rust-строка не найдена) — красный.
    - [J5] антивакуум: импортёров модуля < 7 — красный.
    - --selftest: мутанты по РЕАЛЬНЫМ файлам с уникальными якорями (проверка «якорь ровно один» внутри мутатора): PrinterMark.svelte — вернуть `const PRINTER_TYPE_ID = 2;` → [J1]; PrintersPage.svelte — `result.typeId !== 2` → [J2]; deviceTypeIds.ts — значение 3 → [J4]; плюс синтетика для [J3] и [J5].
  </behavior>
  <action>
Создать ui/src/lib/deviceTypeIds.ts: doc-комментарий «зеркало trackly-core domain/devices.rs (INT-31); сверяется гейтом check-printer-identity [J4]», экспорт `DEVICE_TYPE_ID = 1`, `PRINTER_TYPE_ID = 2`, `isPrinterType(typeId: number | null | undefined): boolean`. Перевести F1–F7 по таблице инвентаря: имена идентификаторов на месте вызова сохранить (якоря существующих гейтов: `type_id: PRINTER_TYPE_ID` в GroupPrintersList — check-group-printer-search.mjs; `PRINTER_DEVICE_TYPE_ID` в DeviceRowMenu — check-device-row-menu.mjs, отсюда импорт с `as`). PrinterMark: серверный `isPrinter` остаётся приоритетным, fallback через `isPrinterType(typeId)`. Если какой-то существующий check-*.mjs покраснеет из-за переноса — поправить его якорь так, чтобы он проверял то же поведение, НЕ ослабляя (описать в SUMMARY).
Написать ui/scripts/check-printer-identity.mjs по образцу check-device-type-labels.mjs (`_strip.mjs` для комментариев, обход ui/src *.svelte|*.ts, исключить bindings*.ts и сам deviceTypeIds.ts из J1/J2, EXEMPT пуст — заведён с проверкой причины ≥20 символов), правила J1–J5, `--selftest`. Добавить в "lint" ui/package.json сразу после пары `check-device-type-labels.mjs`: `node scripts/check-printer-identity.mjs --selftest && node scripts/check-printer-identity.mjs`.
Проверить мутацией реального исходника вручную: временно вернуть `const PRINTER_TYPE_ID = 2;` в GroupsPage.svelte, запустить гейт — красный [J1]; откатить — зелёный; записать в SUMMARY. Коммит `fix(quick-261121-heh): фронт — один источник PRINTER_TYPE_ID, гейт check-printer-identity (INT-31)`.
Финальные гейты (по отдельности — `pnpm lint` не включает svelte-check): pnpm --dir ui lint, pnpm --dir ui check, cargo clippy --workspace --all-targets -- -D warnings, cargo fmt --all --check. Перед каждым коммитом — греп staged-диффа на реальные данные организации/ФИО (только вымышленные имена).
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && node ui/scripts/check-printer-identity.mjs --selftest && node ui/scripts/check-printer-identity.mjs && pnpm --dir ui lint && pnpm --dir ui check && pnpm --dir ui build && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check</automated>
  </verify>
  <done>grep `TYPE_ID\s*=\s*2` по ui/src находит только deviceTypeIds.ts; все пять гейт-команд зелёные; ручная мутация GroupsPage.svelte зафиксирована (красный → откат → зелёный); ui/dist пересобран.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| сотрудник (LAN) → request_printer_options / employee_devices_list | сотрудник видит только свою зону; признак принтера влияет на состав выдачи |
| портативная БД → справочник device_types | файл БД редактируем вне приложения; подписи типов — данные |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-heh-01 | Information disclosure | employee_device_service B6 / request_service B1 | mitigate | меняется только предикат типа; фильтры видимости сотрудника (`employee_device_ids`, зона АРМ) не трогать — проверяют существующие тесты app_server (request_printer_scope) и app_repairs (employee_devices_list) |
| T-heh-02 | Tampering | device_types.name (переименование в БД) | mitigate | признак по id, а не по имени (B1–B5); тест-проба переименования в трёх suite |
| T-heh-03 | Tampering | будущий код с новым литералом/структурным признаком | mitigate | гейты printer_identity_gate (P1–P5) и check-printer-identity (J1–J5), проверенные мутацией реальных файлов |
| T-heh-04 | Information disclosure | коммиты в публичный репо | mitigate | только вымышленные имена в тестах; греп staged-диффа перед каждым коммитом; pre-commit privacy-гейт |
</threat_model>

<verification>
- Инвентарь B1–B15, F1–F7 закрыт построчно (SUMMARY: таблица «место → было → стало»).
- grep по crates/*/src вне `#[cfg(test)]` и devices.rs: нет `type_id = 2`, `type_id == 2`, `type_id: 2`, `name = 'Принтер'`.
- Все затронутые suite зелёные: trackly-core unit; app_server, app_repairs, app_places, app_cartridges, app_reports, app_groups, app_devices, app_gates; infra_tests. Последовательно, один cargo за раз.
- Гейты: pnpm --dir ui lint, pnpm --dir ui check, cargo clippy -D warnings, cargo fmt --check.
- Две ручные мутации (Rust и JS) — красный/зелёный зафиксированы.
</verification>

<success_criteria>
- Один канонический признак (PRINTER_TYPE_ID из сида V001) на бэкенде и одна его зеркальная копия на фронте, сверяемая гейтом с Rust.
- NEW-5: is_printer у сотрудника и варианты принтеров заявки считаются по одному правилу; переименование типа ничего не ломает.
- Новый литерал/признак по имени/структурный фильтр краснит гейт.
- В SUMMARY: правки фикстур, следствие для дрейфа старых БД (принято, без миграции), выводы мутаций.
</success_criteria>

<output>
Create `.planning/quick/261121-heh-printer-identity/261121-heh-SUMMARY.md` when done
</output>
