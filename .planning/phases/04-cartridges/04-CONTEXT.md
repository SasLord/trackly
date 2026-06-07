# Phase 4: Картриджи - Context

**Gathered:** 2026-06-07
**Status:** Ready for planning
**Source:** /gsd-discuss-phase 4 — 4 области (модель/совместимость, раздел/карточка, lifecycle-операции, низкий остаток/код) обсуждены интерактивно. Ключевое расширение от пользователя: раздел охватывает картриджи И фотобарабаны.

<domain>
## Phase Boundary

Поставить раздел «Картриджи» end-to-end (CART-01..12): подраздел «Модели» (с матрицей совместимых принтеров) и подраздел «Картриджи (экземпляры)» с авто-кодом `C-000001`, lifecycle через контекстные действия по статусу (установка в принтер / возврат на склад / отправка-возврат с заправки / списание), история перемещений из `audit_log`, полнотекстовый поиск, switch-bar по статусу со счётчиками и баннер низкого остатка.

**Расширение области (locked пользователем):** раздел «Картриджи» охватывает **и картриджи, и фотобарабаны** — единый раздел, единый lifecycle, единая схема. Это НЕ отдельная фаза/раздел — различие выражается атрибутом `kind` на модели (Картридж / Фотобарабан).

**В scope:**
- CART-01..12 целиком.
- Модели: CRUD (Бренд, Модель, Тип расходника, Цвет, Примечание, Совместимые принтеры).
- Экземпляры: CRUD с авто-кодом из counter `cartridge_seq` + пользовательский override (штрих-код).
- Lifecycle: единая параметризованная модалка операций + контекстное меню по статусу.
- История перемещений в карточке экземпляра (из `audit_log`).
- Switch-bar статусов + фильтры (тип, модель) + FTS-поиск.
- Баннер низкого остатка в разделе «Картриджи».
- Миграция V016 (kind lookup + color + app_settings + cartridges_fts триггеры — см. решения).

**НЕ в scope этой фазы (явно deferred):**
- Управление пометкой складских локаций (`locations.kind`) через UI → Phase 7 (Settings, SET-*).
- Настраиваемый порог низкого остатка (SET-04) UI-редактор → Phase 7 (в Phase 4 — строка в `app_settings`, дефолт 2).
- Дашборд-виджет картриджей + «Динамика расхода» → Phase 7 (DASH-*).
- Связь установки картриджа с конкретным устройством-принтером (FK) и REQ-05 (заявка → установка) → Phase 6.
- Автокомплит совместимых принтеров из реальных устройств-принтеров БД → Phase 6 (сейчас — из ранее введённых текстовых пар).
- Login / RBAC / авторизация операций → Phase 5 (`audit_log.user_id` = NULL в Phase 4).
- Server-mode HTTP-хендлеры картриджей: axum router строится (как в Phase 2/3), но не bind'ится → Phase 5/8.

**Mode:** mvp — вертикальный слайс на каждый план: UI → tauri command (+ axum route) → service → repo → DB.

</domain>

<decisions>
## Implementation Decisions

### Расширение домена

#### D-Scope-01: раздел охватывает картриджи И фотобарабаны
- Единый раздел, единый lifecycle, единая схема `cartridges`/`cartridge_models`. Различие — атрибут `kind` модели (Картридж / Фотобарабан).
- Обоснование пользователя: «это совершенно разные устройства по своему типу» (у фотобарабана не может быть цвета), но оба проходят один и тот же цикл склад→в работе→заправка/возврат→списание и относятся к учёту расходников печати.

### Модель картриджа (CART-01, CART-02)

#### D-Model-Fields-01: Бренд + Модель — два отдельных TEXT-поля с focus-open автокомплитом
- `cartridge_models.brand` + `cartridge_models.model` (уже в V005), оба — focus-open автокомплит из ранее введённых DISTINCT-значений (паттерн `LocationAutocomplete` / DEF-1 focus-open).
- «Название» модели = отображение «{brand} {model}» (напр. «Pantum TL-5120X»). Отдельного поля `name` НЕ добавляем.
- UNIQUE(brand, model) среди живых строк — уже есть (V005 `idx_cartridge_models_brand_model_unique`).

#### D-Model-Kind-01: тип расходника `kind` — Картридж / Фотобарабан
- Добавить lookup-таблицу `cartridge_kinds` (1 = Картридж, 2 = Фотобарабан) + колонку `cartridge_models.kind_id INTEGER NOT NULL DEFAULT 1` (миграция V016). Паттерн как `cartridge_statuses`/`cartridge_states` (V001).
- Default — Картридж.

#### D-Model-Color-01: Цвет — фиксированный набор, скрыт для фотобарабана
- Добавить `cartridge_models.color TEXT` (V016). Фиксированный набор значений в UI (dropdown): **Чёрный, Голубой, Пурпурный, Жёлтый, Светло-голубой, Светло-пурпурный**. Default — **Чёрный**.
- UI: поле «Цвет» **СКРЫТО когда kind = Фотобарабан**. В БД для фотобарабанов допустимо хранить «Чёрный» (или NULL) — главное не показывать в интерфейсе.

#### D-Model-NoCompatType-01: «Оригинальный/Совместимый» НЕ хранить
- Это различие определяется по бренду (Pantum = оригинал, Cactus/прочие = совместимый). Отдельное поле не нужно.

#### D-Model-Compat-01: совместимые принтеры — массив пар Бренд+Модель
- Junction `cartridge_model_compatibility(printer_brand, printer_model)` (уже в V005, без standard4).
- UI: добавляемый список пар (Бренд + Модель) с автокомплитом.
- **Источник автокомплита (фазированно):**
  - **Phase 4 (сейчас):** DISTINCT `printer_brand`/`printer_model` из `cartridge_model_compatibility` (частота DESC) — «ранее введённые» (формулировка CART-02).
  - **Phase 6:** дополнить реальными принтерами из БД (устройства type=Принтер). Сигнатура автокомплита не меняется — расширяется только источник за командой (паттерн как PersonAutocomplete UNION с AD в Phase 5).

### Раздел и навигация (CART-03, CART-05, CART-10)

#### D-Nav-01: один пункт сайдбара, два таба внутри
- Пункт сайдбара «Картриджи» (route `/cartridges` — уже placeholder в `sidebar-config.ts`, `phase: 4`).
- Внутри — два таба: **«Картриджи»** (экземпляры, основной по умолчанию) / **«Модели»**.

#### D-Detail-01: master-detail для экземпляров (как акты)
- Список слева / детали справа — паттерн `ActsMasterDetail.svelte` + `ActDetail.svelte`.
- Детали содержат карточку экземпляра + **хронологию перемещений из `audit_log`** (CART-10).
- Модели — отдельный CRUD-список во вкладке «Модели».

#### D-Filters-01: switch-bar по статусу + фильтр по типу + по модели + поиск
- Switch-bar по статусу со счётчиками: **Все / На складе / В работе / На заправке / Списано** (lookup `cartridge_statuses`, V001).
- Доп. фильтр по **типу** (Картридж / Фотобарабан), доп. фильтр по **модели**.
- Полнотекстовый поиск (CART-11) — см. D-Search-01.

### Lifecycle-операции (CART-06, CART-07, CART-08, CART-09)

#### D-Op-Modal-01: единая параметризованная модалка операций
- Одна `OperationModal` — заголовок, поля и фильтр автокомплита локации зависят от типа перехода.
- Бэкенд — одна команда (напр. `cartridges_transition(cartridge_id, op, fields)`) под single-writer; точная сигнатура — на усмотрение планировщика.

#### D-Op-Transitions-01: контекстное меню по статусу (CART-06)
- **На складе** → Установить в принтер (→ В работе) / Отправить на заправку (→ На заправке) / Списать (→ Списано) / Редактировать / Удалить
- **В работе** → Вернуть на склад (→ На складе) / Редактировать / Удалить
- **На заправке** → Забрать с заправки (→ На складе) / Редактировать / Удалить
- Паттерн меню — `DeviceContextMenu.svelte`.

#### D-Op-Fields-01: поля операций и дефолты заряда
- **Установить в принтер (CART-07):** Дата (DatePicker), Кто выдал (PersonAutocomplete), Кому выдал (PersonAutocomplete), Расположение (LocationAutocomplete «не на складе»). `holder_name` = «Кому выдал» (денорм). Заряд (`state_id`) **НЕ меняется**.
- **Вернуть на склад (CART-08):** Состояние заряда (default **Пустой**), Расположение (LocationAutocomplete «на складе»), Примечания. `holder_name` очищается.
- **Отправить на заправку / Забрать с заправки (CART-09):** аналогично выдаче/возврату, тот же набор полей. **Забрать с заправки** — заряд default **Полный**.
- **Списать:** модал с **Дата + Причина/Примечание** → статус Списано. Причина попадает в `audit_log`.
- Все дефолты заряда **редактируемы**.

#### D-Op-Location-01: «на складе» / «не на складе» через существующий `locations.kind`
- В `locations` (V002) УЖЕ есть колонка `kind` ('office' | 'warehouse' | 'repair' | freeform). **Отдельная колонка `is_warehouse` НЕ нужна** — используем `kind`.
- «на складе» = `kind = 'warehouse'`; «не на складе» = `kind != 'warehouse'` (или NULL).
- **Phase 4:** оба автокомплита показывают ВСЕ локации (т.к. `kind` пока массово не проставлен, управление — Phase 7). Подписи «на складе»/«не на складе» — подсказки; фильтрация по `kind` включится автоматически после Phase 7 (Settings).
- `cartridges.location` — freeform TEXT (V005), не FK на `locations`. Автокомплит брать из таблицы `locations` (общий справочник с устройствами); хранить выбранное имя в `cartridges.location` как TEXT. Планировщик решает, делать ли round-trip `INSERT OR IGNORE` новых значений в `locations` (как devices) — рекомендуется для единого справочника.

### Код экземпляра (CART-04)

#### D-Code-01: авто-код `C-NNNNNN` из counter `cartridge_seq`
- Counter `cartridge_seq` уже seed'нут (V009, current_value=0). Паттерн D-Counter-Acts-01: `UPDATE counters SET current_value = current_value + 1 WHERE name='cartridge_seq' RETURNING current_value` под `BEGIN IMMEDIATE` в single-writer task.
- Формат: «C-» + zero-padded 6 цифр (`C-000001`).

#### D-Code-Override-01: пользовательский код + counter не теряется
- Пользователь может ввести свой код (штрих-код с приёмки). При custom-коде counter **НЕ инкрементируется**; пишется `audit_log` с `action='custom:cartridge_code_override'`.
- При коллизии авто-кода с уже существующим (UNIQUE) — инкрементировать counter до свободного значения в той же tx («номер из счётчика не теряется при коллизии» — ROADMAP крит.1).
- Конфликт по UNIQUE при custom-коде → `AppError::Conflict { field:"code", ... }`.

### История перемещений (CART-10)

#### D-History-01: операции пишут `audit_log`, карточка рендерит хронологию
- Каждая операция (transition) пишет `audit_log` row: `entity_type='cartridge'`, `entity_id`, `action`, `before_json`/`after_json`, `payload_json` с `{op, дата, кто выдал, кому выдал, расположение, заряд, причина}`.
- Карточка экземпляра рендерит хронологический список человекочитаемо (напр. «12.06.2026 — Установлен в принтер; выдал Иванов И.И., получил Петров П.П.; Каб. 305»).
- Префикс `custom:` для не-CRUD операций (соглашение из Phase 3). Конкретные action-коды — на усмотрение планировщика.

### Низкий остаток (CART-12)

#### D-LowStock-01: порог в таблице `app_settings`
- Создать таблицу `app_settings(key TEXT PRIMARY KEY, value TEXT, ...)` (миграция V016), seed `low_stock_threshold = '2'` (дефолт из ROADMAP).
- Phase 7 SET-04 даст UI-редактор; в Phase 4 — фиксированная строка 2 (читается, не редактируется через UI).

#### D-LowStock-02: правило и переиспользуемая команда
- Правило (CART-12, ROADMAP крит.5): для каждой модели `count(экземпляров: status='На складе' AND state='Полный') < threshold` → модель в «низком остатке».
- Команда `cartridges_low_stock() -> Vec<{model, count, threshold}>` — переиспользуема Phase 7 дашбордом.

#### D-LowStock-03: баннер в разделе, задел под дашборд
- Баннер показывается сверху раздела «Картриджи» (Phase 4) с перечнем моделей ниже порога и текущим количеством.
- Дашборд-виджет — Phase 7 (использует ту же команду `cartridges_low_stock`).

### Поиск (CART-11)

#### D-Search-01: FTS по коду/расположению/держателю + JOIN на модель
- `cartridges_fts` уже создана (V012: `code, location, holder_name`, `content='cartridges'`). **Внимание:** триггеры синхронизации FTS для cartridges, вероятно, отсутствуют (devices получили их в V013) — добавить cartridges_fts триггеры в V016. Планировщик проверяет.
- Поиск по модели (brand/model) — через JOIN `cartridge_models`. Паттерн как acts search (LIKE + FTS UNION, Phase 3-05).

### Claude's Discretion

- Сигнатура `cartridges_transition` vs отдельные команды (install/return/to_refill/from_refill/write_off) за единой UI-модалкой.
- `kind` как lookup-таблица (рекомендуется, consistency) vs CHECK-enum.
- Удаление модели при наличии живых экземпляров: запрет с понятной ошибкой (рекомендуется) либо каскадный soft-delete — планировщик.
- Делать ли `INSERT OR IGNORE` новых локаций в `locations` при вводе в операциях картриджей, или хранить только `cartridges.location` TEXT (рекомендуется round-trip для единого справочника).
- Конкретные `audit_log` action-коды для операций.
- Установка «в принтер» в Phase 4 НЕ связывается с конкретным устройством-принтером (FK) — только Расположение/Кому выдал; связь с принтером и REQ-05 — Phase 6.
- Структура hexagonal-слоёв и feature-папки (см. code_context) — паттерн как acts/devices.
- Точный состав миграции V016 (kind lookup + color + app_settings + cartridges_fts триггеры) — финализирует планировщик.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-level (must-read)
- `CLAUDE.md` — стек (rusqlite + refinery, single-writer, tauri 2.11, svelte 5, axum), portable-дисциплина, что НЕ использовать.
- `.planning/PROJECT.md` — Core Value «одной кнопкой, без потери истории»; учёт картриджей по моделям/заправкам/остаткам — болевая точка #3.
- `.planning/REQUIREMENTS.md` §«Картриджи (CART)» — CART-01..12 точные формулировки.
- `.planning/ROADMAP.md` §«Phase 4: Картриджи» — 5 success criteria (1: матрица совместимости + авто-код + override + counter не теряется; 2: switch-bar + контекстное меню по статусу; 3: поля установки/возврата/заправки + история из audit_log; 4: поиск; 5: баннер низкого остатка по SET-04).

### Схема (миграции — уже существуют от Phase 1)
- `migrations/V005__cartridges.sql` — `cartridge_models(brand, model, notes)`, `cartridge_model_compatibility(printer_brand, printer_model)`, `cartridges(code UNIQUE, model_id, status_id DEFAULT 1, state_id, location TEXT, holder_name, notes)`.
- `migrations/V001__init_pragmas_and_lookups.sql` — lookups `cartridge_statuses` (1 На складе / 2 В работе / 3 На заправке / 4 Списано), `cartridge_states` (1 Полный / 2 Частичный / 3 Пустой).
- `migrations/V002__core_entities.sql` — `locations(name UNIQUE, kind 'office'|'warehouse'|'repair', address, notes, ...)` — **`kind` для различения склад/не-склад** (D-Op-Location-01).
- `migrations/V009__counters.sql` — `cartridge_seq` (current_value=0) для авто-кода.
- `migrations/V012__indexes_and_fts.sql` — индексы `idx_cartridges_model`, `idx_cartridge_compat_model` + `cartridges_fts(code, location, holder_name)` (триггеры синхронизации — проверить/добавить).
- `migrations/V008__audit_log.sql` — shape audit_log (before/after/payload_json), retention отложен на Phase 7.
- `migrations/V013__devices_fts_triggers.sql` — образец FTS-триггеров (для cartridges_fts по аналогии).
- `migrations/V014__acts_indexes_and_status_codes.sql`, `migrations/V015__acts_clone_on_handover.sql` — последние миграции (V016 = следующая).

### Phase 1 carry-forward (фундамент)
- `.planning/phases/01-foundation/01-CONTEXT.md` — D-WriterChannel-01 (single-writer), D-AppError-01, D-Schema-03 (standard4 на entity tables), D-Schema-05 (audit_log shape).

### Phase 2 carry-forward (паттерны раздела/UI)
- `.planning/phases/02-ui/02-CONTEXT.md` — D-Repo-01 (hexagonal core/ports + infra/repos + app/services), D-UI-Structure-01 (feature-folders), D-UI-Transport-01, D-UI-State-01 (runes), D-UI-Validation-01, D-Bindings-01 (specta), D-AppCtx-Extension-01.
- `ui/src/features/devices/DeviceContextMenu.svelte` — паттерн контекстного меню по статусу.
- `ui/src/features/devices/DeviceFilters.svelte` — паттерн switch-bar + фильтры.

### Phase 3 carry-forward (counter, audit, master-detail, shared-компоненты)
- `.planning/phases/03-pdf/03-CONTEXT.md` — **D-Counter-Acts-01** (counter под BEGIN IMMEDIATE — образец для `cartridge_seq`), **D-Undo-01** (audit_log как источник истории), **D-Acts-List-01** (master-detail layout).
- `.planning/phases/03.1-acts-quantity-model-uat-gap-closure/03.1-CONTEXT.md` — DatePicker, PersonAutocomplete (giver/receiver), Modal backdrop-дисциплина.
- `.planning/phases/03.2-deferred-uat-gap-closure/03.2-CONTEXT.md` — LocationAutocomplete focus-open, list_grouped паттерн.

### Source files — переиспользуемые компоненты
- `ui/src/lib/components/{Modal,Input,Select,Textarea,Button,Badge,DatePicker,PersonAutocomplete,LocationAutocomplete}.svelte`
- `ui/src/features/acts/{ActsMasterDetail,ActDetail,ActsList,ActListRow,ActsSearchAndTabs}.svelte` — master-detail + switch-bar/поиск образцы.
- `ui/src/features/layout/sidebar-config.ts` — placeholder `/cartridges` (phase 4) → заменить на реальный роут.

### External (researcher fodder)
- rusqlite UNIQUE/conflict handling, FTS5 external-content triggers: https://www.sqlite.org/fts5.html#external_content_tables

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/trackly-infra/src/db/writer_worker.rs::WriterHandle::execute<F,R>` — единственный путь для writes (cartridge mutations + counter increments).
- `crates/trackly-infra/src/db/pools.rs::ReaderPool::acquire()` — reads (список, детали, поиск, low_stock).
- `crates/trackly-app/src/context.rs::AppCtx` — расширяется полем `cartridges: Arc<CartridgeService>` (паттерн D-AppCtx-Extension).
- `crates/trackly-app/src/services/act_service.rs` + `device_service.rs` — образец сервиса (service в trackly-app, repo в trackly-infra, port в trackly-core).
- `crates/trackly-core/src/ports/{acts,devices}.rs` — образец для `ports/cartridges.rs`.
- `crates/trackly-infra/src/repos/{acts_sqlite,devices_sqlite}.rs` — образец для `cartridges_sqlite.rs`.
- UI: `Modal`, `Input`, `Select`, `Textarea`, `Button`, `Badge`, `DatePicker`, `PersonAutocomplete`, `LocationAutocomplete`, `Toast`/`ToastHost` — переиспользуются.
- UI: `ActsMasterDetail`/`ActDetail` (master-detail), `DeviceContextMenu` (меню по статусу), `DeviceFilters` (switch-bar) — образцы.

### Established Patterns (Phase 1–3 locked, Phase 4 наследует)
- **Hexagonal:** core/ports + infra/repos + app/services + app/tauri_cmds + app/http.
- **Single-writer** для всех mutations + counter increments (BEGIN IMMEDIATE).
- **DTO в trackly-app, snake_case JSON; specta export `collect_commands!`** расширяется.
- **UTC unix seconds; soft-delete (`deleted_at_utc`); audit_log на все mutations.**
- **FTS5 external-content** + триггеры (как devices_fts V013) — повторить для cartridges_fts.
- **Counter table** (`cartridge_seq` уже seed'нут).
- **`locations.kind`** уже существует — использовать для склад/не-склад (НЕ добавлять is_warehouse).

### Integration Points
- `AppCtx::build` — `let cartridges = Arc::new(CartridgeService::new(writer.clone(), readers.clone(), clock.clone(), ...))`.
- `specta_export` `collect_commands![..., cartridges_*]` (list, get, create, update, delete, transition, search, counts, low_stock, models CRUD, suggest_* автокомплиты).
- axum `Router` — `http::cartridges::router()` (строится, bind — Phase 5/8).
- `tests/export_bindings.rs` — расширить DTO (CartridgeDto, CartridgeModelDto, ...).
- Sidebar: `/cartridges` placeholder → реальная страница.
- Миграция **V016** (следующая после V015): `cartridge_kinds` lookup + `cartridge_models.kind_id` + `cartridge_models.color` + `app_settings` + cartridges_fts триггеры.

### Not-yet-existing (создаём в Phase 4)
- `crates/trackly-core/src/ports/cartridges.rs`, `crates/trackly-core/src/domain/cartridges.rs`
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs`
- `crates/trackly-app/src/services/cartridge_service.rs`
- `crates/trackly-app/src/dto/cartridge.rs`
- `crates/trackly-app/src/tauri_cmds/cartridges.rs`, `crates/trackly-app/src/http/cartridges.rs`
- `crates/trackly-app/tests/cartridges_*.rs`
- `ui/src/features/cartridges/` (CartridgesPage, табы, CartridgesMasterDetail/CartridgeDetail, CartridgeList(Row), CartridgeFilters, CartridgeFormModal, OperationModal, ModelsList, ModelFormModal, CompatibilityEditor, LowStockBanner, api.ts)
- `ui/src/lib/api/cartridges.ts`
- `migrations/V016__cartridges_kind_color_settings.sql` (имя — на усмотрение планировщика)

</code_context>

<specifics>
## Specific Ideas

- **Примеры моделей (от пользователя, для фикстур/UI-понимания):**
  - `Cactus / TL-5120P` — совместимый картридж, совм. с Pantum BM5100ADN / BM5100ADW / BP5100DN / BP5100DW (вендорский артикул в скобках: CS-TL-5120P).
  - `Cactus / DL-5120` — фотобарабан, совм. с Pantum BP5100DW / BM5100ADW.
  - `Pantum / TL-5120X` — оригинальный картридж повышенной ёмкости, серии Pantum BP5100 / BM5100.
- **Цвета:** фиксированный набор Чёрный/Голубой/Пурпурный/Жёлтый/Светло-голубой/Светло-пурпурный; дефолт «Чёрный»; для фотобарабана поле скрыто.
- **Формат кода:** `C-000001` — «C-» + zero-padded до 6 цифр.
- **Дефолты заряда:** установка не меняет; возврат на склад → Пустой; забрать с заправки → Полный; все редактируемы.
- **focus-open автокомплит** во всех полях (Бренд, Модель, Расположение, ФИО) — как `LocationAutocomplete` (DEF-1).

</specifics>

<deferred>
## Deferred Ideas

- **Управление складскими локациями** (редактирование `locations.kind`) через Settings UI → **Phase 7** (SET). До тех пор в Phase 4 оба автокомплита показывают все локации.
- **Настраиваемый порог низкого остатка (SET-04)** UI-редактор → **Phase 7** (в Phase 4 — строка `app_settings`, дефолт 2).
- **Дашборд-виджет картриджей + «Динамика расхода»** → **Phase 7** (DASH-*), переиспользует `cartridges_low_stock`.
- **Связь установки с конкретным устройством-принтером (FK)** + **REQ-05** (заявка на замену → операция установки) → **Phase 6**.
- **Автокомплит совместимых принтеров из реальных принтеров БД** → **Phase 6** (сейчас — текстовые пары из ранее введённых).
- **RBAC/авторизация операций картриджей** → **Phase 5** (`audit_log.user_id` = NULL в Phase 4).
- **Server-mode bind axum-хендлеров** картриджей → **Phase 5/8** (router строится, не bind'ится).
- **Отчёты по картриджам** (Расход / Что в работе / Что на складе / История заправок) → **Phase 7** (RPT-*).

None из обсуждения не выпало за рамки фазы без учёта.

</deferred>

---

*Phase: 04-cartridges*
*Context gathered: 2026-06-07 via /gsd-discuss-phase 4*
