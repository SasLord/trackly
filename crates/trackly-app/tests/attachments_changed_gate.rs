//! Source gate for `AttachmentsChanged` (Phase 41.3 Plan 09, D-15/D-25).
//!
//! * Gate A: in `attachment_service.rs` the event is sent only through
//!   `notify_changed`, and every call of it lies textually AFTER the end of the
//!   `writer.execute(...)` call (and its `.await`) of the same function, never
//!   inside the writer closure. The check is positional: a count alone is blind
//!   to order.
//! * Gate B: every function owning a `writer.execute(` has a verdict below;
//!   an unlisted writer call turns the gate red (inventory from the code
//!   outward, not from a list of names inward).
//! * Gate C: attachments never reload the tree of places: no
//!   `EntitiesChanged` / `broadcast_entities(` / `send_entities_changed` and no
//!   `std::fs::copy` in the attachment service sources.
//! * Mutation proofs: each gate is shown to go red on an in-memory mutation of
//!   the real source; every anchor is asserted to occur exactly once.

use std::path::PathBuf;

const SERVICE: &str = "src/services/attachment_service.rs";
const ADMIN: &str = "src/services/attachment_admin.rs";

/// Function name -> verdict for every `writer.execute(` owner.
/// `true` = sends `AttachmentsChanged` after the writer await.
const VERDICTS: &[(&str, bool, &str)] = &[
    ("ingest", true, "upload adds a row"),
    ("delete", true, "soft delete hides a row"),
    ("restore", true, "restore shows a row"),
    ("purge", true, "purge removes a row"),
];

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
        if !*sends {
            continue;
        }
        let Some((_, start, end)) = f.iter().find(|(n, _, _)| n == name) else {
            v.push(format!("A: fn `{name}` not found"));
            continue;
        };
        let body = &s[*start..*end];
        let mut last_end = 0usize;
        for p in all_positions(body, "writer.execute(") {
            let open = p + "writer.execute".len();
            let close = match_paren(body, open);
            if !body[close..].starts_with(".await") {
                v.push(format!("A: `{name}` writer call is not awaited at once"));
            }
            last_end = last_end.max(close);
        }
        let notifies = all_positions(body, "notify_changed(");
        if notifies.is_empty() {
            v.push(format!("A: `{name}` never calls notify_changed"));
        }
        for n in notifies {
            if n < last_end {
                v.push(format!(
                    "A: `{name}` calls notify_changed before/inside the writer call"
                ));
            }
        }
    }

    // Single send site: `.send(` only inside notify_changed; notify_changed
    // only called from the listed fns.
    for p in all_positions(&s, ".send(") {
        if enclosing(&f, p) != Some("notify_changed") {
            v.push(format!(
                "A: `.send(` outside notify_changed (in `{}`)",
                enclosing(&f, p).unwrap_or("?")
            ));
        }
    }
    for p in all_positions(&s, "notify_changed(") {
        let owner = enclosing(&f, p).unwrap_or("?");
        if owner != "notify_changed" && !VERDICTS.iter().any(|(n, sd, _)| *n == owner && *sd) {
            v.push(format!("A: notify_changed called from `{owner}`"));
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
        "std::fs::copy",
    ]
    .iter()
    .filter(|n| s.contains(**n))
    .map(|n| format!("C: forbidden `{n}`"))
    .collect()
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

// ---------------------------------------------------------------- real gates

#[test]
fn gate_a_events_are_sent_after_the_writer_await() {
    let v = check_order_and_inventory(&read(SERVICE));
    assert!(v.iter().all(|x| x.starts_with("A")), "{v:?}");
    assert!(v.is_empty(), "{v:#?}");
}

#[test]
fn gate_b_every_writer_call_has_a_verdict() {
    let v = check_order_and_inventory(&read(SERVICE));
    assert!(v.is_empty(), "{v:#?}");
}

#[test]
fn gate_c_attachments_never_reload_the_tree() {
    for rel in [SERVICE, ADMIN] {
        let v = check_no_tree_reload(&read(rel));
        assert!(v.is_empty(), "{rel}: {v:#?}");
    }
}

// ----------------------------------------------------------- mutation proofs

#[test]
fn mutation_send_inside_the_writer_closure_turns_gate_a_red() {
    let src = read(SERVICE);
    let m = mutate_in_fn(
        &src,
        "pub async fn purge(",
        "if last {",
        "self.notify_changed(parent);\n                if last {",
    );
    let v = check_order_and_inventory(&m);
    assert!(
        v.iter()
            .any(|x| x.contains("`purge`") && x.contains("before/inside")),
        "{v:#?}"
    );
}

#[test]
fn mutation_reordering_send_before_the_writer_turns_gate_a_red() {
    // The count of sends stays the same: only a positional check sees this.
    let src = read(SERVICE);
    let m = mutate_in_fn(
        &src,
        "pub async fn delete(",
        "let changed = self",
        "self.notify_changed(parent).await;\n        let changed = self",
    );
    let before = normalize(&src).matches("notify_changed(").count();
    let after = normalize(&m).matches("notify_changed(").count();
    assert_eq!(after, before + 1);
    let v = check_order_and_inventory(&m);
    assert!(
        v.iter()
            .any(|x| x.contains("`delete`") && x.contains("before/inside")),
        "{v:#?}"
    );
    // Pure move (same count): drop the trailing send, keep the early one.
    let moved = mutate_in_fn(
        &m,
        "pub async fn delete(",
        "if changed {\n            self.notify_changed(parent).await;\n        }",
        "",
    );
    assert_eq!(normalize(&moved).matches("notify_changed(").count(), before);
    let v = check_order_and_inventory(&moved);
    assert!(v.iter().any(|x| x.contains("`delete`")), "{v:#?}");
}

#[test]
fn mutation_a_raw_send_outside_the_helper_turns_gate_a_red() {
    let src = read(SERVICE);
    let m = mutate_in_fn(
        &src,
        "pub async fn restore(",
        "self.notify_changed(parent).await;",
        "let _ = self.ws_tx.as_ref().map(|t| t.send(WsEvent::SettingsChanged));\n            self.notify_changed(parent).await;",
    );
    let v = check_order_and_inventory(&m);
    assert!(v.iter().any(|x| x.contains(".send(")), "{v:#?}");
}

#[test]
fn mutation_unlisted_writer_call_turns_gate_b_red() {
    let src = read(SERVICE);
    let m = mutate_in_fn(
        &src,
        "pub async fn max_bytes(",
        "let readers = self.readers.clone();",
        "let _ = self.writer.execute(|_c| Ok(())).await;\n        let readers = self.readers.clone();",
    );
    let v = check_order_and_inventory(&m);
    assert!(
        v.iter()
            .any(|x| x.starts_with("B:") && x.contains("max_bytes")),
        "{v:#?}"
    );
}

#[test]
fn mutation_broadcast_entities_turns_gate_c_red() {
    let src = read(SERVICE);
    let m = mutate_in_fn(
        &src,
        "pub async fn restore(",
        "self.notify_changed(parent).await;",
        "broadcast_entities(&self);\n            self.notify_changed(parent).await;",
    );
    let v = check_no_tree_reload(&m);
    assert!(
        v.iter().any(|x| x.contains("broadcast_entities(")),
        "{v:#?}"
    );
    let m = mutate_in_fn(
        &src,
        "pub async fn delete(",
        "if changed {\n            self.notify_changed",
        "let _e = WsEvent::EntitiesChanged;\n        if changed {\n            self.notify_changed",
    );
    assert!(!check_no_tree_reload(&m).is_empty());
}
