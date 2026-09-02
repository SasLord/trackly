//! DTOs — единый источник истины для двух транспортов (Tauri command +
//! axum HTTP).
//!
//! Plan 05: только `HealthDto`. Каждое следующее phase добавляет DTO сюда
//! (devices, acts, cartridges, …), и они автоматически экспортируются в
//! `ui/src/bindings.ts` через `specta_export::builder` (Builder собирает все
//! DTOs, на которые ссылаются зарегистрированные команды).

pub mod act;
pub mod auth;
pub mod cartridge;
pub mod device;
pub mod health;
pub mod organization;
pub mod place;
pub mod place_movements;
pub mod printer;
pub mod reports;
pub mod request;
pub mod suggest;

pub use health::HealthDto;
pub use organization::OrgDto;
pub use printer::WsEvent;
pub use reports::{
    BackupConfigPatch, ConsumptionPoint, DashboardStatusCount, DashboardWidgetDto, OrgLogoDto,
    OrgPatch, OrgSettingsDto, PeriodDto, ReportFilter, ReportResponse, ReportRow,
    TemplateEditorItem,
};
