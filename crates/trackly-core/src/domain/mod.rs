//! Domain model — pure value types for all entities.
//!
//! No I/O dependencies allowed in this module (enforced by `tests/no_io_deps.rs`).

pub mod acts;
pub mod attachments;
pub mod cartridges;
pub mod chat;
pub mod devices;
pub mod group_move_status;
pub mod group_values;
pub mod groups;
pub mod history_events;
pub mod im;
pub mod number_templates;
pub mod place_movements;
pub mod places;
pub mod printers;
pub mod repairs;
pub mod requests;
