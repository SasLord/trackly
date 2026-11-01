//! Phase 41.13 Plan 15 (D-15, D-16, T-41.13-55): запрет D-32 «сетевой принтер не может
//! входить в группу» СНЯТ и не должен вернуться.
//!
//! Обоснование D-32 путало «есть IP» с «общий»: принтер может быть сетевым и при этом
//! приписанным к одному рабочему месту, а приложение не знает, общий ли он, - решает
//! пользователь. `GroupService::add_devices` поэтому не смотрит ни на тип устройства,
//! ни на таблицу `printers`, ни на IP.
//!
//! Поведение закреплено тестами `groups_anchor.rs` (сетевой принтер входит). Этот файл -
//! страж по ИСХОДНИКУ: он ловит возврат запрета, в том числе под другим именем, когда
//! поведенческие тесты могли бы остаться зелёными по другой причине. Клиентская сторона
//! охраняется правилом [H] гейта `ui/scripts/check-device-row-menu.mjs`.
//!
//! Страж проверен мутацией: на копиях исходника в памяти (тесты (b)-(f)) и на файле на
//! диске (запись в SUMMARY плана). Ни один тест не трогает БД, async не нужен.

const GROUP_SERVICE_SRC: &str = include_str!("../src/services/group_service.rs");

/// Фраза удалённого отказа.
const BAN_PHRASE: &str = "Сетевой принтер не может входить в группу";

/// Якорь мутаций: вставка нового кода сразу после записи в состав. Обязан встречаться в
/// исходнике РОВНО один раз.
const ANCHOR: &str = "added.push(*device_id);";

/// Исходник без строк-комментариев: слова в комментариях (история решения D-32, ссылки на
/// D-16) стража не краснят.
fn strip_comments(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Тело `add_devices`: от объявления до следующей `pub async fn`.
fn add_devices_body(src: &str) -> &str {
    let start = "pub async fn add_devices(";
    let s = src
        .find(start)
        .unwrap_or_else(|| panic!("не найдено начало {start:?}"));
    let rest = &src[s + start.len()..];
    let e = rest
        .find("pub async fn ")
        .unwrap_or_else(|| panic!("не найден конец тела add_devices"));
    &rest[..e]
}

/// Страж. Возвращает `Err` с причиной (а не паникует), чтобы самопроверка могла прогнать
/// его по мутированным копиям.
fn check_guard(src: &str) -> Result<(), String> {
    let clean = strip_comments(src);
    for forbidden in [
        "is_network_printer_in_tx",
        "NETWORK_PRINTER_MESSAGE",
        BAN_PHRASE,
    ] {
        if clean.contains(forbidden) {
            return Err(format!(
                "в group_service.rs вернулось {forbidden:?}: запрет D-32 снят решением D-16"
            ));
        }
    }
    let body = add_devices_body(&clean);
    for forbidden in ["ip_address", "FROM printers", "printer"] {
        if body.to_lowercase().contains(&forbidden.to_lowercase()) {
            return Err(format!(
                "тело add_devices обращается к {forbidden:?}: членство не зависит от IP и \
                 вида устройства (D-15, D-16)"
            ));
        }
    }
    Ok(())
}

/// Вставка после якоря с проверкой, что якорь один (иначе мутация могла тронуть соседа).
fn insert_after_anchor(src: &str, insertion: &str) -> String {
    assert_eq!(
        src.matches(ANCHOR).count(),
        1,
        "якорь мутации {ANCHOR:?} неуникален или исчез"
    );
    let mutated = src.replacen(ANCHOR, &format!("{ANCHOR}\n{insertion}"), 1);
    assert_ne!(mutated, src, "мутация ничего не изменила");
    mutated
}

// ---------------------------------------------------------------------------
// (a) страж по исходнику
// ---------------------------------------------------------------------------

#[test]
fn group_service_has_no_network_printer_ban() {
    if let Err(why) = check_guard(GROUP_SERVICE_SRC) {
        panic!("{why}");
    }
}

// ---------------------------------------------------------------------------
// (b)-(f) мутанты на копиях в памяти
// ---------------------------------------------------------------------------

#[test]
fn guard_selfcheck_clean_copy_is_green_and_anchor_is_unique() {
    assert_eq!(GROUP_SERVICE_SRC.matches(ANCHOR).count(), 1);
    assert_eq!(
        add_devices_body(GROUP_SERVICE_SRC).matches(ANCHOR).count(),
        1,
        "якорь обязан лежать внутри add_devices"
    );
    assert!(check_guard(GROUP_SERVICE_SRC).is_ok());
    // Контроль: слова в комментарии стража не краснят.
    let commented = insert_after_anchor(
        GROUP_SERVICE_SRC,
        "// is_network_printer_in_tx NETWORK_PRINTER_MESSAGE ip_address FROM printers",
    );
    assert!(
        check_guard(&commented).is_ok(),
        "комментарий не должен краснить страж"
    );
}

#[test]
fn guard_selfcheck_detects_returned_helper_call() {
    // (b) возвращены функция-проверка и её вызов.
    let mutated = insert_after_anchor(
        GROUP_SERVICE_SRC,
        "if is_network_printer_in_tx(&tx, *device_id)? { \
         return Err(device_ids_error(String::new())); }",
    );
    assert!(
        check_guard(&mutated).is_err(),
        "страж не заметил возвращённую проверку is_network_printer_in_tx"
    );
}

#[test]
fn guard_selfcheck_detects_inline_sql_on_printers() {
    // (c) инлайн-запрос про IP принтера прямо в теле add_devices.
    let mutated = insert_after_anchor(
        GROUP_SERVICE_SRC,
        "let _n: i64 = tx.query_row(\
         \"SELECT COUNT(*) FROM printers WHERE device_id = ?1 AND ip_address IS NOT NULL\", \
         rusqlite::params![*device_id], |r| r.get(0)).map_err(map_rusqlite)?;",
    );
    assert!(
        check_guard(&mutated).is_err(),
        "страж не заметил инлайн-SQL по printers"
    );
}

#[test]
fn guard_selfcheck_detects_returned_message_constant() {
    // (d) возвращена константа с фразой отказа.
    let mutated =
        format!("{GROUP_SERVICE_SRC}\nconst NETWORK_PRINTER_MESSAGE: &str = \"{BAN_PHRASE}\";\n");
    assert_ne!(mutated, GROUP_SERVICE_SRC);
    assert!(
        check_guard(&mutated).is_err(),
        "страж не заметил возвращённую константу"
    );
    // Одна фраза без имён константы и функции тоже краснит.
    let phrase_only = format!("{GROUP_SERVICE_SRC}\nconst X: &str = \"{BAN_PHRASE}: ...\";\n");
    assert!(
        check_guard(&phrase_only).is_err(),
        "страж не заметил фразу отказа под другим именем"
    );
}

#[test]
fn guard_selfcheck_detects_ban_under_another_name() {
    // (e) запрет под другим именем: хелпер вне тела, в теле только вызов. Имя без слов
    // из (b); ловит правило про слово «printer» в теле add_devices.
    let mutated = insert_after_anchor(
        GROUP_SERVICE_SRC,
        "if is_shared_printer_in_tx(&tx, *device_id)? { \
         return Err(device_ids_error(String::new())); }",
    );
    assert!(
        check_guard(&mutated).is_err(),
        "страж не заметил запрет под другим именем"
    );
}

#[test]
fn guard_selfcheck_detects_ban_placed_before_the_anchor() {
    // (f) ПЕРЕСТАНОВКА: проверка вставлена не после якоря, а перед первой строкой тела
    // цикла (до записи в состав). Страж не зависит от порядка и обязан краснеть.
    // Цикл по устройствам встречается в файле дважды; берём тот, что внутри add_devices.
    let loop_head = "for device_id in &device_ids {";
    let body = add_devices_body(GROUP_SERVICE_SRC);
    assert_eq!(
        body.matches(loop_head).count(),
        1,
        "якорь цикла неуникален внутри add_devices: {loop_head:?}"
    );
    let body_off = body.as_ptr() as usize - GROUP_SERVICE_SRC.as_ptr() as usize;
    let loop_off = body_off + body.find(loop_head).expect("цикл") + loop_head.len();
    let mut mutated = String::from(GROUP_SERVICE_SRC);
    mutated.insert_str(
        loop_off,
        "\n if is_network_printer_in_tx(&tx, *device_id)? { return Err(device_ids_error(String::new())); }",
    );
    assert_ne!(mutated, GROUP_SERVICE_SRC);
    assert!(
        check_guard(&mutated).is_err(),
        "страж не заметил проверку перед циклом"
    );
}
