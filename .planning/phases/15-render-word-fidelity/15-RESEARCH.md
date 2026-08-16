# Phase 15: Рендер и соответствие образцу - Research

**Researched:** 2026-07-04
**Domain:** krilla 0.7 (Rust PDF generation) + MiniJinja DocSpec IR — layout/typography rework for Word-fidelity print output
**Confidence:** HIGH (весь код прочитан напрямую; единственный внешний факт — API `ttf-parser` — подтверждён через WebSearch + локальный `Cargo.lock`)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Мультиустройство (PDFA-02)
- **D-06:** Гибридная вёрстка блока устройств. Короткие идентификационные поля
  (№, Наименование, Инв.№, Серийный №, Модель) — в компактной табличной форме;
  длинные поля (**Комплектация**, **Технические характеристики**, **Состояние**) —
  в отдельном переносящемся блоке на каждое устройство, чтобы избежать overflow
  8-колоночной таблицы на A4 portrait.
  - Рекомендуемая форма (уточнит research/planner): на каждую позицию акта —
    per-device секция: компактная строка идентификации + длинные поля как
    wrapping key-value строки. Должно читаться и при 1, и при 5+ устройствах.
  - Существующий `ItemsTable` — базовый примитив для табличной части; проверить
    перенос длинных ячеек в `renderer.rs` (есть тест `pdf_column_overflow.rs`).
  - Точное визуальное расположение (table-then-blocks vs per-device-card) —
    Claude's Discretion в рамках: без обрезки/наложения, читаемо при 1 vs N.

#### Подписи (PDFA-05)
- **D-07:** Расширить DocSpec-примитив `Signature` (docspec.rs) под двухстрочные
  подписи «Подпись»/«ФИО» для сторон «Выдал»/«Получил» + отрисовка в `renderer.rs`.
  - Новые под-поля пометить `#[serde(default)]`, чтобы старые шаблоны (без под-лейблов)
    десериализовались без ошибки (тот же backward-compat паттерн, что в 14 для HeaderBlock).
  - НЕ собирать подписи вручную из Paragraph+линий в JSON-шаблоне (хрупкое выравнивание).

#### Шапка и логотип (PDFA-01)
- **D-08:** (а) Фикс WR-03 — переключить источник логотипа акта на `org_settings`
  BLOB (`logo_blob`/`logo_mime` из `OrgDbService::get_for_pdf()`), унифицируя с D-05
  (текстовые реквизиты уже берутся оттуда). Сейчас act-рендер читает лого из legacy
  `org.json`, а Settings UI пишет только в BLOB → лого не попадает в акт.
  (б) Вёрстка 2-колоночной шапки как в образце: логотип слева | реквизиты справа
  (название, адрес, телефон, факс, e-mail, ОКПО/ОГРН, ИНН/КПП). Отсутствующие/пустые
  реквизиты деградируют в пусто/«—», не в ошибку рендера.

#### Вводная формулировка и лейблы (PDFA-01)
- **D-09:** Полное соответствие образцу:
  - Заголовок «Акт приема-передачи», реальные № и дата (D-04, без прочерков).
  - Вводная фраза-абзац: «Настоящим актом утверждаю, что мною <ФИО получателя>
    было получено устройство:» (ФИО = `act.receiver_name`).
  - Лейблы подписей «Выдал»/«Получил» (сейчас «Сдал»/«Принял»).
  - Блок «Сроком до: <дата человекочитаемо>».
  - Порядок блоков: шапка → заголовок → №/дата → вводная → блок(и) устройства →
    «Сроком до» → подписи.

#### Ключевые ограничения (из брифа, обязательны)
- Дефолтный шаблон остаётся **редактируемым** через `document_templates` (не хардкод
  в Rust сверх дефолт-сида в `templates/*.minijinja` + `template_service` seed).
- Кириллица рендерится корректно через embedded-шрифт (PDFA-07, регрессия существующего
  пайплайна `fonts.rs`).
- Существующие PDF-тесты проходят; добавить тесты на новый шаблон и мультиустройство
  (1 vs N позиций) — PDFA-08.
- НЕ вводить новый `kind` в `document_templates` (D-03 из фазы 14) — перерабатываем
  существующий `act_handover`.

### Claude's Discretion
- Точное визуальное расположение гибридного блока устройств (в рамках D-06).
- Внутренняя структура расширения примитива `Signature` (поля/имена) при соблюдении
  serde(default)-совместимости.
- Размеры/отступы/шрифт-веса вёрстки, пока результат визуально соответствует образцу.

### Deferred Ideas (OUT OF SCOPE)
- **WR-04** (`migrate_from_org_json` — два зависимых UPDATE в autocommit без транзакции) —
  вне рендер-скоупа; отдельный cleanup-таск (надёжность миграции org.json → org_settings).
- **WR-05** (report `search` LIKE-фильтр без `ESCAPE` — некорректная обработка `%`/`_`) —
  вне скоупа Phase 15 (отчёты, не акт-рендер); отдельный багфикс.
- UI-редактор шаблонов, импорт/экспорт .docx, прочие виды документов — вне милстоуна
  (из «Вне скоупа» брифа).

None beyond the above — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-------------------|
| PDFA-01 | Сгенерированный PDF акта приёма-передачи воспроизводит структуру бумажного образца Word — шапка (логотип + реквизиты), заголовок «Акт приема-передачи», номер/дата, вводная формулировка, блок устройства(-в), «Сроком до», блок подписей «Выдал/Получил». | Architecture Pattern 1 (2-колоночная шапка), Pitfall 5 (приоритет BLOB-лого), D-09 порядок блоков — см. переписанный `act_handover.minijinja` план в Summary/Architecture Patterns |
| PDFA-02 | Акт с несколькими устройствами печатает все позиции корректно — блок «Устройство» переделан под N позиций (через примитив `ItemsTable`), с корректным переносом длинных ячеек (Комплектация, Технические характеристики). | Pattern 2 (word-wrap через `ttf-parser`), Pitfall 4 (почему `ItemsTable` нельзя параметризовать "в лоб"), Open Question 1 (per-device card рекомендация) |
| PDFA-03 | (Phase 14, Complete) Шапка PDF включает расширенные реквизиты организации. | Уже реализовано в Phase 14 — Phase 15 закрывает WR-01 (реквизиты не отрисовывались) через Pattern 1 |
| PDFA-04 | (Phase 14, Complete) Поля «Комплектация», «Технические характеристики» и «Срок до» доступны при формировании акта. | Уже реализовано в Phase 14 (данные в контексте) — Phase 15 закрывает WR-02 (specs не отрисовывались) через Pattern 2 + переписанный шаблон |
| PDFA-05 | Блок подписей печатает двухстрочные подписи «Подпись / ФИО» для сторон «Выдал» и «Получил». | Pattern 3 (расширение `Section::Signature`) |
| PDFA-06 | (Phase 14, Complete) Дефолтный шаблон акта обновлён, редактируем через `document_templates`. | Уже реализовано инфраструктурно в Phase 14; Phase 15 обязана сохранять этот контракт при переписывании `act_handover.minijinja` (см. Don't Hand-Roll / constraints) |
| PDFA-07 | Кириллица во всех полях нового шаблона рендерится корректно через embedded-шрифт. | `fonts.rs` (DejaVu Sans, уже Cyrillic-safe) переиспользуется без изменений; Pattern 2 использует те же embedded байты через `ttf-parser::Face::from_slice` |
| PDFA-08 | Существующие тесты PDF-пайплайна проходят; добавлены тесты на новый шаблон и мультиустройство (1 vs N позиций). | Validation Architecture section — полная карта требований→тестов, Wave 0 gaps, Pitfall 1 (детерминизм-фикстура требует регенерации хэша) |
</phase_requirements>


## Summary

Фаза 15 не начинает с чистого листа: пайплайн MiniJinja→DocSpec→krilla уже существует
и большая часть данных (реквизиты, specs, deadline, N позиций) уже течёт в контекст
рендера (Phase 14). Проблема, зафиксированная в `14-REVIEW.md` (WR-01/WR-02/WR-03), —
**данные посчитаны, но отброшены на двух границах**: (1) дефолтный `.minijinja` не
эмитит новые поля в JSON, и (2) `renderer.rs::render_docspec` их не рисует, даже если
бы они пришли. Отдельно — WR-03: акт-рендер читает лого из legacy `org.json`, хотя
`OrgDbService::get_for_pdf()` уже отдаёт `logo_bytes`/`logo_mime` из BLOB (просто
отброшены как `_logo_bytes`/`_logo_mime` в `act_service.rs:1358`).

Ключевой архитектурный факт, который меняет объём работы: **текущий рендерер шапки
(строки 126-183 `renderer.rs`) — не общий примитив, а захардкоженная последовательность
`draw_text` вызовов** внутри `render_docspec` (org_name → org_address → ИНН/КПП →
date_label → лого). 2-колоночная шапка (D-08б) и расширенные реквизиты (телефон/факс/
email/ОКПО/ОГРН) не встают в эту последовательность без переписывания этого блока —
это не point-fix, а рефакторинг header-рендера в отдельную функцию с колоночной вёрсткой.

Второй архитектурный факт: **`ItemsTable` сейчас truncate-ит длинные ячейки эллипсисом,
а не переносит их** (`truncate_to_width`, доказано тестом `pdf_column_overflow.rs`).
Для «Комплектация»/«Технические характеристики» это неприемлемо (данные обрежутся).
D-06 явно требует гибридную вёрстку: компактная таблица + отдельный переносящийся
блок на каждую позицию — что технически означает **не использовать `ItemsTable` для
длинных полей вообще**, а рисовать их как последовательность `Paragraph`-подобных
блоков с реальным word-wrap (текущий `truncate_to_width` не поддерживает перенос
на несколько строк — только single-line truncation).

Третий факт: текущее измерение ширины текста — приближение `0.5 * font_size` на
глиф (`avg_glyph_w`), не реальные метрики шрифта. Для word-wrap на нескольких строках
эта грубая аппроксимация накопит ошибку сильнее, чем при truncation одной строки.
`ttf-parser` (уже в `Cargo.lock` транзитивно через krilla→rustybuzz/skrifa) даёт
точный `Face::glyph_hor_advance` — целесообразно добавить его как прямую зависимость
для качественного word-wrap вместо приближения.

**Primary recommendation:** Не переиспользовать `ItemsTable`+`Signature` "как есть" —
расширить `renderer.rs` тремя точечными изменениями: (1) новая функция
`render_header_two_column` (заменяет захардкоженный блок, рисует лого слева +
многострочные реквизиты справа, деградирует пустые поля в пропуск строки), (2) новый
примитив word-wrap для длинных key-value полей (используется гибридным блоком
устройства, НЕ трогает существующий `ItemsTable`/`truncate_to_width` — тот остаётся
для компактных таблиц отчётов, чтобы не сломать `pdf_column_overflow.rs` и
`fixture_act_42`), (3) расширение `Section::Signature` доп.полями под-лейблов
(`#[serde(default)]`) + рендер двух строк на сторону. Все три изменения — additive
на уровне serde (старые JSON/шаблоны продолжают десериализоваться), но **меняют
байты вывода рендерера**, что почти наверняка сломает `pdf_determinism.rs`
pinned-hash фикстуру `act_42.json`/`act_42.sha256` — регенерация хэша ожидаема и
безопасна (сам fixture JSON не пришлось бы менять, если header-layout не трогать
для не-акт документов, но раз рендер меняется глобально в `render_docspec`, хэш
неизбежно дрейфует).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Сборка контекста рендера (org/act/items) | API/Backend (`act_service.rs`) | — | Уже сделано в Phase 14; Phase 15 не меняет форму контекста, только шаблон/рендер, потребляющие его |
| Логотип: выбор источника (BLOB vs org.json) | API/Backend (`act_service.rs::render_pdf`) | — | Чистая бэкенд-логика выбора приоритета bytes vs path перед передачей в `HeaderBlock` |
| Разметка DocSpec (JSON-дерево секций) | API/Backend — MiniJinja шаблон (`act_handover.minijinja`) | — | Шаблон — единственное место, где решается «что» попадает в PDF; остаётся редактируемым через `document_templates` |
| Типизация/валидация структуры DocSpec | API/Backend (`docspec.rs`) | — | serde-схема — граница между untrusted JSON от шаблона и typed IR, потребляемым рендерером |
| Отрисовка (позиционирование текста/лого/переносы) | API/Backend (`renderer.rs`, krilla) | — | Krilla работает только на бэкенде (Rust); нет браузерного/frontend аналога — PDF генерируется desktop/server процессом |
| Измерение ширины текста для word-wrap | API/Backend (новый helper на базе `ttf-parser`) | — | Чистая геометрия шрифта, не зависит от DB/сети |
| Redirect: шаблон-редактор в UI (не в скоупе) | Browser/Client (существующий `TemplateEditor`) | — | Вне скоупа фазы — используется как есть, `validate_preview` demo_ctx может потребовать обновления полей |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `krilla` | `=0.7.0` (уже запинено, exact-pin в `Cargo.toml`) | Низкоуровневая отрисовка PDF (текст, изображения, страницы) | Уже используется во всём пайплайне; фаза НЕ поднимает версию — exact-pin `=0.7.0` защищает determinism-фикстуру. [VERIFIED: crates/trackly-app/Cargo.toml:51] |
| `minijinja` | текущая (см. Cargo.lock) | Шаблонизация DocSpec JSON | Уже используется, safe-mode настроен (`build_safe_env`) — фаза не меняет конфигурацию окружения |
| `ttf-parser` | `0.25.1` (уже в `Cargo.lock` транзитивно через `krilla → rustybuzz/skrifa`) | Точное измерение ширины глифов (`Face::glyph_hor_advance`) для корректного word-wrap длинных полей | Не новая внешняя зависимость по факту — уже прошла аудит как часть графа krilla; повышение до прямой зависимости не увеличивает attack surface. [VERIFIED: Cargo.lock содержит `ttf-parser 0.25.1`, `source = "registry+...crates.io-index"`] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `pdf_extract` | текущая (dev-dependency, уже используется в тестах) | Извлечение текста из сгенерированного PDF для assert в тестах | Все новые/расширенные PDF-тесты (multi-device, requisites, signature labels) |
| `sha2` | текущая (dev-dependency) | SHA256 фикстуры `pdf_determinism.rs` | Регенерация `act_42.sha256` после изменения рендерера (ожидаемо в этой фазе) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `ttf-parser` прямая зависимость для измерения | Оставить приближение `0.5 * font_size` | Дешевле по коду, но накопленная ошибка word-wrap на несколько строк хуже, чем на одной строке — при многострочном переносе «Комплектация»/«Тех.характеристики» неточность может вызвать visual overlap, чего D-06 явно требует избежать |
| `ttf-parser` | `rustybuzz` (полный text-shaping) | rustybuzz даёт кернинг/лигатуры, но избыточен для простого left-to-right ASCII/Cyrillic body-текста без сложного шейпинга; ttf-parser достаточен для advance-width word-wrap |
| Новый примитив wrap-block в DocSpec | Пытаться заставить `ItemsTable` переносить текст | `ItemsTable` жёстко привязана к фиксированной колоночной сетке (`col_width = usable_width / col_count`) — перенос текста в одной колонке ломает вертикальное выравнивание остальных колонок той же строки. Нужен отдельный примитив/режим |

**Installation:**
```bash
# Cargo.toml (trackly-app) — promote existing transitive dep to direct
cargo add ttf-parser@0.25.1 -p trackly-app
```

**Version verification:**
```
$ cargo info ttf-parser
version: 0.25.1
license: MIT OR Apache-2.0
repository: https://github.com/harfbuzz/ttf-parser
```
Подтверждено: `Cargo.lock` уже содержит `ttf-parser 0.25.1` из `registry+https://github.com/rust-lang/crates.io-index` (транзитивно через `krilla → rustybuzz/skrifa`). Промоушен в прямую зависимость не меняет resolved-версию (cargo уже разрешил граф).

## Package Legitimacy Audit

> `slopcheck` недоступен в этом окружении (`pip`/`pip3` отсутствуют на машине —
> команда `pip install slopcheck --break-system-packages` завершилась с
> `command not found: pip`). Per protocol — деградация до `[ASSUMED]` со
> смягчающим фактом: пакет уже прошёл транзитивную резолюцию как зависимость
> `krilla` (используется в проекте с момента Phase 3), т.е. это НЕ новая
> неизвестная зависимость, а промоушен существующей транзитивной в прямую.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `ttf-parser` | crates.io | давно (используется harfbuzz-проектами; версия линии 0.25 актуальна) | высокие (транзитивная зависимость `krilla`, `rustybuzz`, `skrifa`) | `github.com/harfbuzz/ttf-parser` | недоступен — `[ASSUMED]` | Approved с оговоркой — планировщик должен вставить `checkpoint:human-verify` перед `cargo add`, несмотря на низкий риск (уже в дереве зависимостей проекта) |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none — `ttf-parser` не новый пакет для дерева зависимостей, только для `Cargo.toml` `trackly-app`; планировщику: gate `cargo add ttf-parser` за `checkpoint:human-verify`, incl. запуск `cargo tree --invert ttf-parser` для подтверждения совпадения версии с уже resolved.

## Architecture Patterns

### System Architecture Diagram

```
MiniJinja-шаблон (document_templates.body_minijinja, kind='act_handover')
        │  (template variables: org.*, act.*, act.items[].*)
        ▼
render_with_timeout()  [safe-mode: Strict undefined, fuel=100k, 5s timeout]
        │  emits JSON string
        ▼
serde_json::from_str::<DocSpec>()  [docspec.rs — typed IR validation]
        │  DocSpec { title, header: HeaderBlock, sections: Vec<Section> }
        ▼
PdfRenderer::render_docspec()  [renderer.rs]
        │
        ├─▶ render_header_two_column()  [NEW — заменяет текущий inline-блок]
        │      логотип (bytes|path) слева │ реквизиты (name/inn/kpp/address/
        │      phone/fax/email/okpo/ogrn) многострочно справа
        │
        ├─▶ render_section() per Section in spec.sections
        │      ├─ Heading / Paragraph / KeyValueTable  [unchanged]
        │      ├─ ItemsTable  [unchanged — компактная идентификация:
        │      │    №, Наименование, Инв.№, Серийный №, Модель]
        │      ├─ NEW: wrapping key-value block per device (Комплектация,
        │      │    Технические характеристики, Состояние) — реальный
        │      │    word-wrap через ttf-parser advance-width
        │      └─ Signature (EXTENDED — двухстрочная «Подпись»/«ФИО»)
        │
        ▼
normalize_pdf_for_determinism()  [regex post-process — unchanged]
        ▼
Vec<u8> PDF bytes
```

Данные (`act.items[].specs/kit/condition`, `org.phone/fax/email/okpo/ogrn`, `org.logo_bytes`)
уже долетают до входа в этот граф (Phase 14). Разрыв — на выходе из MiniJinja-шаблона
(не эмитятся в JSON) и на входе в `render_docspec` (не читаются в draw-вызовы).

### Recommended Project Structure

Без новых файлов — все изменения внутри существующих модулей:
```
crates/trackly-app/src/pdf/
├── docspec.rs      # + новые под-поля Signature (serde(default))
├── renderer.rs     # + render_header_two_column(), + wrap-примитив,
│                   #   + wrap_text_to_width() на базе ttf-parser
├── fonts.rs         # unchanged — уже даёт Cyrillic-safe DejaVu Sans
└── mod.rs           # unchanged exports

crates/trackly-app/templates/
└── act_handover.minijinja   # переписывается под образец (D-09 block order)

crates/trackly-app/tests/
├── pdf_render_act.rs         # extend: multi-device (1 vs N), новые лейблы
├── pdf_column_overflow.rs    # extend: wrap-block НЕ truncate-ит длинные поля
├── pdf_logo.rs               # extend: BLOB-приоритет из get_for_pdf в render_pdf
└── pdf_determinism.rs        # act_42.json/act_42.sha256 — РЕГЕНЕРИРОВАТЬ хэш
```

### Pattern 1: Header two-column layout (D-08б)

**What:** Заменить последовательные `draw_text` вызовы (org_name → org_address →
ИНН/КПП → date_label → лого в углу) на функцию, которая делит верхнюю область
страницы на левую (лого) и правую (текстовый блок реквизитов) колонки, с
условным пропуском пустых строк.

**When to use:** Единственная точка входа в `render_docspec` — заменяет текущий
блок `renderer.rs:126-183`.

**Example (концептуальный, на основе существующего API вызовов):**
```rust
// Source: паттерн уже используется в draw_logo_from_bytes/draw_logo_top_right
// (renderer.rs:393-461, :468-565) — push_transform/draw_image/pop.
// Для двух колонок: лого рисуется в левой X-позиции (не top-right угол,
// как сейчас), текстовый блок реквизитов — в правой колонке с построчным
// draw_text и пропуском пустых строк.
fn render_header_two_column(surface: &mut Surface, header: &HeaderBlock, ...) -> f32 {
    let logo_col_width = 120.0;
    let text_col_x = MARGIN_PT + logo_col_width + 12.0;
    let mut y = MARGIN_PT;

    // логотип: приоритет logo_bytes > logo_path > skip (unchanged priority,
    // просто смещена позиция отрисовки в левую колонку)
    if let Some(bytes) = &header.logo_bytes {
        draw_logo_at(surface, bytes, header.logo_mime.as_deref(), MARGIN_PT, y);
    } else if let Some(path) = &header.logo_path {
        draw_logo_at_path(surface, path, MARGIN_PT, y);
    }

    // реквизиты — построчно, пропуская пустые (D-08б: "—" или пропуск строки)
    let lines: Vec<&str> = [
        header.org_name.as_str(),
        header.org_address.as_str(),
        // условные строки — рисуются только если не пусто
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect();
    for line in lines {
        surface.draw_text(Point::from_xy(text_col_x, y), font, BODY_SIZE_PT, line, ...);
        y += BODY_SIZE_PT + 4.0;
    }
    y
}
```

### Pattern 2: Word-wrap для длинных key-value полей (D-06)

**What:** Новая функция, использующая реальные метрики шрифта (`ttf-parser`)
для разбиения длинного текста на несколько строк по границам слов (не эллипсис).

**When to use:** Только для гибридного блока устройства (Комплектация/Тех.характеристики/
Состояние) — НЕ заменяет `truncate_to_width`, который остаётся для компактной таблицы
и report-экспорта (не трогать, чтобы не сломать `pdf_column_overflow.rs` инвариант
B-3 и `fixture_act_42`).

```rust
// Source: собственная реализация на базе ttf-parser::Face::glyph_hor_advance —
// API подтверждён WebSearch (docs.rs/ttf-parser), не найдено официального
// Context7-источника для этой конкретной версии; помечено как [CITED: docs.rs].
use ttf_parser::Face;

/// Разбивает text на строки по словам так, чтобы каждая строка помещалась
/// в max_width (PDF points) при данном font_size, используя реальные
/// advance-width метрики шрифта вместо приближения 0.5*font_size.
pub fn wrap_text_to_width(
    face: &Face,
    text: &str,
    font_size: f32,
    max_width: f32,
) -> Vec<String> {
    let units_per_em = face.units_per_em() as f32;
    let scale = font_size / units_per_em;
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0.0_f32;

    for word in text.split_whitespace() {
        let word_width: f32 = word
            .chars()
            .filter_map(|c| face.glyph_index(c))
            .map(|gid| face.glyph_hor_advance(gid).unwrap_or(0) as f32 * scale)
            .sum();
        let space_width = if current.is_empty() { 0.0 } else { font_size * 0.25 };

        if current_width + space_width + word_width > max_width && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            current_width = 0.0;
        }
        if !current.is_empty() {
            current.push(' ');
            current_width += space_width;
        }
        current.push_str(word);
        current_width += word_width;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
```

**Note:** `Face::from_slice(font_bytes, 0)` строится из тех же embedded байт
(`DEJAVU_SANS_REGULAR`/`DEJAVU_SANS_BOLD`), что уже хранит `PdfRenderer` —
не нужен отдельный источник шрифта, только парсинг тех же байт через `ttf-parser`
параллельно с `krilla::text::Font`.

### Pattern 3: Extended Signature primitive (D-07)

**What:** Добавить опциональные под-лейблы к `Section::Signature` с
`#[serde(default)]`, рендерить второй ряд подписи под линией.

```rust
// Source: расширение существующей структуры в docspec.rs — паттерн
// идентичен HeaderBlock's org_phone/org_fax (Phase 14, уже проверенный
// backward-compat подход).
Signature {
    left_label: String,
    right_label: String,
    #[serde(default = "default_spacer_pt")]
    spacer_pt: f32,
    /// Двухстрочные под-лейблы «Подпись»/«ФИО» (D-07, Phase 15).
    /// Пусто по умолчанию — старые JSON без этих полей рендерят
    /// однострочные подписи как раньше (backward-compat).
    #[serde(default)]
    left_sublabel: Option<String>,
    #[serde(default)]
    right_sublabel: Option<String>,
}
```
В `render_section` (`renderer.rs`) для `Section::Signature` — после текущей
однострочной пары `left_label`/`right_label`, если `left_sublabel`/`right_sublabel`
заданы, рисовать вторую строку под каждой (напр. «Подпись» слева-сверху под линией
и «ФИО» справа рядом, либо оба лейбла друг под другом — итоговая раскладка на
усмотрение планировщика/исполнителя в рамках Claude's Discretion).

### Anti-Patterns to Avoid
- **Ручная сборка подписей из `Paragraph`+линий в JSON-шаблоне:** явно запрещено
  D-07 — хрупкое выравнивание, нет типовой гарантии структуры. Расширять примитив,
  не обходить его.
- **Переиспользование `ItemsTable` для длинных полей "как есть":** приведёт либо
  к truncation (текущее поведение `truncate_to_width`), либо к overlap, если
  просто дать больше места одной колонке — фиксированная колоночная сетка не
  поддерживает многострочные ячейки без полного рефакторинга layout-модели.
- **Изменение сигнатуры `HeaderBlock`/`Section` без `#[serde(default)]`:** сломает
  ранее сохранённые кастомные `document_templates.body_minijinja`, если пользователь
  уже отредактировал шаблон — Phase 14 уже установила этот паттерн, продолжать его.
- **Полагаться на приближение `0.5 * font_size` для многострочного wrap:** приемлемо
  для truncation одной строки (текущий инвариант `pdf_column_overflow.rs`), но
  накопленная ошибка на несколько строк текста рискует overlap — именно то, что
  D-06 требует избежать.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Точное измерение ширины текста для переноса | Собственную эвристику символ-в-пиксели | `ttf-parser::Face::glyph_hor_advance` на встроенных DejaVu Sans байтах | Точные font-metrics уже разрешены в дереве зависимостей (транзитивно через krilla) — переизобретать приближение хуже, чем взять готовый парсер шрифта |
| Многостраничная пагинация при большом N устройств | Собственный page-break алгоритм с нуля | Не требуется в этой фазе (не входит в success criteria) — но НЕ игнорировать: если N велико (5+), контент может выйти за пределы одной A4-страницы. Задокументировать как открытый вопрос/лимитацию, не реализовывать полную пагинацию без явного requirement | Пагинация krilla требует `doc.start_page_with()` вызывать несколько раз с отслеживанием y-cursor через границу страницы — существенный объём работы, не покрытый явно ни одним PDFA-* требованием этой фазы |
| Растровый рендеринг логотипа с произвольным aspect ratio | Свой image-scaling код | Уже есть `scale_logo_dimensions` (contain-fit, aspect-preserving) — переиспользовать при переносе логотипа в левую колонку шапки | Код уже протестирован (`pdf_logo.rs`), просто меняется точка вызова (позиция), не сама логика масштабирования |

**Key insight:** Инфраструктура для геометрии (масштабирование лого, PDF-примитивы)
уже существует и протестирована — единственный реальный пробел компетенции — точное
измерение текста для переноса на несколько строк, для чего в дереве зависимостей уже
есть готовый инструмент (`ttf-parser`), не нужно ничего "хэндроллить".

## Common Pitfalls

### Pitfall 1: Determinism fixture (`pdf_determinism.rs`) сломается — это ожидаемо, не баг
**What goes wrong:** `fixture_act_42_renders_to_known_hash` пинует SHA256 конкретного
байтового вывода `render_docspec` для фиксированного `act_42.json`. Любое изменение
внутри `render_docspec`/`render_header_two_column` (даже не трогающее сам JSON-фикстур)
меняет байты PDF → тест падает.
**Why it happens:** Тест написан именно для того, чтобы ловить *непреднамеренный*
дрейф — преднамеренное изменение рендерера обязано сопровождаться регенерацией хэша.
**How to avoid:** Запланировать явный шаг «regenerate `act_42.sha256`» как часть
задачи, а не как gap-closure после красного CI. Не пытаться избежать изменения хэша —
это архитектурно неизбежно при рефакторинге header/wrap-логики.
**Warning signs:** Красный `fixture_act_42_renders_to_known_hash` в CI без объяснения
в PR/коммите — планировщик должен явно включить регенерацию хэша в план.

### Pitfall 2: `render_docspec` меняется — репорты (report_service.rs) используют тот же рендерер
**What goes wrong:** `HeaderBlock`/`render_docspec` — общий код между актами и
report-экспортом (`report_service.rs::export_pdf`). Изменение вёрстки шапки
(2-колоночная) визуально изменит и PDF-отчёты, не только акты.
**Why it happens:** `HeaderBlock` спроектирован как общий примитив для «любого
документа с шапкой» — фаза 15 меняет его семантику для одного случая использования,
но реализация в `renderer.rs` едина.
**How to avoid:** Либо (а) принять, что report-шапка тоже становится 2-колоночной
(вероятно допустимо — реквизиты и там уместны), либо (б) убедиться, что нет
report-specific теста, ожидающего старую однострочную раскладку. Поиск подтвердил:
**нет pinned-hash теста для report PDF** — визуальный риск ниже, но стоит
проверить `report_service.rs`-related тесты на предмет позиционных assert'ов
после реализации.
**Warning signs:** Существующие report-тесты, которые парсят текст через
`pdf_extract` и полагаются на порядок строк в шапке (не встречено при чтении,
но стоит перепроверить на этапе планирования конкретных тестов).

### Pitfall 3: `TemplateService::validate_preview`'s `demo_ctx` — рассинхрон со схемой шаблона
**What goes wrong:** `validate_preview` строит собственный демо-контекст (не
переиспользует продовый `render_pdf` context-builder). Если новый шаблон
ссылается на переменные (напр. `item.specs`, `item.kit`, `item.condition`),
которых нет в `demo_ctx["act"]["items"][0]`, `UndefinedBehavior::Strict`
уронит рендер предпросмотра с ошибкой "undefined value" — именно баг,
уже дважды случавшийся в истории проекта (GAP-S6, G2-4, см. комментарии
в файле).
**Why it happens:** Два независимых места строят контекст (`act_service.rs::render_pdf`
и `template_service.rs::validate_preview`) — нет единого источника схемы контекста.
**How to avoid:** При переписывании `act_handover.minijinja` под новую схему —
обязательно обновить `demo_ctx` в `validate_preview` синхронно (добавить
`specs`/`kit`/`condition` в demo item, плюс org.phone/fax/email/okpo/ogrn).
Существующий тест `validate_preview_returns_pdf_bytes` должен продолжать
проходить — но текущая demo_ctx НЕ содержит `specs`/`kit`/`condition` в
`act.items[0]` (только name/inventory_no/serial_no/model/quantity) —
**потребует правки** при переписывании шаблона.
**Warning signs:** `validate_preview_returns_pdf_bytes` падает с "undefined
value: item.specs" (или аналогично) после переписывания шаблона — сигнал,
что demo_ctx не обновлён параллельно.

### Pitfall 4: `ItemsTable` для мультиустройства — колонки нельзя просто добавить
**What goes wrong:** Naïve fix — добавить «Комплектация»/«Тех.характеристики» как
доп. колонки в существующий `ItemsTable` (8 колонок на A4 portrait при текущей
равноширинной сетке `usable_width / col_count`). При usable_width ≈ 495pt и
8 колонках — по ~62pt на колонку — длинный текст truncate-ится агрессивно
(текущее поведение), что напрямую противоречит "без обрезки" из D-06.
**Why it happens:** `ItemsTable` — это простая равноширинная grid, не
рассчитана на колонки разной семантической длины.
**How to avoid:** D-06 уже предписывает гибридную структуру — не пытаться
решить через параметризацию ширины колонок `ItemsTable` (col_width per column
потребовал бы нового поля в структуре и полного переписывания рендера
таблицы). Разделить на компактную идентификацию (короткие поля,
`ItemsTable`, как сейчас, только с меньшим числом колонок: №/Наименование/
Инв.№/Серийный №/Модель) + отдельные wrap-блоки под каждую позицию для
длинных полей.
**Warning signs:** Тест `pdf_column_overflow.rs`-style regression, где длинный
текст в «Комплектация» появляется усечённым эллипсисом вместо переноса.

### Pitfall 5: Логотип из BLOB — приоритет, не замена
**What goes wrong:** WR-03 фикс может по ошибке ПОЛНОСТЬЮ убрать fallback на
`org.json`/`safe_logo_canonical`, сломав старые инсталляции, у которых логотип
всё ещё лежит только в org.json (до миграции на BLOB).
**Why it happens:** D-08а прямо говорит "переключить источник", что можно
неверно прочитать как "убрать legacy путь".
**How to avoid:** `renderer.rs` уже поддерживает приоритет `logo_bytes > logo_path`
(строки 178-183) — правильный фикс: в `act_service.rs::render_pdf` перестать
отбрасывать `_logo_bytes`/`_logo_mime` из `get_for_pdf()` и передать их в
`HeaderBlock.logo_bytes`/`logo_mime`, ОСТАВИВ `safe_logo` (org.json path) как
`logo_path` fallback для случая `logo_bytes = None`. Рендерер уже это делает
правильно — фикс только в `act_service.rs`, где сейчас `let (dto, _logo_bytes,
_logo_mime) = org_db.get_for_pdf().await?;` отбрасывает нужные значения.
**Warning signs:** Акт без BLOB-лого (но с валидным org.json путём) внезапно
рендерится без лого после фикса — признак, что fallback был убран, а не
переставлен приоритет.

### Pitfall 6: MiniJinja safe-mode fuel budget (100_000) при итерации по N устройствам
**What goes wrong:** Гибридная вёрстка требует более сложного шаблона (цикл по
`act.items`, внутри цикла — несколько условных строк на wrap-блок). При N=10+
устройствах с длинными Комплектация/Тех.характеристики полями, объём
JSON-генерации в шаблоне растёт линейно — риск упереться в `set_fuel(100_000)`
при большом N, хотя для типичных актов (1-5 позиций) риска почти нет.
**Why it happens:** Fuel считает инструкции интерпретатора MiniJinja, не строки
вывода — сложные Jinja-выражения (`| default(...) | tojson` цепочки) внутри
цикла умножают fuel cost на N.
**How to avoid:** Не менять fuel-лимит без явного requirement — задокументировать
как открытый вопрос при планировании тестов "1 vs N" (какое N разумно тестировать —
рекомендация: тест на N=5-8 для покрытия успешного случая, не пытаться найти
верхнюю границу fuel в этой фазе).
**Warning signs:** `render_with_timeout` возвращает `AppError::Validation` с
"Render timeout" или fuel-exhaustion ошибкой при большом количестве позиций —
не встречено в текущих тестах (макс. виденное N=1), стоит проверить при
добавлении multi-device теста с N≥5.

## Code Examples

### Текущий приоритет логотипа в рендерере (переиспользуется без изменений)
```rust
// Source: crates/trackly-app/src/pdf/renderer.rs:174-183 (уже в коде)
// Priority (Phase 7 plan 02):
//   1. logo_bytes is Some → draw from in-memory bytes + logo_mime
//   2. logo_path is Some → read from filesystem (Phase 3 path)
//   3. else → no logo
if let Some(logo_bytes) = &spec.header.logo_bytes {
    let mime = spec.header.logo_mime.as_deref().unwrap_or("image/png");
    draw_logo_from_bytes(&mut surface, logo_bytes, mime);
} else if let Some(logo_path_str) = &spec.header.logo_path {
    draw_logo_top_right(&mut surface, logo_path_str);
}
```

### Требуемый фикс в act_service.rs (WR-03 — не отбрасывать logo_bytes)
```rust
// ТЕКУЩЕЕ (act_service.rs:1356-1360) — отбрасывает нужные значения:
let org_dto = match pipeline.org_db {
    Some(org_db) => {
        let (dto, _logo_bytes, _logo_mime) = org_db.get_for_pdf().await?;
        dto
    }
    // ...
};
// ...
"logo_path": safe_logo.map(|p| p.display().to_string()),
// logo_bytes / logo_mime вообще не попадают в HeaderBlock construction ниже

// ТРЕБУЕТСЯ: сохранить logo_bytes/logo_mime и передать в HeaderBlock,
// safe_logo остаётся как logo_path fallback (renderer уже приоритизирует
// logo_bytes, если Some).
```

### Тест-паттерн: assert через pdf_extract (переиспользуется для новых тестов)
```rust
// Source: crates/trackly-app/tests/pdf_render_act.rs:400-408 (существующий паттерн)
let text = pdf_extract::extract_text_from_mem(&bytes).expect("extract");
assert!(
    text.contains("Ромашка"),
    "org_settings org_name missing from rendered PDF. Head: {:?}",
    text.chars().take(500).collect::<String>()
);
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `truncate_to_width` (эллипсис, одна строка, 0.5*font_size приближение) | Оставить как есть для `ItemsTable`/report-экспорта | Не меняется в этой фазе (только для гибридного блока устройства добавляется новый word-wrap путь) | `pdf_column_overflow.rs` инвариант B-3 (byte-identical для коротких строк) не затрагивается |
| Логотип из `org.json` (legacy path) | Логотип из `org_settings` BLOB, org.json как fallback | Phase 14 подготовила данные (`get_for_pdf`), Phase 15 фактически подключает | Акты, созданные после включения BLOB-лого через Settings UI, начнут показывать лого (сейчас — не показывают, WR-03) |
| Однострочная `Signature` (левый/правый лейбл на одной линии) | Двухстрочная (доп. под-лейблы «Подпись»/«ФИО») | D-07, эта фаза | Существующие custom-отредактированные шаблоны (без под-лейблов) продолжат рендериться как раньше — backward-compat через `#[serde(default)]` |

**Deprecated/outdated:**
- Ничего явно не депрекейтится — все изменения additive на уровне serde-схемы.
  `truncate_to_width` не депрекейтится, а сохраняется для не-мультиустройство
  сценариев (report tables, компактная идентификация).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `ttf-parser::Face::glyph_hor_advance` — корректный API для получения horizontal advance глифа в font units (масштабируется через `units_per_em`) | Standard Stack / Pattern 2 | Если API отличается в 0.25.1 (напр. другое имя метода), придётся адаптировать сигнатуру — низкий риск, т.к. `ttf-parser` — стабильный, широко используемый crate (harfbuzz org), но НЕ проверено через Context7/официальную докстраницу конкретной версии в этой сессии |
| A2 | slopcheck-эквивалентная уверенность в `ttf-parser`: пакет безопасен, т.к. уже транзитивно в дереве через `krilla` | Package Legitimacy Audit | Крайне низкий риск — пакет физически уже компилируется и используется в текущем `cargo build` через krilla; промоушен в прямую зависимость не меняет resolved-граф. Единственный сценарий риска — supply-chain атака между текущим Cargo.lock и моментом апдейта, что не специфично для этой фазы |
| A3 | Report-PDF (`report_service.rs::export_pdf`) не имеет pinned-hash/позиционного теста, зависящего от текущей однострочной шапки | Pitfall 2 | Если такой тест всё же существует, но не найден при `grep`, изменение 2-колоночной шапки может неожиданно сломать report-тесты — проверено через целевой grep по `tests/`, ничего не найдено, но полный `cargo test` — единственный надёжный способ подтвердить |

## Open Questions

1. **Точная геометрия гибридного блока устройства (per-device card vs table-then-blocks)**
   - What we know: D-06 явно оставляет это Claude's Discretion в рамках "без
     обрезки/наложения, читаемо при 1 vs N".
   - What's unclear: Компактная таблица сверху + N wrap-блоков снизу, ИЛИ
     повторяющаяся карточка на каждую позицию (заголовок "Устройство N" +
     все поля key-value внутри одной карточки)? Первое ближе к образцу
     (образец имеет одну табличную структуру для всех key-value полей одного
     устройства — единый key-value список, не таблица), второе лучше
     масштабируется на N.
   - Recommendation: Планировщику стоит выбрать **per-device card** (заголовок
     "Устройство №N: <name>" + key-value список из компактных полей + wrap-блоки
     длинных полей внутри одной карточки) — это ближе к семантике образца
     (где на одно устройство весь блок читается как единый key-value список,
     не таблица), и естественно масштабируется на N без изменения ширины
     колонок.

2. **Верхняя граница N устройств для гибридного блока без пагинации**
   - What we know: Success criterion #2 требует "2+ устройств... без обрезки/
     наложения"; фаза явно не требует полной пагинации (нет A4 overflow
     handling ни в одном PDFA-* требовании).
   - What's unclear: Что происходит при N=10+ (контент выходит за пределы
     одной A4-страницы — krilla `render_docspec` строит ровно одну страницу,
     `start_page_with` вызывается один раз).
   - Recommendation: Тестировать 1 vs N=3-5 (типичный кейс), задокументировать
     как известное ограничение (не bug) поведение при очень большом N —
     не реализовывать пагинацию без явного нового требования.

3. **2-колоночная шапка при отсутствующем логотипе — схлопывается ли правая колонка влево?**
   - What we know: D-08б требует деградацию пустых реквизитов в "—"/пусто,
     не в ошибку.
   - What's unclear: Явно не специфицировано поведение layout, если
     `logo_bytes`/`logo_path` оба `None` — должна ли текстовая колонка
     занять всю ширину страницы (как сейчас, single-column) или остаться
     в узкой правой колонке с пустым местом слева?
   - Recommendation: Проще и достаточно — оставить фиксированную 2-колоночную
     сетку независимо от наличия лого (пустое место слева не критично
     визуально), не усложнять адаптивной логикой. Планировщику зафиксировать
     явно в плане, чтобы не тратить время на discovery во время исполнения.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` / Rust toolchain | Сборка/тесты `trackly-app` | ✓ | см. `rust-toolchain`/CI (MSRV ≥1.85 проектный) | — |
| `ttf-parser` (crates.io) | Word-wrap измерение | ✓ (уже в `Cargo.lock` транзитивно) | 0.25.1 | Fallback на текущее приближение `0.5*font_size`, если `cargo add` заблокирован по какой-то причине (маловероятно — MSRV `ttf-parser` 1.63.0 << проектный 1.85) |
| `pip`/`slopcheck` | Аудит легитимности пакетов | ✗ | — | Деградация до `[ASSUMED]` + `checkpoint:human-verify` (см. Package Legitimacy Audit) |
| Исходный образец Word (не хранится в репозитории) | Визуальный эталон | ✓ | — | — |
| Референс-логотип образца (не хранится в репозитории) | Референс лого для UAT-сверки | ✓ (166×88px) | — | — |

**Missing dependencies with no fallback:** нет (все критичные зависимости доступны либо имеют fallback).

**Missing dependencies with fallback:**
- `slopcheck` — деградация до `[ASSUMED]`-маркировки `ttf-parser` + ручная проверка планировщиком.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (стандартный Rust test harness) + `tokio::test` для async сервисных тестов |
| Config file | none — стандартный `cargo test`, интеграционные тесты в `crates/trackly-app/tests/*.rs` |
| Quick run command | `cargo test -p trackly-app --test pdf_render_act -- --test-threads=1` |
| Full suite command | `cargo test -p trackly-app` (напоминание из MEMORY.md: **не запускать `cargo test` конкурентно** — один процесс за раз, иначе конфликт `target/` lock выглядит как многоминутный hang) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PDFA-01 | Шапка (лого+реквизиты 2-кол) → заголовок → №/дата → вводная → устройство(-а) → «Сроком до» → подписи, в правильном порядке | integration (pdf_extract, порядок подстрок) | `cargo test -p trackly-app --test pdf_render_act render_handover_act_produces_cyrillic_pdf` (расширить) | ✅ существующий файл, тест нужно расширить/добавить |
| PDFA-02 | N устройств (2+) печатают все позиции, длинные поля переносятся без обрезки/наложения | integration (pdf_extract на wrap-текст + отсутствие '…' на длинных полях устройства) | `cargo test -p trackly-app --test pdf_render_act render_handover_multi_device_wraps_long_fields` (новый) | ❌ Wave 0 — добавить новый тест |
| PDFA-03 | (Phase 14, Complete) — расширенные реквизиты в контексте | — | — | ✅ уже покрыто в Phase 14 |
| PDFA-04 | (Phase 14, Complete) — Комплектация/Тех.характеристики/Срок до доступны в контексте | — | — | ✅ уже покрыто в Phase 14 |
| PDFA-05 | Двухстрочные подписи «Подпись»/«ФИО» для «Выдал»/«Получил» | integration (pdf_extract на все 4 под-строки: два лейбла сторон + два под-лейбла) | `cargo test -p trackly-app --test pdf_render_act signature_renders_two_line_labels` (новый) | ❌ Wave 0 |
| PDFA-06 | (Phase 14, Complete) — шаблон редактируем через `document_templates` | — | — | ✅ уже покрыто (не переоткрывать) |
| PDFA-07 | Кириллица во всех новых блоках корректна | integration (pdf_extract, проверка кириллических строк во всех новых полях — requisites, wrap-текст, под-лейблы подписи) | Покрывается расширением существующих тестов, отдельный тест не обязателен, но добавить хотя бы одну явную кириллическую проверку в новый multi-device тест | 🟡 частично — расширить существующие assert'ы |
| PDFA-08 | Существующие PDF-тесты проходят + новые тесты на шаблон/мультиустройство | full suite | `cargo test -p trackly-app` (полный прогон, включая `pdf_determinism.rs` после регенерации хэша) | ✅/🟡 — существующие файлы проходят, `act_42.sha256` требует регенерации |

### Sampling Rate
- **Per task commit:** `cargo test -p trackly-app --test pdf_render_act --test pdf_column_overflow --test pdf_logo -- --test-threads=1` (быстрый прогон затронутых файлов; **не запускать параллельно с другим `cargo test`** — см. MEMORY constraint)
- **Per wave merge:** `cargo test -p trackly-app -- --test-threads=1` (полный набор, включая `pdf_determinism.rs`)
- **Phase gate:** Полный `cargo test -p trackly-app` зелёный + `cargo clippy -p trackly-app -- -D warnings` + `cargo fmt --check` до `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `tests/pdf_render_act.rs::render_handover_multi_device_wraps_long_fields` — покрывает PDFA-02 (N=1 vs N=5, длинные Комплектация/Тех.характеристики без truncation-эллипсиса)
- [ ] `tests/pdf_render_act.rs::signature_renders_two_line_labels` — покрывает PDFA-05 (assert четырёх подстрок: «Выдал», «Получил», «Подпись», «ФИО»)
- [ ] `tests/pdf_render_act.rs` — обновить/добавить тест, что requisites (phone/fax/email/okpo/ogrn) реально попадают в extracted text (закрывает WR-01 regression gap, усиливает существующий `render_pdf_with_filled_specs_and_requisites_surfaces_data`, который сейчас проверяет только org_name)
- [ ] `tests/pdf_logo.rs` — новый/расширенный тест: `render_pdf` (не `render_docspec` напрямую) с заполненным BLOB-лого в `org_settings` действительно даёт `/Subtype /Image` в итоговом PDF акта (закрывает WR-03 regression gap — текущие `pdf_logo.rs` тесты бьют `PdfRenderer::render_docspec` напрямую, не через `act_service::render_pdf`, поэтому не ловят баг отброшенного `_logo_bytes`)
- [ ] `tests/fixtures/act_42.sha256` — регенерировать после реализации (ожидаемое, не gap, но обязательный шаг в плане)
- [ ] `crates/trackly-app/src/services/template_service.rs::validate_preview`'s `demo_ctx` — обновить под новую схему шаблона (specs/kit/condition в demo item + org.phone/fax/email/okpo/ogrn), иначе `validate_preview_returns_pdf_bytes` упадёт после переписывания `act_handover.minijinja`

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V2 Authentication | no | Не меняется в этой фазе — рендер PDF не затрагивает auth |
| V3 Session Management | no | — |
| V4 Access Control | no | `document_templates.update_body`/`reset_to_default` уже требуют `Action::ManageSettings` (Phase 14 не меняется здесь) |
| V5 Input Validation | yes | MiniJinja `UndefinedBehavior::Strict` + `set_fuel(100_000)` + без `loader` (уже настроено, `minijinja_env.rs`) — критично не ослаблять при добавлении новых полей/циклов в шаблон |
| V6 Cryptography | no | Не применимо — рендер не работает с криптографией |

### Known Threat Patterns for {krilla + MiniJinja PDF pipeline}

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|----------------------|
| Template injection через редактируемый `document_templates.body_minijinja` (шаблон читает `{% include %}`/filesystem) | Tampering / Information Disclosure | Уже смягчено — `build_safe_env()` не регистрирует `loader`, поэтому `{% include %}`/`{% extends %}` не могут достать файлы с диска. Фаза 15 не должна добавлять `env.set_loader(...)` ни при каких обстоятельствах |
| DoS через сложный шаблон (глубокая рекурсия / огромный цикл по N устройств) | Denial of Service | `set_recursion_limit(64)` + `set_fuel(100_000)` + `tokio::time::timeout(5s)` уже в месте — при увеличении сложности шаблона (доп. условные строки на wrap-блок) проверить, что типичный N (1-8) не приближается к fuel-лимиту (см. Pitfall 6) |
| Внедрение произвольных PDF-операторов через шаблон | Tampering | Уже структурно невозможно — `DocSpec`/`Section` — типизированный enum (serde tag), шаблон не может произвести `raw_pdf_op`; расширение `Signature`/добавление нового wrap-примитива ДОЛЖНО оставаться типизированным enum-вариантом, не строковым "raw" полем |
| Path traversal через `logo_path` (уже существующий вектор, не новый в этой фазе) | Tampering / Information Disclosure | Не расширяется в этой фазе — `logo_path` продолжает читаться как раньше (`std::fs::read`); BLOB-приоритет (WR-03 фикс) снижает частоту обращения к filesystem-пути, что скорее уменьшает поверхность атаки, чем увеличивает |

## Sources

### Primary (HIGH confidence)
- Прямое чтение исходного кода: `crates/trackly-app/src/pdf/docspec.rs`, `renderer.rs`, `fonts.rs`, `mod.rs`, `minijinja_env.rs`
- Прямое чтение: `crates/trackly-app/src/services/act_service.rs` (render_pdf, ~L1342-1457), `template_service.rs`, `org_db_service.rs::get_for_pdf`, `report_service.rs::export_pdf`
- Прямое чтение: `crates/trackly-app/templates/act_handover.minijinja`, `act_acceptance.minijinja`
- Прямое чтение: `crates/trackly-app/tests/pdf_render_act.rs`, `pdf_column_overflow.rs`, `pdf_logo.rs`, `pdf_determinism.rs`, `tests/fixtures/act_42.json`
- `crates/trackly-app/Cargo.toml:51` — `krilla = "=0.7.0"` (exact pin)
- `Cargo.lock` — подтверждение `ttf-parser 0.25.1` уже в графе зависимостей (транзитивно через `krilla → rustybuzz/skrifa`)
- `cargo info ttf-parser` — версия/лицензия/репозиторий подтверждены локально

### Secondary (MEDIUM confidence)
- [ttf-parser docs.rs](https://docs.rs/ttf-parser/latest/ttf_parser/) — `glyph_hor_advance` API существование подтверждено через WebSearch (не через прямой Context7-запрос, т.к. Context7 MCP недоступен в этой сессии; API описание сформулировано по training-знанию + подтверждению существования метода через поиск)
- [github.com/harfbuzz/ttf-parser](https://github.com/harfbuzz/ttf-parser) — репозиторий, лицензия MIT/Apache-2.0, поддерживается организацией harfbuzz

### Tertiary (LOW confidence)
- Точная сигнатура/поведение `Face::glyph_hor_advance` (единицы измерения, обработка variable fonts) — не верифицирована постановкой реального юнит-теста в этой research-сессии; помечено `[ASSUMED]` (A1) — планировщик/исполнитель должен написать unit-тест на wrap-функцию до интеграции в рендерер

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — весь существующий стек прочитан напрямую из кода; единственное новое звено (`ttf-parser`) уже физически в `Cargo.lock`, версия подтверждена локальной `cargo info`
- Architecture: HIGH — весь путь MiniJinja→DocSpec→krilla прочитан построчно, включая все точки, которые нужно менять (header block, ItemsTable, Signature)
- Pitfalls: HIGH — все 6 pitfalls обнаружены прямым чтением существующих тестов/кода (determinism fixture, shared report renderer, validate_preview demo_ctx desync, ItemsTable equal-width grid, logo priority, MiniJinja fuel), не гипотетические

**Research date:** 2026-07-04
**Valid until:** 30 дней (стабильный внутренний код проекта; krilla exact-pinned, не дрейфует)
