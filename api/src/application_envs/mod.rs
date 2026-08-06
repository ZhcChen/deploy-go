mod dotenv;

use std::time::Duration;

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{Duration as ChronoDuration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{Sqlite, Transaction};
use ulid::Ulid;
use utoipa::ToSchema;
use zeroize::Zeroizing;

use crate::{
    AppState, RequestId,
    agents::auth::{AgentAccessIdentity, authenticate_access},
    audit,
    auth::AuthUser,
    crypto::{APPLICATION_ENV_ALGORITHM, EncryptedSecret},
    error::{ApiError, ApiResult},
};

const GRANT_LIFETIME: Duration = Duration::from_secs(5 * 60);
const REAUTH_WINDOW_MINUTES: i64 = 15;
const REAUTH_BLOCK_MINUTES: i64 = 15;
const MAX_REAUTH_FAILURES: i64 = 5;
const MAX_ENV_FILES: usize = 64;

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct ApplicationEnvFileResponse {
    id: String,
    application_id: String,
    file_name: String,
    module: String,
    format: String,
    current_version: i64,
    current_digest: String,
    declared_at: String,
    updated_at: String,
    version: i64,
    target_count: i64,
    pending_count: i64,
    syncing_count: i64,
    succeeded_count: i64,
    failed_count: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApplicationEnvFileListResponse {
    items: Vec<ApplicationEnvFileResponse>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EnvReauthenticateRequest {
    password: String,
    action: EnvGrantAction,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnvGrantAction {
    ReadWrite,
    Delete,
}

impl EnvGrantAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::ReadWrite => "read_write",
            Self::Delete => "delete",
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EnvRevealGrantResponse {
    grant_token: String,
    action: EnvGrantAction,
    expires_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApplicationEnvPlaintextResponse {
    id: String,
    application_id: String,
    file_name: String,
    module: String,
    format: String,
    content: String,
    digest: String,
    env_version: i64,
    version: i64,
    updated_at: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateApplicationEnvRequest {
    content: String,
    expected_version: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeleteApplicationEnvRequest {
    expected_version: i64,
    confirm_file_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvManifest {
    schema_version: u8,
    commit_sha: String,
    files: Vec<EnvManifestFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvManifestFile {
    file_name: String,
    module: String,
    sha256: String,
    size: usize,
    format: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterApplicationEnvsRequest {
    manifest_json: String,
    files: Vec<RegisterApplicationEnvContent>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterApplicationEnvContent {
    file_name: String,
    content_base64: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RegisterApplicationEnvsResponse {
    created: Vec<String>,
    declared: Vec<String>,
}

#[derive(sqlx::FromRow)]
struct EnvVersionRow {
    env_file_id: String,
    application_id: String,
    file_name: String,
    module: String,
    format: String,
    env_version: i64,
    digest: String,
    ciphertext: Vec<u8>,
    nonce: Vec<u8>,
    key_version: i64,
    version_id: String,
    file_version: i64,
    updated_at: String,
}

#[derive(sqlx::FromRow)]
struct RegistrationLease {
    application_id: String,
    deployment_id: String,
    commit_sha: String,
    manifest_digest: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/applications/{application_id}/env-files", get(list))
        .route(
            "/applications/{application_id}/env-reveal-grants",
            post(reauthenticate),
        )
        .route(
            "/application-env-files/{env_file_id}",
            get(reveal).put(update).delete(delete_env),
        )
        .route(
            "/agent/env-registration-leases/{lease_id}/register",
            post(register),
        )
}

#[utoipa::path(operation_id = "application_envs_list", get, path = "/api/v1/applications/{application_id}/env-files", params(("application_id" = String, Path)), responses((status = 200, body = ApplicationEnvFileListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn list(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<ApplicationEnvFileListResponse>> {
    actor.require_administrator(request_id.as_str())?;
    ensure_application(state.pool(), &application_id, request_id.as_str()).await?;
    let items = sqlx::query_as::<_, ApplicationEnvFileResponse>(
        "SELECT f.id,f.application_id,f.file_name,f.module,f.format,f.current_version,f.current_digest,f.declared_at,f.updated_at,f.version,(SELECT COUNT(*) FROM deployment_targets t WHERE t.application_id=f.application_id AND t.status='active') target_count,(SELECT COUNT(*) FROM application_env_syncs s JOIN application_env_versions v ON v.id=s.env_version_id WHERE v.env_file_id=f.id AND v.env_version=f.current_version AND s.status='pending') pending_count,(SELECT COUNT(*) FROM application_env_syncs s JOIN application_env_versions v ON v.id=s.env_version_id WHERE v.env_file_id=f.id AND v.env_version=f.current_version AND s.status='syncing') syncing_count,(SELECT COUNT(*) FROM application_env_syncs s JOIN application_env_versions v ON v.id=s.env_version_id WHERE v.env_file_id=f.id AND v.env_version=f.current_version AND s.status='succeeded') succeeded_count,(SELECT COUNT(*) FROM application_env_syncs s JOIN application_env_versions v ON v.id=s.env_version_id WHERE v.env_file_id=f.id AND v.env_version=f.current_version AND s.status='failed') failed_count FROM application_env_files f WHERE f.application_id=? AND f.deleted_at IS NULL ORDER BY f.file_name COLLATE NOCASE,f.id",
    )
    .bind(&application_id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(ApplicationEnvFileListResponse { items }))
}

#[utoipa::path(operation_id = "application_envs_reauthenticate", post, path = "/api/v1/applications/{application_id}/env-reveal-grants", params(("application_id" = String, Path)), request_body = EnvReauthenticateRequest, responses((status = 200, body = EnvRevealGrantResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 429, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn reauthenticate(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<EnvReauthenticateRequest>,
) -> ApiResult<Response> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    ensure_application(state.pool(), &application_id, request_id.as_str()).await?;
    enforce_reauth_rate_limit(state.pool(), &actor.session_id, request_id.as_str()).await?;

    let row: Option<(String, i64)> = sqlx::query_as(
        "SELECT password_hash,version FROM users WHERE id=? AND identity='administrator' AND status='active'",
    )
    .bind(&actor.id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let Some((password_hash, user_version)) = row else {
        return Err(ApiError::forbidden(request_id.as_str()));
    };
    let verified = PasswordHash::new(&password_hash).ok().is_some_and(|hash| {
        Argon2::default()
            .verify_password(payload.password.as_bytes(), &hash)
            .is_ok()
    });
    if !verified {
        record_reauth_failure(state.pool(), &actor.session_id, request_id.as_str()).await?;
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "env_reauthentication_failed",
            "管理员密码验证失败",
            request_id.as_str(),
        ));
    }
    sqlx::query("DELETE FROM application_env_reauth_attempts WHERE session_id=?")
        .bind(&actor.session_id)
        .execute(state.pool())
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;

    let token = random_token();
    let expires_at =
        (Utc::now() + ChronoDuration::from_std(GRANT_LIFETIME).expect("固定时长")).to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    sqlx::query("INSERT INTO application_env_reveal_grants (id,token_hash,user_id,session_id,application_id,action_scope,user_version,expires_at) VALUES (?,?,?,?,?,?,?,?)")
        .bind(format!("egrant_{}", Ulid::new())).bind(token_hash(&token)).bind(&actor.id).bind(&actor.session_id).bind(&application_id).bind(payload.action.as_str()).bind(user_version).bind(&expires_at)
        .execute(&mut *transaction).await.map_err(|_|ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "application_env.reauthenticate",
        "application",
        &application_id,
        request_id.as_str(),
        json!({"action":payload.action.as_str(),"expires_in_seconds":GRANT_LIFETIME.as_secs()}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(no_store(
        Json(EnvRevealGrantResponse {
            grant_token: token,
            action: payload.action,
            expires_at,
        })
        .into_response(),
    ))
}

#[utoipa::path(operation_id = "application_envs_reveal", get, path = "/api/v1/application-env-files/{env_file_id}", params(("env_file_id" = String, Path), ("X-Env-Reveal-Grant" = String, Header), ("X-CSRF-Token" = String, Header)), responses((status = 200, body = ApplicationEnvPlaintextResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn reveal(
    State(state): State<AppState>,
    Path(env_file_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<Response> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let row = load_current_version(state.pool(), &env_file_id, request_id.as_str()).await?;
    verify_grant(
        state.pool(),
        &headers,
        &actor,
        &row.application_id,
        "read_write",
        request_id.as_str(),
    )
    .await?;
    let content = decrypt_row(&state, &row, request_id.as_str())?;
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(&mut transaction,Some(&actor.id),"application_env.reveal","application_env_file",&row.env_file_id,request_id.as_str(),json!({"application_id":row.application_id,"file_name":row.file_name,"env_version":row.env_version})).await.map_err(|_|ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(no_store(
        Json(ApplicationEnvPlaintextResponse {
            id: row.env_file_id,
            application_id: row.application_id,
            file_name: row.file_name,
            module: row.module,
            format: row.format,
            content: String::from_utf8(content.to_vec())
                .map_err(|_| ApiError::internal(request_id.as_str()))?,
            digest: row.digest,
            env_version: row.env_version,
            version: row.file_version,
            updated_at: row.updated_at,
        })
        .into_response(),
    ))
}

#[utoipa::path(operation_id = "application_envs_update", put, path = "/api/v1/application-env-files/{env_file_id}", params(("env_file_id" = String, Path), ("X-Env-Reveal-Grant" = String, Header)), request_body = UpdateApplicationEnvRequest, responses((status = 200, body = ApplicationEnvPlaintextResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update(
    State(state): State<AppState>,
    Path(env_file_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<UpdateApplicationEnvRequest>,
) -> ApiResult<Response> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let current = load_current_version(state.pool(), &env_file_id, request_id.as_str()).await?;
    verify_grant(
        state.pool(),
        &headers,
        &actor,
        &current.application_id,
        "read_write",
        request_id.as_str(),
    )
    .await?;
    validate_content(&payload.content, request_id.as_str())?;
    if payload.expected_version != current.file_version {
        return Err(version_conflict(request_id.as_str()));
    }
    let digest = hex_digest(payload.content.as_bytes());
    let env_version = current.env_version + 1;
    let version_id = format!("envv_{}", Ulid::new());
    let ring = state
        .master_key_ring()
        .ok_or_else(|| ApiError::service_not_ready(request_id.as_str()))?;
    let encrypted = ring
        .encrypt_application_env(
            &current.application_id,
            &env_file_id,
            &version_id,
            payload.content.as_bytes(),
        )
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let now = Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result=sqlx::query("UPDATE application_env_files SET current_version=?,current_digest=?,updated_at=?,version=version+1 WHERE id=? AND deleted_at IS NULL AND version=?").bind(env_version).bind(&digest).bind(&now).bind(&env_file_id).bind(payload.expected_version).execute(&mut *transaction).await.map_err(|_|ApiError::internal(request_id.as_str()))?;
    if result.rows_affected() != 1 {
        return Err(version_conflict(request_id.as_str()));
    }
    sqlx::query("INSERT INTO application_env_versions (id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest,created_by,created_at) VALUES (?,?,?,?,?,?,?,?,?,?)").bind(&version_id).bind(&env_file_id).bind(env_version).bind(APPLICATION_ENV_ALGORITHM).bind(encrypted.ciphertext).bind(encrypted.nonce).bind(encrypted.key_version).bind(&digest).bind(&actor.id).bind(&now).execute(&mut *transaction).await.map_err(|_|ApiError::internal(request_id.as_str()))?;
    create_sync_rows(&mut transaction, &version_id, &current.application_id)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let target_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deployment_targets WHERE application_id=? AND status='active'",
    )
    .bind(&current.application_id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(&mut transaction,Some(&actor.id),"application_env.update","application_env_file",&env_file_id,request_id.as_str(),json!({"application_id":current.application_id,"file_name":current.file_name,"old_env_version":current.env_version,"new_env_version":env_version,"old_digest":current.digest,"new_digest":digest,"target_count":target_count})).await.map_err(|_|ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let row = load_current_version(state.pool(), &env_file_id, request_id.as_str()).await?;
    Ok(no_store(
        Json(ApplicationEnvPlaintextResponse {
            id: row.env_file_id,
            application_id: row.application_id,
            file_name: row.file_name,
            module: row.module,
            format: row.format,
            content: payload.content,
            digest: row.digest,
            env_version: row.env_version,
            version: row.file_version,
            updated_at: row.updated_at,
        })
        .into_response(),
    ))
}

#[utoipa::path(operation_id = "application_envs_delete", delete, path = "/api/v1/application-env-files/{env_file_id}", params(("env_file_id" = String, Path), ("X-Env-Reveal-Grant" = String, Header)), request_body = DeleteApplicationEnvRequest, responses((status = 204), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn delete_env(
    State(state): State<AppState>,
    Path(env_file_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<DeleteApplicationEnvRequest>,
) -> ApiResult<Response> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let current = load_current_version(state.pool(), &env_file_id, request_id.as_str()).await?;
    verify_grant(
        state.pool(),
        &headers,
        &actor,
        &current.application_id,
        "delete",
        request_id.as_str(),
    )
    .await?;
    if payload.confirm_file_name != current.file_name {
        return Err(ApiError::validation(
            "确认文件名不匹配",
            request_id.as_str(),
        ));
    }
    let now = Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result=sqlx::query("UPDATE application_env_files SET deleted_at=?,updated_at=?,version=version+1 WHERE id=? AND deleted_at IS NULL AND version=?").bind(&now).bind(&now).bind(&env_file_id).bind(payload.expected_version).execute(&mut *transaction).await.map_err(|_|ApiError::internal(request_id.as_str()))?;
    if result.rows_affected() != 1 {
        return Err(version_conflict(request_id.as_str()));
    }
    let target_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deployment_targets WHERE application_id=? AND status='active'",
    )
    .bind(&current.application_id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(&mut transaction,Some(&actor.id),"application_env.delete","application_env_file",&env_file_id,request_id.as_str(),json!({"application_id":current.application_id,"file_name":current.file_name,"env_version":current.env_version,"digest":current.digest,"target_count":target_count})).await.map_err(|_|ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}

#[utoipa::path(operation_id = "application_envs_register", post, path = "/api/v1/agent/env-registration-leases/{lease_id}/register", params(("lease_id" = String, Path), ("Authorization" = String, Header)), request_body = RegisterApplicationEnvsRequest, responses((status = 200, body = RegisterApplicationEnvsResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn register(
    State(state): State<AppState>,
    Path(lease_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    crate::http::ApiJson(payload): crate::http::ApiJson<RegisterApplicationEnvsRequest>,
) -> ApiResult<Json<RegisterApplicationEnvsResponse>> {
    let identity = authenticate_access(state.pool(), &headers, request_id.as_str()).await?;
    let lease =
        load_registration_lease(state.pool(), &lease_id, &identity, request_id.as_str()).await?;
    let manifest_digest = hex_digest(payload.manifest_json.as_bytes());
    if manifest_digest != lease.manifest_digest {
        return Err(ApiError::conflict(
            "env_manifest_mismatch",
            "Env 清单与授权不匹配",
            request_id.as_str(),
        ));
    }
    let manifest: EnvManifest = serde_json::from_str(&payload.manifest_json)
        .map_err(|_| ApiError::validation("Env 清单格式不正确", request_id.as_str()))?;
    validate_registration(&manifest, &payload.files, &lease, request_id.as_str())?;
    let ring = state
        .master_key_ring()
        .ok_or_else(|| ApiError::service_not_ready(request_id.as_str()))?;
    let contents = registration_content_map(&manifest, &payload.files, request_id.as_str())?;
    let now = Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let consumed=sqlx::query("UPDATE application_env_registration_leases SET status='consumed',consumed_at=? WHERE id=? AND agent_id=? AND status='active' AND expires_at>?").bind(&now).bind(&lease_id).bind(&identity.agent_id).bind(&now).execute(&mut *transaction).await.map_err(|_|ApiError::internal(request_id.as_str()))?;
    if consumed.rows_affected() != 1 {
        return Err(ApiError::conflict(
            "env_registration_lease_unavailable",
            "Env 登记授权已失效",
            request_id.as_str(),
        ));
    }
    let mut created = Vec::new();
    let mut declared = Vec::new();
    for entry in &manifest.files {
        let existing:Option<(String,i64)>=sqlx::query_as("SELECT id,current_version FROM application_env_files WHERE application_id=? AND file_name=?").bind(&lease.application_id).bind(&entry.file_name).fetch_optional(&mut *transaction).await.map_err(|_|ApiError::internal(request_id.as_str()))?;
        if let Some((id, _)) = existing {
            if contents.contains_key(&entry.file_name.to_ascii_lowercase()) {
                return Err(ApiError::conflict(
                    "env_plaintext_not_accepted",
                    "已登记 Env 只能确认声明，不能再次上传明文",
                    request_id.as_str(),
                ));
            }
            sqlx::query("UPDATE application_env_files SET module=?,declared_at=?,last_declared_deployment_id=?,last_declared_commit_sha=?,last_manifest_digest=?,updated_at=? WHERE id=?").bind(&entry.module).bind(&now).bind(&lease.deployment_id).bind(&lease.commit_sha).bind(&manifest_digest).bind(&now).bind(&id).execute(&mut *transaction).await.map_err(|_|ApiError::internal(request_id.as_str()))?;
            declared.push(entry.file_name.clone());
            continue;
        }
        let encoded = contents
            .get(&entry.file_name.to_ascii_lowercase())
            .ok_or_else(|| {
                ApiError::validation("首次登记缺少 Env 文件内容", request_id.as_str())
            })?;
        let content = STANDARD
            .decode(encoded)
            .map_err(|_| ApiError::validation("Env 文件编码不正确", request_id.as_str()))?;
        if content.len() != entry.size || hex_digest(&content) != entry.sha256 {
            return Err(ApiError::conflict(
                "env_content_digest_mismatch",
                "Env 文件摘要不匹配",
                request_id.as_str(),
            ));
        }
        let text = std::str::from_utf8(&content)
            .map_err(|_| ApiError::validation("Env 文件必须是 UTF-8", request_id.as_str()))?;
        validate_content(text, request_id.as_str())?;
        let file_id = format!("envf_{}", Ulid::new());
        let version_id = format!("envv_{}", Ulid::new());
        let encrypted = ring
            .encrypt_application_env(&lease.application_id, &file_id, &version_id, &content)
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        sqlx::query("INSERT INTO application_env_files (id,application_id,file_name,module,format,current_version,current_digest,declared_at,last_declared_deployment_id,last_declared_commit_sha,last_manifest_digest,created_at,updated_at) VALUES (?,?,?,?,?,1,?,?,?,?,?,?,?)").bind(&file_id).bind(&lease.application_id).bind(&entry.file_name).bind(&entry.module).bind(&entry.format).bind(&entry.sha256).bind(&now).bind(&lease.deployment_id).bind(&lease.commit_sha).bind(&manifest_digest).bind(&now).bind(&now).execute(&mut *transaction).await.map_err(|error|if is_unique(&error){ApiError::conflict("env_file_already_registered","Env 文件已由其他请求登记",request_id.as_str())}else{ApiError::internal(request_id.as_str())})?;
        sqlx::query("INSERT INTO application_env_versions (id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest,created_at) VALUES (?,?,1,?,?,?,?,?,?)").bind(&version_id).bind(&file_id).bind(APPLICATION_ENV_ALGORITHM).bind(encrypted.ciphertext).bind(encrypted.nonce).bind(encrypted.key_version).bind(&entry.sha256).bind(&now).execute(&mut *transaction).await.map_err(|_|ApiError::internal(request_id.as_str()))?;
        create_sync_rows(&mut transaction, &version_id, &lease.application_id)
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?;
        created.push(entry.file_name.clone());
    }
    audit::record(&mut transaction,None,"application_env.register","application",&lease.application_id,request_id.as_str(),json!({"deployment_id":lease.deployment_id,"agent_id":identity.agent_id,"commit_sha":lease.commit_sha,"manifest_digest":manifest_digest,"created":created,"declared":declared})).await.map_err(|_|ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(RegisterApplicationEnvsResponse { created, declared }))
}

pub async fn create_registration_lease(
    transaction: &mut Transaction<'_, Sqlite>,
    application_id: &str,
    deployment_id: &str,
    agent_id: &str,
    commit_sha: &str,
    manifest_digest: &str,
    expires_at: &str,
) -> sqlx::Result<String> {
    let id = format!("envreg_{}", Ulid::new());
    sqlx::query("INSERT INTO application_env_registration_leases (id,application_id,deployment_id,agent_id,purpose,commit_sha,manifest_digest,status,expires_at) VALUES (?,?,?,?,'env_registration',?,?,'active',?)").bind(&id).bind(application_id).bind(deployment_id).bind(agent_id).bind(commit_sha).bind(manifest_digest).bind(expires_at).execute(&mut **transaction).await?;
    Ok(id)
}

async fn load_registration_lease(
    pool: &sqlx::SqlitePool,
    id: &str,
    identity: &AgentAccessIdentity,
    request_id: &str,
) -> ApiResult<RegistrationLease> {
    sqlx::query_as("SELECT application_id,deployment_id,commit_sha,manifest_digest FROM application_env_registration_leases WHERE id=? AND agent_id=? AND purpose='env_registration' AND status='active' AND expires_at>?").bind(id).bind(&identity.agent_id).bind(Utc::now().to_rfc3339()).fetch_optional(pool).await.map_err(|_|ApiError::internal(request_id))?.ok_or_else(||ApiError::new(StatusCode::NOT_FOUND,"env_registration_lease_not_found","Env 登记授权不存在",request_id))
}

fn validate_registration(
    manifest: &EnvManifest,
    files: &[RegisterApplicationEnvContent],
    lease: &RegistrationLease,
    request_id: &str,
) -> ApiResult<()> {
    if manifest.schema_version != 1
        || manifest.commit_sha != lease.commit_sha
        || !(40..=64).contains(&manifest.commit_sha.len())
        || !manifest
            .commit_sha
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        || manifest.files.is_empty()
        || manifest.files.len() > MAX_ENV_FILES
        || files.len() > manifest.files.len()
    {
        return Err(ApiError::validation("Env 清单不符合约束", request_id));
    }
    let mut names = std::collections::HashSet::new();
    for entry in &manifest.files {
        if !dotenv::validate_file_name(&entry.file_name)
            || !dotenv::validate_module(&entry.module)
            || entry.format != "dotenv-v1"
            || entry.size > dotenv::MAX_ENV_BYTES
            || entry.sha256.len() != 64
            || !entry.sha256.bytes().all(|b| {
                b.is_ascii_hexdigit() && (!b.is_ascii_alphabetic() || b.is_ascii_lowercase())
            })
            || !names.insert(entry.file_name.to_ascii_lowercase())
        {
            return Err(ApiError::validation("Env 清单文件不符合约束", request_id));
        }
    }
    Ok(())
}

fn registration_content_map<'a>(
    manifest: &EnvManifest,
    files: &'a [RegisterApplicationEnvContent],
    request_id: &str,
) -> ApiResult<std::collections::HashMap<String, &'a str>> {
    let mut by_name = std::collections::HashMap::new();
    for file in files {
        if !manifest
            .files
            .iter()
            .any(|entry| entry.file_name.eq_ignore_ascii_case(&file.file_name))
            || by_name
                .insert(
                    file.file_name.to_ascii_lowercase(),
                    file.content_base64.as_str(),
                )
                .is_some()
        {
            return Err(ApiError::validation(
                "Env 文件内容重复或不在清单中",
                request_id,
            ));
        }
    }
    Ok(by_name)
}

async fn ensure_application(pool: &sqlx::SqlitePool, id: &str, request_id: &str) -> ApiResult<()> {
    let found: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM applications WHERE id=? AND status='active')",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    if found {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "application_not_found",
            "应用不存在",
            request_id,
        ))
    }
}

async fn load_current_version(
    pool: &sqlx::SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<EnvVersionRow> {
    sqlx::query_as("SELECT f.id env_file_id,f.application_id,f.file_name,f.module,f.format,f.current_version env_version,v.digest,v.ciphertext,v.nonce,v.key_version,v.id version_id,f.version file_version,f.updated_at FROM application_env_files f JOIN application_env_versions v ON v.env_file_id=f.id AND v.env_version=f.current_version WHERE f.id=? AND f.deleted_at IS NULL").bind(id).fetch_optional(pool).await.map_err(|_|ApiError::internal(request_id))?.ok_or_else(||ApiError::new(StatusCode::NOT_FOUND,"application_env_not_found","Env 文件不存在",request_id))
}

fn decrypt_row(
    state: &AppState,
    row: &EnvVersionRow,
    request_id: &str,
) -> ApiResult<Zeroizing<Vec<u8>>> {
    let ring = state
        .master_key_ring()
        .ok_or_else(|| ApiError::service_not_ready(request_id))?;
    ring.decrypt_application_env(
        &row.application_id,
        &row.env_file_id,
        &row.version_id,
        &EncryptedSecret {
            ciphertext: row.ciphertext.clone(),
            nonce: row.nonce.clone(),
            key_version: row.key_version,
        },
    )
    .map_err(|_| ApiError::internal(request_id))
}

async fn verify_grant(
    pool: &sqlx::SqlitePool,
    headers: &HeaderMap,
    actor: &AuthUser,
    application_id: &str,
    scope: &str,
    request_id: &str,
) -> ApiResult<()> {
    let token = headers
        .get("x-env-reveal-grant")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::forbidden(request_id))?;
    let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM application_env_reveal_grants g JOIN sessions s ON s.id=g.session_id JOIN users u ON u.id=g.user_id WHERE g.token_hash=? AND g.user_id=? AND g.session_id=? AND g.application_id=? AND g.action_scope=? AND g.revoked_at IS NULL AND g.expires_at>? AND g.user_version=u.version AND u.identity='administrator' AND u.status='active' AND s.revoked_at IS NULL AND s.expires_at>?)").bind(token_hash(token)).bind(&actor.id).bind(&actor.session_id).bind(application_id).bind(scope).bind(Utc::now().to_rfc3339()).bind(Utc::now().to_rfc3339()).fetch_one(pool).await.map_err(|_|ApiError::internal(request_id))?;
    if valid {
        Ok(())
    } else {
        Err(ApiError::forbidden(request_id))
    }
}

async fn enforce_reauth_rate_limit(
    pool: &sqlx::SqlitePool,
    session_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let blocked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM application_env_reauth_attempts WHERE session_id=? AND blocked_until>?)").bind(session_id).bind(Utc::now().to_rfc3339()).fetch_one(pool).await.map_err(|_|ApiError::internal(request_id))?;
    if blocked {
        Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "env_reauthentication_rate_limited",
            "重新认证尝试过于频繁",
            request_id,
        ))
    } else {
        Ok(())
    }
}

async fn record_reauth_failure(
    pool: &sqlx::SqlitePool,
    session_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let now = Utc::now();
    let window = (now - ChronoDuration::minutes(REAUTH_WINDOW_MINUTES)).to_rfc3339();
    let block = (now + ChronoDuration::minutes(REAUTH_BLOCK_MINUTES)).to_rfc3339();
    sqlx::query("INSERT INTO application_env_reauth_attempts (session_id,failed_count,window_started_at,blocked_until) VALUES (?,1,?,NULL) ON CONFLICT(session_id) DO UPDATE SET failed_count=CASE WHEN window_started_at<? THEN 1 ELSE failed_count+1 END,window_started_at=CASE WHEN window_started_at<? THEN excluded.window_started_at ELSE window_started_at END,blocked_until=CASE WHEN (CASE WHEN window_started_at<? THEN 1 ELSE failed_count+1 END)>=? THEN ? ELSE NULL END").bind(session_id).bind(now.to_rfc3339()).bind(&window).bind(&window).bind(&window).bind(MAX_REAUTH_FAILURES).bind(block).execute(pool).await.map_err(|_|ApiError::internal(request_id))?;
    Ok(())
}

async fn create_sync_rows(
    transaction: &mut Transaction<'_, Sqlite>,
    version_id: &str,
    application_id: &str,
) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO application_env_syncs (id,env_version_id,target_id,node_id,agent_id,status) SELECT 'envsync_'||lower(hex(randomblob(16))),?,t.id,t.node_id,a.id,'pending' FROM deployment_targets t LEFT JOIN agents a ON a.node_id=t.node_id AND a.revoked_at IS NULL AND a.archived_at IS NULL WHERE t.application_id=? AND t.status='active'").bind(version_id).bind(application_id).execute(&mut **transaction).await?;
    Ok(())
}

fn validate_content(content: &str, request_id: &str) -> ApiResult<()> {
    dotenv::validate(content).map_err(|errors| {
        let details = json!({"field_errors":{"content":errors}});
        ApiError::validation("Env 内容不符合 dotenv-v1 约束", request_id).with_details(details)
    })
}
fn version_conflict(request_id: &str) -> ApiError {
    ApiError::conflict(
        "resource_version_conflict",
        "Env 已被其他请求修改",
        request_id,
    )
}
fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    response
}
fn random_token() -> String {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}
fn hex_digest(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn is_unique(error: &sqlx::Error) -> bool {
    matches!(error,sqlx::Error::Database(db) if db.is_unique_violation())
}

#[cfg(test)]
mod tests {
    use crate::crypto::MasterKeyRing;

    #[test]
    fn application_env_aad_binds_all_immutable_ids_and_uses_unique_nonce() {
        let ring = MasterKeyRing::from_raw(2, [3; 32], Some((1, [2; 32]))).unwrap();
        let first = ring
            .encrypt_application_env("app_a", "env_a", "ver_a", b"SECRET=value")
            .unwrap();
        let second = ring
            .encrypt_application_env("app_a", "env_a", "ver_a", b"SECRET=value")
            .unwrap();
        assert_ne!(first.nonce, second.nonce);
        assert_eq!(
            &*ring
                .decrypt_application_env("app_a", "env_a", "ver_a", &first)
                .unwrap(),
            b"SECRET=value"
        );
        assert!(
            ring.decrypt_application_env("app_b", "env_a", "ver_a", &first)
                .is_err()
        );
        assert!(
            ring.decrypt_application_env("app_a", "env_b", "ver_a", &first)
                .is_err()
        );
        assert!(
            ring.decrypt_application_env("app_a", "env_a", "ver_b", &first)
                .is_err()
        );
    }

    #[test]
    fn previous_key_decrypts_and_file_name_is_not_aad() {
        let old = MasterKeyRing::from_raw(1, [2; 32], None).unwrap();
        let encrypted = old
            .encrypt_application_env("app_a", "env_a", "ver_a", b"A=1")
            .unwrap();
        let rotated = MasterKeyRing::from_raw(2, [3; 32], Some((1, [2; 32]))).unwrap();
        assert_eq!(
            &*rotated
                .decrypt_application_env("app_a", "env_a", "ver_a", &encrypted)
                .unwrap(),
            b"A=1"
        );
    }
}
