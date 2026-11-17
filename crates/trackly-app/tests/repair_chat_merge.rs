//! Phase 41.5, план 09: объединение заявок переносит переписку (D-09).
//! Данные вымышленные: «Иванов И.И.», «Петров П.П.».

mod entities_support;
mod repair_support;

use entities_support::fixture::{admin, make_test_ctx, seed_device, seed_place};
use repair_support::{manager, seed_user};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::dto::chat::ChatItemDto;
use trackly_app::dto::repair::{RepairCreateDto, RepairMergeDto, RepairMoveChoiceDto};
use trackly_infra::error_conversions::map_rusqlite;
use trackly_infra::repos::attachments_sqlite::{AttachmentNew, SqliteAttachmentRepository};
use trackly_infra::repos::chat_sqlite::{ChatMessageNew, SqliteChatRepository};

struct Fx {
    ctx: AppCtx,
    _dir: tempfile::TempDir,
    ivanov: i64,
    petrov: i64,
    device: i64,
    target: i64,
    source: i64,
    source_version: i64,
}

async fn fixture() -> Fx {
    let (ctx, dir) = make_test_ctx().await;
    let ivanov = seed_user(&ctx, "ivanov", "Иванов И.И.").await;
    let petrov = seed_user(&ctx, "petrov", "Петров П.П.").await;
    let room = seed_place(&ctx, "Кабинет 12").await;
    let device = seed_device(&ctx, 1, "ПК-1", "INV-1", Some(room), None).await;
    let b = ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: Some(device),
                description: "Не включается".into(),
            },
            &manager(petrov),
        )
        .await
        .expect("create B");
    ctx.repairs
        .accept(
            b.request.id,
            b.request.version,
            RepairMoveChoiceDto::Stay,
            &admin(),
        )
        .await
        .expect("accept B");
    let a = ctx
        .repairs
        .create(
            RepairCreateDto {
                device_id: None,
                description: "Шумит и греется".into(),
            },
            &manager(ivanov),
        )
        .await
        .expect("create A");
    let acc = ctx
        .repairs
        .accept(
            a.request.id,
            a.request.version,
            RepairMoveChoiceDto::Stay,
            &admin(),
        )
        .await
        .expect("accept A");
    Fx {
        ctx,
        _dir: dir,
        ivanov,
        petrov,
        device,
        target: b.request.id,
        source: a.request.id,
        source_version: acc.request.version,
    }
}

async fn add_message(fx: &Fx, request: i64, author: i64, body: &'static str, at: i64) -> i64 {
    fx.ctx
        .writer
        .execute(move |conn| {
            SqliteChatRepository.insert_message_on(
                conn,
                &ChatMessageNew {
                    request_id: request,
                    author_user_id: Some(author),
                    body,
                    created_at_utc: at,
                    moved_from_request_id: None,
                },
            )
        })
        .await
        .expect("message")
}

async fn merge(fx: &Fx) {
    fx.ctx
        .repairs
        .merge_into_open(
            RepairMergeDto {
                request_id: fx.source,
                version: fx.source_version,
                device_id: fx.device,
            },
            &manager(fx.petrov),
        )
        .await
        .expect("merge");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_merge_moves_messages() {
    let fx = fixture().await;
    let own = add_message(&fx, fx.target, fx.petrov, "Сообщение цели", 1_700_000_200).await;
    let m1 = add_message(&fx, fx.source, fx.ivanov, "Первое", 1_700_000_100).await;
    let m2 = add_message(&fx, fx.source, fx.ivanov, "Второе", 1_700_000_300).await;
    merge(&fx).await;

    let (src_left, rows) = {
        let (s, t) = (fx.source, fx.target);
        fx.ctx
            .writer
            .execute(move |conn| {
                let left: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM chat_messages WHERE request_id = ?1",
                        params![s],
                        |r| r.get(0),
                    )
                    .map_err(map_rusqlite)?;
                let mut stmt = conn
                    .prepare(
                        "SELECT id, created_at_utc, moved_from_request_id FROM chat_messages \
                         WHERE request_id = ?1 ORDER BY id",
                    )
                    .map_err(map_rusqlite)?;
                let rows = stmt
                    .query_map(params![t], |r| {
                        Ok((
                            r.get::<_, i64>(0)?,
                            r.get::<_, i64>(1)?,
                            r.get::<_, Option<i64>>(2)?,
                        ))
                    })
                    .map_err(map_rusqlite)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(map_rusqlite)?;
                Ok((left, rows))
            })
            .await
            .expect("rows")
    };
    assert_eq!(src_left, 0);
    assert_eq!(
        rows,
        vec![
            (own, 1_700_000_200, None),
            (m1, 1_700_000_100, Some(fx.source)),
            (m2, 1_700_000_300, Some(fx.source)),
        ]
    );

    let feed = fx
        .ctx
        .chat
        .feed(&manager(fx.petrov), fx.target)
        .await
        .expect("feed");
    let times: Vec<i64> = feed
        .items
        .iter()
        .filter_map(|i| match i {
            ChatItemDto::Message(m) => Some(m.created_at_utc),
            _ => None,
        })
        .collect();
    assert_eq!(times, vec![1_700_000_100, 1_700_000_200, 1_700_000_300]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_merge_attachment_marks() {
    let fx = fixture().await;
    let m = add_message(&fx, fx.source, fx.ivanov, "Фото", 1_700_000_100).await;
    let (source, ivanov) = (fx.source, fx.ivanov);
    let att = fx
        .ctx
        .writer
        .execute(move |conn| {
            SqliteAttachmentRepository.insert_on(
                conn,
                &AttachmentNew {
                    parent_kind: "request",
                    parent_id: source,
                    original_name: "фото.png",
                    mime: "image/png",
                    size_bytes: 10,
                    content_hash: "abc123",
                    uploaded_by_user_id: Some(ivanov),
                    created_at_utc: 1_700_000_100,
                    message_id: Some(m),
                },
            )
        })
        .await
        .expect("attachment");
    merge(&fx).await;

    let target = fx.target;
    let (orig, copies) = fx
        .ctx
        .writer
        .execute(move |conn| {
            let orig: Option<i64> = conn
                .query_row(
                    "SELECT message_id FROM attachments WHERE id = ?1",
                    params![att],
                    |r| r.get(0),
                )
                .map_err(map_rusqlite)?;
            let mut stmt = conn
                .prepare(
                    "SELECT message_id FROM attachments WHERE parent_kind = 'request' \
                     AND parent_id = ?1",
                )
                .map_err(map_rusqlite)?;
            let copies = stmt
                .query_map(params![target], |r| r.get::<_, Option<i64>>(0))
                .map_err(map_rusqlite)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_rusqlite)?;
            Ok((orig, copies))
        })
        .await
        .expect("query");
    assert_eq!(orig, None);
    assert_eq!(copies, vec![Some(m)]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_merge_without_messages() {
    let fx = fixture().await;
    merge(&fx).await;
}
