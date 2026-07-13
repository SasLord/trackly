// audit_log.action for cartridge install = 'custom:install' (verified in CartridgeTransitionOp::audit_action)
//
//! Organisation settings integration tests — Phase 7 Plan 02 (GREEN).
//!
//! Covers SET-01 (org data save/load round-trip) and SET-02 (logo BLOB upload/delete).
//!
//! Key invariants:
//!   - Only one row can exist in org_settings (CHECK id = 1)
//!   - Save updates the existing row (never inserts a second)
//!   - Logo stored as raw bytes; retrieved as Vec<u8> + mime type
//!   - OrgSettingsDto.has_logo is false when logo_blob IS NULL

use std::sync::Arc;
use std::time::Duration;

use trackly_app::dto::reports::OrgPatch;
use trackly_app::services::OrgDbService;
use trackly_core::auth::Identity;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;
use trackly_infra::Paths;

fn make_org_service() -> (OrgDbService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let paths = Arc::new(
        Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("Paths::resolve_for_exe_dir"),
    );
    let svc = OrgDbService::new(writer, readers, clock, paths);
    (svc, dir)
}

fn admin_caller() -> Identity {
    Identity::trusted_admin()
}

/// Verify that OrgPatch is persisted and retrieved as OrgSettingsDto.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn org_settings_save_and_load_round_trip() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_org_service();
        let caller = admin_caller();

        // Проверяем начальное состояние (placeholder из V026 migration seed)
        let initial = svc.get().await.expect("get initial");
        assert_eq!(initial.org_name, "Ваша организация");
        assert!(!initial.has_logo);
        // V033: новые реквизиты дефолтятся в пустую строку (не placeholder), per D-02
        assert_eq!(initial.phone, "");
        assert_eq!(initial.fax, "");
        assert_eq!(initial.email, "");
        assert_eq!(initial.okpo, "");
        assert_eq!(initial.ogrn, "");

        // Сохраняем новые данные
        let patch = OrgPatch {
            org_name: "ООО Тестовая компания".to_string(),
            inn: "7712345678".to_string(),
            kpp: "771001001".to_string(),
            address: "г. Москва, ул. Тестовая, 42".to_string(),
            phone: "+7 495 123-45-67".to_string(),
            fax: "+7 495 123-45-68".to_string(),
            email: "info@test.ru".to_string(),
            okpo: "12345678".to_string(),
            ogrn: "1027700123456".to_string(),
            address_line2: String::new(),
        };
        svc.save_fields(&caller, patch).await.expect("save_fields");

        // Читаем и проверяем
        let updated = svc.get().await.expect("get updated");
        assert_eq!(updated.org_name, "ООО Тестовая компания");
        assert_eq!(updated.inn, "7712345678");
        assert_eq!(updated.kpp, "771001001");
        assert_eq!(updated.address, "г. Москва, ул. Тестовая, 42");
        assert_eq!(updated.phone, "+7 495 123-45-67");
        assert_eq!(updated.fax, "+7 495 123-45-68");
        assert_eq!(updated.email, "info@test.ru");
        assert_eq!(updated.okpo, "12345678");
        assert_eq!(updated.ogrn, "1027700123456");
        assert!(
            !updated.has_logo,
            "has_logo должен быть false — лого не загружали"
        );
    })
    .await
    .expect("org_settings_save_and_load_round_trip budget")
}

/// Verify that logo BLOB upload sets has_logo=true and can be cleared.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn org_logo_save_and_delete() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_org_service();
        let caller = admin_caller();

        // Изначально лого нет
        let initial = svc.get().await.expect("get initial");
        assert!(!initial.has_logo);

        // Загружаем лого (минимальный валидный PNG 1x1 px)
        let png_bytes = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG magic
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 px
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, // 8bit RGB
            0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, // IDAT chunk
            0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2,
            0x21, 0xBC, 0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, // IEND chunk
            0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        svc.save_logo(&caller, png_bytes.clone(), "image/png".to_string())
            .await
            .expect("save_logo");

        // Проверяем has_logo = true
        let with_logo = svc.get().await.expect("get with logo");
        assert!(
            with_logo.has_logo,
            "has_logo должен быть true после save_logo"
        );

        // Получаем байты лого
        let logo_bytes = svc
            .get_logo_bytes()
            .await
            .expect("get_logo_bytes")
            .expect("лого должно существовать");
        assert_eq!(logo_bytes, png_bytes, "байты лого должны совпадать");

        // Удаляем лого
        svc.remove_logo(&caller).await.expect("remove_logo");

        // Проверяем has_logo = false
        let without_logo = svc.get().await.expect("get without logo");
        assert!(
            !without_logo.has_logo,
            "has_logo должен быть false после remove_logo"
        );

        // Байты лого = None
        let logo_after_remove = svc
            .get_logo_bytes()
            .await
            .expect("get_logo_bytes after remove");
        assert!(
            logo_after_remove.is_none(),
            "лого должно быть None после remove_logo"
        );
    })
    .await
    .expect("org_logo_save_and_delete budget")
}

/// Verify that logo size limit is enforced (> 512 KiB rejected).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn org_logo_size_limit_enforced() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_org_service();
        let caller = admin_caller();

        // 513 KiB > 512 KiB limit
        let big_bytes = vec![0u8; 513 * 1024];
        let result = svc
            .save_logo(&caller, big_bytes, "image/png".to_string())
            .await;
        assert!(
            result.is_err(),
            "должна быть ошибка при превышении размера лого"
        );
        match result {
            Err(trackly_core::error::AppError::Validation { field, .. }) => {
                assert_eq!(field, "logo");
            }
            other => panic!("ожидали Validation, получили: {other:?}"),
        }
    })
    .await
    .expect("org_logo_size_limit_enforced budget")
}

/// Verify that unsupported mime type is rejected.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn org_logo_invalid_mime_rejected() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_org_service();
        let caller = admin_caller();

        let result = svc
            .save_logo(&caller, vec![1, 2, 3], "application/pdf".to_string())
            .await;
        assert!(
            result.is_err(),
            "должна быть ошибка при неподдерживаемом mime"
        );
        match result {
            Err(trackly_core::error::AppError::Validation { field, .. }) => {
                assert_eq!(field, "logo_mime");
            }
            other => panic!("ожидали Validation, получили: {other:?}"),
        }
    })
    .await
    .expect("org_logo_invalid_mime_rejected budget")
}
