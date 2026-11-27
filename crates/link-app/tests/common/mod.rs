//! Общий мок сервера Link для интеграционных тестов потоков: TLS на rcgen (имена вымышленные),
//! минимальный HTTP/1.1 со скриптуемым обработчиком и журналом запросов.
#![allow(dead_code)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use link_proto::spki::spki_sha256;
use rcgen::{CertificateParams, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

pub struct Ident {
    pub key: KeyPair,
    pub cert_der: Vec<u8>,
}

pub fn ident(san: &str) -> Ident {
    let key = KeyPair::generate().unwrap();
    let params = CertificateParams::new(vec![san.to_string()]).unwrap();
    let cert = params.self_signed(&key).unwrap();
    Ident {
        cert_der: cert.der().to_vec(),
        key,
    }
}

pub fn pin_of(id: &Ident) -> [u8; 32] {
    spki_sha256(&id.cert_der).unwrap()
}

#[derive(Debug, Clone)]
pub struct Req {
    pub method: String,
    pub path: String,
    /// Заголовки в нижнем регистре, одной строкой.
    pub head: String,
    pub body: String,
}

pub struct Resp {
    pub status: u16,
    pub body: String,
}

pub fn json(status: u16, body: &str) -> Resp {
    Resp {
        status,
        body: body.to_string(),
    }
}

pub fn api_err(status: u16, code: &str) -> Resp {
    json(
        status,
        &format!(r#"{{"code":"{code}","message":"вымышленное сообщение"}}"#),
    )
}

pub type Handler = Arc<dyn Fn(&Req) -> Resp + Send + Sync>;

pub struct MockSrv {
    pub port: u16,
    pub log: Arc<Mutex<Vec<Req>>>,
    pub connections: Arc<AtomicUsize>,
}

impl MockSrv {
    pub fn requests(&self) -> usize {
        self.log.lock().unwrap().len()
    }

    pub fn count_path(&self, path: &str) -> usize {
        self.log
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.path == path)
            .count()
    }

    pub fn last(&self, path: &str) -> Option<Req> {
        self.log
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|r| r.path == path)
            .cloned()
    }
}

fn status_line(status: u16) -> &'static str {
    match status {
        200 => "200 OK",
        400 => "400 Bad Request",
        401 => "401 Unauthorized",
        403 => "403 Forbidden",
        409 => "409 Conflict",
        410 => "410 Gone",
        429 => "429 Too Many Requests",
        503 => "503 Service Unavailable",
        _ => "500 Internal Server Error",
    }
}

pub async fn spawn(id: &Ident, handler: Handler) -> MockSrv {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let cfg = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(id.cert_der.clone())],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(id.key.serialize_der())),
        )
        .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(cfg));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let srv = MockSrv {
        port,
        log: Arc::new(Mutex::new(Vec::new())),
        connections: Arc::new(AtomicUsize::new(0)),
    };
    let (log, connections) = (srv.log.clone(), srv.connections.clone());
    tokio::spawn(async move {
        loop {
            let Ok((tcp, _)) = listener.accept().await else {
                return;
            };
            let (acceptor, log, connections, handler) = (
                acceptor.clone(),
                log.clone(),
                connections.clone(),
                handler.clone(),
            );
            tokio::spawn(async move {
                let Ok(mut tls) = acceptor.accept(tcp).await else {
                    return;
                };
                connections.fetch_add(1, Ordering::SeqCst);
                let mut buf: Vec<u8> = Vec::new();
                loop {
                    let mut chunk = [0u8; 2048];
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
                    let body = String::from_utf8_lossy(&buf[head_end..head_end + clen]).to_string();
                    buf.drain(..head_end + clen);
                    let mut first = head.split_whitespace();
                    let req = Req {
                        method: first.next().unwrap_or("").to_string(),
                        path: first.next().unwrap_or("/").to_string(),
                        head: head.to_ascii_lowercase(),
                        body,
                    };
                    log.lock().unwrap().push(req.clone());
                    let r = handler(&req);
                    let resp = format!(
                        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        status_line(r.status),
                        r.body.len(),
                        r.body
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
