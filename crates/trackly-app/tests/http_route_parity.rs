//! Сквозной гейт «команда Tauri ↔ HTTP-маршрут» (пробел G1 аудита фазы 41).
//!
//! ЗАЧЕМ. Фронтенд (`ui/src/lib/api/client.ts`) — один вызов `apiCall(name, args)`
//! на два транспорта: в десктопе `invoke(name)`, в LAN-браузере
//! `POST /api/v1/<name>`. Имя команды И имя маршрута — одна и та же строка.
//! Поэтому команда без маршрута ломает ровно тот же экран, но только по LAN.
//! Так и случилось со счётчиками «Отчётов»: `reports_get_report_counts`
//! зарегистрирована в Tauri, маршрута нет, а `spa_fallback`
//! (`src/http/mod.rs`) отдаёт на незарегистрированный `/api/v1/*` сам
//! `index.html` с HTTP 200 и `text/html` — отказ выглядит как успех. Дыра
//! прожила с фазы 28 и вскрылась только живой приёмкой фазы 41. Имевшиеся
//! тесты полноты маршрутов (`role_endpoint_matrix.rs`:
//! `group_types_http_route_completeness`, `groups_http_route_completeness`)
//! скоупнуты на `src/http/group_types.rs` и `src/http/groups.rs` и пропуск в
//! любом другом модуле увидеть не могут — это и был источник ложной
//! уверенности.
//!
//! ИСТОЧНИКИ ИСТИНЫ — читаются ОТ ИСХОДНИКОВ при каждом прогоне (никакого
//! вкомпилированного списка ожидаемых имён; урок «инвентарь от репозитория»):
//!
//!   C. Команды Tauri — `collect_commands![...]` в `src/specta_export.rs`.
//!      Это единственная точка регистрации: тот же `Builder` отдаёт и
//!      `invoke_handler` для рантайма, и `ui/src/bindings.ts`. Значит именно
//!      этот список — API-поверхность, которую видит фронтенд.
//!      Сейчас: 184 записи. Почему НЕ `pub async fn build_*` по
//!      `src/tauri_cmds/*.rs` (их 181): это хелперы, а не команды, и
//!      соответствие не один-к-одному — 16 команд идут через
//!      `build_<name>_tauri` (обёртка, разрешающая desktop-identity), ещё
//!      3 (`app_restart`, `auth_logout`, `auth_me`) хелпера не имеют вовсе,
//!      зато `build_reports_get_report_counts` есть без HTTP-вызывающей
//!      стороны. Число 184 из записи UAT — это и есть `collect_commands!`.
//!
//!   R. HTTP-маршруты — вызовы `.route("/api/v1/<name>", ...)` по
//!      `src/http/*.rs`. Сейчас: 185 вызовов, 183 уникальных пути
//!      (`auth_login` и `auth_status` регистрируются дважды: в
//!      `http/auth.rs::public_router` и отдельными роутерами в
//!      `http/mod.rs`, где к login прикручен rate-limit). Регистрация, а не
//!      любой строковый литерал: литералы ловят ещё и allowlist публичных
//!      путей в `http/mod.rs` и URI внутри `#[cfg(test)]`.
//!
//!   L. Живой роутер — `http::build_router()` на настоящем `AppCtx`. Нужен
//!      потому, что пункты C и R — разбор текста: маршрут, объявленный в
//!      функции-роутере, которую забыли смерджить в `build_router`, пройдёт
//!      проверку R и всё равно не ответит. Пример живого риска:
//!      `http::auth::public_router()` в `build_router` НЕ мерджится.
//!
//! НЕ ВАКУУМЕН. Разбор пустым не бывает незаметно: нижние границы на оба
//! инвентаря (≥150) падают при любой поломке регулярок. Множества
//! расхождений сверяются ТОЧНО с таблицами вердиктов ниже — и лишняя
//! позиция, и исчезнувшая красят гейт. Живой тест сравнивает ответ не с
//! ожидаемым кодом, а с контрольным ответом на заведомо несуществующий путь,
//! поэтому он останется рабочим и после починки `spa_fallback`.
//!
//! Данные в тесте отсутствуют: гейт читает исходники и бьёт по пустым телам.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;

// ---------------------------------------------------------------------------
// Таблицы вердиктов
// ---------------------------------------------------------------------------

struct Verdict {
    name: &'static str,
    /// Почему позиция допустима (или почему она здесь временно).
    reason: &'static str,
}

/// Команды Tauri, у которых HTTP-маршрута НЕТ. Каждая — с вердиктом.
/// Множество сверяется точно: новая команда без маршрута красит гейт.
const CMD_WITHOUT_ROUTE: &[Verdict] = &[
    Verdict {
        name: "app_restart",
        // Проверено по исходнику UI: единственный вызов —
        // ui/src/features/settings/StorageSettings.svelte:70, внутри
        // proceedWithMove(), после раннего выхода по `isTauri`
        // (строки 41-47: в браузере тост «доступно только в
        // десктоп-приложении» и return). По LAN до вызова не доходит.
        reason: "десктопная по смыслу (перезапуск процесса); в UI закрыта проверкой isTauri \
                 в StorageSettings.svelte:41-47 перед единственным вызовом на строке 70",
    },
    Verdict {
        name: "settings_move_db",
        // Тот же блок: вызов на StorageSettings.svelte:69, за тем же
        // `isTauri`-гардом, плюс нативный диалог @tauri-apps/plugin-dialog,
        // которого в браузере нет в принципе.
        reason: "десктопная по смыслу (перенос файла БД через нативный диалог); в UI закрыта \
                 проверкой isTauri в StorageSettings.svelte:41-47 перед вызовом на строке 69",
    },
    Verdict {
        name: "settings_open_db_folder",
        // ВНИМАНИЕ: вердикт отличается от двух предыдущих. Вызов
        // openFolder() (StorageSettings.svelte:31) `isTauri`-гарда НЕ имеет,
        // и кнопка «Открыть папку с базой данных» рисуется в браузере тоже.
        // Освобождение держится на другом: открыть каталог можно только на
        // машине сервера, для LAN-клиента операция бессмысленна, а отказ
        // виден — catch показывает тост «Не удалось открыть папку»
        // (в отличие от пустого .catch() у счётчиков отчётов, который и
        // спрятал дефект). Маршрут не нужен; гард в UI — желательная
        // доработка, не дефект транспорта.
        reason: "десктопная по смыслу (открыть каталог на машине сервера); в UI НЕ закрыта \
                 isTauri (StorageSettings.svelte:29-39 вызывает без гарда), но отказ виден \
                 пользователю тостом «Не удалось открыть папку»",
    },
    Verdict {
        name: "reports_get_report_counts",
        // ИЗВЕСТНЫЙ ДЕФЕКТ, отложен решением пользователя 2026-10-06:
        // .planning/phases/41-groups-model-and-editor/deferred-items.md,
        // раздел «UAT R5 — счётчики отчётов не работают по LAN». Вне границ
        // фазы 41 (счётчики введены в фазе 28).
        //
        // КОГДА МАРШРУТ ДОБАВЯТ — УДАЛИТЬ ЭТУ СТРОКУ. Гейт специально
        // покраснеет («освобождение больше не нужно»), чтобы заставить это
        // сделать и не оставить мёртвое освобождение в таблице.
        reason: "ИЗВЕСТНЫЙ ДЕФЕКТ (не освобождение): случайный пропуск маршрута, из-за которого \
                 счётчики «Отчётов» по LAN показывают 0. Отложен решением пользователя \
                 2026-10-06, см. 41/deferred-items.md «UAT R5». При добавлении маршрута \
                 /api/v1/reports_get_report_counts эту строку УДАЛИТЬ",
    },
];

/// HTTP-маршруты `/api/v1/*`, которым не соответствует команда Tauri.
/// Обратное направление: ловит мёртвые и только-браузерные маршруты.
const ROUTE_WITHOUT_CMD: &[Verdict] = &[
    Verdict {
        name: "auth_ad_sso",
        reason: "только браузерный транспорт: SPNEGO/Kerberos-негоциация делается самим \
                 браузером через GET (http/sso.rs::router), командой invoke не выражается — \
                 вызывается напрямую fetch'ем (ui/src/lib/api/adSso.ts:53, LoginPage.svelte:55)",
    },
    Verdict {
        name: "ws",
        reason: "WebSocket-upgrade для режима сервера (http/ws.rs), method any(); в десктопе \
                 инвалидация идёт внутрипроцессными событиями, команды Tauri здесь нет",
    },
    Verdict {
        name: "users_reset_password",
        reason: "HTTP-only: handler и build_users_reset_password есть в http/users.rs, но ни \
                 команды Tauri, ни вызывающей стороны в ui/src нет — админский сброс пароля \
                 доступен только по HTTP-транспорту. Направление безвредное (экран не ломает), \
                 фиксируется, чтобы паритет не расходился молча",
    },
];

/// Нижние границы: защита от «регулярка перестала совпадать → инвентарь пуст
/// → гейт зелёный». Намеренно сильно ниже фактических 184/183.
const MIN_COMMANDS: usize = 150;
const MIN_ROUTES: usize = 150;
/// Нижняя граница на число просканированных файлов в каждом каталоге.
const MIN_SOURCE_FILES: usize = 15;

fn names(list: &[Verdict]) -> BTreeSet<String> {
    list.iter().map(|v| v.name.to_string()).collect()
}

// ---------------------------------------------------------------------------
// Инвентари ОТ ИСХОДНИКОВ
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Все `.rs` в каталоге (без рекурсии — подкаталогов в `src/http` и
/// `src/tauri_cmds` нет; появятся — упадёт нижняя граница инвентаря).
fn read_rs_dir(rel: &str) -> Vec<(String, String)> {
    let dir: PathBuf = crate_root().join(rel);
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("не читается каталог {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let src = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("не читается {}: {e}", path.display()));
            let label = format!(
                "{rel}/{}",
                path.file_name().and_then(|n| n.to_str()).unwrap_or("?")
            );
            out.push((label, src));
        }
    }
    assert!(
        out.len() >= MIN_SOURCE_FILES,
        "в {} найдено {} .rs-файлов, ожидалось минимум {} — разбор инвентаря сломан, \
         гейт нельзя считать пройденным",
        dir.display(),
        out.len(),
        MIN_SOURCE_FILES
    );
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// C: команды Tauri из `collect_commands![...]` в `src/specta_export.rs`.
fn tauri_commands() -> BTreeSet<String> {
    let path: PathBuf = crate_root().join("src/specta_export.rs");
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("не читается {}: {e}", path.display()));

    // Срез строго внутри макроса: иначе упоминание пути в doc-комментарии
    // где-то ещё в файле попало бы в инвентарь.
    let start = src
        .find("collect_commands![")
        .unwrap_or_else(|| panic!("в {} нет collect_commands![", path.display()));
    let rest = &src[start..];
    let end = rest.find("\n    ])").unwrap_or_else(|| {
        panic!(
            "в {} не найден конец блока collect_commands!",
            path.display()
        )
    });
    let block = &rest[..end];

    let re = regex::Regex::new(r"crate::tauri_cmds::(\w+)::(\w+)").expect("regex");
    let cmds: BTreeSet<String> = re.captures_iter(block).map(|c| c[2].to_string()).collect();

    // Разбор обязан быть исчерпывающим: каждая непустая не-комментарий
    // строка блока должна быть распознанной записью команды.
    for line in block.lines().skip(1) {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        assert!(
            t.starts_with("crate::tauri_cmds::"),
            "строка внутри collect_commands! не разобрана как команда: {t:?} — инвентарь \
             команд неполон, гейт нельзя считать пройденным"
        );
    }

    assert!(
        cmds.len() >= MIN_COMMANDS,
        "из collect_commands! извлечено всего {} команд (минимум {}) — разбор сломан",
        cmds.len(),
        MIN_COMMANDS
    );
    cmds
}

/// R: зарегистрированные пути `/api/v1/<name>` по `src/http/*.rs`.
/// `\s*` в регулярке покрывает многострочный `.route(\n "…",\n …)`.
fn http_routes() -> BTreeSet<String> {
    let re = regex::Regex::new(r#"\.route\(\s*"/api/v1/(\w+)""#).expect("regex");
    let mut routes = BTreeSet::new();
    let mut raw = 0usize;
    for (_label, src) in read_rs_dir("src/http") {
        for c in re.captures_iter(&src) {
            routes.insert(c[1].to_string());
            raw += 1;
        }
    }
    assert!(
        routes.len() >= MIN_ROUTES,
        "из src/http/*.rs извлечено всего {} маршрутов (минимум {}; сырых совпадений {}) — \
         разбор сломан",
        routes.len(),
        MIN_ROUTES,
        raw
    );
    routes
}

/// Хелперы `pub async fn build_*` в `src/tauri_cmds/*.rs` с указанием файла.
fn tauri_build_helpers() -> Vec<(String, String)> {
    let re = regex::Regex::new(r"pub async fn (build_\w+)").expect("regex");
    let mut out = Vec::new();
    for (label, src) in read_rs_dir("src/tauri_cmds") {
        for c in re.captures_iter(&src) {
            out.push((c[1].to_string(), label.clone()));
        }
    }
    assert!(
        out.len() >= MIN_COMMANDS,
        "в src/tauri_cmds/*.rs найдено всего {} хелперов build_* (минимум {}) — разбор сломан",
        out.len(),
        MIN_COMMANDS
    );
    out
}

fn http_sources_concat() -> String {
    read_rs_dir("src/http")
        .into_iter()
        .map(|(_, s)| s)
        .collect::<Vec<_>>()
        .join("\n")
}

fn fmt_set(set: &BTreeSet<String>) -> String {
    if set.is_empty() {
        "<пусто>".to_string()
    } else {
        set.iter().cloned().collect::<Vec<_>>().join(", ")
    }
}

// ---------------------------------------------------------------------------
// 1. Команда Tauri → HTTP-маршрут (то направление, которое ломает LAN)
// ---------------------------------------------------------------------------

#[test]
fn every_tauri_command_is_reachable_over_http() {
    let cmds = tauri_commands();
    let routes = http_routes();

    let missing: BTreeSet<String> = cmds.difference(&routes).cloned().collect();
    let exempt = names(CMD_WITHOUT_ROUTE);

    let unexpected: BTreeSet<String> = missing.difference(&exempt).cloned().collect();
    assert!(
        unexpected.is_empty(),
        "команды Tauri без HTTP-маршрута /api/v1/<name>, не внесённые в CMD_WITHOUT_ROUTE: {}.\n\
         Фронтенд (ui/src/lib/api/client.ts) по LAN делает POST /api/v1/<имя команды>: без \
         маршрута экран молча получает index.html вместо данных (ровно дефект счётчиков \
         «Отчётов»). Либо добавьте маршрут в src/http/<модуль>.rs, либо внесите позицию в \
         CMD_WITHOUT_ROUTE с вердиктом, почему она десктопная по смыслу.\n\
         Всего команд: {}, маршрутов: {}.",
        fmt_set(&unexpected),
        cmds.len(),
        routes.len()
    );

    let stale: BTreeSet<String> = exempt.difference(&missing).cloned().collect();
    assert!(
        stale.is_empty(),
        "в CMD_WITHOUT_ROUTE остались позиции, у которых маршрут уже есть (или команда \
         удалена): {}. Удалите их из таблицы — мёртвое освобождение прячет следующий пропуск.\n\
         Для reports_get_report_counts это ОЖИДАЕМОЕ падение: маршрут добавлен, долг \
         41/deferred-items.md «UAT R5» закрыт, строку из таблицы надо убрать.",
        fmt_set(&stale)
    );
}

// ---------------------------------------------------------------------------
// 2. HTTP-маршрут → команда Tauri (обратное направление)
// ---------------------------------------------------------------------------

#[test]
fn every_http_api_route_maps_to_a_tauri_command() {
    let cmds = tauri_commands();
    let routes = http_routes();

    let orphan: BTreeSet<String> = routes.difference(&cmds).cloned().collect();
    let exempt = names(ROUTE_WITHOUT_CMD);

    let unexpected: BTreeSet<String> = orphan.difference(&exempt).cloned().collect();
    assert!(
        unexpected.is_empty(),
        "маршруты /api/v1/* без одноимённой команды Tauri, не внесённые в ROUTE_WITHOUT_CMD: \
         {}. Либо зарегистрируйте команду в src/specta_export.rs (иначе в десктопе тот же \
         экран не работает), либо внесите позицию в ROUTE_WITHOUT_CMD с вердиктом.",
        fmt_set(&unexpected)
    );

    let stale: BTreeSet<String> = exempt.difference(&orphan).cloned().collect();
    assert!(
        stale.is_empty(),
        "в ROUTE_WITHOUT_CMD остались позиции, которые больше не осиротевшие (команда \
         появилась или маршрут удалён): {}. Удалите их из таблицы.",
        fmt_set(&stale)
    );
}

// ---------------------------------------------------------------------------
// 3. «Один DTO, два транспорта»: хелпер без HTTP-вызывающей стороны
// ---------------------------------------------------------------------------

/// Прямая проверка той аномалии, которую вскрыла приёмка: хелпер
/// `build_reports_get_report_counts` помечен «callable from both Tauri and
/// HTTP», а HTTP-вызывающей стороны не существует. Имя маршрута может
/// разойтись с именем хелпера, поэтому проверка идёт по ссылке на
/// идентификатор в `src/http/*.rs`, а не по соглашению об именах.
#[test]
fn every_dual_transport_build_helper_has_an_http_caller() {
    let helpers = tauri_build_helpers();
    let http_src = http_sources_concat();
    let exempt = names(CMD_WITHOUT_ROUTE);

    let mut unreferenced: Vec<(String, String)> = Vec::new();
    for (helper, file) in &helpers {
        // Обёртки `build_<name>_tauri` разрешают desktop-identity и по
        // смыслу живут только в Tauri-транспорте; HTTP-сторона имеет свой
        // хелпер в src/http/<модуль>.rs. Освобождение не на веру: ниже
        // отдельно проверяется, что парный HTTP-хелпер существует.
        if helper.ends_with("_tauri") {
            continue;
        }
        let core = helper.strip_prefix("build_").unwrap_or(helper);
        if exempt.contains(core) {
            continue;
        }
        let re = regex::Regex::new(&format!(r"\b{}\b", regex::escape(helper))).expect("regex");
        if !re.is_match(&http_src) {
            unreferenced.push((helper.clone(), file.clone()));
        }
    }

    assert!(
        unreferenced.is_empty(),
        "хелперы build_* без единой ссылки из src/http/*.rs (объявлены как общие для двух \
         транспортов, но HTTP-вызывающей стороны нет): {:?}.\n\
         Именно так выглядел build_reports_get_report_counts — помечен «callable from both \
         Tauri and HTTP» и ни разу не вызван из http/.",
        unreferenced
    );

    // Освобождение `_tauri` закодировано, а не принято на веру: у каждой
    // обёртки должен быть парный HTTP-хелпер (`build_X` или `build_X_http`).
    let http_helpers: BTreeSet<String> = regex::Regex::new(r"pub async fn (build_\w+)")
        .expect("regex")
        .captures_iter(&http_src)
        .map(|c| c[1].to_string())
        .collect();
    let wrappers: Vec<&String> = helpers
        .iter()
        .map(|(h, _)| h)
        .filter(|h| h.ends_with("_tauri"))
        .collect();
    assert!(
        !wrappers.is_empty(),
        "не найдено ни одной обёртки build_*_tauri — разбор сломан, освобождение по суффиксу \
         нельзя считать проверенным"
    );
    let mut orphan_wrappers: Vec<String> = Vec::new();
    for w in wrappers {
        let core = w.trim_end_matches("_tauri");
        if !http_helpers.contains(core) && !http_helpers.contains(&format!("{core}_http")) {
            orphan_wrappers.push(w.clone());
        }
    }
    assert!(
        orphan_wrappers.is_empty(),
        "обёртки build_*_tauri без парного HTTP-хелпера ({core} или {core}_http) в \
         src/http/*.rs: {:?} — освобождение по суффиксу _tauri для них недействительно",
        orphan_wrappers,
        core = "build_X"
    );
}

// ---------------------------------------------------------------------------
// 4. Живой роутер: разбор исходника ещё не значит, что маршрут отвечает
// ---------------------------------------------------------------------------

/// Поведенческая проверка на настоящем `build_router()`: по каждому имени
/// команды бьём `POST /api/v1/<name>` ровно так, как это делает
/// `apiCall` в браузере, и требуем, чтобы ответ отличался от ответа на
/// заведомо несуществующий путь.
///
/// Эталон берётся из самого прогона (две разные фиктивные ручки должны дать
/// идентичное тело), а не из ожидаемого кода, — поэтому тест остаётся
/// рабочим и после починки `spa_fallback` на 404 JSON.
///
/// Тест ловит то, чего разбор текста в тесте 1 увидеть не может: маршрут,
/// объявленный в функции-роутере, которую забыли смерджить в `build_router`
/// (живой пример такой функции — `http::auth::public_router()`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_router_answers_every_tauri_command_path() {
    let cmds = tauri_commands();
    let exempt = names(CMD_WITHOUT_ROUTE);

    let dir = tempfile::TempDir::new().expect("tempdir");
    let paths =
        trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("resolve paths");
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config)
        .or_else(|_| {
            let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
            Ok::<_, anyhow::Error>(guard)
        })
        .expect("log guard");
    let ctx = trackly_app::context::AppCtx::build(paths, config, log_guard)
        .await
        .expect("build ctx");
    let app = build_router(
        &ctx,
        RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
    );

    async fn probe(app: &axum::Router, name: &str) -> (u16, String, Vec<u8>) {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/{name}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("router response");
        let status = res.status().as_u16();
        let ct = res
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("body")
            .to_vec();
        (status, ct, body)
    }

    // Эталон «маршрута нет»: два разных заведомо незарегистрированных имени.
    let control_a = probe(&app, "zzz_route_parity_control_alpha").await;
    let control_b = probe(&app, "zzz_route_parity_control_beta").await;
    assert_eq!(
        (control_a.0, &control_a.1, &control_a.2),
        (control_b.0, &control_b.1, &control_b.2),
        "ответ на незарегистрированный /api/v1/* зависит от самого пути — эталон \
         «маршрута нет» построить нельзя, сравнение ниже недействительно"
    );
    assert!(
        !control_a.2.is_empty(),
        "эталонный ответ на незарегистрированный путь пуст — сравнение по телу выродилось"
    );

    let mut no_route: BTreeSet<String> = BTreeSet::new();
    for name in &cmds {
        let got = probe(&app, name).await;
        if (got.0, &got.1, &got.2) == (control_a.0, &control_a.1, &control_a.2) {
            no_route.insert(name.clone());
        }
    }

    ctx.shutdown.cancel();

    let unexpected: BTreeSet<String> = no_route.difference(&exempt).cloned().collect();
    assert!(
        unexpected.is_empty(),
        "живой build_router() отвечает на эти команды так же, как на несуществующий путь \
         (status {}, content-type {:?}) — маршрута фактически нет: {}.\n\
         Проверено {} команд.",
        control_a.0,
        control_a.1,
        fmt_set(&unexpected),
        cmds.len()
    );

    let stale: BTreeSet<String> = exempt.difference(&no_route).cloned().collect();
    assert!(
        stale.is_empty(),
        "позиции CMD_WITHOUT_ROUTE, на которые живой роутер уже отвечает: {}. Маршрут \
         добавлен — удалите строку из таблицы вердиктов.",
        fmt_set(&stale)
    );
}

// ---------------------------------------------------------------------------
// 5. Самопроверка: каталоги на месте, вердикты осмысленны
// ---------------------------------------------------------------------------

#[test]
fn verdict_tables_are_well_formed() {
    for table in [CMD_WITHOUT_ROUTE, ROUTE_WITHOUT_CMD] {
        for v in table {
            assert!(!v.name.is_empty(), "пустое имя в таблице вердиктов");
            assert!(
                v.reason.len() >= 40,
                "вердикт для {:?} короче 40 символов ({}) — освобождение без объяснения \
                 запрещено",
                v.name,
                v.reason.len()
            );
        }
    }
    let cmd_names = names(CMD_WITHOUT_ROUTE);
    assert_eq!(
        cmd_names.len(),
        CMD_WITHOUT_ROUTE.len(),
        "дубли имён в CMD_WITHOUT_ROUTE"
    );
    let route_names = names(ROUTE_WITHOUT_CMD);
    assert_eq!(
        route_names.len(),
        ROUTE_WITHOUT_CMD.len(),
        "дубли имён в ROUTE_WITHOUT_CMD"
    );
    assert!(
        Path::new(&crate_root().join("src/specta_export.rs")).exists(),
        "src/specta_export.rs не найден — источник инвентаря команд переехал, гейт надо \
         перенастроить, а не считать пройденным"
    );
}
