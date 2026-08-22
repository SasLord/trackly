//! Application services — composition layer between Tauri commands / axum handlers
//! and the repository adapters in trackly-infra.
//!
//! Services own the single-writer discipline: all writes go through
//! `WriterHandle::execute(closure)`, all reads through `ReaderPool::acquire()`.
//! Business validation (required fields, optimistic-lock checks) also lives here.

pub mod act_service;
pub mod auth;
pub mod backup_service;
pub mod cartridge_service;
pub mod dashboard_service;
pub mod device_service;
pub mod org_db_service;
pub mod organization_service;
pub mod place_service;
pub mod printer_service;
pub mod report_service;
pub mod request_service;
pub mod supervisor;
pub mod template_service;

pub use act_service::ActService;
pub use auth::AuthService;
pub use backup_service::{BackupConfigDto, BackupResult, BackupService};
pub use cartridge_service::CartridgeService;
pub use dashboard_service::DashboardService;
pub use device_service::DeviceService;
pub use org_db_service::OrgDbService;
pub use organization_service::{OrgData, OrganizationService};
pub use place_service::PlaceService;
pub use printer_service::{run_poll_task, PrinterService};
pub use report_service::ReportService;
pub use request_service::RequestService;
pub use supervisor::{run_supervisor, seed_supervisor_tasks};
pub use template_service::{TemplateService, DEFAULT_TEMPLATES};
