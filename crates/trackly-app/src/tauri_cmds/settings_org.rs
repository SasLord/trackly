//! Organisation settings + backup + templates Tauri commands — Phase 7 Plan 07.
//!
//! Mutations require ManageSettings via resolve_tauri_identity (D-Desktop-01/02).
//! `settings_move_db` and `app_restart` are Tauri-ONLY (NOT in http/settings_org.rs).
//!
//! D-19: DB move workflow — caller invokes settings_move_db then app_restart.

use std::path::Path;

use tauri_plugin_shell::ShellExt;

use crate::context::AppCtx;
use crate::dto::reports::{
    BackupConfigPatch, OrgLogoDto, OrgPatch, OrgPathDisplayDto, OrgSettingsDto, TemplateEditorItem,
    TemplateFileStatus, TemplateStatusDto,
};
use crate::pdf::html_templates::{
    read_template_if_present, resolve_templates_dir, DEFAULT_HTML_TEMPLATES, KNOWN_LEGACY_DEFAULTS,
};
use crate::services::backup_service::{BackupConfigDto, BackupResult};
use crate::tauri_cmds::users::resolve_tauri_identity;
use trackly_core::auth::{authorize, Action};
use trackly_core::domain::cartridges::LowStockBasis;
use trackly_core::domain::places::PathDisplayVariant;
use trackly_core::error::AppError;
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::place_path_settings::{
    DEFAULT_SEP_ENDS, DEFAULT_SEP_LAST_TWO, DEFAULT_VARIANT,
};

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

pub async fn build_settings_get_org_logo(ctx: &AppCtx) -> Result<OrgLogoDto, AppError> {
    ctx.org_db.get_logo().await
}

pub async fn build_settings_save_org_logo(
    ctx: &AppCtx,
    caller_identity: &trackly_core::auth::Identity,
    logo_bytes: Vec<u8>,
    logo_mime: String,
) -> Result<(), AppError> {
    ctx.org_db
        .save_logo(caller_identity, logo_bytes, logo_mime)
        .await
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
    // Return the RESOLVED db path: config override ([paths].db_path) if set,
    // otherwise the portable default. Mirrors the resolution in AppCtx::new so
    // the Settings UI shows the DB the app actually opened — not always the
    // exe-dir default — after a settings_move_db relocation (G2-2).
    let resolved = if !ctx.config.paths.db_path.is_empty() {
        ctx.config.paths.db_path.clone()
    } else {
        ctx.paths.db_path().to_string_lossy().to_string()
    };
    Ok(resolved)
}

/// Open the directory containing the DB file in the system file manager.
///
/// Security (T-07-12-01): path is derived from `ctx.paths.db_path()` — NOT from
/// user input. Canonicalized before use; UNC paths rejected.
pub async fn build_settings_open_db_folder(
    ctx: &AppCtx,
    app: tauri::AppHandle,
) -> Result<(), AppError> {
    let db_path = ctx.paths.db_path();
    let dir_path = db_path.parent().ok_or(AppError::Validation {
        field: "db_path".to_string(),
        message: "DB path has no parent directory".to_string(),
    })?;

    let canonical = std::fs::canonicalize(dir_path).map_err(|e| AppError::Validation {
        field: "db_path".to_string(),
        message: format!("Каталог БД не существует или недоступен: {e}"),
    })?;

    let canonical_str = canonical.to_string_lossy().to_string();

    // Reject UNC paths (D-UNC-01)
    if canonical_str.starts_with("\\\\") || canonical_str.starts_with("//") {
        return Err(AppError::Validation {
            field: "db_path".to_string(),
            message: "UNC-пути не поддерживаются".to_string(),
        });
    }

    // TODO(Phase 4): migrate to tauri-plugin-opener (shell::open deprecated в v2.3+).
    #[allow(deprecated)]
    app.shell()
        .open(canonical_str, None)
        .map_err(|e| AppError::Internal {
            source_chain: format!("shell::open failed: {e}"),
        })?;

    Ok(())
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
            message: format!("Порог должен быть в диапазоне 1..=999, получено {threshold}"),
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
// build_* helpers — Low stock basis (quick task 260819-wq5)
// ---------------------------------------------------------------------------

/// GET never errors on a missing/malformed stored value — falls back to
/// `LowStockBasis::DEFAULT`, mirroring `build_settings_get_low_stock_threshold`'s
/// numeric-fallback UX.
pub async fn build_settings_get_low_stock_basis(ctx: &AppCtx) -> Result<String, AppError> {
    let readers = ctx.readers.clone();
    tokio::task::spawn_blocking(move || -> Result<String, AppError> {
        let conn = readers.acquire();
        let result: rusqlite::Result<Option<String>> = conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'low_stock_basis'",
            [],
            |r| r.get(0),
        );
        let basis = match result {
            Ok(Some(s)) => LowStockBasis::parse(s.trim()).unwrap_or(LowStockBasis::DEFAULT),
            Ok(None) => LowStockBasis::DEFAULT,
            Err(rusqlite::Error::QueryReturnedNoRows) => LowStockBasis::DEFAULT,
            Err(e) => return Err(map_rusqlite(e)),
        };
        Ok(basis.as_str().to_string())
    })
    .await
    .map_err(|e| AppError::Internal {
        source_chain: format!("spawn_blocking get_low_stock_basis: {e}"),
    })?
}

/// SET rejects unknown `basis` strings with `AppError::Validation` rather
/// than silently defaulting — see CONTEXT "Валидация значения на сервере".
pub async fn build_settings_set_low_stock_basis(
    ctx: &AppCtx,
    caller_identity: &trackly_core::auth::Identity,
    basis: String,
) -> Result<(), AppError> {
    authorize(caller_identity, &Action::ManageSettings)?;

    let parsed = LowStockBasis::parse(&basis).ok_or_else(|| AppError::Validation {
        field: "basis".to_string(),
        message: format!("Недопустимое значение базы подсчёта: {basis}"),
    })?;

    let now = ctx.clock.unix_seconds();
    let value = parsed.as_str().to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
                 VALUES ('low_stock_basis', ?1, ?2, ?2) \
                 ON CONFLICT(key) DO UPDATE SET value = ?1, updated_at_utc = ?2",
                rusqlite::params![value, now],
            )
            .map(|_| ())
            .map_err(map_rusqlite)
        })
        .await
}

// ---------------------------------------------------------------------------
// build_* helpers — Place path defaults (Phase 39.1 Plan 02, PLC-07)
// ---------------------------------------------------------------------------

/// GET never errors on a missing/malformed stored value — falls back to
/// `place_path_settings::DEFAULT_VARIANT` / `DEFAULT_SEP_ENDS` /
/// `DEFAULT_SEP_LAST_TWO` (the same values V039 seeds), defensive against a
/// hand-deleted `app_settings` row. Значения намеренно не цитируются здесь:
/// у дефолта один владелец — модуль `trackly_infra::repos::place_path_settings`
/// (WR-08). Mirrors `build_settings_get_low_stock_basis`'s resilience posture.
/// Does NOT `.trim()` the separator values (D-09).
pub async fn build_settings_get_place_path_defaults(
    ctx: &AppCtx,
) -> Result<OrgPathDisplayDto, AppError> {
    let readers = ctx.readers.clone();
    tokio::task::spawn_blocking(move || -> Result<OrgPathDisplayDto, AppError> {
        let conn = readers.acquire();
        let mut stmt = conn
            .prepare(
                "SELECT key, value FROM app_settings \
                 WHERE key IN ('place_path_variant', 'place_path_sep_ends', 'place_path_sep_last_two')",
            )
            .map_err(map_rusqlite)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(map_rusqlite)?;

        // Дефолты берутся из единственного владельца (WR-08, фаза 39.2), а не
        // из литералов: сдвиг дефолта не должен требовать правки этой функции.
        let mut variant = DEFAULT_VARIANT.to_string();
        let mut sep_ends = DEFAULT_SEP_ENDS.to_string();
        let mut sep_last_two = DEFAULT_SEP_LAST_TWO.to_string();
        for row in rows {
            let (key, value) = row.map_err(map_rusqlite)?;
            match key.as_str() {
                "place_path_variant" => variant = value,
                "place_path_sep_ends" => sep_ends = value,
                "place_path_sep_last_two" => sep_last_two = value,
                _ => {}
            }
        }

        Ok(OrgPathDisplayDto {
            variant,
            sep_ends,
            sep_last_two,
        })
    })
    .await
    .map_err(|e| AppError::Internal {
        source_chain: format!("spawn_blocking get_place_path_defaults: {e}"),
    })?
}

/// SET rejects an unknown `variant` token (via `PathDisplayVariant::from_str`) and
/// an empty separator string with `AppError::Validation`, BEFORE any write — the
/// server is the source of truth (D-10), not the frontend. Deliberately checks
/// `.is_empty()`, NOT `.trim().is_empty()`: a whitespace-only separator is valid
/// and must round-trip byte-for-byte (D-09).
pub async fn build_settings_set_place_path_defaults(
    ctx: &AppCtx,
    caller_identity: &trackly_core::auth::Identity,
    patch: OrgPathDisplayDto,
) -> Result<(), AppError> {
    authorize(caller_identity, &Action::ManageSettings)?;

    // `PathDisplayVariant::from_str` (Plan 01) reports its own field as
    // `"path_variant"` (the DB/domain-internal name). Remap it to `"variant"`
    // here so the API-facing field name matches this DTO's own field name,
    // consistent with the `sep_ends`/`sep_last_two` validations below.
    PathDisplayVariant::from_str(&patch.variant).map_err(|e| match e {
        AppError::Validation { message, .. } => AppError::Validation {
            field: "variant".to_string(),
            message,
        },
        other => other,
    })?;

    if patch.sep_ends.is_empty() {
        return Err(AppError::Validation {
            field: "sep_ends".to_string(),
            message: "Разделитель «Крайние» не может быть пустым — введите хотя бы один символ."
                .to_string(),
        });
    }
    if patch.sep_last_two.is_empty() {
        return Err(AppError::Validation {
            field: "sep_last_two".to_string(),
            message: "Разделитель «Два последних» не может быть пустым — введите хотя бы один \
                      символ."
                .to_string(),
        });
    }

    let now = ctx.clock.unix_seconds();
    ctx.writer
        .execute(move |conn| {
            // WR-05 (фаза 39.2): три ключа — один набор. В autocommit отказ на
            // втором операторе оставлял «вариант новый, разделители старые» И
            // возвращал `Err` — пользователь видел «не сохранилось» при уже
            // сдвинутой настройке. Транзакция делает частичное применение
            // невозможным; доказано инъекцией отказа в
            // `set_is_atomic_partial_failure_leaves_all_three_keys_unchanged`.
            let tx = conn.transaction().map_err(map_rusqlite)?;
            // Ключ — параметр, а не кусок текста запроса: конкатенации здесь
            // заводить нельзя (T-39.2-03-02).
            for (key, value) in [
                ("place_path_variant", &patch.variant),
                ("place_path_sep_ends", &patch.sep_ends),
                ("place_path_sep_last_two", &patch.sep_last_two),
            ] {
                tx.execute(
                    "INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc) \
                     VALUES (?1, ?2, ?3, ?3) \
                     ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at_utc = ?3",
                    rusqlite::params![key, value, now],
                )
                .map_err(map_rusqlite)?;
            }
            tx.commit().map_err(map_rusqlite)
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
    ctx.templates
        .update_body(caller_identity, &kind, body)
        .await
}

pub async fn build_templates_reset_to_default(
    ctx: &AppCtx,
    caller_identity: &trackly_core::auth::Identity,
    kind: String,
) -> Result<(), AppError> {
    ctx.templates.reset_to_default(caller_identity, &kind).await
}

/// D-17: read-only per-file upgrade status for the 4 file-based HTML
/// templates (`crate::pdf::html_templates`, Plan 34-02). Never writes to
/// disk — reuses `DEFAULT_HTML_TEMPLATES`/`KNOWN_LEGACY_DEFAULTS` directly
/// rather than duplicating comparison logic.
///
/// Status derivation mirrors `upgrade_untouched_defaults_on_startup`'s
/// fail-closed classification: MISSING file OR byte-identical to the current
/// bundled default → `Current`; byte-identical to any
/// `KNOWN_LEGACY_DEFAULTS` snapshot for that filename → still `Current` (a
/// recognized legacy body is pending the SAME auto-upgrade path, not
/// user-customized); present-but-unreadable → `Unreadable` (WR-03 — it used
/// to fold into `Current`, which is exactly backwards for an endpoint whose
/// purpose is flagging files the user has touched); anything else →
/// `Customized`.
/// WR-04: the read loop runs on `spawn_blocking`, not on the async executor.
/// This function is awaited from an axum handler, and the loop does 4
/// synchronous `read_to_string` calls over files with no size cap
/// (`update_body` enforces none), so running it inline could stall a reactor
/// thread serving other LAN clients. Every other IO path in this module
/// (`build_settings_get_low_stock_threshold`, `build_settings_move_db`,
/// `OrgDbService::*`) already offloads this way.
pub async fn build_templates_status(ctx: &AppCtx) -> Result<Vec<TemplateStatusDto>, AppError> {
    let templates_dir = resolve_templates_dir(&ctx.paths);

    tokio::task::spawn_blocking(move || {
        let templates_dir_str = templates_dir.display().to_string();

        let mut out = Vec::with_capacity(DEFAULT_HTML_TEMPLATES.len());
        for (filename, current_default) in DEFAULT_HTML_TEMPLATES.iter() {
            let status = match read_template_if_present(&templates_dir, filename) {
                // Absent — not yet materialized, no evidence of customization.
                Ok(None) => TemplateFileStatus::Current,
                Ok(Some(body)) if &body == current_default => TemplateFileStatus::Current,
                Ok(Some(body)) => {
                    let legacy_bodies = KNOWN_LEGACY_DEFAULTS
                        .iter()
                        .find(|(name, _)| name == filename)
                        .map(|(_, bodies)| *bodies)
                        .unwrap_or(&[]);
                    if legacy_bodies.iter().any(|legacy| *legacy == body) {
                        TemplateFileStatus::Current // known legacy default — pending auto-upgrade, not customized
                    } else {
                        TemplateFileStatus::Customized
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Cannot read template {} ({e}) — reporting Unreadable. If you edited \
                         this file, make sure it is saved as UTF-8 (не ANSI/Windows-1251).",
                        templates_dir.join(filename).display()
                    );
                    TemplateFileStatus::Unreadable
                }
            };

            out.push(TemplateStatusDto {
                filename: filename.to_string(),
                status,
                templates_dir: templates_dir_str.clone(),
            });
        }

        out
    })
    .await
    .map_err(|e| AppError::Internal {
        source_chain: format!("spawn_blocking templates_status: {e}"),
    })
}

/// Validate template syntax + render an HTML preview with per-kind demo context.
pub async fn build_templates_validate_preview(
    ctx: &AppCtx,
    caller_identity: &trackly_core::auth::Identity,
    kind: String,
    body: String,
) -> Result<String, AppError> {
    // ManageSettings check — only editors can preview
    authorize(caller_identity, &Action::ManageSettings)?;
    ctx.templates.validate_preview(&kind, &body).await
}

// ---------------------------------------------------------------------------
// Tauri command wrappers
// ---------------------------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn settings_get_org(state: tauri::State<'_, AppCtx>) -> Result<OrgSettingsDto, AppError> {
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
) -> Result<OrgLogoDto, AppError> {
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
pub async fn settings_remove_org_logo(state: tauri::State<'_, AppCtx>) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_settings_remove_org_logo(state.inner(), &caller).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_get_db_path(state: tauri::State<'_, AppCtx>) -> Result<String, AppError> {
    build_settings_get_db_path(state.inner()).await
}

/// Open the DB folder in the system file manager. No auth guard — read-only OS action.
#[tauri::command]
#[specta::specta]
pub async fn settings_open_db_folder(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppCtx>,
) -> Result<(), AppError> {
    build_settings_open_db_folder(state.inner(), app).await
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
) -> Result<i32, AppError> {
    build_settings_get_low_stock_threshold(state.inner())
        .await
        .map(|v| v as i32)
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set_low_stock_threshold(
    state: tauri::State<'_, AppCtx>,
    threshold: i32,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_settings_set_low_stock_threshold(state.inner(), &caller, threshold as i64).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_get_low_stock_basis(
    state: tauri::State<'_, AppCtx>,
) -> Result<String, AppError> {
    build_settings_get_low_stock_basis(state.inner()).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set_low_stock_basis(
    state: tauri::State<'_, AppCtx>,
    basis: String,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_settings_set_low_stock_basis(state.inner(), &caller, basis).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_get_place_path_defaults(
    state: tauri::State<'_, AppCtx>,
) -> Result<OrgPathDisplayDto, AppError> {
    build_settings_get_place_path_defaults(state.inner()).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set_place_path_defaults(
    state: tauri::State<'_, AppCtx>,
    patch: OrgPathDisplayDto,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_settings_set_place_path_defaults(state.inner(), &caller, patch).await
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
pub async fn backup_run_manual(state: tauri::State<'_, AppCtx>) -> Result<BackupResult, AppError> {
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

/// D-17: read-only, ManageSettings-gated (deliberately stricter than the
/// unguarded `templates_list_for_editor` — see plan objective).
#[tauri::command]
#[specta::specta]
pub async fn templates_status(
    state: tauri::State<'_, AppCtx>,
) -> Result<Vec<TemplateStatusDto>, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    authorize(&caller, &Action::ManageSettings)?;
    build_templates_status(state.inner()).await
}

#[tauri::command]
#[specta::specta]
pub async fn templates_validate_preview(
    state: tauri::State<'_, AppCtx>,
    kind: String,
    body: String,
) -> Result<String, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_templates_validate_preview(state.inner(), &caller, kind, body).await
}

// ---------------------------------------------------------------------------
// Tests — build_settings_get/set_place_path_defaults (Phase 39.1 Plan 02)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod place_path {
    use super::*;
    use trackly_core::auth::Identity;

    /// Same fixture shape as `crates/trackly-app/tests/settings_ad.rs::make_test_ctx`,
    /// adapted for an in-src unit test (uses `crate::` paths directly).
    async fn make_test_ctx() -> anyhow::Result<(AppCtx, tempfile::TempDir)> {
        let dir = tempfile::TempDir::new()?;
        let dir_path = dir.path().to_path_buf();
        let paths = trackly_infra::Paths::resolve_for_exe_dir(dir_path)?;
        let config = trackly_infra::AppConfig::default();
        let log_guard = crate::logging::init(&paths, &config).or_else(|_| {
            let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
            Ok::<_, anyhow::Error>(guard)
        })?;
        let ctx = AppCtx::build(paths, config, log_guard).await?;
        Ok((ctx, dir))
    }

    /// D-09: on a fresh DB (post-V039), GET returns the migration-seeded
    /// defaults without any preceding SET.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn get_on_fresh_db_returns_migration_defaults() {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let dto = build_settings_get_place_path_defaults(&ctx)
            .await
            .expect("get_place_path_defaults");
        assert_eq!(dto.variant, DEFAULT_VARIANT);
        assert_eq!(dto.sep_ends, DEFAULT_SEP_ENDS);
        assert_eq!(dto.sep_last_two, DEFAULT_SEP_LAST_TWO);
    }

    /// D-10: unknown `variant` token is rejected with `AppError::Validation`,
    /// field == "variant" — before any write.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn set_with_bogus_variant_is_rejected() {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let admin = Identity::trusted_admin();
        let err = build_settings_set_place_path_defaults(
            &ctx,
            &admin,
            OrgPathDisplayDto {
                variant: "bogus".to_string(),
                sep_ends: " ~ ".to_string(),
                sep_last_two: " ~~ ".to_string(),
            },
        )
        .await
        .expect_err("bogus variant должен быть отклонён");
        match err {
            AppError::Validation { field, .. } => assert_eq!(field, "variant"),
            other => panic!("expected AppError::Validation{{field: \"variant\"}}, got {other:?}"),
        }
    }

    /// D-10: empty `sep_ends` is rejected with `AppError::Validation`,
    /// field == "sep_ends" — server is the source of truth, not the client.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn set_with_empty_sep_ends_is_rejected() {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let admin = Identity::trusted_admin();
        let err = build_settings_set_place_path_defaults(
            &ctx,
            &admin,
            OrgPathDisplayDto {
                variant: DEFAULT_VARIANT.to_string(),
                sep_ends: "".to_string(),
                sep_last_two: " ~~ ".to_string(),
            },
        )
        .await
        .expect_err("пустой sep_ends должен быть отклонён");
        match err {
            AppError::Validation { field, message } => {
                assert_eq!(field, "sep_ends");
                assert!(
                    message.contains("не может быть пустым"),
                    "message should mention emptiness: {message}"
                );
            }
            other => panic!("expected AppError::Validation{{field: \"sep_ends\"}}, got {other:?}"),
        }
    }

    /// D-09: whitespace-only separators are accepted (NOT trimmed) and
    /// round-trip byte-for-byte through GET after SET.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn set_with_whitespace_only_sep_round_trips_untrimmed() {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");
        let admin = Identity::trusted_admin();
        build_settings_set_place_path_defaults(
            &ctx,
            &admin,
            OrgPathDisplayDto {
                variant: "last_two".to_string(),
                sep_ends: "   ".to_string(),
                sep_last_two: ", ".to_string(),
            },
        )
        .await
        .expect("set should succeed for whitespace-only sep_ends");

        let dto = build_settings_get_place_path_defaults(&ctx)
            .await
            .expect("get_place_path_defaults");
        assert_eq!(dto.variant, "last_two");
        assert_eq!(dto.sep_ends, "   ");
        assert_eq!(dto.sep_last_two, ", ");
    }

    /// WR-05 (фаза 39.2): три ключа org-дефолтов пишутся ОДНОЙ транзакцией.
    /// Отказ на третьем операторе обязан откатить и первые два — иначе
    /// пользователь получает «не сохранилось» при уже сдвинутом варианте.
    ///
    /// Отказ инъецируется триггером `BEFORE UPDATE`: V039 засеивает все три
    /// строки, поэтому UPSERT идёт по ветке `DO UPDATE`. Триггер и на INSERT
    /// добавлен на случай, если сид когда-нибудь перестанет засеивать строку —
    /// тогда тест продолжит проверять то же самое, а не молча позеленеет.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn set_is_atomic_partial_failure_leaves_all_three_keys_unchanged() {
        let (ctx, _dir) = make_test_ctx().await.expect("make_test_ctx");

        ctx.writer
            .execute(move |conn| {
                conn.execute_batch(
                    "CREATE TRIGGER injected_fail_upd BEFORE UPDATE ON app_settings \
                     WHEN NEW.key = 'place_path_sep_last_two' \
                     BEGIN SELECT RAISE(ABORT, 'injected failure'); END; \
                     CREATE TRIGGER injected_fail_ins BEFORE INSERT ON app_settings \
                     WHEN NEW.key = 'place_path_sep_last_two' \
                     BEGIN SELECT RAISE(ABORT, 'injected failure'); END;",
                )
                .map_err(map_rusqlite)
            })
            .await
            .expect("создание триггера-инъекции");

        let admin = Identity::trusted_admin();
        let err = build_settings_set_place_path_defaults(
            &ctx,
            &admin,
            OrgPathDisplayDto {
                variant: "last".to_string(),
                sep_ends: " @ ".to_string(),
                sep_last_two: " ~ ".to_string(),
            },
        )
        .await
        .expect_err("инъецированный отказ на третьем ключе обязан вернуть Err");
        assert!(
            !matches!(err, AppError::Validation { .. }),
            "отказ должен прийти от записи, а не от валидации: {err:?}"
        );

        let dto = build_settings_get_place_path_defaults(&ctx)
            .await
            .expect("get_place_path_defaults");
        assert_eq!(
            (
                dto.variant.as_str(),
                dto.sep_ends.as_str(),
                dto.sep_last_two.as_str()
            ),
            (DEFAULT_VARIANT, DEFAULT_SEP_ENDS, DEFAULT_SEP_LAST_TWO),
            "WR-05: частичный отказ оставил применённой часть набора — записи \
             org-дефолтов не хватает транзакции"
        );
    }
}
