//! Регрессия: `server-bind-localhost-only`.
//!
//! Root cause: `settings_set_network` сохранял `server_host`/`server_port`/
//! `server_cert_path` в `app_settings`, но НИ ОДИН путь bind'а их не читал —
//! и старт сервера, и hot-toggle биндили из `ctx.config.server` (TOML, дефолт
//! `127.0.0.1`). Поэтому выбранный в Настройках `0.0.0.0` никогда не доходил до
//! `TcpListener::bind`, и сервер слушал только localhost.
//!
//! Фикс: `resolve_effective_network` читает live `app_settings` поверх
//! TOML-bootstrap. Этот тест фиксирует поведение: сохранённый `0.0.0.0`
//! (и порт) должны вернуться из резолвера, а не дефолтный TOML `127.0.0.1`.

use trackly_app::http::settings::resolve_effective_network;

/// Хелпер: upsert ключа в app_settings через writer (паттерн из миграции V016).
async fn set_app_setting(ctx: &trackly_app::context::AppCtx, key: &'static str, value: String) {
    let now = ctx.clock.unix_seconds();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
                 VALUES (?1, ?2, ?3, ?3) \
                 ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at_utc = ?3",
                rusqlite::params![key, value, now],
            )
            .map(|_| ())
            .map_err(trackly_infra::error_conversions::map_rusqlite)
        })
        .await
        .expect("upsert app_setting");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn effective_network_prefers_saved_host_over_toml_default() -> anyhow::Result<()> {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let dir = tempfile::TempDir::new()?;
        let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf())?;
        let config = trackly_infra::AppConfig::default();
        // Sanity: TOML-дефолт — именно localhost, который и был «застрявшим» bind'ом.
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8443);

        let log_guard = trackly_app::logging::init(&paths, &config)?;
        let ctx = trackly_app::context::AppCtx::build(paths, config, log_guard).await?;

        // До сохранения: резолвер падает на TOML-bootstrap (localhost).
        let before = resolve_effective_network(&ctx).await?;
        assert_eq!(
            before.host, "127.0.0.1",
            "без сохранённого значения host должен браться из TOML-bootstrap"
        );
        assert_eq!(before.port, 8443);

        // Пользователь в Настройках выбрал 0.0.0.0 и порт 9000 → app_settings.
        set_app_setting(&ctx, "server_host", "0.0.0.0".to_string()).await;
        set_app_setting(&ctx, "server_port", "9000".to_string()).await;

        // После сохранения: резолвер ДОЛЖЕН вернуть 0.0.0.0:9000 (а не 127.0.0.1).
        // Это и есть то значение, что теперь дойдёт до TcpListener::bind.
        let after = resolve_effective_network(&ctx).await?;
        assert_eq!(
            after.host, "0.0.0.0",
            "сохранённый в Настройках host=0.0.0.0 должен доходить до bind, а не подменяться TOML-дефолтом 127.0.0.1"
        );
        assert_eq!(
            after.port, 9000,
            "сохранённый порт должен использоваться при bind"
        );

        Ok::<(), anyhow::Error>(())
    })
    .await
    .expect("test exceeded 30s budget")
}

/// Пустая/пробельная сохранённая строка host не должна затирать TOML-дефолт
/// (защита от случайного очищения настройки → бинд в никуда).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn effective_network_falls_back_on_blank_saved_host() -> anyhow::Result<()> {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let dir = tempfile::TempDir::new()?;
        let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf())?;
        let config = trackly_infra::AppConfig::default();
        let log_guard = trackly_app::logging::init(&paths, &config)?;
        let ctx = trackly_app::context::AppCtx::build(paths, config, log_guard).await?;

        set_app_setting(&ctx, "server_host", "   ".to_string()).await;

        let net = resolve_effective_network(&ctx).await?;
        assert_eq!(
            net.host, "127.0.0.1",
            "пробельный сохранённый host должен откатываться на TOML-bootstrap"
        );

        Ok::<(), anyhow::Error>(())
    })
    .await
    .expect("test exceeded 30s budget")
}
