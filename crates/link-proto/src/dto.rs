//! JSON-контракт enrollment и входа Link. Читают и сервер, и клиент.
//! Поля — camelCase, как остальной JSON проекта. Сотрудника и учётной записи
//! в enrollment нет (D-04).

use serde::{Deserialize, Serialize};

pub const HEADER_DEVICE: &str = "x-link-device";
pub const HEADER_SECRET: &str = "x-link-secret";

/// Код WS Close-кадра, которым сервер рвёт соединение отозванного устройства/входа
/// (D-23, пишется в ws_auth_middleware плана 14; читает ws_client Link).
pub const REVOKED_CLOSE_CODE: u16 = 4401;

pub const LINK_REVOKED: &str = "LINK_REVOKED";
pub const LINK_DEVICE_BANNED: &str = "LINK_DEVICE_BANNED";
pub const LINK_INVITE_INVALID: &str = "LINK_INVITE_INVALID";
pub const LINK_INVITE_EXPIRED: &str = "LINK_INVITE_EXPIRED";
pub const LINK_INVITE_USED: &str = "LINK_INVITE_USED";
pub const LINK_SSO_DISABLED: &str = "LINK_SSO_DISABLED";
pub const LINK_SSO_DENIED: &str = "LINK_SSO_DENIED";
pub const LINK_BAD_CREDENTIALS: &str = "LINK_BAD_CREDENTIALS";
pub const LINK_USER_BLOCKED: &str = "LINK_USER_BLOCKED";
pub const LINK_RATE_LIMITED: &str = "LINK_RATE_LIMITED";

/// Тело ошибки Link: `{ code, message }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollRequest {
    /// Секрет кода-приглашения, base64url.
    pub secret: String,
    pub device_id: String,
    pub hostname: String,
    pub os: String,
    pub arch: String,
    pub client_version: String,
}

impl std::fmt::Debug for EnrollRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnrollRequest")
            .field("secret", &"<скрыт>")
            .field("device_id", &self.device_id)
            .field("hostname", &self.hostname)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollResponse {
    pub device_secret: String,
}

impl std::fmt::Debug for EnrollResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnrollResponse")
            .field("device_secret", &"<скрыт>")
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkLoginRequest {
    pub login: String,
    pub password: String,
}

impl std::fmt::Debug for LinkLoginRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkLoginRequest")
            .field("login", &self.login)
            .field("password", &"<скрыт>")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkUser {
    pub id: i64,
    pub login: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkMethod {
    Sso,
    Password,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkAuthResponse {
    pub user: LinkUser,
    pub method: LinkMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_token: Option<String>,
}

impl std::fmt::Debug for LinkAuthResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkAuthResponse")
            .field("user", &self.user)
            .field("method", &self.method)
            .field(
                "device_token",
                &self.device_token.as_ref().map(|_| "<скрыт>"),
            )
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeRequest {
    pub device_token: String,
}

impl std::fmt::Debug for ResumeRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResumeRequest")
            .field("device_token", &"<скрыт>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_response_roundtrip_camel_case() {
        let r = LinkAuthResponse {
            user: LinkUser {
                id: 1,
                login: "ivanov".into(),
                display_name: "Иванов И.И.".into(),
            },
            method: LinkMethod::Sso,
            device_token: Some("t".into()),
        };
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains("\"displayName\""));
        assert!(j.contains("\"method\":\"sso\""));
        assert!(j.contains("\"deviceToken\""));
        let back: LinkAuthResponse = serde_json::from_str(&j).unwrap();
        assert_eq!(back.user, r.user);
        assert_eq!(back.method, LinkMethod::Sso);
        let none: LinkAuthResponse = serde_json::from_str(
            r#"{"user":{"id":2,"login":"p","displayName":"Петров П.П."},"method":"password"}"#,
        )
        .unwrap();
        assert!(none.device_token.is_none());
    }

    #[test]
    fn enroll_roundtrip_has_no_user_fields() {
        let r = EnrollRequest {
            secret: "s".into(),
            device_id: "d".into(),
            hostname: "h".into(),
            os: "windows".into(),
            arch: "x86_64".into(),
            client_version: "0.1.0".into(),
        };
        let j = serde_json::to_value(&r).unwrap();
        let keys: Vec<_> = j.as_object().unwrap().keys().cloned().collect();
        assert!(!keys.iter().any(|k| k.contains("user") || k == "login"));
        assert!(j.get("deviceId").is_some() && j.get("clientVersion").is_some());
        assert!(!format!("{r:?}").contains("\"s\""));
    }

    #[test]
    fn constants_are_stable() {
        assert_eq!(HEADER_DEVICE, "x-link-device");
        assert_eq!(HEADER_SECRET, "x-link-secret");
        assert_eq!(LINK_REVOKED, "LINK_REVOKED");
        assert_eq!(LINK_RATE_LIMITED, "LINK_RATE_LIMITED");
        assert_eq!(LINK_INVITE_EXPIRED, "LINK_INVITE_EXPIRED");
    }
}
