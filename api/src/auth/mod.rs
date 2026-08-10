use std::time::Duration;

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{
    Json, Router,
    extract::{Extension, FromRequestParts, State},
    http::{HeaderMap, StatusCode, header::SET_COOKIE, request::Parts},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration as ChronoDuration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use ulid::Ulid;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId, audit,
    error::{ApiError, ApiResult},
};

const SESSION_COOKIE: &str = "deploy_go_session";
const SESSION_LIFETIME: Duration = Duration::from_secs(12 * 60 * 60);
const MAX_REFRESHED_CSRF_TOKENS: i64 = 31;

#[derive(Clone)]
pub struct AuthUser {
    pub id: String,
    pub username: String,
    pub identity: String,
    pub session_id: String,
    csrf_hashes: Vec<Vec<u8>>,
}

impl AuthUser {
    pub fn require_administrator(&self, request_id: &str) -> ApiResult<()> {
        if self.identity == "administrator" {
            Ok(())
        } else {
            Err(ApiError::forbidden(request_id))
        }
    }

    pub fn verify_csrf(&self, headers: &HeaderMap, request_id: &str) -> ApiResult<()> {
        let Some(value) = headers
            .get("x-csrf-token")
            .and_then(|value| value.to_str().ok())
        else {
            return Err(ApiError::forbidden(request_id));
        };
        self.verify_csrf_token(value, request_id)
    }

    pub(crate) fn verify_csrf_token(&self, value: &str, request_id: &str) -> ApiResult<()> {
        let actual = token_hash(value);
        if self
            .csrf_hashes
            .iter()
            .any(|expected| bool::from(actual.ct_eq(expected)))
        {
            Ok(())
        } else {
            Err(ApiError::forbidden(request_id))
        }
    }
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let request_id = parts
            .extensions
            .get::<RequestId>()
            .map(RequestId::as_str)
            .unwrap_or("req_unknown");
        let token = cookie_value(&parts.headers, SESSION_COOKIE)
            .ok_or_else(|| ApiError::unauthorized(request_id))?;
        let hash = token_hash(&token);
        let user = sqlx::query_as::<_, AuthSession>(
            "SELECT u.id, u.username, u.identity, s.id AS session_id, s.csrf_hash FROM sessions s JOIN users u ON u.id = s.user_id WHERE s.token_hash = ? AND s.revoked_at IS NULL AND s.expires_at > ? AND u.status = 'active' AND u.system_account = 0",
        )
        .bind(hash)
        .bind(Utc::now().to_rfc3339())
        .fetch_optional(state.pool())
        .await
        .map_err(|error| {
            tracing::error!(%error, %request_id, "session lookup failed");
            ApiError::internal(request_id)
        })?;
        let Some(user) = user else {
            return Err(ApiError::unauthorized(request_id));
        };
        let mut csrf_hashes = sqlx::query_scalar::<_, Vec<u8>>(
            "SELECT token_hash FROM session_csrf_tokens WHERE session_id = ?",
        )
        .bind(&user.session_id)
        .fetch_all(state.pool())
        .await
        .map_err(|error| {
            tracing::error!(%error, %request_id, "session CSRF token lookup failed");
            ApiError::internal(request_id)
        })?;
        csrf_hashes.push(user.csrf_hash);
        Ok(AuthUser {
            id: user.id,
            username: user.username,
            identity: user.identity,
            session_id: user.session_id,
            csrf_hashes,
        })
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SetupRequest {
    username: String,
    password: String,
    display_name: Option<String>,
    email: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct SessionResponse {
    user: UserIdentity,
    csrf_token: String,
}

#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct UserIdentity {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub identity: String,
}

#[derive(Serialize, ToSchema)]
pub struct SetupStatusResponse {
    setup_required: bool,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateProfileRequest {
    display_name: String,
}

#[derive(Serialize, ToSchema, sqlx::FromRow)]
pub struct UserPreferencesResponse {
    notify_deployment_failed: bool,
    notify_deployment_completed: bool,
    notify_node_unhealthy: bool,
    time_format: String,
    follow_logs: bool,
    version: i64,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateUserPreferencesRequest {
    notify_deployment_failed: bool,
    notify_deployment_completed: bool,
    notify_node_unhealthy: bool,
    time_format: String,
    follow_logs: bool,
    version: i64,
}

#[derive(Serialize, ToSchema)]
pub struct CsrfTokenResponse {
    csrf_token: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/setup", get(setup_status).post(setup))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route("/auth/profile", get(profile).patch(update_profile))
        .route(
            "/auth/preferences",
            get(preferences).put(update_preferences),
        )
        .route("/auth/csrf", post(refresh_csrf))
}

#[utoipa::path(operation_id = "auth_setup_status", get, path = "/api/v1/setup", responses((status = 200, body = SetupStatusResponse)))]
pub(crate) async fn setup_status(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> ApiResult<Json<SetupStatusResponse>> {
    let setup_required: bool = sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM users WHERE system_account = 0)")
        .fetch_one(state.pool())
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(SetupStatusResponse { setup_required }))
}

#[utoipa::path(operation_id = "auth_setup", post, path = "/api/v1/setup", params(("Origin" = String, Header)), request_body = SetupRequest, responses((status = 201, body = UserIdentity), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn setup(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    crate::http::ApiJson(payload): crate::http::ApiJson<SetupRequest>,
) -> ApiResult<impl IntoResponse> {
    verify_origin(&state, &headers, request_id.as_str())?;
    validate_credentials(&payload.username, &payload.password, request_id.as_str())?;
    let display_name = validate_display_name(
        payload.display_name.as_deref().unwrap_or(&payload.username),
        request_id.as_str(),
    )?;
    let email = validate_optional_email(payload.email.as_deref(), request_id.as_str())?;
    let password_hash = hash_password(&payload.password, request_id.as_str())?;
    let user_id = format!("usr_{}", Ulid::new());
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE system_account = 0")
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if exists > 0 {
        return Err(ApiError::conflict(
            "setup_already_completed",
            "系统初始化已经完成",
            request_id.as_str(),
        ));
    }
    sqlx::query("INSERT INTO users (id, username, password_hash, identity, status, display_name, email) VALUES (?, ?, ?, 'administrator', 'active', ?, ?)")
        .bind(&user_id).bind(payload.username.trim()).bind(password_hash).bind(&display_name).bind(&email)
        .execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&user_id),
        "system.setup",
        "user",
        &user_id,
        request_id.as_str(),
        json!({"identity":"administrator"}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok((
        StatusCode::CREATED,
        Json(UserIdentity {
            id: user_id,
            username: payload.username.trim().to_owned(),
            display_name,
            email,
            identity: "administrator".to_owned(),
        }),
    ))
}

#[utoipa::path(operation_id = "auth_login", post, path = "/api/v1/auth/login", params(("Origin" = String, Header)), request_body = LoginRequest, responses((status = 200, body = SessionResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse)))]
pub(crate) async fn login(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    crate::http::ApiJson(payload): crate::http::ApiJson<LoginRequest>,
) -> ApiResult<Response> {
    verify_origin(&state, &headers, request_id.as_str())?;
    let row = sqlx::query_as::<_, LoginUser>("SELECT id, username, display_name, email, password_hash, identity FROM users WHERE (username = ? COLLATE NOCASE OR email = ? COLLATE NOCASE) AND status = 'active' AND system_account = 0")
        .bind(payload.username.trim()).bind(payload.username.trim()).fetch_optional(state.pool()).await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let Some(user) = row else {
        return Err(ApiError::unauthorized(request_id.as_str()));
    };
    verify_password(&payload.password, &user.password_hash, request_id.as_str())?;
    let session_token = random_token();
    let csrf_token = random_token();
    let session_id = format!("ses_{}", Ulid::new());
    let expires_at =
        (Utc::now() + ChronoDuration::from_std(SESSION_LIFETIME).unwrap()).to_rfc3339();
    sqlx::query("INSERT INTO sessions (id, user_id, token_hash, csrf_hash, expires_at) VALUES (?, ?, ?, ?, ?)")
        .bind(&session_id).bind(&user.id).bind(token_hash(&session_token)).bind(token_hash(&csrf_token)).bind(expires_at)
        .execute(state.pool()).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    let mut response = Json(SessionResponse {
        user: UserIdentity {
            id: user.id,
            username: user.username,
            display_name: user
                .display_name
                .unwrap_or_else(|| payload.username.trim().to_owned()),
            email: user.email,
            identity: user.identity,
        },
        csrf_token,
    })
    .into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        session_cookie(&session_token, state.cookie_secure())
            .parse()
            .expect("valid cookie"),
    );
    Ok(response)
}

#[utoipa::path(operation_id = "auth_me", get, path = "/api/v1/auth/me", responses((status = 200, body = UserIdentity), (status = 401, body = crate::error::ErrorResponse)))]
pub(crate) async fn me(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    user: AuthUser,
) -> ApiResult<Json<UserIdentity>> {
    Ok(Json(
        load_identity(state.pool(), &user.id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "auth_profile", get, path = "/api/v1/auth/profile", responses((status = 200, body = UserIdentity), (status = 401, body = crate::error::ErrorResponse)))]
pub(crate) async fn profile(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    user: AuthUser,
) -> ApiResult<Json<UserIdentity>> {
    Ok(Json(
        load_identity(state.pool(), &user.id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "auth_update_profile", patch, path = "/api/v1/auth/profile", params(("X-CSRF-Token" = String, Header)), request_body = UpdateProfileRequest, responses((status = 200, body = UserIdentity), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update_profile(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    user: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<UpdateProfileRequest>,
) -> ApiResult<Json<UserIdentity>> {
    user.verify_csrf(&headers, request_id.as_str())?;
    let display_name = validate_display_name(&payload.display_name, request_id.as_str())?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    sqlx::query("UPDATE users SET display_name = ?, updated_at = ? WHERE id = ?")
        .bind(display_name)
        .bind(Utc::now().to_rfc3339())
        .bind(&user.id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&user.id),
        "user.profile.update",
        "user",
        &user.id,
        request_id.as_str(),
        json!({"fields":["display_name"]}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(
        load_identity(state.pool(), &user.id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "auth_preferences", get, path = "/api/v1/auth/preferences", responses((status = 200, body = UserPreferencesResponse), (status = 401, body = crate::error::ErrorResponse)))]
pub(crate) async fn preferences(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    user: AuthUser,
) -> ApiResult<Json<UserPreferencesResponse>> {
    Ok(Json(
        load_preferences(state.pool(), &user.id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "auth_update_preferences", put, path = "/api/v1/auth/preferences", params(("X-CSRF-Token" = String, Header)), request_body = UpdateUserPreferencesRequest, responses((status = 200, body = UserPreferencesResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update_preferences(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    user: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<UpdateUserPreferencesRequest>,
) -> ApiResult<Json<UserPreferencesResponse>> {
    user.verify_csrf(&headers, request_id.as_str())?;
    if !matches!(payload.time_format.as_str(), "12h" | "24h") {
        return Err(ApiError::validation("时间格式不正确", request_id.as_str()));
    }
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    sqlx::query(
        "INSERT INTO user_preferences (user_id) VALUES (?) ON CONFLICT(user_id) DO NOTHING",
    )
    .bind(&user.id)
    .execute(&mut *transaction)
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("UPDATE user_preferences SET notify_deployment_failed = ?, notify_deployment_completed = ?, notify_node_unhealthy = ?, time_format = ?, follow_logs = ?, updated_at = ?, version = version + 1 WHERE user_id = ? AND version = ?")
        .bind(payload.notify_deployment_failed)
        .bind(payload.notify_deployment_completed)
        .bind(payload.notify_node_unhealthy)
        .bind(payload.time_format)
        .bind(payload.follow_logs)
        .bind(Utc::now().to_rfc3339())
        .bind(&user.id)
        .bind(payload.version)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if result.rows_affected() == 0 {
        return Err(ApiError::conflict(
            "resource_version_conflict",
            "通知偏好已经被其他请求修改",
            request_id.as_str(),
        ));
    }
    audit::record(
        &mut transaction,
        Some(&user.id),
        "user.preferences.update",
        "user",
        &user.id,
        request_id.as_str(),
        json!({"fields":["notify_deployment_failed","notify_deployment_completed","notify_node_unhealthy","time_format","follow_logs"]}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(
        load_preferences(state.pool(), &user.id, request_id.as_str()).await?,
    ))
}

#[utoipa::path(operation_id = "auth_refresh_csrf", post, path = "/api/v1/auth/csrf", params(("Origin" = String, Header), ("Sec-Fetch-Site" = String, Header), ("Sec-Fetch-Mode" = String, Header)), responses((status = 200, body = CsrfTokenResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse)))]
pub(crate) async fn refresh_csrf(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    user: AuthUser,
) -> ApiResult<Json<CsrfTokenResponse>> {
    verify_origin(&state, &headers, request_id.as_str())?;
    verify_fetch_metadata(&headers, request_id.as_str())?;
    let csrf_token = random_token();
    let token_id = format!("ctk_{}", Ulid::new());
    let result = sqlx::query("INSERT INTO session_csrf_tokens (id, session_id, token_hash, created_at) SELECT ?, id, ?, ? FROM sessions WHERE id = ? AND revoked_at IS NULL AND expires_at > ?")
        .bind(&token_id)
        .bind(token_hash(&csrf_token))
        .bind(Utc::now().to_rfc3339())
        .bind(&user.session_id)
        .bind(Utc::now().to_rfc3339())
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if result.rows_affected() != 1 {
        return Err(ApiError::unauthorized(request_id.as_str()));
    }
    sqlx::query("DELETE FROM session_csrf_tokens WHERE session_id = ? AND id NOT IN (SELECT id FROM session_csrf_tokens WHERE session_id = ? ORDER BY created_at DESC, id DESC LIMIT ?)")
        .bind(&user.session_id)
        .bind(&user.session_id)
        .bind(MAX_REFRESHED_CSRF_TOKENS)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(CsrfTokenResponse { csrf_token }))
}

#[utoipa::path(operation_id = "auth_logout", post, path = "/api/v1/auth/logout", responses((status = 204), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse)))]
pub(crate) async fn logout(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    user: AuthUser,
) -> ApiResult<impl IntoResponse> {
    user.verify_csrf(&headers, request_id.as_str())?;
    sqlx::query("UPDATE sessions SET revoked_at = ? WHERE id = ?")
        .bind(Utc::now().to_rfc3339())
        .bind(user.session_id)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(sqlx::FromRow)]
struct LoginUser {
    id: String,
    username: String,
    display_name: Option<String>,
    email: Option<String>,
    password_hash: String,
    identity: String,
}

#[derive(sqlx::FromRow)]
struct AuthSession {
    id: String,
    username: String,
    identity: String,
    session_id: String,
    csrf_hash: Vec<u8>,
}

pub fn hash_password(password: &str, request_id: &str) -> ApiResult<String> {
    let mut salt_bytes = [0_u8; 16];
    rand::rng().fill_bytes(&mut salt_bytes);
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| ApiError::internal(request_id))?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ApiError::internal(request_id))
}

fn verify_password(password: &str, encoded: &str, request_id: &str) -> ApiResult<()> {
    let parsed = PasswordHash::new(encoded).map_err(|_| ApiError::internal(request_id))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| ApiError::unauthorized(request_id))
}

pub fn validate_credentials(username: &str, password: &str, request_id: &str) -> ApiResult<()> {
    let username = username.trim();
    if !(3..=64).contains(&username.len())
        || !username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(ApiError::validation("用户名格式不正确", request_id));
    }
    if !(12..=256).contains(&password.len()) {
        return Err(ApiError::validation(
            "密码长度必须为 12 至 256 个字符",
            request_id,
        ));
    }
    Ok(())
}

fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all("cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|part| part.trim().split_once('='))
        .find_map(|(key, value)| (key == name).then(|| value.to_owned()))
}

fn session_cookie(token: &str, secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{}",
        SESSION_LIFETIME.as_secs(),
        secure
    )
}

pub(crate) fn verify_origin(
    state: &AppState,
    headers: &HeaderMap,
    request_id: &str,
) -> ApiResult<()> {
    let origin = headers.get("origin").and_then(|value| value.to_str().ok());
    if origin.is_some_and(|origin| state.allows_origin(origin)) {
        Ok(())
    } else {
        Err(ApiError::forbidden(request_id))
    }
}

pub(crate) async fn session_is_active_administrator(
    state: &AppState,
    session_id: &str,
    actor_id: &str,
) -> bool {
    sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM sessions s JOIN users u ON u.id=s.user_id WHERE s.id=? AND u.id=? AND u.identity='administrator' AND u.status='active' AND u.system_account=0 AND s.revoked_at IS NULL AND s.expires_at>?)")
        .bind(session_id)
        .bind(actor_id)
        .bind(Utc::now().to_rfc3339())
        .fetch_one(state.pool())
        .await
        .unwrap_or(false)
}

fn verify_fetch_metadata(headers: &HeaderMap, request_id: &str) -> ApiResult<()> {
    let site = headers
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok());
    let mode = headers
        .get("sec-fetch-mode")
        .and_then(|value| value.to_str().ok());
    if site == Some("same-origin") && matches!(mode, Some("cors" | "same-origin")) {
        Ok(())
    } else {
        Err(ApiError::forbidden(request_id))
    }
}

pub(crate) fn validate_display_name(value: &str, request_id: &str) -> ApiResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 80 || value.chars().any(char::is_control) {
        return Err(ApiError::validation("姓名格式不正确", request_id));
    }
    Ok(value.to_owned())
}

pub(crate) fn validate_optional_email(
    value: Option<&str>,
    request_id: &str,
) -> ApiResult<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let valid = value.len() <= 254
        && value.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        })
        && value
            .bytes()
            .all(|byte| byte.is_ascii() && !byte.is_ascii_whitespace());
    if !valid {
        return Err(ApiError::validation("邮箱格式不正确", request_id));
    }
    Ok(Some(value.to_ascii_lowercase()))
}

async fn load_identity(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    request_id: &str,
) -> ApiResult<UserIdentity> {
    sqlx::query_as::<_, UserIdentity>("SELECT id, username, COALESCE(display_name, username) AS display_name, email, identity FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?
        .ok_or_else(|| ApiError::unauthorized(request_id))
}

async fn load_preferences(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    request_id: &str,
) -> ApiResult<UserPreferencesResponse> {
    sqlx::query(
        "INSERT INTO user_preferences (user_id) VALUES (?) ON CONFLICT(user_id) DO NOTHING",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    sqlx::query_as("SELECT notify_deployment_failed, notify_deployment_completed, notify_node_unhealthy, time_format, follow_logs, version FROM user_preferences WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))
}
