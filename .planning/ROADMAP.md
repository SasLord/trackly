# Roadmap: Trackly

## Overview

Trackly — портативное приложение для учёта техники, принтеров и картриджей. Дорожная карта v1 ведёт от фундамента (схема БД, портативность, дисциплина записи) через вертикальные срезы основных учётных сущностей (Устройства, Акты, Картриджи) к авторизации и серверному режиму, затем к мониторингу принтеров и заявкам сотрудников, и завершается отчётностью/дашбордом/настройками плюс AD-входом и релизным пайплайном. Каждая фаза (кроме фундаментальной) поставляет end-to-end ценность — от схемы данных и сервиса до Tauri-команд, axum-роутов (где применимо) и Svelte-UI.

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

- [x] **Phase 1: Фундамент** — Workspace, схема БД, миграции, портативность, single-writer pattern, аудит, CI с ProcMon-тестом (completed 2026-05-25)
- [x] **Phase 2: Устройства и базовый UI** — Полный CRUD устройств с автокомплитом, поиском, CSV-импортом/экспортом и навигационным каркасом приложения (completed 2026-05-27)
- [x] **Phase 3: Акты приёма-передачи и первая PDF-печать** — Акты, возвраты с под-нумерацией, архив, krilla-PDF с кириллицей, шаблоны документа приёма (completed 2026-05-30)
- [x] **Phase 4: Картриджи** — Модели и экземпляры картриджей, lifecycle, контекстные действия, баннер низкого остатка (completed 2026-06-07)
- [x] **Phase 5: Авторизация, локальные пользователи и серверный режим** — Argon2id-логин, роли, HTTPS-сервер axum, единый authorize() для обоих транспортов (completed 2026-06-13)
- [x] **Phase 6: Принтеры (SNMP-мониторинг) и Заявки** — Discovery, SNMP-опрос, Pantum hang detection (alert-only), браузер-портал заявок для сотрудников (gap-closure 06-07/06-08 закрыл дефекты заявок/discovery; human UAT 2026-06-15 — approved; status=verified, см. 06-VERIFICATION.md) (completed 2026-06-15)
- [x] **Phase 7: Отчёты, Дашборд и Настройки** — Отчёты с группировкой по месяцам, виджеты дашборда, организация/логотип/бэкапы/шаблоны (completed 2026-06-16)
- [x] **Phase 8: Релизный пайплайн (Windows/macOS/Linux)** — GitHub Actions Release matrix по push тега, NSIS + portable ZIP, .dmg, .AppImage/.deb, артефакты с SHA256-checksums, README на русском (completed 2026-06-19)

— *Milestone v1.0 завершён (фазы 1–8). Ниже — milestone v1.1 (см. `.planning/MILESTONES.md`).* —

- [ ] **Phase 9: AD-аутентификация и заявки на регистрацию пользователей** — AD-вход через браузер, подтягивание ФИО из AD, заявки на регистрацию с подтверждением админом и опциональным автоприёмом (USR-08..12, REQ-06, SET-10); вынесено из Phase 8 при SPIDR-split 2026-06-18 (planned — 5 plans, 5 waves)

## Phase Details

### Phase 1: Фундамент

**Goal:** Заложить схему БД, миграции, портативный режим, дисциплину записи и кросс-секционные инварианты так, чтобы все последующие фазы строились на надёжном основании без переделок.
**Mode:** mvp
**Depends on:** Nothing (first phase)
**Requirements:** FOUND-01, FOUND-02, FOUND-03, FOUND-04, FOUND-05, FOUND-06, FOUND-07, FOUND-08, FOUND-09, FOUND-10, FOUND-11, FOUND-12, BLD-01, BLD-06
**Success Criteria** (what must be TRUE):

  1. Запуск собранного бинарника в произвольной папке (включая путь с кириллицей `C:\Документы\Учёт\Trackly\`) создаёт `trackly.db`, `portable.txt`, `data/webview/` рядом с .exe и НЕ оставляет никаких следов в `%APPDATA%`, `%LOCALAPPDATA%`, `~/.config`, `~/Library/Application Support` (проверяется ProcMon-тестом в CI на Windows-runner'е).
  2. Concurrent-тест (50 параллельных записей через единый writer-канал из двух транспортов) проходит без ошибок «database is locked», под нагрузкой держится `busy_timeout=5000`, миграции запускаются один раз на write-пуле до открытия первого read-коннекшна.
  3. `cargo clippy -- -D warnings`, `cargo test`, `cargo fmt --check`, `pnpm svelte-check`, `pnpm lint` зелёные на каждый push в main и в PR; clippy-список `disallowed-methods` блокирует `dirs::*_dir()`, `app.path().app_data_dir()`, `chrono::Local::now()`.
  4. Открытие БД с `PRAGMA user_version`, большим текущего, завершается понятной ошибкой и НЕ повреждает файл (тест восстановления из бэкапа в CI).
  5. `tauri-specta v2` генерирует `bindings.ts` из общих DTO; один и тот же тип используется как для Tauri-invoke, так и для HTTP-транспорта (smoke-тест в `cargo test`).

**Plans:** 6/6 plans complete

### Phase 2: Устройства и базовый UI

**Goal:** Поставить end-to-end вертикальный срез по разделу «Устройства» — CRUD, автокомплиты, поиск, CSV, плюс навигационный каркас приложения с темой и русскоязычным UI.
**Mode:** mvp
**Depends on:** Phase 1
**Requirements:** DEV-01, DEV-02, DEV-03, DEV-04, DEV-05, DEV-06, DEV-07, DEV-08, DEV-09, DEV-10, DEV-11, DEV-12, DEV-13, UI-01, UI-02, UI-03, UI-04, UI-05, UI-06
**Success Criteria** (what must be TRUE):

  1. Пользователь может создать устройство (через Tauri-десктоп), заполнив только обязательные поля (Наименование, Расположение, Статус), при этом для опциональных полей (Модель, Состояние, Расположение и пр.) появляется автокомплит из ранее введённых значений. Тип устройства определяется разделом UI (раздел «Устройства» создаёт записи с внутренним type_id=1 = «Устройство»; принтеры будут добавляться в разделе «Принтеры» в Phase 6 с type_id=2).
  2. После выбора Наименования контекстный автокомплит для Модели/Состояния/Комплектации/Расположения предлагает только значения, ранее встречавшиеся с этим Наименованием.
  3. Полнотекстовый поиск по списку устройств находит совпадения по наименованию, инвентарному и серийному номерам, модели; свитч-бар фильтрует по статусу со счётчиками; не-уникальные устройства (без серийного №) группируются в табличном представлении с возможностью развернуть.
  4. Импорт CSV открывает превью первых 5 строк, корректно определяет кодировку (UTF-8 BOM / UTF-8 / CP1251) и делимитер (`,` / `;`); экспорт даёт UTF-8 BOM, открывается в русском Excel без мохибаке.
  5. Sidebar содержит указанные разделы и разделители; переключатель темы Тёмная/Светлая/Системная работает в layout (не в настройках), применяется до первого рендера без вспышки; вся видимая в фазе строка UI — на русском.
  6. При создании устройства без инвентарного и серийного номеров поле «Количество» позволяет создать от 1 до 100 однотипных устройств за одну операцию (bulk-create), каждый с отдельной записью в audit_log; при заполнении инвентарного или серийного поля — поле количества скрывается.

**Plans:** 5/5 plans complete
**UI hint:** yes

### Phase 3: Акты приёма-передачи и первая PDF-печать

**Goal:** Поставить ключевую дифференциирующую ценность продукта — акты с авто-нумерацией, частичные возвраты с под-нумерацией («N в1», «N в2»), архив, отмену с восстановлением, плюс инфраструктуру PDF с кириллицей для печати актов и документа приёма.
**Mode:** mvp
**Depends on:** Phase 2
**Requirements:** ACT-01, ACT-02, ACT-03, ACT-04, ACT-05, ACT-06, ACT-07, ACT-08, ACT-09, ACT-10, ACT-11, ACT-12, ACT-13, ACT-14, DEV-14, DEV-15
**Success Criteria** (what must be TRUE):

  1. Пользователь создаёт акт приёма-передачи, поле «№» предлагает следующий порядковый номер (атомарно в single-writer task), при необходимости можно переопределить — override пишется в `audit_log`; устройство в этом же транзакционном шаге переводится в статус «В работе» с новым Расположением.
  2. Возврат (полный или частичный) создаёт акт возврата с номером «42в», «42в1», «42в2»…; при полном возврате (все устройства вернулись) исходный акт автоматически уезжает в Архив; галочка «Применить ко всем» позволяет bulk-указать Состояние и Расположение на возврате.
  3. Удаление акта приёма-передачи возвращает все устройства в исходные Состояние и Расположение; удаление акта возврата восстанавливает значения, бывшие в момент выдачи (читается из `audit_log`).
  4. Печать/сохранение в PDF Акта приёма-передачи и Документа приёма работают: PDF содержит реальные кириллические глифы (включая «Сидоров-Петроградский Иван Александрович (ё) №42»), хэш фикстурного PDF в CI стабилен между запусками; шаблоны загружаются из БД (переезжают вместе с portable-сборкой) и редактируются (см. Phase 7), валидация при сохранении.
  5. Поиск по актам находит совпадения по номеру, ФИО Сдал/Принял, наименованию устройства; свитч-бар Акты / Возвраты / Архив показывает корректные счётчики.**Plans:** 5/5 plans complete

**Wave 1**

- [x] 03-01-PLAN.md — PDF foundation: krilla 0.7 + DejaVu Sans + MiniJinja safe-mode + DocSpec IR + CI hash fixture (структурный spike)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 03-02-PLAN.md — Acts CRUD handover: V014 indexes, core domain + ports, infra repos + atomic counter, ActService.create под single-writer, DTO + Tauri + axum + UI master-detail + create modal (ACT-01/02/03/05/13/14)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 03-03-PLAN.md — Returns + auto-archive + undo: do_return + sub_number sequencing + recompute_parent_archived + полный undo через audit_log replay + ReturnModal (ACT-06/07/08/09/10)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 03-04-PLAN.md — Templates + Org + PDF endpoints: seed дефолтных шаблонов, OrganizationService (org.json + logo traversal mitigation), ActService.render_pdf + render_acceptance_pdf, PdfPreviewModal с pdfjs-dist iframe (ACT-11/12, DEV-15)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 03-05-PLAN.md — Search + DEV-14 UI + e2e: acts search (LIKE+FTS UNION) + DocumentAcceptanceModal + DeviceContextMenu integration + e2e smoke (ACT-04, DEV-14)

**UI hint:** yes

### Phase 03.1: Acts quantity model + UAT gap closure (INSERTED)

**Goal:** Закрыть 12 UAT-выявленных gaps (G-1..G-12, G-13 deferred в Phase 8) с архитектурным переходом acts/devices model на clone-on-handover quantity-семантику. Включает: V015 schema migration с data-migration существующих qty>1 актов, ActService rewrite на device_ids per item, переписанные ReturnModal/ActFormModal (DatePicker + handover_date + person autocomplete + outstanding-only + symmetric apply_to_all), исправления PDF (column truncation + logo aspect-ratio + Save/Open/Print buttons), и cross-cutting Modal backdrop discipline + DeviceAutocomplete close-on-select.
**Requirements:** G-1, G-2, G-3, G-4, G-5, G-6, G-7, G-8a, G-8b, G-9, G-10, G-11, G-12 (G-IDs из 03-UAT.md; G-13 deferred to Phase 8)
**Depends on:** Phase 3
**Plans:** 6/6 plans complete

Plans:

- [x] 03.1-01-PLAN.md — G-12 clone-on-handover backend + V015 migration + G-7/G-11 (recompute_parent_archived COUNT-based, display rule)
- [x] 03.1-02-PLAN.md — G-5 acts.suggest_person tauri command + PersonAutocomplete.svelte shared component + 3-modal wire-up
- [x] 03.1-03-PLAN.md — G-6 + G-10 ReturnModal UX (per-row symmetric unlock + outstanding-only display, depends on 01+02)
- [x] 03.1-04-PLAN.md — G-2 DatePicker.svelte + handover_date_utc UI/backend + G-3 qty bound (depends on 01)
- [x] 03.1-05-PLAN.md — G-8a column truncation + G-8b PdfPreviewModal Save/Open/Print + G-9 logo aspect-ratio + tauri-plugin-shell
- [x] 03.1-06-PLAN.md — G-1 Modal backdrop mousedown/mouseup discipline + G-4 DeviceAutocomplete close-on-select

### Phase 03.2: Deferred UAT gap closure (INSERTED)

**Goal:** Закрыть 3 отложенных UAT-пункта (DEF-1, DEF-2, DEF-3) из round-3 ручного UAT Phase 03.1 (03.1-DEFERRED-UAT-ITEMS.md) перед финальным merge Phase 03. Архитектурные решения зафиксированы: DEF-2B → Вариант 2 (sub-group DeviceGroup по (name, model, condition)); DEF-3 → Вариант A (ActService::create пишет resolved devices.location_id при handover, restore на return).
**Requirements:** DEF-1, DEF-2A, DEF-2B, DEF-3 (из 03.1-DEFERRED-UAT-ITEMS.md)
**Depends on:** Phase 03.1
**Plans:** 2/2 plans complete

**Wave 1**

- [x] 03.2-01-PLAN.md — DEF-1 (focus-open PersonAutocomplete + DeviceAutocompleteField) + DEF-2A (dedupe выбранных groups в ActFormItemsTable dropdown)

**Wave 2** *(blocked on Wave 1 — общий файл ActFormItemsTable.svelte)*

- [x] 03.2-02-PLAN.md — DEF-2B (list_grouped GROUP BY condition + NULL-safe repr JOIN + UI tooltip) + DEF-3 (handover UPDATE devices.location_id = resolved_location_id)

**Follow-up:** После 03.2 — финальный human UAT остальных pending-пунктов 03.1-HUMAN-UAT.md (13 items) перед merge Phase 03.

### Phase 03.3: Device-list UX round 2 (UAT follow-up на 03.2) (INSERTED)

**Goal:** Закрыть 4 пункта ручного UAT после 03.2, касающихся ТОЛЬКО списка устройств в разделе «Устройства» (см. 03.3-UAT-ITEMS.md). Ключевое архитектурное решение: `devices.listGrouped` сейчас вызывается и страницей «Устройства», и автокомплитом акт-формы — DEF-2B разбивку по Состоянию надо ОТКЛЮЧИТЬ для списка устройств, но СОХРАНИТЬ для акт-формы (развести два вызова — параметр у DeviceFilter/list_grouped либо отдельный путь).
**Requirements:** ITEM-1, ITEM-2, ITEM-3, ITEM-4 (из 03.3-UAT-ITEMS.md)
**Depends on:** Phase 03.2
**Plans:** 2/2 plans complete
Plans:
**Wave 1**

- [x] 03.3-01-PLAN.md — Бэкенд: флаг group_by_condition в DeviceFilter + условный SQL list_grouped + condition_distinct_count + интеграционные тесты + регенерация TS-биндингов (ITEM-1)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 03.3-02-PLAN.md — Фронтенд: колонка «Состояние» + «разное» + tooltip + скрытие «Статус» по вкладке + автокомплит «Все расположения» + HTTP route locations_autocomplete (ITEM-1/2/3/4)

### Phase 4: Картриджи

**Goal:** Поставить раздел «Картриджи» — модели с матрицей совместимости, экземпляры с авто-кодом `C-000001`, lifecycle с контекстными действиями, журнал перемещений и баннер низкого остатка.
**Mode:** mvp
**Depends on:** Phase 3 (использует тот же паттерн single-writer counters, что и ACT-14, и `audit_log` из Phase 1)
**Requirements:** CART-01, CART-02, CART-03, CART-04, CART-05, CART-06, CART-07, CART-08, CART-09, CART-10, CART-11, CART-12
**Success Criteria** (what must be TRUE):

  1. Пользователь создаёт модель картриджа с матрицей совместимых принтеров (массив пар «Бренд+Модель» с автокомплитом ранее введённых значений), затем создаёт экземпляр — код `C-000001` генерируется автоматически, но можно ввести свой (например, штрих-код); номер из счётчика не теряется при коллизии.
  2. Свитч-бар по статусу (Все / На складе / В работе / На заправке / Списано) показывает корректные счётчики; контекстное меню картриджа меняется в зависимости от статуса (на «На складе» — Установить в принтер / Отправить на заправку / Списать / Удалить; на «В работе» — Вернуть на склад / Удалить; и т.д.).
  3. Установка картриджа в принтер запрашивает Дату, ФИО «Кто выдал», ФИО «Кому выдал», Расположение (автокомплит «не на складе»); возврат на склад запрашивает Состояние заряда (по умолчанию Пустой), Расположение (автокомплит «на складе») и Примечания; передача/возврат с заправки — аналогично; вся история операций видна в карточке экземпляра как хронологический список из `audit_log`.
  4. Поиск по картриджам находит совпадения по коду, модели, расположению.
  5. Когда количество картриджей со статусом «На складе» + зарядом «Полный» по конкретной модели опускается ниже настроенного порога (см. SET-04 в Phase 7), в разделе «Картриджи» отображается баннер «низкий остаток» с указанием модели и текущего количества.

**Plans:** 6/6 plans complete

**Wave 1** *(параллельно)*

- [x] 04-01-PLAN.md — V016 миграция (cartridge_kinds + color + app_settings + FTS-триггеры) + test_db assertion 15→16 + 6 тестовых скаффолдов RED (CART-03/04/05/06/07/08/09/10/11/12)
- [x] 04-02-PLAN.md — Hexagonal слой: domain structs (CartridgeRow, CartridgeModelRow, TransitionOp, Filter, Counts, LowStockItem) + CartridgeRepository port + SqliteCartridgeRepository + assign_code_in_tx + transition_in_tx + FTS search + low_stock (CART-03/04/06/07/08/09/10/11/12)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 04-03-PLAN.md — App-слой: CartridgeDto + CartridgeTransitionPayload + CartridgeService (write/read/transition/low_stock/model_*) + Tauri commands + HTTP router (строится, не bind'ится) + AppCtx wire-up + specta export + GREEN тесты + bindings.ts регенерация (CART-01..12)

**Wave 3** *(параллельно, blocked on Wave 2)*

- [x] 04-04-PLAN.md — UI skeleton: api.ts + CartridgesPage + CartridgesSearchAndTabs + CartridgesMasterDetail + CartridgesList + CartridgeListRow + CartridgeDetail + CartridgeFilters + sidebar activation (CART-03/04/05/10/11)
- [x] 04-05-PLAN.md — UI lifecycle: CartridgeContextMenu (portal + status-dependent) + OperationModal (5 ops) + CartridgeFormModal + LowStockBanner (CART-06/07/08/09/12)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 04-06-PLAN.md — Models UI + финальный wire-up: ModelsList + ModelListRow + ModelFormModal + CompatibilityEditor + CartridgesPage полная интеграция всех компонентов + human-verify checkpoint (CART-01/02/03..12 end-to-end)

**UI hint:** yes

### Phase 5: Авторизация, локальные пользователи и серверный режим

**Goal:** Включить локальную аутентификацию (argon2id), три роли, HTTPS-сервер axum для доступа из браузера в LAN, единый `authorize()` для обоих транспортов; десктоп остаётся unlocked-by-default с опциональным локом.
**Mode:** mvp
**Depends on:** Phase 4 (нужна стабильная схема всех сущностей, чтобы накрыть их authorize-проверками)
**Requirements:** USR-01, USR-02, USR-03, USR-04, USR-05, USR-06, USR-07, SRV-01, SRV-02, SRV-03, SRV-04, SRV-05, SET-08
**Success Criteria** (what must be TRUE):

  1. Администратор в десктоп-приложении создаёт локальных пользователей с логином/ФИО/паролем/ролью; пароль хранится только как argon2id-хэш через `Secret<String>` (в логах и `Debug`-выводе — `***`); опционально включает «требовать вход в десктопе».
  2. Администратор переключает сервер-режим в Настройках → Сеть (порт, bind-адрес `127.0.0.1` или `0.0.0.0`); при первом включении автоматически генерируется self-signed сертификат через `rcgen`, путь к собственному сертификату конфигурируется; HTTP-listener отсутствует — только HTTPS.
  3. Сотрудник с ролью «Сотрудник» входит через браузер по логину/паролю, видит только разрешённые разделы (UI-уровень), а при попытке через `curl` дёрнуть mutation-эндпоинт устройств/актов/картриджей получает 403 (тест role × endpoint в CI).
  4. В веб-режиме сотрудник может выйти и войти под другим пользователем; сессия живёт в cookie (`tower-sessions` с rusqlite-store), переживает рестарт сервера, отзывается на logout.
  5. Корректное завершение приложения останавливает axum-сервер (drain in-flight requests, taskTracker.close().await), не оставляет «висящих» портов.

**Plans:** 6/6 plans complete

**Wave 1**

- [x] 05-01-PLAN.md — trackly-core::auth domain types (Identity/Role/Action/authorize()) + auth DTOs + V018 migration + 8 RED test scaffolds

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 05-02-PLAN.md — AuthService (argon2id CRUD + needs_bootstrap + desktop_identity) + RusqliteSessionStore + TLS/rcgen + server/mod.rs start_server + AppCtx wiring

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 05-03-PLAN.md — HTTP auth/users/settings routers + build_router() (session middleware + security headers + rate-limit) + Tauri commands + specta export + main.rs server boot

**Wave 4** *(параллельно, blocked on Wave 3)*

- [x] 05-04-PLAN.md — RBAC enforcement on devices/acts/cartridges HTTP handlers + Tauri commands + role×endpoint CI test matrix (ROADMAP criterion #3)
- [x] 05-05-PLAN.md — UI: auth store + App.svelte bootstrap guard + LoginPage + FirstRunWizard + UsersPage CRUD + NetworkSettings + sidebar role filter + human-verify checkpoint

**UI hint:** yes

### Phase 6: Принтеры (SNMP-мониторинг) и Заявки

**Goal:** Включить SNMP-мониторинг сетевых принтеров (Pantum/Kyocera/HP/Canon), discovery подсети, детекцию Pantum-зависания (alert-only), плюс портал заявок для сотрудников с двумя типами и жизненным циклом.
**Mode:** mvp
**Depends on:** Phase 5 (заявки требуют веб-аутентификации и роли «Сотрудник»; принтер связан с устройством типа «Принтер» из Phase 2)
**Requirements:** PRN-01, PRN-02, PRN-03, PRN-04, PRN-05, PRN-06, PRN-07, PRN-08, REQ-01, REQ-02, REQ-03, REQ-04, REQ-05, REQ-07
**Success Criteria** (what must be TRUE):

  1. Администратор запускает discovery по диапазону IP, система находит принтеры через SNMP, определяет производителя и модель, заводит их как устройства типа «Принтер» (если ещё не заведены); SNMP community string хранится как `Secret<String>`.
  2. На карточке принтера видны уровни тонера/чернил, статус печати, страничные счётчики; для Pantum BM5100ADN / Kyocera ECOSYS / HP LaserJet / Canon iR используются производитель-специфичные OID, для прочих — RFC 3805 fallback; история статусов сохраняется для отчётности.
  3. При зависании Pantum-спулера (стабильный `prtMarkerLifeCount` + растущая очередь спулера, проверяется host-side механизмом по итогам spike) система показывает in-app алерт администратору; авто-restart НЕ выполняется (отложено в v2 → PNT).
  4. Сотрудник через браузер создаёт заявку одного из двух типов — «Замена картриджа» (со связью к принтеру и модели картриджа в БД) или «Свободная форма» (произвольный текст); специалист видит in-app уведомление о новой заявке.
  5. Специалист переводит заявку «Создана» → «Принять в работу» → «Выполнить» или «Отклонить»; при выполнении заявки на замену картриджа можно сразу запустить операцию установки картриджа (CART-07) из контекста заявки; история заявок и их статусов доступна для просмотра.

**Plans:** 9/9 plans complete

Plans:

- [x] 06-01-PLAN.md — V020-V024 миграции + snmp2 + SnmpClient trait + MockSnmpClient (PRN-01/03/04/08)
- [x] 06-02-PLAN.md — Repositories + PrinterService (poll loop, discovery, alerts, retention) + RequestService (lifecycle, optimistic lock) + WsEvent (PRN-01..08, REQ-01..04)
- [x] 06-03-PLAN.md — Tauri commands + axum HTTP handlers + WebSocket /api/v1/ws + AppCtx wire-up (PRN-01..08, REQ-01..05/07)
- [x] 06-04-PLAN.md — Прinters UI: api.ts + ws.ts + 11 компонентов (TonerGauge, DiscoveryModal, PrinterAlertBanner) (PRN-01..08)
- [x] 06-05-PLAN.md — Requests UI: api.ts + 8 компонентов (RequestFormModal, RequestDetail, OperationModal REQ-05 link) (REQ-01..05/07)
- [x] 06-06-PLAN.md — bindings.ts + nav + cargo check + smoke test checkpoint (все PRN + REQ)
- [x] 06-07-PLAN.md — Gap-closure (заявки): arg-key `dto` parity + requests_counts rename + requests_get_history (REQ-07) + a11y tablist + ролевой рендер (REQ-01/02/07)
- [x] 06-08-PLAN.md — Gap-closure (принтеры): реализовать discovery admit + ручное «Завести принтер» + select замены картриджа из devices type=Принтер (PRN-01/04, REQ-02)

**UI hint:** yes

### Phase 7: Отчёты, Дашборд и Настройки

**Goal:** Поставить отчётный слой (Устройства, Картриджи), виджеты дашборда и раздел Настройки (организация, логотип, порог низкого остатка, путь БД, бэкапы, редактирование шаблонов документов).
**Mode:** mvp
**Depends on:** Phase 6 (отчёты и дашборд тянут данные из всех учётных доменов)
**Requirements:** RPT-01, RPT-02, RPT-03, RPT-04, RPT-05, RPT-06, RPT-07, RPT-08, DASH-01, DASH-02, DASH-03, DASH-04, DASH-05, SET-01, SET-02, SET-03, SET-04, SET-05, SET-06, SET-07, SET-09
**Success Criteria** (what must be TRUE):

  1. Пользователь открывает отчёт «Акты приёма-передачи» / «Возвраты» / «Что в работе» / «Что на складе» по устройствам и «Расход» / «Что в работе» / «Что на складе» / «История заправок» по картриджам; выбирает период (месяц / год / диапазон) — границы периода корректно считаются в TZ организации (UTC в БД, форматирование через chrono-tz); список группируется по месяцам с визуальным разделителем «Сентябрь 2026».
  2. Внутри отчёта работают фильтры (по локации, типу, статусу, модели) и поиск; экспорт в CSV (UTF-8 BOM, `;`-делимитер) и в PDF (через инфраструктуру из Phase 3) скачивается без ошибок; печать через системный диалог открывается корректно.
  3. На дашборде отображаются виджеты «Устройства» (общее + по статусам), «Картриджи» (по статусам + alert о низком остатке), «Динамика расхода картриджей» (график за 3/6/12 месяцев), «Заявки» (активные/новые/выполненные за период), «Принтеры» (онлайн/офлайн, проблемные).
  4. Администратор в Настройках задаёт данные организации (название, реквизиты, адрес) и загружает логотип (хранится как BLOB в БД, переезжает с portable-сборкой); логотип появляется в шапке актов и документа приёма; настраивает порог низкого остатка (по умолчанию 2); открывает папку с БД и при необходимости меняет расположение (с проверкой запрета на SMB-шары).
  5. Ручной бэкап одним кликом через `rusqlite::backup::Backup` (НЕ `fs::copy`); автобэкап по расписанию (ежедневно/еженедельно) с настраиваемой ретенцией; integrity_check на бэкапе после записи; редактируемые MiniJinja-шаблоны Акта и Документа приёма сохраняются с валидацией.

**Plans:** 14/14 plans complete

**Wave 1**

- [x] 07-01-PLAN.md — Foundation: V026 org_settings migration + Phase 7 DTOs + 9 RED test scaffolds

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 07-02-PLAN.md — Settings backend: OrgDbService + BackupService + Supervisor + TemplateService extension + DocSpec logo support (Wave 2, parallel)
- [x] 07-03-PLAN.md — Reports + Dashboard backend: ReportService (8 query methods + CSV/PDF export) + DashboardService (5 widgets + chart) (Wave 2, parallel)

**Wave 3** *(blocked on Wave 2)*

- [x] 07-04-PLAN.md — Settings UI: OrgSettings + StorageSettings + BackupSettings + ThresholdSettings + TemplateEditor (Wave 3, parallel)
- [x] 07-05-PLAN.md — Dashboard UI: DashboardPage + StatWidget + ChartWidget (SVG, zero npm deps) + PeriodToggle (Wave 3, parallel)
- [x] 07-06-PLAN.md — Reports UI: ReportsPage + ReportSubNav + PeriodSelector + ReportTable + ReportFilters (Wave 3, parallel)

**Wave 4** *(blocked on Wave 3)*

- [x] 07-07-PLAN.md — Wire-up: AppCtx extension + Tauri commands + axum routes + bindings.ts + human-verify checkpoint

**Gap-Closure Wave (Wave 1 relative, parallel — 12 functional/UX gaps from human verify)**

- [x] 07-08-PLAN.md — Backend gaps: consumption chart runtime error (GAP-D1) + template preview undefined context (GAP-S6)
- [x] 07-09-PLAN.md — Settings component fixes: DB path load + Tauri detection + threshold load + styling (GAP-S3, GAP-S4, GAP-S5)
- [x] 07-10-PLAN.md — Reports frontend: export arg fix + switch-bar row layout + date range styling + filter cleanup + badges (GAP-R1..R5)
- [x] 07-11-PLAN.md — Settings UX: section spacing + sub-section switch-bar (GAP-S1, GAP-S2)

**Gap-Closure Round-2 Wave (G2-1..G2-5 — 5 runtime gaps from human re-verify 2026-06-17)**

- [x] 07-12-PLAN.md — Backend: settings_open_db_folder command + expanded validate_preview demo_ctx for act_acceptance (G2-2 backend, G2-4)
- [x] 07-13-PLAN.md — Frontend: OrgSettings logo detection fix (G2-1) + StorageSettings command rename (G2-2 frontend) + BackupSettings arg wrapping (G2-3)
- [x] 07-14-PLAN.md — Reports: controls-row flush-right alignment + real per-tab status counts via new reports_get_report_counts command (G2-5a + G2-5b)

**UI hint:** yes

### Phase 8: Релизный пайплайн (Windows/macOS/Linux)

**Goal:** As a мейнтейнер Trackly, I want to собирать релизы в GitHub Actions для Windows (приоритет), macOS и Linux по push тега, so that пользователи получают готовые артефакты с checksums для своей ОС.
**Mode:** mvp
**Depends on:** Phase 7 (нужна стабильная база перед release-пайплайном)
**Requirements:** BLD-02, BLD-03, BLD-04, BLD-05
**Success Criteria** (what must be TRUE):

  1. При push-тега `v*.*.*` GitHub Actions Release собирает: Windows 64-bit (NSIS installer + portable ZIP с маркером `portable.txt` и без updater'а), macOS aarch64 (.dmg), Linux x86_64 (.AppImage + .deb); артефакты содержат SHA256-checksums и, где возможно, подписи.
  2. README.md на русском содержит инструкции по запуску для каждой ОС, включая portable-режим, требования к WebView2 на Windows и описание серверного режима с подсказками по доверию self-signed сертификату в локальной сети.

**Plans:** 2/2 plans complete
**Wave 1**

- [x] 08-01-PLAN.md — Tauri bundle config (active:true, icons, macOS ad-hoc), README.md (BLD-05), portable ZIP staging files

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 08-02-PLAN.md — GitHub Actions release.yml: three-job pipeline (create-release → build matrix → checksums), portable ZIP assembly, SHA256SUMS (BLD-02, BLD-03, BLD-04)

### Phase 9: AD-аутентификация и заявки на регистрацию пользователей

**Goal:** Включить вход доменных пользователей через Active Directory (вынесено из Phase 8 при SPIDR-split 2026-06-18, чтобы тестировать на реальной Windows-машине в домене после релизного пайплайна): AD-логин через браузер, подтягивание ФИО из AD, заявки на регистрацию незарегистрированных AD-пользователей с подтверждением администратором и опциональным автоприёмом. Пароли AD НИКОГДА не сохраняются. Цель — авто-SSO, а не только `simple_bind` (см. память проекта `phase8_split_ad_sso`).
**Mode:** mvp
**Depends on:** Phase 8 (релизная Windows-сборка нужна для теста AD-входа в домене)
**Requirements:** USR-08, USR-09, USR-10, USR-11, USR-12, REQ-06, SET-10
**Plans:** 5 plans

Plans:

**Wave 1**

- [ ] 09-01-PLAN.md — AdClient port (trait + AuthOutcome, I/O-free core) + RealAdClient/MockAdClient/discovery (mirror SNMP triad) + AdConfig + ldap3/hickory deps; Wave 0 mock/empty-password/filter-escape/base-DN tests (USR-12)

**Wave 2** *(blocked on Wave 1)*

- [ ] 09-02-PLAN.md — AuthService local→AD login fallback (constant-time preserved) + find_user_any_state + ad_* app_settings readers + AppCtx mock/real switch + V028 ad_subtype migration (USR-08, USR-10)

**Wave 3** *(blocked on Wave 2)*

- [ ] 09-03-PLAN.md — Registration/restoration write paths: auto-accept vs pending modes + ad_register admin-only filter + approve-with-role + reject branching + restoration (USR-09, USR-11, SET-10, REQ-06)

**Wave 4** *(blocked on Wave 3)*

- [ ] 09-04-PLAN.md — DTO + transports: LoginRequest.remember + cookie policy + AdSettingsDto/approve DTO + axum & Tauri endpoints + bindings-phase9.ts (USR-08, USR-11, SET-10, REQ-06)

**Wave 5** *(blocked on Wave 4)*

- [ ] 09-05-PLAN.md — UI vertical: login redesign (remember/hint/generic errors/reserved SSO) + Pending/Blocked screens + Active Directory settings tab + admin ad_register approve UI + docs/AD-SETUP.md + human-verify (USR-08/09/10/11, SET-10, REQ-06)

**UI hint:** yes

## Progress

**Execution Order:**
Phases execute sequentially: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Фундамент | 6/6 | Complete   | 2026-05-25 |
| 2. Устройства и базовый UI | 5/5 | Complete    | 2026-05-28 |
| 3. Акты приёма-передачи и первая PDF-печать | 5/5 | Complete   | 2026-05-30 |
| 4. Картриджи | 6/6 | Complete    | 2026-06-12 |
| 5. Авторизация, локальные пользователи и серверный режим | 6/6 | Complete    | 2026-06-14 |
| 6. Принтеры (SNMP-мониторинг) и Заявки | 9/9 | Complete   | 2026-06-15 |
| 7. Отчёты, Дашборд и Настройки | 14/14 | Complete    | 2026-06-18 |
| 8. Релизный пайплайн (Windows/macOS/Linux) | 2/2 | Complete    | 2026-06-19 |
| 9. AD-аутентификация и заявки на регистрацию пользователей | 0/0 | Not planned | — |

## Coverage

- **v1 requirements mapped:** 120 / 120 ✓
- **v2 requirements:** вынесены за пределы roadmap (MAP, NTF-02/03/04, PNT, WIN7, I18N, ADV)
- **Orphans:** none

## Out of v1 Roadmap (Deferred to v2)

| Category | Reason |
|----------|--------|
| MAP-01..04 (Карта помещений) | Высокая UI-сложность; ценность учёта не зависит от карты — отложено в v2 milestone |
| NTF-02 (SMTP), NTF-03 (Telegram), NTF-04 (Webhook), NTF-05 (event subscriptions) | In-app часть покрыта REQ-04 в Phase 6; внешние каналы — финальная фаза v2 |
| PNT-01..04 (Pantum auto-restart) | В v1 — только детекция и алерт (PRN-06); авто-restart требует подтверждённой гипотезы и безопасного механизма (v2) |
| WIN7-01..02 (Windows 7 32-bit) | Best-effort; MSRV `krilla` 1.92 + WebView2 TLS 1.2 могут закрыть дверь — отдельный spike в v2 |
| I18N-01..03 (Английская локализация) | Команда и пользователи русскоязычные; добавляется без архитектурных переделок |
| ADV-01..05 (SSO/REST API наружу/Signature pad/доп. вендоры принтеров/Postgres) | Преждевременная сложность для текущего масштаба |
