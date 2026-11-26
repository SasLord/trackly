//! Членство и доставка сигналов IM (Phase 43-04, DLV-02, D-05). Имена вымышленные.

mod im_support;
use im_support::{build_ctx, drain, ident, seed_user};
use trackly_app::services::im_hub::ImSignal;
use trackly_core::auth::Role;
use trackly_core::error::AppError;

#[tokio::test]
async fn non_member_gets_not_found_even_admin() {
    let (ctx, _d) = build_ctx().await;
    let a = seed_user(&ctx, "im_a1", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "im_b1", "Петров П.П.", "employee").await;
    let adm = seed_user(&ctx, "im_adm1", "Смирнов С.С.", "admin").await;
    let conv = ctx
        .im
        .create_conversation(a, "dialog", None, &[b])
        .await
        .unwrap();
    let r = ctx
        .im
        .send(&ident(adm, Role::Admin), conv, "c-1", "привет")
        .await;
    assert!(matches!(r, Err(AppError::NotFound { .. })), "{r:?}");
}

#[tokio::test]
async fn signals_only_to_members() {
    let (ctx, _d) = build_ctx().await;
    let a = seed_user(&ctx, "im_a2", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "im_b2", "Петров П.П.", "employee").await;
    let c = seed_user(&ctx, "im_c2", "Сидорова А.А.", "employee").await;
    let d = seed_user(&ctx, "im_d2", "Кузнецов К.К.", "employee").await;
    let conv1 = ctx
        .im
        .create_conversation(a, "dialog", None, &[b])
        .await
        .unwrap();
    let conv2 = ctx
        .im
        .create_conversation(c, "dialog", None, &[d])
        .await
        .unwrap();
    let (mut ra, mut rb, mut rc) = (
        ctx.im_hub.subscribe(a),
        ctx.im_hub.subscribe(b),
        ctx.im_hub.subscribe(c),
    );
    ctx.im
        .send(&ident(a, Role::Employee), conv1, "m1", "текст")
        .await
        .unwrap();
    assert_eq!(drain(&mut ra).len(), 1);
    assert_eq!(drain(&mut rb).len(), 1);
    // c не в беседе 1: сигнала нет.
    assert!(drain(&mut rc).is_empty());
    // Контроль: сигнал в беседу 2 до c доходит — канал c рабочий.
    ctx.im
        .send(&ident(d, Role::Employee), conv2, "m2", "текст")
        .await
        .unwrap();
    let got = drain(&mut rc);
    assert_eq!(got.len(), 1);
    assert!(
        matches!(got[0], ImSignal::Changed { conversation_id, .. } if conversation_id == conv2)
    );
}

#[tokio::test]
async fn removed_member_stops_receiving() {
    let (ctx, _d) = build_ctx().await;
    let a = seed_user(&ctx, "im_a3", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "im_b3", "Петров П.П.", "employee").await;
    let c = seed_user(&ctx, "im_c3", "Сидорова А.А.", "employee").await;
    let conv = ctx
        .im
        .create_conversation(a, "room", Some("Отдел"), &[b, c])
        .await
        .unwrap();
    let (mut rb, mut rc) = (ctx.im_hub.subscribe(b), ctx.im_hub.subscribe(c));
    ctx.im
        .send(&ident(a, Role::Employee), conv, "m1", "до")
        .await
        .unwrap();
    assert_eq!(drain(&mut rb).len(), 1, "до исключения получает");
    assert_eq!(drain(&mut rc).len(), 1);
    ctx.im.remove_member(conv, b).await.unwrap();
    ctx.im
        .send(&ident(a, Role::Employee), conv, "m2", "после")
        .await
        .unwrap();
    assert!(drain(&mut rb).is_empty(), "после исключения не получает");
    assert_eq!(drain(&mut rc).len(), 1, "контроль: остальные получают");
}

#[tokio::test]
async fn removed_member_cannot_send() {
    let (ctx, _d) = build_ctx().await;
    let a = seed_user(&ctx, "im_a4", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "im_b4", "Петров П.П.", "employee").await;
    let conv = ctx
        .im
        .create_conversation(a, "room", Some("Отдел"), &[b])
        .await
        .unwrap();
    ctx.im
        .send(&ident(b, Role::Employee), conv, "x1", "было")
        .await
        .unwrap();
    ctx.im.remove_member(conv, b).await.unwrap();
    // Даже replay уже отправленного не обходит членство.
    let r = ctx
        .im
        .send(&ident(b, Role::Employee), conv, "x1", "было")
        .await;
    assert!(matches!(r, Err(AppError::Forbidden)), "{r:?}");
}

#[tokio::test]
async fn validation_errors() {
    let (ctx, _d) = build_ctx().await;
    let a = seed_user(&ctx, "im_a5", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "im_b5", "Петров П.П.", "employee").await;
    let conv = ctx
        .im
        .create_conversation(a, "dialog", None, &[b])
        .await
        .unwrap();
    let id = ident(a, Role::Employee);
    for (cid, text) in [("ok", "   "), ("bad id", "т"), ("ok", &"я".repeat(4001))] {
        let r = ctx.im.send(&id, conv, cid, text).await;
        assert!(
            matches!(r, Err(AppError::Validation { .. })),
            "{cid}: {r:?}"
        );
    }
}

#[tokio::test]
async fn dialog_is_idempotent_and_rejoin_hides_gap() {
    let (ctx, _d) = build_ctx().await;
    let a = seed_user(&ctx, "im_a6", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "im_b6", "Петров П.П.", "employee").await;
    let c1 = ctx
        .im
        .create_conversation(a, "dialog", None, &[b])
        .await
        .unwrap();
    let c2 = ctx
        .im
        .create_conversation(b, "dialog", None, &[a])
        .await
        .unwrap();
    assert_eq!(c1, c2);

    let room = ctx
        .im
        .create_conversation(a, "room", Some("Отдел"), &[b])
        .await
        .unwrap();
    ctx.im.remove_member(room, b).await.unwrap();
    ctx.im
        .send(&ident(a, Role::Employee), room, "g1", "пока тебя нет")
        .await
        .unwrap();
    ctx.im.add_member(room, b).await.unwrap();
    ctx.im
        .send(&ident(a, Role::Employee), room, "g2", "ты вернулся")
        .await
        .unwrap();
    let resp = ctx
        .im
        .sync(&ident(b, Role::Employee), &Default::default())
        .await
        .unwrap();
    let conv = resp
        .conversations
        .iter()
        .find(|c| c.conversation.id == room)
        .unwrap();
    let texts: Vec<_> = conv.messages.iter().map(|m| m.body_text.as_str()).collect();
    assert_eq!(texts, vec!["ты вернулся"]);
    assert!(!conv.conversation.read_only);
}
