//! Golden-фикстура SPKI-отпечатка (T-44-07). Фикстура — одноразовый сертификат
//! CN=fixture.example.local; ключ в репозиторий не кладётся.

use link_proto::{fmt_colon_upper, parse_colon_hex, spki_sha256, SpkiError};
use sha2::{Digest, Sha256};

const FIXTURE_PEM: &str = include_str!("fixtures/spki_fixture.crt.pem");
const GOLDEN: &str = include_str!("fixtures/spki_golden.txt");

fn pem_to_der(pem: &str) -> Vec<u8> {
    use base64::Engine;
    let b64: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();
    base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .expect("pem")
}

#[test]
fn spki_golden_matches() {
    let der = pem_to_der(FIXTURE_PEM);
    let h = spki_sha256(&der).unwrap();
    assert_eq!(fmt_colon_upper(&h), GOLDEN.trim());
}

#[test]
fn spki_differs_from_whole_der_hash() {
    let der = pem_to_der(FIXTURE_PEM);
    let spki = spki_sha256(&der).unwrap();
    let whole: [u8; 32] = Sha256::digest(&der).into();
    assert_ne!(spki, whole);
}

#[test]
fn fixture_cn_is_expected() {
    let der = pem_to_der(FIXTURE_PEM);
    let (_, cert) = x509_parser::parse_x509_certificate(&der).unwrap();
    let cn = cert
        .subject()
        .iter_common_name()
        .next()
        .unwrap()
        .as_str()
        .unwrap();
    assert_eq!(cn, "fixture.example.local");
}

#[test]
fn same_key_different_san_same_spki() {
    let key = rcgen::KeyPair::generate().unwrap();
    let a = rcgen::CertificateParams::new(vec!["a.example.local".into()])
        .unwrap()
        .self_signed(&key)
        .unwrap();
    let b = rcgen::CertificateParams::new(vec!["b.example.local".into()])
        .unwrap()
        .self_signed(&key)
        .unwrap();
    assert_eq!(spki_sha256(a.der()).unwrap(), spki_sha256(b.der()).unwrap());
    assert_ne!(a.der().as_ref(), b.der().as_ref());
}

#[test]
fn fmt_and_parse_roundtrip() {
    let h = [0xABu8; 32];
    let s = fmt_colon_upper(&h);
    assert_eq!(s.len(), 95);
    assert!(s.starts_with("AB:AB:"));
    assert_eq!(parse_colon_hex(&s).unwrap(), h);
    assert_eq!(parse_colon_hex(&s.to_lowercase()).unwrap(), h);
    assert_eq!(parse_colon_hex(&s.replace(':', "")).unwrap(), h);
    assert_eq!(parse_colon_hex("AB:CD"), Err(SpkiError::BadHex));
    assert_eq!(parse_colon_hex(&"AB".repeat(33)), Err(SpkiError::BadHex));
}

#[test]
fn garbage_der_is_parse_error() {
    assert_eq!(spki_sha256(&[1, 2, 3, 4]), Err(SpkiError::Parse));
}

/// Одноразовый генератор фикстуры: `cargo test -p link-proto gen_fixture -- --ignored --nocapture`
#[test]
#[ignore]
fn gen_fixture() {
    let key = rcgen::KeyPair::generate().unwrap();
    let mut p = rcgen::CertificateParams::new(vec!["fixture.example.local".into()]).unwrap();
    p.distinguished_name
        .push(rcgen::DnType::CommonName, "fixture.example.local");
    let cert = p.self_signed(&key).unwrap();
    println!("{}", cert.pem());
    println!(
        "GOLDEN {}",
        fmt_colon_upper(&spki_sha256(cert.der()).unwrap())
    );
}
