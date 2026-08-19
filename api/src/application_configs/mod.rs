use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use chrono::Utc;
use deploy_go_container_template::{
    ImageDeploySpec as PlatformImageDeploySpec, TemplateFileRole, template_descriptor,
    template_from_id,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
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
    pub template_source_digest: String,
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
    pub digest: Option<String>,
    pub source: String,
    pub source_version_id: Option<String>,
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
        .route("/application-config-files/{id}", get(show).put(update))
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
) -> ApiResult<Json<ApplicationConfigFileListResponse>> {
    grants::require_application_access(state.pool(), &actor, &application_id, request_id.as_str())
        .await?;
    let rows = sqlx::query_as::<_, ConfigFileRow>(&config_file_select(
        "WHERE f.application_id=? AND f.deleted_at IS NULL ORDER BY f.path",
    ))
    .bind(&application_id)
    .fetch_all(state.pool())
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(ApplicationConfigFileListResponse {
        items: rows.into_iter().map(file_response).collect(),
    }))
}

#[utoipa::path(
    operation_id = "application_configs_show",
    get,
    path = "/api/v1/application-config-files/{id}",
    params(("id" = String, Path)),
    responses((status = 200, body = ApplicationConfigFileResponse), (status = 401, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse))
)]
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    actor: AuthUser,
) -> ApiResult<Json<ApplicationConfigFileResponse>> {
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
        None
    };
    Ok(Json(file_response_with_content(row, content)))
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
) -> ApiResult<Json<ApplicationConfigVersionListResponse>> {
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
    Ok(Json(ApplicationConfigVersionListResponse {
        items: rows
            .into_iter()
            .map(|row| version_response(row, file.sensitive != 0))
            .collect(),
    }))
}

#[utoipa::path(
    operation_id = "application_configs_update",
    put,
    path = "/api/v1/application-config-files/{id}",
    params(("id" = String, Path)),
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
) -> ApiResult<Json<ApplicationConfigFileResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let current = load_file(state.pool(), &id, request_id.as_str()).await?;
    if current.editable == 0 || current.deleted_at.is_some() {
        return Err(ApiError::forbidden(request_id.as_str()));
    }
    if current.sensitive != 0 {
        return Err(ApiError::forbidden(request_id.as_str()));
    }
    validate_content(&payload.content, request_id.as_str())?;
    let ring = state
        .master_key_ring()
        .ok_or_else(|| ApiError::service_not_ready(request_id.as_str()))?;
    let version_id = format!("cfgv_{}", Ulid::new());
    let encrypted = ring
        .encrypt_application_config(
            &current.application_id,
            &current.id,
            &version_id,
            payload.content.as_bytes(),
        )
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    let digest = hex_digest(payload.content.as_bytes());
    let status = config_status(current.sensitive != 0, payload.content.as_bytes());
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
            source: "user",
            source_version_id: None,
            source_template_digest: None,
            created_by: Some(&actor.id),
            created_at: &now,
        },
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    audit::record(
        &mut transaction,
        Some(&actor.id),
        "application_config.update",
        "application_config_file",
        &current.id,
        request_id.as_str(),
        json!({
            "application_id": current.application_id,
            "path": current.path,
            "old_version": current.current_version,
            "new_version": current.current_version + 1,
            "digest": (!current.sensitive.eq(&1)).then_some(&digest)
        }),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(file_response_with_content(
        load_file(state.pool(), &id, request_id.as_str()).await?,
        (!current.sensitive.eq(&1)).then_some(payload.content),
    )))
}

#[utoipa::path(
    operation_id = "application_configs_restore",
    post,
    path = "/api/v1/application-config-files/{id}/restore",
    params(("id" = String, Path)),
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
) -> ApiResult<Json<ApplicationConfigFileResponse>> {
    actor.require_administrator(request_id.as_str())?;
    actor.verify_csrf(&headers, request_id.as_str())?;
    let current = load_file(state.pool(), &id, request_id.as_str()).await?;
    if current.editable == 0 || current.deleted_at.is_some() {
        return Err(ApiError::forbidden(request_id.as_str()));
    }
    if current.sensitive != 0 {
        return Err(ApiError::forbidden(request_id.as_str()));
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
            "source_template_digest": source_template_digest
        }),
    )
    .await
    .map_err(|_| ApiError::internal(request_id.as_str()))?;
    transaction
        .commit()
        .await
        .map_err(|_| ApiError::internal(request_id.as_str()))?;
    Ok(Json(file_response_with_content(
        load_file(state.pool(), &id, request_id.as_str()).await?,
        (current.sensitive == 0).then_some(String::from_utf8_lossy(&plaintext).into_owned()),
    )))
}

#[utoipa::path(
    operation_id = "application_configs_initialize",
    post,
    path = "/api/v1/applications/{application_id}/config-files/initialize",
    params(("application_id" = String, Path)),
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
    params(("application_id" = String, Path)),
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
    file_response_with_content(row, None)
}

fn file_response_with_content(
    row: ConfigFileRow,
    content: Option<String>,
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
        template_source_digest: row.template_source_digest,
        current_version: row.current_version,
        current_digest: (!sensitive).then_some(row.current_digest),
        status: row.status,
        deleted_at: row.deleted_at,
        updated_at: row.updated_at,
        version: row.version,
        content: (!sensitive).then_some(content).flatten(),
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
        source_template_digest: row.source_template_digest,
        created_by: row.created_by,
        created_at: row.created_at,
    }
}

fn validate_content(content: &str, request_id: &str) -> ApiResult<()> {
    if content.len() > MAX_CONFIG_BYTES || content.contains('\0') {
        return Err(ApiError::validation(
            "配置正文大小或编码不符合约束",
            request_id,
        ));
    }
    Ok(())
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
        deploy_go_container_template::ImageTemplate::Postgres => "postgres",
        deploy_go_container_template::ImageTemplate::Etcd => "etcd",
    }
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
