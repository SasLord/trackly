//! `LinkService` — коды-приглашения, устройства, входы и отзыв Trackly Link
//! (Phase 44, LNK-07).
//!
//! Все записи идут через единственного писателя; побочные эффекты (разрыв WS
//! через [`LinkHub`]) выполняются только после успешного коммита. Секреты в БД
//! хранятся лишь как SHA-256 (D-10); сам секрет возвращается ровно один раз.

use std::sync::Arc;

use argon2::password_hash::rand_core::{OsRng, RngCore};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;

use crate::dto::link::{
    BannedDeviceDto, InviteRowDto, LinkLoginRowDto, UserLinkStatusDto, INVITE_TTL_HOURS,
};
use crate::dto::printer::WsEvent;
use crate::server::link_hub::LinkHub;

/// Не чаще раза в это число секунд обновляем `last_seen_utc` при проверке устройства.
const LAST_SEEN_THROTTLE_SECS: i64 = 60;
const MAX_FIELD_LEN: usize = 200;

fn db<T>(r: rusqlite::Result<T>) -> Result<T, AppError> {
    r.map_err(map_rusqlite)
}

fn internal(e: tokio::task::JoinError) -> AppError {
    AppError::Internal {
        source_chain: format!("spawn_blocking: {e}"),
    }
}

fn random_32() -> [u8; 32] {
    let mut b = [0u8; 32];
    OsRng.fill_bytes(&mut b);
    b
}

fn sha(s: &str) -> Vec<u8> {
    Sha256::digest(s.as_bytes()).to_vec()
}

/// Сравнение без раннего выхода (T-44-19).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn clip(s: &str) -> String {
    s.chars().take(MAX_FIELD_LEN).collect()
}

// ---------------------------------------------------------------------------
// Ошибки
// ---------------------------------------------------------------------------

/// Исход погашения кода.
#[derive(Debug)]
pub enum EnrollError {
    InviteInvalid,
    InviteExpired,
    InviteUsed,
    InviteRevoked,
    /// device_id уже зарегистрирован; существующее устройство не перезаписывается.
    DeviceExists,
    Internal(AppError),
}

impl From<AppError> for EnrollError {
    fn from(e: AppError) -> Self {
        Self::Internal(e)
    }
}

impl EnrollError {
    /// Преобразование в доменную ошибку без деталей наружу.
    pub fn into_app(self) -> AppError {
        let e = self;
        let msg = match e {
            EnrollError::Internal(a) => return a,
            EnrollError::InviteInvalid => "код не распознан",
            EnrollError::InviteExpired => "срок действия кода истёк",
            EnrollError::InviteUsed => "код уже использован",
            EnrollError::InviteRevoked => "код отозван",
            EnrollError::DeviceExists => "устройство уже зарегистрировано",
        };
        AppError::Validation {
            field: "invite".into(),
            message: msg.into(),
        }
    }
}

#[derive(Debug)]
pub enum DeviceError {
    Unknown,
    BadSecret,
    Banned,
    Internal(AppError),
}

impl From<AppError> for DeviceError {
    fn from(e: AppError) -> Self {
        Self::Internal(e)
    }
}

impl DeviceError {
    /// Преобразование в доменную ошибку без деталей наружу.
    pub fn into_app(self) -> AppError {
        let e = self;
        match e {
            DeviceError::Internal(a) => a,
            DeviceError::Unknown | DeviceError::BadSecret => AppError::Unauthorized,
            DeviceError::Banned => AppError::Forbidden,
        }
    }
}

#[derive(Debug)]
pub enum ResumeError {
    Invalid,
    Revoked,
    DeviceBanned,
    UserInactive,
    Internal(AppError),
}

impl From<AppError> for ResumeError {
    fn from(e: AppError) -> Self {
        Self::Internal(e)
    }
}

impl ResumeError {
    /// Преобразование в доменную ошибку без деталей наружу.
    pub fn into_app(self) -> AppError {
        let e = self;
        match e {
            ResumeError::Internal(a) => a,
            ResumeError::Invalid | ResumeError::Revoked => AppError::Unauthorized,
            ResumeError::DeviceBanned | ResumeError::UserInactive => AppError::Forbidden,
        }
    }
}

// ---------------------------------------------------------------------------
// Типы
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct InviteIssued {
    pub id: i64,
    /// Строка `TL1.…`; показывается один раз.
    pub code: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteRow {
    pub id: i64,
    /// `active` | `used` | `expired` | `revoked`.
    pub status: &'static str,
    pub created_by_user_id: i64,
    pub created_at_utc: i64,
    pub expires_at_utc: i64,
    pub used_at_utc: Option<i64>,
    pub used_by_device_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DeviceMeta {
    pub device_id: String,
    pub hostname: String,
    pub os: String,
    pub arch: String,
    pub client_version: String,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginMethod {
    Sso,
    Password,
}

impl LoginMethod {
    fn as_str(self) -> &'static str {
        match self {
            Self::Sso => "sso",
            Self::Password => "password",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoginIssued {
    pub login_id: i64,
    /// Только для password; показывается один раз.
    pub device_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginRow {
    pub login_id: i64,
    pub user_id: i64,
    pub device_id: String,
    pub method: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveLoginRow {
    pub login_id: i64,
    pub user_id: i64,
    pub user_login: Option<String>,
    pub full_name: Option<String>,
    pub device_id: String,
    pub hostname: Option<String>,
    pub method: String,
    pub created_at_utc: i64,
    pub last_used_utc: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BannedDeviceRow {
    pub device_id: String,
    pub hostname: Option<String>,
    pub revoked_at_utc: i64,
    pub revoked_by_user_id: Option<i64>,
}

// ---------------------------------------------------------------------------
// Сервис
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct LinkService {
    writer: Arc<WriterHandle>,
    readers: Arc<ReaderPool>,
    clock: Arc<dyn Clock + Send + Sync>,
    hub: Arc<LinkHub>,
    ws_tx: Option<Arc<tokio::sync::broadcast::Sender<WsEvent>>>,
}

/// Записи старше этого срока (использованные/истёкшие/отозванные коды) в списке
/// не показываются (UI-SPEC §8.3).
const INVITE_LIST_RETENTION_SECS: i64 = 30 * 24 * 3600;

/// Срок кода из допустимого набора часов в секундах; иначе `Validation`.
pub fn ttl_hours_to_secs(hours: i32) -> Result<i64, AppError> {
    if INVITE_TTL_HOURS.contains(&hours) {
        Ok(i64::from(hours) * 3600)
    } else {
        Err(AppError::Validation {
            field: "ttlHours".into(),
            message: "срок действия кода: 1 час, 24 часа или 7 суток".into(),
        })
    }
}

fn invite_status(now: i64, expires: i64, used: Option<i64>, revoked: Option<i64>) -> &'static str {
    if revoked.is_some() {
        "revoked"
    } else if used.is_some() {
        "used"
    } else if expires <= now {
        "expired"
    } else {
        "active"
    }
}

impl LinkService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
        hub: Arc<LinkHub>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            hub,
            ws_tx: None,
        }
    }

    /// Подключить общий канал WS: изменения входов/кодов/запретов уходят админам.
    pub fn with_ws_tx(mut self, ws_tx: Arc<tokio::sync::broadcast::Sender<WsEvent>>) -> Self {
        self.ws_tx = Some(ws_tx);
        self
    }

    /// `WsEvent::LinkChanged` (без полезной нагрузки, виден только Admin).
    /// Вызывать ТОЛЬКО после успешного `.await?` записи и вне замыкания писателя.
    fn broadcast_link_changed(&self, op: &'static str) {
        let Some(tx) = &self.ws_tx else {
            return;
        };
        tracing::debug!(op, "link_changed");
        let _ = tx.send(WsEvent::LinkChanged {});
    }

    async fn read<T, F>(&self, f: F) -> Result<T, AppError>
    where
        T: Send + 'static,
        F: FnOnce(&rusqlite::Connection) -> Result<T, AppError> + Send + 'static,
    {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            f(&conn)
        })
        .await
        .map_err(internal)?
    }

    // ----- коды-приглашения ------------------------------------------------

    pub async fn create_invite(
        &self,
        host: &str,
        port: u16,
        spki: [u8; 32],
        ttl_secs: i64,
        admin_id: i64,
    ) -> Result<InviteIssued, AppError> {
        if ttl_secs <= 0 {
            return Err(AppError::Validation {
                field: "ttl".into(),
                message: "срок действия кода должен быть положительным".into(),
            });
        }
        let now = self.clock.unix_seconds();
        let exp = now + ttl_secs;
        let mut secret = [0u8; 16];
        OsRng.fill_bytes(&mut secret);
        let code = link_proto::InviteCode {
            host: host.to_string(),
            port,
            spki,
            secret,
            exp_unix: exp,
        }
        .encode()
        .map_err(|_| AppError::Validation {
            field: "host".into(),
            message: "некорректный адрес сервера".into(),
        })?;
        let hash = link_proto::secret_hash(&secret).to_vec();
        let id = self
            .writer
            .execute(move |conn| {
                db(conn.execute(
                    "INSERT INTO link_invites (code_hash, created_by_user_id, created_at_utc, \
                     expires_at_utc) VALUES (?1, ?2, ?3, ?4)",
                    params![hash, admin_id, now, exp],
                ))?;
                Ok(conn.last_insert_rowid())
            })
            .await?;
        self.broadcast_link_changed("create_invite");
        Ok(InviteIssued {
            id,
            code,
            expires_at: exp,
        })
    }

    pub async fn list_invites(&self) -> Result<Vec<InviteRow>, AppError> {
        let now = self.clock.unix_seconds();
        self.read(move |conn| {
            let mut st = db(conn.prepare(
                "SELECT id, created_by_user_id, created_at_utc, expires_at_utc, used_at_utc, \
                 used_by_device_id, revoked_at_utc FROM link_invites ORDER BY id DESC",
            ))?;
            let rows = db(st.query_map([], |r| {
                let expires: i64 = r.get(3)?;
                let used: Option<i64> = r.get(4)?;
                let revoked: Option<i64> = r.get(6)?;
                Ok(InviteRow {
                    id: r.get(0)?,
                    status: invite_status(now, expires, used, revoked),
                    created_by_user_id: r.get(1)?,
                    created_at_utc: r.get(2)?,
                    expires_at_utc: expires,
                    used_at_utc: used,
                    used_by_device_id: r.get(5)?,
                })
            }))?;
            db(rows.collect::<rusqlite::Result<Vec<_>>>())
        })
        .await
    }

    pub async fn revoke_invite(&self, id: i64, admin_id: i64) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                let n = db(conn.execute(
                    "UPDATE link_invites SET revoked_at_utc = ?1, revoked_by_user_id = ?2 \
                     WHERE id = ?3 AND used_at_utc IS NULL AND revoked_at_utc IS NULL",
                    params![now, admin_id, id],
                ))?;
                if n == 1 {
                    return Ok(());
                }
                let exists: Option<i64> = db(conn
                    .query_row("SELECT id FROM link_invites WHERE id = ?1", [id], |r| {
                        r.get(0)
                    })
                    .optional())?;
                match exists {
                    None => Err(AppError::NotFound {
                        entity: "link_invite",
                        id,
                    }),
                    Some(_) => Err(AppError::Conflict {
                        reason: "код уже использован или отозван".into(),
                    }),
                }
            })
            .await?;
        self.broadcast_link_changed("revoke_invite");
        Ok(())
    }

    // ----- устройства ------------------------------------------------------

    /// Погашение кода (D-03/D-04). Возвращает `device_secret` — один раз.
    pub async fn enroll(&self, secret_b64: &str, meta: DeviceMeta) -> Result<String, EnrollError> {
        let raw = URL_SAFE_NO_PAD
            .decode(secret_b64.trim())
            .map_err(|_| EnrollError::InviteInvalid)?;
        let secret: [u8; 16] = raw
            .try_into()
            .map_err(|_: Vec<u8>| EnrollError::InviteInvalid)?;
        if meta.device_id.trim().is_empty() || meta.device_id.chars().count() > 64 {
            return Err(EnrollError::Internal(AppError::Validation {
                field: "deviceId".into(),
                message: "некорректный идентификатор устройства".into(),
            }));
        }
        let code_hash = link_proto::secret_hash(&secret).to_vec();
        let device_secret = URL_SAFE_NO_PAD.encode(random_32());
        let secret_hash = sha(&device_secret);
        let now = self.clock.unix_seconds();

        let out = self
            .writer
            .execute(move |conn| {
                // Один транзакционный блок на единственном писателе: погашение
                // и создание устройства атомарны (T-44-16).
                let tx = db(conn.transaction())?;
                let inv: Option<(i64, i64, Option<i64>, Option<i64>)> = db(tx
                    .query_row(
                        "SELECT id, expires_at_utc, used_at_utc, revoked_at_utc \
                           FROM link_invites WHERE code_hash = ?1",
                        [&code_hash],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                    )
                    .optional())?;
                let Some((id, expires, used, revoked)) = inv else {
                    return Ok(Err(EnrollError::InviteInvalid));
                };
                match invite_status(now, expires, used, revoked) {
                    "revoked" => return Ok(Err(EnrollError::InviteRevoked)),
                    "used" => return Ok(Err(EnrollError::InviteUsed)),
                    "expired" => return Ok(Err(EnrollError::InviteExpired)),
                    _ => {}
                }
                let exists: Option<String> = db(tx
                    .query_row(
                        "SELECT device_id FROM link_devices WHERE device_id = ?1",
                        [&meta.device_id],
                        |r| r.get(0),
                    )
                    .optional())?;
                if exists.is_some() {
                    return Ok(Err(EnrollError::DeviceExists));
                }
                let n = db(tx.execute(
                    "UPDATE link_invites SET used_at_utc = ?1, used_by_device_id = ?2 \
                     WHERE id = ?3 AND used_at_utc IS NULL AND revoked_at_utc IS NULL \
                       AND expires_at_utc > ?1",
                    params![now, meta.device_id, id],
                ))?;
                if n != 1 {
                    return Ok(Err(EnrollError::InviteUsed));
                }
                db(tx.execute(
                    "INSERT INTO link_devices (device_id, secret_hash, hostname, os, arch, \
                     client_version, enrolled_via_invite_id, first_seen_utc, last_seen_utc, \
                     last_ip) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?9)",
                    params![
                        meta.device_id,
                        secret_hash,
                        clip(&meta.hostname),
                        clip(&meta.os),
                        clip(&meta.arch),
                        clip(&meta.client_version),
                        id,
                        now,
                        meta.ip
                    ],
                ))?;
                db(tx.commit())?;
                Ok(Ok(()))
            })
            .await?;
        out?;
        self.broadcast_link_changed("enroll");
        Ok(device_secret)
    }

    pub async fn verify_device(&self, device_id: &str, secret: &str) -> Result<(), DeviceError> {
        self.verify_device_from(device_id, secret, None).await
    }

    /// Проверка устройства по заголовкам; `last_seen`/`last_ip` обновляются
    /// не чаще раза в минуту, чтобы не нагружать писателя.
    pub async fn verify_device_from(
        &self,
        device_id: &str,
        secret: &str,
        ip: Option<&str>,
    ) -> Result<(), DeviceError> {
        let did = device_id.to_string();
        let presented = sha(secret);
        let ip_owned = ip.map(str::to_string);
        let now = self.clock.unix_seconds();
        let row = {
            let did = did.clone();
            self.read(move |conn| {
                db(conn
                    .query_row(
                        "SELECT secret_hash, revoked_at_utc, last_seen_utc, last_ip \
                           FROM link_devices WHERE device_id = ?1",
                        [&did],
                        |r| {
                            Ok((
                                r.get::<_, Vec<u8>>(0)?,
                                r.get::<_, Option<i64>>(1)?,
                                r.get::<_, Option<i64>>(2)?,
                                r.get::<_, Option<String>>(3)?,
                            ))
                        },
                    )
                    .optional())
            })
            .await?
        };
        let Some((stored, revoked, last_seen, last_ip)) = row else {
            return Err(DeviceError::Unknown);
        };
        if !ct_eq(&stored, &presented) {
            return Err(DeviceError::BadSecret);
        }
        if revoked.is_some() {
            return Err(DeviceError::Banned);
        }
        let stale = last_seen.is_none_or(|t| now - t >= LAST_SEEN_THROTTLE_SECS);
        let ip_changed = ip_owned.is_some() && ip_owned != last_ip;
        if stale || ip_changed {
            self.writer
                .execute(move |conn| {
                    db(conn.execute(
                        "UPDATE link_devices SET last_seen_utc = ?1, \
                         last_ip = COALESCE(?2, last_ip) WHERE device_id = ?3",
                        params![now, ip_owned, did],
                    ))?;
                    Ok(())
                })
                .await?;
        }
        Ok(())
    }

    pub async fn allow_device(&self, device_id: &str) -> Result<(), AppError> {
        let did = device_id.to_string();
        self.writer
            .execute(move |conn| {
                let n = db(conn.execute(
                    "UPDATE link_devices SET revoked_at_utc = NULL, revoked_by_user_id = NULL \
                     WHERE device_id = ?1",
                    [&did],
                ))?;
                if n == 0 {
                    return Err(AppError::NotFound {
                        entity: "link_device",
                        id: 0,
                    });
                }
                Ok(())
            })
            .await?;
        self.broadcast_link_changed("allow_device");
        Ok(())
    }

    pub async fn list_banned_devices(&self) -> Result<Vec<BannedDeviceRow>, AppError> {
        self.read(|conn| {
            let mut st = db(conn.prepare(
                "SELECT device_id, hostname, revoked_at_utc, revoked_by_user_id \
                   FROM link_devices WHERE revoked_at_utc IS NOT NULL \
                  ORDER BY revoked_at_utc DESC",
            ))?;
            let rows = db(st.query_map([], |r| {
                Ok(BannedDeviceRow {
                    device_id: r.get(0)?,
                    hostname: r.get(1)?,
                    revoked_at_utc: r.get(2)?,
                    revoked_by_user_id: r.get(3)?,
                })
            }))?;
            db(rows.collect::<rusqlite::Result<Vec<_>>>())
        })
        .await
    }

    // ----- входы -----------------------------------------------------------

    pub async fn record_login(
        &self,
        device_id: &str,
        user_id: i64,
        method: LoginMethod,
    ) -> Result<LoginIssued, AppError> {
        let did = device_id.to_string();
        let now = self.clock.unix_seconds();
        let token = (method == LoginMethod::Password).then(|| URL_SAFE_NO_PAD.encode(random_32()));
        let token_hash = token.as_deref().map(sha);
        let login_id = self
            .writer
            .execute(move |conn| {
                let tx = db(conn.transaction())?;
                db(tx.execute(
                    "INSERT INTO link_logins (device_id, user_id, method, token_hash, \
                     created_at_utc, last_used_utc) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                    params![did, user_id, method.as_str(), token_hash, now],
                ))?;
                let id = tx.last_insert_rowid();
                db(tx.execute(
                    "UPDATE link_devices SET last_user_id = ?1 WHERE device_id = ?2",
                    params![user_id, did],
                ))?;
                db(tx.commit())?;
                Ok(id)
            })
            .await?;
        self.broadcast_link_changed("record_login");
        Ok(LoginIssued {
            login_id,
            device_token: token,
        })
    }

    pub async fn resume_by_token(
        &self,
        device_token: &str,
        device_id: &str,
    ) -> Result<LoginRow, ResumeError> {
        let th = sha(device_token);
        let did = device_id.to_string();
        let now = self.clock.unix_seconds();
        let out = self
            .writer
            .execute(move |conn| {
                type Raw = (i64, i64, String, String, Option<i64>);
                let row: Option<Raw> = db(conn
                    .query_row(
                        "SELECT id, user_id, device_id, method, revoked_at_utc \
                           FROM link_logins WHERE token_hash = ?1",
                        [&th],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                    )
                    .optional())?;
                let Some((login_id, user_id, dev, method, revoked)) = row else {
                    return Ok(Err(ResumeError::Invalid));
                };
                if dev != did {
                    return Ok(Err(ResumeError::Invalid));
                }
                if revoked.is_some() {
                    return Ok(Err(ResumeError::Revoked));
                }
                let banned: Option<Option<i64>> = db(conn
                    .query_row(
                        "SELECT revoked_at_utc FROM link_devices WHERE device_id = ?1",
                        [&dev],
                        |r| r.get(0),
                    )
                    .optional())?;
                let banned = banned.flatten();
                if banned.is_some() {
                    return Ok(Err(ResumeError::DeviceBanned));
                }
                if !user_active(conn, user_id)? {
                    return Ok(Err(ResumeError::UserInactive));
                }
                db(conn.execute(
                    "UPDATE link_logins SET last_used_utc = ?1 WHERE id = ?2",
                    params![now, login_id],
                ))?;
                Ok(Ok(LoginRow {
                    login_id,
                    user_id,
                    device_id: dev,
                    method,
                }))
            })
            .await?;
        out
    }

    /// Обычный «Выйти» (D-11): отзыв входа без запрета устройства.
    pub async fn logout(&self, login_id: i64) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        let device = self
            .writer
            .execute(move |conn| {
                let dev: Option<String> = db(conn
                    .query_row(
                        "SELECT device_id FROM link_logins WHERE id = ?1",
                        [login_id],
                        |r| r.get(0),
                    )
                    .optional())?;
                db(conn.execute(
                    "UPDATE link_logins SET revoked_at_utc = ?1 \
                     WHERE id = ?2 AND revoked_at_utc IS NULL",
                    params![now, login_id],
                ))?;
                Ok(dev)
            })
            .await?;
        if let Some(d) = device {
            self.hub.revoke_device(&d);
        }
        self.broadcast_link_changed("logout");
        Ok(())
    }

    /// D-13: отзыв входа + запрет устройства.
    pub async fn revoke_login(&self, login_id: i64, admin_id: i64) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        let device = self
            .writer
            .execute(move |conn| {
                let tx = db(conn.transaction())?;
                let dev: Option<String> = db(tx
                    .query_row(
                        "SELECT device_id FROM link_logins WHERE id = ?1",
                        [login_id],
                        |r| r.get(0),
                    )
                    .optional())?;
                let Some(dev) = dev else {
                    return Err(AppError::NotFound {
                        entity: "link_login",
                        id: login_id,
                    });
                };
                db(tx.execute(
                    "UPDATE link_logins SET revoked_at_utc = ?1, revoked_by_user_id = ?2 \
                     WHERE id = ?3 AND revoked_at_utc IS NULL",
                    params![now, admin_id, login_id],
                ))?;
                db(tx.execute(
                    "UPDATE link_devices SET revoked_at_utc = ?1, revoked_by_user_id = ?2 \
                     WHERE device_id = ?3 AND revoked_at_utc IS NULL",
                    params![now, admin_id, dev],
                ))?;
                db(tx.commit())?;
                Ok(dev)
            })
            .await?;
        self.hub.revoke_device(&device);
        self.broadcast_link_changed("revoke_login");
        Ok(())
    }

    /// D-12: все входы пользователя. `ban_devices=false` — блокировка
    /// пользователя (устройства остаются доступными другим).
    pub async fn revoke_user_logins(
        &self,
        user_id: i64,
        admin_id: i64,
        ban_devices: bool,
    ) -> Result<usize, AppError> {
        let now = self.clock.unix_seconds();
        let (count, devices) = self
            .writer
            .execute(move |conn| {
                let tx = db(conn.transaction())?;
                let devices: Vec<String> = {
                    let mut st = db(tx.prepare(
                        "SELECT DISTINCT device_id FROM link_logins \
                          WHERE user_id = ?1 AND revoked_at_utc IS NULL",
                    ))?;
                    let rows = db(st.query_map([user_id], |r| r.get(0)))?;
                    db(rows.collect::<rusqlite::Result<Vec<String>>>())?
                };
                let n = db(tx.execute(
                    "UPDATE link_logins SET revoked_at_utc = ?1, revoked_by_user_id = ?2 \
                     WHERE user_id = ?3 AND revoked_at_utc IS NULL",
                    params![now, admin_id, user_id],
                ))?;
                if ban_devices {
                    for d in &devices {
                        db(tx.execute(
                            "UPDATE link_devices SET revoked_at_utc = ?1, \
                             revoked_by_user_id = ?2 \
                             WHERE device_id = ?3 AND revoked_at_utc IS NULL",
                            params![now, admin_id, d],
                        ))?;
                    }
                }
                db(tx.commit())?;
                Ok((n, if ban_devices { devices } else { Vec::new() }))
            })
            .await?;
        self.hub.revoke_user(user_id);
        for d in &devices {
            self.hub.revoke_device(d);
        }
        self.broadcast_link_changed("revoke_user_logins");
        Ok(count)
    }

    pub async fn list_active_logins(&self) -> Result<Vec<ActiveLoginRow>, AppError> {
        self.read(|conn| {
            let mut st = db(conn.prepare(
                "SELECT l.id, l.user_id, u.login, u.full_name, l.device_id, d.hostname, \
                        l.method, l.created_at_utc, l.last_used_utc \
                   FROM link_logins l \
                   LEFT JOIN users u ON u.id = l.user_id \
                   LEFT JOIN link_devices d ON d.device_id = l.device_id \
                  WHERE l.revoked_at_utc IS NULL ORDER BY l.id DESC",
            ))?;
            let rows = db(st.query_map([], |r| {
                Ok(ActiveLoginRow {
                    login_id: r.get(0)?,
                    user_id: r.get(1)?,
                    user_login: r.get(2)?,
                    full_name: r.get(3)?,
                    device_id: r.get(4)?,
                    hostname: r.get(5)?,
                    method: r.get(6)?,
                    created_at_utc: r.get(7)?,
                    last_used_utc: r.get(8)?,
                })
            }))?;
            db(rows.collect::<rusqlite::Result<Vec<_>>>())
        })
        .await
    }

    /// D-12: активные входы для таблицы админки (новые сверху).
    pub async fn list_login_dtos(&self) -> Result<Vec<LinkLoginRowDto>, AppError> {
        let mut rows = self.list_active_logins().await?;
        rows.sort_by(|a, b| b.created_at_utc.cmp(&a.created_at_utc));
        Ok(rows
            .into_iter()
            .map(|r| {
                let login = r.user_login.unwrap_or_default();
                LinkLoginRowDto {
                    login_id: r.login_id,
                    user_id: r.user_id,
                    full_name: r.full_name.unwrap_or_else(|| login.clone()),
                    login,
                    hostname: r.hostname.unwrap_or_default(),
                    method: r.method,
                    created_at: r.created_at_utc,
                }
            })
            .collect())
    }

    /// D-13: запрещённые компьютеры с последним пользователем.
    pub async fn list_banned_dtos(&self) -> Result<Vec<BannedDeviceDto>, AppError> {
        self.read(|conn| {
            let mut st = db(conn.prepare(
                "SELECT d.device_id, d.hostname, u.full_name, d.revoked_at_utc \
                   FROM link_devices d LEFT JOIN users u ON u.id = d.last_user_id \
                  WHERE d.revoked_at_utc IS NOT NULL ORDER BY d.revoked_at_utc DESC, d.device_id",
            ))?;
            let rows = db(st.query_map([], |r| {
                Ok(BannedDeviceDto {
                    device_id: r.get(0)?,
                    hostname: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    last_user_full_name: r.get(2)?,
                    revoked_at: r.get(3)?,
                })
            }))?;
            db(rows.collect::<rusqlite::Result<Vec<_>>>())
        })
        .await
    }

    /// Коды для таблицы админки; использованные/истёкшие/отозванные старше 30 дней скрыты.
    pub async fn list_invite_dtos(&self) -> Result<Vec<InviteRowDto>, AppError> {
        let now = self.clock.unix_seconds();
        self.read(move |conn| {
            let mut st = db(conn.prepare(
                "SELECT i.id, i.created_at_utc, u.full_name, i.expires_at_utc, i.used_at_utc, \
                        i.revoked_at_utc, d.hostname \
                   FROM link_invites i \
                   LEFT JOIN users u ON u.id = i.created_by_user_id \
                   LEFT JOIN link_devices d ON d.device_id = i.used_by_device_id \
                  ORDER BY i.id DESC",
            ))?;
            let rows = db(st.query_map([], |r| {
                let expires: i64 = r.get(3)?;
                let used: Option<i64> = r.get(4)?;
                let revoked: Option<i64> = r.get(5)?;
                Ok((
                    InviteRowDto {
                        id: r.get(0)?,
                        created_at: r.get(1)?,
                        created_by_full_name: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        expires_at: expires,
                        status: invite_status(now, expires, used, revoked).to_string(),
                        used_by_hostname: r.get(6)?,
                    },
                    used,
                    revoked,
                ))
            }))?;
            let all = db(rows.collect::<rusqlite::Result<Vec<_>>>())?;
            Ok(all
                .into_iter()
                .filter(|(dto, used, revoked)| {
                    if dto.status == "active" {
                        return true;
                    }
                    // Момент, с которого код перестал быть активным.
                    let ended = match dto.status.as_str() {
                        "used" => used.unwrap_or(dto.expires_at),
                        "revoked" => revoked.unwrap_or(dto.expires_at),
                        _ => dto.expires_at,
                    };
                    now - ended <= INVITE_LIST_RETENTION_SECS
                })
                .map(|(dto, _, _)| dto)
                .collect())
        })
        .await
    }

    /// Счётчик активных входов пользователя для карточки.
    pub async fn user_link_status(&self, user_id: i64) -> Result<UserLinkStatusDto, AppError> {
        Ok(UserLinkStatusDto {
            active_logins: self.count_active_logins(user_id).await?,
        })
    }

    pub async fn count_active_logins(&self, user_id: i64) -> Result<i64, AppError> {
        self.read(move |conn| {
            db(conn.query_row(
                "SELECT COUNT(*) FROM link_logins WHERE user_id = ?1 AND revoked_at_utc IS NULL",
                [user_id],
                |r| r.get(0),
            ))
        })
        .await
    }

    /// Вход жив: не отозван, устройство не запрещено, пользователь активен.
    /// Ошибка чтения возвращается как `Err` — вызывающий не должен разлогинивать
    /// по ошибке БД.
    pub async fn is_login_valid(&self, login_id: i64) -> Result<bool, AppError> {
        self.read(move |conn| {
            let row: Option<(i64, Option<i64>, Option<i64>)> = db(conn
                .query_row(
                    "SELECT l.user_id, l.revoked_at_utc, d.revoked_at_utc \
                       FROM link_logins l \
                       LEFT JOIN link_devices d ON d.device_id = l.device_id \
                      WHERE l.id = ?1",
                    [login_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional())?;
            let Some((user_id, login_revoked, device_revoked)) = row else {
                return Ok(false);
            };
            if login_revoked.is_some() || device_revoked.is_some() {
                return Ok(false);
            }
            user_active(conn, user_id)
        })
        .await
    }
}

fn user_active(conn: &rusqlite::Connection, user_id: i64) -> Result<bool, AppError> {
    let row: Option<(i64, Option<i64>)> = db(conn
        .query_row(
            "SELECT is_active, deleted_at_utc FROM users WHERE id = ?1",
            [user_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional())?;
    Ok(matches!(row, Some((a, None)) if a != 0))
}
