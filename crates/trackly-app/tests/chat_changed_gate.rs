//! Source gate for the chat broadcast (Phase 41.5 Plan 08, D-23, D-07).
//!
//! * Gate A: in `chat_service.rs` every `.send(` of the WS sender lies
//!   textually AFTER the end of the `writer.execute(...)` call (and its
//!   `.await`) of the same function, never inside the writer closure. The
//!   check is positional: a count alone is blind to order.
//! * Gate B: every function owning a `writer.execute(` has a written verdict
//!   below; an unlisted writer call turns the gate red. A function with the
//!   verdict "sends" must really send; a function with "no signal" must not.
//! * Gate C: chat never reloads the tree of places: no `EntitiesChanged` /
//!   `broadcast_entities(` / `send_entities_changed` in the chat service.
//! * Gate D (TOCTOU, D-07): `can_post_status(` stands INSIDE the writer
//!   closure of `send`, after `.transaction()` and before `.commit()`.
//! * Mutation proofs: each gate is shown to go red on an in-memory mutation of
//!   the real source; every anchor is asserted to occur exactly once.

use std::path::PathBuf;

const SERVICE: &str = "src/services/chat_service.rs";

/// Function name -> (sends a WS event after the writer await, reason) for every
/// `writer.execute(` owner. `discard_drafts` / `purge_stale_drafts` /
/// `discard_inner` reach the writer only through `delete_drafts`, so the
/// verdict lives there; they are checked below to stay signal-free.
const VERDICTS: &[(&str, bool, &str)] = &[
    (
        "send",
        true,
        "new message: ChatMessage (+ AttachmentsChanged)",
    ),
    (
        "mark_read",
        true,
        "read mark moves: ChatReadChanged to own devices",
    ),
    (
        "delete_drafts",
        false,
        "drafts are invisible to everybody but the author; no counter moves",
    ),
];

/// Functions that must stay free of any WS send.
const SILENT: &[&str] = &["discard_drafts", "discard_inner", "purge_stale_drafts"];

fn read(rel: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

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

/// Index just past the `)` matching the `(` at `open`; string and char
/// literals are skipped.
fn match_paren(s: &str, open: usize) -> usize {
    let b = s.as_bytes();
    assert_eq!(b[open], b'(');
    let mut depth = 0i32;
    let mut i = open;
    while i < b.len() {
        match b[i] {
            b'"' => {
                i += 1;
                while b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b'\'' => {
                if i + 2 < b.len() && b[i + 1] != b'\\' && b[i + 2] == b'\'' {
                    i += 2;
                } else if i + 3 < b.len() && b[i + 1] == b'\\' && b[i + 3] == b'\'' {
                    i += 3;
                }
            }
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("unbalanced parentheses");
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

/// Gate A + B over a source text. Returns human-readable violations.
fn check_order_and_inventory(src: &str) -> Vec<String> {
    let s = normalize(src);
    let f = fns(&s);
    let mut v = Vec::new();

    // Gate B: inventory from the code outward.
    let mut owners: Vec<String> = Vec::new();
    for p in all_positions(&s, "writer.execute(") {
        let owner = enclosing(&f, p).unwrap_or("<top level>").to_string();
        if !owners.contains(&owner) {
            owners.push(owner);
        }
    }
    for o in &owners {
        if !VERDICTS.iter().any(|(n, _, _)| n == o) {
            v.push(format!("B: writer.execute in unlisted fn `{o}`"));
        }
    }
    for (n, _, _) in VERDICTS {
        if !owners.iter().any(|o| o == n) {
            v.push(format!("B: verdict for `{n}` but no writer.execute found"));
        }
    }

    // Gate A: ordering inside every fn that sends.
    for (name, sends, _) in VERDICTS {
        let Some((_, start, end)) = f.iter().find(|(n, _, _)| n == name) else {
            v.push(format!("A: fn `{name}` not found"));
            continue;
        };
        let body = &s[*start..*end];
        let sent = all_positions(body, ".send(");
        if !*sends {
            if !sent.is_empty() {
                v.push(format!("B: `{name}` has verdict `no signal` but sends"));
            }
            continue;
        }
        if !body.contains("ws_tx") || sent.is_empty() {
            v.push(format!("B: `{name}` has verdict `sends` but never sends"));
        }
        let mut last_end = 0usize;
        for p in all_positions(body, "writer.execute(") {
            let open = p + "writer.execute".len();
            let close = match_paren(body, open);
            if !body[close..].starts_with(".await") {
                v.push(format!("A: `{name}` writer call is not awaited at once"));
            }
            last_end = last_end.max(close);
        }
        for n in sent {
            if n < last_end {
                v.push(format!("A: `{name}` sends before/inside the writer call"));
            }
        }
    }

    // `.send(` only inside the fns whose verdict says so.
    for p in all_positions(&s, ".send(") {
        let owner = enclosing(&f, p).unwrap_or("?");
        if !VERDICTS.iter().any(|(n, sd, _)| *n == owner && *sd) {
            v.push(format!("A: `.send(` outside the listed fns (in `{owner}`)"));
        }
    }
    for name in SILENT {
        if let Some((_, start, end)) = f.iter().find(|(n, _, _)| n == name) {
            let body = &s[*start..*end];
            if body.contains(".send(") || body.contains("ws_tx") {
                v.push(format!("B: `{name}` must stay free of WS sends"));
            }
        } else {
            v.push(format!("B: silent fn `{name}` not found"));
        }
    }
    v
}

/// Gate C over a source text.
fn check_no_tree_reload(src: &str) -> Vec<String> {
    let s = normalize(src);
    [
        "EntitiesChanged",
        "broadcast_entities(",
        "send_entities_changed",
    ]
    .iter()
    .filter(|n| s.contains(**n))
    .map(|n| format!("C: forbidden `{n}`"))
    .collect()
}

/// Gate D over a source text: the status check lives inside the writer
/// closure of `send`, between `.transaction()` and `.commit()`.
fn check_toctou(src: &str) -> Vec<String> {
    let s = normalize(src);
    let f = fns(&s);
    let Some((_, start, end)) = f.iter().find(|(n, _, _)| n == "send") else {
        return vec!["D: fn `send` not found".into()];
    };
    let body = &s[*start..*end];
    let Some(p) = body.find("writer.execute(") else {
        return vec!["D: `send` has no writer.execute".into()];
    };
    let close = match_paren(body, p + "writer.execute".len());
    let closure = &body[p..close];
    let tx = closure.find(".transaction()");
    let chk = closure.find("can_post_status(");
    let commit = closure.find(".commit()");
    match (tx, chk, commit) {
        (Some(t), Some(c), Some(m)) if t < c && c < m => Vec::new(),
        _ => vec![
            "D: can_post_status must stand inside the writer closure of `send` between transaction() and commit()"
                .into(),
        ],
    }
}

/// Anchor must occur exactly once in the whole text.
fn replace_once(src: &str, anchor: &str, with: &str) -> String {
    assert_eq!(
        src.matches(anchor).count(),
        1,
        "anchor not unique: {anchor}"
    );
    src.replacen(anchor, with, 1)
}

// ---------------------------------------------------------------- real gates

#[test]
fn gate_a_events_are_sent_after_the_writer_await() {
    let v = check_order_and_inventory(&read(SERVICE));
    assert!(v.is_empty(), "{v:#?}");
}

#[test]
fn gate_b_every_writer_call_has_a_verdict() {
    let v: Vec<_> = check_order_and_inventory(&read(SERVICE))
        .into_iter()
        .filter(|x| x.starts_with('B'))
        .collect();
    assert!(v.is_empty(), "{v:#?}");
}

#[test]
fn gate_c_chat_never_reloads_the_tree() {
    let v = check_no_tree_reload(&read(SERVICE));
    assert!(v.is_empty(), "{v:#?}");
}

#[test]
fn gate_d_status_is_checked_inside_the_writer_closure() {
    let v = check_toctou(&read(SERVICE));
    assert!(v.is_empty(), "{v:#?}");
}

// ----------------------------------------------------------- mutation proofs

#[test]
fn mutation_send_inside_the_writer_closure_turns_gate_a_red() {
    let src = read(SERVICE);
    let m = mutate_in_fn(
        &src,
        "pub async fn send(",
        "Ok(Written::Sent(row, promoted))\n            })",
        "if let Some(t) = &ws_probe { let _ = t.send(WsEvent::SettingsChanged); }\n                Ok(Written::Sent(row, promoted))\n            })",
    );
    let v = check_order_and_inventory(&m);
    assert!(
        v.iter()
            .any(|x| x.contains("`send`") && x.contains("before/inside")),
        "{v:#?}"
    );
}

#[test]
fn mutation_reordering_send_before_the_writer_turns_gate_a_red() {
    // Same number of sends: only a positional check sees this.
    let src = read(SERVICE);
    let before = normalize(&src).matches(".send(").count();
    let m = mutate_in_fn(
        &src,
        "pub async fn send(",
        "let written = self",
        "if let Some(t) = &self.ws_tx { let _ = t.send(WsEvent::SettingsChanged); }\n        let written = self",
    );
    let m = mutate_in_fn(
        &m,
        "pub async fn send(",
        "let _ = tx.send(WsEvent::ChatMessage {",
        "let _ = tx.try_probe(WsEvent::ChatMessage {",
    );
    assert_eq!(normalize(&m).matches(".send(").count(), before);
    let v = check_order_and_inventory(&m);
    assert!(
        v.iter()
            .any(|x| x.contains("`send`") && x.contains("before/inside")),
        "{v:#?}"
    );
}

#[test]
fn mutation_unlisted_writer_call_turns_gate_b_red() {
    let src = read(SERVICE);
    let m = mutate_in_fn(
        &src,
        "pub async fn unread_summary(",
        "let readers = self.readers.clone();",
        "let _ = self.writer.execute(|_c| Ok(())).await;\n        let readers = self.readers.clone();",
    );
    let v = check_order_and_inventory(&m);
    assert!(
        v.iter()
            .any(|x| x.starts_with("B:") && x.contains("unread_summary")),
        "{v:#?}"
    );
}

#[test]
fn mutation_removed_send_turns_gate_b_red() {
    let src = read(SERVICE);
    let m = mutate_in_fn(
        &src,
        "pub async fn mark_read(",
        "let _ = tx.send(WsEvent::ChatReadChanged { user_id });",
        "let _ = tx;",
    );
    let v = check_order_and_inventory(&m);
    assert!(
        v.iter()
            .any(|x| x.contains("`mark_read`") && x.contains("never sends")),
        "{v:#?}"
    );
}

#[test]
fn mutation_send_in_a_silent_function_turns_gate_red() {
    let src = read(SERVICE);
    let m = mutate_in_fn(
        &src,
        "pub async fn discard_drafts(",
        "self.discard_inner(request_id, user_id).await",
        "if let Some(t) = &self.ws_tx { let _ = t.send(WsEvent::SettingsChanged); }\n        self.discard_inner(request_id, user_id).await",
    );
    let v = check_order_and_inventory(&m);
    assert!(v.iter().any(|x| x.contains("discard_drafts")), "{v:#?}");
}

#[test]
fn mutation_broadcast_entities_turns_gate_c_red() {
    let src = read(SERVICE);
    let m = mutate_in_fn(
        &src,
        "pub async fn mark_read(",
        "Ok(())\n    }",
        "broadcast_entities(&self);\n        Ok(())\n    }",
    );
    let v = check_no_tree_reload(&m);
    assert!(
        v.iter().any(|x| x.contains("broadcast_entities(")),
        "{v:#?}"
    );
    let m = replace_once(
        &src,
        "let _ = tx.send(WsEvent::ChatReadChanged { user_id });",
        "let _e = WsEvent::EntitiesChanged;",
    );
    assert!(!check_no_tree_reload(&m).is_empty());
}

#[test]
fn mutation_status_check_outside_the_closure_turns_gate_d_red() {
    let src = read(SERVICE);
    // Check moved before the writer: the closure no longer re-reads/decides.
    let m = replace_once(
        &src,
        "Some((st, None)) if can_post_status(&st) => {}",
        "Some((_st, None)) => {}",
    );
    let m = replace_once(
        &m,
        "let written = self",
        "if !can_post_status(&access.status) { return Ok(reject(ChatRejectCode::Closed)); }\n        let written = self",
    );
    let v = check_toctou(&m);
    assert!(!v.is_empty(), "gate D stayed green");
}
