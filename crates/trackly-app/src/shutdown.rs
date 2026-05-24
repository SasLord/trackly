//! Graceful shutdown — Ctrl-C handler, который cancel'ает `CancellationToken`.
//!
//! Phase 1 main.rs вызывает `install_signal_handler` только в Tauri-launching
//! branch (self-test exits сразу). Phase 5 axum serve и Phase 7 supervisor
//! слушают тот же token.

use tokio_util::sync::CancellationToken;

/// Spawn'ит async-task, ожидающий Ctrl-C. На сигнале — `token.cancel()`.
///
/// Возвращается мгновенно; реальное ожидание — внутри spawn'ленной task.
/// Idempotent: повторные сигналы безопасны (token уже cancelled).
pub fn install_signal_handler(token: CancellationToken) {
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                tracing::info!("Ctrl-C received — initiating graceful shutdown");
                token.cancel();
            }
            Err(e) => {
                tracing::error!("failed to install Ctrl-C handler: {e}");
            }
        }
    });
}
