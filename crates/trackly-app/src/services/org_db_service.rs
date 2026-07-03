//! `OrgDbService` — чтение и запись настроек организации через таблицу `org_settings`.
//!
//! Заменяет `OrganizationService` (который работал с `org.json`) — данные теперь
//! хранятся в БД (V026 migration). Миграция из `org.json` выполняется один раз
//! при первом запуске нового приложения.
//!
//! Security (T-07-02-01):
//!   - Лого: до 512 КБ; mime ограничен allowlist (image/png | image/jpeg | image/svg+xml).
//!   - Все мутирующие методы требуют `Action::ManageSettings`.

use std::path::PathBuf;
use std::sync::Arc;

use rusqlite::params;
use trackly_core::auth::{authorize, Action, Identity};
use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::Paths;

use crate::dto::reports::OrgSettingsDto;
use crate::services::organization_service::OrgData;

const LOGO_MAX_BYTES: usize = 512 * 1024; // 512 KiB (T-07-02-01 + T-07-02-04)

#[derive(Clone)]
pub struct OrgDbService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub clock: Arc<dyn Clock + Send + Sync>,
    pub paths: Arc<Paths>,
}

impl OrgDbService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
        paths: Arc<Paths>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            paths,
        }
    }

    /// Читает org_settings WHERE id=1 и возвращает `OrgSettingsDto`.
    pub async fn get(&self) -> Result<OrgSettingsDto, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<OrgSettingsDto, AppError> {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT org_name, inn, kpp, address, \
                 (logo_blob IS NOT NULL) as has_logo, \
                 phone, fax, email, okpo, ogrn \
                 FROM org_settings WHERE id = 1",
                [],
                |r| {
                    Ok(OrgSettingsDto {
                        org_name: r.get(0)?,
                        inn: r.get(1)?,
                        kpp: r.get(2)?,
                        address: r.get(3)?,
                        has_logo: r.get::<_, bool>(4)?,
                        phone: r.get(5)?,
                        fax: r.get(6)?,
                        email: r.get(7)?,
                        okpo: r.get(8)?,
                        ogrn: r.get(9)?,
                    })
                },
            )
            .map_err(map_rusqlite)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking OrgDbService::get: {e}"),
        })?
    }

    /// Сохраняет текстовые поля организации.
    pub async fn save_fields(
        &self,
        caller: &Identity,
        patch: crate::dto::reports::OrgPatch,
    ) -> Result<(), AppError> {
        authorize(caller, &Action::ManageSettings)?;
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE org_settings \
                     SET org_name=?2, inn=?3, kpp=?4, address=?5, \
                         phone=?6, fax=?7, email=?8, okpo=?9, ogrn=?10, \
                         updated_at_utc=?11, version=version+1 \
                     WHERE id=1",
                    params![
                        1i64,
                        patch.org_name,
                        patch.inn,
                        patch.kpp,
                        patch.address,
                        patch.phone,
                        patch.fax,
                        patch.email,
                        patch.okpo,
                        patch.ogrn,
                        now
                    ],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
    }

    /// Загружает лого-BLOB в org_settings.
    ///
    /// Limits: 512 KiB; mime из allowlist (T-07-02-01).
    pub async fn save_logo(
        &self,
        caller: &Identity,
        logo_bytes: Vec<u8>,
        logo_mime: String,
    ) -> Result<(), AppError> {
        if logo_bytes.len() > LOGO_MAX_BYTES {
            return Err(AppError::Validation {
                field: "logo".to_string(),
                message: format!(
                    "Логотип слишком большой: {} байт (максимум {} КБ)",
                    logo_bytes.len(),
                    LOGO_MAX_BYTES / 1024
                ),
            });
        }
        let mime_lower = logo_mime.to_lowercase();
        if !matches!(
            mime_lower.as_str(),
            "image/png" | "image/jpeg" | "image/svg+xml"
        ) {
            return Err(AppError::Validation {
                field: "logo_mime".to_string(),
                message: format!(
                    "Неподдерживаемый тип файла: {logo_mime}. \
                     Разрешены: image/png, image/jpeg, image/svg+xml"
                ),
            });
        }
        authorize(caller, &Action::ManageSettings)?;
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE org_settings \
                     SET logo_blob=?2, logo_mime=?3, \
                         updated_at_utc=?4, version=version+1 \
                     WHERE id=1",
                    params![1i64, logo_bytes, logo_mime, now],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
    }

    /// Удаляет логотип из БД.
    pub async fn remove_logo(&self, caller: &Identity) -> Result<(), AppError> {
        authorize(caller, &Action::ManageSettings)?;
        let now = self.clock.unix_seconds();
        self.writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE org_settings \
                     SET logo_blob=NULL, logo_mime=NULL, \
                         updated_at_utc=?2, version=version+1 \
                     WHERE id=1",
                    params![1i64, now],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
    }

    /// Возвращает сырые байты лого или None если лого нет.
    pub async fn get_logo_bytes(&self) -> Result<Option<Vec<u8>>, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<Option<Vec<u8>>, AppError> {
            let conn = readers.acquire();
            let result: rusqlite::Result<Option<Vec<u8>>> =
                conn.query_row("SELECT logo_blob FROM org_settings WHERE id = 1", [], |r| {
                    r.get(0)
                });
            result.map_err(map_rusqlite)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking OrgDbService::get_logo_bytes: {e}"),
        })?
    }

    /// Стартовый хук: если `org.json` существует И в org_settings ещё стоит
    /// placeholder (`org_name='Ваша организация'`), читает `org.json` и пишет
    /// поля в БД. Переименовывает `org.json` → `org.json.migrated`.
    ///
    /// Никогда не паникует — ошибки логируются в warn и пропускаются.
    pub async fn migrate_from_org_json(&self) {
        let org_json_path = self.paths.exe_dir().join("org.json");
        if !org_json_path.exists() {
            return;
        }

        // Проверяем, нужна ли миграция (placeholder в org_name)
        let needs_migration = match self.get().await {
            Ok(dto) => dto.org_name == "Ваша организация",
            Err(e) => {
                tracing::warn!("OrgDbService::migrate_from_org_json: get() failed: {e}");
                return;
            }
        };
        if !needs_migration {
            return;
        }

        let org_json_path_clone = org_json_path.clone();
        let paths_exe_dir = self.paths.exe_dir().to_path_buf();

        // Читаем org.json в spawn_blocking
        let org_data: Option<OrgData> = tokio::task::spawn_blocking(move || {
            let raw = match std::fs::read_to_string(&org_json_path_clone) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("migrate_from_org_json: read org.json failed: {e}");
                    return None;
                }
            };
            match serde_json::from_str::<OrgData>(&raw) {
                Ok(d) => Some(d),
                Err(e) => {
                    tracing::warn!("migrate_from_org_json: parse org.json failed: {e}");
                    None
                }
            }
        })
        .await
        .unwrap_or(None);

        let Some(data) = org_data else {
            return;
        };

        // Пытаемся загрузить логотип если есть путь
        let logo_bytes_opt: Option<(Vec<u8>, String)> = if !data.logo_path.trim().is_empty() {
            let logo_abs = paths_exe_dir.join(&data.logo_path);
            tokio::task::spawn_blocking(move || -> Option<(Vec<u8>, String)> {
                if !logo_abs.exists() {
                    tracing::warn!(
                        "migrate_from_org_json: logo file not found: {}",
                        logo_abs.display()
                    );
                    return None;
                }
                let bytes = match std::fs::read(&logo_abs) {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::warn!("migrate_from_org_json: read logo failed: {e}");
                        return None;
                    }
                };
                if bytes.len() > LOGO_MAX_BYTES {
                    tracing::warn!(
                        "migrate_from_org_json: logo too large ({} bytes), skipping",
                        bytes.len()
                    );
                    return None;
                }
                // Угадываем mime по расширению
                let lower = logo_abs
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();
                let mime = match lower.as_str() {
                    "png" => "image/png",
                    "jpg" | "jpeg" => "image/jpeg",
                    "svg" => "image/svg+xml",
                    _ => {
                        tracing::warn!(
                            "migrate_from_org_json: unknown logo extension .{lower}, skipping"
                        );
                        return None;
                    }
                };
                Some((bytes, mime.to_string()))
            })
            .await
            .unwrap_or(None)
        } else {
            None
        };

        // Записываем в БД (без авторизации — это системный хук запуска)
        let now = self.clock.unix_seconds();
        let org_name = data.name.clone();
        let inn = data.inn.clone();
        let kpp = data.kpp.clone();
        let address = data.address.clone();

        let write_result = self
            .writer
            .execute(move |conn| {
                conn.execute(
                    "UPDATE org_settings \
                     SET org_name=?2, inn=?3, kpp=?4, address=?5, \
                         updated_at_utc=?6, version=version+1 \
                     WHERE id=1",
                    params![1i64, org_name, inn, kpp, address, now],
                )
                .map(|_| ())
                .map_err(map_rusqlite)?;

                if let Some((logo_bytes, logo_mime)) = logo_bytes_opt {
                    conn.execute(
                        "UPDATE org_settings \
                         SET logo_blob=?2, logo_mime=?3 \
                         WHERE id=1",
                        params![1i64, logo_bytes, logo_mime],
                    )
                    .map(|_| ())
                    .map_err(map_rusqlite)?;
                }
                Ok(())
            })
            .await;

        if let Err(e) = write_result {
            tracing::warn!("migrate_from_org_json: DB write failed: {e}");
            return;
        }

        // Переименовываем org.json → org.json.migrated
        let migrated_path = org_json_path.with_extension("json.migrated");
        let rename_result =
            tokio::task::spawn_blocking(move || std::fs::rename(&org_json_path, &migrated_path))
                .await;

        match rename_result {
            Ok(Ok(())) => {
                tracing::info!("migrate_from_org_json: org.json migrated to DB successfully")
            }
            Ok(Err(e)) => tracing::warn!(
                "migrate_from_org_json: rename org.json failed (data was migrated): {e}"
            ),
            Err(e) => tracing::warn!("migrate_from_org_json: spawn_blocking rename failed: {e}"),
        }
    }

    /// Читает org_settings для PDF-рендеринга: возвращает `(OrgSettingsDto, Option<Vec<u8>>)`
    /// для передачи в `HeaderBlock`.
    pub async fn get_for_pdf(
        &self,
    ) -> Result<(OrgSettingsDto, Option<Vec<u8>>, Option<String>), AppError> {
        type PdfTuple = (OrgSettingsDto, Option<Vec<u8>>, Option<String>);
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<PdfTuple, AppError> {
            let conn = readers.acquire();
            conn.query_row(
                "SELECT org_name, inn, kpp, address, \
                 (logo_blob IS NOT NULL) as has_logo, \
                 logo_blob, logo_mime, \
                 phone, fax, email, okpo, ogrn \
                 FROM org_settings WHERE id = 1",
                [],
                |r| {
                    let dto = OrgSettingsDto {
                        org_name: r.get(0)?,
                        inn: r.get(1)?,
                        kpp: r.get(2)?,
                        address: r.get(3)?,
                        has_logo: r.get::<_, bool>(4)?,
                        phone: r.get(7)?,
                        fax: r.get(8)?,
                        email: r.get(9)?,
                        okpo: r.get(10)?,
                        ogrn: r.get(11)?,
                    };
                    let logo_blob: Option<Vec<u8>> = r.get(5)?;
                    let logo_mime: Option<String> = r.get(6)?;
                    Ok((dto, logo_blob, logo_mime))
                },
            )
            .map_err(map_rusqlite)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking OrgDbService::get_for_pdf: {e}"),
        })?
    }

    /// Читает путь к логотипу из org_settings (для backward compat с PathBuf-логикой).
    /// Возвращает None — в новой схеме лого хранится как BLOB, не как путь.
    pub fn get_logo_path(&self) -> Option<PathBuf> {
        None
    }
}
