//! `db` — SQLite connection lifecycle, PRAGMA discipline, single-writer worker,
//! reader pool, refinery migrations.

pub mod close_serializer;
pub mod migrations;
pub mod pools;
pub mod pragmas;
pub mod writer_worker;

pub use pools::{ReaderHandle, ReaderPool};
pub use writer_worker::{WriterHandle, DEFAULT_WRITER_CAPACITY};
