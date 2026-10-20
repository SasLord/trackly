//! Гейт плана 41.12-02 (INT-14, D-01): бэкенд не должен хранить соответствие
//! «id типа устройства <-> подпись». Источник подписей один: таблица
//! `device_types` (`DeviceService::list_types`, `TypeLookup`).
//!
//! # Что и где ищется
//!
//! Обход `crates/*/src/**/*.rs` (список файлов выводится обходом, вручную не
//! перечислен: новый файл с хардкодом краснит гейт). Из каждого файла сначала
//! вырезаются модули `#[cfg(test)]`, затем лексер отбрасывает комментарии и
//! разбирает строковые литералы (обычные, сырые, байтовые), отличая их от
//! символьных литералов и времён жизни. Лексер и обход скопированы из
//! `device_status_vocabulary_gate.rs` (тест-бинарники друг друга не
//! импортируют); добавлена только позиция литерала в очищенном коде.
//!
//! Правила:
//! * `[R1]` строковый литерал, равный (после trim, без регистра) слову
//!   «расходник», либо содержащий его в одинарных кавычках (SQL). До фазы 41.12
//!   CSV-экспорт печатал это слово на принтерах (type_id 2) — ложная подпись.
//! * `[R2]` ветка `match` с ЧИСЛОВЫМ левым операндом и литералом-подписью типа
//!   («Устройство», «Принтер», «Расходник», без регистра) справа:
//!   `2 => "Принтер"`, `1 | 2 => "Устройство"`, `Some(2) => "Принтер"`. Это
//!   карта id -> подпись; даже с ВЕРНОЙ подписью она копия справочника.
//!   Ветка со строковым/маской-левой частью (`"device" => "Устройство"`,
//!   `_ => "Устройство"`) не флагается.
//! * `[R3]` вакуум: просканировано не меньше `MIN_FILES` файлов и сработало
//!   освобождение из `EXEMPT` (последнее проверяет и точное число).
//!
//! # Освобождения (`EXEMPT`)
//!
//! Одно: псевдоним `LEGACY_EXPORT_PRINTER_LABEL_LOWER` в `device_service.rs`
//! (D-01, фаза 41.12): выгрузки до 41.12 печатали слово «Расходник» на
//! принтерах, и round-trip старых файлов работал по нему. Освобождение
//! описано файлом + правилом + точным текстом + подстрокой контекста + точным
//! числом. Лишнее вхождение, пропавшее вхождение и освобождение, под которое
//! ничего не попало, краснят гейт.
//!
//! # Чего гейт не видит (честно)
//!
//! * подпись, склеенная из частей (`concat!`, `format!("{}{}", ...)`);
//! * числовые сравнения `type_id == 2` — это идентичность, не подпись; вне
//!   правил (так принтеры отличаются от устройств по `printers`/`kind`);
//! * карта id -> подпись, у которой левая часть вычислена косвенно
//!   (`let k = 2; match t { x if x == k => "Принтер" ...}`);
//! * подпись «Устройство» как значение по умолчанию вне `match` с числом.

use std::collections::BTreeMap;

/// Подписи типов, чья пара с числовым id в ветке `match` — карта справочника.
const TYPE_LABELS: [&str; 3] = ["устройство", "принтер", "расходник"];

/// Число файлов `crates/*/src`, ниже которого скан считается молча опустевшим
/// (первый прогон по реальному дереву нашёл 184 файла).
const MIN_FILES: usize = 180;

#[derive(Debug, Clone)]
struct Exempt {
    file: &'static str,
    rule: &'static str,
    /// Точный текст срабатывания (литерал / фрагмент); пусто = любой.
    matched: &'static str,
    /// Подстрока контекста срабатывания (строка исходника).
    context: &'static str,
    count: usize,
    reason: &'static str,
}

const DEVICE_SVC: &str = "crates/trackly-app/src/services/device_service.rs";

const EXEMPT: &[Exempt] = &[Exempt {
    file: DEVICE_SVC,
    rule: "R1",
    matched: "расходник",
    context: "LEGACY_EXPORT_PRINTER_LABEL_LOWER",
    count: 1,
    reason: "псевдоним старой подписи принтера в CSV-импорте (D-01, фаза 41.12): \
             выгрузки до 41.12 печатали это слово на принтерах",
}];

const NEW_SITE_HINT: &str = "справочник device_types: подпись типа берётся из TypeLookup / \
     DeviceService::list_types, а не из карты id -> подпись и не из литерала; если это именованный \
     псевдоним старой подписи, внеси точное освобождение в EXEMPT с причиной";

// ---------------------------------------------------------------------------
// Вырезание тестовых модулей
// ---------------------------------------------------------------------------

/// Заменяет блоки `#[cfg(test)] mod ... { ... }` (и одиночные `#[cfg(test)]`
/// элементы) пустыми строками, сохраняя нумерацию. Распознаётся только форма
/// rustfmt (маркер в нуле колонки, закрытие `}` в нуле колонки); неузнанная
/// форма — ошибка, а не молчаливый пропуск.
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
        // маркер + возможные дополнительные атрибуты
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
// Лексер
// ---------------------------------------------------------------------------

struct Lit {
    line: usize,
    content: String,
    /// Байтовое смещение этого литерала (`""`) в `Scanned::joined`.
    code_pos: usize,
}

struct Scanned {
    /// Тот же код одной строкой (для разбора контекста перед литералом).
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
                // '\n', '\'', '\u{..}'
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
                // время жизни `'a`
                code.push(c);
                i += 1;
            }
        } else {
            code.push(c);
            i += 1;
        }
    }
    Scanned {
        joined: code.clone(),
        literals,
    }
}

/// Если на позиции `i` начинается сырая строка (`r"`, `r#"`, `br#"`), вернуть
/// (число `#`, индекс первого символа тела).
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

#[derive(Debug, Clone)]
struct Hit {
    rule: &'static str,
    file: String,
    line: usize,
    matched: String,
    context: String,
}

/// Перед позицией `prefix` (код до литерала) стоит `<число> =>` либо
/// `Some(<число>) =>`: левая часть ветки `match` — числовой id.
fn numeric_arm_before(prefix: &str) -> bool {
    let ch: Vec<char> = prefix.chars().collect();
    let mut p = ch.len();
    while p > 0 && ch[p - 1].is_whitespace() {
        p -= 1;
    }
    if p < 2 || ch[p - 2] != '=' || ch[p - 1] != '>' {
        return false;
    }
    p -= 2;
    while p > 0 && ch[p - 1].is_whitespace() {
        p -= 1;
    }
    if p == 0 {
        return false;
    }
    if ch[p - 1] == ')' {
        // `Some(2)`: содержимое скобок должно быть числом
        let close = p - 1;
        let mut open = close;
        while open > 0 && ch[open - 1] != '(' {
            open -= 1;
        }
        if open == 0 {
            return false;
        }
        let inner: String = ch[open..close].iter().collect();
        let inner = inner.trim();
        return !inner.is_empty() && inner.chars().all(|c| c.is_ascii_digit());
    }
    if !ch[p - 1].is_ascii_digit() {
        return false;
    }
    let mut s = p;
    while s > 0 && ch[s - 1].is_ascii_digit() {
        s -= 1;
    }
    // `x2 =>` — идентификатор, а не число
    !(s > 0 && is_ident(ch[s - 1]))
}

fn collect_hits(rel: &str, raw_stripped: &str) -> Vec<Hit> {
    let scanned = lex(raw_stripped);
    let raw_lines: Vec<&str> = raw_stripped.lines().collect();
    let line_text = |n: usize| raw_lines.get(n - 1).copied().unwrap_or("").to_string();
    let mut hits = Vec::new();

    for lit in &scanned.literals {
        let trimmed = lit.content.trim();
        let low = trimmed.to_lowercase();
        let full_low = lit.content.to_lowercase();
        if low == "расходник" || full_low.contains("'расходник'") {
            hits.push(Hit {
                rule: "R1",
                file: rel.to_string(),
                line: lit.line,
                matched: trimmed.to_string(),
                context: line_text(lit.line),
            });
        }
        if TYPE_LABELS.contains(&low.as_str())
            && numeric_arm_before(&scanned.joined[..lit.code_pos])
        {
            hits.push(Hit {
                rule: "R2",
                file: rel.to_string(),
                line: lit.line,
                matched: trimmed.to_string(),
                context: line_text(lit.line),
            });
        }
    }
    hits
}

fn exempt_matches(e: &Exempt, h: &Hit) -> bool {
    e.file == h.file
        && e.rule == h.rule
        && (e.matched.is_empty() || e.matched == h.matched)
        && h.context.contains(e.context)
}

struct Report {
    violations: Vec<String>,
    files: usize,
    exempt_found: usize,
}

fn scan(sources: &[(String, String)], exempt: &[Exempt]) -> Report {
    let mut violations = Vec::new();
    let mut found: BTreeMap<usize, usize> = BTreeMap::new();
    for (rel, text) in sources {
        let stripped = match strip_test_modules(text) {
            Ok((s, _)) => s,
            Err(e) => {
                violations.push(format!("{rel}: {e}"));
                continue;
            }
        };
        for h in collect_hits(rel, &stripped) {
            match exempt.iter().position(|e| exempt_matches(e, &h)) {
                Some(k) => *found.entry(k).or_insert(0) += 1,
                None => violations.push(format!(
                    "{}:{}: [{}] `{}` — {NEW_SITE_HINT}",
                    h.file, h.line, h.rule, h.matched
                )),
            }
        }
    }
    let mut exempt_found = 0;
    for (k, e) in exempt.iter().enumerate() {
        let got = found.get(&k).copied().unwrap_or(0);
        exempt_found += got;
        if got != e.count {
            violations.push(format!(
                "освобождение устарело или нарушено: {} [{}] `{}` в контексте `{}` — найдено {got}, \
                 в таблице {} ({}). Подними/опусти число или удали запись",
                e.file, e.rule, e.matched, e.context, e.count, e.reason
            ));
        }
    }
    Report {
        violations,
        files: sources.len(),
        exempt_found,
    }
}

// ---------------------------------------------------------------------------
// Обход дерева
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

/// Корень дерева: репозиторий или (для ручных прогонов и мутаций копии)
/// `TYPE_LABEL_GATE_ROOT`.
fn gate_root() -> std::path::PathBuf {
    match std::env::var("TYPE_LABEL_GATE_ROOT") {
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

fn rules_of(r: &Report) -> Vec<String> {
    r.violations
        .iter()
        .filter_map(|v| v.split("[R").nth(1).map(|s| format!("R{}", &s[..1])))
        .collect()
}

// ---------------------------------------------------------------------------
// Тесты
// ---------------------------------------------------------------------------

fn one(file: &str, text: &str) -> Vec<(String, String)> {
    vec![(file.to_string(), text.to_string())]
}

const DEVICE_SRC: &str = include_str!("../src/services/device_service.rs");

/// Якорь мутаций: ровно одно вхождение в device_service.rs (проверяет `mutate`).
const ALIAS_ANCHOR: &str = "const LEGACY_EXPORT_PRINTER_LABEL_LOWER: &str = \"расходник\";";

/// Подмена текста реального исходника в наборе из дерева.
fn with_replaced(file: &str, new_text: String) -> Vec<(String, String)> {
    let mut sources = real_sources();
    let slot = sources
        .iter_mut()
        .find(|(f, _)| f == file)
        .unwrap_or_else(|| panic!("нет файла {file}"));
    slot.1 = new_text;
    sources
}

/// Мутация с проверкой уникальности якоря.
fn mutate(src: &str, anchor: &str, replacement: &str) -> String {
    assert_eq!(
        src.matches(anchor).count(),
        1,
        "якорь мутации должен быть уникален (урок mutation_test_anchor_must_be_unique): {anchor}"
    );
    src.replace(anchor, replacement)
}

/// Реальное дерево чисто; вакуум [R3]: файлов достаточно, освобождение сработало.
#[test]
fn device_type_label_no_hardcode_in_tree() {
    let sources = real_sources();
    assert!(
        sources.len() >= MIN_FILES,
        "[R3] скан нашёл {} файлов (< {MIN_FILES}) — гейт молча позеленел бы",
        sources.len()
    );
    for e in EXEMPT {
        assert!(
            e.reason.chars().count() >= 20,
            "причина слишком короткая: {e:?}"
        );
        assert!(e.count >= 1, "{e:?}");
    }
    let r = scan(&sources, EXEMPT);
    assert!(r.violations.is_empty(), "\n{}", r.violations.join("\n"));
    let want: usize = EXEMPT.iter().map(|e| e.count).sum();
    assert_eq!(
        r.exempt_found, want,
        "[R3] освобождение не сработало: найдено освобождённых != сумме в таблице"
    );
    assert!(
        r.exempt_found >= 1,
        "[R3] ни одно освобождение не сработало"
    );
    assert!(r.files >= MIN_FILES);
    // include_str! совпадает с диском (без переопределения корня)
    if std::env::var("TYPE_LABEL_GATE_ROOT").is_err() {
        let d = sources.iter().find(|(f, _)| f == DEVICE_SVC).unwrap();
        assert_eq!(d.1, DEVICE_SRC);
    }
}

/// Прежний код (до плана 41.12-01) краснел бы по R1 и R2.
#[test]
fn device_type_label_old_tree_is_red() {
    let old = "\
fn type_id_to_name(t: i64) -> &'static str {
    match t {
        2 => \"Расходник\",
        _ => \"Устройство\",
    }
}
";
    let r = scan(&one("crates/x/src/old.rs", old), EXEMPT);
    let rules = rules_of(&r);
    assert!(rules.contains(&"R1".to_string()), "{:#?}", r.violations);
    assert!(rules.contains(&"R2".to_string()), "{:#?}", r.violations);
    assert!(
        r.violations.iter().any(|v| v.contains("old.rs:3: [R1]")),
        "{:#?}",
        r.violations
    );
}

/// Каждый класс по отдельности; формы карты id -> подпись.
#[test]
fn device_type_label_every_rule_has_a_fixture() {
    let cases: [(&str, &str, &str); 8] = [
        ("R1", "let s = \"Расходник\";\n", "заглавная подпись"),
        ("R1", "let s = \" расходник \";\n", "пробелы и регистр"),
        (
            "R1",
            "let q = \"WHERE name = 'Расходник'\";\n",
            "подпись в SQL",
        ),
        ("R1", "let s = r#\"Расходник\"#;\n", "сырая строка"),
        (
            "R2",
            "let k = match t {\n    2 => \"Принтер\",\n    _ => \"\",\n};\n",
            "карта с верной подписью",
        ),
        (
            "R2",
            "let k = match t { 1 | 2 => \"Устройство\", _ => \"\" };\n",
            "несколько id в ветке",
        ),
        (
            "R2",
            "let k = match t { Some(2) => \"Принтер\", _ => \"\" };\n",
            "Some(число)",
        ),
        (
            "R2",
            "let k = match t { 3 =>\n \"Устройство\", _ => \"\" };\n",
            "перенос строки после =>",
        ),
    ];
    for (rule, text, what) in cases {
        let r = scan(&one("crates/x/src/n.rs", text), &[]);
        assert_eq!(
            rules_of(&r),
            vec![rule.to_string()],
            "{what}: {:#?}",
            r.violations
        );
    }
}

/// Разрешённое: идентичность по числу, строковые ветки, SQL по другой подписи,
/// комментарии и тестовые модули.
#[test]
fn device_type_label_legal_forms_are_not_flagged() {
    let ok = "\
// Расходник в комментарии: 2 => \"Расходник\"
/// и в докстроке: \"Расходник\"
fn is_printer(d: &Dev) -> bool { d.type_id == 2 }
fn kind(s: &str) -> &'static str { match s { \"device\" => \"Устройство\", _ => \"Устройство\" } }
fn fallback(x: Option<&str>) -> &str { x.unwrap_or(\"Устройство\") }
fn by_id(x: usize) -> &'static str { match x { x2 => \"Принтер\" } }
let q = \"SELECT id FROM device_types WHERE name = 'Принтер'\";
let n = \"Принтер Дельта\";
#[cfg(test)]
mod tests {
    fn t() { let _ = \"Расходник\"; let _ = match 2 { 2 => \"Принтер\", _ => \"\" }; }
}
";
    let r = scan(&one("crates/x/src/ok.rs", ok), &[]);
    assert!(r.violations.is_empty(), "{:#?}", r.violations);
}

/// Новый файл с картой краснит (список файлов не ручной).
#[test]
fn device_type_label_brand_new_file_is_caught() {
    let mut sources = real_sources();
    sources.push((
        "crates/trackly-app/src/services/brand_new_service.rs".to_string(),
        "pub fn label(t: i64) -> &'static str { match t { 1 => \"Устройство\", _ => \"\" } }\n"
            .to_string(),
    ));
    let r = scan(&sources, EXEMPT);
    assert!(
        r.violations
            .iter()
            .any(|v| v.contains("brand_new_service.rs:1: [R2]")),
        "{:#?}",
        r.violations
    );
}

/// Освобождение самоинвалидируется: исчезло вхождение или появилось лишнее.
#[test]
fn device_type_label_exemptions_are_self_invalidating() {
    // реальный device_service.rs без строки с константой: освобождению нечего освобождать
    let m = mutate(DEVICE_SRC, ALIAS_ANCHOR, "");
    let r = scan(&with_replaced(DEVICE_SVC, m), EXEMPT);
    assert_eq!(r.violations.len(), 1, "{:#?}", r.violations);
    assert!(r.violations[0].contains("устарело"), "{:#?}", r.violations);

    // синтетика: второе вхождение в освобождённом контексте
    let e = [Exempt {
        file: "crates/x/src/e.rs",
        rule: "R1",
        matched: "расходник",
        context: "LEGACY_ALIAS",
        count: 1,
        reason: "тестовое освобождение для самопроверки",
    }];
    let present = "const LEGACY_ALIAS: &str = \"расходник\";\n";
    assert!(scan(&one("crates/x/src/e.rs", present), &e)
        .violations
        .is_empty());
    let twice = format!("{present}{present}");
    let r = scan(&one("crates/x/src/e.rs", &twice), &e);
    assert_eq!(r.violations.len(), 1, "{:#?}", r.violations);
    // то же слово в другом файле не освобождено (+ устаревшее освобождение)
    let r = scan(&one("crates/x/src/other.rs", present), &e);
    assert_eq!(r.violations.len(), 2, "{:#?}", r.violations);
}

/// Мутация РЕАЛЬНОГО device_service.rs: второй литерал «Расходник» вне псевдонима.
#[test]
fn device_type_label_mutation_second_literal() {
    let m = mutate(
        DEVICE_SRC,
        ALIAS_ANCHOR,
        &format!("{ALIAS_ANCHOR}\nfn second_label() -> &'static str {{ \"Расходник\" }}"),
    );
    let r = scan(&with_replaced(DEVICE_SVC, m), EXEMPT);
    assert_eq!(rules_of(&r), vec!["R1".to_string()], "{:#?}", r.violations);
    assert!(r.violations[0].contains("device_service.rs"));

    // тот же литерал в нижнем регистре, но вне контекста псевдонима
    let m2 = mutate(
        DEVICE_SRC,
        ALIAS_ANCHOR,
        &format!("{ALIAS_ANCHOR}\nfn second_alias() -> &'static str {{ \"расходник\" }}"),
    );
    let r2 = scan(&with_replaced(DEVICE_SVC, m2), EXEMPT);
    assert_eq!(
        rules_of(&r2),
        vec!["R1".to_string()],
        "{:#?}",
        r2.violations
    );
}

/// Мутация РЕАЛЬНОГО device_service.rs: карта id -> ВЕРНАЯ подпись тоже краснит.
#[test]
fn device_type_label_mutation_id_map() {
    let m = mutate(
        DEVICE_SRC,
        ALIAS_ANCHOR,
        &format!(
            "{ALIAS_ANCHOR}\nfn label(t: i64) -> &'static str {{ match t {{ 2 => \"Принтер\", _ => \"Устройство\" }} }}"
        ),
    );
    let r = scan(&with_replaced(DEVICE_SVC, m), EXEMPT);
    assert_eq!(rules_of(&r), vec!["R2".to_string()], "{:#?}", r.violations);
    assert!(r.violations[0].contains("device_service.rs"));
}

/// Лексер: `//` внутри строки не обрезает хвост, позиции литералов указывают
/// на их место в очищенном коде.
#[test]
fn device_type_label_lexer_positions() {
    let src = "let a = \"http://host\"; match t { 2 => \"Принтер\", _ => \"\" }\n";
    let s = lex(src);
    assert_eq!(s.literals[0].content, "http://host");
    let lit = &s.literals[1];
    assert_eq!(lit.content, "Принтер");
    assert!(numeric_arm_before(&s.joined[..lit.code_pos]));
    assert!(!numeric_arm_before(&s.joined[..s.literals[2].code_pos]));
}
