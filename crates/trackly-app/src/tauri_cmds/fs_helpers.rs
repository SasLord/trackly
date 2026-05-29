//! FS helper Tauri commands — Plan 05, B2 pinned strategy.
//!
//! Provides `read_file_bytes` and `write_file_bytes` as backend Tauri commands
//! with path validation (T-02-05-02):
//!   1. `canonicalize()` — resolves symlinks, rejects non-existent parents for writes.
//!   2. Reject `..` path components.
//!   3. Reject UNC paths (Windows `\\?` or `\\server\`).
//!   4. Extension whitelist: only `.csv` allowed.
//!   5. Size cap: 50 MB max for reads; content length cap for writes.
//!
//! Both commands are thin `#[tauri::command]` wrappers over `build_*` helpers,
//! which can be called directly in axum routes and tests.

use std::path::{Component, Path};

use crate::context::AppCtx;
use trackly_core::error::AppError;

// ---------------------------------------------------------------------------
// build_* helpers
// ---------------------------------------------------------------------------

/// Read a file from `path`, returning its bytes.
///
/// Path validation (T-02-05-02):
/// - canonicalize → reject `..` → reject UNC → `.csv` extension only → size ≤ 50 MB.
pub async fn build_read_file_bytes(_ctx: &AppCtx, path: String) -> Result<Vec<u8>, AppError> {
    validate_csv_path_for_read(&path)?;
    let canon = canonicalize_path(&path)?;
    // Size cap: check metadata before reading to avoid loading huge files.
    let meta = std::fs::metadata(&canon).map_err(|e| AppError::Validation {
        field: "path".to_string(),
        message: format!("Не удалось получить информацию о файле: {e}"),
    })?;
    if meta.len() > 50 * 1024 * 1024 {
        return Err(AppError::Validation {
            field: "path".to_string(),
            message: "Файл больше 50 МБ".to_string(),
        });
    }
    std::fs::read(&canon).map_err(|e| AppError::Validation {
        field: "path".to_string(),
        message: format!("Не удалось прочитать файл: {e}"),
    })
}

/// Write `content` (UTF-8 string) to `path`.
///
/// Path validation (T-02-05-02):
/// - For write: canonicalize parent dir, then join filename.
/// - Reject `..` → reject UNC → `.csv` extension only → content ≤ 50 MB.
pub async fn build_write_file_bytes(
    _ctx: &AppCtx,
    path: String,
    content: String,
) -> Result<(), AppError> {
    validate_csv_path_for_write(&path)?;
    if content.len() > 50 * 1024 * 1024 {
        return Err(AppError::Validation {
            field: "path".to_string(),
            message: "Содержимое больше 50 МБ".to_string(),
        });
    }
    let write_path = resolve_write_path(&path)?;
    std::fs::write(&write_path, content.as_bytes()).map_err(|e| AppError::Validation {
        field: "path".to_string(),
        message: format!("Не удалось записать файл: {e}"),
    })
}

// ---------------------------------------------------------------------------
// Path validation helpers (T-02-05-02)
// ---------------------------------------------------------------------------

/// Validate path for read:
/// - canonicalize succeeds (file must exist for reads)
/// - no `..` components
/// - no UNC
/// - `.csv` extension
fn validate_csv_path_for_read(path: &str) -> Result<(), AppError> {
    reject_unc(path)?;
    let p = Path::new(path);
    reject_parent_dir_components(p)?;
    require_csv_extension(p)?;
    Ok(())
}

/// Validate path for write:
/// - no `..` in user-supplied string
/// - no UNC
/// - `.csv` extension
fn validate_csv_path_for_write(path: &str) -> Result<(), AppError> {
    reject_unc(path)?;
    let p = Path::new(path);
    reject_parent_dir_components(p)?;
    require_csv_extension(p)?;
    Ok(())
}

fn canonicalize_path(path: &str) -> Result<std::path::PathBuf, AppError> {
    std::fs::canonicalize(path).map_err(|e| AppError::Validation {
        field: "path".to_string(),
        message: format!("Не удалось распознать путь: {e}"),
    })
}

/// For write: canonicalize the *parent directory* (which must exist),
/// then join the filename back to get a resolved absolute write path.
fn resolve_write_path(path: &str) -> Result<std::path::PathBuf, AppError> {
    let p = Path::new(path);
    let parent = p.parent().unwrap_or(Path::new("."));
    let canon_parent = std::fs::canonicalize(parent).map_err(|e| AppError::Validation {
        field: "path".to_string(),
        message: format!("Не удалось распознать родительскую папку: {e}"),
    })?;
    let filename = p.file_name().ok_or_else(|| AppError::Validation {
        field: "path".to_string(),
        message: "Путь не содержит имени файла".to_string(),
    })?;
    let full = canon_parent.join(filename);
    // Re-check for `..` after joining (should be impossible but defense-in-depth).
    reject_parent_dir_components(&full)?;
    Ok(full)
}

fn reject_unc(path: &str) -> Result<(), AppError> {
    if path.starts_with("\\\\") || path.starts_with("//") {
        return Err(AppError::Validation {
            field: "path".to_string(),
            message: "UNC-пути не поддерживаются".to_string(),
        });
    }
    Ok(())
}

fn reject_parent_dir_components(p: &Path) -> Result<(), AppError> {
    if p.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(AppError::Validation {
            field: "path".to_string(),
            message: "Путь не должен содержать «..»".to_string(),
        });
    }
    Ok(())
}

fn require_csv_extension(p: &Path) -> Result<(), AppError> {
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    if ext.as_deref() != Some("csv") {
        return Err(AppError::Validation {
            field: "path".to_string(),
            message: "Разрешены только файлы .csv".to_string(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri commands — thin wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn read_file_bytes(
    state: tauri::State<'_, AppCtx>,
    path: String,
) -> Result<Vec<u8>, AppError> {
    build_read_file_bytes(state.inner(), path).await
}

#[tauri::command]
#[specta::specta]
pub async fn write_file_bytes(
    state: tauri::State<'_, AppCtx>,
    path: String,
    content: String,
) -> Result<(), AppError> {
    build_write_file_bytes(state.inner(), path, content).await
}

// ---------------------------------------------------------------------------
// Unit tests for path validation
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_dotdot_in_path() {
        let result = validate_csv_path_for_read("/some/../etc/file.csv");
        assert!(result.is_err(), "path with .. should be rejected");
        let err = format!("{:?}", result.unwrap_err());
        assert!(
            err.contains("..") || err.contains("родитель") || err.contains("Путь"),
            "{err}"
        );
    }

    #[test]
    fn rejects_unc_path() {
        let result = validate_csv_path_for_read("\\\\server\\share\\file.csv");
        assert!(result.is_err(), "UNC path should be rejected");
    }

    #[test]
    fn rejects_non_csv_extension() {
        let result = validate_csv_path_for_read("/home/user/data.exe");
        assert!(result.is_err(), "non-csv extension should be rejected");
    }

    #[test]
    fn accepts_csv_extension() {
        // Only validates extension (canonicalize not called yet), so path need not exist.
        let result = validate_csv_path_for_read("/home/user/data.csv");
        assert!(
            result.is_ok(),
            "csv extension should be accepted: {result:?}"
        );
    }

    #[test]
    fn rejects_unc_double_slash() {
        let result = validate_csv_path_for_read("//server/share/file.csv");
        assert!(result.is_err(), "// UNC path should be rejected");
    }

    #[test]
    fn write_validates_extension() {
        let result = validate_csv_path_for_write("/tmp/output.txt");
        assert!(
            result.is_err(),
            "non-csv extension should be rejected for write"
        );
    }

    #[test]
    fn write_accepts_csv() {
        let result = validate_csv_path_for_write("/tmp/output.csv");
        assert!(
            result.is_ok(),
            "csv should be accepted for write: {result:?}"
        );
    }
}
