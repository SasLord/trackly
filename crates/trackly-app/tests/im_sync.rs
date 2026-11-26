//! Курсорная синхронизация IM (Phase 43-04, DLV-01, D-06). Имена вымышленные.

mod im_support;
use im_support::{build_ctx, ident, seed_user};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::im::{ImCursorDto, ImSyncRequestDto};
use trackly_core::auth::Role;
use trackly_infra::error_conversions::map_rusqlite;

fn req(cursors: &[(i64, i64)]) -> ImSyncRequestDto {
    ImSyncRequestDto {
        cursors: cursors
            .iter()
            .map(|(c, r)| ImCursorDto {
                conversation_id: *c,
                rev: *r,
            })
            .collect(),
        limit: None,
    }
}

async fn say(ctx: &AppCtx, who: i64, conv: i64, n: usize) {
    for i in 0..n {
        ctx.im
            .send(
                &ident(who, Role::Employee),
                conv,
                &format!("c{who}-{conv}-{i}-{}", rand_tag()),
                &format!("сообщение {i}"),
            )
            .await
            .unwrap();
    }
}

fn rand_tag() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

#[tokio::test]
async fn no_cursor_gives_summary_and_last_page() {
    let (ctx, _d) = build_ctx().await;
    let a = seed_user(&ctx, "is_a1", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "is_b1", "Петров П.П.", "employee").await;
    let conv = ctx
        .im
        .create_conversation(a, "dialog", None, &[b])
        .await
        .unwrap();
    say(&ctx, a, conv, 3).await;
    let r = ctx
        .im
        .sync(&ident(b, Role::Employee), &req(&[]))
        .await
        .unwrap();
    assert_eq!(r.conversations.len(), 1);
    let c = &r.conversations[0];
    assert!(!c.reload && !c.conversation.read_only);
    assert_eq!(c.conversation.last_seq, 3);
    let seqs: Vec<i64> = c.messages.iter().map(|m| m.seq).collect();
    assert_eq!(seqs, vec![1, 2, 3]);
}

#[tokio::test]
async fn lagging_cursor_gets_delta_and_current_cursor_is_empty() {
    let (ctx, _d) = build_ctx().await;
    let a = seed_user(&ctx, "is_a2", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "is_b2", "Петров П.П.", "employee").await;
    let conv = ctx
        .im
        .create_conversation(a, "dialog", None, &[b])
        .await
        .unwrap();
    say(&ctx, a, conv, 5).await;
    let me = ident(b, Role::Employee);
    let r = ctx.im.sync(&me, &req(&[(conv, 2)])).await.unwrap();
    let seqs: Vec<i64> = r.conversations[0].messages.iter().map(|m| m.rev).collect();
    assert_eq!(seqs, vec![3, 4, 5]);
    assert!(!r.conversations[0].reload);
    let r = ctx.im.sync(&me, &req(&[(conv, 5)])).await.unwrap();
    assert!(r.conversations.is_empty(), "курсор равен last_seq");
}

#[tokio::test]
async fn too_many_changes_forces_reload() {
    let (ctx, _d) = build_ctx().await;
    let a = seed_user(&ctx, "is_a3", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "is_b3", "Петров П.П.", "employee").await;
    let conv = ctx
        .im
        .create_conversation(a, "dialog", None, &[b])
        .await
        .unwrap();
    say(&ctx, a, conv, 205).await;
    let r = ctx
        .im
        .sync(&ident(b, Role::Employee), &req(&[(conv, 0)]))
        .await
        .unwrap();
    let c = &r.conversations[0];
    assert!(c.reload);
    assert!(c.messages.len() <= 50, "последняя страница, не вся история");
    assert_eq!(c.messages.last().unwrap().seq, 205);
}

#[tokio::test]
async fn foreign_conversation_is_not_disclosed() {
    let (ctx, _d) = build_ctx().await;
    let a = seed_user(&ctx, "is_a4", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "is_b4", "Петров П.П.", "employee").await;
    let x = seed_user(&ctx, "is_x4", "Кузнецов К.К.", "employee").await;
    let conv = ctx
        .im
        .create_conversation(a, "dialog", None, &[b])
        .await
        .unwrap();
    say(&ctx, a, conv, 2).await;
    let r = ctx
        .im
        .sync(&ident(x, Role::Employee), &req(&[(conv, 0)]))
        .await
        .unwrap();
    assert!(r.conversations.is_empty());
    let r = ctx
        .im
        .sync(&ident(x, Role::Admin), &req(&[(conv, 0)]))
        .await
        .unwrap();
    assert!(r.conversations.is_empty());
}

async fn tamper(ctx: &AppCtx, conv: i64, seq: i64, rev: i64, text: &str) {
    let text = text.to_string();
    ctx.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE im_messages SET rev = ?3, body_text = ?4 \
                 WHERE conversation_id = ?1 AND seq = ?2",
                params![conv, seq, rev, text],
            )
            .map_err(map_rusqlite)?;
            // Настоящая правка двигает и счётчик беседы.
            conn.execute(
                "UPDATE im_conversations SET last_seq = MAX(last_seq, ?2) WHERE id = ?1",
                params![conv, rev],
            )
            .map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn removed_member_gets_frozen_history_once() {
    let (ctx, _d) = build_ctx().await;
    let a = seed_user(&ctx, "is_a5", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "is_b5", "Петров П.П.", "employee").await;
    let conv = ctx
        .im
        .create_conversation(a, "room", Some("Отдел"), &[b])
        .await
        .unwrap();
    say(&ctx, a, conv, 3).await;
    ctx.im.remove_member(conv, b).await.unwrap();
    let me = ident(b, Role::Employee);

    let before = ctx.im.sync(&me, &req(&[])).await.unwrap();
    let seqs = |r: &trackly_app::dto::im::ImSyncResponseDto| -> Vec<i64> {
        r.conversations[0].messages.iter().map(|m| m.seq).collect()
    };
    assert_eq!(seqs(&before), vec![1, 2, 3]);
    let c = &before.conversations[0];
    assert!(c.conversation.read_only && c.reload);
    assert_eq!(c.conversation.left_seq, Some(3));
    assert_eq!(c.conversation.last_seq, 3);

    // Остальные пишут дальше; неизменённая история не пропадает и не меняется.
    say(&ctx, a, conv, 4).await;
    let after = ctx.im.sync(&me, &req(&[])).await.unwrap();
    assert_eq!(seqs(&after), vec![1, 2, 3]);
    assert_eq!(after.conversations[0].conversation.last_seq, 3);
    let with_old_cursor = ctx.im.sync(&me, &req(&[(conv, 1)])).await.unwrap();
    assert_eq!(seqs(&with_old_cursor), vec![1, 2, 3]);

    // Курсор применён (left_seq) — беседа больше не возвращается.
    let done = ctx.im.sync(&me, &req(&[(conv, 3)])).await.unwrap();
    assert!(done.conversations.is_empty());
}

#[tokio::test]
async fn edit_after_leaving_is_not_delivered() {
    let (ctx, _d) = build_ctx().await;
    let a = seed_user(&ctx, "is_a6", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "is_b6", "Петров П.П.", "employee").await;
    let conv = ctx
        .im
        .create_conversation(a, "room", Some("Отдел"), &[b])
        .await
        .unwrap();
    say(&ctx, a, conv, 3).await;
    ctx.im.remove_member(conv, b).await.unwrap();
    let me = ident(b, Role::Employee);

    let r = ctx.im.sync(&me, &req(&[])).await.unwrap();
    assert!(r.conversations[0].messages.iter().any(|m| m.seq == 2));

    // Правка сообщения 2 после выхода: rev поднимается выше left_seq.
    tamper(&ctx, conv, 2, 10, "правка после выхода").await;
    let r = ctx.im.sync(&me, &req(&[])).await.unwrap();
    let seqs: Vec<i64> = r.conversations[0].messages.iter().map(|m| m.seq).collect();
    assert_eq!(seqs, vec![1, 3], "изменённое после выхода не отдаётся");
    assert!(r.conversations[0]
        .messages
        .iter()
        .all(|m| m.body_text != "правка после выхода"));
    // Остальные участники правку видят.
    let other = ctx
        .im
        .sync(&ident(a, Role::Employee), &req(&[(conv, 3)]))
        .await
        .unwrap();
    assert_eq!(
        other.conversations[0].messages[0].body_text,
        "правка после выхода"
    );
}
