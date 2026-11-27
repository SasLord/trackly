//! Конфиг клиента Link (D-01, D-02, D-26) и локальное состояние устройства (D-27).
//!
//! Порядок источников: машинный (`link.toml` в системном каталоге), затем пользовательский
//! (локальные данные пользователя, не Roaming — Pitfall 7). Машинный побеждает ЦЕЛИКОМ
//! (сервер + пин): пользовательский при его наличии игнорируется, битый машинный конфиг не
//! подменяется молча пользовательским. Чистые функции принимают каталог параметром и не
//! зависят от Tauri; сетевых вызовов здесь нет.

use std::io::Write;
use std::path::{Path, PathBuf};

use link_proto::spki::{fmt_colon_upper, parse_colon_hex};
use serde::{Deserialize, Serialize};

/// Порт сервера Trackly по умолчанию.
pub const DEFAULT_PORT: u16 = 8443;

/// Откуда прочитан конфиг.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Machine,
    User,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("не удалось прочитать файл: {0}")]
    Io(String),
    #[error("файл конфигурации повреждён: {0}")]
    Parse(String),
    #[error("{0}")]
    Invalid(String),
}

/// Сырое содержимое `link.toml` до проверки.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RawConfig {
    #[serde(default)]
    pub server: String,
    #[serde(default)]
    pub pin_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spn: Option<String>,
}

/// Источник конфига. Для Фазы 45 здесь появится второй источник (HKLM) — сейчас не реализуется.
pub trait ConfigSource {
    /// `Ok(None)` — источника нет (файла нет), это не ошибка.
    fn load(&self) -> Result<Option<RawConfig>, ConfigError>;
    fn kind(&self) -> SourceKind;
}

/// Конфиг из TOML-файла.
pub struct FileConfigSource {
    pub path: PathBuf,
    pub kind: SourceKind,
}

impl FileConfigSource {
    pub fn new(path: PathBuf, kind: SourceKind) -> Self {
        Self { path, kind }
    }
}

impl ConfigSource for FileConfigSource {
    fn load(&self) -> Result<Option<RawConfig>, ConfigError> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(ConfigError::Io(e.to_string())),
        };
        toml::from_str(&text)
            .map(Some)
            .map_err(|e| ConfigError::Parse(e.to_string()))
    }

    fn kind(&self) -> SourceKind {
        self.kind
    }
}

/// Проверенный конфиг.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkConfig {
    pub host: String,
    pub port: u16,
    pub pin: [u8; 32],
    pub spn: Option<String>,
    pub source: SourceKind,
}

impl LinkConfig {
    /// `host:port` для отображения и URL (IPv6 в квадратных скобках).
    pub fn server_display(&self) -> String {
        format_server(&self.host, self.port)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigOutcome {
    Ready(LinkConfig),
    /// D-01: конфига нет нигде — экран первой настройки.
    NeedsSetup,
    Invalid {
        source: SourceKind,
        reason: String,
    },
}

fn format_server(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

/// Разбор адреса сервера: host, host:port, IPv4, `[IPv6]:port`; схема `https://` отбрасывается.
pub fn parse_server(input: &str) -> Result<(String, u16), ConfigError> {
    let bad = |m: &str| ConfigError::Invalid(m.to_string());
    let s = input.trim();
    let s = match s.get(..8) {
        Some(p) if p.eq_ignore_ascii_case("https://") => &s[8..],
        _ => s,
    };
    if s.is_empty() {
        return Err(bad("адрес сервера пуст"));
    }
    if s.chars()
        .any(|c| c.is_whitespace() || c == '/' || c == '\\' || c == '@')
    {
        return Err(bad("адрес сервера содержит недопустимые символы"));
    }
    let parse_port = |p: &str| -> Result<u16, ConfigError> {
        match p.parse::<u16>() {
            Ok(n) if n != 0 => Ok(n),
            _ => Err(bad("некорректный порт")),
        }
    };
    if let Some(rest) = s.strip_prefix('[') {
        let (host, tail) = rest
            .split_once(']')
            .ok_or_else(|| bad("нет закрывающей ]"))?;
        if host.is_empty() || !host.contains(':') {
            return Err(bad("некорректный адрес IPv6"));
        }
        let port = match tail {
            "" => DEFAULT_PORT,
            t => parse_port(
                t.strip_prefix(':')
                    .ok_or_else(|| bad("после ] ожидается :порт"))?,
            )?,
        };
        return Ok((host.to_string(), port));
    }
    if s.matches(':').count() > 1 {
        return Err(bad("IPv6 нужно писать в квадратных скобках"));
    }
    let (host, port) = match s.split_once(':') {
        Some((h, p)) => (h, parse_port(p)?),
        None => (s, DEFAULT_PORT),
    };
    if host.is_empty() {
        return Err(bad("адрес сервера пуст"));
    }
    Ok((host.to_string(), port))
}

fn validate(raw: RawConfig, source: SourceKind) -> Result<LinkConfig, ConfigError> {
    let (host, port) = parse_server(&raw.server)?;
    let pin = parse_colon_hex(raw.pin_sha256.trim())
        .map_err(|_| ConfigError::Invalid("pin_sha256: нужно 32 байта в hex".to_string()))?;
    let spn = raw
        .spn
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    Ok(LinkConfig {
        host,
        port,
        pin,
        spn,
        source,
    })
}

/// Выбор конфига по порядку источников (D-02). Первый найденный источник решает всё:
/// если он невалиден — это `Invalid`, а не переход к следующему (D-26).
pub fn resolve(sources: &[&dyn ConfigSource]) -> ConfigOutcome {
    for src in sources {
        let kind = src.kind();
        match src.load() {
            Ok(None) => continue,
            Ok(Some(raw)) => {
                return match validate(raw, kind) {
                    Ok(cfg) => ConfigOutcome::Ready(cfg),
                    Err(e) => ConfigOutcome::Invalid {
                        source: kind,
                        reason: e.to_string(),
                    },
                }
            }
            Err(e) => {
                return ConfigOutcome::Invalid {
                    source: kind,
                    reason: e.to_string(),
                }
            }
        }
    }
    ConfigOutcome::NeedsSetup
}

/// Путь машинного конфига (D-26).
pub fn machine_config_path() -> PathBuf {
    #[cfg(windows)]
    {
        let base = std::env::var_os("ProgramData")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
        base.join("Trackly Link").join("link.toml")
    }
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Library/Application Support/Trackly Link/link.toml")
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        PathBuf::from("/etc/trackly-link/link.toml")
    }
}

/// Каталог пользовательских данных Link: локальные данные пользователя, НЕ Roaming (T-44-15).
/// Link не portable (ADR 0005), поэтому штатный резолвер путей Tauri здесь уместен.
#[allow(clippy::disallowed_methods)]
pub fn user_config_dir(app: &tauri::AppHandle) -> Result<PathBuf, ConfigError> {
    use tauri::Manager;
    app.path()
        .app_local_data_dir()
        .map_err(|e| ConfigError::Io(e.to_string()))
}

pub const USER_CONFIG_FILE: &str = "link.toml";
pub const STATE_FILE: &str = "state.toml";

/// Источники в порядке приоритета для каталога пользователя `user_dir`.
pub fn standard_sources(user_dir: &Path) -> (FileConfigSource, FileConfigSource) {
    (
        FileConfigSource::new(machine_config_path(), SourceKind::Machine),
        FileConfigSource::new(user_dir.join(USER_CONFIG_FILE), SourceKind::User),
    )
}

/// Атомарная запись: временный файл в том же каталоге + rename. При ошибке целевой файл
/// не создаётся и не портится, временный удаляется.
fn atomic_write(path: &Path, content: &str) -> Result<(), ConfigError> {
    let io = |e: std::io::Error| ConfigError::Io(e.to_string());
    let dir = path
        .parent()
        .ok_or_else(|| ConfigError::Io("нет каталога".into()))?;
    std::fs::create_dir_all(dir).map_err(io)?;
    let tmp = path.with_extension("toml.tmp");
    let res = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if let Err(e) = res {
        let _ = std::fs::remove_file(&tmp);
        return Err(io(e));
    }
    Ok(())
}

/// Запись пользовательского конфига из экрана первой настройки. Вызывается только ПОСЛЕ
/// успешной проверки пина (D-03). Сервер проверяется до записи.
pub fn write_user_config(dir: &Path, server: &str, pin: &[u8; 32]) -> Result<(), ConfigError> {
    let (host, port) = parse_server(server)?;
    let raw = RawConfig {
        server: format_server(&host, port),
        pin_sha256: fmt_colon_upper(pin),
        spn: None,
    };
    let text = toml::to_string(&raw).map_err(|e| ConfigError::Parse(e.to_string()))?;
    atomic_write(&dir.join(USER_CONFIG_FILE), &text)
}

/// Локальное состояние устройства (D-27: `device_id` — здесь; секрет — в `secrets`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalState {
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub logged_out: bool,
    #[serde(default)]
    pub revoked: bool,
    #[serde(skip)]
    path: PathBuf,
}

impl LocalState {
    /// Читает `state.toml` из `dir`; при первом запуске создаёт `device_id` (UUIDv4) и сохраняет.
    pub fn load_or_init(dir: &Path) -> Result<Self, ConfigError> {
        let path = dir.join(STATE_FILE);
        let mut st = match std::fs::read_to_string(&path) {
            Ok(t) => {
                toml::from_str::<LocalState>(&t).map_err(|e| ConfigError::Parse(e.to_string()))?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => LocalState::default(),
            Err(e) => return Err(ConfigError::Io(e.to_string())),
        };
        st.path = path;
        if st.device_id.is_empty() {
            st.device_id = uuid::Uuid::new_v4().to_string();
            st.save()?;
        }
        Ok(st)
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let text = toml::to_string(self).map_err(|e| ConfigError::Parse(e.to_string()))?;
        atomic_write(&self.path, &text)
    }

    pub fn set_logged_out(&mut self, v: bool) -> Result<(), ConfigError> {
        self.logged_out = v;
        self.save()
    }

    pub fn set_revoked(&mut self, v: bool) -> Result<(), ConfigError> {
        self.revoked = v;
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIN_A: [u8; 32] = [0xAB; 32];
    const PIN_B: [u8; 32] = [0x12; 32];

    fn write(dir: &Path, name: &str, server: &str, pin: &str) -> FileConfigSource {
        let p = dir.join(name);
        std::fs::write(
            &p,
            format!("server = \"{server}\"\npin_sha256 = \"{pin}\"\n"),
        )
        .unwrap();
        FileConfigSource::new(
            p,
            if name == "m.toml" {
                SourceKind::Machine
            } else {
                SourceKind::User
            },
        )
    }

    fn pin_str(p: &[u8; 32]) -> String {
        fmt_colon_upper(p)
    }

    #[test]
    fn config_machine_wins_entirely_over_user() {
        let d = tempfile::tempdir().unwrap();
        let m = write(
            d.path(),
            "m.toml",
            "machine.example.local",
            &pin_str(&PIN_A),
        );
        let u = write(d.path(), "u.toml", "user.example.local", &pin_str(&PIN_B));
        match resolve(&[&m, &u]) {
            ConfigOutcome::Ready(c) => {
                assert_eq!(c.host, "machine.example.local");
                assert_eq!(c.pin, PIN_A);
                assert_eq!(c.source, SourceKind::Machine);
                assert_eq!(c.port, 8443);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn config_user_only_is_used() {
        let d = tempfile::tempdir().unwrap();
        let m = FileConfigSource::new(d.path().join("absent.toml"), SourceKind::Machine);
        let u = write(
            d.path(),
            "u.toml",
            "user.example.local:9000",
            &pin_str(&PIN_B),
        );
        match resolve(&[&m, &u]) {
            ConfigOutcome::Ready(c) => {
                assert_eq!(
                    (c.host.as_str(), c.port, c.pin),
                    ("user.example.local", 9000, PIN_B)
                );
                assert_eq!(c.source, SourceKind::User);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn config_none_means_needs_setup() {
        let d = tempfile::tempdir().unwrap();
        let m = FileConfigSource::new(d.path().join("a.toml"), SourceKind::Machine);
        let u = FileConfigSource::new(d.path().join("b.toml"), SourceKind::User);
        assert_eq!(resolve(&[&m, &u]), ConfigOutcome::NeedsSetup);
    }

    #[test]
    fn config_invalid_machine_is_not_replaced_by_user() {
        let d = tempfile::tempdir().unwrap();
        let u = write(d.path(), "u.toml", "user.example.local", &pin_str(&PIN_B));
        // битый TOML
        let p = d.path().join("m.toml");
        std::fs::write(&p, "server = [").unwrap();
        let m = FileConfigSource::new(p.clone(), SourceKind::Machine);
        assert!(matches!(
            resolve(&[&m, &u]),
            ConfigOutcome::Invalid {
                source: SourceKind::Machine,
                ..
            }
        ));
        // пин не 32 байта
        let m = write(d.path(), "m.toml", "machine.example.local", "AB:CD");
        assert!(matches!(
            resolve(&[&m, &u]),
            ConfigOutcome::Invalid {
                source: SourceKind::Machine,
                ..
            }
        ));
        // пустой server
        let m = write(d.path(), "m.toml", "", &pin_str(&PIN_A));
        assert!(matches!(
            resolve(&[&m, &u]),
            ConfigOutcome::Invalid {
                source: SourceKind::Machine,
                ..
            }
        ));
    }

    #[test]
    fn config_pin_formats() {
        let d = tempfile::tempdir().unwrap();
        let colon = pin_str(&PIN_A);
        let plain = colon.replace(':', "").to_lowercase();
        for pin in [colon.clone(), colon.to_lowercase(), plain] {
            let m = write(d.path(), "m.toml", "h.example.local", &pin);
            match resolve(&[&m]) {
                ConfigOutcome::Ready(c) => assert_eq!(c.pin, PIN_A),
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn config_server_forms() {
        assert_eq!(parse_server("host").unwrap(), ("host".into(), 8443));
        assert_eq!(parse_server("host:9443").unwrap(), ("host".into(), 9443));
        assert_eq!(parse_server("10.0.0.5").unwrap(), ("10.0.0.5".into(), 8443));
        assert_eq!(
            parse_server("[fe80::1]:9000").unwrap(),
            ("fe80::1".into(), 9000)
        );
        assert_eq!(parse_server("[fe80::1]").unwrap(), ("fe80::1".into(), 8443));
        assert_eq!(
            parse_server("https://host:9000").unwrap(),
            ("host".into(), 9000)
        );
        assert_eq!(parse_server("HTTPS://host").unwrap(), ("host".into(), 8443));
        for bad in [
            "",
            "ho st",
            "host/path",
            "https://host/x",
            "host:0",
            "host:abc",
            "fe80::1",
            "[::1",
            "user@host",
        ] {
            assert!(parse_server(bad).is_err(), "{bad}");
        }
        let c = LinkConfig {
            host: "fe80::1".into(),
            port: 8443,
            pin: PIN_A,
            spn: None,
            source: SourceKind::User,
        };
        assert_eq!(c.server_display(), "[fe80::1]:8443");
    }

    #[test]
    fn config_local_state_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let sub = d.path().join("nested");
        let a = LocalState::load_or_init(&sub).unwrap();
        assert!(uuid::Uuid::parse_str(&a.device_id).is_ok());
        let mut b = LocalState::load_or_init(&sub).unwrap();
        assert_eq!(a.device_id, b.device_id);
        b.set_logged_out(true).unwrap();
        b.set_revoked(true).unwrap();
        let c = LocalState::load_or_init(&sub).unwrap();
        assert!(c.logged_out && c.revoked);
        assert_eq!(c.device_id, a.device_id);
    }

    #[test]
    fn config_write_user_config_atomic_and_validated() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().join("u");
        // ошибка валидации: файл не создаётся
        assert!(write_user_config(&dir, "bad host", &PIN_A).is_err());
        assert!(!dir.join(USER_CONFIG_FILE).exists());
        // ошибка записи (каталог — это файл): ничего не создаётся
        let blocker = d.path().join("blocker");
        std::fs::write(&blocker, "x").unwrap();
        assert!(write_user_config(&blocker, "h.example.local", &PIN_A).is_err());
        // успех: читается обратно, временных файлов нет
        write_user_config(&dir, "h.example.local", &PIN_A).unwrap();
        let src = FileConfigSource::new(dir.join(USER_CONFIG_FILE), SourceKind::User);
        match resolve(&[&src]) {
            ConfigOutcome::Ready(c) => assert_eq!(
                (c.host.as_str(), c.port, c.pin),
                ("h.example.local", 8443, PIN_A)
            ),
            other => panic!("{other:?}"),
        }
        let names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names.len(), 1);
    }
}
