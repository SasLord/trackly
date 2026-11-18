//! Гейт области `plugins.shell.open` в tauri.conf.json (UAT 41.5, тест 10, T-41.5-73).
//!
//! Читает настоящий конфиг и проверяет регэксп на позитивных и негативных адресах:
//! разрешены только http(s)-ссылки и файлы печати `trackly-print-N.html`.

use regex::Regex;

/// Прежний регэксп (до плана 41.5-20): пропускал только файлы печати.
const OLD_SCOPE: &str = r"^(https?://|file://|/|[A-Za-z]:[\\/]).*trackly-print-\d+\.html$";

const ACCEPT: &[&str] = &[
    "https://example.com",
    "http://example.com/a?b=1&c=2#x",
    "https://example.com:8443/путь",
    r"C:\Users\x\AppData\Local\Temp\trackly-print-12.html",
    "/var/folders/ab/T/trackly-print-3.html",
    "file:///tmp/trackly-print-7.html",
];

const REJECT: &[&str] = &[
    "javascript:alert(1)",
    "data:text/html,x",
    "file:///etc/passwd",
    "ftp://example.com/x",
    "mailto:a@example.com",
    "https://",
    "https://a b",
    "HTTPS://",
    "",
    r"C:\Windows\System32\calc.exe",
];

/// Список нарушений: принимаемое, не принятое, и отклоняемое, но принятое.
fn violations(re: &Regex) -> Vec<String> {
    let mut v = Vec::new();
    for s in ACCEPT {
        if !re.is_match(s) {
            v.push(format!("должен приниматься, но отклонён: {s:?}"));
        }
    }
    for s in REJECT {
        if re.is_match(s) {
            v.push(format!("должен отклоняться, но принят: {s:?}"));
        }
    }
    v
}

fn real_scope() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tauri.conf.json");
    let raw = std::fs::read_to_string(path).expect("tauri.conf.json читается");
    let json: serde_json::Value = serde_json::from_str(&raw).expect("tauri.conf.json — JSON");
    json.pointer("/plugins/shell/open")
        .and_then(|v| v.as_str())
        .expect("plugins.shell.open должен быть строкой-регэкспом (не true и не отсутствовать)")
        .to_string()
}

#[test]
fn shell_open_scope_allows_http_and_print_only() {
    assert!(
        ACCEPT.len() >= 6 && REJECT.len() >= 9,
        "набор примеров не вакуумный"
    );
    let scope = real_scope();
    let re = Regex::new(&scope).expect("регэксп области компилируется");
    let v = violations(&re);
    assert!(v.is_empty(), "область shell.open нарушена: {v:#?}");
}

#[test]
fn old_scope_is_detected_as_broken() {
    // Мутационное доказательство: набор проверок обязан краснеть на старом регэкспе.
    let re = Regex::new(OLD_SCOPE).unwrap();
    let v = violations(&re);
    assert!(
        v.iter().any(|m| m.contains("https://example.com\"")),
        "старый регэксп НЕ принимает https — набор обязан это ловить: {v:#?}"
    );
}
