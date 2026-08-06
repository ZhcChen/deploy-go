use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, put},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use ssh_key::{
    Fingerprint, HashAlg, LineEnding, PrivateKey, private::Ed25519Keypair, rand_core::OsRng,
};
use ulid::Ulid;
use utoipa::ToSchema;
use zeroize::Zeroizing;

use crate::{
    AppState, RequestId, audit,
    auth::AuthUser,
    crypto::{EncryptedSecret, MasterKeyRing},
    error::{ApiError, ApiResult},
    pagination,
};

#[derive(Clone, Serialize, ToSchema, sqlx::FromRow)]
pub struct GitCredentialResponse {
    pub id: String,
    pub name: String,
    pub algorithm: String,
    pub public_key: String,
    pub fingerprint: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Serialize, ToSchema)]
pub struct GitCredentialListResponse {
    items: Vec<GitCredentialResponse>,
    next_cursor: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateGitCredentialRequest {
    name: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct GitCredentialStatusRequest {
    status: String,
    version: i64,
}

struct GeneratedCredential {
    private_key: Zeroizing<String>,
    public_key: String,
    fingerprint: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/git-credentials", get(list).post(create))
        .route("/git-credentials/{id}", get(show))
        .route("/git-credentials/{id}/status", put(update_status))
}

#[utoipa::path(operation_id = "git_credentials_list", get, path = "/api/v1/git-credentials", params(("limit" = Option<u32>, Query), ("after" = Option<String>, Query)), responses((status = 200, body = GitCredentialListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn list(
    State(state): State<AppState>,
    Query(query): Query<pagination::ListQuery>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<GitCredentialListResponse>> {
    actor.require_administrator(request_id.as_str())?;
    let limit = pagination::limit(&query, request_id.as_str())?;
    let (created_at, id) = pagination::decode_after(&query, request_id.as_str())?
        .unwrap_or_else(|| ("0000".to_owned(), "".to_owned()));
    let credentials = sqlx::query_as::<_, GitCredentialResponse>(
        "SELECT id, name, algorithm, public_key, fingerprint, status, created_at, updated_at, version FROM git_credentials WHERE (created_at>? OR (created_at=? AND id>?)) ORDER BY created_at, id LIMIT ?",
    )
    .bind(&created_at).bind(&created_at).bind(&id).bind((limit + 1) as i64)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let (items, next_cursor) =
        pagination::finish(credentials, limit, |item| (&item.created_at, &item.id));
    Ok(Json(GitCredentialListResponse { items, next_cursor }))
}

#[utoipa::path(operation_id = "git_credentials_create", post, path = "/api/v1/git-credentials", request_body = CreateGitCredentialRequest, responses((status = 201, body = GitCredentialResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn create(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<CreateGitCredentialRequest>,
) -> ApiResult<(StatusCode, Json<GitCredentialResponse>)> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let name = validate_name(&payload.name, request_id.as_str())?;
    let ring = state.master_key_ring().ok_or_else(|| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "master_key_not_configured",
            "服务未配置主密钥，无法生成 Git 凭证",
            request_id.as_str(),
        )
    })?;
    let generated = generate_credential(request_id.as_str())?;
    let id = format!("git_cred_{}", Ulid::new());
    let encrypted = ring
        .encrypt(&id, "ed25519", generated.private_key.as_bytes())
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    sqlx::query("INSERT INTO git_credentials (id, name, algorithm, public_key, fingerprint, encrypted_private_key, nonce, key_version, status, created_by) VALUES (?, ?, 'ed25519', ?, ?, ?, ?, ?, 'active', ?)")
        .bind(&id).bind(name).bind(&generated.public_key).bind(&generated.fingerprint)
        .bind(encrypted.ciphertext).bind(encrypted.nonce).bind(encrypted.key_version).bind(&actor.id)
        .execute(&mut *transaction).await.map_err(|error| map_unique(error, request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "git_credential.create",
        "git_credential",
        &id,
        request_id.as_str(),
        json!({"name":name,"algorithm":"ed25519","fingerprint":generated.fingerprint}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok((
        StatusCode::CREATED,
        Json(find(state.pool(), &id, request_id.as_str()).await?),
    ))
}

#[utoipa::path(operation_id = "git_credentials_show", get, path = "/api/v1/git-credentials/{id}", params(("id" = String, Path)), responses((status = 200, body = GitCredentialResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<GitCredentialResponse>> {
    actor.require_administrator(request_id.as_str())?;
    Ok(Json(find(state.pool(), &id, request_id.as_str()).await?))
}

#[utoipa::path(operation_id = "git_credentials_update_status", put, path = "/api/v1/git-credentials/{id}/status", params(("id" = String, Path)), request_body = GitCredentialStatusRequest, responses((status = 200, body = GitCredentialResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<GitCredentialStatusRequest>,
) -> ApiResult<Json<GitCredentialResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    if !matches!(payload.status.as_str(), "active" | "archived") {
        return Err(ApiError::validation(
            "Git 凭证状态不正确",
            request_id.as_str(),
        ));
    }
    find(state.pool(), &id, request_id.as_str()).await?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("UPDATE git_credentials SET status=?, updated_at=?, version=version+1 WHERE id=? AND version=?")
        .bind(&payload.status).bind(Utc::now().to_rfc3339()).bind(&id).bind(payload.version)
        .execute(&mut *transaction).await.map_err(|_| ApiError::internal(request_id.as_str()))?;
    if result.rows_affected() == 0 {
        return Err(ApiError::conflict(
            "resource_version_conflict",
            "Git 凭证已经被其他请求修改",
            request_id.as_str(),
        ));
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "git_credential.status.update",
        "git_credential",
        &id,
        request_id.as_str(),
        json!({"status":payload.status}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(find(state.pool(), &id, request_id.as_str()).await?))
}

async fn find(
    pool: &sqlx::SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<GitCredentialResponse> {
    sqlx::query_as("SELECT id, name, algorithm, public_key, fingerprint, status, created_at, updated_at, version FROM git_credentials WHERE id = ?")
        .bind(id).fetch_optional(pool).await.map_err(|_| ApiError::internal(request_id))?
        .ok_or_else(|| ApiError::not_found(request_id))
}

fn validate_name<'a>(name: &'a str, request_id: &str) -> ApiResult<&'a str> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 100 || name.chars().any(char::is_control) {
        return Err(ApiError::validation("Git 凭证名称格式不正确", request_id));
    }
    Ok(name)
}

fn generate_credential(request_id: &str) -> ApiResult<GeneratedCredential> {
    let keypair = Ed25519Keypair::random(&mut OsRng);
    let private = PrivateKey::from(keypair);
    let private_key = private
        .to_openssh(LineEnding::LF)
        .map_err(|_| ApiError::internal(request_id))?;
    let public = private.public_key();
    let public_key = public
        .to_openssh()
        .map_err(|_| ApiError::internal(request_id))?;
    let fingerprint = Fingerprint::new(HashAlg::Sha256, public.key_data()).to_string();
    Ok(GeneratedCredential {
        private_key,
        public_key,
        fingerprint,
    })
}

fn map_unique(error: sqlx::Error, request_id: &str) -> ApiError {
    if error.to_string().contains("UNIQUE constraint failed") {
        ApiError::conflict(
            "git_credential_identity_exists",
            "Git 凭证名称已存在",
            request_id,
        )
    } else {
        ApiError::internal(request_id)
    }
}

#[derive(sqlx::FromRow)]
struct EncryptedCredentialRow {
    id: String,
    algorithm: String,
    encrypted_private_key: Vec<u8>,
    nonce: Vec<u8>,
    key_version: i64,
}

pub async fn reencrypt_all(pool: &sqlx::SqlitePool, ring: &MasterKeyRing) -> anyhow::Result<u64> {
    let mut migrated = 0_u64;
    loop {
        let rows = sqlx::query_as::<_, EncryptedCredentialRow>(
            "SELECT id, algorithm, encrypted_private_key, nonce, key_version FROM git_credentials WHERE key_version != ? ORDER BY id LIMIT 100",
        )
        .bind(ring.current_version())
        .fetch_all(pool)
        .await?;
        if rows.is_empty() {
            return Ok(migrated);
        }
        for row in rows {
            let old = EncryptedSecret {
                ciphertext: row.encrypted_private_key,
                nonce: row.nonce,
                key_version: row.key_version,
            };
            let plaintext = ring
                .decrypt(&row.id, &row.algorithm, &old)
                .map_err(|_| anyhow::anyhow!("Git 凭证重加密失败"))?;
            let encrypted = ring
                .encrypt(&row.id, &row.algorithm, plaintext.as_slice())
                .map_err(|_| anyhow::anyhow!("Git 凭证重加密失败"))?;
            ring.decrypt(&row.id, &row.algorithm, &encrypted)
                .map_err(|_| anyhow::anyhow!("Git 凭证重加密校验失败"))?;
            let result = sqlx::query("UPDATE git_credentials SET encrypted_private_key = ?, nonce = ?, key_version = ?, updated_at = ?, version = version + 1 WHERE id = ? AND key_version = ?")
                .bind(encrypted.ciphertext).bind(encrypted.nonce).bind(encrypted.key_version)
                .bind(Utc::now().to_rfc3339()).bind(&row.id).bind(row.key_version)
                .execute(pool).await?;
            migrated += result.rows_affected();
        }
    }
}
