//! `db` — SQLite connection lifecycle, PRAGMA discipline, refinery migrations.
//!
//! Plan 03 ships `pragmas` (writer/reader PRAGMA helpers) and `migrations`
//! (refinery embed + runner). Plan 04 will add `pools` (read pool) and
//! `writer_worker` (single-writer mpsc task) here.

pub mod migrations;
pub mod pragmas;
