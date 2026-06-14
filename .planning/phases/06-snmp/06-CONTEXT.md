# Phase 6: Принтеры (SNMP-мониторинг) и Заявки - Context

**Gathered:** 2026-06-14
**Status:** Ready for planning
**Source:** /gsd-discuss-phase 6 — 4 области обсуждены интерактивно (Discovery+опрос SNMP, Схема принтера+история, Pantum-детекция, Портал заявок), плюс доп. раунд (USB PRN-04, ретенция, детали discovery, scope WebSocket). Два расхождения с рекомендациями зафиксированы как осознанный выбор (OID-профили data-driven, уведомления через WebSocket). Одно частичное изменение трактовки ROADMAP (Pantum-эвристика → v2).

<domain>
## Phase Boundary

Два связанных под-домена, end-to-end:

1. **SNMP-мониторинг сетевых принтеров (PRN-01..08):** discovery подсети по диапазону IP (review перед добавлением), фоновый опрос тонера/чернил/статуса/страничных счётчиков, data-driven OID-профили (Pantum/Kyocera/HP/Canon + RFC3805 fallback), история уровней/статусов для отчётности, генеричная инфраструктура in-app алертов по проблемным состояниям (offline/error), mock SNMP-клиент для dev на macOS.
2. **Портал заявок для сотрудников (REQ-01..05, REQ-07):** два типа заявок («Замена картриджа», «Свободная форма» с категориями), жизненный цикл Создана→В работе→Выполнить/Отклонить, in-app уведомление специалисту/админу через WebSocket, связь заявки на замену картриджа с операцией установки CART-07, история заявок.

**В scope:**
- **Discovery (PRN-01):** скан диапазона IP по SNMP v2c (community настраивается, дефолт 'public'); список найденных (IP, vendor, модель, sysName) с review-перед-добавлением; галочки «завести как Принтер»; дубликаты (по IP/serial) помечаются; определение vendor/модели по sysObjectID + парсинг sysDescr → маппинг на OID-профиль из БД; параллельный скан с timeout.
- **Опрос (PRN-02, PRN-03):** фоновый tokio-task опрашивает все принтеры по интервалу (настраивается в app_settings) + кнопка «Обновить сейчас» на карточке; vendor-специфичные OID из data-driven профилей, RFC3805 fallback для прочих.
- **Схема принтера:** новая таблица `printers` (FK→devices, ip_address, community `Secret`, snmp_version, vendor, oid_profile_id, last_seen, …); миграция V020+.
- **История (PRN-05):** отдельная таблица snapshot'ов (`printer_readings`: printer_id, ts_utc, toner_levels JSON, page_count, status, …) — одна строка на опрос; прореживание + retention (настройка в app_settings, фоновый prune).
- **OID-профили (PRN-03):** data-driven таблица OID-профилей, засеянная миграцией для Pantum BM5100ADN / Kyocera ECOSYS / HP LaserJet / Canon iR + RFC3805 fallback-профиль.
- **Алерты (часть PRN-06):** генеричная инфраструктура in-app алертов админу (бэйдж/индикатор на карточке + в списке принтеров), один активный алерт на принтер (dedup), persist до разрешения/acknowledge; срабатывают на базовых проблемных SNMP-состояниях (offline/error). Питает виджет DASH-05 «проблемные принтеры» (Phase 7).
- **Mock SNMP (PRN-08):** порт `SnmpClient` (trait) в trackly-core; real (snmp2) + mock (фикстурные детерминированные ответы); переключение через config/env; mock умеет симулировать проблемные состояния для UI/тестов.
- **USB-принтеры (PRN-04) — только учёт:** возможность пометить принтер как USB-подключённый к рабочей станции (связь с device-компьютером), без SNMP/автоопроса. Механизм опроса USB (агент/WMI) — Phase 8 spike.
- **Заявки (REQ-01..03, REQ-07):** наполнение портала `/requests` (placeholder из Phase 5). Сотрудник создаёт заявку из браузера; два типа; жизненный цикл с переходами на стороне специалиста; история заявок и статусов. Таблица `requests` (V006) уже существует.
- **Замена картриджа (REQ-02):** сотрудник указывает **только принтер** (dropdown из devices type=Принтер) + комментарий; модель картриджа опциональна — определяется специалистом при выполнении.
- **Свободная форма (REQ-02):** произвольный текст + опциональная категория из фиксированного набора: «Ремонт техники», «Расходные материалы», «Программное обеспечение», «Прочее».
- **Уведомления (REQ-04):** in-app о новой заявке (и смене статуса) специалисту/админу через WebSocket push.
- **REQ-05:** на заявке «Замена картриджа» при выполнении кнопка «Установить картридж» открывает существующую `OperationModal` (CART-07), pre-filled принтером (и моделью если задана); успешная установка переводит заявку в «Выполнена».
- **Связь картриджа с принтером (PRN-07):** отображение «какой картридж сейчас стоит» — связь установки CART-07 с конкретным устройством-принтером (FK), которая в Phase 4 была отложена в Phase 6.

**НЕ в scope этой фазы (явно deferred):**
- **Pantum-специфичная эвристика зависания** (стабильный `prtMarkerLifeCount` + растущая очередь спулера через SNMP job-table) → **v2 (PNT)**. ⚠️ Это частичное изменение трактовки ROADMAP Phase 6 success criterion #3: в Phase 6 строится только генеричный alert-каркас, конкретная hang-детекция переносится. Авто-restart также v2 (как и было в ROADMAP).
- **Host-side Windows print spooler** (WMI/Get-PrintJob) как источник сигнала очереди → не в v1 (выбран SNMP job-table источник на будущее, измеряется в v2).
- **USB-механизм опроса** (агент/WMI/RPC) → Phase 8 spike (PRN-04 в Phase 6 — только учёт связи).
- **Email/Telegram/Webhook уведомления** (NTF-02..05) → финальная фаза v2 (in-app часть — здесь, REQ-04).
- **Заявка на регистрацию AD-пользователя** (REQ-06, подтип `ad_register`) → Phase 8.
- **Виджет дашборда «Принтеры»** (DASH-05) и отчёты по принтерам → Phase 7 (Phase 6 даёт данные/команды).
- **UI-редактор OID-профилей и настроек discovery** → если потребуется, Phase 7 Settings (в Phase 6 профили засеяны миграцией, настройки — строки app_settings).
- **Управление складскими локациями / порог низкого остатка UI** — остаётся Phase 7 (не трогаем).

**Mode:** mvp — вертикальные слайсы: (а) принтеры: UI → tauri cmd (+axum route) → PrinterService/SnmpClient → repos → DB; (б) заявки: UI (вкл. браузер-портал) → tauri cmd/axum + WebSocket → RequestService → requests repo → DB.

</domain>

<decisions>
## Implementation Decisions

### Discovery и опрос SNMP

#### D-Discovery-01: review перед добавлением
- Скан диапазона IP показывает список найденных принтеров (IP, vendor, модель, sysName); админ галочками выбирает кого завести как устройство «Принтер» (type_id=2).
- Дубликаты (уже заведённые по IP или serial) помечаются в списке, не дублируются в БД.
- Реализует PRN-01 success criterion #1 («заводит их как устройства типа Принтер, если ещё не заведены»).

#### D-Discovery-02: v2c + community, идентификация по sysObjectID/sysDescr
- Discovery по SNMP v2c с настраиваемым community (дефолт 'public').
- vendor/модель определяются по `sysObjectID` + парсинг `sysDescr`, маппятся на OID-профиль из БД (D-OID-01).
- Параллельный скан с timeout на хост. Конкретные числа (concurrency/timeout) — researcher/planner в рамках snmp2.

#### D-Poll-01: фоновый интервал + кнопка «Обновить»
- Отдельный tokio-task опрашивает все принтеры по интервалу (настройка в app_settings); карточка всегда показывает свежие данные; кнопка «Обновить сейчас» для on-demand.
- Опрос пишет snapshot в `printer_readings` (D-History-01).

#### D-Arch-01: SNMP в отдельном task, запись через single-writer
- Сами SNMP-запросы (сетевой I/O) идут в отдельном tokio-task/пуле вне БД; снимки (уровни/статус/счётчики) и обновления `printers.last_seen` пишутся в БД через **единый single-writer** (как все mutations). Не блокирует пользовательские записи.

### Mock SNMP (PRN-08)

#### D-Mock-01: trait + две реализации
- Порт `SnmpClient` (trait) в trackly-core; реализации: real (snmp2) в trackly-infra + mock (фикстурные детерминированные ответы).
- Переключение через config/env (runtime, не cargo feature) — mock даёт стабильные принтеры для UI и тестов, умеет симулировать проблемные/offline состояния.

### Схема принтера и история

#### D-Schema-01: новая таблица `printers` (FK→devices)
- `printers(device_id FK → devices, ip_address, community Secret, snmp_version, vendor, oid_profile_id FK, last_seen_utc, …)` — чистое разделение: device = учётная сущность, printer = SNMP-метаданные. НЕ раздувать `devices` nullable-колонками.
- community хранится как `Secret<String>` (ROADMAP success criterion #1, паттерн Phase 5).
- USB-учёт (PRN-04): признак/связь, что принтер USB-подключён к рабочей станции (device-компьютеру) — без SNMP-полей; точная форма (флаг + FK на host-device) — планировщику.

#### D-History-01: отдельная таблица snapshot'ов `printer_readings`
- Одна строка на опрос: `printer_id, ts_utc, toner_levels (JSON), page_count, status, …`. Оптимально для time-series графиков (Phase 7 динамика расхода) и сравнения значений во времени.

#### D-Retention-01: прореживание + retention
- Хранить частые замеры N дней, дальше downsample (напр. 1/день) или удалять старше retention. Настройка в app_settings, фоновый prune. Сдерживает рост portable-БД. Конкретные значения/стратегия — планировщику.

#### D-OID-01: data-driven OID-профили в БД *(расхождение с рекомендацией — осознанный выбор пользователя)*
- Таблица OID-профилей, засеянная миграцией для Pantum BM5100ADN / Kyocera ECOSYS / HP LaserJet / Canon iR + RFC3805 fallback-профиль.
- Гибче для добавления моделей без пересборки. Рекомендовался hardcoded-вариант (проще для 4 вендоров v1), но пользователь выбрал data-driven — закладываем таблицу профилей + связь `printers.oid_profile_id`.
- UI-редактор профилей — НЕ в scope (Phase 7 при необходимости); в Phase 6 профили только засеяны.

#### D-Settings-01: настройки discovery/опроса в app_settings
- Диапазон IP, интервал опроса, community по умолчанию, retention — строки в существующей таблице `app_settings` (как low_stock_threshold). Переезжают с portable-БД, редактируемы из UI. community — Secret.

### Pantum-детекция и алерты

#### D-Pantum-01: hang-эвристика отложена в v2, alert-каркас — сейчас *(частичное изменение трактовки ROADMAP #3)*
- В Phase 6 НЕ реализуется конкретная Pantum-эвристика зависания (стабильный `prtMarkerLifeCount` + растущая очередь спулера). Она переносится в **v2 (PNT)**; источник сигнала на будущее — **SNMP job-table принтера** (prtJobEntry / Job-MIB / hrDeviceStatus), НЕ host-side Windows spooler.
- В Phase 6 строится **генеричная инфраструктура алертов** на базовых проблемных SNMP-состояниях (offline/error из статуса). Это закрывает alert-часть и питает DASH-05.
- ⚠️ Планировщику/верификатору: ROADMAP Phase 6 success criterion #3 трактуется как «alert-каркас существует»; конкретная hang-детекция помечена deferred. При планировании отразить это в боковых заметках/SUMMARY; при необходимости обновить ROADMAP/REQUIREMENTS (PRN-06 частично → PNT v2).

#### D-Alert-01: in-app админу, persist до clear, dedup на принтер
- In-app алерт только админу (бэйдж/индикатор на карточке принтера + в списке). Один активный алерт на принтер (dedup), держится пока состояние не разрешится или не acknowledged. Хранится в БД (таблица алертов или статусное поле — планировщику).

### Портал заявок и жизненный цикл

#### D-Req-Form-01: замена картриджа — обязателен только принтер, модель опц.
- Сотрудник в заявке «Замена картриджа» указывает **только принтер** (dropdown из devices type=Принтер) + комментарий; `cartridge_model_id` опционален — определяется специалистом при выполнении (проще для нетех-сотрудника).
- Это меняет ранее предполагавшийся в Phase 4 вариант «принтер→модель по совместимости»: модель не выбирается сотрудником на этапе создания.

#### D-Req-Categories-01: свободная форма — фиксированный набор категорий
- Опциональная категория из фиксированного набора: «Ремонт техники», «Расходные материалы», «Программное обеспечение», «Прочее». Хранение (lookup-таблица vs CHECK-enum) — планировщику; редактирование набора через UI — отложено (Phase 7 при необходимости).

#### D-Notify-01: WebSocket push *(расхождение с рекомендацией — осознанный выбор пользователя)*
- Уведомление о новой заявке (и смене статуса) специалисту/админу через **WebSocket push** (рекомендовался polling как проще; пользователь выбрал реал-тайм).
- **Транспорт:** браузер → axum WebSocket (auth по session-cookie, как `/api/*`); десктоп Tauri → нативные Tauri-события от бэкенда (не требует включённого сервера). Пушим: новая заявка + смена статуса; принтер-алерты — по тому же каналу.
- Планировщику/researcher: заложить reconnect/состояние WS, аутентификацию WS по сессии, и эквивалентный путь Tauri-событий.

#### D-Req-Lifecycle-01: переходы на стороне специалиста
- Создана → Принять в работу / Отклонить → Выполнить. Статусы уже в `requests` CHECK (open/in_progress/completed/rejected). Enforcement переходов — в сервис-слое (паттерн Phase 5 authorize/service). История заявок и статусов (REQ-07) — из audit_log (паттерн Phase 3/4).

#### D-Req-CART07-01: кнопка → pre-filled CART-07
- На заявке «Замена картриджа» при выполнении кнопка «Установить картридж» открывает существующую `OperationModal` (CART-07), pre-filled принтером (и моделью если задана). Успешная установка переводит заявку в «Выполнена». Реализует REQ-05 «сразу из контекста заявки».

#### D-PRN07-01: связь установки картриджа с принтером (FK)
- В Phase 6 установка картриджа (CART-07) связывается с конкретным устройством-принтером (FK) — отложенный из Phase 4 пункт. Позволяет отображать «какой картридж сейчас стоит» на карточке принтера (PRN-07).
- Также: автокомплит совместимых принтеров в моделях картриджей (Phase 4 D-Model-Compat-01) дополняется реальными принтерами из БД — сигнатура не меняется, расширяется источник.

### Claude's Discretion
- Concurrency/timeout discovery-скана; точный интервал опроса по умолчанию; формат хранения toner_levels (JSON-форма).
- Точная стратегия retention/downsample и числа.
- Форма USB-учёта (флаг + FK на host-device vs отдельная связь).
- Хранение категорий свободной формы (lookup vs CHECK) и набора алертов (таблица alerts vs статус-поле на printers).
- Точная сигнатура WS-протокола/событий и формат payload.
- Структура hexagonal-слоёв и feature-папок — паттерн как devices/cartridges/acts.
- Состав миграций (V020+): printers, printer_readings, oid_profiles (+seed), request_categories/alerts по необходимости.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Roadmap & requirements
- `.planning/ROADMAP.md` §«Phase 6: Принтеры (SNMP-мониторинг) и Заявки» — goal + 5 success criteria. ⚠️ Criterion #3 (Pantum-детекция) частично deferred — см. D-Pantum-01.
- `.planning/REQUIREMENTS.md` — PRN-01..08 (стр. 83–90), REQ-01..05, REQ-07 (стр. 94–100). Coverage table стр. 295–307, 372. ⚠️ PRN-04 механизм опроса → Phase 8 spike (стр. 86); PRN-06 hang-детекция частично → PNT v2 (стр. 88, 196). DASH-05 (стр. 134) питается алертами Phase 6.
- `CLAUDE.md` — стек (snmp2 0.4 `crypto-rust`, single-writer, axum WebSocket via `axum::extract::ws`, rustls, Secret<T>, portable-дисциплина); «What NOT to Use».
- `.planning/PROJECT.md` — Core Value «одной кнопкой»; учёт принтеров/картриджей как болевые точки.

### Существующая схема (уже от Phase 1, не пере-создавать)
- `migrations/V006__requests.sql` — таблица `requests` (request_type CHECK cartridge_replace/free_form/ad_register, status CHECK open/in_progress/completed/rejected, requested_by_user_id, assigned_to_user_id, printer_device_id FK, cartridge_model_id FK, description, resolution_notes, standard4). **Заполняется в Phase 6** (ad_register — Phase 8).
- `migrations/V001__init_pragmas_and_lookups.sql` — `device_types` (1=Устройство, 2=Принтер); `cartridge_statuses`/`cartridge_states`.
- `migrations/V003__devices.sql` — `devices` (type_id FK; БЕЗ IP/SNMP-колонок → новая таблица `printers`).
- `migrations/V005__cartridges.sql` — `cartridge_models`, `cartridge_model_compatibility(printer_brand, printer_model)` (источник автокомплита совместимости, дополняется реальными принтерами), `cartridges`.
- `migrations/V008__audit_log.sql` — shape audit_log (история заявок REQ-07, история операций).
- `migrations/V016__cartridges_kind_color_settings.sql` — таблица `app_settings(key, value)` (настройки discovery/опроса/retention).
- Последняя миграция — **V019** (`V019__users_is_active.sql`); новые миграции Phase 6 начинаются с **V020**.

### Phase 5 carry-forward (auth/RBAC/server/WS-основа)
- `.planning/phases/05-auth-server-mode/05-CONTEXT.md` — **D-RBAC-01** (единый `authorize(ctx, action)` на оба транспорта — заявки/принтеры проверяют роль здесь), **D-RBAC-02** (employee видит портал «Заявки» — Phase 6 наполняет placeholder), **D-Session-01** (tower-sessions rusqlite — auth для WS по cookie), **D-Server-01** (axum bind'ится, hot start/stop), **Secret<T>** для community.
- `ui/src/features/layout/sidebar-config.ts` — `/printers` (phase 6) и `/requests` (phase 6) — заменить placeholder-страницы.
- `ui/src/pages/PrintersPage.svelte`, `ui/src/pages/RequestsPage.svelte` — существующие placeholder-страницы.

### Phase 4 carry-forward (CART-07, OperationModal, compat)
- `.planning/phases/04-cartridges/04-CONTEXT.md` — **D-Op-Modal-01** (`OperationModal` + `cartridges_transition`) — REQ-05 переиспользует; **D-Op-Fields-01** установка CART-07; **D-Model-Compat-01** (Phase 6 дополняет источник автокомплита реальными принтерами); отложенные в Phase 6 пункты: связь установки с FK-принтером (PRN-07), REQ-05.
- `ui/src/features/cartridges/OperationModal.svelte` (и `cartridges/` feature-папка) — pre-filled запуск из контекста заявки.

### Существующий код (точки интеграции)
- `crates/trackly-app/src/context.rs` — `AppCtx` (расширяется `printers`/`requests`/`snmp` сервисами; `shutdown: CancellationToken` для фонового опросного task).
- `crates/trackly-infra/src/db/writer_worker.rs` (single-writer) + `pools.rs` (reader pool) — snapshots/мутации через writer, чтения/списки через readers.
- `crates/trackly-app/src/http/` — per-resource axum роутеры (bind'ятся с Phase 5) + добавить `/api/v1/printers`, `/api/v1/requests`, WebSocket-эндпоинт.
- `crates/trackly-core/src/primitives/secret.rs` — `Secret<T>` для community.
- `crates/trackly-infra/src/config.rs` — конфиг (snmp2 ещё НЕ в зависимостях — добавить).
- `ui/src/lib/api/client.ts` — dual-transport `apiCall()`; добавить WS-клиент (браузер) + Tauri-события (десктоп).

### External (researcher fodder)
- snmp2 0.4 (`crypto-rust`): https://docs.rs/crate/snmp2/latest — get/getnext/getbulk/walk, v2c, async sessions.
- RFC 3805 Printer-MIB (prtMarkerSuppliesLevel, prtMarkerLifeCount, hrPrinterStatus, prtJobEntry) — стандартные OID + fallback.
- axum WebSocket: `axum::extract::ws` (CLAUDE.md рекомендованный путь для LAN realtime).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `WriterHandle::execute` (single-writer) — snapshots `printer_readings`, мутации `printers`/`requests`, prune retention.
- `ReaderPool::acquire()` — списки принтеров/заявок, история, текущие уровни.
- `Secret<T>` — community string (zeroize-on-drop, `***` Debug).
- `OperationModal` + `cartridges_transition` (Phase 4) — REQ-05 pre-filled установка CART-07.
- `AppCtx` + `shutdown` CancellationToken — фоновый опросный task под под-токеном (паттерн Phase 5 D-Server-01).
- axum роутеры + tower-sessions middleware (Phase 5) — auth для `/api/v1/printers|requests` и WS по session-cookie.
- master-detail + switch-bar/фильтры паттерны (`ActsMasterDetail`, `DeviceFilters`, `DeviceContextMenu`) — для списков принтеров/заявок.
- `app_settings(key, value)` — настройки discovery/опроса/retention (как low_stock_threshold).

### Established Patterns
- **Hexagonal:** core/ports + infra/repos + app/services + app/tauri_cmds + app/http. `SnmpClient` — порт в core, real/mock — в infra/тестах.
- **Single-writer** для всех записей; SNMP-I/O — вне БД, результаты пишутся через writer.
- **«Один DTO, два транспорта»** + specta export `collect_commands!`.
- **UTC unix seconds; soft-delete (standard4); audit_log на mutations** (история заявок/операций).
- **lookup-таблицы** для enum-наборов (device_types, *_statuses) — образец для oid_profiles / request_categories.
- **Counter table** (если заявкам нужен человекочитаемый номер — планировщику решить; в ROADMAP не требуется).

### Integration Points
- `AppCtx::build` — добавить `PrinterService`, `RequestService`, `SnmpClient` (real/mock по config), запустить опросный task.
- `specta_export collect_commands![..., printers_*, requests_*]`.
- Sidebar `/printers`, `/requests` (phase 6) — реальные страницы вместо placeholder.
- Миграции **V020+**: `printers`, `printer_readings`, `oid_profiles` (+seed 4 vendor + RFC3805), при необходимости `request_categories`, `printer_alerts`; добавить USB-учёт в printers; FK установки картриджа на принтер (PRN-07).
- Cargo: добавить `snmp2 0.4` (feature `crypto-rust`) в trackly-infra.

### Not-yet-existing (создаём в Phase 6)
- `crates/trackly-core/src/ports/{printers,requests,snmp}.rs`, `domain/{printers,requests}.rs`
- `crates/trackly-infra/src/repos/{printers_sqlite,requests_sqlite}.rs`, `snmp/{real,mock}.rs`
- `crates/trackly-app/src/services/{printer_service,request_service}.rs`
- `crates/trackly-app/src/dto/{printer,request}.rs`
- `crates/trackly-app/src/tauri_cmds/{printers,requests}.rs`, `http/{printers,requests,ws}.rs`
- `ui/src/features/{printers,requests}/` (страницы, master-detail, формы заявок, discovery-модал, alert-индикаторы, WS-клиент)
- `migrations/V020+__*.sql`

</code_context>

<specifics>
## Specific Ideas

- **Целевые модели (OID-профили seed):** Pantum BM5100ADN (приоритет), Kyocera ECOSYS, HP LaserJet, Canon iR + RFC3805 fallback.
- **Категории свободной формы:** «Ремонт техники» / «Расходные материалы» / «Программное обеспечение» / «Прочее».
- **Discovery дефолты:** SNMP v2c, community 'public' (настраивается).
- **Замена картриджа — UX:** сотрудник выбирает только принтер; модель — забота специалиста при выполнении (low-friction для нетех-сотрудника).
- **WebSocket:** реал-тайм уведомления (новая заявка / смена статуса / принтер-алерт); браузер — WS к axum, десктоп — Tauri-события.
- **«Одной кнопкой» (core value):** discovery + завести принтеры + сразу видеть уровни тонера без ручного ввода OID.

</specifics>

<deferred>
## Deferred Ideas

- **Pantum-специфичная hang-эвристика** (prtMarkerLifeCount + SNMP job-table очередь) + авто-restart → **v2 (PNT)**. ⚠️ Частичный перенос ROADMAP #3 — см. D-Pantum-01.
- **Host-side Windows print spooler** как источник сигнала очереди → не v1 (выбран SNMP job-table на будущее).
- **USB-механизм опроса** (агент/WMI/RPC) → **Phase 8 spike** (PRN-04 здесь — только учёт связи).
- **Email/Telegram/Webhook уведомления** (NTF-02..05) → финальная фаза v2.
- **Заявка на регистрацию AD** (REQ-06, подтип `ad_register`) → **Phase 8**.
- **Виджет «Принтеры» (DASH-05)** и отчёты по принтерам → **Phase 7** (Phase 6 даёт данные/команды).
- **UI-редактор OID-профилей и настроек discovery** → Phase 7 Settings при необходимости (Phase 6 — seed + строки app_settings).
- **Доп. вендоры принтеров сверх 4 целевых** → ADV (v2). Data-driven профили облегчают добавление.

None из обсуждения не выпало за рамки фазы без учёта.

</deferred>

---

*Phase: 06-snmp*
*Context gathered: 2026-06-14 via /gsd-discuss-phase 6*
</content>
</invoke>
