//! Переписка по заявкам (Phase 41.5): лимиты, коды отказов и чистые помощники.
//!
//! Без I/O. Проверки длины и статуса выполняет сервер (D-08, D-07).

use crate::domain::repairs::OPEN_REPAIR_STATUSES;

/// Максимальная длина текста сообщения в символах (D-08).
pub const MAX_TEXT_CHARS: usize = 4000;

/// Машинные коды отказа при отправке сообщения; текст по коду выбирает UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRejectCode {
    TooLong,
    Closed,
    Forbidden,
    Empty,
    NoChat,
}

impl ChatRejectCode {
    pub fn as_code(self) -> &'static str {
        match self {
            ChatRejectCode::TooLong => "too_long",
            ChatRejectCode::Closed => "closed",
            ChatRejectCode::Forbidden => "forbidden",
            ChatRejectCode::Empty => "empty",
            ChatRejectCode::NoChat => "no_chat",
        }
    }
}

/// Писать можно только пока заявка открыта (D-07).
pub fn can_post_status(status: &str) -> bool {
    OPEN_REPAIR_STATUSES.contains(&status)
}

/// Чат есть у всех типов заявок, кроме регистрации в AD (D-05).
pub fn request_type_has_chat(request_type: &str) -> bool {
    matches!(request_type, "repair" | "cartridge_replace" | "free_form")
}

/// Текст без ведущих и замыкающих пробелов.
pub fn normalize_text(raw: &str) -> String {
    raw.trim().to_string()
}

/// Длина в символах, не в байтах.
pub fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Превышает ли текст лимит `MAX_TEXT_CHARS`.
pub fn exceeds_limit(s: &str) -> bool {
    char_len(s) > MAX_TEXT_CHARS
}

/// Подпись роли автора сообщения; `None`, если роль не определена.
pub fn author_role_label(
    role: &str,
    is_requester: bool,
    is_participant: bool,
) -> Option<&'static str> {
    match role {
        "admin" => Some("admin"),
        "manager" => Some("manager"),
        "employee" if is_requester => Some("requester"),
        "employee" if is_participant => Some("participant"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_status() {
        for s in ["open", "in_progress", "in_repair"] {
            assert!(can_post_status(s), "{s}");
        }
        for s in ["completed", "rejected", "cancelled", "junk", ""] {
            assert!(!can_post_status(s), "{s}");
        }
    }

    #[test]
    fn chat_types() {
        for t in ["repair", "cartridge_replace", "free_form"] {
            assert!(request_type_has_chat(t));
        }
        assert!(!request_type_has_chat("ad_register"));
    }

    #[test]
    fn text_helpers() {
        assert_eq!(char_len("Иванов"), 6);
        assert_eq!(normalize_text("  привет \n"), "привет");
        assert!(!exceeds_limit(&"я".repeat(4000)));
        assert!(exceeds_limit(&"я".repeat(4001)));
    }

    #[test]
    fn reject_codes() {
        assert_eq!(ChatRejectCode::TooLong.as_code(), "too_long");
        assert_eq!(ChatRejectCode::Closed.as_code(), "closed");
        assert_eq!(ChatRejectCode::Forbidden.as_code(), "forbidden");
        assert_eq!(ChatRejectCode::Empty.as_code(), "empty");
        assert_eq!(ChatRejectCode::NoChat.as_code(), "no_chat");
    }

    #[test]
    fn role_labels() {
        assert_eq!(author_role_label("admin", false, false), Some("admin"));
        assert_eq!(author_role_label("manager", false, false), Some("manager"));
        assert_eq!(
            author_role_label("employee", true, false),
            Some("requester")
        );
        assert_eq!(
            author_role_label("employee", false, true),
            Some("participant")
        );
        assert_eq!(author_role_label("employee", false, false), None);
    }
}
