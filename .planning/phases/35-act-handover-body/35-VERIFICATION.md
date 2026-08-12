---
phase: 35-act-handover-body
verified: 2026-08-12T10:20:00Z
status: passed
score: 5/5 roadmap Success Criteria verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/5
  gaps_closed:
    - "Success Criterion #4 / DOC-08 (WR-02) — `.signature-row .signature-name` в act_handover.html и act_acceptance.html больше не `white-space: nowrap` без `min-width: 0`. Правило заменено на `min-width: 0; white-space: normal; overflow-wrap: break-word;` в ОБОИХ шаблонах (Plan 35-07, коммит f162c79). Закрытие подтверждено тремя независимыми уровнями: (1) прямое чтение CSS обоих файлов подтверждает новые три объявления и отсутствие голого nowrap в правиле; (2) новый структурный CSS-гейт `signature_name_css_permits_wrap_for_long_names` (html_field_row_underline_gate.rs) читает правило по селектору и проверяет наличие/отсутствие нужных токенов для обоих шаблонов — не текстовый html.contains; (3) два новых теста полного рендер-пайплайна (pdf_render_act.rs) с вымышленной 53-символьной ФИО-фикстурой подтверждают, что имя доходит без обрезания именно в `<span class=\"signature-name\">`; (4) блокирующий human-verify чекпоинт (Plan 35-07 Task 3) подтверждён пользователем на обоих транспортах (десктоп + LAN-браузер) для обоих актов."
  gaps_remaining: []
  regressions: []
gaps: []
deferred: []
human_verification: []
---

# Phase 35: Тело акта приёма-передачи Verification Report

**Phase Goal:** Текст акта приёма-передачи составлен в каноничной форме документа (две
стороны, «составили настоящий акт о нижеследующем», перечень, состояние, срок, подписи),
согласован с пользователем до вёрстки, без полосок-подчёркиваний под автоматически
подставляемым текстом, с горизонтальным блоком подписей — по строке на каждого подписанта
с автоподставленными ФИО.

**Verified:** 2026-08-12
**Status:** passed
**Re-verification:** Yes — после закрытия гэпа Плана 35-07 (Wave 6)

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Финальный текст акта явно согласован с пользователем ДО вёрстки тела, зафиксировано как первый шаг фазы | ✓ VERIFIED (regression-checked) | Без изменений с прошлой верификации. `35-CONTEXT.md` D-01 фиксирует согласование текста Word-образца как первый шаг фазы (Context, не финальный шаг). Текст сохранён дословно в `act_handover.html:131` («Настоящим актом утверждаю, что мною: {{ act.receiver_name }}»). Ни один план фазы, включая 35-07, не трогал интро-текст. |
| 2 | Согласованный текст сверен с каноном вёрстки Word-образца; изменения не ломают Фазы 15/16 | ✓ VERIFIED (regression-checked) | Гейт `{%- if act.items | length == 1 %}` по-прежнему отсутствует (`grep -c "act.items | length == 1" act_handover.html` → 0) — каждый `.device-block` печатает своё имя устройства независимо от N. Регресс-тест `render_handover_multi_device_fields_attributable_to_own_device` (`pdf_render_act.rs:354`) присутствует и не тронут Планом 35-07. |
| 3 | Нет полосок-подчёркиваний под автоподставляемыми полями; полоски остаются только там, где расписываются от руки | ✓ VERIFIED (regression-checked) | `grep -n border-bottom act_handover.html` → 2 совпадения (`.value-blank` строка 75, `.signature-field .signature-line` строка 110); `act_acceptance.html` → 1 совпадение (`.signature-field .signature-line` строка 96). Оба структурных гейта (`field_row_css_has_no_border_bottom_and_only_two_legit_exceptions_remain`, `acceptance_signature_line_css_has_exactly_one_legitimate_border_bottom`) присутствуют и не тронуты Планом 35-07. |
| 4 | Блок подписей горизонтальный, отдельная строка на каждого подписанта, ФИО автоподставляются из `act.giver_name`/`act.receiver_name` без изменений бэкенда | ✓ VERIFIED — **гэп закрыт** | Структура/данные (не менялись, остаются верны): `.signature-row` — `display:flex`, отдельная строка на `Выдал:`/`Получил:`, `{{ act.giver_name }}`/`{{ act.receiver_name }}` в act_handover.html:181,189; `{{ document.giver_name }}`/`{{ document.receiver_name }}` в act_acceptance.html:138,146. Новое (закрывает WR-02): `.signature-row .signature-name` в ОБОИХ файлах теперь `min-width: 0; white-space: normal; overflow-wrap: break-word;` (подтверждено прямым чтением файлов на HEAD, строки 117-121 и 103-107 соответственно) — длинное кириллическое ФИО может переноситься внутри доступной ширины строки вместо ухода за печатную область. `git diff --stat 905289c..HEAD -- crates/trackly-app/src/services/` → 1 файл, 1 строка (template_service.rs demo-context добавление ключа `giver_name` для предпросмотра редактора шаблонов — не бизнес-логика рендера актов, не «изменение бэкенда» в смысле критерия #4/D-06). |
| 5 | Рендер настоящего PDF/превью подтверждает вёрстку на обоих транспортах (десктоп и LAN-браузер) | ✓ VERIFIED (human) | Подтверждено дважды в рамках фазы: Plan 35-06 Task 4 (атрибуция multi-device) и Plan 35-07 Task 3 (перенос длинного ФИО) — оба блокирующих human-verify чекпоинта одобрены пользователем на десктопе и в LAN-браузере, для обоих печатных актов (приёма-передачи и приёмки), с проверкой длинного вымышленного ФИО, максимально близкого к порогу переполнения (53 символа против расчётных ~47-52 символов бюджета строки). |

**Score:** 5/5 roadmap Success Criteria verified.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-app/templates/act_handover.html` | Каноничный текст, без подчёркиваний под автоподстановкой, горизонтальные подписи с переносом ФИО | ✓ VERIFIED | Прямое чтение файла на HEAD подтверждает все элементы: D-01 текст, D-10 отсутствие лишних border-bottom, D-06/D-07 горизонтальные подписи, WR-02-фикс переноса. |
| `crates/trackly-app/templates/act_acceptance.html` | Паритет с act_handover.html по блоку подписей (D-09) | ✓ VERIFIED | Идентичный CSS-фикс применён; таблица `Кто передал`/`Кто принял` отсутствует (дедуп D-09 подтверждён — grep по файлу не находит этих строк). |
| `crates/trackly-app/templates/_legacy_defaults/v23/act_handover.html` + `.../act_acceptance.html` | Пре-фикс снимок для доставки установленным копиям | ✓ VERIFIED | Присутствуют, `git hash-object` подтверждает их отличие от текущего HEAD-тела (`5f5fdee…`/`f2f35fb…` vs `e411cd2…`/`12a2d0f…`), зарегистрированы четвёртым элементом в `KNOWN_LEGACY_DEFAULTS` (html_templates.rs:81,90). |
| `crates/trackly-app/src/pdf/html_templates.rs::upgrade_replaces_v23_legacy_default_with_current_bundled_body` | Регрессионный тест доставки v23 | ✓ VERIFIED | Присутствует (строка 537), структура зеркалит v22-тест, содержит небанальный precondition-guard `assert_ne!`. |
| `crates/trackly-app/tests/html_field_row_underline_gate.rs::signature_name_css_permits_wrap_for_long_names` | Структурный CSS-гейт переноса ФИО | ✓ VERIFIED | Присутствует (строка 125), итерируется по обоим шаблонам, проверяет 4 условия (наличие 3 нужных токенов + отсутствие голого nowrap) через `extract_rule_body`. |
| `crates/trackly-app/tests/pdf_render_act.rs` (два новых теста с `LONG_GIVER_NAME_FICTIONAL`) | Доказательство целостности длинного ФИО через полный пайплайн | ✓ VERIFIED | `render_handover_with_long_giver_name_preserves_full_name_in_signature_block` и `render_acceptance_with_long_giver_name_preserves_full_name_in_signature_block` присутствуют, фикстура «Сидоров-Петроградский-Константинов Иван Александрович» (53 символа, вымышленная) объявлена и используется, точное сравнение содержимого `<span class="signature-name">`. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `.signature-name` (печатное ФИО) | Печатная ширина страницы | CSS flexbox (`display:flex` строка, фиксированный сосед, теперь `min-width:0`/`white-space:normal`/`overflow-wrap:break-word` на имени) | ✓ WIRED | Ранее NOT SAFELY WIRED (WR-02) — закрыто Планом 35-07. Флекс-элемент теперь может сжиматься и переносить текст ниже полного unbroken-width. |
| `act_handover.html` `{{ act.giver_name }}`/`{{ item.name }}` (per-block) | `act_service.rs::render_pdf` ctx | MiniJinja-интерполяция, цикл `.device-block` | ✓ WIRED | Без изменений с прошлой верификации; `git diff --stat` для `src/services/` за весь диапазон фазы — 1 строка, только demo-контекст редактора шаблонов. |
| `html_templates.rs::KNOWN_LEGACY_DEFAULTS` | `_legacy_defaults/v23/*.html` | `include_str!` | ✓ WIRED | Подтверждено регрессионным тестом `bodies.get(3)`. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| `act_handover.html`/`act_acceptance.html` signature block | `act.giver_name`/`document.giver_name` | `act_service.rs`, DB-backed, обязательное поле формы | Да — не статика | ✓ FLOWING, и теперь корректно отображается при любой длине |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `.signature-name` не форсирует `nowrap` в обоих шаблонах | `grep -A2 "signature-row .signature-name" act_handover.html \| grep -c "white-space: nowrap"` и то же для act_acceptance.html | 0, 0 | ✓ PASS |
| `min-width: 0` присутствует ровно один раз в обоих шаблонах | `grep -c "min-width: 0"` | 1, 1 | ✓ PASS |
| v23-снимок зарегистрирован | `grep -c "v23/act_handover.html"` / `"v23/act_acceptance.html"` в html_templates.rs | 1, 1 | ✓ PASS |
| Гейт `length == 1` по-прежнему отсутствует (регресс-проверка SC#2) | `grep -c "act.items \| length == 1" act_handover.html` | 0 | ✓ PASS |
| `border-bottom` ровно 2/1 (регресс-проверка SC#3) | `grep -n border-bottom` на каждом файле | 2, 1 | ✓ PASS |
| Бэкенд не тронут за весь диапазон фазы | `git diff --stat 905289c..HEAD -- crates/trackly-app/src/services/` | 1 файл, 1 строка (demo-контекст ключ) | ✓ PASS |
| Приватность: только вымышленные ФИО/реквизиты в изменённых файлах | `./scripts/check-privacy-requisites.sh` | "Privacy gate OK" | ✓ PASS |
| Долговые маркеры (TBD/FIXME/XXX) в файлах, тронутых фазой | `grep -n -E "TBD\|FIXME\|XXX"` по всем файлам из `files_modified`/`key-files` всех 7 планов | нет совпадений | ✓ PASS |
| Полный тест-сьют (по данным текущей сессии, не перезапускался повторно во избежание contention на `target/`) | `cargo test -p trackly-app -- --test-threads=1 --skip login_remember_persistent_cookie` | 667 passed / 0 failed (90 targets); остальной workspace — 204 passed / 0 failed | ✓ PASS (данные текущей сессии) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| DOC-07 | 35-02, 35-04, 35-05, 35-06 | Нет полосок-подчёркиваний под автоподставляемым текстом | ✓ SATISFIED | Структурные гейты покрывают оба шаблона; ровно 2 и 1 легитимных исключения соответственно. REQUIREMENTS.md отмечает Complete. |
| DOC-08 | 35-01…35-07 | Горизонтальный блок подписей, автоподстановка ФИО без изменений бэкенда, читаемость при любой длине ФИО | ✓ SATISFIED | Структура/данные подтверждены с прошлой верификации; перенос длинного ФИО закрыт Планом 35-07 (CSS-гейт + тесты полного пайплайна + human-verify на обоих транспортах). REQUIREMENTS.md отмечает Complete. |
| DOC-09 | 35-02, 35-04, 35-05, 35-06 | Текст акта в каноничной форме, согласован до вёрстки | ✓ SATISFIED | D-01 (согласование текста Word-образца) и D-02a (per-device атрибуция) подтверждены. REQUIREMENTS.md отмечает Complete. |

Осиротевших требований нет: REQUIREMENTS.md сопоставляет с Фазой 35 только DOC-07/DOC-08/DOC-09,
все три фигурируют в `requirements:` фронтматтере хотя бы одного из 7 планов и отмечены Complete
и в таблице трассировки, и в чекбоксах.

### Anti-Patterns Found

Блокирующих находок (TBD/FIXME/XXX без ссылки на issue) нет ни в одном файле, тронутом фазой.
Ниже — WARNING/INFO из `35-REVIEW.md` (третий проход, после Плана 35-07), которые НЕ блокируют
критерии успеха этой фазы, но требуют явного решения по каждой значимой позиции:

| File | Pattern | Severity | Impact / Решение |
|------|---------|----------|--------|
| `crates/trackly-app/src/pdf/html_templates.rs` | WR-01: из 6 промежуточных состояний тел, существовавших внутри фазы, зарегистрировано только пре-Плана-07 состояние (`v22`) и пре-фикс-состояние Плана 07 (`v23`); три других внутрифазовых промежуточных тела (в т.ч. `5cc4ecf` — тело с ещё не закрытым CR-01) не зарегистрированы | ⚠️ Warning, **не гэп этой фазы** | Разобрано ниже отдельно — риск ограничен dev/UAT-машинами, ни один выпущенный релизный тег не несёт непроверенного промежуточного тела. |
| `act_handover.html` / `act_acceptance.html` | Блок подписей (~48 строк CSS+разметки) продублирован байт-в-байт вместо общего партиала (WR-08 в предыдущем ревью, WR-07 в текущем) | ℹ️ Info/качество кода | Каждая будущая правка подписей требует синхронной правки двух файлов — известный, принятый риск сопровождения, не влияет на текущие критерии успеха. |
| `crates/trackly-app/tests/pdf_render_act.rs:206-210`, `html_act_render.rs:204-207` | Вакуозные ассерты номера акта (WR-05 в текущем ревью) | ℹ️ Info/качество тестов | Предсуществующая находка, не введена этой фазой, не влияет на критерии успеха. |
| `act_handover.html:166` | «Сроком до: ___» печатается безусловно и на актах возврата (WR-02 в текущем ревью) | ℹ️ Info/продуктовое поведение | Прямое следствие D-03 (пользовательское решение фазы); не переоткрывается в рамках этой верификации. |
| `template_service.rs:508` | `"suffix": null` → литерал `None` в preview редактора (WR-03 в текущем ревью) | ℹ️ Info | Подтверждено git blame как предсуществующее (до Фазы 35), вне scope. |

**Отдельное рассуждение по WR-01 (почему это не гэп, а WARNING):** доступные факты (перепроверены
в этой сессии): (а) все выпущенные релизные теги (`v1.1.x`…`v1.3.2`) несут тело снимка `v21`,
зарегистрированного заранее — ни одна выпущенная установка не застрянет на незарегистрированном
промежуточном теле; (б) незарегистрированные тела существовали только внутри окна разработки
самой Фазы 35 (между коммитами `c74a579`..`bbfed54`), то есть до тега; (в) единственная машина
с материализованной копией на этой сессии (текущий dev-бокс) уже синхронизирована с bundled-телом
HEAD. Экспозиция ограничена гипотетической dev/UAT-машиной, которая запускала `cargo tauri dev`
именно в этом узком временном окне фазы и с тех пор не запускалась повторно — то есть тем самым
окном, где стоял блокирующий human-verify гейт Плана 35-06/35-07, который как раз и должен был
такую машину поймать (и, по данным текущей сессии, реально поймал устаревшую материализованную
копию `target/debug/templates/` перед UAT Плана 35-07 — она была обнаружена и удалена вручную).
Критерии успеха фазы (SC#1-5) описывают состояние текущего кода и его наблюдаемое поведение на
HEAD, а не гарантию доставки для каждого промежуточного commit фазы — WR-01 остаётся релевантным
техдолгом для `html_templates.rs`, но не является провалом ни одного из 5 критериев успеха.

### Human Verification Required

Не требуется — оба критерия, нуждавшихся в живой проверке (SC#5 и визуальная часть SC#4),
подтверждены пользователем в рамках блокирующих human-verify чекпоинтов этой фазы (Plan 35-06
Task 4, Plan 35-07 Task 3), с явным упоминанием обоих транспортов (десктоп + LAN-браузер) и
обоих печатных актов.

### Gaps Summary

Единственный оставшийся гэп прошлой верификации (Success Criterion #4 / DOC-08, WR-02 —
`.signature-row .signature-name` объявлен `white-space: nowrap` без права на перенос) **закрыт**
Планом 35-07: CSS-правило заменено на `min-width: 0; white-space: normal; overflow-wrap:
break-word;` в обоих печатных шаблонах (`act_handover.html`, `act_acceptance.html`), закрытие
подтверждено структурным CSS-гейтом, двумя тестами полного рендер-пайплайна с вымышленной
53-символьной ФИО-фикстурой и блокирующей человеческой проверкой на обоих транспортах для обоих
актов. Регрессий по ранее закрытым критериям (SC#1-3, SC#5, ранее закрытый SC#2/CR-01) не
обнаружено. Бэкенд не тронут за весь диапазон фазы (единственная строка правки в
`src/services/` — добавление ключа в demo-контекст предпросмотра редактора шаблонов, не
затрагивающее рендер реальных актов).

Все 3 требования фазы (DOC-07, DOC-08, DOC-09) удовлетворены и подтверждены в REQUIREMENTS.md.
Найдена одна новая находка кодревью (WR-01 — неполная регистрация внутрифазовых промежуточных
тел в `KNOWN_LEGACY_DEFAULTS`), классифицированная как WARNING/техдолг, а не как провал критерия
успеха фазы — обоснование приведено выше в разделе «Anti-Patterns Found».

---

*Verified: 2026-08-12T10:20:00Z*
*Verifier: Claude (gsd-verifier)*
