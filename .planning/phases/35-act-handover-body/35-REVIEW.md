---
phase: 35-act-handover-body
reviewed: 2026-08-12T09:10:00Z
depth: standard
files_reviewed: 12
files_reviewed_list:
  - crates/trackly-app/src/pdf/html_templates.rs
  - crates/trackly-app/src/services/template_service.rs
  - crates/trackly-app/templates/_legacy_defaults/v22/act_acceptance.html
  - crates/trackly-app/templates/_legacy_defaults/v22/act_handover.html
  - crates/trackly-app/templates/_legacy_defaults/v23/act_acceptance.html
  - crates/trackly-app/templates/_legacy_defaults/v23/act_handover.html
  - crates/trackly-app/templates/act_acceptance.html
  - crates/trackly-app/templates/act_handover.html
  - crates/trackly-app/tests/acts_e2e_smoke.rs
  - crates/trackly-app/tests/html_act_render.rs
  - crates/trackly-app/tests/html_field_row_underline_gate.rs
  - crates/trackly-app/tests/pdf_render_act.rs
findings:
  critical: 0
  warning: 8
  info: 7
  total: 15
status: issues_found
---

# Phase 35: Code Review Report (ревью после плана 35-07)

**Reviewed:** 2026-08-12T09:10:00Z
**Depth:** standard
**Files Reviewed:** 12
**Status:** issues_found

## Summary

Третий проход по Фазе 35 — после gap-closure плана 35-07 (`f162c79`: срез
`_legacy_defaults/v23` + CSS-фикс переноса ФИО в блоке подписей). Проверялось
фактическое состояние файлов и git-история тел шаблонов, а не заявления
SUMMARY.

**Что реально закрыто (проверено независимо):** срез `v23` байт-в-байт равен
телам обоих актов на коммите `d274e6b` (то есть снят ДО фикса, не после) —
`git hash-object` подтверждает `5f5fdee…`/`f2f35fb…`; `white-space: nowrap` на
`.signature-name` заменён на `min-width: 0 / white-space: normal /
overflow-wrap: break-word` в обоих шаблонах; появились структурный CSS-гейт
`signature_name_css_permits_wrap_for_long_names` и два end-to-end теста на
53-символьное вымышленное ФИО.

**Главная остающаяся проблема (WR-01):** план 35-07 зарегистрировал ОДНО из
четырёх промежуточных тел фазы. Внутри Фазы 35 `act_handover.html` побывал в
четырёх различных состояниях (`ef08ced`, `af3f4a6`, `5cc4ecf`, `5f5fdee`), а
`act_acceptance.html` — в двух (`0c566fe`, `f2f35fb`); в
`KNOWN_LEGACY_DEFAULTS` попали только `5f5fdee`/`f2f35fb`. Установка, чей файл
на диске равен `5cc4ecf` (тело, на котором CR-01 и был обнаружен — анонимные
`.device-block`), классифицируется как «user-customized» и НИКОГДА не получит
ни фикс CR-01, ни фикс переноса ФИО. Продакшн-риска нет (см. ниже), но это ровно
та машина, на которой стоит блокирующий human-verify гейт фазы.

**Смягчающий факт, проверенный по всем релизным тегам:** `v1.1.x`, `v1.2`,
`v1.2.0`, `v1.3`, `v1.3.0`, `v1.3.1`, `v1.3.2` несут тело `c479c56…` /
`6f82db9…` = снимок `v21`, который зарегистрирован. То есть ни одна выпущенная
установка не залипнет; риск ограничен dev/UAT-машинами. Локальная
материализованная копия на этой машине (`target/debug/templates/act_handover.html`,
`e411cd2…`, mtime 12 Aug 14:23) уже равна текущему bundled-телу — mac-бокс чист,
Windows-бокс не проверяем отсюда.

**Приватность (жёсткое условие CLAUDE.md): нарушений нет.** В шаблонах —
только placeholder'ы (`{{ act.giver_name }}`, `org.*`), реального названия
организации, ИНН/КПП/ОГРН/адресов/телефонов нет; в тестах и demo-контексте —
вымышленные ФИО («Иванов И.И.», «Выдалов В.В.», «Получилов П.П.»,
«Сидоров-Петроградский-Константинов Иван Александрович») и синтетические
реквизиты («ООО Демо Организация», `7700000000`, `+7 495 000-00-00`). Срезы
`_legacy_defaults/v22|v23` — копии тех же обезличенных шаблонов.

**Критических находок нет.** Проверено адресно: в обоих актах не осталось ни
одного `| safe` (единственные санкционированные — в `_header.html`, для
серверно-собранных `logo_data_uri`/`org.full_name`); autoescape включён
(`build_safe_html_env`, `AutoEscape::Html`); новых ключей контекста фаза не
вводит, а `act.giver_name` есть и в `act_service::render_pdf`, и в
`demo_context_for_kind`, так что `UndefinedBehavior::Strict` не даёт краша;
`update_body`/`reset_to_default` по-прежнему проверяют `kind` по фиксированному
allowlist ДО любого `join`, так что path traversal закрыт.

## Проверка закрытия предыдущих находок (ревью 2026-08-12T00:35)

| Находка | Статус | Доказательство |
|---|---|---|
| WR-01 (нет среза `v23`) | **закрыта частично** | Срез `v23` добавлен и зарегистрирован (`html_templates.rs:82,91`), тест `upgrade_replaces_v23_legacy_default_with_current_bundled_body` (`html_templates.rs:536-585`) тянет индекс `3` и имеет анти-вакуозный guard. Но зарегистрировано только тело `d274e6b`; тела `ef08ced`, `af3f4a6`, `5cc4ecf` (handover) и `0c566fe` (acceptance) остались вне реестра → см. новую WR-01. |
| WR-02 (`nowrap` на печатном ФИО) | **закрыта** | `act_handover.html:117-121` и `act_acceptance.html:103-107` содержат `min-width: 0; white-space: normal; overflow-wrap: break-word`. Гейт `signature_name_css_permits_wrap_for_long_names` (`html_field_row_underline_gate.rs:124-156`) не вакуозен: `extract_rule_body` паникует при отсутствии правила, а `!body.contains("nowrap")` падает при откате. |
| WR-03 (дубль имени устройства + число) | **принято продуктом** | 35-CONTEXT.md D-02a прямо фиксирует «избыточность принята осознанно» решением пользователя. Понижено до IN-06, дефектом не считаю. |
| WR-04 («Сроком до» на возвратах) | **не закрыта** | `act_handover.html:166` по-прежнему безусловна → см. WR-02 ниже. |
| WR-05 (`None` в preview) | **не закрыта** | `template_service.rs:508` — `"suffix": null` → см. WR-03 ниже (перепроверено фактическим рендером на minijinja 2.20). |
| WR-06 (demo-контекст с одной позицией) | **не закрыта** | `template_service.rs:516-529` — один элемент `items` → см. WR-04 ниже. |
| WR-07 (вакуозные ассерты номера) | **не закрыта** | `pdf_render_act.rs:206-210` и `html_act_render.rs:204-207` без изменений → см. WR-05 ниже. |
| WR-08 (дубль блока подписей) | **не закрыта, подтверждена практикой** | Фикс 35-07 пришлось вносить в два файла, гейт — писать циклом по двум файлам → см. WR-07 ниже. |
| WR-09 (копипаста тестов снимков) | **не закрыта, усугублена** | Добавлена четвёртая копия того же 50-строчного скелета (`html_templates.rs:536-585`) → см. WR-06 ниже. |
| IN-01…IN-05 | **не закрыты** | Все пять воспроизводятся в текущем HEAD, перенесены ниже без изменений. |

## Structural Findings (fallow)

Структурный пре-проход в задании не передан (`<structural_findings>`
отсутствует) — раздел оставлен пустым намеренно, чтобы narrative-находки ниже
не выдавались за структурный субстрат.

## Narrative Findings (AI reviewer)

## Critical Issues

Не найдено.

## Warnings

### WR-01: Из четырёх промежуточных тел Фазы 35 зарегистрировано одно — установка на теле `5cc4ecf` навсегда теряет и фикс CR-01, и фикс переноса ФИО

**Severity:** WARNING (риск для блокирующего human-verify гейта фазы; продакшн не затронут)
**File:** `crates/trackly-app/src/pdf/html_templates.rs:57-63` (инвариант), `:75-107` (реестр)

**Issue:** Doc-comment модуля объявляет обязательным снимок PRE-CHANGE тела при
*каждом* изменении `DEFAULT_HTML_TEMPLATES`. Внутри Фазы 35 тела менялись так
(`git rev-parse <commit>:crates/trackly-app/templates/…`):

| Коммит | `act_handover.html` | `act_acceptance.html` | В реестре? |
|---|---|---|---|
| `e0d2dca` (pre-35) | `a6e9323` | `7be7a95` | да — `v22` |
| `c74a579` | `ef08ced` | `7be7a95` | **нет** |
| `3904da9` | `af3f4a6` | `7be7a95` | **нет** |
| `d337c7d` | `5cc4ecf` | `7be7a95` | **нет** |
| `81b3d39` | `5cc4ecf` | `0c566fe` | **нет** |
| `bbfed54` | `5cc4ecf` | `f2f35fb` | частично (только acceptance) |
| `d274e6b` | `5f5fdee` | `f2f35fb` | да — `v23` |
| `f162c79` (HEAD) | `e411cd2` | `12a2d0f` | current |

Тело `5cc4ecf` прожило три коммита подряд (планы 35-03…35-05) — это то самое
тело, на котором VERIFICATION SC#2 упал (анонимные `.device-block`, CR-01).
Любая машина, где приложение запускалось в этом окне, имеет `5cc4ecf` на диске;
`upgrade_untouched_defaults_on_startup` (`html_templates.rs:214-238`) уйдёт в
ветку `else` («user-customized»), `build_templates_status`
(`tauri_cmds/settings_org.rs:316-330`) покажет `Customized`, и человек на
UAT-гейте увидит СТАРЫЙ документ — то есть закономерно решит, что фиксы CR-01 и
DOC-08/SC#4 не работают. Ни один из четырёх структурных тестов апгрейда этого не
ловит: они итерируются по зарегистрированным снимкам, а не по истории.

**Fix (любой из двух, но осознанно):**
```rust
// A. Дорегистрировать реально существовавшие промежуточные тела:
//    _legacy_defaults/v22a/act_handover.html  <- git show d337c7d:…/act_handover.html (5cc4ecf)
//    _legacy_defaults/v22b/act_acceptance.html <- git show 81b3d39:…/act_acceptance.html (0c566fe)
//    + добавить их в соответствующие слайсы KNOWN_LEGACY_DEFAULTS.
```
```text
B. Либо зафиксировать в doc-comment (html_templates.rs:57-63) явное правило
   «снимок делается только для тел, вошедших в релизный тег; внутрифазовые
   промежуточные тела не регистрируются» + добавить в 35-VERIFICATION.md
   предусловие UAT: «удалить templates/act_*.html (или проверить, что в логе
   нет "Skipped auto-upgrade") перед проверкой печати».
```
Вариант B дешевле и честнее (проверено: ни один релизный тег не несёт
промежуточных тел), но тогда текущие срезы `v22`/`v23` тоже не нужны — они
описывают невыпущенные состояния.

---

### WR-02: «Сроком до: ______» печатается безусловно, в том числе на актах возврата

**Severity:** WARNING (корректность печатного документа)
**File:** `crates/trackly-app/templates/act_handover.html:166`

**Issue:** D-03 сделал строку безусловной с пустой полоской. Но этот же шаблон
рендерит и акты возврата: `act_service::render_pdf` вызывается для return-акта
(регресс-тест `html_act_render.rs:699-779` рендерит возврат с блоком
`act.parent`), а `deadline_utc` у возврата пуст практически всегда. В итоге на
документе возврата печатается приглашение вписать от руки срок, которого у
возврата быть не может, — и полоска попадает в подписываемый документ.

**Fix:**
```jinja
  {%- if not act.parent %}
  <div class="field-row">Сроком до: {% if act.deadline_human %}{{ act.deadline_human }}{% elif act.deadline %}{{ act.deadline }}{% else %}<span class="value-blank"></span>{% endif %}</div>
  {%- endif %}
```
(и добавить в `pdf_render_act.rs` ассерт `!html.contains("Сроком до")` для
return-акта — иначе регрессия снова пройдёт незамеченной).

---

### WR-03: Preview редактора печатает литеральное `None` вместо суффикса номера акта

**Severity:** WARNING
**File:** `crates/trackly-app/src/services/template_service.rs:508` (`"suffix": null`); потребители — `act_handover.html:45` и `:129`

**Issue:** MiniJinja `default` подменяет только `undefined`, а не `none`
(`minijinja-2.20.0/src/filters.rs:540` — `if value.is_undefined() || (lax &&
!value.is_true())`), а `none` печатается литералом. Перепроверено фактическим
рендером на minijinja 2.20 с тем же env (`UndefinedBehavior::Strict` +
`AutoEscape::Html`) и тем же demo-контекстом:

```
OUT=[№42None от 17 июня 2026]
```

Реальный рендер не затронут (`compute_suffix_from_display` возвращает `String`,
`act_service.rs:2610`), но админ в редакторе шаблонов видит «№42None» и в
`<title>`, и в подзаголовке — и рискует «починить» шаблон под сломанный
demo-контекст. Побочно это значит, что demo-контекст не воспроизводит типы
реального контекста, а именно ради этого он и существует.

**Fix:**
```rust
"suffix": "",   // template_service.rs:508 — String, как в реальном ctx
```
опционально плюс lax-форма в шаблоне: `{{ act.suffix | default('', true) }}`.

---

### WR-04: Demo-контекст preview содержит ровно одну позицию — ветка N > 1 (та, где жил CR-01) в редакторе непроверяема

**Severity:** WARNING
**File:** `crates/trackly-app/src/services/template_service.rs:516-529`

**Issue:** `demo_context_for_kind("act_handover")` отдаёт единственный
`items[0]`, поэтому ни `{% if act.items | length > 1 %}` (сводный `<ul>`,
`act_handover.html:133-140`), ни повторяющиеся `.device-block` в preview никогда
не попадают. Именно multi-device ветка была дефектной (CR-01) и именно её админ
не видит перед сохранением своей правки; тесты `validate_preview_*`
(`template_service.rs:601-684`) тоже покрывают только N=1. После D-02a в этой
ветке живёт вся логика атрибуции полей к устройству — она обязана быть в
preview.

**Fix:** добавить вторую позицию (намеренно без части опциональных полей, чтобы
preview показывал и «бедный» блок):
```rust
{"name":"Монитор 27\"","inventory_no":"ИНВ-002","serial_no":null,
 "model":null,"quantity":1,"specs":null,"kit":null,"condition":"Б/У"}
```

---

### WR-05: Вакуозные ассерты на номер акта — пройдут, даже если номер исчезнет из документа

**Severity:** WARNING (test quality)
**File:** `crates/trackly-app/tests/pdf_render_act.rs:206-210`, `crates/trackly-app/tests/html_act_render.rs:204-207`

**Issue:** Это тот же класс, что фаза уже закрывала для меток подписи:
- `assert!(html.contains("№1") || html.contains('1'))` — правая часть истинна
  всегда: в `<style>` есть `1px`, `1.1em`, `1.2em`, `11pt`, `10pt`;
- `assert!(html.contains(&act.number_raw.to_string()))` при `number_raw == 1`
  вырождается в `contains("1")` — тоже всегда истинно.

**Fix:**
```rust
let subtitle = format!("<div class=\"subtitle\">№{}", act.number_raw);
assert!(html.contains(&subtitle), "номер акта отсутствует в подзаголовке");
```

---

### WR-06: Четвёртая копия теста снимков; нет гейта «тело изменилось → снимок обязателен» и нет проверки попарной различности снимков

**Severity:** WARNING (maintainability + покрытие)
**File:** `crates/trackly-app/src/pdf/html_templates.rs:347-385` (`.first()`), `:418-467` (`get(1)`), `:479-527` (`get(2)`), `:536-585` (`get(3)`)

**Issue:** Четыре теста отличаются ровно одним числом — индексом элемента
слайса. Прогноз прошлого ревью («каждый новый снимок требует ручного копирования
следующего теста») сбылся в этой же фазе. Три следствия:
1. механизм не самозащитный — новый снимок без своей копии теста молча не
   покрывается, а `bodies.get(N)` вернёт `None` и тест `continue`'нет;
2. ни один тест не проверяет, что элементы слайса **различны между собой** —
   если снимок `vNN` окажется копией `vNN-1` (реальный риск при «снимке после
   правки»), оба теста останутся зелёными: guard сравнивает снимок только с
   `current`;
3. ничто не проверяет обратное направление инварианта — «bundled-тело
   изменилось, а нового снимка нет» (это и есть WR-01).

**Fix:** один параметризованный тест вместо четырёх + отдельный CI-гейт на
пункт 3:
```rust
#[test]
fn every_registered_legacy_snapshot_drives_a_real_upgrade() {
    for (filename, current) in DEFAULT_HTML_TEMPLATES.iter() {
        let bodies = KNOWN_LEGACY_DEFAULTS.iter()
            .find(|(n, _)| n == filename).map(|(_, b)| *b).unwrap_or(&[]);
        for (i, legacy) in bodies.iter().enumerate() {
            assert_ne!(legacy, current, "{filename}[{i}] == current");
            for (j, other) in bodies.iter().enumerate().skip(i + 1) {
                assert_ne!(legacy, other, "{filename}: снимки {i} и {j} идентичны");
            }
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join(filename), legacy).unwrap();
            upgrade_untouched_defaults_on_startup(dir.path()).unwrap();
            assert_eq!(&std::fs::read_to_string(dir.path().join(filename)).unwrap(), current);
        }
    }
}
```
```bash
# CI-гейт для п.3: изменение тела обязано сопровождаться новым срезом
git diff --name-only "$BASE"..HEAD -- crates/trackly-app/templates/*.html \
  | grep -q . && git diff --name-only "$BASE"..HEAD -- \
      crates/trackly-app/templates/_legacy_defaults/ | grep -q . \
  || { echo "template body changed without a _legacy_defaults snapshot"; exit 1; }
```

---

### WR-07: Блок подписей продублирован в двух user-editable шаблонах вместо партиала — цена дубля уже материализовалась

**Severity:** WARNING (известная находка прошлого ревью, подтверждена практикой)
**File:** `crates/trackly-app/templates/act_handover.html:88-121, 174-191` и `crates/trackly-app/templates/act_acceptance.html:74-107, 131-148`

**Issue:** ~34 строки CSS и ~18 строк разметки скопированы байт-в-байт. Фаза 34
специально вынесла шапку в `_header.html` именно потому, что дубль в
user-editable файлах немедленно расходится по установкам. За один только план
35-07 дубль стоил: правку в двух файлах, два новых среза `_legacy_defaults/v23`
(вместо одного), цикл по двум файлам в гейте
(`html_field_row_underline_gate.rs:126-129`) и второй почти идентичный
end-to-end тест (`pdf_render_act.rs:701-742`). Следующая правка подписей будет
стоить столько же — и разъедется у пользователей, которые правили только один из
двух файлов.

**Fix:** вынести `_signatures.html` по образцу `_header.html` (регистрация в
`DEFAULT_HTML_TEMPLATES` + `extra_templates` в обоих `render_*_pdf` + пре-флайт в
`validate_preview`; `is_editable_template_filename` уже корректно исключит
`_`-префикс). Асимметрию имён (`act.giver_name` vs `document.giver_name`) снять в
родителе перед include:
```jinja
{% with giver = act.giver_name, receiver = act.receiver_name %}
  {% include "_signatures.html" %}
{% endwith %}
```

---

### WR-08: `update_body` сохраняет пустое тело шаблона — печать после этого молча даёт пустой документ

**Severity:** WARNING (защита от самоповреждения; требует роли `ManageSettings`)
**File:** `crates/trackly-app/src/services/template_service.rs:260-301`

**Issue:** Единственная валидация тела — `validate_preview`, то есть успешный
рендер. Пустая строка (или, скажем, случайно вставленный одиночный пробел)
рендерится успешно, `tokio::fs::write` записывает файл, `load_template`
(`html_templates.rs:252-267`) возвращает on-disk содержимое (пустая строка — это
`Ok`, не `NotFound`), и все последующие печатные формы этого вида отдают пустой
HTML. Диагностики нет: `build_templates_status` покажет `Customized`, что
формально верно и бесполезно. Тот же путь используется для `report.html`.
Побочно: запись не атомарна — обрыв процесса/диска посреди `write` оставляет
усечённый шаблон с тем же эффектом.

**Fix:**
```rust
if body.trim().is_empty() {
    return Err(AppError::Validation {
        field: "body".into(),
        message: "Шаблон не может быть пустым".into(),
    });
}
// и/или потребовать, чтобы отрендеренный preview был непустым:
let preview = self.validate_preview(kind, &body).await.map_err(…)?;
if preview.trim().is_empty() { /* та же Validation */ }
```
Атомарность — запись во временный файл рядом + `rename`.

## Info

### IN-01: Doc-comment обоих актов утверждает наличие `org.logo_data_uri | safe` «at its use site», которого в этих файлах больше нет

**Severity:** INFO
**File:** `crates/trackly-app/templates/act_handover.html:29-40`, `crates/trackly-app/templates/act_acceptance.html:20-30`

**Issue:** С Фазы 34 единственный `| safe` живёт в `_header.html`; в обоих актах
`| safe` отсутствует (проверено grep'ом). Комментарий, переписанный в этой фазе
(`c74a579`), всё ещё описывает «единственное исключение в этом файле» — для
user-editable файла это ложный ориентир, легитимизирующий добавление `| safe` в
пользовательскую правку. Тот же текст теперь размножен в срезы `v22`/`v23`.

**Fix:** заменить абзац на «в этом файле `| safe` не используется вовсе;
единственные санкционированные `| safe` — в `_header.html`».

### IN-02: В контрактном списке контекста перечислены ключи, которые шаблон не рендерит

**Severity:** INFO
**File:** `crates/trackly-app/templates/act_handover.html:24, 27`

**Issue:** `act.location_name` и `act.items[].quantity` перечислены как
контракт, но не печатаются (совпадений нет ни в текущем теле, ни в
`v20`/`v21`/`v22`/`v23`). Для `location_name` это осознанное D-04 — стоит так и
написать; `quantity` при клонировании разворачивается в отдельные `act_items`
(`act_service.rs:362-409`), так что отсутствие количества в документе корректно,
но из списка это не следует.

**Fix:** пометить оба ключа как «в контексте есть, намеренно не печатаются
(D-04)».

### IN-03: Структурные CSS-гейты завязаны на литералы

**Severity:** INFO
**File:** `crates/trackly-app/tests/html_field_row_underline_gate.rs:44-53, 68, 133-148`

**Issue:** (а) гейт подчёркиваний ищет литерал `border-bottom` — регрессия через
`border-block-end`, шорткат `border: 0 0 1px`, `text-decoration: underline` или
инлайновый `style="…"` в разметке пройдёт мимо; (б) новый wrap-гейт требует
точных подстрок `min-width: 0`, `white-space: normal`, `overflow-wrap:
break-word` — семантически эквивалентные `min-width:0` (без пробела),
`min-width: 0px` или переход на `overflow-wrap: anywhere` дадут ложное падение;
(в) `extract_rule_body`'s `([^}]*)` сломается, если правила когда-нибудь завернут
в `@media print { … }`.

**Fix:** нормализовать пробелы перед сравнением (`css.replace(char::is_whitespace, "")`)
и расширить набор запрещённых токенов подчёркивания.

### IN-04: В фикстурах используются реально регистрируемые домены вместо зарезервированных

**Severity:** INFO
**File:** `crates/trackly-app/tests/pdf_render_act.rs:852, 913`, `crates/trackly-app/src/services/template_service.rs:461`

**Issue:** `info@romashka.ru`, `info@test-org.ru`, `info@test.ru` — домены в
реальных TLD. Приватность организации не затронута (значения синтетические), но
в публичном репо чище использовать RFC 2606 (`info@example.test`).

### IN-05: Ассерты меток подписи не унифицированы между двумя тестами

**Severity:** INFO
**File:** `crates/trackly-app/tests/pdf_render_act.rs:253`

**Issue:** Здесь метки проверяются без двоеточия (`["Выдал", "Получил",
"Подпись"]`), тогда как закрытие прошлой WR-02 в `html_act_render.rs:188`
перешло на `"Выдал:"`/`"Получил:"`. Сейчас не вакуозно (фикстуры — «Иванов
И.И.»), но замена фикстуры на «Выдалов В.В.» вернёт ровно ту вакуозность,
которую фаза уже закрывала.

**Fix:** привести к форме с двоеточием.

### IN-06: Дубль имени устройства при N > 1 и «были получены устройства» перед N-кратным «было получено устройство» — принято продуктом

**Severity:** INFO (не дефект; фиксируется, чтобы не переоткрывали)
**File:** `crates/trackly-app/templates/act_handover.html:133-164`

**Issue:** Сводный `<ul>` при `length > 1` и безусловная строка «было получено
устройство: {{ item.name }}» в каждом блоке дают двойное перечисление имён и
рассогласование числа. 35-CONTEXT.md D-02a прямо фиксирует, что пользователь
выбрал вариант с обоими блоками и «избыточность принята осознанно», а D-01
запрещает переписывать текст. Прошлое ревью классифицировало это как WARNING —
понижаю до INFO как принятое продуктовое решение.

### IN-07: Для human-verify гейта: проверить не только длину ФИО, но и вертикальное выравнивание перенесённой строки

**Severity:** INFO
**File:** `crates/trackly-app/templates/act_handover.html:92-97, 117-121`, `crates/trackly-app/templates/act_acceptance.html:78-83, 103-107`

**Issue:** `.signature-row { align-items: flex-end }` выравнивает ФИО по нижнему
краю `.signature-field`, то есть по подписи «Подпись» (8pt), а не по полоске.
Пока ФИО в одну строку это незаметно; после фикса переноса длинное ФИО займёт
две строки, и по низу выровняется ВТОРАЯ строка — первая уедет выше полоски.
Rust-тесты этого класса не видят по построению (документированное ограничение,
RESEARCH.md Pitfall 5), поэтому это пункт для человека на блокирующем гейте
вместе с проверкой самого переноса. Если выглядит плохо — `align-items:
baseline` либо `.signature-name { align-self: flex-end; line-height: 1.1 }`.

---

_Reviewed: 2026-08-12T09:10:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
