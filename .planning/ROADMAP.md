# Roadmap: Trackly

Trackly — портативное приложение для учёта техники, принтеров и картриджей с серверным
режимом для LAN-доступа. Релизная линия v1 завершена (v1.0 + v1.1). Полные детали фаз
заархивированы в `.planning/milestones/`.

## Milestones

- ✅ **v1.0 — Базовый учёт** — Phases 1–8 (shipped 2026-06-19) → `milestones/v1.1-ROADMAP.md`
- ✅ **v1.1 — AD, сотрудники и картриджная взаимосвязь** — Phases 9–13 (shipped 2026-06-26) → `milestones/v1.1-ROADMAP.md`
- ✅ **v1.1.1 — PDF-акт по образцу Word (мультиустройство)** — Phases 14–15 (completed 2026-07-04)
- 🚧 **v1.2 — Документы через HTML-печать** — Phases 16-17 (active)

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

**v1.1.1 — PDF-акт по образцу Word (мультиустройство) — ACTIVE**

- [x] **Phase 14: Данные и структура акта** - Схема/контекст акта содержит все поля образца (реквизиты, комплектация, тех.характеристики, срок до, мультиустройство, двухстрочные подписи) и достижимы через существующий механизм `document_templates`. (completed 2026-07-03)
- [x] **Phase 15: Рендер и соответствие образцу** - Дефолтный шаблон и рендерер производят PDF, визуально соответствующий образцу Word, с мультиустройством и regression-тестами. (completed 2026-07-04)

**v1.2 — Документы через HTML-печать — ACTIVE**

- [x] **Phase 16: Документы через HTML-печать** - Оба акта (приёма-передачи и приёмки устройства) генерируются из HTML-шаблонов (папка `templates/` рядом с exe + вшитый дефолт-fallback) и печатаются/сохраняются в PDF через диалог браузера в обоих режимах (desktop + LAN), визуально по образцу Word; krilla/DocSpec заморожен и не используется. (SPEC: 16-SPEC.md) (completed 2026-07-05)
- [x] **Phase 17: Отчёты и Шаблоны через HTML-печать** - Отчёты и редактор Шаблонов переходят на HTML-печать по паттерну Phase 16; krilla/DocSpec выведены из активного пути (заморожены, не удалены). (SPEC: 17-SPEC.md) (plans 7/7; gap-closure 17-05..17-07 planned 2026-07-07 — см. 17-VERIFICATION.md) (completed 2026-07-07)

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

**Milestone**: v1.2 — Документы через HTML-печать

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
| 16. Документы через HTML-печать | v1.2 | 5/5 | Complete    | 2026-07-05 |
| 17. Отчёты и Шаблоны через HTML-печать | v1.2 | 7/7 | Complete   | 2026-07-07 |

## Coverage

- **v1 requirements mapped:** 120 / 120 ✓ (см. `milestones/v1.1-REQUIREMENTS.md`)
- **v1.1.1 requirements mapped:** 8 / 8 ✓ (см. REQUIREMENTS.md — Traceability)
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

**Milestone**: v1.2 — Документы через HTML-печать

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

---
*Last updated: 2026-07-07 — Phase 17 gap-closure planned: 3 additional plans (17-05..17-07) across 2 waves, closing 17-VERIFICATION.md's blocker (D-03/CR-01) + 3 warnings (WR-01, WR-03, WR-05) + Req-7 full-suite-green uncertainty.*
