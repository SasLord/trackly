//! ЧИСТЫЙ сканер исходников сервисов для слоя (1) реестрового гейта
//! (Phase 41.7, D-06/D-07/D-08/D-16). Без `std::fs`: на вход строка исходника,
//! на выход список нарушений; чтение файлов делает вызывающий тест.
//!
//! Состав:
//!
//! * [`prepare_with`] — конечный автомат по байтам: код / `//` / `/* */` (с
//!   вложенностью) / строка с экранированием / сырая строка `r#"..."#` /
//!   символьный литерал (апостроф отличается от лайфтайма: `'static`, `'_`,
//!   `'a` — это НЕ литерал). Результат — два текста ТОЙ ЖЕ байтовой длины с
//!   сохранёнными переводами строк: `plain` (комментарии вычищены, строки
//!   целы — отсюда читаются метки `"имя"`) и `skeleton` (комментарии и
//!   содержимое строк/символов вычищены — по нему считаются скобки и ищутся
//!   вызовы). Многобайтовые символы (кириллица) заменяются ровно тем же числом
//!   пробелов, что байт, поэтому индексы не сдвигаются.
//! * Вырезание `#[cfg(test)]`-элементов ПО БАЛАНСУ СКОБОК. Хвост от первого
//!   `#[cfg(test)]` НЕ отрезается: в `act_service.rs` тестовый модуль стоит
//!   посреди файла, а нетестовый код после него обязан оставаться видимым —
//!   иначе новая `pub async fn` в этой зоне прошла бы гейт незамеченной.
//!   Вырезанный диапазон заменяется пробелами в обоих текстах.
//! * [`scan`] / [`scan_with_stats`] — правила вердиктов: `NoVerdict`,
//!   `MissingBroadcast`, `BroadcastInsideWriter` (D-04: событие до коммита
//!   недопустимо), `BroadcastBeforeWriter` (D-04: рассылка обязана стоять
//!   ПОСЛЕ последней точки записи тела), `BroadcastWithoutWrite` (строка
//!   Broadcasts, в теле которой сканер не нашёл ни одной точки записи),
//!   `ExemptBroadcasts` (строка Exempt не рассылает: освобождение =
//!   «событие не из этой функции»), `ReadOnlyOwnsWriter`,
//!   `ReadOnlyCallsWriter`, `StaleRow`, `DuplicateFn`, `DeadCodeAllowLeft`.
//!
//! ТОЧКА ЗАПИСИ — не только `writer.execute(..)`: запись может быть
//! делегирована self-методу (`archive` пишет через `set_archived`,
//! `device::create` — через `create_without_broadcast` -> `insert_new_and_get`).
//! Поэтому множество «владельцев записи» файла замыкается фикспойнтом: функция,
//! вызывающая `self.<владелец>(` / `Self::<владелец>(`, сама владеет записью,
//! а вызов такого метода — точка записи наравне с `writer.execute(..)`.
//!
//! ИЗВЕСТНОЕ ОГРАНИЧЕНИЕ. Точки записи ищутся ТОЛЬКО в том же файле
//! (`writer.execute(` и вызовы `self.<имя>(` / `Self::<имя>(` его владельцев).
//! Запись через другой объект, макрос или из другого файла (`group_place::*`,
//! репозитории) сканер НЕ видит: строку Broadcasts без единой видимой точки
//! записи он краснит громко (`BroadcastWithoutWrite`), а не пропускает; строку
//! ReadOnly с такой записью правило `ReadOnlyCallsWriter` НЕ ловит — такие места
//! закрывает слой (2) — поведенческие сценарии с реальным захватом события.

use std::collections::{BTreeMap, BTreeSet};

use regex::Regex;

use super::registry::{Entry, Verdict};

/// Два текста одной байтовой длины, см. описание модуля.
#[derive(Debug, Clone)]
pub struct Prepared {
    pub plain: String,
    pub skeleton: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    NoVerdict,
    MissingBroadcast,
    BroadcastInsideWriter,
    ReadOnlyOwnsWriter,
    ReadOnlyCallsWriter,
    StaleRow,
    DuplicateFn,
    DeadCodeAllowLeft,
    BroadcastBeforeWriter,
    BroadcastWithoutWrite,
    ExemptBroadcasts,
}

/// Что гейт реально измерил (D-08: зелёный, ничего не измерив, недопустим).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScanStats {
    /// Строки Broadcasts, у которых проверка порядка отработала (есть и вызов
    /// рассылки, и хотя бы одна точка записи).
    pub broadcasts_ordered: usize,
    /// Строки Exempt, прошедшие механическую проверку «не рассылает».
    pub exempt_checked: usize,
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub file: String,
    pub func: String,
    pub kind: Kind,
    pub message: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}::{} [{:?}] {}",
            self.file, self.func, self.kind, self.message
        )
    }
}

/// Описание найденной функции.
#[derive(Debug, Clone)]
pub struct FnInfo {
    pub name: String,
    pub is_pub: bool,
    pub owns_write: bool,
    /// Байтовые индексы `{` и парной `}` тела.
    pub body: (usize, usize),
}

impl FnInfo {
    /// Функции, которым обязана соответствовать строка реестра.
    pub fn needs_row(&self) -> bool {
        self.is_pub || self.owns_write
    }
}

// ---------------------------------------------------------------------------
// Подготовка текста
// ---------------------------------------------------------------------------

fn utf8_len(lead: u8) -> usize {
    match lead {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

fn blank(v: &mut [u8], from: usize, to: usize) {
    let to = to.min(v.len());
    for byte in v.iter_mut().take(to).skip(from) {
        if *byte != b'\n' {
            *byte = b' ';
        }
    }
}

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `prepare_with(src, true)`.
pub fn prepare(src: &str) -> Prepared {
    prepare_with(src, true)
}

pub fn prepare_with(src: &str, strip_test_items: bool) -> Prepared {
    let b = src.as_bytes();
    let n = b.len();
    let mut plain = b.to_vec();
    let mut skel = b.to_vec();
    let mut i = 0usize;
    while i < n {
        let c = b[i];
        // Строчный комментарий (в том числе `///`, `//!`).
        if c == b'/' && i + 1 < n && b[i + 1] == b'/' {
            let end = b[i..].iter().position(|&x| x == b'\n').map_or(n, |p| i + p);
            blank(&mut plain, i, end);
            blank(&mut skel, i, end);
            i = end;
            continue;
        }
        // Блочный комментарий с вложенностью.
        if c == b'/' && i + 1 < n && b[i + 1] == b'*' {
            let mut depth = 1usize;
            let mut j = i + 2;
            while j < n && depth > 0 {
                if b[j] == b'/' && j + 1 < n && b[j + 1] == b'*' {
                    depth += 1;
                    j += 2;
                } else if b[j] == b'*' && j + 1 < n && b[j + 1] == b'/' {
                    depth -= 1;
                    j += 2;
                } else {
                    j += 1;
                }
            }
            blank(&mut plain, i, j);
            blank(&mut skel, i, j);
            i = j;
            continue;
        }
        // Сырая строка r#"..."# / br#"..."#.
        if (c == b'r' || (c == b'b' && i + 1 < n && b[i + 1] == b'r'))
            && (i == 0 || !is_ident(b[i - 1]))
        {
            let mut j = i + if c == b'b' { 2 } else { 1 };
            let mut hashes = 0usize;
            while j < n && b[j] == b'#' {
                hashes += 1;
                j += 1;
            }
            if j < n && b[j] == b'"' {
                let content = j + 1;
                let mut k = content;
                let mut close = n;
                while k < n {
                    if b[k] == b'"'
                        && k + 1 + hashes <= n
                        && b[k + 1..k + 1 + hashes].iter().all(|&x| x == b'#')
                    {
                        close = k;
                        break;
                    }
                    k += 1;
                }
                blank(&mut skel, content, close);
                i = (close + 1 + hashes).min(n);
                continue;
            }
        }
        // Обычная строка (в том числе байтовая b"...").
        if c == b'"' {
            let mut j = i + 1;
            while j < n && b[j] != b'"' {
                if b[j] == b'\\' {
                    j += 1;
                }
                j += 1;
            }
            let j = j.min(n);
            blank(&mut skel, i + 1, j);
            i = j + 1;
            continue;
        }
        // Апостроф: символьный литерал или лайфтайм.
        if c == b'\'' {
            if i + 1 < n && b[i + 1] == b'\\' {
                // '\n', '\'', '\u{..}': пропускаем экранированный символ, ищем закрытие.
                let mut j = i + 3;
                while j < n && b[j] != b'\'' {
                    j += 1;
                }
                let j = j.min(n);
                blank(&mut skel, i + 1, j);
                i = j + 1;
                continue;
            }
            if i + 1 < n {
                let clen = utf8_len(b[i + 1]);
                if i + 1 + clen < n && b[i + 1 + clen] == b'\'' && b[i + 1] != b'\'' {
                    blank(&mut skel, i + 1, i + 1 + clen);
                    i += 2 + clen;
                    continue;
                }
            }
            // Лайфтайм ('static, '_, 'a): это не литерал.
            i += 1;
            continue;
        }
        i += 1;
    }

    if strip_test_items {
        let ranges =
            test_item_ranges_in_skeleton(std::str::from_utf8(&skel).expect("skeleton UTF-8"));
        for (s, e) in ranges {
            blank(&mut plain, s, e);
            blank(&mut skel, s, e);
        }
    }

    Prepared {
        plain: String::from_utf8(plain).expect("plain: утверждение UTF-8 сохранено"),
        skeleton: String::from_utf8(skel).expect("skeleton: утверждение UTF-8 сохранено"),
    }
}

/// Индекс закрывающей скобки, парной открывающей на `open`.
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

fn skip_ws(sk: &[u8], mut p: usize) -> usize {
    while p < sk.len() && sk[p].is_ascii_whitespace() {
        p += 1;
    }
    p
}

/// Конец элемента, начинающегося с `from` (после атрибутов `#[cfg(test)]`).
fn item_end(sk: &[u8], from: usize) -> usize {
    let n = sk.len();
    let mut p = skip_ws(sk, from);
    // Прочие атрибуты перед элементом.
    while p < n && sk[p] == b'#' {
        let mut q = skip_ws(sk, p + 1);
        if q < n && sk[q] == b'!' {
            q = skip_ws(sk, q + 1);
        }
        if q < n && sk[q] == b'[' {
            match match_close(sk, q, b'[', b']') {
                Some(e) => p = skip_ws(sk, e + 1),
                None => return n,
            }
        } else {
            break;
        }
    }
    let head_end = (p + 80).min(n);
    let head = String::from_utf8_lossy(&sk[p..head_end]).into_owned();
    let kw_re =
        Regex::new(r"^(?:pub(?:\([^)]*\))?\s+)?(?:(?:async|unsafe|default)\s+)*(\w+)(?:\s+(\w+))?")
            .expect("regex");
    let (kw, kw2) = match kw_re.captures(&head) {
        Some(c) => (
            c.get(1).map_or("", |m| m.as_str()).to_string(),
            c.get(2).map_or("", |m| m.as_str()).to_string(),
        ),
        None => (String::new(), String::new()),
    };
    let semi_mode = matches!(kw.as_str(), "use" | "static" | "type")
        || (kw == "const" && kw2 != "fn" && kw2 != "unsafe" && kw2 != "async");

    let (mut par, mut brk, mut brc) = (0i64, 0i64, 0i64);
    let mut k = p;
    while k < n {
        match sk[k] {
            b'(' => par += 1,
            b')' => par -= 1,
            b'[' => brk += 1,
            b']' => brk -= 1,
            b'{' => {
                if semi_mode {
                    brc += 1;
                } else if par == 0 && brk == 0 {
                    return match match_close(sk, k, b'{', b'}') {
                        Some(e) => e + 1,
                        None => n,
                    };
                }
            }
            b'}' => {
                if semi_mode {
                    brc -= 1;
                }
            }
            b';' => {
                if par == 0 && brk == 0 && brc == 0 {
                    return k + 1;
                }
            }
            _ => {}
        }
        k += 1;
    }
    n
}

fn test_item_ranges_in_skeleton(sk: &str) -> Vec<(usize, usize)> {
    let re = Regex::new(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]").expect("regex");
    let bytes = sk.as_bytes();
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut last_end = 0usize;
    for m in re.find_iter(sk) {
        if m.start() < last_end {
            continue;
        }
        let e = item_end(bytes, m.end());
        out.push((m.start(), e));
        last_end = e;
    }
    out
}

/// Байтовый диапазон ПЕРВОГО `#[cfg(test)]`-элемента в ИСХОДНОМ тексте.
pub fn first_test_item_range(src: &str) -> Option<(usize, usize)> {
    let p = prepare_with(src, false);
    test_item_ranges_in_skeleton(&p.skeleton).into_iter().next()
}

// ---------------------------------------------------------------------------
// Разбор функций
// ---------------------------------------------------------------------------

fn writer_exec_re() -> Regex {
    Regex::new(r"\bwriter\b\s*\.\s*execute\s*\(").expect("regex")
}

/// Диапазоны скобок каждого `writer.execute( ... )` в `[lo, hi)` skeleton.
fn writer_ranges(sk: &[u8], text: &str, lo: usize, hi: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for m in writer_exec_re().find_iter(&text[lo..hi]) {
        let open = lo + m.end() - 1;
        if let Some(close) = match_close(sk, open, b'(', b')') {
            out.push((open, close));
        }
    }
    out
}

/// Все функции уровня элементов (вложенные `fn` внутри тел не выделяются).
pub fn parse_fns(p: &Prepared) -> Vec<FnInfo> {
    let sk = p.skeleton.as_bytes();
    let re = Regex::new(
        r"(?m)(?:^|[{};\]])[ \t]*(pub(?:\([^)]*\))?[ \t]+)?(?:const[ \t]+)?(?:async[ \t]+)?(?:unsafe[ \t]+)?fn[ \t]+(\w+)",
    )
    .expect("regex");
    let wre = writer_exec_re();
    let mut out = Vec::new();
    let mut next_ok = 0usize;
    for c in re.captures_iter(&p.skeleton) {
        let whole = c.get(0).expect("m");
        if c.get(2).expect("name").start() < next_ok {
            continue;
        }
        let name = c.get(2).expect("name").as_str().to_string();
        let is_pub = c.get(1).is_some();
        // До первой `{` на нулевой глубине () и []; `;` раньше — без тела.
        let (mut par, mut brk) = (0i64, 0i64);
        let mut k = whole.end();
        let mut open = None;
        while k < sk.len() {
            match sk[k] {
                b'(' => par += 1,
                b')' => par -= 1,
                b'[' => brk += 1,
                b']' => brk -= 1,
                b'{' if par == 0 && brk == 0 => {
                    open = Some(k);
                    break;
                }
                b';' if par == 0 && brk == 0 => break,
                _ => {}
            }
            k += 1;
        }
        let Some(open) = open else { continue };
        let Some(close) = match_close(sk, open, b'{', b'}') else {
            continue;
        };
        next_ok = close + 1;
        let owns_write = wre.is_match(&p.skeleton[open..=close]);
        out.push(FnInfo {
            name,
            is_pub,
            owns_write,
            body: (open, close),
        });
    }
    out
}

/// Функции, которым нужна строка реестра, по исходнику (вне тестовых элементов).
pub fn inventory(src: &str) -> Vec<FnInfo> {
    parse_fns(&prepare(src))
        .into_iter()
        .filter(FnInfo::needs_row)
        .collect()
}

/// Есть ли в нетестовой части исходника `writer.execute(`.
pub fn owns_writer_outside_tests(src: &str) -> bool {
    writer_exec_re().is_match(&prepare(src).skeleton)
}

/// Вызовы `broadcast_entities(` в `[lo, hi]`: (индекс скобки `(`, метка).
fn broadcast_calls(p: &Prepared, lo: usize, hi: usize) -> Vec<(usize, Option<String>)> {
    let re = Regex::new(r"\bbroadcast_entities\s*\(").expect("regex");
    let mut out = Vec::new();
    for m in re.find_iter(&p.skeleton[lo..=hi]) {
        let paren = lo + m.end() - 1;
        let after = skip_ws(p.plain.as_bytes(), paren + 1);
        let label = if p.plain.as_bytes().get(after) == Some(&b'"') {
            p.plain[after + 1..]
                .find('"')
                .map(|e| p.plain[after + 1..after + 1 + e].to_string())
        } else {
            None
        };
        out.push((paren, label));
    }
    out
}

/// Диапазоны скобок каждого `writer.execute( .. )` в теле функции.
pub fn writer_call_ranges(p: &Prepared, f: &FnInfo) -> Vec<(usize, usize)> {
    writer_ranges(p.skeleton.as_bytes(), &p.skeleton, f.body.0, f.body.1 + 1)
}

/// Регэксп вызова `self.<имя>(` / `Self::<имя>(` любого из `names`; имя — в
/// группе 1. `None` на пустом множестве.
fn self_call_re(names: &BTreeSet<String>) -> Option<Regex> {
    if names.is_empty() {
        return None;
    }
    let alt: Vec<String> = names.iter().map(|n| regex::escape(n)).collect();
    Some(
        Regex::new(&format!(
            r"(?:\bself\s*\.|\bSelf\s*::)\s*({})\s*\(",
            alt.join("|")
        ))
        .expect("regex"),
    )
}

/// Имена функций файла, транзитивно владеющих записью: начальное множество —
/// `owns_write`, дальше фикспойнт «зовёт self-метод из множества».
fn writing_fns(p: &Prepared, fns: &[FnInfo]) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = fns
        .iter()
        .filter(|f| f.owns_write)
        .map(|f| f.name.clone())
        .collect();
    loop {
        let Some(re) = self_call_re(&set) else { break };
        let mut grew = false;
        for f in fns {
            if set.contains(&f.name) {
                continue;
            }
            let body = &p.skeleton[f.body.0..=f.body.1];
            let calls_other = re
                .captures_iter(body)
                .any(|c| c.get(1).is_some_and(|m| m.as_str() != f.name));
            if calls_other {
                set.insert(f.name.clone());
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    set
}

/// Точки записи тела `f`: скобки `writer.execute(..)` и скобки вызовов
/// транзитивных владельцев записи (кроме самой `f`, рекурсия не запись).
fn write_points(p: &Prepared, f: &FnInfo, writing: &BTreeSet<String>) -> Vec<(usize, usize)> {
    let mut out = writer_call_ranges(p, f);
    if let Some(re) = self_call_re(writing) {
        let sk = p.skeleton.as_bytes();
        let (open, close) = f.body;
        for c in re.captures_iter(&p.skeleton[open..=close]) {
            if c.get(1).is_some_and(|m| m.as_str() == f.name) {
                continue;
            }
            let paren = open + c.get(0).expect("m").end() - 1;
            if let Some(end) = match_close(sk, paren, b'(', b')') {
                out.push((paren, end));
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Правила
// ---------------------------------------------------------------------------

pub fn scan(rel: &str, src: &str, registry: &[Entry]) -> Vec<Violation> {
    scan_with_stats(rel, src, registry).0
}

pub fn scan_with_stats(rel: &str, src: &str, registry: &[Entry]) -> (Vec<Violation>, ScanStats) {
    let mut stats = ScanStats::default();
    let p = prepare(src);
    let fns = parse_fns(&p);
    let mut out: Vec<Violation> = Vec::new();
    let v = |func: &str, kind: Kind, message: String| Violation {
        file: rel.to_string(),
        func: func.to_string(),
        kind,
        message,
    };

    let rows: BTreeMap<&str, &Entry> = registry
        .iter()
        .filter(|e| e.file == rel)
        .map(|e| (e.func, e))
        .collect();
    let owners: BTreeSet<String> = fns
        .iter()
        .filter(|f| f.owns_write)
        .map(|f| f.name.clone())
        .collect();
    let writing = writing_fns(&p, &fns);

    // Повторы имён.
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    for f in fns.iter().filter(|f| f.needs_row()) {
        *count.entry(f.name.as_str()).or_default() += 1;
    }
    let mut dup_reported: BTreeSet<&str> = BTreeSet::new();
    let required: BTreeSet<&str> = fns
        .iter()
        .filter(|f| f.needs_row())
        .map(|f| f.name.as_str())
        .collect();

    for f in fns.iter().filter(|f| f.needs_row()) {
        if count[f.name.as_str()] > 1 {
            if dup_reported.insert(f.name.as_str()) {
                out.push(v(
                    &f.name,
                    Kind::DuplicateFn,
                    format!(
                        "имя функции повторено {} раз(а): одна строка реестра не может описать обе",
                        count[f.name.as_str()]
                    ),
                ));
            }
            continue;
        }
        let Some(row) = rows.get(f.name.as_str()) else {
            out.push(v(
                &f.name,
                Kind::NoVerdict,
                "нет строки реестра: добавьте вердикт Broadcasts / Exempt / ReadOnly".into(),
            ));
            continue;
        };
        let (open, close) = f.body;
        match row.verdict {
            Verdict::Broadcasts => {
                let calls = broadcast_calls(&p, open, close);
                let good = calls
                    .iter()
                    .any(|(_, l)| l.as_deref() == Some(f.name.as_str()));
                if !good {
                    let labels: Vec<String> = calls
                        .iter()
                        .map(|(_, l)| format!("{:?}", l.as_deref().unwrap_or("<не литерал>")))
                        .collect();
                    out.push(v(
                        &f.name,
                        Kind::MissingBroadcast,
                        format!(
                            "вердикт Broadcasts, но нет вызова broadcast_entities(\"{}\", ..) (найденные метки: [{}])",
                            f.name,
                            labels.join(", ")
                        ),
                    ));
                }
                let wr = writer_ranges(p.skeleton.as_bytes(), &p.skeleton, open, close + 1);
                let mut inside: BTreeSet<usize> = BTreeSet::new();
                for (paren, _) in &calls {
                    if wr.iter().any(|(a, b)| paren > a && paren < b) {
                        inside.insert(*paren);
                        out.push(v(
                            &f.name,
                            Kind::BroadcastInsideWriter,
                            "broadcast_entities внутри скобок writer.execute(..): событие до коммита (D-04)"
                                .into(),
                        ));
                    }
                }
                // D-04: рассылка СТРОГО ПОСЛЕ последней точки записи тела.
                if !calls.is_empty() {
                    let points = write_points(&p, f, &writing);
                    match points.iter().map(|(_, e)| *e).max() {
                        None => out.push(v(
                            &f.name,
                            Kind::BroadcastWithoutWrite,
                            "вердикт Broadcasts, но сканер не нашёл в теле ни одной точки записи (writer.execute(..) или вызов self-метода, владеющего записью): запись идёт через другой файл/объект/макрос — порядок не проверяем, гейт краснеет громко"
                                .into(),
                        )),
                        Some(last_end) => {
                            stats.broadcasts_ordered += 1;
                            for (paren, _) in &calls {
                                if inside.contains(paren) {
                                    continue;
                                }
                                if *paren < last_end {
                                    out.push(v(
                                        &f.name,
                                        Kind::BroadcastBeforeWriter,
                                        "broadcast_entities стоит раньше конца последней точки записи тела: событие обязано идти ПОСЛЕ записи (D-04)"
                                            .into(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            Verdict::Exempt(_) => {
                // D-08: освобождение = «событие не из этой функции»; рассылка в
                // Exempt-функции противоречит собственной причине и удваивает событие.
                stats.exempt_checked += 1;
                let wr = writer_ranges(p.skeleton.as_bytes(), &p.skeleton, open, close + 1);
                for (paren, _) in broadcast_calls(&p, open, close) {
                    if wr.iter().any(|(a, b)| paren > *a && paren < *b) {
                        out.push(v(
                            &f.name,
                            Kind::BroadcastInsideWriter,
                            "Exempt-функция рассылает внутри скобок writer.execute(..): событие до коммита (D-04)"
                                .into(),
                        ));
                    } else {
                        out.push(v(
                            &f.name,
                            Kind::ExemptBroadcasts,
                            "Exempt-функция рассылает: либо вердикт Broadcasts, либо убрать вызов (освобождение = 'эта функция не шлёт событие')"
                                .into(),
                        ));
                    }
                }
            }
            Verdict::ReadOnly => {
                if f.owns_write {
                    out.push(v(
                        &f.name,
                        Kind::ReadOnlyOwnsWriter,
                        "вердикт ReadOnly, но тело содержит writer.execute(..)".into(),
                    ));
                }
                let body = &p.skeleton[open..=close];
                for owner in &owners {
                    if *owner == f.name {
                        continue;
                    }
                    let re = Regex::new(&format!(
                        r"(?:\bself\s*\.|\bSelf\s*::)\s*{}\s*\(",
                        regex::escape(owner)
                    ))
                    .expect("regex");
                    if re.is_match(body) {
                        out.push(v(
                            &f.name,
                            Kind::ReadOnlyCallsWriter,
                            format!("вердикт ReadOnly, но зовёт владельца записи {owner}(..)"),
                        ));
                    }
                }
            }
        }
    }

    for (name, _) in rows.iter() {
        if !required.contains(name) {
            out.push(v(
                name,
                Kind::StaleRow,
                "строка реестра есть, а функции (pub или владеющей записью) в исходнике нет".into(),
            ));
        }
    }

    let dead = Regex::new(
        r"#\s*\[\s*allow\s*\(\s*dead_code\s*\)\s*\]\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+broadcast_entities\b",
    )
    .expect("regex");
    if dead.is_match(&p.skeleton) {
        out.push(v(
            "broadcast_entities",
            Kind::DeadCodeAllowLeft,
            "#[allow(dead_code)] на заглушке broadcast_entities должен быть снят сервисным планом"
                .into(),
        ));
    }

    (out, stats)
}
