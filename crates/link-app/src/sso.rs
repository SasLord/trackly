//! Автовход (SSO) за трейтом: чистый план попытки, классификация отказов (D-08, D-09, D-15).
//!
//! Реальный SPNEGO живёт только под `cfg(windows)` и проверяется живьём на Windows с AD
//! (план 20); мок эту проверку НЕ заменяет. Модуль не содержит и не должен содержать
//! пути, при котором неудача SSO автоматически вызывает парольный вход.

use std::fmt;
use std::net::IpAddr;

use base64::Engine as _;
use link_proto::dto::LINK_SSO_DENIED;

use crate::state::LoginReason;

/// Значение заголовка `Authorization: Negotiate <base64>`. `Debug` скрывает содержимое (T-44-26).
#[derive(Clone, PartialEq, Eq)]
pub struct NegotiateToken(String);

impl NegotiateToken {
    pub fn from_bytes(raw: &[u8]) -> Self {
        Self(base64::engine::general_purpose::STANDARD.encode(raw))
    }

    /// Готовое значение заголовка «Negotiate <base64>».
    pub fn header_value(&self) -> String {
        format!("Negotiate {}", self.0)
    }
}

impl fmt::Debug for NegotiateToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("NegotiateToken(<скрыт>)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SsoProviderError {
    #[error("провайдер SSO недоступен на этой платформе")]
    Unsupported,
    #[error("нет Kerberos-билета")]
    NoTicket,
    #[error("выбран NTLM вместо Kerberos")]
    NtlmFallback,
    #[error("ошибка провайдера SSO")]
    Other,
}

/// Источник Negotiate-токена для цели `spn`.
pub trait SsoProvider: Send + Sync {
    fn token(&self, spn: &str) -> Result<NegotiateToken, SsoProviderError>;
}

/// Провайдер для не-Windows (D-15): только пароль.
pub struct UnsupportedSso;

impl SsoProvider for UnsupportedSso {
    fn token(&self, _spn: &str) -> Result<NegotiateToken, SsoProviderError> {
        Err(SsoProviderError::Unsupported)
    }
}

/// Мок с заданным ответом. Не считается проверкой реального SPNEGO.
pub struct MockSso(pub Result<Vec<u8>, SsoProviderError>);

impl SsoProvider for MockSso {
    fn token(&self, _spn: &str) -> Result<NegotiateToken, SsoProviderError> {
        self.0
            .as_ref()
            .map(|b| NegotiateToken::from_bytes(b))
            .map_err(|e| *e)
    }
}

/// Windows-провайдер на SSPI через cross-krb5. Не компилировался локально (dev = macOS).
#[cfg(windows)]
pub struct WindowsSso;

#[cfg(windows)]
impl SsoProvider for WindowsSso {
    fn token(&self, spn: &str) -> Result<NegotiateToken, SsoProviderError> {
        use cross_krb5::{ClientCtx, InitiateFlags};
        // MUTUAL_AUTH не запрашиваем: сервер отвечает одним шагом.
        let (_pending, token) = ClientCtx::new(InitiateFlags::NEGOTIATE_TOKEN, None, spn, None)
            .map_err(|_| SsoProviderError::NoTicket)?;
        let raw: &[u8] = &token;
        if raw.starts_with(b"NTLMSSP") {
            return Err(SsoProviderError::NtlmFallback);
        }
        Ok(NegotiateToken::from_bytes(raw))
    }
}

/// Провайдер текущей платформы.
pub fn platform_provider() -> Box<dyn SsoProvider> {
    #[cfg(windows)]
    {
        Box::new(WindowsSso)
    }
    #[cfg(not(windows))]
    {
        Box::new(UnsupportedSso)
    }
}

/// План попытки автовхода.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsoPlan {
    Skip(LoginReason),
    Try { spn: String },
}

/// D-08: по IP SSO не пробуем; SPN = `HTTP/<host>` либо значение из конфига.
pub fn plan_sso(host: &str, spn_override: Option<&str>) -> SsoPlan {
    let bare = host.trim().trim_start_matches('[').trim_end_matches(']');
    if bare.parse::<IpAddr>().is_ok() {
        return SsoPlan::Skip(LoginReason::IpAddress);
    }
    match spn_override.map(str::trim).filter(|s| !s.is_empty()) {
        Some(spn) => SsoPlan::Try {
            spn: spn.to_string(),
        },
        None => SsoPlan::Try {
            spn: format!("HTTP/{bare}"),
        },
    }
}

/// Исход HTTP-обмена автовхода (подготовка; сам запрос делает authflow).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpOutcome {
    Status503,
    /// 401 с кодом ошибки из тела.
    Unauthorized {
        code: Option<String>,
    },
    /// Сетевая ошибка или таймаут.
    Network,
    /// Провайдер не выдал токен.
    Provider(SsoProviderError),
}

/// D-09: причина для экрана.
pub fn classify(outcome: &HttpOutcome) -> LoginReason {
    match outcome {
        HttpOutcome::Status503 => LoginReason::SsoDisabled,
        HttpOutcome::Unauthorized { code } => {
            if code.as_deref() == Some(LINK_SSO_DENIED) {
                LoginReason::NotMapped
            } else {
                LoginReason::Kerberos
            }
        }
        HttpOutcome::Network => LoginReason::NoAnswer,
        HttpOutcome::Provider(_) => LoginReason::Kerberos,
    }
}

/// Итог автовхода. Содержит только причину.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsoOutcome {
    Success,
    Failed(LoginReason),
}

/// UI-SPEC §6.5: можно ли предлагать «Повторить».
pub fn can_retry(reason: LoginReason) -> bool {
    !matches!(reason, LoginReason::IpAddress | LoginReason::SsoDisabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_hosts_skip() {
        assert_eq!(
            plan_sso("10.0.0.5", None),
            SsoPlan::Skip(LoginReason::IpAddress)
        );
        assert_eq!(
            plan_sso("[fe80::1]", None),
            SsoPlan::Skip(LoginReason::IpAddress)
        );
        assert_eq!(plan_sso("::1", None), SsoPlan::Skip(LoginReason::IpAddress));
    }

    #[test]
    fn dns_host_builds_spn_or_override() {
        assert_eq!(
            plan_sso("trackly.example.local", None),
            SsoPlan::Try {
                spn: "HTTP/trackly.example.local".into()
            }
        );
        assert_eq!(
            plan_sso("trackly.example.local", Some("HTTP/alias.example.local")),
            SsoPlan::Try {
                spn: "HTTP/alias.example.local".into()
            }
        );
    }

    #[test]
    fn classification() {
        assert_eq!(classify(&HttpOutcome::Status503), LoginReason::SsoDisabled);
        assert_eq!(
            classify(&HttpOutcome::Unauthorized {
                code: Some(LINK_SSO_DENIED.into())
            }),
            LoginReason::NotMapped
        );
        assert_eq!(
            classify(&HttpOutcome::Unauthorized { code: None }),
            LoginReason::Kerberos
        );
        assert_eq!(classify(&HttpOutcome::Network), LoginReason::NoAnswer);
        for e in [
            SsoProviderError::NoTicket,
            SsoProviderError::NtlmFallback,
            SsoProviderError::Other,
        ] {
            assert_eq!(classify(&HttpOutcome::Provider(e)), LoginReason::Kerberos);
        }
    }

    #[test]
    fn retry_rules() {
        assert!(!can_retry(LoginReason::IpAddress));
        assert!(!can_retry(LoginReason::SsoDisabled));
        assert!(can_retry(LoginReason::Kerberos));
        assert!(can_retry(LoginReason::NotMapped));
        assert!(can_retry(LoginReason::NoAnswer));
        // Исход несёт только причину, без секретов.
        assert_eq!(
            format!("{:?}", SsoOutcome::Failed(LoginReason::Kerberos)),
            "Failed(Kerberos)"
        );
    }

    #[test]
    fn mock_and_unsupported() {
        let ok = MockSso(Ok(vec![1, 2, 3]));
        assert!(ok.token("HTTP/x").is_ok());
        let bad = MockSso(Err(SsoProviderError::NoTicket));
        assert_eq!(bad.token("HTTP/x").unwrap_err(), SsoProviderError::NoTicket);
        #[cfg(not(windows))]
        assert_eq!(
            platform_provider().token("HTTP/x").unwrap_err(),
            SsoProviderError::Unsupported
        );
    }

    #[test]
    fn header_format_and_hidden_debug() {
        let t = NegotiateToken::from_bytes(b"secret-ticket");
        let h = t.header_value();
        assert!(h.starts_with("Negotiate "));
        assert_eq!(h, "Negotiate c2VjcmV0LXRpY2tldA==");
        assert!(!format!("{t:?}").contains("c2VjcmV0"));
    }
}
