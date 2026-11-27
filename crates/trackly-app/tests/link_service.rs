//! Ядро Link: коды, устройства, входы, отзыв (Phase 44-06). Имена вымышленные.

mod im_support;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use im_support::{build_ctx, seed_user};
use rusqlite::params;
use trackly_app::context::AppCtx;
use trackly_app::services::link_service::{
    DeviceError, DeviceMeta, EnrollError, LoginMethod, ResumeError,
};
use trackly_infra::error_conversions::map_rusqlite;

fn meta(id: &str) -> DeviceMeta {
    DeviceMeta {
        device_id: id.to_string(),
        hostname: "PC-TEST-01".into(),
        os: "windows".into(),
        arch: "x86_64".into(),
        client_version: "1.0.0".into(),
        ip: Some("192.0.2.10".into()),
    }
}

/// Выпускает код и возвращает (id кода, секрет для enroll, сам код).
async fn invite(ctx: &AppCtx, admin: i64) -> (i64, String, String) {
    let issued = ctx
        .link
        .create_invite("trackly.example.test", 8443, [7u8; 32], 3600, admin)
        .await
        .unwrap();
    let decoded = link_proto::InviteCode::decode(&issued.code).unwrap();
    (
        issued.id,
        URL_SAFE_NO_PAD.encode(decoded.secret),
        issued.code,
    )
}

/// Регистрирует устройство и возвращает его секрет.
async fn enrolled(ctx: &AppCtx, admin: i64, dev: &str) -> String {
    let (_, secret, _) = invite(ctx, admin).await;
    ctx.link.enroll(&secret, meta(dev)).await.unwrap()
}

async fn sql_exec(ctx: &AppCtx, sql: &'static str, p: i64) {
    ctx.writer
        .execute(move |c| {
            c.execute(sql, params![p]).map_err(map_rusqlite)?;
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn enroll_redeems_once() {
    let (ctx, _d) = build_ctx().await;
    let adm = seed_user(&ctx, "lk_adm1", "Смирнов С.С.", "admin").await;
    let (_, secret, _) = invite(&ctx, adm).await;
    let s = ctx.link.enroll(&secret, meta("dev-1")).await.unwrap();
    assert!(!s.is_empty());
    let again = ctx.link.enroll(&secret, meta("dev-2")).await;
    assert!(matches!(again, Err(EnrollError::InviteUsed)), "{again:?}");
}

#[tokio::test]
async fn invalid_expired_revoked_codes() {
    let (ctx, _d) = build_ctx().await;
    let adm = seed_user(&ctx, "lk_adm2", "Смирнов С.С.", "admin").await;

    let bogus = URL_SAFE_NO_PAD.encode([5u8; 16]);
    let r = ctx.link.enroll(&bogus, meta("dev-x")).await;
    assert!(matches!(r, Err(EnrollError::InviteInvalid)), "{r:?}");
    let r = ctx.link.enroll("не-base64!", meta("dev-x")).await;
    assert!(matches!(r, Err(EnrollError::InviteInvalid)), "{r:?}");

    let (id, secret, _) = invite(&ctx, adm).await;
    sql_exec(
        &ctx,
        "UPDATE link_invites SET expires_at_utc = 1 WHERE id = ?1",
        id,
    )
    .await;
    let r = ctx.link.enroll(&secret, meta("dev-e")).await;
    assert!(matches!(r, Err(EnrollError::InviteExpired)), "{r:?}");

    let (id, secret, _) = invite(&ctx, adm).await;
    ctx.link.revoke_invite(id, adm).await.unwrap();
    let r = ctx.link.enroll(&secret, meta("dev-r")).await;
    assert!(matches!(r, Err(EnrollError::InviteRevoked)), "{r:?}");
}

#[tokio::test]
async fn concurrent_redeem_exactly_one_wins() {
    let (ctx, _d) = build_ctx().await;
    let adm = seed_user(&ctx, "lk_adm3", "Смирнов С.С.", "admin").await;
    let (_, secret, _) = invite(&ctx, adm).await;
    let (a, b) = tokio::join!(
        ctx.link.enroll(&secret, meta("dev-a")),
        ctx.link.enroll(&secret, meta("dev-b")),
    );
    let ok = [a.is_ok(), b.is_ok()].iter().filter(|x| **x).count();
    assert_eq!(ok, 1, "{a:?} / {b:?}");
    let loser = if a.is_ok() { b } else { a };
    assert!(matches!(loser, Err(EnrollError::InviteUsed)), "{loser:?}");
}

#[tokio::test]
async fn existing_device_is_not_overwritten() {
    let (ctx, _d) = build_ctx().await;
    let adm = seed_user(&ctx, "lk_adm3b", "Смирнов С.С.", "admin").await;
    let secret1 = enrolled(&ctx, adm, "dev-same").await;
    let (_, code2, _) = invite(&ctx, adm).await;
    let r = ctx.link.enroll(&code2, meta("dev-same")).await;
    assert!(matches!(r, Err(EnrollError::DeviceExists)), "{r:?}");
    // Старый секрет жив, код не сожжён.
    ctx.link.verify_device("dev-same", &secret1).await.unwrap();
    ctx.link.enroll(&code2, meta("dev-other")).await.unwrap();
}

#[tokio::test]
async fn only_hashes_stored() {
    let (ctx, _d) = build_ctx().await;
    let adm = seed_user(&ctx, "lk_adm4", "Смирнов С.С.", "admin").await;
    let (_, secret, _) = invite(&ctx, adm).await;
    let dsecret = ctx.link.enroll(&secret, meta("dev-h")).await.unwrap();
    let uid = seed_user(&ctx, "lk_u4", "Иванов И.И.", "employee").await;
    let issued = ctx
        .link
        .record_login("dev-h", uid, LoginMethod::Password)
        .await
        .unwrap();
    let token = issued.device_token.unwrap();

    let (code_h, sec_h, tok_h): (Vec<u8>, Vec<u8>, Vec<u8>) = ctx
        .writer
        .execute(|c| {
            let code: Vec<u8> = c
                .query_row("SELECT code_hash FROM link_invites", [], |r| r.get(0))
                .map_err(map_rusqlite)?;
            let sec: Vec<u8> = c
                .query_row("SELECT secret_hash FROM link_devices", [], |r| r.get(0))
                .map_err(map_rusqlite)?;
            let tok: Vec<u8> = c
                .query_row("SELECT token_hash FROM link_logins", [], |r| r.get(0))
                .map_err(map_rusqlite)?;
            Ok((code, sec, tok))
        })
        .await
        .unwrap();
    for (h, open) in [
        (&code_h, URL_SAFE_NO_PAD.decode(&secret).unwrap()),
        (&sec_h, dsecret.clone().into_bytes()),
        (&tok_h, token.clone().into_bytes()),
    ] {
        assert_eq!(h.len(), 32);
        assert_ne!(h, &open);
    }
    // 256 бит: 32 байта -> 43 символа base64url.
    assert_eq!(dsecret.len(), 43);
    assert_eq!(token.len(), 43);
}

#[tokio::test]
async fn verify_device_outcomes() {
    let (ctx, _d) = build_ctx().await;
    let adm = seed_user(&ctx, "lk_adm5", "Смирнов С.С.", "admin").await;
    let uid = seed_user(&ctx, "lk_u5", "Иванов И.И.", "employee").await;
    let sec = enrolled(&ctx, adm, "dev-v").await;
    ctx.link.verify_device("dev-v", &sec).await.unwrap();
    assert!(matches!(
        ctx.link.verify_device("dev-v", "wrong").await,
        Err(DeviceError::BadSecret)
    ));
    assert!(matches!(
        ctx.link.verify_device("dev-none", &sec).await,
        Err(DeviceError::Unknown)
    ));
    let login = ctx
        .link
        .record_login("dev-v", uid, LoginMethod::Sso)
        .await
        .unwrap();
    ctx.link.revoke_login(login.login_id, adm).await.unwrap();
    assert!(matches!(
        ctx.link.verify_device("dev-v", &sec).await,
        Err(DeviceError::Banned)
    ));
    // Неверный секрет запрещённого устройства не раскрывает запрет.
    assert!(matches!(
        ctx.link.verify_device("dev-v", "wrong").await,
        Err(DeviceError::BadSecret)
    ));
}

#[tokio::test]
async fn password_login_resume() {
    let (ctx, _d) = build_ctx().await;
    let adm = seed_user(&ctx, "lk_adm6", "Смирнов С.С.", "admin").await;
    let uid = seed_user(&ctx, "lk_u6", "Иванов И.И.", "employee").await;
    enrolled(&ctx, adm, "dev-p").await;
    enrolled(&ctx, adm, "dev-q").await;
    let issued = ctx
        .link
        .record_login("dev-p", uid, LoginMethod::Password)
        .await
        .unwrap();
    let token = issued.device_token.clone().unwrap();
    let row = ctx.link.resume_by_token(&token, "dev-p").await.unwrap();
    assert_eq!((row.login_id, row.user_id), (issued.login_id, uid));
    assert!(matches!(
        ctx.link.resume_by_token(&token, "dev-q").await,
        Err(ResumeError::Invalid)
    ));
    assert!(matches!(
        ctx.link.resume_by_token("нет-такого", "dev-p").await,
        Err(ResumeError::Invalid)
    ));
    // SSO-вход токена не получает.
    let sso = ctx
        .link
        .record_login("dev-q", uid, LoginMethod::Sso)
        .await
        .unwrap();
    assert!(sso.device_token.is_none());
    // Обычный выход: вход отозван, устройство не запрещено.
    ctx.link.logout(issued.login_id).await.unwrap();
    assert!(matches!(
        ctx.link.resume_by_token(&token, "dev-p").await,
        Err(ResumeError::Revoked)
    ));
    assert!(ctx.link.list_banned_devices().await.unwrap().is_empty());
}

#[tokio::test]
async fn revoke_login_bans_device_and_allow_lifts() {
    let (ctx, _d) = build_ctx().await;
    let adm = seed_user(&ctx, "lk_adm7", "Смирнов С.С.", "admin").await;
    let uid = seed_user(&ctx, "lk_u7", "Иванов И.И.", "employee").await;
    let sec = enrolled(&ctx, adm, "dev-b").await;
    let issued = ctx
        .link
        .record_login("dev-b", uid, LoginMethod::Password)
        .await
        .unwrap();
    let token = issued.device_token.unwrap();
    ctx.link.revoke_login(issued.login_id, adm).await.unwrap();
    assert!(matches!(
        ctx.link.verify_device("dev-b", &sec).await,
        Err(DeviceError::Banned)
    ));
    assert_eq!(ctx.link.list_banned_devices().await.unwrap().len(), 1);
    ctx.link.allow_device("dev-b").await.unwrap();
    ctx.link.verify_device("dev-b", &sec).await.unwrap();
    // Отозванный вход остаётся отозванным.
    assert!(matches!(
        ctx.link.resume_by_token(&token, "dev-b").await,
        Err(ResumeError::Revoked)
    ));
    assert!(!ctx.link.is_login_valid(issued.login_id).await.unwrap());
}

#[tokio::test]
async fn revoke_user_logins_ban_flag() {
    let (ctx, _d) = build_ctx().await;
    let adm = seed_user(&ctx, "lk_adm8", "Смирнов С.С.", "admin").await;
    let uid = seed_user(&ctx, "lk_u8", "Иванов И.И.", "employee").await;
    let other = seed_user(&ctx, "lk_o8", "Петров П.П.", "employee").await;
    let s1 = enrolled(&ctx, adm, "dev-1").await;
    let s2 = enrolled(&ctx, adm, "dev-2").await;
    enrolled(&ctx, adm, "dev-3").await;
    for d in ["dev-1", "dev-2"] {
        ctx.link
            .record_login(d, uid, LoginMethod::Sso)
            .await
            .unwrap();
    }
    let keep = ctx
        .link
        .record_login("dev-3", other, LoginMethod::Sso)
        .await
        .unwrap();
    assert_eq!(ctx.link.count_active_logins(uid).await.unwrap(), 2);

    // Блокировка пользователя: входы долой, устройства не запрещены.
    assert_eq!(
        ctx.link.revoke_user_logins(uid, adm, false).await.unwrap(),
        2
    );
    assert_eq!(ctx.link.count_active_logins(uid).await.unwrap(), 0);
    ctx.link.verify_device("dev-1", &s1).await.unwrap();

    // Отзыв со запретом устройств.
    ctx.link
        .record_login("dev-1", uid, LoginMethod::Sso)
        .await
        .unwrap();
    ctx.link
        .record_login("dev-2", uid, LoginMethod::Sso)
        .await
        .unwrap();
    assert_eq!(
        ctx.link.revoke_user_logins(uid, adm, true).await.unwrap(),
        2
    );
    assert!(matches!(
        ctx.link.verify_device("dev-1", &s1).await,
        Err(DeviceError::Banned)
    ));
    assert!(matches!(
        ctx.link.verify_device("dev-2", &s2).await,
        Err(DeviceError::Banned)
    ));
    assert!(ctx.link.is_login_valid(keep.login_id).await.unwrap());
}

#[tokio::test]
async fn list_active_logins_shape() {
    let (ctx, _d) = build_ctx().await;
    let adm = seed_user(&ctx, "lk_adm9", "Смирнов С.С.", "admin").await;
    let uid = seed_user(&ctx, "lk_u9", "Иванов И.И.", "employee").await;
    enrolled(&ctx, adm, "dev-l1").await;
    enrolled(&ctx, adm, "dev-l2").await;
    let gone = ctx
        .link
        .record_login("dev-l1", uid, LoginMethod::Password)
        .await
        .unwrap();
    ctx.link
        .record_login("dev-l2", uid, LoginMethod::Sso)
        .await
        .unwrap();
    ctx.link.logout(gone.login_id).await.unwrap();
    let list = ctx.link.list_active_logins().await.unwrap();
    assert_eq!(list.len(), 1);
    let r = &list[0];
    assert_eq!(r.method, "sso");
    assert_eq!(r.full_name.as_deref(), Some("Иванов И.И."));
    assert_eq!(r.user_login.as_deref(), Some("lk_u9"));
    assert_eq!(r.hostname.as_deref(), Some("PC-TEST-01"));
}

#[tokio::test]
async fn is_login_valid_tracks_user_state() {
    let (ctx, _d) = build_ctx().await;
    let adm = seed_user(&ctx, "lk_adm10", "Смирнов С.С.", "admin").await;
    let u1 = seed_user(&ctx, "lk_u10a", "Иванов И.И.", "employee").await;
    let u2 = seed_user(&ctx, "lk_u10b", "Петров П.П.", "employee").await;
    enrolled(&ctx, adm, "dev-i").await;
    let l1 = ctx
        .link
        .record_login("dev-i", u1, LoginMethod::Sso)
        .await
        .unwrap();
    let l2 = ctx
        .link
        .record_login("dev-i", u2, LoginMethod::Sso)
        .await
        .unwrap();
    assert!(ctx.link.is_login_valid(l1.login_id).await.unwrap());
    assert!(!ctx.link.is_login_valid(999_999).await.unwrap());
    sql_exec(&ctx, "UPDATE users SET is_active = 0 WHERE id = ?1", u1).await;
    assert!(!ctx.link.is_login_valid(l1.login_id).await.unwrap());
    sql_exec(
        &ctx,
        "UPDATE users SET deleted_at_utc = 1700000000 WHERE id = ?1",
        u2,
    )
    .await;
    assert!(!ctx.link.is_login_valid(l2.login_id).await.unwrap());
}

#[tokio::test]
async fn revoke_tears_down_hub_after_commit() {
    let (ctx, _d) = build_ctx().await;
    let adm = seed_user(&ctx, "lk_adm11", "Смирнов С.С.", "admin").await;
    let uid = seed_user(&ctx, "lk_u11", "Иванов И.И.", "employee").await;
    enrolled(&ctx, adm, "dev-h1").await;
    enrolled(&ctx, adm, "dev-h2").await;
    let login = ctx
        .link
        .record_login("dev-h1", uid, LoginMethod::Sso)
        .await
        .unwrap();
    let t1 = ctx.link_hub.register("dev-h1", uid);
    let t2 = ctx.link_hub.register("dev-h2", uid);
    // Отзыв несуществующего входа не коммитится — хаб не трогается.
    assert!(ctx.link.revoke_login(999_999, adm).await.is_err());
    assert!(!t1.is_cancelled() && !t2.is_cancelled());
    ctx.link.revoke_login(login.login_id, adm).await.unwrap();
    assert!(t1.is_cancelled());
    assert!(!t2.is_cancelled());
    ctx.link.revoke_user_logins(uid, adm, false).await.unwrap();
    assert!(t2.is_cancelled());
}
