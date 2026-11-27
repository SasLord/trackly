//! HTTP-клиент Link: reqwest на том же пин-конфиге, что и WS (LNK-08).
//!
//! Не логируем и не кладём в Debug: Authorization, Cookie, X-Link-Secret, тела входа.

use std::sync::Arc;
use std::time::Duration;

use reqwest::cookie::{CookieStore, Jar};
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use reqwest::{Method, Response, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::Serialize;

use link_proto::dto::{HEADER_DEVICE, HEADER_SECRET};

use crate::pin::{build_client_config, classify_tls_error, PinError, PinProbe};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Учётные данные устройства (D-03): идентификатор и секрет. Debug секрет скрывает.
#[derive(Clone)]
pub struct DeviceCreds {
    pub device_id: String,
    pub secret: String,
}

impl std::fmt::Debug for DeviceCreds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceCreds")
            .field("device_id", &self.device_id)
            .field("secret", &"<скрыт>")
            .finish()
    }
}

/// Ошибки HTTP. «Нет связи» и «вход недействителен» различаются (D-25).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HttpError {
    #[error("отпечаток сервера не совпал с закреплённым")]
    PinMismatch { expected: String, got: String },
    #[error("нет связи с сервером")]
    Network,
    #[error("ошибка сервера: {0}")]
    Server(u16),
    #[error("сервер отклонил запрос: {status}")]
    Api {
        status: u16,
        code: Option<String>,
        message: Option<String>,
    },
    #[error("не удалось разобрать ответ сервера")]
    Decode,
}

impl From<PinError> for HttpError {
    fn from(e: PinError) -> Self {
        let PinError::Mismatch { expected, got } = e;
        HttpError::PinMismatch { expected, got }
    }
}

pub struct LinkHttp {
    client: reqwest::Client,
    jar: Arc<Jar>,
    base: Url,
    probe: PinProbe,
    tls: Arc<rustls::ClientConfig>,
    device: Option<DeviceCreds>,
}

impl std::fmt::Debug for LinkHttp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkHttp")
            .field("base", &self.base.as_str())
            .field("device", &self.device)
            .finish_non_exhaustive()
    }
}

impl LinkHttp {
    pub fn new(
        host: &str,
        port: u16,
        pin: [u8; 32],
        device: Option<DeviceCreds>,
    ) -> Result<Self, HttpError> {
        let host = if host.contains(':') && !host.starts_with('[') {
            format!("[{host}]") // IPv6-литерал
        } else {
            host.to_string()
        };
        let base =
            Url::parse(&format!("https://{host}:{port}/")).map_err(|_| HttpError::Network)?;
        let (config, probe) = build_client_config(pin);
        let tls = Arc::new(config);
        let jar = Arc::new(Jar::default());
        let client = reqwest::Client::builder()
            .tls_backend_preconfigured((*tls).clone())
            .http1_only()
            .cookie_provider(jar.clone())
            .redirect(Policy::none())
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .map_err(|_| HttpError::Network)?;
        Ok(Self {
            client,
            jar,
            base,
            probe,
            tls,
            device,
        })
    }

    /// Общий TLS-конфиг (с пином) для WS-клиента.
    pub fn tls_config(&self) -> Arc<rustls::ClientConfig> {
        self.tls.clone()
    }

    pub fn probe(&self) -> &PinProbe {
        &self.probe
    }

    pub fn base(&self) -> &Url {
        &self.base
    }

    /// Положить cookie сессии в jar (восстановление сессии и тесты WS без входа).
    #[doc(hidden)]
    pub fn seed_cookie(&self, cookie: &str) {
        self.jar.add_cookie_str(cookie, &self.base);
    }

    pub fn device(&self) -> Option<&DeviceCreds> {
        self.device.as_ref()
    }

    /// Значение заголовка Cookie для WS-апгрейда (сессия из jar).
    pub fn cookie_header(&self) -> Option<String> {
        self.jar
            .cookies(&self.base)
            .and_then(|v| v.to_str().ok().map(str::to_owned))
    }

    fn url(&self, path: &str) -> Result<Url, HttpError> {
        self.base
            .join(path.trim_start_matches('/'))
            .map_err(|_| HttpError::Network)
    }

    fn map_transport(&self, err: reqwest::Error) -> HttpError {
        if let Some(p) = classify_tls_error(&err, &self.probe) {
            return p.into();
        }
        HttpError::Network
    }

    /// Отправить запрос; ошибки только транспортные, статус не классифицируется.
    /// `extra` — доп. заголовки (например Authorization: Negotiate), значение чувствительное.
    pub async fn send_raw(
        &self,
        method: Method,
        path: &str,
        json: Option<serde_json::Value>,
        extra: &[(HeaderName, String)],
    ) -> Result<Response, HttpError> {
        let mut req = self.client.request(method, self.url(path)?);
        if let Some(d) = &self.device {
            if let Ok(v) = HeaderValue::from_str(&d.device_id) {
                req = req.header(HeaderName::from_static(HEADER_DEVICE), v);
            }
            if let Ok(mut v) = HeaderValue::from_str(&d.secret) {
                v.set_sensitive(true);
                req = req.header(HeaderName::from_static(HEADER_SECRET), v);
            }
        }
        for (name, value) in extra {
            let mut v = HeaderValue::from_str(value).map_err(|_| HttpError::Decode)?;
            v.set_sensitive(true);
            req = req.header(name.clone(), v);
        }
        if let Some(body) = json {
            req = req.json(&body);
        }
        self.probe.clear();
        req.send().await.map_err(|e| self.map_transport(e))
    }

    /// Классифицировать ответ: 2xx — как есть, 5xx — Server, прочее — Api с кодом из тела.
    pub async fn classify(resp: Response) -> Result<Response, HttpError> {
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        if status.is_server_error() {
            return Err(HttpError::Server(status.as_u16()));
        }
        Err(api_error(status, resp).await)
    }

    pub async fn request(
        &self,
        method: Method,
        path: &str,
        json: Option<serde_json::Value>,
        extra: &[(HeaderName, String)],
    ) -> Result<Response, HttpError> {
        Self::classify(self.send_raw(method, path, json, extra).await?).await
    }

    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, HttpError> {
        let resp = self.request(Method::GET, path, None, &[]).await?;
        resp.json().await.map_err(|_| HttpError::Decode)
    }

    pub async fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, HttpError> {
        let body = serde_json::to_value(body).map_err(|_| HttpError::Decode)?;
        let resp = self.request(Method::POST, path, Some(body), &[]).await?;
        resp.json().await.map_err(|_| HttpError::Decode)
    }

    /// GET с одним дополнительным заголовком (Authorization: Negotiate); значение не логируется.
    pub async fn get_with_header(
        &self,
        path: &str,
        name: HeaderName,
        value: String,
    ) -> Result<Response, HttpError> {
        self.request(Method::GET, path, None, &[(name, value)])
            .await
    }
}

async fn api_error(status: StatusCode, resp: Response) -> HttpError {
    let body: Option<serde_json::Value> = resp.json().await.ok();
    let field = |k: &str| {
        body.as_ref()
            .and_then(|b| b.get(k))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    };
    HttpError::Api {
        status: status.as_u16(),
        code: field("code"),
        message: field("message"),
    }
}
