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

// ---------------------------------------------------------------- HTTP

use link_app::http_client::{DeviceCreds, HttpError, LinkHttp};

struct HttpSrv {
    addr: std::net::SocketAddr,
    requests: Arc<AtomicUsize>,
    connections: Arc<AtomicUsize>,
    heads: Arc<std::sync::Mutex<Vec<String>>>,
}

/// Минимальный HTTP/1.1 поверх TLS: keep-alive, пути /ok /cookie /needs-cookie /revoked /boom /redirect /drop.
async fn spawn_http(id: &Ident) -> HttpSrv {
    let acceptor = TlsAcceptor::from(server_config(&id.cert_der, &id.key));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let srv = HttpSrv {
        addr,
        requests: Arc::new(AtomicUsize::new(0)),
        connections: Arc::new(AtomicUsize::new(0)),
        heads: Arc::new(std::sync::Mutex::new(Vec::new())),
    };
    let (requests, connections, heads) = (
        srv.requests.clone(),
        srv.connections.clone(),
        srv.heads.clone(),
    );
    tokio::spawn(async move {
        loop {
            let Ok((tcp, _)) = listener.accept().await else {
                return;
            };
            let (acceptor, requests, connections, heads) = (
                acceptor.clone(),
                requests.clone(),
                connections.clone(),
                heads.clone(),
            );
            tokio::spawn(async move {
                let Ok(mut tls) = acceptor.accept(tcp).await else {
                    return;
                };
                connections.fetch_add(1, Ordering::SeqCst);
                let mut buf: Vec<u8> = Vec::new();
                loop {
                    let mut chunk = [0u8; 1024];
                    let head_end = loop {
                        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                            break p + 4;
                        }
                        match tls.read(&mut chunk).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => buf.extend_from_slice(&chunk[..n]),
                        }
                    };
                    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
                    let clen = head
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                        })
                        .unwrap_or(0);
                    while buf.len() < head_end + clen {
                        match tls.read(&mut chunk).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => buf.extend_from_slice(&chunk[..n]),
                        }
                    }
                    buf.drain(..head_end + clen);
                    requests.fetch_add(1, Ordering::SeqCst);
                    let path = head.split_whitespace().nth(1).unwrap_or("/").to_string();
                    let has_cookie = head.to_ascii_lowercase().contains("cookie: sid=abc");
                    heads.lock().unwrap().push(head);
                    let (status, extra, body) = match path.as_str() {
                        "/ok" => ("200 OK", "", r#"{"ok":true}"#.to_string()),
                        "/cookie" => (
                            "200 OK",
                            "Set-Cookie: sid=abc; Path=/\r\n",
                            r#"{"ok":true}"#.to_string(),
                        ),
                        "/needs-cookie" => ("200 OK", "", format!(r#"{{"ok":{has_cookie}}}"#)),
                        "/revoked" => (
                            "401 Unauthorized",
                            "",
                            r#"{"code":"LINK_REVOKED","message":"отозван"}"#.to_string(),
                        ),
                        "/boom" => ("500 Internal Server Error", "", "{}".to_string()),
                        "/redirect" => ("302 Found", "Location: /ok\r\n", String::new()),
                        "/drop" => return,
                        _ => ("404 Not Found", "", "{}".to_string()),
                    };
                    let resp = format!(
                        "HTTP/1.1 {status}\r\n{extra}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                        body.len()
                    );
                    if tls.write_all(resp.as_bytes()).await.is_err() {
                        return;
                    }
                }
            });
        }
    });
    srv
}

fn link_http(srv: &HttpSrv, pin: [u8; 32], dev: Option<DeviceCreds>) -> LinkHttp {
    LinkHttp::new("127.0.0.1", srv.addr.port(), pin, dev).unwrap()
}

#[tokio::test]
async fn http_get_json_with_valid_pin() {
    let id = ident("srv.example.local");
    let srv = spawn_http(&id).await;
    let http = link_http(&srv, pin_of(&id), None);
    let v: serde_json::Value = http.get_json("/ok").await.unwrap();
    assert_eq!(v["ok"], true);
}

#[tokio::test]
async fn http_wrong_pin_is_mismatch_and_server_sees_no_request() {
    let id = ident("srv.example.local");
    let other = ident("srv.example.local");
    let srv = spawn_http(&id).await;
    let http = link_http(&srv, pin_of(&other), None);
    let err = http.get_json::<serde_json::Value>("/ok").await.unwrap_err();
    match err {
        HttpError::PinMismatch { expected, got } => {
            assert_eq!(expected, link_proto::spki::fmt_colon_upper(&pin_of(&other)));
            assert_eq!(got, link_proto::spki::fmt_colon_upper(&pin_of(&id)));
        }
        e => panic!("ожидали PinMismatch, получили {e:?}"),
    }
    assert_eq!(srv.requests.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn http_classifies_revoked_server_and_network() {
    let id = ident("srv.example.local");
    let srv = spawn_http(&id).await;
    let http = link_http(&srv, pin_of(&id), None);
    let e = http
        .get_json::<serde_json::Value>("/revoked")
        .await
        .unwrap_err();
    assert_eq!(
        e,
        HttpError::Api {
            status: 401,
            code: Some("LINK_REVOKED".into()),
            message: Some("отозван".into())
        }
    );
    assert_eq!(
        http.get_json::<serde_json::Value>("/boom")
            .await
            .unwrap_err(),
        HttpError::Server(500)
    );
    // Обрыв соединения без ответа.
    assert_eq!(
        http.get_json::<serde_json::Value>("/drop")
            .await
            .unwrap_err(),
        HttpError::Network
    );
    // Отказ соединения: порт, на котором никто не слушает.
    let free = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = free.local_addr().unwrap().port();
    drop(free);
    let dead = LinkHttp::new("127.0.0.1", port, pin_of(&id), None).unwrap();
    assert_eq!(
        dead.get_json::<serde_json::Value>("/ok").await.unwrap_err(),
        HttpError::Network
    );
}

#[tokio::test]
async fn http_cookie_jar_roundtrip() {
    let id = ident("srv.example.local");
    let srv = spawn_http(&id).await;
    let http = link_http(&srv, pin_of(&id), None);
    assert!(http.cookie_header().is_none());
    let before: serde_json::Value = http.get_json("/needs-cookie").await.unwrap();
    assert_eq!(before["ok"], false);
    let _: serde_json::Value = http.get_json("/cookie").await.unwrap();
    assert_eq!(http.cookie_header().as_deref(), Some("sid=abc"));
    let after: serde_json::Value = http.get_json("/needs-cookie").await.unwrap();
    assert_eq!(after["ok"], true);
}

#[tokio::test]
async fn http_no_redirects_and_one_connection_reused() {
    let id = ident("srv.example.local");
    let srv = spawn_http(&id).await;
    let http = link_http(&srv, pin_of(&id), None);
    let e = http
        .get_json::<serde_json::Value>("/redirect")
        .await
        .unwrap_err();
    assert!(matches!(e, HttpError::Api { status: 302, .. }), "{e:?}");
    // Редирект не выполнен: на сервер пришёл ровно один запрос.
    assert_eq!(srv.requests.load(Ordering::SeqCst), 1);
    for _ in 0..3 {
        let _: serde_json::Value = http.get_json("/ok").await.unwrap();
    }
    assert_eq!(srv.requests.load(Ordering::SeqCst), 4);
    assert_eq!(
        srv.connections.load(Ordering::SeqCst),
        1,
        "один TCP на все раунды"
    );
}

#[tokio::test]
async fn http_device_headers_sent_and_secret_not_in_debug() {
    let id = ident("srv.example.local");
    let srv = spawn_http(&id).await;
    let creds = DeviceCreds {
        device_id: "dev-0001".into(),
        secret: "s3cr3t-value-xyz".into(),
    };
    let http = link_http(&srv, pin_of(&id), Some(creds.clone()));
    let _: serde_json::Value = http.get_json("/ok").await.unwrap();
    let head = srv.heads.lock().unwrap()[0].to_ascii_lowercase();
    assert!(head.contains("x-link-device: dev-0001"));
    assert!(head.contains("x-link-secret: s3cr3t-value-xyz"));
    assert!(!format!("{creds:?}").contains("s3cr3t-value-xyz"));
    assert!(!format!("{http:?}").contains("s3cr3t-value-xyz"));
}
