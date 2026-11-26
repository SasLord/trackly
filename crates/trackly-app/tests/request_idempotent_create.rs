//! Идемпотентное создание заявки по клиентскому id (Phase 43, D-11).
//! Имена вымышленные.

use std::time::Duration;

use trackly_app::dto::printer::WsEvent;
use trackly_app::dto::request::{RequestCreateDto, RequestTransitionPayload};
use trackly_core::auth::Role;
use trackly_core::error::AppError;

#[path = "entities_support/fixture.rs"]
#[allow(dead_code)]
mod fixture;

mod chat_support;
use chat_support::*;

macro_rules! timed {
    ($body:expr) => {
        tokio::time::timeout(Duration::from_secs(60), $body)
            .await
            .expect("timeout")
    };
}

fn dto(cid: Option<&str>) -> RequestCreateDto {
    RequestCreateDto {
        request_type: "free_form".to_string(),
        printer_device_id: None,
        cartridge_model_id: None,
        category_id: None,
        description: Some("Не работает сканер".to_string()),
        client_request_id: cid.map(str::to_string),
    }
}

fn new_request_events(evs: &[WsEvent]) -> usize {
    evs.iter()
        .filter(|e| matches!(e, WsEvent::NewRequest { .. }))
        .count()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn replay_returns_same_request_one_row_one_audit_one_event() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let uid = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(uid, Role::Employee);
        let mut rx = ctx.ws_broadcast.subscribe();
        fixture::drain(&mut rx);

        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM requests").await, 0);
        let first = ctx.requests.create(dto(Some("rq-1")), &who).await.unwrap();
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM requests").await, 1);
        let second = ctx.requests.create(dto(Some("rq-1")), &who).await.unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM requests").await, 1);
        assert_eq!(
            count(
                &ctx,
                "SELECT COUNT(*) FROM audit_log WHERE entity_type='request' AND action='create'"
            )
            .await,
            1
        );
        let evs = fixture::collect_events(&mut rx);
        assert_eq!(new_request_events(&evs), 1, "{evs:?}");
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn replay_after_status_change_returns_current_state() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let uid = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let who = ident(uid, Role::Employee);
        let first = ctx.requests.create(dto(Some("rq-1")), &who).await.unwrap();
        ctx.requests
            .transition(
                RequestTransitionPayload::Accept {
                    request_id: first.id,
                    version: first.version,
                    assigned_to_user_id: None,
                },
                &ident(mgr, Role::Manager),
            )
            .await
            .unwrap();
        let mut rx = ctx.ws_broadcast.subscribe();
        fixture::drain(&mut rx);
        let again = ctx.requests.create(dto(Some("rq-1")), &who).await.unwrap();
        assert_eq!(again.id, first.id);
        assert_eq!(
            again.status, "in_progress",
            "текущее состояние, не исходное"
        );
        assert_eq!(
            new_request_events(&fixture::collect_events(&mut rx)),
            0,
            "replay не шлёт NewRequest"
        );
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM requests").await, 1);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn other_author_same_id_creates_own_request() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let a = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let b = seed_user(&ctx, "sidorov", "Сидоров С.С.", "employee").await;
        let ra = ctx
            .requests
            .create(dto(Some("same")), &ident(a, Role::Employee))
            .await
            .unwrap();
        let rb = ctx
            .requests
            .create(dto(Some("same")), &ident(b, Role::Employee))
            .await
            .unwrap();
        assert_ne!(ra.id, rb.id);
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM requests").await, 2);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn invalid_client_id_is_validation_and_no_row() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let uid = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(uid, Role::Employee);
        for bad in [&"x".repeat(65)[..], "a b"] {
            let err = ctx.requests.create(dto(Some(bad)), &who).await.unwrap_err();
            assert!(matches!(err, AppError::Validation { .. }), "{err:?}");
        }
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM requests").await, 0);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn without_client_id_each_create_is_new() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let uid = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(uid, Role::Employee);
        ctx.requests.create(dto(None), &who).await.unwrap();
        ctx.requests.create(dto(None), &who).await.unwrap();
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM requests").await, 2);
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_creates_with_same_id_collapse_to_one() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let uid = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let who = ident(uid, Role::Employee);
        let mut rx = ctx.ws_broadcast.subscribe();
        fixture::drain(&mut rx);
        // Много параллельных create с одним id: часть из них проходит ранний
        // поиск до первой записи и обязана свернуться уже в писателе.
        let svc = ctx.requests.clone();
        let mut tasks = Vec::new();
        for _ in 0..16 {
            let svc = svc.clone();
            let who = who.clone();
            tasks.push(tokio::spawn(async move {
                svc.create(dto(Some("race")), &who).await
            }));
        }
        let mut ids = std::collections::BTreeSet::new();
        for t in tasks {
            ids.insert(t.await.unwrap().unwrap().id);
        }
        assert_eq!(ids.len(), 1);
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM requests").await, 1);
        assert_eq!(
            count(
                &ctx,
                "SELECT COUNT(*) FROM audit_log WHERE entity_type='request' AND action='create'"
            )
            .await,
            1
        );
        assert_eq!(new_request_events(&fixture::collect_events(&mut rx)), 1);
    })
}
