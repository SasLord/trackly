//! SPKI-отпечаток листового сертификата (D-06).

use sha2::{Digest, Sha256};
use x509_parser::parse_x509_certificate;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SpkiError {
    #[error("не удалось разобрать сертификат")]
    Parse,
    #[error("некорректный отпечаток (нужно 32 байта в hex)")]
    BadHex,
}

/// SHA-256 от DER-кодировки SubjectPublicKeyInfo (а не от всего сертификата):
/// отпечаток переживает перевыпуск сертификата на том же ключе.
pub fn spki_sha256(cert_der: &[u8]) -> Result<[u8; 32], SpkiError> {
    let (_, cert) = parse_x509_certificate(cert_der).map_err(|_| SpkiError::Parse)?;
    let digest = Sha256::digest(cert.tbs_certificate.subject_pki.raw);
    Ok(digest.into())
}

/// Формат `AB:CD:…` (95 символов), как `compute_fingerprint` на сервере.
pub fn fmt_colon_upper(h: &[u8; 32]) -> String {
    h.iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// Принимает hex в любом регистре, с разделителями `:` или без.
pub fn parse_colon_hex(s: &str) -> Result<[u8; 32], SpkiError> {
    let clean: String = s.chars().filter(|c| *c != ':').collect();
    if clean.len() != 64 || !clean.is_ascii() {
        return Err(SpkiError::BadHex);
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&clean[i * 2..i * 2 + 2], 16).map_err(|_| SpkiError::BadHex)?;
    }
    Ok(out)
}
