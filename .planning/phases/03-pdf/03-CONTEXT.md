# Phase 3: Акты приёма-передачи и первая PDF-печать - Context

**Gathered:** 2026-05-28
**Status:** Ready for planning
**Source:** /gsd-discuss-phase 3 — 4 области (нумерация/lifecycle, UX актов, PDF-стек, шаблоны+org) обсуждены интерактивно

<domain>
## Phase Boundary

Поставить ключевую дифференцирующую ценность продукта: акты приёма-передачи (ACT-01..14) с auto-нумерацией из counter table, полные/частичные возвраты с под-нумерацией «42в»/«42в1»/«42в2», авто-архив при 100% возврате, undo возврата с восстановлением состояния, **плюс** первую инфраструктуру PDF-печати с кириллицей через embedded DejaVu Sans + MiniJinja-шаблоны (kinds `act_handover`, `act_acceptance`), которая закрывает DEV-14/DEV-15 (документ приёма) и переиспользуется в Phase 6 (печать заявок) и Phase 7 (отчёты).

**В scope:**
- ACT-01..14 целиком (acts CRUD, switch-bar Акты/Возвраты/Архив, return lifecycle, undo, поиск).
- DEV-14 (печать Документа приёма устройства на склад) и DEV-15 (его шаблон в БД).
- PDF-инфраструктура: krilla 0.7 + DejaVu Sans embedded + MiniJinja safe-mode + DocSpec-IR + CI hash-test.
- Org-data источник для шапки PDF (`org.json` рядом с .exe).
- UI master-detail для актов, широкий модал создания, retunr-модал с bulk-default + per-row override, preview-модал с PDF.js.

**НЕ в scope этой фазы (явно deferred):**
- UI редактор шаблонов → Phase 7 (ROADMAP success criterion 4 явно ссылается на Phase 7).
- Полноценная страница Настройки/Организация → Phase 7 (Phase 3 читает `org.json`, UI редактирования нет — изменения вручную в файле).
- Login / RBAC → Phase 5; в Phase 3 `audit_log.user_id` = NULL.
- Картриджи → Phase 4; Принтеры → Phase 6.
- Server-mode HTTP-handlers — axum router строится (как в Phase 2), но не bind'ится; preview-модал и save-через-tauri-plugin-dialog — desktop-only в Phase 3.

**Mode:** mvp — вертикальный слайс на каждый план: UI → command → service → repo → DB (+ PDF renderer).

</domain>

<decisions>
## Implementation Decisions

### Acts: нумерация и lifecycle возвратов

#### D-Numbering-01: суффикс «в» — display-rule на чтении, в БД только sub_number 1, 2, 3, ...
- **Schema invariant (V004):** `acts.sub_number INTEGER NULL` — NULL для handover. Все returns имеют sub_number = 1, 2, 3, ... возрастающий per parent_act_id.
- **Display rule (в сервисе/DTO):** при чтении `format_act_number(act, return_count, all_items_returned_in_single_return)`:
  - handover: `format!("{}", number)` → «42»
  - return AND это единственный return AND он закрыл все позиции (т.е. handover.archived = 1 от этого же return): `format!("{}в", parent.number)` → «42в»
  - иначе return: `format!("{}в{}", parent.number, sub_number)` → «42в1», «42в2», «42в3»
- **Ретроактивное переименование:** если первый возврат отображался как «42в», а потом появился второй — оба автоматически становятся «42в1» и «42в2». Это поведение приемлемо (пользователь принял).
- **PDF stable suffix:** при печати акта возврата суффикс зашивается в сгенерированный PDF (это снимок на момент). Если потом появится новый возврат — старый PDF остаётся со «старым» суффиксом, это OK (он уже выдан на руки).
- **Rationale:** Схема V004 (`COALESCE(sub_number, 0)` в UNIQUE индексе) не разрешает sub_number=NULL для return-актов одновременно с handover. Поэтому всегда нумеруем; «в» — это форматирование.

#### D-Counter-Acts-01: act_number — атомарный `UPDATE ... RETURNING` под `BEGIN IMMEDIATE` в writer task
- Использовать существующую строку `counters.name = 'act_number'` (V009 уже seed'нута).
- Pattern: внутри одной writer-job транзакции:
  ```sql
  BEGIN IMMEDIATE;
  UPDATE counters SET current_value = current_value + 1 WHERE name='act_number' RETURNING current_value;
  INSERT INTO acts(number, sub_number, ...) VALUES (?, NULL, ...);
  INSERT INTO act_items(...) VALUES (...), (...), ...;
  -- update devices.status='В работе' for each item
  -- INSERT INTO audit_log per change
  COMMIT;
  ```
- **Override номера:** если пользователь ввёл свой номер вручную в UI — `INSERT INTO acts(number, ...)` использует его, counter НЕ инкрементируется; пишется отдельная запись в `audit_log` с `action='custom:act_number_override'`, `payload_json={"requested": N, "next_auto_would_be": M}`. Уникальный конфликт по `(number, COALESCE(sub_number,0))` ловится и переводится в `AppError::Conflict { field:"number", message:"Акт №X уже существует" }`.
- **sub_number для returns:** считается на основе `SELECT MAX(sub_number) FROM acts WHERE parent_act_id = ? AND deleted_at_utc IS NULL` + 1 в той же транзакции возврата (БД блокировка `BEGIN IMMEDIATE` гарантирует отсутствие гонок).
- **Rationale:** ACT-14 явно требует thread-safety; single-writer + BEGIN IMMEDIATE даёт абсолютную гарантию без распределённых блокировок.

#### D-Archive-01: автоархив на 100% возврате, без ручного флага
- Сервис в той же транзакции возврата считает «остатки в работе» по handover-акту:
  ```sql
  SELECT SUM(ai.quantity_left) FROM act_items ai
   JOIN devices d ON d.id = ai.device_id
   WHERE ai.act_id = ? AND d.status_id = (SELECT id FROM device_statuses WHERE code='в_работе')
  ```
  (Точная форма зависит от того, как Phase 3 моделирует «сколько вернулось»; ключевой вход — сервис должен надёжно ответить «остаток = 0 → archived=1»).
- При нулевом остатке: `UPDATE acts SET archived=1, updated_at_utc=?, version=version+1 WHERE id=?` на handover.
- При delete return-акта (undo): аналогичный пересчёт; если остаток > 0 → `archived=0`.
- **Ручной кнопки «В архив» НЕТ.** Это derived state.
- **Rationale:** REQ-ACT-09 однозначен; ручной флаг создаёт inconsistencies (архив + остаток в работе).

#### D-Undo-01: восстановление при удалении return-акта — из `audit_log.before_json`
- Каждая mutation device (status_id, location_id, condition, complectation) при возврате пишет `audit_log` row с `entity_type='device'`, `entity_id=device.id`, `action='update'`, `before_json={...полный row до...}`, `after_json={...полный row после...}`, `payload_json={"act_id": N, "kind":"return"}`.
- Undo (delete return-акта) — single transaction:
  1. Находим все audit_log записи с `payload_json.act_id = ?` для этого return-акта.
  2. Для каждой записи `UPDATE devices SET ...before_json WHERE id = entity_id` (восстановление).
  3. Пишем `audit_log` запись с `action='custom:undo_return'`, `before_json/after_json` отражают factual reverse.
  4. Soft-delete return-акт и его act_items (или hard-delete CASCADE — см. D-Soft-vs-Hard).
- Симметрично для delete handover-акта (REQ-ACT-06): восстановление device.status/location из audit_log entries за этот handover.
- **act_items.condition_at_time/complectation_at_time** в схеме V004 НЕ используются для undo — они остаются как denormalized snapshot для отчётности (можно read'ить без JOIN на audit_log).
- **Retention:** Phase 1 SUMMARY говорит «retention — Phase 7 scheduled-tasks». В Phase 3 audit_log НЕ чистится → undo гарантированно работает. Если в будущем retention будет включён — нужна защита от delete acts старше retention window (Phase 7 problem).
- **Rationale:** Универсальный путь для всех типов undo, не требует разных схем для handover vs return; не зависит от того, был ли пользовательский custom edit между actions.

#### D-Soft-vs-Hard-Acts-01: delete акта — soft-delete (`deleted_at_utc`), CASCADE на act_items НЕ срабатывает
- `acts.deleted_at_utc = ?` и `act_items.deleted_at_utc` — оба soft (схема V004 имеет `act_items.deleted_at_utc` через standard4? Проверить в plan, схема показывает «NO standard4 columns» для act_items per D-Schema-03 — значит junction hard-deleted при soft-delete акта).
- Проверка: при soft-delete acts → CASCADE через FK не сработает (FK CASCADE срабатывает только на hard DELETE). Решение: явно `DELETE FROM act_items WHERE act_id = ?` в той же транзакции (junction теряем; данные восстановимы из audit_log).
- **Уникальность номера:** UNIQUE-индекс `idx_acts_number_sub_unique WHERE deleted_at_utc IS NULL` уже учитывает soft-delete — после удаления номер 42 свободен. Это **нежелательно** для аудита (номер 42 можно переиспользовать). Mitigation: на UI блокировать переиспользование удалённых номеров (валидация в сервисе: `SELECT 1 FROM acts WHERE number = ? LIMIT 1` без `deleted_at_utc IS NULL`).
- **Rationale:** Cooperation с REQ-ACT-13 (transactional guarantees) и Phase 1 invariant'ом soft-delete на пользовательских сущностях.

### Acts UI

#### D-Acts-List-01: master-detail (слева список, справа детали)
- Layout: `[switch-bar Акты/Возвраты/Архив со счётчиками] [горизонтальный split: список 35% / детали 65%]`.
- **Список:** компактные карточки (№, Дата, Принял ФИО, count устройств, мини-индикатор статуса). 50 строк/страница, пагинация снизу.
- **Детали:** при выборе строки — карточка акта с шапкой (№, Дата, Сдал, Принял, Сроком до, Расположение), table act_items (Устройство, Количество, [Состояние] [Расположение возврата]), список связанных return-актов, кнопки действий (Просмотр PDF, Редактировать, Возврат, Удалить).
- **Switch-bar:** Акты = `act_type='handover' AND archived=0 AND deleted_at_utc IS NULL`; Возвраты = `act_type='return' AND deleted_at_utc IS NULL`; Архив = `act_type='handover' AND archived=1 AND deleted_at_utc IS NULL`. Счётчики — отдельный command `acts_counts() -> { handover_active, returns, archived }`.
- **Пустое состояние:** при пустом списке справа — placeholder «Выберите акт слева или нажмите ➕ Создать».
- **Поиск:** общий input сверху над split'ом, ищет по `number`, `giver_name`, `receiver_name`, `device.name` (через FTS5; для devices — JOIN на `devices_fts` через `act_items.device_id`). Pattern — расширение D-Search-01 из Phase 2.
- **Rationale:** Master-detail подходит для «прочитал заголовок → посмотрел позиции»; экономит клики vs модал-детали; в server-mode (Phase 5) тот же layout работает в browser.

#### D-Acts-Create-01: широкий модал (~1000px), шапка + добавляемый список позиций
- Layout: шапка (№ с авто-кнопкой «Следующий», Дата, Сдал, Принял, Сроком до, Общее Расположение) + ниже table «Позиции»: каждая строка = device autocomplete (фильтр `status='На складе'`) + Количество (по умолчанию 1, спин-кнопки). Кнопка «➕ Добавить позицию».
- **Override номера:** поле № predicted и подсвечено «авто»; при ручной правке появляется маленький badge «override» и tooltip «Будет записано в журнал».
- **Валидация:** Сдал, Принял, ≥1 позиция, для каждой позиции — выбран device и quantity ≥ 1 (не превышает наличие на складе). Inline + блокировка кнопки «Создать».
- **Backend:** `acts_create(payload: ActCreateDto) -> ActDto` — single writer transaction (D-Counter-Acts-01).
- **Reuse Phase 2:** `Modal.svelte`, `Input.svelte`, `DeviceAutocompleteField.svelte` (расширить filter-prop'ом `status_in=['на_складе']`).
- **Rationale:** Модал держит контекст списка (пользователь видит, что новый акт появился); 1000px достаточно для 8 позиций без скролла.

#### D-Acts-Return-01: модал возврата с bulk-default + per-row override
- Открывается из detail-pane «Возврат». Заголовок: «Возврат по акту №42».
- Layout: верхний блок «Применить ко всем» (поля: Состояние, Расположение на складе) — defaults для всех чекнутых строк. Ниже table «Позиции к возврату»: чекбокс + Устройство + (опционально) Состояние/Расположение override (показывает «(по умолчанию)» серым, если не override). Количество к возврату (для не-уникальных).
- Кнопка «Применить» — для каждой чекнутой строки берёт override-значения если заданы, иначе bulk-default'ы.
- Backend: `acts_return(act_id, items: Vec<{act_item_id, quantity, condition, location_id}>) -> ActDto` — транзакция (D-Counter-Acts-01 для sub_number + D-Archive-01 для авто-архива).
- **Rationale:** REQ-ACT-08 («галочка применить ко всем») требует bulk-flow; per-row override — реальный кейс (часть устройств вернулась хорошими, часть требуют ремонта).

#### D-Print-UX-01: preview-модал с встроенным PDF.js viewer
- Кнопка «Печать» в detail-pane или return-модале → `acts_render_pdf(act_id) -> Vec<u8>` (PDF байты) → `URL.createObjectURL(blob)` → отображение в `<iframe>` или PDF.js viewer.
- В preview-модале: кнопки «Сохранить как...» (через `tauri-plugin-dialog` save), «Открыть в системном просмотрщике» (записать temp-file через `Paths::tmp_pdf()` → `tauri-plugin-shell::open`), «Печать» (`window.print()` на iframe — нативный диалог печати ОС).
- **PDF.js bundling:** добавить `pdfjs-dist` (~300KB в bundle). Альтернатива — нативный `<embed type="application/pdf">` (работает в Tauri WebView2 + WKWebView, но НЕ во всех Linux WebKitGTK). Выбираем pdfjs-dist для consistency.
- **Server-mode (Phase 5):** тот же UI работает — backend отдаёт `GET /api/v1/acts/:id/render.pdf` (Phase 5 task); browser получает PDF через тот же iframe.
- **Rationale:** Preview перед сохранением — стандартный UX для печатных форм; всё работает desktop + browser единым кодом.

### PDF-инфраструктура

#### D-PDF-Engine-01: krilla 0.7 без spike, но первый план фазы = «PDF-инфра + фикстура»
- **Engine:** krilla 0.7 (per CLAUDE.md + research SUMMARY 5.1).
- **Шрифт:** **DejaVu Sans Regular + Bold**, оба cut'а embedded через `include_bytes!` в crate `trackly-app/assets/fonts/`. krilla подсетит автоматически (видим только используемые глифы в финальном PDF).
- **Лицензия:** DejaVu — public-domain-derived (Bitstream Vera), без атрибуции в UI обязательной.
- **CI hash-test:** байт-в-байт hash фикстурного PDF на всех 3 ОС (linux/macOS/windows GH-runners). krilla 0.7 + фиксированный font + фиксированный шаблон → детерминированный output; pinned `krilla = "=0.7.X"` в Cargo.toml. Тест в `crates/trackly-app/tests/pdf_fixture.rs`:
  ```rust
  let pdf = render_act_pdf(fixture_act());
  assert_eq!(sha256(&pdf), "expected_hash_here");
  ```
- **Fixture content:** «Сидоров-Петроградский Иван Александрович (ё) №42» обязан присутствовать в text-content (verify через extract либо просто включить в hash через стабильный шаблон).
- **Структура фазы — план 01 = PDF-инфра:** первый plan = «PDF foundation»: добавить krilla, fonts, MiniJinja-сервис, DocSpec-IR, renderer заглушку, CI hash-test на hardcoded-шаблоне «Привет, мир». До завершения этого plan'а реальные acts-print commands не пишем. Если krilla даст сбой на этом этапе — спайк typst-as-lib в отдельную mini-фазу до продолжения.
- **MSRV:** krilla 0.7 требует Rust ≥ 1.92 — CI matrix Phase 3 обновляет `rust-toolchain.toml` (если ниже) или CI step. Это закрывает Phase 1 deferred-item «Windows 7 32-bit MSRV check» (или окончательно его deferr'ит как «не v1»).
- **Rationale:** Research флагнул krilla как primary; spike до commit — лишний шаг, если основной риск (Cyrillic glyphs) проверяется первым же planом. План 01 — это и есть структурный спайк.

#### D-PDF-Render-Path-01: MiniJinja → DocSpec JSON → krilla renderer (трёхэтапный pipeline)
- **Stage 1 — MiniJinja:** template (из БД) + context (`act`, `org`) → JSON-строка. Engine в safe-mode: `UndefinedBehavior::Strict`, `add_global`-только, **БЕЗ** `loader::Loader` (значит — никаких `{% include %}` с file paths), render timeout 5s через `tokio::time::timeout`.
- **Stage 2 — Validate:** `serde_json::from_str::<DocSpec>(rendered)` — strict deser. Невалидный JSON → `AppError::Validation { field: "template", message: ... }`.
- **Stage 3 — krilla render:** DocSpec → krilla calls → PDF bytes.
- **DocSpec schema (черновик):**
  ```rust
  pub struct DocSpec {
      pub title: String,
      pub header: HeaderBlock,        // org name/inn/address/logo + Act # + Date
      pub sections: Vec<Section>,     // ordered, semantic blocks
  }
  pub enum Section {
      Paragraph { text: String, style: TextStyle },
      Heading { level: u8, text: String },
      KeyValueTable { rows: Vec<(String, String)> },  // Сдал: ФИО / Принял: ФИО / ...
      ItemsTable { columns: Vec<String>, rows: Vec<Vec<String>> },
      Signature { left_label: String, right_label: String, spacer: Pt },
      Spacer { height: Pt },
  }
  ```
  Это уточняется в plan 01 (PDF foundation); схема жёстко сериализуема через serde и сохраняется как контракт.
- **Rationale:** Изоляция шаблона от krilla даёт безопасность (нет произвольного кода), валидируемость (типизированный AST), и testability (можно тестировать DocSpec без рендера).

#### D-PDF-Templates-Schema-01: MiniJinja-context, что видит шаблон
```jinja
{# доступные переменные #}
org.name           # из org.json
org.inn
org.kpp
org.address
org.logo_path     # абсолютный путь (внутри Paths::root()) — krilla читает файл, шаблон только передаёт путь
act.number        # 42
act.suffix        # "" для handover, "в1" для return, etc. (отформатировано на бэке)
act.date          # ISO + локализованная форма «28 мая 2026 г.»
act.giver_name
act.receiver_name
act.deadline      # nullable
act.location_name # имя локации, не id
act.items         # массив { name, inventory_no, serial_no, model, specs, kit, condition, quantity }
act.parent        # для return — { number, date }; для handover — null
return.condition_default  # bulk-default'ы возврата для рендера в «акте возврата»
return.location_default
```
- Контракт — единый для act_handover, act_acceptance, в будущем — других kind'ов. Phase 7 расширяет (logo binary, settings).
- **Rationale:** Стабильный контракт — основа для пользовательских шаблонов; Phase 7 редактор будет работать на этом же контракте.

### Шаблоны и org-data

#### D-Templates-Seed-01: runtime-seeding из `include_str!`-файлов на startup
- В `crates/trackly-app/templates/` — два файла:
  - `act_handover.minijinja` — дефолтный шаблон Акта приёма-передачи (Russian, с реквизитами организации, таблицей позиций и подписями)
  - `act_acceptance.minijinja` — дефолтный шаблон Документа приёма товара на склад
- В `AppCtx::build` (после миграций):
  ```rust
  for (kind, template_str) in DEFAULT_TEMPLATES {
      let count: i64 = conn.query_row(
          "SELECT COUNT(*) FROM document_templates WHERE kind = ?1 AND deleted_at_utc IS NULL",
          [kind], |r| r.get(0))?;
      if count == 0 {
          // INSERT default
      }
  }
  ```
- **Идемпотентность:** при count > 0 — пропуск; пользователь soft-удалил все шаблоны kind'а — пересоздаём (это feature: «сбросить к дефолтам»).
- **Versioning:** дефолтный шаблон хранит `name = "Дефолтный (v1)"` в БД — Phase 7 UI редактор покажет это и позволит создать копию.
- **Rationale:** Файлы шаблонов в репо удобны для review/lint/diff'а; SQL-миграция с большими строками — неудобна. Idempotency через count = простой и понятный invariant.

#### D-OrgData-01: `org.json` рядом с .exe (через `Paths::root()`)
- Расположение: `<paths.root>/org.json`. Через `Paths` (НЕ `dirs::*_dir`).
- Формат:
  ```json
  {
    "name": "Ваша организация",
    "inn": "0000000000",
    "kpp": "000000000",
    "address": "г. Москва, ул. ...",
    "logo_path": "logo.png"
  }
  ```
- При отсутствии — создаём с placeholder'ами при первом старте (логируется warning).
- **Сервис:** `OrganizationService` (новый, в `trackly-app/src/services/`) — читает на запрос (без хранения), Phase 3 не делает file-watch (вернёмся в Phase 7). Один command `organization_get() -> OrgDto`.
- **Logo:** `logo_path` — относительный путь от `Paths::root()`. krilla читает файл при render'е (поддерживает PNG/JPEG embedding). Если файла нет — рендерим без логотипа (warning в логи, не ошибка).
- **Backup БД НЕ захватывает org.json:** это намеренно. Org-data — это локальная конфигурация инстанса, не данные. Phase 7 может пересмотреть: если решим хранить в БД — миграция данных из org.json одноразовая.
- **Rationale:** Phase 7 (SET-*) ещё не определён; не хочется тянуть в Phase 3 половинчатую таблицу. JSON-файл проще в первой версии; UI редактор появится в Phase 7 (либо как замена на DB, либо как простой form-редактор JSON).

### Composition + Tests

#### D-AppCtx-Extension-03: AppCtx расширяется `acts`, `organization`, `templates`, `pdf`
- `AppCtx { ..., devices: Arc<DeviceService>, acts: Arc<ActService>, organization: Arc<OrganizationService>, templates: Arc<TemplateService>, pdf: Arc<PdfRenderer> }`.
- `PdfRenderer` держит embedded fonts (`Arc<[u8]>` для DejaVu Regular/Bold) и MiniJinja `Environment` (тяжёлый объект — переиспользуем).
- Все четыре новых поля — `Arc<...>`, так что AppCtx остаётся `Clone`.

#### D-Test-Phase3-01: тесты по слоям
- **Unit:** sub_number computation, format_act_number, archive predicate, DocSpec serde round-trip.
- **Integration (с real SQLite):** create handover → status='В работе', counter incremented; partial return → sub_number=1, archived=0; full second return → sub_number=2, archived=1; delete return → archived=0, devices restored from audit_log; delete handover → devices fully restored.
- **PDF:**
  - hash test: фикстурный act + дефолтный шаблон → expected sha256 (per D-PDF-Engine-01).
  - text-extract test: rendered PDF содержит «Сидоров-Петроградский» и «№42» (через `pdf-extract` или аналог).
- **Tauri commands:** для каждого command — `build_*` helper test (как Phase 2 D-Test-Phase2-01).

### Claude's Discretion

- Точная форма DocSpec enum-вариантов и optional полей — уточняется в plan 01.
- Структура `crates/trackly-app/src/pdf/` (mod.rs split на renderer/docspec/fonts) — на усмотрение планировщика.
- Точная структура `act_items` колонок — есть ли поле `quantity` или `condition_returned` (для multi-state в-работе/вернулось — может потребоваться доп. колонки в Phase 3 или separate `act_item_returns` таблица). Plan'у решать после изучения схемы.
- Имена commands (`acts_list`, `acts_create`, ...) — следовать snake_case + namespace `acts_*` как для `devices_*`.
- Файл `pdfjs-dist` подключение — через npm dep + dynamic import (для tree-shake'а в server-mode bundle).
- Конкретный шаблон Russian-typography (отступы, шрифт-размеры) — планировщик согласует с дизайном Phase 2 (`_tokens.scss` цвета НЕ используются в PDF — там монохром по умолчанию).
- Конкретные миграции (V014__acts_indexes_or_seeds.sql если нужны) — планировщик решает.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-Level (must-read)
- `CLAUDE.md` — стек (krilla 0.7, MiniJinja, MSRV 1.92, шрифты embed), что НЕ использовать (printpdf без font embed; wkhtmltopdf shellout).
- `.planning/PROJECT.md` — Core Value: «одной кнопкой, без потери истории» — это и есть Phase 3.
- `.planning/REQUIREMENTS.md` §«Acts (ACT)» — ACT-01..14 + DEV-14..15 точные формулировки.
- `.planning/ROADMAP.md` §«Phase 3: Акты приёма-передачи и первая PDF-печать» — 5 success criteria (1: атомарная нумерация + override + audit_log; 2: «в»/«в1..N» суффиксы + bulk-apply + auto-archive; 3: undo восстанавливает state из audit_log; 4: PDF с реальными глифами + CI hash + шаблоны из БД; 5: поиск + switch-bar + счётчики).

### Phase 1 carry-forward (Phase 3 надстраивает поверх)
- `.planning/phases/01-foundation/01-CONTEXT.md` — D-WriterChannel-01, D-AppError-01, D-Logging-01, D-Schema-03 (standard4 на entity tables), D-Schema-05 (audit_log shape).
- `.planning/phases/01-foundation/01-VERIFICATION.md` — что доказано (WAL, single-writer, migrations, Paths).
- `.planning/phases/01-foundation/01-04-SUMMARY.md` — `WriterHandle::execute<F,R>`, `ReaderPool::acquire()`, `AppError` 9 variants, `AppCtx::build`.
- `.planning/phases/01-foundation/01-05-SUMMARY.md` — `build_*` helper pattern, тонкие Tauri+axum adapter'ы, HealthDto как DTO-эталон.
- `migrations/V004__acts.sql` — schema acts + act_items + UNIQUE индекс на (number, COALESCE(sub_number,0)) WHERE not deleted.
- `migrations/V007__document_templates.sql` — kinds `act_handover`, `act_acceptance`, body_minijinja TEXT, partial UNIQUE на (kind) WHERE is_active+not deleted.
- `migrations/V008__audit_log.sql` — before_json/after_json/payload_json + retention откладывается на Phase 7.
- `migrations/V009__counters.sql` — `act_number` row seeded current_value=0.

### Phase 2 carry-forward (паттерны, которые повторяем)
- `.planning/phases/02-ui/02-CONTEXT.md` — D-Repo-01 (гексагональная раскладка core/infra/app — повторяем для acts), D-UI-Structure-01 (feature-folders), D-UI-Transport-01 (transport detection), D-UI-State-01 (runes), D-UI-Errors-01 (toast host), D-UI-Validation-01 (manual через runes — реиспользуем для широкого модала), D-Bindings-01 (specta export), D-AppCtx-Extension-01 (расширение AppCtx).
- `.planning/phases/02-ui/02-RESEARCH.md` — паттерны для feature folder, FTS5.
- `.planning/phases/02-ui/02-PATTERNS.md` — реиспользуемые модули.
- `.planning/phases/02-ui/02-05-SUMMARY.md` — CSV import/export — НЕ для Phase 3 напрямую, но дает паттерн «encoding-sensitive workflow» (PDF тоже encoding-sensitive).

### Research (общая для проекта)
- `.planning/research/ARCHITECTURE.md` — hexagonal layout, dual-transport pattern.
- `.planning/research/STACK.md` — versions (krilla 0.7, MiniJinja).
- `.planning/research/PITFALLS.md` §«Pitfall 7: PDF generation with Cyrillic» — embed font, hash test, missing glyphs detection.
- `.planning/research/PITFALLS.md` §«Pitfall 14: User-editable templates» — UndefinedBehavior::Strict, 5s timeout, version field, reset-to-default.
- `.planning/research/SUMMARY.md` §«Phase 5: Акты приёма-передачи + PDF» — krilla default + spike at fixture, MiniJinja safe-mode + render timeout.

### External (researcher fodder если копать глубже)
- krilla: https://github.com/LaurenzV/krilla
- MiniJinja safe-mode: https://docs.rs/minijinja/latest/minijinja/struct.Environment.html
- DejaVu Fonts license: https://dejavu-fonts.github.io/License.html
- PDF.js: https://github.com/mozilla/pdf.js
- `pdf-extract` for text-extract tests: https://docs.rs/pdf-extract/
- Tauri 2 plugin-dialog + plugin-shell: https://v2.tauri.app/plugin/dialog/, https://v2.tauri.app/plugin/shell/

</canonical_refs>

<code_context>
## Existing Code Insights (от Phase 1+2)

### Reusable Assets
- `crates/trackly-infra/src/db/writer_worker.rs::WriterHandle::execute<F,R>` — единственный путь для writes (Phase 3 acts mutations + counter increments).
- `crates/trackly-infra/src/db/pools.rs::ReaderPool::acquire()` — reads (acts list, detail, search).
- `crates/trackly-app/src/context.rs::AppCtx` — расширяется 4 новыми полями (см. D-AppCtx-Extension-03).
- `crates/trackly-app/src/dto/health.rs::HealthDto` — DTO pattern (snake_case, manual derives, `specta::Type`).
- `crates/trackly-app/src/services/device_service.rs` — pattern для ActService (service в trackly-app, repo в trackly-infra, port в trackly-core).
- `crates/trackly-core/src/ports/devices.rs` — pattern для `crates/trackly-core/src/ports/acts.rs`.
- `crates/trackly-infra/src/repos/devices_sqlite.rs` — pattern для `acts_sqlite.rs`.
- `crates/trackly-app/src/csv/` (Phase 2) — pattern для `crates/trackly-app/src/pdf/` (отдельный mod с MiniJinja env + krilla renderer).
- `migrations/V004`, `V007`, `V008`, `V009` — все нужные tables уже есть, V013 — последняя миграция.
- `ui/src/lib/api/client.ts` — apiCall<R>, добавим `acts.ts` + `templates.ts` + `organization.ts` + `pdf.ts`.
- `ui/src/lib/components/Modal.svelte`, `Input.svelte`, `Button.svelte`, `Toast.svelte`, `ToastHost.svelte` — переиспользуются.
- `ui/src/features/devices/DeviceAutocompleteField.svelte` — расширим filter-prop для «device на складе»; либо клонируем как `ActDeviceAutocomplete.svelte`.
- `ui/src/features/layout/sidebar-config.ts` — раздел «Акты» уже placeholder; Phase 3 заменяет на реальный роут.

### Established Patterns (Phase 1+2 locked, Phase 3 наследует)
- **Hexagonal:** core/ports + infra/repos + app/services + app/tauri_cmds + app/http.
- **Single-writer для всех mutations + counter increments** — Phase 3 acts/returns/template-seeds.
- **DTO в trackly-app, snake_case JSON.**
- **`AppError` unified shape** — добавятся новые variants? Возможно нет — Validation/Conflict/NotFound/Internal покрывают всё.
- **UTC unix seconds для timestamps.**
- **Soft-delete (`deleted_at_utc`) на entity tables.**
- **Audit_log на все mutations.**
- **specta_export `collect_commands!` — расширяется.**

### Integration Points
- **`AppCtx::build`** — после device_service init: `let acts = Arc::new(ActService::new(writer.clone(), readers.clone(), clock.clone(), audit_writer))`, `let templates = Arc::new(TemplateService::new(...))`, etc. + first-run seed templates check.
- **`specta_export::builder()`** — `collect_commands![..., acts_list, acts_get, acts_create, acts_update, acts_delete, acts_return, acts_render_pdf, acts_search, acts_counts, organization_get, templates_get, templates_render_preview]`.
- **axum `Router`** — `http::acts::router().merge(http::organization::router())` в `AppCtx::build`.
- **Migrations** — возможно V014 для дополнительных индексов (`act_items.act_id`, `acts.parent_act_id`, `audit_log.entity_type+entity_id+created_at_utc` для undo-lookup).
- **`tests/export_bindings.rs`** assertions расширяются — DocSpec, ActDto, ActCreateDto, RenderPdfResponse и т.д.
- **Sidebar:** заглушка «Акты» → реальная страница.

### Not-yet-existing (создаём в Phase 3)
- `crates/trackly-core/src/ports/acts.rs` — `ActRepository` trait + domain types.
- `crates/trackly-core/src/domain/acts.rs` — `ActNew`, `ActPatch`, `ReturnItem`, `ActFilter`.
- `crates/trackly-infra/src/repos/acts_sqlite.rs` — SQLite impl.
- `crates/trackly-app/src/services/act_service.rs`, `template_service.rs`, `organization_service.rs`.
- `crates/trackly-app/src/dto/{act.rs, organization.rs, doc_spec.rs}`.
- `crates/trackly-app/src/tauri_cmds/{acts.rs, organization.rs}`.
- `crates/trackly-app/src/http/{acts.rs, organization.rs}`.
- `crates/trackly-app/src/pdf/mod.rs` — submod'ы `renderer.rs`, `docspec.rs`, `fonts.rs`, `minijinja_env.rs`.
- `crates/trackly-app/assets/fonts/DejaVuSans.ttf`, `DejaVuSans-Bold.ttf`.
- `crates/trackly-app/templates/act_handover.minijinja`, `act_acceptance.minijinja`.
- `crates/trackly-app/tests/{acts_*.rs, pdf_fixture.rs}`.
- `ui/src/features/acts/{ActsPage.svelte, ActsList.svelte, ActDetail.svelte, ActFormModal.svelte, ReturnModal.svelte, PdfPreviewModal.svelte, api.ts}`.
- `ui/src/lib/api/{acts.ts, organization.ts}`.

</code_context>

<specifics>
## Specific Ideas

- **Фикстурная строка для PDF hash-test:** «Сидоров-Петроградский Иван Александрович (ё) №42» — уже была обозначена как канонический Cyrillic-tester в Phase 1/2; в Phase 3 это становится **обязательным содержимым** фикстурного шаблона.
- **«в» suffix форматирование:** padded zero не нужен («42в», «42в1», не «42в01»).
- **PDF.js viewer:** прочитанный `<iframe src="blob:...">` — простейший путь; кастомный PDF.js worker — overkill для preview.
- **Org.json placeholders при создании:** `"name": "Ваша организация"`, `"address": "Укажите адрес в settings/org.json"` — пользователь сразу видит, что надо заполнить.
- **`logo.png` рядом с `org.json`:** дефолт `"logo_path": "logo.png"`; если файл не существует — рендер без логотипа.
- **Master-detail split persistence:** размеры split'а (35/65) — фиксированные в Phase 3; resizable splitter — Phase 7 UX-полишинг.
- **Return-модал «Применить ко всем»:** галочка по умолчанию ВКЛ (90% кейсов — bulk).
- **Counter override audit:** пишется как `action='custom:act_number_override'` (соглашение префикса `custom:` для не-CRUD действий).

</specifics>

<deferred>
## Deferred Ideas

- **UI редактор шаблонов** (CRUD `document_templates` в UI настроек) → Phase 7.
- **Полноценная страница Организация/Настройки** → Phase 7. Phase 3 только читает `org.json`.
- **3-way merge для обновлённых дефолтных шаблонов** (Pitfall 14 рекомендация) → Phase 7.
- **«Сбросить шаблон к дефолту»** кнопка → Phase 7 (в Phase 3 достижимо через soft-delete всех записей kind'а + restart).
- **Logo binary в БД** (вместо file path) → пересмотрим в Phase 7 (вместе с org-data table если уйдём от JSON).
- **PDF.js custom worker / встроенный print** → Phase 3 использует нативный `window.print()` диалог; кастомизация — out of scope.
- **Retention для audit_log** → Phase 7 (scheduled tasks); до того момента Phase 3 undo полагается на full history.
- **Resizable split master-detail** → Phase 7 UX полишинг.
- **Виртуализация списка актов** (>5000 актов) → отдельная perf-фаза если потребуется.
- **Печать списка/отчёта по актам** (не PDF одного акта) → Phase 7 (отчёты переиспользуют PDF-инфраструктуру).
- **Заявки печать (REQ-04)** — переиспользует Phase 3 PDF-pipeline → Phase 6.
- **Watch-режим для `org.json`** (автоперечитка при изменении файла) → Phase 7 если будет UI редактор JSON.
- **Запрет переиспользования удалённых номеров** через отдельную таблицу/индекс → отслеживаем; пока валидация в сервисе.
- **Spike krilla vs typst-as-lib** — НЕ делаем upfront; делаем только если plan 01 (PDF-инфра) покажет проблемы с krilla.

</deferred>

---

*Phase: 3-pdf*
*Context gathered: 2026-05-28 via /gsd-discuss-phase 3*
