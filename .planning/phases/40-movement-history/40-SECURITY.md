---
phase: 40
slug: movement-history
status: verified
threats_open: 0
threats_total: 72
threats_closed: 72
asvs_level: 1
block_on: high
created: 2026-09-05
---

# Phase 40 — Security

> Контракт безопасности фазы: реестр угроз, принятые риски, аудиторский след.

**Область аудита:** git-диапазон `4defcce2..HEAD` (первый коммит фазы — `cb75005b`
«docs(40): capture phase context», родитель — `4defcce2`). Реестр угроз составлен
на этапе планирования: все 35 PLAN-файлов (`40-01` … `40-35`) содержат разбираемый
блок `<threat_model>` — 72 угрозы, 41 `mitigate` + 31 `accept`. Данный аудит
**проверяет наличие заявленных мер в реализации**, а не выполняет ретроспективный
STRIDE-скан.

**Коллизии ID:** планировщик переиспользовал номера — `T-40-19`, `T-40-20`,
`T-40-21` встречаются в двух планах каждый с разным смыслом. Ниже все ID
пространственно именованы как `{план}/{threat_id}`.

**Threat Flags из SUMMARY:** секция `## Threat Flags` присутствует только в
`40-10`, `40-16`, `40-17` — все три «None» с обоснованием. Ни один SUMMARY не
поднял новую угрозу. Проверено: `grep -l "Threat Flags" *SUMMARY.md`.

---

## Trust Boundaries

| Граница | Описание | Что пересекает |
|---------|----------|----------------|
| LAN-браузер / Tauri-webview → `build_*` helper | Пять НОВЫХ эндпоинтов фазы, каждый на двух транспортах; `Identity` резолвится сервером (`session_identity` / `resolve_tauri_identity`), никогда не десериализуется из тела запроса | `entity_type`/`entity_id`, `ReportFilter`, `rootId`/`targetPlaceId`, `op`/`cartridge_id` |
| Клиентский payload → `place_movements.source` | `source` НИКОГДА не приходит от клиента: ни `DevicePatch`, ни `CartridgeTransitionOp` не имеют такого поля; сервер жёстко подставляет `MovementSource::Manual`/`::Act` в каждом write-site | — (граница закрыта по конструкции) |
| `ReportFilter.from_place_id`/`to_place_id` → SQL WHERE | Типизированные `Option<i64>`, связываются через `?N` + `owned_params`; в `format!` попадает только НОМЕР плейсхолдера | целые id мест |
| Пул читателей → таймлайн/отчёт | До 20 одновременных LAN-читателей на 8 read-only соединений; вложенный `acquire()` в цикле = self-inflicted deadlock | — |
| Сервер → UI (`MovementEntryDto`) | Пути мест уже сокращены сервером; ФИО актора — снапшот на момент записи; сырой токен `source` проходит насквозь с UI-фолбэком | ФИО сотрудников, пути мест |
| Диф фазы → ПУБЛИЧНЫЙ git-репозиторий | Репозиторий публичный; в тестах и фикстурах — только вымышленные имена | вымышленные ФИО («Иванов И.И.», «Петров П.П.», «Сидоров С.С.») |

**Новая поверхность атаки фазы (полный перечень, из дифа роутеров):**
`/api/v1/place_movements_get_timeline`, `/api/v1/reports_list_movements`,
`/api/v1/places_move_subtree_contents`, `/api/v1/cartridges_operation_default_place`,
`/api/v1/cartridges_to_refill_last_send` — плюс их Tauri-двойники. Каждый
отображается на угрозу реестра (см. ниже); незарегистрированной поверхности нет.

---

## Threat Register — mitigate (41)

| Threat ID | Категория | Диспозиция | Доказательство в коде | Статус |
|-----------|-----------|------------|------------------------|--------|
| 40-01/T-40-02 | Information Disclosure | mitigate | Read-side гейт присутствует ниже по потоку, как и требовал план: `services/place_movement_service.rs:56` `authorize(caller, &Action::ReadPlaces)?` ПЕРВОЙ строкой, до любого запроса к БД | closed |
| 40-02/T-40-03 | Tampering | mitigate | Единственный владелец формулы — `services/place_path_display.rs:42` / `:61`; оба потребителя импортируют, не переизобретают (`place_movement_service.rs:27`, `report_service.rs:33` — импорт с алиасом). Сам алгоритм живёт в core `shorten_place_path`; локальный `report_service.rs:286` — предсуществующая тонкая обёртка над тем же core-вызовом, не вторая копия | closed |
| 40-03/T-40-04 | Spoofing | mitigate | `caller` — только серверный `Identity`: `http/devices.rs:181` `session_identity(&session)`, Tauri — `resolve_tauri_identity`; гейт не тронут: `tauri_cmds/devices.rs:64` `authorize(caller, &Action::MutateDevices)?` | closed |
| 40-03/T-40-05 | Elevation of Privilege | mitigate | Оба транспорта проходят через один `build_devices_update` (`http/devices.rs:185`), сигнатура `device_service::update(caller, …)` (`device_service.rs:266-272`) обязательная — пропущенный call site не собрался бы | closed |
| 40-04/T-40-06 | Spoofing | mitigate | `tauri_cmds/cartridges.rs:66` и `:89` — `authorize(caller, &Action::MutateCartridges)?` остаётся единственным гейтом; `cartridge_service.rs:192-199`, `:425-428` принимают `caller: &Identity` | closed |
| 40-04/T-40-07 | Elevation of Privilege | mitigate | Ноль `user_id: None` внутри `transition_in_tx`: `cartridges_sqlite.rs:817` и `:851` пишут `user_id: caller_user_id`; вложенный авто-возврат — `:665`, `:769`. Регрессионные тесты `cartridges_sqlite.rs:2316`, `:2366` (main + auto-return) | closed |
| 40-05/T-40-08 | Tampering | mitigate | Гейт централизован: `repos/place_movements_sqlite.rs:119-135` (`is_reportable_place_change` → ранний `Ok(())`). Ни один write-site не переизобретает его: `INSERT INTO place_movements` в продакшн-коде существует ТОЛЬКО в `place_movements_sqlite.rs:82`; два других вхождения (`report_service.rs:3351`, `cartridges_sqlite.rs:3003`) — внутри `#[cfg(test)]` (модули с 2584 и 1965 строки соответственно) | closed |
| 40-05/T-40-10 | Information Disclosure | mitigate | `place_movements_sqlite.rs:146-153`: лукап ФИО через `.ok()`, без `?` — отсутствующая строка `users` даёт `None`, мутация не падает, сырая SQL-ошибка не утекает | closed |
| 40-06/T-40-11 | Spoofing | mitigate | Все четыре мутации получают серверный `caller`: `act_service.rs:219/221`, `:627/629`, `:1670/1672`, `do_return`; гейты не тронуты — `tauri_cmds/acts.rs:60,71,94,107` `authorize(caller, &Action::MutateActs)?` | closed |
| 40-06/T-40-12 | Elevation of Privilege | mitigate | ВСЕ семь мест записи движения в `act_service.rs` (506, 789, 960, 1538, 2030, 2117, 2203) передают `user_id_opt`, ни одного захардкоженного `None` не осталось — проверено построчно, включая обе ветки циклов `update_return` | closed |
| 40-07/T-40-15 | Spoofing | mitigate | `dto/device.rs:136-150` — в `DevicePatch` НЕТ поля `source`; сервер жёстко ставит `MovementSource::Manual` (`device_service.rs:326`, `:352`) | closed |
| 40-07/T-40-16 | Tampering | mitigate | D-04 guard проверен и на сервисном уровне: тесты `tests/place_movements_write_sites_devices.rs:206` (`status_only_noop`) и `:253` (`first_assignment_noop`) | closed |
| 40-08/T-40-17 | EoP / Info Disclosure | mitigate | `tests/place_movements_write_sites_cartridges.rs:214` — тест вложенного авто-возврата, утверждения на ВТОРОЙ строке (`a_rows[1]`, строки 293-305) и на принадлежности `entity_id` первому картриджу (`:275`) | closed |
| 40-08/T-40-18 | Spoofing | mitigate | `domain/cartridges.rs:110-158` — ни один вариант `CartridgeTransitionOp` не имеет поля `source`; оба call site жёстко задают `MovementSource::Manual` (`cartridge_service.rs:282`, `cartridges_sqlite.rs:613`, `:662`, `:766`) | closed |
| 40-09/T-40-19 | Data Integrity | mitigate | Both-Some гейт до любого INSERT — `place_movements_sqlite.rs:133-139`; тест `tests/place_movements_act_link.rs:250` `place_movements_null_place_skip` | closed |
| 40-10/T-40-21 | Elevation of Privilege (BOLA) | mitigate | `place_movement_service.rs:56` — ролевой гейт до запроса; per-item логики, которую можно обойти подбором `entity_id`, нет вовсе. Матрица: Case 52/53 (`role_endpoint_matrix.rs:1931`, `:1976`) — Manager allow / Employee 403 на ОБОИХ транспортах | closed |
| 40-10/T-40-22 | Elevation of Privilege | mitigate | Оба транспорта делегируют в один `build_place_movements_get_timeline` (`tauri_cmds/place_movements.rs:24-33`, `http/place_movements.rs:39-42`); гейт продублирован на границе транспорта (`tauri_cmds/place_movements.rs:30`) как defense-in-depth | closed |
| 40-10/T-40-23 | Information Disclosure | mitigate | Сырой токен проходит насквозь по дизайну (`place_movement_service.rs:116-120`), типизированный разбор с фолбэком доступен (`domain/place_movements.rs:45-56`) и реально используется на Rust-стороне отчёта (`report_service.rs:1367-1377`); паники нет | closed |
| 40-11/T-40-13 | Tampering (SQLi) | mitigate | `report_service.rs:1416-1446`: `owned_params.push(Box::new(from_place_id))` / `to_place_id`, в `format!` подставляется только индекс `?{idx}`; DTO-поля типизированы `Option<i64>` (`dto/reports.rs:38`, `:43`) — строковая интерполяция пользовательских значений отсутствует | closed |
| 40-12/T-40-24 | Elevation of Privilege | mitigate | `tauri_cmds/reports.rs:269` — `Action::ReadPlaces`, НЕ `ReadData`. Экспорт согласован: `export_gate_action` (`:287-292`) + юнит-тест на КОНКРЕТНЫЙ вариант `Action` (`:311-313`), устойчивый к совпадению ролевых наборов | closed |
| 40-12/T-40-25 | Elevation of Privilege | mitigate | `http/reports.rs:246-258` — handler только резолвит сессию и делегирует в тот же `build_reports_list_movements`; матрица Case 54/55 (`role_endpoint_matrix.rs:2037`, `:2090`), экспорт — Case 56/57 | closed |
| 40-13/T-40-26 | Elevation of Privilege | mitigate | `tauri_cmds/places.rs:192-193` — переиспользованы `MutateDevices` + `MutateCartridges`; новых вариантов `Action` не введено: диф `crates/trackly-core/src/auth.rs` за фазу добавляет только `PartialEq, Eq` к `derive` | closed |
| 40-13/T-40-27 | Data Integrity | mitigate | Одна транзакция на всю операцию — `place_service.rs:694-…` (`self.writer.execute` → `conn.transaction()`, цикл по всем предметам внутри); тест с инъекцией сбоя через `BEFORE UPDATE` триггер: `tests/place_movements_bulk_move.rs:454` `…_atomicity_on_failure` | closed |
| 40-14/T-40-28 | Elevation of Privilege | mitigate | Для эндпоинтов, существовавших на момент плана 40-14, оба транспорта покрыты: Cases 52/53 (timeline), 54/55 (отчёт), 56/57 (экспорт), 58/59 (bulk move) — `role_endpoint_matrix.rs:1918-2348`. См. WARNING-01 ниже про два эндпоинта, добавленных ПОЗЖЕ | closed (с оговоркой) |
| 40-15/T-40-29 | Tampering | mitigate | `MovementTimeline.svelte:101-113` — только отображение серверных строк (`*_path_short` с фолбэком на полный путь, полный путь в `title=`); JS-зеркала формулы нет: в компоненте нет ни `split`/`slice`/`join`/`substring` над путями | closed |
| 40-15/T-40-30 | Information Disclosure | mitigate | `MovementTimeline.svelte:61-64` — любой нераспознанный `source` даёт безопасную строку «причина не определена», исключение не выбрасывается | closed |
| 40-17/T-40-34 | Information Disclosure | mitigate | `formatHistoryEntry` в `CartridgeDetail.svelte` не изменялась за фазу (диф `4defcce2..HEAD` по файлу не содержит ни одной строки с этим идентификатором); новая секция рендерится исключительно через `MovementTimeline` (`CartridgeDetail.svelte:255`), сырых `place_id` не показывает | closed |
| 40-19/T-40-32 | Repudiation | mitigate | `PlaceContents.svelte:425-428` — текст подтверждения прямо предупреждает: «Для каждого появится запись в истории перемещений с причиной “вручную”» | closed |
| 40-20/T-40-20 | DoS / Data Integrity | mitigate | Удаление строго по `act_id` — `place_movements_sqlite.rs:186`; тест `tests/place_movements_act_link.rs:334` сеет контрольный акт и утверждает выживание его строк (`:442-444`) | closed |
| 40-21/T-40-21-01 | Tampering | mitigate | `cartridges_sqlite.rs:1202-1213` — каскад выбирает картриджи по `current_printer_device_id` ИЗ БД, `printer_device_id` связан параметром; клиент не задаёт список | closed |
| 40-22/T-40-22-01 | Tampering | mitigate | `cartridges_sqlite.rs:987-1018` — оба запроса параметризованы `params![cartridge_id]`, id извлечён сервером из уже провалидированного `prev_id` (`:714`) | closed |
| 40-27/T-40-27-01 | DoS | mitigate | `PdfPreviewModal.svelte:295` `let printing = $state(false)`; `:578-580` ранний выход `if (!ready \|\| htmlContent === null \|\| printing) return;`, сброс в `finally` (`:592`) | closed |
| 40-28/T-40-28-01 | Repudiation | mitigate | `device_service.rs:346` — `if after.place_id.is_some() && before_place_id != after.place_id` перед каскадом; в самом каскаде `debug_assert!` дублирует инвариант (`cartridges_sqlite.rs:1195-1201`); тест `tests/place_movements_write_sites_devices.rs:533` | closed |
| 40-28/T-40-28-02 | Tampering (integrity) | mitigate | `cartridges_sqlite.rs:987-1018` — трёхступенчатая цепочка (`to_place_id` ИЛИ `from_place_id` складского места → собственное место), кандидаты фильтруются `archived_at_utc IS NULL AND deleted_at_utc IS NULL` (WR-07) на обеих ступенях | closed |
| 40-29/T-40-29-01 | Denial of Service | mitigate | Вложенных `acquire()` не осталось: `place_movement_service.rs:63` — единственный acquire на весь запрос, форматирование пути идёт через `compute_place_path_short_with_conn(&conn, …)` (`:94`, `:99`); в `report_service.rs` `query_movements_inner` работает на переданном `conn`. Bounded-примитив добавлен: `db/pools.rs:111-140` `acquire_timeout` с `wait_timeout` и финальной попыткой | closed |
| 40-29/T-40-29-02 | Repudiation | mitigate | Единый владелец формата номера акта — `services/act_number_display.rs:34` `resolve_movement_act_number`, вызывается обеими поверхностями: `place_movement_service.rs:89` и `report_service.rs:1557` | closed |
| 40-30/T-40-30-01 | Information Disclosure | mitigate | `tauri_cmds/cartridges.rs:226` `authorize(caller, &Action::ReadData)?` ДО запроса к БД; оба транспорта делегируют в этот `build_*` (`tauri_cmds/cartridges.rs:459`, `http/cartridges.rs::handler_operation_default_place`). Матрица: Case 60/61 (HTTP) | closed |
| 40-30/T-40-30-02 | Tampering | mitigate | `cartridge_service.rs:1015-1040` — `match op` по точному литералу `"from_refill"`, ветка `other =>` возвращает `AppError::Validation`; «молчаливого default» нет | closed |
| 40-33/T-40-33-01 | Information Disclosure | mitigate | `tauri_cmds/cartridges.rs:240` `authorize(caller, &Action::ReadData)?` до `ctx.cartridges.to_refill_last_send()`; Tauri-обёртка `:471-476` делегирует в тот же `build_*`. Матрица: Case 62/63 (HTTP) | closed |
| 40-33/T-40-33-03 | Tampering | mitigate | `"to_refill"` удалён, а не скрыт: `cartridge_service.rs:1033-1039` явная `AppError::Validation`; тест `tests/cartridges_lifecycle.rs:1983` `operation_default_place_to_refill_now_returns_validation_error`. `most_common_to_refill_destination` в коде отсутствует (остались только упоминания в комментариях) | closed |
| 40-34/T-40-34-01 | Tampering | mitigate | `act_service.rs:2552-2565` — новая арка `given_to_name_arm` использует тот же связанный параметр `?1` c `LIKE ?1 ESCAPE '\\'`; паттерн строится один раз через `escape_like` (`:2525`, определение `:3179`); имена колонок — whitelisted enum, не строка от пользователя | closed |

---

## Accepted Risks (31) — принятые риски, обоснование перепроверено по коду

| Threat ID | Категория | Обоснование принятия | Проверка факта | Статус |
|-----------|-----------|----------------------|----------------|--------|
| 40-01/T-40-01 | Tampering | Нет SQL CHECK на `source`/`entity_type` по дизайну (Pitfall 6 / IN-01): значения всегда серверные, read-side мягко деградирует | `migrations/V040__place_movements.sql:57,62` — CHECK отсутствует; `from_str_lenient` есть (`domain/place_movements.rs:45`, `:88`) и используется на чтении (`report_service.rs:1367`); ни один клиентский DTO не несёт `source` | closed |
| 40-01/T-40-SC | Supply chain | План не добавляет пакетов | `git diff 4defcce2..HEAD -- Cargo.lock ui/pnpm-lock.yaml` — пусто; в `ui/package.json` изменена только строка `lint` | closed |
| 40-02/T-40-SC | Supply chain | То же | То же | closed |
| 40-05/T-40-09 | Denial of Service | `get_history` без LIMIT — масштаб «единицы перемещений за годы», пагинация отклонена пользователем | `place_movements_sqlite.rs:203-207` — `ORDER BY created_at_utc DESC, id DESC` без LIMIT, подтверждено | closed |
| 40-11/T-40-14 | Information Disclosure | Гейт откладывался на план 40-12; на момент 40-11 маршрута не существовало | Факт снят последующим планом: гейт есть (`tauri_cmds/reports.rs:269`), риск больше не актуален | closed |
| 40-16/T-40-33 | Elevation of Privilege | Скрытие пункта меню от Employee косметическое; реальный гейт серверный | Подтверждено: `place_movement_service.rs:56` + матрица Case 52/53 — Employee получает 403/`Forbidden`, а не данные | closed |
| 40-18/T-40-31 | Elevation of Privilege | Скрытие вкладки отчёта косметическое; реальный гейт серверный | Подтверждено: `tauri_cmds/reports.rs:269` + Cases 54-57 | closed |
| 40-20/T-40-19 | Repudiation | D-03: компенсирующая запись не создаётся; событие удаления акта фиксирует `audit_log` | `act_service.rs:2682/2715/2746` — `audit_repo.insert(AuditEntry{…})` в `delete_soft`; `place_movements` чистится только по своему `act_id` | closed |
| 40-21/T-40-21-02 | Repudiation | `source` жёстко `Manual`, `note` — серверная константа, `user_id` пишется как обычно | `device_service.rs:350-353` — `MovementSource::Manual`, литерал «вместе с принтером», `user_id_opt` | closed |
| 40-21/T-40-21-03 | DoS | На принтере физически 1 картридж «В работе»; масштаб организации мал | Каскад ограничен `WHERE current_printer_device_id = ?1 AND deleted_at_utc IS NULL` (`cartridges_sqlite.rs:1205-1206`) — без пользовательского расширения выборки | closed |
| 40-22/T-40-22-02 | Information Disclosure | Fallback показывает оператору место, уже доступное ему через таймлайн (Admin/Manager) | Тот же ролевой круг: `Action::ReadPlaces`/`ReadData`, новых данных не появляется | closed |
| 40-23/T-40-23-01 | Tampering | Клиентская валидация — UX, не гейт; сервер уже принимает `place_id: None` и сам резолвит | `OperationModal.svelte:695-707` — блок снят только там, где сервер сам восстанавливает место; серверный резолв — `cartridge_service.rs:430-433` (D-13) | closed |
| 40-24/T-40-24-01 | Information Disclosure | `parent_act_id`/`sub_number` уже доступны через `acts.get(id)` под тем же гейтом | Расширенное чтение идёт внутри `resolve_movement_act_number` по серверному FK `act_id` | closed |
| 40-24/T-40-24-02 | Tampering | `id` из hash парсится как число и не попадает в SQL-фильтр | `lib/utils/hashId.ts` — `Number(raw)` + `Number.isInteger(n)`, иначе `null` | closed |
| 40-25/T-40-25-01 | Information Disclosure | `is_deleted` уже приходит в каждой строке отчёта; правка меняет только видимость | `dto/reports.rs:147` `pub is_deleted: Option<bool>`; `ReportTable.svelte:175` — только условие показа бейджа | closed |
| 40-26/T-40-26-01 | Information Disclosure | `place_distinct_count` выводим из уже доступного `list_by_ids` | `repos/devices_sqlite.rs:1102/1147/1194` — `COUNT(DISTINCT COALESCE(d.place_id, -1))` в тех же гейтед-запросах | closed |
| 40-30/T-40-30-03 | Denial of Service | Масштаб одной LAN-организации; пользовательского фильтра, расширяющего скан, нет | Резолвер заменён на `LIMIT 1`-запрос (`cartridges_sqlite.rs:1093-1105`), без клиентских параметров | closed |
| 40-30/T-40-30-04 | EoP / BOLA | Admin/Manager не имеют per-item ownership ни на одном чтении картриджей | Та же модель, что у `get_history`; гейт `ReadData` (`tauri_cmds/cartridges.rs:226`) | closed |
| 40-30/T-40-30-SC | Supply chain | Нет новых crate | Диф lock-файлов пуст | closed |
| 40-31/T-40-31-01 | Tampering | Ответ используется только для предзаполнения; финальную мутацию валидирует backend | Мутация `cartridges_transition` по-прежнему гейтится `MutateCartridges` (`tauri_cmds/cartridges.rs:89`) | closed |
| 40-31/T-40-31-SC | Supply chain | Нет новых npm-пакетов | `ui/pnpm-lock.yaml` не менялся | closed |
| 40-32/T-40-32-01 | Information Disclosure | Стор содержит только числовые `place.id`, не пересекает сетевую границу | `lib/stores/placeContentEvents.svelte.ts:30-38` — `{ seq: number; placeIds: number[] }`, module-level | closed |
| 40-32/T-40-32-02 | Denial of Service | Перезапрос ленивый, только для уже развёрнутых узлов | `PlaceTree.svelte:375-392` — эффект лишь удаляет ключи из `statsCache`, запись условна (`changed`), перезапрос — по факту видимости | closed |
| 40-32/T-40-32-SC | Supply chain | Нет новых npm-пакетов | Диф lock-файлов пуст | closed |
| 40-33/T-40-33-02 | Information Disclosure | ФИО видны тому же кругу Admin/Manager, что и в `get_history`/актах | Гейт `ReadData` до запроса (`tauri_cmds/cartridges.rs:240`) | closed |
| 40-33/T-40-33-04 | Denial of Service | `audit_log` растёт линейно с числом операций при малом масштабе | `cartridges_sqlite.rs:1103-1105` — `WHERE entity_type='cartridge' AND action='custom:to_refill' ORDER BY … LIMIT 1`, без клиентских параметров (`params![]`) | closed |
| 40-33/T-40-33-SC | Supply chain | Нет новых crate | Диф lock-файлов пуст | closed |
| 40-34/T-40-34-02 | Information Disclosure | Тот же круг видимости, что у уже принятой арки `given_by_name` (Фаза 12) | Симметричная арка в том же запросе под тем же гейтом `ReadData` (`tauri_cmds/acts.rs:142`) | closed |
| 40-34/T-40-34-SC | Supply chain | Нет новых crate | Диф lock-файлов пуст | closed |
| 40-35/T-40-35-01 | Tampering | Предзаполнение полей; мутацию валидирует и гейтует backend | `cartridges_transition` гейтится `MutateCartridges` | closed |
| 40-35/T-40-35-SC | Supply chain | Нет новых npm-пакетов | Диф lock-файлов пуст | closed |

---

## Findings

### WARNING-01 — неполное покрытие транспорт-матрицы для двух поздних эндпоинтов

Не блокер. `40-14/T-40-28` объявляла инвариант «каждый новый эндпоинт получает и
HTTP-, и Tauri-кейс». Эндпоинты, добавленные ПОЗЖЕ плана 40-14 в раундах
gap-closure — `cartridges_operation_default_place` (40-30) и
`cartridges_to_refill_last_send` (40-33) — имеют в `role_endpoint_matrix.rs`
только HTTP-кейсы (Case 60-63); Tauri-половины (`build_*` напрямую, как в Case
53/55/59) нет.

Почему это не BLOCKER: гейт живёт в общей функции `build_*`
(`tauri_cmds/cartridges.rs:226` и `:240`), а Tauri-обёртки (`:453-467`, `:471-476`)
делают ровно `resolve_tauri_identity` + делегирование — обойти гейт со стороны
Tauri невозможно без правки самой `build_*`. Проверено чтением кода, а не
предположением. Риск — регрессионный, не эксплуатационный: будущая правка,
снимающая гейт на Tauri-пути, не будет поймана матрицей.

Рекомендация (не входит в область данного аудита, реализация не правится):
добавить Case 64/65 — `build_cartridges_operation_default_place` и
`build_cartridges_to_refill_last_send` с Manager-allow / Employee-deny.

### Unregistered flags

Нет. Все пять новых эндпоинтов фазы отображаются на угрозы реестра
(40-10, 40-12, 40-13, 40-30, 40-33). Новых вариантов `Action` не введено. Новых
зависимостей (crate/npm) не добавлено. Ни один SUMMARY не поднял открытый флаг.

### Приватность (жёсткое условие CLAUDE.md)

Реальные данные организации и людей в артефактах фазы и в проверенном коде не
обнаружены: тестовые ФИО — вымышленные («Иванов И.И.», «Петров П.П.»,
«Сидоров С.С.»), места — «Каб. 401», «Склад №4», модели — обезличенные. Данный
документ реальных данных не содержит.

---

## Audit Trail

- Реестр: 72 угрозы из 35 `<threat_model>`-блоков; 41 `mitigate` + 31 `accept`.
- Проверено: 41/41 mitigate — найдена конкретная строка кода/теста; 31/31 accept —
  обоснование перепроверено против кода, не принято на слово.
- Открытых угроз: 0. Блокеров: 0. Предупреждений: 1 (WARNING-01).
- Файлы реализации в ходе аудита не изменялись.

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-05 | 72 | 72 | 0 | gsd-security-auditor (/gsd-secure-phase 40) |

---

## Sign-Off

- [x] Все угрозы имеют диспозицию (mitigate / accept / transfer)
- [x] Принятые риски задокументированы в разделе «Accepted Risks»
- [x] `threats_open: 0` подтверждено
- [x] `status: verified` установлен во frontmatter

**Approval:** verified 2026-09-05
