//! Хранилище секретов Link (D-10, D-27) с деградацией вместо падения.
//!
//! * токен входа: хранилище ОС, а без него ТОЛЬКО память; в файл не попадает никогда;
//! * `device_secret`: хранилище ОС, а без него файл с правами 0600 (осознанное отступление
//!   от ADR 0002, D-27: секрет устройства отзываем сервером);
//! * пароль не сохраняется нигде (в этом модуле его нет вообще).
//!
//! Значения секретов не логируются: только факты вида «хранилище недоступно».

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::config::LocalState;

/// Имя службы записей в хранилище ОС.
pub const SERVICE: &str = "org.trackly.link";
/// Переменная окружения: не обращаться к хранилищу ОС (тесты, UAT без запросов Keychain).
pub const NO_KEYRING_ENV: &str = "TRACKLY_LINK_NO_KEYRING";

#[derive(Debug, thiserror::Error)]
#[error("хранилище секретов: {0}")]
pub struct SecretError(pub String);

type Result<T> = std::result::Result<T, SecretError>;

/// Секрет: Debug скрыт, память затирается при drop.
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(***)")
    }
}

/// Ключ записи `device_secret` для сервера (разные серверы не смешиваются).
pub fn device_secret_key(host: &str, port: u16) -> String {
    format!("device_secret:{host}:{port}")
}

/// Ключ записи токена входа для сервера.
pub fn login_token_key(host: &str, port: u16) -> String {
    format!("login_token:{host}:{port}")
}

pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<Secret>>;
    fn set(&self, key: &str, value: &Secret) -> Result<()>;
    fn delete(&self, key: &str) -> Result<()>;
}

/// Только память процесса.
#[derive(Default)]
pub struct InMemoryStore {
    map: Mutex<HashMap<String, Secret>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Secret>> {
        self.map.lock().unwrap_or_else(|p| p.into_inner())
    }
}

impl SecretStore for InMemoryStore {
    fn get(&self, key: &str) -> Result<Option<Secret>> {
        Ok(self.lock().get(key).cloned())
    }

    fn set(&self, key: &str, value: &Secret) -> Result<()> {
        self.lock().insert(key.to_string(), value.clone());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<()> {
        self.lock().remove(key);
        Ok(())
    }
}

/// Файл на ключ в каталоге локальных данных приложения; права 0600 на unix.
/// Используется ТОЛЬКО для `device_secret` (D-27).
pub struct FileSecretStore {
    dir: PathBuf,
}

impl FileSecretStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn path(&self, key: &str) -> PathBuf {
        let safe: String = key
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.dir.join(format!("{safe}.secret"))
    }
}

fn io_err(e: std::io::Error) -> SecretError {
    SecretError(e.to_string())
}

impl SecretStore for FileSecretStore {
    fn get(&self, key: &str) -> Result<Option<Secret>> {
        match std::fs::read_to_string(self.path(key)) {
            Ok(s) => Ok(Some(Secret::new(s))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(io_err(e)),
        }
    }

    fn set(&self, key: &str, value: &Secret) -> Result<()> {
        use std::io::Write;
        std::fs::create_dir_all(&self.dir).map_err(io_err)?;
        let path = self.path(key);
        let tmp = path.with_extension("secret.tmp");
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let res = (|| {
            let mut f = opts.open(&tmp)?;
            f.write_all(value.expose().as_bytes())?;
            f.sync_all()?;
            std::fs::rename(&tmp, &path)
        })();
        if let Err(e) = res {
            let _ = std::fs::remove_file(&tmp);
            return Err(io_err(e));
        }
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<()> {
        match std::fs::remove_file(self.path(key)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_err(e)),
        }
    }
}

/// Хранилище учётных данных ОС через keyring-core. Один `Entry` не используется из нескольких
/// потоков одновременно: обращения сериализованы мьютексом.
pub struct KeyringStore {
    gate: Mutex<()>,
}

impl KeyringStore {
    /// Устанавливает хранилище ОС по умолчанию. Любая ошибка — `None`: хранилища нет (D-10).
    pub fn init() -> Option<Self> {
        let res: std::result::Result<(), String> = (|| {
            #[cfg(windows)]
            keyring_core::set_default_store(
                windows_native_keyring_store::Store::new().map_err(|e| e.to_string())?,
            );
            #[cfg(target_os = "macos")]
            keyring_core::set_default_store(
                apple_native_keyring_store::keychain::Store::new().map_err(|e| e.to_string())?,
            );
            #[cfg(target_os = "linux")]
            keyring_core::set_default_store(
                zbus_secret_service_keyring_store::Store::new().map_err(|e| e.to_string())?,
            );
            #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
            return Err("платформа без хранилища ключей".to_string());
            #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
            Ok(())
        })();
        match res {
            Ok(()) => Some(Self::attach()),
            Err(_) => {
                tracing::warn!("хранилище ключей ОС недоступно, секреты в памяти или файле");
                None
            }
        }
    }

    /// Использовать уже установленное `keyring_core` хранилище по умолчанию.
    pub fn attach() -> Self {
        Self {
            gate: Mutex::new(()),
        }
    }

    fn entry(key: &str) -> Result<keyring_core::Entry> {
        keyring_core::Entry::new(SERVICE, key).map_err(|e| SecretError(e.to_string()))
    }

    fn guard(&self) -> std::sync::MutexGuard<'_, ()> {
        self.gate.lock().unwrap_or_else(|p| p.into_inner())
    }
}

impl SecretStore for KeyringStore {
    fn get(&self, key: &str) -> Result<Option<Secret>> {
        let _g = self.guard();
        match Self::entry(key)?.get_secret() {
            Ok(bytes) => String::from_utf8(bytes)
                .map(|s| Some(Secret::new(s)))
                .map_err(|_| SecretError("значение не UTF-8".into())),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(e) => Err(SecretError(e.to_string())),
        }
    }

    fn set(&self, key: &str, value: &Secret) -> Result<()> {
        let _g = self.guard();
        Self::entry(key)?
            .set_secret(value.expose().as_bytes())
            .map_err(|e| SecretError(e.to_string()))
    }

    fn delete(&self, key: &str) -> Result<()> {
        let _g = self.guard();
        match Self::entry(key)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(e) => Err(SecretError(e.to_string())),
        }
    }
}

/// Значение `TRACKLY_LINK_NO_KEYRING` считается включённым при `1`/`true`.
pub fn no_keyring_flag(value: Option<&str>) -> bool {
    matches!(
        value.map(str::trim),
        Some("1") | Some("true") | Some("TRUE")
    )
}

/// Выбор основного хранилища: при флаге обхода `init` не вызывается (ОС не трогаем).
pub fn select_primary(
    no_keyring: bool,
    init: impl FnOnce() -> Option<Box<dyn SecretStore>>,
) -> Option<Box<dyn SecretStore>> {
    if no_keyring {
        None
    } else {
        init()
    }
}

/// Основное хранилище по окружению и ОС.
pub fn primary_from_env() -> Option<Box<dyn SecretStore>> {
    let flag = no_keyring_flag(std::env::var(NO_KEYRING_ENV).ok().as_deref());
    select_primary(flag, || {
        KeyringStore::init().map(|k| Box::new(k) as Box<dyn SecretStore>)
    })
}

/// Токен входа: ОС-хранилище, иначе только память (D-10). Файлового хранилища тут НЕТ.
pub struct TokenStore {
    primary: Option<Box<dyn SecretStore>>,
    memory: InMemoryStore,
    degraded: AtomicBool,
}

impl TokenStore {
    pub fn new(primary: Option<Box<dyn SecretStore>>) -> Self {
        Self {
            primary,
            memory: InMemoryStore::new(),
            degraded: AtomicBool::new(false),
        }
    }

    /// Переживёт ли токен закрытие Link (для `LinkEvent.secrets_persistent`).
    pub fn persistent(&self) -> bool {
        self.primary.is_some() && !self.degraded.load(Ordering::Relaxed)
    }

    fn degrade(&self) {
        if !self.degraded.swap(true, Ordering::Relaxed) {
            tracing::warn!("хранилище ключей отказало, токен входа только в памяти");
        }
    }

    pub fn get(&self, key: &str) -> Option<Secret> {
        if let Some(p) = &self.primary {
            match p.get(key) {
                Ok(Some(v)) => return Some(v),
                Ok(None) => {}
                Err(_) => self.degrade(),
            }
        }
        self.memory.get(key).ok().flatten()
    }

    pub fn set(&self, key: &str, value: &Secret) {
        if let Some(p) = &self.primary {
            if p.set(key, value).is_ok() {
                let _ = self.memory.delete(key);
                return;
            }
            self.degrade();
        }
        let _ = self.memory.set(key, value);
    }

    pub fn delete(&self, key: &str) {
        if let Some(p) = &self.primary {
            if p.delete(key).is_err() {
                self.degrade();
            }
        }
        let _ = self.memory.delete(key);
    }
}

/// `device_secret`: ОС-хранилище, иначе файл 0600 (D-27).
pub struct DeviceSecretStore {
    primary: Option<Box<dyn SecretStore>>,
    fallback: FileSecretStore,
}

impl DeviceSecretStore {
    pub fn new(primary: Option<Box<dyn SecretStore>>, fallback: FileSecretStore) -> Self {
        Self { primary, fallback }
    }

    pub fn get(&self, key: &str) -> Result<Option<Secret>> {
        if let Some(p) = &self.primary {
            if let Ok(Some(v)) = p.get(key) {
                return Ok(Some(v));
            }
        }
        self.fallback.get(key)
    }

    pub fn set(&self, key: &str, value: &Secret) -> Result<()> {
        if let Some(p) = &self.primary {
            if p.set(key, value).is_ok() {
                // Копия в файле больше не нужна.
                let _ = self.fallback.delete(key);
                return Ok(());
            }
            tracing::warn!("хранилище ключей отказало, секрет устройства в файле");
        }
        self.fallback.set(key, value)
    }

    pub fn delete(&self, key: &str) -> Result<()> {
        if let Some(p) = &self.primary {
            let _ = p.delete(key);
        }
        self.fallback.delete(key)
    }
}

/// Идентичность устройства: `device_id` из локального состояния, секрет из хранилища.
/// Секрета нет, пока устройство не подключено (его выдаёт сервер при подключении).
#[derive(Debug)]
pub struct DeviceIdentity {
    pub device_id: String,
    pub device_secret: Option<Secret>,
}

impl DeviceIdentity {
    pub fn load(
        local: &LocalState,
        store: &DeviceSecretStore,
        host: &str,
        port: u16,
    ) -> Result<Self> {
        Ok(Self {
            device_id: local.device_id.clone(),
            device_secret: store.get(&device_secret_key(host, port))?,
        })
    }
}

/// Каталог файлового хранилища секрета устройства (рядом с локальным состоянием).
pub fn file_store_dir(user_dir: &Path) -> PathBuf {
    user_dir.join("secrets")
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Failing;
    impl SecretStore for Failing {
        fn get(&self, _: &str) -> Result<Option<Secret>> {
            Err(SecretError("отказ".into()))
        }
        fn set(&self, _: &str, _: &Secret) -> Result<()> {
            Err(SecretError("отказ".into()))
        }
        fn delete(&self, _: &str) -> Result<()> {
            Err(SecretError("отказ".into()))
        }
    }

    #[test]
    fn secrets_in_memory_roundtrip() {
        let s = InMemoryStore::new();
        assert!(s.get("k").unwrap().is_none());
        s.set("k", &Secret::new("v")).unwrap();
        assert_eq!(s.get("k").unwrap().unwrap().expose(), "v");
        s.delete("k").unwrap();
        assert!(s.get("k").unwrap().is_none());
    }

    #[test]
    fn secrets_file_store_is_0600_and_roundtrips() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().join("secrets");
        let s = FileSecretStore::new(&dir);
        let key = device_secret_key("h.example.local", 8443);
        s.set(&key, &Secret::new("abc")).unwrap();
        assert_eq!(s.get(&key).unwrap().unwrap().expose(), "abc");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for e in std::fs::read_dir(&dir).unwrap() {
                let mode = e.unwrap().metadata().unwrap().permissions().mode() & 0o777;
                assert_eq!(mode, 0o600);
            }
        }
        s.set(&key, &Secret::new("def")).unwrap();
        assert_eq!(s.get(&key).unwrap().unwrap().expose(), "def");
        s.delete(&key).unwrap();
        assert!(s.get(&key).unwrap().is_none());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    }

    #[test]
    fn secrets_token_store_degrades_to_memory() {
        let t = TokenStore::new(Some(Box::new(Failing)));
        assert!(t.persistent());
        t.set("k", &Secret::new("tok"));
        assert!(!t.persistent());
        assert_eq!(t.get("k").unwrap().expose(), "tok");
        t.delete("k");
        assert!(t.get("k").is_none());
    }

    #[test]
    fn secrets_token_never_written_to_file_store() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().join("secrets");
        // Файловое хранилище существует рядом (как у device_secret), токен идёт мимо него.
        let _device = DeviceSecretStore::new(None, FileSecretStore::new(&dir));
        let t = TokenStore::new(None);
        t.set(&login_token_key("h", 1), &Secret::new("tok"));
        assert_eq!(t.get(&login_token_key("h", 1)).unwrap().expose(), "tok");
        assert!(!t.persistent());
        assert!(!dir.exists() || std::fs::read_dir(&dir).unwrap().count() == 0);
    }

    #[test]
    fn secrets_no_keyring_env_skips_os() {
        assert!(no_keyring_flag(Some("1")));
        assert!(!no_keyring_flag(Some("0")));
        assert!(!no_keyring_flag(None));
        let mut called = false;
        let p = select_primary(true, || {
            called = true;
            None
        });
        assert!(p.is_none() && !called);
        let _ = select_primary(false, || {
            called = true;
            None
        });
        assert!(called);
    }

    #[test]
    fn secrets_debug_is_hidden() {
        let s = Secret::new("очень-секретно");
        assert_eq!(format!("{s:?}"), "Secret(***)");
        let id = DeviceIdentity {
            device_id: "d".into(),
            device_secret: Some(s),
        };
        assert!(!format!("{id:?}").contains("секретно"));
    }

    #[test]
    fn secrets_keys_and_service_scoped_per_server() {
        assert_eq!(device_secret_key("h", 8443), "device_secret:h:8443");
        assert_eq!(login_token_key("h", 8443), "login_token:h:8443");
        assert_ne!(login_token_key("h", 8443), login_token_key("h", 9000));
        assert_eq!(SERVICE, "org.trackly.link");
        // Единственный тест, трогающий глобальное хранилище keyring (mock).
        keyring_core::set_default_store(keyring_core::mock::Store::new().unwrap());
        let k = KeyringStore::attach();
        let key = device_secret_key("h", 8443);
        assert!(k.get(&key).unwrap().is_none());
        k.set(&key, &Secret::new("zz")).unwrap();
        let raw = keyring_core::Entry::new(SERVICE, &key)
            .unwrap()
            .get_secret()
            .unwrap();
        assert_eq!(raw, b"zz");
        assert_eq!(k.get(&key).unwrap().unwrap().expose(), "zz");
        k.delete(&key).unwrap();
        assert!(k.get(&key).unwrap().is_none());
    }

    #[test]
    fn secrets_device_store_fallback_and_identity() {
        let d = tempfile::tempdir().unwrap();
        let store = DeviceSecretStore::new(
            Some(Box::new(Failing)),
            FileSecretStore::new(d.path().join("secrets")),
        );
        let local = LocalState::load_or_init(d.path()).unwrap();
        let id = DeviceIdentity::load(&local, &store, "h", 1).unwrap();
        assert!(id.device_secret.is_none());
        store
            .set(&device_secret_key("h", 1), &Secret::new("dev"))
            .unwrap();
        let id = DeviceIdentity::load(&local, &store, "h", 1).unwrap();
        assert_eq!(id.device_secret.unwrap().expose(), "dev");
        assert_eq!(id.device_id, local.device_id);
    }
}
