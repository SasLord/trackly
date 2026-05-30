//! Organization DTO — для UI чтения шапки `org.json`.
//!
//! Phase 7 расширит: edit endpoint + file-watcher. В Phase 3 — read-only.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::services::OrgData;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct OrgDto {
    pub name: String,
    pub inn: String,
    pub kpp: String,
    pub address: String,
    pub logo_path: String,
}

impl From<OrgData> for OrgDto {
    fn from(o: OrgData) -> Self {
        Self {
            name: o.name,
            inn: o.inn,
            kpp: o.kpp,
            address: o.address,
            logo_path: o.logo_path,
        }
    }
}
