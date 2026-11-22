//! `HistoryEntryDto` — серверно склеенная лента истории (фаза 41.10, D-02).
//!
//! Одна запись ленты = одна операция: перемещение (`place_movements`) и события
//! (`history_events`) склеены на сервере по `movement_id` / `operation_id` (D-25, D-04).
//! Токены `kind` / `source` отдаются сырыми строками (D-23): неизвестное значение из
//! будущей версии не роняет ленту, UI подбирает текст сам.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::dto::place_movements::MovementEntryDto;

/// Один сегмент записи ленты: состав, место или статус.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct HistorySegmentDto {
    /// `"move"` | `"membership"` | `"status"`; неизвестное значение допустимо.
    pub kind: String,
    /// Сырой токен `HistoryEventKind` (для `kind = "move"` — `None`).
    pub event_kind: Option<String>,
    /// Перемещение целиком (для `kind = "move"`).
    pub movement: Option<MovementEntryDto>,
    pub subject_type: Option<String>,
    #[specta(type = Option<i32>)]
    pub subject_id: Option<i64>,
    pub subject_label: Option<String>,
    #[specta(type = Option<i32>)]
    pub from_group_id: Option<i64>,
    pub from_group_label: Option<String>,
    #[specta(type = Option<i32>)]
    pub to_group_id: Option<i64>,
    pub to_group_label: Option<String>,
    pub status_from_code: Option<String>,
    pub status_from_label: Option<String>,
    pub status_to_code: Option<String>,
    pub status_to_label: Option<String>,
}

/// Запись ленты истории.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct HistoryEntryDto {
    /// Уникальный ключ записи: `"m:<id>"` (чистое перемещение) / `"e:<operation_id>"`.
    pub key: String,
    #[specta(type = i32)]
    pub created_at_utc: i64,
    /// ФИО-снимок → login → «система».
    pub actor_display: String,
    /// Сырой токен причины.
    pub reason_source: String,
    /// `true` — основание из словаря `HistoryReason`; `false` — из `movementReason.ts`.
    pub reason_is_event: bool,
    #[specta(type = Option<i32>)]
    pub act_id: Option<i64>,
    pub act_number: Option<String>,
    #[specta(type = Option<i32>)]
    pub request_id: Option<i64>,
    #[specta(type = Option<i32>)]
    pub ref_group_id: Option<i64>,
    pub ref_group_label: Option<String>,
    pub segments: Vec<HistorySegmentDto>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Вымышленный пример (privacy gate).
    fn sample() -> HistoryEntryDto {
        HistoryEntryDto {
            key: "e:op-1".to_string(),
            created_at_utc: 1_700_000_000,
            actor_display: "Иванов И.И.".to_string(),
            reason_source: "manual".to_string(),
            reason_is_event: true,
            act_id: None,
            act_number: None,
            request_id: None,
            ref_group_id: None,
            ref_group_label: None,
            segments: vec![HistorySegmentDto {
                kind: "membership".to_string(),
                event_kind: Some("group_joined".to_string()),
                movement: None,
                subject_type: Some("device".to_string()),
                subject_id: Some(1),
                subject_label: Some("АРМ-3".to_string()),
                from_group_id: None,
                from_group_label: None,
                to_group_id: Some(2),
                to_group_label: Some("АРМ-5".to_string()),
                status_from_code: None,
                status_from_label: None,
                status_to_code: None,
                status_to_label: None,
            }],
        }
    }

    #[test]
    fn unknown_kind_passes_through_without_panicking() {
        let mut dto = sample();
        dto.segments[0].kind = "future_kind".to_string();
        dto.segments[0].event_kind = Some("garbage".to_string());
        dto.reason_source = "garbage".to_string();
        let json = serde_json::to_value(&dto).expect("serialize");
        assert_eq!(json["segments"][0]["kind"], "future_kind");
        assert_eq!(json["segments"][0]["event_kind"], "garbage");
        assert_eq!(json["reason_source"], "garbage");
    }
}
