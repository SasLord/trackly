//! Phase 41.10 (HST-06, план 13): реестровый гейт мест записи статуса и состава.
//!
//! Инвентарь снимается ОТ РЕПОЗИТОРИЯ (методы `crates/trackly-infra/src/repos/*.rs`,
//! пишущие `devices.status_id` и состав групп), а не от списка из плана: метод без
//! вердикта краснит гейт. Вызовы писателей в сервисах закреплены числом по файлам и
//! сверены с числом вызовов записывающих историю примитивов; освобождения
//! (рождение устройства, restore отката) кодированы вердиктами с причиной.
//!
//! Гейт слеп к порядку (счёт не видит перестановки), поэтому порядок D-25
//! «перемещение, потом событие» держит отдельная проверка, а selftest включает
//! мутацию-ПЕРЕСТАНОВКУ реального исходника. Источник конструкции: `group_write_sites.rs`
//! (его функции приватны, поэтому здесь копии с пометкой).

use std::sync::OnceLock;

const REPOS_PREFIX: &str = "crates/trackly-infra/src/repos/";

// ---------------------------------------------------------------------------
// Общие помощники (копии из group_write_sites.rs, доработанные под production-код)
// ---------------------------------------------------------------------------

fn code_of(line: &str) -> &str {
    line.split("//").next().unwrap_or("")
}

/// Исходник без `#[cfg(test)]`-блоков (по балансу скобок) и без `//`-комментариев.
/// Номера строк сохраняются (вырезанные строки становятся пустыми).
fn production_code(src: &str) -> String {
    let lines: Vec<&str> = src.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim() == "#[cfg(test)]" {
            // пропустить сам атрибут и весь следующий элемент
            let mut depth: i32 = 0;
            let mut opened = false;
            out.push(String::new());
            i += 1;
            while i < lines.len() {
                let code = code_of(lines[i]);
                out.push(String::new());
                for ch in code.chars() {
                    match ch {
                        '{' => {
                            depth += 1;
                            opened = true;
                        }
                        '}' => depth -= 1,
                        _ => {}
                    }
                }
                i += 1;
                if (opened && depth <= 0) || (!opened && code.trim_end().ends_with(';')) {
                    break;
                }
            }
            continue;
        }
        out.push(code_of(lines[i]).to_string());
        i += 1;
    }
    out.join("\n")
}

fn ident_boundary_before(code: &str, pos: usize) -> bool {
    code[..pos]
        .chars()
        .next_back()
        .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
}

/// Число вызовов `needles`: определения (`fn `) не считаются, слева от вхождения
/// нет символа идентификатора. Принимает УЖЕ очищенный production-код.
fn count_calls(code_src: &str, needles: &[&str]) -> usize {
    code_src
        .lines()
        .map(|code| {
            if code.contains("fn ") {
                return 0;
            }
            needles
                .iter()
                .map(|n| {
                    code.match_indices(n)
                        .filter(|(pos, _)| ident_boundary_before(code, *pos))
                        .count()
                })
                .sum::<usize>()
        })
        .sum()
}

/// (файл, число вызовов) -> нарушения. `sources` — production-код.
fn registry_violations(
    sources: &[(String, String)],
    needles: &[&str],
    registry: &[(&str, usize)],
    skip_prefix: Option<&str>,
    hint: &str,
) -> Vec<String> {
    let mut violations = Vec::new();
    let mut seen: Vec<(String, usize)> = Vec::new();
    for (rel, text) in sources {
        if skip_prefix.is_some_and(|p| rel.starts_with(p)) {
            continue;
        }
        let n = count_calls(text, needles);
        if n > 0 {
            seen.push((rel.clone(), n));
        }
    }
    for (rel, n) in &seen {
        match registry.iter().find(|(f, _)| f == rel) {
            None => violations.push(format!("новое место {hint} в {rel} ({n} вызовов)")),
            Some((_, want)) if n != want => violations.push(format!(
                "число вызовов {hint} в {rel} изменилось ({n}, в реестре {want})"
            )),
            Some(_) => {}
        }
    }
    for (rel, want) in registry {
        if !seen.iter().any(|(f, _)| f == rel) {
            violations.push(format!(
                "файл реестра {rel} ({want} вызовов {hint}) больше не содержит вызовов"
            ));
        }
    }
    violations
}

fn raw_sources() -> &'static Vec<(String, String)> {
    static CELL: OnceLock<Vec<(String, String)>> = OnceLock::new();
    CELL.get_or_init(|| {
        fn walk(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<(String, String)>) {
            for entry in std::fs::read_dir(dir).expect("read_dir") {
                let path = entry.expect("entry").path();
                if path.is_dir() {
                    walk(&path, root, out);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let rel = path
                        .strip_prefix(root)
                        .expect("strip_prefix")
                        .to_string_lossy()
                        .replace('\\', "/");
                    let text = std::fs::read_to_string(&path).expect("read source");
                    out.push((rel, text));
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let root = root.canonicalize().expect("repo root");
        let mut out = Vec::new();
        for krate in std::fs::read_dir(root.join("crates")).expect("crates dir") {
            let src = krate.expect("crate entry").path().join("src");
            if src.is_dir() {
                walk(&src, &root, &mut out);
            }
        }
        out.sort();
        out
    })
}

fn prod_sources() -> Vec<(String, String)> {
    raw_sources()
        .iter()
        .map(|(rel, text)| (rel.clone(), production_code(text)))
        .collect()
}

/// Исходник из реального дерева; отсутствие файла — провал, а не молчание.
fn raw_of(rel: &str) -> &'static str {
    raw_sources()
        .iter()
        .find(|(r, _)| r == rel)
        .map(|(_, t)| t.as_str())
        .unwrap_or_else(|| panic!("файл реестра отсутствует в дереве: {rel}"))
}

// ---------------------------------------------------------------------------
// Реестр 1: методы репозиториев (инвентарь ОТ РЕПОЗИТОРИЯ)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum Verdict {
    /// вызов в сервисе обязан сопровождаться записью события (см. реестры вызовов)
    EventRequired,
    /// рождение устройства: события «смена статуса» нет по смыслу (D-06)
    BirthExempt,
    /// restore отката: события удаляются по act_id, новых не пишется (D-24)
    RestoreNoEvent,
    /// продакшн-вызовов нет (не-tx методы трейта)
    NoProductionCallers,
    /// состав/статус не меняется, события по решению D-08 нет (см. `NEUTRAL_CALLS`)
    NoEventByDesign,
}

const DEVICES_REPO: &str = "crates/trackly-infra/src/repos/devices_sqlite.rs";
const GROUPS_REPO: &str = "crates/trackly-infra/src/repos/groups_sqlite.rs";

/// Методы репозиториев, пишущие `devices.status_id` (SQL-литерал INSERT INTO/UPDATE devices
/// упоминает `status_id`). Метод без вердикта краснит гейт.
const STATUS_WRITER_METHODS: [(&str, &str, Verdict); 9] = [
    (DEVICES_REPO, "update_in_tx", Verdict::EventRequired),
    (
        DEVICES_REPO,
        "update_status_and_place_in_tx",
        Verdict::EventRequired,
    ),
    (DEVICES_REPO, "set_status_in_tx", Verdict::EventRequired),
    (DEVICES_REPO, "update_full_in_tx", Verdict::EventRequired),
    (DEVICES_REPO, "create_in_tx", Verdict::BirthExempt),
    (DEVICES_REPO, "clone_device_in_tx", Verdict::BirthExempt),
    (
        DEVICES_REPO,
        "restore_from_snapshot_in_tx",
        Verdict::RestoreNoEvent,
    ),
    (DEVICES_REPO, "create", Verdict::NoProductionCallers),
    (DEVICES_REPO, "update", Verdict::NoProductionCallers),
];

/// Методы репозитория групп, пишущие состав (`group_devices`, строки/родитель/якорь `groups`).
/// `EventRequired` = событие пишет сервисный слой (см. реестры вызовов); `RestoreNoEvent`
/// относится к конкретному файлу-вызывателю и закодирован в `EXEMPT_CALLS`.
const MEMBERSHIP_WRITER_METHODS: [(&str, &str, Verdict); 6] = [
    (GROUPS_REPO, "insert_group_in_tx", Verdict::EventRequired),
    (GROUPS_REPO, "delete_group_in_tx", Verdict::EventRequired),
    (GROUPS_REPO, "add_device_in_tx", Verdict::EventRequired),
    (GROUPS_REPO, "remove_device_in_tx", Verdict::EventRequired),
    (GROUPS_REPO, "set_parent_in_tx", Verdict::EventRequired),
    (GROUPS_REPO, "set_anchor_in_tx", Verdict::NoEventByDesign),
];

const STATUS_SQL_MARKERS: [&str; 2] = ["INSERT INTO devices", "UPDATE devices"];
const MEMBERSHIP_SQL_MARKERS: [&str; 6] = [
    "INSERT INTO group_devices",
    "DELETE FROM group_devices",
    "INSERT INTO groups",
    "DELETE FROM groups",
    "UPDATE groups SET parent_group_id",
    "UPDATE groups SET anchor_device_id",
];

/// Имена методов (`fn`) production-кода, в теле которых SQL-литерал с одним из `markers`
/// (и, если задан `must_mention`, упоминающий его до закрывающей кавычки).
fn writing_methods(code_src: &str, markers: &[&str], must_mention: Option<&str>) -> Vec<String> {
    let mut methods: Vec<(String, String)> = Vec::new();
    for code in code_src.lines() {
        if let Some(pos) = code.find("fn ") {
            if ident_boundary_before(code, pos) {
                let name: String = code[pos + 3..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                methods.push((name, String::new()));
            }
        }
        if let Some((_, body)) = methods.last_mut() {
            body.push_str(code);
            body.push('\n');
        }
    }
    let mut out: Vec<String> = methods
        .into_iter()
        .filter(|(_, body)| {
            markers.iter().any(|m| {
                body.match_indices(m).any(|(pos, _)| match must_mention {
                    None => true,
                    Some(word) => body[pos..].split('"').next().unwrap_or("").contains(word),
                })
            })
        })
        .map(|(n, _)| n)
        .collect();
    out.sort();
    out.dedup();
    out
}

/// (новые без вердикта, пропавшие из исходника) для набора методов файла `rel`.
fn writer_diff(
    code_src: &str,
    rel: &str,
    markers: &[&str],
    must_mention: Option<&str>,
    registry: &[(&str, &str, Verdict)],
) -> (Vec<String>, Vec<String>) {
    let found = writing_methods(code_src, markers, must_mention);
    let frozen: Vec<&str> = registry
        .iter()
        .filter(|(f, _, _)| *f == rel)
        .map(|(_, n, _)| *n)
        .collect();
    let new = found
        .iter()
        .filter(|n| !frozen.contains(&n.as_str()))
        .cloned()
        .collect();
    let gone = frozen
        .iter()
        .filter(|n| !found.iter().any(|f| f == *n))
        .map(|n| n.to_string())
        .collect();
    (new, gone)
}

// ---------------------------------------------------------------------------
// Реестр 2: вызовы писателей в сервисах (обязательные) и освобождения
// ---------------------------------------------------------------------------

/// (needle, файл, число вызовов): каждый такой вызов сопровождается записью события.
/// Числа сняты grep'ом по production-коду на момент плана 13 и сверены с SUMMARY 05-10, 18.
const REQUIRED_CALLS: [(&str, &str, usize); 13] = [
    // --- статус (D-06) ---
    (
        "update_in_tx(",
        "crates/trackly-app/src/services/device_service.rs",
        1,
    ),
    (
        "update_status_and_place_in_tx(",
        "crates/trackly-app/src/services/act_service.rs",
        2,
    ),
    (
        "update_status_and_place_in_tx(",
        "crates/trackly-app/src/services/group_place.rs",
        1,
    ),
    (
        "update_status_and_place_in_tx(",
        "crates/trackly-app/src/services/place_service.rs",
        1,
    ),
    (
        "update_status_and_place_in_tx(",
        "crates/trackly-app/src/services/repair_service.rs",
        1,
    ),
    (
        "set_status_in_tx(",
        "crates/trackly-app/src/services/group_service.rs",
        3,
    ),
    (
        "update_full_in_tx(",
        "crates/trackly-app/src/services/act_service.rs",
        3,
    ),
    // --- состав (D-08) ---
    (
        "add_device_in_tx(",
        "crates/trackly-app/src/services/group_service.rs",
        2,
    ),
    (
        "remove_device_in_tx(",
        "crates/trackly-app/src/services/group_membership.rs",
        1,
    ),
    (
        "set_parent_in_tx(",
        "crates/trackly-app/src/services/group_act_plan.rs",
        3,
    ),
    (
        "set_parent_in_tx(",
        "crates/trackly-app/src/services/group_service.rs",
        1,
    ),
    (
        "delete_group_in_tx(",
        "crates/trackly-app/src/services/group_service.rs",
        1,
    ),
    (
        "insert_group_in_tx(",
        "crates/trackly-app/src/services/group_service.rs",
        1,
    ),
];

/// Вызовы без требования события: (needle, файл, число, причина >= 40 символов).
/// Одна пара (needle, файл) не может быть одновременно в `REQUIRED_CALLS`.
const EXEMPT_CALLS: [(&str, &str, usize, &str); 5] = [
    (
        "repo.create_in_tx(",
        "crates/trackly-app/src/services/device_service.rs",
        2,
        "рождение устройства (D-06): статуса «до» нет, смены статуса не происходит",
    ),
    (
        "clone_device_in_tx(",
        "crates/trackly-app/src/services/act_service.rs",
        1,
        "рождение клона при передаче: у новой строки нет статуса «до», это не смена",
    ),
    (
        "restore_from_snapshot_in_tx(",
        "crates/trackly-app/src/services/act_service.rs",
        3,
        "restore отката акта (D-24): события удаляются по act_id, новых не пишется",
    ),
    (
        "add_device_in_tx(",
        "crates/trackly-app/src/services/group_membership.rs",
        1,
        "restore состава при откате акта (D-24): откат событий не пишет, а удаляет",
    ),
    (
        "set_parent_in_tx(",
        "crates/trackly-app/src/services/group_act_plan.rs",
        1,
        "возврат вложенности при откате акта (D-24): событие удалено по act_id",
    ),
];

/// Вызовы, сопровождающие писателей, из `REQUIRED_CALLS`, не имеющие своего вердикта
/// кроме `set_anchor_in_tx(` (смена якоря группы: состав не меняется, события нет по D-08).
const NEUTRAL_CALLS: [(&str, &str, usize, &str); 1] = [(
    "set_anchor_in_tx(",
    "crates/trackly-app/src/services/group_service.rs",
    1,
    "смена якоря не меняет состав и статус: отдельного события D-08 нет",
)];

/// Примитивы записи событий: (needle, файл, число). `history_events.rs` (определение)
/// пропускается. `record_act_status_event(` — приватная обёртка act_service, зовущая
/// `record_status_change_in_tx` один раз (7 сайтов -> 1 вызов внутри).
const STATUS_EVENT_NEEDLE: &str = "record_status_change_in_tx(";
const STATUS_EVENT_REGISTRY: [(&str, usize); 6] = [
    ("crates/trackly-app/src/services/act_service.rs", 1),
    ("crates/trackly-app/src/services/device_service.rs", 1),
    ("crates/trackly-app/src/services/group_place.rs", 1),
    ("crates/trackly-app/src/services/group_service.rs", 3),
    ("crates/trackly-app/src/services/place_service.rs", 1),
    ("crates/trackly-app/src/services/repair_service.rs", 1),
];
const ACT_WRAPPER_NEEDLE: &str = "record_act_status_event(";
const ACT_WRAPPER_REGISTRY: [(&str, usize); 1] =
    [("crates/trackly-app/src/services/act_service.rs", 7)];
const UNNESTED_WRAPPER_NEEDLE: &str = "record_act_unnested(";
const UNNESTED_WRAPPER_REGISTRY: [(&str, usize); 1] =
    [("crates/trackly-app/src/services/group_act_plan.rs", 3)];
const GROUP_EVENT_NEEDLES: [&str; 7] = [
    "record_group_joined_in_tx(",
    "record_group_left_in_tx(",
    "record_group_transferred_in_tx(",
    "record_group_formed_in_tx(",
    "record_group_disbanded_in_tx(",
    "record_group_nested_in_tx(",
    "record_group_unnested_in_tx(",
];
const GROUP_EVENT_REGISTRY: [(&str, usize); 3] = [
    ("crates/trackly-app/src/services/group_act_plan.rs", 1),
    // left (release_inner) + joined (возврат в состав при правке живого акта, CR-01)
    ("crates/trackly-app/src/services/group_membership.rs", 2),
    ("crates/trackly-app/src/services/group_service.rs", 10),
];
const HISTORY_EVENTS_FILE: &str = "crates/trackly-app/src/services/history_events.rs";

/// Все needle'ы писателей (обязательные + освобождённые + нейтральные), без повторов.
fn all_writer_needles() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = REQUIRED_CALLS
        .iter()
        .map(|(n, _, _)| *n)
        .chain(EXEMPT_CALLS.iter().map(|(n, _, _, _)| *n))
        .chain(NEUTRAL_CALLS.iter().map(|(n, _, _, _)| *n))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Реестр файл->число для одного needle: сумма обязательных, освобождённых и нейтральных.
fn registry_for(needle: &str) -> Vec<(&'static str, usize)> {
    let mut v: Vec<(&'static str, usize)> = Vec::new();
    let mut add = |file: &'static str, n: usize| match v.iter_mut().find(|(f, _)| *f == file) {
        Some((_, c)) => *c += n,
        None => v.push((file, n)),
    };
    for (n, f, c) in REQUIRED_CALLS {
        if n == needle {
            add(f, c);
        }
    }
    for (n, f, c, _) in EXEMPT_CALLS {
        if n == needle {
            add(f, c);
        }
    }
    for (n, f, c, _) in NEUTRAL_CALLS {
        if n == needle {
            add(f, c);
        }
    }
    v
}

/// Нарушения реестра вызовов писателей на ЗАДАННЫХ production-исходниках.
fn writer_call_violations(sources: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for needle in all_writer_needles() {
        let reg = registry_for(needle);
        out.extend(registry_violations(
            sources,
            &[needle],
            &reg,
            Some(REPOS_PREFIX),
            &format!("писателя `{needle}`"),
        ));
    }
    out
}

/// Нарушения реестров вызовов примитивов событий.
fn event_call_violations(sources: &[(String, String)]) -> Vec<String> {
    let skip_history: Vec<(String, String)> = sources
        .iter()
        .filter(|(r, _)| r != HISTORY_EVENTS_FILE)
        .cloned()
        .collect();
    let mut out = registry_violations(
        &skip_history,
        &[STATUS_EVENT_NEEDLE],
        &STATUS_EVENT_REGISTRY,
        None,
        "record_status_change_in_tx",
    );
    out.extend(registry_violations(
        &skip_history,
        &[ACT_WRAPPER_NEEDLE],
        &ACT_WRAPPER_REGISTRY,
        None,
        "record_act_status_event",
    ));
    out.extend(registry_violations(
        &skip_history,
        &[UNNESTED_WRAPPER_NEEDLE],
        &UNNESTED_WRAPPER_REGISTRY,
        None,
        "record_act_unnested",
    ));
    out.extend(registry_violations(
        &skip_history,
        &GROUP_EVENT_NEEDLES,
        &GROUP_EVENT_REGISTRY,
        None,
        "record_group_*",
    ));
    out
}

// ---------------------------------------------------------------------------
// Порядок D-25: перемещение РАНЬШЕ события
// ---------------------------------------------------------------------------

const MOVEMENT_NEEDLES: [&str; 3] = [
    "record_movement_if_applicable(",
    "record_repair_movement_if_applicable(",
    "record_batch_movement_if_applicable(",
];
const ORDER_EVENT_NEEDLES: [&str; 2] = [STATUS_EVENT_NEEDLE, ACT_WRAPPER_NEEDLE];

/// Диапазоны функций по строкам: (имя, первая строка, последняя строка исключительно).
fn fn_ranges(code_lines: &[&str]) -> Vec<(String, usize, usize)> {
    let mut starts: Vec<(String, usize)> = Vec::new();
    for (i, code) in code_lines.iter().enumerate() {
        if let Some(pos) = code.find("fn ") {
            if ident_boundary_before(code, pos) {
                let name: String = code[pos + 3..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    starts.push((name, i));
                }
            }
        }
    }
    let mut out = Vec::new();
    for (k, (name, s)) in starts.iter().enumerate() {
        let e = starts.get(k + 1).map_or(code_lines.len(), |(_, n)| *n);
        out.push((name.clone(), *s, e));
    }
    out
}

fn line_calls(code: &str, needles: &[&str]) -> usize {
    needles
        .iter()
        .map(|n| {
            code.match_indices(n)
                .filter(|(p, _)| ident_boundary_before(code, *p))
                .count()
        })
        .sum()
}

/// Нарушения порядка в production-коде: в каждой функции, где есть и перемещение, и событие,
/// перед каждым событием должно стоять не меньше перемещений, чем событий (префиксный баланс).
/// Функция-определение самой обёртки (`record_act_status_event`) не проверяется.
fn order_violations(code_src: &str) -> Vec<String> {
    let lines: Vec<&str> = code_src.lines().collect();
    let mut out = Vec::new();
    for (name, s, e) in fn_ranges(&lines) {
        let mut moves = 0usize;
        let mut events = 0usize;
        let mut has_both = false;
        let total_m: usize = lines[s..e]
            .iter()
            .map(|l| line_calls(l, &MOVEMENT_NEEDLES))
            .sum();
        let total_e: usize = lines[s..e]
            .iter()
            .map(|l| line_calls(l, &ORDER_EVENT_NEEDLES))
            .sum();
        if total_m > 0 && total_e > 0 {
            has_both = true;
        }
        if !has_both {
            continue;
        }
        for (off, l) in lines[s..e].iter().enumerate() {
            // порядок внутри одной строки: сперва перемещения
            moves += line_calls(l, &MOVEMENT_NEEDLES);
            let ev = line_calls(l, &ORDER_EVENT_NEEDLES);
            if ev > 0 && l.contains("fn ") {
                continue;
            }
            events += ev;
            if events > moves {
                out.push(format!(
                    "{name}: событие (строка {}) записано раньше перемещения (D-25)",
                    s + off + 1
                ));
                break;
            }
        }
    }
    out
}

/// Файлы, где порядок обязан проверяться (есть и перемещение, и событие в одной функции).
const ORDER_FILES: [&str; 3] = [
    "crates/trackly-app/src/services/device_service.rs",
    "crates/trackly-app/src/services/act_service.rs",
    "crates/trackly-app/src/services/repair_service.rs",
];

/// Для каждой функции с парой «перемещение+событие» меняет местами ИМЕНА первого перемещения
/// и первого события на их строках — это перестановка двух вызовов, какой её видит гейт.
/// Возвращает (мутированный исходник, имя функции). Якорь уникален по построению: берётся
/// первая строка кода каждой стороны внутри диапазона функции, строки различны.
fn swap_first_pair(raw: &str, target_fn: &str) -> Option<String> {
    let code = production_code(raw);
    let code_lines: Vec<&str> = code.lines().collect();
    let (_, s, e) = fn_ranges(&code_lines).into_iter().find(|(n, s, e)| {
        n == target_fn
            && code_lines[*s..*e]
                .iter()
                .any(|l| line_calls(l, &MOVEMENT_NEEDLES) > 0)
    })?;
    let mi = (s..e).find(|i| line_calls(code_lines[*i], &MOVEMENT_NEEDLES) > 0)?;
    let ei = (s..e).find(|i| line_calls(code_lines[*i], &ORDER_EVENT_NEEDLES) > 0)?;
    if mi == ei {
        return None;
    }
    let mneedle = MOVEMENT_NEEDLES
        .iter()
        .find(|n| code_lines[mi].contains(*n))?;
    let eneedle = ORDER_EVENT_NEEDLES
        .iter()
        .find(|n| code_lines[ei].contains(*n))?;
    let mut raw_lines: Vec<String> = raw.lines().map(String::from).collect();
    // production_code сохраняет нумерацию строк, поэтому индексы совпадают с raw
    raw_lines[mi] = raw_lines[mi].replacen(mneedle, "__SWAP_PLACEHOLDER__(", 1);
    raw_lines[ei] = raw_lines[ei].replacen(eneedle, mneedle, 1);
    raw_lines[mi] = raw_lines[mi].replacen("__SWAP_PLACEHOLDER__(", eneedle, 1);
    Some(raw_lines.join("\n"))
}

/// Имена функций исходника, где есть пара «перемещение+событие».
fn pair_fns(raw: &str) -> Vec<String> {
    let code = production_code(raw);
    let lines: Vec<&str> = code.lines().collect();
    fn_ranges(&lines)
        .into_iter()
        .filter(|(_, s, e)| {
            lines[*s..*e]
                .iter()
                .any(|l| line_calls(l, &MOVEMENT_NEEDLES) > 0)
                && lines[*s..*e]
                    .iter()
                    .any(|l| line_calls(l, &ORDER_EVENT_NEEDLES) > 0)
        })
        .map(|(n, _, _)| n)
        .collect()
}

// ---------------------------------------------------------------------------
// Тесты на реальном дереве
// ---------------------------------------------------------------------------

#[test]
fn history_write_sites_scan_is_not_vacuous() {
    let sources = raw_sources();
    assert!(
        sources.len() > 50,
        "скан нашёл только {} исходников — гейт вакуумен",
        sources.len()
    );
    assert!(!STATUS_WRITER_METHODS.is_empty() && !MEMBERSHIP_WRITER_METHODS.is_empty());
    assert!(!REQUIRED_CALLS.is_empty() && !STATUS_EVENT_REGISTRY.is_empty());
    // каждый файл любого реестра существует (SKIP-able файл молчал бы навсегда)
    let mut files: Vec<&str> = Vec::new();
    files.extend(STATUS_WRITER_METHODS.iter().map(|(f, _, _)| *f));
    files.extend(MEMBERSHIP_WRITER_METHODS.iter().map(|(f, _, _)| *f));
    files.extend(REQUIRED_CALLS.iter().map(|(_, f, _)| *f));
    files.extend(EXEMPT_CALLS.iter().map(|(_, f, _, _)| *f));
    files.extend(NEUTRAL_CALLS.iter().map(|(_, f, _, _)| *f));
    files.extend(STATUS_EVENT_REGISTRY.iter().map(|(f, _)| *f));
    files.extend(ACT_WRAPPER_REGISTRY.iter().map(|(f, _)| *f));
    files.extend(UNNESTED_WRAPPER_REGISTRY.iter().map(|(f, _)| *f));
    files.extend(GROUP_EVENT_REGISTRY.iter().map(|(f, _)| *f));
    files.extend(ORDER_FILES);
    for f in files {
        raw_of(f);
    }
}

/// HST-06 / T-41.10-35: инвентарь от репозитория — каждый метод, пишущий статус или
/// состав, есть в реестре вердиктов, и наоборот.
#[test]
fn history_write_sites_repo_inventory_matches_registry() {
    let sources = prod_sources();
    // (1) статус: сканируем ВСЕ файлы репозиториев
    let mut status_new: Vec<String> = Vec::new();
    for (rel, code) in sources.iter().filter(|(r, _)| r.starts_with(REPOS_PREFIX)) {
        let (new, _) = writer_diff(
            code,
            rel,
            &STATUS_SQL_MARKERS,
            Some("status_id"),
            &STATUS_WRITER_METHODS,
        );
        status_new.extend(new.into_iter().map(|m| format!("{rel}::{m}")));
    }
    assert!(
        status_new.is_empty(),
        "метод пишет devices.status_id без вердикта в STATUS_WRITER_METHODS: {status_new:?}"
    );
    let dev_code = &sources.iter().find(|(r, _)| r == DEVICES_REPO).unwrap().1;
    let (_, gone) = writer_diff(
        dev_code,
        DEVICES_REPO,
        &STATUS_SQL_MARKERS,
        Some("status_id"),
        &STATUS_WRITER_METHODS,
    );
    assert!(
        gone.is_empty(),
        "реестр хранит исчезнувшие методы: {gone:?}"
    );
    // (2) состав
    let mut mem_new: Vec<String> = Vec::new();
    for (rel, code) in sources.iter().filter(|(r, _)| r.starts_with(REPOS_PREFIX)) {
        let (new, _) = writer_diff(
            code,
            rel,
            &MEMBERSHIP_SQL_MARKERS,
            None,
            &MEMBERSHIP_WRITER_METHODS,
        );
        mem_new.extend(new.into_iter().map(|m| format!("{rel}::{m}")));
    }
    assert!(
        mem_new.is_empty(),
        "метод пишет состав групп без вердикта в MEMBERSHIP_WRITER_METHODS: {mem_new:?}"
    );
    let grp_code = &sources.iter().find(|(r, _)| r == GROUPS_REPO).unwrap().1;
    let (_, gone) = writer_diff(
        grp_code,
        GROUPS_REPO,
        &MEMBERSHIP_SQL_MARKERS,
        None,
        &MEMBERSHIP_WRITER_METHODS,
    );
    assert!(
        gone.is_empty(),
        "реестр хранит исчезнувшие методы: {gone:?}"
    );
}

/// Каждый EventRequired-метод репозитория имеет needle в `REQUIRED_CALLS`, каждый
/// BirthExempt/RestoreNoEvent — в `EXEMPT_CALLS` (вердикт кодирован числом, не комментарием).
#[test]
fn history_write_sites_verdicts_match_call_registries() {
    let required: Vec<&str> = REQUIRED_CALLS.iter().map(|(n, _, _)| *n).collect();
    let exempt: Vec<&str> = EXEMPT_CALLS.iter().map(|(n, _, _, _)| *n).collect();
    for (_, m, v) in STATUS_WRITER_METHODS
        .iter()
        .chain(MEMBERSHIP_WRITER_METHODS.iter())
    {
        let probe = |ns: &[&str]| ns.iter().any(|n| n.trim_end_matches('(').ends_with(m));
        match v {
            Verdict::EventRequired => assert!(
                probe(&required),
                "{m}: EventRequired, но нет строки в REQUIRED_CALLS"
            ),
            Verdict::BirthExempt | Verdict::RestoreNoEvent => assert!(
                probe(&exempt),
                "{m}: вердикт-освобождение без строки в EXEMPT_CALLS"
            ),
            Verdict::NoEventByDesign => assert!(
                NEUTRAL_CALLS
                    .iter()
                    .any(|(n, _, _, r)| n.trim_end_matches('(').ends_with(m)
                        && r.chars().count() >= 40),
                "{m}: NoEventByDesign без строки с причиной в NEUTRAL_CALLS"
            ),
            Verdict::NoProductionCallers => {
                assert!(
                    !probe(&required) && !probe(&exempt),
                    "{m}: NoProductionCallers, но у метода есть вызовы в реестрах"
                );
            }
        }
    }
    // освобождение не прячет обязательный вызов (пара needle+файл уникальна)
    for (n, f, _, reason) in EXEMPT_CALLS {
        assert!(
            reason.chars().count() >= 40,
            "причина освобождения {n} в {f} короче 40 символов"
        );
        // одна пара needle+файл может делиться (set_parent_in_tx в group_act_plan: 3 отцепления
        // + 1 restore), но тогда освобождённое число обязано быть меньше всего числа в файле
        let total = registry_for(n)
            .iter()
            .find(|(rf, _)| *rf == f)
            .map_or(0, |(_, c)| *c);
        let exempt_here: usize = EXEMPT_CALLS
            .iter()
            .filter(|(en, ef, _, _)| *en == n && *ef == f)
            .map(|(_, _, c, _)| *c)
            .sum();
        let required_here: usize = REQUIRED_CALLS
            .iter()
            .filter(|(rn, rf, _)| *rn == n && *rf == f)
            .map(|(_, _, c)| *c)
            .sum();
        assert_eq!(
            total,
            exempt_here + required_here,
            "{n} в {f}: итог реестра не равен сумме обязательных и освобождённых"
        );
    }
}

/// D-06 / D-08: числа вызовов писателей по файлам закреплены.
#[test]
fn history_write_sites_writer_calls_match_registry() {
    let v = writer_call_violations(&prod_sources());
    assert!(v.is_empty(), "реестр вызовов писателей разошёлся: {v:#?}");
}

/// Числа вызовов примитивов событий закреплены (по файлам; обёртка act_service —
/// отдельно, внутри неё один вызов `record_status_change_in_tx`).
#[test]
fn history_write_sites_event_calls_match_registry() {
    let v = event_call_violations(&prod_sources());
    assert!(v.is_empty(), "реестр вызовов событий разошёлся: {v:#?}");
}

/// Файл, где есть писатель статуса, обязан иметь и событие (иначе писатель без события).
#[test]
fn history_write_sites_status_writer_files_have_events() {
    let event_files: Vec<&str> = STATUS_EVENT_REGISTRY
        .iter()
        .map(|(f, _)| *f)
        .chain(ACT_WRAPPER_REGISTRY.iter().map(|(f, _)| *f))
        .collect();
    let status_needles = [
        "update_in_tx(",
        "update_status_and_place_in_tx(",
        "set_status_in_tx(",
        "update_full_in_tx(",
    ];
    for (n, f, _) in REQUIRED_CALLS {
        if status_needles.contains(&n) {
            assert!(
                event_files.contains(&f),
                "{f}: писатель {n} без события статуса в реестре"
            );
        }
    }
}

#[test]
fn history_write_sites_order_movement_before_event() {
    let sources = prod_sources();
    let mut total_pairs = 0usize;
    for (rel, code) in &sources {
        if rel.starts_with("crates/trackly-app/src/") {
            let v = order_violations(code);
            assert!(v.is_empty(), "{rel}: {v:?}");
        }
    }
    for f in ORDER_FILES {
        total_pairs += pair_fns(raw_of(f)).len();
    }
    assert!(
        total_pairs >= 3,
        "в ORDER_FILES нашлось только {total_pairs} функций с парой — проверка порядка вакуумна"
    );
}

/// D-24 / MB-8: `DELETE FROM history_events` живёт только в репозитории истории, а запись
/// в `group_devices` в production-коде — только в `groups_sqlite.rs`.
#[test]
fn history_write_sites_raw_sql_stays_in_repositories() {
    let sources = prod_sources();
    for (rel, code) in &sources {
        if rel != "crates/trackly-infra/src/repos/history_events_sqlite.rs"
            && code.contains("DELETE FROM history_events")
        {
            panic!("{rel}: DELETE FROM history_events вне репозитория истории (D-24)");
        }
        if rel != GROUPS_REPO
            && [
                "INSERT INTO group_devices",
                "UPDATE group_devices",
                "DELETE FROM group_devices",
            ]
            .iter()
            .any(|m| code.contains(m))
        {
            panic!("{rel}: прямая запись в group_devices вне groups_sqlite.rs (MB-8)");
        }
        // статус устройства только через репозиторий устройств
        if !rel.starts_with(REPOS_PREFIX)
            && rel.starts_with("crates/")
            && STATUS_SQL_MARKERS.iter().any(|m| {
                code.match_indices(m).any(|(p, _)| {
                    code[p..]
                        .split('"')
                        .next()
                        .unwrap_or("")
                        .contains("status_id")
                })
            })
        {
            panic!("{rel}: прямая запись devices.status_id вне репозитория");
        }
    }
}

// ---------------------------------------------------------------------------
// selftest счётчика, сканера и порядка (синтетика)
// ---------------------------------------------------------------------------

#[test]
fn history_write_sites_counter_selftest() {
    let fixture = "\
let a = repo.update_full_in_tx(tx, 1);
// repo.update_full_in_tx(tx, 2);
pub fn update_full_in_tx(
let b = foo_update_full_in_tx(tx, 3);
let c = repo.update_full_in_tx(tx, 4); // хвост update_full_in_tx(
";
    let code = production_code(fixture);
    assert_eq!(count_calls(&code, &["update_full_in_tx("]), 2);
    // пропуск и лишний вызов краснят реестр
    let src = vec![("f.rs".to_string(), code.clone())];
    assert!(
        registry_violations(&src, &["update_full_in_tx("], &[("f.rs", 2)], None, "x").is_empty()
    );
    assert!(
        !registry_violations(&src, &["update_full_in_tx("], &[("f.rs", 3)], None, "x").is_empty()
    );
    assert!(
        !registry_violations(&src, &["update_full_in_tx("], &[("f.rs", 1)], None, "x").is_empty()
    );
    assert!(!registry_violations(&src, &["update_full_in_tx("], &[], None, "x").is_empty());
    assert!(
        !registry_violations(&[], &["update_full_in_tx("], &[("f.rs", 2)], None, "x").is_empty()
    );
}

#[test]
fn history_write_sites_production_code_selftest() {
    let fixture = "\
repo.update_in_tx(a);
#[cfg(test)]
mod tests {
    repo.update_in_tx(b);
}
repo.update_in_tx(c);
#[cfg(test)]
fn one_liner() {}
repo.update_in_tx(d);
";
    let code = production_code(fixture);
    assert_eq!(
        count_calls(&code, &["update_in_tx("]),
        3,
        "cfg(test)-блок вырезан, код после него сохранён"
    );
    assert_eq!(code.lines().count(), fixture.lines().count());
}

#[test]
fn history_write_sites_scanner_selftest() {
    let repo = "\
pub fn good(&self) { conn.execute(\"UPDATE devices SET status_id = ?1\", p); }
pub fn other(&self) { conn.execute(\"UPDATE devices SET name = ?1 WHERE status_id = 2\", p); }
pub fn sneaky(&self) {
    conn.execute(
        \"UPDATE devices SET status_id = ?1\", p);
}
";
    let code = production_code(repo);
    let found = writing_methods(&code, &STATUS_SQL_MARKERS, Some("status_id"));
    assert!(found.contains(&"good".to_string()) && found.contains(&"sneaky".to_string()));
    // WHERE status_id тоже упоминает слово в SQL — консервативно требует вердикта
    let registry = [("r.rs", "good", Verdict::EventRequired)];
    let (new, gone) = writer_diff(
        &code,
        "r.rs",
        &STATUS_SQL_MARKERS,
        Some("status_id"),
        &registry,
    );
    assert!(
        new.contains(&"sneaky".to_string()),
        "новый писатель без вердикта должен быть найден"
    );
    assert!(gone.is_empty());
    let (_, gone) = writer_diff(
        "fn x() {}",
        "r.rs",
        &STATUS_SQL_MARKERS,
        Some("status_id"),
        &registry,
    );
    assert_eq!(gone, vec!["good".to_string()], "исчезнувший метод замечен");
}

#[test]
fn history_write_sites_order_selftest() {
    let mv = "record_movement_if_applicable(";
    let ev = "record_status_change_in_tx(";
    let ok = format!("fn a() {{\n {mv}x);\n {ev}y);\n}}\n");
    let swapped = format!("fn a() {{\n {ev}y);\n {mv}x);\n}}\n");
    let two_ok = format!("fn a() {{\n {mv});\n {ev});\n {mv});\n {ev});\n}}\n");
    let two_bad = format!("fn a() {{\n {mv});\n {ev});\n {ev});\n {mv});\n}}\n");
    let no_move = format!("fn a() {{\n {ev});\n}}\n");
    assert!(order_violations(&ok).is_empty());
    assert!(!order_violations(&swapped).is_empty());
    assert!(order_violations(&two_ok).is_empty());
    assert!(!order_violations(&two_bad).is_empty());
    assert!(
        order_violations(&no_move).is_empty(),
        "без перемещения порядок не проверяется"
    );
    // перестановка не меняет число вызовов — счёт слеп, порядок несущий
    assert_eq!(
        count_calls(&ok, &[mv, ev]),
        count_calls(&swapped, &[mv, ev])
    );
}

// ---------------------------------------------------------------------------
// Мутации реального исходника (в памяти; файлы на диске не меняются)
// ---------------------------------------------------------------------------

fn with_source(sources: &[(String, String)], rel: &str, new_raw: &str) -> Vec<(String, String)> {
    sources
        .iter()
        .map(|(r, t)| {
            if r == rel {
                (r.clone(), production_code(new_raw))
            } else {
                (r.clone(), t.clone())
            }
        })
        .collect()
}

/// Заменяет n-е (с нуля) вхождение needle в КОДЕ (не в комментарии, не определение) на `noop(`.
/// Возвращает None, если такого вхождения нет.
fn remove_nth_call(raw: &str, needle: &str, nth: usize) -> Option<String> {
    let code = production_code(raw);
    let code_lines: Vec<&str> = code.lines().collect();
    let mut raw_lines: Vec<String> = raw.lines().map(String::from).collect();
    let mut seen = 0usize;
    for (i, cl) in code_lines.iter().enumerate() {
        if cl.contains("fn ") {
            continue;
        }
        for (p, _) in cl.match_indices(needle) {
            if !ident_boundary_before(cl, p) {
                continue;
            }
            if seen == nth {
                // production_code сохраняет нумерацию и колонки до `//` — позиция совпадает с raw
                raw_lines[i].replace_range(p..p + needle.len(), "noop_removed(");
                return Some(raw_lines.join("\n"));
            }
            seen += 1;
        }
    }
    None
}

/// Снятие ЛЮБОГО одного вызова из реестров краснит гейт; число после мутации ровно на 1 меньше.
#[test]
fn history_write_sites_mutation_removed_call_turns_red() {
    let sources = prod_sources();
    let mut checked = 0usize;
    // писатели
    for needle in all_writer_needles() {
        for (n, f, c) in REQUIRED_CALLS
            .iter()
            .map(|(n, f, c)| (*n, *f, *c))
            .chain(EXEMPT_CALLS.iter().map(|(n, f, c, _)| (*n, *f, *c)))
            .chain(NEUTRAL_CALLS.iter().map(|(n, f, c, _)| (*n, *f, *c)))
            .filter(|(n, _, _)| *n == needle)
        {
            let raw = raw_of(f);
            let before = count_calls(&production_code(raw), &[n]);
            assert!(before >= c, "{f}: {n}");
            let mutated = remove_nth_call(raw, n, 0)
                .unwrap_or_else(|| panic!("{f}: вызов {n} не найден для мутации"));
            let after = count_calls(&production_code(&mutated), &[n]);
            assert_eq!(
                after + 1,
                before,
                "{f}: мутация {n} сняла не ровно один вызов"
            );
            let v = writer_call_violations(&with_source(&sources, f, &mutated));
            assert!(!v.is_empty(), "{f}: снятый вызов {n} не покраснил гейт");
            checked += 1;
        }
    }
    // примитивы событий
    let mut event_rows: Vec<(&str, &str)> = Vec::new();
    event_rows.extend(
        STATUS_EVENT_REGISTRY
            .iter()
            .map(|(f, _)| (STATUS_EVENT_NEEDLE, *f)),
    );
    event_rows.extend(
        ACT_WRAPPER_REGISTRY
            .iter()
            .map(|(f, _)| (ACT_WRAPPER_NEEDLE, *f)),
    );
    event_rows.extend(
        UNNESTED_WRAPPER_REGISTRY
            .iter()
            .map(|(f, _)| (UNNESTED_WRAPPER_NEEDLE, *f)),
    );
    for (needle, f) in event_rows {
        let raw = raw_of(f);
        let mutated = remove_nth_call(raw, needle, 0)
            .unwrap_or_else(|| panic!("{f}: вызов {needle} не найден для мутации"));
        let v = event_call_violations(&with_source(&sources, f, &mutated));
        assert!(
            !v.is_empty(),
            "{f}: снятый вызов {needle} не покраснил гейт"
        );
        checked += 1;
    }
    for (f, _) in GROUP_EVENT_REGISTRY {
        let raw = raw_of(f);
        let first = GROUP_EVENT_NEEDLES
            .iter()
            .find(|n| count_calls(&production_code(raw), &[**n]) > 0)
            .expect("в файле реестра есть группа-событие");
        let mutated = remove_nth_call(raw, first, 0).expect("вызов для мутации");
        let v = event_call_violations(&with_source(&sources, f, &mutated));
        assert!(!v.is_empty(), "{f}: снятый вызов {first} не покраснил гейт");
        checked += 1;
    }
    assert!(checked >= 25, "мутаций проверено слишком мало: {checked}");
}

/// Лишний (новый) вызов писателя без вердикта краснит гейт, а лишний метод репозитория —
/// инвентарь.
#[test]
fn history_write_sites_mutation_new_writer_turns_red() {
    let sources = prod_sources();
    let f = "crates/trackly-app/src/services/place_service.rs";
    let injected = format!("{}\nrepo.set_status_in_tx(&tx, 1, 2, 3);\n", raw_of(f));
    let v = writer_call_violations(&with_source(&sources, f, &injected));
    assert!(
        !v.is_empty(),
        "новый вызов писателя статуса не покраснил гейт"
    );

    let injected_repo = format!(
        "{}\nimpl X {{ pub fn sneaky_in_tx(&self) {{ tx.execute(\"UPDATE devices SET status_id = 1\", []); }} }}\n",
        raw_of(DEVICES_REPO)
    );
    let (new, _) = writer_diff(
        &production_code(&injected_repo),
        DEVICES_REPO,
        &STATUS_SQL_MARKERS,
        Some("status_id"),
        &STATUS_WRITER_METHODS,
    );
    assert!(new.contains(&"sneaky_in_tx".to_string()));

    let injected_groups = format!(
        "{}\nimpl X {{ pub fn sneaky_member(&self) {{ tx.execute(\"INSERT INTO group_devices (a) VALUES (1)\", []); }} }}\n",
        raw_of(GROUPS_REPO)
    );
    let (new, _) = writer_diff(
        &production_code(&injected_groups),
        GROUPS_REPO,
        &MEMBERSHIP_SQL_MARKERS,
        None,
        &MEMBERSHIP_WRITER_METHODS,
    );
    assert!(new.contains(&"sneaky_member".to_string()));
}

/// D-25: мутация-ПЕРЕСТАНОВКА. Для каждой функции с парой «перемещение+событие» меняем
/// местами первое перемещение и первое событие. Число вызовов не меняется (счётный реестр
/// зелёный), а проверка порядка краснеет — именно она несущая.
#[test]
fn history_write_sites_order_mutation_swap_turns_order_red_not_count() {
    let sources = prod_sources();
    let mut swapped_fns = 0usize;
    for f in ORDER_FILES {
        let raw = raw_of(f);
        let names = pair_fns(raw);
        assert!(!names.is_empty(), "{f}: нет функций с парой для мутации");
        for name in names {
            // якорь уникален: имя функции должно встречаться как определение ровно один раз
            let defs = raw.matches(&format!("fn {name}(")).count()
                + raw.matches(&format!("fn {name}<")).count();
            if defs != 1 {
                continue; // перегрузки по имени в одном файле не мутируем (неоднозначно)
            }
            let Some(mutated) = swap_first_pair(raw, &name) else {
                continue;
            };
            // 1) счёт не изменился
            let mutated_sources = with_source(&sources, f, &mutated);
            assert!(
                writer_call_violations(&mutated_sources).is_empty()
                    && event_call_violations(&mutated_sources).is_empty(),
                "{f}::{name}: перестановка не должна менять счётные реестры"
            );
            // 2) порядок краснеет
            let v = order_violations(&production_code(&mutated));
            assert!(
                v.iter().any(|m| m.starts_with(&format!("{name}:"))),
                "{f}::{name}: перестановка не покраснила проверку порядка: {v:?}"
            );
            swapped_fns += 1;
        }
    }
    assert!(
        swapped_fns >= 3,
        "перестановка проверена на {swapped_fns} функциях — мало"
    );
}

#[test]
fn history_write_sites_mutation_anchor_is_unique() {
    // единичные вызовы, на которых строится мутация перестановки, уникальны внутри функции
    let raw = raw_of("crates/trackly-app/src/services/device_service.rs");
    let code = production_code(raw);
    let lines: Vec<&str> = code.lines().collect();
    let hits: Vec<_> = fn_ranges(&lines)
        .into_iter()
        .filter(|(_, s, e)| {
            lines[*s..*e]
                .iter()
                .map(|l| line_calls(l, &MOVEMENT_NEEDLES))
                .sum::<usize>()
                > 0
                && lines[*s..*e]
                    .iter()
                    .map(|l| line_calls(l, &ORDER_EVENT_NEEDLES))
                    .sum::<usize>()
                    > 0
        })
        .collect();
    assert_eq!(hits.len(), 1, "device_service: ровно одна функция с парой");
    let (_, s, e) = &hits[0];
    assert_eq!(
        lines[*s..*e]
            .iter()
            .map(|l| line_calls(l, &MOVEMENT_NEEDLES))
            .sum::<usize>(),
        1
    );
    assert_eq!(
        lines[*s..*e]
            .iter()
            .map(|l| line_calls(l, &ORDER_EVENT_NEEDLES))
            .sum::<usize>(),
        1
    );
}
