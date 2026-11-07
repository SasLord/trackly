//! Attachment DTOs (Phase 41.3) — shared by Tauri commands and axum handlers.
//!
//! The content hash is NEVER a DTO field: clients address a file only by its
//! row id, and the server derives the path from the stored hash (D-04).

use serde::{Deserialize, Serialize};
use specta::Type;

/// One attachment row as the UI sees it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentDto {
    #[specta(type = i32)]
    pub id: i64,
    pub original_name: String,
    pub mime: String,
    #[specta(type = i32)]
    pub size_bytes: i64,
    pub has_thumbnail: bool,
    pub viewable: bool,
    /// Blob is absent on disk (computed at read time, D-13).
    pub missing: bool,
    pub uploaded_by_name: Option<String>,
    #[specta(type = Option<i32>)]
    pub uploaded_by_id: Option<i64>,
    #[specta(type = i32)]
    pub created_at_utc: i64,
    pub deleted_by_name: Option<String>,
    #[specta(type = Option<i32>)]
    pub deleted_at_utc: Option<i64>,
}

/// List of a parent's attachments with the caller's permissions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentListDto {
    pub items: Vec<AttachmentDto>,
    /// Soft-deleted rows; filled only when the caller can manage deleted.
    pub deleted: Vec<AttachmentDto>,
    #[specta(type = i32)]
    pub max_size_bytes: i64,
    pub can_attach: bool,
    pub can_delete_any: bool,
    pub can_manage_deleted: bool,
    pub readonly: bool,
}

/// Per-file upload outcome: either the stored row or a machine reject code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentUploadDto {
    pub attachment: Option<AttachmentDto>,
    pub reject_code: Option<String>,
}

/// One history line for an attachment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentHistoryEventDto {
    #[specta(type = i32)]
    pub attachment_id: i64,
    pub name: String,
    pub action: String,
    /// "active" | "deleted" | "purged".
    pub state: String,
    pub user_name: Option<String>,
    #[specta(type = i32)]
    pub created_at_utc: i64,
}

/// Integrity report (filled by plan 08).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReportDto {
    #[specta(type = i32)]
    pub checked_at_utc: i64,
    #[specta(type = i32)]
    pub missing_files: i32,
    #[specta(type = i32)]
    pub orphan_files: i32,
    #[specta(type = i32)]
    pub size_mismatches: i32,
}

/// Size-limit setting (filled by plan 08).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentLimitDto {
    #[specta(type = i32)]
    pub max_bytes: i64,
    #[specta(type = i32)]
    pub ceiling_bytes: i64,
}
