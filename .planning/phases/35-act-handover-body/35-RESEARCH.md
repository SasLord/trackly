# Phase 35: Тело акта приёма-передачи - Research

**Researched:** 2026-08-11
**Domain:** MiniJinja HTML-шаблоны печатных форм (Rust/Tauri/axum, файловый рендер-пайплайн Фазы 16)
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Текст акта (DOC-09, критерии #1 и #2)**

- **D-01:** Текст Word-образца сохраняется дословно. Каноничная двусторонняя форма («ООО … в
  лице …, с одной стороны, и …, с другой стороны, составили настоящий акт о нижеследующем»,
  нумерованные пункты) — отвергнута, равно как и гибрид с двусторонней преамбулой одной фразой.
  Остаётся текущая формулировка: «Настоящим актом утверждаю, что мною: ⟨act.receiver_name⟩ /
  было получено устройство: ⟨item.name⟩ / Инвентарный номер: … / Серийный номер: … / Модель: …
  / Комплектация: … / Технические характеристики: … / Состояние: … / Сроком до: …».
  DOC-09 закрывается формулировкой «сверено с каноном Word-образца, решено не менять» — это и
  есть требуемое критерием #1 согласование текста до вёрстки. Планировщику НЕ поручать
  переписывание текста и НЕ переоткрывать этот вопрос.
- **D-02:** Множественное число при N > 1. При одном устройстве строка образца дословно — «было
  получено устройство: ⟨name⟩». При нескольких — одна строка «были получены устройства:» и
  дальше список, без повтора метки на каждое устройство. Отвергнуты: повтор метки на каждое
  устройство и отсылка этого вопроса целиком в Фазу 36. Граница с Фазой 36: здесь решается
  только формулировка; ветвление вёрстки на «первый лист / Приложение №1» и разрывы страниц
  остаются Фазе 36.
- **D-03:** Строка «Сроком до:» выводится всегда, даже когда `act.deadline_utc` пустой; в этом
  случае под значением остаётся полоска для вписывания от руки. Сейчас блок целиком скрывается
  (`{%- if act.deadline_human %}…{%- elif act.deadline %}`). Согласуется с DOC-07: полоска
  остаётся ровно там, где пишут рукой. Отвергнуты: скрывать (текущее поведение) и показывать
  прочерк «—».

**Стороны и состав тела**

- **D-04:** Организация и локация в теле НЕ упоминаются. Организация присутствует только как
  шапка-бланк (`_header.html`, Фаза 34) — как в Word-образце. `act.location_name` есть в
  контексте, но не выводится и выводиться не будет. `act.notes` в контексте шаблона вообще нет
  и добавляться не будет.
- **D-05:** Должности подписантов не выводятся. Их нет ни в схеме БД акта, ни в `ActCreateDto`,
  ни в контексте рендера; добавление потребовало бы миграции, полей DTO и правки формы создания
  акта — это противоречит «без изменений бэкенда» в критерии #4. Отложено (см. Deferred).

**Блок подписей (DOC-08, критерий #4)**

- **D-06:** Горизонтальный блок, отдельная строка на подписанта: `Выдал: ⟨полоска под подпись⟩
  ⟨ФИО текстом⟩` и `Получил: ⟨полоска⟩ ⟨ФИО текстом⟩`. ФИО берутся из `act.giver_name` и
  `act.receiver_name` соответственно — бэкенд не меняется. Текущий `.signatures`
  (`display: grid; grid-template-columns: 1fr 1fr`) с двумя полосками на подписанта заменяется.
- **D-07:** Мелкое пояснение «Подпись» остаётся только под полоской. Пояснение «ФИО» под
  напечатанным ФИО убирается — оно избыточно, когда там уже стоит реальное ФИО.
- **D-08:** Даты подписания в строке подписанта нет. Дата акта уже выводится в подзаголовке
  («№1 от 11 августа 2026»), как в образце.
- **D-09:** Акт приёмки (`act_acceptance.html`) приводится к тому же виду: его блок подписей
  переверстывается по D-06/D-07, а дублирующие строки `Кто передал` / `Кто принял` удаляются из
  таблицы — ФИО остаются в одном месте, в блоке подписей. Контекст приёмки уже содержит
  `document.giver_name` / `document.receiver_name` — бэкенд не меняется. `report.html` не
  трогается (блока подписей там нет).

**Подчёркивания и разделители в теле (DOC-07, критерий #3)**

- **D-10:** `border-bottom` у `.field-row .value` снимается во всём теле акта приёма-передачи.
  Полоски остаются ровно в двух местах: под подписью (D-06) и под пустым «Сроком до» (D-03).
- **D-11:** Метка и значение — сплошным текстом через пробел («Инвентарный номер: отсутствует»),
  значение переносится естественно. Отвергнуты: выравнивание всех значений в общую колонку и
  выделение метки жирным (в Word-образце метки НЕ жирные).
- **D-12:** Пустая строка-заглушка удаляется совсем — `act_handover.html:119-121`
  (`.field-row` с `<span class="value">&nbsp;</span>`). В образце это была вторая полоска под
  рукописное продолжение ФИО; при автоподстановке она не нужна.

**Ограничения (выведены, не обсуждались)**

- **C-01:** Нужен новый срез `_legacy_defaults/v22/` с телами `act_handover.html` и
  `act_acceptance.html` в состоянии после Фазы 34 (текущий HEAD), добавленный в
  `KNOWN_LEGACY_DEFAULTS` дополнительными элементами слайсов.
- **C-02:** D-11 означает, что `.field-row` перестаёт быть flex-строкой `label | value`; doc-
  комментарий в шапке `act_handover.html` (строки 2-36) описывает старую структуру — его надо
  обновить вместе с разметкой, там же добавить `act.giver_name` в перечень ключей контекста.
- **C-03:** D-06 возвращает в тело акта `giver_name`, который Фаза 15 (D-09) намеренно оттуда
  убрала. Тесты, писавшиеся вокруг того решения, придётся тронуть.
- **C-04:** D-09 меняет `act_acceptance.html` — значит срез v22 обязан включать и его тело.
- **C-05:** Проверка результата — рендер настоящего PDF/превью, а не тест извлечения текста.
  Проверять на обоих транспортах; для LAN-браузера обязателен `pnpm --dir ui build`.
- **C-06:** durable-гейт `ui/scripts/check-print-isolation.mjs` в `pnpm --dir ui lint` должен
  остаться зелёным.
- **C-07:** 🔒 Word-образец и реальные акты содержат настоящие ФИО и реквизиты организации. Ни в
  коммитах фазы, ни в её планах/саммари/тестах этих значений быть не должно — только
  вымышленные («Иванов И.И.», «Петров П.П.») и переменные.

### Claude's Discretion

- Точные величины: ширина полоски под подпись, отступы между строками подписантов, выравнивание
  меток «Выдал:»/«Получил:» друг под другом, вертикальный воздух между интро и перечнем (после
  удаления заглушки D-12), кегль пояснения «Подпись» (в образце 8pt).
- Способ реализации D-02 в MiniJinja (`{% if act.items | length > 1 %}` против вынесенной
  переменной) и точная формулировка списка имён при N > 1.
- Разметочный приём для D-11 (единый `<div>` с меткой в `<span>` против отказа от flex вовсе).
- Нужен ли структурный тест-гейт «в теле нет `border-bottom`» по образцу Фазы 33 D-13 — если
  дёшев, стоит завести; обязательным его пользователь не делал.
- Имя каталога среза (`v22` предполагается по нумерации Фазы 34 — **research подтвердил
  фактически**, см. §Package/Slice Legitimacy ниже).

### Deferred Ideas (OUT OF SCOPE)

- Должности подписантов в акте (D-05) — требует миграции таблицы актов, полей в `ActCreateDto`
  и в форме создания акта. Отдельная фаза, если понадобится.
- Локация и примечание к акту в печатной форме (D-04) — вывод обоих отклонён ради верности
  образцу.
- Каноничная двусторонняя форма акта (отклонённый вариант A из D-01) — сохранена как описанная
  альтернатива на случай передачи контрагенту, а не сотруднику.
- Блок подписей в `report.html` — сейчас его там нет; если потребуется, это Future Requirement.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-------------------|
| DOC-07 | В акте приёма-передачи нет полосок-подчёркиваний под автоматически подставляемым текстом; полоски только там, где расписываются от руки | §Architecture Patterns (Pattern 1 — снятие `border-bottom`), §Code Examples, §Common Pitfalls (Pitfall 4) |
| DOC-08 | Блок подписей — горизонтальный, по строке на подписанта, с автоподставленными ФИО | §Architecture Patterns (Pattern 2 — горизонтальный блок подписей), §Common Pitfalls (Pitfall 1, Pitfall 3) |
| DOC-09 | Текст акта составлен в каноничной форме и согласован с пользователем до вёрстки | Закрыто CONTEXT.md D-01 — согласование уже состоялось; research подтверждает, что текст образца дословно совпадает с текущим `act_handover.html` (см. §Summary) |

</phase_requirements>

## Summary

Фаза 35 — это правка ДВУХ файлов шаблонов (`act_handover.html`, `act_acceptance.html`) и
демо-контекста предпросмотра (`template_service.rs`), без единого изменения в бэкенд-логике
сбора контекста рендера. Все решения по содержанию уже приняты пользователем в CONTEXT.md
(D-01..D-12); задача этого research — не переоткрыть их, а зафиксировать точные координаты
правки, механизм доставки в уже установленные копии (legacy-defaults slice) и то, какие
существующие тесты сломаются предсказуемо (плановый дрейф) — включая **один тестовый файл,
который CONTEXT.md не назвал**, но который содержит идентичную ломающуюся ассерцию.

Ключевые находки:

1. **`act.giver_name` уже в контексте, `_header.html` не трогается.** Обе стороны (D-06)
   реализуются чистой правкой разметки/CSS. `act.deadline` / `act.deadline_human` уже в
   контексте — D-03 требует только снять `{%- if %}`-обёртку вокруг всего блока.
2. **Следующий срез — `v22`, подтверждено фактически** (на диске только `v20`/`v21`,
   `KNOWN_LEGACY_DEFAULTS` содержит ровно 2 элемента на файл + пустую запись для
   `_header.html`).
3. **Демо-контекст предпросмотра НЕ содержит `act.giver_name`.** Под `UndefinedBehavior::Strict`
   это значит: как только `act_handover.html` начнёт читать `act.giver_name` (D-06),
   `TemplateService::validate_preview("act_handover", …)` — а значит и live-редактор шаблонов в
   Settings → Шаблоны — упадёт с ошибкой рендера на ЛЮБОМ теле, включая нетронутый бандл. Это
   правка, которую CONTEXT.md не called out явно (только «проверить заглушку»), но она
   **обязательна**, не опциональна.
4. **Тестовый дрейф шире, чем зафиксировано в CONTEXT.md C-03.** Кроме `pdf_render_act.rs` и
   `acts_e2e_smoke.rs`, идентичная строгая ассерция `html.contains("ФИО")` (по D-07 подлежащая
   удалению) есть и в `crates/trackly-app/tests/html_act_render.rs:188`
   (`html_handover_contains_required_blocks_and_logo`). Планировщик обязан включить этот файл в
   `files_modified`.
5. **`length`-фильтр MiniJinja уже используется в проде** (`report.html:117` —
   `groups | length == 0`), значит `{% if act.items | length > 1 %}` для D-02 — не гипотеза, а
   подтверждённый рабочий паттерн этого же движка с включённой фичей `builtins`.
6. **Проверка настоящим рендером теперь означает браузерный Paged.js-предпросмотр, не
   `qlmanage`-PDF.** Память `act-pdf-word-fidelity` описывает эпоху krilla/PDF-байтов (Фазы
   14-15); с Фазы 16 `render_pdf` возвращает HTML-строку, а «настоящий PDF» появляется только
   через `window.print()` в `PdfPreviewModal.svelte` (Paged.js, Фаза 33) — то есть единственный
   путь проверки C-05 — открыть реальное приложение (десктоп + LAN-браузер после
   `pnpm --dir ui build`), а не автоматизированный `#[ignore]`-тест, генерирующий файл на диске.

**Primary recommendation:** правка ограничивается `act_handover.html` (doc-comment, CSS
`.field-row`/`.signatures`/`.signature`, интро/заглушка/цикл по устройствам/«Сроком до»/блок
подписей), `act_acceptance.html` (таблица + `.signature`), новым срезом
`_legacy_defaults/v22/{act_handover,act_acceptance}.html`, регистрацией в
`KNOWN_LEGACY_DEFAULTS`, добавлением `act.giver_name` (и, для N>1 веток, ничего нового — ключ
`act.items` уже есть) в `demo_context_for_kind`'s `_` ветку в `template_service.rs`, и правкой
минимум ДВУХ тестовых файлов (`pdf_render_act.rs`, `html_act_render.rs`) плюс проверкой
`acts_e2e_smoke.rs`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Текст и вёрстка тела акта (HTML-разметка + inline CSS) | API / Backend (файловый шаблон, читается сервисным слоем) | — | Шаблоны — не UI-компоненты Svelte, а файлы в `templates/`, читаемые и рендерящиеся в Rust (`act_service.rs` → `minijinja_env.rs`). Правка происходит на бэкенд-стороне репозитория, хотя результат — HTML. |
| Контекст рендера (`act.giver_name`, `act.items`, `act.deadline*`) | API / Backend | — | Собирается в `act_service::render_pdf`/`render_acceptance_pdf`; уже полный, фаза не меняет. |
| Демо-контекст предпросмотра шаблонов | API / Backend | — | `template_service::demo_context_for_kind` — синтетические данные для live-редактора; должен быть расширён `act.giver_name`, иначе живой предпросмотр несогласован с реальным рендером под `UndefinedBehavior::Strict`. |
| Доставка обновлённого шаблона в уже установленные копии | Database / Storage (файловая ФС рядом с `.exe`, не БД) | API / Backend (upgrade-логика в `html_templates.rs`) | `_legacy_defaults` — структурное сравнение байтов на диске при старте; классифицируется как «Storage», т.к. это файловая персистентность конфигурации печатной формы, но управляется backend-кодом. |
| Печать / визуальное превью (Paged.js) | Browser / Client | Frontend Server (SSR-эквивалента нет — SPA раздаётся `tower-http::ServeDir`) | Вне границы фазы (Фаза 33); упоминается только для §Validation Architecture — рендер тела акта проверяется через этот слой, а не через Rust-тест. |

## Standard Stack

Фаза не устанавливает новых зависимостей — правка ограничена уже используемым стеком.

### Core (уже в проекте, без изменений версий)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `minijinja` | `^2.20` (Cargo.toml, `crates/trackly-app`) [VERIFIED: codebase grep, Cargo.toml:53] | Рендер HTML-шаблонов актов | `UndefinedBehavior::Strict` + autoescape HTML — уже сконфигурированный безопасный движок (Фаза 16 T-16-01/02), фичи `["builtins", "json", "fuel", "serde", "multi_template"]` включают фильтр `length`, использованный D-02 |
| `axum` / `tower-http::ServeDir` | как в проекте | Раздача `ui/dist` для LAN-браузерного предпросмотра | Не меняется этой фазой; упомянуто только для C-05/§Validation |

### Package Legitimacy Audit

**Не применимо.** Фаза не устанавливает и не обновляет ни один внешний пакет (ни Rust crate, ни
npm-зависимость) — правка ограничена содержимым `.html`-шаблонов, тестов и одной Rust-функции
демо-контекста внутри уже существующего файла. `slopcheck`/registry-проверки пропущены как
неприменимые к этой фазе.

## Architecture Patterns

### System Architecture Diagram

```
Пользователь (десктоп webview ИЛИ LAN-браузер)
        │  открывает предпросмотр акта (PdfPreviewModal.svelte, Фаза 33)
        ▼
Tauri invoke  ИЛИ  axum HTTP (/api/v1/acts/{id}/pdf)
        │
        ▼
act_service::render_pdf(act_id)                       [act_service.rs:~2500-2673]
  ├─ читает org-реквизиты (OrgDbService::get_for_pdf)
  ├─ resolve_templates_dir()  → <exe_dir>/templates ИЛИ TRACKLY_TEMPLATES_DIR
  ├─ load_template("act_handover.html", embedded_default)   ← файл с диска ПРИОРИТЕТНО
  ├─ load_template("_header.html", embedded_default)
  ├─ собирает ctx = { org: {...}, act: { giver_name, receiver_name, items[], deadline*, ... } }
  ▼
minijinja_env::render_with_timeout(build_safe_html_env(), "act_handover_html", template_src, ctx,
                                    extra_templates=[("_header.html", header_src)])
  ├─ UndefinedBehavior::Strict  → любой непереданный ключ = ошибка рендера
  ├─ AutoEscape::Html           → {{ var }} экранируется по умолчанию
  ├─ {% include "_header.html" %}  разрешается ТОЛЬКО из in-memory реестра (T-16-02, без loader)
  ▼
HTML-строка (не PDF-байты!)
        │
        ▼
Возвращается вызывающей стороне → вставляется в #act-print-root (PdfPreviewModal.svelte)
        │  window.print()  (Paged.js, Фаза 33)
        ▼
Реальный многостраничный PDF/печать — ЕДИНСТВЕННОЕ место, где видна геометрия
(полоски, перенос текста, разрывы) — text-extraction тесты этого не видят.
```

### Recommended Project Structure (не меняется, для ориентира)
```
crates/trackly-app/
├── templates/
│   ├── act_handover.html        # правится этой фазой (тело + doc-comment)
│   ├── act_acceptance.html      # правится этой фазой (таблица + .signature)
│   ├── report.html              # НЕ трогается
│   ├── _header.html             # НЕ трогается (Фаза 34)
│   └── _legacy_defaults/
│       ├── v20/                 # существует
│       ├── v21/                 # существует (Фаза 34 pre-header-share snapshot)
│       └── v22/                 # НОВЫЙ — снимок ПЕРЕД правками этой фазы (C-01)
├── src/pdf/html_templates.rs    # DEFAULT_HTML_TEMPLATES + KNOWN_LEGACY_DEFAULTS — добавить v22
├── src/pdf/minijinja_env.rs     # НЕ трогается (env/autoescape уже готовы)
├── src/services/act_service.rs  # НЕ трогается (контекст уже полный)
└── src/services/template_service.rs  # demo_context_for_kind — ДОБАВИТЬ act.giver_name
```

### Pattern 1: Снятие подчёркивания у автоподставляемых полей (DOC-07 / D-10 / D-11)

**Что:** `.field-row .value { border-bottom: 1px solid #000; }` в `act_handover.html` (строки
73-78) убирается для всего тела; вместо `label | value` в отдельных `<span>` — сплошной текст
через пробел (D-11), в единственном `<div class="field-row">`. Полоска остаётся только в двух
местах: пустое «Сроком до» (D-03) и линия подписи `.signature .line` (D-06, уже с
`border-bottom`, не трогается по сути стиля — трогается geometry/layout).

**Когда:** применить к каждому `.field-row` внутри цикла `{%- for item in act.items %}` и к
интро-строке.

**Пример (текущее, ДО правки):**
```html
<!-- Source: crates/trackly-app/templates/act_handover.html:129-134 (текущий HEAD) -->
{%- if item.inventory_no %}
<div class="field-row">
  <span class="label">Инвентарный номер:</span>
  <span class="value">{{ item.inventory_no }}</span>
</div>
{%- endif %}
```
Вариант приведения к D-11 (сплошной текст, без подчёркивания) — концептуальная схема, точная
разметка на усмотрение исполнителя (Claude's Discretion):
```html
{%- if item.inventory_no %}
<div class="field-row">Инвентарный номер: {{ item.inventory_no }}</div>
{%- endif %}
```
CSS-класс `.field-row` при этом теряет `display: flex; align-items: baseline; gap: 6pt` (уже не
нужен для двух `<span>`) — сохраняется как блочный элемент с вертикальным `margin`.

### Pattern 2: Горизонтальный блок подписей, строка на подписанта (DOC-08 / D-06 / D-07 / D-08)

**Что:** `.signatures { display: grid; grid-template-columns: 1fr 1fr; }` с двумя `.signature`
(«Выдал» и «Получил» рядом, каждая с двумя полосками «Подпись»/«ФИО») заменяется на блок, где
каждый подписант — ОДНА строка: метка → полоска → напечатанное ФИО. Только «Подпись» остаётся
под полоской; «ФИО»-подпись под напечатанным именем убирается (D-07). Дат в строке нет (D-08).

**Пример (текущее, ДО правки):**
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
Целевая форма по D-06/D-07/D-08 (концептуально, точные величины — discretion): одна строка на
подписанта, полоска и напечатанное ФИО в одной горизонтали, «Подпись» мелким кеглем ПОД
полоской, БЕЗ второй полоски и без подписи «ФИО»:
```html
<div class="signatures">
  <div class="signature-row">
    <span class="signature-label">Выдал:</span>
    <span class="signature-line"></span>
    <span class="signature-name">{{ act.giver_name }}</span>
  </div>
  <div class="signature-row">
    <span class="signature-label">Получил:</span>
    <span class="signature-line"></span>
    <span class="signature-name">{{ act.receiver_name }}</span>
  </div>
</div>
```
`{{ act.giver_name }}` / `{{ act.receiver_name }}` рендерятся через `AutoEscape::Html` без
`| safe` (обычная HTML-экранированная интерполяция) — сохраняет тот же инвариант T-16-01,
никакого нового escaping-кода не требуется.

`act_acceptance.html` (D-09) переверстывается по этому же паттерну, читая
`document.giver_name`/`document.receiver_name` (ключи уже в контексте, см.
`act_service.rs:2681+`), и таблица `Кто передал`/`Кто принял` (строки 88-89) удаляется.

### Pattern 3: Множественное число и всегда-видимое «Сроком до» (D-02 / D-03)

```html
<!-- Существующий рабочий пример length-фильтра в проде -->
<!-- Source: crates/trackly-app/templates/report.html:117 -->
{%- if groups is not defined or groups | length == 0 %}
```
Подтверждает, что `{% if act.items | length > 1 %}…{% else %}…{% endif %}` — рабочий паттерн
того же движка (фича `builtins` включена в Cargo.toml). Для D-03 достаточно убрать внешний
`{%- if act.deadline_human %}…{%- elif act.deadline %}…{%- endif %}` и оставить всегда:
```html
<!-- Текущее (ДО, act_handover.html:168-178) -->
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
После D-03 строка выводится безусловно; полоска (D-10 исключение) под пустым значением
достигается тем, что `.field-row .value` в этом ОДНОМ месте сохраняет `border-bottom` (или
получает выделенный CSS-класс), тогда как остальные `.field-row` этот стиль теряют.

### Anti-Patterns to Avoid
- **Не переносить `border-bottom` на уровень отдельного класса `.underline`, забыв снять базовый
  `.field-row .value`** — иначе DOC-07 формально не закрывается (полоски останутся везде по
  умолчанию, а не только в двух местах).
- **Не выводить `act.giver_name` где-либо, кроме блока подписей** — D-04/D-01 фиксируют, что
  преамбула остаётся от первого лица получателя; повторное появление ФИО сдающего в интро было
  бы отходом от согласованного текста.
- **Не забыть обновить `demo_context_for_kind`** — иначе живой редактор шаблонов
  (Settings → Шаблоны → предпросмотр) начнёт падать на `act.giver_name` под
  `UndefinedBehavior::Strict`, даже если сам production-рендер акта работает штатно (два разных
  контекста, оба должны содержать одинаковый набор ключей).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Условие «одно устройство vs несколько» (D-02) | Кастомный Rust-флаг `is_plural` в JSON-контексте | `act.items | length > 1` прямо в шаблоне | `length`-фильтр уже используется в `report.html`; изменение контекста в Rust потребовало бы правки `act_service.rs`, что противоречит «бэкенд не меняется» |
| HTML-экранирование `act.giver_name`/`act.receiver_name` | Ручной `escape()`/regex в Rust перед вставкой в JSON | Обычная `{{ var }}`-интерполяция под `AutoEscape::Html` (уже включено) | Движок уже настроен на автоэкранирование HTML (T-16-01); ручное экранирование поверх него либо избыточно, либо создаёт двойное экранирование |
| Доставка нового вида шаблона в уже установленные копии | Скрипт миграции/апдейтер поверх файловой системы | `KNOWN_LEGACY_DEFAULTS` + `upgrade_untouched_defaults_on_startup` (уже написан и покрыт тестами) | Механизм существует с Фазы 16/34; фазе нужно только добавить срез `v22`, не писать новый апдейтер |

**Key insight:** вся инфраструктура (безопасный MiniJinja-движок, файловый резолвер шаблонов,
legacy-defaults апгрейд) уже построена предыдущими фазами (16, 20, 34) — Фаза 35 работает строго
внутри неё, никакого нового «hand-rolled» механизма не требуется.

## Common Pitfalls

### Pitfall 1: Демо-контекст предпросмотра не знает про `act.giver_name`
**Что идёт не так:** `TemplateService::demo_context_for_kind("act_handover")` (и любой
нераспознанный `kind`, деградирующий на ту же ветку) не содержит ключ `act.giver_name`
([template_service.rs:504-529], проверено research). После того как `act_handover.html`
получит `{{ act.giver_name }}` в блоке подписей (D-06), `validate_preview` — вызываемый как
из `update_body` (сохранение правки в редакторе), так и напрямую для предпросмотра — начнёт
падать с `AppError::Validation { field: "template" }` под `UndefinedBehavior::Strict`, даже
если реальный production-рендер акта (`act_service::render_pdf`) работает штатно (там
`act.giver_name` уже есть).
**Почему это происходит:** два независимых JSON-контекста (боевой в `act_service.rs`,
демонстрационный в `template_service.rs`) обязаны содержать одинаковый набор ключей под
`UndefinedBehavior::Strict`, но ничто не проверяет их синхронность автоматически.
**Как избежать:** добавить `"giver_name": "Иванов И.И."` в `act`-объект `_`-ветки
`demo_context_for_kind` (та же вымышленная форма ФИО, что уже используется в
`document.giver_name` для `act_acceptance`-ветки).
**Признаки:** `template_edit.rs`/`templates_seed.rs` тесты, вызывающие `validate_preview`, либо
живой UI-редактор шаблонов, начинают падать на пустом/нетронутом теле `act_handover.html`.

### Pitfall 2: Каталог `target/debug/templates/` содержит устаревшую копию от Фазы 34
**Что идёт не так:** research подтвердил (`ls -la target/debug/templates/`), что каталог уже
материализован (файлы датированы 11 августа — временем завершения Фазы 34) и содержит
`act_handover.html`/`act_acceptance.html` В СОСТОЯНИИ ДО правок этой фазы. `cargo tauri dev`
материализует его заново при следующем запуске, если он отсутствует — но если он уже есть,
`materialize_defaults_on_startup` его не тронет (insert-only), а `upgrade_untouched_defaults_on_startup`
обновит его только если байты совпадают с известным legacy-срезом.
**Почему это происходит:** это build-артефакт вне git (Фаза 34 D-18 задокументировала эту же
ловушку); правки шаблона в `crates/trackly-app/templates/` не видны в запущенном
`cargo tauri dev`, пока не пересоберётся embedded-default И не сработает upgrade-путь (или
каталог не будет удалён вручную).
**Как избежать:** перед UAT этой фазы удалить `target/debug/templates/` (как это уже делалось
в 34-06) — тогда `cargo tauri dev` материализует свежий embedded-default из обновлённого
`act_handover.html`.
**Признаки:** UAT показывает старую вёрстку (с подчёркиваниями/старым блоком подписей) несмотря
на то, что исходный `.html`-файл в `templates/` уже отредактирован.

### Pitfall 3: Тестовый дрейф шире заявленного в CONTEXT.md C-03
**Что идёт не так:** CONTEXT.md называет только `pdf_render_act.rs` и
`acts_e2e_smoke.rs::handover_pdf_render_within_e2e` как тесты, требующие правки. Research нашёл
ТРЕТИЙ файл с идентичной ломающейся ассерцией:
`crates/trackly-app/tests/html_act_render.rs:188` в тесте
`html_handover_contains_required_blocks_and_logo`:
```rust
for expected in ["Акт приема-передачи", "Выдал", "Получил", "Подпись", "ФИО"]
```
Строка `"ФИО"` здесь проверяет ИМЕННО ту сублейбл-подпись, которую отменяет D-07. Без правки
этого файла `cargo test -p trackly-app` не пройдёт после изменения `act_handover.html`, даже
если `pdf_render_act.rs`/`acts_e2e_smoke.rs` уже поправлены.
**Почему это происходит:** три разных integration-test файла независимо писали похожие
smoke-проверки блока подписей в разное время (Фазы 3/16/20) — грепа по одному файлу
недостаточно, нужен греп по всему `tests/`.
**Как избежать:** искать `"ФИО"`, `"Подпись"`, `field-row`, `border-bottom`, `deadline`,
`giver_name` по ВСЕМ `crates/trackly-app/tests/*.rs`, не только по файлам, названным в
CONTEXT.md. Полный список файлов, где встречаются акт-шаблоны:
`html_page_parity.rs` (безопасен — трогает только `@page`, не body), `html_act_render.rs`
(**требует правки**, см. выше), `pdf_column_overflow.rs` (безопасен — не проверяет метки),
`pdf_render_act.rs` (**требует правки**, см. C-03), `template_edit.rs`/`templates_seed.rs`
(безопасны сами по себе, но зависят от Pitfall 1 выше через `validate_preview`),
`templates_status.rs` (безопасен — не проверяет содержимое body), `acts_e2e_smoke.rs`
(**требует правки**, см. C-03), `html_header_parity.rs` (безопасен — только шапка).
**Признаки:** `cargo test -p trackly-app` красный на `html_handover_contains_required_blocks_and_logo`
после правки `act_handover.html`, хотя план не упоминал этот файл в `files_modified`.

### Pitfall 4: Text-extraction тесты не видят визуальную регрессию
**Что идёт не так:** ни один из существующих Rust-тестов не проверяет CSS
(`border-bottom` присутствует/отсутствует, `display: grid` vs `display: block`) — все они
проверяют только НАЛИЧИЕ текстовых меток в HTML-строке. Тест может остаться зелёным, даже если
DOC-07 (подчёркивания сняты) формально не выполнен — например, если исполнитель забыл убрать
`border-bottom` из CSS-блока, но переставил разметку `label|value` в один `<div>`.
**Почему это происходит:** унаследованная архитектура тестового покрытия с Фазы 3/14/15 (память
`act-pdf-word-fidelity`): текстовая экстракция «выживает» после любых геометрических изменений.
**Как избежать:** для критериев #3 и #5 фазы полагаться на два механизма: (а) дешёвый
структурный regex-тест на ОТСУТСТВИЕ `border-bottom` внутри тела `.field-row .value` (по образцу
`html_page_parity.rs`'s `extract_page_block`, discretion пользователя — заводить не обязательно,
но дёшево и рекомендуется); (б) обязательный визуальный рендер в реальном приложении (см.
§Validation Architecture, Manual-Only).
**Признаки:** `cargo test` зелёный, но живой предпросмотр показывает полоски там, где их не
должно быть.

### Pitfall 5: `qlmanage`-путь верификации из памяти проекта устарел для этой фазы
**Что идёт не так:** память `act-pdf-word-fidelity` описывает верификацию через
`qlmanage -t -s 1400 -o <dir> file.pdf` — этот путь относится к эпохе krilla/PDF-байтов (Фазы
14-15, до Фазы 16). С Фазы 16 `render_pdf` возвращает HTML-строку, а не PDF-файл; PDF
материализуется ТОЛЬКО в момент `window.print()` внутри `PdfPreviewModal.svelte`
(Paged.js-движок, Фаза 33). Автоматизированного пути «сгенерировать PDF-файл на диске из
Rust-теста» для ЭТОГО шаблона больше не существует — планировщик не должен закладывать
throwaway `#[ignore]`-тест, пишущий `.pdf` на диск, как способ проверки DOC-07/DOC-08.
**Почему это происходит:** архитектурный пивот Фазы 16 (`pdf-pivot-to-html-print`, память
проекта) сменил источник истины с серверного PDF-рендера на браузерную печать; часть старых
project-memory заметок описывает уже неактуальный для HTML-пайплайна workflow.
**Как избежать:** верификация C-05 = запустить настоящее приложение (десктоп через
`cargo tauri dev`; LAN-браузер после `pnpm --dir ui build`), открыть предпросмотр акта
приёма-передачи и акта приёмки, визуально сравнить с ожиданиями D-06..D-12. Никакого
Rust-side PDF-файла не создаётся и не проверяется автоматически.
**Признаки:** попытка написать `#[ignore]` тест, вызывающий `qlmanage`, зависает или требует
воссоздания byte-PDF генератора, которого для HTML-пайплайна не существует.

## Code Examples

### Текущее состояние `.field-row` CSS (act_handover.html:64-78, до правки)
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
D-10/D-11 требуют: `.field-row .value { border-bottom: ... }` — снять; `display: flex` на
`.field-row` — вероятно снять (текст идёт сплошным потоком, а не в две колонки), но точная
разметка — discretion.

### Использование `length`-фильтра, уже проверенное в проде
```html
<!-- Source: crates/trackly-app/templates/report.html:117 -->
{%- if groups is not defined or groups | length == 0 %}
```
Подтверждает доступность `| length` без дополнительной настройки Environment (фича `builtins`
уже в `Cargo.toml:53`).

### Контекст рендера handover (не меняется, для справки)
```rust
// Source: crates/trackly-app/src/services/act_service.rs:2629-2661 (текущий HEAD)
let ctx = serde_json::json!({
    "org": { /* ... не используется в теле, только в _header.html ... */ },
    "act": {
        "number": act.number_raw,
        "suffix": suffix,
        "date": format_iso_date(act.handover_date_utc),
        "date_human": format_ru_date(act.handover_date_utc),
        "giver_name": act.giver_name,           // уже есть — используется D-06
        "receiver_name": act.receiver_name,     // уже используется интро (D-01)
        "deadline": act.deadline_utc.map(format_iso_date),
        "deadline_human": act.deadline_utc.map(format_ru_date),
        "location_name": act.location,          // не выводится (D-04)
        "items": items_json,                    // .length используется для D-02
        "parent": parent_block,
    },
    // ...
});
```

### Демо-контекст, требующий правки (Pitfall 1)
```rust
// Source: crates/trackly-app/src/services/template_service.rs:504-529 (текущий HEAD)
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
        // giver_name ОТСУТСТВУЕТ — обязательная правка этой фазы
        "location_name": "Офис 101",
        "deadline": null,
        "deadline_human": null,
        "parent": null,
        "items": [ /* один элемент */ ]
    }
}),
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| PDF-байты генерируются в Rust через krilla/DocSpec, проверяются `pdf-extract` в тестах и `qlmanage` вручную | HTML-строка генерируется в Rust, PDF материализуется браузером через Paged.js при `window.print()` | Фаза 16 (2026-07, `pdf-pivot-to-html-print`) | Автоматизированные Rust-тесты этой фазы могут проверять только НАЛИЧИЕ текста/структурных маркеров в HTML-строке — геометрия (подчёркивания, перенос, разрывы) проверяется исключительно человеком через реальный рендер |
| `giver_name` не выводился в теле handover-акта (только в подписи «Выдал» без ФИО) | `giver_name` выводится текстом рядом с полоской подписи | D-06 этой фазы, откат части D-09 Фазы 15 | Требует правки трёх тестовых файлов, ранее закрепивших отсутствие `giver_name` в теле |

**Deprecated/outdated:**
- `qlmanage`-путь верификации PDF (память `act-pdf-word-fidelity`) — относится к krilla-эпохе, не
  применим к текущему HTML+Paged.js пайплайну без адаптации (см. Pitfall 5).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|-----------------|
| A1 | Точное значение ширины полоски под подпись, отступов и кегля «Подпись» (discretion пользователя) реализуемо чисто CSS-величинами без переработки структуры `.signatures` | §Architecture Patterns Pattern 2 | Низкий — это чисто визуальная точная подгонка, откатывается правкой CSS без переписывания разметки |
| A2 | Структурный regex-гейт «в теле нет `border-bottom`» (discretion, не обязателен) технически реализуем по образцу `html_page_parity.rs`'s `extract_page_block`, применённому к диапазону между `{% include "_header.html" %}` и `.signatures` | §Common Pitfalls Pitfall 4 | Низкий — если окажется дороже ожидаемого, пользователь явно не сделал его обязательным, можно опустить без нарушения критериев |

**Обе записи — низкий риск и не блокируют планирование**; ни одна не требует подтверждения
пользователем до старта работ (в отличие от факта наличия `v20`/`v21` и отсутствия
`act.giver_name` в демо-контексте — эти два факта проверены инструментально, не ASSUMED).

## Open Questions

1. **Нужен ли структурный тест-гейт «в теле handover нет `border-bottom`» (по образцу Фазы 33
   D-13)?**
   - Что мы знаем: пользователь явно оставил это на discretion Claude, не сделал обязательным;
     `html_page_parity.rs` — готовый образец такого гейта (regex-извлечение блока + сравнение).
   - Что неясно: оправдана ли дополнительная тестовая поверхность для CSS-инварианта, который
     иначе проверяется только визуально (Pitfall 4).
   - Recommendation: завести, если стоимость невелика (~15-20 строк regex-теста на образце
     `html_page_parity.rs`) — это дешёвый durable-гейт против будущей регрессии («кто-то вернул
     border-bottom при следующей правке»), аналогично тому, как это уже сделано для `@page`.

2. **Требует ли живой предпросмотр в UI Settings → Шаблоны (`TemplateEditor`) отдельного шага
   ручной UAT-проверки?**
   - Что мы знаем: `validate_preview`/`demo_context_for_kind` — тот же движок, что и боевой
     рендер; правка демо-контекста (Pitfall 1) обязательна, иначе редактор сломан для ЛЮБОГО
     тела `act_handover.html`, включая нетронутый бандл.
   - Что неясно: тестирует ли Фаза 35 UAT именно редактор шаблонов, или только конечный
     печатный вывод (боевой акт).
   - Recommendation: включить быстрый ручной прогон Settings → Шаблоны → редактор
     `act_handover`/`act_acceptance` → «Предпросмотр» без сохранения — в §Validation Architecture
     Manual-Only ниже уже заложено.

## Environment Availability

Фаза не добавляет внешних зависимостей — секция пропущена (только код/шаблоны + существующий
`cargo test`/`pnpm` тулчейн, уже верифицированный Фазой 34).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (workspace), integration-таргеты в `crates/trackly-app/tests/*.rs` |
| Config file | нет отдельного — workspace `Cargo.toml` |
| Quick run command | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib pdf:: -- --test-threads=1` |
| Full suite command | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --test-threads=1` (требует реального `pnpm --dir ui build` в `ui/dist`) |

**Жёсткие ограничения (project memory, не переоткрывать):**
- Никогда не запускать два `cargo test` параллельно — контенция на `target/`-lock выглядит как
  зависание (`cargo-no-concurrent-test`).
- `cargo test --workspace` виснет на `auth_remember_cookie` — использовать таргетированные
  `-p trackly-app` команды выше.

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|--------------------|--------------|
| DOC-07 | В теле handover нет `border-bottom` под автоподставляемыми полями | structural (discretion) | `cargo test -p trackly-app --test html_field_row_underline_gate` (имя примерное, если заведён — см. Open Question 1) | ❌ Wave 0 (опционально) |
| DOC-07 / DOC-08 | Существующий N=1 handover-рендер: без «ФИО»-сублейбла, с напечатанным `giver_name` в блоке подписей | integration | `cargo test -p trackly-app --test pdf_render_act signature_renders_two_line_labels` (переименовать/переписать) | ✅ существует, требует переписывания |
| DOC-08 | `giver_name` в блоке подписей handover без изменений бэкенда | integration | `cargo test -p trackly-app --test pdf_render_act render_handover_act_produces_cyrillic_pdf` (комментарий обновить, ассерция уже совместима) | ✅ существует, комментарий устарел |
| DOC-08 | `html_handover_contains_required_blocks_and_logo` — метки без «ФИО» | integration | `cargo test -p trackly-app --test html_act_render html_handover_contains_required_blocks_and_logo` | ✅ существует, требует переписывания (Pitfall 3) |
| DOC-08 | Акт приёмки: горизонтальный блок подписей, без дубля в таблице | integration | `cargo test -p trackly-app --test html_act_render html_acceptance_contains_required_blocks` (расширить ассерции под D-09) | ✅ существует, требует расширения |
| DOC-08 | `acts_e2e_smoke::handover_pdf_render_within_e2e` — комментарий про D-09/Фаза-15 устарел, ассерция технически совместима | integration | `cargo test -p trackly-app --test acts_e2e_smoke handover_pdf_render_within_e2e` | ✅ существует, комментарий обновить |
| DOC-09 | Интро-фраза текста акта не изменилась (текст образца сохранён дословно) | integration | `cargo test -p trackly-app --test pdf_render_act render_handover_act_contains_d09_intro_phrase` | ✅ существует, не трогать (уже проходит) |
| DOC-08 (preview) | Живой редактор шаблонов не падает на `act.giver_name` под strict-контекстом | integration | `cargo test -p trackly-app --test template_edit` (существующие тесты `update_body_*`/`reset_to_default` косвенно упадут, если демо-контекст не поправлен) | ✅ существует, косвенное покрытие |
| — | `@page`-паритет трёх шаблонов не сломан (durable-гейт, не трогать) | structural | `cargo test -p trackly-app --test html_page_parity` | ✅ существует, не трогать |
| — | Шапка (`_header.html`) не задета правкой тела | structural/integration | `cargo test -p trackly-app --test html_header_parity` | ✅ существует, не трогать |

### Sampling Rate
- **Per task commit:** `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib pdf:: -- --test-threads=1` (~20 с)
- **Per wave merge:** полный `cargo test -p trackly-app -- --test-threads=1` с предварительным `pnpm --dir ui build`
- **Phase gate:** полный сьют зелёный + Level-2 ручной визуальный проход (ниже) на обоих транспортах, ДО `/gsd-verify-work`

### Wave 0 Gaps
- [ ] Правка `demo_context_for_kind`'s `_`-ветки (`template_service.rs`) — добавить
      `act.giver_name` (Pitfall 1). Без этого live-редактор шаблонов ломается на любом теле
      `act_handover.html`.
- [ ] Опционально: `crates/trackly-app/tests/html_field_row_underline_gate.rs` — структурный
      regex-тест «в диапазоне между `_header.html`-include и `.signatures` нет
      `border-bottom`» (Open Question 1; не обязателен, но дёшев по образцу
      `html_page_parity.rs`).
- [ ] Новый срез `_legacy_defaults/v22/{act_handover,act_acceptance}.html` + регистрация в
      `KNOWN_LEGACY_DEFAULTS` (C-01/C-04) — без него существующая тестовая пара
      `every_default_template_has_a_known_legacy_defaults_entry` /
      `upgrade_replaces_v21_legacy_default_with_current_bundled_body` продолжит проходить
      (они проверяют СУЩЕСТВОВАНИЕ слайсов, не их полноту), но реальные установленные копии
      не получат новое тело — регрессия того же класса, что билась в Фазе 34 (D-15).
- [ ] Удалить (или задокументировать необходимость удаления) `target/debug/templates/` перед
      UAT (Pitfall 2) — этот шаг не тест, а операционное действие перед ручной проверкой.

**Manual-Only (обязательно, C-05):**

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|-------------|--------------------|
| Отсутствие полосок под автоподставляемыми полями визуально, перенос текста без «лесенки» под метку (D-11) | DOC-07 (критерий #3) | Text-extraction тесты не видят CSS/геометрию (Pitfall 4, память `act-pdf-word-fidelity`) | Открыть реальный предпросмотр акта приёма-передачи (десктоп-вебвью), визуально подтвердить: полоски только под подписью и под пустым «Сроком до» |
| Горизонтальный блок подписей с двумя строками, напечатанными ФИО (D-06/D-07/D-08) | DOC-08 (критерий #4) | То же — геометрия невидима text-extraction тестам | В том же предпросмотре сравнить блок подписей со Спецификацией D-06/D-07/D-08 |
| Многострочные/длинные значения переносятся естественно без обрыва слова (D-11) | DOC-07 | Перенос — CSS-поведение движка печати | Ввести акт с длинными «Комплектация»/«Технические характеристики», убедиться в естественном переносе |
| То же на LAN-транспорте | DOC-07/DOC-08 (критерий #5) | Сервер-режим раздаёт `ui/dist`; десктоп HMR не покрывает браузер (память `dev-browser-testing-needs-ui-build`) | `pnpm --dir ui build`, запустить сервер-режим, открыть предпросмотр в LAN-браузере, повторить визуальное сравнение |
| Акт приёмки (`act_acceptance.html`) приведён к тому же виду блока подписей, дубль ФИО в таблице убран (D-09) | DOC-08 | Геометрия/дубли не проверяются text-extraction | Открыть предпросмотр «Документ приёма устройства на склад», подтвердить единственное появление ФИО и новый формат подписей |
| Редактор шаблонов (Settings → Шаблоны) не падает на предпросмотре `act_handover`/`act_acceptance` | DOC-08 (косвенно) | Требует живого UI, не покрыто фронтенд-тестами (`ui/` без раннера — Фаза 34 constraint) | Открыть Settings → Шаблоны → выбрать `act_handover`/`act_acceptance` → нажать «Предпросмотр» без изменений |

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V5 Input Validation / Output Encoding | yes | `AutoEscape::Html` в `build_safe_html_env()` уже покрывает интерполяцию `act.giver_name`/`act.receiver_name` (те же самые значения, что уже проходят через `act.receiver_name` в интро-параграфе — новый sink не вводится, только новое МЕСТО использования уже проверенного паттерна) |
| V2/V3/V4/V6 | no | Фаза не касается аутентификации, сессий, авторизации или криптографии — правка ограничена шаблонами печатной формы |

### Known Threat Patterns for HTML-шаблонов MiniJinja

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|-----------------------|
| Stored-XSS через `giver_name`/`receiver_name`, если бы шаблон использовал `| safe` | Tampering/Injection (T-16-01, уже закрыт Фазой 16) | НЕ добавлять `| safe` к `{{ act.giver_name }}`/`{{ act.receiver_name }}` — обычная экранированная интерполяция (это и есть требование этой фазы: новых `| safe`-сайтов D-06/D-07/D-08/D-09 не создают) |
| Рассинхрон демо-контекста и боевого контекста под `UndefinedBehavior::Strict` (Pitfall 1) | Availability (DoS предпросмотра для админа) | Держать оба JSON-контекста (`act_service::render_pdf` и `template_service::demo_context_for_kind`) с идентичным набором ключей — уже установленный проектный инвариант (Фаза 16/17), эта фаза обязана его соблюсти для нового ключа `act.giver_name` в демо-ветке |

Новых угроз (T-35-*) правка не вводит — единственный ключ, впервые появляющийся в теле шаблона
(`act.giver_name`), уже присутствует в контексте, уже того же типа (строка `String`), что и
`act.receiver_name`, уже интерполируемый в этом же файле безопасным способом.

## Sources

### Primary (HIGH confidence — прочитано напрямую из репозитория, [VERIFIED: codebase grep])
- `crates/trackly-app/templates/act_handover.html` (полностью, 205 строк) — точные координаты CSS/разметки
- `crates/trackly-app/templates/act_acceptance.html` (полностью, 110 строк)
- `crates/trackly-app/src/pdf/html_templates.rs` (полностью) — `DEFAULT_HTML_TEMPLATES`,
  `KNOWN_LEGACY_DEFAULTS`, механизм materialize/upgrade + существующие unit-тесты
- `crates/trackly-app/src/pdf/minijinja_env.rs` (полностью) — `build_safe_html_env`,
  `render_with_timeout`, инвариант autoescape/UndefinedBehavior
- `crates/trackly-app/src/services/act_service.rs:2540-2720` — контекст рендера handover +
  acceptance
- `crates/trackly-app/src/services/template_service.rs:330-530` — `validate_preview`,
  `demo_context_for_kind` (найден пробел — Pitfall 1)
- `crates/trackly-app/tests/pdf_render_act.rs` (1-420) — тесты, требующие правки (C-03)
- `crates/trackly-app/tests/html_act_render.rs` (150-508) — **дополнительный тест, найденный
  research**, требующий правки (не был в CONTEXT.md)
- `crates/trackly-app/tests/acts_e2e_smoke.rs:259-320` — `handover_pdf_render_within_e2e`,
  `acceptance_pdf_render_smoke`
- `crates/trackly-app/tests/html_page_parity.rs`, `html_header_parity.rs`,
  `pdf_column_overflow.rs`, `template_edit.rs`, `templates_seed.rs`, `templates_status.rs` —
  просмотрены на предмет скрытых зависимостей от тела акта (все безопасны, кроме описанных выше)
- `ui/scripts/check-print-isolation.mjs` (1-60) — подтверждено: не зависит от классов
  `.field-row`/`.signature`, гейт не затронут
- `scripts/check-privacy-requisites.sh` (1-60) — подтверждено: проверяет только org-реквизиты
  (inn/kpp/okpo/ogrn/phone/fax), не ФИО; фазу не затрагивает
- `ls crates/trackly-app/templates/_legacy_defaults/` — подтверждено: только `v20`/`v21`,
  следующий срез — `v22`
- `ls target/debug/templates/` — подтверждено: каталог существует, датирован временем
  завершения Фазы 34 (Pitfall 2)
- `crates/trackly-app/Cargo.toml:53` — версия minijinja и включённые фичи
- `.planning/phases/34-document-header/34-VALIDATION.md` (полностью) — образец структуры
  Validation Architecture/Manual-Only для этого же кодового пути

### Secondary (MEDIUM confidence)
- `~/.claude/projects/.../memory/act_pdf_word_fidelity.md` — верно для эпохи krilla (Фазы
  14-15), для этой фазы устарело в части `qlmanage`-команды (см. Pitfall 5), но верно в части
  «text-extraction тесты не видят геометрию»
- `~/.claude/projects/.../memory/dev_browser_testing_needs_ui_build.md` — подтверждает
  необходимость `pnpm --dir ui build` для LAN-браузерной проверки

### Tertiary (LOW confidence)
- нет — вся фактура этой фазы проверяема прямым чтением кода проекта, внешние источники
  (Context7/WebSearch) не требовались (нет новых библиотек)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — новых зависимостей нет, всё уже сконфигурировано и протестировано
  предыдущими фазами
- Architecture: HIGH — прочитаны все затрагиваемые файлы целиком, координаты правок точные
- Pitfalls: HIGH — три из пяти найденных pitfall подтверждены прямым запуском команд
  (`ls`, `grep`) против реального состояния репозитория, не гипотетически

**Research date:** 2026-08-11
**Valid until:** пока не изменится `act_handover.html`/`act_acceptance.html`/
`template_service.rs::demo_context_for_kind` — практически «до конца выполнения этой фазы»;
30 дней как верхняя граница для остального (версии зависимостей, project memory)
