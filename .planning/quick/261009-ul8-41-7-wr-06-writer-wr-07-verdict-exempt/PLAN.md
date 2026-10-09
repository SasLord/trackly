---
phase: quick-261009-ul8
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-app/tests/entities_support/scan.rs
  - crates/trackly-app/tests/entities_support/registry.rs
  - crates/trackly-app/tests/entities_changed_gate.rs
  - .planning/phases/41.7-v1-4-ws-bootstrap-lan/41.7-REVIEW.md
  - .planning/phases/41.7-v1-4-ws-bootstrap-lan/41.7-SECURITY.md
  - .planning/todos/pending/2026-10-09-review-41.7-residual-findings.md
autonomous: true
requirements: [PLC-05, PLC-06, HST-02, GRP-04, GRP-05, GRP-06, GRP-07, GRP-08]
must_haves:
  truths:
    - "D-04: рассылка EntitiesChanged идёт ПОСЛЕ коммита - гейт краснит вызов broadcast_entities, стоящий раньше конца ПОСЛЕДНЕЙ точки записи (writer.execute(..) или вызов self-метода, транзитивно владеющего записью) в теле строки Broadcasts"
    - "D-04: строка Broadcasts, в теле которой сканер не нашёл ни одной точки записи, краснеет громко (BroadcastWithoutWrite), а не проходит молча"
    - "D-08: строка Exempt получает механическую проверку - любой вызов broadcast_entities в Exempt-функции краснит гейт (внутри writer-скобок - BroadcastInsideWriter, вне - ExemptBroadcasts); освобождение больше не верится на слово"
    - "D-07: проверки порядка и Exempt доказаны КУСАЮЩИМИСЯ на реальных исходниках сервисов: свип по всем 31 строке Broadcasts и всем 21 строке Exempt реестра, плюс именованные перенос-мутации с утверждённой уникальностью якоря"
    - "D-08: гейт не может быть зелёным, ничего не измерив - счётчики scan_with_stats обязаны совпасть с числом строк Broadcasts/Exempt в REGISTRY и не опускаться ниже нижних границ"
    - "D-16: область гейта (восемь SCOPE_FILES) и 118 строк REGISTRY не тронуты; существующие 34 теста остаются зелёными"
  artifacts:
    - path: "crates/trackly-app/tests/entities_support/scan.rs"
      provides: "Kind::BroadcastBeforeWriter / BroadcastWithoutWrite / ExemptBroadcasts, транзитивные точки записи, scan_with_stats"
      contains: "BroadcastBeforeWriter"
    - path: "crates/trackly-app/tests/entities_changed_gate.rs"
      provides: "selftest на синтетике, свипы-мутации на реальных исходниках, тест счётчиков"
      contains: "ExemptBroadcasts"
  key_links:
    - from: "scan.rs::scan (Broadcasts-ветка)"
      to: "scan.rs::write_points / writing_fns (фикспойнт)"
      via: "max конца точек записи против позиции каждого broadcast_entities("
      pattern: "BroadcastBeforeWriter"
    - from: "scan.rs::scan (Exempt-ветка)"
      to: "broadcast_calls + writer_ranges"
      via: "каждый вызов -> InsideWriter либо ExemptBroadcasts"
      pattern: "ExemptBroadcasts"
    - from: "entities_changed_gate.rs свипы"
      to: "src/services/*.rs (read_service)"
      via: "вставка вызова по позиции body.0+1, scan(..) обязан покраснеть"
      pattern: "mutation_.*every_"
---

<objective>
Закрыть две находки код-ревью фазы 41.7, которые ослабляют реестровый гейт (они же R-1 и R-2 аудита безопасности):

- WR-06: сканер проверяет, что рассылка НЕ внутри `writer.execute(..)`, но не что она ПОСЛЕ записи. Рассылка в начале мутации гейт проходит.
- WR-07: ветка `Verdict::Exempt(_) => {}` в scan.rs - освобождённые строки не получают никакой механической проверки.

Purpose: контракт D-04 ("событие только после `.await?` writer-замыкания и вне него") и D-08 ("освобождение кодируется, гейт доказывает, что не мёртв") должны запираться гейтом, а не доверием к прочтению.
Output: усиленный сканер, selftest на синтетике, свипы-мутации на реальных исходниках, тест "гейт что-то измерил", обновлённые три документа.

Область: ТОЛЬКО тест-инфраструктура. Продакшн-код `src/` не меняется (кроме временных on-disk мутаций в Task 3, которые откатываются `git checkout`).
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/STATE.md
@./CLAUDE.md
@.planning/phases/41.7-v1-4-ws-bootstrap-lan/41.7-REVIEW.md
@.planning/phases/41.7-v1-4-ws-bootstrap-lan/41.7-SECURITY.md
@.planning/phases/41.7-v1-4-ws-bootstrap-lan/41.7-14-SUMMARY.md
@crates/trackly-app/tests/entities_support/scan.rs
@crates/trackly-app/tests/entities_support/registry.rs
@crates/trackly-app/tests/entities_changed_gate.rs

<interfaces>
<!-- Извлечено из scan.rs / registry.rs. Исполнителю не нужно изучать кодовую базу заново. -->

scan.rs (существующее):
- `pub struct Prepared { plain: String, skeleton: String }` - две строки одной байтовой длины; индексы общие.
- `pub enum Kind { NoVerdict, MissingBroadcast, BroadcastInsideWriter, ReadOnlyOwnsWriter, ReadOnlyCallsWriter, StaleRow, DuplicateFn, DeadCodeAllowLeft }` (derive Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord).
- `pub struct FnInfo { name, is_pub, owns_write, body: (usize, usize) }` - body = байтовые индексы `{` и парной `}`.
- `pub fn parse_fns(p: &Prepared) -> Vec<FnInfo>` (все fn уровня элементов, включая приватные без строки реестра).
- `pub fn scan(rel: &str, src: &str, registry: &[Entry]) -> Vec<Violation>`.
- приватные: `writer_ranges(sk, text, lo, hi) -> Vec<(open, close)>` (диапазоны скобок каждого `writer.execute(..)`), `broadcast_calls(p, lo, hi) -> Vec<(paren_index, Option<label>)>`, `match_close(sk, open, o, c)`.
- В Broadcasts-ветке сейчас: проверка метки (MissingBroadcast) + "paren внутри любого writer-диапазона" (BroadcastInsideWriter). В Exempt-ветке: `{}`.

registry.rs: `Verdict::{Broadcasts, Exempt(&'static str), ReadOnly}`, `REGISTRY` (118 строк: 31 Broadcasts, 21 Exempt, 66 ReadOnly), `SCOPE_FILES` (8 файлов).

entities_changed_gate.rs (существующее): `read_service(file)`, `kinds(&vs)`, `has(&vs, func, Kind)`, `row(func, verdict)`, `SYN`, `SYN_CLEAN`, `EXEMPT_REASON`, `services_dir()`; импорты `first_test_item_range, inventory, owns_writer_outside_tests, parse_fns, prepare, scan, Kind`.

Реальные формы, которые правило обязано пережить (проверено чтением исходников):
- `place_service::archive` / `unarchive`: в теле НЕТ `writer.execute`; запись делегирована `self.set_archived(..).await?` (приватный владелец записи), затем `self.broadcast_entities("archive", ..)`.
- `device_service::create`: `self.create_without_broadcast(new).await?` (приватная, НЕ владеет записью напрямую, зовёт `self.insert_new_and_get(..)`, который владеет), рассылка внутри `if let` после. `import_csv_commit` зовёт `self.create_without_broadcast(..)` в цикле, рассылка после цикла. `create_single_with_number_check_with_printer` зовёт `self.insert_new_and_get(..)`. Значит "точка записи" обязана быть ТРАНЗИТИВНОЙ внутри файла.
- `group_place.rs`, `group_membership.rs`: ни одного `broadcast_entities` (grep = 0); все пять строк Exempt.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Сканер - порядок "рассылка ПОСЛЕ записи" (WR-06), механика Exempt (WR-07), счётчики; selftest на синтетике</name>
  <files>crates/trackly-app/tests/entities_support/scan.rs, crates/trackly-app/tests/entities_support/registry.rs, crates/trackly-app/tests/entities_changed_gate.rs</files>
  <behavior>
    Все selftest - на синтетике в памяти (имена файлов/людей не нужны; вымышленные идентификаторы):
    - selftest_broadcast_before_writer_is_red: broadcast_entities("create", ..) в начале create, writer.execute после - has(BroadcastBeforeWriter); SYN_CLEAN остаётся зелёным.
    - selftest_broadcast_between_two_writers_is_red: рассылка после первого writer.execute, но перед вторым - красный (правило "после ПОСЛЕДНЕЙ точки записи").
    - selftest_broadcast_before_delegated_write_is_red: archive-подобная форма - рассылка до `self.set_archived(..)`, где set_archived владеет writer.execute - красный; рассылка после - зелёный.
    - selftest_broadcast_before_transitive_delegate_is_red / _after_is_green: A зовёт self.B(), B зовёт self.C(), C владеет writer.execute (форма device create) - порядок решает.
    - selftest_broadcasts_row_without_any_write_is_red: строка Broadcasts, в теле нет ни writer.execute, ни вызова транзитивного владельца - has(BroadcastWithoutWrite).
    - selftest_broadcast_inside_writer_is_not_double_reported: на существующей фикстуре "внутри writer" остаётся InsideWriter и НЕ добавляется BroadcastBeforeWriter (существующий selftest_broadcast_inside_writer_closure_is_red обязан остаться зелёным как есть).
    - selftest_exempt_row_with_broadcast_is_red: Exempt-функция с broadcast_entities вне writer-скобок - has(ExemptBroadcasts).
    - selftest_exempt_row_with_broadcast_inside_writer_is_red: внутри writer.execute(..) - has(BroadcastInsideWriter).
    - selftest_exempt_row_without_broadcast_stays_green: существующий selftest_exempt_row_is_green не меняется.
    - selftest_scan_stats_count_checked_rows: scan_with_stats на SYN_CLEAN -> broadcasts_ordered == 1, exempt_checked == 0; на фикстуре с одной Exempt-строкой -> exempt_checked == 1.
  </behavior>
  <action>
    Сначала (RED): допиши selftest из <behavior> в entities_changed_gate.rs (раздел "часть 2: selftest") и зафиксируй, что они красные/не компилируются. Затем реализуй в scan.rs (GREEN).

    Реализация в scan.rs (per D-04, D-08):
    1. Kind: добавить варианты BroadcastBeforeWriter, BroadcastWithoutWrite, ExemptBroadcasts в конец enum (Ord-деривация не страдает). Обновить шапку модуля: список правил, и в блоке "ИЗВЕСТНОЕ ОГРАНИЧЕНИЕ" - что точки записи ищутся только в том же файле (self./Self:: и writer.execute), запись через другой файл/объект/макрос сканер не видит - такую строку Broadcasts он краснит громко (BroadcastWithoutWrite), а не пропускает.
    2. Множество writing_fns файла: фикспойнт. Начальное = имена fn с owns_write (из parse_fns, включая приватные и не требующие строки реестра). Цикл: fn добавляется, если в её теле есть вызов `(?:\bself\s*\.|\bSelf\s*::)\s*ИМЯ\s*\(` любого имени из множества, кроме собственного; повторять до отсутствия изменений. Регэкспы строить через regex::escape (как в ReadOnlyCallsWriter), не компилировать внутри самого горячего цикла лишний раз.
    3. write_points(f): writer_ranges тела (как сейчас) плюс диапазон скобок каждого вызова транзитивного владельца (open = индекс `(` после имени, close = match_close). Верхний предел - конец тела.
    4. Broadcasts-ветка (после существующих MissingBroadcast и InsideWriter, которые НЕ ослаблять и не переписывать): если вызовов broadcast_entities нет - ничего нового (уже MissingBroadcast). Иначе: если write_points пусты - BroadcastWithoutWrite; иначе last_end = максимум close среди write_points, и каждый вызов, чей paren < last_end И который не попал в уже зарепорченный InsideWriter, даёт BroadcastBeforeWriter. Счётчик broadcasts_ordered++ когда проверка порядка реально отработала (есть и вызовы, и точки записи).
    5. Exempt-ветка: для каждого вызова broadcast_calls тела - внутри writer-диапазона -> BroadcastInsideWriter (тот же Kind, что у Broadcasts), иначе ExemptBroadcasts с сообщением "Exempt-функция рассылает: либо вердикт Broadcasts, либо убрать вызов (освобождение = 'эта функция не шлёт событие')". Обоснование: причина каждой из 21 строки Exempt утверждает "события нет из этой функции" (примитив в транзакции вызывающего, обёртка над рассылающим, справочник, посев при старте); рассылка в такой функции противоречит собственной причине и удваивает событие. Счётчик exempt_checked++ для каждой обработанной Exempt-строки.
    6. `pub struct ScanStats { pub broadcasts_ordered: usize, pub exempt_checked: usize }` и `pub fn scan_with_stats(rel, src, registry) -> (Vec<Violation>, ScanStats)`; `scan` становится тонкой обёрткой (сигнатура и поведение остальных 34 тестов не меняются).
    7. Для Task 2 экспортировать `pub fn writer_call_ranges(p: &Prepared, f: &FnInfo) -> Vec<(usize, usize)>` (тонкая обёртка над writer_ranges по телу функции), чтобы тест мог вставлять вызов внутрь writer-скобок по позиции.
    8. registry.rs: ТОЛЬКО doc-комментарий вида `Verdict::Exempt` в шапке - добавить, что Exempt-функция механически не должна содержать broadcast_entities. Ни одна строка REGISTRY не меняется (118 строк, нижние границы 110 не трогать).
    9. В шапке entities_changed_gate.rs обновить перечень тестов (новые selftest, свипы - они появятся в Task 2).

    Если на реальных исходниках новые правила дадут красный (проверяется запуском every_service_mutation_has_verdict_and_broadcasts) - см. раздел "Если укрепление вскроет нарушение" ниже. НЕ ослаблять правило, НЕ править реестр, НЕ добавлять исключений в сканер ради зелёного.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test entities_changed_gate 2>&1 | tail -n 15</automated>
  </verify>
  <done>
    Все старые 34 теста зелёные + новые selftest зелёные; боевой `every_service_mutation_has_verdict_and_broadcasts` зелёный на неизменённых исходниках (порядок на всех 31 точке верен - иначе действует раздел про вскрытое нарушение); `git diff --stat` показывает изменения только в трёх test-файлах; REGISTRY по-прежнему 118 строк (31/21/66). При exit 69 на линковке - префикс DEVELOPER_DIR=/Library/Developer/CommandLineTools. Один cargo за раз.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Мутации на РЕАЛЬНЫХ исходниках (свипы по всему реестру + именованные переносы) и тест "гейт что-то измерил"</name>
  <files>crates/trackly-app/tests/entities_changed_gate.rs</files>
  <behavior>
    - mutation_early_broadcast_in_every_broadcasts_row_turns_gate_red: для КАЖДОЙ строки Broadcasts реестра (все 31) вставка вызова в самое начало тела -> has(BroadcastBeforeWriter); до вставки у функции нарушений нет (разностная проверка).
    - mutation_broadcast_moved_before_write_on_real_sources_is_red: три именованных переноса с якорями, уникальность которых утверждена в коде (см. action).
    - mutation_broadcast_in_every_exempt_row_turns_gate_red: для КАЖДОЙ строки Exempt (все 21) вставка вызова в начало тела -> has(ExemptBroadcasts); для Exempt-функций, владеющих writer.execute, вставка внутрь первых writer-скобок -> has(BroadcastInsideWriter). Явно проверено, что среди обработанных строк есть group_place.rs и group_membership.rs (R-2 для T-41.7-30).
    - gate_new_checks_measured_something: сумма scan_with_stats по SCOPE_FILES на неизменённых исходниках: broadcasts_ordered == число Broadcasts-строк REGISTRY (и >= 30), exempt_checked == число Exempt-строк REGISTRY (и >= 20); ноль нарушений.
  </behavior>
  <action>
    Все мутации - в памяти (строки), файлы на диске тест не трогает (в файле гейта нет fs::write - это инвариант 41.7-14, сохранить). Файлы читать через существующий read_service.

    Свип по Broadcasts (per D-07): для каждой строки e с verdict Broadcasts: src = read_service(e.file); fns = parse_fns(&prepare(&src)); утвердить `assert_eq!(fns.iter().filter(|f| f.name == e.func).count(), 1)` (уникальность цели) и `assert_eq!(src.as_bytes()[body.0], b'{')`; разностная проверка - в scan(e.file, &src, REGISTRY) нет BroadcastBeforeWriter для e.func; вставить в позицию body.0 + 1 текст вызова self.broadcast_entities с меткой e.func и пустыми Vec::new() (позиция - граница символа, это ASCII `{`); scan мутанта обязан иметь has(.., e.func, Kind::BroadcastBeforeWriter). Счётчик проверенных строк утвердить равным числу Broadcasts-строк REGISTRY (loud: свип по нулю строк невозможен).

    Именованные переносы (per D-04; семантика "перенёс вызов выше записи", а не "добавил лишний"):
    (A) place_service.rs::create - "перенос": убрать оригинал по якорю `self.broadcast_entities("create", place_ids, Vec::new(), Vec::new());` и вставить вызов сразу после якоря-сигнатуры `pub async fn create(&self, caller: &Identity, new: PlaceNew) -> Result<PlaceDto, AppError> {`;
    (B) place_service.rs::archive - форма с делегированием: вставить вызов broadcast_entities("archive", ..) непосредственно ПЕРЕД якорем `self.set_archived(caller.user_id, id, version, true, "archive")` (доказывает, что делегированная запись считается точкой записи);
    (C) device_service.rs::create - транзитивное делегирование: вставить вызов перед якорем `let outcome = self.create_without_broadcast(new).await?;` (доказывает фикспойнт create -> create_without_broadcast -> insert_new_and_get).
    Для КАЖДОГО якоря: `assert_eq!(src.matches(anchor).count(), 1, "якорь неуникален или исчез")` ДО замены; мутация через replacen(anchor, new_text, 1) только после этого утверждения; `assert_ne!(mutated, src)`; разностная проверка (до - у функции нет BroadcastBeforeWriter, после - есть). Если форматирование исходника изменилось и якорь пропал - тест падает громко, якорь править осознанно, не ослаблять до неуникального.

    Свип по Exempt (per D-08): для каждой строки e с verdict Exempt: те же утверждения уникальности и `{`; разностная проверка (в базовом scan нет ExemptBroadcasts/InsideWriter для e.func); вставка вызова в body.0 + 1 -> has(ExemptBroadcasts). Затем, если FnInfo.owns_write: взять первый диапазон из writer_call_ranges(&prepare(&src), &fi) и вставить тот же текст в позицию open + 1 -> has(BroadcastInsideWriter). Утвердить: число обработанных Exempt-строк == числу Exempt-строк REGISTRY; множество файлов обработанных строк содержит "group_place.rs" и "group_membership.rs"; число Exempt-функций, владеющих writer, >= 3 (set_archived, insert_new_and_get, строки group_type_service - не привязываться к точному числу).

    Тест счётчиков gate_new_checks_measured_something (per D-08 "зелёный, потому что ничего не измерил - недопустим"): суммировать ScanStats по SCOPE_FILES через scan_with_stats(f, &read_service(f), REGISTRY); числа сверять с `REGISTRY.iter().filter(..).count()` и с нижними границами 30 / 20 (по прецеденту MIN_REGISTRY_ROWS = 110: небольшой запас, не точное совпадение с константой).

    Не менять MIN_REGISTRY_ROWS, MIN_FNS_IN_SCOPE и существующие тесты. Прогнать rustfmt только по своим файлам: `rustfmt --edition <edition из Cargo.toml> --check crates/trackly-app/tests/entities_support/scan.rs crates/trackly-app/tests/entities_changed_gate.rs` (НЕ `cargo fmt -p trackly-app` и не `cargo fmt --all` без --check - чужие файлы не переформатировать).
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test entities_changed_gate 2>&1 | tail -n 12 && cargo fmt --all -- --check 2>&1 | tail -n 5</automated>
  </verify>
  <done>
    Тестов в entities_changed_gate стало больше 34, все зелёные, 0 failed / 0 filtered out; `cargo fmt --all -- --check` чист; свипы обработали ровно 31 Broadcasts-строку и ровно 21 Exempt-строку; в выводе явно отражено, что group_place.rs и group_membership.rs попали в свип. Для тестов слоя (2) (`entities_changed_places` и др.) повторный прогон не нужен - продакшн-код не менялся; один cargo за раз.
  </done>
</task>

<task type="auto">
  <name>Task 3: Доказательство на диске (RED -> откат -> GREEN), документация, privacy-гейт, коммиты</name>
  <files>.planning/phases/41.7-v1-4-ws-bootstrap-lan/41.7-REVIEW.md, .planning/phases/41.7-v1-4-ws-bootstrap-lan/41.7-SECURITY.md, .planning/todos/pending/2026-10-09-review-41.7-residual-findings.md</files>
  <action>
    ЧАСТЬ A - доказательство кусания на НАСТОЯЩИХ файлах (требование проекта: нарушение вносится в реальный исходник, показывается красный, откат, зелёный). Скрипт писать в scratchpad (не в репозиторий), запускать `python3 -I script.py`. Перед стартом `git status --short` обязан быть чистым по crates/trackly-app/src/services/ (иначе откат `git checkout` потеряет чужую правку - тогда остановиться).
    Три мутации ОДНОВРЕМЕННО (одна компиляция вместо трёх), каждая через `assert src.count(anchor) == 1` ДО замены:
      1. place_service.rs::create (WR-06): после якоря-сигнатуры create вставить `self.broadcast_entities("create", Vec::new(), Vec::new(), Vec::new());` - компилируется (приватный метод с такой сигнатурой).
      2. place_service.rs::set_archived (WR-07, Exempt, внутри writer-замыкания): перед якорем `repo.archive(conn, id, version, now)?;` вставить `let broadcast_entities = |_: &str| {}; broadcast_entities("probe");` - локальное замыкание с этим именем компилируется, а регэксп сканера `\bbroadcast_entities\s*\(` его видит.
      3. group_place.rs::apply_group_place_to_device_in_tx (WR-07, Exempt вне writer): сразу после `{` тела этой функции (найти от сигнатуры `pub(crate) fn apply_group_place_to_device_in_tx(` первую `{`, утвердить единственность сигнатуры) вставить ту же пару `let broadcast_entities = ...; broadcast_entities("probe");`.
    Запустить `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test entities_changed_gate every_service_mutation_has_verdict_and_broadcasts` - ОБЯЗАН быть красным ровно с тремя нарушениями: place_service.rs::create [BroadcastBeforeWriter], place_service.rs::set_archived [BroadcastInsideWriter], group_place.rs::apply_group_place_to_device_in_tx [ExemptBroadcasts]. Сохранить сырой вывод в scratchpad. Если красных меньше трёх - проверка НЕ кусается, вернуться в Task 1 (не продолжать).
    Откат: `git checkout -- crates/trackly-app/src/services/place_service.rs crates/trackly-app/src/services/group_place.rs`; `git status --short crates/trackly-app/src/services/` обязан быть пустым. Повторить тот же тест - ЗЕЛЁНЫЙ. Затем полный `cargo test -p trackly-app --test entities_changed_gate` - зелёный.

    ЧАСТЬ B - privacy и коммит кода: `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256`; в диффе только вымышленные идентификаторы; `git diff --stat` = ровно три test-файла (scan.rs, registry.rs - только комментарии, entities_changed_gate.rs). Закоммитить код отдельным коммитом (сообщение по-русски: `test(41.7): гейт EntitiesChanged - порядок рассылки после записи и механика Exempt (WR-06, WR-07)`, плюс строка Co-Authored-By из системной вставки). Получить хэш: `git rev-parse --short HEAD`.

    ЧАСТЬ C - три документа (хэш кода подставить в каждый):
      1. 41.7-REVIEW.md, frontmatter disposition: fixed -> [WR-01, WR-02, WR-03, WR-06, WR-07]; deferred -> [WR-04, WR-05, WR-08, WR-09]; fix_commits добавить хэш; disposition_updated оставить датой правки. Под заголовками WR-06 и WR-07 добавить по одной строке "**Resolution:** fixed in <hash> ..." с названием новых Kind. Тело находок не переписывать.
      2. 41.7-SECURITY.md: в строках T-41.7-02, T-41.7-21, T-41.7-30, T-41.7-38 убрать оговорку про остаток и поставить "closed (остаток R-1/R-2 снят <hash>)", заменив устаревшие ссылки на строки scan.rs:544-556 на имена Kind (BroadcastInsideWriter, BroadcastBeforeWriter, ExemptBroadcasts); в таблице Residual Findings строки R-1 и R-2 пометить CLOSED <hash> с одной фразой что именно теперь проверяется и чем доказано (свипы по 31 и 21 строке + on-disk RED/GREEN), R-3 и R-4 не трогать; во вводной фразе секции и в строке Approval поправить "R-1..R-4" на "R-3, R-4 остаются; R-1, R-2 закрыты". status/threats_open не менять (`SECURED`, 0). Перед правкой прочитать frontmatter файла - если там есть счётчик остатков, обновить.
      3. .planning/todos/pending/2026-10-09-review-41.7-residual-findings.md: удалить пункты WR-06 и WR-07 из "Отложенных находок"; в frontmatter `files:` убрать scan.rs и registry.rs, если ни один оставшийся пункт их не касается; абзац "Почему это не закрыли сразу" и таблицу подтверждения аудитом поправить в прошедшее время / пометить строки R-1 и R-2 закрытыми; WR-04, WR-05, WR-08, WR-09, Info и хвост WR-02 оставить как есть. Заголовок/title не менять.
    Проверка перед вторым коммитом: grep по трём документам и по коду на реквизиты не нужна, но `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` повторить; закоммитить документы отдельным коммитом `docs(41.7): WR-06 и WR-07 закрыты - REVIEW, SECURITY, todo`. Пуш не делать, если оркестратор не попросил (по памяти проекта push origin main заранее разрешён пользователем, решение за оркестратором).
    Написать SUMMARY.md в каталоге этого quick-плана: что изменено, вывод RED (три нарушения) и GREEN после отката, счётчики 31/21, число тестов, и РАЗДЕЛ "Нарушения, вскрытые укреплением" (по умолчанию: "нет - порядок верен на всех 31 точке, Exempt не рассылает"; иначе - перечень с file:line).
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && git status --short crates/trackly-app/src/ && node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256 && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test entities_changed_gate 2>&1 | tail -n 5 && ! grep -q "^- \*\*WR-0[67]" .planning/todos/pending/2026-10-09-review-41.7-residual-findings.md</automated>
  </verify>
  <done>
    Три мутации на диске дали красный ровно с тремя ожидаемыми нарушениями разных Kind, после `git checkout` тот же тест зелёный, `git status` по src/ чист; код закоммитан (хэш назван), документы обновлены (REVIEW: WR-06/WR-07 в fixed с хэшем; SECURITY: R-1/R-2 закрыты, R-3/R-4 остались; todo: WR-06/WR-07 сняты, остальное цело), privacy-гейт зелёный, SUMMARY.md написан.
  </done>
</task>

</tasks>

## Если укрепление вскроет нарушение в текущем коде

Базовая гипотеза (верификатор фазы и аудитор независимо перечитали все 31 точку): порядок верен, Exempt не рассылает. Если новые правила всё же краснят боевой скан:

1. НЕ ослаблять правило, НЕ править REGISTRY (в том числе не переводить строку в Exempt и не менять вердикт), НЕ добавлять исключение в сканер ради зелёного.
2. Прочитать красную функцию целиком и решить, что это:
   - НАСТОЯЩИЙ дефект порядка (рассылка действительно до записи / Exempt действительно шлёт): это находка фазы 41.7. Остановиться перед коммитом, не маскировать; вернуть оркестратору `## CHECKPOINT` с file:line, видом нарушения и вариантами (a - перенести один вызов в продакшн-коде в этой же задаче с поведенческим тестом из `entities_changed_*.rs`; b - отложить). Красный гейт в main не коммитить.
   - ЛОЖНОЕ срабатывание из-за неточности сканера (например, вызов помощника, который сканер считает владельцем записи, хотя запись в нём идёт ДО рассылки, или имя метода совпало у двух impl): сузить правило точечно и добавить selftest, воспроизводящий именно этот ложный случай; в SUMMARY описать функцию, причину и как правило сужено. Сужение не должно снимать проверку с настоящих форм из selftest Task 1.
3. В любом случае факт и решение записать в раздел SUMMARY "Нарушения, вскрытые укреплением".

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| тест-инфраструктура -> продакшн-код | Меняются только файлы под tests/ и документация; src/ трогается лишь временными on-disk мутациями Task 3 с откатом `git checkout` |
| исходник сервисов -> сканер | Сканер читает исходник как текст; ложно-зелёный сканер ослабляет гарантию D-04 |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-ul8-01 | Tampering | Broadcasts-ветка scan.rs | mitigate | проверка "после ПОСЛЕДНЕЙ точки записи" + BroadcastWithoutWrite; свип по 31 строке и три именованных переноса на реальных исходниках, on-disk RED/GREEN |
| T-ul8-02 | Tampering | Exempt-ветка scan.rs | mitigate | ExemptBroadcasts / BroadcastInsideWriter на всех 21 строках, включая group_place.rs и group_membership.rs (T-41.7-30) |
| T-ul8-03 | Repudiation | гейт "зелёный, ничего не измерив" | mitigate | gate_new_checks_measured_something: счётчики == числу строк REGISTRY + нижние границы |
| T-ul8-04 | Tampering | мутационная проба снимает гейт у соседней функции | mitigate | assert_eq!(src.matches(anchor).count(), 1) для каждого якоря; в свипах - уникальность имени функции и проверка `{` по позиции |
| T-ul8-05 | Information Disclosure | публичный репозиторий | mitigate | только вымышленные идентификаторы в selftest; check-privacy перед каждым коммитом; в SUMMARY/документах нет реальных данных |
| T-ul8-06 | Tampering | on-disk мутация остаётся в дереве | mitigate | `git status --short crates/trackly-app/src/` обязан быть пуст после отката; проверка в verify Task 3 |
</threat_model>

<verification>
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test entities_changed_gate` - все тесты зелёные (34 прежних + новые), 0 failed / 0 filtered out.
- On-disk RED: три одновременные мутации -> ровно три нарушения трёх Kind; после `git checkout` - зелёный.
- `cargo fmt --all -- --check` чист (до задачи был чист).
- `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` - зелёный перед каждым коммитом.
- Полный пакет НЕ запускать (продакшн-код не менялся); один cargo за раз.
</verification>

<success_criteria>
- WR-06: рассылка до записи (в начале, между двумя записями, до делегированной и до транзитивно делегированной записи) краснит гейт; доказано на реальных place_service.rs / device_service.rs и свипом по всем 31 строке Broadcasts.
- WR-07: Exempt-функция, которая рассылает, краснит гейт; доказано свипом по всем 21 строке Exempt, включая group_place.rs и group_membership.rs, и on-disk мутациями.
- Уникальность каждого якоря утверждена в коде; гейт не зелёный при нулевом измерении.
- 118 строк REGISTRY и все прежние 34 теста не тронуты; REVIEW, SECURITY и todo больше не лгут.
- Нарушения, вскрытые укреплением, показаны в SUMMARY (или явно "нет"), не замазаны.
</success_criteria>

<output>
После завершения создать `.planning/quick/261009-ul8-41-7-wr-06-writer-wr-07-verdict-exempt/SUMMARY.md`
</output>
