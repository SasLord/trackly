//! Интеграционные тесты модуля `repos::place_path_settings` (WR-08, фаза 39.2).
//!
//! Тесты синхронные и однопоточные (`#[test]`, как `seed_data.rs` /
//! `migration_idempotency.rs`), а не `#[tokio::test]`: здесь нет ни асинхронного
//! кода, ни конкуренции за writer-слот, от которых защищается timeout-обёртка в
//! `places_crud.rs`.
//!
//! **КРИТИЧНО:** ожидаемые значения задаются КОНСТАНТАМИ модуля, а не литералами.
//! Иначе тест сравнивал бы сид миграции сам с собой, и расхождение между двумя
//! законными носителями дефолта (константы модуля ↔ сид
//! `migrations/V039__place_path_display.sql`) не ловилось бы ничем, кроме
//! doc-комментария.
//!
//! Реальных данных организации в тестах нет и быть не может — только вымышленные
//! разделители вида «~» и «Здание А» в пояснениях.

use trackly_infra::repos::place_path_settings::{
    read_org_default_variant_token, read_path_display_separators, DEFAULT_SEP_ENDS,
    DEFAULT_SEP_LAST_TWO, DEFAULT_VARIANT,
};
use trackly_infra::test_support::test_db;

/// Связывает константы модуля с сидом `migrations/V039__place_path_display.sql`.
///
/// Это ЕДИНСТВЕННОЕ, что удерживает синхронность двух законных владельцев одних и
/// тех же значений. Сдвиньте `DEFAULT_SEP_ENDS` в модуле, не поправив V039 (или
/// наоборот) — тест покраснеет здесь, а не у пользователя в печатной форме.
#[test]
fn fresh_db_seed_matches_module_defaults() {
    let (conn, _dir) = test_db();

    assert_eq!(
        read_path_display_separators(&conn),
        (
            DEFAULT_SEP_ENDS.to_string(),
            DEFAULT_SEP_LAST_TWO.to_string()
        ),
        "сид V039 разошёлся с DEFAULT_SEP_ENDS/DEFAULT_SEP_LAST_TWO: у дефолта \
         формата пути два законных носителя (модуль place_path_settings и \
         миграция V039), менять надо оба"
    );
    assert_eq!(
        read_org_default_variant_token(&conn),
        DEFAULT_VARIANT,
        "сид V039 разошёлся с DEFAULT_VARIANT (см. комментарий выше)"
    );
}

/// Guarded-read: отсутствие строк в `app_settings` — не ошибка и не паника.
/// Состояние недостижимо штатным кодом (V039 всегда сеет оба ключа, а
/// `settings_set_place_path_defaults` делает upsert), но чтение обязано быть
/// мягким: это косметика отображения, из-за неё не должен падать ни список
/// устройств, ни печатная форма акта.
#[test]
fn read_separators_falls_back_when_rows_missing() {
    let (conn, _dir) = test_db();

    let deleted = conn
        .execute(
            "DELETE FROM app_settings WHERE key LIKE 'place_path_sep%'",
            [],
        )
        .expect("удалить ключи разделителей");
    assert_eq!(deleted, 2, "V039 сеет ровно два ключа разделителей");

    assert_eq!(
        read_path_display_separators(&conn),
        (
            DEFAULT_SEP_ENDS.to_string(),
            DEFAULT_SEP_LAST_TWO.to_string()
        )
    );
}

/// Тот же guarded-read для org-дефолта варианта. Это состояние БД зафиксировано
/// планом 01 как WR-02b: строка во вью `place_effective_variant` при нём есть, но
/// колонка `effective_variant` равна NULL — читатель падает именно сюда.
#[test]
fn read_org_default_variant_token_falls_back_when_key_missing() {
    let (conn, _dir) = test_db();

    let deleted = conn
        .execute(
            "DELETE FROM app_settings WHERE key = 'place_path_variant'",
            [],
        )
        .expect("удалить ключ варианта");
    assert_eq!(deleted, 1, "V039 сеет ровно одну строку place_path_variant");

    assert_eq!(read_org_default_variant_token(&conn), DEFAULT_VARIANT);
}

/// Записанное пользователем значение возвращается побайтово, включая значимые
/// пробелы по краям (D-09) — дефолт применяется только при ОТСУТСТВИИ строки,
/// а не «когда значение выглядит непривычно».
#[test]
fn stored_values_win_over_defaults_and_round_trip_byte_for_byte() {
    let (conn, _dir) = test_db();

    conn.execute(
        "UPDATE app_settings SET value = ' ~ ' WHERE key = 'place_path_sep_ends'",
        [],
    )
    .expect("записать разделитель");
    conn.execute(
        "UPDATE app_settings SET value = 'last' WHERE key = 'place_path_variant'",
        [],
    )
    .expect("записать вариант");

    let (sep_ends, sep_last_two) = read_path_display_separators(&conn);
    assert_eq!(sep_ends, " ~ ");
    assert_eq!(sep_last_two, DEFAULT_SEP_LAST_TWO);
    assert_eq!(read_org_default_variant_token(&conn), "last");
}

// ---------------------------------------------------------------------------
// Тест-сканер: гейт против возвращения дублей (WR-08)
// ---------------------------------------------------------------------------
//
// Проверяет инвариант КОДОВОЙ БАЗЫ, а не поведение — по образцу
// `crates/trackly-core/tests/no_io_deps.rs`. Без него WR-08 держался бы на
// doc-комментарии, то есть ни на чём: следующая правка спокойно заинлайнила бы
// шестую копию, и разъезд дефолтов вернулся бы молча.

/// Бывшие носители пяти копий. Список задан явно, а не обходом дерева: гейт
/// должен быть читаемым и предсказуемым, а не «что нашлось».
///
/// `crates/trackly-app/src/tauri_cmds/settings_org.rs` в списке НЕТ намеренно —
/// он легитимно читает и пишет те же три ключа одним `SELECT`/upsert'ом как API
/// настроек. За ним остаётся право знать ИМЕНА ключей, но не ЗНАЧЕНИЯ по
/// умолчанию; это проверяется отдельным утверждением ниже.
const SCANNED_SOURCES: &[&str] = &[
    "crates/trackly-infra/src/repos/devices_sqlite.rs",
    "crates/trackly-infra/src/repos/cartridges_sqlite.rs",
    "crates/trackly-infra/src/repos/places_sqlite.rs",
    "crates/trackly-app/src/services/report_service.rs",
    "crates/trackly-app/src/services/act_service.rs",
];

/// Все три ключа обязательны. `place_path_variant` — не «за компанию»:
/// пятая копия в `act_service` была именно заинлайненным fallback'ом на
/// org-дефолт ВАРИАНТА, и без этого ключа сканер её повторное появление
/// не поймал бы.
const OWNED_KEYS: &[&str] = &[
    "place_path_sep_ends",
    "place_path_sep_last_two",
    "place_path_variant",
];

fn workspace_root() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = <root>/crates/trackly-infra
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("корень workspace на два уровня выше crates/trackly-infra")
        .to_path_buf()
}

/// Строки-комментарии (`//`, `///`, `//!`) игнорируются: упоминание имени ключа
/// в пояснении легально и полезно (например, doc-комментарий
/// `act_service::compute_place_path_short` объясняет, откуда берётся org-дефолт).
/// Дублируется КОД, а не проза. Без этого фильтра гейт был бы
/// самоинвалидирующимся: честный комментарий про WR-08 сам бы его и ронял.
fn is_comment_line(line: &str) -> bool {
    line.trim_start().starts_with("//")
}

/// Чистая функция сканирования — вынесена, чтобы её саму можно было проверить
/// на «дофиксном» тексте (см. `scanner_detects_the_pre_fix_act_service_block`).
fn scan_source_for_needles(rel_path: &str, src: &str, needles: &[&str]) -> Vec<String> {
    let mut hits = Vec::new();
    for (idx, line) in src.lines().enumerate() {
        if is_comment_line(line) {
            continue;
        }
        for needle in needles {
            if line.contains(needle) {
                hits.push(format!("{rel_path}:{}: `{needle}`", idx + 1));
            }
        }
    }
    hits
}

fn read_source(rel_path: &str) -> String {
    let path = workspace_root().join(rel_path);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "не удалось прочитать {rel_path}: {e}. Файл переименован или перенесён? \
             Обнови SCANNED_SOURCES — молча пустой гейт хуже отсутствующего"
        )
    })
}

/// WR-08: у дефолтов формата пути ровно один владелец в коде.
#[test]
fn no_duplicate_path_default_owners_in_sources() {
    let mut violations = Vec::new();
    for rel in SCANNED_SOURCES {
        violations.extend(scan_source_for_needles(rel, &read_source(rel), OWNED_KEYS));
    }

    // `settings_org.rs` — задокументированное исключение по ИМЕНАМ ключей
    // (он и есть API настроек), но не по ЗНАЧЕНИЯМ: литералы-дефолты в нём
    // означали бы, что у дефолта снова два владельца. Иглы собираются из самих
    // констант, поэтому гейт следует за дефолтом, а не за вчерашним литералом.
    const SETTINGS_ORG: &str = "crates/trackly-app/src/tauri_cmds/settings_org.rs";
    let sep_ends_literal = format!("\"{DEFAULT_SEP_ENDS}\"");
    let sep_last_two_literal = format!("\"{DEFAULT_SEP_LAST_TWO}\"");
    let literals: Vec<&str> = vec![sep_ends_literal.as_str(), sep_last_two_literal.as_str()];
    violations.extend(scan_source_for_needles(
        SETTINGS_ORG,
        &read_source(SETTINGS_ORG),
        &literals,
    ));

    assert!(
        violations.is_empty(),
        "Регрессия WR-08: дефолт формата пути снова получил второго владельца.\n\
         Найдено:\n  {}\n\n\
         Единственный владелец — crates/trackly-infra/src/repos/place_path_settings.rs \
         (константы DEFAULT_VARIANT/DEFAULT_SEP_ENDS/DEFAULT_SEP_LAST_TWO и функции \
         read_path_display_separators/read_org_default_variant_token). Импортируй их \
         вместо копии.\n\
         Исключение по ИМЕНАМ ключей — только {SETTINGS_ORG}: он читает и пишет их одним \
         запросом как API настроек. Значения по умолчанию ему всё равно не принадлежат.\n\
         Второй законный носитель ЗНАЧЕНИЙ — сид migrations/V039__place_path_display.sql \
         (SQL не может импортировать константу Rust); он связан с модулем тестом \
         fresh_db_seed_matches_module_defaults.",
        violations.join("\n  ")
    );
}

/// Самотест гейта: сканер обязан краснеть на «дофиксном» тексте.
///
/// Без этого теста `no_duplicate_path_default_owners_in_sources` был бы
/// непроверяемым: зелёный сканер, который не умеет находить, выглядит ровно так
/// же, как зелёный сканер, которому нечего находить. Ниже — дословный фрагмент
/// той самой заинлайненной пятой копии из `act_service::compute_place_path_short`
/// (плюс комментарий, который обязан быть проигнорирован).
#[test]
fn scanner_detects_the_pre_fix_act_service_block() {
    let pre_fix = "\
        // fallback на org-дефолт — этот КОММЕНТАРИЙ про place_path_variant легален\n\
        .unwrap_or_else(|| {\n\
        conn.query_row(\n\
        \"SELECT value FROM app_settings WHERE key = 'place_path_variant'\",\n\
        [],\n\
        |r| r.get::<_, String>(0),\n\
        )\n\
        });\n\
        let sep_ends = read_sep(\"place_path_sep_ends\", \" ~ \");\n";

    let hits = scan_source_for_needles("<pre-fix act_service.rs>", pre_fix, OWNED_KEYS);

    assert_eq!(
        hits.len(),
        2,
        "сканер обязан найти ровно два КОДОВЫХ вхождения (строки 4 и 9) и \
         проигнорировать комментарий в строке 1; найдено: {hits:?}"
    );
    assert!(hits[0].contains("place_path_variant"));
    assert!(hits[1].contains("place_path_sep_ends"));
}
