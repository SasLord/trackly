//! Shape validation for a stored number value (BE-WR-10): act number,
//! device/printer inventory number. Cartridge/drum codes keep their own,
//! stricter rule in `cartridge_service` (≤ 32 chars, no chars < U+0020).
//!
//! Numbers are printed into act forms and CSV reports and compared by the
//! occupied check, so they are bounded in length and must not contain
//! control characters (`\n`, `\t`, … inside the value — `.trim()` only
//! removes them at the edges).

use crate::error::AppError;

/// Maximum length of a number value, in characters (not bytes).
pub const MAX_NUMBER_CHARS: usize = 64;

/// Validate an already-`.trim()`med, non-empty number value. `field` is the
/// `AppError::Validation` field name the caller's form binds to.
pub fn validate_number_value(field: &str, trimmed: &str) -> Result<(), AppError> {
    if trimmed.chars().count() > MAX_NUMBER_CHARS {
        return Err(AppError::Validation {
            field: field.to_string(),
            message: format!("Номер не должен быть длиннее {MAX_NUMBER_CHARS} символов."),
        });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(AppError::Validation {
            field: field.to_string(),
            message: "Номер не должен содержать управляющих символов (перевод строки, табуляция)."
                .to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ordinary_numbers() {
        assert!(validate_number_value("number", "ОРГ-00-000001").is_ok());
        assert!(validate_number_value("number", "42в").is_ok());
        assert!(validate_number_value("number", &"Я".repeat(MAX_NUMBER_CHARS)).is_ok());
    }

    #[test]
    fn rejects_too_long_and_control_chars() {
        assert!(validate_number_value("number", &"1".repeat(MAX_NUMBER_CHARS + 1)).is_err());
        assert!(validate_number_value("number", "42\n43").is_err());
        assert!(validate_number_value("number", "ИНВ\t1").is_err());
    }
}
