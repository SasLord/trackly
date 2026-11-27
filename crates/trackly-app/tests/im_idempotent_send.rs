//! Идемпотентная отправка IM (Phase 43-04, DLV-03, D-11). Имена вымышленные.

mod im_support;
use im_support::{build_ctx, count_messages, drain, ident, last_seq, seed_user};
use trackly_core::auth::Role;

#[tokio::test]
async fn replay_returns_original_without_row_or_signal() {
    let (ctx, _d) = build_ctx().await;
    let a = seed_user(&ctx, "ii_a1", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "ii_b1", "Петров П.П.", "employee").await;
    let conv = ctx
        .im
        .create_conversation(a, "dialog", None, &[b])
        .await
        .unwrap();
    let mut rb = ctx.im_hub.subscribe(b);
    let me = ident(a, Role::Employee);

    let first = ctx
        .im
        .send(&me, conv, "cid-1", "первый текст")
        .await
        .unwrap();
    assert!(!first.replay);
    assert_eq!(first.message.seq, 1);
    assert_eq!(first.message.rev, 1);
    assert_eq!(drain(&mut rb).len(), 1);

    let again = ctx
        .im
        .send(&me, conv, "cid-1", "первый текст")
        .await
        .unwrap();
    assert!(again.replay);
    assert_eq!(again.message.id, first.message.id);
    assert_eq!(count_messages(&ctx, conv).await, 1);
    assert_eq!(last_seq(&ctx, conv).await, 1);
    assert!(drain(&mut rb).is_empty(), "при replay сигнала нет");

    // Другой текст с тем же id — возвращается оригинал.
    let other = ctx
        .im
        .send(&me, conv, "cid-1", "совсем другой")
        .await
        .unwrap();
    assert!(other.replay);
    assert_eq!(other.message.id, first.message.id);
    assert_eq!(other.message.body_text, "первый текст");
    assert_eq!(count_messages(&ctx, conv).await, 1);
}

#[tokio::test]
async fn same_client_id_from_other_author_is_independent() {
    let (ctx, _d) = build_ctx().await;
    let a = seed_user(&ctx, "ii_a2", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "ii_b2", "Петров П.П.", "employee").await;
    let conv = ctx
        .im
        .create_conversation(a, "dialog", None, &[b])
        .await
        .unwrap();
    let m1 = ctx
        .im
        .send(&ident(a, Role::Employee), conv, "same", "от Иванова")
        .await
        .unwrap();
    let m2 = ctx
        .im
        .send(&ident(b, Role::Employee), conv, "same", "от Петрова")
        .await
        .unwrap();
    assert!(!m1.replay && !m2.replay);
    assert_ne!(m1.message.id, m2.message.id);
    assert_eq!(count_messages(&ctx, conv).await, 2);
    assert_eq!(m2.message.seq, 2);
}

#[tokio::test]
async fn client_msg_id_hidden_from_non_author() {
    let (ctx, _d) = build_ctx().await;
    let a = seed_user(&ctx, "ii_a3", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "ii_b3", "Петров П.П.", "employee").await;
    let conv = ctx
        .im
        .create_conversation(a, "dialog", None, &[b])
        .await
        .unwrap();
    ctx.im
        .send(&ident(a, Role::Employee), conv, "secret-id", "текст")
        .await
        .unwrap();
    let mine = ctx
        .im
        .sync(&ident(a, Role::Employee), &Default::default())
        .await
        .unwrap();
    let theirs = ctx
        .im
        .sync(&ident(b, Role::Employee), &Default::default())
        .await
        .unwrap();
    assert_eq!(
        mine.conversations[0].messages[0].client_msg_id.as_deref(),
        Some("secret-id")
    );
    assert_eq!(theirs.conversations[0].messages[0].client_msg_id, None);
}

/// WR-06 / D-11: повтор после исключения автора — replay, новая отправка — 403.
#[tokio::test]
async fn replay_after_author_removed_still_returns_original() {
    let (ctx, _d) = build_ctx().await;
    let a = seed_user(&ctx, "ii_a4", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "ii_b4", "Петров П.П.", "employee").await;
    let conv = ctx
        .im
        .create_conversation(a, "room", Some("Отдел"), &[b])
        .await
        .unwrap();
    let me = ident(b, Role::Employee);
    let first = ctx.im.send(&me, conv, "cid-x", "текст").await.unwrap();
    ctx.im.remove_member(conv, b).await.unwrap();
    let mut ra = ctx.im_hub.subscribe(a);
    let again = ctx.im.send(&me, conv, "cid-x", "текст").await.unwrap();
    assert!(again.replay);
    assert_eq!(again.message.id, first.message.id);
    assert!(drain(&mut ra).is_empty());
    let err = ctx
        .im
        .send(&me, conv, "cid-new", "новый")
        .await
        .unwrap_err();
    assert!(
        matches!(err, trackly_core::error::AppError::Forbidden),
        "{err:?}"
    );
    let stranger = seed_user(&ctx, "ii_x4", "Кузнецов К.К.", "employee").await;
    let err = ctx
        .im
        .send(&ident(stranger, Role::Employee), conv, "cid-x", "текст")
        .await
        .unwrap_err();
    assert!(
        matches!(err, trackly_core::error::AppError::NotFound { .. }),
        "{err:?}"
    );
}
