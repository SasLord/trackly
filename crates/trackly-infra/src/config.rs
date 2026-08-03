//! `AppConfig` — TOML-парсер `trackly.config.toml` (D-Config-01).
//!
//! Все секции и поля опциональны: отсутствующий файл → `Self::default()`
//! (НЕ ошибка), отсутствующие секции → их `Default::default()`. Дефолты
//! заданы вручную через `impl Default` для каждой секции (понятнее, чем
//! `#[serde(default = "...")]` paths-функции на каждое поле).
//!
//! Malformed TOML → `AppError::Validation { field: "trackly.config.toml", ... }`,
//! ловится в `main.rs` через `?` и печатается админу.
//!
//! Неизвестные ключи (forward-compat): `toml::from_str` по умолчанию их
//! игнорирует (мы не пишем `#[serde(deny_unknown_fields)]`) — это
//! сознательное решение, чтобы старый бинарь не падал при чтении конфига
//! от новой версии.

use std::io::ErrorKind;
use std::path::Path;

use serde::Deserialize;
use trackly_core::error::AppError;

/// Корневая структура `trackly.config.toml`.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct AppConfig {
    /// Параметры сервер-режима (axum + tower-sessions, Phase 5/Plan 05).
    #[serde(default)]
    pub server: ServerConfig,
    /// Переопределения путей (db_path; остальное диктует `Paths`).
    #[serde(default)]
    pub paths: PathsConfig,
    /// Параметры tracing (Plan 05).
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Параметры организации (timezone для UI; в БД всё в UTC).
    #[serde(default)]
    pub organization: OrganizationConfig,
    /// Параметры Active Directory (Phase 9, D-AD-01). Bootstrap-only:
    /// `enabled`/`use_mock` читаются отсюда, но live-настройки (вкл/выкл AD,
    /// автоприём, host/domain/base_dn) администратор редактирует через
    /// `app_settings` (plan 03 wires that) — TOML задаёт только дефолты на
    /// первом запуске и dev-переключатель мока.
    #[serde(default)]
    pub ad: AdConfig,
}

/// `[server]` — параметры HTTP/LAN сервера.
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    /// Включить ли встроенный axum сервер в режиме «доступ из локальной сети».
    pub enabled: bool,
    /// Адрес bind'а. По умолчанию `127.0.0.1` (только локальные подключения).
    pub host: String,
    /// Порт HTTP/HTTPS сервера.
    pub port: u16,
    /// Путь к TLS-сертификату. Пусто — генерируется self-signed на первом запуске.
    pub cert_path: String,
    /// Путь к TLS-приватному ключу (PEM). Пусто — выводится из `cert_path`
    /// (замена расширения на `.key`). WR-01: явное поле устраняет хрупкую
    /// эвристику и риск передать сам сертификат как ключ.
    #[serde(default)]
    pub key_path: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        // D-Config-01 дефолты.
        Self {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 8443,
            cert_path: String::new(),
            key_path: String::new(),
        }
    }
}

/// `[paths]` — переопределения путей. Сейчас только `db_path`.
///
/// `Default` производный: пустая строка `db_path` означает «использовать
/// `Paths::db_path()` дефолт» (`<exe_dir>/trackly.db`).
#[derive(Debug, Deserialize, Clone, Default)]
pub struct PathsConfig {
    /// Полный путь к SQLite-файлу. Пусто = `Paths::db_path()` default
    /// (`<exe_dir>/trackly.db`).
    pub db_path: String,
}

/// `[logging]` — параметры tracing.
#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    /// Уровень: `trace|debug|info|warn|error`.
    pub level: String,
    /// Формат: `compact|json`.
    pub format: String,
    /// Сколько дней хранить логи. Background-чистка — Phase 7.
    pub retention_days: u32,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "compact".to_string(),
            retention_days: 14,
        }
    }
}

/// `[organization]` — UI-параметры организации.
#[derive(Debug, Deserialize, Clone)]
pub struct OrganizationConfig {
    /// Часовой пояс для отображения в UI. В БД всё в UTC.
    pub timezone: String,
}

impl Default for OrganizationConfig {
    fn default() -> Self {
        Self {
            timezone: "Europe/Moscow".to_string(),
        }
    }
}

/// Одна запись таблицы «AD-группа → роль» (Phase 31, SSO-03).
///
/// `group_dn` — полный distinguished name группы (например
/// `CN=IT-Admins,OU=Groups,DC=example,DC=local`), НЕ короткое имя —
/// избегаем лишнего LDAP round-trip на резолв имени в DN (Pitfall 2).
/// `role` — строка, парсится в `trackly_core::auth::Role` через
/// `Role::from_str` в `RealAdDirectory` (не здесь — это чистый config-слой).
///
/// Безопасно деривить `Debug` — тут нет секретов, только DN группы и
/// имя роли.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct RoleMappingEntry {
    /// Полный DN AD-группы.
    pub group_dn: String,
    /// Имя роли (`"admin"` | `"manager"` | `"employee"`).
    pub role: String,
}

/// Дефолт TTL кэша displayName (сек) — длиннее, косметический риск низкий.
fn default_display_name_cache_ttl_secs() -> u64 {
    1800
}

/// Дефолт TTL кэша роли/группы (сек) — короче, прямое влияние на авторизацию.
fn default_group_cache_ttl_secs() -> u64 {
    300
}

/// `[ad]` — параметры Active Directory (Phase 9, D-AD-01 / D-Config-01).
///
/// Все поля, кроме `enabled`/`use_mock`/`port`/`name_attr`/`no_tls_verify`,
/// допускаются пустыми — пустая строка означает «auto-detect» (см.
/// `trackly_infra::ad::discovery`): `host`/`base_dn` выводятся из `domain`
/// или из DNS SRV/env на домен-joined Windows-хосте.
///
/// NOTE: `Debug` НЕ деривится — см. ручной `impl std::fmt::Debug for AdConfig`
/// ниже, который редактирует `bind_password` (Phase 31, T-31-02b).
#[derive(Deserialize, Clone)]
pub struct AdConfig {
    /// Включить ли AD-вход. Live-источник истины — `app_settings`; это поле
    /// — только bootstrap-дефолт на первом запуске.
    pub enabled: bool,
    /// Использовать `MockAdClient` вместо `RealAdClient` (D-Mock-01).
    /// `TRACKLY_AD_MOCK` env var имеет приоритет при runtime-switch.
    pub use_mock: bool,
    /// Хост контроллера домена. Пусто → auto-detect (DNS SRV / env).
    pub host: String,
    /// Порт LDAPS. По умолчанию 636.
    pub port: u16,
    /// DNS-суффикс домена (например `corp.local`). Пусто → auto-detect
    /// (`USERDNSDOMAIN`).
    pub domain: String,
    /// Base DN для LDAP-поиска (например `dc=corp,dc=local`). Пусто →
    /// выводится из `domain` (`derive_base_dn`).
    pub base_dn: String,
    /// Имя атрибута для ФИО (D-Config-02). По умолчанию `displayName`,
    /// fallback на `cn`, затем на логин.
    pub name_attr: String,
    /// Отключить проверку TLS-сертификата LDAPS (Pitfall 3). По умолчанию
    /// `false` — включается явно только в «Расширенные» как небезопасный
    /// opt-in для нестандартных сетей.
    pub no_tls_verify: bool,

    // ── AD SSO (Kerberos/SPNEGO passwordless вход, spike-002) ──────────────
    // Все три поля `#[serde(default)]` — старые конфиги без них парсятся,
    // а SSO по умолчанию выключен (безопасный дефолт).
    /// Включить passwordless-вход через AD (Kerberos/Negotiate) в server mode.
    /// Требует заполненных `spn` и `keytab_path`.
    #[serde(default)]
    pub sso_enabled: bool,
    /// Service Principal Name сервиса в форме `HTTP/host.domain` (например
    /// `HTTP/web.example.local`), под который сгенерирован keytab. Пусто → SSO
    /// не активируется. РЕАЛЬНОЕ значение задаётся в рантайм-конфиге рядом с БД
    /// (gitignored), в git — только плейсхолдеры.
    #[serde(default)]
    pub spn: String,
    /// Путь к `.keytab` (ktpass `/crypto AES256-SHA1 /out server.keytab`) рядом
    /// с исполняемым файлом. Читается при попытке SSO-входа; байты ключа службы
    /// никогда не логируются и не покидают процесс.
    #[serde(default)]
    pub keytab_path: String,

    // ── Служебный AD-bind (Phase 31, SSO-01/SSO-03) ─────────────────────────
    /// DN (или UPN) служебной учётной записи для read-only LDAP-bind
    /// (например `svc-trackly-ro@example.local`). Пусто → служебный bind не
    /// настроен, `RealAdDirectory::resolve` возвращает `DirectoryError::NotConfigured`
    /// (тихая деградация — SSO продолжает работать без обогащения ФИО/роли).
    #[serde(default)]
    pub bind_dn: String,
    /// Пароль служебной учётной записи. НИКОГДА не логируется/не печатается —
    /// см. ручной `impl std::fmt::Debug for AdConfig` ниже (T-31-02b).
    #[serde(default)]
    pub bind_password: String,
    /// TTL (сек) кэша резолва displayName. Дефолт 1800 (30 мин) — низкие
    /// ставки безопасности, длинный TTL приемлем. Именованная default-fn
    /// (не голый `#[serde(default)]`), потому что 0 секунд молча ломает
    /// кэширование целиком при частично заданной секции `[ad]`.
    #[serde(default = "default_display_name_cache_ttl_secs")]
    pub display_name_cache_ttl_secs: u64,
    /// TTL (сек) кэша резолва роли/группы. Дефолт 300 (5 мин) — короче
    /// displayName-кэша, т.к. напрямую влияет на авторизацию (доступ должен
    /// отзываться оперативно). Именованная default-fn по той же причине,
    /// что и выше.
    #[serde(default = "default_group_cache_ttl_secs")]
    pub group_cache_ttl_secs: u64,
    /// Таблица «AD-группа → роль», приоритет Admin > Manager > Employee
    /// (highest-privilege-wins) независимо от порядка записей. Пустой список —
    /// валидное стационарное состояние («маппинг не настроен»).
    #[serde(default)]
    pub role_mapping: Vec<RoleMappingEntry>,
    /// Список доверенных доменных логинов (Phase 32, SSO-02) в форме
    /// `sAMAccountName` (например `us100`). Матчинг case-insensitive и
    /// нормализует UPN/NetBIOS-формы к чистому логину — см.
    /// `trackly-app::services::auth::is_admin_login`. Любой логин из списка
    /// получает роль `admin` и немедленную активацию на КАЖДОМ AD/SSO-входе,
    /// в обход `ad_auto_accept`, pending-заявок и ручной блокировки (D-07) —
    /// это осознанная точка доверия деплою (тот, кто редактирует этот файл,
    /// может создать администратора). Пустой/отсутствующий список (дефолт) —
    /// фича полностью выключена (D-03). Не содержит секретов — безопасно
    /// печатать неотредактированным, в отличие от `bind_password`.
    #[serde(default)]
    pub admin_logins: Vec<String>,
}

impl std::fmt::Debug for AdConfig {
    /// Ручная (не производная) реализация: печатает все поля как есть,
    /// КРОМЕ `bind_password`, который редактируется как `"***"` — мирроринг
    /// `Secret<T>`'s `"***"`-конвенции (T-31-02b, `crates/trackly-core/src/primitives/secret.rs`).
    /// `AppConfig` (родитель) деривит `Debug` и корректно вызывает этот impl
    /// при форматировании поля `ad: AdConfig` — доп. работы не требуется.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdConfig")
            .field("enabled", &self.enabled)
            .field("use_mock", &self.use_mock)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("domain", &self.domain)
            .field("base_dn", &self.base_dn)
            .field("name_attr", &self.name_attr)
            .field("no_tls_verify", &self.no_tls_verify)
            .field("sso_enabled", &self.sso_enabled)
            .field("spn", &self.spn)
            .field("keytab_path", &self.keytab_path)
            .field("bind_dn", &self.bind_dn)
            .field("bind_password", &"***")
            .field(
                "display_name_cache_ttl_secs",
                &self.display_name_cache_ttl_secs,
            )
            .field("group_cache_ttl_secs", &self.group_cache_ttl_secs)
            .field("role_mapping", &self.role_mapping)
            .field("admin_logins", &self.admin_logins)
            .finish()
    }
}

impl Default for AdConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            use_mock: false,
            host: String::new(),
            port: 636,
            domain: String::new(),
            base_dn: String::new(),
            name_attr: "displayName".to_string(),
            no_tls_verify: false,
            sso_enabled: false,
            spn: String::new(),
            keytab_path: String::new(),
            bind_dn: String::new(),
            bind_password: String::new(),
            display_name_cache_ttl_secs: 1800,
            group_cache_ttl_secs: 300,
            role_mapping: Vec::new(),
            admin_logins: Vec::new(),
        }
    }
}

impl AppConfig {
    /// Парсит `trackly.config.toml` по указанному пути.
    ///
    /// - Файл отсутствует (`ErrorKind::NotFound`) → `Self::default()` (НЕ ошибка).
    /// - I/O-ошибка чтения → `AppError::Internal`.
    /// - Файл не парсится → `AppError::Validation { field: <file_name>, message }`.
    /// - Файл валиден → распарсенный `AppConfig`. Неизвестные поля
    ///   игнорируются (forward-compat).
    pub fn load_or_default(path: &Path) -> Result<Self, AppError> {
        let contents = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => {
                return Err(AppError::Internal {
                    source_chain: format!("read config {}: {e}", path.display()),
                });
            }
        };

        let field = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("trackly.config.toml")
            .to_string();

        toml::from_str::<Self>(&contents).map_err(|e| AppError::Validation {
            field,
            message: format!("TOML parse error: {e}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Behavior 1: empty file (whole `[ad]` section absent) still parses to
    /// sane, non-zero cache-TTL defaults — proves the new fields don't
    /// silently become 0 when a config omits the whole section.
    #[test]
    fn empty_config_gets_nonzero_cache_ttl_defaults() {
        let cfg: AppConfig = toml::from_str("").expect("empty config parses");
        assert_eq!(cfg.ad.display_name_cache_ttl_secs, 1800);
        assert_eq!(cfg.ad.group_cache_ttl_secs, 300);
        assert_eq!(cfg.ad.bind_dn, "");
        assert_eq!(cfg.ad.bind_password, "");
        assert_eq!(cfg.ad.role_mapping, Vec::new());
        assert_eq!(cfg.ad.admin_logins, Vec::<String>::new());
    }

    /// Behavior 2: an `[ad]` section present but omitting bind_dn/bind_password/
    /// the new TTL fields/role_mapping entirely still parses successfully with
    /// per-field defaults — proves partial-section defaults work too, not just
    /// whole-section-absent.
    #[test]
    fn partial_ad_section_gets_per_field_defaults() {
        let toml_str = "[ad]\n\
             enabled = true\n\
             use_mock = false\n\
             host = \"dc1.example.local\"\n\
             port = 636\n\
             domain = \"example.local\"\n\
             base_dn = \"dc=example,dc=local\"\n\
             name_attr = \"displayName\"\n\
             no_tls_verify = false\n";
        let cfg: AppConfig = toml::from_str(toml_str).expect("partial [ad] section parses");
        assert_eq!(cfg.ad.bind_dn, "");
        assert_eq!(cfg.ad.bind_password, "");
        assert_eq!(cfg.ad.display_name_cache_ttl_secs, 1800);
        assert_eq!(cfg.ad.group_cache_ttl_secs, 300);
        assert_eq!(cfg.ad.role_mapping, Vec::new());
        assert_eq!(cfg.ad.admin_logins, Vec::<String>::new());
    }

    /// Behavior 3: manual Debug impl redacts bind_password, both on AdConfig
    /// itself and transitively through the derived-Debug parent AppConfig.
    #[test]
    fn debug_impl_redacts_bind_password() {
        let mut ad = AdConfig {
            bind_dn: "svc-trackly-ro@example.local".to_string(),
            bind_password: "hunter2".to_string(),
            ..AdConfig::default()
        };
        let ad_debug = format!("{ad:?}");
        assert!(
            !ad_debug.contains("hunter2"),
            "AdConfig Debug must not leak bind_password: {ad_debug}"
        );
        assert!(
            ad_debug.contains("***"),
            "AdConfig Debug must redact bind_password as ***: {ad_debug}"
        );

        let app = AppConfig {
            ad: std::mem::replace(&mut ad, AdConfig::default()),
            ..AppConfig::default()
        };
        let app_debug = format!("{app:?}");
        assert!(
            !app_debug.contains("hunter2"),
            "AppConfig (derived Debug) must not leak bind_password transitively: {app_debug}"
        );
    }

    /// Behavior 4: a `role_mapping` TOML array-of-tables deserializes into
    /// `RoleMappingEntry` values. The base `[ad]` fields (which predate this
    /// plan and are NOT `#[serde(default)]`) must still be present for the
    /// table to parse — only the phase-31 fields are optional.
    #[test]
    fn role_mapping_array_of_tables_deserializes() {
        let toml_str = "[ad]\n\
             enabled = true\n\
             use_mock = false\n\
             host = \"dc1.example.local\"\n\
             port = 636\n\
             domain = \"example.local\"\n\
             base_dn = \"dc=example,dc=local\"\n\
             name_attr = \"displayName\"\n\
             no_tls_verify = false\n\
             [[ad.role_mapping]]\n\
             group_dn = \"CN=IT-Admins,OU=Groups,DC=example,DC=local\"\n\
             role = \"admin\"\n";
        let cfg: AppConfig = toml::from_str(toml_str).expect("role_mapping parses");
        assert_eq!(
            cfg.ad.role_mapping[0],
            RoleMappingEntry {
                group_dn: "CN=IT-Admins,OU=Groups,DC=example,DC=local".to_string(),
                role: "admin".to_string(),
            }
        );
    }

    /// Behavior 5 (Phase 32, SSO-02): `admin_logins` is a flat TOML string
    /// array (no `[[ad.admin_logins]]` table-array needed, unlike
    /// `role_mapping` which wraps a struct). Defaults to empty when absent,
    /// deserializes populated when present.
    #[test]
    fn admin_logins_flat_array_deserializes_and_defaults_empty() {
        let empty: AppConfig = toml::from_str("").expect("empty config parses");
        assert_eq!(empty.ad.admin_logins, Vec::<String>::new());

        let toml_str = "[ad]\n\
             enabled = true\n\
             use_mock = false\n\
             host = \"dc1.example.local\"\n\
             port = 636\n\
             domain = \"example.local\"\n\
             base_dn = \"dc=example,dc=local\"\n\
             name_attr = \"displayName\"\n\
             no_tls_verify = false\n\
             admin_logins = [\"us100\", \"us777\"]\n";
        let cfg: AppConfig = toml::from_str(toml_str).expect("admin_logins parses");
        assert_eq!(
            cfg.ad.admin_logins,
            vec!["us100".to_string(), "us777".to_string()]
        );
    }
}
