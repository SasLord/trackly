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
