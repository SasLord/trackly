//! `OrganizationService` — чтение `org.json` рядом с .exe + path-traversal
//! mitigation для `logo_path`.
//!
//! Portable-mode discipline (D-OrgData-01): файл `org.json` лежит в
//! `paths.exe_dir()`. На первом запуске service не находит файл, создаёт
//! placeholder с tracing::warn и возвращает дефолтные значения, чтобы UI
//! не показывал error-state — пользователь увидит «Ваша организация» в
//! шапке PDF и сможет отредактировать `org.json` в Phase 7.
//!
//! Security (T-03-04-01): `logo_path` поле — это потенциальный
//! path-traversal vector (admin отредактирует `org.json` и поставит
//! `../../etc/passwd`). `safe_logo_canonical` canonicalize-ит путь и
//! отвергает всё, что не starts_with(`paths.exe_dir().canonicalize()`).

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use trackly_core::error::AppError;
use trackly_infra::Paths;

/// Данные организации — то, что попадает в шапку любого печатного документа.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrgData {
    pub name: String,
    pub inn: String,
    pub kpp: String,
    pub address: String,
    /// Путь к файлу логотипа, относительный к `paths.exe_dir()`. Может быть
    /// пустой строкой — тогда `safe_logo_canonical` вернёт `Ok(None)`.
    pub logo_path: String,
}

impl OrgData {
    /// Placeholder, который создаётся на первом запуске если `org.json` отсутствует.
    pub fn placeholder() -> Self {
        Self {
            name: "Ваша организация".to_string(),
            inn: "0000000000".to_string(),
            kpp: "000000000".to_string(),
            address: "Адрес не указан".to_string(),
            logo_path: String::new(),
        }
    }
}

#[derive(Clone)]
pub struct OrganizationService {
    pub paths: Arc<Paths>,
}

impl OrganizationService {
    pub fn new(paths: Arc<Paths>) -> Self {
        Self { paths }
    }

    /// `<exe_dir>/org.json`.
    pub fn file_path(&self) -> PathBuf {
        self.paths.exe_dir().join("org.json")
    }

    /// Читает `org.json`. Если файла нет — создаёт placeholder и возвращает его.
    ///
    /// Не кэширует (Phase 7 добавит file-watcher). Каждый вызов делает
    /// `fs::read` под `spawn_blocking`.
    pub async fn read(&self) -> Result<OrgData, AppError> {
        let path = self.file_path();
        tokio::task::spawn_blocking(move || -> Result<OrgData, AppError> {
            if !path.exists() {
                tracing::warn!(
                    "org.json не найден по пути {} — создаю placeholder",
                    path.display()
                );
                let placeholder = OrgData::placeholder();
                let json =
                    serde_json::to_string_pretty(&placeholder).map_err(|e| AppError::Internal {
                        source_chain: format!("serialize org placeholder: {e}"),
                    })?;
                std::fs::write(&path, json).map_err(|e| AppError::Internal {
                    source_chain: format!("write org.json placeholder: {e}"),
                })?;
                return Ok(placeholder);
            }
            let raw = std::fs::read_to_string(&path).map_err(|e| AppError::Internal {
                source_chain: format!("read org.json: {e}"),
            })?;
            serde_json::from_str::<OrgData>(&raw).map_err(|e| AppError::Validation {
                field: "org.json".to_string(),
                message: format!("Невалидный JSON в org.json: {e}"),
            })
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking read org.json: {e}"),
        })?
    }

    /// Сырой абсолютный путь к лого (БЕЗ canonicalize + traversal check).
    /// Используется только внутри `safe_logo_canonical`.
    pub fn logo_abs_path(&self, org: &OrgData) -> PathBuf {
        self.paths.exe_dir().join(&org.logo_path)
    }

    /// Возвращает canonicalized путь к лого, если он существует И находится
    /// внутри `paths.exe_dir()` (T-03-04-01 mitigation).
    ///
    /// - `Ok(None)` — `logo_path` пуст ИЛИ файла нет (warning в лог, не ошибка).
    /// - `Ok(Some(path))` — canonical absolute path, гарантированно внутри exe_dir.
    /// - `Err(Validation { field: "org.logo_path", ... })` — попытка path traversal.
    pub async fn safe_logo_canonical(&self, org: &OrgData) -> Result<Option<PathBuf>, AppError> {
        if org.logo_path.trim().is_empty() {
            return Ok(None);
        }
        let logo_abs = self.logo_abs_path(org);
        let exe_dir = self.paths.exe_dir().to_path_buf();
        tokio::task::spawn_blocking(move || -> Result<Option<PathBuf>, AppError> {
            if !logo_abs.exists() {
                tracing::warn!(
                    "Файл логотипа не найден: {} — PDF будет без лого",
                    logo_abs.display()
                );
                return Ok(None);
            }
            let canonical = logo_abs.canonicalize().map_err(|e| AppError::Internal {
                source_chain: format!("canonicalize logo path {}: {e}", logo_abs.display()),
            })?;
            let exe_canonical = exe_dir.canonicalize().map_err(|e| AppError::Internal {
                source_chain: format!("canonicalize exe_dir {}: {e}", exe_dir.display()),
            })?;
            if !canonical.starts_with(&exe_canonical) {
                return Err(AppError::Validation {
                    field: "org.logo_path".to_string(),
                    message: format!("Путь к логотипу вне рабочей папки: {}", canonical.display()),
                });
            }
            Ok(Some(canonical))
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking safe_logo_canonical: {e}"),
        })?
    }

    /// Читает байты логотипа из legacy `org.json`-пути (для embedding как
    /// `data:` URI в HTML act-шаблонах — Phase 16 D-11).
    ///
    /// Использует уже проверенный `safe_logo_canonical` (path-traversal guard,
    /// T-03-04-01) — читает файл только если он canonical и внутри
    /// `paths.exe_dir()`. MIME выводится из расширения файла.
    ///
    /// - `Ok(None)` — логотип не настроен ИЛИ файл отсутствует.
    /// - `Ok(Some((bytes, mime)))` — содержимое файла + MIME-тип.
    pub async fn read_logo_bytes(
        &self,
        org: &OrgData,
    ) -> Result<Option<(Vec<u8>, String)>, AppError> {
        let canonical = match self.safe_logo_canonical(org).await? {
            Some(path) => path,
            None => return Ok(None),
        };
        let mime = match canonical
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_ascii_lowercase())
        {
            Some(ext) if ext == "png" => "image/png",
            Some(ext) if ext == "jpg" || ext == "jpeg" => "image/jpeg",
            _ => "image/png",
        }
        .to_string();
        let bytes = tokio::fs::read(&canonical)
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("read_logo_bytes: read {} failed: {e}", canonical.display()),
            })?;
        Ok(Some((bytes, mime)))
    }
}
