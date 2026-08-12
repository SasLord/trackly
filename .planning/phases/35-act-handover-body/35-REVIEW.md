---
phase: 35-act-handover-body
reviewed: 2026-08-12T00:35:00Z
depth: standard
files_reviewed: 10
files_reviewed_list:
  - crates/trackly-app/src/pdf/html_templates.rs
  - crates/trackly-app/src/services/template_service.rs
  - crates/trackly-app/templates/_legacy_defaults/v22/act_acceptance.html
  - crates/trackly-app/templates/_legacy_defaults/v22/act_handover.html
  - crates/trackly-app/templates/act_acceptance.html
  - crates/trackly-app/templates/act_handover.html
  - crates/trackly-app/tests/acts_e2e_smoke.rs
  - crates/trackly-app/tests/html_act_render.rs
  - crates/trackly-app/tests/html_field_row_underline_gate.rs
  - crates/trackly-app/tests/pdf_render_act.rs
findings:
  critical: 0
  warning: 9
  info: 5
  total: 14
status: issues_found
---

# Phase 35: Code Review Report (re-review после gap closure)

**Reviewed:** 2026-08-12T00:35:00Z
**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Summary

Повторный ревью Фазы 35 после плана 35-06. Проверено фактическое состояние кода
(не заявления плана): **все четыре предыдущие находки закрыты по-настоящему** —
см. раздел «Проверка закрытия предыдущих находок».

Дополнительно прогнаны все затронутые тест-бинарники — зелёные:
`--lib pdf::html_templates` (13), `--lib services::template_service` (15),
`--test pdf_render_act` (13), `--test html_field_row_underline_gate` (2),
`--test html_act_render` (8), `--test acts_e2e_smoke` (4),
`--test templates_seed` (4), `--test templates_status` (11),
`--test html_report_render` (3), `--test html_header_parity` (5),
`--test html_page_parity` (1). Компиляция/тесты — не доказательство
корректности печатного документа, поэтому основная часть находок ниже получена
чтением шаблонов и фактическим рендером через MiniJinja.

**Приватность:** нарушений нет. Все реквизиты в фикстурах и demo-контексте —
синтетические (последовательные цифры, `ООО Ромашка`/`ООО Паритет`/
`ООО Демо Организация`, номера вида `+7 495 000-00-00`), ФИО — вымышленные
(«Иванов И.И.», «Морозов М.М.», «Выдалов В.В.»). Реального названия
организации, реквизитов и реальных ФИО в проверенных файлах нет.

**Критических находок нет.** Ключевые остающиеся риски: (1) пропущенный снимок
промежуточного тела в `_legacy_defaults` (уже трекается, подтверждаю механизм и
приложил результат независимой проверки релизных тегов и локальных
материализованных копий); (2) `white-space: nowrap` на новом печатном ФИО —
класс дефекта «ФИО обрезалось», который в этом проекте уже случался;
(3) дублирование имени устройства и безусловное «Сроком до» — качество
печатного юридического документа; (4) preview редактора печатает литеральное
`none` вместо суффикса (подтверждено фактическим рендером).

## Проверка закрытия предыдущих находок

| Находка | Статус | Доказательство |
|---|---|---|
| CR-01 (multi-device `.device-block` без имени устройства) | **закрыта** | `d274e6b` снял гейт `{%- if act.items \| length == 1 %}`; строка «было получено устройство: {{ item.name }}» теперь безусловна (`act_handover.html:142`). Проверено фактическим рендером на N=3: каждый блок содержит своё имя, включая блок без опциональных полей. Регресс-тест `render_handover_multi_device_fields_attributable_to_own_device` (`pdf_render_act.rs:345-453`) не вакуозен: он разбивает HTML по `<div class="device-block">`, проверяет co-location и отсутствие чужих значений; на прежнем гейтированном шаблоне упал бы на первом же `block.contains(names[i])`. |
| WR-01 (нет теста на `bodies.get(2)`/v22) | **закрыта** | `upgrade_replaces_v22_legacy_default_with_current_bundled_body` (`html_templates.rs:476-525`) тянет индекс `2`, имеет анти-вакуозный guard `assert_ne!(v22_body, current)`, проходит. Независимо проверено: `_legacy_defaults/v22/act_{handover,acceptance}.html` байт-в-байт равны телам на `e0d2dca^` (pre-Phase-35), т.е. снимок взят ДО правок. |
| WR-02 (вакуозные `"Выдал"`/`"Получил"` на фоне фикстурных ФИО) | **закрыта** | `html_act_render.rs:188` теперь ассертит метки с двоеточием (`"Выдал:"`, `"Получил:"`), которые не могут совпасть с «Выдалов В.В.», плюс отдельный явный ассерт на печатное ФИО (`html_act_render.rs:198-202`). |
| IN-01 (гейт подчёркиваний только для `act_handover.html`) | **закрыта** | `html_field_row_underline_gate.rs:97-114` добавляет эквивалентный гейт для `act_acceptance.html` (ровно один легитимный `border-bottom`), тест проходит. |

## Structural Findings (fallow)

Структурный пре-проход не передан (`<structural_findings>` в задании
отсутствует) — раздел оставлен пустым намеренно, чтобы narrative-находки ниже
не выдавались за структурный субстрат.

## Narrative Findings (AI reviewer)

## Critical Issues

Не найдено. Ни один из проверенных путей не даёт некорректного поведения,
утечки данных или уязвимости: в обоих актах не осталось ни одного `| safe`
(единственные санкционированные `| safe` живут в `_header.html` и относятся к
серверно-собранным `logo_data_uri`/`org.full_name`), autoescape включён
(`build_safe_html_env`), новых ключей контекста, кроме `act.giver_name`, не
добавлено, а `act.giver_name` присутствует во всех путях рендера
(`act_service::render_pdf` — единственный путь для `act_handover.html`) и в
preview-контексте (`demo_context_for_kind`, добавлен в 35-01), так что
`UndefinedBehavior::Strict` не даёт краша.

## Warnings

### WR-01: Промежуточное тело `act_handover.html` (35-02/35-03) не снято в `_legacy_defaults/v23` — установки на нём навсегда теряют авто-апгрейд

**Severity:** WARNING (уже трекается; подтверждаю независимо)
**File:** `crates/trackly-app/src/pdf/html_templates.rs:57-63, 75-105`, `crates/trackly-app/templates/act_handover.html:140-142`

**Issue:** Doc-comment самого модуля объявляет снимок PRE-CHANGE тела
обязательным при *каждом* изменении `DEFAULT_HTML_TEMPLATES`. План 35-06
(`d274e6b`) изменил тело `act_handover.html` во второй раз внутри фазы
(снят гейт `length == 1`), но `_legacy_defaults/` по-прежнему содержит только
`v20 v21 v22`. Для установки, чей on-disk файл равен промежуточному телу
(коммиты `3904da9`…`bbfed54`), `upgrade_untouched_defaults_on_startup`
(`html_templates.rs:229-236`) уйдёт в ветку «user-customized» и **никогда** не
доставит исправление CR-01; `build_templates_status`
(`tauri_cmds/settings_org.rs:316-330`) при этом покажет файл как `Customized`,
т.е. диагностика будет вводить в заблуждение. Ни один из трёх структурных
тестов апгрейда этого не поймает — они итерируются по зарегистрированным
снимкам, а не по истории.

**Независимо проверенные смягчающие факты:**
- ни один релизный тег не несёт промежуточное тело: `v1.2, v1.3, v1.3.0,
  v1.3.1, v1.3.2` — все три шаблона байт-в-байт равны снимку `v21`
  (т.е. даже тело Phase 34, зарегистрированное как `v22`, ещё не выпускалось);
- локальные материализованные копии на этой машине
  (`target/debug/templates/*.html`, mtime 2026-08-12 06:07) байт-в-байт равны
  **текущим** bundled-телам, т.е. здесь ничего не залипло. (Замечу: тезис
  «материализованных копий на машине не осталось» неточен — каталог
  существует; просто его содержимое актуально.)

**Остаточный риск:** любая машина (в частности Windows-бокс для UAT), где
приложение запускалось со сборки между `3904da9` и `d274e6b`, залипнет на теле
без имён устройств — то есть UAT «фикса CR-01» там покажет старое поведение и
это будет выглядеть как неработающий фикс.

**Fix:**
```rust
// 1) templates/_legacy_defaults/v23/act_handover.html  <- тело на коммите bbfed54
// 2) html_templates.rs
(
    "act_handover.html",
    &[
        include_str!("../../templates/_legacy_defaults/v20/act_handover.html"),
        include_str!("../../templates/_legacy_defaults/v21/act_handover.html"),
        include_str!("../../templates/_legacy_defaults/v22/act_handover.html"),
        include_str!("../../templates/_legacy_defaults/v23/act_handover.html"),
    ],
),
```
Либо — если решено снимок не делать — зафиксировать в doc-comment явное
исключение с обоснованием («тело не выпускалось»), чтобы инвариант не выглядел
молча нарушенным, и удалить устаревшие `templates/act_handover.html` на UAT-машинах.

---

### WR-02: `white-space: nowrap` на печатном ФИО в блоке подписей — длинное ФИО не переносится и обрезается на печати

**Severity:** WARNING
**File:** `crates/trackly-app/templates/act_handover.html:117-119` (+ разметка `172-189`), `crates/trackly-app/templates/act_acceptance.html:103-105` (+ разметка `129-146`)

**Issue:** Фаза 35 впервые печатает ФИО в строке подписи (D-06/D-07) и делает
это внутри flex-строки: `.signature-row { display:flex }` + `.signature-label
{ nowrap }` + `.signature-field { flex: 0 0 160pt }` + `.signature-name
{ white-space: nowrap }`. При `nowrap` min-content ширина элемента равна полной
ширине строки, а `flex-shrink` не может сжать элемент ниже min-content без
`min-width: 0` — значит содержимое выходит за пределы полосы набора
(`@page` A4 с margin 15mm → ~180 mm ≈ 510 pt контента). Бюджет строки:
метка (~36 pt) + 2 gap (20 pt) + поле подписи (160 pt) = ~216 pt, остаётся
~294 pt на ФИО, что при Times 12 pt исчерпывается примерно на 48-52 символах.
Русское ФИО с двойной фамилией легко перекрывает этот лимит (собственная
фикстура проекта — «Сидоров-Петроградский Иван Александрович», 39 символов —
уже в 80 % бюджета). До Фазы 35 риска не было: ФИО не печаталось вовсе.
Текстовые ассерты `html.contains(...)` этот класс дефекта не видят по построению
(в проекте это уже зафиксированный урок).

**Fix:**
```css
  .signature-row .signature-name {
    min-width: 0;            /* разрешить flex-сжатие */
    white-space: normal;     /* разрешить перенос */
    overflow-wrap: break-word;
  }
```
(то же в `act_acceptance.html`). Проверять не текстовым тестом, а реальным
рендером/печатью с ФИО ≥ 55 символов.

---

### WR-03: При N > 1 имя каждого устройства печатается дважды, а множественный вводный оборот противоречит повторяющемуся единственному числу

**Severity:** WARNING (печатная корректность юридического документа)
**File:** `crates/trackly-app/templates/act_handover.html:131-162`

**Issue:** Закрытие CR-01 сделало строку «было получено устройство: {{ item.name }}»
безусловной, но верхний перечень при `act.items | length > 1` остался. Фактический
рендер (проверено прогоном шаблона через MiniJinja с N=3):

```
были получены устройства:
  • Ноутбук-0
  • Ноутбук-1
  • Ноутбук-2
было получено устройство: Ноутбук-0
  Инвентарный номер: ИНВ-0
было получено устройство: Ноутбук-1
  Комплектация: Сумка
было получено устройство: Ноутбук-2
```

То есть каждое имя дублируется, а после множественного «были получены
устройства» N раз идёт «было получено устройство» в единственном числе. Для акта
приёма-передачи это читается как перечисление разных фактов передачи и создаёт
почву для спора о комплекте.

**Fix:** выбрать один носитель идентификации (продуктовое решение, но избыточность
надо снять). Минимальный вариант — убрать верхний `<ul>` (per-block имя теперь
покрывает атрибуцию, и регресс-тест CR-01 продолжит проходить):
```jinja
{#- убрать блок 131-138 целиком -#}
```
Альтернатива — оставить перечень, а per-block подпись сделать нейтральной:
`<div class="field-row">Устройство: {{ item.name }}</div>`.

---

### WR-04: «Сроком до: ____» теперь печатается безусловно, в том числе на актах возврата, где срок бессмысленен

**Severity:** WARNING
**File:** `crates/trackly-app/templates/act_handover.html:164`

**Issue:** D-03 сделал строку безусловной с пустым подчёркиванием при отсутствии
значения. Но тот же шаблон рендерит и акты возврата (`render_pdf` вызывается для
return-акта — см. `pdf_render_act.rs:699-779`, где рендерится return с блоком
`act.parent`), а у возврата `deadline_utc` пуст практически всегда. В результате
на документе возврата появляется пустая строка «Сроком до: ______», приглашающая
дописать от руки срок, которого у возврата быть не может.

**Fix:**
```jinja
  {%- if act.parent %}
  {#- возврат: срок не применим — строку не печатаем -#}
  {%- else %}
  <div class="field-row">Сроком до: {% if act.deadline_human %}...{% endif %}</div>
  {%- endif %}
```

---

### WR-05: Preview редактора печатает литеральное `none` вместо суффикса номера акта

**Severity:** WARNING (дефект существовал до фазы, но живёт в проверяемом файле, изменённом в 35-01)
**File:** `crates/trackly-app/src/services/template_service.rs:508` (`"suffix": null`), используется `act_handover.html:45` и `:127`

**Issue:** MiniJinja `default` подменяет только `undefined`, а не `none`
(`filters.rs: if value.is_undefined() || (lax && !value.is_true())`), а `none`
печатается как строка `none` (`value/mod.rs:481`). Проверено фактическим
рендером с тем же env (`Strict` + `AutoEscape::Html`):

```
RENDERED: [№42none от 17 июня 2026]
```

Реальный рендер не затронут (`compute_suffix_from_display` возвращает `String`),
но админ в редакторе шаблонов видит «№42none» и в `<title>`, и в подзаголовке —
и может «починить» шаблон под сломанный demo-контекст.

**Fix:**
```rust
"suffix": "",           // вместо null
```
и/или в шаблоне `{{ act.suffix | default('', true) }}` (lax-форма гасит и `none`,
и пустую строку).

---

### WR-06: Demo-контекст preview содержит ровно одну позицию — ветка N > 1 (та самая, где жил CR-01) в редакторе непроверяема

**Severity:** WARNING
**File:** `crates/trackly-app/src/services/template_service.rs:516-529`

**Issue:** `demo_context_for_kind("act_handover")` даёт один `items[0]`, поэтому
`{% if act.items | length > 1 %}` и множественные `.device-block` никогда не
попадают в preview. Именно multi-device ветка была дефектной (CR-01) и именно её
админ не может увидеть перед сохранением своего шаблона; тесты
`validate_preview_*` тоже покрывают только N=1. Добавление второй позиции — одна
строка и закрывает целый класс невидимых регрессий.

**Fix:** добавить второй элемент в `items` (второе устройство без части
опциональных полей, чтобы preview показывал и «пустой» блок), например
`{"name":"Монитор 27\"","inventory_no":"ИНВ-002","serial_no":null,"model":null,
"quantity":1,"specs":null,"kit":null,"condition":"Б/У"}`.

---

### WR-07: Вакуозные ассерты на номер акта (`contains("1")`)

**Severity:** WARNING (test quality; в проверяемых файлах, доработанных в этой фазе)
**File:** `crates/trackly-app/tests/pdf_render_act.rs:198-202`, `crates/trackly-app/tests/html_act_render.rs:204-207`

**Issue:** Это ровно класс WR-02 из предыдущего ревью, оставшийся незакрытым в
соседних ассертах:
- `assert!(html.contains("№1") || html.contains('1'))` — правая часть истинна
  всегда: в `<style>` есть `1px`, `1.1em`, `1.2em`, `11pt`;
- `assert!(html.contains(&act.number_raw.to_string()))` при `number_raw == 1`
  сводится к `contains("1")` — тоже всегда истинно.

Оба ассерта пройдут, даже если номер акта вообще исчезнет из документа.

**Fix:**
```rust
let expected = format!("№{}{}", act.number_raw, /* suffix */ "");
assert!(html.contains(&expected), "act number missing: {expected}");
```
(или проверять подстроку подзаголовка `<div class="subtitle">№1 от `).

---

### WR-08: Блок подписей продублирован в двух пользовательских шаблонах вместо партиала — вопреки паттерну `_header.html`, введённому в Фазе 34

**Severity:** WARNING
**File:** `crates/trackly-app/templates/act_handover.html:88-119, 172-189` и `crates/trackly-app/templates/act_acceptance.html:74-105, 129-146`

**Issue:** ~30 строк CSS и ~18 строк разметки блока подписей скопированы байт-в-байт
в оба шаблона. Фаза 34 специально вынесла шапку в `_header.html` именно потому,
что дубль в user-editable файлах немедленно расходится по установкам. Прямое
следствие дубля уже видно в самом наборе изменений: гейт подчёркиваний пришлось
дублировать (`html_field_row_underline_gate.rs:56-90` и `97-114`), а исправление
WR-02 из этого отчёта придётся вносить дважды и в обоих файлах на каждой установке.

**Fix:** вынести `_signatures.html` и подключать как `_header.html`
(регистрация в `DEFAULT_HTML_TEMPLATES` + `extra_templates` в обоих
`render_*_pdf` + пре-флайт в `validate_preview`; `is_editable_template_filename`
уже корректно исключит `_`-префикс). Асимметрию имён контекста
(`act.giver_name` vs `document.giver_name`) снять в родителе перед include:
```jinja
{% with giver = act.giver_name, receiver = act.receiver_name %}
  {% include "_signatures.html" %}
{% endwith %}
```

---

### WR-09: Тесты legacy-снимков размножаются копипастой по индексу и не проверяют, что снимки попарно различны

**Severity:** WARNING
**File:** `crates/trackly-app/src/pdf/html_templates.rs:345-383` (`.first()`), `416-465` (`get(1)`), `476-525` (`get(2)`)

**Issue:** Три почти идентичных теста отличаются только индексом элемента. Из
этого следуют два дефекта покрытия:
1. каждый новый снимок требует ручного копирования четвёртого теста — именно так
   и возникла WR-01 предыдущего ревью (новый элемент без теста); механизм не
   самозащитный;
2. ни один тест не проверяет, что элементы слайса **различны между собой** —
   если снимок `vNN` случайно окажется копией `vNN-1` (реальный риск при
   «снимке после правки»), оба теста останутся зелёными: guard сравнивает снимок
   только с `current`, а апгрейд с дубликата тоже сработает.

**Fix:** один параметризованный тест вместо трёх:
```rust
#[test]
fn every_registered_legacy_snapshot_drives_a_real_upgrade() {
    for (filename, current) in DEFAULT_HTML_TEMPLATES.iter() {
        let bodies = /* slice for filename */;
        for (i, legacy) in bodies.iter().enumerate() {
            assert_ne!(legacy, current, "{filename}[{i}] == current");
            for (j, other) in bodies.iter().enumerate().skip(i + 1) {
                assert_ne!(legacy, other, "{filename}: snapshots {i} и {j} идентичны");
            }
            // tempdir: write legacy -> upgrade -> assert == current
        }
    }
}
```

## Info

### IN-01: Doc-comment оба акта утверждают наличие `org.logo_data_uri | safe` «at its use site», которого в этих файлах больше нет

**Severity:** INFO
**File:** `crates/trackly-app/templates/act_handover.html:29-40`, `crates/trackly-app/templates/act_acceptance.html:20-30`

**Issue:** С Фазы 34 единственный `| safe` живёт в `_header.html`; в обоих актах
`| safe` отсутствует. Комментарий (переписанный в этой фазе, `c74a579`) всё ещё
описывает «исключение в этом файле» — для user-editable файла это ложный
ориентир, который легитимизирует добавление `| safe` в пользовательскую правку.

**Fix:** заменить абзац на «в этом файле `| safe` не используется вовсе;
единственные санкционированные `| safe` — в `_header.html`».

### IN-02: В контрактном списке контекста перечислены ключи, которые шаблон не рендерит

**Severity:** INFO
**File:** `crates/trackly-app/templates/act_handover.html:24, 27` (`act.location_name`, `act.items[].quantity`)

**Issue:** Ни расположение, ни количество не печатаются (проверено: совпадений в
разметке нет ни в текущем теле, ни в `v20/v21/v22`). Список читается как
обещание, а для акта с `quantity > 1` в документе действительно нет количества —
стоит осознанно решить: рендерить или убрать из контракта.

### IN-03: Гейт подчёркиваний ловит только литерал `border-bottom`

**Severity:** INFO
**File:** `crates/trackly-app/tests/html_field_row_underline_gate.rs:44-53, 68`

**Issue:** Регрессия через `border-block-end`, шорткат `border: 0 0 1px`,
`text-decoration: underline` или инлайновый `style="..."` в разметке гейт
обойдёт. Плюс `extract_rule_body`'s `([^}]*)` перестанет работать, если правила
когда-нибудь завернут в `@media print { ... }`.

**Fix:** расширить набор запрещённых токенов (`border-bottom|border-block-end|
text-decoration`) и проверять также отсутствие `style="` в разметке `.field-row`.

### IN-04: В фикстурах используются реально регистрируемые домены вместо резервированных

**Severity:** INFO
**File:** `crates/trackly-app/tests/pdf_render_act.rs:755, 816`, `crates/trackly-app/src/services/template_service.rs:461`

**Issue:** `info@romashka.ru`, `info@test-org.ru`, `info@test.ru` — домены в
реальных TLD. Приватность организации не затронута (значения синтетические), но в
публичном репо чище использовать зарезервированные RFC 2606 `example.com`/`.test`
целиком (`info@example.test`).

### IN-05: Ассерты меток подписи не унифицированы между двумя тестами

**Severity:** INFO
**File:** `crates/trackly-app/tests/pdf_render_act.rs:245`

**Issue:** Здесь метки проверяются без двоеточия (`["Выдал", "Получил", "Подпись"]`),
тогда как закрытие WR-02 в `html_act_render.rs:188` перешло на `"Выдал:"`/`"Получил:"`.
Сейчас не вакуозно (фикстуры — «Иванов И.И.»/«Петров П.П.»), но одно изменение
фикстуры на «Выдалов В.В.» возвращает ровно ту вакуозность, которую фаза только
что закрыла.

**Fix:** привести к форме с двоеточием, как в `html_act_render.rs`.

---

_Reviewed: 2026-08-12T00:35:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
