//! Код-приглашение `TL1.…` (D-03). Несёт только настройку сервера и одноразовый
//! секрет регистрации устройства; сотрудника и учётную запись — не несёт (D-04).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};

const PREFIX: &str = "TL1.";
const VERSION: u8 = 1;
const CHECKSUM_LEN: usize = 4;
/// version + port + exp + spki + secret + host_len
const FIXED_LEN: usize = 1 + 2 + 8 + 32 + 16 + 1;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InviteError {
    #[error("код не распознан")]
    Format,
    #[error("контрольная сумма кода не сходится")]
    Checksum,
    #[error("неподдерживаемая версия кода")]
    Version,
    #[error("некорректный адрес сервера в коде")]
    Host,
}

#[derive(Clone, PartialEq, Eq)]
pub struct InviteCode {
    pub host: String,
    pub port: u16,
    pub spki: [u8; 32],
    /// 128-битный одноразовый секрет. Не выводится в Debug.
    pub secret: [u8; 16],
    pub exp_unix: i64,
}

// Ручной Debug: секрет не должен попадать в логи (T-44-05).
impl std::fmt::Debug for InviteCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InviteCode")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("exp_unix", &self.exp_unix)
            .field("secret", &"<скрыт>")
            .finish_non_exhaustive()
    }
}

fn host_ok(h: &str) -> bool {
    !h.is_empty() && h.len() <= 253 && !h.chars().any(|c| c.is_whitespace() || c.is_control())
}

fn checksum(body: &[u8]) -> [u8; CHECKSUM_LEN] {
    let d = Sha256::digest(body);
    [d[0], d[1], d[2], d[3]]
}

impl InviteCode {
    pub fn encode(&self) -> Result<String, InviteError> {
        if !host_ok(&self.host) {
            return Err(InviteError::Host);
        }
        let mut body = Vec::with_capacity(FIXED_LEN + self.host.len() + CHECKSUM_LEN);
        body.push(VERSION);
        body.extend_from_slice(&self.port.to_be_bytes());
        body.extend_from_slice(&self.exp_unix.to_be_bytes());
        body.extend_from_slice(&self.spki);
        body.extend_from_slice(&self.secret);
        body.push(self.host.len() as u8);
        body.extend_from_slice(self.host.as_bytes());
        let sum = checksum(&body);
        body.extend_from_slice(&sum);
        Ok(format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(body)))
    }

    /// Срок не проверяется — см. [`InviteCode::is_expired`].
    pub fn decode(input: &str) -> Result<Self, InviteError> {
        let clean: String = input.chars().filter(|c| !c.is_whitespace()).collect();
        let rest = clean.strip_prefix(PREFIX).ok_or(InviteError::Format)?;
        let raw = URL_SAFE_NO_PAD
            .decode(rest)
            .map_err(|_| InviteError::Format)?;
        if raw.len() < FIXED_LEN + CHECKSUM_LEN {
            return Err(InviteError::Format);
        }
        let (body, sum) = raw.split_at(raw.len() - CHECKSUM_LEN);
        if checksum(body) != sum {
            return Err(InviteError::Checksum);
        }
        if body[0] != VERSION {
            return Err(InviteError::Version);
        }
        let port = u16::from_be_bytes([body[1], body[2]]);
        let exp_unix = i64::from_be_bytes(body[3..11].try_into().map_err(|_| InviteError::Format)?);
        let mut spki = [0u8; 32];
        spki.copy_from_slice(&body[11..43]);
        let mut secret = [0u8; 16];
        secret.copy_from_slice(&body[43..59]);
        let host_len = body[59] as usize;
        if body.len() != FIXED_LEN + host_len {
            return Err(InviteError::Format);
        }
        let host = std::str::from_utf8(&body[60..])
            .map_err(|_| InviteError::Host)?
            .to_string();
        if !host_ok(&host) {
            return Err(InviteError::Host);
        }
        Ok(Self {
            host,
            port,
            spki,
            secret,
            exp_unix,
        })
    }

    /// Граница включена: `now == exp` — уже истёк.
    pub fn is_expired(&self, now_unix: i64) -> bool {
        now_unix >= self.exp_unix
    }
}

/// То, что сервер хранит как `code_hash` вместо самого секрета.
pub fn secret_hash(secret: &[u8; 16]) -> [u8; 32] {
    Sha256::digest(secret).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> InviteCode {
        InviteCode {
            host: "trackly.example.local".into(),
            port: 8443,
            spki: [7u8; 32],
            secret: [9u8; 16],
            exp_unix: 1_800_000_000,
        }
    }

    #[test]
    fn roundtrip() {
        let c = sample();
        let s = c.encode().unwrap();
        assert!(s.starts_with("TL1."));
        assert_eq!(InviteCode::decode(&s).unwrap(), c);
    }

    #[test]
    fn tampering_breaks_checksum() {
        let s = sample().encode().unwrap();
        let mut ch: Vec<char> = s.chars().collect();
        let i = 10;
        ch[i] = if ch[i] == 'A' { 'B' } else { 'A' };
        let bad: String = ch.into_iter().collect();
        assert_eq!(InviteCode::decode(&bad), Err(InviteError::Checksum));
    }

    #[test]
    fn whitespace_ignored_and_prefix_checked() {
        let s = sample().encode().unwrap();
        let spaced = format!("  {}\n{}\r\n", &s[..20], &s[20..]);
        assert_eq!(InviteCode::decode(&spaced).unwrap(), sample());
        assert_eq!(
            InviteCode::decode(&s.replace("TL1.", "TL2.")),
            Err(InviteError::Format)
        );
    }

    #[test]
    fn expiry_boundary() {
        let c = sample();
        assert!(!c.is_expired(c.exp_unix - 1));
        assert!(c.is_expired(c.exp_unix));
        assert!(c.is_expired(c.exp_unix + 1));
        // decode сам срок не проверяет
        let old = InviteCode {
            exp_unix: 1,
            ..sample()
        };
        assert!(InviteCode::decode(&old.encode().unwrap()).is_ok());
    }

    #[test]
    fn bad_host_rejected() {
        for h in ["", "a b", &"a".repeat(254)] {
            let c = InviteCode {
                host: h.into(),
                ..sample()
            };
            assert_eq!(c.encode(), Err(InviteError::Host));
        }
    }

    #[test]
    fn secret_hash_deterministic_and_debug_hides_secret() {
        let c = sample();
        assert_eq!(secret_hash(&c.secret), secret_hash(&c.secret));
        assert_ne!(secret_hash(&c.secret), secret_hash(&[1u8; 16]));
        let d = format!("{c:?}");
        assert!(!d.contains("[9, 9"));
        assert!(d.contains("скрыт"));
    }
}
