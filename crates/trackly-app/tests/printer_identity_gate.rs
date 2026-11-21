//! Гейт quick-261121-heh (INT-31, NEW-5): признак «это принтер» определён в
//! бэкенде ОДИН раз — `PRINTER_TYPE_ID` / `is_printer_type()` /
//! `printer_type_id_sql!()` в `crates/trackly-core/src/domain/devices.rs`
//! (значение — id сида V001). До этого пакета признак был закодирован шестью
//! способами (литерал `type_id = 2`, подзапрос по имени 'Принтер', `EXISTS`/
//! `JOIN printers`, локальные копии константы…), и разные места расходились:
//! `is_printer` сотрудника считался по строке `printers`, варианты принтеров
//! заявки — по имени типа (NEW-5); переименование типа в справочнике молча
//! обнуляло варианты.
//!
//! # Что и где ищется
//!
//! Обход `crates/*/src/**/*.rs` (список файлов выводится обходом; новый файл с
//! литералом краснит гейт). Из каждого файла вырезаются модули `#[cfg(test)]`,
//! затем лексер отбрасывает комментарии и разбирает строковые литералы
//! (обычные, сырые, байтовые), отличая их от символьных литералов и времён
//! жизни. Лексер, вырезание тестов и обход скопированы из
//! `device_type_label_gate.rs` (тест-бинарники друг друга не импортируют).
//! Канонический файл — `crates/trackly-core/src/domain/devices.rs`.
//!
//! Правила (сообщение начинается с кода):
//! * `[P1]` числовой признак вне канона: в строковом литерале (SQL)
//!   `type_id = 2` / `== 2` / `!= 2` / `<> 2` (без регистра, с алиасом `d.`);
//!   в коде `type_id == 2`, `type_id != 2`, `2 == x.type_id`, `type_id: 2`.
//! * `[P2]` признак по имени: строковый литерал с `'принтер'` в одинарных
//!   кавычках (SQL-сравнение с подписью), либо `==`/`!=` вплотную (через
//!   пробелы) к литералу «Принтер» в коде. Подпись в выводе (toast, CSV,
//!   `"printer" => "принтер"`) не флагается — только сравнение.
//! * `[P3]` структурный признак: строковый литерал с `FROM printers` /
//!   `JOIN printers`. Чтение ПОЛЕЙ принтера (тонер, SNMP, USB-хост, собственный
//!   репозиторий) законно — такие файлы перечислены в `PRINTER_DATA_READS` с
//!   точным числом и вердиктом. Новый файл, рост или убыль числа — красный.
//! * `[P4]` определение значения вне канона: `…PRINTER…TYPE…: i64 = 2`,
//!   `macro_rules! printer_type_id_sql`, ветка макроса `=> { "2" }`.
//! * `[P5]` антивакуум: файлов не меньше `MIN_FILES`; канон содержит
//!   `pub const PRINTER_TYPE_ID: i64 = 2;` и `macro_rules! printer_type_id_sql`;
//!   употреблений `PRINTER_TYPE_ID|is_printer_type|printer_type_id_sql!` вне
//!   канона не меньше `MIN_USES`; записи реестра P3 сработали (сверка точного
//!   числа — в `[P3]`).
//!
//! # Чего гейт не видит (честно)
//!
//! * SQL, склеенный из частей так, что `type_id =` и `2` в разных литералах
//!   (`concat!("… type_id = ", "2")`, `format!("… = {}", 2)`);
//! * число через переменную или константу с другим именем
//!   (`let k = 2; … type_id == k`, `const KIND: i64 = 2`);
//! * множества `type_id IN (1, 2)`, `BETWEEN`, сравнение с `Some(2)`,
//!   `matches!(t, 2)`, 2 в ветке `match` без `type_id` рядом, `2i64`;
//! * имя типа, сравниваемое не через `==`/`!=` (`.eq("Принтер")`,
//!   `starts_with`, `LIKE 'Принт%'`) или пришедшее из константы;
//! * структурный признак без ключевого слова `printers` в литерале (имя
//!   таблицы из константы, `EXISTS(SELECT 1 FROM "printers" …)` с кавычками);
//! * фронтенд — его держит `ui/scripts/check-printer-identity.mjs`;
//! * тесты (`crates/*/tests`, `#[cfg(test)]`): фикстуры законно сеют `type_id = 2`.

use std::collections::BTreeMap;

use regex::Regex;

/// Число файлов `crates/*/src`, ниже которого скан считается молча опустевшим.
const MIN_FILES: usize = 180;

/// Минимум употреблений канона вне канонического файла (после пакета — больше 20).
const MIN_USES: usize = 10;

const CANON: &str = "crates/trackly-core/src/domain/devices.rs";

const CARTRIDGE_SVC: &str = "crates/trackly-app/src/services/cartridge_service.rs";
const REQUEST_SVC: &str = "crates/trackly-app/src/services/request_service.rs";
const EMPLOYEE_SVC: &str = "crates/trackly-app/src/services/employee_device_service.rs";

/// Реестр вердиктов P3: файлы, где `FROM/JOIN printers` читает ДАННЫЕ принтера,
/// а не решает «это принтер». `(файл, точное число, вердикт ≥ 20 символов)`.
const PRINTER_DATA_READS: &[(&str, usize, &str)] = &[
    (
        "crates/trackly-app/src/services/dashboard_service.rs",
        3,
        "SNMP-счётчики/алерты: число опрашиваемых принтеров и алерты по строкам printers",
    ),
    (
        "crates/trackly-app/src/services/device_service.rs",
        2,
        "связь USB-хоста и сетевые поля принтера (ip, community, usb_host_device_id)",
    ),
    (
        EMPLOYEE_SVC,
        1,
        "данные принтера: тонер (последние показания) в карточке сотрудника",
    ),
    (
        "crates/trackly-infra/src/repos/groups_sqlite.rs",
        1,
        "связь USB-хоста: printers.usb_host_device_id для USB-принтеров группы",
    ),
    (
        "crates/trackly-infra/src/repos/printers_sqlite.rs",
        4,
        "репозиторий принтеров: собственная таблица printers (список, существование, удаление)",
    ),
    (
        "crates/trackly-infra/src/repos/repairs_sqlite.rs",
        2,
        "связь USB-хоста: принтеры, подключённые к ремонтируемому устройству",
    ),
];

const HINT: &str = "признак принтера = devices.type_id = printer_type_id_sql!() / \
     PRINTER_TYPE_ID / is_printer_type() из trackly-core (INT-31); JOIN printers — только \
     для чтения полей принтера, тогда добавьте вердикт в PRINTER_DATA_READS";

// ---------------------------------------------------------------------------
// Вырезание тестовых модулей (копия device_type_label_gate.rs)
// ---------------------------------------------------------------------------

fn strip_test_modules(src: &str) -> Result<(String, usize), String> {
    let lines: Vec<&str> = src.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut removed = 0usize;
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim_end() != "#[cfg(test)]" {
            out.push(lines[i].to_string());
            i += 1;
            continue;
        }
        let start = i;
        let mut j = i + 1;
        while j < lines.len() && lines[j].trim_start().starts_with("#[") {
            j += 1;
        }
        if j >= lines.len() {
            return Err(format!("строка {}: #[cfg(test)] без элемента", start + 1));
        }
        let head = lines[j].trim_end();
        let end = if head.ends_with(';') || (head.contains('{') && head.ends_with('}')) {
            j
        } else if head.ends_with('{') {
            let mut k = j + 1;
            while k < lines.len() && lines[k].trim_end() != "}" {
                k += 1;
            }
            if k >= lines.len() {
                return Err(format!(
                    "строка {}: тестовый блок без закрывающей `}}` в нуле колонки",
                    start + 1
                ));
            }
            k
        } else {
            return Err(format!(
                "строка {}: неузнанная форма элемента после #[cfg(test)]: `{head}`",
                start + 1
            ));
        };
        for _ in start..=end {
            out.push(String::new());
        }
        removed += 1;
        i = end + 1;
    }
    Ok((out.join("\n"), removed))
}

// ---------------------------------------------------------------------------
// Лексер (копия device_type_label_gate.rs)
// ---------------------------------------------------------------------------

struct Lit {
    line: usize,
    content: String,
    /// Байтовое смещение этого литерала (`""`) в `Scanned::joined`.
    code_pos: usize,
}

struct Scanned {
    /// Код без комментариев; литералы заменены на `""`, переводы строк сохранены.
    joined: String,
    literals: Vec<Lit>,
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn lex(src: &str) -> Scanned {
    let ch: Vec<char> = src.chars().collect();
    let n = ch.len();
    let mut code = String::new();
    let mut literals = Vec::new();
    let mut line = 1usize;
    let mut i = 0;
    while i < n {
        let c = ch[i];
        let next = ch.get(i + 1).copied();
        if c == '\n' {
            code.push('\n');
            line += 1;
            i += 1;
        } else if c == '/' && next == Some('/') {
            while i < n && ch[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 1;
            i += 2;
            while i < n && depth > 0 {
                if ch[i] == '/' && ch.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if ch[i] == '*' && ch.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                } else {
                    if ch[i] == '\n' {
                        code.push('\n');
                        line += 1;
                    }
                    i += 1;
                }
            }
        } else if c == 'r' && raw_string_start(&ch, i).is_some() {
            let (hashes, body) = raw_string_start(&ch, i).unwrap_or((0, i));
            let start_line = line;
            let mut k = body;
            let mut content = String::new();
            loop {
                if k >= n {
                    break;
                }
                if ch[k] == '"' && (0..hashes).all(|h| ch.get(k + 1 + h) == Some(&'#')) {
                    k += 1 + hashes;
                    break;
                }
                if ch[k] == '\n' {
                    line += 1;
                    code.push('\n');
                }
                content.push(ch[k]);
                k += 1;
            }
            literals.push(Lit {
                line: start_line,
                content,
                code_pos: code.len(),
            });
            code.push_str("\"\"");
            i = k;
        } else if c == '"' {
            let start_line = line;
            let mut k = i + 1;
            let mut content = String::new();
            while k < n && ch[k] != '"' {
                if ch[k] == '\\' && k + 1 < n {
                    content.push(ch[k]);
                    k += 1;
                }
                if ch[k] == '\n' {
                    line += 1;
                    code.push('\n');
                }
                content.push(ch[k]);
                k += 1;
            }
            literals.push(Lit {
                line: start_line,
                content,
                code_pos: code.len(),
            });
            code.push_str("\"\"");
            i = k + 1;
        } else if c == '\'' {
            if next == Some('\\') {
                let mut k = i + 3;
                while k < n && ch[k] != '\'' {
                    k += 1;
                }
                code.push_str("''");
                i = k + 1;
            } else if ch.get(i + 2) == Some(&'\'') {
                code.push_str("''");
                i += 3;
            } else {
                code.push(c);
                i += 1;
            }
        } else {
            code.push(c);
            i += 1;
        }
    }
    Scanned {
        joined: code,
        literals,
    }
}

fn raw_string_start(ch: &[char], i: usize) -> Option<(usize, usize)> {
    if i > 0 {
        let p = ch[i - 1];
        let pp = if i > 1 { Some(ch[i - 2]) } else { None };
        let prefix_ok = !is_ident(p) || (p == 'b' && !pp.is_some_and(is_ident));
        if !prefix_ok {
            return None;
        }
    }
    let mut j = i + 1;
    let mut hashes = 0;
    while ch.get(j) == Some(&'#') {
        hashes += 1;
        j += 1;
    }
    if ch.get(j) == Some(&'"') {
        Some((hashes, j + 1))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Правила
// ---------------------------------------------------------------------------

struct Rules {
    p1_sql: Regex,
    p1_code: Vec<Regex>,
    p3_sql: Regex,
    p4_const: Regex,
    p4_macro: Regex,
    uses: Regex,
}

fn rules() -> Rules {
    let re = |s: &str| Regex::new(s).expect("regex");
    Rules {
        p1_sql: re(r"(?i)type_id\s*(=|==|!=|<>)\s*2\b"),
        p1_code: vec![
            re(r"type_id\s*(==|!=)\s*2\b"),
            re(r"\b2\s*(==|!=)\s*[\w.]*type_id"),
            re(r"type_id\s*:\s*2\b"),
        ],
        p3_sql: re(r"(?i)\b(from|join)\s+printers\b"),
        p4_const: re(r"(?i)\b\w*printer\w*type\w*\s*:\s*i64\s*=\s*2\b"),
        p4_macro: re(r"macro_rules!\s*printer_type_id_sql\b"),
        uses: re(r"\bPRINTER_TYPE_ID\b|\bis_printer_type\b|\bprinter_type_id_sql!"),
    }
}

#[derive(Debug, Clone)]
struct Hit {
    rule: &'static str,
    file: String,
    line: usize,
    matched: String,
}

fn line_at(joined: &str, pos: usize) -> usize {
    joined[..pos].matches('\n').count() + 1
}

/// Ближайший непробельный хвост кода перед позицией: оканчивается на `==`/`!=`?
fn compare_before(prefix: &str) -> bool {
    let t = prefix.trim_end();
    t.ends_with("==") || t.ends_with("!=")
}

/// Начало кода после позиции: `==`/`!=`?
fn compare_after(suffix: &str) -> bool {
    let t = suffix.trim_start();
    t.starts_with("==") || t.starts_with("!=")
}

/// Перед литералом — `=> {` (ветка макроса с телом-литералом).
fn macro_arm_before(prefix: &str) -> bool {
    let t = prefix.trim_end();
    t.strip_suffix('{')
        .map(|h| h.trim_end().ends_with("=>"))
        .unwrap_or(false)
}

fn collect_hits(rel: &str, stripped: &str, r: &Rules) -> (Vec<Hit>, usize) {
    let scanned = lex(stripped);
    let canon = rel == CANON;
    let mut hits = Vec::new();
    let push = |hits: &mut Vec<Hit>, rule, line, matched: &str| {
        hits.push(Hit {
            rule,
            file: rel.to_string(),
            line,
            matched: matched.trim().to_string(),
        })
    };

    for lit in &scanned.literals {
        let low = lit.content.to_lowercase();
        if !canon {
            for m in r.p1_sql.find_iter(&lit.content) {
                push(&mut hits, "P1", lit.line, m.as_str());
            }
            if low.contains("'принтер'") {
                push(&mut hits, "P2", lit.line, "'Принтер'");
            }
            if low.trim() == "принтер" {
                let before = &scanned.joined[..lit.code_pos];
                let after = &scanned.joined[lit.code_pos + 2..];
                if compare_before(before) || compare_after(after) {
                    push(&mut hits, "P2", lit.line, "== \"Принтер\"");
                }
            }
            if lit.content.trim() == "2" && macro_arm_before(&scanned.joined[..lit.code_pos]) {
                push(&mut hits, "P4", lit.line, "=> { \"2\" }");
            }
        }
        for m in r.p3_sql.find_iter(&lit.content) {
            push(&mut hits, "P3", lit.line, m.as_str());
        }
    }

    let code = &scanned.joined;
    if !canon {
        for re in &r.p1_code {
            for m in re.find_iter(code) {
                push(&mut hits, "P1", line_at(code, m.start()), m.as_str());
            }
        }
        for re in [&r.p4_const, &r.p4_macro] {
            for m in re.find_iter(code) {
                push(&mut hits, "P4", line_at(code, m.start()), m.as_str());
            }
        }
    }
    let uses = if canon {
        0
    } else {
        r.uses.find_iter(code).count()
    };
    (hits, uses)
}

struct Report {
    violations: Vec<String>,
    files: usize,
    uses: usize,
    registry_found: BTreeMap<String, usize>,
    canon_text: Option<String>,
}

fn scan(sources: &[(String, String)], registry: &[(&str, usize, &str)]) -> Report {
    let r = rules();
    let mut violations = Vec::new();
    let mut uses = 0usize;
    let mut p3: BTreeMap<String, Vec<Hit>> = BTreeMap::new();
    let mut canon_text = None;
    for (rel, text) in sources {
        let stripped = match strip_test_modules(text) {
            Ok((s, _)) => s,
            Err(e) => {
                violations.push(format!("{rel}: [P5] {e}"));
                continue;
            }
        };
        if rel == CANON {
            canon_text = Some(stripped.clone());
        }
        let (hits, u) = collect_hits(rel, &stripped, &r);
        uses += u;
        for h in hits {
            if h.rule == "P3" {
                p3.entry(h.file.clone()).or_default().push(h);
            } else {
                violations.push(format!(
                    "{}:{}: [{}] `{}` — {HINT}",
                    h.file, h.line, h.rule, h.matched
                ));
            }
        }
    }
    let mut registry_found = BTreeMap::new();
    for (file, hits) in &p3 {
        match registry.iter().find(|(f, _, _)| f == file) {
            None => {
                for h in hits {
                    violations.push(format!(
                        "{}:{}: [P3] `{}` в файле без вердикта — {HINT}",
                        h.file, h.line, h.matched
                    ));
                }
            }
            Some((_, want, verdict)) => {
                registry_found.insert(file.clone(), hits.len());
                if hits.len() != *want {
                    let lines: Vec<String> = hits.iter().map(|h| h.line.to_string()).collect();
                    violations.push(format!(
                        "{file}: [P3] FROM/JOIN printers найдено {} (строки {}), в реестре {want} \
                         («{verdict}») — новое вхождение = новый структурный признак? {HINT}",
                        hits.len(),
                        lines.join(", ")
                    ));
                }
            }
        }
    }
    for (file, want, verdict) in registry {
        if !p3.contains_key(*file) {
            violations.push(format!(
                "{file}: [P3] запись реестра без срабатывания (в реестре {want}, «{verdict}») — \
                 удалите запись или верните чтение"
            ));
        }
    }
    Report {
        violations,
        files: sources.len(),
        uses,
        registry_found,
        canon_text,
    }
}

/// P5: антивакуум полного дерева.
fn vacuum(r: &Report) -> Vec<String> {
    let mut v = Vec::new();
    if r.files < MIN_FILES {
        v.push(format!(
            "[P5] скан нашёл {} файлов (< {MIN_FILES}) — гейт молча позеленел бы",
            r.files
        ));
    }
    match &r.canon_text {
        None => v.push(format!("[P5] нет канонического файла {CANON}")),
        Some(t) => {
            if !t.contains("pub const PRINTER_TYPE_ID: i64 = 2;") {
                v.push(format!(
                    "[P5] {CANON}: нет `pub const PRINTER_TYPE_ID: i64 = 2;`"
                ));
            }
            if !t.contains("macro_rules! printer_type_id_sql") {
                v.push(format!(
                    "[P5] {CANON}: нет `macro_rules! printer_type_id_sql`"
                ));
            }
        }
    }
    if r.uses < MIN_USES {
        v.push(format!(
            "[P5] употреблений канона вне {CANON}: {} (< {MIN_USES}) — признак не используется",
            r.uses
        ));
    }
    v
}

// ---------------------------------------------------------------------------
// Обход дерева (копия device_type_label_gate.rs)
// ---------------------------------------------------------------------------

fn rust_sources_under(root: &std::path::Path) -> Vec<(String, String)> {
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
    let mut out = Vec::new();
    for krate in std::fs::read_dir(root.join("crates")).expect("crates dir") {
        let src = krate.expect("crate entry").path().join("src");
        if src.is_dir() {
            walk(&src, root, &mut out);
        }
    }
    out.sort();
    out
}

/// Корень дерева: репозиторий или (для ручных прогонов) `PRINTER_IDENTITY_GATE_ROOT`.
fn gate_root() -> std::path::PathBuf {
    match std::env::var("PRINTER_IDENTITY_GATE_ROOT") {
        Ok(p) if !p.is_empty() => std::path::PathBuf::from(p),
        _ => std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root"),
    }
}

fn real_sources() -> Vec<(String, String)> {
    rust_sources_under(&gate_root())
}

fn rules_of(violations: &[String]) -> Vec<String> {
    violations
        .iter()
        .filter_map(|v| v.split("[P").nth(1).map(|s| format!("P{}", &s[..1])))
        .collect()
}

// ---------------------------------------------------------------------------
// Тесты
// ---------------------------------------------------------------------------

fn one(file: &str, text: &str) -> Vec<(String, String)> {
    vec![(file.to_string(), text.to_string())]
}

const CARTRIDGE_SRC: &str = include_str!("../src/services/cartridge_service.rs");
const REQUEST_SRC: &str = include_str!("../src/services/request_service.rs");
const EMPLOYEE_SRC: &str = include_str!("../src/services/employee_device_service.rs");
const CANON_SRC: &str = include_str!("../../trackly-core/src/domain/devices.rs");

fn with_replaced(file: &str, new_text: String) -> Vec<(String, String)> {
    let mut sources = real_sources();
    let slot = sources
        .iter_mut()
        .find(|(f, _)| f == file)
        .unwrap_or_else(|| panic!("нет файла {file}"));
    slot.1 = new_text;
    sources
}

/// Мутация по регэкспу-якорю с проверкой «ровно одно вхождение»
/// (урок mutation_test_anchor_must_be_unique).
fn mutate(src: &str, anchor: &str, replacement: &str) -> String {
    let re = Regex::new(anchor).expect("anchor regex");
    assert_eq!(
        re.find_iter(src).count(),
        1,
        "якорь мутации должен встречаться ровно один раз: {anchor}"
    );
    re.replace(src, regex::NoExpand(replacement)).into_owned()
}

/// Реальное дерево чисто; антивакуум [P5]; реестр P3 сработал с точными числами.
#[test]
fn printer_identity_tree_is_clean() {
    let sources = real_sources();
    for (f, n, verdict) in PRINTER_DATA_READS {
        assert!(verdict.chars().count() >= 20, "короткий вердикт: {f}");
        assert!(*n >= 1, "{f}");
    }
    let r = scan(&sources, PRINTER_DATA_READS);
    assert!(r.violations.is_empty(), "\n{}", r.violations.join("\n"));
    let v = vacuum(&r);
    assert!(v.is_empty(), "\n{}", v.join("\n"));
    for (f, n, _) in PRINTER_DATA_READS {
        assert_eq!(r.registry_found.get(*f), Some(n), "[P5] реестр P3: {f}");
    }
    // include_str! совпадает с диском (без переопределения корня)
    if std::env::var("PRINTER_IDENTITY_GATE_ROOT").is_err() {
        for (file, text) in [
            (CARTRIDGE_SVC, CARTRIDGE_SRC),
            (REQUEST_SVC, REQUEST_SRC),
            (EMPLOYEE_SVC, EMPLOYEE_SRC),
            (CANON, CANON_SRC),
        ] {
            let d = sources.iter().find(|(f, _)| f == file).unwrap();
            assert_eq!(d.1, text, "{file}");
        }
    }
}

/// Фикстура на каждое правило P1–P4.
#[test]
fn printer_identity_every_rule_has_a_fixture() {
    let cases: [(&str, &str, &str); 9] = [
        (
            "P1",
            "let q = \"SELECT 1 FROM devices d WHERE d.type_id = 2 AND x\";\n",
            "SQL-литерал с алиасом",
        ),
        (
            "P1",
            "let q = \"SELECT 1 FROM devices WHERE TYPE_ID<>2\";\n",
            "SQL <> без пробелов и регистра",
        ),
        (
            "P1",
            "fn f(type_id: i64) -> bool { if type_id == 2 { true } else { false } }\n",
            "сравнение в коде",
        ),
        (
            "P1",
            "fn f(d: &D) -> bool { 2 != d.type_id }\n",
            "обратный порядок",
        ),
        (
            "P1",
            "let n = DeviceNew { type_id: 2, name: x, origin_group_id: None };\n",
            "инициализация структуры",
        ),
        (
            "P2",
            "let q = \"WHERE d.type_id = (SELECT id FROM device_types WHERE name = 'Принтер')\";\n",
            "подзапрос по имени",
        ),
        (
            "P2",
            "fn f(t: &T) -> bool { t.name == \"Принтер\" }\n",
            "сравнение с подписью в коде",
        ),
        (
            "P3",
            "let q = \"SELECT d.id, EXISTS(SELECT 1 FROM printers p WHERE p.device_id = d.id)\";\n",
            "структурный признак в незарегистрированном файле",
        ),
        (
            "P4",
            "pub const PRINTER_TYPE_ID: i64 = 2;\n",
            "копия константы вне канона",
        ),
    ];
    for (rule, text, what) in cases {
        let r = scan(&one("crates/x/src/n.rs", text), &[]);
        assert_eq!(
            rules_of(&r.violations),
            vec![rule.to_string()],
            "{what}: {:#?}",
            r.violations
        );
    }
    // P4: копия макроса (определение и ветка-литерал)
    let r = scan(
        &one(
            "crates/x/src/m.rs",
            "macro_rules! printer_type_id_sql {\n    () => {\n        \"2\"\n    };\n}\n",
        ),
        &[],
    );
    assert_eq!(
        rules_of(&r.violations),
        vec!["P4".to_string(); 2],
        "{:#?}",
        r.violations
    );
    // P2: обратный порядок и `!=`
    let r = scan(
        &one(
            "crates/x/src/n.rs",
            "fn f(s: &str) -> bool { \"принтер\" != s }\n",
        ),
        &[],
    );
    assert_eq!(rules_of(&r.violations), vec!["P2".to_string()]);
}

/// Разрешённое: канон, подписи в выводе, ?2-параметры, комментарии, тесты.
#[test]
fn printer_identity_legal_forms_are_not_flagged() {
    let ok = "\
// type_id = 2 в комментарии; WHERE name = 'Принтер'
/// и в докстроке: `type_id == 2`, FROM printers
fn a(t: i64) -> bool { trackly_core::domain::devices::is_printer_type(t) }
const Q: &str = concat!(\"WHERE d.type_id = \", trackly_core::printer_type_id_sql!(), \" AND x\");
fn b(x: i64) -> String { format!(\"d.type_id = ?2 AND n = {x}\") }
fn c(k: &str) -> &'static str { match k { \"printer\" => \"принтер\", _ => \"устройство\" } }
fn d(ip: &str) -> String { format!(\"Принтер {ip}\") }
fn e(t: i64) -> bool { t == 20 || t == 12 }
fn f() -> i64 { let status_id = 2; status_id }
let q = \"SELECT * FROM printers_view\";
#[cfg(test)]
mod tests {
    fn t() { let _ = \"WHERE type_id = 2\"; let _ = \"FROM printers\"; let x = DeviceNew { type_id: 2, origin_group_id: None }; }
}
";
    let r = scan(&one("crates/x/src/ok.rs", ok), &[]);
    assert!(r.violations.is_empty(), "{:#?}", r.violations);
    // канонический файл сам по себе не флагается ни P1, ни P4
    let r = scan(&one(CANON, CANON_SRC), &[]);
    assert!(r.violations.is_empty(), "{:#?}", r.violations);
}

/// Реестр P3 самоинвалидируется: лишнее вхождение, пропавшее вхождение и
/// запись без срабатывания краснят.
#[test]
fn printer_identity_registry_is_self_invalidating() {
    let reg: &[(&str, usize, &str)] = &[(
        "crates/x/src/r.rs",
        1,
        "тестовый вердикт для самопроверки реестра",
    )];
    let once = "let q = \"SELECT toner FROM printers WHERE id = ?1\";\n";
    assert!(scan(&one("crates/x/src/r.rs", once), reg)
        .violations
        .is_empty());
    // лишнее
    let twice = format!("{once}{once}");
    let r = scan(&one("crates/x/src/r.rs", &twice), reg);
    assert_eq!(rules_of(&r.violations), vec!["P3".to_string()]);
    assert!(r.violations[0].contains("найдено 2"), "{:#?}", r.violations);
    // пропавшее (запись без срабатывания)
    let r = scan(&one("crates/x/src/r.rs", "fn x() {}\n"), reg);
    assert_eq!(rules_of(&r.violations), vec!["P3".to_string()]);
    assert!(
        r.violations[0].contains("без срабатывания"),
        "{:#?}",
        r.violations
    );
    // то же чтение в другом файле: не освобождено + запись устарела
    let r = scan(&one("crates/x/src/other.rs", once), reg);
    assert_eq!(r.violations.len(), 2, "{:#?}", r.violations);
}

/// Новый файл с литералом краснит (список файлов не ручной).
#[test]
fn printer_identity_brand_new_file_is_caught() {
    let mut sources = real_sources();
    sources.push((
        "crates/trackly-app/src/services/brand_new_service.rs".to_string(),
        "pub const Q: &str = \"SELECT id FROM devices WHERE type_id = 2\";\n".to_string(),
    ));
    let r = scan(&sources, PRINTER_DATA_READS);
    assert!(
        r.violations
            .iter()
            .any(|v| v.contains("brand_new_service.rs:1: [P1]")),
        "{:#?}",
        r.violations
    );
}

/// Мутация РЕАЛЬНОГО cartridge_service.rs: макрос в SQL `suggest_compat_printer`
/// заменён литералом `2` -> [P1] с путём файла.
#[test]
fn printer_identity_mutation_cartridge_literal() {
    let m = mutate(
        CARTRIDGE_SRC,
        r#"type_id = ",\s*trackly_core::printer_type_id_sql!\(\),\s*" AND"#,
        "type_id = 2 AND",
    );
    let r = scan(&with_replaced(CARTRIDGE_SVC, m), PRINTER_DATA_READS);
    assert_eq!(
        rules_of(&r.violations),
        vec!["P1".to_string()],
        "{:#?}",
        r.violations
    );
    assert!(r.violations[0].contains("cartridge_service.rs"));
}

/// Мутация РЕАЛЬНОГО request_service.rs: B1 возвращён к подзапросу по имени -> [P2].
#[test]
fn printer_identity_mutation_request_by_name() {
    let m = mutate(
        REQUEST_SRC,
        r#"type_id = ",\s*trackly_core::printer_type_id_sql!\(\),\s*" AND"#,
        "type_id = (SELECT id FROM device_types WHERE name = 'Принтер') AND",
    );
    let r = scan(&with_replaced(REQUEST_SVC, m), PRINTER_DATA_READS);
    assert_eq!(
        rules_of(&r.violations),
        vec!["P2".to_string()],
        "{:#?}",
        r.violations
    );
    assert!(r.violations[0].contains("request_service.rs"));
}

/// Мутация РЕАЛЬНОГО employee_device_service.rs: B6 возвращён к
/// `EXISTS(SELECT 1 FROM printers …)` -> [P3] (число в файле больше реестра).
#[test]
fn printer_identity_mutation_employee_exists_printers() {
    let m = mutate(
        EMPLOYEE_SRC,
        r#"\(d\.type_id = ",\s*trackly_core::printer_type_id_sql!\(\),\s*"\)"#,
        "EXISTS(SELECT 1 FROM printers p WHERE p.device_id = d.id)",
    );
    let r = scan(&with_replaced(EMPLOYEE_SVC, m), PRINTER_DATA_READS);
    assert_eq!(
        rules_of(&r.violations),
        vec!["P3".to_string()],
        "{:#?}",
        r.violations
    );
    assert!(r.violations[0].contains("employee_device_service.rs"));
    assert!(r.violations[0].contains("найдено 2"), "{:#?}", r.violations);
}

/// Мутация РЕАЛЬНОГО канона: без константы/макроса антивакуум [P5] краснеет.
#[test]
fn printer_identity_mutation_canon_removed() {
    let m = mutate(
        CANON_SRC,
        r"pub const PRINTER_TYPE_ID: i64 = 2;",
        "pub const PRINTER_TYPE_ID: i64 = 3;",
    );
    let r = scan(&with_replaced(CANON, m), PRINTER_DATA_READS);
    let v = vacuum(&r);
    assert_eq!(rules_of(&v), vec!["P5".to_string()], "{v:#?}");

    let m = mutate(
        CANON_SRC,
        r"macro_rules! printer_type_id_sql",
        "macro_rules! printer_type_sql",
    );
    let r = scan(&with_replaced(CANON, m), PRINTER_DATA_READS);
    assert_eq!(rules_of(&vacuum(&r)), vec!["P5".to_string()]);

    // мало употреблений и мало файлов: синтетический набор
    let r = scan(&one(CANON, CANON_SRC), &[]);
    let v = vacuum(&r);
    assert_eq!(rules_of(&v), vec!["P5".to_string(); 2], "{v:#?}");
}
