//! Реестровый гейт «серверная мутация -> рассылка WS-события EntitiesChanged»
//! (Phase 41.7, критерий 2 ROADMAP; D-06, D-07, D-08, D-16).
//!
//! Гейт двухслойный:
//!
//! * Слой (1) — ЭТОТ файл: реестр вердиктов (`entities_support::registry`) на
//!   КАЖДУЮ функцию области (восемь файлов `src/services/`), сканер исходников,
//!   selftest на синтетике, мутационные проверки реальных исходников, гард
//!   области. Освобождение от рассылки — только строка реестра `Exempt` с
//!   причиной не короче 40 СИМВОЛОВ (видна в диффе ревью, D-08).
//! * Слой (2) — файлы `tests/entities_changed_*.rs`, по одному на рассылающий
//!   сервис (places, groups, acts, devices, cartridges): поведенческие сценарии
//!   на настоящем `AppCtx::build`; привязка к реестру — `BEHAVIOUR_TEST`.
//!
//! Сборка гейта в два плана: план 41.7-05 (этот каркас) — реестр, фикстуры и
//! тесты формы таблицы; план 41.7-14 добавляет сканер, selftest, боевой скан,
//! мутационные проверки и гард области. Здесь они НЕ реализованы намеренно.
//!
//! Тесты каркаса:
//!   * `registry_shape_is_valid` — форма таблицы без чтения исходников;
//!   * `registry_inventory_is_not_empty` — пустой инвентарь краснит гейт.

mod entities_support;

use std::collections::BTreeSet;

use entities_support::registry::*;

/// Нижняя граница размера инвентаря: гейт, который «зелёный, потому что
/// измерил ноль», — худший режим отказа. Снято по факту: 118 строк.
const MIN_REGISTRY_ROWS: usize = 110;

fn reason_of(e: &Entry) -> Option<&'static str> {
    match e.verdict {
        Verdict::Exempt(r) => Some(r),
        _ => None,
    }
}

#[test]
fn registry_inventory_is_not_empty() {
    assert!(
        !REGISTRY.is_empty(),
        "REGISTRY пуст: гейт без инвентаря зелёный, ничего не измерив"
    );
    assert!(
        REGISTRY.len() >= MIN_REGISTRY_ROWS,
        "в REGISTRY {} строк, ожидали не меньше {MIN_REGISTRY_ROWS}: инвентарь усох",
        REGISTRY.len()
    );
    assert!(
        REGISTRY
            .iter()
            .filter(|e| e.verdict == Verdict::Broadcasts)
            .count()
            > 0,
        "в REGISTRY нет ни одной строки Broadcasts"
    );
}

#[test]
fn registry_shape_is_valid() {
    let mut problems: Vec<String> = Vec::new();

    // Пустые имена и дубли ключа (file, func).
    let mut seen: BTreeSet<(&str, &str)> = BTreeSet::new();
    for e in REGISTRY {
        if e.file.trim().is_empty() || e.func.trim().is_empty() {
            problems.push(format!("пустое имя в строке ({:?}, {:?})", e.file, e.func));
        }
        if !seen.insert((e.file, e.func)) {
            problems.push(format!("дубль ключа {}::{}", e.file, e.func));
        }
    }

    // Причины освобождений: длина в СИМВОЛАХ, не заглушки.
    for e in REGISTRY {
        if let Some(reason) = reason_of(e) {
            let n = reason.chars().count();
            if n < MIN_EXEMPT_REASON_CHARS {
                problems.push(format!(
                    "{}::{}: причина Exempt короче {MIN_EXEMPT_REASON_CHARS} символов ({n})",
                    e.file, e.func
                ));
            }
            let t = reason.trim().to_lowercase();
            if t == "todo" || t == "n/a" || t == "tbd" {
                problems.push(format!("{}::{}: причина-заглушка", e.file, e.func));
            }
        }
    }

    // Спорный update_type помечен явно (Q8).
    match REGISTRY
        .iter()
        .find(|e| e.file == "group_type_service.rs" && e.func == "update_type")
    {
        None => problems.push("нет строки group_type_service.rs::update_type".into()),
        Some(e) => match reason_of(e) {
            Some(r) if r.starts_with("СПОРНО") => {}
            _ => problems.push(
                "group_type_service.rs::update_type должна быть Exempt с причиной «СПОРНО…»".into(),
            ),
        },
    }

    // Область: ровно восемь файлов, дубль-free, у каждого есть строка,
    // каждый файл реестра — внутри области.
    if SCOPE_FILES.len() != 8 {
        problems.push(format!(
            "SCOPE_FILES: {} файлов, ожидали 8",
            SCOPE_FILES.len()
        ));
    }
    let scope: BTreeSet<&str> = SCOPE_FILES.iter().copied().collect();
    if scope.len() != SCOPE_FILES.len() {
        problems.push("SCOPE_FILES содержит дубли".into());
    }
    for f in SCOPE_FILES {
        if !REGISTRY.iter().any(|e| e.file == *f) {
            problems.push(format!(
                "в области {f}, но в реестре нет ни одной его строки"
            ));
        }
    }
    for e in REGISTRY {
        if !scope.contains(e.file) {
            problems.push(format!("{}::{}: файл вне SCOPE_FILES", e.file, e.func));
        }
    }

    // BEHAVIOUR_TEST <-> файлы со строкой Broadcasts, в обе стороны; ровно пять.
    let broadcasting: BTreeSet<&str> = REGISTRY
        .iter()
        .filter(|e| e.verdict == Verdict::Broadcasts)
        .map(|e| e.file)
        .collect();
    let mapped: BTreeSet<&str> = BEHAVIOUR_TEST.iter().map(|(f, _)| *f).collect();
    if mapped.len() != BEHAVIOUR_TEST.len() {
        problems.push("BEHAVIOUR_TEST: дубль файла сервиса".into());
    }
    for f in broadcasting.difference(&mapped) {
        problems.push(format!("{f} рассылает, но нет записи в BEHAVIOUR_TEST"));
    }
    for f in mapped.difference(&broadcasting) {
        problems.push(format!(
            "{f} в BEHAVIOUR_TEST, но не имеет строки Broadcasts"
        ));
    }
    if broadcasting.len() != 5 {
        problems.push(format!(
            "рассылающих файлов {}, ожидали ровно 5",
            broadcasting.len()
        ));
    }
    let tests: BTreeSet<&str> = BEHAVIOUR_TEST.iter().map(|(_, t)| *t).collect();
    if tests.len() != BEHAVIOUR_TEST.len() {
        problems.push("BEHAVIOUR_TEST: два сервиса указывают на один поведенческий файл".into());
    }
    for (_, t) in BEHAVIOUR_TEST {
        if !(t.starts_with("entities_changed_") && t.ends_with(".rs")) {
            problems.push(format!(
                "поведенческий файл {t} вне шаблона entities_changed_*.rs"
            ));
        }
    }

    // Allowlist гарда области: причина в СИМВОЛАХ, файл вне области.
    for (f, reason) in OUT_OF_SCOPE_WRITERS {
        if reason.chars().count() < MIN_EXEMPT_REASON_CHARS {
            problems.push(format!(
                "OUT_OF_SCOPE_WRITERS {f}: причина короче {MIN_EXEMPT_REASON_CHARS} символов"
            ));
        }
        if scope.contains(f) {
            problems.push(format!(
                "OUT_OF_SCOPE_WRITERS {f}: файл уже в области гейта"
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "реестр невалиден:\n  - {}",
        problems.join("\n  - ")
    );
}
