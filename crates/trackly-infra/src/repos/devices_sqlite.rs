//! SQLite adapter for `DeviceRepository`.
//!
//! `SqliteDeviceRepository` implements `trackly_core::ports::devices::DeviceRepository`
//! using `rusqlite::Connection` as the `Conn` associated type.
//!
//! Scaffold only in Plan 01. Full CRUD implementation lands in Plan 03;
//! search / autocomplete / grouping lands in Plan 04.

use rusqlite::Connection;
use trackly_core::domain::devices::{
    DeviceFilter, DeviceGroupRow, DeviceNew, DevicePatch, DeviceRow, Pagination,
};
use trackly_core::error::AppError;
use trackly_core::ports::devices::DeviceRepository;

/// SQLite-backed device repository adapter.
#[derive(Debug, Default, Clone)]
pub struct SqliteDeviceRepository;

impl DeviceRepository for SqliteDeviceRepository {
    type Conn = Connection;

    fn create(
        &self,
        _conn: &mut Self::Conn,
        _new: &DeviceNew,
        _now_utc: i64,
    ) -> Result<i64, AppError> {
        todo!("Plan 03: CRUD implementation")
    }

    fn get(&self, _conn: &Self::Conn, _id: i64) -> Result<DeviceRow, AppError> {
        todo!("Plan 03: CRUD implementation")
    }

    fn list(
        &self,
        _conn: &Self::Conn,
        _filter: &DeviceFilter,
        _page: &Pagination,
    ) -> Result<(Vec<DeviceRow>, u64), AppError> {
        todo!("Plan 03: CRUD implementation")
    }

    fn update(
        &self,
        _conn: &mut Self::Conn,
        _id: i64,
        _version: i64,
        _patch: &DevicePatch,
        _now_utc: i64,
    ) -> Result<DeviceRow, AppError> {
        todo!("Plan 03: CRUD implementation")
    }

    fn delete_soft(
        &self,
        _conn: &mut Self::Conn,
        _id: i64,
        _version: i64,
        _now_utc: i64,
    ) -> Result<(), AppError> {
        todo!("Plan 03: CRUD implementation")
    }

    fn search_fts(
        &self,
        _conn: &Self::Conn,
        _fts_query: &str,
        _page: &Pagination,
    ) -> Result<Vec<DeviceRow>, AppError> {
        todo!("Plan 04: search/autocomplete/grouping implementation")
    }

    fn autocomplete(
        &self,
        _conn: &Self::Conn,
        _field: &str,
        _prefix: &str,
        _ctx_name: Option<&str>,
    ) -> Result<Vec<String>, AppError> {
        todo!("Plan 04: search/autocomplete/grouping implementation")
    }

    fn list_grouped(
        &self,
        _conn: &Self::Conn,
        _filter: &DeviceFilter,
        _page: &Pagination,
    ) -> Result<Vec<DeviceGroupRow>, AppError> {
        todo!("Plan 04: search/autocomplete/grouping implementation")
    }
}
