# Roadmap: Trackly

Trackly — портативное приложение для учёта техники, принтеров и картриджей с серверным
режимом для LAN-доступа. Релизная линия v1 завершена (v1.0 + v1.1). Полные детали фаз
заархивированы в `.planning/milestones/`.

## Milestones

- ✅ **v1.0 — Базовый учёт** — Phases 1–8 (shipped 2026-06-19) → `milestones/v1.1-ROADMAP.md`
- ✅ **v1.1 — AD, сотрудники и картриджная взаимосвязь** — Phases 9–13 (shipped 2026-06-26) → `milestones/v1.1-ROADMAP.md`
- ✅ **v1.1.1 — PDF-акт по образцу Word (мультиустройство)** — Phases 14–15 (completed 2026-07-04)
- ✅ **v1.1.1 — Документы через HTML-печать** — Phases 16–17 (shipped 2026-07-07)
- ✅ **v1.1.2 — Пост-релизные доработки UX и печати** — Phases 18–22 (shipped 2026-07-15) → `milestones/v1.1.2-ROADMAP.md`
- 🚧 **v1.2 — Редизайн UI и дизайн-система** — Phases 23–30 (planning)

## Phases

<details>
<summary>✅ v1.0 — Базовый учёт (Phases 1–8) — SHIPPED 2026-06-19</summary>

- [x] Phase 1: Фундамент (6/6 plans) — completed 2026-05-25
- [x] Phase 2: Устройства и базовый UI (5/5 plans) — completed 2026-05-28
- [x] Phase 3: Акты приёма-передачи и первая PDF-печать (5/5 plans) — completed 2026-05-30
- [x] Phase 03.1: Acts quantity model + UAT gap closure (6/6 plans, INSERTED)
- [x] Phase 03.2: Deferred UAT gap closure (2/2 plans, INSERTED)
- [x] Phase 03.3: Device-list UX round 2 (2/2 plans, INSERTED) — completed 2026-06-07
- [x] Phase 4: Картриджи (6/6 plans) — completed 2026-06-12
- [x] Phase 5: Авторизация и серверный режим (6/6 plans) — completed 2026-06-14
- [x] Phase 6: Принтеры (SNMP-мониторинг) и Заявки (9/9 plans) — completed 2026-06-15
- [x] Phase 7: Отчёты, Дашборд и Настройки (14/14 plans) — completed 2026-06-18
- [x] Phase 8: Релизный пайплайн (Windows/macOS/Linux) (2/2 plans) — completed 2026-06-19

</details>

<details>
<summary>✅ v1.1 — AD, сотрудники и картриджная взаимосвязь (Phases 9–13) — SHIPPED 2026-06-26</summary>

- [x] Phase 9: AD-аутентификация и заявки на регистрацию пользователей (5/5 plans) — completed 2026-06-20
- [x] Phase 10: Ограничение роли employee + employee-UI + role-gating read (4/4 plans) — completed 2026-06-21
- [x] Phase 11: Заявки/employee UX gap-closure (3/3 plans) — completed 2026-06-22
- [x] Phase 12: Взаимосвязь картриджной заявки (21/21 plans) — completed 2026-06-25
- [x] Phase 13: Редизайн совместимости Принтеры↔Картриджи (8/8 plans) — completed 2026-06-26

</details>

**v1.1.1 — PDF-акт по образцу Word (мультиустройство) — SHIPPED 2026-07-04**

- [x] **Phase 14: Данные и структура акта** - Схема/контекст акта содержит все поля образца (реквизиты, комплектация, тех.характеристики, срок до, мультиустройство, двухстрочные подписи) и достижимы через существующий механизм `document_templates`. (completed 2026-07-03)
- [x] **Phase 15: Рендер и соответствие образцу** - Дефолтный шаблон и рендерер производят PDF, визуально соответствующий образцу Word, с мультиустройством и regression-тестами. (completed 2026-07-04)

**v1.1.1 — Документы через HTML-печать — SHIPPED 2026-07-07**

<!-- Планировался как v1.2, но вышел в составе релиза v1.1.1 (см. PROJECT.md → Current State
     и MILESTONES.md). Ярлык приведён в соответствие с фактом 2026-07-16 при старте
     настоящего v1.2 «Редизайн UI и дизайн-система» (фазы 23–30). -->


- [x] **Phase 16: Документы через HTML-печать** - Оба акта (приёма-передачи и приёмки устройства) генерируются из HTML-шаблонов (папка `templates/` рядом с exe + вшитый дефолт-fallback) и печатаются/сохраняются в PDF через диалог браузера в обоих режимах (desktop + LAN), визуально по образцу Word; krilla/DocSpec заморожен и не используется. (SPEC: 16-SPEC.md) (completed 2026-07-05)
- [x] **Phase 17: Отчёты и Шаблоны через HTML-печать** - Отчёты и редактор Шаблонов переходят на HTML-печать по паттерну Phase 16; krilla/DocSpec выведены из активного пути (заморожены, не удалены). (SPEC: 17-SPEC.md) (plans 7/7; gap-closure 17-05..17-07 planned 2026-07-07 — см. 17-VERIFICATION.md) (completed 2026-07-07)

**v1.1.2 — Пост-релизные доработки UX и печати — SHIPPED 2026-07-15**

- [x] **Phase 18: Автокомплит и дропдауны** - Все автокомплиты рендерятся в `body` (portal), не обрезаются в модалках; выбор устройства в актах открывается по фокусу, фильтруется вводом, группирует одинаковые устройства с раскрытием деталей и схлопывает единственную группу до плоского списка. (completed 2026-07-10)
- [x] **Phase 19: Акты — дата и редактирование** - Дата «Когда отдали» сохраняется как дата акта вместо текущей даты; кнопка «Редактировать» открывает рабочую форму редактирования существующего акта. (verification: gaps_found — открыт BLOCKER CR-01) (completed 2026-07-11)
- [x] **Phase 20: Печать актов и организация** - Печать device-акта выводит полную шапку организации (логотип, название, ИНН, реквизиты); организация поддерживает безопасную загрузку SVG-логотипа и вторую строку адреса в печатных формах. (completed 2026-07-14)
- [x] **Phase 21: Точечные фиксы — коды картриджей/фотобарабанов** - Автокод нового картриджа — `C-XXXX`, нового фотобарабана — `D-XXXX`. (completed 2026-07-14)

**v1.2 — Редизайн UI и дизайн-система — ACTIVE**

- [ ] **Phase 23: Токены и основы дизайн-системы** - Единый слой `--tr-*` (поверхности, текст, акцент, семантика, нейтрали, тени), типографика и миграция space/radius по значению без сдвига вёрстки; фикс undefined-token багов.
- [ ] **Phase 24: Базовые компоненты** - Button, Input/Select/Textarea/Checkbox, Badge, Tabs, Modal на новой системе.
- [ ] **Phase 25: Таблицы и Dropdown** - Строки таблицы + строка-группа, новый компонент Dropdown/комбобокс.
- [ ] **Phase 26: Окна с готовым макетом** - Дашборд и Устройства — точное соответствие макету Claude Design.
- [ ] **Phase 27: Окна основного рабочего процесса** - Акты, Картриджи, Принтеры — без макета, вёрстка из компонентной системы.
- [ ] **Phase 28: Окна поддержки и администрирования** - Заявки, Отчёты, Настройки, Пользователи — без макета.
- [ ] **Phase 29: Вход и интерфейс сотрудника** - Логин/Pending/Blocked/FirstRunWizard, EmployeeLayout — отдельные layout-shell.
- [ ] **Phase 30: Качество — доступность и паритет платформ** - AA-контраст, focus ring, визуальный паритет Tauri WebView vs LAN-браузер.

## Phase Details

### Phase 14: Данные и структура акта

**Goal**: Все данные, которых не хватает для образца Word (расширенные реквизиты организации, Комплектация, Технические характеристики, Срок до, N позиций устройства, двухуровневые подписи), доступны в контексте генерации PDF — через схему БД и/или ввод при создании акта — и передаются в рендер-пайплайн (DocSpec).

**Depends on**: Phase 13 (предыдущий milestone, завершён)

**Requirements**: PDFA-03, PDFA-04, PDFA-06

**Decisions to make during planning (не блокируют фазу, разрешаются в рамках неё):**

- Где хранить «Комплектация»/«Технические характеристики»: на устройстве, на позиции акта (снимок), или свободный ввод в форме акта.
- Хранить ли «Срок до» в `acts` как поле.
- Расширять ли `HeaderBlock`/`org_settings` явными полями (телефон/факс/email/ОКПО/ОГРН) или укладывать в существующий `org_address` как многострочный текст.
- Новый `kind` в `document_templates` (CHECK-constraint + миграция) или переработка `act_handover` под семантику «Выдал/Получил».
- Печатать реальный номер/дату акта или прочерки под ручное заполнение.

**Success Criteria** (what must be TRUE):

1. Форма создания/просмотра акта позволяет ввести (или подтягивает автоматически) Комплектацию, Технические характеристики и Срок до для каждой позиции акта — без обращения к внешним таблицам.
2. Настройки организации (`org_settings`) хранят и отдают телефон, факс, e-mail, ОКПО, ОГРН в дополнение к уже существующим name/inn/kpp/address.
3. Контекст рендера (то, что уходит в MiniJinja/DocSpec) включает: список из N позиций устройства с полными атрибутами (не одна запись), расширенные реквизиты организации, срок действия акта.
4. Существующие акты (созданные до этой фазы) продолжают открываться и генерировать PDF без ошибок (обратная совместимость схемы/миграций).
5. Изменения схемы применяются через миграцию (`refinery`), запускаются автоматически при старте — без ручных SQL-шагов.

**Plans**: 3 plans
**Wave 1**

- [x] 14-01-PLAN.md — Миграция V033 org_settings +5 реквизитов; OrgPatch/OrgSettingsDto/3 SQL-сайта/HeaderBlock

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 14-02-PLAN.md — Сквозной путь реквизитов: HTTP/Tauri passthrough + bindings + input-поля Настроек
- [x] 14-03-PLAN.md — Контекст акта: specs↔notes (D-01) + источник org на org_settings (D-05) + backward-compat

---

### Phase 15: Рендер и соответствие образцу

**Goal**: Сгенерированный PDF акта визуально воспроизводит структуру и содержание образца Word (шапка, заголовок, вводная формулировка, мультиустройство, срок, подписи), кириллица рендерится корректно, а дефолтный редактируемый шаблон и тесты обновлены и проходят.

**Depends on**: Phase 14

**Requirements**: PDFA-01, PDFA-02, PDFA-05, PDFA-07, PDFA-08

**Success Criteria** (what must be TRUE):

1. PDF акта, сгенерированный из дефолтного шаблона, содержит все блоки образца в правильном порядке: шапка (логотип + расширенные реквизиты), заголовок «Акт приема-передачи», номер/дата, вводная формулировка с ФИО получателя, блок устройства(-в), «Сроком до», подписи «Выдал/Получил».
2. Акт с несколькими устройствами (2+) печатает все позиции корректно в табличном виде (`ItemsTable`), включая корректный перенос длинных значений (Комплектация, Технические характеристики) без обрезки и наложения текста. *(Уточнение при планировании: параметрическое «(`ItemsTable`)» заменено гибридной вёрсткой per-device card из D-06 — `ItemsTable`/`truncate_to_width` обрезает, а не переносит длинные поля, поэтому длинные значения рендерятся отдельным word-wrap-примитивом. Суть критерия — все позиции без обрезки/наложения — сохранена.)*
3. Блок подписей печатает две подписи («Выдал», «Получил»), каждая двухстрочная («Подпись» / «ФИО»), как в образце.
4. Кириллица (включая длинные значения комплектации/характеристик и реквизиты организации) рендерится корректно во всех блоках нового шаблона — без квадратов/пропусков символов.
5. Дефолтный `.minijinja`-шаблон обновлён под новый вид, сидируется при первом запуске через `template_service`, и остаётся редактируемым через `document_templates` (не хардкод в Rust сверх дефолт-сида); существующие PDF-пайплайн тесты проходят, добавлены новые тесты на мультиустройство (1 vs N позиций) и новый шаблон.

**Plans**: 4 plans (gap closure round 1 added after verification found a pagination gap)

**Wave 1**

- [x] 15-01-PLAN.md — Renderer capability: DocSpec Signature two-line sublabels (D-07), render_header_two_column, wrap_text_to_width (ttf-parser), extended Signature render arm

**Wave 2** *(depends on Wave 1)*

- [x] 15-02-PLAN.md — Wire pipeline: fix WR-03 logo-BLOB plumbing in act_service.rs, DeviceCard hybrid section (D-06), rewrite act_handover.minijinja (D-09), sync validate_preview demo_ctx

**Wave 3** *(depends on Wave 2)*

- [x] 15-03-PLAN.md — Test coverage: multi-device (1 vs N) wrap tests, two-line signature test, full-pipeline logo test (closes WR-03 regression gap), regenerate pdf_determinism fixture

**Wave 4** *(gap closure — depends on Wave 3; closes WR-05/PDFA-02 pagination gap from 15-VERIFICATION.md)*

- [x] 15-04-PLAN.md — Page-break/pagination in render_docspec/render_section (DeviceCard kept atomic across page boundaries) + full-pipeline page-count regression test

---

### Phase 16: Документы через HTML-печать

**Goal**: Генерация обоих актов переходит с krilla/DocSpec-пайплайна на HTML-шаблоны: шаблоны лежат в папке `templates/` рядом с исполняемым файлом (редактируются как файлы) с вшитым дефолтом-fallback, рендерятся в self-contained HTML по образцу Word и печатаются/сохраняются в PDF через диалог браузера в обоих режимах (desktop + LAN).

**Depends on**: Phase 15 (контекст акта + образец Word зафиксированы)

**Milestone**: v1.1.1 — Документы через HTML-печать

**Spec**: `16-SPEC.md` (ambiguity 0.17)

**Success Criteria** (what must be TRUE): см. `16-SPEC.md` — Acceptance Criteria.

**Plans**: 5 plans in 4 waves

**Wave 1**

- [x] 16-01-PLAN.md — HTML template contracts: act_handover.html/act_acceptance.html ported from .minijinja, pdf/html_templates.rs (resolver + materialize + fallback), build_safe_html_env (autoescape ON), Paths::templates_dir()

**Wave 2** *(depends on Wave 1)*

- [x] 16-02-PLAN.md — Wire act_service.rs: render_pdf/render_acceptance_pdf return HTML String, data: URI logo, AppCtx startup materialization

**Wave 3** *(depends on Wave 2, parallel plans — no file overlap)*

- [x] 16-03-PLAN.md — Tauri/HTTP adapters: String return type, text/html content-type, delete acts_open_pdf_in_system, regenerate bindings.ts
- [x] 16-05-PLAN.md — Backend tests: migrate existing full-pipeline tests off PDF assertions, new html_act_render.rs (D-14 coverage), krilla #[ignore] hygiene (D-13)

**Wave 4** *(depends on Wave 3 — needs regenerated bindings.ts)*

- [x] 16-04-PLAN.md — Frontend: acts.ts/pdf.ts return-type update, PdfPreviewModal.svelte srcdoc + print, remove system-open button

### Phase 18: Автокомплит и дропдауны

**Goal**: Автокомплиты по всему приложению рендерят свой выпадающий список через portal в `body` (не обрезаются и не ломают вёрстку внутри модалок), а выбор устройства в форме акта работает полноценно: раскрывается по фокусу, фильтруется вводом, группирует одинаковые устройства с раскрытием деталей экземпляра и схлопывает единственную оставшуюся группу до плоского списка.

**Depends on**: Phase 17 (предыдущий milestone, завершён)

**Milestone**: v1.1.2 — Пост-релизные доработки UX и печати

**Requirements**: AUTO-01, AUTO-02, AUTO-03, AUTO-04, AUTO-05

**Success Criteria** (what must be TRUE):

1. Любой автокомплит, открытый внутри модального окна, разворачивает список поверх содержимого модалки (portal в `body`) — без обрезки, без появления внутреннего скролла и без искажения вёрстки диалога.
2. В форме акта (Акты → Позиции) поле выбора устройства раскрывает список доступных устройств сразу при получении фокуса, без необходимости начать ввод.
3. Ввод текста в поле выбора устройства фильтрует список по наименованию в реальном времени.
4. Одинаковые по наименованию устройства объединены в раскрываемую группу; раскрыв группу, пользователь видит и может выбрать конкретный экземпляр с его инвентарным №, серийным №, моделью и состоянием.
5. Если после фильтрации в списке остаётся единственная группа, она не отображается как группа — вместо неё сразу показывается плоский список устройств из этой группы.

**Plans**: 5 plans in 3 waves
**UI hint**: yes

**Wave 1**

- [x] 18-01-PLAN.md — Backend: list_grouped группировка name+model, сортировка count DESC, текстовый фильтр name/inv#/SN через FTS5 (AUTO-03/04/05 контракт)
- [x] 18-02-PLAN.md — dropdownAnchor.ts (portal-anchor слой) + первый потребитель LocationAutocomplete (AUTO-01)

**Wave 2** *(depends on Wave 1 — 18-03 нужен dropdownAnchor.ts, 18-04 нужен backend-контракт + dropdownAnchor.ts; параллельны друг другу — нет пересечения файлов)*

- [x] 18-03-PLAN.md — PersonAutocomplete/DeviceAutocompleteField portal-миграция + AUTO-01 аудит 4 native-select компонентов
- [x] 18-04-PLAN.md — ActFormItemsTable: portal-дропдаун per-row, focus-open (AUTO-02), фильтрация (AUTO-03), рендер группы name+model+count (D-05)

**Wave 3** *(depends on Wave 2 — тот же файл, что 18-04)*

- [x] 18-05-PLAN.md — ActFormItemsTable: drill-in по группе (AUTO-04 D-06/D-07), схлопывание единственной группы (AUTO-05 D-09), финальный чекпоинт

---

### Phase 19: Акты — дата и редактирование

**Goal**: Дата, введённая пользователем при создании акта, используется как дата акта, а существующий акт можно открыть в рабочей форме редактирования и сохранить изменения.

**Depends on**: Phase 18

**Milestone**: v1.1.2 — Пост-релизные доработки UX и печати

**Requirements**: ACT-01, ACT-02

**Success Criteria** (what must be TRUE):

1. При создании акта значение поля «Когда отдали» сохраняется как дата акта — не подставляется автоматически текущая дата.
2. Кнопка «Редактировать» на карточке существующего акта активна (не задизейблена).
3. Нажатие «Редактировать» открывает форму со всеми текущими данными акта, и внесённые изменения сохраняются без ошибок.

**Plans**: 8 plans (19-01..19-05 + gap-closure 19-06..19-08)
**UI hint**: yes

**Note**: ACT-02 — диагностика проведена в ходе research/planning: причина «не работает» — `ActService::update` физически не существовал в бэкенде (подтверждено grep по кодовой базе), а не баг в существующей логике.
Plans:
**Wave 1**

- [x] 19-01-PLAN.md — ACT-01: handover_date_utc в ActDto + переключение 5 read-сайтов (сортировка, PDF, список, карточка)
- [x] 19-06-PLAN.md — Gap CR-01 (BLOCKER): update() пересчитывает archived (recompute_parent_archived, gated) + 2 regression-теста
- [x] 19-08-PLAN.md — Gap WR-02/IN-01: edit-режим форсирует single-device rows + todayISO() на UTC

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 19-02-PLAN.md — ACT-02 контракты: ActPatch, ActUpdateDto/ActUpdateItemDto, update_act_header_in_tx, select_latest_device_mutation
- [x] 19-07-PLAN.md — Gap WR-01/WR-03: каскад номера на return-акты + аудит изменения комплектации

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 19-03-PLAN.md — ACT-02 backend: ActService::update (CAS, D-05/D-06/D-07/D-08, номер-уникальность) + 9 интеграционных тестов

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 19-04-PLAN.md — ACT-02 транспорты: Tauri-команда + HTTP-хендлер + RBAC-тест + frontend-клиент + bindings

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 19-05-PLAN.md — ACT-02 UI: ActFormBody/ActFormModal edit-режим, ActDetail D-07-гейтинг, ActsPage-оркестрация

---

### Phase 20: Печать актов и организация

**Goal**: Печать акта приёма-передачи из раздела Устройства выводит полный организационный контекст в шапке, а настройки организации поддерживают безопасный SVG-логотип и вторую строку адреса, которые попадают в печатные шаблоны.

**Depends on**: Phase 17 (шаблоны/HTML-печать зафиксированы в v1.1.1)

**Milestone**: v1.1.2 — Пост-релизные доработки UX и печати

**Requirements**: PRN-01, ORG-01, ORG-02

**Success Criteria** (what must be TRUE):

1. При печати акта приёма-передачи из раздела Устройства в шапку документа попадает полный контекст организации (логотип, название, ИНН и прочие реквизиты), а не только данные устройства и сторон «сдал/принял».
2. Пользователь может загрузить логотип организации в формате SVG через настройки; логотип отображается в печатных шаблонах.
3. Загруженный SVG-логотип встраивается безопасно — санитизируется или вставляется только через `<img src="data:...">`, так что встроенный `<script>` не исполняется ни в превью, ни при печати.
4. В настройках организации есть поле «Вторая строка адреса»; заполненное значение отображается отдельной строкой в печатных формах.

**Plans**: 6 plans in 3 waves
**UI hint**: yes

**Wave 1**

- [x] 20-01-PLAN.md — Контракты: миграция V035 (address_line2), OrgPatch/OrgSettingsDto, 3 SQL-сайта org_db_service.rs, компиляция всего крейта

**Wave 2** *(depends on Wave 1 — параллельны друг другу, нет пересечения файлов)*

- [x] 20-02-PLAN.md — PRN-01: render_acceptance_pdf на org_db.get_for_pdf() (D-02/D-03/D-11), address_line2 в ctx render_pdf/report_service
- [x] 20-03-PLAN.md — Шаблоны: act_acceptance.html паритет с act_handover.html (D-01), address_line2 во всех трёх шаблонах (D-06)
- [x] 20-04-PLAN.md — Настройки: поле «Адрес (2-я строка)» в OrgSettings.svelte (D-05), регенерация bindings.ts

**Wave 3** *(depends on Wave 2 — нужны и render-фикс, и шаблоны)*

- [x] 20-05-PLAN.md — Regression-тесты: PRN-01 паритет acceptance/handover, ORG-01 SVG-`<script>` img-only (D-09), ORG-02 address_line2 в report.html
- [x] 20-06-PLAN.md — Auto-upgrade-untouched-defaults (D-12): доводит правки 20-03 до УЖЕ существующих установок (не только fresh installs), regression-тест на пред-материализованный старый файл

---

### Phase 21: Точечные фиксы — коды картриджей/фотобарабанов

**Goal**: Автоматически присваиваемые коды новых картриджей и фотобарабанов используют укороченный, согласованный формат.

**Depends on**: Nothing (независима от Phase 18–20)

**Milestone**: v1.1.2 — Пост-релизные доработки UX и печати

**Requirements**: CRT-01

**Success Criteria** (what must be TRUE):

1. Новый картридж, создаваемый без явно указанного кода, получает автокод в формате `C-XXXX` (4 цифры).
2. Новый фотобарабан, создаваемый без явно указанного кода, получает автокод в формате `D-XXXX` (4 цифры).

**Plans**: 1 plan

Plans:
- [x] 21-01-PLAN.md — Сократить формат автокода картриджей/фотобарабанов до 4 цифр (C-XXXX/D-XXXX)

---

### Phase 22: Правка возвратов

**Goal**: Существующий return-акт можно открыть в рабочей форме (диалог «Возврат по акту №XXX») с теми же значениями, что были на момент оформления возврата, и сохранить изменённый возврат без ошибок — с корректной пересборкой эффектов на устройства по дельте.

**Depends on**: Phase 19

**Milestone**: v1.1.2 — Пост-релизные доработки UX и печати

**Requirements**: ACT-03

**Success Criteria** (what must be TRUE):

1. Кнопка «Редактировать» на карточке возврата активна (не задизейблена, не скрыта).
2. Нажатие «Редактировать» открывает диалог «Возврат по акту №XXX», предзаполненный теми же значениями (состав возвращаемых устройств, состояние, дата, кто), что были на момент оформления возврата.
3. Изменения возврата сохраняются без ошибок; эффекты на устройства (статус/локация/история) пересобираются по дельте, а derived-флаги (в т.ч. `archived` родительского акта) остаются согласованными.

**Plans**: 6 plans in 6 waves
**UI hint**: yes

**Wave 1**

- [x] 22-01-PLAN.md — Контракты: ActUpdateReturnDto + ActReturnDto/ActItemDto extend, select_latest_device_mutation_pair, V034 backfill миграция

**Wave 2** *(depends on Wave 1)*

- [x] 22-02-PLAN.md — Backend: do_return giver/receiver/date фикс (Pitfall 1, D-05/D-12) + ActService::update_return (D-09/D-10/D-11) + 11 интеграционных тестов

**Wave 3** *(depends on Wave 2)*

- [x] 22-03-PLAN.md — Транспорты: Tauri-команда + HTTP-хендлер + RBAC-тест + frontend-клиент + bindings

**Wave 4** *(depends on Wave 3)*

- [x] 22-04-PLAN.md — UI: ReturnModal edit-режим (dual prefill, дата возврата, ФИО без swap) + ActDetail/ActsPage-оркестрация

**Wave 5** *(gap closure, depends on Wave 4 — code-review blockers)*

- [x] 22-05-PLAN.md — CR-01 (retained/added loops NULL location data loss) + CR-02 (un-return restores wrong post-edit snapshot) + 4 регрессионных теста

**Wave 6** *(gap closure, depends on Wave 5 — code-review warnings)*

- [x] 22-06-PLAN.md — WR-01 (validate_update_return parity) + WR-02 (panic→domain error) + WR-03 (over-return qty guard) + WR-04 (V034 comment fix) + IN-01 (baseline comment) + 4 регрессионных теста

**Note**: Отменяет D-07 (Фаза 19) в части «return-акты нередактируемы». Семантика — полная правка возврата (выбрана пользователем 2026-07-12): можно менять состав/состояние/дату, backend пересобирает эффекты по дельте (как правка handover-акта). Источник: 19-CONTEXT.md → Deferred Ideas. Waves 5-6 добавлены после `22-REVIEW.md` (2026-07-12, standard-depth code review нашёл 2 BLOCKER + 4 WARNING + 1 INFO дефекта delta-движка update_return).

---

### Phase 23: Токены и основы дизайн-системы

**Goal**: Интерфейс переходит на единый слой токенов `--tr-*` (поверхности, 5 уровней текста, акцент с hover/active/soft, семантика с парами -soft/-text, нейтральная шкала n-0…n-950, 5 уровней теней), типографика следует новой шкале из 9 уровней, а отступы/радиусы мигрированы **по значению** — вёрстка не сдвигается. Попутно устранены 2 известных бага неопределённых токенов.

**Depends on**: Phase 22 (предыдущий milestone, завершён)

**Milestone**: v1.2 — Редизайн UI и дизайн-система

**Requirements**: DS-01, DS-02, DS-03, DS-04, QA-01

**Key context**: 105/118 svelte-файлов уже ходят через токены — смена значений разойдётся по UI сама. Мигрировать `--space-*`/`--radius-*`/`--font-size-*` нужно по карте соответствия значений (см. REQUIREMENTS.md), а не по имени шкалы — иначе 642 использования `--space-*` тихо поедут без единой ошибки сборки.

**Success Criteria** (what must be TRUE):

1. Все компоненты читают цвета через `--tr-*` custom properties; захардкоженных hex (~40 найденных) в стилях компонентов не остаётся.
2. Переключение светлой/тёмной темы не показывает визуальных артефактов (вспышка, нестилизованные поверхности, нечитаемый текст) ни на одном экране.
3. Текст рендерится по одному из 9 уровней новой типографической шкалы; инвентарные/серийные номера и номера актов отображаются моноширинным шрифтом.
4. Отступы и радиусы мигрированы по значению согласно карте соответствия — сравнение экрана до/после не показывает сдвига вёрстки.
5. `--font-size-sm` (PersonAutocomplete.svelte) и `--radius-lg` (LoginPage/BlockedScreen/FirstRunWizard) резолвятся в определённое значение — без fallback на браузерный дефолт.

**Plans**: 6 plans in 5 waves
**UI hint**: yes

**Wave 1**

- [x] 23-01-PLAN.md — Токен-слой: _tokens.scss полностью переписан в --tr-* (D-01/D-02/D-03/D-05/D-10/D-12/D-14), global.scss мигрирован + .tr-mono
- [x] 23-02-PLAN.md — Гейт-скрипты: check-tokens.mjs (D-04) + verify-value-map.mjs (D-08) + D-15 eslint-фикс (5 pre-existing ошибок)

**Wave 2** *(depends on Wave 1)*

- [x] 23-03-PLAN.md — Цвет + элевация по роли: sweep --color-*/--shadow-* (DS-01), инверсия поверхностей (D-11), hardcoded hex, --shadow-md bug fix (D-17)

**Wave 3** *(depends on Wave 2 — тот же набор файлов, что и цвет)*

- [x] 23-04-PLAN.md — Space + radius по значению: sweep --space-*/--radius-* (DS-04), split --radius-sm (D-07), --radius-lg QA-01 fix, verify-value-map.mjs

**Wave 4** *(depends on Wave 3 — тот же набор файлов)*

- [ ] 23-05-PLAN.md — Типографика + .tr-mono охват: sweep --font-size-*/--font-weight-*/--line-height-* (DS-03), --font-size-sm QA-01 fix, .tr-mono на 9 in-scope сайтах (D-13/D-16)

**Wave 5** *(depends on все предыдущие — финальный гейт)*

- [ ] 23-06-PLAN.md — Финальная верификация: check-tokens.mjs + verify-value-map.mjs полный прогон, pnpm lint/svelte-check зелёные, hand-off чек-лист для UAT (D-09)

---

### Phase 24: Базовые компоненты

**Goal**: Пять базовых примитивов (Button, поля ввода, бейджи, вкладки, модальное окно) отражают новую дизайн-систему, так что всё, что их переиспользует, автоматически наследует новый визуальный язык.

**Depends on**: Phase 23

**Milestone**: v1.2 — Редизайн UI и дизайн-система

**Requirements**: CMP-01, CMP-02, CMP-03, CMP-04, CMP-05

**Success Criteria** (what must be TRUE):

1. Каждый из 5 вариантов кнопки в обоих размерах визуально различим в состояниях наведение/фокус/нажатие/отключено/загрузка.
2. Input/Select/Textarea/Checkbox визуально различимы в состояниях обычное/фокус/ошибка/отключено в новом визуальном языке.
3. Бейджи-статусы рендерятся в 4 тонах в вариантах мягкая подложка/сплошной/с точкой/счётчик-пилюля.
4. Вкладки switch-bar показывают счётчики и подчёркивание активной вкладки.
5. Модальное окно показывает оверлей + шапку + тело + футер действий с тенью уровня 3 и радиусом 12px.

**Plans**: TBD
**UI hint**: yes

---

### Phase 25: Таблицы и Dropdown

**Goal**: Строки таблицы и новый компонент Dropdown/комбобокс отражают дизайн-систему, сохраняя плотный список и групповой UX, на которые опирается приложение.

**Depends on**: Phase 24

**Milestone**: v1.2 — Редизайн UI и дизайн-система

**Requirements**: CMP-06, CMP-07

**Success Criteria** (what must be TRUE):

1. Строки таблицы визуально различимы в состояниях обычная/наведение/выбрана.
2. Строка-группа сворачивается/разворачивается, показывая счётчик-пилюлю и вложенные устройства при раскрытии.
3. Dropdown корректно отображает плоский список.
4. Dropdown корректно отображает список с группами (заголовки секций).
5. Существующее portal/anchor-позиционирование (Фаза 18) продолжает работать без регрессий с новым визуалом.

**Plans**: TBD
**UI hint**: yes

---

### Phase 26: Окна с готовым макетом

**Goal**: Два окна, для которых в Claude Design есть готовый макет (Дашборд, Список устройств), реализованы с точным визуальным соответствием этому макету.

**Depends on**: Phase 25

**Milestone**: v1.2 — Редизайн UI и дизайн-система

**Requirements**: WIN-01, WIN-02

**Success Criteria** (what must be TRUE):

1. Дашборд визуально соответствует макету Claude Design (виджеты, отступы, тональность).
2. Список устройств визуально соответствует макету Claude Design, включая групповые строки.
3. Оба окна сохраняют всю существующую функциональность (фильтры, автокомплиты, CRUD, CSV import/export) без изменений поведения.

**Plans**: TBD
**UI hint**: yes

---

### Phase 27: Окна основного рабочего процесса

**Goal**: Три ключевых транзакционных окна (Акты, Картриджи, Принтеры) переходят на новую дизайн-систему, несмотря на отсутствие готового макета — раскладка выводится из компонентной системы, построенной в фазах 24–25.

**Depends on**: Phase 26

**Milestone**: v1.2 — Редизайн UI и дизайн-система

**Requirements**: WIN-03, WIN-04, WIN-05

**Success Criteria** (what must be TRUE):

1. Окна Актов (список, деталь/редактирование, диалог возврата, вызов печати) используют новые токены/компоненты повсеместно — без остатков старых классов.
2. Окна Картриджей (модели, экземпляры, действия жизненного цикла) используют новые токены/компоненты повсеместно.
3. Окна Принтеров (список, деталь, агрегаты совместимости) используют новые токены/компоненты повсеместно.
4. Каждое существующее поле/действие/workflow в этих окнах остаётся на месте и работает (изменение чисто визуальное).

**Plans**: TBD
**UI hint**: yes

---

### Phase 28: Окна поддержки и администрирования

**Goal**: Окна поддержки и администрирования (Заявки, Отчёты, Настройки, Пользователи) переходят на новую дизайн-систему.

**Depends on**: Phase 27

**Milestone**: v1.2 — Редизайн UI и дизайн-система

**Requirements**: WIN-06, WIN-07, WIN-08, WIN-09

**Success Criteria** (what must be TRUE):

1. Окно Заявок (список, деталь, селекты категории/принтера) использует новые токены/компоненты.
2. Окно Отчётов использует новые токены/компоненты.
3. Окно Настроек (организация, шаблоны, бэкапы, вкладка AD) использует новые токены/компоненты.
4. Окно Пользователей использует новые токены/компоненты.

**Plans**: TBD
**UI hint**: yes

---

### Phase 29: Вход и интерфейс сотрудника

**Goal**: Экраны входа и отдельная оболочка для роли «Сотрудник» показывают тот же визуальный язык, что и основное приложение, несмотря на отдельный layout-shell.

**Depends on**: Phase 28

**Milestone**: v1.2 — Редизайн UI и дизайн-система

**Requirements**: WIN-10, WIN-11

**Success Criteria** (what must be TRUE):

1. Экраны Логин/Pending/Blocked/FirstRunWizard используют новые токены/компоненты — без артефактов неопределённых токенов.
2. EmployeeLayout (сайдбар, форма заявки, список собственных заявок) использует новые токены/компоненты.
3. Визуальный язык соответствует переработанному основному приложению, несмотря на отдельную оболочку.

**Plans**: TBD
**UI hint**: yes

---

### Phase 30: Качество — доступность и паритет платформ

**Goal**: Переработанный интерфейс проходит планку доступности и паритета между платформами на всех окнах, затронутых фазами 23–29.

**Depends on**: Phase 29

**Milestone**: v1.2 — Редизайн UI и дизайн-система

**Requirements**: QA-02, QA-03

**Success Criteria** (what must be TRUE):

1. Контраст текст/фон соответствует WCAG AA в обеих темах на всех переработанных окнах.
2. Каждый интерактивный элемент (кнопка, поле, ссылка, строка таблицы, вкладка) показывает видимое кольцо фокуса при навигации клавиатурой.
3. Десктоп (Tauri WebView) и LAN-браузер визуально идентичны на репрезентативной выборке окон (дашборд, устройства, акты, логин).

**Plans**: TBD
**UI hint**: yes

---

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Фундамент | v1.0 | 6/6 | Complete | 2026-05-25 |
| 2. Устройства и базовый UI | v1.0 | 5/5 | Complete | 2026-05-28 |
| 3. Акты + PDF | v1.0 | 5/5 | Complete | 2026-05-30 |
| 03.1. Acts quantity model | v1.0 | 6/6 | Complete | 2026-06-05 |
| 03.2. Deferred UAT gap closure | v1.0 | 2/2 | Complete | 2026-06-06 |
| 03.3. Device-list UX round 2 | v1.0 | 2/2 | Complete | 2026-06-07 |
| 4. Картриджи | v1.0 | 6/6 | Complete | 2026-06-12 |
| 5. Авторизация и серверный режим | v1.0 | 6/6 | Complete | 2026-06-14 |
| 6. Принтеры (SNMP) и Заявки | v1.0 | 9/9 | Complete | 2026-06-15 |
| 7. Отчёты, Дашборд и Настройки | v1.0 | 14/14 | Complete | 2026-06-18 |
| 8. Релизный пайплайн | v1.0 | 2/2 | Complete | 2026-06-19 |
| 9. AD-аутентификация | v1.1 | 5/5 | Complete | 2026-06-20 |
| 10. Роль employee + role-gating | v1.1 | 4/4 | Complete | 2026-06-21 |
| 11. Заявки/employee UX | v1.1 | 3/3 | Complete | 2026-06-22 |
| 12. Взаимосвязь картриджной заявки | v1.1 | 21/21 | Complete | 2026-06-25 |
| 13. Редизайн совместимости | v1.1 | 8/8 | Complete | 2026-06-26 |
| 14. Данные и структура акта | v1.1.1 | 3/3 | Complete    | 2026-07-03 |
| 15. Рендер и соответствие образцу | v1.1.1 | 4/4 | Complete   | 2026-07-04 |
| 16. Документы через HTML-печать | v1.1.1 | 5/5 | Complete    | 2026-07-05 |
| 17. Отчёты и Шаблоны через HTML-печать | v1.1.1 | 7/7 | Complete    | 2026-07-07 |
| 18. Автокомплит и дропдауны | v1.1.2 | 5/5 | Complete    | 2026-07-11 |
| 19. Акты — дата и редактирование | v1.1.2 | 10/10 | Complete   | 2026-07-11 |
| 20. Печать актов и организация | v1.1.2 | 6/6 | Complete    | 2026-07-14 |
| 21. Точечные фиксы — коды картриджей | v1.1.2 | 1/1 | Complete    | 2026-07-14 |
| 22. Правка возвратов | v1.1.2 | 6/6 | Complete    | 2026-07-13 |
| 23. Токены и основы дизайн-системы | v1.2 | 4/6 | In Progress|  |
| 24. Базовые компоненты | v1.2 | 0/TBD | Not started | - |
| 25. Таблицы и Dropdown | v1.2 | 0/TBD | Not started | - |
| 26. Окна с готовым макетом | v1.2 | 0/TBD | Not started | - |
| 27. Окна основного рабочего процесса | v1.2 | 0/TBD | Not started | - |
| 28. Окна поддержки и администрирования | v1.2 | 0/TBD | Not started | - |
| 29. Вход и интерфейс сотрудника | v1.2 | 0/TBD | Not started | - |
| 30. Качество — доступность и паритет платформ | v1.2 | 0/TBD | Not started | - |

## Coverage

- **v1 requirements mapped:** 120 / 120 ✓ (см. `milestones/v1.1-REQUIREMENTS.md`)
- **v1.1.1 requirements mapped:** 8 / 8 ✓ (см. REQUIREMENTS.md — Traceability)
- **v1.2 requirements mapped:** 25 / 25 ✓ (см. REQUIREMENTS.md — Traceability; roadmap: Phases 23–30 below)
- **Orphans:** none

## v1.1.1 Requirement Coverage

| Requirement | Phase | Status |
|--------------|-------|--------|
| PDFA-01 | Phase 15 | Complete |
| PDFA-02 | Phase 15 | Complete |
| PDFA-03 | Phase 14 | Complete |
| PDFA-04 | Phase 14 | Complete |
| PDFA-05 | Phase 15 | Complete |
| PDFA-06 | Phase 14 | Complete |
| PDFA-07 | Phase 15 | Complete |
| PDFA-08 | Phase 15 | Complete |

## Out of v1.1.1 Roadmap (Deferred)

| Category | Reason |
|----------|--------|
| DOC-01 (Редактор шаблонов WYSIWYG) | Существующий `document_templates` + textarea покрывает потребность v1.1.1; отдельная работа в будущем milestone |
| DOC-02 (Импорт/экспорт .docx) | Образец разобран вручную; парсинг .docx не нужен для генерации |
| DOC-03 (Прочие виды документов) | Milestone фокусируется только на акте приёма-передачи/выдачи |

## Out of v1 Roadmap (Deferred to v2)

| Category | Reason |
|----------|--------|
| MAP-01..04 (Карта помещений) | Высокая UI-сложность; ценность учёта не зависит от карты — отложено в v2 milestone |
| NTF-02 (SMTP), NTF-03 (Telegram), NTF-04 (Webhook), NTF-05 (event subscriptions) | In-app часть покрыта REQ-04 в Phase 6; внешние каналы — финальная фаза v2 |
| PNT-01..04 (Pantum auto-restart) | В v1 — только детекция и алерт (PRN-06); авто-restart требует подтверждённой гипотезы и безопасного механизма (v2) |
| WIN7-01..02 (Windows 7 32-bit) | Best-effort; MSRV `krilla` 1.92 + WebView2 TLS 1.2 могут закрыть дверь — отдельный spike в v2 |
| I18N-01..03 (Английская локализация) | Команда и пользователи русскоязычные; добавляется без архитектурных переделок |
| ADV-01..05 (SSO/REST API наружу/Signature pad/доп. вендоры принтеров/Postgres) | Преждевременная сложность для текущего масштаба |

### Phase 17: Отчёты и Шаблоны через HTML-печать (убрать krilla, как в актах)

**Goal**: Экспорт Отчётов и редактор Шаблонов переходят с krilla/DocSpec на HTML-печать по паттерну Phase 16 (акты): `export_pdf` возвращает HTML-строку, печать/сохранение идёт через диалог браузера в превью-модалке (desktop + LAN), редактор Шаблонов правит HTML-файлы в `templates/`, и `krilla`/`DocSpec` полностью выведены из активного пути (заморожены, не удалены).

**Depends on**: Phase 16 (контекст акта + HTML-печать актов зафиксированы)

**Milestone**: v1.1.1 — Документы через HTML-печать

**Spec**: `17-SPEC.md` (ambiguity 0.15)

**Success Criteria** (what must be TRUE): см. `17-SPEC.md` — Acceptance Criteria.

**Plans**: 7 plans in 6 waves (17-05..17-07 added 2026-07-07 as gap-closure after verification found gaps — см. 17-VERIFICATION.md)

**Wave 1**

- [x] 17-01-PLAN.md — Reports backend: templates/report.html + html_templates.rs registration, ReportService::export_pdf → HTML String (wire OrganizationService), Tauri/HTTP adapters text/html

**Wave 2** *(depends on Wave 1 — shares context.rs construction block)*

- [x] 17-02-PLAN.md — TemplateService retarget: list_all_for_editor/update_body/reset_to_default/validate_preview → file I/O on templates/*.html (wire OrganizationService), Tauri/HTTP kind passthrough

**Wave 3** *(depends on Wave 1 + Wave 2)*

- [x] 17-04-PLAN.md — Backend tests: html_report_render.rs (1/N rows, month grouping, empty), template_edit.rs rewritten for file-backed contract, krilla #[ignore] hygiene sweep + struct-field doc-comments

**Wave 4** *(depends on Wave 3 — merged Rust tree guaranteed before bindings regeneration)*

- [x] 17-03-PLAN.md — Frontend: PdfPreviewModal mode='report', ReportsPage export/print via modal, TemplateEditor retargeted to file-backed HTML editor, bindings.ts regenerated

**Wave 5 (gap-closure)** *(independent of each other — no file overlap)*

- [x] 17-05-PLAN.md — BLOCKER: column_labels_for(report_type) — русские подписи колонок вместо сырых ключей (D-03/CR-01); WR-05 logo mime allowlist enforcement; regression tests
- [x] 17-06-PLAN.md — WR-01: update_body валидирует тем же строгим build_safe_html_env, что и реальный рендер; WR-03: sandbox на preview-iframe (PdfPreviewModal.svelte, TemplateEditor.svelte) + checkpoint визуальной проверки

**Wave 6 (gap-closure)** *(depends on Wave 5 — confirms full-suite green after fixes)*

- [x] 17-07-PLAN.md — Req-7: воспроизвести/подтвердить cargo test -p trackly-app зелёный под задокументированным корректным вызовом (mock env vars + --test-threads=1 + prebuilt ui/dist); документировать в devices_csv_import.rs

## Phase 17 Requirement Coverage

| Requirement | Plan |
|--------------|------|
| Req-1 | 17-01, 17-05 |
| Req-2 | 17-01 |
| Req-3 | 17-01 |
| Req-4 | 17-03, 17-06 |
| Req-5 | 17-02, 17-03, 17-06 |
| Req-6 | 17-01, 17-02, 17-04 |
| Req-7 | 17-04, 17-07 |

## v1.1.2 Requirement Coverage

| Requirement | Phase | Status |
|-------------|-------|--------|
| AUTO-01 | Phase 18 | Complete |
| AUTO-02 | Phase 18 | Complete |
| AUTO-03 | Phase 18 | Complete |
| AUTO-04 | Phase 18 | Complete |
| AUTO-05 | Phase 18 | Complete |
| ACT-01 | Phase 19 | Complete |
| ACT-02 | Phase 19 | Complete |
| ACT-03 | Phase 22 | Complete |
| PRN-01 | Phase 20 | Complete |
| ORG-01 | Phase 20 | Complete |
| ORG-02 | Phase 20 | Complete |
| CRT-01 | Phase 21 | Complete |

**Coverage:** 12/12 v1.1.2 requirements satisfied ✓ — no orphans.

## v1.2 Requirement Coverage

| Requirement | Phase | Status |
|--------------|-------|--------|
| DS-01 | Phase 23 | Pending |
| DS-02 | Phase 23 | Pending |
| DS-03 | Phase 23 | Pending |
| DS-04 | Phase 23 | Pending |
| QA-01 | Phase 23 | Pending |
| CMP-01 | Phase 24 | Pending |
| CMP-02 | Phase 24 | Pending |
| CMP-03 | Phase 24 | Pending |
| CMP-04 | Phase 24 | Pending |
| CMP-05 | Phase 24 | Pending |
| CMP-06 | Phase 25 | Pending |
| CMP-07 | Phase 25 | Pending |
| WIN-01 | Phase 26 | Pending |
| WIN-02 | Phase 26 | Pending |
| WIN-03 | Phase 27 | Pending |
| WIN-04 | Phase 27 | Pending |
| WIN-05 | Phase 27 | Pending |
| WIN-06 | Phase 28 | Pending |
| WIN-07 | Phase 28 | Pending |
| WIN-08 | Phase 28 | Pending |
| WIN-09 | Phase 28 | Pending |
| WIN-10 | Phase 29 | Pending |
| WIN-11 | Phase 29 | Pending |
| QA-02 | Phase 30 | Pending |
| QA-03 | Phase 30 | Pending |

**Coverage:** 25/25 v1.2 requirements mapped ✓ — no orphans.

---
*Last updated: 2026-07-15 — v1.1.2 shipped (Phases 18–22, 28 plans). All 12 requirements Complete. Full detail archived to `milestones/v1.1.2-ROADMAP.md`.*
*Last updated: 2026-07-16 — v1.2 roadmap created (Phases 23–30, 8 phases, standard granularity, 25/25 requirements mapped, 0 plans yet). Ready for `/gsd-plan-phase 23`.*
