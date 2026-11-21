---
phase: quick-261121-hjw
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-app/tests/entities_support/registry.rs
  - crates/trackly-app/tests/entities_support/scan.rs
  - crates/trackly-app/tests/entities_changed_gate.rs
  - crates/trackly-app/src/services/auth.rs
  - crates/trackly-app/src/http/auth.rs
  - crates/trackly-app/src/tauri_cmds/auth.rs
  - crates/trackly-app/tests/auth_bootstrap.rs
  - crates/trackly-app/tests/desktop_system_admin.rs
  - crates/trackly-app/tests/http_route_parity.rs
  - crates/trackly-app/src/services/cartridge_service.rs
  - crates/trackly-app/src/services/device_service.rs
  - ui/src/lib/api/ws.ts
  - ui/scripts/check-ws-disconnect-toast.mjs
  - ui/scripts/check-place-tree-invalidation.mjs
  - .planning/REQUIREMENTS.md
  - .planning/ROADMAP.md
  - .planning/PROJECT.md
  - .planning/STATE.md
  - .planning/todos/pending/2026-11-21-wr02-multi-execute-transactional.md
  - scripts/check-privacy.mjs
  - scripts/check-privacy.selftest.mjs
  - scripts/fixtures/privacy/README.md
  - scripts/fixtures/privacy/wrapped-marker.md
  - scripts/fixtures/privacy/paragraph-break.md
  - .planning/todos/pending/2026-10-09-review-41.7-residual-findings.md
  - .planning/todos/completed/2026-10-09-review-41.7-residual-findings.md
  - .planning/todos/pending/2026-09-26-privacy-gate-line-wrap-and-scope-gaps.md
  - .planning/todos/completed/2026-09-26-privacy-gate-line-wrap-and-scope-gaps.md
  - crates/trackly-app/src/services/attachment_admin.rs
  - crates/trackly-app/tests/attachments_integrity.rs
  - crates/trackly-app/tests/backup_service.rs
  - ui/src/features/attachments/AttachmentViewer.svelte
  - ui/scripts/check-attachments-block.mjs
  - .github/workflows/release.yml
  - .github/workflows/heic-spike.yml
  - .planning/phases/41.3-attachments-storage/41.3-SECURITY.md
autonomous: true
requirements: [TODO-2026-10-09-41.7-RESIDUAL, TODO-2026-09-26-PRIVACY-GATE, WR-04, WR-05, WR-07, WR-08, WR-09, SEC-41.3-OBS-4, SEC-41.3-OBS-5, SEC-41.3-OBS-7, SEC-41.3-OBS-8]

must_haves:
  truths:
    - "WR-07: Exempt-функция, владеющая записью (writer.execute в теле или транзитивный self-вызов владельца записи), обязана стоять в реестре отдельным громким вердиктом ExemptWrites (конструктор exw); обычный ex на такой функции краснит гейт (Kind::ExemptOwnsWriter), exw на функции без записи краснит (Kind::ExemptWritesWithoutWrite); свип по реальным исходникам доказывает, что снятие exw→ex у каждой такой строки краснеет"
    - "WR-05: аудит custom:bootstrap, записанный через LAN-маршрут, содержит IP пира в payload_json ({login, peer}); на пути Tauri peer = null; тест bootstrap_route_open_when_needs_bootstrap это проверяет"
    - "WR-08: проба GET /apiary в http_route_parity выполняется и без собранного ui/dist; внутри if ui_bundled остаётся только GET /"
    - "IN-01/IN-02/IN-03: transition картриджа рассылает событие ДО read-back; auth_bootstrap-тесты не протекают TempDir; расхождение «без места» у устройства и картриджа объяснено комментарием в обоих местах"
    - "WR-04: висящий таймер переподключения отменяется в activeCleanup (release/disconnectWs) и сам не открывает сокет при refCount 0; гейт check-ws-disconnect-toast краснеет на дореформенном ws.ts из git и на мутантах без хранения/отмены таймера"
    - "WR-09: гейт check-place-tree-invalidation требует ровно один $effect PlaceTree с вызовом reloadSeqNow() вне untrack рядом с loadTree(), а reloadSeqNow обязан возвращать placeContentEventsStore.reloadSeq; удаление вызова или подмена поля краснит гейт"
    - "Коллизия INT-NN снята документально: отложенное требование печати INT-02 переименовано в PAG-01 в живых документах (REQUIREMENTS, ROADMAP-бэклог, PROJECT, STATE), в REQUIREMENTS записано правило: префикс INT-NN закреплён за находками интеграционного чекера аудитов вех; исторические артефакты не переписаны"
    - "Хвост WR-02 не потерян: многократные writer.execute в сервисах перечислены (файл::функция) в новом pending-todo вместе с UF-1/UF-2 и пунктом живой UAT остановки сервера"
    - "Гейт приватности ловит значение, разорванное переносом строки внутри абзаца (в том числе за префиксами комментариев // и #), и не срабатывает через пустую строку; фикстура wrapped-marker.md с вымышленным «Пчёлкин Артём» даёт exit 1 на новом гейте и exit 0 на дореформенном гейте из git"
    - "Гейт приватности видит текст в cp1251 и UTF-16 (не только UTF-8) и текстовые чанки PNG (tEXt/zTXt/iTXt); расширен список бинарных расширений (doc/xls/odt/zip/sqlite/… — класс D)"
    - "--add больше не тупик для значения длиннее NGRAM_MAX_WORDS: значение раскладывается на покрывающий набор смежных 3-грамм, оператору показано, сколько под-фраз уже встречается в репозитории; round-trip разложения проверен selftest-ом на вымышленном значении"
    - "Охват каталогов закреплён selftest-ом: пути в .planning/{todos,quick,debug,threads,milestones,phases}, templates/, тестовых фикстурах и ui/scripts/fixtures не исключены ни одним предикатом гейта"
    - "Оба todo перенесены в .planning/todos/completed/ с полями resolved и resolution"
    - "41.3 набл. 5: скан целостности удаляет блоб-сироту только после повторной проверки ссылок внутри замыкания единственного писателя; блоб, на который к моменту удаления появилась строка, остаётся на диске (тест на устаревшем списке кандидатов)"
    - "41.3 набл. 7: iframe предпросмотра PDF либо несёт sandbox с минимальными разрешениями и PDF открывается в WKWebView, WebView2 и LAN-браузере, либо (если sandbox ломает встроенный просмотрщик) sandbox не ставится и это записано принятым риском в 41.3-SECURITY.md и в гейте check-attachments-block — функция предпросмотра не сломана ни в одном исходе"
    - "41.3 набл. 8: тест (cfg(unix)) доказывает, что копирование вложений при бэкапе/переносе БД пропускает симлинк-шард и симлинк-файл, а обычные блобы копирует"
    - "41.3 набл. 4: vcpkg в release.yml (и heic-spike.yml) берётся с закреплённого тега/коммита microsoft/vcpkg, а не из образа раннера"
  artifacts:
    - path: "crates/trackly-app/tests/entities_support/registry.rs"
      provides: "Verdict::ExemptWrites + конструктор exw"
      contains: "ExemptWrites"
    - path: "crates/trackly-app/tests/entities_support/scan.rs"
      provides: "Kind::ExemptOwnsWriter, Kind::ExemptWritesWithoutWrite"
      contains: "ExemptOwnsWriter"
    - path: "ui/src/lib/api/ws.ts"
      provides: "reconnectTimer с отменой в activeCleanup"
      contains: "clearTimeout"
    - path: "scripts/check-privacy.mjs"
      provides: "межстрочный проход n-грамм, декодирование cp1251/UTF-16, PNG-текст, decomposeLongValue"
      contains: "decomposeLongValue"
    - path: "scripts/fixtures/privacy/wrapped-marker.md"
      provides: "вымышленный маркер, разорванный переносом строки"
    - path: ".planning/todos/pending/2026-11-21-wr02-multi-execute-transactional.md"
      provides: "перечень сервисных методов с ≥2 writer.execute (хвост WR-02)"
    - path: "crates/trackly-app/src/services/attachment_admin.rs"
      provides: "удаление сирот с перепроверкой count_by_hash_on в writer.execute"
      contains: "count_by_hash_on"
    - path: ".github/workflows/release.yml"
      provides: "vcpkg с закреплённой версии"
      contains: "bootstrap-vcpkg"
  key_links:
    - from: "crates/trackly-app/src/http/auth.rs::handler_auth_bootstrap"
      to: "AuthService::bootstrap_first_admin"
      via: "ConnectInfo<SocketAddr> → peer: Option<String>"
      pattern: "bootstrap_first_admin\\(.*peer"
    - from: "scripts/check-privacy.mjs::scanHashes"
      to: "межстрочный проход по словам абзаца"
      via: "окна 2..NGRAM_MAX_WORDS, пересекающие перенос строки"
      pattern: "NGRAM_MAX_WORDS"
    - from: ".githooks/pre-commit и .github/workflows/ci-fast.yml"
      to: "scripts/check-privacy.mjs"
      via: "--staged / HEAD-режим, оба через scanHashes"
      pattern: "check-privacy.mjs"
---

<objective>
Пакет F: закрыть два просроченных todo вехи v1.4.

1. `.planning/todos/pending/2026-10-09-review-41.7-residual-findings.md` — остаток код-ревью 41.7: WR-04, WR-05, WR-08, WR-09, Info IN-01..03, коллизия идентификаторов INT-NN, хвост WR-02, плюс остаток WR-07 из аудита вехи (`v1.4-MILESTONE-AUDIT.md`, tech_debt 41.7: «Verdict::Exempt не проверяется механически»).
2. `.planning/todos/pending/2026-09-26-privacy-gate-line-wrap-and-scope-gaps.md` — гейт приватности построчный, потолок `NGRAM_MAX_WORDS = 3`, пробелы охвата.
3. Дополнение координатора: наблюдения 4, 5, 7, 8 из `.planning/phases/41.3-attachments-storage/41.3-SECURITY.md` (раздел «наблюдения», не блокеры). Наблюдение 3 (самотест гейта приватности не в CI) закрывает пакет 261121-heg (WR-13) — здесь НЕ дублируется; наблюдения 1, 2, 6 — не код.

Purpose: долги, которые ослабляют реестровый гейт и гейт приватности — главные механические гарантии проекта; их держат следующие фазы.
Output: правки гейтов и кода, новые фикстуры, документальное разведение INT-NN, новое pending-todo для транзакционного редизайна (хвост WR-02), оба исходных todo в completed/.

Порядок исполнения: этот пакет идёт ПОВЕРХ коммитов параллельных пакетов 261121-hed/hef/heg/heh/hei. Они трогают те же файлы (`registry.rs` — hed; `http_route_parity.rs`, `http/auth.rs`, `tauri_cmds/auth.rs` — hei; `cartridge_service.rs`, `device_service.rs` — heh/hef/hed/hei). Перед правкой КАЖДОГО файла перечитать его на текущем HEAD; номера строк ниже — ориентир на 2026-11-21, не истина. Пункты этих пакетов не дублировать: мёртвое сохранение TLS-сертификата (hei), WR-13 / подключение `check-privacy.selftest.mjs` в CI (heg) — не наши.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@./CLAUDE.md
@.planning/todos/pending/2026-10-09-review-41.7-residual-findings.md
@.planning/todos/pending/2026-09-26-privacy-gate-line-wrap-and-scope-gaps.md
@.planning/phases/41.7-v1-4-ws-bootstrap-lan/41.7-REVIEW.md

## Сверка с HEAD (сделана планировщиком 2026-11-21) — что уже закрыто, что нет

| Пункт | Статус на HEAD | Что делаем |
|---|---|---|
| WR-06 | ЗАКРЫТ 0db2d02b (BroadcastBeforeWriter/BroadcastWithoutWrite) | не трогаем |
| WR-07 | ЧАСТИЧНО: 0db2d02b краснит Exempt-функцию, которая РАССЫЛАЕТ (ExemptBroadcasts). Предложенная ревью часть — «Exempt-функция, владеющая записью, должна быть отдельным громким вердиктом» — НЕ сделана: `ex(...)` на пишущей функции по-прежнему молчит, поэтому аудит вехи держит WR-07 в tech_debt | Задача 1: ExemptWrites/exw |
| WR-04 | ОТКРЫТ: `ui/src/lib/api/ws.ts` ~стр. 144 — `setTimeout(() => connectBrowser(), delay)` без хранения; `activeCleanup` закрывает только `if (ws)` | Задача 2 |
| WR-05 | ОТКРЫТ: `bootstrap_first_admin(&self, new: UserNew)` (`services/auth.rs` ~1518) не получает peer; `build_auth_bootstrap` (`http/auth.rs` ~194) зовёт без него; ConnectInfo вставляется в `server/mod.rs` ~223 | Задача 1 |
| WR-08 | ОТКРЫТ: `/apiary` внутри `if ui_bundled` (`http_route_parity.rs` ~669-684) | Задача 1 |
| WR-09 | ОТКРЫТ: `PlaceTree.svelte` ~305 `void reloadSeqNow();`, гейт INV-1/INV-4 видит только эффекты с именем стора | Задача 2 |
| IN-01 | ОТКРЫТ: `cartridge_service.rs` ~770 — `self.get(..)` раньше `broadcast_entities("transition", ..)` | Задача 1 |
| IN-02 | ОТКРЫТ: `auth_bootstrap.rs:36` `dir.keep()` | Задача 1 |
| IN-03 | ОТКРЫТ (асимметрия без комментария) | Задача 1, только комментарии |
| Decision objections | «None» в REVIEW.md | нечего делать, отметить в resolution |
| Коллизия INT-NN | ЧАСТИЧНО: 1a665bd7 внёс 11 отложенных ID в таблицу трассируемости и примечание (REQUIREMENTS.md ~565-567); само требование по-прежнему называется INT-02 (стр. ~419, ~558) | Задача 2: переименовать в PAG-01 (PRN занят, PAG свободен — проверено грепом) |
| Дрейф трассируемости (12 ID) | ЗАКРЫТ 1a665bd7 (строки MAPX/GRPX/QA-05/INT-02/DOC-12/PRIV-03 есть) | только переименование строки INT-02 |
| Хвост WR-02 | ОТКРЫТ по существу; (1) задокументирован у константы DRAIN_GRACE; (2) нужен транзакционный редизайн нескольких сервисных методов — объём отдельной фазы (законная причина: стоимость контекста), (3) живая UAT | Задача 2: инвентарь + новое pending-todo, не редизайн |
| Гейт приватности п.1 (перенос строки) | ОТКРЫТ: `scanHashes` режет `content.split('\n')`, `extractNgrams(line)` построчно | Задача 3 |
| п.2 (NGRAM_MAX_WORDS) | ОТКРЫТ: `canonicalizeAddValue` отказывает `too_many_words` | Задача 3 |
| п.3 (охват) | Авто-гейт уже сканирует ВСЕ `git ls-files` (исключены только node_modules/, target/, Cargo.lock, сам файл хэшей, scripts/fixtures/privacy/ в авто-режиме) — .planning/ покрыт. Реальные слепые зоны, найденные планировщиком: (а) `crates/trackly-app/tests/fixtures/devices/cp1251_*.csv` — читаются как UTF-8, кириллица превращается в U+FFFD, режим 2 по ним слеп; (б) PNG под BINARY_ALLOWLIST (иконки, logo_test.png) — текстовые чанки не разбираются; (в) .doc/.xls/.odt/.zip/.sqlite и т.п. не входят в BINARY_EXTENSIONS — проходят как «текст». Ручная разведка фазы 37 не покрыла .planning/todos — это человеческая разведка, не дыра гейта | Задача 3 |

## Откуда гейт приватности берёт запрещённые значения (ответ на вопрос оркестратора)

Открытого текста запрещённых значений в репозитории НЕТ и локального незакоммиченного файла с ними тоже нет. Гейт читает обязательный `--hashes <path>`: продовый `scripts/privacy-tokens.sha256` (закоммичен, строки `<sha256-hex> <класс A|B|C>`, sha256 от нормализованной n-граммы: lowercase + ё→е + NFC, слова через один пробел). Значения заносит человек интерактивно `node scripts/check-privacy.mjs --add --hashes scripts/privacy-tokens.sha256` (только TTY). Для проверок есть ВЫМЫШЛЕННЫЙ реестр `scripts/fixtures/privacy/tokens.fixture.sha256` — единственная строка: хэш «пчелкин артем» (класс B), открытым текстом в `with-marker.md`. Значит, мутационная проверка делается на вымышленном «Пчёлкин Артём» с fixture-хэшами; продовые хэши используются только для итогового прогона HEAD (должен остаться PASS).

<interfaces>
registry.rs (сейчас): `pub enum Verdict { Broadcasts, Exempt(&'static str), ReadOnly }`; конструкторы `const fn b(file, func)`, `ro(file, func)`, `ex(file, func, reason)`; `MIN_EXEMPT_REASON_CHARS = 40`; `SCOPE_FILES`; `REGISTRY`.
scan.rs: `pub enum Kind { NoVerdict, MissingBroadcast, BroadcastInsideWriter, ReadOnlyOwnsWriter, ReadOnlyCallsWriter, StaleRow, DuplicateFn, DeadCodeAllowLeft, BroadcastBeforeWriter, BroadcastWithoutWrite, ExemptBroadcasts }`; `pub struct ScanStats { broadcasts_ordered, exempt_checked }`; `FnInfo { name, is_pub, owns_write, body: (usize, usize) }`; внутри scan есть `write_points(&p, f, &writing)` (транзитивные точки записи) и арм `Verdict::Exempt(_) => {...}` (~стр. 691).
entities_changed_gate.rs: селфтесты `selftest_*` на синтетике через `row(func, verdict)`, `has(&vs, func, Kind)`, `kinds(..)`; свип `mutation_broadcast_in_every_exempt_row_turns_gate_red` фильтрует `matches!(e.verdict, Verdict::Exempt(_))` — все такие фильтры обязаны учесть новый вариант.
services/auth.rs: `pub async fn bootstrap_first_admin(&self, mut new: UserNew) -> Result<UserDto, AppError>`; внутри writer-замыкания `insert_user_in_tx(&tx, None, Some("custom:bootstrap"), false, &NewUserRow { login, full_name, password_hash, role, email, now })`. Вызывающие: `http/auth.rs::build_auth_bootstrap(ctx, payload)` (из `handler_auth_bootstrap`), `tauri_cmds/auth.rs` ~64, `tests/desktop_system_admin.rs` ~98.
check-privacy.mjs: `WORD_RE = /[\p{L}\p{N}]+/gu`, `NGRAM_MAX_WORDS = 3`, `normalize`, `extractNgrams(line)`, `sha256Hex`, `canonicalizeAddValue(value) -> {ok, canonical, words, hash} | {ok:false, reason}`, `scanHashes(targets, hashMap)` (построчно, `violations.push({path, line, cls})`), `formatViolation` (печатает путь:строку и класс, НИКОГДА значение), сборщики `collectStagedTargets` (execFileSync git show, encoding utf8), `collectHeadTargets` (fs.readFileSync utf8), `collectExplicitTargets`; `isExcludedPath`, `isAutoScanExcludedFixture`, `BINARY_EXTENSIONS`, `BINARY_ALLOWLIST_*`; экспорт `{ scanForbiddenPaths, canonicalizeAddValue, extractNgrams, normalize, sha256Hex, NGRAM_MAX_WORDS }`.
check-privacy.selftest.mjs: `runGate(args)`, `assertExitCode`, `assertTrue`; блок 10 (WR-04) держит `EXPECTED_FIXTURES` — явный состав каталога фикстур.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Задача 1: Rust — WR-07 (ExemptWrites), WR-05 (peer в аудите bootstrap), WR-08 (/apiary), IN-01..03</name>
  <files>crates/trackly-app/tests/entities_support/registry.rs, crates/trackly-app/tests/entities_support/scan.rs, crates/trackly-app/tests/entities_changed_gate.rs, crates/trackly-app/src/services/auth.rs, crates/trackly-app/src/http/auth.rs, crates/trackly-app/src/tauri_cmds/auth.rs, crates/trackly-app/tests/auth_bootstrap.rs, crates/trackly-app/tests/desktop_system_admin.rs, crates/trackly-app/tests/http_route_parity.rs, crates/trackly-app/src/services/cartridge_service.rs, crates/trackly-app/src/services/device_service.rs</files>
  <behavior>
    - selftest: синтетическая функция с writer.execute под ex(...) → Kind::ExemptOwnsWriter
    - selftest: та же под exw(...) → зелено; функция без записи под exw(...) → Kind::ExemptWritesWithoutWrite
    - selftest: функция, пишущая только транзитивно (self-вызов владельца записи), под ex(...) → ExemptOwnsWriter
    - свип по реальному REGISTRY: каждая exw-строка, временно подменённая в копии реестра на ex, даёт ExemptOwnsWriter; счётчик таких строк > 0 (защита от вакуумности)
    - auth_bootstrap: после POST /api/v1/auth_bootstrap c ConnectInfo(addr) payload_json аудита custom:bootstrap содержит addr.ip() строкой
    - http_route_parity: проба /apiary исполняется вне if ui_bundled
  </behavior>
  <action>
В начале исполнения записать `BASE=$(git rev-parse HEAD)` в SUMMARY — он нужен задаче 3 для проверки дореформенным гейтом.

WR-07 (остаток, аудит вехи). В registry.rs добавить вариант `Verdict::ExemptWrites(&'static str)` и конструктор `const fn exw(file, func, reason)`; обновить модульный doc-комментарий: Exempt = «не шлёт событие и не пишет сам»; ExemptWrites = «пишет, но событие осознанно не из этой функции» — громкий вердикт, видимый в диффе. Причина ExemptWrites подчиняется тому же `MIN_EXEMPT_REASON_CHARS` (проверить, что `registry_shape_is_valid` и `reason_of` учитывают оба варианта). В scan.rs добавить `Kind::ExemptOwnsWriter` и `Kind::ExemptWritesWithoutWrite`; в арме Exempt дополнительно к существующей проверке рассылки: если `f.owns_write` ИЛИ `write_points(..)` не пуст — нарушение ExemptOwnsWriter с сообщением «Exempt-функция пишет: вердикт exw(..) с причиной». Новый арм ExemptWrites: та же проверка рассылки (ExemptBroadcasts/BroadcastInsideWriter), плюс если точек записи нет — ExemptWritesWithoutWrite («устаревший exw: функция больше не пишет, верни ex»). В ScanStats добавить счётчик `exempt_writes_checked`. Затем прогнать гейт: каждую строку реестра, на которой зажглось ExemptOwnsWriter, перевести с `ex` на `exw` БЕЗ изменения текста причины (в SUMMARY — число таких строк и их file::func). Все `matches!(e.verdict, Verdict::Exempt(_))` в entities_changed_gate.rs расширить на оба варианта там, где смысл «освобождение», и добавить селфтесты из behavior плюс свип `mutation_exw_to_ex_in_every_writing_exempt_row_turns_gate_red` (мутирует копию REGISTRY, не файл). Учитывать, что пакет hed мог добавить строки в registry.rs — инвентарь брать с HEAD.

WR-05. Сигнатура `bootstrap_first_admin(&self, new: UserNew, peer: Option<String>)`. Для публичного пути payload_json аудита — JSON-объект с полями login и peer (peer = null на пути Tauri); путь create_user не меняется. Протащить через NewUserRow (новое поле `audit_peer: Option<&str>`) или отдельный параметр insert_user_in_tx — на усмотрение исполнителя, но формат payload у create_user обязан остаться прежним. Перед сменой формата грепнуть потребителей `custom:bootstrap` / payload_json аудита пользователей (журнал/история в UI и Rust) и, если кто-то читает payload как голый логин, поправить чтение. В handler_auth_bootstrap принять адрес пира экстрактором, терпимым к отсутствию (например `Option<axum::Extension<axum::extract::ConnectInfo<std::net::SocketAddr>>>`; убедиться, что компилируется на axum 0.8) и передать `ip().to_string()`; tauri_cmds/auth.rs и desktop_system_admin.rs передают None. Обновить doc-комментарий bootstrap_first_admin (D-13 теперь отвечает на вопрос «какой хост занял установку»). В auth_bootstrap.rs: в `bootstrap_route_open_when_needs_bootstrap` утверждение, что payload_json содержит IP из вставленного ConnectInfo; IN-02 — держать TempDir в возвращаемом кортеже (как `_dir` в ws_close_on_server_stop.rs) вместо `dir.keep()`.

WR-08. В http_route_parity.rs вынести пробу GET /apiary из блока `if ui_bundled`; внутри оставить только GET /. Утверждение то же: не JSON-404.

IN-01. В cartridge_service.rs `transition`: `broadcast_entities("transition", ..)` перенести перед `self.get(cartridge_id)` (как у соседей). Реестровый гейт порядка (после последней точки записи) остаётся зелёным — это проверит прогон app_gates.
IN-03. Однострочные комментарии у создания устройства без места (device_service.rs, device_ids при пустых place_ids) и у создания картриджа без места (cartridge_service.rs): почему устройство шлёт deviceIds, а картридж — ничего (счётчики дерева не меняются; устройство видно в списках, картридж без места — нет). Только комментарии.

Тесты/фикстуры — только вымышленные данные (127.0.0.1/192.0.2.x, «Иванов И.И.»).
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_gates -- entities_changed_gate http_route_parity entities_changed_cartridges && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_auth -- auth_bootstrap desktop_system_admin && cargo clippy -p trackly-app --all-targets -- -D warnings && cargo fmt --all -- --check</automated>
  </verify>
  <done>Новые селфтесты и свип зелёные, ExemptOwnsWriter на HEAD-реестре 0 после перевода строк на exw; на копии реестра с ex вместо exw — красный (доказано свипом); аудит bootstrap по LAN несёт peer; /apiary проверяется без ui/dist; IN-01..03 сделаны; clippy/fmt чисты. Команды cargo — по одной, не параллельно; при exit 69 префикс DEVELOPER_DIR=/Library/Developer/CommandLineTools.</done>
</task>

<task type="auto" tdd="true">
  <name>Задача 2: UI-гейты WR-04 и WR-09; документальное разведение INT-NN; инвентарь хвоста WR-02</name>
  <files>ui/src/lib/api/ws.ts, ui/scripts/check-ws-disconnect-toast.mjs, ui/scripts/check-place-tree-invalidation.mjs, .planning/REQUIREMENTS.md, .planning/ROADMAP.md, .planning/PROJECT.md, .planning/STATE.md, .planning/todos/pending/2026-11-21-wr02-multi-execute-transactional.md</files>
  <behavior>
    - check-ws-disconnect-toast: новое структурное правило W6 — результат setTimeout переподключения хранится в модульной переменной, browser-cleanup зовёт clearTimeout(этой переменной) и обнуляет её; колбэк таймера обнуляет переменную и не зовёт connectBrowser при refCount <= 0
    - мутанты M13 (таймер не сохраняется), M14 (clearTimeout убран из cleanup), M15 (снят guard refCount в колбэке) — красные; эталон — зелёный
    - check-place-tree-invalidation: INV-10 — ровно один $effect PlaceTree содержит reloadSeqNow( вне untrack и в нём же loadTree(; функция reloadSeqNow в сторе возвращает placeContentEventsStore.reloadSeq
    - мутанты в selftest (по РЕАЛЬНЫМ файлам в памяти): удалён вызов reloadSeqNow() — красно; вызов завёрнут в untrack — красно; reloadSeqNow возвращает .seq — красно
  </behavior>
  <action>
WR-04 (ws.ts): ввести модульную `reconnectTimer: ReturnType<typeof setTimeout> | null`; в onclose сохранять таймер, колбэк сначала обнуляет переменную и выходит, если refCount <= 0, иначе зовёт connectBrowser; в browser-ветке activeCleanup до блока `if (ws)` — clearTimeout и обнуление. disconnectWs идёт через activeCleanup — отдельной правки не требует (проверить). В check-ws-disconnect-toast.mjs добавить W6 и мутанты M13–M15 по образцу W1–W5/M1–M12 (текст без комментариев, пробелы схлопнуты); обновить шапку (перечень правил, «WR-04 ревью 41.7»). Мутационная проверка на реальном исходнике: `git show $BASE:ui/src/lib/api/ws.ts` в scratchpad, прогнать на нём проверку W6 (через `--src=` или временную копию дерева ui в scratchpad — как поддерживает скрипт; если флага нет, добавить `--file-ws=<path>` только для этой цели) → обязана быть красной; на текущем ws.ts — зелёной.

WR-09 (check-place-tree-invalidation.mjs): добавить INV-10 в checkTree (поиск по effectBlocks/untrackRanges, как INV-4) и проверку тела reloadSeqNow в checkStore (через functionBody или эквивалент для `export function`); мутанты в selftest — уникальным якорем (урок mutation_test_anchor_must_be_unique: замена `replace(...)` по паттерну, встречающемуся ровно один раз, иначе selftest-утверждение об уникальности). Обновить шапку скрипта (INV-10, WR-09 ревью 41.7). PlaceTree.svelte и стор НЕ меняются.

INT-NN (документально): в REQUIREMENTS.md отложенное требование `INT-02` (Paged.js RepeatTableHeadHandler, ~стр. 419, строка таблицы ~558, примечание ~565-567) переименовать в `PAG-01`; примечание заменить правилом пространства имён: «INT-NN — идентификаторы находок интеграционного чекера в *-MILESTONE-AUDIT.md, нумерация своя в каждом аудите (INT-02 аудита v1.3.3 → требование PAG-01; INT-01..03 аудита v1.4 — долги выпуска, закрытые Фазой 41.7); требования с префиксом INT не заводить». Синхронно переименовать в живых документах: ROADMAP.md (бэклог ~2334), PROJECT.md (~130, ~293, ~416), STATE.md (~1804) — с пометкой «(бывш. INT-02 аудита v1.3.3)» при первом упоминании. НЕ переписывать исторические артефакты: MILESTONES.md, RETROSPECTIVE.md, .planning/milestones/*, .planning/phases/*, STATE.md строки про INT-01..03 Фазы 41.7 (~76, ~757) и журнал (~1621). Проверить, что `gsd-sdk query` не ломается на новом ID (например `gsd-sdk query state.load` отрабатывает).

Хвост WR-02: создать `.planning/todos/pending/2026-11-21-wr02-multi-execute-transactional.md` (frontmatter как у соседних todo: created, title, area: quality, milestone: v1.4, files). Содержимое: (1) DRAIN_GRACE — смягчение, задокументировано у константы в server/mod.rs, принятый остаточный риск; (2) ИНВЕНТАРЬ сервисных методов crates/trackly-app/src/services/*.rs, в теле которых ≥2 последовательных `writer.execute(` (собрать скриптом/грепом по HEAD, список file::function с числом вызовов; транзитивные self-вызовы владельцев записи отметить отдельно, если исполнитель их видит) — кандидаты на транзакционный редизайн; (3) живая UAT: «Остановить сервер» в момент создания акта по LAN; (4) UF-1/UF-2 из 41.7-SECURITY.md внести в реестр угроз фазы, которая возьмёт этот todo. Обоснование отделения: редизайн нескольких сервисных методов — объём отдельной фазы (стоимость контекста), а не упрощение.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && node ui/scripts/check-ws-disconnect-toast.mjs --selftest && node ui/scripts/check-ws-disconnect-toast.mjs && node ui/scripts/check-place-tree-invalidation.mjs --selftest && node ui/scripts/check-place-tree-invalidation.mjs && pnpm --dir ui lint && pnpm --dir ui check && test "$(grep -cE '^\- \*\*INT-[0-9]+\*\*|^\| INT-[0-9]+ ' .planning/REQUIREMENTS.md)" = 0 && grep -q 'PAG-01' .planning/REQUIREMENTS.md && test -f .planning/todos/pending/2026-11-21-wr02-multi-execute-transactional.md</automated>
  </verify>
  <done>W6 и INV-10 зелёные на HEAD, красные на мутантах; W6 красный на ws.ts из $BASE (вывод прогона в SUMMARY); pnpm lint и pnpm check (svelte-check — отдельная команда) зелёные; в REQUIREMENTS нет требований с префиксом INT, есть PAG-01 и правило пространства имён; новое todo с инвентарём существует.</done>
</task>

<task type="auto" tdd="true">
  <name>Задача 3: Гейт приватности — межстрочный проход, кодировки и PNG-текст, разложение длинных значений, охват; перенос обоих todo в completed</name>
  <files>scripts/check-privacy.mjs, scripts/check-privacy.selftest.mjs, scripts/fixtures/privacy/README.md, scripts/fixtures/privacy/wrapped-marker.md, scripts/fixtures/privacy/paragraph-break.md, .planning/todos/completed/2026-10-09-review-41.7-residual-findings.md, .planning/todos/completed/2026-09-26-privacy-gate-line-wrap-and-scope-gaps.md</files>
  <behavior>
    - wrapped-marker.md: вымышленное «Пчёлкин» в конце строки, «Артём» в начале следующей той же абзацной группы (плюс второй случай за префиксами «// » на обеих строках) → exit 1, вывод не содержит ни одного из слов
    - paragraph-break.md: те же слова, разделённые пустой строкой → exit 0
    - without-marker.md и with-marker.md — прежнее поведение (0 и 1)
    - cp1251: временный файл (os.tmpdir) с «Пчёлкин Артём» в windows-1251 → exit 1; то же в UTF-16LE с BOM → exit 1
    - PNG: временный .png под путём вне allowlist — класс D как раньше; функция извлечения текста PNG (экспорт) на синтетическом PNG с tEXt и zTXt «Пчёлкин Артём» возвращает текст, по которому сканер находит маркер
    - decomposeLongValue('Пчёлкин Артём Ильич из отдела снабжения') → 4 смежные 3-граммы; каждая пара соседних слов покрыта; хэш каждой фразы == хэш сканера; файл с этим значением, разорванным переносом в любой точке, ловится по временному файлу хэшей из разложения
    - охват: isExcludedPath / isAutoScanExcludedFixture возвращают false для образцов .planning/todos/pending/x.md, .planning/quick/x/y.md, .planning/debug/x.md, .planning/threads/x.md, .planning/milestones/x.md, .planning/phases/x/y.md, templates/x.html, crates/trackly-app/tests/fixtures/devices/x.csv, ui/scripts/fixtures/x/y.txt; true только для node_modules/, target/, Cargo.lock, scripts/fixtures/privacy/
  </behavior>
  <action>
Только вымышленные данные: «Пчёлкин Артём», «Ильич», «отдел снабжения». Ни одного реального ФИО/реквизита в скрипте, фикстурах, тестах, SUMMARY и сообщениях коммитов.

Межстрочный проход (п.1 todo). В scanHashes, помимо построчного прохода, собрать слова файла с номером строки каждого; абзац = подряд идущие строки, в каждой из которых есть хотя бы одно совпадение WORD_RE (строка без слов — пустая или только пунктуация/«//»/«#» — граница абзаца). Для n = 2..NGRAM_MAX_WORDS хэшировать только окна, слова которых лежат на РАЗНЫХ строках (внутристрочные уже покрыты); нарушение — `{path, line: строка первого слова окна, cls}`, дедуп с построчными по (line, cls). Префиксы комментариев отсекает сам WORD_RE. Обновить шапку скрипта (режим 2 больше не строго построчный; граница — пустая строка) и README фикстур; добавить wrapped-marker.md и paragraph-break.md (по образцу with-marker.md, с шапкой «вымышлено, C-01») и внести их в EXPECTED_FIXTURES блока 10 selftest.

Кодировки (п.3, слепая зона cp1251). Сборщики читают Buffer (staged — execFileSync без encoding, maxBuffer как есть). Декодирование: BOM FF FE / FE FF → UTF-16LE/BE; иначе UTF-8 через `new TextDecoder('utf-8', { fatal: true })`; при ошибке — windows-1251 собственной таблицей 0x80–0xFF (128 символов литералом в скрипте; сгенерировать один раз локально и сверить с эталоном Node TextDecoder('windows-1251') в selftest, если он доступен — без зависимости гейта от ICU). Декодированный текст идёт и в режим 1, и в режим 2. Контракт fail-closed (CR-02) сохраняется.

PNG-текст (п.3, «скриншоты-метаданные»). Экспортируемая чистая функция извлечения текста из PNG: разобрать чанки tEXt (latin1), zTXt (node:zlib inflateSync), iTXt (UTF-8, с флагом сжатия); для каждого .png-таргета (включая allowlisted) прогнать извлечённый текст через режим 2, нарушение — `{path, line: 0, cls}`. Битый PNG — не падать, а сканировать как есть (класс D/allowlist решают отдельно). В шапке скрипта добавить node:zlib в перечень встроенных модулей. Прогнать на 15 закоммиченных PNG с продовыми хэшами — должно быть чисто.

BINARY_EXTENSIONS расширить: .doc .xls .xlsm .xlsb .ppt .pptx .odt .ods .odp .rtf .zip .7z .rar .gz .tgz .gif .webp .bmp .tif .tiff .heic .db .sqlite .sqlite3 .db-wal .db-shm .eml .msg. Перед этим `git ls-files` по этим расширениям — должно быть пусто (иначе остановиться и сообщить).

Длинные значения (п.2). Экспортировать `decomposeLongValue(value)` → `{ok:true, phrases:[{canonical, hash}]}` (все смежные окна NGRAM_MAX_WORDS слов; для ≤3 слов — одна фраза, как canonicalizeAddValue) или `{ok:false, reason}`. В runAdd вместо отказа too_many_words: показать число фраз, для каждой — сколько отслеживаемых файлов HEAD уже содержат её как n-грамму (переиспользовать сбор HEAD и extractNgrams/межстрочный проход; пути НЕ печатать), спросить подтверждение y/N на весь набор; записать подтверждённые хэши тем же форматом, без дублей. NGRAM_MAX_WORDS не повышать. Селфтест — через экспорт (TTY-ветку не вызывать).

Охват (п.3). Экспортировать isExcludedPath и isAutoScanExcludedFixture; selftest-блок по списку из behavior. Ручная разведка .planning/todos, .planning/quick, .planning/debug, .planning/threads, .planning/milestones на значения, ещё не занесённые в токены (эвристика «Фамилия И.О.» / «Фамилия Имя Отчество», за вычетом известных вымышленных): результаты — ТОЛЬКО в scratchpad-файле вне репозитория; в SUMMARY и финальном сообщении — только число файлов, число кандидатов и путь к scratchpad-файлу, без значений; заносит токены пользователь сам через --add.

Мутационная проверка (обязательна, урок vacuous_test / mutation_test_anchor_must_be_unique): (а) `git show $BASE:scripts/check-privacy.mjs` в scratchpad → прогон с `--hashes scripts/fixtures/privacy/tokens.fixture.sha256 scripts/fixtures/privacy/wrapped-marker.md` даёт exit 0 (фикстура несущая), новый гейт — exit 1; (б) копия нового гейта в scratchpad с выключенным межстрочным проходом (удалить один уникальный вызов) — exit 0 на wrapped-marker.md; (в) путь pre-commit: временный `git init` в scratchpad, туда копия нового гейта в scripts/, вымышленный файл с переносом внутри «Пчёлкин / Артём» в индексе, `--staged --hashes <fixture>` → exit 1; путь CI (HEAD-режим) — итоговый прогон по репозиторию. Рабочее дерево и индекс основного репозитория для мутаций не использовать (параллельные пакеты).

Если новый проход или декодер найдёт срабатывание в HEAD с ПРОДОВЫМИ хэшами — это реальная скрытая утечка: путь:строку посмотреть локально, заменить значение обезличенной формулировкой, в SUMMARY — только путь и класс; если это фикстура тестов — заменить на вымышленное значение и прогнать соответствующий тест.

Финал: оба исходных todo `git mv` в `.planning/todos/completed/`, во frontmatter добавить `resolved: 2026-11-21` и `resolution: >` — по пунктам: что закрыто (с хэшами коммитов), что уже было закрыто ранее (WR-06/WR-07-часть 0db2d02b, трассируемость 1a665bd7, decision objections «None»), что вынесено в 2026-11-21-wr02-multi-execute-transactional.md, и что NGRAM_MAX_WORDS сохранён 3 с разложением в --add. PRIV-03 (история git) не входит — указать.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && node scripts/check-privacy.selftest.mjs && node scripts/check-privacy.mjs --hashes scripts/fixtures/privacy/tokens.fixture.sha256 scripts/fixtures/privacy/paragraph-break.md && ! node scripts/check-privacy.mjs --hashes scripts/fixtures/privacy/tokens.fixture.sha256 scripts/fixtures/privacy/wrapped-marker.md && node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256 && test -f .planning/todos/completed/2026-10-09-review-41.7-residual-findings.md && test -f .planning/todos/completed/2026-09-26-privacy-gate-line-wrap-and-scope-gaps.md && test ! -e .planning/todos/pending/2026-10-09-review-41.7-residual-findings.md && test ! -e .planning/todos/pending/2026-09-26-privacy-gate-line-wrap-and-scope-gaps.md && grep -q '^resolution:' .planning/todos/completed/2026-09-26-privacy-gate-line-wrap-and-scope-gaps.md && grep -q '^resolution:' .planning/todos/completed/2026-10-09-review-41.7-residual-findings.md</automated>
  </verify>
  <done>Selftest 0 нарушений; wrapped-marker ловится (exit 1), paragraph-break и without-marker проходят; дореформенный гейт из $BASE и мутант без межстрочного прохода на wrapped-marker дают exit 0 (выводы в SUMMARY); staged-путь во временном репо — exit 1; HEAD с продовыми хэшами — PASS; оба todo в completed/ с resolved/resolution.</done>
</task>

<task type="auto" tdd="true">
  <name>Задача 4: 41.3-SECURITY — гонка скана целостности (набл. 5), тест симлинков в бэкапе (набл. 8), закрепление vcpkg (набл. 4), sandbox iframe PDF (набл. 7)</name>
  <files>crates/trackly-app/src/services/attachment_admin.rs, crates/trackly-app/tests/attachments_integrity.rs, crates/trackly-app/tests/backup_service.rs, ui/src/features/attachments/AttachmentViewer.svelte, ui/scripts/check-attachments-block.mjs, .github/workflows/release.yml, .github/workflows/heic-spike.yml, .planning/phases/41.3-attachments-storage/41.3-SECURITY.md</files>
  <behavior>
    - скан целостности: блоб-сирота старше 24 ч, на который ПОСЛЕ чтения хешей из БД появилась строка (имитация: список кандидатов устаревший), не удаляется; без строки — удаляется (orphans_removed += 1)
    - бэкап/перенос (cfg(unix)): в исходном каталоге вложений симлинк-шард (каталог из 2 hex → вне хранилища) и симлинк-файл с валидным хеш-именем — оба не скопированы; обычный блоб скопирован; функция не падает
    - check-attachments-block: правило про iframe PDF — либо атрибут sandbox присутствует, либо в исходнике стоит маркер принятого риска со ссылкой на 41.3-SECURITY.md (одно из двух, иначе красно); мутант «оба убраны» — красный
  </behavior>
  <action>
Набл. 5 (гонка). В `AttachmentAdmin::run_integrity_scan` (`services/attachment_admin.rs` ~88-146) сохранить разбор диска и чтение хешей вне писателя, но удаление сирот вынести в одно замыкание `self.writer.execute(..)`: для каждого кандидата (старше GRACE_SECS, нет в db_hashes) внутри замыкания повторно `repo.count_by_hash_on(conn, hash, None)`; удалять блоб (`store.remove_blob`) только при 0, иначе считать как живой (не orphans_removed). Загрузка делает `store.commit_staged` и вставку строки внутри того же единственного писателя (T-41.3-26), поэтому перепроверка в писателе сериализована с ней — это и закрывает окно. Сохранить: `unavailable` ничего не удаляет; .tmp — как было; ошибка удаления — warn. Если сканер вызывается до готовности писателя на старте (`context.rs` ~536-543) — проверить, что writer уже поднят. Для детерминированного теста выделить шаг удаления в отдельную функцию/метод, принимающий список кандидатов (pub(crate) с тестовым доступом либо `#[doc(hidden)] pub`), и в `tests/attachments_integrity.rs` проверить: кандидат с существующей строкой (вставлена после формирования списка) — блоб на месте; кандидат без строки — удалён. Мутация: временно убрать перепроверку — новый тест обязан покраснеть (вывод в SUMMARY), вернуть.

Набл. 8. В `tests/backup_service.rs` добавить `#[cfg(unix)]` тест рядом с `backup_copies_attachments_incrementally` (~219): в исходном хранилище создать `std::os::unix::fs::symlink` на каталог вне хранилища под именем шарда из 2 hex и симлинк-файл с валидным 64-hex именем в настоящем шарде; после бэкапа (и тем же путём, которым пользуется «Переместить БД», если он публично вызываем — `copy_attachments_incremental` в `backup_service.rs` ~312) убедиться, что ни симлинк, ни содержимое цели не появились в приёмнике, обычный блоб скопирован. Мутация: в копии функции убрать проверку `symlink_metadata` — тест краснеет (можно проверить временной правкой и откатом; вывод в SUMMARY).

Набл. 4 (vcpkg). В `release.yml` (шаг «Install libheif via vcpkg», ~121-137) и `heic-spike.yml` (~48-51) перестать брать vcpkg образа раннера: клонировать `https://github.com/microsoft/vcpkg` на закреплённый релизный тег (актуальный выбрать через `gh api repos/microsoft/vcpkg/releases/latest`, в комментарии записать тег И SHA коммита, на который он указывает) в `$env:RUNNER_TEMP\vcpkg`, `bootstrap-vcpkg.bat -disableMetrics`, ставить libheif этим vcpkg и экспортировать VCPKG_ROOT на него. Сохранить цикл повторов (4 попытки, 60 с) и триплет x64-windows-static-md; клон тоже с повтором. Проверка статически: YAML валиден (например `python3 -c 'import yaml,sys; yaml.safe_load(open(sys.argv[1]))'` по обоим файлам или actionlint, если установлен). Живой прогон Windows-раннера НЕ запускать (порождает черновик релиза; урок release_version_churn) — в SUMMARY пометить «UNVERIFIED на раннере, проверится ближайшим релизным прогоном».

Набл. 7 (iframe). Сначала измерить, а потом менять: в scratchpad поднять локальную страницу (например `python3 -m http.server` в scratchpad-каталоге) с вымышленным одностраничным PDF, отдаваемым с `Content-Type: application/pdf` и `X-Content-Type-Options: nosniff`, и двумя iframe — без sandbox и с минимальным sandbox (пустой атрибут; затем, если пустой блокирует, — минимальный набор токенов без `allow-same-origin` вместе с `allow-scripts`, эта пара снимает изоляцию и бессмысленна). Открыть в установленных Microsoft Edge (тот же движок, что WebView2) и Google Chrome, и в Playwright WebKit как приближение WKWebView; снять скриншоты в scratchpad. Решение: если PDF рендерится в sandbox-варианте во всех трёх — поставить этот sandbox в `AttachmentViewer.svelte` (~261); если хотя бы один движок показывает заглушку/блокировку — sandbox НЕ ставить (функцию не ломать), в `41.3-SECURITY.md` наблюдение 7 перевести в принятый риск с обоснованием (тип строго application/pdf + nosniff, Chromium не рендерит PDF в sandbox-iframe, результат замера с датой) и поставить в разметке рядом с iframe комментарий-маркер со ссылкой на этот риск. В обоих исходах в `check-attachments-block.mjs` добавить правило из behavior + мутант в его selftest. Синтетический замер — не верификация (урок synthetic_harness_not_verification): итог в реальных приложениях — human-check ниже, исполнитель его НЕ отмечает пройденным сам (auto_advance авто-одобряет — пункт должен уйти пользователю).

Набл. 4, 5, 7, 8 в `41.3-SECURITY.md`: дописать к каждому строку «Закрыто quick 261121-hjw, коммит …» (или «принятый риск» для 7 по исходу замера). Набл. 3 — пометить «закрывается quick 261121-heg (WR-13)», не трогать CI.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_attachments -- attachments_integrity backup_service && cargo clippy -p trackly-app --all-targets -- -D warnings && cargo fmt --all -- --check && node ui/scripts/check-attachments-block.mjs --selftest && node ui/scripts/check-attachments-block.mjs && pnpm --dir ui lint && pnpm --dir ui check && python3 -c 'import yaml,sys; [yaml.safe_load(open(f)) for f in sys.argv[1:]]' .github/workflows/release.yml .github/workflows/heic-spike.yml && grep -q 'bootstrap-vcpkg' .github/workflows/release.yml</automated>
    <human-check>Предпросмотр PDF-вложения (вымышленный PDF) открывается и листается: (1) десктоп macOS (`cargo tauri dev`, WKWebView); (2) десктоп Windows (WebView2, сборка с этим коммитом); (3) LAN-браузер Chrome/Edge (после `pnpm --dir ui build`). Если sandbox поставлен и хоть в одном окружении PDF не показан — откатить атрибут и оформить принятый риск, как описано в action.</human-check>
  </verify>
  <done>Перепроверка ссылок в писателе есть, тест на устаревший список кандидатов зелёный и краснеет без перепроверки; unix-тест симлинков зелёный и краснеет без symlink_metadata; vcpkg закреплён тегом+SHA в обоих workflow, YAML валиден, отметка UNVERIFIED на раннере; iframe — sandbox или задокументированный принятый риск по результату замера, гейт check-attachments-block держит одно из двух; human-check передан пользователю, а не отмечен исполнителем.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| LAN-клиент → POST /api/v1/auth_bootstrap | анонимный запрос; адрес пира — из TCP-соединения (ConnectInfo), не из заголовков |
| рабочее дерево → публичный git | любой коммиченный текст публичен навсегда |
| браузерный клиент → WS | лишний живой сокет удваивает события (GAP-12-10) |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-hjw-01 | Repudiation | аудит custom:bootstrap | mitigate | peer = ConnectInfo.ip() (не X-Forwarded-For — подделывается клиентом); тест утверждает наличие IP в payload |
| T-hjw-02 | Information disclosure | payload аудита bootstrap | accept | IP внутренней LAN не секрет для администратора, журнал аудита виден только Admin |
| T-hjw-03 | Information disclosure | check-privacy.mjs, фикстуры, SUMMARY, коммиты | mitigate | только вымышленное «Пчёлкин Артём»; гейт печатает путь:строку и класс, не значение (D-16, утверждается selftest-ом для новых фикстур); результаты разведки — только в scratchpad; pre-commit-хук перед каждым коммитом |
| T-hjw-04 | Tampering | ослабление гейтов (entities, ws-toast, place-tree, privacy) | mitigate | каждая правка гейта доказана мутацией реального исходника или дореформенной версии из $BASE; свипы с счётчиком против вакуумности |
| T-hjw-05 | Denial of service | межстрочный проход / PNG-инфляция | mitigate | окна ограничены NGRAM_MAX_WORDS; inflateSync с maxOutputLength (например 1 МБ), сбой разбора PNG — не падение |
| T-hjw-06 | Denial of service | второй живой WS-сокет | mitigate | reconnectTimer отменяется в activeCleanup, колбэк проверяет refCount; W6 + мутанты |
| T-hjw-07 | Tampering | скан целостности ↔ загрузка | mitigate | удаление сироты только после count_by_hash_on == 0 внутри writer.execute, сериализованного с commit_staged загрузки |
| T-hjw-08 | Tampering | симлинк в хранилище при бэкапе/переносе | mitigate | существующий пропуск через symlink_metadata закреплён cfg(unix)-тестом с мутацией |
| T-hjw-09 | Tampering | цепочка поставки vcpkg/libheif | mitigate | vcpkg с закреплённого тега + SHA в комментарии; обновление — осознанным диффом |
| T-hjw-10 | Elevation of privilege | iframe предпросмотра PDF | mitigate / accept | sandbox, если не ломает встроенный просмотрщик; иначе принятый риск (application/pdf + nosniff) с записью в 41.3-SECURITY.md и гейтом-маркером |
</threat_model>

<verification>
- Rust: `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_gates -- entities_changed_gate http_route_parity entities_changed_cartridges`; `... --test app_auth -- auth_bootstrap desktop_system_admin`; `... --test app_attachments -- attachments_integrity backup_service`; по одному cargo за раз; сплошной прогон не нужен.
- `cargo clippy -p trackly-app --all-targets -- -D warnings`; `cargo fmt --all -- --check`.
- UI: `pnpm --dir ui lint` и `pnpm --dir ui check` — две разные команды, обе.
- Приватность: `node scripts/check-privacy.selftest.mjs`; `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` (HEAD — путь CI); staged-путь — во временном репозитории (задача 3).
- Перед каждым коммитом: `git diff --cached | grep -n` по вымышленным маркерам — допустимы только в scripts/fixtures/privacy/ и selftest; pre-commit-хук проходит.
</verification>

<success_criteria>
- WR-04, WR-05, WR-07 (остаток), WR-08, WR-09, IN-01..03 закрыты кодом и закреплены гейтами/тестами с доказанной мутацией.
- Коллизия INT-NN снята в живых документах, правило пространства имён записано.
- Хвост WR-02 оформлен отдельным pending-todo с инвентарём методов.
- Гейт приватности ловит перенос строки, cp1251/UTF-16, PNG-текст, расширенные бинарные форматы; --add раскладывает длинные значения; охват каталогов закреплён selftest-ом; HEAD чист.
- Наблюдения 4, 5, 7, 8 из 41.3-SECURITY.md закрыты кодом/тестами или (7) оформлены принятым риском по результату замера; human-check предпросмотра PDF передан пользователю.
- Оба todo в completed/ с resolution. Коммиты атомарные (по одному на задачу или мельче), каждый заканчивается строкой `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
</success_criteria>

<output>
Create `.planning/quick/261121-hjw-residual-41-7-privacy/261121-hjw-SUMMARY.md` when done (с BASE, числом строк ex→exw, выводами мутационных прогонов, числом кандидатов разведки без значений).
</output>
