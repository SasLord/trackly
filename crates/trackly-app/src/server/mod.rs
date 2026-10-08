//! Серверный режим — lifecycle управление axum HTTP/HTTPS сервером.
//!
//! ## Архитектура
//!
//! Горячий старт/стоп (D-Server-01) реализован через дочерний `CancellationToken`
//! (никогда не отменяет мастер `AppCtx.shutdown`):
//!
//! ```text
//! AppCtx.shutdown (master)
//!   └── server token (child, per-run)
//!         └── start_server() listen loop
//! ```
//!
//! `ServerHandle` хранит cancel-token и JoinHandle для управления lifecycle.
//!
//! ## TLS
//!
//! Использует `tokio-rustls::TlsAcceptor` поверх `TcpListener::bind`.
//! axum 0.8 не поддерживает TLS напрямую через `axum::serve` — используем
//! ручной accept-loop + `hyper::server::conn::http1`.
//!
//! ## Submodules
//!
//! - [`tls`] — генерация self-signed, загрузка из PEM, fingerprint
//! - [`rusqlite_session_store`] — tower-sessions SessionStore impl

pub mod rusqlite_session_store;
pub mod tls;

use axum::Router;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

/// Сколько даём УЖЕ НАЧАТОМУ запросу доехать до ответа после остановки сервера
/// (WR-02 ревью фазы 41.7).
///
/// Это не «graceful drain» в смысле «дождаться всех» — цикл `accept` обрывается
/// сразу, новые запросы на живых соединениях отклоняются сразу
/// (`graceful_shutdown` = `disable_keep_alive`), и ждём мы ровно один начатый
/// запрос на соединение.
///
/// Верхняя граница нужна, потому что ждать бесконечно нельзя: зависший
/// обработчик держал бы задачу соединения вечно. 3 секунды выбраны как
/// компромисс с `WriterHandle::send_timeout` (5 с): типичная LAN-мутация
/// укладывается в единицы миллисекунд, так что грейс покрывает её с запасом, но
/// НЕ покрывает полностью патологический случай «очередь записи занята почти
/// весь свой таймаут». Остаток окна — принятый риск, ограниченный сверху
/// грейсом вместо прежнего «всегда» (до WR-02 соединение бросалось мгновенно,
/// т.е. окно было 100%).
const DRAIN_GRACE: std::time::Duration = std::time::Duration::from_secs(3);

/// Токен остановки ЭТОГО инстанса сервера, прокинутый в расширения запроса.
///
/// Нужен обработчикам, которые переживают сам запрос. Главный потребитель —
/// `http::ws::ws_handler`: апгрейженный WebSocket живёт в ОТДЕЛЬНОЙ задаче,
/// которую axum спаунит внутри `on_upgrade`, и дроп future hyper-соединения её
/// сокет НЕ закрывает (`hyper::upgrade::on` уже отдал туда владение IO).
/// Поэтому единственный способ закрыть живой WS при остановке серверного
/// режима — отдать токен внутрь цикла сокета (отладочная сессия
/// `ws-disconnect-toast-no-show`, GAP-1 фазы 41.7).
///
/// Newtype, а не голый `CancellationToken`, чтобы расширение нельзя было спутать
/// с чужим токеном в том же наборе extensions.
#[derive(Clone)]
pub struct ServerShutdown(pub CancellationToken);

/// Handle для управления жизненным циклом запущенного сервера.
///
/// Хранит cancel-token (дочерний к AppCtx.shutdown) и JoinHandle.
pub struct ServerHandle {
    /// Дочерний CancellationToken для этого инстанса сервера.
    /// `cancel()` останавливает сервер, не трогая мастер-shutdown.
    pub cancel: CancellationToken,
    /// JoinHandle фоновой задачи сервера.
    pub task: tokio::task::JoinHandle<()>,
}

/// Запустить HTTPS сервер.
///
/// Принимает уже забинженный `TcpListener` (caller отвечает за bind и получение
/// локального адреса до вызова этой функции), принимает TLS соединения через
/// `tls_acceptor`, раздаёт их hyper HTTP/1.1 сервис из `app` (axum Router).
///
/// Завершается при `shutdown.cancelled()` — цикл accept прерывается, функция
/// возвращает `Ok(())`. JoinHandle в `ServerHandle` завершится следом.
///
/// Новое соединение спаунится как независимая tokio-задача, но ПОЛУЧАЕТ клон
/// `shutdown`. По отмене токена соединение ЗАКРЫВАЕТСЯ — это обязательная часть
/// контракта: WS-клиент в LAN-браузере узнаёт об остановке сервера ТОЛЬКО из
/// события `onclose`. Пока соединения оставались жить, браузер не видел ничего
/// (см. отладочную сессию `ws-disconnect-toast-no-show`, GAP-1 фазы 41.7).
///
/// Закрытие при этом НЕ мгновенное для уже начатого запроса: ему даётся
/// [`DRAIN_GRACE`] на то, чтобы доехать до ответа (WR-02 — очередь записи не
/// отменяема, и мгновенный дроп коммитил запись, оставляя клиента с
/// транспортной ошибкой). Новые запросы на живых соединениях не принимаются с
/// самого момента отмены. Подробности — в [`serve_connection_inner`].
///
/// # Convenience
///
/// Для стандартного сценария используй [`start_server_on_addr`] — он сам
/// биндит `SocketAddr` и вызывает эту функцию.
pub async fn start_server(
    app: Router,
    listener: TcpListener,
    tls_acceptor: TlsAcceptor,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let addr = listener.local_addr()?;
    tracing::info!("HTTPS server listening on {addr}");

    loop {
        tokio::select! {
            biased;

            _ = shutdown.cancelled() => {
                tracing::info!("server shutdown signal received, stopping accept loop");
                break;
            }

            result = listener.accept() => {
                match result {
                    Ok((stream, peer_addr)) => {
                        let tls = tls_acceptor.clone();
                        let app_clone = app.clone();
                        // Токен ЭТОГО инстанса сервера уезжает внутрь задачи
                        // соединения — без него «Остановить сервер» гасило лишь
                        // цикл accept, а уже установленные соединения жили
                        // вечно (детали — в `serve_connection_inner`).
                        let conn_shutdown = shutdown.clone();
                        tokio::spawn(async move {
                            // Реакция на отмену токена живёт ВНУТРИ
                            // `serve_connection_inner`: она разная на разных
                            // стадиях соединения (рукопожатие бросаем, начатый
                            // запрос доводим до ответа). Снаружи гонки больше
                            // нет — иначе дроп future убивал бы и запрос,
                            // который уже успел что-то записать в БД (WR-02).
                            serve_connection_inner(
                                tls,
                                stream,
                                app_clone,
                                peer_addr,
                                conn_shutdown,
                            )
                            .await;
                        });
                    }
                    Err(e) => {
                        tracing::error!("TCP accept error: {e}");
                    }
                }
            }
        }
    }

    tracing::info!("HTTPS server stopped");
    Ok(())
}

/// Обслужить ОДНО принятое TCP-соединение: TLS-рукопожатие → hyper HTTP/1.1
/// сервис из axum `Router`.
///
/// Вынесено из `start_server` отдельной функцией, чтобы вся реакция на
/// `shutdown.cancelled()` жила в одном месте. Реакция РАЗНАЯ по стадиям:
///
/// * **TLS-рукопожатие** — бросаем немедленно. Запроса ещё нет, терять нечего,
///   а зависший handshake переживать остановку сервера не должен.
/// * **Обслуживание HTTP** — `graceful_shutdown()` (он же
///   `disable_keep_alive`): УЖЕ НАЧАТЫЙ запрос доезжает до ответа, НОВЫЕ на
///   этом соединении не принимаются, после ответа соединение закрывается.
///   Ограничено сверху [`DRAIN_GRACE`], чтобы зависший обработчик не держал
///   задачу вечно.
///
/// Почему не дроп future соединения (как было до WR-02): очередь записи
/// НЕ ОТМЕНЯЕМА. `WriterHandle::execute` отправляет замыкание в mpsc и ждёт
/// oneshot; дроп ожидающей стороны замыкание не отменяет — оно исполняется на
/// writer-потоке и коммитит, а `let _ = reply_tx.send(...)` молча съедает
/// недоставленный ответ. Поэтому «бросить соединение» означало: акт, его
/// `act_items`, смены мест и строки аудита ЗАКОММИЧЕНЫ, но браузер получил
/// транспортную ошибку, а рассылки `EntitiesChanged`/`NumberSpaceChanged`,
/// живущие ПОСЛЕ `.await?`, не выполнились. Пользователь повторяет отправку —
/// второй акт на ту же передачу.
///
/// Апгрейженных WebSocket'ов это не касается ни до, ни после: при апгрейде
/// future соединения становится `Ready` (hyper отдаёт владение IO задаче из
/// `on_upgrade`), поэтому задача соединения выходит ещё до остановки сервера, а
/// `graceful_shutdown` на апгрейженном соединении — no-op (`inner` уже `None`).
/// WS закрывается ветвью `ServerShutdown` внутри `http::ws::handle_ws_socket`,
/// и именно поэтому drain-grace НЕ возвращает GAP-1 (гарантия закреплена
/// тестом `ws_close_on_server_stop`).
async fn serve_connection_inner(
    tls: TlsAcceptor,
    stream: tokio::net::TcpStream,
    app: Router,
    peer_addr: SocketAddr,
    shutdown: CancellationToken,
) {
    let handshake = tokio::select! {
        _ = shutdown.cancelled() => {
            tracing::debug!("server stopped during TLS handshake from {peer_addr} — dropping");
            return;
        }
        res = tls.accept(stream) => res,
    };
    match handshake {
        Ok(tls_stream) => {
            let io = TokioIo::new(tls_stream);
            // Клон для расширений запроса: сам `shutdown` нужен ниже, в гонке с
            // future соединения.
            let shutdown_for_req = shutdown.clone();
            // Use ServiceExt::oneshot to consume Router per-request
            let hyper_service = hyper::service::service_fn(move |mut req: hyper::Request<_>| {
                // Inject ConnectInfo so tower_governor's PeerIpKeyExtractor
                // can derive the client IP for per-IP rate limiting on
                // /auth_login. The manual hyper accept-loop (no axum::serve)
                // otherwise leaves ConnectInfo absent → the extractor fails
                // with "Unable to extract key!" → 500 on every login.
                req.extensions_mut()
                    .insert(axum::extract::ConnectInfo(peer_addr));
                // Токен остановки этого инстанса сервера — для обработчиков,
                // переживающих запрос (WS-сокет, см. `ServerShutdown`).
                req.extensions_mut()
                    .insert(ServerShutdown(shutdown_for_req.clone()));
                // Clone Router for each request — Router is cheap Clone
                app.clone().oneshot(req)
            });
            // `.with_upgrades()` is REQUIRED for WebSocket support.
            // axum's `WebSocketUpgrade` emits a 101 response and then
            // awaits `hyper::upgrade::on(req)` inside `on_upgrade` to
            // obtain the upgraded stream. That upgrade future only ever
            // resolves when the hyper connection is driven with
            // `.with_upgrades()`. Without it, hyper writes the 101, the
            // connection future completes, and the socket is closed ~1s
            // later — the client sees "101 Switching Protocols" then
            // "network connection was lost", on a reconnect loop, and the
            // server-side `handle_ws_socket` never runs. (See debug
            // session ui-ws-toast-reports-flicker, Bug A.)
            let conn = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, hyper_service)
                .with_upgrades();
            tokio::pin!(conn);

            let outcome = tokio::select! {
                res = conn.as_mut() => Some(res),
                _ = shutdown.cancelled() => {
                    // Новые запросы на этом соединении больше не принимаются,
                    // но начатый доводим до ответа — иначе уже закоммиченная
                    // запись осталась бы без ответа клиенту (WR-02).
                    conn.as_mut().graceful_shutdown();
                    match tokio::time::timeout(DRAIN_GRACE, conn.as_mut()).await {
                        Ok(res) => Some(res),
                        Err(_elapsed) => {
                            tracing::debug!(
                                "server stopped — drain grace expired, dropping connection from {peer_addr}"
                            );
                            None
                        }
                    }
                }
            };
            if let Some(Err(e)) = outcome {
                tracing::debug!("HTTP connection error from {peer_addr}: {e}");
            }
        }
        Err(e) => {
            tracing::warn!("TLS accept error from {peer_addr}: {e}");
        }
    }
}

/// Вспомогательная функция: биндит `addr`, затем запускает [`start_server`].
///
/// Удобно для продакшена когда адрес известен заранее и получать `local_addr`
/// не нужно. В тестах предпочтительнее использовать `start_server` напрямую
/// с предварительно забинженным `TcpListener::bind("127.0.0.1:0")`.
pub async fn start_server_on_addr(
    app: Router,
    addr: SocketAddr,
    tls_acceptor: TlsAcceptor,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    start_server(app, listener, tls_acceptor, shutdown).await
}
