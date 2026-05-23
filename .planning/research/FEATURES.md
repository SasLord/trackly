# Feature Research

**Domain:** IT asset & consumable (cartridge/printer) tracking — self-hosted, single-org, Russian-context
**Researched:** 2026-05-24
**Confidence:** HIGH (cross-referenced Snipe-IT, GLPI, ManageEngine ServiceDesk Plus/AssetExplorer, Lansweeper, PaperCut; plus Russian-org conventions for акты приёма-передачи)

## Executive Synthesis

PROJECT.md describes a product that sits at the intersection of three mature category leaders:

- **Snipe-IT** — clean asset tracking + checkout/checkin with signatures (closest in spirit). No network discovery, no printer SNMP, no Russian-style numbered acts.
- **GLPI** — heavyweight ITSM with first-class printer/cartridge modules (CartridgeItem + Cartridge, SNMP toner levels, page counters). Plugin-heavy, complex.
- **ManageEngine ServiceDesk Plus / AssetExplorer** — gold standard for lifecycle states (In Use / In Store / In Repair / Expired / Disposed) and asset loans. Commercial.

The user's spec is **strongly aligned with table stakes** in all three. Notable strengths in the spec: contextual autocomplete, partial-return semantics with sub-numbering ("N в1", "N в2"), per-printer SNMP focus on real problem printers (Pantum), Russian-first templates. Notable **gaps** flagged below: no categories/types beyond hard-coded enums, no warranty/purchase fields, no audit log section explicitly named, no asset-to-cartridge installation history, no soft-delete recovery.

The single biggest risk in the spec is **conflating "Устройство" and "Расходник" types under the same CRUD** — Snipe-IT and GLPI both learned to separate them (Assets vs Consumables vs Components) because lifecycles diverge sharply. The user has hinted at this by having a separate "Картриджи" section, but explicit "Расходник" inside Devices may confuse the model.

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist in any IT asset / inventory tool. Missing these = product feels incomplete or amateurish.

| Feature | Why Expected | Complexity | In PROJECT.md? | Notes |
|---------|--------------|------------|----------------|-------|
| CRUD устройств с обязательными/опциональными полями | Базовая функция любой системы учёта | LOW | YES | Spec is solid; matches Snipe-IT/GLPI core model |
| Серийный + Инвентарный номера | Стандарт для основных средств в РФ; в Snipe-IT — asset_tag + serial | LOW | YES | Good — inventory_no is uniquely Russian/CIS convention |
| Жизненные статусы (In Use / In Store / In Repair / Disposed) | Lifecycle states are universal — ServiceDesk Plus defaults: In Use, In Store, In Repair, Expired, Disposed | LOW | YES (На складе / В работе / На ремонте / Списано) | Matches ServiceDesk Plus 1:1; consider adding "Утеряно" (Lost) — common state |
| Полнотекстовый поиск | Без него таблица в 1000+ устройств непригодна | MEDIUM | YES | SQLite FTS5 — built-in, fast |
| CSV import/export | Bulk-загрузка из Excel — главный путь миграции с ручного учёта | MEDIUM | YES | Critical for adoption — first-time onboarding load |
| Группировка одинаковых позиций | Когда 30 одинаковых мышек — список не должен быть из 30 строк | MEDIUM | YES | Snipe-IT не имеет — это differentiator, см. ниже |
| Check-out / Check-in (Акт приёма-передачи + Возврат) | Core workflow — "выдал-вернул" | MEDIUM | YES | Spec's partial-return numbering ("N в1", "N в2") is more sophisticated than Snipe-IT |
| Acceptance signature / подпись принимающего | Snipe-IT supports on-screen signature (v3.6+) | MEDIUM | NO (НЕТ в spec) | **GAP — see Recommendations.** ФИО + дата есть, но физическая подпись отсутствует |
| Printable PDF документов (Акт, Документ приёма) | Юридическая обязательность в РФ — акт должен быть подписан и подшит | MEDIUM | YES | Spec has editable templates — strong |
| Sequential document numbering | Любая бух/учётная система — № акта обязателен, монотонно растущий | LOW | YES | Spec: auto-suggest next + override capability — good |
| История перемещений устройства (transfer history) | "Где и у кого было это устройство?" — core ITAM question | MEDIUM | PARTIAL | Spec упоминает "восстановление устройства на склад в исходное расположение", но **явного отдельного "Audit Log" по устройству нет.** В Snipe-IT это первое, что смотрят |
| Кому сейчас выдано (assigned to) | Прямой выход из текущего активного акта | LOW | IMPLICIT | Должно быть видно прямо на карточке устройства, не через клик в Акты |
| Категории / типы устройств | В Snipe-IT — Categories with per-category EULA, custom fields; в GLPI — Computer/Monitor/Printer/Phone/Peripheral types | LOW | PARTIAL | Spec фиксирует 3 типа (Устройство/Принтер/Расходник) — too rigid? See pitfalls |
| Custom fields | Snipe-IT и GLPI оба учли — без них пользователи упираются | MEDIUM | NO | **GAP** — типовая жалоба на ригидные системы. Хотя бы 3–5 freeform полей spec уже даёт (Техн. характеристики, Комплектация, Состояние) |
| Низкий остаток / low-stock alerts для картриджей | GLPI: CartridgeItem с настраиваемым порогом alert; универсально | LOW | YES | Spec: "Баннер на дашборде при низком остатке" + порог в настройках — отлично |
| Cartridge → Printer compatibility matrix | GLPI: PrinterModel × CartridgeItem связи; обязательно для realistic учёта | LOW | YES | Spec: "Совместимые принтеры — массив пар Бренд+Модель" — корректно |
| Cartridge instance lifecycle (полный/частичный/пустой; на заправке) | GLPI tracks date_use, date_out, pages_printed; ваш spec богаче в части "На заправке" | MEDIUM | YES | Контекстные действия по статусу — отличное UX-решение |
| Cartridge installation history (когда поставили / в какой принтер / сколько отработал) | GLPI: page counter с момента установки картриджа | MEDIUM | PARTIAL | Spec логирует передачи (Дата/Кто выдал/Кому), но **связь "в этот принтер установлен" неявна.** См. рекомендации |
| SNMP printer status (toner, page count, error state) | PaperCut, GLPI, Lansweeper — все берут данные из Printer-MIB (1.3.6.1.2.1.43) + Host-Resources MIB | MEDIUM | YES | Industry-standard OID set — `prtMarkerColorantValue`, `prtAlert`, `hrPrinterDetectedErrorState` |
| Subnet discovery / network scan | Lansweeper's killer feature; GLPI Inventory plugin; Snipe-IT does NOT have it | HIGH | YES | Spec включает — это сильный differentiator vs Snipe-IT |
| End-user self-service portal | GLPI Self-Service profile — наиболее урезанный UI для заявок; универсально | MEDIUM | YES | Spec корректно ограничивает Сотрудника заявками |
| Заявки / requests lifecycle (создана → принять → выполнить / отклонить) | Базовый ticket workflow без излишеств | LOW | YES | Минимальный пайплайн — правильно, без enterprise-overkill |
| User roles (admin / technician / self-service) | GLPI: Admin, Supervisor, Technician, Hotliner, Observer, Self-Service, Read-only — overkill; Snipe-IT proper RBAC | LOW | YES (3 роли) | 3 роли — достаточно для целевого масштаба; не клонировать GLPI |
| LDAP/AD authentication | Snipe-IT: bind + sync, read-only AD account; никогда не пишет в AD | MEDIUM | YES (поздняя фаза) | Стандарт — bind для проверки пароля + pull атрибутов (ФИО) |
| Email/SMTP notifications | Любая система имеет — низкий остаток, новая заявка | LOW | YES (финальная фаза) | OK |
| Дашборд с виджетами | Все ITAM-системы — landing page с состоянием | LOW | YES | Хорошо подобраны виджеты |
| Reports: по периоду + экспорт | "Что в работе / На складе / Акты за месяц" — стандартный набор | MEDIUM | YES | Хорошее разбиение по месяцу/году/диапазону |
| Backup (manual + scheduled) | Self-hosted — критично; пользователи теряют данные без него | MEDIUM | YES | SQLite — backup через VACUUM INTO или копия с WAL-checkpoint |
| Organization branding (logo, реквизиты) на печатных формах | Документ — это лицо компании; без логотипа выглядит непрофессионально | LOW | YES | Хорошо |
| Editable document templates | GLPI имеет templates; Snipe-IT — слабее; пользователю критично для РФ-форм | HIGH | YES | Шаблоны Акта приёма-передачи и Документа приёма в БД — хорошо, переносится с portable |
| Audit log / changelog по записи | Snipe-IT shows full asset history tab; GLPI — historical tab; **must have** | MEDIUM | NO (не явно) | **GAP** — без этого "кто изменил статус" не отследить |
| Soft delete + восстановление | Снайп-IT trash bin; защита от случайного удаления | LOW | NO (не явно) | **GAP** — спец удалит акт, потеряет историю |

### Differentiators (Competitive Advantage)

Features that set Trackly apart from Snipe-IT/GLPI/Lansweeper for the **small Russian organisation** use case.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Portable single-executable + LAN-server toggle | Snipe-IT, GLPI — обе требуют LAMP-стек, MySQL, обслуживания; для 3–10 локаций это overhead. Tauri-портабл + встроенный axum = **запустил-и-работает** | HIGH | Этот один аргумент продаёт продукт целевой аудитории |
| Контекстный автокомплит ("при Наименовании=X — подсказки только из строк с этим Наименованием") | Сильно ускоряет ввод, снижает разнобой в данных; ни Snipe-IT, ни GLPI этого не делают | MEDIUM | Тонкая, но мощная UX-фича |
| Русская печатная форма Акта приёма-передачи как first-class citizen | Snipe-IT/GLPI требуют допиливания шаблонов под РФ-формат; Trackly — родная | MEDIUM | "Один клик — печать Акта" — то, что просит спецификация |
| Партиционные возвраты с подактами ("N в1", "N в2") | Реальный сценарий: выдали 3 ноутбука актом №42, вернули 2 актом «42 в1», ещё 1 актом «42 в2». Snipe-IT просто пере-checkin'ит — без юр. бумаги. GLPI — slow + plugin-зависимо | MEDIUM | Сильная differentiation |
| Группировка одинаковых non-unique позиций | "30 одинаковых мышек" — одна строка, разворачивается; Snipe-IT — 30 строк | MEDIUM | UX-killer для складов с расходниками |
| Pantum hung-spooler detection + (позже) auto-restart | Pantum BM5100ADN — известная проблема в AD-сетях; ни одна универсальная система не лечит конкретно её. **Это и есть ваш Core Value** | HIGH | После validation — реальная боль решаемая «одной кнопкой» |
| Cartridge "На заправке" статус | Большинство систем (Snipe-IT, GLPI) тонер либо есть, либо нет; "ушёл на заправку и вернётся" — российская специфика (refilled cartridges культурно нормальны) | LOW | Хорошо, что есть в spec |
| Заявки на регистрацию пользователя AD + автоприём | Сглаживает onboarding: новый сотрудник из AD сам создаёт себе учётку через web; админ просто approve. Snipe-IT/GLPI — manual create | MEDIUM | Хороший компромисс между full SSO и manual user mgmt |
| Авто-генерация номеров (C-000001, акт №N) с возможностью переопределить | Юр. требование РФ — последовательная нумерация; spec реализует корректно | LOW | Snipe-IT тоже умеет, но менее гибко |
| Темы (Тёмная/Светлая/Системная) в layout, а не глубоко в настройках | DX-полировка — пользователи это замечают и любят | LOW | Маленькое, но приятно |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem attractive but you should **explicitly NOT** build in v1. PROJECT.md already correctly excludes most of these — listed here to keep them out.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Multi-tenant / multi-organization | "Что если поставим клиенту-MSP?" | Меняет всю модель данных (entity scoping в каждой query), усложняет миграции, нагружает UI выбором tenant. GLPI делает это и страдает от сложности конфигурации | Single-org. Если нужно — отдельный экземпляр |
| Полный workflow-engine для заявок (правила, эскалация, SLA, очереди) | "Хотим как Jira / ServiceDesk" | 90% мощи никогда не используется в org на 10 локаций; spec'у с двумя типами заявок (картридж/свободная) этого с головой | Минимальный 3-state lifecycle (создана → в работе → выполнена/отклонена). Spec корректен |
| Глубокая интеграция с ticketing (Jira / Zendesk / GLPI / Redmine) | "Хочется, чтобы заявка автоматически попадала в основную тикет-систему" | Open API + webhook — достаточно; жёсткая интеграция = vendor lock | Webhook outbound (есть в spec) + REST API |
| Cloud sync / SaaS режим | "А если несколько офисов?" | Уничтожает portable-модель, требует server-side, multi-tenancy, ssl, identity, billing | Каждая локация — отдельный экземпляр, либо v2 централизованный режим |
| Mobile app (iOS / Android) | "Хочется со смартфона" | Tauri-приложение на десктопе + web-режим в LAN покрывают; mobile = удвоение поверхности кода | Responsive web UI в LAN режиме |
| Barcode/QR scanning через камеру | "Сканер штрих-кодов как в Snipe-IT" | Привлекательно, но требует доп. кода и тестирования на мобильных; в org на 1000 устройств админ всё равно использует ручной сканер | Поддержка ручного сканера (USB HID — он шлёт текст в активное поле) — бесплатно |
| Покупки / счета / поставщики (procurement) | "А давайте ещё счета будем учитывать" | Это уже бухучёт. 1С это делает лучше | Поле "Дата покупки / Поставщик" в custom fields, не отдельный модуль |
| Лицензии на ПО (Software licenses) | Snipe-IT и GLPI имеют — выглядит как «полнота» | Лицензии — отдельная модель данных (seats, expiry, attached to user vs device), отдельная сложность; не цель Trackly (это «техника + картриджи») | Если пользователь спросит — отдельный milestone, не v1 |
| Multi-language UI (i18n) с самого начала | "А вдруг будет англоязычный клиент?" | Удваивает QA, вытяжка строк сейчас усложняет; spec корректно откладывает | Только русский в v1, строки локализуемы там, где естественно |
| Карта помещений с расположением устройств (floor plan) | Визуально круто, демо-друг | UI/UX-monster, требует canvas, drag-drop, persistence координат; в spec корректно отложено в v2 | Поле "Расположение" (текст с автокомплитом) — 80% пользы за 5% сложности |
| Полное SSO Kerberos/NTLM на старте | "Чтобы не вводить пароль" | Kerberos через axum/tauri — нетривиально на Windows, особенно portable; spec корректно откладывает | Локальные учётки v1 → AD bind v2 → Kerberos если попросят |
| Версионирование документов / совместное редактирование шаблонов | "А вдруг два админа правят шаблон" | YAGNI на масштаб spec (1 админ, 1–2 спеца) | Простой last-write-wins, шаблон в БД |
| Real-time WebSocket-обновления для всех таблиц | "Чтобы у всех всё мгновенно" | SQLite + 20 одновременных пользователей — polling каждые 10–30 сек хватит с запасом | Polling/refresh, либо SSE для конкретных событий (новая заявка) |
| Plugin / расширения architecture | "Чтобы можно было дополнять" | GLPI-стиль: становится maintenance hell, каждый плагин ломает при обновлении | Стабильный schema + REST API + webhooks — пусть интеграции живут снаружи |

## Cross-Reference: PROJECT.md vs Industry Standard

### Things in PROJECT.md that look non-standard or risky

| Spec item | Concern | Recommendation |
|-----------|---------|----------------|
| `Тип устройства: Устройство / Принтер / **Расходник**` | "Расходник" внутри устройств конфликтует с отдельным разделом "Картриджи". Snipe-IT и GLPI чётко разделяют Assets / Consumables — потому что lifecycle разный (расходник списывается, не возвращается) | Либо убрать "Расходник" из типов устройств (пусть всё non-cartridge consumable идёт в отдельный раздел или как non-unique device with count), либо чётко определить, чем Расходник отличается от Картриджа. Иначе пользователь не поймёт, куда заводить, например, пачку бумаги |
| Жёстко 3 типа (Устройство/Принтер/Расходник), жёстко 4 статуса | ServiceDesk Plus, Snipe-IT и GLPI дают **настраиваемые** типы и статусы. На масштабе spec — может оказаться достаточно, но через год эксплуатации появятся "Монитор", "Сетевое оборудование", "Утеряно" | Запланировать в схеме БД таблицу `device_types` и `device_statuses` с seed-values, а не enum в коде. Дешёвая инвестиция на будущее |
| Возврат с под-нумерацией "N в1", "N в2" | Не стандарт ни в одной из исследованных систем — это **ваш собственный invariant** | Сильная фича, но нужно явно описать в схеме БД: parent_act_id + sub_number. Будет нетривиальный edge case при удалении промежуточного возврата |
| Удаление акта возврата с восстановлением предыдущих Состояния/Расположения | Snipe-IT, GLPI — soft-delete + log; явный rollback редко делают | Хорошая идея, но требует хранения "before" state в audit log; иначе откатывать неоткуда. Это **косвенно требует audit log**, которого в spec нет явно |
| Авто-приём заявок на регистрацию пользователя — настройка | Большинство систем этого не дают (security concern: автоматическая регистрация = surface для атаки) | OK для LAN-only, но дефолт = OFF. Сделайте явный чекбокс с предупреждением. В web-режиме это особенно важно |
| Cartridge статусы: Полный / Частичный / Пустой + На складе / В работе / На заправке / Списано | Два ортогональных измерения (заряд + локация-статус) — это правильно, но в spec они смешаны | Прямо разделите в модели: `charge_state` (full/partial/empty) и `lifecycle_state` (in_stock/in_use/refilling/disposed). UI может показывать единую строку, БД — две колонки |
| Печать заявки | Никто из исследованных систем не печатает заявки на бумаге — они digital | Если это требование пользователя — оставить, но не приоритет. Pdf-export достаточно |
| In-app + Email + Telegram + Webhook — **все** в финальной фазе | Это много для одной фазы | Email — критично (стандарт); Telegram — nice; Webhook — power-user; In-app — лёгкое. Расположите по приоритету. Делать все одновременно — рискованно |

### Things missing from PROJECT.md that you'd expect (recommendations)

| Missing feature | Why it matters | Priority |
|----------------|----------------|----------|
| **Явный Audit Log / История изменений по записи** | "Кто изменил статус с 'В работе' на 'Списано'?" — без этого спор между админом и спецом не разрешить. Snipe-IT и GLPI оба делают это first-class | **P1** — закладывать в схему сразу (events / activity_log table); UI можно подтянуть позже |
| **Soft delete + корзина / восстановление** | Спец случайно удалил акт — данных уже нет. Snipe-IT имеет trash bin | **P1** — `deleted_at` column + UI "Корзина" в разделах |
| **Кому сейчас выдано** (assigned_to) — поле прямо на карточке устройства | Сейчас выяснить, у кого устройство — нужно идти в Акты и искать активный незавершённый | **P1** — это denormalized field, обновляется при создании/возврате акта |
| **Связь Картридж → Принтер при установке (current_printer_id)** | Spec логирует "Кому выдал", но не моделирует "сейчас стоит в принтере X". GLPI это делает — позволяет посчитать износ | **P2** — текущий принтер для cartridge instance |
| **Подпись принимающего (signature pad)** | Snipe-IT v3.6+ — on-screen подпись мышью/пальцем. ФИО + подпись в Акте = юр. вес выше | **P2** — нетривиально в Tauri (canvas + сохранение PNG/SVG) |
| **Поле "Утеряно" в статусах устройств** | Реальный кейс — устройство не списано, а потеряно. ServiceDesk Plus имеет | **P2** — низкая стоимость, высокая полезность |
| **Warranty / срок гарантии + дата покупки** | Стандартный custom field, но нужен очень часто (когда покупали, когда кончится гарантия) | **P2** — пара дат на устройстве |
| **REST API (минимальный — read для интеграций)** | "Как выгрузить данные в наш Power BI / 1С?" — типовой вопрос. CSV-export это лишь частично решает | **P3** — но если webhook outbound уже есть, REST GET-endpoint близко |
| **Печать инвентарных этикеток (со штрих-кодом)** | Стандартная задача — этикетки на принтере Brother/Zebra с инвентарным номером | **P3** — простой PDF-шаблон + QR/Code128 |
| **Bulk operations в UI** (массовая смена статуса, массовый возврат) | Spec упоминает "bulk-применение для нескольких устройств в одном акте" — хорошо; но bulk-edit в списке устройств не назван | **P2** — выбрать checkbox-ом строки и сменить статус группе |
| **Поле "Подразделение / Отдел" (department)** для пользователей и устройств | Российская специфика — "выдано в IT-отдел", "выдано бухгалтерии" | **P2** — справочник Подразделений + связь |
| **Защита от concurrent edit конфликтов в LAN-режиме** | До 20 одновременных пользователей, SQLite WAL — возможны race-conditions при редактировании одной и той же записи | **P1** — оптимистический lock (version-колонка) на сущностях |

## Feature Dependencies

```
Foundation (Phase 1: схема БД)
    ├── Devices CRUD
    │     ├── requires ──> Locations справочник
    │     ├── requires ──> Device types & statuses (table, не enum)
    │     ├── enhances ──> Audit log (для истории изменений)
    │     └── enables  ──> Bulk operations
    │
    ├── Acts (Акты)
    │     ├── requires ──> Devices CRUD
    │     ├── requires ──> Sequential numbering (counters)
    │     ├── requires ──> Users (для ФИО Сдал/Принял)
    │     └── enables  ──> Returns (Возвраты)
    │                       └── requires ──> Parent act + sub-numbering
    │
    ├── Cartridges
    │     ├── requires ──> CartridgeModels справочник
    │     ├── requires ──> Locations
    │     ├── requires ──> Printers (для compatibility + installation tracking)
    │     └── enables  ──> Low-stock alerts (требует Notifications)
    │
    ├── Printers
    │     ├── requires ──> SNMP scanner (subnet discovery)
    │     ├── requires ──> Cartridges (для compatibility)
    │     └── enables  ──> Pantum hung-spooler detection
    │                       └── requires ──> History of statuses
    │
    ├── Requests (Заявки)
    │     ├── requires ──> Users (создаёт сотрудник)
    │     ├── enhances ──> Cartridges (link cartridge-replacement request)
    │     └── enables  ──> AD registration requests
    │                       └── requires ──> AD integration (later phase)
    │
    ├── Reports
    │     ├── requires ──> Acts, Devices, Cartridges (все core entities)
    │     └── requires ──> Audit log (для отчётов за период)
    │
    ├── Document Templates
    │     ├── requires ──> Organization settings (logo, реквизиты)
    │     ├── used by  ──> Acts (печать Акта)
    │     ├── used by  ──> Devices (печать Документа приёма)
    │     └── used by  ──> Requests (печать заявки)
    │
    └── Notifications (final phase)
          ├── requires ──> Low-stock thresholds (settings)
          ├── requires ──> Email SMTP settings
          ├── enhances ──> Requests (notify on new request)
          ├── enhances ──> Cartridges (low-stock banner — может уйти и in-app only)
          └── enhances ──> Pantum-detection (alert on hung spooler)

Cross-cutting:
  Audit Log ──underlies──> ALL writable entities (Devices, Acts, Cartridges, ...)
  Soft Delete ──applies-to──> Acts, Devices, Cartridges (recoverable)
  Optimistic Lock ──guards──> Concurrent edits in LAN-server mode
```

### Critical Dependency Notes

- **Acts требуют Users с ФИО** — даже если AD не интегрирован, ФИО должны быть полями. В spec это так.
- **Cartridge installation tracking** требует `printers` таблицы — поэтому Printers и Cartridges должны быть в одной фазе или Printers раньше.
- **Returns с под-нумерацией "N в1"** — требуют parent_act_id в схеме. **Не вырезайте это решение позже — это всё переписывать**.
- **Audit log должен быть в Phase 1 (схема БД)** — добавлять его потом = переписывать write-paths.
- **Soft delete должен быть в Phase 1** — `deleted_at` поле во всех recoverable entities. Дешёво заложить сразу.
- **Шаблоны документов в БД** — корректно: portable-сборка переносит и шаблоны. Но **картинка логотипа** — где? В БД как BLOB или рядом? Решить рано.
- **Web-режим (axum) и AD-интеграция** разнесены по фазам, но **AD bind требует HTTPS**, иначе пароли пользователей идут по сети в открытом виде. HTTPS должен прийти не позже AD-фазы.

## MVP Definition

### Launch With (v1.0 — критический минимум)

These are non-negotiable to call it "Trackly v1".

- [ ] Devices CRUD + статусы + локация + (инв№/серийный)
- [ ] Локации — справочник (даже простой, с autocomplete)
- [ ] Acts CRUD (создание / редактирование / удаление с восстановлением)
- [ ] Returns (полный + частичный с под-нумерацией)
- [ ] Sequential numbering (auto + override)
- [ ] Печать Акта в PDF с editable template
- [ ] Печать Документа приёма
- [ ] Полнотекстовый поиск по устройствам + актам
- [ ] CSV import/export (хотя бы для устройств)
- [ ] Пользователи + 3 роли + локальная аутентификация
- [ ] Backup (manual at minimum; scheduled опционально)
- [ ] Organization settings + logo
- [ ] Portable-режим + LAN-server toggle
- [ ] Базовый дашборд (виджеты по устройствам и заявкам)
- [ ] **Audit log** — даже если без UI, должна писаться история изменений
- [ ] **Soft delete** — `deleted_at` на Acts и Devices как минимум

### Add Soon After (v1.1–v1.3 — последовательные пост-релизы)

- [ ] Картриджи (модели + экземпляры + lifecycle) — может быть в v1 если время позволит; иначе ранний follow-up
- [ ] Printers с SNMP-мониторингом (toner, status)
- [ ] Subnet discovery
- [ ] Заявки (CRUD + lifecycle)
- [ ] Browser-доступ для сотрудников (web-UI для заявок)
- [ ] Отчёты с фильтрами + экспорт + печать
- [ ] Email notifications (SMTP)
- [ ] Pantum hung-spooler detection (только алерт, без авто-restart)
- [ ] In-app уведомления / баннеры
- [ ] Корзина (восстановление soft-deleted)

### Future (v2+)

- [ ] AD-вход (bind + auto-fill ФИО + registration-requests)
- [ ] Pantum auto-restart spooler (после подтверждения гипотезы)
- [ ] Telegram-бот + Webhook
- [ ] Карта помещений (deferred per spec)
- [ ] Подпись принимающего (signature pad)
- [ ] REST API (read endpoints для интеграций)
- [ ] Печать этикеток с инвентарным номером и QR-кодом
- [ ] Warranty / dates tracking как custom fields
- [ ] Подразделения (department)
- [ ] Bulk-edit в списке устройств
- [ ] Соответствие RFC 1759 (стандартный Printer-MIB) для большего набора моделей принтеров

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Devices CRUD | HIGH | LOW | P1 |
| Acts + Returns | HIGH | MEDIUM | P1 |
| Sequential numbering + auto-suggest | HIGH | LOW | P1 |
| Печать Акта в PDF | HIGH | MEDIUM | P1 |
| Editable document templates | HIGH | MEDIUM | P1 |
| Контекстный автокомплит | HIGH | MEDIUM | P1 |
| Audit log (write-only сначала) | MEDIUM | LOW (если заложить рано) | P1 |
| Soft delete | MEDIUM | LOW | P1 |
| Optimistic locking | MEDIUM | LOW | P1 |
| Portable + LAN server toggle | HIGH | HIGH | P1 |
| 3 роли + локальная аутентификация | HIGH | LOW | P1 |
| Backup (manual) | HIGH | LOW | P1 |
| CSV import/export | HIGH | MEDIUM | P1 |
| Cartridges (модели + экземпляры) | HIGH | MEDIUM | P1/P2 |
| Low-stock alerts | MEDIUM | LOW | P2 |
| Subnet discovery | MEDIUM | MEDIUM | P2 |
| SNMP printer monitoring | HIGH | MEDIUM | P2 |
| Заявки CRUD + lifecycle | HIGH | MEDIUM | P2 |
| Browser-режим для сотрудников | HIGH | HIGH (HTTPS, auth, UI) | P2 |
| Отчёты | MEDIUM | MEDIUM | P2 |
| Email notifications | MEDIUM | MEDIUM | P2 |
| Pantum hung-spooler detection | HIGH | MEDIUM | P2 |
| Дашборд виджеты | MEDIUM | LOW | P2 |
| Темы (Тёмная/Светлая) | LOW | LOW | P2 |
| AD-вход (bind + sync ФИО) | HIGH | HIGH | P3 |
| Pantum auto-restart spooler | HIGH (если работает) | HIGH (с риском) | P3 |
| Telegram-бот | MEDIUM | LOW–MEDIUM | P3 |
| Webhook outbound | MEDIUM | LOW | P3 |
| Карта помещений | LOW–MEDIUM | HIGH | DEFERRED v2 |
| Подпись принимающего | MEDIUM | MEDIUM | DEFERRED v2 |
| REST API | LOW (для целевой аудитории) | MEDIUM | DEFERRED v2 |

## Competitor Feature Analysis

| Feature | Snipe-IT | GLPI | Lansweeper | ServiceDesk Plus | Trackly approach |
|---------|----------|------|------------|------------------|------------------|
| Asset CRUD | YES (Asset, Accessory, Component, Consumable) | YES (15+ asset types) | YES (auto from discovery) | YES (configurable) | YES — 3 типа в v1 (расширяемо) |
| Lifecycle states | Status Labels (configurable) | State (configurable) | Limited | In Use / In Store / In Repair / Expired / Disposed | 4 status + рекомендуем добавить "Утеряно" |
| Check-out/Check-in | YES + signature (v3.6+) | YES (via reservation) | NO (read-only inventory) | YES (Loans) | YES — с РФ-формой акта |
| Partial return | NO (just re-checkin) | LIMITED | N/A | LIMITED | **YES — "N в1", "N в2"** (differentiator) |
| Custom fields | YES | YES | LIMITED | YES | **MISSING in spec** (рекомендация) |
| Categories | YES (per-category EULA) | YES | YES | YES | PARTIAL — фиксированные типы |
| Audit log | YES (asset history tab) | YES (historical tab) | YES | YES | **MISSING in spec** (рекомендация) |
| Soft delete | YES (trash bin) | YES (trashbin module) | N/A | YES | **MISSING in spec** (рекомендация) |
| Bulk operations | YES | YES | YES | YES | PARTIAL (есть в acts, нет в device list) |
| CSV import/export | YES | YES | YES | YES | YES |
| Sequential numbering | LIMITED | LIMITED (plugin) | NO | YES | **YES — first-class** |
| Cartridge tracking | Via Consumables (generic) | YES (CartridgeItem + Cartridge) | NO | YES | **YES — first-class + статус "На заправке"** |
| Cartridge → printer compat | NO | YES (CartridgeItem.Printer) | NO | LIMITED | YES — array of Brand+Model pairs |
| Cartridge install history | NO | YES (with page counter) | NO | LIMITED | PARTIAL (нет current_printer link) |
| SNMP printer monitoring | NO | YES (via Inventory plugin) | YES (toner graph) | YES | YES — focus on Pantum/Kyocera/HP/Canon |
| Subnet discovery | NO | YES (Inventory plugin) | YES (core) | YES | YES |
| Hung spooler detection | NO | NO | NO | NO | **YES (planned) — unique differentiator** |
| End-user portal | LIMITED | YES (Self-Service profile) | NO | YES (full ticketing) | YES — minimal (заявки only) |
| Заявки workflow | NO (asset-only) | YES (full ITIL) | NO | YES (full ITIL) | MINIMAL (3 states — правильно) |
| LDAP / AD | YES (bind + sync) | YES | YES | YES | YES (планируется в поздней фазе) |
| RBAC | YES (admin/user/superuser + per-permission) | YES (7+ profiles) | YES | YES | YES — 3 роли (достаточно) |
| Document templates (РФ-стиль) | NO (need custom dev) | LIMITED (plugin needed) | NO | LIMITED | **YES — first-class** |
| Sequential акт numbering (РФ-стиль) | LIMITED | LIMITED | NO | YES | **YES — first-class** |
| Reports | LIMITED (basic) | YES (configurable) | YES (powerful) | YES (powerful) | YES (period-based, per spec) |
| Low-stock alerts | YES | YES | NO | YES | YES — баннер на дашборде |
| Notifications (email) | YES | YES | YES | YES | YES (планируется) |
| Telegram | NO | Via plugin | NO | NO | **YES (planned) — niche differentiator** |
| Backup (built-in) | NO (DIY MySQL dump) | NO (DIY MySQL dump) | NO | YES (commercial) | **YES — first-class portable backup** |
| Portable / single-binary | NO (LAMP stack) | NO (LAMP stack) | NO (Windows service) | NO (heavy install) | **YES — Tauri portable** |
| Multi-tenant | LIMITED | YES (Entities) | LIMITED | YES | NO (intentional) |
| Russian UI / РФ-формы | Via translation | Via translation | EN | EN | **YES native** |
| License cost | FREE (open-source) | FREE (open-source) | Commercial (free tier <100) | Commercial | FREE (presumably) |

## Russian-Org Specific Context

The spec correctly recognises Russian organisational conventions. For roadmap clarity, here's what's specifically Russian and why it matters:

| Russian convention | Why it matters | Spec status |
|--------------------|----------------|-------------|
| **Инвентарный номер** (inventory number) | Separate from serial; assigned by accounting; required for fixed assets (основные средства) per 1С-style учёт | YES — отдельное поле |
| **Акт приёма-передачи** as legal document | Signed paper artifact required to prove transfer. Без него — спор о пропаже устройства не выиграть | YES — печатается, шаблон редактируем |
| **Последовательная нумерация** актов | Бухгалтерская норма — пропуск номера = вопросы от ревизора. Override должен быть **редким исключением** | YES — auto-suggest + override |
| **ФИО (фамилия+имя+отчество)** в Сдал/Принял | "ФИО" в РФ — обычно фамилия + инициалы или полностью; not "first name + last name" | OK — текстовое поле |
| **МОЛ (материально ответственное лицо)** | Person accepting асset becomes МОЛ. В spec — это "Принял" в акте | IMPLICIT — стоит явно отметить в UI labels |
| **Заправка картриджа** (cartridge refilling) culturally normal | В Snipe-IT/GLPI картридж — disposable. В РФ — refillable, имеет lifecycle "На заправке" | YES — статус есть |
| **Подразделение / отдел** | "В бухгалтерию", "в IT-отдел" — стандартное измерение учёта | **MISSING** — рекомендация добавить |
| **Описание состояния** ("Б/У", "Новое в заводской упаковке") | Russian-specific lexicon, нестандартный для англоязычных систем | YES — autocomplete с подсказками |
| **Печатные формы с логотипом + реквизитами** организации | Требование — документ должен быть на бланке организации | YES — настройки + шаблоны |
| **М-15, ОС-1, МХ-1** — формы 1С | Trackly не пытается соответствовать унифицированным формам Госкомстата (отмененным с 2013, но ещё используемым). Это **сознательный выбор** — кастомный шаблон проще | Корректное решение для не-бухгалтерской системы |

## Open Questions for User / Future Phases

1. **"Расходник" внутри типов устройств** — пересекается с разделом "Картриджи". Перечитать spec и определить boundary.
2. **Custom fields** — добавить хотя бы базовую таблицу `device_custom_fields` или жить с тем, что есть в spec (Техн. характеристики, Комплектация — freeform строки)?
3. **Audit log** — отдельная таблица событий или per-entity history columns? Рекомендуется отдельная таблица `activity_log(entity_type, entity_id, action, user_id, before_json, after_json, created_at)`.
4. **HTTPS в LAN-режиме** — self-signed cert by default? Без HTTPS AD-bind небезопасен, но и self-signed раздражает пользователей с предупреждением браузера. Решить, когда дойдёт до web-server фазы.
5. **Логотип организации** — где хранится? BLOB в БД (переносится с portable) или файл рядом с БД (быстрее, но один лишний файл)? Рекомендуется BLOB.
6. **REST API** — есть в anti-features в v1, но webhook outbound уже планируется. Webhook без REST GET-endpoints — половинчатое решение. Решить по факту запроса.
7. **Print server vs print client мониторинг** — spec говорит "USB-принтеры на компьютерах (подключённых и общих)". Это требует агента на каждой машине или WMI/RPC удалённо? Большая разница в архитектуре.

## Sources

- [Snipe-IT product features](https://snipeitapp.com/product)
- [Snipe-IT managing assets docs](https://snipe-it.readme.io/docs/managing-assets)
- [Snipe-IT custom fields](https://snipe-it.readme.io/docs/custom-fields)
- [Snipe-IT asset acceptance / signature](https://snipe-it.readme.io/docs/requiring-acceptance)
- [Snipe-IT LDAP sync & login](https://snipe-it.readme.io/docs/ldap-sync-login)
- [Snipe-IT review and limitations](https://www.goworkwize.com/blog/snipe-it-review)
- [GLPI features overview](https://www.glpi-project.org/en/features/)
- [GLPI Printers documentation](https://help.glpi-project.org/documentation/modules/assets/printers)
- [GLPI Inventory FAQ](https://help.glpi-project.org/faq/glpi/inventory)
- [GLPI User profiles documentation](https://help.glpi-project.org/documentation/modules/administration/profiles/profiles)
- [GLPI user & permission system (DeepWiki)](https://deepwiki.com/glpi-project/glpi/6-user-and-permission-system)
- [GLPI asset model and inventory (DeepWiki)](https://deepwiki.com/glpi-project/glpi/5.1-asset-model-and-inventory)
- [ManageEngine ServiceDesk Plus — Asset Management](https://www.manageengine.com/products/service-desk-msp/help/adminguide/configurations/asset_management/inventory-configurations.html)
- [ManageEngine AssetExplorer ITAM](https://www.manageengine.com/products/asset-explorer/)
- [ManageEngine asset life-cycle management](https://www.manageengine.com/products/asset-explorer/it-asset-life-cycle-management.html)
- [Lansweeper asset discovery](https://www.lansweeper.com/product/asset-discovery/)
- [Lansweeper printer assets](https://www.lansweeper.com/asset/printer/)
- [PaperCut toner levels (SNMP)](https://www.papercut.com/help/manuals/ng-mf/applicationserver/printer-toner-levels/)
- [PaperCut hardware page count via SNMP](https://www.papercut.com/help/manuals/ng-mf/applicationserver/printer-hwcheck/)
- [Printer-MIB OID reference](https://mibbrowser.online/mibdb_search.php?mib=Printer-MIB)
- [Snipe-IT vs GLPI comparison](https://www.softwaresuggest.com/compare/snipe-it-vs-glpi)
- [Russian — инвентарные номера в 2026](https://ppt.ru/art/inventarizaciya/kak-prisvaivayutsya-inventarnye-nomera)
- [Russian — акт приёма-передачи ТМЦ работнику](https://assistentus.ru/forma/akt-priema-peredachi-materialnyh-cennostej-rabotniku/)
- [Russian — учёт ИТ-оборудования (Habr)](https://habr.com/ru/articles/750256/)

---
*Feature research for: IT asset & cartridge/printer tracking system (self-hosted, single-org, Russian-context)*
*Researched: 2026-05-24*
