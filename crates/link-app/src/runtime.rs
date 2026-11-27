//! Сборка рантайма Link: конфиг, секреты, супервизор связи, приёмник событий Tauri, трей.
//!
//! Один владелец состояния: `ConnectionSupervisor` (`connection.rs`). Команды webview и
//! `RealTransport` делят одни и те же `Arc<LinkHttp>`, `Arc<TokenStore>` и `Arc<Mutex<LocalState>>`:
//! cookie-сессия, полученная командой `link_login`, нужна WS-подключению автомата.
//! Сетевых вызовов отсюда в webview нет (ADR 0002).

use std::sync::{Arc, Mutex as StdMutex};

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

use crate::authflow::Platform;
use crate::config::{
    resolve, standard_sources, user_config_dir, ConfigOutcome, LinkConfig, LocalState,
};
use crate::connection::{
    Command, ConnectionSupervisor, EventMeta, EventSink, RealTransport, SupervisorConfig,
    SupervisorHandle,
};
use crate::enroll::{setup_with_invite, ClientInfo, ConfigPaths};
use crate::http_client::{DeviceCreds, LinkHttp};
use crate::secrets::{
    device_secret_key, file_store_dir, primary_from_env, DeviceSecretStore, FileSecretStore,
    TokenStore,
};
use crate::sso::{platform_provider, SsoProvider};
use crate::state::{CmdErrorCode, LinkCmdError, LinkEvent, LinkState, LINK_EVENT};
use crate::tray::{update_tray, TrayHandlers};

/// Событие Tauri с сырым событием беседы (мост WS -> фронт).
pub const BRIDGE_EVENT: &str = "trackly-event";
/// Событие Tauri «перечитай всё» после восстановления связи.
pub const RESYNC_EVENT: &str = "trackly-resync";
/// Метка главного окна.
pub const MAIN_WINDOW: &str = "main";

/// Выход из приложения через абстракцию: тесты подставляют фейк, выхода
/// в обход Tauri в коде нет (T-44-66).
pub trait AppControl: Send + Sync + 'static {
    fn exit(&self, code: i32);
}

/// Реальный выход через Tauri: `RunEvent::ExitRequested` получает `code = Some(..)`.
pub struct TauriControl(pub AppHandle);

impl AppControl for TauriControl {
    fn exit(&self, code: i32) {
        self.0.exit(code);
    }
}

/// Показать, развернуть и сфокусировать главное окно.
pub fn show_and_focus(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN_WINDOW) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Состояние, в котором без действия сотрудника дальше не пойти (D-18).
pub fn needs_employee_action(state: &LinkState) -> bool {
    matches!(
        state,
        LinkState::NeedsSetup
            | LinkState::NeedsLogin { .. }
            | LinkState::PinMismatch { .. }
            | LinkState::Revoked { .. }
    )
}

/// Решение о показе окна (D-18): один раз на ВХОД в состояние, требующее действия.
/// Повторное событие того же вида (например, смена причины) окно не дёргает.
pub fn should_auto_show(prev: &LinkState, next: &LinkState) -> bool {
    needs_employee_action(next) && std::mem::discriminant(prev) != std::mem::discriminant(next)
}

/// Приёмник событий для Tauri: `link-event`, `trackly-event`, `trackly-resync`, трей, окно.
pub struct TauriSink {
    app: AppHandle,
    last: StdMutex<LinkEvent>,
    last_user: StdMutex<Option<String>>,
}

impl TauriSink {
    fn new(app: AppHandle, meta: &EventMeta) -> Self {
        Self {
            app,
            last: StdMutex::new(LinkEvent {
                state: LinkState::Starting,
                tray_available: meta.tray_available,
                secrets_persistent: meta.secrets_persistent,
                platform: meta.platform.clone(),
            }),
            last_user: StdMutex::new(None),
        }
    }

    /// Последнее отправленное событие (ответ `link_get_state`).
    pub fn current(&self) -> LinkEvent {
        self.last.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// Состояние, о котором автомат не знает (NeedsSetup до конфига, Starting после enroll).
    fn publish(&self, state: LinkState) {
        let mut ev = self.current();
        ev.state = state;
        self.state_changed(&ev);
    }
}

impl EventSink for TauriSink {
    fn state_changed(&self, ev: &LinkEvent) {
        let prev = {
            let mut last = self.last.lock().unwrap_or_else(|p| p.into_inner());
            std::mem::replace(&mut *last, ev.clone())
        };
        if let LinkState::Connected { user, .. } = &ev.state {
            *self.last_user.lock().unwrap_or_else(|p| p.into_inner()) =
                Some(user.display_name.clone());
        }
        if let Err(e) = self.app.emit(LINK_EVENT, ev) {
            tracing::warn!("не удалось отправить link-event: {e}");
        }
        let name = self
            .last_user
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        if update_tray(&self.app, &ev.state, name.as_deref()).is_err() {
            tracing::warn!("не удалось обновить значок трея");
        }
        if should_auto_show(&prev.state, &ev.state) {
            show_and_focus(&self.app);
        }
    }

    fn bridge_event(&self, payload: serde_json::Value) {
        let _ = self.app.emit(BRIDGE_EVENT, payload);
    }

    fn resync(&self) {
        let _ = self.app.emit(RESYNC_EVENT, ());
    }
}

/// Активная сессия: общие с командами `http`/`cfg` и управление автоматом.
#[derive(Clone)]
pub struct Session {
    pub cfg: LinkConfig,
    pub http: Arc<LinkHttp>,
    pub handle: Arc<SupervisorHandle>,
}

/// Состояние приложения, регистрируется через `app.manage`.
pub struct Runtime {
    pub sink: Arc<TauriSink>,
    pub control: Arc<dyn AppControl>,
    pub meta: EventMeta,
    pub paths: ConfigPaths,
    pub tokens: Arc<TokenStore>,
    pub device_secrets: Arc<DeviceSecretStore>,
    pub local: Arc<Mutex<LocalState>>,
    pub sso: Arc<dyn SsoProvider>,
    session: Mutex<Option<Session>>,
}

fn device_creds(
    local: &LocalState,
    secrets: &DeviceSecretStore,
    cfg: &LinkConfig,
) -> Option<DeviceCreds> {
    secrets
        .get(&device_secret_key(&cfg.host, cfg.port))
        .ok()
        .flatten()
        .map(|s| DeviceCreds {
            device_id: local.device_id.clone(),
            secret: s.expose().to_string(),
        })
}

impl Runtime {
    /// Собрать слои и запустить автомат. Конфига нет -> `NeedsSetup` без сетевых вызовов.
    pub fn start(app: &AppHandle, tray_available: bool) -> Result<Self, String> {
        let user_dir = user_config_dir(app).map_err(|e| e.to_string())?;
        let local = LocalState::load_or_init(&user_dir).map_err(|e| e.to_string())?;
        let tokens = Arc::new(TokenStore::new(primary_from_env()));
        let device_secrets = Arc::new(DeviceSecretStore::new(
            primary_from_env(),
            FileSecretStore::new(file_store_dir(&user_dir)),
        ));
        let meta = EventMeta {
            tray_available,
            secrets_persistent: tokens.persistent(),
            platform: std::env::consts::OS.to_string(),
        };
        let sink = Arc::new(TauriSink::new(app.clone(), &meta));
        let (machine, user) = standard_sources(&user_dir);
        let outcome = resolve(&[&machine, &user]);
        let paths = ConfigPaths {
            user_dir,
            machine_config: crate::config::machine_config_path(),
        };
        let rt = Self {
            sink,
            control: Arc::new(TauriControl(app.clone())),
            meta,
            paths,
            tokens,
            device_secrets,
            local: Arc::new(Mutex::new(local)),
            sso: Arc::from(platform_provider()),
            session: Mutex::new(None),
        };
        match outcome {
            ConfigOutcome::Ready(cfg) => {
                // block_on даёт контекст tokio рантайма Tauri для `tokio::spawn` автомата.
                tauri::async_runtime::block_on(rt.start_session(cfg)).map_err(|e| e.message)?;
            }
            ConfigOutcome::NeedsSetup => rt.sink.publish(LinkState::NeedsSetup),
            ConfigOutcome::Invalid { reason, .. } => {
                // Битый конфиг не пишется в лог целиком: причина без значений секретов.
                tracing::warn!("конфигурация Link не принята: {reason}");
                rt.sink.publish(LinkState::NeedsSetup);
            }
        }
        Ok(rt)
    }

    /// Запустить автомат для `cfg` (заменяя прежний, если он был).
    pub async fn start_session(&self, cfg: LinkConfig) -> Result<(), LinkCmdError> {
        let creds = {
            let local = self.local.lock().await;
            device_creds(&local, &self.device_secrets, &cfg)
        };
        let http = Arc::new(
            LinkHttp::new(&cfg.host, cfg.port, cfg.pin, creds)
                .map_err(|_| LinkCmdError::new(CmdErrorCode::Internal))?,
        );
        let transport = Arc::new(RealTransport {
            http: http.clone(),
            sso: self.sso.clone(),
            tokens: self.tokens.clone(),
            local: self.local.clone(),
            cfg: cfg.clone(),
            platform: Platform::current(),
        });
        let sup_cfg = SupervisorConfig::new(cfg.server_display(), self.meta.clone());
        let handle = Arc::new(ConnectionSupervisor::spawn(
            transport,
            self.sink.clone(),
            sup_cfg,
        ));
        let old = self
            .session
            .lock()
            .await
            .replace(Session { cfg, http, handle });
        if let Some(old) = old {
            old.handle.command(Command::Stop).await;
        }
        Ok(())
    }

    /// Активная сессия (клон дешёвый: все поля за `Arc`).
    pub async fn session(&self) -> Option<Session> {
        self.session.lock().await.clone()
    }

    /// Первая настройка: код -> регистрация -> автомат. Сеть только после разбора кода.
    pub async fn enroll(&self, code: &str) -> Result<(), LinkCmdError> {
        let cfg = {
            let local = self.local.lock().await;
            setup_with_invite(
                code,
                &self.paths,
                &self.device_secrets,
                &local,
                &ClientInfo::detect(),
            )
            .await
            .map_err(|e| e.to_cmd_error())?
        };
        // UI уходит с экрана настройки сразу: тихий старт без сети не должен его держать.
        self.sink.publish(LinkState::Starting);
        self.start_session(cfg).await
    }

    /// Команда автомату; без сессии (нет конфига) — ошибка.
    pub async fn command(&self, cmd: Command) -> Result<(), LinkCmdError> {
        match self.session().await {
            Some(s) => {
                s.handle.command(cmd).await;
                Ok(())
            }
            None => Err(LinkCmdError::new(CmdErrorCode::Internal)),
        }
    }

    /// Выход из учётной записи (меню трея и `link_logout`).
    pub async fn request_logout(&self) {
        let _ = self.command(Command::Logout).await;
    }

    /// Единственный выход из Link (D-16).
    pub fn quit(&self) {
        quit_via(&*self.control);
    }
}

/// Выход через абстракцию; вынесено для теста.
pub fn quit_via(control: &dyn AppControl) {
    control.exit(0);
}

/// Обработчики меню трея (D-16): «Открыть», «Выйти из учётной записи», «Закрыть».
pub fn tray_handlers() -> TrayHandlers {
    TrayHandlers {
        on_open: Arc::new(show_and_focus),
        on_logout: Arc::new(|app: &AppHandle| {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Some(rt) = app.try_state::<Runtime>() {
                    rt.request_logout().await;
                }
            });
        }),
        on_quit: Arc::new(|app: &AppHandle| {
            if let Some(rt) = app.try_state::<Runtime>() {
                rt.quit();
            } else {
                app.exit(0);
            }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{LinkUserView, LoginMethod};

    fn connected() -> LinkState {
        LinkState::Connected {
            user: LinkUserView {
                display_name: "Иванов И.И.".into(),
                login: "ivanov".into(),
            },
            method: LoginMethod::Password,
            server: "s".into(),
        }
    }

    fn login(after_logout: bool) -> LinkState {
        LinkState::NeedsLogin {
            reason: None,
            can_retry_sso: false,
            after_logout,
            server: "s".into(),
        }
    }

    #[test]
    fn window_shown_only_when_employee_action_needed() {
        let s = "s".to_string();
        for st in [
            LinkState::NeedsSetup,
            login(false),
            LinkState::PinMismatch {
                server: s.clone(),
                expected: "a".into(),
                got: "b".into(),
            },
            LinkState::Revoked { server: s.clone() },
        ] {
            assert!(should_auto_show(&LinkState::Starting, &st), "{st:?}");
        }
        for st in [
            LinkState::Starting,
            connected(),
            LinkState::Offline { server: s },
        ] {
            assert!(!should_auto_show(&LinkState::Starting, &st), "{st:?}");
        }
    }

    #[test]
    fn window_not_shown_twice_for_same_kind() {
        assert!(!should_auto_show(&login(false), &login(true)));
        assert!(should_auto_show(&connected(), &login(true)));
        assert!(should_auto_show(
            &login(false),
            &LinkState::Revoked { server: "s".into() }
        ));
    }

    #[test]
    fn connected_to_offline_does_not_pop_window() {
        assert!(!should_auto_show(
            &connected(),
            &LinkState::Offline { server: "s".into() }
        ));
    }
}
