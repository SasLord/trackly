//! Словарь токенов ленты истории (фаза 41.10, D-23).
//!
//! Чистый доменный слой без I/O (`tests/no_io_deps.rs`). Таблица `history_events`
//! (V057) хранит `kind`/`source` как TEXT без CHECK; читающий код обязан вызывать
//! `from_str_lenient`: неизвестный токен даёт `None`, а не панику и не ошибку.

/// Вид события истории.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryEventKind {
    GroupJoined,
    GroupLeft,
    GroupTransferred,
    GroupFormed,
    GroupDisbanded,
    GroupNested,
    GroupUnnested,
    StatusChanged,
}

impl HistoryEventKind {
    pub const ALL: [HistoryEventKind; 8] = [
        Self::GroupJoined,
        Self::GroupLeft,
        Self::GroupTransferred,
        Self::GroupFormed,
        Self::GroupDisbanded,
        Self::GroupNested,
        Self::GroupUnnested,
        Self::StatusChanged,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GroupJoined => "group_joined",
            Self::GroupLeft => "group_left",
            Self::GroupTransferred => "group_transferred",
            Self::GroupFormed => "group_formed",
            Self::GroupDisbanded => "group_disbanded",
            Self::GroupNested => "group_nested",
            Self::GroupUnnested => "group_unnested",
            Self::StatusChanged => "status_changed",
        }
    }

    pub fn from_str_lenient(s: &str) -> Option<Self> {
        match s {
            "group_joined" => Some(Self::GroupJoined),
            "group_left" => Some(Self::GroupLeft),
            "group_transferred" => Some(Self::GroupTransferred),
            "group_formed" => Some(Self::GroupFormed),
            "group_disbanded" => Some(Self::GroupDisbanded),
            "group_nested" => Some(Self::GroupNested),
            "group_unnested" => Some(Self::GroupUnnested),
            "status_changed" => Some(Self::StatusChanged),
            _ => None,
        }
    }
}

/// Причина события (токен `source` в `history_events`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryReason {
    Manual,
    Act,
    Repair,
    GroupMove,
    GroupCreate,
    GroupDisband,
    DeviceWriteOff,
    GroupWriteOff,
    Teardown,
    /// UAT-3 (фаза 41.10): удаление акта вернуло устройство, которое перемещали после акта,
    /// на место «до акта» с отметкой в истории. `act_id` у записи нет (акт удалён), номер
    /// акта несёт примечание строки перемещения.
    ActDelete,
}

impl HistoryReason {
    pub const ALL: [HistoryReason; 10] = [
        Self::Manual,
        Self::Act,
        Self::Repair,
        Self::GroupMove,
        Self::GroupCreate,
        Self::GroupDisband,
        Self::DeviceWriteOff,
        Self::GroupWriteOff,
        Self::Teardown,
        Self::ActDelete,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Act => "act",
            Self::Repair => "repair",
            Self::GroupMove => "group_move",
            Self::GroupCreate => "group_create",
            Self::GroupDisband => "group_disband",
            Self::DeviceWriteOff => "device_write_off",
            Self::GroupWriteOff => "group_write_off",
            Self::Teardown => "teardown",
            Self::ActDelete => "act_delete",
        }
    }

    pub fn from_str_lenient(s: &str) -> Option<Self> {
        match s {
            "manual" => Some(Self::Manual),
            "act" => Some(Self::Act),
            "repair" => Some(Self::Repair),
            "group_move" => Some(Self::GroupMove),
            "group_create" => Some(Self::GroupCreate),
            "group_disband" => Some(Self::GroupDisband),
            "device_write_off" => Some(Self::DeviceWriteOff),
            "group_write_off" => Some(Self::GroupWriteOff),
            "teardown" => Some(Self::Teardown),
            "act_delete" => Some(Self::ActDelete),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn kind_roundtrip_and_unique() {
        let mut seen = HashSet::new();
        for k in HistoryEventKind::ALL {
            assert_eq!(HistoryEventKind::from_str_lenient(k.as_str()), Some(k));
            assert!(seen.insert(k.as_str()), "дубль токена {}", k.as_str());
        }
        assert_eq!(HistoryEventKind::from_str_lenient("мусор"), None);
        assert_eq!(HistoryEventKind::from_str_lenient(""), None);
    }

    #[test]
    fn reason_roundtrip_and_unique() {
        let mut seen = HashSet::new();
        for r in HistoryReason::ALL {
            assert_eq!(HistoryReason::from_str_lenient(r.as_str()), Some(r));
            assert!(seen.insert(r.as_str()), "дубль токена {}", r.as_str());
        }
        assert_eq!(HistoryReason::from_str_lenient("мусор"), None);
    }
}
