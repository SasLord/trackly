//! `NumberTemplateService` CRUD integration tests — Plan 40.2-04 Task 2.
//!
//! Covers: create/list/update/delete round-trip with a real preview,
//! duplicate (type, mask) rejection with a human-readable "уже есть"
//! message, optimistic-lock mismatch on `update_mask`, and `preview_mask`
//! validating a mask WITHOUT persisting it (D-10/D-12 adjacent NUM-02/NUM-04
//! coverage at the service layer).

use std::sync::Arc;

use trackly_core::error::AppError;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

use trackly_app::dto::number_template::TemplateTypeDto;
use trackly_app::services::NumberTemplateService;

/// Set up a fresh `NumberTemplateService` backed by an in-memory migrated DB.
fn make_service() -> (NumberTemplateService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let svc = NumberTemplateService::new(writer, readers, clock);
    (svc, dir)
}

#[tokio::test]
async fn create_list_update_delete_roundtrip_with_preview() {
    let (svc, _dir) = make_service();

    let created = svc
        .create(TemplateTypeDto::ActNumber, "[YYYY]/[MM]-[X]".to_string())
        .await
        .expect("create");
    assert_eq!(created.mask, "[YYYY]/[MM]-[X]");
    assert_eq!(created.version, 1);
    // Unbounded `[X]` token → never overflows, first preview is always 1.
    assert!(created.next_first_free.ends_with("-1"));
    assert!(!created.overflowed);

    let listed = svc
        .list(Some(TemplateTypeDto::ActNumber))
        .await
        .expect("list");
    assert!(
        listed.iter().any(|t| t.id == created.id),
        "created template must appear in list()"
    );

    let updated = svc
        .update_mask(created.id, "[YYYY]-[X]".to_string(), created.version)
        .await
        .expect("update_mask");
    assert_eq!(updated.mask, "[YYYY]-[X]");
    assert_eq!(updated.version, 2, "version must have incremented");

    svc.delete(created.id).await.expect("delete");

    let err = svc
        .get(created.id)
        .await
        .expect_err("get after delete must fail");
    match err {
        AppError::NotFound { entity, id } => {
            assert_eq!(entity, "number_template");
            assert_eq!(id, created.id);
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn create_rejects_duplicate_type_and_mask() {
    let (svc, _dir) = make_service();

    svc.create(
        TemplateTypeDto::CartridgeCode,
        "ЗАПРАВКА-[XXXX]".to_string(),
    )
    .await
    .expect("first create");

    let err = svc
        .create(
            TemplateTypeDto::CartridgeCode,
            "ЗАПРАВКА-[XXXX]".to_string(),
        )
        .await
        .expect_err("duplicate (type, mask) must be rejected");
    match err {
        AppError::Conflict { reason } => {
            assert!(
                reason.contains("уже есть"),
                "conflict text must contain 'уже есть', got: {reason}"
            );
            assert!(
                reason.contains("Картриджи"),
                "conflict text must name the Russian type, got: {reason}"
            );
        }
        other => panic!("expected Conflict, got {other:?}"),
    }
}

#[tokio::test]
async fn update_mask_rejects_duplicate_among_other_templates_of_same_type() {
    let (svc, _dir) = make_service();

    let a = svc
        .create(TemplateTypeDto::DrumCode, "ФБ-А-[XXX]".to_string())
        .await
        .expect("create a");
    let b = svc
        .create(TemplateTypeDto::DrumCode, "ФБ-Б-[XXX]".to_string())
        .await
        .expect("create b");

    // Editing b's own mask to a's value must be rejected (excludes self, not
    // "other templates").
    let err = svc
        .update_mask(b.id, "ФБ-А-[XXX]".to_string(), b.version)
        .await
        .expect_err("must reject colliding with another template's mask");
    match err {
        AppError::Conflict { reason } => assert!(reason.contains("уже есть")),
        other => panic!("expected Conflict, got {other:?}"),
    }

    // Editing a's own mask back to its own current value must NOT be a
    // self-conflict.
    let unchanged = svc
        .update_mask(a.id, "ФБ-А-[XXX]".to_string(), a.version)
        .await
        .expect("no-op edit to own current mask must succeed");
    assert_eq!(unchanged.version, 2);
}

#[tokio::test]
async fn update_mask_stale_version_is_optimistic_lock_mismatch() {
    let (svc, _dir) = make_service();

    let created = svc
        .create(TemplateTypeDto::DeviceInventory, "ИНВ-[XXXXXX]".to_string())
        .await
        .expect("create");

    let err = svc
        .update_mask(
            created.id,
            "ИНВ-НОВ-[XXXXXX]".to_string(),
            created.version + 1,
        )
        .await
        .expect_err("stale version must fail");
    match err {
        AppError::OptimisticLockMismatch {
            entity,
            id,
            expected,
            actual,
        } => {
            assert_eq!(entity, "number_template");
            assert_eq!(id, created.id);
            assert_eq!(expected, created.version + 1);
            assert_eq!(actual, created.version);
        }
        other => panic!("expected OptimisticLockMismatch, got {other:?}"),
    }
}

#[tokio::test]
async fn preview_mask_validates_without_persisting() {
    let (svc, _dir) = make_service();

    let today = SystemClock.unix_seconds();

    let before = svc.list(None).await.expect("list before").len();

    let err = svc
        .preview_mask(TemplateTypeDto::ActNumber, "ОРГ-[YYYY]".to_string(), today)
        .await
        .expect_err("mask without [X…] must fail validation");
    match err {
        AppError::Validation { message, .. } => {
            assert!(message.contains("нужен порядковый номер"), "got: {message}");
        }
        other => panic!("expected Validation, got {other:?}"),
    }

    let after = svc.list(None).await.expect("list after").len();
    assert_eq!(before, after, "preview_mask must never persist a row");

    // A valid, never-saved mask previews correctly too.
    let preview = svc
        .preview_mask(
            TemplateTypeDto::DeviceInventory,
            "ПРЕВЬЮ-[XXX]".to_string(),
            today,
        )
        .await
        .expect("valid preview mask");
    assert_eq!(preview.rendered, "ПРЕВЬЮ-001");

    let after_valid_preview = svc
        .list(None)
        .await
        .expect("list after valid preview")
        .len();
    assert_eq!(
        before, after_valid_preview,
        "a successful preview_mask must also never persist a row"
    );
}

/// BE-CR-05: edge whitespace in a mask (copy-paste accident) is trimmed on
/// save, so the template recognises the numbers it itself suggests.
#[tokio::test]
async fn mask_edge_whitespace_is_trimmed_on_create_update_and_preview() {
    let (svc, _dir) = make_service();

    let created = svc
        .create(TemplateTypeDto::DeviceInventory, "  ОРГ-[XXX] ".to_string())
        .await
        .expect("create");
    assert_eq!(created.mask, "ОРГ-[XXX]");
    assert_eq!(created.next_first_free, "ОРГ-001");

    let updated = svc
        .update_mask(created.id, "\tИНВ-[XXX]\n".to_string(), created.version)
        .await
        .expect("update_mask");
    assert_eq!(updated.mask, "ИНВ-[XXX]");

    let preview = svc
        .preview_mask(
            TemplateTypeDto::DeviceInventory,
            "К-[XX] ".to_string(),
            SystemClock.unix_seconds(),
        )
        .await
        .expect("preview");
    assert_eq!(preview.rendered, "К-01");

    // A trimmed duplicate of an existing mask is still a duplicate.
    let dup = svc
        .create(TemplateTypeDto::DeviceInventory, "ИНВ-[XXX] ".to_string())
        .await;
    assert!(matches!(dup, Err(AppError::Conflict { .. })), "got {dup:?}");
}
