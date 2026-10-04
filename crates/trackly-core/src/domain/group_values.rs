//! Чистая нормализация значений свойств групп (GRP-03).
//!
//! Источник истины — сервер: клиентская проверка только подсказка. Все функции здесь
//! без I/O и без внешних зависимостей; оба транспорта идут через один и тот же код.

use std::net::IpAddr;
use std::str::FromStr;

use crate::domain::groups::{PropertyDataType, NUMBER_MAX_CHARS, TEXT_MAX_CHARS};
use crate::error::AppError;

/// Ошибка нормализации одного скалярного значения.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueError {
    Ip,
    Mac,
    Number,
    Text,
}

impl ValueError {
    /// Русский текст ошибки (UI-SPEC §17.3).
    pub fn message(&self) -> String {
        match self {
            Self::Ip => "Укажите IPv4- или IPv6-адрес, например 192.168.1.10.".to_string(),
            Self::Mac => "Укажите MAC-адрес, например 00:1b:44:11:3a:b7. \
                          Разделитель — «:», «-» или без него."
                .to_string(),
            Self::Number => "Укажите число.".to_string(),
            Self::Text => {
                format!("Значение слишком длинное: не более {TEXT_MAX_CHARS} символов.")
            }
        }
    }
}

impl std::fmt::Display for ValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

/// IPv4/IPv6 → каноническая строка (`IpAddr::to_string`).
pub fn normalize_ip(raw: &str) -> Result<String, ValueError> {
    IpAddr::from_str(raw.trim())
        .map(|ip| ip.to_string())
        .map_err(|_| ValueError::Ip)
}

/// MAC → `aa:bb:cc:dd:ee:ff`. Допустимы разделители «:», «-» (один на всю строку)
/// или запись из 12 hex-цифр без разделителей.
pub fn normalize_mac(raw: &str) -> Result<String, ValueError> {
    let s = raw.trim();
    if !s.is_ascii() {
        return Err(ValueError::Mac);
    }
    let has_colon = s.contains(':');
    let has_dash = s.contains('-');
    let digits: String = match (has_colon, has_dash) {
        (true, true) => return Err(ValueError::Mac),
        (false, false) => s.to_string(),
        (true, false) | (false, true) => {
            let sep = if has_colon { ':' } else { '-' };
            let groups: Vec<&str> = s.split(sep).collect();
            if groups.len() != 6 || groups.iter().any(|g| g.len() != 2) {
                return Err(ValueError::Mac);
            }
            groups.concat()
        }
    };
    if digits.len() != 12 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ValueError::Mac);
    }
    let lower = digits.to_ascii_lowercase();
    let parts: Vec<&str> = (0..6).map(|i| &lower[i * 2..i * 2 + 2]).collect();
    Ok(parts.join(":"))
}

/// Число: запятая → точка, ведущий «+» убирается, NaN/inf и мусор отвергаются.
pub fn normalize_number(raw: &str) -> Result<String, ValueError> {
    let s = raw.trim().replace(',', ".");
    if s.is_empty() || s.chars().count() > NUMBER_MAX_CHARS {
        return Err(ValueError::Number);
    }
    match f64::from_str(&s) {
        Ok(v) if v.is_finite() => {}
        _ => return Err(ValueError::Number),
    }
    Ok(s.strip_prefix('+').unwrap_or(&s).to_string())
}

/// Текст: trim; пустое → `None` («нет значения»); длиннее лимита → ошибка.
pub fn normalize_text(raw: &str) -> Result<Option<String>, ValueError> {
    let s = raw.trim();
    if s.is_empty() {
        return Ok(None);
    }
    if s.chars().count() > TEXT_MAX_CHARS {
        return Err(ValueError::Text);
    }
    Ok(Some(s.to_string()))
}

/// Диспетчер по типу свойства. `Ok(None)` — «нет значения» (пустая строка).
/// Для `Users`/`DeviceRefs` — ошибка: связи нормализует сервис, а не эта функция.
pub fn normalize_scalar(
    data_type: PropertyDataType,
    raw: &str,
) -> Result<Option<String>, AppError> {
    let wrap = |e: ValueError| AppError::Validation {
        field: "value".to_string(),
        message: e.message(),
    };
    if raw.trim().is_empty() {
        return Ok(None);
    }
    match data_type {
        PropertyDataType::Text => normalize_text(raw).map_err(wrap),
        PropertyDataType::Number => normalize_number(raw).map(Some).map_err(wrap),
        PropertyDataType::Ip => normalize_ip(raw).map(Some).map_err(wrap),
        PropertyDataType::Mac => normalize_mac(raw).map(Some).map_err(wrap),
        PropertyDataType::Users | PropertyDataType::DeviceRefs => Err(AppError::Validation {
            field: "value".to_string(),
            message: "Для этого типа свойства значение задаётся связями, а не строкой.".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_valid_cases() {
        let cases = [
            ("192.168.1.10", "192.168.1.10"),
            (" 10.0.0.1 ", "10.0.0.1"),
            (" 2001:DB8:0:0:0:0:0:1 ", "2001:db8::1"),
            ("::1", "::1"),
            ("255.255.255.255", "255.255.255.255"),
            ("0.0.0.0", "0.0.0.0"),
        ];
        for (raw, want) in cases {
            assert_eq!(normalize_ip(raw).as_deref(), Ok(want), "ip {raw:?}");
        }
    }

    #[test]
    fn ip_invalid_cases() {
        for raw in [
            "01.2.3.4",
            "1.2.3",
            "fe80::1%eth0",
            "10.0.0.1/24",
            "192.168.0.256",
            "",
            "abc",
            "1.2.3.4.5",
        ] {
            assert_eq!(normalize_ip(raw), Err(ValueError::Ip), "ip {raw:?}");
        }
    }

    #[test]
    fn mac_valid_cases_all_separators_same_result() {
        for raw in [
            "AA-BB-CC-DD-EE-FF",
            "aa:bb:cc:dd:ee:ff",
            "aabbccddeeff",
            " AABBCCDDEEFF ",
            "Aa:Bb:Cc:Dd:Ee:Ff",
        ] {
            assert_eq!(
                normalize_mac(raw).as_deref(),
                Ok("aa:bb:cc:dd:ee:ff"),
                "mac {raw:?}"
            );
        }
        assert_eq!(
            normalize_mac("00-1B-44-11-3A-B7").as_deref(),
            Ok("00:1b:44:11:3a:b7")
        );
    }

    #[test]
    fn mac_invalid_cases() {
        for raw in [
            "aa:bb-cc:dd:ee:ff",
            "aabbccddeef",
            "zz:bb:cc:dd:ee:ff",
            "a:bb:cc:dd:ee:ff",
            "aaa:bb:cc:dd:ee:ff",
            "aa:bb:cc:dd:ee",
            "aa:bb:cc:dd:ee:ff:00",
            "",
            "aabbccddeeffaa",
            "аabbccddeeff",
        ] {
            assert_eq!(normalize_mac(raw), Err(ValueError::Mac), "mac {raw:?}");
        }
    }

    #[test]
    fn number_valid_cases() {
        for (raw, want) in [
            ("1,5", "1.5"),
            ("+7", "7"),
            ("-3.25", "-3.25"),
            (" 42 ", "42"),
            ("0", "0"),
        ] {
            assert_eq!(normalize_number(raw).as_deref(), Ok(want), "num {raw:?}");
        }
    }

    #[test]
    fn number_invalid_cases() {
        let long = "1".repeat(33);
        for raw in ["abc", "NaN", "inf", "-inf", "", "1.2.3", long.as_str()] {
            assert_eq!(
                normalize_number(raw),
                Err(ValueError::Number),
                "num {raw:?}"
            );
        }
    }

    #[test]
    fn text_trim_empty_and_limit() {
        assert_eq!(normalize_text("  привет  "), Ok(Some("привет".to_string())));
        assert_eq!(normalize_text("   "), Ok(None));
        assert_eq!(normalize_text(""), Ok(None));
        assert!(normalize_text(&"я".repeat(TEXT_MAX_CHARS)).is_ok());
        assert_eq!(
            normalize_text(&"я".repeat(TEXT_MAX_CHARS + 1)),
            Err(ValueError::Text)
        );
    }

    #[test]
    fn scalar_dispatches_by_type() {
        assert_eq!(
            normalize_scalar(PropertyDataType::Ip, " 10.0.0.1 ").unwrap(),
            Some("10.0.0.1".to_string())
        );
        assert_eq!(
            normalize_scalar(PropertyDataType::Mac, "AABBCCDDEEFF").unwrap(),
            Some("aa:bb:cc:dd:ee:ff".to_string())
        );
        assert_eq!(
            normalize_scalar(PropertyDataType::Number, "2,5").unwrap(),
            Some("2.5".to_string())
        );
        assert_eq!(
            normalize_scalar(PropertyDataType::Text, "  ").unwrap(),
            None
        );
        assert!(matches!(
            normalize_scalar(PropertyDataType::Ip, "nope"),
            Err(AppError::Validation { ref field, .. }) if field == "value"
        ));
        assert!(matches!(
            normalize_scalar(PropertyDataType::Users, "1"),
            Err(AppError::Validation { .. })
        ));
        assert!(matches!(
            normalize_scalar(PropertyDataType::DeviceRefs, "1"),
            Err(AppError::Validation { .. })
        ));
    }

    #[test]
    fn value_error_messages_are_russian() {
        assert!(ValueError::Ip.message().starts_with("Укажите IPv4"));
        assert!(ValueError::Mac.message().starts_with("Укажите MAC"));
        assert_eq!(ValueError::Number.message(), "Укажите число.");
    }
}
