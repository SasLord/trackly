//! Inventory gate "every audit_log entry of a request has a WS signal"
//! (Phase 41.5 Plan 08, D-24), built from the repository outward.
//!
//! The unread counters of the chat follow request events; an audit entry
//! written without any WS signal freezes the counter of other clients. The
//! gate scans every source under `src/`, finds each function that writes
//! `entity_type: "request"` and demands one of:
//! * the function itself sends `RequestStatusChanged` / `AttachmentsChanged` /
//!   `NewRequest` / `ChatMessage` or calls `finish_transition(`;
//! * a row in `DELEGATED` (the signal lives in a named caller that calls it);
//! * a row in `EXEMPT` with a reason of at least 40 characters.
//!
//! A new function with such an audit entry and no row turns the gate red; a
//! count floor guards against a green-by-emptiness scan.

use std::path::{Path, PathBuf};

const SIGNALS: &[&str] = &[
    "WsEvent::RequestStatusChanged",
    "WsEvent::AttachmentsChanged",
    "WsEvent::NewRequest",
    "WsEvent::ChatMessage",
    "finish_transition(",
];

/// (file name, fn, carrier fn in the same file). The carrier must itself
/// signal and must call the fn.
const DELEGATED: &[(&str, &str, &str)] = &[("repair_service.rs", "try_create", "create")];

/// (file name, fn, reason).
const EXEMPT: &[(&str, &str, &str)] = &[(
    "request_service.rs",
    "delete",
    "admin soft delete of a request: the row leaves the unread summary (deleted_at_utc filter); other clients refresh on their next load, no counter is frozen by a missing event; known gap, a WS signal is deferred",
)];

const MIN_FUNCTIONS: usize = 10;

/// Drops `//` comments (line and doc), collapses whitespace and glues a
/// leading `.` to the previous token so formatting does not matter.
fn normalize(src: &str) -> String {
    let mut no_comments = String::new();
    for line in src.lines() {
        let mut in_str = false;
        let b = line.as_bytes();
        let mut cut = line.len();
        let mut i = 0;
        while i < b.len() {
            match b[i] {
                b'\\' if in_str => i += 1,
                b'"' => in_str = !in_str,
                b'/' if !in_str && i + 1 < b.len() && b[i + 1] == b'/' => {
                    cut = i;
                    break;
                }
                _ => {}
            }
            i += 1;
        }
        no_comments.push_str(&line[..cut]);
        no_comments.push('\n');
    }
    let collapsed = no_comments.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.replace(" .", ".")
}

/// `(name, start, end)` of every `fn` (end = next fn header or EOF).
fn fns(s: &str) -> Vec<(String, usize, usize)> {
    let mut heads = Vec::new();
    let mut from = 0;
    while let Some(p) = s[from..].find("fn ") {
        let at = from + p;
        let prev_ok = at == 0 || s.as_bytes()[at - 1] == b' ';
        let name: String = s[at + 3..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if prev_ok && !name.is_empty() {
            heads.push((name, at));
        }
        from = at + 3;
    }
    let mut out = Vec::new();
    for (k, (name, start)) in heads.iter().enumerate() {
        let end = heads.get(k + 1).map(|h| h.1).unwrap_or(s.len());
        out.push((name.clone(), *start, end));
    }
    out
}

fn all_positions(hay: &str, needle: &str) -> Vec<usize> {
    hay.match_indices(needle).map(|(i, _)| i).collect()
}

fn enclosing(f: &[(String, usize, usize)], pos: usize) -> Option<&str> {
    f.iter()
        .find(|(_, s, e)| pos >= *s && pos < *e)
        .map(|(n, _, _)| n.as_str())
}

/// Replaces `anchor` exactly once inside the function `fn_header` (a raw
/// source slice from that header to the next `\n    ///` / `\n    pub`).
fn mutate_in_fn(src: &str, fn_header: &str, anchor: &str, with: &str) -> String {
    assert_eq!(
        src.matches(fn_header).count(),
        1,
        "fn header must be unique: {fn_header}"
    );
    let start = src.find(fn_header).unwrap();
    let rest = &src[start + fn_header.len()..];
    let end = start
        + fn_header.len()
        + rest
            .find("\n    ///")
            .or_else(|| rest.find("\n    pub "))
            .unwrap_or(rest.len());
    let body = &src[start..end];
    assert_eq!(
        body.matches(anchor).count(),
        1,
        "mutation anchor must occur exactly once in `{fn_header}`: {anchor}"
    );
    format!(
        "{}{}{}",
        &src[..start],
        body.replacen(anchor, with, 1),
        &src[end..]
    )
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn sources() -> Vec<(String, String)> {
    let mut paths = Vec::new();
    walk(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut paths,
    );
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            let text = std::fs::read_to_string(&p).unwrap();
            (name, text)
        })
        .collect()
}

fn signals(body: &str) -> bool {
    SIGNALS.iter().any(|s| body.contains(s))
}

/// Returns violations and the number of distinct `(file, fn)` audit writers.
fn check(files: &[(String, String)]) -> (Vec<String>, usize) {
    let mut v = Vec::new();
    let mut found: Vec<(String, String)> = Vec::new();
    for (file, text) in files {
        let s = normalize(text);
        let f = fns(&s);
        for p in all_positions(&s, "entity_type: \"request\"") {
            let owner = enclosing(&f, p).unwrap_or("<top level>").to_string();
            if !found.contains(&(file.clone(), owner.clone())) {
                found.push((file.clone(), owner));
            }
        }
    }
    for (file, name) in &found {
        let text = &files.iter().find(|(n, _)| n == file).unwrap().1;
        let s = normalize(text);
        let f = fns(&s);
        let body_of = |n: &str| {
            f.iter()
                .find(|(x, _, _)| x == n)
                .map(|(_, a, b)| s[*a..*b].to_string())
        };
        let body = body_of(name).unwrap_or_default();
        if signals(&body) {
            continue;
        }
        if let Some((_, _, carrier)) = DELEGATED.iter().find(|(fl, n, _)| fl == file && n == name) {
            match body_of(carrier) {
                Some(cb) if signals(&cb) && cb.contains(&format!("{name}(")) => {}
                _ => v.push(format!(
                    "{file}::{name}: carrier `{carrier}` does not signal or does not call it"
                )),
            }
            continue;
        }
        if let Some((_, _, why)) = EXEMPT.iter().find(|(fl, n, _)| fl == file && n == name) {
            if why.chars().count() < 40 {
                v.push(format!("{file}::{name}: exemption reason too short"));
            }
            continue;
        }
        v.push(format!(
            "{file}::{name}: audit entry for a request without a WS signal"
        ));
    }
    for (file, name, _) in EXEMPT {
        if !found.iter().any(|(f, n)| f == file && n == name) {
            v.push(format!("stale EXEMPT row {file}::{name}"));
        }
    }
    for (file, name, _) in DELEGATED {
        if !found.iter().any(|(f, n)| f == file && n == name) {
            v.push(format!("stale DELEGATED row {file}::{name}"));
        }
    }
    if found.len() < MIN_FUNCTIONS {
        v.push(format!(
            "only {} audit-writing functions found, floor is {MIN_FUNCTIONS}",
            found.len()
        ));
    }
    let n = found.len();
    (v, n)
}

fn replace_in_file(
    files: &[(String, String)],
    file: &str,
    f: impl Fn(&str) -> String,
) -> Vec<(String, String)> {
    files
        .iter()
        .map(|(n, t)| (n.clone(), if n == file { f(t) } else { t.clone() }))
        .collect()
}

#[test]
fn every_request_audit_entry_has_a_ws_signal() {
    let (v, n) = check(&sources());
    assert!(v.is_empty(), "{v:#?}");
    assert!(n >= MIN_FUNCTIONS);
}

#[test]
fn mutation_removed_signal_turns_the_inventory_red() {
    let files = sources();
    let m = replace_in_file(&files, "repair_service.rs", |t| {
        mutate_in_fn(
            t,
            "pub async fn complete(",
            "self.finish_transition(request_id, caller).await",
            "self.get_state(request_id, caller).await.map(|_| unreachable!())",
        )
    });
    let (v, _) = check(&m);
    assert!(v.iter().any(|x| x.contains("::complete")), "{v:#?}");
}

#[test]
fn mutation_new_audit_writer_without_a_row_turns_the_inventory_red() {
    let files = sources();
    let m = replace_in_file(&files, "request_service.rs", |t| {
        format!("{t}\nfn rogue_writer() {{ let _ = AuditEntry {{ entity_type: \"request\", entity_id: 1 }}; }}\n")
    });
    let (v, _) = check(&m);
    assert!(v.iter().any(|x| x.contains("::rogue_writer")), "{v:#?}");
}

#[test]
fn mutation_broken_delegation_turns_the_inventory_red() {
    let files = sources();
    let m = replace_in_file(&files, "repair_service.rs", |t| {
        mutate_in_fn(t, "pub async fn create(", "try_create(", "try_make(")
    });
    let (v, _) = check(&m);
    assert!(v.iter().any(|x| x.contains("try_create")), "{v:#?}");
}
