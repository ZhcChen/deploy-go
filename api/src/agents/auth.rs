use std::time::Duration;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, header::AUTHORIZATION},
    routing::post,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration as ChronoDuration, Utc};
use deploy_go_agent_protocol::{MIN_SUPPORTED_PROTOCOL_VERSION, PROTOCOL_VERSION};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, Sqlite, Transaction};
use ulid::Ulid;
use utoipa::ToSchema;

use crate::{
    AppState, RequestId, audit,
    error::{ApiError, ApiResult},
};

const ENROLLMENT_LIFETIME: Duration = Duration::from_secs(30 * 60);
const ACCESS_LIFETIME: Duration = Duration::from_secs(30 * 60);
const REFRESH_LIFETIME: Duration = Duration::from_secs(90 * 24 * 60 * 60);

pub struct EnrollmentToken {
    pub token: String,
    pub expires_at: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EnrollRequest {
    agent_id: String,
    enrollment_token: String,
    agent_version: String,
    #[schema(minimum = 11, maximum = 13)]
    protocol_version: u16,
    hostname: String,
    os: String,
    architecture: String,
}

#[derive(Serialize, ToSchema)]
pub struct TokenPairResponse {
    agent_id: String,
    access_token: String,
    access_expires_at: String,
    refresh_token: String,
    refresh_expires_at: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RefreshRequest {
    refresh_token: String,
    rotation_id: String,
}

#[derive(Serialize, ToSchema)]
pub struct RefreshTokenPairResponse {
    agent_id: String,
    rotation_id: String,
    access_token: String,
    access_expires_at: String,
    refresh_token: String,
    refresh_expires_at: String,
}

#[derive(FromRow)]
struct EnrollmentRow {
    id: String,
    agent_id: String,
}

#[derive(Clone, sqlx::FromRow)]
pub(crate) struct AgentAccessIdentity {
    pub access_id: String,
    pub agent_id: String,
    pub family_id: String,
    pub expires_at: String,
}

pub(crate) async fn authenticate_access(
    pool: &sqlx::SqlitePool,
    headers: &HeaderMap,
    request_id: &str,
) -> ApiResult<AgentAccessIdentity> {
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| {
            !token.is_empty() && token.len() <= 256 && !token.chars().any(char::is_control)
        })
        .ok_or_else(|| ApiError::unauthorized(request_id))?;
    sqlx::query_as::<_, AgentAccessIdentity>(
        "SELECT session.id AS access_id,session.agent_id,session.family_id,session.expires_at FROM agent_access_sessions session JOIN agent_credential_families family ON family.id=session.family_id JOIN agents agent ON agent.id=session.agent_id WHERE session.token_hash=? AND session.revoked_at IS NULL AND session.expires_at>? AND family.revoked_at IS NULL AND agent.revoked_at IS NULL AND agent.archived_at IS NULL",
    )
    .bind(token_hash("access", token))
    .bind(Utc::now().to_rfc3339())
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?
    .ok_or_else(|| ApiError::unauthorized(request_id))
}

#[derive(FromRow)]
struct RefreshRow {
    id: String,
    family_id: String,
    agent_id: String,
    generation: i64,
    expires_at: String,
    rotation_id: Option<String>,
    replaced_by_id: Option<String>,
    committed_at: Option<String>,
    revoked_at: Option<String>,
    family_revoked_at: Option<String>,
}

#[derive(FromRow)]
struct RotationResultRow {
    refresh_id: String,
    refresh_expires_at: String,
    refresh_key_version: i64,
    access_id: String,
    access_expires_at: String,
    access_key_version: i64,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/agent/enroll", post(enroll))
        .route("/agent/refresh", post(refresh))
}

pub async fn issue_enrollment(
    transaction: &mut Transaction<'_, Sqlite>,
    agent_id: &str,
    created_by: Option<&str>,
) -> sqlx::Result<EnrollmentToken> {
    let now = Utc::now();
    sqlx::query("UPDATE agent_enrollment_tokens SET revoked_at=? WHERE agent_id=? AND consumed_at IS NULL AND revoked_at IS NULL")
        .bind(now.to_rfc3339())
        .bind(agent_id)
        .execute(&mut **transaction)
        .await?;
    let id = format!("enroll_{}", Ulid::new());
    let token = random_token("dga_enroll");
    let expires_at =
        (now + ChronoDuration::from_std(ENROLLMENT_LIFETIME).expect("固定 TTL 有效")).to_rfc3339();
    sqlx::query("INSERT INTO agent_enrollment_tokens (id,agent_id,token_hash,expires_at,created_by) VALUES (?,?,?,?,?)")
        .bind(id)
        .bind(agent_id)
        .bind(token_hash("enrollment", &token))
        .bind(&expires_at)
        .bind(created_by)
        .execute(&mut **transaction)
        .await?;
    Ok(EnrollmentToken { token, expires_at })
}

#[utoipa::path(operation_id = "agent_enroll", post, path = "/api/v1/agent/enroll", request_body = EnrollRequest, responses((status = 200, body = TokenPairResponse), (status = 401, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn enroll(
    State(state): State<AppState>,
    axum::extract::Extension(request_id): axum::extract::Extension<RequestId>,
    crate::http::ApiJson(payload): crate::http::ApiJson<EnrollRequest>,
) -> ApiResult<Json<TokenPairResponse>> {
    validate_metadata(&payload, request_id.as_str())?;
    let key_ring = state
        .master_key_ring()
        .ok_or_else(|| ApiError::service_not_ready(request_id.as_str()))?;
    let now = Utc::now();
    let now_text = now.to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let enrollment = sqlx::query_as::<_, EnrollmentRow>(
        "SELECT id,agent_id FROM agent_enrollment_tokens WHERE token_hash=? AND agent_id=? AND consumed_at IS NULL AND revoked_at IS NULL AND expires_at>?",
    )
    .bind(token_hash("enrollment", &payload.enrollment_token))
    .bind(&payload.agent_id)
    .bind(&now_text)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?
    .ok_or_else(|| ApiError::unauthorized(request_id.as_str()))?;
    let consumed = sqlx::query("UPDATE agent_enrollment_tokens SET consumed_at=? WHERE id=? AND consumed_at IS NULL AND revoked_at IS NULL")
        .bind(&now_text)
        .bind(&enrollment.id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if consumed.rows_affected() != 1 {
        return Err(ApiError::unauthorized(request_id.as_str()));
    }

    let family_id = format!("family_{}", Ulid::new());
    let refresh_id = format!("refresh_{}", Ulid::new());
    let access_id = format!("access_{}", Ulid::new());
    let key_version = key_ring.current_version();
    let refresh_token = key_ring
        .derive_agent_token("refresh", &refresh_id, key_version)
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let access_token = key_ring
        .derive_agent_token("access", &access_id, key_version)
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let refresh_expires_at =
        (now + ChronoDuration::from_std(REFRESH_LIFETIME).expect("固定 TTL 有效")).to_rfc3339();
    let access_expires_at =
        (now + ChronoDuration::from_std(ACCESS_LIFETIME).expect("固定 TTL 有效")).to_rfc3339();
    sqlx::query("INSERT INTO agent_credential_families (id,agent_id) VALUES (?,?)")
        .bind(&family_id)
        .bind(&enrollment.agent_id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_registration_conflict(error, request_id.as_str()))?;
    sqlx::query("INSERT INTO agent_refresh_credentials (id,family_id,generation,token_hash,expires_at,token_key_version) VALUES (?,?,1,?,?,?)")
        .bind(&refresh_id)
        .bind(&family_id)
        .bind(token_hash("refresh", &refresh_token))
        .bind(&refresh_expires_at)
        .bind(key_version)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    sqlx::query("INSERT INTO agent_access_sessions (id,agent_id,family_id,refresh_credential_id,token_hash,expires_at,token_key_version) VALUES (?,?,?,?,?,?,?)")
        .bind(&access_id)
        .bind(&enrollment.agent_id)
        .bind(&family_id)
        .bind(&refresh_id)
        .bind(token_hash("access", &access_token))
        .bind(&access_expires_at)
        .bind(key_version)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    sqlx::query("UPDATE agents SET registered_at=COALESCE(registered_at,?),revoked_at=NULL,archived_at=NULL,agent_version=?,protocol_version=?,hostname=?,os_name=?,architecture=?,updated_at=?,version=version+1 WHERE id=?")
        .bind(&now_text)
        .bind(&payload.agent_version)
        .bind(i64::from(payload.protocol_version))
        .bind(&payload.hostname)
        .bind(&payload.os)
        .bind(&payload.architecture)
        .bind(&now_text)
        .bind(&enrollment.agent_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;

    Ok(Json(TokenPairResponse {
        agent_id: enrollment.agent_id,
        access_token: access_token.to_string(),
        access_expires_at,
        refresh_token: refresh_token.to_string(),
        refresh_expires_at,
    }))
}

#[utoipa::path(operation_id = "agent_refresh", post, path = "/api/v1/agent/refresh", request_body = RefreshRequest, responses((status = 200, body = RefreshTokenPairResponse), (status = 401, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn refresh(
    State(state): State<AppState>,
    axum::extract::Extension(request_id): axum::extract::Extension<RequestId>,
    crate::http::ApiJson(payload): crate::http::ApiJson<RefreshRequest>,
) -> ApiResult<Json<RefreshTokenPairResponse>> {
    validate_rotation_id(&payload.rotation_id, request_id.as_str())?;
    let key_ring = state
        .master_key_ring()
        .ok_or_else(|| ApiError::service_not_ready(request_id.as_str()))?;
    let now = Utc::now();
    let now_text = now.to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let credential = sqlx::query_as::<_, RefreshRow>(
        "SELECT r.id,r.family_id,f.agent_id,r.generation,r.expires_at,r.rotation_id,r.replaced_by_id,r.committed_at,r.revoked_at,f.revoked_at AS family_revoked_at FROM agent_refresh_credentials r JOIN agent_credential_families f ON f.id=r.family_id WHERE r.token_hash=?",
    )
    .bind(token_hash("refresh", &payload.refresh_token))
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?
    .ok_or_else(|| ApiError::unauthorized(request_id.as_str()))?;

    if credential.family_revoked_at.is_some() || credential.expires_at <= now_text {
        return Err(ApiError::unauthorized(request_id.as_str()));
    }

    let rotation = if credential.replaced_by_id.is_none()
        && credential.rotation_id.is_none()
        && credential.committed_at.is_none()
        && credential.revoked_at.is_none()
    {
        create_rotation(
            &mut transaction,
            &credential,
            &payload.rotation_id,
            key_ring,
            now,
            request_id.as_str(),
        )
        .await?
    } else if credential.rotation_id.as_deref() == Some(payload.rotation_id.as_str())
        && credential.committed_at.is_none()
        && credential.revoked_at.is_none()
    {
        load_rotation(
            &mut transaction,
            &credential,
            &now_text,
            request_id.as_str(),
        )
        .await?
    } else {
        revoke_family_for_reuse(
            &mut transaction,
            &credential,
            &now_text,
            request_id.as_str(),
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        state.agent_connections().disconnect(&credential.agent_id);
        return Err(ApiError::unauthorized(request_id.as_str()));
    };
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;

    let refresh_token = key_ring
        .derive_agent_token(
            "refresh",
            &rotation.refresh_id,
            rotation.refresh_key_version,
        )
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let access_token = key_ring
        .derive_agent_token("access", &rotation.access_id, rotation.access_key_version)
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(RefreshTokenPairResponse {
        agent_id: credential.agent_id,
        rotation_id: payload.rotation_id,
        access_token: access_token.to_string(),
        access_expires_at: rotation.access_expires_at,
        refresh_token: refresh_token.to_string(),
        refresh_expires_at: rotation.refresh_expires_at,
    }))
}

async fn create_rotation(
    transaction: &mut Transaction<'_, Sqlite>,
    credential: &RefreshRow,
    rotation_id: &str,
    key_ring: &crate::crypto::MasterKeyRing,
    now: chrono::DateTime<Utc>,
    request_id: &str,
) -> ApiResult<RotationResultRow> {
    let refresh_id = format!("refresh_{}", Ulid::new());
    let access_id = format!("access_{}", Ulid::new());
    let key_version = key_ring.current_version();
    let refresh_token = key_ring
        .derive_agent_token("refresh", &refresh_id, key_version)
        .map_err(|_| ApiError::internal(request_id))?;
    let access_token = key_ring
        .derive_agent_token("access", &access_id, key_version)
        .map_err(|_| ApiError::internal(request_id))?;
    let refresh_expires_at =
        (now + ChronoDuration::from_std(REFRESH_LIFETIME).expect("固定 TTL 有效")).to_rfc3339();
    let access_expires_at =
        (now + ChronoDuration::from_std(ACCESS_LIFETIME).expect("固定 TTL 有效")).to_rfc3339();
    sqlx::query("INSERT INTO agent_refresh_credentials (id,family_id,generation,token_hash,expires_at,token_key_version) VALUES (?,?,?,?,?,?)")
        .bind(&refresh_id)
        .bind(&credential.family_id)
        .bind(credential.generation + 1)
        .bind(token_hash("refresh", &refresh_token))
        .bind(&refresh_expires_at)
        .bind(key_version)
        .execute(&mut **transaction)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    sqlx::query("INSERT INTO agent_access_sessions (id,agent_id,family_id,refresh_credential_id,token_hash,expires_at,token_key_version) VALUES (?,?,?,?,?,?,?)")
        .bind(&access_id)
        .bind(&credential.agent_id)
        .bind(&credential.family_id)
        .bind(&refresh_id)
        .bind(token_hash("access", &access_token))
        .bind(&access_expires_at)
        .bind(key_version)
        .execute(&mut **transaction)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    let updated = sqlx::query("UPDATE agent_refresh_credentials SET rotation_id=?,replaced_by_id=? WHERE id=? AND rotation_id IS NULL AND replaced_by_id IS NULL AND committed_at IS NULL AND revoked_at IS NULL")
        .bind(rotation_id)
        .bind(&refresh_id)
        .bind(&credential.id)
        .execute(&mut **transaction)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    if updated.rows_affected() != 1 {
        return Err(ApiError::unauthorized(request_id));
    }
    Ok(RotationResultRow {
        refresh_id,
        refresh_expires_at,
        refresh_key_version: key_version,
        access_id,
        access_expires_at,
        access_key_version: key_version,
    })
}

async fn load_rotation(
    transaction: &mut Transaction<'_, Sqlite>,
    credential: &RefreshRow,
    now: &str,
    request_id: &str,
) -> ApiResult<RotationResultRow> {
    sqlx::query_as::<_, RotationResultRow>(
        "SELECT successor.id AS refresh_id,successor.expires_at AS refresh_expires_at,successor.token_key_version AS refresh_key_version,access.id AS access_id,access.expires_at AS access_expires_at,access.token_key_version AS access_key_version FROM agent_refresh_credentials successor JOIN agent_access_sessions access ON access.refresh_credential_id=successor.id WHERE successor.id=? AND successor.family_id=? AND successor.revoked_at IS NULL AND access.revoked_at IS NULL AND access.expires_at>?",
    )
    .bind(&credential.replaced_by_id)
    .bind(&credential.family_id)
    .bind(now)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| ApiError::internal(request_id))?
    .ok_or_else(|| {
        ApiError::new(
            axum::http::StatusCode::UNAUTHORIZED,
            "expired_pending_rotation",
            "待确认凭证轮换已过期",
            request_id,
        )
    })
}

async fn revoke_family_for_reuse(
    transaction: &mut Transaction<'_, Sqlite>,
    credential: &RefreshRow,
    now: &str,
    request_id: &str,
) -> ApiResult<()> {
    sqlx::query("UPDATE agent_credential_families SET revoked_at=COALESCE(revoked_at,?),revoke_reason=COALESCE(revoke_reason,'refresh_token_reuse') WHERE id=?")
        .bind(now)
        .bind(&credential.family_id)
        .execute(&mut **transaction)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    sqlx::query(
        "UPDATE agent_refresh_credentials SET revoked_at=COALESCE(revoked_at,?) WHERE family_id=?",
    )
    .bind(now)
    .bind(&credential.family_id)
    .execute(&mut **transaction)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    sqlx::query(
        "UPDATE agent_access_sessions SET revoked_at=COALESCE(revoked_at,?) WHERE family_id=?",
    )
    .bind(now)
    .bind(&credential.family_id)
    .execute(&mut **transaction)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    audit::record(
        transaction,
        None,
        "agent.refresh_token_reuse",
        "agent",
        &credential.agent_id,
        request_id,
        serde_json::json!({"credential_family_id":credential.family_id}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id))
}

fn validate_metadata(payload: &EnrollRequest, request_id: &str) -> ApiResult<()> {
    for value in [
        payload.agent_id.as_str(),
        payload.agent_version.as_str(),
        payload.hostname.as_str(),
        payload.os.as_str(),
        payload.architecture.as_str(),
    ] {
        if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
            return Err(ApiError::validation("Agent 注册信息格式不正确", request_id));
        }
    }
    if !(MIN_SUPPORTED_PROTOCOL_VERSION..=PROTOCOL_VERSION).contains(&payload.protocol_version) {
        return Err(ApiError::validation("Agent 协议版本不受支持", request_id));
    }
    Ok(())
}

fn validate_rotation_id(rotation_id: &str, request_id: &str) -> ApiResult<()> {
    if !(16..=128).contains(&rotation_id.len())
        || !rotation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(ApiError::validation("rotation_id 格式不正确", request_id));
    }
    Ok(())
}

fn map_registration_conflict(error: sqlx::Error, request_id: &str) -> ApiError {
    if error
        .to_string()
        .contains("agent_credential_families.agent_id")
    {
        ApiError::conflict("agent_already_registered", "Agent 已完成注册", request_id)
    } else {
        ApiError::internal(request_id)
    }
}

fn random_token(prefix: &str) -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    format!("{prefix}_{}", URL_SAFE_NO_PAD.encode(bytes))
}

pub fn token_hash(purpose: &str, token: &str) -> Vec<u8> {
    let mut digest = Sha256::new();
    digest.update(b"deploy-go/agent-token-hash/v1\0");
    digest.update(purpose.as_bytes());
    digest.update(b"\0");
    digest.update(token.as_bytes());
    digest.finalize().to_vec()
}
