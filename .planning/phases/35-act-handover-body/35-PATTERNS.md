# Phase 35: Тело акта приёма-передачи — Pattern Map

**Mapped:** 2026-08-11
**Файлов проанализировано:** 9 (7 обязательных + 1 опциональный + 1 демо-контекст-функция)
**Аналогов найдено:** 9 / 9

Особенность этой фазы: почти всё редактируется «на месте» — новый файл-аналог обычно есть в
том же самом файле (шапка `act_handover.html` — аналог для тела `act_handover.html`) либо в
соседнем шаблоне (`act_acceptance.html` ↔ `act_handover.html`). Внешних аналогов из других
доменов кодовой базы не требуется — весь паттерн-набор уже живёт внутри `templates/` +
`src/pdf/html_templates.rs` + `src/services/template_service.rs`.

🔒 Все ФИО в примерах ниже — вымышленные (Иванов И.И./Петров П.П./Сидоров С.С.), взяты из уже
закоммиченных тестов и демо-контекста; реальных данных организации/сотрудников нет.

## File Classification

| Файл (создать/править) | Роль | Data Flow | Ближайший аналог | Качество совпадения |
|---|---|---|---|---|
| `crates/trackly-app/templates/act_handover.html` (тело) | template (MiniJinja HTML) | file-I/O → request-response | эта же секция файла ДО правки (строки 64-201) | exact (правка на месте) |
| `crates/trackly-app/templates/act_acceptance.html` (таблица+подписи) | template (MiniJinja HTML) | file-I/O → request-response | `act_handover.html`'s `.signatures` (целевой паттерн D-06/D-07) | role-match (перенос паттерна из соседнего шаблона) |
| `_legacy_defaults/v22/act_handover.html` (НОВЫЙ) | snapshot/fixture | file-I/O (compile-time `include_str!`) | `_legacy_defaults/v21/act_handover.html` | exact (тот же механизм слайса) |
| `_legacy_defaults/v22/act_acceptance.html` (НОВЫЙ) | snapshot/fixture | file-I/O (compile-time `include_str!`) | `_legacy_defaults/v21/act_acceptance.html` | exact |
| `src/pdf/html_templates.rs` (`KNOWN_LEGACY_DEFAULTS`) | config/registry | in-memory static data | добавление v21-элемента в Фазе 34 (тот же паттерн, +1 элемент к существующим слайсам) | exact |
| `src/services/template_service.rs` (`demo_context_for_kind`, ветка `_`) | service (демо-fixture builder) | transform (JSON построение) | соседняя ветка `"act_acceptance"` того же match — уже содержит `document.giver_name` | exact (тот же файл, соседний паттерн) |
| `tests/pdf_render_act.rs` | test (integration) | request-response assertion | сам файл ДО правки — правится 3 существующие функции | exact (правка на месте) |
| `tests/html_act_render.rs` | test (integration) | request-response assertion | сам файл ДО правки — `html_handover_contains_required_blocks_and_logo` | exact (правка на месте) |
| `tests/acts_e2e_smoke.rs` | test (integration, e2e smoke) | request-response assertion | сам файл ДО правки — комментарий устарел, ассерция технически совместима | exact (комментарий-only правка) |
| `tests/html_field_row_underline_gate.rs` (НОВЫЙ, опционально) | test (structural regex gate) | file-I/O (compile-time `include_str!`) → assertion | `tests/html_page_parity.rs` | exact (готовый образец для копирования целиком) |

## Pattern Assignments

### `crates/trackly-app/templates/act_handover.html` (template, file-I/O)

**Аналог:** эта же секция файла до правки — Фаза 35 не приносит новый паттерн, а трансформирует
существующий по решениям D-02/D-03/D-06..D-12.

**Doc-комментарий в шапке (обновить, C-02)** — текущее состояние строк 2-36:
```html
<!-- Source: crates/trackly-app/templates/act_handover.html:2-22 (текущий HEAD) -->
{#- Default HTML template for Акт приёма-передачи (Phase 16, D-01/D-02/D-03).

  Self-contained HTML5 document (inline <style>, no external CSS/CDN, D-11
  data: URI logo) reproducing the Word-sample block order fixed in Phase 15's
  act_handover.minijinja: shared header partial (logo + org requisites,
  Phase 34, see _header.html) -> centered title -> number/date -> intro
  field_row -> per-item field_rows (label | underlined value, no "Устройство
  №N") -> "Сроком до" -> optional parent-act paragraph -> two-line signatures
  "Выдал/Получил".

  Context variables (same shape as act_service::render_pdf's ctx, D-04, plus
  org.logo_data_uri replacing org.logo_path per D-11, plus org.full_name for
  the shared header partial per Phase 34):
    org.name, org.full_name, org.inn, org.kpp, org.address, org.address_line2,
    org.logo_data_uri, org.phone, org.fax, org.email, org.okpo, org.ogrn
    act.number, act.suffix, act.date, act.date_human,
    act.receiver_name, act.deadline, act.deadline_human,
    act.location_name, act.items[], act.parent
    act.items[].name, act.items[].inventory_no, act.items[].serial_no,
    act.items[].model, act.items[].specs, act.items[].kit,
    act.items[].condition, act.items[].quantity
-#}
```
Требуется по C-02: описать НОВУЮ структуру («label + value сплошным текстом, без underline») и
добавить `act.giver_name` в перечень ключей контекста (сейчас отсутствует, хотя `act_service.rs`
его туда уже кладёт). Doc-комментарий у `act_acceptance.html` (строки 2-27, ниже) построен по
тому же шаблону-конвенции «что это / как устроено / перечень ключей» — использовать как образец
формулировок, если нужен второй пример стиля.

**CSS `.field-row` (снять `border-bottom`, D-10) — текущее состояние строк 64-78:**
```css
/* Source: crates/trackly-app/templates/act_handover.html:64-78 (текущий HEAD) */
.field-row {
  display: flex;
  align-items: baseline;
  gap: 6pt;
  margin: 4pt 0;
}
.field-row .label {
  white-space: nowrap;
}
.field-row .value {
  flex: 1;
  border-bottom: 1px solid #000;
  min-height: 1.2em;
  padding: 0 2pt;
}
```
D-10/D-11: `border-bottom` убирается из базового `.field-row .value`; `display: flex` на
`.field-row`, вероятно, тоже не нужен (сплошной текст, а не колонки label|value) — точная
разметка на усмотрение исполнителя. Полоска сохраняется РОВНО в двух местах: пустое «Сроком до»
(отдельный класс/модификатор) и `.signature .line` (уже с `border-bottom`, ниже).

**Цикл по устройствам (D-11, сплошной текст вместо `label|value` в двух `<span>`) —
текущее состояние строк 129-134 (репрезентативный `field-row`):**
```html
<!-- Source: crates/trackly-app/templates/act_handover.html:129-134 (текущий HEAD) -->
{%- if item.inventory_no %}
<div class="field-row">
  <span class="label">Инвентарный номер:</span>
  <span class="value">{{ item.inventory_no }}</span>
</div>
{%- endif %}
```
Такой же паттерн `{%- if item.X %}...{%- endif %}` повторяется для `serial_no` (135-140),
`model` (141-146), `kit` (147-152), `specs` (153-158), `condition` (159-164) — все шесть
`field-row` внутри `{%- for item in act.items %}` (123-166) должны быть переведены на сплошной
текст единообразно.

**Интро + заглушка (D-01 текст не трогать, D-12 заглушку удалить) — строки 115-121:**
```html
<!-- Source: crates/trackly-app/templates/act_handover.html:115-121 (текущий HEAD) -->
<div class="intro field-row">
  <span class="label">Настоящим актом утверждаю, что мною:</span>
  <span class="value">{{ act.receiver_name }}</span>
</div>
<div class="field-row">
  <span class="value">&nbsp;</span>
</div>
```
Вторая `div.field-row` (119-121, пустая заглушка `&nbsp;`) удаляется целиком (D-12). Первая
меняет только разметку (сплошной текст вместо двух `<span>`), текст НЕ меняется (D-01).

**«Сроком до» — снять условное скрытие (D-03) — строки 168-178:**
```html
<!-- Source: crates/trackly-app/templates/act_handover.html:168-178 (текущий HEAD) -->
{%- if act.deadline_human %}
<div class="field-row">
  <span class="label">Сроком до:</span>
  <span class="value">{{ act.deadline_human }}</span>
</div>
{%- elif act.deadline %}
<div class="field-row">
  <span class="label">Сроком до:</span>
  <span class="value">{{ act.deadline }}</span>
</div>
{%- endif %}
```
Строка выводится безусловно; пустое значение получает выделенный CSS-класс с `border-bottom`
(единственное место в теле, где полоска сохраняется, кроме подписи).

**Множественное число N>1 (D-02) — рабочий образец `length`-фильтра из соседнего шаблона:**
```html
<!-- Source: crates/trackly-app/templates/report.html:117 -->
{%- if groups is not defined or groups | length == 0 %}
```
Подтверждает `{% if act.items | length > 1 %}…{% else %}…{% endif %}` как рабочий паттерн того
же движка (фича `builtins`, `Cargo.toml:53`) — не нужно ничего добавлять в `Environment`.

**Блок подписей (D-06/D-07/D-08) — текущее состояние строк 186-201:**
```html
<!-- Source: crates/trackly-app/templates/act_handover.html:186-201 (текущий HEAD) -->
<div class="signatures">
  <div class="signature">
    <div class="line"></div>
    <div class="sublabel">Выдал</div>
    <div class="sublabel">Подпись</div>
    <div class="line"></div>
    <div class="sublabel">ФИО</div>
  </div>
  <div class="signature">
    <div class="line"></div>
    <div class="sublabel">Получил</div>
    <div class="sublabel">Подпись</div>
    <div class="line"></div>
    <div class="sublabel">ФИО</div>
  </div>
</div>
```
Заменяется на горизонтальный блок, одна строка на подписанта:
`Выдал: ⟨полоска⟩ ⟨act.giver_name⟩` / `Получил: ⟨полоска⟩ ⟨act.receiver_name⟩`, «Подпись» только
под полоской, без второй полоски и без «ФИО». `{{ act.giver_name }}`/`{{ act.receiver_name }}` —
обычная `{{ var }}`-интерполяция под `AutoEscape::Html` (T-16-01), НЕ добавлять `| safe`.

**Не трогать (границы фазы):** `@page` (42-45, Фаза 33 D-01), `body` font-family/size (46-52,
Фаза 34 D-09/D-11), `.title`/`.subtitle` (53-63, Фаза 34), `{% include "_header.html" %}` (110).

---

### `crates/trackly-app/templates/act_acceptance.html` (template, file-I/O)

**Аналог:** целевой паттерн блока подписей `act_handover.html` (D-06/D-07/D-09); контекст —
`document.giver_name`/`document.receiver_name` (уже в рендере, не меняется).

**Таблица с дублем ФИО (удалить строки, D-09) — текущее состояние строк 86-90:**
```html
<!-- Source: crates/trackly-app/templates/act_acceptance.html:86-90 (текущий HEAD) -->
<table class="kv">
  <tr><td class="key">Дата</td><td>{{ document.date_human }}</td></tr>
  <tr><td class="key">Кто передал</td><td>{{ document.giver_name }}</td></tr>
  <tr><td class="key">Кто принял</td><td>{{ document.receiver_name }}</td></tr>
</table>
```
Строки «Кто передал»/«Кто принял» удаляются — ФИО остаются только в блоке подписей (строка
«Дата» остаётся).

**Блок подписей (переверстать по D-06/D-07) — текущее состояние строк 103-106:**
```html
<!-- Source: crates/trackly-app/templates/act_acceptance.html:103-106 (текущий HEAD) -->
<div class="signature">
  <div class="line">Кто передал: {{ document.giver_name }}</div>
  <div class="line">Кто принял: {{ document.receiver_name }}</div>
</div>
```
CSS `.signature`/`.signature .line` (строки 71-77) сегодня НЕ рисует полоску подписи (`.line`
здесь просто `margin: 10pt 0`, текст «Кто передал: …» вписан прямо в строку) — это отличается от
`act_handover.html`'s `.signature .line` (`border-bottom`, пустой `<div>`). Приводя к общему виду
D-09, стоит скопировать оба класса (`.signature .line`, `.signature .sublabel`) из
`act_handover.html`'s CSS (строки 96-105) в `act_acceptance.html`'s `<style>` (или объединить их
в общий класс, если структура позволяет) — иначе полоска подписи в акте приёмки не появится
физически, только текстом.

**Doc-комментарий (обновить перечень, если меняется набор ключей) — строки 2-27:**
```html
<!-- Source: crates/trackly-app/templates/act_acceptance.html:11-16 (текущий HEAD) -->
Context (full org header, brought to parity with act_handover.html per
PRN-01/D-01/D-02/D-03 — Phase 20, plus org.full_name per Phase 34):
  org.{name,full_name,inn,kpp,address,address_line2,logo_data_uri,phone,fax,email,okpo,ogrn}
  device.{name,inventory_no,serial_no,model,condition}
  document.{giver_name, receiver_name, date_human}
```
Ключи не меняются (D-09 не трогает бэкенд/контекст) — комментарий требует правки только если
меняется описание структуры таблицы/подписей.

---

### `_legacy_defaults/v22/{act_handover,act_acceptance}.html` (НОВЫЕ, snapshot/fixture)

**Аналог:** `_legacy_defaults/v21/act_handover.html` и `.../v21/act_acceptance.html` —
Фаза 34 создала этот срез по тому же образцу перед своей правкой заголовка.

**Инструкция по срезу (уже задокументирована в коде, копировать процесс буквально):**
```rust
// Source: crates/trackly-app/src/pdf/html_templates.rs:56-63 (текущий HEAD)
/// **Extension point:** whenever a body in `DEFAULT_HTML_TEMPLATES` changes
/// again in a future phase, the PRE-CHANGE body MUST be captured as a new
/// snapshot (a new sibling directory, e.g. `_legacy_defaults/v21/`) and added
/// here as an additional entry in that filename's slice — otherwise installs
/// that predate THAT phase stop being recognized as untouched and silently
/// stop receiving upgrades. Forgetting this only causes a MISSED upgrade (file
/// stays on older-but-valid content), never a wrongful overwrite.
```
Практический шаг: **до** правки `act_handover.html`/`act_acceptance.html` скопировать их текущий
(Фаза-34-финальный) HEAD-контент байт-в-байт в
`crates/trackly-app/templates/_legacy_defaults/v22/act_handover.html` и
`.../v22/act_acceptance.html` (командой `cp`, не переписывая руками — байтовое сравнение в
`upgrade_untouched_defaults_on_startup` чувствительно к любому расхождению). `report.html` и
`_header.html` в v22 НЕ нужны — они не меняются этой фазой (директория `_legacy_defaults/v22/`
может содержать только эти два файла — сверить с тем, что v20/v21 при этом содержат ВСЕ три
файла; расхождение допустимо, т.к. `report.html` слайс для v22 просто не понадобится).

---

### `crates/trackly-app/src/pdf/html_templates.rs` (`KNOWN_LEGACY_DEFAULTS`, config/registry)

**Аналог:** добавление v21-элемента в Фазе 34 — тот же приём, +1 строка `include_str!` на
слайс `act_handover.html`/`act_acceptance.html`.

**Текущее состояние (строки 75-89, два релевантных слайса):**
```rust
// Source: crates/trackly-app/src/pdf/html_templates.rs:75-89 (текущий HEAD)
pub const KNOWN_LEGACY_DEFAULTS: &[(&str, &[&str])] = &[
    (
        "act_handover.html",
        &[
            include_str!("../../templates/_legacy_defaults/v20/act_handover.html"),
            include_str!("../../templates/_legacy_defaults/v21/act_handover.html"),
        ],
    ),
    (
        "act_acceptance.html",
        &[
            include_str!("../../templates/_legacy_defaults/v20/act_acceptance.html"),
            include_str!("../../templates/_legacy_defaults/v21/act_acceptance.html"),
        ],
    ),
    // "report.html" и "_header.html" — без изменений, не трогать
```
Требуется добавить третий `include_str!(".../v22/act_handover.html")` и
`.../v22/act_acceptance.html` элементом в конец каждого из двух слайсов (после v21). `report.html`
и `_header.html` записи НЕ меняются (фаза их не затрагивает).

**Тест-гейт, который проверит правку автоматически (не нужно писать заново):**
```rust
// Source: crates/trackly-app/src/pdf/html_templates.rs:394-407 (текущий HEAD)
#[test]
fn every_default_template_has_a_known_legacy_defaults_entry() { /* ... */ }
```
Уже существует и покрывает WR-06-инвариант «каждый файл в `DEFAULT_HTML_TEMPLATES` имеет запись
в `KNOWN_LEGACY_DEFAULTS`» — новых тестов сюда добавлять не нужно, C-01 закрывается только
самим содержимым слайсов (см. также `upgrade_replaces_v21_legacy_default_with_current_bundled_body`,
строки 413-463, как образец для гипотетического будущего `..._v22_...` теста, если планировщик
решит его завести — discretion, не обязателен).

---

### `crates/trackly-app/src/services/template_service.rs` (`demo_context_for_kind`, service)

**Аналог:** соседняя ветка `"act_acceptance"` того же `match` — уже содержит `document.giver_name`
в demo-fixture, паттерн переносится буквально в ветку `_` (act_handover).

**Текущее состояние ветки `_` (строки 502-529, отсутствует `giver_name`):**
```rust
// Source: crates/trackly-app/src/services/template_service.rs:502-529 (текущий HEAD)
// "act_handover" and any unrecognized kind — degrade gracefully to
// the act_handover demo context rather than erroring.
_ => serde_json::json!({
    "org": org,
    "act": {
        "number": "42",
        "suffix": null,
        "date": "2026-06-17",
        "date_human": "17 июня 2026",
        "receiver_name": "Петров П.П.",
        "location_name": "Офис 101",
        "deadline": null,
        "deadline_human": null,
        "parent": null,
        "items": [ /* ... */ ]
    }
}),
```
**Аналог рядом (act_acceptance ветка, строки 467-481) — уже имеет giver_name:**
```rust
// Source: crates/trackly-app/src/services/template_service.rs:467-481 (текущий HEAD)
"act_acceptance" => serde_json::json!({
    "org": org,
    "device": { /* ... */ },
    "document": {
        "giver_name": "Иванов И.И.",
        "receiver_name": "Петров П.П.",
        "date_human": "17 июня 2026"
    }
}),
```
Правка: добавить `"giver_name": "Иванов И.И."` (та же вымышленная форма, что уже используется в
соседней ветке) в `act`-объект ветки `_`, рядом с `"receiver_name": "Петров П.П."`. Это обязательная
правка (Pitfall 1 из RESEARCH.md) — без неё `validate_preview("act_handover", …)` упадёт под
`UndefinedBehavior::Strict` на ЛЮБОМ теле шаблона, включая нетронутый бандл, как только
`{{ act.giver_name }}` появится в `act_handover.html`.

---

### `tests/pdf_render_act.rs` (test, integration)

**Аналог:** сам файл до правки — три существующих теста нужно скорректировать под новое
поведение, паттерн вызова `create_handover_with_giver` + `render_pdf` сохраняется.

**`render_handover_act_produces_cyrillic_pdf` — комментарий устарел (строки 157-168), сама
ассерция технически совместима, но комментарий описывает старое поведение D-09 Фазы 15
(«giver_name отсутствует в теле») — обновить формулировку под D-06 (giver_name теперь снова в
теле, в блоке подписей).**

**`signature_renders_two_line_labels` — требует переписывания (строки 233-252):**
```rust
// Source: crates/trackly-app/tests/pdf_render_act.rs:233-252 (текущий HEAD)
/// Two-line signature sublabels (D-07): «Подпись»/«ФИО» под «Выдал»/«Получил».
/// N=1 is sufficient — the signature block does not vary with device count.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn signature_renders_two_line_labels() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let p = make_full_pipeline().await;
        let device_ids = seed_devices(&p.writer, 1).await;
        let act = create_handover_with_giver(&p.acts, &device_ids, "Иванов И.И.").await;
        let html = p.acts.render_pdf(act.id).await.expect("render_pdf");
        for expected in ["Выдал", "Получил", "Подпись", "ФИО"] {
            assert!(
                html.contains(expected),
                "expected signature label {expected:?} missing. Head: {:?}",
                html.chars().take(500).collect::<String>()
            );
        }
    })
    .await
    .expect("timeout");
}
```
D-06/D-07 отменяют двухстрочные подписи: `"ФИО"` больше не должна присутствовать как отдельная
подпись под именем (пояснение убрано), но напечатанное имя (например, `"Иванов И.И."`) теперь
ДОЛЖНО появляться в блоке подписей. Переписать тест: заменить набор ожидаемых строк на
`["Выдал", "Получил", "Подпись"]` (без «ФИО») и добавить отдельную ассерцию, что фактическое ФИО
подписанта (`act.giver_name`/`"Иванов И.И."`) присутствует в HTML. Переименовать функцию (например,
`signature_renders_giver_name_horizontal_block`), т.к. «two_line» больше не описывает поведение.

**`render_handover_multi_device_wraps_long_fields` — вероятно не требует правки контента
ассерций** (проверяет длинный `kit`/`specs`-текст без урезания — D-11 меняет разметку, но не
семантику проверки «текст не обрезан»), но стоит перечитать после правки шаблона на предмет
завязки на `<span class="value">`.

---

### `tests/html_act_render.rs` (test, integration)

**Аналог:** сам файл до правки — паттерн `for expected in [...] { assert!(html.contains(...)) }`
уже используется, меняется только список ожидаемых строк.

**`html_handover_contains_required_blocks_and_logo` — требует правки (строки 188-195):**
```rust
// Source: crates/trackly-app/tests/html_act_render.rs:188-195 (текущий HEAD)
for expected in ["Акт приема-передачи", "Выдал", "Получил", "Подпись", "ФИО"]
{
    assert!(
        html.contains(expected),
        "expected block/label {expected:?} missing from handover HTML. Head: {:?}",
        html.chars().take(500).collect::<String>()
    );
}
```
`"ФИО"` подлежит удалению из списка (D-07 убирает эту сублейбл-подпись) — этот файл НЕ был назван
в CONTEXT.md C-03, но найден research'ем (Pitfall 3) как содержащий идентичную ломающуюся
ассерцию. **Обязательно включить в files_modified.**

**`html_acceptance_contains_required_blocks` (строки 212-240) — вероятно не требует правки**
(проверяет только заголовок документа + оба ФИО текстом, оба остаются истинными после D-09 —
ФИО просто переезжают из таблицы в блок подписей, но продолжают присутствовать где-то в HTML).
Перепроверить после правки `act_acceptance.html`, что `"Отдалов О.О."`/`"Принялов П.П."`
по-прежнему появляются (теперь только в блоке подписей, а не дважды).

---

### `tests/acts_e2e_smoke.rs` (test, integration e2e smoke)

**Аналог:** сам файл до правки — ассерции технически совместимы, требуется только правка
устаревших комментариев.

**`handover_pdf_render_within_e2e` (строки 259-294) — комментарий устарел:**
```rust
// Source: crates/trackly-app/tests/acts_e2e_smoke.rs:288-291 (текущий HEAD)
// D-09 (Phase 15 plan 02) removed giver_name from the rendered body —
// it now only appears via the bare "Выдал" signature label.
// receiver_name is the D-09 intro-paragraph value that IS rendered.
assert!(html.contains("Петров"));
```
Ассерция (`html.contains("Петров")`) остаётся истинной после D-06 (receiver_name всё ещё в
интро), но комментарий описывает уже отменённое поведение — обновить формулировку. Само
переименование/добавление ассерции не обязательно (research: «ассерция технически совместима»).

**`acceptance_pdf_render_smoke` (строки 297-321) — не требует правки ассерций**, проверяет
`html.contains("Иванов") && html.contains("Пётр")` — обе строки продолжают появляться в HTML
после D-09 (переезд из таблицы в блок подписей не убирает их из документа).

---

### `tests/html_field_row_underline_gate.rs` (НОВЫЙ, опционально, structural gate)

**Аналог:** `tests/html_page_parity.rs` — целиком копируется приём «`include_str!` +
regex-извлечение блока + assert», меняется только извлекаемый паттерн и утверждение.

**Полный образец для копирования (см. также `tests/html_header_parity.rs` как второй пример
того же приёма, если нужна сверка):**
```rust
// Source: crates/trackly-app/tests/html_page_parity.rs:1-49 (текущий HEAD, целиком)
//! D-13 structural regression guard (Phase 33, Plan 02): the three shipped
//! HTML print templates (`act_handover.html`, `act_acceptance.html`,
//! `report.html`) must declare byte-identical `@page { size; margin }`
//! blocks. PRV-02's cross-document-consistency guarantee and Paged.js's
//! pagination (D-04) both depend on the three documents sharing the exact
//! same page geometry — a desync here would silently break either the
//! WYSIWYG preview/print match or make one document's pages a different
//! physical size than the other two.
//!
//! Reads the templates via `include_str!` (compile-time, relative to this
//! test file's own location) rather than a runtime `std::fs::read_to_string`
//! with a CWD-relative path, so the test is independent of `cargo test`'s
//! working directory. Per D-01, this test only READS the templates — it
//! never modifies `crates/trackly-app/templates/*.html`.

const ACT_HANDOVER_HTML: &str = include_str!("../templates/act_handover.html");
const ACT_ACCEPTANCE_HTML: &str = include_str!("../templates/act_acceptance.html");
const REPORT_HTML: &str = include_str!("../templates/report.html");

/// Extracts the first `@page { ... }` block from `text`, panicking with
/// `label` in the message if no match is found.
fn extract_page_block(text: &str, label: &str) -> String {
    let re = regex::Regex::new(r"(?s)@page\s*\{[^}]*\}").expect("valid regex");
    re.find(text)
        .unwrap_or_else(|| panic!("{label}: no @page block found in template"))
        .as_str()
        .to_string()
}

#[test]
fn all_three_templates_share_identical_page_block() {
    let handover = extract_page_block(ACT_HANDOVER_HTML, "act_handover.html");
    let acceptance = extract_page_block(ACT_ACCEPTANCE_HTML, "act_acceptance.html");
    let report = extract_page_block(REPORT_HTML, "report.html");

    assert_eq!(
        handover, acceptance,
        "act_handover.html and act_acceptance.html @page blocks differ:\n\
         act_handover.html:\n{handover}\n\
         act_acceptance.html:\n{acceptance}"
    );
    assert_eq!(
        acceptance, report,
        "act_acceptance.html and report.html @page blocks differ:\n\
         act_acceptance.html:\n{acceptance}\n\
         report.html:\n{report}"
    );
}
```
Для гейта DOC-07 (нет `border-bottom` в теле): вместо извлечения `@page {...}` regex'ом,
извлечь диапазон текста МЕЖДУ `{% include "_header.html" %}` и `<div class="signatures">` (или
до открывающего `<div class="signature` включительно) в `act_handover.html`, и assert'ить, что
в этом диапазоне НЕТ подстроки `border-bottom` — с явным исключением диапазона «Сроком до»
(D-03), если тот реализован через выделенный класс с собственным `border-bottom` внутри этого же
среза. Не обязателен (Claude's Discretion, RESEARCH.md Open Question 1) — заводить, если дёшево
(~15-20 строк по этому образцу).

## Shared Patterns

### MiniJinja autoescape — новая интерполяция без `| safe`
**Источник:** `crates/trackly-app/templates/act_handover.html` (текущее использование
`{{ act.receiver_name }}` в интро, строка 117) + doc-комментарий T-16-01 (строки 24-30).
**Применить к:** `{{ act.giver_name }}` в новом блоке подписей `act_handover.html`,
`{{ document.giver_name }}`/`{{ document.receiver_name }}` в переверстанном блоке подписей
`act_acceptance.html`.
```html
<!-- Обычная HTML-экранированная интерполяция — НЕ добавлять | safe -->
<span class="value">{{ act.receiver_name }}</span>
```
Единственное существующее исключение с `| safe` в этих файлах — `org.logo_data_uri | safe`
(сервер-конструируемый base64 data: URI) — этот прецедент НЕ применим к ФИО, не копировать его
паттерн для новых полей.

### Legacy-defaults slice — доставка изменённого дефолта в установленные копии
**Источник:** `crates/trackly-app/src/pdf/html_templates.rs:46-103` (`KNOWN_LEGACY_DEFAULTS` +
инструкция) — уже реализовано и покрыто тестами (Фазы 16/34).
**Применить к:** обоим правящимся файлам (`act_handover.html`, `act_acceptance.html`) — снимок
ДО правки → `_legacy_defaults/v22/` → дополнительный элемент в оба слайса. Механизм апгрейда
(`upgrade_untouched_defaults_on_startup`) и материализация (`materialize_defaults_on_startup`)
НЕ трогаются — они уже итерируют `KNOWN_LEGACY_DEFAULTS` структурно, новых веток кода не нужно.

### `UndefinedBehavior::Strict` — синхронизация production/demo контекстов
**Источник:** `crates/trackly-app/src/services/template_service.rs:434-439` (doc-комментарий) +
`act_service.rs:2639` (production-ключ `act.giver_name` уже существует).
**Применить к:** `demo_context_for_kind`'s ветка `_` (act_handover) — единственная правка,
необходимая для синхронизации с новым использованием ключа в шаблоне. Инвариант: любой ключ,
впервые прочитанный шаблоном, обязан появиться в ОБОИХ JSON-контекстах (боевом и демо)
одновременно — иначе живой редактор шаблонов (`Settings → Шаблоны`) ломается независимо от
production-рендера.

## No Analog Found

Нет файлов без аналога — весь набор правок этой фазы имеет либо аналог «то же место до правки»,
либо готовый паттерн в соседнем файле того же домена (`templates/`, `src/pdf/`,
`src/services/template_service.rs`, `tests/`).

## Metadata

**Область поиска аналогов:** `crates/trackly-app/templates/`, `crates/trackly-app/src/pdf/`,
`crates/trackly-app/src/services/template_service.rs`, `crates/trackly-app/tests/*.rs`.
**Файлов просмотрено:** `act_handover.html` (205 строк, целиком), `act_acceptance.html`
(110 строк, целиком), `html_templates.rs` (688 строк, целиком), `template_service.rs`
(строки 374-530), `pdf_render_act.rs` (строки 120-294), `html_act_render.rs` (строки 164-241),
`acts_e2e_smoke.rs` (строки 259-321), `html_page_parity.rs` (49 строк, целиком),
`_legacy_defaults/v20/`, `_legacy_defaults/v21/` (листинг каталогов).
**Дата извлечения паттернов:** 2026-08-11
