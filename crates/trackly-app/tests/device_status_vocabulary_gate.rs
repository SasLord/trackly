//! Гейт плана 41.1-08: бэкенд не должен хранить соответствие «id статуса
//! устройства <-> подпись» (D-09). Источник подписей один: таблица
//! `device_statuses` (ответ `device_statuses_list`).
//!
//! # Что и где ищется
//!
//! Обход `crates/*/src/**/*.rs` (список файлов выводится обходом, вручную не
//! перечислен: новый файл с хардкодом краснит гейт). Из каждого файла сначала
//! вырезаются модули `#[cfg(test)]` (блоки целиком, а НЕ «всё после первого
//! маркера»: в `act_service.rs` тестовых модулей два, а продакшен-код может
//! стоять и после них), затем лексер отбрасывает комментарии (`//`, `///`,
//! `//!`, вложенные `/* */`) и разбирает строковые литералы (обычные, сырые,
//! байтовые), отличая их от символьных литералов и времён жизни.
//!
//! Правила:
//! * `[R1]` строковый литерал, равный (без регистра) одной из подписей статусов
//!   устройства, либо содержащий её в одинарных кавычках (SQL). Регистр
//!   игнорируется нарочно: строчный псевдоним «на ремонте» — тоже хардкод.
//!   Машинные code (`на_складе`, `в_работе`, с подчёркиванием) подписями НЕ
//!   являются и разрешены.
//! * `[R2]` `match` / `matches!` по выражению с `status_id`: ветвление по
//!   числовому id статуса.
//! * `[R3]` числовое сравнение/инициализация на уровне Rust-кода:
//!   `status_id: 1`, `status_id == 3`, `status_id = 2`, `3 != x.status_id`.
//!   Проверки валидности (`status_id <= 0`) не считаются.
//! * `[R4]` числовое сравнение в SQL-литерале: `status_id = 1`, `status_id IN (1, 2)`,
//!   а внутри литерала с `device_statuses` ещё и `id = N`.
//!
//! Область: ВСЕ крейты, а не один `device_service.rs`.
//!
//! # Освобождения (`EXEMPT`)
//!
//! Собраны ОТ РЕПОЗИТОРИЯ запуском самого гейта по `crates/*/src`, а не списком
//! из брифинга. Каждое освобождение: файл + правило + точный текст срабатывания
//! (если задан) + подстрока контекста (строка исходника, для SQL весь литерал) +
//! точное число. Лишнее вхождение, пропавшее вхождение и освобождение, под
//! которое ничего не попало, краснят гейт (освобождение не может тихо устареть).
//!
//! * Картриджные статусы (`cartridge_statuses`, `c.status_id = 1/2/3/4`) —
//!   отдельный справочник, D-07; освобождены по правилам `[R1]`, `[R3]`, `[R4]`.
//! * `translate_request_status` — подпись статуса ЗАЯВКИ, не устройства.
//! * `LEGACY_REPAIR_LABEL_LOWER` — псевдоним старой подписи для CSV (план 21).
//! * `tauri_cmds/printers.rs`: `status_id: 1` при автосоздании устройства из
//!   SNMP-опроса. НЕ переведён на code (вне файлов плана 21). Освобождение
//!   временное и точное: переведёшь сайт на code -> гейт сам потребует снять
//!   запись.
//!
//! # Чего гейт не видит (честно)
//!
//! * числовой id статуса, вычисленный косвенно (`let k = 3; ... status_id: k`);
//! * SQL, собранный `format!` из кусков, ни один из которых не содержит пары
//!   `status_id` + число;
//! * подпись, склеенная из частей (`concat!`).

use std::collections::BTreeMap;

/// Подписи статусов устройств: три прежних + три добавленных V047.
const STATUS_NAMES: [&str; 7] = [
    "На складе",
    "В работе",
    "На ремонте",
    "Ожидает ремонта",
    "В ремонте",
    "На списание",
    "Списано",
];

#[derive(Debug, Clone)]
struct Exempt {
    file: &'static str,
    rule: &'static str,
    /// Точный текст срабатывания (литерал / фрагмент); пусто = любой.
    matched: &'static str,
    /// Подстрока контекста срабатывания (строка исходника; для SQL весь литерал).
    context: &'static str,
    count: usize,
    reason: &'static str,
}

const REPORT: &str = "crates/trackly-app/src/services/report_service.rs";
const DEVICE_SVC: &str = "crates/trackly-app/src/services/device_service.rs";

const EXEMPT: &[Exempt] = &[
    Exempt {
        file: REPORT,
        rule: "R1",
        matched: "В работе",
        context: "\"in_progress\" =>",
        count: 1,
        reason: "translate_request_status: подпись статуса ЗАЯВКИ, не устройства",
    },
    Exempt {
        file: REPORT,
        rule: "R1",
        matched: "В работе",
        context: "cartridge_snapshot",
        count: 2,
        reason: "имя картриджного статуса «В работе» (cartridge_statuses, D-07)",
    },
    Exempt {
        file: REPORT,
        rule: "R1",
        matched: "На складе",
        context: "cartridge_snapshot",
        count: 2,
        reason: "имя картриджного статуса «На складе» (cartridge_statuses, D-07)",
    },
    Exempt {
        file: DEVICE_SVC,
        rule: "R1",
        matched: "на ремонте",
        context: "LEGACY_REPAIR_LABEL_LOWER",
        count: 1,
        reason: "псевдоним старой подписи в CSV (обратная совместимость, план 21)",
    },
    Exempt {
        file: "crates/trackly-app/src/tauri_cmds/printers.rs",
        rule: "R3",
        matched: "status_id: 1",
        context: "status_id: 1,",
        count: 1,
        reason: "автосоздание устройства из SNMP: id «На складе» пока числом; перевести на code",
    },
    Exempt {
        file: "crates/trackly-core/src/domain/cartridges.rs",
        rule: "R3",
        matched: "status_id == 4",
        context: "current_status_id == 4",
        count: 1,
        reason: "статус КАРТРИДЖА «Списано» (cartridge_statuses, D-07), не устройства",
    },
    Exempt {
        file: "crates/trackly-app/src/services/dashboard_service.rs",
        rule: "R4",
        matched: "",
        context: "cartridges",
        count: 2,
        reason: "SQL по таблице cartridges: числовой статус картриджа (D-07)",
    },
    Exempt {
        file: "crates/trackly-app/src/services/cartridge_service.rs",
        rule: "R4",
        matched: "",
        context: "cartridges",
        count: 1,
        reason: "SQL по таблице cartridges: числовой статус картриджа (D-07)",
    },
    Exempt {
        file: "crates/trackly-infra/src/repos/cartridges_sqlite.rs",
        rule: "R4",
        matched: "",
        context: "cartridges",
        count: 12,
        reason: "SQL по таблице cartridges: числовой статус картриджа (D-07)",
    },
    Exempt {
        file: "crates/trackly-infra/src/repos/cartridges_sqlite.rs",
        rule: "R4",
        matched: "",
        context: "SELECT_CARTRIDGES",
        count: 1,
        reason: "SQL на SELECT_CARTRIDGES (cartridges c): числовой статус картриджа (D-07)",
    },
];

const NEW_SITE_HINT: &str = "справочник device_statuses: сопоставляй по code через StatusLookup / \
     device_statuses_list, а не по числу и не по подписи; если это статус другого справочника, \
     внеси точное освобождение в EXEMPT с причиной";

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
}

struct Scanned {
    /// Код без комментариев; содержимое литералов заменено на `""`.
    code: Vec<String>,
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
        code: code.split('\n').map(str::to_string).collect(),
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

fn find_all(hay: &[char], needle: &[char]) -> Vec<usize> {
    let mut out = Vec::new();
    if needle.is_empty() || hay.len() < needle.len() {
        return out;
    }
    for s in 0..=(hay.len() - needle.len()) {
        if hay[s..s + needle.len()] == *needle {
            out.push(s);
        }
    }
    out
}

fn skip_ws(ch: &[char], mut j: usize) -> usize {
    while ch.get(j).is_some_and(|c| c.is_whitespace() || *c == '\\') {
        j += 1;
    }
    j
}

/// Числовое сравнение/присваивание с идентификатором `ident`. `sql` выбирает
/// набор операторов. Возвращает фрагменты («status_id = 1»).
fn numeric_compare(text: &str, ident: &str, left_boundary: bool, sql: bool) -> Vec<String> {
    let ch: Vec<char> = text.chars().collect();
    let id: Vec<char> = ident.chars().collect();
    let mut out = Vec::new();
    for s in find_all(&ch, &id) {
        let e = s + id.len();
        if ch.get(e).copied().is_some_and(is_ident) {
            continue;
        }
        if left_boundary && s > 0 && is_ident(ch[s - 1]) {
            continue;
        }
        let mut j = skip_ws(&ch, e);
        let c0 = ch.get(j).copied();
        let c1 = ch.get(j + 1).copied();
        let mut op_found = false;
        if sql {
            if (c0 == Some('<') && c1 == Some('>'))
                || (c0 == Some('!') && c1 == Some('='))
                || (c0 == Some('=') && c1 == Some('='))
            {
                j += 2;
                op_found = true;
            } else if c0 == Some('=') {
                j += 1;
                op_found = true;
            } else if c0.is_some_and(|c| c == 'i' || c == 'I')
                && c1.is_some_and(|c| c == 'n' || c == 'N')
            {
                let k = skip_ws(&ch, j + 2);
                if ch.get(k) == Some(&'(') {
                    j = skip_ws(&ch, k + 1);
                    op_found = true;
                }
            }
        } else if (c0 == Some('=') && c1 == Some('=')) || (c0 == Some('!') && c1 == Some('=')) {
            j += 2;
            op_found = true;
        } else if (c0 == Some(':') && c1 != Some(':')) || (c0 == Some('=') && c1 != Some('>')) {
            j += 1;
            op_found = true;
        }
        if op_found {
            j = skip_ws(&ch, j);
            if ch.get(j).is_some_and(|c| c.is_ascii_digit()) {
                let mut k = j;
                while ch.get(k).is_some_and(|c| c.is_ascii_digit()) {
                    k += 1;
                }
                out.push(ch[s..k].iter().collect::<String>());
                continue;
            }
        }
        // обратный порядок: `3 == x.status_id` (только Rust)
        if !sql {
            let mut b = s;
            while b > 0 && (is_ident(ch[b - 1]) || ch[b - 1] == '.') {
                b -= 1;
            }
            let mut p = b;
            while p > 0 && ch[p - 1].is_whitespace() {
                p -= 1;
            }
            if p >= 2 && (ch[p - 2..p] == ['=', '='] || ch[p - 2..p] == ['!', '=']) {
                let mut q = p - 2;
                while q > 0 && ch[q - 1].is_whitespace() {
                    q -= 1;
                }
                if q > 0 && ch[q - 1].is_ascii_digit() {
                    out.push(ch[q - 1..e].iter().collect::<String>());
                }
            }
        }
    }
    out
}

fn collect_hits(rel: &str, raw_stripped: &str) -> Vec<Hit> {
    let scanned = lex(raw_stripped);
    let raw_lines: Vec<&str> = raw_stripped.lines().collect();
    let line_text = |n: usize| raw_lines.get(n - 1).copied().unwrap_or("").to_string();
    let lower_names: Vec<String> = STATUS_NAMES.iter().map(|s| s.to_lowercase()).collect();
    let mut hits = Vec::new();

    for lit in &scanned.literals {
        let trimmed = lit.content.trim();
        let low = trimmed.to_lowercase();
        let full_low = lit.content.to_lowercase();
        // Однословная подпись («Списано») в нижнем регистре — это машинный code
        // (`device_statuses.code = 'списано'`), поэтому для неё различается
        // регистр; многословные подписи («на ремонте») сравниваются без регистра.
        let exact = STATUS_NAMES.iter().zip(&lower_names).any(|(name, ln)| {
            if name.contains(' ') {
                *ln == low
            } else {
                *name == trimmed
            }
        });
        let quoted = STATUS_NAMES.iter().zip(&lower_names).any(|(name, ln)| {
            if name.contains(' ') {
                full_low.contains(&format!("'{ln}'"))
            } else {
                lit.content.contains(&format!("'{name}'"))
            }
        });
        if exact || quoted {
            hits.push(Hit {
                rule: "R1",
                file: rel.to_string(),
                line: lit.line,
                matched: trimmed.to_string(),
                context: line_text(lit.line),
            });
        }
        // R4: числовой статус в SQL
        let mut frags = numeric_compare(&lit.content, "status_id", false, true);
        if lit.content.contains("device_statuses") {
            frags.extend(numeric_compare(&lit.content, "id", true, true));
        }
        for f in frags {
            hits.push(Hit {
                rule: "R4",
                file: rel.to_string(),
                line: lit.line,
                matched: f,
                context: lit.content.clone(),
            });
        }
    }

    for (idx, code) in scanned.code.iter().enumerate() {
        let line_no = idx + 1;
        // R2
        let b: Vec<char> = code.chars().collect();
        for (kw, whole_rest) in [("match ", false), ("matches!(", true)] {
            let kwc: Vec<char> = kw.chars().collect();
            for s in find_all(&b, &kwc) {
                if s > 0 && is_ident(b[s - 1]) {
                    continue;
                }
                let rest: String = b[s..].iter().collect();
                let expr = if whole_rest {
                    rest.as_str()
                } else {
                    rest.split('{').next().unwrap_or("")
                };
                if expr.contains("status_id") {
                    hits.push(Hit {
                        rule: "R2",
                        file: rel.to_string(),
                        line: line_no,
                        matched: expr.trim().to_string(),
                        context: line_text(line_no),
                    });
                }
            }
        }
        // R3
        for f in numeric_compare(code, "status_id", false, false) {
            hits.push(Hit {
                rule: "R3",
                file: rel.to_string(),
                line: line_no,
                matched: f,
                context: line_text(line_no),
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
    stripped_blocks: usize,
    literals: usize,
    exempt_found: usize,
}

fn scan(sources: &[(String, String)], exempt: &[Exempt]) -> Report {
    let mut violations = Vec::new();
    let mut found: BTreeMap<usize, usize> = BTreeMap::new();
    let mut stripped_blocks = 0;
    let mut literals = 0;
    for (rel, text) in sources {
        let stripped = match strip_test_modules(text) {
            Ok((s, n)) => {
                stripped_blocks += n;
                s
            }
            Err(e) => {
                violations.push(format!("{rel}: {e}"));
                continue;
            }
        };
        literals += lex(&stripped).literals.len();
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
        stripped_blocks,
        literals,
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
/// `VOCAB_GATE_ROOT`.
fn gate_root() -> std::path::PathBuf {
    match std::env::var("VOCAB_GATE_ROOT") {
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

/// Счётчик отличает «ничего не ищет» от «зелёный»: комментарий, докстрока,
/// определение и тестовый модуль не считаются, реальный литерал считается.
#[test]
fn device_status_vocabulary_counter_selftest() {
    let fixture = "\
// \"На складе\" в комментарии
/// \"В работе\" в докстроке
//! \"На ремонте\" во внутренней докстроке
/* \"Ожидает ремонта\" /* вложенный */ \"В ремонте\" */
pub fn status_label(code: &str) -> &'static str { code_to_label(code) }
let url = \"http://x\"; // \"На списание\"
let s = \"Списано\";
#[cfg(test)]
mod tests {
    fn t() { let _ = \"На складе\"; let _ = Row { status_id: 2 }; }
}
";
    let r = scan(&one("crates/x/src/a.rs", fixture), &[]);
    assert_eq!(r.violations.len(), 1, "{:#?}", r.violations);
    assert!(
        r.violations[0].contains("a.rs:7: [R1] `Списано`"),
        "{:#?}",
        r.violations
    );
    assert_eq!(r.stripped_blocks, 1, "тестовый модуль вырезан");

    // без комментариев и тестов ничего не находится
    let clean = "// \"На складе\"\nfn f() {}\n";
    assert!(scan(&one("crates/x/src/b.rs", clean), &[])
        .violations
        .is_empty());
}

/// Каждый класс хардкода по отдельности: регистр, SQL, числа, match.
#[test]
fn device_status_vocabulary_every_rule_has_a_fixture() {
    let cases: [(&str, &str, &str); 12] = [
        ("R1", "let s = \"На ремонте\";\n", "заглавная подпись"),
        ("R1", "let s = \"в ремонте\";\n", "строчная подпись"),
        (
            "R1",
            "let s = \" ожидает ремонта \";\n",
            "пробелы и регистр",
        ),
        (
            "R1",
            "let q = \"WHERE name = 'Списано'\";\n",
            "подпись в SQL",
        ),
        ("R1", "let s = r#\"На списание\"#;\n", "сырая строка"),
        (
            "R2",
            "let k = match device.status_id {\n    3 => 1,\n    _ => 0,\n};\n",
            "match по status_id",
        ),
        (
            "R2",
            "if matches!(d.status_id, 3 | 4) {}\n",
            "matches! по status_id",
        ),
        (
            "R3",
            "let d = Dto { status_id: 3, name };\n",
            "инициализация числом",
        ),
        ("R3", "if d.status_id == 5 {}\n", "сравнение числом"),
        ("R3", "if 5 != d.status_id {}\n", "число слева"),
        (
            "R4",
            "let q = \"SELECT 1 FROM devices WHERE status_id = 3\";\n",
            "SQL =",
        ),
        ("R4", "let q = \"WHERE d.status_id IN (1, 2)\";\n", "SQL IN"),
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
    // join со справочником: id = N внутри литерала с device_statuses
    let r = scan(
        &one(
            "crates/x/src/n.rs",
            "let q = \"SELECT 1 FROM device_statuses s WHERE s.id = 3\";\n",
        ),
        &[],
    );
    assert_eq!(rules_of(&r), vec!["R4".to_string()], "{:#?}", r.violations);
}

/// Разрешённое: коды, проверки валидности, не-статусные id, имена в тестах.
#[test]
fn device_status_vocabulary_legal_forms_are_not_flagged() {
    let ok = "\
const A: &str = \"на_складе\";
const W: &str = \"списано\";
let q0 = \"WHERE code = 'списано'\";
let q = \"SELECT id FROM device_statuses WHERE code = 'в_работе'\";
if new.status_id <= 0 { return; }
let q2 = \"WHERE d.type_id = 1 AND status_id = ?1\";
let id: i64 = row.status_id;
let x = Dto { status_id: new.status_id };
let e = \"Устройство в работе\";
fn f<'a>(s: &'a str) -> &'a str { let c = '\"'; let d = '\\''; s }
let t = \"// не комментарий: На складе?\";
";
    let r = scan(&one("crates/x/src/ok.rs", ok), &[]);
    assert!(r.violations.is_empty(), "{:#?}", r.violations);
}

/// Лексер: `//` внутри строки не обрезает хвост, символьный литерал `'"'` не
/// открывает строку, сырая строка с кавычками и решётками.
#[test]
fn device_status_vocabulary_lexer_edge_cases() {
    let src = "\
let a = \"http://host/path\"; let b = \"Списано\";
let c = '\"'; let d = \"Списано\";
let e = r##\"x\"# y \"Списано\" \"##;
";
    let s = lex(src);
    let contents: Vec<&str> = s.literals.iter().map(|l| l.content.as_str()).collect();
    assert_eq!(contents[0], "http://host/path");
    assert_eq!(
        contents[1], "Списано",
        "хвост после `//` внутри строки виден"
    );
    assert_eq!(contents[2], "Списано", "`'\"'` не открывает строку");
    assert_eq!(contents.len(), 4, "{contents:?}");
}

/// Новый файл с хардкодом краснит (список файлов не ручной).
#[test]
fn device_status_vocabulary_brand_new_file_is_caught() {
    let mut sources = real_sources();
    sources.push((
        "crates/trackly-app/src/services/brand_new_service.rs".to_string(),
        "pub fn label() -> &'static str { \"Списано\" }\n".to_string(),
    ));
    let r = scan(&sources, EXEMPT);
    assert!(
        r.violations
            .iter()
            .any(|v| v.contains("brand_new_service.rs:1: [R1]")),
        "{:#?}",
        r.violations
    );
}

/// Порядок: продакшен-код ПОСЛЕ тестового модуля тоже сканируется (вырезается
/// блок, а не хвост файла). Тот же литерал внутри модуля — не нарушение.
#[test]
fn device_status_vocabulary_code_after_test_module_is_scanned() {
    let inside = "\
#[cfg(test)]
mod tests {
    fn t() { let _ = \"Списано\"; }
}
";
    let after = "\
#[cfg(test)]
mod tests {
    fn t() {}
}
pub fn leaked() -> &'static str { \"Списано\" }
";
    assert!(scan(&one("crates/x/src/i.rs", inside), &[])
        .violations
        .is_empty());
    let r = scan(&one("crates/x/src/a.rs", after), &[]);
    assert_eq!(rules_of(&r), vec!["R1".to_string()], "{:#?}", r.violations);

    // неузнанная форма — ошибка, а не молчание
    let weird = "#[cfg(test)]\nimpl Foo for Bar\n{\n}\n";
    let r = scan(&one("crates/x/src/w.rs", weird), &[]);
    assert_eq!(r.violations.len(), 1, "{:#?}", r.violations);
    assert!(r.violations[0].contains("неузнанная форма") || r.violations[0].contains("тестовый"));
}

/// Освобождение самоинвалидируется: исчезло вхождение или появилось лишнее.
#[test]
fn device_status_vocabulary_exemptions_are_self_invalidating() {
    let e = [Exempt {
        file: "crates/x/src/e.rs",
        rule: "R3",
        matched: "status_id: 1",
        context: "status_id: 1,",
        count: 1,
        reason: "тестовое освобождение для самопроверки",
    }];
    let present = "let d = Dto { status_id: 1, };\n";
    assert!(scan(&one("crates/x/src/e.rs", present), &e)
        .violations
        .is_empty());
    // конструкция исчезла -> освобождение устарело
    let gone = "let d = Dto { status_id: new.status_id, };\n";
    let r = scan(&one("crates/x/src/e.rs", gone), &e);
    assert_eq!(r.violations.len(), 1, "{:#?}", r.violations);
    assert!(r.violations[0].contains("устарело"));
    // появился второй сайт
    let twice = "let d = Dto { status_id: 1, };\nlet e = Dto { status_id: 1, };\n";
    let r = scan(&one("crates/x/src/e.rs", twice), &e);
    assert_eq!(r.violations.len(), 1, "{:#?}", r.violations);
    // то же значение в другом файле не освобождено
    let r = scan(&one("crates/x/src/other.rs", present), &e);
    assert_eq!(r.violations.len(), 2, "{:#?}", r.violations);
    // то же значение, но другой текст -> не покрыто
    let diff = "let d = Dto { status_id: 2, };\n";
    let r = scan(&one("crates/x/src/e.rs", diff), &e);
    assert_eq!(r.violations.len(), 2, "{:#?}", r.violations);
}

/// Чистое дерево + защита от вакуумности.
#[test]
fn device_status_vocabulary_no_hardcode_in_tree() {
    let sources = real_sources();
    assert!(
        sources.len() > 50,
        "скан нашёл {} файлов — гейт молча позеленел бы",
        sources.len()
    );
    assert!(!EXEMPT.is_empty());
    for e in EXEMPT {
        assert!(
            e.reason.chars().count() >= 20,
            "причина слишком короткая: {e:?}"
        );
        assert!(e.count >= 1, "{e:?}");
    }
    let r = scan(&sources, EXEMPT);
    assert!(
        r.stripped_blocks >= 5,
        "вырезано тестовых блоков: {}",
        r.stripped_blocks
    );
    assert!(r.literals > 1000, "разобрано литералов: {}", r.literals);
    // сначала нарушения (с адресами файл:строка), потом сверка суммы
    assert!(r.violations.is_empty(), "\n{}", r.violations.join("\n"));
    let want: usize = EXEMPT.iter().map(|e| e.count).sum();
    assert_eq!(
        r.exempt_found, want,
        "найдено освобождённых != сумме в таблице"
    );
    let _ = r.files;
}

/// Ручной прогон по дереву ДО фазы: `git archive d823d326 crates | tar -x -C DIR`,
/// затем `VOCAB_GATE_ROOT=DIR cargo test ... -- --ignored old_tree_is_red`.
#[test]
#[ignore = "нужен VOCAB_GATE_ROOT с деревом до фазы 41.1"]
fn device_status_vocabulary_old_tree_is_red() {
    let r = scan(&real_sources(), EXEMPT);
    assert!(
        r.violations.iter().any(|v| v.contains("device_service.rs")),
        "дерево до фазы обязано краснеть на device_service.rs: {:#?}",
        r.violations
    );
    eprintln!("{}", r.violations.join("\n"));
}

// ---------------------------------------------------------------------------
// Мутации РЕАЛЬНЫХ исходников (в памяти)
// ---------------------------------------------------------------------------

const DEVICE_SRC: &str = include_str!("../src/services/device_service.rs");
const REPORT_SRC: &str = include_str!("../src/services/report_service.rs");

/// Подмена текста реального исходника в наборе из дерева (или добавление).
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

#[test]
fn device_status_vocabulary_mutation_clean_sources_are_green() {
    // красный до, зелёный после на одном и том же коде гейта
    let r = scan(&real_sources(), EXEMPT);
    assert!(r.violations.is_empty(), "{:#?}", r.violations);
    // include_str! совпадает с диском (без VOCAB_GATE_ROOT)
    if std::env::var("VOCAB_GATE_ROOT").is_err() {
        let disk = real_sources();
        let d = disk.iter().find(|(f, _)| f == DEVICE_SVC).unwrap();
        assert_eq!(d.1, DEVICE_SRC);
        let rp = disk.iter().find(|(f, _)| f == REPORT).unwrap();
        assert_eq!(rp.1, REPORT_SRC);
    }
}

#[test]
fn device_status_vocabulary_mutation_label_function_in_device_service() {
    let anchor = "pub async fn list_statuses(";
    let m = mutate(
        DEVICE_SRC,
        anchor,
        &format!(
            "fn status_label(id: i64) -> &'static str {{ match id {{ 3 => \"Ожидает ремонта\", _ => \"\" }} }}\n    {anchor}"
        ),
    );
    let r = scan(&with_replaced(DEVICE_SVC, m), EXEMPT);
    assert_eq!(rules_of(&r), vec!["R1".to_string()], "{:#?}", r.violations);
    assert!(r.violations[0].contains("device_service.rs"));
}

#[test]
fn device_status_vocabulary_mutation_match_status_id_in_device_service() {
    let anchor = "pub async fn list_statuses(";
    let m = mutate(
        DEVICE_SRC,
        anchor,
        &format!("fn k(d: &Dev) -> i32 {{ match d.status_id {{ 3 => 1, _ => 0 }} }}\n    {anchor}"),
    );
    let r = scan(&with_replaced(DEVICE_SVC, m), EXEMPT);
    assert_eq!(rules_of(&r), vec!["R2".to_string()], "{:#?}", r.violations);
}

#[test]
fn device_status_vocabulary_mutation_numeric_status_in_device_service() {
    let anchor = "pub async fn list_statuses(";
    let m = mutate(
        DEVICE_SRC,
        anchor,
        &format!("fn d() -> Dev {{ Dev {{ status_id: 3 }} }}\n    {anchor}"),
    );
    let r = scan(&with_replaced(DEVICE_SVC, m), EXEMPT);
    assert_eq!(rules_of(&r), vec!["R3".to_string()], "{:#?}", r.violations);
}

#[test]
fn device_status_vocabulary_mutation_alias_duplicate_and_case() {
    // второй псевдоним в другом регистре и в другом месте — не освобождён
    let anchor = "pub async fn list_statuses(";
    let m = mutate(
        DEVICE_SRC,
        anchor,
        &format!("fn alias() -> &'static str {{ \"На ремонте\" }}\n    {anchor}"),
    );
    let r = scan(&with_replaced(DEVICE_SVC, m), EXEMPT);
    assert_eq!(rules_of(&r), vec!["R1".to_string()], "{:#?}", r.violations);
    // удалённый псевдоним: освобождение устарело
    let anchor2 = "const LEGACY_REPAIR_LABEL_LOWER: &str = \"на ремонте\";";
    let m2 = mutate(
        DEVICE_SRC,
        anchor2,
        "const LEGACY_REPAIR_LABEL_LOWER: &str = \"\";",
    );
    let r2 = scan(&with_replaced(DEVICE_SVC, m2), EXEMPT);
    assert_eq!(r2.violations.len(), 1, "{:#?}", r2.violations);
    assert!(r2.violations[0].contains("устарело"));
}

#[test]
fn device_status_vocabulary_mutation_report_service_device_label() {
    let anchor = "query_device_snapshot(&conn, &filter, \"в_работе\")";
    let m = mutate(
        REPORT_SRC,
        anchor,
        "query_device_snapshot(&conn, &filter, \"В работе\")",
    );
    let r = scan(&with_replaced(REPORT, m), EXEMPT);
    // лишний литерал вне освобождённого контекста + число картриджного не менялось
    assert_eq!(rules_of(&r), vec!["R1".to_string()], "{:#?}", r.violations);
    assert!(r.violations[0].contains("report_service.rs"));
}

#[test]
fn device_status_vocabulary_mutation_report_service_exempt_site_removed() {
    // один освобождённый сайт пропал -> число изменилось -> красный
    let anchor = "query_cartridge_snapshot(&conn, &filter, \"На складе\")";
    let m = mutate(
        REPORT_SRC,
        anchor,
        "query_cartridge_snapshot(&conn, &filter, \"stock\")",
    );
    let r = scan(&with_replaced(REPORT, m), EXEMPT);
    assert_eq!(r.violations.len(), 1, "{:#?}", r.violations);
    assert!(r.violations[0].contains("устарело"), "{:#?}", r.violations);
}

#[test]
fn device_status_vocabulary_mutation_report_service_request_label_reused() {
    // подпись заявки «В работе» перенесена в строку без контекста "in_progress =>"
    let anchor = "\"in_progress\" => \"В работе\".to_string()";
    let m = mutate(
        REPORT_SRC,
        anchor,
        "\"in_progress\" => \"stub\".to_string()",
    );
    let m = format!("{m}\npub fn leaked() -> String {{ \"В работе\".to_string() }}\n");
    let r = scan(&with_replaced(REPORT, m), EXEMPT);
    // и новый литерал вне контекста, и устаревшее освобождение заявки
    assert_eq!(r.violations.len(), 2, "{:#?}", r.violations);
}
