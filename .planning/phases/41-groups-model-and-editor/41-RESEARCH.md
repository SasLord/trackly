# Phase 41: Группы: модель и редактор — Research

**Researched:** 2026-10-04
**Domain:** Rust (rusqlite single-writer, refinery, axum + Tauri dual transport) + Svelte 5 runes (WKWebView + LAN-браузер); новая сущность «группа устройств» поверх дерева мест и журнала перемещений
**Confidence:** HIGH по бэкенду и кодовой базе (всё прочитано в репозитории, DDL проверен в `sqlite3`, поведение `std::net::IpAddr` проверено компиляцией); MEDIUM по UI-механике (pointer-перетаскивание списка строк — прецедент есть, но для вертикальной сортировки код новый)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Состав группы**

- **D-01:** Устройства добавляются **двумя путями**: строка-поиск внизу таблицы состава
  (`Dropdown` + FTS5-поиск устройств, логика переиспользуется из
  `ui/src/features/acts/ActFormItemsTable.svelte`, включая drill-in по свёрткам одинаковых,
  клавиатуру и ARIA, которые уже живут внутри `Dropdown`) **и** кнопка «Добавить несколько…»,
  открывающая модальный мультивыбор с фильтрами. Оба пути идут в один серверный обработчик.
- **D-02:** Вывод из состава — через `ActionMenu` (⋯) в строке, один стиль с разделами
  «Устройства» и «Принтеров». Отдельного подтверждения нет.
- **D-03:** Вложенные группы в таблице состава — **раскрываемая строка** со счётчиком,
  разворачивающаяся в свои устройства. Тот же рисунок, что у `DeviceGroupRow.svelte` в
  «Устройствах» и у drill-in в `Dropdown` акта.
- **D-04:** Вложение группы в группу делается **тем же полем поиска**: в `Dropdown` строки
  добавления находятся и устройства, и группы, разделённые заголовками внутри списка.
  Перетаскивания нет (HTML5 DnD не работает в WKWebView — фаза 39, живой UAT).
- **D-05:** Столбцы таблицы состава копируют набор `ui/src/features/places/PlaceContents.svelte`
  один в один — панель группы выглядит как знакомое «Содержимое места».
- **D-06:** Пустой состав — `Placeholder.svelte` с подсказкой «Состав пуст. Добавьте устройства
  поиском ниже», один стиль с остальными пустыми состояниями проекта.
- **D-07:** Меню (⋯) строки вложенной группы: «Открыть группу» (переход-фокус на её узел дерева,
  прецедент перехода-фокуса фазы 39) + «Вывести из состава» + «Переименовать».
- **D-08:** Инвалидация счётчиков: новые серверные мутации (добавление в состав, вывод, перенос
  группы, массовый перенос, вынос актом) вносятся в **реестровый гейт INV-7** фазы 40.1 —
  экран без инвалидации проваливает проверку сам. Не точечной правкой по экранам.

**Свойства: редактор типа и форма группы**

- **D-09:** Свойства типа администратор ведёт **таблицей прямо в панели типа** со строкой
  добавления. Модалок-над-модалками нет; `behavior` в панели типа — только чтение.
- **D-10:** Порядок свойств (`sort_order`) меняется **перетаскиванием на pointer-events**
  (`pointerdown`/`pointermove`), НЕ через HTML5 drag-and-drop API — он уже подводил в WKWebView
  (фаза 39, живой UAT). Это новый для проекта механизм: нужен живой чек на обоих транспортах
  (Tauri WKWebView + LAN-браузер). Исследователю: проверить, стоит ли вынести drag-примитив в
  переиспользуемый компонент — редактор карты фазы 44 потребует того же.
- **D-11:** Поле типа `users` в форме группы — **список-чипсы** добавленных людей с
  переключаемой звёздочкой «основной» и `Dropdown` добавления внизу. Выбор только из таблицы
  `users` (локальные + ранее зарегистрированные AD-пользователи), поиска по AD-каталогу нет.
- **D-12:** Подключённые принтеры — **один список** «Подключённые принтеры»: у выведенных из
  `printers.usb_host_device_id` строк бейдж «USB» и нет кнопки удаления; явные ссылки
  `device_refs` удаляются. Дедупликация невидима для пользователя.
- **D-13:** Скрытое свойство (`archived_at_utc`) по умолчанию **исчезает** из таблицы; чекбокс
  «Показать скрытые» показывает их с возможностью вернуть. Значения остаются в БД.
- **D-14:** Отказ «включить обязательность нельзя: есть группы с пустым значением» — **попап со
  списком групп-нарушителей и переходом к ним**, по рисунку `NumberTakenPopup.svelte` фаз
  40.2/40.5.
- **D-15:** Флаг `show_on_map` — **видимый столбец-чекбокс** в таблице свойств с подсказкой
  «будет показано на карте». Админ расставляет флаги до фазы 43; потребитель флага — фаза 43.
- **D-16:** У свойства с заполненными значениями `Dropdown` типа данных **disabled** с текстом
  «заполнено в N группах». Серверная проверка на обоих транспортах остаётся источником истины —
  UI-блокировка её не заменяет.

**Место группы и запрет индивидуального перемещения**

- **D-17:** Место группы меняется **отдельным действием «Перенести»** с подтверждением
  «переедет N устройств» (рисунок `PlaceMoveModal.svelte`), а не полем в форме группы: перенос —
  осознанное действие, а не следствие правки формы.
- **D-18:** Действие «Перенести» доступно **из двух точек** — кнопкой в панели группы и
  действием в меню узла дерева. Обработчик один.
- **D-19:** В форме устройства, входящего в группу, `PlacePicker` **disabled** + текст «Место
  задаётся группой «АРМ #3»» с переходом-фокусом в раздел «Группы» (прецедент перехода-фокуса
  фазы 39).
- **D-20:** Поле места вложенной группы — **место + имя корневой группы** в скобках с переходом
  на неё, только чтение.
- **D-21:** Группа **без места** (`place_id IS NULL`) допустима. Пока места нет, протаскивания
  нет и место устройств состава редактируется как обычно; запрет включается, когда группе задали
  место («запрет спит»). Это позволяет «собрать сейчас, разместить потом».
- **D-22:** Акт приёма-передачи для устройства в группе **проходит и выводит устройство из
  состава** — состав не врёт, акты работают как сейчас. Подтверждение выноса добавит фаза 41.2.
- **D-23:** «Перенести всё содержимое в…» (`places_move_subtree_contents`): если в содержимом
  есть устройство группы, **переносится и сама группа** — её `place_id` меняется на целевое, и
  остальной состав едет за ней. Запрет на индивидуальное перемещение не нарушается, массовый
  перенос склада не блокируется.
- **D-24:** После удачного переноса — **Toast «Перенесено: группа и 6 устройств»**, чтобы было
  видно, что протаскивание сработало, а не только группа переехала.

**Свёрнутая строка журнала**

- **D-25:** Пакет (`batch_id`) в отчёте «Перемещения» раскрывается **шевроном inline** — строка
  пакета распахивает строки устройств прямо в таблице. Тот же рисунок, что у D-03 и
  `DeviceGroupRow`.
- **D-26:** В свёрнутой строке — **имя группы + счётчик**: «АРМ #3 (6 устройств)» в столбце
  предмета; откуда/куда/дата/основание как у обычной строки.
- **D-27:** Печатная версия отчёта — **всегда полный состав**, все 7 строк независимо от
  состояния раскрытия на экране: для архива важен каждый инвентарный. Печать и экран здесь
  расходятся сознательно.
- **D-28:** В `MovementTimeline` (попап устройства) своя строка устройства читается как
  **«в составе группы «АРМ #3»»** с переходом на группу — ровно как уже сделано для
  «актом №…».
- **D-29:** У группы **есть своя история перемещений**: секция в панели группы на том же
  `MovementTimeline`, показывает только строки `entity_type = 'group'`.
- **D-30:** Событие «устройство без места попало в группу с местом» в фазе 41 **только пишется в
  `audit_log`** и в UI не показывается. Показ потребовал бы слияния двух источников на чтении —
  это задача MSG-02 (фаза 41.5). Требование 10 SPEC выполняется записью.

### Claude's Discretion

- Схема EAV (одна таблица значений с текстовой колонкой vs типизированные колонки vs JSON),
  форма хранения связей `users` / `device_refs`, индексы — прецедента в проекте нет
  (`org_settings` V026 — фиксированные колонки, не key-value). Решает исследователь/планировщик
  при двух ограничениях: только аддитивные миграции и нормализация `ip`/`mac` на сервере.
- Контракты портов и DTO, разбивка на планы, порядок волн.
- Имена таблиц, миграций и Tauri-команд — по сложившимся соглашениям (`places_*`,
  `place_movements_*`).
- Точная формулировка русских надписей, кроме явно зафиксированных в D-06, D-19, D-24, D-26, D-28.

### Deferred Ideas (OUT OF SCOPE)

- **Показ события «размещено в составе группы» в истории устройства** — требует слияния
  `audit_log` и `place_movements` на чтении; отложено в MSG-02 (фаза 41.5). В фазе 41 событие
  только пишется в `audit_log` (D-30).
- **Переиспользуемый drag-примитив на pointer-events** — редактор карты фазы 44 потребует того
  же механизма, что D-10. В фазе 41 достаточно решения внутри таблицы свойств; вынесение в общий
  компонент — кандидат на фазу 44.
- **Токен `source = 'workstation'`**, зарезервированный V040 и не используемый никем: фаза
  вводит `source = 'group'`; уборка мёртвого токена — отдельная мелочь.
- Вне фазы (из SPEC): якорное замещение/разбор/статусы/контекстные меню (41.1); группа в акте,
  `act_id` в перемещениях членов, группы в «Местах» (41.2); вложения, ремонт, переписка,
  раздел «Устройства» сотрудника (41.3–41.6); ранжирование принтеров (42); свойства на карте (43);
  типы свойств enum/списки/файлы (GRPX-02); группы в отчётах/CSV (GRPX-03); четвёртая роль.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| GRP-01 | Тип группы: неизменяемые `code` и `behavior`, 3 встроенных типа, идемпотентный засев по коду | §«Засев»: `INSERT … ON CONFLICT(code) DO NOTHING` из стартового сервиса; §«Схема»: уникальный индекс `code` + триггер неизменяемости (проверен в sqlite3) |
| GRP-02 | CRUD свойств типа, 6 типов данных, `is_required`, `show_on_map`, порядок | §«EAV-схема» (`group_type_properties`), §«Транспорт» (`group_type_properties_*`, `reorder` одним вызовом), §«UI: перетаскивание» |
| GRP-03 | Скрытие заполненного свойства, запрет смены `data_type`, нормализация IP/MAC на сервере | §«EAV-схема» (`archived_at_utc`, EXISTS-проверки), §«Валидация значений» (std `IpAddr` проверен, MAC-нормализатор) |
| GRP-04 | Раздел «Группы», CRUD группы, имя `{тип} #{seq}` из колонки | §«Схема» (`groups.seq`, `UNIQUE(type_id, seq)`), §«UI-карта», §«Транспорт» |
| GRP-05 | Членство не более чем в одной группе, вложенность без циклов, teardown не содержит групп | §«Схема» (`group_devices` PK по `device_id`), §«Образец защиты от циклов» (`move_node`) |
| GRP-06 | Протаскивание места одной транзакцией + журнал пакетом `batch_id` | §«Единая транзакция переноса», §«Журнал и отчёт», V046 |
| GRP-07 | Запрет индивидуального перемещения члена группы | §«Инвентаризация write-site’ов» (от серверных мутаций), Pitfall 1 |
| GRP-08 | Карточка группы: состав, место, люди, принтеры (USB производные + ссылки, дедуп) | §«Карточка группы» (SQL), §«EAV-схема» (`value_ref`) |
| GRP-09 | Права на обоих транспортах: типы — admin; группы — admin+manager; employee — нет | §«Права и матрица», `Action::{ManageGroupTypes, MutateGroups, ReadGroups}` |
| GRP-10 | Удаление группы освобождает устройства, место не меняется; подтверждение с количеством | §«Схема» (`ON DELETE CASCADE` для членства, `SET NULL` для вложенных), Pitfall про `subtree_stats` |
| GRD-06 | Переименование прежней свёртки в «Свернуть одинаковые» | §«Переименование и гейт словаря» (полный перечень строк с номерами строк) |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **Приватность (жёсткое):** репозиторий публичный. Ни названий организации/реквизитов/ФИО в коде, шаблонах, тестах, фикстурах, `.planning/`. В тестах только вымышленные («Иванов И.И.», «Петров П.П.»). Гейт: `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` (pre-commit + CI). Проверять ДО коммита. [VERIFIED: CLAUDE.md, scripts/check-privacy.mjs]
- **Стек зафиксирован:** Rust + Tauri 2 + Svelte 5 (runes) + SCSS + SQLite (rusqlite + refinery); axum для LAN-режима. Без SvelteKit. [VERIFIED: CLAUDE.md]
- **Portable:** ничего в `%APPDATA%`; БД и конфиг рядом с exe. Фазы это не касается (нет файлов), но запрещает `dirs::*` (clippy `disallowed-methods`). [VERIFIED: clippy.toml]
- **Concurrent-доступ:** WAL, единый writer; обработчики Tauri и axum — тонкие адаптеры над одним сервисом. [VERIFIED: CLAUDE.md, writer_worker.rs]
- **UI только русский.** Выпадающие списки — `Dropdown`, не нативный `<select>`, не `Select` (повторяющаяся коррекция пользователя, фазы 27/28). [VERIFIED: MEMORY.md, SPEC Constraints]
- **GSD-воркфлоу:** правки репозитория только через GSD-команды (`/gsd-execute-phase` и т.д.).
- **Фактические версии отличаются от CLAUDE.md:** `rusqlite 0.38` (не 0.39), `refinery 0.9` (не 0.8), `tower-sessions 0.15` (не 0.13), `rust-version = 1.92`, `specta =2.0.0-rc.22`, `tauri-specta =2.0.0-rc.21`. Планировать по `Cargo.toml`, не по CLAUDE.md. [VERIFIED: /Cargo.toml]

## Summary

Фаза добавляет в проект пять новых таблиц (типы, свойства типов, группы, членство, значения свойств), три колонки-дополнения к существующим (`place_movements.batch_id`, `place_movements.entity_label` — `ALTER TABLE ADD COLUMN`), одну сервисную связку `GroupService` + `SqliteGroupRepository`, раздел «Группы» в UI и ряд правок в чужих местах (запрет индивидуального перемещения, акты, массовый перенос, отчёт «Перемещения», `MovementTimeline`, переименование надписей). Нового внешнего кода (crate/npm) не нужно: нормализация IP — `std::net::IpAddr`, идентификатор пакета — `uuid` (уже в workspace), сериализация — `serde_with::double_option` (уже используется в `DevicePatch`).

Самое рискованное — не схема, а **полнота перечня write-site’ов** `devices.place_id`: их 8 вызовов в `act_service.rs` + `DeviceService::update` + `PlaceService::move_subtree_contents` + `DeviceService::delete_soft` (членство) — и уже зафиксированный проектный урок (фаза 40.1: три раунда верификации по одному экрану). Их надо закрыть сплошной таблицей «серверная мутация → поведение», с поведенческим тестом на каждую строку и счётным гейтом на исходник. Второе по риску — единая транзакция переноса: существующий идиом `PlaceService` («repo-вызов в автокоммите, потом отдельная транзакция для аудита») для групп **непригоден**, нужен образец `move_subtree_contents` (одна `conn.transaction()` на всё). Третье — UI: `Dropdown` не умеет секции-заголовки (нужна аддитивная правка), `Placeholder.svelte` — не компонент пустого состояния, дерево `PlaceTree.svelte` не обобщено (1219 строк, завязано на `PlaceDto`).

**Primary recommendation:** одна таблица значений `group_property_values` (скаляры в `value_text`, связи — строки с `value_ref`), членство — junction `group_devices` с PK по `device_id`, место хранится денормализованно в `groups.place_id` у каждой группы (включая вложенные) и протаскивается одной транзакцией через `*_in_tx`-метод репозитория; засев — `INSERT … ON CONFLICT(code) DO NOTHING` из стартового сервиса; каждый write-site `devices.place_id` проходит через явный helper (`release_device_in_tx` для актов, guard для `DeviceService::update`, группа-в-пакете для `move_subtree_contents`) под поведенческим тестом и счётным гейтом.

## Расхождения с CONTEXT/SPEC, найденные при исследовании (прочитать планировщику ПЕРВЫМ)

| # | Утверждение в CONTEXT/SPEC | Что на самом деле | Следствие для плана |
|---|---------------------------|-------------------|---------------------|
| 1 | D-10: pointer-drag — «новый для проекта механизм» | **Прецедент уже есть и прошёл живой UAT на Tauri + LAN**: `PlaceTree.svelte:717-846` (pointerdown/move/up/cancel, порог 6 px, `setPointerCapture`, `elementFromPoint`, призрак с `pointer-events:none`); 39-UAT GAP-2/GAP-11 закрыты пользователем («ок»). Новое — только вычисление индекса вставки для вертикального списка | Не изобретать; копировать структуру обработчиков. Живой чек всё равно нужен (другой контейнер — `<table>`), но риск ниже, чем написано в D-10 |
| 2 | D-06: «`Placeholder.svelte` с подсказкой «Состав пуст…»» | `Placeholder.svelte` — заглушка раздела («Раздел в разработке», пропсы `section`/`phase`), не пустое состояние. Пустое состояние проекта — `Table` (`empty`, `emptyTitle`, `emptyBody`), так сделано в `PlaceContents.svelte:329-340` | Использовать `<Table empty emptyTitle=… emptyBody=…>`; формулировка из D-06 остаётся |
| 3 | D-04: `Dropdown` покажет устройства и группы «разделёнными заголовками внутри списка» | `Dropdown.svelte` — дженерик `TGroup`/`TMember` с drill-in, **секций-заголовков нет** | Нужна аддитивная опциональная правка `Dropdown` (напр. `getGroupSection?: (g) => string \| undefined` — заголовок рисуется при смене секции), с витриной и без изменения поведения существующих потребителей. Альтернатива — два `Dropdown` рядом (хуже UX) |
| 4 | SPEC req.14: прежняя подпись «Группировать одинаковые» | Реальная строка — **«Группировать похожие»** (`DeviceFilters.svelte:102`) | Грепать по фактическим строкам, см. §«Переименование» |
| 5 | CONTEXT: дерево групп «ложится на каркас `PlaceTree`» | `PlaceTree.svelte` (1219 строк) и `PlaceTreeNode.svelte` типизированы на `PlaceDto`, встроены счётчики/архив/drag-to-move/`places_search` | Переиспользовать **как есть** только `PlacesMasterDetail.svelte` (generic по snippet’ам) и контракт ARIA/клавиатуры; `GroupTree`/`GroupTreeNode` — новые сиблинги |
| 6 | SPEC acc.: отказ с `AppError::Validation` | `Validation` → HTTP **400** (`error_axum.rs:38`), `Conflict` → 409; в проекте нет 422 | В тестах HTTP ждать 400 для `Validation`, 409 для `Conflict`, 403 для `Forbidden` |
| 7 | D-11: выбор людей из `users` | `users_list` — `Action::ManageUsers` (Admin only, `auth.rs:1679`), manager его вызвать не может; `list_users` ищет `LIKE` (ASCII-only регистр для кириллицы) | Нужна отдельная команда `groups_user_options(query)` под `ReadGroups`; фильтр по подстроке — в Rust (`to_lowercase().contains`), как `PlaceService::search`; возвращать только `id`, `full_name`, `login` |
| 8 | SPEC req.11 «смена места отклоняется» | `DeviceFormBody.svelte:615/684` шлёт `place_id` в КАЖДОМ сохранении устройства (даже без изменения места) | Серверный guard обязан сравнивать новое значение с текущим и отклонять только реальную смену; иначе переименование члена группы перестанет сохраняться |
| 9 | CLAUDE.md: rusqlite 0.39 / refinery 0.8 | В `Cargo.toml`: rusqlite 0.38, refinery 0.9 | Использовать фактические API (транзакции `conn.transaction()`, `&Transaction` → `Deref<Target=Connection>`) |
| 10 | SPEC/CONTEXT о таблице `place_movements` | У `from_place_id`/`to_place_id` `NOT NULL`; `MovementEntityKind` знает только `Device`/`Cartridge`; `MovementSource` — `Manual/Act/Map/Workstation` | Нужны варианты `Group` у обоих enum’ов (core), иначе чтение деградирует в «причина не определена» и сырой `group` в отчёте |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Хранение типов/свойств/групп/значений | Database / Storage | API / Backend | SQLite — источник истины; уникальность `code`, `(type_id, seq)`, `device_id` — БД-ограничения |
| Неизменяемость `code`/`behavior` | Database (триггер) | API / Backend | Защита на уровне БД + проверка в сервисе с понятным сообщением |
| Идемпотентный засев встроенных типов | API / Backend (стартовый сервис) | Database (`UNIQUE(code)`) | Сервис вызывается при `AppCtx::build`; БД гарантирует отсутствие дублей |
| Нормализация/валидация значений `ip`/`mac`/`number`/refs | API / Backend (домен `trackly-core`) | Browser (подсказки) | Источник истины — сервер на обоих транспортах; UI-проверка только удобство |
| Протаскивание места на состав + журнал | API / Backend (`*_in_tx` в одном writer-замыкании) | Database | Атомарность «всё или ничего» обеспечивает одна транзакция writer’а |
| Запрет индивидуального перемещения | API / Backend (`DeviceService::update` и др.) | Browser (disabled `PlacePicker`) | UI-блокировка не заменяет серверную (D-16/D-19) |
| Права (матрица 3 роли × тип/группа) | API / Backend (`authorize` в сервисе + `build_*`) | Browser (скрытие пункта сайдбара) | Роль проверяется на обоих транспортах; сайдбар — косметика |
| Раздел «Группы», дерево, панели, формы | Browser / Client | — | Svelte SPA одинаково в Tauri-webview и LAN-браузере |
| Перестановка свойств | Browser (pointer-events) | API (`reorder` одним вызовом) | Жест — на клиенте; порядок сохраняется атомарно списком id |
| Свёртка пакета в отчёте «Перемещения» | Browser (экран) | API (плоские строки + `batch_*` поля) | Печать — серверный HTML из плоских строк → полный состав без логики сворачивания (D-27) |
| Принтеры карточки (USB-производные + ссылки, дедуп) | API / Backend (SQL на чтении) | Browser | Дедуп по `printers.device_id` на сервере; UI показывает один список |
| Инвалидация счётчиков дерева «Места» | Browser (store) | API (`changed_place_ids` в ответе) | Прецедент фазы 40.1: сервер сообщает затронутые места, клиент инвалидирует |

## Standard Stack

### Core (всё уже в проекте; новых зависимостей фаза не добавляет)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rusqlite (bundled) | 0.38 | SQL, транзакции, `Transaction` | единственный DB-слой проекта [VERIFIED: /Cargo.toml] |
| refinery | 0.9 | миграции V045–V046 (`embed_migrations!("../../migrations")`) | [VERIFIED: migrations.rs] |
| serde / serde_json / serde_with | 1 / 1 / 3 | DTO; `rust::double_option` для `Option<Option<T>>` | [VERIFIED: DevicePatch в `dto/device.rs:196-217`] |
| specta (`=2.0.0-rc.22`) + tauri-specta (`=2.0.0-rc.21`) | pinned | генерация `ui/src/bindings.ts`; `#[specta(type = i32)]` на каждом `i64` | [VERIFIED: dto/place.rs, specta_export.rs] |
| uuid | 1 (`v4`) | `batch_id` пакета; зависимость есть только у `trackly-app` → генерировать в сервисе, передавать в репозиторий параметром | [VERIFIED: /Cargo.toml, crates/trackly-app/Cargo.toml; в `trackly-infra` её нет] |
| std::net::IpAddr | std | валидация и нормализация IPv4/IPv6 | проверено компиляцией (см. Code Examples) [VERIFIED: rustc 1.95 в этой сессии] |
| Svelte 5 (runes) + svelte-spa-router | ^5.55 / ^5.1 | UI, хеш-маршрутизация | [VERIFIED: ui/package.json] |

### Supporting (существующие компоненты UI — собирать из них)

`PlacesMasterDetail` (импортировать, не копировать), `Dropdown` (+аддитивная правка секций), `Table`/`TableRow`, `ActionMenu`, `Modal`, `Button`, `Checkbox`, `Badge`, `Input`, `Tabs`, `DetailPanel`/`DetailSection`/`DetailField`, `MovementTimeline`, `NumberTakenPopup` (образец, не потребитель), `PageHeader`, `Toast` (`pushToast`), `notifyPlaceContentChanged`.

### Alternatives Considered

| Вместо | Альтернатива | Компромисс |
|--------|--------------|------------|
| `group_devices` (junction, PK по `device_id`) | колонка `devices.group_id` через `ALTER ADD COLUMN` | Колонка проще для «в группе ли устройство», но трогает горячую таблицу `devices` (FTS-триггеры, 40+ тестов с `INSERT INTO devices`), смешивает членство с карточкой устройства. Junction не требует правок `devices` вовсе |
| Свой тип-обёртка нумерации | `uuid` для `batch_id` | Целочисленный `batch_id` = `id` первой строки пакета не требует зависимости, но ломается, если группа без места (нет строки группы) — тогда «первой» будет строка устройства; UUID проще |
| Отдельные таблицы значений по типам | одна `group_property_values` | см. §«EAV-схема» |

**Installation:** `# нет — новых пакетов не требуется`

**Version verification:** новые внешние пакеты не рекомендуются; версии существующих зависимостей прочитаны из `/Cargo.toml` и `ui/package.json` (npm/crates registry не запрашивались, так как ничего не устанавливается).

## Package Legitimacy Audit

> Фаза не устанавливает внешних пакетов (ни crates, ни npm). `slopcheck` не запускался за ненадобностью.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| — (новых нет) | — | — | — | — | n/a | — |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
Если планировщик всё же захочет добавить crate (например, `rust_decimal` для `number`) — это решение выходит за рекомендации; тогда обязателен `checkpoint:human-verify` перед установкой. Рекомендация: не добавлять (см. «Don't Hand-Roll»).

## Architecture Patterns

### System Architecture Diagram

```
 Tauri webview (invoke)                       LAN-браузер (POST /api/v1/<cmd>, cookie-сессия)
        │ #[tauri::command] groups_*                    │ axum handler (тонкий адаптер)
        ▼                                               ▼
        └──────────── build_groups_*(ctx, &Identity, …)  (tauri_cmds/groups.rs) ────────────┘
                              │  authorize(Action::{ManageGroupTypes|MutateGroups|ReadGroups})
                              ▼
                        GroupService (trackly-app/services/group_service.rs)
        ┌───────────────┬──────┴───────────────┬─────────────────────────────┐
   чтения (reader pool) │              запись: WriterHandle::execute(closure) │
   spawn_blocking       │                      │  ОДНА conn.transaction():     │
        │               │                      │  1 валидация (домен: ip/mac/…)│
        │               │                      │  2 SqliteGroupRepository      │
        │               │                      │     ::*_in_tx(&tx, …)         │
        │               │                      │  3 SqlitePlaceMovementsRepo   │
        │               │                      │     ::record_*_in_tx (batch)  │
        │               │                      │  4 audit_log (в той же tx)    │
        ▼               ▼                      ▼
  SQLite (WAL): group_types · group_type_properties · groups · group_devices ·
                group_property_values · place_movements(+batch_id, entity_label) ·
                devices.place_id · audit_log
        ▲
  стартовый засев: GroupService::seed_builtin_types_on_startup()  ← AppCtx::build (после миграций)

 Чужие write-site’ы devices.place_id (каждый проходит через group-helper):
   DeviceService::update ──guard──┐   ActService.{create,update,do_return,update_return,undo} ──release──┤
   PlaceService::move_subtree_contents ──«двигать группу целиком»──┤   DeviceService::delete_soft ──release──┘
```

### Recommended Project Structure

```
migrations/
├── V045__groups.sql                       # 5 таблиц + индексы + триггер неизменяемости
└── V046__place_movements_batch.sql        # ALTER TABLE place_movements ADD COLUMN ×2 + индекс
crates/trackly-core/src/
├── domain/groups.rs                       # GroupTypeRow, GroupBehavior, PropertyDataType, GroupRow … (без serde/specta)
├── domain/group_values.rs                 # normalize_ip/mac/number, ValueError — чистые функции
├── ports/groups.rs                        # GroupRepository (CRUD-часть; compound-операции — inherent *_in_tx)
└── auth.rs                                # + Action::{ManageGroupTypes, MutateGroups, ReadGroups}
crates/trackly-infra/src/repos/
└── groups_sqlite.rs                       # SqliteGroupRepository: *_in_tx, рекурсивные CTE
crates/trackly-app/src/
├── dto/groups.rs                          # DTO (specta Type, i64 → #[specta(type = i32)])
├── services/group_service.rs              # GroupService (+ seed_builtin_types_on_startup)
├── tauri_cmds/groups.rs                   # build_groups_* + #[tauri::command] обёртки
├── http/groups.rs                         # router(): POST /api/v1/<cmd>
├── specta_export.rs                       # + регистрация всех команд
└── context.rs                             # + groups: Arc<GroupService>, вызов засева
ui/src/
├── features/groups/                       # GroupsPage, GroupTree(+Node), GroupTypePanel, GroupPanel,
│                                          # PropertiesTable, GroupFormFields, CompositionTable, GroupMoveModal …
├── lib/api/groups.ts                      # обёртка apiCall (стиль devices.ts)
├── routes.ts, features/layout/sidebar-config.ts
└── scripts/check-group-vocabulary.mjs     # гейт словаря (+ --selftest) в `pnpm lint`
```

### Pattern 1: EAV-схема — одна таблица значений (рекомендация) [VERIFIED: DDL выполнен в sqlite3 3.51]

**Сравнение вариантов против ограничений фазы:**

| Критерий | A. Одна `group_property_values` (`value_text` + `value_ref`) | B. Типизированные колонки (`value_text/num/ip/…`) | C. JSON-колонка в `groups` | D. Junction на каждый тип связи + таблица скаляров |
|----------|------|------|------|------|
| Только аддитивные миграции, будущие типы GRPX-02 (enum/list/file) без перестройки | ✅ новый `data_type` — строка, колонки уже есть | ❌ новый тип = `ADD COLUMN` | ✅ | ❌ каждая новая связь — новая таблица |
| Нормализация `ip`/`mac` на сервере | ✅ хранится нормализованный `value_text` | ✅ | ⚠️ нет схемы | ✅ |
| «Скрыть, а не удалить заполненное» (`EXISTS` по property) | ✅ один `EXISTS … WHERE property_id=?` | ✅ | ❌ JSON-запрос по всем группам | ⚠️ `EXISTS` по 3 таблицам |
| «Нельзя сменить `data_type` при значениях» | ✅ тот же `EXISTS` | ✅ | ❌ | ⚠️ |
| «Нельзя включить `is_required` — назвать группы» | ✅ один anti-join (`NOT EXISTS`) | ✅ | ❌ | ⚠️ UNION по таблицам |
| Запрос панели группы | ✅ один `SELECT … ORDER BY p.sort_order, v.position` | ✅ | ✅ | ⚠️ 3 запроса |
| Потребитель Фазы 43 (`show_on_map`) | ✅ `JOIN` по `p.show_on_map=1` и `group_id IN (…)` | ✅ | ❌ | ⚠️ |
| Обратный поиск «какие группы ссылаются на это устройство» (Фаза 42, ранжирование принтеров) | ✅ индекс `(value_ref, property_id)` | ✅ | ❌ | ✅ |
| Целостность ссылок | ⚠️ нет FK на `users`/`devices` (полиморфная ссылка) → проверка «жив» при записи + фильтр живых при чтении | ✅ FK | ❌ | ✅ FK |

**Рекомендация: вариант A.** Минус (нет FK у `value_ref`) смягчается тем, что «жив» всё равно приходится проверять на чтении: устройства удаляются мягко (`deleted_at_utc`), FK тут бы не помог. Остаточная цена принята.

```sql
-- V045__groups.sql (фрагмент; полный набор — в плане). Все _at_utc — INTEGER (тест per_record_invariants).
CREATE TABLE group_types (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  code TEXT NOT NULL,                       -- машинно-стабильный, неизменяемый
  name TEXT NOT NULL,
  behavior TEXT NOT NULL,                   -- 'container'|'substitute'|'teardown' — БЕЗ SQL CHECK (см. Pitfall 9)
  is_builtin INTEGER NOT NULL DEFAULT 0,
  sort_order INTEGER NOT NULL DEFAULT 0,
  quick_action_enabled INTEGER NOT NULL DEFAULT 0,
  quick_action_label TEXT NULL,
  created_at_utc INTEGER NOT NULL, updated_at_utc INTEGER NOT NULL,
  deleted_at_utc INTEGER NULL,              -- standard4; не пишется (hard delete, как places)
  version INTEGER NOT NULL DEFAULT 1
);
CREATE UNIQUE INDEX idx_group_types_code ON group_types(code);
CREATE TRIGGER trg_group_types_immutable BEFORE UPDATE OF code, behavior ON group_types
  WHEN NEW.code IS NOT OLD.code OR NEW.behavior IS NOT OLD.behavior
  BEGIN SELECT RAISE(ABORT, 'group_types.code/behavior are immutable'); END;

CREATE TABLE group_type_properties (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  type_id INTEGER NOT NULL REFERENCES group_types(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  data_type TEXT NOT NULL,                  -- 'text'|'number'|'ip'|'mac'|'users'|'device_refs' — БЕЗ CHECK (GRPX-02)
  sort_order INTEGER NOT NULL DEFAULT 0,
  is_required INTEGER NOT NULL DEFAULT 0,
  show_on_map INTEGER NOT NULL DEFAULT 0,
  archived_at_utc INTEGER NULL,
  created_at_utc INTEGER NOT NULL, updated_at_utc INTEGER NOT NULL,
  deleted_at_utc INTEGER NULL, version INTEGER NOT NULL DEFAULT 1
);
CREATE UNIQUE INDEX idx_gtp_name_live ON group_type_properties(type_id, name) WHERE archived_at_utc IS NULL;

CREATE TABLE groups (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  type_id INTEGER NOT NULL REFERENCES group_types(id) ON DELETE RESTRICT,
  name TEXT NOT NULL,                       -- свободный, НЕ уникальный
  seq INTEGER NOT NULL,                     -- MAX(seq)+1 по типу, считается в writer-транзакции
  place_id INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT,
  parent_group_id INTEGER NULL REFERENCES groups(id) ON DELETE SET NULL,
  created_at_utc INTEGER NOT NULL, updated_at_utc INTEGER NOT NULL,
  deleted_at_utc INTEGER NULL, version INTEGER NOT NULL DEFAULT 1
);
CREATE UNIQUE INDEX idx_groups_type_seq ON groups(type_id, seq);       -- бэкстоп гонки
CREATE INDEX idx_groups_parent ON groups(parent_group_id) WHERE parent_group_id IS NOT NULL;
CREATE INDEX idx_groups_place  ON groups(place_id)        WHERE place_id IS NOT NULL;

CREATE TABLE group_devices (               -- junction: НЕТ version/deleted (per_record_invariants)
  device_id INTEGER PRIMARY KEY REFERENCES devices(id) ON DELETE CASCADE,   -- PK = «не более одной группы»
  group_id  INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
  added_at_utc INTEGER NOT NULL
);
CREATE INDEX idx_group_devices_group ON group_devices(group_id);

CREATE TABLE group_property_values (       -- НЕТ version/deleted
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  group_id    INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
  property_id INTEGER NOT NULL REFERENCES group_type_properties(id) ON DELETE CASCADE,
  position    INTEGER NOT NULL DEFAULT 0,
  value_text  TEXT NULL,                   -- text | канонический number | нормализованный ip | нормализованный mac
  value_ref   INTEGER NULL,                -- users.id | devices.id (по data_type свойства)
  is_primary  INTEGER NOT NULL DEFAULT 0 CHECK (is_primary IN (0,1)),
  updated_at_utc INTEGER NOT NULL
);
CREATE UNIQUE INDEX uq_gpv_scalar  ON group_property_values(group_id, property_id) WHERE value_ref IS NULL;
CREATE UNIQUE INDEX uq_gpv_ref     ON group_property_values(group_id, property_id, value_ref) WHERE value_ref IS NOT NULL;
CREATE UNIQUE INDEX uq_gpv_primary ON group_property_values(group_id, property_id) WHERE is_primary = 1;
CREATE INDEX idx_gpv_property ON group_property_values(property_id);
CREATE INDEX idx_gpv_ref ON group_property_values(value_ref, property_id) WHERE value_ref IS NOT NULL;
PRAGMA user_version = 45;
```

**Правила, которые держит эта схема:**
- «Пусто» = **отсутствие строки** (пустое значение не пишется). Поэтому «обязательное, но пустое» — это `NOT EXISTS`, а «заполнено» — `EXISTS`.
- Скаляр: ровно одна строка `(group, property)` с `value_ref IS NULL`. Связи: по строке на ссылку, `value_ref` = id; `is_primary=1` не более одной на `(group, property)` (частичный уникальный индекс) — применимо к `users`.
- `show_on_map` хранится и отдаётся DTO; запрос Фазы 43: `… FROM group_type_properties p JOIN group_property_values v ON v.property_id = p.id WHERE p.show_on_map = 1 AND p.archived_at_utc IS NULL AND v.group_id IN (…)`.
- Standard4 (`deleted_at_utc`/`version`) у трёх «пользовательских» таблиц — по конвенции D-Schema-03/04; junction/значения — без них. Добавить `group_types`/`group_type_properties`/`groups` в `USER_MUTABLE_TABLES`, а `group_devices`/`group_property_values` в `SYSTEM_TABLES` теста `per_record_invariants.rs` [VERIFIED: crates/trackly-infra/tests/per_record_invariants.rs]. Если планировщик решит не вводить `deleted_at_utc` для hard-delete таблиц — тест просто не включает их в первый список; конвенция `places` (колонка есть, не пишется) — предпочтительный вариант.
- Уникальность имён свойств — частичный индекс по байтам + проверка без учёта регистра в Rust (SQLite `lower()` не сворачивает кириллицу; прецедент — комментарий V043).

### Pattern 2: Миграции — следующие номера V045, V046 [VERIFIED: `ls migrations` → голова V044]

- **V045** — таблицы выше. **V046** — только `ALTER TABLE place_movements ADD COLUMN batch_id TEXT NULL; ADD COLUMN entity_label TEXT NULL;` + `CREATE INDEX idx_place_movements_batch ON place_movements(batch_id) WHERE batch_id IS NOT NULL; PRAGMA user_version = 46;` Оба `ALTER` проверены в `sqlite3`. Каждый файл обязан заканчиваться `PRAGMA user_version = N;` (конвенция V0xx) [VERIFIED].
- **Никакой перестройки** `place_movements`/`devices`. Подтверждено чтением V040: `entity_type`/`source` — голый `TEXT NOT NULL` без `CHECK` (V040 шапка, «validation … Rust-side only»), поэтому `entity_type='group'` и `source='group'` схемы не меняют [VERIFIED: migrations/V040__place_movements.sql:30-44].
- Почему перестройка запрещена: `migrations::run` сам выключает `foreign_keys` вне транзакции перед refinery и сверяет `PRAGMA foreign_key_check` с базовой линией, потому что `PRAGMA foreign_keys=OFF` внутри refinery-транзакции — no-op; V042 стёр бы `act_items` (`ON DELETE CASCADE`) [VERIFIED: migrations.rs `run`/`set_foreign_keys`, шапка V042]. Аддитивные V045/V046 этой проблеме не подвержены.
- **`batch_id` — TEXT (UUID v4)**, генерируется сервисом. `entity_label` — снимок имени группы («АРМ #3») на момент переноса: журнал не JOIN-ит (прецедент снимков путей D-09/D-10 V040), а группа может быть переименована/удалена; без него отчёт «АРМ #3 (6 устройств)» (D-26) не собрать для удалённой группы и для пакета без строки группы (см. Pitfall 7).
- **Тест миграции** (образец `acts_number_text_migration.rs` + `run_up_to`): `run_up_to(conn, 44)` → засеять `places`/`devices`/`place_movements`/`act_items` → `run` → проверить: id и количество строк `place_movements`/`act_items` не изменились, `PRAGMA foreign_key_check` пуст, в `sqlite_master` нет `place_movements_new`, колонки `batch_id`/`entity_label` присутствуют, `max_known_version() == 46`.

### Pattern 3: Идемпотентный засев [VERIFIED: context.rs, supervisor.rs, V014]

Прецеденты: `templates.seed_defaults_on_startup()` (`context.rs:239`), `seed_supervisor_tasks` (`INSERT OR IGNORE`, `supervisor.rs:331`), `device_statuses.code` с уникальным индексом (V014).

**Где:** стартовый сервис `GroupService::seed_builtin_types_on_startup()`, вызывается из `AppCtx::build` после создания writer’а (рядом с `seed_supervisor_tasks`), **не миграция**. Почему: (1) SPEC req.2 формулирует «при старте приложения отсутствующие типы создаются» — будущие встроенные типы получат засев без новой миграции; (2) миграция выполняется один раз и не способна «досеять» удалённую вручную строку; (3) тест «переименовал → перезапустил засев» требует вызываемой функции. Миграция остаётся чисто DDL.

```sql
INSERT INTO group_types (code, name, behavior, is_builtin, sort_order, quick_action_enabled,
                         quick_action_label, created_at_utc, updated_at_utc, version)
VALUES (?1, ?2, ?3, 1, ?4, ?5, ?6, ?7, ?7, 1)
ON CONFLICT(code) DO NOTHING;      -- НЕ DO UPDATE и НЕ INSERT OR REPLACE: переименование выживает
```

Засеваемые значения: `workstation` («АРМ», `container`, «Сформировать группу»), `system_unit` («Системный блок», `substitute`, «Замещение группой»), `teardown` («Разбор», `teardown`, «На разбор»). Сид идёт через сервис с `clock.unix_seconds()`; в одном writer-замыкании на все три вставки (одна транзакция).

### Pattern 4: Единая транзакция переноса группы [VERIFIED: place_service.rs `move_subtree_contents`, writer_worker.rs]

Writer: `WriterHandle::execute(FnOnce(&mut Connection) -> Result<R, AppError>)` исполняет замыкание на единственном `spawn_blocking`-потоке; очередь `mpsc(256)`, `send_timeout` 5 с → `WriteQueueBusy` [VERIFIED: writer_worker.rs]. Любое замыкание = нарушение «писатель один» невозможно.

**Идиом, который НЕ копировать:** `PlaceService::{create,rename,move_node,…}` вызывают метод репозитория на `&mut Connection` (автокоммит), а потом открывают короткую `conn.transaction()` только ради `audit_log` (шапка `place_service.rs:16-30` объясняет: `&mut Transaction` не удовлетворяет `&mut Self::Conn`, так как нет `DerefMut`). Для групп мутация и журнал/аудит обязаны лежать в ОДНОЙ транзакции — копировать нужно **`move_subtree_contents`** (`conn.transaction()` → цикл → `tx.commit()`) и `DeviceService::update` (мутация + `audit_log` + `place_movements` в одной `tx`).

**Следствие для портов:** составные операции — inherent-методы `SqliteGroupRepository::*_in_tx(&self, tx: &Transaction<'_>, …)` (как `SqliteDeviceRepository::*_in_tx` и `SqlitePlaceMovementsRepository::*_in_tx`), а не методы trait’а `GroupRepository`. Trait — для простого CRUD, если планировщик его вводит.

**Образец защиты от циклов** (`places_sqlite.rs::move_node`, Pattern 3 фазы 39) — перенести на `parent_group_id` дословно, в той же транзакции перед `UPDATE`:

```sql
WITH RECURSIVE ancestors(id) AS (
  SELECT parent_group_id FROM groups WHERE id = ?1          -- ?1 = НОВЫЙ родитель
  UNION ALL
  SELECT g.parent_group_id FROM groups g JOIN ancestors a ON g.id = a.id
  WHERE g.parent_group_id IS NOT NULL
)
SELECT EXISTS(SELECT 1 WHERE ?1 = ?2                          -- ?2 = перемещаемая группа
              UNION ALL SELECT 1 FROM ancestors WHERE id = ?2);
```
Дополнительные правила вложения (SPEC req.8): родитель с `behavior = 'teardown'` отклоняется; `Validation{field:"parent_group_id"}` с русским сообщением.

**Состав группы (с вложенными)** — рекурсивный CTE вниз по `parent_group_id` (форма `subtree_stats_impl`):

```sql
WITH RECURSIVE sub(id) AS (
  SELECT id FROM groups WHERE id = ?1
  UNION ALL
  SELECT g.id FROM groups g JOIN sub s ON g.parent_group_id = s.id
)
SELECT gd.device_id FROM group_devices gd
JOIN sub ON gd.group_id = sub.id
JOIN devices d ON d.id = gd.device_id AND d.deleted_at_utc IS NULL;
```

**Скелет `move_group_in_tx`** (всё в одном замыкании writer’а; `Identity` не `Send` через границу — вынуть `user_id` ДО замыкания, как в `DeviceService::update`):

1. `get` группы; если `parent_group_id IS NOT NULL` → `Validation{field:"place_id", message:"Место вложенной группы задаётся корневой группой"}` (SPEC: только чтение).
2. `places.get(tx, target)` → `NotFound`/`Validation`, если места нет.
3. Собрать `(group_ids, device_ids)` CTE выше.
4. `UPDATE groups SET place_id=?, updated_at_utc=?, version=version+1 WHERE id IN (SELECT id FROM sub)` — **денормализация**: у вложенных групп `place_id` дублирует корневой и обновляется в той же транзакции (поэтому guard запрета читает место ПРЯМОЙ группы устройства без подъёма по цепочке).
5. Строка группы: `record_batch_movement…(Group, group.id, before=Some(old), after=Some(target), source=Group, batch_id, label=group.name)`; затем на каждое устройство: `before = devices.get_in_tx`; `update_status_and_place_in_tx(tx, id, before.status_id, Some(target), now)` (тот же вызов использует `move_subtree_contents`); строка устройства с тем же `batch_id`/`label`; если устройство — принтер, `cartridge_repo.cascade_place_for_printer_in_tx(…)` с тем же `batch_id`.
6. `audit_log` (`entity_type:"group"`, `action:"move"`, payload с `batch_id`, числом устройств) → `tx.commit()`. Возвращать `{moved_devices, moved_nested_groups, changed_place_ids}` (старые+новые места — для INV-7/D-24 «группа и 6 устройств»).

**Правила журнала** (SPEC req.10 + `is_reportable_place_change`): строка группы пишется только при `Some→Some` разных мест; вложенные группы строк **не** пишут (приёмка: 2 из 6 во вложенной → ровно **7** строк = 1 группа + 6 устройств); устройство без места (`NULL → место`) — строки нет, вместо неё `audit_log` (D-30). Подробности — §«Журнал и отчёт».

### Pattern 5: Инвентаризация write-site’ов `devices.place_id` — ОТ СЕРВЕРНЫХ МУТАЦИЙ (урок 40.1) [VERIFIED: grep + чтение, HEAD 5ee71084]

> Номера строк даны на HEAD `5ee71084`; перед планированием пересвериться `grep -n "update_status_and_place_in_tx\|update_full_in_tx\|restore_from_snapshot_in_tx\|UPDATE devices SET" crates/*/src -r`.

| # | Серверная мутация (место) | Транспорт (команда) | Что пишет | Поведение фазы 41 | Тест |
|---|---------------------------|---------------------|-----------|-------------------|------|
| S1 | `DeviceService::update` → `SqliteDeviceRepository::update_in_tx` (`device_service.rs:633`, CASE по `place_id`) | `devices_update` | `devices.place_id` | **Guard (D-19, SPEC 11):** внутри writer-замыкания ДО `update_in_tx`: если `patch.place_id` задан и **отличается от текущего** и у прямой группы устройства `place_id IS NOT NULL` → `Validation{field:"place_id", message:"Место задаётся группой «…». Выведите устройство из состава, чтобы переместить его отдельно."}`. Тот же `place_id` (форма шлёт его всегда!) — проходит. Группа без места (D-21) — проходит | `devices_update` на обоих транспортах: реальная смена → ошибка; повторная отправка текущего места → ok; после вывода → ok; группа без места → ok |
| S2 | `PlaceService::move_subtree_contents` (`place_service.rs:686`; через `update_status_and_place_in_tx`) | `places_move_subtree_contents` | `devices.place_id`, `cartridges.place_id` | **D-23:** в той же внешней `tx` для устройств, чей **корневой** группа имеет место, — не двигать устройство, а вызвать `move_group_in_tx` для корневой группы (дедуп по группе), устройства группы из цикла исключить (иначе двойные строки журнала). Спящая группа (place NULL) — устройство двигается как обычное. Возвращаемое `usize` не менять (bindings/UI) | Склад с группой (6 устройств) + 2 одиночных → группа переехала целиком, строк журнала: 1+6 (batch) + 2 (manual), всё откатывается при сбое |
| S3 | `ActService::create` (handover), `act_service.rs:596` (`update_status_and_place_in_tx`) | `acts_create` | `devices.place_id` | **D-22:** после UPDATE — `release_device_in_tx` (удалить строку `group_devices` + `audit_log` `action:"custom:group_member_released"`, payload `{group_id, act_id}`) | handover устройства-члена → членство снято, акт создан |
| S4 | `ActService::update` — добавленные (`:948`) и убранные (`:1133`, `restore_from_snapshot_in_tx`) | `acts_update` | `devices.place_id` | то же: release для добавленных; для убранных (restore) — release, если устройство сейчас член активной группы (иначе restore затёр бы «место принадлежит группе») | 2 кейса |
| S5 | `ActService::do_return` (`:1756`, `update_full_in_tx`) | `acts_return` | `devices.place_id` | release (D-22) | кейс |
| S6 | `ActService::update_return` — `:2285` (restore), `:2355`, `:2443` (`update_full_in_tx`) | `acts_update_return` | `devices.place_id` | release на каждом из 3 | 3 кейса |
| S7 | `undo_device_mutations_for_act` (`act_service.rs:3645`, `restore_from_snapshot_in_tx`) | `acts_delete` (undo) | `devices.place_id` из `before_json` | release, если устройство сейчас член группы с местом; членство НЕ восстанавливается | кейс |
| S8 | `DeviceService::delete_soft` (`device_service.rs:801`) | `devices_delete` | `devices.deleted_at_utc` | **удалить членство в той же `tx`** (иначе «мёртвые» члены в составе, счётчик и запрет «живут» на удалённом устройстве); плюс запросы состава фильтруют `d.deleted_at_utc IS NULL` | кейс |
| S9 | `CartridgeService::transition` (install) → `cartridges_sqlite.rs:646` (`UPDATE devices SET place_id … WHERE place_id IS NULL`) | `cartridges_transition` | `devices.place_id` принтера, только если было NULL | **Без guard’а — сознательно:** у принтера-члена группы с местом место не NULL → условие `IS NULL` не срабатывает; у спящей группы (D-21) место редактируется как обычное. Зафиксировать комментарием и тестом | кейс |
| S10 | `DeviceService::create*`/`bulk_create*`/`import_csv_commit`, `printers_sqlite.rs:523` | `devices_create`, CSV | INSERT с `place_id` | **Без изменений:** новое устройство не может быть членом группы | не нужен |
| — | `places_*` (rename/move/archive/delete узла дерева) | `places_*` | не трогают `devices.place_id` | **Но:** `delete_hard` места, на которое ссылается группа, упадёт на FK `groups.place_id ON DELETE RESTRICT` — см. Pitfall 6 | кейс |

**Что должен проверять сквозной гейт.** (а) Поведенческий тест на каждую строку S1–S9 (таблица-драйвер). (б) **Счётный гейт на исходник** (по образцу `number_space_broadcast_gate.rs`, где реестр = комментарий «добавь кейс» + кейсы): тест читает `include_str!("../src/services/act_service.rs")` и утверждает `matches("update_status_and_place_in_tx(").count() + matches("update_full_in_tx(").count() + matches("restore_from_snapshot_in_tx(").count() == 8` (текущее число + helper’ы), с сообщением «новое место записи `devices.place_id` в act_service.rs — добавь кейс release в `group_write_sites.rs`». Якорь мутации подбирать **уникальный** (урок `mutation_test_anchor_must_be_unique`): строка вызова `release_device_in_tx(` с аргументами конкретного сайта, а не общий `authorize`/имя метода.
Текущий счёт проверен: ровно 8 вхождений этих трёх вызовов в `act_service.rs` (строки 596, 948, 1133, 1756, 2285, 2355, 2443, 3645), комментарии их без скобки не засоряют. Гейт расширить на весь `crates/*/src`: вызовы `update_status_and_place_in_tx(`/`update_full_in_tx(`/`restore_from_snapshot_in_tx(` допустимы только в `act_service.rs` (8), `place_service.rs` (1 — `move_subtree_contents`) и самих определениях в `devices_sqlite.rs`; новый файл-потребитель — красный. (в) Клиентский INV-7 (см. §UI) для новых клиентских producer’ов.

### Pattern 6: Паритет транспортов — инвентарь команд [VERIFIED: tauri_cmds/places.rs, http/places.rs, role_endpoint_matrix.rs]

Устоявшаяся форма: `pub async fn build_places_*(ctx, &Identity, …)` в `tauri_cmds/places.rs`; `#[tauri::command] #[specta::specta]` обёртка берёт `resolve_tauri_identity` и делегирует; HTTP-обработчик в `http/places.rs` берёт `session_identity(&session)` и вызывает **ту же** `build_places_*`; маршруты — `POST /api/v1/<имя_команды>` с `#[serde(rename_all="camelCase")]` payload’ами; Tauri-аргументы id — `i32` (specta запрещает BigInt), DTO — `i64` с `#[specta(type = i32)]`; регистрация в `specta_export.rs::builder()`; DTO — `trackly-app/src/dto`, домен `trackly-core` без serde/specta. `build_*` и сервис оба зовут `authorize` (защита в глубину). Так как HTTP делегирует в те же `build_*`, паритет структурный; тестировать надо обе точки входа (прецедент IN-02: «гейт, доказанный на одном транспорте, — реальная прошлая дыра»).

**Операции (24 новых; каждая — Tauri-команда + HTTP-маршрут + строка в `specta_export.rs`):**

| Группа гейта | Команды |
|--------------|---------|
| **`ManageGroupTypes` (Admin)** | `group_types_create`, `group_types_update` (имя, сортировка, быстрое действие; DTO **содержит** опциональные `code`/`behavior`, отклоняемые при отличии — иначе HTTP-приёмка «изменение `code` отклоняется» вакуумна: serde молча проигнорировал бы неизвестное поле), `group_types_delete` (встроенный и тип с группами — отказ), `group_type_properties_create`, `group_type_properties_update` (+ смена `data_type`/`is_required`), `group_type_properties_delete` (скрыть, если есть значения; иначе удалить; вернуть исход), `group_type_properties_unarchive`, `group_type_properties_reorder(type_id, ordered_ids)` (одной транзакцией) |
| **`ReadGroups` (Admin\|Manager)** | `group_types_list(include_archived)` (типы с вложенными свойствами), `group_type_properties_empty_groups(property_id)` (для попапа D-14), `groups_list`, `groups_get`, `groups_composition(group_id)`, `groups_search(query, exclude_group_id)` (кандидаты во вложенные, без самой группы и потомков), `groups_for_devices(device_ids)` (членство пачкой — для D-19 и пометки уже занятых в поиске D-01), `groups_user_options(query)` |
| **`MutateGroups` (Admin\|Manager)** | `groups_create`, `groups_update` (имя), `groups_delete`, `groups_set_parent`, `groups_add_devices(group_id, device_ids[])` (один обработчик на оба пути D-01), `groups_remove_devices`, `groups_move(id, target_place_id)`, `groups_set_values(id, version, values[])` |
| **Расширяемые существующие** | `devices_update` (S1), `places_move_subtree_contents` (S2), `acts_*` (S3–S7), `devices_delete` (S8), `place_movements_get_timeline` (поддержать `entity_type='group'`, поля `batch_id`/`group_id`/`group_label`), `reports_list_movements`/`reports_export_*` (пакет), `places_subtree_stats` (+`referencing_group_count`) |

Матрица прав (SPEC req.13) — **3 роли × (тип, группа) × 2 транспорта**; план теста — таблица-драйвер `(команда, GateKind::{TypeMut, GroupMut, Read}, payload)` и тест полноты: `include_str!` исходника `http/groups.rs`, регэксп `\.route\("/api/v1/([a-z_]+)"` → множество, сравнить с множеством таблицы (новый маршрут без строки в таблице — красный).

### Pattern 7: Права — новые `Action` [VERIFIED: auth.rs]

`Action` — перечисление с **исчерпывающим** `match` в `authorize`, поэтому добавление вариантов без ветки не скомпилируется. Добавить: `ManageGroupTypes` (Admin only, ветка рядом с `ManageUsers|ManageSettings|MutatePlaces`), `MutateGroups` и `ReadGroups` (Admin|Manager). Дописать unit-тесты матрицы в `auth.rs::tests`. Не использовать `MutatePlaces` (он Admin-only — маскировка ошибки «менеджер не создаёт группу»).

### Pattern 8: Валидация значений — чистый домен [VERIFIED: IpAddr проверен компиляцией]

Модуль `trackly-core/src/domain/group_values.rs`, без I/O (проходит `tests/no_io_deps.rs`); сервис вызывает его **до** записи; обе транспортные точки входа идут через сервис, поэтому «невалидное отклоняется не только в UI» выполняется структурно.

- `ip`: `value.trim().parse::<IpAddr>()` → `to_string()` (проверено: `"2001:DB8:0:0:0:0:0:1"` → `2001:db8::1`; отвергнуты `01.2.3.4`, `1.2.3`, `fe80::1%eth0`, `10.0.0.1/24`, `192.168.0.256`, пробел перед числом без `trim`).
- `mac`: убрать разделитель только если он **один и тот же** по всей строке (`:`, `-` или ни одного), ровно 12 hex-цифр → нижний регистр → канонический вид `aa:bb:cc:dd:ee:ff`; смешанные разделители и группы не по 2 цифры — отказ.
- `number`: `trim`, запятая → точка, допустимы знак и дробная часть, `f64::from_str` с проверкой `is_finite`, ограничение длины (напр. ≤ 32 символов); без нового crate.
- `text`: `trim`, ограничение длины (константа, напр. 2000); пустое → «нет значения» (строка не пишется).
- `users`: каждый `user_id` существует в `users` с `deleted_at_utc IS NULL AND is_active = 1` (V002 + V019); ≤ 1 основной; дубликаты отклоняются (частичный уникальный индекс — бэкстоп).
- `device_refs`: каждый id — `devices` с `deleted_at_utc IS NULL`; дубликаты отклоняются; верхняя граница числа ссылок (напр. 100).
- `is_required`: непустое значение (для связей — ≥ 1 запись) для **неархивных** свойств типа; проверяется при `groups_create` (DTO несёт `values`) и `groups_set_values`.

Ошибки — `AppError::Validation{field:"values.<property_id>", message:…}` по-русски.

### Pattern 9: Карточка группы и принтеры [VERIFIED: V020 `printers.usb_host_device_id`; `printers.device_id UNIQUE`]

USB-принтеры состава (включая вложенные) — производные, чтение:

```sql
-- device_ids состава из CTE «sub» выше
SELECT d.id AS device_id, d.name, d.inventory_number, 'usb' AS origin
FROM printers p
JOIN devices d ON d.id = p.device_id AND d.deleted_at_utc IS NULL
WHERE p.usb_host_device_id IN (SELECT gd.device_id FROM group_devices gd JOIN sub ON gd.group_id = sub.id)
```
Явные ссылки: `value_ref` строк свойств `data_type='device_refs'`. **Дедупликация — по `device_id` принтера** (`printers.device_id`), на сервере: итоговый список = `UNION` по `device_id`, при совпадении `origin='usb'` побеждает (нет кнопки удаления, D-12). Приёмка: USB-принтер + явная ссылка → одна строка.

### Pattern 10: Журнал и отчёт «Перемещения» [VERIFIED: place_movements_sqlite.rs, report_service.rs:1444-1620, place_movement_service.rs]

1. **Репозиторий:** добавить `batch_id: Option<&str>` и `entity_label: Option<&str>` в `NewMovement` — структура строится только внутри `place_movements_sqlite.rs` [VERIFIED: grep]. Добавить **соседний** метод `record_batch_movement_if_applicable` (+ общий приватный хелпер), **не менять** сигнатуру `record_movement_if_applicable` (11 позиционных аргументов, 7 существующих write-site’ов; риск — урок «5 копий, одна забыла»).
2. **Core:** `MovementEntityKind::Group` (`as_str "group"`, `label_ru "Группа"`, `from_str_lenient`), `MovementSource::Group` (`"group"`); существующий тест `…rejects_printer` остаётся. `Workstation` не удалять (CONTEXT: оставить мёртвым).
3. **`get_history`/`MovementRow`/`MovementEntryDto`:** добавить `batch_id`, `entity_label`; в DTO для устройств с `source='group'` — `group_id` (выводится: `SELECT entity_id FROM place_movements WHERE batch_id=? AND entity_type='group'`; `NULL`, если строки группы нет) и `group_label` (из `entity_label`). `place_movements_get_timeline(entity_type="group", id)` уже работает без правок сервиса — репозиторий не валидирует токен.
4. **`MovementTimeline.svelte` — единственный владелец анатомии строки:** `reasonText` получает ветки: `entity_type==='group'` → «перенос группы»; `source==='group'` у устройства → «в составе группы «{group_label}»» с кнопкой-ссылкой на группу (как «актом №» — `onNavigateToAct`-образец, новый проп `onNavigateToGroup`). JS-зеркала формулы `*_place_path_short` не создавать.
5. **Отчёт (`query_movements_inner`):** текущие места, где группа ломает отчёт, если ничего не сделать:
   - `CASE pm.entity_type … ELSE pm.entity_type` → у группы выйдет сырое «group»: добавить `WHEN 'group' THEN 'Группа'`;
   - `COALESCE(d.name, c.code) AS device_name` → у группы `NULL`: `COALESCE(d.name, c.code, pm.entity_label)`;
   - `LIMIT 1000` может разрезать пакет → счётчик «(N устройств)» считать подзапросом по `batch_id`, а не по видимым строкам;
   - фильтр «тип устройства» (`d.type_id = ?`) убирает строку группы, оставляя «осиротевшие» строки пакета → UI обязан рисовать строки с `batch_id` без заголовка как обычные;
   - `movement_reason`: добавить `Group => «в составе группы «{label}»»`/«перенос группы»; сырой токен для неизвестного остаётся как есть.
   - Добавить в `ReportRow` опциональные `batch_id`, `batch_role` (`header|member|null`), `batch_size` — **не менять** `columns_for("movements")`, чтобы не трогать `check-report-type-parity.mjs`, CSV и `COLUMNS_MAP` (UI).
6. **Печать (D-27) — без изменений шаблона:** печать идёт из серверного HTML (`reports_export_pdf` → `report.html`), то есть из плоских строк; в плоском списке уже есть и строка группы, и строки устройств (`ORDER BY created_at_utc, id` — внутри транзакции id идут подряд, пакет не разрывается другими записями). Сворачивание — только в `ReportTable.svelte` на экране. **Не менять `templates/report.html`** (иначе включаются механизм `_legacy_defaults/vNN` и «strict-undefined» предпросмотр редактора шаблонов — оба известные ловушки проекта).
7. **Аудит D-30:** устройство `NULL → место группы` — строки журнала нет; писать `audit_log` (`entity_type:"device"`, `entity_id=<device>`, `action:"custom:group_place_assigned"`, `payload_json {group_id, place_id}`) в той же транзакции; `entity_type:"device"` выбран, чтобы Фаза 41.5 (MSG-02) читала историю устройства из одного места. (Решение в дискреции; альтернатива `entity_type:"group"`.)

### Pattern 11: UI — перестановка свойств на pointer-events (D-10) [VERIFIED: PlaceTree.svelte:717-846, 39-UAT GAP-2/GAP-11]

**Прецедент (живой UAT на Tauri + LAN, закрыт):** контейнер ловит `onpointerdown/move/up/cancel`; `pointerdown` только запоминает `originId`, `pointerId`, старт; на `pointermove` при `hypot(dx,dy) ≥ 6` px — старт драга, `setPointerCapture(e.pointerId)` на `e.currentTarget`, `e.preventDefault()`; призрак — `position:fixed`, `pointer-events:none` (иначе он перехватывает `elementFromPoint`, GAP-11); `pointerup` без превышения порога = обычный клик; `pointercancel` сбрасывает состояние; `releasePointerCapture` в `try/catch`. Структуру обработчиков копировать оттуда.

**Что новое для вертикального списка строк `<tr>`:** вместо `elementFromPoint` — индекс вставки по серединам прямоугольников строк (стабильнее над `<tr>` и призраком):

```ts
// ui/src/lib/utils/reorder.ts — чистые функции без DOM/рун, проверяются node-гейтом
export function insertionIndex(rects: { top: number; height: number }[], pointerY: number): number {
  for (let i = 0; i < rects.length; i++) {
    if (pointerY < rects[i].top + rects[i].height / 2) return i;
  }
  return rects.length;
}
/** `to` — индекс вставки в ИСХОДНОМ массиве; после удаления элемента индекс сдвигается. */
export function reorder<T>(items: T[], from: number, to: number): T[] {
  const next = items.slice();
  const [moved] = next.splice(from, 1);
  next.splice(to > from ? to - 1 : to, 0, moved);
  return next;
}
```

**Правила:** (1) запускать драг только за явную ручку (иконка «⠿» в первой ячейке, `button` с `aria-label="Переместить свойство"`, `touch-action:none` на ней), иначе клик по чекбоксам/полям строки превращается в драг; (2) обработчики вешать на контейнер `<tbody>`, `setPointerCapture` — на него; (3) во время драга заглушить выделение текста (`user-select:none` через класс на контейнере, снять в `resetDrag`); (4) **клавиатурный/доступный путь — основной, не запасной:** пункты «Выше»/«Ниже» в `ActionMenu` строки (ноль новой механики, одинаково в обоих webview, проверяется автоматически как вызов `reorder`) + `aria-live` «Свойство «X» перемещено на позицию N из M»; (5) сохранение — один вызов `group_type_properties_reorder(type_id, ordered_ids)`, оптимистичное обновление с откатом при ошибке (в `PlaceTree` отката нет — не копировать); (6) ghost — по образцу `.drag-ghost` (токены `--tr-surface-raised`, `--tr-elev-3`).

**Выносить ли примитив в общий компонент сейчас?** Нет — **вынести только чистые функции `reorder.ts` в `ui/src/lib/utils/`, остальное оставить локально.** Причины: 2D hit-testing карты (Фаза 44) отличается от одномерной вставки; первый реальный второй DOM-потребитель — Фаза 44; рабочий inline-образец уже есть в `PlaceTree`. Чистые функции дёшево проверить `node`-гейтом (фикстура: порядок и граничные индексы), в отличие от жеста.

**⚠ Компиляционные гейты слепы к рантайму рун и к жесту.** `svelte-check`, `pnpm lint`, `pnpm build` зелёные при `effect_update_depth_exceeded` (самоинвалидирующийся `$effect`, прецеденты quick 260820-rdj и Phase 40-32) и ничего не скажут о том, что drop не срабатывает в WKWebView (GAP-2). Обязательны **живые проверки в запущенном приложении на ОБОИХ транспортах** (Tauri-десктоп из `cargo tauri dev` с main + LAN-браузер после `pnpm --dir ui build` — серверный режим отдаёт устаревший `ui/dist`): перетаскивание, ghost, отмена Esc/`pointercancel`, клавиатурный путь, а также любой новый `$effect`, пишущий `$state`, который сам же читает (запись — в `untrack`). Одноразовый Vite-харнесс уместен только для вопросов «рантайм фреймворка» (`effect_update_depth_exceeded` от движка не зависит), но не для жеста и вёрстки (WKWebView ≠ Chromium).

### Pattern 12: UI — карта переиспользования [VERIFIED: чтение компонентов]

| Нужно | Брать | Жёсткие ограничения |
|-------|-------|---------------------|
| Сетка 35%/65% | **импортировать** `features/places/PlacesMasterDetail.svelte` (не копировать, не пересчитывать — запрет 39-UI-SPEC; значения скопированы из `RequestsMasterDetail`) | |
| Дерево | **новые** `GroupTree.svelte` + `GroupTreeNode.svelte` | Скопировать анатомию `PlaceTreeNode` (32 px строка, chevron, `role="treeitem"`, `aria-level/selected/expanded`, клавиатура на контейнере `role="tree"` по плоскому `visibleNodes`); `PlaceTree` не обобщён. Узлы двух видов: «тип» (корень) и «группа» |
| Выбор «тип/группа» → правая панель | `DetailPanel`/`DetailSection`/`DetailField` + `Tabs` | Корневой `*-page`: `height:100%; min-height:0; flex column` + внутренний скролл-регион; оболочка `.content` не скроллится (память `app_shell_scroll_pattern`) |
| Любой выбор из списка | `Dropdown` (`variant="select"`, `flat`) | **Никогда** нативный `<select>` и не `Select`-обёртка (память: фазы 27/28, повторная коррекция) |
| Таблица свойств / состава | `Table` + `TableRow` | Фокус таблицы — только inset-кольцо на первой ячейке (память `table_focus_ring_decision`); столбцы состава — копия `PlaceContents`: «Тип» (на вкладке «Все»), «Название», «Инв. № / Серийный №», «Место», «Статус» |
| Пустой состав | `Table` с `empty` + `emptyTitle`/`emptyBody` | НЕ `Placeholder.svelte` (расхождение №2) |
| Раскрываемая строка вложенной группы (D-03, D-25) | `TableRow` в режиме `group` (`groupExpanded`, `onToggleGroup`) | `aria-label` шеврона в `TableRow` сейчас «Свернуть группу/Развернуть группу» (`TableRow.svelte:61`) — это ровно та терминология, которую запрещает GRD-06 → заменить на нейтральное «Свернуть/Развернуть» |
| Меню строки (⋯) | `ActionMenu` (`variant="ghost-sm"`) | |
| Добавление в состав (D-01/D-04) | `Dropdown` combobox + `devices.search`/`listGrouped` из `ActFormItemsTable` | В акте стоит `status_id: 1` («на складе») — **для состава группы фильтр по статусу не нужен** (АРМ в работе); заимствованный код нельзя брать буквально. Пометка «уже в группе «…»» — пачкой `groups_for_devices(ids)` по результату поиска. Секции «Устройства/Группы» — аддитивная правка `Dropdown` (расхождение №3) |
| Мультивыбор «Добавить несколько…» | `Modal` + фильтры + `Checkbox` | тот же серверный `groups_add_devices` |
| Попап нарушителей `is_required` (D-14) | свой презентационный попап по образцу `NumberTakenPopup` (всё через props) | данные — из `group_type_properties_empty_groups(property_id)` после отказа сервера |
| «Перенести» (D-17/D-18) | `Modal` по образцу блока массового переноса в `PlaceContents.svelte` (ближе, чем `PlaceMoveModal`, который переносит узел дерева) + `PlacePicker` | подтверждение «переедет N устройств»; один обработчик из панели и из меню узла |
| Чипсы пользователей (D-11) | `Badge`/чип + кнопка-звёздочка + `Dropdown` добавления | пользователей отдаёт `groups_user_options` (не `users_list`) |
| Блокировка места в форме устройства (D-19) | `PlacePicker disabled` (проп уже есть) + текст и ссылка | данные — `groups_for_devices([id])` при открытии формы редактирования, без расширения `DeviceDto` |
| Тост (D-24) | `pushToast('success', …)` | текст из ответа `groups_move` («группа и N устройств»); `ru_plural` сейчас приватна в `place_service.rs` — вынести в общее место или продублировать осознанно |
| Журнал группы (D-29) | `MovementTimeline` (4-й потребитель) | форк запрещён |
| Свёртка пакета в отчёте (D-25/D-26) | `TableRow group` в `ReportTable.svelte` | печать — серверный HTML; сворачивать только экран |
| Сайдбар | `sidebar-config.ts`: вставить `{ kind:'item', route:'/groups', label:'Группы', roles:['admin','manager'] }` между `/devices` и `/acts`; обновить пометку **PINNED: 13 items + 4 dividers = 17 entries** | Сейчас 12+4=16 [VERIFIED пересчётом]. Комментарий над массивом про позиции разделителей уже устарел — переписать |
| Маршрут | `routes.ts`: добавить `'/groups'` в `routes`, **не** добавлять в `employeeRoutes` — `'*' → AccessDenied` даёт «Доступ запрещён» под `employee` без правок | серверный 403 проверяется матрицей отдельно |
| Сохранение выбора/раскрытия | `localStorage` с префиксом `trackly:groups:*` по образцу `PlacesPage` | URL-хеш `#/groups?id=…` побеждает (переход-фокус D-07/D-19) |

**INV-7 (D-08) — как расширять реестр.** `ui/scripts/check-place-tree-invalidation.mjs` — реестровый гейт клиентских producer’ов `notifyPlaceContentChanged(`: `MUTATING_COMPONENTS` (компонент + имя success-пропа), `FORWARDING_COMPONENTS`, `DIRECT_CALL_MARKERS` (литеральная подстрока вызова в исходнике компонента; сейчас `'places_move_subtree_contents'` и `acts.delete(`). Новые producer’ы, меняющие `place_id`: `groups_move`, `groups_add_devices` (устройство получает место группы), `groups_set_parent` (вложение двигает состав), плюс уже зарегистрированный `places_move_subtree_contents` (теперь двигает ещё и группу). Маркер **должен совпадать со стилем вызова в компоненте**: если компоненты зовут `groups.move(…)` через обёртку `lib/api/groups.ts`, то маркер — `'groups.move('`, а не имя команды (как `acts.delete(`). Серверные ответы этих трёх команд обязаны нести `changed_place_ids: Vec<i64>` (старые + новые места, как `ActDto::changed_place_ids`) — по ним UI вызывает `notifyPlaceContentChanged`. `groups_remove_devices` и `groups_delete` место не меняют — в реестр не входят, но **счётчики «Мест» не затрагивают** (в 41 группы в «Местах» не показываются — это 41.2).

### Pattern 14: Переименование «группа» → «Свернуть одинаковые» и гейт [VERIFIED: grep по ui/src, crates, templates]

**Пользовательские строки, где свёртка названа группой (исчерпывающий grep по `[Гг]руп|[Сс]груп|[Сс]вернуть|[Рр]азвернуть` в `ui/src`, кроме `bindings.ts`):**

| Файл:строка | Сейчас | Действие |
|-------------|--------|----------|
| `features/devices/DeviceFilters.svelte:102` | «Группировать похожие» | → «Свернуть одинаковые» (в SPEC названа «Группировать одинаковые» — фактическая строка иная) |
| `features/devices/DeviceList.svelte:99` | «Групп: {groups.length}» | переформулировать (напр. «Строк: N» / «Свёрнуто: N») — формулировка в дискреции планировщика |
| `lib/components/TableRow.svelte:61` | aria-label «Свернуть группу»/«Развернуть группу» | → «Свернуть»/«Развернуть» (компонент общий, будет у состава и отчёта) |
| `features/showcase/sections/TableSection.svelte:209` | «Строка-группа» | переименовать (напр. «Раскрываемая строка») — витрина админская, но гейт должен быть чистым |
| `features/showcase/sections/DropdownSection.svelte:109` | «Комбобокс с группами (drill-in)» | переименовать («… со свёрнутыми наборами (drill-in)») |
| `bindings.ts` | комментарии про «группу одинаковых устройств» | генерируется из Rust doc-комментариев → править комментарии в `dto/device.rs`/портах, не руками в `bindings.ts` (файл в `.gitignore`); пользователю не виден |

В Rust-литералах и `templates/*.html` слова «групп» в пользовательских строках нет [VERIFIED: grep]. Остальное в `ActFormItemsTable.svelte`/`DeviceGroupRow.svelte` — комментарии и идентификаторы (`group`, `DeviceGroup`); переименовывать идентификаторы НЕ нужно (риск без выгоды).

**Гейт (`ui/scripts/check-group-vocabulary.mjs` + `--selftest`, подключить в `pnpm lint`):** по образцу `check-device-form-quantity-gate.mjs`/`check-place-tree-invalidation.mjs` (zero-dependency, `stripComments`, самотест на копии дерева). Правило: в `ui/src/**/*.{svelte,ts}` вне allowlist (`features/groups/`, `lib/api/groups.ts`, и явные пары файл+маркер для легитимных надписей новой сущности — «Место задаётся группой…» в `DeviceFormBody.svelte`, пункт «Добавить в группу» из 41.1) пользовательские литералы (строки в кавычках, текст между тегами, `aria-label`/`title`) не содержат корней `[Гг]руп`/`[Сс]груп`. Selftest: на подложенной копии с «Группировать» гейт обязан упасть; без неё — пройти. Сравнение с allowlist, а не «ровно 0», чтобы D-19 и 41.1 не требовали ослабления гейта. Отдельного JS-тест-раннера в проекте нет (vitest/jest отсутствуют в `ui/package.json`) — этот гейт и есть автоматическая проверка SPEC req.14 («снимок/тест раздела «Устройства»»); раннер в 41 не вводить.

### Anti-Patterns to Avoid

- **Копировать идиом `PlaceService` «repo в автокоммите + отдельная tx для аудита» в групповые мутации.** Сбой между ними оставляет мутацию без аудита; для переноса — частично перенесённую группу. Одна `conn.transaction()` на всё замыкание.
- **Разбирать имя группы обратно в номер** («АРМ #3» → 3). Запрещено принципом фаз 40.2/40.5; `seq` — целочисленная колонка, имя только отображение.
- **Вычислять «обязательное пустое» по значению `''`.** Пустое значение не хранится — проверять отсутствие строки.
- **Держать у вложенной группы собственное редактируемое место.** Место вложенной — копия корневой, обновляется только `move_group_in_tx`/`set_parent`; API места не принимает.
- **Менять `columns_for("movements")`/`templates/report.html` ради пакета** — тянет `check-report-type-parity.mjs`, CSV и ловушку шаблонов; пакет кодируется полями строки.
- **Класть логику запрета/протаскивания в UI или в `devices_sqlite.rs` как скрытый побочный эффект `update_status_and_place_in_tx`.** Этот метод используют акты и массовый перенос с РАЗНЫМ желаемым поведением — явные helper’ы на сайте вызова.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Валидация/нормализация IP | свой разбор октетов/IPv6 | `std::net::IpAddr::from_str` + `to_string()` | Проверено: каноническое сжатие IPv6, отказ лидирующих нулей и `%zone`, `/mask`; свои регэкспы ошибаются на IPv6 |
| Идентификатор пакета | счётчик/MAX+1 в журнале | `uuid::Uuid::new_v4()` в сервисе | Не требует блокировки и работает для пакета без строки группы |
| Защита от циклов вложенности | обход в Rust по одному `SELECT` | рекурсивный CTE из `places_sqlite.rs::move_node` | Один запрос внутри той же транзакции, обкатан в фазе 39 |
| Подсчёт состава и вложенных групп | рекурсия в Rust N+1 | CTE `sub` (форма `subtree_stats_impl`) | 5000 устройств / 100–150 на место — один запрос |
| Сокращение путей мест в журнале | JS/Rust-копия формулы | `compute_place_path_short_with_conn` (единственный владелец) | WR-03/WR-08 фазы 39.2: копии расходились |
| Уникальность `code`, `(type_id,seq)`, «одна группа на устройство» | проверка только в сервисе | уникальный индекс / PK + дружелюбное сообщение в сервисе | Единственный writer не отменяет бэкстоп БД (V043/V044 прецеденты) |
| Поиск устройств для состава | новый поиск | `devices.search`/`listGrouped` (FTS5, drill-in в `Dropdown`) | Уже решено в `ActFormItemsTable` |
| Свёртка пакета для печати | сворачивание в HTML-шаблоне | плоские строки из `query_movements_inner` | D-27: печать — полный состав без логики |
| Склонение «устройство/устройства/устройств» | ручной `if` | `ru_plural` из `place_service.rs` (вынести) | Уже есть и протестировано |

**Key insight:** почти вся фаза — применение уже обкатанных идиом проекта к новой сущности; единственный по-настоящему новый механизм (вертикальный pointer-reorder) имеет прямой прецедент в `PlaceTree`.

## Runtime State Inventory

> Фаза не рефакторинг, но включает аддитивную миграцию БД и переименование пользовательских строк; инвентарь заполнен явно.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | Переименование затрагивает только литералы интерфейса — строк-ключей в БД нет. Новые таблицы пусты; «Миграция существующих устройств в группы — не требуется, БД обнуляется» (SPEC). На БД без обнуления V045/V046 применяются аддитивно (проверяется тестом `run_up_to(44)`) | Code edit + тест миграции; data migration не нужна |
| Live service config | None — внешних сервисов с этим словом/сущностью нет (проверено: приложение автономно, AD/SNMP не затронуты) | none |
| OS-registered state | None — планировщиков/служб с именами групп нет | none |
| Secrets/env vars | None новых. Для тестов `trackly-app` нужны `TRACKLY_AD_MOCK`/`TRACKLY_SNMP_MOCK` (уже существуют) | none |
| Build artifacts | `ui/src/bindings.ts` — **генерируемый и в `.gitignore`**: после добавления DTO/команд пересоздать (`cargo test -p trackly-app --test export_bindings`, он же `prebuild`); `ui/dist` устаревает для LAN-режима → `pnpm --dir ui build` перед живой проверкой в браузере. Копии репозитория в `.kilo/worktrees/` и `.claude/worktrees/` содержат те же файлы (исключены из git через `.git/info/exclude`) — любые grep-гейты привязывать к `ui/src`, а не к корню | пересборка перед UAT |
| localStorage (клиент) | Существующие ключи `trackly:places:*` не меняются; новые — `trackly:groups:*` | none |

## Common Pitfalls

### Pitfall 1: Guard запрета отклоняет любое сохранение члена группы
**What goes wrong:** `DeviceFormBody` шлёт `place_id` при каждом сохранении (`:615/:684`). Наивное «`patch.place_id.is_some()` → отказ» ломает переименование/правку статуса устройства-члена.
**How to avoid:** сравнивать `patch.place_id` с текущим `devices.place_id` внутри writer-замыкания, отказывать только при реальном отличии; тест «повторная отправка того же места проходит» (фикстура обязана РАЗЛИЧАТЬ: до-фикс и после-фикс дают разный исход, урок `vacuous_test_fixture_must_diverge`).
**Warning signs:** в UAT нельзя сохранить правку устройства из АРМ.

### Pitfall 2: Двойные строки журнала при массовом переносе (D-23)
**What goes wrong:** `move_subtree_contents` обходит `list_subtree_contents` (в нём уже есть устройства-члены) и пишет `manual`-строки; затем перенос группы пишет свои → 2 строки на устройство.
**How to avoid:** собрать корневые группы заранее, исключить их устройства из цикла, переносить через `move_group_in_tx` в той же внешней `tx`. Тест: подсчёт строк журнала по `entity_id`.

### Pitfall 3: Картриджи принтера не едут за принтером-членом группы
**What goes wrong:** `DeviceService::update` каскадит место на прикреплённые картриджи (`cascade_place_for_printer_in_tx`); групповой перенос по умолчанию — нет → картридж остаётся на старом месте.
**How to avoid:** вызывать каскад для каждого перенесённого принтера, в той же транзакции; решение для строк журнала картриджей — класть в тот же пакет (`source='group'`, тот же `batch_id`), иначе в отчёте появятся «осиротевшие» строки «вместе с принтером» (приёмка «6 устройств → 7 строк» проверять на составе БЕЗ принтеров с картриджами; кейс с картриджами — отдельный тест). Каскад вызывать только при `new_place_id IS NOT NULL` (контракт CR-03 функции).

### Pitfall 4: Дрейф денормализованного `groups.place_id`
**What goes wrong:** место вложенной группы — копия корневого; если путь вложения/выхода (`set_parent`, удаление родителя `ON DELETE SET NULL`) его не обновит, guard запрета и карточки врут.
**How to avoid:** все операции, меняющие `parent_group_id`, идут через один `*_in_tx` с протаскиванием; тест-инвариант «для каждой вложенной группы место = месту корня» после каждой операции (вложение, выход из родителя, удаление родителя, перенос корня). Правила: вложение под родителя с местом → группа и состав берут место родителя (с пакетом журнала); вложение под родителя БЕЗ места → место группы становится NULL, состав сохраняет свои места («запрет спит»); выход из родителя → группа остаётся с тем же местом корня; удаление родителя → вложенные становятся корнями с тем же местом.

### Pitfall 5: `places_delete` блокируется сырой FK-ошибкой
**What goes wrong:** `groups.place_id … ON DELETE RESTRICT`; пустая группа на месте не попадает в `subtree_stats_impl` (считает только устройства/картриджи/акты/перемещения) → предпроверка «можно удалить», реальный `DELETE` падает. Тот же класс дефекта, что BLOCKER-1 аудита 40 (`referencing_movement_count`).
**How to avoid:** добавить `referencing_group_count` в `SubtreeStats` (core + DTO + `build_delete_blocked_message` + UI-сообщение), либо ловить FK в `map_place_delete_error` (он уже превращает FK в русский `Conflict`). Тест: удаление места с пустой группой → контролируемое русское сообщение.

### Pitfall 6: Пакет в отчёте без строки группы и «осиротевшие» члены
**What goes wrong:** (а) первая установка места группе (`NULL → X`) строки группы не пишет (`is_reportable_place_change`), но члены с местами пишут `Some→Some` строки; (б) фильтр «тип устройства» отсекает строку группы; (в) `LIMIT 1000` режет пакет; (г) удалённая/переименованная группа.
**How to avoid:** `entity_label` пишется в **каждую** строку пакета; заголовок пакета в UI строится из строки группы, а при её отсутствии — из `entity_label` + счётчика по `batch_id`; строки с `batch_id` без заголовка рисуются как обычные; `batch_size` считается подзапросом.

### Pitfall 7: Меняется сигнатура `record_movement_if_applicable`
**What goes wrong:** 7 существующих write-site’ов (устройство, картридж, акты, массовый перенос) + их тесты ломаются; велик соблазн «добавить ещё два аргумента».
**How to avoid:** соседний метод + расширение `NewMovement`; существующие вызовы не трогать.

### Pitfall 8: Отчёт показывает сырой «group» и пустое имя
**What goes wrong:** `CASE pm.entity_type … ELSE pm.entity_type`, `COALESCE(d.name, c.code)` не знают группу; `movement_reason` отдаёт сырой токен.
**How to avoid:** три правки из Pattern 10 п.5 + тест отчёта с группой (строка группы: «Группа», имя из `entity_label`).

### Pitfall 9: SQL `CHECK` на `behavior`/`data_type` блокирует будущее
**What goes wrong:** GRPX-02 добавит `enum`/`list`/`file`, а `CHECK (data_type IN (…))` потребует перестройки таблицы — ровно тот случай, из-за которого V042 стёр `act_items`.
**How to avoid:** без `CHECK` (как V040 для `entity_type`/`source`), валидация строгая на записи, мягкая на чтении (`from_str_lenient`, сырой токен вместо падения экрана — урок IN-01). Закрытость `behavior` обеспечивает код + триггер неизменяемости.

### Pitfall 10: Вакуумный тест неизменяемости `code`/`behavior` по HTTP
**What goes wrong:** если DTO правки не содержит поля `code`, serde молча игнорирует `{"code":"x"}` и «отклонение» никогда не проверяется — тест зелёный при снятой проверке.
**How to avoid:** DTO несёт опциональные `code`/`behavior` и сервис отклоняет отличие; тест шлёт ОТЛИЧАЮЩЕЕСЯ значение, проверяет ошибку И что строка в БД не изменилась; мутация — вырезать проверку (якорь **уникальный**, `assert s.count(old) == 1`), плюс отдельный тест на триггер БД (`UPDATE group_types SET code=…` напрямую → `Conflict`).

### Pitfall 11: Засев, который перезаписывает переименование
**What goes wrong:** `INSERT OR REPLACE`/`ON CONFLICT … DO UPDATE` вернут «АРМ» и сменят `id`.
**How to avoid:** `ON CONFLICT(code) DO NOTHING`; тест переименовывает в имя, **отличное** от сидового, после повторного засева проверяет `COUNT(*)=1`, имя и `id` прежние; мутация `DO NOTHING → DO UPDATE SET name = excluded.name` должна покраснить. Также тест полного рестарта (`AppCtx::build` дважды на одном каталоге, как `session_survives_restart.rs`).

### Pitfall 12: Устройство удалили — оно осталось в составе
**How to avoid:** S8 (удалить членство в `delete_soft`) + фильтр `d.deleted_at_utc IS NULL` в запросах состава/принтеров/счётчиков.

### Pitfall 13: Поиск пользователей по `LIKE` не находит кириллицу в другом регистре
**How to avoid:** `groups_user_options` фильтрует в Rust (`to_lowercase().contains`), образец `PlaceService::search`; лимит выдачи (≈50).

### Pitfall 14: Аддитивная правка `Dropdown` ломает существующих потребителей
**How to avoid:** новый проп опционален, без него разметка/ARIA идентичны; проверить витриной и живым прогоном формы акта и пикера принтера (там `Dropdown` в проде).

### Pitfall 15: `effect_update_depth_exceeded` в новых экранах
**How to avoid:** `$effect`, который читает `$state` и пишет его же (кэши, выбранный узел, развёрнутые узлы), — запись в `untrack`; живой прогон экрана «Группы» на обоих транспортах (компиляционные гейты слепы). Дерево с загрузкой по выбору — держать «загрузку» отдельным состоянием, не перезаписывающим входную зависимость.

### Pitfall 16: Рабочие дерево/сборка расходятся с запущенным приложением
**How to avoid:** LAN-режим отдаёт `ui/dist` → `pnpm --dir ui build` перед живой проверкой; исправления из spawned-worktree не видны в `cargo tauri dev` до слияния в main.

### Pitfall 17: Номер `seq` повторно используется после удаления последней группы
**What goes wrong:** `MAX(seq)+1` после удаления максимума выдаёт тот же номер; подпись «АРМ #3» в журнале (снимок `entity_label`) станет двусмысленной.
**Decision needed:** SPEC фиксирует `MAX(seq)+1` буквально — реализовать так; зафиксировать как принятое. Альтернатива (мягкое удаление групп + `MAX` по всем строкам) конфликтует с `GRP-10` («удаление освобождает» — hard) и не рекомендуется. [ASSUMED] см. A3.

### Pitfall 18: Разархивация свойства с именем, уже занятым живым
**How to avoid:** частичный уникальный индекс `(type_id, name) WHERE archived_at_utc IS NULL` даст `Conflict` при `unarchive` — перехватить и показать русское сообщение «Свойство с таким названием уже есть».

### Pitfall 19: Жёсткие лимиты на пакетные входы
**How to avoid:** `groups_add_devices` — потолок числа id (напр. 500), `groups_set_values` — потолок записей на свойство (напр. 100), длина имени ≤ 200, текст ≤ 2000; иначе один запрос блокирует единственный writer (очередь 256, `send_timeout` 5 с → `WriteQueueBusy` у остальных).

### Pitfall 20: Гонки «добавил в группу» ‖ «сменил место в форме»
**Why it can't happen:** единственный writer сериализует; guard (S1) читает членство внутри того же замыкания. Бэкстоп — PK `group_devices.device_id`.

### Pitfall 21: Запуск тестов
**What goes wrong:** полный `cargo test --workspace` ≈ 80 мин (~1372 тестов); два одновременных `cargo test` конфликтуют; `login_remember_persistent_cookie` (`tests/auth_remember_cookie.rs`) зависает; после обновления Xcode линковка `trackly-app` падает с exit 69.
**How to avoid:** скоупить по целям; полный прогон — один раз на границе волны, в фоне; для пакета целиком — `-- --skip login_remember_persistent_cookie`; префикс `DEVELOPER_DIR=/Library/Developer/CommandLineTools` при exit 69; `cargo fmt --check` имеет заранее существующий дрейф (запускать только для изменённых файлов или `cargo fmt -- --check <file>`). Подробнее — §Validation Architecture.

## Code Examples

### Нормализация MAC и числа (чистый домен, без crate) — эскиз
```rust
// trackly-core/src/domain/group_values.rs
pub fn normalize_mac(raw: &str) -> Result<String, ValueError> {
    let s = raw.trim();
    let sep = if s.contains(':') { Some(':') } else if s.contains('-') { Some('-') } else { None };
    if s.contains(':') && s.contains('-') { return Err(ValueError::Mac); }          // смешанные разделители
    let hex: String = match sep {
        Some(c) => {
            let parts: Vec<&str> = s.split(c).collect();
            if parts.len() != 6 || parts.iter().any(|p| p.len() != 2) { return Err(ValueError::Mac); }
            parts.concat()
        }
        None => s.to_string(),
    };
    if hex.len() != 12 || !hex.chars().all(|c| c.is_ascii_hexdigit()) { return Err(ValueError::Mac); }
    let lower = hex.to_ascii_lowercase();
    Ok((0..6).map(|i| &lower[i * 2..i * 2 + 2]).collect::<Vec<_>>().join(":"))
}

pub fn normalize_number(raw: &str) -> Result<String, ValueError> {
    let s = raw.trim().replace(',', ".");
    if s.is_empty() || s.len() > 32 { return Err(ValueError::Number); }
    match s.parse::<f64>() {
        Ok(v) if v.is_finite() => Ok(s.trim_start_matches('+').to_string()),
        _ => Err(ValueError::Number),
    }
}
```
Таблица-тест на каждый тип: валидные (`aa:bb:cc:dd:ee:ff`, `AA-BB-CC-DD-EE-FF`, `aabbccddeeff` → один вид), невалидные (`aa:bb-cc:dd:ee:ff`, `aabbccddeef`, `zz:…`, `1,5`→`1.5`, `abc`, `NaN`, `inf`, `""`).

### Guard запрета (S1) — внутри writer-замыкания `DeviceService::update`
```rust
// после let before = repo.get_in_tx(&tx, id) и ДО repo.update_in_tx(...)
if let Some(new_place) = domain_patch.place_id /* Option<Option<i64>> */ {
    if new_place != before_place_id {                       // реальная смена, а не повтор текущего
        if let Some(g) = group_repo.locked_group_for_device_in_tx(&tx, id)? {   // группа с place_id IS NOT NULL
            return Err(AppError::Validation {
                field: "place_id".into(),
                message: format!("Место задаётся группой «{}». Выведите устройство из состава, чтобы переместить его отдельно.", g.name),
            });
        }
    }
}
```

### Нарушители `is_required` и «заполнено в N группах»
```sql
-- нарушители (имена — в сообщение Validation, id — в group_type_properties_empty_groups)
SELECT g.id, g.name FROM groups g
WHERE g.type_id = ?1
  AND NOT EXISTS (SELECT 1 FROM group_property_values v
                  WHERE v.group_id = g.id AND v.property_id = ?2
                    AND (v.value_ref IS NOT NULL OR (v.value_text IS NOT NULL AND v.value_text <> '')))
ORDER BY g.seq;
-- скрыть вместо удаления / запрет смены data_type / D-16
SELECT COUNT(DISTINCT group_id) FROM group_property_values WHERE property_id = ?1;   -- >0 → скрыть, не удалять
```

### Следующий `seq` и имя по умолчанию
```sql
SELECT COALESCE(MAX(seq), 0) + 1 FROM groups WHERE type_id = ?1;   -- внутри writer-транзакции; индекс (type_id, seq) — бэкстоп
```
Имя: `format!("{} #{}", type_name, seq)` в сервисе; обратного разбора нет.

### Фикстура атомарности переноса (R9) — сбой на середине
```sql
-- в тесте: после создания группы из 6 устройств (2 во вложенной) и ПЕРЕД groups_move
CREATE TRIGGER t_boom BEFORE UPDATE OF place_id ON devices
  WHEN NEW.id = <id четвёртого устройства> BEGIN SELECT RAISE(ABORT, 'boom'); END;
```
После ошибки: `groups.place_id` корневой и вложенной, все 6 `devices.place_id`, `COUNT(*) FROM place_movements`, `COUNT(*) FROM audit_log` — **как до вызова**. Мутация «автокоммит вместо `tx`» должна покраснить этот тест.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| HTML5 drag-and-drop (`draggable`/`dataTransfer`) | Pointer Events + `setPointerCapture` | Фаза 39, 2026-08-25 (UAT GAP-2) | WKWebView/WebView2/браузер ведут себя одинаково; призрак рисуется вручную |
| Контекстное «группа» = свёртка одинаковых | «группа» = новая сущность; свёртка — «Свернуть одинаковые» | Эта фаза | Один смысл слова в UI |
| `place_movements` — строка на сущность без корреляции | `batch_id` + `entity_label` | V046 | Перенос читается как событие |

**Deprecated/outdated:** `MovementSource::Workstation` — зарезервирован V040, не пишется; фаза вводит `Group`, токен оставить мёртвым.

## Assumptions Log

> Все пункты — решения/допущения, которых НЕТ в SPEC/CONTEXT и которые не проверены у пользователя; планировщик/discuss-phase должен подтвердить до превращения в зафиксированное решение.

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `batch_id` — TEXT (UUID v4, генерируется сервисом) и дополнительная колонка `entity_label` (снимок имени группы) в V046; SPEC требует только `batch_id` | Pattern 2/10 | Если пользователь хочет только `batch_id`, отчёт удалённой группы потеряет имя; лишняя колонка безвредна |
| A2 | У вложенных групп `groups.place_id` хранится денормализованно (копия корневого), API места вложенной — только чтение | Pattern 4 | Дрейф инварианта (Pitfall 4); альтернатива — вычислять по цепочке (медленнее на горячем пути guard’а) |
| A3 | `seq = MAX(seq)+1` повторно использует номер после удаления последней группы типа; принято буквально по SPEC | Pitfall 17 | Двусмысленные подписи «АРМ #N» в журнале; смягчает `entity_label` |
| A4 | Вложенная группа в дереве показывается только под родителем (не дублируется под корнем своего типа); счётчик типа считает все его группы | Pattern 12 | UX-расхождение; тип «Системный блок» не покажет вложенные блоки без раскрытия родителя |
| A5 | Вложение под родителя без места обнуляет место вложенной группы, состав сохраняет свои места | Pitfall 4 | Альтернатива — запретить такое вложение; влияет на сценарий «собрать сейчас» |
| A6 | Строки журнала картриджей, едущих за принтером-членом, входят в тот же пакет (`source='group'`) | Pitfall 3 | Без этого в отчёте появятся одиночные строки «вместе с принтером» |
| A7 | Событие D-30 пишется в `audit_log` с `entity_type:"device"`, `action:"custom:group_place_assigned"` | Pattern 10 п.7 | Для 41.5 удобнее читать историю устройства из одного `entity_type`; альтернатива — `entity_type:"group"` |
| A8 | Ограничение вложенности — только «родитель с `behavior='teardown'` отклоняется» (SPEC буквально); правило для `substitute`-родителя (брифинг: «контейнерные могут содержать контейнерные и якорные») не вводится в 41 | Pattern 4 | Если нужно строже — это правило 41.1 (якорь) |
| A9 | «Живой пользователь» = `users.deleted_at_utc IS NULL AND is_active = 1` | Pattern 8 | Заблокированный пользователь окажется выбираемым/невыбираемым |
| A10 | Обязательность проверяется при `groups_create` (DTO несёт `values`) и `groups_set_values` | Pattern 8 | Иначе форма создания обходит обязательные поля |
| A11 | Лимиты: имя ≤ 200, текст ≤ 2000, `number` ≤ 32 символов, `groups_add_devices` ≤ 500, ссылок на свойство ≤ 100 | Pitfall 19 | Числа выбраны без согласования |
| A12 | Попап нарушителей получает данные отдельной командой `group_type_properties_empty_groups`, а отказ сервера — обычный `Validation` с именами групп в сообщении (а не `*SaveOutcome`-вариант) | Pattern 6 | Альтернатива `Outcome::Blocked{groups}` (прецедент `DeviceSaveOutcome`) строже типизирована, но SPEC говорит «отклоняется» |
| A13 | Нет `CHECK` на `behavior`/`data_type` (валидация в Rust + триггер неизменяемости) | Pitfall 9 | SPEC молчит; `CHECK` приемлем, но сделает GRPX-02 перестройкой таблицы |
| A14 | Живая работоспособность pointer-перестановки на `<tbody>` в WKWebView будет такой же, как в `PlaceTree` (контейнер `div`) | Pattern 11 | Требует живого чека (D-10) |
| A15 | Принтеры вложенных групп входят в «подключённые принтеры» группы-корня | Pattern 9 | SPEC: «устройств состава» — вложенные включены по CONTEXT (состав = с вложенными) |

## Open Questions

1. **Где в дереве показывать вложенную группу другого типа?**
   - Known: корни дерева — типы, внутри — группы (SPEC req.6); вложенность — между группами разных типов (АРМ ⊃ Системный блок).
   - Unclear: под типом своей собственной принадлежности, под родителем, или в обоих местах.
   - Recommendation: только под родителем (A4), поиск по дереву — плоский.

2. **Что делает «Перенести» при очистке места?**
   - Known: D-17 — отдельное действие «Перенести» с подтверждением.
   - Unclear: нужна ли возможность вернуть группе «без места».
   - Recommendation: `groups_move(target_place_id: i64)` — не nullable; очистки места нет (меньше сценариев «запрет спит»).

3. **Форма создания группы и обязательные свойства.** Создание из дерева («Создать группу в типе») — открывает ли модалку с полями свойств типа (иначе обязательное свойство нарушается при создании)? Рекомендация: модалка с именем (по умолчанию «{тип} #{seq}»), полями свойств и необязательным `PlacePicker` (A10).

4. **`DeviceDto` и членство.** Нужна ли пометка «в группе» в списке «Устройства» уже в 41? Рекомендация: нет (это GRD-01/41.1); в 41 хватает `groups_for_devices`.

5. **Счёт `places_move_subtree_contents`.** Возвращаемое `usize` сейчас — число элементов содержимого; после D-23 группы входят туда же устройствами. Рекомендация: не менять тип (bindings/UI), тост D-24 строить из ответа `groups_move` для явного переноса группы.

6. **Строгость вложенности `substitute` ↔ `container`.** Отложить до 41.1 (A8)?

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (cargo/rustc) | сборка, тесты | ✓ | rustc 1.95.0 (`rust-version = 1.92`) | — |
| Node.js | `pnpm lint`-гейты, `node scripts/check-*.mjs` | ✓ | v24.21.0 (`engines: >=20`) | — |
| pnpm | сборка UI | ✓ | 10.17.1 | — |
| `sqlite3` CLI | только справочно (проверка DDL); тесты используют bundled SQLite | ✓ | 3.51.0 | — |
| `ui/dist` | тесты `trackly-app` + LAN-режим | ✓ (устаревший) | — | `pnpm --dir ui build` (запускает `prebuild` = `cargo test -p trackly-app --test export_bindings`) |
| Windows-машина | живой UAT WebView2 | ✗ (отдельная машина пользователя) | — | UAT на macOS-WKWebView + LAN-браузер; Windows — на этапе проверки пользователем |
| AD/целевые принтеры | не нужны | ✗ | — | `TRACKLY_AD_MOCK`, `TRACKLY_SNMP_MOCK` |
| `gsd-sdk` | коммит артефактов | ✓ | — | — |

**Missing dependencies with no fallback:** нет.
**Missing dependencies with fallback:** Windows-проверка — человеком, вне автоматизации.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust: `cargo test` (libtest; CI — `--workspace --no-fail-fast -- --test-threads=1`); UI: `svelte-check`, `eslint`+`prettier`, zero-dependency `node ui/scripts/check-*.mjs` (в цепочке `pnpm lint`). JS-тест-раннера в проекте нет — не вводить |
| Config file | `/Cargo.toml` (workspace), `ui/package.json` (`lint`), `clippy.toml` |
| Quick run command | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test <файл>` (одна цель), `cargo test -p trackly-infra --test <файл>`, `cargo test -p trackly-core <фильтр>`, `node ui/scripts/check-group-vocabulary.mjs` |
| Full suite command | `cargo test --workspace --no-fail-fast -- --test-threads=1` (≈80 мин, ~1372 тестов) + `pnpm --dir ui svelte-check && pnpm --dir ui lint` + `cargo clippy --workspace --all-targets -- -D warnings` |

**Ограничения запуска (передать планировщику):** (1) один `cargo test` одновременно; (2) полный прогон — один раз на границе волны, в фоне, остальное — по целям `--test`; (3) тестам `trackly-app` нужны `TRACKLY_AD_MOCK`/`TRACKLY_SNMP_MOCK` и **настоящий** `ui/dist` (`pnpm --dir ui build`); (4) при запуске пакета `trackly-app` целиком — `-- --skip login_remember_persistent_cookie` (зависает; заранее известный дефект); (5) `cargo fmt --check` имеет существующий дрейф — для чистоты фазы проверять изменённые файлы; (6) при exit 69 линковки — `DEVELOPER_DIR=/Library/Developer/CommandLineTools`; (7) перед `svelte-check`/`lint` сгенерировать `bindings.ts`: `cargo test -p trackly-app --test export_bindings`; (8) CI — один последовательный job: первый красный шаг скипает следующие гейты, поэтому прогонять шаги в порядке CI.

### Phase Requirements → Test Map

| Req (SPEC) | Behavior | Test Type / Layer | Automated Command | File Exists? |
|------------|----------|-------------------|-------------------|--------------|
| 1 | `code`/`behavior` неизменяемы на обоих транспортах; rename проходит; неизвестный `behavior` отклонён | infra (триггер БД: прямой `UPDATE` → `Conflict`) + app service + dual-transport (`build_group_types_update` и HTTP с **отличающимся** `code` → 400, строка в БД не изменилась) | `cargo test -p trackly-infra --test groups_repo` / `cargo test -p trackly-app --test groups_types_service` | ❌ Wave 0 |
| 2 | Засев не дублирует и не перезаписывает; удаление встроенного отклонено | app service: переименовать «АРМ»→«Рабочее место» (≠ сидового), повторить `seed_builtin_types_on_startup`, `COUNT(*)=1`, имя и `id` прежние; + `AppCtx::build` дважды на одном каталоге | `cargo test -p trackly-app --test groups_types_service seed_` | ❌ Wave 0 |
| 3 | Свойства всех 6 типов создаются; `sort_order`/`reorder` меняет порядок; `show_on_map` возвращается | app service (по свойству каждого типа) + DTO round-trip | `… --test groups_types_service properties_` | ❌ Wave 0 |
| 4 | (а) заполненное свойство скрыто, значения читаются в БД; (б) смена `data_type` — `Validation`; (в) `is_required` — отказ с именами групп | **три отдельных теста**; в (а) обязателен прямой `SELECT COUNT(*) FROM group_property_values WHERE property_id=?` > 0 после скрытия (иначе вакуумно) | `… --test groups_types_service protect_` | ❌ Wave 0 |
| 5 | IP/MAC нормализуются, `number` отвергает нечисловое, `users`/`device_refs` — только живые; отклонение на Tauri и HTTP | core: таблица-тест (`group_values`); app: `groups_set_values` через `build_*` и HTTP (400) | `cargo test -p trackly-core group_values` + `… --test groups_service values_` | ❌ Wave 0 |
| 6 | Пункт «Группы» виден admin/manager, не employee; `/groups` под employee — «Доступ запрещён»; PINNED обновлён | UI-гейт `check-groups-section.mjs` (позиция между `/devices` и `/acts`, `roles`, счёт 13+4=17 в комментарии = факт, `'/groups'` есть в `routes` и нет в `employeeRoutes`) + серверный 403 (матрица) ; **живая** проверка отображения — человек | `node ui/scripts/check-groups-section.mjs` | ❌ Wave 0 |
| 7 | `seq` 1,2; rename не сдвигает; одинаковые имена сохраняются; гонка | app service + конкурентный `tokio::join!` двух `create` → разные `seq`; infra: `UNIQUE(type_id,seq)` бэкстоп | `… --test groups_service numbering_` | ❌ Wave 0 |
| 8 | второе членство отклонено (понятное сообщение); цикл; вложение в `teardown` | infra (PK, CTE цикла) + app (русские сообщения) | `cargo test -p trackly-infra --test groups_repo` / `… --test groups_service membership_` | ❌ Wave 0 |
| 9 | перенос группы 6 устройств (2 во вложенной) меняет все 6, **атомарно**; место вложенной read-only; присоединение берёт место группы | infra+app: фикстура `t_boom` (триггер, сбой на 4-м устройстве) → всё как до вызова; мутация «автокоммит вместо tx» красит | `… --test groups_service move_` | ❌ Wave 0 |
| 10 | 7 строк с общим `batch_id`; отчёт — пакет с `batch_role`; история устройства — своя строка; `NULL`-место → 0 строк + `audit_log`; `batch_id` через `ALTER` | app: подсчёт строк; `reports_list_movements` поля пакета; `place_movements_get_timeline("device")` и `("group")`; infra-миграция `run_up_to(44)` с данными | `… --test group_movements_journal` / `cargo test -p trackly-infra --test groups_migration` | ❌ Wave 0 |
| 11 | `DeviceService::update` с новым `place_id` отклонён на обоих транспортах; повтор текущего проходит; после вывода проходит; спящая группа проходит | app: таблица-драйвер S1–S9 + счётный гейт исходника `act_service.rs` | `… --test group_write_sites` | ❌ Wave 0 |
| 12 | USB-принтер состава автоматически, без дубля при ссылке; primary сохраняется | app service (`printers.usb_host_device_id` + `device_refs` на один принтер → одна строка, `origin='usb'`) | `… --test groups_service card_` | ❌ Wave 0 |
| 13 | матрица 3 роли × (тип, группа) × 2 транспорта | HTTP-сессии + `build_*` напрямую; таблица-драйвер + тест полноты маршрутов (парсинг `http/groups.rs` через `include_str!`) | `… --test role_endpoint_matrix cases_groups` (Cases 76+ в существующем файле — его хелперы приватны) | ❌ Wave 0 (расширение существующего) |
| 14 | нет пользовательских строк «группа» для свёртки | UI-гейт `check-group-vocabulary.mjs --selftest` + прогон | `node ui/scripts/check-group-vocabulary.mjs --selftest && node …` | ❌ Wave 0 |
| — | `Action` матрица | core unit (`auth.rs::tests`) | `cargo test -p trackly-core auth` | ✅ файл есть, дописать |
| — | `per_record_invariants` для новых таблиц | infra | `cargo test -p trackly-infra --test per_record_invariants` | ✅ дописать списки |
| — | INV-7: новые producer’ы | UI-гейт | `node ui/scripts/check-place-tree-invalidation.mjs` | ✅ дописать реестр |
| — | чистые функции `reorder.ts` | node-гейт-фикстура (границы индексов, порядок) | `node ui/scripts/check-reorder.mjs` | ❌ Wave 0 |

### Не доказывается автоматически — нужен живой UAT человеком
- **Перестановка свойств перетаскиванием** (D-10): жест, ghost, отмена Esc/`pointercancel`, клавиатурный путь («Выше/Ниже»), `aria-live` — на **Tauri WKWebView и LAN-браузере** (после `pnpm --dir ui build`); и в идеале на Windows WebView2.
- **Асимметрия «десктоп vs браузер»** по всему экрану «Группы» (дерево, `Dropdown` с секциями, модалки): компиляционные гейты слепы (память `compile_gates_miss_svelte_runtime`, `synthetic_harness_not_verification`).
- **Рантайм Svelte:** отсутствие `effect_update_depth_exceeded` на экранах «Группы», «Устройства» (форма с заблокированным `PlacePicker`), «Отчёты» (свёртка пакета).
- **Печать отчёта «Перемещения»** (D-27): серверный HTML содержит все строки пакета — проверяется автотестом; **визуальный** результат печати (в десктопе и из LAN-браузера), многостраничность — человек. В печати не должно быть раскрытия/шевронов.
- **Инвалидация счётчиков дерева «Места»** после переноса группы (INV-7 ловит структуру, не рантайм).
- Вёрстка 35/65, фокус-кольцо (inset на первой ячейке), прокрутка региона (`app_shell_scroll_pattern`).
- Тексты тостов/подсказок D-19/D-24 в живом приложении.
- Согласно `auto_advance_auto_approves_human_verify`: `human-verify`-чекпоинты исполнители закрывают без запуска приложения — **собирать их и отдавать пользователю ДО верификации**, чтобы они не стали пробелами. Пункт чеклиста должен называть существующий компонент UI.

### Защита от ранее выявленных отказов проверки
- **Вакуумный тест:** (R1) HTTP-тест неизменяемости шлёт ОТЛИЧАЮЩИЙСЯ `code` и проверяет БД; (R2) переименование в имя ≠ сидового; (R4а) прямой SELECT значений после скрытия; (R11) фикстура с повтором ТЕКУЩЕГО места vs реальной сменой; (R10) тест 7 строк использует состав без принтеров с картриджами. Любой тест с «ожидаемой строкой» вычислять из прочитанного обратно, не хардкодить.
- **Неуникальные якоря мутаций** (`replace(old,new,1)` уезжает в соседнюю функцию): якоря, специфичные для функции — `ON CONFLICT(code) DO NOTHING` (одно вхождение в `seed_builtin_types_on_startup`), строка проверки `if new_place != before_place_id` в guard’е S1, `tx.commit()` внутри `move_group_in_tx`, строка вызова `release_device_in_tx(` с аргументами конкретного act-сайта; в скрипте мутации **`assert s.count(old) == 1`** и `git diff` после патча. Если мутация не покраснила — сперва доказать, что она применилась.
- **Фикстура, не различающая до/после:** для D-22/S3–S7 фикстура должна иметь устройство-члена с местом, отличным от места акта; для S1 — тест с «сменой на другое место» и «повтором текущего».
- **Исполнитель закрыл «зелёным по своим файлам» при красном пакете:** на границе волны прогонять пакет целиком по целям (`groups_*`, `group_*`, `role_endpoint_matrix`, `place_movements_*`, `places_*`, `acts_*`, `devices_crud`), а не только добавленные файлы.

### Sampling Rate
- **Per task commit:** целевой `--test` изменённого слоя + `node` гейт, если трогался UI.
- **Per wave merge:** все `groups_*`/`group_*` цели + `role_endpoint_matrix` + `place_movements_*` + `per_record_invariants` + `acts_*` (затронуты write-site’ы) + `svelte-check` + `pnpm lint`.
- **Phase gate:** полный `cargo test --workspace` (один раз, в фоне) + clippy `-D warnings` + `svelte-check` + `pnpm lint` + privacy-гейт, затем `/gsd-verify-work` и живой UAT.

### Wave 0 Gaps
- [ ] `crates/trackly-core/src/domain/group_values.rs` — таблицы-тесты ip/mac/number/text (R5)
- [ ] `crates/trackly-infra/tests/groups_migration.rs` — V045/V046, `run_up_to(44)` без потери данных (R10)
- [ ] `crates/trackly-infra/tests/groups_repo.rs` — триггер, PK, CTE цикла/состава, атомарность `move_group_in_tx` (R1, R8, R9)
- [ ] `crates/trackly-app/tests/groups_types_service.rs` — R1–R4 (+ засев)
- [ ] `crates/trackly-app/tests/groups_service.rs` — R5, R7–R9, R12
- [ ] `crates/trackly-app/tests/group_write_sites.rs` — R11, S1–S9, счётный гейт
- [ ] `crates/trackly-app/tests/group_movements_journal.rs` — R10 (журнал, отчёт, таймлайн)
- [ ] Cases 76+ в `crates/trackly-app/tests/role_endpoint_matrix.rs` — R13 (+ полнота маршрутов)
- [ ] `ui/scripts/check-group-vocabulary.mjs` (+`--selftest`), `check-groups-section.mjs`, `check-reorder.mjs`; подключить в цепочку `lint` `ui/package.json`
- [ ] Расширить: `auth.rs::tests`, `per_record_invariants.rs` (списки таблиц), `check-place-tree-invalidation.mjs` (реестр)
- [ ] Framework install: не требуется

## Security Domain

> `security_enforcement` включён (ASVS level 1, `security_block_on: high`).

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no (фаза не меняет вход) | существующая сессионная схема |
| V3 Session Management | no | `tower-sessions`, не затронуто |
| V4 Access Control | **yes** | `authorize(Action::{ManageGroupTypes,MutateGroups,ReadGroups})` в сервисе и в `build_*`; матрица 3×2 тестами; employee — 403 на всём API |
| V5 Input Validation | **yes** | серверная валидация в домене (`group_values`), лимиты длины/числа, параметризованные запросы `rusqlite::params!` (никакой конкатенации ввода в SQL — как в `places_sqlite.rs`) |
| V6 Cryptography | no | не применимо |
| V7/V8 Logging & data protection | yes (минимально) | `audit_log` на мутации групп/типов/свойств; `groups_user_options` отдаёт только `id`, `full_name`, `login` (без email/роли/хэша) |

### Known Threat Patterns for этого стека

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Манипуляция ролью: менеджер вызывает мутацию типа напрямую по HTTP | Elevation of Privilege | отдельный `Action::ManageGroupTypes` (Admin only), тест Manager→403 на каждой команде типа/свойства; НЕ переиспользовать `MutatePlaces` |
| Смена `code`/`behavior` встроенного типа обходом UI | Tampering | триггер БД + проверка сервиса (Pitfall 10) |
| Инъекция через значения свойств (IP/MAC/текст) | Tampering | нормализация/валидация на сервере; значения в SQL только параметрами; Svelte экранирует вывод (фаза 43 при рисовании SVG — отдельно) |
| Пакетный запрос с огромным списком (DoS единственного writer’а) | Denial of Service | потолки `device_ids`/ссылок/длин (A11); очередь writer’а 256 + `send_timeout` |
| Обход запрета индивидуального перемещения через другие write-site’ы | Tampering | инвентаризация S1–S9 + счётный гейт |
| Ссылка на чужие/мёртвые `user_id`/`device_id` в значениях | Tampering/Info disclosure | проверка «жив» при записи, фильтр живых при чтении |
| Утечка реальных данных организации в фикстурах/планах/отчёте | Information Disclosure | только вымышленные ФИО («Иванов И.И.», «Петров П.П.»), `scripts/check-privacy.mjs` до коммита; в журнале и тестах — placeholder’ы вида «АРМ #3» |

## Sources

### Primary (HIGH confidence) — прочитано в репозитории (HEAD 5ee71084)
- `.planning/phases/41-groups-model-and-editor/41-SPEC.md`, `41-CONTEXT.md`, `.planning/v1.4-GROUPS-MODEL-DECISIONS.md`, `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md` (Phase 41)
- `migrations/V014__*.sql`, `V019`, `V020`, `V037`, `V038`, `V040`, `V042`, `V043`
- `crates/trackly-infra/src/db/{migrations.rs,writer_worker.rs}`, `repos/{places_sqlite.rs,place_movements_sqlite.rs,devices_sqlite.rs,cartridges_sqlite.rs}`
- `crates/trackly-app/src/services/{place_service.rs,place_movement_service.rs,device_service.rs,act_service.rs,report_service.rs,supervisor.rs}`, `context.rs`, `specta_export.rs`, `tauri_cmds/places.rs`, `http/places.rs`, `dto/{place.rs,place_movements.rs,device.rs}`
- `crates/trackly-core/src/{auth.rs,domain/place_movements.rs}`, `crates/trackly-app/tests/{role_endpoint_matrix.rs,number_space_broadcast_gate.rs}`, `crates/trackly-infra/tests/{per_record_invariants.rs,place_movements_migration.rs}`
- `ui/src/features/places/{PlacesMasterDetail,PlaceTree,PlaceTreeNode,PlaceContents,PlacesPage}.svelte`, `ui/src/lib/components/{Dropdown,TableRow,Table,ActionMenu,Placeholder,DetailPanel,MovementTimeline,NumberTakenPopup,PlacePicker}.svelte`, `ui/src/features/{devices,reports,layout,acts}/*`, `ui/src/routes.ts`, `ui/scripts/check-place-tree-invalidation.mjs`, `ui/package.json`
- `.planning/phases/39-place-tree/39-UAT.md` (GAP-2, GAP-11, итог UAT)
- Эксперименты в этой сессии: DDL всей схемы + `ALTER TABLE ADD COLUMN` + триггер + `ON CONFLICT DO NOTHING` в `sqlite3 3.51.0`; `std::net::IpAddr` на 9 входах (rustc 1.95.0)

### Secondary (MEDIUM confidence)
- Память проекта (`MEMORY.md` и связанные заметки): `compile_gates_miss_svelte_runtime`, `mutation_test_anchor_must_be_unique`, `vacuous_test_fixture_must_diverge`, `invalidation_write_sites_enumerate_from_server`, `app_shell_scroll_pattern`, `table_focus_ring_decision`, `native_select_vs_custom_dropdown`

### Tertiary (LOW confidence)
- Внешних источников (веб/Context7) не использовалось: все вопросы фазы — внутрипроектные; сторонние библиотеки не добавляются.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — новых зависимостей нет, версии прочитаны из `Cargo.toml`/`package.json`.
- Architecture (схема, транзакция, засев, write-site’ы): HIGH — код прочитан, DDL и ключевые SQL-приёмы проверены выполнением.
- Журнал/отчёт: MEDIUM-HIGH — места поломок найдены чтением `query_movements_inner`; финальная форма DTO — решение планировщика.
- UI (reorder, Dropdown-секции, дерево): MEDIUM — прецеденты есть и подтверждены UAT, но `<tbody>`-перестановка и секции `Dropdown` — новый код, требует живого чека.
- Pitfalls: HIGH — каждый опирается на найденный в коде факт или зафиксированный проектный урок.

**Research date:** 2026-10-04
**Valid until:** 2026-11-03 (30 дней; зависит от изменений `act_service.rs`/`device_service.rs` — номера строк write-site’ов пересверить перед планированием)

## Предлагаемая декомпозиция (подсказка планировщику, не решение)

1. **Wave 0 (схема/домен):** V045+V046, `domain/groups.rs`+`group_values.rs`, `Action`-варианты, `per_record_invariants`, `groups_migration.rs`, `MovementSource::Group`/`EntityKind::Group`.
2. **Wave 1 (репозиторий+сервис типов):** `SqliteGroupRepository` (CRUD, CTE), `GroupService` типы/свойства/засев, DTO, тесты R1–R4.
3. **Wave 2 (группы и перенос):** CRUD групп, членство, значения, `move_group_in_tx`, журнал `batch_id`/`entity_label`, карточка и принтеры, тесты R5, R7–R10, R12.
4. **Wave 3 (чужие write-site’ы):** guard S1, release S3–S7, `delete_soft` S8, `move_subtree_contents` S2, `referencing_group_count`, `group_write_sites.rs`; отчёт/таймлайн (DTO-поля).
5. **Wave 4 (транспорты):** `tauri_cmds/groups.rs`, `http/groups.rs`, `specta_export.rs`, `AppCtx`, матрица прав (Cases 76+).
6. **Wave 5 (UI):** сайдбар/маршрут, `GroupTree`, панели типа/группы, таблица свойств + `reorder.ts`, состав + `Dropdown`-секции, «Перенести», форма устройства (D-19), `MovementTimeline`/отчёт, переименование + гейты, INV-7.
7. **Wave 6:** человеческий UAT на обоих транспортах (сбор `human-verify` отдать пользователю ДО верификации).
