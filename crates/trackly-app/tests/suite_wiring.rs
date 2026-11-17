//! Гейт разводки интеграционных тестов по suite-бинарникам.
//!
//! В `trackly-app` и `trackly-infra` стоит `autotests = false`: cargo больше НЕ
//! находит `tests/*.rs` сам, каждая цель перечислена в `[[test]]` своего
//! `Cargo.toml`, а корни suite (`tests/suites/*.rs`) подключают файлы тестов
//! через `#[path = "../<файл>.rs"] mod <файл>;`. Цена такой схемы — новый файл,
//! который забыли подключить, молча не компилируется и не запускается. Этот
//! гейт закрывает дыру: каждый `tests/*.rs` обоих крейтов обязан входить ровно
//! в одну цель (либо строкой `#[path]` в одном suite, либо собственным
//! `[[test]]`), каждый suite обязан быть целью, каждая строка `#[path]` —
//! указывать на существующий файл.
//!
//! Если гейт покраснел на новом файле: добавьте в подходящий
//! `tests/suites/<suite>.rs` пару строк
//! `#[path = "../<файл>.rs"]` / `mod <файл>;` (алфавитный порядок).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Снимок одного крейта: манифест, имена `tests/*.rs`, корни suite.
struct CrateTests {
    manifest: String,
    /// Имена файлов `tests/*.rs` без расширения.
    files: Vec<String>,
    /// (`tests/suites/<имя>.rs`, текст).
    suites: Vec<(String, String)>,
}

fn crate_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(name)
}

fn load(name: &str) -> CrateTests {
    let dir = crate_dir(name);
    let manifest = std::fs::read_to_string(dir.join("Cargo.toml"))
        .unwrap_or_else(|e| panic!("{name}/Cargo.toml не прочитан: {e}"));
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir.join("tests")).expect("read_dir tests") {
        let path = entry.expect("entry").path();
        if path.is_file() && path.extension().is_some_and(|e| e == "rs") {
            files.push(path.file_stem().unwrap().to_string_lossy().into_owned());
        }
    }
    files.sort();
    let mut suites = Vec::new();
    let suites_dir = dir.join("tests").join("suites");
    if suites_dir.is_dir() {
        for entry in std::fs::read_dir(&suites_dir).expect("read_dir suites") {
            let path = entry.expect("entry").path();
            if path.extension().is_some_and(|e| e == "rs") {
                let rel = format!(
                    "tests/suites/{}",
                    path.file_name().unwrap().to_string_lossy()
                );
                let src = std::fs::read_to_string(&path).expect("read suite");
                suites.push((rel, src));
            }
        }
    }
    suites.sort();
    CrateTests {
        manifest,
        files,
        suites,
    }
}

/// Значения `path = "..."` внутри секций `[[test]]` манифеста.
fn test_target_paths(manifest: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_test = false;
    for line in manifest.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_test = t == "[[test]]";
            continue;
        }
        if !in_test {
            continue;
        }
        if let Some(rest) = t.strip_prefix("path") {
            let rest = rest.trim_start();
            if let Some(v) = rest.strip_prefix('=') {
                out.push(v.trim().trim_matches('"').to_string());
            }
        }
    }
    out
}

/// Имена файлов, подключённых строками `#[path = "../<имя>.rs"]` (комментарии
/// не считаются: строка должна начинаться с `#[path`).
fn path_includes(src: &str) -> Vec<String> {
    src.lines()
        .filter_map(|l| {
            l.trim()
                .strip_prefix("#[path = \"../")?
                .strip_suffix(".rs\"]")
                .map(str::to_string)
        })
        .collect()
}

fn problems(c: &CrateTests) -> Vec<String> {
    let mut out = Vec::new();
    let autotests_off = c
        .manifest
        .lines()
        .any(|l| l.split('#').next().unwrap_or("").replace(' ', "") == "autotests=false");
    if !autotests_off {
        out.push("в [package] нет `autotests = false`: cargo соберёт каждый файл отдельным бинарником вдобавок к suite".into());
    }
    let targets = test_target_paths(&c.manifest);
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    for f in &c.files {
        count.insert(f.as_str(), 0);
    }
    for t in &targets {
        if let Some(stem) = t
            .strip_prefix("tests/")
            .and_then(|r| r.strip_suffix(".rs"))
            .filter(|r| !r.contains('/'))
        {
            match count.get_mut(stem) {
                Some(n) => *n += 1,
                None => out.push(format!("цель [[test]] `{t}`: файла нет на диске")),
            }
        } else if !c.suites.iter().any(|(rel, _)| rel == t) {
            out.push(format!("цель [[test]] `{t}`: файла нет на диске"));
        }
    }
    for (rel, src) in &c.suites {
        if !targets.iter().any(|t| t == rel) {
            out.push(format!(
                "{rel}: корень suite не объявлен целью [[test]] — его тесты не запускаются"
            ));
        }
        let inc = path_includes(src);
        if inc.is_empty() {
            out.push(format!(
                "{rel}: ни одной строки `#[path = \"../<файл>.rs\"]`"
            ));
        }
        for stem in inc {
            match count.get_mut(stem.as_str()) {
                Some(n) => *n += 1,
                None => out.push(format!(
                    "{rel}: подключён `{stem}.rs`, которого нет в tests/"
                )),
            }
        }
    }
    for (stem, n) in count {
        match n {
            1 => {}
            0 => out.push(format!(
                "tests/{stem}.rs не входит ни в одну цель — его тесты молча не компилируются и не запускаются"
            )),
            n => out.push(format!(
                "tests/{stem}.rs подключён {n} раз — тесты прогоняются повторно"
            )),
        }
    }
    out
}

#[test]
fn every_app_test_file_is_wired_exactly_once() {
    let c = load("trackly-app");
    assert!(
        c.files.len() > 100,
        "подозрительно мало файлов: {}",
        c.files.len()
    );
    let p = problems(&c);
    assert!(p.is_empty(), "trackly-app:\n  - {}", p.join("\n  - "));
}

#[test]
fn every_infra_test_file_is_wired_exactly_once() {
    let c = load("trackly-infra");
    assert!(
        c.files.len() > 20,
        "подозрительно мало файлов: {}",
        c.files.len()
    );
    let p = problems(&c);
    assert!(p.is_empty(), "trackly-infra:\n  - {}", p.join("\n  - "));
}

// ---- самопроверка: гейт отличает зелёное от «ничего не ищет» ---------------

fn fixture() -> CrateTests {
    CrateTests {
        manifest: "[package]\nname = \"x\"\nautotests = false\n\n[[test]]\nname = \"x_a\"\npath = \"tests/suites/a.rs\"\n\n[[test]]\nname = \"solo\"\npath = \"tests/solo.rs\"\n".into(),
        files: vec!["one".into(), "solo".into(), "two".into()],
        suites: vec![(
            "tests/suites/a.rs".into(),
            "//! #[path = \"../ghost.rs\"] в комментарии не считается\n#[path = \"../one.rs\"]\nmod one;\n#[path = \"../two.rs\"]\nmod two;\n".into(),
        )],
    }
}

#[test]
fn suite_wiring_selftest_clean_fixture_is_green() {
    assert_eq!(problems(&fixture()), Vec::<String>::new());
}

#[test]
fn suite_wiring_selftest_detects_each_defect() {
    let has = |c: &CrateTests, needle: &str| problems(c).iter().any(|p| p.contains(needle));

    let mut c = fixture();
    c.files.push("forgotten".into());
    assert!(has(&c, "tests/forgotten.rs не входит ни в одну цель"));

    let mut c = fixture();
    c.suites[0]
        .1
        .push_str("#[path = \"../solo.rs\"]\nmod solo;\n");
    assert!(has(&c, "tests/solo.rs подключён 2 раз"));

    let mut c = fixture();
    c.suites.push((
        "tests/suites/b.rs".into(),
        "#[path = \"../one.rs\"]\nmod one;\n".into(),
    ));
    assert!(has(&c, "tests/suites/b.rs: корень suite не объявлен целью"));

    let mut c = fixture();
    c.manifest = c.manifest.replace("autotests = false", "");
    assert!(has(&c, "autotests = false"));

    let mut c = fixture();
    c.suites[0]
        .1
        .push_str("#[path = \"../gone.rs\"]\nmod gone;\n");
    assert!(has(&c, "подключён `gone.rs`, которого нет"));

    let mut c = fixture();
    c.manifest = c.manifest.replace("tests/solo.rs", "tests/missing.rs");
    assert!(has(&c, "цель [[test]] `tests/missing.rs`: файла нет"));
}

/// Мутация реального входа: вычеркнуть одну строку `#[path]` из настоящего
/// suite — гейт краснеет ровно на этом файле.
#[test]
fn suite_wiring_mutation_on_real_suite_drops_file() {
    let mut c = load("trackly-app");
    let (rel, src) = c.suites.first().cloned().expect("есть хотя бы один suite");
    let victim = path_includes(&src)
        .into_iter()
        .next()
        .expect("suite не пуст");
    let line = format!("#[path = \"../{victim}.rs\"]");
    assert_eq!(
        src.matches(&line).count(),
        1,
        "{rel}: якорь мутации неуникален"
    );
    c.suites[0].1 = src.replace(&line, "// снято мутацией");
    let p = problems(&c);
    assert!(
        p.iter()
            .any(|m| m.contains(&format!("tests/{victim}.rs не входит"))),
        "снятие строки из {rel} не покраснило гейт: {p:?}"
    );
}
