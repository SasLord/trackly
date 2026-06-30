//! TLS utilities — генерация self-signed сертификатов и загрузка из PEM файлов.
//!
//! Использует `rcgen` для генерации и `rustls` + `tokio-rustls` для TLS acceptor.
//! Fingerprint — SHA-256 DER, отображается пользователю для верификации.
//!
//! Portable: pure Rust, без OpenSSL DLL.

use std::sync::{Arc, Once};

use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::ServerConfig;
use sha2::{Digest, Sha256};
use tokio_rustls::TlsAcceptor;

static CRYPTO_PROVIDER_INIT: Once = Once::new();

/// Установить процесс-level rustls `CryptoProvider` (ring) ровно один раз.
///
/// Нужно, потому что в графе зависимостей одновременно присутствуют `ring`
/// (через `rcgen`/`tokio-rustls`) и `aws-lc-rs` (транзитивно через `ldap3`),
/// поэтому rustls 0.23 не может автоматически выбрать провайдер — без явного
/// `install_default()` любой вызов `ServerConfig::builder()` паникует в
/// runtime. Выбираем pure-Rust `ring`, а не `aws-lc-rs`, чтобы не тащить
/// C-тулчейн/NASM в portable Windows-сборку (см. CLAUDE.md).
pub fn ensure_crypto_provider() {
    CRYPTO_PROVIDER_INIT.call_once(|| {
        // Игнорируем Result: Err означает, что другой вызывающий уже
        // установил провайдер — нам достаточно, что хоть какой-то есть.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// TLS bundle: готовый acceptor + fingerprint + PEM для сохранения на диск.
pub struct TlsBundle {
    /// Готовый TLS acceptor для принятия TLS соединений.
    pub acceptor: TlsAcceptor,
    /// SHA-256 fingerprint DER-сертификата в формате XX:XX:XX:... (95 символов).
    ///
    /// Отображается пользователю в UI для верификации подключения браузера.
    pub fingerprint_hex: String,
    /// PEM-encoded сертификат — для сохранения на диск при первом запуске.
    pub cert_pem: String,
    /// PEM-encoded приватный ключ — для сохранения на диск (перезагрузка).
    ///
    /// **Никогда не отправлять клиенту и не логировать.**
    pub key_pem: String,
}

/// Вычисляет SHA-256 fingerprint DER-байт в формате XX:XX:XX:...
///
/// 32 байта → 32*2 hex + 31 двоеточие = 95 символов.
fn compute_fingerprint(der_bytes: &[u8]) -> String {
    let hash = Sha256::digest(der_bytes);
    hash.iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// Создать `rustls::ServerConfig` из сертификата и ключа в DER.
fn build_server_config(cert_der: Vec<u8>, key_der: Vec<u8>) -> anyhow::Result<ServerConfig> {
    ensure_crypto_provider();
    let certs = vec![CertificateDer::from(cert_der)];
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_der));
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;
    Ok(config)
}

/// `true`, если `host` — wildcard/unspecified адрес (`0.0.0.0`, `::`) или пустая
/// строка. В этом случае сервер слушает на всех интерфейсах, и сам `host` не
/// годится для SAN сертификата — браузеры из LAN подключаются по реальному
/// IP машины, а не по `0.0.0.0`.
fn is_wildcard_host(host: &str) -> bool {
    let h = host.trim();
    h.is_empty()
        || matches!(
            h.parse::<std::net::IpAddr>(),
            Ok(ip) if ip.is_unspecified()
        )
}

/// Проверяет, что строка пригодна как DNS-имя в SAN (RFC 1123 label-ish):
/// только ASCII буквы/цифры/`-`/`.`, непустая, без ведущих/замыкающих точек.
/// rcgen отвергает невалидные DNS-имена ошибкой — фильтруем заранее, чтобы
/// странный hostname не ронял генерацию всего сертификата.
fn is_valid_dns_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.ends_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
}

/// Собрать список subject-alt-names для self-signed сертификата.
///
/// - Не-wildcard `host` → `[host, "localhost"]` (прежнее поведение).
/// - Wildcard/unspecified `host` (`0.0.0.0`, `::`, пусто) → `"localhost"` +
///   все non-loopback IPv4/IPv6 адреса машины (как IP-SAN) + OS hostname
///   (если валиден как DNS-имя). Это убирает hostname-mismatch ошибку, когда
///   браузер из LAN подключается по `https://<LAN-IP>:port`.
///
/// rcgen авто-классифицирует каждую строку: парсится как `IpAddr` → IP-SAN,
/// иначе DnsName (`CertificateParams::new`), поэтому достаточно класть строки.
///
/// Дедуплицирует с сохранением порядка. Если детект интерфейсов вернул пусто
/// (нет non-loopback адресов / ошибка перечисления), список содержит только
/// `"localhost"` (+ hostname) — генерация не падает, но в этом случае
/// IP-mismatch ожидаем (см. unit-тест).
fn collect_subject_alt_names(host: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut push_unique = |s: String| {
        if !s.is_empty() && !names.contains(&s) {
            names.push(s);
        }
    };

    if is_wildcard_host(host) {
        push_unique("localhost".to_string());

        // Перечислить интерфейсы; не-loopback, не-unspecified IPv4/IPv6 → IP-SAN.
        match if_addrs::get_if_addrs() {
            Ok(ifaces) => {
                for iface in ifaces {
                    let ip = iface.ip();
                    if ip.is_loopback() || ip.is_unspecified() {
                        continue;
                    }
                    push_unique(ip.to_string());
                }
            }
            Err(e) => {
                tracing::warn!("collect_subject_alt_names: if_addrs failed: {e}");
            }
        }

        // OS hostname — даёт браузерам путь https://<machine-name>:port.
        if let Ok(h) = hostname::get() {
            let h = h.to_string_lossy().into_owned();
            if is_valid_dns_name(&h) && h != "localhost" {
                push_unique(h);
            }
        }
    } else {
        push_unique(host.to_string());
        push_unique("localhost".to_string());
    }

    names
}

/// Генерировать self-signed TLS сертификат для указанного хоста.
///
/// SAN включает `host` и `"localhost"`. Если `host` — wildcard (`0.0.0.0`/`::`/
/// пусто), вместо бесполезного `0.0.0.0` в SAN кладутся реальные non-loopback
/// IP машины (IP-SAN) и OS hostname — чтобы LAN-браузеры по `https://<LAN-IP>`
/// не получали hostname-mismatch (см. `collect_subject_alt_names`).
/// Возвращает `TlsBundle` с готовым acceptor и SHA-256 fingerprint.
///
/// Используется при первом включении server mode (D-Server-04).
pub fn generate_self_signed(host: &str) -> anyhow::Result<TlsBundle> {
    let subject_alt_names = collect_subject_alt_names(host);
    let rcgen::CertifiedKey { cert, signing_key } = generate_simple_self_signed(subject_alt_names)?;

    let cert_der = cert.der().to_vec();
    let key_der = signing_key.serialize_der();

    let fingerprint_hex = compute_fingerprint(&cert_der);
    let cert_pem = cert.pem();
    let key_pem = signing_key.serialize_pem();

    let config = build_server_config(cert_der, key_der)?;

    Ok(TlsBundle {
        acceptor: TlsAcceptor::from(Arc::new(config)),
        fingerprint_hex,
        cert_pem,
        key_pem,
    })
}

/// Определить путь к приватному ключу для заданного `cert_path` (WR-01).
///
/// - Если `key_path` непустой — используется как есть.
/// - Иначе путь выводится из `cert_path` заменой расширения на `.key`.
///
/// Возвращает ошибку, если итоговый путь к ключу совпадает с `cert_path`
/// (иначе мы попытались бы скормить сертификат функции загрузки ключа —
/// что давало бы запутанную ошибку «no private key found»).
pub fn resolve_key_path(cert_path: &str, key_path: &str) -> anyhow::Result<String> {
    let resolved = if !key_path.is_empty() {
        key_path.to_string()
    } else {
        let p = std::path::Path::new(cert_path);
        p.with_extension("key").to_string_lossy().into_owned()
    };

    if resolved == cert_path {
        anyhow::bail!(
            "resolve_key_path: путь к ключу совпадает с путём к сертификату ({cert_path}); \
             укажите server.key_path явно"
        );
    }
    Ok(resolved)
}

/// Загрузить TLS bundle из cert/key файлов с валидацией путей (WR-01).
///
/// Читает сертификат и (выведенный/явный) ключ, проверяя, что это разные
/// файлы и что ключ парсится. Объединяет логику, ранее дублированную в
/// HTTP- и Tauri-транспортах.
pub fn load_from_files(cert_path: &str, key_path: &str) -> anyhow::Result<TlsBundle> {
    let cert_pem = std::fs::read_to_string(cert_path)
        .map_err(|e| anyhow::anyhow!("read cert {cert_path}: {e}"))?;
    let resolved_key = resolve_key_path(cert_path, key_path)?;
    let key_pem = std::fs::read_to_string(&resolved_key)
        .map_err(|e| anyhow::anyhow!("read key {resolved_key}: {e}"))?;
    load_from_pem(&cert_pem, &key_pem)
}

/// Загрузить TLS bundle из PEM строк (пользовательский cert/key).
///
/// Вычисляет fingerprint из первого сертификата в PEM.
pub fn load_from_pem(cert_pem: &str, key_pem: &str) -> anyhow::Result<TlsBundle> {
    ensure_crypto_provider();
    use rustls_pemfile::{certs, private_key};

    // Parse certificates
    let cert_bytes = cert_pem.as_bytes();
    let mut cert_reader = std::io::BufReader::new(cert_bytes);
    let cert_ders: Vec<CertificateDer<'static>> =
        certs(&mut cert_reader).collect::<Result<Vec<_>, _>>()?;

    if cert_ders.is_empty() {
        anyhow::bail!("load_from_pem: no certificates found in cert_pem");
    }

    // Fingerprint from first cert
    let fingerprint_hex = compute_fingerprint(cert_ders[0].as_ref());

    // Parse private key
    let key_bytes = key_pem.as_bytes();
    let mut key_reader = std::io::BufReader::new(key_bytes);
    let key_der = private_key(&mut key_reader)?
        .ok_or_else(|| anyhow::anyhow!("load_from_pem: no private key found in key_pem"))?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_ders, key_der)?;

    Ok(TlsBundle {
        acceptor: TlsAcceptor::from(Arc::new(config)),
        fingerprint_hex,
        cert_pem: cert_pem.to_string(),
        key_pem: key_pem.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Регрессия: `ServerConfig::builder()` ранее паниковал в runtime, потому
    /// что одновременно в графе зависимостей есть `ring` и `aws-lc-rs`, и
    /// rustls 0.23 не может автоматически выбрать процесс-level
    /// `CryptoProvider`. `ensure_crypto_provider()` (вызывается первой строкой
    /// в `build_server_config`) должен устранить панику.
    #[test]
    fn generate_self_signed_does_not_panic() {
        let bundle = generate_self_signed("127.0.0.1").expect("tls bundle");
        assert_eq!(bundle.fingerprint_hex.len(), 95);
        assert!(bundle.cert_pem.contains("BEGIN CERTIFICATE"));
    }

    #[test]
    fn is_wildcard_host_classifies_correctly() {
        assert!(is_wildcard_host("0.0.0.0"));
        assert!(is_wildcard_host("::"));
        assert!(is_wildcard_host(""));
        assert!(is_wildcard_host("  "));
        assert!(!is_wildcard_host("127.0.0.1"));
        assert!(!is_wildcard_host("192.168.1.10"));
        assert!(!is_wildcard_host("localhost"));
        assert!(!is_wildcard_host("trackly.local"));
    }

    /// Не-wildcard host сохраняет прежнее поведение: `[host, "localhost"]`.
    #[test]
    fn collect_sans_non_wildcard_unchanged() {
        assert_eq!(
            collect_subject_alt_names("192.168.1.10"),
            vec!["192.168.1.10".to_string(), "localhost".to_string()]
        );
        // host == localhost дедуплицируется.
        assert_eq!(
            collect_subject_alt_names("localhost"),
            vec!["localhost".to_string()]
        );
    }

    /// Для wildcard host SAN-список должен включать хотя бы один обнаруженный
    /// non-loopback LAN IP. Если детект интерфейсов на тестовой машине ничего
    /// не вернул (CI-окружение без non-loopback адресов), мы это документируем
    /// и не падаем: тогда список содержит только "localhost" (+ возможный
    /// hostname), но НЕ литеральный "0.0.0.0".
    #[test]
    fn collect_sans_wildcard_includes_detected_lan_ip() {
        let sans = collect_subject_alt_names("0.0.0.0");

        // Литеральный wildcard никогда не должен попадать в SAN.
        assert!(
            !sans.iter().any(|s| s == "0.0.0.0" || s == "::"),
            "wildcard literal must not appear in SAN: {sans:?}"
        );
        // localhost всегда присутствует.
        assert!(sans.iter().any(|s| s == "localhost"), "SAN: {sans:?}");

        // Независимо перечисляем non-loopback IP, чтобы понять, ожидаем ли
        // мы их в SAN на этой машине.
        let detected: Vec<String> = if_addrs::get_if_addrs()
            .map(|ifaces| {
                ifaces
                    .into_iter()
                    .map(|i| i.ip())
                    .filter(|ip| !ip.is_loopback() && !ip.is_unspecified())
                    .map(|ip| ip.to_string())
                    .collect()
            })
            .unwrap_or_default();

        if detected.is_empty() {
            // Документируем: на этой машине нет non-loopback интерфейсов —
            // IP-SAN добавить неоткуда. Это допустимо (генерация не падает).
            eprintln!(
                "collect_sans_wildcard_includes_detected_lan_ip: no non-loopback \
                 interfaces detected on this host — skipping LAN-IP assertion"
            );
        } else {
            assert!(
                detected.iter().any(|ip| sans.contains(ip)),
                "expected at least one detected LAN IP {detected:?} in SAN {sans:?}"
            );
        }
    }
}
