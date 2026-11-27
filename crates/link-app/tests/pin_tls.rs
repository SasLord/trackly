//! Интеграция сетевого слоя Link с локальным TLS-сервером на rcgen. Сертификаты и ключи
//! генерируются в памяти, имена вымышленные.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use link_app::pin::{build_client_config, pin_matches};
use link_proto::spki::spki_sha256;
use rcgen::{CertificateParams, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::{TlsAcceptor, TlsConnector};

pub struct Ident {
    pub key: KeyPair,
    pub cert_der: Vec<u8>,
}

pub fn ident_with_key(key: KeyPair, san: &str) -> Ident {
    let params = CertificateParams::new(vec![san.to_string()]).unwrap();
    let cert = params.self_signed(&key).unwrap();
    Ident {
        cert_der: cert.der().to_vec(),
        key,
    }
}

pub fn ident(san: &str) -> Ident {
    ident_with_key(KeyPair::generate().unwrap(), san)
}

pub fn pin_of(id: &Ident) -> [u8; 32] {
    spki_sha256(&id.cert_der).unwrap()
}

fn server_config(cert_der: &[u8], key: &KeyPair) -> Arc<rustls::ServerConfig> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let cfg = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(cert_der.to_vec())],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der())),
        )
        .unwrap();
    Arc::new(cfg)
}

/// Эхо-сервер: считает принятые прикладные байты.
async fn spawn_echo(cert_der: &[u8], key: &KeyPair) -> (std::net::SocketAddr, Arc<AtomicUsize>) {
    let acceptor = TlsAcceptor::from(server_config(cert_der, key));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let bytes = Arc::new(AtomicUsize::new(0));
    let counter = bytes.clone();
    tokio::spawn(async move {
        loop {
            let Ok((tcp, _)) = listener.accept().await else {
                return;
            };
            let acceptor = acceptor.clone();
            let counter = counter.clone();
            tokio::spawn(async move {
                let Ok(mut tls) = acceptor.accept(tcp).await else {
                    return;
                };
                let mut buf = [0u8; 64];
                while let Ok(n) = tls.read(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                    counter.fetch_add(n, Ordering::SeqCst);
                    let _ = tls.write_all(&buf[..n]).await;
                }
            });
        }
    });
    (addr, bytes)
}

async fn try_roundtrip(
    addr: std::net::SocketAddr,
    config: rustls::ClientConfig,
) -> std::io::Result<Vec<u8>> {
    let tcp = tokio::net::TcpStream::connect(addr).await?;
    let mut tls = TlsConnector::from(Arc::new(config))
        .connect(ServerName::try_from("127.0.0.1").unwrap(), tcp)
        .await?;
    tls.write_all(b"hello").await?;
    let mut out = [0u8; 5];
    tls.read_exact(&mut out).await?;
    Ok(out.to_vec())
}

#[test]
fn pin_matches_same_spki_only() {
    let a = ident("srv.example.local");
    let b = ident("srv.example.local");
    assert!(pin_matches(&a.cert_der, &pin_of(&a)));
    assert!(!pin_matches(&a.cert_der, &pin_of(&b)));
    assert!(!pin_matches(b"not a cert", &pin_of(&a)));
}

#[tokio::test]
async fn pin_handshake_ok_by_ip_ignoring_name() {
    let id = ident("other.example.local");
    let (addr, _) = spawn_echo(&id.cert_der, &id.key).await;
    let (cfg, probe) = build_client_config(pin_of(&id));
    let echoed = try_roundtrip(addr, cfg).await.unwrap();
    assert_eq!(echoed, b"hello");
    assert!(probe.mismatch().is_none());
}

#[tokio::test]
async fn pin_wrong_pin_rejected_before_first_byte() {
    let id = ident("srv.example.local");
    let other = ident("srv.example.local");
    let (addr, bytes) = spawn_echo(&id.cert_der, &id.key).await;
    let (cfg, probe) = build_client_config(pin_of(&other));
    let res = try_roundtrip(addr, cfg).await;
    assert!(res.is_err(), "рукопожатие с чужим пином должно упасть");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(
        bytes.load(Ordering::SeqCst),
        0,
        "ни одного прикладного байта"
    );
    assert_eq!(probe.seen(), Some(pin_of(&id)));
    assert_ne!(probe.seen(), Some(pin_of(&other)));
    assert!(probe.mismatch().is_some());
}

#[tokio::test]
async fn pin_survives_reissue_on_same_key() {
    let key = KeyPair::generate().unwrap();
    let key2 = KeyPair::try_from(key.serialize_der().as_slice()).unwrap();
    let first = ident_with_key(key, "first.example.local");
    let second = ident_with_key(key2, "second.example.local");
    assert_ne!(first.cert_der, second.cert_der);
    let pin = pin_of(&first);
    let (addr, _) = spawn_echo(&second.cert_der, &second.key).await;
    let (cfg, _) = build_client_config(pin);
    assert_eq!(try_roundtrip(addr, cfg).await.unwrap(), b"hello");
}

#[tokio::test]
async fn pin_forged_signature_rejected() {
    // Сертификат A (верный SPKI), но подпись рукопожатия ключом B.
    let a = ident("srv.example.local");
    let b = ident("srv.example.local");
    let provider = rustls::crypto::ring::default_provider();
    let signing_key = provider
        .key_provider
        .load_private_key(PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
            b.key.serialize_der(),
        )))
        .unwrap();
    let certified = rustls::sign::CertifiedKey::new(
        vec![CertificateDer::from(a.cert_der.clone())],
        signing_key,
    );
    let cfg = rustls::ServerConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(StaticResolver(Arc::new(certified))));
    let acceptor = TlsAcceptor::from(Arc::new(cfg));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        if let Ok((tcp, _)) = listener.accept().await {
            let _ = acceptor.accept(tcp).await;
        }
    });
    let (cfg, _) = build_client_config(pin_of(&a));
    assert!(try_roundtrip(addr, cfg).await.is_err());
}

#[derive(Debug)]
struct StaticResolver(Arc<rustls::sign::CertifiedKey>);

impl rustls::server::ResolvesServerCert for StaticResolver {
    fn resolve(
        &self,
        _: rustls::server::ClientHello<'_>,
    ) -> Option<Arc<rustls::sign::CertifiedKey>> {
        Some(self.0.clone())
    }
}
