//! 对外 API：应用启停、Env 同步重试与配置文件元数据（非生产应用）。
//!
//! 写入统一经过：Key 绑定 → 非生产守卫 → 与管理面共用的核心函数。
//! 配置文件只返回元数据，永不返回内容。

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use utoipa::{IntoParams, ToSchema};

use super::{
    ExternalApiKey, require_env_file_owner, require_external_configurable_application,
    require_key_application_access, require_no_production_targets,
};
use crate::{
    AppState, RequestId,
    application_configs::{self, ApplicationConfigFileMetadata},
    application_envs::{self, RetryApplicationEnvSyncResponse},
    applications,
    auth::service_actor,
    error::{ApiError, ApiResult},
};

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExternalApplicationStatusRequest {
    /// `active` 或 `archived`
    status: String,
    version: i64,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalApplicationStatus {
    id: String,
    status: String,
    version: i64,
}

#[derive(Deserialize, IntoParams)]
pub struct ExternalEnvSyncRetryQuery {
    /// 只重试指定部署目标的同步；省略表示该 Env 文件所有失败的同步
    target_id: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ExternalConfigFileListResponse {
    items: Vec<ApplicationConfigFileMetadata>,
}

/// 与 `require_key_application_access` 相同的绑定检查，但不要求应用处于启用状态，
/// 否则被归档的应用会从 Key 视角消失，无法再启用。仍受非生产守卫约束。
async fn require_key_application_binding(
    pool: &SqlitePool,
    key: &ExternalApiKey,
    application_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let bound: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM external_api_key_applications key_app JOIN applications a ON a.id=key_app.application_id WHERE key_app.api_key_id=? AND key_app.application_id=?)",
    )
    .bind(&key.id)
    .bind(application_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    if bound {
        Ok(())
    } else {
        Err(ApiError::not_found(request_id))
    }
}

/// 管理员（或其他 Key）归档的应用不能被持 Key 者重新启用：归档是停用手段，外部调用方只能撤销自己做的归档。
/// 依据是该应用最近一次状态变更的审计记录：必须是由同一个外部 Key 归档的；查不到或不一致一律拒绝（fail closed）。
/// 应用当前已是启用状态时不需要检查（启用 → 启用不改变可用性）。
async fn require_reactivation_by_archiving_key(
    pool: &SqlitePool,
    key: &ExternalApiKey,
    application_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM applications WHERE id=?")
        .bind(application_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal(request_id))?;
    if status.as_deref() != Some("archived") {
        return Ok(());
    }
    let last: Option<String> = sqlx::query_scalar(
        "SELECT summary_json FROM audit_logs WHERE action='application.status.update' AND resource_type='application' AND resource_id=? ORDER BY created_at DESC,id DESC LIMIT 1",
    )
    .bind(application_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal(request_id))?;
    let archived_by_this_key = last
        .and_then(|summary| serde_json::from_str::<serde_json::Value>(&summary).ok())
        .is_some_and(|summary| {
            summary["status"] == "archived" && summary["external_api_key_id"] == key.id.as_str()
        });
    if archived_by_this_key {
        Ok(())
    } else {
        Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "external_application_reactivation_forbidden",
            "该应用不是由此 API Key 归档的，不能经对外 API 重新启用",
            request_id,
        ))
    }
}

/// 生产守卫不要求应用启用：按 ID 读取环境，不存在返回 404。
async fn require_non_production_application(
    pool: &SqlitePool,
    application_id: &str,
    request_id: &str,
) -> ApiResult<()> {
    let environment: Option<String> =
        sqlx::query_scalar("SELECT environment FROM applications WHERE id=?")
            .bind(application_id)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::internal(request_id))?;
    match environment.as_deref() {
        None => Err(ApiError::not_found(request_id)),
        Some("prod") => Err(super::production_application_forbidden(request_id)),
        Some(_) => Ok(()),
    }
}

#[utoipa::path(operation_id = "external_application_status_update", put, path = "/external/v1/applications/{id}/status", params(("id" = String, Path)), request_body = ExternalApplicationStatusRequest, responses((status = 200, body = ExternalApplicationStatus), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse), (status = 409, body = crate::error::ErrorResponse), (status = 422, body = crate::error::ErrorResponse)))]
pub(crate) async fn update_application_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
    crate::http::ApiJson(payload): crate::http::ApiJson<ExternalApplicationStatusRequest>,
) -> ApiResult<Json<ExternalApplicationStatus>> {
    require_key_application_binding(state.pool(), &key, &id, request_id.as_str()).await?;
    require_non_production_application(state.pool(), &id, request_id.as_str()).await?;
    if payload.status == "active" {
        require_reactivation_by_archiving_key(state.pool(), &key, &id, request_id.as_str()).await?;
    }
    let updated = applications::update_status_core(
        state.pool(),
        &id,
        &payload.status,
        payload.version,
        &service_actor().id,
        Some(&key.id),
        request_id.as_str(),
    )
    .await?;
    Ok(Json(ExternalApplicationStatus {
        id: updated.id,
        status: updated.status,
        version: updated.version,
    }))
}

#[utoipa::path(operation_id = "external_env_file_sync_retry", post, path = "/external/v1/applications/{id}/env-files/{env_file_id}/sync-retry", params(("id" = String, Path), ("env_file_id" = String, Path), ExternalEnvSyncRetryQuery), responses((status = 200, body = RetryApplicationEnvSyncResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn retry_env_sync(
    State(state): State<AppState>,
    Path((id, env_file_id)): Path<(String, String)>,
    Query(query): Query<ExternalEnvSyncRetryQuery>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Json<RetryApplicationEnvSyncResponse>> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    require_external_configurable_application(state.pool(), &id, request_id.as_str()).await?;
    require_env_file_owner(&state, &id, &env_file_id, request_id.as_str()).await?;
    // 重试会向该应用所有目标的 Agent 重新排队同步；带正式环境目标的应用不得借此触达生产节点。
    require_no_production_targets(state.pool(), &id, request_id.as_str()).await?;
    Ok(Json(
        application_envs::retry_sync_core(
            &state,
            &env_file_id,
            query.target_id.as_deref(),
            &service_actor().id,
            Some(&key.id),
            request_id.as_str(),
        )
        .await?,
    ))
}

#[utoipa::path(operation_id = "external_application_config_files_list", get, path = "/external/v1/applications/{id}/config-files", params(("id" = String, Path)), responses((status = 200, body = ExternalConfigFileListResponse), (status = 401, body = crate::error::ErrorResponse), (status = 403, body = crate::error::ErrorResponse), (status = 404, body = crate::error::ErrorResponse)))]
pub(crate) async fn list_config_files(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Extension(request_id): Extension<RequestId>,
    key: ExternalApiKey,
) -> ApiResult<Response> {
    require_key_application_access(state.pool(), &key, &id, request_id.as_str()).await?;
    // 配置文件元数据只对非生产应用开放：生产应用的配置结构本身也不经对外 API 暴露。
    require_external_configurable_application(state.pool(), &id, request_id.as_str()).await?;
    let items =
        application_configs::list_file_metadata(state.pool(), &id, request_id.as_str()).await?;
    let mut response = Json(ExternalConfigFileListResponse { items }).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}
