//! Формат отображения пути места: **единственный владелец значений по умолчанию**
//! и единственная точка чтения ключей `app_settings.place_path_*` (WR-08, фаза 39.2).
//!
//! # Зачем модуль существует
//!
//! До этой фазы `read_path_display_separators` была скопирована дословно четыре раза
//! (`repos/devices_sqlite.rs`, `repos/cartridges_sqlite.rs`, `repos/places_sqlite.rs`,
//! `trackly-app/src/services/report_service.rs`), пятая копия была заинлайнена в
//! `act_service::compute_place_path_short`, а сами литералы-дефолты жили ещё и в
//! `tauri_cmds/settings_org.rs::build_settings_get_place_path_defaults`. Комментарии
//! называли это «конвенцией фазы 39.1», но конвенцией здесь было **отсутствие
//! владельца**: любой будущий сдвиг дефолта почти наверняка обновил бы не все копии —
//! и пользователь увидел бы разные разделители в списках, отчётах и печатных формах.
//!
//! Поэтому: **любая новая копия `read_path_display_separators` или литералов
//! `" // "` / `" / "` / `"ends"` в роли дефолта — регрессия WR-08.** Это не просьба,
//! а проверяемое утверждение: тест-сканер `no_duplicate_path_default_owners_in_sources`
//! (`crates/trackly-infra/tests/place_path_settings.rs`) читает исходники пяти бывших
//! копий и краснеет, как только ключ настройки снова появится в коде вне этого модуля.
//!
//! # Второй законный носитель тех же значений
//!
//! `migrations/V039__place_path_display.sql` засеивает ровно эти три значения в
//! `app_settings`. Это не дубль-регрессия, а неизбежность: миграция — SQL, она не
//! может импортировать константу Rust. При сдвиге дефолта менять надо **оба** места.
//! Расхождение «константа ↔ сид V039» ловится автоматически тестом
//! `fresh_db_seed_matches_module_defaults`, который сравнивает прочитанное из свежей
//! БД с константами ниже, — а не только этим комментарием.
//!
//! # Почему модуль в `trackly-infra`, а не в `trackly-core`
//!
//! Обе функции принимают `rusqlite::Connection`, а гейт
//! `crates/trackly-core/tests/no_io_deps.rs` держит `rusqlite` в списке крейтов,
//! запрещённых в ядре (гексагональная граница, FOUND-01). Чистая доменная часть —
//! `PathDisplayVariant` и `shorten_place_path` — как и прежде живёт в
//! `trackly_core::domain::places`; здесь только чтение настроек из БД.
//!
//! # Форма чтения
//!
//! Чтение «мягкое» (guarded): отсутствие строки в `app_settings` — не ошибка, а
//! возврат константы-дефолта. Ни одна из этих настроек не критична: это косметика
//! отображения (например, «Здание А // 1-05» вместо «Здание А / 1 этаж / 1-05»), и
//! ни список устройств, ни печатная форма акта не должны падать из-за неё.
//! Вызывать один раз на запрос, а не на строку результата.

use rusqlite::Connection;

/// Организационный дефолт варианта сокращения пути
/// (`app_settings.place_path_variant`). Токен разбирается
/// `trackly_core::domain::places::PathDisplayVariant::from_str`.
///
/// Обязан совпадать с сидом `migrations/V039__place_path_display.sql`
/// (проверяется тестом `fresh_db_seed_matches_module_defaults`).
pub const DEFAULT_VARIANT: &str = "ends";

/// Разделитель варианта «Крайние» (`app_settings.place_path_sep_ends`).
///
/// Пробелы по краям значимы (D-09): значение обязано round-trip'иться побайтово,
/// поэтому здесь нельзя «прибрать» пробелы ни в коде, ни в сиде миграции.
pub const DEFAULT_SEP_ENDS: &str = " // ";

/// Разделитель вариантов «Последние два» / «Последнее»
/// (`app_settings.place_path_sep_last_two`). Пробелы по краям значимы (D-09).
pub const DEFAULT_SEP_LAST_TWO: &str = " / ";

/// Guarded-read одного ключа `app_settings`: `Ok(value)` → `Some`, любая ошибка
/// (нет строки, нет таблицы, не тот тип) → `None`. Запрос параметризован —
/// значение ключа никогда не конкатенируется в SQL.
fn read_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        [key],
        |r| r.get::<_, String>(0),
    )
    .ok()
}

/// Читает пару организационных разделителей `(sep_ends, sep_last_two)`.
///
/// Отсутствующая строка `app_settings` не ошибка — возвращается
/// [`DEFAULT_SEP_ENDS`] / [`DEFAULT_SEP_LAST_TWO`]. Это защитная ветка: миграция
/// V039 всегда сеет оба ключа.
pub fn read_path_display_separators(conn: &Connection) -> (String, String) {
    (
        read_setting(conn, "place_path_sep_ends").unwrap_or_else(|| DEFAULT_SEP_ENDS.to_string()),
        read_setting(conn, "place_path_sep_last_two")
            .unwrap_or_else(|| DEFAULT_SEP_LAST_TWO.to_string()),
    )
}

/// Читает организационный дефолт варианта сокращения пути как СЫРОЙ токен.
///
/// Возвращает [`DEFAULT_VARIANT`], если ключа нет. Токен намеренно не
/// разбирается здесь: у вызывающих разная политика деградации на нераспознанном
/// значении (акт откатывается к `Ends`, чтобы не потерять поле-строку), а модуль
/// не должен навязывать одну из них.
///
/// Внимание: это дефолт **организации**. Per-place override
/// (`places.path_variant_override`, вью `place_effective_variant`) читается
/// вызывающим отдельно и имеет приоритет — сюда падают только тогда, когда
/// строки во вью нет.
pub fn read_org_default_variant_token(conn: &Connection) -> String {
    read_setting(conn, "place_path_variant").unwrap_or_else(|| DEFAULT_VARIANT.to_string())
}
