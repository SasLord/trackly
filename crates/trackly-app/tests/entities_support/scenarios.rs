//! Привязка слоя (2) реестрового гейта к РЕАЛЬНЫМ тестам (Phase 41.7, D-07, W4).
//!
//! Константы с именами сценариев, которую пишет человек, здесь нет: это был бы
//! тот же дрейф реестра, который чинит D-07. Имена берутся из ИСХОДНИКА
//! поведенческого файла по соглашению: на КАЖДУЮ строку `Broadcasts` своего
//! сервиса приходится ровно один `#[tokio::test(..)] async fn
//! scenario_<имя_функции>` (имя — буквально `func` из реестра). Прочие тесты
//! файла (откат, массовая операция, обёртки) носят ДРУГИЕ префиксы
//! (`rollback_`, `mass_`, `p4_`, `wrappers_`) и сценариями не считаются.
//!
//! Не засчитываются: комментарий, `#[ignore]`, функция без атрибута
//! `#[tokio::test]`, тело без единого `assert*` (пустой «сценарий» — подделка).

use std::collections::{BTreeMap, BTreeSet};

use regex::Regex;

use super::registry::{Entry, Verdict, REGISTRY};
use super::scan::prepare_with;

/// Итог разбора поведенческого файла.
#[derive(Debug, Default, Clone)]
pub struct Report {
    /// Настоящие сценарии (с assert-ами).
    pub real: BTreeSet<String>,
    /// Сценарии без единого assert.
    pub hollow: BTreeSet<String>,
    /// Имена, встретившиеся более одного раза.
    pub duplicated: BTreeSet<String>,
}

fn match_close(sk: &[u8], open: usize, o: u8, c: u8) -> Option<usize> {
    let mut depth = 0usize;
    for (k, &x) in sk.iter().enumerate().skip(open) {
        if x == o {
            depth += 1;
        } else if x == c {
            depth -= 1;
            if depth == 0 {
                return Some(k);
            }
        }
    }
    None
}

pub fn report(src: &str) -> Report {
    let p = prepare_with(src, false);
    let sk = p.skeleton.as_bytes();
    let attr_re = Regex::new(r"#\s*\[").expect("regex");
    // Атрибуты по порядку: (начало, конец включительно, текст skeleton внутри []).
    let mut attrs: Vec<(usize, usize, String)> = Vec::new();
    let mut resume = 0usize;
    for m in attr_re.find_iter(&p.skeleton) {
        if m.start() < resume {
            continue;
        }
        let open = m.end() - 1;
        if let Some(close) = match_close(sk, open, b'[', b']') {
            attrs.push((
                m.start(),
                close,
                p.skeleton[open + 1..close].trim().to_string(),
            ));
            resume = close + 1;
        }
    }
    let fn_re =
        Regex::new(r"^\s*(?:pub(?:\([^)]*\))?\s+)?async\s+fn\s+scenario_(\w+)").expect("regex");
    let assert_re = Regex::new(r"\bassert\w*\s*[!(]").expect("regex");
    let mut rep = Report::default();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();

    let mut k = 0usize;
    while k < attrs.len() {
        // Серия подряд идущих атрибутов.
        let mut j = k;
        while j + 1 < attrs.len()
            && p.skeleton[attrs[j].1 + 1..attrs[j + 1].0]
                .chars()
                .all(char::is_whitespace)
        {
            j += 1;
        }
        let run = &attrs[k..=j];
        let tail = attrs[j].1 + 1;
        k = j + 1;
        let tokio_test = run.iter().any(|a| {
            let t: String = a.2.chars().filter(|c| !c.is_whitespace()).collect();
            t.starts_with("tokio::test")
        });
        let ignored = run.iter().any(|a| {
            let t = a.2.trim_start();
            t == "ignore"
                || t.starts_with("ignore(")
                || t.starts_with("ignore ")
                || t.starts_with("ignore=")
        });
        if !tokio_test || ignored {
            continue;
        }
        let Some(c) = fn_re.captures(&p.skeleton[tail..]) else {
            continue;
        };
        let name = c.get(1).expect("name").as_str().to_string();
        let name_end = tail + c.get(0).expect("m").end();
        // Тело: первая `{` на нулевой глубине () и [].
        let (mut par, mut brk) = (0i64, 0i64);
        let mut open = None;
        for (q, &x) in sk.iter().enumerate().skip(name_end) {
            match x {
                b'(' => par += 1,
                b')' => par -= 1,
                b'[' => brk += 1,
                b']' => brk -= 1,
                b'{' if par == 0 && brk == 0 => {
                    open = Some(q);
                    break;
                }
                b';' if par == 0 && brk == 0 => break,
                _ => {}
            }
        }
        let Some(open) = open else { continue };
        let Some(close) = match_close(sk, open, b'{', b'}') else {
            continue;
        };
        *seen.entry(name.clone()).or_default() += 1;
        if assert_re.is_match(&p.skeleton[open..=close]) {
            rep.real.insert(name);
        } else {
            rep.hollow.insert(name);
        }
    }
    rep.duplicated = seen
        .into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(k, _)| k)
        .collect();
    rep
}

/// Имена настоящих сценариев (суффиксы после `scenario_`).
pub fn scenario_names(src: &str) -> BTreeSet<String> {
    report(src).real
}

/// Чистая сверка поведенческого файла с реестром; пустой список — всё в порядке.
pub fn check_behaviour_file(
    service_file: &str,
    behaviour_src: &str,
    registry: &[Entry],
) -> Vec<String> {
    let rep = report(behaviour_src);
    let expected: BTreeSet<&str> = registry
        .iter()
        .filter(|e| e.file == service_file && e.verdict == Verdict::Broadcasts)
        .map(|e| e.func)
        .collect();
    let mut problems = Vec::new();
    for f in &expected {
        if rep.hollow.contains(*f) && !rep.real.contains(*f) {
            problems.push(format!("scenario_{f} пуст (нет assert)"));
        } else if !rep.real.contains(*f) {
            problems.push(format!(
                "для строки Broadcasts {service_file}::{f} нет настоящего #[tokio::test] async fn scenario_{f}"
            ));
        }
    }
    for x in rep.real.iter().chain(rep.hollow.iter()) {
        if !expected.contains(x.as_str()) {
            problems.push(format!(
                "scenario_{x} не соответствует ни одной строке Broadcasts файла {service_file} (устаревший сценарий)"
            ));
        }
    }
    for x in &rep.duplicated {
        problems.push(format!("scenario_{x} объявлен более одного раза"));
    }
    problems
}

/// Читает `tests/<behaviour_file>` и паникует со ВСЕМИ проблемами сразу.
/// Зовётся однострочным тестом каждого поведенческого файла И гейтом.
pub fn assert_behaviour_file_covers(behaviour_file: &str, service_file: &str) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(behaviour_file);
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("не удалось прочитать {}: {e}", path.display()));
    let problems = check_behaviour_file(service_file, &src, REGISTRY);
    assert!(
        problems.is_empty(),
        "{behaviour_file}: привязка слоя (2) нарушена:\n  - {}",
        problems.join("\n  - ")
    );
}
