//! G-5 — DTO для autocomplete-сервисов.
//!
//! Изолирован от `dto/act.rs` (W-2): wave-1 `plan 03.1-01` модифицирует
//! `dto/act.rs` (ActItemDto/ActReturnItemDto shift), wave-1 `plan 03.1-02`
//! owns этот файл — file-level disjoint, без merge conflict.
//!
//! Phase 5 (future): источник UNION с AD displayName — расширение `enum`
//! не требуется, сервис добавит вторую ветку в SQL `UNION ALL`.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Serialize, Deserialize, Type, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SuggestPersonField {
    Giver,
    Receiver,
}
