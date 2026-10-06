---
phase: quick-261006-svt
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-app/tests/group_write_sites.rs
autonomous: true
requirements:
  - SEC-41-RESIDUAL-1
must_haves:
  truths:
    - "D-01: needle'ы разделены по вердикту: WRITE_SITE_CALLS (требует release: три существующих + update_in_tx) и BIRTH_EXEMPT_CALLS (рождение устройства: create_in_tx, clone_device_in_tx) — две константы, два реестра с точным числом, обоснование освобождения комментарием у константы"
    - "D-02: каждый write-site update_in_tx обязан иметь рядом release_*-helper (после) ЛИБО маркер guard'а S1 locked_group_for_device_in_tx( вместе с AppError::Validation (до); без того и без другого гейт краснеет"
    - "D-03: ловушка имён create_in_tx решена комбинацией (а)+(б): needle привязан к получателю с границей идентификатора (repo.create_in_tx( не ловит printer_repo.create_in_tx() и скан освобождённых needle'ов исключает crates/trackly-infra/src/repos/; вариант (в) не используется"
    - "D-04: множество методов devices_sqlite.rs, пишущих devices.place_id, РАВНО замороженному классифицированному реестру из 8 имён (6 *_in_tx + не-tx create/update); новый метод краснит гейт, даже если его никто не вызывает; не-tx пара помечена «известна, продакшн-вызовов нет, needle по имени невозможен»"
    - "D-05: каскад картриджа (cartridges_sqlite.rs:651, сырой SQL) вне области — зафиксирован комментарием как известный третий класс write-site'ов, покрытый S9, но не инвентарь-гейтами; новых гейтов для сырого SQL нет"
    - "D-06: комментарий «ровно тремя методами» заменён фактической картиной (требует release / освобождён-рождение / не-tx пара без вызовов / каскад картриджа)"
    - "Мутационная проверка: якорь реальной мутации утверждён как уникальный (matches().count()==1); мутированный текст краснит проверку, немутированный проходит (не вакуумно)"
    - "Продакшн-код не изменён; правится только crates/trackly-app/tests/group_write_sites.rs"
  artifacts:
    - path: "crates/trackly-app/tests/group_write_sites.rs"
      provides: "Полный инвентарь-гейт write-site'ов devices.place_id с вердиктами и мутационными самопроверками"
      contains: "BIRTH_EXEMPT_CALLS"
  key_links:
    - from: "PLACE_WRITER_METHODS (замороженный реестр)"
      to: "WRITE_SITE_CALLS / BIRTH_EXEMPT_CALLS"
      via: "гейт согласованности: множества имён вердиктов равны множествам needle'ов"
      pattern: "PLACE_WRITER_METHODS"
    - from: "uncovered_guarded_sites"
      to: "crates/trackly-app/src/services/device_service.rs"
      via: "include_str! + мутация копии текста"
      pattern: "uncovered_guarded_sites"
---

<objective>
Расширить инвентарь гейта write-site'ов `devices.place_id` в `crates/trackly-app/tests/group_write_sites.rs` до полного набора методов `SqliteDeviceRepository`, исправить неверный комментарий «ровно тремя методами», закодировать для каждого метода вердикт (требует release / освобождён) и доказать мутацией, что гейт краснеет.

Purpose: закрыть residual item 1 из `41-SECURITY.md`. Сейчас дыры в безопасности нет (`update_in_tx` стоит за guard'ом S1), но будущий сервис, переносящий устройство через `repo.update_in_tx`, не поймается ни одним гейтом. Это тот же класс дефекта, что урок фазы 40.1 («инвентаризация ОТ СЕРВЕРНЫХ МУТАЦИЙ»).
Output: один изменённый тест-файл; продакшн-код не трогается. Если кажется, что нужна правка продакшна, это выход за область: остановиться и сообщить.

Нумерация решений оркестратора: D-01..D-06 = пункты 1..6 из `decisions_locked_by_orchestrator`.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@.planning/phases/41-groups-model-and-editor/41-SECURITY.md

<interfaces>
Текущее состояние гейта в `crates/trackly-app/tests/group_write_sites.rs` (строки ~1578-1780), только то, что нужно:
- `WRITE_SITE_CALLS: [&str; 3]`, `RELEASE_CALLS: [&str; 2]`, `WRITE_SITE_REGISTRY: [(&str, usize); 3]` (act_service.rs 8, place_service.rs 1, group_place.rs 1; пути от корня репозитория).
- `fn count_calls(src: &str, needles: &[&str]) -> usize` — построчно: отрезает хвост после `//`, пропускает строки с `fn `, считает `matches(needle)` (сейчас БЕЗ проверки границы слева).
- `fn rust_sources_under_crates() -> Vec<(String, String)>` — все `.rs` под `crates/*/src`, путь относительно корня с `/`.
- Тесты: `write_site_gate_counter_selftest`, `write_site_gate_act_service_every_site_has_release` (gate 1, читает `include_str!("../src/services/act_service.rs")`, ожидает 8/8, окно WINDOW=14 строк ПОСЛЕ write-site'а), `write_site_gate_no_unregistered_place_writers` (gate 2, inline-цикл по реестру).
- Путь `include_str!` относительно `crates/trackly-app/tests/`: сервисы — `../src/services/<f>.rs`; репозиторий устройств — `../../trackly-infra/src/repos/devices_sqlite.rs`.

Факты кода (уже проверены оркестратором и перепроверены планировщиком, не искать заново):
- `device_service.rs`: единственный вызов `repo.update_in_tx(` на :741; guard S1 на :727-739 (`groups.locked_group_for_device_in_tx(&tx, id)?` на :729, `AppError::Validation` на :730, то есть ДО write-site'а, 11-12 строк назад). `locked_group_for_device_in_tx(` в этом файле встречается ровно один раз. `release_*` в окне 742-755 нет (ближайший на :854, delete_soft).
- `repo.create_in_tx(` для устройств: `device_service.rs:333` и `:1717`. На :268 стоит `printer_repo.create_in_tx(` — подстрока `repo.create_in_tx(` внутри `printer_repo.` (граница слева нужна именно для этого). `printer_service.rs:300` и ~12 вызовов в `#[cfg(test)]` внутри `printers_sqlite.rs` — принтеры, `place_id` не пишут.
- `devices_repo.clone_device_in_tx(` — только `act_service.rs:526`.
- `devices_sqlite.rs` (1468 строк): нет `#[cfg(test)]`-модуля; методы с `INSERT INTO devices` / `UPDATE devices` и `place_id` в теле: `create_in_tx`, `update_in_tx`, `update_status_and_place_in_tx`, `update_full_in_tx`, `restore_from_snapshot_in_tx`, `clone_device_in_tx`, трейтовые `create`, `update`. `delete_soft_in_tx`/`delete_soft` содержат `UPDATE devices`, но `place_id` не упоминают. Остальные методы (`get`, `list`, `search_fts`, `list_grouped`, ...) — только чтение.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Разделить needle'ы по вердикту, граница идентификатора, реестр освобождённых (рождение), правка комментария</name>
  <files>crates/trackly-app/tests/group_write_sites.rs</files>
  <behavior>
    - count_calls: `printer_repo.create_in_tx(` НЕ считается needle'ом `repo.create_in_tx(`; `x.repo.create_in_tx(` и `= repo.create_in_tx(` считаются; `foo_update_in_tx(` НЕ считается needle'ом `update_in_tx(`; `.update_in_tx(` и `update_in_tx(` в начале строки считаются.
    - Существующие ожидания selftest (2 write-site'а в fixture, 2 release-вызова, вызов без комментария = 1, пустая строка = 0) и gate 1 (8/8 в act_service.rs) не меняют смысла.
    - Birth-гейт: файл вне `crates/trackly-infra/src/repos/` с `repo.create_in_tx(` или `clone_device_in_tx(` вне реестра, либо с иным числом вызовов, даёт нарушение; вызовы внутри `crates/trackly-infra/src/repos/` игнорируются.
  </behavior>
  <action>
В `group_write_sites.rs`, блок «Plan 41-16: счётный гейт» (после тестов S1-S9, строки ~1578+):

1. Усилить `count_calls` проверкой границы идентификатора слева: перебирать вхождения needle через `match_indices` и принимать вхождение, только если предыдущий символ в `code` отсутствует или не является буквой, цифрой либо `_`. Это нужно для D-03 (отличить `repo.` от `printer_repo.`) и не меняет счёт реальных вызовов (слева всегда `.`, пробел или начало строки). Остальная семантика (срез `//`, пропуск строк с `fn `) не меняется. Обновить докстроку.

2. Расщепить константы по вердикту (D-01):
   - `WRITE_SITE_CALLS` стал `[&str; 4]`: прежние три плюс `"update_in_tx("`. Докстрока: «требует release ИЛИ guard, отклоняющий перенос». Отметить, что `update_in_tx(` уникален по имени среди репозиториев на сегодня; будущий одноимённый метод другого репозитория сделает гейт красным, и это сознательно безопасное направление (решается уточнением реестра, не отключением).
   - `BIRTH_EXEMPT_CALLS: [&str; 2]` = `"repo.create_in_tx("` и `"clone_device_in_tx("`. Комментарий прямо у константы: это РОЖДЕНИЕ устройства (в т.ч. клона), новое устройство ещё не член группы, выводить из состава некого (сценарий S10 в шапке файла), release не нужен и не должен добавляться. Объяснить ловушку имён: `create_in_tx(` носит и `SqlitePrinterRepository` (`printers_sqlite.rs`, `printer_service.rs`, `printer_repo.create_in_tx(` в `device_service.rs`), поэтому выбраны способы (а) needle привязан к получателю `repo.` с границей слева и (б) скан этого набора исключает `crates/trackly-infra/src/repos/` (константа `BIRTH_SCAN_SKIP_PREFIX`); вариант (в) (внести `printers_sqlite.rs` в реестр) отвергнут: число вызовов в его тест-модуле меняется от любой несвязанной правки, гейт стал бы шумным и его бы отключили. Оговорка: если сервис назовёт репозиторий устройств иначе, чем `repo`, birth-гейт его не увидит; это допустимо, потому что вердикт «рождение» не требует release, а полноту «от репозитория» держит гейт из Task 2.
   - `WRITE_SITE_REGISTRY` стал `[(&str, usize); 4]`: прежние три плюс `("crates/trackly-app/src/services/device_service.rs", 1)`. Дополнить докстроку: device_service.rs: 1 = S1 `update_in_tx`, прикрыт guard'ом (см. Task 2).
   - `BIRTH_EXEMPT_REGISTRY: [(&str, usize); 2]`: `device_service.rs` 2 (одиночное создание и массовое), `act_service.rs` 1 (клон). Точные числа, как в `WRITE_SITE_REGISTRY`.

3. Вынести проверку реестра из тела gate 2 в чистую функцию `registry_violations(sources: &[(String, String)], needles: &[&str], registry: &[(&str, usize)], skip_prefix: Option<&str>) -> Vec<String>`. Она воспроизводит текущую логику gate 2 без изменения условий: файл с вызовами вне реестра даёт нарушение «новое место записи ...», совпадающий файл с иным числом даёт нарушение «число изменилось», файл реестра без вызовов даёт нарушение «больше не содержит». Файлы, путь которых начинается с `skip_prefix`, пропускаются. Gate 2 `write_site_gate_no_unregistered_place_writers` вызывает её с `WRITE_SITE_CALLS`, `WRITE_SITE_REGISTRY`, `None`, сохраняет проверку `sources.len() > 50` и делает `assert!(violations.is_empty(), "{}", violations.join("\n"))`. Сообщения нарушений сохранить по смыслу («добавь кейс release в group_write_sites.rs ...»).

4. Новый тест `write_site_gate_birth_exemptions_are_inventoried`: те же `rust_sources_under_crates()`, `registry_violations(..., &BIRTH_EXEMPT_CALLS, &BIRTH_EXEMPT_REGISTRY, Some(BIRTH_SCAN_SKIP_PREFIX))`, плюс assert, что множества `WRITE_SITE_CALLS` и `BIRTH_EXEMPT_CALLS` не пересекаются (освобождение закодировано, а не описано). Сообщение нарушения для birth-набора: «новое место РОЖДЕНИЯ устройства — подтверди, что оно не переносит существующее устройство, и внеси в BIRTH_EXEMPT_REGISTRY».

5. Дополнить `write_site_gate_counter_selftest` отдельными ассертами на границу слева (см. behavior): `count_calls("let _ = printer_repo.create_in_tx(a);\n", &["repo.create_in_tx("])` равно 0, `count_calls("let id = repo.create_in_tx(a);\n", ...)` равно 1, `count_calls("foo_update_in_tx(x);\nr.update_in_tx(x);\n", &["update_in_tx("])` равно 1. Существующие ассерты не менять.

6. Заменить комментарий строк ~1584-1587 («ровно тремя методами ...»): описать фактическую картину по D-06. Методы, пишущие `devices.place_id` в `SqliteDeviceRepository` (всего 8): ТРЕБУЮТ release или guard: `update_status_and_place_in_tx`, `update_full_in_tx`, `restore_from_snapshot_in_tx`, `update_in_tx` (последний закрыт guard'ом S1 в `DeviceService::update`, он ОТКЛОНЯЕТ перенос, а не выводит из состава); ОСВОБОЖДЕНЫ как рождение: `create_in_tx`, `clone_device_in_tx`; не-tx пара трейта `create`/`update` — известна, продакшн-вызовов нет (сервисы используют только `*_in_tx`), needle по имени невозможен (`update(`/`create(` слишком общие), держится только реестром «от репозитория» из Task 2. Добавить абзац про каскад картриджа по D-05: `cartridges_sqlite.rs:651` (`UPDATE devices SET place_id=?1 ...`, сырой SQL) — известный третий класс write-site'ов, покрыт сценарием S9 выше, но НЕ входит ни в один инвентарь-гейт; гейт для сырого SQL не строился, потому что в `report_service.rs` тестах есть `INSERT INTO devices` и гейт был бы шумным. Сохранить абзац «Как добавить новый write-site», скорректировав «трёх методов» на «методов из WRITE_SITE_CALLS», и сослаться на вердикты.

Приватность: только вымышленные данные; реальных названий/ФИО не вводить.
  </action>
  <verify>
    <automated>cargo test -p trackly-app --test group_write_sites write_site_gate 2>&1 | tail -30</automated>
  </verify>
  <done>Все `write_site_gate_*` зелёные: gate 1 по-прежнему 8/8, gate 2 принимает 4-й реестр (device_service.rs: 1), birth-гейт зелёный на реальных исходниках с точными числами (device_service.rs 2, act_service.rs 1), selftest проверяет границу слева; в файле нет текста «ровно тремя методами»; констант две (WRITE_SITE_CALLS и BIRTH_EXEMPT_CALLS) с обоснованием; продакшн-файлы не менялись (`git diff --stat` показывает только group_write_sites.rs). Если счёт birth-needle'ов на реальных исходниках не равен 2 и 1, остановиться и сверить с интерфейсами выше, не подгонять числа.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Гейт парности update_in_tx (release ИЛИ guard S1) и гейт полноты «от репозитория» с замороженным реестром из 8 имён</name>
  <files>crates/trackly-app/tests/group_write_sites.rs</files>
  <behavior>
    - `uncovered_guarded_sites(text)` возвращает 1-based номера строк `update_in_tx(`-сайтов, у которых нет ни release-helper'а в 14 строках после (до следующего write-site'а), ни guard'а в 20 строках до (одновременно `locked_group_for_device_in_tx(` и `AppError::Validation`, оба вне комментариев).
    - На реальном `device_service.rs` результат пуст; сайт один.
    - `place_writing_methods(text)` на реальном `devices_sqlite.rs` возвращает ровно 8 имён: `clone_device_in_tx`, `create`, `create_in_tx`, `restore_from_snapshot_in_tx`, `update`, `update_full_in_tx`, `update_in_tx`, `update_status_and_place_in_tx` (отсортированы).
    - Метод, читающий `d.place_id`, и метод с `UPDATE devices SET notes = ...` без `place_id` не попадают в результат.
  </behavior>
  <action>
В том же блоке гейта (после birth-гейта из Task 1):

1. Чистая функция `uncovered_guarded_sites(src: &str) -> Vec<usize>` (принимает ТЕКСТ, не читает файл, чтобы мутационные тесты Task 3 могли вызывать её на мутированной копии). Для каждой строки (срез `//`, пропуск строк с `fn `), где `count_calls(code, &["update_in_tx("]) > 0`: сайт покрыт, если (а) среди строк окна ПОСЛЕ (константа `RELEASE_WINDOW = 14`, окно обрезается на ближайшей следующей строке с `WRITE_SITE_CALLS`, как в gate 1) есть любой из `RELEASE_CALLS` ИЛИ (б) среди строк окна ДО (`GUARD_WINDOW = 20`, тоже без комментариев) есть и `locked_group_for_device_in_tx(`, и `AppError::Validation` (guard отклоняет перенос, D-02). Возвращать номера непокрытых строк. Докстрока: guard стоит ДО write-site'а, release ПОСЛЕ, поэтому окна разнонаправленные; на сайте `device_service.rs:741` guard лежит на :729-730.

2. Тест `write_site_gate_update_in_tx_released_or_guarded`: по всем `rust_sources_under_crates()` для каждого файла с хотя бы одним `update_in_tx(`-сайтом вызвать `uncovered_guarded_sites`; нарушения собрать в сообщение «write-site update_in_tx в {файл}:{строка} без release-helper'а и без guard'а S1 — перенос существующего устройства обойдёт членство в группе (D-22/GRP-07)». Убедиться, что суммарно просмотрен хотя бы один сайт (иначе гейт вакуумен). Точное число и файл фиксирует `WRITE_SITE_REGISTRY` (device_service.rs: 1), здесь не дублировать.

3. Реестр «от репозитория» (D-04). Определить `enum PlaceWriterVerdict { ReleaseRequired, BirthExempt, NoProductionCallers }` и замороженную константу `PLACE_WRITER_METHODS: [(&str, PlaceWriterVerdict); 8]`:
   - `ReleaseRequired`: `update_status_and_place_in_tx`, `update_full_in_tx`, `restore_from_snapshot_in_tx`, `update_in_tx`;
   - `BirthExempt`: `create_in_tx`, `clone_device_in_tx`;
   - `NoProductionCallers`: `create`, `update` (не-tx методы трейта `DeviceRepository`).
   Комментарий к константе: «известна, продакшн-вызовов нет, needle по имени невозможен (`update(`/`create(` — слишком общие)»; новая запись добавляется только вместе с вердиктом и, для ReleaseRequired/BirthExempt, с needle'ом в соответствующей константе выше.

4. Чистая функция `place_writing_methods(src: &str) -> Vec<String>`: взять текст до первого `#[cfg(test)]` (если есть), пройти по строкам со срезом `//`; строка с `fn ` открывает новый метод (имя = символы идентификатора после `fn `, это корректно разбирает `fn from_row<'a>(`); тело метода = строки до следующей строки с `fn `. Метод пишет `place_id`, если его тело содержит `INSERT INTO devices` или `UPDATE devices` И содержит `place_id`. Вернуть имена, отсортировав. Докстрока: эвристика намеренно консервативна в сторону ложных срабатываний (метод с `UPDATE devices ... WHERE place_id` покраснеет и потребует явной классификации), пропуск невозможен, пока запись идёт SQL-строкой по `devices`; запись через хелпер в другом файле эта проверка не увидит (граница честно названа в комментарии).

5. Тест `write_site_gate_repo_place_writers_match_frozen_registry`: `include_str!("../../trackly-infra/src/repos/devices_sqlite.rs")`; сравнить `place_writing_methods` с отсортированными именами `PLACE_WRITER_METHODS`; при расхождении panic с двумя списками «новые: ...» и «исчезли: ...» и подсказкой «классифицируй метод: ReleaseRequired / BirthExempt / NoProductionCallers». Добавить assert `!methods.is_empty()` (не вакуумно). Вынести сравнение в функцию `place_writer_diff(src: &str) -> (Vec<String>, Vec<String>)` (новые, исчезли), её же используют мутационные тесты Task 3.

6. Тест `write_site_gate_verdicts_match_needles`: связывает три слоя. Имена с `ReleaseRequired` + `(` равны множеству `WRITE_SITE_CALLS`; имена с `BirthExempt` равны множеству `BIRTH_EXEMPT_CALLS` после снятия префикса `repo.` и суффикса `(`; имена с `NoProductionCallers` равны ровно `{create, update}`. Так новый метод с вердиктом, но без needle'а, и needle без вердикта краснят гейт.

Приватность: фикстуры только вымышленные/технические.
  </action>
  <verify>
    <automated>cargo test -p trackly-app --test group_write_sites write_site_gate 2>&1 | tail -30</automated>
  </verify>
  <done>Три новых теста зелёны на реальных исходниках: update_in_tx-гейт видит один сайт и он покрыт guard'ом; реестр «от репозитория» совпадает с 8 именами; согласованность вердиктов и needle'ов выполняется. Если `place_writing_methods` на реальном `devices_sqlite.rs` возвращает больше или меньше 8 имён, не менять реестр под результат, а проверить эвристику (ложное срабатывание значит уточнить разбор, не расширять реестр).</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Мутационные самопроверки с уникальными якорями, финальный прогон, форматирование</name>
  <files>crates/trackly-app/tests/group_write_sites.rs</files>
  <behavior>
    - Баз-кейс (немутированный текст проходит) и мутант (краснит) для каждой из трёх проверок: парность update_in_tx, полнота «от репозитория», birth-гейт и ловушка имён.
    - Каждая реальная мутация через `replace` предваряется assert'ом `matches(anchor).count() == 1` с внятным сообщением.
  </behavior>
  <action>
Добавить в тот же файл три теста-самопроверки (имена начинаются с `write_site_gate_mutation_`), все работают на копиях текста в памяти, файлы не пишут. Образец стиля: `write_site_gate_counter_selftest`. Не вводить зависимостей.

1. `write_site_gate_mutation_update_in_tx_without_guard_or_release`:
   - Фикстуры-расходящиеся: (i) голый `let a = repo.update_in_tx(tx, 1);` без окружения даёт `uncovered_guarded_sites` с номером этой строки; (ii) тот же сайт, за которым через пару строк идёт `release_device_in_tx(tx, g);`, даёт пусто; (iii) перед сайтом строки с `groups.locked_group_for_device_in_tx(&tx, id)?` и `AppError::Validation {` дают пусто; (iv) маркер guard'а только в `// комментарии` перед сайтом даёт непокрыто (комментарии не считаются).
   - Реальный текст `include_str!("../src/services/device_service.rs")`: базовая линия: `uncovered_guarded_sites` пуст и сайт ровно один (номер сайта получить из `count_calls`-перебора строк, не хардкодить). Мутация: якорь `groups.locked_group_for_device_in_tx(&tx, id)?`; перед заменой assert `src.matches(anchor).count() == 1` с сообщением «якорь мутации должен быть уникален (урок mutation_test_anchor_must_be_unique)»; заменить на `groups.some_other_lookup_in_tx(&tx, id)?`. Ожидание: результат равен `vec![номер_сайта]`. Это моделирует целевой дефект: write-site `update_in_tx` без release и без маркера guard'а.

2. `write_site_gate_mutation_new_repo_method_writing_place_id`:
   - Реальный `devices_sqlite.rs`: базовая линия: `place_writer_diff` возвращает два пустых списка.
   - Мутация «новый метод»: дописать в конец копии метод `pub fn move_device_in_tx(` с телом `"UPDATE devices SET place_id = ?1 WHERE id = ?2"`; ожидание: `новые == ["move_device_in_tx"]`, `исчезли` пусто. Якорь не нужен, это добавление текста.
   - Мутация «пропал write»: якорь `status_id = ?1, place_id = ?2,` (единственное вхождение в файле, подтвердить assert'ом на `matches().count() == 1`), заменить на `status_id = ?1,`; ожидание: `исчезли == ["update_status_and_place_in_tx"]`.
   - Негативные фикстуры: метод с `SELECT d.place_id FROM devices d` и метод с `UPDATE devices SET notes = ?1` (без `place_id`) не попадают в результат `place_writing_methods`.

3. `write_site_gate_mutation_birth_inventory_name_trap`:
   - Через `registry_violations(..., &BIRTH_EXEMPT_CALLS, &BIRTH_EXEMPT_REGISTRY, Some(BIRTH_SCAN_SKIP_PREFIX))` на синтетическом списке источников: реальные тексты `device_service.rs` и `act_service.rs` под их реальными путями реестра. Базовая линия: нарушений нет. Мутация A: к копии `device_service.rs` дописать строку `let _ = repo.create_in_tx(&tx, &domain_new, 0);` даёт ровно одно нарушение, упоминающее `device_service.rs`. Мутация B (ловушка имён): дописать `let _ = printer_repo.create_in_tx(&tx, &p, 0);` даёт нарушений нет (принтерный репозиторий не шумит). Мутация C: добавить третий источник под путём `crates/trackly-infra/src/repos/printers_sqlite.rs` с несколькими `repo.create_in_tx(` даёт нарушений нет (исключён префиксом). Мутация D: тот же текст под путём `crates/trackly-app/src/services/some_new_service.rs` даёт нарушение «новое место рождения».

Финальные шаги:
   - Форматирование ТОЛЬКО затронутого файла (в проекте есть предсуществующий fmt-drift, чужое не чинить): определить edition из `Cargo.toml` и выполнить `rustfmt --edition <edition> crates/trackly-app/tests/group_write_sites.rs`; затем `git diff --stat` должен показать только этот файл.
   - Проверка приватности ДО коммита: `git diff -- crates/trackly-app/tests/group_write_sites.rs | grep '^+'` просмотреть на реальные названия/ФИО/реквизиты (ожидается: только технические идентификаторы и вымышленные имена). Хук `.githooks/pre-commit` прогонится сам при коммите.
   - Коммит делает оркестратор quick-воркфлоу; этот план шаг коммита не включает.
  </action>
  <verify>
    <automated>cargo test -p trackly-app --test group_write_sites 2>&1 | tail -40</automated>
  </verify>
  <done>Весь тест-таргет `group_write_sites` зелёный (сценарии S1-S9 плюс все `write_site_gate_*` и `write_site_gate_mutation_*`; полный `--workspace` не запускать, он около 80 минут; если линковка падает с exit 69, добавить префикс `DEVELOPER_DIR=/Library/Developer/CommandLineTools`). Каждый мутант краснит свою проверку, каждая немутированная линия проходит; якоря реальных мутаций подтверждены как уникальные assert'ами. Файл отформатирован rustfmt, `git diff --stat` затрагивает только `crates/trackly-app/tests/group_write_sites.rs`, продакшн-код не изменён.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| тест-код -> исходники репозитория | Гейт читает исходники и не исполняет их; продакшн-поведение не меняется |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-261006-01 | E | будущий сервис переносит устройство через `repo.update_in_tx` в обход членства группы | mitigate | `update_in_tx(` в `WRITE_SITE_CALLS` + реестр с точным числом + гейт парности «release ИЛИ guard S1» |
| T-261006-02 | T | новый метод репозитория пишет `devices.place_id`, но ни один needle его не знает | mitigate | гейт «от репозитория» сравнивает множество методов с замороженным реестром из 8 имён |
| T-261006-03 | R | гейт позеленел вакуумно (якорь мутации неуникален или базовая линия не расходится с мутантом) | mitigate | assert уникальности якоря, базовая линия + мутант в каждом мутационном тесте, проверка `len > 50` и непустоты просмотра |
| T-261006-04 | I | утечка реальных данных организации через фикстуры | mitigate | только технические идентификаторы и вымышленные имена; grep диффа до коммита; pre-commit хук приватности |
| T-261006-05 | T | каскад картриджа (сырой SQL) не покрыт инвентарь-гейтом | accept | покрыт сценарием S9; гейт для сырого SQL был бы шумным (`INSERT INTO devices` в тестах `report_service.rs`); зафиксировано комментарием (D-05) |
</threat_model>

<verification>
- `cargo test -p trackly-app --test group_write_sites` зелёный целиком (один cargo за раз).
- `git diff --stat` затрагивает только `crates/trackly-app/tests/group_write_sites.rs`.
- В файле нет подстроки «ровно тремя методами».
- Source audit: GOAL (полный набор методов + вердикты + мутация) покрыт задачами 1-3; CONTEXT D-01..D-06 покрыты (D-01/D-03/D-06 в Task 1, D-02/D-04 в Task 2, D-05 в Task 1 п.6, мутационное требование в Task 3); отложенных идей нет.
</verification>

<success_criteria>
- Для каждого из 8 методов `SqliteDeviceRepository`, пишущих `devices.place_id`, в коде закодирован вердикт: требует release/guard, освобождён как рождение, либо не-tx без вызовов.
- Новый write-site `update_in_tx` без release и без guard'а краснит гейт; новый метод репозитория, пишущий `place_id`, краснит гейт; новое место рождения краснит birth-гейт; принтерные `create_in_tx` не шумят.
- Мутационные самопроверки доказывают это на копиях реальных исходников с уникальными якорями, и немутированные линии проходят.
- Продакшн-код и поведение сервисов не изменены.
</success_criteria>

<output>
Создать `.planning/quick/261006-svt-write-site-devices-place-id-group-write-/261006-svt-SUMMARY.md` по завершении (без реальных данных организации и людей).
</output>
