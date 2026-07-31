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

#[derive(Clone, sqlx::FromRow)]
pub struct AuthUser {
    pub id: String,
    pub username: String,
    pub identity: String,
    pub session_id: String,
    csrf_hash: Vec<u8>,
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
        let actual = token_hash(value);
        if actual.ct_eq(&self.csrf_hash).into() {
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
        let user = sqlx::query_as::<_, AuthUser>(
            "SELECT u.id, u.username, u.identity, s.id AS session_id, s.csrf_hash FROM sessions s JOIN users u ON u.id = s.user_id WHERE s.token_hash = ? AND s.revoked_at IS NULL AND s.expires_at > ? AND u.status = 'active'",
        )
        .bind(hash)
        .bind(Utc::now().to_rfc3339())
        .fetch_optional(state.pool())
        .await
        .map_err(|error| {
            tracing::error!(%error, %request_id, "session lookup failed");
            ApiError::internal(request_id)
        })?;
        user.ok_or_else(|| ApiError::unauthorized(request_id))
    }
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct SetupRequest {
    username: String,
    password: String,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize, ToSchema)]
struct SessionResponse {
    user: UserIdentity,
    csrf_token: String,
}

#[derive(Serialize, ToSchema)]
pub struct UserIdentity {
    pub id: String,
    pub username: String,
    pub identity: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/setup", post(setup))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
}

#[utoipa::path(post, path = "/api/v1/setup", request_body = SetupRequest, responses((status = 201, body = UserIdentity), (status = 401), (status = 409)))]
pub(crate) async fn setup(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    Json(payload): Json<SetupRequest>,
) -> ApiResult<impl IntoResponse> {
    verify_origin(&state, &headers, request_id.as_str())?;
    let configured = state
        .setup_token()
        .ok_or_else(|| ApiError::unauthorized(request_id.as_str()))?;
    let supplied = headers
        .get("x-setup-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if !constant_time_token_eq(configured, supplied) {
        return Err(ApiError::unauthorized(request_id.as_str()));
    }
    validate_credentials(&payload.username, &payload.password, request_id.as_str())?;
    let password_hash = hash_password(&payload.password, request_id.as_str())?;
    let user_id = format!("usr_{}", Ulid::new());
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
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
    sqlx::query("INSERT INTO users (id, username, password_hash, identity, status) VALUES (?, ?, ?, 'administrator', 'active')")
        .bind(&user_id).bind(payload.username.trim()).bind(password_hash)
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
            identity: "administrator".to_owned(),
        }),
    ))
}

#[utoipa::path(post, path = "/api/v1/auth/login", request_body = LoginRequest, responses((status = 200, body = SessionResponse), (status = 401), (status = 403)))]
pub(crate) async fn login(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> ApiResult<Response> {
    verify_origin(&state, &headers, request_id.as_str())?;
    let row = sqlx::query_as::<_, LoginUser>("SELECT id, username, password_hash, identity FROM users WHERE username = ? COLLATE NOCASE AND status = 'active'")
        .bind(payload.username.trim()).fetch_optional(state.pool()).await
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

#[utoipa::path(get, path = "/api/v1/auth/me", responses((status = 200, body = UserIdentity), (status = 401)))]
pub(crate) async fn me(user: AuthUser) -> Json<UserIdentity> {
    Json(UserIdentity {
        id: user.id,
        username: user.username,
        identity: user.identity,
    })
}

#[utoipa::path(post, path = "/api/v1/auth/logout", responses((status = 204), (status = 401), (status = 403)))]
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
    password_hash: String,
    identity: String,
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

fn constant_time_token_eq(expected: &str, actual: &str) -> bool {
    token_hash(expected).ct_eq(&token_hash(actual)).into()
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

fn verify_origin(state: &AppState, headers: &HeaderMap, request_id: &str) -> ApiResult<()> {
    let origin = headers.get("origin").and_then(|value| value.to_str().ok());
    if origin == Some(state.allowed_origin()) {
        Ok(())
    } else {
        Err(ApiError::forbidden(request_id))
    }
}
