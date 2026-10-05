# Roadmap: Trackly

## Milestones

- ✅ **v1.0 — Базовый учёт** (shipped 2026-06-19) — `MILESTONES.md`
- ✅ **v1.1 — AD, сотрудники, картриджная взаимосвязь** (shipped 2026-06-26) — `milestones/v1.1-*`
- ✅ **v1.1.2 — Пост-релизные доработки UX и печати** (Фазы 18–22, shipped 2026-07-15) — `milestones/v1.1.2-*`
- ✅ **v1.2 — Редизайн UI и дизайн-система** (Фазы 23–30, shipped 2026-07-29) — `milestones/v1.2-*`
- ✅ **v1.3 — AD-SSO паритет + полировка превью печати** (Фазы 31–33, shipped 2026-08-08) — `milestones/v1.3-*`
- ✅ **v1.3.3 — Печатные формы и приватность данных** (Фазы 34–38, shipped 2026-08-19) — `milestones/v1.3.3-*`
- 🚧 **v1.4 — Карта и осмысленное размещение** (Фазы 39–45, in progress, roadmap создан 2026-08-22)

## Phases

### 🚧 v1.4 Карта и осмысленное размещение (Фазы 39–45, in progress)

**Milestone Goal:** заменить свободнотекстовое «Размещение» деревом мест, ввести группы
устройств (АРМ, системный блок, разбор) и историю перемещений, дать схематичную карту с
расстановкой устройств по этажам и территориям, и научить заявки на картриджи ранжировать
принтеры по месту автора. Расширено 2026-10-04: группы универсальны (тип задаёт поведение и
набор свойств), к ним примыкают ремонтные заявки с перемещениями, вложения и раздел «Устройства»
для сотрудника.

Ключевой архитектурный инвариант вехи: семантика мест (дерево) отделена от геометрии (чертёж) —
место работает без плана, чертёж — необязательный слой поверх. Фазы 39–42 намеренно не содержат
графики — если редактор планов (Фаза 44) окажется тяжелее ожидаемого, веха всё равно приезжает
целой с деревом мест, историей, группами и умными заявками.

- [x] **Phase 39: Дерево мест** - Свободнотекстовое «Размещение» заменено деревом мест (completed 2026-08-26)
  произвольной вложенности с поиском по полному пути.

- [x] **Phase 39.1: Формат пути Места в UI** - Вариант сокращения пути выбирается в приложении (completed 2026-08-31)
  (настройка организации + переопределение на конкретном месте с наследованием), а не в конфиге.

- [x] **Phase 39.2: Долг фазы 39.1** - Настройки формата пути получают единственного владельца (completed 2026-09-01)
  дефолтов, перестают ломаться на битых состояниях БД, экран в Настройках доводится до
  принятых в проекте форм (6 Warning + 4 Info из `39.1-REVIEW.md`).

- [x] **Phase 40: История перемещений** - Каждая смена места устройства/картриджа фиксируется в (completed 2026-09-04)
  истории, включая автоматическую смену места актом.

- [x] **Phase 40.1: Пробелы аудита v1.4** *(INSERTED 2026-09-17)* - Два блокера с межфазных
  стыков закрыты: удаление места с историей перемещений отклоняется по-русски с точными
  счётчиками, а фильтр по типу устройства в отчёте «Перемещения» достижим из интерфейса
  (completed 2026-09-18)

- [x] **Phase 40.2: Шаблоны инвентарных номеров** *(INSERTED 2026-09-18)* - CRUD шаблонов номеров (completed 2026-09-22)
  в настройках организации и автоподстановка следующего номера (с учётом разрывов) в попапах
  создания устройства, принтера, акта и картриджа/фотобарабана; запрет дубликатов.

- [x] **Phase 40.3: Долг аудита v1.4** *(INSERTED 2026-09-22)* - Очистка полей устройства, (completed 2026-09-23)
  номер акта в тосте возврата, «Завести принтер» через шаблоны и INV-7, повторная проверка FK
  после миграций (WR-03, N-1, N-2, N-4 из аудита 2026-09-22).

- [x] **Phase 40.4: Долг аудита v1.4, раунд 2** *(INSERTED 2026-09-24)* - Откат акта не (completed 2026-09-25)
  воскрешает очищенные поля устройства, импорт CSV обновляет дерево мест одним WS-событием,
  автоподстановка номера не блокирует массовое создание, номер акта в печатной форме — из
  одного источника (NEW-1, N-3, NEW-3, NEW-2 из аудита 2026-09-23).

- [x] **Phase 40.5: Нумерация возвратов: плотные суффиксы в1..вN** *(INSERTED 2026-09-26)* - Возврат (completed 2026-09-30)
  всегда отображается как «в1», «в2» … «вN»; безцифрового «в» больше нет. Удаление возврата
  оставляет дырку, которую занимает следующий СОЗДАННЫЙ возврат (наименьший свободный номер
  вместо MAX+1). Существующие акты не перенумеровываются. Чинится до Фазы 41 — акты АРМ
  унаследовали бы дефект через тот же контекст печати (риск F7 аудита).

- [ ] **Phase 41: Группы: модель и редактор** - Устройства собираются в группы с типами
  (АРМ, Системный блок, Разбор), своими свойствами и местом; состав следует за местом группы.

- [ ] **Phase 41.1: Группы × Устройства** *(INSERTED 2026-10-04)* - Быстрые действия в
  контекстном меню, якорное замещение устройства группой, разбор с сохранением происхождения
  деталей, статусы «На списание» и «В ремонте».

- [ ] **Phase 41.2: Группы × Акты и Места** *(INSERTED 2026-10-04)* - Группа выбирается в акте
  целиком и отображается в дереве мест вместо входящих в неё устройств.

- [ ] **Phase 41.3: Вложения** *(INSERTED 2026-10-04)* - Файлы рядом с БД, метаданные в БД,
  отдача под сессией и просмотр фото и PDF.

- [ ] **Phase 41.4: Заявки: Ремонт** *(INSERTED 2026-10-04)* - Третий вид заявки с жизненным
  циклом перемещений: на склад, в сервис и обратно на своё место.

- [ ] **Phase 41.5: Переписка в заявке** *(INSERTED 2026-10-04)* - Двусторонняя переписка с
  вложениями вместо одностороннего комментария специалиста.

- [ ] **Phase 41.6: Раздел «Устройства» для сотрудника** *(INSERTED 2026-10-04)* - Сотрудник
  видит устройства своих АРМ и создаёт из списка заявки на ремонт и на картридж.

- [ ] **Phase 42: Умный подбор принтера в заявке** - Принтеры в заявке на картридж ранжируются и
  автоподставляются по месту автора и его АРМ.

- [ ] **Phase 43: Карта — просмотр** - Администратор/менеджер видит схематичный план этажа/зоны
  с устройствами и АРМ, кликабельный и печатаемый.

- [ ] **Phase 44: Карта — редактор планов** - Администратор рисует планы помещений на SVG-сетке
  и расставляет устройства перетаскиванием, с undo/redo и слоями.

- [ ] **Phase 45: Живые статусы на карте** - Маркеры принтеров окрашены по SNMP-статусу/тонеру и
  обновляются на открытой карте через WebSocket без перезагрузки.

<details>
<summary>✅ v1.3.3 Печатные формы и приватность данных (Фазы 34–38) — SHIPPED 2026-08-19</summary>

- [x] Phase 34: Единая шапка документов (6/6 plans) — completed 2026-08-11
- [x] Phase 35: Тело акта приёма-передачи (7/7 plans) — completed 2026-08-12
- [x] Phase 36: Пагинация акта по количеству устройств (6/6 plans) — completed 2026-08-13
- [x] Phase 37: Приватность данных (4/4 plans) — completed 2026-08-18
- [x] Phase 38: Nyquist-покрытие Фазы 32 (0/0 plans) — completed 2026-08-18

Полная детализация — `milestones/v1.3.3-ROADMAP.md`; требования — `milestones/v1.3.3-REQUIREMENTS.md`;
аудит — `milestones/v1.3.3-MILESTONE-AUDIT.md` (11/11 требований, 5/5 фаз, tech_debt, без блокеров).

Живой UAT печати выполнен на Windows 2026-08-19 из релизной сборки `v1.3.3` — дефектов нет.

</details>

<details>
<summary>✅ v1.3 AD-SSO паритет + полировка превью печати (Фазы 31–33) — SHIPPED 2026-08-08</summary>

- [x] Phase 31: Служебный AD-bind — ФИО и роли из AD-групп (4/4 plans) — completed 2026-08-03
- [x] Phase 32: Авто-админ по списку логинов + релиз SSO в main (5/5 plans) — completed 2026-08-04
- [x] Phase 33: Полировка предпросмотра печати (4/4 plans) — completed 2026-08-04

Полная детализация — `milestones/v1.3-ROADMAP.md`; требования — `milestones/v1.3-REQUIREMENTS.md`;
аудит — `milestones/v1.3-MILESTONE-AUDIT.md` (6/6 требований, tech_debt, без блокеров).

Релизы вехи: `v1.3.0` (SSO), `v1.3.1` (LDAP plain/StartTLS + Paged.js-печать),
`v1.3.2` (синхронизация ФИО при смене фамилии в AD).

</details>

<details>
<summary>✅ v1.2 Редизайн UI и дизайн-система (Фазы 23–30) — SHIPPED 2026-07-29</summary>

- [x] Phase 23: Design tokens foundations
- [x] Phase 24: Base components
- [x] Phase 25: Tables / Dropdown
- [x] Phase 26: Windows with mockup
- [x] Phase 27: Core workflow windows
- [x] Phase 28: Support / admin windows
- [x] Phase 29: Login & employee shell
- [x] Phase 30: Quality — a11y & platform parity (9/9 plans) — completed 2026-07-29

Полная детализация — `milestones/v1.2-ROADMAP.md`; требования — `milestones/v1.2-REQUIREMENTS.md`;
аудит — `milestones/v1.2-MILESTONE-AUDIT.md` (26/26, tech_debt, без блокеров).

</details>

Более ранние milestone'ы (v1.0 / v1.1 / v1.1.2) — в `milestones/` и `MILESTONES.md`.

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
| ----- | --------- | --------------- | ------ | --------- |
| 23–29 | v1.2 | — | Complete | (см. архив) |
| 30. Quality — a11y & parity | v1.2 | 9/9 | Complete | 2026-07-29 |
| 31. Служебный AD-bind — ФИО и роли из AD-групп | v1.3 | 4/4 | Complete | 2026-08-03 |
| 32. Авто-админ по логинам + релиз SSO в main | v1.3 | 5/5 | Complete | 2026-08-04 |
| 33. Полировка предпросмотра печати | v1.3 | 4/4 | Complete | 2026-08-04 |
| 34. Единая шапка документов | v1.3.3 | 6/6 | Complete | 2026-08-11 |
| 35. Тело акта приёма-передачи | v1.3.3 | 7/7 | Complete | 2026-08-12 |
| 36. Пагинация акта по количеству устройств | v1.3.3 | 6/6 | Complete | 2026-08-13 |
| 37. Приватность данных | v1.3.3 | 4/4 | Complete | 2026-08-18 |
| 38. Nyquist-покрытие Фазы 32 | v1.3.3 | 0/0 | Complete | 2026-08-18 |
| 39. Дерево мест | v1.4 | 22/22 | Complete    | 2026-08-26 |
| 39.1. Формат пути Места в UI | v1.4 | 10/10 | Complete    | 2026-08-31 |
| 39.2. Долг фазы 39.1 | v1.4 | 5/5 | Complete   | 2026-09-01 |
| 40. История перемещений | v1.4 | 35/35 | Complete    | 2026-09-04 |
| 40.1. Пробелы аудита v1.4 | v1.4 | 4/4 | Complete    | 2026-09-18 |
| 40.2. Шаблоны инвентарных номеров | v1.4 | 15/15 | Complete    | 2026-09-22 |
| 40.3. Долг аудита v1.4 | v1.4 | 9/9 | Complete    | 2026-09-23 |
| 40.4. Долг аудита v1.4, раунд 2 | v1.4 | 5/5 | Complete    | 2026-09-25 |
| 40.5. Нумерация возвратов: плотные суффиксы в1..вN | v1.4 | 14/14 | Complete    | 2026-09-30 |
| 41. Группы: модель и редактор | v1.4 | 32/34 | In Progress|  |
| 41.1. Группы × Устройства | v1.4 | 0/TBD | Not started | - |
| 41.2. Группы × Акты и Места | v1.4 | 0/TBD | Not started | - |
| 41.3. Вложения | v1.4 | 0/TBD | Not started | - |
| 41.4. Заявки: Ремонт | v1.4 | 0/TBD | Not started | - |
| 41.5. Переписка в заявке | v1.4 | 0/TBD | Not started | - |
| 41.6. Раздел «Устройства» для сотрудника | v1.4 | 0/TBD | Not started | - |
| 42. Умный подбор принтера в заявке | v1.4 | 0/TBD | Not started | - |
| 43. Карта — просмотр | v1.4 | 0/TBD | Not started | - |
| 44. Карта — редактор планов | v1.4 | 0/TBD | Not started | - |
| 45. Живые статусы на карте | v1.4 | 0/TBD | Not started | - |

## Phase Details

### Phase 39: Дерево мест

**Goal**: Заменить свободнотекстовое поле «Размещение» деревом мест произвольной вложенности —
семантика места существует и работает независимо от того, есть ли для неё чертёж.

**Depends on**: Nothing (первая фаза вехи)

**Requirements**: PLC-01, PLC-02, PLC-03, PLC-04, PLC-05, PLC-06

**Success Criteria** (what must be TRUE):

  1. Администратор может построить дерево мест (территория / зона / здание / этаж / помещение /
     уличный объект) произвольной вложенности, переименовать и переместить узел, не потеряв
     привязок устройств.

  2. Этаж имеет числовой уровень, допускающий 0 и отрицательные значения (подвал); этажи
     сортируются по уровню, а не по имени.

  3. Пользователь выбирает место устройства и картриджа из дерева с поиском по полному пути
     («Здание А / 2 этаж / 214»); свободнотекстовое поле «Размещение» и таблица `locations` из
     приложения удалены — формы, списки, отчёты, автокомплиты и печатные формы используют место
     из дерева.

  4. Переименование или перемещение узла дерева мгновенно отражается в полнотекстовом поиске и во
     всех списках без ручной переиндексации.

  5. Открыв любое место, пользователь видит одним списком всё размещённое в нём и во вложенных
     местах (устройства, АРМ, принтеры, картриджи).

**Plans:** 22/22 plans complete
Plans:
**Wave 1**

- [x] 39-01-PLAN.md — places schema (V037/V038), no-data-migration, migration_idempotency coverage
- [x] 39-02-PLAN.md — domain::places contracts + PlaceRepository trait + auth.rs D-20 split
- [x] 39-03-PLAN.md — acts/cartridges/printers/requests domain field renames onto place_id

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 39-04-PLAN.md — SqlitePlaceRepository (CRUD, cycle-check move, delete-conflict, subtree/storage queries) + places_crud.rs

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 39-05-PLAN.md — PlaceService mutations (create/rename/move/archive/delete) + places_move_cycle.rs/places_delete_blocked.rs
- [x] 39-06-PLAN.md — device_service/devices_sqlite/domain::devices onto place_id, drop resolve_location_id_in_tx
- [x] 39-09-PLAN.md — cartridge_service/cartridges_sqlite onto place_id, drop upsert_location_in_tx
- [x] 39-10-PLAN.md — report_service/request_service/requests_sqlite/printers_sqlite onto place_full_paths (D-28 subtree filter)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 39-07-PLAN.md — act_service create/update onto place_id + place_path_snapshot capture
- [x] 39-08-PLAN.md — PlaceService reads incl. Cyrillic-safe search (places_search.rs/places_contents.rs)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 39-11-PLAN.md — act_service return-flow onto place_id + act_handover.minijinja place_path rename + acts_place_snapshot.rs
- [x] 39-12-PLAN.md — tauri_cmds/places.rs + http/places.rs + specta_export + role_endpoint_matrix D-20 coverage

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 39-13-PLAN.md — PlacePicker.svelte (tree + search modes, D-18 create-row) + showcase
- [x] 39-22-PLAN.md — test-fixup: 31 pre-existing test files onto renamed place_id/place_path vocabulary

**Wave 7** *(blocked on Wave 6 completion)*

- [x] 39-15-PLAN.md — device-family PlacePicker wiring (form, autocomplete, printer create, CSV import)
- [x] 39-16-PLAN.md — cartridge-family PlacePicker wiring (form, 5 transition ops, D-11.3 checkbox)
- [x] 39-17-PLAN.md — act-family PlacePicker wiring (form, bulk return, per-row override)
- [x] 39-18-PLAN.md — reports PlacePicker wiring (D-26 short-path columns, D-28 subtree filter)
- [x] 39-19-PLAN.md — PlaceFormModal + PlaceMoveModal

**Wave 8** *(blocked on Wave 7 completion)*

- [x] 39-14-PLAN.md — /places route + sidebar + PlacesMasterDetail/PlaceTree/PlaceTreeNode

**Wave 9** *(blocked on Wave 8 completion)*

- [x] 39-20-PLAN.md — PlaceContents.svelte (PLC-06 content screen) + end-to-end checkpoint

**Wave 10** *(blocked on Wave 9 completion)*

- [x] 39-21-PLAN.md — delete LocationAutocomplete.svelte, full-repo sweep, CI gate, DB-upgrade checkpoint (depends on 39-22)

**UI hint**: yes

### Phase 39.1: Формат пути Места в UI (INSERTED)

**Goal**: Как сокращать путь места — решение пользователя в приложении, а не строка в конфиге:
умолчание задаётся для всей организации в Настройках, а любое место может его переопределить,
наследуя выбор родителя по умолчанию.

**Depends on**: Phase 39 (дерево мест), quick 260827-ui3 (текущая реализация через `trackly.config.toml`,
которую эта фаза заменяет)

**Requirements**: PLC-07, PLC-08 (заводятся этой фазой — см. REQUIREMENTS.md)

**Success Criteria** (what must be TRUE):

1. В «Настройки → Организация» есть блок выбора сокращения пути для всей организации:
   радио-кнопки «Крайние» (по умолчанию) / «Два последних» / «Последнее», с живым примером
   на двух образцах разной формы — «Здание А / 1 этаж / 1-05» и «Территория А / Объект Х / помещение 3».

2. Там же — текстовое поле разделителя, по умолчанию « // » с сохранением пробелов, так что
   значение «, » даёт перечисление через запятую.

3. В форме создания/редактирования Места есть дропдаун «Вариант сокращения» со значениями
   «Как у родителя» (по умолчанию), «Крайние», «Два последних», «Последнее».

4. «Как у родителя» разрешается по цепочке предков; если ни один предок не переопределил вариант,
   применяется умолчание организации.

5. Выбранный вариант виден везде, где путь сокращается (отчёты, списки устройств и картриджей,
   содержимое места), а полный путь остаётся в `title`.

6. Ключ `[organization] place_path_display` из `trackly.config.toml` удалён — источник истины
   переезжает в приложение.

**Открытые вопросы для discuss-фазы:**

- Где разрешается наследование — на бэкенде (обход предков рядом с `place_full_paths`, отдельное
  поле в DTO) или на фронте по уже загруженному дереву. Списки и отчёты несут `place_path` строкой
  и не всегда несут `place_id`, что склоняет к бэкенду.

- Применяется ли разделитель ко всем вариантам (тогда «Два последних» с « // » даёт
  «2 этаж // Кабинет 214») или только к «Крайние». Формулировка запроса — «перечисление получится
  через запятую» — читается как «разделитель соединяет показанные сегменты».

- Что показывает вариант «Последнее» для пути из одного сегмента (вероятно, его же).

**Plans:** 10/10 plans complete

Plans:
**Wave 1**

- [x] 39.1-01-PLAN.md — миграция V039 (path_variant_override + app_settings умолчания + вью
  place_effective_variant) + интеграционные тесты наследования + shorten_place_path в trackly-core

**Wave 2** *(blocked on Wave 1)*

- [x] 39.1-02-PLAN.md — settings_get/set_place_path_defaults (backend, оба транспорта) + Case 49
- [x] 39.1-03-PLAN.md — place_path_short в списках устройств (list/search_fts/list_grouped) + D-24 тест-замок export_csv
- [x] 39.1-04-PLAN.md — place_path_short в списке картриджей
- [x] 39.1-06-PLAN.md — печатная форма акта: place_path_short по текущему варианту + снапшот шаблона v27

**Wave 3** *(blocked on Wave 2)*

- [x] 39.1-05-PLAN.md — place_path_short в 5 доменах отчётов + row_field(shorten) CSV/PDF асимметрия
- [x] 39.1-07-PLAN.md — places_set_path_variant (backend) + PlaceContents place_path_short + Case 50
- [x] 39.1-08-PLAN.md — OrgSettings.svelte: подраздел «Формат отображения пути места» (UI)

**Wave 4** *(blocked on Wave 3)*

- [x] 39.1-09-PLAN.md — PlaceFormModal.svelte: дропдаун «Вариант сокращения» (UI)
- [x] 39.1-10-PLAN.md — фронт переключается на place_path_short + удаление устаревшего конфига (D-22)

### Phase 39.2: Долг фазы 39.1 (INSERTED)

**Goal**: Настройки формата пути места получают единственного владельца значения по
умолчанию, перестают ломаться на битых и недостижимых-сегодня состояниях БД, а их экран
в Настройках доводится до принятой в проекте формы. Ни одна из находок не блокирует
success criteria фазы 39.1 — фаза закрывает накопленный после неё советательный долг,
чтобы он не тянулся в Фазу 40 (история перемещений читает те же настройки пути).

**Depends on**: Phase 39.1 (все находки — из её `39.1-REVIEW.md`; WR-03 и WR-04 уже
закрыты коммитами `10707242`/`8fa995e5` и с 2026-09-01 удерживаются гейтами
`ui/scripts/check-placepath-parity.mjs` и `ui/scripts/check-place-path-short.mjs`)

**Requirements**: PLC-07, PLC-08 (существующие, заведены фазой 39.1 — новых не требуется)

**Источник**: `39.1-REVIEW.md`, находки, оставшиеся открытыми на 2026-09-01.
Проверены против HEAD, а не взяты из текста ревью.

**Success Criteria** (what must be TRUE):

1. **Дефолты формата пути имеют ровно одного владельца.** Сегодня
   `read_path_display_separators` скопирована дословно 4 раза
   (`devices_sqlite.rs:77`, `cartridges_sqlite.rs:105`, `places_sqlite.rs:196`,
   `report_service.rs:280`), пятая копия заинлайнена в `act_service.rs:3051`, а литералы
   `" // "` / `" / "` / `"ends"` живут ещё и в `V039`, в `build_settings_get_place_path_defaults`
   и в `OrgSettings.svelte` — 8 мест, обязанных меняться синхронно (WR-08). После фазы
   функция и константы живут в одном модуле, остальные места импортируют их.

2. **Запись org-дефолтов атомарна.** `build_settings_set_place_path_defaults`
   (`settings_org.rs:385`) делает три независимых `conn.execute` в autocommit: отказ на
   втором операторе оставляет «вариант новый, разделители старые» И возвращает `Err` —
   пользователь видит «не сохранилось», а часть настроек уже изменилась (WR-05).
   После фазы три записи идут одной транзакцией, что доказано тестом на частичный отказ.

3. **Чтение org-дефолтов симметрично записи.** GET отдаёт `variant` сырым из
   `app_settings` (WR-05/IN-05): мусор в БД → UI молча покажет «Крайние» и при следующем
   «Сохранить» перезапишет настройку без действия пользователя. После фазы GET валидирует
   значение так же, как SET, с `tracing::warn!` на fallback.

4. **Битые и оборванные состояния БД не выносят экраны целиком.** Неизвестный токен в
   `places.path_variant_override` роняет весь список мест через `FromSqlConversionFailure`
   (`places_sqlite.rs:59-72`, IN-01), тогда как четыре других потребителя того же токена
   мягко деградируют через `.ok()`. Вью `place_effective_variant` выбрасывает всех потомков
   soft-deleted места и даёт NULL всем местам при отсутствии ключа `place_path_variant`
   (WR-02) — оба состояния сегодня недостижимы штатным кодом, но не покрыты ни одним тестом,
   а желаемое поведение нигде не зафиксировано. После фазы поведение зафиксировано тестами
   и деградирует мягко.

5. **Список не теряет путь, который у него есть.** `place_path_short ?? '—'` в
   `DeviceListRow:68`, `DeviceGroupRow:156`, `CartridgeListRow:102` утверждает «места нет»,
   хотя полный путь лежит рядом в `title` (WR-01). Остальные потребители деградируют к
   полному пути. После фазы деградация единообразна.

6. **Экран «Формат пути» в Настройках соответствует принятым в проекте формам.**
   Кнопка «Сохранить» активна при заведомо невалидном разделителе и превращает inline-ошибку
   в дублирующий красный тост после round-trip'а (WR-06); `<Radio>` обёрнут во внешний
   `<label>`, внутри которого компонент рендерит свой — вложенный `<label>` запрещён
   спецификацией, все прочие call-site'ы проекта используют `<Radio ...>Текст</Radio>`
   (WR-07); `monoReadout` подсвечивает только U+0020, оставляя таб/NBSP/узкий пробел
   невидимыми, а `.field-hint`/`.field-error` не связаны с полем через `aria-describedby`
   (IN-04).

7. **Матрица ролей не имеет дырки на новой мутации.** `places_set_path_variant` попал
   только в Case 50 (Employee → 403), но не в Case 45 (Manager → все `places_*` мутации
   → 403), хотя гейт у неё тот же `Action::MutatePlaces`; для
   `settings_set_place_path_defaults` Manager тоже не покрыт (IN-02).

**Не входит в фазу**: изменение самой формулы сокращения, формата хранения настроек и
поведения, которое пользователь видит на исправной БД. Фаза не должна менять ни один
результат, зафиксированный в `scripts/fixtures/place-path/shorten-cases.json` — оба гейта
паритета обязаны остаться зелёными.

**UI hint**: yes (пункты 5-6 трогают `OrgSettings.svelte` и три строки списков)

**Plans:** 5/9 plans complete (5 исполнены; 40.3-06..40.3-09 — раунд закрытия пробелов gaps_found после верификации 2026-09-22)

Plans:

- [x] 39.2-01-PLAN.md — волна 1, Wave 0 фазы: тесты вью `place_effective_variant` (WR-02a/WR-02b) + мягкая деградация неизвестного токена в дереве мест (IN-01)
- [x] 39.2-02-PLAN.md — волна 2, единственный владелец дефолтов формата пути в Rust: модуль `place_path_settings` + перевод 6 мест + связь константы с сидом V039 тестом (WR-08, 7 из 8 мест)
- [x] 39.2-03-PLAN.md — волна 3, атомарная запись org-дефолтов (WR-05), валидация варианта на чтении (IN-05), Manager-кейсы в матрице ролей (IN-02)
- [x] 39.2-04-PLAN.md — волна 4, деградация `place_path_short` к полному пути в репозиториях устройств и картриджей (WR-01)
- [x] 39.2-05-PLAN.md — волна 5, экран «Формат пути»: снятие литералов-дефолтов из состояния (WR-08, 8-е место), `disabled` у кнопки (WR-06), форма `<Radio>` + новый структурный гейт из 4 инвариантов (WR-07), `monoReadout` и `aria-describedby` (IN-04)

### Phase 40: История перемещений

**Goal**: Каждая смена места устройства или картриджа наблюдаема — вручную, актом или (структурно,
на будущее) перетаскиванием на карте — с указанием откуда, куда, когда, кем и почему.

**Depends on**: Phase 39 (дерево мест — источник значений «откуда»/«куда» в истории)

**Requirements**: HST-01, HST-02, HST-03, HST-04

**Success Criteria** (what must be TRUE):

  1. Пользователь видит в карточке устройства и картриджа таймлайн перемещений: откуда, куда,
     когда, кем и по какой причине.

  2. Ручное изменение места фиксируется в истории с причиной «вручную»; схема причины уже
     предусматривает будущий источник «перетаскиванием на карте» — задействуется в Фазе 44, когда
     появится сам редактор.

  3. Акт приёма-передачи автоматически меняет место переданных устройств и создаёт запись в
     истории со ссылкой на номер акта.

  4. Пользователь может получить отчёт о перемещениях за период с фильтром по месту и типу
     устройства.

**Plans**: 32 plans in 10 waves (20 initial + 12 gap-closure, UAT 2026-09-03 + re-verification 2026-09-03 + live UAT re-run round 3 2026-09-03)

Plans:
**Wave 1**

- [x] 40-01-PLAN.md — миграция V040 (place_movements) + домен MovementSource/MovementEntityKind + Wave 0 migration test
- [x] 40-02-PLAN.md — промоушен compute_place_path_short в place_path_display.rs (единственный владелец, D-18)
- [x] 40-03-PLAN.md — caller: &Identity в device_service::update (сигнатурная плотина, без записи истории)
- [x] 40-04-PLAN.md — caller: &Identity в cartridge_service::update/transition + before-fetch (сигнатурная плотина)

**Wave 2** *(blocked on Wave 1)*

- [x] 40-05-PLAN.md — репозиторий place_movements_sqlite.rs (insert/record_movement_if_applicable/delete_by_act_id/get_history)
- [x] 40-06-PLAN.md — caller: &Identity в act_service create/update/do_return/update_return (сигнатурная плотина)
- [x] 40-11-PLAN.md — отчёт «Перемещения», часть A: ReportFilter/ReportRow + list_movements/query_movements_inner

**Wave 3** *(blocked on Wave 2)*

- [x] 40-07-PLAN.md — запись перемещений: device_service::update (D-27/D-04/D-06) + тесты devices
- [x] 40-08-PLAN.md — запись перемещений: cartridge update/transition + вложенный auto-return (Pitfall 3) + осмысленный note на операцию (D-05) + тесты cartridges
- [x] 40-09-PLAN.md — запись перемещений в актах (HST-03): create/update/do_return/update_return + NULL-skip (D-06/Pitfall 4)
- [x] 40-10-PLAN.md — таймлайн: MovementEntryDto + PlaceMovementService + оба транспорта (Action::ReadPlaces)
- [x] 40-12-PLAN.md — отчёт «Перемещения», часть B: columns_for/gate ReadPlaces (не ReadData!) + HTTP + экспорт
- [x] 40-13-PLAN.md — D-28 массовый перенос: PlaceService::move_subtree_contents + оба транспорта

**Wave 4** *(blocked on Wave 3)*

- [x] 40-14-PLAN.md — матрица ролей: новые Cases для таймлайна/отчёта/массового переноса (оба транспорта, IN-02)
- [x] 40-15-PLAN.md — MovementTimeline.svelte (общий компонент) + ActsPage ?id= (D-19)
- [x] 40-18-PLAN.md — UI отчёта: ReportSubNav 4-й домен + два PlacePicker + колонки + бейдж «Удалено»
- [x] 40-19-PLAN.md — UI массового переноса: кнопка + confirm-модалка на PlaceContents.svelte
- [x] 40-20-PLAN.md — отмена акта удаляет свои записи истории (D-03/Pitfall 5), вынесен из 40-09 по ревью планировщика

**Wave 5** *(blocked on Wave 4)*

- [x] 40-16-PLAN.md — таймлайн в PlaceEntityViewModal + «Просмотр» в DeviceContextMenu (D-14)
- [x] 40-17-PLAN.md — CartridgeDetail (переименование + новая секция) + PrinterDetail (D-16/D-21)

**Gap closure (UAT 2026-09-03, 7 gaps — см. 40-UAT.md)**

**Wave 6 (gap-closure wave 1)**

- [x] 40-21-PLAN.md — cartridge-does-not-follow-printer: каскад места «принтер → его картриджи» + обратная запись места принтеру (координация cargo test с 40-24/40-26 внутри волны)
- [x] 40-24-PLAN.md — timeline-act-link-wrong-subsection: подраздел «Акты» из act_type/archived + канонический номер возврата + пояснение D-06 (координация cargo test с 40-21/40-26 внутри волны)
- [x] 40-25-PLAN.md — deleted-badge-missing-in-live-report-table: reportType={reportTypeKey()} + гейт check-report-type-parity.mjs (первым дописывает в ui/package.json "lint")
- [x] 40-26-PLAN.md — grouped-device-list-place-inversion: list_by_ids short-path fix (A) + place_distinct_count (B) (координация cargo test с 40-21/40-24 внутри волны)

**Wave 7 (gap-closure wave 2, blocked on 40-21 и/или 40-25)**

- [x] 40-22-PLAN.md — return-to-stock-empty-place-field: автоподстановка последнего складского места из place_movements (blocked on 40-21 — общий файл cartridges_sqlite.rs)
- [x] 40-27-PLAN.md — lan-print-duplicate-first-page: идемпотентный printViaTopLevel + re-entrancy guard + гейт check-print-idempotency.mjs (blocked on 40-25 — обе правки дописывают в одну строку "lint" ui/package.json)

**Wave 8 (gap-closure wave 3, blocked on 40-21/40-22)**

- [x] 40-23-PLAN.md — OperationModal: место необязательно при установке в принтер + подсказка автоподстановки

**Gap closure round 2 (re-verification 2026-09-03T22:30, 4 gaps — см. 40-VERIFICATION.md)**

**Wave 9 (gap-closure wave 4, независимая от 40-21..27 — новые файлы/новые находки)**

- [x] 40-28-PLAN.md — cascade Some->None не трогает место картриджей (CR-03) + полная fallback-цепочка last_known_storage_place_in_tx через реальный поток (CR-02)
- [x] 40-29-PLAN.md — устранение вложенного ReaderPool.acquire() в get_timeline/query_movements_inner (CR-01) + acquire_timeout defense-in-depth + единый resolve_movement_act_number для таймлайна и отчёта (WR-10)

**Gap closure round 3 (живой UAT на Windows 2026-09-03, 2 gaps — см. 40-HUMAN-UAT.md; третий gap
UAT3-02 закрыт вне планов отладочной сессией, коммит 08e56c25)**

**Wave 10 (gap-closure wave 5)**

- [x] 40-30-PLAN.md — UAT3-01 backend: last_known_storage_place_in_tx обобщена на &Connection (переиспользована для дефолта from_refill) + новая most_common_to_refill_destination + оба транспорта + role-матрица
- [ ] 40-32-PLAN.md — UAT3-03: инвалидация statsCache в дереве «Места» после bulk-переноса содержимого (общий store + PlaceTree/PlaceContents)

**Wave 11 (gap-closure wave 6, blocked on 40-30)**

- [ ] 40-31-PLAN.md — UAT3-01 frontend: OperationModal.svelte подключает cartridges_operation_default_place к обоим диалогам (to_refill/from_refill)

**UI hint**: yes

### Phase 40.1: Пробелы аудита v1.4 (INSERTED)

**Goal**: Закрыть два блокера, найденных аудитом вехи на межфазных стыках — там, где пофазная
верификация по построению не смотрела: удаление места с историей перемещений и недостижимый из
интерфейса фильтр по типу устройства в отчёте «Перемещения».

**Depends on**: Phase 40 (история перемещений — оба блокера порождены её стыком с Фазой 39 и с UI)

**Requirements**: PLC-01, HST-04 (плюс попутно PLC-06, PLC-04)

**Success Criteria** (what must be TRUE):

  1. Попытка удалить место, участвовавшее в перемещениях, отклоняется русскоязычным сообщением
     с точными счётчиками — как это уже сделано для актов (CR-01), а не сырым текстом SQLite.
     Предполётная проверка считает ссылки из `place_movements`, и это зафиксировано тестом.

  2. Пользователь может построить отчёт «Перемещения» за период с фильтром одновременно по месту
     и по типу устройства — то есть HST-04 выполняется дословно, а не наполовину.

  3. Счётчики содержимого в дереве мест обновляются после смены места предмета с любого экрана,
     а не только после массового переноса.

  4. Заголовки CSV-экспорта отчётов совпадают с подписями на экране и в PDF.

**Plans:** 4/4 plans complete

Plans:
**Wave 1**

- [x] 40.1-01-PLAN.md — BLOCKER-1: referencing_movement_count в SubtreeStats + предполётная проверка + русское сообщение + регрессионный тест
- [x] 40.1-02-PLAN.md — BLOCKER-2: контрол «Тип устройства» в ReportFilters.svelte (ветка movements) + живая проверка
- [x] 40.1-03-PLAN.md — WARNING-1: три новых продюсера notifyPlaceContentChanged (PlaceEntityViewModal, CartridgesPage, DevicesPage) + INV-7 гейт + документирование каскада принтер-картридж (D-18) + раунд-2 фикс после провала живой проверки (точное старое/новое место записи вместо только текущего узла)
- [x] 40.1-04-PLAN.md — WARNING-4: export_csv принимает column_labels (русские заголовки) + build_reports_export_csv + переписанный тест report_movements.rs

**Источник:** `.planning/v1.4-MILESTONE-AUDIT.md` (аудит 2026-09-17) — BLOCKER-1, BLOCKER-2,
WARNING-1, WARNING-4.

**UI hint**: yes

### Phase 40.2: Шаблоны инвентарных номеров (INSERTED)

**Goal**: Администратор ведёт в «Настройки / Организация» шаблоны номеров с токенами
([YYYY], [YY], [MM], [DD], [X], [XXXX…]); в попапах «Новое устройство», «Новый принтер»,
«Новый акт» и «Новый картридж/фотобарабан» поле номера автоподставляет следующий номер
по последнему использованному в этом попапе шаблону (с переключателем ↑/↓ между «первым
разрывом» и «max+1»), выбор шаблона через кнопку «Вставка» с контекстным меню, запрет
дубликатов с попапом о занявшей номер записи и подтверждение номера, не совпадающего с шаблоном.

**Depends on**: Phase 40 (текущая модель устройств/актов/картриджей и счётчики V009)

**Requirements**: NUM-01, NUM-02, NUM-03, NUM-04, NUM-05, NUM-06, NUM-07, NUM-08, NUM-09, NUM-10, NUM-11, NUM-12, NUM-13, NUM-14, NUM-15, NUM-16

**Plans:** 15/15 plans complete

Plans:
**Wave 1**

- [x] 40.2-01-PLAN.md — схема V041 (number_templates + number_template_contexts, засев, DROP counters) + контракты trackly-core
- [x] 40.2-02-PLAN.md — mask.rs/homoglyphs.rs/sequence.rs (грамматика маски, гомоглифы, первый свободный/max+1)
- [x] 40.2-09-PLAN.md — UI-примитивы: IconInsertTemplate, Button iconOnly, ActionMenu ghost-md + portal
- [x] 40.2-11-PLAN.md — попапы цепочки D-01: NumberTakenPopup/NumberMismatchPopup/NumberScriptWarningPopup

**Wave 2** *(blocked on Wave 1)*

- [x] 40.2-03-PLAN.md — SqliteNumberTemplateRepository (CRUD + compute_next по трём пространствам)

**Wave 3** *(blocked on Wave 2)*

- [x] 40.2-04-PLAN.md — NumberTemplateService (CRUD, память контекстов, occupied, предупреждения)

**Wave 4** *(blocked on Wave 3)*

- [x] 40.2-05-PLAN.md — dual-transport для шаблонов (ManageSettings vs Mutate*), WsEvent::NumberSpaceChanged, role_endpoint_matrix

**Wave 5** *(blocked on Wave 4)*

- [x] 40.2-06-PLAN.md — Акты: V042 (number→TEXT), occupied по отображаемым номерам (D-06), отказ от counters
- [x] 40.2-07-PLAN.md — Картриджи/фотобарабаны: V043 (code unique среди живых), отказ от assign_code_in_tx
- [x] 40.2-08-PLAN.md — Устройства: V044 (дедуп ДУБЛЬ-NNNNNN + уникальный индекс), CSV within-file dedup
- [x] 40.2-10-PLAN.md — Блок «Шаблоны для инвентарных номеров» в Настройки/Организация + модалка + справка
- [x] 40.2-12-PLAN.md — NumberTemplateField.svelte (составное поле номера)

**Wave 6** *(blocked on Wave 5)*

- [x] 40.2-13-PLAN.md — Устройства/принтеры: подключение поля + цепочки попапов
- [x] 40.2-14-PLAN.md — Акты: подключение поля + цепочки попапов + финальная зачистка ActNumberField/acts_peek_next_number
- [x] 40.2-15-PLAN.md — Картриджи/фотобарабаны: подключение поля + переключение контекста по виду

**UI hint**: yes

### Phase 40.3: Долг аудита v1.4 (INSERTED)

**Goal**: Закрыть четыре находки повторного аудита вехи (2026-09-22), которые дёшевы сейчас и
лежат под фундаментом Фазы 41: пользователь может стереть необязательные поля устройства, тост
возврата показывает номер акта по шаблону без искажения, «Завести принтер» на экране «Принтеры»
идёт через шаблоны номеров и инвалидацию дерева мест, а проверка целостности FK после миграций
не пропускается на повторном старте.
**Requirements**: без новых REQ-ID; дефекты на стыках NUM-06, NUM-08, NUM-14, HST-03, PLC-06
**Depends on:** Phase 40.2
**Источник:** `.planning/v1.4-MILESTONE-AUDIT.md` (аудит 2026-09-22) — WR-03, N-1, N-2, N-4
**Success Criteria** (what must be TRUE):

  1. **WR-03:** Очистка серийного номера, модели, характеристик, комплектации и состояния в
     форме правки устройства реально сохраняет пустое значение (оба транспорта); незатронутое
     поле по-прежнему не меняется. Регрессионный тест различает «не передано» и «очищено».

  2. **N-1:** Тост после оформления возврата показывает `ActDto.number` как есть (например,
     «№2026/09-1в»), без склейки из `number_raw` и суффикса; в UI не остаётся кода, который
     считает номер акта начинающимся с цифр.

  3. **N-2:** «Завести принтер» на экране «Принтеры» даёт поле номера с шаблонами контекста
     «Новый принтер» (автоподстановка, «Вставка», цепочки попапов) и обновляет счётчики дерева
     мест; точка входа зарегистрирована в реестре INV-7; тип ответа `devices.create`
     соответствует серверному.

  4. **N-4:** Новое FK-нарушение после миграций блокирует старт не только в первый раз, но и
     на каждом следующем запуске, пока не устранено; нарушения, существовавшие до миграций, по-
     прежнему не блокируют старт.
**Plans:** 9/9 plans complete

**Waves (раунд 1, 40.3-01..05):** 1 — 40.3-01, 40.3-02, 40.3-03, 40.3-04 (независимы);
2 — 40.3-05 (depends_on 40.3-04).
**Waves (раунд закрытия пробелов, 40.3-06..09):** 1 — 40.3-06, 40.3-07, 40.3-08 (независимы,
без пересечения файлов); 2 — 40.3-09 (depends_on все три предыдущих, чисто для размещения
обязательного полного прогона пакета на границе волны — функциональной зависимости у
миграций от 06/07/08 нет).
**Координация внутри Rust-волн:** `cargo build`/`cargo test` планов 40.3-01/02/04 (раунд 1) и
40.3-06/07 (раунд закрытия пробелов) запускать строго последовательно (лок `target/`);
`trackly-app` — с `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1` и `--skip login_remember_persistent_cookie`.
Plans:
**Wave 1**

- [x] 40.3-01-PLAN.md — WR-03: double-Option для serial_no/model/specs/kit/state (domain+DTO+SQL) + JSON-boundary regression-тест
- [x] 40.3-02-PLAN.md — N-4: безусловная персистентная FK-проверка после миграций на каждом старте
- [x] 40.3-03-PLAN.md — N-1: тост возврата использует ActDto.number напрямую + постоянный структурный гейт
- [x] 40.3-04-PLAN.md — N-2 backend: printer IP/SNMP community в той же транзакции, что и создание устройства

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 40.3-05-PLAN.md — N-2 frontend: «Завести принтер» через DeviceFormModal (шаблоны номеров, INV-7), удаление PrinterCreateModal

Раунд закрытия пробелов (verification 2026-09-22: 3 BLOCKER + 2 WARNING из CR-01/CR-02/WR-07/WR-08/WR-01/WR-03):

- [x] 40.3-06-PLAN.md — BLOCKER-1 (CR-01): serde double_option на place_id (dto/device.rs) и email (dto/auth.rs) + JSON-boundary regression-тесты
- [x] 40.3-07-PLAN.md — BLOCKER-2 (CR-02): quantityDisabled учитывает IP для принтера + bulk_create_with_printer явно отклоняет printer-блок (defense-in-depth)
- [x] 40.3-08-PLAN.md — BLOCKER-3 (WR-07+WR-08): убрать клиентское предсказание номера в ReturnModal предпросмотре + расширить check-act-number-no-regex-split.mjs до семантического инварианта с self-test
- [x] 40.3-09-PLAN.md — WARNING-1+2 (WR-03+WR-01 в migrations.rs): guarded FK-baseline persist + rowid-стабильная сигнатура сравнения + полный прогон пакета

### Phase 40.4: Долг аудита v1.4, раунд 2 (INSERTED)

**Goal**: Закрыть четыре находки повторного аудита вехи (2026-09-23), которые дёшевы сейчас и
лежат прямо под фундаментом Фазы 41: откат акта больше не воскрешает очищенные поля устройства,
импорт CSV обновляет дерево мест и не устраивает шторм WS-событий, автоподстановка номера не
блокирует массовое создание, а номер акта в печатной форме приходит из одного источника, а не
собирается из сырого номера и суффикса.
**Requirements**: без новых REQ-ID; дефекты на стыках HST-03, PLC-06, NUM-06, NUM-08, NUM-14, NUM-16
**Depends on:** Phase 40.3
**Источник:** `.planning/v1.4-MILESTONE-AUDIT.md` (аудит 2026-09-23) — NEW-1, N-3, NEW-3, NEW-2
**Success Criteria** (what must be TRUE):

  1. **NEW-1:** Откат акта (мягкое удаление / undo) не воскрешает поля устройства, которые
     пользователь очистил после создания акта — `serial_number`, `model`, `condition`,
     `complectation`, `notes` восстанавливаются из snapshot по тому же правилу «не передано vs
     очищено», что и оба пути правки; `inventory_number` по-прежнему сознательно не
     восстанавливается (комментарий BE-WR-06 остаётся в силе). Это третий SQL-путь того же
     дефекта WR-03, который Фаза 40.3 закрыла на двух.

  2. **N-3 (инвалидация):** После импорта CSV счётчики узлов дерева мест обновляются без
     перехода на другой экран — точка входа импорта зарегистрирована в реестре INV-7 наравне с
     остальными мутирующими поверхностями.

  3. **N-3 (шторм событий):** Импорт CSV шлёт `NumberSpaceChanged` один раз на импорт, а не на
     строку — импорт сотен строк не рискует `Lagged` на широковещательном канале и не вытесняет
     соседние `NewRequest`/`PrinterAlert` у LAN-клиентов.

  4. **NEW-3:** Массовое создание устройств («Количество» > 1) достижимо, когда номер в поле
     подставлен автоматически по запомненному шаблону, а не введён руками; введённый вручную
     номер по-прежнему блокирует количество.

  5. **NEW-2:** Печатная форма акта получает готовый отображаемый номер из того же источника,
     что таймлайн и отчёт «Перемещения», и не собирает его из сырого номера и суффикса; шаблон,
     в котором автор написал только поле номера, не теряет суффикс возврата.
**Plans:** 5/5 plans complete
Plans:
**Wave 1**

- [x] 40.4-01-PLAN.md — NEW-1: undo акта восстанавливает очищенные поля устройства через presence-based CASE WHEN (сценарий 1)
- [x] 40.4-02-PLAN.md — N-3 backend: CsvImportReport.affected_place_ids + single-broadcast-after-loop в import_csv_commit
- [x] 40.4-04-PLAN.md — NEW-3: автоподставленный номер не блокирует «Количество», bulk-payload безусловно обнуляет inventory_no
- [x] 40.4-05-PLAN.md — NEW-2: act.number_display как единый источник номера в печати + версионирование шаблона + demo-контекст

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 40.4-03-PLAN.md — N-3 frontend: DevicesPage/DeviceImportCsvModal инвалидируют дерево мест после импорта, реестр INV-7 обновлён

### Phase 40.5: Нумерация возвратов: плотные суффиксы в1..вN (INSERTED)

**Goal**: Сделать суффикс возврата свойством самого возврата, а не функцией от изменчивого
состояния соседей: возврат всегда отображается как «в1», «в2» … «вN», а освободившийся после
удаления номер занимает следующий СОЗДАННЫЙ возврат. Сегодня номер уже выданного акта меняется
сам — единственный возврат показывается как «42в», а появление второго превращает его в «42в1»;
и удаление не-последнего возврата навсегда сдвигает нумерацию, потому что `MAX+1` не
переиспользует дырки. Чинить нужно ДО Фазы 41: акты АРМ пойдут через тот же контекст печати и
унаследуют дефект (риск F7 аудита 2026-09-23).
**Requirements**: без новых REQ-ID; отменяет продуктовое решение D-Numbering-01, затрагивает
стыки NUM-14 (формат «в»/«вN» сужается до «вN») и NUM-09 (что с чем конфликтует по отображаемому номеру)
**Depends on:** Phase 40.4
**Источник:** живая UAT Фазы 40.4 (пункт 3), находки пользователя 2026-09-26. Решения по обеим
развилкам зафиксированы в `.planning/todos/pending/2026-09-26-return-act-suffix-numbering-rework.md`
**Plans:** 14/14 plans complete

**Success Criteria** (what must be TRUE):

  1. **Суффикс стабилен.** Отображаемый номер возврата не меняется от создания или удаления
     соседнего возврата у того же родителя. Безцифровой формы «42в» больше не существует ни в
     одной поверхности: DTO, таймлайн, отчёт «Перемещения», превью шаблона номера, печатная
     форма, проверка занятости номера.

  2. **Плотная нумерация.** Выделение `sub_number` берёт НАИМЕНЬШИЙ свободный положительный
     номер среди неудалённых возвратов родителя, а не `MAX+1`. Сценарий пользователя:
     {в1, в2} → удалить в1 → создать возврат → снова {в1, в2}, а не {в2, в3}. При нескольких
     дырках занимается меньшая.

  3. **Существующие акты не перенумеровываются.** Удаление возврата не меняет номер ни одного
     уже созданного акта — дырка просто ждёт следующего созданного. Инвариант «в1..вN без
     разрывов» выполняется после заполнения, а не в каждый момент времени.

  4. **Номер перестаёт зависеть от состояния соседей структурно, а не по договорённости.**
     `sibling_return_count` больше не участвует в вычислении номера; его сквозная проводка
     удалена (либо, если что-то её требует по другой причине, причина названа в плане явно).
     Пока параметр жив, к нему можно вернуться — а это и был корень дефекта.

  5. **Риск F7 снят с причины, а не заплаткой.** `compute_suffix_from_display` больше не ищет
     литеральную «в» где угодно в строке (`display.find('в')`): суффикс выводится из
     `sub_number`. Шаблон номера с литеральной «в» в маске до цифр не ломает печать.

  6. **Уникальность по отображаемому номеру пересмотрена по смыслу.** Тесты
     `acts_numbering.rs` на конфликт handover'а с отображаемым номером возврата и на каскад
     при переименовании родителя отражают НОВОЕ правило, а не механически заменённые строки:
     handover «42в» больше не конфликтует с возвратом, «42в1» — конфликтует.

Plans:
**Wave 1**

- [x] 40.5-01-PLAN.md — формула отображения: format_act_number теряет sibling_return_count (D-01), compute_suffix_from_display снимает риск F7 (D-08)
- [x] 40.5-02-PLAN.md — next_sub_number_for_parent: наименьший свободный вместо MAX+1 (D-04/D-07)

**Wave 2** *(blocked on Wave 1)*

- [x] 40.5-03-PLAN.md — D-16: SELECT_ACTS/from_row без коррелированного подзапроса, позиционные индексы сверены по значениям; sibling_return_count удалён из ActRow/ports/act_service.rs (D-03)

**Wave 3** *(blocked on Wave 2)*

- [x] 40.5-04-PLAN.md — анти-вакуумный якорь: Wave 0 тесты плотной нумерации (Success Criterion 2/3) + инверсия готовых якорей в acts_returns.rs/acts_clone_handover.rs (D-14)
- [x] 40.5-05-PLAN.md — acts_numbering.rs: пересмотр уникальности по смыслу, не заменой строк (D-09/NUM-09)
- [x] 40.5-06-PLAN.md — place_movements_timeline.rs/report_movements.rs: канонический «Nв1» вместо безцифрового «Nв»

**Wave 4** *(blocked on Wave 3)*

- [x] 40.5-07-PLAN.md — grep-гейты (D-03/D-08), фронтендовый регресс (D-11), фоновый полный `cargo test --workspace` (D-13), передача 40.5-HUMAN-UAT.md пользователю (D-12)

**Раунд закрытия пробелов (gaps_found 4/6, решения D-17..D-20)**

*Wave 1*

- [x] 40.5-08-PLAN.md — GAP 1/WR-02: аллокатор перешагивает позицию, занятую чужим актом (D-17), + честный доккомментарий (WR-03)
- [x] 40.5-09-PLAN.md — GAP 2/WR-05: отчёт «Возвраты» отдаёт номер через format_act_number (D-19), колонка «Суб-номер» убрана
- [x] 40.5-10-PLAN.md — WR-04/WR-06: снятие тавтологичных тестов, реальный ассерт стабильности номера в таймлайне

*Wave 2 (blocked on 40.5-08)*

- [x] 40.5-11-PLAN.md — D-18: сообщение о коллизии называет блокирующий акт; WR-01: удаление возврата без мёртвой пост-проверки

*Wave 3 (blocked on Wave 1-2)*

- [x] 40.5-12-PLAN.md — граница раунда: гейты D-02/D-03/D-08/D-10/D-11, один полный прогон (D-13), дополнение 40.5-HUMAN-UAT.md для пользователя

**Раунд 2 закрытия пробелов (gaps_found 5/6, остаточный GAP 1 / ревью WR-01)**

*Wave 1*

- [x] 40.5-13-PLAN.md — остаточный GAP 1/WR-01: пост-проверка семьи снята из do_return (аллокатор — единственный гейт нового номера), тест «возврат в БД с существующей коллизией проходит», честный доккомментарий

*Wave 2 (blocked on 40.5-13)*

- [x] 40.5-14-PLAN.md — граница раунда: гейты, один полный прогон (D-13), пункт GAP 1.4 в 40.5-HUMAN-UAT.md и передача полного чеклиста из 7 пунктов пользователю

### Phase 41: Группы: модель и редактор

**Goal**: Ввести универсальную группу устройств как единицу размещения поверх дерева мест —
тип группы задаёт поведение и набор свойств, состав и место задаются в отдельном разделе
«Группы», а устройства в составе следуют за местом группы с записью в историю по каждому.

**Depends on**: Phase 39 (дерево мест — место группы), Phase 40 (история — запись по каждому
устройству при переносе группы)

**Входной брифинг**: `.planning/v1.4-GROUPS-MODEL-DECISIONS.md` — решения модели групп от
2026-10-04 (якорь вместо подмены строки, происхождение вместо членства при разборе, запрет
индивидуального перемещения, нумерация через `seq`), решения по умолчанию и вскрытый техдолг.
Распространяется на Фазы 41–41.6.

**Requirements**: GRP-01, GRP-02, GRP-03, GRP-04, GRP-05, GRP-06, GRP-07, GRP-08, GRP-09,
GRP-10, GRD-06

**Success Criteria** (what must be TRUE):

  1. В сайдбаре есть раздел «Группы» между «Устройства» и «Акты»: слева дерево (корни — типы
     групп, внутри — сами группы), справа панель просмотра и правки выбранного узла. Прежняя
     группировка одинаковых устройств в разделе «Устройства» переименована в «Свернуть
     одинаковые» — слово «группа» в приложении означает только новую сущность.

  2. Три встроенных типа (АРМ, Системный блок, Разбор) засеваются при запуске идемпотентно по
     машинному коду: их можно переименовать и дополнить свойствами, но нельзя удалить, а код и
     поведение (`container` / `substitute` / `teardown`) неизменны; повторный запуск после
     переименования не создаёт дубликат.

  3. Администратор создаёт свой тип группы с таблицей свойств: Текст, Число, IP, MAC,
     Пользователи (с отметкой основного), Ссылки на устройства; у каждого свойства есть порядок,
     признак обязательности и флаг «На карте». Свойство с заполненными значениями скрывается, а
     не удаляется; тип данных у заполненного свойства не меняется; IP и MAC нормализуются и
     проверяются на сервере.

  4. Группа создаётся с именем по умолчанию «{тип} #{seq}», где `seq` — целое из счётчика типа;
     имя редактируется, уникальность имён не требуется, и номер никогда не вычисляется обратным
     разбором имени.

  5. Устройство входит не более чем в одну группу, вывод из состава — явное действие; группа
     может содержать группу (Системный блок внутри АРМ) с защитой от циклов и правилами
     допустимых детей; место вложенной группы производное от корневой.

  6. Место группы протаскивается на все устройства состава, включая вложенные группы: у каждого
     устройства меняется `place_id`, по каждому появляется запись в журнале перемещений, и записи
     одного переноса связаны в один пакет. Отдельное перемещение устройства, входящего в группу,
     запрещено — поле места доступно только через группу.

  7. Карточка группы показывает состав, место, привязанных пользователей (один отмечен основным)
     и подключённые принтеры: USB — производные от `printers.usb_host_device_id` и не
     редактируемые вручную, сетевые — по явным ссылкам; при отображении списки дедуплицируются.

  8. Права соблюдаются на обоих транспортах: типы групп и их свойства — только администратор;
     группы, состав и значения свойств — администратор и менеджер (роль «Специалист» — это
     существующий `manager`, четвёртая роль не вводится); сотрудник доступа к разделу не имеет.

**Plans:** 32/34 plans executed

Plans:
**Wave 1**

- [x] 41-01-PLAN.md — схема: V045 (5 таблиц, триггер code/behavior) и V046 (batch_id, entity_label) — только аддитивно; тест run_up_to(44)
- [x] 41-02-PLAN.md — домен: GroupBehavior/PropertyDataType, нормализация ip/mac/число, Action::{ManageGroupTypes,MutateGroups,ReadGroups}, токены журнала Group
- [x] 41-03-PLAN.md — GRD-06: «Свернуть одинаковые» (5 фактических строк) и словарный гейт check-group-vocabulary с --selftest

**Wave 2** *(blocked on Wave 1)*

- [x] 41-04-PLAN.md — репозиторий типов и свойств: триггер, идемпотентный засев ON CONFLICT DO NOTHING, скрытие, reorder, нарушители обязательности
- [x] 41-05-PLAN.md — журнал перемещений: record_batch_movement_if_applicable (сигнатура старого метода не тронута), DTO таймлайна с group_id/group_label

**Wave 3** *(blocked on Wave 2)*

- [x] 41-06-PLAN.md — репозиторий групп: seq из колонки, PK членства, CTE цикла и состава, значения, USB-принтеры
- [x] 41-07-PLAN.md — GroupTypeService: типы, свойства, засев при старте, защита заполненных свойств, D-13..D-16

**Wave 4** *(blocked on Wave 3)*

- [x] 41-08-PLAN.md — GroupService часть 1: чтения, состав, поиск вложенных, создание с нумерацией, переименование, удаление
- [x] 41-09-PLAN.md — транспорты типов: 10 пар команда+роут, матрица прав 3 роли x 2 транспорта, неизменяемость по HTTP

**Wave 5**

- [x] 41-10-PLAN.md — перенос группы: единая транзакция, пакет 7 строк с batch_id, D-30 в audit_log, картриджи, ru_plural

**Wave 6**

- [x] 41-11-PLAN.md — состав и вложенность: add/remove/set_parent, производное место, release-примитив для актов

**Wave 7**

- [x] 41-12-PLAN.md — значения свойств (серверная нормализация), groups_user_options, карточка группы, дедупликация принтеров

**Wave 8** *(blocked on Wave 7)*

- [x] 41-13-PLAN.md — транспорты групп: 15 пар команда+роут, матрица прав, валидация по HTTP, полнота маршрутов
- [x] 41-14-PLAN.md — write-site'ы: запрет индивидуального перемещения в DeviceService::update, release при удалении устройства
- [x] 41-15-PLAN.md — массовый перенос места двигает группы целиком (D-23), счётчики групп, понятная блокировка удаления места
- [x] 41-17-PLAN.md — отчёт «Перемещения»: поля пакета, строка группы, причины, печать с полным составом

**Wave 9** *(blocked on Wave 8)*

- [x] 41-16-PLAN.md — акты выводят устройство из состава на 8 write-site'ах, счётный гейт исходников
- [x] 41-18-PLAN.md — клиентская основа: lib/api/groups.ts (25 команд), reorder.ts и гейт check-reorder

**Wave 10** *(blocked on Wave 9)*

- [x] 41-19-PLAN.md — дерево типов и групп, модалки типа/группы/удаления
- [x] 41-20-PLAN.md — панель типа: таблица свойств, перестановка на pointer-events, попап нарушителей
- [x] 41-21-PLAN.md — состав: Dropdown с секциями, таблица состава, мультивыбор, реестр INV-7
- [x] 41-22-PLAN.md — вкладка «Свойства»: чипсы пользователей, единый список принтеров, явное сохранение

**Wave 11** *(blocked on Wave 10)*

- [x] 41-23-PLAN.md — страница раздела, панель группы, перенос (две точки входа), маршрут, сайдбар, гейт раздела
- [x] 41-24-PLAN.md — форма устройства (D-19) и таймлайн «в составе группы» (D-28)

**Wave 12**

- [x] 41-25-PLAN.md — отчёт: свёртка пакета; модалка массового переноса с группами (D-23)

**Wave 13**

- [x] 41-26-PLAN.md — граница фазы: полный regress, UI-гейты в порядке CI, приватность, карта валидации, список живых проверок

**Догоняющая волна (gap closure, 2026-10-05)** — закрывает 2 проваленных пункта UAT и 4 WARNING верификации. Cargo-планы сериализованы по одному на волну (правило «один `cargo test` за раз»).

**Wave 1** *(41-28 идёт параллельно: cargo не запускает)*

- [x] 41-27-PLAN.md — GAP-1: портал `ActionMenu` единственным режимом, все 13 вызовов, гейт `check-action-menu-portal.mjs` с 9 мутантами
- [x] 41-28-PLAN.md — GAP-6 (W-F03): нейтральный хинт панели типа для всех ролей, синхронизация UI-SPEC §9.1 и deferred-items

**Wave 2** *(blocked on 41-27)*

- [x] 41-29-PLAN.md — GAP-3 (W-B01): скрытое свойство никогда не обязательное, проверка нарушителей при «Показать», тесты `protect_d_*`

**Wave 3** *(blocked on 41-29)*

- [x] 41-30-PLAN.md — GAP-5 (W-B03), сервер: истинный `total`, уведомление об усечении в печати и CSV

**Wave 4** *(blocked on 41-27, 41-28, 41-29, 41-30)*

- [x] 41-31-PLAN.md — GAP-2: копирайт ветвится по `filled_group_count` («Удалить безвозвратно» vs «Скрыть»), переписанный H20-1

**Wave 5** *(blocked on 41-29, 41-31)*

- [x] 41-32-PLAN.md — GAP-4 (W-B02): 8 мутаций типов и свойств атомарны со строкой `audit_log`, 10 тестов `atomic_*`

**Wave 6** *(blocked on 41-30, 41-31, 41-32)*

- [ ] 41-33-PLAN.md — GAP-5, экран: баннер усечения в `ReportsPage`, общая с Rust golden-фикстура

**Wave 7** *(blocked on 41-31, 41-32, 41-33)*

- [ ] 41-34-PLAN.md — граница волны: полный регресс в порядке CI, гейт приватности, перевод UAT в `fixed_pending_reverify`, живые проверки R1–R5 за пользователем

**UI hint**: yes

### Phase 41.1: Группы × Устройства (INSERTED)

**Goal**: Связать группы с разделом «Устройства»: быстрые действия из контекстного меню, якорное
замещение без подмены строк списка, разбор с сохранением происхождения деталей и два новых
статуса устройства.

**Depends on**: Phase 41 (типы, поведение, членство)

**Requirements**: GRD-01, GRD-02, GRD-03, GRD-04, GRD-05, GRP-10 (остаток)

**Success Criteria** (what must be TRUE):

  1. Контекстное меню устройства: Просмотр / Редактировать / Печать документа приёма /
     разделитель / Добавить в группу / быстрые действия типов / разделитель / Удалить. Пункты
     быстрых действий порождаются типами с включённым быстрым действием (по умолчанию
     «Сформировать группу», «Замещение группой», «На разбор»), а не захардкожены.

  2. Быстрое действие открывает попап создания группы с предзаполненными значениями: тип,
     признак «Сформировать на основе устройства» и само устройство. Групповые действия скрыты,
     когда бессмысленны: устройство уже в группе или является якорем, статус «Списано» или
     «На списание».

  3. «Замещение группой» прицепляет группу к устройству (`anchor_device_id`): в «Устройствах»
     остаётся одна строка с признаком группы и раскрытием состава; инвентарный и серийный номера
     остаются на устройстве, группа их только отображает — второго источника правды по номерам
     не возникает. Удаление якорного устройства запрещено, пока группа существует — это
     перенесённый остаток **GRP-10** (в фазе 41 закрыть было нельзя: колонки
     `groups.anchor_device_id` в миграции V045 нет, зафиксировано в SUMMARY плана 41-08 и
     подтверждено верификацией фазы 41).

  4. «На разбор» переводит устройство в новый статус «На списание» и создаёт группу-запись
     разбора; устройства, добавленные в неё, появляются в общем списке устройств и сохраняют
     неэксклюзивную ссылку «происхождение», которая не теряется при включении детали в другую
     группу.

  5. Статусы «На списание» и «В ремонте» доступны в фильтрах, формах, отчётах, «Местах» и на
     дашборде; подписи статусов читаются из одного справочника, прежний хардкод `id → подпись`
     удалён и закрыт гейтом.

  6. Правый клик по строке устройства или принтера открывает то же меню, что кнопка действий;
     двойной клик открывает попап просмотра.

**Plans**: TBD

**UI hint**: yes

### Phase 41.2: Группы × Акты и Места (INSERTED)

**Goal**: Группы участвуют в акте приёма-передачи целиком и отображаются в дереве мест вместо
входящих в них устройств.

**Depends on**: Phase 41 (состав и место группы), Phase 41.1 (якорные группы)

**Requirements**: GAM-01, GAM-02, GAM-03, GAM-04

**Success Criteria** (what must be TRUE):

  1. В акте можно выбрать группу: контейнерная (АРМ) разворачивается в список входящих устройств
     построчно, якорная (Системный блок) идёт одной строкой, её внутренний состав печатается в
     комплектации.

  2. Передача актом устройства из состава группы в другое место требует подтверждения
     пользователя; после подтверждения устройство выводится из состава.

  3. Перемещения членов группы, вызванные актом, несут `act_id` и снимаются откатом акта — после
     откат в журнале не остаётся строк отменённого акта.

  4. В «Местах» вместо входящих в группу устройств отображается сама группа; чекбокс
     «Разворачивать группы» (по умолчанию выключен, рядом с «Только здесь») прячет группы и
     показывает их устройства; счётчики содержимого и статистика поддерева считают физические
     устройства в обоих режимах.

**Plans**: TBD

**UI hint**: yes

### Phase 41.3: Вложения (INSERTED)

**Goal**: Дать приложению хранение файлов, которого в нём нет: каталог рядом с БД, метаданные в
БД, безопасная отдача в обоих режимах и просмотр фото и PDF.

**Depends on**: Nothing (инфраструктурная фаза, нужна Фазам 41.4 и 41.5)

**Requirements**: ATT-01, ATT-02, ATT-03

**Success Criteria** (what must be TRUE):

  1. Файл сохраняется в каталог `./attachments/` рядом с БД под именем от хэша содержимого; в БД
     остаётся строка с исходным именем, MIME, размером и хэшем. Лимит размера и whitelist MIME
     проверяются на сервере, а не только в UI.

  2. В серверном режиме файл отдаётся только под сессией и только пользователю, имеющему доступ
     к родительской сущности; в десктопном режиме — из того же каталога тем же кодом.

  3. Фото и PDF открываются в попапе просмотра, остальные типы — ссылкой на скачивание.

  4. Потерянный файл (строка в БД есть, файла на диске нет) деградирует сообщением в интерфейсе,
     а не ошибкой экрана.

  5. Каталог вложений исключён из git и покрыт privacy-гейтом; инструкция по переносу
     portable-сборки и бэкапу называет каталог вложений, а не только файл БД.

**Plans**: TBD

**UI hint**: yes

### Phase 41.4: Заявки: Ремонт (INSERTED)

**Goal**: Ввести третий вид заявки — ремонт устройства — с полным жизненным циклом перемещений:
на склад, в сервис и обратно на своё место.

**Depends on**: Phase 41 (устройства АРМ автора и привязанные принтеры), Phase 41.3 (вложение
результатов ремонта), Phase 41.1 (статус «В ремонте»)

**Requirements**: REP-01, REP-02, REP-03, REP-04, REP-05, REP-06, REP-07

**Success Criteria** (what must be TRUE):

  1. В создании заявки три раздела: «Замена картриджа» / «Ремонт» / «Свободная форма». В
     «Ремонте» — дропдаун устройств автора (устройства его АРМ и привязанные принтеры) плюс
     «Другое» и поле описания. Категория «Ремонт техники» в свободной форме больше не
     предлагается — скрыта, а не удалена, старые заявки её по-прежнему показывают.

  2. Принятие в работу заявки с указанным устройством открывает попап «Перемещение устройства» с
     быстрым выбором складского места и выбором любого места из дерева; кнопки «Отмена» слева,
     «Без перемещения» и «Переместить» справа; последний выбор запоминается и предлагается
     по умолчанию. После перемещения устройство получает статус «На ремонте».

  3. У заявки в работе в шапке доступны «Забрать на склад», «Отправить в ремонт» и «Завершить»;
     «Забрать на склад» открывает попап перемещения для случая, когда при принятии выбрали
     «Без перемещения».

  4. «Отправить в ремонт» открывает попап с датой, кем выдано, кому выдано и местом (последнее
     место ремонта предлагается по умолчанию), переводит устройство в статус «В ремонте», а
     заявку — в статус «В ремонте».

  5. После отправки кнопка сменяется на «Забрать из ремонта»; забор работает как забор картриджа
     с заправки и дополнительно позволяет приложить файл с результатами работ.

  6. «Завершить» предлагает вернуть устройство на место, которое было до ремонта (сохранённое на
     заявке при первом перемещении), с комментарием и вложением.

  7. История заявки показывает, когда устройство ушло в ремонт и когда вернулось. Устройство в
     активной ремонтной фазе может иметь место, отличное от места его группы — это единственное
     санкционированное исключение из инварианта Фазы 41, и членство в группе при ремонте не
     теряется.

**Plans**: TBD

**UI hint**: yes

### Phase 41.5: Переписка в заявке (INSERTED)

**Goal**: Заменить односторонний комментарий специалиста двусторонней перепиской с вложениями,
видимой в истории заявки.

**Depends on**: Phase 41.3 (вложения), Phase 41.4 (ремонтные заявки как основной сценарий
переписки)

**Requirements**: MSG-01, MSG-02, MSG-03

**Success Criteria** (what must be TRUE):

  1. У заявки в работе и инициатор, и специалист видят поле отправки сообщения с возможностью
     приложить файл; прежний блок комментария, доступный только администратору, убран.

  2. Сообщения отображаются в «Истории» вместе с событиями жизненного цикла в одном
     хронологическом порядке: сообщения хранятся в своей таблице, события остаются в `audit_log`,
     слияние выполняется на чтении.

  3. Переписка видна только участникам заявки: её автору и ролям с доступом к заявкам; сотрудник
     не видит переписку по чужой заявке ни на Tauri-, ни на HTTP-транспорте.

  4. Новое сообщение доезжает до открытой карточки заявки без перезагрузки (WebSocket).

**Plans**: TBD

**UI hint**: yes

### Phase 41.6: Раздел «Устройства» для сотрудника (INSERTED)

**Goal**: Дать роли «Сотрудник» второй раздел — его устройства — и позволить создавать заявки
прямо из списка, не раскрывая ни дерева мест, ни чужих устройств.

**Depends on**: Phase 41 (привязка пользователей к группам), Phase 41.4 (заявка на ремонт из
строки)

**Requirements**: EMP-01, EMP-02, EMP-03

**Success Criteria** (what must be TRUE):

  1. У роли «Сотрудник» в веб-интерфейсе два раздела — «Заявки» и «Устройства»; остальные
     маршруты по-прежнему отдают «Доступ запрещён».

  2. В «Устройствах» сотрудник видит одним плоским списком все устройства всех своих АРМ и
     привязанные принтеры с полным описанием: наименование, тип, модель, серийный и инвентарный
     номера, место и статус.

  3. Если сотрудник не привязан ни к одной группе, список показывает пустое состояние с
     подсказкой обратиться к администратору.

  4. Из строки сотрудник создаёт заявку на ремонт (для любого устройства) или на замену картриджа
     (если в строке принтер) с уже подставленным устройством.

  5. Сервер отдаёт сотруднику только его устройства: запрос чужого устройства отклоняется на
     обоих транспортах.

**Plans**: TBD

**UI hint**: yes

### Phase 42: Умный подбор принтера в заявке

**Goal**: Заявка на замену картриджа сама предлагает наиболее вероятный принтер, используя место
автора и его АРМ — без того, чтобы роль «Сотрудник» когда-либо видела карту или дерево мест
напрямую.

**Depends on**: Phase 41 (группы — источник «USB-принтер моего АРМ»), Phase 39 (дерево мест —
иерархия помещение → этаж → здание для ранжирования)

**Requirements**: REQ-07, REQ-08

**Success Criteria** (what must be TRUE):

  1. В форме заявки на замену картриджа принтеры отсортированы по близости к автору: USB-принтер
     его АРМ → принтеры его помещения → его этажа → его здания → остальные.

  2. Если у автора есть единственный очевидный кандидат (принтер его АРМ или единственный принтер
     его помещения), поле принтера в форме заявки подставляется автоматически.

**Plans**: TBD

### Phase 43: Карта — просмотр

**Goal**: Дать администратору и менеджеру визуальный обзор устройств и АРМ на плане этажа/зоны —
план читает ту же модель мест, что и остальное приложение, и является необязательным слоем поверх
неё, а не источником истины.

**Depends on**: Phase 39 (дерево мест — семантика помещений), Phase 41 (группы — маркеры на плане)

**Requirements**: MAP-01, MAP-02, MAP-03, MAP-04, MAP-05

**Success Criteria** (what must be TRUE):

  1. Администратор или менеджер выбирает план этажа/зоны и видит схему помещений с размещёнными
     на ней устройствами и АРМ; место, у которого ещё нет плана, продолжает нормально работать во
     всех остальных разделах приложения.

  2. К плану можно приложить готовую подложку в формате JPG (хранится в БД, а не файлом рядом с
     БД) и работать поверх неё.

  3. Клик по маркеру открывает карточку устройства или АРМ; клик по помещению — список
     размещённого в нём.

  4. План печатается на лист A4 через существующий механизм печати на обоих транспортах (десктоп
     и LAN-браузер).

  5. Роль «Сотрудник» не видит раздел «Карта» в интерфейсе и не может получить её данные ни через
     Tauri-команды, ни через HTTP-эндпоинты — попытка обращения отклоняется на сервере
     (`Action::ReadMap` только Admin/Manager).

**Plans**: TBD

**UI hint**: yes

### Phase 44: Карта — редактор планов

**Goal**: Дать администратору инструмент для рисования планов помещений и расстановки устройств
поверх JPG-подложки на SVG (не canvas), без сторонних JS-библиотек.

**Depends on**: Phase 43 (карта — просмотр; редактор пишет в ту же модель плана и мест)

**Requirements**: EDT-01, EDT-02, EDT-03, EDT-04, EDT-05, EDT-06

**Success Criteria** (what must be TRUE):

  1. Пользователь рисует контур помещений ортогональными стенами по сетке с привязкой к её узлам.
  2. Пользователь размещает двери и окна как вырезы на стене, с общепринятыми условными
     обозначениями.

  3. Нарисованное помещение связывается с местом из дерева — после связывания клик по нему
     показывает содержимое этого места (переиспользует поведение Фазы 43).

  4. Устройства и АРМ размещаются на плане перетаскиванием из палитры с поворотом значка;
     перемещение маркера обновляет место устройства/АРМ и создаёт запись в истории перемещений
     (Фаза 40) с причиной «перетаскиванием на карте».

  5. Действия в редакторе отменяются и повторяются (undo / redo).
  6. Слои содержимого (подложка / стены / помещения / устройства) переключаются независимо друг
     от друга.

**Plans**: TBD

**UI hint**: yes

### Phase 45: Живые статусы на карте

**Goal**: Показать оперативное состояние принтеров прямо на карте, без похода в раздел
«Принтеры» и без перезагрузки страницы.

**Depends on**: Phase 43 (карта — просмотр; источник маркеров принтеров)

**Requirements**: LIV-01, LIV-02

**Success Criteria** (what must be TRUE):

  1. Маркер принтера на карте окрашен по состоянию: уровень тонера, недоступность по SNMP,
     наличие открытой заявки.

  2. При изменении состояния принтера цвет маркера на уже открытой карте обновляется через
     WebSocket без перезагрузки страницы.

**Plans**: TBD

**UI hint**: yes

---

Детализация фаз 31–33 — `milestones/v1.3-ROADMAP.md`.
Детализация фаз 34–38 — `milestones/v1.3.3-ROADMAP.md`.

## Backlog

- **999.1 — role-based route gating** — UX-полировка: гейт маршрута ≠ гейт меню для admin-vs-manager
  (реальная граница безопасности — backend 403). Отложено из v1.2.
  Каталог: `phases/999.1-role-based-route-gating/`.

- **DOC-12 — пользовательский редактор печатных форм в UI** — отложено из v1.3.3 (Future
  Requirements); сейчас шаблоны правятся файлами в `templates/`.

- **DOC-13 — единая шапка распространяется на будущие печатные формы** — отложено из v1.3.3, если
  появятся новые формы за пределами трёх текущих.

- **PRIV-03 — очистка утёкших данных из истории git** — отложено из v1.3.3 (решение пользователя
  2026-08-08: чистим только HEAD, история не переписывается; PRIV-03 остаётся опцией на будущее).

- **QA-05 — Nyquist-покрытие Фазы 36** — единственная фаза v1.3.3 без подтверждённого
  Nyquist-покрытия (`/gsd-validate-phase 36`). В объём v1.4 не входит.

- **INT-02 — общий источник `RepeatTableHeadHandler`** — сейчас дублирован между десктопным и
  LAN-путём печати без гейта синхронности. В объём v1.4 не входит.

- **MAPX (v2) — развитие карты** — масштаб в метрах (MAPX-01), режим «обход кабинетов» для
  инвентаризации (MAPX-02), генплан территории (MAPX-03), автоподстановка размещения по
  IP-подсети (MAPX-04). Отложено осознанным решением пользователя 2026-08-22 — не предлагать
  повторно в объём v1.4.

- **999.2 — предупреждение о маске номера, пересекающейся с пространством суффиксов возвратов** —
  маску вида `[X]в1` и акт с таким номером сейчас можно создать молча; коллизия всплывает только
  при первой попытке возврата. Найдено на живой UAT фазы 40.5; это зафиксированное решение D-09,
  а не регрессия. Точки: `ui/src/features/settings/NumberTemplateModal.svelte` (превью
  «Следующий номер»; списка занятых номеров на экране нет),
  `crates/trackly-app/src/services/number_template_service.rs::detect_warnings` (сейчас только
  NUM-12 — смешение кириллицы/латиницы и омоглифы). В объём 40.5 не входило: scope fence #4
  запрещает трогать формат шаблонов NUM-14/NUM-16 сверх стыка.
  Каталог: `phases/999.2-number-mask-return-suffix-collision-warning/`.
  ВАЖНО: это ТОЛЬКО превентивное предупреждение в UI. Само поведение аллокатора (посторонний
  акт «Nв1» делает родителя невозвратимым навсегда — WR-02) закрывается gap-closure планом
  фазы 40.5, не здесь.
