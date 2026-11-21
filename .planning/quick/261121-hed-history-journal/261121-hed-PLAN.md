---
phase: quick-261121-hed
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-core/src/domain/place_movements.rs
  - migrations/V056__place_movements_request_id.sql
  - crates/trackly-infra/src/repos/place_movements_sqlite.rs
  - crates/trackly-infra/src/repos/cartridges_sqlite.rs
  - crates/trackly-infra/tests/place_movements_migration.rs
  - crates/trackly-app/src/dto/place_movements.rs
  - crates/trackly-app/src/services/place_movement_service.rs
  - crates/trackly-app/src/services/repair_service.rs
  - crates/trackly-app/src/services/report_service.rs
  - crates/trackly-app/src/services/device_service.rs
  - crates/trackly-app/tests/repair_accept.rs
  - crates/trackly-app/tests/repair_cancel_reject.rs
  - crates/trackly-app/tests/repair_movement_source.rs
  - crates/trackly-app/tests/repair_ws_signals.rs
  - crates/trackly-app/tests/devices_printer_link_audit.rs
  - crates/trackly-app/tests/suites/repairs.rs
  - crates/trackly-app/tests/suites/devices.rs
  - crates/trackly-app/tests/entities_support/registry.rs
  - ui/src/bindings.ts
  - ui/src/lib/movementReason.ts
  - ui/src/lib/components/MovementTimeline.svelte
  - ui/src/features/printers/PrinterDetail.svelte
  - ui/src/features/cartridges/CartridgeDetail.svelte
  - ui/src/features/places/PlaceEntityViewModal.svelte
  - ui/src/features/showcase/sections/MovementTimelineSection.svelte
  - ui/src/features/requests/RequestDetail.svelte
  - ui/scripts/check-movement-source-labels.mjs
  - ui/package.json
autonomous: true
requirements: [INT-30, NEW-3, INT-09, NEW-7, NEW-4]

must_haves:
  truths:
    - "NEW-3: перенос устройства ремонтом пишет в place_movements source='repair' и request_id = id заявки (note у строки устройства NULL); каскад картриджей при ремонтном переносе пишет source='repair', тот же request_id, note «вместе с принтером»"
    - "NEW-3: таймлайн показывает «по ремонтной заявке №N», где N — кнопка-ссылка, открывающая заявку (openRequestTarget(id, 'info')); отчёт «Перемещения» показывает «по ремонтной заявке №N» текстом"
    - "NEW-3: прежние строки ремонта (source='manual', note ровно 'по заявке №<id>' при существующей заявке request_type='repair') миграция V056 переводит в source='repair' + request_id, прочие ручные строки не трогает"
    - "INT-09: каждый вариант MovementSource (manual, act, map, workstation, group, repair) имеет явную русскую подпись и в MovementTimeline, и в отчёте; «причина не определена» остаётся только для нераспознанного токена"
    - "INT-09: гейт check-movement-source-labels.mjs в pnpm lint краснеет, если в Rust-enum появился вариант без подписи во фронтовом словаре (проверено мутацией копии реальных исходников в --selftest)"
    - "INT-30: смена USB-хоста, IP или community принтера через devices update пишет ровно одну строку audit_log action='custom:printer_connection' с payload только по изменившимся полям; IP и хост — «было/стало», community — только факт смены без значения; сохранение без изменений этих полей строки не пишет"
    - "NEW-7: repairs_link_scans после коммита шлёт RequestStatusChanged участникам заявки; открытая карточка ремонтной заявки у второго клиента перечитывает состояние ремонта по этому событию"
    - "NEW-4: создание новой ремонтной заявки на устройство (не присоединение) после коммита шлёт EmployeeScopeChanged {} (видим только Employee, без данных) — второй сотрудник того же АРМ перечитывает свои устройства и видит «Есть заявка»; Manager/Admin это событие не видят"
  artifacts:
    - path: "migrations/V056__place_movements_request_id.sql"
      provides: "колонка place_movements.request_id + перенос прежних ремонтных строк"
      contains: "PRAGMA user_version = 56;"
    - path: "crates/trackly-core/src/domain/place_movements.rs"
      provides: "MovementSource::Repair и MovementSource::ALL"
      contains: "Repair"
    - path: "ui/src/lib/movementReason.ts"
      provides: "словарь подписей источников + reasonText"
      exports: ["MOVEMENT_SOURCE_LABELS", "reasonText"]
    - path: "ui/scripts/check-movement-source-labels.mjs"
      provides: "гейт паритета Rust-enum ↔ фронтовые подписи"
    - path: "crates/trackly-app/tests/devices_printer_link_audit.rs"
      provides: "тесты INT-30"
    - path: "crates/trackly-app/tests/repair_ws_signals.rs"
      provides: "тесты NEW-4 и NEW-7"
    - path: "crates/trackly-app/tests/repair_movement_source.rs"
      provides: "тесты NEW-3: source/request_id в журнале, таймлайне и отчёте"
  key_links:
    - from: "crates/trackly-app/src/services/repair_service.rs"
      to: "place_movements.request_id"
      via: "record_repair_movement_if_applicable + cascade_place_for_printer_repair_in_tx"
      pattern: "MovementSource::Repair"
    - from: "ui/src/lib/components/MovementTimeline.svelte"
      to: "ui/src/lib/movementReason.ts"
      via: "import reasonText"
      pattern: "movementReason"
    - from: "ui/package.json"
      to: "ui/scripts/check-movement-source-labels.mjs"
      via: "lint chain"
      pattern: "check-movement-source-labels.mjs --selftest"
    - from: "ui/src/features/requests/RequestDetail.svelte"
      to: "loadRepairState"
      via: "onWsEvent request_status_changed для текущей ремонтной заявки"
      pattern: "request_status_changed"
---

<objective>
Пакет A «история и журнал»: закрыть находки аудита вехи v1.4 INT-30, NEW-3, INT-09, NEW-7, NEW-4 (точные места — .planning/v1.4-MILESTONE-AUDIT.md).

Purpose: история перемещений и журнал аудита должны отвечать на «почему переехало» и «кто подключил принтер», а второй открытый клиент — видеть изменения без перезагрузки.
Output: источник Repair с id заявки (миграция V056), подписи для всех источников + гейт паритета, аудит смены сети/хоста принтера, WS-сигналы для link_scans и создания ремонтной заявки.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@./CLAUDE.md
@.planning/v1.4-MILESTONE-AUDIT.md

<interfaces>
Факты, снятые с кодовой базы при планировании (исполнителю не нужно их искать):

- crates/trackly-core/src/domain/place_movements.rs — enum MovementSource { Manual, Act, Map, Workstation, Group } с as_str() / from_str_lenient(); токены "manual"|"act"|"map"|"workstation"|"group". Колонка place_movements.source — TEXT без CHECK, новый токен схемы не требует.
- place_movements (V040 + V046): id, entity_type, entity_id, from_place_id, from_place_path, to_place_id, to_place_path, source, note, act_id, user_id, actor_name_snapshot, batch_id, entity_label, group_id, created_at_utc. Колонки request_id НЕТ. Последняя миграция — migrations/V055__chat_mutes.sql (user_version 55). Миграции встраиваются через build.rs (новый .sql пересобирает крейт).
- crates/trackly-infra/src/repos/place_movements_sqlite.rs: NewMovement<'a> (поля как колонки), insert_in_tx (INSERT из 15 колонок), pub record_movement_if_applicable(tx, places_repo, entity_type, entity_id, before, after, source, note, act_id, user_id, now) и pub record_batch_movement_if_applicable(..., batch_id, entity_label, group_id) — обе зовут приватный record_inner. MovementRow + list-запрос (SELECT ... ORDER BY created_at_utc DESC, id DESC) около строк 355-399.
- crates/trackly-infra/src/repos/cartridges_sqlite.rs:1187 cascade_place_for_printer_in_tx(tx, printer_device_id, new_place_id, source, note, user_id, now) -> зовёт cascade_place_for_printer_batch_in_tx(..., batch_id, entity_label, group_id, act_id), который в цикле зовёт record_batch_movement_if_applicable.
- crates/trackly-app/src/services/repair_service.rs:2231-2262 — единственные две записи журнала ремонта: record_movement_if_applicable(..., MovementSource::Manual, Some(&format!("по заявке №{request_id}")), None, user_id, now) и cascade_place_for_printer_in_tx(..., MovementSource::Manual, "вместе с принтером", ...). Номер заявки = requests.id.
- repair_service.rs: finish_transition (стр. ~190) шлёт WsEvent::RequestStatusChanged { request_id, new_status, requested_by_user_id, participant_user_ids: self.participants_of(id).await }. create (стр. ~743-758): joined → RequestStatusChanged, иначе только NewRequest (Admin/Manager). link_scans (стр. 1923) пишет repo.link_scan_in_tx и возвращает self.get_state — событий нет.
- crates/trackly-app/src/dto/printer.rs: WsEvent::EmployeeScopeChanged {} — is_visible_to: только Role::Employee, без данных; entities_broadcast::send_entities_changed шлёт его рядом с EntitiesChanged.
- crates/trackly-app/tests/entities_support/registry.rs:394-416 — строки ex(REPAIR, "link_scans", "...") и ex(REPAIR, "create", "...") с причинами (Exempt-причина ≥ 40 символов). Сканер гейта ищет только broadcast_entities( — прямой ws.send других событий Exempt-строку не ломает.
- report_service.rs:1687 movement_reason(source, note, act_id, act_number, entity_type, entity_label) — match по from_str_lenient, Map → «по карте», Workstation → «назначение АРМ», None → сырой токен; SQL отчёта ~1827-1840 (pm.batch_id, pm.entity_type, pm.entity_label, ... читаются по индексам r.get(N)), вызов movement_reason ~1929.
- crates/trackly-app/src/dto/place_movements.rs MovementEntryDto (snake_case в bindings.ts, specta i64 → #[specta(type = Option<i32>)]); маппинг в place_movement_service.rs ~105-130. bindings.ts генерирует тест export_bindings.
- device_service.rs::update (стр. 1268): внутри writer-транзакции после sync_printer_row_in_tx — set_network_in_tx (стр. ~1468, ip_patch/community_patch) и set_usb_host_in_tx (стр. ~1496, usb_host_patch); затем INSERT audit_log 'update' с payload_json NULL (стр. ~1510). Таблица printers: device_id, ip_address, community, usb_host_device_id. Community — секрет: UI видит только community_configured = (community <> 'public'), printers_sqlite.rs:321 «значение community нигде не логируется».
- ui/src/lib/components/MovementTimeline.svelte:69-95 — функция reasonText(entry) (manual/act/group, иначе «причина не определена»); шаблон ~162-205 рисует act- и group-ссылки кнопками class="timeline-link". Пропсы onNavigateToPlace/onNavigateToAct/onNavigateToGroup. Вызовы: PrinterDetail.svelte:407-413, CartridgeDetail.svelte:296-302 (push(`#/acts?id=`)), PlaceEntityViewModal.svelte:319-334 (async: await push(...); onClose()), GroupPanel.svelte:242, showcase MovementTimelineSection.svelte (литералы MovementEntryDto).
- ui/src/features/requests/requestTarget.ts: openRequestTarget(requestId, 'info' | 'chat'), requestTargetHref.
- ui/src/features/requests/RequestDetail.svelte:370-386 — onWsEvent: attachments_changed → loadHistory; entities_changed (repair) → loadRepairState + loadHistory. request_status_changed здесь не обрабатывается; RequestsPage перечитывает заявку, но loadRepairState перезапускается только при смене request.version, а link_scans версию не меняет.
- ui/src/features/employee/EmployeeDevicesPage.svelte:85-93 — scheduleRefresh на employee_scope_changed | request_status_changed | chat_message.
- Образец гейт-скрипта: ui/scripts/check-history-labels.mjs (транспиляция .ts через typescript, режим --selftest с мутантами настоящего исходника). Цепочка pnpm lint — ui/package.json "lint".
- Suite-бинарники trackly-app: app_repairs (tests/suites/repairs.rs), app_devices (devices.rs), app_places, app_reports, app_gates; infra — infra_tests (crates/trackly-infra/tests/suites/infra.rs). Новый файл теста обязателен в suite строками #[path = "../<файл>.rs"] / mod <файл>; (алфавит) — иначе краснеет suite_wiring. Хелперы: tests/repair_support (seed_user, emp, manager, seed_user_group), tests/entities_support/fixture (make_test_ctx, seed_place, seed_device, drain, admin). Подписка на WS: ctx.ws_broadcast.subscribe() до мутации, try_recv после.
- Существующие тесты, завязанные на старую запись ремонта: repair_accept.rs:155 (source='manual' AND note LIKE 'по заявке №%'), repair_accept.rs:547 (note='вместе с принтером' AND source='manual'), repair_cancel_reject.rs:569-575 (note = format!("по заявке №{req}")).
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Источник Repair с id заявки в журнале и отчёте (NEW-3, серверная часть INT-09)</name>
  <files>crates/trackly-core/src/domain/place_movements.rs, migrations/V056__place_movements_request_id.sql, crates/trackly-infra/src/repos/place_movements_sqlite.rs, crates/trackly-infra/src/repos/cartridges_sqlite.rs, crates/trackly-infra/tests/place_movements_migration.rs, crates/trackly-app/src/dto/place_movements.rs, crates/trackly-app/src/services/place_movement_service.rs, crates/trackly-app/src/services/repair_service.rs, crates/trackly-app/src/services/report_service.rs, crates/trackly-app/tests/repair_accept.rs, crates/trackly-app/tests/repair_cancel_reject.rs, crates/trackly-app/tests/repair_movement_source.rs, crates/trackly-app/tests/suites/repairs.rs, ui/src/bindings.ts</files>
  <behavior>
    - core: MovementSource::Repair.as_str() == "repair"; from_str_lenient("repair") == Some(Repair); для каждого v из MovementSource::ALL from_str_lenient(v.as_str()) == Some(v); ALL содержит 6 вариантов.
    - infra (place_movements_migration.rs): после миграций у place_movements есть колонка request_id; строка source='manual', note='по заявке №<id>' при существующей заявке request_type='repair' переводится в source='repair', request_id=<id>, note=NULL; строка с note 'по заявке №<id>' без такой ремонтной заявки и строка с иным текстом примечания не трогаются; последняя непустая строка V056 — «PRAGMA user_version = 56;».
    - app (repair_movement_source.rs): accept с переносом на склад даёт строку устройства source='repair', request_id=req, note IS NULL; каскад картриджа принтера даёт source='repair', request_id=req, note='вместе с принтером'; ctx.place_movements (сервис таймлайна) отдаёт entry.source=="repair" и entry.request_id==Some(req); отчёт «Перемещения» даёт reason == format!("по ремонтной заявке №{req}") для устройства и «по ремонтной заявке №{req} · вместе с принтером» для картриджа.
    - app (report_service unit-тест в #[cfg(test)]): movement_reason для каждого v из MovementSource::ALL не возвращает сырой токен v.as_str(); Map → «по карте», Workstation → «назначение АРМ».
  </behavior>
  <action>
Сначала тесты (RED), затем реализация.

1) core: добавить вариант MovementSource::Repair (док: «перемещение по ремонтной заявке, Phase 41.4; id заявки — place_movements.request_id») в enum, as_str ("repair"), from_str_lenient; добавить pub const ALL: [MovementSource; 6] со всеми вариантами в порядке объявления; обновить шапку модуля (там сказано «четыре токена»). Юнит-тесты в том же файле.

2) Миграция migrations/V056__place_movements_request_id.sql (аддитивная, без перестройки таблиц — см. память про FK OFF в refinery): ALTER TABLE place_movements ADD COLUMN request_id INTEGER NULL REFERENCES requests(id) ON DELETE SET NULL; частичный индекс idx_place_movements_request ON place_movements(request_id) WHERE request_id IS NOT NULL; перенос прежних ремонтных строк одним UPDATE: source='repair', request_id = id найденной заявки, note=NULL для строк entity_type='device', source='manual', у которых существует requests r с r.request_type='repair' и note = 'по заявке №' || r.id (точное равенство, не LIKE — ручная пометка пользователя с другим текстом не задевается). Картриджные строки «вместе с принтером» прошлого не переносить: по ним заявку не восстановить однозначно — отметить это в шапке миграции. Последняя строка файла — PRAGMA user_version = 56;. После добавления файла убедиться, что build.rs пересобрал embed (user_version в тестовой БД = 56).

3) infra place_movements_sqlite.rs: в NewMovement поле request_id: Option<i64>; INSERT в insert_in_tx — 16 колонок; в record_inner параметр request_id; существующие record_movement_if_applicable / record_batch_movement_if_applicable передают None (их сигнатуры не меняются — у них много вызывающих). Новый pub fn record_repair_movement_if_applicable(tx, places_repo, entity_type, entity_id, before, after, request_id: i64, note: Option<&str>, user_id, now) — source жёстко MovementSource::Repair, request_id Some. MovementRow + list-запрос читают request_id (индексы r.get сдвинуть аккуратно, created_at_utc остаётся последним).
   cartridges_sqlite.rs: тело cascade_place_for_printer_batch_in_tx вынести в приватный cascade_inner с дополнительным request_id: Option<i64>, который пишет журнал через record_inner с request_id (оба файла в одном крейте trackly-infra, поэтому достаточно сделать record_inner pub(crate)); существующие pub cascade-функции передают None; новый pub fn cascade_place_for_printer_repair_in_tx(tx, printer_device_id, new_place_id, request_id, note, user_id, now) — source Repair, тот же debug_assert про new_place_id Some, что у cascade_place_for_printer_in_tx.

4) app: MovementEntryDto.request_id: Option<i64> с #[specta(type = Option<i32>)] и doc-комментарием (id ремонтной заявки, Some только у source="repair"); обновить doc поля source (перечень токенов + "repair"); маппинг в place_movement_service.rs; юнит-тест sample() в dto дополнить request_id: None. repair_service.rs:2236-2261 — заменить запись устройства на record_repair_movement_if_applicable(..., request_id, None, user_id, now), каскад — на cascade_place_for_printer_repair_in_tx(..., request_id, "вместе с принтером", ...). Импорт MovementSource в repair_service, если станет неиспользуемым, убрать (clippy -D warnings).
   report_service.rs: в SQL отчёта читать pm.request_id, передать в movement_reason новым параметром request_id: Option<i64>; ветка Some(MovementSource::Repair) → «по ремонтной заявке №{id}» (без id — «по ремонтной заявке»), при непустом note добавлять « · {note}». Match остаётся исчерпывающим (без `_`), чтобы новый вариант enum ломал компиляцию. Добавить #[cfg(test)]-тест по MovementSource::ALL из behavior.

5) Обновить старые тесты под новое поведение: repair_accept.rs:155 → source='repair' AND request_id=<req> AND note IS NULL; repair_accept.rs:547 → source='repair' AND note='вместе с принтером' AND request_id=<req>; repair_cancel_reject.rs:569-575 → вместо note сравнивать source='repair' AND request_id=req (from/to проверки сохранить). Новый файл tests/repair_movement_source.rs (данные вымышленные: «Кабинет 12», «Склад А», «Сидоров С.С.») с behavior-сценариями, подключить в tests/suites/repairs.rs по алфавиту. Миграционный тест — в существующий crates/trackly-infra/tests/place_movements_migration.rs (вставить строки на версии 55 нельзя через публичный run — поэтому проверяй backfill через применение SQL V056 к БД, мигрированной до 55, если в infra есть такой хелпер; иначе выполни перенос-UPDATE текстом файла V056 (include_str!) на подготовленных строках после полного прогона, предварительно вернув строки в старый вид — главное, чтобы проверялся НАСТОЯЩИЙ текст V056, а не копия).

6) Перегенерировать ui/src/bindings.ts: TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test export_bindings. Литералы MovementEntryDto во фронте (showcase) правит Task 2 — svelte-check до Task 2 может краснеть только на них.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && cargo test -p trackly-core place_movements && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-infra --test infra_tests place_movements && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib report_service && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_repairs && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_reports report_movements && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_places place_movements && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test export_bindings && tail -n 1 migrations/V056__place_movements_request_id.sql | grep -qx 'PRAGMA user_version = 56;'</automated>
  </verify>
  <done>Ремонтный перенос пишет source='repair' + request_id; прежние ремонтные строки перенесены миграцией V056; таймлайн-DTO отдаёт request_id; отчёт подписывает «по ремонтной заявке №N»; movement_reason покрыт тестом по всем вариантам; старые тесты ремонта обновлены и зелёные; bindings.ts содержит request_id в MovementEntryDto. cargo test запускались строго по одному.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Подписи всех источников в таймлайне, ссылка на ремонтную заявку и гейт паритета (INT-09, UI NEW-3)</name>
  <files>ui/src/lib/movementReason.ts, ui/src/lib/components/MovementTimeline.svelte, ui/src/features/printers/PrinterDetail.svelte, ui/src/features/cartridges/CartridgeDetail.svelte, ui/src/features/places/PlaceEntityViewModal.svelte, ui/src/features/showcase/sections/MovementTimelineSection.svelte, ui/scripts/check-movement-source-labels.mjs, ui/package.json</files>
  <behavior>
    - reasonText: manual → «вручную» / «вручную · {note}»; act → «актом №{n}» / «актом»; map → «по карте»; workstation → «назначение АРМ» (совпадают с отчётом); group → прежние «перенос группы» / «в составе группы «{имя}»»; repair → «по ремонтной заявке №{request_id}» (+ « · {note}» при непустом note), без request_id — «по ремонтной заявке»; неизвестный токен → «причина не определена».
    - Гейт: множество ключей MOVEMENT_SOURCE_LABELS == множество токенов as_str() в impl MovementSource == множество вариантов enum == множество вариантов в MovementSource::ALL; reasonText для каждого токена не равен «причина не определена»; для «garbage» равен.
    - --selftest краснеет на мутантах НАСТОЯЩИХ исходников (в памяти): (а) из копии movementReason.ts удалён ключ map; (б) в копию place_movements.rs добавлен вариант Survey с токеном "survey"; (в) в копии movementReason.ts подпись workstation заменена возвратом в fallback; (г) из ALL в копии Rust убран Repair. Нетронутые исходники — зелёные (разностная проверка, якоря мутаций уникальны: replace только при matches.count == 1, иначе сам selftest падает).
  </behavior>
  <action>
1) Новый модуль ui/src/lib/movementReason.ts: экспорт type MovementSourceToken (union 'manual'|'act'|'map'|'workstation'|'group'|'repair'), const MOVEMENT_SOURCE_LABELS: Record<MovementSourceToken, (entry: MovementEntryDto) => string> и function reasonText(entry: MovementEntryDto): string (поиск по словарю через Object.hasOwn, иначе «причина не определена»). Логику manual/act/group перенести из MovementTimeline.svelte:69-95 без изменения текстов; map/workstation — тексты как в report_service.rs (единые подписи, per INT-09). Тип MovementEntryDto импортировать как type-only из bindings (memory: bindings.ts в рантайме не импортировать).

2) MovementTimeline.svelte: удалить локальную reasonText, импортировать из $lib/movementReason; новый опциональный проп onNavigateToRequest?: (requestId: number) => void | Promise<void>; в шаблоне новая ветка перед {:else} — при entry.source === 'repair' && entry.request_id !== null && onNavigateToRequest рисовать «по ремонтной заявке №<button class="timeline-link">{request_id}</button>» и, при непустом note, « · {note}» отдельным span (по образцу act-ветки, без {@html}); иначе — текст reasonText. Обновить комментарий про «Unrecognized/future source values (map, workstation…)».

3) Проводка ссылки: PrinterDetail.svelte и CartridgeDetail.svelte — onNavigateToRequest={(id) => openRequestTarget(id, 'info')}; PlaceEntityViewModal.svelte — async, как соседние: openRequestTarget(id, 'info'), затем onClose(). Импорт openRequestTarget из features/requests/requestTarget (путь относительно файла). GroupPanel не трогать (строки группы не бывают repair). Showcase MovementTimelineSection.svelte: добавить request_id: null во все литералы и одну демонстрационную строку source 'repair' с request_id (вымышленные места/ФИО, например «Петров П.П.»).

4) Гейт ui/scripts/check-movement-source-labels.mjs по образцу check-history-labels.mjs (TAG «[check-movement-source-labels]», транспиляция movementReason.ts через typescript, импорт bindings вырезается или type-only исчезает при транспиляции): читает crates/trackly-core/src/domain/place_movements.rs, вырезает блок pub enum MovementSource { … } (варианты), тело impl MovementSource → as_str (токены "…" после =>) и массив ALL; проверки из behavior; пустой разбор любого множества — красный (гейт не зелёный на нулевом измерении). --selftest — четыре мутанта из behavior на копиях реальных файлов в памяти, каждый обязан дать красный, исходник — зелёный; файлы на диске не трогать. Подключить в ui/package.json в конец цепочки "lint": node scripts/check-movement-source-labels.mjs --selftest && node scripts/check-movement-source-labels.mjs. Проверить вживую: временно удалить ключ map в рабочем movementReason.ts → pnpm lint красный → вернуть.

5) Сборка: pnpm --dir ui build (ui/dist нужен Rust-тестам и LAN-режиму).
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && node ui/scripts/check-movement-source-labels.mjs --selftest && node ui/scripts/check-movement-source-labels.mjs && pnpm --dir ui lint && pnpm --dir ui check && pnpm --dir ui build</automated>
  </verify>
  <done>Все шесть источников подписаны явно и одинаково с отчётом; ремонтная строка таймлайна ведёт на заявку из карточек принтера, картриджа и устройства; гейт в pnpm lint зелёный на исходниках и красный на каждом из четырёх мутантов; pnpm lint, svelte-check (0 ошибок) и build зелёные.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Аудит смены сети/хоста принтера и WS-сигналы ремонта (INT-30, NEW-7, NEW-4)</name>
  <files>crates/trackly-app/src/services/device_service.rs, crates/trackly-app/src/services/repair_service.rs, crates/trackly-app/tests/entities_support/registry.rs, crates/trackly-app/tests/devices_printer_link_audit.rs, crates/trackly-app/tests/repair_ws_signals.rs, crates/trackly-app/tests/suites/devices.rs, crates/trackly-app/tests/suites/repairs.rs, ui/src/features/requests/RequestDetail.svelte</files>
  <behavior>
    - INT-30 (devices_printer_link_audit.rs): смена только IP → одна строка audit_log entity_type='device', action='custom:printer_connection', payload {"ip_address":{"from":old,"to":new}} без ключей usb_host_device_id/community; смена только USB-хоста → payload {"usb_host_device_id":{"from":a,"to":b}}; снятие хоста (Some → None) тоже фиксируется; смена только community → payload {"community":{"changed":true}}, в payload_json и во всей строке audit_log нет подстроки нового значения community (например «s3cr3t-comm»); смена IP+community одним сохранением → одна строка с двумя ключами; сохранение принтера с теми же IP/хостом/community (и правка только имени) → 0 строк custom:printer_connection; обычная строка 'update' пишется как раньше.
    - NEW-7 (repair_ws_signals.rs): после link_scans в канале есть RequestStatusChanged { request_id = заявка захода, participant_user_ids содержит участников }; событие не уходит, если link_scans вернул ошибку валидации (чужое вложение).
    - NEW-4: create ремонтной заявки на устройство (новая, не присоединение) → в канале NewRequest и EmployeeScopeChanged {}; EmployeeScopeChanged.is_visible_to(emp(..)) == true, для manager(..) == false; присоединение (joined) EmployeeScopeChanged не шлёт (флаг «Есть заявка» уже стоял); заявка без устройства — не шлёт.
  </behavior>
  <action>
1) INT-30, device_service.rs::update внутри той же writer-транзакции: перед блоком set_network_in_tx/set_usb_host_in_tx (стр. ~1462) прочитать текущую строку printers (ip_address, community, usb_host_device_id) по device_id через tx.query_row(...).optional() — строки может не быть (не принтер / только что создана sync_*); после обоих блоков прочитать снова. Сравнить: ip_address и usb_host_device_id — «было/стало» значениями (from/to, null допустим); community — только факт изменения ({"changed": true}), значение не писать никуда (per printers_sqlite.rs:321; в UI community не показывается, только community_configured — значит это секрет). Если изменилось хотя бы одно поле — один INSERT в audit_log: entity_type 'device', entity_id id, action 'custom:printer_connection', user_id user_id_opt, before_json/after_json NULL, payload_json — JSON-объект только с изменившимися ключами, created_at_utc now. Вынести сравнение в чистую функцию printer_connection_diff(before, after) -> Option<serde_json::Value> с #[cfg(test)]-юнит-тестами (неизменённое → None). Читать фактические значения строки, а не патч: set_network_in_tx сам подставляет 'public' при пустом IP — сравнение по строке это учитывает.

2) NEW-7, repair_service.rs::link_scans: после .await writer'а (не внутри замыкания — D-04/D-15) отправить RequestStatusChanged так же, как finish_transition: прочитать заявку (request_repo.get через readers + spawn_blocking), participants_of(request_id), ws.send(WsEvent::RequestStatusChanged{...}). Удобно вынести из finish_transition приватный notify_request_status(request_id) -> Result<RequestDto, AppError> и звать его из обоих мест (finish_transition сохраняет своё поведение). Обновить причину строки ex(REPAIR, "link_scans", ...) в registry.rs: места/устройства/группы не меняются, сигнал отдельным WsEvent RequestStatusChanged участникам заявки (≥ 40 символов, без упоминания broadcast_entities).
   RequestDetail.svelte (обработчик onWsEvent ~370-386): добавить ветку ev.type === 'request_status_changed' && ev.requestId === id && request?.requestType === 'repair' → void loadRepairState(id). Иначе событие у второго клиента перечитает заявку, но не состояние ремонта (link_scans не меняет version).

3) NEW-4, repair_service.rs::create (~743-758): в ветке !joined, когда заявка создана с устройством (dto.device_id / new.printer_device_id Some — взять значение до move в замыкание), после NewRequest отправить WsEvent::EmployeeScopeChanged {} — событие видно только Employee и не несёт данных, сотрудник перечитывает свои устройства серверной командой со своей областью, поэтому чужим ничего не утекает (не расширять видимость NewRequest/RequestStatusChanged). Обновить причину ex(REPAIR, "create", ...) в registry.rs: упомянуть EmployeeScopeChanged для обновления пометки «Есть заявка» у сотрудников.

4) Тесты: devices_printer_link_audit.rs (suite devices.rs) — по образцу devices_usb_host.rs (тот же способ создать принтер и хост-устройство и звать update), community-значение вымышленное; repair_ws_signals.rs (suite repairs.rs) — по образцу entities_changed_repairs.rs: make_test_ctx, подписка ctx.ws_broadcast.subscribe() + drain до мутации, сбор событий try_recv после; сотрудники «Иванов И.И.», «Петров П.П.» через seed_user + seed_user_group на одно устройство; вложение заявки для link_scans создать тем же путём, что существующие тесты link_scans (grep link_scans в tests/repair_send_receive.rs). Имена тестов НЕ начинать с scenario_ (префикс зарезервирован слоем (2) реестрового гейта).
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib device_service && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_devices && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_repairs && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_gates && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check && pnpm --dir ui lint && pnpm --dir ui check && pnpm --dir ui build</automated>
  </verify>
  <done>Смена IP/USB-хоста/community пишет одну строку custom:printer_connection только по изменившимся полям, без значения community; неизменённые поля строк не дают; link_scans шлёт RequestStatusChanged и карточка ремонта у второго клиента перечитывается; новая ремонтная заявка на устройство шлёт EmployeeScopeChanged (только Employee); реестр entities_changed_gate зелёный с обновлёнными причинами; app_devices, app_repairs, app_gates, clippy -D warnings, fmt --check, pnpm lint, svelte-check, build — зелёные.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| сервер → WS-клиенты | события фан-аутятся по is_visible_to; лишнее событие = утечка факта о чужой заявке |
| сервис → audit_log | журнал читают администраторы и бэкапы; секрет SNMP туда попадать не должен |
| БД → UI таймлайна | note/request_id из БД рендерятся в DOM |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-hed-01 | Information Disclosure | repair_service::create → EmployeeScopeChanged | mitigate | событие без полей, видимость только Role::Employee (существующий is_visible_to); клиент перечитывает данные своей серверной командой с BOLA-скоупом; тест is_visible_to emp=true / manager=false |
| T-hed-02 | Information Disclosure | repair_service::link_scans → RequestStatusChanged | mitigate | тот же конструктор, что в finish_transition (автор + participants + Admin/Manager); видимость не расширяется |
| T-hed-03 | Information Disclosure | device_service::update → audit_log custom:printer_connection | mitigate | community только {"changed":true}; тест на отсутствие значения community в строке аудита |
| T-hed-04 | Tampering | place_movements.source / request_id | mitigate | source и request_id задаёт только сервер (record_repair_movement_if_applicable, Repair жёстко); клиент их не передаёт |
| T-hed-05 | Tampering (XSS) | MovementTimeline note/request_id | mitigate | текстовая интерполяция Svelte, без {@html}; request_id — number в onclick |
| T-hed-06 | Tampering | миграция V056 backfill | mitigate | точное равенство note = 'по заявке №' \|\| r.id и r.request_type='repair'; тест, что чужие ручные строки не меняются |
</threat_model>

<verification>
- Только один cargo test одновременно; команды скоуплены по suite-бинарникам; при exit 69 на линковке — префикс DEVELOPER_DIR=/Library/Developer/CommandLineTools. Команды `timeout` на машине нет — не оборачивать.
- ui/dist собран (pnpm --dir ui build) до Rust-тестов trackly-app, после изменений фронта — пересобран.
- Гейт check-movement-source-labels проверен мутацией копии реальных исходников (--selftest) и одной ручной порчей рабочего файла с откатом.
- Приватность: перед каждым коммитом git diff --cached просмотрен — только вымышленные имена («Иванов И.И.», «Петров П.П.», «Сидоров С.С.»), никаких реальных реквизитов; pre-commit гейт приватности зелёный.
- Коммиты атомарные по задачам, сообщение оканчивается строкой «Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>».
</verification>

<success_criteria>
- NEW-3: история и отчёт отличают ремонт от ручного переноса; номер заявки в таймлайне — ссылка.
- INT-09: ни один известный источник не даёт «причина не определена»; паритет Rust ↔ фронт удерживается гейтом в pnpm lint.
- INT-30: «кто и когда сменил IP/хост/community принтера» есть в audit_log без утечки community.
- NEW-7: привязка сканов обновляет открытую карточку ремонта у второго клиента.
- NEW-4: второй сотрудник того же АРМ получает пометку «Есть заявка» без перезагрузки, другие роли/сотрудники лишних данных не получают.
- Все указанные verify-команды зелёные.
</success_criteria>

<output>
Создать `.planning/quick/261121-hed-history-journal/261121-hed-SUMMARY.md` по завершении.
</output>
