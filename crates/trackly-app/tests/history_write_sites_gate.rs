//! Phase 41.10 (HST-06, план 13): реестровый гейт мест записи статуса и состава.
//!
//! Инвентарь снимается ОТ РЕПОЗИТОРИЯ (методы `crates/trackly-infra/src/repos/*.rs`,
//! пишущие `devices.status_id` и состав групп), а не от списка из плана: метод без
//! вердикта краснит гейт. Вызовы писателей в сервисах закреплены числом по файлам и
//! сверены с числом вызовов записывающих историю примитивов; освобождения
//! (рождение устройства, restore отката) кодированы вердиктами с причиной.
//!
//! Ревью 41.10 WR-03: счёт по файлам не различает путь вызова. Тот же restore-код идёт и
//! из отката акта (D-24 (b), событий нет), и из правки живого акта (D-24 (a), события
//! обычные). Поэтому «тихие» режимы истории (`ReleaseHistory::Suppressed`,
//! `RevertHistory::Undo`) и освобождения отката привязаны к ИМЕНИ функции
//! (`SILENT_MODE_TOKENS`, `UNDO_SCOPED_EXEMPTS`), а функции отката — к списку вызывающих
//! (`UNDO_ONLY_FNS`). Тихий режим на пути правки или вызов отката из правки краснит гейт.
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
    // правка живого акта пишет статус событием (record_act_status_event); вызов отката
    // освобождён ТОЛЬКО внутри undo_device_mutations_for_act (`UNDO_SCOPED_EXEMPTS`).
    (
        DEVICES_REPO,
        "restore_from_snapshot_in_tx",
        Verdict::EventRequired,
    ),
    (DEVICES_REPO, "create", Verdict::NoProductionCallers),
    (DEVICES_REPO, "update", Verdict::NoProductionCallers),
];

/// Методы репозитория групп, пишущие состав (`group_devices`, строки/родитель/якорь `groups`).
/// `EventRequired` = событие пишет сервисный слой (см. реестры вызовов). Откат акта, где
/// событий нет (D-24 (b)), отличается от правки режимом истории, а не файлом: см.
/// `SILENT_MODE_TOKENS` и `UNDO_ONLY_FNS`.
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
const REQUIRED_CALLS: [(&str, &str, usize); 14] = [
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
    // правка живого акта (update, update_return): статус снятой позиции пишет
    // record_act_status_event после перемещения (D-24 (a), D-25)
    (
        "restore_from_snapshot_in_tx(",
        "crates/trackly-app/src/services/act_service.rs",
        2,
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
    // restore_membership_with_policy_in_tx: RevertHistory::Edit пишет group_joined (CR-01);
    // Undo разрешён только в функциях отката (`SILENT_MODE_TOKENS`)
    (
        "add_device_in_tx(",
        "crates/trackly-app/src/services/group_membership.rs",
        1,
    ),
    // 3 отцепления актом (record_act_unnested) + 1 возврат вложенности:
    // RevertHistory::Edit пишет group_nested (CR-02), Undo — только в откате
    (
        "set_parent_in_tx(",
        "crates/trackly-app/src/services/group_act_plan.rs",
        4,
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
const EXEMPT_CALLS: [(&str, &str, usize, &str); 4] = [
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
        1,
        "откат/удаление акта (D-24 (b)): события акта удаляются по act_id, новых нет",
    ),
    (
        "update_status_and_place_in_tx(",
        "crates/trackly-app/src/services/place_service.rs",
        1,
        "статус не меняется: передаётся before.status_id (ST-11), смены статуса нет (WR-02)",
    ),
];

/// Освобождения «статус не меняется» (WR-02): (needle, файл, функция, число, закреплённый
/// аргумент). Вызов освобождён, только пока в его аргументах стоит `pin`: настоящая смена
/// статуса (другой аргумент) снимает освобождение и краснит гейт, а не проходит молча.
const PINNED_EXEMPTS: [(&str, &str, &str, usize, &str); 1] = [(
    "update_status_and_place_in_tx(",
    "crates/trackly-app/src/services/place_service.rs",
    "move_subtree_contents",
    1,
    "before.status_id",
)];

/// Освобождения, привязанные к ИМЕНИ функции (WR-03): (needle, файл, функция, число).
/// Каждая строка `EXEMPT_CALLS` с методом не-`BirthExempt` обязана быть здесь, а функция —
/// в `UNDO_ONLY_FNS` (её вызывающие проверены). Вызов в другой функции того же файла
/// (например, в правке акта) освобождения не получает.
const UNDO_SCOPED_EXEMPTS: [(&str, &str, &str, usize); 1] = [(
    "restore_from_snapshot_in_tx(",
    "crates/trackly-app/src/services/act_service.rs",
    "undo_device_mutations_for_act",
    1,
)];

const ACT_SERVICE: &str = "crates/trackly-app/src/services/act_service.rs";
const GROUP_ACT_PLAN: &str = "crates/trackly-app/src/services/group_act_plan.rs";
const GROUP_MEMBERSHIP: &str = "crates/trackly-app/src/services/group_membership.rs";
const GROUP_SERVICE: &str = "crates/trackly-app/src/services/group_service.rs";

/// «Тихие» режимы истории (WR-03): (токен, файл, функция, число, причина >= 40 символов).
/// Инвентарь снимается от исходника: токен в функции без строки — красный гейт. Правка живого
/// акта обязана писать события (D-24 (a)), поэтому её функций здесь нет и быть не может.
const SILENT_MODE_TOKENS: [(&str, &str, &str, usize, &str); 5] = [
    (
        "ReleaseHistory::Suppressed",
        GROUP_MEMBERSHIP,
        "release_inner",
        1,
        "определение: ветка «тихого» режима внутри тела вывода, а не передача режима",
    ),
    (
        "ReleaseHistory::Suppressed",
        GROUP_SERVICE,
        "move_devices",
        1,
        "перенос из группы в группу (D-05): одно событие group_transferred пишет сам перенос",
    ),
    (
        "ReleaseHistory::Suppressed",
        ACT_SERVICE,
        "undo_device_mutations_for_act",
        1,
        "откат/удаление акта (D-24 (b)): события акта удаляются по act_id, новых нет",
    ),
    (
        "RevertHistory::Undo",
        GROUP_ACT_PLAN,
        "revert_group_moves_in_tx",
        1,
        "откат/удаление акта (D-24 (b)): возврат вложенности без события, события акта удалены",
    ),
    (
        "RevertHistory::Undo",
        GROUP_ACT_PLAN,
        "restore_released_members_in_tx",
        1,
        "откат/удаление акта (D-24 (b)): возврат в состав без события, события акта удалены",
    ),
];

/// Функции пути отката и единственные допустимые вызывающие (WR-03). Вызов функции отката
/// из правки акта (или откуда-либо ещё) краснит гейт: там тихий режим стал бы дырой ленты.
const UNDO_ONLY_FNS: [(&str, &[&str]); 3] = [
    (
        "undo_device_mutations_for_act",
        &["delete_soft_with_choices"],
    ),
    (
        "revert_group_moves_in_tx",
        &["undo_device_mutations_for_act"],
    ),
    (
        "restore_released_members_in_tx",
        &["undo_device_mutations_for_act"],
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
const STATUS_EVENT_REGISTRY: [(&str, usize); 5] = [
    ("crates/trackly-app/src/services/act_service.rs", 1),
    ("crates/trackly-app/src/services/device_service.rs", 1),
    ("crates/trackly-app/src/services/group_place.rs", 1),
    ("crates/trackly-app/src/services/group_service.rs", 3),
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
    // unnested (record_act_unnested) + nested (возврат вложенности при правке акта, CR-02)
    ("crates/trackly-app/src/services/group_act_plan.rs", 2),
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
// WR-03: путь вызова — откат или правка (по имени функции, не по файлу)
// ---------------------------------------------------------------------------

const APP_SRC_PREFIX: &str = "crates/trackly-app/src/";

/// Имя функции, в которую попадает строка `idx` (по `fn_ranges`).
fn enclosing_fn(ranges: &[(String, usize, usize)], idx: usize) -> Option<&str> {
    ranges
        .iter()
        .find(|(_, s, e)| *s <= idx && idx < *e)
        .map(|(n, _, _)| n.as_str())
}

/// (файл, функция) -> число вхождений `needle` (с границей идентификатора, без строк `fn `).
fn occurrences_by_fn(code_src: &str, needle: &str) -> Vec<(String, usize)> {
    let lines: Vec<&str> = code_src.lines().collect();
    let ranges = fn_ranges(&lines);
    let mut out: Vec<(String, usize)> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if l.contains("fn ") {
            continue;
        }
        let n = line_calls(l, &[needle]);
        if n == 0 {
            continue;
        }
        let f = enclosing_fn(&ranges, i)
            .unwrap_or("<вне функции>")
            .to_string();
        match out.iter_mut().find(|(name, _)| *name == f) {
            Some((_, c)) => *c += n,
            None => out.push((f, n)),
        }
    }
    out
}

/// Нарушения инвентаря «тихих» режимов истории: токен в функции без строки реестра, число
/// разошлось, строка реестра без токена в исходнике.
fn silent_mode_violations(sources: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    let mut tokens: Vec<&str> = SILENT_MODE_TOKENS.iter().map(|(t, ..)| *t).collect();
    tokens.sort();
    tokens.dedup();
    for token in tokens {
        let mut seen: Vec<(String, String, usize)> = Vec::new();
        for (rel, code) in sources
            .iter()
            .filter(|(r, _)| r.starts_with(APP_SRC_PREFIX))
        {
            for (f, n) in occurrences_by_fn(code, token) {
                seen.push((rel.clone(), f, n));
            }
        }
        for (rel, f, n) in &seen {
            match SILENT_MODE_TOKENS
                .iter()
                .find(|(t, rf, rfn, _, _)| *t == token && rf == rel && rfn == f)
            {
                None => out.push(format!(
                    "тихий режим `{token}` в {rel}::{f} ({n}) без вердикта: правка акта обязана писать события (D-24 (a))"
                )),
                Some((_, _, _, want, _)) if want != n => out.push(format!(
                    "число `{token}` в {rel}::{f} изменилось ({n}, в реестре {want})"
                )),
                Some(_) => {}
            }
        }
        for (t, rf, rfn, want, _) in SILENT_MODE_TOKENS.iter().filter(|(t, ..)| *t == token) {
            if !seen.iter().any(|(r, f, _)| r == rf && f == rfn) {
                out.push(format!(
                    "строка реестра `{t}` в {rf}::{rfn} ({want}) больше не встречается"
                ));
            }
        }
    }
    out
}

/// Нарушения списка вызывающих функций отката; пустой набор вызывающих — тоже нарушение
/// (иначе проверка вакуумна).
fn undo_caller_violations(sources: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (callee, allowed) in UNDO_ONLY_FNS {
        let needle = format!("{callee}(");
        let mut callers = 0usize;
        for (rel, code) in sources
            .iter()
            .filter(|(r, _)| r.starts_with(APP_SRC_PREFIX))
        {
            for (f, n) in occurrences_by_fn(code, &needle) {
                callers += n;
                if !allowed.contains(&f.as_str()) {
                    out.push(format!(
                        "функция отката {callee} вызвана из {rel}::{f}: там тихий режим истории скрыл бы событие (D-24)"
                    ));
                }
            }
        }
        if callers == 0 {
            out.push(format!(
                "у функции отката {callee} нет вызывающих: строка реестра устарела"
            ));
        }
    }
    out
}

/// Текст аргументов каждого вызова `needle` внутри функции `func` (от вызова до первой
/// строки с `)?` или `);`), по production-коду.
fn call_args_in_fn(code_src: &str, needle: &str, func: &str) -> Vec<String> {
    let lines: Vec<&str> = code_src.lines().collect();
    let ranges = fn_ranges(&lines);
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if l.contains("fn ") || line_calls(l, &[needle]) == 0 {
            continue;
        }
        if enclosing_fn(&ranges, i) != Some(func) {
            continue;
        }
        let mut args = String::new();
        for tail in &lines[i..] {
            args.push_str(tail);
            args.push('\n');
            if tail.contains(")?") || tail.contains(");") {
                break;
            }
        }
        out.push(args);
    }
    out
}

/// Нарушения закреплённых освобождений (WR-02): число вызовов в функции и аргумент-якорь.
fn pinned_exempt_violations(sources: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (needle, file, func, want, pin) in PINNED_EXEMPTS {
        let Some((_, code)) = sources.iter().find(|(r, _)| r == file) else {
            out.push(format!("{file}: нет в дереве"));
            continue;
        };
        let calls = call_args_in_fn(code, needle, func);
        if calls.len() != want {
            out.push(format!(
                "{file}::{func}: вызовов {needle} {}, в реестре {want}",
                calls.len()
            ));
        }
        for args in calls {
            if !args.contains(pin) {
                out.push(format!(
                    "{file}::{func}: {needle} без `{pin}` — статус меняется, нужно событие (D-06)"
                ));
            }
        }
    }
    out
}

/// Нарушения освобождений, привязанных к функции: вызов освобождён только внутри
/// указанной функции, а она — функция отката.
fn scoped_exempt_violations(sources: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (needle, file, func, want) in UNDO_SCOPED_EXEMPTS {
        let Some((_, code)) = sources.iter().find(|(r, _)| r == file) else {
            out.push(format!("{file}: нет в дереве"));
            continue;
        };
        let got = occurrences_by_fn(code, needle)
            .into_iter()
            .find(|(f, _)| f == func)
            .map_or(0, |(_, n)| n);
        if got != want {
            out.push(format!(
                "{file}::{func}: освобождённых вызовов {needle} {got}, в реестре {want}"
            ));
        }
        if !UNDO_ONLY_FNS.iter().any(|(f, _)| *f == func) {
            out.push(format!("{func}: освобождение отката вне функции отката"));
        }
    }
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

/// Ревью WR-04 (D-25): события вложения, сопровождающие перенос места группы, пишутся ПОСЛЕ
/// него и несут `movement_id` строки группы. (файл, функция, перенос, события).
const NESTING_ORDER_SITES: [(&str, &str, &str, [&str; 2]); 1] = [(
    "crates/trackly-app/src/services/group_service.rs",
    "set_parent",
    "propagate_group_place_in_tx(",
    ["record_group_nested_in_tx(", "record_group_unnested_in_tx("],
)];

/// Нарушения порядка «перенос места группы -> событие вложения» на заданных исходниках.
fn nesting_order_violations(sources: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (file, func, mv, events) in NESTING_ORDER_SITES {
        let Some((_, code)) = sources.iter().find(|(r, _)| r == file) else {
            out.push(format!("{file}: нет в дереве"));
            continue;
        };
        let lines: Vec<&str> = code.lines().collect();
        let Some((_, s, e)) = fn_ranges(&lines).into_iter().find(|(n, _, _)| n == func) else {
            out.push(format!("{file}: функции {func} нет"));
            continue;
        };
        let first = |needles: &[&str]| (s..e).find(|i| line_calls(lines[*i], needles) > 0);
        match (first(&[mv]), first(&events)) {
            (Some(m), Some(ev)) if m < ev => {}
            (Some(_), Some(ev)) => out.push(format!(
                "{func}: событие вложения (строка {}) раньше переноса места (D-25)",
                ev + 1
            )),
            _ => out.push(format!(
                "{func}: нет переноса или события вложения — проверка вакуумна"
            )),
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
/// BirthExempt — в `EXEMPT_CALLS` (вердикт кодирован числом, не комментарием).
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
            Verdict::BirthExempt => assert!(
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

// ---------------------------------------------------------------------------
// WR-03: правка живого акта против отката (D-24 (a)/(b))
// ---------------------------------------------------------------------------

#[test]
fn history_write_sites_silent_modes_only_on_undo_paths() {
    let sources = prod_sources();
    let v = silent_mode_violations(&sources);
    assert!(v.is_empty(), "тихие режимы истории: {v:#?}");
    let v = undo_caller_violations(&sources);
    assert!(v.is_empty(), "вызывающие функций отката: {v:#?}");
    let v = scoped_exempt_violations(&sources);
    assert!(v.is_empty(), "освобождения отката: {v:#?}");
    for (t, f, func, _, reason) in SILENT_MODE_TOKENS {
        assert!(
            reason.chars().count() >= 40,
            "причина `{t}` в {f}::{func} короче 40 символов"
        );
    }
    // каждое освобождение не-рождения привязано к функции отката
    for (n, f, c, _) in EXEMPT_CALLS {
        let birth = STATUS_WRITER_METHODS
            .iter()
            .chain(MEMBERSHIP_WRITER_METHODS.iter())
            .any(|(_, m, v)| *v == Verdict::BirthExempt && n.trim_end_matches('(').ends_with(m));
        if birth {
            continue;
        }
        let scoped: usize = UNDO_SCOPED_EXEMPTS
            .iter()
            .filter(|(sn, sf, _, _)| *sn == n && *sf == f)
            .map(|(_, _, _, sc)| *sc)
            .sum::<usize>()
            + PINNED_EXEMPTS
                .iter()
                .filter(|(pn, pf, _, _, _)| *pn == n && *pf == f)
                .map(|(_, _, _, pc, _)| *pc)
                .sum::<usize>();
        assert_eq!(
            scoped, c,
            "{n} в {f}: освобождение без привязки к функции отката или аргументу (WR-03, WR-02)"
        );
    }
}

/// Мутации реального исходника: тихий режим на пути правки, вызов отката из правки и
/// снятая запись события правки краснят гейт.
#[test]
fn history_write_sites_mutation_edit_path_silence_turns_red() {
    let sources = prod_sources();
    let act = raw_of(ACT_SERVICE);
    // (1) правка акта выводит устройство «тихо»
    let m1 = act.replacen("ReleaseHistory::Default,", "ReleaseHistory::Suppressed,", 1);
    assert_ne!(
        m1, act,
        "якорь ReleaseHistory::Default в act_service найден"
    );
    let v = silent_mode_violations(&with_source(&sources, ACT_SERVICE, &m1));
    assert!(
        v.iter().any(|m| m.contains("::update ")),
        "тихий вывод в правке акта не покраснил гейт: {v:?}"
    );
    // (2) правка акта возвращает вложенность без события
    let m2 = act.replacen("RevertHistory::Edit,", "RevertHistory::Undo,", 1);
    assert_ne!(m2, act, "якорь RevertHistory::Edit в act_service найден");
    assert!(
        !silent_mode_violations(&with_source(&sources, ACT_SERVICE, &m2)).is_empty(),
        "тихий возврат вложенности в правке акта не покраснил гейт"
    );
    // (3) правка возврата возвращает в состав без события
    let gap = raw_of(GROUP_ACT_PLAN);
    let m3 = gap.replacen("RevertHistory::Edit,", "RevertHistory::Undo,", 1);
    assert_ne!(m3, gap, "якорь RevertHistory::Edit в group_act_plan найден");
    assert!(
        !silent_mode_violations(&with_source(&sources, GROUP_ACT_PLAN, &m3)).is_empty(),
        "тихий возврат в состав при правке возврата не покраснил гейт"
    );
    // (4) правка акта зовёт функцию отката
    let m4 = act.replacen(
        "let back = revert_detach_effect_in_tx(",
        "let back = revert_group_moves_in_tx(",
        1,
    );
    assert_ne!(m4, act, "якорь вызова revert_detach_effect_in_tx найден");
    assert!(
        !undo_caller_violations(&with_source(&sources, ACT_SERVICE, &m4)).is_empty(),
        "вызов функции отката из правки не покраснил гейт"
    );
    // (5) освобождённый вызов restore отката переехал в правку: счёт по файлу тот же,
    // привязка к функции краснеет
    let m5 = act.replacen(
        "devices_repo.restore_from_snapshot_in_tx(tx, device_id, &snapshot, now)?",
        "devices_repo.noop_removed(tx, device_id, &snapshot, now)?",
        1,
    );
    assert_ne!(m5, act, "якорь restore в undo найден");
    let m5 = m5.replacen(
        "let restored = devices_repo\n",
        "let _x = devices_repo.restore_from_snapshot_in_tx(&tx, removed_id, &snapshot, now)?;\n                            let restored = devices_repo\n",
        1,
    );
    let ms = with_source(&sources, ACT_SERVICE, &m5);
    assert!(
        writer_call_violations(&ms).is_empty(),
        "счёт по файлу не изменился — счётный гейт слеп (поэтому нужна привязка)"
    );
    assert!(
        !scoped_exempt_violations(&ms).is_empty(),
        "освобождение отката, уехавшее в правку, не покраснило гейт"
    );
    // (6) снятая запись событий правки (CR-01 group_joined, CR-02 group_nested)
    for (f, needle) in [
        (GROUP_MEMBERSHIP, "record_group_joined_in_tx("),
        (GROUP_ACT_PLAN, "record_group_nested_in_tx("),
    ] {
        let m = remove_nth_call(raw_of(f), needle, 0).expect("вызов события правки найден");
        assert!(
            !event_call_violations(&with_source(&sources, f, &m)).is_empty(),
            "{f}: снятый {needle} не покраснил гейт"
        );
    }
}

/// WR-02: «статус не меняется» закреплён аргументом, а не мёртвым вызовом события.
#[test]
fn history_write_sites_pinned_exempt_status_unchanged() {
    let sources = prod_sources();
    let v = pinned_exempt_violations(&sources);
    assert!(v.is_empty(), "закреплённые освобождения: {v:#?}");
    // мутация: в «Местах» появилась настоящая смена статуса — освобождение снимается
    let f = "crates/trackly-app/src/services/place_service.rs";
    let raw = raw_of(f);
    let m = raw.replacen(
        "item.id,\n                                before.status_id,\n                                Some(target_place_id),",
        "item.id,\n                                new_status_id,\n                                Some(target_place_id),",
        1,
    );
    assert_ne!(m, raw, "якорь аргумента статуса в place_service найден");
    assert!(
        !pinned_exempt_violations(&with_source(&sources, f, &m)).is_empty(),
        "настоящая смена статуса в «Местах» не покраснила гейт"
    );
}

/// WR-04: событие вложения после переноса места; перестановка блоков краснит проверку,
/// а счётные реестры не видят её.
#[test]
fn history_write_sites_nesting_event_after_group_move() {
    let sources = prod_sources();
    let v = nesting_order_violations(&sources);
    assert!(v.is_empty(), "порядок вложения: {v:#?}");
    let f = "crates/trackly-app/src/services/group_service.rs";
    let raw = raw_of(f);
    // мутация-перестановка: первое событие вложения и перенос места меняются именами
    let code = production_code(raw);
    let lines: Vec<&str> = code.lines().collect();
    let (_, s, e) = fn_ranges(&lines)
        .into_iter()
        .find(|(n, _, _)| n == "set_parent")
        .expect("set_parent");
    let mi = (s..e)
        .find(|i| line_calls(lines[*i], &["propagate_group_place_in_tx("]) > 0)
        .expect("перенос");
    let ei = (s..e)
        .find(|i| line_calls(lines[*i], &["record_group_unnested_in_tx("]) > 0)
        .expect("событие");
    let mut raw_lines: Vec<String> = raw.lines().map(String::from).collect();
    raw_lines[mi] = raw_lines[mi].replacen("propagate_group_place_in_tx(", "__SWAP__(", 1);
    raw_lines[ei] = raw_lines[ei].replacen(
        "record_group_unnested_in_tx(",
        "propagate_group_place_in_tx(",
        1,
    );
    raw_lines[mi] = raw_lines[mi].replacen("__SWAP__(", "record_group_unnested_in_tx(", 1);
    let mutated = raw_lines.join("\n");
    let ms = with_source(&sources, f, &mutated);
    assert!(
        writer_call_violations(&ms).is_empty() && event_call_violations(&ms).is_empty(),
        "перестановка не меняет счёт — счётный гейт слеп к порядку"
    );
    assert!(
        !nesting_order_violations(&ms).is_empty(),
        "событие вложения раньше переноса места не покраснило проверку"
    );
}
