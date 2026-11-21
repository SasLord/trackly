//! Правило D-34 (фаза 41.8): смена статуса устройств при переносе группы
//! через границу «склад <-> не склад».
//!
//! Только чистые функции. Что именно переносится (место устройства, исключения по
//! ремонту, открытая заявка) решает примитив переноса в trackly-app; здесь решается
//! одно: какой статус получает устройство с данным кодом при данном переходе.
//!
//! Выписка правила по статусам:
//! - «На складе» при уходе со склада -> «В работе»;
//! - «В работе» при переносе на склад -> «На складе»;
//! - «Ожидание ремонта», «В ремонте», «На списание», «Списано» и прочие -> статус не
//!   меняется (устройство либо переезжает как есть, либо остаётся на месте — это
//!   решает примитив по D-17 фазы 41.4);
//! - без перехода «склад <-> не склад» статус не меняется никогда.

use super::repairs::{DEVICE_STATUS_CODE_IN_STORAGE, DEVICE_STATUS_CODE_IN_USE};

/// Направление перехода через границу «склад <-> не склад».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageTransition {
    /// Со склада на не-склад.
    ToWork,
    /// С не-склада на склад.
    ToStorage,
}

impl StorageTransition {
    /// Код статуса, который получает «обычное» устройство при этом переходе.
    pub fn target_status_code(self) -> &'static str {
        match self {
            StorageTransition::ToWork => DEVICE_STATUS_CODE_IN_USE,
            StorageTransition::ToStorage => DEVICE_STATUS_CODE_IN_STORAGE,
        }
    }

    /// Код статуса, который сдвигается этим переходом.
    pub fn source_status_code(self) -> &'static str {
        match self {
            StorageTransition::ToWork => DEVICE_STATUS_CODE_IN_STORAGE,
            StorageTransition::ToStorage => DEVICE_STATUS_CODE_IN_USE,
        }
    }
}

/// Переход по признакам «склад» старого и нового места группы.
pub fn storage_transition(from_is_storage: bool, to_is_storage: bool) -> Option<StorageTransition> {
    match (from_is_storage, to_is_storage) {
        (true, false) => Some(StorageTransition::ToWork),
        (false, true) => Some(StorageTransition::ToStorage),
        _ => None,
    }
}

/// Новый код статуса устройства с кодом `status_code` при переходе `transition`;
/// `None` — статус не меняется.
pub fn status_after_move(
    transition: Option<StorageTransition>,
    status_code: &str,
) -> Option<&'static str> {
    let t = transition?;
    (status_code == t.source_status_code()).then(|| t.target_status_code())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repairs::is_repair_exempt_status_code;

    const ALL_CODES: [&str; 6] = [
        "на_складе",
        "в_работе",
        "на_ремонте",
        "в_ремонте",
        "на_списание",
        "списано",
    ];

    #[test]
    fn transition_table() {
        assert_eq!(
            storage_transition(true, false),
            Some(StorageTransition::ToWork)
        );
        assert_eq!(
            storage_transition(false, true),
            Some(StorageTransition::ToStorage)
        );
        assert_eq!(storage_transition(true, true), None);
        assert_eq!(storage_transition(false, false), None);
    }

    #[test]
    fn status_table() {
        let w = Some(StorageTransition::ToWork);
        let s = Some(StorageTransition::ToStorage);
        assert_eq!(status_after_move(w, "на_складе"), Some("в_работе"));
        assert_eq!(status_after_move(s, "в_работе"), Some("на_складе"));
        for t in [w, s] {
            for code in [
                "на_ремонте",
                "в_ремонте",
                "на_списание",
                "списано",
                "нечто",
                "",
            ] {
                assert_eq!(status_after_move(t, code), None, "{code:?}");
            }
        }
        assert_eq!(status_after_move(w, "в_работе"), None);
        assert_eq!(status_after_move(s, "на_складе"), None);
        for code in ALL_CODES {
            assert_eq!(status_after_move(None, code), None, "{code:?}");
        }
    }

    #[test]
    fn exactly_two_pairs_change_and_exempt_never() {
        let mut changed = 0;
        for t in [StorageTransition::ToWork, StorageTransition::ToStorage] {
            for code in ALL_CODES {
                let r = status_after_move(Some(t), code);
                if r.is_some() {
                    changed += 1;
                }
                if is_repair_exempt_status_code(code) {
                    assert_eq!(r, None, "{code}");
                }
            }
        }
        assert_eq!(changed, 2);
    }
}
