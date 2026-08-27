---
quick_id: 260827-rzq
slug: sibling-cmp-sort-by-places-list-all
phase: 260827-rzq
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-core/src/domain/places.rs
  - crates/trackly-app/src/services/place_service.rs
  - crates/trackly-app/tests/places_contents.rs
  - ui/src/features/places/PlaceTree.svelte
autonomous: false
requirements: [RZQ-01, RZQ-02, RZQ-03]
must_haves:
  truths:
    - "Дерево мест с большим числом братских узлов, часть которых имеет ручной sort_order, а часть — нет (типичный результат drag-and-drop переупорядочивания), больше не роняет соединение places_list_all с ERR_EMPTY_RESPONSE — эндпоинт успешно возвращает список в режиме сервера (RZQ-01/RZQ-02)"
    - "sibling_cmp — настоящий полный порядок (рефлексивность/антисимметрия/транзитивность) для любых комбинаций sort_order/level/name; sort_by(sibling_cmp) над любым набором PlaceRow завершается без паники (проверка Rust ≥1.81) независимо от размера набора (RZQ-01)"
    - "Порядок братьев определяется одной и той же трёхступенчатой цепочкой для КАЖДОЙ пары узлов (а не разным правилом в зависимости от того, что заполнено у конкретной пары): сначала sort_order (узел со значением — раньше узла без него), затем level (аналогично: со значением — раньше), затем натуральное сравнение имени (RZQ-01)"
    - "places_list_all группирует строки по parent_id и лишь внутри группы применяет sibling_cmp — массив больше не сравнивает несвязанные не-братские узлы (например, здание с посторонней комнатой) единым плоским sort_by без всякого смысла (RZQ-02)"
    - "Клиентский порт компаратора в PlaceTree.svelte (siblingCmp/naturalNameCmp) сортирует братьев по тем же трём ступеням и той же конвенции Some/None, что и бэкенд, — дерево в браузере не расходится молча с тем, что вернул бы отсортированный бэкенд (RZQ-03)"
    - "Отрицательные и нулевые этажи по-прежнему сортируются корректно относительно друг друга и положительных этажей (PLC-02 не регрессирует) (RZQ-01)"
  artifacts:
    - path: "crates/trackly-core/src/domain/places.rs"
      provides: "sibling_cmp — лексикографическая цепочка (sort_order, затем level, затем natural_name_cmp) с явной Some/None-конвенцией на каждой ступени; exhaustive-тест законов полного порядка; регрессионный тест case C (≥60 строк, частичный sort_order, имена и уровни, противоречащие ручному порядку); тест полного порядка для natural_name_cmp"
      contains: "fn sibling_cmp"
    - path: "crates/trackly-app/src/services/place_service.rs"
      provides: "list_all сортирует по (parent_id, затем sibling_cmp) вместо плоского sibling_cmp по всему дереву; list_children — без изменений (уже настоящие братья)"
      contains: "a.parent_id.cmp(&b.parent_id)"
    - path: "crates/trackly-app/tests/places_contents.rs"
      provides: "Регрессионный тест на уровне сервиса, воспроизводящий вчерашнюю панику (частичный sort_order + противоречащие имена) через реальный вызов list_children/list_all — доказывает, что путь places_list_all/places_list_children не падает"
      contains: "fn list_children_and_list_all_survive_partial_sort_order_without_panicking"
    - path: "ui/src/features/places/PlaceTree.svelte"
      provides: "siblingCmp — та же трёхступенчатая цепочка с явной Some/None-конвенцией, что и в Rust (не «оба заданы → сравнить, иначе следующая ступень», а Some всегда раньше None на каждой ступени)"
      contains: "function siblingCmp"
  key_links:
    - from: "crates/trackly-core/src/domain/places.rs sibling_cmp"
      to: "crates/trackly-app/src/services/place_service.rs list_children/list_all"
      via: "прямой вызов функции между крейтами (rows.sort_by(sibling_cmp) / замыкание с parent_id.cmp().then_with(sibling_cmp))"
      pattern: "sibling_cmp"
    - from: "crates/trackly-core/src/domain/places.rs sibling_cmp"
      to: "ui/src/features/places/PlaceTree.svelte siblingCmp"
      via: "ручной вербатим-порт на JS (нет общего кода между Rust и TS) — риск молчаливого расхождения, если один поправить, а другой нет"
      pattern: "function siblingCmp"
---

<objective>
Починить `crates/trackly-core/src/domain/places.rs::sibling_cmp` — компаратор братских мест не
является полным порядком (для каждой пары применяется РАЗНОЕ правило в зависимости от того, что у
неё заполнено: «оба со sort_order» / «оба с level» / иначе имя), из-за чего транзитивность рвётся
на смешанных наборах. Начиная с Rust 1.81 `slice::sort_by` детектирует нарушение полного порядка и
паникует: `user-provided comparison function does not correctly implement a total order`. У
приложения нет `CatchPanicLayer` — паника внутри axum-хендлера обрывает соединение без ответа,
именно это увидел пользователь как `ERR_EMPTY_RESPONSE` на `places_list_all` при построении дерева
мест из ~20 узлов на Windows в режиме сервера. Малые срезы используют insertion sort и вместо
паники тихо дают мусорный порядок — поэтому дефект пережил всю UAT фазы 39 на маленьком демо-дереве
и проявился только когда дерево выросло.

**Фикс:** заменить компаратор на настоящую лексикографическую цепочку, где КАЖДАЯ пара проходит
ОДНУ и ТУ ЖЕ последовательность стадий (sort_order → level → natural_name_cmp), и каждая стадия
явно определяет, как Some сортируется относительно None, вместо того чтобы пропускать стадию,
когда только у одной стороны есть значение. Конвенция (см. `<task 1>` за обоснованием): узел со
значением на данной стадии сортируется РАНЬШЕ узла без значения — это соответствует
39-UI-SPEC.md/D-05 «ручной, если задан; иначе автоматический» и делает драг-н-дроп-позиционированные
узлы (`sort_order` задан) видимо приоритетными над естественно упорядоченными. Порядок для
смешанных братских наборов НЕИЗБЕЖНО меняется по сравнению со старым (недетерминированным)
поведением — это фиксируется явно как осознанное решение, а не сюрприз.

**Побочный вопрос из брифа (обязателен к решению, не к молчаливому игнору):**
`PlaceService::list_all` сегодня применяет `sibling_cmp` плоско ко ВСЕМУ дереву — здания, этажи и
комнаты, не являющиеся братьями, сравниваются друг с другом бессмысленно. Ни один текущий
потребитель (`PlaceTree.svelte` перестраивает дерево из `parent_id` и пересортировывает КАЖДУЮ
братскую группу сама; `PlaceContents.svelte` не зависит от порядка; `search()` вызывает
`repo.list_all` напрямую, минуя сервис) не полагается на этот плоский порядок. Решение: `list_all`
группирует строки по `parent_id`, затем применяет `sibling_cmp` ВНУТРИ группы — массив остаётся
детерминированным и осмысленным (братья рядом и в правильном порядке) для любого будущего прямого
потребителя, вместо произвольного глобального порядка. Это не меняет наблюдаемое поведение UI
сегодня (фронтенд всё равно перегруппирует), но убирает саму возможность сравнения несвязанных
узлов.

Заодно чинится тихий второй дефект: verbatim JS-порт того же компаратора в
`PlaceTree.svelte` (`siblingCmp`/`naturalNameCmp`) — `Array.prototype.sort` в JS не бросает
исключение на непоследовательном компараторе, а молча даёт implementation-defined порядок, так что
дерево в браузере могло просто отрисоваться в неверном порядке без единой ошибки в консоли. Порт
переносится в тот же трёхступенчатый вид с той же Some/None-конвенцией.

Purpose: устранить панику/DoS на `places_list_all`/`places_list_children` для любой формы данных
(в частности — частичный `sort_order` после drag-n-drop реордера части братьев), сделать порядок
братьев детерминированным и одинаковым на бэкенде и во фронтенде.

Output: настоящий полный порядок в `sibling_cmp` + доказывающие это тесты (exhaustive-проверка
законов порядка, регрессия case C воспроизводящая вчерашнюю панику, сохранённое покрытие PLC-02);
осмысленная group-by-parent сортировка в `list_all`; синхронизированный JS-порт во фронтенде.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md

<interfaces>
Перечитать все три файла перед правкой — номера строк ниже ориентировочные (на момент планирования).

`crates/trackly-core/src/domain/places.rs`:

- `pub struct PlaceRow { id: i64, parent_id: Option<i64>, kind: PlaceKind, name: String, level:
  Option<i64>, is_storage: bool, sort_order: Option<i64>, ... }` (~строка 70) — поля, участвующие в
  сравнении: `sort_order: Option<i64>`, `level: Option<i64>`, `name: String`.

- Текущий `sibling_cmp` (~строка 162-170):
  ```
  if let (Some(sa), Some(sb)) = (a.sort_order, b.sort_order) { return sa.cmp(&sb); }
  if let (Some(la), Some(lb)) = (a.level, b.level) { return la.cmp(&lb); }
  natural_name_cmp(&a.name, &b.name)
  ```
  Дефект: разное правило для разных пар (транзитивность рвётся на смешанных наборах — контрпример
  в дефект-брифе).

- `natural_name_cmp(a: &str, b: &str) -> std::cmp::Ordering` (~строка 175) — существующий tie-break,
  НЕ ТРОГАТЬ реализацию (только добавить тест, подтверждающий, что это уже полный порядок и не
  зацикливается).

- `#[cfg(test)] mod tests` (~строка 233) — 5 существующих тестов на `sibling_cmp`/`natural_name_cmp`,
  включая `sibling_cmp_orders_negative_zero_positive_levels` (PLC-02) — должны остаться зелёными БЕЗ
  изменения своих тел (проверено при планировании: все 5 сравнивают пары, где оба узла имеют
  одинаковое «состояние заполненности» на каждой стадии — None/None или Some/Some, — поэтому новая
  Some/None-конвенция их не задевает).

`crates/trackly-app/src/services/place_service.rs`:

- `use trackly_core::domain::places::{... sibling_cmp, ...}` (~строка 36).
- `list_children` (~строка 481-499): `rows.sort_by(sibling_cmp);` — НЕ МЕНЯТЬ, это уже настоящие
  братья (общий `parent_id` гарантирован репозиторием).
- `list_all` (~строка 503-520): `rows.sort_by(sibling_cmp);` — заменить на group-by-parent сортировку
  (см. Task 2).

`crates/trackly-app/tests/places_contents.rs`:

- `fn make_service() -> (PlaceService, tempfile::TempDir)` (~строка 20), `fn admin_caller() ->
  Identity` (~строка 27), `fn new_place(kind, name, parent_id) -> PlaceNew` (~строка 31, всегда
  `level: None, sort_order: None` — для нового регрессионного теста собирать `PlaceNew` вручную,
  не через этот хелпер, там понадобятся ненулевые `level`/`sort_order`).
- Существующий тест `list_children_sorted_by_sibling_cmp_not_insertion_order` (~строка 120) — не
  трогать, служит образцом стиля (`tokio::test(flavor = "multi_thread", worker_threads = 4)` +
  `tokio::time::timeout(Duration::from_secs(30), async { ... }).await.expect("test timed out")`).

`ui/src/features/places/PlaceTree.svelte` (~строки 60-115):

- `function naturalNameCmp(a: string, b: string): number` — verbatim JS-порт `natural_name_cmp`, НЕ
  МЕНЯТЬ (соответствует Rust-версии один в один, дефект не в ней).
- `function siblingCmp(a: PlaceDto, b: PlaceDto): number` (~строка 103) — текущий JS-порт со ТЕМ ЖЕ
  дефектом («оба заданы → сравнить, иначе следующая ступень»), заменить на трёхступенчатую цепочку
  с явной Some/None-конвенцией (Rust `Option<i64>` ↔ TS `number | null` из `PlaceDto`).
- `for (const arr of map.values()) arr.sort(siblingCmp);` (~строка 160) — группировка по
  `parent_id` и сортировка КАЖДОЙ братской группы уже существует во фронтенде; не трогать, только
  саму функцию `siblingCmp`.
- `type PlaceDto = { ...; level: number | null; sort_order: number | null; name: string; ... }`
  (`ui/src/bindings.ts`, генерируется `specta` — не редактировать вручную).

Нет автотестового харнесса для фронтенда (`ui/package.json` не содержит `vitest`/аналога, в
репозитории нет ни одного `*.test.ts`/`*.spec.ts`) — паритет JS-порта с Rust проверяется вручную
(Task 3, checkpoint), а не изобретённым для этой задачи тестовым файлом.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: sibling_cmp — настоящий полный порядок + доказывающие тесты</name>
  <files>crates/trackly-core/src/domain/places.rs</files>
  <behavior>
    - Тест 1 (exhaustive, законы полного порядка): сгенерировать все `PlaceRow` из декартова
      произведения `sort_order ∈ {None, Some(0), Some(1)}` × `level ∈ {None, Some(-1), Some(0),
      Some(1)}` × `name ∈ {"2", "10", "Zzz"}` (36 строк). Для каждой строки — рефлексивность
      (`sibling_cmp(a, a) == Equal`). Для каждой пары — антисимметрия
      (`sibling_cmp(a, b) == sibling_cmp(b, a).reverse()`). Для каждой тройки — транзитивность
      (если `a<=b` и `b<=c`, то `a<=c`, через `!= Greater`).
    - Тест 2 (регрессия case C — воспроизводит вчерашнюю панику): собрать ≥60 `PlaceRow` с
      частичным `sort_order` (часть Some в порядке, противоречащем именам/level, часть None),
      вызвать `rows.sort_by(sibling_cmp)` — тест должен просто завершиться (сегодня здесь паника
      «user-provided comparison function does not correctly implement a total order»), затем
      проверить, что результат неубывающий (`windows(2)` — каждая соседняя пара `!= Greater`).
    - Тест 3 (`natural_name_cmp` — тоже полный порядок): те же три закона (рефлексивность,
      антисимметрия, транзитивность) на наборе строк, включающем кириллицу, цифровые пробеги разной
      длины («2» vs «10») и пустую строку; тест, будучи синхронным и завершающимся, заодно
      подтверждает отсутствие бесконечного цикла на любой из проверенных пар.
    - Существующий тест `sibling_cmp_orders_negative_zero_positive_levels` (PLC-02) остаётся
      зелёным без изменений своего тела.
  </behavior>
  <action>
Заменить тело `sibling_cmp` на лексикографическую цепочку из трёх стадий, где КАЖДАЯ стадия
применяется к КАЖДОЙ паре одинаково (а не выборочно, как сегодня), и явно решает Some-vs-None:

1. Стадия `sort_order`: `(Some(sa), Some(sb))` → сравнить `sa.cmp(&sb)`, при неравенстве вернуть
   результат, при равенстве — провалиться на следующую стадию (не возвращать `Equal` сразу).
   `(Some(_), None)` → `Less` (узел с ручным порядком — раньше). `(None, Some(_))` → `Greater`.
   `(None, None)` → провалиться на следующую стадию.
2. Стадия `level`: та же форма — `(Some(la), Some(lb))` сравнить, при неравенстве вернуть, при
   равенстве провалиться дальше; `(Some(_), None) => Less`; `(None, Some(_)) => Greater`; `(None,
   None)` → провалиться дальше.
3. `natural_name_cmp(&a.name, &b.name)` — как и раньше, финальный tie-break.

Написать doc-комментарий над `sibling_cmp`, заменяющий текущий (~строка 159-161): объяснить, ПОЧЕМУ
раньше это не было полным порядком (разное правило для разных пар — не транзитивно на смешанных
наборах), сослаться на quick 260827-rzq, и явно назвать конвенцию Some-раньше-None с обоснованием
(соответствует D-05 «ручной порядок, если задан, иначе автоматический» — узел с явным значением на
данной стадии получает приоритет над узлом, где решение принимается автоматически).

Добавить три теста из `<behavior>` в `#[cfg(test)] mod tests`, переиспользуя существующий хелпер
`place(id, level, sort_order, name)` (~строка 237) для конструирования строк — только вымышленные
имена (privacy-условие CLAUDE.md).

Прогнать `cargo fmt --check` перед коммитом.
  </action>
  <verify>
    <automated>cargo test -p trackly-core --lib domain::places:: 2>&1 | tail -40</automated>
  </verify>
  <done>Все 8 тестов в domain::places (5 существующих + 3 новых) зелёные; `sort_by(sibling_cmp)` на 60-строчном частично-упорядоченном наборе (case C) больше не паникует; cargo fmt --check чист.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: list_all — group-by-parent сортировка + регрессия на уровне сервиса + JS-паритет</name>
  <files>crates/trackly-app/src/services/place_service.rs, crates/trackly-app/tests/places_contents.rs, ui/src/features/places/PlaceTree.svelte</files>
  <behavior>
    - Тест (places_contents.rs, интеграционный, через реальный `PlaceService`): создать здание,
      вставить ~15 комнат — часть с явным `sort_order` (в порядке, обратном вставке), часть без
      него, имена — в порядке, противоречащем и level, и sort_order. Вызвать
      `svc.list_children(&admin, Some(building.id))` И `svc.list_all(&admin, false)` — оба вызова
      должны завершиться без паники (сегодня это ровно тот путь, что уронил
      `places_list_all`/`places_list_children` в проде) и вернуть результат, где для каждой пары
      соседних элементов внутри одной родительской группы `sibling_cmp` не даёт `Greater`.
  </behavior>
  <action>
В `place_service.rs::list_all` (~строка 519) заменить `rows.sort_by(sibling_cmp);` на сортировку по
составному ключу — сначала `parent_id`, затем `sibling_cmp` внутри группы:
`rows.sort_by(|a, b| a.parent_id.cmp(&b.parent_id).then_with(|| sibling_cmp(a, b)));`. Обновить
doc-комментарий метода (~строка 501-502): объяснить решение из брифа — раньше плоский `sibling_cmp`
сравнивал НЕ-братьев (здание против случайной комнаты) без всякого смысла; ни один текущий
потребитель (`PlaceTree.svelte` перегруппирует и пересортировывает сама, `PlaceContents.svelte` не
зависит от порядка, `search()` использует `repo.list_all` напрямую, минуя сервис) не полагается на
плоский порядок, но group-by-parent делает массив осмысленным (братья рядом, верно упорядочены) для
любого будущего прямого потребителя. `list_children` НЕ трогать — там уже настоящие братья.

В `places_contents.rs` добавить интеграционный тест
`list_children_and_list_all_survive_partial_sort_order_without_panicking` рядом с существующим
`list_children_sorted_by_sibling_cmp_not_insertion_order` (тот же стиль:
`tokio::test(flavor = "multi_thread", worker_threads = 4)` +
`tokio::time::timeout(Duration::from_secs(30), ...)`), собирая `PlaceNew` вручную (не через
`new_place` — там `level`/`sort_order` всегда `None`) для сценария из `<behavior>`.

В `ui/src/features/places/PlaceTree.svelte::siblingCmp` (~строка 103) заменить тело на ту же
трёхступенчатую цепочку, что и в Rust (Task 1), с той же Some/None-конвенцией на TS-типах
`number | null`: стадия `sort_order` — оба не `null` → сравнить разностью, при равенстве провалиться
дальше; ровно один не `null` → он раньше (`-1`)/позже (`1`); оба `null` → провалиться дальше. Та же
форма для стадии `level`. Финал — существующий `naturalNameCmp(a.name, b.name)` без изменений.
`naturalNameCmp` саму не трогать. Пересобрать фронтенд-бандл после правки (`pnpm --dir ui build`) —
обязательно для последующей LAN-браузер/desktop-webview проверки в Task 3.

Прогнать полный закрывающий набор бэкенд-проверок ОДИН `cargo test` за раз (repo_constraints):
`cargo fmt --check`, затем `cargo test -p trackly-core` (полный пакет, не только domain::places —
холодная компиляция ~2-3 мин, не путать с зависанием), затем `TRACKLY_AD_MOCK=1
TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip login_remember_persistent_cookie` (ПОЛНЫЙ
пакет, не только places_contents.rs — непроверенный тестовый бинарник не считается зелёным). Затем
фронтенд-гейты: `pnpm --dir ui exec svelte-check`, `pnpm --dir ui lint`, `pnpm --dir ui build`.
  </action>
  <verify>
    <automated>TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test places_contents -- --test-threads=1 2>&1 | tail -40</automated>
  </verify>
  <done>Новый интеграционный тест в places_contents.rs зелёный; полный `cargo test -p trackly-core` и полный `cargo test -p trackly-app --skip login_remember_persistent_cookie` зелёные; cargo fmt --check чист; svelte-check/lint/build зелёные; PlaceTree.svelte::siblingCmp зеркалит новую Rust-конвенцию.</done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 3: Живая проверка — большое дерево с частичным ручным порядком не роняет «Места»</name>
  <action>Блокирующая ручная проверка: Task 1/2 автоматизировали и доказали тестами всё, что можно (полный порядок sibling_cmp + group-by-parent list_all + JS-паритет), но svelte-check/eslint/build не доказывают поведение рун Svelte 5 в рантайме, а синтетический DOM-харнесс не заменяет реальное WKWebView/браузер — нужно визуально подтвердить, что дерево мест с частичным ручным sort_order загружается без ошибок и в детерминированном порядке.</action>
  <what-built>
Task 1/2 автоматизировали и доказали тестами всё, что можно: `sibling_cmp` — настоящий полный
порядок (exhaustive-проверка законов + регрессия на 60-строчном case C, воспроизводящем вчерашнюю
панику), `list_all` больше не сравнивает несвязанные не-братские узлы, JS-порт синхронизирован с
Rust. Но: (а) компиляционные гейты (svelte-check/eslint/build) не доказывают поведение рун Svelte 5
в рантайме (см. память проекта «Compile gates miss Svelte runtime»), и (б) синтетический
DOM-харнесс — не верификация в реальном WKWebView/браузере (память «Synthetic harness not
verification») — нужно визуально подтвердить в живом приложении и на дереве, приближённом по форме
к тому, что уронило прод.
  </what-built>
  <how-to-verify>
1. Убедиться, что `ui/dist` пересобран (`pnpm --dir ui build` из Task 2), затем запустить `cargo
   tauri dev` (десктоп-вебвью) — AD/SNMP не нужны для этой проверки (дефект только в местах),
   `TRACKLY_AD_MOCK`/`TRACKLY_SNMP_MOCK` не требуются для самого приложения в dev-режиме.
2. В разделе «Места» создать дерево из ≥3 зданий/этажей и ≥20 комнат под одним зданием (можно
   вымышленные имена вида «Кабинет 1».«Кабинет 20» вперемешку с «Кабинет 2», «Кабинет 10» —
   проверка natural-sort заодно).
3. Если в UI есть drag-and-drop переупорядочивание братьев — перетащить 2-3 комнаты в новую позицию
   (это выставляет `sort_order` ТОЛЬКО у перетащенных узлов — ровно тот частичный-набор сценарий,
   что ронял прод); если drag-and-drop недоступен в текущем UI — задать «Порядок» вручную через
   форму редактирования для 2-3 комнат (поле `sort_order`, per D-05/39-UI-SPEC.md).
4. Перезагрузить страницу «Места» (или переоткрыть приложение) — дерево должно загрузиться БЕЗ
   «Не удалось загрузить места…», без ошибок в консоли DevTools, независимо от размера дерева.
5. Проверить, что перетащенные/вручную-упорядоченные узлы отображаются В НАЧАЛЕ своей братской
   группы (per новую конвенцию Some-раньше-None), а остальные — по естественному порядку имён,
   ОДИНАКОВО при каждой перезагрузке (детерминированность).
6. Если есть доступ к режиму сервера (LAN-браузер) — повторить шаг 4 через браузер на
   `https://localhost:8443` (или настроенный порт) — это тот транспорт, где именно упал прод
   (`net::ERR_EMPTY_RESPONSE`).
7. (Опционально, для подтверждения причинно-следственной связи с исходным репортом — необязательно
   для одобрения этой задачи) Проверить `logs/` рядом с исполняемым файлом на отсутствие новых
   строк «user-provided comparison function does not correctly implement a total order» после шагов
   выше.
  </how-to-verify>
  <resume-signal>Напишите "approved" или опишите, что увидели не так</resume-signal>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|--------------|
| Нет новой границы доверия | Задача не вводит новый вход от клиента и не меняет авторизацию (`Action::ReadPlaces`) — чинит внутренний компаратор и его использование на уже читаемых из БД данных |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-260827-rzq-01 | Denial of Service | axum-хендлеры `places_list_all`/`places_list_children` (нет `CatchPanicLayer`) — паника в `sort_by(sibling_cmp)` рвёт соединение без ответа | mitigate | Собственно фикс этой задачи: `sibling_cmp` теперь настоящий полный порядок, `sort_by` не может паниковать ни на каком входе (доказано exhaustive-тестом и регрессией case C на 60 строках) — этот конкретный путь к DoS закрыт |
| T-260827-rzq-02 | Denial of Service | Любой ДРУГОЙ будущий паникующий компаратор/хендлер — у приложения по-прежнему нет `CatchPanicLayer`, паника где угодно всё ещё роняет соединение молча | accept | Вне рамок этой задачи — добавление глобального panic-catching middleware это архитектурное изменение уровня приложения, а не фикс одного компаратора; зафиксировать как отдельный будущий quick/фазу |
| T-260827-rzq-SC | Tampering | npm/pip/cargo-установки | n/a | В этой задаче нет задач на установку новых пакетов — package legitimacy gate не применяется |
</threat_model>

<verification>
1. `cargo test -p trackly-core --lib domain::places::` — 8/8 зелёных (5 существующих + 3 новых).
2. `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test places_contents -- --test-threads=1` — все зелёные, включая новый регрессионный тест.
3. `cargo test -p trackly-core` (полный пакет) — 0 упавших.
4. `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip login_remember_persistent_cookie` (полный пакет, один `cargo test` за раз) — 0 упавших.
5. `cargo fmt --check` — чист.
6. `pnpm --dir ui exec svelte-check`, `pnpm --dir ui lint`, `pnpm --dir ui build` — все зелёные.
7. Живая проверка (Task 3) — большое дерево с частичным ручным порядком грузится без ошибок в десктоп-вебвью и (если доступно) в LAN-браузере; порядок братьев детерминирован и совпадает с документированной конвенцией.
</verification>

<success_criteria>
- `sibling_cmp` — доказанный полный порядок; `places_list_all`/`places_list_children` не падают ни на каком наблюдаемом наборе данных о местах.
- `list_all` больше не сравнивает несвязанные не-братские узлы плоским sort_by — группировка по parent_id задокументирована и реализована как осознанное решение.
- JS-порт компаратора во фронтенде синхронизирован с бэкендом — нет молчаливого расхождения в порядке отображения.
- PLC-02 (отрицательные/нулевые этажи) не регрессирует.
- Полный набор существующих backend/frontend гейтов зелёный (не только затронутые тестовые бинарники).
</success_criteria>

<output>
Create `.planning/quick/260827-rzq-sibling-cmp-sort-by-places-list-all/260827-rzq-SUMMARY.md` when done
</output>
