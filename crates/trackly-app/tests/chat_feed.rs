//! Chat feed (Phase 41.5, D-07/D-11/D-12/D-13/D-16): merge on read, roles,
//! resolution block, `can_post`. Names are fictional.

use std::time::Duration;

use trackly_app::dto::chat::{ChatItemDto, ChatMessageDto};
use trackly_app::services::attachment_service::ParentRef;
use trackly_core::auth::{Identity, Role};
use trackly_core::domain::attachments::ParentKind;
use trackly_infra::repos::chat_sqlite::SqliteChatRepository;

#[path = "entities_support/fixture.rs"]
#[allow(dead_code)]
mod fixture;

mod attachments_support;
mod chat_support;
use chat_support::*;

macro_rules! timed {
    ($body:expr) => {
        tokio::time::timeout(Duration::from_secs(60), $body)
            .await
            .expect("timeout")
    };
}

fn keys(items: &[ChatItemDto]) -> Vec<String> {
    items.iter().map(|i| i.key().to_string()).collect()
}

fn msg<'a>(items: &'a [ChatItemDto], key: &str) -> &'a ChatMessageDto {
    items
        .iter()
        .find_map(|i| match i {
            ChatItemDto::Message(m) if m.key == key => Some(m),
            _ => None,
        })
        .expect("message item")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_feed_merge() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let r = seed_request(&ctx, author, "repair", "in_progress").await;
        let e1 = audit(&ctx, r, "create", Some(author), None, 1_700_000_100).await;
        // Same second as the message: the event goes first.
        let m1 = message(&ctx, r, Some(mgr), "Принято в работу", 1_700_000_200, None).await;
        let e2 = audit(&ctx, r, "accept", Some(mgr), None, 1_700_000_200).await;
        let up = upload(
            &ctx,
            &ident(author, Role::Employee),
            ParentRef {
                kind: ParentKind::Request,
                id: r,
            },
            "фото.png",
            &attachments_support::tiny_png(),
        )
        .await
        .unwrap();
        let att_id = up.attachment.unwrap().id;

        let feed = ctx
            .chat
            .feed(&ident(author, Role::Employee), r)
            .await
            .unwrap();
        let got = keys(&feed.items);
        assert_eq!(got.len(), 4, "{got:?}");
        assert_eq!(got[0], format!("e{e1}"));
        assert_eq!(
            got[1],
            format!("e{e2}"),
            "event before message at equal time"
        );
        assert_eq!(got[2], format!("m{m1}"));
        assert!(got[3].starts_with(&format!("a{att_id}:added:")), "{got:?}");
        let mut uniq = got.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(uniq.len(), got.len(), "keys are unique");
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_feed_message_file_in_bubble_only() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, author, "free_form", "open").await;
        let who = ident(author, Role::Employee);
        let draft = ParentRef {
            kind: ParentKind::MessageDraft,
            id: r,
        };
        let up = upload(
            &ctx,
            &who,
            draft,
            "фото.png",
            &attachments_support::tiny_png(),
        )
        .await
        .unwrap();
        let att_id = up.attachment.unwrap().id;
        let mid = message(
            &ctx,
            r,
            Some(author),
            "Фото во вложении",
            1_700_000_300,
            None,
        )
        .await;
        ctx.writer
            .execute(move |conn| {
                SqliteChatRepository.promote_drafts_on(conn, r, author, &[att_id], mid)?;
                Ok(())
            })
            .await
            .unwrap();
        // The audit row of the message file carries the marker: no separate block.
        audit(
            &ctx,
            r,
            "custom:attachment_added",
            Some(author),
            Some(&format!(
                "{{\"attachment_id\":{att_id},\"name\":\"фото.png\",\"message_id\":{mid}}}"
            )),
            1_700_000_300,
        )
        .await;

        let feed = ctx.chat.feed(&who, r).await.unwrap();
        assert_eq!(feed.items.len(), 1, "{:?}", keys(&feed.items));
        let m = msg(&feed.items, &format!("m{mid}"));
        assert_eq!(m.attachments.len(), 1);
        assert!(m.attachments[0].from_message);
        assert!(!feed
            .items
            .iter()
            .any(|i| matches!(i, ChatItemDto::AttachmentEvent(_))));
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_not_in_audit() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, author, "repair", "completed").await;
        set_resolution(&ctx, r, "Заменён ролик подачи").await;
        let before = count(&ctx, "SELECT COUNT(*) FROM audit_log").await;
        message(&ctx, r, Some(author), "Спасибо", 1_700_000_100, None).await;
        ctx.chat
            .feed(&ident(author, Role::Employee), r)
            .await
            .unwrap();
        assert_eq!(count(&ctx, "SELECT COUNT(*) FROM audit_log").await, before);
        // The resolution text is not copied into chat_messages (D-12).
        assert_eq!(
            count(
                &ctx,
                "SELECT COUNT(*) FROM chat_messages WHERE body LIKE '%ролик%'"
            )
            .await,
            0
        );
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_feed_resolution_block() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let r = seed_request(&ctx, author, "repair", "completed").await;
        set_resolution(&ctx, r, "Заменён ролик подачи").await;
        let e_rej = audit(
            &ctx,
            r,
            "reject",
            Some(mgr),
            Some("{\"notes\":\"Нужны уточнения\"}"),
            1_700_000_100,
        )
        .await;
        let e_done = audit(
            &ctx,
            r,
            "complete",
            Some(mgr),
            Some("{\"notes\":\"Заменён ролик подачи\"}"),
            1_700_000_200,
        )
        .await;

        let feed = ctx.chat.feed(&ident(mgr, Role::Manager), r).await.unwrap();
        assert_eq!(
            feed.resolution_notes.as_deref(),
            Some("Заменён ролик подачи")
        );
        let show: Vec<(i64, bool, Option<String>)> = feed
            .items
            .iter()
            .filter_map(|i| match i {
                ChatItemDto::Event(e) => Some((e.id, e.show_notes, e.notes.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(show.len(), 2);
        assert_eq!(show[0].0, e_rej);
        assert!(!show[0].1, "only the last resolution event shows notes");
        assert_eq!(show[1].0, e_done);
        assert!(show[1].1);
        assert_eq!(show[1].2.as_deref(), Some("Заменён ролик подачи"));

        // An old request without any message yields the same block.
        assert!(feed
            .items
            .iter()
            .all(|i| !matches!(i, ChatItemDto::Message(_))));
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_feed_can_post_by_status() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        for (status, want) in [
            ("open", true),
            ("in_progress", true),
            ("in_repair", true),
            ("completed", false),
            ("rejected", false),
            ("cancelled", false),
        ] {
            let r = seed_request(&ctx, author, "repair", status).await;
            let feed = ctx
                .chat
                .feed(&ident(author, Role::Employee), r)
                .await
                .unwrap();
            assert_eq!(feed.can_post, want, "{status}");
        }
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_feed_author_roles_and_flags() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let member = seed_user(&ctx, "sidorova", "Сидорова А.А.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let adm = seed_user(&ctx, "admin2", "Админов А.А.", "admin").await;
        let r = seed_request(&ctx, author, "repair", "open").await;
        add_participant(&ctx, r, member).await;
        let ma = message(&ctx, r, Some(author), "а", 1_700_000_100, None).await;
        let mp = message(&ctx, r, Some(member), "б", 1_700_000_110, None).await;
        let mm = message(&ctx, r, Some(mgr), "в", 1_700_000_120, Some(55)).await;
        let md = message(&ctx, r, Some(adm), "г", 1_700_000_130, None).await;
        let mx = message(&ctx, r, Some(99_999), "д", 1_700_000_140, None).await;

        let feed = ctx.chat.feed(&ident(mgr, Role::Manager), r).await.unwrap();
        let role = |id: i64| msg(&feed.items, &format!("m{id}")).author_role.clone();
        assert_eq!(role(ma).as_deref(), Some("requester"));
        assert_eq!(role(mp).as_deref(), Some("participant"));
        assert_eq!(role(mm).as_deref(), Some("manager"));
        assert_eq!(role(md).as_deref(), Some("admin"));
        // Deleted user: neither name nor role.
        let gone = msg(&feed.items, &format!("m{mx}"));
        assert!(gone.actor_name.is_none());
        assert!(gone.author_role.is_none());
        // Own flag only for the caller.
        assert!(msg(&feed.items, &format!("m{mm}")).is_own);
        assert!(!msg(&feed.items, &format!("m{ma}")).is_own);
        assert_eq!(
            msg(&feed.items, &format!("m{mm}")).moved_from_request_id,
            Some(55)
        );
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_feed_max_size_matches_attachments() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let r = seed_request(&ctx, author, "free_form", "open").await;
        let feed = ctx
            .chat
            .feed(&ident(author, Role::Employee), r)
            .await
            .unwrap();
        assert_eq!(
            feed.max_size_bytes,
            ctx.attachments.effective_limit().await.unwrap()
        );
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_feed_first_unread() {
    timed!(async {
        let (ctx, _dir) = fixture::make_test_ctx().await;
        let author = seed_user(&ctx, "ivanov", "Иванов И.И.", "employee").await;
        let mgr = seed_user(&ctx, "petrov", "Петров П.П.", "manager").await;
        let r = seed_request(&ctx, author, "repair", "open").await;
        let who = ident(author, Role::Employee);
        let own = message(&ctx, r, Some(author), "мой", 1_700_000_100, None).await;
        let m1 = message(&ctx, r, Some(mgr), "первый", 1_700_000_110, None).await;
        let feed = ctx.chat.feed(&who, r).await.unwrap();
        assert_eq!(feed.first_unread_id, Some(format!("m{m1}")), "own skipped");

        ctx.chat.mark_read(&who, r).await.unwrap();
        let feed = ctx.chat.feed(&who, r).await.unwrap();
        assert_eq!(feed.first_unread_id, None);

        let m2 = message(&ctx, r, Some(mgr), "второй", 1_700_000_120, None).await;
        let feed = ctx.chat.feed(&who, r).await.unwrap();
        assert_eq!(feed.first_unread_id, Some(format!("m{m2}")));
        let _ = own;

        // Staff identity without a user id has no read mark.
        let feed = ctx.chat.feed(&Identity::trusted_admin(), r).await.unwrap();
        assert_eq!(feed.first_unread_id, None);
    })
}
