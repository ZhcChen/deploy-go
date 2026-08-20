use std::collections::{HashMap, HashSet};

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration as ChronoDuration, Utc};
use deploy_go_container_template::{
    ImageDeploySpec as PlatformImageDeploySpec, TemplateFileRole, template_descriptor,
    template_from_id,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use sha2::{Digest, Sha256};
use sqlx::{Sqlite, Transaction};
use ulid::Ulid;
use utoipa::ToSchema;
use zeroize::Zeroizing;

use crate::{
    AppState, RequestId, audit,
    auth::AuthUser,
    crypto::{APPLICATION_CONFIG_ALGORITHM, EncryptedSecret},
    error::{ApiError, ApiResult},
    grants,
};

const MAX_CONFIG_BYTES: usize = 1024 * 1024;
const CONFIG_GRANT_LIFETIME_MINUTES: i64 = 5;
const REAUTH_WINDOW_MINUTES: i64 = 15;
const REAUTH_BLOCK_MINUTES: i64 = 15;
const MAX_REAUTH_FAILURES: i64 = 5;
const DEFAULT_SECRET_BYTES: usize = 32;
const MAX_SECRET_BYTES: usize = 96;

pub async fn reencrypt_all(
    pool: &sqlx::SqlitePool,
    ring: &crate::crypto::MasterKeyRing,
) -> anyhow::Result<u64> {
    let mut migrated = 0_u64;
    loop {
        let rows = sqlx::query_as::<_, ConfigVersionRow>(
            "SELECT id,application_config_file_id,application_id,config_version,algorithm,ciphertext,nonce,key_version,digest,source,source_version_id,source_template_digest,created_by,created_at FROM application_config_versions WHERE key_version != ? ORDER BY id LIMIT 100",
        )
        .bind(ring.current_version())
        .fetch_all(pool)
        .await?;
        if rows.is_empty() {
            return Ok(migrated);
        }
        for row in rows {
            if row.algorithm != APPLICATION_CONFIG_ALGORITHM {
                anyhow::bail!("应用配置加密算法无效");
            }
            let previous_key_version = row.key_version;
            let plaintext = ring.decrypt_application_config(
                &row.application_id,
                &row.application_config_file_id,
                &row.id,
                &EncryptedSecret {
                    ciphertext: row.ciphertext,
                    nonce: row.nonce,
                    key_version: previous_key_version,
                },
            )?;
            let replacement = ring.encrypt_application_config(
                &row.application_id,
                &row.application_config_file_id,
                &row.id,
                plaintext.as_slice(),
            )?;
            let result = sqlx::query("UPDATE application_config_versions SET ciphertext=?,nonce=?,key_version=? WHERE id=? AND key_version=?")
                .bind(replacement.ciphertext)
                .bind(replacement.nonce)
                .bind(replacement.key_version)
                .bind(&row.id)
                .bind(previous_key_version)
                .execute(pool)
                .await?;
            migrated += result.rows_affected();
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ConfigFileRow {
    id: String,
    binding_id: String,
    application_id: String,
    path: String,
    deploy_path: Option<String>,
    label: String,
    format: String,
    language: String,
    role: String,
    delivery: String,
    sensitive: i64,
    editable: i64,
    description: String,
    recommended_changes: String,
    template_source_digest: String,
    current_version: i64,
    current_digest: String,
    status: String,
    deleted_at: Option<String>,
    updated_at: String,
    version: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ConfigVersionRow {
    id: String,
    application_config_file_id: String,
    application_id: String,
    config_version: i64,
    algorithm: String,
    ciphertext: Vec<u8>,
    nonce: Vec<u8>,
    key_version: i64,
    digest: String,
    source: String,
    source_version_id: Option<String>,
    source_template_digest: Option<String>,
    created_by: Option<String>,
    created_at: String,
}

type LegacyEnvRow = (String, String, String, Vec<u8>, Vec<u8>, i64);

struct CloneTemplateOptions<'a> {
    application_id: &'a str,
    template_id: &'a str,
    target_id: Option<&'a str>,
    actor_id: Option<&'a str>,
    source: &'a str,
    overrides: &'a HashMap<String, Vec<u8>>,
    request_id: &'a str,
}

struct NewConfigVersion<'a> {
    config_version: i64,
    version_id: &'a str,
    encrypted: EncryptedSecret,
    digest: &'a str,
    source: &'a str,
    source_version_id: Option<&'a str>,
    source_template_digest: Option<&'a str>,
    created_by: Option<&'a str>,
    created_at: &'a str,
}

struct SaveContentOptions<'a> {
    content: &'a str,
    expected_version: i64,
    source: &'a str,
    consume_grant: Option<&'a str>,
    actor_id: Option<&'a str>,
    request_id: &'a str,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApplicationConfigFileResponse {
    pub id: String,
    pub binding_id: String,
    pub application_id: String,
    pub path: String,
    pub deploy_path: Option<String>,
    pub label: String,
    pub format: String,
    pub language: String,
    pub role: String,
    pub delivery: String,
    pub sensitive: bool,
    pub editable: bool,
    pub description: String,
    pub recommended_changes: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_source_digest: Option<String>,
    pub current_version: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_digest: Option<String>,
    pub status: String,
    pub deleted_at: Option<String>,
    pub updated_at: String,
    pub version: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApplicationConfigFileListResponse {
    pub items: Vec<ApplicationConfigFileResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApplicationConfigVersionResponse {
    pub id: String,
    pub application_config_file_id: String,
    pub config_version: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    pub source: String,
    pub source_version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_template_digest: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApplicationConfigVersionListResponse {
    pub items: Vec<ApplicationConfigVersionResponse>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateApplicationConfigRequest {
    pub content: String,
    pub expected_version: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RestoreApplicationConfigRequest {
    pub version: Option<i64>,
    pub template_version: Option<String>,
    pub expected_version: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InitializeApplicationConfigsRequest {
    pub target_id: String,
    #[serde(default)]
    pub template_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InitializeApplicationConfigsResponse {
    pub binding_id: String,
    pub created: bool,
    pub status: String,
    pub file_count: usize,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteApplicationConfigWorkspaceRequest {
    pub binding_id: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConfigGrantAction {
    ReadWrite,
}

impl ConfigGrantAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::ReadWrite => "read_write",
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigReauthenticateRequest {
    pub password: String,
    #[serde(default = "default_config_grant_action")]
    pub action: ConfigGrantAction,
}

fn default_config_grant_action() -> ConfigGrantAction {
    ConfigGrantAction::ReadWrite
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConfigRevealGrantResponse {
    pub grant_token: String,
    pub action: ConfigGrantAction,
    pub expires_at: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ControlledPatchRequest {
    pub key: String,
    pub value: String,
    pub expected_version: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ValidateApplicationConfigRequest {
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConfigDiagnostic {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApplicationConfigValidationResponse {
    pub valid: bool,
    pub diagnostics: Vec<ConfigDiagnostic>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateSecretRequest {
    pub key: String,
    pub expected_version: i64,
    #[serde(default)]
    pub bytes: Option<usize>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GenerateSecretResponse {
    pub file: ApplicationConfigFileResponse,
    pub key: String,
    pub secret: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConfigDiffQuery {
    pub version: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApplicationConfigDiffResponse {
    pub file_id: String,
    pub current_version: i64,
    pub compare_version: Option<i64>,
    pub sensitive: bool,
    pub changed: bool,
    pub current_content: String,
    pub compare_content: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/applications/{application_id}/config-files",
            get(list).delete(delete_workspace),
        )
        .route(
            "/applications/{application_id}/config-files/initialize",
            post(initialize),
        )
        .route(
            "/applications/{application_id}/config-files/validate",
            post(validate_all),
        )
        .route(
            "/applications/{application_id}/config-reveal-grants",
            post(reauthenticate),
        )
        .route("/application-config-files/{id}", get(show).put(update))
        .route(
            "/application-config-files/{id}/controlled-patch",
            post(controlled_patch),
        )
        .route(
            "/application-config-files/{id}/validate",
            post(validate_file),
        )
        .route("/application-config-files/{id}/diff", get(diff))
        .route(
            "/application-config-files/{id}/generate-secret",
            post(generate_secret),
        )
        .route("/application-config-files/{id}/versions", get(versions))
        .route("/application-config-files/{id}/restore", post(restore))
}

#[utoipa::path(
    operation_id = "application_configs_list",
    get,
    path = "/api/v1/applications/{application_id}/config-files",
    params(("application_id" = String, Path)),
    responses((status = 200, body = ApplicationConfigFileListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse))
)]
pub(crate) async fn list(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Response> {
    grants::require_application_access(state.pool(), &actor, &application_id, request_id.as_str())
        .await?;
    let rows = sqlx::query_as::<_, ConfigFileRow>(&config_file_select(
        "WHERE f.application_id=? AND f.deleted_at IS NULL ORDER BY f.path",
    ))
    .bind(&application_id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(no_store(
        Json(ApplicationConfigFileListResponse {
            items: rows.into_iter().map(file_response).collect(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    operation_id = "application_configs_show",
    get,
    path = "/api/v1/application-config-files/{id}",
    params(("id" = String, Path), ("X-Env-Reveal-Grant" = Option<String>, Header), ("X-CSRF-Token" = Option<String>, Header)),
    responses((status = 200, body = ApplicationConfigFileResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse))
)]
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<Response> {
    let row = load_file(state.pool(), &id, request_id.as_str()).await?;
    grants::require_application_access(
        state.pool(),
        &actor,
        &row.application_id,
        request_id.as_str(),
    )
    .await?;
    let content = if row.sensitive == 0 {
        Some(read_version_content(&state, &row, request_id.as_str()).await?)
    } else {
        actor.require_administrator(request_id.as_str())?;
        actor.verify_csrf(&headers, request_id.as_str())?;
        verify_config_grant(
            &headers,
            &state,
            &actor,
            &row.application_id,
            request_id.as_str(),
        )
        .await?;
        Some(read_version_content(&state, &row, request_id.as_str()).await?)
    };
    Ok(no_store(
        Json(file_response_with_content(row, content, true)).into_response(),
    ))
}

#[utoipa::path(
    operation_id = "application_configs_versions",
    get,
    path = "/api/v1/application-config-files/{id}/versions",
    params(("id" = String, Path)),
    responses((status = 200, body = ApplicationConfigVersionListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse))
)]
pub(crate) async fn versions(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Response> {
    let file = load_file(state.pool(), &id, request_id.as_str()).await?;
    grants::require_application_access(
        state.pool(),
        &actor,
        &file.application_id,
        request_id.as_str(),
    )
    .await?;
    let rows = sqlx::query_as::<_, ConfigVersionRow>(
        "SELECT id,application_config_file_id,application_id,config_version,algorithm,ciphertext,nonce,key_version,digest,source,source_version_id,source_template_digest,created_by,created_at FROM application_config_versions WHERE application_config_file_id=? ORDER BY config_version DESC",
    )
    .bind(&id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(no_store(
        Json(ApplicationConfigVersionListResponse {
            items: rows
                .into_iter()
                .map(|row| version_response(row, file.sensitive != 0))
                .collect(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    operation_id = "application_configs_update",
    put,
    path = "/api/v1/application-config-files/{id}",
    params(("id" = String, Path), ("X-Env-Reveal-Grant" = Option<String>, Header), ("X-CSRF-Token" = String, Header)),
    request_body = UpdateApplicationConfigRequest,
    responses((status = 200, body = ApplicationConfigFileResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse))
)]
pub(crate) async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<UpdateApplicationConfigRequest>,
) -> ApiResult<Response> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let current = load_file(state.pool(), &id, request_id.as_str()).await?;
    grants::require_application_access(
        state.pool(),
        &actor,
        &current.application_id,
        request_id.as_str(),
    )
    .await?;
    if current.editable == 0 || current.deleted_at.is_some() {
        return Err(ApiError::forbidden(request_id.as_str()));
    }
    if current.sensitive != 0 {
        verify_config_grant(
            &headers,
            &state,
            &actor,
            &current.application_id,
            request_id.as_str(),
        )
        .await?;
    }
    let response = save_content(
        &state,
        &current,
        SaveContentOptions {
            content: &payload.content,
            expected_version: payload.expected_version,
            source: "user",
            consume_grant: None,
            actor_id: Some(&actor.id),
            request_id: request_id.as_str(),
        },
    )
    .await?;
    Ok(no_store(Json(response).into_response()))
}

#[utoipa::path(
    operation_id = "application_configs_restore",
    post,
    path = "/api/v1/application-config-files/{id}/restore",
    params(("id" = String, Path), ("X-Env-Reveal-Grant" = Option<String>, Header), ("X-CSRF-Token" = String, Header)),
    request_body = RestoreApplicationConfigRequest,
    responses((status = 200, body = ApplicationConfigFileResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse))
)]
pub(crate) async fn restore(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<RestoreApplicationConfigRequest>,
) -> ApiResult<Response> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let current = load_file(state.pool(), &id, request_id.as_str()).await?;
    grants::require_application_access(
        state.pool(),
        &actor,
        &current.application_id,
        request_id.as_str(),
    )
    .await?;
    if current.editable == 0 || current.deleted_at.is_some() {
        return Err(ApiError::forbidden(request_id.as_str()));
    }
    if current.sensitive != 0 {
        verify_config_grant(
            &headers,
            &state,
            &actor,
            &current.application_id,
            request_id.as_str(),
        )
        .await?;
    }
    if payload.version.is_some() == payload.template_version.is_some() {
        return Err(ApiError::validation(
            "恢复必须指定历史版本或模板版本",
            request_id.as_str(),
        ));
    }
    let ring = state
        .master_key_ring()
        .ok_or_else(|| ApiError::service_not_ready(request_id.as_str()))?;
    let (plaintext, source, source_version_id, source_template_digest) = if let Some(version) =
        payload.version
    {
        let old = sqlx::query_as::<_, ConfigVersionRow>(
                "SELECT id,application_config_file_id,application_id,config_version,algorithm,ciphertext,nonce,key_version,digest,source,source_version_id,source_template_digest,created_by,created_at FROM application_config_versions WHERE application_config_file_id=? AND config_version=?",
            )
            .bind(&id)
            .bind(version)
            .fetch_optional(state.pool())
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?
            .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
        let plaintext = decrypt_version(&state, &old, request_id.as_str())?;
        (plaintext, "restore_version", Some(old.id), None)
    } else {
        let template_version = payload.template_version.as_deref().unwrap();
        let (template_id, binding_version): (String, String) = sqlx::query_as(
                "SELECT b.template_id,b.template_version FROM application_template_bindings b JOIN application_config_files f ON f.binding_id=b.id WHERE f.id=?",
            )
            .bind(&id)
            .fetch_optional(state.pool())
            .await
            .map_err(|_| ApiError::internal(request_id.as_str()))?
            .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
        if template_version != binding_version {
            return Err(ApiError::new(
                StatusCode::NOT_FOUND,
                "application_template_version_not_found",
                "模板版本不存在",
                request_id.as_str(),
            ));
        }
        let template = template_from_id(&template_id)
            .map(template_descriptor)
            .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
        let source_file = template
            .files
            .into_iter()
            .find(|file| file.deploy_path.as_deref() == current.deploy_path.as_deref())
            .ok_or_else(|| ApiError::not_found(request_id.as_str()))?;
        (
            Zeroizing::new(source_file.content.into_bytes()),
            "restore_template",
            None,
            Some(source_file.digest),
        )
    };
    let plaintext_text = String::from_utf8(plaintext.to_vec()).map_err(|_| {
        invalid_config(
            request_id.as_str(),
            vec![diagnostic(
                &current.path,
                1,
                1,
                "utf8_invalid",
                "配置正文必须是 UTF-8 文本",
            )],
        )
    })?;
    validate_configuration(&state, &current, &plaintext_text, request_id.as_str()).await?;
    let version_id = format!("cfgv_{}", Ulid::new());
    let encrypted = ring
        .encrypt_application_config(
            &current.application_id,
            &current.id,
            &version_id,
            &plaintext,
        )
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let digest = hex_digest(&plaintext);
    let status = config_status(current.sensitive != 0, &plaintext);
    let now = Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let result = sqlx::query("UPDATE application_config_files SET current_version=?,current_digest=?,status=?,updated_at=?,version=version+1 WHERE id=? AND application_id=? AND deleted_at IS NULL AND editable=1 AND version=?")
        .bind(current.current_version + 1)
        .bind(&digest)
        .bind(status)
        .bind(&now)
        .bind(&id)
        .bind(&current.application_id)
        .bind(payload.expected_version)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if result.rows_affected() != 1 {
        return Err(version_conflict(request_id.as_str()));
    }
    insert_version(
        &mut transaction,
        &current,
        NewConfigVersion {
            config_version: current.current_version + 1,
            version_id: &version_id,
            encrypted,
            digest: &digest,
            source,
            source_version_id: source_version_id.as_deref(),
            source_template_digest: source_template_digest.as_deref(),
            created_by: Some(&actor.id),
            created_at: &now,
        },
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "application_config.restore",
        "application_config_file",
        &current.id,
        request_id.as_str(),
        json!({
            "application_id": current.application_id,
            "path": current.path,
            "new_version": current.current_version + 1,
            "source": source,
            "source_version_id": source_version_id,
            "source_template_digest": source_template_digest.filter(|_| current.sensitive == 0)
        }),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(no_store(
        Json(file_response_with_content(
            load_file(state.pool(), &id, request_id.as_str()).await?,
            (current.sensitive == 0).then_some(String::from_utf8_lossy(&plaintext).into_owned()),
            false,
        ))
        .into_response(),
    ))
}

#[utoipa::path(
    operation_id = "application_configs_reauthenticate",
    post,
    path = "/api/v1/applications/{application_id}/config-reveal-grants",
    params(("application_id" = String, Path), ("X-CSRF-Token" = String, Header)),
    request_body = ConfigReauthenticateRequest,
    responses((status = 200, body = ConfigRevealGrantResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 429, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse))
)]
pub(crate) async fn reauthenticate(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<ConfigReauthenticateRequest>,
) -> ApiResult<Response> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    grants::require_application_access(state.pool(), &actor, &application_id, request_id.as_str())
        .await?;
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
        (Utc::now() + ChronoDuration::minutes(CONFIG_GRANT_LIFETIME_MINUTES)).to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    sqlx::query("INSERT INTO application_env_reveal_grants (id,token_hash,user_id,session_id,application_id,action_scope,user_version,expires_at) VALUES (?,?,?,?,?,?,?,?)")
        .bind(format!("egrant_{}", Ulid::new()))
        .bind(token_hash(&token))
        .bind(&actor.id)
        .bind(&actor.session_id)
        .bind(&application_id)
        .bind(payload.action.as_str())
        .bind(user_version)
        .bind(&expires_at)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "application_config.reauthenticate",
        "application",
        &application_id,
        request_id.as_str(),
        json!({"action":payload.action.as_str(),"expires_in_seconds":CONFIG_GRANT_LIFETIME_MINUTES * 60}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(no_store(
        Json(ConfigRevealGrantResponse {
            grant_token: token,
            action: payload.action,
            expires_at,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    operation_id = "application_configs_controlled_patch",
    post,
    path = "/api/v1/application-config-files/{id}/controlled-patch",
    params(("id" = String, Path), ("X-Env-Reveal-Grant" = Option<String>, Header), ("X-CSRF-Token" = String, Header)),
    request_body = ControlledPatchRequest,
    responses((status = 200, body = ApplicationConfigFileResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse))
)]
pub(crate) async fn controlled_patch(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<ControlledPatchRequest>,
) -> ApiResult<Response> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let current = load_file(state.pool(), &id, request_id.as_str()).await?;
    grants::require_application_access(
        state.pool(),
        &actor,
        &current.application_id,
        request_id.as_str(),
    )
    .await?;
    if current.editable == 0 || current.deleted_at.is_some() {
        return Err(ApiError::forbidden(request_id.as_str()));
    }
    if current.sensitive != 0 {
        verify_config_grant(
            &headers,
            &state,
            &actor,
            &current.application_id,
            request_id.as_str(),
        )
        .await?;
    }
    let content = read_version_content(&state, &current, request_id.as_str()).await?;
    let patched = apply_controlled_patch(&current, &content, &payload.key, &payload.value)
        .map_err(|diagnostics| invalid_config(request_id.as_str(), diagnostics))?;
    let response = save_content(
        &state,
        &current,
        SaveContentOptions {
            content: &patched,
            expected_version: payload.expected_version,
            source: "user",
            consume_grant: None,
            actor_id: Some(&actor.id),
            request_id: request_id.as_str(),
        },
    )
    .await?;
    Ok(no_store(Json(response).into_response()))
}

#[utoipa::path(
    operation_id = "application_configs_validate",
    post,
    path = "/api/v1/application-config-files/{id}/validate",
    params(("id" = String, Path), ("X-Env-Reveal-Grant" = Option<String>, Header), ("X-CSRF-Token" = String, Header)),
    request_body = ValidateApplicationConfigRequest,
    responses((status = 200, body = ApplicationConfigValidationResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse))
)]
pub(crate) async fn validate_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<ValidateApplicationConfigRequest>,
) -> ApiResult<Response> {
    actor.verify_csrf(&headers, request_id.as_str())?;
    let file = load_file(state.pool(), &id, request_id.as_str()).await?;
    grants::require_application_access(
        state.pool(),
        &actor,
        &file.application_id,
        request_id.as_str(),
    )
    .await?;
    if file.sensitive != 0 {
        actor.require_administrator(request_id.as_str())?;
        verify_config_grant(
            &headers,
            &state,
            &actor,
            &file.application_id,
            request_id.as_str(),
        )
        .await?;
    }
    let content = match payload.content {
        Some(content) => content,
        None => read_version_content(&state, &file, request_id.as_str()).await?,
    };
    let diagnostics =
        configuration_diagnostics(&state, &file, &content, request_id.as_str()).await?;
    Ok(no_store(
        Json(ApplicationConfigValidationResponse {
            valid: diagnostics.is_empty(),
            diagnostics,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    operation_id = "application_configs_validate_all",
    post,
    path = "/api/v1/applications/{application_id}/config-files/validate",
    params(("application_id" = String, Path), ("X-Env-Reveal-Grant" = Option<String>, Header), ("X-CSRF-Token" = String, Header)),
    request_body = ValidateApplicationConfigRequest,
    responses((status = 200, body = ApplicationConfigValidationResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse))
)]
pub(crate) async fn validate_all(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(_payload): crate::http::ApiJson<ValidateApplicationConfigRequest>,
) -> ApiResult<Response> {
    actor.verify_csrf(&headers, request_id.as_str())?;
    grants::require_application_access(state.pool(), &actor, &application_id, request_id.as_str())
        .await?;
    let files = sqlx::query_as::<_, ConfigFileRow>(&config_file_select(
        "WHERE f.application_id=? AND f.deleted_at IS NULL ORDER BY f.path",
    ))
    .bind(&application_id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    if files.iter().any(|file| file.sensitive != 0) {
        actor.require_administrator(request_id.as_str())?;
        verify_config_grant(
            &headers,
            &state,
            &actor,
            &application_id,
            request_id.as_str(),
        )
        .await?;
    }
    let mut diagnostics = Vec::new();
    for file in files {
        let content = read_version_content(&state, &file, request_id.as_str()).await?;
        diagnostics
            .extend(configuration_diagnostics(&state, &file, &content, request_id.as_str()).await?);
    }
    Ok(no_store(
        Json(ApplicationConfigValidationResponse {
            valid: diagnostics.is_empty(),
            diagnostics,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    operation_id = "application_configs_diff",
    get,
    path = "/api/v1/application-config-files/{id}/diff",
    params(("id" = String, Path), ("version" = Option<i64>, Query), ("X-Env-Reveal-Grant" = Option<String>, Header), ("X-CSRF-Token" = Option<String>, Header)),
    responses((status = 200, body = ApplicationConfigDiffResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse))
)]
pub(crate) async fn diff(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ConfigDiffQuery>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
) -> ApiResult<Response> {
    let file = load_file(state.pool(), &id, request_id.as_str()).await?;
    grants::require_application_access(
        state.pool(),
        &actor,
        &file.application_id,
        request_id.as_str(),
    )
    .await?;
    if file.sensitive != 0 {
        actor.require_administrator(request_id.as_str())?;
        actor.verify_csrf(&headers, request_id.as_str())?;
        verify_config_grant(
            &headers,
            &state,
            &actor,
            &file.application_id,
            request_id.as_str(),
        )
        .await?;
    }
    let current = read_version_content(&state, &file, request_id.as_str()).await?;
    let (compare, compare_version): (Zeroizing<Vec<u8>>, Option<i64>) =
        if let Some(version) = query.version {
            let row = load_version(state.pool(), &id, version, request_id.as_str()).await?;
            (
                decrypt_version(&state, &row, request_id.as_str())?,
                Some(version),
            )
        } else {
            (
                template_content_for_file(&state, &file, request_id.as_str()).await?,
                None,
            )
        };
    let changed = current.as_bytes() != compare.as_slice();
    let current_content = if file.sensitive != 0 {
        redact_sensitive_content(&current)
    } else {
        current.into_bytes()
    };
    let compare_content = if file.sensitive != 0 {
        redact_sensitive_content(std::str::from_utf8(&compare).unwrap_or_default())
    } else {
        compare.to_vec()
    };
    Ok(no_store(
        Json(ApplicationConfigDiffResponse {
            file_id: file.id,
            current_version: file.current_version,
            compare_version,
            sensitive: file.sensitive != 0,
            changed,
            current_content: String::from_utf8_lossy(&current_content).into_owned(),
            compare_content: String::from_utf8_lossy(&compare_content).into_owned(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    operation_id = "application_configs_generate_secret",
    post,
    path = "/api/v1/application-config-files/{id}/generate-secret",
    params(("id" = String, Path), ("X-Env-Reveal-Grant" = String, Header), ("X-CSRF-Token" = String, Header)),
    request_body = GenerateSecretRequest,
    responses((status = 200, body = GenerateSecretResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse))
)]
pub(crate) async fn generate_secret(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<GenerateSecretRequest>,
) -> ApiResult<Response> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let current = load_file(state.pool(), &id, request_id.as_str()).await?;
    grants::require_application_access(
        state.pool(),
        &actor,
        &current.application_id,
        request_id.as_str(),
    )
    .await?;
    if current.sensitive == 0 || current.editable == 0 || current.deleted_at.is_some() {
        return Err(ApiError::validation(
            "只有可编辑敏感配置文件支持 Secret 生成",
            request_id.as_str(),
        ));
    }
    verify_config_grant(
        &headers,
        &state,
        &actor,
        &current.application_id,
        request_id.as_str(),
    )
    .await?;
    let bytes = payload.bytes.unwrap_or(DEFAULT_SECRET_BYTES);
    if !(16..=MAX_SECRET_BYTES).contains(&bytes) {
        return Err(ApiError::validation(
            "Secret 长度必须在 16 到 96 字节之间",
            request_id.as_str(),
        ));
    }
    let mut random = vec![0_u8; bytes];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut random);
    let secret = URL_SAFE_NO_PAD.encode(&random);
    let content = read_version_content(&state, &current, request_id.as_str()).await?;
    let patched = apply_controlled_patch(&current, &content, &payload.key, &secret)
        .map_err(|diagnostics| invalid_config(request_id.as_str(), diagnostics))?;
    let grant_token = config_grant_token(&headers, request_id.as_str())?;
    let file = save_content(
        &state,
        &current,
        SaveContentOptions {
            content: &patched,
            expected_version: payload.expected_version,
            source: "user",
            consume_grant: Some(&grant_token),
            actor_id: Some(&actor.id),
            request_id: request_id.as_str(),
        },
    )
    .await?;
    Ok(no_store(
        Json(GenerateSecretResponse {
            file,
            key: payload.key,
            secret,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    operation_id = "application_configs_initialize",
    post,
    path = "/api/v1/applications/{application_id}/config-files/initialize",
    params(("application_id" = String, Path), ("X-CSRF-Token" = String, Header)),
    request_body = InitializeApplicationConfigsRequest,
    responses((status = 200, body = InitializeApplicationConfigsResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse))
)]
pub(crate) async fn initialize(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<InitializeApplicationConfigsRequest>,
) -> ApiResult<Json<InitializeApplicationConfigsResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let result = initialize_existing_application(
        &state,
        &application_id,
        &payload.target_id,
        payload.template_id.as_deref(),
        Some(&actor.id),
        request_id.as_str(),
    )
    .await?;
    Ok(Json(InitializeApplicationConfigsResponse {
        binding_id: result.binding_id,
        created: result.created,
        status: result.status,
        file_count: result.file_count,
    }))
}

#[utoipa::path(
    operation_id = "application_configs_delete_workspace",
    delete,
    path = "/api/v1/applications/{application_id}/config-files",
    params(("application_id" = String, Path), ("X-CSRF-Token" = String, Header)),
    request_body = DeleteApplicationConfigWorkspaceRequest,
    responses((status = 204), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse))
)]
pub(crate) async fn delete_workspace(
    State(state): State<AppState>,
    Path(application_id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    actor: AuthUser,
    crate::http::ApiJson(payload): crate::http::ApiJson<DeleteApplicationConfigWorkspaceRequest>,
) -> ApiResult<StatusCode> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    delete_unused_binding(
        &state,
        &application_id,
        &payload.binding_id,
        Some(&actor.id),
        request_id.as_str(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug)]
struct CloneResult {
    binding_id: String,
    template_id: String,
    status: String,
    created: bool,
    file_count: usize,
}

/// 在创建应用的事务内克隆模板。调用者必须把应用 INSERT 和本函数放在同一事务中。
pub(crate) async fn clone_template_for_application(
    transaction: &mut Transaction<'_, Sqlite>,
    ring: &crate::crypto::MasterKeyRing,
    application_id: &str,
    template_id: &str,
    actor_id: Option<&str>,
    request_id: &str,
) -> ApiResult<()> {
    let (app_type, type_version): (String, String) =
        sqlx::query_as("SELECT app_type,type_version FROM applications WHERE id=?")
            .bind(application_id)
            .fetch_optional(&mut **transaction)
            .await
            .map_err(|_| ApiError::internal(request_id))?
            .ok_or_else(|| ApiError::not_found(request_id))?;
    let template = template_from_id(template_id)
        .map(template_descriptor)
        .ok_or_else(|| ApiError::not_found(request_id))?;
    if app_type != template.id || type_version != template.version {
        return Err(ApiError::validation("应用类型与模板版本不匹配", request_id));
    }
    let result = clone_template_in_transaction(
        transaction,
        ring,
        CloneTemplateOptions {
            application_id,
            template_id,
            target_id: None,
            actor_id,
            source: "template_creation",
            overrides: &HashMap::new(),
            request_id,
        },
    )
    .await?;
    audit::record(
        transaction,
        actor_id,
        "application_config.clone",
        "application_template_binding",
        &result.binding_id,
        request_id,
        json!({
            "application_id": application_id,
            "template_id": template.id,
            "template_version": template.version,
            "template_digest": template.digest,
            "file_count": result.file_count
        }),
    )
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    Ok(())
}

async fn initialize_existing_application(
    state: &AppState,
    application_id: &str,
    target_id: &str,
    requested_template_id: Option<&str>,
    actor_id: Option<&str>,
    request_id: &str,
) -> ApiResult<CloneResult> {
    let ring = state
        .master_key_ring()
        .ok_or_else(|| ApiError::service_not_ready(request_id))?;
    let (target_application_id, execution_mode, image_spec_json): (String, String, String) =
        sqlx::query_as(
            "SELECT application_id,execution_mode,COALESCE(image_spec_json,'') FROM deployment_targets WHERE id=?",
        )
        .bind(target_id)
        .fetch_optional(state.pool())
        .await
        .map_err(|_| ApiError::internal(request_id))?
        .ok_or_else(|| ApiError::not_found(request_id))?;
    if target_application_id != application_id {
        return Err(ApiError::not_found(request_id));
    }
    if execution_mode != "image" || image_spec_json.is_empty() {
        return Err(ApiError::validation(
            "只有 image target 可以初始化配置副本",
            request_id,
        ));
    }
    let image_spec: PlatformImageDeploySpec = serde_json::from_str(&image_spec_json)
        .map_err(|_| ApiError::validation("image spec 无效", request_id))?;
    let template_id = requested_template_id.unwrap_or(match image_spec.template {
        deploy_go_container_template::ImageTemplate::Redis => "redis",
        deploy_go_container_template::ImageTemplate::Valkey => "valkey",
        deploy_go_container_template::ImageTemplate::Postgres => "postgres",
        deploy_go_container_template::ImageTemplate::Etcd => "etcd",
    });
    if template_id != template_id_for_platform(image_spec.template) {
        return Err(ApiError::conflict(
            "application_config_template_conflict",
            "请求模板与 image target 不一致",
            request_id,
        ));
    }
    let template = template_from_id(template_id)
        .map(template_descriptor)
        .ok_or_else(|| ApiError::not_found(request_id))?;
    let allowed_env_files = image_spec
        .env_files
        .iter()
        .map(|name| name.to_ascii_lowercase())
        .collect::<std::collections::HashSet<_>>();
    let mut overrides = HashMap::new();
    let env_rows: Vec<LegacyEnvRow> = sqlx::query_as(
        "SELECT f.id,f.file_name,v.id,v.ciphertext,v.nonce,v.key_version FROM application_env_files f JOIN application_env_versions v ON v.env_file_id=f.id AND v.env_version=f.current_version WHERE f.application_id=? AND f.deleted_at IS NULL",
    )
    .bind(application_id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    for (env_file_id, file_name, version_id, ciphertext, nonce, key_version) in env_rows {
        if !allowed_env_files.contains(&file_name.to_ascii_lowercase()) {
            continue;
        }
        let plaintext = ring
            .decrypt_application_env(
                application_id,
                &env_file_id,
                &version_id,
                &EncryptedSecret {
                    ciphertext,
                    nonce,
                    key_version,
                },
            )
            .map_err(|_| ApiError::internal(request_id))?;
        overrides.insert(file_name.to_ascii_lowercase(), plaintext.to_vec());
    }
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    let result = clone_template_in_transaction(
        &mut transaction,
        ring,
        CloneTemplateOptions {
            application_id,
            template_id,
            target_id: Some(target_id),
            actor_id,
            source: "legacy_initialization",
            overrides: &overrides,
            request_id,
        },
    )
    .await?;
    audit::record(
        &mut transaction,
        actor_id,
        "application_config.initialize",
        "application_template_binding",
        &result.binding_id,
        request_id,
        json!({
            "application_id": application_id,
            "target_id": target_id,
            "template_id": result.template_id,
            "created": result.created,
            "file_count": result.file_count
        }),
    )
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    let _ = template;
    Ok(result)
}

async fn clone_template_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
    ring: &crate::crypto::MasterKeyRing,
    options: CloneTemplateOptions<'_>,
) -> ApiResult<CloneResult> {
    let CloneTemplateOptions {
        application_id,
        template_id,
        target_id,
        actor_id,
        source,
        overrides,
        request_id,
    } = options;
    let template = template_from_id(template_id)
        .map(template_descriptor)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "application_template_not_found",
                "应用模板不存在",
                request_id,
            )
        })?;
    let existing: Option<(String, String, i64, String)> = sqlx::query_as(
        "SELECT id,status,enabled,template_id FROM application_template_bindings WHERE application_id=? AND template_id=? AND template_version=?",
    )
    .bind(application_id)
    .bind(&template.id)
    .bind(&template.version)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    if let Some((id, status, _enabled, _)) = existing {
        if status == "deleted" {
            return Err(ApiError::conflict(
                "application_config_binding_conflict",
                "该模板版本的配置副本已经回退删除",
                request_id,
            ));
        }
        let file_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM application_config_files WHERE binding_id=? AND deleted_at IS NULL",
        )
        .bind(&id)
        .fetch_one(&mut **transaction)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
        return Ok(CloneResult {
            binding_id: id,
            template_id: template.id,
            status,
            created: false,
            file_count: file_count as usize,
        });
    }
    let conflicting: Option<(String, String)> = sqlx::query_as(
        "SELECT template_id,template_version FROM application_template_bindings WHERE application_id=? AND status<>'deleted' LIMIT 1",
    )
    .bind(application_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    if let Some((existing_template, existing_version)) = conflicting {
        return Err(ApiError::conflict(
            "application_config_binding_conflict",
            "应用已经绑定其他模板版本",
            request_id,
        )
        .with_details(json!({"existing_template_id":existing_template,"existing_template_version":existing_version,"requested_template_id":template.id,"requested_template_version":template.version})));
    }
    let binding_id = format!("cfgb_{}", Ulid::new());
    let binding_status = if source == "legacy_initialization" {
        "draft"
    } else {
        "active"
    };
    let enabled = i64::from(source == "template_creation");
    sqlx::query("INSERT INTO application_template_bindings(id,application_id,target_id,template_id,template_version,template_digest,source,status,enabled,created_by) VALUES(?,?,?,?,?,?,?,?,?,?)")
        .bind(&binding_id)
        .bind(application_id)
        .bind(target_id)
        .bind(&template.id)
        .bind(&template.version)
        .bind(&template.digest)
        .bind(source)
        .bind(binding_status)
        .bind(enabled)
        .bind(actor_id)
        .execute(&mut **transaction)
        .await
        .map_err(|error| map_unique(error, request_id))?;

    let mut file_count = 0;
    for file in template
        .files
        .iter()
        .filter(|file| matches!(file.role, TemplateFileRole::Configuration))
    {
        let Some(deploy_path) = file.deploy_path.as_deref() else {
            continue;
        };
        let file_id = format!("cfgf_{}", Ulid::new());
        let version_id = format!("cfgv_{}", Ulid::new());
        let content = overrides
            .get(&deploy_path.to_ascii_lowercase())
            .or_else(|| overrides.get(deploy_path))
            .map(|content| content.as_slice())
            .unwrap_or(file.content.as_bytes());
        let digest = hex_digest(content);
        let status = config_status(file.sensitive, content);
        let encrypted = ring
            .encrypt_application_config(application_id, &file_id, &version_id, content)
            .map_err(|_| ApiError::internal(request_id))?;
        sqlx::query("INSERT INTO application_config_files(id,binding_id,application_id,path,deploy_path,label,format,language,role,delivery,sensitive,editable,description,recommended_changes,template_source_digest,current_version,current_digest,status,version) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,1)")
            .bind(&file_id)
            .bind(&binding_id)
            .bind(application_id)
            .bind(&file.path)
            .bind(deploy_path)
            .bind(&file.label)
            .bind(enum_string(file.format))
            .bind(&file.language)
            .bind(enum_string(file.role))
            .bind(enum_string(file.delivery))
            .bind(i64::from(file.sensitive))
            .bind(i64::from(file.editable))
            .bind(&file.description)
            .bind(&file.recommended_changes)
            .bind(&file.digest)
            .bind(1_i64)
            .bind(&digest)
            .bind(status)
            .execute(&mut **transaction)
            .await
            .map_err(|_| ApiError::internal(request_id))?;
        insert_version(
            transaction,
            &ConfigFileRow {
                id: file_id,
                binding_id: binding_id.clone(),
                application_id: application_id.to_owned(),
                path: file.path.clone(),
                deploy_path: Some(deploy_path.to_owned()),
                label: file.label.clone(),
                format: enum_string(file.format),
                language: file.language.clone(),
                role: enum_string(file.role),
                delivery: enum_string(file.delivery),
                sensitive: i64::from(file.sensitive),
                editable: i64::from(file.editable),
                description: file.description.clone(),
                recommended_changes: file.recommended_changes.clone(),
                template_source_digest: file.digest.clone(),
                current_version: 1,
                current_digest: digest.clone(),
                status: status.to_owned(),
                deleted_at: None,
                updated_at: Utc::now().to_rfc3339(),
                version: 1,
            },
            NewConfigVersion {
                config_version: 1,
                version_id: &version_id,
                encrypted,
                digest: &digest,
                source: if source == "legacy_initialization" {
                    "legacy_initialization"
                } else {
                    "template"
                },
                source_version_id: None,
                source_template_digest: Some(&file.digest),
                created_by: actor_id,
                created_at: &Utc::now().to_rfc3339(),
            },
        )
        .await
        .map_err(|_| ApiError::internal(request_id))?;
        if source == "template_creation" && enum_string(file.delivery) == "env_lease" {
            sync_env_lease_to_legacy(
                transaction,
                ring,
                application_id,
                deploy_path,
                content,
                actor_id,
                request_id,
            )
            .await?;
        }
        file_count += 1;
    }
    Ok(CloneResult {
        binding_id,
        template_id: template.id,
        status: binding_status.to_owned(),
        created: true,
        file_count,
    })
}

async fn delete_unused_binding(
    state: &AppState,
    application_id: &str,
    binding_id: &str,
    actor_id: Option<&str>,
    request_id: &str,
) -> ApiResult<()> {
    let enabled: Option<(String, i64, String)> = sqlx::query_as(
        "SELECT status,enabled,application_id FROM application_template_bindings WHERE id=?",
    )
    .bind(binding_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let Some((status, enabled, owner)) = enabled else {
        return Err(ApiError::not_found(request_id));
    };
    if owner != application_id {
        return Err(ApiError::not_found(request_id));
    }
    if status != "draft" || enabled != 0 {
        return Err(ApiError::conflict(
            "application_config_binding_in_use",
            "已启用的配置副本不能回退删除",
            request_id,
        ));
    }
    let deployment_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM deployments WHERE application_id=?")
            .bind(application_id)
            .fetch_one(state.pool())
            .await
            .map_err(|_| ApiError::internal(request_id))?;
    if deployment_count > 0 {
        return Err(ApiError::conflict(
            "application_config_binding_in_use",
            "已有部署快照引用该应用，不能回退删除",
            request_id,
        ));
    }
    let now = Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    sqlx::query("UPDATE application_template_bindings SET status='deleted',updated_at=?,version=version+1 WHERE id=? AND status='draft' AND enabled=0")
        .bind(&now)
        .bind(binding_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    sqlx::query("UPDATE application_config_files SET deleted_at=?,updated_at=?,version=version+1 WHERE binding_id=? AND deleted_at IS NULL")
        .bind(&now)
        .bind(&now)
        .bind(binding_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    audit::record(
        &mut transaction,
        actor_id,
        "application_config.delete_workspace",
        "application_template_binding",
        binding_id,
        request_id,
        json!({"application_id":application_id}),
    )
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    Ok(())
}

async fn load_file(
    pool: &sqlx::SqlitePool,
    id: &str,
    request_id: &str,
) -> ApiResult<ConfigFileRow> {
    sqlx::query_as::<_, ConfigFileRow>(&config_file_select("WHERE f.id=? AND f.deleted_at IS NULL"))
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?
        .ok_or_else(|| ApiError::not_found(request_id))
}

async fn load_version(
    pool: &sqlx::SqlitePool,
    file_id: &str,
    version: i64,
    request_id: &str,
) -> ApiResult<ConfigVersionRow> {
    sqlx::query_as::<_, ConfigVersionRow>(
        "SELECT id,application_config_file_id,application_id,config_version,algorithm,ciphertext,nonce,key_version,digest,source,source_version_id,source_template_digest,created_by,created_at FROM application_config_versions WHERE application_config_file_id=? AND config_version=?",
    )
    .bind(file_id)
    .bind(version)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?
    .ok_or_else(|| ApiError::not_found(request_id))
}

async fn save_content(
    state: &AppState,
    current: &ConfigFileRow,
    options: SaveContentOptions<'_>,
) -> ApiResult<ApplicationConfigFileResponse> {
    let SaveContentOptions {
        content,
        expected_version,
        source,
        consume_grant,
        actor_id,
        request_id,
    } = options;
    validate_configuration(state, current, content, request_id).await?;
    let ring = state
        .master_key_ring()
        .ok_or_else(|| ApiError::service_not_ready(request_id))?;
    let version_id = format!("cfgv_{}", Ulid::new());
    let encrypted = ring
        .encrypt_application_config(
            &current.application_id,
            &current.id,
            &version_id,
            content.as_bytes(),
        )
        .map_err(|_| ApiError::internal(request_id))?;
    let digest = hex_digest(content.as_bytes());
    let status = config_status(current.sensitive != 0, content.as_bytes());
    let now = Utc::now().to_rfc3339();
    let mut transaction = state
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    let result = sqlx::query("UPDATE application_config_files SET current_version=?,current_digest=?,status=?,updated_at=?,version=version+1 WHERE id=? AND application_id=? AND deleted_at IS NULL AND editable=1 AND version=?")
        .bind(current.current_version + 1)
        .bind(&digest)
        .bind(status)
        .bind(&now)
        .bind(&current.id)
        .bind(&current.application_id)
        .bind(expected_version)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    if result.rows_affected() != 1 {
        return Err(version_conflict(request_id));
    }
    insert_version(
        &mut transaction,
        current,
        NewConfigVersion {
            config_version: current.current_version + 1,
            version_id: &version_id,
            encrypted,
            digest: &digest,
            source,
            source_version_id: None,
            source_template_digest: None,
            created_by: actor_id,
            created_at: &now,
        },
    )
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    if let Some(token) = consume_grant {
        let revoked = sqlx::query(
            "UPDATE application_env_reveal_grants SET revoked_at=? WHERE token_hash=? AND user_id=? AND application_id=? AND action_scope='read_write' AND revoked_at IS NULL AND expires_at>?",
        )
        .bind(&now)
        .bind(token_hash(token))
        .bind(actor_id.unwrap_or_default())
        .bind(&current.application_id)
        .bind(&now)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
        if revoked.rows_affected() != 1 {
            return Err(ApiError::forbidden(request_id));
        }
    }
    if current.delivery == "env_lease"
        && let Some(deploy_path) = current.deploy_path.as_deref()
    {
        sync_env_lease_to_legacy(
            &mut transaction,
            ring,
            &current.application_id,
            deploy_path,
            content.as_bytes(),
            actor_id,
            request_id,
        )
        .await?;
    }
    audit::record(
        &mut transaction,
        actor_id,
        "application_config.update",
        "application_config_file",
        &current.id,
        request_id,
        json!({
            "application_id": current.application_id,
            "path": current.path,
            "old_version": current.current_version,
            "new_version": current.current_version + 1,
            "source": source,
            "digest": (current.sensitive == 0).then_some(&digest)
        }),
    )
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    let file = load_file(state.pool(), &current.id, request_id).await?;
    let content = (current.sensitive == 0).then_some(content.to_owned());
    Ok(file_response_with_content(file, content, false))
}

async fn validate_configuration(
    state: &AppState,
    file: &ConfigFileRow,
    content: &str,
    request_id: &str,
) -> ApiResult<()> {
    let diagnostics = configuration_diagnostics(state, file, content, request_id).await?;
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(invalid_config(request_id, diagnostics))
    }
}

fn invalid_config(request_id: &str, diagnostics: Vec<ConfigDiagnostic>) -> ApiError {
    ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "application_config_invalid",
        "应用配置校验失败",
        request_id,
    )
    .with_details(json!({"diagnostics":diagnostics}))
}

async fn configuration_diagnostics(
    state: &AppState,
    file: &ConfigFileRow,
    content: &str,
    request_id: &str,
) -> ApiResult<Vec<ConfigDiagnostic>> {
    let mut diagnostics = validate_format(&file.format, &file.path, content);
    if file.path == "compose.yaml" && file.format == "yaml" {
        let allowed_paths = sqlx::query_scalar::<_, String>(
            "SELECT deploy_path FROM application_config_files WHERE binding_id=? AND deploy_path IS NOT NULL AND deleted_at IS NULL",
        )
        .bind(&file.binding_id)
        .fetch_all(state.pool())
        .await
        .map_err(|_| ApiError::internal(request_id))?
        .into_iter()
        .collect::<HashSet<_>>();
        let template_content = template_content_for_file(state, file, request_id).await?;
        diagnostics.extend(compose_policy_diagnostics(
            &file.path,
            content,
            &template_content,
            &allowed_paths,
        ));
    }
    Ok(diagnostics)
}

fn validate_format(format: &str, path: &str, content: &str) -> Vec<ConfigDiagnostic> {
    let mut diagnostics = Vec::new();
    if content.len() > MAX_CONFIG_BYTES {
        diagnostics.push(diagnostic(
            path,
            1,
            1,
            "content_too_large",
            "配置正文超过 1 MiB 上限",
        ));
        return diagnostics;
    }
    if content.contains('\0')
        || content.chars().any(|character| {
            character.is_control() && character != '\n' && character != '\t' && character != '\r'
        })
    {
        diagnostics.push(diagnostic(
            path,
            1,
            1,
            "control_character",
            "配置正文包含不允许的控制字符",
        ));
        return diagnostics;
    }
    match format {
        "dotenv" => {
            if let Err(errors) = crate::application_envs::dotenv::validate(content) {
                diagnostics.extend(
                    errors
                        .into_iter()
                        .map(|error| diagnostic(path, error.line, 1, error.code, error.message)),
                );
            }
        }
        "yaml" => {
            if content.lines().any(|line| {
                let trimmed = line.trim_start();
                !trimmed.starts_with('#') && contains_yaml_anchor_or_alias(trimmed)
            }) {
                diagnostics.push(diagnostic(
                    path,
                    1,
                    1,
                    "yaml_alias_not_allowed",
                    "YAML 不允许使用 alias 或 merge key",
                ));
            }
            if let Err(error) = serde_yaml::from_str::<serde_yaml::Value>(content) {
                let (line, column) = error
                    .location()
                    .map(|location| (location.line(), location.column()))
                    .unwrap_or((1, 1));
                diagnostics.push(diagnostic(
                    path,
                    line,
                    column,
                    "yaml_parse_error",
                    "YAML 格式无效",
                ));
            }
        }
        "json" => {
            if let Err(error) = serde_json::from_str::<Value>(content) {
                diagnostics.push(diagnostic(
                    path,
                    error.line(),
                    error.column(),
                    "json_parse_error",
                    "JSON 格式无效",
                ));
            }
        }
        "ini" => diagnostics.extend(validate_ini(path, content)),
        "shell" | "makefile" | "markdown" => {}
        _ => diagnostics.push(diagnostic(
            path,
            1,
            1,
            "unsupported_format",
            "配置文件格式不受支持",
        )),
    }
    diagnostics
}

fn validate_ini(path: &str, content: &str) -> Vec<ConfigDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut keys = HashSet::new();
    for (index, line) in content.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            continue;
        }
        let Some((key, _)) = trimmed.split_once('=') else {
            diagnostics.push(diagnostic(
                path,
                line_number,
                1,
                "ini_assignment_required",
                "INI 行必须使用 key=value 语法",
            ));
            continue;
        };
        if key.trim().is_empty() || !keys.insert(key.trim().to_owned()) {
            diagnostics.push(diagnostic(
                path,
                line_number,
                1,
                "ini_duplicate_or_empty_key",
                "INI 键为空或重复",
            ));
        }
    }
    diagnostics
}

fn contains_yaml_anchor_or_alias(line: &str) -> bool {
    if line.contains("<<:") {
        return true;
    }
    line.bytes().enumerate().any(|(index, byte)| {
        if byte != b'&' && byte != b'*' {
            return false;
        }
        index == 0
            || line.as_bytes()[index - 1].is_ascii_whitespace()
            || matches!(line.as_bytes()[index - 1], b':' | b'[' | b',')
    })
}

fn diagnostic(
    path: &str,
    line: usize,
    column: usize,
    code: &str,
    message: &str,
) -> ConfigDiagnostic {
    ConfigDiagnostic {
        path: path.to_owned(),
        line,
        column,
        code: code.to_owned(),
        message: message.to_owned(),
    }
}

fn compose_policy_diagnostics(
    path: &str,
    content: &str,
    template_content: &[u8],
    allowed_paths: &HashSet<String>,
) -> Vec<ConfigDiagnostic> {
    let mut diagnostics = Vec::new();
    let Ok(value) = serde_yaml::from_str::<YamlValue>(content) else {
        return diagnostics;
    };
    let Ok(template) = serde_yaml::from_slice::<YamlValue>(template_content) else {
        return diagnostics;
    };
    let Some(root) = value.as_mapping() else {
        diagnostics.push(diagnostic(
            path,
            1,
            1,
            "compose_root_invalid",
            "Compose 根节点必须是对象",
        ));
        return diagnostics;
    };
    for key in ["include", "extends"] {
        if yaml_get(root, key).is_some() {
            diagnostics.push(diagnostic(
                path,
                1,
                1,
                "compose_external_reference",
                "Compose 不允许 include 或 extends",
            ));
        }
    }
    if yaml_get(root, "build").is_some() {
        diagnostics.push(diagnostic(
            path,
            1,
            1,
            "compose_build_context_forbidden",
            "Compose 不允许 build 或 build.context",
        ));
    }
    let Some(services) = yaml_get(root, "services").and_then(YamlValue::as_mapping) else {
        diagnostics.push(diagnostic(
            path,
            1,
            1,
            "compose_services_required",
            "Compose 必须声明 services",
        ));
        return diagnostics;
    };
    let template_services = template
        .as_mapping()
        .and_then(|root| yaml_get(root, "services"))
        .and_then(YamlValue::as_mapping);
    let template_volume_names = template
        .as_mapping()
        .and_then(|root| yaml_get(root, "volumes"))
        .and_then(YamlValue::as_mapping)
        .map(|volumes| {
            volumes
                .keys()
                .filter_map(YamlValue::as_str)
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    if let Some(template_services) = template_services {
        for service_name in template_services.keys().filter_map(YamlValue::as_str) {
            if !services.contains_key(YamlValue::String(service_name.to_owned())) {
                diagnostics.push(diagnostic(
                    path,
                    1,
                    1,
                    "compose_service_missing",
                    "Compose 不能删除模板声明的 service",
                ));
            }
        }
    }
    for (service_name, service) in services {
        let Some(service_name) = service_name.as_str() else {
            diagnostics.push(diagnostic(
                path,
                1,
                1,
                "compose_service_invalid",
                "Compose service 名称必须是字符串",
            ));
            continue;
        };
        let Some(service) = service.as_mapping() else {
            diagnostics.push(diagnostic(
                path,
                1,
                1,
                "compose_service_invalid",
                "Compose service 必须是对象",
            ));
            continue;
        };
        if template_services.is_some_and(|templates| {
            !templates.contains_key(YamlValue::String(service_name.to_owned()))
        }) {
            diagnostics.push(diagnostic(
                path,
                1,
                1,
                "compose_service_not_declared",
                "Compose 只能声明模板已有 service",
            ));
        }
        for key in [
            "privileged",
            "devices",
            "network_mode",
            "pid",
            "ipc",
            "userns_mode",
            "cgroup",
            "uts",
            "cap_add",
            "volumes",
        ] {
            if yaml_get(service, key).is_some_and(contains_compose_interpolation) {
                diagnostics.push(diagnostic(
                    path,
                    1,
                    1,
                    "compose_interpolation_forbidden",
                    "Compose 安全敏感字段不允许使用环境变量插值",
                ));
            }
        }
        for key in ["include", "extends"] {
            if yaml_get(service, key).is_some() {
                diagnostics.push(diagnostic(
                    path,
                    1,
                    1,
                    "compose_external_reference",
                    "Compose 不允许 include 或 extends",
                ));
            }
        }
        if yaml_get(service, "devices").is_some() {
            diagnostics.push(diagnostic(
                path,
                1,
                1,
                "compose_devices_forbidden",
                "Compose 不允许设备映射",
            ));
        }
        if yaml_get(service, "privileged")
            .is_some_and(|value| !matches!(value, YamlValue::Bool(false)))
        {
            diagnostics.push(diagnostic(
                path,
                1,
                1,
                "compose_privileged",
                "Compose 不允许 privileged",
            ));
        }
        for key in ["network_mode", "pid", "ipc", "userns_mode", "cgroup", "uts"] {
            if yaml_get(service, key).and_then(YamlValue::as_str) == Some("host") {
                diagnostics.push(diagnostic(
                    path,
                    1,
                    1,
                    "compose_host_namespace",
                    "Compose 不允许使用宿主命名空间",
                ));
            }
        }
        if let Some(capabilities) = yaml_get(service, "cap_add").and_then(YamlValue::as_sequence)
            && capabilities
                .iter()
                .filter_map(YamlValue::as_str)
                .any(|cap| {
                    matches!(
                        cap.to_ascii_uppercase().as_str(),
                        "ALL" | "SYS_ADMIN" | "NET_ADMIN" | "SYS_PTRACE" | "DAC_READ_SEARCH"
                    )
                })
        {
            diagnostics.push(diagnostic(
                path,
                1,
                1,
                "compose_dangerous_capability",
                "Compose 包含危险 capability",
            ));
        }
        if yaml_get(service, "build").is_some() {
            diagnostics.push(diagnostic(
                path,
                1,
                1,
                "compose_build_context_forbidden",
                "Compose 不允许 build 或 build.context",
            ));
        }
        for key in ["env_file", "configs", "secrets"] {
            if let Some(value) = yaml_get(service, key) {
                if (key == "configs" || key == "secrets") && contains_external_reference(value) {
                    diagnostics.push(diagnostic(
                        path,
                        1,
                        1,
                        "compose_external_reference",
                        "Compose 不允许引用外部 config 或 secret",
                    ));
                }
                for reference in path_references(value) {
                    if !safe_declared_path(&reference, allowed_paths) {
                        diagnostics.push(diagnostic(
                            path,
                            1,
                            1,
                            "compose_path_reference_forbidden",
                            "Compose 路径引用必须是模板声明的相对文件",
                        ));
                    }
                }
            }
        }
        if let Some(volumes) = yaml_get(service, "volumes").and_then(YamlValue::as_sequence) {
            for volume in volumes {
                let Some(volume) = volume.as_str() else {
                    diagnostics.push(diagnostic(
                        path,
                        1,
                        1,
                        "compose_bind_mount_forbidden",
                        "Compose volume 必须使用受控字符串格式",
                    ));
                    continue;
                };
                let source = volume.split(':').next().unwrap_or_default();
                if source.contains("docker.sock") {
                    diagnostics.push(diagnostic(
                        path,
                        1,
                        1,
                        "compose_docker_socket_forbidden",
                        "Compose 不允许挂载 Docker socket",
                    ));
                } else if !safe_volume_source(source, allowed_paths, &template_volume_names) {
                    diagnostics.push(diagnostic(
                        path,
                        1,
                        1,
                        "compose_bind_mount_forbidden",
                        "Compose bind mount 必须位于模板声明的 artifact 根内",
                    ));
                }
            }
        }
        for key in ["command", "entrypoint"] {
            if let Some(value) = yaml_get(service, key) {
                let allowed = template_services
                    .and_then(|services| services.get(YamlValue::String(service_name.to_owned())))
                    .and_then(YamlValue::as_mapping)
                    .and_then(|template_service| yaml_get(template_service, key));
                if allowed != Some(value) {
                    diagnostics.push(diagnostic(
                        path,
                        1,
                        1,
                        "compose_executable_override_forbidden",
                        "Compose 不允许覆盖模板执行入口",
                    ));
                }
            }
        }
    }
    if let Some(volumes) = yaml_get(root, "volumes").and_then(YamlValue::as_mapping) {
        for (name, definition) in volumes {
            if !template_volume_names.contains(name.as_str().unwrap_or_default()) {
                diagnostics.push(diagnostic(
                    path,
                    1,
                    1,
                    "compose_volume_not_declared",
                    "Compose 只能声明模板已有数据卷",
                ));
            }
            if contains_forbidden_volume_option(definition) {
                diagnostics.push(diagnostic(
                    path,
                    1,
                    1,
                    "compose_volume_driver_forbidden",
                    "Compose 数据卷不允许覆盖宿主驱动或设备参数",
                ));
            }
        }
    }
    for key in ["configs", "secrets"] {
        if yaml_get(root, key).is_some_and(contains_external_reference) {
            diagnostics.push(diagnostic(
                path,
                1,
                1,
                "compose_external_reference",
                "Compose 不允许声明外部 config 或 secret",
            ));
        }
    }
    diagnostics
}

fn yaml_get<'a>(map: &'a serde_yaml::Mapping, key: &str) -> Option<&'a YamlValue> {
    map.get(YamlValue::String(key.to_owned()))
}

fn contains_compose_interpolation(value: &YamlValue) -> bool {
    match value {
        YamlValue::String(value) => value.contains('$'),
        YamlValue::Sequence(values) => values.iter().any(contains_compose_interpolation),
        YamlValue::Mapping(map) => map.values().any(contains_compose_interpolation),
        _ => false,
    }
}

fn contains_external_reference(value: &YamlValue) -> bool {
    match value {
        YamlValue::Mapping(map) => map.iter().any(|(key, value)| {
            (key.as_str() == Some("external") && truthy(Some(value)))
                || contains_external_reference(value)
        }),
        YamlValue::Sequence(values) => values.iter().any(contains_external_reference),
        _ => false,
    }
}

fn truthy(value: Option<&YamlValue>) -> bool {
    value.is_some_and(|value| match value {
        YamlValue::Bool(value) => *value,
        YamlValue::String(value) => value.eq_ignore_ascii_case("true"),
        _ => false,
    })
}

fn path_references(value: &YamlValue) -> Vec<String> {
    match value {
        YamlValue::String(value) => vec![value.clone()],
        YamlValue::Sequence(values) => values.iter().flat_map(path_references).collect(),
        YamlValue::Mapping(map) => ["file", "source"]
            .into_iter()
            .filter_map(|key| map.get(YamlValue::String(key.to_owned())))
            .flat_map(path_references)
            .collect(),
        _ => Vec::new(),
    }
}

fn safe_declared_path(reference: &str, allowed_paths: &HashSet<String>) -> bool {
    let path = reference.strip_prefix("./").unwrap_or(reference);
    !path.is_empty()
        && !path.starts_with('/')
        && !path.split('/').any(|part| part == "..")
        && allowed_paths.contains(path)
}

fn safe_volume_source(
    source: &str,
    allowed_paths: &HashSet<String>,
    allowed_volume_names: &HashSet<&str>,
) -> bool {
    if source.is_empty() {
        return true;
    }
    if !source.contains('/') && !source.starts_with('.') {
        return allowed_volume_names.contains(source);
    }
    safe_declared_path(source, allowed_paths)
}

fn contains_forbidden_volume_option(value: &YamlValue) -> bool {
    match value {
        YamlValue::Mapping(map) => map.iter().any(|(key, value)| {
            matches!(
                key.as_str(),
                Some("driver")
                    | Some("driver_opts")
                    | Some("device")
                    | Some("name")
                    | Some("external")
                    | Some("type")
                    | Some("o")
            ) || contains_forbidden_volume_option(value)
        }),
        YamlValue::Sequence(values) => values.iter().any(contains_forbidden_volume_option),
        _ => false,
    }
}

async fn template_content_for_file(
    state: &AppState,
    file: &ConfigFileRow,
    request_id: &str,
) -> ApiResult<Zeroizing<Vec<u8>>> {
    let template_id: String = sqlx::query_scalar(
        "SELECT template_id FROM application_template_bindings WHERE id=? AND application_id=?",
    )
    .bind(&file.binding_id)
    .bind(&file.application_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id))?
    .ok_or_else(|| ApiError::not_found(request_id))?;
    let template = template_from_id(&template_id)
        .map(template_descriptor)
        .ok_or_else(|| ApiError::not_found(request_id))?;
    let source = template
        .files
        .into_iter()
        .find(|source| source.deploy_path == file.deploy_path)
        .ok_or_else(|| ApiError::not_found(request_id))?;
    Ok(Zeroizing::new(source.content.into_bytes()))
}

fn apply_controlled_patch(
    file: &ConfigFileRow,
    content: &str,
    key: &str,
    value: &str,
) -> Result<String, Vec<ConfigDiagnostic>> {
    if file.format != "dotenv" {
        return Err(vec![diagnostic(
            &file.path,
            1,
            1,
            "controlled_patch_unsupported_format",
            "受控字段修改目前只支持 dotenv 文件",
        )]);
    }
    if value
        .chars()
        .any(|character| character == '\n' || character == '\r' || character == '\0')
    {
        return Err(vec![diagnostic(
            &file.path,
            1,
            1,
            "controlled_patch_invalid_value",
            "受控字段值不能包含换行或 NUL 字符",
        )]);
    }
    let valid_key = key.bytes().enumerate().all(|(index, byte)| {
        (index == 0 && (byte.is_ascii_alphabetic() || byte == b'_'))
            || (index > 0 && (byte.is_ascii_alphanumeric() || byte == b'_'))
    });
    if !valid_key {
        return Err(vec![diagnostic(
            &file.path,
            1,
            1,
            "controlled_patch_invalid_key",
            "受控字段名格式无效",
        )]);
    }
    let mut found = false;
    let mut output = String::with_capacity(content.len() + value.len());
    for (index, line) in content.split_inclusive('\n').enumerate() {
        let line_without_newline = line.strip_suffix('\n').unwrap_or(line);
        let newline = if line.ends_with('\n') { "\n" } else { "" };
        let leading = line_without_newline.len() - line_without_newline.trim_start().len();
        let trimmed = line_without_newline.trim_start();
        if !trimmed.starts_with('#')
            && let Some((candidate, rest)) = trimmed.split_once('=')
            && candidate.trim() == key
        {
            if found {
                return Err(vec![diagnostic(
                    &file.path,
                    index + 1,
                    leading + 1,
                    "controlled_patch_duplicate_key",
                    "受控字段重复",
                )]);
            }
            found = true;
            let prefix = &line_without_newline[..leading + candidate.len() + 1];
            output.push_str(prefix);
            output.push_str(value);
            if rest.ends_with(' ') || rest.ends_with('\t') {
                output.push_str(&rest[rest.trim_end().len()..]);
            }
            output.push_str(newline);
            continue;
        }
        output.push_str(line);
    }
    if !found {
        return Err(vec![diagnostic(
            &file.path,
            1,
            1,
            "controlled_patch_key_not_found",
            "受控字段不存在",
        )]);
    }
    Ok(output)
}

fn redact_sensitive_content(content: &str) -> Vec<u8> {
    content
        .lines()
        .map(|line| {
            if line.trim().is_empty() {
                ""
            } else {
                "<redacted>"
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes()
}

async fn verify_config_grant(
    headers: &HeaderMap,
    state: &AppState,
    actor: &AuthUser,
    application_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let mut normalized = headers.clone();
    if normalized.get("x-env-reveal-grant").is_none()
        && let Some(token) = headers.get("x-config-reveal-grant")
    {
        normalized.insert("x-env-reveal-grant", token.clone());
    }
    crate::application_envs::verify_grant(
        state.pool(),
        &normalized,
        actor,
        application_id,
        "read_write",
        request_id,
    )
    .await
}

fn config_grant_token(headers: &HeaderMap, request_id: &str) -> ApiResult<String> {
    headers
        .get("x-env-reveal-grant")
        .or_else(|| headers.get("x-config-reveal-grant"))
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
        .ok_or_else(|| ApiError::forbidden(request_id))
}

async fn enforce_reauth_rate_limit(
    pool: &sqlx::SqlitePool,
    session_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let blocked: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM application_env_reauth_attempts WHERE session_id=? AND blocked_until>?)",
    )
    .bind(session_id)
    .bind(Utc::now().to_rfc3339())
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
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
    sqlx::query("INSERT INTO application_env_reauth_attempts (session_id,failed_count,window_started_at,blocked_until) VALUES (?,1,?,NULL) ON CONFLICT(session_id) DO UPDATE SET failed_count=CASE WHEN window_started_at<? THEN 1 ELSE failed_count+1 END,window_started_at=CASE WHEN window_started_at<? THEN excluded.window_started_at ELSE window_started_at END,blocked_until=CASE WHEN (CASE WHEN window_started_at<? THEN 1 ELSE failed_count+1 END)>=? THEN ? ELSE NULL END")
        .bind(session_id)
        .bind(now.to_rfc3339())
        .bind(&window)
        .bind(&window)
        .bind(&window)
        .bind(MAX_REAUTH_FAILURES)
        .bind(block)
        .execute(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    Ok(())
}

fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
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

async fn read_version_content(
    state: &AppState,
    file: &ConfigFileRow,
    request_id: &str,
) -> ApiResult<String> {
    let version = sqlx::query_as::<_, ConfigVersionRow>(
        "SELECT id,application_config_file_id,application_id,config_version,algorithm,ciphertext,nonce,key_version,digest,source,source_version_id,source_template_digest,created_by,created_at FROM application_config_versions WHERE application_config_file_id=? AND config_version=?",
    )
    .bind(&file.id)
    .bind(file.current_version)
    .fetch_one(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let plaintext = decrypt_version(state, &version, request_id)?;
    String::from_utf8(plaintext.to_vec()).map_err(|_| ApiError::internal(request_id))
}

fn decrypt_version(
    state: &AppState,
    row: &ConfigVersionRow,
    request_id: &str,
) -> ApiResult<Zeroizing<Vec<u8>>> {
    state
        .master_key_ring()
        .ok_or_else(|| ApiError::service_not_ready(request_id))?
        .decrypt_application_config(
            &row.application_id,
            &row.application_config_file_id,
            &row.id,
            &EncryptedSecret {
                ciphertext: row.ciphertext.clone(),
                nonce: row.nonce.clone(),
                key_version: row.key_version,
            },
        )
        .map_err(|_| ApiError::internal(request_id))
}

async fn insert_version(
    transaction: &mut Transaction<'_, Sqlite>,
    file: &ConfigFileRow,
    version: NewConfigVersion<'_>,
) -> sqlx::Result<()> {
    let NewConfigVersion {
        config_version,
        version_id,
        encrypted,
        digest,
        source,
        source_version_id,
        source_template_digest,
        created_by,
        created_at,
    } = version;
    sqlx::query("INSERT INTO application_config_versions(id,application_config_file_id,application_id,config_version,algorithm,ciphertext,nonce,key_version,digest,source,source_version_id,source_template_digest,created_by,created_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(version_id)
        .bind(&file.id)
        .bind(&file.application_id)
        .bind(config_version)
        .bind(APPLICATION_CONFIG_ALGORITHM)
        .bind(encrypted.ciphertext)
        .bind(encrypted.nonce)
        .bind(encrypted.key_version)
        .bind(digest)
        .bind(source)
        .bind(source_version_id)
        .bind(source_template_digest)
        .bind(created_by)
        .bind(created_at)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

fn config_file_select(suffix: &str) -> String {
    format!(
        "SELECT f.id,f.binding_id,f.application_id,f.path,f.deploy_path,f.label,f.format,f.language,f.role,f.delivery,f.sensitive,f.editable,f.description,f.recommended_changes,f.template_source_digest,f.current_version,f.current_digest,f.status,f.deleted_at,f.updated_at,f.version FROM application_config_files f {suffix}"
    )
}

fn file_response(row: ConfigFileRow) -> ApplicationConfigFileResponse {
    file_response_with_content(row, None, false)
}

fn file_response_with_content(
    row: ConfigFileRow,
    content: Option<String>,
    expose_sensitive: bool,
) -> ApplicationConfigFileResponse {
    let sensitive = row.sensitive != 0;
    ApplicationConfigFileResponse {
        id: row.id,
        binding_id: row.binding_id,
        application_id: row.application_id,
        path: row.path,
        deploy_path: row.deploy_path,
        label: row.label,
        format: row.format,
        language: row.language,
        role: row.role,
        delivery: row.delivery,
        sensitive,
        editable: row.editable != 0,
        description: row.description,
        recommended_changes: row.recommended_changes,
        template_source_digest: (!sensitive).then_some(row.template_source_digest),
        current_version: row.current_version,
        current_digest: (!sensitive).then_some(row.current_digest),
        status: row.status,
        deleted_at: row.deleted_at,
        updated_at: row.updated_at,
        version: row.version,
        content: ((!sensitive) || expose_sensitive)
            .then_some(content)
            .flatten(),
    }
}

fn version_response(row: ConfigVersionRow, sensitive: bool) -> ApplicationConfigVersionResponse {
    let _ = (
        &row.algorithm,
        row.key_version,
        row.ciphertext.len(),
        row.nonce.len(),
    );
    ApplicationConfigVersionResponse {
        id: row.id,
        application_config_file_id: row.application_config_file_id,
        config_version: row.config_version,
        digest: (!sensitive).then_some(row.digest),
        source: row.source,
        source_version_id: row.source_version_id,
        source_template_digest: (!sensitive).then_some(row.source_template_digest).flatten(),
        created_by: row.created_by,
        created_at: row.created_at,
    }
}

fn config_status(sensitive: bool, content: &[u8]) -> &'static str {
    if sensitive && contains_secret_placeholder(content) {
        "incomplete"
    } else {
        "ready"
    }
}

fn contains_secret_placeholder(content: &[u8]) -> bool {
    let lower = String::from_utf8_lossy(content).to_ascii_lowercase();
    ["change-me", "replace-me", "<secret>", "<password>"]
        .iter()
        .any(|needle| lower.contains(needle))
}

fn enum_string<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .expect("模板枚举可序列化")
        .as_str()
        .expect("模板枚举为字符串")
        .to_owned()
}

fn template_id_for_platform(template: deploy_go_container_template::ImageTemplate) -> &'static str {
    match template {
        deploy_go_container_template::ImageTemplate::Redis => "redis",
        deploy_go_container_template::ImageTemplate::Valkey => "valkey",
        deploy_go_container_template::ImageTemplate::Postgres => "postgres",
        deploy_go_container_template::ImageTemplate::Etcd => "etcd",
    }
}

/// 返回部署制品需要使用的应用配置副本（delivery=artifact 的可编辑非敏感文件）。
/// 敏感文件和 Env 文件继续走受控交付链路，不进入普通制品。
pub(crate) async fn collect_artifact_overrides(
    state: &AppState,
    application_id: &str,
    request_id: &str,
) -> ApiResult<HashMap<String, Vec<u8>>> {
    let files = sqlx::query_as::<_, ConfigFileRow>(&config_file_select(
        "WHERE f.application_id=? AND f.deleted_at IS NULL AND f.delivery='artifact' AND f.editable=1 AND f.sensitive=0 ORDER BY f.path",
    ))
    .bind(application_id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let mut overrides = HashMap::new();
    for file in files {
        let Some(deploy_path) = file.deploy_path.as_deref() else {
            continue;
        };
        let content = read_version_content(state, &file, request_id).await?;
        overrides.insert(deploy_path.to_owned(), content.into_bytes());
    }
    Ok(overrides)
}

pub(crate) fn artifact_overrides_digest(overrides: &HashMap<String, Vec<u8>>) -> String {
    let mut entries = overrides.iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(right.0));
    let mut bytes = Vec::new();
    for (path, content) in entries {
        bytes.extend_from_slice(path.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(content);
    }
    hex_digest(&bytes)
}

/// 固化 preview 引用的配置版本集合。普通 snapshot 不包含敏感文件正文，
/// 只记录 opaque version 和可用摘要；artifact_digest 用于 confirm 时校验配置未漂移。
pub(crate) async fn configuration_snapshot(
    state: &AppState,
    application_id: &str,
    request_id: &str,
) -> ApiResult<Option<Value>> {
    let binding: Option<(String, String, String)> = sqlx::query_as(
        "SELECT b.template_id,b.template_version,b.template_digest FROM application_template_bindings b JOIN application_config_files f ON f.binding_id=b.id WHERE f.application_id=? AND f.deleted_at IS NULL ORDER BY b.created_at LIMIT 1",
    )
    .bind(application_id)
    .fetch_optional(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let Some((template_id, template_version, template_digest)) = binding else {
        return Ok(None);
    };
    let files = sqlx::query_as::<_, ConfigFileRow>(&config_file_select(
        "WHERE f.application_id=? AND f.deleted_at IS NULL ORDER BY f.path",
    ))
    .bind(application_id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let mut artifact_entries = Vec::new();
    let mut env_entries = Vec::new();
    let mut file_infos = Vec::new();
    for file in files {
        let Some(deploy_path) = file.deploy_path.as_deref() else {
            continue;
        };
        let content = read_version_content(state, &file, request_id).await?;
        if file.delivery == "artifact" && file.editable != 0 && file.sensitive == 0 {
            artifact_entries.push((deploy_path.to_owned(), content.into_bytes()));
        } else if file.delivery == "env_lease" {
            env_entries.push((deploy_path.to_owned(), content.into_bytes()));
        }
        file_infos.push(json!({
            "path": file.path,
            "deploy_path": deploy_path,
            "sensitive": file.sensitive != 0,
            "version": file.current_version,
            "digest": (file.sensitive == 0).then_some(file.current_digest.clone()),
        }));
    }
    artifact_entries.sort_by(|left, right| left.0.cmp(&right.0));
    env_entries.sort_by(|left, right| left.0.cmp(&right.0));
    let mut artifact_bytes = Vec::new();
    for (path, content) in &artifact_entries {
        artifact_bytes.extend_from_slice(path.as_bytes());
        artifact_bytes.push(0);
        artifact_bytes.extend_from_slice(content);
    }
    let mut env_bytes = Vec::new();
    for (path, content) in &env_entries {
        env_bytes.extend_from_slice(path.as_bytes());
        env_bytes.push(0);
        env_bytes.extend_from_slice(content);
    }
    Ok(Some(json!({
        "application_id": application_id,
        "template_id": template_id,
        "template_version": template_version,
        "template_digest": template_digest,
        "files": file_infos,
        "artifact_digest": hex_digest(&artifact_bytes),
        "env_digest": hex_digest(&env_bytes),
        "artifact_files": artifact_entries.iter().map(|(path, _)| path).collect::<Vec<_>>(),
    })))
}

/// 把配置工作区的 dotenv 文件同步到旧 Env 交付链路。
/// 旧链路负责 Agent 节点同步、版本门禁和敏感文件租赁，部署时 Env 仍从该链路取用。
async fn sync_env_lease_to_legacy(
    transaction: &mut Transaction<'_, Sqlite>,
    ring: &crate::crypto::MasterKeyRing,
    application_id: &str,
    deploy_path: &str,
    content: &[u8],
    actor_id: Option<&str>,
    request_id: &str,
) -> ApiResult<()> {
    let template_id: String = sqlx::query_scalar(
        "SELECT b.template_id FROM application_template_bindings b JOIN application_config_files f ON f.binding_id=b.id WHERE f.application_id=? AND f.deploy_path=? AND f.deleted_at IS NULL LIMIT 1",
    )
    .bind(application_id)
    .bind(deploy_path)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| ApiError::internal(request_id))?
    .ok_or_else(|| ApiError::not_found(request_id))?;
    let digest = hex_digest(content);
    let now = Utc::now().to_rfc3339();
    let existing: Option<(String, i64)> = sqlx::query_as(
        "SELECT id,current_version FROM application_env_files WHERE application_id=? AND file_name=? AND deleted_at IS NULL",
    )
    .bind(application_id)
    .bind(deploy_path)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let (env_file_id, next_version) = if let Some((id, version)) = existing {
        (id, version + 1)
    } else {
        let file_id = format!("envf_{}", Ulid::new());
        sqlx::query("INSERT INTO application_env_files (id,application_id,file_name,module,format,current_version,current_digest,declared_at,created_at,updated_at) VALUES (?,?,?,?,?,1,?,?,?,?)")
            .bind(&file_id)
            .bind(application_id)
            .bind(deploy_path)
            .bind(&template_id)
            .bind("dotenv-v1")
            .bind(&digest)
            .bind(&now)
            .bind(&now)
            .bind(&now)
            .execute(&mut **transaction)
            .await
            .map_err(|_| ApiError::internal(request_id))?;
        (file_id, 1)
    };
    let version_id = format!("envv_{}", Ulid::new());
    let encrypted = ring
        .encrypt_application_env(application_id, &env_file_id, &version_id, content)
        .map_err(|_| ApiError::internal(request_id))?;
    sqlx::query("UPDATE application_env_files SET current_version=?,current_digest=?,updated_at=?,version=version+1 WHERE id=? AND deleted_at IS NULL")
        .bind(next_version)
        .bind(&digest)
        .bind(&now)
        .bind(&env_file_id)
        .execute(&mut **transaction)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    sqlx::query("INSERT INTO application_env_versions (id,env_file_id,env_version,algorithm,ciphertext,nonce,key_version,digest,created_by,created_at) VALUES (?,?,?,?,?,?,?,?,?,?)")
        .bind(&version_id)
        .bind(&env_file_id)
        .bind(next_version)
        .bind(crate::crypto::APPLICATION_ENV_ALGORITHM)
        .bind(encrypted.ciphertext)
        .bind(encrypted.nonce)
        .bind(encrypted.key_version)
        .bind(&digest)
        .bind(actor_id)
        .bind(&now)
        .execute(&mut **transaction)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    crate::application_envs::create_sync_rows(transaction, &version_id, application_id, "write")
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    Ok(())
}

fn hex_digest(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn version_conflict(request_id: &str) -> ApiError {
    ApiError::conflict(
        "resource_version_conflict",
        "应用配置已经被其他请求修改",
        request_id,
    )
}

fn map_unique(error: sqlx::Error, request_id: &str) -> ApiError {
    if error.to_string().contains("UNIQUE constraint failed") {
        ApiError::conflict(
            "application_config_binding_conflict",
            "应用配置模板绑定已存在",
            request_id,
        )
    } else {
        ApiError::internal(request_id)
    }
}

#[cfg(test)]
mod tests {
    use super::{config_status, contains_secret_placeholder};

    #[test]
    fn secret_placeholders_start_incomplete_and_real_values_are_ready() {
        assert!(contains_secret_placeholder(b"PASSWORD=change-me"));
        assert_eq!(config_status(true, b"PASSWORD=change-me"), "incomplete");
        assert_eq!(config_status(true, b"PASSWORD=generated"), "ready");
        assert_eq!(config_status(false, b"PASSWORD=change-me"), "ready");
    }
}
