//! Общие фикстуры реестрового гейта (Phase 41.7, план 05).
//!
//! Настоящий `AppCtx::build` на временном каталоге (НЕ ручная сборка сервисов
//! с `with_ws_tx`: так гейт проверял бы проводку теста, а не приложения —
//! анти-аналог `number_space_broadcast_gate.rs`). Из того файла берётся только
//! ловля событий.
//!
//! Ловля события без ожидания: подписка `ctx.ws_broadcast.subscribe()` ДО
//! мутации, `try_recv()` сразу после `.await` детерминирован — сервис шлёт до
//! возврата из `async fn`. В канале ходят и чужие варианты `WsEvent`
//! (`NumberSpaceChanged` рядом с мутациями номеров, `RequestStatusChanged`):
//! захват фильтрует по варианту и никогда не берёт «первое событие».
//!
//! Пустое событие: «пусто» = все три списка пусты, тогда оно не шлётся (хелпер
//! `send_entities_changed`). Событие с пустым `place_ids`, но непустым
//! `group_ids`/`device_ids` ШЛЁТСЯ (D-17: клиент трактует его как «перезагрузить
//! всё»).
//!
//! Планы 06–10 в этот файл не пишут — свои хелперы держат в своём тестовом
//! файле. Имена в сидерах вымышленные.

use rusqlite::params;
use tokio::sync::broadcast;

use trackly_app::context::AppCtx;
use trackly_app::dto::printer::WsEvent;
use trackly_core::auth::Identity;
use trackly_infra::error_conversions::map_rusqlite;

/// Тройка списков `EntitiesChanged`: (place_ids, device_ids, group_ids).
pub type Triple = (Vec<i64>, Vec<i64>, Vec<i64>);

pub async fn make_test_ctx() -> (AppCtx, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let paths =
        trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("resolve paths");
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    });
    let log_guard = log_guard.expect("log guard");
    let ctx = AppCtx::build(paths, config, log_guard)
        .await
        .expect("build ctx");
    (ctx, dir)
}

pub fn admin() -> Identity {
    Identity::trusted_admin()
}

// ---------------------------------------------------------------------------
// Захват событий
// ---------------------------------------------------------------------------

/// Выбросить всё накопленное к этому моменту.
pub fn drain(rx: &mut broadcast::Receiver<WsEvent>) {
    while rx.try_recv().is_ok() {}
}

/// Все накопленные события без ожидания (для сценариев, проверяющих оба
/// варианта рядом, например `NumberSpaceChanged` и `EntitiesChanged`).
pub fn collect_events(rx: &mut broadcast::Receiver<WsEvent>) -> Vec<WsEvent> {
    let mut out = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        out.push(ev);
    }
    out
}

/// Первое событие ВАРИАНТА `EntitiesChanged`; любые другие варианты
/// пропускаются (не принимаются за искомое).
pub fn next_entities(rx: &mut broadcast::Receiver<WsEvent>) -> Option<Triple> {
    while let Ok(ev) = rx.try_recv() {
        if let WsEvent::EntitiesChanged {
            place_ids,
            device_ids,
            group_ids,
        } = ev
        {
            return Some((place_ids, device_ids, group_ids));
        }
    }
    None
}

/// Ровно одно `EntitiesChanged`: оно есть, второго нет. Возвращает его тройку.
pub fn assert_one_entities(rx: &mut broadcast::Receiver<WsEvent>, what: &str) -> Triple {
    let first = next_entities(rx).unwrap_or_else(|| {
        panic!("{what}: ожидали ровно одно EntitiesChanged, не пришло ни одного")
    });
    if let Some(second) = next_entities(rx) {
        panic!("{what}: ожидали ровно одно EntitiesChanged, пришло второе: {second:?}");
    }
    first
}

/// Ни одного `EntitiesChanged` (чужие варианты не мешают).
pub fn assert_no_entities(rx: &mut broadcast::Receiver<WsEvent>, what: &str) {
    if let Some(found) = next_entities(rx) {
        panic!("{what}: EntitiesChanged быть не должно, пришло {found:?}");
    }
}

// ---------------------------------------------------------------------------
// Сидеры (прямой SQL через writer; копии из group_write_sites.rs)
// ---------------------------------------------------------------------------

pub async fn seed_place(ctx: &AppCtx, name: &str) -> i64 {
    let name = name.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO places (kind, name, parent_id, is_storage, created_at_utc, \
                 updated_at_utc, version) VALUES ('room', ?1, NULL, 0, 1700000000, 1700000000, 1)",
                params![name],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed place")
}

/// Id встроенного типа групп по коду (`workstation` и т. п.).
pub async fn type_id_by_code(ctx: &AppCtx, code: &str) -> i64 {
    ctx.group_types
        .list_types(&admin(), false)
        .await
        .expect("list_types")
        .into_iter()
        .find(|t| t.code == code)
        .unwrap_or_else(|| panic!("тип {code} не найден"))
        .id
}

/// Группа встроенного типа `workstation` с именем `name` и порядковым `seq`
/// (необязательно привязанная к месту). Возвращает `(type_id, group_id)`.
pub async fn seed_group_type_and_group(
    ctx: &AppCtx,
    name: &str,
    seq: i64,
    place: Option<i64>,
) -> (i64, i64) {
    let type_id = type_id_by_code(ctx, "workstation").await;
    let name = name.to_string();
    let group_id = ctx
        .writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO groups (type_id, name, seq, place_id, parent_group_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, NULL, 1700000000, 1700000000, 1)",
                params![type_id, name, seq, place],
            )
            .map_err(map_rusqlite)?;
            Ok(conn.last_insert_rowid())
        })
        .await
        .expect("seed group");
    (type_id, group_id)
}

/// Живое устройство (`device_type` 1 — обычное, 2 — принтер) и, при `group_id`,
/// сразу в составе группы.
pub async fn seed_device(
    ctx: &AppCtx,
    device_type: i64,
    name: &str,
    inv: &str,
    place: Option<i64>,
    group_id: Option<i64>,
) -> i64 {
    let (name, inv) = (name.to_string(), inv.to_string());
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO devices (type_id, name, inventory_number, place_id, status_id, \
                 created_at_utc, updated_at_utc, version) \
                 VALUES (?1, ?2, ?3, ?4, 1, 1700000000, 1700000000, 1)",
                params![device_type, name, inv, place],
            )
            .map_err(map_rusqlite)?;
            let id = conn.last_insert_rowid();
            if let Some(g) = group_id {
                conn.execute(
                    "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
                     VALUES (?1, ?2, 1700000000)",
                    params![id, g],
                )
                .map_err(map_rusqlite)?;
            }
            Ok(id)
        })
        .await
        .expect("seed device")
}

/// Ввести устройство в состав группы прямой вставкой (место не меняется).
pub async fn join_group(ctx: &AppCtx, device_id: i64, group_id: i64) {
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO group_devices (device_id, group_id, added_at_utc) \
                 VALUES (?1, ?2, 1700000100)",
                params![device_id, group_id],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .expect("join group");
}
