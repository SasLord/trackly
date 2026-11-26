//! Visibility gate for the `ws_broadcast` → desktop webview bridge
//! (Phase 41.5 security audit, T-41.5-06).
//!
//! The browser WS path (`http/ws.rs`) filters every event through
//! `WsEvent::is_visible_to(&identity)`. The desktop bridge in `main.rs` used to
//! forward everything, on the premise that the desktop always runs as an
//! Admin/Manager-tier identity and every variant passes for those roles.
//! `WsEvent::ChatReadChanged` broke that premise: it is visible only to the
//! marker owner, even Admin must not receive other users' read/mute markers.
//!
//! The desktop identity is resolved PER EVENT (`resolve_tauri_identity`), not
//! snapshotted once: it depends on the runtime `desktop_lock_enabled` setting
//! and can change while the app runs. If the identity cannot be resolved the
//! event is dropped (fail closed) — the frontend re-reads state on its next
//! fetch, so a dropped refresh hint is harmless.

use crate::context::AppCtx;
use crate::dto::printer::WsEvent;
use crate::tauri_cmds::users::resolve_tauri_identity;

/// Should this broadcast event reach the desktop webview?
pub async fn desktop_event_visible(ctx: &AppCtx, event: &WsEvent) -> bool {
    match resolve_tauri_identity(ctx).await {
        Ok(identity) => event.is_visible_to(&identity),
        Err(e) => {
            tracing::warn!(
                "ws_broadcast->tauri bridge: desktop identity unresolved, event dropped: {e}"
            );
            false
        }
    }
}

/// Имя Tauri-события «пропущены события, перечитай данные» (Phase 43, D-07).
/// Служебное: не `WsEvent`, не проходит `desktop_event_visible`.
pub const RESYNC_EVENT: &str = "trackly-resync";

/// Мост отстал на `skipped` событий (`Lagged`): логируем и просим фронтенд
/// перечитать данные через `emit` (в main.rs это `app_handle.emit(RESYNC_EVENT, ())`).
pub fn resync_on_lagged(skipped: u64, emit: impl FnOnce()) {
    tracing::warn!("ws_broadcast->tauri bridge lagged {skipped} events — resync");
    emit();
}
