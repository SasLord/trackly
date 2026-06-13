//! TLS utilities — генерация self-signed сертификатов и загрузка из PEM файлов.
//!
//! Использует `rcgen` для генерации и `rustls` + `tokio-rustls` для TLS acceptor.
//! Fingerprint — SHA-256 DER, отображается пользователю для верификации.
//!
//! Portable: pure Rust, без OpenSSL DLL.

use std::sync::Arc;

use rcgen::generate_simple_self_signed;
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use sha2::{Digest, Sha256};
use tokio_rustls::TlsAcceptor;

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
fn build_server_config(
    cert_der: Vec<u8>,
    key_der: Vec<u8>,
) -> anyhow::Result<ServerConfig> {
    let certs = vec![CertificateDer::from(cert_der)];
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_der));
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;
    Ok(config)
}

/// Генерировать self-signed TLS сертификат для указанного хоста.
///
/// SAN включает `host` и `"localhost"` для удобства локального тестирования.
/// Возвращает `TlsBundle` с готовым acceptor и SHA-256 fingerprint.
///
/// Используется при первом включении server mode (D-Server-04).
pub fn generate_self_signed(host: &str) -> anyhow::Result<TlsBundle> {
    let subject_alt_names = vec![host.to_string(), "localhost".to_string()];
    let rcgen::CertifiedKey { cert, signing_key } =
        generate_simple_self_signed(subject_alt_names)?;

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
    use rustls_pemfile::{certs, private_key};

    // Parse certificates
    let cert_bytes = cert_pem.as_bytes();
    let mut cert_reader = std::io::BufReader::new(cert_bytes);
    let cert_ders: Vec<CertificateDer<'static>> = certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()?;

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
