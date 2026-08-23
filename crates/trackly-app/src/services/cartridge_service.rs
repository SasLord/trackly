//! `CartridgeService` — application service for cartridge lifecycle.
//!
//! Single-writer discipline: every mutation goes through
//! `WriterHandle::execute(closure)` with a `BEGIN IMMEDIATE` transaction.
//! `counters.cartridge_seq` is incremented atomically via
//! `assign_code_in_tx` (retry loop — D-Code-01).
//!
//! Validation rules (T-04-03-01):
//!   - model_id == 0 → Validation
//!   - code_override trimmed empty → Validation
//!   - code_override trimmed > 32 chars → Validation
//!   - code_override contains char < U+0020 (control chars) → Validation

use std::sync::Arc;

use rusqlite::{params, OptionalExtension};
use trackly_core::domain::cartridges::CartridgeModelNew;
use trackly_core::error::AppError;
use trackly_core::ports::cartridges::CartridgeRepository;
use trackly_core::primitives::clock::Clock;
use trackly_infra::db::{pools::ReaderPool, writer_worker::WriterHandle};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::audit_log_sqlite::AuditEntry;
use trackly_infra::repos::{
    SqliteAuditLogRepository, SqliteCartridgeRepository, SqliteDeviceRepository,
    SqlitePlaceRepository,
};

use crate::dto::cartridge::{
    AuditEntryDto, CartridgeCountsDto, CartridgeCreateDto, CartridgeDto, CartridgeFilter,
    CartridgeListResponse, CartridgeModelCreateDto, CartridgeModelDto, CartridgeModelPatchDto,
    CartridgeTransitionPayload, LowStockItemDto, Pagination,
};

/// Application service for cartridge lifecycle. `Arc`-fields keep `Clone` O(1).
#[derive(Clone)]
pub struct CartridgeService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) cart_repo: Arc<SqliteCartridgeRepository>,
    pub(crate) audit_repo: Arc<SqliteAuditLogRepository>,
    /// Install's printer-derived place default (D-13) — reads `devices.place_id`
    /// for the target printer, never client-supplied text.
    pub(crate) device_repo: Arc<SqliteDeviceRepository>,
    /// D-11.4 storage-place listing for the ReturnToStock suggestion UX.
    pub(crate) place_repo: Arc<SqlitePlaceRepository>,
}

impl CartridgeService {
    pub fn new(
        writer: Arc<WriterHandle>,
        readers: Arc<ReaderPool>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            writer,
            readers,
            clock,
            cart_repo: Arc::new(SqliteCartridgeRepository),
            audit_repo: Arc::new(SqliteAuditLogRepository),
            device_repo: Arc::new(SqliteDeviceRepository),
            place_repo: Arc::new(SqlitePlaceRepository),
        }
    }

    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------

    /// Validate a create payload before writing (T-04-03-01).
    ///
    /// Rules checked in order:
    ///   1. `model_id == 0` → field "model_id"
    ///   2. `code_override == Some(s)`:
    ///      a. `s.trim().is_empty()` → field "code_override"
    ///      b. `s.trim().chars().count() > 32` → field "code_override"
    ///      c. any char < U+0020 → field "code_override"
    fn validate_create(p: &CartridgeCreateDto) -> Result<(), AppError> {
        if p.model_id == 0 {
            return Err(AppError::Validation {
                field: "model_id".into(),
                message: "Выберите модель картриджа".into(),
            });
        }
        if let Some(ref code) = p.code_override {
            let trimmed = code.trim();
            if trimmed.is_empty() {
                return Err(AppError::Validation {
                    field: "code_override".into(),
                    message: "Код не может быть пустым".into(),
                });
            }
            if trimmed.chars().count() > 32 {
                return Err(AppError::Validation {
                    field: "code_override".into(),
                    message: "Код не должен превышать 32 символа".into(),
                });
            }
            if code.chars().any(|c| (c as u32) < 0x20) {
                return Err(AppError::Validation {
                    field: "code_override".into(),
                    message: "Код содержит недопустимые символы".into(),
                });
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Create
    // -----------------------------------------------------------------------

    pub async fn create(&self, payload: CartridgeCreateDto) -> Result<CartridgeDto, AppError> {
        Self::validate_create(&payload)?;
        let now = self.clock.unix_seconds();
        let cart_repo = self.cart_repo.clone();
        let audit_repo = self.audit_repo.clone();

        let cart_id = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // Вид расходника берём из модели — он определяет префикс кода
                // (C- картридж / D- фотобарабан).
                let kind_id = SqliteCartridgeRepository::model_kind_in_tx(&tx, payload.model_id)?;

                // Assign code (auto or custom).
                let (code, was_auto) = SqliteCartridgeRepository::assign_code_in_tx(
                    &tx,
                    payload.code_override.as_deref(),
                    kind_id,
                    now,
                )?;

                // Insert cartridge row (initial status_id = 1 = На складе).
                let cart_id = cart_repo.insert_cartridge_in_tx(
                    &tx,
                    &code,
                    payload.model_id,
                    1, // status_id: На складе
                    payload.state_id,
                    payload.place_id,
                    None, // holder_name — empty on creation
                    payload.notes.as_deref(),
                    now,
                )?;

                // Audit: create or custom:code_override
                let action = if was_auto {
                    "create"
                } else {
                    "custom:cartridge_code_override"
                };
                let payload_json = serde_json::json!({
                    "code": &code,
                    "model_id": payload.model_id,
                })
                .to_string();
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "cartridge",
                        entity_id: cart_id,
                        action,
                        user_id: None,
                        before_json: None,
                        after_json: None,
                        payload_json: Some(payload_json),
                        created_at_utc: now,
                    },
                )?;

                tx.commit().map_err(map_rusqlite)?;
                Ok(cart_id)
            })
            .await?;

        self.get(cart_id).await
    }

    // -----------------------------------------------------------------------
    // Update (fields only — status changes use transition)
    // -----------------------------------------------------------------------

    pub async fn update(
        &self,
        id: i64,
        version: i64,
        place_id: Option<i64>,
        notes: Option<String>,
    ) -> Result<CartridgeDto, AppError> {
        let now = self.clock.unix_seconds();
        let audit_repo = self.audit_repo.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                let affected = tx
                    .execute(
                        "UPDATE cartridges SET place_id=?1, notes=?2, \
                         updated_at_utc=?3, version=version+1 \
                         WHERE id=?4 AND version=?5 AND deleted_at_utc IS NULL",
                        params![place_id, notes, now, id, version],
                    )
                    .map_err(map_rusqlite)?;

                if affected == 0 {
                    let actual: Option<i64> = tx
                        .query_row(
                            "SELECT version FROM cartridges WHERE id = ?1",
                            params![id],
                            |r| r.get(0),
                        )
                        .optional()
                        .map_err(map_rusqlite)?;
                    return match actual {
                        None => Err(AppError::NotFound {
                            entity: "cartridge",
                            id,
                        }),
                        Some(actual) => Err(AppError::OptimisticLockMismatch {
                            entity: "cartridge",
                            id,
                            expected: version,
                            actual,
                        }),
                    };
                }

                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "cartridge",
                        entity_id: id,
                        action: "update",
                        user_id: None,
                        before_json: None,
                        after_json: None,
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;

                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await?;

        self.get(id).await
    }

    // -----------------------------------------------------------------------
    // Delete (soft)
    // -----------------------------------------------------------------------

    pub async fn delete(&self, id: i64, version: i64) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        let audit_repo = self.audit_repo.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                let affected = tx
                    .execute(
                        "UPDATE cartridges SET deleted_at_utc=?1, updated_at_utc=?1, \
                         version=version+1 \
                         WHERE id=?2 AND version=?3 AND deleted_at_utc IS NULL",
                        params![now, id, version],
                    )
                    .map_err(map_rusqlite)?;

                if affected == 0 {
                    let actual: Option<i64> = tx
                        .query_row(
                            "SELECT version FROM cartridges WHERE id = ?1",
                            params![id],
                            |r| r.get(0),
                        )
                        .optional()
                        .map_err(map_rusqlite)?;
                    return match actual {
                        None => Err(AppError::NotFound {
                            entity: "cartridge",
                            id,
                        }),
                        Some(actual) => Err(AppError::OptimisticLockMismatch {
                            entity: "cartridge",
                            id,
                            expected: version,
                            actual,
                        }),
                    };
                }

                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "cartridge",
                        entity_id: id,
                        action: "delete",
                        user_id: None,
                        before_json: None,
                        after_json: None,
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;

                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await
    }

    // -----------------------------------------------------------------------
    // Read
    // -----------------------------------------------------------------------

    pub async fn get(&self, id: i64) -> Result<CartridgeDto, AppError> {
        let readers = self.readers.clone();
        let repo = self.cart_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            let row = repo.get(&conn, id)?;
            Ok(CartridgeDto::from(row))
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    pub async fn list(
        &self,
        filter: CartridgeFilter,
        pagination: Pagination,
    ) -> Result<CartridgeListResponse, AppError> {
        if pagination.limit > 200 {
            return Err(AppError::Validation {
                field: "pagination.limit".into(),
                message: "Максимум 200 элементов на страницу".into(),
            });
        }
        let domain_filter = filter.into_domain();
        let domain_page = pagination.into();
        let readers = self.readers.clone();
        let repo = self.cart_repo.clone();
        let (rows, total) =
            tokio::task::spawn_blocking(move || -> Result<(Vec<CartridgeDto>, u64), AppError> {
                let conn = readers.acquire();
                let (rows, total) = repo.list(&conn, &domain_filter, &domain_page)?;
                Ok((rows.into_iter().map(CartridgeDto::from).collect(), total))
            })
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("spawn_blocking: {e}"),
            })??;
        Ok(CartridgeListResponse { items: rows, total })
    }

    pub async fn status_counts(&self) -> Result<CartridgeCountsDto, AppError> {
        let readers = self.readers.clone();
        let repo = self.cart_repo.clone();
        let counts = tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.counts(&conn)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })??;
        Ok(counts.into())
    }

    // -----------------------------------------------------------------------
    // Transition (lifecycle mutation)
    // -----------------------------------------------------------------------

    pub async fn transition(
        &self,
        mut payload: CartridgeTransitionPayload,
    ) -> Result<CartridgeDto, AppError> {
        // D-13: Install with no explicit place_id defaults from the target
        // printer's own place_id — a NEW server-computed lookup (never
        // client-supplied text). If the device lookup misses or has no
        // place_id, leave place_id as None (D-07 — place stays optional).
        if let CartridgeTransitionPayload::Install {
            place_id: None,
            printer_device_id: Some(pid),
            ..
        } = &payload
        {
            let pid = *pid;
            let readers = self.readers.clone();
            let device_repo = self.device_repo.clone();
            let resolved: Option<i64> =
                tokio::task::spawn_blocking(move || -> Result<Option<i64>, AppError> {
                    use trackly_core::ports::devices::DeviceRepository;
                    let conn = readers.acquire();
                    match device_repo.get(&conn, pid) {
                        Ok(row) => Ok(row.place_id),
                        Err(AppError::NotFound { .. }) => Ok(None),
                        Err(e) => Err(e),
                    }
                })
                .await
                .map_err(|e| AppError::Internal {
                    source_chain: format!("spawn_blocking: {e}"),
                })??;

            if let CartridgeTransitionPayload::Install { place_id, .. } = &mut payload {
                *place_id = resolved;
            }
        }

        let cartridge_id = payload.cartridge_id();
        let version = payload.version();
        let op: trackly_core::domain::cartridges::CartridgeTransitionOp = payload.into();
        let now = self.clock.unix_seconds();
        let cart_repo = self.cart_repo.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                cart_repo.transition_in_tx(&tx, cartridge_id, version, &op, now)?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await?;

        self.get(cartridge_id).await
    }

    // -----------------------------------------------------------------------
    // Search
    // -----------------------------------------------------------------------

    pub async fn search(
        &self,
        query: String,
        filter: CartridgeFilter,
    ) -> Result<CartridgeListResponse, AppError> {
        let trimmed = query.trim().to_string();
        if trimmed.is_empty() {
            return self.list(filter, Pagination::default()).await;
        }
        let domain_filter = filter.into_domain();
        let readers = self.readers.clone();
        let repo = self.cart_repo.clone();
        let rows = tokio::task::spawn_blocking(move || -> Result<Vec<CartridgeDto>, AppError> {
            let conn = readers.acquire();
            let rows = repo.search(&conn, &trimmed, &domain_filter)?;
            Ok(rows.into_iter().map(CartridgeDto::from).collect())
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })??;
        let total = rows.len() as u64;
        Ok(CartridgeListResponse { items: rows, total })
    }

    // -----------------------------------------------------------------------
    // History
    // -----------------------------------------------------------------------

    pub async fn get_history(&self, cartridge_id: i64) -> Result<Vec<AuditEntryDto>, AppError> {
        let readers = self.readers.clone();
        let repo = self.cart_repo.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<AuditEntryDto>, AppError> {
            let conn = readers.acquire();
            let rows = repo.get_history(&conn, cartridge_id)?;
            Ok(rows
                .into_iter()
                .map(|r| AuditEntryDto {
                    id: r.id,
                    action: r.action,
                    payload_json: r.payload_json,
                    before_json: r.before_json,
                    after_json: r.after_json,
                    created_at_utc: r.created_at_utc,
                })
                .collect())
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    // -----------------------------------------------------------------------
    // Low stock
    // -----------------------------------------------------------------------

    pub async fn low_stock(&self) -> Result<Vec<LowStockItemDto>, AppError> {
        let readers = self.readers.clone();
        let repo = self.cart_repo.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<LowStockItemDto>, AppError> {
            let conn = readers.acquire();
            let items = repo.low_stock(&conn)?;
            Ok(items.into_iter().map(LowStockItemDto::from).collect())
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    // -----------------------------------------------------------------------
    // Printer-card compatible-models aggregate (R4, Phase 13)
    // -----------------------------------------------------------------------

    /// Aggregate counts (by status) for every cartridge model compatible with
    /// `printer_device_id` — backs `printers_get_compatible_aggregates`
    /// (both transports). Lives on `CartridgeService` because the underlying
    /// query lives in `cartridges_sqlite.rs` (Plan 13-01); printers.rs's
    /// `build_*` helper calls through `ctx.cartridges`.
    pub async fn compatible_aggregates_for_printer(
        &self,
        printer_device_id: i64,
    ) -> Result<Vec<trackly_core::domain::cartridges::CompatibleModelAggregate>, AppError> {
        let readers = self.readers.clone();
        let repo = self.cart_repo.clone();
        tokio::task::spawn_blocking(move || {
            let conn = readers.acquire();
            repo.compatible_model_aggregates(&conn, printer_device_id)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    // -----------------------------------------------------------------------
    // Cartridge models CRUD
    // -----------------------------------------------------------------------

    pub async fn model_list(&self) -> Result<Vec<CartridgeModelDto>, AppError> {
        let readers = self.readers.clone();
        let repo = self.cart_repo.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<CartridgeModelDto>, AppError> {
            let conn = readers.acquire();
            let rows = repo.list_models(&conn)?;
            let counts = repo.count_instances_by_model(&conn)?;
            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                let compat = repo.get_compatibility(&conn, row.id)?;
                let instances = counts.get(&row.id).copied().unwrap_or(0);
                out.push(CartridgeModelDto::from_row(row, compat).with_instance_count(instances));
            }
            Ok(out)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    pub async fn model_get(&self, id: i64) -> Result<CartridgeModelDto, AppError> {
        let readers = self.readers.clone();
        let repo = self.cart_repo.clone();
        tokio::task::spawn_blocking(move || -> Result<CartridgeModelDto, AppError> {
            let conn = readers.acquire();
            let row = repo.get_model(&conn, id)?;
            let compat = repo.get_compatibility(&conn, row.id)?;
            let instances = repo
                .count_instances_by_model(&conn)?
                .get(&id)
                .copied()
                .unwrap_or(0);
            Ok(CartridgeModelDto::from_row(row, compat).with_instance_count(instances))
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    pub async fn model_create(
        &self,
        payload: CartridgeModelCreateDto,
    ) -> Result<CartridgeModelDto, AppError> {
        if payload.brand.trim().is_empty() {
            return Err(AppError::Validation {
                field: "brand".into(),
                message: "Укажите бренд модели".into(),
            });
        }
        if payload.model.trim().is_empty() {
            return Err(AppError::Validation {
                field: "model".into(),
                message: "Укажите название модели".into(),
            });
        }
        let now = self.clock.unix_seconds();
        let cart_repo = self.cart_repo.clone();
        let audit_repo = self.audit_repo.clone();

        let model_id = self
            .writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // Pre-check for duplicate (brand, model) to return a Russian
                // conflict reason instead of leaking the raw SQLite UNIQUE error
                // to the UI (WR-02).
                let exists: bool = tx
                    .query_row(
                        "SELECT EXISTS( \
                             SELECT 1 FROM cartridge_models \
                              WHERE brand = ?1 AND model = ?2 \
                                AND deleted_at_utc IS NULL LIMIT 1)",
                        params![payload.brand.trim(), payload.model.trim()],
                        |r| r.get(0),
                    )
                    .map_err(map_rusqlite)?;
                if exists {
                    return Err(AppError::Conflict {
                        reason: format!(
                            "Модель «{} {}» уже существует",
                            payload.brand.trim(),
                            payload.model.trim()
                        ),
                    });
                }

                let new = CartridgeModelNew {
                    brand: payload.brand.clone(),
                    model: payload.model.clone(),
                    kind_id: payload.kind_id,
                    color: payload.color.clone(),
                    notes: payload.notes.clone(),
                };
                let model_id = cart_repo.insert_model_in_tx(&tx, &new, now)?;
                cart_repo.upsert_compatibility_in_tx(&tx, model_id, &payload.compatibility)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "cartridge_model",
                        entity_id: model_id,
                        action: "create",
                        user_id: None,
                        before_json: None,
                        after_json: None,
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(model_id)
            })
            .await?;

        self.model_get(model_id).await
    }

    pub async fn model_update(
        &self,
        payload: CartridgeModelPatchDto,
    ) -> Result<CartridgeModelDto, AppError> {
        let now = self.clock.unix_seconds();
        let cart_repo = self.cart_repo.clone();
        let audit_repo = self.audit_repo.clone();
        let model_id = payload.id;

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;

                // Pre-check for duplicate (brand, model) excluding the current
                // row to return a Russian conflict reason (WR-02).
                let conflict: bool = tx
                    .query_row(
                        "SELECT EXISTS( \
                             SELECT 1 FROM cartridge_models \
                              WHERE brand = ?1 AND model = ?2 \
                                AND id != ?3 \
                                AND deleted_at_utc IS NULL LIMIT 1)",
                        params![payload.brand.trim(), payload.model.trim(), payload.id],
                        |r| r.get(0),
                    )
                    .map_err(map_rusqlite)?;
                if conflict {
                    return Err(AppError::Conflict {
                        reason: format!(
                            "Модель «{} {}» уже существует",
                            payload.brand.trim(),
                            payload.model.trim()
                        ),
                    });
                }

                cart_repo.update_model_in_tx(
                    &tx,
                    payload.id,
                    payload.version,
                    &payload.brand,
                    &payload.model,
                    payload.kind_id,
                    payload.color.as_deref(),
                    payload.notes.as_deref(),
                    now,
                )?;
                cart_repo.upsert_compatibility_in_tx(&tx, payload.id, &payload.compatibility)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "cartridge_model",
                        entity_id: payload.id,
                        action: "update",
                        user_id: None,
                        before_json: None,
                        after_json: None,
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await?;

        self.model_get(model_id).await
    }

    pub async fn model_delete(&self, id: i64, version: i64) -> Result<(), AppError> {
        // Guard: count live instances first (T-04-03-03).
        let live_count = {
            let readers = self.readers.clone();
            tokio::task::spawn_blocking(move || -> Result<i64, AppError> {
                let conn = readers.acquire();
                conn.query_row(
                    "SELECT COUNT(*) FROM cartridges \
                      WHERE model_id = ?1 AND deleted_at_utc IS NULL",
                    params![id],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)
            })
            .await
            .map_err(|e| AppError::Internal {
                source_chain: format!("spawn_blocking: {e}"),
            })??
        };

        if live_count > 0 {
            return Err(AppError::Conflict {
                reason: format!(
                    "Нельзя удалить модель: она используется {} картриджами",
                    live_count
                ),
            });
        }

        let now = self.clock.unix_seconds();
        let cart_repo = self.cart_repo.clone();
        let audit_repo = self.audit_repo.clone();

        self.writer
            .execute(move |conn| {
                let tx = conn.transaction().map_err(map_rusqlite)?;
                cart_repo.soft_delete_model_in_tx(&tx, id, version, now)?;
                audit_repo.insert(
                    &tx,
                    AuditEntry {
                        entity_type: "cartridge_model",
                        entity_id: id,
                        action: "delete",
                        user_id: None,
                        before_json: None,
                        after_json: None,
                        payload_json: None,
                        created_at_utc: now,
                    },
                )?;
                tx.commit().map_err(map_rusqlite)?;
                Ok(())
            })
            .await
    }

    // -----------------------------------------------------------------------
    // Autocomplete / suggest helpers
    // -----------------------------------------------------------------------

    /// DISTINCT brand FROM cartridge_models WHERE brand LIKE ?% ORDER BY brand LIMIT 20.
    pub async fn suggest_brand(&self, prefix: String) -> Result<Vec<String>, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<String>, AppError> {
            let conn = readers.acquire();
            let pattern = format!("{}%", prefix);
            let mut stmt = conn
                .prepare(
                    "SELECT DISTINCT brand FROM cartridge_models \
                      WHERE brand LIKE ?1 AND deleted_at_utc IS NULL \
                      ORDER BY brand ASC LIMIT 20",
                )
                .map_err(map_rusqlite)?;
            let rows = stmt
                .query_map(params![pattern], |r| r.get::<_, String>(0))
                .map_err(map_rusqlite)?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(map_rusqlite)?);
            }
            Ok(out)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    /// DISTINCT model WHERE brand=? AND model LIKE ?%.
    pub async fn suggest_model(
        &self,
        brand: String,
        prefix: String,
    ) -> Result<Vec<String>, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<String>, AppError> {
            let conn = readers.acquire();
            let pattern = format!("{}%", prefix);
            let mut stmt = conn
                .prepare(
                    "SELECT DISTINCT model FROM cartridge_models \
                      WHERE brand = ?1 AND model LIKE ?2 AND deleted_at_utc IS NULL \
                      ORDER BY model ASC LIMIT 20",
                )
                .map_err(map_rusqlite)?;
            let rows = stmt
                .query_map(params![brand, pattern], |r| r.get::<_, String>(0))
                .map_err(map_rusqlite)?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(map_rusqlite)?);
            }
            Ok(out)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    /// Autocomplete for printer names when editing a cartridge model's
    /// compatibility list (D-06, Plan 13-05).
    ///
    /// Sources suggestions from the real printer roster (`devices.name WHERE
    /// type_id = 2`), not from previously-entered free-text values in
    /// `cartridge_model_compatibility` — the compatibility column stores
    /// free text (D-04), so suggesting from its own history would surface
    /// typos/stale names instead of actual printers a model could match.
    /// `prefix` is passed as a bind parameter (T-13-10) — never concatenated
    /// into the SQL text.
    pub async fn suggest_compat_printer(&self, prefix: String) -> Result<Vec<String>, AppError> {
        let readers = self.readers.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<String>, AppError> {
            let conn = readers.acquire();
            let pattern = format!("{}%", prefix);
            let mut stmt = conn
                .prepare(
                    "SELECT DISTINCT name FROM devices \
                      WHERE type_id = 2 AND deleted_at_utc IS NULL AND name LIKE ?1 \
                      ORDER BY name ASC LIMIT 20",
                )
                .map_err(map_rusqlite)?;
            let rows = stmt
                .query_map(params![pattern], |r| r.get::<_, String>(0))
                .map_err(map_rusqlite)?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(map_rusqlite)?);
            }
            Ok(out)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }

    /// D-11.4: storage places (self OR any ancestor has `is_storage=1`),
    /// exposed for the frontend's ReturnToStock place-suggestion UX (D-11.3).
    /// The backend does not pick a default — a UI concern.
    pub async fn storage_place_ids(&self) -> Result<Vec<i64>, AppError> {
        let readers = self.readers.clone();
        let place_repo = self.place_repo.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<i64>, AppError> {
            use trackly_core::ports::places::PlaceRepository;
            let conn = readers.acquire();
            place_repo.list_storage_place_ids(&conn)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking: {e}"),
        })?
    }
}
