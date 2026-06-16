//! Organisation settings + backup + templates Tauri commands — Phase 7 Plan 07.
//!
//! Mutations require ManageSettings via resolve_tauri_identity (D-Desktop-01/02).
//! `settings_move_db` and `app_restart` are Tauri-ONLY (NOT in http/settings_org.rs).
//!
//! D-19: DB move workflow — caller invokes settings_move_db then app_restart.

use std::path::Path;

use crate::context::AppCtx;
use crate::dto::reports::{
    BackupConfigPatch, OrgPatch, OrgSettingsDto, TemplateEditorItem,
};
use crate::services::backup_service::{BackupConfigDto, BackupResult};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action};
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;

// ---------------------------------------------------------------------------
// build_* helpers — Organisation settings
// ---------------------------------------------------------------------------

pub async fn build_settings_get_org(ctx: &AppCtx) -> Result<OrgSettingsDto, AppError> {
    ctx.org_db.get().await
}

pub async fn build_settings_save_org_fields(
    ctx: &AppCtx,
    caller_identity: &trackly_core::auth::Identity,
    patch: OrgPatch,
) -> Result<(), AppError> {
    ctx.org_db.save_fields(caller_identity, patch).await
}

pub async fn build_settings_get_org_logo(ctx: &AppCtx) -> Result<Vec<u8>, AppError> {
    Ok(ctx.org_db.get_logo_bytes().await?.unwrap_or_default())
}

pub async fn build_settings_save_org_logo(
    ctx: &AppCtx,
    caller_identity: &trackly_core::auth::Identity,
    logo_bytes: Vec<u8>,
    logo_mime: String,
) -> Result<(), AppError> {
    ctx.org_db.save_logo(caller_identity, logo_bytes, logo_mime).await
}

pub async fn build_settings_remove_org_logo(
    ctx: &AppCtx,
    caller_identity: &trackly_core::auth::Identity,
) -> Result<(), AppError> {
    ctx.org_db.remove_logo(caller_identity).await
}

// ---------------------------------------------------------------------------
// build_* helpers — DB path / move / restart (Tauri-only, D-19)
// ---------------------------------------------------------------------------

pub async fn build_settings_get_db_path(ctx: &AppCtx) -> Result<String, AppError> {
    Ok(ctx.paths.db_path().to_string_lossy().to_string())
}

/// Move DB to new_path via rusqlite::backup::Backup then update config.
/// Returns Ok(()) — caller invokes app_restart unconditionally after this.
///
/// Security (T-07-07-03): this function is Tauri-only, NOT exposed in HTTP router.
pub async fn build_settings_move_db(
    ctx: &AppCtx,
    caller_identity: &trackly_core::auth::Identity,
    new_path: String,
) -> Result<(), AppError> {
    authorize(caller_identity, &Action::ManageSettings)?;

    // UNC rejection (D-UNC-01)
    if new_path.starts_with("\\\\") || new_path.starts_with("//") {
        return Err(AppError::Validation {
            field: "new_path".to_string(),
            message: "UNC-пути не поддерживаются".to_string(),
        });
    }

    // Step 1: Copy DB to new path via BackupService::backup_to_path.
    ctx.backup.backup_to_path(Path::new(&new_path)).await?;

    // Step 2: Update [paths].db_path in config file.
    // We rewrite the config by parsing via toml and re-serializing with the new db_path.
    let config_path = ctx.paths.config_file().to_path_buf();
    let new_path_clone = new_path.clone();
    tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        use std::collections::BTreeMap;

        // Parse existing config or use empty map
        let raw = std::fs::read_to_string(&config_path).unwrap_or_default();
        let mut doc: BTreeMap<String, toml::Value> = toml::from_str(&raw).unwrap_or_default();

        // Update or insert [paths] section with new db_path
        let paths_table = doc
            .entry("paths".to_string())
            .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));
        if let toml::Value::Table(ref mut t) = paths_table {
            t.insert("db_path".to_string(), toml::Value::String(new_path_clone));
        }

        let new_toml = toml::to_string_pretty(&doc).map_err(|e| AppError::Internal {
            source_chain: format!("serialize config toml: {e}"),
        })?;

        std::fs::write(&config_path, new_toml).map_err(|e| AppError::Internal {
            source_chain: format!("write config: {e}"),
        })
    })
    .await
    .map_err(|e| AppError::Internal {
        source_chain: format!("spawn_blocking move_db: {e}"),
    })??;

    Ok(())
}

// ---------------------------------------------------------------------------
// build_* helpers — Low stock threshold
// ---------------------------------------------------------------------------

pub async fn build_settings_get_low_stock_threshold(ctx: &AppCtx) -> Result<i64, AppError> {
    let readers = ctx.readers.clone();
    tokio::task::spawn_blocking(move || -> Result<i64, AppError> {
        let conn = readers.acquire();
        let result: rusqlite::Result<Option<String>> = conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'low_stock_threshold'",
            [],
            |r| r.get(0),
        );
        match result {
            Ok(Some(s)) => Ok(s.parse::<i64>().unwrap_or(5)),
            Ok(None) => Ok(5), // default threshold
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(5),
            Err(e) => Err(map_rusqlite(e)),
        }
    })
    .await
    .map_err(|e| AppError::Internal {
        source_chain: format!("spawn_blocking get_low_stock_threshold: {e}"),
    })?
}

pub async fn build_settings_set_low_stock_threshold(
    ctx: &AppCtx,
    caller_identity: &trackly_core::auth::Identity,
    threshold: i64,
) -> Result<(), AppError> {
    authorize(caller_identity, &Action::ManageSettings)?;

    if !(1..=999).contains(&threshold) {
        return Err(AppError::Validation {
            field: "threshold".to_string(),
            message: format!(
                "Порог должен быть в диапазоне 1..=999, получено {threshold}"
            ),
        });
    }

    let now = ctx.clock.unix_seconds();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
                 VALUES ('low_stock_threshold', ?1, ?2, ?2) \
                 ON CONFLICT(key) DO UPDATE SET value = ?1, updated_at_utc = ?2",
                rusqlite::params![threshold.to_string(), now],
            )
            .map(|_| ())
            .map_err(map_rusqlite)
        })
        .await
}

// ---------------------------------------------------------------------------
// build_* helpers — Backup config
// ---------------------------------------------------------------------------

pub async fn build_settings_get_backup_config(ctx: &AppCtx) -> Result<BackupConfigDto, AppError> {
    ctx.backup.get_config().await
}

pub async fn build_settings_save_backup_config(
    ctx: &AppCtx,
    caller_identity: &trackly_core::auth::Identity,
    patch: BackupConfigPatch,
) -> Result<(), AppError> {
    ctx.backup.set_config(caller_identity, patch).await
}

pub async fn build_backup_run_manual(ctx: &AppCtx) -> Result<BackupResult, AppError> {
    let config = ctx.backup.get_config().await?;
    let folder = config.backup_folder.ok_or_else(|| AppError::Validation {
        field: "backup_folder".to_string(),
        message: "Папка не выбрана".to_string(),
    })?;
    ctx.backup.run_backup(&folder).await
}

// ---------------------------------------------------------------------------
// build_* helpers — Templates
// ---------------------------------------------------------------------------

pub async fn build_templates_list_for_editor(
    ctx: &AppCtx,
) -> Result<Vec<TemplateEditorItem>, AppError> {
    ctx.templates.list_all_for_editor().await
}

pub async fn build_templates_update_body(
    ctx: &AppCtx,
    caller_identity: &trackly_core::auth::Identity,
    kind: String,
    body: String,
) -> Result<(), AppError> {
    ctx.templates.update_body(caller_identity, &kind, body).await
}

pub async fn build_templates_reset_to_default(
    ctx: &AppCtx,
    caller_identity: &trackly_core::auth::Identity,
    kind: String,
) -> Result<(), AppError> {
    ctx.templates.reset_to_default(caller_identity, &kind).await
}

/// Validate template syntax + render preview PDF with dummy context.
pub async fn build_templates_validate_preview(
    ctx: &AppCtx,
    caller_identity: &trackly_core::auth::Identity,
    _kind: String,
    body: String,
) -> Result<Vec<u8>, AppError> {
    // ManageSettings check — only editors can preview
    authorize(caller_identity, &Action::ManageSettings)?;
    ctx.templates.validate_preview(&body).await
}

// ---------------------------------------------------------------------------
// Tauri command wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn settings_get_org(
    state: tauri::State<'_, AppCtx>,
) -> Result<OrgSettingsDto, AppError> {
    build_settings_get_org(state.inner()).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_save_org_fields(
    state: tauri::State<'_, AppCtx>,
    patch: OrgPatch,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_settings_save_org_fields(state.inner(), &caller, patch).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_get_org_logo(
    state: tauri::State<'_, AppCtx>,
) -> Result<Vec<u8>, AppError> {
    build_settings_get_org_logo(state.inner()).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_save_org_logo(
    state: tauri::State<'_, AppCtx>,
    logo_bytes: Vec<u8>,
    logo_mime: String,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_settings_save_org_logo(state.inner(), &caller, logo_bytes, logo_mime).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_remove_org_logo(
    state: tauri::State<'_, AppCtx>,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_settings_remove_org_logo(state.inner(), &caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_get_db_path(
    state: tauri::State<'_, AppCtx>,
) -> Result<String, AppError> {
    build_settings_get_db_path(state.inner()).await
}

/// Move DB to a new location. Tauri-only — NOT registered in HTTP router (T-07-07-03).
#[tauri::command]
#[specta::specta]
pub async fn settings_move_db(
    state: tauri::State<'_, AppCtx>,
    new_path: String,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_settings_move_db(state.inner(), &caller, new_path).await
}

/// Restart the app (Tauri-only, D-19). Called by StorageSettings.svelte after
/// successful settings_move_db to complete the DB relocation workflow.
#[tauri::command]
#[specta::specta]
pub async fn app_restart(app: tauri::AppHandle) -> Result<(), AppError> {
    app.request_restart();
    #[allow(unreachable_code)]
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn settings_get_low_stock_threshold(
    state: tauri::State<'_, AppCtx>,
) -> Result<i64, AppError> {
    build_settings_get_low_stock_threshold(state.inner()).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set_low_stock_threshold(
    state: tauri::State<'_, AppCtx>,
    threshold: i64,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_settings_set_low_stock_threshold(state.inner(), &caller, threshold).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_get_backup_config(
    state: tauri::State<'_, AppCtx>,
) -> Result<BackupConfigDto, AppError> {
    build_settings_get_backup_config(state.inner()).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_save_backup_config(
    state: tauri::State<'_, AppCtx>,
    patch: BackupConfigPatch,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_settings_save_backup_config(state.inner(), &caller, patch).await
}

#[tauri::command]
#[specta::specta]
pub async fn backup_run_manual(
    state: tauri::State<'_, AppCtx>,
) -> Result<BackupResult, AppError> {
    build_backup_run_manual(state.inner()).await
}

#[tauri::command]
#[specta::specta]
pub async fn templates_list_for_editor(
    state: tauri::State<'_, AppCtx>,
) -> Result<Vec<TemplateEditorItem>, AppError> {
    build_templates_list_for_editor(state.inner()).await
}

#[tauri::command]
#[specta::specta]
pub async fn templates_update_body(
    state: tauri::State<'_, AppCtx>,
    kind: String,
    body: String,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_templates_update_body(state.inner(), &caller, kind, body).await
}

#[tauri::command]
#[specta::specta]
pub async fn templates_reset_to_default(
    state: tauri::State<'_, AppCtx>,
    kind: String,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_templates_reset_to_default(state.inner(), &caller, kind).await
}

#[tauri::command]
#[specta::specta]
pub async fn templates_validate_preview(
    state: tauri::State<'_, AppCtx>,
    kind: String,
    body: String,
) -> Result<Vec<u8>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_templates_validate_preview(state.inner(), &caller, kind, body).await
}
