//! Чистые правила ядра мессенджера im_* (Phase 43, DLV-02/DLV-03).
//!
//! Идентификаторы клиента, документ-обёртка тела сообщения, окно видимости
//! участника и пагинация sync. Нормализацию и лимит длины текста сервис берёт из
//! `domain::chat::{normalize_text, exceeds_limit}` — здесь не дублируются.

use serde_json::json;

/// Максимальная длина клиентского идентификатора (id сообщения / заявки).
pub const CLIENT_ID_MAX_LEN: usize = 64;
/// Размер страницы sync по умолчанию (ADR 0003, раздел протокола).
pub const SYNC_PAGE_DEFAULT: i64 = 50;
/// Верхняя граница размера страницы sync (ADR 0003, раздел протокола).
pub const SYNC_PAGE_MAX: i64 = 200;
/// Порог числа пропущенных событий, после которого sync отвечает `reload:true`
/// (ADR 0003, раздел протокола).
pub const SYNC_RELOAD_THRESHOLD: i64 = 200;

/// Клиентский id допустим: UUID или `[A-Za-z0-9_-]`, длина 1..=64.
pub fn is_valid_client_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= CLIENT_ID_MAX_LEN
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Версионируемая обёртка документа для простого текста (D-01). Валидатор
/// rich-документа — Фаза 47.
pub fn body_doc_json(text: &str) -> String {
    json!({
        "v": 1,
        "doc": {
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [{ "type": "text", "text": text }]
            }]
        }
    })
    .to_string()
}

/// Ключ диалога двух пользователей: меньший id первым.
pub fn dialog_key(a: i64, b: i64) -> String {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    format!("{lo}:{hi}")
}

/// Размер страницы: None -> по умолчанию, иначе в пределах 1..=SYNC_PAGE_MAX.
pub fn clamp_page_limit(limit: Option<i64>) -> i64 {
    match limit {
        None => SYNC_PAGE_DEFAULT,
        Some(n) => n.clamp(1, SYNC_PAGE_MAX),
    }
}

/// Окно видимости участника в беседе.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibleWindow {
    /// Нижняя граница seq (включительно).
    pub lo_seq: i64,
    /// Верхняя граница seq (включительно); None — без ограничения.
    pub hi_seq: Option<i64>,
    /// Верхняя граница rev; правки после выхода не доставляются (D-06).
    pub max_rev: Option<i64>,
}

/// Строка участника -> окно видимости (D-06): исключённый видит замороженную
/// копию до `left_seq`.
pub fn member_window(visible_from_seq: i64, left_seq: Option<i64>) -> VisibleWindow {
    VisibleWindow {
        lo_seq: visible_from_seq,
        hi_seq: left_seq,
        max_rev: left_seq,
    }
}

/// Писать в беседу может только не вышедший участник.
pub fn can_post(left_at_utc: Option<i64>) -> bool {
    left_at_utc.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_id_accepts_uuid_and_simple() {
        assert!(is_valid_client_id("123e4567-e89b-12d3-a456-426614174000"));
        assert!(is_valid_client_id("a_B-9"));
        assert!(is_valid_client_id(&"x".repeat(64)));
    }

    #[test]
    fn client_id_rejects_bad() {
        assert!(!is_valid_client_id(""));
        assert!(!is_valid_client_id(&"x".repeat(65)));
        assert!(!is_valid_client_id("a b"));
        assert!(!is_valid_client_id("привет"));
        assert!(!is_valid_client_id("a\nb"));
        assert!(!is_valid_client_id("a\u{0}b"));
    }

    #[test]
    fn body_doc_wraps_text() {
        let s = body_doc_json("привет\nмир");
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["v"], 1);
        assert_eq!(v["doc"]["content"][0]["content"][0]["text"], "привет\nмир");
    }

    #[test]
    fn window_active_member_unbounded() {
        let w = member_window(0, None);
        assert_eq!(
            w,
            VisibleWindow {
                lo_seq: 0,
                hi_seq: None,
                max_rev: None
            }
        );
    }

    #[test]
    fn window_left_member_bounded() {
        let w = member_window(3, Some(10));
        assert_eq!(w.lo_seq, 3);
        assert_eq!(w.hi_seq, Some(10));
        assert_eq!(w.max_rev, Some(10));
    }

    #[test]
    fn page_limit_clamps() {
        assert_eq!(clamp_page_limit(None), SYNC_PAGE_DEFAULT);
        assert_eq!(clamp_page_limit(Some(0)), 1);
        assert_eq!(clamp_page_limit(Some(-5)), 1);
        assert_eq!(clamp_page_limit(Some(30)), 30);
        assert_eq!(clamp_page_limit(Some(10_000)), SYNC_PAGE_MAX);
    }

    #[test]
    fn dialog_key_is_symmetric() {
        assert_eq!(dialog_key(7, 3), "3:7");
        assert_eq!(dialog_key(3, 7), dialog_key(7, 3));
    }

    #[test]
    fn can_post_only_for_active() {
        assert!(can_post(None));
        assert!(!can_post(Some(1_700_000_000)));
    }
}
