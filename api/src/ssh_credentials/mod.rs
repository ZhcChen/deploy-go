use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use ssh_key::{Algorithm, HashAlg, LineEnding, PrivateKey};
use ulid::Ulid;
use utoipa::ToSchema;

use crate::crypto::{EncryptedSecret, MasterKeyRing};
use crate::{
    AppState, RequestId, audit,
    auth::AuthUser,
    error::{ApiError, ApiResult},
    pagination,
};

const ALGORITHM: &str = "ed25519";

#[derive(Clone, Serialize, ToSchema, sqlx::FromRow)]
pub struct SshCredentialResponse {
    pub id: String,
    pub name: String,
    pub algorithm: String,
    pub public_key: String,
    pub fingerprint: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Serialize, ToSchema)]
pub struct SshCredentialListResponse {
    items: Vec<SshCredentialResponse>,
    next_cursor: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateCredentialRequest {
    name: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RenameCredentialRequest {
    name: String,
    version: i64,
}

#[derive(Serialize, sqlx::FromRow)]
struct NodeSummary {
    id: String,
    name: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ssh-credentials", get(list).post(create))
        .route(
            "/ssh-credentials/{id}",
            get(show).patch(rename).delete(delete_credential),
        )
}

#[utoipa::path(operation_id = "ssh_credentials_list", get, path = "/api/v1/ssh-credentials", params(("limit" = Option<u32>, Query), ("after" = Option<String>, Query)), responses((status = 200, body = SshCredentialListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn list(
    State(state): State<AppState>,
    Query(query): Query<pagination::ListQuery>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<SshCredentialListResponse>> {
    actor.require_administrator(request_id.as_str())?;
    let limit = pagination::limit(&query, request_id.as_str())?;
    let (created_at, id) = pagination::decode_after(&query, request_id.as_str())?
        .unwrap_or_else(|| ("0000".to_owned(), "".to_owned()));
    let credentials = sqlx::query_as::<_, SshCredentialResponse>(
        "SELECT id, name, algorithm, public_key, fingerprint, created_at, updated_at, version FROM ssh_credentials WHERE (created_at>? OR (created_at=? AND id>?)) ORDER BY created_at, id LIMIT ?",
    )
    .bind(&created_at).bind(&created_at).bind(&id).bind((limit + 1) as i64)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let (items, next_cursor) =
        pagination::finish(credentials, limit, |item| (&item.created_at, &item.id));
    Ok(Json(SshCredentialListResponse { items, next_cursor }))
}

#[utoipa::path(operation_id = "ssh_credentials_show", get, path = "/api/v1/ssh-credentials/{id}", params(("id" = String, Path)), responses((status = 200, body = SshCredentialResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<SshCredentialResponse>> {
    actor.require_administrator(request_id.as_str())?;
    Ok(Json(find(state.pool(), &id, request_id.as_str()).await?))
}

#[utoipa::path(operation_id = "ssh_credentials_create", post, path = "/api/v1/ssh-credentials", request_body = CreateCredentialRequest, responses((status = 201, body = SshCredentialResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn create(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<CreateCredentialRequest>,
) -> ApiResult<(StatusCode, Json<SshCredentialResponse>)> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let name = validate_name(&payload.name, request_id.as_str())?;
    let ring = state
        .master_key_ring()
        .ok_or_else(|| ApiError::internal(request_id.as_str()))?;
    let id = format!("cred_{}", Ulid::new());
    let (public_key, fingerprint, private_key) =
        generate_key_pair(&id).map_err(|_| ApiError::internal(request_id.as_str()))?;
    let encrypted = ring
        .encrypt(&id, ALGORITHM, private_key.as_bytes())
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("INSERT INTO ssh_credentials (id, name, algorithm, public_key, fingerprint, encrypted_private_key, nonce, key_version, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&id).bind(&name).bind(ALGORITHM).bind(&public_key).bind(&fingerprint)
        .bind(encrypted.ciphertext).bind(encrypted.nonce).bind(encrypted.key_version).bind(&actor.id)
        .execute(&mut *transaction).await;
    if let Err(error) = result {
        if error.to_string().contains("UNIQUE constraint failed") {
            return Err(ApiError::conflict(
                "credential_name_exists",
                "SSH 密钥名称已经存在",
                request_id.as_str(),
            ));
        }
        return Err(ApiError::internal(request_id.as_str()));
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "ssh_credential.create",
        "ssh_credential",
        &id,
        request_id.as_str(),
        json!({"name":name,"algorithm":ALGORITHM,"fingerprint":fingerprint}),
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

#[utoipa::path(operation_id = "ssh_credentials_rename", patch, path = "/api/v1/ssh-credentials/{id}", params(("id" = String, Path)), request_body = RenameCredentialRequest, responses((status = 200, body = SshCredentialResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn rename(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<RenameCredentialRequest>,
) -> ApiResult<Json<SshCredentialResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let name = validate_name(&payload.name, request_id.as_str())?;
    find(state.pool(), &id, request_id.as_str()).await?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("UPDATE ssh_credentials SET name = ?, updated_at = ?, version = version + 1 WHERE id = ? AND version = ?")
        .bind(&name).bind(Utc::now().to_rfc3339()).bind(&id).bind(payload.version)
        .execute(&mut *transaction).await;
    let result = match result {
        Ok(result) => result,
        Err(error) if error.to_string().contains("UNIQUE constraint failed") => {
            return Err(ApiError::conflict(
                "credential_name_exists",
                "SSH 密钥名称已经存在",
                request_id.as_str(),
            ));
        }
        Err(_) => return Err(ApiError::internal(request_id.as_str())),
    };
    if result.rows_affected() == 0 {
        return Err(ApiError::conflict(
            "resource_version_conflict",
            "SSH 密钥已经被其他请求修改",
            request_id.as_str(),
        ));
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "ssh_credential.rename",
        "ssh_credential",
        &id,
        request_id.as_str(),
        json!({"name":name}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(find(state.pool(), &id, request_id.as_str()).await?))
}

#[utoipa::path(operation_id = "ssh_credentials_delete_credential", delete, path = "/api/v1/ssh-credentials/{id}", params(("id" = String, Path)), responses((status = 204), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse)))]
pub(crate) async fn delete_credential(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<StatusCode> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let credential = find(state.pool(), &id, request_id.as_str()).await?;
    let nodes = referenced_nodes(state.pool(), &id, request_id.as_str()).await?;
    if !nodes.is_empty() {
        return Err(in_use_error(nodes, request_id.as_str()));
    }
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("DELETE FROM ssh_credentials WHERE id = ?")
        .bind(&id)
        .execute(&mut *transaction)
        .await;
    match result {
        Err(_) => {
            drop(transaction);
            let nodes = referenced_nodes(state.pool(), &id, request_id.as_str()).await?;
            if !nodes.is_empty() {
                return Err(in_use_error(nodes, request_id.as_str()));
            }
            return Err(ApiError::internal(request_id.as_str()));
        }
        Ok(result) if result.rows_affected() == 0 => {
            return Err(ApiError::not_found(request_id.as_str()));
        }
        Ok(_) => {}
    }
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "ssh_credential.delete",
        "ssh_credential",
        &id,
        request_id.as_str(),
        json!({"name":credential.name,"algorithm":credential.algorithm,"fingerprint":credential.fingerprint}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(StatusCode::NO_CONTENT)
}

fn generate_key_pair(
    credential_id: &str,
) -> Result<(String, String, zeroize::Zeroizing<String>), ssh_key::Error> {
    let mut key = PrivateKey::random(&mut ssh_key::rand_core::OsRng, Algorithm::Ed25519)?;
    key.set_comment(format!("deploy-go:{credential_id}"));
    let public_key = key.public_key().to_openssh()?;
    let fingerprint = key.public_key().fingerprint(HashAlg::Sha256).to_string();
    let private_key = key.to_openssh(LineEnding::LF)?;
    Ok((public_key, fingerprint, private_key))
}

fn validate_name(name: &str, request_id: &str) -> ApiResult<String> {
    let name = name.trim();
    if !(1..=64).contains(&name.chars().count()) || name.chars().any(char::is_control) {
        return Err(ApiError::validation(
            "SSH 密钥名称长度必须为 1 至 64 个字符",
            request_id,
        ));
    }
    Ok(name.to_owned())
}

async fn find(
    pool: &sqlx::SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<SshCredentialResponse> {
    sqlx::query_as("SELECT id, name, algorithm, public_key, fingerprint, created_at, updated_at, version FROM ssh_credentials WHERE id = ?")
        .bind(id).fetch_optional(pool).await.map_err(|_| ApiError::internal(request_id))?
        .ok_or_else(|| ApiError::not_found(request_id))
}

async fn referenced_nodes(
    pool: &sqlx::SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<Vec<NodeSummary>> {
    sqlx::query_as(
        "SELECT id, name FROM nodes WHERE ssh_credential_id = ? ORDER BY name, id LIMIT 20",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))
}

fn in_use_error(nodes: Vec<NodeSummary>, request_id: &str) -> ApiError {
    ApiError::conflict(
        "credential_in_use",
        "SSH 密钥仍被节点引用，必须先解绑",
        request_id,
    )
    .with_details(json!({"nodes":nodes}))
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
            "SELECT id, algorithm, encrypted_private_key, nonce, key_version FROM ssh_credentials WHERE key_version != ? ORDER BY id LIMIT 100",
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
                .map_err(|_| anyhow::anyhow!("SSH 凭证重加密失败"))?;
            let encrypted = ring
                .encrypt(&row.id, &row.algorithm, plaintext.as_slice())
                .map_err(|_| anyhow::anyhow!("SSH 凭证重加密失败"))?;
            ring.decrypt(&row.id, &row.algorithm, &encrypted)
                .map_err(|_| anyhow::anyhow!("SSH 凭证重加密校验失败"))?;
            let result = sqlx::query("UPDATE ssh_credentials SET encrypted_private_key = ?, nonce = ?, key_version = ?, updated_at = ?, version = version + 1 WHERE id = ? AND key_version = ?")
                .bind(encrypted.ciphertext).bind(encrypted.nonce).bind(encrypted.key_version)
                .bind(Utc::now().to_rfc3339()).bind(&row.id).bind(row.key_version)
                .execute(pool).await?;
            migrated += result.rows_affected();
        }
    }
}
