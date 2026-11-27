//! SPKI-пин (D-06..D-08): общий для HTTP и WS `rustls::ClientConfig`.
//!
//! Доверие держится только на SHA-256 от SPKI листового сертификата. Имя хоста, цепочка,
//! срок и издатель не проверяются (D-08), зато подписи рукопожатия проверяются штатными
//! алгоритмами провайдера: сертификат с верным SPKI, но подписью чужого ключа, отвергается.

use std::sync::{Arc, Mutex};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{verify_tls12_signature, verify_tls13_signature, WebPkiSupportedAlgorithms};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{CertificateError, ClientConfig, DigitallySignedStruct, Error, SignatureScheme};

use link_proto::spki::{fmt_colon_upper, spki_sha256};

/// Отпечаток не совпал: `expected` и `got` в формате `AB:CD:…` для экрана администратора.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PinError {
    #[error("отпечаток сервера не совпал с закреплённым")]
    Mismatch { expected: String, got: String },
}

/// Сравнение без раннего выхода: время не зависит от позиции первого различия.
pub fn ct_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut diff = 0u8;
    for i in 0..32 {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

/// Совпадает ли SPKI сертификата с пином. Нечитаемый сертификат не совпадает.
pub fn pin_matches(cert_der: &[u8], pin: &[u8; 32]) -> bool {
    match spki_sha256(cert_der) {
        Ok(got) => ct_eq(&got, pin),
        Err(_) => false,
    }
}

/// Ручка к результатам проверки: что закреплено и какой SPKI последний раз предъявил сервер.
#[derive(Clone)]
pub struct PinProbe {
    expected: [u8; 32],
    seen: Arc<Mutex<Option<[u8; 32]>>>,
}

impl std::fmt::Debug for PinProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PinProbe").finish_non_exhaustive()
    }
}

impl PinProbe {
    /// Последний предъявленный SPKI (None, если рукопожатие не дошло до сертификата).
    pub fn seen(&self) -> Option<[u8; 32]> {
        *self.seen.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Сбросить перед новой попыткой, чтобы старый отпечаток не выдал сетевую ошибку за подмену.
    pub fn clear(&self) {
        *self.seen.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    pub fn expected(&self) -> [u8; 32] {
        self.expected
    }

    /// Предъявленный отпечаток отличается от закреплённого?
    pub fn mismatch(&self) -> Option<PinError> {
        let got = self.seen()?;
        if ct_eq(&got, &self.expected) {
            None
        } else {
            Some(PinError::Mismatch {
                expected: fmt_colon_upper(&self.expected),
                got: fmt_colon_upper(&got),
            })
        }
    }
}

/// Определить, что ошибка соединения вызвана несовпадением пина (reqwest, tungstenite, io).
/// Рукопожатие с чужим SPKI завершиться успехом не может, поэтому чужой `seen` после
/// ошибки и есть отказ пина; ошибки без предъявленного сертификата остаются сетевыми.
/// Вызывающий обязан делать `probe.clear()` перед попыткой.
pub fn classify_tls_error(
    _err: &(dyn std::error::Error + 'static),
    probe: &PinProbe,
) -> Option<PinError> {
    probe.mismatch()
}

#[derive(Debug)]
struct PinVerifier {
    pin: [u8; 32],
    seen: Arc<Mutex<Option<[u8; 32]>>>,
    algs: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for PinVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>, // D-08: имя не сверяется
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        let got = spki_sha256(end_entity.as_ref())
            .map_err(|_| Error::InvalidCertificate(CertificateError::BadEncoding))?;
        *self.seen.lock().unwrap_or_else(|e| e.into_inner()) = Some(got);
        if ct_eq(&got, &self.pin) {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(Error::InvalidCertificate(
                CertificateError::ApplicationVerificationFailure,
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(message, cert, dss, &self.algs)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(message, cert, dss, &self.algs)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algs.supported_schemes()
    }
}

/// Единый `ClientConfig` для HTTP и WS. Провайдер ring задан явно, а не process-default
/// (в графе trackly-app живёт ещё и aws-lc-rs). ALPN только http/1.1.
pub fn build_client_config(pin: [u8; 32]) -> (ClientConfig, PinProbe) {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let seen = Arc::new(Mutex::new(None));
    let verifier = PinVerifier {
        pin,
        seen: seen.clone(),
        algs: provider.signature_verification_algorithms,
    };
    let mut config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("ring поддерживает протоколы по умолчанию")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    (
        config,
        PinProbe {
            expected: pin,
            seen,
        },
    )
}
