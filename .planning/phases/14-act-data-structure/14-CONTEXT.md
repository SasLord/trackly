# Phase 14: Данные и структура акта - Context

**Gathered:** 2026-07-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Обеспечить, чтобы **все данные**, нужные для PDF-акта по образцу Word, были доступны
в контексте генерации PDF (то, что уходит в MiniJinja → DocSpec): расширенные реквизиты
организации, «Технические характеристики» позиции, «Комплектация», «Состояние», «Срок до»
и полный список N позиций устройства с атрибутами. Схема БД и контекст рендера — в объёме
этой фазы.

**Только данные/схема/контекст.** Визуальный рендер PDF под образец Word (шаблон,
вёрстка блоков, двухстрочные подписи, «Выдал/Получил», кириллица, regression-тесты) —
это **Phase 15**, не здесь.

Requirements этой фазы: **PDFA-03, PDFA-04, PDFA-06** (см. `.planning/REQUIREMENTS.md`).

**Важный факт из scout'а — большая часть уже существует:**
- **Мультиустройство** — контекст рендера уже отдаёт `act.items[]` (список позиций),
  шаблон уже идёт циклом. Данные готовы.
- **«Срок до»** — колонка `acts.deadline_utc` (миграция V014) существует; захватывается
  при создании акта (`act_service.rs` L270) и прокидывается в контекст (`deadline`/`deadline_human`).
- **«Комплектация» / «Состояние»** — снимок на позиции акта уже есть и уже в контексте
  рендера: `act_items.complectation_at_time` → `item.kit`, `act_items.condition_at_time` → `item.condition`.
- **Единственный пробел по данным позиции:** `specs` («Технические характеристики»)
  в контексте рендера **захардкожен `serde_json::Value::Null`** (`act_service.rs`, items_json).

</domain>

<decisions>
## Implementation Decisions

### D-01 — «Технические характеристики»: использовать существующее поле устройства
- Не создавать новую колонку. Использовать **уже существующее** поле устройства
  `specs` (DTO/UI «Технические характеристики»), которое маппится на DB-колонку
  `devices.notes` (см. `crates/trackly-infra/src/repos/devices_sqlite.rs` L9–11:
  `specs ↔ notes`, `kit ↔ complectation`, `state ↔ condition`).
- Задача фазы: **заменить захардкоженный `specs: Null`** в контексте рендера
  (`act_service.rs` items_json) на реальное значение `specs` из устройства позиции.
  Вероятно потребуется дотащить `device.specs`(=`notes`) до `ActItemRow`/запроса,
  используемого при сборке контекста.
- Читаем **живое значение с устройства** (не снимок): пользователь выбрал «поле устройства».
  Отдельный снимок `specs_at_time` на `act_items` НЕ создаём.
- «Комплектация»/«Состояние» остаются как есть — снимки `complectation_at_time`/
  `condition_at_time` на `act_items` (point-in-time), уже присутствуют в контексте.

### D-02 — Расширенные реквизиты организации: явные колонки
- Расширить `org_settings` **явными колонками**: `phone`, `fax`, `email`, `okpo`, `ogrn`
  (миграция `ALTER TABLE ADD COLUMN ... TEXT NOT NULL DEFAULT ''`, паттерн V026 —
  DEFAULT-строки, чтобы старые строки не давали NULL).
- Расширить `HeaderBlock` (`crates/trackly-app/src/pdf/docspec.rs`) полями
  phone/fax/email/okpo/ogrn и прокинуть их в контекст рендера (`org.*`).
- НЕ укладывать многострочно в существующий `address` — реквизиты структурированы,
  редактируются пополе в Настройках, чисто привязываются в шаблоне шапки (Phase 15).
- Затрагивает также: settings service/DTO/HTTP + Tauri bindings + UI Настроек
  (ввод новых полей) — планировщику учесть сквозной путь.

### D-03 — Семантика документа: переработать `act_handover`, без нового kind
- НЕ вводить новый `kind` в `document_templates` (нет CHECK-миграции, нет UI-выбора вида).
- Образец Word = «Акт приёма-передачи» (выдача сотруднику, «Выдал/Получил»), что и есть
  семантика существующего `act_handover` (`giver_name`/`receiver_name`). Дефолтный шаблон
  `act_handover` перерабатывается под образец; лейблы «Сдал/Принял» → «Выдал/Получил» —
  это работа шаблона/рендера в **Phase 15**.
- Роль Phase 14: убедиться, что контекст рендера `act_handover` несёт всё, что
  понадобится переработанному шаблону (specs позиции + расширенные реквизиты).

### D-04 — Номер и дата акта: печатать реальные значения
- Печатать **реальные `№` и дату** акта (как сейчас), а не прочерки под ручное заполнение.
- Обоснование: автоприсвоение номера акта — core value проекта («без ручного присвоения
  номеров актов»). Прочерки противоречат этому и рвут связь PDF↔запись в БД.
- Практически изменений в данных не требует (контекст уже отдаёт `act.number`/`act.date`).

### Claude's Discretion
- Точная форма запроса, дотаскивающего `device.specs` до сборки контекста
  (расширить `ActItemRow` vs отдельный SELECT) — на усмотрение планировщика/исполнителя.
- Порядок/именование новых колонок реквизитов внутри миграции.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Образец и бриф (главный источник «что»)
- `.planning/PHASE-BRIEF-act-pdf-word-fidelity.md` — самодостаточный бриф: точный разбор
  образца Word, гэп-анализ, скоуп-решения пользователя. **Читать первым.**
- `.planning/reference/act-word-source/act-sample.docx` — исходный образец акта
- `.planning/reference/act-word-source/image1.png` — логотип; `image2.png` — линия-разделитель
- `.planning/ROADMAP.md` §«Phase 14» — цель, success criteria, requirements
- `.planning/REQUIREMENTS.md` — PDFA-03 (расширенная шапка), PDFA-04 (комплектация/
  тех.характеристики/срок), PDFA-06 (дефолтный шаблон редактируем через `document_templates`)

### Схема и данные (что уже есть)
- `migrations/V003__devices.sql` — таблица `devices` (`condition`, `complectation`, `notes`)
- `migrations/V004__acts.sql` — `acts` + `act_items` (`condition_at_time`, `complectation_at_time`)
- `migrations/V014__acts_indexes_and_status_codes.sql` L34–36 — `acts.deadline_utc` («Срок до»)
- `migrations/V026__org_settings.sql` — `org_settings` (name/inn/kpp/address/logo); **сюда добавлять реквизиты**
- `crates/trackly-infra/src/repos/devices_sqlite.rs` L9–11 — маппинг `specs↔notes`, `kit↔complectation`, `state↔condition`

### Рендер-пайплайн (куда прокидывать данные)
- `crates/trackly-app/src/services/act_service.rs` — сборка контекста рендера
  (`items_json` с захардкоженным `specs: Null`; `render_pdf` ~L1322, `render_acceptance_pdf` ~L1411)
- `crates/trackly-app/src/pdf/docspec.rs` — `HeaderBlock` (расширить реквизитами), `Section`-примитивы
- `crates/trackly-app/templates/act_handover.minijinja` — дефолтный шаблон (контекст-переменные документированы в шапке файла)
- `crates/trackly-app/src/services/template_service.rs` — `DEFAULT_TEMPLATES`, `seed_defaults_on_startup`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`act.items[]` в контексте** — список позиций уже есть; для мультиустройства не нужна
  новая инфраструктура, только богаче поля per-item (добавить `specs`).
- **`act_items.complectation_at_time`/`condition_at_time`** — снимки уже пишутся при
  создании акта (`act_service.rs` L388–396 из `source_before.kit`/`.state`) и уже в контексте.
- **`acts.deadline_utc`** — «Срок до» полностью прокинут end-to-end; изменений не требует.
- **`org_settings` (V026)** — паттерн NOT NULL DEFAULT '' для безопасного ADD COLUMN;
  optimistic-lock `version`; single-row (id=1).
- **`HeaderBlock`** — единая точка реквизитов в DocSpec; расширяется полями + renderer.

### Established Patterns
- **Миграции через `refinery`** — авто-применение при старте, sequential `PRAGMA user_version`,
  `downgrade_protection` тест. Новая миграция обязана следовать нумерации (следующая свободная V0NN).
- **Снимок на позиции акта** — point-in-time значения фиксируются в `act_items` на момент
  создания (для комплектации/состояния). Тех.характеристики намеренно НЕ снимок (D-01).
- **Сквозной путь настроек** — новое поле org требует: миграция → repo → service → DTO →
  HTTP route + Tauri command (bindings regen) → UI Настроек. Учесть все звенья.

### Integration Points
- Контекст рендера (`act_service.rs`): `org.*` (+реквизиты) и `items[].specs` (из device).
- `document_templates` / seed: шаблон `act_handover` остаётся редактируемым (PDFA-06),
  переработка дефолт-сида — Phase 15.

</code_context>

<specifics>
## Specific Ideas

- Пользователь явно поправил: «В Устройствах есть такое поле Технические, его и надо
  использовать» → D-01 (device `specs`), а не новая колонка/снимок.
- Обратная совместимость обязательна: существующие акты (до фазы) должны открываться
  и генерировать PDF без ошибок (ROADMAP success criterion #4). Новые org-колонки — с
  DEFAULT; отсутствующие `specs`/реквизиты деградируют в «—»/пусто, не в ошибку.

</specifics>

<deferred>
## Deferred Ideas

- **Весь визуальный рендер под образец Word** (шаблон, порядок блоков, `ItemsTable`
  с колонками Комплектация/Тех.характеристики + перенос длинных ячеек, двухстрочные
  подписи «Выдал/Получил», кириллица, regression-тесты) → **Phase 15** (PDFA-01/02/05/07/08).
- Редактор шаблонов в UI, импорт/экспорт .docx, прочие виды документов — вне скоупа
  milestone (см. бриф §«Вне скоупа»).

None иных — обсуждение осталось в границах фазы.

</deferred>

---

*Phase: 14-Данные и структура акта*
*Context gathered: 2026-07-03*
