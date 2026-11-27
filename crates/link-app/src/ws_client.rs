//! WS-клиент Link: тот же пин-конфиг, что и у HTTP (LNK-08), служебные кадры и сторож (D-23).
//!
//! TLS поднимается вручную поверх tokio-rustls (ring): у tokio-tungstenite в этом крейте
//! нет TLS-фич, чтобы в граф не попал aws-lc-rs. Конфиг берётся у `LinkHttp::tls_config`.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::time::Instant;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Error as TgError;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use link_proto::dto::{HEADER_DEVICE, REVOKED_CLOSE_CODE};

use crate::http_client::LinkHttp;
use crate::pin::classify_tls_error;

/// Ожидание любого кадра после пробного ping (как PROBE_TIMEOUT_MS в packages/transport).
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
/// Пока hello не пришёл: серверный IDLE_TIMEOUT по умолчанию.
const DEFAULT_IDLE: Duration = Duration::from_secs(60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq)]
pub enum WsFrame {
    Hello {
        idle_timeout_secs: u64,
    },
    ResyncRequired,
    Event(serde_json::Value),
    /// Сервер закрыл соединение; `revoked` — код REVOKED_CLOSE_CODE (вход/устройство отозваны).
    Closed {
        revoked: bool,
    },
    /// Сторож: тишина и нет ответа на пробу, либо запись не прошла.
    Dead,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WsError {
    #[error("отпечаток сервера не совпал с закреплённым")]
    PinMismatch { expected: String, got: String },
    /// 401 на апгрейде: нужна повторная аутентификация, а не «нет связи» (Pitfall 8).
    #[error("сервер не принял сессию")]
    Unauthorized,
    #[error("нет связи с сервером")]
    Network,
}

pub type TlsWsConn = WsConn<TlsStream<TcpStream>>;

pub struct WsConn<S> {
    ws: WebSocketStream<S>,
    idle: Duration,
    deadline: Instant,
    probe_deadline: Option<Instant>,
}

impl<S: AsyncRead + AsyncWrite + Unpin> WsConn<S> {
    pub fn new(ws: WebSocketStream<S>) -> Self {
        Self {
            ws,
            idle: DEFAULT_IDLE,
            deadline: Instant::now() + DEFAULT_IDLE,
            probe_deadline: None,
        }
    }

    /// Послать пробный ping (`{"type":"ping"}`); сервер отвечает `pong`.
    pub async fn send_ping(&mut self) -> Result<(), WsError> {
        self.ws
            .send(Message::text(r#"{"type":"ping"}"#))
            .await
            .map_err(|_| WsError::Network)
    }

    /// Следующий кадр. Внутри сторож: за `idle` тишины после hello шлёт ping и ждёт любой
    /// кадр `PROBE_TIMEOUT`; не дождался — `Dead`. Отменяема на `.await`.
    pub async fn next_frame(&mut self) -> WsFrame {
        loop {
            let wake = self.probe_deadline.unwrap_or(self.deadline);
            tokio::select! {
                msg = self.ws.next() => {
                    let Some(Ok(msg)) = msg else {
                        return WsFrame::Closed { revoked: false };
                    };
                    self.deadline = Instant::now() + self.idle;
                    self.probe_deadline = None;
                    match msg {
                        Message::Text(t) => {
                            if let Some(frame) = self.parse_text(t.as_str()) {
                                return frame;
                            }
                        }
                        Message::Close(frame) => {
                            let revoked = frame
                                .is_some_and(|f| u16::from(f.code) == REVOKED_CLOSE_CODE);
                            return WsFrame::Closed { revoked };
                        }
                        // Ping/Pong/Binary: живость подтверждена; pong на ping отправит сам tungstenite.
                        _ => {}
                    }
                }
                () = tokio::time::sleep_until(wake) => {
                    if self.probe_deadline.is_some() {
                        return WsFrame::Dead;
                    }
                    let sent = tokio::time::timeout(PROBE_TIMEOUT, self.send_ping()).await;
                    if !matches!(sent, Ok(Ok(()))) {
                        return WsFrame::Dead;
                    }
                    self.probe_deadline = Some(Instant::now() + PROBE_TIMEOUT);
                }
            }
        }
    }

    /// `None` — служебный кадр, наружу не отдаётся (pong, мусор).
    fn parse_text(&mut self, text: &str) -> Option<WsFrame> {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        match v.get("type").and_then(|t| t.as_str()) {
            Some("hello") => {
                let secs = v
                    .get("idleTimeoutSecs")
                    .and_then(serde_json::Value::as_u64)
                    .filter(|s| *s > 0)
                    .unwrap_or(DEFAULT_IDLE.as_secs());
                self.idle = Duration::from_secs(secs);
                self.deadline = Instant::now() + self.idle;
                Some(WsFrame::Hello {
                    idle_timeout_secs: secs,
                })
            }
            Some("pong") => None,
            Some("resync_required") => Some(WsFrame::ResyncRequired),
            _ => Some(WsFrame::Event(v)),
        }
    }
}

/// Подключиться к `wss://host:port{path}` с пином, Cookie из jar и заголовком устройства.
/// Токены и куки не логируются.
pub async fn connect(http: &LinkHttp, path: &str) -> Result<TlsWsConn, WsError> {
    tokio::time::timeout(CONNECT_TIMEOUT, connect_inner(http, path))
        .await
        .map_err(|_| WsError::Network)?
}

async fn connect_inner(http: &LinkHttp, path: &str) -> Result<TlsWsConn, WsError> {
    let base = http.base();
    let host = base.host_str().ok_or(WsError::Network)?;
    let port = base.port_or_known_default().ok_or(WsError::Network)?;
    let bare_host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string();

    let mut req = format!("wss://{host}:{port}{path}")
        .into_client_request()
        .map_err(|_| WsError::Network)?;
    if let Some(cookie) = http.cookie_header() {
        if let Ok(mut v) = HeaderValue::from_str(&cookie) {
            v.set_sensitive(true);
            req.headers_mut().insert("cookie", v);
        }
    }
    if let Some(d) = http.device() {
        if let Ok(v) = HeaderValue::from_str(&d.device_id) {
            req.headers_mut().insert(HEADER_DEVICE, v);
        }
    }

    http.probe().clear();
    let tcp = TcpStream::connect((bare_host.as_str(), port))
        .await
        .map_err(|_| WsError::Network)?;
    let name = rustls::pki_types::ServerName::try_from(bare_host).map_err(|_| WsError::Network)?;
    let tls = TlsConnector::from(http.tls_config())
        .connect(name, tcp)
        .await
        .map_err(|e| match classify_tls_error(&e, http.probe()) {
            Some(crate::pin::PinError::Mismatch { expected, got }) => {
                WsError::PinMismatch { expected, got }
            }
            None => WsError::Network,
        })?;

    match tokio_tungstenite::client_async(req, tls).await {
        Ok((ws, _resp)) => Ok(WsConn::new(ws)),
        Err(TgError::Http(resp)) if resp.status().as_u16() == 401 => Err(WsError::Unauthorized),
        Err(_) => Err(WsError::Network),
    }
}
