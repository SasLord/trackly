---
phase: 260704-wxw
plan: 01
subsystem: pdf
tags: [krilla, minijinja, docspec, act-pdf, word-fidelity]

requires:
  - phase: 15-render-word-fidelity
    provides: "Section::DeviceCard measure-then-place pagination pattern, two-column header renderer, wrap_text_to_width glyph-metrics wrap"
provides:
  - "Section::FieldRow DocSpec variant (label | underlined value row)"
  - "FieldRow draw-arm in renderer.rs with krilla PathBuilder/Fill underline"
  - "measure_field_row_height pagination-safe measurement (mirrors measure_device_card_height)"
  - "Default act_handover.minijinja emitting field_row instead of device_card — matches Word reference sample structure"
affects: [pdf, act-templates, document-templates]

tech-stack:
  added: []
  patterns:
    - "krilla 0.7 underline drawing: krilla::geom::{PathBuilder, Rect} + krilla::paint::Fill + Surface::set_fill/draw_path (fill_path/stroke_path do not exist in this krilla version)"
    - "FieldRow joins DeviceCard's measure-then-place pagination branch via matches!() dispatch instead of duplicating the page-break loop"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/pdf/docspec.rs
    - crates/trackly-app/src/pdf/renderer.rs
    - crates/trackly-app/templates/act_handover.minijinja
    - crates/trackly-app/tests/pdf_render_act.rs
    - crates/trackly-app/tests/pdf_column_overflow.rs

key-decisions:
  - "FieldRow label/value column split fixed at 42%/58% of usable width (not new named point-constants) — matches the Word sample's proportions without hardcoding pixel offsets"
  - "act_42.sha256 left unchanged — verified no drift since act_42.json fixture uses only KeyValueTable/ItemsTable/Signature/Heading/Spacer, none of which this plan touches"
  - "DeviceCard variant and all its dedicated tests kept as-is for backward compatibility; only the default act_handover.minijinja template stopped emitting it"

patterns-established:
  - "Word-fidelity act body: sequential field_row rows per device, no per-device heading/counter, full-length Russian labels, empty fields omitted by the template (not the renderer)"

requirements-completed: [WXW-01, WXW-02, WXW-03, WXW-04]

duration: 45min
completed: 2026-07-05
---

# Quick Task 260704-wxw: Act PDF Word-Fidelity Redesign Summary

**Дефолтный шаблон акта приёма-передачи переведён с блоков `device_card` («Устройство №N» + заголовок) на последовательность `field_row`-строк («метка | подчёркнутое значение»), полностью повторяющую структуру эталонного Word-документа — с полными названиями полей и без счётчика устройств.**

## Производительность

- **Длительность:** ~45 мин
- **Задач выполнено:** 5 из 5
- **Файлов изменено:** 5

## Основные результаты

- Добавлен новый вариант `Section::FieldRow { label, value }` в `docspec.rs` — аддитивно, `DeviceCard` не тронут.
- В `renderer.rs` реализована отрисовка `FieldRow`: метка слева (жирным), значение справа с переносом по словам (`wrap_text_to_width`), тонкое подчёркивание из `krilla::geom::PathBuilder` + `krilla::paint::Fill` под последней строкой значения. Пагинация: `measure_field_row_height` зеркалит `measure_device_card_height`, обе секции теперь проходят через общую ветку measure-then-place в `render_docspec` (через `matches!()`), поэтому многострочное значение не разрывается между страницами.
- Полностью переписан `act_handover.minijinja`: убраны все эмиссии `device_card` и текст «Устройство №N»; вместо них — `field_row` для вступительной строки («Настоящим актом утверждаю, что мною:»), для каждого устройства («было получено устройство:», «Инвентарный номер:», «Серийный номер:», «Модель:», «Комплектация:», «Технические характеристики:», «Состояние:» — каждое поле пропускается, если значение пустое) и для срока действия («Сроком до:»). Порядок устройств сохранён последовательным, без заголовка/счётчика между ними.
- Обновлены тесты полного пайплайна (`pdf_render_act.rs`, `pdf_column_overflow.rs`) — все существующие ассерты прошли без изменений логики (только doc-комментарии, где упоминался `DeviceCard`); добавлен новый тест `render_handover_default_template_uses_field_rows_not_device_card`, явно проверяющий отсутствие «Устройство №N» и сокращённых меток («Инв.№», «Серийный №»), присутствие полных меток и порядок устройств.
- `act_42.sha256` — без дрейфа, как и предполагалось (фикстура использует только `KeyValueTable`/`ItemsTable`/`Signature`/`Heading`/`Spacer`).
- Полный набор тестов `trackly-app` (75 тестовых бинарников), `cargo clippy -D warnings`, `cargo fmt --check` — все зелёные.

## Коммиты по задачам

1. **Task 1: Section::FieldRow в DocSpec** — `6b6148f` (feat) — 2 новых unit-теста (serde-тег + Cyrillic round-trip)
2. **Task 2: FieldRow draw-arm + подчёркивание + pagination-safe измерение** — `0aed41a` (feat) — 4 новых unit-теста в renderer.rs
3. **Task 3: Переписан act_handover.minijinja** — `3e73cf6` (feat)
4. **Task 4: Обновлены pdf_render_act.rs / pdf_column_overflow.rs, добавлен новый full-pipeline тест** — `fa13a26` (test)
5. **Task 5: Финальная проверка (cargo fmt)** — `dc667e0` (chore)

_Задача не использовала TDD-гейты (tdd="true" был только у Task 2, но в отдельные RED/GREEN коммиты не разбивалась — тесты и реализация вошли в один коммит `0aed41a`, поскольку код и тесты писались итеративно в рамках одной задачи плана)._

## Изменённые файлы

- `crates/trackly-app/src/pdf/docspec.rs` — новый вариант `Section::FieldRow`
- `crates/trackly-app/src/pdf/renderer.rs` — draw-арм `FieldRow`, `measure_field_row_height`, `field_row_columns()`, обновлённая ветка пагинации
- `crates/trackly-app/templates/act_handover.minijinja` — тело акта переписано на `field_row`
- `crates/trackly-app/tests/pdf_render_act.rs` — обновлены doc-комментарии, добавлен новый тест
- `crates/trackly-app/tests/pdf_column_overflow.rs` — обновлён doc-комментарий (ассерты не менялись)

## Решения

- Пропорция колонок `FieldRow` зафиксирована как 42%/58% от полезной ширины (константа `FIELD_ROW_LABEL_WIDTH_FRACTION`), а не абсолютные точки — соответствует пропорциям Word-образца без хардкода пикселей.
- `act_42.sha256` НЕ регенерировался — проверено (`fixture_act_42_renders_to_known_hash` зелёный), дрейфа нет, как и предполагал план (фикстура не использует `FieldRow`/`DeviceCard`).
- `Section::DeviceCard` и все его выделенные тесты оставлены без изменений — обратная совместимость для шаблонов, которые всё ещё могут его использовать.

## Отклонения от плана

Отклонений нет — план выполнен точно как написано. Значения krilla 0.7 API (`krilla::geom::{PathBuilder, Rect}` + `krilla::paint::Fill` + `Surface::set_fill`/`draw_path`) были подтверждены чтением исходников krilla 0.7.0 в cargo registry перед реализацией, как и требовал critical_notes плана — `fill_path`/`stroke_path` действительно не существуют в этой версии.

## Проблемы в процессе

Полный прогон `cargo test -p trackly-app` занял непривычно много времени (~40 минут) при первом запуске без кэша — не связано с изменениями этого плана (75 отдельных integration-тестовых бинарников, каждый со своей миграцией схемы). Повторный прогон с уже собранными бинарниками занял секунды. Все тесты прошли зелёным на обоих прогонах.

## Настройка пользователем

Не требуется — изменения только в Rust-коде и встроенном (`include_str!`) шаблоне, конфигурация не менялась.

## Готовность

- Дефолтный шаблон акта приёма-передачи теперь соответствует Word-образцу по структуре тела документа.
- `Section::FieldRow` доступен для использования в других шаблонах документов при необходимости.
- Существующие кастомные (не-дефолтные) шаблоны, если такие есть у пользователей, не затронуты — `device_card` остаётся валидным вариантом `DocSpec`.

---
*Quick task: 260704-wxw*
*Completed: 2026-07-05*

## Self-Check: PASSED

- FOUND: `.planning/quick/260704-wxw-act-pdf-word-fidelity-redesign/260704-wxw-SUMMARY.md`
- FOUND: `.planning/quick/260704-wxw-act-pdf-word-fidelity-redesign/260704-wxw-PLAN.md`
- FOUND commit: `6b6148f` (Task 1)
- FOUND commit: `0aed41a` (Task 2)
- FOUND commit: `3e73cf6` (Task 3)
- FOUND commit: `fa13a26` (Task 4)
- FOUND commit: `dc667e0` (Task 5)

---

## Continuation (2026-07-05): два дефекта наложения текста из реального рендера

Реальный рендер акта (не тестовая фикстура) выявил два настоящих «наложения текста», о которых сообщал пользователь. Исправлено в `crates/trackly-app/src/pdf/renderer.rs`.

### BUG 1 — название организации в шапке вылезало за правое поле страницы

`render_header_two_column` рисовал `header.org_name` одной строкой без переноса — при реальном (длинном) названии организации текст выходил за правую границу страницы. Исправление:
- `org_name` и `org_address` теперь переносятся по словам через `wrap_text_to_width` в пределах правой колонки шапки (ширина = `(A4_WIDTH_PT - MARGIN_PT) - text_col_x`, с запасом `* 0.97`, т.к. перенос жирного `org_name` считается по метрикам обычного начертания — `&Face` протянут в функцию как новый параметр `face`).
- `cursor_y` корректно накапливается по каждой строке переноса, так что заголовок ниже шапки никогда не накладывается на неё.

### BUG 2 — длинная метка FieldRow наезжала на значение

В арме отрисовки `Section::FieldRow` метка (например, «Настоящим актом утверждаю, что мною:») рисовалась одной нежирной... то есть жирной строкой без переноса и без учёта ширины колонки — при длинных метках текст залезал в колонку значения. Исправление:
- `field_row_columns()` теперь возвращает четвёрку `(label_x, label_width, value_x, value_width)` (было `(label_x, value_x, value_width)`).
- Метка переносится по словам в пределах `label_width` и рисуется построчно (жирным, при `x = MARGIN_PT`).
- Итоговая высота строки — `max(кол-во строк метки, кол-во строк значения) * (BODY_SIZE_PT + 4.0)`; подчёркивание по-прежнему рисуется под последней строкой значения.
- `measure_field_row_height` обновлена: меряет перенос ОБЕИХ частей (метку по `label_width`, значение по `value_width`) и возвращает высоту для `max(label_lines, value_lines)` — измерение и отрисовка используют идентичные входные данные переноса, поэтому пагинация не может разойтись с реальной отрисовкой.

### BUG 3 (попутно, по плану) — убрана дублирующая строка даты в шапке

Отдельная строка `header.date_label` в конце шапки (после ИНН/КПП) не встречается в эталонном Word-документе — дата уже присутствует в теле акта («№ {number} от {date}»). Проверено, что ни один тест не проверял наличие этого текста в извлечённом PDF-тексте — удаление безопасно. Поле `HeaderBlock::date_label` оставлено в структуре (используется другими вызывающими сторонами), но рендерер его больше не рисует.

### Тесты

Добавлены 2 новых unit-теста в `pdf::renderer::tests`:
- `header_long_org_name_wraps_and_grows_header_height` — рендерит шапку напрямую (`render_header_two_column`) с коротким и длинным `org_name`, проверяет, что длинное название даёт больший `cursor_y` (перенос реально произошёл), плюс полный рендер `DocSpec` с длинным названием не паникует и даёт валидный PDF.
- `field_row_long_label_wraps_and_measure_reflects_it` — проверяет, что `measure_field_row_height` для строки с длинной меткой («Настоящим актом утверждаю, что мною:») больше высоты одной строки, что перенос метки даёт 2+ строки, и что полный рендер не паникует и содержит и метку, и значение.

Все существующие тесты (renderer unit-тесты — 22/22, `pdf_render_act.rs` — 12/12, `pdf_column_overflow.rs`, `pdf_logo.rs`, `pdf_logo_aspect.rs`, `pdf_text_extract.rs`) остались зелёными без изменений логики.

### act_42.sha256 — регенерирован (легитимный дрейф)

Фикстура `act_42.json` использует `header.org_name`/`org_address`/`date_label` — все три поля затронуты этим исправлением (перенос + удаление date_label), поэтому геометрия PDF действительно изменилась. `fixture_act_42_renders_to_known_hash` упал с ожидаемым дрейфом хэша; `.sha256`-файл обновлён на новое значение (`c21fa40113a52cdb8c27104a8acbfcfafc86742d67f7cd7af564d7fea8951552`), `rendering_twice_yields_identical_bytes` остался зелёным (детерминированность не нарушена).

### Полная проверка

`cargo test -p trackly-app` (весь набор, ~40 мин на первом прогоне без кэша — не связано с этим изменением), `cargo clippy -p trackly-app -- -D warnings`, `cargo fmt --check` — все зелёные.

### Коммит

- `c6202dc` (fix) — `fix(pdf): wrap header org_name/address and FieldRow labels, drop dupe date`
