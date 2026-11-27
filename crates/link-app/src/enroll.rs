//! Первая настройка по коду-приглашению (D-03, D-04, D-07).
//!
//! Порядок защищает от подмены сервера: пин берётся ТОЛЬКО из кода, рукопожатие с чужим
//! сертификатом обрывается до первого прикладного байта (запрос с секретом не уходит), а
//! пользовательский конфиг и `device_secret` пишутся лишь после успешного ответа сервера.
//! Секрет кода нигде не логируется.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use link_proto::dto::{
    EnrollRequest, EnrollResponse, LINK_INVITE_EXPIRED, LINK_INVITE_INVALID, LINK_INVITE_USED,
};
use link_proto::InviteCode;

use crate::config::{
    write_user_config, ConfigSource, FileConfigSource, LinkConfig, LocalState, SourceKind,
};
use crate::http_client::{HttpError, LinkHttp};
use crate::secrets::{device_secret_key, DeviceSecretStore, Secret};
use crate::state::{CmdErrorCode, LinkCmdError};

/// Пути конфигурации, нужные первой настройке.
#[derive(Debug, Clone)]
pub struct ConfigPaths {
    /// Каталог пользовательских данных Link (`link.toml`, `state.toml`).
    pub user_dir: PathBuf,
    /// Машинный конфиг (D-26): если он есть, он побеждает и ввод кода не нужен.
    pub machine_config: PathBuf,
}

/// Сведения об устройстве для регистрации.
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub hostname: String,
    pub os: String,
    pub arch: String,
    pub version: String,
}

impl ClientInfo {
    pub fn detect() -> Self {
        let hostname = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .ok()
            .filter(|h| !h.trim().is_empty())
            .unwrap_or_else(|| "unknown".to_string());
        Self {
            hostname,
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EnrollError {
    #[error("код не введён")]
    EmptyCode,
    #[error("код не распознан")]
    InvalidCode,
    #[error("срок действия кода истёк")]
    ExpiredCode,
    #[error("код использован или отозван")]
    UsedCode,
    #[error("сервер недоступен")]
    ServerUnreachable { server: String },
    #[error("отпечаток сервера не совпал с кодом")]
    PinMismatch { expected: String, got: String },
    #[error("конфигурация задана машинным конфигом")]
    AlreadyConfigured,
    #[error("внутренняя ошибка")]
    Internal,
}

impl EnrollError {
    /// Ошибка команды для UI (UI-SPEC §6.3).
    pub fn to_cmd_error(&self) -> LinkCmdError {
        match self {
            Self::EmptyCode => LinkCmdError::new(CmdErrorCode::EmptyCode),
            Self::InvalidCode => LinkCmdError::new(CmdErrorCode::InvalidCode),
            Self::ExpiredCode => LinkCmdError::new(CmdErrorCode::ExpiredCode),
            Self::UsedCode => LinkCmdError::new(CmdErrorCode::UsedCode),
            Self::ServerUnreachable { server } => {
                LinkCmdError::with_server(CmdErrorCode::ServerUnreachable, server)
            }
            Self::PinMismatch { .. } => LinkCmdError::new(CmdErrorCode::PinMismatch),
            Self::AlreadyConfigured | Self::Internal => LinkCmdError::new(CmdErrorCode::Internal),
        }
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn server_string(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

/// Первая настройка: разобрать код, зарегистрировать устройство, сохранить конфиг и секрет.
///
/// Вход сотрудника здесь не выполняется (D-04): это отдельный шаг (SSO или пароль).
pub async fn setup_with_invite(
    code: &str,
    paths: &ConfigPaths,
    secrets: &DeviceSecretStore,
    local: &LocalState,
    client_info: &ClientInfo,
) -> Result<LinkConfig, EnrollError> {
    if code.trim().is_empty() {
        return Err(EnrollError::EmptyCode);
    }
    // D-26: машинный конфиг побеждает, пользовательский не трогаем и в сеть не идём.
    let machine = FileConfigSource::new(paths.machine_config.clone(), SourceKind::Machine);
    if !matches!(machine.load(), Ok(None)) {
        tracing::info!("первая настройка не нужна: задан машинный конфиг");
        return Err(EnrollError::AlreadyConfigured);
    }

    let invite = InviteCode::decode(code).map_err(|_| EnrollError::InvalidCode)?;
    if invite.is_expired(now_unix()) {
        return Err(EnrollError::ExpiredCode);
    }

    let server = server_string(&invite.host, invite.port);
    // Пин из кода (D-03/D-07): при чужом сертификате запрос не уходит.
    let http = LinkHttp::new(&invite.host, invite.port, invite.spki, None)
        .map_err(|_| EnrollError::Internal)?;
    let req = EnrollRequest {
        secret: URL_SAFE_NO_PAD.encode(invite.secret),
        device_id: local.device_id.clone(),
        hostname: client_info.hostname.clone(),
        os: client_info.os.clone(),
        arch: client_info.arch.clone(),
        client_version: client_info.version.clone(),
    };
    tracing::info!("регистрация устройства на сервере");
    let resp: EnrollResponse = http
        .post_json("/api/v1/link_enroll", &req)
        .await
        .map_err(|e| map_http(e, &server))?;

    // Секрет первым: при сбое записи конфига откатываем его, чтобы не остался конфиг без секрета.
    let key = device_secret_key(&invite.host, invite.port);
    secrets
        .set(&key, &Secret::new(resp.device_secret))
        .map_err(|_| EnrollError::Internal)?;
    if write_user_config(&paths.user_dir, &server, &invite.spki).is_err() {
        let _ = secrets.delete(&key);
        return Err(EnrollError::Internal);
    }
    tracing::info!("устройство зарегистрировано, настройка сохранена");
    Ok(LinkConfig {
        host: invite.host,
        port: invite.port,
        pin: invite.spki,
        spn: None,
        source: SourceKind::User,
    })
}

fn map_http(e: HttpError, server: &str) -> EnrollError {
    match e {
        HttpError::PinMismatch { expected, got } => EnrollError::PinMismatch { expected, got },
        HttpError::Network | HttpError::Server(_) => EnrollError::ServerUnreachable {
            server: server.to_string(),
        },
        HttpError::Api { code, .. } => match code.as_deref() {
            Some(LINK_INVITE_INVALID) => EnrollError::InvalidCode,
            Some(LINK_INVITE_EXPIRED) => EnrollError::ExpiredCode,
            Some(LINK_INVITE_USED) => EnrollError::UsedCode,
            _ => EnrollError::Internal,
        },
        HttpError::Decode => EnrollError::Internal,
    }
}
