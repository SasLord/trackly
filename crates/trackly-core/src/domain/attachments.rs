//! Вложения: типы родителя, коды отказов, лимиты и чистые помощники.
//!
//! Без I/O. Валидатор хеша — единственный способ собрать сегмент пути на диске.

/// Лимит размера по умолчанию (D-07).
pub const DEFAULT_MAX_BYTES: u64 = 20 * 1024 * 1024;
/// Верхняя граница настраиваемого лимита (D-07).
pub const MAX_BYTES_CEILING: u64 = 100 * 1024 * 1024;
/// Ключ настройки лимита размера.
pub const SETTING_MAX_BYTES: &str = "attachment_max_bytes";
/// Ключ настройки отчёта целостности.
pub const SETTING_INTEGRITY_REPORT: &str = "attachments_integrity_report";

/// К чему привязано вложение. Принтеры — это устройства (D-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentKind {
    Request,
    Device,
    Cartridge,
}

impl ParentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ParentKind::Request => "request",
            ParentKind::Device => "device",
            ParentKind::Cartridge => "cartridge",
        }
    }

    /// `None` для неизвестного значения (в том числе `message` — это 41.5).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "request" => Some(ParentKind::Request),
            "device" => Some(ParentKind::Device),
            "cartridge" => Some(ParentKind::Cartridge),
            _ => None,
        }
    }
}

/// Машинные коды отказа; текст по коду выбирает UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectCode {
    TooLarge,
    Empty,
    TypeNotAllowed,
    HeicFailed,
    Forbidden,
}

impl RejectCode {
    pub fn as_code(self) -> &'static str {
        match self {
            RejectCode::TooLarge => "too_large",
            RejectCode::Empty => "empty",
            RejectCode::TypeNotAllowed => "type_not_allowed",
            RejectCode::HeicFailed => "heic_failed",
            RejectCode::Forbidden => "forbidden",
        }
    }
}

/// `^[0-9a-f]{64}$`.
pub fn is_valid_hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Безопасное отображаемое имя: базовое имя без разделителей путей,
/// без управляющих символов, CR/LF и кавычек; не длиннее 255 символов.
pub fn sanitize_file_name(raw: &str) -> String {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or("");
    let cleaned: String = base
        .chars()
        .filter(|c| !c.is_control() && *c != '"' && *c != '\'')
        .collect();
    let cleaned: String = cleaned.chars().take(255).collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "файл".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_kind_roundtrip() {
        for k in [
            ParentKind::Request,
            ParentKind::Device,
            ParentKind::Cartridge,
        ] {
            assert_eq!(ParentKind::parse(k.as_str()), Some(k));
        }
        assert_eq!(ParentKind::parse("message"), None);
        assert_eq!(ParentKind::parse(""), None);
    }

    #[test]
    fn attachments_hash_validation() {
        assert!(is_valid_hash(&"a1".repeat(32)));
        assert!(!is_valid_hash(&"A1".repeat(32)));
        assert!(!is_valid_hash(&"a".repeat(63)));
        assert!(!is_valid_hash(&"a".repeat(65)));
        assert!(!is_valid_hash("../x"));
        assert!(!is_valid_hash(""));
    }

    #[test]
    fn attachments_sanitize_name() {
        assert_eq!(
            sanitize_file_name("../../etc/скан-акта.pdf"),
            "скан-акта.pdf"
        );
        assert_eq!(sanitize_file_name("C:\\dir\\a.pdf"), "a.pdf");
        assert_eq!(sanitize_file_name("a\r\nb\"c.pdf"), "abc.pdf");
        assert_eq!(sanitize_file_name(""), "файл");
        assert_eq!(sanitize_file_name("dir/"), "файл");
        assert_eq!(sanitize_file_name(&"я".repeat(400)).chars().count(), 255);
    }

    #[test]
    fn attachments_reject_codes_and_limits() {
        assert_eq!(RejectCode::TooLarge.as_code(), "too_large");
        assert_eq!(RejectCode::Forbidden.as_code(), "forbidden");
        const { assert!(DEFAULT_MAX_BYTES < MAX_BYTES_CEILING) };
    }
}
