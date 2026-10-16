//! 控制台登录会话：空闲期滑动续期 + 绝对期上限。
//!
//! 约定：`sessions.expires_at` 始终是“当前生效的到期时间”，续期时取 `min(现在 + 空闲期, absolute_expires_at)`，
//! 所以其它按 `expires_at` 判断会话有效性的查询无需知道绝对期。

mod common;

use axum::{Router, http::StatusCode};
use chrono::{DateTime, Duration, Utc};
use common::{
    ADMIN_PASSWORD, admin_session, initialize_admin, json_request, response_json, test_app,
};
use serde_json::json;
use sqlx::SqlitePool;

const DAY: i64 = 24 * 60 * 60;

struct SessionRow {
    expires_at: DateTime<Utc>,
    absolute_expires_at: Option<DateTime<Utc>>,
    last_seen_at: Option<DateTime<Utc>>,
}

fn parse(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

async fn session_row(pool: &SqlitePool) -> SessionRow {
    let (expires, absolute, last_seen): (String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT expires_at, absolute_expires_at, last_seen_at FROM sessions ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    SessionRow {
        expires_at: parse(&expires),
        absolute_expires_at: absolute.as_deref().map(parse),
        last_seen_at: last_seen.as_deref().map(parse),
    }
}

async fn set_times(
    pool: &SqlitePool,
    expires: DateTime<Utc>,
    absolute: Option<DateTime<Utc>>,
    last_seen: Option<DateTime<Utc>>,
) {
    sqlx::query("UPDATE sessions SET expires_at = ?, absolute_expires_at = ?, last_seen_at = ?")
        .bind(expires.to_rfc3339())
        .bind(absolute.map(|time| time.to_rfc3339()))
        .bind(last_seen.map(|time| time.to_rfc3339()))
        .execute(pool)
        .await
        .unwrap();
}

/// 响应里重发的会话 Cookie（没有则为 None）。
fn refreshed_cookie(response: &axum::http::Response<axum::body::Body>) -> Option<String> {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with("deploy_go_session="))
        .map(str::to_owned)
}

fn max_age(cookie: &str) -> i64 {
    cookie
        .split(';')
        .find_map(|part| part.trim().strip_prefix("Max-Age="))
        .expect("cookie 必须带 Max-Age")
        .parse()
        .unwrap()
}

fn assert_close(actual: DateTime<Utc>, expected: DateTime<Utc>, context: &str) {
    let drift = (actual - expected).num_seconds().abs();
    assert!(drift <= 120, "{context}：期望 {expected}，实际 {actual}");
}

async fn me(app: Router, cookie: &str) -> axum::http::Response<axum::body::Body> {
    json_request(
        app,
        "GET",
        "/api/v1/auth/me",
        json!({}),
        &[("cookie", cookie)],
    )
    .await
}

#[tokio::test]
async fn login_issues_default_idle_and_absolute_expiry_and_matching_cookie_lifetime() {
    let (app, pool) = test_app().await;
    initialize_admin(app.clone()).await;
    let response = json_request(
        app,
        "POST",
        "/api/v1/auth/login",
        json!({"username":"admin","password":ADMIN_PASSWORD}),
        &[("origin", "http://localhost")],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = refreshed_cookie(&response).unwrap();
    let now = Utc::now();
    let row = session_row(&pool).await;
    assert_close(row.expires_at, now + Duration::days(7), "空闲期默认 7 天");
    assert_close(
        row.absolute_expires_at.unwrap(),
        now + Duration::days(30),
        "绝对期默认 30 天",
    );
    assert_close(
        row.last_seen_at.unwrap(),
        now,
        "登录即视为刚续期，避免立刻再写一次",
    );
    assert!(
        (max_age(&cookie) - 7 * DAY).abs() <= 120,
        "Cookie 寿命必须与空闲期一致：{cookie}"
    );
}

async fn patch_session_settings(
    app: &Router,
    cookie: &str,
    csrf: &str,
    version: i64,
    idle: i64,
    absolute: i64,
) {
    let patched = json_request(
        app.clone(),
        "PATCH",
        "/api/v1/settings",
        json!({"max_concurrent_deployments":2,"max_log_bytes":52428800,"log_retention_days":30,
               "session_idle_days":idle,"session_absolute_days":absolute,"version":version}),
        &[("cookie", cookie), ("x-csrf-token", csrf)],
    )
    .await;
    assert_eq!(patched.status(), StatusCode::OK);
}

#[tokio::test]
async fn login_uses_the_configured_lifetimes() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    patch_session_settings(&app, &cookie, &csrf, 1, 2, 5).await;
    sqlx::query("DELETE FROM sessions")
        .execute(&pool)
        .await
        .unwrap();

    let response = json_request(
        app,
        "POST",
        "/api/v1/auth/login",
        json!({"username":"admin","password":ADMIN_PASSWORD}),
        &[("origin", "http://localhost")],
    )
    .await;
    let now = Utc::now();
    let row = session_row(&pool).await;
    assert_close(row.expires_at, now + Duration::days(2), "按设置的空闲期");
    assert_close(
        row.absolute_expires_at.unwrap(),
        now + Duration::days(5),
        "按设置的绝对期",
    );
    assert!((max_age(&refreshed_cookie(&response).unwrap()) - 2 * DAY).abs() <= 120);
}

#[tokio::test]
async fn login_caps_idle_at_absolute_even_if_stored_settings_are_inconsistent() {
    // PATCH 会拒绝“绝对期 < 空闲期”，但存储里的值可能来自手工修改或旧版本：登录仍不得让生效到期晚于绝对期。
    let (app, pool) = test_app().await;
    initialize_admin(app.clone()).await;
    sqlx::query("INSERT INTO system_settings(key,value_json,version) VALUES('runtime',?,1)")
        .bind(
            json!({"max_concurrent_deployments":2,"max_log_bytes":52428800,"log_retention_days":30,
                     "session_idle_days":10,"session_absolute_days":3,"version":1})
            .to_string(),
        )
        .execute(&pool)
        .await
        .unwrap();
    let (_, _) = common::login(app, "admin", ADMIN_PASSWORD).await;
    let row = session_row(&pool).await;
    assert_eq!(row.expires_at, row.absolute_expires_at.unwrap());
    assert_close(
        row.expires_at,
        Utc::now() + Duration::days(3),
        "被绝对期封顶",
    );
}

#[tokio::test]
async fn unreadable_settings_still_allow_login_but_only_with_the_shortest_session() {
    // 设置读不出来时不能把管理员锁在外面（否则连设置页都进不去），但也不能签出比配置更长的会话：退到下限 1 天。
    let (app, pool) = test_app().await;
    initialize_admin(app.clone()).await;
    sqlx::query(
        "INSERT INTO system_settings(key,value_json,version) VALUES('runtime','not json',1)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let (_, _) = common::login(app, "admin", ADMIN_PASSWORD).await;
    let now = Utc::now();
    let row = session_row(&pool).await;
    assert_close(row.expires_at, now + Duration::days(1), "最短空闲期");
    assert_close(
        row.absolute_expires_at.unwrap(),
        now + Duration::days(1),
        "最短绝对期",
    );
}

#[tokio::test]
async fn activity_slides_expiry_reissues_the_cookie_once_and_is_throttled() {
    let (app, pool) = test_app().await;
    let (cookie, _) = admin_session(app.clone()).await;
    let now = Utc::now();
    // 模拟“已经用了一天、两小时前续过期”。
    set_times(
        &pool,
        now + Duration::days(1),
        Some(now + Duration::days(30)),
        Some(now - Duration::hours(2)),
    )
    .await;

    let first = me(app.clone(), &cookie).await;
    assert_eq!(first.status(), StatusCode::OK);
    let reissued =
        refreshed_cookie(&first).expect("到期续期后必须重发 Cookie，否则浏览器照样在原时间丢弃它");
    assert!(
        reissued.starts_with(&format!("{cookie};")),
        "必须是同一个令牌：{reissued}"
    );
    for attribute in ["HttpOnly", "SameSite=Lax", "Secure", "Path=/"] {
        assert!(
            reissued.contains(attribute),
            "重发的 Cookie 缺少 {attribute}：{reissued}"
        );
    }
    assert!((max_age(&reissued) - 7 * DAY).abs() <= 120, "{reissued}");
    let renewed = session_row(&pool).await;
    assert_close(
        renewed.expires_at,
        Utc::now() + Duration::days(7),
        "续到空闲期",
    );
    assert_close(renewed.last_seen_at.unwrap(), Utc::now(), "记录续期时间");

    // 节流：紧接着再请求，既不写库也不重发 Cookie（管理页每 10 秒轮询一次遥测）。
    for _ in 0..3 {
        let again = me(app.clone(), &cookie).await;
        assert_eq!(again.status(), StatusCode::OK);
        assert!(
            refreshed_cookie(&again).is_none(),
            "节流窗口内不得重发 Cookie"
        );
    }
    let unchanged = session_row(&pool).await;
    assert_eq!(unchanged.expires_at, renewed.expires_at);
    assert_eq!(unchanged.last_seen_at, renewed.last_seen_at);
}

#[tokio::test]
async fn renewal_is_capped_by_the_absolute_expiry_and_stops_there() {
    let (app, pool) = test_app().await;
    let (cookie, _) = admin_session(app.clone()).await;
    let now = Utc::now();
    let absolute = now + Duration::days(2);
    set_times(
        &pool,
        now + Duration::days(1),
        Some(absolute),
        Some(now - Duration::hours(2)),
    )
    .await;

    let first = me(app.clone(), &cookie).await;
    assert_eq!(first.status(), StatusCode::OK);
    let reissued = refreshed_cookie(&first).unwrap();
    let capped = session_row(&pool).await;
    assert_eq!(
        capped.expires_at, absolute,
        "续期不得越过绝对期，否则被盗的 Cookie 只要常用就永不过期"
    );
    assert!(
        max_age(&reissued) <= 2 * DAY,
        "Cookie 寿命也不得超过绝对期：{reissued}"
    );

    // 已经顶到上限：即便超过节流窗口也不再续期、不再重发。
    sqlx::query("UPDATE sessions SET last_seen_at = ?")
        .bind((Utc::now() - Duration::hours(3)).to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    let second = me(app, &cookie).await;
    assert_eq!(second.status(), StatusCode::OK);
    assert!(refreshed_cookie(&second).is_none());
    assert_eq!(session_row(&pool).await.expires_at, absolute);
}

#[tokio::test]
async fn legacy_sessions_without_an_absolute_expiry_keep_their_fixed_expiry() {
    let (app, pool) = test_app().await;
    let (cookie, _) = admin_session(app.clone()).await;
    let expires = Utc::now() + Duration::hours(3);
    set_times(&pool, expires, None, None).await;

    let response = me(app, &cookie).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        refreshed_cookie(&response).is_none(),
        "升级前签发的会话不续期"
    );
    assert_eq!(session_row(&pool).await.expires_at, expires);
}

#[tokio::test]
async fn expired_and_revoked_sessions_are_rejected_and_never_renewed() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let now = Utc::now();

    set_times(
        &pool,
        now - Duration::seconds(1),
        Some(now + Duration::days(30)),
        None,
    )
    .await;
    let expired = me(app.clone(), &cookie).await;
    assert_eq!(expired.status(), StatusCode::UNAUTHORIZED);
    assert!(
        refreshed_cookie(&expired).is_none(),
        "过期会话不能被续期复活"
    );
    assert!(session_row(&pool).await.expires_at < Utc::now());

    set_times(
        &pool,
        now + Duration::days(7),
        Some(now + Duration::days(30)),
        Some(now),
    )
    .await;
    let logout = json_request(
        app.clone(),
        "POST",
        "/api/v1/auth/logout",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);
    sqlx::query("UPDATE sessions SET last_seen_at = ?")
        .bind((now - Duration::hours(3)).to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    let revoked = me(app, &cookie).await;
    assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED, "作废后立即失效");
    assert!(refreshed_cookie(&revoked).is_none());
}

#[tokio::test]
async fn concurrent_due_requests_renew_and_reissue_exactly_once() {
    let (app, pool) = test_app().await;
    let (cookie, _) = admin_session(app.clone()).await;
    let now = Utc::now();
    set_times(
        &pool,
        now + Duration::days(1),
        Some(now + Duration::days(30)),
        Some(now - Duration::hours(2)),
    )
    .await;

    let (a, b, c) = tokio::join!(
        me(app.clone(), &cookie),
        me(app.clone(), &cookie),
        me(app.clone(), &cookie)
    );
    for response in [&a, &b, &c] {
        assert_eq!(response.status(), StatusCode::OK);
    }
    let reissued = [&a, &b, &c]
        .into_iter()
        .filter(|response| refreshed_cookie(response).is_some())
        .count();
    assert_eq!(
        reissued, 1,
        "条件更新保证并发请求里只有写成功的那个重发 Cookie"
    );
}

#[tokio::test]
async fn purge_removes_only_sessions_expired_or_revoked_more_than_a_week_ago() {
    let (app, pool) = test_app().await;
    let (_cookie, _) = admin_session(app).await;
    let now = Utc::now();
    let user_id: String = sqlx::query_scalar("SELECT id FROM users LIMIT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    let insert = |id: &'static str, expires: DateTime<Utc>, revoked: Option<DateTime<Utc>>| {
        let pool = pool.clone();
        let user_id = user_id.clone();
        async move {
            sqlx::query(
                "INSERT INTO sessions (id, user_id, token_hash, csrf_hash, expires_at, revoked_at) VALUES (?, ?, randomblob(32), randomblob(32), ?, ?)",
            )
            .bind(id)
            .bind(user_id)
            .bind(expires.to_rfc3339())
            .bind(revoked.map(|time| time.to_rfc3339()))
            .execute(&pool)
            .await
            .unwrap();
        }
    };
    insert("ses_stale_expired", now - Duration::days(10), None).await;
    insert(
        "ses_stale_revoked",
        now + Duration::days(3),
        Some(now - Duration::days(10)),
    )
    .await;
    insert("ses_recent_expired", now - Duration::days(1), None).await;
    insert(
        "ses_recent_revoked",
        now + Duration::days(3),
        Some(now - Duration::days(1)),
    )
    .await;
    sqlx::query("INSERT INTO session_csrf_tokens (id, session_id, token_hash, created_at) VALUES ('csrf_stale', 'ses_stale_expired', randomblob(32), ?)")
        .bind(now.to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

    let removed = deploy_go_api::auth::purge_stale_sessions(&pool)
        .await
        .unwrap();
    assert_eq!(removed, 2);
    let remaining: Vec<String> = sqlx::query_scalar("SELECT id FROM sessions ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(
        remaining.len(),
        3,
        "活跃会话和一周内过期/作废的会话必须保留：{remaining:?}"
    );
    assert!(remaining.contains(&"ses_recent_expired".to_owned()));
    assert!(remaining.contains(&"ses_recent_revoked".to_owned()));
    let orphans: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM session_csrf_tokens WHERE id = 'csrf_stale'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(orphans, 0, "从属的 CSRF 令牌随会话级联删除");
    assert_eq!(
        deploy_go_api::auth::purge_stale_sessions(&pool)
            .await
            .unwrap(),
        0,
        "幂等"
    );
}

#[tokio::test]
async fn settings_expose_defaults_validate_and_preserve_omitted_session_fields() {
    let (app, _pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let headers = [("cookie", cookie.as_str()), ("x-csrf-token", csrf.as_str())];

    let shown = response_json(
        json_request(app.clone(), "GET", "/api/v1/settings", json!({}), &headers).await,
    )
    .await;
    assert_eq!(shown["session_idle_days"], 7);
    assert_eq!(shown["session_absolute_days"], 30);

    let base = |version: i64| json!({"max_concurrent_deployments":2,"max_log_bytes":52428800,"log_retention_days":30,"version":version});
    for (idle, absolute) in [(0, 30), (31, 90), (5, 3), (7, 366)] {
        let mut payload = base(1);
        payload["session_idle_days"] = json!(idle);
        payload["session_absolute_days"] = json!(absolute);
        let rejected =
            json_request(app.clone(), "PATCH", "/api/v1/settings", payload, &headers).await;
        assert_eq!(
            rejected.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "idle={idle} absolute={absolute}"
        );
    }

    let mut payload = base(1);
    payload["session_idle_days"] = json!(3);
    payload["session_absolute_days"] = json!(10);
    let updated = json_request(app.clone(), "PATCH", "/api/v1/settings", payload, &headers).await;
    assert_eq!(updated.status(), StatusCode::OK);

    // 旧客户端不传这两个字段：保持当前值，不能被悄悄改回默认。
    let legacy = json_request(app, "PATCH", "/api/v1/settings", base(2), &headers).await;
    assert_eq!(legacy.status(), StatusCode::OK);
    let body = response_json(legacy).await;
    assert_eq!(body["session_idle_days"], 3);
    assert_eq!(body["session_absolute_days"], 10);
}

#[tokio::test]
async fn logout_clears_the_cookie_and_never_reissues_a_session() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let now = Utc::now();
    // 会话正好到了续期时间：提取器会续期，但登出响应里只能出现“清除 Cookie”，不能再给一个长寿命的。
    set_times(
        &pool,
        now + Duration::days(1),
        Some(now + Duration::days(30)),
        Some(now - Duration::hours(2)),
    )
    .await;

    let logout = json_request(
        app,
        "POST",
        "/api/v1/auth/logout",
        json!({}),
        &[("cookie", &cookie), ("x-csrf-token", &csrf)],
    )
    .await;
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);
    let session_cookies: Vec<&str> = logout
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter(|value| value.starts_with("deploy_go_session="))
        .collect();
    assert_eq!(session_cookies.len(), 1, "{session_cookies:?}");
    assert!(
        session_cookies[0].starts_with("deploy_go_session=;"),
        "必须清空值：{}",
        session_cookies[0]
    );
    assert_eq!(max_age(session_cookies[0]), 0);
}

#[tokio::test]
async fn renewal_adopts_the_current_idle_setting_in_both_directions() {
    let (app, pool) = test_app().await;
    let (cookie, csrf) = admin_session(app.clone()).await;
    let now = Utc::now();
    let due = |expires: DateTime<Utc>| {
        let pool = pool.clone();
        async move {
            set_times(
                &pool,
                expires,
                Some(Utc::now() + Duration::days(30)),
                Some(Utc::now() - Duration::hours(2)),
            )
            .await;
        }
    };

    // 调小：管理员收紧空闲期后，活跃会话下一次续期就采用新值（原来的 6 天被缩短到 2 天），收紧才真正生效。
    patch_session_settings(&app, &cookie, &csrf, 1, 2, 30).await;
    due(now + Duration::days(6)).await;
    let shrunk = me(app.clone(), &cookie).await;
    assert_eq!(shrunk.status(), StatusCode::OK);
    assert_close(
        session_row(&pool).await.expires_at,
        Utc::now() + Duration::days(2),
        "缩短到新的空闲期",
    );
    assert!((max_age(&refreshed_cookie(&shrunk).unwrap()) - 2 * DAY).abs() <= 120);

    // 调大：下一次续期顺延到新值。
    patch_session_settings(&app, &cookie, &csrf, 2, 20, 30).await;
    due(now + Duration::days(2)).await;
    let grown = me(app, &cookie).await;
    assert_eq!(grown.status(), StatusCode::OK);
    assert_close(
        session_row(&pool).await.expires_at,
        Utc::now() + Duration::days(20),
        "顺延到新的空闲期",
    );
}

#[tokio::test]
async fn unreadable_settings_skip_renewal_instead_of_guessing() {
    let (app, pool) = test_app().await;
    let (cookie, _) = admin_session(app.clone()).await;
    let now = Utc::now();
    let expires = now + Duration::days(1);
    set_times(
        &pool,
        expires,
        Some(now + Duration::days(30)),
        Some(now - Duration::hours(2)),
    )
    .await;
    sqlx::query(
        "INSERT INTO system_settings(key,value_json,version) VALUES('runtime','not json',1)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let response = me(app, &cookie).await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "续期失败不能影响本次请求"
    );
    assert!(refreshed_cookie(&response).is_none());
    assert_eq!(session_row(&pool).await.expires_at, expires);
}

#[tokio::test]
async fn purge_batches_through_more_rows_than_one_batch() {
    let (app, pool) = test_app().await;
    let (_cookie, _) = admin_session(app).await;
    let user_id: String = sqlx::query_scalar("SELECT id FROM users LIMIT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    let stale = (Utc::now() - Duration::days(30)).to_rfc3339();
    for index in 0..1_200 {
        sqlx::query("INSERT INTO sessions (id, user_id, token_hash, csrf_hash, expires_at) VALUES (?, ?, randomblob(32), randomblob(32), ?)")
            .bind(format!("ses_bulk_{index}"))
            .bind(&user_id)
            .bind(&stale)
            .execute(&pool)
            .await
            .unwrap();
    }
    assert_eq!(
        deploy_go_api::auth::purge_stale_sessions(&pool)
            .await
            .unwrap(),
        1_200
    );
    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, 1, "只剩活跃的那一个");
}

#[tokio::test]
async fn insecure_cookie_deployments_reissue_without_the_secure_attribute() {
    let (app, pool) = common::test_app_with_cookie_secure(false).await;
    let (cookie, _) = admin_session(app.clone()).await;
    let now = Utc::now();
    set_times(
        &pool,
        now + Duration::days(1),
        Some(now + Duration::days(30)),
        Some(now - Duration::hours(2)),
    )
    .await;
    let response = me(app, &cookie).await;
    let reissued = refreshed_cookie(&response).unwrap();
    assert!(
        !reissued.contains("Secure"),
        "本地联调不启用 Secure，重发也必须一致：{reissued}"
    );
    assert!(reissued.contains("HttpOnly") && reissued.contains("SameSite=Lax"));
}
