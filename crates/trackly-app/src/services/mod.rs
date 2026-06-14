//! Application services — composition layer between Tauri commands / axum handlers
//! and the repository adapters in trackly-infra.
//!
//! Services own the single-writer discipline: all writes go through
//! `WriterHandle::execute(closure)`, all reads through `ReaderPool::acquire()`.
//! Business validation (required fields, optimistic-lock checks) also lives here.

pub mod act_service;
pub mod auth;
pub mod cartridge_service;
pub mod device_service;
pub mod organization_service;
pub mod printer_service;
pub mod request_service;
pub mod template_service;

pub use act_service::ActService;
pub use auth::AuthService;
pub use cartridge_service::CartridgeService;
pub use device_service::DeviceService;
pub use organization_service::{OrgData, OrganizationService};
pub use printer_service::{run_poll_task, PrinterService};
pub use request_service::RequestService;
pub use template_service::{TemplateService, DEFAULT_TEMPLATES};
