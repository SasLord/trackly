//! Реестровый гейт «серверная мутация -> рассылка WS-события EntitiesChanged»
//! (Phase 41.7, критерий 2 ROADMAP; D-06, D-07, D-08, D-16).
//!
//! Гейт двухслойный:
//!
//! * Слой (1) — ЭТОТ файл: реестр вердиктов (`entities_support::registry`) на
//!   КАЖДУЮ функцию области (восемь файлов `src/services/`), сканер исходников,
//!   selftest на синтетике, мутационные проверки реальных исходников, гард
//!   области. Освобождение от рассылки — только строка реестра `Exempt` с
//!   причиной не короче 40 СИМВОЛОВ (видна в диффе ревью, D-08).
//! * Слой (2) — файлы `tests/entities_changed_*.rs`, по одному на рассылающий
//!   сервис (places, groups, acts, devices, cartridges): поведенческие сценарии
//!   на настоящем `AppCtx::build`; привязка к реестру — `BEHAVIOUR_TEST`.
//!
//! Сборка гейта в два плана: план 41.7-05 — реестр, фикстуры и тесты формы
//! таблицы; план 41.7-14 — сканер (`entities_support::scan`, чистый модуль без
//! fs), привязка слоя (2) (`entities_support::scenarios`), selftest, боевой
//! скан, мутационные проверки и гард области.
//!
//! Известное ограничение слоя (1): правило `ReadOnlyCallsWriter` видит только
//! вызовы `self.<имя>(` / `Self::<имя>(` владельцев записи того же файла;
//! вызовы через другой объект, макрос или из другого файла (`group_place::*`,
//! репозитории) закрывает слой (2). Подробнее — шапка `scan.rs`.
//!
//! Тесты:
//!   * `registry_shape_is_valid`, `registry_inventory_is_not_empty` — форма
//!     таблицы без чтения исходников;
//!   * `fixture_capture_*` — захват событий на настоящем `AppCtx`;
//!   * `selftest_*` — сканер и привязка слоя (2) на синтетике в памяти: гейт
//!     доказывает, что не мёртв (D-08);
//!   * `every_service_mutation_has_verdict_and_broadcasts` — боевой скан
//!     исходников сервисов; КРАСНЫЙ до планов 06-10 (нерассылающий код);
//!   * `registry_matches_sources_exactly` — инвентарь исходников и реестр
//!     совпадают в обе стороны, пустой разбор краснит гейт;
//!   * `behaviour_files_have_real_scenario_for_every_broadcasting_row` —
//!     слой (2) привязан к реальным `#[tokio::test] async fn scenario_*`;
//!   * `mutation_*` — мутации реальных исходников в памяти (файлы на диске не
//!     трогаются), якорь уникален (`matches(anchor).count() == 1`), красный
//!     результат сверяется с зелёным исходным (разностная проверка);
//!   * `writer_owning_services_outside_scope_are_listed` — гард области.

mod entities_support;

use std::collections::BTreeSet;
use std::time::Duration;

use entities_support::fixture::{
    assert_no_entities, assert_one_entities, make_test_ctx, next_entities,
};
use entities_support::registry::*;
use entities_support::scan::{
    first_test_item_range, inventory, owns_writer_outside_tests, parse_fns, prepare, scan, Kind,
};
use entities_support::scenarios::{assert_behaviour_file_covers, check_behaviour_file};
use trackly_app::dto::printer::WsEvent;

/// Нижняя граница размера инвентаря: гейт, который «зелёный, потому что
/// измерил ноль», — худший режим отказа. Снято по факту: 118 строк.
const MIN_REGISTRY_ROWS: usize = 110;

fn reason_of(e: &Entry) -> Option<&'static str> {
    match e.verdict {
        Verdict::Exempt(r) => Some(r),
        _ => None,
    }
}

#[test]
fn registry_inventory_is_not_empty() {
    assert!(
        !REGISTRY.is_empty(),
        "REGISTRY пуст: гейт без инвентаря зелёный, ничего не измерив"
    );
    assert!(
        REGISTRY.len() >= MIN_REGISTRY_ROWS,
        "в REGISTRY {} строк, ожидали не меньше {MIN_REGISTRY_ROWS}: инвентарь усох",
        REGISTRY.len()
    );
    assert!(
        REGISTRY
            .iter()
            .filter(|e| e.verdict == Verdict::Broadcasts)
            .count()
            > 0,
        "в REGISTRY нет ни одной строки Broadcasts"
    );
}

#[test]
fn registry_shape_is_valid() {
    let mut problems: Vec<String> = Vec::new();

    // Пустые имена и дубли ключа (file, func).
    let mut seen: BTreeSet<(&str, &str)> = BTreeSet::new();
    for e in REGISTRY {
        if e.file.trim().is_empty() || e.func.trim().is_empty() {
            problems.push(format!("пустое имя в строке ({:?}, {:?})", e.file, e.func));
        }
        if !seen.insert((e.file, e.func)) {
            problems.push(format!("дубль ключа {}::{}", e.file, e.func));
        }
    }

    // Причины освобождений: длина в СИМВОЛАХ, не заглушки.
    for e in REGISTRY {
        if let Some(reason) = reason_of(e) {
            let n = reason.chars().count();
            if n < MIN_EXEMPT_REASON_CHARS {
                problems.push(format!(
                    "{}::{}: причина Exempt короче {MIN_EXEMPT_REASON_CHARS} символов ({n})",
                    e.file, e.func
                ));
            }
            let t = reason.trim().to_lowercase();
            if t == "todo" || t == "n/a" || t == "tbd" {
                problems.push(format!("{}::{}: причина-заглушка", e.file, e.func));
            }
        }
    }

    // Спорный update_type помечен явно (Q8).
    match REGISTRY
        .iter()
        .find(|e| e.file == "group_type_service.rs" && e.func == "update_type")
    {
        None => problems.push("нет строки group_type_service.rs::update_type".into()),
        Some(e) => match reason_of(e) {
            Some(r) if r.starts_with("СПОРНО") => {}
            _ => problems.push(
                "group_type_service.rs::update_type должна быть Exempt с причиной «СПОРНО…»".into(),
            ),
        },
    }

    // Область: ровно восемь файлов, дубль-free, у каждого есть строка,
    // каждый файл реестра — внутри области.
    if SCOPE_FILES.len() != 8 {
        problems.push(format!(
            "SCOPE_FILES: {} файлов, ожидали 8",
            SCOPE_FILES.len()
        ));
    }
    let scope: BTreeSet<&str> = SCOPE_FILES.iter().copied().collect();
    if scope.len() != SCOPE_FILES.len() {
        problems.push("SCOPE_FILES содержит дубли".into());
    }
    for f in SCOPE_FILES {
        if !REGISTRY.iter().any(|e| e.file == *f) {
            problems.push(format!(
                "в области {f}, но в реестре нет ни одной его строки"
            ));
        }
    }
    for e in REGISTRY {
        if !scope.contains(e.file) {
            problems.push(format!("{}::{}: файл вне SCOPE_FILES", e.file, e.func));
        }
    }

    // BEHAVIOUR_TEST <-> файлы со строкой Broadcasts, в обе стороны; ровно пять.
    let broadcasting: BTreeSet<&str> = REGISTRY
        .iter()
        .filter(|e| e.verdict == Verdict::Broadcasts)
        .map(|e| e.file)
        .collect();
    let mapped: BTreeSet<&str> = BEHAVIOUR_TEST.iter().map(|(f, _)| *f).collect();
    if mapped.len() != BEHAVIOUR_TEST.len() {
        problems.push("BEHAVIOUR_TEST: дубль файла сервиса".into());
    }
    for f in broadcasting.difference(&mapped) {
        problems.push(format!("{f} рассылает, но нет записи в BEHAVIOUR_TEST"));
    }
    for f in mapped.difference(&broadcasting) {
        problems.push(format!(
            "{f} в BEHAVIOUR_TEST, но не имеет строки Broadcasts"
        ));
    }
    if broadcasting.len() != 5 {
        problems.push(format!(
            "рассылающих файлов {}, ожидали ровно 5",
            broadcasting.len()
        ));
    }
    let tests: BTreeSet<&str> = BEHAVIOUR_TEST.iter().map(|(_, t)| *t).collect();
    if tests.len() != BEHAVIOUR_TEST.len() {
        problems.push("BEHAVIOUR_TEST: два сервиса указывают на один поведенческий файл".into());
    }
    for (_, t) in BEHAVIOUR_TEST {
        if !(t.starts_with("entities_changed_") && t.ends_with(".rs")) {
            problems.push(format!(
                "поведенческий файл {t} вне шаблона entities_changed_*.rs"
            ));
        }
    }

    // Allowlist гарда области: причина в СИМВОЛАХ, файл вне области.
    for (f, reason) in OUT_OF_SCOPE_WRITERS {
        if reason.chars().count() < MIN_EXEMPT_REASON_CHARS {
            problems.push(format!(
                "OUT_OF_SCOPE_WRITERS {f}: причина короче {MIN_EXEMPT_REASON_CHARS} символов"
            ));
        }
        if scope.contains(f) {
            problems.push(format!(
                "OUT_OF_SCOPE_WRITERS {f}: файл уже в области гейта"
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "реестр невалиден:\n  - {}",
        problems.join("\n  - ")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fixture_capture_skips_other_variants_and_sees_entities_changed() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (ctx, _dir) = make_test_ctx().await;
        let entities = || WsEvent::EntitiesChanged {
            place_ids: vec![7],
            device_ids: vec![],
            group_ids: vec![],
        };
        let other = || WsEvent::NumberSpaceChanged {
            contexts: vec!["act".to_string()],
        };

        // Чужой вариант ПЕРЕД искомым: next_entities обязан его пропустить.
        let mut rx = ctx.ws_broadcast.subscribe();
        ctx.ws_broadcast.send(other()).expect("send other");
        ctx.ws_broadcast.send(entities()).expect("send entities");
        assert_eq!(
            next_entities(&mut rx),
            Some((vec![7], vec![], vec![])),
            "чужой вариант не должен заслонять EntitiesChanged"
        );

        // Ровно одно EntitiesChanged среди чужих: проходит и возвращает тройку.
        let mut rx = ctx.ws_broadcast.subscribe();
        ctx.ws_broadcast.send(other()).expect("send other");
        ctx.ws_broadcast.send(entities()).expect("send entities");
        assert_eq!(
            assert_one_entities(&mut rx, "одно событие"),
            (vec![7], vec![], vec![])
        );

        // Только чужой вариант: EntitiesChanged нет, next_entities даёт None.
        let mut rx = ctx.ws_broadcast.subscribe();
        ctx.ws_broadcast.send(other()).expect("send other");
        assert_no_entities(&mut rx, "только чужой вариант");

        // Антивакуумность: assert_no_entities ДОЛЖЕН краснеть, когда событие есть.
        let mut rx = ctx.ws_broadcast.subscribe();
        ctx.ws_broadcast.send(entities()).expect("send entities");
        let red = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_no_entities(&mut rx, "событие есть")
        }));
        assert!(
            red.is_err(),
            "assert_no_entities не заметил EntitiesChanged"
        );

        // И assert_one_entities краснеет на двух событиях и на нуле.
        let mut rx = ctx.ws_broadcast.subscribe();
        ctx.ws_broadcast.send(entities()).expect("send entities");
        ctx.ws_broadcast.send(entities()).expect("send entities");
        let two = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_one_entities(&mut rx, "два события")
        }));
        assert!(two.is_err(), "assert_one_entities пропустил два события");
        let mut rx = ctx.ws_broadcast.subscribe();
        let zero = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_one_entities(&mut rx, "ноль событий")
        }));
        assert!(
            zero.is_err(),
            "assert_one_entities пропустил отсутствие события"
        );
    })
    .await
    .expect("таймаут 60 с");
}

// ===========================================================================
// Слой (1), часть 2: selftest сканера на синтетике в памяти (D-08)
// ===========================================================================

const SYN: &str = "svc.rs";

fn row(func: &'static str, verdict: Verdict) -> Entry {
    Entry {
        file: SYN,
        func,
        verdict,
    }
}

fn kinds(vs: &[entities_support::scan::Violation]) -> Vec<(String, Kind)> {
    vs.iter().map(|v| (v.func.clone(), v.kind)).collect()
}

fn has(vs: &[entities_support::scan::Violation], func: &str, kind: Kind) -> bool {
    vs.iter().any(|v| v.func == func && v.kind == kind)
}

const EXEMPT_REASON: &str =
    "вымышленная причина освобождения для проверки сканера, длиннее сорока символов";

const SYN_CLEAN: &str = r#"
impl S {
    pub async fn create(&self, x: i64) -> R {
        let row = self
            .writer
            .execute(move |c| { Ok(x) })
            .await?;
        self.broadcast_entities("create", vec![row], vec![], vec![]);
        Ok(row)
    }

    fn broadcast_entities(&self, op: &'static str, a: Vec<i64>, b: Vec<i64>, c: Vec<i64>) {}
}
"#;

#[test]
fn selftest_clean_source_is_green() {
    let reg = [row("create", Verdict::Broadcasts)];
    let vs = scan(SYN, SYN_CLEAN, &reg);
    assert!(vs.is_empty(), "чистый источник покраснел: {:?}", kinds(&vs));
}

#[test]
fn selftest_mutation_without_broadcast_is_red() {
    let src = SYN_CLEAN.replace(
        "self.broadcast_entities(\"create\", vec![row], vec![], vec![]);",
        "",
    );
    assert_ne!(src, SYN_CLEAN, "мутация не применилась");
    let reg = [row("create", Verdict::Broadcasts)];
    let vs = scan(SYN, &src, &reg);
    assert!(
        has(&vs, "create", Kind::MissingBroadcast),
        "{:?}",
        kinds(&vs)
    );
}

#[test]
fn selftest_mutation_outside_registry_is_red() {
    let vs = scan(SYN, SYN_CLEAN, &[]);
    assert!(has(&vs, "create", Kind::NoVerdict), "{:?}", kinds(&vs));
}

#[test]
fn selftest_exempt_row_is_green() {
    let src = r#"
impl S {
    pub async fn seed(&self) {
        self.writer.execute(|c| Ok(())).await.unwrap();
    }
}
"#;
    let reg = [row("seed", Verdict::Exempt(EXEMPT_REASON))];
    let vs = scan(SYN, src, &reg);
    assert!(vs.is_empty(), "{:?}", kinds(&vs));
}

#[test]
fn selftest_broadcast_inside_writer_closure_is_red() {
    let src = r#"
impl S {
    pub async fn create(&self) {
        self.writer.execute(move |c| {
            self.broadcast_entities("create", vec![1], vec![], vec![]);
            Ok(())
        }).await.unwrap();
    }
}
"#;
    let reg = [row("create", Verdict::Broadcasts)];
    let vs = scan(SYN, src, &reg);
    assert!(
        has(&vs, "create", Kind::BroadcastInsideWriter),
        "{:?}",
        kinds(&vs)
    );
}

#[test]
fn selftest_readonly_row_with_writer_is_red() {
    let reg = [row("create", Verdict::ReadOnly)];
    let vs = scan(SYN, SYN_CLEAN, &reg);
    assert!(
        has(&vs, "create", Kind::ReadOnlyOwnsWriter),
        "{:?}",
        kinds(&vs)
    );
}

#[test]
fn selftest_stale_row_is_red() {
    let reg = [
        row("create", Verdict::Broadcasts),
        row("ghost", Verdict::ReadOnly),
    ];
    let vs = scan(SYN, SYN_CLEAN, &reg);
    assert!(has(&vs, "ghost", Kind::StaleRow), "{:?}", kinds(&vs));
}

#[test]
fn selftest_lifetimes_and_braces_in_strings_do_not_break_balance() {
    let src = r##"
impl S {
    pub fn label(&self) -> &'static str {
        // комментарий с открывающей фигурной скобкой {
        let _c = '{';
        let _r = r#"} } {"#;
        let _f = format!("{id} {{ }}", id = 1);
        "}"
    }

    pub async fn create(&self) {
        let _t: &'static str = "x";
        self.writer.execute(|c| Ok(())).await.unwrap();
        self.broadcast_entities("create", vec![1], vec![], vec![]);
    }
}
"##;
    let reg = [
        row("label", Verdict::ReadOnly),
        row("create", Verdict::Broadcasts),
    ];
    let vs = scan(SYN, src, &reg);
    assert!(vs.is_empty(), "баланс скобок сломан: {:?}", kinds(&vs));
}

#[test]
fn selftest_comment_mentioning_broadcast_does_not_count() {
    let src = r#"
impl S {
    /// Рассылает: self.broadcast_entities("create", vec![1], vec![], vec![]);
    pub async fn create(&self) {
        // self.broadcast_entities("create", vec![1], vec![], vec![]);
        /* self.broadcast_entities("create", vec![1], vec![], vec![]); */
        self.writer.execute(|c| Ok(())).await.unwrap();
    }
}
"#;
    let reg = [row("create", Verdict::Broadcasts)];
    let vs = scan(SYN, src, &reg);
    assert!(
        has(&vs, "create", Kind::MissingBroadcast),
        "{:?}",
        kinds(&vs)
    );
}

#[test]
fn selftest_broadcast_label_must_equal_function_name() {
    let src = SYN_CLEAN.replace(
        "broadcast_entities(\"create\"",
        "broadcast_entities(\"other\"",
    );
    let reg = [row("create", Verdict::Broadcasts)];
    let vs = scan(SYN, &src, &reg);
    assert!(
        has(&vs, "create", Kind::MissingBroadcast),
        "{:?}",
        kinds(&vs)
    );
}

#[test]
fn selftest_doc_comment_writer_execute_is_not_a_write() {
    let src = r#"
impl S {
    /// Читает данные, не вызывает writer.execute(..) и не трогает
    /// self.writer
    ///     .execute(..) ни в каком виде.
    pub fn get(&self) -> i64 { 1 }
}
"#;
    let reg = [row("get", Verdict::ReadOnly)];
    let vs = scan(SYN, src, &reg);
    assert!(vs.is_empty(), "{:?}", kinds(&vs));
    assert!(!owns_writer_outside_tests(src));
}

#[test]
fn selftest_dead_code_allow_left_is_red() {
    let src = format!(
        "{SYN_CLEAN}\nimpl T {{\n    // Снимается планом.\n    #[allow(dead_code)]\n    fn broadcast_entities(&self) {{}}\n}}\n"
    );
    let reg = [row("create", Verdict::Broadcasts)];
    let vs = scan(SYN, &src, &reg);
    assert!(
        has(&vs, "broadcast_entities", Kind::DeadCodeAllowLeft),
        "{:?}",
        kinds(&vs)
    );
    assert!(!kinds(&scan(SYN, SYN_CLEAN, &reg))
        .iter()
        .any(|(_, k)| *k == Kind::DeadCodeAllowLeft));
}

const SYN_MID_TEST: &str = r#"
impl S {
    pub async fn first(&self) {
        self.writer.execute(|c| Ok(())).await.unwrap();
        self.broadcast_entities("first", vec![1], vec![], vec![]);
    }
}

#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn it() { let _ = "}"; }
}

impl S {
    pub async fn late(&self) {
        self.writer.execute(|c| Ok(())).await.unwrap();
    }
}
"#;

#[test]
fn selftest_function_after_mid_file_test_module_is_still_scanned() {
    // BLOCKER 2: нетестовый код ПОСЛЕ тестового модуля посреди файла виден.
    let reg = [row("first", Verdict::Broadcasts)];
    let vs = scan(SYN, SYN_MID_TEST, &reg);
    assert!(has(&vs, "late", Kind::NoVerdict), "{:?}", kinds(&vs));

    let reg = [
        row("first", Verdict::Broadcasts),
        row("late", Verdict::Broadcasts),
    ];
    let vs = scan(SYN, SYN_MID_TEST, &reg);
    assert!(has(&vs, "late", Kind::MissingBroadcast), "{:?}", kinds(&vs));

    let fixed = SYN_MID_TEST.replace(
        "        self.writer.execute(|c| Ok(())).await.unwrap();\n    }\n}\n",
        "        self.writer.execute(|c| Ok(())).await.unwrap();\n        self.broadcast_entities(\"late\", vec![1], vec![], vec![]);\n    }\n}\n",
    );
    assert_ne!(fixed, SYN_MID_TEST);
    let vs = scan(SYN, &fixed, &reg);
    assert!(vs.is_empty(), "{:?}", kinds(&vs));
}

#[test]
fn selftest_functions_inside_test_module_are_ignored() {
    let src = r#"
impl S {
    pub fn get(&self) -> i64 { 1 }
}

#[cfg(test)]
mod tests {
    pub async fn helper(&self) { self.writer.execute(|c| Ok(())).await.unwrap(); }
}

#[cfg(not(test))]
pub async fn only_prod(&self) { self.writer.execute(|c| Ok(())).await.unwrap(); }
"#;
    let reg = [row("get", Verdict::ReadOnly)];
    let vs = scan(SYN, src, &reg);
    assert!(!has(&vs, "helper", Kind::NoVerdict), "{:?}", kinds(&vs));
    assert!(
        has(&vs, "only_prod", Kind::NoVerdict),
        "cfg(not(test)) не должен вырезаться: {:?}",
        kinds(&vs)
    );
}

#[test]
fn selftest_readonly_calling_writer_owner_is_red() {
    let src = r#"
impl S {
    pub fn get(&self) -> i64 { self.set_x(1); 1 }
    pub fn get2(&self) -> i64 { self.repo.set_x(1); repo.set_x(2); 1 }
    fn set_x(&self, v: i64) {
        self.writer.execute(move |c| Ok(v)).unwrap();
    }
}
"#;
    let reg = [
        row("get", Verdict::ReadOnly),
        row("get2", Verdict::ReadOnly),
        row("set_x", Verdict::Exempt(EXEMPT_REASON)),
    ];
    let vs = scan(SYN, src, &reg);
    assert!(
        has(&vs, "get", Kind::ReadOnlyCallsWriter),
        "{:?}",
        kinds(&vs)
    );
    assert!(
        !has(&vs, "get2", Kind::ReadOnlyCallsWriter),
        "вызов у чужого объекта не считается: {:?}",
        kinds(&vs)
    );
}

#[test]
fn selftest_multibyte_comment_keeps_offsets() {
    let src = r#"
// Кириллический комментарий перед функциями: переносы и { скобки }
const ТЕКСТ: &str = "строка с кириллицей и } скобкой";
impl S {
    /// Документация на русском языке.
    pub async fn create(&self) {
        let _s = "ещё одна строка — с тире";
        self.writer.execute(|c| Ok(())).await.unwrap();
        self.broadcast_entities("create", vec![1], vec![], vec![]);
    }
    pub fn get(&self) -> i64 { 1 } // хвостовой комментарий
}
"#;
    let reg = [
        row("create", Verdict::Broadcasts),
        row("get", Verdict::ReadOnly),
    ];
    let vs = scan(SYN, src, &reg);
    assert!(vs.is_empty(), "{:?}", kinds(&vs));
    let p = prepare(src);
    assert_eq!(p.plain.len(), src.len());
    assert_eq!(p.skeleton.len(), src.len());
}

#[test]
fn selftest_duplicate_fn_name_is_red() {
    let src = r#"
impl A { pub fn go(&self) {} }
impl B { pub fn go(&self) {} }
"#;
    let reg = [row("go", Verdict::ReadOnly)];
    let vs = scan(SYN, src, &reg);
    assert!(has(&vs, "go", Kind::DuplicateFn), "{:?}", kinds(&vs));
}

// ---- selftest слоя (2): scenarios.rs ----------------------------------------

fn bc(funcs: &[&'static str]) -> Vec<Entry> {
    funcs.iter().map(|f| row(f, Verdict::Broadcasts)).collect()
}

#[test]
fn selftest_scenario_missing_is_red() {
    let src = "#[tokio::test]\nasync fn rollback_create() { assert!(true); }\n";
    let p = check_behaviour_file(SYN, src, &bc(&["create"]));
    assert!(p.iter().any(|m| m.contains("нет настоящего")), "{p:?}");
}

#[test]
fn selftest_scenario_in_comment_does_not_count() {
    let src = "// #[tokio::test]\n// async fn scenario_create() { assert!(true); }\n/* #[tokio::test] async fn scenario_create() { assert!(true); } */\n";
    let p = check_behaviour_file(SYN, src, &bc(&["create"]));
    assert!(p.iter().any(|m| m.contains("нет настоящего")), "{p:?}");
}

#[test]
fn selftest_scenario_ignored_does_not_count() {
    let after = "#[tokio::test]\n#[ignore]\nasync fn scenario_create() { assert!(true); }\n";
    let before = "#[ignore = \"позже\"]\n#[tokio::test(flavor = \"multi_thread\")]\nasync fn scenario_create() { assert!(true); }\n";
    for src in [after, before] {
        let p = check_behaviour_file(SYN, src, &bc(&["create"]));
        assert!(
            p.iter().any(|m| m.contains("нет настоящего")),
            "{src}: {p:?}"
        );
    }
}

#[test]
fn selftest_scenario_without_tokio_test_attr_does_not_count() {
    for src in [
        "async fn scenario_create() { assert!(true); }\n",
        "#[test]\nasync fn scenario_create() { assert!(true); }\n",
        "#[tokio::test]\nfn other() {}\nasync fn scenario_create() { assert!(true); }\n",
    ] {
        let p = check_behaviour_file(SYN, src, &bc(&["create"]));
        assert!(
            p.iter().any(|m| m.contains("нет настоящего")),
            "{src}: {p:?}"
        );
    }
}

#[test]
fn selftest_scenario_hollow_body_is_red() {
    let src = "#[tokio::test]\nasync fn scenario_create() { let _x = 1; // assert!(true)\n }\n";
    let p = check_behaviour_file(SYN, src, &bc(&["create"]));
    assert!(p.iter().any(|m| m.contains("пуст")), "{p:?}");
}

#[test]
fn selftest_scenario_stale_without_registry_row_is_red() {
    let src = "#[tokio::test]\nasync fn scenario_create() { assert!(true); }\n#[tokio::test]\nasync fn scenario_removed() { assert!(true); }\n";
    let p = check_behaviour_file(SYN, src, &bc(&["create"]));
    assert!(p.iter().any(|m| m.contains("устаревший сценарий")), "{p:?}");
}

#[test]
fn selftest_scenarios_complete_is_green() {
    let src = r#"
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_create() {
    assert_eq!(1, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_rename() {
    let r = 2;
    assert_one_entities(&mut rx, "rename");
    let _ = r;
}

#[tokio::test]
async fn rollback_create() { assert!(true); }
"#;
    let p = check_behaviour_file(SYN, src, &bc(&["create", "rename"]));
    assert!(p.is_empty(), "{p:?}");
}

// ===========================================================================
// Слой (1), часть 3: боевой скан реальных исходников
// ===========================================================================

/// Нижняя граница числа функций (pub и владеющих записью) в области гейта:
/// снято по факту (118) с небольшим запасом.
const MIN_FNS_IN_SCOPE: usize = 110;

fn services_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("services")
}

fn read_service(file: &str) -> String {
    let path = services_dir().join(file);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("не удалось прочитать {}: {e}", path.display()))
}

#[test]
fn every_service_mutation_has_verdict_and_broadcasts() {
    let mut all: Vec<String> = Vec::new();
    for f in SCOPE_FILES {
        let src = read_service(f);
        for v in scan(f, &src, REGISTRY) {
            all.push(v.to_string());
        }
    }
    assert!(
        all.is_empty(),
        "гейт слоя (1) красный, нарушений {}:\n  - {}",
        all.len(),
        all.join("\n  - ")
    );
}

#[test]
fn registry_matches_sources_exactly() {
    let mut problems: Vec<String> = Vec::new();
    let mut total = 0usize;
    let mut place_count = 0usize;
    for f in SCOPE_FILES {
        let src = read_service(f);
        let found = inventory(&src);
        total += found.len();
        if *f == "place_service.rs" {
            place_count = found.len();
        }
        let mut names: BTreeSet<String> = BTreeSet::new();
        for fi in &found {
            if !names.insert(fi.name.clone()) {
                problems.push(format!(
                    "{f}::{}: имя функции повторено в исходнике",
                    fi.name
                ));
            }
        }
        let rows: BTreeSet<String> = REGISTRY
            .iter()
            .filter(|e| e.file == *f)
            .map(|e| e.func.to_string())
            .collect();
        for n in names.difference(&rows) {
            problems.push(format!("{f}::{n}: есть в исходнике, нет в реестре"));
        }
        for n in rows.difference(&names) {
            problems.push(format!("{f}::{n}: есть в реестре, нет в исходнике"));
        }
    }
    assert!(
        total >= MIN_FNS_IN_SCOPE,
        "сканер нашёл {total} функций в области, ожидали не меньше {MIN_FNS_IN_SCOPE}: пустой или усохший разбор"
    );
    assert!(
        place_count >= 15,
        "в place_service.rs найдено {place_count} функций, ожидали не меньше 15"
    );
    assert_eq!(
        total,
        REGISTRY.len(),
        "число найденных функций и строк реестра расходится"
    );
    assert!(
        problems.is_empty(),
        "реестр расходится с исходниками:\n  - {}",
        problems.join("\n  - ")
    );
}

#[test]
fn behaviour_files_have_real_scenario_for_every_broadcasting_row() {
    // Зовёт ту же функцию, что и самопроверка каждого поведенческого файла:
    // удаление самопроверки из файла гейт не отключает.
    let mut problems: Vec<String> = Vec::new();
    for (service_file, behaviour_file) in BEHAVIOUR_TEST {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join(behaviour_file);
        match std::fs::read_to_string(&path) {
            Err(_) => problems.push(format!(
                "{behaviour_file}: поведенческого файла нет на диске (сервис {service_file})"
            )),
            Ok(src) => {
                for m in check_behaviour_file(service_file, &src, REGISTRY) {
                    problems.push(format!("{behaviour_file}: {m}"));
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "слой (2) не привязан к реальным тестам:\n  - {}",
        problems.join("\n  - ")
    );
    // Контрольная связка с публичной функцией: компилируется и паникует так же.
    let _ = assert_behaviour_file_covers;
}

// ---- мутации реальных исходников (в памяти, файлы на диске не трогаются) -----

#[test]
fn mutation_on_real_source_removing_broadcast_turns_gate_red() {
    let src = read_service("place_service.rs");
    let anchor = "broadcast_entities(\"move_node\"";
    assert_eq!(
        src.matches(anchor).count(),
        1,
        "якорь неуникален или рассылки нет (до плана 06 — ожидаемо)"
    );
    // Разностная проверка: на исходном тексте нарушения НЕТ.
    let before = scan("place_service.rs", &src, REGISTRY);
    assert!(
        !has(&before, "move_node", Kind::MissingBroadcast),
        "move_node красный уже на исходном тексте: {:?}",
        kinds(&before)
    );
    let mutated = src.replacen(anchor, "/* removed */ (\"move_node\"", 1);
    assert_ne!(mutated, src, "мутация не применилась");
    let after = scan("place_service.rs", &mutated, REGISTRY);
    assert!(
        has(&after, "move_node", Kind::MissingBroadcast),
        "снятие рассылки не покраснело: {:?}",
        kinds(&after)
    );
}

const PROBE: &str =
    "pub async fn new_mutation_for_gate_probe(&self) { self.writer.execute(|c| Ok(())).await }\n";

#[test]
fn mutation_new_pub_fn_before_first_test_module_without_verdict_is_red() {
    let mut with_tests = 0usize;
    for f in SCOPE_FILES {
        let src = read_service(f);
        assert_eq!(
            src.matches("fn new_mutation_for_gate_probe").count(),
            0,
            "{f}: проба уже есть в исходнике"
        );
        let before = scan(f, &src, REGISTRY);
        assert!(
            !has(&before, "new_mutation_for_gate_probe", Kind::NoVerdict),
            "{f}: проба красная до вставки"
        );
        // Перед первым тестовым элементом; нет тестов — в конец файла.
        let at = match first_test_item_range(&src) {
            Some((start, _)) => {
                with_tests += 1;
                start
            }
            None => src.len(),
        };
        let mut mutated = src.clone();
        mutated.insert_str(at, PROBE);
        let after = scan(f, &mutated, REGISTRY);
        assert!(
            has(&after, "new_mutation_for_gate_probe", Kind::NoVerdict),
            "{f}: новая pub fn без вердикта не покраснела: {:?}",
            kinds(&after)
        );
    }
    assert!(
        with_tests >= 2,
        "ожидали тестовые модули минимум в двух файлах области, нашли {with_tests}"
    );
}

#[test]
fn mutation_new_pub_fn_after_mid_file_test_module_in_act_service_is_red() {
    // Канарейка слепой зоны: в act_service.rs первый #[cfg(test)] стоит посреди
    // файла, дальше идёт нетестовый код.
    let f = "act_service.rs";
    let src = read_service(f);
    assert_eq!(src.matches("fn new_mutation_for_gate_probe").count(), 0);
    let (_, end) = first_test_item_range(&src).expect("в act_service.rs есть #[cfg(test)]");
    assert!(
        parse_fns(&prepare(&src)).iter().any(|fi| fi.body.0 > end),
        "после первого тестового модуля нет нетестовых функций: проба не имеет смысла"
    );
    let before = scan(f, &src, REGISTRY);
    assert!(!has(
        &before,
        "new_mutation_for_gate_probe",
        Kind::NoVerdict
    ));
    let mut mutated = src.clone();
    mutated.insert_str(end, &format!("\n{PROBE}"));
    let after = scan(f, &mutated, REGISTRY);
    assert!(
        has(&after, "new_mutation_for_gate_probe", Kind::NoVerdict),
        "функция после тестового модуля посреди файла осталась невидимой: {:?}",
        kinds(&after)
    );
}

// ---- гард области -------------------------------------------------------------

#[test]
fn writer_owning_services_outside_scope_are_listed() {
    let mut problems: Vec<String> = Vec::new();
    let mut found: BTreeSet<String> = BTreeSet::new();
    let mut scanned = 0usize;
    for entry in std::fs::read_dir(services_dir()).expect("read_dir services") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        scanned += 1;
        if SCOPE_FILES.contains(&name.as_str()) {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("read");
        if owns_writer_outside_tests(&src) {
            found.insert(name);
        }
    }
    assert!(
        scanned >= 20,
        "просмотрено {scanned} файлов services/, ожидали не меньше 20"
    );
    let listed: BTreeSet<String> = OUT_OF_SCOPE_WRITERS
        .iter()
        .map(|(f, _)| f.to_string())
        .collect();
    for f in found.difference(&listed) {
        problems.push(format!(
            "{f}: владеет writer.execute вне области гейта и не внесён в OUT_OF_SCOPE_WRITERS (если файл меняет места/состав/устройства/картриджи/группы — внесите его в SCOPE_FILES и реестр)"
        ));
    }
    for f in listed.difference(&found) {
        problems.push(format!(
            "{f}: в OUT_OF_SCOPE_WRITERS, но writer.execute в нём нет"
        ));
    }
    for (f, reason) in OUT_OF_SCOPE_WRITERS {
        if reason.chars().count() < MIN_EXEMPT_REASON_CHARS {
            problems.push(format!(
                "{f}: причина короче {MIN_EXEMPT_REASON_CHARS} символов"
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "гард области:\n  - {}",
        problems.join("\n  - ")
    );
}
