---
quick_id: 260826-rbe
slug: extend-d-28-subtree-place-filter-to-cart
phase: 260826-rbe
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-app/src/services/report_service.rs
  - crates/trackly-app/tests/report_place_subtree.rs
autonomous: true
requirements: [D-28]
must_haves:
  truths:
    - "На вкладке «Картриджи» раздела «Отчёты» фильтр по месту (PlacePicker) сужает оба builder-а расхода/заправок (query_cartridge_audit, вызывается из list_cartridge_consumption/list_cartridge_refills) и оба снимка (query_cartridge_snapshot, вызывается из list_cartridge_in_use/list_cartridge_in_stock) до выбранного места И всех вложенных мест — не только точного совпадения place_id (D-28)"
    - "Счётчики вкладок домена «Картриджи» (get_report_counts(\"cartridges\") → count_cartridge_audit_inner/count_cartridge_snapshot_inner) отражают тот же subtree-inclusive фильтр, что и сами списки — счётчик и список никогда не расходятся"
    - "На вкладке «Заявки» раздела «Отчёты» фильтр по месту сужает список заявок (query_requests_inner, вызывается из list_requests_all/open/in_progress/completed) по месту ПРИНТЕРА заявки (d.place_id через LEFT JOIN devices), включая вложенные места под выбранным"
    - "Счётчики вкладок домена «Заявки» (get_report_counts(\"requests\") → count_requests_inner) отражают тот же subtree-inclusive фильтр по месту принтера"
    - "Фильтр «на складе» (D-11.2/D-11.4, ReportFilter.is_storage) продолжает работать корректно при ОДНОВРЕМЕННОМ выборе с фильтром по месту на всех затронутых доменах — новая CTE подтрева не перезаписывает существующую CTE «на складе» и наоборот (обе используют merge-safe with_prefix композицию)"
    - "Существующее поведение домена «Устройства» (query_acts_inner/query_device_snapshot/count_acts_inner/count_device_snapshot) не регрессирует — все 6 существующих тестов report_place_subtree.rs остаются зелёными без изменений в их коде"
  artifacts:
    - path: "crates/trackly-app/src/services/report_service.rs"
      provides: "D-28 subtree-inclusive place-фильтр применён в query_cartridge_audit / query_cartridge_snapshot / count_cartridge_audit_inner / count_cartridge_snapshot_inner (алиас c.place_id) и в query_requests_inner / count_requests_inner (алиас d.place_id через LEFT JOIN devices d ON d.id = r.printer_device_id)"
      contains: "c.place_id IN (SELECT id FROM subtree)"
    - path: "crates/trackly-app/tests/report_place_subtree.rs"
      provides: "5 новых интеграционных тестов, покрывающих домены «Картриджи» и «Заявки» — каждый проверяет и захват вложенного места (root/ancestor filter), и исключение соседнего поддерева, точными счётчиками (не просто non-empty)"
      contains: "cartridge_consumption_report_root_place_filter"
  key_links:
    - from: "ReportFilter.place_id"
      to: "query_cartridge_audit / query_cartridge_snapshot / count_cartridge_audit_inner / count_cartridge_snapshot_inner — c.place_id IN (SELECT id FROM subtree)"
      via: "merge-safe with_prefix CTE-композиция (if with_prefix.is_empty() {...} else {...}), идентичная уже работающему паттерну storage_cte в query_acts_inner"
      pattern: "c\\.place_id IN \\(SELECT id FROM subtree\\)"
    - from: "ReportFilter.place_id"
      to: "query_requests_inner / count_requests_inner — d.place_id IN (SELECT id FROM subtree)"
      via: "LEFT JOIN devices d ON d.id = r.printer_device_id (уже существует в query_requests_inner для printer_name/printer_place; ДОБАВЛЯЕТСЯ в count_requests_inner, где join сегодня отсутствует)"
      pattern: "d\\.place_id IN \\(SELECT id FROM subtree\\)"
    - from: "ReportFilters.svelte PlacePicker (все 3 таба: Устройства/Картриджи/Заявки)"
      to: "ReportFilter.place_id (общий DTO для всех доменов)"
      via: "фронтенд не меняется в этой задаче — контрол уже рендерится на всех табах и уже шлёт place_id; фикс целиком на бэкенде, который теперь его читает"
      pattern: "place_id"
---

<objective>
Распространить D-28 (subtree-inclusive фильтр по месту в отчётах) с домена «Устройства» на домены
«Картриджи» и «Заявки». Сейчас `ReportFilters.svelte` рендерит PlacePicker на всех трёх табах
раздела «Отчёты», но `report_service.rs` читает `ReportFilter.place_id` только в
`query_acts_inner`/`query_device_snapshot` (+ их count-парах) — на «Картриджи» и «Заявки» контрол
молча ничего не делает. Пользователь явно выбрал вариант «доделать бэкенд» (не прятать контрол).

Затрагиваются 6 SQL-builder-ов в `report_service.rs`:
- `query_cartridge_audit` / `query_cartridge_snapshot` — алиас `c.place_id` (у cartridges уже
  есть своя FK на places, см. V038).
- `count_cartridge_audit_inner` / `count_cartridge_snapshot_inner` — сегодня НЕ объявляют
  `with_prefix` вообще (единственные из шести, где его пока нет).
- `query_requests_inner` / `count_requests_inner` — у `requests` нет своего `place_id`, фильтр
  идёт по месту ПРИНТЕРА заявки (`d.place_id` через join на `devices`); в `query_requests_inner`
  такой join уже есть (для колонки «Принтер / Место»), в `count_requests_inner` его нет и его
  нужно добавить.

Каждый новый CTE-блок обязан использовать merge-safe композицию `with_prefix` (`if
with_prefix.is_empty() {...} else {...}`), а не безусловную перезапись — иначе одновременный выбор
места И фильтра «на складе» (`is_storage`, D-11.2/D-11.4) на одном и том же алиасе тихо ломает один
из двух CTE. Три из шести функций (`query_cartridge_audit`, `query_cartridge_snapshot`,
`query_requests_inner`) уже имеют `is_storage`-блок, но он СЕГОДНЯ не merge-safe (безусловная
перезапись `with_prefix = format!(...)`) — этот блок тоже правится попутно.

Purpose: PlacePicker на «Картриджи»/«Заявки» — реальный, работающий UI-контрол, а не visual dead
end; фильтр по месту (базовый сценарий этой вехи — «Карта и осмысленное размещение») работает
одинаково на всех трёх отчётных доменах.

Output: report_service.rs — 6 функций с новым/дополненным subtree-фильтром;
report_place_subtree.rs — 5 новых тестов (root-захват + sibling-исключение, точные счётчики) поверх
5 новых builder-ов (query_requests_inner уже частично тестировался косвенно, но не по D-28).
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md

<interfaces>
Перечитать `crates/trackly-app/src/services/report_service.rs` перед правкой — номера строк ниже
ориентировочные (~1030-1900 на момент планирования), сместятся после первой правки этого же файла.

РЕФЕРЕНСНЫЙ (уже работающий, менять НЕ надо) блок из `query_acts_inner` — образец
merge-safe-совместимого места фильтра, скопированного в него из `is_storage`-блока НИЖЕ по коду:

  // D-28: subtree-inclusive place filter
  if let Some(place_id) = filter.place_id {
      let idx = next_idx(&owned_params);
      owned_params.push(Box::new(place_id));
      with_prefix.push_str(&format!(
          "WITH RECURSIVE subtree(id) AS ( \
               SELECT id FROM places WHERE id = ?{idx} AND deleted_at_utc IS NULL \
               UNION ALL \
               SELECT p.id FROM places p JOIN subtree s ON p.parent_id = s.id \
               WHERE p.deleted_at_utc IS NULL \
           ) "
      ));
      clauses.push("a.place_id IN (SELECT id FROM subtree)".to_string());
  }

Заметьте: этот КОНКРЕТНЫЙ блок в query_acts_inner делает безусловный push, потому что он в этой
функции идёт ПЕРВЫМ (with_prefix точно пуст в этот момент) — безопасно только из-за порядка. В
шести функциях, которые правит этот план, порядок гарантировать нельзя (is_storage-блок может
остаться или уже существовать рядом), поэтому ВЕЗДЕ, где план добавляет этот блок, использовать
merge-safe форму — идентичную уже работающему `storage_cte`-блоку из ТОЙ ЖЕ query_acts_inner чуть
ниже:

  if with_prefix.is_empty() {
      with_prefix = format!("WITH RECURSIVE {storage_cte}");
  } else {
      with_prefix = format!("{}, {storage_cte}", with_prefix.trim_end());
  }

Текущие (до правки) сигнатуры шести затрагиваемых функций:

  fn query_cartridge_audit(conn: &rusqlite::Connection, filter: &ReportFilter,
      ts_from: Option<i64>, ts_to: Option<i64>, actions: &[&str]) -> Result<ReportResponse, AppError>
  // алиас c.place_id; JOIN cartridges c / cartridge_models m; уже есть is_storage-блок
  // (НЕ merge-safe: `with_prefix = format!("WITH RECURSIVE {storage_cte}");` без if/else).

  fn query_cartridge_snapshot(conn: &rusqlite::Connection, filter: &ReportFilter,
      default_status_name: &str) -> Result<ReportResponse, AppError>
  // тот же алиас/join, тот же не-merge-safe is_storage-блок.

  fn count_cartridge_audit_inner(conn: &rusqlite::Connection, filter: &ReportFilter,
      ts_from: Option<i64>, ts_to: Option<i64>, actions: &[&str]) -> Result<i64, AppError>
  // НЕ объявляет with_prefix вообще; НЕ читает filter.is_storage (существующий, вне-скоупный
  // гэп — трогать не нужно). sql = "SELECT COUNT(*) FROM audit_log al JOIN cartridges c ...".

  fn count_cartridge_snapshot_inner(conn: &rusqlite::Connection, filter: &ReportFilter,
      default_status_name: &str) -> Result<i64, AppError>
  // та же ситуация; sql = "SELECT COUNT(*) FROM cartridges c JOIN cartridge_models m ...".

  fn query_requests_inner(conn: &rusqlite::Connection, ts_from: Option<i64>, ts_to: Option<i64>,
      status_filter: Option<&str>, exclude_ad_register: bool,
      category_filter: Option<&[String]>, is_storage: Option<bool>) -> Result<ReportResponse, AppError>
  // loose params (НЕ &ReportFilter). Уже джойнит LEFT JOIN devices d ON d.id = r.printer_device_id
  // (для колонки «Принтер / Место»). is_storage-блок фильтрует на d.place_id, НЕ merge-safe.
  // 4 вызова из list_requests_all/open/in_progress/completed, каждый передаёт
  // filter.is_storage последним аргументом.

  fn count_requests_inner(conn: &rusqlite::Connection, ts_from: Option<i64>, ts_to: Option<i64>,
      status_filter: Option<&str>, exclude_ad_register: bool,
      category_filter: Option<&[String]>) -> Result<i64, AppError>
  // НЕТ join на devices вообще; НЕТ is_storage-параметра; sql =
  // "SELECT COUNT(*) FROM requests r WHERE {where_clause}". 4 вызова внутри
  // get_report_counts()'s domain == "requests" ветки.

`ReportFilter` (crates/trackly-app/src/dto/reports.rs) — общий DTO для всех доменов, все поля
`Option`, уже несёт `pub place_id: Option<i64>` (документирован как D-28 subtree-inclusive) — новых
полей DTO НЕ добавлять, поле уже существует и уже долетает через оба транспорта (Tauri/HTTP)
без изменений вне report_service.rs.

`next_idx(params: &[Box<dyn ToSql>]) -> usize` — хелпер для позиционных `?N`, уже используется
везде в файле, не менять.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Домен «Картриджи» — subtree-фильтр по месту в 4 builder-ах</name>
  <files>crates/trackly-app/src/services/report_service.rs</files>
  <action>
Перечитать report_service.rs (номера строк сместились с момента планирования).

В `query_cartridge_audit` И `query_cartridge_snapshot` (обе функции, идентичная правка в каждой):
вставить новый блок `if let Some(place_id) = filter.place_id { ... }` СРАЗУ ПЕРЕД существующим
блоком `if let Some(want_storage) = filter.is_storage { ... }`. Новый блок — merge-safe копия
референсного `D-28`-блока из `query_acts_inner` (см. interfaces): бинд `place_id` через
`next_idx(&owned_params)`/`owned_params.push`, построить `subtree(id) AS (...)` CTE (та же
рекурсия по `places.parent_id`, тот же `deleted_at_utc IS NULL` фильтр), скомпоновать в
`with_prefix` через форму `if with_prefix.is_empty() { with_prefix = format!("WITH RECURSIVE
{subtree_cte}"); } else { with_prefix = format!("{}, {subtree_cte}", with_prefix.trim_end()); }`
(НЕ безусловное присваивание), и запушить клаузу `c.place_id IN (SELECT id FROM subtree)`.

Затем ИСПРАВИТЬ существующий `is_storage`-блок сразу под ним в ОБЕИХ функциях: сегодня он делает
`with_prefix = format!("WITH RECURSIVE {storage_cte}");` безусловно (перезаписывая только что
добавленный subtree-CTE, если оба фильтра активны одновременно) — заменить на ту же merge-safe
`if with_prefix.is_empty() {...} else {...}` форму, что и новый place-блок.

В `count_cartridge_audit_inner` И `count_cartridge_snapshot_inner` (обе функции): добавить `let
mut with_prefix = String::new();` рядом с существующими `clauses`/`owned_params` (сегодня
переменная отсутствует в обеих). Добавить тот же merge-safe `place_id`-блок (клауза
`c.place_id IN (SELECT id FROM subtree)`) — merge-safe форма нужна для консистентности/будущей
безопасности, даже если сегодня `with_prefix` в этих двух функциях всегда пуст на момент блока.
НЕ добавлять обработку `filter.is_storage` в эти две count-функции — это существующий, вне-скоупный
для этой задачи гэп (is_storage сегодня не читается ни в одной из них), трогать не нужно. Изменить
оба `sql = format!(...)` так, чтобы `{with_prefix}` шёл первым токеном строки (сегодня
`"SELECT COUNT(*) FROM audit_log al JOIN cartridges c ..."` и `"SELECT COUNT(*) FROM cartridges c
JOIN cartridge_models m ..."` — оба без `{with_prefix}` вовсе).
  </action>
  <verify>
    <automated>cargo check -p trackly-app 2>&1 | tail -60</automated>
  </verify>
  <done>report_service.rs компилируется (cargo check зелёный); все 4 cartridge-builder-а читают filter.place_id через merge-safe with_prefix; существующий is_storage-блок в query_cartridge_audit/query_cartridge_snapshot больше не может тихо перезаписать место-CTE.</done>
</task>

<task type="auto">
  <name>Task 2: Домен «Заявки» — subtree-фильтр по месту принтера в query_requests_inner / count_requests_inner</name>
  <files>crates/trackly-app/src/services/report_service.rs</files>
  <action>
Добавить в сигнатуру `query_requests_inner` новый ПОСЛЕДНИЙ параметр `place_id: Option<i64>` (после
существующего `is_storage: Option<bool>`). Внутри тела функции вставить новый блок `if let
Some(place_id) = place_id { ... }` СРАЗУ ПЕРЕД существующим `if let Some(want_storage) = is_storage
{ ... }` — та же merge-safe CTE-композиция, что в Task 1, но клауза `d.place_id IN (SELECT id FROM
subtree)`, НЕ `r.place_id` (у `requests` нет собственного `place_id`; фильтр — по месту принтера
заявки, через уже существующий в этой функции `LEFT JOIN devices d ON d.id = r.printer_device_id`
— тот же приём, которым уже фильтрует существующий is_storage-блок на `d.place_id`). Затем
исправить этот существующий `is_storage`-блок на merge-safe форму (сегодня безусловная
перезапись, как в Task 1).

Обновить все 4 вызова `query_requests_inner(...)` внутри методов `list_requests_all`,
`list_requests_open`, `list_requests_in_progress`, `list_requests_completed` — каждый вызов сегодня
заканчивается аргументом `filter.is_storage,` — добавить `filter.place_id,` новым последним
аргументом после него.

Добавить в сигнатуру `count_requests_inner` новый последний параметр `place_id: Option<i64>`.
Добавить `let mut with_prefix = String::new();`. Добавить тот же merge-safe `place_id`-блок
(клауза `d.place_id IN (SELECT id FROM subtree)`) — здесь `is_storage` в этой функции сегодня
не существует вовсе (вне-скоупный гэп, не трогать). Изменить SQL: сегодня `sql = format!("SELECT
COUNT(*) FROM requests r WHERE {where_clause}")` — заменить на `"{with_prefix}SELECT COUNT(*) FROM
requests r LEFT JOIN devices d ON d.id = r.printer_device_id WHERE {where_clause}"` — JOIN должен
быть добавлен, сегодня его в этой функции нет вообще (в отличие от query_requests_inner).

Обновить все 4 вызова `count_requests_inner(...)` внутри `get_report_counts`'s `domain ==
"requests"` ветки — каждый вызов сегодня заканчивается аргументом `category_filter,` — добавить
`filter.place_id,` новым последним аргументом после него.
  </action>
  <verify>
    <automated>cargo check -p trackly-app 2>&1 | tail -60</automated>
  </verify>
  <done>report_service.rs компилируется; query_requests_inner и count_requests_inner оба принимают place_id и фильтруют по d.place_id (месту принтера) через merge-safe subtree-CTE; count_requests_inner теперь джойнит devices; все 8 вызовных сайтов (4+4) обновлены новым аргументом.</done>
</task>

<task type="auto">
  <name>Task 3: Тесты для доменов «Картриджи»/«Заявки» + полный регресс</name>
  <files>crates/trackly-app/tests/report_place_subtree.rs</files>
  <action>
Перечитать report_place_subtree.rs целиком (уже приведён выше в контексте планирования; Task 1-2
не трогают этот файл, номера строк не сместились). Добавить рядом с существующими
`seed_device`/`create_handover` хелперами (тот же стиль: одноцелевая async-функция,
`writer.execute(move |conn| { conn.execute(...).map_err(map_rusqlite)?; Ok(conn.last_insert_rowid())
}).await.expect(...)`, константа `NOW` для created_at_utc):

- `seed_cartridge_model(writer, brand: &str, model: &str) -> i64` — raw INSERT INTO
  cartridge_models (brand, model, created_at_utc, updated_at_utc, version) VALUES (?1, ?2, ?3, ?3,
  1).
- `seed_cartridge(writer, code: &str, model_id: i64, place_id: i64) -> i64` — raw INSERT INTO
  cartridges (code, model_id, status_id, place_id, created_at_utc, updated_at_utc, version) VALUES
  (?1, ?2, 1, ?3, ?4, ?4, 1) — `status_id = 1` = «На складе» (сид V001, тот же id что и у
  device_statuses, комментарий-конвенция уже есть у seed_device в этом файле).
- `seed_audit_log(writer, entity_type: &str, entity_id: i64, action: &str, created_at_utc: i64)` —
  raw INSERT INTO audit_log (entity_type, entity_id, action, created_at_utc) VALUES (?1, ?2, ?3,
  ?4) — V008-схема, НЕТ колонок deleted_at_utc/version (append-only hard-delete таблица).
- `seed_requester(writer, login: &str, full_name: &str) -> i64` — raw INSERT INTO users (login,
  full_name, password_hash, role, ad_user, is_active, created_at_utc, updated_at_utc, version)
  VALUES (?1, ?2, NULL, 'employee', 1, 0, ?3, ?3, 1) — мирроring хелпера из
  tests/dashboard_widgets.rs.
- `seed_request(writer, request_type: &str, status: &str, requested_by_user_id: i64,
  printer_device_id: Option<i64>, created_at_utc: i64) -> i64` — raw INSERT INTO requests
  (request_type, status, requested_by_user_id, printer_device_id, created_at_utc, updated_at_utc,
  version) VALUES (?1, ?2, ?3, ?4, ?5, ?5, 1).

Добавить 5 новых `#[tokio::test(flavor = "multi_thread", worker_threads = 4)]` функций, каждая
обёрнута в `tokio::time::timeout(Duration::from_secs(30), async { ... }).await.expect("timeout")`
точно как существующие 6, переиспользуя `seed_tree`/`make_ctx`/`wide_period`/`count_for` из этого
же файла. Все новые имена мест/картриджей/принтеров/устройств — ТОЛЬКО вымышленные (репозиторий
публичный, CLAUDE.md жёсткое условие); переиспользовать уже существующие в файле ФИО («Иванов
И.И.», «Петров П.П.», «Сидоров С.С.») для requester/giver, не выдумывать новые ФИО. Каждый тест
доказывает контракт В ОБЕ СТОРОНЫ (root-фильтр захватывает вложенную строку И sibling-поддерево не
захватывается) точными `assert_eq!` по количеству строк, не просто non-empty:

1. `cartridge_consumption_report_root_place_filter_and_excludes_sibling` — через
   `ctx.reports.list_cartridge_consumption(filter, wide_period())`: `seed_tree`, 1 cartridge_model,
   2 картриджа (по одному в tree.room_a/tree.room_b), по 1 audit_log-строке
   action='custom:install' на каждый; unfiltered = 2 строки; `place_id: Some(tree.building_a)` →
   ровно 1 строка (картридж из room_a, `place_path` == "Здание А / 2 этаж / Кабинет 214");
   `place_id: Some(tree.building_b)` → ровно 1 строка (картридж из room_b).

2. `cartridge_snapshot_root_place_filter_and_excludes_sibling` — через
   `ctx.reports.list_cartridge_in_stock(filter)`: тот же tree, 2 картриджа (status_id=1 по
   умолчанию, БЕЗ audit_log), те же root/sibling точные assert-ы против snapshot-запроса.

3. `report_counts_cartridges_domain_place_filter_is_subtree_inclusive` — через
   `ctx.reports.get_report_counts("cartridges", filter, wide_period(), false)` +
   `count_for(&counts, key)`: 2 картриджа с install-audit_log (для ключа "consumption") ПЛЮС ещё 2
   ОТДЕЛЬНЫХ картриджа без audit_log (для ключа "in_stock" — раздельные фикстуры по ключу, как в
   существующем devices-counts тесте, чтобы избежать пересечения состояний), по одному на
   root_a/root_b каждая пара; unfiltered: consumption=2, in_stock=2; `place_id:
   Some(tree.building_a)`: consumption=1, in_stock=1; `place_id: Some(tree.building_b)`:
   consumption=1, in_stock=1.

4. `requests_report_root_place_filter_and_excludes_sibling` — через
   `ctx.reports.list_requests_all(filter, wide_period(), false)`: 1 requester (`seed_requester`), 2
   принтера-устройства (переиспользовать существующий `seed_device`, по одному в
   tree.room_a/tree.room_b), 2 заявки (request_type='free_form', status='open'), каждая с
   `printer_device_id` на свой принтер; unfiltered = 2 строки; `place_id:
   Some(tree.building_a)` → ровно 1 строка, `place_path` содержит "Кабинет 214"; `place_id:
   Some(tree.building_b)` → ровно 1 строка, `place_path` НЕ содержит "Здание А".

5. `report_counts_requests_domain_place_filter_is_subtree_inclusive` — через
   `ctx.reports.get_report_counts("requests", filter, wide_period(), false)`: та же фикстура, что
   тест 4; unfiltered: all=2, open=2; `place_id: Some(tree.building_a)`: all=1, open=1; `place_id:
   Some(tree.building_b)`: all=1, open=1.

Обновить верхний doc-комментарий файла (`//! ... таблица builder → публичный вход`) — добавить
строки для 6 новых покрытых builder-ов (`query_cartridge_audit`, `query_cartridge_snapshot`,
`count_cartridge_audit_inner`, `count_cartridge_snapshot_inner`, `query_requests_inner`,
`count_requests_inner`) и их публичных входов; поправить формулировку "ЧЕТЫРЬМЯ" на актуальное
число реализаций (десять).

Прогнать полный набор проверок из repo_constraints.
  </action>
  <verify>
    <automated>TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test report_place_subtree --test report_cartridges --test report_requests -- --test-threads=1</automated>
  </verify>
  <done>11 тестов в report_place_subtree.rs (6 существующих devices + 5 новых cartridges/requests) зелёные; report_cartridges.rs и report_requests.rs остаются зелёными (регресс); cargo clippy -p trackly-app --all-targets -- -D warnings чист.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|--------------|
| Браузер/десктоп UI (`ReportFilters.svelte` PlacePicker, все 3 таба) -> `ReportFilter.place_id` -> SQL (`report_service.rs`) | `place_id` — числовой ID, приходит от недоверенного клиента (LAN-браузер); связывается в SQL исключительно через `?N`-параметры, как и уже принятый идентичный путь на домене «Устройства» (D-28, Phase 39) |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|------------------|
| T-260826-rbe-01 | Tampering | `query_cartridge_audit`/`query_cartridge_snapshot`/`count_cartridge_*`/`query_requests_inner`/`count_requests_inner` — SQL-инъекция через place_id | mitigate | `place_id: Option<i64>` — типизированное число, никогда не текстовая конкатенация; бинд только через `next_idx(&owned_params)`/`owned_params.push` и `?N`-плейсхолдер, идентично уже принятому паттерну devices-домена (D-28) |
| T-260826-rbe-02 | Tampering | Одновременный `place_id` + `is_storage` на одном алиасе — некорректная merge-safe композиция CTE ломает WHERE тихо (не ошибка, а неверный результат — молчаливая деградация до одного из двух фильтров) | mitigate | Обе CTE-ветки (place-subtree и storage_ids) переведены на единую `if with_prefix.is_empty() {...} else {...}` форму во всех 6 функциях (Tasks 1-2); покрыто не отдельным тестом на комбинацию, но структурно устраняет источник бага, воспроизведённого в devices-домене как эталон (`query_acts_inner`, где паттерн уже верен) |
| T-260826-rbe-03 | Information Disclosure | Место принтера заявки (`d.place_id` через LEFT JOIN devices) в count_requests_inner — новый JOIN может неявно расширить видимость строк, если `printer_device_id IS NULL` обрабатывается некорректно | accept | `LEFT JOIN` (не `INNER JOIN`) сохраняет заявки без принтера в выборке при отсутствии place_id-фильтра (та же семантика, что уже в `query_requests_inner`); при активном place_id-фильтре `d.place_id IN (...)` естественно исключает NULL-принтер строки — то же поведение, что уже принято для is_storage на этом же алиасе |
</threat_model>

<verification>
1. `cargo check -p trackly-app` — 0 ошибок после Task 1 и Task 2.
2. `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test report_place_subtree --test report_cartridges --test report_requests -- --test-threads=1` — все тесты зелёные (6 существующих + 5 новых в report_place_subtree.rs, плюс полный регресс двух соседних report-тест-файлов).
3. `cargo clippy -p trackly-app --all-targets -- -D warnings` — чисто.
4. Полный регресс (один `cargo test` за раз — НЕ параллелить с другими вызовами):
   `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip login_remember_persistent_cookie --test-threads=1` — 0 упавших.
</verification>

<success_criteria>
- PlacePicker на табах «Картриджи» и «Заявки» раздела «Отчёты» реально фильтрует и список, и счётчики вкладок, subtree-inclusive (выбранное место + всё вложенное), идентично уже работающему табу «Устройства».
- Комбинация «место» + «на складе» (is_storage) не ломает друг друга ни на одном из шести затронутых builder-ов.
- Домен «Устройства» и все ранее зелёные report-тесты не регрессируют.
- Фронтенд (`ReportFilters.svelte`) не изменён — контрол уже корректно рендерится на всех табах, фикс целиком в бэкенде.
</success_criteria>

<output>
Create `.planning/quick/260826-rbe-extend-d-28-subtree-place-filter-to-cart/260826-rbe-SUMMARY.md` when done
</output>
